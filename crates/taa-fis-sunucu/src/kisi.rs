//! Kişi kaydı: fişteki satış danışmanı, ölçü alan ve düzenleyen buradan seçilir; fişe ad değil kimlik yazılır.
//! Kaynaklar: (a) TAA Çatı/havuz kişi listesi (`rehber.kisi` + görevlendirme, `kurulum` modülü eşitler,
//! `cati-kisiler.json` önbelleği); bu liste bir kez alındıysa ESAS odur ve yerel kayıt seçimden düşer;
//! (b) bu sunucunun `kisiler.jsonl` dosyası (İlk kurulum ya da Personel ekranında yönetici ekler/kapatır),
//! (c) onaylı Telegram hesapları (usta/personel/yönetici; mağaza ortak hesabı kişi değildir).
//! Kayıt eklemelidir; kapatılan kişi listeden düşer ama eski fişlerde adı çözülmeye devam eder.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kisi {
    pub id: String,
    pub ad: String,
    /// "danisman", "usta", "personel", "yonetici" (görüntü içindir; yetki vermez).
    #[serde(default)]
    pub rol: String,
    #[serde(default)]
    pub tarih: String,
    /// Nereden geldi: "" (bu bilgisayar, yönetici), "ilk-kurulum", "cati", "telegram". Yetki vermez.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kaynak: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "lowercase")]
enum Kayit {
    Ekle(Kisi),
    Kapat { id: String, tarih: String },
}

pub struct Kisiler {
    pub yol: PathBuf,
}

pub const ROLLER: [&str; 4] = ["danisman", "usta", "personel", "yonetici"];

