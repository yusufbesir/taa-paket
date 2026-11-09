// Sahip: Yusuf. yazan: [Claude#taa-kimlik-T7 2026-11-06]
//! `taa-kimlik` komut satırı. Yalnız std::env::args; clap yok.
//! Parola komut satırına yazılmaz: standart girdiden tek satır okunur. Oturum anahtarı hiç basılmaz.
//! Çıkış kodları: 0 tamam, 2 kullanım/doğrulama, 3 sunucuya ulaşılamadı, 4 giriş reddedildi, 5 oturum/yetki yok, 7 yetkisiz, 1 diğer.

use std::io::{BufRead, Read};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use taa_kimlik::{
    cihaz_imzalayici, giris_dogrulayarak, kuyrugu_gonder_hedef, sunucu_anahtari_oku,
    varsayilan_depo, yetki_al, yetki_oku, CurlGonderici, KimlikHata,
};

const YARDIM: &str = "\
taa-kimlik — merkez sunucuya giriş ve oturum saklama (Windows Kimlik Bilgisi Yöneticisi).
Kullanım:
  taa-kimlik giris  --sunucu <https://adres> --kullanici <ad> [--kod <kod>] [--sertifika <pem>] [--json]   (parola stdin'den)
  taa-kimlik durum  --sunucu <https://adres> [--json]
  taa-kimlik cikis  --sunucu <https://adres>
  taa-kimlik gonder --sunucu <https://adres> [--kok <veri kökü>] [--sertifika <pem>] [--en-cok N] [--json]
  --sunucu yoksa <kök>\\yol\\sunucu-yollari.json'daki yollar sırayla denenir (taa-yol); oturum anahtarı listenin
  'sunucu' adıdır; yol değişse de oturum korunur. Çevrimdışı komutlar (durum, yetki, cikis) ağa çıkmaz.
  taa-kimlik yetki  --sunucu <https://adres> [--program <ad>] [--izin <program.islem>] [--kok] [--json]
               Saklı yetki belgesine göre (internet gerekmez): çıkış 0 izinli, 7 yetkisiz, 5 belge yok/süresi geçmiş.
  Sunucu genel anahtarı <kök>\\anahtarlar\\sunucu.pub varsa giriş yanıtı ve yetki belgesi imzası doğrulanır.
  --sunucu     Verilmezse TAA_SUNUCU ortam değişkeni.   --kok  Verilmezse TAA_VERI_KOK, yoksa %APPDATA%\\TAA\\veri.
  giris: parola standart girdiden okunur (örnek: echo parola | taa-kimlik giris ...  ya da yazıp Enter).
  gonder: depodaki oturumla <kök>\\kuyruk\\* altındaki tüm program kuyruklarını sunucuya yollar (taa-kuyruk).
  Cihaz anahtarı (<kök>\\cihaz\\cihaz.key, taa-cihaz uret) varsa her istek onunla imzalanır (X-TAA-* başlıkları).
Parola ve oturum anahtarı hiçbir dosyaya, depoya ya da ekrana yazılmaz.";

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

fn hata_kodu(e: &KimlikHata) -> u8 {
    match e {
        KimlikHata::GecersizGirdi(_) => 2,
        KimlikHata::GirisReddi { .. } => 4,
        KimlikHata::Baglanti(_) => 3,
        KimlikHata::OturumYok | KimlikHata::YetkiYok(_) => 5,
        KimlikHata::BozukYanit(_) | KimlikHata::Depo(_) => 1,
    }
}

