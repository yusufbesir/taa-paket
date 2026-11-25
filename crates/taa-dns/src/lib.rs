// Sahip: Yusuf. yazan: [Claude#dinamik-adres 2026-11-25]
//! taa-dns: dinamik DNS güncelleyici (Yusuf 2026-11-25: "DNS için alt yapı; DuckDNS olur ya da kendin yaz").
//!
//! Sunucu PC'sinin genel IP'si değişirse, sağlayıcıdaki ad (ör. `taa-yusuf.duckdns.org`) yeni IP'yi göstersin diye
//! sağlayıcının güncelleme adresine düzenli istek atar. Sağlayıcı bir ayardır (kod değişmez):
//! - `duckdns`: `https://www.duckdns.org/update?domains=<alan>&token=<belirteç>&ip=` (IP boş: sağlayıcı kaynağı görür), yanıt `OK`/`KO`.
//! - `url`: kendi şablonunuz (`{alan}`, `{token}`, `{ip}` yer tutucuları, yalnız `https://`); HTTP 2xx başarı sayılır.
//!   (No-IP, Dynu, kendi sunucunuz vb. aynı biçimi kullanır.)
//!
//! Dosyalar: `<kök>/dns/ayar.json` (belirteçsiz: `{"saglayici","alan","sablon"?}`), `<kök>/dns/son.json` (son sonuç),
//! belirteç `<kök>/anahtarlar/dns-token` (tek satır; komut satırına VERİLMEZ). Belirteç curl'e komut satırından değil
//! standart girdi ile verilir (süreç listesinde görünmez), hata iletilerinde ve çıktıda maskelenir.

#[cfg(test)]
mod testler;

use serde_json::{json, Value};
use std::fmt;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const KLASOR: &str = "dns";
pub const AYAR_DOSYASI: &str = "ayar.json";
pub const SON_DOSYASI: &str = "son.json";
pub const TOKEN_DOSYASI: &str = "dns-token";
pub const DUCKDNS_TABAN: &str = "https://www.duckdns.org/update";
pub const BAGLANTI_SN: u32 = 8;
pub const TOPLAM_SN: u32 = 20;

#[derive(Debug)]
pub enum DnsHata {
    GecersizGirdi(String),
    Dosya(String),
    /// Ağ/curl hatası ya da sağlayıcı reddi (ileti maskeli).
    Guncellenemedi(String),
}

impl fmt::Display for DnsHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DnsHata::GecersizGirdi(m) => write!(f, "geçersiz girdi: {m}"),
            DnsHata::Dosya(m) => write!(f, "dns dosyası: {m}"),
            DnsHata::Guncellenemedi(m) => write!(f, "dns güncellenemedi: {m}"),
        }
    }
}

impl std::error::Error for DnsHata {}

#[derive(Debug, Clone, PartialEq)]
pub enum Saglayici {
    DuckDns,
    /// Şablon: `https://…{alan}…{token}…{ip}…`
    Url(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ayar {
    pub saglayici: Saglayici,
    /// DuckDNS'te alt alan adı (`taa-yusuf` ya da `taa-yusuf.duckdns.org`), `url` sağlayıcıda ana bilgisayar adı.
    pub alan: String,
}

/// Alan adı: küçük harf/rakam/`-`/`.`, 1..=100, `-`/`.` ile başlamaz/bitmez.
pub fn alan_gecerli(a: &str) -> bool {
    !a.is_empty()
        && a.len() <= 100
        && a.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'-' | b'.'))
        && !a.starts_with(['-', '.'])
        && !a.ends_with(['-', '.'])
}

/// Belirteç: harf/rakam/`-`/`_`, 8..=128 (tırnak, boşluk, `&`, `%` yok → komut/URL enjeksiyonu olmaz).
pub fn token_gecerli(t: &str) -> bool {
    (8..=128).contains(&t.len())
        && t.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_'))
}

/// IPv4 (isteğe bağlı, elle verilirse).
pub fn ip_gecerli(ip: &str) -> bool {
    ip.parse::<std::net::Ipv4Addr>().is_ok()
}

