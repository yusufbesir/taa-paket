//! taa-pdf: PDF sayfa seç/sırala/böl, birleştir, sıkıştır. Girdi dosyasına dokunmaz; çıktı `taa-dosya::atomik_yaz`
//! ile yeni dosyaya yazılır (üzerine yazmaz). Özellik anahtarı: "pdf-araclari" (varsayılan açık).
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use lopdf::{Dictionary, Document, Object, ObjectId};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const OZELLIK: &str = "pdf-araclari";

fn kapali() -> Result<(), String> {
    if taa_ayar::ozellik_acik(OZELLIK, true) {
        Ok(())
    } else {
        Err(taa_ayar::kapali_mesaji(OZELLIK))
    }
}

pub fn bellekten(b: &[u8]) -> Result<Document, String> {
    kapali()?;
    if taa_dosya::bicim_algila(b) != Some("pdf") {
        return Err("dosya PDF değil".into());
    }
    Document::load_mem(b).map_err(|e| format!("PDF okunamadı: {e}"))
}

pub fn sayfa_sayisi(d: &Document) -> usize {
    d.get_pages().len()
}

const MIRAS: [&[u8]; 4] = [b"Resources", b"MediaBox", b"CropBox", b"Rotate"];

/// Sayfa ağacından miras alınan özellikleri sayfanın kendisine yazar (düzleştirmeden önce zorunlu).
fn miras_yaz(doc: &mut Document) {
    let sayfalar: Vec<ObjectId> = doc.get_pages().into_values().collect();
    for id in sayfalar {
        let mut ek: Vec<(Vec<u8>, Object)> = Vec::new();
        let mut gecerli = doc.get_dictionary(id).ok().and_then(|d| d.get(b"Parent").ok()).and_then(|p| p.as_reference().ok());
        let mut kendi: Vec<Vec<u8>> = doc
            .get_dictionary(id)
            .map(|d| d.iter().map(|(k, _)| k.clone()).collect())
            .unwrap_or_default();
        let mut sayac = 0;
        while let Some(pid) = gecerli {
            sayac += 1;
            if sayac > 64 {
                break;
            }
            let Ok(pd) = doc.get_dictionary(pid) else { break };
            for k in MIRAS {
                if !kendi.iter().any(|x| x == k) {
                    if let Ok(v) = pd.get(k) {
                        ek.push((k.to_vec(), v.clone()));
                        kendi.push(k.to_vec());
                    }
                }
            }
            gecerli = pd.get(b"Parent").ok().and_then(|p| p.as_reference().ok());
        }
        if let Ok(Object::Dictionary(d)) = doc.get_object_mut(id) {
            for (k, v) in ek {
                d.set(k, v);
            }
        }
    }
}

fn kok_sayfalar_id(doc: &Document) -> Result<ObjectId, String> {
    doc.catalog()
        .map_err(|e| e.to_string())?
        .get(b"Pages")
        .and_then(|p| p.as_reference())
        .map_err(|_| "sayfa ağacı bulunamadı".to_string())
}

/// Yalnız verilen sayfaları (1'den başlar) verilen sırada bırakır. Tekrar eden sayfa kabul edilmez.
pub fn sec(doc: &mut Document, sayfalar: &[u32]) -> Result<(), String> {
    kapali()?;
    let tum = doc.get_pages();
    if sayfalar.is_empty() {
        return Err("en az bir sayfa seçilmeli".into());
    }
    let mut gorulen = std::collections::BTreeSet::new();
    let mut idler = Vec::new();
    for s in sayfalar {
        if !gorulen.insert(*s) {
            return Err(format!("sayfa {s} birden fazla kez seçildi"));
        }
        idler.push(*tum.get(s).ok_or_else(|| format!("sayfa {s} yok (toplam {})", tum.len()))?);
    }
    miras_yaz(doc);
    let kok = kok_sayfalar_id(doc)?;
    for id in &idler {
        if let Ok(Object::Dictionary(d)) = doc.get_object_mut(*id) {
            d.set("Parent", Object::Reference(kok));
        }
    }
    if let Ok(Object::Dictionary(d)) = doc.get_object_mut(kok) {
        d.set("Kids", Object::Array(idler.iter().map(|i| Object::Reference(*i)).collect()));
        d.set("Count", Object::Integer(idler.len() as i64));
    }
    doc.prune_objects();
    Ok(())
}

