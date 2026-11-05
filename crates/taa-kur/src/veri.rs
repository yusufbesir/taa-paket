// Sahip: Yusuf. yazan: [Claude#taa-paket-T2 2026-11-05]
//! `tani` için veri alanı taraması (T2-TASARIM §1). `SAHIP.toml` düz TOML alt kümesi küçük yerel bir çözücüyle
//! okunur (taa-veri ayrı PR'dır; ona bağımlılık yok, `toml` crate'i yok). Bu modül veri alanlarına YAZMAZ.

use crate::kayit::Kayit;
use crate::{Sorun, VeriAlani};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const MAKS_SAHIP_BAYT: u64 = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Deger {
    Metin(String),
    Mantik(bool),
    Sayi(i64),
}

/// Veri kökü: `TAA_VERI_KOK` doluysa o; yoksa Windows `%APPDATA%\TAA\veri`, diğerlerinde
/// `$XDG_DATA_HOME/TAA/veri` ya da `~/.local/share/TAA/veri`. Çözülemezse `None`.
pub fn veri_kok_coz(
    ortam: Option<&str>,
    appdata: Option<&str>,
    xdg: Option<&str>,
    home: Option<&str>,
) -> Option<PathBuf> {
    let dolu = |o: Option<&str>| o.filter(|s| !s.trim().is_empty()).map(PathBuf::from);
    if let Some(k) = dolu(ortam) {
        return Some(k);
    }
    if cfg!(windows) {
        return dolu(appdata).map(|a| a.join("TAA").join("veri"));
    }
    if let Some(x) = dolu(xdg) {
        return Some(x.join("TAA").join("veri"));
    }
    dolu(home).map(|h| h.join(".local/share/TAA/veri"))
}

pub fn veri_kok_ortamdan() -> Option<PathBuf> {
    let ev = |a: &str| std::env::var(a).ok();
    veri_kok_coz(
        ev("TAA_VERI_KOK").as_deref(),
        ev("APPDATA").as_deref(),
        ev("XDG_DATA_HOME").as_deref(),
        ev("HOME").as_deref(),
    )
}

/// Düz alt küme: `anahtar = "metin" | true | false | 123`; `#` yorum, boş satır serbest. Tablo, dizi, çok satır yok.
/// Hata: (satır numarası, açıklama).
pub fn sahip_toml_coz(metin: &str) -> Result<BTreeMap<String, Deger>, (usize, String)> {
    let mut m = BTreeMap::new();
    for (i, ham) in metin.lines().enumerate() {
        let no = i + 1;
        let satir = ham.trim();
        if satir.is_empty() || satir.starts_with('#') {
            continue;
        }
        let Some((a, d)) = satir.split_once('=') else {
            return Err((no, "'anahtar = değer' biçimi bekleniyordu".into()));
        };
        let (a, d) = (a.trim(), d.trim());
        if a.is_empty() || !a.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'_') {
            return Err((no, format!("geçersiz anahtar: {a:?}")));
        }
        let deger = if let Some(i) = d.strip_prefix('"') {
            let Some(son) = i.find('"') else {
                return Err((no, "kapanmamış metin".into()));
            };
            let artik = i[son + 1..].trim();
            if !(artik.is_empty() || artik.starts_with('#')) {
                return Err((no, "metinden sonra beklenmeyen içerik".into()));
            }
            Deger::Metin(i[..son].to_string())
        } else {
            let d = d.split('#').next().unwrap_or("").trim();
            match d {
                "true" => Deger::Mantik(true),
                "false" => Deger::Mantik(false),
                _ => Deger::Sayi(
                    d.parse::<i64>()
                        .map_err(|_| (no, format!("tanınmayan değer: {d:?}")))?,
                ),
            }
        };
        if m.insert(a.to_string(), deger).is_some() {
            return Err((no, format!("anahtar iki kez verilmiş: {a}")));
        }
    }
    Ok(m)
}

fn alan_adi_gecerli(a: &str) -> bool {
    !a.is_empty()
        && a.len() <= 32
        && a.bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-')
}

