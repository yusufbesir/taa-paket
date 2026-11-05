// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! Boru taşıyıcısının platformdan bağımsız parçaları (Linux'ta da derlenir ve test edilir).
//!
//! * Boru adı: `\\.\pipe\taa-<kullanici-kimligi>-<program>`.
//! * Çerçeveleme: 4 bayt uzunluk (büyük bayt sırası) + JSON gövde; gövde en çok 256 KiB.
//! * Zaman aşımı: bağlantı ve alışveriş için tek toplam süre (`Sure`).
//! * ACL kararı: boruya yalnız geçerli kullanıcı erişir (`acl_sddl`).
//! * Sunucu tarafı teslim: gelen çerçeve doğrulanır ve dosya tabanlı `GelenKutusu` düzenine yazılır
//!   (kalıcılık); ardından `Onay` çerçevesi döner. Alıcı `al`/`yanit_al` ile her zamanki gibi okur.
//! * `YedekliTasiyici`: boru yolunu dener, boru yoksa/başarısızsa dosyaya düşer. Gerçek boru
//!   uygulaması (`BoruUcu`) yalnız Windows'tadır (`boru.rs`); testler sahte uçla çalışır.
//!
//! Olaylar boruyla taşınmaz: `veri.degisti@1` gibi olaylar `olaylar/<gün>.jsonl` günlüğüne yazılır
//! (abone `son` kimliğiyle yoklar); sıralama ve 7 gün penceresi dosyadadır.

use super::{program_dogrula, yetenek_dogrula, GelenKutusu, IpcHata, Tasiyici, Zarf, ZarfTuru};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::io::{self, Read, Write};
use std::time::{Duration, Instant};

/// Boru çerçeve gövdesinin en büyük boyutu: 256 KiB. Daha büyük zarf (en çok 1 MiB) dosyaya düşer.
pub const BORU_CERCEVE_SINIRI: usize = 256 * 1024;
/// Bir boru alışverişinin (bağlan, yaz, onay oku) toplam süresi.
pub const BORU_ZAMAN_ASIMI: Duration = Duration::from_secs(3);
/// Boru adındaki kullanıcı kimliği parçasının en çok uzunluğu (ad toplamı 256 karakteri aşmasın).
const KULLANICI_KIMLIGI_SINIRI: usize = 100;
/// SID metni için üst sınır (en çok 15 alt yetkili SID ≈ 184 karakter).
const SID_SINIRI: usize = 184;

// ---------------------------------------------------------------------------------------------
// Boru adı
// ---------------------------------------------------------------------------------------------

/// Kullanıcı kimliğini boru adına uygun hale getirir: küçük harfe çevirir; yalnız `[a-z0-9-]`,
/// harf/rakamla başlar, en çok 100 karakter. Başka karakter DEĞİŞTİRİLMEZ, reddedilir (iki farklı
/// kimliğin aynı ada düşmesi engellenir). Windows'ta kimlik, kullanıcının SID metnidir
/// (`S-1-5-21-...` → `s-1-5-21-...`).
pub fn kullanici_kimligi_temizle(ham: &str) -> Result<String, IpcHata> {
    let k = ham.trim().to_ascii_lowercase();
    let baslangic_iyi = k
        .chars()
        .next()
        .is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    let iyi = baslangic_iyi
        && k.len() <= KULLANICI_KIMLIGI_SINIRI
        && k.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if iyi {
        Ok(k)
    } else {
        Err(IpcHata::Gecersiz(
            "kullanıcı kimliği yalnız [a-z0-9-] içerebilir, harf/rakamla başlamalı ve en çok 100 karakter olmalı"
                .into(),
        ))
    }
}

/// Boru adı: `\\.\pipe\taa-<kullanici-kimligi>-<program>`. Program adı `program_dogrula`
/// kurallarına uymalıdır (yol kaçışı ve ayrılmış ad yok).
pub fn boru_adi(kullanici_kimligi: &str, program: &str) -> Result<String, IpcHata> {
    let k = kullanici_kimligi_temizle(kullanici_kimligi)?;
    program_dogrula(program)?;
    Ok(format!(r"\\.\pipe\taa-{k}-{program}"))
}

// ---------------------------------------------------------------------------------------------
// ACL kararı
// ---------------------------------------------------------------------------------------------

