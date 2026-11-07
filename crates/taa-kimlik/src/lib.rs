// Sahip: Yusuf. yazan: [Claude#taa-kimlik-T7 2026-11-06]
//! taa-kimlik: merkez sunucuya giriş ve oturum saklama (bkz. docs/T7-KIMLIK.md).
//!
//! Sözleşme (Tutaa API-SOZLESMESI.md): `POST /v1/giris {kullanici, parola[, kod]} -> {oturum, sube_id, roller[]}`;
//! sonraki her istek `Authorization: Bearer <oturum>` taşır; şube sunucuda oturumdan çıkarılır.
//!
//! Kurallar:
//! - Parola hiçbir yere yazılmaz; yalnız giriş isteğinde sunucuya gider.
//! - Oturum anahtarı Windows'ta **Kimlik Bilgisi Yöneticisi**'nde saklanır (`KimlikDeposu`, hedef `TAA:<sunucu>`).
//!   Windows dışında `DosyaDeposu` (`<kök>/kimlik/oturum-<sunucu>.json`, yalnız sahibi okur) **test içindir**.
//! - Depo/sohbet/çıktıya anahtar yazılmaz: `Oturum::ozet()` anahtarı içermez; CLI anahtarı hiç basmaz.
//! - HTTP için yeni crate yok: taa-kuyruk'un `CurlGonderici`si kullanılır.
//!
//! Özellik anahtarı: "kimlik".

pub mod yetki;
pub use yetki::{sunucu_anahtari_oku, yetki_al, yetki_oku, Yetki, YETKI_UCU};

#[cfg(test)]
mod testler;

pub use taa_kuyruk::{CurlGonderici, GonderHata, Gonderici, Imzalayici};

use serde_json::{json, Value};
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const OZELLIK: &str = "kimlik";
/// Giriş ucu.
pub const GIRIS_UCU: &str = "/v1/giris";
/// Kullanıcı adı uzunluk sınırı.
pub const KULLANICI_EN_COK: usize = 64;

#[derive(Debug)]
pub enum KimlikHata {
    /// Kullanıcı adı boş, çok uzun ya da satır sonu içeriyor; parola boş.
    GecersizGirdi(String),
    /// Sunucu girişi reddetti (401/403 vb.).
    GirisReddi { kod: u16, mesaj: String },
    /// Sunucuya ulaşılamadı / yapılandırma.
    Baglanti(String),
    /// Sunucu yanıtı sözleşmeye uymuyor.
    BozukYanit(String),
    /// Depoda oturum yok.
    OturumYok,
    /// Saklı yetki belgesi yok, bozuk ya da süresi geçmiş (varsayılan: reddet).
    YetkiYok(String),
    /// Depo (Kimlik Bilgisi Yöneticisi / dosya) hatası.
    Depo(String),
}

impl fmt::Display for KimlikHata {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            KimlikHata::GecersizGirdi(m) => write!(f, "geçersiz girdi: {m}"),
            KimlikHata::GirisReddi { kod, mesaj } => {
                write!(f, "giriş reddedildi ({kod}): {mesaj}. Öneri: kullanıcı adı ve parolayı denetleyin.")
            }
            KimlikHata::Baglanti(m) => write!(f, "{m}"),
            KimlikHata::BozukYanit(m) => write!(f, "sunucu yanıtı beklenen biçimde değil: {m}"),
            KimlikHata::OturumYok => write!(f, "kayıtlı oturum yok. Öneri: taa-kimlik giris"),
            KimlikHata::YetkiYok(m) => write!(f, "yetki yok: {m}"),
            KimlikHata::Depo(m) => write!(f, "oturum deposu hatası: {m}"),
        }
    }
}

impl std::error::Error for KimlikHata {}

/// Sunucudan alınan oturum. `oturum` alanı gizlidir: `Debug` ve `ozet()` onu göstermez.
#[derive(Clone, PartialEq)]
pub struct Oturum {
    pub sunucu: String,
    pub kullanici: String,
    pub sube_id: Value,
    pub roller: Vec<String>,
    /// Sunucunun verdiği izin adları (`program.islem`); eski sunucu yanıtında boş.
    pub izinler: Vec<String>,
    /// Bu kullanıcı/şubeye açık programlar; boşsa sınırlama bilgisi yok.
    pub programlar: Vec<String>,
    /// İzin listesinin çevrimdışı geçerlilik sonu (ISO); boşsa sınırsız.
    pub gecerlilik: String,
    pub zaman: String,
    oturum: String,
}

