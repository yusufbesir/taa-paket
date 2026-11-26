//! Eski fişler için ayarlanabilir geçiş süresi (koordinatör kararı 2026-11-26, geri alınabilir varsayılan).
//!
//! Fişli kanıt (5b1f004, `teslim_sepet`): müşterinin fişi yalnız gizli anahtarlı karekodla (takip kodu ya da `&k=`
//! parçalı personel karekodu) "fişli" sayılır. Bu mekanizmadan ÖNCE basılmış fişlerin karekodunda kanıt parçası
//! yoktur; müşteri fişiyle gelse de fişsiz sayılır (son 4 + imza + fişsiz teslim yetkisi gerekir).
//!
//! Geçiş süresi: yönetici "eski fiş geçiş süresi"ni gün olarak verir (varsayılan 0 = kapalı, en çok 90). Süre ayarın
//! kaydedildiği andan başlar ve bitince KENDİLİĞİNDEN kapanır (ayar dosyası değişmez; yalnız karar zamana bakar).
//! Yeniden kaydetmek süreyi yeniler, ama geçiş `fisli_kanit_baslangic`'tan `MUTLAK_SON_GUN` (180) gün sonra hiçbir
//! şekilde açık kalmaz (toplam süre sınırlı; bu sınır koordinatöre bildirilen, geri alınabilir bir varsayılandır).
//! Açıkken, kanıt mekanizması devreye girmeden önce basılmış fişlerde personel karekodu (kanıt parçasız) ya da elle
//! yazılan fiş no "fişli" sayılır. "Önce basılmış": fiş tarihi < `fisli_kanit_baslangic` (programın bu sürümle ilk
//! açıldığı gün — teslim sepeti günlüğü daha eskiyse, yani kanıt mekanizması 5b1f004 ile daha önce çalıştıysa o gün;
//! ayar dosyasına ilk açılışta bir kez yazılır, yönetim günlüğüne de kaydedilir, dosya silinirse günlükten geri
//! alınır, dosya bozuksa yeniden yazılmaz ve geçiş kapalı kalır), ya da aynı gün ve fiş no ≤ `fisli_kanit_son_eski_fis`
//! (ilk açılışta defterdeki son fiş; sıra sayısal karşılaştırılır). Takip karekodu ve kanıtlı karekod zaten fişlidir; terzihane nüshasının kanıtlı
//! karekodu (takip ayarlıyken) geçişle de fişli SAYILMAZ. Bağlı zincirden otomatik gelen (okutulmamış) fiş fişsiz kalır.
//!
//! Her kullanım `teslim-sepet.jsonl` günlüğüne yazılır (olay `eski_fis_gecisi`: fiş no'ları, kim, zaman; kişisel veri
//! yok). Satır teslim kaydından HEMEN ÖNCE (tüm denetimler geçtikten sonra) yazılır; yazılamazsa teslim yapılmaz
//! (geçişle fişli teslim günlüksüz olmaz). Kayıt sonra başarısız olursa aynı teslim_id ile hata satırı düşülür. Ayar değişikliği `eslestirme.jsonl` yönetim günlüğüne (kim, eski/yeni gün) yazılır. Ayar yalnız yönetici
//! oturumuyla, bu bilgisayardan değişir (arayüzde Ayarlar > Yönetim).
//!
//! Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

use super::*;
use serde::{Deserialize, Serialize};

