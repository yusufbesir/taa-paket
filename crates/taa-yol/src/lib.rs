// Sahip: Yusuf. yazan: [Claude#taa-yol-S4a 2026-11-07]
//! taa-yol: merkez sunucuya **birden çok ulaşım yolu** (Yusuf 2026-11-07 03:15: "hepsi olsun").
//!
//! Yollar sıralı bir listedir; program ilk ulaşabildiği yolu kullanır, ulaşamazsa sonrakine düşer:
//! 1. doğrudan `https://<statik IP>` + kendi sertifikamız (Caddy iç CA kökü, `<kök>/anahtarlar/sunucu-ca.pem`),
//! 2. VPN iç adresi (WireGuard, `https://10.x.x.x`) + aynı sertifika,
//! 3. alan adı (`https://sunucu.<alan>`) + Let's Encrypt → sertifika `"sistem"` (işletim sistemi kökleri),
//! 4. Cloudflare Tunnel adresi → `"sistem"`.
//!
//! Hangi yol seçilirse seçilsin cihaz imzası, parola ve yetki belgesi aynıdır; yol yalnız "nereden gidileceği"dir.
//!
//! Dosyalar (veri alanı `yol`, sahibi taa-yol): `<kök>/yol/sunucu-yollari.json`
//! `{"sunucu": "merkez", "yollar": [{"ad", "taban", "sertifika"?}]}` ve `<kök>/yol/son.json` (son çalışan yolun adı;
//! bir sonraki seçimde önce o denenir). `sunucu` oturum deposu ve yetki belgesi anahtarıdır: yol değişse de oturum
//! aynı kalır. Dosya yoksa eski davranış sürer (`--sunucu` / `TAA_SUNUCU` tek adres).
//!
//! Ulaşılabilirlik denemesi: `GET /v1/saglik`. Sunucudan **herhangi bir HTTP yanıtı** (401/403 dahil) "ulaşıldı"
//! sayılır; yalnız ağ hatası/zaman aşımı "ulaşılamadı"dır. Böylece sağlık ucu olmayan sunucu da seçilir.

#[cfg(test)]
mod testler;

use serde_json::{json, Value};
use std::cell::Cell;
use std::fmt;
use std::path::{Path, PathBuf};
use taa_kuyruk::{CurlGonderici, GonderHata, Gonderici};

pub const OZELLIK: &str = "yol";
pub const KLASOR: &str = "yol";
pub const DOSYA: &str = "sunucu-yollari.json";
pub const SON_DOSYA: &str = "son.json";
pub const SAGLIK_UCU: &str = "/v1/saglik";
/// `sertifika` alanında bu değer: işletim sistemi kök sertifikaları (Let's Encrypt, Cloudflare).
pub const SERTIFIKA_SISTEM: &str = "sistem";
/// Varsayılan sertifika dosyası (`<kök>/anahtarlar/` altında): Caddy iç CA kökü.
pub const SERTIFIKA_VARSAYILAN: &str = "sunucu-ca.pem";
/// Yol denemesinde bağlantı kurma süresi (saniye); ulaşılamayan IP'de beklememek için kısa.
pub const DENEME_BAGLANTI_SN: u64 = 5;
pub const AD_EN_COK: usize = 32;
pub const YOL_EN_COK: usize = 16;

#[derive(Debug)]
pub enum YolHata {
    GecersizGirdi(String),
    Dosya(String),
    /// Listedeki hiçbir yola ulaşılamadı.
    Ulasilamadi(Vec<String>),
    /// Yol listesi yok ve `--sunucu` / `TAA_SUNUCU` verilmedi.
    HedefYok,
}

