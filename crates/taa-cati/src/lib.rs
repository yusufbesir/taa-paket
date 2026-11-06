// Sahip: Yusuf. yazan: [Claude#taa-cati-S3b 2026-11-06]
//! taa-cati: **TAA Çatı** — yalnız sunucu PC'sinde çalışan yönetim programı (bkz. docs/S3-CATI.md,
//! yol haritası §3). Sunucunun yönetim uçlarını çağırır (API-SOZLESMESI "Yönetim uçları"):
//! kullanıcı/rol, cihaz onayı, program yetkisi, olay günlüğü, şube.
//!
//! Kurallar:
//! - Her istek cihaz imzası + Bearer taşır (`taa_kimlik::Oturum::gonderici_ile`). Yazan uçlar son 15 dk içinde
//!   `POST /v1/yeniden-dogrula {parola}` ister; sunucu 401 `yeniden_dogrula` dönerse `CatiHata::YenidenDogrula`.
//! - Varsayılan reddet (K1): komut, oturumun izin listesinde gerekli izin yoksa sunucuya gitmeden reddedilir; rol adı
//!   tek başına izin vermez (yönetici de listeyle gelir); sunucu yine her istekte denetler (K6).
//! - Silme yok: kullanıcı "kapat", cihaz "iptal"; günlük yalnız okunur (K10).
//! - Parola hiçbir dosyaya/çıktıya yazılmaz; yalnız istek gövdesinde sunucuya gider.
//! - Yeni dış crate yok.

#[cfg(test)]
mod testler;

pub use taa_kimlik::{KimlikHata, Oturum};
pub use taa_kuyruk::{GonderHata, Gonderici};

use serde_json::{json, Map, Value};
use std::fmt;
use std::path::Path;

pub const OZELLIK: &str = "cati";
/// Bu programın adı (program yetkisi tablosunda ve sunucu `programlar` listesinde).
pub const PROGRAM: &str = "taa-cati";

pub const KULLANICI_UCU: &str = "/v1/kullanici";
pub const ROL_UCU: &str = "/v1/rol";
pub const IZIN_UCU: &str = "/v1/izin";
pub const CIHAZ_UCU: &str = "/v1/cihaz";
pub const PROGRAM_YETKI_UCU: &str = "/v1/program-yetki";
pub const OLAY_UCU: &str = "/v1/olay";
pub const SUBE_UCU: &str = "/v1/sube";
pub const YENIDEN_DOGRULA_UCU: &str = "/v1/yeniden-dogrula";

/// İzin adları (sunucu tohumu, API-SOZLESMESI).
pub const IZIN_KULLANICI: &str = "kullanici.yonet";
pub const IZIN_CIHAZ: &str = "cihaz.yonet";
pub const IZIN_PROGRAM: &str = "program.yonet";
pub const IZIN_GUNLUK: &str = "gunluk.gor";
/// Şube ekleme/listeleme kullanıcı yönetimiyle aynı izni ister (sunucu tablosunda ayrı izin yok).
pub const IZIN_SUBE: &str = "kullanici.yonet";

/// Günlük sorgusunda en çok kayıt (sunucu sınırı).
pub const OLAY_EN_COK: u32 = 500;
pub const AD_EN_COK: usize = 64;
pub const PAROLA_EN_AZ: usize = 8;

#[derive(Debug)]
pub enum CatiHata {
    /// Girdi doğrulaması (ad, rol, kimlik, sayı…).
    GecersizGirdi(String),
    /// Oturumda gerekli izin yok (yerel varsayılan reddet; sunucuya gidilmedi).
    Yetkisiz { izin: String },
    /// Sunucu yazma isteği için parola doğrulaması istedi (401 `yeniden_dogrula`).
    YenidenDogrula,
    /// Sunucuya ulaşılamadı ya da sunucu reddetti.
    Sunucu(GonderHata),
    /// Oturum/depo/cihaz anahtarı (taa-kimlik).
    Kimlik(KimlikHata),
    /// Sunucu yanıtı beklenen biçimde değil.
    BozukYanit(String),
}

impl fmt::Display for CatiHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CatiHata::GecersizGirdi(m) => write!(f, "geçersiz girdi: {m}"),
            CatiHata::Yetkisiz { izin } => write!(
                f,
                "bu oturumun '{izin}' izni yok (varsayılan reddet). Öneri: yönetici hesabıyla taa-kimlik giris"
            ),
            CatiHata::YenidenDogrula => write!(
                f,
                "sunucu parola doğrulaması istiyor (yazma işlemleri son 15 dk içinde doğrulama ister). Öneri: taa-cati dogrula"
            ),
            CatiHata::Sunucu(e) => write!(f, "{e}"),
            CatiHata::Kimlik(e) => write!(f, "{e}"),
            CatiHata::BozukYanit(m) => write!(f, "sunucu yanıtı beklenen biçimde değil: {m}"),
        }
    }
}

