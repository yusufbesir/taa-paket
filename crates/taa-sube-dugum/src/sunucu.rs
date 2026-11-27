// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-sube-dugum 2026-11-27]
//! HTTP kapısı: `POST /v1/<uç>`, `GET /v1/esitle`, `GET /durum`, `GET /v1/saglik`.
//!
//! İstek işleme ([`isle`]) ağdan bağımsız saf işlevdir (testler soketsiz çalışır); [`hizmet`] onu `tiny_http`ye bağlar.
//! Düz HTTP'dir (TLS yok; bkz. docs/SUBE-DUGUMU.md §5 ve "Uygulama" açık noktalar).
//!
//! Sınırlar (docs/SUBE-DUGUMU.md §7): `0.0.0.0` dinlenmez; bağlanan adres yerel ağda değilse istek kimlik
//! denetimine geçmeden reddedilir; `Host` yalnız düğümün bilinen adlarından biri olabilir (DNS yeniden bağlama);
//! `Origin` yalnız düğümün kendi kökeni ya da ayarda izin verilen kökenler olabilir.

use crate::{uc_kabul, Dugum, GelenDenetim, GelenIstek, Ret, Yazma, ESITLE_LIMIT_VARSAYILAN};
use serde_json::{json, Value};
use std::io::{self, Read};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use taa_kuyruk::zaman;

/// Ağ ayarı: düğümün kabul ettiği adlar ve kökenler.
#[derive(Debug, Clone, PartialEq)]
pub struct AgAyari {
    /// `Host` başlığında kabul edilen adlar (kapısız, küçük harf): `127.0.0.1`, `localhost`, LAN adresi, ek adlar.
    pub adlar: Vec<String>,
    /// Başka kökenden çağrıya izin verilen kökenler (örnek: merkez kabuğunun kökeni `https://sunucu.ornek.com`).
    pub izinli_kokenler: Vec<String>,
    pub govde_en_cok: usize,
}

impl AgAyari {
    /// Yalnız bu bilgisayar (`127.0.0.1`, `localhost`).
    pub fn yerel() -> AgAyari {
        AgAyari {
            adlar: vec!["127.0.0.1".into(), "localhost".into()],
            izinli_kokenler: Vec::new(),
            govde_en_cok: crate::GOVDE_EN_COK,
        }
    }

    pub fn ad_ekle(&mut self, ad: &str) {
        let a = ad.trim().to_ascii_lowercase();
        if !a.is_empty() && !self.adlar.contains(&a) {
            self.adlar.push(a);
        }
    }

    /// Köken `http(s)://ad[:kapı]` biçiminde olmalı (yol, sorgu, boşluk yok); değilse eklenmez ve `false` döner.
    pub fn koken_ekle(&mut self, koken: &str) -> bool {
        let k = koken.trim().trim_end_matches('/').to_ascii_lowercase();
        let govde = k
            .strip_prefix("https://")
            .or_else(|| k.strip_prefix("http://"));
        let gecerli = matches!(govde, Some(g) if !g.is_empty()
            && g.bytes().all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':' | b'[' | b']')));
        if gecerli && !self.izinli_kokenler.contains(&k) {
            self.izinli_kokenler.push(k);
        }
        gecerli
    }
}

/// Yerel ağ dinleme adresi olarak kabul edilir mi: yalnız özel IPv4 (10/8, 172.16/12, 192.168/16) ya da
/// bağlantı-yerel (169.254/16). `0.0.0.0`, genel adres ve Tailscale (100.64/10) reddedilir.
pub fn lan_adresi_gecerli(ip: Ipv4Addr) -> Result<(), String> {
    if ip.is_unspecified() {
        return Err(
            "0.0.0.0 dinlenmez; şube bilgisayarının yerel ağ adresini verin (örnek 192.168.1.20)"
                .into(),
        );
    }
    if ip.is_loopback() {
        return Err("127.0.0.1 zaten dinleniyor; --lan için yerel ağ adresi verin".into());
    }
    let o = ip.octets();
    if o[0] == 100 && (o[1] & 0xC0) == 64 {
        return Err(
            "Tailscale/CGNAT adresi (100.64.0.0/10) yerel ağ değildir; düğüm orada dinlemez".into(),
        );
    }
    if ip.is_private() || ip.is_link_local() {
        return Ok(());
    }
    Err(format!(
        "{ip} yerel ağ adresi değil; düğüm yalnız özel adreste (10.x, 172.16-31.x, 192.168.x) dinler"
    ))
}

