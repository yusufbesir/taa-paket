// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-kurulum 2026-11-27]
//! Şube bilgisayarı kurulum profili testleri (sahte sistem; gerçek schtasks/reg/netsh çağrılmaz).

use crate::sube_pc::*;
use crate::{Kur, KurHata};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::net::Ipv4Addr;
use std::path::{Path, PathBuf};

/// Çağrıları kaydeden sahte sistem.
struct Sahte {
    windows: bool,
    /// Bu programlar başarısız döner (yönetici yokmuş gibi).
    reddeden: Vec<&'static str>,
    acik_portlar: Vec<u16>,
    yerel: Option<Ipv4Addr>,
    /// Bu bilgisayarın arayüzlerindeki adresler.
    adresler: Vec<Ipv4Addr>,
    komutlar: RefCell<Vec<(String, Vec<String>)>>,
    baslatilan: RefCell<Vec<(PathBuf, Vec<String>, PathBuf)>>,
}

impl Sahte {
    fn yeni(windows: bool) -> Sahte {
        Sahte {
            windows,
            reddeden: vec![],
            acik_portlar: vec![],
            yerel: None,
            adresler: vec![Ipv4Addr::new(192, 168, 1, 20)],
            komutlar: RefCell::new(vec![]),
            baslatilan: RefCell::new(vec![]),
        }
    }

    fn programlar(&self) -> Vec<String> {
        self.komutlar
            .borrow()
            .iter()
            .map(|(p, _)| p.clone())
            .collect()
    }
}

impl Sistem for Sahte {
    fn calistir(&self, program: &str, argumanlar: &[String]) -> Result<(bool, String), String> {
        self.komutlar
            .borrow_mut()
            .push((program.to_string(), argumanlar.to_vec()));
        if self.reddeden.contains(&program) {
            Ok((false, "Erişim engellendi.".into()))
        } else {
            Ok((true, String::new()))
        }
    }

    fn baslat(&self, exe: &Path, argumanlar: &[String], cikti: &Path) -> Result<u32, String> {
        self.baslatilan.borrow_mut().push((
            exe.to_path_buf(),
            argumanlar.to_vec(),
            cikti.to_path_buf(),
        ));
        Ok(4242)
    }

    fn port_acik(&self, port: u16) -> bool {
        self.acik_portlar.contains(&port)
    }

    fn windows(&self) -> bool {
        self.windows
    }

    fn yerel_ag_adresi(&self) -> Option<Ipv4Addr> {
        self.yerel
    }

    fn adres_bu_bilgisayarda(&self, ip: Ipv4Addr) -> bool {
        self.adresler.contains(&ip)
    }
}

fn exe(ad: &str) -> String {
    if cfg!(windows) {
        format!("{ad}.exe")
    } else {
        ad.to_string()
    }
}

/// Kök + veri kökü; istenen ikililer kökte (boş dosya olarak) bulunur.
fn ortam(ikililer: &[&str]) -> (Kur, PathBuf, PathBuf) {
    let kok = taa_dosya::gecici_klasor().unwrap();
    let veri = taa_dosya::gecici_klasor().unwrap();
    for i in ikililer {
        std::fs::write(kok.join(exe(i)), b"").unwrap();
    }
    (Kur::yeni(&kok).unwrap(), kok, veri)
}

fn secenek() -> Secenek {
    Secenek {
        merkez: Some("https://merkez.ornek.com/".into()),
        sube_adresi: Some("http://192.168.1.20:8788".into()),
        lan_ac: true,
        ..Secenek::default()
    }
}

fn json_oku(yol: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(yol).unwrap()).unwrap()
}

