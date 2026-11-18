// Sahip: Yusuf. yazan: [Claude#hata-bildirimi 2026-11-18]
//! taa-hata: ortak hata raporlama.
//!
//! Akış: çökme/hata → [`temizle`] (kişisel veri, sır, yol adı çıkar) → [`Rapor`] → yerel kuyruk
//! (`<kök>/kuyruk/<program>/`, taa-kuyruk) → sunucuya ulaşınca `POST /v1/hata-raporu`.
//! Aynı hata aynı gün tek kayıt olur (parmak izi). Rapor müşteri verisi, parola, anahtar, dosya içeriği taşımaz.
//! Panik kancası ağa gitmez, yalnız yerel kuyruğa yazar; gönderim sonraki açılışta ya da zamanlayıcıyla yapılır.
//! Not: kuyruk dosyası şifrelenmez; güvence, raporun temizlenmiş olmasıdır (bkz. docs/HATA-BILDIRIMI.md).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde_json::{json, Value};
use taa_kuyruk::{zaman, Ekleme, Gonderici, Kuyruk};

/// Sunucu ucu.
pub const UC: &str = "/v1/hata-raporu";
/// Rapor biçim sürümü (sunucu sözleşmesi).
pub const BICIM: u32 = 1;
const MESAJ_EN_COK: usize = 400;
const SATIR_EN_COK: usize = 20;
const SATIR_UZUNLUK: usize = 200;

/// Hata türü.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tur {
    Cokme,
    Hata,
}

impl Tur {
    pub fn metin(self) -> &'static str {
        match self {
            Tur::Cokme => "cokme",
            Tur::Hata => "hata",
        }
    }
}

/// Temizlenmiş rapor.
#[derive(Clone, Debug, PartialEq)]
pub struct Rapor {
    pub program: String,
    pub surum: String,
    pub tur: Tur,
    /// `dosya.rs:satır` (yol temizlenmiş, yalnız dosya adı).
    pub yer: String,
    pub mesaj: String,
    pub son_satirlar: Vec<String>,
    pub parmak_izi: String,
}

fn parca_ayir(c: char) -> bool {
    c.is_whitespace() || matches!(c, ',' | ';' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>')
}

/// Anahtar sözcüklerden sonra gelen değerin sır sayılacağı alan adları.
const SIR_ANAHTARLARI: &[&str] = &[
    "parola",
    "sifre",
    "şifre",
    "password",
    "pass",
    "pin",
    "token",
    "jeton",
    "anahtar",
    "key",
    "secret",
    "bearer",
    "authorization",
    "oturum",
    "cookie",
];

fn sir_anahtari_mi(s: &str) -> bool {
    let k = s.trim_end_matches(['=', ':']).to_lowercase();
    !k.is_empty() && SIR_ANAHTARLARI.iter().any(|a| k == *a || k.ends_with(a))
}

fn dosya_adi(yol: &str) -> &str {
    yol.rsplit(['\\', '/']).next().unwrap_or(yol)
}

/// Metinden kişisel veri ve sırları çıkarır:
/// tırnak içi değerler `"<metin>"`, e-posta `<eposta>`, 4+ rakamlı sayılar `<sayi>`, 24+ karakterlik uzun
/// belirteçler `<belirtec>`, yollar yalnız dosya adına iner, sır anahtarından sonraki değer `<gizli>`.
pub fn temizle(girdi: &str) -> String {
    // 1) tırnak içi metinler (müşteri adı, ürün vb. burada olur)
    let mut kalan = String::with_capacity(girdi.len());
    let mut tirnakta: Option<char> = None;
    for c in girdi.chars() {
        match tirnakta {
            Some(t) if c == t => {
                tirnakta = None;
                kalan.push_str("<metin>");
                kalan.push(c);
            }
            Some(_) => {}
            None => {
                if c == '"' || c == '`' {
                    tirnakta = Some(c);
                }
                kalan.push(c);
            }
        }
    }
    if tirnakta.is_some() {
        kalan.push_str("<metin>");
    }
    // 2) sözcük sözcük
    let mut cikti = String::with_capacity(kalan.len());
    let mut sonraki_gizli = false;
    let mut sozcuk = String::new();
    let mut bosluklar = String::new();
    let isle = |s: &str, cikti: &mut String, sonraki_gizli: &mut bool| {
        if s.is_empty() {
            return;
        }
        if *sonraki_gizli {
            *sonraki_gizli = false;
            cikti.push_str("<gizli>");
            return;
        }
        // anahtar=değer
        if let Some(i) = s.find(['=', ':']) {
            let (a, d) = s.split_at(i);
            if sir_anahtari_mi(a) && d.len() > 1 {
                cikti.push_str(a);
                cikti.push_str(&d[..1]);
                cikti.push_str("<gizli>");
                return;
            }
        }
        if sir_anahtari_mi(s) {
            *sonraki_gizli = true;
            cikti.push_str(s);
            return;
        }
        cikti.push_str(&sozcuk_temizle(s));
    };
    for c in kalan.chars() {
        if parca_ayir(c) {
            isle(&sozcuk, &mut cikti, &mut sonraki_gizli);
            sozcuk.clear();
            bosluklar.clear();
            bosluklar.push(c);
            cikti.push(c);
        } else {
            sozcuk.push(c);
        }
    }
    isle(&sozcuk, &mut cikti, &mut sonraki_gizli);
    cikti
}

fn sozcuk_temizle(s: &str) -> String {
    let t = s.trim_matches(|c: char| matches!(c, '.' | '"' | '\'' | '`'));
    if t.contains('@') && t.contains('.') {
        return s.replace(t, "<eposta>");
    }
    let yol_gibi = (t.contains('\\') || t.starts_with('/') || t.contains(":\\")) && t.len() > 3;
    if yol_gibi {
        return s.replace(t, dosya_adi(t));
    }
    let rakam = t.chars().filter(|c| c.is_ascii_digit()).count();
    if rakam >= 4
        && t.chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '+' | '-' | '.' | '/' | ' '))
    {
        return s.replace(t, "<sayi>");
    }
    if t.chars().count() >= 24
        && t.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '/' | '=' | '-' | '_'))
    {
        return s.replace(t, "<belirtec>");
    }
    s.to_string()
}

