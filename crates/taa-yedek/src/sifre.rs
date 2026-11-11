//! Yedek şifreleme (XChaCha20-Poly1305, akışlı). Anahtar yalnız verilen dosyadan okunur; yedeğe/günlüğe yazılmaz.
//! Dosya biçimi: "TAAY1" | 19 bayt rastgele ön ek | 64 KiB'lık parçalar (her biri şifreli + 16 bayt etiket).
//! Son parça ayrıca işaretlenir: kesilen/sıralaması değişen dosya çözülmez. Yanlış anahtar ya da bozuk bayt = hata.
//! Yazan: [Claude#taa-kur-sifre 2026-11-11].

use chacha20poly1305::aead::stream::{DecryptorBE32, EncryptorBE32};
use chacha20poly1305::{Key, KeyInit, XChaCha20Poly1305};
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

pub const ON_EK: &[u8; 5] = b"TAAY1";
pub const UZANTI: &str = ".taay";
const PARCA: usize = 64 * 1024;
const ETIKET: usize = 16;
const ANAHTAR_ON_EK: &str = "taa-yedek-anahtar:";

/// 32 baytlık yedek anahtarı. `Debug` içeriği göstermez.
pub struct Anahtar([u8; 32]);

impl std::fmt::Debug for Anahtar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Anahtar(***)")
    }
}

impl Anahtar {
    pub fn bayt(b: [u8; 32]) -> Anahtar {
        Anahtar(b)
    }
    fn sifreleyici(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new(Key::from_slice(&self.0))
    }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn hex_coz(s: &str) -> Option<[u8; 32]> {
    if s.len() != 64 || !s.is_ascii() {
        return None;
    }
    let mut o = [0u8; 32];
    for (i, c) in o.iter_mut().enumerate() {
        *c = u8::from_str_radix(&s[2 * i..2 * i + 2], 16).ok()?;
    }
    Some(o)
}

#[cfg(unix)]
fn rastgele(b: &mut [u8]) -> std::io::Result<()> {
    fs::File::open("/dev/urandom")?.read_exact(b)
}

#[cfg(windows)]
fn rastgele(b: &mut [u8]) -> std::io::Result<()> {
    #[link(name = "advapi32")]
    extern "system" {
        // RtlGenRandom
        fn SystemFunction036(tampon: *mut u8, uzunluk: u32) -> u8;
    }
    // SAFETY: tampon geçerli ve `uzunluk` bayt yazılabilir; işlev yalnız bu aralığa yazar.
    if unsafe { SystemFunction036(b.as_mut_ptr(), b.len() as u32) } == 0 {
        Err(std::io::Error::other("RtlGenRandom başarısız"))
    } else {
        Ok(())
    }
}

#[cfg(not(any(unix, windows)))]
fn rastgele(_b: &mut [u8]) -> std::io::Result<()> {
    Err(std::io::Error::other("bu platformda rastgelelik yok"))
}

/// Yeni rastgele anahtarı `yol` dosyasına YENİ dosya olarak yazar (var olan ezilmez; Unix'te 0600).
pub fn anahtar_uret(yol: &Path) -> Result<(), String> {
    let mut a = [0u8; 32];
    rastgele(&mut a).map_err(|e| e.to_string())?;
    let mut oz = fs::OpenOptions::new();
    oz.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        oz.mode(0o600);
    }
    if let Some(d) = yol.parent().filter(|d| !d.as_os_str().is_empty()) {
        fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let mut f = oz.open(yol).map_err(|e| {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            format!(
                "{} zaten var; üzerine yazılmaz. Öneri: başka bir yol verin.",
                yol.display()
            )
        } else {
            e.to_string()
        }
    })?;
    f.write_all(format!("{ANAHTAR_ON_EK}{}\n", hex(&a)).as_bytes())
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())
}

pub fn anahtar_oku(yol: &Path) -> Result<Anahtar, String> {
    let m = fs::read_to_string(yol).map_err(|e| format!("anahtar dosyası okunamadı: {e}"))?;
    m.lines()
        .map(str::trim)
        .find(|s| !s.is_empty() && !s.starts_with('#'))
        .and_then(|s| s.strip_prefix(ANAHTAR_ON_EK))
        .and_then(hex_coz)
        .map(Anahtar)
        .ok_or_else(|| {
            "anahtar dosyası biçimi bozuk (taa-yedek-anahtar:<64 onaltılık hane>)".into()
        })
}

