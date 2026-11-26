//! Görünüm tercihi (tema): Açık, Koyu, Tek renkli. Cihazda hatırlanır: `<veri kökü>/rehber-panel/gorunum.json`.
//! Sunucuya gitmez. Pencere her açılışta başka kapıda açıldığı için tarayıcı depolaması kalıcı olmaz; bu yüzden
//! tercih yerel dosyada tutulur ve açılışta HTML'e yazılır (ilk boyamada doğru tema).
//! Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tema {
    #[default]
    Acik,
    Koyu,
    Tek,
}

impl Tema {
    pub fn kod(self) -> &'static str {
        match self {
            Tema::Acik => "acik",
            Tema::Koyu => "koyu",
            Tema::Tek => "tek",
        }
    }

    pub fn koddan(k: &str) -> Option<Tema> {
        match k {
            "acik" => Some(Tema::Acik),
            "koyu" => Some(Tema::Koyu),
            "tek" => Some(Tema::Tek),
            _ => None,
        }
    }
}

pub fn gorunum_yolu(kok: &Path) -> PathBuf {
    kok.join("rehber-panel").join("gorunum.json")
}

/// Dosya yok ya da bozuksa varsayılan (Açık).
pub fn yukle(yol: &Path) -> Tema {
    std::fs::read_to_string(yol)
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            v.get("tema")
                .and_then(|t| t.as_str())
                .and_then(Tema::koddan)
        })
        .unwrap_or_default()
}

/// Geçici dosyaya yazıp yeniden adlandırır (yarım dosya kalmaz).
pub fn kaydet(yol: &Path, t: Tema) -> std::io::Result<()> {
    if let Some(d) = yol.parent() {
        std::fs::create_dir_all(d)?;
    }
    let gecici = yol.with_extension("json.tmp");
    std::fs::write(&gecici, format!("{{\"tema\":\"{}\"}}", t.kod()))?;
    std::fs::rename(&gecici, yol)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kod_gidis_donus_ve_kalicilik() {
        for t in [Tema::Acik, Tema::Koyu, Tema::Tek] {
            assert_eq!(Tema::koddan(t.kod()), Some(t));
        }
        assert_eq!(Tema::koddan("mor"), None);
        let d = taa_dosya::gecici_klasor().unwrap();
        let y = gorunum_yolu(&d);
        assert_eq!(yukle(&y), Tema::Acik, "dosya yokken varsayılan");
        kaydet(&y, Tema::Koyu).unwrap();
        assert_eq!(yukle(&y), Tema::Koyu);
        std::fs::write(&y, "bozuk").unwrap();
        assert_eq!(yukle(&y), Tema::Acik, "bozuk dosyada varsayılan");
        let _ = std::fs::remove_dir_all(&d);
    }
}
