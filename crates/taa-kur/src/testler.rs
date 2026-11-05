// Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05]
//! §1.5 zorunlu testleri: hepsi geçici kökte (taa_dosya::gecici_klasor); testlerin kendi geçici klasörleri sonda silinir.

use crate::paket::{icerik_hesapla, sha256_bayt, DosyaKaydi};
use crate::*;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

struct Ortam {
    taban: PathBuf,
    kur: Kur,
}

impl Ortam {
    fn yeni() -> Ortam {
        let taban = taa_dosya::gecici_klasor().unwrap();
        let kur = Kur::yeni(taban.join("kok")).unwrap();
        Ortam { taban, kur }
    }
    fn paket(
        &self,
        ad: &str,
        surum: &str,
        arayuz: &str,
        gerek: &[(&str, &str)],
        dosyalar: &[(&str, &str)],
    ) -> PathBuf {
        paket_yap(&self.taban, ad, surum, arayuz, "ortak", gerek, dosyalar)
    }
}

impl Drop for Ortam {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.taban);
    }
}

pub(crate) fn paket_yap(
    taban: &Path,
    ad: &str,
    surum: &str,
    arayuz: &str,
    program: &str,
    gerek: &[(&str, &str)],
    dosyalar: &[(&str, &str)],
) -> PathBuf {
    let k = taban
        .join("paketler")
        .join(format!("{ad}-{surum}-{arayuz}-{program}"));
    fs::create_dir_all(&k).unwrap();
    let mut liste = Vec::new();
    for (yol, icerik) in dosyalar {
        let h = k.join(yol);
        fs::create_dir_all(h.parent().unwrap()).unwrap();
        fs::write(&h, icerik).unwrap();
        liste.push(DosyaKaydi {
            yol: yol.to_string(),
            sha256: sha256_bayt(icerik.as_bytes()),
        });
    }
    let g: BTreeMap<String, String> = gerek
        .iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    let p = Paket {
        sema: 1,
        ad: ad.into(),
        surum: surum.into(),
        arayuz: arayuz.into(),
        program: program.into(),
        gerektirir: g,
        sunar: vec![],
        ister: vec![],
        veri_sahibi: vec![],
        veri_okur: vec![],
        icerik_sha256: icerik_hesapla(&liste),
        dosyalar: liste,
        imza: None,
    };
    fs::write(k.join("paket.json"), serde_json::to_vec_pretty(&p).unwrap()).unwrap();
    k
}

fn paket_degistir(klasor: &Path, f: impl FnOnce(&mut Paket)) {
    let mut p: Paket =
        serde_json::from_slice(&fs::read(klasor.join("paket.json")).unwrap()).unwrap();
    f(&mut p);
    fs::write(
        klasor.join("paket.json"),
        serde_json::to_vec_pretty(&p).unwrap(),
    )
    .unwrap();
}

fn goruntu(kok: &Path) -> BTreeMap<String, Vec<u8>> {
    fn gez(taban: &Path, d: &Path, m: &mut BTreeMap<String, Vec<u8>>) {
        let Ok(rd) = fs::read_dir(d) else { return };
        for o in rd.flatten() {
            let p = o.path();
            if p.is_dir() {
                gez(taban, &p, m);
            } else {
                m.insert(
                    p.strip_prefix(taban)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(&p).unwrap(),
                );
            }
        }
    }
    let mut m = BTreeMap::new();
    gez(kok, kok, &mut m);
    m.retain(|k, _| k != "gunluk.jsonl" && k != "kilit.serbest");
    m
}

