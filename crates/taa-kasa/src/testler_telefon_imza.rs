// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! Telefonla imza istemcisi testleri: sahte köprü kanalıyla onay, red, zaman aşımı, özet uyuşmazlığı,
//! görev ayrılığı ve köprü yokken (yer tutucu boru) davranış. Tüm veriler anonim test verisidir.

use super::*;
use crate::gunsonu::{
    DovizSatiri, GunSonuGirdi, KasaAyar, NoktaGunSonu, Rol, SayimGirdi, SayimTuru,
};
use serde_json::json;
use std::collections::{BTreeMap, VecDeque};

const T: &str = "2026-11-26";
const NONCE: &str = "0123456789abcdef0123456789abcdef";
const SIMDI: i64 = 1_800_000_000;

fn defter(ad: &str) -> KasaDefteri {
    KasaDefteri::yeni(
        taa_dosya::gecici_klasor()
            .unwrap()
            .join(format!("kasa-tel-{ad}")),
    )
}

/// İmzalayan kişi: imza yalnız yöneticilerin (mağaza müdürü rolü).
fn kisi(ad: &str) -> Kisi {
    Kisi {
        ad: ad.into(),
        rol: Rol::Mudur,
        dogrulandi: false,
    }
}

fn yonetici() -> Kisi {
    Kisi {
        ad: "Merkez".into(),
        rol: Rol::Yonetici,
        dogrulandi: true,
    }
}

fn ayar() -> KasaAyar {
    KasaAyar {
        sube: "SB1".into(),
        ..KasaAyar::default()
    }
}

fn girdi() -> GunSonuGirdi {
    GunSonuGirdi {
        tarih: T.into(),
        cekmece: "c1".into(),
        nakit_adetler: BTreeMap::from([(10_000, 3)]),
        noktalar: vec![NoktaGunSonu {
            nokta: "k1".into(),
            z_toplam_kurus: 30_000,
            pos: BTreeMap::from([("ziraat".into(), 10_000)]),
        }],
        nebim_toplam_kurus: 30_000,
        ciro: BTreeMap::from([("nakit".into(), 20_000), ("ziraat_pos".into(), 10_000)]),
        doviz: vec![DovizSatiri {
            kod: "USD".into(),
            tutar: 100,
            kur_onbinde: 400_000,
        }],
        devreden_kurus: 0,
        kontrol: vec![true; 7],
        aciklamalar: BTreeMap::new(),
        not: "test notu".into(),
    }
}

/// Açılış + gün sonu (giren: "Kasiyer A"). Dönen: (defter, ayar, gün sonu id).
fn hazir(ad: &str) -> (KasaDefteri, KasaAyar, String) {
    let d = defter(ad);
    let a = ayar();
    d.sayim_ekle(
        &a,
        SayimTuru::Acilis,
        &SayimGirdi {
            tarih: T.into(),
            cekmece: "c1".into(),
            adetler: BTreeMap::from([(10_000, 1)]),
            ..Default::default()
        },
        &kisi("Kasiyer A"),
    )
    .unwrap();
    let gs = d.gun_sonu_ekle(&a, &girdi(), &kisi("Kasiyer A")).unwrap();
    (d, a, gs.id)
}

/// Sahte köprü: gönderilen istekleri saklar, sıradaki cevabı yoklamada verir.
#[derive(Default)]
struct SahteKanal {
    giden: Vec<OnayIstegi>,
    cevaplar: VecDeque<Option<Value>>,
    hata: Option<KanalHatasi>,
}

impl KopruKanali for SahteKanal {
    fn gonder(&mut self, istek: &OnayIstegi) -> Result<(), KanalHatasi> {
        if let Some(h) = &self.hata {
            return Err(h.clone());
        }
        self.giden.push(istek.clone());
        Ok(())
    }
    fn yokla(&mut self, _nonce: &str) -> Result<Option<Value>, KanalHatasi> {
        if let Some(h) = &self.hata {
            return Err(h.clone());
        }
        Ok(self.cevaplar.pop_front().flatten())
    }
}

