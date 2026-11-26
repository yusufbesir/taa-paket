//! Kargo kaydı (Telegram'dan girilir): alıcı, adres, ürünler, firma ve takip numarası. Sahip: Yusuf.
//! yazan: [Claude#terzitakip-kargo 2026-11-26].
//!
//! Kayıt YALNIZ yerel `kargo.jsonl` dosyasındadır (kişisel veri: alıcı adı, telefon, adres, ürün içeriği buluta GİTMEZ).
//! Buluta yalnız durum, firma anahtarı, takip numarası, kargolanma günü ve "İlçe/İl" gider (`musteri_takip::kayit`).
//! Serbest metin ayrıştırma (WhatsApp'a yazılan biçim): etiketli satırlar ("Alıcı: …") ve etiketsiz satırlar (telefon,
//! "Büyükçekmece İstanbul", ürün satırları). Kullanıcıya her zaman "Şunu anladım" özeti gösterilip onay alınır.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// (firma anahtarı, görünen ad). Bulut şemasındaki anahtarlarla aynıdır.
pub const FIRMALAR: [(&str, &str); 11] = [
    ("yurtici", "Yurtiçi Kargo"),
    ("aras", "Aras Kargo"),
    ("mng", "MNG Kargo"),
    ("ptt", "PTT Kargo"),
    ("surat", "Sürat Kargo"),
    ("ups", "UPS"),
    ("dhl", "DHL"),
    ("hepsijet", "HepsiJet"),
    ("kolaygelsin", "Kolay Gelsin"),
    ("trendyol", "Trendyol Express"),
    ("diger", "Diğer"),
];

pub const DURUMLAR: [&str; 3] = ["hazirlaniyor", "yolda", "teslim"];

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kargo {
    pub fis_no: String,
    #[serde(default)]
    pub alici_ad: String,
    #[serde(default)]
    pub alici_tel: String,
    #[serde(default)]
    pub adres: String,
    #[serde(default)]
    pub ilce: String,
    #[serde(default)]
    pub il: String,
    #[serde(default)]
    pub urunler: Vec<String>,
    /// Firma anahtarı (`FIRMALAR`), boş olabilir.
    #[serde(default)]
    pub firma: String,
    #[serde(default)]
    pub takip_no: String,
    /// "YYYY-MM-DD" ya da boş.
    #[serde(default)]
    pub kargo_tarihi: String,
    /// `DURUMLAR`'dan biri.
    #[serde(default)]
    pub durum: String,
    #[serde(default)]
    pub guncelleme: String,
    #[serde(default)]
    pub kaydeden: String,
}

fn kucuk(s: &str) -> String {
    s.trim()
        .chars()
        .map(|c| match c {
            'İ' | 'I' | 'ı' => 'i',
            'Ş' | 'ş' => 's',
            'Ğ' | 'ğ' => 'g',
            'Ü' | 'ü' => 'u',
            'Ö' | 'ö' => 'o',
            'Ç' | 'ç' => 'c',
            c => c.to_ascii_lowercase(),
        })
        .collect()
}

/// "Yurtiçi", "yurtici kargo", "Aras", "kolay gelsin" … → firma anahtarı.
pub fn firma_anahtari(girdi: &str) -> Option<&'static str> {
    let k = kucuk(girdi).replace([' ', '-', '_', '.'], "");
    let k = k.trim_end_matches("kargo").trim_end_matches("express");
    FIRMALAR.iter().map(|(a, _)| *a).find(|a| *a == k)
}

pub fn firma_adi(anahtar: &str) -> &'static str {
    FIRMALAR.iter().find(|(a, _)| *a == anahtar).map(|(_, ad)| *ad).unwrap_or("")
}

