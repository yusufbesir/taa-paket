// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! Yerel dosya kaynağıyla uçtan uca testler (geçici klasörde): kontrol, uygula, imza, geri dönme.

use super::*;
use std::collections::BTreeMap;
use std::io::Write;
use taa_imza::{anahtar_uret, kanonik_bayt, paket_imzala, yayin_imzala, OzelAnahtar};
use taa_kur::paket::{icerik_hesapla, sha256_bayt, DosyaKaydi};

struct Ort {
    taban: PathBuf,
    kok: PathBuf,
    kaynak: PathBuf,
}

impl Ort {
    fn yeni() -> Ort {
        let taban = taa_dosya::gecici_klasor().unwrap();
        let (kok, kaynak) = (taban.join("kok"), taban.join("kaynak"));
        fs::create_dir_all(&kaynak).unwrap();
        Ort { taban, kok, kaynak }
    }
    fn kur(&self) -> Kur {
        Kur::yeni(&self.kok).unwrap()
    }
    fn anahtar(&self, ad: &str, guvenilir: bool) -> OzelAnahtar {
        let ozel = self.taban.join(format!("{ad}.key"));
        let genel = if guvenilir {
            self.kok.join("anahtarlar").join(format!("{ad}.pub"))
        } else {
            self.taban.join(format!("{ad}.pub"))
        };
        anahtar_uret(&ozel, &genel).unwrap();
        OzelAnahtar::dosyadan(&ozel).unwrap().0
    }
    /// Zip paketini kaynağa yazar; (url, sha256, boyut) döner.
    fn zip(&self, ad: &str, surum: &str, imza: Option<&OzelAnahtar>) -> (String, String, u64) {
        let liste = vec![DosyaKaydi {
            yol: "bin/x.txt".into(),
            sha256: sha256_bayt(surum.as_bytes()),
        }];
        let mut p = serde_json::json!({
            "sema": 1, "ad": ad, "surum": surum, "arayuz": "1", "program": "ortak",
            "gerektirir": {}, "sunar": [], "ister": [], "veri_sahibi": [], "veri_okur": [],
            "dosyalar": liste, "icerik_sha256": icerik_hesapla(&liste),
        });
        if let Some(o) = imza {
            paket_imzala(&mut p, o).unwrap();
        }
        let url = format!("{ad}-{surum}.zip");
        let yol = self.kaynak.join(&url);
        let mut z = zip::ZipWriter::new(fs::File::create(&yol).unwrap());
        let sec = zip::write::SimpleFileOptions::default();
        z.start_file("paket.json", sec).unwrap();
        z.write_all(&serde_json::to_vec_pretty(&p).unwrap())
            .unwrap();
        z.start_file("bin/x.txt", sec).unwrap();
        z.write_all(surum.as_bytes()).unwrap();
        z.finish().unwrap();
        let b = fs::read(&yol).unwrap();
        (url, sha256_bayt(&b), b.len() as u64)
    }
    fn yayin(&self, paketler: Vec<serde_json::Value>, imza: Option<&OzelAnahtar>) {
        let mut y = serde_json::json!({"sema": 1, "paketler": paketler});
        if let Some(o) = imza {
            yayin_imzala(&mut y, o).unwrap();
        }
        fs::write(
            self.kaynak.join("yayin.json"),
            serde_json::to_vec_pretty(&y).unwrap(),
        )
        .unwrap();
    }
    /// İlk kurulum için zip'i doğrudan Kur ile kurar.
    fn kurulu_olsun(&self, ad: &str, surum: &str, imza: Option<&OzelAnahtar>) {
        let (url, _, _) = self.zip(ad, surum, imza);
        self.kur().kur(&self.kaynak.join(url), true).unwrap();
    }
    fn secili(&self, ad: &str) -> Option<String> {
        self.kur()
            .liste()
            .unwrap()
            .into_iter()
            .find(|s| s.ad == ad)
            .and_then(|s| s.secili)
    }
}

