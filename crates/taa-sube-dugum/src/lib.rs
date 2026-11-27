// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-sube-dugum 2026-11-27]
//! taa-sube-dugum: şube bilgisayarında çalışan yerel düğüm (docs/SUBE-DUGUMU.md "Uygulama").
//!
//! Yusuf'un kuralları (2026-11-27): çevrimiçiyken cihazlar doğrudan merkeze gönderir; merkeze ulaşamayan cihaz
//! şube bilgisayarına (bu düğüme) yazar. Düğüm o şubenin sunucusu gibi çalışır ama **merkezin yerini tutmaz**:
//! aldığı her kayıt aynı `no` ile `taa-kuyruk`a eklenir ve arka planda merkeze gönderilir.
//!
//! Dosyalar (hepsi `<veri kökü>` altında; hiçbir satır silinmez, JSONL dosyaları yalnız eklenir):
//! - `kuyruk/sube-dugum/` — `taa_kuyruk` kuyruğu (giden / gonderildi / reddedildi); gövdeler şifreli saklanır.
//! - `kuyruk/sube-dugum-gunluk/` — günlük özetlerinin merkeze gidecek kuyruğu (asıl kayıtları bekletmesin diye ayrı).
//! - `dugum/surum.jsonl` — yerel sürüm sayacı: `{"surum","no","olay","zaman"}` (`alindi|gonderildi|reddedildi`).
//! - `dugum/cakisma.jsonl` — çakışma kutusu: aynı `no`, farklı içerik. Düğümdeki (ilk gelen) hal geçerli kalır.
//! - `dugum/durum.json` — merkezle son başarılı bağlantı zamanı (atomik yazılır).
//! - `gunluk/YYYY-MM-DD.jsonl` — günlük (UTC gün); yalnız ekleme. Kayıt içeriği günlüğe yazılmaz.
//!
//! Kimlik: bu crate cihaz/kişi doğrulaması **yapmaz**; iki kanca bırakır (bkz. [`GelenDenetim`] ve
//! `merkez::baslikli`). Özellik anahtarı: "sube-dugum".

pub mod gunluk;
pub mod merkez;
pub mod sunucu;

#[cfg(test)]
mod testler;

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Condvar, Mutex};
use std::time::Duration;
use taa_kuyruk::{sifre, zaman, Durum, Ekleme, Kuyruk, KuyrukHata};

pub const OZELLIK: &str = "sube-dugum";
/// Düğümün varsayılan kapısı (sözleşme).
pub const PORT_VARSAYILAN: u16 = 8788;
/// `taa_kuyruk` program adı: asıl kayıtlar.
pub const KUYRUK_PROGRAM: &str = "sube-dugum";
/// `taa_kuyruk` program adı: günlük özetleri.
pub const GUNLUK_KUYRUK_PROGRAM: &str = "sube-dugum-gunluk";
/// Düğümün kendi veri klasörü (`<kök>/dugum/`).
pub const KLASOR: &str = "dugum";
/// Bir isteğin gövdesi en çok bu kadar bayt olabilir (aşımda 413).
pub const GOVDE_EN_COK: usize = 256 * 1024;
pub const ESITLE_LIMIT_VARSAYILAN: usize = 200;
pub const ESITLE_LIMIT_EN_COK: usize = 500;
pub const SUBE_EN_COK: usize = 32;

/// Düğümün kuyruğa almadığı uçlar: oturum, yetki ve cihaz uçları yalnız merkezde yanıtlanır. Özellikle
/// `/v1/giris` gövdesi parola taşır; düğüm onu hiçbir dosyaya yazmaz.
pub const KAPALI_UCLAR: &[&str] = &[
    "/v1/giris",
    "/v1/cikis",
    "/v1/yetki",
    "/v1/cihaz",
    "/v1/esitle",
    "/v1/saglik",
    "/v1/yol-bilgisi",
    "/v1/gunluk",
    "/v1/cakisma",
];

