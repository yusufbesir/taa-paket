// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26] değiştiren: [Claude#taa-kasa-tk-ek3 2026-11-27]
//! Telefonla imza istemcisi testleri (TK Ek 3): sahte köprü kanalıyla onay, red, zaman aşımı, bilinmeyen alan
//! reddi, sayaç tekrarı, bitis daraltma, nonce eşleşmesi, özet uyuşmazlığı, görev ayrılığı, boyut sınırları ve
//! köprü yokken (yer tutucu boru) davranış. Tüm veriler anonim test verisidir.

use super::*;
use crate::gunsonu::{
    DovizSatiri, GunSonuGirdi, KasaAyar, NoktaGunSonu, Rol, SayimGirdi, SayimTuru,
};
use serde_json::json;
use std::collections::{BTreeMap, VecDeque};

const T: &str = "2026-11-26";
/// TK hizmetinin ürettiği nonce (sahte kanal).
const NONCE: &str = "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff";
const SIMDI: i64 = 1_800_000_000;
const BITIS: i64 = SIMDI + GECERLILIK_SN;

/// Geçerli biçimde ASN.1 DER ECDSA imzası (r ve s 32 bayt); kriptografik olarak anlamsız test değeri.
fn der() -> String {
    format!("30440220{}0220{}", "7a".repeat(32), "2b".repeat(32))
}

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

/// Sahte köprü: gönderilen istekleri saklar, kabulde hizmet nonce'u ve bitişi döner, sıradaki cevabı
/// yoklamada verir.
struct SahteKanal {
    giden: Vec<OnayIstegi>,
    cevaplar: VecDeque<Option<Value>>,
    hata: Option<KanalHatasi>,
    kabul: Value,
}

impl Default for SahteKanal {
    fn default() -> Self {
        SahteKanal {
            giden: vec![],
            cevaplar: VecDeque::new(),
            hata: None,
            kabul: json!({"nonce": NONCE, "bitis": BITIS}),
        }
    }
}

impl KopruKanali for SahteKanal {
    fn gonder(&mut self, istek: &OnayIstegi) -> Result<Value, KanalHatasi> {
        if let Some(h) = &self.hata {
            return Err(h.clone());
        }
        self.giden.push(istek.clone());
        Ok(self.kabul.clone())
    }
    fn yokla(&mut self, _nonce: &str) -> Result<Option<Value>, KanalHatasi> {
        if let Some(h) = &self.hata {
            return Err(h.clone());
        }
        Ok(self.cevaplar.pop_front().flatten())
    }
}

/// Ek 3 `onay@1`: yalnız sekiz alan.
fn onay(o: &TelefonImzaOturumu, sayac: u64) -> Value {
    json!({
        "tur": "onay",
        "surum": 1,
        "kapsam": o.istek.kapsam,
        "ozet": o.istek.ozet,
        "nonce": o.nonce,
        "bitis": o.bitis,
        "sayac": sayac,
        "imza": der(),
    })
}

fn red(nonce: &str, neden: &str) -> Value {
    json!({"tur": "onay_red", "surum": 1, "nonce": nonce, "neden": neden})
}

fn imza_sayisi(d: &KasaDefteri, id: &str) -> usize {
    d.imza_haritasi().unwrap().get(id).map_or(0, Vec::len)
}

fn gs_bul(d: &KasaDefteri, id: &str) -> GunSonuKaydi {
    d.gun_sonlari()
        .unwrap()
        .into_iter()
        .find(|s| s.id == id)
        .unwrap()
}

