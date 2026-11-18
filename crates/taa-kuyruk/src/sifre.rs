// Sahip: Yusuf. yazan: [Claude#hata-bildirimi 2026-11-18]
//! Yerel kuyruk şifrelemesi: kayıt gövdesi `{"_sifreli":1,"n":<12 bayt hex>,"c":<şifreli+etiket hex>}` olur,
//! gönderim sırasında `Kuyruk::gonder` kendiliğinden çözüp sunucuya düz gövdeyi yollar (TLS ile).
//! ChaCha20-Poly1305 (IETF, 12 bayt nonce; Node `crypto` ile birebir uyumlu). Anahtar: `<kök>/anahtar/kuyruk.key`
//! (64 hex, makine başına, ilk kullanımda atomik üretilir; kullanıcı profilinin izniyle korunur, depoya/günlüğe yazılmaz).
//! Koruma kapsamı: kuyruk dosyasının tek başına kopyalanıp okunması; anahtar dosyasına da erişen saldırgana karşı değil.

use chacha20poly1305::aead::{Aead, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce};
use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Şifreli gövdeyi işaretleyen alan.
pub const ALAN: &str = "_sifreli";

pub fn anahtar_yolu(kok: &Path) -> PathBuf {
    kok.join("anahtar").join("kuyruk.key")
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hex_coz(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 || !s.is_ascii() {
        return None;
    }
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok())
        .collect()
}

#[cfg(unix)]
fn rastgele(b: &mut [u8]) -> io::Result<()> {
    use std::io::Read;
    fs::File::open("/dev/urandom")?.read_exact(b)
}

#[cfg(windows)]
fn rastgele(b: &mut [u8]) -> io::Result<()> {
    #[link(name = "advapi32")]
    extern "system" {
        // RtlGenRandom
        fn SystemFunction036(tampon: *mut u8, uzunluk: u32) -> u8;
    }
    // SAFETY: tampon geçerli ve `uzunluk` bayt yazılabilir; işlev yalnız bu aralığa yazar.
    let ok = unsafe { SystemFunction036(b.as_mut_ptr(), b.len() as u32) };
    if ok == 0 {
        Err(io::Error::other("RtlGenRandom başarısız"))
    } else {
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
fn rastgele(_b: &mut [u8]) -> io::Result<()> {
    Err(io::Error::other("bu platformda rastgelelik kaynağı yok"))
}

/// Anahtarı okur; yoksa üretip atomik (`create_new`) yazar. Yarışta kaybeden, kazananın anahtarını okur.
pub fn anahtar_al(kok: &Path) -> io::Result<[u8; 32]> {
    let yol = anahtar_yolu(kok);
    if let Ok(m) = fs::read_to_string(&yol) {
        if let Some(b) = hex_coz(m.trim()).filter(|b| b.len() == 32) {
            let mut a = [0u8; 32];
            a.copy_from_slice(&b);
            return Ok(a);
        }
    }
    if let Some(d) = yol.parent() {
        fs::create_dir_all(d)?;
    }
    let mut a = [0u8; 32];
    rastgele(&mut a)?;
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&yol)
    {
        Ok(mut f) => {
            f.write_all(hex(&a).as_bytes())?;
            Ok(a)
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            // başka süreç az önce yazdı (yarım okunabilir): kısa bekleyip yeniden oku
            for _ in 0..20 {
                std::thread::sleep(std::time::Duration::from_millis(25));
                if let Ok(m) = fs::read_to_string(&yol) {
                    if let Some(b) = hex_coz(m.trim()).filter(|b| b.len() == 32) {
                        let mut a = [0u8; 32];
                        a.copy_from_slice(&b);
                        return Ok(a);
                    }
                }
            }
            Err(io::Error::other("kuyruk anahtarı okunamadı"))
        }
        Err(e) => Err(e),
    }
}

/// Gövdeyi (JSON nesnesi) şifreler.
pub fn kapat(anahtar: &[u8; 32], govde: &Value) -> io::Result<Value> {
    let mut n = [0u8; 12];
    rastgele(&mut n)?;
    let s = ChaCha20Poly1305::new(Key::from_slice(anahtar));
    let c = s
        .encrypt(Nonce::from_slice(&n), govde.to_string().as_bytes())
        .map_err(|_| io::Error::other("şifreleme başarısız"))?;
    Ok(json!({ ALAN: 1, "n": hex(&n), "c": hex(&c) }))
}

/// Şifreli gövde mü?
pub fn sifreli_mi(govde: &Value) -> bool {
    govde.get(ALAN).is_some()
}

/// Şifreli gövdeyi çözer; anahtar/biçim/etiket yanlışsa `None`.
pub fn ac(anahtar: &[u8; 32], govde: &Value) -> Option<Value> {
    let n = hex_coz(govde.get("n")?.as_str()?)?;
    let c = hex_coz(govde.get("c")?.as_str()?)?;
    if n.len() != 12 {
        return None;
    }
    let s = ChaCha20Poly1305::new(Key::from_slice(anahtar));
    let d = s.decrypt(Nonce::from_slice(&n), c.as_ref()).ok()?;
    serde_json::from_slice(&d).ok()
}