#[derive(Debug)]
pub enum DugumHata {
    /// Şube kodu `[A-Za-z0-9_-]`, 1..=32 kuralına uymuyor.
    GecersizSube(String),
    Kuyruk(KuyrukHata),
    Io(io::Error),
}

impl fmt::Display for DugumHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DugumHata::GecersizSube(s) => write!(
                f,
                "geçersiz şube kodu (yalnız [A-Za-z0-9_-], en çok {SUBE_EN_COK}): {s}"
            ),
            DugumHata::Kuyruk(e) => write!(f, "kuyruk hatası: {e}"),
            DugumHata::Io(e) => write!(f, "dosya hatası: {e}"),
        }
    }
}

impl std::error::Error for DugumHata {}

impl From<KuyrukHata> for DugumHata {
    fn from(e: KuyrukHata) -> Self {
        DugumHata::Kuyruk(e)
    }
}

impl From<io::Error> for DugumHata {
    fn from(e: io::Error) -> Self {
        DugumHata::Io(e)
    }
}

pub fn sube_gecerli(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= SUBE_EN_COK
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

/// `<kök>/sube.json` (`{"kod": "MRK01"}`; TAA Kur yazar, taa-fis-sunucu da aynı dosyayı okur). Yoksa/bozuksa None.
pub fn sube_oku(kok: &Path) -> Option<String> {
    let m = std::fs::read_to_string(kok.join("sube.json")).ok()?;
    let v: Value = serde_json::from_str(&m).ok()?;
    let kod = v.get("kod")?.as_str()?.trim().to_string();
    sube_gecerli(&kod).then_some(kod)
}

/// Yazma isteğine verilen kalıcı ret (istemci yeniden denemez). Mesaj kişi verisi ve başka şube adı içermez.
#[derive(Debug, Clone, PartialEq)]
pub struct Ret {
    pub kod: u16,
    pub hata: &'static str,
    pub mesaj: String,
}

impl Ret {
    pub fn yeni(kod: u16, hata: &'static str, mesaj: &str) -> Ret {
        Ret {
            kod,
            hata,
            mesaj: mesaj.to_string(),
        }
    }
}

/// Gelen yazma isteğinin denetime verilen özeti (gövde dahil; kanca yetki belgesini gövdeden de okuyabilir).
pub struct GelenIstek<'a> {
    pub yontem: &'a str,
    /// Sorgu dahil yol (`/v1/kasa/sayim`).
    pub yol: &'a str,
    /// (ad, değer); adlar küçük harfe çevrilmiştir.
    pub basliklar: &'a [(String, String)],
    pub govde: &'a [u8],
}

/// **Kanca:** gelen isteğin cihaz/kişi denetimi (eşleşmiş cihaz, cihaz imzası, 7 günlük yetki belgesi;
/// docs/SUBE-DUGUMU.md §6). Bu crate hiçbir doğrulama uydurmaz: varsayılan [`DenetimYok`] her isteği geçirir.
/// `taa-cihaz` (`taa_cihaz::dogrula`) ve yetki belgesi doğrulaması bağlanınca burası doldurulur.
pub trait GelenDenetim: Send + Sync {
    fn denetle(&self, istek: &GelenIstek) -> Result<(), Ret>;
}

/// Denetimsiz varsayılan. Yerel ağ açıkken herkes yazabilir demektir; bkz. docs/SUBE-DUGUMU.md "Uygulama" açık noktalar.
pub struct DenetimYok;

impl GelenDenetim for DenetimYok {
    fn denetle(&self, _istek: &GelenIstek) -> Result<(), Ret> {
        Ok(())
    }
}

/// Yazma sonucu.
#[derive(Debug, Clone, PartialEq)]
pub enum Yazma {
    /// Kayıt diske yazıldı ve merkez kuyruğuna girdi.
    Yazildi { surum: u64 },
    /// Aynı `no`, aynı içerik: ikinci satır yazılmadı.
    ZatenVar { surum: u64, kesin: bool },
    /// Aynı `no`, farklı içerik: düğümdeki hal geçerli; gelen hal çakışma kutusuna yazıldı.
    Cakisma,
}

