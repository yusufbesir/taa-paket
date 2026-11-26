// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! `taa-kasa`: TAA Kasa gün sonu programı (tek exe).
//!
//! 127.0.0.1'de rastgele kapıda yerel sunucu açar ve ekranı gömülü pencerede gösterir: Windows'ta Edge
//! uygulama penceresi (`--app`, adres çubuğu yok; WebView2 ile aynı motor), yoksa varsayılan tarayıcı.
//! Pencere kapanıp istek gelmeyince kendiliğinden kapanır. Kalıp: crates/taa-cati/src/panel_main.rs.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};
use taa_kasa_ekran::api::{Cevap, Uygulama};
use taa_kasa_ekran::{self as ek, IstekBilgi};
use tiny_http::{Header, Method, Request, Response, Server};

const YARDIM: &str = "\
taa-kasa — TAA Kasa gün sonu programı.
Kullanım: taa-kasa [--kok <veri kökü>] [--pencere-acma]
  --pencere-acma   pencereyi açmaz, açılış adresini yazar (tek seferlik).
Program yalnız bu bilgisayardan (127.0.0.1) açılır; pencere kapanınca birkaç dakika içinde kendiliğinden kapanır.";

const ACILIS_SURESI: Duration = Duration::from_secs(5 * 60);
const SESSIZLIK_SURESI: Duration = Duration::from_secs(3 * 60);

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
    taa_hata::kur_varsayilan("taa-kasa", env!("CARGO_PKG_VERSION"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args
        .iter()
        .any(|a| matches!(a.as_str(), "--yardim" | "--help" | "-h" | "yardim"))
    {
        println!("{YARDIM}");
        return ExitCode::SUCCESS;
    }
    let mut kok = None;
    let mut pencere = true;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--kok" => {
                kok = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            "--pencere-acma" => {
                pencere = false;
                i += 1;
            }
            x => {
                eprintln!("bilinmeyen seçenek: {x}\n{YARDIM}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(kok) = kok.or_else(veri_kok_ortamdan) else {
        eprintln!("veri kökü bulunamadı. Öneri: --kok verin.");
        return ExitCode::from(2);
    };
    match calistir(kok, pencere) {
        Ok(()) => ExitCode::SUCCESS,
        Err(m) => {
            eprintln!("{m}");
            ExitCode::from(1)
        }
    }
}

fn calistir(kok: PathBuf, pencere: bool) -> Result<(), String> {
    let sunucu = Server::http("127.0.0.1:0").map_err(|e| format!("yerel ekran açılamadı: {e}"))?;
    let kapi = sunucu
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .ok_or("yerel ekran adresi alınamadı")?;
    let acilis = ek::belirtec_uret()?;
    let cerez = ek::belirtec_uret()?;
    let adres = format!("http://127.0.0.1:{kapi}/ac?b={acilis}");
    if pencere {
        pencere_ac(&adres);
    } else {
        println!("{adres}");
    }
    let mut uyg = Uygulama::yeni(kok);
    let mut acildi = false;
    let mut son = Instant::now();
    loop {
        let sinir = if acildi {
            SESSIZLIK_SURESI
        } else {
            ACILIS_SURESI
        };
        if son.elapsed() > sinir {
            return Ok(());
        }
        let istek = match sunucu.recv_timeout(Duration::from_secs(5)) {
            Ok(Some(r)) => r,
            Ok(None) => continue,
            Err(e) => return Err(format!("yerel ekran hatası: {e}")),
        };
        if isle(istek, &mut uyg, kapi, &acilis, &cerez, &mut acildi) {
            son = Instant::now();
        }
    }
}

fn baslik<'a>(r: &'a Request, ad: &str) -> Option<&'a str> {
    r.headers()
        .iter()
        .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(ad))
        .map(|h| h.value.as_str())
}

fn yanitla<R: Read>(istek: Request, mut y: Response<R>, tur: &str, ek_baslik: &[(&str, String)]) {
    for (a, d) in ek::GUVENLIK_BASLIKLARI {
        if let Ok(h) = Header::from_bytes(a.as_bytes(), d.as_bytes()) {
            y.add_header(h);
        }
    }
    if let Ok(h) = Header::from_bytes(&b"Content-Type"[..], tur.as_bytes()) {
        y.add_header(h);
    }
    for (a, d) in ek_baslik {
        if let Ok(h) = Header::from_bytes(a.as_bytes(), d.as_bytes()) {
            y.add_header(h);
        }
    }
    let _ = istek.respond(y);
}

fn metin(istek: Request, durum: u16, m: &str) {
    yanitla(
        istek,
        Response::from_string(m).with_status_code(durum),
        "text/plain; charset=utf-8",
        &[],
    );
}