fn onay(istek: &OnayIstegi) -> Value {
    json!({
        "tur": ONAY_TURU,
        "nonce": istek.nonce,
        "kapsam": KAPSAM,
        "ozet": istek.ozet,
        "cihaz": {"eslesme_kimligi": "es-1", "pc_pin": "ab12"},
        "kisi_dogrulandi": true,
        "zaman": SIMDI + 5,
        "bilinmeyen_alan": 1
    })
}

#[test]
fn ozet_kanonik_ve_tutar_degisince_degisir() {
    let (d, _, id) = hazir("ozet");
    let gs = d
        .gun_sonlari()
        .unwrap()
        .into_iter()
        .find(|s| s.id == id)
        .unwrap();
    let m = ozet_metni(&gs);
    assert!(
        m.starts_with("cekmece=c1\n"),
        "anahtarlar sözlük sırasında: {m}"
    );
    assert!(m.contains("ciro.nakit=20000\n"));
    assert!(m.contains("doviz.USD.kur_onbinde=400000\n"));
    assert!(m.contains("nokta.k1.pos.ziraat=10000\n"));
    assert!(m.contains(&format!("kapsam={KAPSAM}\n")));
    assert!(!m.contains("Kasiyer"), "özet metnine kişi adı girmez");
    let h = ozet_sha256(&gs);
    assert_eq!(h.len(), 64);
    assert_eq!(h, ozet_sha256(&gs.clone()), "aynı kayıt aynı özet");
    let mut g2 = gs.clone();
    *g2.ciro.get_mut("nakit").unwrap() += 1;
    assert_ne!(h, ozet_sha256(&g2), "bir kuruş fark özeti değiştirir");
}

#[test]
fn onay_gelince_telefon_imzasi_deftere_yazilir() {
    let (d, _, id) = hazir("onay");
    let mut k = SahteKanal::default();
    let mut o =
        TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), NONCE, SIMDI).unwrap();
    assert_eq!(o.durum, ImzaDurumu::Bekliyor);
    let ist = k.giden[0].clone();
    assert_eq!(
        (
            ist.tur.as_str(),
            ist.kapsam.as_str(),
            ist.sira,
            ist.rol.as_str()
        ),
        (ISTEK_TURU, KAPSAM, 1, "kasa_sorumlusu")
    );
    assert_eq!(ist.bitis - ist.olusturma, GECERLILIK_SN);
    assert_eq!(ist.nonce, NONCE);
    assert_eq!(ist.baslik, format!("Gün sonu · SB1 · {T}"));
    assert_eq!(ist.ozet.len(), 64);
    assert!(ist
        .ozet
        .chars()
        .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)));
    assert_eq!(
        ist.ozet,
        hex(&Sha256::digest(ist.ozet_metni.as_bytes())),
        "ozet = sha256(ozet_metni)"
    );
    // Tel üstü biçim: TK sözleşmesi alan adları; pc_adi kasa tarafından gönderilmez.
    let v = serde_json::to_value(&ist).unwrap();
    let mut adlar: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    adlar.sort();
    assert_eq!(
        adlar,
        [
            "baslik",
            "bitis",
            "gun_sonu_id",
            "istenen_kisi",
            "kapsam",
            "nonce",
            "olusturma",
            "ozet",
            "ozet_metni",
            "rol",
            "rol_adi",
            "sira",
            "tur"
        ]
    );
    // Henüz cevap yok: bekler.
    k.cevaplar.push_back(None);
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 10), &ImzaDurumu::Bekliyor);
    k.cevaplar.push_back(Some(onay(&ist)));
    let ImzaDurumu::Onaylandi(i) = o.ilerle(&d, &mut k, SIMDI + 20).clone() else {
        panic!("onaylanmadı: {:?}", o.durum)
    };
    assert_eq!(i.kanal, ImzaKanali::Telefon);
    assert_eq!(i.cihaz.as_deref(), Some("es-1:ab12"));
    assert_eq!(i.ozet.as_deref(), Some(ist.ozet.as_str()));
    assert_eq!((i.sira, i.kisi.as_str()), (1, "Kasiyer A"));
    // Bitmiş oturum yeniden ilerletilince ikinci imza atılmaz.
    k.cevaplar.push_back(Some(onay(&ist)));
    o.ilerle(&d, &mut k, SIMDI + 21);
    assert_eq!(d.imza_haritasi().unwrap()[&id].len(), 1);
}

