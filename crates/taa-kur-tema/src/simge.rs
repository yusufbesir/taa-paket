// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz coPLkm 2026-11-17]
//! Geçme Temel logosunun **kenar yumuşatmalı** (HD) taranması ve Windows simge dosyaları.
//!
//! GDI `Polygon` köşeleri tam piksele yuvarlar ve kenar yumuşatma yapmaz; 48 birimlik ızgara 16–40 px'e
//! inince parçalar düzensiz kalınlaşır, logo "piksel piksel" görünür. Burada logo, her piksel alt
//! örneklerle (16×16, büyük boyutta 4×4) taranıp kapsama oranıyla boyanır: çıktı önceden çarpılmış
//! (premultiplied) 32 bit BGRA; GDI tarafında `AlphaBlend` ile basılır, simgede alfa kanalı olarak kullanılır.
//!
//! Ayrıca: çok boyutlu `.ico` dosyası ve exe'ye gömülecek `.rsrc` COFF nesnesi (build.rs'ler kullanır;
//! dış araç, rc.exe ya da windres gerekmez).

use crate::logo::{ayristir, ANA_PATH, KUTU, VURGU_PATH};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// Logonun altına konan karo (simgelerde krem kare; Mezura açıkta koyu kare).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Karo {
    /// Karo rengi `0xRRGGBB`.
    pub renk: u32,
    /// Köşe yarıçapı, kenara oranla (0 = keskin köşe).
    pub yaricap: f64,
    /// Logonun karo içindeki payı, kenara oranla (her yandan).
    pub pay: f64,
}

/// Tarama isteği.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogoCizimi {
    /// Kare kenarı (piksel).
    pub boyut: u32,
    /// T (ana parça) rengi `0xRRGGBB`.
    pub ana: u32,
    /// Yuva (vurgu parçası) rengi `0xRRGGBB`.
    pub vurgu: u32,
    /// Vurgu parçasına içten 1 px çerçeve (Mezura açık temada sarı yuvayı belirginleştirir).
    pub vurgu_kenari: Option<u32>,
    /// İsteğe bağlı alt karo.
    pub karo: Option<Karo>,
}

impl LogoCizimi {
    /// Karosuz, çerçevesiz istek.
    pub fn yeni(boyut: u32, ana: u32, vurgu: u32) -> Self {
        LogoCizimi {
            boyut,
            ana,
            vurgu,
            vurgu_kenari: None,
            karo: None,
        }
    }

    fn anahtar(&self) -> Anahtar {
        let k = self.karo.unwrap_or(Karo {
            renk: u32::MAX,
            yaricap: -1.0,
            pay: -1.0,
        });
        (
            self.boyut,
            self.ana,
            self.vurgu,
            self.vurgu_kenari.unwrap_or(u32::MAX),
            k.renk as u64,
            k.yaricap.to_bits(),
            k.pay.to_bits(),
        )
    }
}