/// SID metni biçimi: `S-1-<rakamlar>(-<rakamlar>)*`.
fn sid_gecerli(sid: &str) -> bool {
    let Some(kalan) = sid.strip_prefix("S-1-") else {
        return false;
    };
    sid.len() <= SID_SINIRI
        && !kalan.is_empty()
        && kalan
            .split('-')
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

/// Boru güvenlik tanımlayıcısı (SDDL): korumalı (devralma yok) DACL, tek giriş = yalnız verilen
/// kullanıcıya tam erişim. Başka kullanıcı, Administrators ve SYSTEM dahil, bağlanamaz.
/// SID biçimi doğrulanır; SDDL'e `)` `;` gibi karakter sızdırılamaz.
pub fn acl_sddl(kullanici_sid: &str) -> Result<String, IpcHata> {
    if !sid_gecerli(kullanici_sid) {
        return Err(IpcHata::Gecersiz(format!(
            "SID biçimi geçersiz: {kullanici_sid:?}"
        )));
    }
    Ok(format!("D:P(A;;GA;;;{kullanici_sid})"))
}

// ---------------------------------------------------------------------------------------------
// Zaman aşımı
// ---------------------------------------------------------------------------------------------

/// Toplam süre sayacı: tek bir alışverişin bütün adımları aynı son tarihi paylaşır.
#[derive(Debug, Clone, Copy)]
pub struct Sure {
    son: Instant,
}

impl Sure {
    pub fn yeni(sure: Duration) -> Sure {
        let simdi = Instant::now();
        let son = simdi
            .checked_add(sure)
            .unwrap_or_else(|| simdi + Duration::from_secs(3600));
        Sure { son }
    }

    /// Kalan süre; dolduysa `None`.
    pub fn kalan(&self) -> Option<Duration> {
        self.son
            .checked_duration_since(Instant::now())
            .filter(|d| !d.is_zero())
    }

    /// Kalan süre milisaniye; dolduysa 0. Dolmadıysa en az 1; Windows `INFINITE` (u32::MAX)
    /// değerine hiç ulaşmaz.
    pub fn kalan_ms(&self) -> u32 {
        self.kalan().map_or(0, |d| {
            d.as_millis().clamp(1, u128::from(u32::MAX - 1)) as u32
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Çerçeveleme
// ---------------------------------------------------------------------------------------------

/// Çerçeve okuma/yazma hataları.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CerceveHata {
    /// Gövde 256 KiB sınırını aşıyor (okumada bellek AYRILMADAN reddedilir).
    Buyuk { boyut: usize, sinir: usize },
    /// Uzunluk 0: boş çerçeve geçersizdir.
    Bos,
    /// Akış çerçeve bitmeden kesildi.
    Kesik,
    /// Süre doldu.
    ZamanAsimi,
    /// Başka G/Ç hatası.
    Io(String),
}

impl fmt::Display for CerceveHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CerceveHata::Buyuk { boyut, sinir } => {
                write!(f, "boru çerçevesi çok büyük: {boyut} bayt (sınır {sinir})")
            }
            CerceveHata::Bos => write!(f, "boş boru çerçevesi"),
            CerceveHata::Kesik => write!(f, "boru çerçevesi yarıda kesildi"),
            CerceveHata::ZamanAsimi => write!(f, "boru zaman aşımı"),
            CerceveHata::Io(n) => write!(f, "boru G/Ç hatası: {n}"),
        }
    }
}

impl std::error::Error for CerceveHata {}

impl From<io::Error> for CerceveHata {
    fn from(e: io::Error) -> Self {
        match e.kind() {
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => CerceveHata::ZamanAsimi,
            io::ErrorKind::UnexpectedEof => CerceveHata::Kesik,
            _ => CerceveHata::Io(e.to_string()),
        }
    }
}

impl From<CerceveHata> for IpcHata {
    fn from(e: CerceveHata) -> Self {
        match e {
            CerceveHata::Buyuk { boyut, sinir } => IpcHata::ZarfBuyuk { boyut, sinir },
            diger => IpcHata::Io(diger.to_string()),
        }
    }
}

/// Gövdenin boruyla gidip gidemeyeceği (`<= BORU_CERCEVE_SINIRI`). Sığmayan zarf dosyaya düşer.
pub fn boru_icin_uygun(govde_boyutu: usize) -> bool {
    (1..=BORU_CERCEVE_SINIRI).contains(&govde_boyutu)
}

/// Çerçeve yazar: 4 bayt uzunluk (büyük bayt sırası) + gövde, tek yazımda.
pub fn cerceve_yaz<W: Write>(w: &mut W, govde: &[u8]) -> Result<(), CerceveHata> {
    if govde.is_empty() {
        return Err(CerceveHata::Bos);
    }
    if govde.len() > BORU_CERCEVE_SINIRI {
        return Err(CerceveHata::Buyuk {
            boyut: govde.len(),
            sinir: BORU_CERCEVE_SINIRI,
        });
    }
    let mut tampon = Vec::with_capacity(4 + govde.len());
    // Sınır 256 KiB olduğundan u32'ye sığar.
    tampon.extend_from_slice(&(govde.len() as u32).to_be_bytes());
    tampon.extend_from_slice(govde);
    w.write_all(&tampon)?;
    w.flush()?;
    Ok(())
}

/// Çerçeve okur. Uzunluk sınırı gövde için bellek ayrılmadan ÖNCE denetlenir.
pub fn cerceve_oku<R: Read>(r: &mut R) -> Result<Vec<u8>, CerceveHata> {
    let mut baslik = [0u8; 4];
    r.read_exact(&mut baslik)?;
    let boyut = u32::from_be_bytes(baslik) as usize;
    if boyut == 0 {
        return Err(CerceveHata::Bos);
    }
    if boyut > BORU_CERCEVE_SINIRI {
        return Err(CerceveHata::Buyuk {
            boyut,
            sinir: BORU_CERCEVE_SINIRI,
        });
    }
    let mut govde = vec![0u8; boyut];
    r.read_exact(&mut govde)?;
    Ok(govde)
}

// ---------------------------------------------------------------------------------------------
// Onay ve sunucu tarafı teslim
// ---------------------------------------------------------------------------------------------

/// Sunucunun her çerçeveye verdiği yanıt (JSON, çerçevelenmiş).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Onay {
    pub tamam: bool,
    /// Red nedeni (insan okuyabilir).
    #[serde(default)]
    pub neden: Option<String>,
    /// Makine okuyabilir red kodu: `yetenek_yok`, `id_cakismasi`, `zarf_buyuk`, `gecersiz`.
    #[serde(default)]
    pub kod: Option<String>,
}

impl Onay {
    pub fn tamam() -> Onay {
        Onay {
            tamam: true,
            neden: None,
            kod: None,
        }
    }

    pub fn red(h: &IpcHata) -> Onay {
        let kod = match h {
            IpcHata::YetenekYok(_) => "yetenek_yok",
            IpcHata::IdCakismasi(_) => "id_cakismasi",
            IpcHata::ZarfBuyuk { .. } => "zarf_buyuk",
            _ => "gecersiz",
        };
        Onay {
            tamam: false,
            neden: Some(h.to_string()),
            kod: Some(kod.to_string()),
        }
    }

