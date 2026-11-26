// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! Gün örnekleri test iskeleti: `testdata/gun-ornekleri/*.json` dosyalarının her biri için genel toplam ve Z
//! beklenen formülü doğrulanır. Bugün yalnız SENTETİK örnekler var (iade, kasa açığı, e-fatura/kredi çeki
//! henüz gerçek günle doğrulanmadı). Gerçek gün gelince tutarlar anonimleştirilip (ölçeklenip) aynı biçimde
//! dosya eklenir, `"sentetik": false` yazılır; test kendiliğinden kapsar. Depoya gerçek tutar yazılmaz.
//! Gerçek tutarlı doğrulama depo dışında yapılır: `TAA_GUN_ORNEKLERI_EK=<klasör>` verilirse oradaki dosyalar da
//! aynı denetimden geçer (o klasör depoya girmez).

use super::*;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Ornek {
    #[serde(default)]
    sentetik: bool,
    #[serde(default)]
    z_formulu: Option<ZFormulu>,
    ciro: BTreeMap<String, i64>,
    #[serde(default)]
    doviz: Vec<DovizSatiri>,
    #[serde(default)]
    masraf_kurus: i64,
    /// Masraf kalemleri (ad, kuruş); verilirse toplamı `masraf_kurus`'a eşit olmalı.
    #[serde(default)]
    masraf_kalemleri: Vec<(String, i64)>,
    beklenen_genel_toplam_kurus: i64,
    /// Z verisi olmayan günlerde yok.
    #[serde(default)]
    beklenen_z_toplam_kurus: Option<i64>,
    /// Aynı günün çekmeceleri/mağazaları: grup adı ve grubun genel toplamı (mağaza genel toplamı).
    #[serde(default)]
    magaza: Option<String>,
    #[serde(default)]
    beklenen_magaza_toplam_kurus: Option<i64>,
}

fn klasor() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/gun-ornekleri")
}

#[test]
fn gun_ornekleri_genel_toplam_ve_z_tutar() {
    let oku = |k: PathBuf| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(k)
            .map(|d| {
                d.filter_map(Result::ok)
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|x| x == "json"))
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    };
    let mut yollar = oku(klasor());
    assert!(!yollar.is_empty(), "en az bir gün örneği olmalı");
    if let Ok(ek) = std::env::var("TAA_GUN_ORNEKLERI_EK") {
        yollar.extend(oku(PathBuf::from(ek)));
    }
    let mut gercek = 0;
    let mut gruplar: BTreeMap<String, (i64, Option<i64>)> = BTreeMap::new();
    for y in &yollar {
        let o: Ornek = serde_json::from_slice(&std::fs::read(y).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", y.display()));
        for k in o.ciro.keys() {
            assert!(
                ciro_anahtari(k).is_some(),
                "{}: bilinmeyen ciro anahtarı {k}",
                y.display()
            );
        }
        let f = o.z_formulu.unwrap_or_default();
        let gt = ciro_toplami(&o.ciro, &o.doviz, o.masraf_kurus);
        assert_eq!(
            gt,
            o.beklenen_genel_toplam_kurus,
            "{}: genel toplam",
            y.display()
        );
        if let Some(z) = o.beklenen_z_toplam_kurus {
            assert_eq!(
                z_beklenen_ayarli(gt, &o.ciro, &f),
                z,
                "{}: Z beklenen",
                y.display()
            );
        }
        if !o.masraf_kalemleri.is_empty() {
            let t: i64 = o.masraf_kalemleri.iter().map(|x| x.1).sum();
            assert_eq!(t, o.masraf_kurus, "{}: masraf kalemleri", y.display());
        }
        if let Some(m) = &o.magaza {
            let g = gruplar.entry(m.clone()).or_insert((0, None));
            g.0 += gt;
            if let Some(b) = o.beklenen_magaza_toplam_kurus {
                assert!(
                    g.1.is_none_or(|x| x == b),
                    "{}: grup toplamı çelişkili",
                    y.display()
                );
                g.1 = Some(b);
            }
        }
        if !o.sentetik {
            gercek += 1;
        }
    }
    for (m, (t, b)) in &gruplar {
        if let Some(b) = b {
            assert_eq!(t, b, "{m}: mağaza genel toplamı");
        }
    }
    // Bilgi: gerçek (anonim) gün örneği sayısı. Sıfırken iade/kredi çeki/e-fatura varsayımları kanıtsızdır.
    eprintln!("gün örnekleri: {} dosya, {} gerçek", yollar.len(), gercek);
}

#[test]
fn z_formulu_varsayilani_eski_formulle_ayni() {
    let ciro = BTreeMap::from([
        ("degisim".to_string(), 500),
        ("internet_degisimi".to_string(), 200),
        ("iade".to_string(), 100),
        ("kredi_ceki".to_string(), 50),
    ]);
    assert_eq!(z_beklenen(10_000, &ciro), 10_000 + 500 - 200 + 100);
    let hepsi_kapali = ZFormulu {
        degisim_eklenir: false,
        internet_degisimi_dusulur: false,
        iade_eklenir: false,
        kredi_ceki_dusulur: false,
    };
    assert_eq!(z_beklenen_ayarli(10_000, &ciro, &hepsi_kapali), 10_000);
    // Eski ayar dosyası (alan yok) varsayılanı alır.
    let a: KasaAyar = serde_json::from_str(r#"{"sube":"SB1"}"#).unwrap();
    assert_eq!(a.z_formulu, ZFormulu::default());
}

#[test]
fn eksi_kredi_ceki_toplamdan_duser_efatura_eski_kayitta_okunur() {
    let ciro = BTreeMap::from([
        ("nakit".to_string(), 10_000),
        ("kredi_ceki".to_string(), -1_000),
        ("kasa_fazlasi".to_string(), 7),
    ]);
    // Kasa fazlası bilgi satırıdır; eksi kredi çeki düşer.
    assert_eq!(ciro_toplami(&ciro, &[], 500), 10_000 - 1_000 + 500);
    assert_eq!(
        ciro_anahtari("kredi_ceki"),
        Some(true),
        "kredi çeki eksi olabilir"
    );
    // E-FATURA POS kaldırıldı: yeni anahtar değil, ama eski kayıttaki tutar toplama girer.
    assert_eq!(ciro_anahtari(ESKI_EFATURA), None);
    assert!(!ODEME_TURLERI.iter().any(|(k, _)| *k == ESKI_EFATURA));
    let eski = BTreeMap::from([("nakit".to_string(), 100), (ESKI_EFATURA.to_string(), 30)]);
    assert_eq!(ciro_toplami(&eski, &[], 0), 130);
    // Eski ayar dosyasındaki `efatura_dusulur` alanı sorun çıkarmaz.
    let a: KasaAyar =
        serde_json::from_str(r#"{"sube":"SB1","z_formulu":{"efatura_dusulur":true}}"#).unwrap();
    assert_eq!(a.z_formulu, ZFormulu::default());
}
