// Sahip: Yusuf. yazan: [Claude#taa-cihaz-S2 2026-11-06]
//! taa-cihaz testleri: üretim/yükleme, imza+doğrulama, zaman payı, başlıklar, kayıt/durum, üzerine yazmama.

use super::*;
use std::sync::Mutex;

struct Sahte {
    yanit: Value,
    son: Mutex<Option<(String, Value)>>,
}

impl Gonderici for Sahte {
    fn gonder(&self, uc: &str, govde: &Value) -> Result<Value, GonderHata> {
        *self.son.lock().unwrap() = Some((uc.into(), govde.clone()));
        Ok(self.yanit.clone())
    }
}

#[test]
fn uret_yukle_ve_uzerine_yazma() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    assert!(!CihazAnahtari::var_mi(&kok));
    assert!(CihazAnahtari::yukle(&kok).is_err());
    let c = CihazAnahtari::uret(&kok, Some("Pendik-PC")).unwrap();
    assert_eq!(c.ad(), "Pendik-PC");
    assert_eq!(c.kimlik().len(), 16);
    assert!(CihazAnahtari::var_mi(&kok));
    assert!(kok.join("cihaz/cihaz.pub").is_file() && kok.join("cihaz/cihaz.json").is_file());
    let y = CihazAnahtari::yukle(&kok).unwrap();
    assert_eq!(y.kimlik(), c.kimlik());
    assert_eq!(y.ad(), "Pendik-PC");
    assert!(
        CihazAnahtari::uret(&kok, None).is_err(),
        "üzerine yazılmamalı"
    );
    assert!(CihazAnahtari::uret(&taa_dosya::gecici_klasor().unwrap(), Some("a\nb")).is_err());
    let d = format!("{c:?}");
    assert!(d.contains("kimlik") && !d.contains("ozel"));
    assert!(c.ozet().get("ozel").is_none());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let kip = std::fs::metadata(kok.join("cihaz/cihaz.key"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(kip & 0o777, 0o600);
    }
}

#[test]
fn imza_ve_dogrulama() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    let c = CihazAnahtari::uret(&kok, Some("x")).unwrap();
    let z = 1_762_470_000;
    let g = br#"{"no":"1"}"#;
    let i = c.imzala("post", "/v1/tutanak?no=1", z, g);
    assert_eq!(i.len(), 128);
    assert!(dogrula(c.genel(), "POST", "/v1/tutanak?no=1", z, g, &i, z + 10).is_ok());
    assert!(
        dogrula(c.genel(), "POST", "/v1/tutanak", z, g, &i, z).is_err(),
        "yol değişti"
    );
    assert!(
        dogrula(c.genel(), "POST", "/v1/tutanak?no=1", z, b"{}", &i, z).is_err(),
        "gövde değişti"
    );
    assert!(
        dogrula(c.genel(), "POST", "/v1/tutanak?no=1", z + 1, g, &i, z).is_err(),
        "zaman değişti"
    );
    assert!(
        dogrula(c.genel(), "POST", "/v1/tutanak?no=1", z, g, &i, z + 301).is_err(),
        "zaman payı"
    );
    assert_eq!(
        imza_mesaji("post", "/a", 5, b"xy"),
        b"POST\n/a\n5\nxy".to_vec()
    );
}

#[test]
fn basliklar_ve_gonderici() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    let c = Arc::new(CihazAnahtari::uret(&kok, None).unwrap());
    let b = c.basliklar("POST", "/v1/olay", b"{}").unwrap();
    assert_eq!(b.len(), 3);
    assert_eq!(b[0].0, BASLIK_CIHAZ);
    assert_eq!(b[0].1, c.kimlik());
    let z: i64 = b[1].1.parse().unwrap();
    assert!((simdi_sn() - z).abs() < 5);
    assert!(dogrula(c.genel(), "POST", "/v1/olay", z, b"{}", &b[2].1, z).is_ok());
    let g = Arc::clone(&c)
        .gonderici("https://sunucu:8443/", None)
        .unwrap();
    assert!(g.imzali_mi());
    assert_eq!(g.taban(), "https://sunucu:8443");
    assert!(Arc::clone(&c).gonderici("ftp://x", None).is_err());
}

#[test]
fn kayit_ve_durum() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    let c = CihazAnahtari::uret(&kok, Some("Ümraniye PC")).unwrap();
    let s = Sahte {
        yanit: json!({"kimlik": c.kimlik(), "durum": "bekliyor"}),
        son: Mutex::new(None),
    };
    let y = kayit(&s, &c, Some("UMR")).unwrap();
    assert_eq!(y["durum"], "bekliyor");
    let (uc, govde) = s.son.lock().unwrap().clone().unwrap();
    assert_eq!(uc, KAYIT_UCU);
    assert_eq!(govde["ad"], "Ümraniye PC");
    assert_eq!(govde["sube_kodu"], "UMR");
    assert_eq!(govde["acik_anahtar"], c.genel().metin());
    assert!(govde.get("ozel").is_none());
    assert!(kayit(&s, &c, Some("a\nb")).is_err());
    let d = durum(&s, &c).unwrap();
    assert_eq!(d["kimlik"], c.kimlik());
    assert_eq!(s.son.lock().unwrap().clone().unwrap().0, DURUM_UCU);
    let yanlis = Sahte {
        yanit: json!({"kimlik": "0000000000000000", "durum": "onayli"}),
        son: Mutex::new(None),
    };
    assert!(durum(&yanlis, &c).is_err(), "başka kimlik reddedilmeli");
    assert_eq!(hata_kodu("sunucu reddetti (403): x"), 4);
    assert_eq!(hata_kodu("sunucuya ulaşılamadı: y"), 3);
}
