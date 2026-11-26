// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! Gün örnekleri test iskeleti: `testdata/gun-ornekleri/*.json` dosyalarının her biri için genel toplam ve Z
//! beklenen formülü doğrulanır. Bugün yalnız SENTETİK örnekler var (iade, kasa açığı, e-fatura/kredi çeki
//! henüz gerçek günle doğrulanmadı). Gerçek gün gelince tutarlar anonimleştirilip (ölçeklenip) aynı biçimde
//! dosya eklenir, `"sentetik": false` yazılır; test kendiliğinden kapsar. Depoya gerçek tutar yazılmaz.

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
    beklenen_genel_toplam_kurus: i64,
    beklenen_z_toplam_kurus: i64,
}

fn klasor() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata/gun-ornekleri")
}

#[test]
fn gun_ornekleri_genel_toplam_ve_z_tutar() {
    let mut yollar: Vec<PathBuf> = std::fs::read_dir(klasor())
        .unwrap()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .collect();
    yollar.sort();
    assert!(!yollar.is_empty(), "en az bir gün örneği olmalı");
    let mut gercek = 0;
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
        assert_eq!(
            z_beklenen_ayarli(gt, &o.ciro, &f),
            o.beklenen_z_toplam_kurus,
            "{}: Z beklenen",
            y.display()
        );
        if !o.sentetik {
            gercek += 1;
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
        ("efatura_pos".to_string(), 30),
    ]);
    assert_eq!(z_beklenen(10_000, &ciro), 10_000 + 500 - 200 + 100);
    let hepsi_kapali = ZFormulu {
        degisim_eklenir: false,
        internet_degisimi_dusulur: false,
        iade_eklenir: false,
        kredi_ceki_dusulur: false,
        efatura_dusulur: false,
    };
    assert_eq!(z_beklenen_ayarli(10_000, &ciro, &hepsi_kapali), 10_000);
    // Eski ayar dosyası (alan yok) varsayılanı alır.
    let a: KasaAyar = serde_json::from_str(r#"{"sube":"SB1"}"#).unwrap();
    assert_eq!(a.z_formulu, ZFormulu::default());
}
