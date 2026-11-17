// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz 2026-11-17]
//! WCAG 2.x kontrast hesabı ve özel vurgu koruması (KARAR §5 "Vurgu rengi").

use crate::{Jetonlar, Tema};

/// Düzeltmede her adımın `metin` yönüne harman oranı (%4).
pub const ADIM_ORANI: f32 = 0.04;
/// Düzeltmede en çok adım sayısı.
pub const EN_COK_ADIM: u32 = 12;
/// Vurgunun `yuzey` üstünde gösterge (ilerleme, seçim, odak) olarak vermesi gereken en az kontrast.
pub const GOSTERGE_ESIK: f64 = 3.0;
/// `vurgu_ustu` yazısının vurgu dolgusu üstünde en az kontrastı (Mezura'da [`MEZURA_YAZI_ESIK`]).
pub const YAZI_ESIK: f64 = 4.5;
pub const MEZURA_YAZI_ESIK: f64 = 7.0;
/// Hata rengine "çok yakın" sayılan ton farkı (derece).
pub const HATA_TON_ESIK: f64 = 20.0;

/// Seçilen vurgu 12 adım düzeltmeyle de kurallara uymadı; temanın kendi rengi kalmalı.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Red {
    /// `denenen`: son (en çok düzeltilmiş) aday, yalnız bilgi için.
    KontrastYetersiz { denenen: u32 },
}

impl std::fmt::Display for Red {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Red::KontrastYetersiz { .. } => {
                f.write_str("Bu renk okunabilir kalmıyor; temanın rengi kullanılıyor.")
            }
        }
    }
}

fn kanal(rgb: u32, kaydir: u32) -> f64 {
    let c = ((rgb >> kaydir) & 0xFF) as f64 / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// WCAG bağıl parlaklık (0..1).
pub fn bagil_parlaklik(rgb: u32) -> f64 {
    0.2126 * kanal(rgb, 16) + 0.7152 * kanal(rgb, 8) + 0.0722 * kanal(rgb, 0)
}

/// İki renk arası kontrast oranı (1..21), sıra önemsiz.
pub fn kontrast(a: u32, b: u32) -> f64 {
    let (la, lb) = (bagil_parlaklik(a), bagil_parlaklik(b));
    let (acik, koyu) = if la >= lb { (la, lb) } else { (lb, la) };
    (acik + 0.05) / (koyu + 0.05)
}

/// Dolgu üstüne yazı rengi: saf siyah ya da saf beyazdan kontrastı yüksek olan.
/// (Biri her renkte >= 4,5:1 verir.)
pub fn ustune_yazi(dolgu: u32) -> u32 {
    if kontrast(dolgu, 0x000000) >= kontrast(dolgu, 0xFFFFFF) {
        0x000000
    } else {
        0xFFFFFF
    }
}

/// `a`'dan `b`'ye `t` (0..1) oranında kanal kanal doğrusal harman (sRGB). Hareketlerde
/// "%50 harman" ve vurgu düzeltmesinde kullanılır. Yalnız dolguya uygulanır, metne değil.
pub fn ara_renk(a: u32, b: u32, t: f32) -> u32 {
    let t = t.clamp(0.0, 1.0);
    let k = |s: u32| {
        let x = ((a >> s) & 0xFF) as f32;
        let y = ((b >> s) & 0xFF) as f32;
        ((x + (y - x) * t).round() as u32).min(255) << s
    };
    k(16) | k(8) | k(0)
}

/// Özel vurgu rengini temaya uydurur (KARAR §5):
///
/// 1. `vurgu_ustu` = [`ustune_yazi`]; vurgu üstünde >= 4,5:1 (Mezura'da 7:1) vermeli.
/// 2. Vurgu `yuzey` üstünde >= 3:1 vermeli (gösterge). Mezura açıkta bu adım atlanır: orada vurgu
///    hiçbir zaman tek başına gösterge olmaz, hep 2 px `kenar` çerçeveyle çizilir (§4.1, §5.5).
/// 3. Uymazsa renk `metin` yönüne %4'lük adımlarla (en çok 12) kaydırılır; ilk uyan aday döner.
///
/// `Ok((renk, duzeltildi_mi))`: `duzeltildi_mi` true ise arayüz "Okunabilirlik için renk biraz
/// koyulaştırıldı." notunu gösterir. `Err` ise renk reddedilir, temanın rengi kalır.
pub fn vurgu_duzelt(tema: Tema, koyu: bool, renk: u32) -> Result<(u32, bool), Red> {
    let j = tema.jetonlar(koyu);
    let renk = renk & 0xFF_FFFF;
    let yazi_esik = if tema == Tema::Mezura {
        MEZURA_YAZI_ESIK
    } else {
        YAZI_ESIK
    };
    let gosterge_gerekli = !tema.vurgu_cerceveli(koyu);
    let uygun = |c: u32| {
        (!gosterge_gerekli || kontrast(c, j.yuzey) >= GOSTERGE_ESIK)
            && kontrast(c, ustune_yazi(c)) >= yazi_esik
    };
    let mut aday = renk;
    for adim in 0..=EN_COK_ADIM {
        aday = ara_renk(renk, j.metin, adim as f32 * ADIM_ORANI);
        if uygun(aday) {
            return Ok((aday, adim > 0));
        }
    }
    Err(Red::KontrastYetersiz { denenen: aday })
}

/// Vurgu renkli metin (bağlantı) kullanılabilir mi? Değilse `metin` + alt çizgi kullanılır (§5.3).
pub fn vurgu_metin_olabilir(j: &Jetonlar) -> bool {
    [j.yuzey, j.yuzey2, j.zemin]
        .iter()
        .all(|&y| kontrast(j.vurgu, y) >= YAZI_ESIK)
}

/// (ton derece 0..360, doygunluk 0..1) — HSL.
fn ton_doygunluk(rgb: u32) -> (f64, f64) {
    let r = ((rgb >> 16) & 0xFF) as f64 / 255.0;
    let g = ((rgb >> 8) & 0xFF) as f64 / 255.0;
    let b = (rgb & 0xFF) as f64 / 255.0;
    let en_b = r.max(g).max(b);
    let en_k = r.min(g).min(b);
    let d = en_b - en_k;
    if d < 1e-9 {
        return (0.0, 0.0);
    }
    let l = (en_b + en_k) / 2.0;
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if en_b == r {
        60.0 * (((g - b) / d).rem_euclid(6.0))
    } else if en_b == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    (h, s)
}

/// Seçilen renk temanın `hata` rengine çok mu yakın (ton farkı < 20°)? Gri tonlar (doygunluk < %15)
/// hiçbir zaman yakın sayılmaz. Uyarı notu için (§5.4); rengi reddetmez.
pub fn hataya_yakin(tema: Tema, koyu: bool, renk: u32) -> bool {
    let (h1, s1) = ton_doygunluk(renk & 0xFF_FFFF);
    if s1 < 0.15 {
        return false;
    }
    let (h2, _) = ton_doygunluk(tema.jetonlar(koyu).hata);
    let fark = (h1 - h2).abs();
    fark.min(360.0 - fark) < HATA_TON_ESIK
}
