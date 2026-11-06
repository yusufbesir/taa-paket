// Sahip: Yusuf. yazan: [Claude#taa-cihaz-S2 2026-11-06]
//! taa-cihaz: şube PC'sinin **cihaz kimliği**. VPN yok kararı (2026-11-06) sonrası sunucu yalnız kayıtlı ve onaylı
//! cihazdan gelen isteği kabul eder; bu crate kurulumda bir ed25519 anahtar çifti üretir ve her isteği imzalar
//! (API-SOZLESMESI.md "Cihaz kimliği").
//!
//! Dosyalar (`<veri kökü>/cihaz/`): `cihaz.key` (özel, 0600, hiç okunup basılmaz), `cihaz.pub` (`ed25519:<hex>`),
//! `cihaz.json` ({ad, kimlik, olusturma}). Özel anahtar yalnız imzalamada bellekte tutulur.
//!
//! İmza: başlıklar `X-TAA-Cihaz` (kimlik = genel anahtarın ilk 8 baytı), `X-TAA-Zaman` (unix sn),
//! `X-TAA-Imza` (128 hex); imzalanan bayt dizisi `"{YONTEM}\n{YOL}\n{ZAMAN}\n"` + gövde.

use serde_json::{json, Value};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use taa_imza::{GenelAnahtar, OzelAnahtar};
use taa_kuyruk::{GonderHata, Gonderici, Imzalayici};

pub const KLASOR: &str = "cihaz";
pub const OZEL_DOSYA: &str = "cihaz.key";
pub const GENEL_DOSYA: &str = "cihaz.pub";
pub const BILGI_DOSYASI: &str = "cihaz.json";
pub const BASLIK_CIHAZ: &str = "X-TAA-Cihaz";
pub const BASLIK_ZAMAN: &str = "X-TAA-Zaman";
pub const BASLIK_IMZA: &str = "X-TAA-Imza";
pub const KAYIT_UCU: &str = "/v1/cihaz/kayit";
pub const DURUM_UCU: &str = "/v1/cihaz/durum";
/// Sunucunun kabul ettiği saat farkı (saniye); istemci bunu aşan sapmada uyarır.
pub const ZAMAN_PAYI_SN: i64 = 300;
pub const AD_EN_COK: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CihazHata {
    /// Anahtar yok: önce `taa-cihaz uret`.
    AnahtarYok(String),
    /// Dosya/anahtar biçimi hatası.
    Dosya(String),
    /// Girdi doğrulama (ad, şube kodu).
    GecersizGirdi(String),
    /// Sunucu ulaşılamadı / reddetti.
    Sunucu(String),
}

impl fmt::Display for CihazHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CihazHata::AnahtarYok(m) => write!(f, "cihaz anahtarı yok: {m}"),
            CihazHata::Dosya(m) => write!(f, "cihaz dosyası: {m}"),
            CihazHata::GecersizGirdi(m) => write!(f, "geçersiz girdi: {m}"),
            CihazHata::Sunucu(m) => write!(f, "sunucu: {m}"),
        }
    }
}

impl std::error::Error for CihazHata {}

/// Cihaz adı: harf/rakam/`-`/`_`/`.`/boşluk, en çok 64. Boşsa bilgisayar adı.
pub fn ad_gecerli(ad: &str) -> bool {
    let a = ad.trim();
    !a.is_empty()
        && a.len() <= AD_EN_COK
        && a.chars()
            .all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' '))
}

/// Bilgisayar adı (COMPUTERNAME / HOSTNAME), yoksa "pc".
pub fn bilgisayar_adi() -> String {
    ["COMPUTERNAME", "HOSTNAME"]
        .iter()
        .filter_map(|a| std::env::var(a).ok())
        .map(|s| s.trim().to_string())
        .find(|s| ad_gecerli(s))
        .unwrap_or_else(|| "pc".into())
}

/// İmzalanan bayt dizisi (sunucu da aynısını kurar).
pub fn imza_mesaji(yontem: &str, yol: &str, zaman: i64, govde: &[u8]) -> Vec<u8> {
    let mut m = format!("{}\n{yol}\n{zaman}\n", yontem.to_ascii_uppercase()).into_bytes();
    m.extend_from_slice(govde);
    m
}

