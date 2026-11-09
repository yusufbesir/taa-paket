// Sahip: Yusuf. yazan: [Claude#taa-paket-harici 2026-11-09]
//! Harici bağımlılık kurucusu: TAA paketinin ihtiyaç duyduğu üçüncü taraf programlar (ilk örnek: Tailscale)
//! paket kurulurken SORMADAN indirilir, doğrulanır, sessiz kurulur ve ayarlanır (Yusuf 2026-11-09).
//!
//! Tanım paketin içindeki `harici.toml` dosyasındadır. Dosya `paket.json` → `dosyalar` listesinde olduğundan SHA256'sı
//! `icerik_sha256` içine girer; paket imzalıysa tanım da imzalıdır (indirme adresi/özet değiştirilemez).
//!
//! Güvenlik zinciri (her adım zorunlu, biri düşerse kurulum çalıştırılmaz):
//!
//! 1. yalnız `https://` adresi, en çok 512 MiB, curl ile;
//! 2. indirilen dosyanın SHA256'sı tanımdaki sabit değerle eşleşmeli;
//! 3. Windows'ta Authenticode imzası geçerli ve imzalayan adı tanımdakini içermeli;
//! 4. sessiz kurulum bağımsız değişkenleri tanımdan gelir, kabuk kullanılmaz (`Command::args`).
//!
//! Kurulum sonrası ayar (ör. `tailscale up`): anahtar KODA/TANIMA YAZILMAZ; `<kök>\gizli\<ad>.anahtar` dosyasından
//! `{anahtar_dosya}` yer tutucusuyla verilir (komut satırında anahtar görünmez). Dosya yoksa ayar atlanır, uyarı verilir.
//! Hiçbir dosya silinmez: kurucu iş bitince `eski\` altına taşınır.

