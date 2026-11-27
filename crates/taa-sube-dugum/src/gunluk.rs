// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-sube-dugum 2026-11-27]
//! Düğüm günlüğü: `<kök>/gunluk/YYYY-MM-DD.jsonl` (UTC gün), yalnız ekleme; hiçbir satır silinmez.
//!
//! Satır: `{"zaman", "olay", ...ayrıntı}`. Kayıt gövdesi (müşteri verisi) günlüğe yazılmaz; yalnız `no`, uç,
//! kaynak adresi ve sonuç yazılır. Biten her günün **özeti** (satır sayısı, bayt, sha256, olay sayıları) merkeze
//! gitmek üzere ayrı bir kuyruğa eklenir (`/v1/gunluk/ozet`, `no = GNL-<şube>-<gün>`); günlük dosyası yerinde kalır.

use crate::{Dugum, DugumHata};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use taa_kuyruk::zaman;

pub const KLASOR: &str = "gunluk";
/// Günlük özetinin merkezdeki ucu (öneri; taa-sunucu'da karşılığı doğrulanmadı).
pub const OZET_UCU: &str = "/v1/gunluk/ozet";

pub fn klasor(kok: &Path) -> PathBuf {
    kok.join(KLASOR)
}

pub fn dosya(kok: &Path, gun: &str) -> PathBuf {
    klasor(kok).join(format!("{gun}.jsonl"))
}

/// `YYYY-MM-DD` biçimi (yalnız biçim denetimi).
pub fn gun_gecerli(g: &str) -> bool {
    let b = g.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

/// Bugünün (UTC) günü.
pub fn bugun() -> String {
    zaman::simdi()[..10].to_string()
}

/// Günlüğe bir satır ekler. `ayrinti` nesneyse alanları satıra katılır (`zaman` ve `olay` ezilemez).
pub fn yaz(kok: &Path, olay: &str, ayrinti: Value) -> io::Result<()> {
    let simdi = zaman::simdi();
    let mut m = Map::new();
    if let Value::Object(a) = ayrinti {
        m = a;
    }
    m.insert("zaman".into(), Value::String(simdi.clone()));
    m.insert("olay".into(), Value::String(olay.to_string()));
    taa_dosya::satir_ekle(&dosya(kok, &simdi[..10]), &Value::Object(m).to_string())
}

/// Günlük klasöründeki günler (artan sırada).
pub fn gunler(kok: &Path) -> io::Result<Vec<String>> {
    let mut v = Vec::new();
    let okunan = match fs::read_dir(klasor(kok)) {
        Ok(r) => r,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(v),
        Err(e) => return Err(e),
    };
    for g in okunan {
        let ad = g?.file_name().to_string_lossy().into_owned();
        if let Some(gun) = ad.strip_suffix(".jsonl") {
            if gun_gecerli(gun) {
                v.push(gun.to_string());
            }
        }
    }
    v.sort();
    Ok(v)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Bir günün özeti: `{gun, satir, bayt, sha256, olaylar: {olay: adet}, ilk, son}`.
pub fn ozet(kok: &Path, gun: &str) -> io::Result<Value> {
    let bayt = fs::read(dosya(kok, gun))?;
    let mut olaylar: BTreeMap<String, u64> = BTreeMap::new();
    let (mut satir, mut ilk, mut son) = (0u64, Value::Null, Value::Null);
    for s in String::from_utf8_lossy(&bayt).lines() {
        if s.trim().is_empty() {
            continue;
        }
        satir += 1;
        let v: Value = serde_json::from_str(s).unwrap_or(Value::Null);
        let olay = v.get("olay").and_then(Value::as_str).unwrap_or("bozuk");
        *olaylar.entry(olay.to_string()).or_insert(0) += 1;
        if let Some(z) = v.get("zaman").filter(|z| z.is_string()) {
            if ilk.is_null() {
                ilk = z.clone();
            }
            son = z.clone();
        }
    }
    Ok(json!({
        "gun": gun,
        "satir": satir,
        "bayt": bayt.len(),
        "sha256": hex(&Sha256::digest(&bayt)),
        "olaylar": olaylar,
        "ilk": ilk,
        "son": son,
    }))
}

pub fn ozet_no(sube: &str, gun: &str) -> String {
    format!("GNL-{sube}-{gun}")
}

/// `bugun`den önceki her günün özetini (daha önce eklenmediyse) günlük kuyruğuna ekler; eklenen günleri döndürür.
/// Bugünün dosyası hâlâ yazıldığı için özeti çıkarılmaz.
pub fn ozetleri_kuyrukla(d: &Dugum, bugun: &str) -> Result<Vec<String>, DugumHata> {
    let var: Vec<String> = d
        .gunluk_kuyruk()
        .kayitlar()?
        .into_iter()
        .map(|k| k.no)
        .collect();
    let mut eklenen = Vec::new();
    for gun in gunler(d.kok())? {
        if gun.as_str() >= bugun {
            continue;
        }
        let no = ozet_no(d.sube(), &gun);
        if var.contains(&no) {
            continue;
        }
        let mut govde = ozet(d.kok(), &gun)?;
        govde["no"] = Value::String(no.clone());
        govde["sube"] = Value::String(d.sube().to_string());
        govde["kaynak"] = Value::String("sube-dugum".into());
        d.gunluk_kuyruk().ekle(OZET_UCU, &no, govde)?;
        eklenen.push(gun);
    }
    for gun in &eklenen {
        d.gunluk("gunluk.ozet_kuyrukta", json!({"gun": gun}));
    }
    Ok(eklenen)
}
