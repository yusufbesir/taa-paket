// Sahip: Yusuf. yazan: [Claude#taa-kuyruk-T6 2026-11-06]
//! taa-kuyruk testleri: ekleme/idempotentlik, sıra, durma, ret; curl göndericisi yerel HTTP sunucusuyla.

use super::*;
use std::cell::RefCell;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

fn gecici() -> PathBuf {
    taa_dosya::gecici_klasor().unwrap()
}

/// Sahte gönderici: verilen senaryoya göre yanıt verir, çağrıları kaydeder.
struct Sahte {
    senaryo: RefCell<Vec<Result<Value, GonderHata>>>,
    cagrilar: RefCell<Vec<(String, Value)>>,
}

impl Sahte {
    fn yeni(s: Vec<Result<Value, GonderHata>>) -> Sahte {
        Sahte {
            senaryo: RefCell::new(s),
            cagrilar: RefCell::new(Vec::new()),
        }
    }
}

impl Gonderici for Sahte {
    fn gonder(&self, uc: &str, govde: &Value) -> Result<Value, GonderHata> {
        self.cagrilar
            .borrow_mut()
            .push((uc.to_string(), govde.clone()));
        let mut s = self.senaryo.borrow_mut();
        if s.is_empty() {
            Ok(json!({"id": 1}))
        } else {
            s.remove(0)
        }
    }
}

#[test]
fn dogrulamalar() {
    assert!(no_gecerli("S1-2026-000123"));
    assert!(no_gecerli("a.b_c"));
    assert!(!no_gecerli(""));
    assert!(!no_gecerli("ab c"));
    assert!(!no_gecerli("..") && !no_gecerli("."));
    assert!(!no_gecerli(&"x".repeat(65)));
    assert!(uc_gecerli("/v1/olay"));
    assert!(uc_gecerli("/v1/etiket-kuyrugu"));
    assert!(!uc_gecerli("v1/olay"));
    assert!(!uc_gecerli("/v1/../olay"));
    assert!(!uc_gecerli("/v1//olay"));
    assert!(!uc_gecerli("/v1/olay?x=1"));
    assert!(program_gecerli("taa-fis") && !program_gecerli("TaaFis") && !program_gecerli(""));
}

#[test]
fn ekle_ve_idempotent() {
    let k = Kuyruk::ac(gecici().join("p"));
    assert!(k.bekleyenler().unwrap().is_empty());
    let e = k
        .ekle("/v1/olay", "S1-1", json!({"tur": "kod_okundu"}))
        .unwrap();
    assert!(matches!(e, Ekleme::Eklendi(_)));
    let e2 = k.ekle("/v1/olay", "S1-1", json!({"tur": "baska"})).unwrap();
    match e2 {
        Ekleme::ZatenVar(eski) => assert_eq!(eski.govde, json!({"tur": "kod_okundu"})),
        _ => panic!("aynı no ikinci kez eklendi"),
    }
    assert_eq!(k.kayitlar().unwrap().len(), 1);
    assert_eq!(
        taa_dosya::satirlari_oku(&k.klasor().join("giden.jsonl"))
            .unwrap()
            .len(),
        1
    );
    assert!(matches!(
        k.ekle("/v1/olay", "kötü no", json!({})),
        Err(KuyrukHata::GecersizNo(_))
    ));
    assert!(matches!(
        k.ekle("olay", "S1-2", json!({})),
        Err(KuyrukHata::GecersizUc(_))
    ));
    assert!(matches!(
        k.ekle("/v1/olay", "S1-2", json!([1])),
        Err(KuyrukHata::GecersizGovde(_))
    ));
    assert_eq!(
        k.kayitlar().unwrap().len(),
        1,
        "hatalı ekleme satır yazmamalı"
    );
}

