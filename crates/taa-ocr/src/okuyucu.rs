//! Yetenekler: oku, kod, tablo, bilgi. Metin katmanı varsa doğrudan çıkarılır (`kaynak: "metin-katmani"`, güven 1.0);
//! yoksa görüntü etkin model paketine verilir (`kaynak: "ocr"`). Model yoksa uydurma yok: `belirsiz: true`.

use crate::eslestir;
use crate::girdi::{self, Tur};
use crate::metin;
use crate::model::{self, Model};
use serde_json::{json, Map, Value};
use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const MAKS_GIRDI: usize = 200 * 1024 * 1024;
const MODEL_ZAMAN: Duration = Duration::from_secs(180);

pub struct Secenek {
    pub tur: String,
    pub dil: String,
    pub sozluk: Vec<String>,
}

impl Default for Secenek {
    fn default() -> Self {
        Secenek {
            tur: "otomatik".into(),
            dil: "tr".into(),
            sozluk: vec![],
        }
    }
}

/// Bir sayfanın ortak okuma sonucu.
#[derive(Default, Debug)]
pub struct Sayfa {
    pub no: u32,
    pub kaynak: &'static str,
    pub metin: String,
    pub guven: f64,
    pub satirlar: Vec<Value>,
    pub tablolar: Vec<Value>,
    pub goruntu: Option<Value>,
    pub model: Option<Value>,
    pub uyari: Vec<String>,
    pub belirsiz: bool,
}

fn hata(k: &str) -> Value {
    json!({ "hata": k })
}

fn metin_katmani_sayfa(no: u32, metin: &str) -> Sayfa {
    let satirlar: Vec<Value> = metin
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| json!({"metin": s, "guven": 1.0, "kaynak": "metin-katmani"}))
        .collect();
    Sayfa {
        no,
        kaynak: "metin-katmani",
        metin: metin.trim().to_string(),
        guven: 1.0,
        satirlar,
        goruntu: Some(json!({"w": 0, "h": 0})),
        ..Default::default()
    }
}

/// Görüntüyü etkin modele verir. Model yoksa ya da çalışmazsa belirsiz sonuç döner (hata değil).
fn goruntu_oku(b: &[u8], uzanti: &str, no: u32, sec: &Secenek, modeller: &[Model]) -> Sayfa {
    let mut s = Sayfa {
        no,
        kaynak: "ocr",
        belirsiz: true,
        ..Default::default()
    };
    let Some(m) = model::sec(modeller, &sec.tur) else {
        s.uyari.push("model-yok".into());
        return s;
    };
    s.model = Some(json!({"ad": m.ad, "surum": m.surum}));
    match model_calistir(&m, b, uzanti, sec) {
        Ok(j) => {
            s.metin = j["metin"].as_str().unwrap_or("").to_string();
            s.guven = j["guven"].as_f64().unwrap_or(0.0).clamp(0.0, 1.0);
            s.satirlar = j["satirlar"].as_array().cloned().unwrap_or_default();
            s.tablolar = j["tablolar"].as_array().cloned().unwrap_or_default();
            s.goruntu = j.get("goruntu").cloned();
            s.belirsiz = j["belirsiz"]
                .as_bool()
                .unwrap_or(s.metin.is_empty() && s.tablolar.is_empty());
        }
        Err(e) => s.uyari.push(format!("model-calismadi: {e}")),
    }
    s
}

