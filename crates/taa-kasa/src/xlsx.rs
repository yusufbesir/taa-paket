// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! En küçük xlsx yazıcısı: yeni dış bağımlılık yok. Zip "stored" (sıkıştırmasız) olarak elle yazılır,
//! hücreler satır içi metin (`inlineStr`) ya da sayıdır. Excel, LibreOffice ve Google E-Tablolar açar.
//!
//! Para sütunları kuruş olarak verilir ve `#,##0.00` biçimiyle TL gösterilir (sayı olarak kalır, toplanabilir).
//!
//! Akşam ciro şablonu için (merkez::ciro_sablonu) sabit bir stil tablosu (`Stil`), formül (önbellek değeriyle),
//! sütun genişliği, bölme dondurma, sayfa koruması ve yazdırma alanı da desteklenir.

/// Bir hücre.
#[derive(Debug, Clone, PartialEq)]
pub enum Hucre {
    Metin(String),
    /// Kuruş; Excel'de TL olarak (`/100`) ve iki ondalıkla görünür.
    Kurus(i64),
    Sayi(i64),
    Bos,
    /// Stil verilmiş hücre (boş da olabilir: yalnız çerçeve/dolgu).
    Stilli(Box<Hucre>, Stil),
    /// Formül (`=` olmadan) ve önbellek değeri (kuruş; TL olarak yazılır). Excel açılışta yeniden hesaplar.
    Formul(String, i64),
    /// Ondalıklı sayı: `deger / 10^ondalik` (ör. kur onbinde → 4).
    Ondalik(i64, u32),
}

/// Sabit stil tablosu (styles.xml `cellXfs` sırası). Renk ve biçimler akşam ciro şablonundan (V2) alındı:
/// sarı = elle girilen, gri = hesaplanan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stil {
    Normal = 0,
    Kalin = 1,
    Kurus = 2,
    Baslik = 3,
    TarihBaslik = 4,
    Etiket = 5,
    GirisTl = 6,
    GirisUsd = 7,
    GirisEur = 8,
    GirisGbp = 9,
    GirisChf = 10,
    HesapTl = 11,
    HesapUsd = 12,
    HesapEur = 13,
    HesapGbp = 14,
    HesapChf = 15,
    Bolum = 16,
    ToplamMasraf = 17,
    ToplamMasrafTutar = 18,
    GenelToplam = 19,
    GenelToplamTutar = 20,
    GenelToplam2 = 21,
    GenelToplam2Tutar = 22,
    MagazaToplam = 23,
    MagazaToplamTutar = 24,
    GirisMetin = 25,
    HesapBos = 26,
    GirisKur = 27,
    YardimBaslik = 28,
    Yardim = 29,
    YardimGri = 30,
    YardimBeyaz = 31,
    BolumOrta = 32,
    Cerceve = 33,
}

