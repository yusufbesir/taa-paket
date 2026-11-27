//! taa-yazdir: tarayıcıdaki TAA Web'in şube yazıcısına ham komut (PPLB, ESC/POS) gönderebilmesi için
//! şube PC'sinde çalışan küçük HTTP köprüsü. Sahip: Yusuf.
//! yazan: [Claude Fable 5.2 #taa-web-W2-yazdir 2026-11-27]. Plan: PLAN-TAA-WEB-2026-11-27.md §4.
//!
//! Kurallar:
//! - Yalnız `127.0.0.1`'i dinler; dinleme adresi ayarlanamaz (`dinle`). Karşı uç geri döngü değilse 403.
//! - `Host` başlığı `127.0.0.1` ya da `localhost` olmalı (DNS yeniden bağlama saldırısına karşı).
//! - `POST /yazdir` yalnız ayar dosyasındaki izinli kökenlerden (`Origin`) kabul edilir; `*` yazılamaz.
//! - Gövde boyutu sınırlıdır (`en_cok_bayt`); aşan istek okunmadan 413 alır.
//! - Cihaz imzası bir kancadır (`CihazDenetimi`); varsayılan kapalıdır. Kim imzalayacak kararı açıktır (BENIOKU).
//! - Yazıcıya gönderme yeni bir yol değildir: `taa-fis-sunucu`daki `yaziciya_yaz` ile aynı yoldur (dosya/paylaşım
//!   yoluna yazma ya da `tcp://IP[:9100]`); bkz. bu dosyadaki `yaziciya_yaz` notu. Hedefi ayarlanmamış
//!   sürücü ve `sistem` sürücüsü 501 "henüz bağlı değil" döner.
//! - ESC/POS içeriğinde para çekmecesi açma komutu reddedilir (`cekmece_komutu_var`; yazdir.sun Kural 12).
//! - Hiçbir şey silinmez, diske yazı yazılmaz (yalnız ayar okunur).

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpListener, TcpStream};
use std::path::Path;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use taa_imza::GenelAnahtar;

pub const VARSAYILAN_PORT: u16 = 8797;
/// Varsayılan gövde sınırı (1 MiB). Bir etiket/fiş işi bunun çok altındadır.
pub const VARSAYILAN_EN_COK_BAYT: usize = 1024 * 1024;
/// Ayar dosyasıyla da aşılamayan üst sınır (8 MiB).
pub const UST_SINIR_BAYT: usize = 8 * 1024 * 1024;
/// İstek satırı + başlıklar için sınır.
pub const BASLIK_EN_COK: usize = 16 * 1024;
/// Aynı anda işlenen bağlantı sayısı; fazlası 503 alır.
pub const ES_ZAMANLI_EN_COK: usize = 8;
const OKUMA_SURESI: Duration = Duration::from_secs(5);

/// Desteklenen sürücü adları (sözleşme: `argox-pplb` | `escpos` | `sistem`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Surucu {
    ArgoxPplb,
    Escpos,
    Sistem,
}

impl Surucu {
    pub const HEPSI: [Surucu; 3] = [Surucu::ArgoxPplb, Surucu::Escpos, Surucu::Sistem];

    pub fn ad(self) -> &'static str {
        match self {
            Surucu::ArgoxPplb => "argox-pplb",
            Surucu::Escpos => "escpos",
            Surucu::Sistem => "sistem",
        }
    }

    pub fn coz(ad: &str) -> Option<Surucu> {
        Self::HEPSI.into_iter().find(|s| s.ad() == ad)
    }
}

/// Ayar dosyası (JSON). Örnek: `ornek_ayar()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ayar {
    pub port: u16,
    /// Küçük harfe çevrilmiş, sonunda `/` olmayan kökenler (`https://ad[:port]`).
    pub izinli_kokenler: Vec<String>,
    pub en_cok_bayt: usize,
    /// sürücü adı -> hedef (dosya/paylaşım yolu ya da `tcp://IP[:9100]`).
    pub yazicilar: BTreeMap<Surucu, String>,
    /// `true` ise `POST /yazdir` cihaz imzası ister (kanca; varsayılan kapalı).
    pub cihaz_imza_zorunlu: bool,
}

