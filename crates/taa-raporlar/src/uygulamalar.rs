// Sahip: Yusuf. yazan: [Claude#taa-cati-raporlar 2026-11-26]
//! TAA Çatı > Raporlar: tüm TAA uygulamalarından rapor. Her uygulama için tanım (hangi veri alanları ve
//! `taa-rapor` bölümleri) + genel "veri alanı özeti" (kayıt sayısı, boyut, son değişim, sahip).
//! SALT OKUNUR: hiçbir uygulama dosyasına yazılmaz. Kişisel veri alanlarında (kimlik) yalnız sayı verilir.
//!
//! Yönetici denetimi çağıranda yapılır (`yonetici_mi`); bu modül yetki vermez, yalnız veri toplar.
//! Usta puanı ayrı bir uygulama değildir: TerziTakip raporunda, yalnız `usta-puan` özelliği açıksa (varsayılan
//! KAPALI, avukat görüşüne kadar gölge mod) hesaplanır.

use crate::{yorumla, Esikler, Seviye, Yorum};
use serde_json::{json, Value};
use std::path::Path;
use taa_rapor::{tarih, Bolum, Durum, Kaynaklar, Rapor};

/// Bir TAA uygulamasının rapor tanımı.
pub struct UygulamaTanim {
    /// Makine adı (`/api/raporlar/...?uygulama=`).
    pub kimlik: &'static str,
    pub ad: &'static str,
    pub aciklama: &'static str,
    /// Ortak veri alanları (`<kök>/<alan>/`).
    pub alanlar: &'static [&'static str],
    /// `taa-rapor` bölüm adları (`fis`, `kasa`, …).
    pub bolumler: &'static [&'static str],
    /// Kişisel veri içeren alanlar: yalnız sayı gösterilir, içerik ve dışa aktarma yok.
    pub kisisel_alanlar: &'static [&'static str],
    /// TerziTakip: usta puanı bölümü (gölge mod).
    pub usta_puani: bool,
}

pub const UYGULAMALAR: &[UygulamaTanim] = &[
    UygulamaTanim {
        kimlik: "terzitakip",
        ad: "TerziTakip",
        aciklama: "Fişler, teslimler, kargo, bildirimler, usta iş yükü",
        alanlar: &["fis-defteri", "bildirim-kuyrugu"],
        bolumler: &["fis", "bildirim"],
        kisisel_alanlar: &[],
        usta_puani: true,
    },
    UygulamaTanim {
        kimlik: "kasa",
        ad: "TAA Kasa",
        aciklama: "Kasa defteri, kapanış ve fark",
        alanlar: &["kasa-defteri"],
        bolumler: &["kasa"],
        kisisel_alanlar: &[],
        usta_puani: false,
    },
    UygulamaTanim {
        kimlik: "tutaa",
        ad: "Tutaa",
        aciklama: "Ürün kataloğu ve ortak kimlik",
        alanlar: &["urunler", "kimlik"],
        bolumler: &["katalog"],
        kisisel_alanlar: &["kimlik"],
        usta_puani: false,
    },
    UygulamaTanim {
        kimlik: "etiket",
        ad: "TAA Etiket",
        aciklama: "Etiket basım kuyruğu",
        alanlar: &["etiket-kuyrugu"],
        bolumler: &[],
        kisisel_alanlar: &[],
        usta_puani: false,
    },
    UygulamaTanim {
        kimlik: "taarkod",
        ad: "TaaRkod",
        aciklama: "Karekod/barkod işleri",
        alanlar: &["taarkod"],
        bolumler: &[],
        kisisel_alanlar: &[],
        usta_puani: false,
    },
    UygulamaTanim {
        kimlik: "taadon",
        ad: "TaaDön",
        aciklama: "Belge dönüştürme",
        alanlar: &["belgeler"],
        bolumler: &[],
        kisisel_alanlar: &[],
        usta_puani: false,
    },
    UygulamaTanim {
        kimlik: "giydir",
        ad: "TAA Giydir",
        aciklama: "Kombin kayıtları",
        alanlar: &["giydir-kombinleri"],
        bolumler: &[],
        kisisel_alanlar: &[],
        usta_puani: false,
    },
    UygulamaTanim {
        kimlik: "vidyo",
        ad: "Video Yapıcı",
        aciklama: "Video üretimi (ortak veri alanı yazmaz)",
        alanlar: &[],
        bolumler: &[],
        kisisel_alanlar: &[],
        usta_puani: false,
    },
    UygulamaTanim {
        kimlik: "paket",
        ad: "Paket (pano ve yedek)",
        aciklama: "Pano kartları, yedekler",
        alanlar: &["pano-kartlari", "yedekler"],
        bolumler: &["pano", "yedek"],
        kisisel_alanlar: &[],
        usta_puani: false,
    },
];

/// Kimlikten tanım bulur.
pub fn tanim(kimlik: &str) -> Option<&'static UygulamaTanim> {
    UYGULAMALAR.iter().find(|u| u.kimlik == kimlik)
}

