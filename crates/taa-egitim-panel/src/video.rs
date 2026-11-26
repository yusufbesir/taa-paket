// Sahip: Yusuf. yazan: [Claude#egitim-panel 2026-11-26]
//! Video kaynağı ve Range kararı. Video üretilmez: kaynak ya yerel bir dosyadır (video klasörü altında, yalnız göreli
//! ve güvenli ad) ya da https adresidir. Dosya yoksa durum dürüstçe "dosya-yok" döner. Range kararı
//! `taa_egitim::aralik_coz` ile verilir; tek yanıtta en çok `PARCA_BOYU` bayt gönderilir (açık uçlu `bytes=0-`).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use taa_egitim::{aralik_coz, AralikSonuc, ParcaPlani};

/// Bir 206 yanıtında gönderilecek en çok bayt.
pub const PARCA_BOYU: u64 = 4 * 1024 * 1024;
const URL_EN_COK: usize = 2048;
const YOL_EN_COK: usize = 128;

/// Katalog dosyasında bir dersin video bağı: `{"yol": "kasa/acilis.mp4"}` ya da `{"url": "https://…"}` (yalnız biri).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoKaynagi {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Kaynağın türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KaynakTuru {
    Yerel,
    Url,
}

impl VideoKaynagi {
    pub fn dogrula(&self) -> Result<KaynakTuru, String> {
        match (&self.yol, &self.url) {
            (Some(y), None) => {
                if !yol_gecerli(y) {
                    return Err(format!(
                        "video yolu geçersiz: {y} (göreli, en çok 3 parça, [A-Za-z0-9._-], uzantı mp4/m4v/webm/ogv)"
                    ));
                }
                Ok(KaynakTuru::Yerel)
            }
            (None, Some(u)) => {
                if !url_gecerli(u) {
                    return Err("video adresi geçersiz: https:// ile başlamalı, boşluk ve tırnak içermemeli".into());
                }
                Ok(KaynakTuru::Url)
            }
            _ => Err("video bağında yalnız biri verilmeli: yol ya da url".into()),
        }
    }
}

/// Göreli, güvenli yerel video yolu: `/` ile ayrılmış en çok 3 parça; her parça `[A-Za-z0-9._-]`, nokta ile
/// başlamaz (`..` yok); son parçanın uzantısı desteklenir.
pub fn yol_gecerli(s: &str) -> bool {
    if s.is_empty() || s.len() > YOL_EN_COK {
        return false;
    }
    let parcalar: Vec<&str> = s.split('/').collect();
    if parcalar.len() > 3 {
        return false;
    }
    let parca_ok = |p: &str| {
        !p.is_empty()
            && !p.starts_with('.')
            && p.bytes()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'.' | b'_' | b'-'))
    };
    parcalar.iter().all(|p| parca_ok(p)) && icerik_turu(s).is_some()
}

/// Yalnız https; boşluk, denetim karakteri, tırnak ve `<>` yok.
pub fn url_gecerli(s: &str) -> bool {
    let Some(kalan) = s.strip_prefix("https://") else {
        return false;
    };
    s.len() <= URL_EN_COK
        && kalan
            .split(['/', '?', '#'])
            .next()
            .is_some_and(|host| !host.is_empty() && !host.contains('@'))
        && !s.chars().any(|c| {
            c.is_whitespace() || c.is_control() || matches!(c, '"' | '\'' | '<' | '>' | '\\' | '`')
        })
}

/// Uzantıdan `Content-Type`; desteklenmeyen uzantı None.
pub fn icerik_turu(yol: &str) -> Option<&'static str> {
    let uzanti = yol.rsplit_once('.')?.1.to_ascii_lowercase();
    match uzanti.as_str() {
        "mp4" | "m4v" => Some("video/mp4"),
        "webm" => Some("video/webm"),
        "ogv" => Some("video/ogg"),
        _ => None,
    }
}

/// Bir dersin videosunun durumu (ekranda dürüst boş durum için).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VideoDurumu {
    /// Oynatılabilir.
    Hazir,
    /// Derse video bağlanmamış.
    Yok,
    /// Bağ var ama dosya bulunamadı.
    DosyaYok,
}

impl VideoDurumu {
    pub fn ad(self) -> &'static str {
        match self {
            VideoDurumu::Hazir => "hazir",
            VideoDurumu::Yok => "yok",
            VideoDurumu::DosyaYok => "dosya-yok",
        }
    }
}

/// Yerel dosyanın tam yolu (`kok` + göreli yol). Geçersiz yol None.
pub fn yerel_yol(kok: &Path, yol: &str) -> Option<PathBuf> {
    yol_gecerli(yol).then(|| yol.split('/').fold(kok.to_path_buf(), |p, s| p.join(s)))
}