/// Unix saniyesi.
pub fn simdi_sn() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Sunucu tarafı/test doğrulaması: `imza_hex` verilen anahtar ve mesajla doğru mu, zaman payı içinde mi.
pub fn dogrula(
    genel: &GenelAnahtar,
    yontem: &str,
    yol: &str,
    zaman: i64,
    govde: &[u8],
    imza_hex: &str,
    simdi: i64,
) -> Result<(), CihazHata> {
    if (simdi - zaman).abs() > ZAMAN_PAYI_SN {
        return Err(CihazHata::GecersizGirdi(format!(
            "zaman payı aşıldı ({} sn)",
            (simdi - zaman).abs()
        )));
    }
    let imza = taa_imza::Imza {
        alg: taa_imza::ALGORITMA.into(),
        anahtar_kimlik: genel.kimlik(),
        deger: imza_hex.to_string(),
    };
    genel
        .dogrula(&imza_mesaji(yontem, yol, zaman, govde), &imza)
        .map_err(|e| CihazHata::GecersizGirdi(e.to_string()))
}

/// Yüklü cihaz anahtarı. `Debug` özel anahtarı göstermez.
pub struct CihazAnahtari {
    ozel: OzelAnahtar,
    genel: GenelAnahtar,
    ad: String,
    olusturma: String,
    klasor: PathBuf,
}

impl fmt::Debug for CihazAnahtari {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CihazAnahtari")
            .field("kimlik", &self.kimlik())
            .field("ad", &self.ad)
            .finish()
    }
}

impl CihazAnahtari {
    pub fn klasor(kok: &Path) -> PathBuf {
        kok.join(KLASOR)
    }

    /// Özel anahtar dosyası var mı?
    pub fn var_mi(kok: &Path) -> bool {
        Self::klasor(kok).join(OZEL_DOSYA).is_file()
    }

    /// Yeni anahtar üretir (var olanın üzerine yazmaz). `ad` boşsa bilgisayar adı.
    pub fn uret(kok: &Path, ad: Option<&str>) -> Result<CihazAnahtari, CihazHata> {
        let ad = ad
            .map(str::trim)
            .filter(|a| !a.is_empty())
            .map(str::to_string)
            .unwrap_or_else(bilgisayar_adi);
        if !ad_gecerli(&ad) {
            return Err(CihazHata::GecersizGirdi(format!(
                "cihaz adı harf/rakam/-/_/./boşluk, en çok {AD_EN_COK}: {ad:?}"
            )));
        }
        let klasor = Self::klasor(kok);
        if Self::var_mi(kok) {
            return Err(CihazHata::Dosya(format!(
                "{} zaten var; üzerine yazılmaz. Öneri: mevcut anahtarı kullanın ya da klasörü eski\\ altına taşıyın.",
                klasor.join(OZEL_DOSYA).display()
            )));
        }
        let genel = taa_imza::anahtar_uret(&klasor.join(OZEL_DOSYA), &klasor.join(GENEL_DOSYA))
            .map_err(|e| CihazHata::Dosya(e.to_string()))?;
        let olusturma = taa_kuyruk::zaman::simdi();
        let bilgi = json!({"ad": ad, "kimlik": genel.kimlik(), "olusturma": olusturma});
        taa_dosya::uzerine_yaz_atomik(&klasor.join(BILGI_DOSYASI), bilgi.to_string().as_bytes())
            .map_err(|e| CihazHata::Dosya(format!("{BILGI_DOSYASI} yazılamadı: {e}")))?;
        Self::yukle(kok)
    }

    /// Var olan anahtarı yükler.
    pub fn yukle(kok: &Path) -> Result<CihazAnahtari, CihazHata> {
        let klasor = Self::klasor(kok);
        let ozel_yol = klasor.join(OZEL_DOSYA);
        if !ozel_yol.is_file() {
            return Err(CihazHata::AnahtarYok(format!(
                "{} yok. Öneri: taa-cihaz uret",
                ozel_yol.display()
            )));
        }
        let (ozel, uyari) =
            OzelAnahtar::dosyadan(&ozel_yol).map_err(|e| CihazHata::Dosya(e.to_string()))?;
        if let Some(u) = uyari {
            eprintln!("{u}");
        }
        let genel = ozel.genel();
        // cihaz.pub varsa özel anahtarla uyuşmalı (dosya karışıklığına karşı).
        if let Ok(g) = GenelAnahtar::dosyadan(&klasor.join(GENEL_DOSYA)) {
            if g != genel {
                return Err(CihazHata::Dosya(format!(
                    "{GENEL_DOSYA} ile {OZEL_DOSYA} uyuşmuyor."
                )));
            }
        }
        let bilgi: Value = std::fs::read_to_string(klasor.join(BILGI_DOSYASI))
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or(Value::Null);
        let al = |a: &str| bilgi.get(a).and_then(Value::as_str).map(str::to_string);
        Ok(CihazAnahtari {
            ad: al("ad")
                .filter(|a| ad_gecerli(a))
                .unwrap_or_else(bilgisayar_adi),
            olusturma: al("olusturma").unwrap_or_default(),
            ozel,
            genel,
            klasor,
        })
    }

