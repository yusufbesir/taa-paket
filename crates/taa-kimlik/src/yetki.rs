// Sahip: Yusuf. yazan: [Claude#taa-kimlik-S3a 2026-11-06]
//! Yetki belgesi: sunucunun imzaladığı `{kullanici, cihaz?, sube_id, roller, izinler, programlar, gecerlilik, imza}`
//! (API-SOZLESMESI "Cihaz kimliği"; yol haritası §2 K1–K13). Girişten sonra `POST /v1/yetki {}` ile alınır ve
//! `<kök>/kimlik/yetki-<sunucu>.json` olarak saklanır: internet yokken program 7 gün bu belgeyle açılır.
//! Doğrulama: sunucu genel anahtarı `<kök>/anahtarlar/sunucu.pub` (taa-imza `ed25519:<hex>`); anahtar varsa imzasız
//! ya da yanlış imzalı belge reddedilir, yoksa belge "doğrulanmadı" uyarısıyla kabul edilir (sunucu kurulana kadar).
//! Varsayılan: reddet (K1) — belge yok/süresi geçmiş/bozuk → yetki yok.

use crate::{roller_coz, sunucu_anahtari, KimlikHata};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use taa_imza::GenelAnahtar;
use taa_kuyruk::{GonderHata, Gonderici};

pub const YETKI_UCU: &str = "/v1/yetki";
pub const SUNUCU_ANAHTAR_DOSYASI: &str = "sunucu.pub";
pub const ANAHTAR_KLASORU: &str = "anahtarlar";
pub const YETKI_KLASORU: &str = "kimlik";
/// Belgede `gecerlilik` yoksa alınma anından itibaren bu kadar gün geçerli sayılır.
pub const VARSAYILAN_GUN: i64 = 7;

/// Çözülmüş yetki belgesi (imza doğrulanmış ya da anahtar yokken "dogrulandi=false").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yetki {
    pub kullanici: String,
    pub sube_id: Value,
    pub roller: Vec<String>,
    pub izinler: Vec<String>,
    pub programlar: Vec<String>,
    /// ISO `YYYY-AA-GGTSS:DD:SSZ`.
    pub gecerlilik: String,
    pub alinma: String,
    pub dogrulandi: bool,
}

impl Yetki {
    /// `yonetici` rolü her izni taşır (K4 hiyerarşi); sunucu yine her istekte denetler (K6).
    pub fn izinli(&self, izin: &str) -> bool {
        self.roller.iter().any(|r| r == "yonetici") || self.izinler.iter().any(|i| i == izin)
    }

    /// Liste boşsa (eski sunucu) açık sayılır.
    pub fn program_acik(&self, program: &str) -> bool {
        self.programlar.is_empty() || self.programlar.iter().any(|p| p == program)
    }

    pub fn ozet(&self) -> Value {
        json!({
            "kullanici": self.kullanici, "sube_id": self.sube_id, "roller": self.roller,
            "izinler": self.izinler, "programlar": self.programlar,
            "gecerlilik": self.gecerlilik, "alinma": self.alinma, "dogrulandi": self.dogrulandi,
        })
    }

    fn belgeden(v: &Value, dogrulandi: bool) -> Result<Yetki, KimlikHata> {
        let kullanici = v
            .get("kullanici")
            .and_then(Value::as_str)
            .filter(|k| !k.is_empty())
            .ok_or_else(|| KimlikHata::BozukYanit("yetki belgesinde 'kullanici' yok".into()))?;
        let alinma = v
            .get("alinma")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(taa_kuyruk::zaman::simdi);
        let gecerlilik = match v.get("gecerlilik").and_then(Value::as_str) {
            Some(g) if iso_gecerli(g) => g.to_string(),
            Some(g) => {
                return Err(KimlikHata::BozukYanit(format!(
                    "gecerlilik ISO (YYYY-AA-GGTSS:DD:SSZ) olmalı: {g}"
                )))
            }
            None => gun_ekle(&alinma, VARSAYILAN_GUN),
        };
        Ok(Yetki {
            kullanici: kullanici.to_string(),
            sube_id: v.get("sube_id").cloned().unwrap_or(Value::Null),
            roller: roller_coz(v.get("roller")),
            izinler: roller_coz(v.get("izinler")),
            programlar: roller_coz(v.get("programlar")),
            gecerlilik,
            alinma,
            dogrulandi,
        })
    }
}

fn iso_gecerli(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 20
        && b[4] == b'-'
        && b[7] == b'-'
        && b[10] == b'T'
        && b[13] == b':'
        && b[16] == b':'
        && b[19] == b'Z'
        && b.iter()
            .enumerate()
            .all(|(i, c)| matches!(i, 4 | 7 | 10 | 13 | 16 | 19) || c.is_ascii_digit())
}

/// ISO zamana gün ekler (yalnız biçim korunur; `taa_kuyruk::zaman` ile).
pub fn gun_ekle(iso: &str, gun: i64) -> String {
    let sn = taa_kuyruk::zaman::coz(iso).unwrap_or_else(|| {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0)
    });
    taa_kuyruk::zaman::bicimle(sn + gun * 86_400)
}

