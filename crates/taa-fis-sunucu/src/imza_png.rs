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
//! 6 (RGBA: tuval.toDataURL yedeği); taramalı (interlace) değil. Alfa beyaz zemine karıştırılır; renk türü 0/2'de
//! `tRNS` (saydam renk) varsa o renkteki pikseller de tam saydam sayılıp beyaz zemine karıştırılır (saydam
//! "mürekkep" koyu sayılmaz). `tRNS` 4/6'da (zaten alfalı) ya da IDAT'tan sonra geçersizdir.
//!
//! Sıkı yapı (denetçi bulgusu 2026-11-26): IEND verisi boş olmalı, IEND'den sonra hiçbir bayt olmamalı (PNG'nin
//! arkasına gizli veri eklenemez). Kabul edilen imza OLDUĞU GİBİ saklanmaz: `imza_temizle` görünen pikselleri
//! (beyaz zemine karıştırılmış gri) yeniden kodlar; yalnız IHDR + IDAT + IEND kalır, yardımcı parçalar (tEXt,
//! iTXt, eXIf, zTXt ...) ve içlerindeki olası kişisel/gizli veri atılır. Saklanan PNG 8 bit gridir.
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
#[cfg(test)]
pub(crate) fn coz(b: &[u8]) -> Result<Ozet, String> {
    coz_ic(b).map(|(o, _)| o)
}

/// PNG'yi çözer: (özet, beyaz zemine karıştırılmış görünen gri pikseller, satır satır `en * boy` bayt).
fn coz_ic(b: &[u8]) -> Result<(Ozet, Vec<u8>), String> {
    if b.len() < 8 + 25 + 12 || b[..8] != PNG_IMZASI {
        return Err("PNG değil".into());
    }
    let mut i = 8;
    let mut ihdr: Option<(u32, u32, u8, u8)> = None;
    let mut idat: Vec<u8> = Vec::new();
    let mut iend = false;
    // tRNS saydam rengi (renk 0: [gri, -, -]; renk 2: [r, g, b]); 16 bitlik örnekler (8 bit derinlikte üst bayt 0).
    let mut saydam: Option<[u16; 3]> = None;
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
            b"IEND" => {
                if uz != 0 {
                    return Err("IEND verisi boş değil".into());
                }
                iend = true
            }
            b"tRNS" => {
                let renk = ihdr.map(|h| h.2).unwrap_or(0);
                if saydam.is_some() || !idat.is_empty() {
                    return Err("tRNS geçersiz (tekrar ya da IDAT'tan sonra)".into());
                }
                let u16_be = |i: usize| u16::from_be_bytes([veri[i], veri[i + 1]]);
                saydam = Some(match (renk, uz) {
                    (0, 2) => [u16_be(0), 0, 0],
                    (2, 6) => [u16_be(0), u16_be(2), u16_be(4)],
                    _ => return Err("tRNS bu renk türünde geçersiz".into()),
                });
            }
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
    let mut gorunur: Vec<u8> = Vec::with_capacity(en as usize * boy as usize);
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
            // tRNS: saydam renkle birebir aynı piksel tam saydamdır (beyaz zemin görünür).
            let saydam_mi = match (renk, saydam) {
                (0, Some(s)) => s[0] == p[0] as u16,
                (2, Some(s)) => s == [p[0] as u16, p[1] as u16, p[2] as u16],
                _ => false,
            };
            let (l, alfa) = match renk {
                _ if saydam_mi => (0u32, 0u32),
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
            gorunur.push(gorunen as u8);
            if gorunen < 128 {
                koyu += 1;
                sol = sol.min(x as u32);
                sag = sag.max(x as u32);
            }
        }
        std::mem::swap(&mut onceki, &mut simdiki);
    }
    Ok((
        Ozet {
            en,
            boy,
            koyu,
            koyu_en: if koyu == 0 { 0 } else { sag - sol + 1 },
        },
        gorunur,
    ))
}

/// Bir PNG parçası: uzunluk + tür + veri + CRC.
fn parca(tur: &[u8], veri: &[u8]) -> Vec<u8> {
    let mut p = (veri.len() as u32).to_be_bytes().to_vec();
    p.extend_from_slice(tur);
    p.extend_from_slice(veri);
    let mut h = crc32fast::Hasher::new();
    h.update(tur);
    h.update(veri);
    p.extend_from_slice(&h.finalize().to_be_bytes());
    p
}