#[test]
fn tam_senaryo_kur_liste_sec_geri_don_kaldir() {
    let o = Ortam::yeni();
    let v1 = o.paket(
        "taa-ornek",
        "0.1.0",
        "1",
        &[],
        &[("bin/a.exe", "bir"), ("veri/ek.txt", "ek")],
    );
    let v2 = o.paket("taa-ornek", "0.2.0", "1", &[], &[("bin/a.exe", "iki")]);
    let s = o.kur.kur(&v1, false).unwrap();
    assert!(s.secili, "ilk kurulum otomatik seçili");
    let l = o.kur.liste().unwrap();
    assert_eq!(l.len(), 1);
    assert_eq!(l[0].secili.as_deref(), Some("0.1.0"));
    // ikinci sürüm yan yana, seçimi değiştirmez
    let s2 = o.kur.kur(&v2, false).unwrap();
    assert!(!s2.secili);
    let l = o.kur.liste().unwrap();
    assert_eq!(l[0].kurulu, vec!["0.1.0", "0.2.0"]);
    assert_eq!(l[0].secili.as_deref(), Some("0.1.0"));
    assert!(o.kur.kok().join("depo/taa-ornek/0.1.0/bin/a.exe").is_file());
    assert!(o.kur.kok().join("depo/taa-ornek/0.2.0/bin/a.exe").is_file());
    // sec -> geri-don
    o.kur.sec("taa-ornek", "0.2.0").unwrap();
    let l = o.kur.liste().unwrap();
    assert_eq!(
        (l[0].secili.as_deref(), l[0].onceki.as_deref()),
        (Some("0.2.0"), Some("0.1.0"))
    );
    assert_eq!(o.kur.geri_don("taa-ornek").unwrap(), "0.1.0");
    assert_eq!(o.kur.liste().unwrap()[0].secili.as_deref(), Some("0.1.0"));
    let sk: serde_json::Value = serde_json::from_slice(
        &fs::read(o.kur.kok().join("programlar/taa-ornek/secili.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(sk["surum"], "0.1.0");
    assert!(o.kur.dogrula(None).unwrap().is_empty());
    // kaldır: seçili olan 0.1.0 gider, 0.2.0'a dönülür
    let k = o.kur.kaldir("taa-ornek", None).unwrap();
    assert_eq!(k.surum, "0.1.0");
    assert_eq!(k.yeni_secili.as_deref(), Some("0.2.0"));
    assert!(o.kur.dogrula(None).unwrap().is_empty());
    let k = o.kur.kaldir("taa-ornek", Some("0.2.0")).unwrap();
    assert_eq!(k.yeni_secili, None);
    assert!(o.kur.liste().unwrap().is_empty());
    // hiçbir şey silinmedi: iki sürüm de eski/ altında
    let eski: Vec<_> = fs::read_dir(o.kur.kok().join("eski"))
        .unwrap()
        .flatten()
        .collect();
    assert!(
        eski.len() >= 3,
        "iki sürüm ve secili.json eski/ altında olmalı"
    );
    assert!(o.kur.dogrula(None).unwrap().is_empty());
}

#[test]
fn cozum_yok_reddi_ve_arayuz_uyumsuzlugu() {
    let o = Ortam::yeni();
    let p = o.paket(
        "taa-uygulama",
        "0.1.0",
        "1",
        &[("taa-nebim", "1")],
        &[("a", "x")],
    );
    let hata = o.kur.kur(&p, false).unwrap_err();
    assert_eq!(hata.cikis_kodu(), 3);
    assert!(
        hata.mesaj().contains("taa-nebim") && hata.mesaj().contains("arayüz 1"),
        "{hata}"
    );
    assert!(hata.mesaj().contains("taa-kur kur taa-nebim"));
    assert!(!o.kur.kok().join("kayit.json").exists());
    assert!(!o.kur.kok().join("depo").exists());
    // arayüz 2 sunan nebim kurulu: yine reddedilir
    let n2 = o.paket("taa-nebim", "2.0.0", "2", &[], &[("n", "n")]);
    o.kur.kur(&n2, false).unwrap();
    let hata = o.kur.kur(&p, false).unwrap_err();
    assert_eq!(hata.cikis_kodu(), 3);
    assert!(hata.mesaj().contains("arayüz 2"), "{hata}");
    // uyumlu sürüm kurulup seçilince kurulum geçer
    let n1 = o.paket("taa-nebim", "1.0.0", "1", &[], &[("n", "n1")]);
    o.kur.kur(&n1, true).unwrap();
    o.kur.kur(&p, false).unwrap();
    // seçimi arayüz 2'ye çevirmek, onu gerektireni bozacağı için reddedilir
    let hata = o.kur.sec("taa-nebim", "2.0.0").unwrap_err();
    assert_eq!(hata.cikis_kodu(), 3);
    assert_eq!(
        o.kur
            .liste()
            .unwrap()
            .iter()
            .find(|l| l.ad == "taa-nebim")
            .unwrap()
            .secili
            .as_deref(),
        Some("1.0.0")
    );
}

#[test]
fn ince_cizgi_baska_programa_baglanamaz() {
    let o = Ortam::yeni();
    let hedef = paket_yap(
        &o.taban,
        "taa-tutaa",
        "0.1.0",
        "1",
        "tutaa",
        &[],
        &[("a", "x")],
    );
    o.kur.kur(&hedef, false).unwrap();
    let kotu = paket_yap(
        &o.taban,
        "taa-rehber",
        "0.1.0",
        "1",
        "taarehber",
        &[("taa-tutaa", "1")],
        &[("b", "y")],
    );
    let h = o.kur.kur(&kotu, false).unwrap_err();
    assert_eq!(h.cikis_kodu(), 3);
    assert!(h.mesaj().contains("başka programın"), "{h}");
    // aynı programın paketi serbest
    let iyi = paket_yap(
        &o.taban,
        "taa-tutaa-ek",
        "0.1.0",
        "1",
        "tutaa",
        &[("taa-tutaa", "1")],
        &[("c", "z")],
    );
    o.kur.kur(&iyi, false).unwrap();
}

#[test]
fn sha256_uyusmazsa_hicbir_sey_degismez() {
    let o = Ortam::yeni();
    let v1 = o.paket("taa-ornek", "0.1.0", "1", &[], &[("a.txt", "ilk")]);
    o.kur.kur(&v1, false).unwrap();
    let once = goruntu(o.kur.kok());
    // yeni sürüm: dosya listede yazan hash'ten farklı
    let v2 = o.paket("taa-ornek", "0.2.0", "1", &[], &[("a.txt", "iki")]);
    fs::write(v2.join("a.txt"), "BOZULDU").unwrap();
    let h = o.kur.kur(&v2, true).unwrap_err();
    assert_eq!(h.cikis_kodu(), 2);
    assert!(h.mesaj().contains("sha256"), "{h}");
    // aynı sürümü bozuk paketle üzerine kurmak da depoyu değiştirmez
    let v1b = o.paket("taa-ornek", "0.1.0", "1", &[], &[("a.txt", "farkli")]);
    fs::write(v1b.join("a.txt"), "BOZUK").unwrap();
    assert_eq!(o.kur.kur(&v1b, true).unwrap_err().cikis_kodu(), 2);
    assert_eq!(goruntu(o.kur.kok()), once);
}

#[test]
fn yol_kacisi_reddedilir() {
    let o = Ortam::yeni();
    for kotu in [
        "../disari.txt",
        "a/../../b",
        "/mutlak",
        "C:/x.txt",
        "a\\b",
        "a//b",
    ] {
        let k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("iyi.txt", "x")]);
        paket_degistir(&k, |p| {
            p.dosyalar[0].yol = kotu.into();
            p.icerik_sha256 = icerik_hesapla(&p.dosyalar);
        });
        let h = o.kur.kur(&k, false).unwrap_err();
        assert_eq!(h.cikis_kodu(), 2, "{kotu}: {h}");
    }
    assert!(!o.taban.join("disari.txt").exists());
    assert!(!o.kur.kok().join("depo").exists());
    // NUL karakteri
    let k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("iyi.txt", "x")]);
    paket_degistir(&k, |p| {
        p.dosyalar[0].yol = "a\0b".into();
        p.icerik_sha256 = icerik_hesapla(&p.dosyalar);
    });
    assert_eq!(o.kur.kur(&k, false).unwrap_err().cikis_kodu(), 2);
    // geçersiz ad (yol kaçışı adla)
    let k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("iyi.txt", "x")]);
    paket_degistir(&k, |p| p.ad = "../kacis".into());
    assert_eq!(o.kur.kur(&k, false).unwrap_err().cikis_kodu(), 2);
}

