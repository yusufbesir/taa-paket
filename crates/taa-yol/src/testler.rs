// Sahip: Yusuf. yazan: [Claude#taa-yol-S4a 2026-11-07]

use crate::*;
use std::cell::RefCell;

struct Sahte {
    ulasilan: Vec<&'static str>,
    denenen: RefCell<Vec<String>>,
}

impl Deneyici for Sahte {
    fn ulasilabilir(&self, taban: &str, _s: Option<&Path>) -> bool {
        self.denenen.borrow_mut().push(taban.to_string());
        self.ulasilan.iter().any(|u| taban.contains(u))
    }
}

fn ornek() -> Yollar {
    let mut y = Yollar::yeni("merkez").unwrap();
    y.ekle(Yol::yeni("dogrudan", "https://85.1.2.3/", None).unwrap())
        .unwrap();
    y.ekle(Yol::yeni("vpn", "https://10.8.0.1", Some("sunucu-ca.pem")).unwrap())
        .unwrap();
    y.ekle(Yol::yeni("alan-adi", "https://sunucu.ornek.com", Some("sistem")).unwrap())
        .unwrap();
    y
}

#[test]
fn dosya_yaz_oku_ve_dogrulama() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    assert!(yollar_oku(&kok).unwrap().is_none());
    let y = ornek();
    yollar_yaz(&kok, &y).unwrap();
    let geri = yollar_oku(&kok).unwrap().unwrap();
    assert_eq!(geri, y);
    assert_eq!(geri.yollar[0].taban, "https://85.1.2.3"); // sondaki / atıldı
                                                          // sertifika yolu
    assert_eq!(
        geri.yollar[0].sertifika_yolu(&kok),
        Some(kok.join("anahtarlar").join("sunucu-ca.pem"))
    );
    assert_eq!(geri.yollar[2].sertifika_yolu(&kok), None);
    // geçersizler
    assert!(Yol::yeni("Dogrudan", "https://x", None).is_err());
    assert!(Yol::yeni("a", "ftp://x", None).is_err());
    assert!(Yol::yeni("a", "https://x y", None).is_err());
    assert!(Yollar::yeni("Merkez").is_err());
    // bozuk dosya → hata (sessizce yok sayılmaz)
    std::fs::write(dosya_yolu(&kok), "{bozuk").unwrap();
    assert!(matches!(yollar_oku(&kok), Err(YolHata::Dosya(_))));
    // ekle/kaldir/one-al
    let mut y = ornek();
    assert!(y.one_al("alan-adi"));
    assert_eq!(y.yollar[0].ad, "alan-adi");
    assert!(y.kaldir("vpn"));
    assert!(!y.kaldir("vpn"));
    assert_eq!(y.yollar.len(), 2);
    y.ekle(Yol::yeni("dogrudan", "https://1.1.1.1", None).unwrap())
        .unwrap();
    assert_eq!(y.yollar.len(), 2, "aynı ad yerine yazılır");
    assert_eq!(y.yollar[1].taban, "https://1.1.1.1");
}

#[test]
fn secim_sirasi_ve_son_yol() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    let y = ornek();
    // yalnız vpn ulaşılabilir: sırayla dogrudan (başarısız), vpn (tamam); son.json = vpn
    let d = Sahte {
        ulasilan: vec!["10.8.0.1"],
        denenen: RefCell::new(vec![]),
    };
    let v = sec(&kok, &y, &d).unwrap();
    assert_eq!(v.ad, "vpn");
    assert_eq!(
        d.denenen.borrow().as_slice(),
        ["https://85.1.2.3", "https://10.8.0.1"]
    );
    assert_eq!(son_oku(&kok).as_deref(), Some("vpn"));
    // sonraki seçimde önce vpn denenir
    let d2 = Sahte {
        ulasilan: vec!["10.8.0.1", "85.1.2.3"],
        denenen: RefCell::new(vec![]),
    };
    assert_eq!(sec(&kok, &y, &d2).unwrap().ad, "vpn");
    assert_eq!(d2.denenen.borrow().len(), 1);
    // hiçbiri → Ulasilamadi, adlar listelenir
    let d3 = Sahte {
        ulasilan: vec![],
        denenen: RefCell::new(vec![]),
    };
    match sec(&kok, &y, &d3) {
        Err(YolHata::Ulasilamadi(a)) => assert_eq!(a, ["dogrudan", "vpn", "alan-adi"]),
        x => panic!("{x:?}"),
    }
    assert_eq!(d3.denenen.borrow().len(), 3);
    // boş liste
    assert!(matches!(
        sec(&kok, &Yollar::yeni("m").unwrap(), &d3),
        Err(YolHata::HedefYok)
    ));
}

