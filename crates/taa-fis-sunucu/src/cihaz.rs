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
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "lowercase")]
enum Kayit {
    Cihaz(Cihaz),
    Kapat { id: String, tarih: String },
}

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
            .collect())
    }

    pub fn ekle(&self, ad: &str, token: &str, tarih: &str) -> Result<Cihaz, String> {
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
