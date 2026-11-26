// Sahip: Yusuf. yazan: [Claude#egitim-panel 2026-11-26]
//! Panelin veri girişleri.
//!
//! - Katalog dosyası (`<kök>/egitim/katalog.json`): dersler (`taa_egitim::Ders`), derslerin video bağları, konu görünen adları.
//! - Rehber dosyası (`<kök>/rehber/rehber.json`): rehberin yerel, salt okunur önbelleği (kişi, birim, görevlendirme, rol).
//!   Panel rehbere yazmaz. Dosya yoksa ya da tutarsızsa rehber boştur (kimse etkin değil) ve ekranda uyarı çıkar.
//! - Örnek veri yalnız `--demo` ile kurulur ve her yerde "ÖRNEK" etiketlidir; video içermez.

use crate::video::VideoKaynagi;
use serde::Deserialize;
use std::collections::BTreeMap;
use taa_egitim::{Ders, Katalog, KONU_KASA};
use taa_rehber_model::model::{
    BirimDurum, BirimKaydi, BirimTur, EtkinPasif, GorevTur, GorevlendirmeKaydi, KisiDurum,
    KisiKaydi, RolKaydi, Veri,
};

const ADLAR_EN_COK: usize = 128;
const AD_EN_COK: usize = 60;

/// Katalog dosyasının biçimi.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KatalogDosyasi {
    #[serde(default)]
    pub dersler: Vec<Ders>,
    /// ders kimliği → video bağı.
    #[serde(default)]
    pub videolar: BTreeMap<String, VideoKaynagi>,
    /// konu kimliği → görünen ad (yoksa kimlikten türetilir).
    #[serde(default)]
    pub konu_adlari: BTreeMap<String, String>,
}

/// Doğrulanmış katalog.
#[derive(Debug, Default, Clone)]
pub struct YuklenenKatalog {
    pub katalog: Katalog,
    pub videolar: BTreeMap<String, VideoKaynagi>,
    pub konu_adlari: BTreeMap<String, String>,
}

fn ad_ok(a: &str) -> bool {
    let t = a.trim();
    !t.is_empty() && t.chars().count() <= AD_EN_COK && !a.chars().any(char::is_control)
}

/// JSON metninden kataloğu kurar ve doğrular (ders kuralları `taa-egitim`de; video bağları burada).
pub fn katalog_yukle(metin: &str) -> Result<YuklenenKatalog, String> {
    let d: KatalogDosyasi =
        serde_json::from_str(metin).map_err(|e| format!("katalog dosyası okunamadı: {e}"))?;
    let mut k = Katalog::yeni();
    for ders in d.dersler {
        let id = ders.id.clone();
        k.ekle(ders).map_err(|e| format!("ders {id}: {e}"))?;
    }
    if d.videolar.len() > ADLAR_EN_COK * 40 {
        return Err("çok fazla video bağı".into());
    }
    for (id, v) in &d.videolar {
        if k.al(id).is_none() {
            return Err(format!("video bağı katalogda olmayan derse ait: {id}"));
        }
        v.dogrula().map_err(|e| format!("ders {id}: {e}"))?;
    }
    for (konu, ad) in &d.konu_adlari {
        if !ad_ok(ad) || konu.len() > ADLAR_EN_COK {
            return Err(format!("konu adı geçersiz: {konu}"));
        }
    }
    Ok(YuklenenKatalog {
        katalog: k,
        videolar: d.videolar,
        konu_adlari: d.konu_adlari,
    })
}

/// Rehber dosyasının biçimi (salt okunur önbellek).
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RehberDosyasi {
    #[serde(default)]
    pub kisiler: Vec<KisiKaydi>,
    #[serde(default)]
    pub birimler: Vec<BirimKaydi>,
    #[serde(default)]
    pub gorevlendirmeler: Vec<GorevlendirmeKaydi>,
    #[serde(default)]
    pub roller: Vec<RolKaydi>,
}

fn doldur<T>(
    ad: &str,
    kayitlar: Vec<T>,
    anahtar: impl Fn(&T) -> String,
    hedef: &mut BTreeMap<String, T>,
) -> Result<(), String> {
    for k in kayitlar {
        let a = anahtar(&k);
        if hedef.insert(a.clone(), k).is_some() {
            return Err(format!("rehber dosyasında {ad} tekrarı: {a}"));
        }
    }
    Ok(())
}

