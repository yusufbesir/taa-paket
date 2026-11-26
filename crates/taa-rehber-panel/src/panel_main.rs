// Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]
//! `taa-rehber-panel`: TAA Rehber masaüstü paneli.
//!
//! 127.0.0.1'de rastgele kapıda yerel bir sunucu açar, Edge'i uygulama penceresi olarak (`--app`) başlatır. Pencere
//! kapanıp istek gelmeyince kendiliğinden kapanır (taa-cati paneliyle aynı kalıp). Tüm iş mantığı kütüphanededir
//! (`taa_rehber_panel::yonlendir`, saf işlev); bu ikili yalnız HTTP, pencere ve `taa-kimlik` yönetici girişini bağlar.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};
use taa_rehber_panel::guvenlik::{self, IstekBilgi};
use taa_rehber_panel::{
    arayuz_html, yonlendir, Durum, Istek, Kimlik, KimlikBilgisi, KimlikRed, SistemOrtami,
    Yapilandirma, SIMGE_SVG,
};
use tiny_http::{Header, Method, Request, Response, Server};

const YARDIM: &str = "\
taa-rehber-panel — TAA Rehber masaüstü paneli.
Kullanım: taa-rehber-panel [--sube <kod>] [--demo] [--sunucu <https://adres>] [--kok <veri kökü>] [--sertifika <pem>] [--pencere-acma]
  --sube <kod>    bu bilgisayarın şubesi (rehber kapsamı); verilmezse yalnız merkez kapsamı görünür.
  --demo          \"ÖRNEK\" etiketli uydurma veriyle açar (gerçek veri yok); yalnız deneme içindir.
  --pencere-acma  pencereyi açmaz, açılış adresini yazar (tek seferlik).
Panel yalnız bu bilgisayardan (127.0.0.1) açılır; pencere kapanınca birkaç dakika içinde kendiliğinden kapanır.
Yönetici işlemleri yalnız taa-kimlik ile doğrulanmış ve rehber.yaz izinli girişle açılır; kimlik doğrulanamazsa kapalı kalır.";

/// Açılış bağlantısı kullanılmazsa bekleme süresi.
const ACILIS_SURESI: Duration = Duration::from_secs(5 * 60);
/// Pencere açıkken ekran 30 sn'de bir yoklar; bu kadar sessizlikte panel kapanır.
const SESSIZLIK_SURESI: Duration = Duration::from_secs(3 * 60);

struct Ayar {
    kok: PathBuf,
    sunucu: Option<String>,
    sertifika: Option<PathBuf>,
    sube: Option<String>,
    demo: bool,
}

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
    taa_hata::kur_varsayilan("taa-rehber-panel", env!("CARGO_PKG_VERSION"));
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args
        .iter()
        .any(|a| matches!(a.as_str(), "--yardim" | "--help" | "-h" | "yardim"))
    {
        println!("{YARDIM}");
        return ExitCode::SUCCESS;
    }
    let (mut sunucu, mut kok, mut sertifika, mut sube) = (None, None, None, None);
    let (mut pencere, mut demo) = (true, false);
    let mut i = 0;
    while i < args.len() {
        let deger = |i: usize| args.get(i + 1).cloned();
        match args[i].as_str() {
            "--sunucu" => sunucu = deger(i),
            "--kok" => kok = deger(i).map(PathBuf::from),
            "--sertifika" => sertifika = deger(i).map(PathBuf::from),
            "--sube" => sube = deger(i).filter(|s| !s.trim().is_empty()),
            "--pencere-acma" => {
                pencere = false;
                i += 1;
                continue;
            }
            "--demo" => {
                demo = true;
                i += 1;
                continue;
            }
            x => {
                eprintln!("bilinmeyen seçenek: {x}\n{YARDIM}");
                return ExitCode::from(2);
            }
        }
        i += 2;
    }
    let Some(kok) = kok.or_else(veri_kok_ortamdan) else {
        eprintln!("veri kökü bulunamadı. Öneri: --kok verin.");
        return ExitCode::from(2);
    };
    let ayar = Ayar {
        kok,
        sunucu,
        sertifika,
        sube,
        demo,
    };
    match calistir(&ayar, pencere) {
        Ok(()) => ExitCode::SUCCESS,
        Err(m) => {
            eprintln!("{m}");
            ExitCode::from(1)
        }
    }
}

