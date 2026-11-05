//! taa-fis (TaaFiş): terzi fişi modeli, eklemeli defter (JSONL), durum takibi, termal (ESC/POS) ve düz metin çıktı.
//! Telefon sayfası, QR eşleştirme, OCR ve ağ sunucusu bu crate'in DIŞINDADIR (sonraki aşamalar); burada saf çekirdek var.
//! ESC/POS çıktısı gerçek yazıcıda denenmedi; yazıcı kod sayfası CP857 (Türkçe) varsayılır.
//! Defter silmez: durum değişikliği yeni satırdır. Özellik anahtarı: "fis" (varsayılan açık).
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use serde::{Deserialize, Serialize};

pub const OZELLIK: &str = "fis";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Durum {
    Bekliyor,
    /// Ürün terzihaneye verildi.
    Terzihanede,
    Hazir,
    Teslim,
    Iptal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kalem {
    /// Ürün (ceket, pantolon...).
    pub urun: String,
    /// Yapılacak işlem (paça kısalt, bel daralt...). Katalogdan seçildiyse katalog adı.
    pub islem: String,
    #[serde(default)]
    pub not: String,
    /// Katalog işlem kodu (taa-terzi); yoksa serbest metin işlem.
    #[serde(default)]
    pub islem_kodu: Option<String>,
    /// Düz paça / duble paça gibi seçenek.
    #[serde(default)]
    pub varyant: Option<String>,
    /// Yapılacak ölçü, milimetre (2,5 cm = 25).
    #[serde(default)]
    pub olcu_mm: Option<i32>,
    /// İçeride bırakılan pay, milimetre (tekrar uzatma için).
    #[serde(default)]
    pub pay_mm: Option<i32>,
    /// Sınır aşımında usta onayı verildi mi?
    #[serde(default)]
    pub usta_onayli: bool,
    /// Ölçü iğneyle işaretlendi (cm bilinmiyor).
    #[serde(default)]
    pub igneli: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeslimDegisti {
    pub eski: String,
    pub yeni: String,
    pub neden: String,
    pub tarih: String,
}

/// Telefonla atılan imza: kim (eşleşmiş telefonun kullanıcısı), rol (usta, teslim...), tarih.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Imza {
    pub kim: String,
    pub rol: String,
    pub tarih: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fis {
    pub no: String,
    pub tarih: String,
    pub musteri: String,
    pub telefon: String,
    pub kalemler: Vec<Kalem>,
    #[serde(default)]
    pub olcu_notu: String,
    #[serde(default)]
    pub prova: String,
    pub teslim: String,
    /// HH:MM
    #[serde(default = "varsayilan_saat")]
    pub teslim_saati: String,
    pub ucret_kurus: i64,
    pub on_odeme_kurus: i64,
    #[serde(default)]
    pub usta: String,
    #[serde(default)]
    pub satis_danismani: String,
    #[serde(default)]
    pub olcu_alan: String,
    /// Kargo ile gönderilecek mi? Fişte büyük yazılır (not kâğıdı düşebildiği için).
    #[serde(default)]
    pub kargo: bool,
    #[serde(default)]
    pub kargo_notu: String,
    /// Kargo işaretliyse zorunlu: alıcı ad-soyad, açık adres, alıcı telefonu.
    #[serde(default)]
    pub kargo_ad: String,
    #[serde(default)]
    pub kargo_adres: String,
    #[serde(default)]
    pub kargo_telefon: String,
    /// Müşteri "hazır" bilgilendirmesine izin verdi mi? (KVKK; varsayılan hayır)
    #[serde(default)]
    pub bildirim_izni: bool,
    #[serde(default)]
    pub teslim_degisiklikleri: Vec<TeslimDegisti>,
    pub durum: Durum,
    /// Hazır ürünün asılı olduğu stant/raf kodu (QR okutularak yazılır).
    #[serde(default)]
    pub raf: String,
    #[serde(default)]
    pub imzalar: Vec<Imza>,
}

fn varsayilan_saat() -> String {
    "17:00".into()
}

impl Fis {
    pub fn kalan_kurus(&self) -> i64 {
        self.ucret_kurus - self.on_odeme_kurus
    }
}

/// Yeni fiş girdisi (numara ve durum defter tarafından verilir).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Girdi {
    #[serde(default)]
    pub tarih: String,
    pub musteri: String,
    pub telefon: String,
    pub kalemler: Vec<Kalem>,
    #[serde(default)]
    pub olcu_notu: String,
    #[serde(default)]
    pub prova: String,
    pub teslim: String,
    #[serde(default = "varsayilan_saat")]
    pub teslim_saati: String,
    pub ucret_kurus: i64,
    #[serde(default)]
    pub on_odeme_kurus: i64,
    #[serde(default)]
    pub usta: String,
    #[serde(default)]
    pub satis_danismani: String,
    #[serde(default)]
    pub olcu_alan: String,
    /// Kargo ile gönderilecek mi? Fişte büyük yazılır (not kâğıdı düşebildiği için).
    #[serde(default)]
    pub kargo: bool,
    #[serde(default)]
    pub kargo_notu: String,
    /// Kargo işaretliyse zorunlu: alıcı ad-soyad, açık adres, alıcı telefonu.
    #[serde(default)]
    pub kargo_ad: String,
    #[serde(default)]
    pub kargo_adres: String,
    #[serde(default)]
    pub kargo_telefon: String,
    /// Müşteri "hazır" bilgilendirmesine izin verdi mi? (KVKK; varsayılan hayır)
    #[serde(default)]
    pub bildirim_izni: bool,
}

pub fn tarih_gecerli(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && t.char_indices()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && matches!(t[5..7].parse::<u32>(), Ok(1..=12))
        && matches!(t[8..10].parse::<u32>(), Ok(1..=31))
}

/// Yalnız rakamlar kalır; 10 hane (5xx...) ya da 0 ile 11 hane kabul edilir. "05321234567" biçimine çevirir.
pub fn telefon_duzelt(t: &str) -> Result<String, String> {
    let r: String = t.chars().filter(char::is_ascii_digit).collect();
    let r = r
        .strip_prefix("90")
        .filter(|x| x.len() == 10)
        .map(str::to_string)
        .unwrap_or(r);
    match r.len() {
        10 if r.starts_with('5') => Ok(format!("0{r}")),
        11 if r.starts_with("05") => Ok(r),
        _ => Err(format!("geçersiz cep telefonu: '{t}'")),
    }
}

/// Fiş girdisini denetler; telefonu düzeltilmiş halde ve kalem uyarılarıyla döndürür.
pub fn dogrula(
    g: &Girdi,
    katalog: &taa_terzi::Katalog,
    serbest_islem: bool,
    olcu_zorunlu: bool,
) -> Result<(String, Vec<String>), String> {
    if g.musteri.trim().is_empty() {
        return Err("müşteri adı gerekli".into());
    }
    if g.satis_danismani.trim().is_empty() || g.olcu_alan.trim().is_empty() {
        return Err("satış danışmanı ve ölçü alan kişi yazılmalı".into());
    }
    if !saat_gecerli(&g.teslim_saati) {
        return Err("teslim saati SS:DD olmalı".into());
    }
    let mut uyarilar = Vec::new();
    for k in &g.kalemler {
        match &k.islem_kodu {
            Some(kod) => uyarilar.extend(katalog.kontrol(
                kod,
                k.olcu_mm,
                k.varyant.as_deref(),
                k.usta_onayli,
                olcu_zorunlu,
                k.igneli,
            )?),
            None if !serbest_islem => {
                return Err("işlem listeden seçilmeli (serbest işlem kapalı)".into())
            }
            None => {}
        }
        if matches!(k.pay_mm, Some(p) if p < 0) {
            return Err("pay negatif olamaz".into());
        }
    }
    if g.kalemler.is_empty()
        || g.kalemler
            .iter()
            .any(|k| k.urun.trim().is_empty() || k.islem.trim().is_empty())
    {
        return Err("her kalemde ürün ve işlem yazılmalı (en az bir kalem)".into());
    }
    if !tarih_gecerli(&g.tarih)
        || !tarih_gecerli(&g.teslim)
        || (!g.prova.is_empty() && !tarih_gecerli(&g.prova))
    {
        return Err("tarihler YYYY-AA-GG olmalı".into());
    }
    if g.teslim < g.tarih || (!g.prova.is_empty() && (g.prova < g.tarih || g.prova > g.teslim)) {
        return Err(
            "prova/teslim tarihi fiş tarihinden önce olamaz; prova teslimden sonra olamaz".into(),
        );
    }
    if g.ucret_kurus < 0 || g.on_odeme_kurus < 0 || g.on_odeme_kurus > g.ucret_kurus {
        return Err("ücret/ön ödeme geçersiz (ön ödeme ücretten fazla olamaz)".into());
    }
    Ok((telefon_duzelt(&g.telefon)?, uyarilar))
}

pub fn saat_gecerli(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && matches!(s[0..2].parse::<u32>(), Ok(0..=23))
        && matches!(s[3..5].parse::<u32>(), Ok(0..=59))
}

mod basim;
mod defter;
pub use basim::*;
pub use defter::*;

#[cfg(test)]
mod testler {
    use super::*;

    fn kalem(urun: &str, islem: &str, not: &str) -> Kalem {
        Kalem {
            urun: urun.into(),
            islem: islem.into(),
            not: not.into(),
            islem_kodu: None,
            varyant: None,
            olcu_mm: None,
            pay_mm: None,
            usta_onayli: false,
            igneli: false,
        }
    }

    fn kat() -> taa_terzi::Katalog {
        taa_terzi::Katalog::varsayilan()
    }

    fn girdi() -> Girdi {
        Girdi {
            tarih: "2026-11-04".into(),
            musteri: " Ayşe Çelik ".into(),
            telefon: "+90 532 123 45 67".into(),
            kalemler: vec![
                kalem("Pantolon", "paça kısalt", "2 cm"),
                kalem("Ceket", "kol kısalt", ""),
            ],
            olcu_notu: "bel 84".into(),
            prova: "2026-11-07".into(),
            teslim: "2026-11-10".into(),
            teslim_saati: "17:00".into(),
            ucret_kurus: 85000,
            on_odeme_kurus: 20000,
            usta: "Hasan".into(),
            satis_danismani: "Mert".into(),
            olcu_alan: "Kadir Y.".into(),
            kargo: false,
            kargo_notu: String::new(),
            kargo_ad: String::new(),
            kargo_adres: String::new(),
            kargo_telefon: String::new(),
            bildirim_izni: false,
        }
    }

    fn yol() -> std::path::PathBuf {
        taa_dosya::gecici_klasor().unwrap().join("fis.jsonl")
    }

    #[test]
    fn aydinlatma_yalniz_musteri_nushasinda() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let f = Defter { yol: &y }.ekle(girdi()).unwrap();
        let url = "http://x/aydinlatma";
        let b = escpos_nushalar_tam(&f, 42, "http://x/telefon#fis=F", Some(url));
        let sayi = |hedef: &[u8]| b.windows(hedef.len()).filter(|w| *w == hedef).count();
        assert_eq!(sayi(url.as_bytes()), 1); // yalnız müşteri nüshası
        assert_eq!(
            sayi(
                "Kişisel verileriniz"
                    .chars()
                    .map(cp857)
                    .collect::<Vec<_>>()
                    .as_slice()
            ),
            1
        );
        // verilmezse hiçbir nüshada yok
        let c = escpos_nushalar_tam(&f, 42, "q", None);
        assert!(!c.windows(19).any(|w| w == url.as_bytes()));
        assert_eq!(
            c.windows(4).filter(|w| *w == [0x1D, 0x56, 0x42, 0]).count(),
            2
        );
    }

    #[test]
    fn raf_ve_imza() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let d = Defter { yol: &y };
        let f = d.ekle(girdi()).unwrap();
        assert!(d.raf_ata(&f.no, "", "Ali", "2026-11-05").is_err());
        assert!(d.raf_ata("yok", "A-1", "Ali", "2026-11-05").is_err());
        d.raf_ata(&f.no, "a-12", "Ali", "2026-11-05").unwrap();
        d.raf_ata(&f.no, "B-3", "Ali", "2026-11-06").unwrap();
        d.imzala(&f.no, "Usta Ahmet", "usta", "2026-11-05").unwrap();
        assert!(d.imzala(&f.no, "Usta Ahmet", "usta", "2026-11-05").is_err());
        assert!(d.imzala(&f.no, "", "usta", "2026-11-05").is_err());
        assert!(d.imzala(&f.no, "X", "patron", "2026-11-05").is_err());
        let g = d.bul(&f.no).unwrap().unwrap();
        assert_eq!(g.raf, "B-3");
        assert_eq!(g.imzalar.len(), 1);
        let m = duz_metin(&g, 42);
        assert!(m.contains("Raf/Stant: B-3") && m.contains("İmza (usta): Usta Ahmet"));
        let q = escpos_nushalar_qr(&g, 42, "http://x/telefon#fis=F");
        assert!(q.windows(22).any(|w| w == b"http://x/telefon#fis=F"));
        for st in [Durum::Terzihanede, Durum::Hazir, Durum::Teslim] {
            d.durum_ata(&f.no, st, "2026-11-06").unwrap();
        }
        assert!(d.raf_ata(&f.no, "C-1", "Ali", "2026-11-07").is_err());
    }

    #[test]
    fn kargo_fiste_basilir() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let d = Defter { yol: &y };
        let mut g = girdi();
        g.kargo = true;
        g.kargo_notu = "Zile kapıda ödeme yok".into();
        assert!(d.ekle(g.clone()).unwrap_err().contains("ad ve soyad"));
        g.kargo_ad = "Ayşe Çelik".into();
        assert!(d.ekle(g.clone()).unwrap_err().contains("açık adres"));
        g.kargo_adres = "Atatürk Mah. 5. Sk. No:3 Çankaya/Ankara".into();
        assert!(d.ekle(g.clone()).unwrap_err().contains("telefon"));
        g.kargo_telefon = "0532 111 22 33".into();
        let f = d.ekle(g).unwrap();
        for e in [
            "TERZİHANE NÜSHASI 1",
            "TERZİHANE NÜSHASI 2",
            "MÜŞTERİ NÜSHASI",
        ] {
            let m = duz_metin_nusha(&f, 42, e);
            assert!(
                m.contains("KARGO YAPILACAK") && m.contains("Çankaya"),
                "{e}"
            );
        }
        let f2 = d.ekle(girdi()).unwrap();
        assert!(!duz_metin(&f2, 42).contains("KARGO"));
    }

    #[test]
    fn telefon() {
        assert_eq!(telefon_duzelt("+90 532 123 45 67").unwrap(), "05321234567");
        assert_eq!(telefon_duzelt("0532 123 4567").unwrap(), "05321234567");
        assert_eq!(telefon_duzelt("5321234567").unwrap(), "05321234567");
        assert!(telefon_duzelt("12345").is_err());
        assert!(telefon_duzelt("02121234567").is_err());
    }

    #[test]
    fn dogrulama() {
        let mut g = girdi();
        assert!(dogrula(&g, &kat(), true, true).is_ok());
        g.on_odeme_kurus = 90000;
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.teslim = "2026-11-01".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.prova = "2026-11-12".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.kalemler.clear();
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.musteri = "  ".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
    }

    #[test]
    fn defter_dongusu() {
        let y = yol();
        let d = Defter { yol: &y };
        let f1 = d.ekle(girdi()).unwrap();
        let f2 = d.ekle(girdi()).unwrap();
        assert_eq!(
            (f1.no.as_str(), f2.no.as_str()),
            ("F-20261104-001", "F-20261104-002")
        );
        assert_eq!(f1.musteri, "Ayşe Çelik");
        assert_eq!(f1.kalan_kurus(), 65000);
        assert!(d.durum_ata(&f1.no, Durum::Teslim, "2026-11-05").is_err()); // Hazir'dan geçmeli
        d.durum_ata(&f1.no, Durum::Hazir, "2026-11-08").unwrap();
        d.durum_ata(&f1.no, Durum::Teslim, "2026-11-09").unwrap();
        assert!(d.durum_ata(&f1.no, Durum::Iptal, "2026-11-09").is_err());
        assert_eq!(d.bul(&f1.no).unwrap().unwrap().durum, Durum::Teslim);
        assert_eq!(d.geciken("2026-11-11").unwrap().len(), 1);
        assert_eq!(d.geciken("2026-11-10").unwrap().len(), 0);
        assert_eq!(taa_dosya::satirlari_oku(&y).unwrap().len(), 4); // hiçbir şey silinmedi
    }

    #[test]
    fn metin_ve_termal() {
        let f = Fis {
            no: "F-20261104-001".into(),
            tarih: "2026-11-04".into(),
            durum: Durum::Bekliyor,
            raf: String::new(),
            imzalar: Vec::new(),
            musteri: "Ayşe Çelik".into(),
            telefon: "05321234567".into(),
            kalemler: girdi().kalemler,
            olcu_notu: "".into(),
            prova: "".into(),
            teslim: "2026-11-10".into(),
            teslim_saati: "17:00".into(),
            ucret_kurus: 85000,
            on_odeme_kurus: 20000,
            usta: "".into(),
            satis_danismani: "Mert".into(),
            olcu_alan: "Kadir Y.".into(),
            kargo: false,
            kargo_notu: String::new(),
            kargo_ad: String::new(),
            kargo_adres: String::new(),
            kargo_telefon: String::new(),
            bildirim_izni: false,
            teslim_degisiklikleri: Vec::new(),
        };
        let m = duz_metin(&f, 42);
        assert!(m.contains("TADİLAT FİŞİ") && m.contains("F-20261104-001"));
        assert!(m.contains("ÜRÜN TESLİM TARİHİ") && m.contains("10.11.2026 17:00"));
        assert!(m.lines().all(|l| l.chars().count() <= 42));
        // ödeme alınmadan işlem yapılmaz: fiyat basılmaz
        assert!(!m.contains("KALAN") && !m.contains(" TL"));
        let b = escpos(&f, 42);
        assert_eq!(&b[..5], &[0x1B, 0x40, 0x1B, 0x74, 13]);
        assert!(b.contains(&0x98)); // İ -> CP857
        assert!(b.ends_with(&[0x1D, 0x56, 0x42, 0]));
    }

    #[test]
    fn para_ve_sarma() {
        assert_eq!(para(123456), "1.234,56");
        assert_eq!(para(5), "0,05");
        assert!(sar(&"a".repeat(100), 42).iter().all(|l| l.len() <= 42));
    }

    fn katalogsuz_girdi() -> Girdi {
        let mut g = girdi();
        g.kalemler = vec![Kalem {
            islem_kodu: Some("pant-bel-daralt".into()),
            olcu_mm: Some(30),
            ..kalem("Pantolon", "Bel daraltma", "")
        }];
        g
    }

    #[test]
    fn katalog_kalemi_ve_sinir() {
        let mut g = katalogsuz_girdi();
        assert!(dogrula(&g, &kat(), false, true).is_ok());
        g.kalemler[0].olcu_mm = Some(60);
        assert!(dogrula(&g, &kat(), false, true)
            .unwrap_err()
            .contains("Usta onayı"));
        g.kalemler[0].usta_onayli = true;
        assert_eq!(dogrula(&g, &kat(), false, true).unwrap().1.len(), 1);
        g.kalemler[0].olcu_mm = None; // unutulan ölçü
        assert!(dogrula(&g, &kat(), false, true)
            .unwrap_err()
            .contains("ölçü"));
    }

    #[test]
    fn serbest_islem_kapatilabilir_ve_zorunlu_isimler() {
        let g = girdi(); // serbest metin kalemler
        assert!(dogrula(&g, &kat(), true, true).is_ok());
        assert!(dogrula(&g, &kat(), false, true)
            .unwrap_err()
            .contains("listeden"));
        let mut g = girdi();
        g.olcu_alan = " ".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.teslim_saati = "25:00".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
    }

    #[test]
    fn teslim_degisikligi_izlenir() {
        let y = yol();
        let d = Defter { yol: &y };
        let f = d.ekle(girdi()).unwrap();
        assert!(d
            .teslim_degistir(&f.no, "2026-11-12", "17:00", " ", "2026-11-05")
            .is_err());
        assert!(d
            .teslim_degistir(&f.no, "2026-11-01", "17:00", "x", "2026-11-05")
            .is_err());
        d.teslim_degistir(&f.no, "2026-11-12", "18:30", "müşteri istedi", "2026-11-05")
            .unwrap();
        let g = d.bul(&f.no).unwrap().unwrap();
        assert_eq!(
            (g.teslim.as_str(), g.teslim_saati.as_str()),
            ("2026-11-12", "18:30")
        );
        assert_eq!(g.teslim_degisiklikleri[0].eski, "2026-11-10");
        assert!(duz_metin(&g, 42).contains("1 kez değişti"));
        d.durum_ata(&f.no, Durum::Terzihanede, "2026-11-05")
            .unwrap();
        d.durum_ata(&f.no, Durum::Hazir, "2026-11-06").unwrap();
        d.durum_ata(&f.no, Durum::Teslim, "2026-11-12").unwrap();
        assert!(d
            .teslim_degistir(&f.no, "2026-11-20", "17:00", "x", "2026-11-13")
            .is_err());
    }

    #[test]
    fn eski_defter_satiri_okunur() {
        // yeni alanlardan önce yazılmış bir satır
        let y = yol();
        let eski = r#"{"tur":"fis","v":{"no":"F-20260101-001","tarih":"2026-01-01","musteri":"A","telefon":"05321234567","kalemler":[{"urun":"Ceket","islem":"kol"}],"teslim":"2026-01-05","ucret_kurus":0,"on_odeme_kurus":0,"durum":"bekliyor"}}"#;
        taa_dosya::satir_ekle(&y, eski).unwrap();
        let f = Defter { yol: &y }.bul("F-20260101-001").unwrap().unwrap();
        assert_eq!(f.teslim_saati, "17:00");
        assert!(f.kalemler[0].olcu_mm.is_none());
    }

    #[test]
    fn uc_nusha_ve_cm() {
        assert_eq!(cm_metni(25), "2,5");
        assert_eq!(cm_metni(40), "4");
        let y = yol();
        let d = Defter { yol: &y };
        let f = d.ekle(katalogsuz_girdi()).unwrap();
        let m = duz_metin(&f, 42);
        assert!(
            m.contains("Bel daraltma 3 cm")
                && m.contains("Sat. Dan.: Mert")
                && m.contains("Ölçü Alan: Kadir Y.")
        );
        let b = escpos_nushalar(&f, 42);
        assert_eq!(
            b.windows(4).filter(|w| *w == [0x1D, 0x56, 0x42, 0]).count(),
            2
        );
        assert!(String::from_utf8_lossy(&b).contains("mesul"));
    }

    #[test]
    fn kagit_tasarrufu_duzeni() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let f = Defter { yol: &y }.ekle(girdi()).unwrap();
        assert_eq!(nusha_etiketleri(2).len(), 2);
        assert_eq!(nusha_etiketleri(3).len(), 3);
        let t = duz_metin_nusha(&f, 42, "TERZİHANE NÜSHASI");
        let m = duz_metin_nusha(&f, 42, "MÜŞTERİ NÜSHASI");
        assert!(t.contains("Teslim Eden") && !m.contains("Teslim Eden"));
        assert!(t.contains("Fiş Tarihi: 04.11.2026"));
        assert!(t.contains("Sat. Dan.: Mert | Ölçü Alan: Kadir Y."));
        assert!(t.lines().all(|l| l.chars().count() <= 42));
        // alt bilgi cümleleri yalnız müşteri nüshasında basılır
        let b = escpos_nushalar(&f, 42);
        let yer = |hedef: &str| {
            let h: Vec<u8> = hedef.chars().map(cp857).collect();
            b.windows(h.len()).filter(|w| *w == h.as_slice()).count()
        };
        assert_eq!(yer("mesul değiliz"), 1);
    }

    #[test]
    fn logo_gun_adi_ve_serit() {
        assert_eq!(gun_adi("2026-11-10"), Some("Salı"));
        assert_eq!(gun_adi("2026-11-05"), Some("Perşembe"));
        assert_eq!(gun_adi("bozuk"), None);
        // logo bitmap'i: başlık + 72 bayt x yükseklik, 80 mm genişliğe sığar
        let h = LOGO[0] as usize | (LOGO[1] as usize) << 8;
        assert!(h > 20 && h < 200 && LOGO.len() == 2 + 72 * h);
        let l = logo_baytlari();
        assert_eq!(&l[..6], &[0x1D, 0x76, 0x30, 0, 72, 0]);
        assert!(LOGO[2..].iter().any(|b| *b != 0)); // siyah nokta var
        let dir = taa_dosya::gecici_klasor().unwrap();
        let f = Defter {
            yol: &dir.join("d.jsonl"),
        }
        .ekle(girdi())
        .unwrap();
        let b = escpos_nusha_tam(&f, 42, "MÜŞTERİ NÜSHASI", "q", None);
        assert!(b.windows(l.len()).any(|w| w == l.as_slice()));
        let m = duz_metin_nusha(&f, 42, "");
        assert!(
            m.contains("TERZİ ADEM ALTUN")
                && m.contains("Salı")
                && m.contains("= ÜRÜN TESLİM TARİHİ =")
        );
    }
}
