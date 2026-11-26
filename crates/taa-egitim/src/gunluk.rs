//! Yalnız-ekleme JSONL izleme günlüğü. Her satır bir olaydır; dosya hiçbir zaman kısaltılmaz ya da yeniden yazılmaz.
//! Satırda yalnız kişi kimliği vardır (ad, e-posta yok). Zamanı çağıran verir (RFC 3339 UTC).
//! Sahip: Yusuf. yazan: [Claude#egitim 2026-11-26]

use crate::{gecersiz, Hata};
use serde::{Deserialize, Serialize};
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use taa_rehber_model::model::zaman_gecerli;

/// İzleme ve atama olayları. `tur` alanı ayırıcıdır.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tur", rename_all = "snake_case", deny_unknown_fields)]
pub enum Olay {
    Atama {
        kisi: String,
        ders: String,
        atayan: String,
    },
    AtamaIptal {
        kisi: String,
        ders: String,
        atayan: String,
    },
    /// Kaldığı saniye (devam noktası).
    Konum {
        kisi: String,
        ders: String,
        surum: u32,
        saniye: u64,
    },
    /// "İzledim" işareti.
    Izledi {
        kisi: String,
        ders: String,
        surum: u32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GunlukSatiri {
    pub sira: u64,
    pub zaman: String,
    #[serde(flatten)]
    pub olay: Olay,
}

pub struct Gunluk {
    yol: PathBuf,
    dosya: File,
    sonraki: u64,
}

fn g(e: std::io::Error) -> Hata {
    Hata::Gunluk(e.to_string())
}

impl Gunluk {
    /// Dosyayı ekleme kipinde açar (yoksa oluşturur). Önceki bir yazma yarıda kesilmişse (son satır `\n` ile bitmiyorsa)
    /// bozuk satır olduğu gibi bırakılıp yeni satır ayrı satırdan başlar.
    pub fn ac(yol: impl AsRef<Path>) -> Result<Self, Hata> {
        let yol = yol.as_ref().to_path_buf();
        let mut dosya = OpenOptions::new()
            .create(true)
            .append(true)
            .read(true)
            .open(&yol)
            .map_err(g)?;
        let uzunluk = dosya.metadata().map_err(g)?.len();
        if uzunluk > 0 {
            dosya.seek(SeekFrom::Start(uzunluk - 1)).map_err(g)?;
            let mut son = [0u8; 1];
            dosya.read_exact(&mut son).map_err(g)?;
            if son[0] != b'\n' {
                dosya.write_all(b"\n").map_err(g)?;
            }
        }
        let (satirlar, _) = Self::oku(&yol)?;
        let sonraki = satirlar.iter().map(|s| s.sira).max().map_or(1, |m| m + 1);
        Ok(Self {
            yol,
            dosya,
            sonraki,
        })
    }

    pub fn yol(&self) -> &Path {
        &self.yol
    }

    pub fn sonraki_sira(&self) -> u64 {
        self.sonraki
    }

    /// Bir olay satırı ekler ve diske eşitler.
    pub fn ekle(&mut self, zaman: &str, olay: Olay) -> Result<GunlukSatiri, Hata> {
        if !zaman_gecerli(zaman) {
            return Err(gecersiz("zaman", "RFC 3339 UTC (…Z) olmalı"));
        }
        let satir = GunlukSatiri {
            sira: self.sonraki,
            zaman: zaman.to_string(),
            olay,
        };
        let mut metin = serde_json::to_string(&satir).map_err(|e| Hata::Gunluk(e.to_string()))?;
        metin.push('\n');
        self.dosya.write_all(metin.as_bytes()).map_err(g)?;
        self.dosya.sync_data().map_err(g)?;
        self.sonraki += 1;
        Ok(satir)
    }

    /// Dosyayı okur: (geçerli satırlar, bozuk satır sayısı). Bozuk satırlar atlanır, dosyaya dokunulmaz.
    pub fn oku(yol: impl AsRef<Path>) -> Result<(Vec<GunlukSatiri>, usize), Hata> {
        let mut metin = Vec::new();
        File::open(yol.as_ref())
            .map_err(g)?
            .read_to_end(&mut metin)
            .map_err(g)?;
        let metin = String::from_utf8_lossy(&metin);
        let (mut satirlar, mut bozuk) = (Vec::new(), 0usize);
        for s in metin.lines().filter(|s| !s.trim().is_empty()) {
            match serde_json::from_str::<GunlukSatiri>(s) {
                Ok(x) => satirlar.push(x),
                Err(_) => bozuk += 1,
            }
        }
        Ok((satirlar, bozuk))
    }
}

/// Yalnız testler: tekil geçici günlük yolu.
#[cfg(test)]
pub(crate) fn testler_gecici(ad: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("taa-egitim-test-{}-{ad}.jsonl", std::process::id()));
    let _ = std::fs::remove_file(&p);
    p
}

#[cfg(test)]
mod testler {
    use super::*;

    const Z: &str = "2026-11-26T09:00:00Z";

    use super::testler_gecici as gecici;