impl fmt::Debug for Oturum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Oturum")
            .field("sunucu", &self.sunucu)
            .field("kullanici", &self.kullanici)
            .field("sube_id", &self.sube_id)
            .field("roller", &self.roller)
            .field("izinler", &self.izinler)
            .field("programlar", &self.programlar)
            .field("gecerlilik", &self.gecerlilik)
            .field("zaman", &self.zaman)
            .field("oturum", &"<gizli>")
            .finish()
    }
}

impl Oturum {
    /// Bearer anahtarı. Yalnız gönderici kurulurken kullanılır; çıktıya yazılmaz.
    pub fn anahtar(&self) -> &str {
        &self.oturum
    }

    /// Anahtar içermeyen özet (ekrana/JSON'a basılabilir).
    pub fn ozet(&self) -> Value {
        json!({
            "sunucu": self.sunucu,
            "kullanici": self.kullanici,
            "sube_id": self.sube_id,
            "roller": self.roller,
            "izinler": self.izinler,
            "programlar": self.programlar,
            "gecerlilik": self.gecerlilik,
            "zaman": self.zaman,
        })
    }

    /// Depoya yazılan biçim (anahtar dahil). Yalnız depo kullanır.
    fn depo_json(&self) -> String {
        let mut v = self.ozet();
        v["oturum"] = Value::String(self.oturum.clone());
        v.to_string()
    }

    fn depo_jsondan(metin: &str) -> Result<Oturum, KimlikHata> {
        let v: Value = serde_json::from_str(metin)
            .map_err(|e| KimlikHata::Depo(format!("kayıt çözülemedi: {e}")))?;
        let al = |a: &str| {
            v.get(a)
                .and_then(Value::as_str)
                .map(str::to_string)
                .ok_or_else(|| KimlikHata::Depo(format!("kayıtta '{a}' yok")))
        };
        Ok(Oturum {
            sunucu: al("sunucu")?,
            kullanici: al("kullanici")?,
            sube_id: v.get("sube_id").cloned().unwrap_or(Value::Null),
            roller: roller_coz(v.get("roller")),
            izinler: roller_coz(v.get("izinler")),
            programlar: roller_coz(v.get("programlar")),
            gecerlilik: al("gecerlilik").unwrap_or_default(),
            zaman: al("zaman").unwrap_or_default(),
            oturum: al("oturum")?,
        })
    }

    /// Bu izin var mı? Yalnız sunucunun verdiği `izinler` listesine bakılır (K1 varsayılan reddet): rol adı izin vermez,
    /// `yonetici` dahil (sunucu rol mirasını açarak listeyi tam gönderir). Var olmayan izin adı her zaman reddedilir.
    pub fn izinli(&self, izin: &str) -> bool {
        self.izinler.iter().any(|i| i == izin)
    }

    /// Program bu oturuma açık mı? Liste boşsa (eski sunucu) açık sayılır.
    pub fn program_acik(&self, program: &str) -> bool {
        self.programlar.is_empty() || self.programlar.iter().any(|p| p == program)
    }

    /// Bu oturumla kuyruk göndericisi kurar (cihaz imzası yok; bkz. `gonderici_ile`).
    pub fn gonderici(&self, sertifika: Option<&Path>) -> Result<CurlGonderici, KimlikHata> {
        self.gonderici_ile(sertifika, None)
    }

    /// Oturum + isteğe bağlı cihaz imzalayıcısı (`taa_cihaz::CihazAnahtari`). VPN yok kararıyla sunucu
    /// cihaz imzası olmayan isteği reddeder; `cihaz_imzalayici(kok)` ile yüklenir.
    pub fn gonderici_ile(
        &self,
        sertifika: Option<&Path>,
        cihaz: Option<Arc<dyn Imzalayici>>,
    ) -> Result<CurlGonderici, KimlikHata> {
        self.gonderici_taban(&self.sunucu, sertifika, cihaz)
    }

    /// Oturum anahtarı mantıksal ad (`merkez`) iken istek adresi yol listesinden gelir (taa-yol, S4a):
    /// `taban` o adrestir; `self.sunucu` yalnız depo anahtarıdır.
    pub fn gonderici_taban(
        &self,
        taban: &str,
        sertifika: Option<&Path>,
        cihaz: Option<Arc<dyn Imzalayici>>,
    ) -> Result<CurlGonderici, KimlikHata> {
        let mut g = CurlGonderici::yeni(taban)
            .and_then(|g| g.oturum(&self.oturum))
            .map_err(|e| KimlikHata::Baglanti(e.to_string()))?;
        if let Some(c) = sertifika {
            g = g.sertifika(c);
        }
        if let Some(i) = cihaz {
            g = g.imzalayici(i);
        }
        Ok(g)
    }
}

