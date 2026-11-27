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
    // argümansız çalıştırma yardım basar, 0 çıkar (2026-11-05 Windows denemesi)
    assert_eq!(kod(&o.calistir(&[])), 0);
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

#[test]
fn zip_kurulumu_ve_tani_veri_alanlari() {
    use std::io::Write;
    let o = Ortam::yeni();
    // zip: paket.json + veri.txt (klasör paketiyle aynı içerik)
    let k = o.paket("taa-z", "1.0.0", &[]);
    let z = o.taban.join("taa-z.zip");
    let mut zw = zip::ZipWriter::new(fs::File::create(&z).unwrap());
    let sec = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for ad in ["paket.json", "veri.txt"] {
        zw.start_file(ad, sec).unwrap();
        zw.write_all(&fs::read(k.join(ad)).unwrap()).unwrap();
    }
    zw.finish().unwrap();
    let r = o.calistir(&["kur", yol(&z)]);
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    let r = o.calistir(&["liste", "--json"]);
    assert!(cikti(&r).contains("taa-z"));
    assert_eq!(kod(&o.calistir(&["dogrula"])), 0);
    // sahte TAA_VERI_KOK
    let vk = o.taban.join("veri");
    fs::create_dir_all(vk.join("etiket-kuyrugu")).unwrap();
    fs::write(
        vk.join("etiket-kuyrugu/SAHIP.toml"),
        "sema = 1\nsahip = \"taa-z\"\ngecici = true\n",
    )
    .unwrap();
    let r = Command::new(env!("CARGO_BIN_EXE_taa-kur"))
        .args(["tani", "--json"])
        .env("TAA_KOK", &o.kok)
        .env("TAA_VERI_KOK", &vk)
        .output()
        .unwrap();
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    let v: serde_json::Value = serde_json::from_str(&cikti(&r)).unwrap();
    assert_eq!(
        v["veri_alanlari"][0],
        serde_json::json!({"alan": "etiket-kuyrugu", "sahip": "taa-z", "gecici": true, "sahip_kurulu": true})
    );
}

#[test]
fn yardim_ve_argumansiz_sifir_cikar() {
    let o = Ortam::yeni();
    for a in [&[][..], &["--help"], &["-h"], &["yardim"]] {
        let r = o.calistir(a);
        assert_eq!(kod(&r), 0, "{a:?}: {}", hata(&r));
        assert!(cikti(&r).contains("dogrula <paket.zip|klasör>"), "{a:?}");
        assert!(hata(&r).is_empty());
    }
    // bilinmeyen komut hâlâ 2
    assert_eq!(kod(&o.calistir(&["--surum"])), 2);
}

