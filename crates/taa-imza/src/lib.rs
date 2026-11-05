// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! taa-imza: ed25519 imzalama ve doğrulama (docs/T3-TASARIM.md §1).
//! Genel anahtar dosyası: tek satır `ed25519:<64 onaltılık hane>` (`#` ile başlayan satırlar yorum).
//! Özel anahtar dosyası: tek satır `ed25519-ozel:<64 onaltılık hane>`; YALNIZ çağıranın verdiği dosyadan okunur,
//! hiçbir günlüğe, hata iletisine ya da `Debug` çıktısına yazılmaz ve depoya konmaz. Dosya yeni oluşturulur (üzerine yazılmaz),
//! Unix'te 0600 izniyle; başkasının okuyabildiği dosya okunurken uyarı verilir.
//! Rastgelelik: ek crate yok; Unix'te `/dev/urandom`, Windows'ta `RtlGenRandom` (SystemFunction036).

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::Path;

pub const ALGORITMA: &str = "ed25519";
const GENEL_ON_EK: &str = "ed25519:";
const OZEL_ON_EK: &str = "ed25519-ozel:";

// ---------------------------------------------------------------- hata

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImzaHata {
    /// Bicim, uzunluk ya da algoritma hatası (girdi bozuk).
    Bicim(String),
    /// İmza ya da anahtar eşleşmiyor.
    Dogrulama(String),
    /// Dosya/rastgelelik hatası.
    Dosya(String),
}

impl ImzaHata {
    pub fn mesaj(&self) -> &str {
        match self {
            ImzaHata::Bicim(m) | ImzaHata::Dogrulama(m) | ImzaHata::Dosya(m) => m,
        }
    }
}

impl std::fmt::Display for ImzaHata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.mesaj())
    }
}

impl std::error::Error for ImzaHata {}

// ---------------------------------------------------------------- hex

pub fn hex_kodla(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Küçük harf onaylı hex çözer; tek sayıda hane ya da hex olmayan karakter hata verir.
pub fn hex_coz(s: &str) -> Result<Vec<u8>, ImzaHata> {
    let b = s.as_bytes();
    if !b.len().is_multiple_of(2) {
        return Err(ImzaHata::Bicim(
            "Onaltılık metnin hane sayısı çift olmalı.".into(),
        ));
    }
    let hane = |c: u8| match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(ImzaHata::Bicim(
            "Onaltılık metinde geçersiz karakter var.".into(),
        )),
    };
    b.chunks(2)
        .map(|p| Ok(hane(p[0])? << 4 | hane(p[1])?))
        .collect()
}

fn sabit<const N: usize>(s: &str, ne: &str) -> Result<[u8; N], ImzaHata> {
    hex_coz(s)?
        .try_into()
        .map_err(|_| ImzaHata::Bicim(format!("{ne} {N} bayt (onaltılık {}) olmalı.", N * 2)))
}

// ---------------------------------------------------------------- imza alanı

/// `paket.json` ve `yayin.json` içindeki `imza` alanı.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Imza {
    pub alg: String,
    pub anahtar_kimlik: String,
    /// 64 baytlık imza, onaltılık.
    pub deger: String,
}

/// Bir JSON değerinin kanonik baytları: nesne anahtarları sıralı, boşluksuz (serde_json varsayılanı; `preserve_order` kapalı).
/// `imza` alanı varsa çıkarılır: `yayin.json` imzası bu bayt dizisi üzerindedir.
pub fn kanonik_bayt(v: &serde_json::Value) -> Vec<u8> {
    let mut k = v.clone();
    if let Some(o) = k.as_object_mut() {
        o.remove("imza");
    }
    serde_json::to_vec(&k).unwrap_or_default()
}

// ---------------------------------------------------------------- genel anahtar

#[derive(Clone, PartialEq, Eq)]
pub struct GenelAnahtar(VerifyingKey);

impl std::fmt::Debug for GenelAnahtar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GenelAnahtar({})", self.kimlik())
    }
}

