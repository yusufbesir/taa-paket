// Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05]
//! kayit.json modeli (serde) ve sürüm sıralama yardımcısı.

use crate::paket::sha256_bayt;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PaketKaydi {
    #[serde(default)]
    pub kurulu: Vec<String>,
    #[serde(default)]
    pub secili: Option<String>,
    #[serde(default)]
    pub sabitle: bool,
    /// Geri dönülebilecek önceki seçili sürüm.
    #[serde(default)]
    pub onceki: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Kayit {
    pub sema: u32,
    pub kok: String,
    #[serde(default)]
    pub paketler: BTreeMap<String, PaketKaydi>,
    /// veri alanı -> sahibi olan paket adı
    #[serde(default)]
    pub veri_sahipleri: BTreeMap<String, String>,
    /// Kayıt bütünlüğü: bu alan boş sayılarak hesaplanan kanonik JSON'un SHA256'sı (`ozet_hesapla`).
    #[serde(default)]
    pub ozet: String,
}

impl Kayit {
    pub fn bos(kok: &str) -> Self {
        Kayit {
            sema: 1,
            kok: kok.to_string(),
            paketler: BTreeMap::new(),
            veri_sahipleri: BTreeMap::new(),
            ozet: String::new(),
        }
    }

    /// `ozet` alanı boş sayılarak kanonik (alan sırası sabit, harita anahtarları sıralı) JSON'un SHA256'sı.
    pub fn ozet_hesapla(&self) -> String {
        let mut k = self.clone();
        k.ozet = String::new();
        // Kayit yalnız dize/sayı/bool/harita içerir; serileştirme hata veremez.
        sha256_bayt(&serde_json::to_vec_pretty(&k).unwrap_or_default())
    }
}

/// Noktalı sürümleri sayısal parçalarla karşılaştırır (sayı olmayan parça metin sırasıyla).
pub fn surum_karsilastir(a: &str, b: &str) -> Ordering {
    let pa: Vec<&str> = a.split(['.', '-', '+']).collect();
    let pb: Vec<&str> = b.split(['.', '-', '+']).collect();
    for (x, y) in pa.iter().zip(pb.iter()) {
        let o = match (x.parse::<u64>(), y.parse::<u64>()) {
            (Ok(i), Ok(j)) => i.cmp(&j),
            _ => x.cmp(y),
        };
        if o != Ordering::Equal {
            return o;
        }
    }
    pa.len().cmp(&pb.len()).then_with(|| a.cmp(b))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn siralama() {
        assert_eq!(surum_karsilastir("0.2.0", "0.10.0"), Ordering::Less);
        assert_eq!(surum_karsilastir("1.0.0", "1.0.0"), Ordering::Equal);
        assert_eq!(surum_karsilastir("1.0", "1.0.1"), Ordering::Less);
    }

    #[test]
    fn ozet_kendini_saymaz() {
        let mut k = Kayit::bos("/x");
        let o = k.ozet_hesapla();
        k.ozet = o.clone();
        assert_eq!(k.ozet_hesapla(), o);
        k.paketler.insert("a".into(), PaketKaydi::default());
        assert_ne!(k.ozet_hesapla(), o);
    }
}
