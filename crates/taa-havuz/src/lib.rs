// Sahip: Yusuf.
//! taa-havuz: ortak veri havuzu istemcisi (plan H3; sözleşme docs/havuz-sozlesmesi.md).
//!
//! Her program kendi önbelleğini `<veri kökü>/havuz-onbellek/<program>/` altında tutar:
//! - `durum.json`   iki imleç: `oge_sira` (etkin öğeler) ve `olay_sira` (arşivlenenler).
//! - `dizin.json`   önbellekteki etkin öğelerin üst verisi.
//! - `tablo/<id>.json`  tablo türlerinin tüm satırları.
//! - `icerik/ab/cd/<sha256>`  indirilmiş ve doğrulanmış dosyalar; `icerik/<sha256>.part` yarım indirme.
//!
//! `esitle` yalnız ağ gerektirir: önce yeni etkin öğeleri (`/ogeler?since=`), sonra arşivlenenleri
//! (`/olaylar?since=`) alır. Sıra önemlidir: sunucu `sira` değerini etkinleştirme anında verir, bu yüzden
//! bir öğe `ogeler`'de görünüp sonra arşivlenirse olay akışı onu düşürür; ters sırada bir öğe kaçabilirdi.
//! Imleç her öğeden sonra diske yazılır: bağlantı kesilirse kaldığı yerden sürer, aynı öğe iki kez işlenmez
//! (tablo satırları dosyaya öğe başına tam yazılır). Okuma yöntemleri (`ogeler`, `satirlar`, `dosya_yolu` …)
//! ağa hiç dokunmaz; sunucu yokken önbellek okunur.
//!
//! Silme yok: arşivlenen öğe dizinden çıkar, tablo/içerik dosyaları yerinde kalır (yetim sayısı `yetimler`).
//! Dosyalar isteğe bağlı indirilir (`indir`): 1 MiB'lik `Range` parçalarıyla, yarım kalırsa sürdürülür, bitince
//! sha256 ve boyut sunucudaki kayıtla karşılaştırılır; uyuşmazsa yarım dosya sıfırlanır ve dosya sunulmaz.
//! Ulaşım yolu (taa-yol) seçimi çağıranındır: `Gonderici` hangi tabana bağlıysa o kullanılır.
//! Özellik anahtarı: "havuz".

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use taa_kuyruk::{GonderHata, Gonderici};

pub const OZELLIK: &str = "havuz";
/// Önbellek klasörünün veri kökü altındaki adı.
pub const KLASOR: &str = "havuz-onbellek";
/// Sunucudan tek sayfada istenen en çok öğe/satır/olay.
pub const SAYFA: usize = 500;
/// Dosya indirmede tek `Range` isteğinin bayt sayısı (taa-kuyruk yanıt sınırı 4 MiB'in altında).
pub const PARCA: u64 = 1024 * 1024;

#[derive(Debug)]
pub enum HavuzHata {
    /// Sunucuyla iletişim (ulaşılamadı / reddedildi / yapılandırma). Önbellek bozulmaz.
    Gonder(GonderHata),
    /// Yerel disk hatası.
    Disk(String),
    /// Sunucu yanıtı ya da önbellek dosyası beklenen biçimde değil.
    Bicim(String),
    /// İndirilen dosyanın özeti sunucudaki kayıtla uyuşmuyor (yarım dosya sıfırlandı).
    Ozet { beklenen: String, bulunan: String },
    /// İstenen öğe önbellekte yok ya da türü uygun değil.
    Yok(String),
    /// Geçersiz argüman (program adı vb.).
    Gecersiz(String),
}

impl HavuzHata {
    /// Ağ yokluğu/sunucu sorunu mu (çağıran önbellekle devam eder)?
    pub fn cevrimdisi_mi(&self) -> bool {
        matches!(self, HavuzHata::Gonder(GonderHata::Ulasilamadi(_)))
    }
}

