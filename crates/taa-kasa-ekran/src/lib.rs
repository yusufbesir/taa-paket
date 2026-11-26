// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! TAA Kasa programının platformdan bağımsız çekirdeği: yerel güvenlik, oturum, yönetici girişi ve API.
//!
//! Kalıp `taa-cati` panelinden alındı (crates/taa-cati/src/panel.rs): yalnız 127.0.0.1, rastgele kapı, tek
//! seferlik açılış belirteci → `HttpOnly; SameSite=Strict` çerez, Host/Origin denetimi, sıkı CSP. Programlar
//! arası zorunlu bağımlılık yasak olduğu için (ince çizgi) kod kopyalandı; ortak bir kabuk crate'ine taşınması
//! önerilir (A0).
//!
//! Giriş: ortak giriş (KM1 `kimlik.yonetici@1`, KM2 personel PIN'i) henüz bu programa bağlanmadı.
//! - Personel girişi **yer tutucudur**: yöneticinin Ayarlar'da tanımladığı listeden ad seçilir, PIN sorulmaz.
//! - Yönetici girişi **geçici yerel PIN**dir (ana plan §5.3, "paketsiz kurulumda yerel PIN"): ilk kullanımda
//!   belirlenir, tuzlu SHA-256 özeti (100.000 tur) veri kökünde tutulur, PIN'in kendisi hiçbir yere yazılmaz.
//!   KM1 bağlanınca bu yol kaldırılır.

pub mod api;

use sha2::{Digest, Sha256};

/// Gömülü ekran (tek dosya, dış kaynak yok).
pub const ARAYUZ_HTML: &str = include_str!("arayuz.html");
/// Uygulama simgesi.
pub const SIMGE_SVG: &str = include_str!("simge.svg");

pub const CEREZ: &str = "taa_kasa";

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Base64 çözücü (Excel içe aktarma; dış bağımlılık yok). `data:...;base64,` öneki kabul edilir.
pub fn base64_coz(s: &str) -> Result<Vec<u8>, String> {
    let s = s.rsplit(',').next().unwrap_or(s);
    let mut o = Vec::with_capacity(s.len() * 3 / 4);
    let (mut tampon, mut bit) = (0u32, 0u32);
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace() && *c != b'=') {
        let v = B64
            .iter()
            .position(|x| *x == c)
            .ok_or("dosya kodlaması bozuk")? as u32;
        tampon = (tampon << 6) | v;
        bit += 6;
        if bit >= 8 {
            bit -= 8;
            o.push((tampon >> bit) as u8);
            tampon &= (1 << bit) - 1;
        }
    }
    Ok(o)
}

pub fn base64_kodla(b: &[u8]) -> String {
    let mut o = String::new();
    for p in b.chunks(3) {
        let n = (p[0] as u32) << 16
            | (*p.get(1).unwrap_or(&0) as u32) << 8
            | *p.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= p.len() {
                o.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                o.push('=');
            }
        }
    }
    o
}
/// İndirme başlığı (`Content-Disposition`). Başlık değeri ASCII olmalı: Türkçe harfler sadeleştirilir, öteki
/// karakterler `_` olur (tırnak, `\`, `/`, `:` dahil; Windows'ta geçersiz dosya adı oluşmasın). Asıl ad
/// `filename*` (RFC 5987) ile de verilir. Şube adı Türkçe ya da boşluklu olsa da başlık düşmez.
pub fn indirme_basligi(ad: &str) -> String {
    let sade: String = ad
        .chars()
        .map(|c| match c {
            'ç' => 'c',
            'Ç' => 'C',
            'ğ' => 'g',
            'Ğ' => 'G',
            'ı' => 'i',
            'İ' => 'I',
            'ö' => 'o',
            'Ö' => 'O',
            'ş' => 's',
            'Ş' => 'S',
            'ü' => 'u',
            'Ü' => 'U',
            c if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') => c,
            _ => '_',
        })
        .collect();
    let mut kod = String::new();
    for b in ad.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.') {
            kod.push(*b as char);
        } else {
            kod.push_str(&format!("%{b:02X}"));
        }
    }
    format!("attachment; filename=\"{sade}\"; filename*=UTF-8''{kod}")
}

pub const GOVDE_EN_COK: usize = 256 * 1024;

pub const GUVENLIK_BASLIKLARI: &[(&str, &str)] = &[
    (
        "Content-Security-Policy",
        "default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'unsafe-inline'; img-src 'self' data:; \
         connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
    ),
    ("X-Frame-Options", "DENY"),
    ("X-Content-Type-Options", "nosniff"),
    ("Referrer-Policy", "no-referrer"),
    ("Cache-Control", "no-store"),
    ("Cross-Origin-Opener-Policy", "same-origin"),
    ("Cross-Origin-Resource-Policy", "same-origin"),
];

pub fn rastgele_hex(n: usize) -> Result<String, String> {
    let mut b = vec![0u8; n];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

pub fn belirtec_uret() -> Result<String, String> {
    rastgele_hex(32)
}

/// Sabit süreli karşılaştırma.
pub fn esit(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |f, (x, y)| f | (x ^ y)) == 0
}

pub fn cerez_degeri<'a>(baslik: &'a str, ad: &str) -> Option<&'a str> {
    baslik.split(';').find_map(|p| {
        let (k, v) = p.trim().split_once('=')?;
        (k == ad).then_some(v)
    })
}