#[test]
fn ozet_v2_kisa_ayrinti_hashli_ve_tutar_degisince_degisir() {
    let (d, _, id) = hazir("ozet");
    let gs = gs_bul(&d, &id);
    let a = ayrinti_metni(&gs);
    assert!(
        a.starts_with("cekmece=c1\n"),
        "ayrıntı sözlük sırasında: {a}"
    );
    assert!(a.contains("ciro.nakit=20000\n") && a.contains("surum=1\n"));
    let m = ozet_metni(&gs);
    assert!(
        m.starts_with(&format!("kapsam={KAPSAM}\nsurum=2\nsube=SB1\n")),
        "{m}"
    );
    assert!(m.contains(&format!("ayrinti_sha256={}\n", ayrinti_sha256(&gs))));
    assert!(
        m.contains("z_toplam_kurus=30000\npos_toplam_kurus=10000\nnokta_adet=1\ndoviz_adet=1\n")
    );
    assert!(
        !m.contains("Kasiyer") && !a.contains("Kasiyer"),
        "kişi adı girmez"
    );
    let h = ozet_sha256(&gs);
    assert!(hex64_mi(&h));
    assert!(ozet_eslesir(&h, &gs) && ozet_eslesir(&ayrinti_sha256(&gs), &gs));
    let mut g2 = gs.clone();
    *g2.ciro.get_mut("nakit").unwrap() += 1;
    assert_ne!(
        h,
        ozet_sha256(&g2),
        "okunur kısımda olmayan bir kuruş fark da özeti değiştirir"
    );
    assert!(!ozet_eslesir(&h, &g2));
}

#[test]
fn ozet_metni_cok_nokta_cok_dovizde_1_kib_altinda() {
    let (d, _, id) = hazir("boyut");
    let mut gs = gs_bul(&d, &id);
    // Uç durum: 12 kasa noktası × 6 POS, 8 döviz, uzun ve çok baytlı metin alanları, en büyük tutarlar.
    gs.noktalar = (0..12)
        .map(|i| NoktaGunSonu {
            nokta: format!("kasa-noktasi-{i}"),
            z_toplam_kurus: crate::gunsonu::TUTAR_EN_COK,
            pos: (0..6)
                .map(|p| (format!("pos-terminali-{p}"), crate::gunsonu::TUTAR_EN_COK))
                .collect(),
        })
        .collect();
    gs.doviz = ["USD", "EUR", "GBP", "CHF", "RUB", "SAR", "JPY", "AED"]
        .iter()
        .map(|k| DovizSatiri {
            kod: (*k).into(),
            tutar: crate::gunsonu::TUTAR_EN_COK,
            kur_onbinde: crate::gunsonu::TUTAR_EN_COK,
        })
        .collect();
    gs.sube = "Ş".repeat(200);
    gs.cekmece = "ğ\n".repeat(100);
    gs.id = "İ".repeat(100);
    gs.nakit_sayilan_kurus = i64::MIN;
    assert!(
        ayrinti_metni(&gs).len() > OZET_METNI_EN_COK_BAYT,
        "sürüm 1 sınırı aşar"
    );
    let m = ozet_metni(&gs);
    assert!(m.len() <= 700, "sürüm 2: {} bayt", m.len());
    assert_eq!(m.lines().count(), 15);
    let sube = m.lines().find(|l| l.starts_with("sube=")).unwrap();
    assert!(sube.len() - 5 <= METIN_ALANI_EN_COK_BAYT && sube.ends_with('Ş'));
    let ist = OnayIstegi {
        kapsam: KAPSAM.into(),
        baslik: baslik(&gs),
        ozet: ozet_sha256(&gs),
        ozet_metni: m,
    };
    assert!(ist.bayt().unwrap().len() <= ISTEK_EN_COK_BAYT);
    let mut buyuk = ist.clone();
    buyuk.ozet_metni = "x".repeat(OZET_METNI_EN_COK_BAYT + 1);
    assert!(buyuk.bayt().unwrap_err().contains("özet metni"));
    let mut buyuk = ist;
    buyuk.baslik = "y".repeat(ISTEK_EN_COK_BAYT);
    assert!(buyuk.bayt().unwrap_err().contains("istek"));
}