/// JSON metninden rehber verisini kurar. Bütünlük hatası varsa tüm dosya reddedilir (yarım veri kullanılmaz).
pub fn rehber_yukle(metin: &str) -> Result<Veri, String> {
    let d: RehberDosyasi =
        serde_json::from_str(metin).map_err(|e| format!("rehber dosyası okunamadı: {e}"))?;
    let mut v = Veri::default();
    doldur("kişi", d.kisiler, |k| k.id.clone(), &mut v.kisiler)?;
    doldur("birim", d.birimler, |b| b.id.clone(), &mut v.birimler)?;
    doldur(
        "görevlendirme",
        d.gorevlendirmeler,
        |g| g.id.clone(),
        &mut v.gorevlendirmeler,
    )?;
    doldur("rol", d.roller, |r| r.kod.clone(), &mut v.roller)?;
    let hatalar = v.hatalar();
    if let Some(h) = hatalar.first() {
        return Err(format!(
            "rehber dosyası tutarsız ({} hata), ilki: {h}",
            hatalar.len()
        ));
    }
    Ok(v)
}

/// Konu kimliğinden görünen ad: katalogdaki ad, yoksa kasa eğitimi için sabit ad, yoksa kimlikten türetilir.
pub fn konu_adi(id: &str, adlar: &BTreeMap<String, String>) -> String {
    if let Some(a) = adlar.get(id) {
        return a.clone();
    }
    if id == KONU_KASA {
        return "Kasa eğitimi".into();
    }
    baslik_yap(id)
}