#[test]
fn red_ve_zaman_asimi_imza_atmaz() {
    let (d, _, id) = hazir("red");
    let mut k = SahteKanal::default();
    let mut o =
        TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), NONCE, SIMDI).unwrap();
    k.cevaplar.push_back(Some(
        json!({"tur": RED_TURU, "nonce": NONCE, "neden": "kullanici"}),
    ));
    assert_eq!(
        o.ilerle(&d, &mut k, SIMDI + 5),
        &ImzaDurumu::Reddedildi {
            neden: RedNedeni::Kullanici
        }
    );
    // Zaman aşımı: süre dolduktan sonra gelen onay kabul edilmez.
    let n2 = "fedcba9876543210fedcba9876543210";
    let mut o2 =
        TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), n2, SIMDI).unwrap();
    let ist = o2.istek.clone();
    k.cevaplar.push_back(Some(onay(&ist)));
    assert_eq!(
        o2.ilerle(&d, &mut k, SIMDI + GECERLILIK_SN + 1),
        &ImzaDurumu::ZamanAsimi
    );
    assert_eq!(d.imza_haritasi().unwrap().get(&id).map_or(0, Vec::len), 0);
}

#[test]
fn ozet_nonce_veya_tutar_tutmazsa_gecersiz() {
    let (d, a, id) = hazir("gecersiz");
    let mut k = SahteKanal::default();
    let ki = kisi("Kasiyer A");
    // Başka özet.
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &ki, NONCE, SIMDI).unwrap();
    let mut c = onay(&o.istek);
    c["ozet"] = json!("00".repeat(32));
    k.cevaplar.push_back(Some(c));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1).kod(), "gecersiz");
    // Başka nonce.
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &ki, NONCE, SIMDI).unwrap();
    let mut c = onay(&o.istek);
    c["nonce"] = json!("baska-nonce-0000000000000");
    k.cevaplar.push_back(Some(c));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1).kod(), "gecersiz");
    // Cihaz kimliği eksik.
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &ki, NONCE, SIMDI).unwrap();
    let mut c = onay(&o.istek);
    c["cihaz"] = json!({"eslesme_kimligi": "es-1"});
    k.cevaplar.push_back(Some(c));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1).kod(), "gecersiz");
    // Telefondaki kişi başka.
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &ki, NONCE, SIMDI).unwrap();
    let mut c = onay(&o.istek);
    c["kisi"] = json!("Kasiyer B");
    k.cevaplar.push_back(Some(c));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1).kod(), "gecersiz");
    // İstekten sonra gün sonu düzeltildi: eski özetle imza atılmaz.
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &ki, NONCE, SIMDI).unwrap();
    let mut g = girdi();
    g.ciro.insert("nakit".into(), 20_100);
    d.gun_sonu_duzelt(&a, &id, &g, "nakit düzeltme", &yonetici())
        .unwrap();
    k.cevaplar.push_back(Some(onay(&o.istek)));
    let ImzaDurumu::Gecersiz { neden } = o.ilerle(&d, &mut k, SIMDI + 1).clone() else {
        panic!("{:?}", o.durum)
    };
    assert!(neden.contains("geçerli değil"), "{neden}");
    assert_eq!(d.imza_haritasi().unwrap().get(&id).map_or(0, Vec::len), 0);
}