/// Gri pikselleri (satır satır `en * boy`) yalnız IHDR + IDAT + IEND içeren 8 bit gri PNG olarak kodlar.
fn gri_kodla(en: u32, boy: u32, gri: &[u8]) -> Vec<u8> {
    let mut ham = Vec::with_capacity(gri.len() + boy as usize);
    for satir in gri.chunks(en as usize) {
        ham.push(0u8); // süzgeç None
        ham.extend_from_slice(satir);
    }
    let z = miniz_oxide::deflate::compress_to_vec_zlib(&ham, 6);
    let mut ihdr = en.to_be_bytes().to_vec();
    ihdr.extend_from_slice(&boy.to_be_bytes());
    ihdr.extend_from_slice(&[8, 0, 0, 0, 0]);
    let mut b = PNG_IMZASI.to_vec();
    b.extend(parca(b"IHDR", &ihdr));
    b.extend(parca(b"IDAT", &z));
    b.extend(parca(b"IEND", &[]));
    b
}

/// İmza sayılır mı: PNG çözülür ve içerik kuralı (modül başı) tutar. Hata: kısa Türkçe açıklama.
#[cfg(test)]
pub(crate) fn imza_yeterli(b: &[u8]) -> Result<Ozet, String> {
    imza_denetle_ic(b).map(|(o, _)| o)
}

/// İmzayı denetler ve SAKLANACAK biçimde yeniden kodlar (modül başı): görünen pikseller 8 bit gri, yalnız
/// IHDR + IDAT + IEND; yardımcı parçalar ve IEND sonrası hiçbir şey taşınmaz. Saklanan dosyanın özeti
/// (`imza_sha256`) bu çıktının özetidir.
pub(crate) fn imza_temizle(b: &[u8]) -> Result<Vec<u8>, String> {
    let (o, gri) = imza_denetle_ic(b)?;
    Ok(gri_kodla(o.en, o.boy, &gri))
}

