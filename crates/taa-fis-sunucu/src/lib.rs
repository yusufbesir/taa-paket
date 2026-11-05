//! taa-fis-sunucu: TaaFiş yerel ağ sunucusunun mantığı (HTTP'den bağımsız, test edilebilir).
//! Güvenlik: dışarıya açık port yok (yalnız mağaza ağı); telefon, bilgisayarda üretilen tek kullanımlık
//! 6 haneli kodla eşleşir (10 dk, 5 yanlışta 60 sn kilit); belirteç yalnız SHA256 özetiyle saklanır;
//! yönetim sayfası ve kod üretimi yalnız yerel bilgisayardan; yönetici telefonu kapatabilir.
//! Özellik anahtarları: "fis-sunucu", "telefon-eslestirme" (varsayılan açık). Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

pub mod cihaz;
pub mod sayfalar;

use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use taa_fis::{Defter, Durum, Girdi};

pub const GOVDE_SINIRI: usize = 64 * 1024;
/// Ürün fotoğrafı (JPEG) için gövde sınırı.
pub const FOTO_SINIRI: usize = 3 * 1024 * 1024;
const EN_COK_FOTO: usize = 6;
const KOD_OMRU: u64 = 600;
const KILIT_SURESI: u64 = 60;
const EN_COK_YANLIS: u32 = 5;

pub struct Istek {
    pub metot: String,
    pub yol: String,
    pub yetki: Option<String>,
    pub govde: Vec<u8>,
    /// İstek bu bilgisayardan (loopback) geldi mi?
    pub yerel: bool,
}

pub struct Yanit {
    pub kod: u16,
    pub tur: &'static str,
    pub govde: Vec<u8>,
}

impl Yanit {
    fn json(kod: u16, v: serde_json::Value) -> Self {
        Yanit {
            kod,
            tur: "application/json; charset=utf-8",
            govde: v.to_string().into_bytes(),
        }
    }
    fn hata(kod: u16, m: &str) -> Self {
        Self::json(kod, json!({ "hata": m }))
    }
    fn html(s: String) -> Self {
        Yanit {
            kod: 200,
            tur: "text/html; charset=utf-8",
            govde: s.into_bytes(),
        }
    }
}

pub struct Uygulama {
    pub dizin: PathBuf,
    /// Yazıcıya ham baytların yazılacağı yol (Windows'ta paylaşılan yazıcı `\\localhost\ADI`, Linux'ta aygıt dosyası).
    pub yazici_yolu: Option<PathBuf>,
    /// QR'a gömülecek taban adres (örn. `http://192.168.1.20:8787`).
    pub taban_url: String,
    /// Saati ileri alır (yalnız test/deneme; kod süresi ve kilit sınanır).
    pub ileri_sn: std::sync::atomic::AtomicU64,
    kodlar: Mutex<Vec<(String, u64)>>,
    yanlis: Mutex<(u32, u64)>,
}