#[test]
fn boruya_yalniz_dort_alan_gider_nonce_hizmetten_gelir_ve_onay_deftere_yazilir() {
    let (d, _, id) = hazir("onay");
    let mut k = SahteKanal::default();
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
    assert_eq!(o.durum, ImzaDurumu::Bekliyor);
    assert_eq!(
        (o.nonce.as_str(), o.bitis),
        (NONCE, BITIS),
        "nonce ve bitiş hizmetten"
    );
    assert_eq!((o.sira, o.rol.as_str()), (1, "kasa_sorumlusu"));
    let ist = k.giden[0].clone();
    let v = serde_json::to_value(&ist).unwrap();
    let mut adlar: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
    adlar.sort();
    assert_eq!(
        adlar,
        ["baslik", "kapsam", "ozet", "ozet_metni"],
        "nonce/bitis/pc_adi/pc_imza yok"
    );
    assert_eq!(ist.baslik, format!("Gün sonu · SB1 · {T}"));
    assert_eq!(ist.ozet, hex(&Sha256::digest(ist.ozet_metni.as_bytes())));
    assert!(
        serde_json::from_value::<OnayIstegi>(json!({"kapsam":"k","baslik":"b","ozet":"o",
        "ozet_metni":"m","nonce":"x"}))
        .is_err(),
        "istek biçimi de katı"
    );
    k.cevaplar.push_back(None);
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 10), &ImzaDurumu::Bekliyor);
    k.cevaplar.push_back(Some(onay(&o, 7)));
    let ImzaDurumu::Onaylandi(i) = o.ilerle(&d, &mut k, SIMDI + 20).clone() else {
        panic!("onaylanmadı: {:?}", o.durum)
    };
    assert_eq!(i.kanal, ImzaKanali::Telefon);
    assert_eq!(i.cihaz, None, "eşleşme kaydı yok: cihaz boş");
    assert_eq!(i.ozet.as_deref(), Some(ist.ozet.as_str()));
    let t = i.telefon.clone().unwrap();
    assert_eq!((t.nonce.as_str(), t.sayac, t.bitis), (NONCE, 7, BITIS));
    assert_eq!((t.imza, t.imza_dogrulama.as_str()), (der(), "yapilmadi"));
    assert_eq!((i.sira, i.kisi.as_str()), (1, "Kasiyer A"));
    assert_eq!(d.telefon_sayaci().unwrap(), Some(7));
    // Bitmiş oturum yeniden ilerletilince ikinci imza atılmaz.
    k.cevaplar.push_back(Some(onay(&o, 8)));
    o.ilerle(&d, &mut k, SIMDI + 21);
    assert_eq!(imza_sayisi(&d, &id), 1);
}

#[test]
fn hizmet_kabulu_gecersizse_istek_iptal() {
    let (d, _, id) = hazir("kabul");
    for (kabul, parca) in [
        (json!({"bitis": BITIS}), "nonce"),
        (json!({"nonce": "ABCD", "bitis": BITIS}), "nonce"),
        (
            json!({"nonce": NONCE.to_uppercase(), "bitis": BITIS}),
            "nonce",
        ),
        (json!({"nonce": NONCE}), "bitis"),
        (json!({"nonce": NONCE, "bitis": SIMDI}), "bitis"),
        (
            json!({"nonce": NONCE, "bitis": SIMDI + GECERLILIK_SN + SAAT_PAYI_SN + 1}),
            "bitis",
        ),
    ] {
        let mut k = SahteKanal {
            kabul,
            ..Default::default()
        };
        let o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
        let ImzaDurumu::Gecersiz { neden } = &o.durum else {
            panic!("{:?}", o.durum)
        };
        assert!(neden.contains(parca), "{neden}");
        assert!(o.durum.imzasiz_bitti());
    }
}

