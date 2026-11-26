// Sahip: Yusuf. yazan: [Claude#taa-cati-raporlar 2026-11-26]
//! Bağımlılıksız minimal .xlsx yazıcı (Office Open XML, sıkıştırmasız ZIP). Yalnız metin hücreleri (inlineStr;
//! formül yok, bu yüzden "=..." başlayan metin formül olarak çalışmaz). Excel, LibreOffice ve Numbers açar.

/// Çalışma sayfası: ad + satırlar (ilk satır başlık kalın yazılır).
pub struct Sayfa {
    pub ad: String,
    pub satirlar: Vec<Vec<String>>,
}

fn crc32(b: &[u8]) -> u32 {
    let mut t = [0u32; 256];
    for (i, e) in t.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *e = c;
    }
    let mut c = 0xFFFF_FFFFu32;
    for &x in b {
        c = t[((c ^ x as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

fn xml_kac(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => o.push_str("&amp;"),
            '<' => o.push_str("&lt;"),
            '>' => o.push_str("&gt;"),
            '"' => o.push_str("&quot;"),
            // XML 1.0'da geçersiz denetim karakterleri atılır (sekme/satır sonu kalır).
            c if ((c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r'))
                || matches!(c as u32, 0xFFFE | 0xFFFF) => {}
            c => o.push(c),
        }
    }
    o
}

fn sutun(mut n: usize) -> String {
    let mut s = String::new();
    loop {
        s.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    s
}

/// Sayfa adı: Excel kuralı (en çok 31 karakter, `[]:*?/\` yok).
fn sayfa_adi(a: &str) -> String {
    let t: String = a
        .chars()
        .filter(|c| !matches!(c, '[' | ']' | ':' | '*' | '?' | '/' | '\\'))
        .take(31)
        .collect();
    if t.trim().is_empty() {
        "Sayfa".into()
    } else {
        t
    }
}

fn sayfa_xml(s: &Sayfa) -> String {
    let mut x = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><cols><col min=\"1\" max=\"5\" width=\"28\" customWidth=\"1\"/></cols><sheetData>",
    );
    for (r, satir) in s.satirlar.iter().enumerate() {
        x.push_str(&format!("<row r=\"{}\">", r + 1));
        for (c, h) in satir.iter().enumerate() {
            let stil = if r == 0 { " s=\"1\"" } else { "" };
            x.push_str(&format!(
                "<c r=\"{}{}\" t=\"inlineStr\"{stil}><is><t xml:space=\"preserve\">{}</t></is></c>",
                sutun(c),
                r + 1,
                xml_kac(h)
            ));
        }
        x.push_str("</row>");
    }
    x.push_str("</sheetData></worksheet>");
    x
}

fn zip(dosyalar: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut o: Vec<u8> = vec![];
    let mut merkez: Vec<u8> = vec![];
    let u16b = |v: &mut Vec<u8>, x: u16| v.extend_from_slice(&x.to_le_bytes());
    let u32b = |v: &mut Vec<u8>, x: u32| v.extend_from_slice(&x.to_le_bytes());
    for (ad, veri) in dosyalar {
        let ofset = o.len() as u32;
        let crc = crc32(veri);
        let adb = ad.as_bytes();
        // yerel başlık
        u32b(&mut o, 0x0403_4B50);
        u16b(&mut o, 20);
        u16b(&mut o, 0x0800); // UTF-8 ad
        u16b(&mut o, 0); // sıkıştırmasız
        u16b(&mut o, 0);
        u16b(&mut o, 0x21); // tarih 1980-01-01
        u32b(&mut o, crc);
        u32b(&mut o, veri.len() as u32);
        u32b(&mut o, veri.len() as u32);
        u16b(&mut o, adb.len() as u16);
        u16b(&mut o, 0);
        o.extend_from_slice(adb);
        o.extend_from_slice(veri);
        // merkez dizin kaydı
        u32b(&mut merkez, 0x0201_4B50);
        u16b(&mut merkez, 20);
        u16b(&mut merkez, 20);
        u16b(&mut merkez, 0x0800);
        u16b(&mut merkez, 0);
        u16b(&mut merkez, 0);
        u16b(&mut merkez, 0x21);
        u32b(&mut merkez, crc);
        u32b(&mut merkez, veri.len() as u32);
        u32b(&mut merkez, veri.len() as u32);
        u16b(&mut merkez, adb.len() as u16);
        u16b(&mut merkez, 0);
        u16b(&mut merkez, 0);
        u16b(&mut merkez, 0);
        u16b(&mut merkez, 0);
        u32b(&mut merkez, 0);
        u32b(&mut merkez, ofset);
        merkez.extend_from_slice(adb);
    }
    let basla = o.len() as u32;
    let boy = merkez.len() as u32;
    o.extend_from_slice(&merkez);
    u32b(&mut o, 0x0605_4B50);
    u16b(&mut o, 0);
    u16b(&mut o, 0);
    u16b(&mut o, dosyalar.len() as u16);
    u16b(&mut o, dosyalar.len() as u16);
    u32b(&mut o, boy);
    u32b(&mut o, basla);
    u16b(&mut o, 0);
    o
}

/// En az bir sayfa verilmelidir (boşsa boş bir "Rapor" sayfası konur).
pub fn xlsx_yaz(sayfalar: &[Sayfa]) -> Vec<u8> {
    let bos = [Sayfa {
        ad: "Rapor".into(),
        satirlar: vec![],
    }];
    let sayfalar = if sayfalar.is_empty() {
        &bos[..]
    } else {
        sayfalar
    };
    let mut tur = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Types xmlns=\"http://schemas.openxmlformats.org/package/2006/content-types\"><Default Extension=\"rels\" ContentType=\"application/vnd.openxmlformats-package.relationships+xml\"/><Default Extension=\"xml\" ContentType=\"application/xml\"/><Override PartName=\"/xl/workbook.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml\"/><Override PartName=\"/xl/styles.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.styles+xml\"/>",
    );
    let mut wb = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<workbook xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"><sheets>",
    );
    let mut rel = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\">",
    );
    let mut dosyalar: Vec<(String, Vec<u8>)> = vec![];
    let mut kullanilan: Vec<String> = vec![];
    for (i, s) in sayfalar.iter().enumerate() {
        let mut ad = sayfa_adi(&s.ad);
        let taban = ad.clone();
        let mut sira = 2usize;
        while kullanilan.iter().any(|k| k.eq_ignore_ascii_case(&ad)) {
            let sonek = format!("~{sira}");
            let uzun = 31 - sonek.chars().count();
            ad = format!("{}{sonek}", taban.chars().take(uzun).collect::<String>());
            sira += 1;
        }
        kullanilan.push(ad.clone());
        let n = i + 1;
        tur.push_str(&format!("<Override PartName=\"/xl/worksheets/sheet{n}.xml\" ContentType=\"application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml\"/>"));
        wb.push_str(&format!(
            "<sheet name=\"{}\" sheetId=\"{n}\" r:id=\"rId{n}\"/>",
            xml_kac(&ad)
        ));
        rel.push_str(&format!("<Relationship Id=\"rId{n}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet{n}.xml\"/>"));
        dosyalar.push((
            format!("xl/worksheets/sheet{n}.xml"),
            sayfa_xml(s).into_bytes(),
        ));
    }
    let st = sayfalar.len() + 1;
    rel.push_str(&format!("<Relationship Id=\"rId{st}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/></Relationships>"));
    tur.push_str("</Types>");
    wb.push_str("</sheets></workbook>");
    let kok_rel = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>";
    let stiller = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\"><fonts count=\"2\"><font><sz val=\"11\"/><name val=\"Calibri\"/></font><font><b/><sz val=\"11\"/><name val=\"Calibri\"/></font></fonts><fills count=\"2\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill></fills><borders count=\"1\"><border><left/><right/><top/><bottom/><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs><cellXfs count=\"2\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\" xfId=\"0\"/><xf numFmtId=\"0\" fontId=\"1\" fillId=\"0\" borderId=\"0\" xfId=\"0\" applyFont=\"1\"/></cellXfs><cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles></styleSheet>";
    let mut hepsi: Vec<(String, Vec<u8>)> = vec![
        ("[Content_Types].xml".into(), tur.into_bytes()),
        ("_rels/.rels".into(), kok_rel.as_bytes().to_vec()),
        ("xl/workbook.xml".into(), wb.into_bytes()),
        ("xl/_rels/workbook.xml.rels".into(), rel.into_bytes()),
        ("xl/styles.xml".into(), stiller.as_bytes().to_vec()),
    ];
    hepsi.extend(dosyalar);
    zip(&hepsi)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn xml_gecersiz_unicode_atilir() {
        assert_eq!(xml_kac("a\u{FFFE}b\u{1}c"), "abc");
    }

    #[test]
    fn crc32_bilinen_deger() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn sutun_adlari() {
        assert_eq!(sutun(0), "A");
        assert_eq!(sutun(25), "Z");
        assert_eq!(sutun(26), "AA");
        assert_eq!(sutun(27), "AB");
    }

    #[test]
    fn xlsx_zip_imzali_ve_kacisli() {
        let b = xlsx_yaz(&[Sayfa {
            ad: "Özet/ [x]".into(),
            satirlar: vec![
                vec!["a".into(), "b".into()],
                vec!["=1+1".into(), "<&\u{1}>".into()],
            ],
        }]);
        assert_eq!(&b[..4], b"PK\x03\x04");
        let m = String::from_utf8_lossy(&b);
        assert!(m.contains("&lt;&amp;&gt;"));
        assert!(m.contains("name=\"Özet x\""));
        let x = sayfa_xml(&Sayfa {
            ad: "a".into(),
            satirlar: vec![vec!["<&\u{1}>".into()]],
        });
        assert!(!x.contains('\u{1}'));
        // Boş girdi de geçerli dosya üretir.
        assert_eq!(&xlsx_yaz(&[])[..2], b"PK");
    }
}
