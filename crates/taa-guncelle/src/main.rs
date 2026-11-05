// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! `taa-guncelle kontrol|uygula [--json] [--kaynak <klasor|url>] [--kanal <ad>] [--paket <ad>]` (docs/T3-TASARIM.md §3).
//! Kaynak: `--kaynak`; yoksa `<kök>/kur.toml` içindeki `kaynak_url`, o da yoksa `kaynak_klasoru`.
//! `http://`/`https://` ile başlayan değer HTTP kaynağıdır (curl ile indirir). Kök: TAA_KOK (taa-kur ile aynı).
//! `--paket <ad>`: yalnız o paket (pencere "Güncelle" düğmesi).
//! Çıkış kodları taa-kur ile aynı: 0 tamam, 2 doğrulama/kullanım, 3 çözüm, 4 yok, 5 bozuk, 6 kilitli, 1 diğer;
//! `uygula`da en az bir paket hatalıysa 1.

use std::process::ExitCode;
use taa_guncelle::{
    http_url_mu, kontrol_paket, uygula_paket, HttpKaynak, Kaynak, KontrolSonuc, YerelKaynak,
};
use taa_kur::{kur_ayari_oku, Kur, KurHata};

const KULLANIM: &str = "Kullanım: taa-guncelle <kontrol|uygula> [--json] [--kaynak <klasor|url>] [--kanal <ad>] [--paket <ad>]. Öneri: ayrıntı için docs/T3-TASARIM.md.";

fn main() -> ExitCode {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let json = args.iter().any(|a| a == "--json");
    args.retain(|a| a != "--json");
    match calistir(&args, json) {
        Ok(kod) => ExitCode::from(kod),
        Err(h) => {
            let m = h.mesaj().split_whitespace().collect::<Vec<_>>().join(" ");
            if json {
                eprintln!("{}", serde_json::json!({"hata": m, "kod": h.cikis_kodu()}));
            } else {
                eprintln!("{m}");
            }
            ExitCode::from(h.cikis_kodu() as u8)
        }
    }
}

fn kullanim(m: &str) -> KurHata {
    KurHata::Dogrulama(format!("{m} {KULLANIM}"))
}

fn kontrol_yaz(k: &KontrolSonuc) {
    println!("Kanal: {} (imza: {})", k.kanal, k.imza);
    for u in &k.uyarilar {
        println!("Uyarı: {u}");
    }
    for g in &k.guncellenebilir {
        println!(
            "Güncellenebilir: {} {} -> {}{}{} ({} bayt)",
            g.ad,
            g.kurulu,
            g.yeni,
            if g.zorunlu { " [ZORUNLU]" } else { "" },
            if g.sabitli { " [sabitli]" } else { "" },
            g.indirme_boyutu
        );
    }
    for r in &k.reddedilen {
        println!(
            "Reddedildi: {} (kurulu {}, yayında {}): {}",
            r.ad, r.kurulu, r.yayinda, r.neden
        );
    }
    if k.guncellenebilir.is_empty() {
        println!("Güncellenecek paket yok.");
    }
}

fn calistir(args: &[String], json: bool) -> Result<u8, KurHata> {
    let Some(komut) = args.first() else {
        return Err(kullanim("Komut verilmedi."));
    };
    if !matches!(komut.as_str(), "kontrol" | "uygula") {
        return Err(kullanim(&format!("Bilinmeyen komut: {komut}.")));
    }
    let (mut kaynak_arg, mut kanal_arg, mut paket_arg) = (None, None, None);
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--kaynak" | "--kanal" | "--paket" => {
                let ad = args[i].clone();
                i += 1;
                let Some(d) = args.get(i) else {
                    return Err(kullanim(&format!("{ad} için değer eksik.")));
                };
                match ad.as_str() {
                    "--kaynak" => kaynak_arg = Some(d.clone()),
                    "--kanal" => kanal_arg = Some(d.clone()),
                    _ => paket_arg = Some(d.clone()),
                }
            }
            x => return Err(kullanim(&format!("Bilinmeyen bağımsız değişken: {x}."))),
        }
        i += 1;
    }
    let kur = Kur::ortamdan()?;
    let ayar = kur_ayari_oku(kur.kok())?;
    let kanal = kanal_arg.unwrap_or_else(|| ayar.kanal().to_string());
    let deger = kaynak_arg
        .or(ayar.kaynak_url.clone())
        .or(ayar.kaynak_klasoru.clone())
        .ok_or_else(|| {
            kullanim("Güncelleme kaynağı belirtilmedi: --kaynak verin ya da kur.toml'a kaynak_url / kaynak_klasoru yazın.")
        })?;
    let kaynak: Box<dyn Kaynak> = if http_url_mu(&deger) {
        Box::new(HttpKaynak::yeni(&deger)?)
    } else {
        Box::new(YerelKaynak::yeni(deger))
    };
    let paket = paket_arg.as_deref();
    if komut == "kontrol" {
        let k = kontrol_paket(&kur, kaynak.as_ref(), &kanal, paket)?;
        if json {
            println!("{}", serde_json::to_value(&k).unwrap_or_default());
        } else {
            kontrol_yaz(&k);
        }
        return Ok(0);
    }
    let s = uygula_paket(&kur, kaynak.as_ref(), &kanal, paket)?;
    if json {
        println!("{}", serde_json::to_value(&s).unwrap_or_default());
    } else {
        kontrol_yaz(&s.kontrol);
        for r in &s.satirlar {
            println!(
                "{}: {} {} -> {}: {}{}",
                r.durum,
                r.ad,
                r.onceki,
                r.yeni,
                r.mesaj,
                if r.geri_donuldu {
                    " (önceki sürüm seçili)"
                } else {
                    ""
                }
            );
        }
    }
    Ok(if s.hata_var() { 1 } else { 0 })
}
