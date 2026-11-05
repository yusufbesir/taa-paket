// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! T3 imza testleri: geçerli, bozuk, imzasız+anahtar var, imzasız+anahtar yok, başka anahtar, bozuk anahtar dosyası.

use crate::testler::paket_yap;
use crate::*;
use std::fs;
use std::path::{Path, PathBuf};
use taa_imza::{anahtar_uret, paket_imzala, OzelAnahtar};

struct Ort {
    taban: PathBuf,
}

impl Ort {
    fn yeni() -> Ort {
        Ort {
            taban: taa_dosya::gecici_klasor().unwrap(),
        }
    }
    fn kok(&self) -> PathBuf {
        self.taban.join("kok")
    }
    /// Yeni anahtar çifti üretir; genel anahtarı `kok/anahtarlar/<ad>.pub` altına koyar (`guvenilir`), özel anahtarı döner.
    fn anahtar(&self, ad: &str, guvenilir: bool) -> OzelAnahtar {
        let ozel = self.taban.join(format!("{ad}.key"));
        let genel = if guvenilir {
            self.kok().join("anahtarlar").join(format!("{ad}.pub"))
        } else {
            self.taban.join(format!("{ad}.pub"))
        };
        anahtar_uret(&ozel, &genel).unwrap();
        OzelAnahtar::dosyadan(&ozel).unwrap().0
    }
    fn paket(&self, ad: &str, imza: Option<&OzelAnahtar>) -> PathBuf {
        let k = paket_yap(
            &self.taban,
            ad,
            "1.0.0",
            "1",
            "ortak",
            &[],
            &[("a.txt", "x")],
        );
        if let Some(o) = imza {
            imzala(&k, o);
        }
        k
    }
}