fn model_calistir(m: &Model, b: &[u8], uzanti: &str, sec: &Secenek) -> Result<Value, String> {
    let ilk = m.komut.first().ok_or("model komutu yok")?;
    let tmp = taa_dosya::gecici_klasor().map_err(|e| e.to_string())?;
    let g = taa_dosya::atomik_yaz(&tmp.join(format!("girdi.{uzanti}")), b)
        .map_err(|e| e.to_string())?;
    let sozluk = if sec.sozluk.is_empty() {
        None
    } else {
        Some(
            taa_dosya::atomik_yaz(&tmp.join("sozluk.txt"), sec.sozluk.join("\n").as_bytes())
                .map_err(|e| e.to_string())?,
        )
    };
    let cozum = |p: &str| match p.strip_prefix("./") {
        Some(g) => m.dizin.join(g).to_string_lossy().into_owned(),
        None => p.to_string(),
    };
    let mut c = Command::new(cozum(ilk));
    c.args(m.komut[1..].iter().map(|a| cozum(a)))
        .arg(&g)
        .args(["--tur", &sec.tur, "--dil", &sec.dil]);
    if let Some(sz) = &sozluk {
        c.arg("--sozluk").arg(sz);
    }
    c.current_dir(&m.dizin)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let sonuc = (|| {
        let mut cocuk = c.spawn().map_err(|e| e.to_string())?;
        let bas = Instant::now();
        let mut cikti = String::new();
        loop {
            match cocuk.try_wait().map_err(|e| e.to_string())? {
                Some(d) => {
                    if let Some(mut o) = cocuk.stdout.take() {
                        let _ = o.read_to_string(&mut cikti);
                    }
                    if !d.success() {
                        return Err(format!("çıkış kodu {:?}", d.code()));
                    }
                    break;
                }
                None if bas.elapsed() > MODEL_ZAMAN => {
                    let _ = cocuk.kill();
                    return Err("zaman aşımı".into());
                }
                None => std::thread::sleep(Duration::from_millis(20)),
            }
        }
        serde_json::from_str::<Value>(cikti.trim().lines().last().unwrap_or(""))
            .map_err(|_| "model çıktısı JSON değil".to_string())
    })();
    // Geçici girdi iş bitince temizlenir (gizlilik: görüntü motorda kalmaz).
    let _ = std::fs::remove_file(&g);
    if let Some(sz) = sozluk {
        let _ = std::fs::remove_file(sz);
    }
    sonuc
}

/// Girdiyi sayfalara ayırıp okur. Hata kodları sözleşme bölüm 2b ile aynıdır.
pub fn sayfalari_oku(b: &[u8], sec: &Secenek, kok: &Path) -> Result<Vec<Sayfa>, Value> {
    if b.len() > MAKS_GIRDI {
        return Err(hata("girdi-cok-buyuk"));
    }
    let modeller = model::etkin_modeller(kok);
    match girdi::tani(b) {
        Tur::Korumali => Err(hata("korumali")),
        Tur::Desteklenmeyen => Err(hata("desteklenmeyen-bicim")),
        Tur::Goruntu(u) => Ok(vec![goruntu_oku(b, u, 1, sec, &modeller)]),
        Tur::Docx => {
            let d = metin::docx_oku(b).map_err(|_| hata("bozuk-girdi"))?;
            let mut s = metin_katmani_sayfa(1, &d.satirlar.join("\n"));
            s.tablolar = docx_tablolari(&d, &sec.sozluk);
            Ok(vec![s])
        }
        Tur::Pdf => {
            let sayfalar = metin::pdf_oku(b).map_err(|e| match e {
                metin::PdfHata::Korumali => hata("korumali"),
                metin::PdfHata::Bozuk(_) => hata("bozuk-girdi"),
            })?;
            let mut v = Vec::new();
            for p in sayfalar {
                if !p.metin.trim().is_empty() {
                    v.push(metin_katmani_sayfa(p.no, &p.metin));
                } else if p.jpegler.is_empty() {
                    // Taranmış ama gömülü JPEG yok: sayfa görüntüye çevrilemedi, uydurma yok.
                    let mut s = Sayfa {
                        no: p.no,
                        kaynak: "ocr",
                        belirsiz: true,
                        ..Default::default()
                    };
                    s.uyari.push("sayfa-goruntusu-yok".into());
                    v.push(s);
                } else {
                    let mut birlesik = goruntu_oku(&p.jpegler[0], "jpg", p.no, sec, &modeller);
                    for j in &p.jpegler[1..] {
                        let ek = goruntu_oku(j, "jpg", p.no, sec, &modeller);
                        if !ek.metin.is_empty() {
                            birlesik.metin = format!("{}\n{}", birlesik.metin, ek.metin)
                                .trim()
                                .to_string();
                        }
                        birlesik.satirlar.extend(ek.satirlar);
                        birlesik.tablolar.extend(ek.tablolar);
                        birlesik.belirsiz &= ek.belirsiz;
                    }
                    v.push(birlesik);
                }
            }
            Ok(v)
        }
    }
}

fn sayi_adet(s: &str) -> Option<i64> {
    s.trim()
        .parse::<i64>()
        .ok()
        .filter(|n| (1..=99).contains(n))
}

