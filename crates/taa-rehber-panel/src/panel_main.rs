// Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]
//! `taa-rehber-panel`: TAA Rehber masaüstü paneli.
//!
//! 127.0.0.1'de rastgele kapıda yerel bir sunucu açar, Edge'i uygulama penceresi olarak (`--app`) başlatır. Pencere
//! kapanıp istek gelmeyince kendiliğinden kapanır (taa-cati paneliyle aynı kalıp). Tüm iş mantığı kütüphanededir
//! (`taa_rehber_panel::yonlendir`, saf işlev); bu ikili yalnız HTTP, pencere ve `taa-kimlik` yönetici girişini bağlar.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

use serde_json::{json, Value};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};
use taa_rehber_panel::egitim::video::Karar;
use taa_rehber_panel::egitim::{self, kullanici_olustur, Kullanici, VideoYaniti, YetkiBilgisi};
use taa_rehber_panel::guvenlik::{self, IstekBilgi};
use taa_rehber_panel::{
    arayuz_html, yonlendir, Durum, Istek, Kimlik, KimlikBilgisi, KimlikRed, SistemOrtami,
    Yapilandirma, SIMGE_SVG,
};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};

/// Eğitim bölümü oturumunun "açık programlar" listesindeki adı (Eğitim ayrı program değil: TAA Rehber'in bölümü).
const PROGRAM_ADI: &str = "taarehber";
/// Oturum okuma önbelleği (video parça istekleri sık gelir).
const OTURUM_ONBELLEK: Duration = Duration::from_secs(5);

const YARDIM: &str = "\
taa-rehber-panel — TAA Rehber masaüstü paneli (Kişiler ve Görevler, El Kitabı, İhbar Hattı, Eğitim).
Kullanım: taa-rehber-panel [--sube <kod>] [--demo] [--sunucu <https://adres>] [--kok <veri kökü>] [--sertifika <pem>]
                          [--katalog <katalog.json>] [--rehber-onbellek <rehber.json>] [--video-klasoru <klasör>] [--pencere-acma]
  --sube <kod>    bu bilgisayarın şubesi (rehber kapsamı); verilmezse yalnız merkez kapsamı görünür.
  --katalog       Eğitim kataloğu (varsayılan <kök>/egitim/katalog.json); yoksa eğitim listesi boş görünür.
  --rehber-onbellek  Eğitim bölümünün okuduğu rehber önbelleği (varsayılan <kök>/rehber/rehber.json).
  --video-klasoru yerel eğitim videolarının klasörü (varsayılan <kök>/egitim/video).
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
    katalog: Option<PathBuf>,
    rehber_onbellek: Option<PathBuf>,
    video_klasoru: Option<PathBuf>,
}

#[derive(Default)]
struct OturumOnbellegi {
    sonuc: Option<(Instant, Option<Kullanici>)>,
}

impl OturumOnbellegi {
    fn unut(&mut self) {
        self.sonuc = None;
    }
}

