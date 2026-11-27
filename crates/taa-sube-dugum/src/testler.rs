// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-sube-dugum 2026-11-27]
//! taa-sube-dugum testleri: `no` tekilliği, çakışma kutusu, merkez yokken bekleme ve sıra, şube kapsamı,
//! günlük (yalnız ekleme + özet), ağ sınırları, gerçek soketle uçtan uca.

use crate::merkez::{self, tur};
use crate::sunucu::{self, isle, AgAyari, Istek, Yanit};
use crate::*;
use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use taa_kuyruk::{GonderHata, Gonderici, Imzalayici};

fn dugum() -> Dugum {
    let kok = taa_dosya::gecici_klasor().unwrap();
    Dugum::ac(&kok, "MRK01").unwrap()
}

/// Sahte merkez: `acik` değilse ulaşılamaz; `no`ya göre tek kayıt tutar; `kalici_ret`teki `no`ları 422 ile reddeder.
#[derive(Default)]
struct SahteMerkez {
    acik: AtomicBool,
    gelen: Mutex<Vec<(String, Value)>>,
    kalici_ret: Mutex<Vec<String>>,
    /// Bu `no`da 503 döner (geçici hata).
    gecici: Mutex<Option<String>>,
    deneme: Mutex<usize>,
}

impl SahteMerkez {
    fn nolar(&self) -> Vec<String> {
        self.gelen
            .lock()
            .unwrap()
            .iter()
            .map(|(_, g)| g["no"].as_str().unwrap().to_string())
            .collect()
    }
}

impl Gonderici for SahteMerkez {
    fn gonder(&self, uc: &str, govde: &Value) -> Result<Value, GonderHata> {
        *self.deneme.lock().unwrap() += 1;
        if !self.acik.load(Ordering::SeqCst) {
            return Err(GonderHata::Ulasilamadi("ağ yok".into()));
        }
        let no = govde["no"].as_str().unwrap_or_default().to_string();
        if self.gecici.lock().unwrap().as_deref() == Some(&no) {
            return Err(GonderHata::Ulasilamadi("HTTP 503: bakım".into()));
        }
        if self.kalici_ret.lock().unwrap().contains(&no) {
            return Err(GonderHata::Reddedildi {
                kod: 422,
                mesaj: "şema".into(),
            });
        }
        let mut g = self.gelen.lock().unwrap();
        if g.iter().any(|(_, v)| v["no"] == govde["no"]) {
            return Ok(json!({"no": no, "durum": "zaten_var"}));
        }
        g.push((uc.to_string(), govde.clone()));
        Ok(json!({"no": no, "durum": "yazildi"}))
    }

    fn al(&self, _uc: &str) -> Result<Value, GonderHata> {
        if self.acik.load(Ordering::SeqCst) {
            Ok(json!({"durum": "calisiyor"}))
        } else {
            Err(GonderHata::Ulasilamadi("ağ yok".into()))
        }
    }
}

fn gunluk_satirlari(d: &Dugum) -> Vec<Value> {
    let mut v = Vec::new();
    for gun in gunluk::gunler(d.kok()).unwrap() {
        for s in taa_dosya::satirlari_oku(&gunluk::dosya(d.kok(), &gun)).unwrap() {
            v.push(serde_json::from_str(&s).unwrap());
        }
    }
    v
}

fn olay_sayisi(d: &Dugum, olay: &str) -> usize {
    gunluk_satirlari(d)
        .iter()
        .filter(|s| s["olay"] == olay)
        .count()
}

// ---------------------------------------------------------------- çekirdek

#[test]
fn ayni_no_iki_kez_yazilmaz() {
    let d = dugum();
    let g = json!({"no": "A-1", "tutar": 1500, "musteri": "Gizli Kişi"});
    assert_eq!(
        d.al("/v1/kasa/sayim", "A-1", g.clone(), "192.168.1.7")
            .unwrap(),
        Yazma::Yazildi { surum: 1 }
    );
    assert_eq!(
        d.al("/v1/kasa/sayim", "A-1", g.clone(), "192.168.1.8")
            .unwrap(),
        Yazma::ZatenVar {
            surum: 1,
            kesin: false
        }
    );
    assert_eq!(d.kuyruk().kayitlar().unwrap().len(), 1);
    let o = d.durum().unwrap();
    assert_eq!((o.bekleyen, o.gonderilen, o.reddedilen), (1, 0, 0));
    assert_eq!(o.son_merkez_zamani, None);
    // Gövde diskte şifreli; günlükte kayıt içeriği yok.
    let ham = std::fs::read_to_string(d.kuyruk().klasor().join("giden.jsonl")).unwrap();
    assert!(!ham.contains("Gizli"));
    let gunluk_metni = serde_json::to_string(&gunluk_satirlari(&d)).unwrap();
    assert!(!gunluk_metni.contains("Gizli") && !gunluk_metni.contains("1500"));
    assert_eq!(olay_sayisi(&d, "kayit.alindi"), 1);
    assert_eq!(olay_sayisi(&d, "kayit.zaten_var"), 1);
}

