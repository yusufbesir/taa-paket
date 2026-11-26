// Sahip: Yusuf. yazan: [Claude#egitim-panel 2026-11-26]
//! API testleri (saf işlevler; HTTP ve pencere yok). Gerçek kişi, parola ya da anahtar yok: tüm adlar kurgusaldır.

use crate::api::{Panel, VideoYaniti};
use crate::gorunum::Tema;
use crate::kimlik::Kullanici;
use crate::veri::{katalog_yukle, rehber_yukle};
use crate::video::Karar;
use serde_json::{json, Value};
use std::path::PathBuf;

const Z: &str = "2026-11-26T09:00:00Z";
const Z2: &str = "2026-11-26T09:05:00Z";

const KATALOG: &str = r#"{
  "dersler": [
    {"id":"kasa-acilis","baslik":"Kasa açılışı","konu":"kasa-egitimi","gorev_etiketleri":["kasa-sorumlusu"],"sure_sn":600,"surum":1},
    {"id":"kasa-gun-sonu","baslik":"Kasa gün sonu","konu":"kasa-egitimi","gorev_etiketleri":["kasa-sorumlusu"],"sure_sn":900,"surum":1},
    {"id":"iade","baslik":"İade işlemleri","konu":"satis","gorev_etiketleri":["satis-danismani"],"sure_sn":300,"surum":1},
    {"id":"etiket-oyunu","baslik":"<b>Etiket</b> & \"tırnak\"","konu":"genel","sure_sn":60,"surum":1}
  ],
  "videolar": {
    "kasa-acilis": {"url": "https://ornek.test/kasa-acilis.mp4"},
    "kasa-gun-sonu": {"yol": "kasa/gun-sonu.mp4"},
    "iade": {"yol": "iade-yok.mp4"}
  },
  "konu_adlari": {"satis": "Satış"}
}"#;

const REHBER: &str = r#"{
  "kisiler": [
    {"id":"ayse","ad":"Ayşe Kurgu","eposta":"ayse@sirket.test","durum":"etkin","surum":1,"guncelleme":"2026-11-26T09:00:00Z"},
    {"id":"mehmet","ad":"Mehmet Kurgu","eposta":"mehmet@sirket.test","durum":"etkin","surum":1,"guncelleme":"2026-11-26T09:00:00Z"},
    {"id":"mudur","ad":"Müdür Kurgu","eposta":"mudur@sirket.test","durum":"etkin","surum":1,"guncelleme":"2026-11-26T09:00:00Z"},
    {"id":"eski","ad":"Eski Kurgu","eposta":"eski@sirket.test","durum":"pasif","pasif_tarihi":"2026-10-01","surum":1,"guncelleme":"2026-11-26T09:00:00Z"}
  ],
  "birimler": [{"id":"merkez","ad":"Merkez","tur":"merkez","durum":"etkin","surum":1,"guncelleme":"2026-11-26T09:00:00Z"}],
  "roller": [
    {"kod":"kasa-sorumlusu","ad":"Kasa sorumlusu","durum":"etkin"},
    {"kod":"satis-danismani","ad":"Satış danışmanı","durum":"etkin"}
  ],
  "gorevlendirmeler": [
    {"id":"g1","kisi_id":"ayse","birim_id":"merkez","rol":"kasa-sorumlusu","tur":"asil","durum":"etkin","baslangic":"2026-01-01","surum":1,"guncelleme":"2026-11-26T09:00:00Z"},
    {"id":"g2","kisi_id":"mehmet","birim_id":"merkez","rol":"satis-danismani","tur":"asil","durum":"etkin","baslangic":"2026-01-01","surum":1,"guncelleme":"2026-11-26T09:00:00Z"}
  ]
}"#;

fn kok(ad: &str) -> PathBuf {
    let mut p = std::env::temp_dir();
    p.push(format!("taa-egitim-panel-test-{}-{ad}", std::process::id()));
    let _ = std::fs::remove_dir_all(&p);
    std::fs::create_dir_all(p.join("video").join("kasa")).unwrap();
    p
}

