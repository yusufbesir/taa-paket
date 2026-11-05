// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
use super::*;

fn gecici() -> std::path::PathBuf {
    taa_dosya::gecici_klasor().unwrap()
}

#[test]
fn hex_gidis_donus_ve_hatalar() {
    assert_eq!(hex_kodla(&[0, 255, 16]), "00ff10");
    assert_eq!(hex_coz("00ff10").unwrap(), vec![0, 255, 16]);
    assert_eq!(hex_coz("00FF10").unwrap(), vec![0, 255, 16]);
    assert!(hex_coz("abc").is_err());
    assert!(hex_coz("zz").is_err());
}

#[test]
fn uret_imzala_dogrula() {
    let t = gecici();
    let (oz, ge) = (t.join("ozel.key"), t.join("genel.pub"));
    let g = anahtar_uret(&oz, &ge).unwrap();
    let metin = fs::read_to_string(&ge).unwrap();
    assert!(metin.starts_with("ed25519:") && metin.trim().len() == 8 + 64);
    assert_eq!(GenelAnahtar::dosyadan(&ge).unwrap(), g);
    let (o, uyari) = OzelAnahtar::dosyadan(&oz).unwrap();
    // Unix'te 0600 dosyada uyarı olmaz; Windows'ta izin denetlenmez, uyarı da basılmaz (yardım metni açıklar).
    assert!(uyari.is_none(), "uyarı olmamalı: {uyari:?}");
    let im = o.imzala(b"merhaba");
    assert_eq!(im.alg, "ed25519");
    assert_eq!(im.anahtar_kimlik, g.kimlik());
    g.dogrula(b"merhaba", &im).unwrap();
    assert!(g.dogrula(b"merhabA", &im).is_err());
    // Debug özel anahtarı sızdırmaz
    assert_eq!(format!("{o:?}"), "OzelAnahtar(gizli)");
}

