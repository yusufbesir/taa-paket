//! TaaDön çıktı imza denetimi (src/donustur.js CIKTI_IMZALARI + ciktiGecerli ile birebir aynı kural).
//! Sahip: Yusuf (TaaDön projesi). yazan: [Claude#taadon-rust-asama1 2026-11-04], [Claude#taadon-rust-asama2 2026-11-04] (ad: taa-donustur).
//! Değişiklik: Yusuf'un kuralları çerçevesinde; imza listesi Node tarafıyla birlikte güncellenmelidir.

use crate::{uzanti_al, Hata, Sonuc};
use std::fs::File;
use std::io::Read;
use std::path::Path;

const PK: &[u8] = &[0x50, 0x4b];

/// Uzantıya göre beklenen ilk baytlar; listede olmayan uzantı denetlenmez.
pub fn beklenen_imza(uzanti: &str) -> Option<&'static [u8]> {
    match uzanti {
        "pdf" => Some(&[0x25, 0x50, 0x44, 0x46]),
        "docx" | "xlsx" | "pptx" | "odt" | "ods" | "odp" | "epub" | "zip" => Some(PK),
        "png" => Some(&[0x89, 0x50, 0x4e, 0x47]),
        "jpg" | "jpeg" => Some(&[0xff, 0xd8, 0xff]),
        "gif" => Some(&[0x47, 0x49, 0x46, 0x38]),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImzaSonucu {
    pub uzanti: String,
    pub imza_uygun: bool,
    pub boyut: u64,
}

/// ciktiGecerli kuralı: klasör → uygun; dosya değil ya da boş → uygun değil; imzası yoksa → uygun;
/// dosya imzadan kısa ya da baytlar tutmuyor → uygun değil.
pub fn denetle(yol: &Path) -> Sonuc<ImzaSonucu> {
    let uzanti = uzanti_al(yol);
    let md = std::fs::metadata(yol)
        .map_err(|_| Hata::yeni(format!("Dosya bulunamadı: {}", yol.display())))?;
    if md.is_dir() {
        return Ok(ImzaSonucu {
            uzanti,
            imza_uygun: true,
            boyut: 0,
        });
    }
    let boyut = md.len();
    if !md.is_file() || boyut == 0 {
        return Ok(ImzaSonucu {
            uzanti,
            imza_uygun: false,
            boyut,
        });
    }
    let Some(imza) = beklenen_imza(&uzanti) else {
        return Ok(ImzaSonucu {
            uzanti,
            imza_uygun: true,
            boyut,
        });
    };
    if boyut < imza.len() as u64 {
        return Ok(ImzaSonucu {
            uzanti,
            imza_uygun: false,
            boyut,
        });
    }
    let mut tampon = vec![0u8; imza.len()];
    let mut f = File::open(yol).map_err(|e| Hata::yeni(format!("Dosya açılamadı: {e}")))?;
    f.read_exact(&mut tampon)
        .map_err(|e| Hata::yeni(format!("Dosya okunamadı: {e}")))?;
    Ok(ImzaSonucu {
        uzanti,
        imza_uygun: tampon == imza,
        boyut,
    })
}
