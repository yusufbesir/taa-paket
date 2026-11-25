//! Gömülü sayfalar: masaüstü program (`/masaustu`) ve telefon sitesi (`/telefon`).
//! Kaynaklar `src/sayfa/` altındadır ve derlemede gömülür (dış kaynak, yazı tipi, CDN yok).
//! Tema jetonları (`tema.css`) ve tema/vurgu betiği (`ortak.js`) iki sayfada ortaktır; istek anında
//! sayfadaki yer tutuculara yerleştirilir.

use std::sync::OnceLock;

/// Altı temanın jetonları (masaüstü ve telefon ortak).
pub const TEMA_CSS: &str = include_str!("sayfa/tema.css");
/// Tema, ton, vurgu ve yazı boyutu (localStorage `taa:terzitakip:*`), kontrast denetimi, Yaka logosu.
pub const ORTAK_JS: &str = include_str!("sayfa/ortak.js");
const MASAUSTU_HTML: &str = include_str!("sayfa/masaustu.html");
const TELEFON_HTML: &str = include_str!("sayfa/telefon.html");
/// Uygulama simgesi (Yaka), PWA bildirgesi ve sekme simgesi.
pub const SIMGE: &str = include_str!("sayfa/simge.svg");

/// Yaka simgesi, çok boyutlu .ico (16, 20, 24, 32, 40, 48, 64, 96, 128, 256 px) ve PWA PNG'leri (SVG'den üretildi).
pub const SIMGE_ICO: &[u8] = include_bytes!("sayfa/simge.ico");
pub const SIMGE_192: &[u8] = include_bytes!("sayfa/simge-192.png");
pub const SIMGE_512: &[u8] = include_bytes!("sayfa/simge-512.png");

/// Masaüstü program bildirgesi: Edge/Chrome "Uygulamayı yükle" ile kurulunca /masaustu açılır; başlık çubuğu
/// window-controls-overlay ile sayfanın üst bandıyla birleşir ve tema rengini alır.
pub const MANIFEST_MASAUSTU: &str = r##"{"id":"/masaustu","name":"TAA Terzi Takip","short_name":"Terzi Takip","lang":"tr","start_url":"/masaustu","scope":"/masaustu","display":"standalone","display_override":["window-controls-overlay","standalone"],"background_color":"#FFFFFF","theme_color":"#F1F1EE","icons":[{"src":"/simge.svg","sizes":"any","type":"image/svg+xml","purpose":"any"},{"src":"/simge-192.png","sizes":"192x192","type":"image/png","purpose":"any"},{"src":"/simge-512.png","sizes":"512x512","type":"image/png","purpose":"any"}]}"##;

pub const MANIFEST: &str =r##"{"name":"TAA Terzi Takip","short_name":"Terzi Takip","lang":"tr","start_url":"/telefon","scope":"/","display":"standalone","display_override":["window-controls-overlay","standalone"],"background_color":"#FFFFFF","theme_color":"#F1F1EE","icons":[{"src":"/simge.svg","sizes":"any","type":"image/svg+xml","purpose":"any"},{"src":"/simge-192.png","sizes":"192x192","type":"image/png","purpose":"any"},{"src":"/simge-512.png","sizes":"512x512","type":"image/png","purpose":"any"}]}"##;

/// TAA ortak giriş ekranı: çalışma alanındaki tek kaynaktan derlemede gömülür (kopya yok, kayma olmaz).
pub const GIRIS_EKRANI_JS: &str = include_str!("../../../araclar/js/taa-giris-ekrani.js");
/// Masaüstü açılışında ortak giriş (window.taaGiris yerel /api/giris/* uçlarına gider).
pub const GIRIS_BASLAT_JS: &str = include_str!("sayfa/taa-giris-baslat.js");

fn birlestir(s: &str) -> String {
    s.replace("/*@TEMA@*/", TEMA_CSS)
        .replace("/*@ORTAK@*/", ORTAK_JS)
        .replace("/*@GIRIS-EKRANI@*/", GIRIS_EKRANI_JS)
        .replace("/*@GIRIS-BASLAT@*/", GIRIS_BASLAT_JS)
}

pub fn masaustu() -> String {
    static S: OnceLock<String> = OnceLock::new();
    S.get_or_init(|| birlestir(MASAUSTU_HTML)).clone()
}

pub fn telefon() -> String {
    static S: OnceLock<String> = OnceLock::new();
    S.get_or_init(|| birlestir(TELEFON_HTML)).clone()
}

/// HTML kaçışı (sunucuda üretilen küçük sayfalar için).
pub fn kacis(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn sayfalar_ortak_jetonu_ve_alti_temayi_tasir() {
        for s in [masaustu(), telefon()] {
            assert!(!s.contains("/*@TEMA@*/") && !s.contains("/*@ORTAK@*/"));
            for t in ["kocan", "tebesir", "aski", "cozgu", "civit", "keten"] {
                assert!(s.contains(&format!("html[data-tema=\"{t}\"]")), "{t}");
            }
            assert!(s.contains("taa:terzitakip:"));
            assert!(s.contains("prefers-reduced-motion"));
        }
        // Telefon: eşleştirme belirteci ve çevrimdışı kuyruk anahtarları korunur.
        let t = telefon();
        assert!(t.contains("taa_tok") && t.contains("taa_kuyruk"));
        assert!(t.contains("BarcodeDetector"));
        // Çifte kod: telefon kod2'yi gösterip yoklar; bilgisayar Telefonlar ekranında onaylar.
        assert!(t.contains("'/api/eslestir/'") && t.contains("kod2") && t.contains("410"));
        let m = masaustu();
        assert!(m.contains("/api/yonetim/eslestir/onayla") && m.contains("esles-kod2"));
        // Ortak giriş yalnız masaüstünde: ekran + başlatıcı gömülü, yer tutucu kalmaz; telefon sayfasında yok.
        assert!(
            m.contains("TaaGirisEkrani") && m.contains("'/api/giris/'") && !m.contains("@GIRIS-")
        );
        assert!(!t.contains("TaaGirisEkrani"));
    }

    #[test]
    fn dis_kaynak_yok() {
        for s in [masaustu(), telefon()] {
            for k in [
                "<script src",
                "<link rel=\"stylesheet\"",
                "@import",
                "fonts.googleapis",
                "cdn.",
            ] {
                assert!(!s.contains(k), "dış kaynak: {k}");
            }
            // Yalnız SVG ad alanı ve yerel adresler; başka http(s) bağlantısı yok.
            let dis = s
                .match_indices("http")
                .filter(|(i, _)| {
                    let r = &s[*i..(*i + 40).min(s.len())];
                    !r.starts_with("http://www.w3.org/")
                })
                .count();
            assert_eq!(dis, 0, "sayfada dış adres var");
        }
    }

    #[test]
    fn duzenleyen_fise_basilmaz() {
        let m = masaustu();
        let i = m.find("function fis80(").unwrap();
        let govde = &m[i..i + m[i..].find("function onizle(").unwrap()];
        assert!(!govde.contains("duzenleyen") && !govde.contains("Düzenleyen"));
    }
}