/// Takip numarası: 4–40 karakter, harf, rakam ve tire.
pub fn takip_no_gecerli(s: &str) -> bool {
    (4..=40).contains(&s.len()) && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// "İlçe/İl" bulut alanı: en çok 60 karakter, yalnız harf, boşluk, "/" ve "-".
pub fn ilce_il_metni(k: &Kargo) -> String {
    if k.ilce.is_empty() || k.il.is_empty() {
        return String::new();
    }
    let m = format!("{}/{}", k.ilce.trim(), k.il.trim());
    if m.chars().count() <= 60 && m.chars().all(|c| c.is_alphabetic() || matches!(c, ' ' | '/' | '-')) {
        m
    } else {
        String::new()
    }
}

const ILLER: [&str; 81] = [
    "adana", "adiyaman", "afyonkarahisar", "agri", "aksaray", "amasya", "ankara", "antalya", "ardahan", "artvin", "aydin",
    "balikesir", "bartin", "batman", "bayburt", "bilecik", "bingol", "bitlis", "bolu", "burdur", "bursa", "canakkale",
    "cankiri", "corum", "denizli", "diyarbakir", "duzce", "edirne", "elazig", "erzincan", "erzurum", "eskisehir",
    "gaziantep", "giresun", "gumushane", "hakkari", "hatay", "igdir", "isparta", "istanbul", "izmir", "kahramanmaras",
    "karabuk", "karaman", "kars", "kastamonu", "kayseri", "kilis", "kirikkale", "kirklareli", "kirsehir", "kocaeli",
    "konya", "kutahya", "malatya", "manisa", "mardin", "mersin", "mugla", "mus", "nevsehir", "nigde", "ordu", "osmaniye",
    "rize", "sakarya", "samsun", "sanliurfa", "siirt", "sinop", "sirnak", "sivas", "tekirdag", "tokat", "trabzon",
    "tunceli", "usak", "van", "yalova", "yozgat", "zonguldak",
];

pub fn il_mi(s: &str) -> bool {
    ILLER.contains(&kucuk(s).as_str())
}

const URUN_ANAHTARLARI: [&str; 14] = [
    "takim", "elbise", "pantolon", "ceket", "yelek", "gomlek", "kravat", "kruvaze", "mont", "palto", "etek", "smokin",
    "jile", "kemer",
];

fn urun_satiri_mi(s: &str) -> bool {
    let k = kucuk(s);
    (k.chars().next().is_some_and(|c| c.is_ascii_digit()) && !telefon_gibi(s)) || URUN_ANAHTARLARI.iter().any(|a| k.contains(a))
}

fn telefon_gibi(s: &str) -> bool {
    let r: String = s.chars().filter(char::is_ascii_digit).collect();
    (10..=13).contains(&r.len()) && s.chars().all(|c| c.is_ascii_digit() || matches!(c, ' ' | '+' | '(' | ')' | '-'))
}

/// Parantez dışındaki virgül ve noktalı virgülle böler: "2 takım (bej, haki), 2 pantolon" → 2 parça.
pub fn parantez_disi_bol(s: &str) -> Vec<String> {
    let (mut derin, mut parca, mut v) = (0i32, String::new(), Vec::new());
    for c in s.chars() {
        match c {
            '(' => {
                derin += 1;
                parca.push(c)
            }
            ')' => {
                derin = (derin - 1).max(0);
                parca.push(c)
            }
            ',' | ';' | '\n' if derin == 0 => {
                v.push(std::mem::take(&mut parca));
            }
            _ => parca.push(c),
        }
    }
    v.push(parca);
    v.into_iter().map(|p| p.trim().to_string()).filter(|p| !p.is_empty()).collect()
}

/// Metinden ilçe ve il çıkarır: "Büyükçekmece İstanbul", "Büyükçekmece/İstanbul", "… Kadıköy, İstanbul".
fn ilce_il_ayikla(s: &str) -> Option<(String, String)> {
    let ayir = s.replace(['/', ',', '-'], " ");
    let kelimeler: Vec<&str> = ayir.split_whitespace().collect();
    let n = kelimeler.len();
    if n >= 2 && il_mi(kelimeler[n - 1]) {
        return Some((kelimeler[n - 2].to_string(), kelimeler[n - 1].to_string()));
    }
    None
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Cozum {
    pub kargo: Kargo,
    /// "Adres iletecek" gibi: adres sonra gelecek.
    pub adres_sonra: bool,
}

fn etiket(satir: &str) -> Option<(&'static str, String)> {
    let (e, d) = satir.split_once(':')?;
    let anahtar = match kucuk(e).replace(' ', "").as_str() {
        "alici" | "alicia" | "aliciad" | "ad" | "isim" | "adsoyad" => "ad",
        "tel" | "telefon" | "gsm" | "aliciteleon" | "alicitelefon" | "alicitel" => "tel",
        "adres" => "adres",
        "ilce/il" | "il/ilce" | "ilceil" | "ililce" | "yer" => "ilceil",
        "ilce" => "ilce",
        "il" => "il",
        "urun" | "urunler" | "icerik" | "parcalar" => "urun",
        "firma" | "kargo" | "tasiyici" => "firma",
        "takip" | "takipno" | "takipnumarasi" => "takip",
        _ => return None,
    };
    Some((anahtar, d.trim().to_string()))
}

/// Serbest metni ya da etiketli satırları ayrıştırır. Eksik alanlar boş kalır; çağıran özetle onay alır.
pub fn coz(metin: &str) -> Cozum {
    let mut c = Cozum::default();
    let mut ad_aday: Option<String> = None;
    for satir in metin.lines().map(str::trim).filter(|s| !s.is_empty()) {
        if let Some((e, d)) = etiket(satir) {
            match e {
                "ad" => c.kargo.alici_ad = d,
                "tel" => c.kargo.alici_tel = d,
                "adres" => c.kargo.adres = d,
                "ilceil" => {
                    if let Some((a, b)) = ilce_il_ayikla(&d) {
                        c.kargo.ilce = a;
                        c.kargo.il = b;
                    }
                }
                "ilce" => c.kargo.ilce = d,
                "il" => c.kargo.il = d,
                "urun" => c.kargo.urunler.extend(parantez_disi_bol(&d)),
                "firma" => c.kargo.firma = firma_anahtari(&d).unwrap_or("").to_string(),
                "takip" => c.kargo.takip_no = d.replace(' ', ""),
                _ => {}
            }
            continue;
        }
        let k = kucuk(satir);
        if k.starts_with("adres") && (k.contains("iletecek") || k.contains("sonra") || k.contains("gelecek")) {
            c.adres_sonra = true;
            continue;
        }
        if telefon_gibi(satir) {
            c.kargo.alici_tel = satir.to_string();
            continue;
        }
        if satir.chars().any(|ch| ch.is_ascii_digit()) && !urun_satiri_mi(satir) {
            c.kargo.adres = satir.to_string();
            if let Some((a, b)) = ilce_il_ayikla(satir) {
                c.kargo.ilce = a;
                c.kargo.il = b;
            }
            continue;
        }
        if urun_satiri_mi(satir) {
            c.kargo.urunler.extend(parantez_disi_bol(satir));
            continue;
        }
        if let Some((a, b)) = ilce_il_ayikla(satir) {
            c.kargo.ilce = a;
            c.kargo.il = b;
            continue;
        }
        // Rakamsız, ürün anahtarsız ilk satır: kişi adı adayı.
        if ad_aday.is_none() && satir.split_whitespace().count() >= 2 {
            ad_aday = Some(satir.to_string());
        }
    }
    if c.kargo.alici_ad.is_empty() {
        if let Some(a) = ad_aday {
            c.kargo.alici_ad = a;
        }
    }
    c
}

/// Eksik zorunlu alanlar (Türkçe). Zorunlu: alıcı adı, telefon, ürünler; adres ya da "adres sonra".
pub fn eksikler(k: &Kargo, adres_sonra: bool) -> Vec<&'static str> {
    let mut v = Vec::new();
    if k.alici_ad.trim().is_empty() {
        v.push("alıcı adı");
    }
    if k.alici_tel.trim().is_empty() {
        v.push("telefon");
    }
    if k.urunler.is_empty() {
        v.push("ürünler");
    }
    if !adres_sonra && k.adres.trim().is_empty() && (k.ilce.is_empty() || k.il.is_empty()) {
        v.push("adres (ilçe/il) ya da \"Adres iletecek\"");
    }
    v
}

/// Adres + ilçe/il tek metin: ilçe ve il adreste zaten geçiyorsa tekrar edilmez. Hiçbiri yoksa boş döner.
fn adres_metni(k: &Kargo) -> String {
    let adres = k.adres.trim();
    let (ilce, il) = (k.ilce.trim(), k.il.trim());
    let dolu = !ilce.is_empty() && !il.is_empty();
    let a = kucuk(adres);
    let yer_var = dolu && a.contains(&kucuk(ilce)) && a.contains(&kucuk(il));
    match (adres.is_empty(), dolu && !yer_var) {
        (true, true) => format!("{ilce} {il}"),
        (false, true) => format!("{adres} {ilce} {il}"),
        _ => adres.to_string(),
    }
}

/// Kullanıcıya gösterilen "Şunu anladım" özeti (maskesiz değil: yalnız yazanın kendi girdisi geri gösterilir).
pub fn anlasilan(k: &Kargo, adres_sonra: bool) -> String {
    let satir = |e: &str, d: &str| format!("{e}: {}", if d.trim().is_empty() { "—" } else { d.trim() });
    let a = adres_metni(k);
    let adres = if a.is_empty() && adres_sonra { "Adres iletecek".to_string() } else { a };
    format!(
        "Fiş {}\n{}\n{}\n{}\nÜrünler:\n{}\n{}",
        k.fis_no,
        satir("Alıcı", &k.alici_ad),
        satir("Telefon", &k.alici_tel),
        satir("Adres", &adres),
        if k.urunler.is_empty() {
            "—".to_string()
        } else {
            k.urunler.iter().map(|u| format!("- {u}")).collect::<Vec<_>>().join("\n")
        },
        if k.takip_no.is_empty() {
            "Takip no: —".to_string()
        } else {
            format!("{} · Takip: {}", firma_adi(&k.firma), k.takip_no)
        },
    )
}

/// WhatsApp'a yapıştırılacak kargo özeti. `tam` false ise ad ve telefon maskelenir.
pub fn whatsapp_ozeti(k: &Kargo, fis_musteri: &str, fis_tel: &str, adres_sonra: bool, tam: bool) -> String {
    let ad = |s: &str| if tam { s.to_string() } else { crate::maske_ad(s) };
    let tel = |s: &str| if tam { s.to_string() } else { crate::maske_tel(s) };
    let mut s = format!("KARGO · Fiş {}\nAlıcı: {}\nTelefon: {}", k.fis_no, ad(&k.alici_ad), tel(&k.alici_tel));
    if !fis_musteri.is_empty() && kucuk(fis_musteri) != kucuk(&k.alici_ad) {
        s += &format!("\nSatın alan (fişteki isim): {} · {}", ad(fis_musteri), tel(fis_tel));
    }
    s += "\nÜrünler:";
    for u in &k.urunler {
        s += &format!("\n- {u}");
    }
    let a = adres_metni(k);
    let _ = adres_sonra; // adres yoksa her durumda "Adres iletecek" yazılır
    s += &format!("\nAdres: {}", if a.is_empty() { "Adres iletecek".to_string() } else { a });
    if !k.takip_no.is_empty() {
        s += &format!("\nKargo: {} · Takip no: {}", firma_adi(&k.firma), k.takip_no);
    }
    s
}

fn yol(dizin: &Path) -> std::path::PathBuf {
    dizin.join("kargo.jsonl")
}

/// Kaydı ekler (yalnız ekleme; aynı fiş için son kayıt geçerlidir).
pub fn kaydet(dizin: &Path, k: &Kargo) -> Result<(), String> {
    use std::io::Write;
    let satir = serde_json::to_string(k).map_err(|e| e.to_string())?;
    let mut f = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(yol(dizin))
        .map_err(|e| format!("kargo kaydı yazılamadı: {e}"))?;
    writeln!(f, "{satir}").map_err(|e| e.to_string())
}

/// Her fişin son kargo kaydı.
pub fn hepsi(dizin: &Path) -> BTreeMap<String, Kargo> {
    let mut m = BTreeMap::new();
    if let Ok(s) = std::fs::read_to_string(yol(dizin)) {
        for l in s.lines() {
            if let Ok(k) = serde_json::from_str::<Kargo>(l) {
                m.insert(k.fis_no.clone(), k);
            }
        }
    }
    m
}

pub fn son(dizin: &Path, fis_no: &str) -> Option<Kargo> {
    hepsi(dizin).remove(fis_no)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn yusufun_whatsapp_ornegi() {
        let c = coz(
            "Ayşe Yılmaz\n0532 111 22 33\n2 takım elbise (yelekli Bej ve haki Kruvaze), 2 pantolon (mavi ve siyah)\nAdres iletecek",
        );
        assert_eq!(c.kargo.alici_ad, "Ayşe Yılmaz");
        assert_eq!(c.kargo.alici_tel, "0532 111 22 33");
        assert_eq!(
            c.kargo.urunler,
            vec!["2 takım elbise (yelekli Bej ve haki Kruvaze)", "2 pantolon (mavi ve siyah)"]
        );
        assert!(c.adres_sonra);
        assert!(eksikler(&c.kargo, c.adres_sonra).is_empty());
    }

    #[test]
    fn ilce_il_ve_urun_satiri() {
        let c = coz("Antrasit Yelekli Takım, kruvaze takım\nBüyükçekmece İstanbul");
        assert_eq!(c.kargo.urunler, vec!["Antrasit Yelekli Takım", "kruvaze takım"]);
        assert_eq!((c.kargo.ilce.as_str(), c.kargo.il.as_str()), ("Büyükçekmece", "İstanbul"));
        assert_eq!(ilce_il_metni(&c.kargo), "Büyükçekmece/İstanbul");
    }

    #[test]
    fn etiketli_satirlar() {
        let c = coz("Alıcı: Mehmet Kaya\nTel: 05551112233\nAdres: Atatürk Mah. 5. Sk. No 3 Kadıköy İstanbul\nÜrünler: 1 ceket (lacivert), 2 kravat\nFirma: Yurtiçi\nTakip: 1234567890");
        assert_eq!(c.kargo.alici_ad, "Mehmet Kaya");
        assert_eq!(c.kargo.firma, "yurtici");
        assert_eq!(c.kargo.takip_no, "1234567890");
        assert_eq!(c.kargo.urunler.len(), 2);
        assert!(eksikler(&c.kargo, false).is_empty());
    }

    #[test]
    fn eksik_alan_bildirilir() {
        let c = coz("2 pantolon");
        let e = eksikler(&c.kargo, false);
        assert!(e.contains(&"alıcı adı") && e.contains(&"telefon") && e.iter().any(|x| x.starts_with("adres")));
    }

    #[test]
    fn firma_ve_takip_dogrulama() {
        for (g, a) in [("Yurtiçi", "yurtici"), ("yurtici kargo", "yurtici"), ("Aras", "aras"), ("MNG Kargo", "mng"), ("sürat", "surat"), ("Kolay Gelsin", "kolaygelsin"), ("Trendyol Express", "trendyol"), ("PTT", "ptt")] {
            assert_eq!(firma_anahtari(g), Some(a), "{g}");
        }
        assert_eq!(firma_anahtari("bilinmeyen"), None);
        assert!(takip_no_gecerli("1234567890") && takip_no_gecerli("AB-12cd") && !takip_no_gecerli("12") && !takip_no_gecerli("ab cd 12"));
    }

    #[test]
    fn bulut_ilce_il_alani_guvenli() {
        let mut k = Kargo { ilce: "Beşiktaş".into(), il: "İstanbul".into(), ..Default::default() };
        assert_eq!(ilce_il_metni(&k), "Beşiktaş/İstanbul");
        k.ilce = "A1<script>".into();
        assert_eq!(ilce_il_metni(&k), "");
    }

    #[test]
    fn kayit_son_gecerli_ve_ozet_maskeleme() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let mut k = Kargo { fis_no: "F-1".into(), alici_ad: "Ayşe Yılmaz".into(), alici_tel: "05321112233".into(), urunler: vec!["2 pantolon".into()], ilce: "Kadıköy".into(), il: "İstanbul".into(), ..Default::default() };
        kaydet(&d, &k).unwrap();
        k.takip_no = "TR12345".into();
        k.firma = "aras".into();
        kaydet(&d, &k).unwrap();
        assert_eq!(son(&d, "F-1").unwrap().takip_no, "TR12345");
        assert!(son(&d, "F-9").is_none());
        let tam = whatsapp_ozeti(&k, "Ali Veli", "05001112233", false, true);
        assert!(tam.contains("Ayşe Yılmaz") && tam.contains("05321112233") && tam.contains("Satın alan") && tam.contains("Kadıköy İstanbul") && tam.contains("Aras Kargo · Takip no: TR12345"));
        let maske = whatsapp_ozeti(&k, "Ali Veli", "05001112233", false, false);
        assert!(!maske.contains("Ayşe Yılmaz") && !maske.contains("05321112233"), "{maske}");
    }
}