/// docx tablolarını `ocr.tablo@1` biçimine çevirir; sözlük verilmişse stok ve adet alanları da doldurulur.
fn docx_tablolari(d: &metin::Docx, sozluk: &[String]) -> Vec<Value> {
    d.tablolar
        .iter()
        .map(|t| {
            let satirlar: Vec<Value> = t
                .iter()
                .map(|satir| {
                    let hucreler: Vec<Value> = satir
                        .iter()
                        .enumerate()
                        .map(|(i, m)| json!({"sutun": i, "metin": m, "guven": 1.0, "kaynak": "metin-katmani"}))
                        .collect();
                    let mut o = Map::new();
                    if !sozluk.is_empty() {
                        if let Some((i, e)) = satir
                            .iter()
                            .enumerate()
                            .map(|(i, m)| (i, eslestir::kisitli(m, sozluk, 1.0)))
                            .find(|(_, e)| e.durum == "kabul")
                        {
                            o.insert("stok".into(), stok_json(&e));
                            let adet = satir.get(i + 1).map(String::as_str).unwrap_or("");
                            o.insert("adet".into(), match sayi_adet(adet) {
                                Some(n) => json!({"ham": adet, "deger": n, "guven": 1.0, "durum": "kabul"}),
                                None if adet.trim().is_empty() => json!({"ham": "", "deger": 1, "guven": 0.0, "durum": "bos"}),
                                None => json!({"ham": adet, "deger": 1, "guven": 0.0, "durum": "sor"}),
                            });
                        }
                    }
                    o.insert("hucreler".into(), Value::Array(hucreler));
                    o.insert("kaynak".into(), json!("metin-katmani"));
                    Value::Object(o)
                })
                .collect();
            json!({"tur": "bilinmiyor", "kaynak": "metin-katmani", "kutu": null, "belirsiz": false, "satirlar": satirlar})
        })
        .collect()
}

fn stok_json(e: &eslestir::Eslesme) -> Value {
    json!({
        "ham": e.ham, "secilen": e.secilen, "guven": e.guven, "durum": e.durum,
        "aday": e.aday.iter().map(|(k, d)| json!({"deger": k, "kod": k, "mesafe": d, "guven": if *d == 0 { e.guven } else { 0.0 }})).collect::<Vec<_>>()
    })
}

fn kaynak_ozeti(s: &[Sayfa]) -> &'static str {
    if s.iter().all(|x| x.kaynak == "metin-katmani") {
        "metin-katmani"
    } else if s.iter().all(|x| x.kaynak == "ocr") {
        "ocr"
    } else {
        "karma"
    }
}

fn sayfa_json(s: &Sayfa) -> Value {
    let mut o = json!({"no": s.no, "kaynak": s.kaynak, "metin": s.metin, "guven": s.guven,
        "satirlar": s.satirlar, "belirsiz": s.belirsiz});
    if let Some(g) = &s.goruntu {
        o["goruntu"] = g.clone();
    }
    if !s.uyari.is_empty() {
        o["uyari"] = json!(s.uyari);
    }
    o
}

