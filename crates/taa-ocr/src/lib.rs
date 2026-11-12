//! taa-ocr: TAA ortak OCR motoru. Sözleşme: docs/OCR-MOTORU.md (ocr.oku@1, ocr.kod@1, ocr.tablo@1, ocr.bilgi@1).
//! Girdi içeriğe göre tanınır (imza baytları). Metin katmanı (PDF metni, docx) doğrudan çıkarılır; görüntü ve taranmış
//! sayfalar etkin model paketine verilir. Model yoksa uydurma sonuç dönmez: `belirsiz: true`, boş metin.
//! Model paketi ayrı sürümlenir, imzalıdır, geri dönülebilir (bkz. model.rs). Ağ yok, sonuç saklanmaz.

pub mod eslestir;
pub mod girdi;
pub mod metin;
pub mod model;
pub mod okuyucu;

pub const MOTOR_SURUMU: &str = env!("CARGO_PKG_VERSION");
pub const ARAYUZ: u32 = 1;
pub const YETENEKLER: [&str; 4] = ["ocr.oku@1", "ocr.kod@1", "ocr.tablo@1", "ocr.bilgi@1"];