/// Kökeni doğrular ve karşılaştırma biçimine çevirir. `*`, `null`, yol içeren ve şemasız değerler reddedilir.
pub fn koken_duzelt(ham: &str) -> Result<String, String> {
    let k = ham.trim().to_ascii_lowercase();
    let k = k.strip_suffix('/').unwrap_or(&k).to_string();
    let kalan = k
        .strip_prefix("https://")
        .or_else(|| k.strip_prefix("http://"))
        .ok_or_else(|| format!("köken http:// ya da https:// ile başlamalı: '{ham}'"))?;
    let gecerli = !kalan.is_empty()
        && kalan.len() <= 255
        && kalan
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'));
    if !gecerli {
        return Err(format!(
            "köken yalnız şema + ad + isteğe bağlı port olmalı (yol, * ya da boşluk olamaz): '{ham}'"
        ));
    }
    Ok(k)
}

impl Ayar {
    pub fn coz(metin: &str) -> Result<Ayar, String> {
        let v: Value = serde_json::from_str(metin.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("ayar JSON değil: {e}"))?;
        let o = v.as_object().ok_or("ayar bir JSON nesnesi olmalı")?;
        for a in o.keys() {
            if !matches!(
                a.as_str(),
                "port"
                    | "izinli_kokenler"
                    | "en_cok_bayt"
                    | "yazicilar"
                    | "cihaz_imza"
                    | "aciklama"
            ) {
                return Err(format!("bilinmeyen ayar alanı: '{a}'"));
            }
        }
        let port = match o.get("port") {
            None => VARSAYILAN_PORT,
            Some(p) => p
                .as_u64()
                .filter(|p| (1024..=65535).contains(p))
                .ok_or("port 1024-65535 arasında tam sayı olmalı")? as u16,
        };
        let mut izinli_kokenler = Vec::new();
        for k in o.get("izinli_kokenler").and_then(Value::as_array).ok_or(
            "izinli_kokenler listesi zorunlu (boş olabilir; boşsa hiçbir sayfa yazdıramaz)",
        )? {
            let k = koken_duzelt(k.as_str().ok_or("izinli_kokenler metin listesi olmalı")?)?;
            if !izinli_kokenler.contains(&k) {
                izinli_kokenler.push(k);
            }
        }
        let en_cok_bayt = match o.get("en_cok_bayt") {
            None => VARSAYILAN_EN_COK_BAYT,
            Some(b) => b
                .as_u64()
                .filter(|b| (1..=UST_SINIR_BAYT as u64).contains(b))
                .ok_or(format!("en_cok_bayt 1-{UST_SINIR_BAYT} arasında olmalı"))?
                as usize,
        };
        let mut yazicilar = BTreeMap::new();
        if let Some(y) = o.get("yazicilar") {
            for (ad, hedef) in y.as_object().ok_or("yazicilar bir nesne olmalı")? {
                let s = Surucu::coz(ad).ok_or(format!("bilinmeyen sürücü: '{ad}'"))?;
                if s == Surucu::Sistem {
                    return Err(
                        "'sistem' sürücüsüne hedef yazılamaz: sistem yazdırması henüz bağlı değil"
                            .into(),
                    );
                }
                let h = hedef.as_str().map(str::trim).unwrap_or("");
                if h.is_empty() {
                    return Err(format!("'{ad}' hedefi boş olamaz"));
                }
                yazicilar.insert(s, h.to_string());
            }
        }
        let cihaz_imza_zorunlu = match o.get("cihaz_imza").map(|c| c.as_str()) {
            None | Some(Some("kapali")) => false,
            Some(Some("zorunlu")) => true,
            _ => return Err("cihaz_imza 'kapali' ya da 'zorunlu' olmalı".into()),
        };
        Ok(Ayar {
            port,
            izinli_kokenler,
            en_cok_bayt,
            yazicilar,
            cihaz_imza_zorunlu,
        })
    }

    pub fn dosyadan(yol: &Path) -> Result<Ayar, String> {
        let m = std::fs::read_to_string(yol)
            .map_err(|e| format!("ayar dosyası okunamadı ({}): {e}", yol.display()))?;
        Self::coz(&m).map_err(|e| format!("{}: {e}", yol.display()))
    }

    pub fn koken_izinli(&self, koken: &str) -> bool {
        let k = koken.trim().to_ascii_lowercase();
        self.izinli_kokenler.contains(&k)
    }
}