#[test]
fn ayni_no_farkli_icerik_cakisma_kutusuna() {
    let d = dugum();
    let ilk = json!({"no": "A-1", "tutar": 10});
    let ikinci = json!({"no": "A-1", "tutar": 99});
    d.al("/v1/kasa/sayim", "A-1", ilk.clone(), "a").unwrap();
    assert_eq!(
        d.al("/v1/kasa/sayim", "A-1", ikinci.clone(), "b").unwrap(),
        Yazma::Cakisma
    );
    // Aynı kaybeden yeniden gelirse kutu büyümez.
    assert_eq!(
        d.al("/v1/kasa/sayim", "A-1", ikinci.clone(), "b").unwrap(),
        Yazma::Cakisma
    );
    // Aynı no başka uca: o da çakışmadır.
    assert_eq!(
        d.al("/v1/fis/olay", "A-1", ilk.clone(), "b").unwrap(),
        Yazma::Cakisma
    );
    let kutu: Vec<Value> = taa_dosya::satirlari_oku(&d.cakisma_yolu())
        .unwrap()
        .iter()
        .map(|s| serde_json::from_str(s).unwrap())
        .collect();
    assert_eq!(kutu.len(), 2);
    assert_eq!(kutu[0]["gecerli"], ilk);
    assert_eq!(kutu[0]["kaybeden"], ikinci);
    // Düğümdeki (ilk) hal geçerli kaldı ve merkeze yalnız o gider.
    assert_eq!(d.kuyruk().kayitlar().unwrap().len(), 1);
    let m = SahteMerkez::default();
    m.acik.store(true, Ordering::SeqCst);
    tur(&d, &m).unwrap();
    assert_eq!(m.gelen.lock().unwrap()[0].1, ilk);
    assert_eq!(d.durum().unwrap().cakisma, 2);
}

#[test]
fn merkez_yokken_bekler_gelince_sirayla_gonderir() {
    let d = dugum();
    for n in ["K-1", "K-2", "K-3"] {
        d.al("/v1/fis/olay", n, json!({"no": n}), "x").unwrap();
    }
    let m = SahteMerkez::default();
    let o = tur(&d, &m).unwrap();
    assert!(!o.ulasti);
    assert_eq!((o.gonderilen, o.kalan), (0, 3));
    assert!(o.durma_nedeni.is_some());
    assert_eq!(d.durum().unwrap().bekleyen, 3);
    assert_eq!(d.durum().unwrap().son_merkez_zamani, None);
    // Yalnız ilk kayıt denendi: sıra bozulmasın diye gönderim orada durdu.
    assert_eq!(*m.deneme.lock().unwrap(), 1);

    // Merkez geldi ama K-2'de geçici hata: K-1 gider, K-2'de durur, K-3 K-2'yi geçemez.
    m.acik.store(true, Ordering::SeqCst);
    *m.gecici.lock().unwrap() = Some("K-2".into());
    let o = tur(&d, &m).unwrap();
    assert_eq!((o.gonderilen, o.kalan), (1, 2));
    assert_eq!(m.nolar(), ["K-1"]);

    *m.gecici.lock().unwrap() = None;
    let o = tur(&d, &m).unwrap();
    assert_eq!((o.gonderilen, o.kalan), (2, 0));
    assert_eq!(m.nolar(), ["K-1", "K-2", "K-3"]);
    let s = d.durum().unwrap();
    assert_eq!((s.bekleyen, s.gonderilen, s.reddedilen), (0, 3, 0));
    assert!(s.son_merkez_zamani.is_some());
    assert_eq!(olay_sayisi(&d, "merkez.gonderildi"), 3);

    // Yeniden tur: hiçbir şey ikinci kez gitmez.
    tur(&d, &m).unwrap();
    assert_eq!(m.nolar().len(), 3);
    // Gönderildikten sonra aynı kayıt yeniden gelirse: zaten var ve kesin.
    assert_eq!(
        d.al("/v1/fis/olay", "K-1", json!({"no": "K-1"}), "x")
            .unwrap(),
        Yazma::ZatenVar {
            surum: 1,
            kesin: true
        }
    );
}

#[test]
fn merkez_kalici_reddi_kuyrugu_tikamaz_ve_gorunur() {
    let d = dugum();
    for n in ["R-1", "R-2"] {
        d.al("/v1/fis/olay", n, json!({"no": n}), "x").unwrap();
    }
    let m = SahteMerkez::default();
    m.acik.store(true, Ordering::SeqCst);
    m.kalici_ret.lock().unwrap().push("R-1".into());
    let o = tur(&d, &m).unwrap();
    assert_eq!((o.gonderilen, o.reddedilen, o.kalan), (1, 1, 0));
    let s = d.durum().unwrap();
    assert_eq!((s.bekleyen, s.gonderilen, s.reddedilen), (0, 1, 1));
    let e = d.esitle(0, 10).unwrap();
    let k = e["belge"]["kayitlar"].as_array().unwrap();
    let r1 = k.iter().find(|x| x["no"] == "R-1").unwrap();
    assert_eq!(r1["durum"], "merkez_reddetti");
    assert_eq!(r1["kesin"], false);
    assert_eq!(olay_sayisi(&d, "merkez.reddetti"), 1);
}