#[test]
fn sinirlar_ve_yinelenen_yol() {
    let o = Ortam::yeni();
    let k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("a", "x")]);
    paket_degistir(&k, |p| {
        let d = p.dosyalar[0].clone();
        p.dosyalar = (0..=paket::MAKS_DOSYA)
            .map(|i| DosyaKaydi {
                yol: format!("f{i}"),
                ..d.clone()
            })
            .collect();
        p.icerik_sha256 = icerik_hesapla(&p.dosyalar);
    });
    let h = o.kur.kur(&k, false).unwrap_err();
    assert_eq!(h.cikis_kodu(), 2);
    assert!(h.mesaj().contains("sınırı"), "{h}");
    let k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("a", "x")]);
    paket_degistir(&k, |p| {
        p.dosyalar.push(p.dosyalar[0].clone());
        p.icerik_sha256 = icerik_hesapla(&p.dosyalar);
    });
    assert_eq!(o.kur.kur(&k, false).unwrap_err().cikis_kodu(), 2);
}

#[cfg(unix)]
#[test]
fn sembolik_bag_izlenmez() {
    let o = Ortam::yeni();
    let k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("gercek.txt", "x")]);
    let gizli = o.taban.join("gizli.txt");
    fs::write(&gizli, "x").unwrap(); // aynı içerik: hash eşleşse bile bağ reddedilmeli
    fs::remove_file(k.join("gercek.txt")).unwrap();
    std::os::unix::fs::symlink(&gizli, k.join("gercek.txt")).unwrap();
    let h = o.kur.kur(&k, false).unwrap_err();
    assert_eq!(h.cikis_kodu(), 2);
    assert!(h.mesaj().contains("sembolik"), "{h}");
}

