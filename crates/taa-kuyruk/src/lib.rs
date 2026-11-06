// Sahip: Yusuf. yazan: [Claude#taa-kuyruk-T6 2026-11-06]
//! taa-kuyruk: merkez sunucuya giden yerel kuyruk (bkz. docs/T6-KUYRUK.md).
//!
//! Her program kendi kuyruk klasörüne (`<veri kökü>/kuyruk/<program>/`) kayıt ekler; sunucu yoksa ya da
//! bağlantı kesikse kayıtlar yerelde birikir. `gonder` bağlantı gelince bekleyenleri sırayla sunucuya yollar.
//! Her kaydın `no` değeri tektir; sunucu sözleşmesi (API-SOZLESMESI.md) aynı `no` ile gelen kaydı ikinci kez
//! yazmaz, bu yüzden yarım kalan gönderim yeniden denendiğinde çift kayıt oluşmaz. Aynı `no` kuyruğa ikinci
//! kez eklenmez.
//!
//! Dosyalar (hepsi yalnız eklenen JSONL; hiçbir satır silinmez, hiçbir dosya ezilmez):
//! - `giden.jsonl`     `{"no","uc","zaman","govde"}` — eklenen her kayıt.
//! - `gonderildi.jsonl` `{"no","zaman","yanit"}` — sunucunun kabul ettiği kayıtlar.
//! - `reddedildi.jsonl` `{"no","zaman","kod","mesaj"}` — sunucunun kalıcı olarak reddettiği (4xx) kayıtlar;
//!   bunlar yeniden denenmez, kişi bakar.
//!
//! Sunucuya erişilemiyorsa (ağ yok, 5xx, zaman aşımı) gönderim o kayıtta durur; sıra bozulmaz.
//! HTTP için yeni crate yok: `CurlGonderici` `curl` komutunu çalıştırır. Oturum anahtarı komut satırına
//! yazılmaz (geçici başlık dosyası ile verilir) ve bu crate onu hiçbir yere kaydetmez.
//! Özellik anahtarı: "kuyruk".

mod gonderici;
pub mod zaman;

#[cfg(test)]
mod testler;

pub use gonderici::{CurlGonderici, GonderHata, Gonderici, Imzalayici};

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const OZELLIK: &str = "kuyruk";
/// Kuyruk klasörünün veri kökü altındaki adı.
pub const KLASOR: &str = "kuyruk";
/// `no` ve program adı uzunluk sınırı.
pub const NO_EN_COK: usize = 64;
/// Bir kaydın gövdesi (JSON metni) en çok bu kadar bayt olabilir.
pub const GOVDE_EN_COK: usize = 1024 * 1024;

#[derive(Debug)]
pub enum KuyrukHata {
    /// `no` `[A-Za-z0-9._-]`, 1..=64 kuralına uymuyor.
    GecersizNo(String),
    /// Uç `/` ile başlamalı, yalnız `[A-Za-z0-9/_-]` içermeli, `..` ve `//` olmamalı.
    GecersizUc(String),
    /// Program adı `[a-z0-9-]`, 1..=64 kuralına uymuyor.
    GecersizProgram(String),
    /// Gövde nesne (`{}`) olmalı ve `GOVDE_EN_COK` baytı aşmamalı.
    GecersizGovde(String),
    /// Bozuk JSONL satırı; `dosya` adı ve 1'den başlayan satır.
    BozukSatir {
        dosya: String,
        satir: usize,
    },
    Io(io::Error),
}

impl fmt::Display for KuyrukHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KuyrukHata::GecersizNo(n) => {
                write!(
                    f,
                    "geçersiz no (yalnız [A-Za-z0-9._-], en çok {NO_EN_COK}): {n}"
                )
            }
            KuyrukHata::GecersizUc(u) => write!(f, "geçersiz uç (örnek /v1/olay): {u}"),
            KuyrukHata::GecersizProgram(p) => {
                write!(
                    f,
                    "geçersiz program adı (yalnız [a-z0-9-], en çok {NO_EN_COK}): {p}"
                )
            }
            KuyrukHata::GecersizGovde(m) => write!(f, "geçersiz gövde: {m}"),
            KuyrukHata::BozukSatir { dosya, satir } => {
                write!(
                    f,
                    "{dosya} bozuk (satır {satir}); Öneri: dosyayı elle denetleyin, satır silinmez"
                )
            }
            KuyrukHata::Io(e) => write!(f, "dosya hatası: {e}"),
        }
    }
}