/// Önceden çarpılmış BGRA piksel (`0xAARRGGBB`; bellekte B, G, R, A sırası = 32 bit DIB). Satırlar
/// yukarıdan aşağı, `boyut * boyut` öğe.
pub fn tara(c: &LogoCizimi) -> Vec<u32> {
    let n = c.boyut as usize;
    if n == 0 {
        return Vec::new();
    }
    // Küçük boyutta ince alt örnek; 256 px'te 4×4 yeterli (piksel başına 1/16 kapsama adımı).
    let s: usize = if n <= 96 { 16 } else { 4 };
    let (ox, oy, kenar) = match c.karo {
        Some(k) => {
            let p = k.pay.clamp(0.0, 0.45) * n as f64;
            (p, p, n as f64 - 2.0 * p)
        }
        None => (0.0, 0.0, n as f64),
    };
    let olcek = kenar / KUTU;
    // 48 px altında köşeler tam piksele oturtulur (ipucu): çubuk kenarları bulanıklaşmaz, keskin kalır.
    // Büyük boyutta (açılış ekranı, 48+ simgeler) gerçek oranlar ve kenar yumuşatma kullanılır.
    let ipucu = kenar < 48.0;
    let sabitle = |v: f64| if ipucu { v.round() } else { v };
    let yol = |d: &str| -> Vec<Vec<(f64, f64)>> {
        ayristir(d)
            .unwrap_or_default()
            .into_iter()
            .map(|y| {
                y.into_iter()
                    .map(|(x, yy)| (ox + sabitle(x * olcek), oy + sabitle(yy * olcek)))
                    .collect()
            })
            .collect()
    };
    let ana = yol(ANA_PATH);
    let vurgu = yol(VURGU_PATH);
    let mut cikti = vec![0u32; n * n];
    let adim = 1.0 / s as f64;
    for py in 0..n {
        for px in 0..n {
            // Her renk için kapsanan alt örnek sayısı.
            let (mut k_karo, mut k_ana, mut k_vurgu, mut k_kenar) = (0u32, 0u32, 0u32, 0u32);
            for sy in 0..s {
                let y = py as f64 + (sy as f64 + 0.5) * adim;
                for sx in 0..s {
                    let x = px as f64 + (sx as f64 + 0.5) * adim;
                    let karoda = match c.karo {
                        Some(k) => yuvarlak_kare_icinde(x, y, n as f64, k.yaricap * n as f64),
                        None => false,
                    };
                    if ic_mi(&ana, x, y) {
                        k_ana += 1;
                    } else if ic_mi(&vurgu, x, y) {
                        match c.vurgu_kenari {
                            Some(_) if kenara_uzaklik(&vurgu, x, y) < 1.0 => k_kenar += 1,
                            _ => k_vurgu += 1,
                        }
                    } else if karoda {
                        k_karo += 1;
                    }
                }
            }
            let toplam = (s * s) as f64;
            let mut r = 0.0;
            let mut g = 0.0;
            let mut b = 0.0;
            let mut a = 0.0;
            let mut ekle = |renk: u32, sayi: u32| {
                if sayi == 0 {
                    return;
                }
                let w = sayi as f64 / toplam;
                r += ((renk >> 16) & 0xFF) as f64 * w;
                g += ((renk >> 8) & 0xFF) as f64 * w;
                b += (renk & 0xFF) as f64 * w;
                a += w;
            };
            if let Some(k) = c.karo {
                ekle(k.renk, k_karo);
            }
            ekle(c.ana, k_ana);
            ekle(c.vurgu, k_vurgu);
            if let Some(kr) = c.vurgu_kenari {
                ekle(kr, k_kenar);
            }
            // Karo varken logo parçaları karonun üstündedir; karo dışına taşan alt örnek olmaz
            // (logo karonun içinde), bu yüzden toplam alfa yukarıdaki toplamdır.
            let yuvarla = |v: f64| (v.round().clamp(0.0, 255.0)) as u32;
            cikti[py * n + px] =
                (yuvarla(a * 255.0) << 24) | (yuvarla(r) << 16) | (yuvarla(g) << 8) | yuvarla(b);
        }
    }
    cikti
}

type Anahtar = (u32, u32, u32, u32, u64, u64, u64);

thread_local! {
    static ONBELLEK: RefCell<HashMap<Anahtar, Rc<Vec<u32>>>> = RefCell::new(HashMap::new());
}

/// [`tara`] ile aynı; aynı istek (boyut + renkler) tekrar taranmaz. Pencere her karede logo çizdiği
/// için kullanılır; 48 farklı istekten sonra önbellek boşaltılır (tema/DPI değişimi).
pub fn tara_onbellekli(c: &LogoCizimi) -> Rc<Vec<u32>> {
    let anahtar = c.anahtar();
    ONBELLEK.with(|o| {
        let mut o = o.borrow_mut();
        if let Some(v) = o.get(&anahtar) {
            return v.clone();
        }
        if o.len() >= 48 {
            o.clear();
        }
        let v = Rc::new(tara(c));
        o.insert(anahtar, v.clone());
        v
    })
}

