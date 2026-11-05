// Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05]
//! taa-kur: TAA kurulum yöneticisi (kütüphane API'si; CLI ayrı işte eklenir).
//! Kök: `TAA_KOK` → kayit.json içindeki `kok` → varsayılan. Tüm yazımlar atomiktir (`taa_dosya::uzerine_yaz_atomik`).
//! Süreçler arası kilit: `<kok>/kilit` (create_new); değiştiren işlemler kilit altında çalışır, iş bitince kilit
//! `kilit.serbest` adına çevrilir; 10 dakikadan eski kilit bayat sayılıp `eski/kilit-<zaman>` altına taşınır.
//! Hiçbir dosya silinmez: kaldırılan ya da üzerine yazılan içerik `eski/` altına taşınır.
//! Veri alanlarına (`%APPDATA%\TAA\veri`) bu crate hiçbir zaman dokunmaz.
//! Özellik anahtarı: "kur" (varsayılan açık).

pub mod ayar;
pub mod kayit;
pub mod paket;
pub mod pencere_model;
mod veri;
#[cfg(windows)]
pub mod win;
mod zaman;
pub mod zipac;

#[cfg(test)]
mod testler;
#[cfg(test)]
mod testler_imza;
#[cfg(test)]
mod testler_zip;

pub use ayar::{kur_ayari_oku, KurAyar};
pub use kayit::{Kayit, PaketKaydi};
pub use paket::{paket_dogrula, Paket};
pub use veri::{sahip_toml_coz, veri_kok_coz, Deger};

use kayit::surum_karsilastir;
use paket::{ad_gecerli, guvenli_coz, icerik_hesapla, paket_oku, sha256_dosya, surum_gecerli};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use taa_imza::{GenelAnahtar, Imza};

pub const OZELLIK: &str = "kur";

/// Bu süreden (saniye) eski kilit bayat sayılır.
const KILIT_BAYAT_SN: i64 = 600;

/// Kilit tutulduğu sürece yaşar; düşünce `kilit` → `kilit.serbest` (rename; var olanı üzerine yazar, silme yok).
struct KilitTutacagi {
    yol: PathBuf,
    serbest: PathBuf,
}

impl Drop for KilitTutacagi {
    fn drop(&mut self) {
        let _ = fs::rename(&self.yol, &self.serbest);
    }
}

// Testlerde `rename` başarısızlığını (sürücüler arası taşıma) taklit eder; yalnız o testin iş parçacığını etkiler.
#[cfg(test)]
thread_local! {
    pub(crate) static RENAME_HATA: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn rename_dene(kaynak: &Path, hedef: &Path) -> std::io::Result<()> {
    #[cfg(test)]
    if RENAME_HATA.with(|c| c.get()) {
        return Err(std::io::Error::other("test: rename başarısız"));
    }
    fs::rename(kaynak, hedef)
}

// ---------------------------------------------------------------- hata

/// Hata türleri CLI çıkış kodlarına karşılık gelir: 2 doğrulama, 3 çözüm, 4 yok, 5 bozuk, 6 kilitli, 1 diğer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KurHata {
    Dogrulama(String),
    Cozum(String),
    Yok(String),
    Bozuk(String),
    Diger(String),
    /// Başka bir taa-kur işlemi kök kilidini tutuyor (çıkış 6).
    Kilitli(String),
}

impl KurHata {
    pub fn cikis_kodu(&self) -> i32 {
        match self {
            KurHata::Dogrulama(_) => 2,
            KurHata::Cozum(_) => 3,
            KurHata::Yok(_) => 4,
            KurHata::Bozuk(_) => 5,
            KurHata::Diger(_) => 1,
            KurHata::Kilitli(_) => 6,
        }
    }

    pub fn mesaj(&self) -> &str {
        match self {
            KurHata::Dogrulama(m)
            | KurHata::Cozum(m)
            | KurHata::Yok(m)
            | KurHata::Bozuk(m)
            | KurHata::Diger(m)
            | KurHata::Kilitli(m) => m,
        }
    }
}

impl std::fmt::Display for KurHata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.mesaj())
    }
}

impl std::error::Error for KurHata {}

impl From<std::io::Error> for KurHata {
    fn from(e: std::io::Error) -> Self {
        KurHata::Diger(format!(
            "Dosya işlemi başarısız: {e}. Öneri: kök klasörün yazılabilir olduğunu denetleyin."
        ))
    }
}

type Sonuc<T> = Result<T, KurHata>;

