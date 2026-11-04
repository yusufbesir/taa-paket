//! taa-rehber: e-posta rehberi (CSV) okur, doğrular, hiyerarşi ağacı ve düz liste üretir. Salt okunur.
//! Özellik anahtarı: "rehber" (varsayılan açık). Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use std::collections::{BTreeMap, BTreeSet};

pub const OZELLIK: &str = "rehber";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kisi {
    pub ad: String,
    pub eposta: String,
    pub unvan: String,
    pub bolum: String,
    /// Yöneticinin e-postası (boş = en üst).
    pub yonetici: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sorun {
    GecersizEposta(String),
    TekrarEposta(String),
    YoneticiYok { kisi: String, yonetici: String },
    Dongu(String),
}

impl std::fmt::Display for Sorun {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Sorun::GecersizEposta(e) => write!(f, "geçersiz e-posta: {e}"),
            Sorun::TekrarEposta(e) => write!(f, "tekrar eden e-posta: {e}"),
            Sorun::YoneticiYok { kisi, yonetici } => {
                write!(f, "{kisi}: yönetici rehberde yok ({yonetici})")
            }
            Sorun::Dongu(e) => write!(f, "yönetici döngüsü: {e}"),
        }
    }
}

pub fn eposta_gecerli(s: &str) -> bool {
    let Some((yerel, alan)) = s.split_once('@') else {
        return false;
    };
    !yerel.is_empty()
        && !s.chars().any(char::is_whitespace)
        && !alan.contains('@')
        && alan.contains('.')
        && !alan.starts_with('.')
        && !alan.ends_with('.')
}

fn katla(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .replace(['ı', 'İ'], "i")
        .replace('ş', "s")
        .replace('ğ', "g")
        .replace('ü', "u")
        .replace('ö', "o")
        .replace('ç', "c")
}

pub fn csv_oku(metin: &str) -> Result<Vec<Kisi>, String> {
    if !taa_ayar::ozellik_acik(OZELLIK, true) {
        return Err(taa_ayar::kapali_mesaji(OZELLIK));
    }
    let ilk = metin
        .trim_start_matches('\u{feff}')
        .lines()
        .next()
        .unwrap_or("");
    let ayirac = [';', '\t', ',']
        .into_iter()
        .max_by_key(|c| ilk.matches(*c).count())
        .unwrap_or(';');
    let satirlar = taa_nebim::csv_satirlari(metin.trim_start_matches('\u{feff}'), ayirac);
    let (bas, govde) = satirlar.split_first().ok_or("dosya boş")?;
    let b: Vec<String> = bas.iter().map(|x| katla(x)).collect();
    let bul = |adaylar: &[&str]| b.iter().position(|x| adaylar.contains(&x.as_str()));
    let ad = bul(&["ad", "adsoyad", "isim", "adisoyadi"]).ok_or("ad sütunu yok")?;
    let ep = bul(&["eposta", "email", "mail", "epostaadresi"]).ok_or("e-posta sütunu yok")?;
    let (un, bo, yo) = (
        bul(&["unvan", "gorev"]),
        bul(&["bolum", "departman"]),
        bul(&["yonetici", "yoneticieposta", "bagliolduguyonetici"]),
    );
    let al = |s: &Vec<String>, i: Option<usize>| {
        i.and_then(|i| s.get(i))
            .map(|x| x.trim().to_string())
            .unwrap_or_default()
    };
    Ok(govde
        .iter()
        .filter(|s| s.iter().any(|x| !x.trim().is_empty()))
        .map(|s| Kisi {
            ad: al(s, Some(ad)),
            eposta: al(s, Some(ep)).to_lowercase(),
            unvan: al(s, un),
            bolum: al(s, bo),
            yonetici: al(s, yo).to_lowercase(),
        })
        .collect())
}

pub fn dogrula(kisiler: &[Kisi]) -> Vec<Sorun> {
    let mut s = Vec::new();
    let mut gorulen = BTreeSet::new();
    for k in kisiler {
        if !eposta_gecerli(&k.eposta) {
            s.push(Sorun::GecersizEposta(k.eposta.clone()));
        } else if !gorulen.insert(k.eposta.clone()) {
            s.push(Sorun::TekrarEposta(k.eposta.clone()));
        }
    }
    let var: BTreeSet<&str> = kisiler.iter().map(|k| k.eposta.as_str()).collect();
    let yon: BTreeMap<&str, &str> = kisiler
        .iter()
        .map(|k| (k.eposta.as_str(), k.yonetici.as_str()))
        .collect();
    for k in kisiler {
        if !k.yonetici.is_empty() && !var.contains(k.yonetici.as_str()) {
            s.push(Sorun::YoneticiYok {
                kisi: k.eposta.clone(),
                yonetici: k.yonetici.clone(),
            });
        }
        // döngü: yönetici zincirinde kendine dönme
        let (mut cur, mut adim) = (k.yonetici.as_str(), 0);
        while !cur.is_empty() && adim <= kisiler.len() {
            if cur == k.eposta {
                s.push(Sorun::Dongu(k.eposta.clone()));
                break;
            }
            cur = yon.get(cur).copied().unwrap_or("");
            adim += 1;
        }
    }
    s
}