/// Bağlanan adres yerel mi (bu bilgisayar ya da özel/bağlantı-yerel ağ).
pub fn uzak_izinli(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(v) => v.is_loopback() || v.is_private() || v.is_link_local(),
        IpAddr::V6(v) => {
            if let Some(v4) = v.to_ipv4_mapped() {
                return uzak_izinli(IpAddr::V4(v4));
            }
            let ilk = v.segments()[0];
            v.is_loopback() || (ilk & 0xfe00) == 0xfc00 || (ilk & 0xffc0) == 0xfe80
        }
    }
}

/// Ağdan bağımsız istek.
#[derive(Debug, Clone, Default)]
pub struct Istek {
    pub yontem: String,
    /// Sorgu dahil yol.
    pub yol: String,
    /// (ad, değer); ad küçük harf.
    pub basliklar: Vec<(String, String)>,
    pub govde: Vec<u8>,
    /// Bağlanan adres (bilinmiyorsa istek reddedilir).
    pub uzak: Option<IpAddr>,
    /// Gövde sınırı aştı (gövde okunmadı ya da kesildi).
    pub govde_asildi: bool,
}

impl Istek {
    pub fn baslik(&self, ad: &str) -> Option<&str> {
        self.basliklar
            .iter()
            .find(|(a, _)| a == ad)
            .map(|(_, d)| d.trim())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Yanit {
    pub kod: u16,
    /// `None`: gövdesiz (204).
    pub govde: Option<Value>,
    pub basliklar: Vec<(String, String)>,
}

fn hata(kod: u16, hata: &str, mesaj: &str) -> Yanit {
    Yanit {
        kod,
        govde: Some(json!({"hata": hata, "mesaj": mesaj})),
        basliklar: Vec::new(),
    }
}

fn tamam(govde: Value) -> Yanit {
    Yanit {
        kod: 200,
        govde: Some(govde),
        basliklar: Vec::new(),
    }
}

impl From<Ret> for Yanit {
    fn from(r: Ret) -> Yanit {
        hata(r.kod, r.hata, &r.mesaj)
    }
}

/// `Host` değerinden kapıyı atar (`[::1]:8788` → `[::1]`).
fn host_adi(host: &str) -> String {
    let h = host.trim().to_ascii_lowercase();
    if let Some(son) = h.rfind(']') {
        return h[..=son].to_string();
    }
    match h.rsplit_once(':') {
        Some((ad, kapi)) if kapi.bytes().all(|b| b.is_ascii_digit()) => ad.to_string(),
        _ => h,
    }
}

fn sorgu_degeri<'a>(sorgu: &'a str, ad: &str) -> Option<&'a str> {
    sorgu
        .split('&')
        .filter_map(|p| p.split_once('='))
        .find(|(a, _)| *a == ad)
        .map(|(_, d)| d)
}

const SUBE_MESAJ: &str = "Bu istek bu şube bilgisayarına ait değil.";

/// `X-TAA-Sube` düğümün şubesiyle aynı olmalı; değilse ret ve günlüğe `dugum.red`.
fn sube_denetle(d: &Dugum, i: &Istek, kaynak: &str) -> Result<(), Yanit> {
    match i.baslik("x-taa-sube") {
        Some(s) if s.eq_ignore_ascii_case(d.sube()) => Ok(()),
        Some(_) => {
            d.gunluk(
                "dugum.red",
                json!({"neden": "sube_uyusmaz", "yol": yol_kismi(&i.yol), "kaynak": kaynak}),
            );
            Err(hata(409, "sube_uyusmaz", SUBE_MESAJ))
        }
        None => {
            d.gunluk(
                "dugum.red",
                json!({"neden": "sube_eksik", "yol": yol_kismi(&i.yol), "kaynak": kaynak}),
            );
            Err(hata(
                400,
                "sube_eksik",
                "X-TAA-Sube başlığı gerekli. Öneri: uygulamayı güncelleyin.",
            ))
        }
    }
}

