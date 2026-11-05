// Sahip: Yusuf. yazan: [Claude#taa-paket-T2 2026-11-05]
//! docs/T2-TASARIM.md §1.1 zorunlu testleri; hepsi geçici kökte (temizlik yok: silme kuralı).

use crate::*;
use serde_json::json;
use std::fs;

fn kok() -> (VeriKok, PathBuf) {
    let t = taa_dosya::gecici_klasor().unwrap();
    (VeriKok::yeni(t.join("veri")), t)
}

#[test]
fn ilk_acilis_sahip_toml_uretir() {
    let (k, _) = kok();
    let s = k
        .ac_sahip("etiket-kuyrugu", "fiyatetiketi", "1.33.0")
        .unwrap();
    assert_eq!(s.kayit().sahip, "fiyatetiketi");
    assert!(!s.kayit().gecici);
    let metin = fs::read_to_string(s.dizin().join("SAHIP.toml")).unwrap();
    let h = sahip_toml_coz(&metin).unwrap();
    assert_eq!(h["sema"], Deger::Sayi(1));
    assert_eq!(h["alan"], Deger::Metin("etiket-kuyrugu".into()));
    assert_eq!(h["yazan_surum"], Deger::Metin("1.33.0".into()));
    assert!(matches!(&h["son_yazim"], Deger::Metin(z) if z.ends_with('Z')));
}

#[test]
fn ayni_sahip_yeniden_acar_ve_surumu_gunceller() {
    let (k, _) = kok();
    k.ac_sahip("belgeler", "taadon", "1.0.0").unwrap();
    let s = k.ac_sahip("belgeler", "taadon", "1.1.0").unwrap();
    assert_eq!(s.kayit().yazan_surum, "1.1.0");
    assert!(!s.dizin().join("eski").exists());
}

#[test]
fn baska_asil_sahip_hata_verir() {
    let (k, _) = kok();
    k.ac_sahip("belgeler", "taadon", "1.0.0").unwrap();
    match k.ac_sahip("belgeler", "digeri", "1.0.0") {
        Err(VeriHata::BaskaSahip { alan, sahip }) => {
            assert_eq!(alan, "belgeler");
            assert_eq!(sahip, "taadon");
        }
        d => panic!("BaskaSahip bekleniyordu: {d:?}"),
    }
    // geçici talep de asıl sahibi alamaz
    assert!(matches!(
        k.ac_gecici_sahip("belgeler", "digeri", "1"),
        Err(VeriHata::BaskaSahip { .. })
    ));
}

#[test]
fn gecici_sahip_devralinir_eski_dosya_eskiye_gider() {
    let (k, _) = kok();
    let g = k.ac_gecici_sahip("rehber", "tutaa", "2.5.0").unwrap();
    assert!(g.kayit().gecici);
    let eski_metin = fs::read_to_string(g.dizin().join("SAHIP.toml")).unwrap();
    let s = k.ac_sahip("rehber", "taarehber", "0.1.0").unwrap();
    assert_eq!(s.kayit().sahip, "taarehber");
    assert!(!s.kayit().gecici);
    let eskiler: Vec<_> = fs::read_dir(s.dizin().join("eski"))
        .unwrap()
        .flatten()
        .collect();
    assert_eq!(eskiler.len(), 1);
    let ad = eskiler[0].file_name().to_string_lossy().into_owned();
    assert!(ad.starts_with("SAHIP.toml-"), "{ad}");
    assert_eq!(fs::read_to_string(eskiler[0].path()).unwrap(), eski_metin);
    // eski geçici sahip artık yazamaz
    assert!(matches!(
        k.ac_gecici_sahip("rehber", "tutaa", "2.5.0"),
        Err(VeriHata::BaskaSahip { .. })
    ));
}

#[test]
fn okuyan_yalniz_oneri_yazar_sahip_uygular_ve_kapatir() {
    let (k, _) = kok();
    let s = k
        .ac_sahip("etiket-kuyrugu", "fiyatetiketi", "1.0.0")
        .unwrap();
    s.yaz("kuyruk.json", b"[]").unwrap();
    let o = k.ac_okur("etiket-kuyrugu", "taarkod").unwrap();
    assert_eq!(o.sahip().sahip, "fiyatetiketi");
    assert_eq!(o.oku("kuyruk.json").unwrap(), b"[]");
    let id = o
        .oner(json!({"ekle": [{"barkod": "123", "adet": 2}]}))
        .unwrap();
    // alan dosyası değişmedi, yalnız oneriler/ altında dosya var
    assert_eq!(s.oku("kuyruk.json").unwrap(), b"[]");
    let ham: Value = serde_json::from_slice(
        &fs::read(s.dizin().join("oneriler").join(format!("{id}.json"))).unwrap(),
    )
    .unwrap();
    assert_eq!(ham["sema"], 1);
    assert_eq!(ham["oneren"], "taarkod");
    assert!(ham["zaman"].as_str().unwrap().ends_with('Z'));
    assert_eq!(ham["degisiklik"]["ekle"][0]["adet"], 2);
    let liste = s.oneriler();
    assert_eq!(liste.len(), 1);
    assert_eq!(liste[0].id, id);
    assert_eq!(liste[0].oneren, "taarkod");
    s.oneri_kapat(&id).unwrap();
    assert!(s.oneriler().is_empty());
    let eski = s.dizin().join("oneriler").join("eski");
    assert_eq!(fs::read_dir(eski).unwrap().count(), 1);
    // iki öneri aynı saniyede ayrı kimlik alır
    let a = o.oner(json!({"x": 1})).unwrap();
    let b = o.oner(json!({"x": 2})).unwrap();
    assert_ne!(a, b);
    assert!(s.oneri_kapat("yok-boyle-bir").is_err());
}