/// Çift-tek kuralıyla nokta çokgen(ler) içinde mi.
fn ic_mi(yollar: &[Vec<(f64, f64)>], x: f64, y: f64) -> bool {
    let mut ic = false;
    for yol in yollar {
        let n = yol.len();
        if n < 3 {
            continue;
        }
        let mut j = n - 1;
        for i in 0..n {
            let (xi, yi) = yol[i];
            let (xj, yj) = yol[j];
            if (yi > y) != (yj > y) && x < (xj - xi) * (y - yi) / (yj - yi) + xi {
                ic = !ic;
            }
            j = i;
        }
    }
    ic
}

/// Noktanın çokgen kenarlarına en kısa uzaklığı.
fn kenara_uzaklik(yollar: &[Vec<(f64, f64)>], x: f64, y: f64) -> f64 {
    let mut en = f64::MAX;
    for yol in yollar {
        let n = yol.len();
        for i in 0..n {
            let (ax, ay) = yol[i];
            let (bx, by) = yol[(i + 1) % n];
            let (dx, dy) = (bx - ax, by - ay);
            let uz2 = dx * dx + dy * dy;
            let t = if uz2 == 0.0 {
                0.0
            } else {
                (((x - ax) * dx + (y - ay) * dy) / uz2).clamp(0.0, 1.0)
            };
            let (qx, qy) = (ax + t * dx - x, ay + t * dy - y);
            en = en.min((qx * qx + qy * qy).sqrt());
        }
    }
    en
}

fn yuvarlak_kare_icinde(x: f64, y: f64, n: f64, r: f64) -> bool {
    if x < 0.0 || y < 0.0 || x > n || y > n {
        return false;
    }
    let r = r.clamp(0.0, n / 2.0);
    let cx = x.clamp(r, n - r);
    let cy = y.clamp(r, n - r);
    let (dx, dy) = (x - cx, y - cy);
    dx * dx + dy * dy <= r * r
}

// ------------------------------------------------------------------ simge dosyaları

/// Uygulama simgesinin karosu (İlmek açık: krem zemin, mürdüm yuva, koyu T). Görev çubuğunda açık ve
/// koyu zeminde de okunur.
pub fn uygulama_simgesi(boyut: u32) -> LogoCizimi {
    let j = crate::Tema::Ilmek.jetonlar(false);
    LogoCizimi {
        boyut,
        ana: j.metin,
        vurgu: j.vurgu,
        vurgu_kenari: None,
        karo: Some(Karo {
            renk: j.zemin,
            yaricap: 0.22,
            pay: 0.12,
        }),
    }
}

/// `.ico` içine konan boyutlar (Windows Gezgini, görev çubuğu, Alt+Tab, %100–%400 ölçek).
pub const ICO_BOYUTLARI: [u32; 8] = [16, 20, 24, 32, 40, 48, 64, 256];

/// Tek simge görüntüsü (ICO/RT_ICON verisi): BITMAPINFOHEADER (yükseklik ×2) + alt-üst 32 bit BGRA
/// (önceden çarpılmamış, düz alfa) + 1 bit AND maskesi (hepsi 0; saydamlık alfadan).
pub fn ico_goruntusu(c: &LogoCizimi) -> Vec<u8> {
    let n = c.boyut as usize;
    let piks = tara(c);
    let maske_satir = n.div_ceil(32) * 4;
    let mut v = Vec::with_capacity(40 + n * n * 4 + maske_satir * n);
    v.extend_from_slice(&40u32.to_le_bytes());
    v.extend_from_slice(&(n as i32).to_le_bytes());
    v.extend_from_slice(&((n * 2) as i32).to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&32u16.to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes()); // BI_RGB
    v.extend_from_slice(&((n * n * 4 + maske_satir * n) as u32).to_le_bytes());
    v.extend_from_slice(&[0u8; 16]);
    for y in (0..n).rev() {
        for x in 0..n {
            let p = piks[y * n + x];
            let a = p >> 24;
            let duz = |k: u32| {
                (k * 255 + a / 2)
                    .checked_div(a)
                    .map_or(0, |v| v.min(255) as u8)
            };
            v.extend_from_slice(&[
                duz(p & 0xFF),
                duz((p >> 8) & 0xFF),
                duz((p >> 16) & 0xFF),
                a as u8,
            ]);
        }
    }
    v.extend(std::iter::repeat_n(0u8, maske_satir * n));
    v
}