fn kisalt(s: &str, en_cok: usize) -> String {
    if s.chars().count() <= en_cok {
        s.to_string()
    } else {
        let k: String = s.chars().take(en_cok).collect();
        format!("{k}…")
    }
}

fn parmak_izi_hesapla(program: &str, tur: Tur, yer: &str, mesaj: &str) -> String {
    // sayıları ve <...> işaretlerini düşür: aynı hata farklı değerlerle aynı izi taşısın
    let norm: String = mesaj.chars().filter(|c| !c.is_ascii_digit()).collect();
    let mut h = DefaultHasher::new();
    (program, tur.metin(), yer, norm).hash(&mut h);
    format!("{:016x}", h.finish())
}

impl Rapor {
    /// Ham veriden temizlenmiş rapor kurar. `yer`: panikte `dosya:satır`, yoksa boş.
    pub fn yeni(
        program: &str,
        surum: &str,
        tur: Tur,
        yer: &str,
        mesaj: &str,
        son_satirlar: &[String],
    ) -> Rapor {
        let yer = kisalt(&temizle(yer), 120);
        let mesaj = kisalt(&temizle(mesaj), MESAJ_EN_COK);
        let basla = son_satirlar.len().saturating_sub(SATIR_EN_COK);
        let son_satirlar: Vec<String> = son_satirlar[basla..]
            .iter()
            .map(|s| kisalt(&temizle(s), SATIR_UZUNLUK))
            .collect();
        let parmak_izi = parmak_izi_hesapla(program, tur, &yer, &mesaj);
        Rapor {
            program: program.to_string(),
            surum: surum.to_string(),
            tur,
            yer,
            mesaj,
            son_satirlar,
            parmak_izi,
        }
    }

    /// Sunucuya gidecek gövde.
    pub fn govde(&self) -> Value {
        json!({
            "bicim": BICIM,
            "program": self.program,
            "surum": self.surum,
            "tur": self.tur.metin(),
            "yer": self.yer,
            "mesaj": self.mesaj,
            "son_satirlar": self.son_satirlar,
            "parmak_izi": self.parmak_izi,
            "isletim": std::env::consts::OS,
            "zaman": zaman::simdi(),
        })
    }
}