#[test]
fn okur_sahip_yokken_sahip_yok_hatasi() {
    let (k, _) = kok();
    assert!(matches!(
        k.ac_okur("urunler", "taarkod"),
        Err(VeriHata::SahipYok)
    ));
}

#[test]
fn buyuk_oneri_reddedilir() {
    let (k, _) = kok();
    k.ac_sahip("belgeler", "taadon", "1").unwrap();
    let o = k.ac_okur("belgeler", "tutaa").unwrap();
    let dev = "a".repeat(ONERI_EN_COK);
    assert!(matches!(
        o.oner(json!({ "m": dev })),
        Err(VeriHata::OneriBuyuk(_))
    ));
}

#[test]
fn bozuk_toml_satir_numarasiyla_hata() {
    let (k, _) = kok();
    let dizin = k.alan_yolu("belgeler").unwrap();
    fs::create_dir_all(&dizin).unwrap();
    fs::write(
        dizin.join("SAHIP.toml"),
        "sema = 1\n# yorum\nalan = \"belgeler\"\nsahip = taadon\n",
    )
    .unwrap();
    match k.ac_okur("belgeler", "x") {
        Err(VeriHata::GecersizToml { satir, .. }) => assert_eq!(satir, 4),
        d => panic!("GecersizToml bekleniyordu: {d:?}"),
    }
    match k.ac_sahip("belgeler", "taadon", "1") {
        Err(VeriHata::GecersizToml { satir, .. }) => assert_eq!(satir, 4),
        d => panic!("GecersizToml bekleniyordu: {d:?}"),
    }
}

#[test]
fn toml_cozucu_ve_yazici() {
    let h = sahip_toml_coz(
        "\u{feff}# baş\r\nsema = 1\r\nalan = \"a-b\" # son\nx = -5\ny = true\nz = \"çift \\\"tırnak\\\" ve \\\\ ters\"\n",
    )
    .unwrap();
    assert_eq!(h["sema"], Deger::Sayi(1));
    assert_eq!(h["x"], Deger::Sayi(-5));
    assert_eq!(h["y"], Deger::Mantik(true));
    assert_eq!(h["z"], Deger::Metin("çift \"tırnak\" ve \\ ters".into()));
    // gidiş-dönüş
    assert_eq!(sahip_toml_coz(&sahip_toml_yaz(&h)).unwrap(), h);
    // sabit sıra: sema, alan başta
    assert!(sahip_toml_yaz(&h).starts_with("sema = 1\nalan = "));
    let hatali = [
        ("[tablo]\n", 1),
        ("a = 1\nb = [1,2]\n", 2),
        ("a = 1\n\nb = \"acik\n", 3),
        ("a = 1\na = 2\n", 2),
        ("a = 1.5\n", 1),
        ("a = \"x\" y\n", 1),
        ("sadece\n", 1),
        ("a b = 1\n", 1),
        ("a = \"\"\"cok\nsatir\"\"\"\n", 1),
        ("a = \n", 1),
        ("a = \"\\q\"\n", 1),
    ];
    for (girdi, satir) in hatali {
        match sahip_toml_coz(girdi) {
            Err(VeriHata::GecersizToml { satir: s, .. }) => assert_eq!(s, satir, "{girdi:?}"),
            d => panic!("hata bekleniyordu {girdi:?}: {d:?}"),
        }
    }
    assert!(sahip_toml_coz("").unwrap().is_empty());
}

#[test]
fn yol_kacisi_reddedilir() {
    let (k, t) = kok();
    let s = k.ac_sahip("belgeler", "taadon", "1").unwrap();
    for kotu in [
        "../disari.txt",
        "a/../../b",
        "/mutlak",
        "C:/x",
        "a\\b",
        "",
        "a//b",
        "./a",
        "SAHIP.toml",
        "oneriler/x.json",
        "eski/x",
        "a/\0b",
    ] {
        assert!(
            matches!(s.yaz(kotu, b"x"), Err(VeriHata::YolDisari(_))),
            "{kotu:?}"
        );
    }
    let o = k.ac_okur("belgeler", "tutaa").unwrap();
    assert!(matches!(o.oku("../x"), Err(VeriHata::YolDisari(_))));
    assert!(matches!(o.oku("/etc/passwd"), Err(VeriHata::YolDisari(_))));
    assert!(!t.join("veri").join("disari.txt").exists());
    // alt klasöre yazma serbest
    s.yaz("alt/dosya.json", b"{}").unwrap();
    assert_eq!(o.oku("alt/dosya.json").unwrap(), b"{}");
}

