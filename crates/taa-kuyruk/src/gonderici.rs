// Sahip: Yusuf. yazan: [Claude#taa-kuyruk-T6 2026-11-06]
//! Gönderici: bir kaydı sunucuya ulaştıran arayüz ve `curl` ile çalışan gerçeklemesi.
//!
//! `CurlGonderici` her kaydı `POST <taban><uç>` olarak JSON gövdeyle yollar. Oturum anahtarı
//! (`Authorization: Bearer …`) komut satırına yazılmaz: geçici bir başlık dosyasına yazılır, curl `--header @dosya`
//! ile okur, iş bitince dosya kaldırılır (anahtar kalıcı hiçbir yere yazılmaz). HTTP durum kodu
//! `--write-out` ile alınır: 2xx kabul, 4xx (408/429 dışında) kalıcı ret, diğerleri "ulaşılamadı" (yeniden denenir).

use serde_json::Value;
use std::fmt;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;

/// Saniye cinsinden bir isteğin en uzun süresi (curl `--max-time`).
pub const EN_UZUN_SURE_SN: u64 = 60;
/// Sunucu yanıtı en çok bu kadar bayt okunur.
pub const YANIT_EN_COK: u64 = 4 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq)]
pub enum GonderHata {
    /// Ağ yok, zaman aşımı, 5xx, 408/429: kayıt bekler, sonra yeniden denenir.
    Ulasilamadi(String),
    /// Sunucu kaydı kalıcı olarak reddetti (4xx): yeniden denenmez.
    Reddedildi { kod: u16, mesaj: String },
    /// Yapılandırma hatası (geçersiz adres, curl yok): hiçbir kayıt gönderilmez.
    Yapilandirma(String),
}

impl fmt::Display for GonderHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GonderHata::Ulasilamadi(m) => write!(f, "sunucuya ulaşılamadı: {m}"),
            GonderHata::Reddedildi { kod, mesaj } => write!(f, "sunucu reddetti ({kod}): {mesaj}"),
            GonderHata::Yapilandirma(m) => write!(f, "yapılandırma hatası: {m}"),
        }
    }
}

impl std::error::Error for GonderHata {}

/// Yalnız testler için: son yazılan geçici başlık dosyasının yolu (silindiği doğrulanır).
#[cfg(test)]
pub(crate) static SON_BASLIK_DOSYASI: std::sync::Mutex<Option<PathBuf>> =
    std::sync::Mutex::new(None);

pub trait Gonderici {
    /// `POST`: `uc` `/v1/...` biçiminde; `govde` JSON nesnesi. Başarıda sunucu yanıtı (JSON ya da Null).
    fn gonder(&self, uc: &str, govde: &Value) -> Result<Value, GonderHata>;

    /// `GET`: `uc` sorgu içerebilir (`/v1/olay?limit=50`, bkz. `uc_sorgulu_gecerli`). Varsayılan: desteklenmez
    /// (kuyruk yalnız POST kullanır; yönetim okumaları için `CurlGonderici` gerçekler).
    fn al(&self, uc: &str) -> Result<Value, GonderHata> {
        Err(GonderHata::Yapilandirma(format!(
            "bu gönderici GET desteklemiyor: {uc}"
        )))
    }
}

/// İstek imzalayıcı (cihaz kimliği, API-SOZLESMESI "Cihaz kimliği"): her isteğe ek başlıklar üretir.
/// `yontem` ("POST"), `yol` (sorgu dahil `/v1/...`), `govde` gönderilecek baytlar. Dönen (ad, değer) çiftleri
/// tek satır ASCII olmalı; aksi halde gönderim yapılandırma hatasıyla durur.
pub trait Imzalayici: Send + Sync {
    fn basliklar(
        &self,
        yontem: &str,
        yol: &str,
        govde: &[u8],
    ) -> Result<Vec<(String, String)>, String>;
}

pub fn http_url_mu(s: &str) -> bool {
    let k = s.trim().to_ascii_lowercase();
    k.starts_with("http://") || k.starts_with("https://")
}

fn yerel_mi(url: &str) -> bool {
    let k = url.to_ascii_lowercase();
    let govde = k
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let sunucu = govde.split(['/', '?', '#']).next().unwrap_or("");
    let sunucu = sunucu.rsplit('@').next().unwrap_or(sunucu);
    sunucu.starts_with("127.0.0.1")
        || sunucu.starts_with("localhost")
        || sunucu.starts_with("[::1]")
}

/// `curl` ile HTTP(S) gönderici.
#[derive(Clone)]
pub struct CurlGonderici {
    taban: String,
    oturum: Option<String>,
    ca: Option<PathBuf>,
    kullanici_ajani: String,
    imzalayici: Option<Arc<dyn Imzalayici>>,
}