#[test]
fn gorev_ayriligi_istekten_once_denetlenir() {
    let (d, _, id) = hazir("gorev");
    let mut k = SahteKanal::default();
    // Personel telefonla da imzalayamaz (yalnız yöneticiler); telefona istek gitmez.
    let personel = Kisi {
        rol: Rol::Personel,
        ..kisi("Kasiyer B")
    };
    let e = TelefonImzaOturumu::baslat(&d, &mut k, &id, &personel, NONCE, SIMDI).unwrap_err();
    assert_eq!(e, crate::gunsonu::YETKISIZ_IMZA);
    assert!(k.giden.is_empty());
    // Birinci imza (giren, ekrandan).
    d.imzala(&id, ImzaKanali::Ekran, None, &kisi("Kasiyer A"))
        .unwrap();
    // Giren ikinci imzayı telefonla da atamaz; telefona istek hiç gitmez.
    let e =
        TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("kasiyer a"), NONCE, SIMDI).unwrap_err();
    assert!(e.contains("aynı kişi"), "{e}");
    assert!(k.giden.is_empty());
    // Başka kişi: ikinci imza, rol mağaza sorumlusu.
    let mut o =
        TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Sorumlu B"), NONCE, SIMDI).unwrap();
    assert_eq!(
        (o.istek.sira, o.istek.rol.as_str()),
        (2, "magaza_sorumlusu")
    );
    k.cevaplar.push_back(Some(onay(&o.istek)));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 3).kod(), "onaylandi");
    // Üçüncü istek açılmaz.
    assert!(
        TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Biri C"), NONCE, SIMDI)
            .unwrap_err()
            .contains("iki imza")
    );
    // Kısa nonce reddedilir.
    assert!(TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Biri C"), "kisa", SIMDI).is_err());
}

#[test]
fn kopru_yokken_kanal_yok_ve_501_mesaji() {
    let (d, _, id) = hazir("yok");
    let mut b = BoruKanali::default();
    assert_eq!(b.boru, BORU_ADI);
    let mut o =
        TelefonImzaOturumu::baslat(&d, &mut b, &id, &kisi("Kasiyer A"), NONCE, SIMDI).unwrap();
    assert_eq!(
        o.durum,
        ImzaDurumu::KanalYok {
            neden: KOPRU_YOK_MESAJI.into()
        }
    );
    assert_eq!(o.ilerle(&d, &mut b, SIMDI + 1).kod(), "kanal_yok");
    assert_eq!(d.imza_haritasi().unwrap().get(&id).map_or(0, Vec::len), 0);
    // Geçici kanal hatası beklemeye devam eder, süre dolunca zaman aşımı.
    let mut k = SahteKanal::default();
    let mut o =
        TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), NONCE, SIMDI).unwrap();
    k.hata = Some(KanalHatasi::Hata("boru meşgul".into()));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1), &ImzaDurumu::Bekliyor);
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 61), &ImzaDurumu::ZamanAsimi);
}

#[test]
fn cevap_cozumu() {
    // Düz cihaz alanları ve eski uzak köprü adı; özet küçük harfe çevrilir.
    let c = KopruCevabi::jsondan(&json!({"tur":"uk_onay","nonce":"n1","ozet":"ABC",
        "eslesme_kimligi":"e","pc_pin":"p"}))
    .unwrap();
    assert!(matches!(c, KopruCevabi::Onay { ref ozet, .. } if ozet == "abc"));
    assert!(matches!(
        KopruCevabi::jsondan(&json!({"tur":"onay_red@1","nonce":"n1","neden":"biyometri"}))
            .unwrap(),
        KopruCevabi::Red {
            neden: RedNedeni::Biyometri,
            ..
        }
    ));
    assert!(KopruCevabi::jsondan(&json!({"tur":"baska","nonce":"n1"})).is_err());
    assert!(KopruCevabi::jsondan(&json!({"tur":"onay@1"})).is_err());
    // Eski alan adı artık tanınmaz (sözleşme kesin; geriye uyumluluk yok).
    assert!(KopruCevabi::jsondan(&json!({"tur":"onay_red@1","istek_kimligi":"n1"})).is_err());
}

