// Sahip: Yusuf. yazan: [Claude#taa-paket-T2 2026-11-05]
//! SAHIP.toml için düz TOML alt kümesi: `anahtar = "metin" | true | false | 123`, `#` yorum.
//! Tablo, dizi, çok satırlı metin yoktur; hata satır numarası (1'den) taşır.

use crate::VeriHata;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Deger {
    Metin(String),
    Mantik(bool),
    Sayi(i64),
}

fn hata(satir: usize, neden: &str) -> VeriHata {
    VeriHata::GecersizToml {
        satir,
        neden: neden.to_string(),
    }
}

fn anahtar_gecerli(a: &str) -> bool {
    !a.is_empty()
        && a.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Tırnaklı metni çözer; (metin, tırnaktan sonra kalan) döndürür.
fn metin_coz(s: &str, satir: usize) -> Result<(String, &str), VeriHata> {
    let mut cikti = String::new();
    let mut it = s.char_indices();
    it.next(); // açılış tırnağı
    while let Some((i, c)) = it.next() {
        match c {
            '"' => return Ok((cikti, &s[i + 1..])),
            '\\' => match it.next() {
                Some((_, '"')) => cikti.push('"'),
                Some((_, '\\')) => cikti.push('\\'),
                Some((_, 'n')) => cikti.push('\n'),
                Some((_, 't')) => cikti.push('\t'),
                Some((_, 'r')) => cikti.push('\r'),
                _ => return Err(hata(satir, "bilinmeyen kaçış dizisi")),
            },
            _ => cikti.push(c),
        }
    }
    Err(hata(satir, "metin kapanmadı (tek satır olmalı)"))
}

fn sonu_bos_mu(kalan: &str, satir: usize) -> Result<(), VeriHata> {
    let k = kalan.trim();
    if k.is_empty() || k.starts_with('#') {
        Ok(())
    } else {
        Err(hata(satir, "değerden sonra beklenmeyen karakter"))
    }
}

/// SAHIP.toml metnini çözer.
pub fn sahip_toml_coz(metin: &str) -> Result<BTreeMap<String, Deger>, VeriHata> {
    let mut harita = BTreeMap::new();
    for (i, ham) in metin.trim_start_matches('\u{feff}').lines().enumerate() {
        let satir = i + 1;
        let s = ham.trim();
        if s.is_empty() || s.starts_with('#') {
            continue;
        }
        if s.starts_with('[') {
            return Err(hata(satir, "tablo ve dizi desteklenmez"));
        }
        let (anahtar, deger) = s
            .split_once('=')
            .ok_or_else(|| hata(satir, "'anahtar = değer' biçimi bekleniyordu"))?;
        let anahtar = anahtar.trim();
        if !anahtar_gecerli(anahtar) {
            return Err(hata(satir, "geçersiz anahtar"));
        }
        let deger = deger.trim_start();
        let d =
            if deger.starts_with("\"\"\"") || deger.starts_with("'''") {
                return Err(hata(satir, "çok satırlı metin desteklenmez"));
            } else if deger.starts_with('"') {
                let (m, kalan) = metin_coz(deger, satir)?;
                sonu_bos_mu(kalan, satir)?;
                Deger::Metin(m)
            } else if deger.starts_with('[') || deger.starts_with('{') {
                return Err(hata(satir, "dizi ve satır içi tablo desteklenmez"));
            } else {
                let (belirtec, kalan) = match deger.find(['#', ' ', '\t']) {
                    Some(p) => deger.split_at(p),
                    None => (deger, ""),
                };
                sonu_bos_mu(kalan, satir)?;
                match belirtec {
                    "true" => Deger::Mantik(true),
                    "false" => Deger::Mantik(false),
                    "" => return Err(hata(satir, "değer yok")),
                    b => Deger::Sayi(b.parse::<i64>().map_err(|_| {
                        hata(satir, "değer metin, true/false ya da tam sayı olmalı")
                    })?),
                }
            };
        if harita.insert(anahtar.to_string(), d).is_some() {
            return Err(hata(satir, "anahtar tekrarlandı"));
        }
    }
    Ok(harita)
}

const SIRA: [&str; 6] = [
    "sema",
    "alan",
    "sahip",
    "gecici",
    "yazan_surum",
    "son_yazim",
];

fn satir_yaz(cikti: &mut String, a: &str, d: &Deger) {
    cikti.push_str(a);
    cikti.push_str(" = ");
    match d {
        Deger::Metin(m) => {
            cikti.push('"');
            for c in m.chars() {
                match c {
                    '"' => cikti.push_str("\\\""),
                    '\\' => cikti.push_str("\\\\"),
                    '\n' => cikti.push_str("\\n"),
                    '\r' => cikti.push_str("\\r"),
                    '\t' => cikti.push_str("\\t"),
                    c => cikti.push(c),
                }
            }
            cikti.push('"');
        }
        Deger::Mantik(b) => cikti.push_str(if *b { "true" } else { "false" }),
        Deger::Sayi(n) => cikti.push_str(&n.to_string()),
    }
    cikti.push('\n');
}

/// Haritayı yazar: bilinen SAHIP.toml anahtarları sabit sırada, kalanlar alfabetik. `sahip_toml_coz` ile gidiş-dönüş eşittir.
pub fn sahip_toml_yaz(harita: &BTreeMap<String, Deger>) -> String {
    let mut cikti = String::new();
    for a in SIRA {
        if let Some(d) = harita.get(a) {
            satir_yaz(&mut cikti, a, d);
        }
    }
    for (a, d) in harita {
        if !SIRA.contains(&a.as_str()) {
            satir_yaz(&mut cikti, a, d);
        }
    }
    cikti
}
