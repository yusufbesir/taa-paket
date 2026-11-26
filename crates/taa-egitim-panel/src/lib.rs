// Sahip: Yusuf. yazan: [Claude#egitim-panel 2026-11-26]
//! taa-egitim-panel: TAA Eğitim'in yerel grafik paneli (bin `taa-egitim-panel`).
//!
//! 127.0.0.1'de rastgele kapıda yerel bir sunucu açılır, Edge uygulama penceresi (`--app`) olarak başlatılır; pencere
//! kapanıp istek gelmeyince panel kendiliğinden kapanır. Ekran tek dosyadır (dış kaynak yok). Tüm iş mantığı saf
//! işlevlerdir ve testlidir; ağ, pencere ve oturum bağı yalnız `panel_main.rs` içindedir.
//!
//! Ekranlar: eğitim video listesi (konu ve görev süzgeci), oynatıcı (HTML5 video, Range, kaldığı yerden devam,
//! "izledim"), atamalarım, yöneticiye özel izleme özeti ve video atama.
//!
//! Güvenlik özeti (ayrıntı `BENIOKU.md`): yalnız 127.0.0.1, Host/çerez/köken denetimi, sıkı CSP; kimlik `taa-kimlik`
//! oturumundan gelir, yönetici işlemleri varsayılan REDDET (imzası doğrulanmış yetki belgesi + `kimlik.yonetici` izni
//! yoksa 403). Sahte ya da kodlanmış giriş yoktur.

pub mod api;
pub mod gorunum;
pub mod guvenlik;
pub mod kimlik;
pub mod veri;
pub mod video;
pub mod zaman;

#[cfg(test)]
mod testler;

pub use api::{Panel, Yanit};
pub use kimlik::{kullanici_olustur, Kullanici, YetkiBilgisi, YONETICI_IZNI};

/// Gömülü ekran (tek dosya, dış kaynak yok). `__TEMA__` yer tutucusu `arayuz_html` ile doldurulur.
pub const ARAYUZ_HTML: &str = include_str!("arayuz.html");
/// Uygulama simgesi (mezura).
pub const SIMGE_SVG: &str = include_str!("simge.svg");

/// Ekran metnini kayıtlı temayla verir. Tema kapalı bir listeden gelir; metne başka bir şey girmez.
pub fn arayuz_html(tema: gorunum::Tema) -> String {
    ARAYUZ_HTML.replace("__TEMA__", tema.ad())
}

pub const OZELLIK: &str = "egitim-panel";
