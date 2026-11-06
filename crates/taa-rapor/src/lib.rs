// Sahip: Yusuf. yazan: [Claude#taa-rapor-T5 2026-11-06]
//! taa-rapor: paket raporu. Tüm veri alanlarını (fiş defteri, kasa defteri, bildirim kuyruğu, ürün kataloğu,
//! pano kartları, yedekler) TEK YERDEN okur ve tek özet üretir. SALT OKUNUR: hiçbir alan dosyasına yazmaz.
//!
//! Başka programın crate'ine bağımlı değildir (ince çizgi kuralı); alan dosyalarını `docs/veri-alani.md` ve
//! ilgili crate'lerin JSONL/JSON sözleşmesine göre `serde_json::Value` ile okur. Bozuk satır raporu durdurmaz:
//! satır atlanır, bölüme uyarı yazılır. Olmayan kaynak hata değildir: bölüm "yok" olarak görünür.
//!
//! Çıktılar: metin (ekran), JSON (makine) ve CSV (Excel açar; `;` ayraçlı, UTF-8 BOM).
//! Özellik anahtarı: "rapor" (varsayılan açık).

use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

pub mod tarih;

pub const OZELLIK: &str = "rapor";

/// Okunacak kaynaklar. `None` olan kaynak raporda "yok" olarak görünür.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Kaynaklar {
    /// `fisler.jsonl` (taa-fis defteri).
    pub fis: Option<PathBuf>,
    /// Kasa defteri `.jsonl` (taa-kasa).
    pub kasa: Option<PathBuf>,
    /// `bildirimler.jsonl` (taa-bildirim kuyruğu).
    pub bildirim: Option<PathBuf>,
    /// `katalog.json` (taa-katalog).
    pub katalog: Option<PathBuf>,
    /// `*.pano.json` dosyalarının klasörü (taa-pano).
    pub pano: Option<PathBuf>,
    /// Yedek klasörü: altında `<damga>/manifest.json` (taa-yedek).
    pub yedek: Option<PathBuf>,
}

impl Kaynaklar {
    /// Veri kökünden (`<kok>/<alan>/…`) varsayılan dosya adlarıyla kurar. Olmayan dosya `None` olur.
    pub fn veri_kokunden(kok: &Path) -> Kaynaklar {
        let varsa = |p: PathBuf| if p.exists() { Some(p) } else { None };
        Kaynaklar {
            fis: varsa(kok.join("fis-defteri").join("fisler.jsonl")),
            kasa: varsa(kok.join("kasa-defteri").join("kasa.jsonl")),
            bildirim: varsa(kok.join("bildirim-kuyrugu").join("bildirimler.jsonl")),
            katalog: varsa(kok.join("urunler").join("katalog.json")),
            pano: varsa(kok.join("pano-kartlari")),
            yedek: varsa(kok.join("yedekler")),
        }
    }

    /// taa-fis-sunucu `--veri` klasörü: `fisler.jsonl` ve `bildirimler.jsonl` aynı klasördedir.
    /// Var olan dosyalar kökten gelenlerin yerine geçer.
    pub fn fis_klasoru_ekle(&mut self, klasor: &Path) {
        let f = klasor.join("fisler.jsonl");
        if f.exists() {
            self.fis = Some(f);
        }
        let b = klasor.join("bildirimler.jsonl");
        if b.exists() {
            self.bildirim = Some(b);
        }
    }

    pub fn bos_mu(&self) -> bool {
        self == &Kaynaklar::default()
    }
}

/// Bölüm durumu: kaynak var ve okundu / kaynak yok / okunamadı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durum {
    Var,
    Yok,
    Hata,
}

