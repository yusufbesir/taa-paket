//! taa-yazdir komutu: yazdırma köprüsünü 127.0.0.1'de çalıştırır. Sahip: Yusuf.
//! yazan: [Claude Fable 5.2 #taa-web-W2-yazdir 2026-11-27]
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use taa_yazdir::{Ayar, CihazImzasi, DenetimKapali, GercekYazici, Hizmet};

const KULLANIM: &str = "taa-yazdir: tarayıcıdaki TAA Web için yazdırma köprüsü (yalnız 127.0.0.1)
Kullanım:
  taa-yazdir [--kok <veri kökü>] [--ayar <dosya>] calis      Hizmeti başlatır (varsayılan komut)
  taa-yazdir [--kok <veri kökü>] [--ayar <dosya>] denetle    Ayar dosyasını denetler, özet yazar
  taa-yazdir ornek-ayar                                      Örnek ayar dosyasını ekrana yazar
  --kok    Veri kökü. Verilmezse TAA_VERI_KOK, yoksa %APPDATA%\\TAA\\veri.
  --ayar   Ayar dosyası. Verilmezse <veri kökü>\\yazdir\\ayar.json.
Uçlar: GET /durum, POST /yazdir. Dinleme adresi her zaman 127.0.0.1'dir, değiştirilemez.
Çıkış: 0 tamam, 2 kullanım/ayar hatası, 1 diğer.";

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

fn main() -> ExitCode {
    let mut kok: Option<PathBuf> = None;
    let mut ayar_yolu: Option<PathBuf> = None;
    let mut komut = String::from("calis");
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "-h" | "--help" | "yardim" => {
                println!("{KULLANIM}");
                return ExitCode::SUCCESS;
            }
            "--kok" | "--ayar" => {
                let Some(d) = args.next() else {
                    eprintln!("{a} bir değer ister.\n{KULLANIM}");
                    return ExitCode::from(2);
                };
                if a == "--kok" {
                    kok = Some(PathBuf::from(d));
                } else {
                    ayar_yolu = Some(PathBuf::from(d));
                }
            }
            "calis" | "denetle" | "ornek-ayar" => komut = a,
            _ => {
                eprintln!("Bilinmeyen seçenek: {a}\n{KULLANIM}");
                return ExitCode::from(2);
            }
        }
    }
    if komut == "ornek-ayar" {
        println!("{}", taa_yazdir::ornek_ayar());
        return ExitCode::SUCCESS;
    }
    let kok = kok.or_else(veri_kok_ortamdan);
    let Some(ayar_yolu) =
        ayar_yolu.or_else(|| kok.as_ref().map(|k| k.join("yazdir").join("ayar.json")))
    else {
        eprintln!("Veri kökü bulunamadı: --kok ya da --ayar verin.");
        return ExitCode::from(2);
    };
    let ayar = match Ayar::dosyadan(&ayar_yolu) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("{e}\nÖneri: 'taa-yazdir ornek-ayar' çıktısını bu dosyaya yazıp düzenleyin.");
            return ExitCode::from(2);
        }
    };
    let denetim: Box<dyn taa_yazdir::CihazDenetimi> = if ayar.cihaz_imza_zorunlu {
        let Some(k) = kok.as_ref() else {
            eprintln!("cihaz_imza 'zorunlu' iken veri kökü gerekir (--kok).");
            return ExitCode::from(2);
        };
        match CihazImzasi::kokten(k) {
            Ok(c) => Box::new(c),
            Err(e) => {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
        }
    } else {
        Box::new(DenetimKapali)
    };
    println!("Ayar: {}", ayar_yolu.display());
    println!(
        "İzinli kökenler: {}",
        if ayar.izinli_kokenler.is_empty() {
            "(yok: hiçbir sayfa yazdıramaz)".into()
        } else {
            ayar.izinli_kokenler.join(", ")
        }
    );
    println!(
        "Ayarlı sürücüler: {}",
        ayar.yazicilar
            .keys()
            .map(|s| s.ad())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!(
        "Gövde sınırı: {} bayt. Cihaz imzası: {}.",
        ayar.en_cok_bayt,
        denetim.ad()
    );
    if komut == "denetle" {
        return ExitCode::SUCCESS;
    }
    let port = ayar.port;
    let dinleyici = match taa_yazdir::dinle(port) {
        Ok(d) => d,
        Err(e) => {
            eprintln!("127.0.0.1:{port} açılamadı (port dolu olabilir): {e}");
            return ExitCode::from(1);
        }
    };
    println!("taa-yazdir hazır: http://127.0.0.1:{port}/durum");
    let hizmet = Arc::new(Hizmet::ozel(ayar, Box::new(GercekYazici), denetim));
    taa_yazdir::calistir(hizmet, dinleyici, Arc::new(AtomicBool::new(false)));
    ExitCode::SUCCESS
}