/// `GET /durum` gövdesinin kaynağı.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DurumOzeti {
    pub bekleyen: usize,
    pub gonderilen: usize,
    pub reddedilen: usize,
    pub son_merkez_zamani: Option<String>,
    pub cakisma: usize,
    pub surum: u64,
    pub gunluk_bekleyen: usize,
}

/// Sürüm sayacına yeni yazılan olay (`surum_isle` döndürür; merkez döngüsü günlüğe geçirir).
#[derive(Debug, Clone, PartialEq)]
pub struct SurumOlayi {
    pub surum: u64,
    pub no: String,
    pub olay: &'static str,
}

pub struct Dugum {
    kok: PathBuf,
    sube: String,
    kuyruk: Kuyruk,
    gunluk_kuyruk: Kuyruk,
    /// Süreç içi yazma kilidi: `no` tekilliği denetimi + ekleme ve sürüm sayacı tek sırada yürür.
    kilit: Mutex<()>,
    /// Merkez döngüsünü uyandırma (yeni kayıt gelince beklemeden gönderim denensin).
    uyan: (Mutex<bool>, Condvar),
}

fn kilitle<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// Ucun kuyruğa alınıp alınmayacağı: `/v1/<en az bir parça>`, geçerli biçim, kapalı uç değil.
pub fn uc_kabul(uc: &str) -> bool {
    taa_kuyruk::uc_gecerli(uc)
        && uc.len() > "/v1/".len()
        && uc.starts_with("/v1/")
        && !uc.ends_with('/')
        && !KAPALI_UCLAR
            .iter()
            .any(|k| uc == *k || uc.starts_with(&format!("{k}/")))
}

impl Dugum {
    /// Düğümü açar ve açılış onarımını yapar (kuyrukta olup sürüm sayacına yazılamamış kayıtlar tamamlanır).
    pub fn ac(kok: &Path, sube: &str) -> Result<Dugum, DugumHata> {
        if !sube_gecerli(sube) {
            return Err(DugumHata::GecersizSube(sube.to_string()));
        }
        let d = Dugum {
            kok: kok.to_path_buf(),
            sube: sube.to_string(),
            kuyruk: Kuyruk::program(kok, KUYRUK_PROGRAM)?,
            gunluk_kuyruk: Kuyruk::program(kok, GUNLUK_KUYRUK_PROGRAM)?,
            kilit: Mutex::new(()),
            uyan: (Mutex::new(false), Condvar::new()),
        };
        d.surum_isle()?;
        Ok(d)
    }

    pub fn kok(&self) -> &Path {
        &self.kok
    }

    pub fn sube(&self) -> &str {
        &self.sube
    }

    pub fn kuyruk(&self) -> &Kuyruk {
        &self.kuyruk
    }

    pub fn gunluk_kuyruk(&self) -> &Kuyruk {
        &self.gunluk_kuyruk
    }

    fn surum_yolu(&self) -> PathBuf {
        self.kok.join(KLASOR).join("surum.jsonl")
    }

    pub fn cakisma_yolu(&self) -> PathBuf {
        self.kok.join(KLASOR).join("cakisma.jsonl")
    }

    fn durum_yolu(&self) -> PathBuf {
        self.kok.join(KLASOR).join("durum.json")
    }

    /// Günlüğe satır ekler. Günlük yazılamazsa istek başarısız sayılmaz (kayıt zaten diskte); hata stderr'e yazılır.
    pub fn gunluk(&self, olay: &str, ayrinti: Value) {
        if let Err(e) = gunluk::yaz(&self.kok, olay, ayrinti) {
            eprintln!("taa-sube-dugum: günlük yazılamadı ({olay}): {e}");
        }
    }

