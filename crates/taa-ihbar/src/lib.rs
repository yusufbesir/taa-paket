//! taa-ihbar: ihbar hattı çekirdeği (RH12a-b; plan PLAN-MUSTERI-HAZIR-MESAJI-2026-11-15 §S2.5).
//! Karar: ayrı Telegram botu; bildirim Yusuf'un Telegram'ına ve e-postasına gider; ihbar sayfası TAA Rehber'de.
//! Bu crate yalnız çekirdektir ve AĞA ÇIKMAZ: kayıt (varsayılan anonim), yalnız-ekleme defter, maskeli bildirim
//! biçimleyici, `Gonderici` arayüzü (+ sahte gönderici), belirteç kovası hız sınırı (kaynak tutulmadan) ve yalnız
//! yönetici bayrağıyla görüntüleme. Bot anahtarı ve adresler kodda/depoda yoktur; `ihbar.json` ayarından okunur.
//! DÜRÜST SINIR: tam anonimlik garanti edilmez (Telegram şirketi göndereni bilir; metnin içeriği kişiyi ele verebilir).
//! Sahip: Yusuf. yazan: [Claude#ihbar 2026-11-26]

pub mod ayar;
pub mod bicim;
pub mod defter;
pub mod gonderici;
pub mod hiz;
pub mod kayit;

pub use ayar::Ayar;
pub use bicim::{bildirim, maskele, Bildirim};
pub use defter::{Defter, Teslim, Yonetici};
pub use gonderici::{teslim_et, Gonderici, SahteGonderici};
pub use hiz::{Beklet, HizSiniri};
pub use kayit::{takip_no_uret, Girdi, Ihbar, Kategori, METIN_EN_COK};

pub const OZELLIK: &str = "ihbar";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hata {
    Kapali,
    /// Hız sınırı: kaç saniye sonra yeniden denenebilir.
    HizSiniri {
        bekle_sn: u64,
        genel: bool,
    },
    Gecersiz(String),
    Depo(String),
}

impl std::fmt::Display for Hata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Hata::Kapali => f.write_str(&taa_ayar::kapali_mesaji(OZELLIK)),
            Hata::HizSiniri { bekle_sn, .. } => {
                write!(
                    f,
                    "çok sık ihbar gönderildi; {bekle_sn} sn sonra yeniden deneyin"
                )
            }
            Hata::Gecersiz(m) | Hata::Depo(m) => f.write_str(m),
        }
    }
}

/// Alındı belgesi: ihbar edene yalnız takip numarası döner.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Makbuz {
    pub takip_no: String,
    pub telegram: Teslim,
    pub eposta: Teslim,
}

impl Makbuz {
    /// Hiç olmazsa bir kanal ulaştı mı. Ulaşmadıysa ihbar yine de defterdedir.
    pub fn iletildi(&self) -> bool {
        self.telegram == Teslim::Gitti || self.eposta == Teslim::Gitti
    }
}

pub struct Hat<'a> {
    pub defter: &'a Defter<'a>,
    pub ayar: &'a Ayar,
    pub gonderici: &'a dyn Gonderici,
}

