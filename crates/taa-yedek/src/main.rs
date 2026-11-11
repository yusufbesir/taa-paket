//! taa-yedek komut satırı: anahtar üret, (şifreli) yedekle, doğrula, geri yükle. Hiçbir şey silinmez/ezilmez.
//! Anahtar yalnız `--anahtar <dosya>` ile verilir (örn. USB'de); çıktıya yazılmaz.
//! Yazan: [Claude#taa-kur-sifre 2026-11-11].

use std::path::PathBuf;
use std::process::ExitCode;
use taa_yedek::{
    anahtar_oku, anahtar_uret, dogrula, geri_yukle, geri_yukle_sifreli, yedekle, yedekle_sifreli,
    Sorun,
};

const KULLANIM: &str = "Kullanım:
  taa-yedek anahtar-uret --anahtar <dosya>
  taa-yedek yedekle --kaynak <klasor> [--kaynak ...] --hedef <klasor> --damga <ad> [--anahtar <dosya>]
  taa-yedek dogrula <yedek-klasoru>
  taa-yedek geri-yukle <yedek-klasoru> --hedef <klasor> [--anahtar <dosya>]
Anahtar verilirse yedek XChaCha20-Poly1305 ile şifrelenir/çözülür. Anahtar dosyasını USB'de tutun, kimseye vermeyin.";

fn calistir(a: &[String]) -> Result<(), String> {
    let (mut kaynaklar, mut hedef, mut damga, mut anahtar) = (Vec::new(), None, None, None);
    let mut konum: Vec<String> = Vec::new();
    let mut i = 1;
    while i < a.len() {
        let t = a[i].as_str();
        if matches!(t, "--kaynak" | "--hedef" | "--damga" | "--anahtar") {
            i += 1;
            let d = a
                .get(i)
                .ok_or_else(|| format!("{t} için değer eksik.\n{KULLANIM}"))?;
            match t {
                "--kaynak" => kaynaklar.push(PathBuf::from(d)),
                "--hedef" => hedef = Some(PathBuf::from(d)),
                "--damga" => damga = Some(d.clone()),
                _ => anahtar = Some(PathBuf::from(d)),
            }
        } else if t.starts_with("--") {
            return Err(format!("Bilinmeyen seçenek: {t}\n{KULLANIM}"));
        } else {
            konum.push(t.to_string());
        }
        i += 1;
    }
    let anahtar_yukle = || anahtar.as_ref().map(|y| anahtar_oku(y)).transpose();
    match a.first().map(String::as_str) {
        Some("anahtar-uret") => {
            let y = anahtar.ok_or("--anahtar <dosya> gerekli")?;
            anahtar_uret(&y)?;
            println!(
                "Anahtar yazıldı: {} (yedeğini ikinci bir USB'ye alın).",
                y.display()
            );
        }
        Some("yedekle") => {
            let h = hedef.ok_or("--hedef gerekli")?;
            let d = damga.ok_or("--damga gerekli")?;
            if kaynaklar.is_empty() {
                return Err("en az bir --kaynak gerekli".into());
            }
            let y = match anahtar_yukle()? {
                Some(k) => yedekle_sifreli(&kaynaklar, &h, &d, &k)?,
                None => yedekle(&kaynaklar, &h, &d)?,
            };
            println!("Yedek hazır: {}", y.display());
        }
        Some("dogrula") => {
            let y = konum.first().ok_or("yedek klasörü gerekli")?;
            let s = dogrula(&PathBuf::from(y))?;
            if s.is_empty() {
                println!("Yedek sağlam.");
            } else {
                for x in &s {
                    match x {
                        Sorun::Eksik(p) => println!("EKSİK: {p}"),
                        Sorun::Bozuk(p) => println!("BOZUK: {p}"),
                    }
                }
                return Err(format!("{} sorun bulundu", s.len()));
            }
        }
        Some("geri-yukle") => {
            let y = PathBuf::from(konum.first().ok_or("yedek klasörü gerekli")?);
            let h = hedef.ok_or("--hedef gerekli")?;
            let atlanan = match anahtar_yukle()? {
                Some(k) => geri_yukle_sifreli(&y, &h, &k)?,
                None => geri_yukle(&y, &h)?,
            };
            println!("Geri yüklendi. Var olduğu için atlanan: {}", atlanan.len());
        }
        _ => return Err(KULLANIM.to_string()),
    }
    Ok(())
}

fn main() -> ExitCode {
    let a: Vec<String> = std::env::args().skip(1).collect();
    match calistir(&a) {
        Ok(()) => ExitCode::SUCCESS,
        Err(m) => {
            eprintln!("{m}");
            ExitCode::from(2)
        }
    }
}
