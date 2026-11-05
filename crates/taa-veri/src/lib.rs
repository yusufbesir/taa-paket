// Sahip: Yusuf. yazan: [Claude#taa-paket-T2 2026-11-05]
//! taa-veri: paylaşılan veri alanları (bkz. docs/veri-alani.md).
//!
//! Her alanın tek yazarı (`sahip`) vardır; okuyanlar alan dosyalarına yazamaz, değişiklik isterse `oneriler/`
//! altına öneri bırakır. Sahiplik `<kok>/<alan>/SAHIP.toml` ile kayıtlıdır (düz TOML alt kümesi).
//! Yazımlar atomiktir (`taa_dosya::uzerine_yaz_atomik`). Bu crate hiçbir dosyayı silmez: devralınan SAHIP.toml
//! ve kapatılan öneriler `eski/` altına taşınır. Alan ve dosya adları yol kaçışına karşı denetlenir.
//! Özellik anahtarı: "veri" (varsayılan açık).

mod toml;
mod zaman;

#[cfg(test)]
mod testler;

pub use toml::{sahip_toml_coz, sahip_toml_yaz, Deger};

use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub const OZELLIK: &str = "veri";
/// SAHIP.toml şema sürümü.
pub const SEMA: i64 = 1;
/// Bir öneri dosyasının en büyük boyutu (bayt).
pub const ONERI_EN_COK: usize = 256 * 1024;
/// Alan ve program adı uzunluk sınırı.
const AD_EN_COK: usize = 32;

#[derive(Debug)]
pub enum VeriHata {
    /// Alanın asıl (geçici olmayan) sahibi başka program; alan salt okunur açılmalı.
    BaskaSahip {
        alan: String,
        sahip: String,
    },
    /// Okunmak istenen alanda SAHIP.toml yok.
    SahipYok,
    /// Alan ya da program adı `[a-z0-9-]`, 1..=32 kuralına uymuyor.
    GecersizAd(String),
    /// Bozuk SAHIP.toml; `satir` 1'den başlar, satıra bağlanamayan (eksik anahtar vb.) hatalarda 0.
    GecersizToml {
        satir: usize,
        neden: String,
    },
    /// Dosya yolu alanın dışına çıkıyor (`..`, mutlak yol, ayırıcı/sürücü karakteri, ayrılmış ad).
    YolDisari(String),
    /// Öneri en çok `ONERI_EN_COK` bayt olabilir.
    OneriBuyuk(usize),
    Io(io::Error),
}

impl fmt::Display for VeriHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VeriHata::BaskaSahip { alan, sahip } => {
                write!(f, "'{alan}' alanının sahibi başka program: {sahip}")
            }
            VeriHata::SahipYok => write!(f, "alanın SAHIP.toml kaydı yok"),
            VeriHata::GecersizAd(a) => write!(f, "geçersiz ad (yalnız [a-z0-9-], en çok 32): {a}"),
            VeriHata::GecersizToml { satir, neden } => {
                write!(f, "SAHIP.toml geçersiz (satır {satir}): {neden}")
            }
            VeriHata::YolDisari(y) => write!(f, "dosya yolu alan dışına çıkıyor: {y}"),
            VeriHata::OneriBuyuk(n) => {
                write!(f, "öneri çok büyük: {n} bayt (sınır {ONERI_EN_COK})")
            }
            VeriHata::Io(e) => write!(f, "dosya hatası: {e}"),
        }
    }
}

impl std::error::Error for VeriHata {}

impl From<io::Error> for VeriHata {
    fn from(e: io::Error) -> Self {
        VeriHata::Io(e)
    }
}

/// SAHIP.toml içeriği.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SahipKaydi {
    pub sema: i64,
    pub alan: String,
    pub sahip: String,
    pub gecici: bool,
    pub yazan_surum: String,
    pub son_yazim: String,
}

/// Okuyanın bıraktığı değişiklik önerisi (`oneriler/<id>.json`).
#[derive(Debug, Clone, PartialEq)]
pub struct Oneri {
    pub id: String,
    pub oneren: String,
    pub zaman: String,
    pub degisiklik: Value,
}