#[test]
fn yol_listesi_sirasi_merkez_sube_yerel() {
    let v = yol_listesi(
        Some("https://Merkez.Ornek.com/kabuk/"),
        Some("http://192.168.1.20:8788"),
    )
    .unwrap();
    assert_eq!(
        v,
        json!([
            {"ad": "merkez", "url": "https://merkez.ornek.com"},
            {"ad": "sube", "url": "http://192.168.1.20:8788"},
            {"ad": "yerel"}
        ])
    );
    // Adresi bilinmeyen yol yazılmaz; yerel her zaman sonda.
    assert_eq!(yol_listesi(None, None).unwrap(), json!([{"ad": "yerel"}]));
    assert_eq!(
        yol_listesi(None, Some("http://10.0.0.5:8788")).unwrap(),
        json!([{"ad": "sube", "url": "http://10.0.0.5:8788"}, {"ad": "yerel"}])
    );
    // Merkez düz http olamaz.
    assert!(yol_listesi(Some("http://merkez.ornek.com"), None).is_err());
}

#[test]
fn koken_kurallari() {
    assert_eq!(koken(" HTTPS://A.b:8443/x/y ").unwrap(), "https://a.b:8443");
    assert_eq!(koken("http://192.168.1.2").unwrap(), "http://192.168.1.2");
    for kotu in [
        "merkez.ornek.com",
        "ftp://a",
        "https://",
        "https://kullanici@a.b",
        "https://a.b/?x=1",
        "https://a.b:0",
        "https://a.b:99999",
        "https://a b",
    ] {
        assert!(koken(kotu).is_err(), "{kotu} reddedilmeli");
    }
}

#[test]
fn yerel_ag_araliklari() {
    for (ip, beklenen) in [
        ("10.1.2.3", true),
        ("172.16.0.1", true),
        ("172.31.255.1", true),
        ("172.32.0.1", false),
        ("192.168.1.20", true),
        ("100.64.0.1", false), // Tailscale: düğüm bu arayüzde dinlemez
        ("8.8.8.8", false),
        ("127.0.0.1", false),
    ] {
        assert_eq!(
            yerel_ag_ipv4(ip.parse::<Ipv4Addr>().unwrap()),
            beklenen,
            "{ip}"
        );
    }
}

#[test]
fn komut_satirlari() {
    let veri = Path::new("/veri");
    assert_eq!(yazdir_argumanlari(veri), ["--kok", "/veri", "calis"]);
    let varsayilan = DugumAyari {
        port: DUGUM_PORT,
        ..DugumAyari::default()
    };
    assert_eq!(
        dugum_argumanlari(veri, &varsayilan),
        ["calis", "--kok", "/veri"]
    );
    let tam = dugum_ayari(
        9000,
        Some("https://merkez.ornek.com"),
        Some("http://192.168.1.20:9000"),
        Some(Ipv4Addr::new(192, 168, 1, 20)),
        false,
    );
    // Yol listesi yoksa --merkez verilir.
    assert_eq!(
        dugum_argumanlari(veri, &tam),
        [
            "calis",
            "--kok",
            "/veri",
            "--port",
            "9000",
            "--lan",
            "192.168.1.20",
            "--koken",
            "https://merkez.ornek.com",
            "--merkez",
            "https://merkez.ornek.com"
        ]
    );
    // Şube adresi adla yazıldıysa --ad o ad; yol listesi varsa --merkez yazılmaz.
    let adli = dugum_ayari(
        DUGUM_PORT,
        Some("https://merkez.ornek.com"),
        Some("http://kasa-pc:8788"),
        Some(Ipv4Addr::new(192, 168, 1, 77)),
        true,
    );
    assert_eq!(
        dugum_argumanlari(veri, &adli),
        [
            "calis",
            "--kok",
            "/veri",
            "--lan",
            "192.168.1.77",
            "--ad",
            "kasa-pc",
            "--koken",
            "https://merkez.ornek.com"
        ]
    );
    // Dinlenecek adres: şube adresindeki IP bu bilgisayardaysa o; değilse o anki yerel ağ adresi; hiçbiri yoksa yok.
    let mut s = Sahte::yeni(false);
    s.yerel = Some(Ipv4Addr::new(192, 168, 1, 77));
    assert_eq!(
        lan_sec(Some("http://192.168.1.20:8788"), &s),
        Some(Ipv4Addr::new(192, 168, 1, 20))
    );
    assert_eq!(
        lan_sec(Some("http://192.168.1.99:8788"), &s),
        Some(Ipv4Addr::new(192, 168, 1, 77)),
        "adres değişmişse düğüm yine açılır"
    );
    assert_eq!(lan_sec(Some("http://kasa-pc:8788"), &s), s.yerel);
    assert_eq!(lan_sec(None, &Sahte::yeni(false)), None);
    // Genel IP şube adresi sayılmaz.
    assert_eq!(sube_ip(Some("http://8.8.8.8:8788")), None);
    let komut = gorev_komutu(Path::new("/kok"));
    assert!(komut.starts_with('"') && komut.ends_with("\" --sube-pc-baslat"));
    assert!(komut.contains("taa-kur-pencere.exe"));
    let g = schtasks_kur(&komut);
    assert_eq!(&g[..4], ["/Create", "/TN", GOREV_ADI, "/SC"]);
    assert!(g.contains(&"ONLOGON".to_string()) && g.contains(&"LIMITED".to_string()));
    assert_eq!(g[g.len() - 2], komut);
    let r = run_kaydi_kur(&komut);
    assert!(r[1].starts_with("HKCU\\") && r.contains(&komut));
    let d = guvenlik_duvari_kur(Path::new("/kok/taa-sube-dugum.exe"), 8788);
    for beklenen in [
        "dir=in",
        "action=allow",
        "protocol=TCP",
        "localport=8788",
        "profile=private",
        "remoteip=localsubnet",
        "program=/kok/taa-sube-dugum.exe",
    ] {
        assert!(d.iter().any(|x| x == beklenen), "{beklenen} eksik: {d:?}");
    }
}