fn gun_eki() -> String {
    // "YYYY-AA-GG..." -> "YYYYAAGG"
    zaman::simdi()
        .chars()
        .take(10)
        .filter(|c| c.is_ascii_digit())
        .collect()
}

/// Raporu yerel kuyruğa yazar. Aynı hata aynı gün ikinci kez eklenmez (`Ekleme::ZatenVar`).
pub fn kuyruga_yaz(kok: &Path, r: &Rapor) -> Result<Ekleme, taa_kuyruk::KuyrukHata> {
    let k = Kuyruk::program(kok, &format!("hata-{}", r.program))?;
    let no = format!("H-{}-{}", r.parmak_izi, gun_eki());
    k.ekle(UC, &no, r.govde())
}

/// Bekleyen hata raporlarını (tüm programlar) sunucuya yollar. Ulaşılamazsa sessizce kalır; gönderilen sayısını döner.
pub fn bekleyenleri_gonder(kok: &Path, g: &dyn Gonderici) -> usize {
    let mut toplam = 0;
    if let Ok(liste) = Kuyruk::hepsi(kok) {
        for (ad, k) in liste {
            if ad.starts_with("hata-") {
                if let Ok(o) = k.gonder(g, 20) {
                    toplam += o.gonderilen;
                }
            }
        }
    }
    toplam
}

struct Ayar {
    program: String,
    surum: String,
    kok: PathBuf,
}
static AYAR: OnceLock<Ayar> = OnceLock::new();

/// Program başında bir kez çağrılır: panik kancasını kurar. Kanca yalnız yerel kuyruğa yazar, ağa gitmez
/// ve kendisi asla paniklemez. Mevcut kanca (varsa) sonra çalışır.
pub fn kur(program: &str, surum: &str, kok: impl Into<PathBuf>) {
    let _ = AYAR.set(Ayar {
        program: program.to_string(),
        surum: surum.to_string(),
        kok: kok.into(),
    });
    let onceki = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |bilgi| {
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            if let Some(a) = AYAR.get() {
                let yer = bilgi
                    .location()
                    .map(|l| format!("{}:{}", l.file(), l.line()))
                    .unwrap_or_default();
                let mesaj = if let Some(s) = bilgi.payload().downcast_ref::<&str>() {
                    (*s).to_string()
                } else if let Some(s) = bilgi.payload().downcast_ref::<String>() {
                    s.clone()
                } else {
                    "panik".to_string()
                };
                let r = Rapor::yeni(&a.program, &a.surum, Tur::Cokme, &yer, &mesaj, &[]);
                let _ = kuyruga_yaz(&a.kok, &r);
            }
        }));
        onceki(bilgi);
    }));
}

/// Veri kökü: `TAA_VERI_KOK`, yoksa `%APPDATA%\TAA\veri` (Windows) / `~/.local/share/taa/veri`.
pub fn varsayilan_kok() -> PathBuf {
    if let Some(k) = std::env::var_os("TAA_VERI_KOK") {
        return PathBuf::from(k);
    }
    if let Some(a) = std::env::var_os("APPDATA") {
        return PathBuf::from(a).join("TAA").join("veri");
    }
    if let Some(h) = std::env::var_os("HOME") {
        return PathBuf::from(h).join(".local/share/taa/veri");
    }
    PathBuf::from(".taa-veri")
}

/// `kur` + [`varsayilan_kok`]: programın `main` başında tek satır.
pub fn kur_varsayilan(program: &str, surum: &str) {
    kur(program, surum, varsayilan_kok());
}

/// Çökmeden yakalanan hatayı bildirir (ör. beklenmeyen hata). `kur` çağrılmamışsa hiçbir şey yapmaz.
pub fn bildir(tur: Tur, yer: &str, mesaj: &str, son_satirlar: &[String]) {
    if let Some(a) = AYAR.get() {
        let r = Rapor::yeni(&a.program, &a.surum, tur, yer, mesaj, son_satirlar);
        let _ = kuyruga_yaz(&a.kok, &r);
    }
}

#[cfg(test)]
mod testler;