impl GenelAnahtar {
    pub fn bayt(&self) -> [u8; 32] {
        self.0.to_bytes()
    }

    pub fn hex(&self) -> String {
        hex_kodla(&self.bayt())
    }

    /// Anahtar kimliği: genel anahtarın ilk 8 baytı (16 hane). Güvenlik kimlikten değil, tam anahtardan gelir.
    pub fn kimlik(&self) -> String {
        hex_kodla(&self.bayt()[..8])
    }

    /// Dosya biçimi: `ed25519:<hex>`.
    pub fn metin(&self) -> String {
        format!("{GENEL_ON_EK}{}", self.hex())
    }

    /// `ed25519:<hex>` metnini çözer. Geçersiz eğri noktası da reddedilir.
    pub fn coz(metin: &str) -> Result<GenelAnahtar, ImzaHata> {
        let satir = metin
            .lines()
            .map(str::trim)
            .find(|s| !s.is_empty() && !s.starts_with('#'))
            .ok_or_else(|| ImzaHata::Bicim("Genel anahtar dosyası boş.".into()))?;
        let hex = satir.strip_prefix(GENEL_ON_EK).ok_or_else(|| {
            ImzaHata::Bicim(
                "Genel anahtar `ed25519:<hex>` biçiminde olmalı (algoritma öneki yok ya da başka algoritma).".into(),
            )
        })?;
        let b: [u8; 32] = sabit(hex, "Genel anahtar")?;
        VerifyingKey::from_bytes(&b)
            .map(GenelAnahtar)
            .map_err(|_| ImzaHata::Bicim("Genel anahtar geçerli bir ed25519 noktası değil.".into()))
    }

    pub fn dosyadan(yol: &Path) -> Result<GenelAnahtar, ImzaHata> {
        let m = fs::read_to_string(yol).map_err(|e| {
            ImzaHata::Dosya(format!(
                "Genel anahtar dosyası okunamadı ({}): {e}.",
                yol.display()
            ))
        })?;
        Self::coz(&m)
    }

    /// `veri` üzerindeki `imza` değerini bu anahtarla doğrular (`verify_strict`: şekil değiştirilebilir imzalar reddedilir).
    pub fn dogrula(&self, veri: &[u8], imza: &Imza) -> Result<(), ImzaHata> {
        if imza.alg != ALGORITMA {
            return Err(ImzaHata::Dogrulama(format!(
                "İmza algoritması desteklenmiyor: {:?}.",
                imza.alg
            )));
        }
        let b: [u8; 64] = sabit(&imza.deger, "İmza")?;
        self.0
            .verify_strict(veri, &Signature::from_bytes(&b))
            .map_err(|_| ImzaHata::Dogrulama("İmza bu anahtarla doğrulanamadı.".into()))
    }
}

// ---------------------------------------------------------------- özel anahtar

/// Özel anahtar: `Debug`/`Display` yok (yalnız sabit bir ibare), düşerken sıfırlanmaya çalışılır.
pub struct OzelAnahtar(SigningKey);

impl std::fmt::Debug for OzelAnahtar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("OzelAnahtar(gizli)")
    }
}

impl Drop for OzelAnahtar {
    fn drop(&mut self) {
        // En iyi çaba: ed25519-dalek `zeroize` özelliği kapalı; tohumu kendi kopyamızda sıfırlayamayız,
        // yeni bir sıfır anahtarla değiştirerek eski değerin bu yığın konumundan kalkmasını sağlarız.
        self.0 = SigningKey::from_bytes(&[0u8; 32]);
    }
}

impl OzelAnahtar {
    pub fn genel(&self) -> GenelAnahtar {
        GenelAnahtar(self.0.verifying_key())
    }

    pub fn imzala(&self, veri: &[u8]) -> Imza {
        let g = self.genel();
        Imza {
            alg: ALGORITMA.into(),
            anahtar_kimlik: g.kimlik(),
            deger: hex_kodla(&self.0.sign(veri).to_bytes()),
        }
    }

