// Sahip: Yusuf. yazan: [Claude#dinamik-adres 2026-11-25]

use crate::*;
use std::cell::RefCell;

const TOKEN: &str = "abcd1234-efgh5678";

struct Sahte {
    yanit: Result<(u16, String), String>,
    adresler: RefCell<Vec<String>>,
}

impl Istekci for Sahte {
    fn al(&self, adres: &str) -> Result<(u16, String), String> {
        self.adresler.borrow_mut().push(adres.to_string());
        self.yanit.clone()
    }
}

fn sahte(y: Result<(u16, String), String>) -> Sahte {
    Sahte {
        yanit: y,
        adresler: RefCell::new(vec![]),
    }
}

fn kur(a: &Ayar) -> PathBuf {
    let kok = taa_dosya::gecici_klasor().unwrap();
    ayar_yaz(&kok, a).unwrap();
    std::fs::create_dir_all(kok.join("anahtarlar")).unwrap();
    std::fs::write(token_yolu(&kok), format!("{TOKEN}\n")).unwrap();
    kok
}

#[test]
fn ayar_dogrulama_ve_gidis_donus() {
    let a = Ayar::yeni("duckdns", "TAA-Yusuf.duckdns.org", None).unwrap();
    assert_eq!(a.alan, "taa-yusuf");
    assert_eq!(Ayar::jsondan(&a.json().to_string()).unwrap(), a);
    assert!(Ayar::yeni("duckdns", "a.b", None).is_err());
    assert!(Ayar::yeni("duckdns", "a b", None).is_err());
    assert!(Ayar::yeni("bilinmeyen", "x", None).is_err());
    assert!(Ayar::yeni("url", "x", None).is_err());
    assert!(Ayar::yeni("url", "x", Some("http://h/u?t={token}")).is_err());
    assert!(Ayar::yeni("url", "x", Some("https://h/u?a=1")).is_err()); // {token} yok
    assert!(Ayar::yeni("url", "x", Some("https://h/\"u?t={token}")).is_err());
    let u = Ayar::yeni(
        "url",
        "ev.example.net",
        Some("https://h/u?h={alan}&t={token}&ip={ip}"),
    )
    .unwrap();
    assert_eq!(Ayar::jsondan(&u.json().to_string()).unwrap(), u);
}

#[test]
fn adres_olusturma_ve_giris_denetimi() {
    let a = Ayar::yeni("duckdns", "taa-yusuf", None).unwrap();
    assert_eq!(
        a.adres(TOKEN, None).unwrap(),
        format!("https://www.duckdns.org/update?domains=taa-yusuf&token={TOKEN}&ip=")
    );
    assert!(a
        .adres(TOKEN, Some("1.2.3.4"))
        .unwrap()
        .ends_with("&ip=1.2.3.4"));
    assert!(a.adres(TOKEN, Some("1.2.3.4&x=1")).is_err());
    assert!(a.adres("kisa", None).is_err());
    assert!(a.adres("abc\"def12345", None).is_err());
    assert!(a.adres("abc def12345", None).is_err());
    let u = Ayar::yeni(
        "url",
        "ev.example.net",
        Some("https://h/u?h={alan}&t={token}&ip={ip}"),
    )
    .unwrap();
    assert_eq!(
        u.adres(TOKEN, Some("5.6.7.8")).unwrap(),
        format!("https://h/u?h=ev.example.net&t={TOKEN}&ip=5.6.7.8")
    );
}

#[test]
fn yanit_yorumu() {
    let d = Ayar::yeni("duckdns", "x", None).unwrap();
    assert!(d.yanit_tamam(200, "OK\n1.2.3.4"));
    assert!(!d.yanit_tamam(200, "KO"));
    assert!(!d.yanit_tamam(500, "OK"));
    let u = Ayar::yeni("url", "x", Some("https://h/{token}")).unwrap();
    assert!(u.yanit_tamam(204, ""));
    assert!(!u.yanit_tamam(403, "ok"));
}

#[test]
fn guncelle_basarili_ve_son_kaydi() {
    let a = Ayar::yeni("duckdns", "taa-yusuf", None).unwrap();
    let kok = kur(&a);
    let s = sahte(Ok((200, "OK".into())));
    assert_eq!(guncelle(&kok, None, &s).unwrap(), "taa-yusuf güncellendi");
    assert_eq!(s.adresler.borrow().len(), 1);
    let son = son_oku(&kok).unwrap();
    assert_eq!(son["tamam"], true);
    assert!(!son.to_string().contains(TOKEN));
}

#[test]
fn hata_iletisinde_belirtec_yok() {
    let a = Ayar::yeni("duckdns", "taa-yusuf", None).unwrap();
    let kok = kur(&a);
    let s = sahte(Err(format!(
        "curl: (6) bağlanamadı https://x/?token={TOKEN}"
    )));
    let e = guncelle(&kok, None, &s).unwrap_err().to_string();
    assert!(!e.contains(TOKEN), "{e}");
    let s = sahte(Ok((200, format!("KO {TOKEN}"))));
    let e = guncelle(&kok, None, &s).unwrap_err().to_string();
    assert!(!e.contains(TOKEN), "{e}");
    assert!(!std::fs::read_to_string(kok.join("dns").join("son.json"))
        .unwrap()
        .contains(TOKEN));
}

#[test]
fn ayar_ya_da_belirtec_yoksa_hata() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    let s = sahte(Ok((200, "OK".into())));
    assert!(matches!(guncelle(&kok, None, &s), Err(DnsHata::Dosya(_))));
    ayar_yaz(&kok, &Ayar::yeni("duckdns", "x1", None).unwrap()).unwrap();
    assert!(matches!(guncelle(&kok, None, &s), Err(DnsHata::Dosya(_)))); // belirteç yok
    assert!(s.adresler.borrow().is_empty());
}
