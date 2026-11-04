//! taa-fis (TaaFiş): terzi fişi modeli, eklemeli defter (JSONL), durum takibi, termal (ESC/POS) ve düz metin çıktı.
//! Telefon sayfası, QR eşleştirme, OCR ve ağ sunucusu bu crate'in DIŞINDADIR (sonraki aşamalar); burada saf çekirdek var.
//! ESC/POS çıktısı gerçek yazıcıda denenmedi; yazıcı kod sayfası CP857 (Türkçe) varsayılır.
//! Defter silmez: durum değişikliği yeni satırdır. Özellik anahtarı: "fis" (varsayılan açık).
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const OZELLIK: &str = "fis";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Durum {
    Bekliyor,
    Hazir,
    Teslim,
    Iptal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kalem {
    /// Ürün (ceket, pantolon...).
    pub urun: String,
    /// Yapılacak işlem (paça kısalt, bel daralt...).
    pub islem: String,
    #[serde(default)]
    pub not: String,
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
    pub ucret_kurus: i64,
    pub on_odeme_kurus: i64,
    #[serde(default)]
    pub usta: String,
    pub durum: Durum,
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
    pub ucret_kurus: i64,
    #[serde(default)]
    pub on_odeme_kurus: i64,
    #[serde(default)]
    pub usta: String,
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

pub fn dogrula(g: &Girdi) -> Result<String, String> {
    if g.musteri.trim().is_empty() {
        return Err("müşteri adı gerekli".into());
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
    telefon_duzelt(&g.telefon)
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "lowercase")]
enum Kayit {
    Fis(Fis),
    Durum {
        no: String,
        durum: Durum,
        tarih: String,
    },
}

pub struct Defter<'a> {
    pub yol: &'a Path,
}

fn kapali() -> Result<(), String> {
    if taa_ayar::ozellik_acik(OZELLIK, true) {
        Ok(())
    } else {
        Err(taa_ayar::kapali_mesaji(OZELLIK))
    }
}

impl Defter<'_> {
    fn kayitlar(&self) -> Result<Vec<Kayit>, String> {
        taa_dosya::satirlari_oku(self.yol)
            .map_err(|e| e.to_string())?
            .iter()
            .map(|l| serde_json::from_str(l).map_err(|e| format!("defter satırı bozuk: {e}")))
            .collect()
    }

    /// Fişlerin güncel durumları (defter sırasıyla).
    pub fn fisler(&self) -> Result<Vec<Fis>, String> {
        let mut f: Vec<Fis> = Vec::new();
        for k in self.kayitlar()? {
            match k {
                Kayit::Fis(x) => f.push(x),
                Kayit::Durum { no, durum, .. } => {
                    if let Some(x) = f.iter_mut().find(|x| x.no == no) {
                        x.durum = durum;
                    }
                }
            }
        }
        Ok(f)
    }

    pub fn bul(&self, no: &str) -> Result<Option<Fis>, String> {
        Ok(self.fisler()?.into_iter().find(|f| f.no == no))
    }

    pub fn ekle(&self, g: Girdi) -> Result<Fis, String> {
        kapali()?;
        let telefon = dogrula(&g)?;
        let gun = g.tarih.replace('-', "");
        let sira = self.fisler()?.iter().filter(|f| f.tarih == g.tarih).count() + 1;
        let fis = Fis {
            no: format!("F-{gun}-{sira:03}"),
            tarih: g.tarih,
            musteri: g.musteri.trim().into(),
            telefon,
            kalemler: g.kalemler,
            olcu_notu: g.olcu_notu.trim().into(),
            prova: g.prova,
            teslim: g.teslim,
            ucret_kurus: g.ucret_kurus,
            on_odeme_kurus: g.on_odeme_kurus,
            usta: g.usta.trim().into(),
            durum: Durum::Bekliyor,
        };
        let s = serde_json::to_string(&Kayit::Fis(fis.clone())).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())?;
        Ok(fis)
    }

    /// Durumu ilerletir: Bekliyor -> Hazir -> Teslim; Iptal her yerden (Teslim'den sonra değil). Geri dönüş yok.
    pub fn durum_ata(&self, no: &str, yeni: Durum, tarih: &str) -> Result<(), String> {
        kapali()?;
        let f = self.bul(no)?.ok_or_else(|| format!("fiş yok: {no}"))?;
        let izin = matches!(
            (f.durum, yeni),
            (Durum::Bekliyor, Durum::Hazir)
                | (Durum::Hazir, Durum::Teslim)
                | (Durum::Bekliyor | Durum::Hazir, Durum::Iptal)
        );
        if !izin {
            return Err(format!(
                "{:?} durumundan {:?} durumuna geçilemez",
                f.durum, yeni
            ));
        }
        if !tarih_gecerli(tarih) {
            return Err("tarih YYYY-AA-GG olmalı".into());
        }
        let s = serde_json::to_string(&Kayit::Durum {
            no: no.into(),
            durum: yeni,
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    /// Teslim tarihi geçmiş ve teslim edilmemiş fişler.
    pub fn geciken(&self, bugun: &str) -> Result<Vec<Fis>, String> {
        Ok(self
            .fisler()?
            .into_iter()
            .filter(|f| {
                matches!(f.durum, Durum::Bekliyor | Durum::Hazir) && f.teslim.as_str() < bugun
            })
            .collect())
    }
}

pub fn para(k: i64) -> String {
    let tl = (k.unsigned_abs() / 100).to_string();
    let mut g = String::new();
    for (i, c) in tl.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            g.push('.');
        }
        g.push(c);
    }
    format!(
        "{}{},{:02}",
        if k < 0 { "-" } else { "" },
        g.chars().rev().collect::<String>(),
        k.unsigned_abs() % 100
    )
}

