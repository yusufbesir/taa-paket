// Sahip: Yusuf. yazan: [Claude#egitim-panel 2026-11-26]
//! Yerel sunucunun güvenlik denetimleri (Çatı panelinin kalıbı; başka programın crate'ine bağımlı olmamak için burada).
//!
//! - Yalnız 127.0.0.1; `Host` başlığı tam `127.0.0.1:<kapı>` olmalı (DNS yeniden bağlama).
//! - Tek seferlik açılış belirteci → `HttpOnly; SameSite=Strict` çerez; her istek çerezle denetlenir (sabit süreli).
//! - POST: `Origin` varsa yalnız kendi kökeni, gövde JSON, en çok 64 KB.
//! - Sıkı CSP (betik ve biçem satır içi, dış kaynak yok; video yalnız kendi sunucusu ve https).

/// Çerez adı.
pub const CEREZ: &str = "egitim_panel";
/// İstek gövdesi üst sınırı.
pub const GOVDE_EN_COK: usize = 64 * 1024;

/// Her yanıta eklenen güvenlik başlıkları.
pub const GUVENLIK_BASLIKLARI: &[(&str, &str)] = &[
    (
        "Content-Security-Policy",
        "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src 'self' data:; \
         media-src 'self' https:; connect-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'",
    ),
    ("X-Frame-Options", "DENY"),
    ("X-Content-Type-Options", "nosniff"),
    ("Referrer-Policy", "no-referrer"),
    ("Cache-Control", "no-store"),
    ("Cross-Origin-Opener-Policy", "same-origin"),
    ("Cross-Origin-Resource-Policy", "same-origin"),
];

/// 32 bayt rastgele → 64 hex.
pub fn belirtec_uret() -> Result<String, String> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Sabit süreli karşılaştırma (uzunluk farkı hemen reddedilir; belirteç uzunluğu gizli değildir).
pub fn esit(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |f, (x, y)| f | (x ^ y)) == 0
}

/// `Cookie` başlığından adı verilen çerezin değeri.
pub fn cerez_degeri<'a>(baslik: &'a str, ad: &str) -> Option<&'a str> {
    baslik.split(';').find_map(|p| {
        let (k, v) = p.trim().split_once('=')?;
        (k == ad).then_some(v)
    })
}

/// Sorgu dizesinden (`a=b&c=d`) ham değer.
pub fn sorgu_degeri<'a>(sorgu: &'a str, ad: &str) -> Option<&'a str> {
    sorgu.split('&').find_map(|p| {
        let (k, v) = p.split_once('=')?;
        (k == ad).then_some(v)
    })
}

/// Yüzde kodlamasını çözer (`%C3%BC` gibi; `+` boşluk). Geçersiz dizi ya da UTF-8 dışı bayt: None.
pub fn yuzde_coz(s: &str) -> Option<String> {
    let b = s.as_bytes();
    let mut cikti = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'%' => {
                let h = s.get(i + 1..i + 3)?;
                cikti.push(u8::from_str_radix(h, 16).ok()?);
                i += 3;
            }
            b'+' => {
                cikti.push(b' ');
                i += 1;
            }
            c => {
                cikti.push(c);
                i += 1;
            }
        }
    }
    String::from_utf8(cikti).ok()
}

/// Bir isteğin yerel güvenlik denetimi için gereken parçaları.
#[derive(Debug, Default, Clone)]
pub struct IstekBilgi<'a> {
    pub yontem: &'a str,
    pub host: Option<&'a str>,
    pub origin: Option<&'a str>,
    pub cerez: Option<&'a str>,
    pub icerik_turu: Option<&'a str>,
}

/// Denetim sonucu.
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

/// `Host` başlığı yalnız `127.0.0.1:<kapı>` olabilir.
pub fn host_gecerli(host: Option<&str>, kapi: u16) -> bool {
    host.is_some_and(|h| h == format!("127.0.0.1:{kapi}"))
}

/// API isteği denetimi: Host, çerez, (POST için) köken ve içerik türü.
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

