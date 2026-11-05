//! Sanal baskı denetimi: gerçek yazıcı olmadan ESC/POS akışını TM-T20X (80 mm, 576 nokta) kurallarıyla yürütür.
//! Bilinmeyen komut, taşan satır, dengesiz QR, bozuk bitmap ya da eksik kesim testi kırar.
//! Bu bir SİMÜLASYONDUR: gerçek kâğıt, mürekkep ve satır yüksekliği doğrulanmaz.

use super::*;

const NOKTA: usize = 576; // 80 mm kâğıt, 203 dpi
const SUTUN_A: usize = 48; // Font A 12 nokta
const SUTUN_B: usize = 64; // Font B 9 nokta

#[derive(Default, Debug)]
struct Sonuc {
    kesim: usize,
    logo: Vec<(usize, usize)>, // (genişlik nokta, yükseklik nokta)
    qr_kurulum: usize,
    qr_baski: usize,
    metin: String,
}

fn yurut(b: &[u8]) -> Result<Sonuc, String> {
    let mut s = Sonuc::default();
    let (mut font_b, mut genis2) = (false, false);
    let mut satir = 0usize;
    let mut i = 0;
    let al = |i: usize, n: usize| -> Result<&[u8], String> {
        b.get(i..i + n)
            .ok_or_else(|| format!("akış {i}. bayttan sonra kesik"))
    };
    while i < b.len() {
        match b[i] {
            0x1B => match *al(i + 1, 1)?.first().unwrap() {
                0x40 => i += 2,               // ESC @
                0x74 | 0x45 | 0x61 => i += 3, // ESC t n / E n / a n
                0x4D => {
                    font_b = al(i + 2, 1)?[0] == 1; // ESC M n
                    i += 3;
                }
                k => return Err(format!("bilinmeyen ESC 0x{k:02X} @{i}")),
            },
            0x1D => match *al(i + 1, 1)?.first().unwrap() {
                0x21 => {
                    genis2 = al(i + 2, 1)?[0] & 0xF0 != 0; // GS ! n
                    i += 3;
                }
                0x42 => i += 3, // GS B n
                0x56 => {
                    // GS V 66 n: kes
                    if al(i + 2, 1)?[0] != 0x42 {
                        return Err(format!("beklenmeyen GS V @{i}"));
                    }
                    s.kesim += 1;
                    i += 4;
                }
                0x76 => {
                    // GS v 0 m xL xH yL yH d...
                    let h = al(i + 2, 6)?;
                    if h[0] != 0x30 {
                        return Err("GS v biçimi 0 değil".into());
                    }
                    let (xb, y) = (
                        h[2] as usize + 256 * h[3] as usize,
                        h[4] as usize + 256 * h[5] as usize,
                    );
                    al(i + 8, xb * y)?;
                    s.logo.push((xb * 8, y));
                    i += 8 + xb * y;
                }
                0x28 => {
                    // GS ( k pL pH cn fn ...
                    let h = al(i + 2, 3)?;
                    if h[0] != 0x6B {
                        return Err("GS ( k dışında GS (".into());
                    }
                    let n = h[1] as usize + 256 * h[2] as usize;
                    let g = al(i + 5, n)?;
                    match g.get(1) {
                        Some(0x50) => s.qr_kurulum += 1, // veriyi belleğe al
                        Some(0x51) => s.qr_baski += 1,   // yazdır
                        _ => {}
                    }
                    i += 5 + n;
                }
                k => return Err(format!("bilinmeyen GS 0x{k:02X} @{i}")),
            },
            0x0A => {
                let sinir = if font_b { SUTUN_B } else { SUTUN_A } / if genis2 { 2 } else { 1 };
                if satir > sinir {
                    return Err(format!("satır taştı: {satir} > {sinir} (font_b={font_b})"));
                }
                s.metin.push('\n');
                satir = 0;
                i += 1;
            }
            c => {
                satir += 1;
                s.metin.push(c as char);
                i += 1;
            }
        }
    }
    if satir > 0 {
        return Err("akış son satırı LF ile bitirmiyor".into());
    }
    Ok(s)
}

fn ornek() -> Fis {
    let mut f = Fis {
        no: "F-20261105-007".into(),
        tarih: "2026-11-05".into(),
        musteri: "Ayşe Çelik".into(),
        telefon: "+905321234567".into(),
        kalemler: vec![Kalem {
            urun: "Pantolon".into(),
            islem: "Paça kısaltma (çok uzun bir açıklama satırı satır sarmasını denemek için)"
                .into(),
            not: "duble paça".into(),
            islem_kodu: None,
            varyant: None,
            olcu_mm: Some(25),
            pay_mm: None,
            usta_onayli: false,
            igneli: false,
        }],
        olcu_notu: String::new(),
        prova: String::new(),
        teslim: "2026-11-10".into(),
        teslim_saati: "17:00".into(),
        ucret_kurus: 0,
        on_odeme_kurus: 0,
        usta: "Hasan".into(),
        satis_danismani: "Mert".into(),
        olcu_alan: "Kadir".into(),
        kargo: false,
        kargo_notu: String::new(),
        kargo_ad: String::new(),
        kargo_adres: String::new(),
        kargo_telefon: String::new(),
        bildirim_izni: false,
        durum: Durum::Bekliyor,
        raf: String::new(),
        teslim_degisiklikleri: vec![],
        imzalar: vec![],
    };
    f.kalemler.push(f.kalemler[0].clone());
    f
}

#[test]
fn escpos_akisi_gecerli_ve_kesimle_biter() {
    let f = ornek();
    let b = escpos_nushalar_tam(
        &f,
        42,
        "F-20261105-007",
        Some("https://ornek.test/aydinlatma"),
    );
    let s = yurut(&b).expect("ESC/POS akışı geçerli olmalı");
    assert_eq!(s.kesim, 2, "her nüsha bir kesimle biter");
    assert_eq!(
        b[b.len() - 4..],
        [0x1D, 0x56, 0x42, 0],
        "akış kesimle bitmeli"
    );
    assert!(s.metin.len() > 100, "metin bölümü boş olmamalı");
}

#[test]
fn logo_576_nokta_ve_qr_dengeli() {
    let f = ornek();
    let b = escpos_nushalar_tam(
        &f,
        42,
        "F-20261105-007",
        Some("https://ornek.test/aydinlatma"),
    );
    let s = yurut(&b).unwrap();
    if logo_kullan() {
        assert!(!s.logo.is_empty());
        assert!(
            s.logo
                .iter()
                .all(|(w, h)| *w == NOKTA && (1..=200).contains(h)),
            "{:?}",
            s.logo
        );
    }
    // her nüshada fiş QR'ı, müşteri nüshasında ek olarak aydınlatma QR'ı
    assert_eq!(s.qr_kurulum, s.qr_baski);
    assert_eq!(s.qr_baski, 3);
}

#[test]
fn yurutucu_bozuk_akisi_yakalar() {
    assert!(yurut(&[0x1B, 0x99]).is_err(), "bilinmeyen komut");
    assert!(
        yurut(&[0x1D, 0x76, 0x30, 0, 72, 0, 10, 0, 1, 2]).is_err(),
        "kesik bitmap"
    );
    let tasan: Vec<u8> = std::iter::repeat_n(b'x', 49).chain(*b"\n").collect();
    assert!(yurut(&tasan).is_err(), "49 sütun Font A'ya sığmaz");
    assert!(yurut(b"abc").is_err(), "LF'siz son satır");
}