/// Örnek ayar dosyası metni (adresler örnektir; gerçek köken ve yazıcı kurulumda yazılır).
pub fn ornek_ayar() -> String {
    let v = json!({
        "aciklama": "taa-yazdir ayarı. izinli_kokenler: TAA Web'in açıldığı adres(ler). yazicilar: dosya/paylaşım yolu ya da tcp://IP[:9100].",
        "port": VARSAYILAN_PORT,
        "izinli_kokenler": ["https://taa-web.ornek"],
        "en_cok_bayt": VARSAYILAN_EN_COK_BAYT,
        "yazicilar": {
            "argox-pplb": "\\\\localhost\\ArgoxEtiket",
            "escpos": "tcp://192.168.1.50:9100"
        },
        "cihaz_imza": "kapali"
    });
    serde_json::to_string_pretty(&v).unwrap_or_default()
}

/// Çözümlenmiş HTTP isteği. Başlık adları küçük harftir.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Istek {
    pub metot: String,
    /// Sorgu kısmı ayrılmış yol (`/yazdir`).
    pub yol: String,
    pub sorgu: String,
    pub basliklar: Vec<(String, String)>,
    pub govde: Vec<u8>,
    /// Karşı uç geri döngü adresi mi.
    pub yerel: bool,
}

impl Istek {
    pub fn baslik(&self, ad: &str) -> Option<&str> {
        self.basliklar
            .iter()
            .find(|(a, _)| a == ad)
            .map(|(_, d)| d.as_str())
    }

    fn sorgu_degeri(&self, ad: &str) -> Option<&str> {
        self.sorgu
            .split('&')
            .filter_map(|p| p.split_once('='))
            .find(|(a, _)| *a == ad)
            .map(|(_, d)| d)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yanit {
    pub kod: u16,
    pub basliklar: Vec<(String, String)>,
    pub govde: Vec<u8>,
}

impl Yanit {
    pub fn json(kod: u16, v: Value) -> Yanit {
        Yanit {
            kod,
            basliklar: Vec::new(),
            govde: v.to_string().into_bytes(),
        }
    }

    pub fn hata(kod: u16, mesaj: &str) -> Yanit {
        Self::json(kod, json!({ "tamam": false, "hata": mesaj }))
    }

    fn bos(kod: u16) -> Yanit {
        Yanit {
            kod,
            basliklar: Vec::new(),
            govde: Vec::new(),
        }
    }

    fn baslikla(mut self, ad: &str, deger: &str) -> Yanit {
        self.basliklar.push((ad.to_string(), deger.to_string()));
        self
    }
}

/// Yazıcıya gönderme noktası (testte sahte yazıcı takılır).
pub trait YaziciYolu: Send + Sync {
    fn yaz(&self, hedef: &str, baytlar: &[u8]) -> std::io::Result<()>;
}

/// Baytları yazıcıya gönderir. Hedef `tcp://IP[:9100]` ise ağ (Ethernet) yazıcısıdır; değilse hedef
/// yazılabilir bir yol olarak açılır (Windows'ta paylaşılan yazıcı `\\bilgisayar\paylasim` (iki ters bölü ile başlayan paylaşım yolu), `LPT1` vb.).
///
/// Bu işlev `taa-fis-sunucu/src/lib.rs` içindeki `yaziciya_yaz` ile AYNI yoldur ve oradan alınmıştır
/// ([Claude#taa-paket-yeni 2026-11-04]). Doğrudan çağrılmadı, çünkü `taa-yazdir` ortak crate'tir ve
/// `taa-fis-sunucu` (program: taafis) crate'ine bağlanması modül sözleşmesine aykırıdır. Doğrusu bu işlevi
/// ortak bir crate'e taşıyıp iki yerden çağırmaktır; mevcut dosyalara dokunulmadığı için bu adım açık bırakıldı.
pub fn yaziciya_yaz(hedef: &Path, baytlar: &[u8]) -> std::io::Result<()> {
    let h = hedef.to_string_lossy();
    if let Some(adres) = h.strip_prefix("tcp://") {
        use std::net::ToSocketAddrs;
        let adres = if adres.contains(':') {
            adres.to_string()
        } else {
            format!("{adres}:9100")
        };
        let a = adres
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| std::io::Error::other("yazıcı adresi çözülemedi"))?;
        let mut t = TcpStream::connect_timeout(&a, Duration::from_secs(5))?;
        t.set_write_timeout(Some(Duration::from_secs(10)))?;
        t.write_all(baytlar)?;
        t.flush()
    } else {
        std::fs::OpenOptions::new()
            .write(true)
            .open(hedef)
            .and_then(|mut f| f.write_all(baytlar))
    }
}

/// Gerçek yazıcı: `yaziciya_yaz`.
pub struct GercekYazici;

impl YaziciYolu for GercekYazici {
    fn yaz(&self, hedef: &str, baytlar: &[u8]) -> std::io::Result<()> {
        yaziciya_yaz(Path::new(hedef), baytlar)
    }
}

/// Cihaz imzası doğrulama kancası. `POST /yazdir` her istekte buradan geçer.
pub trait CihazDenetimi: Send + Sync {
    /// `/durum` çıktısında görünen ad (`kapali` | `zorunlu`).
    fn ad(&self) -> &'static str;
    fn denetle(&self, istek: &Istek) -> Result<(), String>;
}

/// Varsayılan: imza istenmez. Koruma köken listesi + geri döngü + Host denetimidir.
pub struct DenetimKapali;

impl CihazDenetimi for DenetimKapali {
    fn ad(&self) -> &'static str {
        "kapali"
    }
    fn denetle(&self, _: &Istek) -> Result<(), String> {
        Ok(())
    }
}