/// `Set-Cookie` değeri (oturum çerezi; kalıcı değil, pencere kapanınca biter).
pub fn cerez_ayarla(belirtec: &str) -> String {
    format!("{CEREZ}={belirtec}; HttpOnly; SameSite=Strict; Path=/")
}

#[cfg(test)]
mod testler {
    use super::*;

    fn bilgi<'a>(yontem: &'a str, cerez: Option<&'a str>) -> IstekBilgi<'a> {
        IstekBilgi {
            yontem,
            host: Some("127.0.0.1:5000"),
            origin: None,
            cerez,
            icerik_turu: Some("application/json"),
        }
    }

    #[test]
    fn belirtec_64_hex_ve_benzersiz() {
        let (a, b) = (belirtec_uret().unwrap(), belirtec_uret().unwrap());
        assert_eq!(a.len(), 64);
        assert!(a.bytes().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn esitlik_ve_cerez() {
        assert!(esit("abc", "abc"));
        assert!(!esit("abc", "abd"));
        assert!(!esit("abc", "ab"));
        assert_eq!(
            cerez_degeri("a=1; egitim_panel=xyz; b=2", CEREZ),
            Some("xyz")
        );
        assert_eq!(cerez_degeri("a=1", CEREZ), None);
    }

    #[test]
    fn host_cerez_koken_icerik_turu() {
        let t = "t".repeat(64);
        let c = format!("{CEREZ}={t}");
        assert_eq!(istek_denetle(&bilgi("GET", Some(&c)), &t, 5000), Ok(()));
        let mut yanlis_host = bilgi("GET", Some(&c));
        yanlis_host.host = Some("evil.test:5000");
        assert_eq!(istek_denetle(&yanlis_host, &t, 5000), Err(Red::Host));
        assert_eq!(
            istek_denetle(&bilgi("GET", None), &t, 5000),
            Err(Red::Cerez)
        );
        let yanlis = format!("{CEREZ}={}", "x".repeat(64));
        assert_eq!(
            istek_denetle(&bilgi("GET", Some(&yanlis)), &t, 5000),
            Err(Red::Cerez)
        );
        let mut baska = bilgi("POST", Some(&c));
        baska.origin = Some("http://evil.test");
        assert_eq!(istek_denetle(&baska, &t, 5000), Err(Red::Koken));
        let mut duz = bilgi("POST", Some(&c));
        duz.icerik_turu = Some("text/plain");
        assert_eq!(istek_denetle(&duz, &t, 5000), Err(Red::IcerikTuru));
        let mut tamam = bilgi("POST", Some(&c));
        tamam.origin = Some("http://127.0.0.1:5000");
        assert_eq!(istek_denetle(&tamam, &t, 5000), Ok(()));
        assert_eq!(Red::Host.durum_kodu(), 421);
        assert_eq!(Red::Cerez.durum_kodu(), 401);
    }

    #[test]
    fn csp_dis_kaynak_yok_ama_video_var() {
        let csp = GUVENLIK_BASLIKLARI
            .iter()
            .find(|(a, _)| *a == "Content-Security-Policy")
            .unwrap()
            .1;
        assert!(csp.contains("default-src 'none'"));
        assert!(csp.contains("media-src 'self' https:"));
        assert!(csp.contains("frame-ancestors 'none'"));
        assert!(!csp.contains("script-src 'self'"));
        assert!(cerez_ayarla("t").contains("HttpOnly; SameSite=Strict"));
    }

    #[test]
    fn sorgu_ve_yuzde() {
        assert_eq!(sorgu_degeri("konu=a&gorev=b", "gorev"), Some("b"));
        assert_eq!(sorgu_degeri("konu=a", "gorev"), None);
        assert_eq!(yuzde_coz("kasa-egitimi").as_deref(), Some("kasa-egitimi"));
        assert_eq!(yuzde_coz("%C3%BC+x").as_deref(), Some("ü x"));
        assert_eq!(yuzde_coz("%zz"), None);
        assert_eq!(yuzde_coz("%C3"), None);
    }
}
