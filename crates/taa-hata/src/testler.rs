use super::*;
use std::cell::RefCell;
use taa_kuyruk::GonderHata;

#[test]
fn temizle_sirlari_ve_kisisel_veriyi_cikarir() {
    let t = temizle(
        r#"parola=abc123 ve bearer eyJhbGciOiJIUzI1NiJ9abcdefghijkl ali@x.com C:\Users\Yusuf\gizli\tutaa.db 05321234567 "Ali Yılmaz""#,
    );
    assert!(!t.contains("abc123"), "{t}");
    assert!(!t.contains("eyJ"), "{t}");
    assert!(!t.contains("ali@x.com"), "{t}");
    assert!(!t.contains("Yusuf"), "{t}");
    assert!(!t.contains("05321234567"), "{t}");
    assert!(!t.contains("Ali"), "{t}");
    assert!(t.contains("tutaa.db"), "{t}");
}

#[test]
fn ayni_hata_ayni_parmak_izi() {
    let a = Rapor::yeni(
        "p",
        "1.0",
        Tur::Cokme,
        "src/a.rs:10",
        "indeks 5 sınır dışı",
        &[],
    );
    let b = Rapor::yeni(
        "p",
        "1.0",
        Tur::Cokme,
        "src/a.rs:10",
        "indeks 9 sınır dışı",
        &[],
    );
    assert_eq!(a.parmak_izi, b.parmak_izi);
    let c = Rapor::yeni(
        "p",
        "1.0",
        Tur::Cokme,
        "src/b.rs:10",
        "indeks 9 sınır dışı",
        &[],
    );
    assert_ne!(a.parmak_izi, c.parmak_izi);
}

#[test]
fn kuyruga_yazar_tekrarda_tek_kayit() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    let r = Rapor::yeni(
        "taa-x",
        "1.0",
        Tur::Hata,
        "",
        "bir hata",
        &["satır".to_string()],
    );
    assert!(matches!(kuyruga_yaz(&kok, &r).unwrap(), Ekleme::Eklendi(_)));
    assert!(matches!(
        kuyruga_yaz(&kok, &r).unwrap(),
        Ekleme::ZatenVar(_)
    ));
}

struct Sahte(RefCell<Vec<Value>>);
impl Gonderici for Sahte {
    fn gonder(&self, uc: &str, govde: &Value) -> Result<Value, GonderHata> {
        assert_eq!(uc, UC);
        self.0.borrow_mut().push(govde.clone());
        Ok(Value::Null)
    }
}

#[test]
fn bekleyenleri_gonderir() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    let r = Rapor::yeni("taa-x", "1.0", Tur::Cokme, "src/a.rs:1", "çöktü", &[]);
    kuyruga_yaz(&kok, &r).unwrap();
    let g = Sahte(RefCell::new(vec![]));
    assert_eq!(bekleyenleri_gonder(&kok, &g), 1);
    assert_eq!(g.0.borrow()[0]["program"], "taa-x");
    assert_eq!(bekleyenleri_gonder(&kok, &g), 0);
}

#[test]
fn kuyruk_dosyasinda_mesaj_acik_yazilmaz() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    let r = Rapor::yeni("taa-z", "1.0", Tur::Hata, "", "benzersizifade", &[]);
    kuyruga_yaz(&kok, &r).unwrap();
    let ham = std::fs::read_to_string(kok.join("kuyruk/hata-taa-z/giden.jsonl")).unwrap();
    assert!(
        !ham.contains("benzersizifade") && ham.contains("_sifreli"),
        "{ham}"
    );
}