/// Ayar dosyası (veri klasöründe).
pub const AYAR_DOSYASI: &str = "yonetim-ayar.json";
/// Geçiş süresinin üst sınırı (gün; tek kayıtta).
pub const EN_COK_GUN: u64 = 90;
/// Toplam sınır: geçiş, `fisli_kanit_baslangic`'tan bu kadar gün sonra yeniden kaydedilse de açık kalmaz.
pub const MUTLAK_SON_GUN: u64 = 180;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct YonetimAyari {
    /// Fişli kanıtın devreye girdiği gün (YYYY-AA-GG). İlk açılışta bir kez yazılır, sonra değişmez.
    #[serde(default)]
    pub fisli_kanit_baslangic: String,
    /// İlk açılışta defterdeki son fiş no (aynı gün basılmış eski fişler için).
    #[serde(default)]
    pub fisli_kanit_son_eski_fis: String,
    /// Eski fiş geçiş süresi (gün); 0 = kapalı.
    #[serde(default)]
    pub eski_fis_gecis_gun: u64,
    /// Sürenin başladığı an (Unix saniyesi; ayarın kaydedildiği an).
    #[serde(default)]
    pub eski_fis_gecis_ayar_sn: u64,
    /// Ayarı kaydeden (yönetici oturumunun kişisi).
    #[serde(default)]
    pub eski_fis_gecis_ayarlayan: String,
    /// TAA ortak girişte yönetici sayılan roller (TAA Çatı rol adları). Boş = `yonetici::YONETICI_ROLU` ("yonetici").
    /// Arayüzde yok; gerekirse dosyada elle düzenlenir.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub yonetici_rolleri: Vec<String>,
}

impl YonetimAyari {
    /// Ayar (okunamazsa varsayılan: başlangıç boş olduğundan geçiş KAPALI sayılır).
    pub fn oku(dizin: &Path) -> Self {
        Self::oku_sonuc(dizin).ok().flatten().unwrap_or_default()
    }

