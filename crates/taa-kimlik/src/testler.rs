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
    // Rol adı izin vermez (K1): izin listesi olmayan yönetici de reddedilir; eski sunucu (liste yok) programı açık sayar.
    let y = giris(
        &Sahte::yeni(Ok(json!({"oturum": "t", "roller": ["yonetici"]}))),
        "https://s",
        "v",
        "p",
        None,
    )
    .unwrap();
    assert!(!y.izinli("her.sey") && !y.izinli("yok.islem") && y.program_acik("x"));
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

#[test]
fn yetki_belgesi_imza_sure_ve_dosya() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    // Sunucu anahtarı: özel geçici, genel <kök>/anahtarlar/sunucu.pub.
    let ozel_yol = kok.join("gizli").join("sunucu.key");
    let genel_yol = kok.join("anahtarlar").join("sunucu.pub");
    let genel = taa_imza::anahtar_uret(&ozel_yol, &genel_yol).unwrap();
    let (ozel, _) = taa_imza::OzelAnahtar::dosyadan(&ozel_yol).unwrap();
    assert_eq!(sunucu_anahtari_oku(&kok).unwrap(), Some(genel.clone()));
    let mut belge = json!({
        "kullanici": "ali", "sube_id": 3, "roller": ["kasa"], "izinler": ["kasa.kapat"],
        "programlar": ["tutaa"], "gecerlilik": "2026-11-13T00:00:00Z"
    });
    taa_imza::yayin_imzala(&mut belge, &ozel).unwrap();
    // İmzalı giriş yanıtı: oturum da imzanın içinde.
    let mut giris_yaniti = belge.clone();
    giris_yaniti.as_object_mut().unwrap().remove("imza");
    giris_yaniti["oturum"] = json!("tok");
    taa_imza::yayin_imzala(&mut giris_yaniti, &ozel).unwrap();
    let o = giris_dogrulayarak(
        &Sahte::yeni(Ok(giris_yaniti.clone())),
        "https://s",
        "ali",
        "p",
        None,
        Some(&genel),
    )
    .unwrap();
    assert_eq!(o.izinler, vec!["kasa.kapat"]);
    // Kurcalanmış giriş yanıtı reddedilir; imzasız da reddedilir; anahtar yokken kabul edilir.
    let mut kurcalanmis = giris_yaniti.clone();
    kurcalanmis["roller"] = json!(["yonetici"]);
    assert!(matches!(
        giris_dogrulayarak(
            &Sahte::yeni(Ok(kurcalanmis.clone())),
            "https://s",
            "ali",
            "p",
            None,
            Some(&genel)
        ),
        Err(KimlikHata::BozukYanit(_))
    ));
    let mut imzasiz = giris_yaniti.clone();
    imzasiz.as_object_mut().unwrap().remove("imza");
    assert!(giris_dogrulayarak(
        &Sahte::yeni(Ok(imzasiz.clone())),
        "https://s",
        "ali",
        "p",
        None,
        Some(&genel)
    )
    .is_err());
    assert!(giris(&Sahte::yeni(Ok(imzasiz)), "https://s", "ali", "p", None).is_ok());
    // yetki_al: belge dosyaya yazılır, oturum içeren belge reddedilir.
    assert!(
        yetki_oku(&kok, "https://s", Some(&genel), None).is_err(),
        "dosya yokken yetki yok"
    );
    let y = yetki_al(
        &Sahte::yeni(Ok(belge.clone())),
        &kok,
        "https://s",
        Some(&genel),
    )
    .unwrap();
    assert!(
        y.dogrulandi && y.izinli("kasa.kapat") && !y.izinli("fis.sil") && y.program_acik("tutaa")
    );
    assert!(yetki_al(
        &Sahte::yeni(Ok(giris_yaniti)),
        &kok,
        "https://s",
        Some(&genel)
    )
    .is_err());
    let yol = crate::yetki::yetki_dosyasi(&kok, "https://s");
    assert!(yol.is_file());
    assert!(!std::fs::read_to_string(&yol).unwrap().contains("tok"));
    // yetki_oku: geçerli, süresi geçmiş, kurcalanmış.
    let g = yetki_oku(
        &kok,
        "https://s",
        Some(&genel),
        Some("2026-11-12T23:59:59Z"),
    )
    .unwrap();
    assert_eq!(g.kullanici, "ali");
    assert!(matches!(
        yetki_oku(
            &kok,
            "https://s",
            Some(&genel),
            Some("2026-11-13T00:00:00Z")
        ),
        Err(KimlikHata::YetkiYok(_))
    ));
    let mut sarmal: Value = serde_json::from_str(&std::fs::read_to_string(&yol).unwrap()).unwrap();
    sarmal["belge"]["izinler"] = json!(["fis.sil"]);
    std::fs::write(&yol, sarmal.to_string()).unwrap();
    assert!(
        yetki_oku(
            &kok,
            "https://s",
            Some(&genel),
            Some("2026-11-10T00:00:00Z")
        )
        .is_err(),
        "kurcalanmış belge"
    );
    // Anahtar yokken: dogrulandi=false, gecerlilik yoksa alınma+7 gün.
    let kok2 = taa_dosya::gecici_klasor().unwrap();
    assert!(sunucu_anahtari_oku(&kok2).unwrap().is_none());
    let y2 = yetki_al(
        &Sahte::yeni(Ok(
            json!({"kullanici": "veli", "roller": ["yonetici"], "izinler": ["her.sey"]}),
        )),
        &kok2,
        "https://s",
        None,
    )
    .unwrap();
    assert!(!y2.dogrulandi && y2.izinli("her.sey") && !y2.izinli("yok.islem"));
    assert_eq!(y2.gecerlilik, crate::yetki::gun_ekle(&y2.alinma, 7));
    assert!(yetki_oku(&kok2, "https://s", None, None).is_ok());
    // Bozuk sunucu.pub sessizce atlanmaz.
    std::fs::create_dir_all(kok2.join("anahtarlar")).unwrap();
    std::fs::write(kok2.join("anahtarlar/sunucu.pub"), "bozuk").unwrap();
    assert!(sunucu_anahtari_oku(&kok2).is_err());
}