/// Girintili hiyerarşi metni. Yöneticisi bulunmayan ya da döngüde kalanlar en üstte gösterilir.
pub fn hiyerarsi_metni(kisiler: &[Kisi]) -> String {
    let var: BTreeSet<&str> = kisiler.iter().map(|k| k.eposta.as_str()).collect();
    let mut cocuk: BTreeMap<&str, Vec<&Kisi>> = BTreeMap::new();
    for k in kisiler {
        if !k.yonetici.is_empty() && var.contains(k.yonetici.as_str()) && k.yonetici != k.eposta {
            cocuk.entry(k.yonetici.as_str()).or_default().push(k);
        }
    }
    let mut cikti = String::new();
    let mut yazildi = BTreeSet::new();
    fn yaz<'a>(
        k: &'a Kisi,
        derin: usize,
        cocuk: &BTreeMap<&str, Vec<&'a Kisi>>,
        o: &mut String,
        y: &mut BTreeSet<&'a str>,
    ) {
        if !y.insert(k.eposta.as_str()) {
            return;
        }
        o.push_str(&format!(
            "{}{} <{}>{}\n",
            "  ".repeat(derin),
            k.ad,
            k.eposta,
            if k.unvan.is_empty() {
                String::new()
            } else {
                format!(" – {}", k.unvan)
            }
        ));
        if let Some(c) = cocuk.get(k.eposta.as_str()) {
            for x in c {
                yaz(x, derin + 1, cocuk, o, y);
            }
        }
    }
    for k in kisiler
        .iter()
        .filter(|k| k.yonetici.is_empty() || !var.contains(k.yonetici.as_str()))
    {
        yaz(k, 0, &cocuk, &mut cikti, &mut yazildi);
    }
    for k in kisiler {
        yaz(k, 0, &cocuk, &mut cikti, &mut yazildi); // döngüde kalanlar
    }
    cikti
}

#[cfg(test)]
mod testler {
    use super::*;

    const CSV: &str = "Ad Soyad;E-posta;Ünvan;Bölüm;Yönetici\nAyşe Kaya;ayse@taa.com.tr;Müdür;Satış;\nMehmet Can;Mehmet@taa.com.tr;Satış Uzmanı;Satış;ayse@taa.com.tr\nVeli Er;veli@taa.com.tr;Stajyer;Satış;mehmet@taa.com.tr\n";

    #[test]
    fn okur_ve_agac() {
        let k = csv_oku(CSV).unwrap();
        assert_eq!(k.len(), 3);
        assert_eq!(k[1].eposta, "mehmet@taa.com.tr");
        assert!(dogrula(&k).is_empty());
        let m = hiyerarsi_metni(&k);
        assert!(m.starts_with("Ayşe Kaya"));
        assert!(m.contains("\n  Mehmet Can"));
        assert!(m.contains("\n    Veli Er"));
    }

    #[test]
    fn sorunlar() {
        let mut k = csv_oku(CSV).unwrap();
        k.push(Kisi {
            ad: "X".into(),
            eposta: "bozuk".into(),
            unvan: "".into(),
            bolum: "".into(),
            yonetici: "yok@taa.com.tr".into(),
        });
        k.push(Kisi {
            ad: "Y".into(),
            eposta: "veli@taa.com.tr".into(),
            unvan: "".into(),
            bolum: "".into(),
            yonetici: "".into(),
        });
        let s = dogrula(&k);
        assert!(s.contains(&Sorun::GecersizEposta("bozuk".into())));
        assert!(s.contains(&Sorun::TekrarEposta("veli@taa.com.tr".into())));
        assert!(s.iter().any(|x| matches!(x, Sorun::YoneticiYok { .. })));
    }

    #[test]
    fn dongu_bulunur_ve_agacta_kaybolmaz() {
        let mk = |e: &str, y: &str| Kisi {
            ad: e.into(),
            eposta: e.into(),
            unvan: "".into(),
            bolum: "".into(),
            yonetici: y.into(),
        };
        let k = vec![mk("a@x.com", "b@x.com"), mk("b@x.com", "a@x.com")];
        assert!(dogrula(&k).iter().any(|s| matches!(s, Sorun::Dongu(_))));
        assert_eq!(hiyerarsi_metni(&k).lines().count(), 2);
    }

    #[test]
    fn eposta() {
        assert!(eposta_gecerli("a@b.co"));
        assert!(!eposta_gecerli("a@b"));
        assert!(!eposta_gecerli("a b@c.com"));
        assert!(!eposta_gecerli("@c.com"));
    }

    #[test]
    fn sutun_yok() {
        assert!(csv_oku("Ad;Telefon\nx;1\n").is_err());
    }
}
