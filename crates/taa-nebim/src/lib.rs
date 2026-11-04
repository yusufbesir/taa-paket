//! taa-nebim: Nebim'in dışa aktardığı CSV raporunu klasörden okur (YALNIZ OKUMA).
//! Nebim'e yazma yolu yoktur. Özellik anahtarı: "nebim-alici" (varsayılan açık).
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].
//! Not: .xlsx henüz desteklenmez; klasörde yalnız xlsx varsa bu açıkça bildirilir.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const OZELLIK: &str = "nebim-alici";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Urun {
    pub kod: String,
    pub barkod: String,
    pub ad: String,
    pub renk: String,
    pub beden: String,
    /// Kuruş cinsinden liste fiyatı.
    pub fiyat: i64,
    /// Kuruş cinsinden indirimli fiyat (varsa).
    pub indirimli: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hata(pub String);

impl std::fmt::Display for Hata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Türkçe harfleri ASCII'ye indirip küçültür; başlık eşleştirmesi için.
fn katla(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'İ' | 'I' | 'ı' | 'i' => 'i',
            'Ş' | 'ş' => 's',
            'Ğ' | 'ğ' => 'g',
            'Ü' | 'ü' => 'u',
            'Ö' | 'ö' => 'o',
            'Ç' | 'ç' => 'c',
            c => c.to_ascii_lowercase(),
        })
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ')
        .collect::<String>()
        .trim()
        .to_string()
}

/// UTF-8 değilse Windows-1254 (Türkçe) varsayar.
pub fn metne_cevir(bytes: &[u8]) -> String {
    let b = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(b) {
        Ok(s) => s.to_string(),
        Err(_) => b
            .iter()
            .map(|&x| match x {
                0xD0 => 'Ğ',
                0xDD => 'İ',
                0xDE => 'Ş',
                0xF0 => 'ğ',
                0xFD => 'ı',
                0xFE => 'ş',
                x => x as char,
            })
            .collect(),
    }
}

fn ayirac_bul(ilk_satir: &str) -> char {
    [';', '\t', ','].into_iter().max_by_key(|c| ilk_satir.matches(*c).count()).unwrap_or(';')
}

/// Tırnaklı alanları destekleyen CSV satır ayrıştırıcı.
pub fn csv_satirlari(metin: &str, ayirac: char) -> Vec<Vec<String>> {
    let mut satirlar = Vec::new();
    let mut satir = Vec::new();
    let mut alan = String::new();
    let mut tirnak = false;
    let mut it = metin.chars().peekable();
    while let Some(c) = it.next() {
        if tirnak {
            if c == '"' {
                if it.peek() == Some(&'"') {
                    alan.push('"');
                    it.next();
                } else {
                    tirnak = false;
                }
            } else {
                alan.push(c);
            }
        } else if c == '"' {
            tirnak = true;
        } else if c == ayirac {
            satir.push(std::mem::take(&mut alan));
        } else if c == '\n' || c == '\r' {
            if c == '\r' && it.peek() == Some(&'\n') {
                it.next();
            }
            satir.push(std::mem::take(&mut alan));
            if !(satir.len() == 1 && satir[0].is_empty()) {
                satirlar.push(std::mem::take(&mut satir));
            } else {
                satir.clear();
            }
        } else {
            alan.push(c);
        }
    }
    if !alan.is_empty() || !satir.is_empty() {
        satir.push(alan);
        satirlar.push(satir);
    }
    satirlar
}