#[test]
fn kur_dosyalari_yazar_gorevi_kaydeder_hizmetleri_baslatir() {
    let (kur, kok, veri) = ortam(&[YAZDIR, DUGUM]);
    let s = Sahte::yeni(true);
    let o = kur
        .sube_pc_kur_veri_ile(Some(veri.clone()), &secenek(), &s)
        .unwrap();

    // Günlük klasörü ve yol listesi.
    assert!(veri.join("gunluk").is_dir());
    let yollar = json_oku(&veri.join("sube-pc").join(YOL_DOSYASI));
    assert_eq!(yollar[0]["ad"], "merkez");
    assert_eq!(
        yollar[1],
        json!({"ad": "sube", "url": "http://192.168.1.20:8788"})
    );
    assert_eq!(yollar[2], json!({"ad": "yerel"}));
    assert_eq!(o["yollar"], yollar);

    // taa-yazdir ayarı: iki köken izinli, yazıcı hedefi boş bırakıldı.
    let ayar = json_oku(&veri.join("yazdir").join("ayar.json"));
    assert_eq!(
        ayar["izinli_kokenler"],
        json!(["https://merkez.ornek.com", "http://192.168.1.20:8788"])
    );
    assert_eq!(ayar["port"], YAZDIR_PORT);
    assert_eq!(ayar["yazicilar"], json!({}));

    // Profil kurulum kökünde.
    let profil = json_oku(&kok.join(PROFIL_DOSYASI));
    assert_eq!(profil["veri_kok"], veri.display().to_string());
    assert_eq!(profil["dugum_port"], DUGUM_PORT);

    // Güvenlik duvarı + açılış görevi (schtasks başarılı: reg'e gerek kalmaz).
    assert_eq!(s.programlar(), ["netsh", "schtasks"]);
    assert_eq!(o["guvenlik_duvari"]["durum"], "kuruldu");
    assert_eq!(
        o["gorev"],
        json!({"durum": "kuruldu", "tur": "gorev", "komut": gorev_komutu(&kok)})
    );

    // İki hizmet başlatıldı; çıktıları günlük klasörüne değil sube-pc/cikti altına gider.
    let b = s.baslatilan.borrow();
    assert_eq!(b.len(), 2);
    assert_eq!(b[0].0, kok.join(exe(YAZDIR)));
    assert_eq!(b[0].1, yazdir_argumanlari(&veri));
    assert_eq!(b[1].0, kok.join(exe(DUGUM)));
    assert_eq!(
        b[1].1,
        [
            "calis",
            "--kok",
            &veri.display().to_string(),
            "--lan",
            "192.168.1.20",
            "--koken",
            "https://merkez.ornek.com",
            "--merkez",
            "https://merkez.ornek.com"
        ]
    );
    assert!(b[1].2.starts_with(veri.join("sube-pc").join("cikti")));
    assert_eq!(o["hizmetler"][1]["durum"], "baslatildi");
    assert_eq!(o["eksik"], json!([]));

    // Kurulum izi eklendi.
    let iz = std::fs::read_to_string(veri.join("sube-pc").join("kurulum.jsonl")).unwrap();
    assert_eq!(iz.lines().count(), 1);
    assert!(iz.contains("sube-pc.kur"));
}

