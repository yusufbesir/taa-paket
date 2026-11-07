// Sahip: Yusuf. yazan: [Claude#taa-yol-S4a 2026-11-07]
//! `taa-yol` komut satırı. Yalnız std::env::args. Çıkış: 0 tamam, 2 kullanım/dosya, 3 ulaşılamadı, 5 liste yok, 1 diğer.

use std::path::PathBuf;
use std::process::ExitCode;
use taa_yol::{hata_kodu, sec, yollar_oku, yollar_yaz, CurlDeneyici, Yol, YolHata, Yollar};

const YARDIM: &str = "\
taa-yol — merkez sunucuya ulaşım yolları (doğrudan IP, VPN, alan adı, Cloudflare); ilk ulaşılan kullanılır.
Kullanım (--kok <veri kökü> | TAA_VERI_KOK; --json):
  taa-yol liste
  taa-yol ekle   --ad <ad> --taban <https://adres> [--sertifika sunucu-ca.pem | sistem | <dosya>] [--sunucu <mantıksal ad>]
                 (ilk eklemede --sunucu verilmezse 'merkez'; aynı ad varsa yerine yazar)
  taa-yol kaldir --ad <ad>
  taa-yol one-al --ad <ad>                   (öncelik: başa alır)
  taa-yol dene                               (sırayla GET /v1/saglik; ilk ulaşılanı yazar ve son.json'a kaydeder)
Dosya: <kök>\\yol\\sunucu-yollari.json. Sertifika verilmezse <kök>\\anahtarlar\\sunucu-ca.pem (Caddy iç CA kökü);
'sistem' = işletim sistemi kökleri (Let's Encrypt, Cloudflare).";

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
    let json = args.iter().any(|a| a == "--json");
    match calistir(&args, json) {
        Ok(()) => ExitCode::from(0),
        Err((kod, m)) => {
            if json {
                eprintln!("{}", serde_json::json!({"hata": m, "kod": kod}));
            } else {
                eprintln!("{m}");
            }
            ExitCode::from(kod)
        }
    }
}

type Hata = (u8, String);

fn yh(e: YolHata) -> Hata {
    (hata_kodu(&e), e.to_string())
}

fn calistir(args: &[String], json: bool) -> Result<(), Hata> {
    let mut komut: Option<String> = None;
    let mut kok: Option<PathBuf> = None;
    let mut ad: Option<String> = None;
    let mut taban: Option<String> = None;
    let mut sertifika: Option<String> = None;
    let mut sunucu: Option<String> = None;
    let mut i = 0;
    let deger = |i: &mut usize, a: &str| -> Result<String, Hata> {
        *i += 1;
        args.get(*i).cloned().ok_or_else(|| {
            (
                2,
                format!("{a} için değer gerekli. Öneri: taa-yol --yardim"),
            )
        })
    };
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--kok" => kok = Some(PathBuf::from(deger(&mut i, a)?)),
            "--ad" => ad = Some(deger(&mut i, a)?),
            "--taban" => taban = Some(deger(&mut i, a)?),
            "--sertifika" => sertifika = Some(deger(&mut i, a)?),
            "--sunucu" => sunucu = Some(deger(&mut i, a)?),
            "--json" => {}
            x if x.starts_with("--") => {
                return Err((
                    2,
                    format!("bilinmeyen seçenek: {x}. Öneri: taa-yol --yardim"),
                ))
            }
            x if komut.is_none() => komut = Some(x.to_string()),
            x => return Err((2, format!("fazla argüman: {x}"))),
        }
        i += 1;
    }
    let kok = kok
        .or_else(veri_kok_ortamdan)
        .ok_or_else(|| (2, "veri kökü bulunamadı. Öneri: --kok verin.".to_string()))?;
    let ad_gerek = || ad.clone().ok_or_else(|| (2, "--ad gerekli.".to_string()));
    match komut.as_deref() {
        Some("liste") => {
            let y = yollar_oku(&kok).map_err(yh)?;
            match y {
                None => {
                    if json {
                        println!("null");
                    } else {
                        println!("yol listesi yok ({}).", taa_yol::dosya_yolu(&kok).display());
                    }
                }
                Some(y) => {
                    if json {
                        println!("{}", y.json());
                    } else {
                        println!(
                            "sunucu: {}  (son çalışan: {})",
                            y.sunucu,
                            taa_yol::son_oku(&kok).unwrap_or("-".into())
                        );
                        for (n, v) in y.yollar.iter().enumerate() {
                            println!(
                                "{}. {}\t{}\t{}",
                                n + 1,
                                v.ad,
                                v.taban,
                                v.sertifika
                                    .as_deref()
                                    .unwrap_or(taa_yol::SERTIFIKA_VARSAYILAN)
                            );
                        }
                    }
                }
            }
        }
        Some("ekle") => {
            let ad = ad_gerek()?;
            let taban = taban.ok_or_else(|| (2, "--taban gerekli.".to_string()))?;
            let mut y = match yollar_oku(&kok).map_err(yh)? {
                Some(y) => y,
                None => Yollar::yeni(sunucu.as_deref().unwrap_or("merkez")).map_err(yh)?,
            };
            if let Some(s) = &sunucu {
                if s != &y.sunucu {
                    return Err((
                        2,
                        format!("liste '{}' sunucusuna ait; --sunucu {s} ile değiştirilemez (oturum anahtarı). Öneri: ayrı kök.", y.sunucu),
                    ));
                }
            }
            y.ekle(Yol::yeni(&ad, &taban, sertifika.as_deref()).map_err(yh)?)
                .map_err(yh)?;
            let yol = yollar_yaz(&kok, &y).map_err(yh)?;
            if json {
                println!("{}", y.json());
            } else {
                println!(
                    "eklendi: {ad} → {taban} ({} yol, {})",
                    y.yollar.len(),
                    yol.display()
                );
            }
        }
        Some("kaldir") | Some("one-al") => {
            let ad = ad_gerek()?;
            let mut y = yollar_oku(&kok)
                .map_err(yh)?
                .ok_or_else(|| (5, "yol listesi yok.".to_string()))?;
            let ok = if komut.as_deref() == Some("kaldir") {
                y.kaldir(&ad)
            } else {
                y.one_al(&ad)
            };
            if !ok {
                return Err((2, format!("yol yok: {ad}")));
            }
            yollar_yaz(&kok, &y).map_err(yh)?;
            if json {
                println!("{}", y.json());
            } else {
                println!("tamam: {ad} ({} yol)", y.yollar.len());
            }
        }
        Some("dene") => {
            let y = yollar_oku(&kok)
                .map_err(yh)?
                .ok_or_else(|| (5, "yol listesi yok.".to_string()))?;
            let v = sec(&kok, &y, &CurlDeneyici).map_err(yh)?;
            if json {
                println!(
                    "{}",
                    serde_json::json!({"ad": v.ad, "taban": v.taban, "sunucu": y.sunucu})
                );
            } else {
                println!("ulaşılan yol: {} → {}", v.ad, v.taban);
            }
        }
        None => return Err((2, "komut gerekli. Öneri: taa-yol --yardim".into())),
        Some(x) => return Err((2, format!("bilinmeyen komut: {x}. Öneri: taa-yol --yardim"))),
    }
    Ok(())
}
