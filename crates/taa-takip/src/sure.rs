//! TaaSüre: fiş yazılırken usta yoğunluğuna göre TAHMİNİ teslim tarihi önerisi.
//! Sayı uydurulmaz: işlem süreleri ve usta günlük kapasitesi ustadan gelip `sure.json` dosyasına yazılır.
//! Gömülü varsayılan tablo BOŞTUR; tablo ya da kapasite yoksa sonuç "tahmin yok, elle seç" olur.
//! Öneri yalnız öneridir: fiş hiçbir zaman otomatik tarihlenmez, personel değiştirebilir.
//! Usta karşılaştırması/puanı YOKTUR (kapalı). Sahip: Yusuf.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use taa_fis::{Durum, Fis, Kalem};

pub const OZELLIK: &str = "teslim-oneri";
const VARSAYILAN: &str = include_str!("sure.json");
const GUNLER: [&str; 7] = [
    "pazartesi",
    "sali",
    "carsamba",
    "persembe",
    "cuma",
    "cumartesi",
    "pazar",
];

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SureTablosu {
    /// İşlem kodu -> bir adet işlemin ortalama dakikası (usta verir).
    #[serde(default)]
    pub sureler_dk: BTreeMap<String, u32>,
    /// Zor durum: işlem kodu -> {esik_mm, dk, soru}. Kalemin ölçüsü eşiği aşarsa ya da personel "evet"
    /// derse `dk` kullanılır. Ayrı alan değil, sorulan bir sorudur (Yusuf kararı).
    #[serde(default)]
    pub zor: BTreeMap<String, Zor>,
    /// Bir ustanın günde bu işlere ayırabildiği dakika (usta/Yusuf verir). Yoksa tahmin yapılmaz.
    #[serde(default)]
    pub gunluk_dakika: Option<u32>,
    /// Ek usta adları (fişlerde henüz görünmeyenler).
    #[serde(default)]
    pub ustalar: Vec<String>,
    /// Çalışılmayan haftanın günleri: pazartesi, sali, carsamba, persembe, cuma, cumartesi, pazar.
    #[serde(default)]
    pub kapali_gunler: Vec<String>,
    /// Çalışılmayan tarihler (YYYY-AA-GG), ör. bayram.
    #[serde(default)]
    pub kapali_tarihler: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Zor {
    pub esik_mm: i32,
    pub dk: u32,
    pub soru: String,
}

impl SureTablosu {
    pub fn varsayilan() -> SureTablosu {
        Self::json_den(VARSAYILAN).expect("gömülü süre tablosu geçerli olmalı")
    }

    pub fn json_den(s: &str) -> Result<SureTablosu, String> {
        let t: SureTablosu =
            serde_json::from_str(s).map_err(|e| format!("süre dosyası geçersiz: {e}"))?;
        if t.sureler_dk.values().any(|d| *d == 0)
            || t.gunluk_dakika == Some(0)
            || t.zor.values().any(|z| z.dk == 0 || z.esik_mm <= 0)
        {
            return Err("süre ve günlük dakika 0'dan büyük olmalı".into());
        }
        for g in &t.kapali_gunler {
            if !GUNLER.contains(&g.as_str()) {
                return Err(format!("bilinmeyen gün adı: {g}"));
            }
        }
        if t.kapali_gunler.len() >= 7 {
            return Err("tüm günler kapalı olamaz".into());
        }
        if t.kapali_tarihler.iter().any(|d| gun_no(d).is_none()) {
            return Err("kapalı tarihler YYYY-AA-GG olmalı".into());
        }
        Ok(t)
    }

    /// `dizin/sure.json` varsa onu, yoksa boş gömülü tabloyu yükler. Bozuk dosya hata verir.
    pub fn dizinden(dizin: &Path) -> Result<SureTablosu, String> {
        match std::fs::read_to_string(dizin.join("sure.json")) {
            Ok(s) => Self::json_den(&s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::varsayilan()),
            Err(e) => Err(format!("süre dosyası okunamadı: {e}")),
        }
    }

    fn kapali_mi(&self, gun: i64) -> bool {
        let hafta = GUNLER[(gun + 3).rem_euclid(7) as usize]; // 1970-01-01 perşembe
        self.kapali_gunler.iter().any(|g| g == hafta)
            || self.kapali_tarihler.iter().any(|d| gun_no(d) == Some(gun))
    }

    /// Fiş yazarken personele sorulacak soru (kalemin işleminde zor durum tanımlıysa).
    pub fn zor_soru(&self, k: &Kalem) -> Option<&str> {
        k.islem_kodu
            .as_deref()
            .and_then(|c| self.zor.get(c))
            .map(|z| z.soru.as_str())
    }

    /// `zor_evet`: personel zor-durum sorusuna "evet" dedi mi. Ölçü eşiği aşılmışsa da zor sayılır.
    fn kalem_dk(&self, k: &Kalem, zor_evet: bool) -> Option<u32> {
        let kod = k.islem_kodu.as_deref()?;
        if let Some(z) = self.zor.get(kod) {
            if zor_evet || k.olcu_mm.is_some_and(|m| m > z.esik_mm) {
                return Some(z.dk);
            }
        }
        k.varyant
            .as_deref()
            .and_then(|v| self.sureler_dk.get(&format!("{kod}:{v}")).copied())
            .or_else(|| self.sureler_dk.get(kod).copied())
    }
}

/// YYYY-AA-GG -> 1970-01-01'den beri gün sayısı (geçersizse None).
fn gun_no(s: &str) -> Option<i64> {
    let p: Vec<&str> = s.split('-').collect();
    if p.len() != 3 || p[0].len() != 4 || p[1].len() != 2 || p[2].len() != 2 {
        return None;
    }
    let (y, m, d): (i64, i64, i64) = (p[0].parse().ok()?, p[1].parse().ok()?, p[2].parse().ok()?);
    if !(1..=12).contains(&m) || d < 1 || d > ay_gunu(y, m) {
        return None;
    }
    let y2 = if m <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146097 + doe - 719468)
}

