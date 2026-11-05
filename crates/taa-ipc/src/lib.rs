// Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05]
//! taa-ipc: programlar arası iletişim (T1: dosya tabanlı `GelenKutusu`).
//!
//! Zarf modeli, `Tasiyici` soyutlaması ve dosya tabanlı uygulama. Yetenekler programlara aittir;
//! bu crate yalnız taşır. Hiçbir dosya silinmez: işlenen zarflar `islenen/` altına taşınır.
//! Düzen:
//! ```text
//! <kok>/kayit/<program>.json        kayıt (atomik)
//! <kok>/<program>/gelen/<id>.json   istek zarfı
//! <kok>/<program>/yanit/<id>.json   yanıt zarfı (isteği atan programın klasöründe)
//! <kok>/<program>/islenen/<id>.json işlenmiş zarflar
//! <kok>/olaylar/<YYYYMMDD>.jsonl    eklemeli olay günlüğü
//! <kok>/gunluk.jsonl                bozuk dosya vb. günlüğü
//! ```

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

/// Bir zarfın (serileştirilmiş) en büyük boyutu: 1 MiB.
pub const ZARF_SINIRI: usize = 1024 * 1024;
/// Zarf sürümü.
pub const ZARF_SURUMU: u32 = 1;
/// Olayların `hedef` alanı.
pub const OLAY_HEDEFI: &str = "herkes";

const AD_SINIRI: usize = 64;
const PROGRAM_ADI_SINIRI: usize = 32;
/// Program adı olamayacak adlar (kökteki sabit klasör/dosya adları ve program alt klasörleri).
const AYRILMIS_ADLAR: [&str; 6] = ["kayit", "olaylar", "gunluk", "islenen", "gelen", "yanit"];
/// Bir programın kaydı, başka bir pid'den bu kadar saniyeden yeniyse üzerine yazılmaz.
const KAYIT_TAZELIGI_SN: i64 = 600;
/// `olaylar()` yalnız bu kadar günlük dosyaya bakar (bugün dahil).
const OLAY_GUN_SAYISI: i64 = 7;
/// Bir olay günlüğü dosyasından okunacak en çok satır.
const OLAY_SATIR_SINIRI: usize = 10_000;

/// IPC hataları.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IpcHata {
    /// Hedef programda bu yetenek kayıtlı değil (kabuk düğmeyi gizler).
    YetenekYok(String),
    /// Zarf sınırı aşıyor; dosya yazılmadı.
    ZarfBuyuk { boyut: usize, sinir: usize },
    /// Geçersiz ad, alan ya da biçim.
    Gecersiz(String),
    /// Dosya sistemi hatası.
    Io(String),
    /// Hedef dosya zaten var (aynı kimlikle ikinci istek/yanıt); hiçbir şey yazılmadı.
    IdCakismasi(String),
    /// Program kaydı başka bir süreçte ve taze (10 dakikadan yeni); üzerine yazılmadı.
    KayitDolu(String),
    /// Yeteneği birden çok program sunuyor; hiçbiri seçilmedi (kabuk kullanıcıya sorar).
    Belirsiz {
        yetenek: String,
        programlar: Vec<String>,
    },
}

impl fmt::Display for IpcHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IpcHata::YetenekYok(y) => {
                write!(f, "yetenek bulunamadı: {y}; hedef program kayıtlı değil ya da bu yeteneği sunmuyor")
            }
            IpcHata::ZarfBuyuk { boyut, sinir } => {
                write!(
                    f,
                    "zarf çok büyük: {boyut} bayt (sınır {sinir}); daha küçük yük gönderin"
                )
            }
            IpcHata::Gecersiz(n) => write!(f, "geçersiz: {n}"),
            IpcHata::Io(n) => write!(f, "dosya hatası: {n}"),
            IpcHata::IdCakismasi(id) => {
                write!(
                    f,
                    "kimlik çakışması: {id} zaten var; yeni bir kimlikle yeniden deneyin"
                )
            }
            IpcHata::KayitDolu(p) => {
                write!(f, "program kaydı başka bir süreçte kullanımda: {p}; 10 dakika sonra yeniden deneyin")
            }
            IpcHata::Belirsiz {
                yetenek,
                programlar,
            } => {
                write!(
                    f,
                    "yetenek belirsiz: {yetenek}; sunan programlar: {}; kullanıcıya sorun",
                    programlar.join(", ")
                )
            }
        }
    }
}

impl std::error::Error for IpcHata {}

impl From<io::Error> for IpcHata {
    fn from(e: io::Error) -> Self {
        IpcHata::Io(e.to_string())
    }
}

/// Zarf türü.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ZarfTuru {
    Istek,
    Yanit,
    Olay,
}

/// İletişim zarfı.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zarf {
    pub v: u32,
    pub id: String,
    pub kaynak: String,
    pub hedef: String,
    pub tur: ZarfTuru,
    pub yetenek: String,
    #[serde(default)]
    pub yuk: Value,
    pub zaman: String,
    #[serde(default)]
    pub yanit_id: Option<String>,
}

impl Zarf {
    /// Yeni istek zarfı (kimlik ve zaman otomatik).
    pub fn istek(kaynak: &str, hedef: &str, yetenek: &str, yuk: Value) -> Zarf {
        Zarf {
            v: ZARF_SURUMU,
            id: yeni_id(),
            kaynak: kaynak.to_string(),
            hedef: hedef.to_string(),
            tur: ZarfTuru::Istek,
            yetenek: yetenek.to_string(),
            yuk,
            zaman: simdi_rfc3339(),
            yanit_id: None,
        }
    }

    /// Alanları denetler (sürüm, ad karakterleri).
    pub fn dogrula(&self) -> Result<(), IpcHata> {
        if self.v != ZARF_SURUMU {
            return Err(IpcHata::Gecersiz(format!(
                "zarf sürümü desteklenmiyor: {}",
                self.v
            )));
        }
        kimlik_dogrula("id", &self.id)?;
        program_adi_dogrula("kaynak", &self.kaynak)?;
        program_adi_dogrula("hedef", &self.hedef)?;
        yetenek_dogrula("yetenek", &self.yetenek)?;
        if let Some(y) = &self.yanit_id {
            kimlik_dogrula("yanit_id", y)?;
        }
        Ok(())
    }
}