fn imza_denetle_ic(b: &[u8]) -> Result<(Ozet, Vec<u8>), String> {
    let (o, gri) = coz_ic(b)?;
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
    Ok((o, gri))
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

    /// IEND'den hemen önce bir parça ekler (yalnız testler).
    fn parca_ekle(png: &[u8], tur: &[u8], veri: &[u8]) -> Vec<u8> {
        let mut b = png[..png.len() - 12].to_vec();
        b.extend(parca(tur, veri));
        b.extend_from_slice(&png[png.len() - 12..]);
        b
    }

    /// IHDR'den hemen sonra (IDAT'tan önce) bir parça ekler (yalnız testler).
    fn ihdr_sonrasi_ekle(png: &[u8], tur: &[u8], veri: &[u8]) -> Vec<u8> {
        let mut b = png[..8 + 25].to_vec();
        b.extend(parca(tur, veri));
        b.extend_from_slice(&png[8 + 25..]);
        b
    }

    #[test]
    fn iend_verisi_ve_sonrasi_reddedilir() {
        let iyi = deneme_png(200, 80, 0, true);
        // IEND verisi boş değil (CRC doğru olsa da)
        let mut b = iyi[..iyi.len() - 12].to_vec();
        b.extend(parca(b"IEND", b"x"));
        assert!(
            coz(&b).unwrap_err().contains("IEND verisi"),
            "{:?}",
            coz(&b)
        );
        // IEND'den sonra tam (CRC'li) bir parça daha
        let mut c = iyi.clone();
        c.extend(parca(b"tEXt", b"Gizli\0veri"));
        assert!(coz(&c).unwrap_err().contains("IEND'den sonra"));
        // IEND'den sonra tek bayt
        let mut e = iyi.clone();
        e.push(0);
        assert!(coz(&e).unwrap_err().contains("IEND'den sonra"));
        assert!(imza_temizle(&e).is_err());
    }

    /// Bulutla ORTAK tRNS vektörleri (bulut/musteri-takip/test/ortak-vektorler.json; dene8.mjs aynı dosyayı JS
    /// `imzaIcerik` ile sınar): iki tarafın kuralı aynı sonucu vermeli ("tamam" | "bos" | "gecersiz").
    #[test]
    fn trns_ortak_vektorler_bulutla_ayni() {
        let v: serde_json::Value =
            serde_json::from_str(crate::teslim_onay::testler::ORTAK_VEKTORLER).unwrap();
        let liste = v["png"].as_array().unwrap();
        assert!(liste.len() >= 6);
        for p in liste {
            let ad = p["ad"].as_str().unwrap();
            let b = crate::teslim_sifre::base64_coz(p["b64"].as_str().unwrap()).unwrap();
            let sonuc = match coz(&b) {
                Err(_) => "gecersiz",
                Ok(_) => match imza_yeterli(&b) {
                    Ok(_) => "tamam",
                    Err(_) => "bos",
                },
            };
            assert_eq!(sonuc, p["beklenen"].as_str().unwrap(), "{ad}");
        }
    }

    #[test]
    fn trns_saydam_murekkep_bos_sayilir() {
        // Gri imza: mürekkep 20, zemin 255. tRNS saydam rengi 20 ise mürekkep görünmez: imza boş.
        let png = deneme_png(600, 240, 0, true);
        assert!(imza_yeterli(&png).is_ok());
        let saydam = ihdr_sonrasi_ekle(&png, b"tRNS", &20u16.to_be_bytes());
        assert_eq!(coz(&saydam).unwrap().koyu, 0);
        assert!(imza_yeterli(&saydam).unwrap_err().contains("boş"));
        assert!(imza_temizle(&saydam).is_err());
        // Saydam renk zemin (255) ise mürekkep görünür kalır: geçer, özet aynı.
        let zemin = ihdr_sonrasi_ekle(&png, b"tRNS", &255u16.to_be_bytes());
        assert_eq!(coz(&zemin).unwrap(), coz(&png).unwrap());
        // 8 bit dışında kalan saydam değer (300) hiçbir pikselle eşleşmez.
        let disari = ihdr_sonrasi_ekle(&png, b"tRNS", &300u16.to_be_bytes());
        assert_eq!(coz(&disari).unwrap(), coz(&png).unwrap());
        // RGBA'da tRNS, yanlış boy, tekrar ve IDAT sonrası tRNS geçersiz
        let rgba = deneme_png(600, 240, 6, true);
        assert!(coz(&ihdr_sonrasi_ekle(&rgba, b"tRNS", &[0; 2])).is_err());
        assert!(coz(&ihdr_sonrasi_ekle(&png, b"tRNS", &[0; 6])).is_err());
        let iki = ihdr_sonrasi_ekle(&saydam, b"tRNS", &20u16.to_be_bytes());
        assert!(coz(&iki).unwrap_err().contains("tRNS"));
        assert!(coz(&parca_ekle(&png, b"tRNS", &20u16.to_be_bytes()))
            .unwrap_err()
            .contains("tRNS"));
    }

    #[test]
    fn kabul_edilen_imza_yeniden_kodlanir_ek_parcalar_atilir() {
        for renk in [0, 6] {
            let png = deneme_png(600, 240, renk, true);
            let ekli = parca_ekle(
                &parca_ekle(&png, b"tEXt", b"Comment\0GIZLI-VERI-DENEME"),
                b"eXIf",
                b"GIZLI-EXIF",
            );
            assert!(
                imza_yeterli(&ekli).is_ok(),
                "yardımcı parçalar kabulü engellemez"
            );
            let temiz = imza_temizle(&ekli).unwrap();
            for gizli in [&b"GIZLI-VERI"[..], b"GIZLI-EXIF", b"tEXt", b"eXIf"] {
                assert!(!temiz.windows(gizli.len()).any(|w| w == gizli));
            }
            // yalnız IHDR + IDAT + IEND, 8 bit gri
            let mut turler = Vec::new();
            let mut i = 8;
            while i < temiz.len() {
                let uz = u32_be(&temiz[i..]) as usize;
                turler.push(temiz[i + 4..i + 8].to_vec());
                i += 12 + uz;
            }
            assert_eq!(
                turler,
                [b"IHDR".to_vec(), b"IDAT".to_vec(), b"IEND".to_vec()]
            );
            assert_eq!(temiz[8 + 8 + 9], 0, "renk türü gri");
            // görünen içerik aynı, yeniden kodlama kararlı (ikinci geçiş aynı bayt)
            assert_eq!(coz(&temiz).unwrap(), coz(&png).unwrap());
            assert_eq!(imza_temizle(&temiz).unwrap(), temiz);
        }
        // alfa beyaza karıştırılmış olarak saklanır: tam saydam kara piksel beyaz olur
        let saydam_kara = {
            let mut ham = Vec::new();
            for _ in 0..4 {
                ham.push(0u8);
                ham.extend_from_slice(&[0, 0, 0, 0].repeat(4));
            }
            let z = miniz_oxide::deflate::compress_to_vec_zlib(&ham, 6);
            let mut ihdr = 4u32.to_be_bytes().to_vec();
            ihdr.extend_from_slice(&4u32.to_be_bytes());
            ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
            let mut b = PNG_IMZASI.to_vec();
            b.extend(parca(b"IHDR", &ihdr));
            b.extend(parca(b"IDAT", &z));
            b.extend(parca(b"IEND", &[]));
            b
        };
        let (o, gri) = coz_ic(&saydam_kara).unwrap();
        assert_eq!(o.koyu, 0);
        assert!(gri.iter().all(|&g| g == 255));
    }
}