    /// Reddi istemci tarafında `IpcHata`ya çevirir (`id` = gönderilen zarfın kimliği).
    fn hataya(&self, id: &str, yetenek: &str) -> IpcHata {
        let neden = self
            .neden
            .clone()
            .unwrap_or_else(|| "alıcı reddetti".into());
        match self.kod.as_deref() {
            Some("id_cakismasi") => IpcHata::IdCakismasi(id.to_string()),
            Some("yetenek_yok") => IpcHata::YetenekYok(yetenek.to_string()),
            _ => IpcHata::Gecersiz(neden),
        }
    }
}

/// Sunucu tarafı: `program`ın borusuna gelen bir çerçeve gövdesini doğrular ve dosya tabanlı
/// düzene yazar (istek → `gelen/<id>.json`, yanıt → `yanit/<istek-id>.json`). Yazılmadan onay
/// verilmez. Olaylar boruyla alınmaz.
pub fn sunucu_isle(kutu: &GelenKutusu, program: &str, govde: &[u8]) -> Onay {
    match teslim_al(kutu, program, govde) {
        Ok(()) => Onay::tamam(),
        Err(h) => Onay::red(&h),
    }
}

fn teslim_al(kutu: &GelenKutusu, program: &str, govde: &[u8]) -> Result<(), IpcHata> {
    program_dogrula(program)?;
    let zarf: Zarf =
        serde_json::from_slice(govde).map_err(|e| IpcHata::Gecersiz(format!("JSON bozuk: {e}")))?;
    zarf.dogrula()?;
    program_dogrula(&zarf.kaynak)?;
    if zarf.hedef != program {
        return Err(IpcHata::Gecersiz(
            "zarfın hedefi bu borunun programı değil".into(),
        ));
    }
    let bytes = GelenKutusu::serilestir(&zarf)?;
    match zarf.tur {
        ZarfTuru::Istek => {
            let sunulan = kutu
                .kayit_oku(&kutu.kayit_yolu(program))
                .is_some_and(|y| y.contains(&zarf.yetenek));
            if !sunulan {
                return Err(IpcHata::YetenekYok(zarf.yetenek.clone()));
            }
            let yol = kutu
                .klasor(program, "gelen")
                .join(format!("{}.json", zarf.id));
            GelenKutusu::yeni_dosya_yaz(&yol, &zarf.id, &bytes)
        }
        ZarfTuru::Yanit => {
            let Some(istek_id) = zarf.yanit_id.as_deref() else {
                return Err(IpcHata::Gecersiz("yanıtta yanit_id yok".into()));
            };
            let yol = kutu
                .klasor(program, "yanit")
                .join(format!("{istek_id}.json"));
            GelenKutusu::yeni_dosya_yaz(&yol, istek_id, &bytes)
        }
        ZarfTuru::Olay => Err(IpcHata::Gecersiz(
            "olaylar boruyla alınmaz; olaylar/ günlüğüne yazılır".into(),
        )),
    }
}

// ---------------------------------------------------------------------------------------------
// Yedekli taşıyıcı
// ---------------------------------------------------------------------------------------------

/// Boru ucunun hataları.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoruHatasi {
    /// Boru yok (sunucu çalışmıyor). Hiçbir şey gönderilmedi.
    Yok,
    /// Borudaki sunucu sürecin kimliği kayıttakiyle uyuşmuyor. Hiçbir şey gönderilmedi.
    KimlikUyusmadi,
    /// Süre doldu; zarf iletilmiş olabilir.
    ZamanAsimi,
    /// Sunucu başlatılamadı: ad başka bir süreçte (ilk örnek alınamadı).
    Dolu,
    /// Başka hata; zarf iletilmiş olabilir.
    Io(String),
}

impl fmt::Display for BoruHatasi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BoruHatasi::Yok => write!(f, "boru yok"),
            BoruHatasi::KimlikUyusmadi => write!(f, "boru sunucusunun kimliği kayıtla uyuşmuyor"),
            BoruHatasi::ZamanAsimi => write!(f, "boru zaman aşımı"),
            BoruHatasi::Dolu => write!(f, "boru adı başka bir süreçte"),
            BoruHatasi::Io(n) => write!(f, "boru hatası: {n}"),
        }
    }
}

impl std::error::Error for BoruHatasi {}

/// Boru ucu: gerçek boru G/Ç'si. Windows'ta `WindowsBoru`; testlerde sahte uç.
pub trait BoruUcu {
    /// `hedef` programın borusuna bağlanır, `beklenen_pid` ile sunucu sürecin kimliğini doğrular
    /// (uyuşmazsa HİÇBİR ŞEY yazmadan `KimlikUyusmadi`), tek çerçeve yazar ve onay çerçevesinin
    /// gövdesini döner. Toplam süre `zaman_asimi`'ni aşmaz.
    fn degis(
        &self,
        hedef: &str,
        beklenen_pid: u32,
        istek: &[u8],
        zaman_asimi: Duration,
    ) -> Result<Vec<u8>, BoruHatasi>;

    /// Bu süreçte `program` için boru sunucusunu başlatır (aynı program için ikinci çağrı
    /// başarılı döner). Ad başka süreçteyse `Dolu`. Gelen çerçeveler `sunucu_isle` ile işlenir.
    fn sunucu_baslat(&self, program: &str, kutu: &GelenKutusu) -> Result<(), BoruHatasi>;
}

/// Boru denemesinin sonucu.
enum Deneme {
    /// Alıcı teslim aldı ve onayladı.
    Teslim,
    /// Alıcı süreç ulaşıldı ama reddetti: dosyaya DÜŞÜLMEZ (doğrulamayı atlatmak olurdu).
    Red(IpcHata),
    /// Boru kullanılamadı. `belirsiz`: zarf iletilmiş olabilir (zaman aşımı, kesik onay).
    Yedege { belirsiz: bool },
}

