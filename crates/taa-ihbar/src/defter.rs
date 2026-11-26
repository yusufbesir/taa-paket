//! Yalnız-ekleme JSONL defter (taa-dosya `satir_ekle`). Kayıtlar silinmez, değiştirilmez. Ihbarları görüntülemek
//! yalnız `Yonetici` yetkisiyle yapılır; yetki yalnız açık bir yönetici bayrağından üretilir.
//! Sahip: Yusuf. yazan: [Claude#ihbar 2026-11-26]

use crate::kayit::Ihbar;
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Bir kanalın teslim sonucu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Teslim {
    Gitti,
    /// Kanal ayarda tanımlı değil (ör. e-posta adresi boş).
    Kapali,
    Hata(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "lowercase")]
pub enum Kayit {
    Ihbar(Ihbar),
    Teslimat {
        takip_no: String,
        tarih: String,
        telegram: Teslim,
        eposta: Teslim,
    },
}

/// Yönetici yetkisi. Yalnız `bayraktan(true)` ile üretilir.
#[derive(Debug, Clone, Copy)]
pub struct Yonetici(());

impl Yonetici {
    pub fn bayraktan(yonetici_bayragi: bool) -> Result<Yonetici, String> {
        if yonetici_bayragi {
            Ok(Yonetici(()))
        } else {
            Err("ihbarları yalnız yönetici görüntüleyebilir".into())
        }
    }
}

pub struct Defter<'a> {
    pub yol: &'a Path,
}

impl Defter<'_> {
    fn kayitlar(&self) -> Result<Vec<Kayit>, String> {
        taa_dosya::satirlari_oku(self.yol)
            .map_err(|e| e.to_string())?
            .iter()
            .map(|l| {
                serde_json::from_str(l).map_err(|e| format!("ihbar defteri satırı bozuk: {e}"))
            })
            .collect()
    }

    fn yaz(&self, k: &Kayit) -> Result<(), String> {
        let s = serde_json::to_string(k).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    /// Kayıt içeriği okunmaz; yalnız ihbar sayısı (yetki gerekmez).
    pub fn sayi(&self) -> Result<usize, String> {
        Ok(self
            .kayitlar()?
            .iter()
            .filter(|k| matches!(k, Kayit::Ihbar(_)))
            .count())
    }

    pub(crate) fn takip_no_var(&self, no: &str) -> Result<bool, String> {
        Ok(self
            .kayitlar()?
            .iter()
            .any(|k| matches!(k, Kayit::Ihbar(i) if i.takip_no == no)))
    }

    pub fn ekle(&self, i: &Ihbar) -> Result<(), String> {
        if self.takip_no_var(&i.takip_no)? {
            return Err("bu takip numarası defterde var".into());
        }
        self.yaz(&Kayit::Ihbar(i.clone()))
    }

    pub fn teslimat_yaz(
        &self,
        takip_no: &str,
        tarih: &str,
        telegram: Teslim,
        eposta: Teslim,
    ) -> Result<(), String> {
        self.yaz(&Kayit::Teslimat {
            takip_no: takip_no.into(),
            tarih: tarih.into(),
            telegram,
            eposta,
        })
    }

    /// Yalnız yönetici: tüm ihbarlar, eklenme sırasıyla.
    pub fn ihbarlar(&self, _y: &Yonetici) -> Result<Vec<Ihbar>, String> {
        Ok(self
            .kayitlar()?
            .into_iter()
            .filter_map(|k| match k {
                Kayit::Ihbar(i) => Some(i),
                _ => None,
            })
            .collect())
    }

    pub fn bul(&self, y: &Yonetici, takip_no: &str) -> Result<Option<Ihbar>, String> {
        Ok(self
            .ihbarlar(y)?
            .into_iter()
            .find(|i| i.takip_no == takip_no))
    }

    /// Yalnız yönetici: bir ihbarın en son teslim durumu `(telegram, eposta)`.
    pub fn son_teslimat(
        &self,
        _y: &Yonetici,
        takip_no: &str,
    ) -> Result<Option<(Teslim, Teslim)>, String> {
        let mut son = None;
        for k in self.kayitlar()? {
            if let Kayit::Teslimat {
                takip_no: n,
                telegram,
                eposta,
                ..
            } = k
            {
                if n == takip_no {
                    son = Some((telegram, eposta));
                }
            }
        }
        Ok(son)
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::kayit::{Girdi, Kategori};

    fn ihbar(no: &str) -> Ihbar {
        Ihbar::olustur(
            Girdi {
                kategori: Kategori::Diger,
                metin: "deneme".into(),
                iletisim: None,
            },
            "2026-11-26",
            no.into(),
        )
        .unwrap()
    }

    #[test]
    fn yalniz_ekleme_ve_okuma() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let yol = d.join("alt").join("ihbar.jsonl");
        let df = Defter { yol: &yol };
        let y = Yonetici::bayraktan(true).unwrap();
        assert_eq!(df.sayi().unwrap(), 0);
        df.ekle(&ihbar("IH-1")).unwrap();
        let once = std::fs::read_to_string(&yol).unwrap();
        df.ekle(&ihbar("IH-2")).unwrap();
        let sonra = std::fs::read_to_string(&yol).unwrap();
        assert!(sonra.starts_with(&once), "eski satırlar değişmemeli");
        assert_eq!(sonra.lines().count(), 2);
        assert_eq!(df.sayi().unwrap(), 2);
        assert!(df.ekle(&ihbar("IH-1")).is_err());
        assert_eq!(df.bul(&y, "IH-2").unwrap().unwrap().takip_no, "IH-2");
        assert!(df.bul(&y, "IH-9").unwrap().is_none());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn yonetici_bayragi_sart() {
        assert!(Yonetici::bayraktan(false).is_err());
        assert!(Yonetici::bayraktan(true).is_ok());
    }

    #[test]
    fn teslimat_kaydi_en_sonuncu() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let yol = d.join("ihbar.jsonl");
        let df = Defter { yol: &yol };
        let y = Yonetici::bayraktan(true).unwrap();
        df.ekle(&ihbar("IH-1")).unwrap();
        assert!(df.son_teslimat(&y, "IH-1").unwrap().is_none());
        df.teslimat_yaz(
            "IH-1",
            "2026-11-26",
            Teslim::Hata("x".into()),
            Teslim::Kapali,
        )
        .unwrap();
        df.teslimat_yaz("IH-1", "2026-11-26", Teslim::Gitti, Teslim::Kapali)
            .unwrap();
        assert_eq!(
            df.son_teslimat(&y, "IH-1").unwrap(),
            Some((Teslim::Gitti, Teslim::Kapali))
        );
        assert_eq!(df.sayi().unwrap(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn bozuk_satir_hata_verir() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let yol = d.join("ihbar.jsonl");
        taa_dosya::satir_ekle(&yol, "bozuk").unwrap();
        assert!(Defter { yol: &yol }.sayi().is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
