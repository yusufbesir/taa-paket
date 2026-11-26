//! taa-rehber-panel: TAA Rehber masaüstü paneli (program: taarehber).
//!
//! 127.0.0.1'de rastgele kapıda yerel bir sunucu açar, Edge'i uygulama penceresi olarak (`--app`) başlatır; pencere
//! kapanıp istek gelmeyince kendiliğinden kapanır (taa-cati panel kalıbı). Ekranlar: Kişiler ve Görevler, El Kitabı
//! (`rehber.belge@1`), İhbar Hattı (`taa-ihbar`; gönderim bağlı değil, dürüstçe bildirilir) ve yönetici ekleme formları.
//!
//! Güvenlik ilkeleri:
//! - Görünürlük (şube kapsamı, docs/rehber-sozlesmesi.md §5) sunucuda `taa-rehber-model` ile uygulanır; istemciye güvenilmez.
//! - Yönetici işlemleri varsayılan REDDET: `taa-kimlik` ile doğrulanmış ve `rehber.yaz` izinli oturum yoksa 403.
//!   Sahte, kodlanmış ya da yerel PIN'li giriş yoktur.
//! - Gerçek kişi, parola, anahtar yoktur; örnek veri yalnız `--demo` ile ve "ÖRNEK" etiketiyle gelir.
//! - Rehber verisi dışarı (yapay zekâ, ağ) gönderilmez; bu crate ağa çıkmaz (yönetici girişi dışında, o da `taa-kimlik` üzerinden).
//!
//! Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]

pub mod api;
pub mod belge;
pub mod demo;
pub mod durum;
pub mod gorunum;
pub mod guvenlik;
pub mod ihbar;
pub mod suzgec;
pub mod yonetim;
pub mod zaman;

#[cfg(test)]
mod testler;

pub use api::{yonlendir, Istek};
pub use durum::{
    Durum, Kimlik, KimlikBilgisi, KimlikRed, KimlikYok, Ortam, SistemOrtami, Yapilandirma,
};

use serde_json::Value;

pub const OZELLIK: &str = "rehber-panel";

/// Gömülü ekran (tek dosya, dış kaynak yok). `__TEMA__` açılışta kayıtlı temayla değiştirilir.
pub const ARAYUZ_HTML: &str = include_str!("panel/arayuz.html");
/// Uygulama simgesi (mezura).
pub const SIMGE_SVG: &str = include_str!("panel/simge.svg");

/// Kayıtlı temayı HTML'e yazar (ilk boyamada doğru tema).
pub fn arayuz_html(t: gorunum::Tema) -> String {
    ARAYUZ_HTML.replace("__TEMA__", t.kod())
}

/// API yanıtı: (HTTP durum, JSON gövde).
pub type Yanit = (u16, Value);