/// `kasa-sorumlusu` → `Kasa sorumlusu` (Türkçe karakter üretilemez; gerçek adlar katalog ve rehber rollerinden gelir).
pub fn baslik_yap(kod: &str) -> String {
    let s = kod.replace(['-', '_'], " ");
    let mut c = s.chars();
    match c.next() {
        Some(i) => i.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// Görev kodundan görünen ad: rehberdeki rol adı, yoksa kimlikten türetilir.
pub fn gorev_adi(kod: &str, rehber: &Veri) -> String {
    rehber
        .roller
        .get(kod)
        .map(|r| r.ad.clone())
        .unwrap_or_else(|| baslik_yap(kod))
}

const DEMO_ZAMAN: &str = "2026-11-26T09:00:00Z";

fn demo_ders(id: &str, baslik: &str, konu: &str, etiketler: &[&str], sure: u64) -> Ders {
    Ders {
        id: id.into(),
        baslik: format!("ÖRNEK: {baslik}"),
        konu: konu.into(),
        gorev_etiketleri: etiketler.iter().map(|s| s.to_string()).collect(),
        sure_sn: sure,
        surum: 1,
    }
}

/// `--demo`: ÖRNEK etiketli katalog. Video bağı yoktur (video üretilmez); ekran dürüst boş durumu gösterir.
pub fn demo_katalog() -> YuklenenKatalog {
    let mut k = Katalog::yeni();
    for d in [
        demo_ders(
            "ornek-kasa-acilis",
            "Kasa açılışı",
            KONU_KASA,
            &["kasa-sorumlusu"],
            600,
        ),
        demo_ders(
            "ornek-kasa-gun-sonu",
            "Kasa gün sonu",
            KONU_KASA,
            &["kasa-sorumlusu"],
            900,
        ),
        demo_ders(
            "ornek-iade",
            "İade işlemleri",
            "satis",
            &["satis-danismani"],
            300,
        ),
        demo_ders("ornek-rehber", "Rehber kullanımı", "uygulama", &[], 240),
    ] {
        let _ = k.ekle(d);
    }
    YuklenenKatalog {
        katalog: k,
        videolar: BTreeMap::new(),
        konu_adlari: BTreeMap::from([
            ("satis".into(), "Satış".into()),
            ("uygulama".into(), "Uygulama kullanımı".into()),
        ]),
    }
}

fn demo_kisi(id: &str, ad: &str) -> KisiKaydi {
    KisiKaydi {
        id: id.into(),
        ad: ad.into(),
        eposta: format!("{id}@ornek.invalid"),
        durum: KisiDurum::Etkin,
        pasif_tarihi: None,
        surum: 1,
        guncelleme: DEMO_ZAMAN.into(),
    }
}

/// `--demo`: ÖRNEK etiketli rehber (üç kişi, bir birim, iki rol).
pub fn demo_rehber() -> Veri {
    let mut v = Veri::default();
    for (id, ad) in [
        ("ornek-1", "ÖRNEK Kişi 1"),
        ("ornek-2", "ÖRNEK Kişi 2"),
        ("ornek-3", "ÖRNEK Kişi 3"),
    ] {
        v.kisiler.insert(id.into(), demo_kisi(id, ad));
    }
    v.birimler.insert(
        "ornek-birim".into(),
        BirimKaydi {
            id: "ornek-birim".into(),
            ad: "ÖRNEK Birim".into(),
            tur: BirimTur::Departman,
            ust_birim_id: None,
            sube_kodu: None,
            ortak_eposta: None,
            durum: BirimDurum::Etkin,
            surum: 1,
            guncelleme: DEMO_ZAMAN.into(),
        },
    );
    for (kod, ad) in [
        ("kasa-sorumlusu", "ÖRNEK Kasa sorumlusu"),
        ("satis-danismani", "ÖRNEK Satış danışmanı"),
    ] {
        v.roller.insert(
            kod.into(),
            RolKaydi {
                kod: kod.into(),
                ad: ad.into(),
                aciklama: None,
                durum: EtkinPasif::Etkin,
            },
        );
    }
    for (i, (kisi, rol)) in [
        ("ornek-1", "kasa-sorumlusu"),
        ("ornek-2", "satis-danismani"),
    ]
    .into_iter()
    .enumerate()
    {
        let id = format!("ornek-gorev-{}", i + 1);
        v.gorevlendirmeler.insert(
            id.clone(),
            GorevlendirmeKaydi {
                id,
                kisi_id: kisi.into(),
                birim_id: "ornek-birim".into(),
                rol: rol.into(),
                gorev: None,
                amir_id: None,
                tur: GorevTur::Asil,
                vekil_edilen_id: None,
                durum: EtkinPasif::Etkin,
                baslangic: "2026-01-01".into(),
                bitis: None,
                surum: 1,
                guncelleme: DEMO_ZAMAN.into(),
            },
        );
    }
    v
}

/// `--demo`: oturum sahibi rehberde yoksa ÖRNEK kişi olarak eklenir (gerçek rehber kaydı uydurulmaz; yalnız bellekte).
/// Dönüş: eklendi mi.
pub fn demo_oturum_sahibi_ekle(v: &mut Veri, kisi_id: &str) -> bool {
    if v.kisiler.contains_key(kisi_id) {
        return false;
    }
    v.kisiler
        .insert(kisi_id.into(), demo_kisi(kisi_id, "ÖRNEK oturum sahibi"));
    true
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn katalog_dosyasi_okunur() {
        let m = r#"{
            "dersler": [{"id":"kasa-acilis","baslik":"Kasa açılışı","konu":"kasa-egitimi",
                         "gorev_etiketleri":["kasa-sorumlusu"],"sure_sn":600,"surum":1}],
            "videolar": {"kasa-acilis": {"yol": "kasa/acilis.mp4"}},
            "konu_adlari": {"kasa-egitimi": "Kasa eğitimi"}
        }"#;
        let y = katalog_yukle(m).unwrap();
        assert_eq!(y.katalog.len(), 1);
        assert_eq!(
            y.videolar["kasa-acilis"].yol.as_deref(),
            Some("kasa/acilis.mp4")
        );
    }

    #[test]
    fn katalog_hatalari_reddedilir() {
        assert!(katalog_yukle("{bozuk").is_err());
        assert!(katalog_yukle(r#"{"bilinmeyen":1}"#).is_err());
        let ders = r#"{"id":"a","baslik":"A","konu":"genel","sure_sn":60,"surum":1}"#;
        // yinelenen ders
        assert!(katalog_yukle(&format!(r#"{{"dersler":[{ders},{ders}]}}"#)).is_err());
        // olmayan derse video
        assert!(katalog_yukle(r#"{"videolar":{"yok":{"yol":"a.mp4"}}}"#).is_err());
        // güvensiz yol ve http adresi
        assert!(katalog_yukle(&format!(
            r#"{{"dersler":[{ders}],"videolar":{{"a":{{"yol":"../a.mp4"}}}}}}"#
        ))
        .is_err());
        assert!(katalog_yukle(&format!(
            r#"{{"dersler":[{ders}],"videolar":{{"a":{{"url":"http://o.test/a.mp4"}}}}}}"#
        ))
        .is_err());
        // geçersiz ders (süre 0)
        assert!(katalog_yukle(
            r#"{"dersler":[{"id":"a","baslik":"A","konu":"genel","sure_sn":0,"surum":1}]}"#
        )
        .is_err());
        // bozuk konu adı
        assert!(katalog_yukle(r#"{"konu_adlari":{"genel":"iki\nsatır"}}"#).is_err());
        // boş katalog geçerlidir (dürüst boş durum)
        assert!(katalog_yukle("{}").unwrap().katalog.is_empty());
    }

    #[test]
    fn rehber_dosyasi_okunur_ve_tutarsizlik_reddedilir() {
        let tamam = r#"{
            "kisiler":[{"id":"ayse","ad":"Ayşe","eposta":"ayse@sirket.test","durum":"etkin","surum":1,"guncelleme":"2026-11-26T09:00:00Z"}]
        }"#;
        assert!(rehber_yukle(tamam).unwrap().kisiler.contains_key("ayse"));
        // görevlendirme olmayan kişiye ve birime bakıyor
        let tutarsiz = r#"{
            "gorevlendirmeler":[{"id":"g1","kisi_id":"yok","birim_id":"yok","rol":"kasa","tur":"asil","durum":"etkin","baslangic":"2026-01-01","surum":1,"guncelleme":"2026-11-26T09:00:00Z"}]
        }"#;
        assert!(rehber_yukle(tutarsiz).unwrap_err().contains("tutarsız"));
        assert!(rehber_yukle("{bozuk").is_err());
        let tekrar = r#"{"kisiler":[
            {"id":"a","ad":"A","eposta":"a@s.test","durum":"etkin","surum":1,"guncelleme":"2026-11-26T09:00:00Z"},
            {"id":"a","ad":"B","eposta":"b@s.test","durum":"etkin","surum":1,"guncelleme":"2026-11-26T09:00:00Z"}]}"#;
        assert!(rehber_yukle(tekrar).unwrap_err().contains("tekrarı"));
        // bilinmeyen alan (telefon gibi) reddedilir
        let telefon = r#"{"kisiler":[{"id":"a","ad":"A","eposta":"a@s.test","telefon":"1","durum":"etkin","surum":1,"guncelleme":"2026-11-26T09:00:00Z"}]}"#;
        assert!(rehber_yukle(telefon).is_err());
    }

    #[test]
    fn adlar() {
        let adlar = BTreeMap::from([("satis".to_string(), "Satış".to_string())]);
        assert_eq!(konu_adi("satis", &adlar), "Satış");
        assert_eq!(konu_adi(KONU_KASA, &adlar), "Kasa eğitimi");
        assert_eq!(konu_adi("genel-bilgi", &adlar), "Genel bilgi");
        assert_eq!(baslik_yap(""), "");
        let v = demo_rehber();
        assert_eq!(gorev_adi("kasa-sorumlusu", &v), "ÖRNEK Kasa sorumlusu");
        assert_eq!(gorev_adi("depo-sorumlusu", &v), "Depo sorumlusu");
    }

    #[test]
    fn demo_veri_gecerli_ve_ornek_etiketli() {
        let v = demo_rehber();
        assert!(v.hatalar().is_empty(), "{:?}", v.hatalar());
        assert!(v.kisiler.values().all(|k| k.ad.starts_with("ÖRNEK")));
        let k = demo_katalog();
        assert_eq!(k.katalog.len(), 4);
        assert!(k
            .katalog
            .hepsi()
            .iter()
            .all(|d| d.baslik.starts_with("ÖRNEK")));
        assert!(k.videolar.is_empty(), "demo video üretmez");
        let mut v = v;
        assert!(demo_oturum_sahibi_ekle(&mut v, "ayse"));
        assert!(!demo_oturum_sahibi_ekle(&mut v, "ayse"));
        assert!(v.hatalar().is_empty());
    }
}
