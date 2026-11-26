// Sahip: Yusuf. yazan: [Claude#ocr-ipc 2026-11-26]
//! IPC yolu: ocr.oku@1, ocr.kod@1, ocr.tablo@1, ocr.bilgi@1 yeteneklerini `taa-ipc` ile sunar
//! (sözleşme: docs/OCR-MOTORU.md bölüm 3, birinci yol). Program adı `taa-ocr`.
//!
//! İstek yükü (`yuk`): `dosya` (yol) ya da `bayt` (0..255 sayı dizisi); `tur`, `dil`; `sozluk` (kod dizisi) ya da
//! `sozluk_dosya` (satır satır dosya). Yanıt yükü komut satırı çıktısıyla aynıdır; hata `{"hata": "..."}` olur.
//! Model yoksa uydurma sonuç yok: `belirsiz: true`, boş metin, `uyari: ["model-yok"]` (okuyucu.rs davranışı).
//! Motor sonuç saklamaz, günlükte metin tutmaz. Yalnız çağıranın verdiği yol okunur.

use crate::okuyucu::{self, Secenek, MAKS_GIRDI};
use serde_json::{json, Value};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use taa_ipc::{IpcHata, Tasiyici, Zarf};

/// taa-ipc kayıt adı.
pub const PROGRAM: &str = "taa-ocr";

/// Yoklama aralığı (taa-ipc yoklama tabanlıdır).
pub const YOKLAMA: Duration = Duration::from_millis(250);

fn hata(k: &str) -> Value {
    json!({ "hata": k })
}

/// Programı ve dört yeteneği taşıyıcıya kaydeder.
pub fn kayit_ol(t: &dyn Tasiyici) -> Result<(), IpcHata> {
    let y: Vec<String> = crate::YETENEKLER.iter().map(|s| s.to_string()).collect();
    t.kayit_ol(PROGRAM, &y)
}

fn girdi_baytlari(yuk: &Value) -> Result<Vec<u8>, Value> {
    if let Some(y) = yuk.get("dosya").and_then(Value::as_str) {
        let m = std::fs::metadata(y).map_err(|_| hata("dosya-okunamadi"))?;
        if m.len() as usize > MAKS_GIRDI {
            return Err(hata("girdi-cok-buyuk"));
        }
        return std::fs::read(y).map_err(|_| hata("dosya-okunamadi"));
    }
    if let Some(d) = yuk.get("bayt").and_then(Value::as_array) {
        if d.len() > MAKS_GIRDI {
            return Err(hata("girdi-cok-buyuk"));
        }
        return d
            .iter()
            .map(|v| v.as_u64().filter(|n| *n <= 255).map(|n| n as u8))
            .collect::<Option<Vec<u8>>>()
            .ok_or_else(|| hata("bozuk-girdi"));
    }
    Err(hata("dosya gerekli"))
}

fn secenek(yuk: &Value) -> Result<Secenek, Value> {
    let mut s = Secenek::default();
    if let Some(t) = yuk.get("tur").and_then(Value::as_str) {
        if !["basili", "elyazisi", "otomatik"].contains(&t) {
            return Err(hata("tur gecersiz"));
        }
        s.tur = t.to_string();
    }
    if let Some(d) = yuk.get("dil").and_then(Value::as_str) {
        s.dil = d.to_string();
    }
    if let Some(a) = yuk.get("sozluk").and_then(Value::as_array) {
        s.sozluk = a
            .iter()
            .filter_map(Value::as_str)
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect();
    } else if let Some(y) = yuk.get("sozluk_dosya").and_then(Value::as_str) {
        let m = std::fs::read_to_string(y).map_err(|_| hata("sozluk-okunamadi"))?;
        s.sozluk = m
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect();
    }
    Ok(s)
}

/// Tek bir yetenek çağrısını yürütür; hata dahil her durumda JSON döner (yanıt yükü).
pub fn isle(yetenek: &str, yuk: &Value, kok: &Path) -> Value {
    let sonuc = match yetenek {
        "ocr.bilgi@1" => Ok(okuyucu::bilgi(kok)),
        "ocr.oku@1" | "ocr.kod@1" | "ocr.tablo@1" => secenek(yuk).and_then(|s| {
            let b = girdi_baytlari(yuk)?;
            match yetenek {
                "ocr.oku@1" => okuyucu::oku(&b, &s, kok),
                "ocr.kod@1" => okuyucu::kod(&b, &s, kok),
                _ => okuyucu::tablo(&b, &s, kok),
            }
        }),
        _ => Err(hata("yetenek-yok")),
    };
    match sonuc {
        Ok(v) | Err(v) => v,
    }
}

/// Bekleyen istekleri işler ve yanıtlar; yanıtlanan istek sayısını döner.
/// Yanıtlanamayan istek (kimlik çakışması vb.) `gelen/` altında kalır, sayıma girmez.
pub fn bekleyenleri_isle(t: &dyn Tasiyici, kok: &Path) -> usize {
    let mut n = 0;
    for z in t.al(PROGRAM) {
        let yanit = isle(&z.yetenek, &z.yuk, kok);
        if yanit_gonder(t, &z, yanit).is_ok() {
            n += 1;
        }
    }
    n
}

fn yanit_gonder(t: &dyn Tasiyici, z: &Zarf, yanit: Value) -> Result<String, IpcHata> {
    t.yanitla(z, yanit)
}