#[test]
fn uyarlama_401_bekler_409_kalici() {
    // 401: kalıcı ret değil.
    let r = merkez::uyarla(Err(GonderHata::Reddedildi {
        kod: 401,
        mesaj: "oturum".into(),
    }));
    assert!(matches!(r, Err(GonderHata::Ulasilamadi(_))));
    // 409 çakışma: kalıcı.
    let r = merkez::uyarla(Err(GonderHata::Ulasilamadi(
        "HTTP 409: {\"hata\": \"cakisma\", \"mesaj\": \"x\"}. Öneri: ...".into(),
    )));
    assert!(matches!(r, Err(GonderHata::Reddedildi { kod: 409, .. })));
    // Yedekli göndericinin sarmaladığı ileti de tanınır.
    let r = merkez::uyarla(Err(GonderHata::Ulasilamadi(
        "tüm yollar denendi (a: sunucuya ulaşılamadı: HTTP 409: {\"hata\":\"sube_uyusmaz\"}; b: ağ)".into(),
    )));
    assert!(matches!(r, Err(GonderHata::Reddedildi { kod: 409, .. })));
    // Başka 409 (taa-kuyruk `tekrar`) ve 503 beklemeye devam eder; 422 kalıcı kalır; başarı değişmez.
    for m in ["HTTP 409: {\"hata\":\"tekrar\"}", "HTTP 503: cakisma"] {
        let r = merkez::uyarla(Err(GonderHata::Ulasilamadi(m.into())));
        assert!(matches!(r, Err(GonderHata::Ulasilamadi(_))), "{m}");
    }
    let r = merkez::uyarla(Err(GonderHata::Reddedildi {
        kod: 422,
        mesaj: "x".into(),
    }));
    assert!(matches!(r, Err(GonderHata::Reddedildi { kod: 422, .. })));
    assert_eq!(merkez::uyarla(Ok(json!(1))), Ok(json!(1)));

    // Kuyrukta: 401 veren merkez hiçbir kaydı reddettirmez.
    struct Oturumsuz;
    impl Gonderici for Oturumsuz {
        fn gonder(&self, _u: &str, _g: &Value) -> Result<Value, GonderHata> {
            Err(GonderHata::Reddedildi {
                kod: 401,
                mesaj: "oturum doldu".into(),
            })
        }
    }
    let d = dugum();
    d.al("/v1/fis/olay", "O-1", json!({"no": "O-1"}), "x")
        .unwrap();
    let o = tur(&d, &Oturumsuz).unwrap();
    assert_eq!((o.reddedilen, o.kalan), (0, 1));
    assert_eq!(d.durum().unwrap().reddedilen, 0);
}

#[test]
fn esitle_delta_surum_ve_sayfalama() {
    let d = dugum();
    for n in ["E-1", "E-2", "E-3"] {
        d.al("/v1/fis/olay", n, json!({"no": n, "x": n}), "x")
            .unwrap();
    }
    let e = d.esitle(0, 2).unwrap();
    let b = &e["belge"];
    assert_eq!(b["sube"], "MRK01");
    assert_eq!(b["kayitlar"].as_array().unwrap().len(), 2);
    assert_eq!(b["devami"], true);
    assert_eq!(b["surum"], 2);
    assert_eq!(b["kayitlar"][0]["anahtar"], "fis/olay/E-1");
    assert_eq!(b["kayitlar"][0]["deger"], json!({"no": "E-1", "x": "E-1"}));
    assert_eq!(b["kayitlar"][0]["durum"], "bekliyor");
    assert!(e["imza"].is_null());
    let e = d.esitle(2, 2).unwrap();
    assert_eq!(e["belge"]["kayitlar"].as_array().unwrap().len(), 1);
    assert_eq!(e["belge"]["devami"], false);
    assert_eq!(e["belge"]["surum"], 3);
    // Değişiklik yok: boş delta, surum = sonra.
    let e = d.esitle(3, 50).unwrap();
    assert!(e["belge"]["kayitlar"].as_array().unwrap().is_empty());
    assert_eq!(e["belge"]["surum"], 3);
    // Merkeze gidince kayıtlar yeni sürümle yeniden yayılır (durum değişti).
    let m = SahteMerkez::default();
    m.acik.store(true, Ordering::SeqCst);
    tur(&d, &m).unwrap();
    let e = d.esitle(3, 50).unwrap();
    let k = e["belge"]["kayitlar"].as_array().unwrap();
    assert_eq!(k.len(), 3);
    assert!(k
        .iter()
        .all(|x| x["durum"] == "gonderildi" && x["kesin"] == true));
    assert_eq!(e["belge"]["surum"], 6);
}

#[test]
fn cokus_sonrasi_surum_sayaci_onarilir() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    // Kuyruğa yazıldı ama sürüm sayacına yazılamadan çöktü.
    taa_kuyruk::Kuyruk::program(&kok, KUYRUK_PROGRAM)
        .unwrap()
        .ekle("/v1/fis/olay", "C-1", json!({"no": "C-1"}))
        .unwrap();
    let d = Dugum::ac(&kok, "MRK01").unwrap();
    assert_eq!(d.durum().unwrap().surum, 1);
    assert_eq!(
        d.esitle(0, 10).unwrap()["belge"]["kayitlar"][0]["no"],
        "C-1"
    );
    // Yeniden açılış ikinci satır eklemez.
    drop(d);
    let d = Dugum::ac(&kok, "MRK01").unwrap();
    assert_eq!(d.durum().unwrap().surum, 1);
    assert!(Dugum::ac(&kok, "mrk 01").is_err());
}