/// `taa-cihaz` ile doğrulama: `X-TAA-Cihaz` / `X-TAA-Zaman` / `X-TAA-Imza` başlıkları, mesaj biçimi
/// `taa_cihaz::imza_mesaji` (yöntem, yol, zaman, gövde). Yeni bir imza biçimi tanımlanmadı.
pub struct CihazImzasi {
    genel: GenelAnahtar,
    simdi: fn() -> i64,
}

impl CihazImzasi {
    pub fn yeni(genel: GenelAnahtar) -> CihazImzasi {
        CihazImzasi {
            genel,
            simdi: taa_cihaz::simdi_sn,
        }
    }

    /// `<veri kökü>/cihaz/cihaz.pub` dosyasından (özel anahtara dokunulmaz).
    pub fn kokten(veri_kok: &Path) -> Result<CihazImzasi, String> {
        let yol = taa_cihaz::CihazAnahtari::klasor(veri_kok).join(taa_cihaz::GENEL_DOSYA);
        GenelAnahtar::dosyadan(&yol)
            .map(Self::yeni)
            .map_err(|e| format!("cihaz genel anahtarı okunamadı ({}): {e}", yol.display()))
    }

    pub fn saatle(mut self, simdi: fn() -> i64) -> CihazImzasi {
        self.simdi = simdi;
        self
    }
}

impl CihazDenetimi for CihazImzasi {
    fn ad(&self) -> &'static str {
        "zorunlu"
    }

    fn denetle(&self, i: &Istek) -> Result<(), String> {
        let al = |ad: &str| {
            i.baslik(&ad.to_ascii_lowercase())
                .ok_or(format!("{ad} başlığı yok"))
        };
        let kimlik = al(taa_cihaz::BASLIK_CIHAZ)?;
        if kimlik != self.genel.kimlik() {
            return Err("cihaz kimliği bu bilgisayarın anahtarıyla uyuşmuyor".into());
        }
        let zaman: i64 = al(taa_cihaz::BASLIK_ZAMAN)?
            .parse()
            .map_err(|_| "X-TAA-Zaman sayı değil".to_string())?;
        let imza = al(taa_cihaz::BASLIK_IMZA)?;
        taa_cihaz::dogrula(
            &self.genel,
            &i.metot,
            &i.yol,
            zaman,
            &i.govde,
            imza,
            (self.simdi)(),
        )
        .map_err(|e| e.to_string())
    }
}

/// İstekleri işleyen çekirdek (ağdan bağımsız; testler doğrudan `isle` çağırır).
pub struct Hizmet {
    pub ayar: Ayar,
    yazici: Box<dyn YaziciYolu>,
    denetim: Box<dyn CihazDenetimi>,
    /// Yazıcı işleri birbirine karışmasın diye tek tek gönderilir.
    yazici_kilidi: Mutex<()>,
    yazdirilan: AtomicU64,
    hatali: AtomicU64,
}

impl Hizmet {
    pub fn yeni(ayar: Ayar) -> Hizmet {
        Self::ozel(ayar, Box::new(GercekYazici), Box::new(DenetimKapali))
    }

    pub fn ozel(
        ayar: Ayar,
        yazici: Box<dyn YaziciYolu>,
        denetim: Box<dyn CihazDenetimi>,
    ) -> Hizmet {
        Hizmet {
            ayar,
            yazici,
            denetim,
            yazici_kilidi: Mutex::new(()),
            yazdirilan: AtomicU64::new(0),
            hatali: AtomicU64::new(0),
        }
    }