#[test]
fn alti_red_nedeni_taninir_her_biri_imzasiz_birakir_ve_turkce_mesaj_verir() {
    let nedenler = [
        ("kullanici", RedNedeni::Kullanici, "reddedildi"),
        ("biyometri", RedNedeni::Biyometri, "doğrulaması"),
        ("zaman_asimi", RedNedeni::ZamanAsimi, "60 saniye"),
        ("kip_kapali", RedNedeni::KipKapali, "onay kipi"),
        ("kapsam_yok", RedNedeni::KapsamYok, "izni"),
        ("ozet_uyusmaz", RedNedeni::OzetUyusmaz, "uyuşmadı"),
        (
            "yeni_bir_neden",
            RedNedeni::Bilinmeyen("yeni_bir_neden".into()),
            "yeni_bir_neden",
        ),
        ("", RedNedeni::Bilinmeyen(String::new()), "bildirilmedi"),
    ];
    for (i, (kod, beklenen, parca)) in nedenler.into_iter().enumerate() {
        let (d, _, id) = hazir(&format!("red-{i}"));
        let mut k = SahteKanal::default();
        let mut o =
            TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), NONCE, SIMDI).unwrap();
        k.cevaplar
            .push_back(Some(json!({"tur": RED_TURU, "nonce": NONCE, "neden": kod})));
        let s = o.ilerle(&d, &mut k, SIMDI + 2).clone();
        assert_eq!(
            s,
            ImzaDurumu::Reddedildi {
                neden: beklenen.clone()
            },
            "{kod}"
        );
        assert_eq!(s.kod(), "reddedildi");
        assert!(s.imzasiz_bitti(), "{kod}: sessizce geçmez, imzasız biter");
        assert_eq!(s.red_nedeni(), Some(kod));
        let m = s.mesaj();
        assert!(m.contains(parca), "{kod}: {m}");
        assert!(
            m.contains("imzasız"),
            "{kod}: mesaj gün sonunun imzasız kaldığını söyler: {m}"
        );
        assert_eq!(beklenen.kod(), kod);
        // Hiçbir redde defterde imza yok; red sonrası gelen onay da yok sayılır.
        k.cevaplar.push_back(Some(onay(&o.istek)));
        o.ilerle(&d, &mut k, SIMDI + 3);
        assert_eq!(
            d.imza_haritasi().unwrap().get(&id).map_or(0, Vec::len),
            0,
            "{kod}"
        );
    }
    // Başka isteğe ait red geçersizdir (yine imzasız).
    let (d, _, id) = hazir("red-baska");
    let mut k = SahteKanal::default();
    let mut o =
        TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), NONCE, SIMDI).unwrap();
    k.cevaplar.push_back(Some(
        json!({"tur": RED_TURU, "nonce": "baska-nonce-0000000000000", "neden": "kullanici"}),
    ));
    let s = o.ilerle(&d, &mut k, SIMDI + 1).clone();
    assert_eq!(s.kod(), "gecersiz");
    assert!(s.imzasiz_bitti() && s.mesaj().contains("imzasız"));
    // Kasa tarafı zaman aşımı ve kanal yok da imzasız biter.
    assert!(ImzaDurumu::ZamanAsimi.imzasiz_bitti());
    assert!(ImzaDurumu::ZamanAsimi.mesaj().contains("60 saniye"));
    assert!(!ImzaDurumu::Bekliyor.imzasiz_bitti());
}