fn ay_gunu(y: i64, m: i64) -> i64 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

fn gun_tarih(n: i64) -> String {
    let z = n + 719468;
    let era = z.div_euclid(146097);
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + if m <= 2 { 1 } else { 0 };
    format!("{y:04}-{m:02}-{d:02}")
}

fn acik(f: &Fis) -> bool {
    matches!(f.durum, Durum::Bekliyor | Durum::Terzihanede | Durum::Hazir)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(tag = "sonuc", rename_all = "snake_case")]
pub enum Oneri {
    /// Öneri var; personel değiştirebilir.
    Tarih {
        tarih: String,
        /// Önerinin dayandığı usta.
        usta: String,
        /// Ustanın kuyruğunda bekleyen iş dakikası (süresi bilinenler).
        kuyruk_dk: u32,
        /// Kuyrukta süresi tabloda olmayan iş sayısı; >0 ise tarih eksik olabilir.
        suresiz_kuyruk_isi: usize,
        /// En erken uygun ustanın bile kuyruğu en az bir tam günlükse true.
        yogun: bool,
        gerekce: String,
    },
    /// Sayı yok ya da özellik kapalı: elle seçilir.
    TahminYok { neden: String },
}

/// Yeni (henüz kaydedilmemiş) fiş için teslim tarihi önerir.
/// `zor_kalemler`: personelin zor-durum sorusuna "evet" dediği kalem sıraları (`SureTablosu::zor_soru`).
/// `kalemler`: yeni fişin kalemleri; `usta`: fişte atanmışsa adı (boşsa en az yüklü usta seçilir);
/// `bugun`: YYYY-AA-GG; `acik_fisler`: defterdeki tüm fişler (açık olanlar kuyruğa sayılır).
pub fn oner(
    tablo: &SureTablosu,
    kalemler: &[Kalem],
    zor_kalemler: &[usize],
    usta: &str,
    bugun: &str,
    fisler: &[Fis],
) -> Oneri {
    if !taa_ayar::ozellik_acik(OZELLIK, true) {
        return Oneri::TahminYok {
            neden: taa_ayar::kapali_mesaji(OZELLIK),
        };
    }
    oner_cekirdek(tablo, kalemler, zor_kalemler, usta, bugun, fisler)
}

