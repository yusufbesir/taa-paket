// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-sube-dugum 2026-11-27]
//! taa-sube-dugum: şube bilgisayarında çalışan yerel düğüm (bkz. docs/SUBE-DUGUMU.md "Uygulama").

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;
use taa_sube_dugum::{merkez, sunucu, DenetimYok, Dugum, PORT_VARSAYILAN};

const YARDIM: &str = "taa-sube-dugum: şube bilgisayarı düğümü. Telefonlar ve tarayıcılar merkeze ulaşamayınca buraya yazar;
düğüm her kaydı aynı no ile merkeze gönderir. Merkezin yerini tutmaz.

Kullanım:
  taa-sube-dugum calis [seçenekler]     Düğümü başlatır (durdurulana kadar çalışır).
  taa-sube-dugum durum [--kok ..] [--sube ..]   Kuyruk durumunu yazar (ağa çıkmaz).

Seçenekler:
  --kok <klasör>        Veri kökü. Verilmezse TAA_VERI_KOK, yoksa %APPDATA%\\TAA\\veri.
  --sube <kod>          Şube kodu. Verilmezse <kök>\\sube.json içindeki kod.
  --port <kapı>         Varsayılan 8788.
  --lan <IPv4>          Şube bilgisayarının YEREL AĞ adresi (örnek 192.168.1.20). Verilmezse düğüm yalnız
                        127.0.0.1'de dinler ve telefonlar ulaşamaz. 0.0.0.0, genel adres ve Tailscale reddedilir.
  --ad <ad>             Düğüme ulaşılan ek ad (örnek kasa-pc ya da mrk01.sube.ornek.com); yinelenebilir.
  --koken <köken>       Düğüme yazmasına izin verilen başka köken (örnek https://sunucu.ornek.com); yinelenebilir.
  --merkez <adres>      Merkez sunucu (https://...). Verilmezse TAA_SUNUCU, yoksa taa-yol yol listesi.
  --sertifika <CA.pem>  Merkez için kendi sertifikamız.
  --oturum-dosyasi <yol> Merkez oturum anahtarı (tek satır). Verilmezse TAA_OTURUM. Komut satırına yazılmaz.
  --aralik <saniye>     Merkeze gönderim denemeleri arası süre (varsayılan 30).

Uyarı: bu sürümde cihaz/kişi doğrulaması yoktur (kanca boş) ve bağlantı düz HTTP'dir. --lan yalnız güvenilen şube ağında açılmalıdır.
Hiçbir satır silinmez: kayıtlar <kök>\\kuyruk\\sube-dugum\\, günlük <kök>\\gunluk\\YYYY-MM-DD.jsonl altındadır.";

fn veri_kok_ortamdan() -> Option<PathBuf> {
    let ev = |a: &str| std::env::var(a).ok().filter(|s| !s.trim().is_empty());
    if let Some(k) = ev("TAA_VERI_KOK") {
        return Some(PathBuf::from(k));
    }
    if cfg!(windows) {
        return ev("APPDATA").map(|a| PathBuf::from(a).join("TAA").join("veri"));
    }
    if let Some(x) = ev("XDG_DATA_HOME") {
        return Some(PathBuf::from(x).join("TAA").join("veri"));
    }
    ev("HOME").map(|h| PathBuf::from(h).join(".local/share/TAA/veri"))
}

#[derive(Default)]
struct Secenek {
    komut: Option<String>,
    kok: Option<PathBuf>,
    sube: Option<String>,
    port: Option<u16>,
    lan: Option<Ipv4Addr>,
    adlar: Vec<String>,
    kokenler: Vec<String>,
    merkez: Option<String>,
    sertifika: Option<PathBuf>,
    oturum_dosyasi: Option<PathBuf>,
    aralik: Option<u64>,
}

fn coz(args: &[String]) -> Result<Secenek, String> {
    let mut s = Secenek::default();
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        if !a.starts_with("--") {
            if s.komut.is_some() {
                return Err(format!("beklenmeyen değer: {a}"));
            }
            s.komut = Some(a.to_string());
            i += 1;
            continue;
        }
        let deger = args
            .get(i + 1)
            .ok_or_else(|| format!("{a} için değer gerekli"))?
            .clone();
        match a {
            "--kok" => s.kok = Some(PathBuf::from(deger)),
            "--sube" => s.sube = Some(deger),
            "--port" => {
                s.port = Some(
                    deger
                        .parse()
                        .ok()
                        .filter(|p| *p != 0)
                        .ok_or_else(|| format!("geçersiz kapı: {deger}"))?,
                )
            }
            "--lan" => {
                let ip: Ipv4Addr = deger
                    .parse()
                    .map_err(|_| format!("--lan IPv4 adresi olmalı: {deger}"))?;
                sunucu::lan_adresi_gecerli(ip)?;
                s.lan = Some(ip);
            }
            "--ad" => s.adlar.push(deger),
            "--koken" => s.kokenler.push(deger),
            "--merkez" => s.merkez = Some(deger),
            "--sertifika" => s.sertifika = Some(PathBuf::from(deger)),
            "--oturum-dosyasi" => s.oturum_dosyasi = Some(PathBuf::from(deger)),
            "--aralik" => {
                s.aralik = Some(
                    deger
                        .parse()
                        .ok()
                        .filter(|n| *n >= 1)
                        .ok_or_else(|| format!("geçersiz süre: {deger}"))?,
                )
            }
            _ => return Err(format!("bilinmeyen seçenek: {a}")),
        }
        i += 2;
    }
    Ok(s)
}

fn calistir(s: Secenek) -> Result<(), (u8, String)> {
    let kok = s
        .kok
        .clone()
        .or_else(veri_kok_ortamdan)
        .ok_or_else(|| (2, "veri kökü bulunamadı. Öneri: --kok verin.".to_string()))?;
    let sube = s
        .sube
        .clone()
        .or_else(|| taa_sube_dugum::sube_oku(&kok))
        .ok_or_else(|| {
            (
                2,
                "şube kodu yok. Öneri: --sube verin ya da TAA Kur ile şubeyi kurun (sube.json)."
                    .to_string(),
            )
        })?;
    let d = Arc::new(Dugum::ac(&kok, &sube).map_err(|e| (1, e.to_string()))?);
    match s.komut.as_deref() {
        Some("durum") => {
            let o = d.durum().map_err(|e| (1, e.to_string()))?;
            println!(
                "{}",
                serde_json::json!({
                    "sube": sube, "bekleyen": o.bekleyen, "gonderilen": o.gonderilen,
                    "reddedilen": o.reddedilen, "son_merkez_zamani": o.son_merkez_zamani,
                    "cakisma": o.cakisma, "surum": o.surum, "gunluk_bekleyen": o.gunluk_bekleyen,
                })
            );
            return Ok(());
        }
        Some("calis") => {}
        Some(k) => {
            return Err((
                2,
                format!("bilinmeyen komut: {k}. Öneri: taa-sube-dugum --yardim"),
            ))
        }
        None => return Err((2, "komut gerekli: calis ya da durum".to_string())),
    }

    let port = s.port.unwrap_or(PORT_VARSAYILAN);
    let mut ag = sunucu::AgAyari::yerel();
    let mut adresler = vec![IpAddr::V4(Ipv4Addr::LOCALHOST)];
    if let Some(ip) = s.lan {
        ag.ad_ekle(&ip.to_string());
        adresler.push(IpAddr::V4(ip));
    }
    for a in &s.adlar {
        ag.ad_ekle(a);
    }
    for k in &s.kokenler {
        if !ag.koken_ekle(k) {
            return Err((
                2,
                format!("geçersiz köken (örnek https://sunucu.ornek.com): {k}"),
            ));
        }
    }
    let ag = Arc::new(ag);
    let dur = Arc::new(AtomicBool::new(false));
    let mut isler = Vec::new();
    for ip in adresler {
        let adres = SocketAddr::new(ip, port);
        let sn = Arc::new(sunucu::dinle(adres).map_err(|e| {
            (
                1,
                format!(
                    "{adres} dinlenemedi: {e}. Öneri: adresi ve kapının boş olduğunu denetleyin."
                ),
            )
        })?);
        println!("taa-sube-dugum: http://{adres} dinleniyor (şube {sube})");
        for _ in 0..4 {
            let (sn, d, ag, dur) = (sn.clone(), d.clone(), ag.clone(), dur.clone());
            isler.push(std::thread::spawn(move || {
                sunucu::hizmet(&sn, &d, &ag, &DenetimYok, &dur)
            }));
        }
    }
    if s.lan.is_none() {
        println!("taa-sube-dugum: yalnız bu bilgisayardan ulaşılır. Telefonlar için --lan <yerel ağ adresi> verin.");
    } else {
        println!("taa-sube-dugum: UYARI: cihaz doğrulaması yok ve bağlantı düz HTTP; yalnız güvenilen şube ağında kullanın.");
    }
    d.gunluk(
        "dugum.basladi",
        serde_json::json!({"sube": sube, "port": port, "lan": s.lan.map(|i| i.to_string())}),
    );

    let merkez_adresi = s
        .merkez
        .clone()
        .or_else(|| std::env::var("TAA_SUNUCU").ok());
    let (sertifika, oturum_dosyasi) = (s.sertifika.clone(), s.oturum_dosyasi.clone());
    let kur = move || {
        // Oturum her turda yeniden okunur (yenilenmiş olabilir); hiçbir yere yazılmaz.
        let oturum = match &oturum_dosyasi {
            Some(y) => std::fs::read_to_string(y).ok(),
            None => std::env::var("TAA_OTURUM").ok(),
        };
        // Cihaz imza kancası boş (None): taa-cihaz bağlanınca buraya CihazAnahtari verilir.
        match merkez::gonderici_kur(
            &kok,
            &sube,
            merkez_adresi.as_deref(),
            sertifika.as_deref(),
            oturum.as_deref(),
            None,
        ) {
            Ok(g) => g,
            Err(e) => {
                eprintln!("taa-sube-dugum: merkez göndericisi kurulamadı: {e}");
                None
            }
        }
    };
    merkez::dongu(
        &d,
        &kur,
        Duration::from_secs(s.aralik.unwrap_or(merkez::ARALIK_SN_VARSAYILAN)),
        &dur,
    );
    for i in isler {
        let _ = i.join();
    }
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty()
        || args
            .iter()
            .any(|a| matches!(a.as_str(), "yardim" | "--yardim" | "--help" | "-h" | "help"))
    {
        println!("{YARDIM}");
        return ExitCode::from(if args.is_empty() { 2 } else { 0 });
    }
    let s = match coz(&args) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("taa-sube-dugum: {e}");
            return ExitCode::from(2);
        }
    };
    match calistir(s) {
        Ok(()) => ExitCode::SUCCESS,
        Err((kod, m)) => {
            eprintln!("taa-sube-dugum: {m}");
            ExitCode::from(kod)
        }
    }
}