pub fn sorgu_degeri<'a>(sorgu: &'a str, ad: &str) -> Option<&'a str> {
    sorgu.split('&').find_map(|p| {
        let (k, v) = p.split_once('=')?;
        (k == ad).then_some(v)
    })
}

#[derive(Debug, Default, Clone)]
pub struct IstekBilgi<'a> {
    pub yontem: &'a str,
    pub host: Option<&'a str>,
    pub origin: Option<&'a str>,
    pub cerez: Option<&'a str>,
    pub icerik_turu: Option<&'a str>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Red {
    Host,
    Cerez,
    Koken,
    IcerikTuru,
}

impl Red {
    pub fn durum_kodu(&self) -> u16 {
        match self {
            Red::Host => 421,
            Red::Cerez => 401,
            Red::Koken => 403,
            Red::IcerikTuru => 415,
        }
    }
}

pub fn host_gecerli(host: Option<&str>, kapi: u16) -> bool {
    host.is_some_and(|h| h == format!("127.0.0.1:{kapi}"))
}

pub fn istek_denetle(i: &IstekBilgi, belirtec: &str, kapi: u16) -> Result<(), Red> {
    if !host_gecerli(i.host, kapi) {
        return Err(Red::Host);
    }
    let cerez_ok = i
        .cerez
        .and_then(|c| cerez_degeri(c, CEREZ))
        .is_some_and(|v| esit(v, belirtec));
    if !cerez_ok {
        return Err(Red::Cerez);
    }
    if i.yontem == "POST" {
        if let Some(o) = i.origin {
            if o != format!("http://127.0.0.1:{kapi}") {
                return Err(Red::Koken);
            }
        }
        let json_mu = i.icerik_turu.is_some_and(|t| {
            t.trim()
                .to_ascii_lowercase()
                .starts_with("application/json")
        });
        if !json_mu {
            return Err(Red::IcerikTuru);
        }
    }
    Ok(())
}

pub fn cerez_ayarla(belirtec: &str) -> String {
    format!("{CEREZ}={belirtec}; HttpOnly; SameSite=Strict; Path=/")
}

// ---------------------------------------------------------------- geçici yerel yönetici PIN'i

/// PIN özetleme turu.
pub const PIN_TUR: u32 = 100_000;

/// tuz (hex) + PIN → özet (hex). PIN en az 6 rakam.
pub fn pin_ozeti(tuz: &str, pin: &str) -> String {
    let mut h: [u8; 32] = Sha256::new()
        .chain_update(tuz.as_bytes())
        .chain_update(pin.as_bytes())
        .finalize()
        .into();
    for _ in 1..PIN_TUR {
        h = Sha256::new()
            .chain_update(h)
            .chain_update(tuz.as_bytes())
            .finalize()
            .into();
    }
    h.iter().map(|x| format!("{x:02x}")).collect()
}

pub fn pin_gecerli(pin: &str) -> bool {
    (6..=12).contains(&pin.len()) && pin.chars().all(|c| c.is_ascii_digit())
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn indirme_basligi_ascii_ve_guvenli() {
        let b = indirme_basligi("taa-kasa-Şişli \"Ç\"/x:1.xlsx");
        assert!(b.is_ascii(), "{b}");
        assert!(
            b.starts_with("attachment; filename=\"taa-kasa-Sisli__C__x_1.xlsx\""),
            "{b}"
        );
        assert!(
            b.contains("filename*=UTF-8''taa-kasa-%C5%9Ei%C5%9Fli%20"),
            "{b}"
        );
        assert!(tiny_http::Header::from_bytes(&b"Content-Disposition"[..], b.as_bytes()).is_ok());
    }

    #[test]
    fn istek_denetimi() {
        let b = "ab".repeat(32);
        let c = format!("x=1; {CEREZ}={b}");
        let iyi = IstekBilgi {
            yontem: "POST",
            host: Some("127.0.0.1:5000"),
            origin: Some("http://127.0.0.1:5000"),
            cerez: Some(&c),
            icerik_turu: Some("application/json; charset=utf-8"),
        };
        assert_eq!(istek_denetle(&iyi, &b, 5000), Ok(()));
        let host = IstekBilgi {
            host: Some("localhost:5000"),
            ..iyi.clone()
        };
        assert_eq!(istek_denetle(&host, &b, 5000), Err(Red::Host));
        let koken = IstekBilgi {
            origin: Some("http://kotu.ornek"),
            ..iyi.clone()
        };
        assert_eq!(istek_denetle(&koken, &b, 5000), Err(Red::Koken));
        let tur = IstekBilgi {
            icerik_turu: Some("text/plain"),
            ..iyi.clone()
        };
        assert_eq!(istek_denetle(&tur, &b, 5000), Err(Red::IcerikTuru));
        let cerezsiz = IstekBilgi {
            cerez: None,
            ..iyi.clone()
        };
        assert_eq!(istek_denetle(&cerezsiz, &b, 5000), Err(Red::Cerez));
        let yanlis = format!("{CEREZ}={}", "cd".repeat(32));
        let y = IstekBilgi {
            cerez: Some(&yanlis),
            ..iyi
        };
        assert_eq!(istek_denetle(&y, &b, 5000), Err(Red::Cerez));
    }

    #[test]
    fn pin() {
        assert!(pin_gecerli("123456") && !pin_gecerli("12345") && !pin_gecerli("12345a"));
        let a = pin_ozeti("tuz", "123456");
        assert_eq!(a, pin_ozeti("tuz", "123456"));
        assert_ne!(a, pin_ozeti("tuz2", "123456"));
        assert_eq!(a.len(), 64);
    }
}