/// Kimlik denetimi: yalnız `[a-z0-9-]`, ilk karakter harf/rakam, en çok 64 karakter
/// (zarf kimliği `<pid>-<sayac>-<nanos>` rakamla başlar). Nokta ve `@` yoktur; yol kaçışı olmaz.
pub fn kimlik_gecerli(kimlik: &str) -> bool {
    let mut it = kimlik.chars();
    match it.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    kimlik.len() <= AD_SINIRI
        && kimlik
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Program adı denetimi: yalnız `[a-z0-9-]` (nokta ve `@` yok), harfle başlar, en çok 32 karakter.
/// Ayrılmış adlar için bkz. `program_dogrula`.
pub fn program_adi_gecerli(ad: &str) -> bool {
    let mut it = ad.chars();
    match it.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }
    ad.len() <= PROGRAM_ADI_SINIRI
        && ad
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// Yetenek adı denetimi: `ad.fiil@N`. `ad` ve `fiil` `[a-z0-9-]`, harfle başlar; `N` en çok 9 rakam;
/// toplam en çok 64 karakter.
pub fn yetenek_adi_gecerli(yetenek: &str) -> bool {
    fn parca(p: &str) -> bool {
        p.chars().next().is_some_and(|c| c.is_ascii_lowercase())
            && p.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    }
    if yetenek.len() > AD_SINIRI {
        return false;
    }
    let Some((ad_fiil, surum)) = yetenek.split_once('@') else {
        return false;
    };
    let Some((ad, fiil)) = ad_fiil.split_once('.') else {
        return false;
    };
    parca(ad)
        && parca(fiil)
        && !surum.is_empty()
        && surum.len() <= 9
        && surum.chars().all(|c| c.is_ascii_digit())
}

fn kimlik_dogrula(alan: &str, ad: &str) -> Result<(), IpcHata> {
    if kimlik_gecerli(ad) {
        Ok(())
    } else {
        Err(IpcHata::Gecersiz(format!(
            "{alan} yalnız [a-z0-9-] içerebilir, harf/rakamla başlamalı ve en çok {AD_SINIRI} karakter olmalı"
        )))
    }
}

fn program_adi_dogrula(alan: &str, ad: &str) -> Result<(), IpcHata> {
    if program_adi_gecerli(ad) {
        Ok(())
    } else {
        Err(IpcHata::Gecersiz(format!(
            "{alan} yalnız [a-z0-9-] içerebilir, harfle başlamalı ve en çok {PROGRAM_ADI_SINIRI} karakter olmalı"
        )))
    }
}

fn yetenek_dogrula(alan: &str, ad: &str) -> Result<(), IpcHata> {
    if yetenek_adi_gecerli(ad) {
        Ok(())
    } else {
        Err(IpcHata::Gecersiz(format!(
            "{alan} 'ad.fiil@N' biçiminde olmalı (ad/fiil: [a-z0-9-], harfle başlar; N: rakam)"
        )))
    }
}

fn program_dogrula(program: &str) -> Result<(), IpcHata> {
    program_adi_dogrula("program", program)?;
    if AYRILMIS_ADLAR.contains(&program) {
        return Err(IpcHata::Gecersiz(format!(
            "program adı ayrılmış: {program}"
        )));
    }
    Ok(())
}

fn sayac() -> u64 {
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

fn nanos() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
}

/// Benzersiz zarf kimliği: `<pid>-<sayac>-<nanos>`.
pub fn yeni_id() -> String {
    format!("{}-{}-{}", std::process::id(), sayac(), nanos())
}

/// Gün sayısından (1970-01-01'den) takvim tarihi (Howard Hinnant algoritması).
fn sivil_tarih(gun: i64) -> (i64, u32, u32) {
    let z = gun + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Unix saniyesinden (UTC) `(RFC3339, YYYYMMDD)`.
fn zaman_biciminde(saniye: i64) -> (String, String) {
    let gun = saniye.div_euclid(86_400);
    let kalan = saniye.rem_euclid(86_400);
    let (y, a, g) = sivil_tarih(gun);
    let (sa, dk, sn) = (kalan / 3600, kalan % 3600 / 60, kalan % 60);
    (
        format!("{y:04}-{a:02}-{g:02}T{sa:02}:{dk:02}:{sn:02}Z"),
        format!("{y:04}{a:02}{g:02}"),
    )
}

/// Takvim tarihinden gün sayısı (1970-01-01'den); `sivil_tarih`in tersi.
fn gun_sayisi(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `YYYY-MM-DDTHH:MM:SSZ` biçimini Unix saniyesine çevirir (başka biçim `None`).
fn rfc3339_saniye(z: &str) -> Option<i64> {
    let b = z.as_bytes();
    if b.len() != 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return None;
    }
    let sayi = |a: usize, c: usize| -> Option<i64> {
        let t = &z[a..c];
        t.bytes()
            .all(|x| x.is_ascii_digit())
            .then(|| t.parse().ok())?
    };
    let (y, a, g) = (sayi(0, 4)?, sayi(5, 7)?, sayi(8, 10)?);
    let (sa, dk, sn) = (sayi(11, 13)?, sayi(14, 16)?, sayi(17, 19)?);
    if !(1..=12).contains(&a) || !(1..=31).contains(&g) || sa > 23 || dk > 59 || sn > 60 {
        return None;
    }
    Some(gun_sayisi(y, a, g) * 86_400 + sa * 3600 + dk * 60 + sn)
}

fn simdi_saniye() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Şimdiki zaman, RFC3339 (UTC).
pub fn simdi_rfc3339() -> String {
    zaman_biciminde(simdi_saniye()).0
}

/// Taşıyıcı soyutlaması. T1: `GelenKutusu`; adlandırılmış boru T2'de aynı arayüzle.
///
/// Tasarım §2.1'den sapma: dönüş tipleri açıkça verildi; `olay_yayinla` olayın kaynağı için
/// `kaynak` alır; yanıtı almak için `yanit_al` eklendi.
pub trait Tasiyici {
    /// Programı ve sunduğu yetenekleri kaydeder.
    fn kayit_ol(&self, program: &str, yetenekler: &[String]) -> Result<(), IpcHata>;
    /// Yeteneği sunan programı bulur. Hiçbiri sunmuyorsa `Ok(None)`; birden çok program
    /// sunuyorsa hiçbirini seçmez, `Belirsiz` döner (kabuk kullanıcıya sorar).
    fn yetenek_bul(&self, yetenek: &str) -> Result<Option<String>, IpcHata>;
    /// İstek gönderir; zarf kimliğini döner. Yetenek yoksa `YetenekYok`.
    fn gonder(&self, zarf: &Zarf) -> Result<String, IpcHata>;
    /// Programa gelen bekleyen istekleri okur (bozuklar atlanır).
    fn al(&self, program: &str) -> Vec<Zarf>;
    /// İsteği yanıtlar; yanıt zarfı kimliğini döner. İstek `islenen/` altına taşınır.
    fn yanitla(&self, istek: &Zarf, yuk: Value) -> Result<String, IpcHata>;
    /// `program`ın `istek_id` isteğine gelen yanıtı alır (varsa `islenen/` altına taşır).
    fn yanit_al(&self, program: &str, istek_id: &str) -> Option<Zarf>;
    /// Olay yayınlar; olay kimliğini döner.
    fn olay_yayinla(&self, kaynak: &str, ad: &str, yuk: Value) -> Result<String, IpcHata>;
    /// `son` kimliğinden sonraki olayları verir. Yalnız son 7 günün dosyalarına bakılır;
    /// `son` boşsa bu pencerenin hepsi, `son` bulunamazsa yalnız bugünün olayları gelir.
    /// `program`ın kendi yayınladıkları dahil edilmez.
    fn olaylar(&self, program: &str, son: &str) -> Vec<Zarf>;
}

/// Dosya tabanlı taşıyıcı.
#[derive(Debug, Clone)]
pub struct GelenKutusu {
    kok: PathBuf,
}

impl GelenKutusu {
    /// Verilen kökle kurar (testler geçici kök verir).
    pub fn yeni(kok: impl Into<PathBuf>) -> GelenKutusu {
        GelenKutusu { kok: kok.into() }
    }

    /// Kök sırası: `TAA_IPC_KOK` → `%APPDATA%\TAA\ipc` → `~/.local/share/TAA/ipc`.
    pub fn ortamdan() -> Result<GelenKutusu, IpcHata> {
        Ok(GelenKutusu::yeni(varsayilan_kok()?))
    }

    pub fn kok(&self) -> &Path {
        &self.kok
    }

    fn kayit_yolu(&self, program: &str) -> PathBuf {
        self.kok.join("kayit").join(format!("{program}.json"))
    }

    fn klasor(&self, program: &str, alt: &str) -> PathBuf {
        self.kok.join(program).join(alt)
    }

    fn gunluk(&self, olay: &str, dosya: &Path, neden: &str) {
        let satir = serde_json::json!({
            "zaman": simdi_rfc3339(),
            "olay": olay,
            "dosya": dosya.display().to_string(),
            "neden": neden,
        });
        // Günlük yazılamıyorsa okuma yine de durmaz.
        let _ = taa_dosya::satir_ekle(&self.kok.join("gunluk.jsonl"), &satir.to_string());
    }

    /// Düzenli dosyayı sınır dahilinde okuyup zarfa çevirir; sorunu metin olarak döner (günlüğe yazmaz).
    fn zarf_oku_ham(yol: &Path) -> Result<Zarf, String> {
        let meta = fs::symlink_metadata(yol).map_err(|e| e.to_string())?;
        if !meta.is_file() {
            return Err("düzenli dosya değil".into());
        }
        if meta.len() as usize > ZARF_SINIRI {
            return Err(format!("zarf sınırı aşıldı: {} bayt", meta.len()));
        }
        let bytes = fs::read(yol).map_err(|e| e.to_string())?;
        let z: Zarf = serde_json::from_slice(&bytes).map_err(|e| format!("JSON bozuk: {e}"))?;
        z.dogrula().map_err(|e| e.to_string())?;
        Ok(z)
    }

    /// Bozuk zarfı `<program>/islenen/bozuk/` altına taşır ve bir kez günlüğe yazar
    /// (böylece yeniden teslim edilmez, günlük şişmez). Dosya silinmez.
    fn bozugu_ayir(&self, program: &str, yol: &Path, neden: &str) {
        self.gunluk("bozuk_zarf", yol, neden);
        if let Err(e) = self.tasi(program, yol, "islenen/bozuk") {
            self.gunluk("tasima_hatasi", yol, &e.to_string());
        }
    }

    /// Eşleşmeyen (sahte ya da yanlış yerleştirilmiş) zarfı günlüğe yazıp `islenen/` altına alır.
    fn uyusmayani_ayir(&self, program: &str, yol: &Path, neden: &str) {
        self.gunluk("yanlis_zarf", yol, neden);
        if let Err(e) = self.tasi(program, yol, "islenen") {
            self.gunluk("tasima_hatasi", yol, &e.to_string());
        }
    }

    /// İsteğin hedef programını bulur: isteği atan `program`ın `istek_id` isteği,
    /// hedefin `gelen/` ya da `islenen/` klasöründe durur. Tek ve tutarlı eşleşme yoksa `None`.
    fn istek_hedefi(&self, program: &str, istek_id: &str) -> Option<String> {
        let mut bulunan: Vec<String> = Vec::new();
        for e in fs::read_dir(&self.kok).ok()?.filter_map(|e| e.ok()) {
            if !e.file_type().is_ok_and(|t| t.is_dir()) {
                continue;
            }
            let ad = e.file_name().to_string_lossy().to_string();
            if program_dogrula(&ad).is_err() {
                continue;
            }
            for alt in ["gelen", "islenen"] {
                let yol = e.path().join(alt).join(format!("{istek_id}.json"));
                if let Ok(z) = Self::zarf_oku_ham(&yol) {
                    if z.tur == ZarfTuru::Istek
                        && z.id == istek_id
                        && z.kaynak == program
                        && z.hedef == ad
                    {
                        bulunan.push(ad.clone());
                    }
                }
            }
        }
        bulunan.sort();
        bulunan.dedup();
        (bulunan.len() == 1).then(|| bulunan.remove(0))
    }

    /// Yeni dosya yazar; hedef varsa ÜZERİNE YAZMAZ ve `IdCakismasi` döner.
    /// Geçici dosyaya yazılır, sonra sert bağlantıyla yerine konur (bağlantı hedef varsa hata verir).
    fn yeni_dosya_yaz(yol: &Path, id: &str, bytes: &[u8]) -> Result<(), IpcHata> {
        use std::io::Write;
        let dizin = yol.parent().map(Path::to_path_buf).unwrap_or_default();
        if !dizin.as_os_str().is_empty() {
            fs::create_dir_all(&dizin)?;
        }
        if yol.exists() {
            return Err(IpcHata::IdCakismasi(id.to_string()));
        }
        let gecici = dizin.join(format!(".taa-ipc-{}-{}.tmp", std::process::id(), sayac()));
        {
            let mut f = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&gecici)?;
            f.write_all(bytes)?;
            f.sync_all()?;
        }
        let sonuc = match fs::hard_link(&gecici, yol) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                Err(IpcHata::IdCakismasi(id.to_string()))
            }
            Err(_) if yol.exists() => Err(IpcHata::IdCakismasi(id.to_string())),
            // Dosya sistemi bağlantıyı desteklemiyorsa yeniden adlandır (hedef boş doğrulandı).
            Err(_) => fs::rename(&gecici, yol).map_err(IpcHata::from),
        };
        // Silme yok: bağlantı kurulduysa (veya çakışma olduysa) geçici dosya
        // `.taa-ipc-artik/` altına taşınır; rename ile kullanıldıysa zaten yoktur.
        if gecici.exists() {
            let artik = dizin.join(".taa-ipc-artik");
            if fs::create_dir_all(&artik).is_ok() {
                if let Some(ad) = gecici.file_name() {
                    let _ = fs::rename(&gecici, artik.join(ad));
                }
            }
        }
        sonuc
    }

    fn json_dosyalari(&self, klasor: &Path) -> Vec<PathBuf> {
        let mut liste: Vec<PathBuf> = match fs::read_dir(klasor) {
            Ok(r) => r
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| {
                    p.extension().is_some_and(|x| x == "json")
                        && p.file_name()
                            .is_some_and(|n| !n.to_string_lossy().starts_with('.'))
                })
                .collect(),
            Err(_) => Vec::new(),
        };
        liste.sort();
        liste
    }

    /// Dosyayı silmeden `<program>/<alt>/` altına taşır (hedef doluysa üzerine yazmaz).
    fn tasi(&self, program: &str, yol: &Path, alt: &str) -> io::Result<()> {
        let Some(ad) = yol.file_name() else {
            return Ok(());
        };
        let dizin = self.klasor(program, alt);
        fs::create_dir_all(&dizin)?;
        let mut hedef = dizin.join(ad);
        let mut n = 1;
        while hedef.exists() {
            hedef = dizin.join(format!("{}.{n}", ad.to_string_lossy()));
            n += 1;
        }
        match fs::rename(yol, &hedef) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            s => s,
        }
    }

    /// Dosyayı silmeden `islenen/` altına taşır.
    fn isleneni_tasi(&self, program: &str, yol: &Path) -> io::Result<()> {
        self.tasi(program, yol, "islenen")
    }

    fn serilestir(zarf: &Zarf) -> Result<Vec<u8>, IpcHata> {
        let bytes = serde_json::to_vec(zarf).map_err(|e| IpcHata::Gecersiz(e.to_string()))?;
        if bytes.len() > ZARF_SINIRI {
            return Err(IpcHata::ZarfBuyuk {
                boyut: bytes.len(),
                sinir: ZARF_SINIRI,
            });
        }
        Ok(bytes)
    }

    /// Kaydın `(pid, zaman saniyesi)` değerleri. Kayıt yok/bozuksa ya da zaman okunamıyorsa `None`
    /// (bayat sayılır, üzerine yazılabilir).
    fn kayit_sahibi(&self, yol: &Path) -> Option<(Option<u32>, i64)> {
        let v: Value = serde_json::from_slice(&fs::read(yol).ok()?).ok()?;
        let zaman = rfc3339_saniye(v.get("zaman")?.as_str()?)?;
        let pid = v
            .get("pid")
            .and_then(Value::as_u64)
            .and_then(|p| u32::try_from(p).ok());
        Some((pid, zaman))
    }

    fn kayit_oku(&self, yol: &Path) -> Option<Vec<String>> {
        let bytes = fs::read(yol).ok()?;
        let v: Value = serde_json::from_slice(&bytes).ok()?;
        Some(
            v.get("yetenekler")?
                .as_array()?
                .iter()
                .filter_map(|y| y.as_str().map(str::to_string))
                .collect(),
        )
    }
}

