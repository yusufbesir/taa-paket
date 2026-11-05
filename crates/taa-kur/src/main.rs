// Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05]
//! `taa-kur` komut satırı (T1-TASARIM §1.4). Yalnız std::env::args; clap yok.
//! Çıkış kodları: 0 tamam, 2 doğrulama/kullanım, 3 çözüm yok, 4 yok, 5 bozuk, 6 kilitli, 1 diğer.
//! Mesajlar Türkçe tek satır (neden + öneri); `--json` makine okunur çıktı verir.

use serde_json::{json, Value};
use std::path::Path;
use std::process::ExitCode;
use taa_kur::{Kur, KurHata, Sorun};

const KULLANIM: &str = "Kullanım: taa-kur <liste|kur|kaldir|sec|geri-don|dogrula|tani> [bağımsız değişkenler] [--json]. Öneri: ayrıntı için docs/T1-TASARIM.md §1.4.";

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    args.retain(|a| a != "--json");
    match calistir(&args, json) {
        Ok(()) => ExitCode::from(0),
        Err(h) => {
            if json {
                let v = json!({"hata": h.mesaj(), "kod": h.cikis_kodu()});
                eprintln!("{v}");
            } else {
                eprintln!("{}", tek_satir(h.mesaj()));
            }
            ExitCode::from(h.cikis_kodu() as u8)
        }
    }
}

