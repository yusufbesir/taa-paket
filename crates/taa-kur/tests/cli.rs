// Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05]
//! `taa-kur` komut satırı birleşik testleri: komut başına bir test.
//! Her test kendi geçici kökünü kullanır (TAA_KOK); gerçek kök etkilenmez.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use taa_kur::paket::{icerik_hesapla, sha256_bayt, DosyaKaydi};

struct Ortam {
    taban: PathBuf,
    kok: PathBuf,
}

impl Ortam {
    fn yeni() -> Ortam {
        let taban = taa_dosya::gecici_klasor().unwrap();
        let kok = taban.join("kok");
        Ortam { taban, kok }
    }

    fn calistir(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_taa-kur"))
            .args(args)
            .env("TAA_KOK", &self.kok)
            .output()
            .unwrap()
    }

    /// Paket klasörü üretir (paket.json + tek dosya).
    fn paket(&self, ad: &str, surum: &str, gerek: &[(&str, &str)]) -> PathBuf {
        let k = self.taban.join("paketler").join(format!("{ad}-{surum}"));
        fs::create_dir_all(&k).unwrap();
        let icerik = format!("{ad} {surum}");
        fs::write(k.join("veri.txt"), &icerik).unwrap();
        let liste = vec![DosyaKaydi {
            yol: "veri.txt".into(),
            sha256: sha256_bayt(icerik.as_bytes()),
        }];
        let g: serde_json::Map<String, serde_json::Value> = gerek
            .iter()
            .map(|(a, b)| (a.to_string(), serde_json::json!(b)))
            .collect();
        let p = serde_json::json!({
            "sema": 1, "ad": ad, "surum": surum, "arayuz": "1", "program": "ortak",
            "gerektirir": g, "sunar": [], "ister": [], "veri_sahibi": [], "veri_okur": [],
            "dosyalar": liste, "icerik_sha256": icerik_hesapla(&liste),
        });
        fs::write(k.join("paket.json"), serde_json::to_vec_pretty(&p).unwrap()).unwrap();
        k
    }
}

impl Drop for Ortam {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.taban);
    }
}

fn kod(o: &Output) -> i32 {
    o.status.code().unwrap()
}
fn cikti(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}
fn hata(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}
fn yol(p: &Path) -> &str {
    p.to_str().unwrap()
}

#[test]
fn kur_komutu() {
    let o = Ortam::yeni();
    let a1 = o.paket("taa-a", "1.0.0", &[]);
    let r = o.calistir(&["kur", yol(&a1)]);
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    assert!(
        cikti(&r).contains("Kuruldu: taa-a 1.0.0 (seçili)"),
        "{}",
        cikti(&r)
    );
    assert_eq!(cikti(&r).trim().lines().count(), 1);
    assert!(o.kok.join("depo/taa-a/1.0.0/paket.json").is_file());
    // ikinci sürüm yan yana, --sec ile seçilir
    let a2 = o.paket("taa-a", "2.0.0", &[]);
    let r = o.calistir(&["kur", yol(&a2), "--sec"]);
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    assert!(o.kok.join("depo/taa-a/1.0.0").is_dir());
    // çözüm yok: çıkış 3, tek satır Türkçe mesaj
    let b = o.paket("taa-b", "1.0.0", &[("taa-yok", "1")]);
    let r = o.calistir(&["kur", yol(&b)]);
    assert_eq!(kod(&r), 3);
    assert_eq!(hata(&r).trim().lines().count(), 1);
    assert!(hata(&r).contains("taa-kur kur taa-yok"), "{}", hata(&r));
    // doğrulama hatası: sha256 uyuşmazlığı, kayıt değişmez
    let once = fs::read(o.kok.join("kayit.json")).unwrap();
    let bozuk = o.paket("taa-c", "1.0.0", &[]);
    fs::write(bozuk.join("veri.txt"), "kurcalandi").unwrap();
    let r = o.calistir(&["kur", yol(&bozuk)]);
    assert_eq!(kod(&r), 2);
    assert_eq!(fs::read(o.kok.join("kayit.json")).unwrap(), once);
    // kullanım hatası da 2
    assert_eq!(kod(&o.calistir(&["kur"])), 2);
    assert_eq!(kod(&o.calistir(&["kur", yol(&a1), "--bilinmeyen"])), 2);
}

