//! İhbar kaydı. Varsayılan ANONİM: kayıtta ihbar edenin kimliği, cihazı, Telegram numarası ya da adresi için
//! alan YOKTUR; yalnız kategori, metin, gün (saat yok: saatten kişi çıkarılmasın) ve isteğe bağlı iletişim vardır.
//! Sahip: Yusuf. yazan: [Claude#ihbar 2026-11-26]

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const METIN_EN_COK: usize = 4000;
pub const ILETISIM_EN_COK: usize = 200;
/// Takip numarası türetiminde çağıranın verdiği tohumun en az uzunluğu (bayt).
pub const TOHUM_EN_AZ: usize = 16;

/// Kategori listesi bir tasarım önerisidir (planda kategori yazmıyor); Yusuf onayına bağlıdır.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kategori {
    Hirsizlik,
    Usulsuzluk,
    Taciz,
    Guvenlik,
    Diger,
}

impl Kategori {
    pub fn etiket(self) -> &'static str {
        match self {
            Kategori::Hirsizlik => "Hırsızlık",
            Kategori::Usulsuzluk => "Usulsüzlük",
            Kategori::Taciz => "Taciz",
            Kategori::Guvenlik => "İş güvenliği",
            Kategori::Diger => "Diğer",
        }
    }

    /// Bot menüsünden gelen kodu çözer (`hirsizlik`, `taciz` ...).
    pub fn koddan(k: &str) -> Option<Kategori> {
        match k.trim().to_lowercase().as_str() {
            "hirsizlik" => Some(Kategori::Hirsizlik),
            "usulsuzluk" => Some(Kategori::Usulsuzluk),
            "taciz" => Some(Kategori::Taciz),
            "guvenlik" => Some(Kategori::Guvenlik),
            "diger" => Some(Kategori::Diger),
            _ => None,
        }
    }
}

/// Dışarıdan gelen ham ihbar. Kaynak (kim/hangi cihaz) bilerek burada YOK; hız sınırına ayrıca verilir, saklanmaz.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Girdi {
    pub kategori: Kategori,
    pub metin: String,
    pub iletisim: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ihbar {
    /// Rastgele takip numarası (`IH-` + 10 onaltılık hane). Kimlikle ilişkisi yoktur.
    pub takip_no: String,
    /// Yalnız gün (YYYY-MM-DD).
    pub tarih: String,
    pub kategori: Kategori,
    pub metin: String,
    /// İhbar edenin kendi isteğiyle bıraktığı iletişim; yoksa anonim.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub iletisim: Option<String>,
}

fn kontrol_var(s: &str) -> bool {
    s.chars().any(|c| c.is_control() && c != '\n' && c != '\t')
}