/// Rapor dönemi (YYYY-AA-GG, iki uç dahil). En çok 366 gün.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Donem {
    pub bas: String,
    pub bit: String,
}

impl Donem {
    /// `secim`: "bugun" | "7" | "30" | "ozel" (özelde `bas` ve `bit` gerekir). Boş seçim 30 gün sayılır.
    pub fn coz(secim: &str, bas: &str, bit: &str, bugun: &str) -> Result<Donem, String> {
        let g = crate::tarih_gunu(bugun).ok_or("bugün tarihi çözülemedi")?;
        let gun_metni = |d: i64| tarih::tarih_metni(d * 86_400);
        let d = match secim {
            "bugun" => Donem {
                bas: bugun.into(),
                bit: bugun.into(),
            },
            "7" => Donem {
                bas: gun_metni(g - 6),
                bit: bugun.into(),
            },
            "" | "30" => Donem {
                bas: gun_metni(g - 29),
                bit: bugun.into(),
            },
            "ozel" => {
                if !tarih::tarih_gecerli(bas) || !tarih::tarih_gecerli(bit) {
                    return Err("özel dönemde bas ve bit YYYY-AA-GG olmalı".into());
                }
                Donem {
                    bas: bas.into(),
                    bit: bit.into(),
                }
            }
            x => return Err(format!("bilinmeyen dönem: {x}")),
        };
        let (a, b) = (
            crate::tarih_gunu(&d.bas).ok_or("başlangıç çözülemedi")?,
            crate::tarih_gunu(&d.bit).ok_or("bitiş çözülemedi")?,
        );
        if b < a {
            return Err("bitiş başlangıçtan önce olamaz".into());
        }
        if b - a > 365 {
            return Err("dönem en çok 366 gün olabilir".into());
        }
        Ok(d)
    }

    fn gunler(&self) -> Vec<String> {
        let (a, b) = (
            crate::tarih_gunu(&self.bas).unwrap_or(0),
            crate::tarih_gunu(&self.bit).unwrap_or(-1),
        );
        (a..=b)
            .take(366)
            .map(|d| tarih::tarih_metni(d * 86_400))
            .collect()
    }

    fn icinde(&self, t: &str) -> bool {
        t >= self.bas.as_str() && t <= self.bit.as_str()
    }

    pub fn metin(&self) -> String {
        if self.bas == self.bit {
            self.bas.clone()
        } else {
            format!("{} – {}", self.bas, self.bit)
        }
    }
}

/// Sütun grafiği verisi (arayüz SVG çizer).
#[derive(Debug, Clone, PartialEq)]
pub struct Grafik {
    pub baslik: String,
    pub birim: String,
    pub noktalar: Vec<(String, f64)>,
}

/// Bir uygulamanın raporu: bölümler + yorumlar.
#[derive(Debug, Clone, PartialEq)]
pub struct UygulamaRaporu {
    pub kimlik: String,
    pub ad: String,
    pub aciklama: String,
    pub bolumler: Vec<Bolum>,
    pub yorumlar: Vec<Yorum>,
    pub grafikler: Vec<Grafik>,
}

/// Tüm uygulamaların raporu (tek tarihli).
#[derive(Debug, Clone, PartialEq)]
pub struct PaketRaporu {
    pub bugun: String,
    pub donem: Donem,
    pub uygulamalar: Vec<UygulamaRaporu>,
}