#[test]
fn dogrula_kurulmamis_paket_klasor_ve_zip() {
    let o = Ortam::yeni();
    let a = o.paket("taa-a", "1.0.0", &[]);
    // klasör: kurulmaz, çıkış 0, tek satır
    let r = o.calistir(&["dogrula", yol(&a)]);
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    assert!(
        cikti(&r).contains("Paket geçerli: taa-a 1.0.0 (1 dosya"),
        "{}",
        cikti(&r)
    );
    assert!(cikti(&r).contains("güvenilir anahtar yok"));
    assert!(!o.kok.join("depo").exists());
    let r = o.calistir(&["dogrula", yol(&a), "--json"]);
    let v: serde_json::Value = serde_json::from_str(&cikti(&r)).unwrap();
    assert_eq!(v["ad"], "taa-a");
    assert_eq!(v["imza"], "Denetlenmedi");
    // zip: açılmadan doğrulanır, geçici/eski klasör oluşmaz
    let z = o.taban.join("taa-a-1.0.0.zip");
    {
        let f = fs::File::create(&z).unwrap();
        let mut w = zip::ZipWriter::new(f);
        let sec = zip::write::SimpleFileOptions::default();
        for ad in ["paket.json", "veri.txt"] {
            use std::io::Write;
            w.start_file(ad, sec).unwrap();
            w.write_all(&fs::read(a.join(ad)).unwrap()).unwrap();
        }
        w.finish().unwrap();
    }
    let r = o.calistir(&["dogrula", yol(&z)]);
    assert_eq!(kod(&r), 0, "{}", hata(&r));
    assert!(cikti(&r).contains("Paket geçerli: taa-a 1.0.0"));
    assert!(!o.kok.join("eski").exists());
    assert!(!o.kok.join("depo").exists());
    // bozuk zip (sha uyuşmaz): çıkış 2, kurulmaz
    let zb = o.taban.join("bozuk.zip");
    {
        let f = fs::File::create(&zb).unwrap();
        let mut w = zip::ZipWriter::new(f);
        let sec = zip::write::SimpleFileOptions::default();
        use std::io::Write;
        w.start_file("paket.json", sec).unwrap();
        w.write_all(&fs::read(a.join("paket.json")).unwrap())
            .unwrap();
        w.start_file("veri.txt", sec).unwrap();
        w.write_all(b"kurcalandi").unwrap();
        w.finish().unwrap();
    }
    let r = o.calistir(&["dogrula", yol(&zb)]);
    assert_eq!(kod(&r), 2, "{}", hata(&r));
    assert!(hata(&r).contains("sha256 uyuşmuyor"), "{}", hata(&r));
    // güvenilir anahtar varsa imzasız paket reddedilir (kur ile aynı kural)
    fs::create_dir_all(o.kok.join("anahtarlar")).unwrap();
    let ozel = o.taban.join("deneme.ozel");
    taa_imza::anahtar_uret(&ozel, &o.kok.join("anahtarlar").join("deneme.pub")).unwrap();
    let r = o.calistir(&["dogrula", yol(&z)]);
    assert_eq!(kod(&r), 2);
    assert!(hata(&r).contains("imzasız"), "{}", hata(&r));
    // var olmayan yol: kurulu paket adı sayılır -> 4
    assert_eq!(kod(&o.calistir(&["dogrula", "taa-yok"])), 4);
}

// ---- Şube bilgisayarı profili (`taa-kur sube-pc`). [Claude Fable 5.2 #taa-web-W2-kurulum 2026-11-27]

fn ikili_adi(ad: &str) -> String {
    if cfg!(windows) {
        format!("{ad}.exe")
    } else {
        ad.to_string()
    }
}

#[test]
fn sube_pc_komutu() {
    let o = Ortam::yeni();
    let veri = o.taban.join("veri");
    fs::create_dir_all(&o.kok).unwrap();
    let calistir = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_taa-kur"))
            .args(args)
            .env("TAA_KOK", &o.kok)
            .env("TAA_VERI_KOK", &veri)
            .output()
            .unwrap()
    };
    // İkililer yokken: çıkış 4, hiçbir şey yazılmaz.
    let c = calistir(&["sube-pc", "kur", "--baslatma", "--gorevsiz", "--duvarsiz"]);
    assert_eq!(kod(&c), 4, "{}", hata(&c));
    assert!(hata(&c).contains("taa-sube-dugum"));
    assert!(!veri.join("sube-pc").exists());
    // Alt komut ve seçenek hataları: çıkış 2.
    assert_eq!(kod(&calistir(&["sube-pc"])), 2);
    assert_eq!(kod(&calistir(&["sube-pc", "sil"])), 2);
    assert_eq!(kod(&calistir(&["sube-pc", "kur", "--port", "abc"])), 2);
    assert_eq!(
        kod(&calistir(&["sube-pc", "durum"])),
        4,
        "profil yokken durum"
    );

    for ad in ["taa-yazdir", "taa-sube-dugum"] {
        fs::write(o.kok.join(ikili_adi(ad)), b"").unwrap();
    }
    let c = calistir(&[
        "sube-pc",
        "kur",
        "--merkez",
        "https://merkez.ornek.com",
        "--sube-adres",
        "http://192.168.1.20:8788",
        "--baslatma",
        "--gorevsiz",
        "--duvarsiz",
        "--json",
    ]);
    assert_eq!(kod(&c), 0, "{}", hata(&c));
    let v: serde_json::Value = serde_json::from_str(&cikti(&c)).unwrap();
    assert_eq!(v["yollar"][0]["ad"], "merkez");
    assert_eq!(v["yollar"][1]["url"], "http://192.168.1.20:8788");
    assert_eq!(v["yollar"][2]["ad"], "yerel");
    assert_eq!(v["gorev"]["durum"], "atlandi");
    assert!(veri.join("gunluk").is_dir());
    assert!(veri.join("sube-pc").join("istemci-yollari.json").is_file());
    assert!(veri.join("yazdir").join("ayar.json").is_file());
    assert!(o.kok.join("sube-pc.json").is_file());

    // İnsan okur çıktı ve durum.
    let c = calistir(&[
        "sube-pc",
        "kur",
        "--merkez",
        "https://merkez.ornek.com",
        "--sube-adres",
        "http://192.168.1.20:8788",
        "--baslatma",
        "--gorevsiz",
        "--duvarsiz",
    ]);
    assert_eq!(kod(&c), 0, "{}", hata(&c));
    assert!(cikti(&c).contains(
        "Yol sırası: merkez (https://merkez.ornek.com) → sube (http://192.168.1.20:8788) → yerel"
    ));
    let c = calistir(&["sube-pc", "durum", "--json"]);
    assert_eq!(kod(&c), 0, "{}", hata(&c));
    let v: serde_json::Value = serde_json::from_str(&cikti(&c)).unwrap();
    assert_eq!(v["eksik"], serde_json::json!([]));
    assert_eq!(v["gunluk_var"], true);
}

