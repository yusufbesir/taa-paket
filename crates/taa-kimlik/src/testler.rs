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
    let o2 = giris(&g2, "https://s", "ali", "p", Some("  ")).unwrap();
    assert!(g2.cagrilar.borrow()[0].1.get("kod").is_none());
    assert_eq!(o.uyari_sayisi, 0);
    assert_eq!(o2.uyari_sayisi, 0, "eski sunucu: uyarı alanı yok");
    // KM2: girişte görülmemiş hesap uyarısı sayısı okunur; depoya yazılmaz.
    let g3 = Sahte::yeni(Ok(json!({"oturum": "y", "uyari_sayisi": 3})));
    let o3 = giris(&g3, "https://s", "ali", "p", None).unwrap();
    assert_eq!(o3.uyari_sayisi, 3);
    assert!(o3.ozet().get("uyari_sayisi").is_none());
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
    // KM2: kilit, sınır ve onay bekleme durumları anlaşılır iletiyle döner; 429 "sunucu kapalı" gibi görünmez.
    let kilit = Sahte::yeni(Err(GonderHata::Reddedildi {
        kod: 423,
        mesaj: r#"{"hata":"hesap_kilitli"}"#.into(),
    }));
    let e = giris(&kilit, "https://s", "ali", "p", None).unwrap_err();
    assert!(e.to_string().contains("Yöneticiye başvur"), "{e}");
    let sinir = Sahte::yeni(Err(GonderHata::Ulasilamadi(
        r#"HTTP 429: {"hata":"cihaz_bekliyor"}. Öneri: sunucu toparlanınca yeniden deneyin."#
            .into(),
    )));
    match giris(&sinir, "https://s", "ali", "p", None) {
        Err(e @ KimlikHata::GirisReddi { kod: 429, .. }) => {
            assert!(e.to_string().contains("bir saat sonra"), "{e}")
        }
        d => panic!("{d:?}"),
    }
    let bekliyor = Sahte::yeni(Err(GonderHata::Reddedildi {
        kod: 403,
        mesaj: r#"{"hata":"hesap_onay_bekliyor"}"#.into(),
    }));
    assert!(giris(&bekliyor, "https://s", "ali", "p", None)
        .unwrap_err()
        .to_string()
        .contains("onayı bekliyor"));
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

#[test]
fn yoneticiye_basvuru() {
    let g = Sahte::yeni(Ok(json!({"durum": "iletildi", "dogrulama_sayisi": "07"})));
    assert_eq!(
        yoneticiye_basvur(&g, "ali", Some("Yeni-Sifre-2026")).unwrap(),
        "07",
        "ekranda gösterilecek sayı"
    );
    yoneticiye_basvur(&g, "ali", None).unwrap();
    yoneticiye_basvur(&g, "ali", Some("")).unwrap();
    let c = g.cagrilar.borrow();
    assert_eq!(
        c[0],
        (
            BASVURU_UCU.to_string(),
            json!({"kullanici": "ali", "yeni_parola": "Yeni-Sifre-2026"})
        )
    );
    assert_eq!(c[1].1, json!({"kullanici": "ali"}));
    assert_eq!(
        c[2].1,
        json!({"kullanici": "ali"}),
        "boş şifre gönderilmez (yalnız kilit başvurusu)"
    );
    assert!(matches!(
        yoneticiye_basvur(&g, "", None),
        Err(KimlikHata::GecersizGirdi(_))
    ));
    assert!(matches!(
        yoneticiye_basvur(&g, "ali", Some("a\nb")),
        Err(KimlikHata::GecersizGirdi(_))
    ));
    let eski = Sahte::yeni(Ok(json!({"durum": "iletildi"})));
    assert!(
        matches!(
            yoneticiye_basvur(&eski, "ali", None),
            Err(KimlikHata::Baglanti(_))
        ),
        "sayısız yanıt sessizce kabul edilmez"
    );
    let sinir = Sahte::yeni(Err(GonderHata::Ulasilamadi(
        "HTTP 429: {\"hata\":\"basvuru_sinir\"}".into(),
    )));
    assert!(matches!(
        yoneticiye_basvur(&sinir, "ali", None),
        Err(KimlikHata::GirisReddi { kod: 429, .. })
    ));
}

// ---- Fable D1: durum saklı oturumu sunucuya sormadan kabul etmez ----
fn d1_oturum(gecerlilik: &str) -> Oturum {
    Oturum {
        sunucu: "https://s".into(),
        kullanici: "ayse".into(),
        sube_id: json!(1),
        roller: vec!["personel".into()],
        izinler: vec![],
        programlar: vec!["terzitakip".into()],
        gecerlilik: gecerlilik.into(),
        zaman: "2026-11-25T10:00:00Z".into(),
        uyari_sayisi: 0,
        oturum: "gizli".into(),
    }
}

fn d1_yetki(programlar: &[&str]) -> Yetki {
    Yetki {
        kullanici: "ayse".into(),
        sube_id: json!(1),
        roller: vec!["personel".into()],
        izinler: vec![],
        programlar: programlar.iter().map(|p| p.to_string()).collect(),
        gecerlilik: "2026-12-02T10:00:00Z".into(),
        alinma: "2026-11-25T10:00:00Z".into(),
        dogrulandi: true,
    }
}

