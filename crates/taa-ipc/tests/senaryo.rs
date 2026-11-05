// Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05]
//! taa-ipc bütünleşik senaryoları (T1-TASARIM §2.3): iki sanal program, istek/yanıt,
//! olay yayını ve abone, 2 MiB reddi, bozuk dosya, yetenek yok, eşzamanlı yazım.

use serde_json::json;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::thread;
use taa_ipc::{GelenKutusu, IpcHata, Tasiyici, Zarf, ZarfTuru, ZARF_SINIRI};

/// Geçici kök; düşünce yalnız kendi klasörünü temizler.
struct Kok(PathBuf);

impl Kok {
    fn yeni() -> Kok {
        Kok(taa_dosya::gecici_klasor().unwrap())
    }
}

impl Drop for Kok {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn sv(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

/// Aynı kökte iki sanal program: A (taarkod) `kod.uret@1` sunar, B (tutaa) çağırır.
fn iki_program(k: &Kok) -> (GelenKutusu, GelenKutusu) {
    let a = GelenKutusu::yeni(&k.0);
    let b = GelenKutusu::yeni(&k.0);
    a.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
    b.kayit_ol("tutaa", &sv(&["veri.urunler@1"])).unwrap();
    (a, b)
}

#[test]
fn iki_program_istek_yanit() {
    let k = Kok::yeni();
    let (a, b) = iki_program(&k);

    // B, yeteneği sunan programı bulur.
    assert_eq!(
        b.yetenek_bul("kod.uret@1").unwrap().as_deref(),
        Some("taarkod")
    );
    assert_eq!(
        a.yetenek_bul("veri.urunler@1").unwrap().as_deref(),
        Some("tutaa")
    );

    // B istek yazar, A okur ve yanıtlar, B yanıtı alır.
    let istek = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!({"urun": 7}));
    let id = b.gonder(&istek).unwrap();
    let gelen = a.al("taarkod");
    assert_eq!(gelen.len(), 1);
    assert_eq!(gelen[0].id, id);
    assert_eq!(gelen[0].yuk, json!({"urun": 7}));
    assert!(b.yanit_al("tutaa", &id).is_none(), "yanıt henüz yok");

    a.yanitla(&gelen[0], json!({"kod": "ABC-7"})).unwrap();
    assert!(a.al("taarkod").is_empty());
    let yanit = b.yanit_al("tutaa", &id).expect("yanıt gelmeli");
    assert_eq!(yanit.tur, ZarfTuru::Yanit);
    assert_eq!(yanit.yanit_id.as_deref(), Some(id.as_str()));
    assert_eq!(yanit.yuk, json!({"kod": "ABC-7"}));

    // Dosyalar silinmez, islenen/ altına taşınır.
    for program in ["taarkod", "tutaa"] {
        assert!(
            k.0.join(program)
                .join("islenen")
                .join(format!("{id}.json"))
                .exists(),
            "{program}/islenen altında dosya olmalı"
        );
    }
}

#[test]
fn olay_yayin_abone() {
    let k = Kok::yeni();
    let (a, b) = iki_program(&k);

    let o1 = a
        .olay_yayinla("taarkod", "veri.degisti@1", json!({"n": 1}))
        .unwrap();
    let o2 = a
        .olay_yayinla("taarkod", "veri.degisti@1", json!({"n": 2}))
        .unwrap();

    let hepsi = b.olaylar("tutaa", "");
    assert_eq!(
        hepsi.iter().map(|z| z.id.as_str()).collect::<Vec<_>>(),
        [o1.as_str(), o2.as_str()]
    );
    assert!(hepsi
        .iter()
        .all(|z| z.tur == ZarfTuru::Olay && z.yetenek == "veri.degisti@1"));

    // Abone son okuduğu kimlikten sonrasını alır.
    let o3 = a
        .olay_yayinla("taarkod", "veri.degisti@1", json!({"n": 3}))
        .unwrap();
    let yeni = b.olaylar("tutaa", &o2);
    assert_eq!(yeni.len(), 1);
    assert_eq!(yeni[0].id, o3);
    assert!(b.olaylar("tutaa", &o3).is_empty());

    // Yayıncı kendi olaylarını görmez.
    assert!(a.olaylar("taarkod", "").is_empty());
}

#[test]
fn iki_mib_zarf_reddedilir() {
    let k = Kok::yeni();
    let (a, b) = iki_program(&k);
    let buyuk = json!("x".repeat(2 * 1024 * 1024));
    let z = Zarf::istek("tutaa", "taarkod", "kod.uret@1", buyuk.clone());
    match b.gonder(&z) {
        Err(IpcHata::ZarfBuyuk { boyut, sinir }) => {
            assert!(boyut > sinir);
            assert_eq!(sinir, ZARF_SINIRI);
        }
        s => panic!("beklenmeyen: {s:?}"),
    }
    assert!(a.al("taarkod").is_empty());
    assert_eq!(
        k.0.join("taarkod/gelen").read_dir().unwrap().count(),
        0,
        "dosya yazılmamalı"
    );
    assert!(matches!(
        b.olay_yayinla("tutaa", "veri.degisti@1", buyuk),
        Err(IpcHata::ZarfBuyuk { .. })
    ));
}

#[test]
fn bozuk_dosya_okumayi_durdurmaz() {
    let k = Kok::yeni();
    let (a, b) = iki_program(&k);
    let gelen = k.0.join("taarkod/gelen");
    fs::write(gelen.join("0000-bozuk.json"), b"{ yarim").unwrap();
    fs::write(gelen.join("0001-alan.json"), br#"{"v":1}"#).unwrap();
    let z = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!(1));
    b.gonder(&z).unwrap();

    let alinan = a.al("taarkod");
    assert_eq!(alinan.len(), 1);
    assert_eq!(alinan[0].id, z.id);
    assert!(
        k.0.join("taarkod/islenen/bozuk/0000-bozuk.json").exists(),
        "bozuk dosya silinmemeli, islenen/bozuk altına taşınmalı"
    );
    let gunluk = taa_dosya::satirlari_oku(&k.0.join("gunluk.jsonl")).unwrap();
    assert_eq!(gunluk.len(), 2);

    // Bozuk olay satırı da olay okumayı durdurmaz.
    a.olay_yayinla("taarkod", "veri.degisti@1", json!(1))
        .unwrap();
    let gun = fs::read_dir(k.0.join("olaylar"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    taa_dosya::satir_ekle(&gun, "bu json degil").unwrap();
    let o2 = a
        .olay_yayinla("taarkod", "veri.degisti@1", json!(2))
        .unwrap();
    let olaylar = b.olaylar("tutaa", "");
    assert_eq!(olaylar.len(), 2);
    assert_eq!(olaylar[1].id, o2);
}

#[test]
fn yetenek_yoksa_hata() {
    let k = Kok::yeni();
    let (_a, b) = iki_program(&k);
    assert_eq!(b.yetenek_bul("yok.yok@1").unwrap(), None);
    let z = Zarf::istek("tutaa", "taarkod", "yok.yok@1", json!(null));
    assert_eq!(b.gonder(&z), Err(IpcHata::YetenekYok("yok.yok@1".into())));
    // Kayıtsız hedef de aynı hata.
    let z = Zarf::istek("tutaa", "hayalet", "kod.uret@1", json!(null));
    assert!(matches!(b.gonder(&z), Err(IpcHata::YetenekYok(_))));
    assert_eq!(k.0.join("taarkod/gelen").read_dir().unwrap().count(), 0);
}

#[test]
fn eszamanli_iki_is_parcagi_50_istek_id_cakismaz() {
    let k = Kok::yeni();
    let (a, _b) = iki_program(&k);
    let kutu = Arc::new(GelenKutusu::yeni(&k.0));

    let isciler: Vec<_> = ["tutaa", "taarkod"]
        .into_iter()
        .map(|kaynak| {
            let kutu = Arc::clone(&kutu);
            thread::spawn(move || {
                (0..50)
                    .map(|i| {
                        let z = Zarf::istek(kaynak, "taarkod", "kod.uret@1", json!({"i": i}));
                        kutu.gonder(&z).unwrap()
                    })
                    .collect::<Vec<String>>()
            })
        })
        .collect();

    let mut idler: Vec<String> = Vec::new();
    for is in isciler {
        idler.extend(is.join().unwrap());
    }
    assert_eq!(idler.len(), 100);
    let benzersiz: HashSet<&String> = idler.iter().collect();
    assert_eq!(benzersiz.len(), 100, "id çakışması var");

    // Hiçbir istek kaybolmadı, hiçbiri bozulmadı.
    let alinan = a.al("taarkod");
    assert_eq!(alinan.len(), 100);
    let alinan_idler: HashSet<&String> = alinan.iter().map(|z| &z.id).collect();
    assert_eq!(alinan_idler, benzersiz);
    assert!(
        !k.0.join("gunluk.jsonl").exists(),
        "günlüğe bozuk kayıt düşmemeli"
    );
}