#[test]
fn kaldirmada_eski_icerigi_birebir() {
    let o = Ortam::yeni();
    let k = o.paket(
        "taa-ornek",
        "0.1.0",
        "1",
        &[],
        &[("bin/a.exe", "ikili\0veri"), ("belge/oku.txt", "merhaba")],
    );
    o.kur.kur(&k, false).unwrap();
    let depoda = goruntu(&o.kur.kok().join("depo/taa-ornek/0.1.0"));
    let s = o.kur.kaldir("taa-ornek", None).unwrap();
    assert!(!o.kur.kok().join("depo/taa-ornek/0.1.0").exists());
    assert_eq!(
        goruntu(&s.eski),
        depoda,
        "eski/ içeriği birebir aynı olmalı"
    );
    let ad = s.eski.file_name().unwrap().to_string_lossy().into_owned();
    assert!(ad.starts_with("taa-ornek-0.1.0-"), "{ad}");
    // aynı sürümü üzerine kurmak da eskiyi eski/ altına taşır
    o.kur.kur(&k, false).unwrap();
    let yeni_k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("bin/a.exe", "YENI")]);
    let r = o.kur.kur(&yeni_k, false).unwrap();
    let eski = r.eski.expect("üzerine yazılan içerik eski/ altında olmalı");
    assert_eq!(fs::read(eski.join("bin/a.exe")).unwrap(), b"ikili\0veri");
    assert_eq!(
        fs::read(o.kur.kok().join("depo/taa-ornek/0.1.0/bin/a.exe")).unwrap(),
        b"YENI"
    );
}

#[test]
fn veri_alanina_dokunulmaz() {
    let o = Ortam::yeni();
    let veri = o.taban.join("veri");
    fs::create_dir_all(veri.join("rehber")).unwrap();
    fs::write(veri.join("rehber/kayit.db"), "ÖNEMLİ").unwrap();
    let kok_icinde_veri = o.kur.kok().join("veri");
    fs::create_dir_all(&kok_icinde_veri).unwrap();
    fs::write(kok_icinde_veri.join("x.txt"), "x").unwrap();
    let once = (goruntu(&veri), goruntu(&kok_icinde_veri));
    let k = o.paket("taa-rehber", "0.1.0", "1", &[], &[("a", "x")]);
    paket_degistir(&k, |p| p.veri_sahibi = vec!["rehber".into()]);
    o.kur.kur(&k, false).unwrap();
    o.kur.kaldir("taa-rehber", None).unwrap();
    assert_eq!((goruntu(&veri), goruntu(&kok_icinde_veri)), once);
}