fn gun_metni(t: &str) -> String {
    format!("{}.{}.{}", &t[8..10], &t[5..7], &t[0..4])
}

fn sar(metin: &str, genislik: usize) -> Vec<String> {
    let mut satirlar = Vec::new();
    let mut cur = String::new();
    for kelime in metin.split_whitespace() {
        let kl = kelime.chars().count();
        if !cur.is_empty() && cur.chars().count() + 1 + kl > genislik {
            satirlar.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(kelime);
        while cur.chars().count() > genislik {
            let (a, b): (String, String) = (
                cur.chars().take(genislik).collect(),
                cur.chars().skip(genislik).collect(),
            );
            satirlar.push(a);
            cur = b;
        }
    }
    if !cur.is_empty() {
        satirlar.push(cur);
    }
    satirlar
}

/// Sabit genişlikli (örn. 42 sütun) düz metin fiş.
pub fn duz_metin(f: &Fis, genislik: usize) -> String {
    let g = genislik.max(24);
    let cizgi = "-".repeat(g);
    let mut o = String::new();
    let mut ekle = |s: &str| {
        for l in sar(s, g) {
            o.push_str(&l);
            o.push('\n');
        }
    };
    ekle(&format!("TERZİ FİŞİ {}", f.no));
    ekle(&format!("Tarih: {}", gun_metni(&f.tarih)));
    ekle(&format!("Müşteri: {}", f.musteri));
    ekle(&format!("Tel: {}", f.telefon));
    o.push_str(&cizgi);
    o.push('\n');
    let mut ekle = |s: &str| {
        for l in sar(s, g) {
            o.push_str(&l);
            o.push('\n');
        }
    };
    for k in &f.kalemler {
        ekle(&format!("* {} - {}", k.urun, k.islem));
        if !k.not.is_empty() {
            ekle(&format!("  ({})", k.not));
        }
    }
    if !f.olcu_notu.is_empty() {
        ekle(&format!("Ölçü: {}", f.olcu_notu));
    }
    if !f.prova.is_empty() {
        ekle(&format!("Prova: {}", gun_metni(&f.prova)));
    }
    ekle(&format!("Teslim: {}", gun_metni(&f.teslim)));
    if !f.usta.is_empty() {
        ekle(&format!("Usta: {}", f.usta));
    }
    o.push_str(&cizgi);
    o.push('\n');
    for (e, d) in [
        ("Ücret", f.ucret_kurus),
        ("Ön ödeme", f.on_odeme_kurus),
        ("KALAN", f.kalan_kurus()),
    ] {
        let sag = format!("{} TL", para(d));
        let bosluk = g
            .saturating_sub(e.chars().count() + sag.chars().count())
            .max(1);
        o.push_str(&format!("{e}{}{sag}\n", " ".repeat(bosluk)));
    }
    o
}

fn cp857(c: char) -> u8 {
    match c {
        'Ç' => 0x80,
        'ü' => 0x81,
        'ç' => 0x87,
        'ı' => 0x8D,
        'İ' => 0x98,
        'Ö' => 0x99,
        'Ü' => 0x9A,
        'ö' => 0x94,
        'Ğ' => 0xA6,
        'ğ' => 0xA7,
        'Ş' => 0x9E,
        'ş' => 0x9F,
        c if c.is_ascii() => c as u8,
        _ => b'?',
    }
}

/// ESC/POS baytları: CP857, metin, fiş numarası QR kodu, kesme. Yazıcıya olduğu gibi gönderilir.
pub fn escpos(f: &Fis, genislik: usize) -> Vec<u8> {
    let mut b = vec![0x1B, 0x40, 0x1B, 0x74, 13]; // başlat, kod sayfası 13 = CP857
    b.extend(duz_metin(f, genislik).chars().map(cp857));
    // QR (model 2, boyut 6, düzeltme L)
    let veri = f.no.as_bytes();
    b.extend([0x1B, 0x61, 1]); // ortala
    b.extend([0x1D, 0x28, 0x6B, 4, 0, 0x31, 0x41, 0x32, 0]);
    b.extend([0x1D, 0x28, 0x6B, 3, 0, 0x31, 0x43, 6]);
    b.extend([0x1D, 0x28, 0x6B, 3, 0, 0x31, 0x45, 0x30]);
    let n = veri.len() + 3;
    b.extend([
        0x1D,
        0x28,
        0x6B,
        (n % 256) as u8,
        (n / 256) as u8,
        0x31,
        0x50,
        0x30,
    ]);
    b.extend(veri);
    b.extend([0x1D, 0x28, 0x6B, 3, 0, 0x31, 0x51, 0x30]);
    b.extend(*b"\n\n\n");
    b.extend([0x1B, 0x61, 0]);
    b.extend([0x1D, 0x56, 0x42, 0]); // kısmi kesme
    b
}

#[cfg(test)]
mod testler {
    use super::*;

    fn girdi() -> Girdi {
        Girdi {
            tarih: "2026-11-04".into(),
            musteri: " Ayşe Çelik ".into(),
            telefon: "+90 532 123 45 67".into(),
            kalemler: vec![
                Kalem {
                    urun: "Pantolon".into(),
                    islem: "paça kısalt".into(),
                    not: "2 cm".into(),
                },
                Kalem {
                    urun: "Ceket".into(),
                    islem: "kol kısalt".into(),
                    not: "".into(),
                },
            ],
            olcu_notu: "bel 84".into(),
            prova: "2026-11-07".into(),
            teslim: "2026-11-10".into(),
            ucret_kurus: 85000,
            on_odeme_kurus: 20000,
            usta: "Hasan".into(),
        }
    }

    fn yol() -> std::path::PathBuf {
        taa_dosya::gecici_klasor().unwrap().join("fis.jsonl")
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
        assert!(dogrula(&g).is_ok());
        g.on_odeme_kurus = 90000;
        assert!(dogrula(&g).is_err());
        let mut g = girdi();
        g.teslim = "2026-11-01".into();
        assert!(dogrula(&g).is_err());
        let mut g = girdi();
        g.prova = "2026-11-12".into();
        assert!(dogrula(&g).is_err());
        let mut g = girdi();
        g.kalemler.clear();
        assert!(dogrula(&g).is_err());
        let mut g = girdi();
        g.musteri = "  ".into();
        assert!(dogrula(&g).is_err());
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
            musteri: "Ayşe Çelik".into(),
            telefon: "05321234567".into(),
            kalemler: girdi().kalemler,
            olcu_notu: "".into(),
            prova: "".into(),
            teslim: "2026-11-10".into(),
            ucret_kurus: 85000,
            on_odeme_kurus: 20000,
            usta: "".into(),
        };
        let m = duz_metin(&f, 42);
        assert!(m.contains("TERZİ FİŞİ F-20261104-001"));
        assert!(m.contains("Teslim: 10.11.2026"));
        assert!(m.lines().all(|l| l.chars().count() <= 42));
        assert!(m.contains("KALAN") && m.contains("650,00 TL"));
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
}
