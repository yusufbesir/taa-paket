//! Metin katmanı çıkarma: docx (metin + tablolar) ve PDF (sayfa metni + taranmış sayfadaki gömülü JPEG'ler).

use lopdf::{Document, Object};
use std::io::Read;

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Docx {
    /// Gövde metni, satır satır (tablo satırları hücreler sekmeyle birleşik).
    pub satirlar: Vec<String>,
    /// Üst düzey tablolar: tablo -> satır -> hücre metni.
    pub tablolar: Vec<Vec<Vec<String>>>,
}

fn varlik_coz(s: &str) -> String {
    let mut c = String::with_capacity(s.len());
    let mut r = s;
    while let Some(i) = r.find('&') {
        c.push_str(&r[..i]);
        r = &r[i..];
        let Some(j) = r.find(';') else { break };
        let v = &r[1..j];
        match v {
            "amp" => c.push('&'),
            "lt" => c.push('<'),
            "gt" => c.push('>'),
            "quot" => c.push('"'),
            "apos" => c.push('\''),
            _ => {
                let n = v
                    .strip_prefix("#x")
                    .and_then(|h| u32::from_str_radix(h, 16).ok())
                    .or_else(|| v.strip_prefix('#').and_then(|d| d.parse().ok()));
                match n.and_then(char::from_u32) {
                    Some(ch) => c.push(ch),
                    None => c.push_str(&r[..=j]),
                }
            }
        }
        r = &r[j + 1..];
    }
    c.push_str(r);
    c
}

/// `word/document.xml` içeriğinden metin ve üst düzey tabloları çıkarır.
pub fn docx_xml(xml: &str) -> Docx {
    let mut d = Docx::default();
    let mut tablo_derinlik = 0usize;
    let mut tablo: Vec<Vec<String>> = Vec::new();
    let mut satir: Vec<String> = Vec::new();
    let mut hucre = String::new();
    let mut par = String::new();
    let mut r = xml;
    while let Some(i) = r.find('<') {
        let Some(j) = r[i..].find('>') else { break };
        let etiket = &r[i + 1..i + j];
        let sonra = &r[i + j + 1..];
        let ad = etiket
            .trim_end_matches('/')
            .split_whitespace()
            .next()
            .unwrap_or("");
        match ad {
            "w:t" if !etiket.ends_with('/') => {
                if let Some(k) = sonra.find("</w:t>") {
                    par.push_str(&varlik_coz(&sonra[..k]));
                }
            }
            "w:tab" => par.push('\t'),
            "w:br" | "w:cr" => par.push(' '),
            "/w:p" => {
                if tablo_derinlik == 0 {
                    if !par.trim().is_empty() {
                        d.satirlar.push(par.trim().to_string());
                    }
                } else {
                    if !hucre.is_empty() && !par.trim().is_empty() {
                        hucre.push(' ');
                    }
                    hucre.push_str(par.trim());
                }
                par.clear();
            }
            "w:tbl" => {
                tablo_derinlik += 1;
                if tablo_derinlik == 1 {
                    tablo.clear();
                }
            }
            "w:tr" if tablo_derinlik == 1 => satir.clear(),
            "/w:tc" if tablo_derinlik == 1 => {
                satir.push(std::mem::take(&mut hucre));
            }
            "/w:tr" if tablo_derinlik == 1 => {
                d.satirlar.push(satir.join("\t"));
                tablo.push(std::mem::take(&mut satir));
            }
            "/w:tbl" => {
                if tablo_derinlik == 1 {
                    d.tablolar.push(std::mem::take(&mut tablo));
                }
                tablo_derinlik = tablo_derinlik.saturating_sub(1);
            }
            _ => {}
        }
        r = sonra;
    }
    d
}

pub fn docx_oku(b: &[u8]) -> Result<Docx, String> {
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(b)).map_err(|e| e.to_string())?;
    let mut f = z.by_name("word/document.xml").map_err(|e| e.to_string())?;
    if f.size() > 200 * 1024 * 1024 {
        return Err("docx içeriği çok büyük".into());
    }
    let mut s = String::new();
    f.read_to_string(&mut s).map_err(|e| e.to_string())?;
    Ok(docx_xml(&s))
}

#[derive(Debug, Default)]
pub struct PdfSayfa {
    pub no: u32,
    /// Metin katmanı (boşsa sayfa taranmış sayılır).
    pub metin: String,
    /// Sayfadaki gömülü JPEG görüntüler (DCTDecode); taranmış sayfa OCR'a bunlarla gider.
    pub jpegler: Vec<Vec<u8>>,
}

