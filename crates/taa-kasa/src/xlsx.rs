// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! En küçük xlsx yazıcısı: yeni dış bağımlılık yok. Zip "stored" (sıkıştırmasız) olarak elle yazılır,
//! hücreler satır içi metin (`inlineStr`) ya da sayıdır. Excel, LibreOffice ve Google E-Tablolar açar.
//!
//! Para sütunları kuruş olarak verilir ve `#,##0.00` biçimiyle TL gösterilir (sayı olarak kalır, toplanabilir).

/// Bir hücre.
#[derive(Debug, Clone, PartialEq)]
pub enum Hucre {
    Metin(String),
    /// Kuruş; Excel'de TL olarak (`/100`) ve iki ondalıkla görünür.
    Kurus(i64),
    Sayi(i64),
    Bos,
}

impl From<&str> for Hucre {
    fn from(s: &str) -> Self {
        Hucre::Metin(s.to_string())
    }
}
impl From<String> for Hucre {
    fn from(s: String) -> Self {
        Hucre::Metin(s)
    }
}

/// Bir sayfa: ad (en çok 31 karakter, `[]:*?/\` olmadan) ve satırlar. İlk satır başlık sayılır (kalın).
#[derive(Debug, Clone, Default)]
pub struct Sayfa {
    pub ad: String,
    pub satirlar: Vec<Vec<Hucre>>,
}

fn xml_kacis(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            '\'' => o.push_str("&apos;"),
            // XML 1.0'da geçersiz denetim karakterleri atılır (sekme ve satır sonu kalır).
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {}
            c => o.push(c),
        }
    }
    o
}

fn sutun_adi(mut i: usize) -> String {
    let mut s = Vec::new();
    loop {
        s.push(b'A' + (i % 26) as u8);
        if i < 26 {
            break;
        }
        i = i / 26 - 1;
    }
    s.reverse();
    String::from_utf8(s).unwrap_or_default()
}

fn sayfa_adi_temiz(ad: &str, sira: usize) -> String {
    let t: String = ad
        .chars()
        .filter(|c| !matches!(c, '[' | ']' | ':' | '*' | '?' | '/' | '\\'))
        .take(31)
        .collect();
    if t.trim().is_empty() {
        format!("Sayfa{}", sira + 1)
    } else {
        t
    }
}

fn sayfa_xml(s: &Sayfa) -> String {
    let mut x = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
    );
    let en_genis = s.satirlar.iter().map(Vec::len).max().unwrap_or(0);
    if en_genis > 0 {
        x.push_str("<cols>");
        for i in 0..en_genis {
            x.push_str(&format!(
                "<col min=\"{n}\" max=\"{n}\" width=\"18\" customWidth=\"1\"/>",
                n = i + 1
            ));
        }
        x.push_str("</cols>");
    }
    x.push_str("<sheetData>");
    for (r, satir) in s.satirlar.iter().enumerate() {
        x.push_str(&format!("<row r=\"{}\">", r + 1));
        for (c, h) in satir.iter().enumerate() {
            let ref_ = format!("{}{}", sutun_adi(c), r + 1);
            let baslik = r == 0;
            match h {
                Hucre::Bos => {}
                Hucre::Metin(m) => x.push_str(&format!(
                    "<c r=\"{ref_}\" t=\"inlineStr\"{}><is><t xml:space=\"preserve\">{}</t></is></c>",
                    if baslik { " s=\"1\"" } else { "" },
                    xml_kacis(m)
                )),
                Hucre::Sayi(n) => x.push_str(&format!("<c r=\"{ref_}\"><v>{n}</v></c>")),
                Hucre::Kurus(k) => {
                    let isaret = if *k < 0 { "-" } else { "" };
                    let m = k.unsigned_abs();
                    x.push_str(&format!(
                        "<c r=\"{ref_}\" s=\"2\"><v>{isaret}{}.{:02}</v></c>",
                        m / 100,
                        m % 100
                    ))
                }
            }
        }
        x.push_str("</row>");
    }
    x.push_str("</sheetData></worksheet>");
    x
}

const STILLER: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><numFmts count=\"1\"><numFmt numFmtId=\"164\" formatCode=\"#,##0.00\"/></numFmts><fonts count=\"2\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font><font><b/><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts><fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"3\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/><xf numFmtId=\"0\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/><xf numFmtId=\"164\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyNumberFormat=\"1\"/></cellXfs><cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles></styleSheet>";