/// `<kök>/anahtarlar/sunucu.pub` varsa okur; yoksa None; bozuksa hata.
pub fn sunucu_anahtari_oku(kok: &Path) -> Result<Option<GenelAnahtar>, KimlikHata> {
    let yol = kok.join(ANAHTAR_KLASORU).join(SUNUCU_ANAHTAR_DOSYASI);
    if !yol.is_file() {
        return Ok(None);
    }
    GenelAnahtar::dosyadan(&yol)
        .map(Some)
        .map_err(|e| KimlikHata::Depo(format!("sunucu genel anahtarı: {e}")))
}

/// Belgedeki `imza` alanını sunucu anahtarıyla doğrular (kanonik bayt; `imza` hariç, anahtarlar sıralı).
pub fn belge_dogrula(v: &Value, anahtar: &GenelAnahtar) -> Result<(), KimlikHata> {
    let imza = taa_imza::imza_oku(v)
        .map_err(|e| KimlikHata::BozukYanit(format!("sunucu yanıtı imzasız: {e}")))?;
    anahtar
        .dogrula(&taa_imza::kanonik_bayt(v), &imza)
        .map_err(|e| KimlikHata::BozukYanit(format!("sunucu yanıtının imzası doğrulanamadı: {e}")))
}

pub fn yetki_dosyasi(kok: &Path, sunucu: &str) -> PathBuf {
    kok.join(YETKI_KLASORU)
        .join(format!("yetki-{}.json", sunucu_anahtari(sunucu)))
}

/// `POST /v1/yetki {}` (oturumlu, cihaz imzalı gönderici) → belge doğrulanır (anahtar varsa) ve dosyaya yazılır.
pub fn yetki_al(
    g: &dyn Gonderici,
    kok: &Path,
    sunucu: &str,
    anahtar: Option<&GenelAnahtar>,
) -> Result<Yetki, KimlikHata> {
    let mut v = match g.gonder(YETKI_UCU, &json!({})) {
        Ok(v) => v,
        Err(GonderHata::Reddedildi { kod, mesaj }) => {
            return Err(KimlikHata::GirisReddi { kod, mesaj })
        }
        Err(e) => return Err(KimlikHata::Baglanti(e.to_string())),
    };
    if v.get("oturum").is_some() {
        return Err(KimlikHata::BozukYanit(
            "yetki belgesi 'oturum' içermemeli (diske yazılır)".into(),
        ));
    }
    if let Some(a) = anahtar {
        belge_dogrula(&v, a)?;
    }
    // `alinma` imzalanan belgenin dışında tutulur: sarmal {"belge": …, "alinma": …}.
    let alinma = taa_kuyruk::zaman::simdi();
    let yetki = Yetki::belgeden(&v, anahtar.is_some())?;
    if let Some(o) = v.as_object_mut() {
        o.remove("parola");
    }
    let sarmal = json!({"belge": v, "alinma": alinma, "sunucu": sunucu});
    let yol = yetki_dosyasi(kok, sunucu);
    if let Some(k) = yol.parent() {
        std::fs::create_dir_all(k).map_err(|e| KimlikHata::Depo(e.to_string()))?;
    }
    taa_dosya::uzerine_yaz_atomik(&yol, sarmal.to_string().as_bytes())
        .map_err(|e| KimlikHata::Depo(format!("{} yazılamadı: {e}", yol.display())))?;
    Ok(Yetki {
        alinma: yetki.alinma,
        ..yetki
    })
}

/// Saklı yetki belgesini okur: dosya yok/bozuk/süresi geçmiş/imza tutmuyor → `OturumYok` (varsayılan: reddet).
/// `simdi` ISO; None ise şu an.
pub fn yetki_oku(
    kok: &Path,
    sunucu: &str,
    anahtar: Option<&GenelAnahtar>,
    simdi: Option<&str>,
) -> Result<Yetki, KimlikHata> {
    let yol = yetki_dosyasi(kok, sunucu);
    let metin = std::fs::read_to_string(&yol).map_err(|_| {
        KimlikHata::YetkiYok(format!(
            "{} yok. Öneri: taa-kimlik giris --sunucu {sunucu}",
            yol.display()
        ))
    })?;
    let sarmal: Value = serde_json::from_str(&metin)
        .map_err(|e| KimlikHata::YetkiYok(format!("yetki dosyası bozuk: {e}")))?;
    let belge = sarmal
        .get("belge")
        .ok_or_else(|| KimlikHata::YetkiYok("yetki dosyasında 'belge' yok".into()))?;
    if let Some(a) = anahtar {
        belge_dogrula(belge, a).map_err(|e| KimlikHata::YetkiYok(e.to_string()))?;
    }
    let mut v = belge.clone();
    if let Some(al) = sarmal.get("alinma").cloned() {
        v["alinma"] = al;
    }
    let y =
        Yetki::belgeden(&v, anahtar.is_some()).map_err(|e| KimlikHata::YetkiYok(e.to_string()))?;
    let simdi = simdi
        .map(str::to_string)
        .unwrap_or_else(taa_kuyruk::zaman::simdi);
    if y.gecerlilik.as_str() <= simdi.as_str() {
        return Err(KimlikHata::YetkiYok(format!(
            "yetki belgesinin süresi geçti ({}). Öneri: internet varken taa-kimlik giris",
            y.gecerlilik
        )));
    }
    Ok(y)
}