/// Durdurma bayrağı kalkana dek kaydolur ve yoklar. Kayıt başka süreçteyse (`KayitDolu`) hata döner.
pub fn sun(t: &dyn Tasiyici, kok: &Path, dur: &AtomicBool) -> Result<(), IpcHata> {
    kayit_ol(t)?;
    while !dur.load(Ordering::Relaxed) {
        if bekleyenleri_isle(t, kok) == 0 {
            std::thread::sleep(YOKLAMA);
        }
    }
    Ok(())
}

#[cfg(test)]
mod testler {
    use super::*;
    use taa_ipc::GelenKutusu;

    fn gecici(ad: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("taa-ocr-ipc-{ad}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn istek(t: &GelenKutusu, yetenek: &str, yuk: Value) -> String {
        t.gonder(&Zarf::istek("tutaa", PROGRAM, yetenek, yuk))
            .unwrap()
    }

    #[test]
    fn bilgi_yetenekleri_listeler() {
        let k = gecici("bilgi");
        let v = isle("ocr.bilgi@1", &json!({}), &k);
        assert_eq!(v["yetenekler"].as_array().unwrap().len(), 4);
        assert_eq!(v["modeller"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn bilinmeyen_yetenek_hata() {
        let v = isle("ocr.yok@1", &json!({}), &gecici("yok"));
        assert_eq!(v["hata"], "yetenek-yok");
    }

    #[test]
    fn dosya_ve_gecersiz_girdi_hatalari() {
        let k = gecici("hata");
        assert_eq!(isle("ocr.oku@1", &json!({}), &k)["hata"], "dosya gerekli");
        assert_eq!(
            isle("ocr.oku@1", &json!({"dosya": "/yok/yok.png"}), &k)["hata"],
            "dosya-okunamadi"
        );
        assert_eq!(
            isle("ocr.oku@1", &json!({"bayt": [1, 2, 300]}), &k)["hata"],
            "bozuk-girdi"
        );
        assert_eq!(
            isle("ocr.oku@1", &json!({"bayt": [1, 2, 3]}), &k)["hata"],
            "desteklenmeyen-bicim"
        );
        assert_eq!(
            isle("ocr.oku@1", &json!({"bayt": [1], "tur": "x"}), &k)["hata"],
            "tur gecersiz"
        );
        assert_eq!(
            isle("ocr.kod@1", &json!({"bayt": [1]}), &k)["hata"],
            "sozluk-gerekli"
        );
    }

    #[test]
    fn model_yokken_uydurma_sonuc_yok() {
        let k = gecici("modelyok");
        // PNG imzası: görüntü olarak tanınır, etkin model yok.
        let png = [0x89u8, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];
        let yuk = json!({"bayt": png.to_vec()});
        for y in ["ocr.oku@1", "ocr.tablo@1"] {
            let v = isle(y, &yuk, &k);
            assert_eq!(v["belirsiz"], true, "{y}");
            assert_eq!(v["uyari"][0], "model-yok", "{y}");
        }
        let v = isle("ocr.oku@1", &yuk, &k);
        assert_eq!(v["metin"], "");
        let v = isle(
            "ocr.kod@1",
            &json!({"bayt": png.to_vec(), "sozluk": ["T14525"]}),
            &k,
        );
        assert_eq!(v["belirsiz"], true);
        assert_eq!(v["adaylar"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn kayit_dort_yetenegi_sunar() {
        let k = gecici("kayit");
        let t = GelenKutusu::yeni(k.join("ipc"));
        kayit_ol(&t).unwrap();
        for y in crate::YETENEKLER {
            assert_eq!(t.yetenek_bul(y).unwrap().as_deref(), Some(PROGRAM));
        }
    }

    #[test]
    fn istek_yanit_donusu() {
        let k = gecici("donus");
        let t = GelenKutusu::yeni(k.join("ipc"));
        kayit_ol(&t).unwrap();
        let id = istek(&t, "ocr.bilgi@1", json!({}));
        let id2 = istek(&t, "ocr.oku@1", json!({"bayt": [1, 2, 3]}));
        assert_eq!(bekleyenleri_isle(&t, &k), 2);
        let y = t.yanit_al("tutaa", &id).unwrap();
        assert_eq!(y.yuk["arayuz"], 1);
        let y2 = t.yanit_al("tutaa", &id2).unwrap();
        assert_eq!(y2.yuk["hata"], "desteklenmeyen-bicim");
        assert_eq!(bekleyenleri_isle(&t, &k), 0);
    }

    #[test]
    fn yetenek_kayitsizsa_gonderilemez() {
        let k = gecici("kayitsiz");
        let t = GelenKutusu::yeni(k.join("ipc"));
        let z = Zarf::istek("tutaa", PROGRAM, "ocr.oku@1", json!({}));
        assert!(matches!(t.gonder(&z), Err(IpcHata::YetenekYok(_))));
    }

    #[test]
    fn sun_dongusu_durdurulur() {
        let k = gecici("sun");
        let t = GelenKutusu::yeni(k.join("ipc"));
        kayit_ol(&t).unwrap();
        let id = istek(&t, "ocr.bilgi@1", json!({}));
        let dur = AtomicBool::new(false);
        std::thread::scope(|s| {
            s.spawn(|| sun(&t, &k, &dur).unwrap());
            let mut y = None;
            for _ in 0..100 {
                y = t.yanit_al("tutaa", &id);
                if y.is_some() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            dur.store(true, Ordering::Relaxed);
            assert!(y.is_some());
        });
    }
}