#[test]
fn kaldirmada_salt_okunur_listesi_ve_sahiplik() {
    let o = Ortam::yeni();
    let sahip = o.paket("taa-nebim", "0.1.0", "1", &[], &[("a", "x")]);
    paket_degistir(&sahip, |p| p.veri_sahibi = vec!["nebim".into()]);
    let okur = o.paket(
        "taa-rapor",
        "0.1.0",
        "1",
        &[("taa-nebim", "1")],
        &[("b", "y")],
    );
    paket_degistir(&okur, |p| p.veri_okur = vec!["nebim".into()]);
    o.kur.kur(&sahip, false).unwrap();
    o.kur.kur(&okur, false).unwrap();
    // başka paket aynı alanı sahiplenemez
    let hirsiz = o.paket("taa-baska", "0.1.0", "1", &[], &[("c", "z")]);
    paket_degistir(&hirsiz, |p| p.veri_sahibi = vec!["nebim".into()]);
    assert_eq!(o.kur.kur(&hirsiz, false).unwrap_err().cikis_kodu(), 3);
    let s = o.kur.kaldir("taa-nebim", None).unwrap();
    assert_eq!(s.salt_okunur, vec!["taa-rapor"]);
    assert!(s.uyarilar.iter().any(|u| u.contains("taa-rapor")));
}

#[test]
fn kayit_atomik_ve_butunluk() {
    let o = Ortam::yeni();
    let k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("a", "x")]);
    o.kur.kur(&k, false).unwrap();
    o.kur.sabitle("taa-ornek", true).unwrap();
    // yarım/geçici dosya kalmaz
    for (yol, _) in goruntu(o.kur.kok()) {
        assert!(!yol.contains(".tmp") && !yol.contains(".kur-"), "{yol}");
    }
    let bayt = fs::read(o.kur.kok().join("kayit.json")).unwrap();
    // tek dosya: .sha256 yazılmaz, özet kayıt içinde
    assert!(!o.kur.kok().join("kayit.json.sha256").exists());
    let kayit = serde_json::from_slice::<Kayit>(&bayt).unwrap();
    assert!(!kayit.ozet.is_empty() && kayit.ozet == kayit.ozet_hesapla());
    // bütünlük bozulursa değiştiren işlemler reddedilir ve hiçbir şey değişmez; dogrula bildirir
    let bozuk = String::from_utf8(bayt.clone())
        .unwrap()
        .replace("\"sabitle\": true", "\"sabitle\": false");
    assert_ne!(bozuk.as_bytes(), &bayt[..]);
    fs::write(o.kur.kok().join("kayit.json"), bozuk).unwrap();
    let once = goruntu(o.kur.kok());
    let v2 = o.paket("taa-ornek", "0.2.0", "1", &[], &[("a", "y")]);
    assert_eq!(o.kur.kur(&v2, false).unwrap_err().cikis_kodu(), 5);
    assert_eq!(o.kur.kaldir("taa-ornek", None).unwrap_err().cikis_kodu(), 5);
    assert_eq!(goruntu(o.kur.kok()), once);
    let sorunlar = o.kur.dogrula(None).unwrap();
    assert!(
        sorunlar
            .iter()
            .any(|s| matches!(s, Sorun::Bozuk(m) if m.contains("kayit.json"))),
        "{sorunlar:?}"
    );
}

#[test]
fn dogrula_bozulan_depoyu_bulur() {
    let o = Ortam::yeni();
    let k = o.paket(
        "taa-ornek",
        "0.1.0",
        "1",
        &[],
        &[("a.txt", "x"), ("b.txt", "y")],
    );
    o.kur.kur(&k, false).unwrap();
    assert!(o.kur.dogrula(Some("taa-ornek")).unwrap().is_empty());
    fs::write(o.kur.kok().join("depo/taa-ornek/0.1.0/a.txt"), "degisti").unwrap();
    fs::rename(
        o.kur.kok().join("depo/taa-ornek/0.1.0/b.txt"),
        o.taban.join("b.yedek"),
    )
    .unwrap();
    let s = o.kur.dogrula(None).unwrap();
    assert!(
        s.iter()
            .any(|x| matches!(x, Sorun::Bozuk(m) if m.contains("a.txt"))),
        "{s:?}"
    );
    assert!(
        s.iter()
            .any(|x| matches!(x, Sorun::Eksik(m) if m.contains("b.txt"))),
        "{s:?}"
    );
    assert_eq!(
        o.kur.dogrula(Some("yok-paket")).unwrap_err().cikis_kodu(),
        4
    );
}