/// Sayfalardan xlsx baytları üretir.
pub fn xlsx_uret(sayfalar: &[Sayfa]) -> Vec<u8> {
    let sayfalar: Vec<Sayfa> = if sayfalar.is_empty() {
        vec![Sayfa {
            ad: "Sayfa1".into(),
            satirlar: vec![],
        }]
    } else {
        sayfalar.to_vec()
    };
    let mut ct = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>");
    let mut wb = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets>");
    let mut wbr = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">");
    let mut kullanilan: Vec<String> = Vec::new();
    for (i, s) in sayfalar.iter().enumerate() {
        let n = i + 1;
        let mut ad = sayfa_adi_temiz(&s.ad, i);
        // Excel aynı adlı iki sayfayı (büyük/küçük harf farksız) kabul etmez.
        while kullanilan
            .iter()
            .any(|k| k.to_lowercase() == ad.to_lowercase())
        {
            ad = format!("{} {n}", ad.chars().take(27).collect::<String>());
        }
        kullanilan.push(ad.clone());
        ct.push_str(&format!("<Override PartName=\"/xl/worksheets/sheet{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"));
        wb.push_str(&format!(
            "<sheet name=\"{}\" sheetId=\"{n}\" r:id=\"rId{n}\"/>",
            xml_kacis(&ad)
        ));
        wbr.push_str(&format!("<Relationship Id=\"rId{n}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{n}.xml\"/>"));
    }
    let stil_id = sayfalar.len() + 1;
    wbr.push_str(&format!("<Relationship Id=\"rId{stil_id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>"));
    ct.push_str("</Types>");
    wb.push_str("</sheets></workbook>");
    let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>";

    let mut z = ZipYazici::default();
    z.ekle("[Content_Types].xml", ct.as_bytes());
    z.ekle("_rels/.rels", rels.as_bytes());
    z.ekle("xl/workbook.xml", wb.as_bytes());
    z.ekle("xl/_rels/workbook.xml.rels", wbr.as_bytes());
    z.ekle("xl/styles.xml", STILLER.as_bytes());
    for (i, s) in sayfalar.iter().enumerate() {
        z.ekle(
            &format!("xl/worksheets/sheet{}.xml", i + 1),
            sayfa_xml(s).as_bytes(),
        );
    }
    z.bitir()
}

// ---------------------------------------------------------------- stored zip