impl fmt::Display for YolHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            YolHata::GecersizGirdi(m) => write!(f, "geçersiz girdi: {m}"),
            YolHata::Dosya(m) => write!(f, "yol dosyası: {m}"),
            YolHata::Ulasilamadi(adlar) => write!(
                f,
                "hiçbir sunucu yoluna ulaşılamadı ({}). Öneri: ağı denetleyin ya da taa-yol dene",
                adlar.join(", ")
            ),
            YolHata::HedefYok => write!(
                f,
                "sunucu adresi yok: --sunucu (ya da TAA_SUNUCU) verin ya da yol listesi kurun (taa-yol ekle / taa-kur yol)"
            ),
        }
    }
}

impl std::error::Error for YolHata {}

/// Bir ulaşım yolu.
#[derive(Debug, Clone, PartialEq)]
pub struct Yol {
    /// Kısa ad (`dogrudan`, `vpn`, `alan-adi`, `cloudflare`…): `[a-z0-9][a-z0-9_-]*`, en çok 32.
    pub ad: String,
    /// `https://adres[:port]` (yerel ağ denemesinde `http://`).
    pub taban: String,
    /// `None` → `sunucu-ca.pem`; `"sistem"` → işletim sistemi kökleri; başka → dosya adı/yolu.
    pub sertifika: Option<String>,
}

pub fn ad_gecerli(a: &str) -> bool {
    !a.is_empty()
        && a.len() <= AD_EN_COK
        && a.bytes()
            .next()
            .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
        && a.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || matches!(b, b'_' | b'-'))
}

impl Yol {
    pub fn yeni(ad: &str, taban: &str, sertifika: Option<&str>) -> Result<Yol, YolHata> {
        if !ad_gecerli(ad) {
            return Err(YolHata::GecersizGirdi(format!(
                "yol adı küçük harf/rakam/'-'/'_' (1..=32) olmalı: {ad:?}"
            )));
        }
        let t = taban.trim().trim_end_matches('/');
        if !taa_kuyruk::http_url_mu(t) || t.contains(char::is_whitespace) {
            return Err(YolHata::GecersizGirdi(format!(
                "taban http:// ya da https:// ile başlamalı: {taban:?}"
            )));
        }
        let sertifika = match sertifika.map(str::trim) {
            None | Some("") => None,
            Some(s) if s.chars().any(char::is_control) => {
                return Err(YolHata::GecersizGirdi(
                    "sertifika adı denetim karakteri içeriyor".into(),
                ))
            }
            Some(s) => Some(s.to_string()),
        };
        Ok(Yol {
            ad: ad.to_string(),
            taban: t.to_string(),
            sertifika,
        })
    }

    /// Bu yol için curl'e verilecek CA dosyası: `None` = sistem kökleri. Göreli ad `<kök>/anahtarlar/` altındadır.
    pub fn sertifika_yolu(&self, kok: &Path) -> Option<PathBuf> {
        match self.sertifika.as_deref() {
            Some(SERTIFIKA_SISTEM) => None,
            None => Some(kok.join("anahtarlar").join(SERTIFIKA_VARSAYILAN)),
            Some(s) => {
                let p = Path::new(s);
                Some(if p.is_absolute() {
                    p.to_path_buf()
                } else {
                    kok.join("anahtarlar").join(p)
                })
            }
        }
    }

    fn json(&self) -> Value {
        let mut v = json!({ "ad": self.ad, "taban": self.taban });
        if let Some(s) = &self.sertifika {
            v["sertifika"] = Value::String(s.clone());
        }
        v
    }

    fn jsondan(v: &Value) -> Result<Yol, YolHata> {
        let al = |a: &str| v.get(a).and_then(Value::as_str);
        Yol::yeni(
            al("ad").ok_or_else(|| YolHata::Dosya("yolda 'ad' yok".into()))?,
            al("taban").ok_or_else(|| YolHata::Dosya("yolda 'taban' yok".into()))?,
            al("sertifika"),
        )
    }
}

/// Yol listesi: `sunucu` mantıksal ad (oturum deposu anahtarı), `yollar` öncelik sırasıyla.
#[derive(Debug, Clone, PartialEq)]
pub struct Yollar {
    pub sunucu: String,
    pub yollar: Vec<Yol>,
}