// ---------------------------------------------------------------- sonuç türleri

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum Sorun {
    Eksik(String),
    Bozuk(String),
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KurSonuc {
    pub ad: String,
    pub surum: String,
    /// Bu kurulumla sürüm seçili oldu mu.
    pub secili: bool,
    /// Üzerine yazılan önceki içeriğin taşındığı yer.
    pub eski: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KaldirSonuc {
    pub ad: String,
    pub surum: String,
    pub eski: PathBuf,
    pub yeni_secili: Option<String>,
    /// Paket tamamen kalktığı için veri alanları salt okunur olacak paketler.
    pub salt_okunur: Vec<String>,
    pub uyarilar: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ListeSatiri {
    pub ad: String,
    pub kurulu: Vec<String>,
    pub secili: Option<String>,
    pub sabitle: bool,
    pub onceki: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum KokKaynagi {
    Verilen,
    Ortam,
    Kayit,
    Varsayilan,
}

#[derive(Debug, Clone, Serialize)]
pub struct Tani {
    pub kok: String,
    pub kok_kaynagi: KokKaynagi,
    pub kayit_saglam: bool,
    pub paketler: Vec<ListeSatiri>,
    pub sahiplik: BTreeMap<String, String>,
    /// Veri kökü (`TAA_VERI_KOK` ya da platform varsayılanı); çözülemezse yok.
    pub veri_kok: Option<String>,
    /// Veri kökündeki her `<alan>/SAHIP.toml` için özet (T2-TASARIM §1).
    pub veri_alanlari: Vec<VeriAlani>,
    pub sorunlar: Vec<Sorun>,
    /// Sorun sayılmayan ama bilinmesi gerekenler (ör. güvenilir anahtar yok: imzalar denetlenmiyor).
    pub uyarilar: Vec<String>,
    /// `<kök>\anahtarlar\*.pub` içindeki güvenilir anahtarların kimlikleri.
    pub guvenilir_anahtarlar: Vec<String>,
    pub son_hatalar: Vec<String>,
}

/// İmza denetiminin sonucu.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum ImzaDurumu {
    /// Güvenilir bir anahtarla doğrulandı (kimlik).
    Dogrulandi(String),
    /// Güvenilir anahtar tanımlı olmadığından denetlenmedi (eski davranış).
    Denetlenmedi,
}

/// `taa-kur dogrula <paket.zip|klasör>` sonucu: paket kurulmadan doğrulandı.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PaketDenetim {
    pub ad: String,
    pub surum: String,
    pub dosya_sayisi: usize,
    pub icerik_sha256: String,
    pub imza: ImzaDurumu,
}

/// `taa-kur tani --json` veri alanı satırı.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct VeriAlani {
    pub alan: String,
    pub sahip: String,
    pub gecici: bool,
    /// `kayit.json` paketler içinde `sahip` adı var mı.
    pub sahip_kurulu: bool,
}

#[derive(Serialize, Deserialize)]
struct SeciliKaydi {
    ad: String,
    surum: String,
    sha256: String,
    tarih: String,
}

// ---------------------------------------------------------------- kök çözümü

/// Varsayılan kök: Windows `D:\Programlar\TAA`, diğerlerinde `~/.local/share/TAA`.
pub fn varsayilan_kok() -> Sonuc<PathBuf> {
    if cfg!(windows) {
        return Ok(PathBuf::from(r"D:\Programlar\TAA"));
    }
    match std::env::var_os("HOME") {
        Some(h) if !h.is_empty() => Ok(PathBuf::from(h).join(".local/share/TAA")),
        _ => Err(KurHata::Diger(
            "Kök bulunamadı: HOME tanımsız. Öneri: TAA_KOK ortam değişkenini ayarlayın.".into(),
        )),
    }
}

/// Saf kök çözümü (test edilebilir): ortam değeri → varsayılan kökteki kayit.json içindeki `kok` → varsayılan.
pub fn kok_coz_ile(ortam: Option<&str>, varsayilan: &Path) -> (PathBuf, KokKaynagi) {
    if let Some(o) = ortam.filter(|o| !o.trim().is_empty()) {
        return (PathBuf::from(o), KokKaynagi::Ortam);
    }
    if let Ok(b) = fs::read(varsayilan.join("kayit.json")) {
        if let Ok(k) = serde_json::from_slice::<serde_json::Value>(&b) {
            if let Some(s) = k
                .get("kok")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
            {
                return (PathBuf::from(s), KokKaynagi::Kayit);
            }
        }
    }
    (varsayilan.to_path_buf(), KokKaynagi::Varsayilan)
}

fn sayac() -> u64 {
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

// ---------------------------------------------------------------- Kur

#[derive(Debug, Clone)]
pub struct Kur {
    kok: PathBuf,
    kaynak: KokKaynagi,
    /// `<kök>/anahtarlar/*.pub`; okunamayan/bozuk anahtar dosyası sessizce yok sayılmaz (yoksa imza denetimi kapatılabilirdi).
    anahtarlar: Result<Vec<GenelAnahtar>, String>,
}

impl Kur {
    /// Verilen kökle çalışır (testler geçici kök verir).
    pub fn yeni(kok: impl Into<PathBuf>) -> Sonuc<Kur> {
        kapi()?;
        let kok = kok.into();
        let anahtarlar = anahtarlari_yukle(&kok);
        Ok(Kur {
            kok,
            kaynak: KokKaynagi::Verilen,
            anahtarlar,
        })
    }

    /// Kökü TAA_KOK → kayit.json → varsayılan sırasıyla çözer.
    pub fn ortamdan() -> Sonuc<Kur> {
        kapi()?;
        let ortam = std::env::var("TAA_KOK").ok();
        let (kok, kaynak) = kok_coz_ile(ortam.as_deref(), &varsayilan_kok()?);
        let anahtarlar = anahtarlari_yukle(&kok);
        Ok(Kur {
            kok,
            kaynak,
            anahtarlar,
        })
    }

    pub fn kok(&self) -> &Path {
        &self.kok
    }

    /// Güvenilir anahtarlar (`<kök>\anahtarlar\*.pub`). Dosya bozuksa hata: imza denetimi sessizce kapanmaz.
    pub fn guvenilir_anahtarlar(&self) -> Sonuc<&[GenelAnahtar]> {
        self.anahtarlar.as_deref().map_err(|m| {
            KurHata::Bozuk(format!(
                "Güvenilir anahtar dosyası okunamadı: {m}. Öneri: anahtarlar klasöründeki bozuk .pub dosyasını düzeltin ya da eski/ altına taşıyın."
            ))
        })
    }

    /// İmza kuralı (T3): güvenilir anahtar varsa `imza` ZORUNLU ve anahtarlardan biriyle `veri` üzerinde doğrulanmalıdır;
    /// hiç anahtar yoksa denetlenmez (eski davranış). `ne` iletide kullanılır ("Paket", "Yayın dosyası").
    pub fn imza_denetle(
        &self,
        ne: &str,
        veri: &[u8],
        imza: Option<&serde_json::Value>,
    ) -> Sonuc<ImzaDurumu> {
        let anahtarlar = self.guvenilir_anahtarlar()?;
        if anahtarlar.is_empty() {
            return Ok(ImzaDurumu::Denetlenmedi);
        }
        let ret = |m: String| Err(KurHata::Dogrulama(m));
        let Some(raw) = imza.filter(|v| !v.is_null()) else {
            return ret(format!("{ne} imzasız ama güvenilir anahtar tanımlı olduğundan reddedildi; yayıncıdan imzalı paket isteyin."));
        };
        let Ok(im) = serde_json::from_value::<Imza>(raw.clone()) else {
            return ret(format!("{ne} imza alanı okunamadı (alg, anahtar_kimlik ve deger bekleniyor); paketi yeniden imzalatın."));
        };
        if im.alg != taa_imza::ALGORITMA {
            return ret(format!(
                "{ne} imza algoritması desteklenmiyor ({:?}); yalnız ed25519 kabul edilir.",
                im.alg
            ));
        }
        let Some(k) = anahtarlar.iter().find(|k| k.kimlik() == im.anahtar_kimlik) else {
            return ret(format!("{ne} güvenilir anahtarlar arasında olmayan bir anahtarla (kimlik {}) imzalanmış; yayıncının .pub dosyasını anahtarlar klasörüne koymadan kabul edilmez.", im.anahtar_kimlik.chars().take(32).collect::<String>()));
        };
        match k.dogrula(veri, &im) {
            Ok(()) => Ok(ImzaDurumu::Dogrulandi(k.kimlik())),
            Err(_) => ret(format!(
                "{ne} imzası geçersiz: içerik imzayla uyuşmuyor, dosya değiştirilmiş olabilir."
            )),
        }
    }

    fn kayit_yolu(&self) -> PathBuf {
        self.kok.join("kayit.json")
    }

    fn depo(&self, ad: &str, surum: &str) -> PathBuf {
        self.kok.join("depo").join(ad).join(surum)
    }

    fn secili_yolu(&self, ad: &str) -> PathBuf {
        self.kok.join("programlar").join(ad).join("secili.json")
    }

    fn gunluk_yolu(&self) -> PathBuf {
        self.kok.join("gunluk.jsonl")
    }

    // ---- kayıt okuma / yazma

    /// kayit.json'u okur; iç özet (`ozet`) sorunu varsa ayrıca bildirir (ayrıştırılabiliyorsa kayıt yine döner).
    /// Eski `kayit.json.sha256` dosyası artık okunmaz.
    fn kayit_oku_ham(&self) -> Sonuc<(Kayit, Option<Sorun>)> {
        let yol = self.kayit_yolu();
        let bayt = match fs::read(&yol) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Ok((Kayit::bos(&self.kok.to_string_lossy()), None));
            }
            Err(e) => return Err(e.into()),
        };
        let kayit: Kayit = serde_json::from_slice(&bayt).map_err(|e| {
            KurHata::Bozuk(format!(
                "kayit.json ayrıştırılamadı: {e}. Öneri: yedekten geri yükleyin; dosya elle değiştirilmemeli."
            ))
        })?;
        let sorun = if kayit.ozet.is_empty() {
            Some(Sorun::Eksik("kayit.json içinde ozet alanı yok".into()))
        } else if kayit.ozet != kayit.ozet_hesapla() {
            Some(Sorun::Bozuk(
                "kayit.json iç özeti (ozet) içerikle uyuşmuyor".into(),
            ))
        } else {
            None
        };
        Ok((kayit, sorun))
    }

    fn kayit_yukle(&self) -> Sonuc<Kayit> {
        let (k, sorun) = self.kayit_oku_ham()?;
        match sorun {
            None => Ok(k),
            Some(s) => Err(KurHata::Bozuk(format!(
                "Kayıt bütünlüğü bozuk ({}). Hiçbir şey değiştirilmedi. Öneri: `taa-kur dogrula` çalıştırın, kayıt dosyasını yedekten geri yükleyin.",
                sorun_metni(&s)
            ))),
        }
    }

    /// Özeti içine koyarak tek dosya olarak atomik yazar.
    fn kayit_kaydet(&self, k: &Kayit) -> Sonuc<()> {
        let mut k = k.clone();
        k.ozet = k.ozet_hesapla();
        let bayt = serde_json::to_vec_pretty(&k)
            .map_err(|e| KurHata::Diger(format!("kayıt yazılamadı: {e}")))?;
        taa_dosya::uzerine_yaz_atomik(&self.kayit_yolu(), &bayt)?;
        Ok(())
    }

    // ---- süreçler arası kilit

    /// `<kok>/kilit` dosyasını create_new ile alır (içine pid ve zaman yazar). Bayat kilit (>10 dk) `eski/kilit-<zaman>`
    /// altına taşınıp yeniden denenir. Alınamazsa `KurHata::Kilitli`.
    fn kilit_al(&self) -> Sonuc<KilitTutacagi> {
        fs::create_dir_all(&self.kok)?;
        let yol = self.kok.join("kilit");
        for _ in 0..3 {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&yol)
            {
                Ok(mut f) => {
                    let t = KilitTutacagi {
                        yol: yol.clone(),
                        serbest: self.kok.join("kilit.serbest"),
                    };
                    let icerik = format!(
                        "pid={}\nzaman={}\n",
                        std::process::id(),
                        zaman::unix_saniye()
                    );
                    f.write_all(icerik.as_bytes())?; // hata olursa t düşer, kilit serbest bırakılır
                    return Ok(t);
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    if !self.bayat_kilit_tasi(&yol)? {
                        return Err(KurHata::Kilitli(
                            "Başka bir taa-kur işlemi çalışıyor (kök kilitli). Öneri: bitmesini bekleyip yeniden deneyin; 10 dakikadan eski kilit kendiliğinden bayat sayılır.".into(),
                        ));
                    }
                }
                Err(e) => return Err(e.into()),
            }
        }
        Err(KurHata::Kilitli(
            "Kök kilidi alınamadı (eşzamanlı işlem). Öneri: yeniden deneyin.".into(),
        ))
    }

    /// Kilit bayatsa `eski/kilit-<zaman>` altına taşır ve true döner (yeniden denenebilir); taze ise false.
    fn bayat_kilit_tasi(&self, yol: &Path) -> Sonuc<bool> {
        let icerik = match fs::read_to_string(yol) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(true),
            Err(e) => return Err(e.into()),
        };
        let yazilan = icerik
            .lines()
            .find_map(|s| s.strip_prefix("zaman="))
            .and_then(|s| s.trim().parse::<i64>().ok());
        let zaman_sn = yazilan.or_else(|| {
            fs::metadata(yol)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs() as i64)
        });
        let bayat = zaman_sn.is_some_and(|z| zaman::unix_saniye() - z > KILIT_BAYAT_SN);
        if !bayat {
            return Ok(false);
        }
        let hedef = self.eski_hedef("kilit", "")?;
        match fs::rename(yol, &hedef) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(true),
            Err(e) => return Err(e.into()),
        }
        // Başka süreç araya girip taze kilit aldıysa onu taşımış olabiliriz: içerik farklıysa geri koymayı dene.
        if fs::read_to_string(&hedef).ok().as_deref() != Some(icerik.as_str()) {
            if !yol.exists() {
                let _ = fs::rename(&hedef, yol);
            }
            return Ok(false);
        }
        Ok(true)
    }

    /// Kilit altında çalıştırır; iş bitince (hata dahil) kilit `kilit.serbest` adına çevrilir.
    fn kilitli<T>(&self, f: impl FnOnce() -> Sonuc<T>) -> Sonuc<T> {
        let _kilit = self.kilit_al()?;
        f()
    }

    fn secili_yaz(&self, ad: &str, surum: &str, icerik_sha: &str) -> Sonuc<()> {
        let s = SeciliKaydi {
            ad: ad.into(),
            surum: surum.into(),
            sha256: icerik_sha.into(),
            tarih: zaman::simdi().0,
        };
        let bayt = serde_json::to_vec_pretty(&s)
            .map_err(|e| KurHata::Diger(format!("secili.json yazılamadı: {e}")))?;
        taa_dosya::uzerine_yaz_atomik(&self.secili_yolu(ad), &bayt)?;
        Ok(())
    }

    /// Depodaki kurulu sürümün paket.json'unu okur (içerik doğrulaması `dogrula` işidir).
    fn depo_paketi(&self, ad: &str, surum: &str) -> Sonuc<Paket> {
        let klasor = self.depo(ad, surum);
        paket_oku(&klasor).map(|(p, _)| p).map_err(|_| {
            KurHata::Bozuk(format!(
                "{ad} {surum} depoda okunamadı. Öneri: `taa-kur dogrula {ad}` çalıştırın, gerekirse paketi yeniden kurun."
            ))
        })
    }

    // ---- günlük

    fn gunluk(&self, seviye: &str, olay: &str, ad: &str, surum: &str, mesaj: &str) {
        if !self.kok.is_dir() {
            return;
        }
        let satir = serde_json::json!({
            "zaman": zaman::simdi().0, "seviye": seviye, "olay": olay,
            "ad": ad, "surum": surum, "mesaj": mesaj,
        });
        let _ = taa_dosya::satir_ekle(&self.gunluk_yolu(), &satir.to_string());
    }

    fn gunluklu<T>(&self, olay: &str, ad: &str, surum: &str, s: Sonuc<T>) -> Sonuc<T> {
        match &s {
            Ok(_) => self.gunluk("bilgi", olay, ad, surum, "tamam"),
            Err(e) => self.gunluk("hata", olay, ad, surum, e.mesaj()),
        }
        s
    }

    // ---- çözücü

    /// `gerektirir` her modül için: seçili sürüm var mı, arayüz eşit mi; ince çizgi kuralı.
    fn cozucu(&self, kayit: &Kayit, p: &Paket) -> Sonuc<()> {
        for (gad, garayuz) in &p.gerektirir {
            let secili = kayit.paketler.get(gad).and_then(|k| k.secili.clone());
            let Some(surum) = secili else {
                let kurulu = kayit
                    .paketler
                    .get(gad)
                    .is_some_and(|k| !k.kurulu.is_empty());
                return Err(KurHata::Cozum(if kurulu {
                    format!("{gad} (arayüz {garayuz}) kurulu ama seçili sürüm yok. Öneri: `taa-kur sec {gad} <surum>` çalıştırın.")
                } else {
                    format!("{gad} (arayüz {garayuz}) gerekli ama kurulu değil. Önce `taa-kur kur {gad}` çalıştırın.")
                }));
            };
            let dep = self.depo_paketi(gad, &surum)?;
            if &dep.arayuz != garayuz {
                return Err(KurHata::Cozum(format!(
                    "{gad} seçili sürümü {surum} arayüz {} sunuyor ama {} arayüz {garayuz} gerekli. Önce uyumlu bir `{gad}` sürümü kurun (`taa-kur kur {gad}`).",
                    dep.arayuz, p.ad
                )));
            }
            if dep.program != "ortak" && dep.program != p.program {
                return Err(KurHata::Cozum(format!(
                    "{} (program {}) başka programın paketine zorunlu bağlanamaz: {gad} (program {}). Öneri: yetenek (ister) ya da ortak paket kullanın.",
                    p.ad, p.program, dep.program
                )));
            }
        }
        Ok(())
    }

    /// `p` seçili olursa, onu gerektiren diğer seçili paketler bozulmasın.
    fn bagimlilar_uyumlu(&self, kayit: &Kayit, p: &Paket) -> Sonuc<()> {
        for (ad, k) in &kayit.paketler {
            if ad == &p.ad {
                continue;
            }
            let Some(s) = &k.secili else { continue };
            let Ok(dp) = self.depo_paketi(ad, s) else {
                continue;
            };
            if let Some(a) = dp.gerektirir.get(&p.ad) {
                if a != &p.arayuz {
                    return Err(KurHata::Cozum(format!(
                        "{ad} {s}, {} için arayüz {a} istiyor ama {} {} arayüz {} sunuyor. Öneri: önce {ad} paketini güncelleyin.",
                        p.ad, p.ad, p.surum, p.arayuz
                    )));
                }
            }
        }
        Ok(())
    }

    // ---- eski/ taşıma (silme yok)

    fn eski_hedef(&self, etiket: &str, uzanti: &str) -> Sonuc<PathBuf> {
        let dizin = self.kok.join("eski");
        fs::create_dir_all(&dizin)?;
        let damga = zaman::simdi().1;
        let temel = format!("{etiket}-{damga}");
        let mut aday = dizin.join(format!("{temel}{uzanti}"));
        let mut n = 2;
        while aday.exists() {
            aday = dizin.join(format!("{temel}-{n}{uzanti}"));
            n += 1;
        }
        Ok(aday)
    }

    /// Kaynağı `eski/<etiket>-<damga>` altına taşır. Aynı sürücüde `rename`; olmazsa kopyalar ve kaynağı yerinde
    /// bırakır (ASLA silmez). Dönüş: (hedef, kaynak yerinden kalktı mı).
    fn eskiye_tasi(&self, kaynak: &Path, etiket: &str) -> Sonuc<(PathBuf, bool)> {
        self.eskiye_tasi_sinirli(kaynak, etiket, paket::MAKS_TOPLAM_BAYT)
    }

    /// `eskiye_tasi` ile aynı; kopya yoluna düşülürse toplam boyut `sinir` baytı aşınca durur, yarım klasörü
    /// `eski/yarim-<etiket>-<damga>` altına taşır ve hata döner.
    fn eskiye_tasi_sinirli(
        &self,
        kaynak: &Path,
        etiket: &str,
        sinir: u64,
    ) -> Sonuc<(PathBuf, bool)> {
        let hedef = self.eski_hedef(etiket, "")?;
        if rename_dene(kaynak, &hedef).is_ok() {
            return Ok((hedef, true));
        }
        if kaynak.is_dir() {
            if let Err(e) = agac_kopyala(kaynak, &hedef, sinir) {
                let yarim = self.eski_hedef(&format!("yarim-{etiket}"), "")?;
                return Err(match fs::rename(&hedef, &yarim) {
                    Ok(()) => KurHata::Diger(format!(
                        "{} Yarım kopya {} altına taşındı; kaynak yerinde duruyor.",
                        e.mesaj(),
                        yarim.display()
                    )),
                    Err(_) => KurHata::Diger(format!(
                        "{} Yarım kopya {} içinde kaldı; kaynak yerinde duruyor.",
                        e.mesaj(),
                        hedef.display()
                    )),
                });
            }
        } else {
            fs::copy(kaynak, &hedef)?;
        }
        Ok((hedef, false))
    }

    // ---- işlevler

    /// Paket klasörünü (ya da `.zip` ile biten dosyayı) doğrular, `depo/<ad>/<surum>` altına kopyalar, kaydı günceller.
    /// Zip önce `<kok>/gecici-<pid>/` altına açılır (sınırlar: `zipac`), sonra klasör kurulumu çağrılır; geçici
    /// klasör iş bitince (hata dahil) `eski/` altına taşınır, silinmez.
    pub fn kur(&self, paket_yolu: &Path, sec: bool) -> Sonuc<KurSonuc> {
        if zip_mi(paket_yolu) {
            return self.kur_zip(paket_yolu, sec);
        }
        self.kur_klasor(paket_yolu, sec)
    }

    fn kur_zip(&self, zip: &Path, sec: bool) -> Sonuc<KurSonuc> {
        fs::create_dir_all(&self.kok)?;
        let pid = std::process::id();
        // Geçici klasör `create_dir` ile alınır (varsa sonraki ad): boş olduğu kesin, yarış yok.
        let mut gecici = self.kok.join(format!("gecici-{pid}"));
        let mut n = 2;
        loop {
            match fs::create_dir(&gecici) {
                Ok(()) => break,
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    gecici = self.kok.join(format!("gecici-{pid}-{n}"));
                    n += 1;
                }
                Err(e) => return Err(e.into()),
            }
        }
        let r = match zipac::zip_ac(zip, &gecici) {
            Ok(()) => self.kur_klasor(&gecici, sec),
            Err(e) => {
                self.gunluk("hata", "kur", "", "", e.mesaj());
                Err(e)
            }
        };
        if gecici.exists() {
            if let Err(e) = self.eskiye_tasi(&gecici, &format!("gecici-{pid}")) {
                self.gunluk("hata", "kur", "", "", e.mesaj());
            }
        }
        r
    }

    fn kur_klasor(&self, paket_klasoru: &Path, sec: bool) -> Sonuc<KurSonuc> {
        let r = self.kilitli(|| self.kur_ic(paket_klasoru, sec));
        let (ad, surum) = r
            .as_ref()
            .map(|s| (s.ad.clone(), s.surum.clone()))
            .unwrap_or_default();
        self.gunluklu("kur", &ad, &surum, r)
    }

    fn kur_ic(&self, klasor: &Path, sec: bool) -> Sonuc<KurSonuc> {
        let (paket, paket_bayt) = paket_dogrula(klasor)?;
        self.imza_denetle("Paket", paket.icerik_sha256.as_bytes(), paket.imza.as_ref())?;
        let mut kayit = self.kayit_yukle()?;
        for alan in &paket.veri_sahibi {
            if let Some(s) = kayit.veri_sahipleri.get(alan) {
                if s != &paket.ad {
                    return Err(KurHata::Cozum(format!(
                        "Veri alanı '{alan}' zaten {s} paketine ait. Öneri: veri_sahibi alanını değiştirin."
                    )));
                }
            }
        }
        self.cozucu(&kayit, &paket)?;
        let mevcut = kayit.paketler.get(&paket.ad).cloned().unwrap_or_default();
        let secilecek = sec || mevcut.secili.is_none();
        if secilecek || mevcut.secili.as_deref() == Some(paket.surum.as_str()) {
            self.bagimlilar_uyumlu(&kayit, &paket)?;
        }

        // Hazırlık klasörüne kopyala; hata olursa yarım içerik eski/ altına taşınır (silinmez).
        let depo_ad = self.kok.join("depo").join(&paket.ad);
        fs::create_dir_all(&depo_ad)?;
        let hazirlik = depo_ad.join(format!(".kur-{}-{}", std::process::id(), sayac()));
        if let Err(e) = kopyala(klasor, &paket, &paket_bayt, &hazirlik) {
            let _ = self.eskiye_tasi(&hazirlik, &format!("yarim-{}-{}", paket.ad, paket.surum));
            return Err(e);
        }
        let hedef = self.depo(&paket.ad, &paket.surum);
        let mut eski = None;
        if hedef.exists() {
            let (y, tamam) = self.eskiye_tasi(&hedef, &format!("{}-{}", paket.ad, paket.surum))?;
            if !tamam {
                let _ = self.eskiye_tasi(&hazirlik, &format!("yarim-{}-{}", paket.ad, paket.surum));
                return Err(KurHata::Diger(format!(
                    "Var olan sürüm eski/ altına taşınamadı (kopyası {} içinde). Hiçbir şey silinmedi. Öneri: kök klasörün izinlerini denetleyin.",
                    y.display()
                )));
            }
            eski = Some(y);
        }
        if let Err(e) = fs::rename(&hazirlik, &hedef) {
            if let Some(y) = &eski {
                let _ = fs::rename(y, &hedef); // eski içeriği yerine döndür
            }
            return Err(e.into());
        }

        let pk = kayit.paketler.entry(paket.ad.clone()).or_default();
        if !pk.kurulu.contains(&paket.surum) {
            pk.kurulu.push(paket.surum.clone());
            pk.kurulu.sort_by(|a, b| surum_karsilastir(a, b));
        }
        let mut secildi = false;
        if secilecek && pk.secili.as_deref() != Some(paket.surum.as_str()) {
            pk.onceki = pk.secili.take();
            pk.secili = Some(paket.surum.clone());
        }
        if pk.secili.as_deref() == Some(paket.surum.as_str()) {
            secildi = true;
        }
        for alan in &paket.veri_sahibi {
            kayit.veri_sahipleri.insert(alan.clone(), paket.ad.clone());
        }
        self.kayit_kaydet(&kayit)?;
        if secildi {
            self.secili_yaz(&paket.ad, &paket.surum, &paket.icerik_sha256)?;
        }
        Ok(KurSonuc {
            ad: paket.ad,
            surum: paket.surum,
            secili: secildi,
            eski,
        })
    }

    /// Seçili sürümü (ya da verilen sürümü) `eski/` altına taşır ve kayıttan düşer. Veri alanına dokunmaz.
    pub fn kaldir(&self, ad: &str, surum: Option<&str>) -> Sonuc<KaldirSonuc> {
        let r = self.kilitli(|| self.kaldir_ic(ad, surum));
        let s = r.as_ref().map(|k| k.surum.clone()).unwrap_or_default();
        self.gunluklu("kaldir", ad, &s, r)
    }

    fn kaldir_ic(&self, ad: &str, surum: Option<&str>) -> Sonuc<KaldirSonuc> {
        gecerli_ad_surum(ad, surum)?;
        let mut kayit = self.kayit_yukle()?;
        let pk = kayit.paketler.get(ad).cloned().ok_or_else(|| {
            KurHata::Yok(format!(
                "{ad} kurulu değil. Öneri: `taa-kur liste` ile bakın."
            ))
        })?;
        let s = match surum {
            Some(s) => s.to_string(),
            None => pk.secili.clone().ok_or_else(|| {
                KurHata::Yok(format!(
                    "{ad} için seçili sürüm yok. Öneri: `--surum` verin."
                ))
            })?,
        };
        if !pk.kurulu.contains(&s) {
            return Err(KurHata::Yok(format!(
                "{ad} {s} kurulu değil. Öneri: `taa-kur liste` ile bakın."
            )));
        }
        let paket = self.depo_paketi(ad, &s).ok();
        let (eski, tamam) = self.eskiye_tasi(&self.depo(ad, &s), &format!("{ad}-{s}"))?;
        let mut uyarilar = Vec::new();
        if !tamam {
            // Taşıma (rename) olmadı: özgün klasör depoda duruyor; kayıttan düşersek kayıt gerçekle uyuşmaz.
            return Err(KurHata::Diger(format!(
                "{ad} {s} taşınamadı, kayıt değişmedi (kopyası {} içinde, özgün klasör depoda duruyor; hiçbir şey silinmedi). Öneri: kök klasörün izinlerini ve aynı sürücüde olduğunu denetleyin.",
                eski.display()
            )));
        }

        let mut yeni_pk = pk.clone();
        yeni_pk.kurulu.retain(|x| x != &s);
        if yeni_pk.onceki.as_deref() == Some(s.as_str()) {
            yeni_pk.onceki = None;
        }
        if yeni_pk.secili.as_deref() == Some(s.as_str()) {
            let donus = yeni_pk
                .onceki
                .clone()
                .filter(|o| yeni_pk.kurulu.contains(o))
                .or_else(|| yeni_pk.kurulu.last().cloned());
            yeni_pk.secili = donus;
            yeni_pk.onceki = None;
        }
        let yeni_secili = yeni_pk.secili.clone();
        let tamamen = yeni_pk.kurulu.is_empty();

        let mut salt_okunur = Vec::new();
        if tamamen {
            if let Some(p) = &paket {
                for (diger, k) in &kayit.paketler {
                    if diger == ad {
                        continue;
                    }
                    let Some(ds) = &k.secili else { continue };
                    let Ok(dp) = self.depo_paketi(diger, ds) else {
                        continue;
                    };
                    if dp.veri_okur.iter().any(|a| p.veri_sahibi.contains(a)) {
                        salt_okunur.push(diger.clone());
                    }
                    if dp.gerektirir.contains_key(ad) {
                        uyarilar.push(format!(
                            "{diger} {ds}, kaldırılan {ad} paketini gerektiriyor; yeniden kurulana kadar çalışmayabilir."
                        ));
                    }
                }
            }
            kayit.paketler.remove(ad);
        } else {
            kayit.paketler.insert(ad.to_string(), yeni_pk);
        }
        self.kayit_kaydet(&kayit)?;

        if tamamen {
            let yol = self.secili_yolu(ad);
            if yol.exists() {
                let hedef = self.eski_hedef(&format!("{ad}-secili"), ".json")?;
                if fs::rename(&yol, &hedef).is_err() {
                    fs::copy(&yol, &hedef)?; // silinmez; eski kayıt yerinde kalır
                    uyarilar
                        .push("programlar/ altındaki secili.json taşınamadı, kopyalandı.".into());
                }
            }
        } else if let Some(ys) = &yeni_secili {
            if pk.secili.as_deref() == Some(s.as_str()) {
                match self.depo_paketi(ad, ys) {
                    Ok(dp) => self.secili_yaz(ad, ys, &dp.icerik_sha256)?,
                    Err(e) => uyarilar.push(e.to_string()),
                }
            }
        }
        Ok(KaldirSonuc {
            ad: ad.into(),
            surum: s,
            eski,
            yeni_secili,
            salt_okunur,
            uyarilar,
        })
    }

    /// Çözücü uyumluysa seçili sürümü değiştirir (eski seçim `onceki` olarak tutulur).
    pub fn sec(&self, ad: &str, surum: &str) -> Sonuc<()> {
        let r = self.kilitli(|| self.sec_ic(ad, surum));
        self.gunluklu("sec", ad, surum, r)
    }

    fn sec_ic(&self, ad: &str, surum: &str) -> Sonuc<()> {
        gecerli_ad_surum(ad, Some(surum))?;
        let mut kayit = self.kayit_yukle()?;
        let pk = kayit.paketler.get(ad).cloned().ok_or_else(|| {
            KurHata::Cozum(format!(
                "{ad} kurulu değil. Önce `taa-kur kur {ad}` çalıştırın."
            ))
        })?;
        if !pk.kurulu.iter().any(|x| x == surum) {
            return Err(KurHata::Cozum(format!(
                "{ad} {surum} kurulu değil (kurulu: {}). Önce `taa-kur kur {ad}` çalıştırın.",
                pk.kurulu.join(", ")
            )));
        }
        self.secimi_degistir(&mut kayit, ad, surum)
    }

    fn secimi_degistir(&self, kayit: &mut Kayit, ad: &str, surum: &str) -> Sonuc<()> {
        let p = self.depo_paketi(ad, surum)?;
        self.cozucu(kayit, &p)?;
        self.bagimlilar_uyumlu(kayit, &p)?;
        let pk = kayit.paketler.get_mut(ad).expect("paket var");
        if pk.secili.as_deref() == Some(surum) {
            return Ok(());
        }
        let eski = pk.secili.take();
        pk.secili = Some(surum.to_string());
        pk.onceki = eski.or(pk.onceki.take());
        self.kayit_kaydet(kayit)?;
        self.secili_yaz(ad, surum, &p.icerik_sha256)
    }

    /// Önceki seçili sürüme döner; yeni seçili sürümü döner.
    pub fn geri_don(&self, ad: &str) -> Sonuc<String> {
        let r = self.kilitli(|| self.geri_don_ic(ad));
        let s = r.as_deref().unwrap_or("").to_string();
        self.gunluklu("geri-don", ad, &s, r)
    }

    fn geri_don_ic(&self, ad: &str) -> Sonuc<String> {
        gecerli_ad_surum(ad, None)?;
        let mut kayit = self.kayit_yukle()?;
        let pk = kayit.paketler.get(ad).ok_or_else(|| {
            KurHata::Yok(format!(
                "{ad} kurulu değil. Öneri: `taa-kur liste` ile bakın."
            ))
        })?;
        let onceki = pk
            .onceki
            .clone()
            .filter(|o| pk.kurulu.contains(o))
            .ok_or_else(|| {
                KurHata::Yok(format!("{ad} için dönülecek önceki sürüm yok. Öneri: `taa-kur sec {ad} <surum>` kullanın."))
            })?;
        self.secimi_degistir(&mut kayit, ad, &onceki)?;
        Ok(onceki)
    }

    /// Sürümü sabitler ya da sabitlemeyi kaldırır (sabitliyken `kur` kendiliğinden seçimi değiştirmez).
    pub fn sabitle(&self, ad: &str, deger: bool) -> Sonuc<()> {
        let r = self.kilitli(|| {
            gecerli_ad_surum(ad, None)?;
            let mut kayit = self.kayit_yukle()?;
            kayit
                .paketler
                .get_mut(ad)
                .ok_or_else(|| KurHata::Yok(format!("{ad} kurulu değil.")))?
                .sabitle = deger;
            self.kayit_kaydet(&kayit)
        });
        self.gunluklu("sabitle", ad, "", r)
    }

    pub fn liste(&self) -> Sonuc<Vec<ListeSatiri>> {
        let (k, _) = self.kayit_oku_ham()?;
        Ok(liste_satirlari(&k))
    }

    /// Kurulmamış bir paketi (klasör ya da `.zip`) kurmadan doğrular: alanlar, yollar, sınırlar, her dosyanın SHA256'sı,
    /// `icerik_sha256` ve imza kuralı (güvenilir anahtar varsa imza zorunlu). Zip açılmaz, diske hiçbir şey yazılmaz.
    pub fn paket_denetle(&self, yol: &Path) -> Sonuc<PaketDenetim> {
        let (paket, _) = if zip_mi(yol) {
            zipac::zip_denetle(yol)?
        } else {
            paket::paket_dogrula(yol)?
        };
        let imza =
            self.imza_denetle("Paket", paket.icerik_sha256.as_bytes(), paket.imza.as_ref())?;
        Ok(PaketDenetim {
            ad: paket.ad,
            surum: paket.surum,
            dosya_sayisi: paket.dosyalar.len(),
            icerik_sha256: paket.icerik_sha256,
            imza,
        })
    }

    /// Depo içeriği SHA256 ↔ paket.json; kayit.json iç özeti (`ozet`); tutarlılık. Boş liste = sağlam.
    pub fn dogrula(&self, ad: Option<&str>) -> Sonuc<Vec<Sorun>> {
        let (kayit, ham) = self.kayit_oku_ham()?;
        let mut sorunlar: Vec<Sorun> = ham.into_iter().collect();
        if let Some(a) = ad {
            if !kayit.paketler.contains_key(a) {
                return Err(KurHata::Yok(format!(
                    "{a} kurulu değil. Öneri: `taa-kur liste` ile bakın."
                )));
            }
        }
        for (pad, pk) in &kayit.paketler {
            if ad.is_some_and(|a| a != pad) {
                continue;
            }
            for s in &pk.kurulu {
                self.surum_dogrula(pad, s, &mut sorunlar);
            }
            match &pk.secili {
                Some(s) if !pk.kurulu.contains(s) => {
                    sorunlar.push(Sorun::Bozuk(format!(
                        "{pad}: seçili sürüm {s} kurulu listesinde yok"
                    )));
                }
                Some(s) => match fs::read(self.secili_yolu(pad))
                    .ok()
                    .and_then(|b| serde_json::from_slice::<SeciliKaydi>(&b).ok())
                {
                    Some(sk) if &sk.surum == s && &sk.ad == pad => {}
                    _ => sorunlar.push(Sorun::Bozuk(format!(
                        "{pad}: programlar/{pad}/secili.json kayıtla uyuşmuyor"
                    ))),
                },
                None => {}
            }
        }
        Ok(sorunlar)
    }

    fn surum_dogrula(&self, ad: &str, surum: &str, sorunlar: &mut Vec<Sorun>) {
        let etiket = format!("{ad} {surum}");
        let klasor = self.depo(ad, surum);
        if !klasor.join("paket.json").exists() {
            sorunlar.push(Sorun::Eksik(format!("{etiket}: paket.json yok")));
            return;
        }
        let p = match paket_oku(&klasor) {
            Ok((p, _)) => p,
            Err(e) => {
                sorunlar.push(Sorun::Bozuk(format!("{etiket}: {e}")));
                return;
            }
        };
        if p.ad != ad || p.surum != surum {
            sorunlar.push(Sorun::Bozuk(format!(
                "{etiket}: paket.json başka paketi gösteriyor"
            )));
        }
        if icerik_hesapla(&p.dosyalar) != p.icerik_sha256 {
            sorunlar.push(Sorun::Bozuk(format!(
                "{etiket}: icerik_sha256 dosya listesiyle uyuşmuyor"
            )));
        }
        for d in &p.dosyalar {
            let yol = match guvenli_coz(&klasor, &d.yol) {
                Ok(y) => y,
                Err(_)
                    if paket::yol_dogrula(&d.yol).is_ok()
                        && fs::symlink_metadata(klasor.join(&d.yol)).is_err() =>
                {
                    sorunlar.push(Sorun::Eksik(format!("{etiket}: {} eksik", d.yol)));
                    continue;
                }
                Err(e) => {
                    sorunlar.push(Sorun::Bozuk(format!("{etiket}: {e}")));
                    continue;
                }
            };
            match sha256_dosya(&yol) {
                Ok((sha, _)) if sha == d.sha256 => {}
                Ok(_) => sorunlar.push(Sorun::Bozuk(format!(
                    "{etiket}: {} sha256 uyuşmuyor",
                    d.yol
                ))),
                Err(_) => sorunlar.push(Sorun::Eksik(format!("{etiket}: {} okunamadı", d.yol))),
            }
        }
    }

    /// Kök, sürümler, sahiplik tablosu, son hatalar. Sır içermez.
    pub fn tani(&self) -> Sonuc<Tani> {
        self.tani_veri_ile(veri::veri_kok_ortamdan())
    }

    /// `tani` ile aynı; veri kökü verilir (testler sahte kök kullanır).
    pub fn tani_veri_ile(&self, veri_kok: Option<PathBuf>) -> Sonuc<Tani> {
        let (kayit, ham) = self.kayit_oku_ham()?;
        let mut sorunlar = self.dogrula(None).unwrap_or_default();
        let (veri_alanlari, veri_sorunlari) = match &veri_kok {
            Some(k) => veri::alanlari_tara(k, &kayit),
            None => (Vec::new(), Vec::new()),
        };
        sorunlar.extend(veri_sorunlari);
        let mut uyarilar = Vec::new();
        let mut guvenilir_anahtarlar = Vec::new();
        match &self.anahtarlar {
            Ok(a) if a.is_empty() => uyarilar.push(
                "Güvenilir anahtar yok (anahtarlar/*.pub): paket imzaları denetlenmiyor.".into(),
            ),
            Ok(a) => guvenilir_anahtarlar = a.iter().map(|k| k.kimlik()).collect(),
            Err(m) => sorunlar.push(Sorun::Bozuk(format!("güvenilir anahtar dosyası: {m}"))),
        }
        let satirlar = taa_dosya::satirlari_oku(&self.gunluk_yolu()).unwrap_or_default();
        let mut hatalar: Vec<String> = satirlar
            .iter()
            .filter_map(|s| serde_json::from_str::<serde_json::Value>(s).ok())
            .filter(|v| v["seviye"] == "hata")
            .map(|v| {
                format!(
                    "{} {}: {}",
                    v["zaman"].as_str().unwrap_or("?"),
                    v["olay"].as_str().unwrap_or("?"),
                    v["mesaj"].as_str().unwrap_or("")
                )
            })
            .collect();
        let fazla = hatalar.len().saturating_sub(10);
        hatalar.drain(..fazla);
        Ok(Tani {
            kok: self.kok.to_string_lossy().into_owned(),
            kok_kaynagi: self.kaynak.clone(),
            kayit_saglam: ham.is_none(),
            paketler: liste_satirlari(&kayit),
            sahiplik: kayit.veri_sahipleri.clone(),
            veri_kok: veri_kok.map(|k| k.to_string_lossy().into_owned()),
            veri_alanlari,
            sorunlar,
            uyarilar,
            guvenilir_anahtarlar,
            son_hatalar: hatalar,
        })
    }
}

