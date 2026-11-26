//! Pay basamaklarında ölçüm iskeleti. Sahip: Yusuf. yazan: [Claude#olcum 2026-11-26]
//! Her basamak ayrı ölçülür (başka paya çarpılmaz); ısınma atılır, 3 geçerli tekrarın ortancası alınır;
//! doğrulanmayan tekrar sayılmaz. Ön plan etkisi temsilî sentetik işlemle ölçülür (gerçek kasa işlemi DG1b).

use crate::envanter::{self, Envanter};
use crate::is;
use crate::limit::{self, Duzenek, UygulananLimitler};
use crate::yontem::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

pub const SEMA: &str = "taa-olcum/1";
pub const HIZ_BIRIMI: &str = "sentetik-birim/saat";
pub const TASIMA_YONTEMI: &str = "bellek-ici-iskelet";
/// Temsilî işlem örnekleme aralığı.
const SORGU_ARALIK: Duration = Duration::from_millis(20);

#[derive(Debug, Clone)]
pub struct Ayar {
    pub sure_ms: u64,
    pub isitma: u32,
    pub tekrar: usize,
    pub paylar: Vec<u32>,
    /// None: bu bilgisayarın mantıksal çekirdek sayısı.
    pub cekirdek: Option<usize>,
}

impl Default for Ayar {
    fn default() -> Self {
        Ayar {
            sure_ms: VARSAYILAN_SURE_MS,
            isitma: ISINMA_TEKRAR,
            tekrar: OLCUM_TEKRAR,
            paylar: PAY_BASAMAKLARI.to_vec(),
            cekirdek: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tekrar {
    pub gecerli: bool,
    pub birim: u64,
    pub gecersiz_birim: u64,
    pub sure_ms: f64,
    pub hiz_birim_saat: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OnPlan {
    pub ornek_sayisi: usize,
    pub taban_ortanca_ms: Option<f64>,
    pub yuk_ortanca_ms: Option<f64>,
    pub artis_yuzde: Option<f64>,
    /// En kötü işlemin tabana göre ek gecikmesi (ms).
    pub en_kotu_ek_ms: Option<f64>,
    pub ortanca_gecti: Option<bool>,
    pub tek_islem_gecti: Option<bool>,
    /// İki ölçüt birlikte; veri yoksa null.
    pub gecti: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PayOlcum {
    pub pay_yuzde: u32,
    pub duzenek: Duzenek,
    pub tekrarlar: Vec<Tekrar>,
    pub gecerli_tekrar: usize,
    pub yeterli_tekrar: bool,
    pub hiz_birim_saat: Option<f64>,
    pub hiz_birimi: String,
    pub tasima_ms: f64,
    pub tasima_yontemi: String,
    pub on_plan: OnPlan,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OlcumRapor {
    pub sema: String,
    pub arac_surumu: String,
    pub zaman_unix: u64,
    pub yontem_sha256: String,
    pub yontem_kilit_tutuyor: bool,
    pub is_turu: String,
    pub sure_ms: u64,
    pub envanter: Envanter,
    pub limitler: UygulananLimitler,
    pub paylar: Vec<PayOlcum>,
    pub on_plani_etkilemeyen_en_yuksek_pay: Option<u32>,
    pub uyarilar: Vec<String>,
}

pub fn ortanca(mut v: Vec<f64>) -> Option<f64> {
    if v.is_empty() {
        return None;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = v.len();
    Some(if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    })
}

/// Ön plan ölçütlerini değerlendirir (eşikler yontem.rs'den, kilitli).
pub fn on_plan_degerlendir(taban_ms: &[f64], yuk_ms: &[f64]) -> OnPlan {
    let tb = ortanca(taban_ms.to_vec());
    let yk = ortanca(yuk_ms.to_vec());
    let (Some(tb), Some(yk)) = (tb, yk) else {
        return OnPlan {
            ornek_sayisi: yuk_ms.len(),
            taban_ortanca_ms: tb,
            yuk_ortanca_ms: yk,
            artis_yuzde: None,
            en_kotu_ek_ms: None,
            ortanca_gecti: None,
            tek_islem_gecti: None,
            gecti: None,
        };
    };
    let artis = (yk - tb) / tb.max(1e-9) * 100.0;
    let en_kotu = yuk_ms.iter().cloned().fold(f64::MIN, f64::max) - tb;
    let o = artis <= ORTANCA_ARTIS_EN_COK_YUZDE;
    let t = en_kotu <= TEK_ISLEM_EK_GECIKME_MS;
    OnPlan {
        ornek_sayisi: yuk_ms.len(),
        taban_ortanca_ms: Some(yuvarla3(tb)),
        yuk_ortanca_ms: Some(yuvarla3(yk)),
        artis_yuzde: Some(yuvarla3(artis)),
        en_kotu_ek_ms: Some(yuvarla3(en_kotu)),
        ortanca_gecti: Some(o),
        tek_islem_gecti: Some(t),
        gecti: Some(o && t),
    }
}

/// 5'ten başlayarak ardışık geçen en yüksek basamak.
pub fn en_yuksek_gecen(paylar: &[PayOlcum]) -> Option<u32> {
    let mut son = None;
    for p in paylar {
        if p.on_plan.gecti == Some(true) {
            son = Some(p.pay_yuzde);
        } else {
            break;
        }
    }
    son
}

fn yuvarla3(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

fn ms(d: Duration) -> f64 {
    d.as_secs_f64() * 1000.0
}

/// Temsilî işlemi `dur` bayrağı kalkana dek aralıklarla çalıştırır, gecikmeleri (ms) döndürür.
fn sorgu_dongusu(dur: &AtomicBool) -> Vec<f64> {
    let mut v = Vec::new();
    while !dur.load(Ordering::Relaxed) {
        let t = Instant::now();
        std::hint::black_box(is::sorgu_calistir());
        v.push(ms(t.elapsed()));
        std::thread::sleep(SORGU_ARALIK);
    }
    v
}

/// İşçi kapalıyken temsilî işlem gecikmeleri.
pub fn taban_olc(sure: Duration) -> Vec<f64> {
    let dur = AtomicBool::new(false);
    std::thread::scope(|s| {
        let h = s.spawn(|| sorgu_dongusu(&dur));
        std::thread::sleep(sure);
        dur.store(true, Ordering::Relaxed);
        h.join().unwrap_or_default()
    })
}

/// Bir tekrar: düzenekle yük çalışır, biten her birim referansla karşılaştırılır; aynı sırada temsilî işlem ölçülür.
pub fn yuk_tekrari(
    d: &Duzenek,
    sure: Duration,
    referans: [u8; 32],
    birim: fn() -> [u8; 32],
) -> (Tekrar, Vec<f64>) {
    let dogru = AtomicU64::new(0);
    let yanlis = AtomicU64::new(0);
    let dur = AtomicBool::new(false);
    let baslangic = Instant::now();
    let bitis = baslangic + sure;
    let orani = d.gorev_orani;
    let (gecen, sorgu) = std::thread::scope(|s| {
        let sorgu_h = s.spawn(|| sorgu_dongusu(&dur));
        let isciler: Vec<_> = (0..d.parcacik)
            .map(|_| {
                s.spawn(|| {
                    while Instant::now() < bitis {
                        let t = Instant::now();
                        let c = std::hint::black_box(birim());
                        if c == referans {
                            dogru.fetch_add(1, Ordering::Relaxed);
                        } else {
                            yanlis.fetch_add(1, Ordering::Relaxed);
                        }
                        if orani < 1.0 {
                            let calisti = t.elapsed();
                            let uyku = calisti.mul_f64((1.0 - orani) / orani);
                            let kalan = bitis.saturating_duration_since(Instant::now());
                            std::thread::sleep(uyku.min(kalan));
                        }
                    }
                })
            })
            .collect();
        for i in isciler {
            let _ = i.join();
        }
        let gecen = baslangic.elapsed();
        dur.store(true, Ordering::Relaxed);
        (gecen, sorgu_h.join().unwrap_or_default())
    });
    let birim_say = dogru.load(Ordering::Relaxed);
    let gecersiz = yanlis.load(Ordering::Relaxed);
    let sn = gecen.as_secs_f64().max(1e-9);
    let t = Tekrar {
        gecerli: gecersiz == 0 && birim_say > 0,
        birim: birim_say,
        gecersiz_birim: gecersiz,
        sure_ms: yuvarla3(ms(gecen)),
        hiz_birim_saat: (birim_say as f64 / sn * 3600.0).round(),
    };
    (t, sorgu)
}

/// Taşıma süresi iskeleti: bellek içinde serileştir, geri oku, özet doğrula (gerçek ağ değildir). Ortanca ms.
pub fn tasima_olc() -> f64 {
    let yuk: Vec<u8> = (0..256 * 1024u32)
        .map(|i| (i.wrapping_mul(31) >> 3) as u8)
        .collect();
    let beklenen = crate::yontem::ozet_hex(&yuk);
    let mut ornek = Vec::new();
    for _ in 0..5 {
        let t = Instant::now();
        let hex: String = yuk.iter().map(|b| format!("{b:02x}")).collect();
        let paket = serde_json::to_vec(&serde_json::json!({ "yuk": hex })).unwrap_or_default();
        let geri: serde_json::Value = serde_json::from_slice(&paket).unwrap_or_default();
        let s = geri["yuk"].as_str().unwrap_or("");
        let cozulen: Vec<u8> = (0..s.len() / 2)
            .filter_map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok())
            .collect();
        let tamam = crate::yontem::ozet_hex(&cozulen) == beklenen;
        let gecen = ms(t.elapsed());
        if tamam {
            ornek.push(gecen);
        }
    }
    yuvarla3(ortanca(ornek).unwrap_or(f64::NAN))
}

/// Tek pay basamağı: ısınma, ardından `tekrar` geçerli tekrar (en çok EK_DENEME ek deneme).
pub fn pay_olc(
    ayar: &Ayar,
    cekirdek: usize,
    pay: u32,
    referans: [u8; 32],
    birim: fn() -> [u8; 32],
) -> PayOlcum {
    let d = limit::duzenek_hesapla(cekirdek, pay);
    let sure = Duration::from_millis(ayar.sure_ms);
    let taban = taban_olc(sure);
    for _ in 0..ayar.isitma {
        let _ = yuk_tekrari(&d, sure, referans, birim);
    }
    let mut tekrarlar = Vec::new();
    let mut yuk_ms = Vec::new();
    let mut gecerli = 0usize;
    let mut deneme = 0u32;
    while gecerli < ayar.tekrar && deneme < ayar.tekrar as u32 + EK_DENEME {
        let (t, sorgu) = yuk_tekrari(&d, sure, referans, birim);
        deneme += 1;
        if t.gecerli {
            gecerli += 1;
            yuk_ms.extend(sorgu);
        }
        tekrarlar.push(t);
    }
    let yeterli = gecerli >= ayar.tekrar;
    let hiz = if yeterli {
        ortanca(
            tekrarlar
                .iter()
                .filter(|t| t.gecerli)
                .map(|t| t.hiz_birim_saat)
                .collect(),
        )
    } else {
        None
    };
    PayOlcum {
        pay_yuzde: pay,
        duzenek: d,
        tekrarlar,
        gecerli_tekrar: gecerli,
        yeterli_tekrar: yeterli,
        hiz_birim_saat: hiz,
        hiz_birimi: HIZ_BIRIMI.to_string(),
        tasima_ms: tasima_olc(),
        tasima_yontemi: TASIMA_YONTEMI.to_string(),
        on_plan: if yeterli {
            on_plan_degerlendir(&taban, &yuk_ms)
        } else {
            on_plan_degerlendir(&taban, &[])
        },
    }
}

/// Tam ölçüm: envanter + her pay basamağı ayrı ayrı.
pub fn olc(ayar: &Ayar, ilerleme: &mut dyn FnMut(&str)) -> OlcumRapor {
    let env = envanter::topla();
    let cekirdek = ayar.cekirdek.unwrap_or(env.islemci.mantiksal);
    let referans = is::birim_referans();
    let mut paylar = Vec::new();
    for &p in &ayar.paylar {
        ilerleme(&format!("pay %{p} ölçülüyor"));
        paylar.push(pay_olc(ayar, cekirdek, p, referans, is::birim_calistir));
    }
    let mut uyarilar = Vec::new();
    if !kilit_tutuyor() {
        uyarilar.push(
            "yöntem belgesi kilitle eşleşmiyor; bu rapor Kapı 1 kararında kullanılamaz".to_string(),
        );
    }
    for p in &paylar {
        if !p.yeterli_tekrar {
            uyarilar.push(format!(
                "pay %{}: yeterli geçerli tekrar yok, hız yazılmadı",
                p.pay_yuzde
            ));
        }
    }
    let en_yuksek = en_yuksek_gecen(&paylar);
    let zaman = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    OlcumRapor {
        sema: SEMA.to_string(),
        arac_surumu: env!("CARGO_PKG_VERSION").to_string(),
        zaman_unix: zaman,
        yontem_sha256: yontem_ozeti(),
        yontem_kilit_tutuyor: kilit_tutuyor(),
        is_turu: "sentetik-cpu".to_string(),
        sure_ms: ayar.sure_ms,
        envanter: env,
        limitler: limit::uygulanan_limitler(),
        paylar,
        on_plani_etkilemeyen_en_yuksek_pay: en_yuksek,
        uyarilar,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn yanlis_birim() -> [u8; 32] {
        [7u8; 32]
    }

    fn kisa() -> Ayar {
        Ayar {
            sure_ms: 120,
            isitma: 1,
            tekrar: 3,
            paylar: vec![5, 15],
            cekirdek: Some(4),
        }
    }

    #[test]
    fn ortanca_tek_ve_cift() {
        assert_eq!(ortanca(vec![3.0, 1.0, 2.0]), Some(2.0));
        assert_eq!(ortanca(vec![4.0, 1.0, 2.0, 3.0]), Some(2.5));
        assert_eq!(ortanca(vec![]), None);
    }

    #[test]
    fn on_plan_esikleri() {
        let taban = [2.0, 2.0, 2.0];
        let iyi = on_plan_degerlendir(&taban, &[2.1, 2.1, 2.1, 2.2]);
        assert_eq!(iyi.gecti, Some(true));
        let yavas = on_plan_degerlendir(&taban, &[2.5, 2.6, 2.7]); // ortanca +%30
        assert_eq!(yavas.ortanca_gecti, Some(false));
        assert_eq!(yavas.gecti, Some(false));
        let aykiri = on_plan_degerlendir(&taban, &[2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 500.0]);
        assert_eq!(aykiri.ortanca_gecti, Some(true));
        assert_eq!(aykiri.tek_islem_gecti, Some(false)); // 498 ms ek
        assert_eq!(aykiri.gecti, Some(false));
        // sınır: tam %10 ve tam 200 ms geçer
        assert_eq!(
            on_plan_degerlendir(&[10.0], &[11.0]).ortanca_gecti,
            Some(true)
        );
        assert_eq!(
            on_plan_degerlendir(&[1000.0], &[1200.0]).tek_islem_gecti,
            Some(true)
        );
        assert_eq!(on_plan_degerlendir(&[], &[1.0]).gecti, None);
    }

    fn sahte(pay: u32, gecti: Option<bool>) -> PayOlcum {
        let mut o = on_plan_degerlendir(&[], &[]);
        o.gecti = gecti;
        PayOlcum {
            pay_yuzde: pay,
            duzenek: limit::duzenek_hesapla(4, pay),
            tekrarlar: vec![],
            gecerli_tekrar: 3,
            yeterli_tekrar: true,
            hiz_birim_saat: Some(1.0),
            hiz_birimi: HIZ_BIRIMI.into(),
            tasima_ms: 1.0,
            tasima_yontemi: TASIMA_YONTEMI.into(),
            on_plan: o,
        }
    }

    #[test]
    fn en_yuksek_ardisik_gecen() {
        let hepsi = [5, 8, 10, 12, 15].map(|p| sahte(p, Some(true)));
        assert_eq!(en_yuksek_gecen(&hepsi), Some(15));
        let kesik = vec![
            sahte(5, Some(true)),
            sahte(8, Some(true)),
            sahte(10, Some(false)),
            sahte(12, Some(true)),
        ];
        assert_eq!(en_yuksek_gecen(&kesik), Some(8)); // 12 geçse de sayılmaz
        assert_eq!(en_yuksek_gecen(&[sahte(5, Some(false))]), None);
        assert_eq!(en_yuksek_gecen(&[sahte(5, None)]), None);
    }

    #[test]
    fn tasima_olculur() {
        let t = tasima_olc();
        assert!(t.is_finite() && t > 0.0);
    }

    #[test]
    fn pay_olc_gecerli_uc_tekrar_ve_hiz() {
        let a = kisa();
        let p = pay_olc(&a, 4, 15, is::birim_referans(), is::birim_calistir);
        assert!(p.yeterli_tekrar, "{p:?}");
        assert_eq!(p.gecerli_tekrar, 3);
        assert_eq!(p.tekrarlar.iter().filter(|t| t.gecerli).count(), 3);
        assert!(p.hiz_birim_saat.unwrap() > 0.0);
        assert_eq!(p.hiz_birimi, HIZ_BIRIMI);
        assert!(p.on_plan.ornek_sayisi > 0);
        assert!(p.on_plan.taban_ortanca_ms.is_some());
    }

    #[test]
    fn dogrulanmayan_tekrar_sayilmaz_ve_hiz_yazilmaz() {
        let a = kisa();
        let p = pay_olc(&a, 4, 10, is::birim_referans(), yanlis_birim);
        assert!(!p.yeterli_tekrar);
        assert_eq!(p.gecerli_tekrar, 0);
        assert_eq!(p.hiz_birim_saat, None);
        assert!(p
            .tekrarlar
            .iter()
            .all(|t| !t.gecerli && t.gecersiz_birim > 0));
        assert_eq!(p.tekrarlar.len(), 3 + EK_DENEME as usize);
        assert_eq!(p.on_plan.gecti, None);
    }

    #[test]
    fn paylar_ayri_olculur_ve_tasinmaz() {
        let a = kisa();
        let r = olc(&a, &mut |_| {});
        assert_eq!(r.paylar.len(), 2);
        assert_eq!(r.paylar[0].pay_yuzde, 5);
        assert_eq!(r.paylar[1].pay_yuzde, 15);
        // Her payın kendi tekrarları vardır; düzenekler farklıdır.
        assert_ne!(r.paylar[0].duzenek, r.paylar[1].duzenek);
        assert_eq!(r.is_turu, "sentetik-cpu");
        assert_eq!(r.sema, SEMA);
        assert_eq!(r.yontem_sha256, yontem_ozeti());
        let json = serde_json::to_string(&r).unwrap();
        let geri: OlcumRapor = serde_json::from_str(&json).unwrap();
        assert_eq!(geri.paylar.len(), 2);
    }
}