fn tek_satir(m: &str) -> String {
    m.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn kullanim(m: &str) -> KurHata {
    KurHata::Dogrulama(format!("{m} {KULLANIM}"))
}

/// `--sec` ve `--surum S` gibi seçenekleri ayırır; kalan konumsal bağımsız değişkenleri döner.
struct Ayri {
    konumsal: Vec<String>,
    sec: bool,
    surum: Option<String>,
}

fn ayir(args: &[String], izinli: &[&str]) -> Result<Ayri, KurHata> {
    let mut a = Ayri {
        konumsal: vec![],
        sec: false,
        surum: None,
    };
    let mut i = 0;
    while i < args.len() {
        let x = args[i].as_str();
        match x {
            "--sec" if izinli.contains(&x) => a.sec = true,
            "--surum" if izinli.contains(&x) => {
                i += 1;
                match args.get(i) {
                    Some(s) => a.surum = Some(s.clone()),
                    None => return Err(kullanim("--surum için değer eksik.")),
                }
            }
            _ if x.starts_with("--") => {
                return Err(kullanim(&format!("Bilinmeyen seçenek: {x}.")));
            }
            _ => a.konumsal.push(args[i].clone()),
        }
        i += 1;
    }
    Ok(a)
}

fn konumsal_say(a: &Ayri, n: usize, ornek: &str) -> Result<(), KurHata> {
    if a.konumsal.len() != n {
        return Err(kullanim(&format!(
            "Bağımsız değişken sayısı yanlış (beklenen {n}, verilen {}): {ornek}.",
            a.konumsal.len()
        )));
    }
    Ok(())
}

fn yaz_json(v: &Value) {
    println!("{v}");
}

fn calistir(args: &[String], json: bool) -> Result<(), KurHata> {
    let Some(komut) = args.first() else {
        return Err(kullanim("Komut verilmedi."));
    };
    let kalan = &args[1..];
    match komut.as_str() {
        "liste" => {
            let a = ayir(kalan, &[])?;
            konumsal_say(&a, 0, "taa-kur liste [--json]")?;
            let kur = Kur::ortamdan()?;
            let satirlar = kur.liste()?;
            if json {
                yaz_json(&serde_json::to_value(&satirlar).unwrap_or(Value::Null));
            } else if satirlar.is_empty() {
                println!("Kurulu paket yok. Öneri: `taa-kur kur <paket-klasoru>` çalıştırın.");
            } else {
                for s in &satirlar {
                    println!(
                        "{}\tkurulu: {}\tseçili: {}\tsabit: {}",
                        s.ad,
                        s.kurulu.join(","),
                        s.secili.as_deref().unwrap_or("-"),
                        if s.sabitle { "evet" } else { "hayır" }
                    );
                }
            }
            Ok(())
        }
        "kur" => {
            let a = ayir(kalan, &["--sec"])?;
            konumsal_say(&a, 1, "taa-kur kur <paket-klasoru> [--sec]")?;
            let kur = Kur::ortamdan()?;
            let s = kur.kur(Path::new(&a.konumsal[0]), a.sec)?;
            if json {
                yaz_json(&serde_json::to_value(&s).unwrap_or(Value::Null));
            } else {
                let eski = s
                    .eski
                    .as_ref()
                    .map(|e| format!(" Önceki içerik şuraya taşındı: {}.", e.display()))
                    .unwrap_or_default();
                println!(
                    "Kuruldu: {} {} ({}).{eski}",
                    s.ad,
                    s.surum,
                    if s.secili {
                        "seçili"
                    } else {
                        "seçili değil"
                    }
                );
            }
            Ok(())
        }
        "kaldir" => {
            let a = ayir(kalan, &["--surum"])?;
            konumsal_say(&a, 1, "taa-kur kaldir <ad> [--surum S]")?;
            let kur = Kur::ortamdan()?;
            let s = kur.kaldir(&a.konumsal[0], a.surum.as_deref())?;
            if json {
                yaz_json(&serde_json::to_value(&s).unwrap_or(Value::Null));
            } else {
                let mut m = format!(
                    "Kaldırıldı: {} {} (içerik şuraya taşındı: {}).",
                    s.ad,
                    s.surum,
                    s.eski.display()
                );
                if let Some(y) = &s.yeni_secili {
                    m.push_str(&format!(" Seçili sürüm: {y}."));
                }
                if !s.salt_okunur.is_empty() {
                    m.push_str(&format!(
                        " Veri alanları salt okunur olacak paketler: {}.",
                        s.salt_okunur.join(", ")
                    ));
                }
                for u in &s.uyarilar {
                    m.push_str(&format!(" Uyarı: {u}"));
                }
                println!("{}", tek_satir(&m));
            }
            Ok(())
        }
        "sec" => {
            let a = ayir(kalan, &[])?;
            konumsal_say(&a, 2, "taa-kur sec <ad> <surum>")?;
            let kur = Kur::ortamdan()?;
            kur.sec(&a.konumsal[0], &a.konumsal[1])?;
            if json {
                yaz_json(&json!({"ad": a.konumsal[0], "secili": a.konumsal[1]}));
            } else {
                println!("Seçildi: {} {}.", a.konumsal[0], a.konumsal[1]);
            }
            Ok(())
        }
        "geri-don" => {
            let a = ayir(kalan, &[])?;
            konumsal_say(&a, 1, "taa-kur geri-don <ad>")?;
            let kur = Kur::ortamdan()?;
            let s = kur.geri_don(&a.konumsal[0])?;
            if json {
                yaz_json(&json!({"ad": a.konumsal[0], "secili": s}));
            } else {
                println!("Geri dönüldü: {} {}.", a.konumsal[0], s);
            }
            Ok(())
        }
        "dogrula" => {
            let a = ayir(kalan, &[])?;
            if a.konumsal.len() > 1 {
                return Err(kullanim("Fazla bağımsız değişken: taa-kur dogrula [ad]."));
            }
            let kur = Kur::ortamdan()?;
            let sorunlar = kur.dogrula(a.konumsal.first().map(String::as_str))?;
            if json {
                yaz_json(&json!({
                    "saglam": sorunlar.is_empty(),
                    "sorunlar": serde_json::to_value(&sorunlar).unwrap_or(Value::Null),
                }));
            }
            if sorunlar.is_empty() {
                if !json {
                    println!("Doğrulama tamam: sorun bulunmadı.");
                }
                Ok(())
            } else {
                if !json {
                    for s in &sorunlar {
                        let (tur, m) = match s {
                            Sorun::Eksik(m) => ("eksik", m),
                            Sorun::Bozuk(m) => ("bozuk", m),
                        };
                        println!("Sorun ({tur}): {m}");
                    }
                }
                Err(KurHata::Bozuk(format!(
                    "{} sorun bulundu. Öneri: bozuk paketi yeniden kurun ya da kayıt dosyasını yedekten geri yükleyin.",
                    sorunlar.len()
                )))
            }
        }
        "tani" => {
            let a = ayir(kalan, &[])?;
            konumsal_say(&a, 0, "taa-kur tani [--json]")?;
            let kur = Kur::ortamdan()?;
            let t = kur.tani()?;
            if json {
                yaz_json(&serde_json::to_value(&t).unwrap_or(Value::Null));
            } else {
                println!("Kök: {} (kaynak: {:?})", t.kok, t.kok_kaynagi);
                println!(
                    "Kayıt: {}",
                    if t.kayit_saglam { "sağlam" } else { "SORUNLU" }
                );
                for s in &t.paketler {
                    println!(
                        "Paket: {} kurulu: {} seçili: {} sabit: {}",
                        s.ad,
                        s.kurulu.join(","),
                        s.secili.as_deref().unwrap_or("-"),
                        if s.sabitle { "evet" } else { "hayır" }
                    );
                }
                for (alan, sahip) in &t.sahiplik {
                    println!("Veri sahibi: {alan} -> {sahip}");
                }
                for s in &t.sorunlar {
                    println!("Sorun: {s:?}");
                }
                for h in &t.son_hatalar {
                    println!("Son hata: {}", tek_satir(h));
                }
            }
            Ok(())
        }
        diger => Err(kullanim(&format!("Bilinmeyen komut: {diger}."))),
    }
}