#[test]
fn baslik_en_cok_80_karakter() {
    let (d, _, id) = hazir("baslik");
    let mut gs = d
        .gun_sonlari()
        .unwrap()
        .into_iter()
        .find(|s| s.id == id)
        .unwrap();
    assert_eq!(baslik(&gs), "Gün sonu · SB1 · 2026-11-26");
    gs.sube = "Ş".repeat(100);
    let b = baslik(&gs);
    assert_eq!(b.chars().count(), BASLIK_EN_COK);
    assert!(b.starts_with("Gün sonu · ŞŞ") && b.ends_with('…'));
    gs.sube = "A\nB".into();
    assert!(!baslik(&gs).contains('\n'));
}

/// Sahte TAA Merkez cihaz kaydı.
struct SahteKayit(CihazDurumu);
impl CihazKaydi for SahteKayit {
    fn denetle(&self, _cihaz: &str, _kisi: &str) -> CihazDurumu {
        self.0
    }
}

#[test]
fn merkez_cihaz_kaydi_iptal_ve_baska_kisi_onayi_gecersiz_kilar() {
    for (durum, beklenen) in [
        (CihazDurumu::Iptal, "gecersiz"),
        (CihazDurumu::BaskaKisi, "gecersiz"),
        (CihazDurumu::YetkisizKisi, "gecersiz"),
        (CihazDurumu::Gecerli, "onaylandi"),
    ] {
        let (d, _, id) = hazir(&format!("kayit-{}", durum.kod()));
        let mut k = SahteKanal::default();
        let mut o =
            TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), NONCE, SIMDI).unwrap();
        k.cevaplar.push_back(Some(onay(&o.istek)));
        let s = o
            .ilerle_denetle(&d, &mut k, &SahteKayit(durum), SIMDI + 1)
            .clone();
        assert_eq!(s.kod(), beklenen, "{durum:?}");
        if let ImzaDurumu::Onaylandi(i) = s {
            assert_eq!(i.cihaz_dogrulama.as_deref(), Some("gecerli"));
        }
    }
    // Yer tutucu kayıt: bilinmiyor, imza engellenmez ama sonuç yazılır.
    let (d, _, id) = hazir("kayit-yok");
    let mut k = SahteKanal::default();
    let mut o =
        TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), NONCE, SIMDI).unwrap();
    k.cevaplar.push_back(Some(onay(&o.istek)));
    let ImzaDurumu::Onaylandi(i) = o.ilerle(&d, &mut k, SIMDI + 1).clone() else {
        panic!("{:?}", o.durum)
    };
    assert_eq!(i.cihaz_dogrulama.as_deref(), Some("bilinmiyor"));
}

#[test]
fn merkez_paketteki_telefon_imzasini_ozet_ve_cihazla_denetler() {
    use crate::merkez::telefon_imzalarini_denetle;
    let (d, a, id) = hazir("merkez-denetim");
    let mut k = SahteKanal::default();
    let mut o =
        TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), NONCE, SIMDI).unwrap();
    k.cevaplar.push_back(Some(onay(&o.istek)));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1).kod(), "onaylandi");
    d.ikinci_imza_gerekcesi(&id, "ikinci kişi yok", &kisi("Kasiyer A"))
        .unwrap();
    let p = d.gonderim_paketi(&a, &id).unwrap();
    assert_eq!(p["imzalar"][0]["ozet"], o.istek.ozet);
    // Yer tutucu kayıt: sorun yok, bir imza doğrulanamadı.
    assert_eq!(telefon_imzalarini_denetle(&p, &MerkezKaydiYok), (vec![], 1));
    assert_eq!(
        telefon_imzalarini_denetle(&p, &SahteKayit(CihazDurumu::Gecerli)),
        (vec![], 0)
    );
    let (s, _) = telefon_imzalarini_denetle(&p, &SahteKayit(CihazDurumu::Iptal));
    assert!(s[0].contains("iptal"), "{s:?}");
    let (s, _) = telefon_imzalarini_denetle(&p, &SahteKayit(CihazDurumu::YetkisizKisi));
    assert!(s[0].contains("yönetici değil"), "{s:?}");
    // Rol merkezde de denetlenir: pakette imzalayan personel görünürse sorun (her kanal).
    assert_eq!(p["imzalar"][0]["kisi_rol"], "mudur");
    let mut personel = p.clone();
    personel["imzalar"][0]["kisi_rol"] = json!("personel");
    personel["imzalar"][0]["kanal"] = json!("ekran");
    let (s, _) = telefon_imzalarini_denetle(&personel, &MerkezKaydiYok);
    assert_eq!(
        s,
        vec!["1. imza: imzalayan yönetici değil (personel)".to_string()]
    );
    // Paketteki gün sonu tutarı sonradan değiştirilirse özet tutmaz.
    let mut bozuk = p.clone();
    bozuk["gun_sonu"]["nebim_toplam_kurus"] = json!(1);
    let (s, _) = telefon_imzalarini_denetle(&bozuk, &SahteKayit(CihazDurumu::Gecerli));
    assert!(s[0].contains("özet"), "{s:?}");
}