// ---------------------------------------------------------------- yardımcılar

/// `<kök>/anahtarlar/*.pub` dosyalarını güvenilir anahtar olarak okur (klasör yoksa boş). Aynı anahtar bir kez sayılır.
fn anahtarlari_yukle(kok: &Path) -> Result<Vec<GenelAnahtar>, String> {
    let dizin = kok.join("anahtarlar");
    let okuyucu = match fs::read_dir(&dizin) {
        Ok(o) => o,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("{} listelenemedi ({e})", dizin.display())),
    };
    let mut yollar: Vec<PathBuf> = okuyucu
        .flatten()
        .map(|o| o.path())
        .filter(|y| y.is_file() && y.extension().is_some_and(|u| u.eq_ignore_ascii_case("pub")))
        .collect();
    yollar.sort();
    let mut liste: Vec<GenelAnahtar> = Vec::new();
    for y in yollar {
        let k = GenelAnahtar::dosyadan(&y).map_err(|e| format!("{}: {e}", y.display()))?;
        if !liste.contains(&k) {
            liste.push(k);
        }
    }
    Ok(liste)
}

/// Yol bir dosya olarak `.zip` ile bitiyorsa zip paketidir (klasör `x.zip` adlı olsa bile klasör sayılır).
fn zip_mi(yol: &Path) -> bool {
    !yol.is_dir()
        && yol
            .extension()
            .is_some_and(|u| u.eq_ignore_ascii_case("zip"))
}

