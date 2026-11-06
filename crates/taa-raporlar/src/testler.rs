// Sahip: Yusuf. yazan: [Claude#taa-raporlar-T8 2026-11-06]
//! taa-raporlar testleri: eşikler, para/tarih çözümü, yorum kuralları, karşılaştırma, JSON geri okuma, yönetici denetimi.

use super::*;

fn bolum_var(ad: &str, satirlar: &[(&str, &str)]) -> Bolum {
    Bolum {
        ad: ad.into(),
        baslik: ad.to_uppercase(),
        durum: Durum::Var,
        kaynak: String::new(),
        satirlar: satirlar
            .iter()
            .map(|(a, d)| (a.to_string(), d.to_string()))
            .collect(),
        uyarilar: vec![],
    }
}

fn rapor(bolumler: Vec<Bolum>) -> Rapor {
    Rapor {
        bugun: "2026-11-06".into(),
        bolumler,
    }
}

fn seviyeli<'a>(y: &'a [Yorum], s: Seviye, bolum: &str) -> Vec<&'a str> {
    y.iter()
        .filter(|k| k.seviye == s && k.bolum == bolum)
        .map(|k| k.metin.as_str())
        .collect()
}

#[test]
fn para_ve_tarih_cozumu() {
    assert_eq!(kurus_coz("1.234,56 TL"), Some(123_456));
    assert_eq!(kurus_coz("0,00 TL"), Some(0));
    assert_eq!(kurus_coz("-12,5 TL"), Some(-1250));
    assert_eq!(kurus_coz("7 TL"), Some(700));
    assert_eq!(kurus_coz("abc"), None);
    assert_eq!(tarih_gunu("2026-11-06"), tarih_gunu("20261106-010203"));
    assert_eq!(
        tarih_gunu("2026-11-06").unwrap() - tarih_gunu("2026-11-01").unwrap(),
        5
    );
    assert_eq!(tarih_gunu("2026-13-01"), None);
    assert_eq!(tarih_gunu("kısa"), None);
}

#[test]
fn esikler_json_ve_dosya() {
    let e = Esikler::jsondan(&json!({"geciken_uyari": 2, "bilinmeyen": 9})).unwrap();
    assert_eq!(e.geciken_uyari, 2);
    assert_eq!(e.yedek_gun, Esikler::default().yedek_gun);
    assert!(Esikler::jsondan(&json!({"geciken_uyari": "x"})).is_err());
    assert!(Esikler::jsondan(&json!({"yedek_gun": -1})).is_err());
    let kok = taa_dosya::gecici_klasor().unwrap();
    assert_eq!(Esikler::kokten(&kok).unwrap(), Esikler::default());
    std::fs::create_dir_all(kok.join("raporlar")).unwrap();
    std::fs::write(
        kok.join("raporlar").join(ESIK_DOSYASI),
        b"{\"kargo_dikkat\": 1}",
    )
    .unwrap();
    assert_eq!(Esikler::kokten(&kok).unwrap().kargo_dikkat, 1);
    std::fs::write(kok.join("raporlar").join(ESIK_DOSYASI), b"{bozuk").unwrap();
    assert!(
        Esikler::kokten(&kok).is_err(),
        "bozuk eşik dosyası sessizce varsayılana düşmemeli"
    );
}

#[test]
fn yorum_kurallari() {
    let r = rapor(vec![
        bolum_var(
            "fis",
            &[
                ("toplam_fis", "40"),
                ("acik", "12"),
                ("geciken", "6"),
                ("bugun_teslim", "2"),
                ("kargo_bekleyen", "3"),
                ("acik_kalan_ucret", "6.000,00 TL"),
                ("usta_acik:Mehmet", "16"),
                ("usta_acik:Ali", "2"),
            ],
        ),
        bolum_var(
            "kasa",
            &[
                ("son_kapanis", "2026-11-05"),
                ("son_fark", "-600,00 TL"),
                ("farkli_kapanis", "3"),
                ("toplam_fark", "-900,00 TL"),
            ],
        ),
        bolum_var("bildirim", &[("bekliyor", "25"), ("hata", "1")]),
        bolum_var("katalog", &[("urun", "100"), ("fiyatsiz", "4")]),
        bolum_var("yedek", &[("yedek", "3"), ("son_yedek", "20261101-020000")]),
    ]);
    let y = yorumla(&r, &Esikler::default());
    // Sıra: uyarılar önce.
    assert_eq!(y[0].seviye, Seviye::Uyari);
    assert!(y.iter().all(|k| k.bolum != "genel"));
    let fis_u = seviyeli(&y, Seviye::Uyari, "fis");
    assert!(fis_u[0].starts_with("6 fişin teslim tarihi geçti"));
    let fis_d = seviyeli(&y, Seviye::Dikkat, "fis");
    assert!(fis_d.iter().any(|m| m.contains("3 fiş kargo")));
    assert!(fis_d.iter().any(|m| m.contains("6.000,00 TL")));
    assert!(fis_d.iter().any(|m| m.starts_with("Mehmet ustada 16")));
    assert!(!fis_d.iter().any(|m| m.starts_with("Ali")));
    assert!(seviyeli(&y, Seviye::Bilgi, "fis")
        .iter()
        .any(|m| m == &"Bugün teslim edilecek 2 fiş var."));
    let kasa_u = seviyeli(&y, Seviye::Uyari, "kasa");
    assert!(kasa_u[0].contains("-600,00 TL") && kasa_u[0].contains("2026-11-05"));
    assert!(seviyeli(&y, Seviye::Dikkat, "kasa")[0].contains("3 kapanış"));
    assert!(seviyeli(&y, Seviye::Uyari, "bildirim")[0].starts_with("25 bildirim"));
    assert!(seviyeli(&y, Seviye::Dikkat, "bildirim")[0].starts_with("1 bildirim"));
    assert!(seviyeli(&y, Seviye::Dikkat, "katalog")[0].starts_with("4 ürünün"));
    assert_eq!(
        seviyeli(&y, Seviye::Uyari, "yedek"),
        vec!["Son yedek 5 gün önce alınmış. Yedek alın."]
    );
}