fn varsayilan_kok() -> Result<PathBuf, IpcHata> {
    if let Some(k) = std::env::var_os("TAA_IPC_KOK").filter(|k| !k.is_empty()) {
        return Ok(PathBuf::from(k));
    }
    if let Some(a) = std::env::var_os("APPDATA").filter(|k| !k.is_empty()) {
        return Ok(PathBuf::from(a).join("TAA").join("ipc"));
    }
    if let Some(h) = std::env::var_os("HOME").filter(|k| !k.is_empty()) {
        return Ok(PathBuf::from(h).join(".local/share/TAA/ipc"));
    }
    Err(IpcHata::Gecersiz(
        "IPC kökü belirlenemedi; TAA_IPC_KOK ortam değişkenini ayarlayın".into(),
    ))
}

impl Tasiyici for GelenKutusu {
    fn kayit_ol(&self, program: &str, yetenekler: &[String]) -> Result<(), IpcHata> {
        program_dogrula(program)?;
        for y in yetenekler {
            yetenek_dogrula("yetenek", y)?;
        }
        // Başka bir süreç aynı programı yakın zamanda kaydettiyse kaydı ezmeyiz (bayat kayıt ezilir).
        if let Some((pid, zaman)) = self.kayit_sahibi(&self.kayit_yolu(program)) {
            let yas = simdi_saniye() - zaman;
            if pid != Some(std::process::id()) && (-60..KAYIT_TAZELIGI_SN).contains(&yas) {
                return Err(IpcHata::KayitDolu(program.to_string()));
            }
        }
        for alt in ["gelen", "yanit", "islenen"] {
            fs::create_dir_all(self.klasor(program, alt))?;
        }
        let kayit = serde_json::json!({
            "program": program,
            "yetenekler": yetenekler,
            "pid": std::process::id(),
            "zaman": simdi_rfc3339(),
        });
        let bytes =
            serde_json::to_vec_pretty(&kayit).map_err(|e| IpcHata::Gecersiz(e.to_string()))?;
        taa_dosya::uzerine_yaz_atomik(&self.kayit_yolu(program), &bytes)?;
        Ok(())
    }