impl Ayar {
    pub fn yeni(saglayici: &str, alan: &str, sablon: Option<&str>) -> Result<Ayar, DnsHata> {
        let alan = alan.trim().to_ascii_lowercase();
        // DuckDNS'te `.duckdns.org` eki verilmişse atılır (sağlayıcı yalnız alt adı ister).
        let alan = if saglayici == "duckdns" {
            alan.strip_suffix(".duckdns.org")
                .unwrap_or(&alan)
                .to_string()
        } else {
            alan
        };
        if !alan_gecerli(&alan) {
            return Err(DnsHata::GecersizGirdi(format!(
                "alan adı geçersiz: {alan:?}"
            )));
        }
        let saglayici = match saglayici {
            "duckdns" => {
                if alan.contains('.') {
                    return Err(DnsHata::GecersizGirdi(
                        "duckdns alt adı noktasız olmalı (ör. taa-yusuf)".into(),
                    ));
                }
                Saglayici::DuckDns
            }
            "url" => {
                let s = sablon
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .ok_or_else(|| {
                        DnsHata::GecersizGirdi("url sağlayıcı için şablon gerekli".into())
                    })?;
                if !s.starts_with("https://")
                    || s.contains(char::is_whitespace)
                    || s.contains(['"', '\\'])
                    || !s.contains("{token}")
                {
                    return Err(DnsHata::GecersizGirdi(
                        "şablon https:// ile başlamalı, {token} içermeli, boşluk/tırnak içermemeli"
                            .into(),
                    ));
                }
                Saglayici::Url(s.to_string())
            }
            x => {
                return Err(DnsHata::GecersizGirdi(format!(
                    "sağlayıcı duckdns | url olmalı: {x:?}"
                )))
            }
        };
        Ok(Ayar { saglayici, alan })
    }

    pub fn json(&self) -> Value {
        match &self.saglayici {
            Saglayici::DuckDns => json!({"saglayici": "duckdns", "alan": self.alan}),
            Saglayici::Url(s) => json!({"saglayici": "url", "alan": self.alan, "sablon": s}),
        }
    }

    pub fn jsondan(m: &str) -> Result<Ayar, DnsHata> {
        let v: Value =
            serde_json::from_str(m).map_err(|e| DnsHata::Dosya(format!("çözülemedi: {e}")))?;
        let al = |a: &str| v.get(a).and_then(Value::as_str);
        Ayar::yeni(
            al("saglayici").ok_or_else(|| DnsHata::Dosya("'saglayici' yok".into()))?,
            al("alan").ok_or_else(|| DnsHata::Dosya("'alan' yok".into()))?,
            al("sablon"),
        )
    }

    /// Güncelleme adresi; belirteç içerir, YALNIZ curl'e standart girdi ile verilir.
    pub fn adres(&self, token: &str, ip: Option<&str>) -> Result<String, DnsHata> {
        if !token_gecerli(token) {
            return Err(DnsHata::GecersizGirdi("belirteç biçimi geçersiz".into()));
        }
        let ip = ip.unwrap_or("");
        if !ip.is_empty() && !ip_gecerli(ip) {
            return Err(DnsHata::GecersizGirdi(format!("IPv4 geçersiz: {ip:?}")));
        }
        Ok(match &self.saglayici {
            Saglayici::DuckDns => format!(
                "{DUCKDNS_TABAN}?domains={}&token={token}&ip={ip}",
                self.alan
            ),
            Saglayici::Url(s) => s
                .replace("{alan}", &self.alan)
                .replace("{token}", token)
                .replace("{ip}", ip),
        })
    }

    /// Sağlayıcı yanıtını yorumlar: DuckDNS gövdesi `OK` olmalı; `url` için HTTP 2xx yeter.
    pub fn yanit_tamam(&self, kod: u16, govde: &str) -> bool {
        match &self.saglayici {
            Saglayici::DuckDns => kod == 200 && govde.trim_start().starts_with("OK"),
            Saglayici::Url(_) => (200..300).contains(&kod),
        }
    }
}

/// Belirteci metinden siler (hata iletileri için).
pub fn maskele(metin: &str, token: &str) -> String {
    if token.is_empty() {
        metin.to_string()
    } else {
        metin.replace(token, "***")
    }
}

pub fn ayar_yolu(kok: &Path) -> PathBuf {
    kok.join(KLASOR).join(AYAR_DOSYASI)
}

pub fn token_yolu(kok: &Path) -> PathBuf {
    kok.join("anahtarlar").join(TOKEN_DOSYASI)
}

fn son_yolu(kok: &Path) -> PathBuf {
    kok.join(KLASOR).join(SON_DOSYASI)
}

pub fn ayar_oku(kok: &Path) -> Result<Option<Ayar>, DnsHata> {
    match std::fs::read_to_string(ayar_yolu(kok)) {
        Ok(m) => Ayar::jsondan(&m).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(DnsHata::Dosya(e.to_string())),
    }
}

