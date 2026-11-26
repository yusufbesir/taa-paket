//! İhbar ayarı (`ihbar.json`, çalışma verisi klasöründe; DEPODA DEĞİL). Bot anahtarı, sohbet kimliği ve e-posta
//! adresleri kodda ya da depoda bulunmaz, kurulumda Yusuf girer. Boş bırakılan kanal kapalıdır.
//! Sahip: Yusuf. yazan: [Claude#ihbar 2026-11-26]

use serde::Deserialize;
use std::fmt;
use std::path::Path;

#[derive(Deserialize)]
struct Ham {
    #[serde(default)]
    telegram_bot_anahtari: String,
    #[serde(default)]
    telegram_sohbet: String,
    #[serde(default)]
    eposta_alici: String,
    #[serde(default)]
    eposta_gonderen: String,
    #[serde(default = "v_kaynak_kapasite")]
    kaynak_kapasite: u32,
    #[serde(default = "v_kaynak_dolum")]
    kaynak_dolum_sn: u64,
    #[serde(default = "v_genel_kapasite")]
    genel_kapasite: u32,
    #[serde(default = "v_genel_dolum")]
    genel_dolum_sn: u64,
}

// Varsayılanlar tasarım kararıdır, ölçüm değil; pilotta gözden geçirilir.
fn v_kaynak_kapasite() -> u32 {
    3
}
fn v_kaynak_dolum() -> u64 {
    3600
}
fn v_genel_kapasite() -> u32 {
    30
}
fn v_genel_dolum() -> u64 {
    60
}

pub struct Ayar {
    bot: String,
    sohbet: String,
    alici: String,
    gonderen: String,
    pub kaynak_kapasite: u32,
    pub kaynak_dolum_sn: u64,
    pub genel_kapasite: u32,
    pub genel_dolum_sn: u64,
}

/// Sır sızmasın: `Debug` değerleri göstermez.
impl fmt::Debug for Ayar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ayar")
            .field("telegram", &self.telegram_acik())
            .field("eposta", &self.eposta_acik())
            .field("kaynak_kapasite", &self.kaynak_kapasite)
            .field("genel_kapasite", &self.genel_kapasite)
            .finish()
    }
}

fn eposta_gecerli(a: &str) -> bool {
    let Some((y, d)) = a.split_once('@') else {
        return false;
    };
    !y.is_empty()
        && d.contains('.')
        && !d.starts_with('.')
        && !d.ends_with('.')
        && a.matches('@').count() == 1
        && !a
            .chars()
            .any(|c| c.is_whitespace() || c.is_control() || c == '<' || c == '>' || c == ',')
}

impl Ayar {
    pub fn dizinden(dizin: &Path) -> Result<Ayar, String> {
        let s = std::fs::read_to_string(dizin.join("ihbar.json"))
            .map_err(|_| "ihbar.json yok (ihbar hattı ayarlanmamış)".to_string())?;
        Ayar::metinden(&s)
    }