/// `<kök>/cihaz/` altında cihaz anahtarı varsa imzalayıcı olarak yükler; yoksa None (sunucu reddedebilir),
/// bozuksa hata.
pub fn cihaz_imzalayici(kok: &Path) -> Result<Option<Arc<dyn Imzalayici>>, KimlikHata> {
    if !taa_cihaz::CihazAnahtari::var_mi(kok) {
        return Ok(None);
    }
    let c = taa_cihaz::CihazAnahtari::yukle(kok).map_err(|e| KimlikHata::Depo(e.to_string()))?;
    Ok(Some(Arc::new(c)))
}

fn roller_coz(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

pub fn kullanici_gecerli(k: &str) -> bool {
    !k.is_empty()
        && k.len() <= KULLANICI_EN_COK
        && !k.chars().any(|c| c.is_control() || c.is_whitespace())
}

/// Sunucu adresini depo anahtarına çevirir (`https://192.168.1.10:8443` → `192.168.1.10_8443`).
pub fn sunucu_anahtari(sunucu: &str) -> String {
    let s = sunucu.trim().trim_end_matches('/').to_ascii_lowercase();
    let s = s
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    let mut cikti: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    cikti.truncate(96);
    if cikti.is_empty() {
        "sunucu".into()
    } else {
        cikti
    }
}

/// `POST /v1/giris`. `kod`: isteğe bağlı ikinci adım (SMS/uygulama) kodu. Sunucu imzası denetlenmez
/// (bkz. [`giris_dogrulayarak`]).
pub fn giris(
    g: &dyn Gonderici,
    sunucu: &str,
    kullanici: &str,
    parola: &str,
    kod: Option<&str>,
) -> Result<Oturum, KimlikHata> {
    giris_dogrulayarak(g, sunucu, kullanici, parola, kod, None)
}

/// [`giris`] + sunucu genel anahtarı verilmişse yanıtın `imza` alanı doğrulanır (kanonik bayt, imza hariç);
/// imzasız ya da yanlış imzalı yanıt `BozukYanit` ile reddedilir.
pub fn giris_dogrulayarak(
    g: &dyn Gonderici,
    sunucu: &str,
    kullanici: &str,
    parola: &str,
    kod: Option<&str>,
    sunucu_anahtari: Option<&taa_imza::GenelAnahtar>,
) -> Result<Oturum, KimlikHata> {
    if !kullanici_gecerli(kullanici) {
        return Err(KimlikHata::GecersizGirdi(format!(
            "kullanıcı adı boş/çok uzun/boşluklu olamaz (en çok {KULLANICI_EN_COK})"
        )));
    }
    if parola.is_empty() || parola.contains(['\n', '\r']) {
        return Err(KimlikHata::GecersizGirdi("parola boş olamaz".into()));
    }
    let mut govde = json!({"kullanici": kullanici, "parola": parola});
    if let Some(k) = kod.map(str::trim).filter(|k| !k.is_empty()) {
        govde["kod"] = Value::String(k.to_string());
    }
    let yanit = match g.gonder(GIRIS_UCU, &govde) {
        Ok(y) => y,
        Err(GonderHata::Reddedildi { kod, mesaj }) => {
            return Err(KimlikHata::GirisReddi { kod, mesaj })
        }
        Err(e) => return Err(KimlikHata::Baglanti(e.to_string())),
    };
    if let Some(a) = sunucu_anahtari {
        yetki::belge_dogrula(&yanit, a)?;
    }
    let oturum = yanit
        .get("oturum")
        .and_then(Value::as_str)
        .filter(|o| !o.is_empty() && o.is_ascii() && !o.contains(['\n', '\r']))
        .ok_or_else(|| KimlikHata::BozukYanit("'oturum' alanı yok ya da geçersiz".into()))?
        .to_string();
    Ok(Oturum {
        sunucu: sunucu.trim().trim_end_matches('/').to_string(),
        kullanici: kullanici.to_string(),
        sube_id: yanit.get("sube_id").cloned().unwrap_or(Value::Null),
        roller: roller_coz(yanit.get("roller")),
        izinler: roller_coz(yanit.get("izinler")),
        programlar: roller_coz(yanit.get("programlar")),
        gecerlilik: yanit
            .get("gecerlilik")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        zaman: taa_kuyruk::zaman::simdi(),
        oturum,
    })
}

/// Oturum deposu: sunucu başına bir oturum.
pub trait OturumDeposu {
    fn kaydet(&self, o: &Oturum) -> Result<(), KimlikHata>;
    fn oku(&self, sunucu: &str) -> Result<Oturum, KimlikHata>;
    /// Çıkış: kayıt kaldırılır (kayıt yoksa hata değildir).
    fn cikis(&self, sunucu: &str) -> Result<(), KimlikHata>;
}

/// Dosya deposu: `<klasör>/oturum-<sunucu>.json`, yalnız sahibi okur (Unix 0600). **Windows dışı test içindir**;
/// Windows'ta `KimlikDeposu` kullanılır.
#[derive(Debug, Clone)]
pub struct DosyaDeposu {
    klasor: PathBuf,
}

impl DosyaDeposu {
    pub fn yeni(klasor: impl Into<PathBuf>) -> DosyaDeposu {
        DosyaDeposu {
            klasor: klasor.into(),
        }
    }

    /// `<kök>/kimlik/`.
    pub fn kokten(kok: &Path) -> DosyaDeposu {
        DosyaDeposu::yeni(kok.join("kimlik"))
    }

    fn yol(&self, sunucu: &str) -> PathBuf {
        self.klasor
            .join(format!("oturum-{}.json", sunucu_anahtari(sunucu)))
    }
}

impl OturumDeposu for DosyaDeposu {
    fn kaydet(&self, o: &Oturum) -> Result<(), KimlikHata> {
        let yol = self.yol(&o.sunucu);
        std::fs::create_dir_all(&self.klasor).map_err(|e| KimlikHata::Depo(e.to_string()))?;
        taa_dosya::uzerine_yaz_atomik(&yol, o.depo_json().as_bytes())
            .map_err(|e| KimlikHata::Depo(e.to_string()))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&yol, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| KimlikHata::Depo(e.to_string()))?;
        }
        Ok(())
    }

    fn oku(&self, sunucu: &str) -> Result<Oturum, KimlikHata> {
        let yol = self.yol(sunucu);
        let metin = match std::fs::read_to_string(&yol) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(KimlikHata::OturumYok)
            }
            Err(e) => return Err(KimlikHata::Depo(e.to_string())),
        };
        if metin.trim().is_empty() || metin.trim() == "{}" {
            return Err(KimlikHata::OturumYok);
        }
        Oturum::depo_jsondan(&metin)
    }

    fn cikis(&self, sunucu: &str) -> Result<(), KimlikHata> {
        // Dosya silinmez (silme kuralı); içerik boşaltılır.
        let yol = self.yol(sunucu);
        if !yol.exists() {
            return Ok(());
        }
        taa_dosya::uzerine_yaz_atomik(&yol, b"{}").map_err(|e| KimlikHata::Depo(e.to_string()))
    }
}