#[test]
fn schtasks_reddedilirse_run_kaydina_duser_duvar_adim_olarak_kalir() {
    let (kur, _kok, veri) = ortam(&[YAZDIR, DUGUM]);
    let mut s = Sahte::yeni(true);
    s.reddeden = vec!["schtasks", "netsh"];
    let o = kur
        .sube_pc_kur_veri_ile(Some(veri), &secenek(), &s)
        .unwrap();
    assert_eq!(s.programlar(), ["netsh", "schtasks", "reg"]);
    assert_eq!(o["gorev"]["tur"], "run-kaydi");
    assert_eq!(o["gorev"]["durum"], "kuruldu");
    assert_eq!(o["guvenlik_duvari"]["durum"], "yapilamadi");
    assert!(o["guvenlik_duvari"]["oneri"]
        .as_str()
        .unwrap()
        .contains("netsh advfirewall"));
    // Hiçbiri olmazsa dürüstçe "yapılamadı".
    let (kur, _kok, veri) = ortam(&[DUGUM]);
    let mut s = Sahte::yeni(true);
    s.reddeden = vec!["schtasks", "reg"];
    let o = kur
        .sube_pc_kur_veri_ile(Some(veri), &secenek(), &s)
        .unwrap();
    assert_eq!(o["gorev"]["durum"], "yapilamadi");
}

