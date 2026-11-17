// Sahip: Yusuf. yazan: [Claude#kur-yerel-pencere 2026-11-15] değiştiren: [Claude#kurucu-arayuz 2026-11-17]
//! Kurulum ekranındaki seçim (onay, rol, şube, isteğe bağlı programlar): tek ya da çoklu seçim, klavye imleci.
//! Saf mantık; pencere yalnız çizer ve tuş/tıklama iletir. Konsolda aynı model numaralı soruya döner.

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Secenek {
    pub etiket: String,
    pub aciklama: String,
    /// Aynı gruptakiler art arda gelir; değiştiği yerde başlık gösterilir (boş = başlıksız).
    pub grup: String,
}

impl Secenek {
    pub fn yeni(etiket: &str, aciklama: &str) -> Self {
        Secenek { etiket: etiket.to_string(), aciklama: aciklama.to_string(), grup: String::new() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secim {
    pub baslik: String,
    pub aciklama: String,
    pub secenekler: Vec<Secenek>,
    pub coklu: bool,
    /// Çoklu seçimde işaretliler; tek seçimde imlecin üstündeki seçilir.
    pub isaretli: Vec<bool>,
    pub imlec: usize,
    /// Onay düğmesi (Enter) ve vazgeç düğmesi (Esc) etiketleri.
    pub onay: String,
    pub vazgec: String,
}

impl Secim {
    pub fn tek(baslik: &str, aciklama: &str, secenekler: Vec<Secenek>, varsayilan: usize) -> Self {
        let n = secenekler.len();
        Secim {
            baslik: baslik.to_string(),
            aciklama: aciklama.to_string(),
            isaretli: vec![false; n],
            secenekler,
            coklu: false,
            imlec: varsayilan.min(n.saturating_sub(1)),
            onay: "Devam".into(),
            vazgec: "Atla".into(),
        }
    }

    pub fn coklu(baslik: &str, aciklama: &str, secenekler: Vec<Secenek>) -> Self {
        let mut s = Secim::tek(baslik, aciklama, secenekler, 0);
        s.coklu = true;
        s
    }

    pub fn dugmeler(mut self, onay: &str, vazgec: &str) -> Self {
        self.onay = onay.to_string();
        self.vazgec = vazgec.to_string();
        self
    }

    pub fn yukari(&mut self) {
        self.imlec = self.imlec.saturating_sub(1);
    }

    pub fn asagi(&mut self) {
        if self.imlec + 1 < self.secenekler.len() {
            self.imlec += 1;
        }
    }

    /// Boşluk: çoklu seçimde imlecin üstündekini aç/kapat.
    pub fn degistir(&mut self) {
        if self.coklu {
            if let Some(i) = self.isaretli.get_mut(self.imlec) {
                *i = !*i;
            }
        }
    }

    /// Fareyle tıklama: imleci taşır; çoklu seçimde işareti değiştirir.
    pub fn tikla(&mut self, i: usize) {
        if i < self.secenekler.len() {
            self.imlec = i;
            self.degistir();
        }
    }

    /// Boş olmayan grup adları, ilk görünme sırasıyla (İlmek'te hap çipler için).
    pub fn gruplar(&self) -> Vec<String> {
        let mut v: Vec<String> = vec![];
        for x in &self.secenekler {
            if !x.grup.is_empty() && !v.contains(&x.grup) {
                v.push(x.grup.clone());
            }
        }
        v
    }

    /// Gruptaki işaretli ve toplam seçenek sayısı.
    pub fn grup_sayisi(&self, grup: &str) -> (usize, usize) {
        let mut n = (0, 0);
        for (x, i) in self.secenekler.iter().zip(&self.isaretli) {
            if x.grup == grup {
                n.1 += 1;
                if *i {
                    n.0 += 1;
                }
            }
        }
        n
    }

    /// Hap çip: çoklu seçimde gruptakilerin hepsi işaretliyse hepsini kaldırır, değilse hepsini işaretler;
    /// imleç grubun ilk seçeneğine gider. Tek seçimde ya da bilinmeyen grupta bir şey yapmaz.
    pub fn grup_degistir(&mut self, grup: &str) {
        if !self.coklu || grup.is_empty() {
            return;
        }
        let (isaretli, toplam) = self.grup_sayisi(grup);
        if toplam == 0 {
            return;
        }
        let yeni = isaretli < toplam;
        for (x, i) in self.secenekler.iter().zip(self.isaretli.iter_mut()) {
            if x.grup == grup {
                *i = yeni;
            }
        }
        if let Some(k) = self.secenekler.iter().position(|x| x.grup == grup) {
            self.imlec = k;
        }
    }

    /// Onaylanınca dönen seçim: tek seçimde imleç, çoklu seçimde işaretliler (boş olabilir).
    pub fn sonuc(&self) -> Vec<usize> {
        if self.coklu {
            self.isaretli.iter().enumerate().filter(|(_, x)| **x).map(|(i, _)| i).collect()
        } else if self.secenekler.is_empty() {
            vec![]
        } else {
            vec![self.imlec]
        }
    }

    /// Konsol cevabını çözer: tek seçimde numara (boş = varsayılan imleç); çoklu seçimde "1,3", "H" (hepsi), boş (hiçbiri).
    pub fn konsol_cevabi(&self, cevap: &str) -> Vec<usize> {
        let c = cevap.trim();
        let n = self.secenekler.len();
        if self.coklu {
            if c.eq_ignore_ascii_case("h") {
                return (0..n).collect();
            }
            let mut v: Vec<usize> = c
                .split([',', ' '])
                .filter_map(|x| x.trim().parse::<usize>().ok())
                .filter(|x| (1..=n).contains(x))
                .map(|x| x - 1)
                .collect();
            v.sort();
            v.dedup();
            v
        } else {
            match c.parse::<usize>() {
                Ok(x) if (1..=n).contains(&x) => vec![x - 1],
                _ => self.sonuc(),
            }
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn s3() -> Vec<Secenek> {
        vec![Secenek::yeni("A", ""), Secenek::yeni("B", "b"), Secenek::yeni("C", "")]
    }

    #[test]
    fn tek_secim_imleci() {
        let mut s = Secim::tek("Rol", "", s3(), 9);
        assert_eq!(s.imlec, 2, "varsayılan sınıra kırpılır");
        s.asagi();
        assert_eq!(s.imlec, 2);
        s.yukari();
        s.yukari();
        s.yukari();
        assert_eq!(s.sonuc(), vec![0]);
        s.degistir();
        assert_eq!(s.isaretli, vec![false; 3], "tek seçimde işaret yok");
        s.tikla(1);
        s.tikla(7);
        assert_eq!(s.sonuc(), vec![1]);
        assert_eq!(Secim::tek("x", "", vec![], 0).sonuc(), Vec::<usize>::new());
    }

    #[test]
    fn coklu_secim() {
        let mut s = Secim::coklu("Programlar", "", s3()).dugmeler("Kur", "Hiçbiri");
        assert_eq!((s.onay.as_str(), s.vazgec.as_str()), ("Kur", "Hiçbiri"));
        assert!(s.sonuc().is_empty());
        s.degistir();
        s.asagi();
        s.asagi();
        s.degistir();
        assert_eq!(s.sonuc(), vec![0, 2]);
        s.tikla(0);
        assert_eq!(s.sonuc(), vec![2]);
    }

    #[test]
    fn grup_cipleri() {
        let g = |e: &str, grup: &str| Secenek { etiket: e.into(), aciklama: String::new(), grup: grup.into() };
        let mut s = Secim::coklu("P", "", vec![g("a", "Mağaza"), g("b", "Mağaza"), g("c", "Ofis"), g("d", "")]);
        assert_eq!(s.gruplar(), vec!["Mağaza".to_string(), "Ofis".to_string()]);
        s.tikla(1);
        assert_eq!(s.grup_sayisi("Mağaza"), (1, 2));
        s.grup_degistir("Mağaza");
        assert_eq!(s.sonuc(), vec![0, 1], "kısmi grup tamamlanır");
        assert_eq!(s.imlec, 0);
        s.grup_degistir("Mağaza");
        assert!(s.sonuc().is_empty(), "tam grup boşalır");
        s.grup_degistir("Ofis");
        assert_eq!((s.sonuc(), s.imlec), (vec![2], 2));
        s.grup_degistir("yok");
        s.grup_degistir("");
        assert_eq!(s.sonuc(), vec![2]);
        let mut t = Secim::tek("x", "", vec![g("a", "G")], 0);
        t.grup_degistir("G");
        assert_eq!(t.isaretli, vec![false]);
    }

    #[test]
    fn konsol_cevaplari() {
        let t = Secim::tek("x", "", s3(), 0);
        assert_eq!(t.konsol_cevabi(""), vec![0]);
        assert_eq!(t.konsol_cevabi(" 2 "), vec![1]);
        assert_eq!(t.konsol_cevabi("9"), vec![0]);
        let c = Secim::coklu("x", "", s3());
        assert_eq!(c.konsol_cevabi("h"), vec![0, 1, 2]);
        assert_eq!(c.konsol_cevabi("3, 1 1 0 9 x"), vec![0, 2]);
        assert!(c.konsol_cevabi("").is_empty());
    }
}
