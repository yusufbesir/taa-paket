// Sahip: Yusuf. yazan: [Claude#taa-cati-S3b 2026-11-06]
//! taa-cati testleri: sahte gönderici ile uç/gövde/sorgu, girdi doğrulama, varsayılan reddet, 401 yeniden_dogrula.

use crate::*;
use serde_json::json;
use std::cell::RefCell;

struct Sahte {
    istekler: RefCell<Vec<(String, String, Value)>>,
    yanit: Value,
    hata: Option<GonderHata>,
}

impl Sahte {
    fn yeni(yanit: Value) -> Sahte {
        Sahte {
            istekler: RefCell::new(Vec::new()),
            yanit,
            hata: None,
        }
    }
    fn hatali(h: GonderHata) -> Sahte {
        Sahte {
            istekler: RefCell::new(Vec::new()),
            yanit: Value::Null,
            hata: Some(h),
        }
    }
    fn son(&self) -> (String, String, Value) {
        self.istekler.borrow().last().cloned().expect("istek yok")
    }
}

impl Gonderici for Sahte {
    fn gonder(&self, uc: &str, govde: &Value) -> Result<Value, GonderHata> {
        self.istekler
            .borrow_mut()
            .push(("POST".into(), uc.into(), govde.clone()));
        self.hata.clone().map_or(Ok(self.yanit.clone()), Err)
    }
    fn al(&self, uc: &str) -> Result<Value, GonderHata> {
        self.istekler
            .borrow_mut()
            .push(("GET".into(), uc.into(), Value::Null));
        self.hata.clone().map_or(Ok(self.yanit.clone()), Err)
    }
}

fn dizi(x: &[&str]) -> Vec<String> {
    x.iter().map(|s| s.to_string()).collect()
}

#[test]
fn okuma_uclari_ve_sorgu() {
    let s = Sahte::yeni(json!([{"id": 1}]));
    let c = Cati::izinsiz_denetimle(&s);
    assert_eq!(c.kullanicilar().unwrap().len(), 1);
    assert_eq!(s.son().1, "/v1/kullanici");
    c.roller().unwrap();
    assert_eq!(s.son().1, "/v1/rol");
    c.izinler().unwrap();
    assert_eq!(s.son().1, "/v1/izin");
    c.cihazlar(None).unwrap();
    assert_eq!(s.son().1, "/v1/cihaz");
    c.cihazlar(Some("bekliyor")).unwrap();
    assert_eq!(s.son().1, "/v1/cihaz?durum=bekliyor");
    assert!(matches!(
        c.cihazlar(Some("x")),
        Err(CatiHata::GecersizGirdi(_))
    ));
    c.program_yetkileri().unwrap();
    assert_eq!(s.son().1, "/v1/program-yetki");
    c.subeler().unwrap();
    assert_eq!(s.son().1, "/v1/sube");
    let f = OlaySuzgeci {
        islem: Some("giris".into()),
        kullanici_id: Some(3),
        sube_id: Some(2),
        tarih_bas: Some("2026-11-01".into()),
        tarih_bit: Some("2026-11-06T23:59:59Z".into()),
        limit: Some(50),
        onceki_id: Some(900),
    };
    c.olaylar(&f).unwrap();
    assert_eq!(
        s.son().1,
        "/v1/olay?islem=giris&kullanici_id=3&sube_id=2&tarih_bas=2026-11-01&tarih_bit=2026-11-06T23:59:59Z&limit=50&onceki_id=900"
    );
    // hepsi GET
    assert!(s.istekler.borrow().iter().all(|(y, _, _)| y == "GET"));
    // varsayılan limit; geçersizler
    assert_eq!(OlaySuzgeci::default().sorgu().unwrap(), "?limit=100");
    assert!(OlaySuzgeci {
        limit: Some(0),
        ..Default::default()
    }
    .sorgu()
    .is_err());
    assert!(OlaySuzgeci {
        limit: Some(501),
        ..Default::default()
    }
    .sorgu()
    .is_err());
    assert!(OlaySuzgeci {
        tarih_bas: Some("dün".into()),
        ..Default::default()
    }
    .sorgu()
    .is_err());
    assert!(OlaySuzgeci {
        islem: Some("a b".into()),
        ..Default::default()
    }
    .sorgu()
    .is_err());
    // sorgu, taa-kuyruk doğrulamasından geçer
    assert!(taa_kuyruk::uc_sorgulu_gecerli(&format!(
        "/v1/olay{}",
        f.sorgu().unwrap()
    )));
    // dizi olmayan yanıt
    let s2 = Sahte::yeni(json!({"a": 1}));
    assert!(matches!(
        Cati::izinsiz_denetimle(&s2).kullanicilar(),
        Err(CatiHata::BozukYanit(_))
    ));
}