impl Yollar {
    pub fn yeni(sunucu: &str) -> Result<Yollar, YolHata> {
        if !ad_gecerli(sunucu) {
            return Err(YolHata::GecersizGirdi(format!(
                "sunucu adı küçük harf/rakam/'-'/'_' (1..=32) olmalı: {sunucu:?}"
            )));
        }
        Ok(Yollar {
            sunucu: sunucu.to_string(),
            yollar: Vec::new(),
        })
    }

    /// Aynı ad varsa yerine yazar (sıra korunur); yoksa sona ekler.
    pub fn ekle(&mut self, y: Yol) -> Result<(), YolHata> {
        if let Some(e) = self.yollar.iter_mut().find(|e| e.ad == y.ad) {
            *e = y;
            return Ok(());
        }
        if self.yollar.len() >= YOL_EN_COK {
            return Err(YolHata::GecersizGirdi(format!("en çok {YOL_EN_COK} yol")));
        }
        self.yollar.push(y);
        Ok(())
    }

    /// Yolu listeden çıkarır (dosya silinmez; liste güncellenir). Yoksa false.
    pub fn kaldir(&mut self, ad: &str) -> bool {
        let n = self.yollar.len();
        self.yollar.retain(|y| y.ad != ad);
        self.yollar.len() != n
    }

    /// `ad`ı başa alır (öncelik).
    pub fn one_al(&mut self, ad: &str) -> bool {
        if let Some(i) = self.yollar.iter().position(|y| y.ad == ad) {
            let y = self.yollar.remove(i);
            self.yollar.insert(0, y);
            true
        } else {
            false
        }
    }

    pub fn json(&self) -> Value {
        json!({ "sunucu": self.sunucu, "yollar": self.yollar.iter().map(Yol::json).collect::<Vec<_>>() })
    }

    pub fn jsondan(metin: &str) -> Result<Yollar, YolHata> {
        let v: Value =
            serde_json::from_str(metin).map_err(|e| YolHata::Dosya(format!("çözülemedi: {e}")))?;
        let sunucu = v
            .get("sunucu")
            .and_then(Value::as_str)
            .ok_or_else(|| YolHata::Dosya("'sunucu' yok".into()))?;
        let mut y = Yollar::yeni(sunucu)?;
        for e in v
            .get("yollar")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            y.ekle(Yol::jsondan(e)?)?;
        }
        Ok(y)
    }
}

pub fn dosya_yolu(kok: &Path) -> PathBuf {
    kok.join(KLASOR).join(DOSYA)
}

fn son_dosyasi(kok: &Path) -> PathBuf {
    kok.join(KLASOR).join(SON_DOSYA)
}

/// `<kök>/yol/sunucu-yollari.json` okur; dosya yoksa `Ok(None)`, bozuksa hata (varsayılan: kullanma).
pub fn yollar_oku(kok: &Path) -> Result<Option<Yollar>, YolHata> {
    match std::fs::read_to_string(dosya_yolu(kok)) {
        Ok(m) => Yollar::jsondan(&m).map(Some),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(YolHata::Dosya(e.to_string())),
    }
}

/// Atomik yazar (taa-dosya); klasörü oluşturur. Önceki dosya silinmez, üzerine yazılır.
pub fn yollar_yaz(kok: &Path, y: &Yollar) -> Result<PathBuf, YolHata> {
    let yol = dosya_yolu(kok);
    std::fs::create_dir_all(kok.join(KLASOR)).map_err(|e| YolHata::Dosya(e.to_string()))?;
    let metin =
        serde_json::to_string_pretty(&y.json()).map_err(|e| YolHata::Dosya(e.to_string()))?;
    taa_dosya::uzerine_yaz_atomik(&yol, metin.as_bytes())
        .map_err(|e| YolHata::Dosya(e.to_string()))?;
    Ok(yol)
}

