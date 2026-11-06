// Sahip: Yusuf. yazan: [Claude#taa-kimlik-T7 2026-11-06]
//! taa-kimlik testleri: giriş sözleşmesi, dosya deposu, anahtarın çıktıya sızmaması, kuyruğu oturumla gönderme.

use super::*;
use std::cell::RefCell;

struct Sahte {
    yanit: Result<Value, GonderHata>,
    cagrilar: RefCell<Vec<(String, Value)>>,
}

impl Sahte {
    fn yeni(yanit: Result<Value, GonderHata>) -> Sahte {
        Sahte {
            yanit,
            cagrilar: RefCell::new(Vec::new()),
        }
    }
}

impl Gonderici for Sahte {
    fn gonder(&self, uc: &str, govde: &Value) -> Result<Value, GonderHata> {
        self.cagrilar
            .borrow_mut()
            .push((uc.to_string(), govde.clone()));
        self.yanit.clone()
    }
}

fn gecici() -> PathBuf {
    taa_dosya::gecici_klasor().unwrap()
}

#[test]
fn giris_sozlesmesi() {
    let g = Sahte::yeni(Ok(
        json!({"oturum": "abc.def", "sube_id": 3, "roller": ["kasa", "satis"]}),
    ));
    let o = giris(&g, "https://sunucu:8443/", "ali", "gizli", Some("123456")).unwrap();
    let c = g.cagrilar.borrow();
    assert_eq!(c[0].0, GIRIS_UCU);
    assert_eq!(
        c[0].1,
        json!({"kullanici": "ali", "parola": "gizli", "kod": "123456"})
    );
    assert_eq!(o.sunucu, "https://sunucu:8443");
    assert_eq!(o.kullanici, "ali");
    assert_eq!(o.sube_id, json!(3));
    assert_eq!(o.roller, vec!["kasa", "satis"]);
    assert_eq!(o.anahtar(), "abc.def");
    // Anahtar Debug/özet çıktısında yok.
    let d = format!("{o:?}");
    assert!(!d.contains("abc.def") && d.contains("<gizli>"));
    assert!(!o.ozet().to_string().contains("abc.def"));
    // kod boşsa gönderilmez
    let g2 = Sahte::yeni(Ok(json!({"oturum": "x"})));
    giris(&g2, "https://s", "ali", "p", Some("  ")).unwrap();
    assert!(g2.cagrilar.borrow()[0].1.get("kod").is_none());
}

#[test]
fn giris_hatalari() {
    let g = Sahte::yeni(Ok(json!({"oturum": "x"})));
    assert!(matches!(
        giris(&g, "https://s", "", "p", None),
        Err(KimlikHata::GecersizGirdi(_))
    ));
    assert!(matches!(
        giris(&g, "https://s", "a li", "p", None),
        Err(KimlikHata::GecersizGirdi(_))
    ));
    assert!(matches!(
        giris(&g, "https://s", "ali", "", None),
        Err(KimlikHata::GecersizGirdi(_))
    ));
    let ret = Sahte::yeni(Err(GonderHata::Reddedildi {
        kod: 401,
        mesaj: "yanlış parola".into(),
    }));
    match giris(&ret, "https://s", "ali", "p", None) {
        Err(KimlikHata::GirisReddi { kod: 401, mesaj }) => assert_eq!(mesaj, "yanlış parola"),
        d => panic!("{d:?}"),
    }
    let yok = Sahte::yeni(Err(GonderHata::Ulasilamadi("ağ yok".into())));
    assert!(matches!(
        giris(&yok, "https://s", "ali", "p", None),
        Err(KimlikHata::Baglanti(_))
    ));
    let bozuk = Sahte::yeni(Ok(json!({"token": "x"})));
    assert!(matches!(
        giris(&bozuk, "https://s", "ali", "p", None),
        Err(KimlikHata::BozukYanit(_))
    ));
    let bozuk2 = Sahte::yeni(Ok(json!({"oturum": "a\nb"})));
    assert!(matches!(
        giris(&bozuk2, "https://s", "ali", "p", None),
        Err(KimlikHata::BozukYanit(_))
    ));
}

#[test]
fn sunucu_anahtari_bicimi() {
    assert_eq!(
        sunucu_anahtari("https://192.168.1.10:8443/"),
        "192.168.1.10_8443"
    );
    assert_eq!(sunucu_anahtari("HTTP://Sunucu.Yerel"), "sunucu.yerel");
    assert_eq!(sunucu_anahtari(""), "sunucu");
}

