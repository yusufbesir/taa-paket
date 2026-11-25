// Sahip: Yusuf. yazan: [Claude#dinamik-adres 2026-11-25]
//! `taa-dns` komut satırı. Çıkış: 0 tamam, 2 kullanım/dosya, 1 güncellenemedi.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use taa_dns::*;

const YARDIM: &str = "\
taa-dns — dinamik DNS güncelleyici (yalnız sunucu PC'sinde). Sağlayıcı: duckdns | url (kendi güncelleme adresi).
Kullanım (--kok <veri kökü> | TAA_VERI_KOK; --json):
  taa-dns ayarla --saglayici duckdns --alan <alt-ad>
  taa-dns ayarla --saglayici url --alan <ana-bilgisayar> --sablon \"https://…?h={alan}&t={token}&ip={ip}\"
  taa-dns guncelle [--ip <IPv4>]    (IP verilmezse sağlayıcı bağlantı kaynağını görür)
  taa-dns durum                     (ayar + son sonuç; belirteç gösterilmez)
  taa-dns zamanla kur|kaldir|durum  (Windows) 5 dakikada bir `guncelle` çalıştıran görev
Belirteç: <kök>\\anahtarlar\\dns-token dosyasına (tek satır) SİZ yazın; komut satırına verilmez, hiçbir yere yazdırılmaz.";

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

type Hata = (u8, String);

