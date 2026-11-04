//! taa-pano: şube ekranı için SALT OKUNUR pano. Bir klasördeki `*.pano.json` dosyalarından kart okur
//! (`{"baslik":"Etiket kuyruğu","deger":"3","durum":"tamam|uyari|hata"}`); kartları üreten programlar kendi dosyasını yazar.
//! Stok bu panoya girmez (stok kaynağı Nebim). Özellik anahtarı: "pano" (varsayılan açık).
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use serde::Deserialize;
use std::fs;
use std::path::Path;

pub const OZELLIK: &str = "pano";

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Kart {
    pub baslik: String,
    pub deger: String,
    #[serde(default = "tamam")]
    pub durum: String,
}

fn tamam() -> String {
    "tamam".into()
}

#[derive(Debug, Default)]
pub struct Pano {
    pub kartlar: Vec<Kart>,
    /// Okunamayan dosyalar (ad, neden).
    pub sorunlar: Vec<(String, String)>,
}

pub fn klasorden(klasor: &Path) -> Result<Pano, String> {
    if !taa_ayar::ozellik_acik(OZELLIK, true) {
        return Err(taa_ayar::kapali_mesaji(OZELLIK));
    }
    let mut dosyalar: Vec<_> = fs::read_dir(klasor)
        .map_err(|e| format!("klasör okunamadı: {e}"))?
        .flatten()
        .map(|o| o.path())
        .filter(|p| {
            p.file_name()
                .map(|n| n.to_string_lossy().ends_with(".pano.json"))
                .unwrap_or(false)
        })
        .collect();
    dosyalar.sort();
    let mut pano = Pano::default();
    for d in dosyalar {
        let ad = d.file_name().unwrap().to_string_lossy().to_string();
        match fs::read(&d)
            .map_err(|e| e.to_string())
            .and_then(|b| serde_json::from_slice::<Kart>(&b).map_err(|e| e.to_string()))
        {
            Ok(k) => pano.kartlar.push(k),
            Err(e) => pano.sorunlar.push((ad, e)),
        }
    }
    Ok(pano)
}

impl Pano {
    pub fn metin(&self) -> String {
        let mut s = String::new();
        for k in &self.kartlar {
            let i = match k.durum.as_str() {
                "hata" => "[!]",
                "uyari" => "[~]",
                _ => "[ok]",
            };
            s.push_str(&format!("{i} {}: {}\n", k.baslik, k.deger));
        }
        s
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn okur_ve_sorun_bildirir() {
        let d = std::env::temp_dir().join(format!("taa-pano-test-{}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        fs::write(
            d.join("a.pano.json"),
            r#"{"baslik":"Etiket kuyruğu","deger":"3"}"#,
        )
        .unwrap();
        fs::write(
            d.join("b.pano.json"),
            r#"{"baslik":"Yazıcı","deger":"çevrimdışı","durum":"hata"}"#,
        )
        .unwrap();
        fs::write(d.join("c.pano.json"), "bozuk").unwrap();
        fs::write(d.join("diger.txt"), "x").unwrap();
        let p = klasorden(&d).unwrap();
        assert_eq!(p.kartlar.len(), 2);
        assert_eq!(p.sorunlar.len(), 1);
        assert_eq!(
            p.metin(),
            "[ok] Etiket kuyruğu: 3\n[!] Yazıcı: çevrimdışı\n"
        );
        let _ = fs::remove_dir_all(&d);
    }
}