    /// Verilen dosyadan okur. Unix'te başkasının okuyabildiği dosya için uyarı metni döner (çağıran stderr'e yazar).
    pub fn dosyadan(yol: &Path) -> Result<(OzelAnahtar, Option<String>), ImzaHata> {
        let m = fs::read_to_string(yol).map_err(|e| {
            ImzaHata::Dosya(format!(
                "Özel anahtar dosyası okunamadı ({}): {e}.",
                yol.display()
            ))
        })?;
        let hex = m.trim().strip_prefix(OZEL_ON_EK).ok_or_else(|| {
            ImzaHata::Bicim("Özel anahtar dosyası `ed25519-ozel:<hex>` biçiminde olmalı.".into())
        })?;
        let b: [u8; 32] = sabit(hex, "Özel anahtar")?;
        Ok((OzelAnahtar(SigningKey::from_bytes(&b)), izin_uyarisi(yol)))
    }
}

#[cfg(unix)]
fn izin_uyarisi(yol: &Path) -> Option<String> {
    use std::os::unix::fs::PermissionsExt;
    let kip = fs::metadata(yol).ok()?.permissions().mode();
    (kip & 0o077 != 0).then(|| {
        format!(
            "Uyarı: özel anahtar dosyasına başkaları erişebiliyor (izin {:o}). Öneri: chmod 600 {}.",
            kip & 0o777,
            yol.display()
        )
    })
}

/// Windows'ta POSIX izin kipi yoktur; her imzalamada aynı uyarıyı basmak yerine sessiz kalınır (yardım metni açıklar).
#[cfg(not(unix))]
fn izin_uyarisi(_yol: &Path) -> Option<String> {
    None
}

// ---------------------------------------------------------------- anahtar üretimi

#[cfg(unix)]
fn rastgele_doldur(b: &mut [u8]) -> std::io::Result<()> {
    use std::io::Read;
    fs::File::open("/dev/urandom")?.read_exact(b)
}

#[cfg(windows)]
fn rastgele_doldur(b: &mut [u8]) -> std::io::Result<()> {
    #[link(name = "advapi32")]
    extern "system" {
        // RtlGenRandom
        fn SystemFunction036(tampon: *mut u8, uzunluk: u32) -> u8;
    }
    // SAFETY: tampon geçerli ve `uzunluk` bayt yazılabilir; işlev yalnız bu aralığa yazar.
    let ok = unsafe { SystemFunction036(b.as_mut_ptr(), b.len() as u32) };
    if ok == 0 {
        Err(std::io::Error::other("RtlGenRandom başarısız"))
    } else {
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
fn rastgele_doldur(_b: &mut [u8]) -> std::io::Result<()> {
    Err(std::io::Error::other(
        "bu platformda rastgelelik kaynağı yok",
    ))
}

fn yeni_dosya(yol: &Path, icerik: &str, gizli: bool) -> Result<(), ImzaHata> {
    let mut oz = fs::OpenOptions::new();
    oz.write(true).create_new(true);
    #[cfg(unix)]
    if gizli {
        use std::os::unix::fs::OpenOptionsExt;
        oz.mode(0o600);
    }
    #[cfg(not(unix))]
    let _ = gizli;
    if let Some(d) = yol.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(d).map_err(|e| ImzaHata::Dosya(format!("Klasör açılamadı: {e}.")))?;
    }
    let mut f = oz.open(yol).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            ImzaHata::Dosya(format!(
                "{} zaten var; üzerine yazılmaz. Öneri: başka bir yol verin.",
                yol.display()
            ))
        } else {
            ImzaHata::Dosya(format!("{} yazılamadı: {e}.", yol.display()))
        }
    })?;
    f.write_all(icerik.as_bytes())
        .and_then(|_| f.sync_all())
        .map_err(|e| ImzaHata::Dosya(format!("{} yazılamadı: {e}.", yol.display())))
}

