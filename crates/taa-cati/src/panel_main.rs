// Sahip: Yusuf. yazan: [Claude#taa-cati-panel 2026-11-17]
//! `taa-cati-panel`: TAA Çatı'nın grafik yönetim paneli (docs/S3-CATI-PANEL.md).
//!
//! 127.0.0.1'de rastgele kapıda yerel bir sunucu açar, Edge'i uygulama penceresi olarak (`--app`) başlatır.
//! Pencere kapanıp istek gelmeyince kendiliğinden kapanır. Sunucuya giden istekler `taa-cati` ile aynı yoldan
//! gider (cihaz imzası + Bearer, varsayılan reddet, 15 dk yeniden doğrulama). Komut satırı `taa-cati` değişmedi.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use serde_json::{json, Value};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};
use taa_cati::panel::{self, IstekBilgi, Yanit};
use taa_cati::{hata_kodu, yonetici_baglantisi_hedef, Cati};
use tiny_http::{Header, Method, Request, Response, Server};

const YARDIM: &str = "\
taa-cati-panel — TAA Çatı yönetim paneli (yalnız sunucu PC'sinde).
Kullanım: taa-cati-panel [--sunucu <https://adres>] [--kok <veri kökü>] [--sertifika <pem>] [--pencere-acma]
  --pencere-acma   pencereyi açmaz, açılış adresini yazar (tek seferlik).
Panel yalnız bu bilgisayardan (127.0.0.1) açılır; pencere kapanınca birkaç dakika içinde kendiliğinden kapanır.";

/// Açılış bağlantısı kullanılmazsa bekleme süresi.
const ACILIS_SURESI: Duration = Duration::from_secs(5 * 60);
/// Pencere açıkken ekran 30 sn'de bir yoklar; bu kadar sessizlikte panel kapanır.
const SESSIZLIK_SURESI: Duration = Duration::from_secs(3 * 60);

struct Ayar {
    kok: PathBuf,
    sunucu: Option<String>,
    sertifika: Option<PathBuf>,
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
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args
        .iter()
        .any(|a| matches!(a.as_str(), "--yardim" | "--help" | "-h" | "yardim"))
    {
        println!("{YARDIM}");
        return ExitCode::SUCCESS;
    }
    let mut sunucu = None;
    let mut kok = None;
    let mut sertifika = None;
    let mut pencere = true;
    let mut i = 0;
    while i < args.len() {
        let deger = |i: usize| args.get(i + 1).cloned();
        match args[i].as_str() {
            "--sunucu" => sunucu = deger(i),
            "--kok" => kok = deger(i).map(PathBuf::from),
            "--sertifika" => sertifika = deger(i).map(PathBuf::from),
            "--pencere-acma" => {
                pencere = false;
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
    let sunucu = Server::http("127.0.0.1:0").map_err(|e| format!("yerel panel açılamadı: {e}"))?;
    let kapi = sunucu
        .server_addr()
        .to_ip()
        .map(|a| a.port())
        .ok_or("yerel panel adresi alınamadı")?;
    // İki ayrı belirteç: açılış bağlantısı tek seferliktir; çerez belirteci adreste hiç görünmez.
    let acilis = panel::belirtec_uret()?;
    let cerez = panel::belirtec_uret()?;
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
        if let Some(gecerli) = isle(istek, ayar, kapi, &acilis, &cerez, &mut acildi) {
            if gecerli {
                son = Instant::now();
            }
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
    for (a, d) in panel::GUVENLIK_BASLIKLARI {
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

fn json_yanit(istek: Request, (durum, v): Yanit) {
    yanitla(
        istek,
        Response::from_string(v.to_string()).with_status_code(durum),
        "application/json; charset=utf-8",
    );
}

/// Bir isteği işler. Dönüş: Some(true) geçerli (çerezli) istek; Some(false)/None etkinlik sayılmaz.
fn isle(
    mut istek: Request,
    ayar: &Ayar,
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
        if !panel::host_gecerli(baslik(&istek, "Host"), kapi) {
            metin(istek, 421, "geçersiz adres");
            return None;
        }
        let b = panel::sorgu_degeri(&sorgu, "b").unwrap_or_default();
        if *acildi || !panel::esit(b, acilis) {
            metin(
                istek,
                403,
                "Bu açılış bağlantısı kullanıldı ya da geçersiz. Paneli yeniden başlatın.",
            );
            return None;
        }
        *acildi = true;
        let mut y = Response::from_string("").with_status_code(303);
        if let Ok(h) = Header::from_bytes(&b"Set-Cookie"[..], panel::cerez_ayarla(cerez).as_bytes())
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
    if let Err(red) = panel::istek_denetle(&bilgi, cerez, kapi) {
        metin(istek, red.durum_kodu(), "erişim reddedildi");
        return Some(false);
    }

    let mut govde = Value::Null;
    if yontem == "POST" {
        let mut b = Vec::new();
        let n = istek
            .as_reader()
            .take(panel::GOVDE_EN_COK as u64 + 1)
            .read_to_end(&mut b)
            .unwrap_or(0);
        if n > panel::GOVDE_EN_COK {
            metin(istek, 413, "gövde çok büyük");
            return Some(true);
        }
        govde = match serde_json::from_slice(&b) {
            Ok(v @ Value::Object(_)) => v,
            _ => {
                json_yanit(
                    istek,
                    (400, json!({"hata": "gövde JSON nesnesi olmalı", "kod": 2})),
                );
                return Some(true);
            }
        };
    }

    match (yontem, yol.as_str()) {
        ("GET", "/") => yanitla(
            istek,
            Response::from_string(panel::ARAYUZ_HTML),
            "text/html; charset=utf-8",
        ),
        ("GET", "/simge.svg") => yanitla(
            istek,
            Response::from_string(panel::SIMGE_SVG),
            "image/svg+xml",
        ),
        ("GET", "/api/oturum") => json_yanit(istek, oturum(ayar)),
        ("POST", "/api/giris") => json_yanit(istek, giris(ayar, &govde)),
        ("POST", "/api/cikis") => json_yanit(istek, cikis(ayar)),
        ("POST", "/api/kapat") => {
            json_yanit(istek, (200, json!({"tamam": true})));
            std::process::exit(0);
        }
        _ => {
            let y = baglan(ayar, |c, g| panel::api(c, g, yontem, &yol, &sorgu, &govde));
            json_yanit(istek, y);
        }
    }
    Some(true)
}

fn kimlik_hata(e: taa_kimlik::KimlikHata) -> Yanit {
    let e = taa_cati::CatiHata::from(e);
    let k = hata_kodu(&e);
    (
        panel::hata_durumu(k),
        json!({"hata": e.to_string(), "kod": k}),
    )
}

fn yol_hata(e: taa_yol::YolHata) -> Yanit {
    let k = taa_yol::hata_kodu(&e);
    (
        panel::hata_durumu(k),
        json!({"hata": e.to_string(), "kod": k}),
    )
}

fn sunucu_acik(ayar: &Ayar) -> bool {
    ayar.sunucu.is_some() || std::env::var("TAA_SUNUCU").is_ok_and(|v| !v.trim().is_empty())
}

/// `taa-cati` ile aynı bağlantı: yol listesi (taa-yol) varsa yedekli gönderici.
fn baglan(ayar: &Ayar, f: impl FnOnce(&Cati, &dyn taa_kimlik::Gonderici) -> Yanit) -> Yanit {
    let h =
        match taa_yol::hedef_coz_curl(&ayar.kok, ayar.sunucu.as_deref(), ayar.sertifika.as_deref())
        {
            Ok(h) => h,
            Err(e) => return yol_hata(e),
        };
    let (oturum, g) =
        match yonetici_baglantisi_hedef(&ayar.kok, &h.anahtar, &h.taban, h.sertifika.as_deref()) {
            Ok(x) => x,
            Err(e) => {
                let k = hata_kodu(&e);
                return (
                    panel::hata_durumu(k),
                    json!({"hata": e.to_string(), "kod": k}),
                );
            }
        };
    let yedekli = if sunucu_acik(ayar) {
        None
    } else {
        match taa_kimlik::yedekli_gonderici(&ayar.kok, Some(&oturum), ayar.sertifika.as_deref()) {
            Ok(y) => y,
            Err(e) => return kimlik_hata(e),
        }
    };
    let gonderici: &dyn taa_kimlik::Gonderici = match &yedekli {
        Some(y) => y,
        None => &g,
    };
    let c = Cati::oturumdan(gonderici, &oturum);
    f(&c, gonderici)
}

fn depo_anahtari(ayar: &Ayar) -> Result<String, Yanit> {
    taa_yol::anahtar_coz_ortam(&ayar.kok, ayar.sunucu.as_deref()).map_err(yol_hata)
}

fn oturum(ayar: &Ayar) -> Yanit {
    let anahtar = match depo_anahtari(ayar) {
        Ok(a) => a,
        Err(y) => return y,
    };
    match taa_kimlik::varsayilan_depo(&ayar.kok).oku(&anahtar) {
        Ok(o) => (200, panel::oturum_ozeti(&o)),
        Err(taa_kimlik::KimlikHata::OturumYok) => {
            (200, json!({"girisli": false, "sunucu": anahtar}))
        }
        Err(e) => kimlik_hata(e),
    }
}

fn cikis(ayar: &Ayar) -> Yanit {
    let anahtar = match depo_anahtari(ayar) {
        Ok(a) => a,
        Err(y) => return y,
    };
    match taa_kimlik::varsayilan_depo(&ayar.kok).cikis(&anahtar) {
        Ok(()) => (200, json!({"tamam": true})),
        Err(e) => kimlik_hata(e),
    }
}

/// `taa-kimlik giris` ile aynı akış: cihaz imzalı giriş, sunucu yanıt imzası (anahtar varsa) doğrulanır,
/// oturum depoya yazılır, çevrimdışı yetki belgesi alınır (başarısızlığı girişi bozmaz).
fn giris(ayar: &Ayar, govde: &Value) -> Yanit {
    let (Some(k), Some(p)) = (
        govde.get("kullanici").and_then(Value::as_str),
        govde.get("parola").and_then(Value::as_str),
    ) else {
        return (
            400,
            json!({"hata": "kullanıcı adı ve parola gerekli", "kod": 2}),
        );
    };
    let kod = govde.get("kod").and_then(Value::as_str);
    let anahtar_ad = match depo_anahtari(ayar) {
        Ok(a) => a,
        Err(y) => return y,
    };
    let h = match taa_yol::hedef_coz_curl(
        &ayar.kok,
        sunucu_acik(ayar).then_some(anahtar_ad.as_str()),
        ayar.sertifika.as_deref(),
    ) {
        Ok(h) => h,
        Err(e) => return yol_hata(e),
    };
    let imzalayici = match taa_kimlik::cihaz_imzalayici(&ayar.kok) {
        Ok(Some(i)) => i,
        Ok(None) => {
            return (
                401,
                json!({"hata": format!("Bu bilgisayarın cihaz anahtarı yok ({}). Öneri: taa-cihaz uret, sonra sunucuda merkez cihazı onayı.", ayar.kok.join("cihaz").display()), "kod": 5}),
            )
        }
        Err(e) => return kimlik_hata(e),
    };
    let mut g = match taa_kimlik::CurlGonderici::yeni(&h.taban) {
        Ok(g) => g,
        Err(e) => return (400, json!({"hata": e.to_string(), "kod": 2})),
    };
    if let Some(c) = &h.sertifika {
        g = g.sertifika(c);
    }
    g = g.imzalayici(imzalayici);
    let yedekli = if sunucu_acik(ayar) {
        None
    } else {
        match taa_kimlik::yedekli_gonderici(&ayar.kok, None, ayar.sertifika.as_deref()) {
            Ok(y) => y,
            Err(e) => return kimlik_hata(e),
        }
    };
    let sunucu_anahtari = match taa_kimlik::sunucu_anahtari_oku(&ayar.kok) {
        Ok(a) => a,
        Err(e) => return kimlik_hata(e),
    };
    let gonderici: &dyn taa_kimlik::Gonderici = match &yedekli {
        Some(y) => y,
        None => &g,
    };
    let o = match taa_kimlik::giris_dogrulayarak(
        gonderici,
        &anahtar_ad,
        k,
        p,
        kod,
        sunucu_anahtari.as_ref(),
    ) {
        Ok(o) => o,
        Err(e) => return kimlik_hata(e),
    };
    if let Err(e) = taa_kimlik::varsayilan_depo(&ayar.kok).kaydet(&o) {
        return kimlik_hata(e);
    }
    let yetki = baglan_oturumla(ayar, &o, &h, |g| {
        taa_kimlik::yetki_al(g, &ayar.kok, &anahtar_ad, sunucu_anahtari.as_ref()).is_ok()
    });
    let mut v = panel::oturum_ozeti(&o);
    v["yetki_belgesi"] = Value::Bool(yetki);
    (200, v)
}

fn baglan_oturumla(
    ayar: &Ayar,
    o: &taa_kimlik::Oturum,
    h: &taa_yol::Hedef,
    f: impl FnOnce(&dyn taa_kimlik::Gonderici) -> bool,
) -> bool {
    let tek = || {
        o.gonderici_taban(
            &h.taban,
            h.sertifika.as_deref(),
            taa_kimlik::cihaz_imzalayici(&ayar.kok).ok().flatten(),
        )
    };
    if !sunucu_acik(ayar) {
        if let Ok(Some(y)) =
            taa_kimlik::yedekli_gonderici(&ayar.kok, Some(o), ayar.sertifika.as_deref())
        {
            return f(&y);
        }
    }
    match tek() {
        Ok(g) => f(&g),
        Err(_) => false,
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