/// Windows Kimlik Bilgisi Yöneticisi (genel kimlik bilgisi, hedef `TAA:<sunucu>`, yerel makine kalıcılığı).
#[cfg(windows)]
#[derive(Debug, Clone, Default)]
pub struct KimlikDeposu;

#[cfg(windows)]
mod kimlik_yoneticisi {
    use super::*;
    use windows_sys::Win32::Foundation::{GetLastError, ERROR_NOT_FOUND};
    use windows_sys::Win32::Security::Credentials::{
        CredDeleteW, CredFree, CredReadW, CredWriteW, CREDENTIALW, CRED_PERSIST_LOCAL_MACHINE,
        CRED_TYPE_GENERIC,
    };

    fn genis(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    pub fn hedef(sunucu: &str) -> String {
        format!("TAA:{}", sunucu_anahtari(sunucu))
    }

    fn son_hata(ne: &str) -> KimlikHata {
        // GÜVENLİ: yalnız son hata kodunu okur.
        let k = unsafe { GetLastError() };
        KimlikHata::Depo(format!("{ne} (Windows hata {k})"))
    }

    impl OturumDeposu for KimlikDeposu {
        fn kaydet(&self, o: &Oturum) -> Result<(), KimlikHata> {
            let mut hedef_w = genis(&hedef(&o.sunucu));
            let mut kullanici_w = genis(&o.kullanici);
            let mut yorum_w = genis("TAA merkez sunucu oturumu (taa-kimlik)");
            let mut blob = o.depo_json().into_bytes();
            if blob.len() > 2560 {
                return Err(KimlikHata::Depo("oturum kaydı 2560 baytı aşıyor".into()));
            }
            let k = CREDENTIALW {
                Type: CRED_TYPE_GENERIC,
                TargetName: hedef_w.as_mut_ptr(),
                Comment: yorum_w.as_mut_ptr(),
                CredentialBlobSize: blob.len() as u32,
                CredentialBlob: blob.as_mut_ptr(),
                Persist: CRED_PERSIST_LOCAL_MACHINE,
                UserName: kullanici_w.as_mut_ptr(),
                ..Default::default()
            };
            // GÜVENLİ: tüm işaretçiler bu kapsamda yaşayan tamponlara; CredWriteW kopyalar.
            let ok = unsafe { CredWriteW(&k, 0) };
            if ok == 0 {
                return Err(son_hata("Kimlik Bilgisi Yöneticisi'ne yazılamadı"));
            }
            Ok(())
        }

        fn oku(&self, sunucu: &str) -> Result<Oturum, KimlikHata> {
            let hedef_w = genis(&hedef(sunucu));
            let mut p: *mut CREDENTIALW = std::ptr::null_mut();
            // GÜVENLİ: CredReadW başarıda p'yi doldurur; CredFree ile bırakılır.
            let ok = unsafe { CredReadW(hedef_w.as_ptr(), CRED_TYPE_GENERIC, 0, &mut p) };
            if ok == 0 {
                let k = unsafe { GetLastError() };
                return Err(if k == ERROR_NOT_FOUND {
                    KimlikHata::OturumYok
                } else {
                    KimlikHata::Depo(format!(
                        "Kimlik Bilgisi Yöneticisi okunamadı (Windows hata {k})"
                    ))
                });
            }
            let sonuc = unsafe {
                let k = &*p;
                let blob =
                    std::slice::from_raw_parts(k.CredentialBlob, k.CredentialBlobSize as usize);
                String::from_utf8(blob.to_vec())
            };
            unsafe { CredFree(p as *const _) };
            let metin = sonuc.map_err(|_| KimlikHata::Depo("kayıt UTF-8 değil".into()))?;
            Oturum::depo_jsondan(&metin)
        }

        fn cikis(&self, sunucu: &str) -> Result<(), KimlikHata> {
            let hedef_w = genis(&hedef(sunucu));
            // GÜVENLİ: yalnız hedef adı verilir.
            let ok = unsafe { CredDeleteW(hedef_w.as_ptr(), CRED_TYPE_GENERIC, 0) };
            if ok == 0 {
                let k = unsafe { GetLastError() };
                if k == ERROR_NOT_FOUND {
                    return Ok(());
                }
                return Err(KimlikHata::Depo(format!(
                    "Kimlik Bilgisi Yöneticisi kaydı kaldırılamadı (Windows hata {k})"
                )));
            }
            Ok(())
        }
    }
}

/// Platformun varsayılan deposu: Windows'ta Kimlik Bilgisi Yöneticisi, diğerlerinde `<kök>/kimlik/` dosyası (test).
pub fn varsayilan_depo(kok: &Path) -> Box<dyn OturumDeposu> {
    #[cfg(windows)]
    {
        let _ = kok;
        Box::new(KimlikDeposu)
    }
    #[cfg(not(windows))]
    {
        Box::new(DosyaDeposu::kokten(kok))
    }
}

/// Depodaki oturumla `<kök>/kuyruk/` altındaki tüm program kuyruklarını gönderir.
/// Dönen: (program, özet) listesi; sunucuya ulaşılamayan ilk kuyrukta durur.
pub fn kuyrugu_gonder(
    depo: &dyn OturumDeposu,
    sunucu: &str,
    kok: &Path,
    sertifika: Option<&Path>,
    en_cok: usize,
) -> Result<Vec<(String, taa_kuyruk::GonderOzeti)>, KimlikHata> {
    kuyrugu_gonder_hedef(depo, sunucu, sunucu, kok, sertifika, en_cok)
}

/// `anahtar` depo anahtarı (mantıksal ad ya da adres), `taban` istek adresi (taa-yol ile seçilen yol).
pub fn kuyrugu_gonder_hedef(
    depo: &dyn OturumDeposu,
    anahtar: &str,
    taban: &str,
    kok: &Path,
    sertifika: Option<&Path>,
    en_cok: usize,
) -> Result<Vec<(String, taa_kuyruk::GonderOzeti)>, KimlikHata> {
    let o = depo.oku(anahtar)?;
    let g = o.gonderici_taban(taban, sertifika, cihaz_imzalayici(kok)?)?;
    let mut sonuc = Vec::new();
    for (ad, k) in taa_kuyruk::Kuyruk::hepsi(kok).map_err(|e| KimlikHata::Depo(e.to_string()))? {
        let oz = k
            .gonder(&g, en_cok)
            .map_err(|e| KimlikHata::Depo(e.to_string()))?;
        let dur = oz.durma_nedeni.is_some();
        sonuc.push((ad, oz));
        if dur {
            break;
        }
    }
    Ok(sonuc)
}