    pub fn isle(&self, i: &Istek) -> Yanit {
        if !i.yerel {
            return Yanit::hata(403, "yalnız bu bilgisayardan erişilir");
        }
        if !host_yerel(i.baslik("host")) {
            return Yanit::hata(403, "Host başlığı 127.0.0.1 ya da localhost olmalı");
        }
        // Köken varsa izinli olmalı; izinli değilse CORS başlığı da verilmez.
        let koken = i.baslik("origin");
        if let Some(k) = koken {
            if !self.ayar.koken_izinli(k) {
                return Yanit::hata(
                    403,
                    "bu köken izinli değil (taa-yazdir ayarı: izinli_kokenler)",
                );
            }
        }
        let y = self.yonlendir(i, koken);
        match koken {
            Some(k) => y
                .baslikla("Access-Control-Allow-Origin", k)
                .baslikla("Vary", "Origin"),
            None => y,
        }
    }

    fn yonlendir(&self, i: &Istek, koken: Option<&str>) -> Yanit {
        match (i.metot.as_str(), i.yol.as_str()) {
            ("OPTIONS", "/yazdir" | "/durum") => {
                if koken.is_none() {
                    return Yanit::hata(403, "Origin başlığı yok");
                }
                Yanit::bos(204)
                    .baslikla("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
                    .baslikla(
                        "Access-Control-Allow-Headers",
                        "Content-Type, X-TAA-Cihaz, X-TAA-Zaman, X-TAA-Imza",
                    )
                    // Chrome "Private Network Access": genel adresteki sayfa geri döngüye istek atarken sorar.
                    .baslikla("Access-Control-Allow-Private-Network", "true")
                    .baslikla("Access-Control-Max-Age", "600")
            }
            ("GET", "/durum") => self.durum(),
            ("POST", "/yazdir") => {
                if koken.is_none() {
                    return Yanit::hata(
                        403,
                        "Origin başlığı yok: yazdırma yalnız izinli sayfadan yapılır",
                    );
                }
                if let Err(e) = self.denetim.denetle(i) {
                    return Yanit::hata(401, &format!("cihaz imzası doğrulanamadı: {e}"));
                }
                self.yazdir(i)
            }
            (_, "/yazdir" | "/durum") => Yanit::hata(405, "bu yöntem desteklenmez"),
            _ => Yanit::hata(404, "bulunamadı"),
        }
    }

    fn surucu_durumu(&self, s: Surucu) -> &'static str {
        if s != Surucu::Sistem && self.ayar.yazicilar.contains_key(&s) {
            "ayarli"
        } else {
            "bagli-degil"
        }
    }

    /// Yazıcı hedefleri (yol/IP) bilerek gösterilmez; yalnız ayarlı olup olmadığı.
    fn durum(&self) -> Yanit {
        let s: serde_json::Map<String, Value> = Surucu::HEPSI
            .into_iter()
            .map(|s| (s.ad().to_string(), json!(self.surucu_durumu(s))))
            .collect();
        Yanit::json(
            200,
            json!({
                "tamam": true,
                "hizmet": "taa-yazdir",
                "surum": env!("CARGO_PKG_VERSION"),
                "en_cok_bayt": self.ayar.en_cok_bayt,
                "suruculer": s,
                "cihaz_imza": self.denetim.ad(),
                "yazdirilan": self.yazdirilan.load(Ordering::Relaxed),
                "hatali": self.hatali.load(Ordering::Relaxed),
            }),
        )
    }