fn alan_adi_gecerli(a: &str) -> bool {
    !a.is_empty()
        && a.len() <= 32
        && a.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

fn boyut_metni(b: u64) -> String {
    if b >= 1024 * 1024 {
        format!("{:.1} MB", b as f64 / 1024.0 / 1024.0)
    } else if b >= 1024 {
        format!("{:.1} KB", b as f64 / 1024.0)
    } else {
        format!("{b} bayt")
    }
}

fn sahip_oku(klasor: &Path) -> Option<String> {
    let yol = klasor.join("SAHIP.toml");
    if !normal_dosya(&yol) {
        return None;
    }
    let m = std::fs::read_to_string(yol).ok()?;
    m.lines().find_map(|s| {
        let s = s.trim();
        let r = s.strip_prefix("sahip")?.trim_start().strip_prefix('=')?;
        Some(r.split('#').next()?.trim().trim_matches('"').to_string())
    })
}

/// Bir veri alanının genel özeti: dosya sayısı, kayıt (JSONL satırı), boyut, son değişim, sahip.
/// `kisisel` ise yalnız dosya/kayıt sayısı; içerik hiç okunmaz.
pub fn alan_ozeti(kok: &Path, alan: &str, kisisel: bool) -> Bolum {
    let baslik = format!("Veri alanı: {alan}");
    let ad = format!("alan-{alan}");
    let yok = |neden: &str| Bolum {
        ad: ad.clone(),
        baslik: baslik.clone(),
        durum: Durum::Yok,
        kaynak: String::new(),
        satirlar: vec![],
        uyarilar: if neden.is_empty() {
            vec![]
        } else {
            vec![neden.to_string()]
        },
    };
    if !alan_adi_gecerli(alan) {
        return yok("geçersiz alan adı");
    }
    let klasor = kok.join(alan);
    if !normal_klasor(&klasor) {
        return yok("");
    }
    let Ok(girdiler) = std::fs::read_dir(&klasor) else {
        return yok("");
    };
    let (mut dosya, mut kayit, mut bayt, mut son) = (0u64, 0u64, 0u64, 0i64);
    let mut uyarilar = vec![];
    for g in girdiler.flatten() {
        let ad_g = g.file_name().to_string_lossy().to_string();
        // symlink_metadata: bağlantı izlenmez (veri kökü dışına çıkış olmaz).
        let Ok(meta) = std::fs::symlink_metadata(g.path()) else {
            continue;
        };
        if !meta.is_file() || meta.file_type().is_symlink() || ad_g == "SAHIP.toml" {
            continue;
        }
        dosya += 1;
        bayt += meta.len();
        if let Ok(t) = meta.modified() {
            if let Ok(d) = t.duration_since(std::time::UNIX_EPOCH) {
                son = son.max(d.as_secs() as i64);
            }
        }
        if ad_g.ends_with(".jsonl") && meta.len() <= 64 * 1024 * 1024 {
            match std::fs::read(g.path()) {
                Ok(b) => kayit += b.split(|&c| c == b'\n').filter(|s| !s.is_empty()).count() as u64,
                Err(e) => uyarilar.push(format!("{ad_g} okunamadı: {e}")),
            }
        }
    }
    let mut b = Bolum {
        ad,
        baslik,
        durum: Durum::Var,
        kaynak: klasor.display().to_string(),
        satirlar: vec![],
        uyarilar,
    };
    b.satirlar.push((
        "sahip".into(),
        sahip_oku(&klasor).unwrap_or_else(|| "kayıt yok".into()),
    ));
    b.satirlar.push(("dosya".into(), dosya.to_string()));
    if !kisisel {
        b.satirlar.push(("kayıt".into(), kayit.to_string()));
        b.satirlar.push(("boyut".into(), boyut_metni(bayt)));
    } else {
        b.satirlar
            .push(("not".into(), "kişisel veri: yalnız sayı gösterilir".into()));
    }
    if son > 0 {
        b.satirlar
            .push(("son değişim".into(), tarih::tarih_metni(son)));
    }
    b
}

/// Usta puanı bölümü (TerziTakip, gölge mod). `usta-puan` kapalıysa yalnız açıklama döner.
pub fn usta_puani_bolumu(
    fis_klasoru: Option<&Path>,
    bugun: &str,
    donem: &Donem,
    grafik: &mut Vec<Grafik>,
) -> Bolum {
    let mut b = Bolum {
        ad: "usta-puan".into(),
        baslik: "Usta puanı (yalnız yönetici)".into(),
        durum: Durum::Yok,
        kaynak: String::new(),
        satirlar: vec![],
        uyarilar: vec![],
    };
    if !taa_ayar::ozellik_acik("usta-puan", false) {
        b.uyarilar.push(
            "Usta puanı kapalı: avukat görüşüne kadar gölge mod. Veri toplanır, puan hesaplanmaz; açmak yönetici kararıdır."
                .into(),
        );
        return b;
    }
    let Some(dizin) = fis_klasoru else {
        b.uyarilar
            .push("TerziTakip veri klasörü (--fis-klasoru) verilmedi.".into());
        return b;
    };
    b.kaynak = dizin.display().to_string();
    let _ = bugun;
    // Puan hesabı TerziTakip'in (taa-takip) işidir; ince çizgi kuralı gereği burada o crate'e bağlanılmaz.
    // Rapor, `usta-puan.jsonl` defterinden yalnız ham sayıları okur (okutma, işaret, tutanak).
    let usta_yol = dizin.join("usta-puan.jsonl");
    let metin = match if usta_yol.exists() && !normal_dosya(&usta_yol) {
        Err(std::io::Error::other("defter sıradan bir dosya değil"))
    } else {
        std::fs::read_to_string(&usta_yol)
    } {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => {
            b.durum = Durum::Hata;
            b.uyarilar.push(format!("usta puan defteri okunamadı: {e}"));
            return b;
        }
    };
    let mut okutma: std::collections::BTreeMap<String, u64> = Default::default();
    let (mut isaret, mut tutanak, mut bozuk) = (0u64, 0u64, 0u64);
    for satir in metin.lines().filter(|l| !l.trim().is_empty()) {
        let Ok(k) = serde_json::from_str::<Value>(satir) else {
            bozuk += 1;
            continue;
        };
        let tarih_ok = k
            .get("tarih")
            .and_then(Value::as_str)
            .is_some_and(|t| donem.icinde(t));
        match k.get("kayit").and_then(Value::as_str) {
            Some("olay") if tarih_ok => {
                let usta = k.get("usta").and_then(Value::as_str).unwrap_or("?");
                *okutma.entry(usta.to_string()).or_default() += 1;
            }
            Some("isaret") if tarih_ok => isaret += 1,
            Some("tutanak") => tutanak += 1,
            _ => {}
        }
    }
    b.durum = Durum::Var;
    b.satirlar.push(("dönem".into(), donem.metin()));
    if !okutma.is_empty() {
        grafik.push(Grafik {
            baslik: "Ustaya göre okutma (dönem)".into(),
            birim: "okutma".into(),
            noktalar: okutma.iter().map(|(u, n)| (u.clone(), *n as f64)).collect(),
        });
    }
    for (usta, n) in okutma {
        b.satirlar.push((usta, format!("{n} okutma")));
    }
    b.satirlar
        .push(("yeniden iş / şikâyet işareti".into(), isaret.to_string()));
    b.satirlar
        .push(("tutanak (tüm zamanlar)".into(), tutanak.to_string()));
    b.uyarilar.push(
        "Puan hesabı ve yorumlar TerziTakip yönetici ekranındadır (/yonetim/puan); bu raporda ham sayılar vardır."
            .into(),
    );
    if bozuk > 0 {
        b.uyarilar.push(format!("{bozuk} bozuk satır atlandı"));
    }
    b
}

/// Sembolik bağlantı / junction izlenmez: yalnız gerçek, sıradan dosya.
fn normal_dosya(yol: &Path) -> bool {
    std::fs::symlink_metadata(yol)
        .map(|m| m.is_file() && !m.file_type().is_symlink())
        .unwrap_or(false)
}

/// Sembolik bağlantı / junction olmayan gerçek klasör.
fn normal_klasor(yol: &Path) -> bool {
    std::fs::symlink_metadata(yol)
        .map(|m| m.is_dir() && !m.file_type().is_symlink())
        .unwrap_or(false)
}

const JSONL_ENCOK: u64 = 64 * 1024 * 1024;

/// JSONL defterini okur: (geçerli kayıtlar, bozuk satır sayısı). Okunamayan, bağlantı olan
/// ya da çok büyük dosya hata sayılır; boş dosya ile okunamayan dosya karışmaz.
fn jsonl_oku(yol: &Path) -> Result<(Vec<Value>, usize), String> {
    use std::io::BufRead;
    if !normal_dosya(yol) {
        return Err("defter sıradan bir dosya değil".into());
    }
    let boy = std::fs::metadata(yol).map_err(|e| e.to_string())?.len();
    if boy > JSONL_ENCOK {
        return Err(format!(
            "defter çok büyük ({} MB üstü)",
            JSONL_ENCOK / 1024 / 1024
        ));
    }
    let f = std::fs::File::open(yol).map_err(|e| e.to_string())?;
    let (mut kayit, mut bozuk) = (vec![], 0usize);
    for satir in std::io::BufReader::new(f).lines() {
        let satir = satir.map_err(|e| e.to_string())?;
        if satir.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(&satir) {
            Ok(v) => kayit.push(v),
            Err(_) => bozuk += 1,
        }
    }
    Ok((kayit, bozuk))
}

fn kurus_tl(k: i64) -> String {
    taa_rapor::kurus_metni(k)
}

/// Kasa: dönemdeki kapanışlar, toplam sayılan/fark, kapanışı girilmeyen günler (kural) ve günlük sayım grafiği.
fn kasa_donemi(
    kok: &Path,
    donem: &Donem,
    bugun: &str,
    bolumler: &mut Vec<Bolum>,
    yorumlar: &mut Vec<Yorum>,
    grafikler: &mut Vec<Grafik>,
) {
    let yol = kok.join("kasa-defteri").join("kasa.jsonl");
    if !yol.exists() {
        return;
    }
    let (kayitlar, bozuk) = match jsonl_oku(&yol) {
        Ok(x) => x,
        Err(e) => {
            let metin = format!("Kasa defteri okunamadı: {e}");
            bolumler.push(Bolum {
                ad: "kasa-donem".into(),
                baslik: format!("Kasa kapanışları ({})", donem.metin()),
                durum: Durum::Hata,
                kaynak: String::new(),
                satirlar: vec![],
                uyarilar: vec![metin.clone()],
            });
            yorumlar.push(Yorum {
                seviye: Seviye::Uyari,
                bolum: "kasa".into(),
                metin,
            });
            return;
        }
    };
    let str_ = |v: &Value, a: &str| v.get(a).and_then(Value::as_str).unwrap_or("").to_string();
    let iptal: Vec<String> = kayitlar
        .iter()
        .filter(|k| str_(k, "tur") == "iptal")
        .map(|k| str_(k.get("v").unwrap_or(&Value::Null), "id"))
        .collect();
    let mut gunluk: std::collections::BTreeMap<String, i64> = Default::default();
    let (mut n, mut fark, mut sayilan, mut tasti) = (0u64, 0i64, 0i64, false);
    for k in kayitlar.iter().filter(|k| str_(k, "tur") == "kapanis") {
        let v = k.get("v").unwrap_or(&Value::Null);
        if iptal.contains(&str_(v, "id")) || !donem.icinde(&str_(v, "tarih")) {
            continue;
        }
        n += 1;
        let (sy, fk) = (
            v.get("sayilan_kurus").and_then(Value::as_i64).unwrap_or(0),
            v.get("fark_kurus").and_then(Value::as_i64).unwrap_or(0),
        );
        match (sayilan.checked_add(sy), fark.checked_add(fk)) {
            (Some(a), Some(b)) => {
                sayilan = a;
                fark = b;
            }
            _ => {
                tasti = true;
            }
        }
        let g = gunluk.entry(str_(v, "tarih")).or_default();
        *g = g.saturating_add(sy);
    }
    let eksik: Vec<String> = donem
        .gunler()
        .into_iter()
        .filter(|g| g.as_str() < bugun && !gunluk.contains_key(g))
        .collect();
    let mut b = Bolum {
        ad: "kasa-donem".into(),
        baslik: format!("Kasa kapanışları ({})", donem.metin()),
        durum: Durum::Var,
        kaynak: String::new(),
        satirlar: vec![
            ("kapanış".into(), n.to_string()),
            ("toplam sayılan".into(), kurus_tl(sayilan)),
            ("toplam fark".into(), kurus_tl(fark)),
            ("kapanışı girilmeyen gün".into(), eksik.len().to_string()),
        ],
        uyarilar: vec![],
    };
    if bozuk > 0 {
        let metin =
            format!("Kasa defterinde {bozuk} bozuk satır atlandı; toplamlar eksik olabilir.");
        b.uyarilar.push(metin.clone());
        yorumlar.push(Yorum {
            seviye: Seviye::Dikkat,
            bolum: "kasa".into(),
            metin,
        });
    }
    if tasti {
        b.durum = Durum::Hata;
        let metin =
            "Kasa toplamları sayı sınırını aştı; toplamlar güvenilmez, kayıtları kontrol edin."
                .to_string();
        b.uyarilar.push(metin.clone());
        yorumlar.push(Yorum {
            seviye: Seviye::Uyari,
            bolum: "kasa".into(),
            metin,
        });
    }
    if !eksik.is_empty() {
        let ilk: Vec<&str> = eksik.iter().take(5).map(String::as_str).collect();
        let metin = format!(
            "{} günde kasa kapanışı girilmemiş (ör. {}). Mağaza kapalı günler de bu sayıya girer.",
            eksik.len(),
            ilk.join(", ")
        );
        b.uyarilar.push(metin.clone());
        yorumlar.push(Yorum {
            seviye: Seviye::Dikkat,
            bolum: "kasa".into(),
            metin,
        });
    }
    if !gunluk.is_empty() {
        grafikler.push(Grafik {
            baslik: "Günlük sayılan kasa".into(),
            birim: "TL".into(),
            noktalar: gunluk
                .into_iter()
                .map(|(g, k)| (g, k as f64 / 100.0))
                .collect(),
        });
    }
    bolumler.push(b);
}

/// TerziTakip: dönemde teslimi gelen açık fişler (iş yükü) ve gün gün grafik.
fn fis_donemi(
    kok: &Path,
    fis_klasoru: Option<&Path>,
    donem: &Donem,
    bolumler: &mut Vec<Bolum>,
    grafikler: &mut Vec<Grafik>,
) {
    let yol = fis_klasoru
        .map(|f| f.join("fisler.jsonl"))
        .filter(|y| y.exists())
        .unwrap_or_else(|| kok.join("fis-defteri").join("fisler.jsonl"));
    if !yol.exists() {
        return;
    }
    let mut fisler: Vec<(String, String, String)> = vec![]; // no, teslim, durum
    let (kayitlar, bozuk) = match jsonl_oku(&yol) {
        Ok(x) => x,
        Err(e) => {
            bolumler.push(Bolum {
                ad: "fis-donem".into(),
                baslik: format!("Teslimi gelen açık fişler ({})", donem.metin()),
                durum: Durum::Hata,
                kaynak: String::new(),
                satirlar: vec![],
                uyarilar: vec![format!("Fiş defteri okunamadı: {e}")],
            });
            return;
        }
    };
    for k in kayitlar {
        let v = k.get("v").unwrap_or(&Value::Null);
        let al = |a: &str| v.get(a).and_then(Value::as_str).unwrap_or("").to_string();
        let no = al("no");
        match k.get("tur").and_then(Value::as_str) {
            Some("fis") => fisler.push((no, al("teslim"), al("durum"))),
            Some("durum") => {
                if let Some(f) = fisler.iter_mut().find(|f| f.0 == no) {
                    f.2 = al("durum");
                }
            }
            Some("teslim") => {
                if let (Some(f), Some(y)) = (
                    fisler.iter_mut().find(|f| f.0 == no),
                    v.pointer("/degisiklik/yeni").and_then(Value::as_str),
                ) {
                    f.1 = y.to_string();
                }
            }
            _ => {}
        }
    }
    let mut gunluk: std::collections::BTreeMap<String, u64> = Default::default();
    for (_, teslim, durum) in &fisler {
        if durum != "teslim" && donem.icinde(teslim) {
            *gunluk.entry(teslim.clone()).or_default() += 1;
        }
    }
    let toplam: u64 = gunluk.values().sum();
    bolumler.push(Bolum {
        ad: "fis-donem".into(),
        baslik: format!("Teslimi gelen açık fişler ({})", donem.metin()),
        durum: Durum::Var,
        kaynak: String::new(),
        satirlar: vec![("açık fiş".into(), toplam.to_string())],
        uyarilar: if bozuk > 0 {
            vec![format!("Fiş defterinde {bozuk} bozuk satır atlandı.")]
        } else {
            vec![]
        },
    });
    if !gunluk.is_empty() {
        grafikler.push(Grafik {
            baslik: "Gün gün teslimi gelen açık fiş".into(),
            birim: "fiş".into(),
            noktalar: gunluk.into_iter().map(|(g, n)| (g, n as f64)).collect(),
        });
    }
}

/// Tek uygulamanın raporu. `rapor`: `taa_rapor::uret` çıktısı (bir kez üretilip paylaşılır).
pub fn uygulama_raporu(
    kok: &Path,
    fis_klasoru: Option<&Path>,
    t: &UygulamaTanim,
    rapor: &Rapor,
    tum_yorumlar: &[Yorum],
    donem: &Donem,
) -> UygulamaRaporu {
    let mut grafikler: Vec<Grafik> = vec![];
    let mut bolumler: Vec<Bolum> = t
        .bolumler
        .iter()
        .filter_map(|ad| rapor.bolumler.iter().find(|b| b.ad == *ad).cloned())
        .collect();
    for a in t.alanlar {
        bolumler.push(alan_ozeti(kok, a, t.kisisel_alanlar.contains(a)));
    }
    if t.usta_puani {
        bolumler.push(usta_puani_bolumu(
            fis_klasoru,
            &rapor.bugun,
            donem,
            &mut grafikler,
        ));
    }
    let mut yorumlar: Vec<Yorum> = tum_yorumlar
        .iter()
        .filter(|y| t.bolumler.contains(&y.bolum.as_str()))
        .cloned()
        .collect();
    // Dönem bölümleri (kasa kapanışı, teslim iş yükü) ve grafikler.
    match t.kimlik {
        "kasa" => kasa_donemi(
            kok,
            donem,
            &rapor.bugun,
            &mut bolumler,
            &mut yorumlar,
            &mut grafikler,
        ),
        "terzitakip" => fis_donemi(kok, fis_klasoru, donem, &mut bolumler, &mut grafikler),
        _ => {}
    }
    for b in &rapor.bolumler {
        if t.bolumler.contains(&b.ad.as_str()) && b.durum == Durum::Var && b.ad == "fis" {
            let sayilar: Vec<(String, f64)> =
                ["acik", "bekliyor", "terzihanede", "hazir", "teslim"]
                    .iter()
                    .filter_map(|k| {
                        b.satirlar
                            .iter()
                            .find(|(a, _)| a == k)
                            .and_then(|(_, v)| v.parse::<f64>().ok())
                            .map(|n| (k.to_string(), n))
                    })
                    .collect();
            if sayilar.iter().any(|(_, n)| *n > 0.0) {
                grafikler.push(Grafik {
                    baslik: "Fişlerin durumu".into(),
                    birim: "fiş".into(),
                    noktalar: sayilar,
                });
            }
        }
    }
    if bolumler.iter().all(|b| b.durum == Durum::Yok) {
        let metin = if t.alanlar.is_empty() && t.bolumler.is_empty() {
            format!("{} ortak veri alanı yazmaz; rapor verisi yok.", t.ad)
        } else {
            format!("{} için henüz okunabilir veri yok.", t.ad)
        };
        yorumlar.push(Yorum {
            seviye: Seviye::Bilgi,
            bolum: t.kimlik.into(),
            metin,
        });
    }
    UygulamaRaporu {
        kimlik: t.kimlik.into(),
        ad: t.ad.into(),
        aciklama: t.aciklama.into(),
        bolumler,
        yorumlar,
        grafikler,
    }
}

/// Tüm uygulamalar (ya da `yalniz` ile seçilen biri) için rapor üretir.
pub fn paket_raporu(
    kok: &Path,
    fis_klasoru: Option<&Path>,
    bugun: &str,
    esikler: &Esikler,
    yalniz: Option<&str>,
    donem: &Donem,
) -> Result<PaketRaporu, String> {
    let secili: Vec<&UygulamaTanim> = match yalniz {
        None | Some("") | Some("hepsi") => UYGULAMALAR.iter().collect(),
        Some(k) => vec![tanim(k).ok_or_else(|| format!("bilinmeyen uygulama: {k}"))?],
    };
    let mut kaynaklar = Kaynaklar::veri_kokunden(kok);
    if let Some(f) = fis_klasoru {
        kaynaklar.fis_klasoru_ekle(f);
    }
    let rapor = taa_rapor::uret(&kaynaklar, bugun)?;
    let yorumlar = yorumla(&rapor, esikler);
    Ok(PaketRaporu {
        bugun: bugun.into(),
        donem: donem.clone(),
        uygulamalar: secili
            .into_iter()
            .map(|t| uygulama_raporu(kok, fis_klasoru, t, &rapor, &yorumlar, donem))
            .collect(),
    })
}

fn hucre(s: &str) -> String {
    // Excel'de formül olarak çalışmasın: =, +, -, @ ile başlayanın önüne kesme konur.
    // Baştaki boşluk/denetim karakterini atan içe aktarıcılar için de korunur.
    let ilk = s.trim_start_matches(|c: char| c.is_whitespace() || c.is_control());
    let s = if ilk.starts_with(['=', '+', '-', '@']) {
        format!("'{s}")
    } else {
        s.to_string()
    };
    if s.contains(';') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s
    }
}