fn dh(e: DnsHata) -> Hata {
    (
        match e {
            DnsHata::GecersizGirdi(_) | DnsHata::Dosya(_) => 2,
            DnsHata::Guncellenemedi(_) => 1,
        },
        e.to_string(),
    )
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
        Ok(k) => ExitCode::from(k),
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

fn calistir(args: &[String], json: bool) -> Result<u8, Hata> {
    let (mut komut, mut alt, mut kok) = (None::<String>, None::<String>, None::<PathBuf>);
    let (mut saglayici, mut alan, mut sablon, mut ip) = (
        None::<String>,
        None::<String>,
        None::<String>,
        None::<String>,
    );
    let mut i = 0;
    while i < args.len() {
        let a = args[i].as_str();
        let mut deger = || -> Result<String, Hata> {
            i += 1;
            args.get(i).cloned().ok_or_else(|| {
                (
                    2,
                    format!("{a} için değer gerekli. Öneri: taa-dns --yardim"),
                )
            })
        };
        match a {
            "--kok" => kok = Some(PathBuf::from(deger()?)),
            "--saglayici" => saglayici = Some(deger()?),
            "--alan" => alan = Some(deger()?),
            "--sablon" => sablon = Some(deger()?),
            "--ip" => ip = Some(deger()?),
            "--json" => {}
            x if x.starts_with("--") => {
                return Err((
                    2,
                    format!("bilinmeyen seçenek: {x}. Öneri: taa-dns --yardim"),
                ))
            }
            x if komut.is_none() => komut = Some(x.to_string()),
            x if komut.as_deref() == Some("zamanla") && alt.is_none() => alt = Some(x.to_string()),
            x => return Err((2, format!("fazla argüman: {x}"))),
        }
        i += 1;
    }
    let kok = kok
        .or_else(veri_kok_ortamdan)
        .ok_or_else(|| (2, "veri kökü bulunamadı. Öneri: --kok verin.".to_string()))?;
    match komut.as_deref() {
        Some("ayarla") => {
            let a = Ayar::yeni(
                &saglayici.ok_or((2, "--saglayici gerekli.".to_string()))?,
                &alan.ok_or((2, "--alan gerekli.".to_string()))?,
                sablon.as_deref(),
            )
            .map_err(dh)?;
            let yol = ayar_yaz(&kok, &a).map_err(dh)?;
            let var = token_yolu(&kok).exists();
            if json {
                println!(
                    "{}",
                    serde_json::json!({"ayar": a.json(), "belirtec_var": var})
                );
            } else {
                println!("ayar yazıldı: {} ({})", a.alan, yol.display());
                if !var {
                    println!(
                        "Sıradaki adım: belirteci siz şu dosyaya yazın (tek satır): {}",
                        token_yolu(&kok).display()
                    );
                }
            }
            Ok(0)
        }
        Some("guncelle") => {
            if let Some(x) = &ip {
                if !ip_gecerli(x) {
                    return Err((2, format!("IPv4 geçersiz: {x}")));
                }
            }
            let m = guncelle(&kok, ip.as_deref(), &CurlIstekci).map_err(dh)?;
            if json {
                println!("{}", serde_json::json!({"tamam": true, "ileti": m}));
            } else {
                println!("{m}");
            }
            Ok(0)
        }
        Some("durum") => {
            let a = ayar_oku(&kok).map_err(dh)?;
            let var = token_yolu(&kok).exists();
            let son = son_oku(&kok);
            if json {
                println!(
                    "{}",
                    serde_json::json!({"ayar": a.map(|a| a.json()), "belirtec_var": var, "son": son})
                );
            } else {
                match a {
                    None => println!("dns ayarı yok."),
                    Some(a) => println!(
                        "alan: {}  belirteç dosyası: {}",
                        a.alan,
                        if var { "var" } else { "YOK" }
                    ),
                }
                if let Some(s) = son {
                    println!("son sonuç: {s}");
                }
            }
            Ok(0)
        }
        Some("zamanla") => zamanla(alt.as_deref(), &kok, json),
        None => Err((2, "komut gerekli. Öneri: taa-dns --yardim".into())),
        Some(x) => Err((2, format!("bilinmeyen komut: {x}. Öneri: taa-dns --yardim"))),
    }
}

const GOREV_ADI: &str = "TAA DNS Guncelle";

fn zamanla(alt: Option<&str>, kok: &Path, json: bool) -> Result<u8, Hata> {
    if !cfg!(windows) {
        return Err((2, "zamanla yalnız Windows'ta çalışır (schtasks).".into()));
    }
    let schtasks = |a: &[&str]| -> Result<(bool, String), Hata> {
        let o = std::process::Command::new("schtasks")
            .args(a)
            .output()
            .map_err(|e| (1u8, format!("schtasks çalışmadı: {e}")))?;
        let m = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        Ok((o.status.success(), m.trim().to_string()))
    };
    let bitir = |durum: &str, ok: bool, m: String| -> Result<u8, Hata> {
        if json {
            println!(
                "{}",
                serde_json::json!({"durum": durum, "tamam": ok, "ayrinti": m})
            );
        } else if m.is_empty() {
            println!("{durum}");
        } else {
            println!("{durum}: {m}");
        }
        Ok(if ok { 0 } else { 1 })
    };
    match alt {
        Some("kur") => {
            let exe = std::env::current_exe().map_err(|e| (1u8, e.to_string()))?;
            let arka = exe.with_file_name("taa-dns-arka.exe");
            let hedef = if arka.exists() { arka } else { exe };
            let komut = format!("\"{}\" --kok \"{}\"", hedef.display(), kok.display());
            let (ok, m) = schtasks(&[
                "/Create", "/TN", GOREV_ADI, "/SC", "MINUTE", "/MO", "5", "/TR", &komut, "/F",
            ])?;
            bitir(if ok { "kuruldu" } else { "kurulamadı" }, ok, m)
        }
        Some("kaldir") => {
            let (ok, m) = schtasks(&["/Delete", "/TN", GOREV_ADI, "/F"])?;
            bitir(
                if ok {
                    "kaldırıldı"
                } else {
                    "kaldırılamadı"
                },
                ok,
                m,
            )
        }
        Some("durum") => {
            let (ok, m) = schtasks(&["/Query", "/TN", GOREV_ADI])?;
            bitir(if ok { "kurulu" } else { "kurulu değil" }, ok, m)
        }
        _ => Err((
            2,
            "zamanla kur|kaldir|durum olmalı. Öneri: taa-dns --yardim".into(),
        )),
    }
}
