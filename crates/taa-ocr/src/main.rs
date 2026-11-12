//! taa-ocr: komut satırı (sözleşme docs/OCR-MOTORU.md bölüm 3). Çıktı her zaman tek satır JSON; hata `{"hata": "..."}` ve çıkış kodu 2.
//!   taa-ocr oku <dosya> [--tur basili|elyazisi|otomatik] [--dil tr] [--json]
//!   taa-ocr kod <dosya> --sozluk <dosya> [--json]
//!   taa-ocr tablo <dosya> [--sozluk <dosya>] [--json]
//!   taa-ocr bilgi [--json]
//!   taa-ocr model-liste | model-etkinlestir <paket-klasoru> [--genel-anahtar <dosya>] | model-geri-al <ad>
//! Kök klasör: --kok <yol> ya da TAA_KOK ya da exe klasörü. Genel anahtar varsayılanı: <kök>/ocr/model-imza.pub.

use serde_json::{json, Value};
use std::path::PathBuf;
use taa_ocr::okuyucu::{self, Secenek};

fn bitir(r: Result<Value, Value>) -> ! {
    match r {
        Ok(v) => {
            println!("{v}");
            std::process::exit(0)
        }
        Err(v) => {
            println!("{v}");
            std::process::exit(2)
        }
    }
}

fn hata(m: &str) -> Value {
    json!({ "hata": m })
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let deger = |ad: &str, args: &mut Vec<String>| -> Option<String> {
        let i = args.iter().position(|a| a == ad)?;
        if i + 1 < args.len() {
            let v = args.remove(i + 1);
            args.remove(i);
            Some(v)
        } else {
            args.remove(i);
            None
        }
    };
    let kok = deger("--kok", &mut args).map(PathBuf::from).unwrap_or_else(taa_ocr::model::kok);
    let tur = deger("--tur", &mut args);
    let dil = deger("--dil", &mut args);
    let sozluk_yolu = deger("--sozluk", &mut args);
    let anahtar = deger("--genel-anahtar", &mut args);
    args.retain(|a| a != "--json");
    let Some(komut) = args.first().cloned() else {
        bitir(Err(hata("komut gerekli: oku | kod | tablo | bilgi | model-liste | model-etkinlestir | model-geri-al")));
    };
    let konum = args.get(1).cloned();
    let mut sec = Secenek::default();
    if let Some(t) = tur {
        if !["basili", "elyazisi", "otomatik"].contains(&t.as_str()) {
            bitir(Err(hata("tur gecersiz")));
        }
        sec.tur = t;
    }
    if let Some(d) = dil {
        sec.dil = d;
    }
    if let Some(y) = sozluk_yolu {
        match std::fs::read_to_string(&y) {
            Ok(s) => sec.sozluk = s.lines().map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect(),
            Err(_) => bitir(Err(hata("sozluk-okunamadi"))),
        }
    }
    let oku_dosya = |k: &Option<String>| -> Result<Vec<u8>, Value> {
        let y = k.as_ref().ok_or_else(|| hata("dosya gerekli"))?;
        std::fs::read(y).map_err(|_| hata("dosya-okunamadi"))
    };
    match komut.as_str() {
        "oku" => bitir(oku_dosya(&konum).and_then(|b| okuyucu::oku(&b, &sec, &kok))),
        "kod" => bitir(oku_dosya(&konum).and_then(|b| okuyucu::kod(&b, &sec, &kok))),
        "tablo" => bitir(oku_dosya(&konum).and_then(|b| okuyucu::tablo(&b, &sec, &kok))),
        "bilgi" => bitir(Ok(okuyucu::bilgi(&kok))),
        "model-liste" => bitir(Ok(taa_ocr::model::liste(&kok))),
        "model-etkinlestir" => {
            let Some(p) = konum else { bitir(Err(hata("paket klasörü gerekli"))) };
            let a = anahtar.map(PathBuf::from).unwrap_or_else(|| kok.join("ocr").join("model-imza.pub"));
            bitir(taa_ocr::model::etkinlestir(&kok, &PathBuf::from(p), &a).map_err(|e| hata(&e)))
        }
        "model-geri-al" => {
            let Some(ad) = konum else { bitir(Err(hata("model adı gerekli"))) };
            bitir(taa_ocr::model::geri_al(&kok, &ad).map_err(|e| hata(&e)))
        }
        _ => bitir(Err(hata("bilinmeyen komut"))),
    }
}