#[test]
fn ikinci_kurulum_var_olani_ezmez_eskiyi_yana_tasir() {
    let (kur, _kok, veri) = ortam(&[YAZDIR, DUGUM]);
    // Kullanıcının elle düzenlediği taa-yazdir ayarı ve düğümün yazdığı günlük.
    std::fs::create_dir_all(veri.join("yazdir")).unwrap();
    std::fs::write(
        veri.join("yazdir").join("ayar.json"),
        r#"{"port": 9911, "izinli_kokenler": ["https://merkez.ornek.com/"], "yazicilar": {"escpos": "tcp://192.168.1.50:9100"}}"#,
    )
    .unwrap();
    std::fs::create_dir_all(veri.join("gunluk")).unwrap();
    std::fs::write(veri.join("gunluk").join("2026-11-26.jsonl"), "{\"x\":1}\n").unwrap();

    let s = Sahte::yeni(false);
    let o = kur
        .sube_pc_kur_veri_ile(Some(veri.clone()), &secenek(), &s)
        .unwrap();
    // Var olan köken yinelenmedi; yalnız şube eklendi; yazıcı ve kapı korundu.
    assert_eq!(o["eklenen_kokenler"], json!(["http://192.168.1.20:8788"]));
    let ayar = json_oku(&veri.join("yazdir").join("ayar.json"));
    assert_eq!(ayar["port"], 9911);
    assert_eq!(ayar["yazicilar"]["escpos"], "tcp://192.168.1.50:9100");
    assert_eq!(ayar["izinli_kokenler"].as_array().unwrap().len(), 2);
    // Değişen ayarın eski hali silinmedi.
    let eskiler: Vec<_> = std::fs::read_dir(veri.join("yazdir").join("eski"))
        .unwrap()
        .collect();
    assert_eq!(eskiler.len(), 1);
    // Köprünün ayardaki kapısı kullanıldı.
    assert_eq!(o["hizmetler"][0]["port"], 9911);
    // Günlük dosyasına dokunulmadı.
    assert_eq!(
        std::fs::read_to_string(veri.join("gunluk").join("2026-11-26.jsonl")).unwrap(),
        "{\"x\":1}\n"
    );
    // Windows değil: görev ve duvar atlandı, hiçbir sistem komutu çalışmadı.
    assert!(s.programlar().is_empty());
    assert_eq!(o["gorev"]["durum"], "atlandi");
    assert_eq!(o["guvenlik_duvari"]["durum"], "atlandi");

    // Aynı seçenekle yeniden: hiçbir dosya değişmez, yeni "eski" kopya oluşmaz.
    let once = std::fs::read_to_string(veri.join("sube-pc").join(YOL_DOSYASI)).unwrap();
    let o2 = kur
        .sube_pc_kur_veri_ile(Some(veri.clone()), &secenek(), &s)
        .unwrap();
    assert_eq!(o2["eklenen_kokenler"], json!([]));
    assert!(o2["eski_yol_dosyasi"].is_null());
    assert_eq!(
        std::fs::read_to_string(veri.join("sube-pc").join(YOL_DOSYASI)).unwrap(),
        once
    );
    assert_eq!(
        std::fs::read_dir(veri.join("yazdir").join("eski"))
            .unwrap()
            .count(),
        1
    );
    assert!(!veri.join("sube-pc").join("eski").exists());

    // Şube adresi değişirse eski yol listesi eski\ altına taşınır.
    let mut yeni = secenek();
    yeni.sube_adresi = Some("http://192.168.1.99:8788".into());
    let o3 = kur
        .sube_pc_kur_veri_ile(Some(veri.clone()), &yeni, &s)
        .unwrap();
    assert!(
        o3["uyarilar"]
            .as_array()
            .unwrap()
            .iter()
            .any(|u| u.as_str().unwrap().contains("bu bilgisayarın adresi değil")),
        "bu bilgisayarda olmayan şube adresi uyarılır"
    );
    let eski = PathBuf::from(o3["eski_yol_dosyasi"].as_str().unwrap());
    assert!(eski.is_file() && eski.starts_with(veri.join("sube-pc").join("eski")));
    assert_eq!(std::fs::read_to_string(&eski).unwrap(), once);
    // Kurulum izi her çalıştırmada bir satır büyür, eskisi durur.
    let iz = std::fs::read_to_string(veri.join("sube-pc").join("kurulum.jsonl")).unwrap();
    assert_eq!(iz.lines().count(), 3);
}

#[test]
fn calisan_hizmet_yeniden_baslatilmaz_ve_baslatma_secenegi() {
    let (kur, _kok, veri) = ortam(&[YAZDIR, DUGUM]);
    let mut s = Sahte::yeni(false);
    s.acik_portlar = vec![DUGUM_PORT];
    let o = kur
        .sube_pc_kur_veri_ile(Some(veri.clone()), &secenek(), &s)
        .unwrap();
    assert_eq!(o["hizmetler"][0]["durum"], "baslatildi");
    assert_eq!(o["hizmetler"][1]["durum"], "zaten_calisiyor");
    assert_eq!(s.baslatilan.borrow().len(), 1);

    let s = Sahte::yeni(false);
    let mut sec = secenek();
    sec.baslatma = true;
    let o = kur.sube_pc_kur_veri_ile(Some(veri), &sec, &s).unwrap();
    assert_eq!(o["hizmetler"], json!([]));
    assert!(s.baslatilan.borrow().is_empty());
}

