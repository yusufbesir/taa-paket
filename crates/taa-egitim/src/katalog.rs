//! Video/ders kataloğu: kimlik, başlık, konu, görev etiketleri, süre, sürüm. Silme ucu yoktur; yeni sürüm eskinin yerine geçer.
//! Sahip: Yusuf. yazan: [Claude#egitim 2026-11-26]

use crate::{gecersiz, Hata};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use taa_rehber_model::model::{kimlik_gecerli, rol_kodu_gecerli};

/// Kasa eğitimi (EG5; açılış, devir, gün sonu) bir konudur.
pub const KONU_KASA: &str = "kasa-egitimi";
pub const EN_COK_ETIKET: usize = 16;
pub const EN_COK_SURE_SN: u64 = 86_400;
pub const EN_COK_DERS: usize = 5000;
const EN_COK_BASLIK: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ders {
    pub id: String,
    pub baslik: String,
    /// Konu kimliği (`[a-z0-9][a-z0-9_-]{0,63}`), örn. `kasa-egitimi`.
    pub konu: String,
    /// Görev (rol) kodları: rehberdeki rol kodu biçimi. Boş olabilir (göreve bağlı değil).
    #[serde(default)]
    pub gorev_etiketleri: Vec<String>,
    pub sure_sn: u64,
    pub surum: u32,
}