#[test]
fn geri_don_yoksa_4_ve_sabitle_ve_tani() {
    let o = Ortam::yeni();
    let k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("a", "x")]);
    o.kur.kur(&k, false).unwrap();
    assert_eq!(o.kur.geri_don("taa-ornek").unwrap_err().cikis_kodu(), 4);
    assert_eq!(o.kur.geri_don("taa-yok").unwrap_err().cikis_kodu(), 4);
    assert_eq!(o.kur.sec("taa-ornek", "9.9.9").unwrap_err().cikis_kodu(), 3);
    o.kur.sabitle("taa-ornek", true).unwrap();
    let v2 = o.paket("taa-ornek", "0.2.0", "1", &[], &[("a", "y")]);
    o.kur.kur(&v2, false).unwrap();
    assert_eq!(o.kur.liste().unwrap()[0].secili.as_deref(), Some("0.1.0"));
    // açıkça --sec istenirse değişir
    o.kur.kur(&v2, true).unwrap();
    assert_eq!(o.kur.liste().unwrap()[0].secili.as_deref(), Some("0.2.0"));
    let t = o.kur.tani().unwrap();
    assert!(t.kayit_saglam && t.sorunlar.is_empty());
    assert!(
        t.son_hatalar.iter().any(|h| h.contains("kur")),
        "{:?}",
        t.son_hatalar
    );
    assert!(serde_json::to_string(&t).is_ok());
}

#[test]
fn kok_cozumu_sirasi() {
    let taban = taa_dosya::gecici_klasor().unwrap();
    let varsayilan = taban.join("varsayilan");
    let (k, kay) = kok_coz_ile(None, &varsayilan);
    assert_eq!((k, kay), (varsayilan.clone(), KokKaynagi::Varsayilan));
    fs::create_dir_all(&varsayilan).unwrap();
    fs::write(
        varsayilan.join("kayit.json"),
        r#"{"sema":1,"kok":"/baska/kok"}"#,
    )
    .unwrap();
    let (k, kay) = kok_coz_ile(None, &varsayilan);
    assert_eq!((k, kay), (PathBuf::from("/baska/kok"), KokKaynagi::Kayit));
    let (k, kay) = kok_coz_ile(Some("/ortam/kok"), &varsayilan);
    assert_eq!((k, kay), (PathBuf::from("/ortam/kok"), KokKaynagi::Ortam));
    let (_, kay) = kok_coz_ile(Some("  "), &varsayilan);
    assert_eq!(kay, KokKaynagi::Kayit);
    let _ = fs::remove_dir_all(&taban);
}

// ---------------------------------------------------------------- güvenlik düzeltmeleri

#[test]
fn kayit_tek_dosya_ve_eski_sha256_okunmaz() {
    let o = Ortam::yeni();
    let k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("a", "x")]);
    o.kur.kur(&k, false).unwrap();
    let kok = o.kur.kok();
    // var olan (yanlış) .sha256 dosyası yok sayılır, silinmez
    fs::write(kok.join("kayit.json.sha256"), "yanlis").unwrap();
    assert!(o.kur.dogrula(None).unwrap().is_empty());
    o.kur.sabitle("taa-ornek", true).unwrap();
    assert_eq!(
        fs::read_to_string(kok.join("kayit.json.sha256")).unwrap(),
        "yanlis"
    );
    // ozet alanı silinirse (eski biçim) eksik sayılır, değiştiren işlem reddedilir
    let mut j: serde_json::Value =
        serde_json::from_slice(&fs::read(kok.join("kayit.json")).unwrap()).unwrap();
    j.as_object_mut().unwrap().remove("ozet");
    fs::write(
        kok.join("kayit.json"),
        serde_json::to_vec_pretty(&j).unwrap(),
    )
    .unwrap();
    let s = o.kur.dogrula(None).unwrap();
    assert!(
        s.iter()
            .any(|x| matches!(x, Sorun::Eksik(m) if m.contains("ozet"))),
        "{s:?}"
    );
    assert_eq!(
        o.kur.sabitle("taa-ornek", false).unwrap_err().cikis_kodu(),
        5
    );
    // ozet yanlışsa bozuk
    j["ozet"] = serde_json::Value::String("0".repeat(64));
    fs::write(
        kok.join("kayit.json"),
        serde_json::to_vec_pretty(&j).unwrap(),
    )
    .unwrap();
    let s = o.kur.dogrula(None).unwrap();
    assert!(
        s.iter()
            .any(|x| matches!(x, Sorun::Bozuk(m) if m.contains("ozet"))),
        "{s:?}"
    );
}

