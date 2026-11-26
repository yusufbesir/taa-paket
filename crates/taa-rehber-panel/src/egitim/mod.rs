// Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]
//! TAA Rehber'in "Eğitim" bölümü (eskiden ayrı `taa-egitim-panel` programıydı; Yusuf 11-26: Eğitim ve Rehber tek program).
//!
//! Eğitim videoları, oynatıcı (Range), atamalarım, yönetici izleme özeti ve video atama. Uçlar `/api/egitim/...`
//! önekiyle, video `/video/<ders>` ile sunulur. Kimlik `taa-kimlik` oturumundan gelir; yönetici işlemleri imzası
//! doğrulanmış yetki belgesi olmadan açılmaz (varsayılan reddet). Çekirdek mantık `taa-egitim` crate'indedir.

pub mod api;
pub mod gorunum;
pub mod kimlik;
pub mod veri;
pub mod video;
pub mod zaman;

#[cfg(test)]
mod testler;

pub use api::{Panel, VideoYaniti};
pub use kimlik::{kullanici_olustur, Kullanici, YetkiBilgisi, YONETICI_IZNI};
