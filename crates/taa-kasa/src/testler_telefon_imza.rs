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

fn kisi(ad: &str) -> Kisi {
    Kisi {
        ad: ad.into(),
        rol: Rol::Personel,
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
    fn yokla(&mut self, _istek_kimligi: &str) -> Result<Option<Value>, KanalHatasi> {
        if let Some(h) = &self.hata {
            return Err(h.clone());
        }
        Ok(self.cevaplar.pop_front().flatten())
    }
}

fn onay(istek: &OnayIstegi) -> Value {
    json!({
        "tur": ONAY_TURU,
        "istek_kimligi": istek.istek_kimligi,
        "kapsam": KAPSAM,
        "ozet_sha256": istek.ozet_sha256,
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
    assert_eq!(ist.son_gecerlilik - ist.olusturma, GECERLILIK_SN);
    // Henüz cevap yok: bekler.
    k.cevaplar.push_back(None);
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 10), &ImzaDurumu::Bekliyor);
    k.cevaplar.push_back(Some(onay(&ist)));
    let ImzaDurumu::Onaylandi(i) = o.ilerle(&d, &mut k, SIMDI + 20).clone() else {
        panic!("onaylanmadı: {:?}", o.durum)
    };
    assert_eq!(i.kanal, ImzaKanali::Telefon);
    assert_eq!(i.cihaz.as_deref(), Some("es-1:ab12"));
    assert_eq!(i.ozet.as_deref(), Some(ist.ozet_sha256.as_str()));
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
        json!({"tur": RED_TURU, "istek_kimligi": NONCE, "neden": "tutar yanlış"}),
    ));
    assert_eq!(
        o.ilerle(&d, &mut k, SIMDI + 5),
        &ImzaDurumu::Reddedildi {
            neden: "tutar yanlış".into()
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
fn ozet_istek_kimligi_veya_tutar_tutmazsa_gecersiz() {
    let (d, a, id) = hazir("gecersiz");
    let mut k = SahteKanal::default();
    let ki = kisi("Kasiyer A");
    // Başka özet.
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &ki, NONCE, SIMDI).unwrap();
    let mut c = onay(&o.istek);
    c["ozet_sha256"] = json!("00".repeat(32));
    k.cevaplar.push_back(Some(c));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1).kod(), "gecersiz");
    // Başka istek kimliği.
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &ki, NONCE, SIMDI).unwrap();
    let mut c = onay(&o.istek);
    c["istek_kimligi"] = json!("baska-istek-kimligi-0000");
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
    // Kısa istek kimliği reddedilir.
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
fn cevap_cozumu_esnek() {
    // Düz alanlar ve eski uzak köprü adı.
    let c = KopruCevabi::jsondan(&json!({"tur":"uk_onay","nonce":"n1","ozet":"ABC",
        "eslesme_kimligi":"e","pc_pin":"p"}))
    .unwrap();
    assert!(matches!(c, KopruCevabi::Onay { ref ozet_sha256, .. } if ozet_sha256 == "abc"));
    assert!(matches!(
        KopruCevabi::jsondan(&json!({"tur":"onay_red@1","istek_kimligi":"n1"})).unwrap(),
        KopruCevabi::Red { .. }
    ));
    assert!(KopruCevabi::jsondan(&json!({"tur":"baska","istek_kimligi":"n1"})).is_err());
    assert!(KopruCevabi::jsondan(&json!({"tur":"onay@1"})).is_err());
}