#[test]
fn kilit_alinir_ve_serbest_birakilir() {
    let o = Ortam::yeni();
    let kok = o.kur.kok().to_path_buf();
    fs::create_dir_all(&kok).unwrap();
    fs::write(kok.join("kilit.serbest"), "onceki").unwrap(); // önceki serbest kilit alımı engellemez
    {
        let _k = o.kur.kilit_al().unwrap();
        let icerik = fs::read_to_string(kok.join("kilit")).unwrap();
        assert!(
            icerik.contains(&format!("pid={}", std::process::id())),
            "{icerik}"
        );
        assert!(icerik.contains("zaman="), "{icerik}");
        // ikinci alma: Kilitli, çıkış 6
        let h = o.kur.kilit_al().err().unwrap();
        assert!(matches!(h, KurHata::Kilitli(_)));
        assert_eq!(h.cikis_kodu(), 6);
    }
    assert!(!kok.join("kilit").exists());
    let serbest = fs::read_to_string(kok.join("kilit.serbest")).unwrap();
    assert!(serbest.contains("pid="), "{serbest}"); // eskinin üzerine yazıldı
    o.kur.kilit_al().unwrap();
}

#[test]
fn degistiren_islemler_kilit_altinda() {
    let o = Ortam::yeni();
    let k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("a", "x")]);
    o.kur.kur(&k, false).unwrap();
    let kok = o.kur.kok().to_path_buf();
    let k2 = o.paket("taa-ornek", "0.2.0", "1", &[], &[("a", "y")]);
    o.kur.kur(&k2, false).unwrap();
    // taze kilit varken her değiştiren işlem 6 ile reddedilir, hiçbir şey değişmez
    fs::write(
        kok.join("kilit"),
        format!("pid=1\nzaman={}\n", zaman::unix_saniye()),
    )
    .unwrap();
    let once = goruntu(&kok);
    let k3 = o.paket("taa-ornek", "0.3.0", "1", &[], &[("a", "z")]);
    assert_eq!(o.kur.kur(&k3, true).unwrap_err().cikis_kodu(), 6);
    assert_eq!(o.kur.kaldir("taa-ornek", None).unwrap_err().cikis_kodu(), 6);
    assert_eq!(o.kur.sec("taa-ornek", "0.2.0").unwrap_err().cikis_kodu(), 6);
    assert_eq!(o.kur.geri_don("taa-ornek").unwrap_err().cikis_kodu(), 6);
    assert_eq!(
        o.kur.sabitle("taa-ornek", true).unwrap_err().cikis_kodu(),
        6
    );
    assert_eq!(goruntu(&kok), once);
    // kilit kalkınca (rename) işlemler çalışır ve kilit yine serbest bırakılır
    fs::rename(kok.join("kilit"), kok.join("kilit.serbest")).unwrap();
    o.kur.sec("taa-ornek", "0.1.0").unwrap();
    assert!(!kok.join("kilit").exists());
    // hata ile biten işlem de kilidi bırakır
    assert!(o.kur.sec("taa-ornek", "9.9.9").is_err());
    assert!(!kok.join("kilit").exists());
}

#[test]
fn bayat_kilit_eskiye_tasinir() {
    let o = Ortam::yeni();
    let kok = o.kur.kok().to_path_buf();
    fs::create_dir_all(&kok).unwrap();
    let eski_zaman = zaman::unix_saniye() - 700;
    let icerik = format!("pid=1\nzaman={eski_zaman}\n");
    fs::write(kok.join("kilit"), &icerik).unwrap();
    let k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("a", "x")]);
    o.kur.kur(&k, false).unwrap();
    let tasinan: Vec<_> = fs::read_dir(kok.join("eski"))
        .unwrap()
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("kilit-"))
        .collect();
    assert_eq!(tasinan.len(), 1);
    assert_eq!(fs::read_to_string(tasinan[0].path()).unwrap(), icerik);
    assert!(!kok.join("kilit").exists());
    assert!(o.kur.liste().unwrap().len() == 1);
}