/// Kaynağın durumu: URL bağı hazır sayılır (erişilebilirliği tarayıcı bildirir); yerel dosya varsa hazır.
pub fn durum(kaynak: Option<&VideoKaynagi>, kok: &Path) -> VideoDurumu {
    let Some(k) = kaynak else {
        return VideoDurumu::Yok;
    };
    match (&k.yol, &k.url) {
        (Some(y), None) => match yerel_yol(kok, y) {
            Some(p) if p.is_file() => VideoDurumu::Hazir,
            _ => VideoDurumu::DosyaYok,
        },
        (None, Some(u)) if url_gecerli(u) => VideoDurumu::Hazir,
        _ => VideoDurumu::Yok,
    }
}

/// Range kararı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Karar {
    /// 200: tüm içerik.
    Tam { toplam: u64 },
    /// 206: `bas..=son`.
    Kismi { bas: u64, son: u64, toplam: u64 },
    /// 416.
    Karsilanamaz { toplam: u64 },
}

impl Karar {
    /// Gövde uzunluğu.
    pub fn uzunluk(&self) -> u64 {
        match self {
            Karar::Tam { toplam } => *toplam,
            Karar::Kismi { bas, son, .. } => son - bas + 1,
            Karar::Karsilanamaz { .. } => 0,
        }
    }

    pub fn durum_kodu(&self) -> u16 {
        match self {
            Karar::Tam { .. } => 200,
            Karar::Kismi { .. } => 206,
            Karar::Karsilanamaz { .. } => 416,
        }
    }

    /// `Content-Range` başlığı (206 ve 416'da).
    pub fn icerik_araligi(&self) -> Option<String> {
        match self {
            Karar::Tam { .. } => None,
            Karar::Kismi { bas, son, toplam } => Some(format!("bytes {bas}-{son}/{toplam}")),
            Karar::Karsilanamaz { toplam } => {
                Some(taa_egitim::aralik::karsilanamaz_basligi(*toplam))
            }
        }
    }
}