impl fmt::Debug for CurlGonderici {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CurlGonderici")
            .field("taban", &self.taban)
            .field("oturum", &self.oturum.as_ref().map(|_| "<gizli>"))
            .field("ca", &self.ca)
            .field("imzalayici", &self.imzalayici.is_some())
            .finish()
    }
}

impl CurlGonderici {
    /// `taban`: `https://sunucu[:port]` (sondaki `/` önemsiz). `http://` yalnız yerel ağ denemeleri içindir.
    pub fn yeni(taban: &str) -> Result<CurlGonderici, GonderHata> {
        let t = taban.trim();
        if !http_url_mu(t) || t.contains(char::is_whitespace) {
            return Err(GonderHata::Yapilandirma(format!(
                "sunucu adresi http:// ya da https:// ile başlamalı ({t}). Öneri: --sunucu değerini düzeltin."
            )));
        }
        Ok(CurlGonderici {
            taban: t.trim_end_matches('/').to_string(),
            oturum: None,
            ca: None,
            kullanici_ajani: "taa-kuyruk".into(),
            imzalayici: None,
        })
    }

    /// Cihaz imzalayıcısı: her isteğe `X-TAA-*` başlıklarını ekler (taa-cihaz).
    pub fn imzalayici(mut self, i: Arc<dyn Imzalayici>) -> CurlGonderici {
        self.imzalayici = Some(i);
        self
    }

    pub fn imzali_mi(&self) -> bool {
        self.imzalayici.is_some()
    }

    /// Oturum anahtarı (Bearer). Boş ya da satır sonu içeren değer reddedilir.
    pub fn oturum(mut self, anahtar: &str) -> Result<CurlGonderici, GonderHata> {
        let a = anahtar.trim();
        if a.is_empty() || a.contains(['\n', '\r']) || !a.is_ascii() {
            return Err(GonderHata::Yapilandirma(
                "oturum anahtarı boş olamaz, tek satır ASCII olmalı".into(),
            ));
        }
        self.oturum = Some(a.to_string());
        Ok(self)
    }

    /// Kendi sertifikamız (PEM): sunucu TLS'i bununla doğrulanır.
    pub fn sertifika(mut self, ca: &Path) -> CurlGonderici {
        self.ca = Some(ca.to_path_buf());
        self
    }

    pub fn taban(&self) -> &str {
        &self.taban
    }

    fn basliklar(&self, yontem: &str, uc: &str, govde: &[u8]) -> Result<PathBuf, GonderHata> {
        let klasor = taa_dosya::gecici_klasor()
            .map_err(|e| GonderHata::Yapilandirma(format!("geçici klasör açılamadı: {e}")))?;
        let yol = klasor.join("basliklar.txt");
        let mut icerik = String::from("Accept: application/json\r\n");
        if yontem == "POST" {
            icerik.push_str("Content-Type: application/json\r\n");
        }
        if let Some(o) = &self.oturum {
            icerik.push_str(&format!("Authorization: Bearer {o}\r\n"));
        }
        if let Some(i) = &self.imzalayici {
            for (ad, deger) in i
                .basliklar(yontem, uc, govde)
                .map_err(|e| GonderHata::Yapilandirma(format!("istek imzalanamadı: {e}")))?
            {
                let tek_satir =
                    |s: &str| s.is_ascii() && !s.contains(['\n', '\r']) && !s.is_empty();
                if !tek_satir(&ad) || !tek_satir(&deger) || ad.contains(':') {
                    return Err(GonderHata::Yapilandirma(format!(
                        "imza başlığı tek satır ASCII olmalı: {ad}"
                    )));
                }
                icerik.push_str(&format!("{ad}: {deger}\r\n"));
            }
        }
        let mut f = fs::File::create(&yol)
            .map_err(|e| GonderHata::Yapilandirma(format!("başlık dosyası yazılamadı: {e}")))?;
        f.write_all(icerik.as_bytes())
            .map_err(|e| GonderHata::Yapilandirma(format!("başlık dosyası yazılamadı: {e}")))?;
        Ok(yol)
    }

    fn komut(&self, yontem: &str, url: &str, basliklar: &Path) -> Command {
        let mut c = Command::new("curl");
        c.args(["--silent", "--show-error", "--request", yontem])
            .args(["--proto", "=http,https", "--max-redirs", "0"])
            .args(["--max-time", &EN_UZUN_SURE_SN.to_string()])
            .args(["--max-filesize", &YANIT_EN_COK.to_string()])
            .args(["--user-agent", &self.kullanici_ajani])
            .args(["--write-out", "\n%{http_code}"]);
        if yontem == "POST" {
            c.args(["--data-binary", "@-"]);
        }
        c.arg("--header").arg(format!("@{}", basliklar.display()));
        if let Some(ca) = &self.ca {
            c.arg("--cacert").arg(ca);
        }
        if yerel_mi(url) {
            c.args(["--noproxy", "*"]);
        }
        c.arg(url);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        c
    }
}