impl std::error::Error for CatiHata {}

impl From<KimlikHata> for CatiHata {
    fn from(e: KimlikHata) -> CatiHata {
        CatiHata::Kimlik(e)
    }
}

impl From<GonderHata> for CatiHata {
    fn from(e: GonderHata) -> CatiHata {
        match &e {
            GonderHata::Reddedildi { kod: 401, mesaj } if mesaj.contains("yeniden_dogrula") => {
                CatiHata::YenidenDogrula
            }
            _ => CatiHata::Sunucu(e),
        }
    }
}

/// Çıkış kodu: 0 tamam, 2 girdi, 3 ulaşılamadı, 4 sunucu reddetti, 5 oturum yok, 6 yeniden doğrula, 7 yetkisiz, 1 diğer.
pub fn hata_kodu(e: &CatiHata) -> u8 {
    match e {
        CatiHata::GecersizGirdi(_) => 2,
        CatiHata::Yetkisiz { .. } => 7,
        CatiHata::YenidenDogrula => 6,
        CatiHata::Sunucu(GonderHata::Ulasilamadi(_)) => 3,
        CatiHata::Sunucu(GonderHata::Reddedildi { .. }) => 4,
        CatiHata::Sunucu(GonderHata::Yapilandirma(_)) => 2,
        CatiHata::Kimlik(KimlikHata::OturumYok) | CatiHata::Kimlik(KimlikHata::YetkiYok(_)) => 5,
        CatiHata::Kimlik(KimlikHata::Baglanti(_)) => 3,
        CatiHata::Kimlik(_) | CatiHata::BozukYanit(_) => 1,
    }
}

// ---- girdi doğrulama ----

/// Kullanıcı adı / rol / program / şube kodu gibi kısa tanıtıcılar: `[a-z0-9][a-z0-9_.-]*`, en çok 64.
pub fn tanitici_gecerli(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= AD_EN_COK
        && s.bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && s.bytes().all(|b| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'.' | b'-')
        })
}

/// Cihaz kimliği: genel anahtarın ilk 8 baytı, 16 küçük hex.
pub fn kimlik_gecerli(k: &str) -> bool {
    k.len() == 16 && k.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Görünen ad / şube adı: tek satır, 1..=64 karakter, denetim karakteri yok.
pub fn gorunen_ad_gecerli(s: &str) -> bool {
    let t = s.trim();
    !t.is_empty() && t.chars().count() <= AD_EN_COK && !t.chars().any(char::is_control)
}

/// Parola: en az 8 karakter, satır sonu yok. (Sunucu da argon2 öncesi denetler.)
pub fn parola_gecerli(p: &str) -> bool {
    p.chars().count() >= PAROLA_EN_AZ && !p.chars().any(|c| matches!(c, '\n' | '\r'))
}

fn tanitici(ad: &str, s: &str) -> Result<String, CatiHata> {
    if tanitici_gecerli(s) {
        Ok(s.to_string())
    } else {
        Err(CatiHata::GecersizGirdi(format!(
            "{ad} yalnız küçük harf, rakam, '_', '.', '-' içerir (1..=64): {s:?}"
        )))
    }
}

fn kimlik(k: &str) -> Result<String, CatiHata> {
    if kimlik_gecerli(k) {
        Ok(k.to_string())
    } else {
        Err(CatiHata::GecersizGirdi(format!(
            "cihaz kimliği 16 hex karakter olmalı: {k:?}"
        )))
    }
}

/// Tarih süzgeci: `YYYY-AA-GG` ya da `YYYY-AA-GGTSS:DD:SSZ` biçimi; sorgu dizesine güvenle yazılır.
fn tarih_gecerli(t: &str) -> bool {
    taa_kuyruk::zaman::coz(t).is_some()
        || (t.len() == 10 && taa_kuyruk::zaman::coz(&format!("{t}T00:00:00Z")).is_some())
}

/// Rol ataması: rol adı ve isteğe bağlı şube.
#[derive(Debug, Clone, PartialEq)]
pub struct RolAtama {
    pub rol: String,
    pub sube_id: Option<i64>,
}

impl RolAtama {
    pub fn yeni(rol: &str, sube_id: Option<i64>) -> Result<RolAtama, CatiHata> {
        Ok(RolAtama {
            rol: tanitici("rol", rol)?,
            sube_id,
        })
    }

    fn json(&self) -> Value {
        let mut m = Map::new();
        m.insert("rol".into(), Value::String(self.rol.clone()));
        if let Some(s) = self.sube_id {
            m.insert("sube_id".into(), json!(s));
        }
        Value::Object(m)
    }
}

/// `GET /v1/olay` süzgeci. Hepsi isteğe bağlı; `limit` 1..=500 (varsayılan 100).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OlaySuzgeci {
    pub islem: Option<String>,
    pub kullanici_id: Option<i64>,
    pub sube_id: Option<i64>,
    pub tarih_bas: Option<String>,
    pub tarih_bit: Option<String>,
    pub limit: Option<u32>,
    pub onceki_id: Option<i64>,
}