/// `Range` başlığı ve dosya boyundan karar. Tek aralık desteklenir; çözülemeyen başlık tam içerik sayılır
/// (RFC 9110). Açık uçlu ya da çok büyük aralık `PARCA_BOYU` ile kısaltılır (206'da kısa yanıt geçerlidir).
pub fn karar(range: Option<&str>, toplam: u64) -> Karar {
    match aralik_coz(range, toplam) {
        AralikSonuc::Tam => Karar::Tam { toplam },
        AralikSonuc::Karsilanamaz => Karar::Karsilanamaz { toplam },
        AralikSonuc::Kismi(a) => {
            let sinir = ParcaPlani::yeni(toplam, PARCA_BOYU)
                .and_then(|p| p.devam(a.bas))
                .map_or(a.son, |p| p.son);
            Karar::Kismi {
                bas: a.bas,
                son: a.son.min(sinir),
                toplam,
            }
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn yol_kurallari() {
        assert!(yol_gecerli("kasa-acilis.mp4"));
        assert!(yol_gecerli("kasa/acilis.webm"));
        assert!(yol_gecerli("a/b/c.OGV"));
        assert!(!yol_gecerli(""));
        assert!(!yol_gecerli("../gizli.mp4"));
        assert!(!yol_gecerli("a/../b.mp4"));
        assert!(!yol_gecerli("/etc/video.mp4"));
        assert!(!yol_gecerli("C:\\video.mp4"));
        assert!(!yol_gecerli("a//b.mp4"));
        assert!(!yol_gecerli("a/b/c/d.mp4"), "en çok 3 parça");
        assert!(!yol_gecerli(".gizli.mp4"));
        assert!(!yol_gecerli("video.exe"));
        assert!(!yol_gecerli("video"));
        assert!(!yol_gecerli("bosluk var.mp4"));
        assert!(!yol_gecerli(&format!("{}.mp4", "a".repeat(130))));
    }

    #[test]
    fn url_kurallari() {
        assert!(url_gecerli("https://ornek.test/v/kasa.mp4?x=1"));
        assert!(!url_gecerli("http://ornek.test/v.mp4"));
        assert!(!url_gecerli("https://"));
        assert!(!url_gecerli("https:///v.mp4"));
        assert!(!url_gecerli("https://k@ornek.test/v.mp4"));
        assert!(!url_gecerli("https://ornek.test/a b.mp4"));
        assert!(!url_gecerli("https://ornek.test/\"x"));
        assert!(!url_gecerli("javascript:alert(1)"));
        assert!(!url_gecerli(&format!(
            "https://o.test/{}",
            "a".repeat(2100)
        )));
    }

    #[test]
    fn kaynak_dogrulama() {
        let yol = |y: &str| VideoKaynagi {
            yol: Some(y.into()),
            url: None,
        };
        assert_eq!(yol("a.mp4").dogrula(), Ok(KaynakTuru::Yerel));
        assert!(yol("../a.mp4").dogrula().is_err());
        let url = VideoKaynagi {
            yol: None,
            url: Some("https://o.test/a.mp4".into()),
        };
        assert_eq!(url.dogrula(), Ok(KaynakTuru::Url));
        let ikisi = VideoKaynagi {
            yol: Some("a.mp4".into()),
            url: Some("https://o.test/a.mp4".into()),
        };
        assert!(ikisi.dogrula().is_err());
        assert!(VideoKaynagi {
            yol: None,
            url: None
        }
        .dogrula()
        .is_err());
        assert!(serde_json::from_str::<VideoKaynagi>(r#"{"yol":"a.mp4","x":1}"#).is_err());
    }

    #[test]
    fn icerik_turleri() {
        assert_eq!(icerik_turu("a.mp4"), Some("video/mp4"));
        assert_eq!(icerik_turu("a.M4V"), Some("video/mp4"));
        assert_eq!(icerik_turu("a.webm"), Some("video/webm"));
        assert_eq!(icerik_turu("a.ogv"), Some("video/ogg"));
        assert_eq!(icerik_turu("a.txt"), None);
        assert_eq!(icerik_turu("a"), None);
    }

    #[test]
    fn durum_dosya_yoksa_dosya_yok() {
        let mut kok = std::env::temp_dir();
        kok.push(format!("taa-egitim-panel-video-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&kok);
        std::fs::create_dir_all(&kok).unwrap();
        let k = VideoKaynagi {
            yol: Some("var.mp4".into()),
            url: None,
        };
        assert_eq!(durum(None, &kok), VideoDurumu::Yok);
        assert_eq!(durum(Some(&k), &kok), VideoDurumu::DosyaYok);
        std::fs::write(kok.join("var.mp4"), b"0123456789").unwrap();
        assert_eq!(durum(Some(&k), &kok), VideoDurumu::Hazir);
        let u = VideoKaynagi {
            yol: None,
            url: Some("https://o.test/a.mp4".into()),
        };
        assert_eq!(durum(Some(&u), &kok), VideoDurumu::Hazir);
        assert_eq!(
            yerel_yol(&kok, "var.mp4").unwrap().file_name().unwrap(),
            "var.mp4"
        );
        assert!(yerel_yol(&kok, "../x.mp4").is_none());
        let _ = std::fs::remove_dir_all(&kok);
        assert_eq!(VideoDurumu::Hazir.ad(), "hazir");
        assert_eq!(VideoDurumu::DosyaYok.ad(), "dosya-yok");
    }

    #[test]
    fn range_karari() {
        assert_eq!(karar(None, 1000), Karar::Tam { toplam: 1000 });
        assert_eq!(
            karar(Some("bytes=0-99"), 1000),
            Karar::Kismi {
                bas: 0,
                son: 99,
                toplam: 1000
            }
        );
        let k = karar(Some("bytes=900-"), 1000);
        assert_eq!(k.durum_kodu(), 206);
        assert_eq!(k.uzunluk(), 100);
        assert_eq!(k.icerik_araligi().as_deref(), Some("bytes 900-999/1000"));
        assert_eq!(karar(Some("bytes=-100"), 1000).uzunluk(), 100);
        let t = karar(Some("bytes=5000-"), 1000);
        assert_eq!(t, Karar::Karsilanamaz { toplam: 1000 });
        assert_eq!(t.durum_kodu(), 416);
        assert_eq!(t.icerik_araligi().as_deref(), Some("bytes */1000"));
        assert_eq!(t.uzunluk(), 0);
        assert_eq!(
            karar(Some("bytes=0-1,5-6"), 1000),
            Karar::Tam { toplam: 1000 }
        );
        assert_eq!(karar(Some("anlamsiz"), 1000), Karar::Tam { toplam: 1000 });
        assert!(Karar::Tam { toplam: 5 }.icerik_araligi().is_none());
        assert_eq!(Karar::Tam { toplam: 5 }.durum_kodu(), 200);
    }

    #[test]
    fn buyuk_dosyada_aralik_parcalanir() {
        let toplam = 100 * 1024 * 1024;
        let k = karar(Some("bytes=0-"), toplam);
        assert_eq!(
            k,
            Karar::Kismi {
                bas: 0,
                son: PARCA_BOYU - 1,
                toplam
            }
        );
        assert_eq!(k.uzunluk(), PARCA_BOYU);
        // ortadan istek: parça sınırı bas'tan itibaren sayılır
        let k = karar(Some("bytes=1048576-"), toplam);
        assert_eq!(k.uzunluk(), PARCA_BOYU);
        // küçük istek aynen karşılanır
        assert_eq!(karar(Some("bytes=10-19"), toplam).uzunluk(), 10);
        // boş dosya
        assert_eq!(karar(None, 0), Karar::Tam { toplam: 0 });
        assert_eq!(karar(Some("bytes=0-"), 0).durum_kodu(), 416);
    }
}