fn yol_kismi(yol: &str) -> &str {
    yol.split('?').next().unwrap_or(yol)
}

fn sunucu_hatasi(d: &Dugum, e: &dyn std::fmt::Display) -> Yanit {
    // Geçici durum için 4xx dönülmez: istemci kaydı bekletir, sonra yeniden dener.
    eprintln!("taa-sube-dugum: {e}");
    d.gunluk("dugum.hata", json!({"mesaj": e.to_string()}));
    hata(
        503,
        "gecici",
        "Şube bilgisayarı şu an yazamıyor. Kayıt cihazınızda bekliyor; biraz sonra yeniden denenecek.",
    )
}

fn yaz(d: &Dugum, i: &Istek, uc: &str, kaynak: &str) -> Yanit {
    if !uc_kabul(uc) {
        return hata(
            404,
            "uc_yok",
            "Bu işlem şube bilgisayarından yapılamaz; merkez bağlantısı gerekir.",
        );
    }
    if i.govde_asildi {
        d.gunluk(
            "dugum.red",
            json!({"neden": "govde_buyuk", "yol": uc, "kaynak": kaynak}),
        );
        return hata(413, "govde_buyuk", "Kayıt çok büyük.");
    }
    let mut govde: Value = match serde_json::from_slice(&i.govde) {
        Ok(v @ Value::Object(_)) => v,
        _ => return hata(400, "govde_bozuk", "Kayıt JSON nesnesi olmalı."),
    };
    let govde_no = match govde.get("no") {
        None => None,
        Some(Value::String(s)) => Some(s.clone()),
        Some(_) => return hata(400, "no_bozuk", "Kayıt numarası (no) metin olmalı."),
    };
    let baslik_no = i.baslik("x-taa-no").map(str::to_string);
    let no = match (baslik_no, govde_no) {
        (Some(b), Some(g)) if b != g => {
            return hata(
                400,
                "no_uyusmaz",
                "Başlıktaki ve gövdedeki kayıt numarası (no) aynı değil.",
            )
        }
        (Some(b), _) => b,
        (None, Some(g)) => g,
        (None, None) => {
            return hata(
                400,
                "no_eksik",
                "Kayıt numarası (no) gerekli: X-TAA-No başlığı ya da gövdede no.",
            )
        }
    };
    if !taa_kuyruk::no_gecerli(&no) {
        return hata(
            400,
            "no_bozuk",
            "Kayıt numarası (no) yalnız harf, rakam, nokta, tire ve alt çizgi içerebilir (en çok 64).",
        );
    }
    // Merkez ile aynı gövde: `no` gövdede yoksa eklenir (tarayıcı çekirdeği de böyle yapar).
    govde["no"] = Value::String(no.clone());
    for alan in ["sube", "sube_id", "sube_kodu"] {
        if let Some(s) = govde.get(alan).and_then(Value::as_str) {
            if s != d.sube() {
                d.gunluk(
                    "dugum.red",
                    json!({"neden": "sube_uyusmaz", "yol": uc, "no": no, "kaynak": kaynak}),
                );
                return hata(409, "sube_uyusmaz", SUBE_MESAJ);
            }
        }
    }
    match d.al(uc, &no, govde, kaynak) {
        Ok(Yazma::Yazildi { surum }) => tamam(json!({
            "no": no, "durum": "yazildi", "surum": surum, "kesin": false,
            "dugum": "sube", "sunucu_zamani": zaman::simdi(),
        })),
        Ok(Yazma::ZatenVar { surum, kesin }) => tamam(json!({
            "no": no, "durum": "zaten_var", "surum": surum, "kesin": kesin,
            "dugum": "sube", "sunucu_zamani": zaman::simdi(),
        })),
        Ok(Yazma::Cakisma) => hata(
            409,
            "cakisma",
            "Aynı numarayla farklı bir kayıt var. Şube bilgisayarındaki kayıt geçerli; bu kayıt çakışma kutusuna alındı.",
        ),
        Err(crate::DugumHata::Kuyruk(taa_kuyruk::KuyrukHata::GecersizGovde(_))) => {
            hata(413, "govde_buyuk", "Kayıt çok büyük.")
        }
        Err(e) => sunucu_hatasi(d, &e),
    }
}