fn calistir(ayar: &Ayar, pencere: bool) -> Result<(), String> {
    let mut durum = Durum::yeni(
        Yapilandirma {
            kok: ayar.kok.clone(),
            sube_kodu: ayar.sube.clone(),
            demo: ayar.demo,
            kimlik: Box::new(KimlikTaa {
                kok: ayar.kok.clone(),
                sunucu: ayar.sunucu.clone(),
                sertifika: ayar.sertifika.clone(),
            }),
        },
        &SistemOrtami,
    )?;
    let sunucu = Server::http("127.0.0.1:0").map_err(|e| format!("yerel panel açılamadı: {e}"))?;
    let kapi = sunucu
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .ok_or("yerel panel adresi alınamadı")?;
    // İki ayrı belirteç: açılış bağlantısı tek seferliktir; çerez belirteci adreste hiç görünmez.
    let acilis = guvenlik::belirtec_uret()?;
    let cerez = guvenlik::belirtec_uret()?;
    let adres = format!("http://127.0.0.1:{kapi}/ac?b={acilis}");
    if pencere {
        pencere_ac(&adres);
    } else {
        println!("{adres}");
    }
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
            Err(e) => return Err(format!("yerel panel hatası: {e}")),
        };
        if isle(istek, &mut durum, kapi, &acilis, &cerez, &mut acildi) == Some(true) {
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

fn yanitla<R: Read>(istek: Request, mut y: Response<R>, tur: &str) {
    for (a, d) in guvenlik::GUVENLIK_BASLIKLARI {
        if let Ok(h) = Header::from_bytes(a.as_bytes(), d.as_bytes()) {
            y.add_header(h);
        }
    }
    if let Ok(h) = Header::from_bytes(&b"Content-Type"[..], tur.as_bytes()) {
        y.add_header(h);
    }
    let _ = istek.respond(y);
}

fn metin(istek: Request, durum: u16, m: &str) {
    yanitla(
        istek,
        Response::from_string(m).with_status_code(durum),
        "text/plain; charset=utf-8",
    );
}

fn json_yanit(istek: Request, (durum, v): (u16, Value)) {
    yanitla(
        istek,
        Response::from_string(v.to_string()).with_status_code(durum),
        "application/json; charset=utf-8",
    );
}

/// Bir isteği işler. Dönüş: Some(true) geçerli (çerezli) istek; Some(false)/None etkinlik sayılmaz.
fn isle(
    mut istek: Request,
    durum: &mut Durum,
    kapi: u16,
    acilis: &str,
    cerez: &str,
    acildi: &mut bool,
) -> Option<bool> {
    let url = istek.url().to_string();
    let (yol, sorgu) = url.split_once('?').unwrap_or((&url, ""));
    let (yol, sorgu) = (yol.to_string(), sorgu.to_string());
    let yontem = match istek.method() {
        Method::Get => "GET",
        Method::Post => "POST",
        _ => {
            metin(istek, 405, "yöntem desteklenmiyor");
            return None;
        }
    };

    // Açılış: tek seferlik bağlantı → çerez.
    if yontem == "GET" && yol == "/ac" {
        if !guvenlik::host_gecerli(baslik(&istek, "Host"), kapi) {
            metin(istek, 421, "geçersiz adres");
            return None;
        }
        let b = guvenlik::sorgu_degeri(&sorgu, "b").unwrap_or_default();
        if *acildi || !guvenlik::esit(b, acilis) {
            metin(
                istek,
                403,
                "Bu açılış bağlantısı kullanıldı ya da geçersiz. Paneli yeniden başlatın.",
            );
            return None;
        }
        *acildi = true;
        let mut y = Response::from_string("").with_status_code(303);
        if let Ok(h) =
            Header::from_bytes(&b"Set-Cookie"[..], guvenlik::cerez_ayarla(cerez).as_bytes())
        {
            y.add_header(h);
        }
        if let Ok(h) = Header::from_bytes(&b"Location"[..], &b"/"[..]) {
            y.add_header(h);
        }
        yanitla(istek, y, "text/plain; charset=utf-8");
        return Some(true);
    }

    let bilgi = IstekBilgi {
        yontem,
        host: baslik(&istek, "Host"),
        origin: baslik(&istek, "Origin"),
        cerez: baslik(&istek, "Cookie"),
        icerik_turu: baslik(&istek, "Content-Type"),
    };
    if let Err(red) = guvenlik::istek_denetle(&bilgi, cerez, kapi) {
        metin(istek, red.durum_kodu(), "erişim reddedildi");
        return Some(false);
    }

    let mut govde = Value::Null;
    if yontem == "POST" {
        let mut b = Vec::new();
        let n = istek
            .as_reader()
            .take(guvenlik::GOVDE_EN_COK as u64 + 1)
            .read_to_end(&mut b)
            .unwrap_or(0);
        if n > guvenlik::GOVDE_EN_COK {
            metin(istek, 413, "gövde çok büyük");
            return Some(true);
        }
        govde = match serde_json::from_slice(&b) {
            Ok(v @ Value::Object(_)) => v,
            _ => {
                json_yanit(
                    istek,
                    (
                        400,
                        json!({"hata": "gövde JSON nesnesi olmalı", "kod": "gecersiz"}),
                    ),
                );
                return Some(true);
            }
        };
    }

    match (yontem, yol.as_str()) {
        ("GET", "/") => yanitla(
            istek,
            Response::from_string(arayuz_html(durum.tema)),
            "text/html; charset=utf-8",
        ),
        ("GET", "/simge.svg") => yanitla(istek, Response::from_string(SIMGE_SVG), "image/svg+xml"),
        ("POST", "/api/kapat") => {
            json_yanit(istek, (200, json!({"tamam": true})));
            std::process::exit(0);
        }
        (_, y) if y.starts_with("/api/") => {
            let r = yonlendir(
                durum,
                &SistemOrtami,
                &Istek {
                    yontem,
                    yol: y,
                    sorgu: &sorgu,
                    govde: &govde,
                },
            );
            json_yanit(istek, r);
        }
        _ => metin(istek, 404, "bulunamadı"),
    }
    Some(true)
}

/// `taa-kimlik` ile merkezi yönetici girişi (taa-cati paneliyle aynı akış: cihaz imzalı gönderici, sunucu yanıt imzası
/// anahtar varsa doğrulanır). Oturum anahtarı SAKLANMAZ ve belleğe alınmaz; yalnız kullanıcı adı ve izin listesi döner.
/// Cihaz anahtarı ya da sunucu yoksa giriş doğrulanamaz ve reddedilir (varsayılan REDDET).
struct KimlikTaa {
    kok: PathBuf,
    sunucu: Option<String>,
    sertifika: Option<PathBuf>,
}

impl KimlikTaa {
    fn sunucu_acik(&self) -> bool {
        self.sunucu.is_some() || std::env::var("TAA_SUNUCU").is_ok_and(|v| !v.trim().is_empty())
    }
}

impl Kimlik for KimlikTaa {
    fn giris(&self, kullanici: &str, parola: &str) -> Result<KimlikBilgisi, KimlikRed> {
        let dog = |m: String| KimlikRed::Dogrulanamadi(m);
        let anahtar_ad = taa_yol::anahtar_coz_ortam(&self.kok, self.sunucu.as_deref())
            .map_err(|e| dog(e.to_string()))?;
        let h = taa_yol::hedef_coz_curl(
            &self.kok,
            self.sunucu_acik().then_some(anahtar_ad.as_str()),
            self.sertifika.as_deref(),
        )
        .map_err(|e| dog(e.to_string()))?;
        let imzalayici = taa_kimlik::cihaz_imzalayici(&self.kok)
            .map_err(|e| dog(e.to_string()))?
            .ok_or_else(|| {
                dog("bu bilgisayarın cihaz anahtarı yok (taa-cihaz uret, sonra sunucuda merkez cihazı onayı)".into())
            })?;
        let mut g = taa_kimlik::CurlGonderici::yeni(&h.taban).map_err(|e| dog(e.to_string()))?;
        if let Some(c) = &h.sertifika {
            g = g.sertifika(c);
        }
        g = g.imzalayici(imzalayici);
        let yedekli = if self.sunucu_acik() {
            None
        } else {
            taa_kimlik::yedekli_gonderici(&self.kok, None, self.sertifika.as_deref())
                .map_err(|e| dog(e.to_string()))?
        };
        let sunucu_anahtari =
            taa_kimlik::sunucu_anahtari_oku(&self.kok).map_err(|e| dog(e.to_string()))?;
        let gonderici: &dyn taa_kimlik::Gonderici = match &yedekli {
            Some(y) => y,
            None => &g,
        };
        match taa_kimlik::giris_dogrulayarak(
            gonderici,
            &anahtar_ad,
            kullanici,
            parola,
            None,
            sunucu_anahtari.as_ref(),
        ) {
            Ok(o) => Ok(KimlikBilgisi {
                kullanici: o.kullanici.clone(),
                izinler: o.izinler.clone(),
            }),
            Err(taa_kimlik::KimlikHata::GecersizGirdi(m)) => Err(KimlikRed::Gecersiz(m)),
            Err(e @ taa_kimlik::KimlikHata::GirisReddi { .. }) => {
                Err(KimlikRed::Reddedildi(e.to_string()))
            }
            Err(e) => Err(dog(e.to_string())),
        }
    }
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