impl OlaySuzgeci {
    /// Sorgu dizesi (`?a=b&c=d`); süzgeç boşsa yalnız `limit`.
    pub fn sorgu(&self) -> Result<String, CatiHata> {
        let mut p: Vec<String> = Vec::new();
        if let Some(i) = &self.islem {
            p.push(format!("islem={}", tanitici("islem", i)?));
        }
        if let Some(k) = self.kullanici_id {
            p.push(format!("kullanici_id={k}"));
        }
        if let Some(s) = self.sube_id {
            p.push(format!("sube_id={s}"));
        }
        for (ad, t) in [
            ("tarih_bas", &self.tarih_bas),
            ("tarih_bit", &self.tarih_bit),
        ] {
            if let Some(t) = t {
                if !tarih_gecerli(t) {
                    return Err(CatiHata::GecersizGirdi(format!(
                        "{ad} YYYY-AA-GG ya da YYYY-AA-GGTSS:DD:SSZ olmalı: {t:?}"
                    )));
                }
                p.push(format!("{ad}={t}"));
            }
        }
        let limit = self.limit.unwrap_or(100);
        if limit == 0 || limit > OLAY_EN_COK {
            return Err(CatiHata::GecersizGirdi(format!(
                "limit 1..={OLAY_EN_COK} olmalı: {limit}"
            )));
        }
        p.push(format!("limit={limit}"));
        if let Some(o) = self.onceki_id {
            p.push(format!("onceki_id={o}"));
        }
        Ok(format!("?{}", p.join("&")))
    }
}

/// Yönetim istemcisi: bir gönderici (cihaz imzalı + Bearer) ve oturumun izinleri.
pub struct Cati<'a> {
    g: &'a dyn Gonderici,
    izinler: Vec<String>,
    yonetici: bool,
}