/// Boru yolunu deneyen, olmazsa dosya tabanlı `GelenKutusu`'na düşen taşıyıcı.
///
/// Kurallar:
/// * `kayit_ol`, `yetenek_bul`, `al`, `yanit_al`, `olay_yayinla`, `olaylar` dosya tabanlıdır
///   (kayıt ve olay günlüğü kalıcılık yoludur; boru teslimleri de dosyaya yazılır).
/// * `gonder` / `yanitla`: gövde <= 256 KiB ve hedef kayıtlıysa boru denenir. Alıcı çerçeveyi
///   doğrulayıp dosyaya yazar ve onaylar. Boru yoksa, kimlik uyuşmazsa ya da süre dolarsa dosyaya
///   düşülür; süre dolduktan sonra dosyada kimlik çakışması çıkarsa zarf zaten iletilmiştir
///   (başarı sayılır, çift teslim olmaz). Alıcı REDDETTİYSE dosyaya düşülmez, hata döner.
#[derive(Debug)]
pub struct YedekliTasiyici<U: BoruUcu> {
    kutu: GelenKutusu,
    ucu: U,
    zaman_asimi: Duration,
}

impl<U: BoruUcu> YedekliTasiyici<U> {
    pub fn yeni(kutu: GelenKutusu, ucu: U) -> Self {
        YedekliTasiyici {
            kutu,
            ucu,
            zaman_asimi: BORU_ZAMAN_ASIMI,
        }
    }

    /// Alışveriş süresini değiştirir (testler kısa süre kullanır).
    pub fn zaman_asimi(mut self, sure: Duration) -> Self {
        self.zaman_asimi = sure;
        self
    }

    /// Dosya tabanlı kutu (kalıcılık/yedek yol).
    pub fn kutu(&self) -> &GelenKutusu {
        &self.kutu
    }

    /// Hedef programın kayıttaki süreç kimliği (yoksa boru denenmez).
    fn hedef_pid(&self, hedef: &str) -> Option<u32> {
        self.kutu
            .kayit_sahibi(&self.kutu.kayit_yolu(hedef))
            .and_then(|(pid, _)| pid)
    }

    fn dene(&self, hedef: &str, govde: &[u8], id: &str, yetenek: &str) -> Deneme {
        if !boru_icin_uygun(govde.len()) {
            return Deneme::Yedege { belirsiz: false };
        }
        let Some(pid) = self.hedef_pid(hedef) else {
            return Deneme::Yedege { belirsiz: false };
        };
        match self.ucu.degis(hedef, pid, govde, self.zaman_asimi) {
            Ok(onay_govdesi) => match serde_json::from_slice::<Onay>(&onay_govdesi) {
                Ok(o) if o.tamam => Deneme::Teslim,
                Ok(o) => Deneme::Red(o.hataya(id, yetenek)),
                // Onay okunamadı: zarf iletilmiş olabilir.
                Err(_) => Deneme::Yedege { belirsiz: true },
            },
            Err(BoruHatasi::Yok | BoruHatasi::KimlikUyusmadi | BoruHatasi::Dolu) => {
                Deneme::Yedege { belirsiz: false }
            }
            Err(BoruHatasi::ZamanAsimi | BoruHatasi::Io(_)) => Deneme::Yedege { belirsiz: true },
        }
    }
}

impl<U: BoruUcu> Tasiyici for YedekliTasiyici<U> {
    fn kayit_ol(&self, program: &str, yetenekler: &[String]) -> Result<(), IpcHata> {
        program_dogrula(program)?;
        for y in yetenekler {
            yetenek_dogrula("yetenek", y)?;
        }
        match self.ucu.sunucu_baslat(program, &self.kutu) {
            Ok(()) => {}
            // Ad başka süreçte: sahte sunucu ya da ikinci kopya; kaydı ezmeyiz.
            Err(BoruHatasi::Dolu) => return Err(IpcHata::KayitDolu(program.to_string())),
            // Boru kurulamadı: dosya tabanlı yola düşülür, kayıt yine de yapılır.
            Err(e) => self.kutu.gunluk(
                "boru_baslamadi",
                &self.kutu.kayit_yolu(program),
                &e.to_string(),
            ),
        }
        self.kutu.kayit_ol(program, yetenekler)
    }

    fn yetenek_bul(&self, yetenek: &str) -> Result<Option<String>, IpcHata> {
        self.kutu.yetenek_bul(yetenek)
    }

    fn gonder(&self, zarf: &Zarf) -> Result<String, IpcHata> {
        // Dosya yolundaki aynı denetimler (yetenek yok, geçersiz ad, ZarfBuyuk) önce çalışır.
        let (bytes, _) = self.kutu.gonder_hazirla(zarf)?;
        match self.dene(&zarf.hedef, &bytes, &zarf.id, &zarf.yetenek) {
            Deneme::Teslim => Ok(zarf.id.clone()),
            Deneme::Red(h) => Err(h),
            Deneme::Yedege { belirsiz } => match self.kutu.gonder(zarf) {
                Err(IpcHata::IdCakismasi(_)) if belirsiz => Ok(zarf.id.clone()),
                s => s,
            },
        }
    }

    fn al(&self, program: &str) -> Vec<Zarf> {
        self.kutu.al(program)
    }