impl Ders {
    pub fn dogrula(&self) -> Result<(), Hata> {
        if !kimlik_gecerli(&self.id) {
            return Err(gecersiz("id", "[a-z0-9][a-z0-9_-]{0,63} olmalı"));
        }
        let b = self.baslik.trim();
        if b.is_empty()
            || b.chars().count() > EN_COK_BASLIK
            || self.baslik.chars().any(char::is_control)
        {
            return Err(gecersiz(
                "baslik",
                "boş olamaz, en çok 120 karakter, denetim karakteri yok",
            ));
        }
        if !kimlik_gecerli(&self.konu) {
            return Err(gecersiz("konu", "[a-z0-9][a-z0-9_-]{0,63} olmalı"));
        }
        if self.gorev_etiketleri.len() > EN_COK_ETIKET {
            return Err(gecersiz("gorev_etiketleri", "en çok 16 etiket"));
        }
        for (i, e) in self.gorev_etiketleri.iter().enumerate() {
            if !rol_kodu_gecerli(e) {
                return Err(gecersiz(
                    "gorev_etiketleri",
                    format!("geçersiz rol kodu: {e}"),
                ));
            }
            if self.gorev_etiketleri[..i].contains(e) {
                return Err(gecersiz("gorev_etiketleri", format!("tekrar: {e}")));
            }
        }
        if self.sure_sn == 0 || self.sure_sn > EN_COK_SURE_SN {
            return Err(gecersiz("sure_sn", "1..=86400 olmalı"));
        }
        if self.surum == 0 {
            return Err(gecersiz("surum", "en az 1"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Katalog {
    dersler: BTreeMap<String, Ders>,
}

impl Katalog {
    pub fn yeni() -> Self {
        Self::default()
    }

    pub fn len(&self) -> usize {
        self.dersler.len()
    }

    pub fn is_empty(&self) -> bool {
        self.dersler.is_empty()
    }

    pub fn al(&self, id: &str) -> Option<&Ders> {
        self.dersler.get(id)
    }

    /// Yeni ders ekler (kimlik benzersiz).
    pub fn ekle(&mut self, ders: Ders) -> Result<(), Hata> {
        ders.dogrula()?;
        if self.dersler.contains_key(&ders.id) {
            return Err(Hata::DersVar(ders.id));
        }
        if self.dersler.len() >= EN_COK_DERS {
            return Err(Hata::Sinir("en çok 5000 ders"));
        }
        self.dersler.insert(ders.id.clone(), ders);
        Ok(())
    }

    /// Var olan dersin yeni sürümünü yazar; sürüm kesin artmalıdır. Eski sürümün izleme kayıtları geçersiz sayılır.
    pub fn yeni_surum(&mut self, ders: Ders) -> Result<(), Hata> {
        ders.dogrula()?;
        let eski = self
            .dersler
            .get(&ders.id)
            .ok_or_else(|| Hata::DersYok(ders.id.clone()))?;
        if ders.surum <= eski.surum {
            return Err(Hata::SurumGeri {
                ders: ders.id,
                eski: eski.surum,
                yeni: ders.surum,
            });
        }
        self.dersler.insert(ders.id.clone(), ders);
        Ok(())
    }

    /// Tüm dersler, kimliğe göre sıralı.
    pub fn hepsi(&self) -> Vec<&Ders> {
        self.dersler.values().collect()
    }

    /// Konuya ve/veya göreve göre süzer (ikisi de verilirse ikisi birlikte sağlanmalı). Görev eşleşmesi tam etiket
    /// eşleşmesidir; etiketi boş ders hiçbir görev süzgecine girmez.
    pub fn suz(&self, konu: Option<&str>, gorev: Option<&str>) -> Vec<&Ders> {
        self.dersler
            .values()
            .filter(|d| konu.is_none_or(|k| d.konu == k))
            .filter(|d| gorev.is_none_or(|g| d.gorev_etiketleri.iter().any(|e| e == g)))
            .collect()
    }

    /// Kullanılan konular, tekrarsız ve sıralı.
    pub fn konular(&self) -> Vec<&str> {
        let mut k: Vec<&str> = self.dersler.values().map(|d| d.konu.as_str()).collect();
        k.sort();
        k.dedup();
        k
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    pub(crate) fn ders(id: &str, konu: &str, etiketler: &[&str], sure: u64) -> Ders {
        Ders {
            id: id.into(),
            baslik: format!("Ders {id}"),
            konu: konu.into(),
            gorev_etiketleri: etiketler.iter().map(|s| s.to_string()).collect(),
            sure_sn: sure,
            surum: 1,
        }
    }

    #[test]
    fn ekle_ve_al() {
        let mut k = Katalog::yeni();
        k.ekle(ders("kasa-acilis", KONU_KASA, &["kasa-sorumlusu"], 600))
            .unwrap();
        assert_eq!(k.al("kasa-acilis").unwrap().sure_sn, 600);
        assert_eq!(k.len(), 1);
    }

    #[test]
    fn ayni_kimlik_reddedilir() {
        let mut k = Katalog::yeni();
        k.ekle(ders("a", "genel", &[], 60)).unwrap();
        assert_eq!(
            k.ekle(ders("a", "genel", &[], 60)),
            Err(Hata::DersVar("a".into()))
        );
    }

    #[test]
    fn dogrulama_kurallari() {
        let mut d = ders("a", "genel", &[], 60);
        d.id = "A b".into();
        assert!(d.dogrula().is_err());
        let mut d = ders("a", "genel", &[], 60);
        d.baslik = "  ".into();
        assert!(d.dogrula().is_err());
        let mut d = ders("a", "genel", &[], 60);
        d.baslik = "iki\nsatır".into();
        assert!(d.dogrula().is_err());
        let mut d = ders("a", "Genel", &[], 60);
        assert!(d.dogrula().is_err());
        d.konu = "genel".into();
        d.sure_sn = 0;
        assert!(d.dogrula().is_err());
        d.sure_sn = EN_COK_SURE_SN + 1;
        assert!(d.dogrula().is_err());
        d.sure_sn = 10;
        d.surum = 0;
        assert!(d.dogrula().is_err());
        assert!(ders("a", "genel", &["Kasa"], 60).dogrula().is_err());
        assert!(ders("a", "genel", &["kasa", "kasa"], 60).dogrula().is_err());
        let cok: Vec<String> = (0..17).map(|i| format!("r{i}")).collect();
        let mut d = ders("a", "genel", &[], 60);
        d.gorev_etiketleri = cok;
        assert!(d.dogrula().is_err());
    }

    #[test]
    fn konuya_ve_goreve_gore_suzme() {
        let mut k = Katalog::yeni();
        k.ekle(ders(
            "kasa-acilis",
            KONU_KASA,
            &["kasa-sorumlusu", "magaza-muduru"],
            600,
        ))
        .unwrap();
        k.ekle(ders("kasa-gun-sonu", KONU_KASA, &["kasa-sorumlusu"], 900))
            .unwrap();
        k.ekle(ders("iade", "satis", &["satis-danismani"], 300))
            .unwrap();
        k.ekle(ders("hosgeldin", "genel", &[], 120)).unwrap();
        let kasa: Vec<&str> = k
            .suz(Some(KONU_KASA), None)
            .iter()
            .map(|d| d.id.as_str())
            .collect();
        assert_eq!(kasa, ["kasa-acilis", "kasa-gun-sonu"]);
        let mudur: Vec<&str> = k
            .suz(None, Some("magaza-muduru"))
            .iter()
            .map(|d| d.id.as_str())
            .collect();
        assert_eq!(mudur, ["kasa-acilis"]);
        assert_eq!(k.suz(Some("satis"), Some("kasa-sorumlusu")).len(), 0);
        assert_eq!(k.suz(None, None).len(), 4);
        assert_eq!(k.suz(None, Some("yok")).len(), 0);
        assert_eq!(k.konular(), ["genel", KONU_KASA, "satis"]);
    }

    #[test]
    fn yeni_surum_artmali_ve_ders_var_olmali() {
        let mut k = Katalog::yeni();
        k.ekle(ders("a", "genel", &[], 60)).unwrap();
        let mut v2 = ders("a", "genel", &[], 90);
        assert!(matches!(
            k.yeni_surum(v2.clone()),
            Err(Hata::SurumGeri { .. })
        ));
        v2.surum = 2;
        k.yeni_surum(v2).unwrap();
        assert_eq!(k.al("a").unwrap().sure_sn, 90);
        let mut yok = ders("b", "genel", &[], 60);
        yok.surum = 2;
        assert_eq!(k.yeni_surum(yok), Err(Hata::DersYok("b".into())));
    }

    #[test]
    fn bilinmeyen_alan_reddedilir() {
        let j = r#"{"id":"a","baslik":"x","konu":"genel","sure_sn":5,"surum":1,"ad":"Ali"}"#;
        assert!(serde_json::from_str::<Ders>(j).is_err());
        let j = r#"{"id":"a","baslik":"x","konu":"genel","sure_sn":5,"surum":1}"#;
        assert!(serde_json::from_str::<Ders>(j)
            .unwrap()
            .gorev_etiketleri
            .is_empty());
    }
}