/// Gerçek süreç başlatma (yalnız Unix'te: ikili yerine küçük bir kabuk betiği konur). Çıktı günlüğe eklenir.
#[cfg(unix)]
#[test]
fn sube_pc_baslat_gercek_surec() {
    use std::os::unix::fs::PermissionsExt;
    let o = Ortam::yeni();
    let veri = o.taban.join("veri");
    fs::create_dir_all(&o.kok).unwrap();
    let betik = o.kok.join("taa-sube-dugum");
    fs::write(&betik, "#!/bin/sh\necho \"dugum basladi: $*\"\n").unwrap();
    fs::set_permissions(&betik, fs::Permissions::from_mode(0o755)).unwrap();
    let calistir = |args: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_taa-kur"))
            .args(args)
            .env("TAA_KOK", &o.kok)
            .env("TAA_VERI_KOK", &veri)
            .output()
            .unwrap()
    };
    // Kapı 1: hiçbir şey dinlemez, "zaten çalışıyor" sayılmaz. Kullanılmayan yüksek bir kapı seçilir.
    let c = calistir(&[
        "sube-pc",
        "kur",
        "--sube-adres",
        "http://192.168.1.20:18788",
        "--port",
        "18788",
        "--gorevsiz",
        "--duvarsiz",
        "--json",
    ]);
    assert_eq!(kod(&c), 0, "{}", hata(&c));
    let v: serde_json::Value = serde_json::from_str(&cikti(&c)).unwrap();
    assert_eq!(v["hizmetler"][0]["durum"], "baslatildi", "{v}");
    let gunluk = veri
        .join("sube-pc")
        .join("cikti")
        .join("taa-sube-dugum.log");
    let mut metin = String::new();
    for _ in 0..50 {
        metin = fs::read_to_string(&gunluk).unwrap_or_default();
        if metin.contains("dugum basladi") {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(metin.contains("dugum basladi: calis --kok"), "{metin}");
    // 192.168.1.20 bu makinede yok: --lan olarak verilmez (verilseydi düğüm hiç açılamazdı).
    assert!(metin.contains("--port 18788"), "{metin}");
    assert!(!metin.contains("--lan 192.168.1.20"), "{metin}");
    // İkinci başlatma çıktıyı ezmez, ekler.
    assert_eq!(kod(&calistir(&["sube-pc", "baslat"])), 0);
    for _ in 0..50 {
        metin = fs::read_to_string(&gunluk).unwrap_or_default();
        if metin.matches("dugum basladi").count() == 2 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(metin.matches("dugum basladi").count(), 2, "{metin}");
}
