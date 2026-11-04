//! taa-donustur.exe: TaaDön Rust çekirdeği komut satırı (JSON satırlı çıktı, stderr'e ILERLEME satırları).
//! Sahip: Yusuf (TaaDön projesi). yazan: [Claude#taadon-rust-asama1 2026-11-04], [Claude#taadon-rust-asama2 2026-11-04] (ad: taa-donustur).
//! Değişiklik: Yusuf'un kuralları çerçevesinde. Node tarafı: src/motorlar/rust.js (surec.calistir ile çağırır).
//!
//! Kullanım:
//!   taa-donustur imza <dosya>
//!   taa-donustur zip-listele <zip>
//!   taa-donustur zip-ac <zip> <hedefKlasor> [--maks-bayt N] [--maks-dosya N]
//!   taa-donustur zip-olustur <cikti.zip> <girdi1> [girdi2...] [--maks-dosya N]
//!   taa-donustur gzip <girdi> <cikti.gz>
//!   taa-donustur gunzip <girdi.gz> <cikti> [--maks-bayt N]
//!   taa-donustur surum
//! Başarı: stdout'a tek satır {"ok":true,...}, çıkış kodu 0. Hata: {"ok":false,"hata":"..."}, çıkış kodu 2.

use serde_json::{json, Value};
use std::ffi::OsString;
use std::io::Write;
use std::path::PathBuf;
use taa_donustur::arsiv::{self, VARSAYILAN_MAKS_BOYUT, VARSAYILAN_MAKS_DOSYA};
use taa_donustur::{imza, Hata, Ilerleme, Sonuc};

struct Argumanlar {
    konumlu: Vec<OsString>,
    maks_bayt: Option<u64>,
    maks_dosya: Option<u64>,
}

fn sayi(ad: &str, d: Option<OsString>) -> Sonuc<u64> {
    let d = d.ok_or_else(|| Hata::yeni(format!("{ad} için değer gerekli.")))?;
    d.to_str()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .ok_or_else(|| Hata::yeni(format!("Geçersiz {ad} değeri.")))
}

fn ayristir(ham: Vec<OsString>) -> Sonuc<Argumanlar> {
    let mut a = Argumanlar {
        konumlu: Vec::new(),
        maks_bayt: None,
        maks_dosya: None,
    };
    let mut it = ham.into_iter();
    let mut yalniz_konumlu = false;
    while let Some(x) = it.next() {
        if !yalniz_konumlu {
            match x.to_str() {
                Some("--") => {
                    yalniz_konumlu = true;
                    continue;
                }
                Some("--maks-bayt") => {
                    a.maks_bayt = Some(sayi("--maks-bayt", it.next())?);
                    continue;
                }
                Some("--maks-dosya") => {
                    a.maks_dosya = Some(sayi("--maks-dosya", it.next())?);
                    continue;
                }
                Some(s) if s.starts_with("--") => {
                    return Err(Hata::yeni(format!("Bilinmeyen seçenek: {s}")))
                }
                _ => {}
            }
        }
        a.konumlu.push(x);
    }
    Ok(a)
}

fn konumlu(a: &Argumanlar, en_az: usize, en_cok: usize, kullanim: &str) -> Sonuc<Vec<PathBuf>> {
    let n = a.konumlu.len();
    if n < en_az || n > en_cok {
        return Err(Hata::yeni(format!(
            "Eksik ya da fazla argüman. Kullanım: taa-donustur {kullanim}"
        )));
    }
    Ok(a.konumlu.iter().map(PathBuf::from).collect())
}

/// Sonuç nesnesinin başına "ok": true ekler.
fn birlestir(v: serde_json::Result<Value>) -> Sonuc<Value> {
    let v = v.map_err(|e| Hata::yeni(format!("Sonuç yazılamadı: {e}")))?;
    let mut m = serde_json::Map::new();
    m.insert("ok".into(), Value::Bool(true));
    if let Value::Object(o) = v {
        m.extend(o);
    }
    Ok(Value::Object(m))
}

fn calis() -> Sonuc<Value> {
    let mut ham: Vec<OsString> = std::env::args_os().skip(1).collect();
    if ham.is_empty() {
        return Err(Hata::yeni(
            "Komut gerekli: imza, zip-listele, zip-ac, zip-olustur, gzip, gunzip, surum.",
        ));
    }
    let komut = ham.remove(0).to_string_lossy().into_owned();
    let a = ayristir(ham)?;
    let mut il = Ilerleme::yeni(true);
    match komut.as_str() {
        "surum" | "--surum" | "--version" => {
            Ok(json!({ "ok": true, "surum": env!("CARGO_PKG_VERSION") }))
        }
        "imza" => {
            let p = konumlu(&a, 1, 1, "imza <dosya>")?;
            let s = imza::denetle(&p[0])?;
            Ok(
                json!({ "ok": true, "uzanti": s.uzanti, "imzaUygun": s.imza_uygun, "boyut": s.boyut }),
            )
        }
        "zip-listele" => {
            let p = konumlu(&a, 1, 1, "zip-listele <zip>")?;
            birlestir(serde_json::to_value(arsiv::zip_listele(&p[0])?))
        }
        "zip-ac" => {
            let p = konumlu(
                &a,
                2,
                2,
                "zip-ac <zip> <hedefKlasor> [--maks-bayt N] [--maks-dosya N]",
            )?;
            let maks_bayt = a.maks_bayt.unwrap_or(VARSAYILAN_MAKS_BOYUT);
            let maks_dosya = a.maks_dosya.unwrap_or(VARSAYILAN_MAKS_DOSYA);
            let s = arsiv::zip_ac(&p[0], &p[1], maks_bayt, maks_dosya, &mut il)?;
            il.bildir(100);
            birlestir(serde_json::to_value(s))
        }
        "zip-olustur" => {
            let p = konumlu(
                &a,
                2,
                usize::MAX,
                "zip-olustur <cikti.zip> <girdi1> [girdi2...]",
            )?;
            let s = arsiv::zip_olustur(
                &p[0],
                &p[1..],
                a.maks_dosya.unwrap_or(VARSAYILAN_MAKS_DOSYA),
                &mut il,
            )?;
            il.bildir(100);
            birlestir(serde_json::to_value(s))
        }
        "gzip" => {
            let p = konumlu(&a, 2, 2, "gzip <girdi> <cikti.gz>")?;
            let s = arsiv::gzip(&p[0], &p[1], &mut il)?;
            il.bildir(100);
            birlestir(serde_json::to_value(s))
        }
        "gunzip" => {
            let p = konumlu(&a, 2, 2, "gunzip <girdi.gz> <cikti> [--maks-bayt N]")?;
            let s = arsiv::gunzip(
                &p[0],
                &p[1],
                a.maks_bayt.unwrap_or(VARSAYILAN_MAKS_BOYUT),
                &mut il,
            )?;
            il.bildir(100);
            birlestir(serde_json::to_value(s))
        }
        k => Err(Hata::yeni(format!("Bilinmeyen komut: {k}"))),
    }
}

fn main() {
    let (cikti, kod) = match calis() {
        Ok(v) => (v, 0),
        Err(h) => (json!({ "ok": false, "hata": h.0 }), 2),
    };
    let mut o = std::io::stdout().lock();
    let _ = writeln!(o, "{cikti}");
    let _ = o.flush();
    drop(o);
    std::process::exit(kod);
}