/// Belgeleri sırayla tek PDF'te birleştirir.
pub fn birlestir(belgeler: Vec<Document>) -> Result<Document, String> {
    kapali()?;
    if belgeler.len() < 2 {
        return Err("birleştirmek için en az iki PDF gerekir".into());
    }
    let mut max_id = 1;
    let mut sayfa_idleri: Vec<ObjectId> = Vec::new();
    let mut nesneler: BTreeMap<ObjectId, Object> = BTreeMap::new();
    for mut d in belgeler {
        miras_yaz(&mut d);
        d.renumber_objects_with(max_id);
        max_id = d.max_id + 1;
        sayfa_idleri.extend(d.get_pages().into_values());
        for (id, o) in d.objects {
            let tip = o.as_dict().ok().and_then(|x| x.get(b"Type").ok()).and_then(|t| t.as_name().ok()).map(|n| n.to_vec());
            // Katalog, sayfa ağacı düğümleri ve ana hatlar yeniden kurulur.
            if matches!(tip.as_deref(), Some(b"Catalog") | Some(b"Pages") | Some(b"Outlines") | Some(b"Outline")) {
                continue;
            }
            nesneler.insert(id, o);
        }
    }
    let mut yeni = Document::with_version("1.5");
    yeni.max_id = max_id;
    for (id, o) in nesneler {
        yeni.objects.insert(id, o);
    }
    let kok = yeni.new_object_id();
    for id in &sayfa_idleri {
        if let Some(Object::Dictionary(d)) = yeni.objects.get_mut(id) {
            d.set("Parent", Object::Reference(kok));
        }
    }
    let mut p = Dictionary::new();
    p.set("Type", "Pages");
    p.set("Count", Object::Integer(sayfa_idleri.len() as i64));
    p.set("Kids", Object::Array(sayfa_idleri.iter().map(|i| Object::Reference(*i)).collect()));
    yeni.objects.insert(kok, Object::Dictionary(p));
    let mut k = Dictionary::new();
    k.set("Type", "Catalog");
    k.set("Pages", Object::Reference(kok));
    let katalog = yeni.add_object(Object::Dictionary(k));
    yeni.trailer.set("Root", Object::Reference(katalog));
    yeni.renumber_objects();
    Ok(yeni)
}

/// Akışları sıkıştırır ve kullanılmayan nesneleri atar.
pub fn sikistir(doc: &mut Document) {
    doc.prune_objects();
    doc.compress();
}

pub fn bayt_al(doc: &mut Document) -> Result<Vec<u8>, String> {
    let mut v = Vec::new();
    doc.save_to(&mut v).map_err(|e| e.to_string())?;
    Ok(v)
}

/// Çıktıyı yeni dosyaya yazar (hedef doluysa "(n)" eki); gerçek yolu döndürür.
pub fn kaydet(doc: &mut Document, yol: &Path) -> Result<PathBuf, String> {
    kapali()?;
    let b = bayt_al(doc)?;
    taa_dosya::atomik_yaz(yol, &b).map_err(|e| e.to_string())
}

/// "1-3,5,8-6" gibi metni sayfa listesine çevirir ("8-6" azalan sırayı verir).
pub fn sayfa_listesi(metin: &str) -> Result<Vec<u32>, String> {
    let mut c = Vec::new();
    for parca in metin.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        let sayi = |s: &str| s.trim().parse::<u32>().ok().filter(|n| *n >= 1).ok_or_else(|| format!("geçersiz sayfa: '{s}'"));
        match parca.split_once('-') {
            Some((a, b)) => {
                let (a, b) = (sayi(a)?, sayi(b)?);
                if a <= b {
                    c.extend(a..=b);
                } else {
                    c.extend((b..=a).rev());
                }
            }
            None => c.push(sayi(parca)?),
        }
    }
    Ok(c)
}