    fn yanitla(&self, istek: &Zarf, yuk: Value) -> Result<String, IpcHata> {
        let (yanit, bytes) = GelenKutusu::yanit_hazirla(istek, yuk)?;
        let istek_dosyasi = self
            .kutu
            .klasor(&istek.hedef, "gelen")
            .join(format!("{}.json", istek.id));
        match self.dene(&istek.kaynak, &bytes, &istek.id, &istek.yetenek) {
            Deneme::Teslim => {
                self.kutu.isleneni_tasi(&istek.hedef, &istek_dosyasi)?;
                Ok(yanit.id)
            }
            Deneme::Red(h) => Err(h),
            Deneme::Yedege { belirsiz } => match self.kutu.yanitla(istek, yanit.yuk) {
                Err(IpcHata::IdCakismasi(_)) if belirsiz => {
                    // Yanıt boruyla zaten yazılmış; isteği işlenmiş say.
                    self.kutu.isleneni_tasi(&istek.hedef, &istek_dosyasi)?;
                    Ok(yanit.id)
                }
                s => s,
            },
        }
    }

    fn yanit_al(&self, program: &str, istek_id: &str) -> Option<Zarf> {
        self.kutu.yanit_al(program, istek_id)
    }

    fn olay_yayinla(&self, kaynak: &str, ad: &str, yuk: Value) -> Result<String, IpcHata> {
        self.kutu.olay_yayinla(kaynak, ad, yuk)
    }

