// Sahip: Yusuf. yazan: [Claude#kur-yerel-pencere 2026-11-15]
//! Şube listesi: `kurulum/subeler.txt` (kod, ad, il) gömülür; kurulum ekranında şube seçimi için. Saf mantık.

/// Exe'ye gömülen şube verisi.
pub const VERI: &str = include_str!("../subeler.txt");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sube {
    pub kod: String,
    pub ad: String,
    pub il: String,
}

/// Şube kodu: küçük ASCII harf, rakam, '-'; boş değil, en çok 40 karakter.
pub fn kod_gecerli(k: &str) -> bool {
    !k.is_empty() && k.len() <= 40 && k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Listeyi çözer. Hata: satır numarası ve neden.
pub fn coz(veri: &str) -> Result<Vec<Sube>, String> {
    let mut l: Vec<Sube> = vec![];
    for (no, ham) in veri.lines().enumerate() {
        let no = no + 1;
        let s = ham.trim_end_matches('\r');
        if s.trim().is_empty() || s.trim_start().starts_with('#') {
            continue;
        }
        let p: Vec<&str> = s.split('\t').map(str::trim).collect();
        let [kod, ad, il] = p.as_slice() else {
            return Err(format!("{no}. satır: kod<TAB>ad<TAB>il bekleniyordu"));
        };
        if !kod_gecerli(kod) {
            return Err(format!("{no}. satır: geçersiz şube kodu '{kod}'"));
        }
        if ad.is_empty() {
            return Err(format!("{no}. satır: ad boş"));
        }
        if l.iter().any(|x| x.kod == *kod) {
            return Err(format!("{no}. satır: '{kod}' iki kez var"));
        }
        l.push(Sube { kod: kod.to_string(), ad: ad.to_string(), il: il.to_string() });
    }
    Ok(l)
}

/// Gömülü liste; bozuksa boş (şube adımı elle yapılacak olarak gösterilir, kurulum durmaz).
pub fn gomulu() -> Vec<Sube> {
    coz(VERI).unwrap_or_default()
}

pub fn bul<'a>(l: &'a [Sube], kod: &str) -> Option<&'a Sube> {
    l.iter().find(|s| s.kod == kod)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn gomulu_liste_on_bes_sube() {
        let l = coz(VERI).expect("subeler.txt çözülmeli");
        assert_eq!(l.len(), 15);
        assert_eq!(gomulu().len(), 15);
        assert_eq!(bul(&l, "ankara").map(|s| s.il.as_str()), Some("Ankara"));
        assert!(bul(&l, "istanbul-merkez").unwrap().ad.contains("Outlet"));
        // Yalnız kod/ad/il: telefon ya da adres gömülmez.
        assert!(!VERI.contains("+90"));
    }

    #[test]
    fn cozum_hatalari() {
        assert_eq!(coz("# yorum\n\na\tA\tİl\r\n").unwrap()[0], Sube { kod: "a".into(), ad: "A".into(), il: "İl".into() });
        assert!(coz("a\tA").unwrap_err().contains("bekleniyordu"));
        assert!(coz("A\tA\tB").unwrap_err().contains("geçersiz"));
        assert!(coz("a\t\tB").unwrap_err().contains("boş"));
        assert!(coz("a\tA\tB\na\tC\tD").unwrap_err().contains("iki kez"));
        assert!(!kod_gecerli("") && !kod_gecerli("a b") && kod_gecerli("van-2"));
    }
}