impl Drop for Ort {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.taban);
    }
}

fn girdi(ad: &str, surum: &str, kanal: &str, f: (String, String, u64)) -> serde_json::Value {
    serde_json::json!({
        "ad": ad, "surum": surum, "kanal": kanal, "arayuz": "1", "gerektirir": {},
        "tam": {"url": f.0, "sha256": f.1, "boyut": f.2}, "yamalar": [],
    })
}

fn kaynak(o: &Ort) -> YerelKaynak {
    YerelKaynak::yeni(&o.kaynak)
}

#[test]
fn kontrol_guncellenebilir_guncel_ve_kurulu_olmayan() {
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.0.0", None);
    o.kurulu_olsun("b", "2.0.0", None);
    let (a, b, c) = (
        o.zip("a", "1.1.0", None),
        o.zip("b", "2.0.0", None),
        o.zip("c", "1.0.0", None),
    );
    o.yayin(
        vec![
            girdi("a", "1.1.0", "kararli", a),
            girdi("b", "2.0.0", "kararli", b),
            girdi("c", "1.0.0", "kararli", c),
        ],
        None,
    );
    let k = kontrol(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert_eq!(k.guncellenebilir.len(), 1);
    assert_eq!(
        (
            k.guncellenebilir[0].ad.as_str(),
            k.guncellenebilir[0].yeni.as_str()
        ),
        ("a", "1.1.0")
    );
    assert!(!k.guncellenebilir[0].zorunlu);
    assert_eq!(k.guncel, vec!["b".to_string()]); // c kurulu değil: listelenmez
    assert_eq!(k.imza, "denetlenmedi");
    assert!(k.uyarilar.iter().any(|u| u.contains("denetlenmiyor")));
}

#[test]
fn kanal_secimi_ve_en_yuksek_surum() {
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.0.0", None);
    let (k1, k2, d) = (
        o.zip("a", "1.1.0", None),
        o.zip("a", "1.5.0", None),
        o.zip("a", "2.0.0", None),
    );
    o.yayin(
        vec![
            girdi("a", "1.1.0", "kararli", k1),
            girdi("a", "1.5.0", "kararli", k2),
            girdi("a", "2.0.0", "deneme", d),
        ],
        None,
    );
    let k = kontrol(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert_eq!(k.guncellenebilir[0].yeni, "1.5.0");
    let k = kontrol(&o.kur(), &kaynak(&o), "deneme").unwrap();
    assert_eq!(k.guncellenebilir[0].yeni, "2.0.0");
    assert!(kontrol(&o.kur(), &kaynak(&o), "yok")
        .unwrap()
        .guncellenebilir
        .is_empty());
}

#[test]
fn en_az_surum_zorunlu_isaretler() {
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.0.0", None);
    o.kurulu_olsun("b", "1.5.0", None);
    let (a, b) = (o.zip("a", "1.6.0", None), o.zip("b", "1.6.0", None));
    let mut ga = girdi("a", "1.6.0", "kararli", a);
    ga["en_az_surum"] = "1.2.0".into();
    let mut gb = girdi("b", "1.6.0", "kararli", b);
    gb["en_az_surum"] = "1.2.0".into();
    o.yayin(vec![ga, gb], None);
    let k = kontrol(&o.kur(), &kaynak(&o), "kararli").unwrap();
    let z: BTreeMap<_, _> = k
        .guncellenebilir
        .iter()
        .map(|g| (g.ad.as_str(), g.zorunlu))
        .collect();
    assert!(z["a"]);
    assert!(!z["b"]);
}

#[test]
fn geri_alma_korumasi_dusuk_surumu_reddeder() {
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.5.0", None);
    let eski = o.zip("a", "1.2.0", None);
    o.yayin(vec![girdi("a", "1.2.0", "kararli", eski)], None);
    let k = kontrol(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(k.guncellenebilir.is_empty());
    assert_eq!(k.reddedilen.len(), 1);
    assert!(k.reddedilen[0].neden.contains("geri alma"));
    let s = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(s.satirlar.is_empty());
    assert_eq!(o.secili("a").as_deref(), Some("1.5.0"));
}

#[test]
fn uygula_uctan_uca_kurar_ve_secer() {
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.0.0", None);
    let f = o.zip("a", "1.1.0", None);
    o.yayin(vec![girdi("a", "1.1.0", "kararli", f)], None);
    let s = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(!s.hata_var(), "{s:?}");
    assert_eq!(
        (s.satirlar[0].durum.as_str(), s.satirlar[0].yeni.as_str()),
        ("guncellendi", "1.1.0")
    );
    assert_eq!(o.secili("a").as_deref(), Some("1.1.0"));
    let l = o.kur().liste().unwrap();
    assert_eq!(l[0].kurulu, vec!["1.0.0", "1.1.0"]);
    assert_eq!(l[0].onceki.as_deref(), Some("1.0.0"));
    assert!(o.kur().dogrula(None).unwrap().is_empty());
    // indirilen dosya silinmedi, eski/ altında
    assert!(fs::read_dir(o.kok.join("eski"))
        .unwrap()
        .flatten()
        .any(|e| e
            .file_name()
            .to_string_lossy()
            .starts_with("indirilen-a-1.1.0")));
    // ikinci kontrol: güncel
    assert!(kontrol(&o.kur(), &kaynak(&o), "kararli")
        .unwrap()
        .guncellenebilir
        .is_empty());
}

#[test]
fn bozuk_sha256_ve_boyut_kurulumu_engeller() {
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.0.0", None);
    let (url, sha, boyut) = o.zip("a", "1.1.0", None);
    // yayin.json'daki sha yanlış
    o.yayin(
        vec![girdi(
            "a",
            "1.1.0",
            "kararli",
            (url.clone(), "0".repeat(64), boyut),
        )],
        None,
    );
    let s = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(s.hata_var());
    assert!(
        s.satirlar[0].mesaj.contains("SHA256"),
        "{:?}",
        s.satirlar[0]
    );
    // boyut küçük bildirilmiş (indirme sınırı aşılır)
    o.yayin(
        vec![girdi(
            "a",
            "1.1.0",
            "kararli",
            (url.clone(), sha.clone(), boyut - 1),
        )],
        None,
    );
    let s = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(
        s.hata_var() && s.satirlar[0].mesaj.contains("boyut"),
        "{:?}",
        s.satirlar[0]
    );
    // boyut büyük bildirilmiş
    o.yayin(
        vec![girdi("a", "1.1.0", "kararli", (url, sha, boyut + 1))],
        None,
    );
    assert!(uygula(&o.kur(), &kaynak(&o), "kararli").unwrap().hata_var());
    assert_eq!(o.secili("a").as_deref(), Some("1.0.0"));
    assert_eq!(o.kur().liste().unwrap()[0].kurulu, vec!["1.0.0"]);
}

#[test]
fn zip_icindeki_paket_yayinla_uyusmazsa_onceki_surume_donulur() {
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.0.0", None);
    // yayin 1.1.0 diyor, zip'in içinde 1.2.0 var: kur 1.2.0'ı seçer, uyuşmazlık fark edilir ve geri dönülür
    let (_, _, _) = o.zip("a", "1.2.0", None);
    let sahte = {
        let b = fs::read(o.kaynak.join("a-1.2.0.zip")).unwrap();
        fs::write(o.kaynak.join("a-1.1.0.zip"), &b).unwrap();
        (String::from("a-1.1.0.zip"), sha256_bayt(&b), b.len() as u64)
    };
    o.yayin(vec![girdi("a", "1.1.0", "kararli", sahte)], None);
    let s = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(s.hata_var());
    assert!(s.satirlar[0].geri_donuldu, "{:?}", s.satirlar[0]);
    assert_eq!(o.secili("a").as_deref(), Some("1.0.0"));
}

#[test]
fn imzali_akis_ve_imza_ihlalleri() {
    let o = Ort::yeni();
    let oz = o.anahtar("yayinci", true);
    o.kurulu_olsun("a", "1.0.0", Some(&oz));
    let f = o.zip("a", "1.1.0", Some(&oz));
    // tam imzalı: başarılı
    o.yayin(vec![girdi("a", "1.1.0", "kararli", f.clone())], Some(&oz));
    let k = kontrol(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert_eq!(k.imza, "dogrulandi");
    assert!(k.uyarilar.is_empty());
    let s = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(!s.hata_var(), "{s:?}");
    assert_eq!(o.secili("a").as_deref(), Some("1.1.0"));
}

#[test]
fn yayin_imzasiz_bozuk_ya_da_yabanci_anahtarli_reddedilir() {
    let o = Ort::yeni();
    let oz = o.anahtar("yayinci", true);
    let yabanci = o.anahtar("yabanci", false);
    o.kurulu_olsun("a", "1.0.0", Some(&oz));
    let f = o.zip("a", "1.1.0", Some(&oz));
    let g = girdi("a", "1.1.0", "kararli", f);
    // imzasız
    o.yayin(vec![g.clone()], None);
    let e = kontrol(&o.kur(), &kaynak(&o), "kararli").unwrap_err();
    assert!(
        matches!(e, KurHata::Dogrulama(_)) && e.mesaj().contains("imzasız"),
        "{e}"
    );
    // yabancı anahtar
    o.yayin(vec![g.clone()], Some(&yabanci));
    assert!(kontrol(&o.kur(), &kaynak(&o), "kararli").is_err());
    // imzadan sonra içerik değiştirildi (sha256 değiştirilerek kötü paket yönlendirme)
    o.yayin(vec![g.clone()], Some(&oz));
    let yol = o.kaynak.join("yayin.json");
    let mut v: serde_json::Value = serde_json::from_slice(&fs::read(&yol).unwrap()).unwrap();
    v["paketler"][0]["tam"]["url"] = "baska.zip".into();
    fs::write(&yol, serde_json::to_vec(&v).unwrap()).unwrap();
    let e = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap_err();
    assert!(e.mesaj().contains("geçersiz"), "{e}");
    assert_eq!(o.secili("a").as_deref(), Some("1.0.0"));
    // kanonik bayt imza alanından bağımsız
    assert_eq!(
        kanonik_bayt(&v),
        kanonik_bayt(&serde_json::json!({"sema":1,"paketler":v["paketler"].clone()}))
    );
}

#[test]
fn imzali_yayinda_imzasiz_paket_kurulumu_reddeder_onceki_kalir() {
    let o = Ort::yeni();
    let oz = o.anahtar("yayinci", true);
    o.kurulu_olsun("a", "1.0.0", Some(&oz));
    let f = o.zip("a", "1.1.0", None); // paket imzasız
    o.yayin(vec![girdi("a", "1.1.0", "kararli", f)], Some(&oz));
    let s = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(s.hata_var());
    assert!(
        s.satirlar[0].mesaj.contains("imzasız"),
        "{:?}",
        s.satirlar[0]
    );
    assert_eq!(o.secili("a").as_deref(), Some("1.0.0"));
    assert_eq!(o.kur().liste().unwrap()[0].kurulu, vec!["1.0.0"]);
}

#[test]
fn sabitli_paket_atlanir_ama_zorunlu_olan_uygulanir() {
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.0.0", None);
    o.kurulu_olsun("b", "1.0.0", None);
    o.kur().sabitle("a", true).unwrap();
    o.kur().sabitle("b", true).unwrap();
    let (a, b) = (o.zip("a", "1.1.0", None), o.zip("b", "1.1.0", None));
    let gb = {
        let mut g = girdi("b", "1.1.0", "kararli", b);
        g["en_az_surum"] = "1.1.0".into();
        g
    };
    o.yayin(vec![girdi("a", "1.1.0", "kararli", a), gb], None);
    let s = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap();
    let d: BTreeMap<_, _> = s
        .satirlar
        .iter()
        .map(|r| (r.ad.as_str(), r.durum.as_str()))
        .collect();
    assert_eq!(d["a"], "atlandi");
    assert_eq!(d["b"], "guncellendi");
    assert_eq!(o.secili("a").as_deref(), Some("1.0.0"));
    assert_eq!(o.secili("b").as_deref(), Some("1.1.0"));
}

#[test]
fn yayin_semasi_hatalari_ve_yol_kacisi() {
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.0.0", None);
    let yaz = |j: &str| fs::write(o.kaynak.join("yayin.json"), j).unwrap();
    yaz("{bozuk");
    assert!(matches!(
        kontrol(&o.kur(), &kaynak(&o), "kararli"),
        Err(KurHata::Dogrulama(_))
    ));
    yaz(r#"{"sema":2,"paketler":[]}"#);
    assert!(kontrol(&o.kur(), &kaynak(&o), "kararli").is_err());
    // yol kaçışı: url ../ içeriyor
    let sha = "a".repeat(64);
    let kotu = serde_json::json!({"sema":1,"paketler":[{"ad":"a","surum":"1.1.0","kanal":"kararli","arayuz":"1",
        "tam":{"url":"../kok/kayit.json","sha256":sha,"boyut":10}}]});
    yaz(&kotu.to_string());
    let s = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(s.hata_var());
    assert!(
        s.satirlar[0].mesaj.contains("güvenli değil") || s.satirlar[0].mesaj.contains(".zip"),
        "{:?}",
        s.satirlar[0]
    );
    let kotu2 = serde_json::json!({"sema":1,"paketler":[{"ad":"a","surum":"1.1.0","kanal":"kararli","arayuz":"1",
        "tam":{"url":"../x.zip","sha256":sha,"boyut":10}}]});
    yaz(&kotu2.to_string());
    let s = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(
        s.satirlar[0].mesaj.contains("güvenli değil"),
        "{:?}",
        s.satirlar[0]
    );
    // yayin.json yok
    fs::rename(
        o.kaynak.join("yayin.json"),
        o.taban.join("yayin.json.yedek"),
    )
    .unwrap();
    assert!(matches!(
        kontrol(&o.kur(), &kaynak(&o), "kararli"),
        Err(KurHata::Yok(_))
    ));
}

// ---- HTTP kaynağı (curl ile): kaynak klasörünü yerel bir HTTP sunucusuyla yayınlar ----

/// Tek iş parçacıklı küçük HTTP/1.1 sunucusu: `kok` altındaki dosyaları GET ile verir; `/yonlendir/<x>` → `/<x>` (302).
/// Yalnız test içindir. Döndürdüğü `http://127.0.0.1:<port>` tabanı curl'ün ulaşacağı adrestir.
fn http_sunucu(kok: PathBuf) -> String {
    use std::io::{BufRead, BufReader};
    use std::net::TcpListener;
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
            // başlıkları tüket
            loop {
                let mut s = String::new();
                if okuyucu.read_line(&mut s).unwrap_or(0) == 0 || s == "\r\n" || s == "\n" {
                    break;
                }
            }
            let yol = satir.split_whitespace().nth(1).unwrap_or("/").to_string();
            let yanit: Vec<u8> = if let Some(hedef) = yol.strip_prefix("/yonlendir/") {
                format!("HTTP/1.1 302 Found\r\nLocation: /{hedef}\r\nContent-Length: 0\r\nConnection: close\r\n\r\n")
                    .into_bytes()
            } else {
                let dosya = kok.join(yol.trim_start_matches('/'));
                match fs::read(&dosya) {
                    Ok(govde) => {
                        let mut v = format!(
                            "HTTP/1.1 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                            govde.len()
                        )
                        .into_bytes();
                        v.extend_from_slice(&govde);
                        v
                    }
                    Err(_) => {
                        b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
                            .to_vec()
                    }
                }
            };
            let _ = b.write_all(&yanit);
            let _ = b.flush();
        }
    });
    adres
}

fn curl_var() -> bool {
    std::process::Command::new("curl")
        .arg("--version")
        .output()
        .is_ok()
}

#[test]
fn http_kaynak_adres_denetimi() {
    assert!(HttpKaynak::yeni("ftp://x").is_err());
    assert!(HttpKaynak::yeni("D:/Kaynak").is_err());
    assert!(HttpKaynak::yeni("https://a b").is_err());
    let k = HttpKaynak::yeni("https://sunucu/yol/").unwrap();
    assert_eq!(k.taban(), "https://sunucu/yol");
    assert!(http_url_mu("HTTP://x") && !http_url_mu("/tmp/x"));
    // göreli url tabana eklenir; mutlak kalır; yol kaçışı/şema dışı reddedilir
    let t = taa_dosya::gecici_klasor().unwrap();
    let hedef = t.join("x.zip");
    assert!(matches!(
        k.getir("/mutlak.zip", &hedef, 1),
        Err(KurHata::Dogrulama(_))
    ));
    assert!(matches!(
        k.getir("ftp://x/a.zip", &hedef, 1),
        Err(KurHata::Dogrulama(_))
    ));
    let _ = fs::remove_dir_all(&t);
}

#[test]
fn http_kaynak_uctan_uca_kontrol_ve_uygula() {
    if !curl_var() {
        eprintln!("curl yok; test atlandı");
        return;
    }
    let o = Ort::yeni();
    let imza = o.anahtar("yayinci", true);
    o.kurulu_olsun("a", "1.0.0", Some(&imza));
    o.kurulu_olsun("b", "2.0.0", Some(&imza));
    let fa = o.zip("a", "1.1.0", Some(&imza));
    let fb = o.zip("b", "2.1.0", Some(&imza));
    // b'nin url'si yönlendirme üzerinden (GitHub Releases gibi)
    let fb = (format!("yonlendir/{}", fb.0), fb.1, fb.2);
    o.yayin(
        vec![
            girdi("a", "1.1.0", "kararli", fa),
            girdi("b", "2.1.0", "kararli", fb),
        ],
        Some(&imza),
    );
    let taban = http_sunucu(o.kaynak.clone());
    let h = HttpKaynak::yeni(&format!("{taban}/")).unwrap();
    let k = kontrol(&o.kur(), &h, "kararli").unwrap();
    assert_eq!(k.imza, "dogrulandi");
    assert_eq!(k.guncellenebilir.len(), 2, "{k:?}");
    // yalnız b (pencere düğmesi): a dokunulmaz
    let s = uygula_paket(&o.kur(), &h, "kararli", Some("b")).unwrap();
    assert!(!s.hata_var(), "{s:?}");
    assert_eq!(s.satirlar.len(), 1);
    assert_eq!(o.secili("b").as_deref(), Some("2.1.0"));
    assert_eq!(o.secili("a").as_deref(), Some("1.0.0"));
    // kalan: a
    let s = uygula(&o.kur(), &h, "kararli").unwrap();
    assert!(!s.hata_var(), "{s:?}");
    assert_eq!(o.secili("a").as_deref(), Some("1.1.0"));
    assert!(o.kur().dogrula(None).unwrap().is_empty());
    assert!(kontrol(&o.kur(), &h, "kararli")
        .unwrap()
        .guncellenebilir
        .is_empty());
    // indirilen dosya eski/ altında, indirilen/ boş
    assert!(fs::read_dir(o.kok.join("eski"))
        .unwrap()
        .flatten()
        .any(|e| e
            .file_name()
            .to_string_lossy()
            .starts_with("indirilen-b-2.1.0")));
    assert_eq!(fs::read_dir(o.kok.join("indirilen")).unwrap().count(), 0);
}

#[test]
fn http_kaynak_hatalari_bulunamadi_buyuk_ve_var_olan_hedef() {
    if !curl_var() {
        eprintln!("curl yok; test atlandı");
        return;
    }
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.0.0", None);
    let taban = http_sunucu(o.kaynak.clone());
    let h = HttpKaynak::yeni(&taban).unwrap();
    // yayin.json yok → Yok
    assert!(matches!(
        kontrol(&o.kur(), &h, "kararli"),
        Err(KurHata::Yok(_))
    ));
    // olmayan dosya → Yok
    let hedef = o.taban.join("indir.zip");
    assert!(matches!(
        h.getir("yok.zip", &hedef, 100),
        Err(KurHata::Yok(_))
    ));
    assert!(!hedef.exists());
    // bildirilen boyuttan büyük → Dogrulama (curl --max-filesize ya da sonradan ölçüm)
    let (url, _, boyut) = o.zip("a", "1.1.0", None);
    let e = h.getir(&url, &hedef, boyut - 1).unwrap_err();
    assert!(matches!(e, KurHata::Dogrulama(_)), "{e}");
    assert!(e.mesaj().contains("büyük"), "{e}");
    // tam boyut → tamam; aynı hedef ikinci kez → üzerine yazılmaz
    let _ = fs::remove_file(&hedef);
    assert_eq!(h.getir(&url, &hedef, boyut).unwrap(), boyut);
    assert!(matches!(
        h.getir(&url, &hedef, boyut),
        Err(KurHata::Dogrulama(_))
    ));
    // uygula akışında boyut uyuşmazlığı: önceki sürüm kalır
    o.yayin(
        vec![girdi("a", "1.1.0", "kararli", (url, "0".repeat(64), boyut))],
        None,
    );
    let s = uygula(&o.kur(), &h, "kararli").unwrap();
    assert!(s.hata_var());
    assert!(
        s.satirlar[0].mesaj.contains("SHA256"),
        "{:?}",
        s.satirlar[0]
    );
    assert_eq!(o.secili("a").as_deref(), Some("1.0.0"));
}

#[test]
fn kontrol_paket_yalniz_verilen_paketi_degerlendirir() {
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.0.0", None);
    o.kurulu_olsun("b", "1.0.0", None);
    let fa = o.zip("a", "1.1.0", None);
    let fb = o.zip("b", "1.1.0", None);
    o.yayin(
        vec![
            girdi("a", "1.1.0", "kararli", fa),
            girdi("b", "1.1.0", "kararli", fb),
        ],
        None,
    );
    let k = kontrol_paket(&o.kur(), &kaynak(&o), "kararli", Some("b")).unwrap();
    assert_eq!(k.guncellenebilir.len(), 1);
    assert_eq!(k.guncellenebilir[0].ad, "b");
    let k = kontrol_paket(&o.kur(), &kaynak(&o), "kararli", Some("yok")).unwrap();
    assert!(k.guncellenebilir.is_empty() && k.guncel.is_empty());
}

#[test]
fn zaten_kurulu_surum_indirilmeden_secilir() {
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.0.0", None);
    o.kurulu_olsun("a", "1.1.0", None);
    o.kur().geri_don("a").unwrap();
    assert_eq!(o.secili("a").as_deref(), Some("1.0.0"));
    // yayında 1.1.0; url olmayan dosyaya işaret etsin: indirme denenirse hata olur
    o.yayin(
        vec![girdi(
            "a",
            "1.1.0",
            "kararli",
            ("yok.zip".into(), "0".repeat(64), 10),
        )],
        None,
    );
    let k = kontrol(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(k.guncellenebilir[0].zaten_kurulu);
    let s = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(!s.hata_var(), "{s:?}");
    assert_eq!(s.satirlar[0].durum, "secildi");
    assert_eq!(s.cikis_kodu(), 0);
    assert_eq!(o.secili("a").as_deref(), Some("1.1.0"));
    assert!(!o.kok.join("indirilen").exists());
    assert!(o.kur().dogrula(None).unwrap().is_empty());
}

#[test]
fn hatali_indirme_ayri_klasore_ve_cikis_kodu_hatanin_kodu() {
    let o = Ort::yeni();
    o.kurulu_olsun("a", "1.0.0", None);
    let (url, _, boyut) = o.zip("a", "1.1.0", None);
    o.yayin(
        vec![girdi("a", "1.1.0", "kararli", (url, "0".repeat(64), boyut))],
        None,
    );
    let s = uygula(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert!(s.hata_var());
    assert_eq!(s.satirlar[0].kod, 2, "{:?}", s.satirlar[0]);
    assert_eq!(s.cikis_kodu(), 2);
    assert_eq!(o.secili("a").as_deref(), Some("1.0.0"));
    // bozuk indirme eski/indirilen-hatali/ altında; eski/ kökünde indirilen-* yok
    let hatali: Vec<_> = fs::read_dir(o.kok.join("eski").join("indirilen-hatali"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        hatali.iter().any(|a| a.starts_with("indirilen-a-1.1.0")),
        "{hatali:?}"
    );
    assert!(!fs::read_dir(o.kok.join("eski"))
        .unwrap()
        .flatten()
        .any(|e| e.file_name().to_string_lossy().starts_with("indirilen-a-")));
    assert_eq!(fs::read_dir(o.kok.join("indirilen")).unwrap().count(), 0);
}

#[test]
fn katalog_ve_yayindan_ilk_kurulum() {
    let o = Ort::yeni();
    let fa = o.zip("a", "1.0.0", None);
    let fa2 = o.zip("a", "1.2.0", None);
    let fb = o.zip("b", "2.0.0", None);
    let mut b = girdi("b", "2.0.0", "kararli", fb);
    b["gerektirir"] = serde_json::json!({"a": "1"});
    o.yayin(
        vec![
            girdi("a", "1.0.0", "kararli", fa),
            girdi("a", "1.2.0", "kararli", fa2),
            b,
        ],
        None,
    );
    let k = katalog(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert_eq!(k.len(), 2);
    assert_eq!((k[0].ad.as_str(), k[0].yayinda.as_str()), ("a", "1.2.0"));
    assert!(k[0].kurulu.is_none() && k[1].gerektirir == vec!["a".to_string()]);
    // b kurulunca önce bağımlılığı a (en yüksek sürüm) kurulur.
    let r = kur_yayindan(&o.kur(), &kaynak(&o), "kararli", "b").unwrap();
    assert_eq!(r, vec!["a 1.2.0".to_string(), "b 2.0.0".to_string()]);
    assert_eq!(o.secili("a").as_deref(), Some("1.2.0"));
    assert_eq!(o.secili("b").as_deref(), Some("2.0.0"));
    assert!(o.kur().dogrula(None).unwrap().is_empty());
    // ikinci kez: zaten kurulu → hata; olmayan paket → yok
    assert!(kur_yayindan(&o.kur(), &kaynak(&o), "kararli", "b")
        .unwrap_err()
        .mesaj()
        .contains("zaten kurulu"));
    assert!(matches!(
        kur_yayindan(&o.kur(), &kaynak(&o), "kararli", "yok"),
        Err(KurHata::Yok(_))
    ));
    let k = katalog(&o.kur(), &kaynak(&o), "kararli").unwrap();
    assert_eq!(k[1].kurulu.as_deref(), Some("2.0.0"));
}

#[test]
fn yayindan_kurulum_bozuk_sha_ile_kurmaz() {
    let o = Ort::yeni();
    let mut f = o.zip("a", "1.0.0", None);
    f.1 = "0".repeat(64);
    o.yayin(vec![girdi("a", "1.0.0", "kararli", f)], None);
    assert!(kur_yayindan(&o.kur(), &kaynak(&o), "kararli", "a").is_err());
    assert!(o.secili("a").is_none());
}
