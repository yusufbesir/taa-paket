//! taa-subesor (TaaŞubeSor): "48 beden lacivert takım var mı?" sorusunu şubelere iletir, "var / yok / ayırdım" yanıtlarını toplar.
//! Eklemeli JSONL defter (silme yok). Ağ taşıması bu crate'in dışındadır (sonraki aşama: yerel sunucu/telefon sayfası);
//! stok sistemine (Nebim) hiç dokunulmaz. Özellik anahtarı: "subesor" (varsayılan açık).
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const OZELLIK: &str = "subesor";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Yanit {
    Var,
    Yok,
    Ayirdim,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Istek {
    pub id: String,
    pub soran_sube: String,
    pub metin: String,
    pub tarih: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct YanitKaydi {
    pub id: String,
    pub sube: String,
    pub yanit: Yanit,
    #[serde(default)]
    pub not: String,
    pub tarih: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "lowercase")]
enum Kayit {
    Istek(Istek),
    Yanit(YanitKaydi),
}

/// Bir isteğin güncel özeti (her şubenin son yanıtı).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ozet {
    pub istek: Istek,
    pub yanitlar: Vec<YanitKaydi>,
}

impl Ozet {
    /// En az bir şube "var" ya da "ayırdım" dediyse karşılandı sayılır.
    pub fn karsilandi(&self) -> bool {
        self.yanitlar
            .iter()
            .any(|y| matches!(y.yanit, Yanit::Var | Yanit::Ayirdim))
    }

    /// Henüz yanıt vermeyen şubeler (soran şube hariç).
    pub fn bekleyenler(&self, subeler: &[&str]) -> Vec<String> {
        subeler
            .iter()
            .filter(|s| {
                **s != self.istek.soran_sube && !self.yanitlar.iter().any(|y| y.sube == **s)
            })
            .map(|s| s.to_string())
            .collect()
    }
}

/// Metinden beden sayısını (30..=70, tek başına sayı) ve geri kalan ürün tarifini ayırır.
pub fn metin_coz(metin: &str) -> (Option<u32>, String) {
    let mut beden = None;
    let mut kalan = Vec::new();
    for k in metin.split_whitespace() {
        match k.parse::<u32>() {
            Ok(n) if beden.is_none() && (30..=70).contains(&n) => beden = Some(n),
            _ => kalan.push(k),
        }
    }
    (beden, kalan.join(" "))
}

pub struct Defter<'a> {
    pub yol: &'a Path,
}

fn kapali() -> Result<(), String> {
    if taa_ayar::ozellik_acik(OZELLIK, true) {
        Ok(())
    } else {
        Err(taa_ayar::kapali_mesaji(OZELLIK))
    }
}

impl Defter<'_> {
    fn kayitlar(&self) -> Result<Vec<Kayit>, String> {
        taa_dosya::satirlari_oku(self.yol)
            .map_err(|e| e.to_string())?
            .iter()
            .map(|l| serde_json::from_str(l).map_err(|e| format!("defter satırı bozuk: {e}")))
            .collect()
    }

    pub fn ozetler(&self) -> Result<Vec<Ozet>, String> {
        let mut o: Vec<Ozet> = Vec::new();
        for k in self.kayitlar()? {
            match k {
                Kayit::Istek(i) => o.push(Ozet {
                    istek: i,
                    yanitlar: Vec::new(),
                }),
                Kayit::Yanit(y) => {
                    if let Some(x) = o.iter_mut().find(|x| x.istek.id == y.id) {
                        x.yanitlar.retain(|e| e.sube != y.sube); // aynı şubenin son yanıtı geçerli
                        x.yanitlar.push(y);
                    }
                }
            }
        }
        Ok(o)
    }

    pub fn sor(&self, soran_sube: &str, metin: &str, tarih: &str) -> Result<Istek, String> {
        kapali()?;
        if soran_sube.trim().is_empty() || metin.trim().is_empty() {
            return Err("şube ve soru metni gerekli".into());
        }
        if metin.chars().count() > 200 {
            return Err("soru en çok 200 karakter olmalı".into());
        }
        let n = self.ozetler()?.len() + 1;
        let i = Istek {
            id: format!("S-{n:04}"),
            soran_sube: soran_sube.trim().into(),
            metin: metin.trim().into(),
            tarih: tarih.into(),
        };
        let s = serde_json::to_string(&Kayit::Istek(i.clone())).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())?;
        Ok(i)
    }

    pub fn yanitla(
        &self,
        id: &str,
        sube: &str,
        yanit: Yanit,
        not: &str,
        tarih: &str,
    ) -> Result<(), String> {
        kapali()?;
        let o = self
            .ozetler()?
            .into_iter()
            .find(|o| o.istek.id == id)
            .ok_or_else(|| format!("soru yok: {id}"))?;
        if o.istek.soran_sube == sube {
            return Err("soran şube kendi sorusunu yanıtlayamaz".into());
        }
        if sube.trim().is_empty() {
            return Err("şube gerekli".into());
        }
        let y = YanitKaydi {
            id: id.into(),
            sube: sube.trim().into(),
            yanit,
            not: not.trim().into(),
            tarih: tarih.into(),
        };
        let s = serde_json::to_string(&Kayit::Yanit(y)).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn yol() -> std::path::PathBuf {
        taa_dosya::gecici_klasor().unwrap().join("s.jsonl")
    }

    #[test]
    fn metin() {
        assert_eq!(
            metin_coz("48 beden lacivert takım"),
            (Some(48), "beden lacivert takım".into())
        );
        assert_eq!(metin_coz("gri pantolon"), (None, "gri pantolon".into()));
        assert_eq!(
            metin_coz("2 adet 50 siyah"),
            (Some(50), "2 adet siyah".into())
        );
    }

    #[test]
    fn akis() {
        let y = yol();
        let d = Defter { yol: &y };
        let i = d.sor("Merkez", "48 lacivert takım", "2026-11-04").unwrap();
        assert_eq!(i.id, "S-0001");
        assert!(d
            .yanitla(&i.id, "Merkez", Yanit::Var, "", "2026-11-04")
            .is_err());
        assert!(d
            .yanitla("S-0009", "Çarşı", Yanit::Var, "", "2026-11-04")
            .is_err());
        d.yanitla(&i.id, "Çarşı", Yanit::Yok, "", "2026-11-04")
            .unwrap();
        let o = &d.ozetler().unwrap()[0];
        assert!(!o.karsilandi());
        assert_eq!(o.bekleyenler(&["Merkez", "Çarşı", "AVM"]), vec!["AVM"]);
        // Çarşı fikrini değiştirir: son yanıt geçerli
        d.yanitla(
            &i.id,
            "Çarşı",
            Yanit::Ayirdim,
            "kasada bekliyor",
            "2026-11-04",
        )
        .unwrap();
        let o = &d.ozetler().unwrap()[0];
        assert!(o.karsilandi());
        assert_eq!(o.yanitlar.len(), 1);
        assert_eq!(taa_dosya::satirlari_oku(&y).unwrap().len(), 3);
    }

    #[test]
    fn dogrulama() {
        let y = yol();
        let d = Defter { yol: &y };
        assert!(d.sor("", "x", "2026-11-04").is_err());
        assert!(d.sor("M", " ", "2026-11-04").is_err());
        assert!(d.sor("M", &"a".repeat(201), "2026-11-04").is_err());
    }
}
