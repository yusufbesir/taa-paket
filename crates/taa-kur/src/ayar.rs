// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! `<kök>/kur.toml` okuyucusu (T3): `kaynak_klasoru` (yerel güncelleme kaynağı) ve `kanal`. Dosya yoksa varsayılanlar.
//! Bozuk TOML tek cümlelik Türkçe hata verir. Bilinmeyen anahtarlar yok sayılır (ileriye uyumluluk).

use crate::paket::ad_gecerli;
use crate::KurHata;
use serde::Deserialize;
use std::fs;
use std::path::Path;

pub const VARSAYILAN_KANAL: &str = "kararli";
const MAKS_BAYT: u64 = 64 * 1024;

#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
pub struct KurAyar {
    /// Yerel güncelleme kaynağı klasörü (içinde `yayin.json`).
    #[serde(default)]
    pub kaynak_klasoru: Option<String>,
    /// Yayın kanalı; boşsa [`VARSAYILAN_KANAL`].
    #[serde(default)]
    pub kanal: Option<String>,
}

impl KurAyar {
    pub fn kanal(&self) -> &str {
        self.kanal.as_deref().unwrap_or(VARSAYILAN_KANAL)
    }
}

/// `kok/kur.toml` dosyasını okur; yoksa varsayılan döner.
pub fn kur_ayari_oku(kok: &Path) -> Result<KurAyar, KurHata> {
    let yol = kok.join("kur.toml");
    let hata = |m: String| KurHata::Dogrulama(format!("{} {m}", yol.display()));
    let meta = match fs::metadata(&yol) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(KurAyar::default()),
        Err(e) => {
            return Err(hata(format!(
                "okunamadı ({e}); dosya izinlerini denetleyin."
            )))
        }
    };
    if !meta.is_file() || meta.len() > MAKS_BAYT {
        return Err(hata(
            "normal bir dosya değil ya da çok büyük; kur.toml'u yeniden yazın.".into(),
        ));
    }
    let metin = fs::read_to_string(&yol)
        .map_err(|e| hata(format!("okunamadı ({e}); dosya UTF-8 olmalı.")))?;
    let a: KurAyar = toml::from_str(&metin).map_err(|e| {
        let satir = e
            .span()
            .map(|s| metin[..s.start.min(metin.len())].matches('\n').count() + 1);
        hata(match satir {
            Some(n) => format!("bozuk TOML (satır {n}); kur.toml'u düzeltin ya da silmek yerine eski/ altına taşıyın."),
            None => "bozuk TOML; kur.toml'u düzeltin ya da silmek yerine eski/ altına taşıyın.".into(),
        })
    })?;
    if let Some(k) = &a.kanal {
        if !ad_gecerli(k) {
            return Err(hata(
                "kanal küçük harf, rakam ve '-' içermeli (örnek: kararli); kur.toml'daki kanal değerini düzeltin.".into(),
            ));
        }
    }
    if a.kaynak_klasoru
        .as_deref()
        .is_some_and(|k| k.trim().is_empty())
    {
        return Err(hata(
            "kaynak_klasoru boş olamaz; yol yazın ya da satırı kaldırın.".into(),
        ));
    }
    Ok(a)
}

#[cfg(test)]
mod testler {
    use super::*;

    fn kok_ile(icerik: Option<&str>) -> std::path::PathBuf {
        let k = taa_dosya::gecici_klasor().unwrap();
        if let Some(i) = icerik {
            fs::write(k.join("kur.toml"), i).unwrap();
        }
        k
    }

    #[test]
    fn dosya_yoksa_varsayilan() {
        let a = kur_ayari_oku(&kok_ile(None)).unwrap();
        assert_eq!(a, KurAyar::default());
        assert_eq!(a.kanal(), "kararli");
    }

    #[test]
    fn gecerli_dosya_okunur() {
        let a = kur_ayari_oku(&kok_ile(Some(
            "# yorum\nkaynak_klasoru = \"D:/Kaynak\"\nkanal = \"deneme\"\nbilinmeyen = 1\n",
        )))
        .unwrap();
        assert_eq!(a.kaynak_klasoru.as_deref(), Some("D:/Kaynak"));
        assert_eq!(a.kanal(), "deneme");
    }

    #[test]
    fn bozuk_toml_tek_cumle_hata() {
        let e = kur_ayari_oku(&kok_ile(Some("kanal = \n"))).unwrap_err();
        assert!(matches!(e, KurHata::Dogrulama(_)));
        assert!(e.mesaj().contains("bozuk TOML (satır 1)"), "{e}");
        assert!(!e.mesaj().contains('\n'));
    }

    #[test]
    fn yanlis_tur_ve_gecersiz_kanal_reddedilir() {
        assert!(kur_ayari_oku(&kok_ile(Some("kanal = 5\n"))).is_err());
        assert!(kur_ayari_oku(&kok_ile(Some("kanal = \"../x\"\n"))).is_err());
        assert!(kur_ayari_oku(&kok_ile(Some("kaynak_klasoru = \"  \"\n"))).is_err());
    }
}
