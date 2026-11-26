//! Teslim alan bilgisinin yerel şifreli saklanması (Yusuf 2026-11-26: "Bu bilgiler veri tabanında şifreli olarak
//! tutulsun").
//!
//! - Defterdeki teslim kaydına (JSONL) ad, telefon, cihaz (user-agent) ve IP DÜZ METİN olarak yazılmaz; hepsi tek bir
//!   JSON olarak AES-256-GCM ile şifrelenir (`TeslimSifreli.c`), ek doğrulama verisi (aad) fiş numarasıdır: şifreli
//!   blok başka bir fişin kaydına taşınırsa çözülmez.
//! - Anahtar ayrı dosyadadır: varsayılan yeri veri klasöründe `teslim-sifre.anahtar` (ilk kullanımda rastgele 32
//!   bayt, içerik `aes256-yerel:<hex>`). Sunucu `--teslim-anahtar YOL` ile başlatılırsa anahtar o dosyadadır (ör.
//!   veri klasörü dışında, ayrı yedeklenen bir yer; `D:\YusufII\HAFIZAM1\anahtarlar`'a DOKUNULMAZ). Unix'te izin
//!   0600; Windows'ta izin daraltması YOK (DPAPI sarması yapılmadı). DİKKAT: anahtar veri klasöründe durursa, veri
//!   klasörünü ya da yedeğini ele geçiren anahtarı da alır; şifreleme yalnız defter dosyası tek başına sızarsa korur.
//! - Anahtar kaybına karşı: defterde şifreli teslim kaydı varken anahtar dosyası yoksa YENİ anahtar üretilmez,
//!   teslim açık bir hatayla durur (eski kayıtlar sessizce okunamaz hâle gelmesin); çözme hiçbir zaman anahtar üretmez.
//!   Yedekte anahtar dosyası ayrıca (ve veriden ayrı yerde) saklanmalıdır.
//! - Kör indeks: `ad_indeks`/`tel_indeks` = HMAC-SHA256(ayrı türetilmiş anahtar, normalize ad / telefon) ilk 16
//!   onaltılık hane. Yönetici arama ucu (`GET /api/yonetim/teslim-ara`) bununla fiş bulur; değer geri çevrilemez.
//! - Anahtarın yeri ve yedeği ŞİMDİLİK DEĞİŞMEZ (Yusuf 2026-11-26: en son, sunucuya kurulumda sorulacak): kuruluma
//!   geçerken yer (veri klasörü içi/dışı, `--teslim-anahtar`) ve ayrı yedek yeri yöneticiye sorulacak.
//! - Aynı anahtar imza dosyalarını (`imzalar/<no>-<n>.png.enc`, aad = fiş no) ve geçiş yedeklerini
//!   (`yedek/*.jsonl.enc`, aad = "yedek:<dosya adı>") da şifreler (`bayt_sifrele` / `bayt_coz`); anahtar kaybolursa
//!   bunlar da okunamaz.
//! - Bu modül hiçbir şeyi günlüğe yazmaz.
//!
//! Sürüm 2 (denetçi bulgusu 2026-11-26: tek anahtar her türde doğrudan kullanılıyor, aad yalnız fiş no'ydu):
//! - Veri türüne göre ALT ANAHTAR: HKDF-SHA256(ikm = ana anahtar, tuz yok, info = "taa-teslim-bilgi-v2" teslim
//!   bilgisi için, "taa-teslim-imza-v2" imza dosyası için). Bir türün şifreli metni öbür türün anahtarıyla çözülmez.
//! - aad = "taa-teslim-v2\n<tür>\n<fiş no>\n<teslim_id>": şifreli blok başka fişe, başka teslimata (teslim_id)
//!   ya da başka türe taşınırsa çözülmez. Bağlı fiş imzasında fiş no ana fiştir (dosya adıyla aynı).
//! - Yeni kayıtlar v2 yazılır: teslim bilgisi `TeslimSifreli.surum = 2`, imza dosyası başlığı `TAAENC2\n`
//!   (`DOSYA_BASLIGI_V2`). Eski v1 kayıtlar (surum 1 / `TAAENC1\n`, ana anahtar doğrudan, aad = fiş no) eski yolla
//!   çözülür; yeniden yazılmazlar.
//! - Defter yedekleri ve geçişin eski şifresiz imzalardan ürettiği kopyalar (`bayt_sifrele`) şimdilik v1 biçimini
//!   korur: geçiş kopyası hangi teslimata ait olduğunu bilmeden yazılır (eski veri, eski biçim).
//!
//! Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

use crate::musteri_takip::{dosya_ya_da_olustur, hex, hex_coz, hmac_sha256, rastgele};
use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Nonce};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use taa_fis::TeslimSifreli;

/// Veri klasöründeki yerel şifre anahtarı dosyası.
pub const ANAHTAR_DOSYASI: &str = "teslim-sifre.anahtar";
const ONEK: &str = "aes256-yerel:";