#[test]
fn olagan_rapor_ve_yok_hata_bolumleri() {
    let r = rapor(vec![
        bolum_var(
            "fis",
            &[("toplam_fis", "3"), ("acik", "1"), ("geciken", "0")],
        ),
        bolum_var("kasa", &[("son_fark", "0,00 TL"), ("farkli_kapanis", "0")]),
        bolum_var("yedek", &[("yedek", "1"), ("son_yedek", "20261105-230000")]),
    ]);
    let y = yorumla(&r, &Esikler::default());
    assert_eq!(y[0].bolum, "genel");
    assert!(y.iter().all(|k| k.seviye == Seviye::Bilgi));
    // yedek yok → uyarı; hata bölümü → uyarı; var bölümünün uyarıları → dikkat
    let mut hata = bolum_var("kasa", &[]);
    hata.durum = Durum::Hata;
    hata.uyarilar = vec!["bozuk".into()];
    let mut yok = bolum_var("yedek", &[]);
    yok.durum = Durum::Yok;
    let mut fis = bolum_var("fis", &[("toplam_fis", "1"), ("acik", "0")]);
    fis.uyarilar = vec!["satır 3 bozuk".into()];
    let y = yorumla(&rapor(vec![hata, yok, fis]), &Esikler::default());
    assert!(seviyeli(&y, Seviye::Uyari, "kasa")[0].contains("okunamadı: bozuk"));
    assert_eq!(seviyeli(&y, Seviye::Uyari, "yedek").len(), 1);
    assert!(seviyeli(&y, Seviye::Dikkat, "fis")[0].contains("satır 3 bozuk"));
}

#[test]
fn karsilastirma() {
    let simdi = rapor(vec![
        bolum_var("fis", &[("geciken", "4"), ("acik", "10")]),
        bolum_var("bildirim", &[("hata", "0")]),
    ]);
    let mut onceki = rapor(vec![
        bolum_var("fis", &[("geciken", "2"), ("acik", "12")]),
        bolum_var("bildirim", &[("hata", "3")]),
    ]);
    onceki.bugun = "2026-11-05".into();
    let y = karsilastir(&simdi, &onceki);
    assert_eq!(y.len(), 3);
    assert_eq!(y[0].seviye, Seviye::Dikkat);
    assert!(y[0].metin.contains("geciken fiş 2 → 4 (arttı)"));
    assert_eq!(y[1].seviye, Seviye::Bilgi);
    assert!(y[1].metin.contains("açık fiş 12 → 10 (azaldı)"));
    assert_eq!(y[2].seviye, Seviye::Bilgi);
    assert!(karsilastir(&simdi, &simdi).is_empty());
}

#[test]
fn json_geri_okuma_ve_onceki_rapor() {
    let r = rapor(vec![bolum_var("fis", &[("geciken", "4"), ("acik", "10")])]);
    let geri = rapor_jsondan(&r.json()).unwrap();
    assert_eq!(geri.bugun, "2026-11-06");
    assert_eq!(sayi(&geri.bolumler[0], "geciken"), Some(4));
    let kok = taa_dosya::gecici_klasor().unwrap();
    assert!(onceki_rapor(&kok).is_none());
    let klasor = kok.join("raporlar");
    std::fs::create_dir_all(&klasor).unwrap();
    std::fs::write(
        klasor.join("rapor-20261105-010000.json"),
        json!({"rapor": r.json(), "yorum": []}).to_string(),
    )
    .unwrap();
    // Kayıt biçimi {"rapor": …} sarmalı; onceki_rapor doğrudan Rapor JSON'u da kabul etmeli.
    let mut eski = r.clone();
    eski.bugun = "2026-11-04".into();
    std::fs::write(
        klasor.join("rapor-20261104-010000.json"),
        eski.json().to_string(),
    )
    .unwrap();
    let o = onceki_rapor(&kok).expect("en yeni rapor okunmalı");
    assert_eq!(o.bugun, "2026-11-06");
    assert!(!yorum_json(&yorumla(&o, &Esikler::default()))
        .as_array()
        .unwrap()
        .is_empty());
}

#[test]
fn yonetici_denetimi() {
    struct Sahte(Value);
    impl taa_kimlik::Gonderici for Sahte {
        fn gonder(&self, _: &str, _: &Value) -> Result<Value, taa_kimlik::GonderHata> {
            Ok(self.0.clone())
        }
    }
    let y = taa_kimlik::giris(
        &Sahte(json!({"oturum": "t", "roller": ["kasa", "yonetici"]})),
        "https://s",
        "ali",
        "p",
        None,
    )
    .unwrap();
    assert!(yonetici_mi(&y));
    let k = taa_kimlik::giris(
        &Sahte(json!({"oturum": "t", "roller": ["kasa"]})),
        "https://s",
        "veli",
        "p",
        None,
    )
    .unwrap();
    assert!(!yonetici_mi(&k));
    let metin = yorum_metni(&[Yorum::yeni(Seviye::Uyari, "fis", "x")]);
    assert!(metin.contains("[uyarı] fis: x"));
}
