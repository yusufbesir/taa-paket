// Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05]
//! paket.json modeli ve doğrulaması (T1: klasör paketi). Yol kaçışı, boyut sınırı, sembolik bağ ve SHA256 denetimi.

use crate::KurHata;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const MAKS_DOSYA: usize = 10_000;
/// Tasarımdaki "2 GB" ikili (GiB) olarak uygulandı.
pub const MAKS_TOPLAM_BAYT: u64 = 2 * 1024 * 1024 * 1024;
const MAKS_PAKET_JSON: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DosyaKaydi {
    /// Paket klasörüne göre "/" ayraçlı göreli yol.
    pub yol: String,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Paket {
    pub sema: u32,
    pub ad: String,
    pub surum: String,
    pub arayuz: String,
    pub program: String,
    #[serde(default)]
    pub gerektirir: BTreeMap<String, String>,
    #[serde(default)]
    pub sunar: Vec<String>,
    #[serde(default)]
    pub ister: Vec<String>,
    #[serde(default)]
    pub veri_sahibi: Vec<String>,
    #[serde(default)]
    pub veri_okur: Vec<String>,
    pub dosyalar: Vec<DosyaKaydi>,
    pub icerik_sha256: String,
    /// Ayrılmış alan (T3 imza): şimdilik `null`. Rust bu alanı yok sayar, doğrulamaz.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub imza: Option<serde_json::Value>,
}

pub fn sha256_bayt(b: &[u8]) -> String {
    hex(&Sha256::digest(b))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// Dosyanın (SHA256, boyut) değerini akışla hesaplar.
pub fn sha256_dosya(yol: &Path) -> std::io::Result<(String, u64)> {
    let mut f = fs::File::open(yol)?;
    let mut h = Sha256::new();
    let mut tampon = vec![0u8; 1 << 16];
    let mut toplam = 0u64;
    loop {
        let n = f.read(&mut tampon)?;
        if n == 0 {
            break;
        }
        h.update(&tampon[..n]);
        toplam += n as u64;
    }
    Ok((hex(&h.finalize()), toplam))
}

/// icerik_sha256: dosyalar yol sırasıyla, her biri için "{yol}\n{sha256}\n" (UTF-8) SHA256'ya beslenir.
pub fn icerik_hesapla(dosyalar: &[DosyaKaydi]) -> String {
    let mut sirali: Vec<&DosyaKaydi> = dosyalar.iter().collect();
    sirali.sort_by(|a, b| a.yol.cmp(&b.yol));
    let mut h = Sha256::new();
    for d in sirali {
        h.update(d.yol.as_bytes());
        h.update(b"\n");
        h.update(d.sha256.as_bytes());
        h.update(b"\n");
    }
    hex(&h.finalize())
}

const WINDOWS_AYRILMIS: [&str; 22] = [
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Ad ve sürüm için ortak kural: yalnız küçük harf `[a-z0-9.+-]`, 1..=64 karakter, baş/son `.` ya da `-` olamaz,
/// ardışık `..` olamaz, ilk noktadan önceki kısım Windows ayrılmış adı (con, prn, aux, nul, com1-9, lpt1-9) olamaz.
fn parca_gecerli(s: &str) -> bool {
    let b = s.as_bytes();
    if b.is_empty() || b.len() > 64 {
        return false;
    }
    if !b
        .iter()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(*c, b'.' | b'+' | b'-'))
    {
        return false;
    }
    if matches!(b[0], b'.' | b'-') || matches!(b[b.len() - 1], b'.' | b'-') || s.contains("..") {
        return false;
    }
    let temel = s.split('.').next().unwrap_or("");
    !WINDOWS_AYRILMIS.contains(&temel)
}

/// Paket ya da modül adı: [`parca_gecerli`] kuralları; ilk karakter harf ya da rakam.
pub fn ad_gecerli(ad: &str) -> bool {
    parca_gecerli(ad) && ad.as_bytes()[0] != b'+'
}

/// Sürüm: rakamla başlar; [`parca_gecerli`] kuralları.
pub fn surum_gecerli(s: &str) -> bool {
    parca_gecerli(s) && s.as_bytes()[0].is_ascii_digit()
}

/// Göreli paket yolu denetimi: boş, 0x20 altı denetim karakterleri ve 0x7F (NUL dahil), ters eğik çizgi, sürücü harfi/`:`, mutlak yol, `.`/`..`/boş parça reddedilir.
pub fn yol_dogrula(yol: &str) -> Result<(), String> {
    if yol.is_empty() {
        return Err("boş yol".into());
    }
    if yol.chars().any(|c| (c as u32) < 0x20 || c as u32 == 0x7F) {
        return Err(format!("yolda denetim karakteri var: {yol:?}"));
    }
    if yol.contains('\\') {
        return Err(format!("yolda ters eğik çizgi var: {yol}"));
    }
    if yol.contains(':') {
        return Err(format!("yolda sürücü harfi ya da ':' var: {yol}"));
    }
    if yol.starts_with('/') {
        return Err(format!("mutlak yol: {yol}"));
    }
    for parca in yol.split('/') {
        match parca {
            "" => return Err(format!("boş yol parçası: {yol}")),
            "." => return Err(format!("'.' parçalı yol: {yol}")),
            ".." => return Err(format!("üst klasöre çıkan yol: {yol}")),
            _ => {}
        }
        if parca.ends_with('.') || parca.ends_with(' ') {
            return Err(format!("yol parçası nokta ya da boşlukla bitiyor: {yol}"));
        }
        if windows_aygit_adi(parca) {
            return Err(format!("Windows aygıt adı: {yol}"));
        }
    }
    Ok(())
}

/// Windows ayrılmış aygıt adları (uzantılı biçimleri de): CON, PRN, AUX, NUL, COM1-9, LPT1-9. Harfe duyarsız.
pub fn windows_aygit_adi(parca: &str) -> bool {
    let govde = parca.split('.').next().unwrap_or("").to_ascii_lowercase();
    matches!(govde.as_str(), "con" | "prn" | "aux" | "nul")
        || ((govde.starts_with("com") || govde.starts_with("lpt"))
            && govde.len() == 4
            && govde.as_bytes()[3].is_ascii_digit()
            && govde.as_bytes()[3] != b'0')
}

/// Paket klasörü içindeki göreli yolu gerçek yola çevirir; ara klasörlerde ya da dosyada sembolik bağ varsa reddeder.
pub fn guvenli_coz(klasor: &Path, yol: &str) -> Result<PathBuf, String> {
    yol_dogrula(yol)?;
    let mut p = klasor.to_path_buf();
    for parca in yol.split('/') {
        p.push(parca);
        match fs::symlink_metadata(&p) {
            Ok(m) if m.file_type().is_symlink() => {
                return Err(format!("sembolik bağ izlenmez: {yol}"));
            }
            Ok(_) => {}
            Err(e) => return Err(format!("{yol} okunamadı: {e}")),
        }
    }
    Ok(p)
}

/// paket.json dosyasını okur ve ayrıştırır (içerik doğrulaması yapmaz). Ham baytları da döner.
pub fn paket_oku(klasor: &Path) -> Result<(Paket, Vec<u8>), KurHata> {
    let yol = klasor.join("paket.json");
    let m = fs::symlink_metadata(&yol).map_err(|e| {
        KurHata::Dogrulama(format!(
            "paket.json okunamadı ({}): {e}. Öneri: klasörde paket.json olmalı.",
            klasor.display()
        ))
    })?;
    if !m.is_file() || m.len() > MAKS_PAKET_JSON {
        return Err(KurHata::Dogrulama(
            "paket.json normal dosya değil ya da çok büyük. Öneri: paketi yeniden üretin.".into(),
        ));
    }
    let bayt = fs::read(&yol)?;
    let paket: Paket = serde_json::from_slice(&bayt).map_err(|e| {
        KurHata::Dogrulama(format!(
            "paket.json bozuk: {e}. Öneri: tools/paket_uret.py ile yeniden üretin."
        ))
    })?;
    Ok((paket, bayt))
}

fn dogrula_alanlar(p: &Paket) -> Result<(), String> {
    if p.sema != 1 {
        return Err(format!("desteklenmeyen paket şeması: {}", p.sema));
    }
    if !ad_gecerli(&p.ad) {
        return Err(format!("geçersiz paket adı: {:?}", p.ad));
    }
    if !surum_gecerli(&p.surum) {
        return Err(format!("geçersiz sürüm: {:?}", p.surum));
    }
    if p.arayuz.is_empty() || p.program.is_empty() {
        return Err("arayuz ve program boş olamaz".into());
    }
    for (g, a) in &p.gerektirir {
        if !ad_gecerli(g) {
            return Err(format!("geçersiz gerektirir adı: {g:?}"));
        }
        if g == &p.ad {
            return Err("paket kendini gerektiremez".into());
        }
        if a.is_empty() {
            return Err(format!("{g} için arayüz sürümü boş"));
        }
    }
    if p.dosyalar.len() > MAKS_DOSYA {
        return Err(format!(
            "dosya sayısı sınırı aşıldı ({} > {MAKS_DOSYA})",
            p.dosyalar.len()
        ));
    }
    let mut gorulen = BTreeSet::new();
    for d in &p.dosyalar {
        yol_dogrula(&d.yol)?;
        if d.yol == "paket.json" {
            return Err("dosyalar listesinde paket.json olamaz".into());
        }
        if !gorulen.insert(d.yol.as_str()) {
            return Err(format!("yol iki kez listelenmiş: {}", d.yol));
        }
        if d.sha256.len() != 64
            || !d
                .sha256
                .bytes()
                .all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
        {
            return Err(format!("geçersiz sha256 ({}): {}", d.yol, d.sha256));
        }
    }
    Ok(())
}

/// Paket klasörünü tam doğrular: alanlar, yollar, sınırlar, sembolik bağ, her dosyanın SHA256'sı ve icerik_sha256.
pub fn paket_dogrula(klasor: &Path) -> Result<(Paket, Vec<u8>), KurHata> {
    let (paket, bayt) = paket_oku(klasor)?;
    let hata = |m: String| KurHata::Dogrulama(format!("{m}. Öneri: paketi yeniden üretin."));
    dogrula_alanlar(&paket).map_err(hata)?;
    if icerik_hesapla(&paket.dosyalar) != paket.icerik_sha256 {
        return Err(hata("icerik_sha256 dosya listesiyle uyuşmuyor".into()));
    }
    let mut toplam = 0u64;
    for d in &paket.dosyalar {
        let yol = guvenli_coz(klasor, &d.yol).map_err(hata)?;
        let m = fs::symlink_metadata(&yol)?;
        if !m.is_file() {
            return Err(hata(format!("{} normal dosya değil", d.yol)));
        }
        toplam = toplam.saturating_add(m.len());
        if toplam > MAKS_TOPLAM_BAYT {
            return Err(hata("toplam boyut sınırı (2 GiB) aşıldı".into()));
        }
        let (sha, _) = sha256_dosya(&yol)?;
        if sha != d.sha256 {
            return Err(hata(format!("{} için sha256 uyuşmuyor", d.yol)));
        }
    }
    Ok((paket, bayt))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn windows_aygit_adlari_ve_sondaki_nokta_reddedilir() {
        for kotu in [
            "NUL",
            "bin/con.exe",
            "COM1",
            "lpt9.txt",
            "son.",
            "bosluk /x",
            "a/AUX.a.b",
        ] {
            assert!(yol_dogrula(kotu).is_err(), "{kotu}");
        }
        for iyi in ["com10", "console.txt", "nula", "lpt0", "bin/x.exe"] {
            assert!(yol_dogrula(iyi).is_ok(), "{iyi}");
        }
    }

    #[test]
    fn yol_kurallari() {
        for iyi in ["bin/a.exe", "a", "a/b/c.txt"] {
            assert!(yol_dogrula(iyi).is_ok(), "{iyi}");
        }
        for kotu in [
            "", "../x", "a/../b", "/etc/x", "C:/x", "a\\b", "a//b", "./a", "a/.", "a\0b", "d:x",
            "a\nb", "a\tb", "a\x1fb", "a\x7fb", "\x01",
        ] {
            assert!(yol_dogrula(kotu).is_err(), "{kotu:?}");
        }
    }

    #[test]
    fn ad_ve_surum() {
        assert!(
            ad_gecerli("taa-kur") && !ad_gecerli("../x") && !ad_gecerli("A") && !ad_gecerli("")
        );
        assert!(surum_gecerli("0.1.0") && !surum_gecerli("..") && !surum_gecerli("a/b"));
    }

    #[test]
    fn ad_ve_surum_sikilastirildi() {
        for iyi in ["taa-kur", "a", "a.b", "a+b", "x1-2", "con1x", "comx"] {
            assert!(ad_gecerli(iyi), "{iyi}");
        }
        for kotu in [
            "a_b", "A", "Taa", ".a", "a.", "-a", "a-", "a..b", "+a", "con", "prn", "aux", "nul",
            "com1", "com9", "lpt1", "lpt9", "con.txt", "a b", "a/b", "a\\b", "a\0", "ü",
        ] {
            assert!(!ad_gecerli(kotu), "{kotu:?}");
        }
        assert!(ad_gecerli(&"a".repeat(64)) && !ad_gecerli(&"a".repeat(65)));
        for iyi in ["0.1.0", "1", "1.0.0-rc1", "2.0+yapi5", "10.0.0-alfa.1"] {
            assert!(surum_gecerli(iyi), "{iyi}");
        }
        for kotu in [
            "",
            "1.",
            "1-",
            ".1",
            "-1",
            "1..2",
            "1.0.0-RC1",
            "1_0",
            "v1",
            "1 0",
            "1/2",
            "1\\2",
            "1\x01",
            "1\u{7f}",
        ] {
            assert!(!surum_gecerli(kotu), "{kotu:?}");
        }
        assert!(surum_gecerli(&format!("1{}", "0".repeat(63))));
        assert!(!surum_gecerli(&format!("1{}", "0".repeat(64))));
    }
}