    /// Saklanan gövdeyi açar (şifreliyse çözer). Anahtar yoksa/yanlışsa None.
    fn govde_ac(&self, govde: &Value) -> Option<Value> {
        if !sifre::sifreli_mi(govde) {
            return Some(govde.clone());
        }
        let a = sifre::anahtar_al(&self.kok).ok()?;
        sifre::ac(&a, govde)
    }

    /// Sürüm dosyasındaki olaylar: (surum, no, olay).
    fn surum_olaylari(&self) -> Result<Vec<(u64, String, String)>, DugumHata> {
        let mut v = Vec::new();
        for (i, s) in taa_dosya::satirlari_oku(&self.surum_yolu())?
            .into_iter()
            .enumerate()
        {
            let bozuk = || {
                DugumHata::Kuyruk(KuyrukHata::BozukSatir {
                    dosya: "surum.jsonl".into(),
                    satir: i + 1,
                })
            };
            let d: Value = serde_json::from_str(&s).map_err(|_| bozuk())?;
            let surum = d.get("surum").and_then(Value::as_u64).ok_or_else(bozuk)?;
            let no = d.get("no").and_then(Value::as_str).ok_or_else(bozuk)?;
            let olay = d.get("olay").and_then(Value::as_str).ok_or_else(bozuk)?;
            v.push((surum, no.to_string(), olay.to_string()));
        }
        Ok(v)
    }

    /// Kuyruk ile sürüm sayacını eşitler: sayaçta olmayan her `alindi` / `gonderildi` / `reddedildi` olayı için
    /// yeni sürüm satırı ekler. Tekrar çağrılması zararsızdır; çöküşten sonraki onarım da budur.
    pub fn surum_isle(&self) -> Result<Vec<SurumOlayi>, DugumHata> {
        let _k = kilitle(&self.kilit);
        self.surum_isle_kilitli()
    }

    fn surum_isle_kilitli(&self) -> Result<Vec<SurumOlayi>, DugumHata> {
        let olaylar = self.surum_olaylari()?;
        let mut son = olaylar.iter().map(|o| o.0).max().unwrap_or(0);
        let var: BTreeSet<(String, String)> = olaylar
            .into_iter()
            .map(|(_, no, olay)| (no, olay))
            .collect();
        let mut yeni = Vec::new();
        for (k, d) in self.kuyruk.durumlar()? {
            let mut gereken = vec!["alindi"];
            match d {
                Durum::Gonderildi => gereken.push("gonderildi"),
                Durum::Reddedildi => gereken.push("reddedildi"),
                Durum::Bekliyor => {}
            }
            for olay in gereken {
                if var.contains(&(k.no.clone(), olay.to_string())) {
                    continue;
                }
                son += 1;
                let s = json!({"surum": son, "no": k.no, "olay": olay, "zaman": zaman::simdi()})
                    .to_string();
                taa_dosya::satir_ekle(&self.surum_yolu(), &s)?;
                yeni.push(SurumOlayi {
                    surum: son,
                    no: k.no.clone(),
                    olay,
                });
            }
        }
        Ok(yeni)
    }

    /// Çakışma kutusuna yazar (aynı `no` + aynı kaybeden içerik ikinci kez yazılmaz).
    fn cakisma_yaz(
        &self,
        no: &str,
        uc: &str,
        gecerli_uc: &str,
        gecerli: &Value,
        kaybeden: &Value,
        kaynak: &str,
    ) -> Result<(), DugumHata> {
        for s in taa_dosya::satirlari_oku(&self.cakisma_yolu())? {
            if let Ok(v) = serde_json::from_str::<Value>(&s) {
                if v.get("no").and_then(Value::as_str) == Some(no)
                    && v.get("uc").and_then(Value::as_str) == Some(uc)
                    && v.get("kaybeden") == Some(kaybeden)
                {
                    return Ok(());
                }
            }
        }
        let s = json!({
            "no": no,
            "zaman": zaman::simdi(),
            "uc": uc,
            "gecerli_uc": gecerli_uc,
            "gecerli": gecerli,
            "kaybeden": kaybeden,
            "kaynak": kaynak,
            "durum": "acik",
        })
        .to_string();
        taa_dosya::satir_ekle(&self.cakisma_yolu(), &s)?;
        Ok(())
    }