    /// Dosya yoksa `Ok(None)`; var ama okunamıyor/bozuksa `Err` (başlangıç yeniden yazılmaz, geçiş kapalı kalır).
    pub fn oku_sonuc(dizin: &Path) -> Result<Option<Self>, String> {
        match std::fs::read_to_string(dizin.join(AYAR_DOSYASI)) {
            Ok(m) => serde_json::from_str(&m)
                .map(Some)
                .map_err(|e| format!("{AYAR_DOSYASI} bozuk: {e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(format!("{AYAR_DOSYASI} okunamadı: {e}")),
        }
    }

    fn yaz(&self, dizin: &Path) -> Result<(), String> {
        let m = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        taa_dosya::uzerine_yaz_atomik(&dizin.join(AYAR_DOSYASI), m.as_bytes())
            .map_err(|e| format!("yönetim ayarı yazılamadı: {e}"))
    }

    /// Sürenin bittiği an (kapalıysa `None`).
    /// Süre, ayarın kaydedildiği andan `eski_fis_gecis_gun` gün sonra biter; yeniden kaydetmek süreyi yeniler ama
    /// bitiş hiçbir zaman `fisli_kanit_baslangic` + `MUTLAK_SON_GUN` günü aşmaz (toplam geçiş sınırlı).
    pub fn gecis_bitis_sn(&self) -> Option<u64> {
        if self.eski_fis_gecis_gun == 0 {
            return None;
        }
        let b = self.eski_fis_gecis_ayar_sn + self.eski_fis_gecis_gun * 86_400;
        Some(match self.mutlak_son_sn() {
            Some(m) => b.min(m),
            None => b,
        })
    }

    /// Geçişin açık olabileceği son an: başlangıç gününün (TR) başı + `MUTLAK_SON_GUN` gün.
    pub fn mutlak_son_sn(&self) -> Option<u64> {
        gun_basi_sn(&self.fisli_kanit_baslangic).map(|g| g + MUTLAK_SON_GUN * 86_400)
    }

    /// Geçiş şu an açık mı (süre bitince kendiliğinden kapanır)?
    pub fn gecis_acik(&self, simdi: u64) -> bool {
        !self.fisli_kanit_baslangic.is_empty() && self.gecis_bitis_sn().is_some_and(|b| simdi < b)
    }

    /// Fiş, fişli kanıt mekanizmasından önce mi basıldı?
    pub fn eski_fis(&self, f: &taa_fis::Fis) -> bool {
        let b = self.fisli_kanit_baslangic.as_str();
        if b.is_empty() || f.tarih.is_empty() {
            return false;
        }
        f.tarih.as_str() < b
            || (f.tarih == b
                && !self.fisli_kanit_son_eski_fis.is_empty()
                && no_anahtari(&f.no) <= no_anahtari(&self.fisli_kanit_son_eski_fis))
    }
}

/// Fiş no'sunun sıralama anahtarı: "F-YYYYAAGG-NNN" için (gün, sıra sayısı); günde 999'u aşan sıra da doğru
/// sıralanır (metin sıralaması "F-…-1000" < "F-…-999" derdi). Biçim dışı no'lar metin olarak sıralanır.
pub(crate) fn no_anahtari(no: &str) -> (String, u64, String) {
    match no.rsplit_once('-') {
        Some((on, sira)) if !sira.is_empty() && sira.bytes().all(|b| b.is_ascii_digit()) => (
            on.to_string(),
            sira.parse().unwrap_or(u64::MAX),
            String::new(),
        ),
        _ => (String::new(), 0, no.to_string()),
    }
}

/// "YYYY-AA-GG" gününün Türkiye saatiyle başlangıcı (Unix sn); biçim dışıysa `None`.
fn gun_basi_sn(t: &str) -> Option<u64> {
    let p: Vec<i64> = t
        .split('-')
        .map(|x| x.parse().ok())
        .collect::<Option<_>>()?;
    let [y, m, d] = p[..] else { return None };
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    // Gün sayısı (1970-01-01'den), Howard Hinnant'ın days_from_civil'i.
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let gun = era * 146_097 + doe - 719_468;
    u64::try_from(gun * 86_400 - 3 * 3600).ok()
}

/// Yönetim günlüğündeki (`eslestirme.jsonl`) ilk `fisli_kanit_baslangic` kaydı: ayar dosyası silinirse başlangıç
/// bugüne kaymasın diye buradan geri alınır.
fn gunlukteki_baslangic(dizin: &Path) -> Option<(String, String)> {
    taa_dosya::satirlari_oku(&dizin.join("eslestirme.jsonl"))
        .ok()?
        .iter()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .find(|v| v["olay"] == "fisli_kanit_baslangic")
        .and_then(|v| {
            let b = v["baslangic"].as_str()?.to_string();
            gun_basi_sn(&b)?;
            Some((b, v["son_eski_fis"].as_str().unwrap_or("").to_string()))
        })
}

/// Teslim sepeti günlüğündeki (5b1f004 ile gelen, kanıt mekanizmasıyla birlikte yazılan) en eski gün: bu sürüm ilk
/// açıldığında kanıt mekanizması daha önce çalışmışsa başlangıç o güne çekilir (aradaki kanıtlı fişler "eski" sayılmaz).
fn sepet_gunlugundeki_ilk_gun(dizin: &Path) -> Option<String> {
    taa_dosya::satirlari_oku(&dizin.join("teslim-sepet.jsonl"))
        .ok()?
        .iter()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|v| v["tarih"].as_str().map(str::to_string))
        .filter(|t| gun_basi_sn(t).is_some())
        .min()
}

/// İlk açılışta `fisli_kanit_baslangic` ve defterdeki son fiş no'sunu bir kez yazar; varsa dokunmaz.
/// - Dosya var ama okunamıyor/bozuksa `Err`: başlangıç YENİDEN YAZILMAZ (geçiş kapalı kalır; yönetici dosyayı düzeltir).
/// - Dosya yoksa: yönetim günlüğünde önceki başlangıç kaydı varsa o geri yazılır (silinen dosya kapsamı genişletmez);
///   yoksa başlangıç = bugün, ama teslim sepeti günlüğü (kanıt mekanizmasıyla gelen) daha eskiyse o gün (o zaman aynı
///   günün fişleri "eski" sayılmaz: `fisli_kanit_son_eski_fis` boş). İlk yazım yönetim günlüğüne de kaydedilir.
pub fn baslangic_yaz_yoksa(dizin: &Path, simdi: u64) -> Result<YonetimAyari, String> {
    let mut a = YonetimAyari::oku_sonuc(dizin)?.unwrap_or_default();
    if !a.fisli_kanit_baslangic.is_empty() {
        return Ok(a);
    }
    let (baslangic, son, kaynak) = match gunlukteki_baslangic(dizin) {
        Some((b, s)) => (b, s, "gunluk"),
        None => {
            let bugun = tarih_iso(simdi);
            match sepet_gunlugundeki_ilk_gun(dizin).filter(|g| *g < bugun) {
                Some(g) => (g, String::new(), "teslim-sepet-gunlugu"),
                None => {
                    let yol = dizin.join("fisler.jsonl");
                    let son = Defter { yol: &yol }
                        .fisler()
                        .unwrap_or_default()
                        .into_iter()
                        .map(|f| f.no)
                        .max_by_key(|n| no_anahtari(n))
                        .unwrap_or_default();
                    (bugun, son, "ilk-acilis")
                }
            }
        }
    };
    a.fisli_kanit_baslangic = baslangic;
    a.fisli_kanit_son_eski_fis = son;
    a.yaz(dizin)?;
    if kaynak != "gunluk" {
        let satir = json!({"olay": "fisli_kanit_baslangic", "baslangic": a.fisli_kanit_baslangic, "son_eski_fis": a.fisli_kanit_son_eski_fis, "kaynak": kaynak, "tarih": tarih_iso(simdi), "saat": saat_iso(simdi)}).to_string();
        let _ = taa_dosya::satir_ekle(&dizin.join("eslestirme.jsonl"), &satir);
    }
    Ok(a)
}

impl Uygulama {
    /// Bu kanıt, eski fiş geçiş süresiyle "fişli" sayılır mı? (Yalnız kanıt parçasız personel karekodu ve elle no.)
    pub(crate) fn gecis_fisli(&self, k: teslim_sepet::Kanit, f: &taa_fis::Fis) -> bool {
        if !matches!(
            k,
            teslim_sepet::Kanit::KarekodKanitsiz | teslim_sepet::Kanit::Elle
        ) {
            return false;
        }
        let a = YonetimAyari::oku(&self.dizin);
        a.gecis_acik(self.simdi()) && a.eski_fis(f)
    }