fn calistir(args: &[String]) -> Result<u8, (u8, String)> {
    let mut sunucu: Option<String> = None;
    let mut kullanici: Option<String> = None;
    let mut program: Option<String> = None;
    let mut izin: Option<String> = None;
    let mut kod: Option<String> = None;
    let mut sertifika: Option<PathBuf> = None;
    let mut kok: Option<PathBuf> = None;
    let mut en_cok = 0usize;
    let mut json = false;
    let mut komut: Option<String> = None;
    let mut i = 0;
    let deger = |i: &mut usize, ad: &str| -> Result<String, (u8, String)> {
        *i += 1;
        args.get(*i).cloned().ok_or_else(|| {
            (
                2,
                format!("{ad} için değer gerekli. Öneri: taa-kimlik --yardim"),
            )
        })
    };
    while i < args.len() {
        match args[i].as_str() {
            "--sunucu" => sunucu = Some(deger(&mut i, "--sunucu")?),
            "--kullanici" => kullanici = Some(deger(&mut i, "--kullanici")?),
            "--kod" => kod = Some(deger(&mut i, "--kod")?),
            "--program" => program = Some(deger(&mut i, "--program")?),
            "--izin" => izin = Some(deger(&mut i, "--izin")?),
            "--sertifika" => sertifika = Some(PathBuf::from(deger(&mut i, "--sertifika")?)),
            "--kok" => kok = Some(PathBuf::from(deger(&mut i, "--kok")?)),
            "--en-cok" => {
                let d = deger(&mut i, "--en-cok")?;
                en_cok = d
                    .parse()
                    .map_err(|_| (2, format!("--en-cok sayı olmalı: {d}")))?;
            }
            "--json" => json = true,
            "--parola" => {
                return Err((
                    2,
                    "parola komut satırında verilmez; standart girdiden okunur.".into(),
                ))
            }
            a if a.starts_with("--") => {
                return Err((
                    2,
                    format!("bilinmeyen seçenek: {a}. Öneri: taa-kimlik --yardim"),
                ))
            }
            a if komut.is_none() => komut = Some(a.to_string()),
            a => return Err((2, format!("fazla argüman: {a}. Öneri: taa-kimlik --yardim"))),
        }
        i += 1;
    }
    let sunucu_acik = sunucu.is_some()
        || std::env::var("TAA_SUNUCU")
            .ok()
            .is_some_and(|s| !s.trim().is_empty());
    let kok = kok
        .or_else(veri_kok_ortamdan)
        .ok_or_else(|| (2, "veri kökü bulunamadı. Öneri: --kok verin.".to_string()))?;
    let yh = |e: taa_yol::YolHata| (taa_yol::hata_kodu(&e), e.to_string());
    // Çevrimdışı komutlar (durum, yetki, cikis) yalnız depo anahtarını ister; ağa çıkılmaz.
    // Ağ komutları (giris, gonder) yol listesinden ilk ulaşılan adresi seçer (taa-yol, S4a).
    let sunucu = taa_yol::anahtar_coz_ortam(&kok, sunucu.as_deref()).map_err(yh)?;
    let hedef = |sertifika: Option<&Path>| -> Result<taa_yol::Hedef, (u8, String)> {
        let h = taa_yol::hedef_coz_curl(
            &kok,
            sunucu_acik.then_some(&sunucu).map(String::as_str),
            sertifika,
        )
        .map_err(yh)?;
        if let Some(y) = &h.yol {
            eprintln!("yol: {y} → {}", h.taban);
        }
        Ok(h)
    };
    let depo = varsayilan_depo(&kok);
    let kh = |e: KimlikHata| (hata_kodu(&e), e.to_string());
    match komut.as_deref() {
        Some("giris") => {
            let k = kullanici.ok_or_else(|| (2, "giris için --kullanici gerekli.".to_string()))?;
            let mut parola = String::new();
            let stdin = std::io::stdin();
            if stdin.lock().read_line(&mut parola).is_err() || parola.trim().is_empty() {
                return Err((2, "parola standart girdiden okunamadı (boş).".into()));
            }
            // Fazlası okunup atılır; parola bellekte yalnız istek süresince kalır.
            let _ = stdin.lock().read_to_end(&mut Vec::new());
            let parola = parola.trim_end_matches(['\n', '\r']);
            let h = hedef(sertifika.as_deref())?;
            // Yol listesi varsa giriş isteği de yedekli gönderilir (bir yol düşerse sıradakine geçer).
            let yedekli = if sunucu_acik {
                None
            } else {
                taa_kimlik::yedekli_gonderici(&kok, None, sertifika.as_deref()).map_err(kh)?
            };
            let mut g = CurlGonderici::yeni(&h.taban).map_err(|e| (2, e.to_string()))?;
            if let Some(c) = &h.sertifika {
                g = g.sertifika(c);
            }
            // Cihaz anahtarı varsa istek imzalanır (sunucu imzasız girişi reddeder; taa-cihaz uret).
            match cihaz_imzalayici(&kok).map_err(kh)? {
                Some(i) => g = g.imzalayici(i),
                None => eprintln!(
                    "uyarı: cihaz anahtarı yok ({}); sunucu imzasız isteği reddedebilir. Öneri: taa-cihaz uret",
                    kok.join("cihaz").display()
                ),
            }
            let anahtar = sunucu_anahtari_oku(&kok).map_err(kh)?;
            if anahtar.is_none() {
                eprintln!(
                    "uyarı: sunucu genel anahtarı yok ({}); sunucu yanıtının imzası doğrulanmıyor.",
                    kok.join("anahtarlar").join("sunucu.pub").display()
                );
            }
            let gonderici: &dyn taa_kimlik::Gonderici = match &yedekli {
                Some(y) => y,
                None => &g,
            };
            let o = giris_dogrulayarak(
                gonderici,
                &sunucu,
                &k,
                parola,
                kod.as_deref(),
                anahtar.as_ref(),
            )
            .map_err(kh)?;
            depo.kaydet(&o).map_err(kh)?;
            // Çevrimdışı yetki belgesi (POST /v1/yetki, imzalı, oturumsuz). Başarısızlık girişi bozmaz.
            let yetki_sonucu = if sunucu_acik {
                o.gonderici_taban(
                    &h.taban,
                    h.sertifika.as_deref(),
                    cihaz_imzalayici(&kok).ok().flatten(),
                )
                .and_then(|g| yetki_al(&g, &kok, &sunucu, anahtar.as_ref()))
            } else {
                match taa_kimlik::yedekli_gonderici(&kok, Some(&o), sertifika.as_deref()) {
                    Ok(Some(y)) => yetki_al(&y, &kok, &sunucu, anahtar.as_ref()),
                    _ => o
                        .gonderici_taban(
                            &h.taban,
                            h.sertifika.as_deref(),
                            cihaz_imzalayici(&kok).ok().flatten(),
                        )
                        .and_then(|g| yetki_al(&g, &kok, &sunucu, anahtar.as_ref())),
                }
            };
            match yetki_sonucu {
                Ok(y) if !json => println!(
                    "yetki belgesi alındı: {} izin, geçerlilik {}{}",
                    y.izinler.len(),
                    y.gecerlilik,
                    if y.dogrulandi {
                        ""
                    } else {
                        " (imza doğrulanmadı)"
                    }
                ),
                Ok(_) => {}
                Err(e) => eprintln!("uyarı: yetki belgesi alınamadı: {e}"),
            }
            if json {
                println!("{}", o.ozet());
            } else {
                println!(
                    "giriş tamam: {} @ {} (şube {}, roller {})",
                    o.kullanici,
                    o.sunucu,
                    o.sube_id,
                    o.roller.join(",")
                );
            }
            Ok(0)
        }
        Some("durum") => {
            let o = depo.oku(&sunucu).map_err(kh)?;
            if json {
                println!("{}", o.ozet());
            } else {
                println!(
                    "oturum var: {} @ {} (şube {}, roller {}, giriş {})",
                    o.kullanici,
                    o.sunucu,
                    o.sube_id,
                    o.roller.join(","),
                    o.zaman
                );
            }
            Ok(0)
        }
        Some("yetki") => {
            let anahtar = sunucu_anahtari_oku(&kok).map_err(kh)?;
            let y = yetki_oku(&kok, &sunucu, anahtar.as_ref(), None).map_err(kh)?;
            let mut red = Vec::new();
            if let Some(p) = &program {
                if !y.program_acik(p) {
                    red.push(format!("program kapalı: {p}"));
                }
            }
            if let Some(i) = &izin {
                if !y.izinli(i) {
                    red.push(format!("izin yok: {i}"));
                }
            }
            if json {
                let mut v = y.ozet();
                v["sonuc"] = serde_json::json!(if red.is_empty() { "izinli" } else { "yetkisiz" });
                v["neden"] = serde_json::json!(red);
                println!("{v}");
            } else if red.is_empty() {
                println!(
                    "izinli: {} (roller {}, {} izin, geçerlilik {}{})",
                    y.kullanici,
                    y.roller.join(","),
                    y.izinler.len(),
                    y.gecerlilik,
                    if y.dogrulandi {
                        ""
                    } else {
                        ", imza doğrulanmadı"
                    }
                );
            } else {
                println!("yetkisiz: {}", red.join("; "));
            }
            Ok(if red.is_empty() { 0 } else { 7 })
        }
        Some("cikis") => {
            depo.cikis(&sunucu).map_err(kh)?;
            if json {
                println!("{}", serde_json::json!({"durum": "cikis"}));
            } else {
                println!("çıkış yapıldı: {sunucu}");
            }
            Ok(0)
        }
        Some("gonder") => {
            let h = hedef(sertifika.as_deref())?;
            let yedekli = if sunucu_acik {
                None
            } else {
                let o = depo.oku(&sunucu).map_err(kh)?;
                taa_kimlik::yedekli_gonderici(&kok, Some(&o), sertifika.as_deref()).map_err(kh)?
            };
            let sonuc = match &yedekli {
                // Yol listesi: her kayıt yol düşse de sıradaki yoldan gider; aktif yol `son.json`a yazılır.
                Some(y) => taa_kimlik::kuyrugu_gonder_ile(y, &kok, en_cok).map_err(kh)?,
                None => kuyrugu_gonder_hedef(
                    depo.as_ref(),
                    &sunucu,
                    &h.taban,
                    &kok,
                    h.sertifika.as_deref(),
                    en_cok,
                )
                .map_err(kh)?,
            };
            let mut toplam = (0usize, 0usize, 0usize);
            let mut durma = None;
            let mut programlar = Vec::new();
            for (p, o) in &sonuc {
                toplam.0 += o.gonderilen;
                toplam.1 += o.reddedilen;
                toplam.2 += o.kalan;
                if let Some(n) = &o.durma_nedeni {
                    durma = Some(n.clone());
                }
                programlar.push(serde_json::json!({"program": p, "gonderilen": o.gonderilen,
                    "reddedilen": o.reddedilen, "kalan": o.kalan, "durma_nedeni": o.durma_nedeni}));
            }
            if json {
                println!(
                    "{}",
                    serde_json::json!({"gonderilen": toplam.0, "reddedilen": toplam.1, "kalan": toplam.2,
                        "durma_nedeni": durma, "programlar": programlar})
                );
            } else {
                println!(
                    "gönderilen {} · reddedilen {} · kalan {}",
                    toplam.0, toplam.1, toplam.2
                );
                if let Some(n) = &durma {
                    println!("durdu: {n}");
                }
            }
            Ok(if durma.is_some() { 3 } else { 0 })
        }
        Some(k) => Err((
            2,
            format!("bilinmeyen komut: {k}. Öneri: taa-kimlik --yardim"),
        )),
        None => Err((2, "komut gerekli: giris | durum | cikis | gonder".into())),
    }
}