    /// Bir kaydı alır: aynı `no` ikinci kez yazılmaz. `govde` nesne olmalı ve `no` alanını taşımalıdır
    /// (çağıran `sunucu::isle` bunu sağlar). `kaynak` yalnız günlük içindir (istemci adresi).
    pub fn al(&self, uc: &str, no: &str, govde: Value, kaynak: &str) -> Result<Yazma, DugumHata> {
        let sonuc = {
            let _k = kilitle(&self.kilit);
            let eski = self.kuyruk.kayitlar()?.into_iter().find(|k| k.no == no);
            match eski {
                Some(e) => {
                    let saklanan = self.govde_ac(&e.govde);
                    if e.uc == uc && saklanan.as_ref() == Some(&govde) {
                        let durumlar = self.kuyruk.durumlar()?;
                        let kesin = durumlar
                            .iter()
                            .any(|(k, d)| k.no == no && *d == Durum::Gonderildi);
                        self.surum_isle_kilitli()?;
                        let surum = self
                            .surum_olaylari()?
                            .into_iter()
                            .find(|(_, n, o)| n == no && o == "alindi")
                            .map(|o| o.0)
                            .unwrap_or(0);
                        Yazma::ZatenVar { surum, kesin }
                    } else {
                        self.cakisma_yaz(
                            no,
                            uc,
                            &e.uc,
                            saklanan.as_ref().unwrap_or(&Value::Null),
                            &govde,
                            kaynak,
                        )?;
                        Yazma::Cakisma
                    }
                }
                None => {
                    match self.kuyruk.ekle_sifreli(&self.kok, uc, no, govde)? {
                        Ekleme::Eklendi(_) | Ekleme::ZatenVar(_) => {}
                    }
                    let yeni = self.surum_isle_kilitli()?;
                    let surum = yeni
                        .iter()
                        .find(|o| o.no == no && o.olay == "alindi")
                        .map(|o| o.surum)
                        .unwrap_or(0);
                    Yazma::Yazildi { surum }
                }
            }
        };
        match &sonuc {
            Yazma::Yazildi { surum } => {
                self.gunluk(
                    "kayit.alindi",
                    json!({"no": no, "uc": uc, "surum": surum, "kaynak": kaynak}),
                );
                self.uyandir();
            }
            Yazma::ZatenVar { .. } => self.gunluk(
                "kayit.zaten_var",
                json!({"no": no, "uc": uc, "kaynak": kaynak}),
            ),
            Yazma::Cakisma => self.gunluk(
                "kayit.cakisma",
                json!({"no": no, "uc": uc, "kaynak": kaynak}),
            ),
        }
        Ok(sonuc)
    }

    /// Çakışma kutusundaki satır sayısı.
    pub fn cakisma_sayisi(&self) -> usize {
        taa_dosya::satirlari_oku(&self.cakisma_yolu())
            .map(|v| v.len())
            .unwrap_or(0)
    }

    pub fn son_merkez_zamani(&self) -> Option<String> {
        let m = std::fs::read_to_string(self.durum_yolu()).ok()?;
        let v: Value = serde_json::from_str(&m).ok()?;
        v.get("son_merkez_zamani")?.as_str().map(str::to_string)
    }

    /// Merkezden yanıt alındığını kaydeder (`dugum/durum.json`, atomik).
    pub fn merkez_goruldu(&self) -> Result<(), DugumHata> {
        let _k = kilitle(&self.kilit);
        let v = json!({"son_merkez_zamani": zaman::simdi()});
        std::fs::create_dir_all(self.kok.join(KLASOR))?;
        taa_dosya::uzerine_yaz_atomik(&self.durum_yolu(), format!("{v}\n").as_bytes())?;
        Ok(())
    }