fn json_yanit(istek: Request, durum: u16, v: &Value) {
    yanitla(
        istek,
        Response::from_string(v.to_string()).with_status_code(durum),
        "application/json; charset=utf-8",
        &[],
    );
}

/// Bir isteği işler. Dönüş: geçerli (çerezli) istek mi.
fn isle(
    mut istek: Request,
    uyg: &mut Uygulama,
    kapi: u16,
    acilis: &str,
    cerez: &str,
    acildi: &mut bool,
) -> bool {
    let url = istek.url().to_string();
    let (yol, sorgu) = url.split_once('?').unwrap_or((&url, ""));
    let (yol, sorgu) = (yol.to_string(), sorgu.to_string());
    let yontem = match istek.method() {
        Method::Get => "GET",
        Method::Post => "POST",
        _ => {
            metin(istek, 405, "yöntem desteklenmiyor");
            return false;
        }
    };

    if yontem == "GET" && yol == "/ac" {
        if !ek::host_gecerli(baslik(&istek, "Host"), kapi) {
            metin(istek, 421, "geçersiz adres");
            return false;
        }
        let b = ek::sorgu_degeri(&sorgu, "b").unwrap_or_default();
        if *acildi || !ek::esit(b, acilis) {
            metin(
                istek,
                403,
                "Bu açılış bağlantısı kullanıldı ya da geçersiz. TAA Kasa'yı yeniden başlatın.",
            );
            return false;
        }
        *acildi = true;
        let y = Response::from_string("").with_status_code(303);
        yanitla(
            istek,
            y,
            "text/plain; charset=utf-8",
            &[
                ("Set-Cookie", ek::cerez_ayarla(cerez)),
                ("Location", "/".into()),
            ],
        );
        return true;
    }

    let bilgi = IstekBilgi {
        yontem,
        host: baslik(&istek, "Host"),
        origin: baslik(&istek, "Origin"),
        cerez: baslik(&istek, "Cookie"),
        icerik_turu: baslik(&istek, "Content-Type"),
    };
    if let Err(red) = ek::istek_denetle(&bilgi, cerez, kapi) {
        metin(istek, red.durum_kodu(), "erişim reddedildi");
        return false;
    }

    let mut govde = Value::Null;
    if yontem == "POST" {
        let mut b = Vec::new();
        let n = istek
            .as_reader()
            .take(ek::GOVDE_EN_COK as u64 + 1)
            .read_to_end(&mut b)
            .unwrap_or(0);
        if n > ek::GOVDE_EN_COK {
            metin(istek, 413, "gövde çok büyük");
            return true;
        }
        govde = match serde_json::from_slice(&b) {
            Ok(v @ Value::Object(_)) => v,
            _ => {
                json_yanit(istek, 400, &json!({"hata": "gövde JSON nesnesi olmalı"}));
                return true;
            }
        };
    }

    match (yontem, yol.as_str()) {
        ("GET", "/") => yanitla(
            istek,
            Response::from_string(ek::ARAYUZ_HTML),
            "text/html; charset=utf-8",
            &[],
        ),
        ("GET", "/simge.svg") => yanitla(
            istek,
            Response::from_string(ek::SIMGE_SVG),
            "image/svg+xml",
            &[],
        ),
        _ => match uyg.istek(yontem, &yol, &sorgu, &govde) {
            Cevap::Json(d, v) => json_yanit(istek, d, &v),
            Cevap::Dosya { ad, tur, bayt } => yanitla(
                istek,
                Response::from_data(bayt),
                tur,
                &[(
                    "Content-Disposition",
                    format!("attachment; filename=\"{ad}\""),
                )],
            ),
            Cevap::Kapat => {
                json_yanit(istek, 200, &json!({"tamam": true}));
                std::process::exit(0);
            }
        },
    }
    true
}

/// Pencereyi açar: Windows'ta Edge uygulama penceresi (adres çubuğu yok), yoksa varsayılan tarayıcı.
fn pencere_ac(adres: &str) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let adaylar = ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"]
            .iter()
            .filter_map(|e| std::env::var(e).ok())
            .map(|k| {
                Path::new(&k)
                    .join("Microsoft")
                    .join("Edge")
                    .join("Application")
                    .join("msedge.exe")
            });
        for edge in adaylar {
            if edge.is_file()
                && std::process::Command::new(&edge)
                    .arg(format!("--app={adres}"))
                    .arg("--window-size=1280,820")
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()
                    .is_ok()
            {
                return;
            }
        }
        let _ = std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", adres])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn();
    }
    #[cfg(not(windows))]
    {
        let _ = Path::new(adres);
        if std::process::Command::new("xdg-open")
            .arg(adres)
            .spawn()
            .is_err()
        {
            println!("{adres}");
        }
    }
}