#[test]
fn dosya_deposu_kaydet_oku_cikis() {
    let kok = gecici();
    let depo = DosyaDeposu::kokten(&kok);
    assert!(matches!(depo.oku("https://s"), Err(KimlikHata::OturumYok)));
    let g = Sahte::yeni(Ok(
        json!({"oturum": "tok", "sube_id": "S1", "roller": ["kasa"]}),
    ));
    let o = giris(&g, "https://s", "ali", "p", None).unwrap();
    depo.kaydet(&o).unwrap();
    let yol = kok.join("kimlik").join("oturum-s.json");
    assert!(yol.is_file());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&yol).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    let okunan = depo.oku("https://s/").unwrap();
    assert_eq!(okunan, o);
    assert_eq!(okunan.anahtar(), "tok");
    depo.cikis("https://s").unwrap();
    assert!(matches!(depo.oku("https://s"), Err(KimlikHata::OturumYok)));
    assert!(yol.is_file(), "dosya silinmez, boşaltılır");
    depo.cikis("https://baska").unwrap();
}

#[test]
fn kuyrugu_oturumla_gonder() {
    // Oturum yoksa gönderim olmaz.
    let kok = gecici();
    let depo = DosyaDeposu::kokten(&kok);
    assert!(matches!(
        kuyrugu_gonder(&depo, "http://127.0.0.1:9", &kok, None, 0),
        Err(KimlikHata::OturumYok)
    ));
    // Oturum var, sunucu kapalı: kuyruk bekler, durma nedeni döner (curl gerekir).
    if std::process::Command::new("curl")
        .arg("--version")
        .output()
        .is_err()
    {
        return;
    }
    let dinle = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let adres = format!("http://{}", dinle.local_addr().unwrap());
    drop(dinle);
    let g = Sahte::yeni(Ok(json!({"oturum": "tok"})));
    depo.kaydet(&giris(&g, &adres, "ali", "p", None).unwrap())
        .unwrap();
    taa_kuyruk::Kuyruk::program(&kok, "taa-fis")
        .unwrap()
        .ekle("/v1/olay", "N-1", json!({}))
        .unwrap();
    let s = kuyrugu_gonder(&depo, &adres, &kok, None, 0).unwrap();
    assert_eq!(s.len(), 1);
    assert_eq!(s[0].0, "taa-fis");
    assert_eq!(s[0].1.kalan, 1);
    assert!(s[0].1.durma_nedeni.is_some());
}

#[test]
fn izinler_programlar_ve_cihaz() {
    let g = Sahte::yeni(Ok(json!({
        "oturum": "t", "roller": ["kasa"], "izinler": ["fis.ekle", "kasa.kapat"],
        "programlar": ["tutaa", "taa-takip"], "gecerlilik": "2026-11-13T00:00:00Z"
    })));
    let o = giris(&g, "https://s", "ali", "p", None).unwrap();
    assert!(o.izinli("kasa.kapat") && !o.izinli("fis.sil"));
    assert!(o.program_acik("tutaa") && !o.program_acik("taa-raporlar"));
    assert_eq!(o.gecerlilik, "2026-11-13T00:00:00Z");
    // Depoya yaz-oku izinleri korur; ozet anahtarı içermez.
    let kok = taa_dosya::gecici_klasor().unwrap();
    let depo = DosyaDeposu::kokten(&kok);
    depo.kaydet(&o).unwrap();
    let geri = depo.oku("https://s").unwrap();
    assert_eq!(geri.izinler, o.izinler);
    assert_eq!(geri.programlar, o.programlar);
    assert!(o.ozet().get("oturum").is_none());
    // Yönetici her izni taşır; eski sunucu (liste yok) her programı açık sayar.
    let y = giris(
        &Sahte::yeni(Ok(json!({"oturum": "t", "roller": ["yonetici"]}))),
        "https://s",
        "v",
        "p",
        None,
    )
    .unwrap();
    assert!(y.izinli("her.sey") && y.program_acik("x"));
    // Cihaz anahtarı yoksa None; üretilince imzalayıcı gelir ve gönderici imzalı olur.
    assert!(cihaz_imzalayici(&kok).unwrap().is_none());
    assert!(!o.gonderici(None).unwrap().imzali_mi());
    taa_cihaz::CihazAnahtari::uret(&kok, Some("test")).unwrap();
    let i = cihaz_imzalayici(&kok).unwrap().expect("cihaz yüklenmeli");
    assert!(o.gonderici_ile(None, Some(i)).unwrap().imzali_mi());
    std::fs::write(kok.join("cihaz/cihaz.key"), "bozuk").unwrap();
    assert!(
        cihaz_imzalayici(&kok).is_err(),
        "bozuk anahtar sessizce atlanmamalı"
    );
}
