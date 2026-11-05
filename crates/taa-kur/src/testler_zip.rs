// Sahip: Yusuf. yazan: [Claude#taa-paket-T2 2026-11-05]
//! T2 PR-A2 testleri: zip kurulumu uçtan uca, bozuk sha, yol kaçışı, sınır aşımı, tani veri alanları. Hepsi geçici kökte.

use crate::paket::{icerik_hesapla, sha256_bayt, DosyaKaydi};
use crate::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

struct Ortam {
    taban: PathBuf,
    kur: Kur,
}

impl Ortam {
    fn yeni() -> Ortam {
        let taban = taa_dosya::gecici_klasor().unwrap();
        let kur = Kur::yeni(taban.join("kok")).unwrap();
        Ortam { taban, kur }
    }
}

impl Drop for Ortam {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.taban);
    }
}

fn paket_json(ad: &str, surum: &str, dosyalar: &[(&str, &str)], sha_boz: bool) -> Vec<u8> {
    let liste: Vec<DosyaKaydi> = dosyalar
        .iter()
        .map(|(y, i)| DosyaKaydi {
            yol: y.to_string(),
            sha256: if sha_boz {
                "0".repeat(64)
            } else {
                sha256_bayt(i.as_bytes())
            },
        })
        .collect();
    let p = serde_json::json!({
        "sema": 1, "ad": ad, "surum": surum, "arayuz": "1", "program": "ortak",
        "gerektirir": {}, "sunar": [], "ister": [], "veri_sahibi": [], "veri_okur": [],
        "dosyalar": liste, "icerik_sha256": icerik_hesapla(&liste), "imza": null,
    });
    serde_json::to_vec_pretty(&p).unwrap()
}

/// Ham girdilerle zip yazar (yol denetimi yok: kaçış yolları da yazılabilir).
fn zip_yaz(yol: &Path, girdiler: &[(&str, Vec<u8>)]) {
    let f = fs::File::create(yol).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let sec = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (ad, bayt) in girdiler {
        z.start_file(*ad, sec).unwrap();
        z.write_all(bayt).unwrap();
    }
    z.finish().unwrap();
}

fn paket_zip(
    taban: &Path,
    ad: &str,
    surum: &str,
    dosyalar: &[(&str, &str)],
    sha_boz: bool,
) -> PathBuf {
    let yol = taban.join(format!("{ad}-{surum}.zip"));
    let mut g: Vec<(&str, Vec<u8>)> =
        vec![("paket.json", paket_json(ad, surum, dosyalar, sha_boz))];
    for (y, i) in dosyalar {
        g.push((*y, i.as_bytes().to_vec()));
    }
    zip_yaz(&yol, &g);
    yol
}

fn eski_say(kok: &Path) -> usize {
    fs::read_dir(kok.join("eski"))
        .map(|r| r.count())
        .unwrap_or(0)
}

#[test]
fn zip_kurulumu_uca_uca() {
    let o = Ortam::yeni();
    let z = paket_zip(
        &o.taban,
        "taa-ornek",
        "0.1.0",
        &[("bin/a.exe", "bir"), ("veri/ek.txt", "ek")],
        false,
    );
    let s = o.kur.kur(&z, false).unwrap();
    assert!(s.secili);
    let l = o.kur.liste().unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].secili.as_deref(), Some("0.1.0"));
    assert_eq!(
        fs::read_to_string(o.kur.kok().join("depo/taa-ornek/0.1.0/bin/a.exe")).unwrap(),
        "bir"
    );
    assert!(o.kur.dogrula(None).unwrap().is_empty());
    // geçici klasör kökte kalmadı, eski/ altına taşındı (silinmedi)
    let kalan: Vec<_> = fs::read_dir(o.kur.kok())
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("gecici-"))
        .collect();
    assert!(kalan.is_empty(), "gecici-* kökte kalmamalı");
    assert!(eski_say(o.kur.kok()) >= 1);
    let eski_icinde = fs::read_dir(o.kur.kok().join("eski"))
        .unwrap()
        .flatten()
        .any(|e| e.path().join("bin/a.exe").is_file());
    assert!(eski_icinde, "açılmış geçici içerik eski/ altında durmalı");
}

#[test]
fn zip_bozuk_sha_reddi_hicbir_sey_kurulmaz() {
    let o = Ortam::yeni();
    let z = paket_zip(&o.taban, "taa-ornek", "0.1.0", &[("a.txt", "x")], true);
    let h = o.kur.kur(&z, false).unwrap_err();
    assert_eq!(h.cikis_kodu(), 2, "{h}");
    assert!(h.mesaj().contains("sha256"), "{h}");
    assert!(!o.kur.kok().join("kayit.json").exists());
    assert!(!o.kur.kok().join("depo").exists());
}

#[test]
fn zip_yol_kacisi_reddi() {
    let o = Ortam::yeni();
    for (i, kotu) in [
        "../disari.txt",
        "a/../../b.txt",
        "/mutlak.txt",
        "C:/x.txt",
        "a\\b.txt",
    ]
    .iter()
    .enumerate()
    {
        let z = o.taban.join(format!("kotu{i}.zip"));
        zip_yaz(
            &z,
            &[
                ("paket.json", paket_json("taa-ornek", "0.1.0", &[], false)),
                (kotu, b"x".to_vec()),
            ],
        );
        let h = o.kur.kur(&z, false).unwrap_err();
        assert_eq!(h.cikis_kodu(), 2, "{kotu}: {h}");
    }
    assert!(!o.taban.join("disari.txt").exists());
    assert!(!o.kur.kok().join("depo").exists());
}

