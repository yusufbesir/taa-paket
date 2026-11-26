//! Sentetik CPU işi ve temsilî ön plan işlemi. Sahip: Yusuf. yazan: [Claude#olcum 2026-11-26]
//! OCR yerine şimdilik: sabit girdi üzerinde SHA-256 zinciri. Çıktı belirlidir (aynı girdi, aynı özet), böylece
//! her biten birim referansla bit bit doğrulanır.

use sha2::{Digest, Sha256};

/// Bir iş biriminin zincir uzunluğu.
pub const BIRIM_TUR: u32 = 20_000;
/// Temsilî ön plan işleminin zincir uzunluğu (birimden çok kısa).
pub const SORGU_TUR: u32 = 4_000;

fn zincir(tohum: &[u8], tur: u32) -> [u8; 32] {
    let mut d: [u8; 32] = Sha256::digest(tohum).into();
    for _ in 0..tur {
        d = Sha256::digest(d).into();
    }
    d
}

/// Bir iş birimi: sabit tohum üzerinde zincir.
pub fn birim_calistir() -> [u8; 32] {
    zincir(b"taa-olcum sentetik is birimi v1", BIRIM_TUR)
}

/// Tek iş parçacığında, sınırsız hesaplanan referans özet.
pub fn birim_referans() -> [u8; 32] {
    birim_calistir()
}

/// Temsilî kasa işlemi yerine geçen küçük iş.
pub fn sorgu_calistir() -> [u8; 32] {
    zincir(b"taa-olcum temsili on plan islemi v1", SORGU_TUR)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn birim_belirli() {
        assert_eq!(birim_calistir(), birim_referans());
        assert_ne!(birim_calistir(), sorgu_calistir());
    }
}