    fn ayar_json(&self, a: &YonetimAyari) -> serde_json::Value {
        let simdi = self.simdi();
        let bitis = a.gecis_bitis_sn();
        json!({
            "eski_fis_gecis_gun": a.eski_fis_gecis_gun,
            "eski_fis_gecis_acik": a.gecis_acik(simdi),
            "eski_fis_gecis_baslangic": (a.eski_fis_gecis_gun > 0).then(|| tarih_iso(a.eski_fis_gecis_ayar_sn)),
            "eski_fis_gecis_bitis": bitis.map(tarih_iso),
            "eski_fis_gecis_bitis_saat": bitis.map(saat_iso),
            "eski_fis_gecis_kalan_gun": bitis.filter(|b| *b > simdi).map(|b| (b - simdi).div_ceil(86_400)),
            "eski_fis_gecis_ayarlayan": a.eski_fis_gecis_ayarlayan,
            "fisli_kanit_baslangic": a.fisli_kanit_baslangic,
            "en_cok_gun": EN_COK_GUN,
            "mutlak_son": a.mutlak_son_sn().map(tarih_iso),
        })
    }

    /// GET /api/yonetim/ayar: yönetim ayarları (yalnız bu bilgisayar + yönetici oturumu).
    pub(crate) fn yonetim_ayari(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        match baslangic_yaz_yoksa(&self.dizin, self.simdi()) {
            Ok(a) => Yanit::json(200, self.ayar_json(&a)),
            Err(e) => Yanit::hata(500, &e),
        }
    }

