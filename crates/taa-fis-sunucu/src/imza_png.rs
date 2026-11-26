//! Teslim imzası PNG'sinin İÇERİK denetimi (denetçi bulgusu 2026-11-26: yalnız PNG başlığına bakmak boş beyaz
//! tuvali, hatta 33 baytlık sahte "PNG"yi imza saydırıyordu; fişsiz teslimde imza zorunluluğu atlatılabiliyordu).
//!
//! Sunucu PNG'yi gerçekten çözer: parçaların CRC'si tutmalı, IHDR ilk / IEND son olmalı, IDAT zlib akışı açılmalı ve
//! tam satır sayısı kadar veri vermeli, satır süzgeçleri (None/Sub/Up/Average/Paeth) geçerli olmalı. Sonra koyu
//! piksel (beyaz zemine göre parlaklık < 128) sayılır: imza sayılması için en az `EN_AZ_KOYU` piksel ve tuvalin en az
//! binde 1,5'i koyu, en çok %60'ı koyu (tümü kara tuval imza değildir) ve koyu bölgenin eni tuval eninin en az onda
//! biri olmalı (tek nokta imza değildir). Bu denetim "gerçek imza" kanıtı DEĞİLDİR; yalnız boş/sahte gönderimi
//! keser. Aynı kural bulutta (bulut/musteri-takip/ortak/imza.mjs `imzaIcerik`) ve tarayıcı betiğinde vardır.
//!
//! Desteklenen biçim: 8 bit derinlik; renk türü 0 (gri: istemcinin asıl çıktısı), 2 (RGB), 4 (gri+alfa),
//! 6 (RGBA: tuval.toDataURL yedeği); taramalı (interlace) değil. Alfa beyaz zemine karıştırılır.
//!
//! Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

const PNG_IMZASI: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// İmza tuvali kenar üst sınırı (istemci 600×240 ya da 300×120 üretir; pay bırakıldı).
pub(crate) const EN_BUYUK_KENAR: u32 = 2048;
/// Koyu piksel alt sınırı (mutlak).
pub(crate) const EN_AZ_KOYU: usize = 60;

/// Çözülmüş imzanın özeti.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Ozet {
    pub en: u32,
    pub boy: u32,
    pub koyu: usize,
    /// Koyu piksellerin kapladığı sütun aralığının eni (0: koyu yok).
    pub koyu_en: u32,
}

