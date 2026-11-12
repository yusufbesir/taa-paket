//! Model paketleri: ayrı sürümlenir, ed25519 imzalıdır, etkinleştirmede özetleri doğrulanır, eski sürüme dönülebilir.
//! Yerleşim: `<kök>/ocr/modeller/<ad>/<surum>/` ve `<kök>/ocr/modeller/etkin.json`. Eski sürüm klasörü silinmez.
//!
//! `model.json`: {"ad","surum","turler":["elyazisi"],"komut":["./calistir"...],"dosyalar":{"yol":"sha256"},"imza":{...}}
//! İmza `taa_imza::kanonik_bayt` (imza alanı hariç model.json) üzerindedir. Komut sözleşmesi:
//! `<komut...> <goruntu> --tur T [--sozluk dosya]` ve stdout'a tek satır JSON (metin, guven, satirlar, tablolar...).
//! Model Python ya da yürütülebilir olabilir; motor çekirdeği Rust'tır.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Model {
    pub ad: String,
    pub surum: String,
    pub dizin: PathBuf,
    pub komut: Vec<String>,
    pub turler: Vec<String>,
}

pub fn kok() -> PathBuf {
    if let Some(k) = std::env::var_os("TAA_KOK") {
        return PathBuf::from(k);
    }
    std::env::current_exe()
        .ok()
        .and_then(|e| e.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn modeller_dizini(kok: &Path) -> PathBuf {
    kok.join("ocr").join("modeller")
}

fn etkin_oku(kok: &Path) -> Value {
    fs::read_to_string(modeller_dizini(kok).join("etkin.json"))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({"modeller": {}}))
}

fn etkin_yaz(kok: &Path, v: &Value) -> Result<(), String> {
    let d = modeller_dizini(kok);
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    let s = serde_json::to_vec_pretty(v).map_err(|e| e.to_string())?;
    taa_dosya::uzerine_yaz_atomik(&d.join("etkin.json"), &s).map_err(|e| e.to_string())
}

fn yukle(dizin: &Path) -> Option<Model> {
    let j: Value = serde_json::from_str(&fs::read_to_string(dizin.join("model.json")).ok()?).ok()?;
    let metinler = |a: &str| -> Vec<String> {
        j[a].as_array()
            .map(|x| x.iter().filter_map(|s| s.as_str().map(String::from)).collect())
            .unwrap_or_default()
    };
    Some(Model {
        ad: j["ad"].as_str()?.to_string(),
        surum: j["surum"].as_str()?.to_string(),
        dizin: dizin.to_path_buf(),
        komut: metinler("komut"),
        turler: metinler("turler"),
    })
}

/// `etkin.json`daki her model için etkin sürümü yükler.
pub fn etkin_modeller(kok: &Path) -> Vec<Model> {
    let e = etkin_oku(kok);
    let mut v = Vec::new();
    if let Some(o) = e["modeller"].as_object() {
        for (ad, b) in o {
            if let Some(s) = b["surum"].as_str() {
                if let Some(m) = yukle(&modeller_dizini(kok).join(ad).join(s)) {
                    v.push(m);
                }
            }
        }
    }
    v
}

/// İstenen türe uyan etkin model ("otomatik": ilk model).
pub fn sec(modeller: &[Model], tur: &str) -> Option<Model> {
    modeller
        .iter()
        .find(|m| tur == "otomatik" || m.turler.iter().any(|t| t == tur))
        .cloned()
}

pub fn liste(kok: &Path) -> Value {
    let e = etkin_oku(kok);
    let mut v = Vec::new();
    if let Some(o) = e["modeller"].as_object() {
        for (ad, b) in o {
            v.push(json!({"ad": ad, "surum": b["surum"], "gecmis": b["gecmis"]}));
        }
    }
    json!({"modeller": v})
}

fn guvenli_yol(y: &str) -> bool {
    let p = Path::new(y);
    !y.is_empty() && p.components().all(|c| matches!(c, Component::Normal(_)))
}

fn ozet(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

/// Paketi doğrular (imza + dosya özetleri) ve `ocr/modeller/<ad>/<surum>/` altına kopyalayıp etkinleştirir.
/// Önceki etkin sürüm `gecmis` listesine eklenir; klasörü silinmez.
pub fn etkinlestir(kok: &Path, paket: &Path, genel_anahtar: &Path) -> Result<Value, String> {
    let ham = fs::read_to_string(paket.join("model.json")).map_err(|_| "model.json okunamadı".to_string())?;
    let j: Value = serde_json::from_str(&ham).map_err(|_| "model.json bozuk".to_string())?;
    let imza = taa_imza::imza_oku(&j).map_err(|_| "imzasiz-model".to_string())?;
    let anahtar = taa_imza::GenelAnahtar::dosyadan(genel_anahtar).map_err(|e| format!("genel anahtar: {e:?}"))?;
    anahtar
        .dogrula(&taa_imza::kanonik_bayt(&j), &imza)
        .map_err(|_| "imza-dogrulanamadi".to_string())?;
    let ad = j["ad"].as_str().filter(|a| guvenli_yol(a)).ok_or("model adı geçersiz")?;
    let surum = j["surum"].as_str().filter(|a| guvenli_yol(a)).ok_or("model sürümü geçersiz")?;
    let dosyalar = j["dosyalar"].as_object().ok_or("dosyalar listesi yok")?;
    let hedef = modeller_dizini(kok).join(ad).join(surum);
    if hedef.exists() {
        return Err("bu-surum-zaten-kurulu".into());
    }
    let mut icerik = Vec::new();
    for (yol, beklenen) in dosyalar {
        if !guvenli_yol(yol) {
            return Err(format!("güvensiz dosya yolu: {yol}"));
        }
        let b = fs::read(paket.join(yol)).map_err(|_| format!("dosya yok: {yol}"))?;
        if Some(ozet(&b).as_str()) != beklenen.as_str() {
            return Err(format!("ozet-uyusmuyor: {yol}"));
        }
        icerik.push((yol.clone(), b));
    }
    fs::create_dir_all(&hedef).map_err(|e| e.to_string())?;
    for (yol, b) in icerik {
        let h = hedef.join(&yol);
        if let Some(p) = h.parent() {
            fs::create_dir_all(p).map_err(|e| e.to_string())?;
        }
        fs::write(&h, b).map_err(|e| e.to_string())?;
        // Çalıştırma izni gibi dosya izinleri korunur (model komutu çalıştırılabilir kalmalı).
        if let Ok(m) = fs::metadata(paket.join(&yol)) {
            let _ = fs::set_permissions(&h, m.permissions());
        }
    }
    fs::write(hedef.join("model.json"), ham).map_err(|e| e.to_string())?;
    let mut e = etkin_oku(kok);
    let kayit = e["modeller"][ad].clone();
    let mut gecmis: Vec<Value> = kayit["gecmis"].as_array().cloned().unwrap_or_default();
    if let Some(eski) = kayit["surum"].as_str() {
        gecmis.push(json!(eski));
    }
    e["modeller"][ad] = json!({"surum": surum, "gecmis": gecmis});
    etkin_yaz(kok, &e)?;
    Ok(json!({"ad": ad, "surum": surum}))
}

/// Etkin sürümü bir öncekine döndürür; klasörler yerinde kalır.
pub fn geri_al(kok: &Path, ad: &str) -> Result<Value, String> {
    let mut e = etkin_oku(kok);
    let kayit = e["modeller"][ad].clone();
    if kayit.is_null() {
        return Err("model-yok".into());
    }
    let mut gecmis: Vec<Value> = kayit["gecmis"].as_array().cloned().unwrap_or_default();
    let onceki = gecmis.pop().and_then(|v| v.as_str().map(String::from)).ok_or("geri-donulecek-surum-yok")?;
    if !modeller_dizini(kok).join(ad).join(&onceki).join("model.json").exists() {
        return Err("onceki-surum-klasoru-yok".into());
    }
    e["modeller"][ad] = json!({"surum": onceki, "gecmis": gecmis});
    etkin_yaz(kok, &e)?;
    Ok(json!({"ad": ad, "surum": onceki}))
}

#[cfg(test)]
mod testler {
    use super::*;

    fn gecici(ad: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("taa-ocr-test-{ad}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn paket(kok: &Path, ad: &str, surum: &str, o: &taa_imza::OzelAnahtar) -> PathBuf {
        let p = kok.join(format!("paket-{surum}"));
        fs::create_dir_all(&p).unwrap();
        fs::write(p.join("calistir"), b"#!/bin/sh\n").unwrap();
        let mut j = json!({"ad": ad, "surum": surum, "turler": ["elyazisi"], "komut": ["./calistir"],
            "dosyalar": {"calistir": ozet(b"#!/bin/sh\n")}});
        taa_imza::yayin_imzala(&mut j, o).unwrap();
        fs::write(p.join("model.json"), serde_json::to_vec(&j).unwrap()).unwrap();
        p
    }

    #[test]
    fn etkinlestir_ve_geri_al() {
        let kok = gecici("akis");
        let ozel_yol = kok.join("o.key");
        let genel_yol = kok.join("g.pub");
        taa_imza::anahtar_uret(&ozel_yol, &genel_yol).unwrap();
        let (o, _) = taa_imza::OzelAnahtar::dosyadan(&ozel_yol).unwrap();
        let p1 = paket(&kok, "elyazisi", "0.1.0", &o);
        let p2 = paket(&kok, "elyazisi", "0.2.0", &o);
        etkinlestir(&kok, &p1, &genel_yol).unwrap();
        etkinlestir(&kok, &p2, &genel_yol).unwrap();
        assert_eq!(etkin_modeller(&kok)[0].surum, "0.2.0");
        assert!(etkinlestir(&kok, &p2, &genel_yol).is_err());
        geri_al(&kok, "elyazisi").unwrap();
        assert_eq!(etkin_modeller(&kok)[0].surum, "0.1.0");
        assert!(kok.join("ocr/modeller/elyazisi/0.2.0/model.json").exists());
        assert!(geri_al(&kok, "elyazisi").is_err());
    }

    #[test]
    fn imzasiz_ve_bozuk_ozet_reddedilir() {
        let kok = gecici("red");
        let ozel_yol = kok.join("o.key");
        let genel_yol = kok.join("g.pub");
        taa_imza::anahtar_uret(&ozel_yol, &genel_yol).unwrap();
        let (o, _) = taa_imza::OzelAnahtar::dosyadan(&ozel_yol).unwrap();
        let p = paket(&kok, "elyazisi", "0.1.0", &o);
        fs::write(p.join("calistir"), b"degistirildi").unwrap();
        assert!(etkinlestir(&kok, &p, &genel_yol).unwrap_err().starts_with("ozet-uyusmuyor"));
        let mut j: Value = serde_json::from_str(&fs::read_to_string(p.join("model.json")).unwrap()).unwrap();
        j.as_object_mut().unwrap().remove("imza");
        fs::write(p.join("model.json"), serde_json::to_vec(&j).unwrap()).unwrap();
        assert_eq!(etkinlestir(&kok, &p, &genel_yol).unwrap_err(), "imzasiz-model");
        assert!(etkin_modeller(&kok).is_empty());
    }
}
