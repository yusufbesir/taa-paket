// Sahip: Yusuf. yazan: [Claude#taa-kuyruk-T6 2026-11-06]
//! `taa-kuyruk` komut satırı. Yalnız std::env::args; clap yok.
//! Çıkış kodları: 0 tamam (gönderimde kalan olsa da), 2 kullanım/doğrulama, 3 sunucuya ulaşılamadı, 1 diğer.
//! Oturum anahtarı komut satırında verilmez: `--oturum-dosyasi <yol>` (tek satır) ya da TAA_OTURUM ortam değişkeni.

use std::path::PathBuf;
use std::process::ExitCode;
use taa_kuyruk::{CurlGonderici, Durum, Ekleme, Kuyruk};

const YARDIM: &str = "\
taa-kuyruk — merkez sunucuya giden yerel kuyruk (çevrimdışı birikir, bağlantı gelince no ile idempotent gönderilir).
Kullanım:
  taa-kuyruk [--kok <veri kökü>] ekle --program <ad> <uç> <no> <json|@dosya>
  taa-kuyruk [--kok <veri kökü>] liste [--program <ad>] [--json]
  taa-kuyruk [--kok <veri kökü>] gonder --sunucu <https://adres> [--program <ad>] [--oturum-dosyasi <yol>]
             [--sertifika <pem>] [--en-cok N] [--json]
  --kok       Veri kökü (kuyruk <kök>\\kuyruk\\<program>\\giden.jsonl). Verilmezse TAA_VERI_KOK, yoksa %APPDATA%\\TAA\\veri.
  --program   Kuyruğu yazan program adı ([a-z0-9-]). liste/gonder'de verilmezse tüm programlar.
  <uç>        Sunucu ucu, örnek /v1/olay.   <no>  Kaydın tek numarası (şube önekli, örnek S1-2026-000123).
  Oturum anahtarı: --oturum-dosyasi ile tek satırlık dosyadan ya da TAA_OTURUM ortam değişkeninden; komut satırına yazılmaz.
Hiçbir satır silinmez: kabul edilen gonderildi.jsonl'e, kalıcı reddedilen reddedildi.jsonl'e eklenir.";

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
        Ok(kod) => ExitCode::from(kod),
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

struct Secenekler {
    kok: Option<PathBuf>,
    program: Option<String>,
    sunucu: Option<String>,
    oturum_dosyasi: Option<PathBuf>,
    sertifika: Option<PathBuf>,
    en_cok: usize,
    json: bool,
    komut: Option<String>,
    kalan: Vec<String>,
}

fn secenekleri_coz(args: &[String]) -> Result<Secenekler, (u8, String)> {
    let mut s = Secenekler {
        kok: None,
        program: None,
        sunucu: None,
        oturum_dosyasi: None,
        sertifika: None,
        en_cok: 0,
        json: false,
        komut: None,
        kalan: Vec::new(),
    };
    let mut i = 0;
    let deger = |i: &mut usize, ad: &str| -> Result<String, (u8, String)> {
        *i += 1;
        args.get(*i).cloned().ok_or_else(|| {
            (
                2,
                format!("{ad} için değer gerekli. Öneri: taa-kuyruk --yardim"),
            )
        })
    };
    while i < args.len() {
        match args[i].as_str() {
            "--kok" => s.kok = Some(PathBuf::from(deger(&mut i, "--kok")?)),
            "--program" => s.program = Some(deger(&mut i, "--program")?),
            "--sunucu" => s.sunucu = Some(deger(&mut i, "--sunucu")?),
            "--oturum-dosyasi" => {
                s.oturum_dosyasi = Some(PathBuf::from(deger(&mut i, "--oturum-dosyasi")?))
            }
            "--sertifika" => s.sertifika = Some(PathBuf::from(deger(&mut i, "--sertifika")?)),
            "--en-cok" => {
                let d = deger(&mut i, "--en-cok")?;
                s.en_cok = d
                    .parse()
                    .map_err(|_| (2, format!("--en-cok sayı olmalı: {d}")))?;
            }
            "--json" => s.json = true,
            a if a.starts_with("--") => {
                return Err((
                    2,
                    format!("bilinmeyen seçenek: {a}. Öneri: taa-kuyruk --yardim"),
                ))
            }
            a => {
                if s.komut.is_none() {
                    s.komut = Some(a.to_string());
                } else {
                    s.kalan.push(a.to_string());
                }
            }
        }
        i += 1;
    }
    Ok(s)
}

fn kuyruklar(
    kok: &std::path::Path,
    program: &Option<String>,
) -> Result<Vec<(String, Kuyruk)>, (u8, String)> {
    match program {
        Some(p) => Ok(vec![(
            p.clone(),
            Kuyruk::program(kok, p).map_err(|e| (2, e.to_string()))?,
        )]),
        None => Kuyruk::hepsi(kok).map_err(|e| (1, e.to_string())),
    }
}

