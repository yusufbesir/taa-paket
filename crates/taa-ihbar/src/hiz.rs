//! Hız sınırı: belirteç kovası. Kaynak (ör. Telegram numarası, IP) DÜZ METİN TUTULMAZ: kova anahtarı, yalnız bellekte
//! duran ve değiştirilebilen bir tuzla alınmış SHA-256 özetinin ilk 16 baytıdır; tuz yenilenince (ör. günlük) tüm kovalar
//! unutulur. Hiçbir şey diske yazılmaz. Kaynak başına kovaya ek olarak tüm hat için GENEL bir kova vardır (çok sayıda
//! farklı kaynakla taşırma saldırısına karşı). Tablo `EN_COK_KOVA` ile sınırlıdır; dolunca önce tam dolmuş (boştaki)
//! kovalar atılır, yetmezse en eski kova atılır. Tamsayı hesap, bin'de bir belirteç (milli) birimiyle.
//! Sahip: Yusuf. yazan: [Claude#ihbar 2026-11-26]

use sha2::{Digest, Sha256};
use std::collections::HashMap;

pub const EN_COK_KOVA: usize = 10_000;
pub const TUZ_EN_AZ: usize = 16;
const MILLI: u64 = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kova {
    kapasite: u64, // milli belirteç
    dolum_sn: u64, // 1 belirteç için saniye
    mevcut: u64,   // milli
    son: u64,      // unix sn
}

impl Kova {
    fn yeni(kapasite: u32, dolum_sn: u64, simdi: u64) -> Kova {
        let k = kapasite as u64 * MILLI;
        Kova {
            kapasite: k,
            dolum_sn,
            mevcut: k,
            son: simdi,
        }
    }

    fn doldur(&mut self, simdi: u64) {
        let gecen = simdi.saturating_sub(self.son);
        if gecen > 0 {
            let ek = gecen.saturating_mul(MILLI) / self.dolum_sn;
            self.mevcut = (self.mevcut.saturating_add(ek)).min(self.kapasite);
            self.son = simdi;
        }
    }

    /// Bir sonraki belirtece kaç saniye var (en az 1).
    fn bekleme(&self) -> u64 {
        let eksik = MILLI.saturating_sub(self.mevcut);
        (eksik * self.dolum_sn).div_ceil(MILLI).max(1)
    }