/// Şifrelenen düz bilgi. Yalnız bellekte durur.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeslimDuz {
    pub ad: String,
    /// "+<rakamlar>" ya da (ekran imzası yolunda) boş.
    #[serde(default)]
    pub tel: String,
    /// Okutulan telefonun user-agent'ı (en çok 200 karakter); ekran yolunda boş.
    #[serde(default)]
    pub cihaz: String,
    #[serde(default)]
    pub ip: String,
    /// Onay (QR) ya da teslim (ekran) anı, unix saniyesi.
    #[serde(default)]
    pub zaman: u64,
    /// "qr", "ekran-imza" ya da "elle" (fişli, imzasız).
    pub yontem: String,
}

/// Yerel anahtar dosyası: unix'te yalnız sahibi okuyabilir.
fn izin_daralt(yol: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(yol, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = yol;
}

/// `--teslim-anahtar YOL` ile verilen anahtar dosyası (main.rs bir kez ayarlar; verilmezse veri klasöründeki).
static ANAHTAR_YERI: OnceLock<PathBuf> = OnceLock::new();

/// Anahtar dosyasının yerini veri klasörü dışına alır (yalnız açılışta, bir kez).
pub fn anahtar_yeri_ayarla(yol: PathBuf) -> Result<(), String> {
    ANAHTAR_YERI
        .set(yol)
        .map_err(|_| "teslim anahtarı yeri zaten ayarlı".to_string())
}

/// Kullanılan anahtar dosyası.
pub fn anahtar_yolu(dizin: &Path) -> PathBuf {
    ANAHTAR_YERI
        .get()
        .cloned()
        .unwrap_or_else(|| dizin.join(ANAHTAR_DOSYASI))
}

/// Defterde şifreli teslim kaydı ya da şifreli imza/yedek dosyası var mı? (anahtar kaybını sessizce yeni
/// anahtarla örtmemek için)
pub fn sifreli_kayit_var(dizin: &Path) -> bool {
    let enc_var = |alt: &str| {
        std::fs::read_dir(dizin.join(alt))
            .map(|r| {
                r.flatten()
                    .any(|e| e.file_name().to_string_lossy().ends_with(".enc"))
            })
            .unwrap_or(false)
    };
    std::fs::read(dizin.join("fisler.jsonl"))
        .map(|b| b.windows(11).any(|w| w == b"\"sifreli\":{"))
        .unwrap_or(false)
        || enc_var("imzalar")
        || enc_var(YEDEK_KLASORU)
}

/// Geçiş yedeklerinin klasörü (veri klasöründe). İçindeki dosyalar `bayt_sifrele` ile şifrelidir.
pub const YEDEK_KLASORU: &str = "yedek";
/// Şifreli dosya (imza `.png.enc`, defter yedeği `.jsonl.enc`) başlığı. Ardından 12 bayt IV ve
/// AES-256-GCM şifreli metin + 16 bayt etiket gelir.
pub const DOSYA_BASLIGI: &[u8; 8] = b"TAAENC1\n";
/// Sürüm 2 imza dosyası başlığı (sürüm baytı '2'): alt anahtar + tür/fiş/teslim_id aad'si. Gövde v1 ile aynı düzen.
pub const DOSYA_BASLIGI_V2: &[u8; 8] = b"TAAENC2\n";

/// Sürüm 2'de şifrelenen veri türü: her türün ayrı alt anahtarı ve aad etiketi vardır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VeriTuru {
    /// Teslim alan bilgisi (`TeslimSifreli`).
    Bilgi,
    /// Teslim imzası dosyası (`imzalar/<no>-<n>.png.enc`).
    Imza,
}

impl VeriTuru {
    fn hkdf_bilgi(self) -> &'static [u8] {
        match self {
            VeriTuru::Bilgi => b"taa-teslim-bilgi-v2",
            VeriTuru::Imza => b"taa-teslim-imza-v2",
        }
    }
    fn ad(self) -> &'static str {
        match self {
            VeriTuru::Bilgi => "bilgi",
            VeriTuru::Imza => "imza",
        }
    }
}

/// Sürüm 2 alt anahtarı: HKDF-SHA256(ikm = ana, tuz yok, info = türün bilgi dizesi) -> 32 bayt.
fn alt_anahtar(ana: &[u8; 32], tur: VeriTuru) -> Result<[u8; 32], String> {
    let mut k = [0u8; 32];
    hkdf::Hkdf::<sha2::Sha256>::new(None, ana)
        .expand(tur.hkdf_bilgi(), &mut k)
        .map_err(|_| "hkdf")?;
    Ok(k)
}

/// Sürüm 2 aad'si: tür + fiş no + teslim_id (satır sonlarıyla ayrılır; fiş no ve teslim_id satır sonu içermez).
fn aad_v2(tur: VeriTuru, fis_no: &str, teslim_id: &str) -> Vec<u8> {
    format!("taa-teslim-v2\n{}\n{fis_no}\n{teslim_id}", tur.ad()).into_bytes()
}

/// AES-256-GCM tek blok: (iv, şifreli + etiket).
fn gcm_sifrele(anahtar: &[u8; 32], aad: &[u8], duz: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
    let iv = rastgele(12)?;
    let c = Aes256Gcm::new_from_slice(anahtar)
        .map_err(|_| "anahtar uzunluğu")?
        .encrypt(Nonce::from_slice(&iv), Payload { msg: duz, aad })
        .map_err(|_| "şifrelenemedi")?;
    Ok((iv, c))
}