#[test]
fn kaldir_tasinamazsa_kayit_degismez() {
    let o = Ortam::yeni();
    let k = o.paket("taa-ornek", "0.1.0", "1", &[], &[("a", "x")]);
    o.kur.kur(&k, false).unwrap();
    let k2 = o.paket("taa-ornek", "0.2.0", "1", &[], &[("a", "y")]);
    o.kur.kur(&k2, true).unwrap();
    let kayit_once = fs::read(o.kur.kok().join("kayit.json")).unwrap();
    let secili_once = fs::read(o.kur.kok().join("programlar/taa-ornek/secili.json")).unwrap();
    RENAME_HATA.with(|c| c.set(true));
    let h = o.kur.kaldir("taa-ornek", None);
    RENAME_HATA.with(|c| c.set(false));
    let h = h.unwrap_err();
    assert!(h.mesaj().contains("taşınamadı, kayıt değişmedi"), "{h}");
    assert_eq!(
        fs::read(o.kur.kok().join("kayit.json")).unwrap(),
        kayit_once
    );
    assert_eq!(
        fs::read(o.kur.kok().join("programlar/taa-ornek/secili.json")).unwrap(),
        secili_once
    );
    // özgün klasör depoda duruyor, sürüm hâlâ kayıtlı
    assert_eq!(
        fs::read(o.kur.kok().join("depo/taa-ornek/0.2.0/a")).unwrap(),
        b"y"
    );
    let l = o.kur.liste().unwrap();
    assert_eq!(l[0].kurulu, vec!["0.1.0", "0.2.0"]);
    assert_eq!(l[0].secili.as_deref(), Some("0.2.0"));
    assert!(o.kur.dogrula(None).unwrap().is_empty());
}

#[test]
fn agac_kopyala_siniri_asinca_yarim_eskiye_tasinir() {
    let o = Ortam::yeni();
    let kaynak = o.taban.join("kaynak");
    fs::create_dir_all(kaynak.join("alt")).unwrap();
    fs::write(kaynak.join("a"), "123456").unwrap();
    fs::write(kaynak.join("alt/b"), "abcdef").unwrap();
    RENAME_HATA.with(|c| c.set(true));
    let r = o.kur.eskiye_tasi_sinirli(&kaynak, "deneme", 8);
    RENAME_HATA.with(|c| c.set(false));
    let h = r.unwrap_err();
    assert!(h.mesaj().contains("sınırını aştı"), "{h}");
    // kaynak yerinde ve eksiksiz
    assert_eq!(fs::read(kaynak.join("a")).unwrap(), b"123456");
    assert_eq!(fs::read(kaynak.join("alt/b")).unwrap(), b"abcdef");
    let adlar: Vec<String> = fs::read_dir(o.kur.kok().join("eski"))
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(adlar.len(), 1, "{adlar:?}");
    assert!(adlar[0].starts_with("yarim-deneme-"), "{adlar:?}");
    // yarım klasörde sınırı aşan tüm içerik yok (en çok bir dosya kopyalanmış)
    let yarim = goruntu(&o.kur.kok().join("eski").join(&adlar[0]));
    assert!(yarim.len() <= 1, "{yarim:?}");
    // sınır yeterliyse kopya tamam
    RENAME_HATA.with(|c| c.set(true));
    let r = o.kur.eskiye_tasi_sinirli(&kaynak, "tamam", 12);
    RENAME_HATA.with(|c| c.set(false));
    let (hedef, tasindi) = r.unwrap();
    assert!(!tasindi);
    assert_eq!(fs::read(hedef.join("alt/b")).unwrap(), b"abcdef");
}

#[test]
fn guvensiz_ad_surum_yol_reddi() {
    let o = Ortam::yeni();
    for (ad, surum) in [
        ("Taa-X", "0.1.0"),
        ("con", "0.1.0"),
        ("taa-x", "0.1.0-RC"),
        ("taa..x", "0.1.0"),
        ("taa-x", "1..0"),
    ] {
        let k = o.paket(ad, surum, "1", &[], &[("a", "x")]);
        assert_eq!(
            o.kur.kur(&k, false).unwrap_err().cikis_kodu(),
            2,
            "{ad} {surum}"
        );
    }
    let k = o.paket("taa-x", "0.1.0", "1", &[], &[("a", "x")]);
    paket_degistir(&k, |p| {
        p.dosyalar[0].yol = "a\nb".into();
        p.icerik_sha256 = icerik_hesapla(&p.dosyalar);
    });
    assert_eq!(o.kur.kur(&k, false).unwrap_err().cikis_kodu(), 2);
    assert_eq!(o.kur.kaldir("CON", None).unwrap_err().cikis_kodu(), 2);
}