    fn yazdir(&self, i: &Istek) -> Yanit {
        let (surucu_adi, veri) = match is_coz(i) {
            Ok(x) => x,
            Err(e) => return Yanit::hata(400, &e),
        };
        let Some(surucu) = Surucu::coz(&surucu_adi) else {
            return Yanit::hata(
                400,
                "bilinmeyen sürücü (argox-pplb, escpos ya da sistem olmalı)",
            );
        };
        if veri.is_empty() {
            return Yanit::hata(400, "veri boş");
        }
        if surucu == Surucu::Escpos && cekmece_komutu_var(&veri) {
            self.hatali.fetch_add(1, Ordering::Relaxed);
            return Yanit::json(
                400,
                json!({ "tamam": false, "hata": "cekmece_red",
                        "mesaj": "Para çekmecesi yalnız bağlı olduğu bilgisayardan açılır." }),
            );
        }
        let henuz = |neden: &str| {
            Yanit::json(
                501,
                json!({ "tamam": false, "hata": "henüz bağlı değil", "surucu": surucu.ad(), "neden": neden }),
            )
        };
        if surucu == Surucu::Sistem {
            return henuz(
                "sistem yazdırması bu sürümde yok; tarayıcının kendi yazdırmasını kullanın",
            );
        }
        let Some(hedef) = self.ayar.yazicilar.get(&surucu) else {
            return henuz("bu sürücü için yazıcı hedefi ayar dosyasında yok (yazicilar)");
        };
        let sonuc = {
            let _k = self.yazici_kilidi.lock().unwrap_or_else(|z| z.into_inner());
            self.yazici.yaz(hedef, &veri)
        };
        match sonuc {
            Ok(()) => {
                self.yazdirilan.fetch_add(1, Ordering::Relaxed);
                Yanit::json(
                    200,
                    json!({ "tamam": true, "surucu": surucu.ad(), "bayt": veri.len() }),
                )
            }
            Err(e) => {
                self.hatali.fetch_add(1, Ordering::Relaxed);
                Yanit::hata(502, &format!("yazıcıya yazılamadı: {e}"))
            }
        }
    }
}

/// ESC/POS içeriğinde para çekmecesi açma komutu var mı: ESC p (`1B 70`) ya da DLE DC4 (`10 14 01`).
/// Kaynak kural: `docs/sozlesmeler/yazdir.sun.md` Kural 12 ("yerel gonder içeriği de çekmece komutu taşımaz").
/// Tarama bayt düzeyindedir; görüntü (raster) verisinde aynı baytlar rastlantıyla geçerse iş reddedilir (BENIOKU).
pub fn cekmece_komutu_var(veri: &[u8]) -> bool {
    veri.windows(2).any(|p| p == [0x1b, 0x70]) || veri.windows(3).any(|p| p == [0x10, 0x14, 0x01])
}

/// `Host` başlığı yalnız geri döngü adı olabilir (port serbest).
fn host_yerel(host: Option<&str>) -> bool {
    let Some(h) = host else { return false };
    let h = h.trim().to_ascii_lowercase();
    let ad = match h.rsplit_once(':') {
        Some((a, p)) if !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) => a,
        _ => h.as_str(),
    };
    matches!(ad, "127.0.0.1" | "localhost")
}

/// İsteğin gövdesinden (sürücü adı, yazıcıya gidecek baytlar) çıkarır.
/// - `application/json`: `{"surucu": "...", "veri": "<metin>", "kodlama": "utf8"|"bayt"}`.
///   `utf8` (varsayılan): metnin UTF-8 baytları gider. `bayt`: her karakter U+0000..U+00FF tek bayttır
///   (ESC/POS gibi ikili komutlar için; base64 yok).
/// - `application/octet-stream` ya da `text/plain`: gövde olduğu gibi gider, sürücü `?surucu=` ile verilir.
fn is_coz(i: &Istek) -> Result<(String, Vec<u8>), String> {
    let tur = i
        .baslik("content-type")
        .unwrap_or("")
        .split(';')
        .next()
        .unwrap_or("")
        .trim()
        .to_ascii_lowercase();
    match tur.as_str() {
        "application/json" => {
            let v: Value =
                serde_json::from_slice(&i.govde).map_err(|e| format!("gövde JSON değil: {e}"))?;
            let surucu = v["surucu"]
                .as_str()
                .ok_or("'surucu' alanı zorunlu (metin)")?
                .to_string();
            let veri = v["veri"].as_str().ok_or("'veri' alanı zorunlu (metin)")?;
            let baytlar = match v.get("kodlama").map(|k| k.as_str()) {
                None | Some(Some("utf8")) => veri.as_bytes().to_vec(),
                Some(Some("bayt")) => veri
                    .chars()
                    .map(|c| u8::try_from(u32::from(c)))
                    .collect::<Result<Vec<u8>, _>>()
                    .map_err(|_| {
                        "kodlama 'bayt' iken veri yalnız U+0000-U+00FF karakter içerebilir"
                            .to_string()
                    })?,
                _ => return Err("kodlama 'utf8' ya da 'bayt' olmalı".into()),
            };
            Ok((surucu, baytlar))
        }
        "application/octet-stream" | "text/plain" => {
            let surucu = i
                .sorgu_degeri("surucu")
                .ok_or("ham gövdede sürücü adres satırında verilir: /yazdir?surucu=escpos")?;
            Ok((surucu.to_string(), i.govde.clone()))
        }
        _ => Err(
            "Content-Type application/json, application/octet-stream ya da text/plain olmalı"
                .into(),
        ),
    }
}