impl Hat<'_> {
    /// Bir ihbarı alır. Sıra: özellik anahtarı, hız sınırı (kaynak yalnız burada, özet olarak kullanılır), doğrulama,
    /// deftere ekleme (iletim başarısız olsa da kayıp olmaz), maskeli bildirimi iki kanala iletme, teslimat kaydı.
    /// `kaynak` hiçbir yere yazılmaz. `tohum`: bu ihbar için yeni rastgele bayt (en az 16). `tarih`: bugün (YYYY-MM-DD).
    pub fn al(
        &self,
        hiz: &mut HizSiniri,
        kaynak: &str,
        girdi: Girdi,
        tarih: &str,
        simdi: u64,
        tohum: &[u8],
    ) -> Result<Makbuz, Hata> {
        if !taa_ayar::ozellik_acik(OZELLIK, true) {
            return Err(Hata::Kapali);
        }
        hiz.dene(kaynak, simdi).map_err(|b| Hata::HizSiniri {
            bekle_sn: b.saniye,
            genel: b.genel,
        })?;
        let mut deneme = 0u64;
        let no = loop {
            let n = takip_no_uret(tohum, tarih, deneme).map_err(Hata::Gecersiz)?;
            if !self.defter.takip_no_var(&n).map_err(Hata::Depo)? {
                break n;
            }
            deneme += 1;
            if deneme > 16 {
                return Err(Hata::Depo("takip numarası üretilemedi".into()));
            }
        };
        let ihbar = Ihbar::olustur(girdi, tarih, no).map_err(Hata::Gecersiz)?;
        self.defter.ekle(&ihbar).map_err(Hata::Depo)?;
        let b = bildirim(&ihbar);
        let (telegram, eposta) = teslim_et(self.gonderici, self.ayar, &b);
        self.defter
            .teslimat_yaz(&ihbar.takip_no, tarih, telegram.clone(), eposta.clone())
            .map_err(Hata::Depo)?;
        Ok(Makbuz {
            takip_no: ihbar.takip_no,
            telegram,
            eposta,
        })
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    const AYAR: &str = r#"{"telegram_bot_anahtari":"BOT-SIRRI-123","telegram_sohbet":"777",
        "eposta_alici":"yusuf@ornek.test","eposta_gonderen":"bot@ornek.test"}"#;
    const TUZ: &[u8] = b"hiz-tuzu-hiz-tuzu-1";

    fn girdi(m: &str) -> Girdi {
        Girdi {
            kategori: Kategori::Usulsuzluk,
            metin: m.into(),
            iletisim: None,
        }
    }

    #[test]
    fn uctan_uca_iki_kanal() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let yol = d.join("ihbar.jsonl");
        let defter = Defter { yol: &yol };
        let ayar = Ayar::metinden(AYAR).unwrap();
        let g = SahteGonderici::default();
        let hat = Hat {
            defter: &defter,
            ayar: &ayar,
            gonderici: &g,
        };
        let mut hiz = HizSiniri::ayardan(&ayar, TUZ, 1000).unwrap();
        let m = hat
            .al(
                &mut hiz,
                "kaynak-1",
                girdi("Ali 05321234567 ile anlaştı"),
                "2026-11-26",
                1000,
                &[9u8; 16],
            )
            .unwrap();
        assert!(m.iletildi());
        assert_eq!(g.telegramlar().len(), 1);
        assert_eq!(g.telegramlar()[0].0, "777");
        assert_eq!(g.epostalar()[0].0, "yusuf@ornek.test");
        assert!(!g.telegramlar()[0].1.contains("0532"));
        // Defter: ihbar + teslimat; kaynak ve sır dosyada yok.
        let ham = std::fs::read_to_string(&yol).unwrap();
        assert_eq!(ham.lines().count(), 2);
        for yasak in ["kaynak-1", "BOT-SIRRI-123", "777", "yusuf@ornek.test"] {
            assert!(!ham.contains(yasak), "defterde {yasak} var");
        }
        // Görüntüleme: yönetici bayrağı ile.
        let y = Yonetici::bayraktan(true).unwrap();
        let i = defter.bul(&y, &m.takip_no).unwrap().unwrap();
        assert!(i.metin.contains("05321234567"), "özgün metin defterde");
        assert_eq!(
            defter.son_teslimat(&y, &m.takip_no).unwrap(),
            Some((Teslim::Gitti, Teslim::Gitti))
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn iletim_hatasi_ihbari_kaybetmez_ve_sir_sizdirmaz() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let yol = d.join("ihbar.jsonl");
        let defter = Defter { yol: &yol };
        let ayar = Ayar::metinden(AYAR).unwrap();
        let g = SahteGonderici::telegram_hatali(
            "POST https://api.ornek.test/botBOT-SIRRI-123/sendMessage zaman aşımı",
        );
        let hat = Hat {
            defter: &defter,
            ayar: &ayar,
            gonderici: &g,
        };
        let mut hiz = HizSiniri::ayardan(&ayar, TUZ, 1000).unwrap();
        let m = hat
            .al(
                &mut hiz,
                "k",
                girdi("metin"),
                "2026-11-26",
                1000,
                &[1u8; 16],
            )
            .unwrap();
        assert!(m.iletildi(), "e-posta gitti");
        match &m.telegram {
            Teslim::Hata(h) => assert!(!h.contains("BOT-SIRRI-123") && h.contains("[gizli]")),
            x => panic!("{x:?}"),
        }
        assert_eq!(defter.sayi().unwrap(), 1);
        let ham = std::fs::read_to_string(&yol).unwrap();
        assert!(!ham.contains("BOT-SIRRI-123"));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn kapali_kanal_ve_hic_ulasmama() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let yol = d.join("ihbar.jsonl");
        let defter = Defter { yol: &yol };
        let ayar =
            Ayar::metinden(r#"{"telegram_bot_anahtari":"k","telegram_sohbet":"1"}"#).unwrap();
        let g = SahteGonderici::telegram_hatali("kapalı");
        let hat = Hat {
            defter: &defter,
            ayar: &ayar,
            gonderici: &g,
        };
        let mut hiz = HizSiniri::ayardan(&ayar, TUZ, 1000).unwrap();
        let m = hat
            .al(&mut hiz, "k", girdi("m"), "2026-11-26", 1000, &[2u8; 16])
            .unwrap();
        assert!(!m.iletildi());
        assert_eq!(m.eposta, Teslim::Kapali);
        assert_eq!(defter.sayi().unwrap(), 1);
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn hiz_siniri_ve_gecersiz_girdi() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let yol = d.join("ihbar.jsonl");
        let defter = Defter { yol: &yol };
        let ayar = Ayar::metinden(AYAR).unwrap();
        let g = SahteGonderici::default();
        let hat = Hat {
            defter: &defter,
            ayar: &ayar,
            gonderici: &g,
        };
        let mut hiz = HizSiniri::yeni(TUZ, 2, 3600, 100, 1, 1000).unwrap();
        assert!(hat
            .al(&mut hiz, "k", girdi("bir"), "2026-11-26", 1000, &[3u8; 16])
            .is_ok());
        assert!(hat
            .al(&mut hiz, "k", girdi("iki"), "2026-11-26", 1000, &[4u8; 16])
            .is_ok());
        let e = hat
            .al(&mut hiz, "k", girdi("üç"), "2026-11-26", 1000, &[5u8; 16])
            .unwrap_err();
        assert!(matches!(e, Hata::HizSiniri { genel: false, .. }));
        assert!(e.to_string().contains("sn sonra"));
        assert_eq!(defter.sayi().unwrap(), 2);
        // Geçersiz metin deftere girmez.
        let e = hat
            .al(
                &mut hiz,
                "baska",
                girdi("  "),
                "2026-11-26",
                1000,
                &[6u8; 16],
            )
            .unwrap_err();
        assert!(matches!(e, Hata::Gecersiz(_)));
        let e = hat
            .al(
                &mut hiz,
                "baska2",
                girdi(&"x".repeat(4001)),
                "2026-11-26",
                1000,
                &[7u8; 16],
            )
            .unwrap_err();
        assert!(matches!(e, Hata::Gecersiz(_)));
        assert_eq!(defter.sayi().unwrap(), 2);
        // Kısa tohum reddedilir.
        let e = hat
            .al(
                &mut hiz,
                "baska3",
                girdi("m"),
                "2026-11-26",
                1000,
                &[1u8; 3],
            )
            .unwrap_err();
        assert!(matches!(e, Hata::Gecersiz(_)));
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn ayni_tohumla_takip_no_cakismasi_asilir() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let yol = d.join("ihbar.jsonl");
        let defter = Defter { yol: &yol };
        let ayar = Ayar::metinden(AYAR).unwrap();
        let g = SahteGonderici::default();
        let hat = Hat {
            defter: &defter,
            ayar: &ayar,
            gonderici: &g,
        };
        let mut hiz = HizSiniri::ayardan(&ayar, TUZ, 1000).unwrap();
        let a = hat
            .al(&mut hiz, "a", girdi("1"), "2026-11-26", 1000, &[8u8; 16])
            .unwrap();
        let b = hat
            .al(&mut hiz, "b", girdi("2"), "2026-11-26", 1000, &[8u8; 16])
            .unwrap();
        assert_ne!(a.takip_no, b.takip_no);
        let _ = std::fs::remove_dir_all(&d);
    }
}