#[test]
fn gonder_sira_ret_ve_durma() {
    let k = Kuyruk::ac(gecici().join("p"));
    for i in 1..=4 {
        k.ekle("/v1/olay", &format!("N-{i}"), json!({"i": i}))
            .unwrap();
    }
    // 1 kabul, 2 kalıcı ret, 3 ulaşılamadı → durur; 4 beklemede kalır.
    let g = Sahte::yeni(vec![
        Ok(json!({"id": 10})),
        Err(GonderHata::Reddedildi {
            kod: 422,
            mesaj: "alan eksik".into(),
        }),
        Err(GonderHata::Ulasilamadi("ağ yok".into())),
    ]);
    let o = k.gonder(&g, 0).unwrap();
    assert_eq!(o.gonderilen, 1);
    assert_eq!(o.reddedilen, 1);
    assert_eq!(o.kalan, 2);
    assert!(o.durma_nedeni.unwrap().contains("ağ yok"));
    assert_eq!(g.cagrilar.borrow().len(), 3);
    assert_eq!(g.cagrilar.borrow()[0].1, json!({"i": 1}));
    let d: Vec<Durum> = k.durumlar().unwrap().into_iter().map(|(_, d)| d).collect();
    assert_eq!(
        d,
        vec![
            Durum::Gonderildi,
            Durum::Reddedildi,
            Durum::Bekliyor,
            Durum::Bekliyor
        ]
    );
    assert_eq!(k.gonderilenler().unwrap()["N-1"].yanit, json!({"id": 10}));
    let r = &k.reddedilenler().unwrap()["N-2"];
    assert_eq!((r.kod, r.mesaj.as_str()), (422, "alan eksik"));
    // Yeniden deneme: 3 ve 4 gider; gönderilenler/reddedilenler yeniden denenmez.
    let g2 = Sahte::yeni(vec![]);
    let o2 = k.gonder(&g2, 0).unwrap();
    assert_eq!((o2.gonderilen, o2.kalan, o2.durma_nedeni), (2, 0, None));
    let adlar: Vec<String> = g2
        .cagrilar
        .borrow()
        .iter()
        .map(|c| c.1["i"].to_string())
        .collect();
    assert_eq!(adlar, vec!["3", "4"]);
    // giden.jsonl hiç değişmedi.
    assert_eq!(k.kayitlar().unwrap().len(), 4);
    // en_cok sınırı
    k.ekle("/v1/olay", "N-5", json!({})).unwrap();
    k.ekle("/v1/olay", "N-6", json!({})).unwrap();
    let o3 = k.gonder(&Sahte::yeni(vec![]), 1).unwrap();
    assert_eq!((o3.gonderilen, o3.kalan), (1, 1));
}

