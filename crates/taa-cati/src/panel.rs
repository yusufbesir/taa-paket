// Sahip: Yusuf. yazan: [Claude#taa-cati-panel 2026-11-17]
//! TAA Çatı paneli: platformdan bağımsız çekirdek (bin `taa-cati-panel`, docs/S3-CATI-PANEL.md).
//!
//! Panel yalnız `127.0.0.1` üzerinde, rastgele bir kapıda açılan yerel bir HTTP sunucusudur; ekran gömülü
//! HTML'dir (3 tema: Kule Gecesi, Defter, Taş). Sunucuya giden her istek yine `Cati` üzerinden gider
//! (cihaz imzası + Bearer, varsayılan reddet, 15 dk yeniden doğrulama); panel yeni bir yetki yolu açmaz.
//!
//! Güvenlik (yerel sunucu):
//! - Yalnız 127.0.0.1'e bağlanır; `Host` başlığı tam olarak `127.0.0.1:<kapı>` olmalı (DNS yeniden bağlama).
//! - 32 baytlık tek seferlik açılış belirteci → `HttpOnly; SameSite=Strict` çerez; her istek çerezle denetlenir
//!   (sabit süreli karşılaştırma). Aynı makinedeki başka bir program ya da site belirteci bilmeden çağıramaz.
//! - Yazan istekler (POST) `Origin` başlığı varsa yalnız kendi kökenini kabul eder ve gövde JSON olmalıdır.
//! - Sıkı CSP (dış kaynak yok), `X-Frame-Options: DENY`, `no-store`; gövde en çok 64 KB.
//! - Parola yalnız istek süresince bellekte; hiçbir yere yazılmaz, yanıtta geri dönmez.

use crate::{hata_kodu, Cati, CatiHata, OlaySuzgeci, RolAtama};
use serde_json::{json, Value};

/// Gömülü ekran (tek dosya, dış kaynak yok).
pub const ARAYUZ_HTML: &str = include_str!("panel/arayuz.html");
/// Uygulama simgesi (Üç Çatı).
pub const SIMGE_SVG: &str = include_str!("panel/uc-cati.svg");

/// Çerez adı.
pub const CEREZ: &str = "cati_panel";
/// İstek gövdesi üst sınırı.
pub const GOVDE_EN_COK: usize = 64 * 1024;

/// Her yanıta eklenen güvenlik başlıkları.
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

/// Sorgu dizesinden (`a=b&c=d`) değer. Yüzde kodlaması çözülmez (belirteç hex, tanıtıcılar ASCII).
pub fn sorgu_degeri<'a>(sorgu: &'a str, ad: &str) -> Option<&'a str> {
    sorgu.split('&').find_map(|p| {
        let (k, v) = p.split_once('=')?;
        (k == ad).then_some(v)
    })
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
    /// Host başlığı yanlış (yeniden bağlama girişimi ya da başka ad).
    Host,
    /// Çerez yok ya da yanlış.
    Cerez,
    /// Başka kökenden yazma isteği.
    Koken,
    /// POST gövdesi JSON değil.
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

/// Hata → HTTP durum kodu (gövde `{hata, kod}`; `kod` komut satırının çıkış kodudur).
pub fn hata_durumu(kod: u8) -> u16 {
    match kod {
        2 => 400,
        3 => 502,
        4 => 422,
        5 => 401,
        6 => 428,
        7 => 403,
        _ => 500,
    }
}

/// API yanıtı: (HTTP durum, JSON gövde).
pub type Yanit = (u16, Value);

fn hata(e: CatiHata) -> Yanit {
    let k = hata_kodu(&e);
    (hata_durumu(k), json!({ "hata": e.to_string(), "kod": k }))
}

fn girdi(m: &str) -> Yanit {
    (
        400,
        json!({ "hata": format!("geçersiz girdi: {m}"), "kod": 2 }),
    )
}

fn tamam(v: Value) -> Yanit {
    (200, v)
}

fn dizi(r: Result<Vec<Value>, CatiHata>) -> Yanit {
    r.map(|v| tamam(Value::Array(v))).unwrap_or_else(hata)
}

