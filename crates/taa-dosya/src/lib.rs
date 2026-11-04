//! taa-dosya: biçim algılama (sihirli bayt), atomik yazma ve geçici klasör yardımcıları.
//! Sahip: Yusuf (TAA Paketi). yazan: [Claude#taa-paket-A0 2026-11-04].
//! Değişiklik: Yusuf'un kuralları çerçevesinde; yalnız std kullanır.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Baştaki sihirli baytlardan dosya biçimini algılar; bilinmiyorsa `None`.
/// Dönen değerler: pdf, zip, png, jpg, gif, webp, bmp, mp3, mp4, wav, gz, xml, html.
/// Not: docx/xlsx de zip kapsayıcısıdır; "zip" döner (içerik bakılmaz).
pub fn bicim_algila(bytes: &[u8]) -> Option<&'static str> {
    let b = bytes;
    if b.starts_with(b"%PDF-") {
        Some("pdf")
    } else if b.starts_with(b"PK\x03\x04") || b.starts_with(b"PK\x05\x06") {
        Some("zip")
    } else if b.starts_with(b"\x89PNG\r\n\x1a\n") {
        Some("png")
    } else if b.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some("jpg")
    } else if b.starts_with(b"GIF87a") || b.starts_with(b"GIF89a") {
        Some("gif")
    } else if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WEBP" {
        Some("webp")
    } else if b.len() >= 12 && &b[0..4] == b"RIFF" && &b[8..12] == b"WAVE" {
        Some("wav")
    } else if b.starts_with(b"BM") && b.len() >= 14 {
        Some("bmp")
    } else if b.starts_with(b"ID3") || (b.len() >= 2 && b[0] == 0xFF && (b[1] & 0xE0) == 0xE0) {
        Some("mp3")
    } else if b.len() >= 12 && &b[4..8] == b"ftyp" {
        Some("mp4")
    } else if b.starts_with(&[0x1F, 0x8B]) {
        Some("gz")
    } else {
        let t = b.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(b);
        let t = &t[t
            .iter()
            .position(|c| !c.is_ascii_whitespace())
            .unwrap_or(t.len())..];
        let kucuk: Vec<u8> = t.iter().take(15).map(|c| c.to_ascii_lowercase()).collect();
        if kucuk.starts_with(b"<?xml") {
            Some("xml")
        } else if kucuk.starts_with(b"<!doctype html") || kucuk.starts_with(b"<html") {
            Some("html")
        } else {
            None
        }
    }
}

/// `yol` doluysa "ad (1).uzanti", "ad (2).uzanti" ... biçiminde boş bir yol üretir.
fn bos_yol(yol: &Path) -> PathBuf {
    if !yol.exists() {
        return yol.to_path_buf();
    }
    let dir = yol.parent().map(Path::to_path_buf).unwrap_or_default();
    let govde = yol
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let uzanti = yol.extension().map(|e| e.to_string_lossy().into_owned());
    let mut n = 1u32;
    loop {
        let ad = match &uzanti {
            Some(u) => format!("{govde} ({n}).{u}"),
            None => format!("{govde} ({n})"),
        };
        let aday = dir.join(ad);
        if !aday.exists() {
            return aday;
        }
        n += 1;
    }
}

/// Baytları geçici dosyaya yazıp yeniden adlandırarak atomik kaydeder.
/// Var olan dosyanın üzerine yazmaz; hedef doluysa "(n)" eki alır. Gerçek yolu döndürür.
pub fn atomik_yaz(yol: &Path, bytes: &[u8]) -> io::Result<PathBuf> {
    let dizin = yol.parent().map(Path::to_path_buf).unwrap_or_default();
    if !dizin.as_os_str().is_empty() {
        fs::create_dir_all(&dizin)?;
    }
    let gecici = dizin.join(format!(".taa-{}-{}.tmp", std::process::id(), sayac()));
    {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&gecici)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    // Yarış durumunda hedef başka süreççe alınabilir; birkaç kez dene.
    for _ in 0..100 {
        let hedef = bos_yol(yol);
        // hard_link üzerine yazmaz (hedef varsa hata verir); sonra geçici silinir.
        match fs::hard_link(&gecici, &hedef) {
            Ok(()) => {
                let _ = fs::remove_file(&gecici);
                return Ok(hedef);
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(_) => {
                // Dosya sistemi bağlantıyı desteklemiyorsa: yeniden adlandır (hedef boş doğrulandı).
                if !hedef.exists() {
                    return match fs::rename(&gecici, &hedef) {
                        Ok(()) => Ok(hedef),
                        Err(e) => {
                            let _ = fs::remove_file(&gecici);
                            Err(e)
                        }
                    };
                }
            }
        }
    }
    let _ = fs::remove_file(&gecici);
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "boş hedef adı bulunamadı",
    ))
}