#[test]
fn bozuk_satir_hata_verir_ve_silinmez() {
    let k = Kuyruk::ac(gecici().join("p"));
    k.ekle("/v1/olay", "N-1", json!({})).unwrap();
    taa_dosya::satir_ekle(&k.klasor().join("giden.jsonl"), "{bozuk").unwrap();
    match k.kayitlar() {
        Err(KuyrukHata::BozukSatir { dosya, satir }) => {
            assert_eq!((dosya.as_str(), satir), ("giden.jsonl", 2))
        }
        d => panic!("bozuk satır yakalanmadı: {d:?}"),
    }
    assert_eq!(
        taa_dosya::satirlari_oku(&k.klasor().join("giden.jsonl"))
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn program_klasorleri_ve_hepsi() {
    let kok = gecici();
    assert!(Kuyruk::hepsi(&kok).unwrap().is_empty());
    Kuyruk::program(&kok, "taa-fis")
        .unwrap()
        .ekle("/v1/olay", "A", json!({}))
        .unwrap();
    Kuyruk::program(&kok, "taa-kasa")
        .unwrap()
        .ekle("/v1/olay", "B", json!({}))
        .unwrap();
    assert!(matches!(
        Kuyruk::program(&kok, "Taa"),
        Err(KuyrukHata::GecersizProgram(_))
    ));
    let h = Kuyruk::hepsi(&kok).unwrap();
    let adlar: Vec<&str> = h.iter().map(|(a, _)| a.as_str()).collect();
    assert_eq!(adlar, vec!["taa-fis", "taa-kasa"]);
    assert!(kok
        .join("kuyruk")
        .join("taa-fis")
        .join("giden.jsonl")
        .is_file());
}

#[test]
fn curl_cikti_ve_kod_yorumu() {
    use gonderici::{cikti_ayir, kodu_yorumla};
    assert_eq!(
        cikti_ayir(b"{\"id\":3}\n201"),
        Some((b"{\"id\":3}".to_vec(), 201))
    );
    assert_eq!(cikti_ayir(b"\n000"), Some((Vec::new(), 0)));
    assert_eq!(cikti_ayir(b"sadece"), None);
    assert_eq!(kodu_yorumla(200, b"{\"id\":3}").unwrap(), json!({"id": 3}));
    assert_eq!(kodu_yorumla(204, b"").unwrap(), Value::Null);
    assert_eq!(
        kodu_yorumla(200, b"tamam").unwrap(),
        Value::String("tamam".into())
    );
    assert!(matches!(
        kodu_yorumla(409, b"cakisma"),
        Err(GonderHata::Reddedildi { kod: 409, .. })
    ));
    assert!(matches!(
        kodu_yorumla(503, b""),
        Err(GonderHata::Ulasilamadi(_))
    ));
    assert!(matches!(
        kodu_yorumla(429, b""),
        Err(GonderHata::Ulasilamadi(_))
    ));
    assert!(matches!(
        kodu_yorumla(0, b""),
        Err(GonderHata::Ulasilamadi(_))
    ));
}

#[test]
fn curl_gonderici_dogrulama() {
    assert!(CurlGonderici::yeni("ftp://x").is_err());
    assert!(CurlGonderici::yeni("https://x y").is_err());
    let g = CurlGonderici::yeni("https://sunucu:8443/").unwrap();
    assert_eq!(g.taban(), "https://sunucu:8443");
    assert!(g.clone().oturum("").is_err());
    assert!(g.clone().oturum("a\nb").is_err());
    assert!(g.oturum("abc123").is_ok());
}

// ---- curl ile gerçek gönderim: küçük yerel HTTP sunucusu ----

struct Istek {
    yol: String,
    basliklar: Vec<String>,
    govde: String,
}

/// Tek iş parçacıklı test sunucusu: isteği kaydeder; `/v1/olay` → 201 JSON, `/v1/ret` → 422, `/v1/bozuk` → 500.
fn http_sunucu(kayit: Arc<Mutex<Vec<Istek>>>) -> String {
    let dinle = TcpListener::bind("127.0.0.1:0").unwrap();
    let adres = format!("http://{}", dinle.local_addr().unwrap());
    std::thread::spawn(move || {
        for baglanti in dinle.incoming() {
            let Ok(mut b) = baglanti else { continue };
            let mut okuyucu = BufReader::new(b.try_clone().unwrap());
            let mut satir = String::new();
            if okuyucu.read_line(&mut satir).is_err() {
                continue;
            }
            let mut basliklar = Vec::new();
            let mut uzunluk = 0usize;
            loop {
                let mut s = String::new();
                if okuyucu.read_line(&mut s).unwrap_or(0) == 0 || s == "\r\n" || s == "\n" {
                    break;
                }
                if let Some(v) = s.to_ascii_lowercase().strip_prefix("content-length:") {
                    uzunluk = v.trim().parse().unwrap_or(0);
                }
                basliklar.push(s.trim_end().to_string());
            }
            let mut govde = vec![0u8; uzunluk];
            let _ = okuyucu.read_exact(&mut govde);
            let yol = satir.split_whitespace().nth(1).unwrap_or("/").to_string();
            let (durum, cevap) = match yol.as_str() {
                "/v1/olay" => ("201 Created", "{\"id\":42}"),
                "/v1/ret" => ("422 Unprocessable Entity", "alan eksik"),
                _ => ("500 Internal Server Error", ""),
            };
            kayit.lock().unwrap().push(Istek {
                yol,
                basliklar,
                govde: String::from_utf8_lossy(&govde).into_owned(),
            });
            let yanit = format!(
                "HTTP/1.1 {durum}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{cevap}",
                cevap.len()
            );
            let _ = b.write_all(yanit.as_bytes());
            let _ = b.flush();
        }
    });
    adres
}

/// curl testleri sırayla koşar: SON_BASLIK_DOSYASI tek değer tutar.
static CURL_KILIT: Mutex<()> = Mutex::new(());

fn curl_var() -> bool {
    std::process::Command::new("curl")
        .arg("--version")
        .output()
        .is_ok()
}

#[test]
fn curl_ile_gonderim() {
    if !curl_var() {
        eprintln!("curl yok, test atlandı");
        return;
    }
    let _k = CURL_KILIT.lock().unwrap_or_else(|e| e.into_inner());
    let kayit = Arc::new(Mutex::new(Vec::new()));
    let adres = http_sunucu(kayit.clone());
    let g = CurlGonderici::yeni(&adres)
        .unwrap()
        .oturum("gizli-oturum")
        .unwrap();
    let k = Kuyruk::ac(gecici().join("p"));
    k.ekle(
        "/v1/olay",
        "N-1",
        json!({"tur": "kod_okundu", "kod": "869"}),
    )
    .unwrap();
    k.ekle("/v1/ret", "N-2", json!({})).unwrap();
    k.ekle("/v1/bozuk", "N-3", json!({})).unwrap();
    k.ekle("/v1/olay", "N-4", json!({})).unwrap();
    let o = k.gonder(&g, 0).unwrap();
    assert_eq!((o.gonderilen, o.reddedilen, o.kalan), (1, 1, 2));
    assert!(o.durma_nedeni.unwrap().contains("HTTP 500"));
    let istekler = kayit.lock().unwrap();
    assert_eq!(istekler.len(), 3);
    assert_eq!(istekler[0].yol, "/v1/olay");
    assert_eq!(
        serde_json::from_str::<Value>(&istekler[0].govde).unwrap(),
        json!({"tur": "kod_okundu", "kod": "869"})
    );
    assert!(istekler[0]
        .basliklar
        .iter()
        .any(|b| b == "Authorization: Bearer gizli-oturum"));
    assert!(istekler[0]
        .basliklar
        .iter()
        .any(|b| b.eq_ignore_ascii_case("content-type: application/json")));
    assert_eq!(k.gonderilenler().unwrap()["N-1"].yanit, json!({"id": 42}));
    assert_eq!(k.reddedilenler().unwrap()["N-2"].kod, 422);
    // Geçici başlık dosyası (anahtar) geride kalmadı. (Yalnız bu göndericinin kendi dosyası denetlenir;
    // temp klasörünün tümü taranmaz, çünkü paralel koşan başka test o anda kendi dosyasını yazıyor olabilir.)
    let yol = gonderici::SON_BASLIK_DOSYASI
        .lock()
        .unwrap()
        .clone()
        .expect("başlık dosyası yazılmadı");
    assert!(
        !yol.exists(),
        "başlık dosyası silinmemiş: {}",
        yol.display()
    );
    assert!(!yol.parent().unwrap().exists(), "geçici klasör silinmemiş");
}

struct SahteImzalayici;

impl Imzalayici for SahteImzalayici {
    fn basliklar(
        &self,
        yontem: &str,
        yol: &str,
        govde: &[u8],
    ) -> Result<Vec<(String, String)>, String> {
        Ok(vec![
            ("X-TAA-Cihaz".into(), "abcdef0123456789".into()),
            (
                "X-TAA-Imza".into(),
                format!("{yontem}|{yol}|{}", govde.len()),
            ),
        ])
    }
}

struct BozukImzalayici;

impl Imzalayici for BozukImzalayici {
    fn basliklar(&self, _: &str, _: &str, _: &[u8]) -> Result<Vec<(String, String)>, String> {
        Ok(vec![("X-TAA-Imza".into(), "satır\nsonu".into())])
    }
}

#[test]
fn curl_imzali_basliklar() {
    if !curl_var() {
        return;
    }
    let _k = CURL_KILIT.lock().unwrap_or_else(|e| e.into_inner());
    let kayit = Arc::new(Mutex::new(Vec::new()));
    let adres = http_sunucu(kayit.clone());
    let g = CurlGonderici::yeni(&adres)
        .unwrap()
        .imzalayici(Arc::new(SahteImzalayici));
    assert!(g.imzali_mi());
    let govde = json!({"tur": "kod_okundu"});
    g.gonder("/v1/olay", &govde).unwrap();
    let istekler = kayit.lock().unwrap();
    assert!(istekler[0]
        .basliklar
        .iter()
        .any(|b| b == "X-TAA-Cihaz: abcdef0123456789"));
    // İmzalanan gövde, gönderilen gövdeyle aynı bayt sayısında.
    let beklenen = format!("X-TAA-Imza: POST|/v1/olay|{}", govde.to_string().len());
    assert!(
        istekler[0].basliklar.iter().any(|b| b == &beklenen),
        "{:?}",
        istekler[0].basliklar
    );
    drop(istekler);
    let bozuk = CurlGonderici::yeni(&adres)
        .unwrap()
        .imzalayici(Arc::new(BozukImzalayici));
    assert!(matches!(
        bozuk.gonder("/v1/olay", &json!({})),
        Err(GonderHata::Yapilandirma(_))
    ));
    assert_eq!(
        kayit.lock().unwrap().len(),
        1,
        "bozuk başlıkla istek gitmemeli"
    );
}

#[test]
fn curl_ulasilamayan_sunucu() {
    if !curl_var() {
        return;
    }
    let _k = CURL_KILIT.lock().unwrap_or_else(|e| e.into_inner());
    // Kapalı port: bağlantı reddedilir → Ulasilamadi.
    let dinle = TcpListener::bind("127.0.0.1:0").unwrap();
    let adres = format!("http://{}", dinle.local_addr().unwrap());
    drop(dinle);
    let g = CurlGonderici::yeni(&adres).unwrap();
    assert!(matches!(
        g.gonder("/v1/olay", &json!({})),
        Err(GonderHata::Ulasilamadi(_))
    ));
}