pub enum PdfHata {
    Korumali,
    Bozuk(String),
}

pub fn pdf_oku(b: &[u8]) -> Result<Vec<PdfSayfa>, PdfHata> {
    let doc = Document::load_mem(b).map_err(|e| PdfHata::Bozuk(e.to_string()))?;
    if doc.is_encrypted() {
        return Err(PdfHata::Korumali);
    }
    let mut cikti = Vec::new();
    for (no, id) in doc.get_pages() {
        let metin = doc.extract_text(&[no]).unwrap_or_default();
        let mut jpegler = Vec::new();
        if metin.trim().is_empty() {
            if let Ok((kaynak, _)) = doc.get_page_resources(id) {
                if let Some(Ok(xo)) = kaynak.and_then(|k| k.get(b"XObject").ok()).map(|o| {
                    doc.dereference(o)
                        .map(|(_, o)| o)
                        .and_then(|o| o.as_dict().cloned())
                }) {
                    for (_, nesne) in xo.iter() {
                        let Ok(Object::Stream(s)) = doc.dereference(nesne).map(|(_, o)| o.clone())
                        else {
                            continue;
                        };
                        let alt = s.dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok());
                        let suz = s.dict.get(b"Filter").ok().and_then(|o| o.as_name().ok());
                        if alt == Some(b"Image") && suz == Some(b"DCTDecode") {
                            jpegler.push(s.content.clone());
                        }
                    }
                }
            }
        }
        cikti.push(PdfSayfa { no, metin, jpegler });
    }
    Ok(cikti)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn docx_paragraf_ve_tablo() {
        let xml = r#"<w:document><w:body>
<w:p><w:r><w:t>Teslim Tutanağı</w:t></w:r></w:p>
<w:tbl><w:tr><w:tc><w:p><w:r><w:t>T14525</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>2</w:t></w:r></w:p></w:tc></w:tr>
<w:tr><w:tc><w:p><w:r><w:t>A &amp; B</w:t></w:r></w:p></w:tc><w:tc><w:p><w:r><w:t>1</w:t></w:r></w:p></w:tc></w:tr></w:tbl>
<w:p><w:r><w:t>Toplam</w:t></w:r></w:p></w:body></w:document>"#;
        let d = docx_xml(xml);
        assert_eq!(
            d.satirlar,
            vec!["Teslim Tutanağı", "T14525\t2", "A & B\t1", "Toplam"]
        );
        assert_eq!(d.tablolar.len(), 1);
        assert_eq!(d.tablolar[0][1], vec!["A & B", "1"]);
    }

    #[test]
    fn varlik_sayisal() {
        assert_eq!(varlik_coz("&#x130;&#304;&lt;"), "İİ<");
    }

    #[test]
    fn pdf_metin_katmani_okunur() {
        use lopdf::content::{Content, Operation};
        use lopdf::{dictionary, Stream};
        let mut doc = Document::with_version("1.5");
        let pages = doc.new_object_id();
        let font = doc.add_object(
            dictionary! {"Type" => "Font", "Subtype" => "Type1", "BaseFont" => "Helvetica"},
        );
        let res = doc.add_object(dictionary! {"Font" => dictionary! {"F1" => font}});
        let icerik = Content {
            operations: vec![
                Operation::new("BT", vec![]),
                Operation::new("Tf", vec!["F1".into(), 12.into()]),
                Operation::new("Td", vec![50.into(), 700.into()]),
                Operation::new("Tj", vec![Object::string_literal("T14525 adet 3")]),
                Operation::new("ET", vec![]),
            ],
        };
        let ic = doc.add_object(Stream::new(dictionary! {}, icerik.encode().unwrap()));
        let sayfa = doc.add_object(
            dictionary! {"Type" => "Page", "Parent" => pages, "Contents" => ic, "Resources" => res,
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()]},
        );
        doc.objects.insert(
            pages,
            Object::Dictionary(
                dictionary! {"Type" => "Pages", "Kids" => vec![sayfa.into()], "Count" => 1},
            ),
        );
        let kok = doc.add_object(dictionary! {"Type" => "Catalog", "Pages" => pages});
        doc.trailer.set("Root", kok);
        let mut b = Vec::new();
        doc.save_to(&mut b).unwrap();
        let s = pdf_oku(&b).ok().unwrap();
        assert_eq!(s.len(), 1);
        assert!(s[0].metin.contains("T14525 adet 3"), "{:?}", s[0].metin);
    }
}
