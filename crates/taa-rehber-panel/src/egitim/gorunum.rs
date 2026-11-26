// Sahip: Yusuf. yazan: [Claude#egitim-panel 2026-11-26]
//! Görünüm tercihi (tema). Kapı her açılışta rastgele olduğundan tarayıcı deposu (köken başına) kalıcı olmaz;
//! tercih veri kökünde `egitim/gorunum.json` içinde saklanır, sunucuya gitmez. Tema ekrana açılıştan itibaren uygulanır.

use serde_json::{json, Value};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tema {
    #[default]
    Acik,
    Koyu,
    TekRenkli,
}

impl Tema {
    /// Ekrandaki `data-theme` değeri (ve dosyadaki yazı).
    pub fn ad(self) -> &'static str {
        match self {
            Tema::Acik => "acik",
            Tema::Koyu => "koyu",
            Tema::TekRenkli => "tek",
        }
    }

    pub fn adindan(s: &str) -> Option<Tema> {
        match s {
            "acik" => Some(Tema::Acik),
            "koyu" => Some(Tema::Koyu),
            "tek" => Some(Tema::TekRenkli),
            _ => None,
        }
    }

    pub fn ozet(self) -> Value {
        json!({ "tema": self.ad() })
    }
}

/// Dosyadan okur; yok, bozuk ya da bilinmeyen değer varsayılana (açık) döner.
pub fn oku(yol: &Path) -> Tema {
    std::fs::read_to_string(yol)
        .ok()
        .and_then(|m| serde_json::from_str::<Value>(&m).ok())
        .and_then(|v| {
            v.get("tema")
                .and_then(Value::as_str)
                .and_then(Tema::adindan)
        })
        .unwrap_or_default()
}

/// Yazar (klasör yoksa oluşturur). Hata metni kullanıcıya gösterilebilir.
pub fn yaz(yol: &Path, t: Tema) -> Result<(), String> {
    if let Some(k) = yol.parent() {
        std::fs::create_dir_all(k).map_err(|e| format!("klasör oluşturulamadı: {e}"))?;
    }
    std::fs::write(yol, t.ozet().to_string()).map_err(|e| format!("yazılamadı: {e}"))
}

#[cfg(test)]
mod testler {
    use super::*;

    fn gecici(ad: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "taa-egitim-panel-gorunum-{}-{ad}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&p);
        p.join("egitim").join("gorunum.json")
    }

    #[test]
    fn adlar_gidis_donus() {
        for t in [Tema::Acik, Tema::Koyu, Tema::TekRenkli] {
            assert_eq!(Tema::adindan(t.ad()), Some(t));
        }
        assert_eq!(Tema::adindan("mor"), None);
        assert_eq!(Tema::adindan(""), None);
    }

    #[test]
    fn yaz_oku_ve_varsayilan() {
        let y = gecici("yazoku");
        assert_eq!(oku(&y), Tema::Acik, "dosya yokken açık");
        yaz(&y, Tema::Koyu).unwrap();
        assert_eq!(oku(&y), Tema::Koyu);
        std::fs::write(&y, "{bozuk").unwrap();
        assert_eq!(oku(&y), Tema::Acik);
        std::fs::write(&y, r#"{"tema":"mor"}"#).unwrap();
        assert_eq!(oku(&y), Tema::Acik);
        let _ = std::fs::remove_dir_all(y.parent().unwrap().parent().unwrap());
    }
}