#[cfg(test)]
mod testler {
    use super::*;
    use lopdf::dictionary;

    /// n sayfalık, her sayfada tanıtıcı "Etiket" anahtarı olan deneme PDF'i (MediaBox atadan miras).
    fn deneme(n: u32, onek: &str) -> Document {
        let mut d = Document::with_version("1.5");
        let kok = d.new_object_id();
        let mut kids = Vec::new();
        for i in 1..=n {
            let icerik = d.add_object(lopdf::Stream::new(dictionary! {}, format!("BT /F1 12 Tf 10 10 Td ({onek}{i}) Tj ET").into_bytes()));
            let s = d.add_object(dictionary! {
                "Type" => "Page", "Parent" => kok, "Contents" => icerik,
                "Etiket" => Object::string_literal(format!("{onek}{i}")),
            });
            kids.push(Object::Reference(s));
        }
        d.objects.insert(
            kok,
            Object::Dictionary(dictionary! {
                "Type" => "Pages", "Kids" => kids, "Count" => n as i64,
                "MediaBox" => vec![0.into(), 0.into(), 200.into(), 200.into()],
            }),
        );
        let kat = d.add_object(dictionary! { "Type" => "Catalog", "Pages" => kok });
        d.trailer.set("Root", kat);
        d
    }

    fn etiketler(d: &Document) -> Vec<String> {
        d.get_pages()
            .values()
            .map(|id| String::from_utf8_lossy(d.get_dictionary(*id).unwrap().get(b"Etiket").unwrap().as_str().unwrap()).to_string())
            .collect()
    }

    fn tekrar_oku(d: &mut Document) -> Document {
        bellekten(&bayt_al(d).unwrap()).unwrap()
    }

    #[test]
    fn sec_sirala_ve_mediabox_miras() {
        let mut d = deneme(5, "A");
        sec(&mut d, &[4, 1, 2]).unwrap();
        let d2 = tekrar_oku(&mut d);
        assert_eq!(etiketler(&d2), vec!["A4", "A1", "A2"]);
        let id = *d2.get_pages().values().next().unwrap();
        assert!(d2.get_dictionary(id).unwrap().has(b"MediaBox"));
    }

    #[test]
    fn sec_hatalari() {
        let mut d = deneme(2, "A");
        assert!(sec(&mut d, &[3]).is_err());
        assert!(sec(&mut d, &[1, 1]).is_err());
        assert!(sec(&mut d, &[]).is_err());
    }

    #[test]
    fn birlestirir() {
        let mut m = birlestir(vec![deneme(2, "A"), deneme(3, "B")]).unwrap();
        let m2 = tekrar_oku(&mut m);
        assert_eq!(etiketler(&m2), vec!["A1", "A2", "B1", "B2", "B3"]);
        assert!(birlestir(vec![deneme(1, "A")]).is_err());
    }

    #[test]
    fn sikistirir_ve_okunur() {
        let mut d = deneme(3, "A");
        sikistir(&mut d);
        assert_eq!(sayfa_sayisi(&tekrar_oku(&mut d)), 3);
    }

    #[test]
    fn liste() {
        assert_eq!(sayfa_listesi("1-3,5").unwrap(), vec![1, 2, 3, 5]);
        assert_eq!(sayfa_listesi("4-2").unwrap(), vec![4, 3, 2]);
        assert!(sayfa_listesi("0").is_err());
        assert!(sayfa_listesi("a").is_err());
    }

    #[test]
    fn pdf_degil() {
        assert!(bellekten(b"merhaba").is_err());
    }
}