    fn olaylar(&self, program: &str, son: &str) -> Vec<Zarf> {
        self.kutu.olaylar(program, son)
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use serde_json::json;
    use std::cell::{Cell, RefCell};

    // ----- boru adı, kimlik, ACL -----

    #[test]
    fn boru_adi_bicimi() {
        assert_eq!(
            boru_adi("S-1-5-21-1-2-3-1001", "taarkod").unwrap(),
            r"\\.\pipe\taa-s-1-5-21-1-2-3-1001-taarkod"
        );
        assert_eq!(
            boru_adi("yusuf", "tutaa").unwrap(),
            r"\\.\pipe\taa-yusuf-tutaa"
        );
    }

    #[test]
    fn boru_adi_gecersizleri_reddeder() {
        for program in ["", "..", "a/b", "a\\b", "kayit", "A", "1a", "a.b", "a@1"] {
            assert!(
                matches!(boru_adi("yusuf", program), Err(IpcHata::Gecersiz(_))),
                "{program:?}"
            );
        }
        for kimlik in [
            "",
            " ",
            "a b",
            "a/b",
            "a\\b",
            "a.b",
            "-a",
            "ğ",
            "a\0b",
            &"a".repeat(101),
        ] {
            assert!(
                matches!(boru_adi(kimlik, "tutaa"), Err(IpcHata::Gecersiz(_))),
                "{kimlik:?}"
            );
        }
        // Ad uzunluğu Windows sınırının (256) çok altında kalır.
        let en_uzun = boru_adi(&"a".repeat(100), &"p".repeat(32)).unwrap();
        assert!(en_uzun.len() < 256);
    }

    #[test]
    fn kimlik_kucuk_harfe_cevrilir() {
        assert_eq!(kullanici_kimligi_temizle(" S-1-5-18 ").unwrap(), "s-1-5-18");
        assert!(kullanici_kimligi_temizle("S-1-5_18").is_err());
    }

    #[test]
    fn acl_yalniz_gecerli_kullanici() {
        assert_eq!(
            acl_sddl("S-1-5-21-111-222-333-1001").unwrap(),
            "D:P(A;;GA;;;S-1-5-21-111-222-333-1001)"
        );
        for kotu in [
            "",
            "S-1-",
            "S-1-5-",
            "S-1-5--1",
            "s-1-5-18",
            "WD",
            "S-1-5-18)(A;;GA;;;WD",
            "S-1-5-18;",
            "S-2-5-18",
            &format!("S-1-{}", "1-".repeat(100)),
        ] {
            assert!(
                matches!(acl_sddl(kotu), Err(IpcHata::Gecersiz(_))),
                "{kotu:?}"
            );
        }
    }

    // ----- zaman aşımı -----

    #[test]
    fn sure_kalan() {
        let dolmus = Sure::yeni(Duration::ZERO);
        assert!(dolmus.kalan().is_none());
        assert_eq!(dolmus.kalan_ms(), 0);
        let uzun = Sure::yeni(Duration::from_secs(60));
        assert!(uzun.kalan().is_some());
        let ms = uzun.kalan_ms();
        assert!((1..=60_000).contains(&ms));
        // Çok büyük süre taşmaz ve INFINITE'e (u32::MAX) ulaşmaz.
        assert!(Sure::yeni(Duration::MAX).kalan_ms() < u32::MAX);
    }

    // ----- çerçeveleme -----

    #[test]
    fn cerceve_gidis_donus() {
        let mut hat = Vec::new();
        cerceve_yaz(&mut hat, b"{\"a\":1}").unwrap();
        assert_eq!(&hat[..4], &[0, 0, 0, 7]);
        assert_eq!(cerceve_oku(&mut hat.as_slice()).unwrap(), b"{\"a\":1}");
        // Art arda iki çerçeve ayrı ayrı okunur.
        cerceve_yaz(&mut hat, b"x").unwrap();
        let mut o = hat.as_slice();
        assert_eq!(cerceve_oku(&mut o).unwrap(), b"{\"a\":1}");
        assert_eq!(cerceve_oku(&mut o).unwrap(), b"x");
        assert_eq!(cerceve_oku(&mut o), Err(CerceveHata::Kesik));
    }

    #[test]
    fn cerceve_siniri_256_kib() {
        let tam = vec![b'x'; BORU_CERCEVE_SINIRI];
        let mut hat = Vec::new();
        cerceve_yaz(&mut hat, &tam).unwrap();
        assert_eq!(
            cerceve_oku(&mut hat.as_slice()).unwrap().len(),
            BORU_CERCEVE_SINIRI
        );

        let fazla = vec![b'x'; BORU_CERCEVE_SINIRI + 1];
        let mut hat2 = Vec::new();
        assert_eq!(
            cerceve_yaz(&mut hat2, &fazla),
            Err(CerceveHata::Buyuk {
                boyut: BORU_CERCEVE_SINIRI + 1,
                sinir: BORU_CERCEVE_SINIRI
            })
        );
        assert!(hat2.is_empty(), "sınırı aşan gövde hiç yazılmaz");
        assert!(boru_icin_uygun(BORU_CERCEVE_SINIRI));
        assert!(!boru_icin_uygun(BORU_CERCEVE_SINIRI + 1));
        assert!(!boru_icin_uygun(0));
    }

    #[test]
    fn cerceve_okuma_savunmasi() {
        // Başlık çok büyük uzunluk bildirir: gövde okunmadan, bellek ayrılmadan reddedilir.
        let mut kotu: &[u8] = &[0xFF, 0xFF, 0xFF, 0xFF];
        assert!(matches!(
            cerceve_oku(&mut kotu),
            Err(CerceveHata::Buyuk {
                sinir: BORU_CERCEVE_SINIRI,
                ..
            })
        ));
        // Sınırın bir fazlası.
        let baslik = ((BORU_CERCEVE_SINIRI + 1) as u32).to_be_bytes();
        assert!(matches!(
            cerceve_oku(&mut baslik.as_slice()),
            Err(CerceveHata::Buyuk { .. })
        ));
        // Sıfır uzunluk.
        assert_eq!(
            cerceve_oku(&mut [0u8, 0, 0, 0].as_slice()),
            Err(CerceveHata::Bos)
        );
        assert_eq!(cerceve_yaz(&mut Vec::new(), b""), Err(CerceveHata::Bos));
        // Kesik başlık ve kesik gövde.
        assert_eq!(
            cerceve_oku(&mut [0u8, 0].as_slice()),
            Err(CerceveHata::Kesik)
        );
        assert_eq!(cerceve_oku(&mut [].as_slice()), Err(CerceveHata::Kesik));
        let mut kesik: &[u8] = &[0, 0, 0, 5, b'a', b'b'];
        assert_eq!(cerceve_oku(&mut kesik), Err(CerceveHata::Kesik));
    }

    struct ZamanAsimliOkuyucu;
    impl Read for ZamanAsimliOkuyucu {
        fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::new(io::ErrorKind::TimedOut, "süre doldu"))
        }
    }
    struct ZamanAsimliYazici;
    impl Write for ZamanAsimliYazici {
        fn write(&mut self, _: &[u8]) -> io::Result<usize> {
            Err(io::Error::from(io::ErrorKind::TimedOut))
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn cerceve_zaman_asimi_eslenir() {
        assert_eq!(
            cerceve_oku(&mut ZamanAsimliOkuyucu),
            Err(CerceveHata::ZamanAsimi)
        );
        assert_eq!(
            cerceve_yaz(&mut ZamanAsimliYazici, b"x"),
            Err(CerceveHata::ZamanAsimi)
        );
        assert!(matches!(
            IpcHata::from(CerceveHata::Buyuk { boyut: 9, sinir: 4 }),
            IpcHata::ZarfBuyuk { boyut: 9, sinir: 4 }
        ));
    }

    // ----- sunucu tarafı teslim -----

    struct Kok(std::path::PathBuf);
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
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
    fn sv(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sunucu_istegi_dosyaya_yazar() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        kutu.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        let z = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!({"n": 1}));
        let onay = sunucu_isle(&kutu, "taarkod", &serde_json::to_vec(&z).unwrap());
        assert_eq!(onay, Onay::tamam());
        assert_eq!(kutu.al("taarkod"), vec![z.clone()]);
        // İkinci teslim: kimlik çakışması (ezilmez).
        let onay = sunucu_isle(&kutu, "taarkod", &serde_json::to_vec(&z).unwrap());
        assert!(!onay.tamam);
        assert_eq!(onay.kod.as_deref(), Some("id_cakismasi"));
    }

    #[test]
    fn sunucu_gecersizleri_reddeder() {
        let k = Kok::yeni();
        let kutu = k.kutu();
        kutu.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        let bytes = |z: &Zarf| serde_json::to_vec(z).unwrap();
        // Yanlış hedef.
        let z = Zarf::istek("tutaa", "baska", "kod.uret@1", json!(1));
        assert_eq!(
            sunucu_isle(&kutu, "taarkod", &bytes(&z)).kod.as_deref(),
            Some("gecersiz")
        );
        // Sunulmayan yetenek.
        let z = Zarf::istek("tutaa", "taarkod", "yok.yok@1", json!(1));
        assert_eq!(
            sunucu_isle(&kutu, "taarkod", &bytes(&z)).kod.as_deref(),
            Some("yetenek_yok")
        );
        // Ayrılmış kaynak adı.
        let mut z = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!(1));
        z.kaynak = "kayit".into();
        assert!(!sunucu_isle(&kutu, "taarkod", &bytes(&z)).tamam);
        // Bozuk JSON, olay, yanit_id'siz yanıt.
        assert!(!sunucu_isle(&kutu, "taarkod", b"{ yarim").tamam);
        let mut z = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!(1));
        z.tur = ZarfTuru::Olay;
        assert!(!sunucu_isle(&kutu, "taarkod", &bytes(&z)).tamam);
        let mut z = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!(1));
        z.tur = ZarfTuru::Yanit;
        assert!(!sunucu_isle(&kutu, "taarkod", &bytes(&z)).tamam);
        // Hiçbiri gelen/ altına yazılmadı.
        assert!(kutu.al("taarkod").is_empty());
    }

    // ----- yedekli taşıyıcı (sahte uçla) -----

    #[derive(Clone, Copy, PartialEq, Debug)]
    enum Mod {
        /// Boru çalışır: çerçeve turu + sunucu_isle.
        Calisir,
        Yok,
        KimlikUyusmaz,
        /// Teslim edilir ama istemci süre aşımı görür.
        TeslimAmaZamanAsimi,
        /// Onay çerçevesi bozuk gelir (teslim edilmiştir).
        BozukOnay,
        /// Sunucu başlatılamaz: ad dolu.
        Dolu,
        /// Sunucu başlatılamaz: başka hata.
        BaslatmaHatasi,
    }

    struct Sahte {
        kutu: GelenKutusu,
        mod_: Cell<Mod>,
        cagri: Cell<u32>,
        gorulen_pid: RefCell<Vec<u32>>,
        baslatilan: RefCell<Vec<String>>,
    }

    impl Sahte {
        fn yeni(kutu: GelenKutusu) -> Sahte {
            Sahte {
                kutu,
                mod_: Cell::new(Mod::Calisir),
                cagri: Cell::new(0),
                gorulen_pid: RefCell::new(Vec::new()),
                baslatilan: RefCell::new(Vec::new()),
            }
        }
    }

    impl BoruUcu for &Sahte {
        fn degis(
            &self,
            hedef: &str,
            beklenen_pid: u32,
            istek: &[u8],
            _: Duration,
        ) -> Result<Vec<u8>, BoruHatasi> {
            self.cagri.set(self.cagri.get() + 1);
            self.gorulen_pid.borrow_mut().push(beklenen_pid);
            match self.mod_.get() {
                Mod::Yok => return Err(BoruHatasi::Yok),
                Mod::KimlikUyusmaz => return Err(BoruHatasi::KimlikUyusmadi),
                _ => {}
            }
            // Gerçek hat gibi: çerçeve yaz, çerçeve oku, işle, onayı çerçevele.
            let mut hat = Vec::new();
            cerceve_yaz(&mut hat, istek).map_err(|e| BoruHatasi::Io(e.to_string()))?;
            let alinan =
                cerceve_oku(&mut hat.as_slice()).map_err(|e| BoruHatasi::Io(e.to_string()))?;
            let onay = sunucu_isle(&self.kutu, hedef, &alinan);
            match self.mod_.get() {
                Mod::TeslimAmaZamanAsimi => Err(BoruHatasi::ZamanAsimi),
                Mod::BozukOnay => Ok(b"onay degil".to_vec()),
                _ => Ok(serde_json::to_vec(&onay).unwrap()),
            }
        }

        fn sunucu_baslat(&self, program: &str, _: &GelenKutusu) -> Result<(), BoruHatasi> {
            match self.mod_.get() {
                Mod::Dolu => Err(BoruHatasi::Dolu),
                Mod::BaslatmaHatasi => Err(BoruHatasi::Io("SDDL".into())),
                _ => {
                    self.baslatilan.borrow_mut().push(program.to_string());
                    Ok(())
                }
            }
        }
    }

    fn kurulum(k: &Kok) -> (GelenKutusu, &'static Sahte) {
        let kutu = k.kutu();
        let sahte: &'static Sahte = Box::leak(Box::new(Sahte::yeni(kutu.clone())));
        (kutu, sahte)
    }

    #[test]
    fn boruyla_istek_yanit_dongusu() {
        let k = Kok::yeni();
        let (kutu, sahte) = kurulum(&k);
        let t = YedekliTasiyici::yeni(kutu, sahte);
        t.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        t.kayit_ol("tutaa", &sv(&[])).unwrap();
        assert_eq!(*sahte.baslatilan.borrow(), ["taarkod", "tutaa"]);

        let istek = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!({"urun": 5}));
        assert_eq!(t.gonder(&istek).unwrap(), istek.id);
        assert_eq!(sahte.cagri.get(), 1);
        // Boru, hedefin kayıttaki pid'iyle doğrulandı (burada bu süreç).
        assert_eq!(*sahte.gorulen_pid.borrow(), [std::process::id()]);

        let gelen = t.al("taarkod");
        assert_eq!(gelen, vec![istek.clone()]);
        let yanit_id = t.yanitla(&gelen[0], json!({"kod": "ABC"})).unwrap();
        assert_eq!(sahte.cagri.get(), 2);
        // İstek işlendi; tekrar teslim edilmez.
        assert!(t.al("taarkod").is_empty());
        let yanit = t.yanit_al("tutaa", &istek.id).unwrap();
        assert_eq!(yanit.id, yanit_id);
        assert_eq!(yanit.yuk, json!({"kod": "ABC"}));
        assert!(t.yanit_al("tutaa", &istek.id).is_none());
    }

    #[test]
    fn boru_yoksa_dosyaya_duser() {
        let k = Kok::yeni();
        let (kutu, sahte) = kurulum(&k);
        let t = YedekliTasiyici::yeni(kutu, sahte);
        t.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        for m in [Mod::Yok, Mod::KimlikUyusmaz] {
            sahte.mod_.set(m);
            let istek = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!(1));
            assert_eq!(t.gonder(&istek).unwrap(), istek.id, "{m:?}");
            let gelen = t.al("taarkod");
            assert_eq!(gelen, vec![istek.clone()], "{m:?}");
            // Yanıt da dosya yoluyla gider ve normal doğrulamadan geçer.
            t.yanitla(&gelen[0], json!("tamam")).unwrap();
            assert_eq!(t.yanit_al("tutaa", &istek.id).unwrap().yuk, json!("tamam"));
        }
    }

    #[test]
    fn kayitsiz_hedefe_boru_denenmez() {
        let k = Kok::yeni();
        let (kutu, sahte) = kurulum(&k);
        let t = YedekliTasiyici::yeni(kutu, sahte);
        // Hedef kayıtlı değil: dosya yolundaki aynı hata, boruya hiç dokunulmaz.
        let z = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!(1));
        assert!(matches!(t.gonder(&z), Err(IpcHata::YetenekYok(_))));
        assert_eq!(sahte.cagri.get(), 0);
    }

    #[test]
    fn buyuk_zarf_dosyaya_duser_cok_buyuk_reddedilir() {
        let k = Kok::yeni();
        let (kutu, sahte) = kurulum(&k);
        let t = YedekliTasiyici::yeni(kutu, sahte);
        t.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        // 300 KiB: boru sınırının üstünde, zarf sınırının altında -> dosya.
        let z = Zarf::istek(
            "tutaa",
            "taarkod",
            "kod.uret@1",
            json!("x".repeat(300 * 1024)),
        );
        assert_eq!(t.gonder(&z).unwrap(), z.id);
        assert_eq!(sahte.cagri.get(), 0);
        assert_eq!(t.al("taarkod").len(), 1);
        // 2 MiB: ZarfBuyuk, hiçbir yere yazılmaz.
        let z = Zarf::istek(
            "tutaa",
            "taarkod",
            "kod.uret@1",
            json!("x".repeat(2 * 1024 * 1024)),
        );
        assert!(matches!(t.gonder(&z), Err(IpcHata::ZarfBuyuk { .. })));
        assert_eq!(sahte.cagri.get(), 0);
    }

    #[test]
    fn zaman_asiminda_teslim_edildiyse_cift_olmaz() {
        let k = Kok::yeni();
        let (kutu, sahte) = kurulum(&k);
        let t = YedekliTasiyici::yeni(kutu, sahte);
        t.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        for m in [Mod::TeslimAmaZamanAsimi, Mod::BozukOnay] {
            sahte.mod_.set(m);
            let istek = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!(1));
            // Boru teslim etti ama onay alınamadı; dosya yolu çakışır ve başarı sayılır.
            assert_eq!(t.gonder(&istek).unwrap(), istek.id, "{m:?}");
            let gelen: Vec<Zarf> = t.al("taarkod");
            assert_eq!(
                gelen.iter().filter(|z| z.id == istek.id).count(),
                1,
                "{m:?}"
            );
            // Yanıt için de aynısı.
            let g = gelen.into_iter().find(|z| z.id == istek.id).unwrap();
            t.yanitla(&g, json!("ok")).unwrap();
            assert!(t.al("taarkod").is_empty(), "{m:?}");
            assert_eq!(t.yanit_al("tutaa", &istek.id).unwrap().yuk, json!("ok"));
        }
    }

    #[test]
    fn alici_reddederse_dosyaya_dusulmez() {
        let k = Kok::yeni();
        let (kutu, sahte) = kurulum(&k);
        let t = YedekliTasiyici::yeni(kutu, sahte);
        t.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        let istek = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!(1));
        t.gonder(&istek).unwrap();
        // Aynı kimlikle ikinci gönderim: alıcı IdCakismasi ile reddeder, hata döner, dosya ezilmez.
        assert!(matches!(t.gonder(&istek), Err(IpcHata::IdCakismasi(_))));
        assert_eq!(t.al("taarkod").len(), 1);
    }

    #[test]
    fn kayit_ol_dolu_ad_ve_baslatma_hatasi() {
        let k = Kok::yeni();
        let (kutu, sahte) = kurulum(&k);
        let t = YedekliTasiyici::yeni(kutu, sahte);
        // Boru adı başka süreçte: KayitDolu, kayıt yazılmaz.
        sahte.mod_.set(Mod::Dolu);
        assert!(matches!(
            t.kayit_ol("taarkod", &sv(&["kod.uret@1"])),
            Err(IpcHata::KayitDolu(_))
        ));
        assert_eq!(t.yetenek_bul("kod.uret@1").unwrap(), None);
        // Başlatma başka nedenle başarısız: dosya tabanlı yola düşülür, kayıt yapılır, günlüğe yazılır.
        sahte.mod_.set(Mod::BaslatmaHatasi);
        t.kayit_ol("taarkod", &sv(&["kod.uret@1"])).unwrap();
        assert_eq!(
            t.yetenek_bul("kod.uret@1").unwrap().as_deref(),
            Some("taarkod")
        );
        let gunluk = taa_dosya::satirlari_oku(&k.0.join("gunluk.jsonl")).unwrap();
        assert!(gunluk.iter().any(|s| s.contains("boru_baslamadi")));
        // Geçersiz ad/yetenek sunucu başlatılmadan reddedilir.
        sahte.mod_.set(Mod::Calisir);
        assert!(t.kayit_ol("kayit", &[]).is_err());
        assert!(t.kayit_ol("tutaa", &sv(&["kotu"])).is_err());
        assert!(sahte.baslatilan.borrow().is_empty());
    }

    #[test]
    fn olaylar_dosyada_yazilir() {
        let k = Kok::yeni();
        let (kutu, sahte) = kurulum(&k);
        let t = YedekliTasiyici::yeni(kutu, sahte);
        let id = t
            .olay_yayinla("tutaa", "veri.degisti@1", json!({"alan": "urunler"}))
            .unwrap();
        // Boru kullanılmaz; olay günlükte durur ve aboneye gelir.
        assert_eq!(sahte.cagri.get(), 0);
        let olaylar = t.olaylar("abone", "");
        assert_eq!(olaylar.len(), 1);
        assert_eq!(olaylar[0].id, id);
        assert!(k.0.join("olaylar").is_dir());
    }
}