#[test]
fn hedef_cozumu_oncelik() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    let d = Sahte {
        ulasilan: vec!["sunucu.ornek.com"],
        denenen: RefCell::new(vec![]),
    };
    // liste yok, adres yok → HedefYok
    assert!(matches!(
        hedef_coz(&kok, None, None, None, &d),
        Err(YolHata::HedefYok)
    ));
    // --sunucu her şeyi ezer; anahtar = adres; deneme yapılmaz
    let h = hedef_coz(&kok, Some("https://a.b"), Some("https://c.d"), None, &d).unwrap();
    assert_eq!(
        (h.anahtar.as_str(), h.taban.as_str(), h.yol),
        ("https://a.b", "https://a.b", None)
    );
    let h = hedef_coz(
        &kok,
        None,
        Some("https://c.d/"),
        Some(Path::new("ca.pem")),
        &d,
    )
    .unwrap();
    assert_eq!(h.taban, "https://c.d/");
    assert_eq!(h.sertifika, Some(PathBuf::from("ca.pem")));
    assert!(d.denenen.borrow().is_empty());
    // liste var: anahtar = mantıksal ad, taban = ulaşılan yol, sertifika yoldan
    yollar_yaz(&kok, &ornek()).unwrap();
    let h = hedef_coz(&kok, None, None, None, &d).unwrap();
    assert_eq!(h.anahtar, "merkez");
    assert_eq!(h.taban, "https://sunucu.ornek.com");
    assert_eq!(h.yol.as_deref(), Some("alan-adi"));
    assert_eq!(h.sertifika, None, "'sistem' → CA dosyası yok");
    // --sertifika verildiyse yolunkini ezer
    let h = hedef_coz(&kok, None, None, Some(Path::new("/x/ca.pem")), &d).unwrap();
    assert_eq!(h.sertifika, Some(PathBuf::from("/x/ca.pem")));
    // anahtar_coz ağa çıkmaz
    let d4 = Sahte {
        ulasilan: vec![],
        denenen: RefCell::new(vec![]),
    };
    assert_eq!(anahtar_coz(&kok, None, None).unwrap(), "merkez");
    assert_eq!(
        anahtar_coz(&kok, Some("https://a"), None).unwrap(),
        "https://a"
    );
    assert!(d4.denenen.borrow().is_empty());
    // yanıt yorumu
    assert!(yanit_ulasildi_mi(&Ok(Value::Null)));
    assert!(yanit_ulasildi_mi(&Err(GonderHata::Reddedildi {
        kod: 401,
        mesaj: "cihaz_yok".into()
    })));
    assert!(!yanit_ulasildi_mi(&Err(GonderHata::Ulasilamadi(
        "x".into()
    ))));
    assert!(!yanit_ulasildi_mi(&Err(GonderHata::Yapilandirma(
        "x".into()
    ))));
}

#[test]
fn curl_deneyici_kapali_porta_ulasamaz() {
    // 127.0.0.1:9 (discard) kapalı: bağlantı reddi → ulaşılamadı; curl yoksa da false.
    assert!(!CurlDeneyici.ulasilabilir("http://127.0.0.1:9", None));
}
