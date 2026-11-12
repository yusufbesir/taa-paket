//! Girdi tespiti: dosya uzantısına değil içeriğe bakılır.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tur {
    Goruntu(&'static str),
    Pdf,
    Docx,
    /// Parola korumalı (eski Office OLE kapsayıcısı).
    Korumali,
    Desteklenmeyen,
}

pub const GORUNTULER: [&str; 5] = ["jpg", "png", "tiff", "bmp", "webp"];

/// Girdi türünü bayt başlarından saptar. docx için zip içinde `word/document.xml` aranır.
pub fn tani(b: &[u8]) -> Tur {
    if b.starts_with(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1]) {
        return Tur::Korumali;
    }
    if b.starts_with(b"II*\0") || b.starts_with(b"MM\0*") {
        return Tur::Goruntu("tiff");
    }
    match taa_dosya::bicim_algila(b) {
        Some("pdf") => Tur::Pdf,
        Some(g @ ("jpg" | "png" | "bmp" | "webp")) => Tur::Goruntu(g),
        Some("zip") => {
            let Ok(mut z) = zip::ZipArchive::new(std::io::Cursor::new(b)) else {
                return Tur::Desteklenmeyen;
            };
            if z.by_name("word/document.xml").is_ok() {
                Tur::Docx
            } else if z.by_name("EncryptedPackage").is_ok() {
                Tur::Korumali
            } else {
                Tur::Desteklenmeyen
            }
        }
        _ => Tur::Desteklenmeyen,
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    #[test]
    fn jpeg_png_tiff_pdf_taninir() {
        assert_eq!(tani(&[0xFF, 0xD8, 0xFF, 0xE0]), Tur::Goruntu("jpg"));
        assert_eq!(tani(b"\x89PNG\r\n\x1a\n...."), Tur::Goruntu("png"));
        assert_eq!(tani(b"II*\0abcd"), Tur::Goruntu("tiff"));
        assert_eq!(tani(b"%PDF-1.7"), Tur::Pdf);
        assert_eq!(tani(b"merhaba"), Tur::Desteklenmeyen);
    }
    #[test]
    fn eski_office_korumali_sayilir() {
        assert_eq!(tani(&[0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1, 0]), Tur::Korumali);
    }
}