use crate::paket::{ad_gecerli, sha256_dosya};
use crate::{Kur, KurHata, KurSonuc, Sonuc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub const MANIFEST_DOSYASI: &str = "harici.toml";
const MAKS_MANIFEST: u64 = 64 * 1024;
const MAKS_INDIRME_BAYT: u64 = 512 * 1024 * 1024;
const INDIRME_SURE_SN: u64 = 900;
const VARSAYILAN_KURULUM_SN: u64 = 600;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct HariciManifest {
    pub sema: u32,
    #[serde(default)]
    pub harici: Vec<Harici>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Harici {
    pub ad: String,
    pub surum: String,
    /// Yalnız `https://`; sürüme sabit adres olmalı ("latest" özet sabitlemeyi bozar).
    pub url: String,
    /// Kurucunun beklenen SHA256'sı (64 küçük hex).
    pub sha256: String,
    /// İndirilen dosyanın adı (yol ayracı içermez); boşsa adresin son parçası.
    #[serde(default)]
    pub dosya_adi: Option<String>,
    /// Authenticode imzalayan adında geçmesi gereken metin (Windows'ta zorunlu).
    pub imzalayan: String,
    /// Sessiz kurulum bağımsız değişkenleri (ör. ["/quiet"]).
    #[serde(default)]
    pub sessiz: Vec<String>,
    /// Kurulu olduğunu gösteren yürütülebilir dosya adayları (hangisi varsa kurulu sayılır). Boşsa her seferinde kurulur.
    #[serde(default)]
    pub kurulu_yollar: Vec<String>,
    #[serde(default)]
    pub kurulum_sure_sn: Option<u64>,
    #[serde(default)]
    pub ayar: Option<HariciAyar>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct HariciAyar {
    /// Kurulumdan sonra çalışacak program (`kurulu_yollar` içinden bulunan dosya ya da verilen yol).
    #[serde(default)]
    pub program: Option<String>,
    /// Bağımsız değişkenler; `{anahtar_dosya}` → `<kök>\gizli\<ad>.anahtar` yolu.
    pub args: Vec<String>,
    /// İsteğe bağlı: `<kök>\gizli\<ad>.anahtar` yoksa anahtarın alınacağı, PAKET İÇİNDEKİ göreli dosya (ör. `gizli/tailscale.anahtar`).
    /// Yalnız özel dağıtılan paketler içindir: anahtar paketle birlikte yayılır, herkese açık yayına KONMAZ.
    #[serde(default)]
    pub anahtar_paket_dosyasi: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub enum HariciDurum {
    /// Kuruldu (kurulum + varsa ayar bitti).
    Kuruldu,
    /// Zaten kuruluydu; kurucu indirilmedi.
    ZatenKurulu,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct HariciSonuc {
    pub ad: String,
    pub surum: String,
    pub durum: HariciDurum,
    /// Kurulum sonrası ayar çalıştı mı.
    pub ayarlandi: bool,
    pub uyarilar: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq, Default)]
pub struct HariciRapor {
    pub sonuclar: Vec<HariciSonuc>,
}

// ---------------------------------------------------------------- sistem soyutlaması (testler sahtesini verir)

pub trait HariciSistem {
    fn kurulu_mu(&self, yollar: &[String]) -> Option<PathBuf>;
    fn indir(&self, url: &str, hedef: &Path, en_fazla: u64) -> Sonuc<()>;
    fn imza_denetle(&self, dosya: &Path, imzalayan: &str) -> Sonuc<()>;
    /// Çıkış kodunu döner; süre dolarsa hata.
    fn calistir(&self, program: &Path, args: &[String], sure_sn: u64) -> Sonuc<i32>;
}

pub struct GercekSistem;

impl HariciSistem for GercekSistem {
    fn kurulu_mu(&self, yollar: &[String]) -> Option<PathBuf> {
        yollar
            .iter()
            .map(|y| PathBuf::from(ortam_ac(y)))
            .find(|p| p.is_file())
    }

    fn indir(&self, url: &str, hedef: &Path, en_fazla: u64) -> Sonuc<()> {
        let mut c = Command::new("curl");
        c.args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-redirs",
            "5",
            "--proto",
            "=https",
            "--proto-redir",
            "=https",
        ])
        .args(["--max-time", &INDIRME_SURE_SN.to_string()])
        .args(["--max-filesize", &en_fazla.to_string()])
        .args(["--user-agent", "taa-kur"])
        .arg("--output")
        .arg(hedef)
        .arg(url)
        .stdin(Stdio::null());
        pencere_yok(&mut c);
        let o = c.output().map_err(|e| {
            KurHata::Diger(format!(
                "curl çalıştırılamadı ({e}). Öneri: Windows 10 1803+ ile gelir; yoksa curl'ü kurun."
            ))
        })?;
        if !o.status.success() {
            let m = String::from_utf8_lossy(&o.stderr)
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            return Err(KurHata::Diger(format!(
                "{url} indirilemedi (curl çıkış {}: {m}). Öneri: ağ bağlantısını denetleyip yeniden deneyin.",
                o.status.code().unwrap_or(-1)
            )));
        }
        Ok(())
    }

    fn imza_denetle(&self, dosya: &Path, imzalayan: &str) -> Sonuc<()> {
        if !cfg!(windows) {
            return Err(KurHata::Dogrulama(
                "Authenticode imzası yalnız Windows'ta denetlenir; imza denetlenemeyen kurucu çalıştırılmaz.".into(),
            ));
        }
        // Yol ve ad PowerShell'e betik metni olarak DEĞİL ortam değişkeni olarak geçer (komut enjeksiyonu yok).
        let betik = "$s = Get-AuthenticodeSignature -LiteralPath $env:TAA_HARICI_DOSYA; \
            if ($s.Status -ne 'Valid') { [Console]::Error.WriteLine('imza durumu: ' + $s.Status); exit 3 }; \
            $k = $s.SignerCertificate.Subject; \
            if ($k -notlike ('*' + $env:TAA_HARICI_IMZALAYAN + '*')) { [Console]::Error.WriteLine('imzalayan: ' + $k); exit 4 }; \
            exit 0";
        let mut c = Command::new("powershell");
        c.args(["-NoProfile", "-NonInteractive", "-Command", betik])
            .env("TAA_HARICI_DOSYA", dosya)
            .env("TAA_HARICI_IMZALAYAN", imzalayan)
            .stdin(Stdio::null());
        pencere_yok(&mut c);
        let o = c.output().map_err(|e| {
            KurHata::Diger(format!(
                "PowerShell çalıştırılamadı ({e}); imza denetlenemedi, kurucu çalıştırılmadı."
            ))
        })?;
        if o.status.success() {
            return Ok(());
        }
        let m = String::from_utf8_lossy(&o.stderr).trim().to_string();
        Err(KurHata::Dogrulama(format!(
            "Kurucunun dijital imzası geçersiz ya da beklenen imzalayan ({imzalayan}) değil ({m}). Kurucu çalıştırılmadı."
        )))
    }

    fn calistir(&self, program: &Path, args: &[String], sure_sn: u64) -> Sonuc<i32> {
        let mut c = Command::new(program);
        c.args(args).stdin(Stdio::null());
        pencere_yok(&mut c);
        let mut cocuk = c
            .spawn()
            .map_err(|e| KurHata::Diger(format!("{} başlatılamadı ({e}).", program.display())))?;
        let son = Instant::now() + Duration::from_secs(sure_sn);
        loop {
            match cocuk.try_wait() {
                Ok(Some(d)) => return Ok(d.code().unwrap_or(-1)),
                Ok(None) if Instant::now() >= son => {
                    let _ = cocuk.kill();
                    return Err(KurHata::Diger(format!(
                        "{} {sure_sn} saniyede bitmedi; durduruldu.",
                        program.display()
                    )));
                }
                Ok(None) => std::thread::sleep(Duration::from_millis(250)),
                Err(e) => return Err(KurHata::Diger(format!("süreç beklenemedi: {e}"))),
            }
        }
    }
}

fn pencere_yok(c: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    let _ = c;
}

/// `%ProgramFiles%` gibi ortam değişkenlerini açar (yalnız `%AD%` biçimi).
fn ortam_ac(yol: &str) -> String {
    let mut cikti = String::new();
    let mut parcalar = yol.split('%');
    if let Some(ilk) = parcalar.next() {
        cikti.push_str(ilk);
    }
    let kalan: Vec<&str> = parcalar.collect();
    let mut i = 0;
    while i < kalan.len() {
        if i + 1 < kalan.len() {
            match std::env::var(kalan[i]) {
                Ok(v) => cikti.push_str(&v),
                Err(_) => {
                    cikti.push('%');
                    cikti.push_str(kalan[i]);
                    cikti.push('%');
                }
            }
            cikti.push_str(kalan[i + 1]);
            i += 2;
        } else {
            cikti.push('%');
            cikti.push_str(kalan[i]);
            i += 1;
        }
    }
    cikti
}

// ---------------------------------------------------------------- manifest

pub fn manifest_oku(yol: &Path) -> Sonuc<HariciManifest> {
    let hata = |m: String| KurHata::Dogrulama(format!("{} {m}", yol.display()));
    let meta = fs::symlink_metadata(yol).map_err(|e| {
        hata(format!(
            "okunamadı ({e}); harici.toml paketin dosyalar listesinde olmalı."
        ))
    })?;
    if !meta.is_file() || meta.len() > MAKS_MANIFEST {
        return Err(hata("normal dosya değil ya da çok büyük.".into()));
    }
    let metin = fs::read_to_string(yol).map_err(|e| hata(format!("UTF-8 okunamadı ({e}).")))?;
    let m: HariciManifest =
        toml::from_str(&metin).map_err(|e| hata(format!("bozuk TOML: {}.", e.message())))?;
    manifest_dogrula(&m).map_err(hata)?;
    Ok(m)
}

fn hex64(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
}

fn dosya_adi_gecerli(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 128
        && !s.contains(['/', '\\', ':', '\0'])
        && s != "."
        && s != ".."
        && !s.ends_with('.')
        && !s.chars().any(|c| (c as u32) < 0x20)
}

pub fn manifest_dogrula(m: &HariciManifest) -> Result<(), String> {
    if m.sema != 1 {
        return Err(format!("desteklenmeyen harici şeması: {}", m.sema));
    }
    let mut gorulen = std::collections::BTreeSet::new();
    for h in &m.harici {
        if !ad_gecerli(&h.ad) {
            return Err(format!("geçersiz harici ad: {:?}", h.ad));
        }
        if !gorulen.insert(h.ad.as_str()) {
            return Err(format!("harici iki kez tanımlı: {}", h.ad));
        }
        if h.surum.trim().is_empty() {
            return Err(format!("{}: sürüm boş", h.ad));
        }
        let k = h.url.to_ascii_lowercase();
        if !k.starts_with("https://") || h.url.contains(char::is_whitespace) {
            return Err(format!(
                "{}: url https:// ile başlamalı ve boşluk içermemeli",
                h.ad
            ));
        }
        if !hex64(&h.sha256) {
            return Err(format!("{}: sha256 64 küçük hex karakter olmalı", h.ad));
        }
        if h.imzalayan.trim().is_empty() {
            return Err(format!(
                "{}: imzalayan boş olamaz (imza denetimi zorunlu)",
                h.ad
            ));
        }
        if let Some(d) = &h.dosya_adi {
            if !dosya_adi_gecerli(d) {
                return Err(format!("{}: dosya_adi geçersiz", h.ad));
            }
        }
        if h.sessiz.iter().any(|a| a.is_empty() || a.contains('\0')) {
            return Err(format!("{}: sessiz bağımsız değişkeni boş/geçersiz", h.ad));
        }
        if let Some(a) = &h.ayar {
            if a.args.is_empty() || a.args.iter().any(|x| x.is_empty() || x.contains('\0')) {
                return Err(format!("{}: ayar.args boş/geçersiz", h.ad));
            }
            if let Some(y) = &a.anahtar_paket_dosyasi {
                crate::paket::yol_dogrula(y)
                    .map_err(|e| format!("{}: anahtar_paket_dosyasi geçersiz ({e})", h.ad))?;
            }
        }
    }
    Ok(())
}

fn indirme_adi(h: &Harici) -> String {
    if let Some(d) = &h.dosya_adi {
        return d.clone();
    }
    let son = h
        .url
        .split(['?', '#'])
        .next()
        .unwrap_or("")
        .rsplit('/')
        .next()
        .unwrap_or("");
    if dosya_adi_gecerli(son) {
        son.to_string()
    } else {
        format!("{}-kurulum.exe", h.ad)
    }
}

// ---------------------------------------------------------------- kurucu

impl Kur {
    /// Paketi kurar (`kur`), ardından paketin `harici.toml` tanımındaki bağımlı programları sormadan kurup ayarlar.
    /// Paket kurulumu başarılıysa harici hata, paketi geri almaz; hata `HariciRapor` içinde değil `Err` ile döner
    /// ama paket zaten kayıttadır (mesajda belirtilir). `atla` doğruysa harici adım yapılmaz.
    pub fn kur_tam(
        &self,
        paket_yolu: &Path,
        sec: bool,
        atla: bool,
    ) -> Sonuc<(KurSonuc, HariciRapor)> {
        self.kur_tam_sistemle(paket_yolu, sec, atla, &GercekSistem)
    }

    pub fn kur_tam_sistemle(
        &self,
        paket_yolu: &Path,
        sec: bool,
        atla: bool,
        sistem: &dyn HariciSistem,
    ) -> Sonuc<(KurSonuc, HariciRapor)> {
        let s = self.kur(paket_yolu, sec)?;
        if atla {
            return Ok((s, HariciRapor::default()));
        }
        let manifest = self.depo(&s.ad, &s.surum).join(MANIFEST_DOSYASI);
        if !manifest.is_file() {
            return Ok((s, HariciRapor::default()));
        }
        let rapor = self
            .harici_kur(&manifest, sistem)
            .map_err(|e| {
                e.yeniden(format!(
                    "{} {} kuruldu ama bağımlı program kurulamadı: {} Öneri: sorunu giderip `taa-kur harici {}` çalıştırın.",
                    s.ad,
                    s.surum,
                    e.mesaj(),
                    manifest.display()
                ))
            })?;
        Ok((s, rapor))
    }

    /// Kurulu paketin (sürüm verilmezse seçili sürümün) `harici.toml` tanımını uygular; tanım yoksa boş rapor.
    pub fn harici_kur_paketten(&self, ad: &str, surum: Option<&str>) -> Sonuc<HariciRapor> {
        let surum = match surum {
            Some(s) => s.to_string(),
            None => self
                .kayit_yukle()?
                .paketler
                .get(ad)
                .and_then(|k| k.secili.clone())
                .ok_or_else(|| {
                    KurHata::Yok(format!(
                        "{ad} kurulu ya da seçili değil. Öneri: `taa-kur liste` ile bakın."
                    ))
                })?,
        };
        let manifest = self.depo(ad, &surum).join(MANIFEST_DOSYASI);
        if !manifest.is_file() {
            return Ok(HariciRapor::default());
        }
        self.harici_kur(&manifest, &GercekSistem)
    }

    /// `harici.toml` içindeki her bağımlılığı sırayla kurar ve ayarlar.
    pub fn harici_kur(&self, manifest: &Path, sistem: &dyn HariciSistem) -> Sonuc<HariciRapor> {
        let m = manifest_oku(manifest)?;
        let mut rapor = HariciRapor::default();
        for h in &m.harici {
            let r = self.harici_bir(h, manifest.parent().unwrap_or(Path::new(".")), sistem);
            self.gunluk(
                if r.is_ok() { "bilgi" } else { "hata" },
                "harici",
                &h.ad,
                &h.surum,
                &r.as_ref()
                    .err()
                    .map(|e| e.mesaj().to_string())
                    .unwrap_or_default(),
            );
            rapor.sonuclar.push(r?);
        }
        Ok(rapor)
    }

    fn harici_bir(
        &self,
        h: &Harici,
        paket_dizin: &Path,
        sistem: &dyn HariciSistem,
    ) -> Sonuc<HariciSonuc> {
        let mut uyarilar = Vec::new();
        let mut bulunan = sistem.kurulu_mu(&h.kurulu_yollar);
        let durum = if bulunan.is_some() {
            HariciDurum::ZatenKurulu
        } else {
            let dizin = self.kok.join("indirilen").join("harici");
            fs::create_dir_all(&dizin)?;
            let dosya = dizin.join(format!("{}-{}-{}", h.ad, h.surum, indirme_adi(h)));
            if dosya.exists() {
                self.eskiye_tasi(&dosya, &format!("harici-{}-onceki", h.ad))?;
            }
            sistem.indir(&h.url, &dosya, MAKS_INDIRME_BAYT)?;
            let sonuc = self.kurucu_calistir(h, &dosya, sistem);
            // Kurucu her durumda (başarı/hata) eski/ altına taşınır; silinmez.
            if dosya.exists() {
                let etiket = if sonuc.is_ok() {
                    format!("harici-{}", h.ad)
                } else {
                    format!("harici-{}-hatali", h.ad)
                };
                if let Err(e) = self.eskiye_tasi(&dosya, &etiket) {
                    uyarilar.push(format!("Kurucu eski/ altına taşınamadı: {}", e.mesaj()));
                }
            }
            sonuc?;
            bulunan = sistem.kurulu_mu(&h.kurulu_yollar);
            if !h.kurulu_yollar.is_empty() && bulunan.is_none() {
                return Err(KurHata::Diger(format!(
                    "{} kurucusu başarıyla bitti ama kurulu dosya bulunamadı ({}). Öneri: kurulumu elle denetleyin.",
                    h.ad,
                    h.kurulu_yollar.join(", ")
                )));
            }
            HariciDurum::Kuruldu
        };

        let ayarlandi =
            self.harici_ayarla(h, paket_dizin, bulunan.as_deref(), sistem, &mut uyarilar)?;
        Ok(HariciSonuc {
            ad: h.ad.clone(),
            surum: h.surum.clone(),
            durum,
            ayarlandi,
            uyarilar,
        })
    }

    fn kurucu_calistir(&self, h: &Harici, dosya: &Path, sistem: &dyn HariciSistem) -> Sonuc<()> {
        let (sha, _) = sha256_dosya(dosya)?;
        if sha != h.sha256 {
            return Err(KurHata::Dogrulama(format!(
                "{} kurucusunun SHA256 değeri tanımdakiyle uyuşmuyor (beklenen {}, gelen {sha}). Kurucu çalıştırılmadı. Öneri: tanımı (harici.toml) ve adresi denetleyin.",
                h.ad, h.sha256
            )));
        }
        sistem.imza_denetle(dosya, &h.imzalayan)?;
        let sure = h.kurulum_sure_sn.unwrap_or(VARSAYILAN_KURULUM_SN).min(3600);
        let kod = sistem.calistir(dosya, &h.sessiz, sure)?;
        // 3010: yeniden başlatma gerekli, 1641: kurucu yeniden başlatma başlattı; ikisi de kurulum başarısıdır.
        if !matches!(kod, 0 | 3010 | 1641) {
            return Err(KurHata::Diger(format!(
                "{} kurucusu {kod} koduyla bitti. Öneri: yönetici olarak yeniden deneyin.",
                h.ad
            )));
        }
        Ok(())
    }

    /// Paket içindeki anahtar dosyasını `<kök>\gizli\<ad>.anahtar` olarak kopyalar (içerik hiçbir yere yazılmaz/günlüğe girmez).
    fn anahtari_paketten_al(
        &self,
        ad: &str,
        paket_dizin: &Path,
        goreli: &str,
        hedef: &Path,
        uyarilar: &mut Vec<String>,
    ) -> Sonuc<()> {
        let Ok(kaynak) = crate::paket::guvenli_coz(paket_dizin, goreli) else {
            uyarilar.push(format!(
                "{ad}: pakette anahtar dosyası ({goreli}) yok; ayar atlanacak."
            ));
            return Ok(());
        };
        let m = fs::symlink_metadata(&kaynak)?;
        if !m.is_file() || m.len() == 0 || m.len() > 1024 {
            return Err(KurHata::Bozuk(format!(
                "{ad}: paketteki anahtar dosyası geçersiz (boş ya da 1 KiB'tan büyük)."
            )));
        }
        let b = fs::read(&kaynak)?;
        let metin = String::from_utf8_lossy(&b);
        if !metin.trim().starts_with("tskey-") {
            return Err(KurHata::Bozuk(format!(
                "{ad}: paketteki anahtar dosyası beklenen biçimde değil (tskey- ile başlamalı)."
            )));
        }
        if let Some(d) = hedef.parent() {
            fs::create_dir_all(d)?;
        }
        taa_dosya::uzerine_yaz_atomik(hedef, metin.trim().as_bytes())?;
        Ok(())
    }

    /// Kurulum sonrası ayar. Anahtar dosyası yoksa ayar atlanır (uyarı); varsa komut çalışır.
    fn harici_ayarla(
        &self,
        h: &Harici,
        paket_dizin: &Path,
        kurulu: Option<&Path>,
        sistem: &dyn HariciSistem,
        uyarilar: &mut Vec<String>,
    ) -> Sonuc<bool> {
        let Some(a) = &h.ayar else { return Ok(false) };
        let anahtar = self.kok.join("gizli").join(format!("{}.anahtar", h.ad));
        if !anahtar.is_file() {
            if let Some(y) = &a.anahtar_paket_dosyasi {
                self.anahtari_paketten_al(&h.ad, paket_dizin, y, &anahtar, uyarilar)?;
            }
        }
        let kullaniyor = a.args.iter().any(|x| x.contains("{anahtar_dosya}"));
        if kullaniyor && !anahtar.is_file() {
            uyarilar.push(format!(
                "{} ayarı atlandı: {} dosyası yok. Otomatik katılım için bu dosyaya tek kullanımlık/önceden yetkili anahtarı koyup `taa-kur harici` komutunu yeniden çalıştırın.",
                h.ad,
                anahtar.display()
            ));
            return Ok(false);
        }
        let program = match (&a.program, kurulu) {
            (Some(p), _) => PathBuf::from(ortam_ac(p)),
            (None, Some(k)) => k.to_path_buf(),
            (None, None) => {
                return Err(KurHata::Diger(format!(
                    "{} ayarı için program bilinmiyor (ayar.program ya da kurulu_yollar gerekli).",
                    h.ad
                )))
            }
        };
        let args: Vec<String> = a
            .args
            .iter()
            .map(|x| x.replace("{anahtar_dosya}", &anahtar.to_string_lossy()))
            .collect();
        let kod = sistem.calistir(&program, &args, 120)?;
        if kod != 0 {
            // Hata iletisine bağımsız değişkenler girmez (anahtar yolu içerir).
            return Err(KurHata::Diger(format!(
                "{} ayar komutu {kod} koduyla bitti. Öneri: anahtarın geçerli/süresi dolmamış olduğunu denetleyin.",
                h.ad
            )));
        }
        Ok(true)
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::cell::RefCell;

    struct Sahte {
        kurulu: RefCell<bool>,
        icerik: Vec<u8>,
        imza_tamam: bool,
        cikis: i32,
        ayar_cikis: i32,
        cagrilar: RefCell<Vec<String>>,
        yol: PathBuf,
    }

    impl Sahte {
        fn yeni(taban: &Path) -> Sahte {
            let yol = taban.join("Programlar").join("tailscale.exe");
            Sahte {
                kurulu: RefCell::new(false),
                icerik: b"kurucu".to_vec(),
                imza_tamam: true,
                cikis: 0,
                ayar_cikis: 0,
                cagrilar: RefCell::new(vec![]),
                yol,
            }
        }
    }

    impl HariciSistem for Sahte {
        fn kurulu_mu(&self, _y: &[String]) -> Option<PathBuf> {
            self.kurulu.borrow().then(|| self.yol.clone())
        }
        fn indir(&self, url: &str, hedef: &Path, _e: u64) -> Sonuc<()> {
            self.cagrilar.borrow_mut().push(format!("indir {url}"));
            fs::write(hedef, &self.icerik)?;
            Ok(())
        }
        fn imza_denetle(&self, _d: &Path, _i: &str) -> Sonuc<()> {
            self.cagrilar.borrow_mut().push("imza".into());
            if self.imza_tamam {
                Ok(())
            } else {
                Err(KurHata::Dogrulama("imza geçersiz".into()))
            }
        }
        fn calistir(&self, p: &Path, args: &[String], _s: u64) -> Sonuc<i32> {
            let ayar = p == self.yol;
            self.cagrilar.borrow_mut().push(format!(
                "calistir {} {}",
                if ayar { "ayar" } else { "kurucu" },
                args.join(" ")
            ));
            if ayar {
                return Ok(self.ayar_cikis);
            }
            *self.kurulu.borrow_mut() = self.cikis == 0 || self.cikis == 3010;
            Ok(self.cikis)
        }
    }

    fn ortam(icerik: &[u8], ayar: bool) -> (PathBuf, Kur, PathBuf, Sahte) {
        let taban = taa_dosya::gecici_klasor().unwrap();
        let kur = Kur::yeni(taban.join("kok")).unwrap();
        let sha = crate::paket::sha256_bayt(icerik);
        let ayar_satiri = if ayar {
            "[harici.ayar]\nargs = [\"up\", \"--auth-key=file:{anahtar_dosya}\"]\n"
        } else {
            ""
        };
        let m = taban.join("harici.toml");
        fs::write(
            &m,
            format!(
                "sema = 1\n[[harici]]\nad = \"tailscale\"\nsurum = \"1.0\"\nurl = \"https://pkgs.example/ts-setup.exe\"\nsha256 = \"{sha}\"\nimzalayan = \"Tailscale Inc.\"\nsessiz = [\"/quiet\"]\nkurulu_yollar = [\"x\"]\n{ayar_satiri}"
            ),
        )
        .unwrap();
        let s = Sahte::yeni(&taban);
        (taban, kur, m, s)
    }

    #[test]
    fn kurar_ve_kurucuyu_eskiye_tasir() {
        let (taban, kur, m, s) = ortam(b"kurucu", false);
        let r = kur.harici_kur(&m, &s).unwrap();
        assert_eq!(r.sonuclar[0].durum, HariciDurum::Kuruldu);
        assert!(!r.sonuclar[0].ayarlandi);
        let c = s.cagrilar.borrow().join("|");
        assert!(
            c.contains("indir https://pkgs.example/ts-setup.exe|imza|calistir kurucu /quiet"),
            "{c}"
        );
        assert!(!kur
            .kok()
            .join("indirilen/harici/tailscale-1.0-ts-setup.exe")
            .exists());
        assert!(fs::read_dir(kur.kok().join("eski")).unwrap().count() >= 1);
        let _ = fs::remove_dir_all(taban);
    }

    #[test]
    fn zaten_kuruluysa_indirmez() {
        let (taban, kur, m, s) = ortam(b"kurucu", false);
        *s.kurulu.borrow_mut() = true;
        let r = kur.harici_kur(&m, &s).unwrap();
        assert_eq!(r.sonuclar[0].durum, HariciDurum::ZatenKurulu);
        assert!(s.cagrilar.borrow().is_empty());
        let _ = fs::remove_dir_all(taban);
    }

    #[test]
    fn sha_uyusmazsa_calistirmaz() {
        let (taban, kur, m, mut s) = ortam(b"kurucu", false);
        s.icerik = b"baska".to_vec();
        let e = kur.harici_kur(&m, &s).unwrap_err();
        assert!(e.mesaj().contains("SHA256"), "{e}");
        assert!(!s
            .cagrilar
            .borrow()
            .iter()
            .any(|c| c.starts_with("calistir") || c == "imza"));
        let _ = fs::remove_dir_all(taban);
    }

    #[test]
    fn imza_gecersizse_calistirmaz() {
        let (taban, kur, m, mut s) = ortam(b"kurucu", false);
        s.imza_tamam = false;
        assert!(kur.harici_kur(&m, &s).is_err());
        assert!(!s
            .cagrilar
            .borrow()
            .iter()
            .any(|c| c.starts_with("calistir")));
        let _ = fs::remove_dir_all(taban);
    }

    #[test]
    fn kurucu_hata_kodu_hata_olur_3010_olmaz() {
        let (taban, kur, m, mut s) = ortam(b"kurucu", false);
        s.cikis = 1603;
        assert!(kur.harici_kur(&m, &s).unwrap_err().mesaj().contains("1603"));
        let _ = fs::remove_dir_all(&taban);
        let (taban, kur, m, mut s) = ortam(b"kurucu", false);
        s.cikis = 3010;
        assert!(kur.harici_kur(&m, &s).is_ok());
        let _ = fs::remove_dir_all(taban);
    }

    #[test]
    fn anahtar_yoksa_ayar_atlanir_varsa_calisir_ve_anahtar_yazilmaz() {
        let (taban, kur, m, s) = ortam(b"kurucu", true);
        let r = kur.harici_kur(&m, &s).unwrap();
        assert!(!r.sonuclar[0].ayarlandi);
        assert!(r.sonuclar[0].uyarilar[0].contains("atlandı"));
        fs::create_dir_all(kur.kok().join("gizli")).unwrap();
        fs::write(kur.kok().join("gizli/tailscale.anahtar"), "tskey-gizli").unwrap();
        let r = kur.harici_kur(&m, &s).unwrap(); // artık kurulu: yalnız ayar
        assert_eq!(r.sonuclar[0].durum, HariciDurum::ZatenKurulu);
        assert!(r.sonuclar[0].ayarlandi);
        let c = s.cagrilar.borrow().join("|");
        assert!(
            c.contains("--auth-key=file:") && !c.contains("tskey-gizli"),
            "{c}"
        );
        let _ = fs::remove_dir_all(taban);
    }

    #[test]
    fn anahtar_paketten_alinir_ve_ayar_calisir() {
        let (taban, kur, m, s) = ortam(b"kurucu", true);
        let ic = fs::read_to_string(&m).unwrap().replace(
            "args = [\"up\"",
            "anahtar_paket_dosyasi = \"gizli/ts.anahtar\"\nargs = [\"up\"",
        );
        fs::write(&m, ic).unwrap();
        fs::create_dir_all(taban.join("gizli")).unwrap();
        fs::write(taban.join("gizli/ts.anahtar"), "tskey-auth-abc\n").unwrap();
        let r = kur.harici_kur(&m, &s).unwrap();
        assert!(r.sonuclar[0].ayarlandi, "{:?}", r.sonuclar[0].uyarilar);
        assert_eq!(
            fs::read_to_string(kur.kok().join("gizli/tailscale.anahtar")).unwrap(),
            "tskey-auth-abc"
        );
        assert!(!s.cagrilar.borrow().join("|").contains("tskey-auth-abc"));
        let _ = fs::remove_dir_all(taban);
    }

    #[test]
    fn paketteki_bozuk_anahtar_reddedilir() {
        let (taban, kur, m, s) = ortam(b"kurucu", true);
        let ic = fs::read_to_string(&m).unwrap().replace(
            "args = [\"up\"",
            "anahtar_paket_dosyasi = \"gizli/ts.anahtar\"\nargs = [\"up\"",
        );
        fs::write(&m, ic).unwrap();
        fs::create_dir_all(taban.join("gizli")).unwrap();
        fs::write(taban.join("gizli/ts.anahtar"), "baska-bir-sey").unwrap();
        assert!(kur.harici_kur(&m, &s).is_err());
        assert!(!kur.kok().join("gizli/tailscale.anahtar").exists());
        let _ = fs::remove_dir_all(taban);
    }

    #[test]
    fn ayar_hatasi_anahtar_yolunu_sizdirmaz() {
        let (taban, kur, m, mut s) = ortam(b"kurucu", true);
        fs::create_dir_all(kur.kok().join("gizli")).unwrap();
        fs::write(kur.kok().join("gizli/tailscale.anahtar"), "x").unwrap();
        s.ayar_cikis = 1;
        let e = kur.harici_kur(&m, &s).unwrap_err();
        assert!(!e.mesaj().contains("gizli"), "{e}");
        let _ = fs::remove_dir_all(taban);
    }

    #[test]
    fn manifest_kotu_girdileri_reddeder() {
        let s = "a".repeat(64);
        let temel = |ek: &str| {
            format!("sema = 1\n[[harici]]\nad = \"x\"\nsurum = \"1\"\nurl = \"https://a/b\"\nsha256 = \"{s}\"\nimzalayan = \"Y\"\n{ek}")
        };
        let yaz = |ic: String| {
            let t = taa_dosya::gecici_klasor().unwrap();
            let y = t.join("harici.toml");
            fs::write(&y, ic).unwrap();
            let r = manifest_oku(&y);
            let _ = fs::remove_dir_all(t);
            r
        };
        assert!(yaz(temel("")).is_ok());
        assert!(yaz(temel("").replace("https://a/b", "http://a/b")).is_err());
        assert!(yaz(temel("").replace(&s, "abc")).is_err());
        assert!(yaz(temel("").replace("imzalayan = \"Y\"", "imzalayan = \"\"")).is_err());
        assert!(yaz(temel("dosya_adi = \"../x.exe\"\n")).is_err());
        assert!(yaz(temel("").replace("sema = 1", "sema = 2")).is_err());
        assert!(yaz(temel("").replace("ad = \"x\"", "ad = \"../x\"")).is_err());
    }

    #[test]
    fn ortam_ac_calisir() {
        std::env::set_var("TAA_TEST_ORTAM", "D:\\P");
        assert_eq!(ortam_ac("%TAA_TEST_ORTAM%\\a.exe"), "D:\\P\\a.exe");
        assert_eq!(
            ortam_ac("%YOK_BOYLE_BIR_DEGISKEN_%\\a"),
            "%YOK_BOYLE_BIR_DEGISKEN_%\\a"
        );
        assert_eq!(ortam_ac("düz"), "düz");
    }

    #[test]
    fn kur_tam_manifestsiz_pakette_harici_yapmaz() {
        let taban = taa_dosya::gecici_klasor().unwrap();
        let kur = Kur::yeni(taban.join("kok")).unwrap();
        let p = crate::testler::paket_yap(
            &taban,
            "ornek",
            "1.0.0",
            "1",
            "ortak",
            &[],
            &[("a.txt", "x")],
        );
        let s = Sahte::yeni(&taban);
        let (r, h) = kur.kur_tam_sistemle(&p, true, false, &s).unwrap();
        assert_eq!(r.ad, "ornek");
        assert!(h.sonuclar.is_empty() && s.cagrilar.borrow().is_empty());
        let _ = fs::remove_dir_all(taban);
    }

    #[test]
    fn kur_tam_manifestli_pakette_kurar() {
        let taban = taa_dosya::gecici_klasor().unwrap();
        let kur = Kur::yeni(taban.join("kok")).unwrap();
        let sha = crate::paket::sha256_bayt(b"kurucu");
        let m = format!("sema = 1\n[[harici]]\nad = \"tailscale\"\nsurum = \"1.0\"\nurl = \"https://pkgs.example/ts.exe\"\nsha256 = \"{sha}\"\nimzalayan = \"Tailscale Inc.\"\nkurulu_yollar = [\"x\"]\n");
        let p = crate::testler::paket_yap(
            &taban,
            "ornek",
            "1.0.0",
            "1",
            "ortak",
            &[],
            &[("harici.toml", &m)],
        );
        let s = Sahte::yeni(&taban);
        let (_, h) = kur.kur_tam_sistemle(&p, true, false, &s).unwrap();
        assert_eq!(h.sonuclar[0].durum, HariciDurum::Kuruldu);
        // --harici-atla
        let (_, h2) = kur.kur_tam_sistemle(&p, true, true, &s).unwrap();
        assert!(h2.sonuclar.is_empty());
        let _ = fs::remove_dir_all(taban);
    }
}