    fn yetenek_bul(&self, yetenek: &str) -> Result<Option<String>, IpcHata> {
        if !yetenek_adi_gecerli(yetenek) {
            return Ok(None);
        }
        let mut sunanlar: Vec<String> = Vec::new();
        for yol in self.json_dosyalari(&self.kok.join("kayit")) {
            let Some(program) = yol.file_stem().map(|s| s.to_string_lossy().to_string()) else {
                continue;
            };
            if program_dogrula(&program).is_err() {
                continue;
            }
            match self.kayit_oku(&yol) {
                Some(y) if y.iter().any(|x| x == yetenek) => sunanlar.push(program),
                Some(_) => {}
                None => self.gunluk("bozuk_kayit", &yol, "kayıt okunamadı"),
            }
        }
        match sunanlar.len() {
            0 => Ok(None),
            1 => Ok(sunanlar.pop()),
            _ => Err(IpcHata::Belirsiz {
                yetenek: yetenek.to_string(),
                programlar: sunanlar,
            }),
        }
    }

    fn gonder(&self, zarf: &Zarf) -> Result<String, IpcHata> {
        zarf.dogrula()?;
        if zarf.tur != ZarfTuru::Istek {
            return Err(IpcHata::Gecersiz("gonder yalnız istek zarfı alır".into()));
        }
        program_dogrula(&zarf.hedef)?;
        program_dogrula(&zarf.kaynak)?;
        let sunulan = self
            .kayit_oku(&self.kayit_yolu(&zarf.hedef))
            .is_some_and(|y| y.contains(&zarf.yetenek));
        if !sunulan {
            return Err(IpcHata::YetenekYok(zarf.yetenek.clone()));
        }
        let bytes = Self::serilestir(zarf)?;
        let yol = self
            .klasor(&zarf.hedef, "gelen")
            .join(format!("{}.json", zarf.id));
        // Var olan istek dosyasının üzerine yazılmaz, "(1)" eki de alınmaz.
        Self::yeni_dosya_yaz(&yol, &zarf.id, &bytes)?;
        Ok(zarf.id.clone())
    }

    fn al(&self, program: &str) -> Vec<Zarf> {
        if program_dogrula(program).is_err() {
            return Vec::new();
        }
        let mut sonuc = Vec::new();
        for yol in self.json_dosyalari(&self.klasor(program, "gelen")) {
            let z = match Self::zarf_oku_ham(&yol) {
                Ok(z) => z,
                Err(neden) => {
                    self.bozugu_ayir(program, &yol, &neden);
                    continue;
                }
            };
            // Dosya adı `<id>.json` ve zarf.id aynı olmalı; yoksa sahte/yanlış yerleştirilmiş zarftır.
            let ad_uyumlu = yol.file_stem().is_some_and(|s| s == z.id.as_str());
            if z.tur != ZarfTuru::Istek || z.hedef != program || !ad_uyumlu {
                self.uyusmayani_ayir(
                    program,
                    &yol,
                    "tür, hedef ya da dosya adı/zarf kimliği uyuşmuyor",
                );
                continue;
            }
            sonuc.push(z);
        }
        sonuc
    }

    fn yanitla(&self, istek: &Zarf, yuk: Value) -> Result<String, IpcHata> {
        istek.dogrula()?;
        program_dogrula(&istek.kaynak)?;
        program_dogrula(&istek.hedef)?;
        let yanit = Zarf {
            v: ZARF_SURUMU,
            id: yeni_id(),
            kaynak: istek.hedef.clone(),
            hedef: istek.kaynak.clone(),
            tur: ZarfTuru::Yanit,
            yetenek: istek.yetenek.clone(),
            yuk,
            zaman: simdi_rfc3339(),
            yanit_id: Some(istek.id.clone()),
        };
        let bytes = Self::serilestir(&yanit)?;
        // Yanıt, isteği atan programın klasörüne yazılır; dosya adı isteğin kimliğidir.
        let yol = self
            .klasor(&istek.kaynak, "yanit")
            .join(format!("{}.json", istek.id));
        // Var olan yanıt (ör. sahte yanıt) ezilmez: IdCakismasi döner, istek gelen/ altında kalır.
        Self::yeni_dosya_yaz(&yol, &istek.id, &bytes)?;
        self.isleneni_tasi(
            &istek.hedef,
            &self
                .klasor(&istek.hedef, "gelen")
                .join(format!("{}.json", istek.id)),
        )?;
        Ok(yanit.id)
    }

    fn yanit_al(&self, program: &str, istek_id: &str) -> Option<Zarf> {
        if program_dogrula(program).is_err() || !kimlik_gecerli(istek_id) {
            return None;
        }
        let yol = self
            .klasor(program, "yanit")
            .join(format!("{istek_id}.json"));
        if !yol.exists() {
            return None;
        }
        let z = match Self::zarf_oku_ham(&yol) {
            Ok(z) => z,
            Err(neden) => {
                self.bozugu_ayir(program, &yol, &neden);
                return None;
            }
        };
        // Yanıt, isteğin hedefinden gelmeli ve bu isteğe ait olmalı; yoksa kabul edilmez.
        let beklenen = self.istek_hedefi(program, istek_id);
        let sorun = if z.tur != ZarfTuru::Yanit {
            Some("tür yanıt değil")
        } else if z.hedef != program {
            Some("yanıtın hedefi bu program değil")
        } else if z.yanit_id.as_deref() != Some(istek_id) {
            Some("yanit_id istek kimliğiyle uyuşmuyor")
        } else if beklenen.as_deref() != Some(z.kaynak.as_str()) {
            Some("yanıtın kaynağı isteğin hedefi değil ya da istek bulunamadı")
        } else {
            None
        };
        if let Some(sorun) = sorun {
            let hata = IpcHata::Gecersiz(sorun.to_string());
            self.uyusmayani_ayir(program, &yol, &hata.to_string());
            return None;
        }
        let _ = self.isleneni_tasi(program, &yol);
        Some(z)
    }