    /// POST /api/yonetim/ayar {eski_fis_gecis_gun}: 0 kapatır; >0 süreyi ŞİMDİDEN başlatır (yeniden kaydetmek süreyi
    /// yeniler). Yalnız bu bilgisayar + yönetici oturumu; değişiklik yönetim günlüğüne yazılır.
    pub(crate) fn yonetim_ayari_yaz(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let Some(gun) = v["eski_fis_gecis_gun"]
            .as_u64()
            .filter(|g| *g <= EN_COK_GUN)
        else {
            return Yanit::hata(
                422,
                &format!("eski_fis_gecis_gun 0 ile {EN_COK_GUN} arasında tam sayı olmalı"),
            );
        };
        let simdi = self.simdi();
        let mut a = match baslangic_yaz_yoksa(&self.dizin, simdi) {
            Ok(a) => a,
            Err(e) => return Yanit::hata(500, &e),
        };
        if gun > 0 && a.mutlak_son_sn().is_some_and(|m| simdi >= m) {
            return Yanit::hata(
                422,
                &format!(
                    "eski fiş geçişi en geç karekod kanıtı başlangıcından {MUTLAK_SON_GUN} gün sonrasına kadar açılabilir; bu süre doldu"
                ),
            );
        }
        let eski = a.eski_fis_gecis_gun;
        let kim = self.yonetici_kimi(i);
        a.eski_fis_gecis_gun = gun;
        a.eski_fis_gecis_ayar_sn = if gun > 0 { simdi } else { 0 };
        a.eski_fis_gecis_ayarlayan = kim.clone();
        if let Err(e) = a.yaz(&self.dizin) {
            return Yanit::hata(500, &e);
        }
        let satir = json!({"olay": "ayar", "ayar": "eski_fis_gecis_gun", "eski": eski, "yeni": gun, "kim": kim, "kim_kaynak": self.kim_kaynagi(i), "tarih": tarih_iso(simdi), "saat": saat_iso(simdi)}).to_string();
        let _ = taa_dosya::satir_ekle(&self.dizin.join("eslestirme.jsonl"), &satir);
        Yanit::json(200, self.ayar_json(&a))
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn fis(no: &str, tarih: &str) -> taa_fis::Fis {
        serde_json::from_value(json!({
            "no": no, "tarih": tarih, "musteri": "Deneme", "telefon": "05000000000",
            "kalemler": [], "teslim": tarih, "ucret_kurus": 0, "on_odeme_kurus": 0, "durum": "hazir"
        }))
        .unwrap()
    }

    #[test]
    fn eski_fis_ve_sure_kendiliginden_kapanir() {
        let a = YonetimAyari {
            fisli_kanit_baslangic: "2026-11-26".into(),
            fisli_kanit_son_eski_fis: "F-20261126-003".into(),
            eski_fis_gecis_gun: 2,
            eski_fis_gecis_ayar_sn: 1_000_000,
            eski_fis_gecis_ayarlayan: "deneme".into(),
            yonetici_rolleri: vec![],
        };
        assert!(a.eski_fis(&fis("F-20261125-009", "2026-11-25")));
        assert!(a.eski_fis(&fis("F-20261126-003", "2026-11-26")));
        assert!(!a.eski_fis(&fis("F-20261126-004", "2026-11-26")));
        assert!(!a.eski_fis(&fis("F-20261127-001", "2026-11-27")));
        assert!(a.gecis_acik(1_000_000));
        assert!(a.gecis_acik(1_000_000 + 2 * 86_400 - 1));
        assert!(
            !a.gecis_acik(1_000_000 + 2 * 86_400),
            "süre bitince kapanır"
        );
        let kapali = YonetimAyari {
            eski_fis_gecis_gun: 0,
            ..a.clone()
        };
        assert!(!kapali.gecis_acik(1_000_000));
        // Başlangıç yazılmamışsa (dosya yok) geçiş hiç açılmaz.
        let bos = YonetimAyari {
            fisli_kanit_baslangic: String::new(),
            ..a
        };
        assert!(!bos.gecis_acik(1_000_000) && !bos.eski_fis(&fis("F-20200101-001", "2020-01-01")));
    }

    #[test]
    fn baslangic_bir_kez_yazilir() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let a = baslangic_yaz_yoksa(&d, 1_800_000_000).unwrap();
        assert_eq!(a.fisli_kanit_baslangic, tarih_iso(1_800_000_000));
        let b = baslangic_yaz_yoksa(&d, 1_900_000_000).unwrap();
        assert_eq!(b.fisli_kanit_baslangic, a.fisli_kanit_baslangic, "değişmez");
        // Dosya silinirse başlangıç bugüne kaymaz: yönetim günlüğünden geri alınır (kapsam genişlemez).
        std::fs::remove_file(d.join(AYAR_DOSYASI)).unwrap();
        let c = baslangic_yaz_yoksa(&d, 1_900_000_000).unwrap();
        assert_eq!(
            c.fisli_kanit_baslangic, a.fisli_kanit_baslangic,
            "günlükten"
        );
        // Dosya bozuksa yeniden yazılmaz: hata, geçiş kapalı (oku varsayılan döner, başlangıç boş).
        std::fs::write(d.join(AYAR_DOSYASI), "{bozuk").unwrap();
        assert!(baslangic_yaz_yoksa(&d, 1_900_000_000).is_err());
        assert_eq!(
            std::fs::read_to_string(d.join(AYAR_DOSYASI)).unwrap(),
            "{bozuk",
            "dokunulmaz"
        );
        assert!(!YonetimAyari::oku(&d).gecis_acik(1_900_000_000));
    }