#[test]
fn yazma_uclari_ve_govdeler() {
    let s = Sahte::yeni(json!({"id": 7}));
    let c = Cati::izinsiz_denetimle(&s);
    let r = vec![
        RolAtama::yeni("kasa", Some(2)).unwrap(),
        RolAtama::yeni("personel", None).unwrap(),
    ];
    c.kullanici_ekle("ayse", "Ayşe Yılmaz", "cok-gizli-parola", &r)
        .unwrap();
    let (y, uc, g) = s.son();
    assert_eq!((y.as_str(), uc.as_str()), ("POST", "/v1/kullanici"));
    assert_eq!(
        g,
        json!({"ad": "ayse", "gorunen_ad": "Ayşe Yılmaz", "parola": "cok-gizli-parola",
               "roller": [{"rol": "kasa", "sube_id": 2}, {"rol": "personel"}]})
    );
    c.rol_ver(7, &r[0]).unwrap();
    assert_eq!(s.son().1, "/v1/kullanici/7/rol");
    assert_eq!(s.son().2, json!({"rol": "kasa", "sube_id": 2}));
    c.rol_kaldir(7, &r[1]).unwrap();
    assert_eq!(s.son().1, "/v1/kullanici/7/rol-kaldir");
    c.kullanici_kapat(7).unwrap();
    assert_eq!(s.son().1, "/v1/kullanici/7/kapat");
    assert_eq!(s.son().2, json!({}));
    c.parola_degistir(7, "yeni-parola-1").unwrap();
    assert_eq!(s.son().1, "/v1/kullanici/7/parola");
    assert_eq!(s.son().2, json!({"parola": "yeni-parola-1"}));
    c.cihaz_onayla("0123456789abcdef", Some(3)).unwrap();
    assert_eq!(s.son().1, "/v1/cihaz/0123456789abcdef/onay");
    assert_eq!(s.son().2, json!({"sube_id": 3}));
    c.cihaz_onayla("0123456789abcdef", None).unwrap();
    assert_eq!(s.son().2, json!({}));
    c.cihaz_iptal("0123456789abcdef").unwrap();
    assert_eq!(s.son().1, "/v1/cihaz/0123456789abcdef/iptal");
    c.program_yetki_ayarla(Some(2), Some("kasa"), "tutaa", false)
        .unwrap();
    assert_eq!(s.son().1, "/v1/program-yetki");
    assert_eq!(
        s.son().2,
        json!({"sube_id": 2, "rol": "kasa", "program": "tutaa", "acik": false})
    );
    c.program_yetki_ayarla(None, None, "taa-cati", true)
        .unwrap();
    assert_eq!(s.son().2, json!({"program": "taa-cati", "acik": true}));
    c.sube_ekle("pnd", "Pendik").unwrap();
    assert_eq!(s.son().1, "/v1/sube");
    assert_eq!(s.son().2, json!({"kod": "pnd", "ad": "Pendik"}));
    c.yeniden_dogrula("parola-123").unwrap();
    assert_eq!(s.son().1, "/v1/yeniden-dogrula");
    assert_eq!(s.son().2, json!({"parola": "parola-123"}));
}

#[test]
fn girdi_dogrulama_sunucuya_gitmez() {
    let s = Sahte::yeni(json!({}));
    let c = Cati::izinsiz_denetimle(&s);
    let r = vec![RolAtama::yeni("kasa", None).unwrap()];
    let gec = |x: Result<Value, CatiHata>| matches!(x, Err(CatiHata::GecersizGirdi(_)));
    assert!(gec(c.kullanici_ekle("Ayşe", "A", "cok-gizli-parola", &r)));
    assert!(gec(c.kullanici_ekle("ayse", "", "cok-gizli-parola", &r)));
    assert!(gec(c.kullanici_ekle("ayse", "A", "kisa", &r)));
    assert!(gec(c.kullanici_ekle("ayse", "A", "cok-gizli-parola", &[])));
    assert!(RolAtama::yeni("Yonetici", None).is_err());
    assert!(RolAtama::yeni("", None).is_err());
    assert!(gec(c.parola_degistir(1, "kisa")));
    assert!(gec(c.cihaz_onayla("kisa", None)));
    assert!(gec(c.cihaz_onayla("0123456789ABCDEF", None)));
    assert!(gec(c.cihaz_iptal("0123456789abcdef0")));
    assert!(gec(c.program_yetki_ayarla(None, None, "../x", true)));
    assert!(gec(c.program_yetki_ayarla(
        None,
        Some("a b"),
        "tutaa",
        true
    )));
    assert!(gec(c.sube_ekle("PND", "Pendik")));
    assert!(gec(c.sube_ekle("pnd", "a\nb")));
    assert!(gec(c.yeniden_dogrula("")));
    assert!(gec(c.yeniden_dogrula("a\nb")));
    assert!(
        s.istekler.borrow().is_empty(),
        "geçersiz girdi sunucuya gitmemeli"
    );
    assert!(tanitici_gecerli("sube-muduru"));
    assert!(tanitici_gecerli("fis.sil"));
    assert!(!tanitici_gecerli("-x"));
    assert!(!tanitici_gecerli(&"a".repeat(65)));
    assert!(kimlik_gecerli("0123456789abcdef"));
    assert!(!kimlik_gecerli("0123456789abcde"));
}