fn kur(kok: &std::path::Path, gunluk: bool) -> Panel {
    Panel::kur(
        katalog_yukle(KATALOG).unwrap(),
        rehber_yukle(REHBER).unwrap(),
        kok.join("video"),
        gunluk
            .then(|| kok.join("egitim").join("izleme.jsonl"))
            .as_deref(),
        false,
        Some(kok.join("egitim").join("gorunum.json")),
        vec![],
    )
    .unwrap()
}

fn kul(ad: &str, yonetici: bool) -> Kullanici {
    Kullanici {
        ad: ad.into(),
        yonetici,
    }
}

fn al(p: &mut Panel, k: Option<&Kullanici>, yol: &str, sorgu: &str) -> (u16, Value) {
    p.api(k, "GET", yol, sorgu, &Value::Null, Z)
}

fn gonder(p: &mut Panel, k: Option<&Kullanici>, yol: &str, govde: Value) -> (u16, Value) {
    p.api(k, "POST", yol, "", &govde, Z)
}

fn ids(v: &Value) -> Vec<&str> {
    v["videolar"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["id"].as_str().unwrap())
        .collect()
}

#[test]
fn oturumsuz_her_uc_401_ama_oturum_ve_gorunum_acik() {
    let k = kok("oturumsuz");
    let mut p = kur(&k, false);
    for yol in [
        "/api/videolar",
        "/api/atamalarim",
        "/api/ozet",
        "/api/yonetim",
    ] {
        assert_eq!(al(&mut p, None, yol, "").0, 401, "{yol}");
    }
    for yol in [
        "/api/konum",
        "/api/izledim",
        "/api/atama",
        "/api/atama/iptal",
    ] {
        assert_eq!(gonder(&mut p, None, yol, json!({})).0, 401, "{yol}");
    }
    let (d, o) = al(&mut p, None, "/api/oturum", "");
    assert_eq!(d, 200);
    assert_eq!(o["girisli"], false);
    assert!(o.get("uyarilar").is_none(), "oturumsuz uyarı sızmaz");
    assert_eq!(al(&mut p, None, "/api/gorunum", "").0, 200);
    assert_eq!(al(&mut p, None, "/baska", "").0, 404);
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn yonetici_olmayan_yonetici_uclarinda_403() {
    let k = kok("yonetici-yok");
    let mut p = kur(&k, false);
    let ayse = kul("ayse", false);
    assert_eq!(al(&mut p, Some(&ayse), "/api/ozet", "").0, 403);
    assert_eq!(al(&mut p, Some(&ayse), "/api/yonetim", "").0, 403);
    for yol in ["/api/atama", "/api/atama/iptal", "/api/atama/gorevden"] {
        let (d, g) = gonder(
            &mut p,
            Some(&ayse),
            yol,
            json!({"kisi": "ayse", "ders": "iade"}),
        );
        assert_eq!(d, 403, "{yol}");
        assert_eq!(g["kod"], 7);
    }
    // kendi adına atama yapamaz: atama gerçekten oluşmadı
    let (_, l) = al(&mut p, Some(&ayse), "/api/atamalarim", "");
    assert!(l["atamalar"].as_array().unwrap().is_empty());
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn liste_konu_ve_gorev_suzgeci_ve_kasa_konusu() {
    let k = kok("liste");
    let mut p = kur(&k, false);
    let ayse = kul("ayse", false);
    let (d, v) = al(&mut p, Some(&ayse), "/api/videolar", "");
    assert_eq!(d, 200);
    assert_eq!(ids(&v).len(), 4);
    let (_, v) = al(&mut p, Some(&ayse), "/api/videolar", "konu=kasa-egitimi");
    assert_eq!(ids(&v), ["kasa-acilis", "kasa-gun-sonu"]);
    assert_eq!(v["videolar"][0]["konu_ad"], "Kasa eğitimi");
    let (_, v) = al(
        &mut p,
        Some(&ayse),
        "/api/videolar",
        "gorev=satis-danismani",
    );
    assert_eq!(ids(&v), ["iade"]);
    assert_eq!(v["videolar"][0]["gorevler"][0]["ad"], "Satış danışmanı");
    assert_eq!(v["videolar"][0]["konu_ad"], "Satış");
    let (_, v) = al(
        &mut p,
        Some(&ayse),
        "/api/videolar",
        "konu=satis&gorev=kasa-sorumlusu",
    );
    assert!(ids(&v).is_empty());
    // süzgeç seçenekleri tüm katalogdan gelir
    assert_eq!(v["konular"].as_array().unwrap().len(), 3);
    assert_eq!(v["gorevler"].as_array().unwrap().len(), 2);
    assert_eq!(v["katalog_bos"], false);
    // kodlanmış sorgu
    let (_, v) = al(
        &mut p,
        Some(&ayse),
        "/api/videolar",
        "konu=kasa%2Degitimi&gorev=",
    );
    assert_eq!(ids(&v).len(), 2);
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn video_durumlari_durust_bos_durum() {
    let k = kok("durum");
    let mut p = kur(&k, false);
    let ayse = kul("ayse", false);
    let (_, v) = al(&mut p, Some(&ayse), "/api/videolar", "");
    let bul = |id: &str| {
        v["videolar"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["id"] == id)
            .unwrap()
            .clone()
    };
    assert_eq!(bul("kasa-acilis")["video"], "hazir");
    assert_eq!(bul("kasa-acilis")["kaynak"], "url");
    assert_eq!(
        bul("kasa-acilis")["src"],
        "https://ornek.test/kasa-acilis.mp4"
    );
    assert_eq!(bul("kasa-gun-sonu")["video"], "dosya-yok");
    assert!(bul("kasa-gun-sonu")["src"].is_null());
    assert_eq!(bul("etiket-oyunu")["video"], "yok");
    // dosya konunca hazır olur ve yerel uca bağlanır
    std::fs::write(k.join("video/kasa/gun-sonu.mp4"), b"x").unwrap();
    let (_, v) = al(&mut p, Some(&ayse), "/api/videolar", "konu=kasa-egitimi");
    let g = v["videolar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "kasa-gun-sonu")
        .unwrap();
    assert_eq!(g["video"], "hazir");
    assert_eq!(g["src"], "/video/kasa-gun-sonu");
    assert_eq!(g["kaynak"], "yerel");
    // metin olduğu gibi (kaçışlamayı ekran yapar) ve yapı bozulmaz
    assert_eq!(bul("etiket-oyunu")["baslik"], "<b>Etiket</b> & \"tırnak\"");
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn konum_kaydi_devam_ve_izledim() {
    let k = kok("konum");
    let mut p = kur(&k, false);
    let ayse = kul("ayse", false);
    let (d, y) = gonder(
        &mut p,
        Some(&ayse),
        "/api/konum",
        json!({"ders": "kasa-acilis", "saniye": 120}),
    );
    assert_eq!(
        (d, y["tamam"].clone(), y["kaydedildi"].clone()),
        (200, json!(true), json!(true))
    );
    let (_, v) = al(&mut p, Some(&ayse), "/api/videolar", "");
    let g = v["videolar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "kasa-acilis")
        .unwrap();
    assert_eq!(
        (g["saniye"].clone(), g["yuzde"].clone(), g["durum"].clone()),
        (json!(120), json!(20), json!("basladi"))
    );
    assert_eq!(v["devam"]["id"], "kasa-acilis");
    assert_eq!(v["devam"]["saniye"], 120);
    // aynı konum tekrar: değişiklik yok
    let (_, y) = gonder(
        &mut p,
        Some(&ayse),
        "/api/konum",
        json!({"ders": "kasa-acilis", "saniye": 120}),
    );
    assert_eq!(y["kaydedildi"], false);
    // süre aşımı kırpılır
    let (d, y) = gonder(
        &mut p,
        Some(&ayse),
        "/api/konum",
        json!({"ders": "kasa-acilis", "saniye": 99999}),
    );
    assert_eq!((d, y["saniye"].clone()), (200, json!(600)));
    // izledim: devam noktası sıfırlanır, durum izledi
    let (d, y) = gonder(
        &mut p,
        Some(&ayse),
        "/api/izledim",
        json!({"ders": "kasa-acilis"}),
    );
    assert_eq!((d, y["yeni"].clone()), (200, json!(true)));
    let (_, v) = al(&mut p, Some(&ayse), "/api/videolar", "");
    let g = v["videolar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "kasa-acilis")
        .unwrap();
    assert_eq!(
        (g["durum"].clone(), g["yuzde"].clone()),
        (json!("izledi"), json!(100))
    );
    assert!(v["devam"].is_null());
    // başkasının kaydı etkilenmez
    let mehmet = kul("mehmet", false);
    let (_, v) = al(&mut p, Some(&mehmet), "/api/videolar", "");
    let g = v["videolar"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == "kasa-acilis")
        .unwrap();
    assert_eq!(g["durum"], "baslanmadi");
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn girdi_ve_kisi_hatalari() {
    let k = kok("girdi");
    let mut p = kur(&k, false);
    let ayse = kul("ayse", false);
    let kos = |p: &mut Panel, k: &Kullanici, g: Value| gonder(p, Some(k), "/api/konum", g).0;
    assert_eq!(kos(&mut p, &ayse, json!({"saniye": 5})), 400);
    assert_eq!(kos(&mut p, &ayse, json!({"ders": "kasa-acilis"})), 400);
    assert_eq!(
        kos(&mut p, &ayse, json!({"ders": "kasa-acilis", "saniye": -3})),
        400
    );
    assert_eq!(
        kos(&mut p, &ayse, json!({"ders": "kasa-acilis", "saniye": 1.5})),
        400
    );
    assert_eq!(kos(&mut p, &ayse, json!({"ders": "yok", "saniye": 5})), 404);
    // video yok / dosya yok: kayıt tutulmaz
    assert_eq!(
        kos(&mut p, &ayse, json!({"ders": "etiket-oyunu", "saniye": 5})),
        409
    );
    assert_eq!(
        kos(&mut p, &ayse, json!({"ders": "iade", "saniye": 5})),
        409
    );
    assert_eq!(
        gonder(&mut p, Some(&ayse), "/api/izledim", json!({"ders": "iade"})).0,
        409
    );
    // pasif kişi, rehberde olmayan kişi, rehber kimliğine uymayan giriş adı
    for ad in ["eski", "yok", "ayse.yilmaz"] {
        let (d, _) = gonder(
            &mut p,
            Some(&kul(ad, false)),
            "/api/konum",
            json!({"ders": "kasa-acilis", "saniye": 5}),
        );
        assert_eq!(d, 403, "{ad}");
    }
    // yöntem ve uç
    assert_eq!(
        p.api(Some(&ayse), "DELETE", "/api/konum", "", &Value::Null, Z)
            .0,
        405
    );
    assert_eq!(al(&mut p, Some(&ayse), "/api/yok", "").0, 404);
    // oturum özeti
    let (_, o) = al(&mut p, Some(&ayse), "/api/oturum", "");
    assert_eq!(
        (
            o["girisli"].clone(),
            o["kisi_etkin"].clone(),
            o["kisi_adi"].clone()
        ),
        (json!(true), json!(true), json!("Ayşe Kurgu"))
    );
    let (_, o) = al(&mut p, Some(&kul("yok", false)), "/api/oturum", "");
    assert_eq!(o["kisi_etkin"], false);
    assert!(o["kisi_adi"].is_null());
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn yonetici_atar_iptal_eder_ve_ozet_gorur() {
    let k = kok("atama");
    let mut p = kur(&k, false);
    let mudur = kul("mudur", true);
    let ayse = kul("ayse", false);
    let (d, y) = gonder(
        &mut p,
        Some(&mudur),
        "/api/atama",
        json!({"kisi": "ayse", "ders": "kasa-acilis"}),
    );
    assert_eq!((d, y["yeni"].clone()), (200, json!(true)));
    // aynı atama tekrar: açıklama
    let (d, y) = gonder(
        &mut p,
        Some(&mudur),
        "/api/atama",
        json!({"kisi": "ayse", "ders": "kasa-acilis"}),
    );
    assert_eq!(
        (d, y["yeni"].clone(), y["mesaj"].clone()),
        (200, json!(false), json!("Bu atama zaten var."))
    );
    // rehberde olmayan / pasif kişiye atama reddedilir
    assert_eq!(
        gonder(
            &mut p,
            Some(&mudur),
            "/api/atama",
            json!({"kisi": "yok", "ders": "iade"})
        )
        .0,
        403
    );
    assert_eq!(
        gonder(
            &mut p,
            Some(&mudur),
            "/api/atama",
            json!({"kisi": "eski", "ders": "iade"})
        )
        .0,
        403
    );
    assert_eq!(
        gonder(
            &mut p,
            Some(&mudur),
            "/api/atama",
            json!({"kisi": "ayse", "ders": "yok"})
        )
        .0,
        404
    );
    assert_eq!(
        gonder(&mut p, Some(&mudur), "/api/atama", json!({"kisi": "ayse"})).0,
        400
    );
    // Ayşe atamasını görür
    let (_, l) = al(&mut p, Some(&ayse), "/api/atamalarim", "");
    assert_eq!(l["atamalar"].as_array().unwrap().len(), 1);
    assert_eq!(l["atamalar"][0]["id"], "kasa-acilis");
    assert_eq!(l["atamalar"][0]["durum"], "atandi");
    // görevden atama: Ayşe kasa sorumlusu, Mehmet satış danışmanı
    let (_, y) = gonder(
        &mut p,
        Some(&mudur),
        "/api/atama/gorevden",
        json!({"kisi": "ayse"}),
    );
    assert_eq!(y["yeni"], json!(["kasa-gun-sonu"]));
    let (_, y) = gonder(
        &mut p,
        Some(&mudur),
        "/api/atama/gorevden",
        json!({"kisi": "mehmet"}),
    );
    assert_eq!(y["yeni"], json!(["iade"]));
    let (_, y) = gonder(
        &mut p,
        Some(&mudur),
        "/api/atama/gorevden",
        json!({"kisi": "ayse"}),
    );
    assert_eq!(y["mesaj"], "Görevine uyan yeni bir eğitim yok.");
    // özet: kişi adları rehberden, başlıklar katalogdan
    gonder(
        &mut p,
        Some(&ayse),
        "/api/konum",
        json!({"ders": "kasa-acilis", "saniye": 300}),
    );
    let (d, o) = al(&mut p, Some(&mudur), "/api/ozet", "");
    assert_eq!(d, 200);
    assert_eq!(o["bos"], false);
    let satirlar = o["satirlar"].as_array().unwrap();
    assert_eq!(satirlar.len(), 3);
    assert_eq!(satirlar[0]["kisi_ad"], "Ayşe Kurgu");
    let s0 = satirlar
        .iter()
        .find(|s| s["ders_id"] == "kasa-acilis")
        .unwrap();
    assert_eq!(
        (
            s0["durum"].clone(),
            s0["yuzde"].clone(),
            s0["atandi"].clone()
        ),
        (json!("basladi"), json!(50), json!(true))
    );
    assert!(o["dersler"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["ders_id"] == "kasa-acilis" && d["atanan"] == 1 && d["basladi"] == 1));
    // yönetim formu seçenekleri: yalnız etkin kişiler
    let (_, y) = al(&mut p, Some(&mudur), "/api/yonetim", "");
    let kisiler: Vec<&str> = y["kisiler"]
        .as_array()
        .unwrap()
        .iter()
        .map(|x| x["id"].as_str().unwrap())
        .collect();
    assert_eq!(kisiler, ["ayse", "mehmet", "mudur"]);
    assert_eq!(y["dersler"].as_array().unwrap().len(), 4);
    // iptal: kayıt kalır ama atama kalkar
    let (_, y) = gonder(
        &mut p,
        Some(&mudur),
        "/api/atama/iptal",
        json!({"kisi": "ayse", "ders": "kasa-acilis"}),
    );
    assert_eq!(y["yeni"], true);
    let (_, y) = gonder(
        &mut p,
        Some(&mudur),
        "/api/atama/iptal",
        json!({"kisi": "ayse", "ders": "kasa-acilis"}),
    );
    assert_eq!(y["yeni"], false);
    let (_, l) = al(&mut p, Some(&ayse), "/api/atamalarim", "");
    assert!(l["atamalar"]
        .as_array()
        .unwrap()
        .iter()
        .all(|a| a["id"] != "kasa-acilis"));
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn yonetici_rehberde_etkin_degilse_atayamaz() {
    let k = kok("atayan");
    let mut p = kur(&k, false);
    let yabanci_yonetici = kul("kimse", true);
    let (d, y) = gonder(
        &mut p,
        Some(&yabanci_yonetici),
        "/api/atama",
        json!({"kisi": "ayse", "ders": "iade"}),
    );
    assert_eq!(d, 403);
    assert_eq!(y["kod"], 7);
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn bos_katalog_ve_bos_ozet_durust() {
    let k = kok("bos");
    let mut p = Panel::kur(
        katalog_yukle("{}").unwrap(),
        rehber_yukle("{}").unwrap(),
        k.join("video"),
        None,
        false,
        None,
        vec![],
    )
    .unwrap();
    let mudur = kul("mudur", true);
    let (_, v) = al(&mut p, Some(&mudur), "/api/videolar", "");
    assert!(ids(&v).is_empty());
    assert_eq!(v["katalog_bos"], true);
    let (_, o) = al(&mut p, Some(&mudur), "/api/ozet", "");
    assert_eq!(o["bos"], true);
    let (_, l) = al(&mut p, Some(&mudur), "/api/atamalarim", "");
    assert_eq!(l["kisi_etkin"], false);
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn gunlukten_yeniden_kurulur() {
    let k = kok("gunluk");
    {
        let mut p = kur(&k, true);
        let mudur = kul("mudur", true);
        let ayse = kul("ayse", false);
        gonder(
            &mut p,
            Some(&mudur),
            "/api/atama",
            json!({"kisi": "ayse", "ders": "kasa-acilis"}),
        );
        let (d, _) = p.api(
            Some(&ayse),
            "POST",
            "/api/konum",
            "",
            &json!({"ders": "kasa-acilis", "saniye": 240}),
            Z2,
        );
        assert_eq!(d, 200);
    }
    // yeniden açılış: konum ve atama geri gelir, devam düğmesi çıkar
    let mut p = kur(&k, true);
    let ayse = kul("ayse", false);
    let (_, v) = al(&mut p, Some(&ayse), "/api/videolar", "");
    assert_eq!(v["devam"]["id"], "kasa-acilis");
    assert_eq!(v["devam"]["saniye"], 240);
    let (_, l) = al(&mut p, Some(&ayse), "/api/atamalarim", "");
    assert_eq!(l["atamalar"][0]["saniye"], 240);
    assert!(p.uyarilar.is_empty());
    // günlükte kişi adı ya da e-posta yok, yalnız kimlik
    let metin = std::fs::read_to_string(k.join("egitim/izleme.jsonl")).unwrap();
    assert!(
        metin.contains("\"kisi\":\"ayse\"")
            && !metin.contains("Kurgu")
            && !metin.contains("sirket.test")
    );
    // bozuk satır uyarı olarak görünür, panel açılır
    let mut dosya = std::fs::OpenOptions::new()
        .append(true)
        .open(k.join("egitim/izleme.jsonl"))
        .unwrap();
    std::io::Write::write_all(&mut dosya, b"{bozuk\n").unwrap();
    let p = kur(&k, true);
    assert!(p.uyarilar.iter().any(|u| u.contains("bozuk")));
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn gunluksuz_kipte_diske_yazilmaz() {
    let k = kok("gunluksuz");
    let mut p = kur(&k, false);
    let ayse = kul("ayse", false);
    let (d, _) = gonder(
        &mut p,
        Some(&ayse),
        "/api/konum",
        json!({"ders": "kasa-acilis", "saniye": 10}),
    );
    assert_eq!(d, 200);
    assert!(
        !k.join("egitim").exists(),
        "günlüksüz kipte diske hiçbir şey yazılmaz"
    );
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn demo_kipi_ornek_etiketli_ve_diske_yazmaz() {
    let k = kok("demo");
    let mut p = Panel::kur(
        crate::veri::demo_katalog(),
        crate::veri::demo_rehber(),
        k.join("video"),
        None,
        true,
        None,
        vec![],
    )
    .unwrap();
    let u = kul("deneme", false);
    let (_, o) = al(&mut p, Some(&u), "/api/oturum", "");
    assert_eq!(o["demo"], true);
    assert_eq!(o["kisi_adi"], "ÖRNEK oturum sahibi");
    let (_, v) = al(&mut p, Some(&u), "/api/videolar", "");
    assert!(v["videolar"]
        .as_array()
        .unwrap()
        .iter()
        .all(|d| d["baslik"].as_str().unwrap().starts_with("ÖRNEK")));
    assert!(v["videolar"]
        .as_array()
        .unwrap()
        .iter()
        .all(|d| d["video"] == "yok"));
    // demo yönetici sağlamaz: yönetici bayrağı yalnız gerçek yetkiden gelir
    assert_eq!(al(&mut p, Some(&u), "/api/ozet", "").0, 403);
    assert!(!k.join("egitim").exists());
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn gorunum_kaydi() {
    let k = kok("gorunum");
    let mut p = kur(&k, false);
    let ayse = kul("ayse", false);
    assert_eq!(al(&mut p, None, "/api/gorunum", "").1["tema"], "acik");
    let (d, y) = gonder(&mut p, Some(&ayse), "/api/gorunum", json!({"tema": "koyu"}));
    assert_eq!((d, y["tema"].clone()), (200, json!("koyu")));
    assert_eq!(
        gonder(&mut p, Some(&ayse), "/api/gorunum", json!({"tema": "mor"})).0,
        400
    );
    assert_eq!(
        gonder(&mut p, None, "/api/gorunum", json!({"tema": "tek"})).0,
        401
    );
    // yeniden açılışta hatırlanır ve ekrana açılıştan itibaren yazılır
    let p2 = kur(&k, false);
    assert_eq!(p2.tema, Tema::Koyu);
    let html = crate::arayuz_html(p2.tema);
    assert!(html.contains("data-theme=\"koyu\"") && !html.contains("__TEMA__"));
    let _ = std::fs::remove_dir_all(&k);
}

#[test]
fn video_sunumu_range_ve_yetki() {
    let k = kok("video");
    std::fs::write(k.join("video/kasa/gun-sonu.mp4"), b"0123456789").unwrap();
    let p = kur(&k, false);
    let ayse = kul("ayse", false);
    // oturumsuz 401
    assert!(matches!(
        p.video_hazirla(None, "kasa-gun-sonu", None),
        VideoYaniti::Hata((401, _))
    ));
    // Range yok: tam içerik
    match p.video_hazirla(Some(&ayse), "kasa-gun-sonu", None) {
        VideoYaniti::Dosya { tur, karar, yol } => {
            assert_eq!(tur, "video/mp4");
            assert_eq!(karar, Karar::Tam { toplam: 10 });
            assert!(yol.ends_with("kasa/gun-sonu.mp4"));
        }
        d => panic!("{d:?}"),
    }
    // Range var: 206
    match p.video_hazirla(Some(&ayse), "kasa-gun-sonu", Some("bytes=2-5")) {
        VideoYaniti::Dosya { karar, .. } => {
            assert_eq!(
                karar,
                Karar::Kismi {
                    bas: 2,
                    son: 5,
                    toplam: 10
                }
            );
            assert_eq!(karar.icerik_araligi().as_deref(), Some("bytes 2-5/10"));
        }
        d => panic!("{d:?}"),
    }
    // aralık dışı: 416
    match p.video_hazirla(Some(&ayse), "kasa-gun-sonu", Some("bytes=50-")) {
        VideoYaniti::Dosya { karar, .. } => assert_eq!(karar.durum_kodu(), 416),
        d => panic!("{d:?}"),
    }
    // URL kaynaklı, dosyası olmayan, katalogda olmayan: 404
    for ders in [
        "kasa-acilis",
        "iade",
        "yok",
        "etiket-oyunu",
        "../kasa-acilis",
    ] {
        assert!(
            matches!(
                p.video_hazirla(Some(&ayse), ders, None),
                VideoYaniti::Hata((404, _))
            ),
            "{ders}"
        );
    }
    let _ = std::fs::remove_dir_all(&k);
}