/// Unix saniyesi (UTC).
pub fn simdi() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Unix saniyesinden Türkiye (UTC+3) takvim günü, YYYY-AA-GG.
pub fn tarih_iso(epoch: u64) -> String {
    let gun = ((epoch + 3 * 3600) / 86400) as i64;
    let z = gun + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

fn kapali(ad: &str) -> Option<Yanit> {
    if taa_ayar::ozellik_acik(ad, true) {
        None
    } else {
        Some(Yanit::hata(503, &taa_ayar::kapali_mesaji(ad)))
    }
}

impl Uygulama {
    pub fn yeni(dizin: PathBuf, yazici_yolu: Option<PathBuf>, taban_url: String) -> Self {
        Uygulama {
            dizin,
            yazici_yolu,
            taban_url,
            ileri_sn: std::sync::atomic::AtomicU64::new(0),
            kodlar: Mutex::new(Vec::new()),
            yanlis: Mutex::new((0, 0)),
        }
    }

    fn simdi(&self) -> u64 {
        simdi() + self.ileri_sn.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn cihazlar(&self) -> cihaz::Kayitci {
        cihaz::Kayitci {
            yol: self.dizin.join("cihazlar.jsonl"),
        }
    }

    fn fis_yolu(&self) -> PathBuf {
        self.dizin.join("fisler.jsonl")
    }

    fn yeni_kod(&self) -> Result<String, String> {
        let simdi = self.simdi();
        let kod = cihaz::yeni_kod()?;
        let mut k = self.kodlar.lock().map_err(|_| "kilit hatası")?;
        k.retain(|(_, son)| *son > simdi);
        k.push((kod.clone(), simdi + KOD_OMRU));
        Ok(kod)
    }

    fn kod_tuket(&self, kod: &str) -> Result<(), String> {
        let simdi = self.simdi();
        let mut y = self.yanlis.lock().map_err(|_| "kilit hatası")?;
        if y.1 > simdi {
            return Err(format!(
                "çok fazla yanlış deneme; {} saniye bekleyin",
                y.1 - simdi
            ));
        }
        let mut k = self.kodlar.lock().map_err(|_| "kilit hatası")?;
        k.retain(|(_, son)| *son > simdi);
        if let Some(i) = k.iter().position(|(c, _)| c == kod) {
            k.remove(i);
            *y = (0, 0);
            Ok(())
        } else {
            y.0 += 1;
            if y.0 >= EN_COK_YANLIS {
                *y = (0, simdi + KILIT_SURESI);
            }
            Err("kod yanlış ya da süresi dolmuş".into())
        }
    }

    pub fn isle(&self, i: &Istek) -> Yanit {
        let foto = i.metot == "POST" && i.yol.split('?').next().unwrap_or("").ends_with("/foto");
        if i.govde.len() > if foto { FOTO_SINIRI } else { GOVDE_SINIRI } {
            return Yanit::hata(413, "istek çok büyük");
        }
        let (yol, _sorgu) = i.yol.split_once('?').unwrap_or((i.yol.as_str(), ""));
        let parca: Vec<&str> = yol.trim_matches('/').split('/').collect();
        if let Some(k) = kapali("fis-sunucu") {
            return k;
        }
        match (i.metot.as_str(), parca.as_slice()) {
            ("GET", ["telefon"]) | ("GET", [""]) => Yanit::html(sayfalar::TELEFON.into()),
            ("GET", ["manifest.json"]) => Yanit {
                kod: 200,
                tur: "application/manifest+json",
                govde: sayfalar::MANIFEST.as_bytes().to_vec(),
            },
            ("POST", ["api", "eslestir"]) => self.eslestir(i),
            ("GET", ["yonetim"]) => self.yonetim_sayfasi(i),
            ("POST", ["api", "yonetim", "cihaz", id, "kapat"]) => {
                if !i.yerel {
                    return Yanit::hata(403, "yalnız bu bilgisayardan");
                }
                match self.cihazlar().kapat(id, &tarih_iso(self.simdi())) {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true })),
                    Err(e) => Yanit::hata(404, &e),
                }
            }
            ("GET" | "POST", ["api", ..]) => self.yetkili(i, &parca),
            _ => Yanit::hata(404, "bulunamadı"),
        }
    }

    /// Fişe eklenmiş fotoğraf sayısı. Fiş no yalnızca harf/rakam/tire ise sayılır (yol kaçışına karşı).
    fn foto_sayisi(&self, no: &str) -> usize {
        if no.is_empty() || !no.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return 0;
        }
        (1..=EN_COK_FOTO)
            .filter(|n| {
                self.dizin
                    .join("fotolar")
                    .join(format!("{no}-{n}.jpg"))
                    .exists()
            })
            .count()
    }

    fn eslestir(&self, i: &Istek) -> Yanit {
        if let Some(k) = kapali("telefon-eslestirme") {
            return k;
        }
        let v: serde_json::Value = match serde_json::from_slice(&i.govde) {
            Ok(v) => v,
            Err(_) => return Yanit::hata(400, "geçersiz istek"),
        };
        let (kod, ad) = (
            v["kod"].as_str().unwrap_or(""),
            v["ad"].as_str().unwrap_or(""),
        );
        if let Err(e) = self.kod_tuket(kod) {
            return Yanit::hata(403, &e);
        }
        let token = match cihaz::yeni_token() {
            Ok(t) => t,
            Err(e) => return Yanit::hata(500, &e),
        };
        match self.cihazlar().ekle(ad, &token, &tarih_iso(self.simdi())) {
            Ok(c) => Yanit::json(200, json!({ "token": token, "id": c.id })),
            Err(e) => Yanit::hata(400, &e),
        }
    }

    fn yonetim_sayfasi(&self, i: &Istek) -> Yanit {
        if !i.yerel {
            return Yanit::hata(403, "yönetim sayfası yalnız bu bilgisayardan açılır");
        }
        if let Some(k) = kapali("telefon-eslestirme") {
            return k;
        }
        let kod = match self.yeni_kod() {
            Ok(k) => k,
            Err(e) => return Yanit::hata(500, &e),
        };
        let url = format!("{}/telefon?kod={kod}", self.taban_url.trim_end_matches('/'));
        let qr = match qrcode::QrCode::new(url.as_bytes()) {
            Ok(q) => q
                .render::<qrcode::render::svg::Color>()
                .min_dimensions(260, 260)
                .build(),
            Err(e) => return Yanit::hata(500, &format!("karekod üretilemedi: {e}")),
        };
        let cihazlar = self
            .cihazlar()
            .aktifler()
            .unwrap_or_default()
            .into_iter()
            .map(|c| (c.id, c.ad, c.tarih))
            .collect::<Vec<_>>();
        Yanit::html(sayfalar::yonetim(&qr, &kod, &url, &cihazlar))
    }

    fn yetkili(&self, i: &Istek, parca: &[&str]) -> Yanit {
        let token = i
            .yetki
            .as_deref()
            .and_then(|h| h.strip_prefix("Bearer "))
            .unwrap_or("");
        let Some(cihaz) = self.cihazlar().dogrula(token) else {
            return Yanit::hata(401, "yetkisiz telefon");
        };
        let defter_yolu = self.fis_yolu();
        let d = Defter { yol: &defter_yolu };
        match (i.metot.as_str(), parca) {
            ("POST", ["api", "fis"]) => {
                let mut g: Girdi = match serde_json::from_slice(&i.govde) {
                    Ok(g) => g,
                    Err(e) => return Yanit::hata(400, &format!("geçersiz fiş: {e}")),
                };
                g.tarih = tarih_iso(self.simdi());
                match d.ekle_uyarili(g) {
                    Ok((f, uyarilar)) => {
                        Yanit::json(200, json!({ "no": f.no, "uyarilar": uyarilar }))
                    }
                    Err(e) => Yanit::hata(422, &e),
                }
            }
            ("GET", ["api", "islemler"]) => match taa_terzi::Katalog::dizinden(&self.dizin) {
                Ok(k) => Yanit::json(200, json!(k)),
                Err(e) => Yanit::hata(500, &e),
            },
            ("POST", ["api", "fis", no, "teslim"]) => {
                let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
                match d.teslim_degistir(
                    no,
                    v["yeni"].as_str().unwrap_or(""),
                    v["saat"].as_str().unwrap_or("17:00"),
                    v["neden"].as_str().unwrap_or(""),
                    &tarih_iso(self.simdi()),
                ) {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true })),
                    Err(e) => Yanit::hata(422, &e),
                }
            }
            ("GET", ["api", "fisler"]) => match d.fisler() {
                Ok(f) => Yanit::json(200, json!({ "fisler": f })),
                Err(e) => Yanit::hata(500, &e),
            },
            ("POST", ["api", "fis", no, "durum"]) => {
                let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
                let yeni = match v["durum"].as_str() {
                    Some("terzihanede") => Durum::Terzihanede,
                    Some("hazir") => Durum::Hazir,
                    Some("teslim") => Durum::Teslim,
                    Some("iptal") => Durum::Iptal,
                    _ => return Yanit::hata(400, "durum: terzihanede, hazir, teslim ya da iptal"),
                };
                match d.durum_ata(no, yeni, &tarih_iso(self.simdi())) {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true })),
                    Err(e) => Yanit::hata(409, &e),
                }
            }
            ("GET", ["api", "fis", no, "metin"]) => match d.bul(no) {
                Ok(Some(f)) => Yanit {
                    kod: 200,
                    tur: "text/plain; charset=utf-8",
                    govde: taa_fis::duz_metin(&f, 42).into_bytes(),
                },
                Ok(None) => Yanit::hata(404, "fiş yok"),
                Err(e) => Yanit::hata(500, &e),
            },
            ("GET", ["api", "fis", no]) => match d.bul(no) {
                Ok(Some(f)) => Yanit::json(200, json!(f)),
                Ok(None) => Yanit::hata(404, "fiş yok"),
                Err(e) => Yanit::hata(500, &e),
            },
            ("POST", ["api", "fis", no, "raf"]) => {
                let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
                match d.raf_ata(
                    no,
                    v["raf"].as_str().unwrap_or(""),
                    &cihaz.ad,
                    &tarih_iso(self.simdi()),
                ) {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true })),
                    Err(e) => Yanit::hata(422, &e),
                }
            }
            ("POST", ["api", "fis", no, "imza"]) => {
                let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
                match d.imzala(
                    no,
                    &cihaz.ad,
                    v["rol"].as_str().unwrap_or("usta"),
                    &tarih_iso(self.simdi()),
                ) {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true })),
                    Err(e) => Yanit::hata(422, &e),
                }
            }
            ("POST", ["api", "fis", no, "foto"]) => {
                if !taa_ayar::ozellik_acik("foto-eki", true) {
                    return Yanit::hata(409, &taa_ayar::kapali_mesaji("foto-eki"));
                }
                if !i.govde.starts_with(&[0xFF, 0xD8, 0xFF]) {
                    return Yanit::hata(400, "yalnız JPEG fotoğraf kabul edilir");
                }
                match d.bul(no) {
                    Ok(Some(_)) => {}
                    Ok(None) => return Yanit::hata(404, "fiş yok"),
                    Err(e) => return Yanit::hata(500, &e),
                }
                let n = self.foto_sayisi(no);
                if n >= EN_COK_FOTO {
                    return Yanit::hata(409, "bir fişe en çok 6 fotoğraf eklenir");
                }
                let klasor = self.dizin.join("fotolar");
                let yaz = std::fs::create_dir_all(&klasor).and_then(|_| {
                    std::fs::write(klasor.join(format!("{no}-{}.jpg", n + 1)), &i.govde)
                });
                match yaz {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true, "sira": n + 1 })),
                    Err(e) => Yanit::hata(500, &format!("fotoğraf kaydedilemedi: {e}")),
                }
            }
            ("GET", ["api", "fis", no, "foto"]) => {
                Yanit::json(200, json!({ "adet": self.foto_sayisi(no) }))
            }
            ("GET", ["api", "fis", no, "foto", n]) => {
                let n: usize = n.parse().unwrap_or(0);
                if n == 0 || n > self.foto_sayisi(no) {
                    return Yanit::hata(404, "fotoğraf yok");
                }
                match std::fs::read(self.dizin.join("fotolar").join(format!("{no}-{n}.jpg"))) {
                    Ok(b) => Yanit {
                        kod: 200,
                        tur: "image/jpeg",
                        govde: b,
                    },
                    Err(_) => Yanit::hata(404, "fotoğraf yok"),
                }
            }
            ("POST", ["api", "fis", no, "yazdir"]) => {
                let f = match d.bul(no) {
                    Ok(Some(f)) => f,
                    Ok(None) => return Yanit::hata(404, "fiş yok"),
                    Err(e) => return Yanit::hata(500, &e),
                };
                let Some(yazici) = &self.yazici_yolu else {
                    return Yanit::hata(
                        409,
                        "yazıcı ayarlı değil (sunucuyu --yazici ile başlatın)",
                    );
                };
                let nusha: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
                let baytlar = if nusha["nusha"].as_u64() == Some(1) {
                    taa_fis::escpos(&f, 42)
                } else {
                    let qr = format!(
                        "{}/telefon#fis={}",
                        self.taban_url.trim_end_matches('/'),
                        f.no
                    );
                    taa_fis::escpos_nushalar_qr(&f, 42, &qr)
                };
                match yaziciya_yaz(yazici, &baytlar) {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true })),
                    Err(e) => Yanit::hata(502, &format!("yazıcıya yazılamadı: {e}")),
                }
            }
            _ => Yanit::hata(404, "bulunamadı"),
        }
    }
}

