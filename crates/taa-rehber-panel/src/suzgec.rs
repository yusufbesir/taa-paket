//! Kişiler ve Görevler ekranının sunucu tarafı: arama, şube/görev/birim süzgeci, hiyerarşi ağacı.
//!
//! Görünürlük (docs/rehber-sozlesmesi.md §5) `taa_rehber_model::model::gorunum` ile uygulanır; bu modül yalnız o
//! görünümün DAHA DA daraltılmasını yapar. İstemcinin gönderdiği şube/görev/birim değerleri hiçbir zaman kapsamı
//! genişletmez: görünür olmayan bir değer `SuzgecHata` döner (varlığı sızmaz), görünürse yalnız sonucu daraltır.
//! Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use taa_rehber_model::model::{
    BirimTur, EtkinPasif, Gorunum, GorunurBirim, GorunurGorev, KisiDurum, Veri,
};

/// Arama için katlama: Türkçe büyük/küçük harf ve aksan farkı yok sayılır (`Şube` = `sube` = `ŞUBE`).
pub fn katla(s: &str) -> String {
    let mut c = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            'İ' | 'I' | 'ı' | 'i' => c.push('i'),
            'Ş' | 'ş' => c.push('s'),
            'Ç' | 'ç' => c.push('c'),
            'Ğ' | 'ğ' => c.push('g'),
            'Ö' | 'ö' => c.push('o'),
            'Ü' | 'ü' => c.push('u'),
            x => c.extend(x.to_lowercase()),
        }
    }
    c
}

pub fn tur_kodu(t: BirimTur) -> &'static str {
    match t {
        BirimTur::Sube => "sube",
        BirimTur::Magaza => "magaza",
        BirimTur::Departman => "departman",
        BirimTur::Merkez => "merkez",
    }
}

pub fn tur_etiketi(t: BirimTur) -> &'static str {
    match t {
        BirimTur::Sube => "Şube",
        BirimTur::Magaza => "Mağaza",
        BirimTur::Departman => "Departman",
        BirimTur::Merkez => "Merkez",
    }
}

#[derive(Debug, Default, Clone)]
pub struct Suzgec {
    pub ara: String,
    /// Şube biriminin kimliği (`suzgecler()` listesinden).
    pub sube: Option<String>,
    /// Rol kodu.
    pub gorev: Option<String>,
    /// Hiyerarşi düğümü: birim kimliği.
    pub birim: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SuzgecHata {
    BilinmeyenSube,
    BilinmeyenGorev,
    BilinmeyenBirim,
}

impl std::fmt::Display for SuzgecHata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            SuzgecHata::BilinmeyenSube => "bilinmeyen şube",
            SuzgecHata::BilinmeyenGorev => "bilinmeyen görev",
            SuzgecHata::BilinmeyenBirim => "bilinmeyen birim",
        })
    }
}

/// Açılır liste seçenekleri: yalnız görünür şubeler ve roller.
pub fn suzgecler(g: &Gorunum) -> Value {
    let mut subeler: Vec<&GorunurBirim> = g
        .birimler
        .iter()
        .filter(|b| b.tur == BirimTur::Sube)
        .collect();
    subeler.sort_by_key(|b| katla(&b.ad));
    let mut roller: Vec<_> = g.roller.iter().collect();
    roller.sort_by_key(|r| katla(&r.ad));
    json!({
        "subeler": subeler.iter().map(|b| json!({"id": b.id, "ad": b.ad})).collect::<Vec<_>>(),
        "roller": roller.iter().map(|r| json!({"kod": r.kod, "ad": r.ad})).collect::<Vec<_>>(),
    })
}

/// Birim ve (hiyerarşi izinliyse) tüm alt birimleri.
fn alt_birimler(veri: &Veri, kok: &str, hiyerarsi: bool) -> BTreeSet<String> {
    let mut kume = BTreeSet::from([kok.to_string()]);
    if !hiyerarsi {
        return kume;
    }
    let mut kuyruk = vec![kok.to_string()];
    while let Some(u) = kuyruk.pop() {
        for b in veri.birimler.values() {
            if b.ust_birim_id.as_deref() == Some(u.as_str()) && kume.insert(b.id.clone()) {
                kuyruk.push(b.id.clone());
            }
        }
    }
    kume
}

