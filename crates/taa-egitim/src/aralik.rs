//! Parça parça indirme için HTTP Range hesaplayıcı (RFC 9110 §14). Ağ yok: yalnız başlık metni ve sayı hesabı.
//! Tek aralık desteklenir; çoklu ya da bozuk `Range` başlığı yok sayılır (tam içerik, 200); karşılanamayan aralık 416.
//! Sahip: Yusuf. yazan: [Claude#egitim 2026-11-26]

/// Kapsayıcı bayt aralığı `[bas, son]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Aralik {
    pub bas: u64,
    pub son: u64,
}

impl Aralik {
    pub fn uzunluk(&self) -> u64 {
        self.son - self.bas + 1
    }

    /// Yanıt başlığı: `Content-Range: bytes bas-son/toplam` (206).
    pub fn icerik_araligi(&self, toplam: u64) -> String {
        format!("bytes {}-{}/{}", self.bas, self.son, toplam)
    }

    /// İstemci tarafı istek başlığı: `Range: bytes=bas-son`.
    pub fn istek_basligi(&self) -> String {
        format!("bytes={}-{}", self.bas, self.son)
    }
}

/// 416 yanıtının `Content-Range` değeri: `bytes */toplam`.
pub fn karsilanamaz_basligi(toplam: u64) -> String {
    format!("bytes */{toplam}")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AralikSonuc {
    /// Başlık yok, bozuk ya da çoklu aralık: tam içeriği 200 ile ver.
    Tam,
    /// 206 ile bu aralığı ver.
    Kismi(Aralik),
    /// 416 ile ver (`karsilanamaz_basligi`).
    Karsilanamaz,
}

/// Yalnız rakamlardan oluşan sayı; taşarsa `u64::MAX`.
fn sayi(s: &str) -> Option<u64> {
    if s.is_empty() || !s.bytes().all(|c| c.is_ascii_digit()) {
        return None;
    }
    Some(s.parse::<u64>().unwrap_or(u64::MAX))
}

/// `Range` başlığını `toplam` baytlık içeriğe göre çözer.
pub fn aralik_coz(baslik: Option<&str>, toplam: u64) -> AralikSonuc {
    let Some(b) = baslik else {
        return AralikSonuc::Tam;
    };
    let Some(spec) = b.trim().strip_prefix("bytes=") else {
        return AralikSonuc::Tam;
    };
    if spec.contains(',') {
        return AralikSonuc::Tam;
    }
    let Some((a, z)) = spec.trim().split_once('-') else {
        return AralikSonuc::Tam;
    };
    let (a, z) = (a.trim(), z.trim());
    match (a.is_empty(), z.is_empty()) {
        (true, true) => AralikSonuc::Tam,
        // sondan n bayt
        (true, false) => {
            let Some(n) = sayi(z) else {
                return AralikSonuc::Tam;
            };
            if n == 0 || toplam == 0 {
                return AralikSonuc::Karsilanamaz;
            }
            AralikSonuc::Kismi(Aralik {
                bas: toplam.saturating_sub(n),
                son: toplam - 1,
            })
        }
        // bas-
        (false, true) => {
            let Some(bas) = sayi(a) else {
                return AralikSonuc::Tam;
            };
            if bas >= toplam {
                return AralikSonuc::Karsilanamaz;
            }
            AralikSonuc::Kismi(Aralik {
                bas,
                son: toplam - 1,
            })
        }
        // bas-son
        (false, false) => {
            let (Some(bas), Some(son)) = (sayi(a), sayi(z)) else {
                return AralikSonuc::Tam;
            };
            if son < bas {
                return AralikSonuc::Tam;
            }
            if bas >= toplam {
                return AralikSonuc::Karsilanamaz;
            }
            AralikSonuc::Kismi(Aralik {
                bas,
                son: son.min(toplam - 1),
            })
        }
    }
}

/// İstemci için eşit parçalara bölme planı (son parça kısa olabilir).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParcaPlani {
    pub toplam: u64,
    pub parca_boyu: u64,
}

impl ParcaPlani {
    /// `parca_boyu` 0 ise `None`.
    pub fn yeni(toplam: u64, parca_boyu: u64) -> Option<Self> {
        (parca_boyu > 0).then_some(Self { toplam, parca_boyu })
    }

    pub fn parca_sayisi(&self) -> u64 {
        self.toplam.div_ceil(self.parca_boyu)
    }

    /// `i`. parça (0'dan); aralık dışıysa `None`.
    pub fn parca(&self, i: u64) -> Option<Aralik> {
        if i >= self.parca_sayisi() {
            return None;
        }
        let bas = i * self.parca_boyu;
        Some(Aralik {
            bas,
            son: bas.saturating_add(self.parca_boyu - 1).min(self.toplam - 1),
        })
    }