impl PaketRaporu {
    /// Excel çalışma kitabı (.xlsx): Özet, Bölümler, Yorumlar, Grafik verisi sayfaları.
    pub fn xlsx(&self) -> Vec<u8> {
        use crate::xlsx::{xlsx_yaz, Sayfa};
        let r = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        let mut ozet = vec![r(&["Uygulama", "Uyarı", "Dikkat", "Bilgi", "Dönem"])];
        let mut bol = vec![r(&["Uygulama", "Bölüm", "Durum", "Anahtar", "Değer"])];
        let mut yor = vec![r(&["Uygulama", "Seviye", "Bölüm", "Yorum"])];
        let mut gra = vec![r(&["Uygulama", "Grafik", "Etiket", "Değer", "Birim"])];
        for u in &self.uygulamalar {
            let say = |s: Seviye| {
                u.yorumlar
                    .iter()
                    .filter(|y| y.seviye == s)
                    .count()
                    .to_string()
            };
            ozet.push(vec![
                u.ad.clone(),
                say(Seviye::Uyari),
                say(Seviye::Dikkat),
                say(Seviye::Bilgi),
                self.donem.metin(),
            ]);
            for b in &u.bolumler {
                for (k, v) in &b.satirlar {
                    bol.push(vec![
                        u.ad.clone(),
                        b.baslik.clone(),
                        b.durum.metin().into(),
                        k.clone(),
                        v.clone(),
                    ]);
                }
                for w in &b.uyarilar {
                    bol.push(vec![
                        u.ad.clone(),
                        b.baslik.clone(),
                        b.durum.metin().into(),
                        "uyarı".into(),
                        w.clone(),
                    ]);
                }
            }
            for y in &u.yorumlar {
                yor.push(vec![
                    u.ad.clone(),
                    y.seviye.metin().into(),
                    y.bolum.clone(),
                    y.metin.clone(),
                ]);
            }
            for g in &u.grafikler {
                for (e, d) in &g.noktalar {
                    gra.push(vec![
                        u.ad.clone(),
                        g.baslik.clone(),
                        e.clone(),
                        d.to_string(),
                        g.birim.clone(),
                    ]);
                }
            }
        }
        let sayfa = |ad: &str, satirlar| Sayfa {
            ad: ad.into(),
            satirlar,
        };
        xlsx_yaz(&[
            sayfa("Özet", ozet),
            sayfa("Bölümler", bol),
            sayfa("Yorumlar", yor),
            sayfa("Grafik verisi", gra),
        ])
    }