/// Yeni anahtar çifti üretir; özel anahtarı `ozel_yol` (0600, üzerine yazmaz), genel anahtarı `genel_yol` dosyasına yazar.
/// Özel anahtar hiçbir yere yazdırılmaz; dönen değer yalnız genel anahtardır.
pub fn anahtar_uret(ozel_yol: &Path, genel_yol: &Path) -> Result<GenelAnahtar, ImzaHata> {
    if ozel_yol == genel_yol {
        return Err(ImzaHata::Dosya(
            "Özel ve genel anahtar aynı dosyaya yazılamaz.".into(),
        ));
    }
    let mut tohum = [0u8; 32];
    rastgele_doldur(&mut tohum)
        .map_err(|e| ImzaHata::Dosya(format!("Rastgelelik kaynağı okunamadı: {e}.")))?;
    let ozel = OzelAnahtar(SigningKey::from_bytes(&tohum));
    let genel = ozel.genel();
    yeni_dosya(
        ozel_yol,
        &format!("{OZEL_ON_EK}{}\n", hex_kodla(&tohum)),
        true,
    )?;
    tohum = [0u8; 32];
    let _ = tohum;
    yeni_dosya(genel_yol, &format!("{}\n", genel.metin()), false)?;
    Ok(genel)
}

// ---------------------------------------------------------------- belge imzalama (paket.json / yayin.json)

/// `paket.json` değerini imzalar: imzalanan veri `icerik_sha256` metnidir (UTF-8); sonuç `imza` alanına yazılır.
pub fn paket_imzala(v: &mut serde_json::Value, o: &OzelAnahtar) -> Result<(), ImzaHata> {
    let ozet = v
        .get("icerik_sha256")
        .and_then(|x| x.as_str())
        .ok_or_else(|| ImzaHata::Bicim("paket.json içinde icerik_sha256 metni yok.".into()))?
        .to_string();
    let im = o.imzala(ozet.as_bytes());
    v["imza"] = serde_json::to_value(im).unwrap_or_default();
    Ok(())
}

/// `yayin.json` değerini imzalar: imzalanan veri [`kanonik_bayt`] (imza alanı hariç).
pub fn yayin_imzala(v: &mut serde_json::Value, o: &OzelAnahtar) -> Result<(), ImzaHata> {
    if !v.is_object() {
        return Err(ImzaHata::Bicim("yayin.json bir nesne olmalı.".into()));
    }
    let im = o.imzala(&kanonik_bayt(v));
    v["imza"] = serde_json::to_value(im).unwrap_or_default();
    Ok(())
}

/// Belgedeki `imza` alanını okur.
pub fn imza_oku(v: &serde_json::Value) -> Result<Imza, ImzaHata> {
    let i = v
        .get("imza")
        .filter(|i| !i.is_null())
        .ok_or_else(|| ImzaHata::Dogrulama("Belgede imza yok.".into()))?;
    serde_json::from_value(i.clone()).map_err(|_| {
        ImzaHata::Bicim("imza alanı {alg, anahtar_kimlik, deger} biçiminde değil.".into())
    })
}

// ---------------------------------------------------------------- zip paket (paket.json zipin kökünde)

/// Yol `.zip` ile bitiyorsa (harfe duyarsız) ve klasör değilse zip paketi sayılır.
pub fn zip_mi(yol: &Path) -> bool {
    yol.extension()
        .is_some_and(|u| u.eq_ignore_ascii_case("zip"))
        && !yol.is_dir()
}

fn zip_hatasi(yol: &Path, e: impl std::fmt::Display) -> ImzaHata {
    ImzaHata::Dosya(format!(
        "Zip paketi okunamadı ({}): {e}. Öneri: dosyanın tools/paket_uret.py --zip ile üretildiğini denetleyin.",
        yol.display()
    ))
}

const MAKS_PAKET_JSON: u64 = 16 * 1024 * 1024;

