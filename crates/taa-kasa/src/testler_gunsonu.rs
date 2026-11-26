// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! Gün sonu çekirdeği testleri: görev ayrılığı, silmeme, düzeltme izi, karşılaştırma, kuyruk ve matris.

use super::*;
use crate::merkez::{self, GelenPaket, KlasorTasiyici, Kuyruk, MatrisDurumu};

fn defter(ad: &str) -> KasaDefteri {
    let k = taa_dosya::gecici_klasor()
        .unwrap()
        .join(format!("kasa-{ad}"));
    KasaDefteri::yeni(k)
}

fn ayse() -> Kisi {
    Kisi {
        ad: "Ayşe".into(),
        rol: Rol::Personel,
        dogrulandi: false,
    }
}
fn mehmet_mudur() -> Kisi {
    Kisi {
        ad: "Mehmet".into(),
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

fn esenyurt() -> KasaAyar {
    // 2 kasa noktası, 1 çekmece.
    let mut a = KasaAyar {
        sube: "ESY".into(),
        ..KasaAyar::default()
    };
    let mut n2 = a.cekmeceler[0].noktalar[0].clone();
    n2.id = "k2".into();
    n2.ad = "Kasa noktası 2".into();
    a.cekmeceler[0].noktalar.push(n2);
    a.kasa_limiti_kurus = Some(250_000);
    a
}

fn adet(tl: &[(i64, u32)]) -> BTreeMap<i64, u32> {
    tl.iter().map(|(d, a)| (*d, *a)).collect()
}

const T: &str = "2026-11-26";

fn acilis(d: &KasaDefteri, a: &KasaAyar) -> SayimKaydi {
    d.sayim_ekle(
        a,
        SayimTuru::Acilis,
        &SayimGirdi {
            tarih: T.into(),
            cekmece: "c1".into(),
            adetler: adet(&[(10_000, 5)]),
            ..Default::default()
        },
        &ayse(),
    )
    .unwrap()
}

fn gun_sonu_girdi() -> GunSonuGirdi {
    // Açılış 500 TL + NAKİT TL 1.000 TL = 1.500 TL beklenen.
    GunSonuGirdi {
        tarih: T.into(),
        cekmece: "c1".into(),
        nakit_adetler: adet(&[(20_000, 7), (10_000, 1)]), // 1.500 TL
        noktalar: vec![
            NoktaGunSonu {
                nokta: "k1".into(),
                z_toplam_kurus: 200_000,
                pos: BTreeMap::from([("ziraat".into(), 60_000), ("iyzico".into(), 40_000)]),
            },
            NoktaGunSonu {
                nokta: "k2".into(),
                z_toplam_kurus: 50_000,
                pos: BTreeMap::from([("ziraat".into(), 0), ("iyzico".into(), 0)]),
            },
        ],
        nebim_toplam_kurus: 250_000,
        ciro: BTreeMap::from([
            ("nakit".into(), 100_000),
            ("ziraat_pos".into(), 60_000),
            ("iyzico_pos".into(), 40_000),
            ("havale".into(), 10_000),
        ]),
        // 100 USD × 40 TL = 4.000 TL? Hayır: 10 USD × 40,0000 = 400 TL.
        doviz: vec![DovizSatiri {
            kod: "USD".into(),
            tutar: 1_000,
            kur_onbinde: 400_000,
        }],
        devreden_kurus: 50_000,
        kontrol: vec![true; 7],
        aciklamalar: BTreeMap::new(),
        not: String::new(),
    }
}

#[test]
fn acilis_iki_kez_olmaz_ve_onceki_kapanisla_karsilastirilir() {
    let d = defter("acilis");
    let a = esenyurt();
    let s = acilis(&d, &a);
    assert_eq!(s.beklenen_kurus, None, "ilk gün önceki kapanış yok");
    assert!(d
        .sayim_ekle(
            &a,
            SayimTuru::Acilis,
            &SayimGirdi {
                tarih: T.into(),
                cekmece: "c1".into(),
                ..Default::default()
            },
            &ayse()
        )
        .is_err());
    d.gun_sonu_ekle(&a, &gun_sonu_girdi(), &ayse()).unwrap();
    // Ertesi gün: beklenen = devreden 500 TL; 400 TL sayılırsa not zorunlu.
    let g = SayimGirdi {
        tarih: "2026-11-27".into(),
        cekmece: "c1".into(),
        adetler: adet(&[(10_000, 4)]),
        ..Default::default()
    };
    assert!(d
        .sayim_ekle(&a, SayimTuru::Acilis, &g, &ayse())
        .unwrap_err()
        .contains("fark"));
    let s2 = d
        .sayim_ekle(
            &a,
            SayimTuru::Acilis,
            &SayimGirdi {
                not: "100 TL bankaya".into(),
                ..g
            },
            &ayse(),
        )
        .unwrap();
    assert_eq!(s2.beklenen_kurus, Some(50_000));
    assert_eq!(s2.fark_kurus, Some(-10_000));
}

#[test]
fn ara_sayim_ve_devir_iki_kisi_ister_ve_limit_uyarir() {
    let d = defter("ara");
    let a = esenyurt();
    acilis(&d, &a);
    let g = SayimGirdi {
        tarih: T.into(),
        cekmece: "c1".into(),
        adetler: adet(&[(20_000, 15)]),
        not: "satış".into(),
        ..Default::default()
    };
    assert!(
        d.sayim_ekle(&a, SayimTuru::Ara, &g, &ayse()).is_err(),
        "ikinci kişi yok"
    );
    let ayni = SayimGirdi {
        ikinci_kisi: Some(" ayşe ".into()),
        ..g.clone()
    };
    assert!(
        d.sayim_ekle(&a, SayimTuru::Ara, &ayni, &ayse()).is_err(),
        "aynı kişi"
    );
    let s = d
        .sayim_ekle(
            &a,
            SayimTuru::Ara,
            &SayimGirdi {
                ikinci_kisi: Some("Mehmet".into()),
                ..g.clone()
            },
            &ayse(),
        )
        .unwrap();
    assert!(s.limit_asildi, "3.000 TL > 2.500 TL limit");
    assert!(d
        .sayim_ekle(&a, SayimTuru::Devir, &g, &ayse())
        .unwrap_err()
        .contains("devralan"));
    let dv = d
        .sayim_ekle(
            &a,
            SayimTuru::Devir,
            &SayimGirdi {
                ikinci_kisi: Some("Zeynep".into()),
                ..g
            },
            &ayse(),
        )
        .unwrap();
    assert!(dv.id.starts_with("DV-20261126-c1-"));
    assert!(d.gun_durumu(&a, T, "c1").unwrap().limit_asildi);
}

#[test]
fn masraf_esigi_ve_giren_onaylayamaz() {
    let d = defter("masraf");
    let a = esenyurt();
    let g = MasrafGirdi {
        tarih: T.into(),
        cekmece: "c1".into(),
        tur: "Kargo".into(),
        tutar_kurus: 150_000,
        aciklama: "koli".into(),
        foto: None,
        not: String::new(),
    };
    let kucuk = d.masraf_ekle(&a, &g, &ayse()).unwrap();
    assert_eq!(kucuk.onay_duzeyi, OnayDuzeyi::Mudur);
    let buyuk = d
        .masraf_ekle(
            &a,
            &MasrafGirdi {
                tutar_kurus: 200_000,
                ..g.clone()
            },
            &ayse(),
        )
        .unwrap();
    assert_eq!(
        buyuk.onay_duzeyi,
        OnayDuzeyi::Merkez,
        "2.000 TL ve üstü merkez"
    );
    assert!(d
        .masraf_ekle(
            &a,
            &MasrafGirdi {
                tur: "Uydurma".into(),
                ..g.clone()
            },
            &ayse()
        )
        .is_err());
    assert!(d
        .masraf_ekle(
            &a,
            &MasrafGirdi {
                foto: Some("../x.jpg".into()),
                ..g.clone()
            },
            &ayse()
        )
        .is_err());
    // Giren onaylayamaz (müdür olsa bile).
    let m = mehmet_mudur();
    let mm = d.masraf_ekle(&a, &g, &m).unwrap();
    assert!(d
        .masraf_karar(&mm.id, true, "", &m)
        .unwrap_err()
        .contains("giren"));
    // Personel müdür düzeyini onaylayamaz; müdür onaylar.
    assert!(d
        .masraf_karar(
            &kucuk.id,
            true,
            "",
            &Kisi {
                ad: "Zeynep".into(),
                ..ayse()
            }
        )
        .is_err());
    d.masraf_karar(&kucuk.id, true, "", &m).unwrap();
    assert!(
        d.masraf_karar(&kucuk.id, true, "", &m).is_err(),
        "ikinci karar yok"
    );
    // Merkez düzeyini müdür onaylayamaz, yönetici onaylar; ret nedeni zorunlu.
    assert!(d.masraf_karar(&buyuk.id, true, "", &m).is_err());
    assert!(d.masraf_karar(&buyuk.id, false, "", &yonetici()).is_err());
    d.masraf_karar(&buyuk.id, false, "fatura yok", &yonetici())
        .unwrap();
    let l = d.masraflar().unwrap();
    assert_eq!(
        l.iter().find(|x| x.kayit.id == kucuk.id).unwrap().durum,
        MasrafDurumu::Onaylandi
    );
    assert_eq!(
        l.iter().find(|x| x.kayit.id == buyuk.id).unwrap().durum,
        MasrafDurumu::Reddedildi
    );
    // Reddedilen masraf gün sonu beklentisine girmez: 1.500 + 1.500 (bekleyen mm).
    assert_eq!(d.masraf_toplami(T, "c1").unwrap(), 300_000);
}

#[test]
fn gun_sonu_karsilastirma_ve_not() {
    let d = defter("gs");
    let a = esenyurt();
    acilis(&d, &a);
    let gs = d.gun_sonu_ekle(&a, &gun_sonu_girdi(), &ayse());
    // Z (2.500) ↔ ciro (2.100 + 400 döviz = 2.500), nakit 1.500 ↔ 500+1.000: farksız.
    let gs = gs.unwrap();
    assert_eq!(gs.karsilastirma.ciro_toplam_kurus, 250_000);
    assert!(!gs.karsilastirma.fark_var, "{:?}", gs.karsilastirma);
    assert_eq!(
        gs.karsilastirma.satirlar.len(),
        5,
        "nakit, Z, Ziraat, İyzico, Nebim"
    );
    assert!(
        d.gun_sonu_ekle(&a, &gun_sonu_girdi(), &ayse()).is_err(),
        "ikinci gün sonu yok"
    );

    // Fark: Z raporu eksik girilirse "kasa ile ciro uyuşmuyor", not zorunlu.
    let d2 = defter("gs2");
    acilis(&d2, &a);
    let mut g = gun_sonu_girdi();
    g.noktalar[1].z_toplam_kurus = 40_000;
    assert!(d2
        .gun_sonu_ekle(&a, &g, &ayse())
        .unwrap_err()
        .contains("not"));
    g.not = "ikinci Z raporu yeniden alınacak".into();
    let k = d2.gun_sonu_ekle(&a, &g, &ayse()).unwrap().karsilastirma;
    assert!(k.fark_var && k.kasa_ciro_uyusmuyor);
    assert_eq!(k.satirlar[1].fark_kurus, 10_000);

    // Tanımsız POS, bilinmeyen ödeme türü, kursuz döviz reddedilir.
    let d3 = defter("gs3");
    let mut g = gun_sonu_girdi();
    g.noktalar[0].pos.insert("garanti".into(), 1);
    assert!(d3.gun_sonu_ekle(&a, &g, &ayse()).is_err());
    let mut g = gun_sonu_girdi();
    g.ciro.insert("kripto".into(), 1);
    assert!(d3.gun_sonu_ekle(&a, &g, &ayse()).is_err());
    let mut g = gun_sonu_girdi();
    g.doviz[0].kur_onbinde = 0;
    assert!(d3.gun_sonu_ekle(&a, &g, &ayse()).is_err());
}

#[test]
fn doviz_yuvarlama() {
    let d = DovizSatiri {
        kod: "EUR".into(),
        tutar: 1,
        kur_onbinde: 455_555,
    };
    assert_eq!(d.tl_kurus(), 46); // 0,01 € × 45,5555 = 0,455555 TL → 46 kuruş
    let d = DovizSatiri {
        kod: "EUR".into(),
        tutar: 10_000,
        kur_onbinde: 455_555,
    };
    assert_eq!(d.tl_kurus(), 455_555);
}

#[test]
fn onay_giren_olamaz_ve_gonderim_kosullari() {
    let d = defter("onay");
    let a = esenyurt();
    acilis(&d, &a);
    let mut g = gun_sonu_girdi();
    g.kontrol[3] = false;
    let gs = d.gun_sonu_ekle(&a, &g, &ayse()).unwrap();
    let du = d.gun_durumu(&a, T, "c1").unwrap();
    assert_eq!(du.asama, "kontrol");
    assert!(!du.imza_istegi, "liste eksikken imza isteği çıkmaz");
    // Liste eksik: imza da onay da alınmaz, gönderilmez.
    assert!(d
        .imzala(&gs.id, ImzaKanali::Ekran, None, &ayse())
        .unwrap_err()
        .contains("kapanış"));
    assert!(d
        .onayla(&gs.id, OnayTuru::IkinciKisi, &mehmet_mudur())
        .is_err());
    assert!(d.gonderim_paketi(&a, &gs.id).unwrap_err().contains("7"));
    // Ertesi gün gözden geçirmesi müdürün.
    assert!(d
        .onayla(
            &gs.id,
            OnayTuru::ErtesiGun,
            &Kisi {
                ad: "Can".into(),
                ..ayse()
            }
        )
        .is_err());
    d.onayla(&gs.id, OnayTuru::ErtesiGun, &mehmet_mudur())
        .unwrap();
    let form = KasaDefteri::gunluk_form(&d.gun_durumu(&a, T, "c1").unwrap());
    assert_eq!(form.len(), 8);
    assert!(form[0].1 && !form[5].1 && !form[6].1 && !form[7].1);
}

#[test]
fn imza_kurallari_en_az_tek_imza_ve_ikinci_imza_atmadi() {
    let d = defter("imza");
    let a = esenyurt();
    acilis(&d, &a);
    let gs = d.gun_sonu_ekle(&a, &gun_sonu_girdi(), &ayse()).unwrap();
    let du = d.gun_durumu(&a, T, "c1").unwrap();
    assert_eq!(du.asama, "imza");
    assert!(du.imza_istegi);
    assert_eq!(du.ikinci_imza_durumu, IKINCI_IMZA_ATMADI);
    // İmzasız gönderilemez.
    assert!(d
        .gonderim_paketi(&a, &gs.id)
        .unwrap_err()
        .contains("en az bir imza"));
    // Telefonla imzada cihaz kimliği zorunlu.
    assert!(d
        .imzala(&gs.id, ImzaKanali::Telefon, None, &ayse())
        .is_err());
    // Giren birinci imzayı (kasa sorumlusu) atabilir.
    let i1 = d
        .imzala(&gs.id, ImzaKanali::Telefon, Some("tel-1"), &ayse())
        .unwrap();
    assert_eq!((i1.sira, i1.rol.as_str()), (1, "kasa_sorumlusu"));
    assert!(
        d.imzala(&gs.id, ImzaKanali::Ekran, None, &ayse()).is_err(),
        "aynı kişi iki kez imzalayamaz"
    );
    // Tek imzayla gönderim kısa gerekçe ister (karar 2026-11-26); ikinci imza "imza atmadı" yazar.
    assert!(d
        .gonderim_paketi(&a, &gs.id)
        .unwrap_err()
        .contains("gerekçe"));
    assert!(
        d.ikinci_imza_gerekcesi(&gs.id, "yok", &ayse()).is_err(),
        "çok kısa"
    );
    // Tutanak ekleri ilk imzadan sonra kilitli.
    assert!(d
        .tutanak_kaydet(&gs.id, None, Some("sonradan"), &ayse())
        .unwrap_err()
        .contains("kilitli"));
    d.ikinci_imza_gerekcesi(&gs.id, "  ikinci   kişi izinli ", &ayse())
        .unwrap();
    let du = d.gun_durumu(&a, T, "c1").unwrap();
    assert_eq!(
        du.ikinci_imza_gerekcesi.as_deref(),
        Some("ikinci kişi izinli")
    );
    let p = d.gonderim_paketi(&a, &gs.id).unwrap();
    assert_eq!(p["ikinci_imza_gerekcesi"], "ikinci kişi izinli");
    assert_eq!(p["imzalar"][1]["gerekce"], "ikinci kişi izinli");
    assert_eq!(
        p["cift_dogrulama"]["imzalar"][1]["gerekce"],
        "ikinci kişi izinli"
    );
    let g = d.gerekce_haritasi().unwrap();
    assert_eq!(g[&gs.id], "ikinci kişi izinli");
    assert_eq!(p["imzalar"][0]["durum"], "imzaladi");
    assert_eq!(p["imzalar"][0]["cihaz"], "tel-1");
    assert_eq!(p["imzalar"][1]["durum"], "imza_atmadi");
    assert_eq!(p["ikinci_imza_durumu"], IKINCI_IMZA_ATMADI);
    assert_eq!(p["cift_dogrulama"]["imzalar"][1]["imzalamadi"], true);
    assert_eq!(p["cift_dogrulama"]["kalemler"].as_array().unwrap().len(), 8);
    assert_eq!(d.gun_durumu(&a, T, "c1").unwrap().asama, "gonder");
    // İkinci imza: giren olamaz (müdür olarak bile); başka kişi atar.
    assert!(d
        .onayla(
            &gs.id,
            OnayTuru::IkinciKisi,
            &Kisi {
                ad: "AYŞE".into(),
                ..mehmet_mudur()
            }
        )
        .is_err());
    let i2 = d
        .imzala(&gs.id, ImzaKanali::Telefon, Some("tel-2"), &mehmet_mudur())
        .unwrap();
    assert_eq!((i2.sira, i2.rol.as_str()), (2, "magaza_sorumlusu"));
    let du = d.gun_durumu(&a, T, "c1").unwrap();
    assert!(!du.imza_istegi, "iki imza dolunca istek kapanır");
    assert_eq!(du.ikinci_onay.as_deref(), Some(mehmet_mudur().ad.as_str()));
    assert!(du.ikinci_imza_durumu.starts_with("imzaladı"));
    assert!(
        du.ikinci_imza_gerekcesi.is_none(),
        "ikinci imza atılınca gerekçe görünmez"
    );
    assert!(d.gerekce_haritasi().unwrap().is_empty());
    assert!(d
        .imzala(&gs.id, ImzaKanali::Ekran, None, &yonetici())
        .is_err());
    d.gonderim_kaydet(&gs.id, "x.json", false).unwrap();
    assert_eq!(d.gun_durumu(&a, T, "c1").unwrap().asama, "tamam");
}

#[test]
fn imza_tek_imzayla_gonderildiyse_sonradan_eklenmez() {
    let d = defter("imza2");
    let a = esenyurt();
    acilis(&d, &a);
    let gs = d.gun_sonu_ekle(&a, &gun_sonu_girdi(), &ayse()).unwrap();
    d.imzala(&gs.id, ImzaKanali::Ekran, None, &ayse()).unwrap();
    d.gonderim_kaydet(&gs.id, "x.json", false).unwrap();
    assert!(d
        .imzala(&gs.id, ImzaKanali::Ekran, None, &mehmet_mudur())
        .unwrap_err()
        .contains("gönderildi"));
}

#[test]
fn whatsapp_ozeti_kayittan_ve_canli_girdiden() {
    let d = defter("wa");
    let mut a = esenyurt();
    a.cekmeceler[0].ad = "Merkez".into();
    acilis(&d, &a);
    let bos = d.whatsapp_ozeti(&a, T, None).unwrap();
    assert!(bos.starts_with("MERKEZ MAĞAZA "));
    assert!(bos.ends_with("MAĞAZA GENEL TOPLAM: 0 TL"));
    let g = gun_sonu_girdi();
    let canli = d.whatsapp_ozeti(&a, T, Some(&g)).unwrap();
    let gs = d.gun_sonu_ekle(&a, &g, &ayse()).unwrap();
    let kayit = d.whatsapp_ozeti(&a, T, None).unwrap();
    assert_eq!(canli, kayit, "canlı özet kayıtla aynı");
    assert!(kayit.contains(&format!(
        "GENEL TOPLAM: {}",
        taa_kasa_tl(gs.karsilastirma.ciro_toplam_kurus)
    )));
}

fn taa_kasa_tl(k: i64) -> String {
    crate::whatsapp::tl(k)
}

#[test]
fn iptal_yalniz_yonetici_silmez_duzeltme_iz_birakir() {
    let d = defter("iptal");
    let a = esenyurt();
    acilis(&d, &a);
    let gs = d.gun_sonu_ekle(&a, &gun_sonu_girdi(), &ayse()).unwrap();
    assert!(d
        .iptal(&gs.id, "yanlış", &mehmet_mudur())
        .unwrap_err()
        .contains("yönetici"));
    assert!(d.iptal(&gs.id, " ", &yonetici()).is_err());
    let satir_once = taa_dosya::satirlari_oku(&d.gun_yolu()).unwrap().len();
    // Geçersiz düzeltme eski kaydı iptal etmez.
    let mut kotu = gun_sonu_girdi();
    kotu.ciro.insert("kripto".into(), 1);
    assert!(d
        .gun_sonu_duzelt(&a, &gs.id, &kotu, "z yanlış", &yonetici())
        .is_err());
    assert_eq!(d.gun_sonlari().unwrap().len(), 1);
    let mut g = gun_sonu_girdi();
    g.nebim_toplam_kurus = 251_000;
    g.not = "Nebim raporu yeniden alındı".into();
    let yeni = d
        .gun_sonu_duzelt(&a, &gs.id, &g, "Nebim toplamı yanlış", &yonetici())
        .unwrap();
    assert_eq!(yeni.duzeltir.as_deref(), Some(gs.id.as_str()));
    assert_ne!(yeni.id, gs.id);
    let gecerli = d.gun_sonlari().unwrap();
    assert_eq!(gecerli.len(), 1);
    assert_eq!(gecerli[0].id, yeni.id);
    // Hiçbir satır silinmedi: iptal + yeni kayıt eklendi.
    assert_eq!(
        taa_dosya::satirlari_oku(&d.gun_yolu()).unwrap().len(),
        satir_once + 2
    );
    // Masraf iptali de silmez.
    let m = d
        .masraf_ekle(
            &a,
            &MasrafGirdi {
                tarih: T.into(),
                cekmece: "c1".into(),
                tur: "Yemek".into(),
                tutar_kurus: 100,
                aciklama: "çay".into(),
                foto: None,
                not: String::new(),
            },
            &ayse(),
        )
        .unwrap();
    d.iptal(&m.id, "yanlış çekmece", &yonetici()).unwrap();
    assert_eq!(d.masraflar().unwrap()[0].durum, MasrafDurumu::Iptal);
    assert!(d.iptal(&m.id, "tekrar", &yonetici()).is_err());
}

#[test]
fn ayar_dogrulama_ve_kayit() {
    let mut a = esenyurt();
    a.dogrula().unwrap();
    let y = taa_dosya::gecici_klasor()
        .unwrap()
        .join("kasa-ayar/ayar.json");
    a.kaydet(&y).unwrap();
    assert_eq!(KasaAyar::yukle(&y).unwrap(), a);
    a.cekmeceler[0].noktalar[1].id = "k1".into();
    assert!(a.dogrula().is_err(), "yinelenen kimlik");
    let b = KasaAyar {
        gonderim_son_saat: "25:00".into(),
        ..KasaAyar::default()
    };
    assert!(b.dogrula().is_err());
    assert_eq!(
        KasaAyar::yukle(&y.with_file_name("yok.json")).unwrap(),
        KasaAyar::default()
    );
}

#[test]
fn kuyruk_cevrimdisi_bekletir_sonra_teslim_eder_ve_matris() {
    let d = defter("kuyruk");
    let a = esenyurt();
    acilis(&d, &a);
    let gs = d.gun_sonu_ekle(&a, &gun_sonu_girdi(), &ayse()).unwrap();
    d.onayla(&gs.id, OnayTuru::IkinciKisi, &mehmet_mudur())
        .unwrap();
    d.ikinci_imza_gerekcesi(&gs.id, "tek kişi kapanış", &mehmet_mudur())
        .unwrap();
    let p = d.gonderim_paketi(&a, &gs.id).unwrap();
    assert_eq!(p["imzalar"][0]["kisi"], mehmet_mudur().ad);
    assert_eq!(p["tur"], "kasa.gunsonu@1");
    let q = Kuyruk { kok: d.kok.clone() };
    let ad = format!("{}.json", gs.id);
    q.ekle(&ad, p.to_string().as_bytes()).unwrap();
    assert!(q.ekle("../kotu.json", b"{}").is_err());
    d.gonderim_kaydet(&gs.id, &ad, false).unwrap();
    assert_eq!(d.gun_durumu(&a, T, "c1").unwrap().asama, "tamam");
    // Merkez klasörü yok: kuyrukta kalır.
    let hedef = d.kok.join("merkez");
    let t = KlasorTasiyici {
        hedef: hedef.clone(),
    };
    let (n, h) = q.bosalt(&t);
    assert_eq!((n, h.is_some()), (0, true));
    assert_eq!(q.bekleyenler(), vec![ad.clone()]);
    std::fs::create_dir_all(&hedef).unwrap();
    assert_eq!(q.bosalt(&t), (1, None));
    assert!(q.bekleyenler().is_empty());
    assert_eq!(q.gonderilenler(), vec![ad.clone()]);
    std::fs::write(hedef.join("bozuk.json"), b"{").unwrap();
    let (gelen, bozuk) = merkez::gelenleri_oku(&hedef);
    assert_eq!((gelen.len(), bozuk), (1, 1));
    assert_eq!(gelen[0].sube, "ESY");

    // Matris: zamanında geldi, geç, eksik, bekleniyor.
    let son = |t: &str| zaman::yerel_an(t, "23:00", zaman::TURKIYE_DK).unwrap();
    let gelenler = vec![
        GelenPaket {
            sube: "ESY".into(),
            tarih: "2026-11-24".into(),
            alinma_sn: son("2026-11-24") - 60,
            kasa_ciro_uyusmuyor: true,
            ikinci_imza_atmadi: true,
            ikinci_imza_gerekcesi: Some("ikinci kişi izinli".into()),
            ..Default::default()
        },
        GelenPaket {
            sube: "ESY".into(),
            tarih: "2026-11-25".into(),
            alinma_sn: son("2026-11-25") + 60,
            kasa_ciro_uyusmuyor: false,
            ikinci_imza_atmadi: false,
            ikinci_imza_gerekcesi: None,
            ..Default::default()
        },
    ];
    let tarihler: Vec<String> = ["2026-11-24", "2026-11-25", "2026-11-26", "2026-11-27"]
        .map(String::from)
        .to_vec();
    let m = merkez::matris(
        &gelenler,
        &["ESY".into(), "BKY".into()],
        &tarihler,
        "23:00",
        zaman::TURKIYE_DK,
        son("2026-11-26") + 1,
    );
    let esy: Vec<MatrisDurumu> = m[0].hucreler.iter().map(|h| h.durum).collect();
    assert_eq!(
        esy,
        [
            MatrisDurumu::Geldi,
            MatrisDurumu::Gec,
            MatrisDurumu::Eksik,
            MatrisDurumu::Bekleniyor
        ]
    );
    assert!(m[0].hucreler[0].kasa_ciro_uyusmuyor);
    assert_eq!(m[1].hucreler[0].durum, MatrisDurumu::Eksik);
}

#[test]
fn excel_uc_sayfa_uretir() {
    let d = defter("excel");
    let a = esenyurt();
    acilis(&d, &a);
    d.gun_sonu_ekle(&a, &gun_sonu_girdi(), &ayse()).unwrap();
    let b = merkez::excel(
        "ESY",
        &d.gun_sonlari().unwrap(),
        &d.masraflar().unwrap(),
        &d.sayimlar().unwrap(),
        &d.imza_haritasi().unwrap(),
        &d.gerekce_haritasi().unwrap(),
    );
    let m = String::from_utf8_lossy(&b);
    assert!(m.contains("ikinci imza atmadı"));
    assert!(
        m.contains("name=\"Gün sonu\"")
            && m.contains("name=\"Masraf\"")
            && m.contains("name=\"Sayımlar\"")
    );
    assert!(m.contains("<v>1500.00</v>"), "sayılan nakit TL olarak");
}

#[test]
fn magaza_toplami_cekmecelerden_toplanir() {
    let d = defter("toplam");
    let mut a = esenyurt();
    a.cekmeceler.push(Cekmece {
        id: "c2".into(),
        ad: "Çekmece 2".into(),
        noktalar: vec![KasaNoktasi {
            id: "k3".into(),
            ad: "K3".into(),
            poslar: vec![],
        }],
    });
    acilis(&d, &a);
    d.gun_sonu_ekle(&a, &gun_sonu_girdi(), &ayse()).unwrap();
    let g2 = GunSonuGirdi {
        tarih: T.into(),
        cekmece: "c2".into(),
        nakit_adetler: adet(&[(10_000, 1)]),
        noktalar: vec![NoktaGunSonu {
            nokta: "k3".into(),
            z_toplam_kurus: 10_000,
            pos: BTreeMap::new(),
        }],
        nebim_toplam_kurus: 10_000,
        ciro: BTreeMap::from([("nakit".into(), 10_000)]),
        kontrol: vec![true; 7],
        ..Default::default()
    };
    d.gun_sonu_ekle(&a, &g2, &ayse()).unwrap();
    let t = magaza_toplami(&d.gun_sonlari().unwrap(), T);
    assert_eq!(t["cekmece_sayisi"], 2);
    assert_eq!(t["nakit_sayilan_kurus"], 160_000);
    assert_eq!(t["ciro"]["nakit"], 110_000);
}

#[test]
fn eski_defter_uyumu_korunur() {
    // Eski TaaKasaDefteri API'si değişmedi.
    let y = taa_dosya::gecici_klasor().unwrap().join("kasa-eski.jsonl");
    let d = crate::Defter { yol: &y };
    let c = d
        .kapanis_ekle(T, "M", "u", adet(&[(100, 1)]), 100, "")
        .unwrap();
    assert_eq!(c.id, "K-20261126-1");
}

#[test]
fn eksi_kredi_ceki_kabul_efatura_reddedilir() {
    let d = defter("kredi-ceki");
    let a = esenyurt();
    acilis(&d, &a);
    let mut g = gun_sonu_girdi();
    g.ciro.insert(ESKI_EFATURA.into(), 1_000);
    assert!(d
        .gun_sonu_ekle(&a, &g, &ayse())
        .unwrap_err()
        .contains("İYZİCO"));
    g.ciro.remove(ESKI_EFATURA);
    g.ciro.insert("kredi_ceki".into(), -5_000);
    g.not = "kredi çeki eksi".into();
    let gs = d.gun_sonu_ekle(&a, &g, &ayse()).unwrap();
    let onceki = ciro_toplami(&gun_sonu_girdi().ciro, &gun_sonu_girdi().doviz, 0);
    assert_eq!(gs.karsilastirma.ciro_toplam_kurus, onceki - 5_000);
}
