//! Şube eşitleme: www.terziademaltun.com/iletisimbilgileri sayfasından şube adı, adresi ve telefonunu okur, şube tablosunu
//! (kamusal işletme bilgisi, kişisel veri YOK) imzalayıp müşteri takip bulutlarına (Cloudflare + Vercel) `POST /api/subeler`
//! ile iter. Değişmediyse itmez (14 günde bir tazeler: bulut `ts` ister). Site okunamazsa ya da 10'dan az şube çıkarsa
//! itmez, eski tablo kalır. Yeni şube çıkarsa yeni kod üretilir (3 harf + 01). Sahip: Yusuf. yazan: [Claude#terzitakip-sube 2026-11-26].
//!
//! Elle ek bilgi: `<veri>/sube-ek.json` = {"MRK01": {"not": "...", "saat": "...", "wa": "905..."}, ...}; yoksa MRK01 için
//! "Outlet mağazamız hemen yanında" notu kullanılır.

use crate::musteri_takip::{imzala, Ayar, Gonderici};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

pub const SITE: &str = "https://www.terziademaltun.com/iletisimbilgileri";
const EN_AZ_SUBE: usize = 10;
const YENILE_SN: u64 = 14 * 86_400;
const KLASOR: &str = "musteri-takip";

/// (şehir/ad anahtarı, şube kodu). Anahtar: başlıktaki "… Mağazamız" öncesi, "İstanbul " öneki atılmış, küçük ve ASCII.
const KODLAR: [(&str, &str); 15] = [
    ("merkez", "MRK01"),
    ("pendik", "PND01"),
    ("umraniye", "UMR01"),
    ("ankara", "ANK01"),
    ("bursa", "BRS01"),
    ("sakarya", "SKR01"),
    ("diyarbakir", "DYB01"),
    ("sivas", "SVS01"),
    ("gaziantep", "GZT01"),
    ("sanliurfa", "URF01"),
    ("van", "VAN01"),
    ("siirt", "SRT01"),
    ("elazig", "ELZ01"),
    ("izmir", "IZM01"),
    ("mus", "MUS01"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sube {
    pub kod: String,
    pub ad: String,
    pub adres: String,
    pub tel: String,
}

pub fn katla(s: &str) -> String {
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

/// Kaçış dizilerini çözer, etiketleri kaldırır, boşlukları toplar; '<' ve '>' karakterleri atılır.
pub fn metin_temizle(s: &str) -> String {
    let c = s
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
        .replace("&lt;", "")
        .replace("&gt;", "");
    c.chars().filter(|ch| *ch != '<' && *ch != '>').collect::<String>().split_whitespace().collect::<Vec<_>>().join(" ")
}

fn arasi<'a>(s: &'a str, bas: &str, son: &str) -> Option<&'a str> {
    let i = s.find(bas)? + bas.len();
    let j = s[i..].find(son)? + i;
    Some(&s[i..j])
}

fn sinirla(s: &str, en_cok: usize) -> String {
    s.chars().take(en_cok).collect()
}

/// Yalnız "+" ve rakamlar, en çok 16 karakter.
fn telefon_temizle(s: &str) -> String {
    let t: String = s.chars().filter(|c| c.is_ascii_digit() || *c == '+').collect();
    sinirla(&t, 16)
}

fn kod_uret(anahtar: &str, kullanilan: &BTreeMap<String, ()>) -> String {
    let harf: String = anahtar.chars().filter(char::is_ascii_alphabetic).take(3).collect::<String>().to_ascii_uppercase();
    let harf = if harf.len() < 3 { format!("{harf}XXX").chars().take(3).collect() } else { harf };
    (1..100)
        .map(|n| format!("{harf}{n:02}"))
        .find(|k| !kullanilan.contains_key(k))
        .unwrap_or_else(|| format!("{harf}99"))
}

/// Sayfadaki şube kartlarını ve merkez WhatsApp numarasını ayıklar.
pub fn ayikla(html: &str) -> (Vec<Sube>, String) {
    let mut subeler: Vec<Sube> = Vec::new();
    let mut kullanilan: BTreeMap<String, ()> = BTreeMap::new();
    for kart in html.split("<article class=\"store-card\">").skip(1) {
        let kart = kart.split("</article>").next().unwrap_or(kart);
        let Some(baslik) = arasi(kart, "<h2>", "</h2>") else { continue };
        let baslik = metin_temizle(baslik);
        let ad = baslik.trim_end_matches("Mağazamız").trim_end_matches("Mağaza").trim().to_string();
        let adres = arasi(kart, "<strong>Adres:</strong><span>", "</span>").map(metin_temizle).unwrap_or_default();
        let tel = arasi(kart, "href=\"tel:", "\"").map(telefon_temizle).unwrap_or_default();
        if ad.is_empty() || adres.is_empty() {
            continue;
        }
        // Anahtar: "İstanbul Pendik" → "pendik"; "Ankara" → "ankara".
        let mut a = katla(&ad);
        if let Some(r) = a.strip_prefix("istanbul ") {
            a = r.to_string();
        }
        let kod = KODLAR
            .iter()
            .find(|(k, _)| *k == a)
            .map(|(_, c)| c.to_string())
            .unwrap_or_else(|| kod_uret(&a, &kullanilan));
        if kullanilan.contains_key(&kod) {
            continue; // aynı şube iki kez
        }
        kullanilan.insert(kod.clone(), ());
        subeler.push(Sube { kod, ad: sinirla(&ad, 60), adres: sinirla(&adres, 160), tel });
    }
    let wa = arasi(html, "wa.me/", "\"")
        .map(|s| s.chars().take_while(|c| c.is_ascii_digit() || *c == '+').filter(char::is_ascii_digit).collect::<String>())
        .map(|s| s.chars().take(15).collect::<String>())
        .unwrap_or_default();
    (subeler, wa)
}

/// Elle girilen ek bilgi (not, saat, wa) ya da varsayılan.
fn ek_bilgi(dizin: &Path) -> Value {
    std::fs::read_to_string(dizin.join("sube-ek.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .filter(Value::is_object)
        .unwrap_or_else(|| json!({ "MRK01": { "not": "Outlet mağazamız hemen yanında" } }))
}

/// Buluta giden tablo (ts HARİÇ; değişiklik tespiti bunun özetiyle yapılır).
pub fn tablo(subeler: &[Sube], wa: &str, ek: &Value) -> Value {
    let mut m = Map::new();
    for s in subeler {
        let e = &ek[&s.kod];
        let al = |k: &str, n: usize| sinirla(&metin_temizle(e[k].as_str().unwrap_or("")), n);
        let mut o = json!({
            "ad": s.ad,
            "adres": s.adres,
            "tel": s.tel,
            "saat": al("saat", 40),
            "wa": telefon_temizle(e["wa"].as_str().unwrap_or("")),
        });
        let not = al("not", 80);
        if !not.is_empty() {
            o["not"] = json!(not);
        }
        m.insert(s.kod.clone(), o);
    }
    json!({ "v": 1, "tur": "subeler", "wa": wa, "subeler": Value::Object(m) })
}

fn ozet(t: &Value) -> String {
    let h = Sha256::digest(serde_json::to_string(t).unwrap_or_default().as_bytes());
    h.iter().map(|b| format!("{b:02x}")).collect()
}

fn durum_yolu(dizin: &Path) -> std::path::PathBuf {
    dizin.join(KLASOR).join("subeler-gonderilen.json")
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct Sonuc {
    pub subeler: usize,
    /// Başarıyla itilen hedef sayısı.
    pub itilen: usize,
    /// Tablo önceki itilene göre değişti mi (ilk itme dahil).
    pub degisti: bool,
}

/// `curl` ile siteyi okur (HTTP hatasında başarısız).
pub fn site_getir() -> Result<String, String> {
    use std::process::Command;
    let mut c = Command::new("curl");
    c.args(["-sS", "-f", "-L", "--max-time", "30", "-A", "TAA-sube-esitleme/1.0", SITE]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    let o = c.output().map_err(|e| format!("curl çalışmadı: {e}"))?;
    if !o.status.success() {
        return Err(format!("site okunamadı: {}", String::from_utf8_lossy(&o.stderr).trim()));
    }
    Ok(String::from_utf8_lossy(&o.stdout).into_owned())
}

fn hedef_adresi(h: &str) -> Option<String> {
    h.strip_suffix("/api/yaz").map(|b| format!("{b}/api/subeler"))
}

/// Eşitleme turu. `getir` siteyi okur, `g` imzalı gövdeyi gönderir (testte sahte).
pub fn esitle(dizin: &Path, simdi: u64, getir: &dyn Fn() -> Result<String, String>, g: &dyn Gonderici) -> Result<Sonuc, String> {
    let Some(ayar) = Ayar::dizinden(dizin) else {
        return Ok(Sonuc::default());
    };
    let (subeler, wa) = ayikla(&getir()?);
    if subeler.len() < EN_AZ_SUBE {
        return Err(format!("sitede yalnız {} şube çıkarıldı (en az {EN_AZ_SUBE} gerekir); tablo itilmedi, eski tablo kalır", subeler.len()));
    }
    let t = tablo(&subeler, &wa, &ek_bilgi(dizin));
    let oz = ozet(&t);
    let yol = durum_yolu(dizin);
    let mut durum: BTreeMap<String, Value> = std::fs::read_to_string(&yol)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    let mut sonuc = Sonuc { subeler: subeler.len(), ..Default::default() };
    let mut govde = t.clone();
    govde["ts"] = json!(simdi);
    let (govde_metni, imza) = imzala(dizin, &govde)?;
    for h in &ayar.hedefler {
        let Some(url) = hedef_adresi(h) else { continue };
        let onceki = durum.get(&url);
        let ayni = onceki.and_then(|o| o["ozet"].as_str()) == Some(oz.as_str());
        let taze = onceki.and_then(|o| o["ts"].as_u64()).is_some_and(|ts| simdi.saturating_sub(ts) < YENILE_SN);
        if ayni && taze {
            continue;
        }
        if !ayni {
            sonuc.degisti = true;
        }
        match g.gonder(&url, &govde_metni, &imza) {
            Ok(()) => {
                durum.insert(url, json!({ "ozet": oz, "ts": simdi }));
                sonuc.itilen += 1;
            }
            Err((_, ileti)) => eprintln!("Şube eşitleme: {url} reddetti ya da ulaşılamadı: {ileti}"),
        }
    }
    if sonuc.itilen > 0 {
        std::fs::create_dir_all(dizin.join(KLASOR)).map_err(|e| e.to_string())?;
        std::fs::write(&yol, serde_json::to_string(&durum).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    }
    Ok(sonuc)
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::cell::RefCell;

    fn kart(ad: &str, adres: &str, tel: &str) -> String {
        format!(
            "<article class=\"store-card\">\n<div class=\"store-card__content\">\n<h2>{ad} Mağazamız</h2>\n<div class=\"store-meta\">\n<div class=\"store-meta-item\"><strong>Adres:</strong><span>{adres}</span></div>\n<div class=\"store-meta-item\"><strong>Telefon:</strong><a href=\"tel:{tel}\">{tel}</a></div>\n</div>\n</div>\n</article>\n"
        )
    }

    fn sayfa(n: usize) -> String {
        let adlar = ["İstanbul Merkez", "İstanbul Pendik", "Ankara", "Bursa", "Sakarya", "Diyarbakır", "Sivas", "Gaziantep", "Şanlıurfa", "Van", "Siirt", "Elazığ", "İzmir", "Muş", "Trabzon"];
        let mut s = String::from("<html><a href=\"https://wa.me/905448557525\">WhatsApp</a>");
        for a in adlar.iter().take(n) {
            s += &kart(a, &format!("{a} Cad. No:1 &amp; Merkez / X"), "+90 212 690 02 02");
        }
        s
    }

    fn gecici() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("taa-sube-{}-{}", std::process::id(), simdi_ns()));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn simdi_ns() -> u128 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)
    }

    struct Sahte {
        gelen: RefCell<Vec<(String, String)>>,
        reddet: bool,
    }
    impl Gonderici for Sahte {
        fn gonder(&self, url: &str, govde: &str, _imza: &str) -> Result<(), (bool, String)> {
            self.gelen.borrow_mut().push((url.into(), govde.into()));
            if self.reddet {
                Err((false, "ağ".into()))
            } else {
                Ok(())
            }
        }
    }

    fn kur(d: &Path) {
        std::fs::write(
            d.join("musteri-takip.json"),
            r#"{"acik":true,"sayfa":"https://a.example","yedek_sayfa":"https://b.example","hedefler":["https://a.example/api/yaz","https://b.example/api/yaz"]}"#,
        )
        .unwrap();
        crate::musteri_takip::genel_anahtar(d).unwrap(); // imza anahtarını üretir
    }

    #[test]
    fn ayiklama_kodlar_ve_temizlik() {
        let (s, wa) = ayikla(&sayfa(15));
        assert_eq!(s.len(), 15);
        assert_eq!(wa, "905448557525");
        assert_eq!(s[0].kod, "MRK01");
        assert_eq!(s[0].ad, "İstanbul Merkez");
        assert_eq!(s[1].kod, "PND01");
        assert!(s[0].adres.contains("& Merkez") && s[0].tel == "+902126900202");
        // Bilinmeyen şehir yeni kod alır.
        assert_eq!(s[14].kod, "TRA01");
        assert_eq!(katla("Şanlıurfa"), "sanliurfa");
        assert_eq!(metin_temizle(" a &amp;  b <script> "), "a & b script");
    }

    #[test]
    fn tablo_sinirlari_ve_not() {
        let (s, wa) = ayikla(&sayfa(12));
        let t = tablo(&s, &wa, &json!({ "MRK01": { "not": "x".repeat(200), "saat": "09:30–21:30", "wa": "+90 555" } }));
        assert_eq!(t["tur"], "subeler");
        assert_eq!(t["subeler"]["MRK01"]["not"].as_str().unwrap().chars().count(), 80);
        assert_eq!(t["subeler"]["MRK01"]["wa"], "+90555");
        assert_eq!(t["subeler"]["PND01"]["saat"], "");
        assert!(t["subeler"]["PND01"].get("not").is_none());
    }

    #[test]
    fn itme_degismeyince_itmez_az_subede_itmez_ve_kismi_hata_yeniden_denenir() {
        let d = gecici();
        kur(&d);
        let g = Sahte { gelen: RefCell::new(vec![]), reddet: false };
        // 10'dan az şube: itilmez, hata döner.
        let az = esitle(&d, 1000, &|| Ok(sayfa(9)), &g);
        assert!(az.is_err() && g.gelen.borrow().is_empty());
        // Normal tur: iki hedefe itilir, gövde imzalı biçimde ts taşır, uç /api/subeler.
        let s = esitle(&d, 2000, &|| Ok(sayfa(15)), &g).unwrap();
        assert_eq!((s.subeler, s.itilen, s.degisti), (15, 2, true));
        let gelen = g.gelen.borrow().clone();
        assert!(gelen[0].0.ends_with("/api/subeler") && gelen[1].0.starts_with("https://b.example"));
        let v: Value = serde_json::from_str(&gelen[0].1).unwrap();
        assert_eq!((v["ts"].as_u64(), v["subeler"]["MRK01"]["not"].as_str()), (Some(2000), Some("Outlet mağazamız hemen yanında")));
        assert!(gelen[0].1.len() < 16_384);
        // Değişmedi: itilmez.
        let s = esitle(&d, 3000, &|| Ok(sayfa(15)), &g).unwrap();
        assert_eq!((s.itilen, s.degisti), (0, false));
        assert_eq!(g.gelen.borrow().len(), 2);
        // Değişti (yeni şube): tekrar itilir.
        let s = esitle(&d, 4000, &|| Ok(sayfa(14)), &g).unwrap();
        assert_eq!((s.itilen, s.degisti), (2, true));
        // 14 gün sonra aynı tablo tazelenir (bulut ts kuralı).
        let s = esitle(&d, 4000 + 15 * 86_400, &|| Ok(sayfa(14)), &g).unwrap();
        assert_eq!((s.itilen, s.degisti), (2, false));
        // Hedef hatası: durum yazılmaz, sonraki turda yeniden denenir.
        let r = Sahte { gelen: RefCell::new(vec![]), reddet: true };
        let s = esitle(&d, 4000 + 30 * 86_400, &|| Ok(sayfa(13)), &r).unwrap();
        assert_eq!(s.itilen, 0);
        let g2 = Sahte { gelen: RefCell::new(vec![]), reddet: false };
        let s = esitle(&d, 4000 + 30 * 86_400 + 60, &|| Ok(sayfa(13)), &g2).unwrap();
        assert_eq!(s.itilen, 2);
    }

    #[test]
    fn site_okunamazsa_hata() {
        let d = gecici();
        kur(&d);
        let g = Sahte { gelen: RefCell::new(vec![]), reddet: false };
        assert!(esitle(&d, 1, &|| Err("site okunamadı".into()), &g).is_err());
        assert!(g.gelen.borrow().is_empty());
    }
}