    fn konum(s: u64) -> Olay {
        Olay::Konum {
            kisi: "k1".into(),
            ders: "d1".into(),
            surum: 1,
            saniye: s,
        }
    }

    #[test]
    fn ekler_ve_okur() {
        let y = gecici("ekler");
        let mut g = Gunluk::ac(&y).unwrap();
        assert_eq!(g.sonraki_sira(), 1);
        g.ekle(Z, konum(10)).unwrap();
        g.ekle(
            Z,
            Olay::Izledi {
                kisi: "k1".into(),
                ders: "d1".into(),
                surum: 1,
            },
        )
        .unwrap();
        let (s, bozuk) = Gunluk::oku(&y).unwrap();
        assert_eq!(bozuk, 0);
        assert_eq!(s.len(), 2);
        assert_eq!(s[0].sira, 1);
        assert_eq!(s[1].sira, 2);
        assert_eq!(
            s[1].olay,
            Olay::Izledi {
                kisi: "k1".into(),
                ders: "d1".into(),
                surum: 1
            }
        );
        let _ = std::fs::remove_file(&y);
    }

    #[test]
    fn satir_bicimi_tek_satir_json() {
        let y = gecici("bicim");
        let mut g = Gunluk::ac(&y).unwrap();
        g.ekle(Z, konum(7)).unwrap();
        let ham = std::fs::read_to_string(&y).unwrap();
        assert_eq!(ham.lines().count(), 1);
        assert!(ham.ends_with('\n'));
        let v: serde_json::Value = serde_json::from_str(ham.trim()).unwrap();
        assert_eq!(v["tur"], "konum");
        assert_eq!(v["saniye"], 7);
        assert_eq!(v["kisi"], "k1");
        assert!(v.get("ad").is_none() && v.get("eposta").is_none());
        let _ = std::fs::remove_file(&y);
    }

    #[test]
    fn yeniden_acinca_ekleme_surer_ve_eskiyi_degistirmez() {
        let y = gecici("yeniden");
        {
            let mut g = Gunluk::ac(&y).unwrap();
            g.ekle(Z, konum(1)).unwrap();
        }
        let once = std::fs::read(&y).unwrap();
        let mut g = Gunluk::ac(&y).unwrap();
        assert_eq!(g.sonraki_sira(), 2);
        g.ekle(Z, konum(2)).unwrap();
        let sonra = std::fs::read(&y).unwrap();
        assert!(sonra.starts_with(&once), "önceki bayt dizisi korunmalı");
        assert_eq!(Gunluk::oku(&y).unwrap().0.len(), 2);
        let _ = std::fs::remove_file(&y);
    }

    #[test]
    fn yarim_kalan_satir_sonrakini_bozmaz() {
        let y = gecici("yarim");
        {
            let mut g = Gunluk::ac(&y).unwrap();
            g.ekle(Z, konum(1)).unwrap();
        }
        let mut f = OpenOptions::new().append(true).open(&y).unwrap();
        f.write_all(br#"{"sira":2,"zaman":"2026-11-26T09:00:00Z","tur":"kon"#)
            .unwrap();
        drop(f);
        let mut g = Gunluk::ac(&y).unwrap();
        g.ekle(Z, konum(3)).unwrap();
        let (s, bozuk) = Gunluk::oku(&y).unwrap();
        assert_eq!(bozuk, 1);
        assert_eq!(s.len(), 2);
        assert_eq!(s[1].sira, 2);
        let _ = std::fs::remove_file(&y);
    }

    #[test]
    fn gecersiz_zaman_reddedilir_ve_dosya_degismez() {
        let y = gecici("zaman");
        let mut g = Gunluk::ac(&y).unwrap();
        assert!(g.ekle("dün", konum(1)).is_err());
        assert_eq!(std::fs::metadata(&y).unwrap().len(), 0);
        assert_eq!(g.sonraki_sira(), 1);
        let _ = std::fs::remove_file(&y);
    }

    #[test]
    fn bozuk_satir_atlanir() {
        let y = gecici("bozuk");
        std::fs::write(&y, "{bozuk}\n\n").unwrap();
        let (s, bozuk) = Gunluk::oku(&y).unwrap();
        assert!(s.is_empty());
        assert_eq!(bozuk, 1);
        let _ = std::fs::remove_file(&y);
    }

    #[test]
    fn olay_turleri_gidis_donus() {
        for o in [
            Olay::Atama {
                kisi: "a".into(),
                ders: "d".into(),
                atayan: "y".into(),
            },
            Olay::AtamaIptal {
                kisi: "a".into(),
                ders: "d".into(),
                atayan: "y".into(),
            },
            konum(5),
            Olay::Izledi {
                kisi: "a".into(),
                ders: "d".into(),
                surum: 2,
            },
        ] {
            let s = GunlukSatiri {
                sira: 1,
                zaman: Z.into(),
                olay: o,
            };
            let j = serde_json::to_string(&s).unwrap();
            assert_eq!(serde_json::from_str::<GunlukSatiri>(&j).unwrap(), s);
        }
    }
}