/// Süzülmüş kişi listesi. `hiyerarsi`: `rehber.amir.oku` (ya da tam okuma) izni; yoksa birim seçimi yalnız o birimi getirir.
pub fn kisiler(
    veri: &Veri,
    g: &Gorunum,
    s: &Suzgec,
    hiyerarsi: bool,
    bugun: &str,
) -> Result<Value, SuzgecHata> {
    let birimler: BTreeMap<&str, &GorunurBirim> =
        g.birimler.iter().map(|b| (b.id.as_str(), b)).collect();
    let rol_ad: BTreeMap<&str, &str> = g
        .roller
        .iter()
        .map(|r| (r.kod.as_str(), r.ad.as_str()))
        .collect();

    // Süzgeç değerlerini görünür kümeye göre çöz; görünür olmayan = bilinmeyen.
    let sube_kodu: Option<String> = match &s.sube {
        None => None,
        Some(id) => {
            let b = birimler
                .get(id.as_str())
                .filter(|b| b.tur == BirimTur::Sube)
                .ok_or(SuzgecHata::BilinmeyenSube)?;
            veri.birimler.get(&b.id).and_then(|v| v.sube_kodu.clone())
        }
    };
    if let Some(r) = &s.gorev {
        if !rol_ad.contains_key(r.as_str()) {
            return Err(SuzgecHata::BilinmeyenGorev);
        }
    }
    let birim_kume: Option<BTreeSet<String>> = match &s.birim {
        None => None,
        Some(id) => {
            if !birimler.contains_key(id.as_str()) {
                return Err(SuzgecHata::BilinmeyenBirim);
            }
            Some(alt_birimler(veri, id, hiyerarsi))
        }
    };
    let terimler: Vec<String> = katla(&s.ara)
        .split_whitespace()
        .map(str::to_string)
        .collect();
    let suzgec_var = s.sube.is_some() || s.gorev.is_some() || s.birim.is_some();

    let gorev_eslesir = |gg: &GorunurGorev| -> bool {
        if let Some(r) = &s.gorev {
            if gg.rol.as_deref() != Some(r.as_str()) {
                return false;
            }
        }
        let bid = gg.birim_id.as_deref();
        if let Some(k) = &sube_kodu {
            let ok = bid.is_some_and(|b| {
                birimler.contains_key(b)
                    && veri.birimler.get(b).and_then(|v| v.sube_kodu.as_deref()) == Some(k.as_str())
            });
            if !ok {
                return false;
            }
        }
        if let Some(kume) = &birim_kume {
            if !bid.is_some_and(|b| kume.contains(b)) {
                return false;
            }
        }
        true
    };

    let mut cikti: Vec<(String, Value)> = Vec::new();
    for k in &g.kisiler {
        if suzgec_var && !k.gorevler.iter().any(&gorev_eslesir) {
            continue;
        }
        let mut saman = katla(&k.ad);
        let mut gorevler = Vec::new();
        for gg in &k.gorevler {
            let birim = gg.birim_id.as_deref().and_then(|b| birimler.get(b));
            let rol = gg.rol.as_deref();
            let rad = rol.and_then(|r| rol_ad.get(r)).copied();
            let kod = gg
                .birim_id
                .as_deref()
                .filter(|b| birimler.contains_key(b))
                .and_then(|b| veri.birimler.get(b))
                .and_then(|b| b.sube_kodu.as_deref());
            for parca in [gg.gorev.as_deref(), rad, birim.map(|b| b.ad.as_str()), kod]
                .into_iter()
                .flatten()
            {
                saman.push(' ');
                saman.push_str(&katla(parca));
            }
            let gecmis = gg
                .gorevlendirme_id
                .as_deref()
                .and_then(|id| veri.gorevlendirmeler.get(id))
                .is_some_and(|v| {
                    !(v.durum == EtkinPasif::Etkin
                        && v.baslangic.as_str() <= bugun
                        && v.bitis.as_deref().is_none_or(|b| b >= bugun))
                });
            let amir_ad = gg
                .amir_id
                .as_deref()
                .and_then(|a| veri.kisiler.get(a))
                .map(|a| a.ad.as_str());
            let mut j = json!({
                "birim_ad": birim.map(|b| b.ad.as_str()),
                "birim_tur": birim.map(|b| tur_kodu(b.tur)),
                "birim_tur_etiket": birim.map(|b| tur_etiketi(b.tur)),
                "birim_eposta": birim.and_then(|b| b.ortak_eposta.as_deref()),
                "rol": rol,
                "rol_ad": rad,
                "gorev": gg.gorev,
                "baslangic": gg.baslangic,
                "bitis": gg.bitis,
                "gecmis": gecmis,
                "amir_ad": amir_ad,
            });
            if let Some(t) = gg.tur {
                j["tur"] = json!(match t {
                    taa_rehber_model::model::GorevTur::Asil => "asil",
                    taa_rehber_model::model::GorevTur::Gecici => "gecici",
                    taa_rehber_model::model::GorevTur::Vekil => "vekil",
                });
            }
            gorevler.push(j);
        }
        if !terimler.iter().all(|t| saman.contains(t.as_str())) {
            continue;
        }
        let pasif =
            k.id.as_deref()
                .and_then(|id| veri.kisiler.get(id))
                .is_some_and(|v| v.durum == KisiDurum::Pasif);
        cikti.push((
            katla(&k.ad),
            json!({"ad": k.ad, "eposta": k.eposta, "pasif": pasif, "gorevler": gorevler}),
        ));
    }
    cikti.sort_by(|a, b| a.0.cmp(&b.0));
    let liste: Vec<Value> = cikti.into_iter().map(|(_, v)| v).collect();
    Ok(json!({
        "toplam": liste.len(),
        "gorunur_toplam": g.kisiler.len(),
        "kisiler": liste,
    }))
}