pub fn ayar_yaz(kok: &Path, a: &Ayar) -> Result<PathBuf, DnsHata> {
    std::fs::create_dir_all(kok.join(KLASOR)).map_err(|e| DnsHata::Dosya(e.to_string()))?;
    let y = ayar_yolu(kok);
    let m = serde_json::to_string_pretty(&a.json()).map_err(|e| DnsHata::Dosya(e.to_string()))?;
    taa_dosya::uzerine_yaz_atomik(&y, m.as_bytes()).map_err(|e| DnsHata::Dosya(e.to_string()))?;
    Ok(y)
}

/// Belirteç dosyasını okur (ilk satır, kırpılmış); yoksa/biçim geçersizse hata (içeriği ileti yazmaz).
pub fn token_oku(kok: &Path) -> Result<String, DnsHata> {
    let m = std::fs::read_to_string(token_yolu(kok)).map_err(|_| {
        DnsHata::Dosya(format!(
            "belirteç dosyası okunamadı: {} (belirteci yalnız siz bu dosyaya yazın)",
            token_yolu(kok).display()
        ))
    })?;
    let t = m.lines().next().unwrap_or("").trim().to_string();
    if !token_gecerli(&t) {
        return Err(DnsHata::Dosya(
            "belirteç dosyasındaki değer geçersiz".into(),
        ));
    }
    Ok(t)
}

/// İstek yapıcı (testlerde sahte): adres gövdesi → (HTTP kodu, gövde).
pub trait Istekci {
    fn al(&self, adres: &str) -> Result<(u16, String), String>;
}

/// curl ile: adres komut satırına yazılmaz, `-K -` ile standart girdiden verilir.
#[derive(Debug, Default, Clone, Copy)]
pub struct CurlIstekci;

impl Istekci for CurlIstekci {
    fn al(&self, adres: &str) -> Result<(u16, String), String> {
        let mut k = Command::new("curl");
        k.args([
            "--silent",
            "--show-error",
            "--connect-timeout",
            &BAGLANTI_SN.to_string(),
            "--max-time",
            &TOPLAM_SN.to_string(),
            "--max-filesize",
            "65536",
            "-w",
            "\n%{http_code}",
            "-K",
            "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            k.creation_flags(0x0800_0000);
        }
        let mut c = k.spawn().map_err(|e| format!("curl çalışmadı: {e}"))?;
        if let Some(mut s) = c.stdin.take() {
            let _ = writeln!(s, "url = \"{adres}\"");
        }
        let o = c.wait_with_output().map_err(|e| e.to_string())?;
        if !o.status.success() {
            return Err(String::from_utf8_lossy(&o.stderr).trim().to_string());
        }
        let m = String::from_utf8_lossy(&o.stdout).to_string();
        let (govde, kod) = m.rsplit_once('\n').unwrap_or(("", &m));
        let kod: u16 = kod
            .trim()
            .parse()
            .map_err(|_| "yanıt kodu okunamadı".to_string())?;
        Ok((kod, govde.to_string()))
    }
}

/// Güncellemeyi yapar; sonuç `son.json`a yazılır (belirteçsiz). Dönüş: kısa ileti.
pub fn guncelle(kok: &Path, ip: Option<&str>, i: &dyn Istekci) -> Result<String, DnsHata> {
    let a = ayar_oku(kok)?.ok_or_else(|| {
        DnsHata::Dosya(
            "dns ayarı yok. Öneri: taa-dns ayarla --saglayici duckdns --alan <ad>".into(),
        )
    })?;
    let t = token_oku(kok)?;
    let adres = a.adres(&t, ip)?;
    let sonuc = match i.al(&adres) {
        Ok((kod, govde)) if a.yanit_tamam(kod, &govde) => Ok(format!("{} güncellendi", a.alan)),
        Ok((kod, govde)) => Err(DnsHata::Guncellenemedi(maskele(
            &format!("sağlayıcı yanıtı reddedildi (HTTP {kod}): {}", govde.trim()),
            &t,
        ))),
        Err(e) => Err(DnsHata::Guncellenemedi(maskele(&e, &t))),
    };
    let kayit = match &sonuc {
        Ok(m) => json!({"tamam": true, "ileti": m}),
        Err(e) => json!({"tamam": false, "ileti": e.to_string()}),
    };
    if std::fs::create_dir_all(kok.join(KLASOR)).is_ok() {
        let _ = taa_dosya::uzerine_yaz_atomik(&son_yolu(kok), kayit.to_string().as_bytes());
    }
    sonuc
}

pub fn son_oku(kok: &Path) -> Option<Value> {
    serde_json::from_str(&std::fs::read_to_string(son_yolu(kok)).ok()?).ok()
}