#[test]
fn sik_tek_imza_uyarisi_esikte_cikar() {
    use crate::merkez::{matris, tek_imza_uyarisi_uygula, GelenPaket};
    let p = |t: &str, tek: bool| GelenPaket {
        sube: "SB1".into(),
        tarih: t.into(),
        alinma_sn: 0,
        ikinci_imza_atmadi: tek,
        ..Default::default()
    };
    let gelen = vec![
        p("2026-11-01", true), // pencere dışında
        p("2026-11-20", true),
        p("2026-11-22", true),
        p("2026-11-22", true), // aynı gün iki çekmece: bir gün sayılır
        p("2026-11-24", false),
        p("2026-11-25", true),
    ];
    let subeler = vec!["SB1".to_string()];
    let tarihler = vec!["2026-11-26".to_string()];
    let mut m = matris(&gelen, &subeler, &tarihler, "23:00", 180, 0);
    tek_imza_uyarisi_uygula(&mut m, &gelen, "2026-11-26", 14, 3);
    assert_eq!((m[0].tek_imza_gun_sayisi, m[0].tek_imza_uyarisi), (3, true));
    tek_imza_uyarisi_uygula(&mut m, &gelen, "2026-11-26", 14, 4);
    assert!(!m[0].tek_imza_uyarisi);
    tek_imza_uyarisi_uygula(&mut m, &gelen, "2026-11-26", 14, 0);
    assert!(!m[0].tek_imza_uyarisi, "eşik 0 kapalı");
    assert_eq!(KasaAyar::default().tek_imza_uyarisi.esik, 3);
}

/// Özet biçimi SABİT (`OZET_SURUMU` 1): Telefon Köprüsü ile çapraz test vektörleri
/// (`testdata/telefon-ozet-vektorleri.json`, uydurma veri, Python başvuru uygulamasıyla üretildi). Bu test
/// kırılırsa biçim değişmiştir: vektörler değil `OZET_SURUMU` ve sözleşme (docs/sozlesmeler/kasa.telefon-ozet.md)
/// güncellenmelidir.
#[test]
fn ozet_bicimi_sabit_capraz_test_vektorleri() {
    let yol = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata/telefon-ozet-vektorleri.json");
    let d: serde_json::Value = serde_json::from_slice(&std::fs::read(yol).unwrap()).unwrap();
    let v = d["vektorler"].as_array().unwrap();
    assert!(v.len() >= 3);
    for x in v {
        let ad = x["ad"].as_str().unwrap();
        let gs: crate::gunsonu::GunSonuKaydi =
            serde_json::from_value(x["gun_sonu"].clone()).unwrap();
        assert_eq!(
            ozet_metni(&gs),
            x["beklenen_metin"].as_str().unwrap(),
            "{ad}: metin"
        );
        assert_eq!(
            ozet_sha256(&gs),
            x["beklenen_sha256"].as_str().unwrap(),
            "{ad}: sha256"
        );
    }
    assert_eq!(OZET_SURUMU, 1);
    assert_eq!(KAPSAM, "taa_kasa_gun_sonu");
}