/// Eğitim bölümünün durumu: kurulamazsa (bozuk katalog gibi) Rehber açılır, Eğitim nedenini bildirir.
struct EgitimBolumu {
    panel: Result<egitim::Panel, String>,
    onbellek: OturumOnbellegi,
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
    let (mut katalog, mut rehber_onbellek, mut video) = (None, None, None);
    let (mut pencere, mut demo) = (true, false);
    let mut i = 0;
    while i < args.len() {
        let deger = |i: usize| args.get(i + 1).cloned();
        match args[i].as_str() {
            "--sunucu" => sunucu = deger(i),
            "--kok" => kok = deger(i).map(PathBuf::from),
            "--sertifika" => sertifika = deger(i).map(PathBuf::from),
            "--sube" => sube = deger(i).filter(|s| !s.trim().is_empty()),
            "--katalog" => katalog = deger(i).map(PathBuf::from),
            "--rehber-onbellek" => rehber_onbellek = deger(i).map(PathBuf::from),
            "--video-klasoru" => video = deger(i).map(PathBuf::from),
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
        katalog,
        rehber_onbellek,
        video_klasoru: video,
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
    let mut egitim = EgitimBolumu {
        panel: egitim_kur(ayar),
        onbellek: OturumOnbellegi::default(),
    };
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
        if isle(
            istek,
            &mut durum,
            &mut egitim,
            ayar,
            kapi,
            &acilis,
            &cerez,
            &mut acildi,
        ) == Some(true)
        {
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
#[allow(clippy::too_many_arguments)]
fn isle(
    mut istek: Request,
    durum: &mut Durum,
    egitim: &mut EgitimBolumu,
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

    let simdi = egitim::zaman::simdi_iso();
    match (yontem, yol.as_str()) {
        ("GET", y) if y.starts_with("/video/") => {
            let k = egitim_kullanici(ayar, &mut egitim.onbellek, &simdi);
            let id = &y["/video/".len()..];
            let Ok(panel) = &egitim.panel else {
                json_yanit(istek, egitim_kapali(&egitim.panel));
                return Some(true);
            };
            match panel.video_hazirla(k.as_ref(), id, baslik(&istek, "Range")) {
                VideoYaniti::Hata(r) => json_yanit(istek, r),
                VideoYaniti::Dosya { yol, tur, karar } => video_yanitla(istek, &yol, tur, karar),
            }
        }
        (_, y) if y.starts_with("/api/egitim/") => {
            let r = egitim_api(egitim, ayar, yontem, y, &sorgu, &govde, &simdi);
            json_yanit(istek, r);
        }
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

/// Video yanıtı: dosyadan yalnız istenen aralık akıtılır (Range, 206/416).
fn video_yanitla(istek: Request, yol: &Path, tur: &str, karar: Karar) {
    let Some(aralik) = karar.icerik_araligi() else {
        // Tam içerik.
        return match File::open(yol) {
            Ok(f) => video_gonder(istek, f, tur, karar, None),
            Err(_) => json_yanit(istek, (404, json!({"hata": "Video bulunamadı.", "kod": 2}))),
        };
    };
    if let Karar::Karsilanamaz { .. } = karar {
        let mut y = Response::new(StatusCode(416), vec![], std::io::empty(), Some(0), None);
        if let Ok(h) = Header::from_bytes(&b"Content-Range"[..], aralik.as_bytes()) {
            y.add_header(h);
        }
        return yanitla(istek, y, tur);
    }
    let bas = match karar {
        Karar::Kismi { bas, .. } => bas,
        _ => 0,
    };
    match File::open(yol).and_then(|mut f| f.seek(SeekFrom::Start(bas)).map(|_| f)) {
        Ok(f) => video_gonder(istek, f, tur, karar, Some(aralik)),
        Err(_) => json_yanit(istek, (404, json!({"hata": "Video bulunamadı.", "kod": 2}))),
    }
}

fn video_gonder(istek: Request, f: File, tur: &str, karar: Karar, aralik: Option<String>) {
    let uzunluk = karar.uzunluk();
    let mut y = Response::new(
        StatusCode(karar.durum_kodu()),
        vec![],
        f.take(uzunluk),
        usize::try_from(uzunluk).ok(),
        None,
    );
    if let Ok(h) = Header::from_bytes(&b"Accept-Ranges"[..], &b"bytes"[..]) {
        y.add_header(h);
    }
    if let Some(a) = aralik {
        if let Ok(h) = Header::from_bytes(&b"Content-Range"[..], a.as_bytes()) {
            y.add_header(h);
        }
    }
    yanitla(istek, y, tur);
}

// ---------------------------------------------------------------------------------------------- eğitim kimliği (taa-kimlik)

fn kimlik_hata(e: taa_kimlik::KimlikHata) -> Yanit {
    use taa_kimlik::KimlikHata as K;
    let (durum, kod) = match &e {
        K::GecersizGirdi(_) => (400, 2),
        K::GirisReddi { .. } | K::OturumYok | K::YetkiYok(_) => (401, 5),
        K::Baglanti(_) | K::BozukYanit(_) => (502, 3),
        K::Depo(_) => (500, 1),
    };
    (durum, json!({"hata": e.to_string(), "kod": kod}))
}

fn yol_hata(e: taa_yol::YolHata) -> Yanit {
    let k = taa_yol::hata_kodu(&e);
    let durum = match k {
        2 => 400,
        3 => 502,
        5 => 401,
        _ => 500,
    };
    (durum, json!({"hata": e.to_string(), "kod": k}))
}

fn sunucu_acik(ayar: &Ayar) -> bool {
    ayar.sunucu.is_some() || std::env::var("TAA_SUNUCU").is_ok_and(|v| !v.trim().is_empty())
}

/// Saklı oturumdan kullanıcı. Oturum yok, süresi geçmiş ya da bu program kapalıysa None (giriş gerekir).
/// Yönetici bayrağı yalnız imzası doğrulanmış, süresi geçmemiş, aynı kullanıcıya ait yetki belgesinden gelir.
fn egitim_kullanici_oku(ayar: &Ayar, simdi: &str) -> Option<Kullanici> {
    let anahtar = taa_yol::anahtar_coz_ortam(&ayar.kok, ayar.sunucu.as_deref()).ok()?;
    let o = taa_kimlik::varsayilan_depo(&ayar.kok).oku(&anahtar).ok()?;
    if (!o.gecerlilik.is_empty() && simdi >= o.gecerlilik.as_str()) || !o.program_acik(PROGRAM_ADI)
    {
        return None;
    }
    let sunucu_anahtari = taa_kimlik::sunucu_anahtari_oku(&ayar.kok).ok().flatten();
    let yetki = taa_kimlik::yetki_oku(&ayar.kok, &anahtar, sunucu_anahtari.as_ref(), Some(simdi))
        .ok()
        .map(|y| YetkiBilgisi {
            kullanici: y.kullanici,
            izinler: y.izinler,
            dogrulandi: y.dogrulandi,
            gecerlilik: y.gecerlilik,
        });
    Some(kullanici_olustur(&o.kullanici, yetki.as_ref(), simdi))
}

fn egitim_kullanici(ayar: &Ayar, onbellek: &mut OturumOnbellegi, simdi: &str) -> Option<Kullanici> {
    if let Some((an, k)) = &onbellek.sonuc {
        if an.elapsed() < OTURUM_ONBELLEK {
            return k.clone();
        }
    }
    let k = egitim_kullanici_oku(ayar, simdi);
    onbellek.sonuc = Some((Instant::now(), k.clone()));
    k
}

fn egitim_cikis(ayar: &Ayar) -> Yanit {
    let anahtar = match taa_yol::anahtar_coz_ortam(&ayar.kok, ayar.sunucu.as_deref()) {
        Ok(a) => a,
        Err(e) => return yol_hata(e),
    };
    match taa_kimlik::varsayilan_depo(&ayar.kok).cikis(&anahtar) {
        Ok(()) => {
            // Yetki belgesi boşaltılır (en iyi çaba; silme yok, taa-kimlik çıkışıyla aynı kural): yönetici izni
            // oturumla birlikte biter, boş belge doğrulanamaz sayılır.
            let yol = taa_kimlik::yetki::yetki_dosyasi(&ayar.kok, &anahtar);
            if yol.exists() {
                let _ = std::fs::write(&yol, b"");
            }
            (200, json!({"tamam": true}))
        }
        Err(e) => kimlik_hata(e),
    }
}

/// `taa-kimlik giris` ile aynı akış: cihaz imzalı giriş, sunucu yanıt imzası (anahtar varsa) doğrulanır, oturum
/// depoya yazılır, çevrimdışı yetki belgesi alınır (başarısızlığı girişi bozmaz ama yönetici işlemleri açılmaz).
/// Parola yalnız bu istek süresince bellekte durur; yanıtta dönmez, hiçbir yere yazılmaz.
fn egitim_giris(ayar: &Ayar, govde: &Value, simdi: &str) -> Yanit {
    let (Some(k), Some(p)) = (
        govde.get("kullanici").and_then(Value::as_str),
        govde.get("parola").and_then(Value::as_str),
    ) else {
        return (
            400,
            json!({"hata": "kullanıcı adı ve parola gerekli", "kod": 2}),
        );
    };
    let kod = govde
        .get("kod")
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty());
    let anahtar_ad = match taa_yol::anahtar_coz_ortam(&ayar.kok, ayar.sunucu.as_deref()) {
        Ok(a) => a,
        Err(e) => return yol_hata(e),
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
    let yetki = oturumla(ayar, &o, &h, |g| {
        taa_kimlik::yetki_al(g, &ayar.kok, &anahtar_ad, sunucu_anahtari.as_ref()).is_ok()
    });
    (
        200,
        json!({"girisli": true, "kullanici": o.kullanici, "yetki_belgesi": yetki, "zaman": simdi}),
    )
}

fn oturumla(
    ayar: &Ayar,
    o: &taa_kimlik::Oturum,
    h: &taa_yol::Hedef,
    f: impl FnOnce(&dyn taa_kimlik::Gonderici) -> bool,
) -> bool {
    if !sunucu_acik(ayar) {
        if let Ok(Some(y)) =
            taa_kimlik::yedekli_gonderici(&ayar.kok, Some(o), ayar.sertifika.as_deref())
        {
            return f(&y);
        }
    }
    match o.gonderici_taban(
        &h.taban,
        h.sertifika.as_deref(),
        taa_kimlik::cihaz_imzalayici(&ayar.kok).ok().flatten(),
    ) {
        Ok(g) => f(&g),
        Err(_) => false,
    }
}

type Yanit = (u16, Value);

fn egitim_kapali(p: &Result<egitim::Panel, String>) -> Yanit {
    let m = p.as_ref().err().cloned().unwrap_or_default();
    (
        503,
        json!({"hata": format!("Eğitim bölümü açılamadı: {m}"), "kod": 1}),
    )
}

/// Katalog, rehber önbelleği ve izleme günlüğünü yükler. Dosya yoksa boş başlanır ve ekranda uyarı çıkar;
/// bozuk katalog yalnız Eğitim bölümünü kapatır (Rehber açılır).
fn egitim_kur(ayar: &Ayar) -> Result<egitim::Panel, String> {
    let egitim_klasoru = ayar.kok.join("egitim");
    let video_kok = ayar
        .video_klasoru
        .clone()
        .unwrap_or_else(|| egitim_klasoru.join("video"));
    if ayar.demo {
        let uyari = vec![
            "ÖRNEK VERİ (--demo): katalog ve rehber örnektir; hiçbir şey kaydedilmez, video yoktur.".to_string(),
            "Yönetici işlemleri demo kipinde de yalnız gerçek yetkiyle açılır.".to_string(),
        ];
        return egitim::Panel::kur(
            egitim::veri::demo_katalog(),
            egitim::veri::demo_rehber(),
            video_kok,
            None,
            true,
            None,
            uyari,
        );
    }
    let mut uyarilar = Vec::new();
    let katalog_yolu = ayar
        .katalog
        .clone()
        .unwrap_or_else(|| egitim_klasoru.join("katalog.json"));
    let katalog = match std::fs::read_to_string(&katalog_yolu) {
        Ok(m) => egitim::veri::katalog_yukle(&m)
            .map_err(|e| format!("{}: {e}", katalog_yolu.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            uyarilar.push(format!(
                "Katalog dosyası yok ({}); eğitim listesi boş.",
                katalog_yolu.display()
            ));
            egitim::veri::YuklenenKatalog::default()
        }
        Err(e) => return Err(format!("{} okunamadı: {e}", katalog_yolu.display())),
    };
    let rehber_yolu = ayar
        .rehber_onbellek
        .clone()
        .unwrap_or_else(|| ayar.kok.join("rehber").join("rehber.json"));
    let rehber = match std::fs::read_to_string(&rehber_yolu) {
        Ok(m) => egitim::veri::rehber_yukle(&m).unwrap_or_else(|e| {
            uyarilar.push(format!("Rehber kullanılamıyor, kimse etkin sayılmaz: {e}"));
            Default::default()
        }),
        Err(_) => {
            uyarilar.push(format!(
                "Rehber önbelleği yok ({}); kimse etkin sayılmaz, izleme kaydı ve atama yapılamaz.",
                rehber_yolu.display()
            ));
            Default::default()
        }
    };
    egitim::Panel::kur(
        katalog,
        rehber,
        video_kok,
        Some(&egitim_klasoru.join("izleme.jsonl")),
        false,
        None, // tema Rehber'in ortak görünüm ayarındadır
        uyarilar,
    )
}

/// `/api/egitim/<uç>` isteklerini Eğitim çekirdeğine (`/api/<uç>`) yönlendirir; giriş ve çıkış taa-kimlik oturumudur.
fn egitim_api(
    e: &mut EgitimBolumu,
    ayar: &Ayar,
    yontem: &str,
    yol: &str,
    sorgu: &str,
    govde: &Value,
    simdi: &str,
) -> Yanit {
    let ic = format!("/api/{}", &yol["/api/egitim/".len()..]);
    match (yontem, ic.as_str()) {
        ("POST", "/api/giris") => {
            let y = egitim_giris(ayar, govde, simdi);
            e.onbellek.unut();
            return y;
        }
        ("POST", "/api/cikis") => {
            let y = egitim_cikis(ayar);
            e.onbellek.unut();
            return y;
        }
        _ => {}
    }
    let k = egitim_kullanici(ayar, &mut e.onbellek, simdi);
    match &mut e.panel {
        Ok(p) => p.api(k.as_ref(), yontem, &ic, sorgu, govde, simdi),
        Err(_) => egitim_kapali(&e.panel),
    }
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