#[cfg(unix)]
#[test]
fn ozel_anahtar_0600_ve_ezmez_ve_genis_izin_uyarir() {
    use std::os::unix::fs::PermissionsExt;
    let t = gecici();
    let (oz, ge) = (t.join("ozel.key"), t.join("genel.pub"));
    anahtar_uret(&oz, &ge).unwrap();
    assert_eq!(
        fs::metadata(&oz).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert!(
        anahtar_uret(&oz, &t.join("b.pub")).is_err(),
        "var olan özel anahtar ezilmemeli"
    );
    fs::set_permissions(&oz, fs::Permissions::from_mode(0o644)).unwrap();
    let (_, uyari) = OzelAnahtar::dosyadan(&oz).unwrap();
    assert!(uyari.unwrap().contains("chmod 600"));
}

#[test]
fn iki_uretim_farkli_anahtar() {
    let t = gecici();
    let a = anahtar_uret(&t.join("a.key"), &t.join("a.pub")).unwrap();
    let b = anahtar_uret(&t.join("b.key"), &t.join("b.pub")).unwrap();
    assert_ne!(a, b);
}

#[test]
fn baska_anahtar_ve_bozuk_imza_reddedilir() {
    let t = gecici();
    let g1 = anahtar_uret(&t.join("a.key"), &t.join("a.pub")).unwrap();
    let g2 = anahtar_uret(&t.join("b.key"), &t.join("b.pub")).unwrap();
    let (o1, _) = OzelAnahtar::dosyadan(&t.join("a.key")).unwrap();
    let im = o1.imzala(b"x");
    assert!(g2.dogrula(b"x", &im).is_err());
    let mut bozuk = im.clone();
    bozuk
        .deger
        .replace_range(0..2, if &im.deger[0..2] == "00" { "01" } else { "00" });
    assert!(g1.dogrula(b"x", &bozuk).is_err());
    let mut kisa = im.clone();
    kisa.deger.pop();
    kisa.deger.pop();
    assert!(g1.dogrula(b"x", &kisa).is_err());
    let mut alg = im;
    alg.alg = "rsa".into();
    assert!(g1.dogrula(b"x", &alg).is_err());
}

#[test]
fn genel_anahtar_bicim_hatalari() {
    for kotu in [
        "",
        "# yalniz yorum",
        "ed25519:zz",
        "rsa:00",
        "ed25519:00",
        &format!("ed25519:{}", "0".repeat(63)),
    ] {
        assert!(GenelAnahtar::coz(kotu).is_err(), "{kotu:?}");
    }
    let t = gecici();
    let g = anahtar_uret(&t.join("a.key"), &t.join("a.pub")).unwrap();
    let yorumlu = format!("# yayıncı anahtarı\n\n{}\n", g.metin());
    assert_eq!(GenelAnahtar::coz(&yorumlu).unwrap(), g);
}

#[test]
fn kanonik_imza_alanini_ve_anahtar_sirasini_yok_sayar() {
    let a: serde_json::Value = serde_json::from_str(r#"{"b":1,"a":[1,2],"imza":{"x":1}}"#).unwrap();
    let b: serde_json::Value = serde_json::from_str(r#"{"a":[1,2],"b":1}"#).unwrap();
    assert_eq!(kanonik_bayt(&a), kanonik_bayt(&b));
    assert_eq!(kanonik_bayt(&b), br#"{"a":[1,2],"b":1}"#.to_vec());
}

#[test]
fn zip_paket_imzala_ve_dogrula() {
    use std::io::Write;
    let t = gecici();
    let (oz, ge) = (t.join("ozel.key"), t.join("genel.pub"));
    let g = anahtar_uret(&oz, &ge).unwrap();
    let z = t.join("paket.zip");
    let paket = serde_json::json!({"sema": 1, "ad": "taa-x", "surum": "0.1.0", "icerik_sha256": "abc", "imza": null});
    {
        let mut w = zip::ZipWriter::new(fs::File::create(&z).unwrap());
        let sec = zip::write::SimpleFileOptions::default();
        w.start_file("paket.json", sec).unwrap();
        w.write_all(serde_json::to_string(&paket).unwrap().as_bytes())
            .unwrap();
        w.start_file("bin/a.exe", sec).unwrap();
        w.write_all(b"MZ-sahte").unwrap();
        w.finish().unwrap();
    }
    assert!(zip_mi(&z) && !zip_mi(&t));
    let mut v = zip_paket_json_oku(&z).unwrap();
    let (o, _) = OzelAnahtar::dosyadan(&oz).unwrap();
    paket_imzala(&mut v, &o).unwrap();
    zip_paket_json_yaz(&z, &v).unwrap();
    // eski zip yan yana durur
    assert!(t.join("paket.zip.once-1").is_file());
    // yeni zipte imza var ve doğrulanır; öteki girdi aynı
    let v2 = zip_paket_json_oku(&z).unwrap();
    let im = imza_oku(&v2).unwrap();
    g.dogrula(b"abc", &im).unwrap();
    let mut a = zip::ZipArchive::new(fs::File::open(&z).unwrap()).unwrap();
    assert_eq!(a.len(), 2);
    let mut b = Vec::new();
    std::io::Read::read_to_end(&mut a.by_name("bin/a.exe").unwrap(), &mut b).unwrap();
    assert_eq!(b, b"MZ-sahte");
    // paket.json olmayan zip reddedilir
    let z2 = t.join("bos.zip");
    {
        let mut w = zip::ZipWriter::new(fs::File::create(&z2).unwrap());
        w.start_file("a.txt", zip::write::SimpleFileOptions::default())
            .unwrap();
        w.write_all(b"x").unwrap();
        w.finish().unwrap();
    }
    assert!(zip_paket_json_oku(&z2).is_err());
    assert!(zip_paket_json_yaz(&z2, &v).is_err());
    let _ = fs::remove_dir_all(&t);
}
