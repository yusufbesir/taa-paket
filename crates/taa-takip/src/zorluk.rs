//! İşlem zorluk puanları (1 kolay – 5 çok zor). Bunlar BAŞLANGIÇ değerleridir; usta/Yusuf
//! veri klasöründeki `zorluk.json` ile değiştirir. Prim ya da ücret formülü DEĞİLDİR, yalnız iş yükü ağırlığıdır.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

const VARSAYILAN: &str = include_str!("zorluk.json");

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Zorluk {
    pub varsayilan_puan: u32,
    pub puanlar: BTreeMap<String, u32>,
}

impl Zorluk {
    pub fn varsayilan() -> Zorluk {
        Self::json_den(VARSAYILAN).expect("gömülü zorluk tablosu geçerli olmalı")
    }

    pub fn json_den(s: &str) -> Result<Zorluk, String> {
        let z: Zorluk =
            serde_json::from_str(s).map_err(|e| format!("zorluk dosyası geçersiz: {e}"))?;
        let gecerli = |p: u32| (1..=5).contains(&p);
        if !gecerli(z.varsayilan_puan) || z.puanlar.values().any(|p| !gecerli(*p)) {
            return Err("zorluk puanları 1 ile 5 arasında olmalı".into());
        }
        Ok(z)
    }

    /// `dizin/zorluk.json` varsa onu, yoksa gömülü tabloyu yükler. Bozuk dosya hata verir (sessizce yutulmaz).
    pub fn dizinden(dizin: &Path) -> Result<Zorluk, String> {
        match std::fs::read_to_string(dizin.join("zorluk.json")) {
            Ok(s) => Self::json_den(&s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::varsayilan()),
            Err(e) => Err(format!("zorluk dosyası okunamadı: {e}")),
        }
    }

    /// İşlem koduna göre puan; kodsuz (serbest) ya da tabloda olmayan işlem varsayılan puanı alır.
    pub fn puan(&self, kod: Option<&str>) -> u32 {
        kod.and_then(|k| self.puanlar.get(k).copied())
            .unwrap_or(self.varsayilan_puan)
    }
}