    pub fn kimlik(&self) -> String {
        self.genel.kimlik()
    }

    pub fn genel(&self) -> &GenelAnahtar {
        &self.genel
    }

    pub fn ad(&self) -> &str {
        &self.ad
    }

    /// Ekrana/JSON'a basılabilir özet (özel anahtar yok).
    pub fn ozet(&self) -> Value {
        json!({
            "kimlik": self.kimlik(),
            "ad": self.ad,
            "acik_anahtar": self.genel.metin(),
            "olusturma": self.olusturma,
            "klasor": self.klasor.display().to_string(),
        })
    }

    /// Verilen istek için imza (hex).
    pub fn imzala(&self, yontem: &str, yol: &str, zaman: i64, govde: &[u8]) -> String {
        self.ozel
            .imzala(&imza_mesaji(yontem, yol, zaman, govde))
            .deger
    }

    /// `POST /v1/cihaz/kayit` gövdesi.
    pub fn kayit_govdesi(&self, sube_kodu: Option<&str>) -> Result<Value, CihazHata> {
        let mut g = json!({"acik_anahtar": self.genel.metin(), "ad": self.ad});
        if let Some(s) = sube_kodu.map(str::trim).filter(|s| !s.is_empty()) {
            if !ad_gecerli(s) {
                return Err(CihazHata::GecersizGirdi(format!("şube kodu: {s:?}")));
            }
            g["sube_kodu"] = Value::String(s.to_string());
        }
        Ok(g)
    }

    /// Bu anahtarla imzalayan curl göndericisi (`taban` sunucu adresi).
    pub fn gonderici(
        self: Arc<Self>,
        taban: &str,
        sertifika: Option<&Path>,
    ) -> Result<taa_kuyruk::CurlGonderici, CihazHata> {
        let mut g =
            taa_kuyruk::CurlGonderici::yeni(taban).map_err(|e| CihazHata::Sunucu(e.to_string()))?;
        if let Some(c) = sertifika {
            g = g.sertifika(c);
        }
        Ok(g.imzalayici(self))
    }
}

impl Imzalayici for CihazAnahtari {
    fn basliklar(
        &self,
        yontem: &str,
        yol: &str,
        govde: &[u8],
    ) -> Result<Vec<(String, String)>, String> {
        let zaman = simdi_sn();
        Ok(vec![
            (BASLIK_CIHAZ.into(), self.kimlik()),
            (BASLIK_ZAMAN.into(), zaman.to_string()),
            (BASLIK_IMZA.into(), self.imzala(yontem, yol, zaman, govde)),
        ])
    }
}

/// Sunucuya cihaz kaydı (imzasız tek uç). Yanıt: {kimlik, durum: "bekliyor"|...}.
/// Aynı anahtar yeniden gönderilirse sunucu aynı kaydı döner (idempotent).
pub fn kayit(
    g: &dyn Gonderici,
    c: &CihazAnahtari,
    sube_kodu: Option<&str>,
) -> Result<Value, CihazHata> {
    let yanit = g
        .gonder(KAYIT_UCU, &c.kayit_govdesi(sube_kodu)?)
        .map_err(sunucu_hatasi)?;
    yanit_kimlik_denetle(&yanit, &c.kimlik())?;
    Ok(yanit)
}

/// Cihaz durumu (imzalı, oturumsuz): {kimlik, durum: bekliyor|onayli|iptal, sube_id?}.
pub fn durum(g: &dyn Gonderici, c: &CihazAnahtari) -> Result<Value, CihazHata> {
    let yanit = g.gonder(DURUM_UCU, &json!({})).map_err(sunucu_hatasi)?;
    yanit_kimlik_denetle(&yanit, &c.kimlik())?;
    Ok(yanit)
}

fn yanit_kimlik_denetle(yanit: &Value, kimlik: &str) -> Result<(), CihazHata> {
    match yanit.get("kimlik").and_then(Value::as_str) {
        Some(k) if k == kimlik => Ok(()),
        Some(k) => Err(CihazHata::Sunucu(format!(
            "sunucu başka bir cihaz kimliği döndü ({k} ≠ {kimlik})"
        ))),
        None => Err(CihazHata::Sunucu("yanıtta 'kimlik' yok".into())),
    }
}

fn sunucu_hatasi(e: GonderHata) -> CihazHata {
    CihazHata::Sunucu(e.to_string())
}

/// `GonderHata` türünü dışarıya verir (çıkış kodu seçimi için): 3 ulaşılamadı, 4 reddedildi.
pub fn hata_kodu(m: &str) -> u8 {
    if m.contains("reddetti") {
        4
    } else if m.contains("ulaşılamadı") {
        3
    } else {
        1
    }
}

#[cfg(test)]
mod testler;
