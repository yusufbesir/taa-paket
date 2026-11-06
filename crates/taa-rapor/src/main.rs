// Sahip: Yusuf. yazan: [Claude#taa-rapor-T5 2026-11-06]
//! `taa-rapor` komut satırı. Yalnız std::env::args; clap yok. Salt okunur (yalnız --csv ile verilen dosyayı yazar).
//! Çıkış kodları: 0 tamam, 2 kullanım/doğrulama, 1 diğer.

use std::path::PathBuf;
use std::process::ExitCode;
use taa_rapor::{tarih, Kaynaklar};

const YARDIM: &str = "\
taa-rapor — TAA paket raporu: tüm veri alanlarını tek yerden okur (salt okunur).
Kullanım: taa-rapor [--kok <veri kökü>] [--fis-klasoru <klasör>] [--bugun YYYY-AA-GG] [--json] [--csv <dosya>]
  --kok          Veri kökü (<kök>\\fis-defteri\\fisler.jsonl, kasa-defteri\\kasa.jsonl, bildirim-kuyrugu\\bildirimler.jsonl,
                 urunler\\katalog.json, pano-kartlari\\, yedekler\\). Verilmezse TAA_VERI_KOK, yoksa %APPDATA%\\TAA\\veri.
  --fis-klasoru  taa-fis-sunucu --veri klasörü (fisler.jsonl ve bildirimler.jsonl oradan okunur).
  --fis/--kasa/--bildirim/--katalog <dosya>, --pano/--yedek <klasör>: tek kaynağı elle göster.
  --bugun        Geciken/bugün hesabı için tarih; verilmezse bugün (UTC).
  --json         Makine okunur çıktı.      --csv <dosya>  Excel'in açtığı CSV yazar (var olan dosya ezilmez, \"(n)\" eki alır).
Olmayan kaynak hata değildir: bölüm 'yok' görünür. Bozuk satır atlanır ve uyarı yazılır.";

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
    if args
        .iter()
        .any(|a| matches!(a.as_str(), "yardim" | "--yardim" | "--help" | "-h" | "help"))
    {
        println!("{YARDIM}");
        return ExitCode::from(0);
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

fn calistir(args: &[String]) -> Result<(), (u8, String)> {
    let mut kok: Option<PathBuf> = None;
    let mut fis_klasoru: Option<PathBuf> = None;
    let mut bugun: Option<String> = None;
    let mut json = false;
    let mut csv: Option<PathBuf> = None;
    let mut tekil: Vec<(String, PathBuf)> = vec![];
    let mut i = 0;
    let deger = |i: &mut usize, ad: &str| -> Result<String, (u8, String)> {
        *i += 1;
        args.get(*i).cloned().ok_or_else(|| {
            (
                2,
                format!("{ad} için değer eksik. Öneri: `taa-rapor yardim`."),
            )
        })
    };
    while i < args.len() {
        match args[i].as_str() {
            "--kok" => kok = Some(PathBuf::from(deger(&mut i, "--kok")?)),
            "--fis-klasoru" => fis_klasoru = Some(PathBuf::from(deger(&mut i, "--fis-klasoru")?)),
            "--bugun" => bugun = Some(deger(&mut i, "--bugun")?),
            "--json" => json = true,
            "--csv" => csv = Some(PathBuf::from(deger(&mut i, "--csv")?)),
            a @ ("--fis" | "--kasa" | "--bildirim" | "--katalog" | "--pano" | "--yedek") => {
                let ad = a.to_string();
                tekil.push((ad.clone(), PathBuf::from(deger(&mut i, &ad)?)));
            }
            a => {
                return Err((
                    2,
                    format!("Bilinmeyen bağımsız değişken: {a}. Öneri: `taa-rapor yardim`."),
                ))
            }
        }
        i += 1;
    }
    let kok = kok.or_else(veri_kok_ortamdan);
    let mut k = match &kok {
        Some(k) => Kaynaklar::veri_kokunden(k),
        None => Kaynaklar::default(),
    };
    if let Some(f) = &fis_klasoru {
        k.fis_klasoru_ekle(f);
    }
    for (ad, yol) in tekil {
        match ad.as_str() {
            "--fis" => k.fis = Some(yol),
            "--kasa" => k.kasa = Some(yol),
            "--bildirim" => k.bildirim = Some(yol),
            "--katalog" => k.katalog = Some(yol),
            "--pano" => k.pano = Some(yol),
            _ => k.yedek = Some(yol),
        }
    }
    let bugun = bugun.unwrap_or_else(tarih::bugun);
    let rapor = taa_rapor::uret(&k, &bugun).map_err(|m| (2, m))?;
    if json {
        println!("{}", rapor.json());
    } else {
        print!("{}", rapor.metin());
        if k.bos_mu() {
            println!(
                "Hiç kaynak bulunamadı (kök: {}). Öneri: --kok ya da --fis-klasoru ver.",
                kok.map(|p| p.display().to_string())
                    .unwrap_or_else(|| "çözülemedi".into())
            );
        }
    }
    if let Some(c) = csv {
        let yazilan = taa_dosya::atomik_yaz(&c, rapor.csv().as_bytes())
            .map_err(|e| (1, format!("CSV yazılamadı ({}): {e}", c.display())))?;
        if json {
            eprintln!(
                "{}",
                serde_json::json!({"csv": yazilan.display().to_string()})
            );
        } else {
            println!("CSV yazıldı: {}", yazilan.display());
        }
    }
    Ok(())
}