/// Veri kökündeki her `<alan>/SAHIP.toml` için özet üretir (alan adına göre sıralı). Okunamayan ya da bozuk
/// SAHIP.toml tanı sorunu olarak döner; tarama sürer. Kök yoksa boş liste (sorun değil).
pub fn alanlari_tara(kok: &Path, kayit: &Kayit) -> (Vec<VeriAlani>, Vec<Sorun>) {
    let mut alanlar = Vec::new();
    let mut sorunlar = Vec::new();
    let Ok(rd) = fs::read_dir(kok) else {
        return (alanlar, sorunlar);
    };
    let mut adlar: Vec<String> = rd
        .flatten()
        .filter(|o| o.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|o| o.file_name().into_string().ok())
        .filter(|a| alan_adi_gecerli(a))
        .collect();
    adlar.sort();
    for alan in adlar {
        let yol = kok.join(&alan).join("SAHIP.toml");
        let Ok(meta) = fs::symlink_metadata(&yol) else {
            continue; // SAHIP.toml yok: alan henüz sahipsiz
        };
        if !meta.is_file() || meta.len() > MAKS_SAHIP_BAYT {
            sorunlar.push(Sorun::Bozuk(format!(
                "veri alanı {alan}: SAHIP.toml normal dosya değil ya da çok büyük"
            )));
            continue;
        }
        let metin = match fs::read_to_string(&yol) {
            Ok(m) => m,
            Err(e) => {
                sorunlar.push(Sorun::Bozuk(format!(
                    "veri alanı {alan}: SAHIP.toml okunamadı: {e}"
                )));
                continue;
            }
        };
        match sahip_toml_coz(&metin) {
            Ok(t) => {
                let sahip = match t.get("sahip") {
                    Some(Deger::Metin(s)) if !s.is_empty() => s.clone(),
                    _ => {
                        sorunlar.push(Sorun::Bozuk(format!(
                            "veri alanı {alan}: SAHIP.toml içinde sahip yok"
                        )));
                        continue;
                    }
                };
                let gecici = matches!(t.get("gecici"), Some(Deger::Mantik(true)));
                let sahip_kurulu = kayit.paketler.contains_key(&sahip);
                alanlar.push(VeriAlani {
                    alan,
                    sahip,
                    gecici,
                    sahip_kurulu,
                });
            }
            Err((no, m)) => sorunlar.push(Sorun::Bozuk(format!(
                "veri alanı {alan}: SAHIP.toml satır {no}: {m}"
            ))),
        }
    }
    (alanlar, sorunlar)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn toml_alt_kumesi() {
        let m = sahip_toml_coz(
            "# yorum\nsema = 1\nalan = \"etiket-kuyrugu\" # not\ngecici = false\nson_yazim = \"2026-11-05T14:00:00Z\"\n",
        )
        .unwrap();
        assert_eq!(m["sema"], Deger::Sayi(1));
        assert_eq!(m["alan"], Deger::Metin("etiket-kuyrugu".into()));
        assert_eq!(m["gecici"], Deger::Mantik(false));
        assert_eq!(
            sahip_toml_coz("a = 1\nb\n").unwrap_err().0,
            2,
            "satır numarası"
        );
        assert!(sahip_toml_coz("[tablo]").is_err());
        assert!(sahip_toml_coz("a = \"acik").is_err());
        assert!(sahip_toml_coz("a = 1\na = 2").is_err());
    }

    #[test]
    fn kok_cozumu() {
        assert_eq!(
            veri_kok_coz(Some("/x"), None, None, Some("/h")),
            Some(PathBuf::from("/x"))
        );
        if !cfg!(windows) {
            assert_eq!(
                veri_kok_coz(None, None, Some("/xdg"), Some("/h")),
                Some(PathBuf::from("/xdg/TAA/veri"))
            );
            assert_eq!(
                veri_kok_coz(Some(" "), None, None, Some("/h")),
                Some(PathBuf::from("/h/.local/share/TAA/veri"))
            );
            assert_eq!(veri_kok_coz(None, None, None, None), None);
        }
    }
}