fn temel(sayfalar: &[Sayfa]) -> Value {
    let model = sayfalar
        .iter()
        .find_map(|s| s.model.clone())
        .unwrap_or(Value::Null);
    let metin = sayfalar
        .iter()
        .map(|s| s.metin.as_str())
        .filter(|m| !m.is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let guven = if sayfalar.is_empty() {
        0.0
    } else {
        sayfalar.iter().map(|s| s.guven).sum::<f64>() / sayfalar.len() as f64
    };
    let mut o = json!({
        "arayuz": crate::ARAYUZ, "motor": crate::MOTOR_SURUMU, "model": model,
        "kaynak": kaynak_ozeti(sayfalar), "metin": metin, "guven": guven,
        "satirlar": sayfalar.iter().flat_map(|s| s.satirlar.clone()).collect::<Vec<_>>(),
        "belirsiz": sayfalar.iter().all(|s| s.belirsiz) && !sayfalar.is_empty() || sayfalar.is_empty(),
    });
    let uyari: Vec<String> = sayfalar.iter().flat_map(|s| s.uyari.clone()).collect();
    if !uyari.is_empty() {
        o["uyari"] = json!(uyari);
    }
    if sayfalar.len() > 1 {
        o["sayfalar"] = Value::Array(sayfalar.iter().map(sayfa_json).collect());
    }
    o
}

/// ocr.oku@1
pub fn oku(b: &[u8], sec: &Secenek, kok: &Path) -> Result<Value, Value> {
    Ok(temel(&sayfalari_oku(b, sec, kok)?))
}

/// ocr.kod@1: metindeki her sözcük sözlükle kısıtlı eşlenir; "bos" olmayanlar aday olarak döner.
pub fn kod(b: &[u8], sec: &Secenek, kok: &Path) -> Result<Value, Value> {
    if sec.sozluk.is_empty() {
        return Err(hata("sozluk-gerekli"));
    }
    let sayfalar = sayfalari_oku(b, sec, kok)?;
    let mut o = temel(&sayfalar);
    let mut adaylar = Vec::new();
    for s in &sayfalar {
        let satirlar: Vec<(String, f64)> = if s.satirlar.is_empty() {
            s.metin.lines().map(|l| (l.to_string(), s.guven)).collect()
        } else {
            s.satirlar
                .iter()
                .map(|l| {
                    (
                        l["metin"].as_str().unwrap_or("").to_string(),
                        l["guven"].as_f64().unwrap_or(s.guven),
                    )
                })
                .collect()
        };
        for (satir, g) in satirlar {
            for sozcuk in satir.split_whitespace() {
                let e = eslestir::kisitli(sozcuk, &sec.sozluk, g);
                if e.durum != "bos" {
                    adaylar.push(json!({"ham": e.ham, "deger": e.secilen.clone().or_else(|| e.aday.first().map(|a| a.0.clone())),
                        "guven": e.guven, "durum": e.durum, "kaynak": s.kaynak,
                        "aday": e.aday.iter().map(|(k, d)| json!({"deger": k, "mesafe": d})).collect::<Vec<_>>()}));
                }
            }
        }
    }
    o["belirsiz"] = json!(adaylar.is_empty() || adaylar.iter().all(|a| a["durum"] != "kabul"));
    o["adaylar"] = Value::Array(adaylar);
    Ok(o)
}

/// ocr.tablo@1: sayfalardaki tabloları `tablolar` altında toplar.
pub fn tablo(b: &[u8], sec: &Secenek, kok: &Path) -> Result<Value, Value> {
    let sayfalar = sayfalari_oku(b, sec, kok)?;
    let tablolar: Vec<Value> = sayfalar.iter().flat_map(|s| s.tablolar.clone()).collect();
    let goruntu = sayfalar
        .iter()
        .find_map(|s| s.goruntu.clone())
        .unwrap_or_else(|| json!({"w": 0, "h": 0}));
    let mut o = json!({"arayuz": crate::ARAYUZ, "motor": crate::MOTOR_SURUMU,
        "model": sayfalar.iter().find_map(|s| s.model.clone()).unwrap_or(Value::Null),
        "kaynak": kaynak_ozeti(&sayfalar), "goruntu": goruntu, "belirsiz": tablolar.is_empty()});
    let uyari: Vec<String> = sayfalar.iter().flat_map(|s| s.uyari.clone()).collect();
    if !uyari.is_empty() {
        o["uyari"] = json!(uyari);
    }
    o["tablolar"] = Value::Array(tablolar);
    Ok(o)
}

/// ocr.bilgi@1
pub fn bilgi(kok: &Path) -> Value {
    let modeller = model::etkin_modeller(kok);
    let mut turler: Vec<String> = modeller.iter().flat_map(|m| m.turler.clone()).collect();
    turler.sort();
    turler.dedup();
    json!({
        "arayuz": crate::ARAYUZ, "motor": crate::MOTOR_SURUMU, "yetenekler": crate::YETENEKLER,
        "girdiler": ["jpg", "png", "tiff", "bmp", "webp", "pdf", "docx"],
        "modeller": modeller.iter().map(|m| json!({"ad": m.ad, "surum": m.surum, "turler": m.turler})).collect::<Vec<_>>(),
        "turler": turler,
        "belge_turleri": ["tutanak", "terzi-fisi", "fatura", "defter", "fotograf", "bilinmiyor"],
    })
}