    pub fn durum(&self) -> Result<DurumOzeti, DugumHata> {
        let mut o = DurumOzeti::default();
        for (_, d) in self.kuyruk.durumlar()? {
            match d {
                Durum::Bekliyor => o.bekleyen += 1,
                Durum::Gonderildi => o.gonderilen += 1,
                Durum::Reddedildi => o.reddedilen += 1,
            }
        }
        o.son_merkez_zamani = self.son_merkez_zamani();
        o.cakisma = self.cakisma_sayisi();
        o.surum = self
            .surum_olaylari()?
            .iter()
            .map(|x| x.0)
            .max()
            .unwrap_or(0);
        o.gunluk_bekleyen = self.gunluk_kuyruk.bekleyenler()?.len();
        Ok(o)
    }

    /// Şube önbelleğinden delta (`GET /v1/esitle?sonra=`): bu düğümün aldığı kayıtlar, son durumlarıyla.
    /// Biçim docs/WEB-ESITLE-SOZLESMESI.md §6.2 `belge` nesnesidir; `imza` bu crate'te üretilmez (kanca).
    pub fn esitle(&self, sonra: u64, limit: usize) -> Result<Value, DugumHata> {
        let limit = limit.clamp(1, ESITLE_LIMIT_EN_COK);
        let mut son: BTreeMap<String, u64> = BTreeMap::new();
        let mut en_buyuk = 0u64;
        for (surum, no, _) in self.surum_olaylari()? {
            en_buyuk = en_buyuk.max(surum);
            let e = son.entry(no).or_insert(0);
            *e = (*e).max(surum);
        }
        let mut secilen: Vec<(u64, Value)> = Vec::new();
        for (k, d) in self.kuyruk.durumlar()? {
            let Some(&surum) = son.get(&k.no) else {
                continue;
            };
            if surum <= sonra {
                continue;
            }
            let durum = match d {
                Durum::Bekliyor => "bekliyor",
                Durum::Gonderildi => "gonderildi",
                Durum::Reddedildi => "merkez_reddetti",
            };
            let anahtar = format!("{}/{}", k.uc.trim_start_matches("/v1/"), k.no);
            secilen.push((
                surum,
                json!({
                    "anahtar": anahtar,
                    "surum": surum,
                    "no": k.no,
                    "uc": k.uc,
                    "zaman": k.zaman,
                    "durum": durum,
                    "kesin": d == Durum::Gonderildi,
                    "deger": self.govde_ac(&k.govde).unwrap_or(Value::Null),
                }),
            ));
        }
        secilen.sort_by_key(|x| x.0);
        let devami = secilen.len() > limit;
        secilen.truncate(limit);
        let surum = if devami {
            secilen.last().map(|x| x.0).unwrap_or(sonra)
        } else {
            en_buyuk.max(sonra)
        };
        Ok(json!({
            "belge": {
                "sube": self.sube,
                "kaynak": "sube",
                "sonra": sonra,
                "surum": surum,
                "tam": false,
                "devami": devami,
                "sunucu_zamani": zaman::simdi(),
                "kayitlar": secilen.into_iter().map(|x| x.1).collect::<Vec<_>>(),
            },
            "imza": Value::Null,
        }))
    }

    /// Merkez döngüsünü uyandırır.
    pub fn uyandir(&self) {
        *kilitle(&self.uyan.0) = true;
        self.uyan.1.notify_all();
    }

    /// Uyandırılana ya da süre dolana kadar bekler.
    pub fn bekle(&self, sure: Duration) {
        let mut g = kilitle(&self.uyan.0);
        if !*g {
            g = self
                .uyan
                .1
                .wait_timeout(g, sure)
                .map(|x| x.0)
                .unwrap_or_else(|e| e.into_inner().0);
        }
        *g = false;
    }
}