/// "1.299,90", "1299.90", "1299" -> kuruş.
pub fn fiyat_kurus(s: &str) -> Option<i64> {
    let t: String = s.chars().filter(|c| !c.is_whitespace() && *c != '₺').collect();
    let t = t.trim_end_matches("TL").trim_end_matches("tl");
    if t.is_empty() {
        return None;
    }
    let (tam, kes) = match (t.rfind(','), t.rfind('.')) {
        (Some(v), Some(n)) if v > n => (&t[..v], &t[v + 1..]),
        (Some(_), Some(n)) => (&t[..n], &t[n + 1..]),
        (Some(v), None) => (&t[..v], &t[v + 1..]),
        (None, Some(n)) if t.len() - n - 1 <= 2 => (&t[..n], &t[n + 1..]),
        _ => (t, ""),
    };
    let tam: String = tam.chars().filter(|c| c.is_ascii_digit()).collect();
    if tam.is_empty() && kes.is_empty() {
        return None;
    }
    let tam: i64 = if tam.is_empty() { 0 } else { tam.parse().ok()? };
    let kes_k = match kes.len() {
        0 => 0,
        1 => kes.parse::<i64>().ok()? * 10,
        _ => kes[..2].parse::<i64>().ok()?,
    };
    Some(tam * 100 + kes_k)
}

fn sutun(basliklar: &[String], adaylar: &[&str]) -> Option<usize> {
    basliklar.iter().position(|b| adaylar.contains(&b.as_str()))
}

/// CSV metninden ürünleri okur. Kod (ya da barkod) ve fiyat sütunu zorunludur.
pub fn urunleri_oku(metin: &str) -> Result<Vec<Urun>, Hata> {
    let ilk = metin.lines().next().unwrap_or("");
    let satirlar = csv_satirlari(metin, ayirac_bul(ilk));
    let Some((bas, govde)) = satirlar.split_first() else {
        return Err(Hata("dosya boş".into()));
    };
    let b: Vec<String> = bas.iter().map(|x| katla(x)).collect();
    let kod = sutun(&b, &["urun kodu", "stok kodu", "kod", "urunkodu", "model kodu"]);
    let barkod = sutun(&b, &["barkod", "barcode"]);
    let ad = sutun(&b, &["urun aciklamasi", "aciklama", "urun adi", "ad", "urun"]);
    let renk = sutun(&b, &["renk", "renk aciklamasi"]);
    let beden = sutun(&b, &["beden", "beden aciklamasi"]);
    let fiyat = sutun(&b, &["fiyat", "satis fiyati", "perakende fiyat", "liste fiyati"]);
    let indirimli = sutun(&b, &["indirimli fiyat", "kampanya fiyati", "indirimli satis fiyati"]);
    if kod.is_none() && barkod.is_none() {
        return Err(Hata("ürün kodu ya da barkod sütunu bulunamadı".into()));
    }
    let Some(fiyat) = fiyat else {
        return Err(Hata("fiyat sütunu bulunamadı".into()));
    };
    let al = |s: &Vec<String>, i: Option<usize>| i.and_then(|i| s.get(i)).map(|x| x.trim().to_string()).unwrap_or_default();
    let mut cikti = Vec::new();
    for s in govde {
        let k = al(s, kod);
        let bk = al(s, barkod);
        if k.is_empty() && bk.is_empty() {
            continue;
        }
        let Some(f) = s.get(fiyat).and_then(|x| fiyat_kurus(x)) else {
            continue;
        };
        cikti.push(Urun {
            kod: if k.is_empty() { bk.clone() } else { k },
            barkod: bk,
            ad: al(s, ad),
            renk: al(s, renk),
            beden: al(s, beden),
            fiyat: f,
            indirimli: indirimli.and_then(|i| s.get(i)).and_then(|x| fiyat_kurus(x)),
        });
    }
    Ok(cikti)
}

/// Klasördeki en yeni .csv (salt okunur tarama).
pub fn en_yeni_csv(klasor: &Path) -> Result<Option<PathBuf>, Hata> {
    let ogeler = fs::read_dir(klasor).map_err(|e| Hata(format!("klasör okunamadı: {e}")))?;
    let mut en: Option<(std::time::SystemTime, PathBuf)> = None;
    for o in ogeler.flatten() {
        let yol = o.path();
        let csv = yol.extension().map(|e| e.eq_ignore_ascii_case("csv")).unwrap_or(false);
        if !csv {
            continue;
        }
        if let Ok(z) = o.metadata().and_then(|m| m.modified()) {
            if en.as_ref().map(|(t, _)| z > *t).unwrap_or(true) {
                en = Some((z, yol));
            }
        }
    }
    Ok(en.map(|(_, y)| y))
}