/// curl çıktısını (`gövde\nkod`) ayırır; kod 3 haneli değilse None.
pub fn cikti_ayir(stdout: &[u8]) -> Option<(Vec<u8>, u16)> {
    let n = stdout.iter().rposition(|&b| b == b'\n')?;
    let kod = std::str::from_utf8(&stdout[n + 1..]).ok()?.trim();
    if kod.len() != 3 {
        return None;
    }
    let kod: u16 = kod.parse().ok()?;
    Some((stdout[..n].to_vec(), kod))
}

/// HTTP durum koduna göre sonuç: 2xx kabul; 408/429/5xx ve 0 bekler; diğer 4xx kalıcı ret.
pub fn kodu_yorumla(kod: u16, govde: &[u8]) -> Result<Value, GonderHata> {
    let metin = || {
        let m = String::from_utf8_lossy(govde);
        let m = m.trim();
        if m.is_empty() {
            "ayrıntı yok".to_string()
        } else {
            m.chars().take(300).collect()
        }
    };
    match kod {
        200..=299 => {
            if govde.iter().all(u8::is_ascii_whitespace) {
                return Ok(Value::Null);
            }
            Ok(serde_json::from_slice(govde).unwrap_or_else(|_| Value::String(metin())))
        }
        408 | 429 | 500..=599 => Err(GonderHata::Ulasilamadi(format!(
            "HTTP {kod}: {}. Öneri: sunucu toparlanınca yeniden deneyin.",
            metin()
        ))),
        400..=499 => Err(GonderHata::Reddedildi {
            kod,
            mesaj: metin(),
        }),
        _ => Err(GonderHata::Ulasilamadi(format!("HTTP {kod}: {}", metin()))),
    }
}

impl CurlGonderici {
    /// Ortak istek: `yontem` POST (gövdeli) ya da GET (gövdesiz). Başlık dosyası iş bitince kaldırılır.
    fn istek(&self, yontem: &str, uc: &str, govde_bayt: &[u8]) -> Result<Value, GonderHata> {
        let url = format!("{}{}", self.taban, uc);
        let basliklar = self.basliklar(yontem, uc, govde_bayt)?;
        #[cfg(test)]
        {
            *SON_BASLIK_DOSYASI.lock().unwrap() = Some(basliklar.clone());
        }
        let sonuc = (|| {
            let mut cocuk = self
                .komut(yontem, &url, &basliklar)
                .stdin(if yontem == "POST" {
                    Stdio::piped()
                } else {
                    Stdio::null()
                })
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| {
                    GonderHata::Yapilandirma(format!(
                        "curl çalıştırılamadı ({e}). Öneri: Windows 10 1803+ ile gelir; yoksa curl'ü kurun."
                    ))
                })?;
            if let Some(mut g) = cocuk.stdin.take() {
                let _ = g.write_all(govde_bayt);
            }
            let c = cocuk
                .wait_with_output()
                .map_err(|e| GonderHata::Ulasilamadi(format!("curl beklenemedi: {e}")))?;
            if !c.status.success() {
                let m = String::from_utf8_lossy(&c.stderr)
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ");
                return Err(GonderHata::Ulasilamadi(format!(
                    "{url} (curl çıkış {}: {}). Öneri: ağı ve sunucuyu denetleyip yeniden deneyin.",
                    c.status.code().unwrap_or(-1),
                    if m.is_empty() { "ayrıntı yok" } else { &m }
                )));
            }
            let (g, kod) = cikti_ayir(&c.stdout).ok_or_else(|| {
                GonderHata::Ulasilamadi(format!("{url}: curl çıktısı çözülemedi"))
            })?;
            kodu_yorumla(kod, &g)
        })();
        // Anahtar içeren geçici başlık dosyası kalıcı olmasın (kişi verisi değil, bu crate'in kendi geçici dosyası).
        let _ = fs::remove_file(&basliklar);
        if let Some(k) = basliklar.parent() {
            let _ = fs::remove_dir(k);
        }
        sonuc
    }
}

impl Gonderici for CurlGonderici {
    fn gonder(&self, uc: &str, govde: &Value) -> Result<Value, GonderHata> {
        if !super::uc_gecerli(uc) {
            return Err(GonderHata::Yapilandirma(format!("geçersiz uç: {uc}")));
        }
        self.istek("POST", uc, &govde.to_string().into_bytes())
    }

    fn al(&self, uc: &str) -> Result<Value, GonderHata> {
        if !super::uc_sorgulu_gecerli(uc) {
            return Err(GonderHata::Yapilandirma(format!("geçersiz uç: {uc}")));
        }
        self.istek("GET", uc, &[])
    }
}