fn u32_be(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

/// PNG'yi çözer ve koyu piksel özetini çıkarır. Hata: Türkçe kısa açıklama (kişisel veri içermez).
pub(crate) fn coz(b: &[u8]) -> Result<Ozet, String> {
    if b.len() < 8 + 25 + 12 || b[..8] != PNG_IMZASI {
        return Err("PNG değil".into());
    }
    let mut i = 8;
    let mut ihdr: Option<(u32, u32, u8, u8)> = None;
    let mut idat: Vec<u8> = Vec::new();
    let mut iend = false;
    while i < b.len() {
        if iend {
            return Err("IEND'den sonra veri var".into());
        }
        if b.len() - i < 12 {
            return Err("yarım parça".into());
        }
        let uz = u32_be(&b[i..]) as usize;
        if uz > b.len() - i - 12 {
            return Err("parça boyu dosyayı aşıyor".into());
        }
        let tur = &b[i + 4..i + 8];
        let veri = &b[i + 8..i + 8 + uz];
        let crc = u32_be(&b[i + 8 + uz..]);
        let mut h = crc32fast::Hasher::new();
        h.update(tur);
        h.update(veri);
        if h.finalize() != crc {
            return Err("parça CRC'si tutmuyor".into());
        }
        match tur {
            b"IHDR" => {
                if ihdr.is_some() || i != 8 || uz != 13 {
                    return Err("IHDR geçersiz".into());
                }
                let (en, boy) = (u32_be(veri), u32_be(&veri[4..]));
                let (derinlik, renk, sikistirma, suzgec, tarama) =
                    (veri[8], veri[9], veri[10], veri[11], veri[12]);
                if !(1..=EN_BUYUK_KENAR).contains(&en) || !(1..=EN_BUYUK_KENAR).contains(&boy) {
                    return Err("imza boyutu geçersiz".into());
                }
                if derinlik != 8 || !matches!(renk, 0 | 2 | 4 | 6) {
                    return Err("desteklenmeyen PNG biçimi (8 bit gri/RGB/RGBA olmalı)".into());
                }
                if sikistirma != 0 || suzgec != 0 || tarama != 0 {
                    return Err(
                        "desteklenmeyen PNG biçimi (taramalı ya da bilinmeyen sıkıştırma)".into(),
                    );
                }
                ihdr = Some((en, boy, renk, derinlik));
            }
            _ if ihdr.is_none() => return Err("ilk parça IHDR değil".into()),
            b"IDAT" => idat.extend_from_slice(veri),
            b"IEND" => iend = true,
            // Bilinmeyen kritik parça (ör. PLTE: paletli biçim desteklenmez) reddedilir; yardımcı parçalar geçer.
            t if t[0].is_ascii_uppercase() => {
                return Err("desteklenmeyen kritik PNG parçası".into())
            }
            _ => {}
        }
        i += 12 + uz;
    }
    let (en, boy, renk, _) = ihdr.ok_or("IHDR yok")?;
    if !iend {
        return Err("IEND yok".into());
    }
    let kanal: usize = match renk {
        0 => 1,
        2 => 3,
        4 => 2,
        _ => 4,
    };
    let satir = en as usize * kanal;
    let beklenen = (satir + 1) * boy as usize;
    let ham = miniz_oxide::inflate::decompress_to_vec_zlib_with_limit(&idat, beklenen + 1)
        .map_err(|_| "IDAT açılamadı".to_string())?;
    if ham.len() != beklenen {
        return Err("IDAT boyu tutmuyor".into());
    }
    let mut onceki = vec![0u8; satir];
    let mut simdiki = vec![0u8; satir];
    let (mut koyu, mut sol, mut sag) = (0usize, u32::MAX, 0u32);
    for y in 0..boy as usize {
        let s = &ham[y * (satir + 1)..(y + 1) * (satir + 1)];
        let (suz, veri) = (s[0], &s[1..]);
        for x in 0..satir {
            let a = if x >= kanal { simdiki[x - kanal] } else { 0 };
            let ust = onceki[x];
            let c = if x >= kanal { onceki[x - kanal] } else { 0 };
            let tahmin = match suz {
                0 => 0,
                1 => a,
                2 => ust,
                3 => ((a as u16 + ust as u16) / 2) as u8,
                4 => {
                    let p = a as i16 + ust as i16 - c as i16;
                    let (pa, pb, pc) = (
                        (p - a as i16).abs(),
                        (p - ust as i16).abs(),
                        (p - c as i16).abs(),
                    );
                    if pa <= pb && pa <= pc {
                        a
                    } else if pb <= pc {
                        ust
                    } else {
                        c
                    }
                }
                _ => return Err("PNG satır süzgeci geçersiz".into()),
            };
            simdiki[x] = veri[x].wrapping_add(tahmin);
        }
        for x in 0..en as usize {
            let p = &simdiki[x * kanal..(x + 1) * kanal];
            // Parlaklık (0..255) ve alfa; alfa beyaz zemine karıştırılır.
            let (l, alfa) = match renk {
                0 => (p[0] as u32, 255u32),
                4 => (p[0] as u32, p[1] as u32),
                2 => (
                    (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000,
                    255,
                ),
                _ => (
                    (p[0] as u32 * 299 + p[1] as u32 * 587 + p[2] as u32 * 114) / 1000,
                    p[3] as u32,
                ),
            };
            let gorunen = (l * alfa + 255 * (255 - alfa)) / 255;
            if gorunen < 128 {
                koyu += 1;
                sol = sol.min(x as u32);
                sag = sag.max(x as u32);
            }
        }
        std::mem::swap(&mut onceki, &mut simdiki);
    }
    Ok(Ozet {
        en,
        boy,
        koyu,
        koyu_en: if koyu == 0 { 0 } else { sag - sol + 1 },
    })
}

/// İmza sayılır mı: PNG çözülür ve içerik kuralı (modül başı) tutar. Hata: kısa Türkçe açıklama.
pub(crate) fn imza_yeterli(b: &[u8]) -> Result<Ozet, String> {
    let o = coz(b)?;
    let toplam = o.en as usize * o.boy as usize;
    if o.koyu < EN_AZ_KOYU.max(toplam * 15 / 10_000) {
        return Err("imza boş ya da çok kısa".into());
    }
    if o.koyu > toplam * 6 / 10 {
        return Err("imza tuvali neredeyse tümüyle koyu".into());
    }
    if o.koyu_en < o.en.div_ceil(10) {
        return Err("imza çok dar (tek nokta ya da çizgi)".into());
    }
    Ok(o)
}

/// Deneme PNG'si üretir (yalnız testler): beyaz zemin üstünde, `cizgi` ise birkaç kalın koyu çizgi (imza benzeri).
/// renk: 0 gri, 6 RGBA. Gerçek zlib (miniz_oxide) ve gerçek CRC ile; görüntü olarak açılır.
#[cfg(test)]
pub(crate) fn deneme_png(en: u32, boy: u32, renk: u8, cizgi: bool) -> Vec<u8> {
    let kanal = if renk == 6 { 4 } else { 1 };
    let mut ham = Vec::new();
    for y in 0..boy {
        ham.push(if y % 2 == 0 { 1 } else { 0 }); // Sub ve None süzgeçleri karışık
        let mut onceki = [0u8; 4];
        for x in 0..en {
            // İmza benzeri: iki eğri çizgi (3 px kalın), tuvalin yatayda yarısından fazlası.
            let ic = cizgi && x > en / 6 && x < en * 5 / 6 && {
                let m1 = boy / 2 + ((x as f32 / 9.0).sin() * boy as f32 / 5.0) as u32;
                let m2 = boy / 3 + x * boy / (3 * en);
                y.abs_diff(m1) <= 1 || y.abs_diff(m2) <= 1
            };
            let v = if ic { 20u8 } else { 255u8 };
            let px: Vec<u8> = if kanal == 4 {
                vec![v, v, v, 255]
            } else {
                vec![v]
            };
            for (k, &b) in px.iter().enumerate() {
                if y % 2 == 0 {
                    ham.push(b.wrapping_sub(onceki[k]));
                } else {
                    ham.push(b);
                }
                onceki[k] = b;
            }
        }
    }
    let z = miniz_oxide::deflate::compress_to_vec_zlib(&ham, 6);
    let parca = |tur: &[u8], veri: &[u8]| {
        let mut p = (veri.len() as u32).to_be_bytes().to_vec();
        p.extend_from_slice(tur);
        p.extend_from_slice(veri);
        let mut h = crc32fast::Hasher::new();
        h.update(tur);
        h.update(veri);
        p.extend_from_slice(&h.finalize().to_be_bytes());
        p
    };
    let mut ihdr = en.to_be_bytes().to_vec();
    ihdr.extend_from_slice(&boy.to_be_bytes());
    ihdr.extend_from_slice(&[8, renk, 0, 0, 0]);
    let mut b = PNG_IMZASI.to_vec();
    b.extend(parca(b"IHDR", &ihdr));
    b.extend(parca(b"IDAT", &z));
    b.extend(parca(b"IEND", &[]));
    b
}

/// Gri deneme PNG'si (yalnız testler): f(x, y) piksel değeri (0 kara, 255 beyaz), süzgeç None.
#[cfg(test)]
pub(crate) fn gri_png(en: u32, boy: u32, f: impl Fn(u32, u32) -> u8) -> Vec<u8> {
    let mut ham = Vec::new();
    for y in 0..boy {
        ham.push(0u8);
        for x in 0..en {
            ham.push(f(x, y));
        }
    }
    let z = miniz_oxide::deflate::compress_to_vec_zlib(&ham, 6);
    let mut b = deneme_png(en, boy, 0, false);
    b.truncate(8 + 25);
    for (tur, veri) in [(&b"IDAT"[..], &z[..]), (&b"IEND"[..], &[][..])] {
        b.extend_from_slice(&(veri.len() as u32).to_be_bytes());
        b.extend_from_slice(tur);
        b.extend_from_slice(veri);
        let mut h = crc32fast::Hasher::new();
        h.update(tur);
        h.update(veri);
        b.extend_from_slice(&h.finalize().to_be_bytes());
    }
    b
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn gercek_imza_gecer_bos_ve_sahte_gecmez() {
        for renk in [0, 6] {
            let o = imza_yeterli(&deneme_png(600, 240, renk, true)).unwrap();
            assert!(o.koyu > 1000 && o.koyu_en > 300, "{o:?}");
            assert!(imza_yeterli(&deneme_png(300, 120, renk, true)).is_ok());
            // boş beyaz tuval: geçerli PNG ama imza değil
            let bos = deneme_png(600, 240, renk, false);
            assert_eq!(coz(&bos).unwrap().koyu, 0);
            assert!(imza_yeterli(&bos).unwrap_err().contains("boş"));
        }
    }

    #[test]
    fn bozuk_png_reddedilir() {
        let iyi = deneme_png(200, 80, 0, true);
        assert!(coz(&iyi).is_ok());
        // CRC bozuk (IDAT CRC'sinin son baytı; IEND 12 bayt)
        let mut b = iyi.clone();
        let son = b.len() - 13;
        b[son] ^= 1;
        assert!(coz(&b).unwrap_err().contains("CRC"));
        // IEND yok
        assert!(coz(&iyi[..iyi.len() - 12]).is_err());
        // yalnız başlık (eski 33 baytlık sahte "PNG")
        assert!(coz(&iyi[..33]).is_err());
        // IEND'den sonra çöp
        let mut c = iyi.clone();
        c.extend_from_slice(b"cop");
        assert!(coz(&c).is_err());
    }

    #[test]
    fn tek_nokta_ve_kara_tuval_imza_degil() {
        // 600×240 beyaz, ortada 10×10 koyu kare: koyu 100 (≥60) ama oran ve en tutmaz.
        let b = gri_png(600, 240, |x, y| {
            if (295..305).contains(&x) && (115..125).contains(&y) {
                0
            } else {
                255
            }
        });
        let o = coz(&b).unwrap();
        assert_eq!((o.koyu, o.koyu_en), (100, 10));
        assert!(imza_yeterli(&b).is_err());
        // tümü kara
        let kara = gri_png(600, 240, |_, _| 0);
        assert!(imza_yeterli(&kara).unwrap_err().contains("koyu"));
        // Not: tuval boyunca tek düz çizgi (600 koyu piksel) geçer; denetim yalnız boş / nokta / kara tuvali
        // keser, gerçek imzayı kanıtlamaz (modül başı).
        assert!(imza_yeterli(&gri_png(600, 240, |_, y| if y == 100 { 0 } else { 255 })).is_ok());
    }
}