#[test]
fn liste_komutu() {
    let o = Ortam::yeni();
    let r = o.calistir(&["liste"]);
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    assert!(cikti(&r).contains("Kurulu paket yok"));
    let a = o.paket("taa-a", "1.0.0", &[]);
    assert_eq!(kod(&o.calistir(&["kur", yol(&a)])), 0);
    let r = o.calistir(&["liste"]);
    assert_eq!(kod(&r), 0);
    assert!(cikti(&r).contains("taa-a") && cikti(&r).contains("seçili: 1.0.0"));
    let r = o.calistir(&["liste", "--json"]);
    assert_eq!(kod(&r), 0);
    let v: serde_json::Value = serde_json::from_str(&cikti(&r)).unwrap();
    assert_eq!(v[0]["ad"], "taa-a");
    assert_eq!(v[0]["secili"], "1.0.0");
    assert_eq!(v[0]["kurulu"][0], "1.0.0");
    // bilinmeyen komut
    let r = o.calistir(&["yok-komut"]);
    assert_eq!(kod(&r), 2);
    assert_eq!(kod(&o.calistir(&[])), 2);
}

#[test]
fn sec_komutu() {
    let o = Ortam::yeni();
    let a1 = o.paket("taa-a", "1.0.0", &[]);
    let a2 = o.paket("taa-a", "2.0.0", &[]);
    assert_eq!(kod(&o.calistir(&["kur", yol(&a1)])), 0);
    assert_eq!(kod(&o.calistir(&["kur", yol(&a2)])), 0);
    let r = o.calistir(&["sec", "taa-a", "2.0.0"]);
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    assert!(cikti(&r).contains("Seçildi: taa-a 2.0.0"));
    let j = o.calistir(&["liste", "--json"]);
    let v: serde_json::Value = serde_json::from_str(&cikti(&j)).unwrap();
    assert_eq!(v[0]["secili"], "2.0.0");
    // kurulu olmayan sürüm: sıfırdan farklı, tek satır hata
    let r = o.calistir(&["sec", "taa-a", "9.9.9"]);
    assert_ne!(kod(&r), 0);
    assert_eq!(hata(&r).trim().lines().count(), 1);
    // çözüm uyuşmazlığı: gereken modül kurulu değilken bağımlı paket 3 döner
    let b = o.paket("taa-b", "1.0.0", &[("taa-a", "2")]);
    assert_eq!(kod(&o.calistir(&["kur", yol(&b)])), 3);
    assert_eq!(kod(&o.calistir(&["sec", "taa-a"])), 2);
}

#[test]
fn geri_don_komutu() {
    let o = Ortam::yeni();
    // geri dönülecek önceki sürüm yok: 4
    let a1 = o.paket("taa-a", "1.0.0", &[]);
    assert_eq!(kod(&o.calistir(&["kur", yol(&a1)])), 0);
    let r = o.calistir(&["geri-don", "taa-a"]);
    assert_eq!(kod(&r), 4, "{}", hata(&r));
    assert_eq!(hata(&r).trim().lines().count(), 1);
    // sec sonrası geri dön
    let a2 = o.paket("taa-a", "2.0.0", &[]);
    assert_eq!(kod(&o.calistir(&["kur", yol(&a2), "--sec"])), 0);
    let r = o.calistir(&["geri-don", "taa-a"]);
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    assert!(
        cikti(&r).contains("Geri dönüldü: taa-a 1.0.0"),
        "{}",
        cikti(&r)
    );
    let r = o.calistir(&["geri-don", "taa-yok"]);
    assert_eq!(kod(&r), 4);
}

