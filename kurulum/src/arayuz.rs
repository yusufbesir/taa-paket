// Sahip: Yusuf. yazan: [Claude#kur-ekrani 2026-11-15]
//! Kurulum ekranı: yalnız 127.0.0.1'de küçük HTTP sunucusu + tek HTML sayfa (std-only). İstek ayrıştırma ve
//! yönlendirme saf ve testli; sunucu iş parçacığı ince katman. Bkz. docs/KURULUM-EKRANI.md.

use crate::ilerleme::Ilerleme;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{mpsc, Arc, Mutex};
use std::time::Duration;

const SAYFA: &str = include_str!("arayuz.html");
const BASLIK_SINIRI: usize = 16 * 1024;
const GOVDE_SINIRI: usize = 4 * 1024;

/// Sayfadan kurucuya giden komut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Komut {
    YenidenDene(String),
    Bitir,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Istek {
    pub yontem: String,
    pub yol: String,
    pub sorgu: String,
    pub host: String,
    pub jeton: String,
    pub govde: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yanit {
    pub kod: u16,
    pub tur: &'static str,
    pub govde: Vec<u8>,
}

impl Yanit {
    fn metin(kod: u16, m: &str) -> Self {
        Yanit { kod, tur: "text/plain; charset=utf-8", govde: m.as_bytes().to_vec() }
    }
    fn json(s: String) -> Self {
        Yanit { kod: 200, tur: "application/json; charset=utf-8", govde: s.into_bytes() }
    }
    pub fn bayt(&self) -> Vec<u8> {
        let neden = match self.kod {
            200 => "OK",
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            409 => "Conflict",
            413 => "Payload Too Large",
            _ => "Error",
        };
        let mut b = format!(
            "HTTP/1.1 {} {neden}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\n\
             X-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nX-Frame-Options: DENY\r\n\
             Content-Security-Policy: default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; \
             connect-src 'self'; img-src data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'\r\n\
             Connection: close\r\n\r\n",
            self.kod,
            self.tur,
            self.govde.len()
        )
        .into_bytes();
        b.extend_from_slice(&self.govde);
        b
    }
}

/// Başlık bloğu tamamsa isteği çözer. `Ok(None)`: daha veri gerek. `Err`: bozuk ya da sınır aşıldı.
pub fn istek_coz(ham: &[u8]) -> Result<Option<Istek>, u16> {
    let Some(son) = ham.windows(4).position(|w| w == b"\r\n\r\n") else {
        return if ham.len() > BASLIK_SINIRI { Err(413) } else { Ok(None) };
    };
    if son > BASLIK_SINIRI {
        return Err(413);
    }
    let baslik = std::str::from_utf8(&ham[..son]).map_err(|_| 400u16)?;
    let mut satirlar = baslik.split("\r\n");
    let mut ilk = satirlar.next().unwrap_or("").split(' ');
    let (yontem, hedef, surum) = (ilk.next().unwrap_or(""), ilk.next().unwrap_or(""), ilk.next().unwrap_or(""));
    if yontem.is_empty() || !hedef.starts_with('/') || !surum.starts_with("HTTP/1.") {
        return Err(400);
    }
    let (yol, sorgu) = hedef.split_once('?').unwrap_or((hedef, ""));
    let mut i = Istek { yontem: yontem.into(), yol: yol.into(), sorgu: sorgu.into(), ..Istek::default() };
    let mut uzunluk = 0usize;
    for s in satirlar {
        let Some((a, d)) = s.split_once(':') else { return Err(400) };
        let d = d.trim();
        match a.trim().to_ascii_lowercase().as_str() {
            "host" => i.host = d.to_string(),
            "x-taa-jeton" => i.jeton = d.to_string(),
            "content-length" => uzunluk = d.parse().map_err(|_| 400u16)?,
            "transfer-encoding" => return Err(400),
            _ => {}
        }
    }
    if uzunluk > GOVDE_SINIRI {
        return Err(413);
    }
    let govde = &ham[son + 4..];
    if govde.len() < uzunluk {
        return Ok(None);
    }
    i.govde = String::from_utf8_lossy(&govde[..uzunluk]).into_owned();
    Ok(Some(i))
}

/// Sunucunun sabit bağlamı.
pub struct Baglam {
    pub kapi: u16,
    pub jeton: String,
    pub kartlar_json: String,
}

impl Baglam {
    fn host_gecerli(&self, h: &str) -> bool {
        h == format!("127.0.0.1:{}", self.kapi) || h == format!("localhost:{}", self.kapi)
    }
}

fn sorgu_degeri<'a>(sorgu: &'a str, ad: &str) -> Option<&'a str> {
    sorgu.split('&').find_map(|p| p.split_once('=').filter(|(a, _)| *a == ad).map(|(_, d)| d))
}