fn deger(r: Result<Value, CatiHata>) -> Yanit {
    r.map(tamam).unwrap_or_else(hata)
}

fn s<'a>(g: &'a Value, ad: &str) -> Option<&'a str> {
    g.get(ad).and_then(Value::as_str)
}

fn i(g: &Value, ad: &str) -> Option<i64> {
    g.get(ad).and_then(Value::as_i64)
}

/// Rol listesi: `[{"rol": "kasiyer", "sube_id": 4}, ...]`.
fn rol_listesi(g: &Value) -> Result<Vec<RolAtama>, CatiHata> {
    let Some(a) = g.get("roller").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };
    a.iter()
        .map(|r| {
            let ad = s(r, "rol").unwrap_or_default();
            RolAtama::yeni(ad, i(r, "sube_id"))
        })
        .collect()
}

fn rol_atama(g: &Value) -> Result<RolAtama, CatiHata> {
    RolAtama::yeni(s(g, "rol").unwrap_or_default(), i(g, "sube_id"))
}

/// Sunucu sağlık ucu (oturumsuz okunur; yalnız durum gösterilir).
pub const SAGLIK_UCU: &str = "/v1/saglik";

/// API yönlendirici: `yol` `/api/` ile başlar. Okumalar GET, yazmalar POST + JSON gövde.
/// Oturum açma/kapama ve oturum bilgisi bin tarafındadır (depo ve ağ erişimi ister).
pub fn api(
    c: &Cati,
    g: &dyn crate::Gonderici,
    yontem: &str,
    yol: &str,
    sorgu: &str,
    govde: &Value,
) -> Yanit {
    match (yontem, yol) {
        ("GET", "/api/saglik") => match g.al(SAGLIK_UCU) {
            Ok(v) => tamam(json!({ "ulasildi": true, "yanit": v })),
            Err(e) => (200, json!({ "ulasildi": false, "hata": e.to_string() })),
        },
        ("GET", "/api/kullanicilar") => dizi(c.kullanicilar()),
        ("GET", "/api/roller") => dizi(c.roller()),
        ("GET", "/api/izinler") => dizi(c.izinler()),
        ("GET", "/api/cihazlar") => {
            let d = sorgu_degeri(sorgu, "durum").filter(|d| !d.is_empty());
            dizi(c.cihazlar(d))
        }
        ("GET", "/api/subeler") => dizi(c.subeler()),
        ("GET", "/api/yetkiler") => dizi(c.program_yetkileri()),
        ("GET", "/api/gunluk") => {
            let sayi = |ad: &str| sorgu_degeri(sorgu, ad).and_then(|v| v.parse::<i64>().ok());
            let f = OlaySuzgeci {
                islem: sorgu_degeri(sorgu, "islem")
                    .filter(|v| !v.is_empty())
                    .map(str::to_string),
                kullanici_id: sayi("kullanici_id"),
                sube_id: sayi("sube_id"),
                tarih_bas: sorgu_degeri(sorgu, "tarih_bas")
                    .filter(|v| !v.is_empty())
                    .map(str::to_string),
                tarih_bit: sorgu_degeri(sorgu, "tarih_bit")
                    .filter(|v| !v.is_empty())
                    .map(str::to_string),
                limit: sorgu_degeri(sorgu, "limit").and_then(|v| v.parse().ok()),
                onceki_id: sayi("onceki_id"),
            };
            dizi(c.olaylar(&f))
        }
        ("POST", "/api/dogrula") => match s(govde, "parola") {
            Some(p) => deger(c.yeniden_dogrula(p)),
            None => girdi("parola gerekli"),
        },
        ("POST", "/api/cihaz/onay") => match s(govde, "kimlik") {
            Some(k) => deger(c.cihaz_onayla(k, i(govde, "sube_id"))),
            None => girdi("kimlik gerekli"),
        },
        ("POST", "/api/cihaz/iptal") => match s(govde, "kimlik") {
            Some(k) => deger(c.cihaz_iptal(k)),
            None => girdi("kimlik gerekli"),
        },
        ("POST", "/api/kullanici/ekle") => {
            let (Some(ad), Some(gad), Some(p)) =
                (s(govde, "ad"), s(govde, "gorunen_ad"), s(govde, "parola"))
            else {
                return girdi("ad, görünen ad ve parola gerekli");
            };
            match rol_listesi(govde) {
                Ok(r) => deger(c.kullanici_ekle(ad, gad, p, &r)),
                Err(e) => hata(e),
            }
        }
        ("POST", "/api/kullanici/rol") | ("POST", "/api/kullanici/rol-kaldir") => {
            let Some(id) = i(govde, "id") else {
                return girdi("id gerekli");
            };
            match rol_atama(govde) {
                Ok(r) if yol.ends_with("kaldir") => deger(c.rol_kaldir(id, &r)),
                Ok(r) => deger(c.rol_ver(id, &r)),
                Err(e) => hata(e),
            }
        }
        ("POST", "/api/kullanici/kapat") => match i(govde, "id") {
            Some(id) => deger(c.kullanici_kapat(id)),
            None => girdi("id gerekli"),
        },
        ("POST", "/api/kullanici/parola") => match (i(govde, "id"), s(govde, "parola")) {
            (Some(id), Some(p)) => deger(c.parola_degistir(id, p)),
            _ => girdi("id ve yeni parola gerekli"),
        },
        ("POST", "/api/yetki") => {
            let (Some(p), Some(acik)) = (
                s(govde, "program"),
                govde.get("acik").and_then(Value::as_bool),
            ) else {
                return girdi("program ve açık/kapalı gerekli");
            };
            deger(c.program_yetki_ayarla(
                i(govde, "sube_id"),
                s(govde, "rol").filter(|r| !r.is_empty()),
                p,
                acik,
            ))
        }
        ("POST", "/api/sube/ekle") => match (s(govde, "kod"), s(govde, "ad")) {
            (Some(k), Some(a)) => deger(c.sube_ekle(k, a)),
            _ => girdi("kod ve ad gerekli"),
        },
        (_, y) if y.starts_with("/api/") => (404, json!({ "hata": "bilinmeyen uç", "kod": 2 })),
        _ => (404, json!({ "hata": "bulunamadı", "kod": 2 })),
    }
}