/// Okuma hatası: (HTTP kodu, mesaj).
pub type OkumaHata = (u16, String);

fn alt_dizi_bul(b: &[u8], aranan: &[u8]) -> Option<usize> {
    b.windows(aranan.len()).position(|p| p == aranan)
}

/// Akıştan tek HTTP/1.x isteği okur. Başlıklar `BASLIK_EN_COK`, gövde `en_cok_bayt` ile sınırlıdır;
/// sınırı aşan gövde okunmaz (413). Parçalı (chunked) gövde desteklenmez.
pub fn istek_oku<R: Read>(r: &mut R, en_cok_bayt: usize, yerel: bool) -> Result<Istek, OkumaHata> {
    let mut tampon: Vec<u8> = Vec::with_capacity(1024);
    let mut parca = [0u8; 2048];
    let bas_sonu = loop {
        if let Some(n) = alt_dizi_bul(&tampon, b"\r\n\r\n") {
            break n;
        }
        if tampon.len() > BASLIK_EN_COK {
            return Err((431, "istek başlıkları çok büyük".into()));
        }
        let n = r
            .read(&mut parca)
            .map_err(|e| (408, format!("istek okunamadı: {e}")))?;
        if n == 0 {
            return Err((400, "istek yarım kaldı".into()));
        }
        tampon.extend_from_slice(&parca[..n]);
    };
    if bas_sonu > BASLIK_EN_COK {
        return Err((431, "istek başlıkları çok büyük".into()));
    }
    let bas = std::str::from_utf8(&tampon[..bas_sonu])
        .map_err(|_| (400, "başlıklar UTF-8 değil".to_string()))?;
    let mut satirlar = bas.split("\r\n");
    let ilk = satirlar.next().unwrap_or("");
    let mut p = ilk.split(' ');
    let (metot, hedef, surum) = match (p.next(), p.next(), p.next(), p.next()) {
        (Some(m), Some(h), Some(s), None) => (m, h, s),
        _ => return Err((400, "istek satırı bozuk".into())),
    };
    if !(surum == "HTTP/1.1" || surum == "HTTP/1.0")
        || metot.is_empty()
        || !metot.chars().all(|c| c.is_ascii_uppercase())
        || !hedef.starts_with('/')
    {
        return Err((400, "istek satırı bozuk".into()));
    }
    let (yol, sorgu) = hedef.split_once('?').unwrap_or((hedef, ""));
    let mut basliklar: Vec<(String, String)> = Vec::new();
    for s in satirlar {
        let (ad, deger) = s
            .split_once(':')
            .ok_or((400, "başlık satırı bozuk".to_string()))?;
        if ad.is_empty()
            || ad
                .chars()
                .any(|c| c.is_ascii_whitespace() || c.is_control())
        {
            return Err((400, "başlık adı bozuk".into()));
        }
        basliklar.push((ad.to_ascii_lowercase(), deger.trim().to_string()));
    }
    if basliklar.iter().any(|(a, _)| a == "transfer-encoding") {
        return Err((
            501,
            "parçalı gövde desteklenmez; Content-Length gönderin".into(),
        ));
    }
    let uzunluklar: Vec<&str> = basliklar
        .iter()
        .filter(|(a, _)| a == "content-length")
        .map(|(_, d)| d.as_str())
        .collect();
    let uzunluk: usize = match uzunluklar.as_slice() {
        [] if metot == "POST" => return Err((411, "Content-Length zorunlu".into())),
        [] => 0,
        [tek] if !tek.is_empty() && tek.chars().all(|c| c.is_ascii_digit()) => {
            // Sayıya sığmayan değer de "çok büyük"tür.
            tek.parse()
                .map_err(|_| (413, "gövde çok büyük".to_string()))?
        }
        _ => return Err((400, "Content-Length bozuk".into())),
    };
    if uzunluk > en_cok_bayt {
        return Err((
            413,
            format!("gövde çok büyük: en çok {en_cok_bayt} bayt kabul edilir"),
        ));
    }
    let mut govde = tampon[bas_sonu + 4..].to_vec();
    govde.truncate(uzunluk);
    let eksik = uzunluk - govde.len();
    if eksik > 0 {
        let bas = govde.len();
        govde.resize(uzunluk, 0);
        r.read_exact(&mut govde[bas..])
            .map_err(|e| (400, format!("gövde yarım kaldı: {e}")))?;
    }
    Ok(Istek {
        metot: metot.to_string(),
        yol: yol.to_string(),
        sorgu: sorgu.to_string(),
        basliklar,
        govde,
        yerel,
    })
}