fn oner_cekirdek(
    tablo: &SureTablosu,
    kalemler: &[Kalem],
    zor_kalemler: &[usize],
    usta: &str,
    bugun: &str,
    fisler: &[Fis],
) -> Oneri {
    let yok = |n: &str| Oneri::TahminYok { neden: n.into() };
    let Some(gunluk) = tablo.gunluk_dakika else {
        return yok("tahmin yok, elle seç: ustanın günlük çalışma süresi (sure.json) girilmemiş");
    };
    let Some(bugun_no) = gun_no(bugun) else {
        return yok("bugünün tarihi geçersiz");
    };
    if kalemler.is_empty() {
        return yok("tahmin yok, elle seç: fişte işlem yok");
    }
    let mut yeni_dk: u32 = 0;
    for (i, k) in kalemler.iter().enumerate() {
        match tablo.kalem_dk(k, zor_kalemler.contains(&i)) {
            Some(d) => yeni_dk += d,
            None => {
                return yok(&format!(
                    "tahmin yok, elle seç: '{}' işleminin süresi tabloda yok",
                    if k.islem.is_empty() {
                        &k.urun
                    } else {
                        &k.islem
                    }
                ))
            }
        }
    }

    // usta -> (bilinen dakika, süresiz iş sayısı)
    let mut kuyruk: BTreeMap<String, (u32, usize)> = BTreeMap::new();
    for u in &tablo.ustalar {
        kuyruk.entry(u.trim().to_string()).or_default();
    }
    for f in fisler.iter().filter(|f| acik(f)) {
        let u = f.usta.trim();
        if u.is_empty() {
            continue;
        }
        let e = kuyruk.entry(u.to_string()).or_default();
        for k in &f.kalemler {
            match tablo.kalem_dk(k, false) {
                Some(d) => e.0 += d,
                None => e.1 += 1,
            }
        }
    }

    let usta = usta.trim();
    let (ad, (bekleyen, suresiz)) = if !usta.is_empty() {
        (
            usta.to_string(),
            kuyruk.get(usta).copied().unwrap_or_default(),
        )
    } else if let Some((a, v)) = kuyruk.iter().min_by_key(|(_, v)| v.0) {
        (a.clone(), *v)
    } else {
        ("(atanmamış)".to_string(), (0, 0))
    };

    let toplam = bekleyen + yeni_dk;
    let gun_sayisi = toplam.div_ceil(gunluk).max(1);
    // Bugün (açıksa) 1. iş günü sayılır; kapalı günler atlanır. Güvenlik: en çok 366 gün aranır.
    let mut gun = bugun_no;
    let mut sayilan = 0;
    for _ in 0..366 {
        if !tablo.kapali_mi(gun) {
            sayilan += 1;
            if sayilan == gun_sayisi {
                break;
            }
        }
        gun += 1;
    }
    if sayilan != gun_sayisi {
        return yok("tahmin yok, elle seç: yıl içinde uygun iş günü bulunamadı");
    }
    let yogun = bekleyen >= gunluk;
    let mut gerekce = format!(
        "{ad}: kuyrukta {bekleyen} dk iş + bu fiş {yeni_dk} dk = {toplam} dk; günde {gunluk} dk → {gun_sayisi} iş günü"
    );
    if suresiz > 0 {
        gerekce.push_str(&format!(
            "; kuyruktaki {suresiz} işin süresi tabloda yok, tarih eksik olabilir"
        ));
    }
    if yogun {
        gerekce.push_str("; usta yoğun");
    }
    Oneri::Tarih {
        tarih: gun_tarih(gun),
        usta: ad,
        kuyruk_dk: bekleyen,
        suresiz_kuyruk_isi: suresiz,
        yogun,
        gerekce,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn kalem(kod: Option<&str>) -> Kalem {
        Kalem {
            urun: "Pantolon".into(),
            islem: "paça".into(),
            not: String::new(),
            islem_kodu: kod.map(String::from),
            varyant: None,
            olcu_mm: None,
            pay_mm: None,
            usta_onayli: false,
            igneli: false,
        }
    }

    fn tablo(json: &str) -> SureTablosu {
        SureTablosu::json_den(json).unwrap()
    }

    #[test]
    fn bos_tablo_tahmin_yok_gomulu_tablo_tahmin_verir() {
        let bos = tablo("{}");
        match oner_cekirdek(&bos, &[kalem(Some("x"))], &[], "", "2026-11-05", &[]) {
            Oneri::TahminYok { neden } => assert!(neden.contains("elle seç")),
            o => panic!("{o:?}"),
        }
        let t = SureTablosu::varsayilan();
        assert_eq!(t.gunluk_dakika, Some(510));
        // 2026-11-05 perşembe, boş kuyrukta 30 dk'lık iş aynı gün
        match oner_cekirdek(
            &t,
            &[kalem(Some("cek-kol-kisalt"))],
            &[],
            "",
            "2026-11-05",
            &[],
        ) {
            Oneri::Tarih { tarih, .. } => assert_eq!(tarih, "2026-11-05"),
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn tarih_matematigi() {
        assert_eq!(gun_no("1970-01-01"), Some(0));
        assert_eq!(gun_tarih(gun_no("2026-11-05").unwrap()), "2026-11-05");
        assert_eq!(gun_tarih(gun_no("2028-02-29").unwrap() + 1), "2028-03-01");
        assert!(gun_no("2026-02-29").is_none());
        // 2026-11-05 perşembe (hafta günü hesabı)
        let t = tablo(r#"{"kapali_gunler":["persembe"]}"#);
        assert!(t.kapali_mi(gun_no("2026-11-05").unwrap()));
        assert!(!t.kapali_mi(gun_no("2026-11-06").unwrap()));
    }

    #[test]
    fn suresiz_islem_tahmini_durdurur() {
        let t = tablo(r#"{"gunluk_dakika":100,"sureler_dk":{"a":10}}"#);
        match oner_cekirdek(&t, &[kalem(Some("b"))], &[], "", "2026-11-05", &[]) {
            Oneri::TahminYok { neden } => assert!(neden.contains("süresi tabloda yok")),
            o => panic!("{o:?}"),
        }
        assert!(matches!(
            oner_cekirdek(&t, &[kalem(None)], &[], "", "2026-11-05", &[]),
            Oneri::TahminYok { .. }
        ));
    }

    #[test]
    fn bos_kuyrukta_bugun_ve_kapali_gun_atlanir() {
        let t =
            tablo(r#"{"gunluk_dakika":100,"sureler_dk":{"a":30},"kapali_gunler":["persembe"]}"#);
        // bugün perşembe (kapalı) -> ilk iş günü cuma
        match oner_cekirdek(&t, &[kalem(Some("a"))], &[], "Ali", "2026-11-05", &[]) {
            Oneri::Tarih { tarih, yogun, .. } => {
                assert_eq!(tarih, "2026-11-06");
                assert!(!yogun);
            }
            o => panic!("{o:?}"),
        }
    }

    fn fis(usta: &str, kod: &str, durum: Durum) -> Fis {
        Fis {
            no: "F".into(),
            tarih: "2026-11-01".into(),
            musteri: "A".into(),
            telefon: "0".into(),
            kalemler: vec![kalem(Some(kod))],
            olcu_notu: String::new(),
            prova: String::new(),
            teslim: "2026-11-09".into(),
            teslim_saati: "17:00".into(),
            ucret_kurus: 0,
            on_odeme_kurus: 0,
            usta: usta.into(),
            satis_danismani: String::new(),
            olcu_alan: String::new(),
            kargo: false,
            kargo_notu: String::new(),
            kargo_ad: String::new(),
            kargo_adres: String::new(),
            kargo_telefon: String::new(),
            bildirim_izni: false,
            teslim_degisiklikleri: Vec::new(),
            durum,
            raf: String::new(),
            imzalar: Vec::new(),
        }
    }

    #[test]
    fn kuyruk_tarihi_ileri_atar_ve_en_az_yuklu_ustayi_secer() {
        let t = tablo(r#"{"gunluk_dakika":100,"sureler_dk":{"a":60}}"#);
        let fisler = vec![
            fis("Ali", "a", Durum::Bekliyor),
            fis("Ali", "a", Durum::Terzihanede),
            fis("Veli", "a", Durum::Bekliyor),
            fis("Ali", "a", Durum::Teslim), // kapalı iş kuyruğa girmez
        ];
        match oner_cekirdek(&t, &[kalem(Some("a"))], &[], "", "2026-11-02", &fisler) {
            Oneri::Tarih {
                tarih,
                usta,
                kuyruk_dk,
                yogun,
                ..
            } => {
                assert_eq!(usta, "Veli");
                assert_eq!(kuyruk_dk, 60);
                assert_eq!(tarih, "2026-11-03"); // 120 dk -> 2 iş günü
                assert!(!yogun);
            }
            o => panic!("{o:?}"),
        }
        // atanmış yoğun usta: 120+60=180 dk -> 2 gün, yoğun
        match oner_cekirdek(&t, &[kalem(Some("a"))], &[], "Ali", "2026-11-02", &fisler) {
            Oneri::Tarih {
                tarih,
                yogun,
                gerekce,
                ..
            } => {
                assert_eq!(tarih, "2026-11-03");
                assert!(yogun && gerekce.contains("usta yoğun"));
            }
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn suresiz_kuyruk_isi_uyarilir() {
        let t = tablo(r#"{"gunluk_dakika":100,"sureler_dk":{"a":10}}"#);
        let fisler = vec![fis("Ali", "bilinmeyen", Durum::Bekliyor)];
        match oner_cekirdek(&t, &[kalem(Some("a"))], &[], "Ali", "2026-11-02", &fisler) {
            Oneri::Tarih {
                suresiz_kuyruk_isi,
                gerekce,
                ..
            } => {
                assert_eq!(suresiz_kuyruk_isi, 1);
                assert!(gerekce.contains("eksik olabilir"));
            }
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn duble_paca_ayri_sure_ve_zor_durum() {
        let t = tablo(
            r#"{"gunluk_dakika":600,"sureler_dk":{"p":5,"p:Duble paça":8,"b":10},
                "zor":{"b":{"esik_mm":40,"dk":60,"soru":"4 cm'den fazla mı?"}}}"#,
        );
        let mut duble = kalem(Some("p"));
        duble.varyant = Some("Duble paça".into());
        assert_eq!(t.kalem_dk(&duble, false), Some(8));
        assert_eq!(t.kalem_dk(&kalem(Some("p")), false), Some(5));
        let b = kalem(Some("b"));
        assert_eq!(t.zor_soru(&b), Some("4 cm'den fazla mı?"));
        assert_eq!(t.zor_soru(&duble), None);
        assert_eq!(t.kalem_dk(&b, false), Some(10));
        assert_eq!(t.kalem_dk(&b, true), Some(60)); // personel "evet" dedi
        let mut olcu = kalem(Some("b"));
        olcu.olcu_mm = Some(41);
        assert_eq!(t.kalem_dk(&olcu, false), Some(60)); // ölçü eşiği aştı
        olcu.olcu_mm = Some(40);
        assert_eq!(t.kalem_dk(&olcu, false), Some(10));
        // 600 dk günde, zor bel 60 dk -> aynı gün
        match oner_cekirdek(&t, &[b], &[0], "Ali", "2026-11-02", &[]) {
            Oneri::Tarih { tarih, gerekce, .. } => {
                assert_eq!(tarih, "2026-11-02");
                assert!(gerekce.contains("60 dk"));
            }
            o => panic!("{o:?}"),
        }
    }

    #[test]
    fn gomulu_tablo_gecerli_ve_katalogla_uyumlu() {
        let t = SureTablosu::varsayilan();
        assert_eq!(t.sureler_dk["pant-paca-kisalt:Duble paça"], 8);
        assert_eq!(t.zor["pant-bel-daralt"].dk, 60);
    }

    #[test]
    fn gecersiz_dosya_reddedilir() {
        assert!(SureTablosu::json_den(r#"{"gunluk_dakika":0}"#).is_err());
        assert!(SureTablosu::json_den(r#"{"kapali_gunler":["xx"]}"#).is_err());
        assert!(SureTablosu::json_den(r#"{"kapali_tarihler":["2026-13-01"]}"#).is_err());
        assert!(SureTablosu::json_den("bozuk").is_err());
    }
}