/// Son çalışan yolun adı (yoksa None; bozuksa None).
pub fn son_oku(kok: &Path) -> Option<String> {
    let m = std::fs::read_to_string(son_dosyasi(kok)).ok()?;
    let v: Value = serde_json::from_str(&m).ok()?;
    v.get("ad").and_then(Value::as_str).map(str::to_string)
}

fn son_yaz(kok: &Path, ad: &str) {
    // Başarısızlık seçimi bozmaz; yalnız hızlandırma.
    if std::fs::create_dir_all(kok.join(KLASOR)).is_ok() {
        let v = json!({ "ad": ad, "zaman": taa_kuyruk::zaman::simdi() });
        let _ = taa_dosya::uzerine_yaz_atomik(&son_dosyasi(kok), v.to_string().as_bytes());
    }
}

/// Ulaşılabilirlik denemesi (testlerde sahte).
pub trait Deneyici {
    fn ulasilabilir(&self, taban: &str, sertifika: Option<&Path>) -> bool;
}

/// curl ile `GET /v1/saglik`: HTTP yanıtı geldiyse (kod ne olursa olsun) ulaşıldı; ağ hatası → ulaşılamadı.
#[derive(Debug, Default, Clone, Copy)]
pub struct CurlDeneyici;

impl Deneyici for CurlDeneyici {
    fn ulasilabilir(&self, taban: &str, sertifika: Option<&Path>) -> bool {
        let Ok(mut g) = CurlGonderici::yeni(taban) else {
            return false;
        };
        g = g.baglanti_suresi(DENEME_BAGLANTI_SN);
        if let Some(c) = sertifika {
            g = g.sertifika(c);
        }
        yanit_ulasildi_mi(&g.al(SAGLIK_UCU))
    }
}

/// Sunucudan yanıt geldi mi? `Ok` ya da `Reddedildi` (4xx) evet; `Ulasilamadi`/`Yapilandirma` hayır.
pub fn yanit_ulasildi_mi(r: &Result<Value, GonderHata>) -> bool {
    matches!(r, Ok(_) | Err(GonderHata::Reddedildi { .. }))
}

/// Sıra: önce son çalışan yol, sonra liste sırası. İlk ulaşılan döner ve `son.json`a yazılır.
pub fn sec(kok: &Path, y: &Yollar, d: &dyn Deneyici) -> Result<Yol, YolHata> {
    if y.yollar.is_empty() {
        return Err(YolHata::HedefYok);
    }
    let son = son_oku(kok);
    let mut sira: Vec<&Yol> = Vec::with_capacity(y.yollar.len());
    if let Some(s) = &son {
        sira.extend(y.yollar.iter().filter(|v| &v.ad == s));
    }
    sira.extend(y.yollar.iter().filter(|v| Some(&v.ad) != son.as_ref()));
    for v in sira {
        if d.ulasilabilir(&v.taban, v.sertifika_yolu(kok).as_deref()) {
            son_yaz(kok, &v.ad);
            return Ok(v.clone());
        }
    }
    Err(YolHata::Ulasilamadi(
        y.yollar.iter().map(|v| v.ad.clone()).collect(),
    ))
}

/// Yedekli gönderici: her istek için yolları sırayla dener. Bir yol bağlantı hatası verirse (`Ulasilamadi`,
/// `Yapilandirma`) AYNI istek sıradaki yolla yeniden gönderilir; sunucudan yanıt gelen (başarı ya da 4xx) yol
/// "aktif" olur ve `son.json`a yazılır, sonraki istekler oradan başlar. Böylece bir yol (ör. Tailscale) düşünce
/// iletişim kesilmeden başka yola (Caddy/doğrudan/alan adı/Cloudflare) geçilir; yol geri gelince de çalışan yolda
/// kalınır (gereksiz geçiş yok). Dönüş: ilk anlamlı yanıt; hepsi başarısızsa son hata (hangi yolların denendiği mesajda).
/// Not: 5xx/zaman aşımı da `Ulasilamadi` sayıldığından aynı kayıt başka yoldan tekrar gelebilir; sunucu tarafında
/// kayıtlar kimlikle (idempotent) tekilleştirilir (taa-kuyruk).
pub struct YedekliGonderici {
    kok: PathBuf,
    adaylar: Vec<(String, Box<dyn Gonderici>)>,
    aktif: Cell<usize>,
}