#[test]
fn eksik_ikili_ve_adres_uyarilari() {
    // Yalnız yazdırma köprüsü var: düğüm eksik diye bildirilir, duvar atlanır, kurulum yine tamamlanır.
    let (kur, _kok, veri) = ortam(&[YAZDIR]);
    let s = Sahte::yeni(true);
    let o = kur
        .sube_pc_kur_veri_ile(Some(veri), &secenek(), &s)
        .unwrap();
    assert_eq!(o["eksik"], json!([DUGUM]));
    assert_eq!(o["guvenlik_duvari"]["durum"], "atlandi");
    assert_eq!(s.baslatilan.borrow().len(), 1);
    assert!(o["uyarilar"]
        .as_array()
        .unwrap()
        .iter()
        .any(|u| u.as_str().unwrap().contains(DUGUM)));

    // İkisi de yoksa: hata (çıkış 4), hiçbir şey yazılmaz.
    let (kur, kok, veri) = ortam(&[]);
    let e = kur
        .sube_pc_kur_veri_ile(Some(veri.clone()), &secenek(), &Sahte::yeni(true))
        .unwrap_err();
    assert!(matches!(e, KurHata::Yok(_)), "{e:?}");
    assert!(!veri.join("sube-pc").exists() && !kok.join(PROFIL_DOSYASI).exists());

    // Veri kökü yoksa ve geçersiz adreste hata.
    let (kur, _kok, veri) = ortam(&[DUGUM]);
    assert!(matches!(
        kur.sube_pc_kur_veri_ile(None, &secenek(), &Sahte::yeni(false)),
        Err(KurHata::Cozum(_))
    ));
    let mut kotu = secenek();
    kotu.merkez = Some("http://merkez.ornek.com".into());
    assert!(matches!(
        kur.sube_pc_kur_veri_ile(Some(veri.clone()), &kotu, &Sahte::yeni(false)),
        Err(KurHata::Dogrulama(_))
    ));
    let mut kotu = secenek();
    kotu.port = Some(80);
    assert!(matches!(
        kur.sube_pc_kur_veri_ile(Some(veri), &kotu, &Sahte::yeni(false)),
        Err(KurHata::Dogrulama(_))
    ));
}

#[test]
fn merkez_adresi_ulasim_yolu_listesinden_alinir() {
    let (kur, _kok, veri) = ortam(&[DUGUM]);
    let mut y = taa_yol::Yollar::yeni("merkez").unwrap();
    y.ekle(taa_yol::Yol::yeni("dogrudan", "https://85.1.2.3:8443", None).unwrap())
        .unwrap();
    taa_yol::yollar_yaz(&veri, &y).unwrap();
    assert_eq!(
        merkez_adresi(&veri).as_deref(),
        Some("https://85.1.2.3:8443")
    );
    let sec = Secenek {
        sube_adresi: Some("http://192.168.1.20:8788".into()),
        ..Secenek::default()
    };
    let o = kur
        .sube_pc_kur_veri_ile(Some(veri), &sec, &Sahte::yeni(false))
        .unwrap();
    assert_eq!(
        o["yollar"][0],
        json!({"ad": "merkez", "url": "https://85.1.2.3:8443"})
    );

    // Yol listesi de yoksa merkez yazılmaz ve uyarı verilir.
    let (kur, _kok, veri) = ortam(&[DUGUM]);
    let o = kur
        .sube_pc_kur_veri_ile(Some(veri), &sec, &Sahte::yeni(false))
        .unwrap();
    assert_eq!(o["yollar"][0]["ad"], "sube");
    assert!(o["uyarilar"][0].as_str().unwrap().contains("Merkez adresi"));
}

#[test]
fn sunucu_rolunde_kurulmaz() {
    let (kur, kok, veri) = ortam(&[YAZDIR, DUGUM]);
    std::fs::write(kok.join("rol.txt"), "sunucu\n").unwrap();
    let e = kur
        .sube_pc_kur_veri_ile(Some(veri.clone()), &secenek(), &Sahte::yeni(true))
        .unwrap_err();
    assert!(matches!(e, KurHata::Dogrulama(_)));
    assert!(!veri.join("gunluk").exists());
    // Şube rolünde kurulur.
    std::fs::write(kok.join("rol.txt"), "sube\n").unwrap();
    assert!(kur
        .sube_pc_kur_veri_ile(Some(veri), &secenek(), &Sahte::yeni(false))
        .is_ok());
}