fn calistir(args: &[String]) -> Result<u8, (u8, String)> {
    let s = secenekleri_coz(args)?;
    let kok = s
        .kok
        .clone()
        .or_else(veri_kok_ortamdan)
        .ok_or_else(|| (2, "veri kökü bulunamadı. Öneri: --kok verin.".to_string()))?;
    match s.komut.as_deref() {
        Some("ekle") => {
            let p = s
                .program
                .as_deref()
                .ok_or_else(|| (2, "ekle için --program gerekli.".to_string()))?;
            let [uc, no, govde] = s.kalan.as_slice() else {
                return Err((
                    2,
                    "ekle <uç> <no> <json|@dosya> ister. Öneri: taa-kuyruk --yardim".into(),
                ));
            };
            let metin = match govde.strip_prefix('@') {
                Some(d) => {
                    std::fs::read_to_string(d).map_err(|e| (2, format!("{d} okunamadı: {e}")))?
                }
                None => govde.clone(),
            };
            let v: serde_json::Value =
                serde_json::from_str(&metin).map_err(|e| (2, format!("gövde JSON değil: {e}")))?;
            let k = Kuyruk::program(&kok, p).map_err(|e| (2, e.to_string()))?;
            let sonuc = k.ekle(uc, no, v).map_err(|e| (2, e.to_string()))?;
            let (durum, kayit) = match sonuc {
                Ekleme::Eklendi(k) => ("eklendi", k),
                Ekleme::ZatenVar(k) => ("zaten-var", k),
            };
            if s.json {
                println!(
                    "{}",
                    serde_json::json!({"durum": durum, "no": kayit.no, "uc": kayit.uc, "zaman": kayit.zaman})
                );
            } else {
                println!("{durum}: {} → {} ({})", kayit.no, kayit.uc, kayit.zaman);
            }
            Ok(0)
        }
        Some("liste") => {
            let mut satirlar = Vec::new();
            let mut say = [0usize; 3];
            for (p, k) in kuyruklar(&kok, &s.program)? {
                for (kayit, d) in k.durumlar().map_err(|e| (1, e.to_string()))? {
                    say[match d {
                        Durum::Bekliyor => 0,
                        Durum::Gonderildi => 1,
                        Durum::Reddedildi => 2,
                    }] += 1;
                    satirlar.push(serde_json::json!({
                        "program": p, "no": kayit.no, "uc": kayit.uc, "zaman": kayit.zaman, "durum": d.metin()
                    }));
                }
            }
            if s.json {
                println!(
                    "{}",
                    serde_json::json!({"bekleyen": say[0], "gonderilen": say[1], "reddedilen": say[2], "kayitlar": satirlar})
                );
            } else {
                for r in &satirlar {
                    println!(
                        "{:<11} {:<14} {:<28} {} {}",
                        r["durum"].as_str().unwrap_or(""),
                        r["program"].as_str().unwrap_or(""),
                        r["no"].as_str().unwrap_or(""),
                        r["uc"].as_str().unwrap_or(""),
                        r["zaman"].as_str().unwrap_or("")
                    );
                }
                println!(
                    "bekleyen {} · gönderilen {} · reddedilen {}",
                    say[0], say[1], say[2]
                );
            }
            Ok(0)
        }
        Some("gonder") => {
            let sunucu = s
                .sunucu
                .as_deref()
                .ok_or_else(|| (2, "gonder için --sunucu gerekli.".to_string()))?;
            let mut g = CurlGonderici::yeni(sunucu).map_err(|e| (2, e.to_string()))?;
            let oturum = match &s.oturum_dosyasi {
                Some(y) => Some(
                    std::fs::read_to_string(y)
                        .map_err(|e| (2, format!("oturum dosyası okunamadı: {e}")))?,
                ),
                None => std::env::var("TAA_OTURUM").ok(),
            };
            if let Some(o) = oturum {
                g = g.oturum(&o).map_err(|e| (2, e.to_string()))?;
            }
            if let Some(c) = &s.sertifika {
                g = g.sertifika(c);
            }
            let mut toplam = taa_kuyruk::GonderOzeti::default();
            let mut programlar = Vec::new();
            for (p, k) in kuyruklar(&kok, &s.program)? {
                let o = k.gonder(&g, s.en_cok).map_err(|e| (1, e.to_string()))?;
                toplam.gonderilen += o.gonderilen;
                toplam.reddedilen += o.reddedilen;
                toplam.kalan += o.kalan;
                programlar.push(serde_json::json!({
                    "program": p, "gonderilen": o.gonderilen, "reddedilen": o.reddedilen, "kalan": o.kalan,
                    "durma_nedeni": o.durma_nedeni
                }));
                if let Some(n) = o.durma_nedeni {
                    toplam.durma_nedeni = Some(n);
                    break;
                }
            }
            if s.json {
                println!(
                    "{}",
                    serde_json::json!({"gonderilen": toplam.gonderilen, "reddedilen": toplam.reddedilen,
                        "kalan": toplam.kalan, "durma_nedeni": toplam.durma_nedeni, "programlar": programlar})
                );
            } else {
                println!(
                    "gönderilen {} · reddedilen {} · kalan {}",
                    toplam.gonderilen, toplam.reddedilen, toplam.kalan
                );
                if let Some(n) = &toplam.durma_nedeni {
                    println!("durdu: {n}");
                }
            }
            Ok(if toplam.durma_nedeni.is_some() { 3 } else { 0 })
        }
        Some(k) => Err((
            2,
            format!("bilinmeyen komut: {k}. Öneri: taa-kuyruk --yardim"),
        )),
        None => Err((
            2,
            "komut gerekli: ekle | liste | gonder. Öneri: taa-kuyruk --yardim".into(),
        )),
    }
}
