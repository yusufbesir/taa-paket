//! Sözlükle kısıtlı eşleme (Tutaa `motor/PROTOKOL.md` durum kuralıyla aynı): okunan metin geçerli kodlar listesine
//! en yakın koda bağlanır. Kabul yalnız d1 ≤ 1, d2 > d1 ve güven ≥ 0.80 ise; aksi "sor"; aday yoksa "bos".
//! Hiçbir kod sessizce kabul edilmez.

pub const KABUL_ESIGI: f64 = 0.80;

#[derive(Debug, Clone, PartialEq)]
pub struct Eslesme {
    pub ham: String,
    pub secilen: Option<String>,
    /// En yakın en çok 3 aday: (kod, mesafe).
    pub aday: Vec<(String, usize)>,
    pub guven: f64,
    pub durum: &'static str,
}

pub fn temizle(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_whitespace())
        .flat_map(|c| c.to_uppercase())
        .collect()
}

pub fn mesafe(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut onceki: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut simdiki = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let m = usize::from(a[i - 1] != b[j - 1]);
            simdiki[j] = (onceki[j] + 1)
                .min(simdiki[j - 1] + 1)
                .min(onceki[j - 1] + m);
        }
        onceki = simdiki;
    }
    onceki[b.len()]
}

pub fn kisitli(ham: &str, sozluk: &[String], guven: f64) -> Eslesme {
    let t = temizle(ham);
    let mut m = Eslesme {
        ham: ham.to_string(),
        secilen: None,
        aday: vec![],
        guven,
        durum: "bos",
    };
    if t.is_empty() || sozluk.is_empty() {
        return m;
    }
    let mut hepsi: Vec<(String, usize)> = sozluk
        .iter()
        .map(|k| (k.clone(), mesafe(&t, &temizle(k))))
        .collect();
    hepsi.sort_by(|x, y| x.1.cmp(&y.1).then_with(|| x.0.cmp(&y.0)));
    hepsi.truncate(3);
    let d1 = hepsi[0].1;
    let d2 = hepsi.get(1).map_or(usize::MAX, |x| x.1);
    m.aday = hepsi.into_iter().filter(|x| x.1 <= 2).collect();
    if m.aday.is_empty() {
        return m;
    }
    if d1 <= 1 && d2 > d1 && guven >= KABUL_ESIGI {
        m.secilen = Some(m.aday[0].0.clone());
        m.durum = "kabul";
    } else {
        m.durum = "sor";
    }
    m
}

#[cfg(test)]
mod testler {
    use super::*;
    fn s(l: &[&str]) -> Vec<String> {
        l.iter().map(|x| x.to_string()).collect()
    }
    #[test]
    fn birebir_kabul() {
        let m = kisitli("t14525", &s(&["T14525", "T20236-01"]), 0.93);
        assert_eq!((m.durum, m.secilen.as_deref()), ("kabul", Some("T14525")));
    }
    #[test]
    fn belirsiz_sor() {
        // iki aday da eşit uzaklıkta: sessiz kabul yok
        let m = kisitli("T14535", &s(&["T14525", "T14575"]), 0.99);
        assert_eq!(m.durum, "sor");
        assert!(m.secilen.is_none());
    }
    #[test]
    fn dusuk_guven_sor_ve_bos() {
        assert_eq!(kisitli("T14525", &s(&["T14525"]), 0.5).durum, "sor");
        assert_eq!(kisitli("", &s(&["T14525"]), 1.0).durum, "bos");
        assert_eq!(kisitli("ZZZZZZ", &s(&["T14525"]), 1.0).durum, "bos");
    }
}