fn gcm_coz(anahtar: &[u8; 32], aad: &[u8], iv: &[u8], c: &[u8]) -> Option<Vec<u8>> {
    if iv.len() != 12 {
        return None;
    }
    Aes256Gcm::new_from_slice(anahtar)
        .ok()?
        .decrypt(Nonce::from_slice(iv), Payload { msg: c, aad })
        .ok()
}

/// Teslim imzasını sürüm 2 ile şifreler (`DOSYA_BASLIGI_V2` + IV + şifreli metin). `sahip_no` imzanın dosya
/// adındaki fiş (bağlı fişte ana fiş), `teslim_id` teslimatın kimliği. Anahtar yoksa `sifrele` kuralıyla oluşturulur.
pub fn imza_sifrele(
    dizin: &Path,
    sahip_no: &str,
    teslim_id: &str,
    png: &[u8],
) -> Result<Vec<u8>, String> {
    let k = alt_anahtar(&ana_anahtar(dizin)?, VeriTuru::Imza)?;
    let (iv, c) = gcm_sifrele(&k, &aad_v2(VeriTuru::Imza, sahip_no, teslim_id), png)?;
    let mut v = Vec::with_capacity(DOSYA_BASLIGI_V2.len() + 12 + c.len());
    v.extend_from_slice(DOSYA_BASLIGI_V2);
    v.extend_from_slice(&iv);
    v.extend_from_slice(&c);
    Ok(v)
}

/// İmza dosyasını çözer (anahtar üretmez): `TAAENC2` ise alt anahtar + v2 aad, `TAAENC1` (eski) ise ana anahtar +
/// aad = fiş no. Başlık, aad ya da anahtar uyuşmazsa hata.
pub fn imza_coz(
    dizin: &Path,
    sahip_no: &str,
    teslim_id: &str,
    veri: &[u8],
) -> Result<Vec<u8>, String> {
    match veri.strip_prefix(DOSYA_BASLIGI_V2.as_slice()) {
        Some(g) if g.len() >= 12 + 16 => {
            let k = alt_anahtar(&ana_anahtar_oku(dizin)?, VeriTuru::Imza)?;
            gcm_coz(
                &k,
                &aad_v2(VeriTuru::Imza, sahip_no, teslim_id),
                &g[..12],
                &g[12..],
            )
            .ok_or_else(|| {
                "şifreli imza çözülemedi (anahtar, bağlı fiş ya da teslimat uyuşmuyor)".into()
            })
        }
        Some(_) => Err("şifreli dosya biçimi tanınmadı".into()),
        None => bayt_coz(dizin, sahip_no.as_bytes(), veri),
    }
}

/// İkili veriyi (imza PNG'si, defter yedeği) yerel anahtarla tek blok AES-256-GCM ile şifreler.
/// `aad` dosyanın bağlı olduğu şeydir (imzada fiş no, yedekte "yedek:<dosya adı>"): dosya başka bir fişe/ada
/// taşınırsa çözülmez. Anahtar yoksa `sifrele` ile aynı kuralla oluşturulur (kayıp anahtar yeniden üretilmez).
pub fn bayt_sifrele(dizin: &Path, aad: &[u8], duz: &[u8]) -> Result<Vec<u8>, String> {
    let ana = ana_anahtar(dizin)?;
    let iv = rastgele(12)?;
    let sifreci = Aes256Gcm::new_from_slice(&ana).map_err(|_| "anahtar uzunluğu")?;
    let c = sifreci
        .encrypt(Nonce::from_slice(&iv), Payload { msg: duz, aad })
        .map_err(|_| "şifrelenemedi")?;
    let mut v = Vec::with_capacity(DOSYA_BASLIGI.len() + 12 + c.len());
    v.extend_from_slice(DOSYA_BASLIGI);
    v.extend_from_slice(&iv);
    v.extend_from_slice(&c);
    Ok(v)
}

/// `bayt_sifrele` ile şifrelenmiş veriyi çözer (anahtar üretmez). Başlık, aad ya da anahtar uyuşmazsa hata.
pub fn bayt_coz(dizin: &Path, aad: &[u8], veri: &[u8]) -> Result<Vec<u8>, String> {
    let govde = veri
        .strip_prefix(DOSYA_BASLIGI.as_slice())
        .filter(|g| g.len() >= 12 + 16)
        .ok_or("şifreli dosya biçimi tanınmadı")?;
    let ana = ana_anahtar_oku(dizin)?;
    let sifreci = Aes256Gcm::new_from_slice(&ana).map_err(|_| "anahtar uzunluğu")?;
    sifreci
        .decrypt(
            Nonce::from_slice(&govde[..12]),
            Payload {
                msg: &govde[12..],
                aad,
            },
        )
        .map_err(|_| "şifreli dosya çözülemedi (anahtar ya da bağlı fiş uyuşmuyor)".into())
}

fn anahtar_coz(m: &str) -> Result<[u8; 32], String> {
    m.trim()
        .strip_prefix(ONEK)
        .and_then(hex_coz)
        .and_then(|v| v.try_into().ok())
        .ok_or_else(|| "teslim şifre anahtarı bozuk".into())
}