impl std::error::Error for KuyrukHata {}

impl From<io::Error> for KuyrukHata {
    fn from(e: io::Error) -> Self {
        KuyrukHata::Io(e)
    }
}

/// Kuyruğa eklenmiş bir kayıt.
#[derive(Debug, Clone, PartialEq)]
pub struct Kayit {
    pub no: String,
    pub uc: String,
    pub zaman: String,
    pub govde: Value,
}

/// Gönderilmiş kaydın izi.
#[derive(Debug, Clone, PartialEq)]
pub struct Gonderim {
    pub no: String,
    pub zaman: String,
    pub yanit: Value,
}

/// Sunucunun kalıcı olarak reddettiği kayıt.
#[derive(Debug, Clone, PartialEq)]
pub struct Ret {
    pub no: String,
    pub zaman: String,
    pub kod: u16,
    pub mesaj: String,
}

/// `ekle` sonucu.
#[derive(Debug, Clone, PartialEq)]
pub enum Ekleme {
    Eklendi(Kayit),
    /// Aynı `no` zaten kuyrukta (gönderilmiş ya da bekliyor); yeni satır yazılmadı.
    ZatenVar(Kayit),
}

/// Bir kaydın durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durum {
    Bekliyor,
    Gonderildi,
    Reddedildi,
}

impl Durum {
    pub fn metin(self) -> &'static str {
        match self {
            Durum::Bekliyor => "bekliyor",
            Durum::Gonderildi => "gonderildi",
            Durum::Reddedildi => "reddedildi",
        }
    }
}

/// `gonder` özeti.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GonderOzeti {
    pub gonderilen: usize,
    pub reddedilen: usize,
    pub kalan: usize,
    /// Sunucuya ulaşılamadıysa neden; gönderim o noktada durdu.
    pub durma_nedeni: Option<String>,
}

/// Tek bir programın kuyruğu (`<klasör>/giden.jsonl` vb.).
#[derive(Debug, Clone)]
pub struct Kuyruk {
    klasor: PathBuf,
}

pub fn no_gecerli(no: &str) -> bool {
    !no.is_empty()
        && no.len() <= NO_EN_COK
        && no
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
        && no != "."
        && no != ".."
}

pub fn uc_gecerli(uc: &str) -> bool {
    uc.starts_with('/')
        && uc.len() <= 256
        && !uc.contains("..")
        && !uc.contains("//")
        && uc
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'_' | b'-'))
}