impl Durum {
    pub fn metin(self) -> &'static str {
        match self {
            Durum::Var => "var",
            Durum::Yok => "yok",
            Durum::Hata => "hata",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bolum {
    /// Makine adı (csv/json anahtarı), ör. "fis".
    pub ad: String,
    /// Ekran başlığı, ör. "Fiş defteri".
    pub baslik: String,
    pub durum: Durum,
    /// Okunan dosya/klasör (bilgi).
    pub kaynak: String,
    /// (anahtar, değer) satırları; sıra korunur.
    pub satirlar: Vec<(String, String)>,
    pub uyarilar: Vec<String>,
}

impl Bolum {
    fn yok(ad: &str, baslik: &str) -> Bolum {
        Bolum {
            ad: ad.into(),
            baslik: baslik.into(),
            durum: Durum::Yok,
            kaynak: String::new(),
            satirlar: vec![],
            uyarilar: vec![],
        }
    }
    fn hata(ad: &str, baslik: &str, kaynak: &Path, neden: String) -> Bolum {
        Bolum {
            ad: ad.into(),
            baslik: baslik.into(),
            durum: Durum::Hata,
            kaynak: kaynak.display().to_string(),
            satirlar: vec![],
            uyarilar: vec![neden],
        }
    }
    fn satir(&mut self, anahtar: &str, deger: impl ToString) {
        self.satirlar.push((anahtar.into(), deger.to_string()));
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rapor {
    /// YYYY-AA-GG (geciken/bugün hesapları bu güne göre).
    pub bugun: String,
    pub bolumler: Vec<Bolum>,
}

/// Raporu üretir. Özellik kapalıysa hata.
pub fn uret(k: &Kaynaklar, bugun: &str) -> Result<Rapor, String> {
    if !taa_ayar::ozellik_acik(OZELLIK, true) {
        return Err(taa_ayar::kapali_mesaji(OZELLIK));
    }
    if !tarih::tarih_gecerli(bugun) {
        return Err(format!("bugün tarihi YYYY-AA-GG olmalı: {bugun}"));
    }
    Ok(Rapor {
        bugun: bugun.into(),
        bolumler: vec![
            fis_bolumu(k.fis.as_deref(), bugun),
            kasa_bolumu(k.kasa.as_deref()),
            bildirim_bolumu(k.bildirim.as_deref()),
            katalog_bolumu(k.katalog.as_deref()),
            pano_bolumu(k.pano.as_deref()),
            yedek_bolumu(k.yedek.as_deref()),
        ],
    })
}

// ---------- ortak yardımcılar ----------

/// JSONL satırlarını okur; bozuk satırlar (satır no, neden) olarak döner, okuma durmaz.
fn jsonl_oku(yol: &Path) -> Result<(Vec<Value>, Vec<String>), String> {
    let satirlar = taa_dosya::satirlari_oku(yol).map_err(|e| format!("okunamadı: {e}"))?;
    let mut kayitlar = Vec::with_capacity(satirlar.len());
    let mut uyarilar = vec![];
    for (i, s) in satirlar.iter().enumerate() {
        if s.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(s) {
            Ok(v) => kayitlar.push(v),
            Err(e) => uyarilar.push(format!("{}. satır bozuk, atlandı: {e}", i + 1)),
        }
    }
    Ok((kayitlar, uyarilar))
}

fn metin<'a>(v: &'a Value, anahtar: &str) -> &'a str {
    v.get(anahtar).and_then(Value::as_str).unwrap_or("")
}

fn sayi(v: &Value, anahtar: &str) -> i64 {
    v.get(anahtar).and_then(Value::as_i64).unwrap_or(0)
}

fn mantik(v: &Value, anahtar: &str) -> bool {
    v.get(anahtar).and_then(Value::as_bool).unwrap_or(false)
}

/// Kuruşu "1.234,56 TL" biçiminde yazar (negatif de olur).
pub fn kurus_metni(k: i64) -> String {
    let isaret = if k < 0 { "-" } else { "" };
    let m = k.unsigned_abs();
    let lira = (m / 100).to_string();
    let mut grup = String::new();
    for (i, c) in lira.chars().enumerate() {
        if i > 0 && (lira.len() - i).is_multiple_of(3) {
            grup.push('.');
        }
        grup.push(c);
    }
    format!("{isaret}{grup},{:02} TL", m % 100)
}

// ---------- fiş defteri (taa-fis sözleşmesi: {"tur":"fis"|"durum"|"teslim"|"raf"|"imza","v":{…}}) ----------

#[derive(Default)]
struct FisOzet {
    no: String,
    durum: String,
    teslim: String,
    kargo: bool,
    kalan_kurus: i64,
    usta: String,
}

fn acik(d: &str) -> bool {
    matches!(d, "bekliyor" | "terzihanede" | "hazir")
}

fn fis_bolumu(yol: Option<&Path>, bugun: &str) -> Bolum {
    const AD: &str = "fis";
    const BASLIK: &str = "Fiş defteri";
    let Some(yol) = yol else {
        return Bolum::yok(AD, BASLIK);
    };
    let (kayitlar, mut uyarilar) = match jsonl_oku(yol) {
        Ok(x) => x,
        Err(e) => return Bolum::hata(AD, BASLIK, yol, e),
    };
    let mut fisler: Vec<FisOzet> = vec![];
    for k in &kayitlar {
        let v = k.get("v").unwrap_or(&Value::Null);
        match metin(k, "tur") {
            "fis" => fisler.push(FisOzet {
                no: metin(v, "no").into(),
                durum: metin(v, "durum").into(),
                teslim: metin(v, "teslim").into(),
                kargo: mantik(v, "kargo"),
                kalan_kurus: sayi(v, "ucret_kurus") - sayi(v, "on_odeme_kurus"),
                usta: metin(v, "usta").into(),
            }),
            "durum" => {
                let no = metin(v, "no");
                if let Some(f) = fisler.iter_mut().find(|f| f.no == no) {
                    f.durum = metin(v, "durum").into();
                }
            }
            "teslim" => {
                let no = metin(v, "no");
                if let Some(f) = fisler.iter_mut().find(|f| f.no == no) {
                    f.teslim = v
                        .get("degisiklik")
                        .map(|d| metin(d, "yeni"))
                        .unwrap_or("")
                        .into();
                }
            }
            "raf" | "imza" => {}
            t => uyarilar.push(format!("bilinmeyen kayıt türü atlandı: {t:?}")),
        }
    }
    let mut b = Bolum {
        ad: AD.into(),
        baslik: BASLIK.into(),
        durum: Durum::Var,
        kaynak: yol.display().to_string(),
        satirlar: vec![],
        uyarilar,
    };
    let say = |d: &str| fisler.iter().filter(|f| f.durum == d).count();
    let acik_olanlar: Vec<&FisOzet> = fisler.iter().filter(|f| acik(&f.durum)).collect();
    b.satir("toplam_fis", fisler.len());
    b.satir("acik", acik_olanlar.len());
    b.satir("bekliyor", say("bekliyor"));
    b.satir("terzihanede", say("terzihanede"));
    b.satir("hazir", say("hazir"));
    b.satir("teslim", say("teslim"));
    b.satir("iptal", say("iptal"));
    b.satir(
        "geciken",
        acik_olanlar
            .iter()
            .filter(|f| !f.teslim.is_empty() && f.teslim.as_str() < bugun)
            .count(),
    );
    b.satir(
        "bugun_teslim",
        acik_olanlar.iter().filter(|f| f.teslim == bugun).count(),
    );
    b.satir(
        "kargo_bekleyen",
        acik_olanlar.iter().filter(|f| f.kargo).count(),
    );
    b.satir(
        "acik_kalan_ucret",
        kurus_metni(acik_olanlar.iter().map(|f| f.kalan_kurus).sum()),
    );
    let mut ustalar: BTreeMap<&str, usize> = BTreeMap::new();
    for f in &acik_olanlar {
        if !f.usta.is_empty() {
            *ustalar.entry(f.usta.as_str()).or_default() += 1;
        }
    }
    for (u, n) in ustalar {
        b.satir(&format!("usta_acik:{u}"), n);
    }
    b
}

// ---------- kasa defteri (taa-kasa: {"tur":"kapanis"|"iptal","v":{…}}) ----------

fn kasa_bolumu(yol: Option<&Path>) -> Bolum {
    const AD: &str = "kasa";
    const BASLIK: &str = "Kasa defteri";
    let Some(yol) = yol else {
        return Bolum::yok(AD, BASLIK);
    };
    let (kayitlar, mut uyarilar) = match jsonl_oku(yol) {
        Ok(x) => x,
        Err(e) => return Bolum::hata(AD, BASLIK, yol, e),
    };
    let iptal: Vec<&str> = kayitlar
        .iter()
        .filter(|k| metin(k, "tur") == "iptal")
        .map(|k| metin(k.get("v").unwrap_or(&Value::Null), "id"))
        .collect();
    let mut kapanislar: Vec<&Value> = vec![];
    for k in &kayitlar {
        match metin(k, "tur") {
            "kapanis" => {
                let v = k.get("v").unwrap_or(&Value::Null);
                if !iptal.contains(&metin(v, "id")) {
                    kapanislar.push(v);
                }
            }
            "iptal" => {}
            t => uyarilar.push(format!("bilinmeyen kayıt türü atlandı: {t:?}")),
        }
    }
    let mut b = Bolum {
        ad: AD.into(),
        baslik: BASLIK.into(),
        durum: Durum::Var,
        kaynak: yol.display().to_string(),
        satirlar: vec![],
        uyarilar,
    };
    b.satir("kapanis", kapanislar.len());
    b.satir("iptal", iptal.len());
    let son = kapanislar
        .iter()
        .map(|v| metin(v, "tarih"))
        .max()
        .unwrap_or("");
    b.satir("son_kapanis", son);
    if let Some(v) = kapanislar.iter().rev().find(|v| metin(v, "tarih") == son) {
        b.satir("son_sayilan", kurus_metni(sayi(v, "sayilan_kurus")));
        b.satir("son_fark", kurus_metni(sayi(v, "fark_kurus")));
    }
    b.satir(
        "farkli_kapanis",
        kapanislar
            .iter()
            .filter(|v| sayi(v, "fark_kurus") != 0)
            .count(),
    );
    b.satir(
        "toplam_fark",
        kurus_metni(kapanislar.iter().map(|v| sayi(v, "fark_kurus")).sum()),
    );
    b
}

// ---------- bildirim kuyruğu (taa-bildirim: {"tur":"mesaj"|"sonuc","v":{…}}; durum "Bekliyor"|"Gonderildi"|{"Hata":"…"}) ----------

fn durum_adi(v: &Value) -> String {
    match v.get("durum") {
        Some(Value::String(s)) => s.to_lowercase(),
        Some(Value::Object(o)) => o
            .keys()
            .next()
            .map(|k| k.to_lowercase())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn bildirim_bolumu(yol: Option<&Path>) -> Bolum {
    const AD: &str = "bildirim";
    const BASLIK: &str = "Bildirim kuyruğu";
    let Some(yol) = yol else {
        return Bolum::yok(AD, BASLIK);
    };
    let (kayitlar, mut uyarilar) = match jsonl_oku(yol) {
        Ok(x) => x,
        Err(e) => return Bolum::hata(AD, BASLIK, yol, e),
    };
    // id -> durum (son sonuç kazanır)
    let mut durumlar: Vec<(String, String)> = vec![];
    for k in &kayitlar {
        let v = k.get("v").unwrap_or(&Value::Null);
        match metin(k, "tur") {
            "mesaj" => durumlar.push((metin(v, "id").into(), durum_adi(v))),
            "sonuc" => {
                let id = metin(v, "id");
                if let Some(d) = durumlar.iter_mut().find(|(i, _)| i == id) {
                    d.1 = durum_adi(v);
                }
            }
            t => uyarilar.push(format!("bilinmeyen kayıt türü atlandı: {t:?}")),
        }
    }
    let mut b = Bolum {
        ad: AD.into(),
        baslik: BASLIK.into(),
        durum: Durum::Var,
        kaynak: yol.display().to_string(),
        satirlar: vec![],
        uyarilar,
    };
    let say = |d: &str| durumlar.iter().filter(|(_, x)| x == d).count();
    b.satir("toplam", durumlar.len());
    b.satir("bekliyor", say("bekliyor"));
    b.satir("gonderildi", say("gonderildi"));
    b.satir("hata", say("hata"));
    b
}

// ---------- ürün kataloğu (taa-katalog: {"kaynak":"…","urunler":{"kod|barkod":{…}}}) ----------

fn katalog_bolumu(yol: Option<&Path>) -> Bolum {
    const AD: &str = "katalog";
    const BASLIK: &str = "Ürün kataloğu";
    let Some(yol) = yol else {
        return Bolum::yok(AD, BASLIK);
    };
    let v: Value = match fs::read(yol)
        .map_err(|e| format!("okunamadı: {e}"))
        .and_then(|b| serde_json::from_slice(&b).map_err(|e| format!("bozuk JSON: {e}")))
    {
        Ok(v) => v,
        Err(e) => return Bolum::hata(AD, BASLIK, yol, e),
    };
    let bos = Map::new();
    let urunler = v.get("urunler").and_then(Value::as_object).unwrap_or(&bos);
    let mut b = Bolum {
        ad: AD.into(),
        baslik: BASLIK.into(),
        durum: Durum::Var,
        kaynak: yol.display().to_string(),
        satirlar: vec![],
        uyarilar: vec![],
    };
    b.satir("kaynak_dosya", metin(&v, "kaynak"));
    b.satir("urun", urunler.len());
    b.satir(
        "indirimli",
        urunler
            .values()
            .filter(|u| u.get("indirimli").map(|i| !i.is_null()).unwrap_or(false))
            .count(),
    );
    b.satir(
        "fiyatsiz",
        urunler.values().filter(|u| sayi(u, "fiyat") <= 0).count(),
    );
    b
}

// ---------- pano kartları (taa-pano: *.pano.json {"baslik","deger","durum"}) ----------

fn pano_bolumu(klasor: Option<&Path>) -> Bolum {
    const AD: &str = "pano";
    const BASLIK: &str = "Pano kartları";
    let Some(klasor) = klasor else {
        return Bolum::yok(AD, BASLIK);
    };
    let mut dosyalar: Vec<PathBuf> = match fs::read_dir(klasor) {
        Ok(d) => d
            .flatten()
            .map(|o| o.path())
            .filter(|p| {
                p.file_name()
                    .map(|n| n.to_string_lossy().ends_with(".pano.json"))
                    .unwrap_or(false)
            })
            .collect(),
        Err(e) => return Bolum::hata(AD, BASLIK, klasor, format!("klasör okunamadı: {e}")),
    };
    dosyalar.sort();
    let mut b = Bolum {
        ad: AD.into(),
        baslik: BASLIK.into(),
        durum: Durum::Var,
        kaynak: klasor.display().to_string(),
        satirlar: vec![],
        uyarilar: vec![],
    };
    b.satir("kart", dosyalar.len());
    for d in dosyalar {
        let ad = d
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        match fs::read(&d)
            .map_err(|e| e.to_string())
            .and_then(|x| serde_json::from_slice::<Value>(&x).map_err(|e| e.to_string()))
        {
            Ok(k) => {
                let durum = k.get("durum").and_then(Value::as_str).unwrap_or("tamam");
                b.satir(
                    &format!("kart:{}", metin(&k, "baslik")),
                    format!("{} [{durum}]", metin(&k, "deger")),
                );
            }
            Err(e) => b.uyarilar.push(format!("{ad} okunamadı: {e}")),
        }
    }
    b
}

// ---------- yedekler (taa-yedek: <klasör>/<damga>/manifest.json {"kayitlar":[{"yol","boyut","sha256"}]}) ----------

fn yedek_bolumu(klasor: Option<&Path>) -> Bolum {
    const AD: &str = "yedek";
    const BASLIK: &str = "Yedekler";
    let Some(klasor) = klasor else {
        return Bolum::yok(AD, BASLIK);
    };
    let mut yedekler: Vec<PathBuf> = match fs::read_dir(klasor) {
        Ok(d) => d
            .flatten()
            .map(|o| o.path())
            .filter(|p| p.join("manifest.json").is_file())
            .collect(),
        Err(e) => return Bolum::hata(AD, BASLIK, klasor, format!("klasör okunamadı: {e}")),
    };
    yedekler.sort();
    let mut b = Bolum {
        ad: AD.into(),
        baslik: BASLIK.into(),
        durum: Durum::Var,
        kaynak: klasor.display().to_string(),
        satirlar: vec![],
        uyarilar: vec![],
    };
    b.satir("yedek", yedekler.len());
    if let Some(son) = yedekler.last() {
        let damga = son
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        b.satir("son_yedek", &damga);
        match fs::read(son.join("manifest.json"))
            .map_err(|e| e.to_string())
            .and_then(|x| serde_json::from_slice::<Value>(&x).map_err(|e| e.to_string()))
        {
            Ok(m) => {
                let kayitlar = m.get("kayitlar").and_then(Value::as_array);
                b.satir("son_dosya", kayitlar.map(|k| k.len()).unwrap_or(0));
                let boyut: i64 = kayitlar
                    .map(|k| k.iter().map(|x| sayi(x, "boyut")).sum())
                    .unwrap_or(0);
                b.satir("son_boyut_bayt", boyut);
            }
            Err(e) => b
                .uyarilar
                .push(format!("{damga}/manifest.json okunamadı: {e}")),
        }
    }
    b
}

// ---------- çıktılar ----------

impl Rapor {
    /// Ekran metni: her bölüm başlık + satırlar; yok/hata bölümleri tek satır.
    pub fn metin(&self) -> String {
        let mut s = format!("TAA paket raporu — {}\n", self.bugun);
        for b in &self.bolumler {
            match b.durum {
                Durum::Yok => s.push_str(&format!("\n[{}] kaynak yok\n", b.baslik)),
                Durum::Hata => s.push_str(&format!(
                    "\n[{}] OKUNAMADI ({}): {}\n",
                    b.baslik,
                    b.kaynak,
                    b.uyarilar.join("; ")
                )),
                Durum::Var => {
                    s.push_str(&format!("\n[{}] {}\n", b.baslik, b.kaynak));
                    for (k, v) in &b.satirlar {
                        s.push_str(&format!("  {k}: {v}\n"));
                    }
                    for u in &b.uyarilar {
                        s.push_str(&format!("  ! {u}\n"));
                    }
                }
            }
        }
        s
    }

    pub fn json(&self) -> Value {
        json!({
            "bugun": self.bugun,
            "bolumler": self.bolumler.iter().map(|b| json!({
                "ad": b.ad,
                "baslik": b.baslik,
                "durum": b.durum.metin(),
                "kaynak": b.kaynak,
                "satirlar": b.satirlar.iter().map(|(k, v)| json!({"anahtar": k, "deger": v})).collect::<Vec<_>>(),
                "uyarilar": b.uyarilar,
            })).collect::<Vec<_>>(),
        })
    }

    /// Excel'in doğrudan açtığı CSV: UTF-8 BOM, `;` ayraç, `bolum;durum;anahtar;deger` başlığı.
    pub fn csv(&self) -> String {
        fn hucre(s: &str) -> String {
            if s.contains(';') || s.contains('"') || s.contains('\n') {
                format!("\"{}\"", s.replace('"', "\"\""))
            } else {
                s.to_string()
            }
        }
        let mut s = String::from("\u{feff}bolum;durum;anahtar;deger\r\n");
        s.push_str(&format!("rapor;var;bugun;{}\r\n", self.bugun));
        for b in &self.bolumler {
            if b.satirlar.is_empty() {
                s.push_str(&format!(
                    "{};{};kaynak;{}\r\n",
                    b.ad,
                    b.durum.metin(),
                    hucre(&b.kaynak)
                ));
            }
            for (k, v) in &b.satirlar {
                s.push_str(&format!(
                    "{};{};{};{}\r\n",
                    b.ad,
                    b.durum.metin(),
                    hucre(k),
                    hucre(v)
                ));
            }
            for u in &b.uyarilar {
                s.push_str(&format!(
                    "{};{};uyari;{}\r\n",
                    b.ad,
                    b.durum.metin(),
                    hucre(u)
                ));
            }
        }
        s
    }
}

#[cfg(test)]
mod testler;