/// Ana anahtarı yalnız OKUR (çözme yolu): dosya yoksa açık hata, yeni anahtar üretilmez.
fn ana_anahtar_oku(dizin: &Path) -> Result<[u8; 32], String> {
    let yol = anahtar_yolu(dizin);
    match std::fs::read_to_string(&yol) {
        Ok(m) => anahtar_coz(&m),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Err(format!(
            "teslim şifre anahtarı bulunamadı ({}); yedekten geri yükleyin",
            yol.display()
        )),
        Err(e) => Err(format!("teslim şifre anahtarı okunamadı: {e}")),
    }
}

/// Ana anahtar (32 bayt, şifreleme yolu). Yoksa oluşturulur (üzerine yazmaz); ANCAK defterde şifreli teslim
/// kaydı varken anahtar yoksa oluşturulmaz: anahtar kaybolmuştur, yeni anahtar eski kayıtları okunamaz bırakır.
fn ana_anahtar(dizin: &Path) -> Result<[u8; 32], String> {
    let yol = anahtar_yolu(dizin);
    let yeni = !yol.exists();
    if yeni && sifreli_kayit_var(dizin) {
        return Err(format!(
            "teslim şifre anahtarı bulunamadı ({}) ama defterde şifreli teslim kayıtları var. Yeni anahtar \
             üretilmedi; anahtar dosyasını yedekten geri yükleyin",
            yol.display()
        ));
    }
    let m = dosya_ya_da_olustur(&yol, || Ok(format!("{ONEK}{}", hex(&rastgele(32)?))))?;
    if yeni {
        izin_daralt(&yol);
    }
    anahtar_coz(&m)
}

/// Kör indeks anahtarı: ana anahtardan ayrı türetilir (şifre anahtarıyla aynı değildir).
fn indeks_anahtari(ana: &[u8; 32]) -> [u8; 32] {
    hmac_sha256(ana, b"taa-teslim-kor-indeks-v1")
}