    fn dolu(&self) -> bool {
        self.mevcut >= self.kapasite
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Beklet {
    pub saniye: u64,
    /// `true`: tüm hat dolu (kaynaktan bağımsız).
    pub genel: bool,
}

pub struct HizSiniri {
    tuz: Vec<u8>,
    kaynak_kapasite: u32,
    kaynak_dolum_sn: u64,
    genel: Kova,
    kovalar: HashMap<[u8; 16], Kova>,
}

impl HizSiniri {
    pub fn yeni(
        tuz: &[u8],
        kaynak_kapasite: u32,
        kaynak_dolum_sn: u64,
        genel_kapasite: u32,
        genel_dolum_sn: u64,
        simdi: u64,
    ) -> Result<HizSiniri, String> {
        if tuz.len() < TUZ_EN_AZ {
            return Err(format!("tuz en az {TUZ_EN_AZ} bayt olmalı"));
        }
        if kaynak_kapasite == 0
            || genel_kapasite == 0
            || kaynak_dolum_sn == 0
            || genel_dolum_sn == 0
        {
            return Err("hız sınırı değerleri sıfır olamaz".into());
        }
        Ok(HizSiniri {
            tuz: tuz.to_vec(),
            kaynak_kapasite,
            kaynak_dolum_sn,
            genel: Kova::yeni(genel_kapasite, genel_dolum_sn, simdi),
            kovalar: HashMap::new(),
        })
    }

    /// Ayardan kurar (`Ayar` hız alanları).
    pub fn ayardan(a: &crate::ayar::Ayar, tuz: &[u8], simdi: u64) -> Result<HizSiniri, String> {
        HizSiniri::yeni(
            tuz,
            a.kaynak_kapasite,
            a.kaynak_dolum_sn,
            a.genel_kapasite,
            a.genel_dolum_sn,
            simdi,
        )
    }

    fn anahtar(&self, kaynak: &str) -> [u8; 16] {
        let mut h = Sha256::new();
        h.update(&self.tuz);
        h.update(kaynak.as_bytes());
        let d = h.finalize();
        let mut a = [0u8; 16];
        a.copy_from_slice(&d[..16]);
        a
    }

    /// Tuzu değiştirir ve tüm kaynak kovalarını unutur (kaynakla ilişki kesilir). Genel kova korunur.
    pub fn tuz_yenile(&mut self, yeni_tuz: &[u8]) -> Result<(), String> {
        if yeni_tuz.len() < TUZ_EN_AZ {
            return Err(format!("tuz en az {TUZ_EN_AZ} bayt olmalı"));
        }
        self.tuz = yeni_tuz.to_vec();
        self.kovalar.clear();
        Ok(())
    }

    pub fn kova_sayisi(&self) -> usize {
        self.kovalar.len()
    }

    fn yer_ac(&mut self, simdi: u64) {
        if self.kovalar.len() < EN_COK_KOVA {
            return;
        }
        for k in self.kovalar.values_mut() {
            k.doldur(simdi);
        }
        self.kovalar.retain(|_, k| !k.dolu());
        if self.kovalar.len() >= EN_COK_KOVA {
            if let Some(en_eski) = self
                .kovalar
                .iter()
                .min_by_key(|(_, k)| k.son)
                .map(|(a, _)| *a)
            {
                self.kovalar.remove(&en_eski);
            }
        }
    }

    /// Bir ihbar girişimi için belirteç harcar. Reddedilirse genel kovadan da harcanmaz.
    pub fn dene(&mut self, kaynak: &str, simdi: u64) -> Result<(), Beklet> {
        self.genel.doldur(simdi);
        if self.genel.mevcut < MILLI {
            return Err(Beklet {
                saniye: self.genel.bekleme(),
                genel: true,
            });
        }
        let a = self.anahtar(kaynak);
        if !self.kovalar.contains_key(&a) {
            self.yer_ac(simdi);
        }
        let (kap, dolum) = (self.kaynak_kapasite, self.kaynak_dolum_sn);
        let k = self
            .kovalar
            .entry(a)
            .or_insert_with(|| Kova::yeni(kap, dolum, simdi));
        k.doldur(simdi);
        if k.mevcut < MILLI {
            return Err(Beklet {
                saniye: k.bekleme(),
                genel: false,
            });
        }
        k.mevcut -= MILLI;
        self.genel.mevcut -= MILLI;
        Ok(())
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    const TUZ: &[u8] = b"0123456789abcdef-tuz";

    fn hs(kk: u32, kd: u64, gk: u32, gd: u64) -> HizSiniri {
        HizSiniri::yeni(TUZ, kk, kd, gk, gd, 1000).unwrap()
    }

    #[test]
    fn kova_bosalir_ve_dolar() {
        let mut h = hs(2, 100, 100, 1);
        assert!(h.dene("a", 1000).is_ok());
        assert!(h.dene("a", 1000).is_ok());
        let e = h.dene("a", 1000).unwrap_err();
        assert!(!e.genel);
        assert_eq!(e.saniye, 100);
        assert!(h.dene("a", 1050).is_err());
        assert_eq!(h.dene("a", 1050).unwrap_err().saniye, 50);
        assert!(h.dene("a", 1100).is_ok());
        // Başka kaynak etkilenmez.
        assert!(h.dene("b", 1100).is_ok());
    }

    #[test]
    fn kapasite_asilmaz() {
        let mut h = hs(2, 10, 100, 1);
        assert!(h.dene("a", 1000).is_ok());
        // Çok uzun bekleme yine en çok 2 belirteç verir.
        assert!(h.dene("a", 1_000_000).is_ok());
        assert!(h.dene("a", 1_000_000).is_ok());
        assert!(h.dene("a", 1_000_000).is_err());
    }

    #[test]
    fn genel_kova_cok_kaynaga_karsi() {
        let mut h = hs(5, 100, 3, 1000);
        assert!(h.dene("a", 1000).is_ok());
        assert!(h.dene("b", 1000).is_ok());
        assert!(h.dene("c", 1000).is_ok());
        let e = h.dene("d", 1000).unwrap_err();
        assert!(e.genel);
        assert_eq!(e.saniye, 1000);
        // Reddedilen girişim kaynak kovasını tüketmedi: genel dolunca d girebilir.
        assert!(h.dene("d", 2000).is_ok());
    }

    #[test]
    fn kaynak_duz_metin_tutulmaz_ve_tuz_yenilenir() {
        let mut h = hs(1, 100, 100, 1);
        assert!(h.dene("telegram:987654321", 1000).is_ok());
        assert_eq!(h.kova_sayisi(), 1);
        let anahtar = h.anahtar("telegram:987654321");
        assert!(!anahtar.windows(4).any(|w| w == b"9876"));
        assert!(h.dene("telegram:987654321", 1001).is_err());
        h.tuz_yenile(b"baska-tuz-baska-tuz").unwrap();
        assert_eq!(h.kova_sayisi(), 0);
        assert!(h.dene("telegram:987654321", 1002).is_ok());
        assert_ne!(anahtar, h.anahtar("telegram:987654321"));
        assert!(h.tuz_yenile(b"kisa").is_err());
    }

    #[test]
    fn tablo_sinirli() {
        let mut h = hs(1, 100_000, u32::MAX, 1);
        // Genel kovayı büyük tutmak için doğrudan doldur.
        for i in 0..(EN_COK_KOVA + 50) {
            let _ = h.dene(&format!("k{i}"), 1000);
        }
        assert!(h.kova_sayisi() <= EN_COK_KOVA);
    }

    #[test]
    fn gecersiz_kurulum() {
        assert!(HizSiniri::yeni(b"kisa", 1, 1, 1, 1, 0).is_err());
        assert!(HizSiniri::yeni(TUZ, 0, 1, 1, 1, 0).is_err());
        assert!(HizSiniri::yeni(TUZ, 1, 0, 1, 1, 0).is_err());
    }
}