#[test]
fn kaldir_komutu() {
    let o = Ortam::yeni();
    let a1 = o.paket("taa-a", "1.0.0", &[]);
    let a2 = o.paket("taa-a", "2.0.0", &[]);
    assert_eq!(kod(&o.calistir(&["kur", yol(&a1)])), 0);
    assert_eq!(kod(&o.calistir(&["kur", yol(&a2), "--sec"])), 0);
    // kaldır: seçili 2.0.0 eski/ altına taşınır, 1.0.0'a dönülür; içerik birebir
    let r = o.calistir(&["kaldir", "taa-a"]);
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    assert!(
        cikti(&r).contains("Kaldırıldı: taa-a 2.0.0"),
        "{}",
        cikti(&r)
    );
    assert!(cikti(&r).contains("Seçili sürüm: 1.0.0"), "{}", cikti(&r));
    assert_eq!(cikti(&r).trim().lines().count(), 1);
    assert!(!o.kok.join("depo/taa-a/2.0.0").exists());
    let eski: Vec<_> = fs::read_dir(o.kok.join("eski"))
        .unwrap()
        .flatten()
        .map(|e| e.path())
        .collect();
    assert!(!eski.is_empty());
    let tasinan = eski
        .iter()
        .find(|p| {
            p.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("taa-a-2.0.0")
        })
        .expect("eski/ altında taşınan sürüm");
    assert_eq!(
        fs::read_to_string(tasinan.join("veri.txt")).unwrap(),
        "taa-a 2.0.0"
    );
    // belirli sürüm + olmayan paket
    let r = o.calistir(&["kaldir", "taa-a", "--surum", "1.0.0"]);
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    let r = o.calistir(&["kaldir", "taa-a"]);
    assert_eq!(kod(&r), 4, "{}", hata(&r));
    assert_eq!(kod(&o.calistir(&["kaldir", "taa-a", "--surum"])), 2);
}

#[test]
fn dogrula_komutu() {
    let o = Ortam::yeni();
    let a = o.paket("taa-a", "1.0.0", &[]);
    assert_eq!(kod(&o.calistir(&["kur", yol(&a)])), 0);
    let r = o.calistir(&["dogrula"]);
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    assert!(cikti(&r).contains("Doğrulama tamam"));
    let r = o.calistir(&["dogrula", "taa-a", "--json"]);
    assert_eq!(kod(&r), 0);
    let v: serde_json::Value = serde_json::from_str(&cikti(&r)).unwrap();
    assert_eq!(v["saglam"], true);
    // depodaki dosya bozulursa 5
    fs::write(o.kok.join("depo/taa-a/1.0.0/veri.txt"), "bozuk").unwrap();
    let r = o.calistir(&["dogrula"]);
    assert_eq!(kod(&r), 5, "{}", cikti(&r));
    assert!(cikti(&r).contains("Sorun"));
    let r = o.calistir(&["dogrula", "--json"]);
    assert_eq!(kod(&r), 5);
    let v: serde_json::Value = serde_json::from_str(&cikti(&r)).unwrap();
    assert_eq!(v["saglam"], false);
    assert!(!v["sorunlar"].as_array().unwrap().is_empty());
    assert_eq!(kod(&o.calistir(&["dogrula", "a", "b"])), 2);
}

#[test]
fn tani_komutu() {
    let o = Ortam::yeni();
    let a = o.paket("taa-a", "1.0.0", &[]);
    assert_eq!(kod(&o.calistir(&["kur", yol(&a)])), 0);
    let r = o.calistir(&["tani"]);
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    let c = cikti(&r);
    assert!(c.contains("Kök:") && c.contains("Kayıt: sağlam") && c.contains("Paket: taa-a"));
    let r = o.calistir(&["tani", "--json"]);
    assert_eq!(kod(&r), 0);
    let v: serde_json::Value = serde_json::from_str(&cikti(&r)).unwrap();
    assert_eq!(v["kayit_saglam"], true);
    assert_eq!(v["paketler"][0]["ad"], "taa-a");
    assert!(v["kok"].as_str().unwrap().ends_with("kok"));
    // sır sızmaz: ortamdan başka değişken yazılmaz
    assert!(!c.contains("HOME"));
}