impl Hucre {
    /// Bu hücreye stil verir.
    pub fn stil(self, s: Stil) -> Hucre {
        Hucre::Stilli(Box::new(self), s)
    }
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
    /// Sütun genişlikleri (boşsa hepsi 18).
    pub genislikler: Vec<f64>,
    /// Dondurulan bölmenin sol üst hücresi (ör. `B2`).
    pub dondur: Option<String>,
    /// Sayfa koruması (parolasız; yalnız kilitsiz/sarı hücreler değişir).
    pub koru: bool,
    /// Yazdırma alanı (ör. `$A$1:$E$40`).
    pub yazdirma_alani: Option<String>,
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

fn kurus_metni(k: i64) -> String {
    let isaret = if k < 0 { "-" } else { "" };
    let m = k.unsigned_abs();
    format!("{isaret}{}.{:02}", m / 100, m % 100)
}

fn ondalik_metni(d: i64, o: u32) -> String {
    if o == 0 {
        return d.to_string();
    }
    let b = 10u64.pow(o);
    let isaret = if d < 0 { "-" } else { "" };
    let m = d.unsigned_abs();
    format!("{isaret}{}.{:0w$}", m / b, m % b, w = o as usize)
}

fn hucre_xml(x: &mut String, ref_: &str, h: &Hucre, stil: Option<u32>, baslik: bool) {
    let s = |varsayilan: Option<u32>| match stil.or(varsayilan) {
        Some(n) => format!(" s=\"{n}\""),
        None => String::new(),
    };
    match h {
        Hucre::Bos => {
            if stil.is_some() {
                x.push_str(&format!("<c r=\"{ref_}\"{}/>", s(None)));
            }
        }
        Hucre::Metin(m) => x.push_str(&format!(
            "<c r=\"{ref_}\" t=\"inlineStr\"{}><is><t xml:space=\"preserve\">{}</t></is></c>",
            s(baslik.then_some(1)),
            xml_kacis(m)
        )),
        Hucre::Sayi(n) => x.push_str(&format!("<c r=\"{ref_}\"{}><v>{n}</v></c>", s(None))),
        Hucre::Kurus(k) => x.push_str(&format!(
            "<c r=\"{ref_}\"{}><v>{}</v></c>",
            s(Some(2)),
            kurus_metni(*k)
        )),
        Hucre::Ondalik(d, o) => x.push_str(&format!(
            "<c r=\"{ref_}\"{}><v>{}</v></c>",
            s(None),
            ondalik_metni(*d, *o)
        )),
        Hucre::Formul(f, k) => x.push_str(&format!(
            "<c r=\"{ref_}\"{}><f>{}</f><v>{}</v></c>",
            s(Some(2)),
            xml_kacis(f),
            kurus_metni(*k)
        )),
        Hucre::Stilli(ic, st) => hucre_xml(x, ref_, ic, Some(*st as u32), baslik),
    }
}

fn sayfa_xml(s: &Sayfa) -> String {
    let mut x = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">",
    );
    if let Some(d) = &s.dondur {
        let (sut, sat) = hucre_konumu(d);
        let mut b = String::new();
        if sut > 0 {
            b.push_str(&format!(" xSplit=\"{sut}\""));
        }
        if sat > 0 {
            b.push_str(&format!(" ySplit=\"{sat}\""));
        }
        x.push_str(&format!(
            "<sheetViews><sheetView workbookViewId=\"0\"><pane{b} topLeftCell=\"{}\" activePane=\"bottomRight\" state=\"frozen\"/></sheetView></sheetViews>",
            xml_kacis(d)
        ));
    }
    let en_genis = s
        .satirlar
        .iter()
        .map(Vec::len)
        .max()
        .unwrap_or(0)
        .max(s.genislikler.len());
    if en_genis > 0 {
        x.push_str("<cols>");
        for i in 0..en_genis {
            let g = s.genislikler.get(i).copied().unwrap_or(18.0);
            x.push_str(&format!(
                "<col min=\"{n}\" max=\"{n}\" width=\"{g}\" customWidth=\"1\"/>",
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
            hucre_xml(&mut x, &ref_, h, None, r == 0);
        }
        x.push_str("</row>");
    }
    x.push_str("</sheetData>");
    if s.koru {
        x.push_str("<sheetProtection sheet=\"1\" objects=\"1\" scenarios=\"1\"/>");
    }
    x.push_str("</worksheet>");
    x
}

/// `B2` → (1, 1): dondurulacak sütun ve satır sayısı.
fn hucre_konumu(r: &str) -> (usize, usize) {
    let harf: String = r.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    let sat: usize = r[harf.len()..].parse().unwrap_or(1);
    let sut = harf
        .to_ascii_uppercase()
        .bytes()
        .fold(0usize, |a, b| a * 26 + (b - b'A' + 1) as usize);
    (sut.saturating_sub(1), sat.saturating_sub(1))
}

/// Sütun adı (0 → A); şablon formülleri kurmak için.
pub fn sutun(i: usize) -> String {
    sutun_adi(i)
}

fn stiller() -> String {
    // Sayı biçimleri şablondaki (V2) biçimlerin aynısıdır.
    let bicimler = [
        (164, "#,##0.00"),
        (165, "_-\"₺\"* #,##0.00_-;\\-\"₺\"* #,##0.00_-;_-\"₺\"* \"-\"??_-;_-@_-"),
        (166, "_-[$$-409]* #,##0.00_ ;_-[$$-409]* \\-#,##0.00\\ ;_-[$$-409]* \"-\"??_ ;_-@_ "),
        (167, "_-[$€-2]\\ * #,##0.00_-;\\-[$€-2]\\ * #,##0.00_-;_-[$€-2]\\ * \"-\"??_-;_-@_-"),
        (168, "_-[$£-809]* #,##0.00_-;\\-[$£-809]* #,##0.00_-;_-[$£-809]* \"-\"??_-;_-@_-"),
        (169, "_-* #,##0.00\\ [$CHF-100C]_-;\\-* #,##0.00\\ [$CHF-100C]_-;_-* \"-\"??\\ [$CHF-100C]_-;_-@_-"),
        (170, "#,##0.0000"),
    ];
    // (boyut, kalın, beyaz)
    let fontlar: [(u32, bool, bool); 11] = [
        (11, false, false),
        (11, true, false),
        (13, false, false),
        (13, true, false),
        (15, true, true),
        (20, true, true),
        (14, true, true),
        (14, true, false),
        (16, true, true),
        (12, true, true),
        (11, false, false),
    ];
    let dolgular = [
        "FFF2CC", "EDEDED", "1F3864", "C00000", "2F5597", "D9E2F3", "548235", "375623", "DDEBF7",
        "FFFFFF",
    ];
    // (biçim, font, dolgu, çerçeve, hiza, kilitsiz) — sıra `Stil` ile aynı. Dolgu 0 yok, 1 gray125 (zorunlu),
    // 2.. `dolgular` sırası.
    const SOL: u8 = 1;
    const ORTA: u8 = 2;
    const SAG: u8 = 3;
    let xf: [(u32, u32, u32, u32, u8, bool); 34] = [
        (0, 0, 0, 0, 0, false),
        (0, 1, 0, 0, 0, false),
        (164, 0, 0, 0, 0, false),
        (0, 4, 4, 1, SOL, false),
        (0, 5, 5, 1, ORTA, false),
        (0, 3, 0, 1, SOL, false),
        (165, 3, 2, 1, SAG, true),
        (166, 3, 2, 1, SAG, true),
        (167, 3, 2, 1, SAG, true),
        (168, 3, 2, 1, SAG, true),
        (169, 3, 2, 1, SAG, true),
        (165, 2, 3, 1, SAG, false),
        (166, 3, 3, 1, SAG, false),
        (167, 3, 3, 1, SAG, false),
        (168, 3, 3, 1, SAG, false),
        (169, 3, 3, 1, SAG, false),
        (0, 6, 6, 1, SOL, false),
        (0, 7, 7, 1, SOL, false),
        (165, 7, 7, 1, SAG, false),
        (0, 6, 8, 1, SOL, false),
        (165, 6, 8, 1, SAG, false),
        (0, 8, 9, 1, SOL, false),
        (165, 8, 9, 1, SAG, false),
        (0, 7, 10, 1, SOL, false),
        (165, 7, 10, 1, SAG, false),
        (0, 2, 2, 1, SOL, true),
        (0, 2, 3, 1, 0, false),
        (170, 3, 2, 1, SAG, true),
        (0, 9, 4, 0, 0, false),
        (0, 10, 2, 0, 0, false),
        (0, 10, 3, 0, 0, false),
        (0, 10, 11, 0, 0, false),
        (0, 6, 6, 1, ORTA, false),
        (0, 2, 0, 1, 0, false),
    ];
    let mut x = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<styleSheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">");
    x.push_str(&format!("<numFmts count=\"{}\">", bicimler.len()));
    for (id, k) in bicimler {
        x.push_str(&format!(
            "<numFmt numFmtId=\"{id}\" formatCode=\"{}\"/>",
            xml_kacis(k)
        ));
    }
    x.push_str(&format!("</numFmts><fonts count=\"{}\">", fontlar.len()));
    for (b, kalin, beyaz) in fontlar {
        x.push_str(&format!(
            "<font>{}<sz val=\"{b}\"/>{}<name val=\"Calibri\"/></font>",
            if kalin { "<b/>" } else { "" },
            if beyaz {
                "<color rgb=\"FFFFFFFF\"/>"
            } else {
                ""
            }
        ));
    }
    x.push_str(&format!(
        "</fonts><fills count=\"{}\"><fill><patternFill patternType=\"none\"/></fill><fill><patternFill patternType=\"gray125\"/></fill>",
        dolgular.len() + 2
    ));
    for d in dolgular {
        x.push_str(&format!("<fill><patternFill patternType=\"solid\"><fgColor rgb=\"FF{d}\"/><bgColor indexed=\"64\"/></patternFill></fill>"));
    }
    x.push_str("</fills><borders count=\"2\"><border><left/><right/><top/><bottom/><diagonal/></border><border><left style=\"thin\"><color auto=\"1\"/></left><right style=\"thin\"><color auto=\"1\"/></right><top style=\"thin\"><color auto=\"1\"/></top><bottom style=\"thin\"><color auto=\"1\"/></bottom><diagonal/></border></borders><cellStyleXfs count=\"1\"><xf numFmtId=\"0\" fontId=\"0\" fillId=\"0\" borderId=\"0\"/></cellStyleXfs>");
    x.push_str(&format!("<cellXfs count=\"{}\">", xf.len()));
    for (b, f, d, c, h, kilitsiz) in xf {
        let hiza = match h {
            SOL => "<alignment horizontal=\"left\" vertical=\"center\"/>",
            ORTA => "<alignment horizontal=\"center\" vertical=\"center\"/>",
            SAG => "<alignment horizontal=\"right\" vertical=\"center\"/>",
            _ => "",
        };
        let koruma = if kilitsiz {
            "<protection locked=\"0\"/>"
        } else {
            ""
        };
        let ic = format!("{hiza}{koruma}");
        x.push_str(&format!(
            "<xf numFmtId=\"{b}\" fontId=\"{f}\" fillId=\"{}\" borderId=\"{c}\" xfId=\"0\"{}{}{}{}{}",
            d,
            if b != 0 { " applyNumberFormat=\"1\"" } else { "" },
            if f != 0 { " applyFont=\"1\"" } else { "" },
            if d != 0 { " applyFill=\"1\"" } else { "" },
            if c != 0 { " applyBorder=\"1\"" } else { "" },
            if ic.is_empty() {
                "/>".to_string()
            } else {
                format!(
                    "{}{}>{ic}</xf>",
                    if h != 0 { " applyAlignment=\"1\"" } else { "" },
                    if kilitsiz { " applyProtection=\"1\"" } else { "" }
                )
            }
        ));
    }
    x.push_str("</cellXfs><cellStyles count=\"1\"><cellStyle name=\"Normal\" xfId=\"0\" builtinId=\"0\"/></cellStyles></styleSheet>");
    x
}

/// Sayfalardan xlsx baytları üretir.
pub fn xlsx_uret(sayfalar: &[Sayfa]) -> Vec<u8> {
    let sayfalar: Vec<Sayfa> = if sayfalar.is_empty() {
        vec![Sayfa {
            ad: "Sayfa1".into(),
            ..Default::default()
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
    wb.push_str("</sheets>");
    let alanlar: Vec<String> = sayfalar
        .iter()
        .enumerate()
        .filter_map(|(i, s)| {
            s.yazdirma_alani.as_ref().map(|a| {
                format!(
                    "<definedName name=\"_xlnm.Print_Area\" localSheetId=\"{i}\">'{}'!{}</definedName>",
                    xml_kacis(&kullanilan[i].replace('\'', "''")),
                    xml_kacis(a)
                )
            })
        })
        .collect();
    if !alanlar.is_empty() {
        wb.push_str(&format!(
            "<definedNames>{}</definedNames>",
            alanlar.concat()
        ));
    }
    // Formüller açılışta yeniden hesaplansın (önbellek değerleri programın hesabıdır).
    wb.push_str("<calcPr calcId=\"191029\" fullCalcOnLoad=\"1\"/></workbook>");
    let rels = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<Relationships xmlns=\"http://schemas.openxmlformats.org/package/2006/relationships\"><Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument\" Target=\"xl/workbook.xml\"/></Relationships>";

    let mut z = ZipYazici::default();
    z.ekle("[Content_Types].xml", ct.as_bytes());
    z.ekle("_rels/.rels", rels.as_bytes());
    z.ekle("xl/workbook.xml", wb.as_bytes());
    z.ekle("xl/_rels/workbook.xml.rels", wbr.as_bytes());
    z.ekle("xl/styles.xml", stiller().as_bytes());
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
            ..Default::default()
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