fn sayac() -> u64 {
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

/// Benzersiz, yeni oluşturulmuş bir geçici klasör açar (sistem geçici dizini altında "taa-...").
/// Silme işini çağıran yapar (Geri Dönüşüm Kutusu kuralı).
pub fn gecici_klasor() -> io::Result<PathBuf> {
    let taban = std::env::temp_dir();
    let zaman = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    for _ in 0..100 {
        let p = taban.join(format!("taa-{}-{}-{}", std::process::id(), zaman, sayac()));
        match fs::create_dir(&p) {
            Ok(()) => return Ok(p),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "geçici klasör açılamadı",
    ))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn bicimler() {
        assert_eq!(bicim_algila(b"%PDF-1.7"), Some("pdf"));
        assert_eq!(bicim_algila(b"PK\x03\x04abc"), Some("zip"));
        assert_eq!(bicim_algila(b"\x89PNG\r\n\x1a\nxx"), Some("png"));
        assert_eq!(bicim_algila(&[0xFF, 0xD8, 0xFF, 0xE0]), Some("jpg"));
        assert_eq!(bicim_algila(b"GIF89a.."), Some("gif"));
        assert_eq!(bicim_algila(b"RIFF\0\0\0\0WEBPVP8 "), Some("webp"));
        assert_eq!(bicim_algila(b"RIFF\0\0\0\0WAVEfmt "), Some("wav"));
        assert_eq!(bicim_algila(b"BM\0\0\0\0\0\0\0\0\0\0\0\0"), Some("bmp"));
        assert_eq!(bicim_algila(b"ID3\x03\0"), Some("mp3"));
        assert_eq!(bicim_algila(b"\0\0\0\x18ftypmp42"), Some("mp4"));
        assert_eq!(bicim_algila(&[0x1F, 0x8B, 8]), Some("gz"));
        assert_eq!(bicim_algila(b"  <?xml version=\"1.0\"?>"), Some("xml"));
        assert_eq!(bicim_algila(b"<!DOCTYPE html><html>"), Some("html"));
        assert_eq!(bicim_algila(b"merhaba"), None);
        assert_eq!(bicim_algila(b""), None);
    }

    #[test]
    fn atomik_ustune_yazmaz() {
        let d = gecici_klasor().unwrap();
        let y = d.join("a.txt");
        assert_eq!(atomik_yaz(&y, b"bir").unwrap(), y);
        let y2 = atomik_yaz(&y, b"iki").unwrap();
        assert_eq!(y2, d.join("a (1).txt"));
        let y3 = atomik_yaz(&y, b"uc").unwrap();
        assert_eq!(y3, d.join("a (2).txt"));
        assert_eq!(fs::read(&y).unwrap(), b"bir");
        assert_eq!(fs::read(&y2).unwrap(), b"iki");
        let artik = fs::read_dir(&d)
            .unwrap()
            .filter(|e| {
                e.as_ref()
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".tmp")
            })
            .count();
        assert_eq!(artik, 0);
    }

    #[test]
    fn uzantisiz_ek() {
        let d = gecici_klasor().unwrap();
        let y = d.join("dosya");
        atomik_yaz(&y, b"x").unwrap();
        assert_eq!(atomik_yaz(&y, b"y").unwrap(), d.join("dosya (1)"));
    }

    #[test]
    fn gecici_benzersiz() {
        let a = gecici_klasor().unwrap();
        let b = gecici_klasor().unwrap();
        assert_ne!(a, b);
        assert!(a.is_dir() && b.is_dir());
    }
}
