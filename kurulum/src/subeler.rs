// Sahip: Yusuf. yazan: [Claude#kur-yerel-pencere 2026-11-15]; güncelleme: [Claude Sonnet 5.5, Kur, MRK01 biçimi, 2026-11-15]
//! Şube listesi: `kurulum/subeler.txt` (kod, slug, ad, il) gömülür; kurulum ekranında şube seçimi için. Saf mantık.
//! Şube kodu 3 büyük harf + 2 rakam (`MRK01`; docs/sube-sozlesmesi.md §3). `slug` eski okunur kimliktir
//! (`istanbul-merkez`): eski `sube.txt` içeriğini koda çevirmek için durur.

/// Exe'ye gömülen şube verisi.
pub const VERI: &str = include_str!("../subeler.txt");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sube {
    pub kod: String,
    pub slug: String,
    pub ad: String,
    pub il: String,
}

/// Şube kodu: tam 3 büyük ASCII harf + 2 rakam (taa-fis `sube_kodu_gecerli` ile aynı kural).
pub fn kod_gecerli(k: &str) -> bool {
    let b = k.as_bytes();
    b.len() == 5 && b[..3].iter().all(u8::is_ascii_uppercase) && b[3..].iter().all(u8::is_ascii_digit)
}

/// Eski okunur kimlik: küçük ASCII harf, rakam, '-'; boş değil, en çok 40 karakter.
pub fn slug_gecerli(k: &str) -> bool {
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
        let [kod, slug, ad, il] = p.as_slice() else {
            return Err(format!("{no}. satır: kod<TAB>slug<TAB>ad<TAB>il bekleniyordu"));
        };
        if !kod_gecerli(kod) {
            return Err(format!("{no}. satır: geçersiz şube kodu '{kod}' (3 büyük harf + 2 rakam olmalı)"));
        }
        if !slug_gecerli(slug) {
            return Err(format!("{no}. satır: geçersiz slug '{slug}'"));
        }
        if ad.is_empty() {
            return Err(format!("{no}. satır: ad boş"));
        }
        if l.iter().any(|x| x.kod == *kod || x.slug == *slug) {
            return Err(format!("{no}. satır: '{kod}' ya da '{slug}' iki kez var"));
        }
        l.push(Sube { kod: kod.to_string(), slug: slug.to_string(), ad: ad.to_string(), il: il.to_string() });
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

/// Kayıtlı değer koda çevrilir: zaten geçerli kodsa aynen, eski slug ise listedeki kodu, tanınmazsa `None`.
pub fn koda_cevir(l: &[Sube], deger: &str) -> Option<String> {
    let d = deger.trim();
    if kod_gecerli(d) {
        return Some(d.to_string());
    }
    l.iter().find(|s| s.slug == d).map(|s| s.kod.clone())
}

/// Fiş sunucusunun okuduğu dosyanın içeriği: `{"kod":"MRK01"}`. Geçersiz kod için `None`.
pub fn sube_json(kod: &str) -> Option<String> {
    kod_gecerli(kod).then(|| format!("{{\"kod\":\"{kod}\"}}\n"))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn gomulu_liste_on_bes_sube() {
        let l = coz(VERI).expect("subeler.txt çözülmeli");
        assert_eq!(l.len(), 15);
        assert_eq!(gomulu().len(), 15);
        assert_eq!(bul(&l, "ANK01").map(|s| s.il.as_str()), Some("Ankara"));
        assert!(bul(&l, "MRK01").unwrap().ad.contains("Outlet"));
        assert_eq!(bul(&l, "MRK01").unwrap().slug, "istanbul-merkez");
        // Önek (ilk 3 harf) kesin ve her şubede ayrı.
        let mut onekler: Vec<&str> = l.iter().map(|s| &s.kod[..3]).collect();
        onekler.sort();
        onekler.dedup();
        assert_eq!(onekler.len(), 15);
        // Yalnız kod/slug/ad/il: telefon ya da adres gömülmez.
        assert!(!VERI.contains("+90"));
    }

    #[test]
    fn cozum_hatalari() {
        assert_eq!(
            coz("# yorum\n\nAAA01\ta\tA\tİl\r\n").unwrap()[0],
            Sube { kod: "AAA01".into(), slug: "a".into(), ad: "A".into(), il: "İl".into() }
        );
        assert!(coz("AAA01\ta\tA").unwrap_err().contains("bekleniyordu"));
        assert!(coz("a\ta\tA\tB").unwrap_err().contains("geçersiz şube kodu"));
        assert!(coz("AAA01\tA B\tA\tB").unwrap_err().contains("slug"));
        assert!(coz("AAA01\ta\t\tB").unwrap_err().contains("boş"));
        assert!(coz("AAA01\ta\tA\tB\nAAA01\tb\tC\tD").unwrap_err().contains("iki kez"));
        assert!(coz("AAA01\ta\tA\tB\nBBB01\ta\tC\tD").unwrap_err().contains("iki kez"));
    }

    #[test]
    fn kod_kurali() {
        for ok in ["MRK01", "PND01", "ABC99"] {
            assert!(kod_gecerli(ok), "{ok}");
        }
        for kotu in ["", "mrk01", "MRK1", "MRK001", "MR01", "MRK0A", "İST01", "istanbul-merkez", "MRK01 "] {
            assert!(!kod_gecerli(kotu), "{kotu}");
        }
        assert!(slug_gecerli("van-2") && !slug_gecerli("") && !slug_gecerli("a b") && !slug_gecerli("A"));
    }

    #[test]
    fn eski_deger_koda_cevrilir() {
        let l = coz(VERI).unwrap();
        assert_eq!(koda_cevir(&l, "istanbul-merkez").as_deref(), Some("MRK01"));
        assert_eq!(koda_cevir(&l, "  ankara\n").as_deref(), Some("ANK01"));
        assert_eq!(koda_cevir(&l, "PND01").as_deref(), Some("PND01"));
        assert_eq!(koda_cevir(&l, "yok-sube"), None);
    }

    #[test]
    fn sube_json_biçimi() {
        assert_eq!(sube_json("MRK01").as_deref(), Some("{\"kod\":\"MRK01\"}\n"));
        assert_eq!(sube_json("mrk01"), None);
    }
}