#[test]
fn gunluk_yalniz_eklenir_ve_ozeti_kuyruga_girer() {
    let d = dugum();
    d.al("/v1/fis/olay", "G-1", json!({"no": "G-1"}), "x")
        .unwrap();
    let bugun = gunluk::bugun();
    let yol = gunluk::dosya(d.kok(), &bugun);
    let once = std::fs::read(&yol).unwrap();
    d.al("/v1/fis/olay", "G-2", json!({"no": "G-2"}), "x")
        .unwrap();
    let sonra = std::fs::read(&yol).unwrap();
    assert!(sonra.len() > once.len() && sonra.starts_with(&once));

    // Dünün günlüğü (elle): özeti kuyruğa girer; bugünün özeti girmez.
    let dun = "2026-11-26";
    let dun_yolu = gunluk::dosya(d.kok(), dun);
    taa_dosya::satir_ekle(
        &dun_yolu,
        r#"{"zaman":"2026-11-26T09:00:00Z","olay":"kayit.alindi","no":"D-1"}"#,
    )
    .unwrap();
    taa_dosya::satir_ekle(
        &dun_yolu,
        r#"{"zaman":"2026-11-26T09:05:00Z","olay":"kayit.alindi","no":"D-2"}"#,
    )
    .unwrap();
    let dun_bayt = std::fs::read(&dun_yolu).unwrap();
    assert_eq!(gunluk::ozetleri_kuyrukla(&d, &bugun).unwrap(), [dun]);
    assert!(gunluk::ozetleri_kuyrukla(&d, &bugun).unwrap().is_empty());
    let k = d.gunluk_kuyruk().kayitlar().unwrap();
    assert_eq!(k.len(), 1);
    assert_eq!(k[0].uc, gunluk::OZET_UCU);
    assert_eq!(k[0].no, "GNL-MRK01-2026-11-26");
    assert_eq!(k[0].govde["satir"], 2);
    assert_eq!(k[0].govde["bayt"], dun_bayt.len());
    assert_eq!(k[0].govde["olaylar"]["kayit.alindi"], 2);
    assert_eq!(k[0].govde["ilk"], "2026-11-26T09:00:00Z");
    assert_eq!(k[0].govde["sha256"].as_str().unwrap().len(), 64);
    assert_eq!(d.durum().unwrap().gunluk_bekleyen, 1);

    // Merkeze ulaşınca özet gider; günlük dosyaları yerinde ve aynı kalır.
    let m = SahteMerkez::default();
    m.acik.store(true, Ordering::SeqCst);
    let o = tur(&d, &m).unwrap();
    assert_eq!((o.gonderilen, o.gunluk_gonderilen), (2, 1));
    assert!(m
        .gelen
        .lock()
        .unwrap()
        .iter()
        .any(|(uc, g)| uc == gunluk::OZET_UCU && g["gun"] == dun && g["sube"] == "MRK01"));
    assert_eq!(std::fs::read(&dun_yolu).unwrap(), dun_bayt);
    assert!(std::fs::read(&yol).unwrap().starts_with(&sonra));
    assert_eq!(d.durum().unwrap().gunluk_bekleyen, 0);
    // Özet kuyruğu asıl kayıtların sürümüne ve eşitleme deltasına karışmaz.
    assert_eq!(
        d.esitle(0, 50).unwrap()["belge"]["kayitlar"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn bos_kuyrukta_merkez_yoklanir() {
    let d = dugum();
    let m = SahteMerkez::default();
    assert!(!tur(&d, &m).unwrap().ulasti);
    assert_eq!(d.son_merkez_zamani(), None);
    m.acik.store(true, Ordering::SeqCst);
    assert!(tur(&d, &m).unwrap().ulasti);
    assert!(d.son_merkez_zamani().is_some());
}

#[test]
fn merkez_basliklari_ve_cihaz_kancasi() {
    struct SahteCihaz;
    impl Imzalayici for SahteCihaz {
        fn basliklar(
            &self,
            y: &str,
            yol: &str,
            _g: &[u8],
        ) -> Result<Vec<(String, String)>, String> {
            Ok(vec![("X-Deneme".into(), format!("{y} {yol}"))])
        }
    }
    // Kanca gerçek gönderici üstünde: başlıklar curl'e gitmeden önce üretilir; burada yalnız kurulum sınanır.
    let g = taa_kuyruk::CurlGonderici::yeni("https://merkez.ornek").unwrap();
    assert!(!g.imzali_mi());
    let g = merkez::baslikli(g, "MRK01", Some(Arc::new(SahteCihaz)));
    assert!(g.imzali_mi());

    let kok = taa_dosya::gecici_klasor().unwrap();
    // Adres de yol listesi de yok: gönderici kurulmaz (kayıtlar birikir).
    assert!(merkez::gonderici_kur(&kok, "MRK01", None, None, None, None)
        .unwrap()
        .is_none());
    assert!(merkez::gonderici_kur(
        &kok,
        "MRK01",
        Some("https://m.ornek"),
        None,
        Some("anahtar"),
        None
    )
    .unwrap()
    .is_some());
    assert!(merkez::gonderici_kur(&kok, "MRK01", Some("ftp://m.ornek"), None, None, None).is_err());
    // taa-yol yol listesi varsa oradan kurulur.
    let mut y = taa_yol::Yollar::yeni("merkez").unwrap();
    y.ekle(taa_yol::Yol::yeni("dogrudan", "https://10.0.0.1", Some("sistem")).unwrap())
        .unwrap();
    taa_yol::yollar_yaz(&kok, &y).unwrap();
    assert!(merkez::gonderici_kur(&kok, "MRK01", None, None, None, None)
        .unwrap()
        .is_some());
}

// ---------------------------------------------------------------- HTTP (soketsiz)

fn istek(yontem: &str, yol: &str, sube: Option<&str>, govde: Option<Value>) -> Istek {
    let mut b = vec![("host".to_string(), "127.0.0.1:8788".to_string())];
    if let Some(s) = sube {
        b.push(("x-taa-sube".into(), s.into()));
    }
    Istek {
        yontem: yontem.into(),
        yol: yol.into(),
        basliklar: b,
        govde: govde
            .map(|g| g.to_string().into_bytes())
            .unwrap_or_default(),
        uzak: Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 7))),
        govde_asildi: false,
    }
}

fn cagir(d: &Dugum, i: &Istek) -> Yanit {
    isle(d, &AgAyari::yerel(), &DenetimYok, i)
}

