//! Yerel panel güvenliği (taa-cati paneliyle aynı kalıp; program crate'i başka programa bağlanamadığı için burada).
//!
//! - Yalnız `127.0.0.1`'e bağlanır; `Host` tam olarak `127.0.0.1:<kapı>` olmalı (DNS yeniden bağlama).
//! - Tek seferlik açılış belirteci → `HttpOnly; SameSite=Strict` çerez; her istek çerezle denetlenir (sabit süreli).
//! - POST: `Origin` varsa yalnız kendi kökeni, gövde JSON olmalı.
//! - Sıkı CSP (dış kaynak yok), `X-Frame-Options: DENY`, `no-store`; gövde en çok 64 KB.
//!
//! Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]

/// Çerez adı.
pub const CEREZ: &str = "rehber_panel";
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

/// 32 bayt rastgele → 64 onaltılık hane.
pub fn belirtec_uret() -> Result<String, String> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// Sabit süreli karşılaştırma.
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

/// Sorgu dizesinden tek değer (yüzde kodlaması çözülmez; yalnız belirteç gibi ASCII değerler için).
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

/// `Set-Cookie` değeri (oturum çerezi; pencere kapanınca biter).
pub fn cerez_ayarla(belirtec: &str) -> String {
    format!("{CEREZ}={belirtec}; HttpOnly; SameSite=Strict; Path=/")
}

#[cfg(test)]
mod testler {
    use super::*;

    fn bilgi<'a>(yontem: &'a str, cerez: Option<&'a str>) -> IstekBilgi<'a> {
        IstekBilgi {
            yontem,
            host: Some("127.0.0.1:4000"),
            origin: None,
            cerez,
            icerik_turu: Some("application/json"),
        }
    }

    #[test]
    fn host_ve_cerez_denetimi() {
        let b = "abc";
        let c = format!("{CEREZ}=abc");
        assert_eq!(istek_denetle(&bilgi("GET", Some(&c)), b, 4000), Ok(()));
        assert_eq!(istek_denetle(&bilgi("GET", None), b, 4000), Err(Red::Cerez));
        assert_eq!(
            istek_denetle(&bilgi("GET", Some("rehber_panel=xyz")), b, 4000),
            Err(Red::Cerez)
        );
        let mut yanlis = bilgi("GET", Some(&c));
        yanlis.host = Some("evil.test:4000");
        assert_eq!(istek_denetle(&yanlis, b, 4000), Err(Red::Host));
        yanlis.host = None;
        assert_eq!(istek_denetle(&yanlis, b, 4000), Err(Red::Host));
    }

    #[test]
    fn post_koken_ve_icerik_turu() {
        let c = format!("{CEREZ}=abc");
        let mut p = bilgi("POST", Some(&c));
        assert_eq!(istek_denetle(&p, "abc", 4000), Ok(()));
        p.origin = Some("http://evil.test");
        assert_eq!(istek_denetle(&p, "abc", 4000), Err(Red::Koken));
        p.origin = Some("http://127.0.0.1:4000");
        p.icerik_turu = Some("text/plain");
        assert_eq!(istek_denetle(&p, "abc", 4000), Err(Red::IcerikTuru));
        p.icerik_turu = None;
        assert_eq!(istek_denetle(&p, "abc", 4000), Err(Red::IcerikTuru));
    }

    #[test]
    fn belirtec_ve_esitlik() {
        let a = belirtec_uret().unwrap();
        let b = belirtec_uret().unwrap();
        assert_eq!(a.len(), 64);
        assert_ne!(a, b);
        assert!(esit(&a, &a));
        assert!(!esit(&a, &b));
        assert!(!esit("a", "ab"));
        assert!(cerez_ayarla("x").contains("HttpOnly"));
        assert_eq!(cerez_degeri("a=1; rehber_panel=zz", CEREZ), Some("zz"));
        assert_eq!(sorgu_degeri("b=1&c=2", "c"), Some("2"));
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