/// Oturum bilgisi (gizli oturum anahtarı hariç): ekran menüsü ve izin düğmeleri için.
pub fn oturum_ozeti(o: &crate::Oturum) -> Value {
    json!({
        "girisli": true,
        "kullanici": o.kullanici,
        "sube_id": o.sube_id,
        "roller": o.roller,
        "izinler": o.izinler,
        "gecerlilik": o.gecerlilik,
    })
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::{GonderHata, Gonderici};
    use std::cell::RefCell;

    struct Sahte {
        istekler: RefCell<Vec<(String, String, Value)>>,
        hata: Option<GonderHata>,
    }
    impl Sahte {
        fn yeni() -> Sahte {
            Sahte {
                istekler: RefCell::new(Vec::new()),
                hata: None,
            }
        }
        fn son(&self) -> (String, String, Value) {
            self.istekler.borrow().last().cloned().expect("istek yok")
        }
    }
    impl Gonderici for Sahte {
        fn gonder(&self, uc: &str, g: &Value) -> Result<Value, GonderHata> {
            self.istekler
                .borrow_mut()
                .push(("POST".into(), uc.into(), g.clone()));
            self.hata.clone().map_or(Ok(json!({"tamam": true})), Err)
        }
        fn al(&self, uc: &str) -> Result<Value, GonderHata> {
            self.istekler
                .borrow_mut()
                .push(("GET".into(), uc.into(), Value::Null));
            self.hata.clone().map_or(Ok(json!([{"id": 1}])), Err)
        }
    }

    const B: &str = "ab12";

    fn bilgi<'a>(yontem: &'a str, cerez: &'a str) -> IstekBilgi<'a> {
        IstekBilgi {
            yontem,
            host: Some("127.0.0.1:5000"),
            origin: None,
            cerez: Some(cerez),
            icerik_turu: Some("application/json"),
        }
    }

    #[test]
    fn belirtec_64_hex_ve_her_seferinde_farkli() {
        let a = belirtec_uret().unwrap();
        let b = belirtec_uret().unwrap();
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
    }

    #[test]
    fn esit_sabit_sureli() {
        assert!(esit("abc", "abc"));
        assert!(!esit("abc", "abd"));
        assert!(!esit("abc", "abcd"));
        assert!(esit("", ""));
    }

    #[test]
    fn cerez_ve_sorgu_ayristirma() {
        assert_eq!(cerez_degeri("a=1; cati_panel=xyz; b=2", CEREZ), Some("xyz"));
        assert_eq!(cerez_degeri("cati_panelx=1", CEREZ), None);
        assert_eq!(
            sorgu_degeri("durum=bekliyor&x=1", "durum"),
            Some("bekliyor")
        );
        assert_eq!(sorgu_degeri("", "durum"), None);
    }

    #[test]
    fn denetim_host_cerez_koken_tur() {
        let c = format!("{CEREZ}={B}");
        assert_eq!(istek_denetle(&bilgi("GET", &c), B, 5000), Ok(()));
        // Host: başka ad / kapı / localhost reddedilir (DNS yeniden bağlama).
        for h in [
            "localhost:5000",
            "127.0.0.1:5001",
            "evil.example:5000",
            "127.0.0.1",
        ] {
            let mut x = bilgi("GET", &c);
            x.host = Some(h);
            assert_eq!(istek_denetle(&x, B, 5000), Err(Red::Host), "{h}");
        }
        let mut x = bilgi("GET", &c);
        x.host = None;
        assert_eq!(istek_denetle(&x, B, 5000), Err(Red::Host));
        // Çerez yok / yanlış.
        assert_eq!(
            istek_denetle(&bilgi("GET", "x=1"), B, 5000),
            Err(Red::Cerez)
        );
        let yanlis = format!("{CEREZ}=ab13");
        assert_eq!(
            istek_denetle(&bilgi("GET", &yanlis), B, 5000),
            Err(Red::Cerez)
        );
        // POST: başka köken reddedilir, kendi kökeni ve Origin'siz kabul.
        let mut p = bilgi("POST", &c);
        p.origin = Some("http://evil.example");
        assert_eq!(istek_denetle(&p, B, 5000), Err(Red::Koken));
        p.origin = Some("http://127.0.0.1:5000");
        assert_eq!(istek_denetle(&p, B, 5000), Ok(()));
        p.origin = None;
        assert_eq!(istek_denetle(&p, B, 5000), Ok(()));
        // POST: JSON olmayan gövde (form gönderimi, CORS'suz basit istek) reddedilir.
        p.icerik_turu = Some("text/plain");
        assert_eq!(istek_denetle(&p, B, 5000), Err(Red::IcerikTuru));
        p.icerik_turu = None;
        assert_eq!(istek_denetle(&p, B, 5000), Err(Red::IcerikTuru));
    }

    #[test]
    fn cerez_ozellikleri() {
        let c = cerez_ayarla(B);
        assert!(c.contains("HttpOnly") && c.contains("SameSite=Strict") && c.contains("Path=/"));
        assert!(!c.contains("Expires") && !c.contains("Max-Age"));
    }

    #[test]
    fn guvenlik_basliklari_dis_kaynak_yok() {
        let csp = GUVENLIK_BASLIKLARI
            .iter()
            .find(|(a, _)| *a == "Content-Security-Policy")
            .unwrap()
            .1;
        assert!(csp.contains("default-src 'none'"));
        assert!(csp.contains("frame-ancestors 'none'"));
        assert!(!csp.contains("http"));
        // Gömülü ekran dış adres yüklemez.
        assert!(!ARAYUZ_HTML.contains("https://"));
        assert!(!ARAYUZ_HTML.contains("http://"));
    }

    #[test]
    fn okumalar_dogru_uca_gider() {
        let g = Sahte::yeni();
        let c = Cati::izinsiz_denetimle(&g);
        for (yol, uc) in [
            ("/api/kullanicilar", "/v1/kullanici"),
            ("/api/roller", "/v1/rol"),
            ("/api/subeler", "/v1/sube"),
            ("/api/yetkiler", "/v1/program-yetki"),
        ] {
            let (d, _) = api(&c, &g, "GET", yol, "", &Value::Null);
            assert_eq!(d, 200, "{yol}");
            assert_eq!(g.son().1, uc);
        }
        api(
            &c,
            &g,
            "GET",
            "/api/cihazlar",
            "durum=bekliyor",
            &Value::Null,
        );
        assert_eq!(g.son().1, "/v1/cihaz?durum=bekliyor");
        api(
            &c,
            &g,
            "GET",
            "/api/gunluk",
            "limit=20&islem=giris",
            &Value::Null,
        );
        assert!(g.son().1.starts_with("/v1/olay?islem=giris"));
        assert!(g.son().1.contains("limit=20"));
    }

    #[test]
    fn yazmalar_govdeyi_iletir() {
        let g = Sahte::yeni();
        let c = Cati::izinsiz_denetimle(&g);
        let (d, _) = api(
            &c,
            &g,
            "POST",
            "/api/cihaz/onay",
            "",
            &json!({"kimlik": "0123456789abcdef", "sube_id": 4}),
        );
        assert_eq!(d, 200);
        assert_eq!(g.son().2["sube_id"], 4);
        let (d, _) = api(
            &c,
            &g,
            "POST",
            "/api/yetki",
            "",
            &json!({"program": "taarkod", "acik": false, "sube_id": 7}),
        );
        assert_eq!(d, 200);
        assert_eq!(g.son().1, "/v1/program-yetki");
        assert_eq!(g.son().2["acik"], false);
        let (d, _) = api(
            &c,
            &g,
            "POST",
            "/api/kullanici/ekle",
            "",
            &json!({"ad": "kasa04", "gorunen_ad": "Kasa 04", "parola": "uzun-parola-1", "roller": [{"rol": "kasiyer", "sube_id": 4}]}),
        );
        assert_eq!(d, 200);
        assert_eq!(g.son().2["roller"][0]["rol"], "kasiyer");
    }

    #[test]
    fn eksik_girdi_400_ve_bilinmeyen_404() {
        let g = Sahte::yeni();
        let c = Cati::izinsiz_denetimle(&g);
        assert_eq!(
            api(&c, &g, "POST", "/api/cihaz/onay", "", &json!({})).0,
            400
        );
        assert_eq!(
            api(&c, &g, "POST", "/api/yetki", "", &json!({"program": "x"})).0,
            400
        );
        assert_eq!(api(&c, &g, "GET", "/api/yok", "", &Value::Null).0, 404);
        // Yazma uçları GET ile çağrılamaz.
        assert_eq!(
            api(&c, &g, "GET", "/api/cihaz/onay", "", &Value::Null).0,
            404
        );
        assert!(g.istekler.borrow().is_empty());
    }

    #[test]
    fn yetkisiz_oturum_sunucuya_gitmez() {
        let g = Sahte::yeni();
        let c = Cati::yeni(&g, &["gunluk.gor".to_string()]);
        let (d, v) = api(
            &c,
            &g,
            "POST",
            "/api/cihaz/iptal",
            "",
            &json!({"kimlik": "0123456789abcdef"}),
        );
        assert_eq!(d, 403);
        assert_eq!(v["kod"], 7);
        assert!(g.istekler.borrow().is_empty());
    }

    #[test]
    fn yeniden_dogrulama_428() {
        let mut g = Sahte::yeni();
        g.hata = Some(GonderHata::Reddedildi {
            kod: 401,
            mesaj: "yeniden_dogrula".into(),
        });
        let c = Cati::izinsiz_denetimle(&g);
        let (d, v) = api(
            &c,
            &g,
            "POST",
            "/api/cihaz/iptal",
            "",
            &json!({"kimlik": "0123456789abcdef"}),
        );
        assert_eq!(d, 428);
        assert_eq!(v["kod"], 6);
    }

    #[test]
    fn parola_yanitta_geri_donmez() {
        let mut g = Sahte::yeni();
        g.hata = Some(GonderHata::Reddedildi {
            kod: 400,
            mesaj: "kötü".into(),
        });
        let c = Cati::izinsiz_denetimle(&g);
        let (_, v) = api(
            &c,
            &g,
            "POST",
            "/api/dogrula",
            "",
            &json!({"parola": "gizli-parola-123"}),
        );
        assert!(!v.to_string().contains("gizli-parola-123"));
    }
}
