//! taa-tara: JPEG görüntülerden (telefon/tarayıcı) sayfa başına bir görüntülü PDF üretir; JPEG verisi olduğu gibi
//! gömülür (yeniden sıkıştırma yok, kayıp yok). OCR, kenar düzeltme ve PDF/A HENÜZ YOK (OCR eğitimi en sona bırakıldı).
//! Özellik anahtarı: "tara" (varsayılan açık). Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use lopdf::{dictionary, Document, Object, Stream};
use std::path::{Path, PathBuf};

pub const OZELLIK: &str = "tara";
/// PDF birimi (pt) cinsinden A4 genişliği; görüntü bu genişliğe sığdırılır (taşmazsa küçültülür).
const A4_GEN: f64 = 595.0;
const A4_YUK: f64 = 842.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JpegBilgi {
    pub genislik: u32,
    pub yukseklik: u32,
    pub bilesen: u8,
}

/// SOF işaretçisinden boyut ve bileşen sayısını okur (progressive dahil).
pub fn jpeg_bilgi(b: &[u8]) -> Result<JpegBilgi, String> {
    if !b.starts_with(&[0xFF, 0xD8]) {
        return Err("JPEG değil".into());
    }
    let mut i = 2;
    while i + 4 <= b.len() {
        if b[i] != 0xFF {
            i += 1;
            continue;
        }
        let m = b[i + 1];
        if m == 0xFF {
            i += 1;
            continue;
        }
        if m == 0xD8 || m == 0x01 || (0xD0..=0xD7).contains(&m) {
            i += 2;
            continue;
        }
        let uzun = u16::from_be_bytes([b[i + 2], b[i + 3]]) as usize;
        if matches!(m, 0xC0..=0xC3 | 0xC5..=0xC7 | 0xC9..=0xCB | 0xCD..=0xCF) {
            if i + 10 > b.len() {
                break;
            }
            let yuk = u16::from_be_bytes([b[i + 5], b[i + 6]]) as u32;
            let gen = u16::from_be_bytes([b[i + 7], b[i + 8]]) as u32;
            let bil = b[i + 9];
            if gen == 0 || yuk == 0 {
                return Err("JPEG boyutu geçersiz".into());
            }
            return Ok(JpegBilgi {
                genislik: gen,
                yukseklik: yuk,
                bilesen: bil,
            });
        }
        i += 2 + uzun;
    }
    Err("JPEG başlığı okunamadı".into())
}

/// Her JPEG için bir sayfa; sayfa görüntüye göre boyutlanır (en fazla A4'e sığacak şekilde, orantı korunur).
pub fn jpeglerden_pdf(jpegler: &[Vec<u8>]) -> Result<Document, String> {
    if !taa_ayar::ozellik_acik(OZELLIK, true) {
        return Err(taa_ayar::kapali_mesaji(OZELLIK));
    }
    if jpegler.is_empty() {
        return Err("en az bir görüntü gerekir".into());
    }
    let mut d = Document::with_version("1.5");
    let kok = d.new_object_id();
    let mut kids = Vec::new();
    for (n, j) in jpegler.iter().enumerate() {
        let bilgi = jpeg_bilgi(j).map_err(|e| format!("görüntü {}: {e}", n + 1))?;
        let renk = match bilgi.bilesen {
            1 => "DeviceGray",
            3 => "DeviceRGB",
            4 => "DeviceCMYK",
            x => {
                return Err(format!(
                    "görüntü {}: desteklenmeyen bileşen sayısı {x}",
                    n + 1
                ))
            }
        };
        let (g, y) = (bilgi.genislik as f64, bilgi.yukseklik as f64);
        let olcek = (A4_GEN / g).min(A4_YUK / y).min(1.0);
        let (sg, sy) = ((g * olcek).round().max(1.0), (y * olcek).round().max(1.0));
        let mut sozluk = dictionary! {
            "Type" => "XObject", "Subtype" => "Image",
            "Width" => bilgi.genislik as i64, "Height" => bilgi.yukseklik as i64,
            "ColorSpace" => renk, "BitsPerComponent" => 8, "Filter" => "DCTDecode",
        };
        if bilgi.bilesen == 4 {
            // Adobe CMYK JPEG'ler ters kodlanır.
            sozluk.set(
                "Decode",
                vec![
                    1.into(),
                    0.into(),
                    1.into(),
                    0.into(),
                    1.into(),
                    0.into(),
                    1.into(),
                    0.into(),
                ],
            );
        }
        let img = d.add_object(Stream::new(sozluk, j.clone()).with_compression(false));
        let icerik = d.add_object(Stream::new(
            dictionary! {},
            format!("q {sg} 0 0 {sy} 0 0 cm /Im0 Do Q").into_bytes(),
        ));
        let sayfa = d.add_object(dictionary! {
            "Type" => "Page", "Parent" => kok, "Contents" => icerik,
            "MediaBox" => vec![0.into(), 0.into(), Object::Real(sg as f32), Object::Real(sy as f32)],
            "Resources" => dictionary! { "XObject" => dictionary! { "Im0" => img } },
        });
        kids.push(Object::Reference(sayfa));
    }
    d.objects.insert(
        kok,
        Object::Dictionary(
            dictionary! { "Type" => "Pages", "Count" => kids.len() as i64, "Kids" => kids },
        ),
    );
    let kat = d.add_object(dictionary! { "Type" => "Catalog", "Pages" => kok });
    d.trailer.set("Root", kat);
    Ok(d)
}