#[test]
fn zip_paket_json_kokte_olmali() {
    let o = Ortam::yeni();
    let z = o.taban.join("altta.zip");
    zip_yaz(
        &z,
        &[(
            "alt/paket.json",
            paket_json("taa-ornek", "0.1.0", &[], false),
        )],
    );
    let h = o.kur.kur(&z, false).unwrap_err();
    assert_eq!(h.cikis_kodu(), 2);
    assert!(h.mesaj().contains("paket.json"), "{h}");
}

#[test]
fn zip_sinir_asimi_reddi() {
    let o = Ortam::yeni();
    // dosya sayısı sınırı (küçük sınırla)
    let z = paket_zip(
        &o.taban,
        "taa-ornek",
        "0.1.0",
        &[("a", "1"), ("b", "2"), ("c", "3")],
        false,
    );
    let hedef = o.taban.join("ac1");
    let h = zipac::zip_ac_sinirli(&z, &hedef, 3, 1 << 20).unwrap_err();
    assert_eq!(h.cikis_kodu(), 2);
    assert!(h.mesaj().contains("girdi sayısı"), "{h}");
    assert!(!hedef.exists(), "sınır aşılınca hiçbir şey açılmaz");
    // toplam boyut sınırı: bildirilen boyutlar toplanır, açmadan reddedilir
    let buyuk = "x".repeat(4096);
    let z2 = paket_zip(
        &o.taban,
        "taa-buyuk",
        "0.1.0",
        &[("a", &buyuk), ("b", &buyuk)],
        false,
    );
    let hedef2 = o.taban.join("ac2");
    let h = zipac::zip_ac_sinirli(&z2, &hedef2, 100, 8000).unwrap_err();
    assert!(h.mesaj().contains("toplam boyut"), "{h}");
    assert!(!hedef2.exists());
    // sınır içinde açılır
    zipac::zip_ac_sinirli(&z2, &hedef2, 100, 1 << 20).unwrap();
    assert!(hedef2.join("a").is_file());
    // gerçek sınır: 10.001 girdili zip
    let z3 = o.taban.join("cok.zip");
    let f = fs::File::create(&z3).unwrap();
    let mut zw = zip::ZipWriter::new(f);
    let sec =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for i in 0..10_001 {
        zw.start_file(format!("d{i}"), sec).unwrap();
    }
    zw.finish().unwrap();
    let h = o.kur.kur(&z3, false).unwrap_err();
    assert_eq!(h.cikis_kodu(), 2);
    assert!(h.mesaj().contains("girdi sayısı"), "{h}");
}

#[test]
fn imza_alani_yok_sayilir() {
    let o = Ortam::yeni();
    let k = o.taban.join("klasor");
    fs::create_dir_all(&k).unwrap();
    fs::write(k.join("a.txt"), "x").unwrap();
    let mut p: serde_json::Value =
        serde_json::from_slice(&paket_json("taa-ornek", "0.1.0", &[("a.txt", "x")], false))
            .unwrap();
    p["imza"] = serde_json::json!({"algoritma": "ed25519", "deger": "T3"});
    fs::write(k.join("paket.json"), serde_json::to_vec(&p).unwrap()).unwrap();
    o.kur.kur(&k, false).unwrap();
}

#[test]
fn tani_veri_alanlari() {
    let o = Ortam::yeni();
    let z = paket_zip(&o.taban, "fiyatetiketi", "0.1.0", &[("a", "x")], false);
    o.kur.kur(&z, false).unwrap();
    let vk = o.taban.join("veri");
    for (alan, icerik) in [
        (
            "etiket-kuyrugu",
            "sema = 1\nalan = \"etiket-kuyrugu\"\nsahip = \"fiyatetiketi\"\ngecici = false\n",
        ),
        ("rehber", "sema = 1\nsahip = \"tutaa\"\ngecici = true\n"),
        ("bozuk", "sahip = \n"),
    ] {
        fs::create_dir_all(vk.join(alan)).unwrap();
        fs::write(vk.join(alan).join("SAHIP.toml"), icerik).unwrap();
    }
    fs::create_dir_all(vk.join("sahipsiz")).unwrap(); // SAHIP.toml yok: atlanır
    let t = o.kur.tani_veri_ile(Some(vk.clone())).unwrap();
    assert_eq!(
        t.veri_alanlari,
        vec![
            VeriAlani {
                alan: "etiket-kuyrugu".into(),
                sahip: "fiyatetiketi".into(),
                gecici: false,
                sahip_kurulu: true
            },
            VeriAlani {
                alan: "rehber".into(),
                sahip: "tutaa".into(),
                gecici: true,
                sahip_kurulu: false
            },
        ]
    );
    assert!(t
        .sorunlar
        .iter()
        .any(|s| matches!(s, Sorun::Bozuk(m) if m.contains("bozuk") && m.contains("satır 1"))));
    // veri kökü yoksa boş
    let t = o.kur.tani_veri_ile(None).unwrap();
    assert!(t.veri_alanlari.is_empty() && t.veri_kok.is_none());
    // tani veri alanlarına yazmaz
    assert!(!vk.join("etiket-kuyrugu/eski").exists());
}
