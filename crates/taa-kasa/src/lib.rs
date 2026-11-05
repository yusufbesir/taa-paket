//! taa-kasa (TaaKasaDefteri): banknot/bozuk sayımı, beklenen toplamla fark, kapanış defteri.
//! Defter yalnız eklemeli JSONL'dir: kayıt silinmez/değişmez, düzeltme "iptal" kaydıyla yapılır.
//! Beklenen tutar (örn. Nebim toplamı) ELLE girilir; hiçbir sisteme bağlanmaz.
//! Özellik anahtarı: "kasa-defteri" (varsayılan açık). Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

pub const OZELLIK: &str = "kasa-defteri";

/// Geçerli madeni/kâğıt para değerleri (kuruş).
pub const DEGERLER: [i64; 12] = [20000, 10000, 5000, 2000, 1000, 500, 100, 50, 25, 10, 5, 1];

fn kapali() -> Result<(), String> {
    if taa_ayar::ozellik_acik(OZELLIK, true) {
        Ok(())
    } else {
        Err(taa_ayar::kapali_mesaji(OZELLIK))
    }
}

/// Değer (kuruş) -> adet.
pub fn sayim_toplam(adetler: &BTreeMap<i64, u32>) -> Result<i64, String> {
    let mut t: i64 = 0;
    for (d, a) in adetler {
        if !DEGERLER.contains(d) {
            return Err(format!("geçersiz para değeri: {d} kuruş"));
        }
        t = t
            .checked_add(d.checked_mul(*a as i64).ok_or("taşma")?)
            .ok_or("taşma")?;
    }
    Ok(t)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kapanis {
    pub id: String,
    /// YYYY-AA-GG
    pub tarih: String,
    pub sube: String,
    pub kullanici: String,
    pub adetler: BTreeMap<i64, u32>,
    pub sayilan_kurus: i64,
    pub beklenen_kurus: i64,
    pub fark_kurus: i64,
    pub not: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "lowercase")]
enum Kayit {
    Kapanis(Kapanis),
    Iptal {
        id: String,
        neden: String,
        kullanici: String,
    },
}

fn tarih_gecerli(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && t.char_indices()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && matches!(t[5..7].parse::<u32>(), Ok(1..=12))
        && matches!(t[8..10].parse::<u32>(), Ok(1..=31))
}

pub struct Defter<'a> {
    pub yol: &'a Path,
}

impl Defter<'_> {
    fn kayitlar(&self) -> Result<Vec<Kayit>, String> {
        taa_dosya::satirlari_oku(self.yol)
            .map_err(|e| e.to_string())?
            .iter()
            .map(|l| serde_json::from_str(l).map_err(|e| format!("defter satırı bozuk: {e}")))
            .collect()
    }

    /// Geçerli (iptal edilmemiş) kapanışlar.
    pub fn kapanislar(&self) -> Result<Vec<Kapanis>, String> {
        let k = self.kayitlar()?;
        let iptal: Vec<&str> = k
            .iter()
            .filter_map(|x| {
                if let Kayit::Iptal { id, .. } = x {
                    Some(id.as_str())
                } else {
                    None
                }
            })
            .collect();
        Ok(k.iter()
            .filter_map(|x| {
                if let Kayit::Kapanis(c) = x {
                    Some(c.clone())
                } else {
                    None
                }
            })
            .filter(|c| !iptal.contains(&c.id.as_str()))
            .collect())
    }

    /// Kapanış ekler. Fark sıfır değilse açıklama notu zorunludur.
    pub fn kapanis_ekle(
        &self,
        tarih: &str,
        sube: &str,
        kullanici: &str,
        adetler: BTreeMap<i64, u32>,
        beklenen_kurus: i64,
        not: &str,
    ) -> Result<Kapanis, String> {
        kapali()?;
        if !tarih_gecerli(tarih) {
            return Err("tarih YYYY-AA-GG olmalı".into());
        }
        if sube.trim().is_empty() || kullanici.trim().is_empty() {
            return Err("şube ve kullanıcı gerekli".into());
        }
        if beklenen_kurus < 0 {
            return Err("beklenen tutar negatif olamaz".into());
        }
        let sayilan = sayim_toplam(&adetler)?;
        let fark = sayilan - beklenen_kurus;
        if fark != 0 && not.trim().is_empty() {
            return Err(format!(
                "{} fark var; açıklama notu yazın",
                kurus_metni(fark)
            ));
        }
        let sira = self
            .kayitlar()?
            .iter()
            .filter(|k| matches!(k, Kayit::Kapanis(c) if c.tarih == tarih))
            .count()
            + 1;
        let c = Kapanis {
            id: format!("K-{}-{sira}", tarih.replace('-', "")),
            tarih: tarih.into(),
            sube: sube.trim().into(),
            kullanici: kullanici.trim().into(),
            adetler,
            sayilan_kurus: sayilan,
            beklenen_kurus,
            fark_kurus: fark,
            not: not.trim().into(),
        };
        let satir = serde_json::to_string(&Kayit::Kapanis(c.clone())).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &satir).map_err(|e| e.to_string())?;
        Ok(c)
    }

    /// Kapanışı iptal eder (silmez); neden zorunlu.
    pub fn iptal(&self, id: &str, neden: &str, kullanici: &str) -> Result<(), String> {
        kapali()?;
        if neden.trim().is_empty() {
            return Err("iptal nedeni yazın".into());
        }
        if !self.kapanislar()?.iter().any(|c| c.id == id) {
            return Err(format!("geçerli kapanış yok: {id}"));
        }
        let satir = serde_json::to_string(&Kayit::Iptal {
            id: id.into(),
            neden: neden.trim().into(),
            kullanici: kullanici.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &satir).map_err(|e| e.to_string())
    }
}