    fn olay_yayinla(&self, kaynak: &str, ad: &str, yuk: Value) -> Result<String, IpcHata> {
        program_dogrula(kaynak)?;
        yetenek_dogrula("olay adı", ad)?;
        let olay = Zarf {
            v: ZARF_SURUMU,
            id: yeni_id(),
            kaynak: kaynak.to_string(),
            hedef: OLAY_HEDEFI.to_string(),
            tur: ZarfTuru::Olay,
            yetenek: ad.to_string(),
            yuk,
            zaman: simdi_rfc3339(),
            yanit_id: None,
        };
        let bytes = Self::serilestir(&olay)?;
        let satir = String::from_utf8(bytes).map_err(|e| IpcHata::Gecersiz(e.to_string()))?;
        let gun = zaman_biciminde(simdi_saniye()).1;
        let yol = self.kok.join("olaylar").join(format!("{gun}.jsonl"));
        taa_dosya::satir_ekle(&yol, &satir)?;
        Ok(olay.id)
    }

    fn olaylar(&self, program: &str, son: &str) -> Vec<Zarf> {
        // Yalnız son 7 günün dosyaları (en eskiden bugüne); eski dosyalara hiç bakılmaz.
        let simdi = simdi_saniye();
        let mut hepsi: Vec<Zarf> = Vec::new();
        let mut bugun_baslangic = 0;
        for geri in (0..OLAY_GUN_SAYISI).rev() {
            let gun = zaman_biciminde(simdi - geri * 86_400).1;
            let yol = self.kok.join("olaylar").join(format!("{gun}.jsonl"));
            if geri == 0 {
                bugun_baslangic = hepsi.len();
            }
            self.olay_dosyasi_oku(&yol, &mut hepsi);
        }
        let baslangic = if son.is_empty() {
            0
        } else {
            hepsi
                .iter()
                .position(|z| z.id == son)
                .map_or(bugun_baslangic, |i| i + 1)
        };
        hepsi
            .into_iter()
            .skip(baslangic)
            .filter(|z| z.kaynak != program)
            .collect()
    }
}

impl GelenKutusu {
    /// Bir olay günlüğünü okur; en çok `OLAY_SATIR_SINIRI` (en yeni) satır tutulur, fazlası
    /// uyarıyla (günlüğe yazılarak) kesilir. Bozuk satır atlanır.
    fn olay_dosyasi_oku(&self, yol: &Path, cikti: &mut Vec<Zarf>) {
        use std::collections::VecDeque;
        use std::io::{BufRead, BufReader};
        let dosya = match fs::File::open(yol) {
            Ok(f) => f,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return,
            Err(e) => {
                self.gunluk("bozuk_olay_dosyasi", yol, &e.to_string());
                return;
            }
        };
        let mut son_satirlar: VecDeque<String> = VecDeque::new();
        let mut toplam = 0usize;
        for satir in BufReader::new(dosya).lines() {
            let satir = match satir {
                Ok(s) => s,
                Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                    self.gunluk("bozuk_olay", yol, "satır UTF-8 değil");
                    continue;
                }
                Err(e) => {
                    self.gunluk("bozuk_olay_dosyasi", yol, &e.to_string());
                    break;
                }
            };
            if satir.trim().is_empty() {
                continue;
            }
            toplam += 1;
            if son_satirlar.len() == OLAY_SATIR_SINIRI {
                son_satirlar.pop_front();
            }
            son_satirlar.push_back(satir);
        }
        if toplam > OLAY_SATIR_SINIRI {
            self.gunluk(
                "olay_satir_siniri",
                yol,
                &format!("uyarı: {toplam} satır var; yalnız son {OLAY_SATIR_SINIRI} satır okundu"),
            );
        }
        for satir in son_satirlar {
            if satir.len() > ZARF_SINIRI {
                self.gunluk("bozuk_olay", yol, "satır sınırı aşıyor");
                continue;
            }
            match serde_json::from_str::<Zarf>(&satir) {
                Ok(z) if z.dogrula().is_ok() && z.tur == ZarfTuru::Olay => cikti.push(z),
                _ => self.gunluk("bozuk_olay", yol, "satır ayrıştırılamadı"),
            }
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use serde_json::json;

    /// Geçici kök; düşünce kendi klasörünü temizler (yalnız test için).
    struct Kok(PathBuf);
    impl Kok {
        fn yeni() -> Kok {
            Kok(taa_dosya::gecici_klasor().unwrap())
        }
        fn kutu(&self) -> GelenKutusu {
            GelenKutusu::yeni(&self.0)
        }
    }
    impl Drop for Kok {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn sv(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn yeni_dosya_yaz_gecici_dosyayi_silmeden_kenara_alir() {
        let kok = Kok::yeni();
        let yol = kok.0.join("a.json");
        GelenKutusu::yeni_dosya_yaz(&yol, "a", b"{}").unwrap();
        assert_eq!(fs::read(&yol).unwrap(), b"{}");
        // Çakışmada hedef bozulmaz.
        let e = GelenKutusu::yeni_dosya_yaz(&yol, "a", b"{\"x\":1}");
        assert!(matches!(e, Err(IpcHata::IdCakismasi(_))));
        assert_eq!(fs::read(&yol).unwrap(), b"{}");
        // Dizinde .tmp kalmaz; artık klasörüne taşınmıştır.
        let tmp_var = fs::read_dir(&kok.0)
            .unwrap()
            .filter_map(|e| e.ok())
            .any(|e| e.path().extension().is_some_and(|x| x == "tmp"));
        assert!(!tmp_var);
        assert!(kok.0.join(".taa-ipc-artik").is_dir());
    }

    #[test]
    fn kimlik_denetimi() {
        for iyi in ["taarkod", "123-4-5678", "a-b", "x1"] {
            assert!(kimlik_gecerli(iyi), "{iyi}");
        }
        for kotu in [
            "",
            "..",
            ".gizli",
            "A",
            "a/b",
            "a\\b",
            "a b",
            "a\0b",
            "-a",
            "ğ",
            "a.b",
            "a@1",
            &"a".repeat(65),
        ] {
            assert!(!kimlik_gecerli(kotu), "{kotu:?}");
        }
    }

    #[test]
    fn program_adi_denetimi() {
        for iyi in ["taarkod", "tutaa", "a-b", "x1", &"a".repeat(32)] {
            assert!(program_adi_gecerli(iyi), "{iyi}");
        }
        for kotu in [
            "",
            "..",
            ".gizli",
            "A",
            "a/b",
            "a\\b",
            "a b",
            "a\0b",
            "-a",
            "1a",
            "ğ",
            "a.b",
            "a@1",
            "veri.urunler@1",
            &"a".repeat(33),
        ] {
            assert!(!program_adi_gecerli(kotu), "{kotu:?}");
        }
    }

    #[test]
    fn yetenek_adi_denetimi() {
        for iyi in ["kod.uret@1", "veri.urunler@12", "a-b.c-d@3", "a1.b2@0"] {
            assert!(yetenek_adi_gecerli(iyi), "{iyi}");
        }
        for kotu in [
            "",
            "kod",
            "kod.uret",
            "kod.uret@",
            "kod.uret@x",
            "kod@1",
            "a.b.c@1",
            ".uret@1",
            "kod.@1",
            "1kod.uret@1",
            "Kod.uret@1",
            "../x.y@1",
            "a/b.c@1",
            "kod.uret@1234567890",
            &format!("{}.uret@1", "a".repeat(64)),
        ] {
            assert!(!yetenek_adi_gecerli(kotu), "{kotu:?}");
        }
    }

    #[test]
    fn zaman_bicimi() {
        assert_eq!(zaman_biciminde(0).0, "1970-01-01T00:00:00Z");
        assert_eq!(zaman_biciminde(1_700_000_000).0, "2023-11-14T22:13:20Z");
        assert_eq!(zaman_biciminde(1_700_000_000).1, "20231114");
        assert_eq!(zaman_biciminde(951_782_400).0, "2000-02-29T00:00:00Z");
    }

    #[test]
    fn kimlikler_benzersiz() {
        let a = yeni_id();
        let b = yeni_id();
        assert_ne!(a, b);
        assert!(kimlik_gecerli(&a));
        assert!(a.starts_with(&format!("{}-", std::process::id())));
    }

    #[test]
    fn istek_yanit_dongusu() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        kutu.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        kutu.kayit_ol("tutaa", &sv(&["veri.urunler@1"])).unwrap();

        assert_eq!(
            kutu.yetenek_bul("kod.uret@1").unwrap().as_deref(),
            Some("taarkod")
        );
        assert_eq!(kutu.yetenek_bul("yok.yok@1").unwrap(), None);

        let istek = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!({"urun": 5}));
        let id = kutu.gonder(&istek).unwrap();
        assert_eq!(id, istek.id);

        let gelen = kutu.al("taarkod");
        assert_eq!(gelen.len(), 1);
        assert_eq!(gelen[0], istek);
        // Yanıtlanana kadar istek bekler.
        assert_eq!(kutu.al("taarkod").len(), 1);

        let yanit_id = kutu.yanitla(&gelen[0], json!({"kod": "ABC"})).unwrap();
        assert!(kutu.al("taarkod").is_empty());
        assert!(k
            .0
            .join("taarkod/islenen")
            .join(format!("{}.json", istek.id))
            .exists());

        let yanit = kutu.yanit_al("tutaa", &istek.id).unwrap();
        assert_eq!(yanit.id, yanit_id);
        assert_eq!(yanit.tur, ZarfTuru::Yanit);
        assert_eq!(yanit.yanit_id.as_deref(), Some(istek.id.as_str()));
        assert_eq!(yanit.yuk, json!({"kod": "ABC"}));
        assert_eq!(
            (yanit.kaynak.as_str(), yanit.hedef.as_str()),
            ("taarkod", "tutaa")
        );
        // Yanıt bir kez alınır; dosya silinmez, islenen/ altına taşınır.
        assert!(kutu.yanit_al("tutaa", &istek.id).is_none());
        assert!(k
            .0
            .join("tutaa/islenen")
            .join(format!("{}.json", istek.id))
            .exists());
    }

    #[test]
    fn yetenek_yoksa_hata() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        kutu.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        let z = Zarf::istek("tutaa", "taarkod", "baska.sey@1", json!(null));
        assert_eq!(
            kutu.gonder(&z),
            Err(IpcHata::YetenekYok("baska.sey@1".into()))
        );
        // Kayıtsız hedef de aynı hata.
        let z = Zarf::istek("tutaa", "yokprogram", "kod.uret@1", json!(null));
        assert!(matches!(kutu.gonder(&z), Err(IpcHata::YetenekYok(_))));
        assert!(!k.0.join("taarkod/gelen").read_dir().unwrap().any(|_| true));
    }

