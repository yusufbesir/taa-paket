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

pub const MANIFEST: &str = r##"{"name":"TAA Terzi Takip","short_name":"Terzi Takip","lang":"tr","start_url":"/telefon","scope":"/","display":"standalone","background_color":"#f3ede1","theme_color":"#2b2620","icons":[{"src":"/simge.svg","sizes":"any","type":"image/svg+xml","purpose":"any"}]}"##;

fn birlestir(s: &str) -> String {
    s.replace("/*@TEMA@*/", TEMA_CSS)
        .replace("/*@ORTAK@*/", ORTAK_JS)
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
