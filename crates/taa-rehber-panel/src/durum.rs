//! Panelin tek süreçlik durumu ve dış dünyaya bağlanan arayüzler (`Ortam`: saat/rastgelelik, `Kimlik`: yönetici girişi).
//! İşleyiciler bu arayüzler üzerinden çalışır; böylece saf işlev olarak test edilir.
//! Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]

use crate::belge::BelgeDeposu;
use crate::demo;
use crate::gorunum::{self, Tema};
use std::collections::BTreeSet;
use std::path::PathBuf;
use taa_ihbar::{Ayar, HizSiniri};
use taa_rehber_model::model::Depo;

/// Yönetici oturumu bu kadar saniye etkinlik görmezse düşer (taa-cati paneliyle aynı 15 dk).
pub const YONETICI_BOSTA_SN: u64 = 15 * 60;
/// Yönetici işlemini açan izin (docs/rehber-sozlesmesi.md §5).
pub const IZIN_YAZ: &str = "rehber.yaz";
pub const IZIN_TUM_OKU: &str = "rehber.tum.oku";
pub const IZIN_AMIR_OKU: &str = "rehber.amir.oku";

/// Saat ve rastgelelik kaynağı.
pub trait Ortam {
    /// Unix saniyesi (UTC).
    fn simdi(&self) -> u64;
    fn rastgele(&self, b: &mut [u8]) -> Result<(), String>;
}

pub struct SistemOrtami;

impl Ortam for SistemOrtami {
    fn simdi(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }
    fn rastgele(&self, b: &mut [u8]) -> Result<(), String> {
        getrandom::fill(b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))
    }
}

/// Sunucunun doğruladığı yönetici kimliği. Parola ya da oturum anahtarı burada TUTULMAZ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KimlikBilgisi {
    pub kullanici: String,
    pub izinler: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KimlikRed {
    /// Girdi geçersiz (boş ad vb.).
    Gecersiz(String),
    /// Sunucu girişi reddetti.
    Reddedildi(String),
    /// Kimlik doğrulanamadı (sunucuya ulaşılamadı, cihaz anahtarı yok, doğrulayıcı yok...). Varsayılan: reddet.
    Dogrulanamadi(String),
}

/// Merkezi yönetici girişi. Gerçek gerçekleme `taa-kimlik` ile ikilide; sahte/kodlanmış giriş YOKTUR.
pub trait Kimlik {
    fn giris(&self, kullanici: &str, parola: &str) -> Result<KimlikBilgisi, KimlikRed>;
}

/// Doğrulayıcı yokken kullanılır: her giriş reddedilir (varsayılan REDDET).
pub struct KimlikYok;

impl Kimlik for KimlikYok {
    fn giris(&self, _: &str, _: &str) -> Result<KimlikBilgisi, KimlikRed> {
        Err(KimlikRed::Dogrulanamadi(
            "kimlik doğrulayıcı yok; yönetici işlemleri kapalı".into(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct YoneticiOturum {
    pub kullanici: String,
    pub izinler: Vec<String>,
    pub son_etkinlik: u64,
}

impl YoneticiOturum {
    pub fn izinli(&self, izin: &str) -> bool {
        self.izinler.iter().any(|i| i == izin)
    }
}

pub struct IhbarDurumu {
    pub defter_yolu: PathBuf,
    /// `<kök>/ihbar/ihbar.json`; yoksa ya da geçersizse None (kayıt yine deftere yazılır, iletim kanalı yoktur).
    pub ayar: Option<Ayar>,
    pub hiz: HizSiniri,
}

pub struct Yapilandirma {
    pub kok: PathBuf,
    /// Cihazın şubesi (`--sube`); yoksa kapsam yok: yalnız merkez.
    pub sube_kodu: Option<String>,
    pub demo: bool,
    pub kimlik: Box<dyn Kimlik>,
}

pub struct Durum {
    pub depo: Depo,
    pub belgeler: BelgeDeposu,
    pub sube_kodu: Option<String>,
    pub mudur_rolleri: BTreeSet<String>,
    pub demo: bool,
    pub kimlik: Box<dyn Kimlik>,
    pub yonetici: Option<YoneticiOturum>,
    pub ihbar: IhbarDurumu,
    pub gorunum_yolu: PathBuf,
    pub tema: Tema,
    /// Yönetim işlemlerinin denetim izi (sözleşme §4): kim, ne zaman, hangi tür/işlem, hangi `id`; e-posta yok.
    pub denetim: Vec<String>,
}

impl Durum {
    pub fn yeni(y: Yapilandirma, o: &dyn Ortam) -> Result<Durum, String> {
        let zaman = crate::zaman::zaman(o.simdi());
        let (depo, mudur_rolleri, belgeler, sube_kodu) = if y.demo {
            let (d, m) = demo::ornek_depo(&zaman);
            (
                d,
                m,
                demo::ornek_belgeler(&zaman),
                y.sube_kodu.or_else(|| Some(demo::SUBE_1.to_string())),
            )
        } else {
            (
                Depo::yeni(),
                BTreeSet::new(),
                BelgeDeposu::yeni(),
                y.sube_kodu,
            )
        };
        let ihbar_dizin = y.kok.join("ihbar");
        let ayar = Ayar::dizinden(&ihbar_dizin).ok();
        let mut tuz = [0u8; 32];
        o.rastgele(&mut tuz)?;
        let hiz = match &ayar {
            Some(a) => HizSiniri::ayardan(a, &tuz, o.simdi())?,
            // Ayar yokken: ayardaki varsayılanlarla aynı tasarım değerleri (kaynak başına 3/saat, genel 30/dk).
            None => HizSiniri::yeni(&tuz, 3, 3600, 30, 60, o.simdi())?,
        };
        let gorunum_yolu = gorunum::gorunum_yolu(&y.kok);
        let tema = gorunum::yukle(&gorunum_yolu);
        Ok(Durum {
            depo,
            belgeler,
            sube_kodu,
            mudur_rolleri,
            demo: y.demo,
            kimlik: y.kimlik,
            yonetici: None,
            ihbar: IhbarDurumu {
                defter_yolu: ihbar_dizin.join("ihbar.jsonl"),
                ayar,
                hiz,
            },
            gorunum_yolu,
            tema,
            denetim: Vec::new(),
        })
    }

    /// Boşta kalan yönetici oturumunu düşürür.
    pub fn yonetici_suresi_kontrol(&mut self, simdi: u64) {
        if self
            .yonetici
            .as_ref()
            .is_some_and(|y| simdi.saturating_sub(y.son_etkinlik) > YONETICI_BOSTA_SN)
        {
            self.yonetici = None;
        }
    }

    pub fn yonetici_izinli(&self, izin: &str) -> bool {
        self.yonetici.as_ref().is_some_and(|y| y.izinli(izin))
    }

    pub fn denetim_yaz(&mut self, zaman: &str, tur: &str, islem: &str, id: &str) {
        let k = self
            .yonetici
            .as_ref()
            .map_or("?", |y| y.kullanici.as_str())
            .to_string();
        self.denetim
            .push(format!("{zaman} {k} {tur} {islem} id={id}"));
    }
}

/// Kalıcı kimlik: `<önek>-<12 onaltılık hane>`; sunucu üretir, istemciden alınmaz.
pub fn yeni_kimlik(o: &dyn Ortam, onek: &str) -> Result<String, String> {
    let mut b = [0u8; 6];
    o.rastgele(&mut b)?;
    Ok(format!(
        "{onek}-{}",
        b.iter().map(|x| format!("{x:02x}")).collect::<String>()
    ))
}
