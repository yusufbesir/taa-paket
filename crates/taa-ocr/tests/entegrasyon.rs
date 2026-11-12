//! Komut satırı uçtan uca: docx tablo okuma, model yokken belirsiz sonuç, sahte model paketiyle görüntü okuma, hata kodları.

use serde_json::Value;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn gecici(ad: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("taa-ocr-e2e-{ad}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

fn calistir(kok: &Path, args: &[&str]) -> (i32, Value) {
    let o = Command::new(env!("CARGO_BIN_EXE_taa-ocr"))
        .arg("--kok")
        .arg(kok)
        .args(args)
        .output()
        .unwrap();
    let s = String::from_utf8_lossy(&o.stdout);
    (
        o.status.code().unwrap_or(-1),
        serde_json::from_str(s.trim()).unwrap_or(Value::Null),
    )
}

fn docx(yol: &Path) {
    let xml = r#"<w:document><w:body><w:p><w:r><w:t>Teslim</w:t></w:r></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>T14525</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>3</w:t></w:r></w:p></w:tc></w:tr></w:tbl></w:body></w:document>"#;
    let f = fs::File::create(yol).unwrap();
    let mut z = zip::ZipWriter::new(f);
    z.start_file(
        "word/document.xml",
        zip::write::SimpleFileOptions::default(),
    )
    .unwrap();
    z.write_all(xml.as_bytes()).unwrap();
    z.finish().unwrap();
}

#[test]
fn docx_oku_tablo_ve_kod() {
    let d = gecici("docx");
    let f = d.join("a.docx");
    docx(&f);
    let sz = d.join("sozluk.txt");
    fs::write(&sz, "T14525\nT20236-01\n").unwrap();
    let (k, j) = calistir(&d, &["oku", f.to_str().unwrap(), "--json"]);
    assert_eq!(k, 0);
    assert_eq!(j["kaynak"], "metin-katmani");
    assert_eq!(j["guven"], 1.0);
    assert!(j["metin"].as_str().unwrap().contains("Teslim"));
    let (_, t) = calistir(
        &d,
        &[
            "tablo",
            f.to_str().unwrap(),
            "--sozluk",
            sz.to_str().unwrap(),
        ],
    );
    let satir = &t["tablolar"][0]["satirlar"][0];
    assert_eq!(satir["stok"]["secilen"], "T14525");
    assert_eq!(satir["adet"]["deger"], 3);
    assert_eq!(satir["adet"]["durum"], "kabul");
    let (_, c) = calistir(
        &d,
        &["kod", f.to_str().unwrap(), "--sozluk", sz.to_str().unwrap()],
    );
    assert_eq!(c["adaylar"][0]["deger"], "T14525");
    assert_eq!(c["belirsiz"], false);
}

#[test]
fn modelsiz_goruntu_belirsiz_ve_hata_kodlari() {
    let d = gecici("modelsiz");
    let g = d.join("x.jpg");
    fs::write(&g, [0xFF, 0xD8, 0xFF, 0xE0, 0, 0]).unwrap();
    let (k, j) = calistir(&d, &["oku", g.to_str().unwrap()]);
    assert_eq!(k, 0);
    assert_eq!(j["belirsiz"], true);
    assert_eq!(j["metin"], "");
    assert!(j["uyari"][0].as_str().unwrap().contains("model-yok"));
    let t = d.join("n.txt");
    fs::write(&t, "merhaba").unwrap();
    let (k, j) = calistir(&d, &["oku", t.to_str().unwrap()]);
    assert_eq!((k, j["hata"].as_str()), (2, Some("desteklenmeyen-bicim")));
    let o = d.join("o.doc");
    fs::write(&o, [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0]).unwrap();
    assert_eq!(
        calistir(&d, &["oku", o.to_str().unwrap()]).1["hata"],
        "korumali"
    );
    let (k, j) = calistir(&d, &["bilgi"]);
    assert_eq!(k, 0);
    assert!(j["yetenekler"]
        .as_array()
        .unwrap()
        .iter()
        .any(|y| y == "ocr.tablo@1"));
}

#[cfg(unix)]
#[test]
fn sahte_model_paketiyle_goruntu_okunur() {
    use std::os::unix::fs::PermissionsExt;
    let kok = gecici("model");
    let (o_yol, g_yol) = (kok.join("o.key"), kok.join("g.pub"));
    taa_imza::anahtar_uret(&o_yol, &g_yol).unwrap();
    let (o, _) = taa_imza::OzelAnahtar::dosyadan(&o_yol).unwrap();
    let p = kok.join("paket");
    fs::create_dir_all(&p).unwrap();
    let betik = b"#!/bin/sh\necho '{\"metin\":\"T14525 2\",\"guven\":0.93,\"satirlar\":[{\"metin\":\"T14525 2\",\"guven\":0.93}],\"goruntu\":{\"w\":10,\"h\":20}}'\n";
    fs::write(p.join("calistir"), betik).unwrap();
    fs::set_permissions(p.join("calistir"), fs::Permissions::from_mode(0o755)).unwrap();
    use sha2::{Digest, Sha256};
    let ozet: String = Sha256::digest(betik)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect();
    let mut j = serde_json::json!({"ad":"elyazisi","surum":"0.1.0","turler":["elyazisi"],"komut":["./calistir"],"dosyalar":{"calistir": ozet}});
    taa_imza::yayin_imzala(&mut j, &o).unwrap();
    fs::write(p.join("model.json"), serde_json::to_vec(&j).unwrap()).unwrap();
    let (k, e) = calistir(
        &kok,
        &[
            "model-etkinlestir",
            p.to_str().unwrap(),
            "--genel-anahtar",
            g_yol.to_str().unwrap(),
        ],
    );
    assert_eq!((k, e["surum"].as_str()), (0, Some("0.1.0")));
    let g = kok.join("x.png");
    fs::write(&g, b"\x89PNG\r\n\x1a\n....").unwrap();
    let sz = kok.join("s.txt");
    fs::write(&sz, "T14525\n").unwrap();
    let (k, r) = calistir(
        &kok,
        &[
            "kod",
            g.to_str().unwrap(),
            "--sozluk",
            sz.to_str().unwrap(),
            "--tur",
            "elyazisi",
        ],
    );
    assert_eq!(k, 0);
    assert_eq!(r["kaynak"], "ocr", "{r}");
    assert_eq!(r["model"]["ad"], "elyazisi");
    assert_eq!(r["adaylar"][0]["deger"], "T14525", "{r}");
    assert_eq!(r["adaylar"][0]["durum"], "kabul");
    let (_, b) = calistir(&kok, &["bilgi"]);
    assert_eq!(b["modeller"][0]["surum"], "0.1.0");
}