#[test]
fn gecersiz_alan_ve_program_adi() {
    let (k, _) = kok();
    for ad in ["", "Buyuk", "a/b", "..", "a b", "ç", &"a".repeat(33)] {
        assert!(matches!(
            k.ac_sahip(ad, "taadon", "1"),
            Err(VeriHata::GecersizAd(_))
        ));
        assert!(matches!(
            k.ac_okur(ad, "taadon"),
            Err(VeriHata::GecersizAd(_))
        ));
    }
    assert!(matches!(
        k.ac_sahip("belgeler", "../x", "1"),
        Err(VeriHata::GecersizAd(_))
    ));
    assert!(k.ac_sahip(&"a".repeat(32), "taadon", "1").is_ok());
}

#[test]
fn yazim_atomik_ve_ust_uste() {
    let (k, _) = kok();
    let s = k.ac_sahip("belgeler", "taadon", "1").unwrap();
    s.yaz("kayitlar.json", b"bir").unwrap();
    s.yaz("kayitlar.json", b"iki").unwrap();
    assert_eq!(s.oku("kayitlar.json").unwrap(), b"iki");
    // geçici dosya artığı kalmaz
    let artik = fs::read_dir(s.dizin())
        .unwrap()
        .flatten()
        .filter(|g| g.file_name().to_string_lossy().ends_with(".tmp"))
        .count();
    assert_eq!(artik, 0);
}

#[test]
fn sahip_kurulu_kayit_jsonu_okur() {
    let (k, t) = kok();
    k.ac_sahip("belgeler", "taadon", "1").unwrap();
    let o = k.ac_okur("belgeler", "tutaa").unwrap();
    let kayit = t.join("kayit.json");
    assert!(!o.sahip_kurulu(&kayit)); // dosya yok
    fs::write(&kayit, "bozuk").unwrap();
    assert!(!o.sahip_kurulu(&kayit));
    fs::write(&kayit, r#"{"paketler":{"baska":{"secili":"1.0.0"}}}"#).unwrap();
    assert!(!o.sahip_kurulu(&kayit));
    fs::write(
        &kayit,
        r#"{"paketler":{"taadon":{"kurulu":[],"secili":null}}}"#,
    )
    .unwrap();
    assert!(!o.sahip_kurulu(&kayit));
    fs::write(
        &kayit,
        r#"{"paketler":{"taadon":{"kurulu":["1.2.0"],"secili":"1.2.0"}}}"#,
    )
    .unwrap();
    assert!(o.sahip_kurulu(&kayit));
}

#[test]
fn rehber_istisnasi_ve_ortam_koku() {
    let (k, t) = kok();
    assert_eq!(
        k.alan_yolu("rehber").unwrap(),
        t.join("veri").join("rehber")
    );
    assert_eq!(
        k.alan_yolu("urunler").unwrap(),
        t.join("veri").join("urunler")
    );
    // ortamdan(): TAA_VERI_KOK ve ortamsız varsayılan tek testte (ortam değişkeni süreç geneli)
    let onceki = std::env::var_os("TAA_VERI_KOK");
    std::env::set_var("TAA_VERI_KOK", t.join("ortam"));
    let e = VeriKok::ortamdan();
    assert_eq!(e.kok(), t.join("ortam"));
    assert_eq!(
        e.alan_yolu("rehber").unwrap(),
        t.join("ortam").join("rehber")
    );
    std::env::remove_var("TAA_VERI_KOK");
    let v = VeriKok::ortamdan();
    assert!(v.kok().ends_with("veri") || v.kok().ends_with("taa-veri"));
    if let Some(o) = onceki {
        std::env::set_var("TAA_VERI_KOK", o);
    }
}

#[test]
fn kaynakta_silme_islevi_yok() {
    // Yasak adı parçalayarak yaz: bu dosyanın kendisi taranırken eşleşmesin.
    let yasak = format!("fs::{}_", "remove");
    let yasak2 = format!(".{}_", "remove");
    let kaynak = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for g in fs::read_dir(kaynak).unwrap().flatten() {
        if g.file_name() == "testler.rs" {
            continue;
        }
        let m = fs::read_to_string(g.path()).unwrap();
        assert!(
            !m.contains(&yasak) && !m.contains(&yasak2),
            "{:?}",
            g.path()
        );
    }
    let testler = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("testler.rs"),
    )
    .unwrap();
    // testler.rs'te yalnız ortam değişkeni kaldırma (remove_var) bulunabilir
    assert!(!testler.contains(&format!("{}_file", "remove")));
    assert!(!testler.contains(&format!("{}_dir", "remove")));
}