fn esitle(d: &Dugum, sorgu: &str) -> Yanit {
    let sonra = match sorgu_degeri(sorgu, "sonra") {
        None | Some("") => 0,
        Some(s) => match s.parse::<u64>() {
            Ok(n) => n,
            Err(_) => return hata(400, "sonra_bozuk", "sonra değeri sayı olmalı."),
        },
    };
    let limit = match sorgu_degeri(sorgu, "limit") {
        None | Some("") => ESITLE_LIMIT_VARSAYILAN,
        Some(s) => match s.parse::<usize>() {
            Ok(n) if n >= 1 => n,
            _ => return hata(400, "limit_bozuk", "limit değeri 1-500 arası sayı olmalı."),
        },
    };
    match d.esitle(sonra, limit) {
        Ok(v) => tamam(v),
        Err(e) => sunucu_hatasi(d, &e),
    }
}

fn durum(d: &Dugum) -> Yanit {
    match d.durum() {
        Ok(o) => tamam(json!({
            "bekleyen": o.bekleyen,
            "gonderilen": o.gonderilen,
            "reddedilen": o.reddedilen,
            "son_merkez_zamani": o.son_merkez_zamani,
            "sube": d.sube(),
            "cakisma": o.cakisma,
            "surum": o.surum,
            "gunluk_bekleyen": o.gunluk_bekleyen,
            "sunucu_zamani": zaman::simdi(),
        })),
        Err(e) => sunucu_hatasi(d, &e),
    }
}

/// Bir isteği işler. Sıra: bağlanan adres → `Host` → `Origin` → (ön uçuş) → yol → şube → denetim kancası → iş.
pub fn isle(d: &Dugum, ag: &AgAyari, denetim: &dyn GelenDenetim, i: &Istek) -> Yanit {
    let kaynak = i.uzak.map(|a| a.to_string()).unwrap_or_default();
    if !i.uzak.is_some_and(uzak_izinli) {
        d.gunluk("dugum.red", json!({"neden": "ag_disi", "kaynak": kaynak}));
        return hata(
            403,
            "ag_disi",
            "Şube bilgisayarına yalnız şubenin yerel ağından ulaşılır.",
        );
    }
    let host = i.baslik("host").map(host_adi).unwrap_or_default();
    if !ag.adlar.contains(&host) {
        d.gunluk(
            "dugum.red",
            json!({"neden": "ad_uyusmaz", "kaynak": kaynak}),
        );
        return hata(
            403,
            "ad_uyusmaz",
            "Bu adres şube bilgisayarının adresi değil.",
        );
    }
    let mut ek: Vec<(String, String)> = Vec::new();
    if let Some(koken) = i.baslik("origin") {
        let k = koken.trim_end_matches('/').to_ascii_lowercase();
        let kendi = format!(
            "http://{}",
            i.baslik("host").unwrap_or_default().to_ascii_lowercase()
        );
        if k == kendi {
            // Aynı köken: CORS başlığı gerekmez.
        } else if ag.izinli_kokenler.contains(&k) {
            ek.push(("Access-Control-Allow-Origin".into(), k.clone()));
            ek.push(("Vary".into(), "Origin".into()));
        } else {
            d.gunluk(
                "dugum.red",
                json!({"neden": "koken_izinsiz", "kaynak": kaynak}),
            );
            return hata(
                403,
                "koken_izinsiz",
                "Bu sayfanın şube bilgisayarına yazma izni yok.",
            );
        }
    }
    let (yol, sorgu) = i.yol.split_once('?').unwrap_or((i.yol.as_str(), ""));
    let mut y = match (i.yontem.as_str(), yol) {
        ("OPTIONS", _) => {
            if !ek.is_empty() {
                ek.push((
                    "Access-Control-Allow-Methods".into(),
                    "GET, POST, OPTIONS".into(),
                ));
                ek.push((
                    "Access-Control-Allow-Headers".into(),
                    "Content-Type, Authorization, X-TAA-No, X-TAA-Sube, X-TAA-Cihaz, X-TAA-Zaman, X-TAA-Imza".into(),
                ));
                ek.push(("Access-Control-Max-Age".into(), "600".into()));
                if i.baslik("access-control-request-private-network") == Some("true") {
                    ek.push(("Access-Control-Allow-Private-Network".into(), "true".into()));
                }
            }
            Yanit {
                kod: 204,
                govde: None,
                basliklar: Vec::new(),
            }
        }
        ("GET", "/durum") => durum(d),
        ("GET", "/v1/saglik") => {
            tamam(json!({"durum": "calisiyor", "dugum": "sube", "sube": d.sube()}))
        }
        ("GET", "/v1/esitle") | ("POST", _) => {
            let gi = GelenIstek {
                yontem: &i.yontem,
                yol: &i.yol,
                basliklar: &i.basliklar,
                govde: &i.govde,
            };
            if let Err(r) = sube_denetle(d, i, &kaynak) {
                r
            } else if let Err(r) = denetim.denetle(&gi) {
                d.gunluk(
                    "dugum.red",
                    json!({"neden": r.hata, "yol": yol, "kaynak": kaynak}),
                );
                r.into()
            } else if i.yontem == "GET" {
                esitle(d, sorgu)
            } else {
                yaz(d, i, yol, &kaynak)
            }
        }
        ("GET", _) => hata(404, "uc_yok", "Böyle bir adres yok."),
        _ => hata(405, "yontem_yok", "Bu yöntem desteklenmiyor."),
    };
    y.basliklar.extend(ek);
    y
}