/// Kimlik biçimi: harf/rakam/tire, 3-40 karakter.
pub fn kimlik_gecerli(id: &str) -> bool {
    (3..=40).contains(&id.len())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn rastgele_id() -> Result<String, String> {
    let mut b = [0u8; 4];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(format!(
        "K-{}",
        b.iter().map(|x| format!("{x:02x}")).collect::<String>()
    ))
}

impl Kisiler {
    pub fn dizinden(dizin: &Path) -> Self {
        Kisiler {
            yol: dizin.join("kisiler.jsonl"),
        }
    }

    fn oku(&self) -> Result<Vec<Kayit>, String> {
        taa_dosya::satirlari_oku(&self.yol)
            .map_err(|e| e.to_string())?
            .iter()
            .map(|l| serde_json::from_str(l).map_err(|e| format!("kişi kaydı bozuk: {e}")))
            .collect()
    }

    /// Yerel kayıttaki tüm kişiler (kapatılanlar dahil) ve kapalı kimlikler.
    fn yerel(&self) -> Result<(Vec<Kisi>, Vec<String>), String> {
        let mut hepsi = Vec::new();
        let mut kapali = Vec::new();
        for k in self.oku()? {
            match k {
                Kayit::Ekle(k) => hepsi.push(k),
                Kayit::Kapat { id, .. } => kapali.push(id),
            }
        }
        Ok((hepsi, kapali))
    }

    pub fn ekle(&self, ad: &str, rol: &str, tarih: &str) -> Result<Kisi, String> {
        self.ekle_kaynakli(ad, rol, tarih, "")
    }

    /// Ad ve rolü denetler; kaydetmez (toplu eklemede önce hepsi denetlenir).
    pub fn denetle(ad: &str, rol: &str) -> Result<String, String> {
        let ad: String = ad.split_whitespace().collect::<Vec<_>>().join(" ");
        if ad.is_empty() || ad.chars().count() > 40 || ad.chars().any(char::is_control) {
            return Err("ad 1-40 karakter olmalı".into());
        }
        if !ROLLER.contains(&rol) {
            return Err("rol danisman, usta, personel ya da yonetici olmalı".into());
        }
        Ok(ad)
    }

    pub fn ekle_kaynakli(
        &self,
        ad: &str,
        rol: &str,
        tarih: &str,
        kaynak: &str,
    ) -> Result<Kisi, String> {
        let ad = Self::denetle(ad, rol)?;
        let k = Kisi {
            id: rastgele_id()?,
            ad,
            rol: rol.into(),
            tarih: tarih.into(),
            kaynak: kaynak.into(),
        };
        let s = serde_json::to_string(&Kayit::Ekle(k.clone())).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.yol, &s).map_err(|e| e.to_string())?;
        Ok(k)
    }

    pub fn kapat(&self, id: &str, tarih: &str) -> Result<(), String> {
        let (hepsi, kapali) = self.yerel()?;
        if !hepsi.iter().any(|k| k.id == id) || kapali.iter().any(|k| k == id) {
            return Err(format!("açık kişi yok: {id}"));
        }
        let s = serde_json::to_string(&Kayit::Kapat {
            id: id.into(),
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.yol, &s).map_err(|e| e.to_string())
    }
}

/// Telegram hesaplarından kişiler: onaylı, mağaza ortak hesabı değil, şubesi bizimki ya da boş.
fn telegram_kisileri(dizin: &Path, sube: &str) -> Vec<(Kisi, bool)> {
    use taa_telegram::hesap::{HesapDurum, Hesaplar, Rol};
    Hesaplar { dizin }
        .guncel()
        .unwrap_or_default()
        .into_iter()
        .filter(|h| h.rol != Rol::Magaza && !h.usta_id.is_empty())
        .filter(|h| h.sube.is_empty() || sube.is_empty() || h.sube == sube)
        .map(|h| {
            let rol = match h.rol {
                Rol::Usta => "usta",
                Rol::Personel => "personel",
                _ => "yonetici",
            };
            let ad = if h.profil_adi.trim().is_empty() {
                h.usta_id.clone()
            } else {
                h.profil_adi.clone()
            };
            (
                Kisi {
                    id: h.usta_id,
                    ad,
                    rol: rol.into(),
                    tarih: String::new(),
                    kaynak: "telegram".into(),
                },
                h.durum == HesapDurum::Onayli,
            )
        })
        .collect()
}

/// Seçilebilir kişiler (açık olanlar), ada göre sıralı. TAA Çatı listesi esassa yerel kayıt seçimden düşer.
pub fn secilebilir(dizin: &Path, sube: &str) -> Vec<Kisi> {
    let mut v: Vec<Kisi> = match crate::kurulum::cati_kisileri(dizin) {
        Some(c) => c,
        None => {
            let (hepsi, kapali) = Kisiler::dizinden(dizin).yerel().unwrap_or_default();
            hepsi
                .into_iter()
                .filter(|k| !kapali.contains(&k.id))
                .collect()
        }
    };
    for (k, acik) in telegram_kisileri(dizin, sube) {
        if acik && !v.iter().any(|x| x.id == k.id) {
            v.push(k);
        }
    }
    v.sort_by_key(|k| k.ad.to_lowercase());
    v
}

/// Kimlik -> ad haritası (kapatılmış kişiler dahil; eski fişlerin adı çözülsün diye).
pub fn ad_haritasi(dizin: &Path, sube: &str) -> std::collections::BTreeMap<String, String> {
    let mut m = std::collections::BTreeMap::new();
    for (k, _) in telegram_kisileri(dizin, sube) {
        m.insert(k.id, k.ad);
    }
    let (hepsi, _) = Kisiler::dizinden(dizin).yerel().unwrap_or_default();
    for k in hepsi {
        m.insert(k.id, k.ad);
    }
    for k in crate::kurulum::cati_onbellegi(dizin) {
        m.insert(k.id, k.ad);
    }
    m
}

/// Kimlikten ad. Bulunamazsa None.
pub fn ad_coz(dizin: &Path, sube: &str, id: &str) -> Option<String> {
    if id.is_empty() {
        return None;
    }
    ad_haritasi(dizin, sube).remove(id)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn ekle_kapat_ve_coz() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let k = Kisiler::dizinden(&d);
        assert!(k.ekle(" ", "usta", "t").is_err());
        assert!(k.ekle("Deneme", "patron", "t").is_err());
        let a = k
            .ekle("  Deneme   Kişi ", "danisman", "2026-11-17")
            .unwrap();
        assert_eq!(a.ad, "Deneme Kişi");
        assert!(a.id.starts_with("K-") && kimlik_gecerli(&a.id));
        let b = k.ekle("Bir Usta", "usta", "2026-11-17").unwrap();
        assert_eq!(secilebilir(&d, "MRK01").len(), 2);
        k.kapat(&a.id, "2026-11-17").unwrap();
        assert!(k.kapat(&a.id, "2026-11-17").is_err());
        let s = secilebilir(&d, "MRK01");
        assert_eq!(s, vec![b]);
        // Kapatılan kişinin adı eski fişler için çözülür.
        assert_eq!(ad_coz(&d, "MRK01", &a.id).as_deref(), Some("Deneme Kişi"));
        assert_eq!(ad_coz(&d, "MRK01", "K-yok"), None);
    }
}
