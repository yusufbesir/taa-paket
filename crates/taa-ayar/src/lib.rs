//! taa-ayar: özellik anahtarları. Sıra: ortam değişkeni `TAA_OZELLIK_<AD>` (0/1/acik/kapali),
//! sonra `TAA_AYAR` dosyası (`{"ozellikler":{"ad":true}}`), sonra varsayılan.
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use std::path::Path;

fn deger_coz(s: &str) -> Option<bool> {
    match s.trim().to_lowercase().as_str() {
        "1" | "true" | "acik" | "açık" | "evet" => Some(true),
        "0" | "false" | "kapali" | "kapalı" | "hayir" | "hayır" => Some(false),
        _ => None,
    }
}

fn ortam_adi(ad: &str) -> String {
    let b: String = ad
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_uppercase() } else { '_' })
        .collect();
    format!("TAA_OZELLIK_{b}")
}

/// Ayar JSON metninden özellik değerini okur (saf işlev; test için).
pub fn json_den(json: &str, ad: &str) -> Option<bool> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    v.get("ozellikler")?.get(ad)?.as_bool()
}

/// Özellik açık mı? Hata durumunda varsayılana düşer.
pub fn ozellik_acik(ad: &str, varsayilan: bool) -> bool {
    if let Ok(v) = std::env::var(ortam_adi(ad)) {
        if let Some(b) = deger_coz(&v) {
            return b;
        }
    }
    if let Ok(yol) = std::env::var("TAA_AYAR") {
        if let Ok(metin) = std::fs::read_to_string(Path::new(&yol)) {
            if let Some(b) = json_den(&metin, ad) {
                return b;
            }
        }
    }
    varsayilan
}

/// Kapalı özellik için ortak Türkçe hata metni.
pub fn kapali_mesaji(ad: &str) -> String {
    format!("'{ad}' özelliği ayarlardan kapatılmış")
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn json_okur() {
        let j = r#"{"ozellikler":{"nebim-alici":false,"yedek":true}}"#;
        assert_eq!(json_den(j, "nebim-alici"), Some(false));
        assert_eq!(json_den(j, "yedek"), Some(true));
        assert_eq!(json_den(j, "yok"), None);
        assert_eq!(json_den("bozuk", "yedek"), None);
    }

    #[test]
    fn varsayilan_ve_ortam() {
        assert!(ozellik_acik("deneme-ayar-yok-1", true));
        assert!(!ozellik_acik("deneme-ayar-yok-2", false));
        std::env::set_var("TAA_OZELLIK_DENEME_ORTAM", "0");
        assert!(!ozellik_acik("deneme-ortam", true));
        std::env::set_var("TAA_OZELLIK_DENEME_ORTAM", "açık");
        assert!(ozellik_acik("deneme-ortam", false));
    }

    #[test]
    fn ortam_adi_donusur() {
        assert_eq!(ortam_adi("nebim-alici"), "TAA_OZELLIK_NEBIM_ALICI");
    }
}