#[test]
fn bilinmeyen_ya_da_eksik_alan_red_sayilir() {
    let (d, _, id) = hazir("alan");
    // Eski Ek 2 alanları (cihaz, kisi, zaman, kisi_dogrulandi) ve herhangi bir fazladan alan → red.
    for fazla in ["cihaz", "kisi", "zaman", "kisi_dogrulandi", "pc_adi", "x"] {
        let mut k = SahteKanal::default();
        let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
        let mut c = onay(&o, 1);
        c[fazla] = json!("es-1");
        k.cevaplar.push_back(Some(c));
        let s = o.ilerle(&d, &mut k, SIMDI + 1).clone();
        assert_eq!(s.kod(), "gecersiz", "{fazla}");
        assert!(
            s.mesaj().contains("tanınmayan alan") && s.imzasiz_bitti(),
            "{fazla}: {}",
            s.mesaj()
        );
    }
    // Eksik alan, yanlış tür/sürüm, biçimsiz değerler.
    type Bozan = Box<dyn Fn(&mut Value)>;
    let bozanlar: Vec<(&str, Bozan)> = vec![
        (
            "eksik sayac",
            Box::new(|c: &mut Value| {
                c.as_object_mut().unwrap().remove("sayac");
            }),
        ),
        (
            "eksik imza",
            Box::new(|c: &mut Value| {
                c.as_object_mut().unwrap().remove("imza");
            }),
        ),
        (
            "tur onay@1",
            Box::new(|c: &mut Value| c["tur"] = json!("onay@1")),
        ),
        ("surum 2", Box::new(|c: &mut Value| c["surum"] = json!(2))),
        (
            "surum metin",
            Box::new(|c: &mut Value| c["surum"] = json!("1")),
        ),
        (
            "ozet büyük harf",
            Box::new(|c: &mut Value| c["ozet"] = json!(c["ozet"].as_str().unwrap().to_uppercase())),
        ),
        (
            "sayac eksi",
            Box::new(|c: &mut Value| c["sayac"] = json!(-1)),
        ),
        (
            "sayac metin",
            Box::new(|c: &mut Value| c["sayac"] = json!("5")),
        ),
        (
            "bitis metin",
            Box::new(|c: &mut Value| c["bitis"] = json!("1")),
        ),
        (
            "imza DER değil",
            Box::new(|c: &mut Value| c["imza"] = json!("abcd")),
        ),
        (
            "imza büyük harf",
            Box::new(|c: &mut Value| c["imza"] = json!(der().to_uppercase())),
        ),
    ];
    for (ad, boz) in bozanlar {
        let mut k = SahteKanal::default();
        let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
        let mut c = onay(&o, 1);
        boz(&mut c);
        k.cevaplar.push_back(Some(c));
        assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1).kod(), "gecersiz", "{ad}");
    }
    // Red cevabında da fazladan alan ya da yanlış biçim kabul edilmez (yine imzasız).
    let mut k = SahteKanal::default();
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
    let mut c = red(NONCE, "kullanici");
    c["imza"] = json!(der());
    k.cevaplar.push_back(Some(c));
    assert_eq!(
        o.ilerle(&d, &mut k, SIMDI + 1).kod(),
        "gecersiz",
        "red imzasızdır"
    );
    assert_eq!(imza_sayisi(&d, &id), 0);
    assert_eq!(
        d.telefon_sayaci().unwrap(),
        None,
        "geçersiz cevap sayacı ilerletmez"
    );
}

#[test]
fn sayac_tekrari_ve_geri_gidisi_reddedilir_kalici_saklanir() {
    let (d, _, id) = hazir("sayac");
    let mut k = SahteKanal::default();
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
    k.cevaplar.push_back(Some(onay(&o, 10)));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1).kod(), "onaylandi");
    assert_eq!(d.telefon_sayaci().unwrap(), Some(10));
    // Aynı defter klasörü yeniden açılınca da son sayaç bilinir (dosyada).
    let d2 = KasaDefteri::yeni(d.kok.clone());
    assert_eq!(d2.telefon_sayaci().unwrap(), Some(10));
    // İkinci imza: aynı sayaç (tekrar) ve küçük sayaç reddedilir; büyük sayaç kabul edilir.
    for (sayac, beklenen) in [(10, "gecersiz"), (3, "gecersiz"), (11, "onaylandi")] {
        let mut k = SahteKanal::default();
        let mut o =
            TelefonImzaOturumu::baslat(&d2, &mut k, &id, &kisi("Sorumlu B"), SIMDI).unwrap();
        k.cevaplar.push_back(Some(onay(&o, sayac)));
        let s = o.ilerle(&d2, &mut k, SIMDI + 1).clone();
        assert_eq!(s.kod(), beklenen, "sayac {sayac}");
        if beklenen == "gecersiz" {
            assert!(s.mesaj().contains("sayacı"), "{}", s.mesaj());
        }
    }
    assert_eq!(d2.telefon_sayaci().unwrap(), Some(11));
    assert!(
        d2.telefon_sayaci_yaz(11).is_err(),
        "sayaç geri/aynı yazılamaz"
    );
}