    pub fn json(&self) -> Value {
        json!({
            "bugun": self.bugun,
            "donem": {"bas": self.donem.bas, "bit": self.donem.bit},
            "uygulamalar": self.uygulamalar.iter().map(|u| json!({
                "kimlik": u.kimlik,
                "ad": u.ad,
                "aciklama": u.aciklama,
                "bolumler": u.bolumler.iter().map(|b| json!({
                    "ad": b.ad,
                    "baslik": b.baslik,
                    "durum": b.durum.metin(),
                    "satirlar": b.satirlar.iter().map(|(k, v)| json!({"anahtar": k, "deger": v})).collect::<Vec<_>>(),
                    "uyarilar": b.uyarilar,
                })).collect::<Vec<_>>(),
                "yorumlar": u.yorumlar.iter().map(|y| json!({"seviye": y.seviye.metin(), "bolum": y.bolum, "metin": y.metin})).collect::<Vec<_>>(),
                "grafikler": u.grafikler.iter().map(|g| json!({
                    "baslik": g.baslik,
                    "birim": g.birim,
                    "noktalar": g.noktalar.iter().map(|(e, d)| json!({"etiket": e, "deger": d})).collect::<Vec<_>>(),
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        })
    }

    /// Excel'in doğrudan açtığı CSV (UTF-8 BOM, `;`): `uygulama;bolum;durum;anahtar;deger`.
    /// Veri kaynağı yollarını içermez (yalnız değerler).
    pub fn csv(&self) -> String {
        let mut s = String::from("\u{feff}uygulama;bolum;durum;anahtar;deger\r\n");
        s.push_str(&format!("rapor;;var;tarih;{}\r\n", self.bugun));
        s.push_str(&format!("rapor;;var;dönem;{}\r\n", self.donem.metin()));
        for u in &self.uygulamalar {
            for b in &u.bolumler {
                for (k, v) in &b.satirlar {
                    s.push_str(&format!(
                        "{};{};{};{};{}\r\n",
                        hucre(&u.ad),
                        hucre(&b.baslik),
                        b.durum.metin(),
                        hucre(k),
                        hucre(v)
                    ));
                }
                for w in &b.uyarilar {
                    s.push_str(&format!(
                        "{};{};{};uyarı;{}\r\n",
                        hucre(&u.ad),
                        hucre(&b.baslik),
                        b.durum.metin(),
                        hucre(w)
                    ));
                }
            }
            for y in &u.yorumlar {
                s.push_str(&format!(
                    "{};yorum;{};{};{}\r\n",
                    hucre(&u.ad),
                    y.seviye.metin(),
                    hucre(&y.bolum),
                    hucre(&y.metin)
                ));
            }
        }
        s
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn hucre_bastaki_bosluktan_sonra_formul_korunur() {
        assert!(hucre(" =1+1").starts_with("'"));
        assert!(hucre("\t@x").starts_with("'"));
        assert_eq!(hucre("normal"), "normal");
    }

    #[cfg(unix)]
    #[test]
    fn sembolik_baglanti_izlenmez() {
        let d = gecici("symlink");
        let disari = gecici("symlink-disari");
        std::fs::write(disari.join("x.jsonl"), "{}\n").unwrap();
        std::fs::create_dir_all(d.join("kasa-defteri")).unwrap();
        std::os::unix::fs::symlink(disari.join("x.jsonl"), d.join("kasa-defteri/kasa.jsonl"))
            .unwrap();
        assert!(jsonl_oku(&d.join("kasa-defteri/kasa.jsonl")).is_err());
        std::os::unix::fs::symlink(&disari, d.join("kimlik")).unwrap();
        assert!(!normal_klasor(&d.join("kimlik")));
    }

    #[test]
    fn jsonl_bozuk_satir_sayilir() {
        let d = gecici("bozuk");
        std::fs::write(d.join("a.jsonl"), "{\"a\":1}\nbozuk\n\n{\"b\":2}\n").unwrap();
        let (k, b) = jsonl_oku(&d.join("a.jsonl")).unwrap();
        assert_eq!((k.len(), b), (2, 1));
        assert!(jsonl_oku(&d.join("yok.jsonl")).is_err());
    }

    fn gecici(ad: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("taa-uyg-{}-{ad}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn alan_ozeti_sayar_ve_kisisel_icerigi_gostermez() {
        let d = gecici("alan");
        std::fs::create_dir_all(d.join("belgeler")).unwrap();
        std::fs::write(d.join("belgeler/b.jsonl"), "{\"a\":1}\n{\"a\":2}\n").unwrap();
        std::fs::write(d.join("belgeler/SAHIP.toml"), "sahip = \"taadon\" # not\n").unwrap();
        let b = alan_ozeti(&d, "belgeler", false);
        assert_eq!(b.durum, Durum::Var);
        let al = |k: &str| {
            b.satirlar
                .iter()
                .find(|(a, _)| a == k)
                .map(|(_, v)| v.clone())
        };
        assert_eq!(al("sahip").as_deref(), Some("taadon"));
        assert_eq!(al("dosya").as_deref(), Some("1"));
        assert_eq!(al("kayıt").as_deref(), Some("2"));
        let k = alan_ozeti(&d, "belgeler", true);
        assert!(k.satirlar.iter().all(|(a, _)| a != "kayıt" && a != "boyut"));
        assert_eq!(alan_ozeti(&d, "yok-alan", false).durum, Durum::Yok);
        // Yol kaçışı: geçersiz alan adı okunmaz.
        assert_eq!(alan_ozeti(&d, "../x", false).durum, Durum::Yok);
    }

    #[test]
    fn usta_puani_kapaliyken_hesaplanmaz() {
        let dn = Donem::coz("30", "", "", "2026-11-26").unwrap();
        let b = usta_puani_bolumu(None, "2026-11-26", &dn, &mut vec![]);
        if !taa_ayar::ozellik_acik("usta-puan", false) {
            assert_eq!(b.durum, Durum::Yok);
            assert!(b.uyarilar[0].contains("kapalı"));
            assert!(b.satirlar.is_empty());
        }
    }

    #[test]
    fn bos_kokte_tum_uygulamalar_calisir() {
        let d = gecici("bos");
        let dn = Donem::coz("30", "", "", "2026-11-26").unwrap();
        let r = paket_raporu(&d, None, "2026-11-26", &Esikler::default(), None, &dn).unwrap();
        assert_eq!(r.uygulamalar.len(), UYGULAMALAR.len());
        let vidyo = r.uygulamalar.iter().find(|u| u.kimlik == "vidyo").unwrap();
        assert!(vidyo.yorumlar[0].metin.contains("yazmaz"));
        assert!(paket_raporu(
            &d,
            None,
            "2026-11-26",
            &Esikler::default(),
            Some("yok"),
            &dn
        )
        .is_err());
    }

    #[test]
    fn csv_formul_ve_ayrac_kacisi() {
        assert_eq!(hucre("=1+1"), "'=1+1");
        assert_eq!(hucre("a;b"), "\"a;b\"");
        assert_eq!(hucre("düz"), "düz");
    }
}