/// Verilen adreste dinler. Belirsiz adres (`0.0.0.0`, `::`) reddedilir.
pub fn dinle(adres: SocketAddr) -> io::Result<tiny_http::Server> {
    if adres.ip().is_unspecified() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "0.0.0.0 dinlenmez; açık bir adres verin",
        ));
    }
    tiny_http::Server::http(adres).map_err(|e| io::Error::other(e.to_string()))
}

fn istek_oku(r: &mut tiny_http::Request, en_cok: usize) -> Istek {
    let basliklar: Vec<(String, String)> = r
        .headers()
        .iter()
        .map(|h| {
            (
                h.field.as_str().as_str().to_ascii_lowercase(),
                h.value.as_str().to_string(),
            )
        })
        .collect();
    let mut i = Istek {
        yontem: r.method().as_str().to_ascii_uppercase(),
        yol: r.url().to_string(),
        basliklar,
        govde: Vec::new(),
        uzak: r.remote_addr().map(|a| a.ip()),
        govde_asildi: false,
    };
    if r.body_length().is_some_and(|n| n > en_cok) {
        i.govde_asildi = true;
        return i;
    }
    let mut tampon = Vec::new();
    let okunan = r
        .as_reader()
        .take(en_cok as u64 + 1)
        .read_to_end(&mut tampon);
    if okunan.is_err() || tampon.len() > en_cok {
        i.govde_asildi = true;
    } else {
        i.govde = tampon;
    }
    i
}

/// `dur` işaretlenene kadar istekleri karşılar. Birden çok iş parçacığı aynı sunucuda çağırabilir.
pub fn hizmet(
    sunucu: &tiny_http::Server,
    d: &Dugum,
    ag: &AgAyari,
    denetim: &dyn GelenDenetim,
    dur: &AtomicBool,
) {
    while !dur.load(Ordering::SeqCst) {
        let mut r = match sunucu.recv_timeout(Duration::from_millis(250)) {
            Ok(Some(r)) => r,
            Ok(None) => continue,
            Err(_) => break,
        };
        let i = istek_oku(&mut r, ag.govde_en_cok);
        let y = isle(d, ag, denetim, &i);
        let govde = y
            .govde
            .map(|v| v.to_string().into_bytes())
            .unwrap_or_default();
        let mut cevap = tiny_http::Response::from_data(govde).with_status_code(y.kod);
        let mut basliklar = y.basliklar;
        basliklar.push((
            "Content-Type".into(),
            "application/json; charset=utf-8".into(),
        ));
        basliklar.push(("Cache-Control".into(), "no-store".into()));
        basliklar.push(("X-Content-Type-Options".into(), "nosniff".into()));
        for (a, deger) in basliklar {
            if let Ok(h) = tiny_http::Header::from_bytes(a.as_bytes(), deger.as_bytes()) {
                cevap = cevap.with_header(h);
            }
        }
        let _ = r.respond(cevap);
    }
}