/// Saf yönlendirme: yanıt ve (varsa) kurucuya iletilecek komut.
pub fn yonlendir(i: &Istek, b: &Baglam, durum: &Ilerleme) -> (Yanit, Option<Komut>) {
    // DNS yeniden bağlama: başka ad üzerinden gelen istek reddedilir.
    if !b.host_gecerli(&i.host) {
        return (Yanit::metin(403, "Yalnız yerel adres."), None);
    }
    match (i.yontem.as_str(), i.yol.as_str()) {
        ("GET", "/") => {
            let html = SAYFA.replace("__JETON__", &b.jeton);
            (Yanit { kod: 200, tur: "text/html; charset=utf-8", govde: html.into_bytes() }, None)
        }
        ("GET", "/durum") => {
            let sonra = sorgu_degeri(&i.sorgu, "sonra").and_then(|s| s.parse().ok()).unwrap_or(0);
            (Yanit::json(durum.json(sonra)), None)
        }
        ("GET", "/kartlar") => (Yanit::json(b.kartlar_json.clone()), None),
        ("POST", "/yeniden-dene") | ("POST", "/bitir") if i.jeton != b.jeton || b.jeton.is_empty() => {
            (Yanit::metin(403, "Geçersiz jeton."), None)
        }
        ("POST", "/yeniden-dene") => {
            let ad = i.govde.trim();
            if !durum.komut_bekleniyor {
                return (Yanit::metin(409, "Kurulum sürüyor; programlar bitince yeniden deneyin."), None);
            }
            if !durum.hatalilar().iter().any(|h| h == ad) {
                return (Yanit::metin(400, "Bu program yeniden denenemez."), None);
            }
            (Yanit::metin(200, "tamam"), Some(Komut::YenidenDene(ad.to_string())))
        }
        ("POST", "/bitir") => (Yanit::metin(200, "tamam"), Some(Komut::Bitir)),
        (_, "/" | "/durum" | "/kartlar" | "/yeniden-dene" | "/bitir") => (Yanit::metin(405, "Yöntem desteklenmiyor."), None),
        _ => (Yanit::metin(404, "Bulunamadı."), None),
    }
}

/// Tahmin edilemeyen jeton (std-only: RandomState her süreçte rastgele anahtarlanır).
pub fn jeton_uret() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::BuildHasher;
    let zaman = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0);
    let a = RandomState::new().hash_one((zaman, std::process::id()));
    let b = RandomState::new().hash_one((a, zaman.rotate_left(17)));
    format!("{a:016x}{b:016x}")
}

fn baglanti(mut s: TcpStream, b: &Baglam, durum: &Mutex<Ilerleme>, komut: &mpsc::Sender<Komut>) {
    let _ = s.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = s.set_write_timeout(Some(Duration::from_secs(3)));
    let mut ham = Vec::with_capacity(1024);
    let mut tampon = [0u8; 2048];
    let istek = loop {
        match istek_coz(&ham) {
            Ok(Some(i)) => break Ok(i),
            Err(k) => break Err(k),
            Ok(None) => {}
        }
        match s.read(&mut tampon) {
            Ok(0) | Err(_) => return,
            Ok(n) => ham.extend_from_slice(&tampon[..n]),
        }
    };
    let yanit = match istek {
        Err(k) => Yanit::metin(k, "Hatalı istek."),
        Ok(i) => {
            let (y, k) = match durum.lock() {
                Ok(d) => yonlendir(&i, b, &d),
                Err(_) => (Yanit::metin(409, "Durum okunamadı."), None),
            };
            if let Some(k) = k {
                let _ = komut.send(k);
            }
            y
        }
    };
    let _ = s.write_all(&yanit.bayt());
    let _ = s.flush();
}

