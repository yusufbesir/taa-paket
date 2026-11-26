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
//! - Bu modül hiçbir şeyi günlüğe yazmaz.
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

/// Defterde şifreli teslim kaydı var mı? (anahtar kaybını sessizce yeni anahtarla örtmemek için)
pub fn sifreli_kayit_var(dizin: &Path) -> bool {
    std::fs::read(dizin.join("fisler.jsonl"))
        .map(|b| b.windows(11).any(|w| w == b"\"sifreli\":{"))
        .unwrap_or(false)
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

/// Düz bilgiyi fiş no'ya bağlı olarak şifreler.
pub fn sifrele(dizin: &Path, fis_no: &str, d: &TeslimDuz) -> Result<TeslimSifreli, String> {
    let ana = ana_anahtar(dizin)?;
    let duz = serde_json::to_vec(d).map_err(|e| e.to_string())?;
    let iv = rastgele(12)?;
    let sifreci = Aes256Gcm::new_from_slice(&ana).map_err(|_| "anahtar uzunluğu")?;
    let c = sifreci
        .encrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: &duz,
                aad: fis_no.as_bytes(),
            },
        )
        .map_err(|_| "şifrelenemedi")?;
    let k = indeks_anahtari(&ana);
    let tel = tel_normal(&d.tel).unwrap_or_default();
    Ok(TeslimSifreli {
        surum: 1,
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

/// Şifreli bloğu çözer; fiş no (aad) ya da anahtar uyuşmazsa hata.
pub fn coz(dizin: &Path, fis_no: &str, s: &TeslimSifreli) -> Result<TeslimDuz, String> {
    if !s.gecerli() {
        return Err("şifreli teslim bilgisi bozuk".into());
    }
    let ana = ana_anahtar_oku(dizin)?;
    let iv = hex_coz(&s.iv).ok_or("iv bozuk")?;
    let c = base64_coz(&s.c).ok_or("şifreli metin bozuk")?;
    let sifreci = Aes256Gcm::new_from_slice(&ana).map_err(|_| "anahtar uzunluğu")?;
    let duz = sifreci
        .decrypt(
            Nonce::from_slice(&iv),
            Payload {
                msg: &c,
                aad: fis_no.as_bytes(),
            },
        )
        .map_err(|_| "teslim bilgisi çözülemedi (anahtar ya da fiş uyuşmuyor)")?;
    serde_json::from_slice(&duz).map_err(|_| "teslim bilgisi okunamadı".into())
}

/// Telefonu ekranda maskeler: son 4 hane dışındaki rakamlar gizlenir ("+••••••••4567").
pub fn tel_maskele(tel: &str) -> String {
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
        let s = sifrele(&d, "F-20261126-001", &duz()).unwrap();
        assert!(s.gecerli());
        let j = serde_json::to_string(&s).unwrap();
        for yasak in ["Deneme", "4567", "203.0.113", "Android", "SM-DENEME"] {
            assert!(!j.contains(yasak), "{yasak} şifreli blokta düz metin: {j}");
        }
        assert_eq!(coz(&d, "F-20261126-001", &s).unwrap(), duz());
        // başka fişin kaydına taşınırsa çözülmez
        assert!(coz(&d, "F-20261126-002", &s).is_err());
        // başka kurulumun anahtarıyla çözülmez
        let d2 = taa_dosya::gecici_klasor().unwrap();
        assert!(coz(&d2, "F-20261126-001", &s).is_err());
        // kurcalanmış şifreli metin
        let mut k = s.clone();
        let mut b = base64_coz(&k.c).unwrap();
        b[0] ^= 1;
        k.c = base64_kodla(&b);
        assert!(coz(&d, "F-20261126-001", &k).is_err());
        // her şifrelemede yeni IV
        assert_ne!(sifrele(&d, "F-20261126-001", &duz()).unwrap().iv, s.iv);
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
        assert!(coz(&d, "F-1", &s0).unwrap_err().contains("bulunamadı"));
        assert!(indeksler(&d, "Deneme Kişi", "").is_err());
        assert!(!d.join(ANAHTAR_DOSYASI).exists());
        // defterde şifreli kayıt yokken ilk şifreleme anahtarı üretir
        let s = sifrele(&d, "F-1", &duz()).unwrap();
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
        let e = sifrele(&d, "F-2", &duz()).unwrap_err();
        assert!(e.contains("yedekten"), "{e}");
        assert!(!d.join(ANAHTAR_DOSYASI).exists());
        assert!(coz(&d, "F-1", &s).is_err());
    }

    #[test]
    fn kor_indeks_kararli() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let a = sifrele(&d, "F-1", &duz()).unwrap();
        let mut x = duz();
        x.ad = "  DENEME   kişi ".into();
        x.tel = "0532 000 45 67".into();
        let b = sifrele(&d, "F-2", &x).unwrap();
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
        assert_ne!(sifrele(&d, "F-3", &x).unwrap().ad_indeks, a.ad_indeks);
        // kurulumlar arası farklı (anahtara bağlı)
        let d2 = taa_dosya::gecici_klasor().unwrap();
        assert_ne!(sifrele(&d2, "F-1", &duz()).unwrap().ad_indeks, a.ad_indeks);
        // ekran yolu: telefon yok -> tel_indeks boş
        let mut e = duz();
        e.tel = String::new();
        assert!(sifrele(&d, "F-4", &e).unwrap().tel_indeks.is_empty());
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
