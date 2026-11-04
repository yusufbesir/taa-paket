//! taa-donustur testleri: güvenli yol vakaları, zip-ac (zip-slip, üzerine yazmama, sınırlar), zip/gzip gidiş-dönüş, imza.
//! Sahip: Yusuf (TaaDön projesi). yazan: [Claude#taadon-rust-asama1 2026-11-04], [Claude#taadon-rust-asama2 2026-11-04] (ad: taa-donustur).
//! Değişiklik: Yusuf'un kuralları çerçevesinde; yalnız ekleme/düzeltme. Testler yalnız kendi geçici klasörlerini kaldırır.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use taa_donustur::arsiv::{self, guvenli_yol};
use taa_donustur::{imza, Ilerleme};

struct GeciciKlasor(PathBuf);

impl GeciciKlasor {
    fn yeni(ad: &str) -> Self {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p =
            std::env::temp_dir().join(format!("taa-donustur-test-{ad}-{}-{n}", std::process::id()));
        fs::create_dir_all(&p).unwrap();
        GeciciKlasor(p)
    }
}

impl Drop for GeciciKlasor {
    fn drop(&mut self) {
        // yalnız bu testin kendi oluşturduğu geçici klasör
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn zip_yaz(yol: &Path, girdiler: &[(&str, &[u8])]) {
    let f = fs::File::create(yol).unwrap();
    let mut z = zip::ZipWriter::new(f);
    let sec = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    for (ad, veri) in girdiler {
        if ad.ends_with('/') {
            z.add_directory(*ad, sec).unwrap();
        } else {
            z.start_file(*ad, sec).unwrap();
            z.write_all(veri).unwrap();
        }
    }
    z.finish().unwrap();
}

#[test]
fn guvenli_yol_vakalari() {
    assert_eq!(
        guvenli_yol("a/b.txt").unwrap(),
        Some(vec!["a".to_string(), "b.txt".to_string()])
    );
    assert_eq!(
        guvenli_yol("a\\b.txt").unwrap(),
        Some(vec!["a".to_string(), "b.txt".to_string()])
    );
    assert!(guvenli_yol("../x.txt").unwrap_err().contains("üst klasöre"));
    assert!(guvenli_yol("a/../../x")
        .unwrap_err()
        .contains("üst klasöre"));
    assert!(guvenli_yol("a/..").unwrap_err().contains("üst klasöre"));
    assert!(guvenli_yol("/etc/passwd")
        .unwrap_err()
        .contains("mutlak yol"));
    assert!(guvenli_yol("\\windows\\x")
        .unwrap_err()
        .contains("mutlak yol"));
    assert!(guvenli_yol("C:/x.txt").unwrap_err().contains("mutlak yol"));
    assert!(guvenli_yol("c:x.txt").unwrap_err().contains("mutlak yol"));
    assert!(guvenli_yol("a\u{0}b")
        .unwrap_err()
        .contains("geçersiz dosya adı"));
    assert!(guvenli_yol("").unwrap_err().contains("geçersiz dosya adı"));
    assert_eq!(guvenli_yol("./").unwrap(), None);
    assert_eq!(
        guvenli_yol("./a.txt").unwrap(),
        Some(vec!["a.txt".to_string()])
    );
    assert_eq!(
        guvenli_yol("CON.txt").unwrap(),
        Some(vec!["_CON.txt".to_string()])
    );
    assert_eq!(
        guvenli_yol("d/lpt1").unwrap(),
        Some(vec!["d".to_string(), "_lpt1".to_string()])
    );
    assert_eq!(
        guvenli_yol("com0.txt").unwrap(),
        Some(vec!["com0.txt".to_string()])
    );
    assert_eq!(
        guvenli_yol("console.txt").unwrap(),
        Some(vec!["console.txt".to_string()])
    );
    assert!(guvenli_yol("a:b.txt").unwrap_err().contains("mutlak yol")); // Node: /^[A-Za-z]:/
    assert_eq!(
        guvenli_yol("x/a:b*c?.txt").unwrap(),
        Some(vec!["x".to_string(), "a_b_c_.txt".to_string()])
    );
    assert_eq!(guvenli_yol("ad. . ").unwrap(), Some(vec!["ad".to_string()]));
    assert_eq!(guvenli_yol("...").unwrap(), Some(vec!["_".to_string()]));
    assert_eq!(
        guvenli_yol("x/\u{1}y").unwrap(),
        Some(vec!["x".to_string(), "_y".to_string()])
    );
}

#[test]
fn zip_ac_guvenli_cikarma_ve_ad_cakismasi() {
    let g = GeciciKlasor::yeni("ac");
    let zip = g.0.join("ornek.zip");
    zip_yaz(
        &zip,
        &[
            ("a.txt", b"merhaba"),
            ("alt/", b""),
            ("alt/b.txt", b"dunya"),
            ("CON.txt", b"x"),
            ("A.TXT", b"buyuk"),
        ],
    );
    let hedef = g.0.join("cikti");
    let s = arsiv::zip_ac(&zip, &hedef, 1 << 20, 100, &mut Ilerleme::kapali()).unwrap();
    assert_eq!(s.dosya_sayisi, 5);
    assert_eq!(fs::read(hedef.join("a.txt")).unwrap(), b"merhaba");
    assert_eq!(fs::read(hedef.join("alt").join("b.txt")).unwrap(), b"dunya");
    assert_eq!(fs::read(hedef.join("_CON.txt")).unwrap(), b"x");
    // Windows'ta büyük/küçük harf çakışması "(1)" ekiyle ayrı dosya olur, üzerine yazılmaz
    if cfg!(windows) {
        assert_eq!(fs::read(hedef.join("A (1).TXT")).unwrap(), b"buyuk");
    }
    // ikinci açma: var olan dosyaların üzerine yazmaz
    arsiv::zip_ac(&zip, &hedef, 1 << 20, 100, &mut Ilerleme::kapali()).unwrap();
    assert_eq!(fs::read(hedef.join("a.txt")).unwrap(), b"merhaba");
    assert!(hedef.join("a (1).txt").exists());
}

#[test]
fn zip_ac_zip_slip_ve_mutlak_reddi() {
    let g = GeciciKlasor::yeni("slip");
    for (ad, beklenen) in [
        ("../kacak.txt", "üst klasöre"),
        ("/mutlak.txt", "mutlak yol"),
        ("C:/x.txt", "mutlak yol"),
    ] {
        let zip = g.0.join("kotu.zip");
        let _ = fs::remove_file(&zip);
        zip_yaz(&zip, &[("iyi.txt", b"1"), (ad, b"2")]);
        let hedef = g.0.join("c");
        let e = arsiv::zip_ac(&zip, &hedef, 1 << 20, 100, &mut Ilerleme::kapali()).unwrap_err();
        assert!(e.0.contains(beklenen), "{ad}: {}", e.0);
        assert!(!g.0.join("kacak.txt").exists());
        let liste = arsiv::zip_listele(&zip).unwrap();
        assert!(!liste.guvenli);
        assert!(liste
            .girdiler
            .iter()
            .any(|x| !x.guvenli && x.neden.as_deref().unwrap_or("").contains(beklenen)));
    }
}

#[test]
fn zip_ac_sinirlar() {
    let g = GeciciKlasor::yeni("sinir");
    let zip = g.0.join("buyuk.zip");
    let veri = vec![b'x'; 10_000];
    zip_yaz(&zip, &[("a.bin", &veri), ("b.bin", &veri)]);
    let e = arsiv::zip_ac(&zip, &g.0.join("c1"), 15_000, 100, &mut Ilerleme::kapali()).unwrap_err();
    assert!(e.0.contains("sınırı aşıyor"), "{}", e.0);
    let e = arsiv::zip_ac(&zip, &g.0.join("c2"), 1 << 20, 1, &mut Ilerleme::kapali()).unwrap_err();
    assert!(e.0.contains("çok fazla dosya"), "{}", e.0);
}

#[test]
fn zip_olustur_ve_ac_gidis_donus() {
    let g = GeciciKlasor::yeni("olustur");
    let kaynak = g.0.join("kaynak");
    fs::create_dir_all(kaynak.join("alt").join("bos")).unwrap();
    fs::write(kaynak.join("metin.txt"), "merhaba dünya ".repeat(1000)).unwrap();
    fs::write(
        kaynak.join("alt").join("resim.png"),
        [0x89, 0x50, 0x4e, 0x47, 1, 2, 3],
    )
    .unwrap();
    let zip = g.0.join("cikti.zip");
    let s = arsiv::zip_olustur(
        &zip,
        std::slice::from_ref(&kaynak),
        100,
        &mut Ilerleme::kapali(),
    )
    .unwrap();
    assert_eq!(s.girdi_sayisi, 3);
    // var olan çıktının üzerine yazmaz
    let e = arsiv::zip_olustur(
        &zip,
        std::slice::from_ref(&kaynak),
        100,
        &mut Ilerleme::kapali(),
    )
    .unwrap_err();
    assert!(e.0.contains("zaten var"));
    let mut a = zip::ZipArchive::new(fs::File::open(&zip).unwrap()).unwrap();
    let png = a.by_name("kaynak/alt/resim.png").unwrap();
    assert_eq!(png.compression(), zip::CompressionMethod::Stored);
    drop(png);
    assert_eq!(
        a.by_name("kaynak/metin.txt").unwrap().compression(),
        zip::CompressionMethod::Deflated
    );
    assert!(a.by_name("kaynak/alt/bos/").is_ok());
    let hedef = g.0.join("acilan");
    arsiv::zip_ac(&zip, &hedef, 1 << 30, 100, &mut Ilerleme::kapali()).unwrap();
    assert_eq!(
        fs::read(hedef.join("kaynak").join("metin.txt")).unwrap(),
        fs::read(kaynak.join("metin.txt")).unwrap()
    );
    assert!(hedef.join("kaynak").join("alt").join("bos").is_dir());
    // sınır: maks dosya
    let e =
        arsiv::zip_olustur(&g.0.join("x.zip"), &[kaynak], 2, &mut Ilerleme::kapali()).unwrap_err();
    assert!(e.0.contains("Çok fazla dosya"));
    assert!(!g.0.join("x.zip").exists());
}

#[test]
fn gzip_gidis_donus_ve_sinir() {
    let g = GeciciKlasor::yeni("gz");
    let girdi = g.0.join("veri.txt");
    fs::write(&girdi, "abc".repeat(50_000)).unwrap();
    let gz = g.0.join("veri.txt.gz");
    arsiv::gzip(&girdi, &gz, &mut Ilerleme::kapali()).unwrap();
    assert!(arsiv::gzip(&girdi, &gz, &mut Ilerleme::kapali())
        .unwrap_err()
        .0
        .contains("zaten var"));
    let geri = g.0.join("geri.txt");
    let s = arsiv::gunzip(&gz, &geri, 1 << 30, &mut Ilerleme::kapali()).unwrap();
    assert_eq!(s.toplam_bayt, 150_000);
    assert_eq!(fs::read(&geri).unwrap(), fs::read(&girdi).unwrap());
    let e = arsiv::gunzip(&gz, &g.0.join("kucuk.txt"), 1000, &mut Ilerleme::kapali()).unwrap_err();
    assert!(e.0.contains("sınırı aşıyor"));
    assert!(!g.0.join("kucuk.txt").exists());
    let bozuk = g.0.join("bozuk.gz");
    let b = fs::read(&gz).unwrap();
    fs::write(&bozuk, &b[..b.len() / 2]).unwrap();
    assert!(
        arsiv::gunzip(&bozuk, &g.0.join("b.txt"), 1 << 30, &mut Ilerleme::kapali())
            .unwrap_err()
            .0
            .contains("bozuk")
    );
    assert!(
        arsiv::gzip(&g.0, &g.0.join("k.gz"), &mut Ilerleme::kapali())
            .unwrap_err()
            .0
            .contains("yalnız dosyaları")
    );
}

#[test]
fn imza_kurallari() {
    let g = GeciciKlasor::yeni("imza");
    let pdf = g.0.join("a.pdf");
    fs::write(&pdf, b"%PDF-1.4\n").unwrap();
    assert!(imza::denetle(&pdf).unwrap().imza_uygun);
    let sahte = g.0.join("b.PDF");
    fs::write(&sahte, b"merhaba").unwrap();
    let s = imza::denetle(&sahte).unwrap();
    assert_eq!(s.uzanti, "pdf");
    assert!(!s.imza_uygun);
    let bos = g.0.join("c.png");
    fs::write(&bos, b"").unwrap();
    assert!(!imza::denetle(&bos).unwrap().imza_uygun);
    let kisa = g.0.join("d.png");
    fs::write(&kisa, [0x89, 0x50]).unwrap();
    assert!(!imza::denetle(&kisa).unwrap().imza_uygun);
    let txt = g.0.join("e.txt");
    fs::write(&txt, b"x").unwrap();
    assert!(imza::denetle(&txt).unwrap().imza_uygun);
    assert!(imza::denetle(&g.0).unwrap().imza_uygun);
    assert!(imza::denetle(&g.0.join("yok.pdf")).is_err());
}