#[test]
fn baslat_durum_ve_gorev_kaldir() {
    let (kur, kok, veri) = ortam(&[YAZDIR, DUGUM]);
    // Profil yokken: anlaşılır hata.
    assert!(matches!(
        kur.sube_pc_baslat(&Sahte::yeni(true)),
        Err(KurHata::Yok(_))
    ));
    let mut sec = secenek();
    sec.baslatma = true;
    sec.port = Some(9000);
    kur.sube_pc_kur_veri_ile(Some(veri.clone()), &sec, &Sahte::yeni(false))
        .unwrap();

    // Açılış: profildeki veri kökü ve kapıyla başlatır.
    let s = Sahte::yeni(true);
    let o = kur.sube_pc_baslat(&s).unwrap();
    assert_eq!(o["hizmetler"].as_array().unwrap().len(), 2);
    let a = s.baslatilan.borrow()[1].1.clone();
    assert_eq!(
        &a[..5],
        [
            "calis",
            "--kok",
            &veri.display().to_string(),
            "--port",
            "9000"
        ]
    );
    assert!(a.contains(&"--lan".to_string()) && a.contains(&"192.168.1.20".to_string()));

    // Durum: hiçbir şey değiştirmez.
    let mut s = Sahte::yeni(true);
    s.acik_portlar = vec![9000];
    s.reddeden = vec!["schtasks"];
    let d = kur.sube_pc_durum(&s).unwrap();
    assert_eq!(d["acilis_kaydi"], "run-kaydi");
    assert_eq!(d["hizmetler"][0]["calisiyor"], false);
    assert_eq!(d["hizmetler"][1]["calisiyor"], true);
    assert_eq!(d["yollar"][2]["ad"], "yerel");
    assert_eq!(d["gunluk_var"], true);
    assert!(s.baslatilan.borrow().is_empty());

    // Görev kaldırma yalnız kaydı kaldırır: dosyalar ve veriler yerinde.
    let s = Sahte::yeni(true);
    let k = kur.sube_pc_gorev_kaldir(&s).unwrap();
    assert_eq!(
        k,
        json!({"gorev_kaldirildi": true, "run_kaydi_kaldirildi": true})
    );
    assert_eq!(s.programlar(), ["schtasks", "reg"]);
    assert!(kok.join(PROFIL_DOSYASI).is_file());
    assert!(veri.join("sube-pc").join(YOL_DOSYASI).is_file());
    assert!(kok.join(exe(DUGUM)).is_file());
    assert!(kur.sube_pc_gorev_kaldir(&Sahte::yeni(false)).is_err());
}

#[test]
fn ikili_depodaki_secili_surumden_de_bulunur() {
    let (kur, kok, _veri) = ortam(&[]);
    assert!(kur.sube_pc_ikili(DUGUM).is_none());
    std::fs::write(kok.join(exe(DUGUM)), b"").unwrap();
    assert_eq!(kur.sube_pc_ikili(DUGUM), Some(kok.join(exe(DUGUM))));
}

#[test]
fn bozuk_yazdir_ayarina_dokunulmaz() {
    let (kur, _kok, veri) = ortam(&[YAZDIR]);
    std::fs::create_dir_all(veri.join("yazdir")).unwrap();
    std::fs::write(veri.join("yazdir").join("ayar.json"), "{bozuk").unwrap();
    let e = kur
        .sube_pc_kur_veri_ile(Some(veri.clone()), &secenek(), &Sahte::yeni(false))
        .unwrap_err();
    assert!(matches!(e, KurHata::Bozuk(_)));
    assert_eq!(
        std::fs::read_to_string(veri.join("yazdir").join("ayar.json")).unwrap(),
        "{bozuk"
    );
}