/// Çok boyutlu `.ico` dosyası.
pub fn ico_dosyasi(boyutlar: &[u32]) -> Vec<u8> {
    let goruntuler: Vec<Vec<u8>> = boyutlar
        .iter()
        .map(|&b| ico_goruntusu(&uygulama_simgesi(b)))
        .collect();
    let mut v = Vec::new();
    v.extend_from_slice(&0u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&(boyutlar.len() as u16).to_le_bytes());
    let mut konum = 6 + 16 * boyutlar.len();
    for (b, g) in boyutlar.iter().zip(&goruntuler) {
        let kb = if *b >= 256 { 0 } else { *b as u8 };
        v.extend_from_slice(&[kb, kb, 0, 0]);
        v.extend_from_slice(&1u16.to_le_bytes());
        v.extend_from_slice(&32u16.to_le_bytes());
        v.extend_from_slice(&(g.len() as u32).to_le_bytes());
        v.extend_from_slice(&(konum as u32).to_le_bytes());
        konum += g.len();
    }
    for g in &goruntuler {
        v.extend_from_slice(g);
    }
    v
}

/// Exe simgesi için x86-64 COFF nesnesi: tek `.rsrc` bölümü (RT_ICON 1..n + RT_GROUP_ICON 1, dil
/// 0x0409). Bağlayıcıya (`cargo:rustc-link-arg`) verilir; lld, GNU ld ve link.exe `.rsrc` bölümlü
/// nesneleri kaynak olarak birleştirir. İlk simge grubu Gezgin'de exe simgesi olur.
pub fn rsrc_nesnesi(boyutlar: &[u32]) -> Vec<u8> {
    const RT_ICON: u32 = 3;
    const RT_GROUP_ICON: u32 = 14;
    const DIL: u32 = 0x0409;
    let goruntuler: Vec<Vec<u8>> = boyutlar
        .iter()
        .map(|&b| ico_goruntusu(&uygulama_simgesi(b)))
        .collect();
    // GRPICONDIR
    let mut grup = Vec::new();
    grup.extend_from_slice(&0u16.to_le_bytes());
    grup.extend_from_slice(&1u16.to_le_bytes());
    grup.extend_from_slice(&(boyutlar.len() as u16).to_le_bytes());
    for (i, (b, g)) in boyutlar.iter().zip(&goruntuler).enumerate() {
        let kb = if *b >= 256 { 0 } else { *b as u8 };
        grup.extend_from_slice(&[kb, kb, 0, 0]);
        grup.extend_from_slice(&1u16.to_le_bytes());
        grup.extend_from_slice(&32u16.to_le_bytes());
        grup.extend_from_slice(&(g.len() as u32).to_le_bytes());
        grup.extend_from_slice(&((i + 1) as u16).to_le_bytes());
    }

    let n = goruntuler.len();
    // Dizin düzeni (her dizin 16 bayt başlık + 8 bayt/öğe):
    // kök(2) | RT_ICON adları(n) | RT_GROUP_ICON adı(1) | n×dil(1) | 1×dil(1) | (n+1) veri girdisi (16) | veriler
    let dizin = |oge: usize| 16 + 8 * oge;
    let kok = 0usize;
    let ikon_adlari = kok + dizin(2);
    let grup_adi = ikon_adlari + dizin(n);
    let ikon_dilleri = grup_adi + dizin(1);
    let grup_dili = ikon_dilleri + n * dizin(1);
    let girdiler = grup_dili + dizin(1);
    let mut veri_konumu = girdiler + 16 * (n + 1);
    let mut b: Vec<u8> = Vec::new();
    let baslik = |b: &mut Vec<u8>, kimlikli: u16| {
        b.extend_from_slice(&[0u8; 12]);
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(&kimlikli.to_le_bytes());
    };
    let oge = |b: &mut Vec<u8>, ad: u32, hedef: usize, alt_dizin: bool| {
        b.extend_from_slice(&ad.to_le_bytes());
        let h = hedef as u32 | if alt_dizin { 0x8000_0000 } else { 0 };
        b.extend_from_slice(&h.to_le_bytes());
    };
    baslik(&mut b, 2);
    oge(&mut b, RT_ICON, ikon_adlari, true);
    oge(&mut b, RT_GROUP_ICON, grup_adi, true);
    baslik(&mut b, n as u16);
    for i in 0..n {
        oge(&mut b, (i + 1) as u32, ikon_dilleri + i * dizin(1), true);
    }
    baslik(&mut b, 1);
    oge(&mut b, 1, grup_dili, true);
    for i in 0..n {
        baslik(&mut b, 1);
        oge(&mut b, DIL, girdiler + 16 * i, false);
    }
    baslik(&mut b, 1);
    oge(&mut b, DIL, girdiler + 16 * n, false);
    debug_assert_eq!(b.len(), girdiler);
    // Veri girdileri: OffsetToData bölüm başına göre (yer değiştirme ADDR32NB ile RVA'ya çevrilir).
    let mut veriler: Vec<&[u8]> = goruntuler.iter().map(|g| g.as_slice()).collect();
    veriler.push(&grup);
    let mut yer_degistirmeler = Vec::new();
    let mut konumlar = Vec::new();
    for v in &veriler {
        veri_konumu = veri_konumu.next_multiple_of(8);
        konumlar.push(veri_konumu);
        veri_konumu += v.len();
    }
    for (v, k) in veriler.iter().zip(&konumlar) {
        yer_degistirmeler.push(b.len() as u32);
        b.extend_from_slice(&(*k as u32).to_le_bytes());
        b.extend_from_slice(&(v.len() as u32).to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
        b.extend_from_slice(&0u32.to_le_bytes());
    }
    for (v, k) in veriler.iter().zip(&konumlar) {
        b.resize(*k, 0);
        b.extend_from_slice(v);
    }
    b.resize(b.len().next_multiple_of(8), 0);

    // COFF: dosya başlığı (20) + bölüm başlığı (40) + ham veri + yer değiştirmeler (10'ar) + simge tablosu (18) + dizgi tablosu (4).
    let ham = 20 + 40;
    let yd = ham + b.len();
    let simgeler = yd + 10 * yer_degistirmeler.len();
    let mut o = Vec::new();
    o.extend_from_slice(&0x8664u16.to_le_bytes());
    o.extend_from_slice(&1u16.to_le_bytes());
    o.extend_from_slice(&0u32.to_le_bytes());
    o.extend_from_slice(&(simgeler as u32).to_le_bytes());
    o.extend_from_slice(&1u32.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.extend_from_slice(b".rsrc\0\0\0");
    o.extend_from_slice(&0u32.to_le_bytes());
    o.extend_from_slice(&0u32.to_le_bytes());
    o.extend_from_slice(&(b.len() as u32).to_le_bytes());
    o.extend_from_slice(&(ham as u32).to_le_bytes());
    o.extend_from_slice(&(yd as u32).to_le_bytes());
    o.extend_from_slice(&0u32.to_le_bytes());
    o.extend_from_slice(&(yer_degistirmeler.len() as u16).to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    // IMAGE_SCN_CNT_INITIALIZED_DATA | IMAGE_SCN_ALIGN_8BYTES | IMAGE_SCN_MEM_READ
    o.extend_from_slice(&(0x0000_0040u32 | 0x0040_0000 | 0x4000_0000).to_le_bytes());
    o.extend_from_slice(&b);
    for adres in &yer_degistirmeler {
        o.extend_from_slice(&adres.to_le_bytes());
        o.extend_from_slice(&0u32.to_le_bytes()); // simge 0: .rsrc
        o.extend_from_slice(&3u16.to_le_bytes()); // IMAGE_REL_AMD64_ADDR32NB
    }
    o.extend_from_slice(b".rsrc\0\0\0");
    o.extend_from_slice(&0u32.to_le_bytes());
    o.extend_from_slice(&1i16.to_le_bytes());
    o.extend_from_slice(&0u16.to_le_bytes());
    o.push(3); // IMAGE_SYM_CLASS_STATIC
    o.push(0);
    o.extend_from_slice(&4u32.to_le_bytes());
    o
}