/// Klasördeki en yeni CSV'yi okur. Özellik kapalıysa hata verir.
pub fn klasorden_oku(klasor: &Path) -> Result<(PathBuf, Vec<Urun>), Hata> {
    if !taa_ayar::ozellik_acik(OZELLIK, true) {
        return Err(Hata(taa_ayar::kapali_mesaji(OZELLIK)));
    }
    match en_yeni_csv(klasor)? {
        Some(y) => {
            let bytes = fs::read(&y).map_err(|e| Hata(format!("dosya okunamadı: {e}")))?;
            let u = urunleri_oku(&metne_cevir(&bytes))?;
            Ok((y, u))
        }
        None => {
            let xlsx = fs::read_dir(klasor)
                .map(|d| d.flatten().any(|o| o.path().extension().map(|e| e.eq_ignore_ascii_case("xlsx")).unwrap_or(false)))
                .unwrap_or(false);
            Err(Hata(if xlsx {
                "klasörde yalnız .xlsx var; şimdilik CSV olarak dışa aktarın (xlsx desteği sonra)".into()
            } else {
                "klasörde CSV dosyası yok".into()
            }))
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn fiyatlar() {
        assert_eq!(fiyat_kurus("1.299,90"), Some(129990));
        assert_eq!(fiyat_kurus("1299.9"), Some(129990));
        assert_eq!(fiyat_kurus("1299"), Some(129900));
        assert_eq!(fiyat_kurus("599,00 TL"), Some(59900));
        assert_eq!(fiyat_kurus("1,299.50"), Some(129950));
        assert_eq!(fiyat_kurus(""), None);
    }

    #[test]
    fn csv_tirnakli() {
        let r = csv_satirlari("a;\"b;c\";d\r\n1;\"x\"\"y\";3\n", ';');
        assert_eq!(r[0], vec!["a", "b;c", "d"]);
        assert_eq!(r[1], vec!["1", "x\"y", "3"]);
    }

    #[test]
    fn urun_okur() {
        let m = "\u{feff}Ürün Kodu;Barkod;Ürün Açıklaması;Renk;Beden;Satış Fiyatı;İndirimli Fiyat\n\
                 CEK01;8680001;Takım Elbise;Lacivert;50;4.999,90;3.999,00\n\
                 ;;boş;;;1;\n\
                 PAN02;8680002;Pantolon;Siyah;48;1.299,00;\n";
        let u = urunleri_oku(m).unwrap();
        assert_eq!(u.len(), 2);
        assert_eq!(u[0].fiyat, 499990);
        assert_eq!(u[0].indirimli, Some(399900));
        assert_eq!(u[1].indirimli, None);
        assert_eq!(u[1].ad, "Pantolon");
    }

    #[test]
    fn eksik_sutun() {
        assert!(urunleri_oku("a;b\n1;2\n").is_err());
        assert!(urunleri_oku("Kod;Ad\nx;y\n").unwrap_err().0.contains("fiyat"));
    }

    #[test]
    fn cp1254() {
        let b = [b'K', 0xFE, b'a', 0xFD];
        assert_eq!(metne_cevir(&b), "Kşaı");
    }

    #[test]
    fn klasor_en_yeni() {
        let d = std::env::temp_dir().join(format!("taa-nebim-test-{}", std::process::id()));
        fs::create_dir_all(&d).unwrap();
        fs::write(d.join("a.csv"), "Kod;Fiyat\nX1;10\n").unwrap();
        let (y, u) = klasorden_oku(&d).unwrap();
        assert!(y.ends_with("a.csv"));
        assert_eq!(u[0].fiyat, 1000);
        let _ = fs::remove_dir_all(&d);
    }
}