/// Baytları yazıcıya gönderir. Hedef `tcp://IP[:9100]` ise ağ (Ethernet) yazıcısıdır;
/// aksi halde dosya/paylaşım yoludur (USB yazıcı: `\\\\localhost\\ADI` ya da `/dev/usb/lp0`).
pub fn yaziciya_yaz(hedef: &Path, baytlar: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
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
        let mut t = std::net::TcpStream::connect_timeout(&a, std::time::Duration::from_secs(5))?;
        t.set_write_timeout(Some(std::time::Duration::from_secs(10)))?;
        t.write_all(baytlar)?;
        t.flush()
    } else {
        std::fs::OpenOptions::new()
            .write(true)
            .open(hedef)
            .and_then(|mut f| f.write_all(baytlar))
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn uyg() -> Uygulama {
        Uygulama::yeni(
            taa_dosya::gecici_klasor().unwrap(),
            None,
            "http://192.168.1.20:8787".into(),
        )
    }

    fn ist(metot: &str, yol: &str, yetki: Option<&str>, govde: &str, yerel: bool) -> Istek {
        Istek {
            metot: metot.into(),
            yol: yol.into(),
            yetki: yetki.map(|t| format!("Bearer {t}")),
            govde: govde.as_bytes().to_vec(),
            yerel,
        }
    }

    fn kod_al(u: &Uygulama) -> String {
        let y = u.isle(&ist("GET", "/yonetim", None, "", true));
        let s = String::from_utf8(y.govde).unwrap();
        let i = s.find("telefon?kod=").unwrap() + "telefon?kod=".len();
        s[i..i + 6].to_string()
    }

    fn esles(u: &Uygulama) -> String {
        let k = kod_al(u);
        let y = u.isle(&ist(
            "POST",
            "/api/eslestir",
            None,
            &format!(r#"{{"kod":"{k}","ad":"Ayşe'nin telefonu"}}"#),
            false,
        ));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap()["token"]
            .as_str()
            .unwrap()
            .to_string()
    }

    const FIS: &str = r#"{"satis_danismani":"Mert","olcu_alan":"Kadir Y.","musteri":"Ayşe Çelik","telefon":"0532 123 45 67","kalemler":[{"urun":"Pantolon","islem":"paça kısalt"}],"teslim":"2099-01-01","ucret_kurus":50000,"on_odeme_kurus":10000}"#;

    #[test]
    fn tarih() {
        assert_eq!(tarih_iso(0), "1970-01-01");
        assert_eq!(tarih_iso(1_730_757_600), "2024-11-05"); // 2024-11-04 22:00 UTC = 05 Kasım 01:00 TR
        assert_eq!(tarih_iso(951_782_400), "2000-02-29");
    }

    #[test]
    fn yonetim_yalniz_yerel() {
        let u = uyg();
        assert_eq!(u.isle(&ist("GET", "/yonetim", None, "", false)).kod, 403);
        let y = u.isle(&ist("GET", "/yonetim", None, "", true));
        assert_eq!(y.kod, 200);
        let s = String::from_utf8(y.govde).unwrap();
        assert!(s.contains("<svg") && s.contains("192.168.1.20:8787/telefon?kod="));
    }

    #[test]
    fn eslestirme_ve_fis_akisi() {
        let u = uyg();
        // yetkisiz
        assert_eq!(u.isle(&ist("GET", "/api/fisler", None, "", false)).kod, 401);
        assert_eq!(
            u.isle(&ist("GET", "/api/fisler", Some("sahte"), "", false))
                .kod,
            401
        );
        let t = esles(&u);
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let no = serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        let l = u.isle(&ist("GET", "/api/fisler", Some(&t), "", false));
        assert!(String::from_utf8(l.govde).unwrap().contains(&no));
        assert_eq!(
            u.isle(&ist(
                "POST",
                &format!("/api/fis/{no}/durum"),
                Some(&t),
                r#"{"durum":"teslim"}"#,
                false
            ))
            .kod,
            409
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                &format!("/api/fis/{no}/durum"),
                Some(&t),
                r#"{"durum":"hazir"}"#,
                false
            ))
            .kod,
            200
        );
        let m = u.isle(&ist(
            "GET",
            &format!("/api/fis/{no}/metin"),
            Some(&t),
            "",
            false,
        ));
        assert!(String::from_utf8(m.govde).unwrap().contains("Ayşe Çelik"));
        // geçersiz fiş
        assert_eq!(
            u.isle(&ist(
                "POST",
                "/api/fis",
                Some(&t),
                r#"{"musteri":"","telefon":"1","kalemler":[],"teslim":"x","ucret_kurus":1}"#,
                false
            ))
            .kod,
            422
        );
        assert_eq!(
            u.isle(&ist("POST", "/api/fis", Some(&t), "bozuk", false))
                .kod,
            400
        );
    }

    #[test]
    fn kod_tek_kullanimlik_ve_sureli() {
        let u = uyg();
        let k = kod_al(&u);
        let gov = format!(r#"{{"kod":"{k}","ad":"T1"}}"#);
        assert_eq!(
            u.isle(&ist("POST", "/api/eslestir", None, &gov, false)).kod,
            200
        );
        assert_eq!(
            u.isle(&ist("POST", "/api/eslestir", None, &gov, false)).kod,
            403
        ); // ikinci kullanım
        let k2 = kod_al(&u);
        u.ileri_sn
            .fetch_add(601, std::sync::atomic::Ordering::SeqCst);
        let gov2 = format!(r#"{{"kod":"{k2}","ad":"T2"}}"#);
        assert_eq!(
            u.isle(&ist("POST", "/api/eslestir", None, &gov2, false))
                .kod,
            403
        ); // süresi doldu
    }

    #[test]
    fn yanlis_kod_kilitler() {
        let u = uyg();
        let k = kod_al(&u);
        let yanlis = if k == "000000" { "000001" } else { "000000" };
        for _ in 0..5 {
            assert_eq!(
                u.isle(&ist(
                    "POST",
                    "/api/eslestir",
                    None,
                    &format!(r#"{{"kod":"{yanlis}","ad":"x"}}"#),
                    false
                ))
                .kod,
                403
            );
        }
        // doğru kod bile kilit süresince reddedilir
        let y = u.isle(&ist(
            "POST",
            "/api/eslestir",
            None,
            &format!(r#"{{"kod":"{k}","ad":"x"}}"#),
            false,
        ));
        assert_eq!(y.kod, 403);
        assert!(String::from_utf8_lossy(&y.govde).contains("bekleyin"));
    }

    #[test]
    fn cihaz_kapatilinca_erisim_biter() {
        let u = uyg();
        let t = esles(&u);
        assert_eq!(
            u.isle(&ist("GET", "/api/fisler", Some(&t), "", false)).kod,
            200
        );
        let id = u.cihazlar().aktifler().unwrap()[0].id.clone();
        assert_eq!(
            u.isle(&ist(
                "POST",
                &format!("/api/yonetim/cihaz/{id}/kapat"),
                None,
                "",
                false
            ))
            .kod,
            403
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                &format!("/api/yonetim/cihaz/{id}/kapat"),
                None,
                "",
                true
            ))
            .kod,
            200
        );
        assert_eq!(
            u.isle(&ist("GET", "/api/fisler", Some(&t), "", false)).kod,
            401
        );
    }

    #[test]
    fn yazdirma() {
        let mut u = uyg();
        let t = esles(&u);
        let no = serde_json::from_slice::<serde_json::Value>(
            &u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false)).govde,
        )
        .unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        let yol = format!("/api/fis/{no}/yazdir");
        assert_eq!(u.isle(&ist("POST", &yol, Some(&t), "", false)).kod, 409); // yazıcı yok
        let p = u.dizin.join("yazici.bin");
        std::fs::write(&p, b"").unwrap();
        u.yazici_yolu = Some(p.clone());
        assert_eq!(u.isle(&ist("POST", &yol, Some(&t), "", false)).kod, 200);
        let b = std::fs::read(&p).unwrap();
        assert_eq!(&b[..2], &[0x1B, 0x40]);
        assert_eq!(
            b.windows(4).filter(|w| *w == [0x1D, 0x56, 0x42, 0]).count(),
            3
        ); // 2 terzihane + 1 müşteri
        u.yazici_yolu = Some(u.dizin.join("yok").join("x"));
        assert_eq!(u.isle(&ist("POST", &yol, Some(&t), "", false)).kod, 502);
    }

    /// Sanal yazıcı: yerel TCP dinleyici, ESC/POS baytlarını alır; CP857 ters çevrilip Türkçe karakterler doğrulanır.
    #[test]
    fn sanal_ag_yazicisi_turkce() {
        use std::io::Read;
        let dinle = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = dinle.local_addr().unwrap().port();
        let alici = std::thread::spawn(move || {
            let (mut c, _) = dinle.accept().unwrap();
            let mut v = Vec::new();
            c.read_to_end(&mut v).unwrap();
            v
        });
        let mut u = uyg();
        let t = esles(&u);
        let govde = FIS.replace("Ayşe Çelik", "Şükrü Ğüleç İÖ ığöçş");
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), &govde, false));
        let no = serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        u.yazici_yolu = Some(PathBuf::from(format!("tcp://127.0.0.1:{port}")));
        let yol = format!("/api/fis/{no}/yazdir");
        assert_eq!(u.isle(&ist("POST", &yol, Some(&t), "", false)).kod, 200);
        let baytlar = alici.join().unwrap();
        // CP857 -> metin
        let metin: String = baytlar
            .iter()
            .map(|b| match b {
                0x80 => 'Ç',
                0x81 => 'ü',
                0x87 => 'ç',
                0x8D => 'ı',
                0x98 => 'İ',
                0x99 => 'Ö',
                0x9A => 'Ü',
                0x94 => 'ö',
                0xA6 => 'Ğ',
                0xA7 => 'ğ',
                0x9E => 'Ş',
                0x9F => 'ş',
                b if b.is_ascii() => *b as char,
                _ => '\u{FFFD}',
            })
            .collect();
        assert!(metin.contains("Şükrü Ğüleç İÖ ığöçş"), "{metin}");
        assert!(metin.contains("TERZİHANE NÜSHASI 1") && metin.contains("MÜŞTERİ NÜSHASI"));
        assert!(!metin.contains('?') || !metin.contains("Müşteri Adı: ?"));
    }

    #[test]
    fn raf_imza_ve_qr_adresi() {
        let u = uyg();
        let t = esles(&u);
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false));
        let no = serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        let p = |yol: &str, g: &str| u.isle(&ist("POST", yol, Some(&t), g, false)).kod;
        assert_eq!(p(&format!("/api/fis/{no}/raf"), r#"{"raf":"a-12"}"#), 200);
        assert_eq!(p(&format!("/api/fis/{no}/raf"), r#"{"raf":""}"#), 422);
        assert_eq!(p(&format!("/api/fis/{no}/imza"), r#"{"rol":"usta"}"#), 200);
        assert_eq!(p(&format!("/api/fis/{no}/imza"), r#"{"rol":"usta"}"#), 422);
        let g = u.isle(&ist("GET", &format!("/api/fis/{no}"), Some(&t), "", false));
        let j: serde_json::Value = serde_json::from_slice(&g.govde).unwrap();
        assert_eq!(j["raf"], "A-12");
        assert_eq!(j["imzalar"][0]["kim"], "Ayşe'nin telefonu");
        assert_eq!(
            u.isle(&ist("GET", "/api/fis/F-yok", Some(&t), "", false))
                .kod,
            404
        );
    }

    #[test]
    fn sayfalar_ve_sinirlar() {
        let u = uyg();
        let y = u.isle(&ist("GET", "/telefon", None, "", false));
        assert!(String::from_utf8(y.govde).unwrap().contains("<html"));
        assert_eq!(u.isle(&ist("GET", "/bilinmeyen", None, "", false)).kod, 404);
        let buyuk = "x".repeat(GOVDE_SINIRI + 1);
        assert_eq!(
            u.isle(&ist("POST", "/api/eslestir", None, &buyuk, false))
                .kod,
            413
        );
        assert_eq!(
            sayfalar::kacis("<a href=\"x\">&'"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
        );
    }

    #[test]
    fn foto_eki() {
        let u = uyg();
        let t = esles(&u);
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let no = serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        let jpg = vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3];
        let ist = |m: &str, y: String, g: Vec<u8>| Istek {
            metot: m.into(),
            yol: y,
            yetki: Some(format!("Bearer {t}")),
            govde: g,
            yerel: false,
        };
        let y = u.isle(&ist("POST", format!("/api/fis/{no}/foto"), jpg.clone()));
        assert_eq!(y.kod, 200);
        assert_eq!(
            u.isle(&ist("POST", format!("/api/fis/{no}/foto"), b"yok".to_vec()))
                .kod,
            400
        );
        assert_eq!(
            u.isle(&ist("POST", "/api/fis/F-yok-1/foto".into(), jpg.clone()))
                .kod,
            404
        );
        let g = u.isle(&ist("GET", format!("/api/fis/{no}/foto/1"), vec![]));
        assert_eq!((g.kod, g.govde), (200, jpg));
        assert_eq!(
            u.isle(&ist("GET", format!("/api/fis/{no}/foto/2"), vec![]))
                .kod,
            404
        );
    }

    #[test]
    fn islemler_ve_teslim_degisikligi() {
        let u = uyg();
        let t = esles(&u);
        let k = u.isle(&ist("GET", "/api/islemler", Some(&t), "", false));
        assert_eq!(k.kod, 200);
        assert!(String::from_utf8(k.govde)
            .unwrap()
            .contains("pant-paca-kisalt"));
        let no = serde_json::from_slice::<serde_json::Value>(
            &u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false)).govde,
        )
        .unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        let yol = format!("/api/fis/{no}/teslim");
        assert_eq!(
            u.isle(&ist(
                "POST",
                &yol,
                Some(&t),
                r#"{"yeni":"2099-02-01"}"#,
                false
            ))
            .kod,
            422
        ); // neden yok
        assert_eq!(
            u.isle(&ist(
                "POST",
                &yol,
                Some(&t),
                r#"{"yeni":"2099-02-01","saat":"18:00","neden":"müşteri istedi"}"#,
                false
            ))
            .kod,
            200
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                &format!("/api/fis/{no}/durum"),
                Some(&t),
                r#"{"durum":"terzihanede"}"#,
                false
            ))
            .kod,
            200
        );
        let l = String::from_utf8(
            u.isle(&ist("GET", "/api/fisler", Some(&t), "", false))
                .govde,
        )
        .unwrap();
        assert!(l.contains("terzihanede") && l.contains("müşteri istedi"));
    }

    #[test]
    fn katalog_siniri_sunucuda_calisir() {
        let u = uyg();
        let t = esles(&u);
        let fis = |olcu: i32, onay: bool| {
            format!(
                r#"{{"satis_danismani":"M","olcu_alan":"K","musteri":"A B","telefon":"05321234567","kalemler":[{{"urun":"Pantolon","islem":"Bel daraltma","islem_kodu":"pant-bel-daralt","olcu_mm":{olcu},"usta_onayli":{onay}}}],"teslim":"2099-01-01","ucret_kurus":0}}"#
            )
        };
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), &fis(60, false), false));
        assert_eq!(y.kod, 422);
        assert!(String::from_utf8_lossy(&y.govde).contains("Usta onayı"));
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), &fis(60, true), false));
        assert_eq!(y.kod, 200);
        assert!(String::from_utf8_lossy(&y.govde).contains("doğrulanmadı"));
    }

    #[test]
    fn olcu_cm_istege_bagli_igneli() {
        let u = uyg();
        let t = esles(&u);
        let fis = r#"{"satis_danismani":"M","olcu_alan":"K","musteri":"A B","telefon":"05321234567","kalemler":[{"urun":"Pantolon","islem":"Bel daraltma","islem_kodu":"pant-bel-daralt","igneli":true},{"urun":"Pantolon","islem":"Paça kısaltma","islem_kodu":"pant-paca-kisalt","varyant":"Düz paça","igneli":true}],"teslim":"2099-01-01","ucret_kurus":0}"#;
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), fis, false));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert!(String::from_utf8_lossy(&y.govde).contains("denetlenemedi"));
    }
}
