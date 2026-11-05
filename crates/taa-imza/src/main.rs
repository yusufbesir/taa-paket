// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! `taa-imza uret|imzala|dogrula` (docs/T3-TASARIM.md §1.2). Özel anahtar yalnız `--ozel <dosya>` ile verilen dosyadan okunur;
//! hiçbir çıktıya yazılmaz. Çıkış: 0 tamam, 2 doğrulama/kullanım, 1 diğer.

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use taa_imza::*;

const KULLANIM: &str = "Kullanım: taa-imza uret --ozel <dosya> --genel <dosya> | imzala <dosya> --ozel <dosya> [--tur ham|paket|yayin] [--cikti <dosya>] | dogrula <dosya> --genel <dosya> [--tur ham|paket|yayin] [--imza <dosya>]. Özel anahtarı depoya koymayın.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match calistir(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err((kod, m)) => {
            eprintln!("{}", m.split_whitespace().collect::<Vec<_>>().join(" "));
            ExitCode::from(kod)
        }
    }
}

type Sonuc = Result<(), (u8, String)>;

fn kullanim(m: &str) -> (u8, String) {
    (2, format!("{m} {KULLANIM}"))
}

fn h(e: ImzaHata) -> (u8, String) {
    let kod = match e {
        ImzaHata::Dosya(_) => 1,
        _ => 2,
    };
    (kod, e.mesaj().to_string())
}

struct Secenek {
    konumsal: Vec<String>,
    ozel: Option<PathBuf>,
    genel: Option<PathBuf>,
    tur: String,
    cikti: Option<PathBuf>,
    imza: Option<PathBuf>,
}

fn ayir(args: &[String]) -> Result<Secenek, (u8, String)> {
    let mut s = Secenek {
        konumsal: vec![],
        ozel: None,
        genel: None,
        tur: "ham".into(),
        cikti: None,
        imza: None,
    };
    let mut i = 0;
    while i < args.len() {
        let x = args[i].as_str();
        if x.starts_with("--") {
            i += 1;
            let Some(d) = args.get(i) else {
                return Err(kullanim(&format!("{x} için değer eksik.")));
            };
            match x {
                "--ozel" => s.ozel = Some(d.into()),
                "--genel" => s.genel = Some(d.into()),
                "--tur" => s.tur = d.clone(),
                "--cikti" => s.cikti = Some(d.into()),
                "--imza" => s.imza = Some(d.into()),
                _ => return Err(kullanim(&format!("Bilinmeyen seçenek: {x}."))),
            }
        } else {
            s.konumsal.push(x.to_string());
        }
        i += 1;
    }
    if !matches!(s.tur.as_str(), "ham" | "paket" | "yayin") {
        return Err(kullanim("--tur ham, paket ya da yayin olmalı."));
    }
    Ok(s)
}

fn gerekli<'a>(o: &'a Option<PathBuf>, ad: &str) -> Result<&'a Path, (u8, String)> {
    o.as_deref()
        .ok_or_else(|| kullanim(&format!("{ad} verilmedi.")))
}

fn json_oku(yol: &Path) -> Result<serde_json::Value, (u8, String)> {
    let b = std::fs::read(yol).map_err(|e| (1, format!("{} okunamadı: {e}.", yol.display())))?;
    serde_json::from_slice(&b)
        .map_err(|e| (2, format!("{} geçerli JSON değil: {e}.", yol.display())))
}

