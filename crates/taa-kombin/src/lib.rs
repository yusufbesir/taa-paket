//! taa-kombin (TaaKombin): takım rengine göre gömlek, kravat, ayakkabı ve kemer uyumu önerir.
//! Ürün kataloğu gerekmez; yalnız klasik erkek giyim renk kuralları (aşağıdaki tablo) kullanılır.
//! Kurallar bir öneridir, moda hükmü değildir; tablo Yusuf/mağaza deneyimine göre genişletilebilir.
//! Özellik anahtarı: "kombin" (varsayılan açık). Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

pub const OZELLIK: &str = "kombin";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kombin {
    pub gomlek: &'static str,
    pub kravat: &'static str,
    pub ayakkabi: &'static str,
    /// Kemer her zaman ayakkabıyla aynı renktir.
    pub kemer: &'static str,
}

struct Kural {
    takim: &'static str,
    kombinler: &'static [(&'static str, &'static str, &'static str)],
}

const TABLO: &[Kural] = &[
    Kural {
        takim: "lacivert",
        kombinler: &[
            ("beyaz", "bordo", "kahverengi"),
            ("açık mavi", "lacivert", "siyah"),
            ("beyaz", "gri", "siyah"),
            ("pembe", "lacivert", "kahverengi"),
            ("açık mavi", "bordo", "kahverengi"),
        ],
    },
    Kural {
        takim: "siyah",
        kombinler: &[
            ("beyaz", "siyah", "siyah"),
            ("beyaz", "gümüş gri", "siyah"),
            ("açık gri", "bordo", "siyah"),
            ("beyaz", "bordo", "siyah"),
        ],
    },
    Kural {
        takim: "gri",
        kombinler: &[
            ("beyaz", "lacivert", "siyah"),
            ("açık mavi", "bordo", "kahverengi"),
            ("beyaz", "bordo", "siyah"),
            ("pembe", "gri", "siyah"),
        ],
    },
    Kural {
        takim: "antrasit",
        kombinler: &[
            ("beyaz", "gümüş gri", "siyah"),
            ("açık mavi", "lacivert", "siyah"),
            ("beyaz", "bordo", "siyah"),
        ],
    },
    Kural {
        takim: "kahverengi",
        kombinler: &[
            ("krem", "bej", "kahverengi"),
            ("beyaz", "yeşil", "kahverengi"),
            ("açık mavi", "kahverengi", "kahverengi"),
        ],
    },
    Kural {
        takim: "bej",
        kombinler: &[
            ("beyaz", "lacivert", "kahverengi"),
            ("açık mavi", "kahverengi", "kahverengi"),
            ("krem", "bordo", "kahverengi"),
        ],
    },
    Kural {
        takim: "bordo",
        kombinler: &[
            ("beyaz", "siyah", "siyah"),
            ("beyaz", "gri", "siyah"),
            ("krem", "lacivert", "kahverengi"),
        ],
    },
];

/// Büyük/küçük harf ve Türkçe harf farkını giderir ("Lacivert", "LACİVERT" -> "lacivert").
pub fn normalize(s: &str) -> String {
    s.trim()
        .chars()
        .map(|c| match c {
            'İ' => 'i',
            'I' => 'ı',
            c => c.to_lowercase().next().unwrap_or(c),
        })
        .collect()
}

pub fn takim_renkleri() -> Vec<&'static str> {
    TABLO.iter().map(|k| k.takim).collect()
}

/// Takım rengi için kombin kartları; bilinmeyen renkte bilinen renkleri sayan hata verir.
pub fn oner(takim_rengi: &str) -> Result<Vec<Kombin>, String> {
    if !taa_ayar::ozellik_acik(OZELLIK, true) {
        return Err(taa_ayar::kapali_mesaji(OZELLIK));
    }
    let r = normalize(takim_rengi);
    let r = if r == "füme" {
        "antrasit".to_string()
    } else {
        r
    };
    TABLO
        .iter()
        .find(|k| k.takim == r)
        .map(|k| {
            k.kombinler
                .iter()
                .map(|&(g, kr, a)| Kombin {
                    gomlek: g,
                    kravat: kr,
                    ayakkabi: a,
                    kemer: a,
                })
                .collect()
        })
        .ok_or_else(|| {
            format!(
                "'{takim_rengi}' için kural yok; bilinen renkler: {}",
                takim_renkleri().join(", ")
            )
        })
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn lacivert_kartlari() {
        let k = oner("LACİVERT").unwrap();
        assert_eq!(k.len(), 5);
        assert!(k.iter().all(|x| x.kemer == x.ayakkabi));
    }

    #[test]
    fn fume_antrasit_sayilir() {
        assert_eq!(oner("Füme").unwrap(), oner("antrasit").unwrap());
    }

    #[test]
    fn bilinmeyen_renk() {
        let e = oner("turuncu").unwrap_err();
        assert!(e.contains("lacivert") && e.contains("turuncu"));
    }

    #[test]
    fn siyah_ayakkabi_kahve_kurali() {
        // siyah takımla kahverengi ayakkabı önerilmez
        assert!(oner("siyah").unwrap().iter().all(|k| k.ayakkabi == "siyah"));
    }

    #[test]
    fn her_takimin_en_az_uc_kombini_var() {
        for t in takim_renkleri() {
            assert!(oner(t).unwrap().len() >= 3, "{t}");
        }
    }
}