fn kapi() -> Sonuc<()> {
    if taa_ayar::ozellik_acik(OZELLIK, true) {
        Ok(())
    } else {
        Err(KurHata::Diger(taa_ayar::kapali_mesaji(OZELLIK)))
    }
}

fn sorun_metni(s: &Sorun) -> &str {
    match s {
        Sorun::Eksik(m) | Sorun::Bozuk(m) => m,
    }
}

fn gecerli_ad_surum(ad: &str, surum: Option<&str>) -> Sonuc<()> {
    if !ad_gecerli(ad) {
        return Err(KurHata::Dogrulama(format!("Geçersiz paket adı: {ad:?}.")));
    }
    if let Some(s) = surum {
        if !surum_gecerli(s) {
            return Err(KurHata::Dogrulama(format!("Geçersiz sürüm: {s:?}.")));
        }
    }
    Ok(())
}

fn liste_satirlari(k: &Kayit) -> Vec<ListeSatiri> {
    k.paketler
        .iter()
        .map(|(ad, p)| ListeSatiri {
            ad: ad.clone(),
            kurulu: p.kurulu.clone(),
            secili: p.secili.clone(),
            sabitle: p.sabitle,
            onceki: p.onceki.clone(),
        })
        .collect()
}

/// Doğrulanmış paketin listelenen dosyalarını (ve paket.json baytlarını) hedefe kopyalar; kopyayı yeniden SHA256'lar.
/// Yalnız listelenen dosyalar kopyalanır; sembolik bağlar izlenmez (doğrulamada reddedilir).
fn kopyala(kaynak: &Path, p: &Paket, paket_bayt: &[u8], hedef: &Path) -> Sonuc<()> {
    fs::create_dir_all(hedef)?;
    fs::write(hedef.join("paket.json"), paket_bayt)?;
    for d in &p.dosyalar {
        let src = guvenli_coz(kaynak, &d.yol).map_err(KurHata::Dogrulama)?;
        let mut dst = hedef.to_path_buf();
        dst.extend(d.yol.split('/'));
        if let Some(ust) = dst.parent() {
            fs::create_dir_all(ust)?;
        }
        fs::copy(&src, &dst)?;
        let (sha, _) = sha256_dosya(&dst)?;
        if sha != d.sha256 {
            return Err(KurHata::Dogrulama(format!(
                "{} kopyalanırken değişti (sha256 uyuşmuyor). Öneri: paket klasörünü değiştirmeyin, yeniden deneyin.",
                d.yol
            )));
        }
    }
    Ok(())
}

/// Klasör ağacını kopyalar; sembolik bağlar atlanır. Toplam boyut `sinir` baytı aşılacaksa o dosyayı kopyalamadan durur.
fn agac_kopyala(kaynak: &Path, hedef: &Path, sinir: u64) -> Sonuc<()> {
    let mut toplam = 0u64;
    agac_kopyala_ic(kaynak, hedef, sinir, &mut toplam)
}

fn agac_kopyala_ic(kaynak: &Path, hedef: &Path, sinir: u64, toplam: &mut u64) -> Sonuc<()> {
    fs::create_dir_all(hedef)?;
    for o in fs::read_dir(kaynak)?.flatten() {
        let t = o.file_type()?;
        let h = hedef.join(o.file_name());
        if t.is_dir() {
            agac_kopyala_ic(&o.path(), &h, sinir, toplam)?;
        } else if t.is_file() {
            *toplam = toplam.saturating_add(o.metadata()?.len());
            if *toplam > sinir {
                return Err(KurHata::Diger(format!(
                    "Kopyalama toplam boyut sınırını aştı ({sinir} bayt)."
                )));
            }
            fs::copy(o.path(), h)?;
        }
    }
    Ok(())
}
