//! taa-katalog: Nebim'den gelen ürünlerin yerel, salt okunur aynası. TaaRkod, Fiyat Etiketi ve Tutaa ortak okur.
//! Nebim'e yazılmaz. Özellik anahtarı: "katalog" (varsayılan açık).
//! Kaydederken eski dosya silinmez, `eski\` klasörüne taşınır.
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
pub use taa_nebim::Urun;

pub const OZELLIK: &str = "katalog";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Katalog {
    /// Kaynak dosya adı (bilgi amaçlı).
    pub kaynak: String,
    /// Anahtar: ürün kodu + barkod; aynı koda farklı beden/renk ayrı satırdır.
    pub urunler: BTreeMap<String, Urun>,
}

fn anahtar(u: &Urun) -> String {
    format!("{}|{}", u.kod, u.barkod)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Degisim {
    pub kod: String,
    pub barkod: String,
    pub eski: i64,
    pub yeni: i64,
}

impl Katalog {
    pub fn yeni(kaynak: &str, urunler: Vec<Urun>) -> Self {
        let mut m = BTreeMap::new();
        for u in urunler {
            m.insert(anahtar(&u), u);
        }
        Katalog {
            kaynak: kaynak.into(),
            urunler: m,
        }
    }

    pub fn barkod_ara(&self, barkod: &str) -> Option<&Urun> {
        self.urunler.values().find(|u| u.barkod == barkod)
    }

    pub fn kod_ara(&self, kod: &str) -> Vec<&Urun> {
        self.urunler.values().filter(|u| u.kod == kod).collect()
    }

    /// Ad içinde geçen metni arar (Türkçe büyük/küçük harf duyarsız, basit).
    pub fn ad_ara(&self, metin: &str) -> Vec<&Urun> {
        let m = metin.to_lowercase();
        self.urunler
            .values()
            .filter(|u| u.ad.to_lowercase().contains(&m))
            .collect()
    }

    /// Geçerli fiyat: indirimli varsa o, yoksa liste fiyatı.
    pub fn gecerli_fiyat(u: &Urun) -> i64 {
        u.indirimli.unwrap_or(u.fiyat)
    }

    /// `eski` katalogdan `self`'e geçerli fiyatı değişen (ya da yeni gelen) ürünler.
    /// Yeni gelen ürünün eski fiyatı 0 sayılır.
    pub fn fiyat_farki(&self, eski: &Katalog) -> Vec<Degisim> {
        let mut c = Vec::new();
        for (k, u) in &self.urunler {
            let yeni = Self::gecerli_fiyat(u);
            let e = eski.urunler.get(k).map(Self::gecerli_fiyat).unwrap_or(0);
            if e != yeni {
                c.push(Degisim {
                    kod: u.kod.clone(),
                    barkod: u.barkod.clone(),
                    eski: e,
                    yeni,
                });
            }
        }
        c
    }

    /// JSON olarak kaydeder; hedef varsa eskisi `eski\` altına zaman damgasıyla taşınır (silinmez).
    pub fn kaydet(&self, yol: &Path) -> Result<(), String> {
        if !taa_ayar::ozellik_acik(OZELLIK, true) {
            return Err(taa_ayar::kapali_mesaji(OZELLIK));
        }
        if yol.exists() {
            let dizin = yol.parent().unwrap_or(Path::new("."));
            let eski = dizin.join("eski");
            fs::create_dir_all(&eski).map_err(|e| e.to_string())?;
            let z = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let ad = yol
                .file_name()
                .map(|a| a.to_string_lossy().to_string())
                .unwrap_or_default();
            taa_dosya::atomik_yaz(
                &eski.join(format!("{z}-{ad}")),
                &fs::read(yol).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            fs::remove_file(yol).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        taa_dosya::atomik_yaz(yol, &json)
            .map(|_| ())
            .map_err(|e| e.to_string())
    }

    pub fn yukle(yol: &Path) -> Result<Katalog, String> {
        let b = fs::read(yol).map_err(|e| e.to_string())?;
        serde_json::from_slice(&b).map_err(|e| e.to_string())
    }
}

/// Nebim dışa aktarım klasöründen katalog kurar (salt okunur).
pub fn nebimden_kur(klasor: &Path) -> Result<Katalog, String> {
    let (yol, urunler) = taa_nebim::klasorden_oku(klasor).map_err(|e| e.0)?;
    let ad = yol
        .file_name()
        .map(|a| a.to_string_lossy().to_string())
        .unwrap_or_default();
    Ok(Katalog::yeni(&ad, urunler))
}

#[cfg(test)]
mod testler {
    use super::*;

    fn u(kod: &str, bk: &str, f: i64, i: Option<i64>) -> Urun {
        Urun {
            kod: kod.into(),
            barkod: bk.into(),
            ad: format!("Ürün {kod}"),
            renk: "".into(),
            beden: "".into(),
            fiyat: f,
            indirimli: i,
        }
    }

    #[test]
    fn arama() {
        let k = Katalog::yeni(
            "t",
            vec![
                u("A", "1", 100, None),
                u("A", "2", 100, None),
                u("B", "3", 200, None),
            ],
        );
        assert_eq!(k.kod_ara("A").len(), 2);
        assert_eq!(k.barkod_ara("3").unwrap().kod, "B");
        assert_eq!(k.ad_ara("ürün b").len(), 1);
    }

    #[test]
    fn fark() {
        let eski = Katalog::yeni("e", vec![u("A", "1", 100, None), u("B", "2", 200, None)]);
        let yeni = Katalog::yeni(
            "y",
            vec![
                u("A", "1", 100, Some(80)),
                u("B", "2", 200, None),
                u("C", "3", 50, None),
            ],
        );
        let f = yeni.fiyat_farki(&eski);
        assert_eq!(f.len(), 2);
        assert_eq!((f[0].eski, f[0].yeni), (100, 80));
        assert_eq!((f[1].kod.as_str(), f[1].eski), ("C", 0));
    }

    #[test]
    fn kaydet_eskiyi_tasir() {
        let d = std::env::temp_dir().join(format!("taa-katalog-test-{}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        let yol = d.join("katalog.json");
        Katalog::yeni("1", vec![u("A", "1", 100, None)])
            .kaydet(&yol)
            .unwrap();
        Katalog::yeni("2", vec![u("B", "2", 100, None)])
            .kaydet(&yol)
            .unwrap();
        assert_eq!(Katalog::yukle(&yol).unwrap().kaynak, "2");
        assert_eq!(fs::read_dir(d.join("eski")).unwrap().count(), 1);
        let _ = fs::remove_dir_all(&d);
    }
}