/// Okuyucudan en çok `n` bayt okur (EOF'a kadar); dönen sayı `n`den azsa dosya bitmiştir.
fn doldur(r: &mut impl Read, tampon: &mut [u8]) -> std::io::Result<usize> {
    let mut toplam = 0;
    while toplam < tampon.len() {
        let n = r.read(&mut tampon[toplam..])?;
        if n == 0 {
            break;
        }
        toplam += n;
    }
    Ok(toplam)
}

pub fn sifrele(kaynak: &Path, hedef: &Path, a: &Anahtar) -> Result<(), String> {
    let mut giris = fs::File::open(kaynak).map_err(|e| format!("{}: {e}", kaynak.display()))?;
    let mut cikis = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(hedef)
        .map_err(|e| format!("{}: {e}", hedef.display()))?;
    let mut on = [0u8; 19];
    rastgele(&mut on).map_err(|e| e.to_string())?;
    cikis
        .write_all(ON_EK)
        .and_then(|_| cikis.write_all(&on))
        .map_err(|e| e.to_string())?;
    let mut enc = EncryptorBE32::from_aead(a.sifreleyici(), (&on).into());
    let (mut simdiki, mut sonraki) = (vec![0u8; PARCA], vec![0u8; PARCA]);
    let mut n = doldur(&mut giris, &mut simdiki).map_err(|e| e.to_string())?;
    loop {
        // Bir parça ileri bakılır: sonraki boşsa bu parça sonuncudur (tam dolu olsa bile).
        let m = if n == PARCA {
            doldur(&mut giris, &mut sonraki).map_err(|e| e.to_string())?
        } else {
            0
        };
        if m == 0 {
            let c = enc
                .encrypt_last(&simdiki[..n])
                .map_err(|_| "şifreleme başarısız")?;
            cikis.write_all(&c).map_err(|e| e.to_string())?;
            break;
        }
        let c = enc
            .encrypt_next(&simdiki[..n])
            .map_err(|_| "şifreleme başarısız")?;
        cikis.write_all(&c).map_err(|e| e.to_string())?;
        std::mem::swap(&mut simdiki, &mut sonraki);
        n = m;
    }
    cikis.sync_all().map_err(|e| e.to_string())
}

/// `kaynak` şifreli dosyasını çözüp `hedef`e (YENİ dosya) yazar. Hata olursa `hedef` yarım kalabilir; çağıran
/// geçici bir adla yazıp doğrulandıktan sonra taşımalıdır.
pub fn coz(kaynak: &Path, hedef: Option<&Path>, a: &Anahtar) -> Result<(), String> {
    let mut giris = fs::File::open(kaynak).map_err(|e| format!("{}: {e}", kaynak.display()))?;
    let mut baslik = [0u8; 5 + 19];
    giris.read_exact(&mut baslik).map_err(|_| {
        format!(
            "{} şifreli yedek dosyası değil (başlık eksik)",
            kaynak.display()
        )
    })?;
    if &baslik[..5] != ON_EK {
        return Err(format!(
            "{} şifreli yedek dosyası değil (imza yok)",
            kaynak.display()
        ));
    }
    let mut cikis = match hedef {
        Some(h) => Some(
            fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(h)
                .map_err(|e| format!("{}: {e}", h.display()))?,
        ),
        None => None,
    };
    let mut dec = DecryptorBE32::from_aead(a.sifreleyici(), (&baslik[5..]).into());
    let hata = || {
        format!(
            "{} çözülemedi: anahtar yanlış ya da dosya bozuk/kesik",
            kaynak.display()
        )
    };
    let (mut simdiki, mut sonraki) = (vec![0u8; PARCA + ETIKET], vec![0u8; PARCA + ETIKET]);
    let mut n = doldur(&mut giris, &mut simdiki).map_err(|e| e.to_string())?;
    if n < ETIKET {
        return Err(hata());
    }
    loop {
        let m = if n == PARCA + ETIKET {
            doldur(&mut giris, &mut sonraki).map_err(|e| e.to_string())?
        } else {
            0
        };
        if m == 0 {
            let p = dec.decrypt_last(&simdiki[..n]).map_err(|_| hata())?;
            if let Some(c) = cikis.as_mut() {
                c.write_all(&p).map_err(|e| e.to_string())?;
            }
            break;
        }
        let p = dec.decrypt_next(&simdiki[..n]).map_err(|_| hata())?;
        if let Some(c) = cikis.as_mut() {
            c.write_all(&p).map_err(|e| e.to_string())?;
        }
        std::mem::swap(&mut simdiki, &mut sonraki);
        n = m;
    }
    if let Some(c) = cikis {
        c.sync_all().map_err(|e| e.to_string())?;
    }
    Ok(())
}