fn hata_kodu(y: &Yanit) -> String {
    y.govde.as_ref().unwrap()["hata"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

#[test]
fn http_yazma_ve_no_kurallari() {
    let d = dugum();
    let mut i = istek(
        "POST",
        "/v1/kasa/sayim",
        Some("MRK01"),
        Some(json!({"tutar": 5})),
    );
    // no hiç yok
    let y = cagir(&d, &i);
    assert_eq!((y.kod, hata_kodu(&y).as_str()), (400, "no_eksik"));
    // no yalnız başlıkta: gövdeye eklenir (merkez ile aynı gövde)
    i.basliklar.push(("x-taa-no".into(), "H-1".into()));
    let y = cagir(&d, &i);
    assert_eq!(y.kod, 200);
    let g = y.govde.unwrap();
    assert_eq!(
        (g["durum"].as_str(), g["kesin"].as_bool()),
        (Some("yazildi"), Some(false))
    );
    assert_eq!(g["no"], "H-1");
    assert_eq!(
        d.esitle(0, 5).unwrap()["belge"]["kayitlar"][0]["deger"],
        json!({"no": "H-1", "tutar": 5})
    );
    // aynı istek yeniden: zaten_var, tek kayıt
    let y = cagir(&d, &i);
    assert_eq!(y.govde.unwrap()["durum"], "zaten_var");
    assert_eq!(d.kuyruk().kayitlar().unwrap().len(), 1);
    // başlık ve gövde farklı
    i.govde = json!({"no": "H-2", "tutar": 5}).to_string().into_bytes();
    let y = cagir(&d, &i);
    assert_eq!((y.kod, hata_kodu(&y).as_str()), (400, "no_uyusmaz"));
    // aynı no, farklı içerik
    i.govde = json!({"no": "H-1", "tutar": 6}).to_string().into_bytes();
    let y = cagir(&d, &i);
    assert_eq!((y.kod, hata_kodu(&y).as_str()), (409, "cakisma"));
    // bozuk no, bozuk gövde
    let y = cagir(
        &d,
        &istek(
            "POST",
            "/v1/kasa/sayim",
            Some("MRK01"),
            Some(json!({"no": "a b"})),
        ),
    );
    assert_eq!((y.kod, hata_kodu(&y).as_str()), (400, "no_bozuk"));
    let mut b = istek("POST", "/v1/kasa/sayim", Some("MRK01"), None);
    b.govde = b"[1,2]".to_vec();
    assert_eq!(cagir(&d, &b).kod, 400);
    // gövde sınırı
    b.govde_asildi = true;
    assert_eq!(cagir(&d, &b).kod, 413);
    assert_eq!(d.kuyruk().kayitlar().unwrap().len(), 1);
}

#[test]
fn http_sube_kapsami() {
    let d = dugum();
    let g = json!({"no": "S-1"});
    // Başka şube başlığı
    let y = cagir(
        &d,
        &istek("POST", "/v1/fis/olay", Some("IZM01"), Some(g.clone())),
    );
    assert_eq!((y.kod, hata_kodu(&y).as_str()), (409, "sube_uyusmaz"));
    // Mesaj öteki şubenin adını taşımaz
    assert!(!y.govde.unwrap().to_string().contains("IZM01"));
    // Başlık yok
    let y = cagir(&d, &istek("POST", "/v1/fis/olay", None, Some(g.clone())));
    assert_eq!((y.kod, hata_kodu(&y).as_str()), (400, "sube_eksik"));
    // Başlık doğru ama gövde başka şubeyi gösteriyor
    let y = cagir(
        &d,
        &istek(
            "POST",
            "/v1/fis/olay",
            Some("MRK01"),
            Some(json!({"no": "S-1", "sube": "IZM01"})),
        ),
    );
    assert_eq!((y.kod, hata_kodu(&y).as_str()), (409, "sube_uyusmaz"));
    // Okuma da şube kapsamlı
    assert_eq!(
        cagir(&d, &istek("GET", "/v1/esitle?sonra=0", Some("IZM01"), None)).kod,
        409
    );
    assert_eq!(
        cagir(&d, &istek("GET", "/v1/esitle?sonra=0", None, None)).kod,
        400
    );
    // Hiçbiri yazılmadı; hepsi günlükte
    assert!(d.kuyruk().kayitlar().unwrap().is_empty());
    assert_eq!(olay_sayisi(&d, "dugum.red"), 5);
    // Doğru şube
    let y = cagir(
        &d,
        &istek(
            "POST",
            "/v1/fis/olay",
            Some("MRK01"),
            Some(json!({"no": "S-1", "sube": "MRK01"})),
        ),
    );
    assert_eq!(y.kod, 200);
    let y = cagir(
        &d,
        &istek("GET", "/v1/esitle?sonra=0&limit=10", Some("MRK01"), None),
    );
    assert_eq!(y.kod, 200);
    assert_eq!(y.govde.unwrap()["belge"]["kayitlar"][0]["no"], "S-1");
    assert_eq!(
        cagir(&d, &istek("GET", "/v1/esitle?sonra=x", Some("MRK01"), None)).kod,
        400
    );
}

#[test]
fn http_kapali_uclar_parola_diske_yazilmaz() {
    let d = dugum();
    for uc in [
        "/v1/giris",
        "/v1/yetki",
        "/v1/cihaz/kayit",
        "/v1/esitle",
        "/v1/gunluk/ozet",
        "/v1/",
        "/v1",
        "/api/x",
        "/v1/a/../b",
    ] {
        let y = cagir(
            &d,
            &istek(
                "POST",
                uc,
                Some("MRK01"),
                Some(json!({"no": "P-1", "parola": "cokgizli"})),
            ),
        );
        assert_eq!((y.kod, hata_kodu(&y).as_str()), (404, "uc_yok"), "{uc}");
    }
    assert!(d.kuyruk().kayitlar().unwrap().is_empty());
    assert!(!serde_json::to_string(&gunluk_satirlari(&d))
        .unwrap()
        .contains("cokgizli"));
    assert!(uc_kabul("/v1/kasa/sayim") && uc_kabul("/v1/girisim"));
    assert_eq!(cagir(&d, &istek("GET", "/yok", None, None)).kod, 404);
    assert_eq!(
        cagir(&d, &istek("DELETE", "/v1/fis/olay", Some("MRK01"), None)).kod,
        405
    );
}

#[test]
fn http_durum_ve_saglik() {
    let d = dugum();
    d.al("/v1/fis/olay", "D-1", json!({"no": "D-1"}), "x")
        .unwrap();
    let y = cagir(&d, &istek("GET", "/durum", None, None));
    assert_eq!(y.kod, 200);
    let g = y.govde.unwrap();
    assert_eq!(g["bekleyen"], 1);
    assert_eq!(g["gonderilen"], 0);
    assert_eq!(g["reddedilen"], 0);
    assert!(g["son_merkez_zamani"].is_null());
    assert!(g.as_object().unwrap().contains_key("son_merkez_zamani"));
    assert_eq!(cagir(&d, &istek("GET", "/v1/saglik", None, None)).kod, 200);
}

#[test]
fn http_ag_ad_ve_koken_sinirlari() {
    let d = dugum();
    let iyi = || {
        istek(
            "POST",
            "/v1/fis/olay",
            Some("MRK01"),
            Some(json!({"no": "N-1"})),
        )
    };
    // Yerel ağ dışından (genel adres, Tailscale) ve adresi bilinmeyen bağlantı
    for ip in [Some("8.8.8.8"), Some("100.100.1.2"), None] {
        let mut i = iyi();
        i.uzak = ip.map(|s| s.parse().unwrap());
        let y = cagir(&d, &i);
        assert_eq!((y.kod, hata_kodu(&y).as_str()), (403, "ag_disi"), "{ip:?}");
    }
    // Yabancı Host (DNS yeniden bağlama)
    let mut i = iyi();
    i.basliklar[0].1 = "kotu.ornek.com:8788".into();
    let y = cagir(&d, &i);
    assert_eq!((y.kod, hata_kodu(&y).as_str()), (403, "ad_uyusmaz"));
    // Yabancı köken
    let mut i = iyi();
    i.basliklar
        .push(("origin".into(), "https://kotu.ornek.com".into()));
    let y = cagir(&d, &i);
    assert_eq!((y.kod, hata_kodu(&y).as_str()), (403, "koken_izinsiz"));
    assert!(d.kuyruk().kayitlar().unwrap().is_empty());

    // İzinli köken (merkez kabuğu) ve LAN adı
    let mut ag = AgAyari::yerel();
    ag.ad_ekle("192.168.1.20");
    assert!(ag.koken_ekle("https://Sunucu.ornek.com/"));
    assert!(
        !ag.koken_ekle("sunucu.ornek.com") && !ag.koken_ekle("https://a/b") && !ag.koken_ekle("*")
    );
    let mut i = iyi();
    i.basliklar[0].1 = "192.168.1.20:8788".into();
    i.basliklar
        .push(("origin".into(), "https://sunucu.ornek.com".into()));
    let y = isle(&d, &ag, &DenetimYok, &i);
    assert_eq!(y.kod, 200);
    assert!(y.basliklar.contains(&(
        "Access-Control-Allow-Origin".into(),
        "https://sunucu.ornek.com".into()
    )));
    // Ön uçuş: şube başlığı olmadan da yanıtlanır
    let mut o = istek("OPTIONS", "/v1/fis/olay", None, None);
    o.basliklar[0].1 = "192.168.1.20:8788".into();
    o.basliklar
        .push(("origin".into(), "https://sunucu.ornek.com".into()));
    o.basliklar.push((
        "access-control-request-private-network".into(),
        "true".into(),
    ));
    let y = isle(&d, &ag, &DenetimYok, &o);
    assert_eq!(y.kod, 204);
    assert!(y
        .basliklar
        .iter()
        .any(|(a, v)| a == "Access-Control-Allow-Headers" && v.contains("X-TAA-No")));
    assert!(y
        .basliklar
        .iter()
        .any(|(a, _)| a == "Access-Control-Allow-Private-Network"));
    // Aynı köken: CORS başlığı yok, kabul
    let mut i = istek(
        "POST",
        "/v1/fis/olay",
        Some("MRK01"),
        Some(json!({"no": "N-2"})),
    );
    i.basliklar
        .push(("origin".into(), "http://127.0.0.1:8788".into()));
    let y = cagir(&d, &i);
    assert_eq!(y.kod, 200);
    assert!(y.basliklar.is_empty());

    // Dinleme adresi kuralları
    let gecerli = |s: &str| sunucu::lan_adresi_gecerli(s.parse().unwrap()).is_ok();
    assert!(
        gecerli("192.168.1.20")
            && gecerli("10.0.0.5")
            && gecerli("172.20.1.1")
            && gecerli("169.254.3.4")
    );
    assert!(
        !gecerli("0.0.0.0")
            && !gecerli("127.0.0.1")
            && !gecerli("100.64.0.1")
            && !gecerli("85.1.2.3")
            && !gecerli("172.32.0.1")
    );
    assert!(sunucu::dinle("0.0.0.0:0".parse().unwrap()).is_err());
    assert!(
        sunucu::uzak_izinli("::1".parse().unwrap())
            && sunucu::uzak_izinli("fe80::1".parse().unwrap())
    );
    assert!(sunucu::uzak_izinli("::ffff:192.168.1.5".parse().unwrap()));
    assert!(
        !sunucu::uzak_izinli("2001:db8::1".parse().unwrap())
            && !sunucu::uzak_izinli("::ffff:8.8.8.8".parse().unwrap())
    );
}

#[test]
fn http_denetim_kancasi() {
    struct Kapali;
    impl GelenDenetim for Kapali {
        fn denetle(&self, i: &GelenIstek) -> Result<(), Ret> {
            if i.basliklar
                .iter()
                .any(|(a, v)| a == "x-taa-cihaz" && v == "eslesmis")
            {
                Ok(())
            } else {
                Err(Ret::yeni(
                    401,
                    "cihaz_eslesmemis",
                    "Bu cihaz şube bilgisayarıyla eşleşmemiş.",
                ))
            }
        }
    }
    let d = dugum();
    let ag = AgAyari::yerel();
    let mut i = istek(
        "POST",
        "/v1/fis/olay",
        Some("MRK01"),
        Some(json!({"no": "X-1"})),
    );
    let y = isle(&d, &ag, &Kapali, &i);
    assert_eq!((y.kod, hata_kodu(&y).as_str()), (401, "cihaz_eslesmemis"));
    assert_eq!(
        isle(
            &d,
            &ag,
            &Kapali,
            &istek("GET", "/v1/esitle", Some("MRK01"), None)
        )
        .kod,
        401
    );
    assert!(d.kuyruk().kayitlar().unwrap().is_empty());
    i.basliklar.push(("x-taa-cihaz".into(), "eslesmis".into()));
    assert_eq!(isle(&d, &ag, &Kapali, &i).kod, 200);
    // Durum ucu denetim istemez (kişi verisi taşımaz).
    assert_eq!(
        isle(&d, &ag, &Kapali, &istek("GET", "/durum", None, None)).kod,
        200
    );
}

// ---------------------------------------------------------------- gerçek soket

fn http(
    adres: SocketAddr,
    yontem: &str,
    yol: &str,
    basliklar: &[(&str, &str)],
    govde: &[u8],
) -> (u16, Value) {
    let mut s = TcpStream::connect(adres).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    let mut m = format!(
        "{yontem} {yol} HTTP/1.1\r\nHost: {adres}\r\nConnection: close\r\nContent-Length: {}\r\n",
        govde.len()
    );
    for (a, d) in basliklar {
        m.push_str(&format!("{a}: {d}\r\n"));
    }
    m.push_str("\r\n");
    s.write_all(m.as_bytes()).unwrap();
    s.write_all(govde).unwrap();
    let mut y = Vec::new();
    let _ = s.read_to_end(&mut y);
    let y = String::from_utf8_lossy(&y).into_owned();
    let kod: u16 = y.split_whitespace().nth(1).unwrap().parse().unwrap();
    let govde = y.split_once("\r\n\r\n").map(|x| x.1).unwrap_or("");
    (kod, serde_json::from_str(govde).unwrap_or(Value::Null))
}

/// Sahte merkezin kaydettiği istek: (yol, X-TAA-No, X-TAA-Sube, gövde).
type MerkezIstegi = (String, String, String, String);

fn curl_var() -> bool {
    std::process::Command::new("curl")
        .arg("--version")
        .output()
        .map(|c| c.status.success())
        .unwrap_or(false)
}

/// Uçtan uca: telefon → düğüm (gerçek HTTP) → merkez yok → merkez geldi (gerçek HTTP, curl) → tek kayıt.
#[test]
fn uctan_uca_soket_ve_merkez() {
    let d = Arc::new(dugum());
    let dur = Arc::new(AtomicBool::new(false));
    let sn = Arc::new(sunucu::dinle("127.0.0.1:0".parse().unwrap()).unwrap());
    let adres = sn.server_addr().to_ip().unwrap();
    let is = {
        let (sn, d, dur) = (sn.clone(), d.clone(), dur.clone());
        std::thread::spawn(move || sunucu::hizmet(&sn, &d, &AgAyari::yerel(), &DenetimYok, &dur))
    };
    let govde = json!({"no": "U-1", "tutar": 4200}).to_string();
    let b = [
        ("X-TAA-Sube", "MRK01"),
        ("X-TAA-No", "U-1"),
        ("Content-Type", "application/json"),
    ];
    let (kod, y) = http(adres, "POST", "/v1/kasa/sayim", &b, govde.as_bytes());
    assert_eq!((kod, y["durum"].as_str()), (200, Some("yazildi")));
    let (kod, y) = http(adres, "POST", "/v1/kasa/sayim", &b, govde.as_bytes());
    assert_eq!((kod, y["durum"].as_str()), (200, Some("zaten_var")));
    let (kod, _) = http(
        adres,
        "POST",
        "/v1/kasa/sayim",
        &[("X-TAA-Sube", "IZM01")],
        govde.as_bytes(),
    );
    assert_eq!(kod, 409);
    // Gövde sınırı gerçek sokette
    let buyuk = vec![b'a'; GOVDE_EN_COK + 10];
    let (kod, _) = http(adres, "POST", "/v1/kasa/sayim", &b, &buyuk);
    assert_eq!(kod, 413);
    let (kod, y) = http(adres, "GET", "/durum", &[], b"");
    assert_eq!(
        (kod, y["bekleyen"].as_u64(), y["gonderilen"].as_u64()),
        (200, Some(1), Some(0))
    );
    let (kod, y) = http(
        adres,
        "GET",
        "/v1/esitle?sonra=0",
        &[("X-TAA-Sube", "MRK01")],
        b"",
    );
    assert_eq!(kod, 200);
    assert_eq!(y["belge"]["kayitlar"][0]["deger"]["tutar"], 4200);

    if !curl_var() {
        eprintln!("curl yok: merkeze gerçek gönderim adımı atlandı");
        dur.store(true, Ordering::SeqCst);
        is.join().unwrap();
        return;
    }
    // Merkez yok: kapalı kapı.
    let bos = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let kapali = format!("http://{}", bos.local_addr().unwrap());
    drop(bos);
    let g = merkez::gonderici_kur(d.kok(), "MRK01", Some(&kapali), None, None, None)
        .unwrap()
        .unwrap();
    let o = tur(&d, g.as_ref()).unwrap();
    assert!(!o.ulasti);
    assert_eq!(o.kalan, 1);

    // Merkez geldi: tiny_http ile sahte merkez; başlıkları ve gövdeyi kaydeder.
    let msn = Arc::new(tiny_http::Server::http("127.0.0.1:0").unwrap());
    let madres = msn.server_addr().to_ip().unwrap();
    let gelen: Arc<Mutex<Vec<MerkezIstegi>>> = Arc::new(Mutex::new(Vec::new()));
    let mis = {
        let (msn, gelen, dur) = (msn.clone(), gelen.clone(), dur.clone());
        std::thread::spawn(move || {
            while !dur.load(Ordering::SeqCst) {
                let Ok(Some(mut r)) = msn.recv_timeout(Duration::from_millis(100)) else {
                    continue;
                };
                let al = |r: &tiny_http::Request, ad: &str| {
                    r.headers()
                        .iter()
                        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(ad))
                        .map(|h| h.value.as_str().to_string())
                        .unwrap_or_default()
                };
                let (no, sube) = (al(&r, "X-TAA-No"), al(&r, "X-TAA-Sube"));
                let mut g = String::new();
                let _ = r.as_reader().read_to_string(&mut g);
                if r.method() == &tiny_http::Method::Post {
                    gelen
                        .lock()
                        .unwrap()
                        .push((r.url().to_string(), no, sube, g));
                }
                let _ = r.respond(tiny_http::Response::from_string("{\"durum\":\"yazildi\"}"));
            }
        })
    };
    let g = merkez::gonderici_kur(
        d.kok(),
        "MRK01",
        Some(&format!("http://{madres}")),
        None,
        None,
        None,
    )
    .unwrap()
    .unwrap();
    let o = tur(&d, g.as_ref()).unwrap();
    assert!(o.ulasti);
    assert_eq!((o.gonderilen, o.kalan), (1, 0));
    tur(&d, g.as_ref()).unwrap();
    {
        let gl = gelen.lock().unwrap();
        assert_eq!(gl.len(), 1, "merkeze tek kayıt gitmeli");
        assert_eq!(
            (gl[0].0.as_str(), gl[0].1.as_str(), gl[0].2.as_str()),
            ("/v1/kasa/sayim", "U-1", "MRK01")
        );
        assert_eq!(
            serde_json::from_str::<Value>(&gl[0].3).unwrap(),
            json!({"no": "U-1", "tutar": 4200})
        );
    }
    let (_, y) = http(adres, "GET", "/durum", &[], b"");
    assert_eq!(
        (y["bekleyen"].as_u64(), y["gonderilen"].as_u64()),
        (Some(0), Some(1))
    );
    assert!(y["son_merkez_zamani"].is_string());
    dur.store(true, Ordering::SeqCst);
    is.join().unwrap();
    mis.join().unwrap();
}

/// Arka plan döngüsü: merkez yokken bekler, yeni kayıtla uyanır, merkez gelince gönderir.
#[test]
fn dongu_bekler_ve_uyanir() {
    let d = Arc::new(dugum());
    let m = Arc::new(SahteMerkez::default());
    let dur = Arc::new(AtomicBool::new(false));
    struct Vekil(Arc<SahteMerkez>);
    impl Gonderici for Vekil {
        fn gonder(&self, u: &str, g: &Value) -> Result<Value, GonderHata> {
            self.0.gonder(u, g)
        }
        fn al(&self, u: &str) -> Result<Value, GonderHata> {
            self.0.al(u)
        }
    }
    let is = {
        let (d, m, dur) = (d.clone(), m.clone(), dur.clone());
        std::thread::spawn(move || {
            let kur = || Some(Box::new(Vekil(m.clone())) as Box<dyn Gonderici>);
            // Aralık uzun: testin geçmesi uyandırmaya bağlı.
            merkez::dongu(&d, &kur, Duration::from_secs(600), &dur)
        })
    };
    let bekle = |kosul: &dyn Fn() -> bool| {
        let bas = Instant::now();
        while !kosul() {
            assert!(bas.elapsed() < Duration::from_secs(20), "zaman aşımı");
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    d.al("/v1/fis/olay", "L-1", json!({"no": "L-1"}), "x")
        .unwrap();
    bekle(&|| *m.deneme.lock().unwrap() >= 1);
    assert_eq!(d.durum().unwrap().bekleyen, 1);
    m.acik.store(true, Ordering::SeqCst);
    d.al("/v1/fis/olay", "L-2", json!({"no": "L-2"}), "x")
        .unwrap();
    bekle(&|| d.durum().unwrap().gonderilen == 2);
    assert_eq!(m.nolar(), ["L-1", "L-2"]);
    bekle(&|| olay_sayisi(&d, "merkez.ulasildi") == 1);
    assert_eq!(olay_sayisi(&d, "merkez.ulasilamadi"), 1);
    dur.store(true, Ordering::SeqCst);
    d.uyandir();
    is.join().unwrap();
}