    #[test]
    fn buyuk_zarf_reddedilir_dosya_yazilmaz() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        kutu.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        let z = Zarf::istek(
            "tutaa",
            "taarkod",
            "kod.uret@1",
            json!("x".repeat(2 * 1024 * 1024)),
        );
        match kutu.gonder(&z) {
            Err(IpcHata::ZarfBuyuk { boyut, sinir }) => {
                assert!(boyut > sinir);
                assert_eq!(sinir, ZARF_SINIRI);
            }
            s => panic!("beklenmeyen: {s:?}"),
        }
        assert_eq!(k.0.join("taarkod/gelen").read_dir().unwrap().count(), 0);
        // Olay da aynı sınıra tabi.
        let s = kutu.olay_yayinla(
            "tutaa",
            "veri.degisti@1",
            json!("x".repeat(2 * 1024 * 1024)),
        );
        assert!(matches!(s, Err(IpcHata::ZarfBuyuk { .. })));
        assert!(!k.0.join("olaylar").exists());
    }

    #[test]
    fn bozuk_dosya_atlanir_ve_gunluge_yazilir() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        kutu.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        let gelen = k.0.join("taarkod/gelen");
        fs::write(gelen.join("0000-bozuk.json"), b"{ yarim").unwrap();
        fs::write(gelen.join("0001-sinir.json"), vec![b' '; ZARF_SINIRI + 1]).unwrap();
        fs::write(gelen.join("0002-alan.json"), br#"{"v":1}"#).unwrap();
        let z = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!(1));
        kutu.gonder(&z).unwrap();
        let alinan = kutu.al("taarkod");
        assert_eq!(alinan.len(), 1);
        assert_eq!(alinan[0].id, z.id);
        let gunluk = taa_dosya::satirlari_oku(&k.0.join("gunluk.jsonl")).unwrap();
        assert_eq!(gunluk.len(), 3);
        assert!(gunluk.iter().all(|s| s.contains("bozuk_zarf")));
        // Bozuk dosyalar silinmedi: islenen/bozuk/ altına taşındı, bir daha teslim edilmez.
        assert!(!gelen.join("0000-bozuk.json").exists());
        assert!(k.0.join("taarkod/islenen/bozuk/0000-bozuk.json").exists());
        assert!(k.0.join("taarkod/islenen/bozuk/0001-sinir.json").exists());
        assert!(k.0.join("taarkod/islenen/bozuk/0002-alan.json").exists());
    }

    #[test]
    fn olay_yayinla_ve_abone() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        let a = kutu
            .olay_yayinla("tutaa", "veri.degisti@1", json!({"n": 1}))
            .unwrap();
        let b = kutu
            .olay_yayinla("tutaa", "veri.degisti@1", json!({"n": 2}))
            .unwrap();
        let c = kutu
            .olay_yayinla("taarkod", "baska.olay@1", json!(null))
            .unwrap();

        let hepsi = kutu.olaylar("abone", "");
        assert_eq!(
            hepsi.iter().map(|z| z.id.clone()).collect::<Vec<_>>(),
            [a.clone(), b.clone(), c.clone()]
        );
        assert!(hepsi
            .iter()
            .all(|z| z.tur == ZarfTuru::Olay && z.hedef == OLAY_HEDEFI));
        // Son okunan kimlikten sonrası.
        let sonra = kutu.olaylar("abone", &a);
        assert_eq!(
            sonra.iter().map(|z| z.id.clone()).collect::<Vec<_>>(),
            [b.clone(), c.clone()]
        );
        assert!(kutu.olaylar("abone", &c).is_empty());
        // Kendi olayları gelmez.
        assert_eq!(kutu.olaylar("tutaa", "").len(), 1);
        // Bozuk satır okumayı durdurmaz.
        let gun = fs::read_dir(k.0.join("olaylar"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path();
        taa_dosya::satir_ekle(&gun, "bozuk{").unwrap();
        let d = kutu
            .olay_yayinla("tutaa", "veri.degisti@1", json!(3))
            .unwrap();
        assert_eq!(
            kutu.olaylar("abone", &c)
                .iter()
                .map(|z| z.id.clone())
                .collect::<Vec<_>>(),
            [d]
        );
        assert_eq!(
            taa_dosya::satirlari_oku(&k.0.join("gunluk.jsonl"))
                .unwrap()
                .len(),
            1
        );
    }

    #[test]
    fn yol_kacisi_ve_gecersiz_adlar_reddedilir() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        for kotu in ["..", "a/b", "a\\b", "kayit", "olaylar", "", "A"] {
            assert!(
                matches!(kutu.kayit_ol(kotu, &[]), Err(IpcHata::Gecersiz(_))),
                "{kotu:?}"
            );
        }
        assert!(kutu.kayit_ol("iyi", &sv(&["../x"])).is_err());
        kutu.kayit_ol("iyi", &sv(&["a.b@1"])).unwrap();
        let mut z = Zarf::istek("tutaa", "iyi", "a.b@1", json!(null));
        z.hedef = "../iyi".into();
        assert!(matches!(kutu.gonder(&z), Err(IpcHata::Gecersiz(_))));
        let mut z = Zarf::istek("tutaa", "iyi", "a.b@1", json!(null));
        z.id = "../kacis".into();
        assert!(matches!(kutu.gonder(&z), Err(IpcHata::Gecersiz(_))));
        assert!(kutu.al("..").is_empty());
        assert!(kutu.yanit_al("iyi", "../x").is_none());
        // Yanlış tür gönderilemez.
        let mut z = Zarf::istek("tutaa", "iyi", "a.b@1", json!(null));
        z.tur = ZarfTuru::Olay;
        assert!(matches!(kutu.gonder(&z), Err(IpcHata::Gecersiz(_))));
        // Kök dışına hiçbir şey yazılmadı.
        assert!(!k.0.parent().unwrap().join("kacis.json").exists());
    }

    #[test]
    fn kayit_atomik_ve_icerikli() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        kutu.kayit_ol("taarkod", &sv(&["kod.uret@1", "kod.oku@1"]))
            .unwrap();
        kutu.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap(); // üzerine yazar
        let v: Value =
            serde_json::from_slice(&fs::read(k.0.join("kayit/taarkod.json")).unwrap()).unwrap();
        assert_eq!(v["program"], "taarkod");
        assert_eq!(v["yetenekler"], json!(["kod.uret@1"]));
        assert_eq!(v["pid"], std::process::id());
        // Geçici dosya kalmadı.
        let adlar: Vec<_> = fs::read_dir(k.0.join("kayit")).unwrap().collect();
        assert_eq!(adlar.len(), 1);
    }

    fn olay_satiri(id: &str, kaynak: &str) -> String {
        let z = Zarf {
            v: ZARF_SURUMU,
            id: id.to_string(),
            kaynak: kaynak.to_string(),
            hedef: OLAY_HEDEFI.to_string(),
            tur: ZarfTuru::Olay,
            yetenek: "veri.degisti@1".to_string(),
            yuk: json!(null),
            zaman: simdi_rfc3339(),
            yanit_id: None,
        };
        serde_json::to_string(&z).unwrap()
    }

    fn gunluk_satirlari(k: &Kok) -> Vec<String> {
        taa_dosya::satirlari_oku(&k.0.join("gunluk.jsonl")).unwrap_or_default()
    }

    fn iki_program(k: &Kok) -> GelenKutusu {
        let kutu = k.kutu();
        kutu.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        kutu.kayit_ol("tutaa", &sv(&["veri.urunler@1"])).unwrap();
        kutu
    }

    // --- Madde 1: yanıt sahteciliği / çift dosya ---

    #[test]
    fn ayni_idyle_ikinci_istek_ezmez() {
        let k = Kok::yeni();
        let kutu = iki_program(&k);
        let z = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!({"n": 1}));
        kutu.gonder(&z).unwrap();
        let yol = k.0.join("taarkod/gelen").join(format!("{}.json", z.id));
        let once = fs::read(&yol).unwrap();
        let mut ikinci = z.clone();
        ikinci.yuk = json!({"n": 2});
        assert_eq!(
            kutu.gonder(&ikinci),
            Err(IpcHata::IdCakismasi(z.id.clone()))
        );
        assert_eq!(fs::read(&yol).unwrap(), once, "ilk istek değişmemeli");
        // "(1)" eki alan ikinci dosya oluşmadı; dizinde geçici dosya da kalmadı
        // (geçici dosya silinmez, `.taa-ipc-artik/` altına taşınır).
        let dosyalar =
            k.0.join("taarkod/gelen")
                .read_dir()
                .unwrap()
                .filter_map(|e| e.ok())
                .filter(|e| e.path().is_file())
                .count();
        assert_eq!(dosyalar, 1);
    }

    #[test]
    fn onceden_var_yanit_ezilmez() {
        let k = Kok::yeni();
        let kutu = iki_program(&k);
        let z = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!(1));
        kutu.gonder(&z).unwrap();
        let sahte = k.0.join("tutaa/yanit").join(format!("{}.json", z.id));
        fs::write(&sahte, b"sahte").unwrap();
        let gelen = kutu.al("taarkod");
        assert_eq!(
            kutu.yanitla(&gelen[0], json!({"kod": "GERCEK"})),
            Err(IpcHata::IdCakismasi(z.id.clone()))
        );
        assert_eq!(fs::read(&sahte).unwrap(), b"sahte");
        assert_eq!(k.0.join("tutaa/yanit").read_dir().unwrap().count(), 1);
        // İstek yanıtlanamadığı için gelen/ altında kalır.
        assert_eq!(kutu.al("taarkod").len(), 1);
    }

    #[test]
    fn sahte_yanit_kabul_edilmez_ve_islenene_tasinir() {
        let k = Kok::yeni();
        let kutu = iki_program(&k);
        let z = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!(1));
        kutu.gonder(&z).unwrap();
        let yanit_yolu = k.0.join("tutaa/yanit").join(format!("{}.json", z.id));

        // Kaynağı yanlış (isteğin hedefi taarkod, sahte yanıt "tutaa-kotu" diyor).
        let sahte_yaz = |kaynak: &str, yanit_id: Option<&str>| {
            let mut y = Zarf::istek(kaynak, "tutaa", "kod.uret@1", json!({"kod": "SAHTE"}));
            y.tur = ZarfTuru::Yanit;
            y.yanit_id = yanit_id.map(str::to_string);
            fs::write(&yanit_yolu, serde_json::to_vec(&y).unwrap()).unwrap();
        };
        sahte_yaz("kotu", Some(&z.id));
        assert!(kutu.yanit_al("tutaa", &z.id).is_none());
        assert!(!yanit_yolu.exists());
        assert!(k
            .0
            .join("tutaa/islenen")
            .join(format!("{}.json", z.id))
            .exists());
        assert!(gunluk_satirlari(&k)
            .iter()
            .any(|s| s.contains("yanlis_zarf")));

        // yanit_id istek kimliğiyle uyuşmuyor.
        sahte_yaz("taarkod", Some("baska-id"));
        assert!(kutu.yanit_al("tutaa", &z.id).is_none());
        sahte_yaz("taarkod", None);
        assert!(kutu.yanit_al("tutaa", &z.id).is_none());
        assert!(!yanit_yolu.exists());

        // Gerçek yanıt hâlâ yazılıp alınabilir (sahte dosya yeri tıkamıyor).
        let gelen = kutu.al("taarkod");
        kutu.yanitla(&gelen[0], json!({"kod": "GERCEK"})).unwrap();
        let yanit = kutu.yanit_al("tutaa", &z.id).unwrap();
        assert_eq!(yanit.yuk, json!({"kod": "GERCEK"}));
    }

    #[test]
    fn istegi_olmayan_yanit_kabul_edilmez() {
        let k = Kok::yeni();
        let kutu = iki_program(&k);
        let mut y = Zarf::istek("taarkod", "tutaa", "kod.uret@1", json!(null));
        y.tur = ZarfTuru::Yanit;
        y.yanit_id = Some("hayalet-1".into());
        let yol = k.0.join("tutaa/yanit/hayalet-1.json");
        fs::write(&yol, serde_json::to_vec(&y).unwrap()).unwrap();
        assert!(kutu.yanit_al("tutaa", "hayalet-1").is_none());
        assert!(k.0.join("tutaa/islenen/hayalet-1.json").exists());
    }

    #[test]
    fn al_dosya_adi_ile_zarf_id_uyusmali() {
        let k = Kok::yeni();
        let kutu = iki_program(&k);
        let z = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!(1));
        let gelen = k.0.join("taarkod/gelen");
        fs::write(gelen.join("baska-ad.json"), serde_json::to_vec(&z).unwrap()).unwrap();
        assert!(kutu.al("taarkod").is_empty());
        assert!(!gelen.join("baska-ad.json").exists());
        assert!(k.0.join("taarkod/islenen/baska-ad.json").exists());
        assert!(gunluk_satirlari(&k)
            .iter()
            .any(|s| s.contains("yanlis_zarf")));
        // Doğru adlı olan teslim edilir.
        fs::write(
            gelen.join(format!("{}.json", z.id)),
            serde_json::to_vec(&z).unwrap(),
        )
        .unwrap();
        assert_eq!(kutu.al("taarkod").len(), 1);
    }

    // --- Madde 2: program adı ---

    #[test]
    fn ayrilmis_ve_noktali_program_adlari_reddedilir() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        for kotu in [
            "gunluk", "kayit", "olaylar", "islenen", "gelen", "yanit", "a.b", "a@1", "1a",
        ] {
            assert!(
                matches!(kutu.kayit_ol(kotu, &[]), Err(IpcHata::Gecersiz(_))),
                "{kotu:?}"
            );
        }
        assert!(kutu.al("gunluk").is_empty());
        assert!(kutu.yanit_al("islenen", "x1").is_none());
        let z = Zarf::istek("tutaa", "gelen", "a.b@1", json!(null));
        assert!(matches!(kutu.gonder(&z), Err(IpcHata::Gecersiz(_))));
    }

    // --- Madde 3: kayıt sahipliği, belirsiz yetenek ---

    fn kayit_yaz(k: &Kok, program: &str, pid: u32, zaman: &str, yetenekler: &[&str]) {
        fs::create_dir_all(k.0.join("kayit")).unwrap();
        let v = json!({"program": program, "yetenekler": yetenekler, "pid": pid, "zaman": zaman});
        fs::write(
            k.0.join("kayit").join(format!("{program}.json")),
            serde_json::to_vec(&v).unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn taze_baska_pid_kaydi_ezilmez_bayat_ezilir() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        let yabanci = std::process::id() + 1;
        kayit_yaz(&k, "taarkod", yabanci, &simdi_rfc3339(), &["kod.uret@1"]);
        let yol = k.0.join("kayit/taarkod.json");
        let once = fs::read(&yol).unwrap();
        assert_eq!(
            kutu.kayit_ol("taarkod", &sv(&["kod.oku@1"])),
            Err(IpcHata::KayitDolu("taarkod".into()))
        );
        assert_eq!(fs::read(&yol).unwrap(), once);

        // 11 dakika önceki kayıt bayat sayılır.
        let eski = zaman_biciminde(simdi_saniye() - 11 * 60).0;
        kayit_yaz(&k, "taarkod", yabanci, &eski, &["kod.uret@1"]);
        kutu.kayit_ol("taarkod", &sv(&["kod.oku@1"])).unwrap();
        let v: Value = serde_json::from_slice(&fs::read(&yol).unwrap()).unwrap();
        assert_eq!(v["pid"], std::process::id());

        // Aynı pid taze kaydını yenileyebilir.
        kutu.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        // Zamanı okunamayan kayıt bayat sayılır.
        kayit_yaz(&k, "tutaa", yabanci, "dun", &[]);
        kutu.kayit_ol("tutaa", &[]).unwrap();
    }

    #[test]
    fn yetenegi_birden_cok_program_sunarsa_belirsiz() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        kutu.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        assert_eq!(
            kutu.yetenek_bul("kod.uret@1").unwrap().as_deref(),
            Some("taarkod")
        );
        kutu.kayit_ol("sahtekar", &sv(&["kod.uret@1"])).unwrap();
        match kutu.yetenek_bul("kod.uret@1") {
            Err(IpcHata::Belirsiz {
                yetenek,
                programlar,
            }) => {
                assert_eq!(yetenek, "kod.uret@1");
                assert_eq!(programlar, ["sahtekar", "taarkod"]);
            }
            s => panic!("beklenmeyen: {s:?}"),
        }
        // Başka yetenek etkilenmez.
        assert_eq!(kutu.yetenek_bul("yok.yok@1").unwrap(), None);
    }

    #[test]
    fn rfc3339_gidis_donus() {
        for s in [0, 951_782_400, 1_700_000_000, simdi_saniye()] {
            assert_eq!(rfc3339_saniye(&zaman_biciminde(s).0), Some(s));
        }
        for kotu in ["", "dun", "2023-13-01T00:00:00Z", "2023-11-14 22:13:20Z"] {
            assert_eq!(rfc3339_saniye(kotu), None, "{kotu:?}");
        }
    }

    // --- Madde 4: olaylar penceresi, satır sınırı, bozuk zarf ---

    #[test]
    fn olaylar_yalniz_son_7_gun() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        fs::create_dir_all(k.0.join("olaylar")).unwrap();
        let gun = |geri: i64| zaman_biciminde(simdi_saniye() - geri * 86_400).1;
        let yaz = |geri: i64, id: &str| {
            fs::write(
                k.0.join("olaylar").join(format!("{}.jsonl", gun(geri))),
                format!("{}\n", olay_satiri(id, "tutaa")),
            )
            .unwrap();
        };
        yaz(30, "cok-eski");
        yaz(8, "sekiz-gun");
        yaz(6, "alti-gun");
        yaz(1, "dun");
        let bugun = kutu
            .olay_yayinla("tutaa", "veri.degisti@1", json!(1))
            .unwrap();

        let idler = |v: Vec<Zarf>| v.into_iter().map(|z| z.id).collect::<Vec<String>>();
        assert_eq!(
            idler(kutu.olaylar("abone", "")),
            ["alti-gun", "dun", bugun.as_str()]
        );
        // Bulunan son idden sonrası.
        assert_eq!(
            idler(kutu.olaylar("abone", "alti-gun")),
            ["dun", bugun.as_str()]
        );
        // Pencere dışındaki id "bulunamaz": yalnız bugünün dosyası döner.
        assert_eq!(idler(kutu.olaylar("abone", "sekiz-gun")), [bugun.as_str()]);
        assert_eq!(idler(kutu.olaylar("abone", "hic-yok")), [bugun.as_str()]);
    }

    #[test]
    fn olay_dosyasi_satir_siniri() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        fs::create_dir_all(k.0.join("olaylar")).unwrap();
        let gun = zaman_biciminde(simdi_saniye()).1;
        let toplam = OLAY_SATIR_SINIRI + 5;
        let mut icerik = String::new();
        for i in 0..toplam {
            icerik.push_str(&olay_satiri(&format!("o-{i}"), "tutaa"));
            icerik.push('\n');
        }
        fs::write(k.0.join("olaylar").join(format!("{gun}.jsonl")), icerik).unwrap();
        let olaylar = kutu.olaylar("abone", "");
        assert_eq!(olaylar.len(), OLAY_SATIR_SINIRI);
        // En yeni satırlar korunur.
        assert_eq!(olaylar.last().unwrap().id, format!("o-{}", toplam - 1));
        assert_eq!(olaylar[0].id, "o-5");
        let g = gunluk_satirlari(&k);
        assert_eq!(
            g.iter().filter(|s| s.contains("olay_satir_siniri")).count(),
            1
        );
    }

    #[test]
    fn bozuk_zarf_bir_kez_gunluge_dusar_ve_yeniden_teslim_edilmez() {
        let k = Kok::yeni();
        let kutu = iki_program(&k);
        let gelen = k.0.join("taarkod/gelen");
        fs::write(gelen.join("bozuk-1.json"), b"{ yarim").unwrap();
        assert!(kutu.al("taarkod").is_empty());
        assert!(kutu.al("taarkod").is_empty());
        assert!(kutu.al("taarkod").is_empty());
        assert_eq!(gunluk_satirlari(&k).len(), 1);
        assert!(!gelen.join("bozuk-1.json").exists());
        assert_eq!(
            fs::read(k.0.join("taarkod/islenen/bozuk/bozuk-1.json")).unwrap(),
            b"{ yarim"
        );
        // Aynı adlı ikinci bozuk dosya öncekinin üzerine yazmaz.
        fs::write(gelen.join("bozuk-1.json"), b"ikinci").unwrap();
        assert!(kutu.al("taarkod").is_empty());
        assert_eq!(
            k.0.join("taarkod/islenen/bozuk")
                .read_dir()
                .unwrap()
                .count(),
            2
        );
    }

    #[test]
    fn ortam_koku_calisir() {
        // set_var kullanılmaz (paralel test); yalnız çözümün hata vermediği doğrulanır.
        assert!(GelenKutusu::ortamdan().is_ok() || std::env::var_os("HOME").is_none());
    }
}
