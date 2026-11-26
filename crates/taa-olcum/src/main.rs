//! `taa-olcum` komut satırı. Yalnız std::env::args; clap yok. Sahip: Yusuf. yazan: [Claude#olcum 2026-11-26]
//! Çıkış kodları: 0 tamam, 2 kullanım, 1 diğer hata.

use std::io::Write;
use std::process::ExitCode;
use taa_olcum::envanter;
use taa_olcum::olcum::{olc, Ayar, OlcumRapor};
use taa_olcum::rapor;
use taa_olcum::yontem;

const KULLANIM: &str = "Kullanım: taa-olcum <envanter|olcum|karsilastir|yontem|surum> [seçenekler]. `taa-olcum yardim` açıklar.";

const YARDIM: &str = "\
taa-olcum: DG1a ölçüm aracı (sentetik iş; OCR sonra eklenir).
  envanter [--json]                 Donanım ve sistem envanteri (rastgele işçi no; kişi/bilgisayar adı/seri yok).
  olcum [--sure-ms N] [--pay 5,8,10,12,15] [--cikti dosya.json] [--json]
                                    Pay basamaklarında ayrı ayrı ölçer; okunur özet yazar. --cikti JSON dosyası yazar
                                    (var olan dosyanın üzerine YAZMAZ). --json yalnız JSON'u stdout'a verir.
  karsilastir <a.json> <b.json>     İki koşunun pay başına hız farkı.
  yontem                            Kilitli yöntem değerleri ve belge özeti.
  surum                             Araç sürümü.
Ölçüm sırasında bilgisayarı başka ağır işle yormayın (aynı anda tek ağır iş kuralı).";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match calistir(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err((kod, mesaj)) => {
            eprintln!("{mesaj}");
            ExitCode::from(kod)
        }
    }
}

fn deger<'a>(args: &'a [String], ad: &str) -> Result<Option<&'a str>, (u8, String)> {
    match args.iter().position(|a| a == ad) {
        None => Ok(None),
        Some(i) => args
            .get(i + 1)
            .map(|s| Some(s.as_str()))
            .ok_or((2, format!("{ad} için değer yok. {KULLANIM}"))),
    }
}

fn calistir(args: &[String]) -> Result<(), (u8, String)> {
    let json = args.iter().any(|a| a == "--json");
    match args.first().map(String::as_str) {
        Some("surum") => {
            println!("taa-olcum {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Some("yardim") | Some("--yardim") | Some("-h") => {
            println!("{YARDIM}");
            Ok(())
        }
        Some("envanter") => {
            let e = envanter::topla();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&e).map_err(|x| (1, x.to_string()))?
                );
            } else {
                println!("İşçi no: {}", e.isci_no);
                println!(
                    "İşlemci: {} ({} mantıksal)",
                    e.islemci.model, e.islemci.mantiksal
                );
                println!(
                    "Sistem: {}; güç: {}; disk: {}",
                    e.sistem.surum, e.guc, e.disk.tur
                );
                println!("Toplanmayan: {}", e.toplanmadi.join(", "));
            }
            Ok(())
        }
        Some("yontem") => {
            println!("Belge: docs/olcum/YONTEM.md");
            println!("SHA-256: {}", yontem::yontem_ozeti());
            println!(
                "Kilit: {}",
                if yontem::kilit_tutuyor() {
                    "tutuyor"
                } else {
                    "TUTMUYOR (belge değişmiş)"
                }
            );
            println!(
                "Paylar: {:?}; ısınma {}, geçerli tekrar {}",
                yontem::PAY_BASAMAKLARI,
                yontem::ISINMA_TEKRAR,
                yontem::OLCUM_TEKRAR
            );
            println!(
                "Ortanca artış en çok %{}; tek işlem ek gecikme en çok {} ms",
                yontem::ORTANCA_ARTIS_EN_COK_YUZDE,
                yontem::TEK_ISLEM_EK_GECIKME_MS
            );
            Ok(())
        }
        Some("olcum") => {
            let mut ayar = Ayar::default();
            if let Some(s) = deger(args, "--sure-ms")? {
                ayar.sure_ms = s
                    .parse()
                    .map_err(|_| (2, "--sure-ms sayı olmalı".to_string()))?;
                if ayar.sure_ms < 20 {
                    return Err((2, "--sure-ms en az 20 olmalı".to_string()));
                }
            }
            if let Some(s) = deger(args, "--pay")? {
                let mut p = Vec::new();
                for x in s.split(',') {
                    let n: u32 = x
                        .trim()
                        .parse()
                        .map_err(|_| (2, "--pay virgülle ayrılmış sayılar olmalı".to_string()))?;
                    if !yontem::PAY_BASAMAKLARI.contains(&n) {
                        return Err((
                            2,
                            format!(
                                "pay %{n} kilitli basamaklarda yok ({:?})",
                                yontem::PAY_BASAMAKLARI
                            ),
                        ));
                    }
                    p.push(n);
                }
                ayar.paylar = p;
            }
            let cikti = deger(args, "--cikti")?;
            if let Some(yol) = cikti {
                if std::path::Path::new(yol).exists() {
                    return Err((
                        1,
                        format!("{yol} zaten var; üzerine yazılmaz, başka ad verin"),
                    ));
                }
            }
            let r = olc(&ayar, &mut |m| eprintln!("{m}"));
            let metin = serde_json::to_string_pretty(&r).map_err(|x| (1, x.to_string()))?;
            if let Some(yol) = cikti {
                let mut f = std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(yol)
                    .map_err(|x| (1, format!("{yol} yazılamadı: {x}")))?;
                f.write_all(metin.as_bytes())
                    .map_err(|x| (1, x.to_string()))?;
                eprintln!("JSON yazıldı: {yol}");
            }
            if json {
                println!("{metin}");
            } else {
                print!("{}", rapor::ozet(&r));
            }
            Ok(())
        }
        Some("karsilastir") => {
            let (Some(a), Some(b)) = (args.get(1), args.get(2)) else {
                return Err((2, format!("iki JSON dosyası gerekli. {KULLANIM}")));
            };
            let oku = |y: &str| -> Result<OlcumRapor, (u8, String)> {
                let m =
                    std::fs::read_to_string(y).map_err(|x| (1, format!("{y} okunamadı: {x}")))?;
                serde_json::from_str(&m).map_err(|x| (1, format!("{y} geçerli rapor değil: {x}")))
            };
            print!("{}", rapor::karsilastirma_metni(&oku(a)?, &oku(b)?));
            Ok(())
        }
        _ => Err((2, KULLANIM.to_string())),
    }
}