fn ad_gecerli(ad: &str) -> bool {
    !ad.is_empty()
        && ad.len() <= AD_EN_COK
        && ad
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn ad_denetle(ad: &str) -> Result<(), VeriHata> {
    if ad_gecerli(ad) {
        Ok(())
    } else {
        Err(VeriHata::GecersizAd(ad.to_string()))
    }
}

/// Alan içi göreli dosya yolunu denetler. `yazma` ise ayrılmış üst adlar (SAHIP.toml, oneriler, eski) yasaktır.
fn yol_denetle(dosya: &str, yazma: bool) -> Result<PathBuf, VeriHata> {
    let kotu = || VeriHata::YolDisari(dosya.to_string());
    if dosya.is_empty() || dosya.contains(['\\', ':', '\0']) || dosya.starts_with('/') {
        return Err(kotu());
    }
    let mut yol = PathBuf::new();
    for (i, parca) in dosya.split('/').enumerate() {
        if parca.is_empty() || parca == "." || parca == ".." {
            return Err(kotu());
        }
        if i == 0 && yazma && ["SAHIP.toml", "oneriler", "eski"].contains(&parca) {
            return Err(kotu());
        }
        yol.push(parca);
    }
    Ok(yol)
}

fn kimlik_gecerli(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && !id.starts_with('.')
        && !id.contains("..")
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
}

/// `<dizin>/eski/<ad>-<zaman>` altına taşır (yeniden adlandırma; silme yok). Hedef adı doluysa `-2`, `-3` eklenir.
fn eskiye_tasi(dizin: &Path, kaynak: &Path, ad: &str) -> io::Result<PathBuf> {
    let eski = dizin.join("eski");
    fs::create_dir_all(&eski)?;
    let damga = zaman::simdi().1;
    let mut n = 1u32;
    loop {
        let son = if n == 1 {
            format!("{ad}-{damga}")
        } else {
            format!("{ad}-{damga}-{n}")
        };
        let hedef = eski.join(son);
        if !hedef.exists() {
            fs::rename(kaynak, &hedef)?;
            return Ok(hedef);
        }
        n += 1;
    }
}

fn kayit_cikar(
    harita: &BTreeMap<String, Deger>,
    beklenen_alan: &str,
) -> Result<SahipKaydi, VeriHata> {
    let eksik = |a: &str| VeriHata::GecersizToml {
        satir: 0,
        neden: format!("zorunlu anahtar eksik ya da yanlış türde: {a}"),
    };
    let metin = |a: &str| match harita.get(a) {
        Some(Deger::Metin(m)) => Ok(m.clone()),
        _ => Err(eksik(a)),
    };
    let sema = match harita.get("sema") {
        Some(Deger::Sayi(n)) => *n,
        _ => return Err(eksik("sema")),
    };
    if sema != SEMA {
        return Err(VeriHata::GecersizToml {
            satir: 0,
            neden: format!("desteklenmeyen sema: {sema}"),
        });
    }
    let alan = metin("alan")?;
    if alan != beklenen_alan {
        return Err(VeriHata::GecersizToml {
            satir: 0,
            neden: format!("alan adı klasörle uyuşmuyor: {alan}"),
        });
    }
    let sahip = metin("sahip")?;
    ad_denetle(&sahip).map_err(|_| VeriHata::GecersizToml {
        satir: 0,
        neden: format!("sahip adı geçersiz: {sahip}"),
    })?;
    let gecici = match harita.get("gecici") {
        None => false,
        Some(Deger::Mantik(b)) => *b,
        _ => return Err(eksik("gecici")),
    };
    Ok(SahipKaydi {
        sema,
        alan,
        sahip,
        gecici,
        yazan_surum: metin("yazan_surum").unwrap_or_default(),
        son_yazim: metin("son_yazim").unwrap_or_default(),
    })
}

/// (çözülmüş kayıt, ham anahtar haritası): bilinmeyen anahtarlar güncellemede korunur.
type OkunanKayit = (SahipKaydi, BTreeMap<String, Deger>);

fn kayit_oku(dizin: &Path, alan: &str) -> Result<Option<OkunanKayit>, VeriHata> {
    let yol = dizin.join("SAHIP.toml");
    let metin = match fs::read(&yol) {
        Ok(b) => String::from_utf8(b).map_err(|_| VeriHata::GecersizToml {
            satir: 0,
            neden: "UTF-8 değil".to_string(),
        })?,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    let harita = sahip_toml_coz(&metin)?;
    let kayit = kayit_cikar(&harita, alan)?;
    Ok(Some((kayit, harita)))
}

fn kayit_yaz(
    dizin: &Path,
    alan: &str,
    program: &str,
    surum: &str,
    gecici: bool,
    eski_harita: Option<BTreeMap<String, Deger>>,
) -> Result<SahipKaydi, VeriHata> {
    let mut harita = eski_harita.unwrap_or_default();
    let simdi = zaman::simdi().0;
    harita.insert("sema".into(), Deger::Sayi(SEMA));
    harita.insert("alan".into(), Deger::Metin(alan.into()));
    harita.insert("sahip".into(), Deger::Metin(program.into()));
    harita.insert("gecici".into(), Deger::Mantik(gecici));
    harita.insert("yazan_surum".into(), Deger::Metin(surum.into()));
    harita.insert("son_yazim".into(), Deger::Metin(simdi));
    taa_dosya::uzerine_yaz_atomik(
        &dizin.join("SAHIP.toml"),
        sahip_toml_yaz(&harita).as_bytes(),
    )?;
    kayit_cikar(&harita, alan)
}

/// Veri kökü: alanlar `<kok>/<alan>/` altındadır. `rehber` alanı istisnadır (aşağıya bakın).
#[derive(Debug, Clone)]
pub struct VeriKok {
    kok: PathBuf,
    rehber: PathBuf,
}

impl VeriKok {
    /// `TAA_VERI_KOK` → yoksa Windows'ta `%APPDATA%\TAA\veri`, diğerlerinde `$XDG_DATA_HOME|~/.local/share` + `/TAA/veri`.
    /// `TAA_VERI_KOK` verilmişse `rehber` alanı da onun altındadır (yalıtım); verilmemişse mevcut konumunda kalır:
    /// `%APPDATA%\TAA\rehber` (Windows) / `<veri tabanı>/TAA/rehber`. Hiçbiri çözülemezse göreli `taa-veri`.
    pub fn ortamdan() -> VeriKok {
        let ortam = |a: &str| std::env::var_os(a).filter(|v| !v.is_empty());
        if let Some(k) = ortam("TAA_VERI_KOK") {
            return VeriKok::yeni(PathBuf::from(k));
        }
        let taban = if cfg!(windows) {
            ortam("APPDATA").map(PathBuf::from)
        } else {
            ortam("XDG_DATA_HOME")
                .map(PathBuf::from)
                .or_else(|| ortam("HOME").map(|h| PathBuf::from(h).join(".local").join("share")))
        };
        match taban {
            Some(t) => VeriKok {
                kok: t.join("TAA").join("veri"),
                rehber: t.join("TAA").join("rehber"),
            },
            None => VeriKok::yeni("taa-veri"),
        }
    }

    /// Verilen kökle açar; `rehber` de `<yol>/rehber` olur.
    pub fn yeni(yol: impl Into<PathBuf>) -> VeriKok {
        let kok: PathBuf = yol.into();
        VeriKok {
            rehber: kok.join("rehber"),
            kok,
        }
    }

    pub fn kok(&self) -> &Path {
        &self.kok
    }

    /// Alanın klasörü (adı denetler; klasörü oluşturmaz).
    pub fn alan_yolu(&self, alan: &str) -> Result<PathBuf, VeriHata> {
        ad_denetle(alan)?;
        Ok(if alan == "rehber" {
            self.rehber.clone()
        } else {
            self.kok.join(alan)
        })
    }

    /// Alanın asıl sahibi olarak açar (kural 2). SAHIP.toml yoksa yazılır; sahip bu program ise güncellenir;
    /// sahip başka ve `gecici = true` ise devralınır (eski kayıt `eski/`ye); aksi halde `BaskaSahip`.
    pub fn ac_sahip(&self, alan: &str, program: &str, surum: &str) -> Result<Sahip, VeriHata> {
        self.sahip_ac(alan, program, surum, false)
    }

    /// Geçici sahip olarak açar (`gecici = true`): asıl sahip kurulunca onu devralır. Başka bir sahip (geçici de olsa) varsa `BaskaSahip`.
    pub fn ac_gecici_sahip(
        &self,
        alan: &str,
        program: &str,
        surum: &str,
    ) -> Result<Sahip, VeriHata> {
        self.sahip_ac(alan, program, surum, true)
    }

    fn sahip_ac(
        &self,
        alan: &str,
        program: &str,
        surum: &str,
        gecici: bool,
    ) -> Result<Sahip, VeriHata> {
        ad_denetle(program)?;
        let dizin = self.alan_yolu(alan)?;
        fs::create_dir_all(&dizin)?;
        let kayit = match kayit_oku(&dizin, alan)? {
            None => kayit_yaz(&dizin, alan, program, surum, gecici, None)?,
            Some((k, harita)) if k.sahip == program => {
                kayit_yaz(&dizin, alan, program, surum, gecici, Some(harita))?
            }
            Some((k, _)) if k.gecici && !gecici => {
                eskiye_tasi(&dizin, &dizin.join("SAHIP.toml"), "SAHIP.toml")?;
                kayit_yaz(&dizin, alan, program, surum, gecici, None)?
            }
            Some((k, _)) => {
                return Err(VeriHata::BaskaSahip {
                    alan: alan.to_string(),
                    sahip: k.sahip,
                })
            }
        };
        Ok(Sahip {
            dizin,
            program: program.to_string(),
            kayit,
        })
    }

    /// Salt okunur açar. SAHIP.toml yoksa `SahipYok`. Okuyan alan dosyası yazamaz; yalnız `Okur::oner`.
    pub fn ac_okur(&self, alan: &str, program: &str) -> Result<Okur, VeriHata> {
        ad_denetle(program)?;
        let dizin = self.alan_yolu(alan)?;
        let (kayit, _) = kayit_oku(&dizin, alan)?.ok_or(VeriHata::SahipYok)?;
        Ok(Okur {
            dizin,
            program: program.to_string(),
            kayit,
        })
    }
}

/// Alanın yazarı. Alan dosyalarını yalnız bu tür yazar.
#[derive(Debug)]
pub struct Sahip {
    dizin: PathBuf,
    program: String,
    kayit: SahipKaydi,
}

impl Sahip {
    pub fn kayit(&self) -> &SahipKaydi {
        &self.kayit
    }

    pub fn dizin(&self) -> &Path {
        &self.dizin
    }

    pub fn program(&self) -> &str {
        &self.program
    }

    /// Alan içine atomik yazar (geçici dosya + yeniden adlandırma). `..`, mutlak yol ve ayrılmış adlar reddedilir.
    pub fn yaz(&self, dosya: &str, bytes: &[u8]) -> Result<(), VeriHata> {
        let yol = self.dizin.join(yol_denetle(dosya, true)?);
        taa_dosya::uzerine_yaz_atomik(&yol, bytes)?;
        Ok(())
    }

    /// Okuma kolaylığı (sahip de kendi dosyasını okur).
    pub fn oku(&self, dosya: &str) -> Result<Vec<u8>, VeriHata> {
        Ok(fs::read(self.dizin.join(yol_denetle(dosya, false)?))?)
    }

    /// Bekleyen öneriler (kimliğe göre sıralı). Bozuk ya da `ONERI_EN_COK`'tan büyük dosyalar atlanır.
    pub fn oneriler(&self) -> Vec<Oneri> {
        let mut liste = Vec::new();
        let Ok(girdiler) = fs::read_dir(self.dizin.join("oneriler")) else {
            return liste;
        };
        for g in girdiler.flatten() {
            let yol = g.path();
            let Some(id) = yol
                .file_name()
                .and_then(|a| a.to_str())
                .and_then(|a| a.strip_suffix(".json"))
                .map(str::to_string)
            else {
                continue;
            };
            let boyut_uygun = g
                .metadata()
                .map(|m| m.is_file() && m.len() as usize <= ONERI_EN_COK)
                .unwrap_or(false);
            if !boyut_uygun || !kimlik_gecerli(&id) {
                continue;
            }
            let Ok(bayt) = fs::read(&yol) else { continue };
            let Ok(Value::Object(o)) = serde_json::from_slice::<Value>(&bayt) else {
                continue;
            };
            let metin = |a: &str| o.get(a).and_then(Value::as_str).map(str::to_string);
            let (Some(oneren), Some(degisiklik)) = (metin("oneren"), o.get("degisiklik")) else {
                continue;
            };
            liste.push(Oneri {
                id,
                oneren,
                zaman: metin("zaman").unwrap_or_default(),
                degisiklik: degisiklik.clone(),
            });
        }
        liste.sort_by(|a, b| a.id.cmp(&b.id));
        liste
    }

    /// Öneriyi uygulandı/reddedildi olarak kapatır: `oneriler/<id>.json` → `oneriler/eski/` (silme yok).
    pub fn oneri_kapat(&self, id: &str) -> Result<(), VeriHata> {
        if !kimlik_gecerli(id) {
            return Err(VeriHata::YolDisari(id.to_string()));
        }
        let klasor = self.dizin.join("oneriler");
        let kaynak = klasor.join(format!("{id}.json"));
        if !kaynak.is_file() {
            return Err(VeriHata::Io(io::Error::new(
                io::ErrorKind::NotFound,
                "öneri yok",
            )));
        }
        eskiye_tasi(&klasor, &kaynak, &format!("{id}.json"))?;
        Ok(())
    }
}

/// Alanın salt okunur okuyucusu. API'de alan dosyası yazan hiçbir işlev yoktur.
#[derive(Debug)]
pub struct Okur {
    dizin: PathBuf,
    program: String,
    kayit: SahipKaydi,
}

impl Okur {
    pub fn sahip(&self) -> SahipKaydi {
        self.kayit.clone()
    }

    pub fn dizin(&self) -> &Path {
        &self.dizin
    }

    pub fn oku(&self, dosya: &str) -> Result<Vec<u8>, VeriHata> {
        Ok(fs::read(self.dizin.join(yol_denetle(dosya, false)?))?)
    }

    /// Sahibe değişiklik önerir; yalnız `oneriler/<id>.json` yazılır. `degisiklik` zarfa konur:
    /// `{"sema":1,"oneren":..,"zaman":..,"degisiklik":..}`. Dönen değer öneri kimliğidir.
    pub fn oner(&self, degisiklik: Value) -> Result<String, VeriHata> {
        let zarf = serde_json::json!({
            "sema": SEMA,
            "oneren": self.program,
            "zaman": zaman::simdi().0,
            "degisiklik": degisiklik,
        });
        let bayt = serde_json::to_vec_pretty(&zarf).map_err(io::Error::other)?;
        if bayt.len() > ONERI_EN_COK {
            return Err(VeriHata::OneriBuyuk(bayt.len()));
        }
        let klasor = self.dizin.join("oneriler");
        fs::create_dir_all(&klasor)?;
        let damga = zaman::simdi().1;
        let mut n = 1u32;
        let (id, yol) = loop {
            let id = format!("{damga}-{}-{}-{n}", self.program, std::process::id());
            let yol = klasor.join(format!("{id}.json"));
            if !yol.exists() {
                break (id, yol);
            }
            n += 1;
        };
        taa_dosya::uzerine_yaz_atomik(&yol, &bayt)?;
        Ok(id)
    }

    /// taa-kur `kayit.json` dosyasında sahip program kurulu (seçili ya da kurulu sürümü var) mı? Dosya yoksa/okunamazsa `false`
    /// (kural 3: okuyan son kopyayı salt okunur kullanır, arayüz "sahip kurulu değil" der).
    pub fn sahip_kurulu(&self, kayit_json: &Path) -> bool {
        let Ok(bayt) = fs::read(kayit_json) else {
            return false;
        };
        let Ok(v) = serde_json::from_slice::<Value>(&bayt) else {
            return false;
        };
        let Some(p) = v.get("paketler").and_then(|p| p.get(&self.kayit.sahip)) else {
            return false;
        };
        p.get("secili")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
            || p.get("kurulu")
                .and_then(Value::as_array)
                .is_some_and(|k| !k.is_empty())
    }
}