/// Zip paketinin kökündeki `paket.json` değerini okur (zip açılmaz, diske yazılmaz).
pub fn zip_paket_json_oku(yol: &Path) -> Result<serde_json::Value, ImzaHata> {
    use std::io::Read;
    let f = fs::File::open(yol).map_err(|e| zip_hatasi(yol, e))?;
    let mut arsiv =
        zip::ZipArchive::new(std::io::BufReader::new(f)).map_err(|e| zip_hatasi(yol, e))?;
    let g = arsiv.by_name("paket.json").map_err(|_| {
        ImzaHata::Bicim(format!(
            "Zip kökünde paket.json yok ({}). Öneri: paket.json zipin kökünde olmalı.",
            yol.display()
        ))
    })?;
    if g.size() > MAKS_PAKET_JSON {
        return Err(ImzaHata::Bicim("paket.json çok büyük.".into()));
    }
    let mut b = Vec::with_capacity(g.size() as usize);
    g.take(MAKS_PAKET_JSON + 1)
        .read_to_end(&mut b)
        .map_err(|e| zip_hatasi(yol, e))?;
    serde_json::from_slice(&b)
        .map_err(|e| ImzaHata::Bicim(format!("paket.json geçerli JSON değil: {e}.")))
}

/// Zip paketini kökteki `paket.json` yenilenmiş olarak yeniden yazar. Öteki girdiler bayt bayt (ham) kopyalanır,
/// sırası korunur. Eski zip silinmez: `<zip>.once-N` kopyası alınır, sonra yeni içerik atomik yazılır.
pub fn zip_paket_json_yaz(yol: &Path, v: &serde_json::Value) -> Result<(), ImzaHata> {
    use std::io::Write;
    let eski = fs::read(yol).map_err(|e| zip_hatasi(yol, e))?;
    let mut arsiv =
        zip::ZipArchive::new(std::io::Cursor::new(&eski[..])).map_err(|e| zip_hatasi(yol, e))?;
    let mut yeni = zip::ZipWriter::new(std::io::Cursor::new(Vec::with_capacity(eski.len())));
    let mut yazildi = false;
    for i in 0..arsiv.len() {
        let g = arsiv.by_index_raw(i).map_err(|e| zip_hatasi(yol, e))?;
        let ad = g.name().map_err(|e| zip_hatasi(yol, e))?.into_owned();
        if ad == "paket.json" && !g.is_dir() {
            let sec = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated)
                .last_modified_time(g.last_modified().unwrap_or_default())
                .unix_permissions(0o644);
            let metin = serde_json::to_string_pretty(v)
                .map_err(|e| ImzaHata::Bicim(format!("paket.json yazılamadı: {e}.")))?;
            yeni.start_file("paket.json", sec)
                .and_then(|_| {
                    yeni.write_all(metin.as_bytes())
                        .and_then(|_| yeni.write_all(b"\n"))
                        .map_err(Into::into)
                })
                .map_err(|e| zip_hatasi(yol, e))?;
            yazildi = true;
        } else {
            yeni.raw_copy_file(g).map_err(|e| zip_hatasi(yol, e))?;
        }
    }
    if !yazildi {
        return Err(ImzaHata::Bicim(format!(
            "Zip kökünde paket.json yok ({}). Öneri: paket.json zipin kökünde olmalı.",
            yol.display()
        )));
    }
    let bayt = yeni.finish().map_err(|e| zip_hatasi(yol, e))?.into_inner();
    let mut yedek = None;
    for n in 1..1000 {
        let y = std::path::PathBuf::from(format!("{}.once-{n}", yol.display()));
        if !y.exists() {
            yedek = Some(y);
            break;
        }
    }
    let yedek = yedek.ok_or_else(|| ImzaHata::Dosya("Çok fazla .once-N yedeği var.".into()))?;
    fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&yedek)
        .and_then(|mut f| f.write_all(&eski))
        .map_err(|e| {
            ImzaHata::Dosya(format!(
                "{} yazılamadı (eski zip ezilmez): {e}.",
                yedek.display()
            ))
        })?;
    taa_dosya::uzerine_yaz_atomik(yol, &bayt)
        .map_err(|e| ImzaHata::Dosya(format!("{} yazılamadı: {e}.", yol.display())))
}

#[cfg(test)]
mod testler;