impl YedekliGonderici {
    /// `adaylar` önceliğe göre sıralı (ad, gönderici). Boş liste kurulamaz.
    pub fn yeni(
        kok: &Path,
        adaylar: Vec<(String, Box<dyn Gonderici>)>,
    ) -> Result<YedekliGonderici, YolHata> {
        if adaylar.is_empty() {
            return Err(YolHata::HedefYok);
        }
        Ok(YedekliGonderici {
            kok: kok.to_path_buf(),
            adaylar,
            aktif: Cell::new(0),
        })
    }

    /// Şu an aktif yolun adı.
    pub fn aktif_yol(&self) -> &str {
        &self.adaylar[self.aktif.get()].0
    }

    fn dene(
        &self,
        istek: &dyn Fn(&dyn Gonderici) -> Result<Value, GonderHata>,
    ) -> Result<Value, GonderHata> {
        let n = self.adaylar.len();
        let baslangic = self.aktif.get();
        let mut hatalar: Vec<String> = Vec::new();
        let mut son_hata = None;
        for i in 0..n {
            let idx = (baslangic + i) % n;
            let (ad, g) = &self.adaylar[idx];
            match istek(g.as_ref()) {
                Err(e @ (GonderHata::Ulasilamadi(_) | GonderHata::Yapilandirma(_))) => {
                    hatalar.push(format!("{ad}: {e}"));
                    son_hata = Some(e);
                }
                sonuc => {
                    if idx != baslangic {
                        self.aktif.set(idx);
                        son_yaz(&self.kok, ad);
                    }
                    return sonuc;
                }
            }
        }
        // Hepsi düştü: aktif yol değişmez; hata türü korunur, mesaja denenen yollar yazılır.
        let ozet = hatalar.join("; ");
        Err(match son_hata {
            Some(GonderHata::Yapilandirma(_)) => GonderHata::Yapilandirma(ozet),
            _ => GonderHata::Ulasilamadi(format!("tüm yollar denendi ({ozet})")),
        })
    }
}

impl Gonderici for YedekliGonderici {
    fn gonder(&self, uc: &str, govde: &Value) -> Result<Value, GonderHata> {
        self.dene(&|g| g.gonder(uc, govde))
    }
    fn al(&self, uc: &str) -> Result<Value, GonderHata> {
        self.dene(&|g| g.al(uc))
    }
}

/// Yol başına gönderici üreten fabrika (ikinci bağımsız değişken: yolun CA dosyası).
pub type GondericiFabrikasi<'a> =
    &'a dyn Fn(&Yol, Option<PathBuf>) -> Result<Box<dyn Gonderici>, GonderHata>;

/// Yol listesi varsa yedekli göndericiyi kurar (sıra: son çalışan yol, sonra liste); liste yoksa `None`
/// (çağıran eski tek adres davranışına döner). `fabrika` her yol için göndericiyi (oturum, imza, sertifika dahil) üretir;
/// ikinci bağımsız değişken yolun CA dosyasıdır (`None` = sistem kökleri).
pub fn yedekli_kur(
    kok: &Path,
    fabrika: GondericiFabrikasi,
) -> Result<Option<YedekliGonderici>, YolHata> {
    let Some(y) = yollar_oku(kok)? else {
        return Ok(None);
    };
    let son = son_oku(kok);
    let mut sira: Vec<&Yol> = Vec::with_capacity(y.yollar.len());
    if let Some(s) = &son {
        sira.extend(y.yollar.iter().filter(|v| &v.ad == s));
    }
    sira.extend(y.yollar.iter().filter(|v| Some(&v.ad) != son.as_ref()));
    let mut adaylar = Vec::new();
    for v in sira {
        if let Ok(g) = fabrika(v, v.sertifika_yolu(kok)) {
            adaylar.push((v.ad.clone(), g));
        }
    }
    YedekliGonderici::yeni(kok, adaylar).map(Some)
}