fn imzala(klasor: &Path, o: &OzelAnahtar) {
    let y = klasor.join("paket.json");
    let mut v: serde_json::Value = serde_json::from_slice(&fs::read(&y).unwrap()).unwrap();
    paket_imzala(&mut v, o).unwrap();
    fs::write(&y, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
}

fn json_degistir(klasor: &Path, f: impl FnOnce(&mut serde_json::Value)) {
    let y = klasor.join("paket.json");
    let mut v: serde_json::Value = serde_json::from_slice(&fs::read(&y).unwrap()).unwrap();
    f(&mut v);
    fs::write(&y, serde_json::to_vec_pretty(&v).unwrap()).unwrap();
}

fn reddedildi(r: Result<KurSonuc, KurHata>, parca: &str) {
    match r {
        Err(KurHata::Dogrulama(m)) => {
            assert!(m.contains(parca), "{m}");
            assert!(!m.contains('\n'), "tek cümle olmalı: {m}");
        }
        d => panic!("Dogrulama bekleniyordu: {d:?}"),
    }
}

#[test]
fn gecerli_imza_kabul_edilir() {
    let o = Ort::yeni();
    let oz = o.anahtar("yayinci", true);
    let kur = Kur::yeni(o.kok()).unwrap();
    let s = kur.kur(&o.paket("a", Some(&oz)), true).unwrap();
    assert_eq!((s.ad.as_str(), s.secili), ("a", true));
    let t = kur.tani_veri_ile(None).unwrap();
    assert_eq!(t.guvenilir_anahtarlar, vec![oz.genel().kimlik()]);
    assert!(t.uyarilar.is_empty());
}

#[test]
fn bozuk_imza_reddedilir() {
    let o = Ort::yeni();
    let oz = o.anahtar("yayinci", true);
    let kur = Kur::yeni(o.kok()).unwrap();
    // 1) imza değeri bozuldu
    let k = o.paket("a", Some(&oz));
    json_degistir(&k, |v| {
        let d = v["imza"]["deger"].as_str().unwrap().to_string();
        let c = if d.starts_with('0') { "1" } else { "0" };
        v["imza"]["deger"] = format!("{c}{}", &d[1..]).into();
    });
    reddedildi(kur.kur(&k, true), "imzası geçersiz");
    // 2) imza başka bir paketin içeriğinden alındı (içerik değişti, imza eski)
    let k2 = o.paket("b", Some(&oz));
    let k3 = paket_yap(&o.taban, "b", "2.0.0", "1", "ortak", &[], &[("a.txt", "y")]);
    let imza_b = {
        let v: serde_json::Value =
            serde_json::from_slice(&fs::read(k2.join("paket.json")).unwrap()).unwrap();
        v["imza"].clone()
    };
    json_degistir(&k3, |v| v["imza"] = imza_b);
    reddedildi(kur.kur(&k3, true), "imzası geçersiz");
    // 3) bozuk imza alanı biçimi, bilinmeyen algoritma
    let k4 = o.paket("c", Some(&oz));
    json_degistir(&k4, |v| {
        v["imza"] = serde_json::json!({"algoritma": "ed25519"})
    });
    reddedildi(kur.kur(&k4, true), "okunamadı");
    let k5 = o.paket("d", Some(&oz));
    json_degistir(&k5, |v| v["imza"]["alg"] = "rsa".into());
    reddedildi(kur.kur(&k5, true), "desteklenmiyor");
    // hiçbiri kurulmadı
    assert!(kur.liste().unwrap().is_empty());
}

#[test]
fn imzasiz_paket_anahtar_varken_reddedilir() {
    let o = Ort::yeni();
    o.anahtar("yayinci", true);
    let kur = Kur::yeni(o.kok()).unwrap();
    reddedildi(kur.kur(&o.paket("a", None), true), "imzasız");
    // null imza da imzasızdır
    let k = o.paket("b", None);
    json_degistir(&k, |v| v["imza"] = serde_json::Value::Null);
    reddedildi(kur.kur(&k, true), "imzasız");
    assert!(kur.liste().unwrap().is_empty());
}

#[test]
fn imzasiz_paket_anahtar_yokken_kabul_ve_tani_uyarir() {
    let o = Ort::yeni();
    let kur = Kur::yeni(o.kok()).unwrap();
    kur.kur(&o.paket("a", None), true).unwrap();
    // imza alanı çöp olsa da anahtar yokken içeriği yorumlanmaz
    let k = o.paket("b", None);
    json_degistir(&k, |v| v["imza"] = serde_json::json!("çöp"));
    kur.kur(&k, true).unwrap();
    let t = kur.tani_veri_ile(None).unwrap();
    assert!(t.guvenilir_anahtarlar.is_empty());
    assert!(
        t.uyarilar
            .iter()
            .any(|u| u.contains("imzaları denetlenmiyor")),
        "{:?}",
        t.uyarilar
    );
}

#[test]
fn baska_anahtarla_imzali_paket_reddedilir() {
    let o = Ort::yeni();
    let _guvenilir = o.anahtar("yayinci", true);
    let yabanci = o.anahtar("yabanci", false); // güvenilir listede değil
    let kur = Kur::yeni(o.kok()).unwrap();
    let k = o.paket("a", Some(&yabanci));
    reddedildi(kur.kur(&k, true), "güvenilir anahtarlar arasında olmayan");
    // yabancı, güvenilir anahtarın kimliğini taklit etse de imza tutmaz
    let k2 = o.paket("b", Some(&yabanci));
    let taklit = _guvenilir.genel().kimlik();
    json_degistir(&k2, |v| v["imza"]["anahtar_kimlik"] = taklit.into());
    reddedildi(kur.kur(&k2, true), "imzası geçersiz");
    assert!(kur.liste().unwrap().is_empty());
}

#[test]
fn birden_cok_guvenilir_anahtardan_biri_yeter() {
    let o = Ort::yeni();
    let _a = o.anahtar("a", true);
    let b = o.anahtar("b", true);
    let kur = Kur::yeni(o.kok()).unwrap();
    kur.kur(&o.paket("x", Some(&b)), true).unwrap();
    assert_eq!(
        kur.tani_veri_ile(None).unwrap().guvenilir_anahtarlar.len(),
        2
    );
}

#[test]
fn bozuk_anahtar_dosyasi_imza_denetimini_kapatmaz() {
    let o = Ort::yeni();
    let dizin = o.kok().join("anahtarlar");
    fs::create_dir_all(&dizin).unwrap();
    fs::write(dizin.join("bozuk.pub"), "ed25519:zz").unwrap();
    let kur = Kur::yeni(o.kok()).unwrap();
    match kur.kur(&o.paket("a", None), true) {
        Err(KurHata::Bozuk(m)) => assert!(m.contains("bozuk.pub"), "{m}"),
        d => panic!("Bozuk bekleniyordu: {d:?}"),
    }
    let t = kur.tani_veri_ile(None).unwrap();
    assert!(t
        .sorunlar
        .iter()
        .any(|s| matches!(s, Sorun::Bozuk(m) if m.contains("bozuk.pub"))));
}

#[test]
fn pub_olmayan_dosyalar_yok_sayilir() {
    let o = Ort::yeni();
    let dizin = o.kok().join("anahtarlar");
    fs::create_dir_all(&dizin).unwrap();
    fs::write(dizin.join("notlar.txt"), "anahtar değil").unwrap();
    let kur = Kur::yeni(o.kok()).unwrap();
    kur.kur(&o.paket("a", None), true).unwrap();
}