#[test]
fn bitis_daraltma_simdi_kucuk_bitis_kucuk_esit_istenen() {
    let (d, _, id) = hazir("bitis");
    // (onaydaki bitis, şimdi, beklenen)
    for (bitis, simdi, beklenen) in [
        (BITIS, SIMDI + 1, "onaylandi"),       // istenene eşit: kabul
        (BITIS + 1, SIMDI + 1, "gecersiz"),    // istenenden geç: red (genişletme yok)
        (SIMDI + 5, SIMDI + 5, "gecersiz"),    // şimdi = bitis: red
        (SIMDI + 5, SIMDI + 6, "gecersiz"),    // süresi dolmuş onay bitişi
        (SIMDI + 30, SIMDI + 10, "onaylandi"), // daraltılmış bitiş: kabul
    ] {
        let d = defter(&format!("bitis-{bitis}-{simdi}"));
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
        let id = d
            .gun_sonu_ekle(&a, &girdi(), &kisi("Kasiyer A"))
            .unwrap()
            .id;
        let mut k = SahteKanal::default();
        let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
        let mut c = onay(&o, 1);
        c["bitis"] = json!(bitis);
        k.cevaplar.push_back(Some(c));
        assert_eq!(
            o.ilerle(&d, &mut k, simdi).kod(),
            beklenen,
            "bitis {bitis} şimdi {simdi}"
        );
    }
    // Hizmetin bitişi geçince cevap gelmese de zaman aşımı (şimdi ≥ bitis).
    let mut k = SahteKanal::default();
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
    k.cevaplar.push_back(Some(onay(&o, 1)));
    assert_eq!(o.ilerle(&d, &mut k, BITIS), &ImzaDurumu::ZamanAsimi);
    assert_eq!(imza_sayisi(&d, &id), 0);
}

#[test]
fn nonce_kapsam_ozet_veya_tutar_tutmazsa_gecersiz() {
    let (d, a, id) = hazir("gecersiz");
    let ki = kisi("Kasiyer A");
    let baska = "ff".repeat(32);
    for (alan, deger) in [
        ("nonce", json!(baska)),
        ("ozet", json!("00".repeat(32))),
        ("kapsam", json!("taa_kasa_gun_sonu ")),
        ("kapsam", json!("baska_kapsam")),
    ] {
        let mut k = SahteKanal::default();
        let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &ki, SIMDI).unwrap();
        let mut c = onay(&o, 1);
        c[alan] = deger;
        k.cevaplar.push_back(Some(c));
        assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1).kod(), "gecersiz", "{alan}");
    }
    // Başka nonce'a ait red: geçersiz (yine imzasız).
    let mut k = SahteKanal::default();
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &ki, SIMDI).unwrap();
    k.cevaplar.push_back(Some(red(&baska, "kullanici")));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1).kod(), "gecersiz");
    // İstekten sonra gün sonu düzeltildi: eski özetle imza atılmaz.
    let mut k = SahteKanal::default();
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &ki, SIMDI).unwrap();
    let mut g = girdi();
    g.ciro.insert("nakit".into(), 20_100);
    d.gun_sonu_duzelt(&a, &id, &g, "nakit düzeltme", &yonetici())
        .unwrap();
    k.cevaplar.push_back(Some(onay(&o, 1)));
    let ImzaDurumu::Gecersiz { neden } = o.ilerle(&d, &mut k, SIMDI + 1).clone() else {
        panic!("{:?}", o.durum)
    };
    assert!(neden.contains("geçerli değil"), "{neden}");
    assert_eq!(imza_sayisi(&d, &id), 0);
}

#[test]
fn gorev_ayriligi_istekten_once_denetlenir() {
    let (d, _, id) = hazir("gorev");
    let mut k = SahteKanal::default();
    let personel = Kisi {
        rol: Rol::Personel,
        ..kisi("Kasiyer B")
    };
    let e = TelefonImzaOturumu::baslat(&d, &mut k, &id, &personel, SIMDI).unwrap_err();
    assert_eq!(e, crate::gunsonu::YETKISIZ_IMZA);
    assert!(k.giden.is_empty());
    d.imzala(&id, ImzaKanali::Ekran, None, &kisi("Kasiyer A"))
        .unwrap();
    let e = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("kasiyer a"), SIMDI).unwrap_err();
    assert!(e.contains("aynı kişi"), "{e}");
    assert!(k.giden.is_empty());
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Sorumlu B"), SIMDI).unwrap();
    assert_eq!((o.sira, o.rol.as_str()), (2, "magaza_sorumlusu"));
    k.cevaplar.push_back(Some(onay(&o, 1)));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 3).kod(), "onaylandi");
    assert!(
        TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Biri C"), SIMDI)
            .unwrap_err()
            .contains("iki imza")
    );
    // Telefon kanalı TK kanıtı olmadan doğrudan imzalanamaz.
    let (d, _, id) = hazir("gorev-kanit");
    assert!(d
        .imzala(&id, ImzaKanali::Telefon, Some("es-1"), &kisi("Kasiyer A"))
        .unwrap_err()
        .contains("TK kanıtı"));
}