/// CRC-32 (IEEE 802.3, zip'in kullandığı).
pub fn crc32(veri: &[u8]) -> u32 {
    let mut tablo = [0u32; 256];
    for (i, t) in tablo.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *t = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for b in veri {
        c = tablo[((c ^ *b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

#[derive(Default)]
struct ZipYazici {
    govde: Vec<u8>,
    merkez: Vec<u8>,
    sayi: u16,
}

impl ZipYazici {
    fn ekle(&mut self, ad: &str, veri: &[u8]) {
        let crc = crc32(veri);
        let boy = veri.len() as u32;
        let konum = self.govde.len() as u32;
        let ad_b = ad.as_bytes();
        // Sabit tarih: 1980-01-01 00:00 (DOS biçimi); çıktı belirlenimci olur.
        let (saat, tarih) = (0u16, 0x0021u16);
        let b = &mut self.govde;
        b.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
        b.extend_from_slice(&20u16.to_le_bytes()); // sürüm
        b.extend_from_slice(&0x0800u16.to_le_bytes()); // bayrak: UTF-8 ad
        b.extend_from_slice(&0u16.to_le_bytes()); // stored
        b.extend_from_slice(&saat.to_le_bytes());
        b.extend_from_slice(&tarih.to_le_bytes());
        b.extend_from_slice(&crc.to_le_bytes());
        b.extend_from_slice(&boy.to_le_bytes());
        b.extend_from_slice(&boy.to_le_bytes());
        b.extend_from_slice(&(ad_b.len() as u16).to_le_bytes());
        b.extend_from_slice(&0u16.to_le_bytes());
        b.extend_from_slice(ad_b);
        b.extend_from_slice(veri);

        let m = &mut self.merkez;
        m.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
        m.extend_from_slice(&20u16.to_le_bytes()); // yapan sürüm
        m.extend_from_slice(&20u16.to_le_bytes()); // gereken sürüm
        m.extend_from_slice(&0x0800u16.to_le_bytes());
        m.extend_from_slice(&0u16.to_le_bytes());
        m.extend_from_slice(&saat.to_le_bytes());
        m.extend_from_slice(&tarih.to_le_bytes());
        m.extend_from_slice(&crc.to_le_bytes());
        m.extend_from_slice(&boy.to_le_bytes());
        m.extend_from_slice(&boy.to_le_bytes());
        m.extend_from_slice(&(ad_b.len() as u16).to_le_bytes());
        m.extend_from_slice(&0u16.to_le_bytes()); // ek alan
        m.extend_from_slice(&0u16.to_le_bytes()); // yorum
        m.extend_from_slice(&0u16.to_le_bytes()); // disk
        m.extend_from_slice(&0u16.to_le_bytes()); // iç öznitelik
        m.extend_from_slice(&0u32.to_le_bytes()); // dış öznitelik
        m.extend_from_slice(&konum.to_le_bytes());
        m.extend_from_slice(ad_b);
        self.sayi += 1;
    }

    fn bitir(mut self) -> Vec<u8> {
        let merkez_konum = self.govde.len() as u32;
        let merkez_boy = self.merkez.len() as u32;
        let mut o = std::mem::take(&mut self.govde);
        o.extend_from_slice(&self.merkez);
        o.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        o.extend_from_slice(&0u16.to_le_bytes());
        o.extend_from_slice(&0u16.to_le_bytes());
        o.extend_from_slice(&self.sayi.to_le_bytes());
        o.extend_from_slice(&self.sayi.to_le_bytes());
        o.extend_from_slice(&merkez_boy.to_le_bytes());
        o.extend_from_slice(&merkez_konum.to_le_bytes());
        o.extend_from_slice(&0u16.to_le_bytes());
        o
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn crc_bilinen_deger() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn sutun_adlari() {
        assert_eq!(sutun_adi(0), "A");
        assert_eq!(sutun_adi(25), "Z");
        assert_eq!(sutun_adi(26), "AA");
        assert_eq!(sutun_adi(27), "AB");
        assert_eq!(sutun_adi(701), "ZZ");
        assert_eq!(sutun_adi(702), "AAA");
    }

    #[test]
    fn xlsx_zip_yapisi_ve_icerik() {
        let s = Sayfa {
            ad: "Gün sonu".into(),
            satirlar: vec![
                vec!["Tarih".into(), "Nakit".into()],
                vec!["2026-11-26".into(), Hucre::Kurus(-12345)],
                vec![Hucre::Bos, Hucre::Sayi(3), "a<b & \"c\"".into()],
            ],
        };
        let b = xlsx_uret(&[s.clone(), s]);
        assert_eq!(&b[..4], b"PK\x03\x04");
        // Son kayıt: merkez dizin sonu, 7 dosya.
        let son = &b[b.len() - 22..];
        assert_eq!(&son[..4], &0x0605_4b50u32.to_le_bytes());
        assert_eq!(u16::from_le_bytes([son[10], son[11]]), 7);
        let metin = String::from_utf8_lossy(&b);
        assert!(metin.contains("<v>-123.45</v>"));
        assert!(metin.contains("a&lt;b &amp; &quot;c&quot;"));
        assert!(metin.contains("name=\"Gün sonu\""));
        assert!(
            metin.contains("name=\"Gün sonu 2\""),
            "yinelenen sayfa adı ayrışmalı"
        );
        // Her yerel başlığın CRC'si içerikle tutmalı.
        let mut i = 0;
        let mut adet = 0;
        while i + 30 <= b.len() && b[i..i + 4] == 0x0403_4b50u32.to_le_bytes() {
            let crc = u32::from_le_bytes(b[i + 14..i + 18].try_into().unwrap());
            let boy = u32::from_le_bytes(b[i + 18..i + 22].try_into().unwrap()) as usize;
            let ad = u16::from_le_bytes([b[i + 26], b[i + 27]]) as usize;
            let veri = &b[i + 30 + ad..i + 30 + ad + boy];
            assert_eq!(crc32(veri), crc);
            i += 30 + ad + boy;
            adet += 1;
        }
        assert_eq!(adet, 7);
    }
}