impl<'a> Cati<'a> {
    /// Oturumun izin listesiyle kurar (yerel varsayılan reddet, K1): yalnız sunucunun verdiği `izinler` sayılır;
    /// rol adı (`yonetici` dahil) tek başına izin vermez, var olmayan izin adı her zaman reddedilir.
    pub fn yeni(g: &'a dyn Gonderici, izinler: &[String]) -> Cati<'a> {
        Cati {
            g,
            izinler: izinler.to_vec(),
            yonetici: false,
        }
    }

    /// Depodan okunan oturumla kurar.
    pub fn oturumdan(g: &'a dyn Gonderici, oturum: &Oturum) -> Cati<'a> {
        Cati::yeni(g, &oturum.izinler)
    }

    /// Testler / izin denetimi yapılmayan kullanım: her izin var sayılır (sunucu yine denetler).
    pub fn izinsiz_denetimle(g: &'a dyn Gonderici) -> Cati<'a> {
        Cati {
            g,
            izinler: Vec::new(),
            yonetici: true,
        }
    }

    pub fn izinli(&self, izin: &str) -> bool {
        self.yonetici || self.izinler.iter().any(|i| i == izin)
    }

    fn izin(&self, izin: &str) -> Result<(), CatiHata> {
        if self.izinli(izin) {
            Ok(())
        } else {
            Err(CatiHata::Yetkisiz { izin: izin.into() })
        }
    }

    fn liste(&self, izin: &str, uc: &str) -> Result<Vec<Value>, CatiHata> {
        self.izin(izin)?;
        let v = self.g.al(uc)?;
        match v {
            Value::Array(a) => Ok(a),
            Value::Null => Ok(Vec::new()),
            _ => Err(CatiHata::BozukYanit(format!("{uc} dizi döndürmedi"))),
        }
    }

    fn yaz(&self, izin: &str, uc: &str, govde: Value) -> Result<Value, CatiHata> {
        self.izin(izin)?;
        Ok(self.g.gonder(uc, &govde)?)
    }

    // ---- yeniden doğrulama ----

    /// `POST /v1/yeniden-dogrula {parola}`: yazma işlemleri için 15 dk'lık pencere açar. Parola saklanmaz.
    pub fn yeniden_dogrula(&self, parola: &str) -> Result<Value, CatiHata> {
        if parola.is_empty() || parola.chars().any(|c| matches!(c, '\n' | '\r')) {
            return Err(CatiHata::GecersizGirdi(
                "parola boş ya da satır sonu içeriyor".into(),
            ));
        }
        Ok(self
            .g
            .gonder(YENIDEN_DOGRULA_UCU, &json!({ "parola": parola }))?)
    }

    // ---- kullanıcı ----

    pub fn kullanicilar(&self) -> Result<Vec<Value>, CatiHata> {
        self.liste(IZIN_KULLANICI, KULLANICI_UCU)
    }

    /// `POST /v1/kullanici` → 201 {id, ad}. En az bir rol zorunlu (rolsüz kullanıcı hiçbir şey yapamaz; K1).
    pub fn kullanici_ekle(
        &self,
        ad: &str,
        gorunen_ad: &str,
        parola: &str,
        roller: &[RolAtama],
    ) -> Result<Value, CatiHata> {
        let ad = tanitici("kullanıcı adı", ad)?;
        if !gorunen_ad_gecerli(gorunen_ad) {
            return Err(CatiHata::GecersizGirdi(
                "görünen ad boş/çok uzun/denetim karakteri".into(),
            ));
        }
        if !parola_gecerli(parola) {
            return Err(CatiHata::GecersizGirdi(format!(
                "parola en az {PAROLA_EN_AZ} karakter olmalı"
            )));
        }
        if roller.is_empty() {
            return Err(CatiHata::GecersizGirdi(
                "en az bir rol gerekli (--rol)".into(),
            ));
        }
        self.yaz(
            IZIN_KULLANICI,
            KULLANICI_UCU,
            json!({
                "ad": ad,
                "gorunen_ad": gorunen_ad.trim(),
                "parola": parola,
                "roller": roller.iter().map(RolAtama::json).collect::<Vec<_>>(),
            }),
        )
    }

    pub fn rol_ver(&self, kullanici_id: i64, rol: &RolAtama) -> Result<Value, CatiHata> {
        self.yaz(
            IZIN_KULLANICI,
            &format!("{KULLANICI_UCU}/{kullanici_id}/rol"),
            rol.json(),
        )
    }

    pub fn rol_kaldir(&self, kullanici_id: i64, rol: &RolAtama) -> Result<Value, CatiHata> {
        self.yaz(
            IZIN_KULLANICI,
            &format!("{KULLANICI_UCU}/{kullanici_id}/rol-kaldir"),
            rol.json(),
        )
    }

    /// Kullanıcıyı kapatır (silme yok; günlük ve kayıtlar kalır).
    pub fn kullanici_kapat(&self, kullanici_id: i64) -> Result<Value, CatiHata> {
        self.yaz(
            IZIN_KULLANICI,
            &format!("{KULLANICI_UCU}/{kullanici_id}/kapat"),
            json!({}),
        )
    }

    pub fn parola_degistir(&self, kullanici_id: i64, parola: &str) -> Result<Value, CatiHata> {
        if !parola_gecerli(parola) {
            return Err(CatiHata::GecersizGirdi(format!(
                "parola en az {PAROLA_EN_AZ} karakter olmalı"
            )));
        }
        self.yaz(
            IZIN_KULLANICI,
            &format!("{KULLANICI_UCU}/{kullanici_id}/parola"),
            json!({ "parola": parola }),
        )
    }

    // ---- rol / izin (tohumlu, yalnız okunur) ----

    pub fn roller(&self) -> Result<Vec<Value>, CatiHata> {
        self.liste(IZIN_KULLANICI, ROL_UCU)
    }

    pub fn izinler(&self) -> Result<Vec<Value>, CatiHata> {
        self.liste(IZIN_KULLANICI, IZIN_UCU)
    }

    // ---- cihaz ----

    /// `durum`: bekliyor | onayli | iptal | None (hepsi).
    pub fn cihazlar(&self, durum: Option<&str>) -> Result<Vec<Value>, CatiHata> {
        let uc = match durum {
            None => CIHAZ_UCU.to_string(),
            Some(d) if matches!(d, "bekliyor" | "onayli" | "iptal") => {
                format!("{CIHAZ_UCU}?durum={d}")
            }
            Some(d) => {
                return Err(CatiHata::GecersizGirdi(format!(
                    "durum bekliyor|onayli|iptal olmalı: {d:?}"
                )))
            }
        };
        self.liste(IZIN_CIHAZ, &uc)
    }

    /// Cihazı onaylar; `sube_id` None = merkez (sunucu) cihazı.
    pub fn cihaz_onayla(&self, kimlik_: &str, sube_id: Option<i64>) -> Result<Value, CatiHata> {
        let k = kimlik(kimlik_)?;
        let govde = match sube_id {
            Some(s) => json!({ "sube_id": s }),
            None => json!({}),
        };
        self.yaz(IZIN_CIHAZ, &format!("{CIHAZ_UCU}/{k}/onay"), govde)
    }

    pub fn cihaz_iptal(&self, kimlik_: &str) -> Result<Value, CatiHata> {
        let k = kimlik(kimlik_)?;
        self.yaz(IZIN_CIHAZ, &format!("{CIHAZ_UCU}/{k}/iptal"), json!({}))
    }

    // ---- program yetkisi ----

    pub fn program_yetkileri(&self) -> Result<Vec<Value>, CatiHata> {
        self.liste(IZIN_PROGRAM, PROGRAM_YETKI_UCU)
    }

    /// (şube, rol, program) → açık/kapalı. Şube ve rol None = genel kural; `acik:false` reddet kazanır.
    pub fn program_yetki_ayarla(
        &self,
        sube_id: Option<i64>,
        rol: Option<&str>,
        program: &str,
        acik: bool,
    ) -> Result<Value, CatiHata> {
        let mut m = Map::new();
        if let Some(s) = sube_id {
            m.insert("sube_id".into(), json!(s));
        }
        if let Some(r) = rol {
            m.insert("rol".into(), Value::String(tanitici("rol", r)?));
        }
        m.insert(
            "program".into(),
            Value::String(tanitici("program", program)?),
        );
        m.insert("acik".into(), Value::Bool(acik));
        self.yaz(IZIN_PROGRAM, PROGRAM_YETKI_UCU, Value::Object(m))
    }

    // ---- günlük ----

    pub fn olaylar(&self, s: &OlaySuzgeci) -> Result<Vec<Value>, CatiHata> {
        let uc = format!("{OLAY_UCU}{}", s.sorgu()?);
        self.liste(IZIN_GUNLUK, &uc)
    }

    // ---- şube ----

    pub fn subeler(&self) -> Result<Vec<Value>, CatiHata> {
        self.liste(IZIN_SUBE, SUBE_UCU)
    }

    pub fn sube_ekle(&self, kod: &str, ad: &str) -> Result<Value, CatiHata> {
        let kod = tanitici("şube kodu", kod)?;
        if !gorunen_ad_gecerli(ad) {
            return Err(CatiHata::GecersizGirdi(
                "şube adı boş/çok uzun/denetim karakteri".into(),
            ));
        }
        self.yaz(IZIN_SUBE, SUBE_UCU, json!({ "kod": kod, "ad": ad.trim() }))
    }
}

/// Depodaki oturumu okur ve cihaz imzalı göndericiyi kurar. Çatı yalnız sunucu PC'sinde çalışır; cihaz anahtarı
/// yoksa hata (sunucu imzasız isteği reddeder; `taa-cihaz uret` + `tutaa-sunucu merkez-cihaz`).
pub fn yonetici_baglantisi(
    kok: &Path,
    sunucu: &str,
    sertifika: Option<&Path>,
) -> Result<(Oturum, taa_kuyruk::CurlGonderici), CatiHata> {
    let depo = taa_kimlik::varsayilan_depo(kok);
    let o = depo.oku(sunucu)?;
    let cihaz = taa_kimlik::cihaz_imzalayici(kok)?.ok_or_else(|| {
        CatiHata::Kimlik(KimlikHata::Depo(format!(
            "cihaz anahtarı yok ({}). Öneri: taa-cihaz uret, sonra sunucuda tutaa-sunucu merkez-cihaz",
            kok.join("cihaz").display()
        )))
    })?;
    let g = o.gonderici_ile(sertifika, Some(cihaz))?;
    Ok((o, g))
}