impl fmt::Display for HavuzHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HavuzHata::Gonder(e) => write!(f, "{e}"),
            HavuzHata::Disk(m) => write!(f, "disk hatası: {m}"),
            HavuzHata::Bicim(m) => write!(f, "biçim hatası: {m}"),
            HavuzHata::Ozet { beklenen, bulunan } => write!(
                f,
                "dosya özeti uyuşmuyor (beklenen {beklenen}, bulunan {bulunan}); indirme sıfırlandı, yeniden deneyin"
            ),
            HavuzHata::Yok(m) => write!(f, "bulunamadı: {m}"),
            HavuzHata::Gecersiz(m) => write!(f, "geçersiz değer: {m}"),
        }
    }
}

impl std::error::Error for HavuzHata {}

impl From<GonderHata> for HavuzHata {
    fn from(e: GonderHata) -> Self {
        HavuzHata::Gonder(e)
    }
}

fn disk<T>(r: std::io::Result<T>, ne: &str) -> Result<T, HavuzHata> {
    r.map_err(|e| HavuzHata::Disk(format!("{ne}: {e}")))
}

/// Önbellekteki bir havuz öğesi (sunucu `ogeler` yanıtının bir satırı).
#[derive(Debug, Clone, PartialEq)]
pub struct Oge {
    pub id: i64,
    pub sira: i64,
    pub tur: String,
    /// "tablo" | "dosya"
    pub kip: String,
    pub ad: String,
    pub surum_no: i64,
    pub meta: Value,
    pub sha256: Option<String>,
    pub boyut: Option<i64>,
    pub mime: Option<String>,
}

impl Oge {
    fn coz(v: &Value) -> Result<Oge, HavuzHata> {
        let hata = |a: &str| HavuzHata::Bicim(format!("öğe alanı eksik/yanlış: {a}"));
        let metin = |a: &str| v.get(a).and_then(Value::as_str).map(str::to_string);
        let kip = metin("kip").ok_or_else(|| hata("kip"))?;
        if kip != "tablo" && kip != "dosya" {
            return Err(hata("kip"));
        }
        let sha256 = metin("sha256");
        if let Some(s) = &sha256 {
            if !sha_gecerli(s) {
                return Err(hata("sha256"));
            }
        }
        Ok(Oge {
            id: v
                .get("id")
                .and_then(Value::as_i64)
                .ok_or_else(|| hata("id"))?,
            sira: v
                .get("sira")
                .and_then(Value::as_i64)
                .ok_or_else(|| hata("sira"))?,
            tur: metin("tur").ok_or_else(|| hata("tur"))?,
            kip,
            ad: metin("ad").ok_or_else(|| hata("ad"))?,
            surum_no: v.get("surum_no").and_then(Value::as_i64).unwrap_or(1),
            meta: v.get("meta").cloned().unwrap_or_else(|| json!({})),
            sha256,
            boyut: v.get("boyut").and_then(Value::as_i64),
            mime: metin("mime"),
        })
    }

    fn json(&self) -> Value {
        json!({
            "id": self.id, "sira": self.sira, "tur": self.tur, "kip": self.kip, "ad": self.ad,
            "surum_no": self.surum_no, "meta": self.meta, "sha256": self.sha256,
            "boyut": self.boyut, "mime": self.mime
        })
    }
}

fn sha_gecerli(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}