#[test]
fn varsayilan_reddet_oturum_izinleri() {
    let s = Sahte::yeni(json!([]));
    // yalnız günlük izni: kullanıcı listesi yerelde reddedilir, günlük gider
    let c = Cati::yeni(&s, &dizi(&["gunluk.gor"]));
    assert!(matches!(
        c.kullanicilar(),
        Err(CatiHata::Yetkisiz { izin }) if izin == "kullanici.yonet"
    ));
    assert!(matches!(
        c.cihaz_iptal("0123456789abcdef"),
        Err(CatiHata::Yetkisiz { izin }) if izin == "cihaz.yonet"
    ));
    assert!(matches!(
        c.program_yetki_ayarla(None, None, "tutaa", true),
        Err(CatiHata::Yetkisiz { .. })
    ));
    assert!(s.istekler.borrow().is_empty());
    c.olaylar(&OlaySuzgeci::default()).unwrap();
    assert_eq!(s.istekler.borrow().len(), 1);
    // rol adı tek başına izin vermez: izin listesi boş yönetici de reddedilir (K1); liste tam gelince geçer
    let bos = Cati::yeni(&s, &[]);
    assert!(matches!(bos.kullanicilar(), Err(CatiHata::Yetkisiz { .. })));
    assert!(!bos.izinli("yok.islem"));
    let cy = Cati::yeni(
        &s,
        &dizi(&[
            "kullanici.yonet",
            "cihaz.yonet",
            "program.yonet",
            "gunluk.gor",
        ]),
    );
    cy.kullanicilar().unwrap();
    cy.cihazlar(None).unwrap();
    assert!(cy.izinli("program.yonet"));
    assert!(!cy.izinli("yok.islem"));
    // çıkış kodları
    assert_eq!(hata_kodu(&CatiHata::Yetkisiz { izin: "x".into() }), 7);
    assert_eq!(hata_kodu(&CatiHata::YenidenDogrula), 6);
    assert_eq!(hata_kodu(&CatiHata::GecersizGirdi(String::new())), 2);
    assert_eq!(hata_kodu(&CatiHata::Kimlik(KimlikHata::OturumYok)), 5);
}

#[test]
fn sunucu_hatalari_eslenir() {
    let s = Sahte::hatali(GonderHata::Reddedildi {
        kod: 401,
        mesaj: "yeniden_dogrula: parola doğrulaması gerekli".into(),
    });
    let c = Cati::izinsiz_denetimle(&s);
    let e = c.kullanici_kapat(1).unwrap_err();
    assert!(matches!(e, CatiHata::YenidenDogrula));
    assert_eq!(hata_kodu(&e), 6);
    assert!(e.to_string().contains("taa-cati dogrula"));

    let s = Sahte::hatali(GonderHata::Reddedildi {
        kod: 403,
        mesaj: "yetki_yok".into(),
    });
    let e = Cati::izinsiz_denetimle(&s).kullanici_kapat(1).unwrap_err();
    assert!(matches!(
        e,
        CatiHata::Sunucu(GonderHata::Reddedildi { kod: 403, .. })
    ));
    assert_eq!(hata_kodu(&e), 4);

    let s = Sahte::hatali(GonderHata::Ulasilamadi("ağ yok".into()));
    let e = Cati::izinsiz_denetimle(&s).subeler().unwrap_err();
    assert_eq!(hata_kodu(&e), 3);
}

#[test]
#[cfg(not(windows))]
fn yonetici_baglantisi_oturum_ve_cihaz_ister() {
    let kok = std::env::temp_dir().join(format!("taa-cati-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&kok);
    std::fs::create_dir_all(&kok).unwrap();
    // oturum yok → 5
    let e = yonetici_baglantisi(&kok, "http://127.0.0.1:1", None).unwrap_err();
    assert_eq!(hata_kodu(&e), 5, "{e}");
    // oturum var (dosya deposu, Windows dışı), cihaz anahtarı yok → hata (cihaz uret önerisi)
    let klasor = kok.join("kimlik");
    std::fs::create_dir_all(&klasor).unwrap();
    let dosya = klasor.join(format!(
        "oturum-{}.json",
        taa_kimlik::sunucu_anahtari("http://127.0.0.1:1")
    ));
    std::fs::write(
        &dosya,
        json!({"sunucu": "http://127.0.0.1:1", "kullanici": "ali", "roller": ["yonetici"],
               "oturum": "test-anahtar"})
        .to_string(),
    )
    .unwrap();
    let e = yonetici_baglantisi(&kok, "http://127.0.0.1:1", None).unwrap_err();
    assert!(e.to_string().contains("cihaz anahtarı yok"), "{e}");
    let _ = std::fs::remove_dir_all(&kok);
}
