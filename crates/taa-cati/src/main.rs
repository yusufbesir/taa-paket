// Sahip: Yusuf. yazan: [Claude#taa-cati-S3b 2026-11-06]
//! `taa-cati` komut satırı (TAA Çatı v0). Yalnız std::env::args; clap yok. Parola standart girdiden okunur.
//! Çıkış kodları: 0 tamam, 2 kullanım/doğrulama, 3 ulaşılamadı, 4 sunucu reddetti, 5 oturum/cihaz yok,
//! 6 yeniden doğrulama gerekli (taa-cati dogrula), 7 yetkisiz, 1 diğer.

use serde_json::{json, Value};
use std::io::{BufRead, Read};
use std::path::PathBuf;
use std::process::ExitCode;
use taa_cati::{hata_kodu, yonetici_baglantisi_hedef, Cati, CatiHata, OlaySuzgeci, RolAtama};

const YARDIM: &str = "\
taa-cati — TAA Çatı: merkez sunucunun yönetimi (yalnız sunucu PC'sinde; yönetici oturumu + cihaz imzası gerekir).
Önce: taa-cihaz uret · sunucuda: tutaa-sunucu merkez-cihaz · sonra: taa-kimlik giris --kullanici <yönetici>
Kullanım (ortak seçenekler: --sunucu <https://adres> | TAA_SUNUCU | yol listesi (taa-yol), --kok <veri kökü> | TAA_VERI_KOK, --sertifika <pem>, --json):
  taa-cati dogrula                                  parola stdin'den; yazma komutları için 15 dk pencere açar
  taa-cati kullanici liste
  taa-cati kullanici ekle --ad <ad> --gorunen-ad <Ad Soyad> --rol <rol>[ --sube <id>] [--rol ... ]   (parola stdin'den)
  taa-cati kullanici rol        --id <no> --rol <rol> [--sube <id>]
  taa-cati kullanici rol-kaldir --id <no> --rol <rol> [--sube <id>]
  taa-cati kullanici kapat      --id <no>                           (silme yok)
  taa-cati kullanici parola     --id <no>                           (yeni parola stdin'den)
  taa-cati rol liste | izin liste
  taa-cati cihaz liste [--durum bekliyor|onayli|iptal]
  taa-cati cihaz onay  --kimlik <16 hex> [--sube <id>]              (--sube yoksa merkez cihazı)
  taa-cati cihaz iptal --kimlik <16 hex>
  taa-cati program-yetki liste
  taa-cati program-yetki ayarla --program <ad> (--acik | --kapali) [--sube <id>] [--rol <rol>]   (kapalı kazanır)
  taa-cati gunluk [--islem <ad>] [--kullanici-id <no>] [--sube <id>] [--tarih-bas <YYYY-AA-GG>] [--tarih-bit <..>] [--limit <1..500>] [--onceki-id <no>]
  taa-cati sube liste | sube ekle --kod <kod> --ad <ad>
Kurallar: varsayılan reddet (izin yoksa istek gitmez); silme yok; parola ve oturum anahtarı hiçbir yere yazılmaz.";

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
    let json = args.iter().any(|a| a == "--json");
    match calistir(&args, json) {
        Ok(()) => ExitCode::from(0),
        Err((kod, m)) => {
            if json {
                eprintln!("{}", json!({"hata": m, "kod": kod}));
            } else {
                eprintln!("{m}");
            }
            ExitCode::from(kod)
        }
    }
}

type Hata = (u8, String);

fn ch(e: CatiHata) -> Hata {
    (hata_kodu(&e), e.to_string())
}

#[derive(Default)]
struct Secenekler {
    sunucu: Option<String>,
    kok: Option<PathBuf>,
    sertifika: Option<PathBuf>,
    ad: Option<String>,
    gorunen_ad: Option<String>,
    roller: Vec<String>,
    subeler: Vec<Option<i64>>,
    id: Option<i64>,
    kimlik: Option<String>,
    program: Option<String>,
    acik: Option<bool>,
    durum: Option<String>,
    islem: Option<String>,
    kullanici_id: Option<i64>,
    tarih_bas: Option<String>,
    tarih_bit: Option<String>,
    limit: Option<u32>,
    onceki_id: Option<i64>,
    kod: Option<String>,
    sozcukler: Vec<String>,
}

fn sayi<T: std::str::FromStr>(ad: &str, d: &str) -> Result<T, Hata> {
    d.parse::<T>()
        .map_err(|_| (2, format!("{ad} sayı olmalı: {d}")))
}

/// Komut satırını ayrıştırır. `--rol` birden çok verilebilir; `--sube` kendinden önceki `--rol`a bağlanır
/// (rolden önce ya da rol olmadan verilirse genel şube olur).
fn ayristir(args: &[String]) -> Result<Secenekler, Hata> {
    let mut s = Secenekler::default();
    let mut i = 0;
    let deger = |i: &mut usize, ad: &str| -> Result<String, Hata> {
        *i += 1;
        args.get(*i).cloned().ok_or_else(|| {
            (
                2,
                format!("{ad} için değer gerekli. Öneri: taa-cati --yardim"),
            )
        })
    };
    // --rol olmadan verilen --sube (cihaz onay, program-yetki, gunluk) buraya düşer.
    let mut genel_sube: Option<i64> = None;
    while i < args.len() {
        let a = args[i].as_str();
        match a {
            "--sunucu" => s.sunucu = Some(deger(&mut i, a)?),
            "--kok" => s.kok = Some(PathBuf::from(deger(&mut i, a)?)),
            "--sertifika" => s.sertifika = Some(PathBuf::from(deger(&mut i, a)?)),
            "--ad" => s.ad = Some(deger(&mut i, a)?),
            "--gorunen-ad" => s.gorunen_ad = Some(deger(&mut i, a)?),
            "--kod" => s.kod = Some(deger(&mut i, a)?),
            "--rol" => {
                s.roller.push(deger(&mut i, a)?);
                s.subeler.push(None);
            }
            "--sube" => {
                let d: i64 = sayi("--sube", &deger(&mut i, a)?)?;
                match s.subeler.last_mut() {
                    Some(son) if son.is_none() => *son = Some(d),
                    _ => genel_sube = Some(d),
                }
            }
            "--id" => s.id = Some(sayi("--id", &deger(&mut i, a)?)?),
            "--kimlik" => s.kimlik = Some(deger(&mut i, a)?),
            "--program" => s.program = Some(deger(&mut i, a)?),
            "--acik" => s.acik = Some(true),
            "--kapali" => s.acik = Some(false),
            "--durum" => s.durum = Some(deger(&mut i, a)?),
            "--islem" => s.islem = Some(deger(&mut i, a)?),
            "--kullanici-id" => s.kullanici_id = Some(sayi("--kullanici-id", &deger(&mut i, a)?)?),
            "--tarih-bas" => s.tarih_bas = Some(deger(&mut i, a)?),
            "--tarih-bit" => s.tarih_bit = Some(deger(&mut i, a)?),
            "--limit" => s.limit = Some(sayi("--limit", &deger(&mut i, a)?)?),
            "--onceki-id" => s.onceki_id = Some(sayi("--onceki-id", &deger(&mut i, a)?)?),
            "--json" => {}
            "--parola" => {
                return Err((
                    2,
                    "parola komut satırında verilmez; standart girdiden okunur.".into(),
                ))
            }
            x if x.starts_with("--") => {
                return Err((
                    2,
                    format!("bilinmeyen seçenek: {x}. Öneri: taa-cati --yardim"),
                ))
            }
            x => s.sozcukler.push(x.to_string()),
        }
        i += 1;
    }
    if s.roller.is_empty() {
        // --rol yokken ilk --sube "genel" şubedir; subeler[0] bu değeri taşır.
        s.subeler = vec![genel_sube];
    } else if let Some(g) = genel_sube {
        // --rol'lerden önce verilmiş bir --sube: rolü olmayan ilk atamaya değil, genel şubeye yazılır.
        s.subeler.insert(0, Some(g));
        s.roller.insert(0, String::new());
    }
    Ok(s)
}

fn parola_oku(ne: &str) -> Result<String, Hata> {
    let mut p = String::new();
    let stdin = std::io::stdin();
    if stdin.lock().read_line(&mut p).is_err() || p.trim().is_empty() {
        return Err((2, format!("{ne} standart girdiden okunamadı (boş).")));
    }
    let _ = stdin.lock().read_to_end(&mut Vec::new());
    Ok(p.trim_end_matches(['\n', '\r']).to_string())
}

fn bas(json: bool, v: &Value) {
    if json {
        println!("{v}");
    } else {
        println!("{}", serde_json::to_string_pretty(v).unwrap_or_default());
    }
}

fn tablo(json: bool, satirlar: &[Value], alanlar: &[&str]) {
    if json {
        println!("{}", Value::Array(satirlar.to_vec()));
        return;
    }
    if satirlar.is_empty() {
        println!("(kayıt yok)");
        return;
    }
    println!("{}", alanlar.join("\t"));
    for s in satirlar {
        let h: Vec<String> = alanlar
            .iter()
            .map(|a| match s.get(a) {
                None | Some(Value::Null) => "-".to_string(),
                Some(Value::String(x)) => x.clone(),
                Some(v) => v.to_string(),
            })
            .collect();
        println!("{}", h.join("\t"));
    }
}

fn calistir(args: &[String], json: bool) -> Result<(), Hata> {
    let s = ayristir(args)?;
    let kok = s
        .kok
        .clone()
        .or_else(veri_kok_ortamdan)
        .ok_or_else(|| (2, "veri kökü bulunamadı. Öneri: --kok verin.".to_string()))?;
    // --sunucu / TAA_SUNUCU yoksa yol listesinden ilk ulaşılan adres (taa-yol, S4a); oturum anahtarı listenin adı.
    let h = taa_yol::hedef_coz_curl(&kok, s.sunucu.as_deref(), s.sertifika.as_deref())
        .map_err(|e| (taa_yol::hata_kodu(&e), e.to_string()))?;
    let (oturum, g) = yonetici_baglantisi_hedef(&kok, &h.anahtar, &h.taban, h.sertifika.as_deref())
        .map_err(ch)?;
    let c = Cati::oturumdan(&g, &oturum);
    let gerek = |ad: &str, v: &Option<String>| -> Result<String, Hata> {
        v.clone()
            .ok_or_else(|| (2, format!("{ad} gerekli. Öneri: taa-cati --yardim")))
    };
    let id = || s.id.ok_or_else(|| (2, "--id gerekli.".to_string()));
    let rol_atama = |i: usize| -> Result<RolAtama, Hata> {
        let r = s
            .roller
            .get(i)
            .ok_or_else(|| (2, "--rol gerekli.".to_string()))?;
        RolAtama::yeni(r, s.subeler.get(i).copied().flatten()).map_err(ch)
    };
    let genel_sube = || {
        if s.roller.is_empty() || s.roller.first().is_some_and(String::is_empty) {
            s.subeler.first().copied().flatten()
        } else {
            None
        }
    };
    let mut soz = s.sozcukler.iter().map(String::as_str);
    match (soz.next(), soz.next(), soz.next()) {
        (Some("dogrula"), None, _) => {
            let p = parola_oku("parola")?;
            let v = c.yeniden_dogrula(&p).map_err(ch)?;
            if json {
                bas(true, &v);
            } else {
                println!("doğrulandı: yazma işlemleri 15 dakika açık.");
            }
        }
        (Some("kullanici"), Some("liste"), None) => tablo(
            json,
            &c.kullanicilar().map_err(ch)?,
            &["id", "ad", "gorunen_ad", "aktif", "roller"],
        ),
        (Some("kullanici"), Some("ekle"), None) => {
            let ad = gerek("--ad", &s.ad)?;
            let gad = gerek("--gorunen-ad", &s.gorunen_ad)?;
            let roller: Vec<RolAtama> = (0..s.roller.len())
                .filter(|i| !s.roller[*i].is_empty())
                .map(rol_atama)
                .collect::<Result<_, _>>()?;
            let p = parola_oku("parola")?;
            bas(json, &c.kullanici_ekle(&ad, &gad, &p, &roller).map_err(ch)?);
        }
        (Some("kullanici"), Some("rol"), None) => {
            bas(json, &c.rol_ver(id()?, &rol_atama(0)?).map_err(ch)?)
        }
        (Some("kullanici"), Some("rol-kaldir"), None) => {
            bas(json, &c.rol_kaldir(id()?, &rol_atama(0)?).map_err(ch)?)
        }
        (Some("kullanici"), Some("kapat"), None) => {
            bas(json, &c.kullanici_kapat(id()?).map_err(ch)?)
        }
        (Some("kullanici"), Some("parola"), None) => {
            let i = id()?;
            let p = parola_oku("yeni parola")?;
            bas(json, &c.parola_degistir(i, &p).map_err(ch)?);
        }
        (Some("rol"), Some("liste"), None) => tablo(
            json,
            &c.roller().map_err(ch)?,
            &["ad", "miras_rol", "izinler"],
        ),
        (Some("izin"), Some("liste"), None) => {
            tablo(json, &c.izinler().map_err(ch)?, &["ad", "aciklama"])
        }
        (Some("cihaz"), Some("liste"), None) => tablo(
            json,
            &c.cihazlar(s.durum.as_deref()).map_err(ch)?,
            &[
                "kimlik",
                "ad",
                "durum",
                "istenen_sube",
                "sube_id",
                "kayit_zamani",
                "son_gorulme",
            ],
        ),
        (Some("cihaz"), Some("onay"), None) => {
            let k = gerek("--kimlik", &s.kimlik)?;
            bas(json, &c.cihaz_onayla(&k, genel_sube()).map_err(ch)?);
        }
        (Some("cihaz"), Some("iptal"), None) => {
            let k = gerek("--kimlik", &s.kimlik)?;
            bas(json, &c.cihaz_iptal(&k).map_err(ch)?);
        }
        (Some("program-yetki"), Some("liste"), None) => tablo(
            json,
            &c.program_yetkileri().map_err(ch)?,
            &["id", "sube_id", "rol", "program", "acik"],
        ),
        (Some("program-yetki"), Some("ayarla"), None) => {
            let p = gerek("--program", &s.program)?;
            let acik = s
                .acik
                .ok_or_else(|| (2, "--acik ya da --kapali gerekli.".to_string()))?;
            let rol = s.roller.iter().find(|r| !r.is_empty()).map(String::as_str);
            let sube = s.subeler.iter().copied().flatten().next();
            bas(
                json,
                &c.program_yetki_ayarla(sube, rol, &p, acik).map_err(ch)?,
            );
        }
        (Some("gunluk"), None, _) => {
            let f = OlaySuzgeci {
                islem: s.islem.clone(),
                kullanici_id: s.kullanici_id,
                sube_id: genel_sube(),
                tarih_bas: s.tarih_bas.clone(),
                tarih_bit: s.tarih_bit.clone(),
                limit: s.limit,
                onceki_id: s.onceki_id,
            };
            tablo(
                json,
                &c.olaylar(&f).map_err(ch)?,
                &[
                    "id",
                    "zaman",
                    "kullanici_id",
                    "cihaz_kimlik",
                    "sube_id",
                    "islem",
                    "ayrinti",
                ],
            );
        }
        (Some("sube"), Some("liste"), None) => tablo(
            json,
            &c.subeler().map_err(ch)?,
            &["id", "kod", "ad", "aktif"],
        ),
        (Some("sube"), Some("ekle"), None) => {
            let kod = gerek("--kod", &s.kod)?;
            let ad = gerek("--ad", &s.ad)?;
            bas(json, &c.sube_ekle(&kod, &ad).map_err(ch)?);
        }
        (None, _, _) => return Err((2, "komut gerekli. Öneri: taa-cati --yardim".into())),
        (Some(a), b, _) => {
            return Err((
                2,
                format!(
                    "bilinmeyen komut: {a}{}. Öneri: taa-cati --yardim",
                    b.map(|x| format!(" {x}")).unwrap_or_default()
                ),
            ))
        }
    }
    Ok(())
}