#[test]
fn kopru_yokken_kanal_yok_ve_501_mesaji() {
    let (d, _, id) = hazir("yok");
    let mut b = BoruKanali::default();
    assert_eq!(b.boru, BORU_ADI);
    let mut o = TelefonImzaOturumu::baslat(&d, &mut b, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
    assert_eq!(
        o.durum,
        ImzaDurumu::KanalYok {
            neden: KOPRU_YOK_MESAJI.into()
        }
    );
    assert_eq!(o.ilerle(&d, &mut b, SIMDI + 1).kod(), "kanal_yok");
    assert_eq!(imza_sayisi(&d, &id), 0);
    // Geçici kanal hatası beklemeye devam eder, süre dolunca zaman aşımı.
    let mut k = SahteKanal::default();
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
    k.hata = Some(KanalHatasi::Hata("boru meşgul".into()));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1), &ImzaDurumu::Bekliyor);
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 61), &ImzaDurumu::ZamanAsimi);
}

#[test]
fn der_denetimi() {
    let b = |s: &str| super::hex_coz(s).unwrap();
    assert!(der_ecdsa_mi(&b(&der())));
    assert!(der_ecdsa_mi(&b("3006020101020101")));
    // r başında gerekli 00 (yüksek bit) kabul, gereksiz 00 ve eksi değer red.
    assert!(der_ecdsa_mi(&b("30070202008002 0101"
        .replace(' ', "")
        .as_str())));
    assert!(!der_ecdsa_mi(&b("300702020001020101")));
    assert!(!der_ecdsa_mi(&b("3006020180020101")));
    assert!(!der_ecdsa_mi(&b("3007020101020101ff")), "fazladan bayt");
    assert!(!der_ecdsa_mi(&b("3106020101020101")));
    assert!(!der_ecdsa_mi(&[]));
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
        let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
        k.cevaplar.push_back(Some(red(NONCE, kod)));
        let s = o.ilerle(&d, &mut k, SIMDI + 2).clone();
        assert_eq!(
            s,
            ImzaDurumu::Reddedildi {
                neden: beklenen.clone()
            },
            "{kod}"
        );
        assert!(s.imzasiz_bitti(), "{kod}: sessizce geçmez, imzasız biter");
        assert_eq!(s.red_nedeni(), Some(kod));
        let m = s.mesaj();
        assert!(m.contains(parca) && m.contains("imzasız"), "{kod}: {m}");
        k.cevaplar.push_back(Some(onay(&o, 1)));
        o.ilerle(&d, &mut k, SIMDI + 3);
        assert_eq!(imza_sayisi(&d, &id), 0, "{kod}");
    }
}

#[test]
fn baslik_en_cok_80_karakter() {
    let (d, _, id) = hazir("baslik");
    let mut gs = gs_bul(&d, &id);
    assert_eq!(baslik(&gs), "Gün sonu · SB1 · 2026-11-26");
    gs.sube = "Ş".repeat(100);
    let b = baslik(&gs);
    assert_eq!(b.chars().count(), BASLIK_EN_COK);
    assert!(b.starts_with("Gün sonu · ŞŞ") && b.ends_with('…'));
}

/// Sahte TAA Merkez cihaz kaydı.
struct SahteKayit(CihazDurumu);
impl CihazKaydi for SahteKayit {
    fn denetle(&self, _cihaz: &str, _kisi: &str) -> CihazDurumu {
        self.0
    }
}

