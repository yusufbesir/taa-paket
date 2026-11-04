//! taa-terzi: tadilat işlemleri kataloğu ve ölçü sınır denetimi.
//! Katalog bir VERİ dosyasıdır (`islemler.json`); mağaza `terzi-kurallari.json` ile değiştirebilir.
//! Önemli: sınır sayıları ustasız kural değildir. `dogrulandi:false` olanlar Yusuf'un genel bilgisidir,
//! usta/kıdemli danışman doğrulayana kadar "uyarı + usta onayı" olarak çalışır.
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use serde::{Deserialize, Serialize};
use std::path::Path;

const VARSAYILAN: &str = include_str!("islemler.json");

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Islem {
    pub kod: String,
    pub grup: String,
    pub ad: String,
    /// Ölçü (cm) zorunlu mu?
    #[serde(default)]
    pub olcu: bool,
    #[serde(default)]
    pub varyantlar: Vec<String>,
    /// Bu ölçüyü (milimetre) aşarsa usta onayı gerekir.
    #[serde(default)]
    pub sinir_mm: Option<i32>,
    #[serde(default)]
    pub uyari: String,
    /// Sınır usta tarafından doğrulandı mı?
    #[serde(default)]
    pub dogrulandi: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Katalog {
    pub islemler: Vec<Islem>,
}

impl Katalog {
    pub fn varsayilan() -> Katalog {
        serde_json::from_str(VARSAYILAN).expect("gömülü işlem kataloğu geçerli olmalı")
    }

    pub fn json_den(s: &str) -> Result<Katalog, String> {
        let k: Katalog =
            serde_json::from_str(s).map_err(|e| format!("kural dosyası geçersiz: {e}"))?;
        let mut kodlar = std::collections::BTreeSet::new();
        for i in &k.islemler {
            if i.kod.trim().is_empty() || !kodlar.insert(i.kod.as_str()) {
                return Err(format!("işlem kodu boş ya da tekrar ediyor: '{}'", i.kod));
            }
            if matches!(i.sinir_mm, Some(s) if s <= 0) {
                return Err(format!("{}: sınır pozitif olmalı", i.kod));
            }
        }
        Ok(k)
    }

    /// `dizin/terzi-kurallari.json` varsa onu, yoksa gömülü kataloğu yükler. Dosya bozuksa hata verir (sessizce yutmaz).
    pub fn dizinden(dizin: &Path) -> Result<Katalog, String> {
        let yol = dizin.join("terzi-kurallari.json");
        match std::fs::read_to_string(&yol) {
            Ok(s) => Self::json_den(&s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::varsayilan()),
            Err(e) => Err(format!("kural dosyası okunamadı: {e}")),
        }
    }

    pub fn bul(&self, kod: &str) -> Option<&Islem> {
        self.islemler.iter().find(|i| i.kod == kod)
    }

    /// Bir kalemi denetler. `Err`: kaydedilemez; `Ok(uyarılar)`: kaydedilebilir, kullanıcıya gösterilir.
    pub fn kontrol(
        &self,
        kod: &str,
        olcu_mm: Option<i32>,
        varyant: Option<&str>,
        usta_onayli: bool,
    ) -> Result<Vec<String>, String> {
        let i = self
            .bul(kod)
            .ok_or_else(|| format!("bilinmeyen işlem: {kod}"))?;
        if !i.varyantlar.is_empty()
            && !varyant
                .map(|v| i.varyantlar.iter().any(|x| x == v))
                .unwrap_or(false)
        {
            return Err(format!(
                "{}: seçenek seçin ({})",
                i.ad,
                i.varyantlar.join(" / ")
            ));
        }
        if i.olcu && !olcu_mm.map(|m| m > 0).unwrap_or(false) {
            return Err(format!("{}: ölçü (cm) yazılmalı", i.ad));
        }
        let mut uyarilar = Vec::new();
        let asti = matches!((i.sinir_mm, olcu_mm), (Some(s), Some(m)) if m > s);
        if asti {
            if !usta_onayli {
                return Err(format!(
                    "{}: {} Usta onayı olmadan kaydedilemez.",
                    i.ad, i.uyari
                ));
            }
            uyarilar.push(format!(
                "{}: sınır aşıldı, usta onayıyla kaydedildi{}",
                i.ad,
                if i.dogrulandi {
                    ""
                } else {
                    " (sınır henüz usta tarafından doğrulanmadı)"
                }
            ));
        } else if i.sinir_mm.is_none() && !i.uyari.is_empty() {
            uyarilar.push(format!("{}: {}", i.ad, i.uyari));
        }
        Ok(uyarilar)
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn gomulu_katalog_gecerli_ve_tam() {
        let k = Katalog::varsayilan();
        assert!(k.islemler.len() >= 25);
        assert_eq!(
            Katalog::json_den(&serde_json::to_string(&k).unwrap()).unwrap(),
            k
        );
        for g in ["Pantolon", "Ceket", "Gömlek", "Yelek", "Takım"] {
            assert!(k.islemler.iter().any(|i| i.grup == g), "{g}");
        }
    }

    #[test]
    fn paca_secenek_ve_olcu_ister() {
        let k = Katalog::varsayilan();
        assert!(k
            .kontrol("pant-paca-kisalt", Some(30), None, false)
            .is_err());
        assert!(k
            .kontrol("pant-paca-kisalt", None, Some("Düz paça"), false)
            .is_err());
        assert!(k
            .kontrol("pant-paca-kisalt", Some(30), Some("Duble paça"), false)
            .unwrap()
            .is_empty());
        assert!(k
            .kontrol("pant-paca-kisalt", Some(30), Some("yok"), false)
            .is_err());
    }

    #[test]
    fn sinir_usta_onayi_ister() {
        let k = Katalog::varsayilan();
        assert!(k
            .kontrol("pant-bel-daralt", Some(40), None, false)
            .unwrap()
            .is_empty()); // tam 4 cm serbest
        let e = k
            .kontrol("pant-bel-daralt", Some(50), None, false)
            .unwrap_err();
        assert!(e.contains("Usta onayı"));
        let w = k.kontrol("pant-bel-daralt", Some(50), None, true).unwrap();
        assert!(w[0].contains("doğrulanmadı"));
    }

    #[test]
    fn sinirsiz_uyarili_islem_bilgi_verir() {
        let k = Katalog::varsayilan();
        let w = k.kontrol("cek-sirt-daralt", Some(30), None, false).unwrap();
        assert!(w[0].contains("kasıntı"));
    }

    #[test]
    fn olcusuz_islem_serbest_ve_bilinmeyen_hata() {
        let k = Katalog::varsayilan();
        assert!(k
            .kontrol("pant-fermuar", None, None, false)
            .unwrap()
            .is_empty());
        assert!(k.kontrol("yok", None, None, false).is_err());
    }

    #[test]
    fn kural_dosyasi_dogrulanir() {
        assert!(Katalog::json_den(
            r#"{"islemler":[{"kod":"a","grup":"g","ad":"x"},{"kod":"a","grup":"g","ad":"y"}]}"#
        )
        .is_err());
        assert!(Katalog::json_den(
            r#"{"islemler":[{"kod":"a","grup":"g","ad":"x","sinir_mm":0}]}"#
        )
        .is_err());
        assert!(Katalog::json_den("bozuk").is_err());
    }

    #[test]
    fn dizinden_dosya_ve_bozuk_dosya() {
        let d = std::env::temp_dir().join(format!("taa-terzi-test-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(Katalog::dizinden(&d).unwrap(), Katalog::varsayilan());
        std::fs::write(
            d.join("terzi-kurallari.json"),
            r#"{"islemler":[{"kod":"a","grup":"g","ad":"x"}]}"#,
        )
        .unwrap();
        assert_eq!(Katalog::dizinden(&d).unwrap().islemler.len(), 1);
        std::fs::write(d.join("terzi-kurallari.json"), "bozuk").unwrap();
        assert!(Katalog::dizinden(&d).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