/// Sunucuyu arka planda başlatır; sayfa adresini döndürür.
pub fn baslat(durum: Arc<Mutex<Ilerleme>>, komut: mpsc::Sender<Komut>) -> std::io::Result<String> {
    let dinleyici = TcpListener::bind("127.0.0.1:0")?;
    let kapi = dinleyici.local_addr()?.port();
    let baglam = Arc::new(Baglam { kapi, jeton: jeton_uret(), kartlar_json: crate::kartlar::json(&crate::kartlar::gomulu()) });
    std::thread::spawn(move || {
        for s in dinleyici.incoming().flatten() {
            let (b, d, k) = (Arc::clone(&baglam), Arc::clone(&durum), komut.clone());
            std::thread::spawn(move || baglanti(s, &b, &d, &k));
        }
    });
    Ok(format!("http://127.0.0.1:{kapi}/"))
}

/// Varsayılan tarayıcıda açar. Açılamazsa `false` (adres konsolda kalır).
pub fn tarayicida_ac(adres: &str) -> bool {
    use std::process::{Command, Stdio};
    #[cfg(windows)]
    let k = {
        use std::os::windows::process::CommandExt;
        // `start` ilk tırnaklı argümanı pencere başlığı sayar: boş başlık verilir. Adreste & yok.
        Command::new("cmd").args(["/C", "start", "", adres]).creation_flags(0x0800_0000).stdout(Stdio::null()).stderr(Stdio::null()).status()
    };
    #[cfg(target_os = "macos")]
    let k = Command::new("open").arg(adres).stdout(Stdio::null()).stderr(Stdio::null()).status();
    #[cfg(all(unix, not(target_os = "macos")))]
    let k = Command::new("xdg-open").arg(adres).stdout(Stdio::null()).stderr(Stdio::null()).status();
    k.map(|s| s.success()).unwrap_or(false)
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::ilerleme::Asama;

    fn baglam() -> Baglam {
        Baglam { kapi: 4321, jeton: "j123".into(), kartlar_json: "[]".into() }
    }

    fn istek(y: &str, yol: &str, jeton: &str, govde: &str) -> Istek {
        let (yol, sorgu) = yol.split_once('?').unwrap_or((yol, ""));
        Istek { yontem: y.into(), yol: yol.into(), sorgu: sorgu.into(), host: "127.0.0.1:4321".into(), jeton: jeton.into(), govde: govde.into() }
    }

    #[test]
    fn istek_ayristirma() {
        assert_eq!(istek_coz(b"GET / HTTP/1.1\r\nHost: x"), Ok(None));
        let i = istek_coz(b"GET /durum?sonra=5 HTTP/1.1\r\nHost: 127.0.0.1:9\r\nX-TAA-Jeton: ab\r\n\r\n").unwrap().unwrap();
        assert_eq!((i.yontem.as_str(), i.yol.as_str(), i.sorgu.as_str()), ("GET", "/durum", "sonra=5"));
        assert_eq!((i.host.as_str(), i.jeton.as_str()), ("127.0.0.1:9", "ab"));
        let yarim = b"POST /yeniden-dene HTTP/1.1\r\nHost: h\r\nContent-Length: 5\r\n\r\ntu";
        assert_eq!(istek_coz(yarim), Ok(None));
        let tam = b"POST /yeniden-dene HTTP/1.1\r\nHost: h\r\nContent-Length: 5\r\n\r\ntutaa";
        assert_eq!(istek_coz(tam).unwrap().unwrap().govde, "tutaa");
        assert_eq!(istek_coz(b"BOZUK\r\n\r\n"), Err(400));
        assert_eq!(istek_coz(b"GET x HTTP/1.1\r\n\r\n"), Err(400));
        assert_eq!(istek_coz(b"GET / HTTP/1.1\r\nbaslik-yok\r\n\r\n"), Err(400));
        assert_eq!(istek_coz(b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n"), Err(400));
        assert_eq!(istek_coz(b"POST / HTTP/1.1\r\nContent-Length: 99999\r\n\r\n"), Err(413));
        assert_eq!(istek_coz(&vec![b'a'; BASLIK_SINIRI + 1]), Err(413));
    }

    #[test]
    fn yonlendirme() {
        let b = baglam();
        let mut d = Ilerleme::yeni("1.0");
        let (y, k) = yonlendir(&istek("GET", "/", "", ""), &b, &d);
        assert_eq!((y.kod, k), (200, None));
        let html = String::from_utf8(y.govde).unwrap();
        assert!(html.contains("j123") && !html.contains("__JETON__"));
        assert!(!html.contains("http://") && !html.contains("https://"), "sayfa dış kaynak içermemeli");
        let (y, _) = yonlendir(&istek("GET", "/durum?sonra=0", "", ""), &b, &d);
        assert!(String::from_utf8(y.govde).unwrap().starts_with("{\"surum\":\"1.0\""));
        assert_eq!(yonlendir(&istek("GET", "/kartlar", "", ""), &b, &d).0.govde, b"[]");
        assert_eq!(yonlendir(&istek("GET", "/yok", "", ""), &b, &d).0.kod, 404);
        assert_eq!(yonlendir(&istek("DELETE", "/durum", "", ""), &b, &d).0.kod, 405);
        // Host denetimi.
        let mut yabanci = istek("GET", "/", "", "");
        yabanci.host = "kotu.ornek:4321".into();
        assert_eq!(yonlendir(&yabanci, &b, &d).0.kod, 403);
        let mut yerel = istek("GET", "/", "", "");
        yerel.host = "localhost:4321".into();
        assert_eq!(yonlendir(&yerel, &b, &d).0.kod, 200);
        // Jeton.
        assert_eq!(yonlendir(&istek("POST", "/bitir", "", ""), &b, &d), (Yanit::metin(403, "Geçersiz jeton."), None));
        assert_eq!(yonlendir(&istek("POST", "/bitir", "j123", ""), &b, &d).1, Some(Komut::Bitir));
        // Yeniden dene: yalnız komut beklenirken ve hatalı program için.
        d.program_ekle("tutaa", false);
        d.program_hata("tutaa", "ağ yok");
        assert_eq!(yonlendir(&istek("POST", "/yeniden-dene", "j123", "tutaa"), &b, &d).0.kod, 409);
        d.asama(Asama::Bitti, "bitti");
        d.komut_bekleniyor = true;
        assert_eq!(yonlendir(&istek("POST", "/yeniden-dene", "j123", "yok"), &b, &d).0.kod, 400);
        assert_eq!(yonlendir(&istek("POST", "/yeniden-dene", "j123", "tutaa\n"), &b, &d).1, Some(Komut::YenidenDene("tutaa".into())));
        assert_eq!(yonlendir(&istek("POST", "/yeniden-dene", "yanlis", "tutaa"), &b, &d).0.kod, 403);
    }

    #[test]
    fn bos_jeton_kabul_edilmez() {
        let b = Baglam { jeton: String::new(), ..baglam() };
        assert_eq!(yonlendir(&istek("POST", "/bitir", "", ""), &b, &Ilerleme::yeni("1")).0.kod, 403);
    }

    #[test]
    fn jeton_ve_yanit_baytlari() {
        let (a, b) = (jeton_uret(), jeton_uret());
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
        let y = String::from_utf8(Yanit::metin(200, "ok").bayt()).unwrap();
        assert!(y.starts_with("HTTP/1.1 200 OK\r\n"));
        assert!(y.contains("Content-Length: 2\r\n") && y.contains("Content-Security-Policy: default-src 'none'"));
        assert!(y.ends_with("\r\n\r\nok"));
    }

    #[test]
    fn gercek_sunucu_uctan_uca() {
        let d = Arc::new(Mutex::new(Ilerleme::yeni("9.9")));
        let (tx, rx) = mpsc::channel();
        let adres = baslat(Arc::clone(&d), tx).unwrap();
        let hostport = adres.trim_start_matches("http://").trim_end_matches('/').to_string();
        let iste = |istek: String| {
            let mut s = TcpStream::connect(&hostport).unwrap();
            s.write_all(istek.as_bytes()).unwrap();
            let mut c = String::new();
            s.read_to_string(&mut c).unwrap();
            c
        };
        let c = iste(format!("GET /durum HTTP/1.1\r\nHost: {hostport}\r\n\r\n"));
        assert!(c.starts_with("HTTP/1.1 200") && c.contains("\"surum\":\"9.9\""));
        let sayfa = iste(format!("GET / HTTP/1.1\r\nHost: {hostport}\r\n\r\n"));
        let jeton = sayfa.split("name=\"taa-jeton\" content=\"").nth(1).unwrap().split('"').next().unwrap().to_string();
        let c = iste(format!("POST /bitir HTTP/1.1\r\nHost: {hostport}\r\nX-TAA-Jeton: {jeton}\r\nContent-Length: 0\r\n\r\n"));
        assert!(c.starts_with("HTTP/1.1 200"));
        assert_eq!(rx.recv_timeout(Duration::from_secs(5)).unwrap(), Komut::Bitir);
    }
}
