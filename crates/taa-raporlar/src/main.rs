// Sahip: Yusuf. yazan: [Claude#taa-raporlar-T8 2026-11-06]
//! `taa-raporlar` komut satırı: yönetici raporu + yorum. Yalnız std::env::args; clap yok.
//! `--sunucu` verilirse taa-kimlik deposunda o sunucu için **yönetici** oturumu şart (yoksa çıkış 6).
//! `--sunucu` verilmezse yerel kip (sunucu kurulana kadar): yerel veri kökünden okur.
//! Çıkış kodları: 0 tamam, 2 kullanım/doğrulama, 5 oturum yok, 6 yönetici değil, 1 diğer.

use std::path::PathBuf;
use std::process::ExitCode;
use taa_rapor::{tarih, Kaynaklar};
use taa_raporlar::{
    karsilastir, onceki_rapor, yonetici_mi, yorum_json, yorum_metni, yorumla, Esikler,
};

const YARDIM: &str = "\
taa-raporlar — TAA Raporlar: yöneticiye özel tek yerden rapor ve kural tabanlı yorum (internet/AI gerekmez).
Kullanım: taa-raporlar [--kok <veri kökü>] [--fis-klasoru <klasör>] [--bugun YYYY-AA-GG] [--sunucu <https://adres>]
                       [--json] [--kaydet] [--esikler]
  --kok          Veri kökü; verilmezse TAA_VERI_KOK, yoksa %APPDATA%\\TAA\\veri.
  --fis-klasoru  taa-fis-sunucu --veri klasörü (fisler.jsonl, bildirimler.jsonl).
  --bugun        Geciken/bugün hesabı için tarih; verilmezse bugün (UTC).
  --sunucu       Merkez sunucu adresi: taa-kimlik'te bu sunucu için YÖNETİCİ oturumu aranır; yoksa rapor üretilmez.
                 Verilmezse yerel kip (sunucu kurulana kadar).
  --json         Makine okunur çıktı (rapor + yorum).   --kaydet  <kök>\\raporlar\\rapor-<damga>.json ve .txt yazar (ezmez).
  --esikler      Yürürlükteki eşikleri gösterir (<kök>\\raporlar\\esikler.json ile değiştirilir).
Önceki kayıtlı raporla (rapor-*.json) karşılaştırma yorumları otomatik eklenir.";

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
    let mut sunucu: Option<String> = None;
    let mut json = false;
    let mut kaydet = false;
    let mut esikleri_goster = false;
    let mut i = 0;
    let deger = |i: &mut usize, ad: &str| -> Result<String, (u8, String)> {
        *i += 1;
        args.get(*i).cloned().ok_or_else(|| {
            (
                2,
                format!("{ad} için değer gerekli. Öneri: taa-raporlar --yardim"),
            )
        })
    };
    while i < args.len() {
        match args[i].as_str() {
            "--kok" => kok = Some(PathBuf::from(deger(&mut i, "--kok")?)),
            "--fis-klasoru" => fis_klasoru = Some(PathBuf::from(deger(&mut i, "--fis-klasoru")?)),
            "--bugun" => bugun = Some(deger(&mut i, "--bugun")?),
            "--sunucu" => sunucu = Some(deger(&mut i, "--sunucu")?),
            "--json" => json = true,
            "--kaydet" => kaydet = true,
            "--esikler" => esikleri_goster = true,
            a => {
                return Err((
                    2,
                    format!("bilinmeyen seçenek: {a}. Öneri: taa-raporlar --yardim"),
                ))
            }
        }
        i += 1;
    }
    let kok = kok
        .or_else(veri_kok_ortamdan)
        .ok_or_else(|| (2, "veri kökü bulunamadı. Öneri: --kok verin.".to_string()))?;
    let esikler = Esikler::kokten(&kok).map_err(|m| (2, m))?;
    if esikleri_goster {
        println!("{}", esikler.json());
        return Ok(());
    }
    // Yönetici denetimi (sunucu verilmişse zorunlu).
    if let Some(s) = &sunucu {
        let depo = taa_kimlik::varsayilan_depo(&kok);
        let o = depo.oku(s).map_err(|e| {
            (
                5,
                format!("{e}. Rapor yalnız yönetici oturumuyla üretilir: taa-kimlik giris --sunucu {s} --kullanici <yönetici>"),
            )
        })?;
        if !yonetici_mi(&o) {
            return Err((
                6,
                format!(
                    "{} kullanıcısı yönetici değil (roller: {}). Raporlar yalnız yöneticiye açıktır.",
                    o.kullanici,
                    o.roller.join(",")
                ),
            ));
        }
    }
    let bugun = match bugun {
        Some(b) => {
            if !tarih::tarih_gecerli(&b) {
                return Err((2, format!("--bugun YYYY-AA-GG olmalı: {b}")));
            }
            b
        }
        None => tarih::bugun(),
    };
    let mut kaynaklar = Kaynaklar::veri_kokunden(&kok);
    if let Some(f) = &fis_klasoru {
        kaynaklar.fis_klasoru_ekle(f);
    }
    let rapor = taa_rapor::uret(&kaynaklar, &bugun).map_err(|m| (1, m))?;
    let mut yorumlar = yorumla(&rapor, &esikler);
    if let Some(onceki) = onceki_rapor(&kok) {
        yorumlar.extend(karsilastir(&rapor, &onceki));
    }
    if kaydet {
        let sn = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let kalan = sn.rem_euclid(86_400);
        let damga = format!(
            "{}-{:02}{:02}{:02}",
            tarih::tarih_metni(sn).replace('-', ""),
            kalan / 3600,
            (kalan % 3600) / 60,
            kalan % 60
        );
        let klasor = kok.join("raporlar");
        let j = klasor.join(format!("rapor-{damga}.json"));
        let t = klasor.join(format!("rapor-{damga}.txt"));
        let govde = serde_json::json!({"rapor": rapor.json(), "yorum": yorum_json(&yorumlar)});
        let jy = taa_dosya::atomik_yaz(&j, govde.to_string().as_bytes())
            .map_err(|e| (1, format!("{} yazılamadı: {e}", j.display())))?;
        let ty = taa_dosya::atomik_yaz(
            &t,
            format!("{}\n{}", rapor.metin(), yorum_metni(&yorumlar)).as_bytes(),
        )
        .map_err(|e| (1, format!("{} yazılamadı: {e}", t.display())))?;
        if !json {
            println!("kaydedildi: {} ve {}", jy.display(), ty.display());
        }
    }
    if json {
        println!(
            "{}",
            serde_json::json!({"rapor": rapor.json(), "yorum": yorum_json(&yorumlar), "yerel_kip": sunucu.is_none()})
        );
    } else {
        print!("{}", rapor.metin());
        println!();
        print!("{}", yorum_metni(&yorumlar));
        if sunucu.is_none() {
            println!("(yerel kip: sunucu verilmedi, yönetici denetimi yapılmadı)");
        }
    }
    Ok(())
}
