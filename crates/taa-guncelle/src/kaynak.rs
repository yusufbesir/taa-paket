// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! Güncelleme kaynağı soyutlaması. T3: yalnız yerel klasör; HTTP kaynağı T4'te aynı trait'i uygular.

use crate::yayin::MAKS_YAYIN_BAYT;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use taa_kur::paket::guvenli_coz;
use taa_kur::KurHata;

pub trait Kaynak {
    /// `yayin.json` ham baytları.
    fn yayin_oku(&self) -> Result<Vec<u8>, KurHata>;
    /// `url` ile gösterilen dosyayı `hedef` yoluna YENİ dosya olarak yazar (var olan ezilmez); yazılan bayt sayısını döner.
    /// Gerçek uzunluk `beklenen_boyut`u aşarsa durur ve hata verir (şişirilmiş indirmeye karşı).
    fn getir(&self, url: &str, hedef: &Path, beklenen_boyut: u64) -> Result<u64, KurHata>;
}

/// Klasör kaynağı: `<klasor>/yayin.json` ve `url` olarak klasöre göreli yollar.
#[derive(Debug, Clone)]
pub struct YerelKaynak {
    klasor: PathBuf,
}

impl YerelKaynak {
    pub fn yeni(klasor: impl Into<PathBuf>) -> YerelKaynak {
        YerelKaynak {
            klasor: klasor.into(),
        }
    }
}

impl Kaynak for YerelKaynak {
    fn yayin_oku(&self) -> Result<Vec<u8>, KurHata> {
        let yol = self.klasor.join("yayin.json");
        let m = fs::metadata(&yol).map_err(|e| {
            KurHata::Yok(format!(
                "yayin.json bulunamadı ({}): {e}. Öneri: kaynak klasörünü ve kur.toml'daki kaynak_klasoru değerini denetleyin.",
                yol.display()
            ))
        })?;
        if !m.is_file() || m.len() > MAKS_YAYIN_BAYT {
            return Err(KurHata::Dogrulama(
                "yayin.json normal dosya değil ya da çok büyük. Öneri: yayıncıdan yeniden isteyin."
                    .into(),
            ));
        }
        Ok(fs::read(&yol)?)
    }

    fn getir(&self, url: &str, hedef: &Path, beklenen_boyut: u64) -> Result<u64, KurHata> {
        // Yol kaçışı, mutlak yol ve sembolik bağ reddedilir (paket yolu kurallarıyla aynı).
        let kaynak = guvenli_coz(&self.klasor, url).map_err(|m| {
            KurHata::Dogrulama(format!("Kaynak yolu güvenli değil ({m}). Öneri: yayin.json url alanı kaynak klasörüne göreli olmalı."))
        })?;
        let mut oku = fs::File::open(&kaynak)
            .map_err(|e| {
                KurHata::Yok(format!(
                    "{url} kaynakta okunamadı: {e}. Öneri: yayın klasörünü denetleyin."
                ))
            })?
            .take(beklenen_boyut.saturating_add(1));
        let mut yaz = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(hedef)?;
        let n = std::io::copy(&mut oku, &mut yaz)?;
        yaz.flush()?;
        yaz.sync_all()?;
        if n > beklenen_boyut {
            return Err(KurHata::Dogrulama(format!(
                "{url} yayin.json'daki boyuttan ({beklenen_boyut} bayt) büyük. Öneri: yayını yeniden üretin."
            )));
        }
        Ok(n)
    }
}