/// Dosyaları okuyup PDF'i yeni dosyaya yazar (üzerine yazmaz). Gerçek yolu döndürür.
pub fn dosyalardan_pdf(girdiler: &[PathBuf], cikti: &Path) -> Result<PathBuf, String> {
    let mut jpegler = Vec::new();
    for g in girdiler {
        jpegler.push(std::fs::read(g).map_err(|e| format!("{}: {e}", g.display()))?);
    }
    let mut d = jpeglerden_pdf(&jpegler)?;
    let mut b = Vec::new();
    d.save_to(&mut b).map_err(|e| e.to_string())?;
    taa_dosya::atomik_yaz(cikti, &b).map_err(|e| e.to_string())
}

/// OCR henüz yoktur; çağıran açıkça bilgilendirilir (sahte sonuç dönmez).
pub fn ocr(_goruntu: &[u8]) -> Result<String, String> {
    Err("OCR henüz yok (El-Yazısı-OCR eğitimi en sona bırakıldı)".into())
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Yalnız başlığı geçerli, SOF0 içeren en küçük JPEG iskeleti (çözülmez, yalnız gömülür).
    fn sahte_jpeg(gen: u16, yuk: u16, bil: u8) -> Vec<u8> {
        let mut v = vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x04, 0x00, 0x00];
        v.extend([0xFF, 0xC0, 0x00, 0x11, 8]);
        v.extend(yuk.to_be_bytes());
        v.extend(gen.to_be_bytes());
        v.push(bil);
        v.extend(vec![0u8; 3 * bil as usize]);
        v.extend([0xFF, 0xD9]);
        v
    }

    #[test]
    fn boyut_okur() {
        let b = jpeg_bilgi(&sahte_jpeg(1200, 1600, 3)).unwrap();
        assert_eq!((b.genislik, b.yukseklik, b.bilesen), (1200, 1600, 3));
        assert!(jpeg_bilgi(b"PNG").is_err());
        assert!(jpeg_bilgi(&[0xFF, 0xD8, 0xFF, 0xD9]).is_err());
    }

    #[test]
    fn pdf_uretir_ve_yeniden_okunur() {
        let mut d = jpeglerden_pdf(&[sahte_jpeg(1200, 1600, 3), sahte_jpeg(300, 200, 1)]).unwrap();
        let mut b = Vec::new();
        d.save_to(&mut b).unwrap();
        assert_eq!(taa_dosya::bicim_algila(&b), Some("pdf"));
        let d2 = Document::load_mem(&b).unwrap();
        assert_eq!(d2.get_pages().len(), 2);
    }

    #[test]
    fn hatalar() {
        assert!(jpeglerden_pdf(&[]).is_err());
        assert!(jpeglerden_pdf(&[sahte_jpeg(10, 10, 2)]).is_err());
        assert!(ocr(b"").is_err());
    }
}
