//! Eşleşmiş telefonlar: eklemeli JSONL kayıt; belirteç yalnız SHA256 özeti olarak saklanır.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cihaz {
    pub id: String,
    pub ad: String,
    pub token_sha256: String,
    pub tarih: String,
    /// Kişisel telefon (kullanıcı girişi): fişi düzenleyen her zaman bu kişidir. Boş = mağaza telefonu.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kisi_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "lowercase")]
enum Kayit {
    Cihaz(Cihaz),
    Kapat {
        id: String,
        tarih: String,
    },
    /// Cihaz adı yalnız mağaza bilgisayarındaki programdan (yönetici) değişir.
    Ad {
        id: String,
        ad: String,
        tarih: String,
    },
    /// Yönetici bir cihaza yetki verir ya da geri alır (mağaza PC'si için id "PC").
    Yetki {
        id: String,
        yetki: String,
        ver: bool,
        tarih: String,
    },
}

/// Cihaza verilebilen yetkiler: (anahtar, açıklama, mağaza PC'sinde varsayılan, telefonda varsayılan).
pub const YETKILER: &[(&str, &str, bool, bool)] = &[(
    "duzelt",
    "Fiş düzeltme: sonradan işlem ekleme/çıkarma, kargo ↔ mağaza, teslim tarihi, fiş bağlama",
    true,
    false,
)];

pub struct Kayitci {
    pub yol: PathBuf,
}

pub fn sha256_hex(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 32 bayt işletim sistemi rastgeleliği, onaltılık.
pub fn yeni_token() -> Result<String, String> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// 6 haneli eşleştirme kodu.
pub fn yeni_kod() -> Result<String, String> {
    let mut b = [0u8; 4];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(format!("{:06}", u32::from_le_bytes(b) % 1_000_000))
}

/// 4 haneli ikinci eşleştirme kodu (telefonda görünür, yönetici bilgisayara yazar).
pub fn yeni_kod2() -> Result<String, String> {
    let mut b = [0u8; 4];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(format!("{:04}", u32::from_le_bytes(b) % 10_000))
}

/// Sabit zamanlı karşılaştırma (belirteç özeti).
fn esit(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

impl Kayitci {
    fn oku(&self) -> Result<Vec<Kayit>, String> {
        taa_dosya::satirlari_oku(&self.yol)
            .map_err(|e| e.to_string())?
            .iter()
            .map(|l| serde_json::from_str(l).map_err(|e| format!("cihaz kaydı bozuk: {e}")))
            .collect()
    }

    pub fn aktifler(&self) -> Result<Vec<Cihaz>, String> {
        let k = self.oku()?;
        let kapali: Vec<&str> = k
            .iter()
            .filter_map(|x| {
                if let Kayit::Kapat { id, .. } = x {
                    Some(id.as_str())
                } else {
                    None
                }
            })
            .collect();
        Ok(k.iter()
            .filter_map(|x| {
                if let Kayit::Cihaz(c) = x {
                    Some(c.clone())
                } else {
                    None
                }
            })
            .filter(|c| !kapali.contains(&c.id.as_str()))
            .map(|mut c| {
                for x in &k {
                    if let Kayit::Ad { id, ad, .. } = x {
                        if *id == c.id {
                            c.ad = ad.clone();
                        }
                    }
                }
                c
            })
            .collect())
    }

    /// Cihaz adını değiştirir (yalnız yönetici, mağaza bilgisayarından çağrılır).
    pub fn ad_degistir(&self, id: &str, ad: &str, tarih: &str) -> Result<(), String> {
        let ad = ad.trim();
        if ad.is_empty() || ad.chars().count() > 40 || ad.chars().any(char::is_control) {
            return Err("cihaz adı 1-40 karakter olmalı".into());
        }
        if !self.aktifler()?.iter().any(|c| c.id == id) {
            return Err(format!("aktif cihaz yok: {id}"));
        }
        let s = serde_json::to_string(&Kayit::Ad {
            id: id.into(),
            ad: ad.into(),
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.yol, &s).map_err(|e| e.to_string())
    }

    /// Yetki ver/geri al. id: aktif cihaz ya da "PC" (mağaza bilgisayarı).
    pub fn yetki_ata(&self, id: &str, yetki: &str, ver: bool, tarih: &str) -> Result<(), String> {
        if !YETKILER.iter().any(|(k, ..)| *k == yetki) {
            return Err(format!("bilinmeyen yetki: {yetki}"));
        }
        if id != crate::PC_ID && !self.aktifler()?.iter().any(|c| c.id == id) {
            return Err(format!("aktif cihaz yok: {id}"));
        }
        let s = serde_json::to_string(&Kayit::Yetki {
            id: id.into(),
            yetki: yetki.into(),
            ver,
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.yol, &s).map_err(|e| e.to_string())
    }

    /// Cihazın yetkisi var mı? Son kayıt geçerli; kayıt yoksa varsayılan (PC: açık, telefon: kapalı).
    pub fn yetki_var(&self, id: &str, yetki: &str) -> bool {
        let Some((_, _, pc, tel)) = YETKILER.iter().find(|(k, ..)| *k == yetki) else {
            return false;
        };
        let mut v = if id == crate::PC_ID { *pc } else { *tel };
        if let Ok(k) = self.oku() {
            for x in &k {
                if let Kayit::Yetki {
                    id: i,
                    yetki: y,
                    ver,
                    ..
                } = x
                {
                    if i == id && y == yetki {
                        v = *ver;
                    }
                }
            }
        }
        v
    }

    /// Cihazın açık yetkileri (liste ekranı için).
    pub fn yetkileri(&self, id: &str) -> Vec<&'static str> {
        YETKILER
            .iter()
            .map(|(k, ..)| *k)
            .filter(|k| self.yetki_var(id, k))
            .collect()
    }

    pub fn ekle(&self, ad: &str, token: &str, tarih: &str, kisi_id: &str) -> Result<Cihaz, String> {
        let ad = ad.trim();
        if ad.is_empty() || ad.chars().count() > 40 {
            return Err("telefon adı 1-40 karakter olmalı".into());
        }
        let n = self
            .oku()?
            .iter()
            .filter(|k| matches!(k, Kayit::Cihaz(_)))
            .count()
            + 1;
        let c = Cihaz {
            id: format!("T-{n:03}"),
            ad: ad.into(),
            token_sha256: sha256_hex(token),
            tarih: tarih.into(),
            kisi_id: kisi_id.into(),
        };
        let s = serde_json::to_string(&Kayit::Cihaz(c.clone())).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.yol, &s).map_err(|e| e.to_string())?;
        Ok(c)
    }

    pub fn kapat(&self, id: &str, tarih: &str) -> Result<(), String> {
        if !self.aktifler()?.iter().any(|c| c.id == id) {
            return Err(format!("aktif telefon yok: {id}"));
        }
        let s = serde_json::to_string(&Kayit::Kapat {
            id: id.into(),
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.yol, &s).map_err(|e| e.to_string())
    }

    pub fn dogrula(&self, token: &str) -> Option<Cihaz> {
        let h = sha256_hex(token);
        self.aktifler()
            .ok()?
            .into_iter()
            .find(|c| esit(&c.token_sha256, &h))
    }
}