    pub fn metinden(json: &str) -> Result<Ayar, String> {
        // Hata metni değer içermesin (anahtar sızmasın): serde hatasının yalnız konumu söylenir.
        let h: Ham = serde_json::from_str(json).map_err(|e| {
            format!(
                "ihbar.json geçersiz (satır {}, sütun {})",
                e.line(),
                e.column()
            )
        })?;
        let bot = h.telegram_bot_anahtari.trim().to_string();
        let sohbet = h.telegram_sohbet.trim().to_string();
        let alici = h.eposta_alici.trim().to_string();
        let gonderen = h.eposta_gonderen.trim().to_string();
        if bot.is_empty() != sohbet.is_empty() {
            return Err(
                "ihbar.json: telegram_bot_anahtari ve telegram_sohbet birlikte verilmeli".into(),
            );
        }
        if sohbet.chars().any(|c| c.is_whitespace() || c.is_control()) {
            return Err("ihbar.json: telegram_sohbet boşluk içeremez".into());
        }
        if !alici.is_empty() && !eposta_gecerli(&alici) {
            return Err("ihbar.json: eposta_alici geçerli bir adres değil".into());
        }
        if !gonderen.is_empty() && !eposta_gecerli(&gonderen) {
            return Err("ihbar.json: eposta_gonderen geçerli bir adres değil".into());
        }
        if !alici.is_empty() && gonderen.is_empty() {
            return Err("ihbar.json: eposta_alici varsa eposta_gonderen de verilmeli".into());
        }
        if bot.is_empty() && alici.is_empty() {
            return Err("ihbar.json: en az bir kanal (Telegram ya da e-posta) tanımlanmalı".into());
        }
        if h.kaynak_kapasite == 0
            || h.genel_kapasite == 0
            || h.kaynak_dolum_sn == 0
            || h.genel_dolum_sn == 0
        {
            return Err("ihbar.json: hız sınırı değerleri sıfır olamaz".into());
        }
        Ok(Ayar {
            bot,
            sohbet,
            alici,
            gonderen,
            kaynak_kapasite: h.kaynak_kapasite,
            kaynak_dolum_sn: h.kaynak_dolum_sn,
            genel_kapasite: h.genel_kapasite,
            genel_dolum_sn: h.genel_dolum_sn,
        })
    }

    pub fn telegram_acik(&self) -> bool {
        !self.bot.is_empty() && !self.sohbet.is_empty()
    }
    pub fn eposta_acik(&self) -> bool {
        !self.alici.is_empty()
    }
    /// Yalnız gerçek göndericiyi kuran katman çağırmalı; günlüğe ya da hata metnine yazılmaz.
    pub fn bot_anahtari(&self) -> &str {
        &self.bot
    }
    pub fn telegram_sohbet(&self) -> &str {
        &self.sohbet
    }
    pub fn eposta_alici(&self) -> &str {
        &self.alici
    }
    pub fn eposta_gonderen(&self) -> &str {
        &self.gonderen
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn tam_ayar_ve_debug_sir_vermez() {
        let a = Ayar::metinden(
            r#"{"telegram_bot_anahtari":"SIR-ANAHTAR","telegram_sohbet":"12345",
                "eposta_alici":"a@ornek.test","eposta_gonderen":"b@ornek.test"}"#,
        )
        .unwrap();
        assert!(a.telegram_acik() && a.eposta_acik());
        assert_eq!(a.kaynak_kapasite, 3);
        let d = format!("{a:?}");
        assert!(!d.contains("SIR-ANAHTAR") && !d.contains("12345") && !d.contains("ornek.test"));
    }

    #[test]
    fn tek_kanal_yeter_hic_kanal_olmaz() {
        let a = Ayar::metinden(r#"{"telegram_bot_anahtari":"k","telegram_sohbet":"1"}"#).unwrap();
        assert!(a.telegram_acik() && !a.eposta_acik());
        assert!(Ayar::metinden("{}").is_err());
        assert!(Ayar::metinden(r#"{"telegram_bot_anahtari":"k"}"#).is_err());
    }

    #[test]
    fn eposta_dogrulama_ve_hata_deger_icermez() {
        let kotu = r#"{"eposta_alici":"a@b.c\nBcc: x@y.z","eposta_gonderen":"g@h.i"}"#;
        assert!(Ayar::metinden(kotu).is_err());
        assert!(Ayar::metinden(r#"{"eposta_alici":"a@b.test"}"#).is_err());
        let e = Ayar::metinden(r#"{"telegram_bot_anahtari":"GIZLI-DEGER", bozuk"#).unwrap_err();
        assert!(!e.contains("GIZLI-DEGER"));
    }

    #[test]
    fn dosya_yoksa_hata() {
        let d = taa_dosya::gecici_klasor().unwrap();
        assert!(Ayar::dizinden(&d).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