pub fn program_gecerli(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= NO_EN_COK
        && p.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn deger_metin(v: &Value, anahtar: &str) -> Option<String> {
    v.get(anahtar).and_then(Value::as_str).map(str::to_string)
}

fn satirlar(yol: &Path) -> Result<Vec<(usize, Value)>, KuyrukHata> {
    let ad = yol
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut cikti = Vec::new();
    for (i, s) in taa_dosya::satirlari_oku(yol)?.into_iter().enumerate() {
        let v: Value = serde_json::from_str(&s).map_err(|_| KuyrukHata::BozukSatir {
            dosya: ad.clone(),
            satir: i + 1,
        })?;
        cikti.push((i + 1, v));
    }
    Ok(cikti)
}

impl Kuyruk {
    /// Verilen klasörü kuyruk olarak açar (klasör ilk yazımda oluşur).
    pub fn ac(klasor: impl Into<PathBuf>) -> Kuyruk {
        Kuyruk {
            klasor: klasor.into(),
        }
    }

    /// `<kok>/kuyruk/<program>/`.
    pub fn program(kok: &Path, program: &str) -> Result<Kuyruk, KuyrukHata> {
        if !program_gecerli(program) {
            return Err(KuyrukHata::GecersizProgram(program.to_string()));
        }
        Ok(Kuyruk::ac(kok.join(KLASOR).join(program)))
    }

    /// `<kok>/kuyruk/` altındaki tüm program kuyrukları (ad sırasıyla).
    pub fn hepsi(kok: &Path) -> Result<Vec<(String, Kuyruk)>, KuyrukHata> {
        let k = kok.join(KLASOR);
        let mut liste = Vec::new();
        let okunan = match fs::read_dir(&k) {
            Ok(r) => r,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(liste),
            Err(e) => return Err(e.into()),
        };
        for g in okunan {
            let g = g?;
            if !g.file_type()?.is_dir() {
                continue;
            }
            let ad = g.file_name().to_string_lossy().into_owned();
            if program_gecerli(&ad) {
                liste.push((ad, Kuyruk::ac(g.path())));
            }
        }
        liste.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(liste)
    }

    pub fn klasor(&self) -> &Path {
        &self.klasor
    }

    fn giden_yolu(&self) -> PathBuf {
        self.klasor.join("giden.jsonl")
    }

    fn gonderildi_yolu(&self) -> PathBuf {
        self.klasor.join("gonderildi.jsonl")
    }

    fn reddedildi_yolu(&self) -> PathBuf {
        self.klasor.join("reddedildi.jsonl")
    }

    /// Kaydı kuyruğa ekler. Aynı `no` varsa yeni satır yazmaz, `Ekleme::ZatenVar` döner.
    pub fn ekle(&self, uc: &str, no: &str, govde: Value) -> Result<Ekleme, KuyrukHata> {
        if !no_gecerli(no) {
            return Err(KuyrukHata::GecersizNo(no.to_string()));
        }
        if !uc_gecerli(uc) {
            return Err(KuyrukHata::GecersizUc(uc.to_string()));
        }
        if !govde.is_object() {
            return Err(KuyrukHata::GecersizGovde("nesne ({}) olmalı".into()));
        }
        let kayit = Kayit {
            no: no.to_string(),
            uc: uc.to_string(),
            zaman: zaman::simdi(),
            govde,
        };
        let satir =
            json!({"no": kayit.no, "uc": kayit.uc, "zaman": kayit.zaman, "govde": kayit.govde})
                .to_string();
        if satir.len() > GOVDE_EN_COK {
            return Err(KuyrukHata::GecersizGovde(format!(
                "{} bayt; sınır {GOVDE_EN_COK}",
                satir.len()
            )));
        }
        if let Some(eski) = self.kayitlar()?.into_iter().find(|k| k.no == no) {
            return Ok(Ekleme::ZatenVar(eski));
        }
        taa_dosya::satir_ekle(&self.giden_yolu(), &satir)?;
        Ok(Ekleme::Eklendi(kayit))
    }

    /// `giden.jsonl`deki tüm kayıtlar (ekleme sırasıyla).
    pub fn kayitlar(&self) -> Result<Vec<Kayit>, KuyrukHata> {
        let yol = self.giden_yolu();
        let mut v = Vec::new();
        for (satir, d) in satirlar(&yol)? {
            let bozuk = || KuyrukHata::BozukSatir {
                dosya: "giden.jsonl".into(),
                satir,
            };
            let no = deger_metin(&d, "no").ok_or_else(bozuk)?;
            let uc = deger_metin(&d, "uc").ok_or_else(bozuk)?;
            let zaman = deger_metin(&d, "zaman").unwrap_or_default();
            let govde = d.get("govde").cloned().ok_or_else(bozuk)?;
            if !no_gecerli(&no) || !uc_gecerli(&uc) || !govde.is_object() {
                return Err(bozuk());
            }
            v.push(Kayit {
                no,
                uc,
                zaman,
                govde,
            });
        }
        Ok(v)
    }

    pub fn gonderilenler(&self) -> Result<BTreeMap<String, Gonderim>, KuyrukHata> {
        let mut h = BTreeMap::new();
        for (satir, d) in satirlar(&self.gonderildi_yolu())? {
            let no = deger_metin(&d, "no").ok_or(KuyrukHata::BozukSatir {
                dosya: "gonderildi.jsonl".into(),
                satir,
            })?;
            h.insert(
                no.clone(),
                Gonderim {
                    no,
                    zaman: deger_metin(&d, "zaman").unwrap_or_default(),
                    yanit: d.get("yanit").cloned().unwrap_or(Value::Null),
                },
            );
        }
        Ok(h)
    }

    pub fn reddedilenler(&self) -> Result<BTreeMap<String, Ret>, KuyrukHata> {
        let mut h = BTreeMap::new();
        for (satir, d) in satirlar(&self.reddedildi_yolu())? {
            let no = deger_metin(&d, "no").ok_or(KuyrukHata::BozukSatir {
                dosya: "reddedildi.jsonl".into(),
                satir,
            })?;
            h.insert(
                no.clone(),
                Ret {
                    no,
                    zaman: deger_metin(&d, "zaman").unwrap_or_default(),
                    kod: d.get("kod").and_then(Value::as_u64).unwrap_or(0) as u16,
                    mesaj: deger_metin(&d, "mesaj").unwrap_or_default(),
                },
            );
        }
        Ok(h)
    }

    /// Her kaydın durumu (ekleme sırasıyla).
    pub fn durumlar(&self) -> Result<Vec<(Kayit, Durum)>, KuyrukHata> {
        let g = self.gonderilenler()?;
        let r = self.reddedilenler()?;
        Ok(self
            .kayitlar()?
            .into_iter()
            .map(|k| {
                let d = if g.contains_key(&k.no) {
                    Durum::Gonderildi
                } else if r.contains_key(&k.no) {
                    Durum::Reddedildi
                } else {
                    Durum::Bekliyor
                };
                (k, d)
            })
            .collect())
    }

    /// Henüz gönderilmemiş ve reddedilmemiş kayıtlar (ekleme sırasıyla).
    pub fn bekleyenler(&self) -> Result<Vec<Kayit>, KuyrukHata> {
        Ok(self
            .durumlar()?
            .into_iter()
            .filter(|(_, d)| *d == Durum::Bekliyor)
            .map(|(k, _)| k)
            .collect())
    }

    /// Bekleyenleri sırayla gönderir; en çok `en_cok` kayıt (0 = sınırsız). Sunucuya ulaşılamazsa durur,
    /// kalanlar bir sonraki çağrıya kalır. Kabul edilen kayıt `gonderildi.jsonl`e, kalıcı reddedilen
    /// `reddedildi.jsonl`e yazılır; `giden.jsonl` değişmez.
    pub fn gonder(&self, g: &dyn Gonderici, en_cok: usize) -> Result<GonderOzeti, KuyrukHata> {
        let bekleyen = self.bekleyenler()?;
        let mut ozet = GonderOzeti {
            kalan: bekleyen.len(),
            ..GonderOzeti::default()
        };
        for (i, k) in bekleyen.iter().enumerate() {
            if en_cok != 0 && i >= en_cok {
                break;
            }
            match g.gonder(&k.uc, &k.govde) {
                Ok(yanit) => {
                    let s =
                        json!({"no": k.no, "zaman": zaman::simdi(), "yanit": yanit}).to_string();
                    taa_dosya::satir_ekle(&self.gonderildi_yolu(), &s)?;
                    ozet.gonderilen += 1;
                    ozet.kalan -= 1;
                }
                Err(GonderHata::Reddedildi { kod, mesaj }) => {
                    let s =
                        json!({"no": k.no, "zaman": zaman::simdi(), "kod": kod, "mesaj": mesaj})
                            .to_string();
                    taa_dosya::satir_ekle(&self.reddedildi_yolu(), &s)?;
                    ozet.reddedilen += 1;
                    ozet.kalan -= 1;
                }
                Err(e) => {
                    ozet.durma_nedeni = Some(e.to_string());
                    break;
                }
            }
        }
        Ok(ozet)
    }
}