/// Hiyerarşi ağacı. Üst birim bağı (`ust_birim_id`) yalnız `hiyerarsi` izinliyse ve üst birim de görünürse kullanılır;
/// aksi hâlde birimler türe göre gruplanır (görünmeyen bir üst birimin varlığı sızmaz).
pub fn agac(veri: &Veri, g: &Gorunum, hiyerarsi: bool) -> Vec<Value> {
    fn sirali(mut v: Vec<&GorunurBirim>) -> Vec<&GorunurBirim> {
        v.sort_by_key(|b| katla(&b.ad));
        v
    }
    if hiyerarsi {
        let gorunur: BTreeSet<&str> = g.birimler.iter().map(|b| b.id.as_str()).collect();
        // çocuk -> üst (yalnız üst birim de görünürse)
        let ust: BTreeMap<&str, &str> = g
            .birimler
            .iter()
            .filter_map(|b| {
                let u = veri.birimler.get(&b.id)?.ust_birim_id.as_deref()?;
                (gorunur.contains(u) && u != b.id).then_some((b.id.as_str(), u))
            })
            .collect();
        fn dal(b: &GorunurBirim, g: &Gorunum, ust: &BTreeMap<&str, &str>, derin: usize) -> Value {
            let cocuklar = sirali(
                g.birimler
                    .iter()
                    .filter(|c| ust.get(c.id.as_str()) == Some(&b.id.as_str()))
                    .collect(),
            );
            json!({
                "id": b.id, "ad": b.ad, "tur": tur_kodu(b.tur), "tur_etiket": tur_etiketi(b.tur), "grup": false,
                "cocuklar": if derin > 32 { Vec::new() } else { cocuklar.iter().map(|c| dal(c, g, ust, derin + 1)).collect::<Vec<_>>() },
            })
        }
        let kokler = sirali(
            g.birimler
                .iter()
                .filter(|b| !ust.contains_key(b.id.as_str()))
                .collect(),
        );
        return kokler.iter().map(|b| dal(b, g, &ust, 0)).collect();
    }
    let mut ciktilar = Vec::new();
    for (tur, baslik) in [
        (BirimTur::Merkez, "Merkez"),
        (BirimTur::Departman, "Departmanlar"),
        (BirimTur::Sube, "Şubeler"),
        (BirimTur::Magaza, "Mağazalar"),
    ] {
        let uyeler = sirali(g.birimler.iter().filter(|b| b.tur == tur).collect());
        if uyeler.is_empty() {
            continue;
        }
        ciktilar.push(json!({
            "id": null, "ad": baslik, "tur": tur_kodu(tur), "tur_etiket": tur_etiketi(tur), "grup": true,
            "cocuklar": uyeler.iter().map(|b| json!({
                "id": b.id, "ad": b.ad, "tur": tur_kodu(b.tur), "tur_etiket": tur_etiketi(b.tur),
                "grup": false, "cocuklar": [],
            })).collect::<Vec<_>>(),
        }));
    }
    ciktilar
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::demo;
    use taa_rehber_model::model::{gorunum, Bakis};

    const BUGUN: &str = "2026-11-26";

    fn bakis<'a>(
        sube: Option<&'a str>,
        tum: bool,
        amir: bool,
        m: &'a BTreeSet<String>,
    ) -> Bakis<'a> {
        Bakis {
            sube_kodu: sube,
            tum_oku: tum,
            amir_oku: amir,
            bugun: BUGUN,
            mudur_rolleri: m,
        }
    }

    fn s(ara: &str) -> Suzgec {
        Suzgec {
            ara: ara.into(),
            ..Default::default()
        }
    }

    fn adlar(v: &Value) -> Vec<String> {
        v["kisiler"]
            .as_array()
            .unwrap()
            .iter()
            .map(|k| k["ad"].as_str().unwrap().to_string())
            .collect()
    }

    #[test]
    fn katlama_turkce() {
        assert_eq!(
            katla("ŞUBE Müdürü IĞDIR İstanbul"),
            "sube muduru igdir istanbul"
        );
        assert_eq!(katla("sube"), katla("Şube"));
    }

    #[test]
    fn sube_pc_yalniz_kendi_subesini_ve_ortak_alanlari_gorur() {
        let (depo, m) = demo::ornek_depo("2026-11-26T09:00:00Z");
        let v = depo.veri();
        let g = gorunum(v, &bakis(Some(demo::SUBE_1), false, false, &m));
        let r = kisiler(v, &g, &s(""), false, BUGUN).unwrap();
        let a = adlar(&r);
        assert!(a.contains(&"ÖRNEK Ayşe Şube1".to_string()), "{a:?}");
        // Başka şubenin sıradan çalışanı hiç gelmez; yalnız mağaza müdürü gelir (yalnız ad/e-posta/unvan).
        assert!(!a.contains(&"ÖRNEK Can Şube2".to_string()), "{a:?}");
        assert!(a.contains(&"ÖRNEK Müdür Şube2".to_string()), "{a:?}");
        let mudur = r["kisiler"]
            .as_array()
            .unwrap()
            .iter()
            .find(|k| k["ad"] == "ÖRNEK Müdür Şube2")
            .unwrap();
        assert!(
            mudur["gorevler"][0]["birim_ad"].is_null(),
            "müdür satırında birim yok: {mudur}"
        );
        assert!(mudur["gorevler"][0]["rol"].is_null());
        // Pasif kişi şube cihazında yok.
        assert!(!a.iter().any(|x| x.contains("Pasif")), "{a:?}");
    }

    #[test]
    fn istemci_sube_degeri_kapsami_genisletmez() {
        let (depo, m) = demo::ornek_depo("2026-11-26T09:00:00Z");
        let v = depo.veri();
        let g = gorunum(v, &bakis(Some(demo::SUBE_1), false, false, &m));
        // Görünür başka şubeyi süzmek yalnız daraltır: S2 sıradan çalışanı yine görünmez.
        let sube2 = v
            .birimler
            .values()
            .find(|b| b.sube_kodu.as_deref() == Some(demo::SUBE_2) && b.tur == BirimTur::Sube)
            .unwrap();
        let r = kisiler(
            v,
            &g,
            &Suzgec {
                sube: Some(sube2.id.clone()),
                ..Default::default()
            },
            false,
            BUGUN,
        )
        .unwrap();
        let a = adlar(&r);
        assert!(!a.contains(&"ÖRNEK Can Şube2".to_string()), "{a:?}");
        // Var olmayan / görünür olmayan kimlikler 'bilinmeyen' döner, ayrım yok.
        for kotu in ["yok-boyle-birim", "../etc", ""] {
            assert_eq!(
                kisiler(
                    v,
                    &g,
                    &Suzgec {
                        sube: Some(kotu.into()),
                        ..Default::default()
                    },
                    false,
                    BUGUN
                )
                .unwrap_err(),
                SuzgecHata::BilinmeyenSube
            );
        }
        // Mağaza kimliği şube süzgecine verilemez.
        let magaza = v
            .birimler
            .values()
            .find(|b| b.tur == BirimTur::Magaza)
            .unwrap();
        assert_eq!(
            kisiler(
                v,
                &g,
                &Suzgec {
                    sube: Some(magaza.id.clone()),
                    ..Default::default()
                },
                false,
                BUGUN
            )
            .unwrap_err(),
            SuzgecHata::BilinmeyenSube
        );
        assert_eq!(
            kisiler(
                v,
                &g,
                &Suzgec {
                    gorev: Some("yok".into()),
                    ..Default::default()
                },
                false,
                BUGUN
            )
            .unwrap_err(),
            SuzgecHata::BilinmeyenGorev
        );
        assert_eq!(
            kisiler(
                v,
                &g,
                &Suzgec {
                    birim: Some("yok".into()),
                    ..Default::default()
                },
                false,
                BUGUN
            )
            .unwrap_err(),
            SuzgecHata::BilinmeyenBirim
        );
    }

    #[test]
    fn kapsam_yok_yalniz_merkez() {
        let (depo, m) = demo::ornek_depo("2026-11-26T09:00:00Z");
        let v = depo.veri();
        let g = gorunum(v, &bakis(None, false, false, &m));
        let a = adlar(&kisiler(v, &g, &s(""), false, BUGUN).unwrap());
        assert_eq!(a, vec!["ÖRNEK Merkez Yetkilisi".to_string()], "{a:?}");
        assert!(g.birimler.iter().all(|b| b.tur == BirimTur::Merkez));
    }

    #[test]
    fn tum_oku_hepsini_ve_pasifi_gorur_sube_ve_gorev_suzgeci() {
        let (depo, m) = demo::ornek_depo("2026-11-26T09:00:00Z");
        let v = depo.veri();
        let g = gorunum(v, &bakis(None, true, true, &m));
        let tum = kisiler(v, &g, &s(""), true, BUGUN).unwrap();
        assert!(adlar(&tum).contains(&"ÖRNEK Pasif Kişi".to_string()));
        let pasif = tum["kisiler"]
            .as_array()
            .unwrap()
            .iter()
            .find(|k| k["ad"] == "ÖRNEK Pasif Kişi")
            .unwrap();
        assert_eq!(pasif["pasif"], true);
        // Şube süzgeci: S2 şubesi ve altındaki mağaza.
        let sube2 = v
            .birimler
            .values()
            .find(|b| b.sube_kodu.as_deref() == Some(demo::SUBE_2) && b.tur == BirimTur::Sube)
            .unwrap();
        let r = kisiler(
            v,
            &g,
            &Suzgec {
                sube: Some(sube2.id.clone()),
                ..Default::default()
            },
            true,
            BUGUN,
        )
        .unwrap();
        let a = adlar(&r);
        assert!(
            a.contains(&"ÖRNEK Can Şube2".to_string())
                && a.contains(&"ÖRNEK Müdür Şube2".to_string()),
            "{a:?}"
        );
        assert!(!a.contains(&"ÖRNEK Ayşe Şube1".to_string()), "{a:?}");
        // Görev süzgeci birlikte uygulanır.
        let r2 = kisiler(
            v,
            &g,
            &Suzgec {
                sube: Some(sube2.id.clone()),
                gorev: Some(demo::ROL_MUDUR.into()),
                ..Default::default()
            },
            true,
            BUGUN,
        )
        .unwrap();
        assert_eq!(adlar(&r2), vec!["ÖRNEK Müdür Şube2".to_string()]);
        // Amir adı yalnız amir_oku varken gelir.
        let ayse = r["kisiler"]
            .as_array()
            .unwrap()
            .iter()
            .find(|k| k["ad"] == "ÖRNEK Can Şube2")
            .unwrap();
        assert!(ayse["gorevler"][0]["amir_ad"].is_string(), "{ayse}");
        let g2 = gorunum(v, &bakis(None, true, false, &m));
        let r3 = kisiler(
            v,
            &g2,
            &Suzgec {
                sube: Some(sube2.id.clone()),
                ..Default::default()
            },
            false,
            BUGUN,
        )
        .unwrap();
        let can = r3["kisiler"]
            .as_array()
            .unwrap()
            .iter()
            .find(|k| k["ad"] == "ÖRNEK Can Şube2")
            .unwrap();
        assert!(
            can["gorevler"][0]["amir_ad"].is_null(),
            "amir_oku yokken amir yok: {can}"
        );
    }

    #[test]
    fn arama_ad_gorev_sube_birim_uzerinde_aksansiz() {
        let (depo, m) = demo::ornek_depo("2026-11-26T09:00:00Z");
        let v = depo.veri();
        let g = gorunum(v, &bakis(None, true, true, &m));
        // Ad (aksansız)
        assert_eq!(
            adlar(&kisiler(v, &g, &s("ayse"), true, BUGUN).unwrap()),
            vec!["ÖRNEK Ayşe Şube1".to_string()]
        );
        // Birim adı ve şube kodu
        assert!(adlar(&kisiler(v, &g, &s("ornek-s2"), true, BUGUN).unwrap())
            .contains(&"ÖRNEK Can Şube2".to_string()));
        // Çok terim: hepsi eşleşmeli
        assert!(adlar(&kisiler(v, &g, &s("ayse yok-terim"), true, BUGUN).unwrap()).is_empty());
        // E-posta üzerinde arama yapılmaz
        assert!(adlar(&kisiler(v, &g, &s("ornek.test"), true, BUGUN).unwrap()).is_empty());
        // Görev unvanı
        assert!(!adlar(&kisiler(v, &g, &s("kasa sorumlusu"), true, BUGUN).unwrap()).is_empty());
    }

    #[test]
    fn birim_secimi_alt_birimleri_yalniz_hiyerarsi_izniyle_getirir() {
        let (depo, m) = demo::ornek_depo("2026-11-26T09:00:00Z");
        let v = depo.veri();
        let g = gorunum(v, &bakis(None, true, true, &m));
        let sube2 = v
            .birimler
            .values()
            .find(|b| b.sube_kodu.as_deref() == Some(demo::SUBE_2) && b.tur == BirimTur::Sube)
            .unwrap();
        let magaza = v
            .birimler
            .values()
            .find(|b| {
                b.tur == BirimTur::Magaza && b.ust_birim_id.as_deref() == Some(sube2.id.as_str())
            })
            .unwrap();
        let ben = |h| {
            kisiler(
                v,
                &g,
                &Suzgec {
                    birim: Some(sube2.id.clone()),
                    ..Default::default()
                },
                h,
                BUGUN,
            )
            .unwrap()
        };
        let mag_kisi = "ÖRNEK Müdür Şube2".to_string();
        assert!(
            adlar(&ben(true)).contains(&mag_kisi),
            "alt birim (mağaza) dahil"
        );
        assert!(
            !adlar(&ben(false)).contains(&mag_kisi),
            "izin yokken yalnız seçilen birim"
        );
        let _ = magaza;
    }

    #[test]
    fn agac_izne_gore_hiyerarsi_ya_da_gruplar() {
        let (depo, m) = demo::ornek_depo("2026-11-26T09:00:00Z");
        let v = depo.veri();
        let g = gorunum(v, &bakis(None, true, true, &m));
        let a = agac(v, &g, true);
        // Kökler: merkez ve (üstü olmayan) birimler; mağaza bir şubenin altında.
        fn bul<'a>(n: &'a [Value], id: &str) -> Option<&'a Value> {
            n.iter().find_map(|x| {
                if x["id"] == id {
                    Some(x)
                } else {
                    bul(x["cocuklar"].as_array().unwrap(), id)
                }
            })
        }
        let sube2 = v
            .birimler
            .values()
            .find(|b| b.sube_kodu.as_deref() == Some(demo::SUBE_2) && b.tur == BirimTur::Sube)
            .unwrap();
        let d = bul(&a, &sube2.id).unwrap();
        assert!(
            !d["cocuklar"].as_array().unwrap().is_empty(),
            "şube altında mağaza var"
        );
        // İzinsiz: türe göre gruplar, hiç iç içelik yok.
        let g2 = gorunum(v, &bakis(Some(demo::SUBE_1), false, false, &m));
        let b = agac(v, &g2, false);
        assert!(b.iter().all(|n| n["grup"] == true));
        assert!(b.iter().all(|n| n["cocuklar"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["cocuklar"].as_array().unwrap().is_empty())));
        // Görünmeyen üst birim sızmaz: şube cihazında bile hiyerarşi izniyle bağ yalnız görünür üstlerle kurulur.
        let c = agac(v, &g2, true);
        let toplam: usize = {
            fn say(n: &[Value]) -> usize {
                n.iter()
                    .map(|x| 1 + say(x["cocuklar"].as_array().unwrap()))
                    .sum()
            }
            say(&c)
        };
        assert_eq!(toplam, g2.birimler.len());
    }

    #[test]
    fn suzgec_secenekleri_yalniz_gorunur_subeler() {
        let (depo, m) = demo::ornek_depo("2026-11-26T09:00:00Z");
        let v = depo.veri();
        let g = gorunum(v, &bakis(None, false, false, &m)); // yalnız merkez
        let o = suzgecler(&g);
        assert!(
            o["subeler"].as_array().unwrap().is_empty(),
            "kapsam yokken şube listesi boş: {o}"
        );
        assert!(!o["roller"].as_array().unwrap().is_empty());
    }
}