/// Sahte eşleşme kaydı.
struct SahteEslesme(Option<&'static str>, ImzaDenetimi);
impl EslesmeKaydi for SahteEslesme {
    fn cihaz(&self) -> Option<String> {
        self.0.map(str::to_string)
    }
    fn imza_dogrula(&self, _onay: &OnayCevabi) -> ImzaDenetimi {
        self.1
    }
}

#[test]
fn eslesme_ve_merkez_kaydi_cihaz_bagini_ve_imzayi_denetler() {
    for (durum, beklenen) in [
        (CihazDurumu::Iptal, "gecersiz"),
        (CihazDurumu::BaskaKisi, "gecersiz"),
        (CihazDurumu::YetkisizKisi, "gecersiz"),
        (CihazDurumu::Gecerli, "onaylandi"),
    ] {
        let (d, _, id) = hazir(&format!("kayit-{}", durum.kod()));
        let mut k = SahteKanal::default();
        let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
        k.cevaplar.push_back(Some(onay(&o, 1)));
        let es = SahteEslesme(Some("es-1"), ImzaDenetimi::Dogrulandi);
        let s = o
            .ilerle_denetle(&d, &mut k, &SahteKayit(durum), &es, SIMDI + 1)
            .clone();
        assert_eq!(s.kod(), beklenen, "{durum:?}");
        if let ImzaDurumu::Onaylandi(i) = s {
            assert_eq!(i.cihaz.as_deref(), Some("es-1"));
            assert_eq!(i.cihaz_dogrulama.as_deref(), Some("gecerli"));
            assert_eq!(i.telefon.unwrap().imza_dogrulama, "dogrulandi");
        }
    }
    // İmza eşleşmiş telefonun anahtarıyla tutmazsa red.
    let (d, _, id) = hazir("imza-gecersiz");
    let mut k = SahteKanal::default();
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
    k.cevaplar.push_back(Some(onay(&o, 1)));
    let es = SahteEslesme(Some("es-1"), ImzaDenetimi::Gecersiz);
    let s = o
        .ilerle_denetle(&d, &mut k, &MerkezKaydiYok, &es, SIMDI + 1)
        .clone();
    assert_eq!(s.kod(), "gecersiz");
    assert_eq!(d.telefon_sayaci().unwrap(), None);
}

#[test]
fn merkez_paketteki_telefon_imzasini_ozet_kanit_ve_cihazla_denetler() {
    use crate::merkez::telefon_imzalarini_denetle;
    let (d, a, id) = hazir("merkez-denetim");
    let mut k = SahteKanal::default();
    let mut o = TelefonImzaOturumu::baslat(&d, &mut k, &id, &kisi("Kasiyer A"), SIMDI).unwrap();
    k.cevaplar.push_back(Some(onay(&o, 1)));
    assert_eq!(o.ilerle(&d, &mut k, SIMDI + 1).kod(), "onaylandi");
    d.ikinci_imza_gerekcesi(&id, "ikinci kişi yok", &kisi("Kasiyer A"))
        .unwrap();
    let p = d.gonderim_paketi(&a, &id).unwrap();
    assert_eq!(p["imzalar"][0]["ozet"], o.istek.ozet);
    assert_eq!(p["imzalar"][0]["telefon"]["nonce"], NONCE);
    assert_eq!(p["imzalar"][0]["telefon"]["sayac"], 1);
    // Eşleşme kaydı yok: cihaz boş → sorun değil, doğrulanamadı.
    assert_eq!(telefon_imzalarini_denetle(&p, &MerkezKaydiYok), (vec![], 1));
    // Cihazlı paket: merkez kaydı sonuçları.
    let mut c = p.clone();
    c["imzalar"][0]["cihaz"] = json!("es-1");
    assert_eq!(
        telefon_imzalarini_denetle(&c, &SahteKayit(CihazDurumu::Gecerli)),
        (vec![], 0)
    );
    let (s, _) = telefon_imzalarini_denetle(&c, &SahteKayit(CihazDurumu::Iptal));
    assert!(s[0].contains("iptal"), "{s:?}");
    // Kanıtsız telefon imzası sorun.
    let mut kanitsiz = p.clone();
    kanitsiz["imzalar"][0]
        .as_object_mut()
        .unwrap()
        .remove("telefon");
    let (s, _) = telefon_imzalarini_denetle(&kanitsiz, &MerkezKaydiYok);
    assert!(s[0].contains("kanıtı"), "{s:?}");
    // Rol merkezde de denetlenir.
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
    let (s, _) = telefon_imzalarini_denetle(&bozuk, &MerkezKaydiYok);
    assert!(s[0].contains("özet"), "{s:?}");
    // Eski (sürüm 1) özetle imzalı kayıt hâlâ tutar.
    let mut eski = p.clone();
    let gs: GunSonuKaydi = serde_json::from_value(p["gun_sonu"].clone()).unwrap();
    eski["imzalar"][0]["ozet"] = json!(ayrinti_sha256(&gs));
    assert_eq!(
        telefon_imzalarini_denetle(&eski, &MerkezKaydiYok).0,
        Vec::<String>::new()
    );
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
    assert!(v.len() >= 4);
    for x in v {
        let ad = x["ad"].as_str().unwrap();
        let gs: crate::gunsonu::GunSonuKaydi =
            serde_json::from_value(x["gun_sonu"].clone()).unwrap();
        // Sürüm 1 (eski vektör, korunur): ayrıntı metni.
        assert_eq!(
            ayrinti_metni(&gs),
            x["beklenen_metin"].as_str().unwrap(),
            "{ad}: v1 metin"
        );
        assert_eq!(
            ayrinti_sha256(&gs),
            x["beklenen_sha256"].as_str().unwrap(),
            "{ad}: v1 sha256"
        );
        // Sürüm 2: istekteki ozet_metni / ozet.
        let m = ozet_metni(&gs);
        assert_eq!(
            m,
            x["beklenen_metin_v2"].as_str().unwrap(),
            "{ad}: v2 metin"
        );
        assert_eq!(
            ozet_sha256(&gs),
            x["beklenen_sha256_v2"].as_str().unwrap(),
            "{ad}: v2 sha256"
        );
        assert!(m.len() <= OZET_METNI_EN_COK_BAYT, "{ad}");
    }
    // v1 değerleri PR #114'teki sabitlerle aynı kalır.
    assert_eq!(
        v[0]["beklenen_sha256"],
        "ab1b4529d01f2ba5430d6828e67ad3b7f1bd0b6c9b8029b0ca79f3ca16eae0e0"
    );
    assert!(v[3]["beklenen_metin"].as_str().unwrap().len() > OZET_METNI_EN_COK_BAYT);
    assert_eq!(OZET_SURUMU, 2);
    assert_eq!(KAPSAM, "taa_kasa_gun_sonu");
}

#[test]
fn sube_kodu_buyuk_kucuk_harf_duyarsiz_eslesir() {
    use crate::merkez::{
        matris, sube_anahtari, sube_esit, subeleri_tekille, tek_imza_uyarisi_uygula, GelenPaket,
    };
    assert!(sube_esit("MRK01", "mrk01") && sube_esit(" Mrk01 ", "mRK01"));
    assert!(!sube_esit("MRK01", "MRK02"));
    assert_eq!(sube_anahtari("İST01"), "ist01");
    assert_eq!(
        subeleri_tekille([
            "MRK01".to_string(),
            "mrk01".into(),
            "SB2".into(),
            " ".into()
        ]),
        vec!["MRK01".to_string(), "SB2".into()],
        "ilk yazım kalır, boş atılır"
    );
    // Paket küçük harf (sunucu biçimi), matris satırı büyük harf: aynı hücreye düşer.
    let p = |sube: &str, t: &str| GelenPaket {
        sube: sube.into(),
        tarih: t.into(),
        alinma_sn: 0,
        ikinci_imza_atmadi: true,
        ..Default::default()
    };
    let gelen = vec![
        p("mrk01", "2026-11-24"),
        p("Mrk01", "2026-11-25"),
        p("MRK01", "2026-11-26"),
    ];
    let subeler = vec!["MRK01".to_string()];
    let tarihler = vec!["2026-11-26".to_string()];
    let mut m = matris(&gelen, &subeler, &tarihler, "23:00", 180, 0);
    assert_ne!(
        m[0].hucreler[0].durum,
        matris(&[], &subeler, &tarihler, "23:00", 180, 0)[0].hucreler[0].durum
    );
    tek_imza_uyarisi_uygula(&mut m, &gelen, "2026-11-26", 14, 3);
    assert_eq!((m[0].tek_imza_gun_sayisi, m[0].tek_imza_uyarisi), (3, true));
}