fn kod_metni(kod: u16) -> &'static str {
    match kod {
        200 => "OK",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        411 => "Length Required",
        413 => "Content Too Large",
        431 => "Request Header Fields Too Large",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        _ => "Hata",
    }
}

/// Yanıtı yazar. Her yanıtta bağlantı kapanır.
pub fn yanit_yaz<W: Write>(w: &mut W, y: &Yanit) -> std::io::Result<()> {
    let mut bas = format!("HTTP/1.1 {} {}\r\n", y.kod, kod_metni(y.kod));
    for (ad, deger) in &y.basliklar {
        // Başlık değerine satır sonu sızmasın (köken değeri istemciden gelir).
        if !deger.contains(['\r', '\n']) {
            bas.push_str(&format!("{ad}: {deger}\r\n"));
        }
    }
    if y.kod != 204 {
        bas.push_str("Content-Type: application/json; charset=utf-8\r\n");
    }
    bas.push_str(&format!(
        "Content-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        y.govde.len()
    ));
    w.write_all(bas.as_bytes())?;
    w.write_all(&y.govde)?;
    w.flush()
}

/// Dinleyiciyi açar. Adres HER ZAMAN 127.0.0.1'dir; dışarıya (0.0.0.0) bağlanma seçeneği yoktur.
/// `port` 0 ise işletim sistemi boş bir port verir (testler için).
pub fn dinle(port: u16) -> std::io::Result<TcpListener> {
    TcpListener::bind(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port))
}

fn baglanti_isle(h: &Hizmet, mut akis: TcpStream) {
    let _ = akis.set_read_timeout(Some(OKUMA_SURESI));
    let _ = akis.set_write_timeout(Some(OKUMA_SURESI));
    let yerel = akis
        .peer_addr()
        .map(|a| a.ip().is_loopback())
        .unwrap_or(false);
    let yanit = match istek_oku(&mut akis, h.ayar.en_cok_bayt, yerel) {
        Ok(i) => h.isle(&i),
        Err((kod, mesaj)) => Yanit::hata(kod, &mesaj),
    };
    let _ = yanit_yaz(&mut akis, &yanit);
    let _ = akis.shutdown(std::net::Shutdown::Write);
    // Okunmamış gövde varken kapatmak yanıtı düşürebilir; kısa süre, sınırlı miktarda boşalt.
    let _ = akis.set_read_timeout(Some(Duration::from_millis(500)));
    let mut cop = [0u8; 4096];
    let mut toplam = 0usize;
    while toplam < UST_SINIR_BAYT {
        match akis.read(&mut cop) {
            Ok(n) if n > 0 => toplam += n,
            _ => break,
        }
    }
}

/// Bağlantıları `dur` işaretlenene kadar kabul eder. Her bağlantı ayrı iş parçacığında, en çok
/// `ES_ZAMANLI_EN_COK` tanesi aynı anda işlenir.
pub fn calistir(hizmet: Arc<Hizmet>, dinleyici: TcpListener, dur: Arc<AtomicBool>) {
    let acik = Arc::new(AtomicUsize::new(0));
    for akis in dinleyici.incoming() {
        if dur.load(Ordering::SeqCst) {
            break;
        }
        let Ok(mut akis) = akis else { continue };
        if acik.load(Ordering::SeqCst) >= ES_ZAMANLI_EN_COK {
            let _ = yanit_yaz(
                &mut akis,
                &Yanit::hata(503, "hizmet meşgul, yeniden deneyin"),
            );
            continue;
        }
        acik.fetch_add(1, Ordering::SeqCst);
        let (h, a) = (Arc::clone(&hizmet), Arc::clone(&acik));
        std::thread::spawn(move || {
            baglanti_isle(&h, akis);
            a.fetch_sub(1, Ordering::SeqCst);
        });
    }
}

#[cfg(test)]
mod testler;
