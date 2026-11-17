// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz 2026-11-17]
//! TAA Kurucu logosu: **Geçme Temel** (Yusuf'un seçimi; LOGO-KARAR-FABLE.md §2).
//!
//! Blok T (ana renk, `currentColor`) alttaki U biçimli yuvaya (vurgu rengi) iner; ikisi birbirine
//! değmez. viewBox 0 0 48 48, 6 birimlik ızgara. Kurallar: T hiçbir zaman vurgu rengiyle boyanmaz;
//! Mezura sarısı açık zemine doğrudan basılmaz (koyu karo üstünde ya da tümü tek renk siyah).
//! Diskteki .svg dosyalarında c2pa metadata var; kaynak yalnız buradaki path'lerdir.

/// T (ana renk) path'i.
pub const ANA_PATH: &str = "M6 6H42V15H30V33H18V15H6Z";
/// U yuva (vurgu rengi) path'i.
pub const VURGU_PATH: &str = "M6 21H12V36H36V21H42V42H6Z";
/// viewBox kenarı.
pub const KUTU: f64 = 48.0;

/// Tek dosya simge metni (renkler `currentColor` ve `var(--vurgu, currentColor)`).
pub const SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 48 48\">\n  \
<path fill=\"currentColor\" d=\"M6 6H42V15H30V33H18V15H6Z\"/>\n  \
<path fill=\"var(--vurgu, currentColor)\" d=\"M6 21H12V36H36V21H42V42H6Z\"/>\n</svg>\n";

/// Logonun parçası.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Parca {
    /// T: `metin`/ana renk.
    Ana,
    /// Yuva: vurgu rengi.
    Vurgu,
}

/// Renkleri sabit SVG (`0xRRGGBB`). Tek renk için `ana == vurgu` verin.
pub fn svg(ana: u32, vurgu: u32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 48 48\">\n  \
<path fill=\"{}\" d=\"{}\"/>\n  <path fill=\"{}\" d=\"{}\"/>\n</svg>\n",
        crate::hex(ana),
        ANA_PATH,
        crate::hex(vurgu),
        VURGU_PATH
    )
}

/// SVG path ayrıştırıcı: yalnız mutlak `M H V L Z` komutları (büyük harf). Her alt yol (M ile
/// başlayan) ayrı köşe listesidir; `Z` kapatır, başlangıç noktası tekrar eklenmez. Komut tekrarı
/// (`L 1 2 3 4`, `M` sonrası örtük `L`) desteklenir.
pub fn ayristir(d: &str) -> Result<Vec<Vec<(f64, f64)>>, String> {
    let mut yollar: Vec<Vec<(f64, f64)>> = Vec::new();
    let mut gecerli: Vec<(f64, f64)> = Vec::new();
    let mut nokta = (0.0f64, 0.0f64);
    let mut komut: Option<char> = None;
    let b = d.as_bytes();
    let mut i = 0usize;

    let bosluk = |i: &mut usize| {
        while *i < b.len() && (b[*i].is_ascii_whitespace() || b[*i] == b',') {
            *i += 1;
        }
    };
    let sayi = |i: &mut usize| -> Result<f64, String> {
        bosluk(i);
        let bas = *i;
        if *i < b.len() && (b[*i] == b'-' || b[*i] == b'+') {
            *i += 1;
        }
        while *i < b.len() && (b[*i].is_ascii_digit() || b[*i] == b'.') {
            *i += 1;
        }
        let s = &d[bas..*i];
        s.parse::<f64>()
            .map_err(|_| format!("sayı bekleniyordu, konum {bas}"))
    };

    loop {
        bosluk(&mut i);
        if i >= b.len() {
            break;
        }
        let c = b[i] as char;
        if c.is_ascii_alphabetic() {
            i += 1;
            match c {
                'M' | 'H' | 'V' | 'L' => komut = Some(c),
                'Z' => {
                    if !gecerli.is_empty() {
                        nokta = gecerli[0];
                        yollar.push(std::mem::take(&mut gecerli));
                    }
                    komut = None;
                    continue;
                }
                _ => {
                    return Err(format!(
                        "desteklenmeyen komut '{c}' (yalnız mutlak M H V L Z)"
                    ))
                }
            }
        }
        match komut {
            Some('M') => {
                let x = sayi(&mut i)?;
                let y = sayi(&mut i)?;
                if !gecerli.is_empty() {
                    yollar.push(std::mem::take(&mut gecerli));
                }
                nokta = (x, y);
                gecerli.push(nokta);
                komut = Some('L');
            }
            Some('L') => {
                let x = sayi(&mut i)?;
                let y = sayi(&mut i)?;
                nokta = (x, y);
                gecerli.push(nokta);
            }
            Some('H') => {
                nokta.0 = sayi(&mut i)?;
                gecerli.push(nokta);
            }
            Some('V') => {
                nokta.1 = sayi(&mut i)?;
                gecerli.push(nokta);
            }
            _ => return Err(format!("komut bekleniyordu, konum {i}")),
        }
    }
    if !gecerli.is_empty() {
        yollar.push(gecerli);
    }
    Ok(yollar)
}

/// Logoyu `boyut_px` karelik alana, sol üst köşesi `(x, y)` olacak şekilde ölçekler. GDI
/// `Polygon` için tamsayı köşeler döner; önce Ana, sonra Vurgu. Köşe = `x + round(v * boyut / 48)`.
pub fn cokgenler(boyut_px: i32, x: i32, y: i32) -> Vec<(Parca, Vec<(i32, i32)>)> {
    let olcek = boyut_px as f64 / KUTU;
    let mut cikti = Vec::new();
    for (parca, d) in [(Parca::Ana, ANA_PATH), (Parca::Vurgu, VURGU_PATH)] {
        // Sabit path'ler testle doğrulanır; ayrıştırma burada başarısız olamaz.
        for yol in ayristir(d).unwrap_or_default() {
            let noktalar = yol
                .iter()
                .map(|&(px, py)| {
                    (
                        x + (px * olcek).round() as i32,
                        y + (py * olcek).round() as i32,
                    )
                })
                .collect();
            cikti.push((parca, noktalar));
        }
    }
    cikti
}