/// Çözülmüş hedef: `anahtar` oturum deposu/yetki belgesi anahtarı, `taban` istek adresi, `sertifika` CA dosyası.
#[derive(Debug, Clone, PartialEq)]
pub struct Hedef {
    pub anahtar: String,
    pub taban: String,
    pub sertifika: Option<PathBuf>,
    /// Yol listesinden seçildiyse yolun adı; `--sunucu` ile verildiyse None.
    pub yol: Option<String>,
}

/// Hedef çözümü. Öncelik: `acik_sunucu` (`--sunucu`) → `ortam_sunucu` (`TAA_SUNUCU`) → yol listesi.
/// İlk ikisinde anahtar = adres (eski davranış) ve sertifika = `acik_sertifika`. Yol listesinde anahtar =
/// `yollar.sunucu`, sertifika = `acik_sertifika` varsa o, yoksa yolun sertifikası.
pub fn hedef_coz(
    kok: &Path,
    acik_sunucu: Option<&str>,
    ortam_sunucu: Option<&str>,
    acik_sertifika: Option<&Path>,
    d: &dyn Deneyici,
) -> Result<Hedef, YolHata> {
    let acik = acik_sunucu
        .or(ortam_sunucu)
        .map(str::trim)
        .filter(|s| !s.is_empty());
    if let Some(s) = acik {
        return Ok(Hedef {
            anahtar: s.to_string(),
            taban: s.to_string(),
            sertifika: acik_sertifika.map(Path::to_path_buf),
            yol: None,
        });
    }
    let y = yollar_oku(kok)?.ok_or(YolHata::HedefYok)?;
    let v = sec(kok, &y, d)?;
    Ok(Hedef {
        anahtar: y.sunucu.clone(),
        taban: v.taban.clone(),
        sertifika: acik_sertifika
            .map(Path::to_path_buf)
            .or_else(|| v.sertifika_yolu(kok)),
        yol: Some(v.ad.clone()),
    })
}

/// Yalnız oturum/yetki anahtarı (ağa çıkmadan; çevrimdışı komutlar için): `--sunucu` → `TAA_SUNUCU` → listenin
/// `sunucu` adı.
pub fn anahtar_coz(
    kok: &Path,
    acik_sunucu: Option<&str>,
    ortam_sunucu: Option<&str>,
) -> Result<String, YolHata> {
    if let Some(s) = acik_sunucu
        .or(ortam_sunucu)
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Ok(s.to_string());
    }
    Ok(yollar_oku(kok)?.ok_or(YolHata::HedefYok)?.sunucu)
}

/// `anahtar_coz` + `TAA_SUNUCU`.
pub fn anahtar_coz_ortam(kok: &Path, acik_sunucu: Option<&str>) -> Result<String, YolHata> {
    let ortam = std::env::var("TAA_SUNUCU").ok();
    anahtar_coz(kok, acik_sunucu, ortam.as_deref())
}

/// `hedef_coz` + `TAA_SUNUCU` ortam değişkeni + curl deneyicisi.
pub fn hedef_coz_curl(
    kok: &Path,
    acik_sunucu: Option<&str>,
    acik_sertifika: Option<&Path>,
) -> Result<Hedef, YolHata> {
    let ortam = std::env::var("TAA_SUNUCU").ok();
    hedef_coz(
        kok,
        acik_sunucu,
        ortam.as_deref(),
        acik_sertifika,
        &CurlDeneyici,
    )
}

/// Çıkış kodu: 2 girdi/dosya, 3 ulaşılamadı, 5 hedef yok.
pub fn hata_kodu(e: &YolHata) -> u8 {
    match e {
        YolHata::GecersizGirdi(_) | YolHata::Dosya(_) => 2,
        YolHata::Ulasilamadi(_) => 3,
        YolHata::HedefYok => 5,
    }
}
