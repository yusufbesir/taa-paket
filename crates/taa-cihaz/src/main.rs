// Sahip: Yusuf. yazan: [Claude#taa-cihaz-S2 2026-11-06]
//! `taa-cihaz` komut satırı. Yalnız std::env::args; clap yok. Özel anahtar hiç basılmaz.
//! Çıkış kodları: 0 tamam, 2 kullanım/doğrulama, 3 sunucuya ulaşılamadı, 4 sunucu reddetti, 5 anahtar yok, 1 diğer.

use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;
use taa_cihaz::{durum, kayit, CihazAnahtari, CihazHata};

const YARDIM: &str = "\
taa-cihaz — bu PC'nin cihaz kimliği (ed25519). Sunucu yalnız kayıtlı ve onaylı cihazın isteğini kabul eder.
Kullanım:
  taa-cihaz uret   [--kok <veri kökü>] [--ad <cihaz adı>] [--json]      (var olanın üzerine yazmaz)
  taa-cihaz goster [--kok <veri kökü>] [--json]                           (kimlik ve açık anahtar; özel anahtar basılmaz)
  taa-cihaz kayit  --sunucu <https://adres> [--sube <kod>] [--sertifika <pem>] [--kok] [--json]
  taa-cihaz durum  --sunucu <https://adres> [--sertifika <pem>] [--kok] [--json]
  --kok     Verilmezse TAA_VERI_KOK, yoksa %APPDATA%\\TAA\\veri.   --sunucu  Verilmezse TAA_SUNUCU.
Dosyalar: <kök>\\cihaz\\cihaz.key (gizli), cihaz.pub, cihaz.json. Kayıt sonrası yönetici TAA Çatı'dan cihazı onaylar.";

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
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty()
        || args
            .iter()
            .any(|a| matches!(a.as_str(), "yardim" | "--yardim" | "--help" | "-h" | "help"))
    {
        println!("{YARDIM}");
        return ExitCode::from(if args.is_empty() { 2 } else { 0 });
    }
    match calistir(&args) {
        Ok(()) => ExitCode::from(0),
        Err((kod, m)) => {
            if args.iter().any(|a| a == "--json") {
                eprintln!("{}", serde_json::json!({"hata": m, "kod": kod}));
            } else {
                eprintln!("{m}");
            }
            ExitCode::from(kod)
        }
    }
}

fn hata(e: CihazHata) -> (u8, String) {
    let m = e.to_string();
    let kod = match &e {
        CihazHata::AnahtarYok(_) => 5,
        CihazHata::GecersizGirdi(_) | CihazHata::Dosya(_) => 2,
        CihazHata::Sunucu(_) => taa_cihaz::hata_kodu(&m),
    };
    (kod, m)
}

fn calistir(args: &[String]) -> Result<(), (u8, String)> {
    let mut komut: Option<String> = None;
    let mut kok: Option<PathBuf> = None;
    let mut ad: Option<String> = None;
    let mut sube: Option<String> = None;
    let mut sunucu: Option<String> = None;
    let mut sertifika: Option<PathBuf> = None;
    let mut json = false;
    let mut i = 0;
    let deger = |i: &mut usize, a: &str| -> Result<String, (u8, String)> {
        *i += 1;
        args.get(*i).cloned().ok_or_else(|| {
            (
                2,
                format!("{a} için değer gerekli. Öneri: taa-cihaz --yardim"),
            )
        })
    };
    while i < args.len() {
        match args[i].as_str() {
            "--kok" => kok = Some(PathBuf::from(deger(&mut i, "--kok")?)),
            "--ad" => ad = Some(deger(&mut i, "--ad")?),
            "--sube" => sube = Some(deger(&mut i, "--sube")?),
            "--sunucu" => sunucu = Some(deger(&mut i, "--sunucu")?),
            "--sertifika" => sertifika = Some(PathBuf::from(deger(&mut i, "--sertifika")?)),
            "--json" => json = true,
            a if a.starts_with('-') => {
                return Err((
                    2,
                    format!("bilinmeyen seçenek: {a}. Öneri: taa-cihaz --yardim"),
                ))
            }
            a => {
                if komut.is_some() {
                    return Err((2, format!("fazla komut: {a}")));
                }
                komut = Some(a.to_string());
            }
        }
        i += 1;
    }
    let kok = kok
        .or_else(veri_kok_ortamdan)
        .ok_or_else(|| (2, "veri kökü bulunamadı. Öneri: --kok verin.".to_string()))?;
    let sunucu_al = || {
        sunucu
            .clone()
            .or_else(|| {
                std::env::var("TAA_SUNUCU")
                    .ok()
                    .filter(|s| !s.trim().is_empty())
            })
            .ok_or_else(|| (2, "--sunucu (ya da TAA_SUNUCU) gerekli.".to_string()))
    };
    match komut.as_deref() {
        Some("uret") => {
            let c = CihazAnahtari::uret(&kok, ad.as_deref()).map_err(hata)?;
            if json {
                println!("{}", c.ozet());
            } else {
                println!(
                    "cihaz anahtarı üretildi: kimlik {} ad {} ({})",
                    c.kimlik(),
                    c.ad(),
                    CihazAnahtari::klasor(&kok).display()
                );
            }
            Ok(())
        }
        Some("goster") => {
            let c = CihazAnahtari::yukle(&kok).map_err(hata)?;
            if json {
                println!("{}", c.ozet());
            } else {
                println!(
                    "kimlik {}  ad {}  açık anahtar {}",
                    c.kimlik(),
                    c.ad(),
                    c.genel().metin()
                );
            }
            Ok(())
        }
        Some("kayit") | Some("durum") => {
            let s = sunucu_al()?;
            let c = Arc::new(CihazAnahtari::yukle(&kok).map_err(hata)?);
            let g = Arc::clone(&c)
                .gonderici(&s, sertifika.as_deref())
                .map_err(hata)?;
            let yanit = if komut.as_deref() == Some("kayit") {
                kayit(&g, &c, sube.as_deref()).map_err(hata)?
            } else {
                durum(&g, &c).map_err(hata)?
            };
            if json {
                println!("{yanit}");
            } else {
                println!(
                    "cihaz {} durum: {}{}",
                    c.kimlik(),
                    yanit.get("durum").and_then(|d| d.as_str()).unwrap_or("?"),
                    if yanit.get("durum").and_then(|d| d.as_str()) == Some("bekliyor") {
                        " — yönetici TAA Çatı'dan onaylamalı"
                    } else {
                        ""
                    }
                );
            }
            Ok(())
        }
        Some(k) => Err((
            2,
            format!("bilinmeyen komut: {k}. Öneri: taa-cihaz --yardim"),
        )),
        None => Err((2, "komut gerekli (uret|goster|kayit|durum).".into())),
    }
}