fn calistir(args: &[String]) -> Sonuc {
    let Some(komut) = args.first() else {
        return Err(kullanim("Komut verilmedi."));
    };
    let s = ayir(&args[1..])?;
    match komut.as_str() {
        "uret" => {
            if !s.konumsal.is_empty() {
                return Err(kullanim("uret konumsal bağımsız değişken almaz."));
            }
            let g = anahtar_uret(gerekli(&s.ozel, "--ozel")?, gerekli(&s.genel, "--genel")?)
                .map_err(h)?;
            println!(
                "Anahtar üretildi. Genel anahtar: {} (kimlik {}). Özel anahtar dosyası gizli tutulmalı; depoya koymayın.",
                g.metin(),
                g.kimlik()
            );
            Ok(())
        }
        "imzala" => {
            if s.konumsal.len() != 1 {
                return Err(kullanim("imzala tam bir dosya ister."));
            }
            let dosya = Path::new(&s.konumsal[0]);
            let (oz, uyari) = OzelAnahtar::dosyadan(gerekli(&s.ozel, "--ozel")?).map_err(h)?;
            if let Some(u) = uyari {
                eprintln!("{u}");
            }
            match s.tur.as_str() {
                "ham" => {
                    let b = std::fs::read(dosya)
                        .map_err(|e| (1, format!("{} okunamadı: {e}.", dosya.display())))?;
                    let j = serde_json::to_string(&oz.imzala(&b)).unwrap_or_default();
                    match &s.cikti {
                        Some(c) => yeni_yaz(c, &j)?,
                        None => println!("{j}"),
                    }
                }
                tur => {
                    let mut v = json_oku(dosya)?;
                    if tur == "paket" {
                        paket_imzala(&mut v, &oz).map_err(h)?
                    } else {
                        yayin_imzala(&mut v, &oz).map_err(h)?
                    }
                    belge_yaz(dosya, &v)?;
                    println!(
                        "İmzalandı: {} (anahtar kimliği {}).",
                        dosya.display(),
                        oz.genel().kimlik()
                    );
                }
            }
            Ok(())
        }
        "dogrula" => {
            if s.konumsal.len() != 1 {
                return Err(kullanim("dogrula tam bir dosya ister."));
            }
            let dosya = Path::new(&s.konumsal[0]);
            let g = GenelAnahtar::dosyadan(gerekli(&s.genel, "--genel")?).map_err(h)?;
            let (veri, im) = match s.tur.as_str() {
                "ham" => {
                    let b = std::fs::read(dosya)
                        .map_err(|e| (1, format!("{} okunamadı: {e}.", dosya.display())))?;
                    let iv = json_oku(gerekli(&s.imza, "--imza")?)?;
                    let im: Imza = serde_json::from_value(iv).map_err(|_| {
                        (
                            2,
                            "İmza dosyası {alg, anahtar_kimlik, deger} biçiminde değil."
                                .to_string(),
                        )
                    })?;
                    (b, im)
                }
                "paket" => {
                    let v = json_oku(dosya)?;
                    let o = v
                        .get("icerik_sha256")
                        .and_then(|x| x.as_str())
                        .ok_or((2, "paket.json içinde icerik_sha256 yok.".to_string()))?
                        .as_bytes()
                        .to_vec();
                    (o, imza_oku(&v).map_err(h)?)
                }
                _ => {
                    let v = json_oku(dosya)?;
                    (kanonik_bayt(&v), imza_oku(&v).map_err(h)?)
                }
            };
            if im.anahtar_kimlik != g.kimlik() {
                return Err((
                    2,
                    "İmza başka bir anahtarla atılmış (anahtar kimliği uyuşmuyor).".into(),
                ));
            }
            g.dogrula(&veri, &im).map_err(h)?;
            println!("İmza geçerli (anahtar kimliği {}).", g.kimlik());
            Ok(())
        }
        d => Err(kullanim(&format!("Bilinmeyen komut: {d}."))),
    }
}

fn yeni_yaz(yol: &Path, icerik: &str) -> Sonuc {
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(yol)
        .map_err(|e| {
            (
                1,
                format!(
                    "{} yazılamadı (var olan dosya ezilmez): {e}.",
                    yol.display()
                ),
            )
        })?;
    f.write_all(icerik.as_bytes())
        .map_err(|e| (1, format!("{} yazılamadı: {e}.", yol.display())))
}

/// Belgeyi yerinde günceller; eski içerik silinmez, `<dosya>.once-<n>` olarak yan yana kopyalanır.
fn belge_yaz(yol: &Path, v: &serde_json::Value) -> Sonuc {
    let eski = std::fs::read(yol).map_err(|e| (1, format!("{} okunamadı: {e}.", yol.display())))?;
    for n in 1..1000 {
        let yedek = PathBuf::from(format!("{}.once-{n}", yol.display()));
        if !yedek.exists() {
            yeni_yaz(&yedek, &String::from_utf8_lossy(&eski))?;
            break;
        }
    }
    let b = serde_json::to_vec_pretty(v).map_err(|e| (1, format!("JSON yazılamadı: {e}.")))?;
    taa_dosya::uzerine_yaz_atomik(yol, &b)
        .map_err(|e| (1, format!("{} yazılamadı: {e}.", yol.display())))
}