#[test]
fn sube_adresi_verilmezse_yerel_ag_adresinden_yazilir() {
    let (kur, _kok, veri) = ortam(&[DUGUM]);
    let mut s = Sahte::yeni(false);
    s.yerel = Some(Ipv4Addr::new(10, 0, 0, 7));
    s.adresler = vec![Ipv4Addr::new(10, 0, 0, 7)];
    let sec = Secenek {
        merkez: Some("https://merkez.ornek.com".into()),
        port: Some(9000),
        lan_ac: true,
        ..Secenek::default()
    };
    let o = kur
        .sube_pc_kur_veri_ile(Some(veri.clone()), &sec, &s)
        .unwrap();
    assert_eq!(
        o["yollar"][1],
        json!({"ad": "sube", "url": "http://10.0.0.7:9000"})
    );
    let a = s.baslatilan.borrow()[0].1.clone();
    assert!(a.windows(2).any(|w| w == ["--lan", "10.0.0.7"]), "{a:?}");
    assert!(o["uyarilar"]
        .as_array()
        .unwrap()
        .iter()
        .any(|u| u.as_str().unwrap().contains("sabit IP")));

    // Yerel ağ adresi yoksa: şube yola yazılmaz, uyarı verilir, düğüm --lan'sız başlar.
    let (kur, _kok, veri) = ortam(&[DUGUM]);
    let s = Sahte::yeni(false);
    let o = kur.sube_pc_kur_veri_ile(Some(veri), &sec, &s).unwrap();
    assert_eq!(
        o["yollar"],
        json!([{"ad": "merkez", "url": "https://merkez.ornek.com"}, {"ad": "yerel"}])
    );
    assert!(!s.baslatilan.borrow()[0].1.contains(&"--lan".to_string()));
    assert!(o["uyarilar"]
        .as_array()
        .unwrap()
        .iter()
        .any(|u| u.as_str().unwrap().contains("yerel ağ adresi bulunamadı")));
}

#[test]
fn varsayilan_yerel_aga_kapali() {
    // --lan-ac verilmezse: düğüm --lan'sız başlar, şube yolu 127.0.0.1'dir, güvenlik duvarı kuralı kurulmaz.
    let (kur, kok, veri) = ortam(&[YAZDIR, DUGUM]);
    let mut s = Sahte::yeni(true);
    s.yerel = Some(Ipv4Addr::new(192, 168, 1, 20));
    let sec = Secenek {
        merkez: Some("https://merkez.ornek.com".into()),
        ..Secenek::default()
    };
    let o = kur
        .sube_pc_kur_veri_ile(Some(veri.clone()), &sec, &s)
        .unwrap();
    assert_eq!(
        o["yollar"][1],
        json!({"ad": "sube", "url": "http://127.0.0.1:8788"})
    );
    assert_eq!(o["guvenlik_duvari"]["durum"], "atlandi");
    assert_eq!(s.programlar(), ["schtasks"], "netsh çağrılmadı");
    let a = s.baslatilan.borrow()[1].1.clone();
    assert!(!a.contains(&"--lan".to_string()), "{a:?}");
    assert!(a
        .windows(2)
        .any(|w| w == ["--koken", "https://merkez.ornek.com"]));
    assert_eq!(json_oku(&kok.join(PROFIL_DOSYASI))["lan_acik"], false);
    assert!(o["uyarilar"]
        .as_array()
        .unwrap()
        .iter()
        .any(|u| u.as_str().unwrap().contains("--lan-ac")));
    // Açılışta da kapalı kalır.
    let s2 = Sahte::yeni(true);
    kur.sube_pc_baslat(&s2).unwrap();
    assert!(!s2.baslatilan.borrow()[1].1.contains(&"--lan".to_string()));

    // --lan-ac ile: uyarı "doğrulama yok" der, kural kurulur, --lan verilir.
    let mut acik = sec.clone();
    acik.lan_ac = true;
    let s3 = Sahte {
        yerel: Some(Ipv4Addr::new(192, 168, 1, 20)),
        ..Sahte::yeni(true)
    };
    let o = kur.sube_pc_kur_veri_ile(Some(veri), &acik, &s3).unwrap();
    assert_eq!(o["yollar"][1]["url"], "http://192.168.1.20:8788");
    assert_eq!(o["guvenlik_duvari"]["durum"], "kuruldu");
    assert!(s3.baslatilan.borrow()[1]
        .1
        .windows(2)
        .any(|w| w == ["--lan", "192.168.1.20"]));
    assert!(o["uyarilar"]
        .as_array()
        .unwrap()
        .iter()
        .any(|u| u.as_str().unwrap().contains("cihaz doğrulaması yok")));
}