    /// Kanıt mekanizması (5b1f004, teslim sepeti günlüğü) bu sürümden önce çalıştıysa başlangıç o güne çekilir.
    #[test]
    fn baslangic_sepet_gunlugunden() {
        let d = taa_dosya::gecici_klasor().unwrap();
        std::fs::write(
            d.join("teslim-sepet.jsonl"),
            "{\"olay\":\"teslim_sepet\",\"tarih\":\"2026-11-20\"}\n{\"olay\":\"x\",\"tarih\":\"2026-11-24\"}\n",
        )
        .unwrap();
        let a = baslangic_yaz_yoksa(&d, 1_800_000_000).unwrap();
        assert_eq!(a.fisli_kanit_baslangic, "2026-11-20");
        assert_eq!(
            a.fisli_kanit_son_eski_fis, "",
            "aynı günün fişleri eski sayılmaz"
        );
        assert!(!a.eski_fis(&fis("F-20261120-001", "2026-11-20")));
        assert!(a.eski_fis(&fis("F-20261119-001", "2026-11-19")));
    }

    #[test]
    fn fis_no_sayisal_ve_mutlak_son() {
        assert!(no_anahtari("F-20261126-999") < no_anahtari("F-20261126-1000"));
        assert!(no_anahtari("F-20261126-003") < no_anahtari("F-20261126-004"));
        let a = YonetimAyari {
            fisli_kanit_baslangic: "2026-11-26".into(),
            fisli_kanit_son_eski_fis: "F-20261126-999".into(),
            ..Default::default()
        };
        assert!(a.eski_fis(&fis("F-20261126-998", "2026-11-26")));
        assert!(!a.eski_fis(&fis("F-20261126-1000", "2026-11-26")));
        // 2026-11-26 00:00 TR = 2026-11-25 21:00 UTC.
        let bas = gun_basi_sn("2026-11-26").unwrap();
        assert_eq!(tarih_iso(bas), "2026-11-26");
        assert_eq!(tarih_iso(bas - 1), "2026-11-25");
        // Yeniden kaydetmek süreyi yeniler ama başlangıç + MUTLAK_SON_GUN aşılmaz.
        let b = YonetimAyari {
            eski_fis_gecis_gun: 90,
            eski_fis_gecis_ayar_sn: bas + 170 * 86_400,
            ..a
        };
        assert_eq!(b.gecis_bitis_sn(), Some(bas + MUTLAK_SON_GUN * 86_400));
        assert!(b.gecis_acik(bas + 179 * 86_400));
        assert!(!b.gecis_acik(bas + MUTLAK_SON_GUN * 86_400));
    }
}