/// Ad normalleştirme (kör indeks için): Türkçe küçük harf, boşluklar teke indirilir.
pub fn ad_normal(ad: &str) -> String {
    ad.split_whitespace()
        .map(|k| {
            k.chars()
                .flat_map(|c| match c {
                    'I' => vec!['ı'],
                    'İ' => vec!['i'],
                    c => c.to_lowercase().collect(),
                })
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn kor_indeks(anahtar: &[u8; 32], tur: &str, deger: &str) -> String {
    let mut h = hex(&hmac_sha256(anahtar, format!("{tur}:{deger}").as_bytes()));
    h.truncate(16);
    h
}

/// Aramada kullanılacak kör indeksler: (ad_indeks, tel_indeks). Telefon boşsa tel_indeks boştur.
pub fn indeksler(dizin: &Path, ad: &str, tel: &str) -> Result<(String, String), String> {
    let k = indeks_anahtari(&ana_anahtar_oku(dizin)?);
    let tel = tel_normal(tel).unwrap_or_default();
    Ok((
        kor_indeks(&k, "ad", &ad_normal(ad)),
        if tel.is_empty() {
            String::new()
        } else {
            kor_indeks(&k, "tel", &tel)
        },
    ))
}

/// Telefonu "+<rakamlar>" biçimine çevirir (bulut sayfasındaki kuralla aynı): +90/0 ile başlayan 10-11 haneli TR
/// numarası, ya da + ile 8..15 haneli uluslararası numara. Boş girdi `Some("")`; geçersizse `None`.
pub fn tel_normal(t: &str) -> Option<String> {
    let t = t.trim();
    if t.is_empty() {
        return Some(String::new());
    }
    if !t
        .chars()
        .all(|c| c.is_ascii_digit() || matches!(c, '+' | ' ' | '-' | '(' | ')' | '.'))
    {
        return None;
    }
    let artili = t.starts_with('+');
    if t[1..].contains('+') {
        return None;
    }
    let r: String = t.chars().filter(|c| c.is_ascii_digit()).collect();
    if artili {
        return (8..=15).contains(&r.len()).then(|| format!("+{r}"));
    }
    match r.len() {
        // 05xx xxx xx xx
        11 if r.starts_with('0') => Some(format!("+90{}", &r[1..])),
        // 5xx xxx xx xx
        10 if !r.starts_with('0') => Some(format!("+90{r}")),
        // 90 5xx ...
        12 if r.starts_with("90") => Some(format!("+{r}")),
        _ => None,
    }
}

fn base64_kodla(b: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut s = String::with_capacity(b.len().div_ceil(3) * 4);
    for p in b.chunks(3) {
        let n = (p[0] as u32) << 16
            | (*p.get(1).unwrap_or(&0) as u32) << 8
            | *p.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= p.len() {
                s.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                s.push('=');
            }
        }
    }
    s
}

/// Standart base64 (dolgulu ya da dolgusuz).
pub(crate) fn base64_coz(s: &str) -> Option<Vec<u8>> {
    crate::teslim_al::base64_coz(s).ok()
}

/// Düz bilgiyi sürüm 2 ile şifreler: bilgi alt anahtarı, aad = tür + fiş no + teslim_id.
pub fn sifrele(
    dizin: &Path,
    fis_no: &str,
    teslim_id: &str,
    d: &TeslimDuz,
) -> Result<TeslimSifreli, String> {
    let ana = ana_anahtar(dizin)?;
    let duz = serde_json::to_vec(d).map_err(|e| e.to_string())?;
    let (iv, c) = gcm_sifrele(
        &alt_anahtar(&ana, VeriTuru::Bilgi)?,
        &aad_v2(VeriTuru::Bilgi, fis_no, teslim_id),
        &duz,
    )?;
    let k = indeks_anahtari(&ana);
    let tel = tel_normal(&d.tel).unwrap_or_default();
    Ok(TeslimSifreli {
        surum: 2,
        iv: hex(&iv),
        c: base64_kodla(&c),
        ad_indeks: kor_indeks(&k, "ad", &ad_normal(&d.ad)),
        tel_indeks: if tel.is_empty() {
            String::new()
        } else {
            kor_indeks(&k, "tel", &tel)
        },
    })
}

/// Şifreli bloğu çözer. Sürüm 2: bilgi alt anahtarı, aad = tür + fiş no + teslim_id. Sürüm 1 (eski): ana anahtar,
/// aad = fiş no (teslim_id kullanılmaz). Fiş no, teslim_id ya da anahtar uyuşmazsa hata.
pub fn coz(
    dizin: &Path,
    fis_no: &str,
    teslim_id: &str,
    s: &TeslimSifreli,
) -> Result<TeslimDuz, String> {
    if !s.gecerli() {
        return Err("şifreli teslim bilgisi bozuk".into());
    }
    let ana = ana_anahtar_oku(dizin)?;
    let iv = hex_coz(&s.iv).ok_or("iv bozuk")?;
    let c = base64_coz(&s.c).ok_or("şifreli metin bozuk")?;
    let duz = match s.surum {
        1 => gcm_coz(&ana, fis_no.as_bytes(), &iv, &c),
        2 => gcm_coz(
            &alt_anahtar(&ana, VeriTuru::Bilgi)?,
            &aad_v2(VeriTuru::Bilgi, fis_no, teslim_id),
            &iv,
            &c,
        ),
        _ => None,
    }
    .ok_or("teslim bilgisi çözülemedi (anahtar, fiş ya da teslimat uyuşmuyor)")?;
    serde_json::from_slice(&duz).map_err(|_| "teslim bilgisi okunamadı".into())
}

/// Telefonu ekranda maskeler: son 4 hane dışındaki rakamlar gizlenir ("+••••••••4567").
pub fn tel_maskele(tel: &str) -> String {
    // Not: ad için `taa_fis::ad_maskele` ("M*** Y***"); IP maskeli görünümde hiç gösterilmez.
    let r: Vec<char> = tel.chars().filter(|c| c.is_ascii_digit()).collect();
    if r.is_empty() {
        return String::new();
    }
    let gizli = r.len().saturating_sub(4);
    let mut s = String::from(if tel.starts_with('+') { "+" } else { "" });
    for (i, c) in r.iter().enumerate() {
        s.push(if i < gizli { '•' } else { *c });
    }
    s
}

/// User-agent'tan kısa cihaz özeti: işletim sistemi (+ model) ve tarayıcı. Ör. "Android 14 · SM-S918B · Chrome".
pub fn cihaz_ozeti(ua: &str) -> String {
    let ua = ua.trim();
    if ua.is_empty() {
        return String::new();
    }
    let surum = |onek: &str, ayrac: &[char]| -> Option<String> {
        let i = ua.find(onek)? + onek.len();
        let s: String = ua[i..]
            .chars()
            .take_while(|c| !ayrac.contains(c))
            .collect::<String>()
            .replace('_', ".");
        let s = s.trim().to_string();
        (!s.is_empty() && s.len() <= 12).then_some(s)
    };
    let mut parca: Vec<String> = Vec::new();
    if ua.contains("Android") {
        parca.push(match surum("Android ", &[';', ')']) {
            Some(v) => format!("Android {v}"),
            None => "Android".into(),
        });
        // "Linux; Android 14; SM-S918B Build/..." ya da "Android 10; K)" : modeli al.
        if let Some(i) = ua.find("Android") {
            let geri = &ua[i..];
            let ic = geri.split(')').next().unwrap_or("");
            if let Some(m) = ic.split(';').nth(1) {
                let m = m.split(" Build").next().unwrap_or("").trim();
                if !m.is_empty() && m != "K" && m.len() <= 30 && !m.contains("wv") {
                    parca.push(m.to_string());
                }
            }
        }
    } else if ua.contains("iPhone") {
        parca.push(match surum("iPhone OS ", &[' ', ';', ')']) {
            Some(v) => format!("iPhone · iOS {v}"),
            None => "iPhone".into(),
        });
    } else if ua.contains("iPad") {
        parca.push("iPad".into());
    } else if ua.contains("Windows") {
        parca.push("Windows".into());
    } else if ua.contains("Mac OS X") {
        parca.push("macOS".into());
    } else if ua.contains("Linux") {
        parca.push("Linux".into());
    }
    let tarayici = if ua.contains("SamsungBrowser") {
        "Samsung Internet"
    } else if ua.contains("Edg") {
        "Edge"
    } else if ua.contains("OPR") || ua.contains("Opera") {
        "Opera"
    } else if ua.contains("YaBrowser") {
        "Yandex"
    } else if ua.contains("MiuiBrowser") {
        "Mi Tarayıcı"
    } else if ua.contains("Firefox") || ua.contains("FxiOS") {
        "Firefox"
    } else if ua.contains("CriOS") || ua.contains("Chrome") {
        "Chrome"
    } else if ua.contains("Safari") {
        "Safari"
    } else {
        ""
    };
    if !tarayici.is_empty() {
        parca.push(tarayici.into());
    }
    if parca.is_empty() {
        return "Bilinmeyen cihaz".into();
    }
    parca.join(" · ")
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Sahte teslim kimliği.
    const TID: &str = "T-20261126-ABCDEF";

    fn duz() -> TeslimDuz {
        // Sahte kişi (gerçek değildir).
        TeslimDuz {
            ad: "Deneme Kişi".into(),
            tel: "+905320004567".into(),
            cihaz: "Mozilla/5.0 (Linux; Android 14; SM-DENEME) Chrome/120".into(),
            ip: "203.0.113.7".into(),
            zaman: 1_795_000_000,
            yontem: "qr".into(),
        }
    }

    #[test]
    fn sifrele_coz_ve_aad_baglama() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let s = sifrele(&d, "F-20261126-001", TID, &duz()).unwrap();
        assert!(s.gecerli());
        let j = serde_json::to_string(&s).unwrap();
        for yasak in ["Deneme", "4567", "203.0.113", "Android", "SM-DENEME"] {
            assert!(!j.contains(yasak), "{yasak} şifreli blokta düz metin: {j}");
        }
        assert_eq!(coz(&d, "F-20261126-001", TID, &s).unwrap(), duz());
        // başka fişin kaydına taşınırsa çözülmez
        assert!(coz(&d, "F-20261126-002", TID, &s).is_err());
        // başka kurulumun anahtarıyla çözülmez
        let d2 = taa_dosya::gecici_klasor().unwrap();
        assert!(coz(&d2, "F-20261126-001", TID, &s).is_err());
        // kurcalanmış şifreli metin
        let mut k = s.clone();
        let mut b = base64_coz(&k.c).unwrap();
        b[0] ^= 1;
        k.c = base64_kodla(&b);
        assert!(coz(&d, "F-20261126-001", TID, &k).is_err());
        // her şifrelemede yeni IV
        assert_ne!(sifrele(&d, "F-20261126-001", TID, &duz()).unwrap().iv, s.iv);
        // anahtar dosyası biçimi
        let m = std::fs::read_to_string(d.join(ANAHTAR_DOSYASI)).unwrap();
        assert!(m.starts_with(ONEK) && m.len() == ONEK.len() + 64);
    }

    #[test]
    fn anahtar_kaybinda_yeni_anahtar_uretilmez() {
        let d = taa_dosya::gecici_klasor().unwrap();
        // anahtar yokken çözme ve indeks anahtar üretmez
        let s0 = TeslimSifreli {
            surum: 1,
            iv: "00".repeat(12),
            c: base64_kodla(&[0u8; 32]),
            ad_indeks: "0".repeat(16),
            tel_indeks: String::new(),
        };
        assert!(coz(&d, "F-1", TID, &s0).unwrap_err().contains("bulunamadı"));
        assert!(indeksler(&d, "Deneme Kişi", "").is_err());
        assert!(!d.join(ANAHTAR_DOSYASI).exists());
        // defterde şifreli kayıt yokken ilk şifreleme anahtarı üretir
        let s = sifrele(&d, "F-1", TID, &duz()).unwrap();
        std::fs::write(
            d.join("fisler.jsonl"),
            format!(
                "{{\"tur\":\"teslimkaydi\",\"sifreli\":{}}}\n",
                serde_json::to_string(&s).unwrap()
            ),
        )
        .unwrap();
        assert!(sifreli_kayit_var(&d));
        // anahtar kaybolursa: yeni anahtar üretilmez, açık hata
        std::fs::remove_file(d.join(ANAHTAR_DOSYASI)).unwrap();
        let e = sifrele(&d, "F-2", TID, &duz()).unwrap_err();
        assert!(e.contains("yedekten"), "{e}");
        assert!(!d.join(ANAHTAR_DOSYASI).exists());
        assert!(coz(&d, "F-1", TID, &s).is_err());
    }

    #[test]
    fn bayt_sifreleme_ve_anahtar_koruma() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let png = b"\x89PNG\r\n\x1a\nDENEME-IMZA".to_vec();
        let v = bayt_sifrele(&d, b"F-1", &png).unwrap();
        assert!(v.starts_with(DOSYA_BASLIGI));
        assert!(!v.windows(11).any(|w| w == b"DENEME-IMZA"));
        assert_eq!(bayt_coz(&d, b"F-1", &v).unwrap(), png);
        assert!(
            bayt_coz(&d, b"F-2", &v).is_err(),
            "başka fişe taşınan imza çözülmez"
        );
        let mut k = v.clone();
        let n = k.len() - 1;
        k[n] ^= 1;
        assert!(bayt_coz(&d, b"F-1", &k).is_err());
        assert!(
            bayt_coz(&d, b"F-1", &png).is_err(),
            "şifresiz dosya kabul edilmez"
        );
        // şifreli imza dosyası varken anahtar kaybolursa yeni anahtar üretilmez
        std::fs::create_dir_all(d.join("imzalar")).unwrap();
        std::fs::write(d.join("imzalar").join("F-1-1.png.enc"), &v).unwrap();
        assert!(sifreli_kayit_var(&d));
        std::fs::remove_file(d.join(ANAHTAR_DOSYASI)).unwrap();
        assert!(bayt_sifrele(&d, b"F-1", &png).is_err());
        assert!(!d.join(ANAHTAR_DOSYASI).exists());
    }

    /// Eski (v1) teslim bilgisi yazımı: ana anahtar doğrudan, aad = fiş no (geri uyum testi için; eski kodla aynı).
    fn sifrele_v1(dizin: &Path, fis_no: &str, d: &TeslimDuz) -> TeslimSifreli {
        let ana = ana_anahtar(dizin).unwrap();
        let (iv, c) =
            gcm_sifrele(&ana, fis_no.as_bytes(), &serde_json::to_vec(d).unwrap()).unwrap();
        let k = indeks_anahtari(&ana);
        TeslimSifreli {
            surum: 1,
            iv: hex(&iv),
            c: base64_kodla(&c),
            ad_indeks: kor_indeks(&k, "ad", &ad_normal(&d.ad)),
            tel_indeks: kor_indeks(&k, "tel", &tel_normal(&d.tel).unwrap()),
        }
    }

    #[test]
    fn surum2_alt_anahtar_ve_teslim_id_baglama() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let s = sifrele(&d, "F-1", TID, &duz()).unwrap();
        assert_eq!(s.surum, 2);
        assert!(s.gecerli());
        assert_eq!(coz(&d, "F-1", TID, &s).unwrap(), duz());
        // başka teslimata (teslim_id) taşınırsa ya da teslim_id silinirse çözülmez
        assert!(coz(&d, "F-1", "T-20261126-ZZZZZZ", &s).is_err());
        assert!(coz(&d, "F-1", "", &s).is_err());
        // v2 blok v1 diye işaretlenirse (sürüm düşürme) ana anahtarla çözülmez
        let mut eski = s.clone();
        eski.surum = 1;
        assert!(coz(&d, "F-1", TID, &eski).is_err());
        let mut bilinmeyen = s.clone();
        bilinmeyen.surum = 3;
        assert!(coz(&d, "F-1", TID, &bilinmeyen).is_err());
        // alt anahtarlar türe göre ayrı ve ana anahtardan farklı
        let ana = ana_anahtar_oku(&d).unwrap();
        let (kb, ki) = (
            alt_anahtar(&ana, VeriTuru::Bilgi).unwrap(),
            alt_anahtar(&ana, VeriTuru::Imza).unwrap(),
        );
        assert!(kb != ki && kb != ana && ki != ana);
        // HKDF-SHA256(ikm, tuz yok, info) sabit vektörü: ikm = 0x0b * 32
        assert_eq!(
            hex(&alt_anahtar(&[0x0b; 32], VeriTuru::Bilgi).unwrap()),
            hex(&{
                let mut k = [0u8; 32];
                hkdf::Hkdf::<sha2::Sha256>::new(Some(&[0u8; 32]), &[0x0b; 32])
                    .expand(b"taa-teslim-bilgi-v2", &mut k)
                    .unwrap();
                k
            }),
            "tuz yok = 32 sıfır bayt (RFC 5869)"
        );
        // tür ayrımı: imza alt anahtarı + aynı aad bağlamıyla bilgi bloğu çözülmez
        let iv = hex_coz(&s.iv).unwrap();
        let c = base64_coz(&s.c).unwrap();
        assert!(gcm_coz(&ki, &aad_v2(VeriTuru::Bilgi, "F-1", TID), &iv, &c).is_none());
        assert!(gcm_coz(&kb, &aad_v2(VeriTuru::Imza, "F-1", TID), &iv, &c).is_none());
        assert!(gcm_coz(&kb, &aad_v2(VeriTuru::Bilgi, "F-1", TID), &iv, &c).is_some());
        // imza dosyası: v2 başlık, fiş + teslim_id bağlı
        let png = b"\x89PNG\r\n\x1a\nDENEME-IMZA".to_vec();
        let v = imza_sifrele(&d, "F-1", TID, &png).unwrap();
        assert!(v.starts_with(DOSYA_BASLIGI_V2));
        assert!(!v.windows(11).any(|w| w == b"DENEME-IMZA"));
        assert_eq!(imza_coz(&d, "F-1", TID, &v).unwrap(), png);
        assert!(imza_coz(&d, "F-2", TID, &v).is_err());
        assert!(imza_coz(&d, "F-1", "T-20261126-ZZZZZZ", &v).is_err());
        // v2 imza dosyası v1 başlığıyla sunulursa (sürüm düşürme) çözülmez
        let mut dusuk = v.clone();
        dusuk[..8].copy_from_slice(DOSYA_BASLIGI);
        assert!(imza_coz(&d, "F-1", TID, &dusuk).is_err());
        assert!(
            bayt_coz(&d, b"F-1", &v).is_err(),
            "v2 dosya v1 yoluyla çözülmez"
        );
        // şifreli imza dosyası anahtar koruma kuralına girer
        std::fs::create_dir_all(d.join("imzalar")).unwrap();
        std::fs::write(d.join("imzalar").join("F-1-1.png.enc"), &v).unwrap();
        assert!(sifreli_kayit_var(&d));
    }

    #[test]
    fn surum1_eski_kayitlar_eski_yolla_cozulur() {
        let d = taa_dosya::gecici_klasor().unwrap();
        // eski kodun yazdığı v1 teslim bilgisi: teslim_id ne olursa olsun fiş no ile çözülür
        let v1 = sifrele_v1(&d, "F-20261126-001", &duz());
        assert!(v1.gecerli());
        for tid in ["", TID, "T-20261126-ZZZZZZ"] {
            assert_eq!(coz(&d, "F-20261126-001", tid, &v1).unwrap(), duz());
        }
        assert!(coz(&d, "F-20261126-002", "", &v1).is_err());
        // v1 kör indeksleri v2 ile aynı (indeks anahtarı değişmedi): eski kayıtlar aramada bulunur
        let v2 = sifrele(&d, "F-20261126-003", TID, &duz()).unwrap();
        assert_eq!(
            (v1.ad_indeks.as_str(), v1.tel_indeks.as_str()),
            (v2.ad_indeks.as_str(), v2.tel_indeks.as_str())
        );
        // JSON'dan okunan eski kayıt (defterdeki biçim)
        let j = serde_json::to_string(&v1).unwrap();
        assert!(j.contains("\"surum\":1"));
        let geri: TeslimSifreli = serde_json::from_str(&j).unwrap();
        assert_eq!(coz(&d, "F-20261126-001", TID, &geri).unwrap(), duz());
        // eski v1 imza dosyası (TAAENC1, aad = fiş no) imza_coz ile çözülür; teslim_id önemsenmez
        let png = b"\x89PNG\r\n\x1a\nESKI-IMZA".to_vec();
        let eski = bayt_sifrele(&d, b"F-20261126-001", &png).unwrap();
        assert!(eski.starts_with(DOSYA_BASLIGI));
        assert_eq!(imza_coz(&d, "F-20261126-001", TID, &eski).unwrap(), png);
        assert_eq!(imza_coz(&d, "F-20261126-001", "", &eski).unwrap(), png);
        assert!(imza_coz(&d, "F-20261126-002", TID, &eski).is_err());
    }

    #[test]
    fn kor_indeks_kararli() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let a = sifrele(&d, "F-1", TID, &duz()).unwrap();
        let mut x = duz();
        x.ad = "  DENEME   kişi ".into();
        x.tel = "0532 000 45 67".into();
        let b = sifrele(&d, "F-2", TID, &x).unwrap();
        assert_eq!(
            a.ad_indeks, b.ad_indeks,
            "normalize ad aynı indeksi vermeli"
        );
        assert_eq!(
            a.tel_indeks, b.tel_indeks,
            "normalize telefon aynı indeksi vermeli"
        );
        assert_eq!(
            indeksler(&d, "Deneme Kişi", "+905320004567").unwrap(),
            (a.ad_indeks.clone(), a.tel_indeks.clone())
        );
        assert_eq!(a.ad_indeks.len(), 16);
        x.ad = "Deneme Kisi".into();
        assert_ne!(sifrele(&d, "F-3", TID, &x).unwrap().ad_indeks, a.ad_indeks);
        // kurulumlar arası farklı (anahtara bağlı)
        let d2 = taa_dosya::gecici_klasor().unwrap();
        assert_ne!(
            sifrele(&d2, "F-1", TID, &duz()).unwrap().ad_indeks,
            a.ad_indeks
        );
        // ekran yolu: telefon yok -> tel_indeks boş
        let mut e = duz();
        e.tel = String::new();
        assert!(sifrele(&d, "F-4", TID, &e).unwrap().tel_indeks.is_empty());
        assert_eq!(ad_normal("IŞIK İnce"), "ışık ince");
    }

    #[test]
    fn telefon_normal_maske_ve_cihaz() {
        assert_eq!(tel_normal("0532 000 45 67").unwrap(), "+905320004567");
        assert_eq!(tel_normal("+90 532 000 45 67").unwrap(), "+905320004567");
        assert_eq!(tel_normal("5320004567").unwrap(), "+905320004567");
        assert_eq!(tel_normal("+49 1512 3456789").unwrap(), "+4915123456789");
        assert!(tel_normal("12345").is_none());
        assert!(tel_normal("+1234567").is_none());
        assert!(tel_normal("0532abc").is_none());
        assert_eq!(tel_maskele("+905320004567"), "+••••••••4567");
        assert_eq!(
            cihaz_ozeti("Mozilla/5.0 (Linux; Android 14; SM-S918B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Mobile Safari/537.36"),
            "Android 14 · SM-S918B · Chrome"
        );
        assert_eq!(
            cihaz_ozeti("Mozilla/5.0 (iPhone; CPU iPhone OS 17_1 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Mobile/15E148 Safari/604.1"),
            "iPhone · iOS 17.1 · Safari"
        );
        assert_eq!(
            cihaz_ozeti("Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) SamsungBrowser/23.0 Chrome/115.0 Mobile Safari/537.36"),
            "Android 10 · Samsung Internet"
        );
        assert_eq!(cihaz_ozeti("tuhaf"), "Bilinmeyen cihaz");
    }
}