pub(crate) fn tarih_gecerli(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10
        && b.iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

/// Takip numarasını tohumdan türetir. `tohum` her ihbar için yeni, rastgele olmalıdır (çağıran üretir; bu crate
/// rastgelelik kaynağı taşımaz). `deneme` çakışmada artırılır.
pub fn takip_no_uret(tohum: &[u8], tarih: &str, deneme: u64) -> Result<String, String> {
    if tohum.len() < TOHUM_EN_AZ {
        return Err(format!("tohum en az {TOHUM_EN_AZ} bayt olmalı"));
    }
    let mut h = Sha256::new();
    h.update(tohum);
    h.update(tarih.as_bytes());
    h.update(deneme.to_le_bytes());
    let d = h.finalize();
    let hane: String = d[..5].iter().map(|b| format!("{b:02X}")).collect();
    Ok(format!("IH-{hane}"))
}

impl Ihbar {
    /// Girdiyi doğrular ve kayda çevirir (metin ≤ 4000 karakter, boş olamaz, denetim karakteri yok).
    pub fn olustur(g: Girdi, tarih: &str, takip_no: String) -> Result<Ihbar, String> {
        if !tarih_gecerli(tarih) {
            return Err("tarih YYYY-MM-DD olmalı".into());
        }
        let metin = g.metin.replace("\r\n", "\n").trim().to_string();
        if metin.is_empty() {
            return Err("ihbar metni boş olamaz".into());
        }
        if metin.chars().count() > METIN_EN_COK {
            return Err(format!(
                "ihbar metni {METIN_EN_COK} karakterden uzun olamaz"
            ));
        }
        if kontrol_var(&metin) {
            return Err("ihbar metninde geçersiz denetim karakteri var".into());
        }
        let iletisim = match g.iletisim {
            None => None,
            Some(i) => {
                let i = i.trim().to_string();
                if i.is_empty() {
                    None
                } else if i.chars().count() > ILETISIM_EN_COK {
                    return Err(format!(
                        "iletişim {ILETISIM_EN_COK} karakterden uzun olamaz"
                    ));
                } else if i.chars().any(char::is_control) {
                    return Err("iletişim tek satır olmalı".into());
                } else {
                    Some(i)
                }
            }
        };
        Ok(Ihbar {
            takip_no,
            tarih: tarih.into(),
            kategori: g.kategori,
            metin,
            iletisim,
        })
    }

    pub fn anonim_mi(&self) -> bool {
        self.iletisim.is_none()
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn girdi(m: &str) -> Girdi {
        Girdi {
            kategori: Kategori::Diger,
            metin: m.into(),
            iletisim: None,
        }
    }

    #[test]
    fn varsayilan_anonim_ve_kimlik_alani_yok() {
        let i = Ihbar::olustur(
            girdi("kasada fark var"),
            "2026-11-26",
            "IH-0000000001".into(),
        )
        .unwrap();
        assert!(i.anonim_mi());
        let v: serde_json::Value = serde_json::to_value(&i).unwrap();
        let mut anahtarlar: Vec<_> = v.as_object().unwrap().keys().cloned().collect();
        anahtarlar.sort();
        assert_eq!(anahtarlar, vec!["kategori", "metin", "takip_no", "tarih"]);
    }

    #[test]
    fn metin_siniri() {
        let t = "ş".repeat(METIN_EN_COK);
        assert!(Ihbar::olustur(girdi(&t), "2026-11-26", "x".into()).is_ok());
        let t = "ş".repeat(METIN_EN_COK + 1);
        assert!(Ihbar::olustur(girdi(&t), "2026-11-26", "x".into()).is_err());
        assert!(Ihbar::olustur(girdi("   \n "), "2026-11-26", "x".into()).is_err());
        assert!(Ihbar::olustur(girdi("a\u{0}b"), "2026-11-26", "x".into()).is_err());
    }

    #[test]
    fn iletisim_istege_bagli() {
        let mut g = girdi("metin");
        g.iletisim = Some("  ".into());
        assert!(Ihbar::olustur(g.clone(), "2026-11-26", "x".into())
            .unwrap()
            .anonim_mi());
        g.iletisim = Some("beni arayın".into());
        let i = Ihbar::olustur(g.clone(), "2026-11-26", "x".into()).unwrap();
        assert_eq!(i.iletisim.as_deref(), Some("beni arayın"));
        g.iletisim = Some("a\nb".into());
        assert!(Ihbar::olustur(g, "2026-11-26", "x".into()).is_err());
    }

    #[test]
    fn tarih_ve_takip_no() {
        assert!(Ihbar::olustur(girdi("m"), "2026-11-26 10:00", "x".into()).is_err());
        let t = [7u8; 16];
        let a = takip_no_uret(&t, "2026-11-26", 0).unwrap();
        assert!(a.starts_with("IH-") && a.len() == 13);
        assert_ne!(a, takip_no_uret(&t, "2026-11-26", 1).unwrap());
        assert!(takip_no_uret(&[1, 2, 3], "2026-11-26", 0).is_err());
    }

    #[test]
    fn kategori_kodu() {
        assert_eq!(Kategori::koddan(" Taciz "), Some(Kategori::Taciz));
        assert_eq!(Kategori::koddan("yok"), None);
    }
}