    /// Yarım kalan indirmeyi sürdürmek için sıradaki aralık: `inen` bayt alındıysa `inen`'den başlar.
    /// Hepsi indiyse (ya da `inen` toplamı aşıyorsa) `None`.
    pub fn devam(&self, inen: u64) -> Option<Aralik> {
        if inen >= self.toplam {
            return None;
        }
        Some(Aralik {
            bas: inen,
            son: inen
                .saturating_add(self.parca_boyu - 1)
                .min(self.toplam - 1),
        })
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn k(bas: u64, son: u64) -> AralikSonuc {
        AralikSonuc::Kismi(Aralik { bas, son })
    }

    #[test]
    fn baslik_yok_ya_da_bozuk_tam() {
        assert_eq!(aralik_coz(None, 100), AralikSonuc::Tam);
        for b in [
            "",
            "bytes",
            "bytes=",
            "bytes=-",
            "bytes=abc-5",
            "bytes=5-x",
            "items=0-5",
            "bytes=5",
            "bytes=+1-2",
            "bytes=9-3",
        ] {
            assert_eq!(aralik_coz(Some(b), 100), AralikSonuc::Tam, "{b}");
        }
    }

    #[test]
    fn cekirdek_durumlar() {
        assert_eq!(aralik_coz(Some("bytes=0-9"), 100), k(0, 9));
        assert_eq!(aralik_coz(Some("bytes=10-"), 100), k(10, 99));
        assert_eq!(aralik_coz(Some("bytes=-10"), 100), k(90, 99));
        assert_eq!(aralik_coz(Some(" bytes=0-0 "), 100), k(0, 0));
    }

    #[test]
    fn son_toplama_kirpilir() {
        assert_eq!(aralik_coz(Some("bytes=90-500"), 100), k(90, 99));
        assert_eq!(
            aralik_coz(Some("bytes=0-99999999999999999999999"), 100),
            k(0, 99)
        );
        assert_eq!(aralik_coz(Some("bytes=-500"), 100), k(0, 99));
    }

    #[test]
    fn karsilanamaz_durumlar() {
        assert_eq!(
            aralik_coz(Some("bytes=100-"), 100),
            AralikSonuc::Karsilanamaz
        );
        assert_eq!(
            aralik_coz(Some("bytes=100-120"), 100),
            AralikSonuc::Karsilanamaz
        );
        assert_eq!(aralik_coz(Some("bytes=-0"), 100), AralikSonuc::Karsilanamaz);
        assert_eq!(aralik_coz(Some("bytes=0-5"), 0), AralikSonuc::Karsilanamaz);
        assert_eq!(aralik_coz(Some("bytes=-5"), 0), AralikSonuc::Karsilanamaz);
        assert_eq!(
            aralik_coz(Some("bytes=99999999999999999999999-"), 100),
            AralikSonuc::Karsilanamaz
        );
    }

    #[test]
    fn coklu_aralik_yok_sayilir() {
        assert_eq!(aralik_coz(Some("bytes=0-1,5-6"), 100), AralikSonuc::Tam);
    }

    #[test]
    fn basliklar() {
        let a = Aralik { bas: 10, son: 19 };
        assert_eq!(a.uzunluk(), 10);
        assert_eq!(a.icerik_araligi(100), "bytes 10-19/100");
        assert_eq!(a.istek_basligi(), "bytes=10-19");
        assert_eq!(karsilanamaz_basligi(100), "bytes */100");
    }

    #[test]
    fn parca_plani_tam_ve_kisa_son_parca() {
        let p = ParcaPlani::yeni(25, 10).unwrap();
        assert_eq!(p.parca_sayisi(), 3);
        assert_eq!(p.parca(0), Some(Aralik { bas: 0, son: 9 }));
        assert_eq!(p.parca(2), Some(Aralik { bas: 20, son: 24 }));
        assert_eq!(p.parca(3), None);
        let t = ParcaPlani::yeni(20, 10).unwrap();
        assert_eq!(t.parca_sayisi(), 2);
        assert_eq!(t.parca(1), Some(Aralik { bas: 10, son: 19 }));
    }

    #[test]
    fn parca_plani_uc_durumlar() {
        assert!(ParcaPlani::yeni(10, 0).is_none());
        let bos = ParcaPlani::yeni(0, 10).unwrap();
        assert_eq!(bos.parca_sayisi(), 0);
        assert_eq!(bos.parca(0), None);
        assert_eq!(bos.devam(0), None);
        let b = ParcaPlani::yeni(u64::MAX, u64::MAX).unwrap();
        assert_eq!(b.parca_sayisi(), 1);
        assert_eq!(
            b.parca(0),
            Some(Aralik {
                bas: 0,
                son: u64::MAX - 1
            })
        );
    }

    #[test]
    fn parcalar_kesintisiz_ve_ortusmez() {
        let p = ParcaPlani::yeni(1234, 100).unwrap();
        let mut beklenen = 0;
        for i in 0..p.parca_sayisi() {
            let a = p.parca(i).unwrap();
            assert_eq!(a.bas, beklenen);
            beklenen = a.son + 1;
        }
        assert_eq!(beklenen, 1234);
    }

    #[test]
    fn devam_kaldigi_yerden() {
        let p = ParcaPlani::yeni(25, 10).unwrap();
        assert_eq!(p.devam(0), Some(Aralik { bas: 0, son: 9 }));
        assert_eq!(p.devam(13), Some(Aralik { bas: 13, son: 22 }));
        assert_eq!(p.devam(20), Some(Aralik { bas: 20, son: 24 }));
        assert_eq!(p.devam(25), None);
        assert_eq!(p.devam(99), None);
    }
}