/// Program adı: sunucuyla aynı kural (`[a-z0-9-]`, 1-32).
pub fn program_gecerli(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= 32
        && p.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

/// Sorgu değeri için yüzde kodlama (yalnız `A-Za-z0-9-_.` olduğu gibi kalır).
pub fn yuzde_kodla(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.') {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

/// Bir eşitlemenin özeti.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Ozet {
    /// Önbelleğe yeni giren etkin öğe sayısı.
    pub yeni: usize,
    /// Dizinden düşen (arşivlenen) öğe sayısı.
    pub arsivlenen: usize,
    /// İndirilen tablo satırı sayısı.
    pub satir: usize,
}

/// Bir programın havuz önbelleği.
#[derive(Debug)]
pub struct Havuz {
    klasor: PathBuf,
    program: String,
    oge_sira: i64,
    olay_sira: i64,
    dizin: BTreeMap<i64, Oge>,
}

impl Havuz {
    /// `<kok>/havuz-onbellek/<program>/` önbelleğini açar (yoksa boş başlar). Bozuk dosya sessizce ezilmez.
    pub fn ac(kok: &Path, program: &str) -> Result<Havuz, HavuzHata> {
        if !program_gecerli(program) {
            return Err(HavuzHata::Gecersiz(format!(
                "program adı [a-z0-9-], 1-32 karakter olmalı: {program}"
            )));
        }
        let klasor = kok.join(KLASOR).join(program);
        let mut h = Havuz {
            klasor,
            program: program.to_string(),
            oge_sira: 0,
            olay_sira: 0,
            dizin: BTreeMap::new(),
        };
        if let Some(d) = h.oku_json("durum.json")? {
            h.oge_sira = d
                .get("oge_sira")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0);
            h.olay_sira = d
                .get("olay_sira")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                .max(0);
        }
        if let Some(d) = h.oku_json("dizin.json")? {
            for v in d
                .get("ogeler")
                .and_then(Value::as_array)
                .ok_or_else(|| HavuzHata::Bicim("dizin.json: ogeler dizisi yok".into()))?
            {
                let o = Oge::coz(v)?;
                h.dizin.insert(o.id, o);
            }
        }
        Ok(h)
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    pub fn klasor(&self) -> &Path {
        &self.klasor
    }

    /// (öğe imleci, olay imleci).
    pub fn imlecler(&self) -> (i64, i64) {
        (self.oge_sira, self.olay_sira)
    }

    fn oku_json(&self, ad: &str) -> Result<Option<Value>, HavuzHata> {
        let yol = self.klasor.join(ad);
        match fs::read(&yol) {
            Ok(b) => serde_json::from_slice(&b)
                .map(Some)
                .map_err(|e| HavuzHata::Bicim(format!("{}: {e}", yol.display()))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(HavuzHata::Disk(format!("{}: {e}", yol.display()))),
        }
    }

    fn kaydet(&self) -> Result<(), HavuzHata> {
        let dizin = json!({ "ogeler": self.dizin.values().map(Oge::json).collect::<Vec<_>>() });
        disk(
            taa_dosya::uzerine_yaz_atomik(
                &self.klasor.join("dizin.json"),
                dizin.to_string().as_bytes(),
            ),
            "dizin.json yazılamadı",
        )?;
        let durum = json!({ "oge_sira": self.oge_sira, "olay_sira": self.olay_sira, "program": self.program });
        disk(
            taa_dosya::uzerine_yaz_atomik(
                &self.klasor.join("durum.json"),
                durum.to_string().as_bytes(),
            ),
            "durum.json yazılamadı",
        )
    }

    // ---- eşitleme (ağ) ----

    /// Sunucudaki değişiklikleri önbelleğe alır. Hata olursa o ana kadarki ilerleme diskte kalır.
    pub fn esitle(&mut self, g: &dyn Gonderici) -> Result<Ozet, HavuzHata> {
        let mut ozet = Ozet::default();
        disk(
            fs::create_dir_all(&self.klasor),
            "önbellek klasörü açılamadı",
        )?;
        // 1) yeni etkin öğeler
        loop {
            let uc = format!(
                "/v1/havuz/ogeler?program={}&since={}&limit={SAYFA}",
                self.program, self.oge_sira
            );
            let y = g.al(&uc)?;
            let liste = dizi(&y, "ogeler")?;
            if liste.is_empty() {
                break;
            }
            for v in liste {
                let o = Oge::coz(v)?;
                if o.sira <= self.oge_sira {
                    return Err(HavuzHata::Bicim(format!(
                        "sunucu sırası geriye gitti: {} <= {}",
                        o.sira, self.oge_sira
                    )));
                }
                if o.kip == "tablo" {
                    match self.satirlari_indir(g, &o) {
                        Ok(n) => ozet.satir += n,
                        // Öğe bu arada arşivlendi/görünmez oldu: atla, imleç ilerlesin (takılmasın).
                        Err(HavuzHata::Gonder(GonderHata::Reddedildi { kod: 404, .. })) => {
                            self.oge_sira = o.sira;
                            self.kaydet()?;
                            continue;
                        }
                        Err(e) => return Err(e),
                    }
                }
                self.oge_sira = o.sira;
                self.dizin.insert(o.id, o);
                ozet.yeni += 1;
                self.kaydet()?;
            }
            if liste.len() < SAYFA {
                break;
            }
        }
        // 2) arşivlenenler (olay akışı)
        loop {
            let uc = format!(
                "/v1/havuz/olaylar?program={}&since={}&limit={SAYFA}",
                self.program, self.olay_sira
            );
            let y = g.al(&uc)?;
            let liste = dizi(&y, "olaylar")?;
            if liste.is_empty() {
                break;
            }
            for e in liste {
                let sira = e
                    .get("sira")
                    .and_then(Value::as_i64)
                    .ok_or_else(|| HavuzHata::Bicim("olay: sira yok".into()))?;
                if sira <= self.olay_sira {
                    return Err(HavuzHata::Bicim(format!(
                        "sunucu olay sırası geriye gitti: {sira} <= {}",
                        self.olay_sira
                    )));
                }
                if e.get("tip").and_then(Value::as_str) == Some("arsivlendi") {
                    if let Some(id) = e.get("oge_id").and_then(Value::as_i64) {
                        if self.dizin.remove(&id).is_some() {
                            ozet.arsivlenen += 1;
                        }
                    }
                }
                self.olay_sira = sira;
            }
            self.kaydet()?;
            if liste.len() < SAYFA {
                break;
            }
        }
        self.kaydet()?;
        Ok(ozet)
    }

    fn satirlari_indir(&self, g: &dyn Gonderici, o: &Oge) -> Result<usize, HavuzHata> {
        let mut tum: Map<String, Value> = Map::new();
        let mut sonra: Option<String> = None;
        loop {
            let mut uc = format!(
                "/v1/havuz/oge/{}/satirlar?program={}&limit={SAYFA}",
                o.id, self.program
            );
            if let Some(s) = &sonra {
                uc.push_str(&format!("&sonra={}", yuzde_kodla(s)));
            }
            let y = g.al(&uc)?;
            let liste = dizi(&y, "satirlar")?;
            for s in liste {
                let a = s
                    .get("anahtar")
                    .and_then(Value::as_str)
                    .ok_or_else(|| HavuzHata::Bicim("satır: anahtar yok".into()))?;
                tum.insert(a.to_string(), s.get("veri").cloned().unwrap_or(Value::Null));
            }
            if liste.len() < SAYFA {
                break;
            }
            sonra = y
                .get("son_anahtar")
                .and_then(Value::as_str)
                .map(str::to_string);
            if sonra.is_none() {
                return Err(HavuzHata::Bicim("satirlar: son_anahtar yok".into()));
            }
        }
        let n = tum.len();
        let dosya = self.klasor.join("tablo").join(format!("{}.json", o.id));
        disk(
            taa_dosya::uzerine_yaz_atomik(
                &dosya,
                json!({ "satirlar": tum }).to_string().as_bytes(),
            ),
            "tablo dosyası yazılamadı",
        )?;
        Ok(n)
    }

    // ---- çevrimdışı okuma (ağ yok) ----

    /// Önbellekteki etkin öğeler (tür, ad sırasıyla); `tur` verilirse yalnız o tür.
    pub fn ogeler(&self, tur: Option<&str>) -> Vec<&Oge> {
        let mut v: Vec<&Oge> = self
            .dizin
            .values()
            .filter(|o| tur.map(|t| o.tur == t).unwrap_or(true))
            .collect();
        v.sort_by(|a, b| (&a.tur, &a.ad, a.surum_no).cmp(&(&b.tur, &b.ad, b.surum_no)));
        v
    }

    /// (tür, ad) için en yeni sürüm.
    pub fn bul(&self, tur: &str, ad: &str) -> Option<&Oge> {
        self.dizin
            .values()
            .filter(|o| o.tur == tur && o.ad == ad)
            .max_by_key(|o| o.surum_no)
    }

    pub fn oge(&self, id: i64) -> Option<&Oge> {
        self.dizin.get(&id)
    }

    /// Tablo öğesinin tüm satırları (anahtar → veri).
    pub fn satirlar(&self, id: i64) -> Result<Map<String, Value>, HavuzHata> {
        let o = self
            .dizin
            .get(&id)
            .ok_or_else(|| HavuzHata::Yok(format!("öğe {id} önbellekte yok")))?;
        if o.kip != "tablo" {
            return Err(HavuzHata::Yok(format!("öğe {id} tablo değil")));
        }
        let d = self.oku_json(&format!("tablo/{id}.json"))?.ok_or_else(|| {
            HavuzHata::Yok(format!("öğe {id} satırları önbellekte yok; eşitleyin"))
        })?;
        match d.get("satirlar") {
            Some(Value::Object(m)) => Ok(m.clone()),
            _ => Err(HavuzHata::Bicim(format!(
                "tablo/{id}.json: satirlar nesnesi yok"
            ))),
        }
    }

    /// Tek satır.
    pub fn satir(&self, id: i64, anahtar: &str) -> Result<Option<Value>, HavuzHata> {
        Ok(self.satirlar(id)?.get(anahtar).cloned())
    }

    fn icerik_yolu(&self, sha: &str) -> PathBuf {
        self.klasor
            .join("icerik")
            .join(&sha[0..2])
            .join(&sha[2..4])
            .join(sha)
    }

    /// Dosya öğesi tamamen indirilmiş ve doğrulanmışsa yolu; aksi halde None. Ağ yok.
    pub fn dosya_yolu(&self, id: i64) -> Option<PathBuf> {
        let o = self.dizin.get(&id)?;
        let sha = o.sha256.as_deref()?;
        let yol = self.icerik_yolu(sha);
        let uzunluk = fs::metadata(&yol).ok()?.len();
        (Some(uzunluk as i64) == o.boyut).then_some(yol)
    }

    /// Önbellekte olup dizinde artık olmayan (arşivlenmiş) içerik dosyaları; silinmez, yalnız sayılır.
    pub fn yetimler(&self) -> Result<Vec<PathBuf>, HavuzHata> {
        let kullanilan: Vec<&str> = self
            .dizin
            .values()
            .filter_map(|o| o.sha256.as_deref())
            .collect();
        let mut sonuc = Vec::new();
        let kok = self.klasor.join("icerik");
        let Ok(a) = fs::read_dir(&kok) else {
            return Ok(sonuc);
        };
        for k1 in a.flatten() {
            let Ok(b) = fs::read_dir(k1.path()) else {
                continue;
            };
            for k2 in b.flatten() {
                let Ok(c) = fs::read_dir(k2.path()) else {
                    continue;
                };
                for f in c.flatten() {
                    let ad = f.file_name().to_string_lossy().into_owned();
                    if sha_gecerli(&ad) && !kullanilan.contains(&ad.as_str()) {
                        sonuc.push(f.path());
                    }
                }
            }
        }
        sonuc.sort();
        Ok(sonuc)
    }

    // ---- dosya indirme (ağ) ----

    /// Dosya öğesini indirir (1 MiB `Range` parçalarıyla; yarım kalmışsa kaldığı yerden), sha256 ve boyutu doğrular.
    /// Zaten indirilmişse ağa çıkmadan yolu döndürür.
    pub fn indir(&self, g: &dyn Gonderici, id: i64) -> Result<PathBuf, HavuzHata> {
        if let Some(y) = self.dosya_yolu(id) {
            return Ok(y);
        }
        let o = self
            .dizin
            .get(&id)
            .ok_or_else(|| HavuzHata::Yok(format!("öğe {id} önbellekte yok")))?;
        if o.kip != "dosya" {
            return Err(HavuzHata::Yok(format!("öğe {id} dosya değil")));
        }
        let (Some(sha), Some(boyut)) = (o.sha256.as_deref(), o.boyut) else {
            return Err(HavuzHata::Bicim(format!("öğe {id}: sha256/boyut yok")));
        };
        let boyut = u64::try_from(boyut).map_err(|_| HavuzHata::Bicim("boyut negatif".into()))?;
        let bekleme = self.klasor.join("icerik").join(format!("{sha}.part"));
        disk(
            fs::create_dir_all(bekleme.parent().unwrap_or(&self.klasor)),
            "içerik klasörü açılamadı",
        )?;
        let mut mevcut = fs::metadata(&bekleme).map(|m| m.len()).unwrap_or(0);
        if mevcut > boyut {
            // Tutarsız yarım dosya: sıfırla (sunucudaki veriye dokunulmaz).
            disk(fs::File::create(&bekleme), "yarım dosya sıfırlanamadı")?;
            mevcut = 0;
        }
        let uc = format!("/v1/havuz/icerik/{sha}?program={}", self.program);
        while mevcut < boyut {
            let bit = (mevcut + PARCA).min(boyut) - 1;
            let parca = g.al_ham(&uc, Some((mevcut, bit)))?;
            let beklenen = bit - mevcut + 1;
            if parca.is_empty() || parca.len() as u64 > beklenen {
                return Err(HavuzHata::Bicim(format!(
                    "parça boyutu beklenenle uyuşmuyor ({} / en çok {beklenen})",
                    parca.len()
                )));
            }
            let mut f = disk(
                fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&bekleme),
                "yarım dosya açılamadı",
            )?;
            disk(f.write_all(&parca), "yarım dosyaya yazılamadı")?;
            disk(f.flush(), "yarım dosya yazılamadı")?;
            mevcut += parca.len() as u64;
        }
        let bulunan = dosya_ozeti(&bekleme)?;
        if bulunan != sha {
            disk(fs::File::create(&bekleme), "yarım dosya sıfırlanamadı")?;
            return Err(HavuzHata::Ozet {
                beklenen: sha.to_string(),
                bulunan,
            });
        }
        let hedef = self.icerik_yolu(sha);
        disk(
            fs::create_dir_all(hedef.parent().unwrap_or(&self.klasor)),
            "içerik klasörü açılamadı",
        )?;
        if hedef.exists() {
            // Aynı içerik zaten var (başka öğe aynı dosyayı kullanıyor): bekleme kopyası gereksiz.
            let _ = fs::remove_file(&bekleme);
        } else {
            disk(fs::rename(&bekleme, &hedef), "içerik yerine taşınamadı")?;
        }
        Ok(hedef)
    }
}

fn dizi<'a>(y: &'a Value, alan: &str) -> Result<&'a Vec<Value>, HavuzHata> {
    y.get(alan)
        .and_then(Value::as_array)
        .ok_or_else(|| HavuzHata::Bicim(format!("sunucu yanıtında '{alan}' dizisi yok")))
}

fn dosya_ozeti(yol: &Path) -> Result<String, HavuzHata> {
    let mut f = disk(fs::File::open(yol), "dosya açılamadı")?;
    let mut h = Sha256::new();
    let mut tampon = vec![0u8; 1024 * 1024];
    loop {
        let n = disk(f.read(&mut tampon), "dosya okunamadı")?;
        if n == 0 {
            break;
        }
        h.update(&tampon[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

#[cfg(test)]
mod testler;