/// 123456 -> "1.234,56 TL"; negatif için başına "-".
pub fn kurus_metni(k: i64) -> String {
    let m = k.unsigned_abs();
    let tl = (m / 100).to_string();
    let mut g = String::new();
    for (i, c) in tl.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            g.push('.');
        }
        g.push(c);
    }
    format!(
        "{}{},{:02} TL",
        if k < 0 { "-" } else { "" },
        g.chars().rev().collect::<String>(),
        m % 100
    )
}

#[cfg(test)]
mod testler {
    use super::*;

    fn yol(ad: &str) -> std::path::PathBuf {
        taa_dosya::gecici_klasor()
            .unwrap()
            .join(format!("{ad}.jsonl"))
    }

    fn sayim() -> BTreeMap<i64, u32> {
        BTreeMap::from([(20000, 2), (5000, 1), (100, 3), (50, 1)])
    }

    #[test]
    fn toplam_ve_metin() {
        assert_eq!(sayim_toplam(&sayim()).unwrap(), 45350);
        assert!(sayim_toplam(&BTreeMap::from([(300, 1)])).is_err());
        assert_eq!(kurus_metni(123456), "1.234,56 TL");
        assert_eq!(kurus_metni(-50), "-0,50 TL");
    }

    #[test]
    fn kapanis_ve_fark_notu() {
        let y = yol("a");
        let d = Defter { yol: &y };
        assert!(d
            .kapanis_ekle("2026-11-04", "Merkez", "ayse", sayim(), 45000, "")
            .is_err());
        let c = d
            .kapanis_ekle(
                "2026-11-04",
                "Merkez",
                "ayse",
                sayim(),
                45000,
                "bozuk para fazlası",
            )
            .unwrap();
        assert_eq!(c.fark_kurus, 350);
        assert_eq!(c.id, "K-20261104-1");
        let c2 = d
            .kapanis_ekle("2026-11-04", "Merkez", "ayse", sayim(), 45350, "")
            .unwrap();
        assert_eq!(c2.id, "K-20261104-2");
        assert_eq!(d.kapanislar().unwrap().len(), 2);
    }

    #[test]
    fn iptal_silmez() {
        let y = yol("b");
        let d = Defter { yol: &y };
        let c = d
            .kapanis_ekle("2026-11-04", "M", "u", sayim(), 45350, "")
            .unwrap();
        assert!(d.iptal(&c.id, "", "u").is_err());
        d.iptal(&c.id, "yanlış gün", "u").unwrap();
        assert!(d.kapanislar().unwrap().is_empty());
        assert_eq!(taa_dosya::satirlari_oku(&y).unwrap().len(), 2); // kayıt dosyada durur
        assert!(d.iptal(&c.id, "tekrar", "u").is_err());
    }

    #[test]
    fn dogrulamalar() {
        let y = yol("c");
        let d = Defter { yol: &y };
        assert!(d
            .kapanis_ekle("04.11.2026", "M", "u", sayim(), 0, "x")
            .is_err());
        assert!(d
            .kapanis_ekle("2026-13-01", "M", "u", sayim(), 0, "x")
            .is_err());
        assert!(d
            .kapanis_ekle("2026-11-04", " ", "u", sayim(), 0, "x")
            .is_err());
    }
}