#[test]
fn d1_canli_yanit_programlari_tazeler() {
    let o = d1_oturum("2026-11-26T00:00:00Z");
    let t = oturum_dogrula(&o, "2026-11-25T12:00:00Z", || Ok(d1_yetki(&["tutaa"]))).unwrap();
    assert_eq!(
        t.programlar,
        vec!["tutaa".to_string()],
        "yetki kalktıysa program listesi düşer"
    );
    assert_eq!(t.gecerlilik, "2026-12-02T10:00:00Z");
    assert_eq!(t.anahtar(), "gizli");
}

#[test]
fn d1_sunucu_oturumu_reddederse_silinir() {
    let o = d1_oturum("2026-11-26T00:00:00Z");
    let red = |m: &str| {
        let m = m.to_string();
        oturum_dogrula(&o, "2026-11-25T12:00:00Z", move || {
            Err(KimlikHata::GirisReddi { kod: 401, mesaj: m })
        })
    };
    assert!(matches!(
        red(r#"{"hata":"oturum","ileti":"Oturum geçersiz ya da süresi doldu"}"#),
        Err((true, _))
    ));
    assert!(
        matches!(red(r#"{"hata":"cihaz_yok"}"#), Err((false, _))),
        "cihaz reddi oturumu silmez"
    );
    let r = oturum_dogrula(&o, "2026-11-25T12:00:00Z", || {
        Err(KimlikHata::GirisReddi {
            kod: 403,
            mesaj: "cihaz_onaysiz".into(),
        })
    });
    assert!(matches!(r, Err((false, _))));
}

#[test]
fn d1_cevrimdisi_yalniz_gecerlilik_sonuna_kadar() {
    let ulasilamadi = || Err(KimlikHata::Baglanti("bağlanılamadı".into()));
    let o = d1_oturum("2026-11-26T00:00:00Z");
    assert!(oturum_dogrula(&o, "2026-11-25T12:00:00Z", ulasilamadi).is_ok());
    assert!(matches!(
        oturum_dogrula(&o, "2026-11-26T00:00:00Z", ulasilamadi),
        Err((false, _))
    ));
    assert!(matches!(
        oturum_dogrula(&o, "2026-11-27T00:00:00Z", ulasilamadi),
        Err((false, _))
    ));
    let eski = d1_oturum("");
    assert!(
        matches!(
            oturum_dogrula(&eski, "2026-11-25T12:00:00Z", ulasilamadi),
            Err((false, _))
        ),
        "geçerlilik bilinmiyorsa çevrimdışı kabul yok"
    );
}

#[test]
fn e3_ret_turu_ileti_metnine_degil_hata_koduna_bakar() {
    let o = d1_oturum("2026-11-26T00:00:00Z");
    let r = oturum_dogrula(&o, "2026-11-25T12:00:00Z", || {
        Err(KimlikHata::GirisReddi {
            kod: 401,
            mesaj: r#"HTTP 401 {"hata":"oturum","ileti":"cihaz için oturum süresi doldu"}"#.into(),
        })
    });
    assert!(
        matches!(r, Err((true, _))),
        "ileti 'cihaz' içerse de kod oturum"
    );
    let r = oturum_dogrula(&o, "2026-11-25T12:00:00Z", || {
        Err(KimlikHata::GirisReddi {
            kod: 401,
            mesaj: "gövdesiz ret".into(),
        })
    });
    assert!(matches!(r, Err((false, _))), "kod yoksa silinmez");
    assert_eq!(
        sunucu_hata_kodu(r#"x {"hata":"cihaz_yok"} y"#).as_deref(),
        Some("cihaz_yok")
    );
    assert_eq!(sunucu_hata_kodu("yok"), None);
}

#[test]
fn e1_yanit_vermeyen_sunucuda_butce_icinde_cevrimdisi_kabul() {
    // Bağlantıyı kabul edip hiç yanıt vermeyen sunucu.
    let dinle = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let adres = format!("http://{}", dinle.local_addr().unwrap());
    let tut = std::thread::spawn(move || {
        let mut acik = Vec::new();
        for _ in 0..2 {
            if let Ok((s, _)) = dinle.accept() {
                acik.push(s);
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    });
    let o = d1_oturum("2999-01-01T00:00:00Z");
    let bas = std::time::Instant::now();
    let kok = std::env::temp_dir().join(format!("taa-kimlik-e1-{}", std::process::id()));
    let t = oturum_dogrula(&o, "2026-11-25T12:00:00Z", move || {
        butceli(std::time::Duration::from_secs(1), move || {
            let g = CurlGonderici::yeni(&adres)
                .and_then(|g| g.oturum("gizli"))
                .map_err(|e| KimlikHata::Baglanti(e.to_string()))?;
            yetki_al(&g, &kok, "https://s", None)
        })
    });
    assert!(
        bas.elapsed() < std::time::Duration::from_secs(3),
        "bütçe aşılmadı: {:?}",
        bas.elapsed()
    );
    assert_eq!(
        t.unwrap().programlar,
        vec!["terzitakip".to_string()],
        "çevrimdışı geçerli oturum korunur"
    );
    drop(tut);
}
