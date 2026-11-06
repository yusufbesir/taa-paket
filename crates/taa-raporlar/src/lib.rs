// Sahip: Yusuf. yazan: [Claude#taa-raporlar-T8 2026-11-06]
//! taa-raporlar: yönetici rapor programı ve kural tabanlı yorum motoru (bkz. docs/T8-RAPORLAR.md).
//!
//! Yusuf 2026-11-06 15:35: "raporlar sadece yöneticiler tarafından çekilebilir olmalı … yönetici hesabı ile giriş
//! yapılabilen bir taa raporlar programı … rapor yorumlayan bir motor geliştirmek uygun olur mu?"
//!
//! - **Yorum motoru** (`yorumla`): taa-rapor'un ürettiği `Rapor`u eşiklerle (`Esikler`) okur, Türkçe madde madde
//!   yorum üretir (bilgi / dikkat / uyarı). İnternet ya da yapay zekâ gerekmez; deterministiktir.
//! - **Karşılaştırma** (`karsilastir`): önceki raporla fark (geciken fiş arttı/azaldı vb.).
//! - **Yönetici denetimi** (`yonetici_mi`): taa-kimlik oturumunda `yonetici` rolü aranır. Sunucu yokken yerel kip.
//! - Eşikler `<kök>/raporlar/esikler.json` ile değiştirilebilir (yoksa varsayılanlar); yazılmaz, yalnız okunur.
//!
//! Özellik anahtarları: "raporlar", "yorum".

#[cfg(test)]
mod testler;

pub use taa_rapor::{Bolum, Durum, Rapor};

use serde_json::{json, Value};
use std::path::Path;

pub const OZELLIK: &str = "raporlar";
/// Yönetici rolü adı (sunucu sözleşmesi `roller[]`).
pub const YONETICI_ROLU: &str = "yonetici";
/// Eşik dosyası adı (`<kök>/raporlar/esikler.json`).
pub const ESIK_DOSYASI: &str = "esikler.json";

/// Yorum eşikleri. Sayılar adet, `*_kurus` alanları kuruş, `yedek_gun` gün.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Esikler {
    /// Geciken fiş bu sayıdan az ise "dikkat", bu sayı ve üstü "uyarı".
    pub geciken_uyari: i64,
    /// Kargo bekleyen fiş bu sayı ve üstü "dikkat".
    pub kargo_dikkat: i64,
    /// Açık kalan ücret toplamı bu kuruş ve üstü "dikkat".
    pub acik_ucret_dikkat_kurus: i64,
    /// Bir ustada açık fiş bu sayı ve üstü "dikkat" (kapasite).
    pub usta_acik_dikkat: i64,
    /// Son kasa farkı mutlak değeri bu kuruş ve üstü "uyarı" (sıfır dışı her fark "dikkat").
    pub kasa_fark_uyari_kurus: i64,
    /// Farklı kapanış sayısı bu sayı ve üstü "dikkat".
    pub farkli_kapanis_dikkat: i64,
    /// Bekleyen bildirim bu sayı ve üstü "uyarı".
    pub bildirim_bekleyen_uyari: i64,
    /// Son yedek bu günden eski ise "uyarı".
    pub yedek_gun: i64,
}

impl Default for Esikler {
    fn default() -> Self {
        Esikler {
            geciken_uyari: 5,
            kargo_dikkat: 3,
            acik_ucret_dikkat_kurus: 500_000,
            usta_acik_dikkat: 15,
            kasa_fark_uyari_kurus: 50_000,
            farkli_kapanis_dikkat: 3,
            bildirim_bekleyen_uyari: 20,
            yedek_gun: 2,
        }
    }
}

impl Esikler {
    /// JSON'dan okur; olmayan alanlar varsayılan kalır. Bilinmeyen anahtar hata değildir.
    pub fn jsondan(v: &Value) -> Result<Esikler, String> {
        let mut e = Esikler::default();
        let al = |ad: &str, hedef: &mut i64| -> Result<(), String> {
            match v.get(ad) {
                None | Some(Value::Null) => Ok(()),
                Some(x) => {
                    let n = x
                        .as_i64()
                        .ok_or_else(|| format!("esikler.json: '{ad}' tam sayı olmalı"))?;
                    if n < 0 {
                        return Err(format!("esikler.json: '{ad}' eksi olamaz"));
                    }
                    *hedef = n;
                    Ok(())
                }
            }
        };
        al("geciken_uyari", &mut e.geciken_uyari)?;
        al("kargo_dikkat", &mut e.kargo_dikkat)?;
        al("acik_ucret_dikkat_kurus", &mut e.acik_ucret_dikkat_kurus)?;
        al("usta_acik_dikkat", &mut e.usta_acik_dikkat)?;
        al("kasa_fark_uyari_kurus", &mut e.kasa_fark_uyari_kurus)?;
        al("farkli_kapanis_dikkat", &mut e.farkli_kapanis_dikkat)?;
        al("bildirim_bekleyen_uyari", &mut e.bildirim_bekleyen_uyari)?;
        al("yedek_gun", &mut e.yedek_gun)?;
        Ok(e)
    }

    pub fn json(&self) -> Value {
        json!({
            "geciken_uyari": self.geciken_uyari,
            "kargo_dikkat": self.kargo_dikkat,
            "acik_ucret_dikkat_kurus": self.acik_ucret_dikkat_kurus,
            "usta_acik_dikkat": self.usta_acik_dikkat,
            "kasa_fark_uyari_kurus": self.kasa_fark_uyari_kurus,
            "farkli_kapanis_dikkat": self.farkli_kapanis_dikkat,
            "bildirim_bekleyen_uyari": self.bildirim_bekleyen_uyari,
            "yedek_gun": self.yedek_gun,
        })
    }

    /// `<kök>/raporlar/esikler.json` varsa okur, yoksa varsayılan. Bozuk dosya hata döner (sessizce varsayılana düşmez).
    pub fn kokten(kok: &Path) -> Result<Esikler, String> {
        let yol = kok.join("raporlar").join(ESIK_DOSYASI);
        match std::fs::read(&yol) {
            Ok(b) => {
                let v: Value = serde_json::from_slice(&b)
                    .map_err(|e| format!("{} çözülemedi: {e}", yol.display()))?;
                Esikler::jsondan(&v)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Esikler::default()),
            Err(e) => Err(format!("{} okunamadı: {e}", yol.display())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Seviye {
    Bilgi,
    Dikkat,
    Uyari,
}

impl Seviye {
    pub fn metin(self) -> &'static str {
        match self {
            Seviye::Bilgi => "bilgi",
            Seviye::Dikkat => "dikkat",
            Seviye::Uyari => "uyarı",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yorum {
    pub seviye: Seviye,
    /// Bölüm makine adı ("fis", "kasa", … ya da "genel").
    pub bolum: String,
    pub metin: String,
}

impl Yorum {
    fn yeni(seviye: Seviye, bolum: &str, metin: impl Into<String>) -> Yorum {
        Yorum {
            seviye,
            bolum: bolum.into(),
            metin: metin.into(),
        }
    }
}

/// Bölümdeki satır değerini bulur.
pub fn deger<'a>(b: &'a Bolum, anahtar: &str) -> Option<&'a str> {
    b.satirlar
        .iter()
        .find(|(a, _)| a == anahtar)
        .map(|(_, d)| d.as_str())
}

/// Tam sayı satırı ("12").
pub fn sayi(b: &Bolum, anahtar: &str) -> Option<i64> {
    deger(b, anahtar).and_then(|d| d.trim().parse().ok())
}

/// Para satırı ("1.234,56 TL" → 123456 kuruş; "-12,00 TL" → -1200).
pub fn kurus(b: &Bolum, anahtar: &str) -> Option<i64> {
    kurus_coz(deger(b, anahtar)?)
}

pub fn kurus_coz(metin: &str) -> Option<i64> {
    let m = metin.trim().trim_end_matches("TL").trim();
    let (eksi, m) = match m.strip_prefix('-') {
        Some(k) => (true, k),
        None => (false, m),
    };
    let (lira, kr) = match m.split_once(',') {
        Some((l, k)) => (l, k),
        None => (m, "00"),
    };
    let lira: i64 = lira.replace('.', "").parse().ok()?;
    let kr: i64 = format!("{:0<2}", kr).get(..2)?.parse().ok()?;
    let toplam = lira * 100 + kr;
    Some(if eksi { -toplam } else { toplam })
}

fn gun_sayisi(y: i64, a: u32, g: u32) -> i64 {
    // Gregoryen takvimden epoch gün sayısı (Howard Hinnant).
    let y = if a <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = if a > 2 { a as i64 - 3 } else { a as i64 + 9 };
    let doy = (153 * mp + 2) / 5 + g as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// "YYYY-AA-GG" ya da "YYYYAAGG-…" damgasını gün sayısına çevirir.
pub fn tarih_gunu(metin: &str) -> Option<i64> {
    let t = metin.trim();
    let (y, a, g) = if t.len() >= 10 && t.as_bytes()[4] == b'-' {
        (t.get(0..4)?, t.get(5..7)?, t.get(8..10)?)
    } else if t.len() >= 8 {
        (t.get(0..4)?, t.get(4..6)?, t.get(6..8)?)
    } else {
        return None;
    };
    let (y, a, g): (i64, u32, u32) = (y.parse().ok()?, a.parse().ok()?, g.parse().ok()?);
    if !(1..=12).contains(&a) || !(1..=31).contains(&g) {
        return None;
    }
    Some(gun_sayisi(y, a, g))
}

fn bolum<'a>(r: &'a Rapor, ad: &str) -> Option<&'a Bolum> {
    r.bolumler.iter().find(|b| b.ad == ad)
}

/// Raporu eşiklere göre yorumlar. Sıra: uyarılar, dikkatler, bilgiler; her biri bölüm sırasıyla.
pub fn yorumla(r: &Rapor, e: &Esikler) -> Vec<Yorum> {
    let mut y: Vec<Yorum> = Vec::new();
    for b in &r.bolumler {
        match b.durum {
            Durum::Hata => y.push(Yorum::yeni(
                Seviye::Uyari,
                &b.ad,
                format!(
                    "{} okunamadı: {}. Dosyayı denetleyin.",
                    b.baslik,
                    b.uyarilar.first().cloned().unwrap_or_default()
                ),
            )),
            Durum::Yok if b.ad == "yedek" => y.push(Yorum::yeni(
                Seviye::Uyari,
                "yedek",
                "Hiç yedek yok. taa-yedek ile yedek alın.",
            )),
            _ => {}
        }
        for u in &b.uyarilar {
            if b.durum == Durum::Var {
                y.push(Yorum::yeni(
                    Seviye::Dikkat,
                    &b.ad,
                    format!("{}: {u}", b.baslik),
                ));
            }
        }
    }
    if let Some(b) = bolum(r, "fis").filter(|b| b.durum == Durum::Var) {
        let geciken = sayi(b, "geciken").unwrap_or(0);
        if geciken >= e.geciken_uyari {
            y.push(Yorum::yeni(
                Seviye::Uyari,
                "fis",
                format!("{geciken} fişin teslim tarihi geçti. Müşteriler aranmalı, terzihane sırası öne alınmalı."),
            ));
        } else if geciken > 0 {
            y.push(Yorum::yeni(
                Seviye::Dikkat,
                "fis",
                format!("{geciken} fişin teslim tarihi geçti."),
            ));
        }
        let bugun = sayi(b, "bugun_teslim").unwrap_or(0);
        if bugun > 0 {
            y.push(Yorum::yeni(
                Seviye::Bilgi,
                "fis",
                format!("Bugün teslim edilecek {bugun} fiş var."),
            ));
        }
        let kargo = sayi(b, "kargo_bekleyen").unwrap_or(0);
        if kargo >= e.kargo_dikkat {
            y.push(Yorum::yeni(
                Seviye::Dikkat,
                "fis",
                format!("{kargo} fiş kargo bekliyor; kargo listesi hazırlanmalı."),
            ));
        }
        if let Some(k) = kurus(b, "acik_kalan_ucret") {
            if k >= e.acik_ucret_dikkat_kurus {
                y.push(Yorum::yeni(
                    Seviye::Dikkat,
                    "fis",
                    format!(
                        "Açık fişlerde tahsil edilmemiş {} var.",
                        taa_rapor::kurus_metni(k)
                    ),
                ));
            }
        }
        for (a, d) in &b.satirlar {
            if let Some(usta) = a.strip_prefix("usta_acik:") {
                if let Ok(n) = d.trim().parse::<i64>() {
                    if n >= e.usta_acik_dikkat {
                        y.push(Yorum::yeni(
                            Seviye::Dikkat,
                            "fis",
                            format!("{usta} ustada {n} açık fiş var; kapasite sınırında, iş dağılımı gözden geçirilmeli."),
                        ));
                    }
                }
            }
        }
        let acik = sayi(b, "acik").unwrap_or(0);
        let toplam = sayi(b, "toplam_fis").unwrap_or(0);
        y.push(Yorum::yeni(
            Seviye::Bilgi,
            "fis",
            format!("Toplam {toplam} fiş, {acik} açık."),
        ));
    }
    if let Some(b) = bolum(r, "kasa").filter(|b| b.durum == Durum::Var) {
        if let Some(f) = kurus(b, "son_fark") {
            if f.abs() >= e.kasa_fark_uyari_kurus {
                y.push(Yorum::yeni(
                    Seviye::Uyari,
                    "kasa",
                    format!(
                        "Son kasa kapanışında {} fark var ({}). Sayım yeniden yapılmalı.",
                        taa_rapor::kurus_metni(f),
                        deger(b, "son_kapanis").unwrap_or("")
                    ),
                ));
            } else if f != 0 {
                y.push(Yorum::yeni(
                    Seviye::Dikkat,
                    "kasa",
                    format!(
                        "Son kasa kapanışında {} fark var.",
                        taa_rapor::kurus_metni(f)
                    ),
                ));
            }
        }
        let farkli = sayi(b, "farkli_kapanis").unwrap_or(0);
        if farkli >= e.farkli_kapanis_dikkat {
            y.push(Yorum::yeni(
                Seviye::Dikkat,
                "kasa",
                format!(
                    "{farkli} kapanış farklı çıktı; toplam fark {}.",
                    deger(b, "toplam_fark").unwrap_or("?")
                ),
            ));
        }
    }
    if let Some(b) = bolum(r, "bildirim").filter(|b| b.durum == Durum::Var) {
        let hata = sayi(b, "hata").unwrap_or(0);
        if hata > 0 {
            y.push(Yorum::yeni(
                Seviye::Dikkat,
                "bildirim",
                format!("{hata} bildirim gönderilemedi (hata)."),
            ));
        }
        let bekleyen = sayi(b, "bekliyor").unwrap_or(0);
        if bekleyen >= e.bildirim_bekleyen_uyari {
            y.push(Yorum::yeni(
                Seviye::Uyari,
                "bildirim",
                format!("{bekleyen} bildirim bekliyor; gönderici çalışmıyor olabilir."),
            ));
        }
    }
    if let Some(b) = bolum(r, "katalog").filter(|b| b.durum == Durum::Var) {
        let fiyatsiz = sayi(b, "fiyatsiz").unwrap_or(0);
        if fiyatsiz > 0 {
            y.push(Yorum::yeni(
                Seviye::Dikkat,
                "katalog",
                format!("{fiyatsiz} ürünün fiyatı yok; etiket basılamaz."),
            ));
        }
    }
    if let Some(b) = bolum(r, "yedek").filter(|b| b.durum == Durum::Var) {
        match (
            deger(b, "son_yedek").and_then(tarih_gunu),
            tarih_gunu(&r.bugun),
        ) {
            (Some(son), Some(bugun)) if bugun - son > e.yedek_gun => y.push(Yorum::yeni(
                Seviye::Uyari,
                "yedek",
                format!("Son yedek {} gün önce alınmış. Yedek alın.", bugun - son),
            )),
            (Some(_), Some(_)) => {}
            _ => {
                if sayi(b, "yedek").unwrap_or(0) == 0 {
                    y.push(Yorum::yeni(
                        Seviye::Uyari,
                        "yedek",
                        "Hiç yedek yok. taa-yedek ile yedek alın.",
                    ));
                }
            }
        }
    }
    if y.iter().all(|k| k.seviye == Seviye::Bilgi) {
        y.insert(
            0,
            Yorum::yeni(Seviye::Bilgi, "genel", "Olağan dışı bir durum yok."),
        );
    }
    y.sort_by_key(|k| std::cmp::Reverse(k.seviye));
    y
}

/// Önceki raporla karşılaştırma: izlenen sayılardaki değişimi yazar.
pub fn karsilastir(simdi: &Rapor, onceki: &Rapor) -> Vec<Yorum> {
    const IZLENEN: &[(&str, &str, &str)] = &[
        ("fis", "geciken", "geciken fiş"),
        ("fis", "acik", "açık fiş"),
        ("fis", "kargo_bekleyen", "kargo bekleyen fiş"),
        ("kasa", "farkli_kapanis", "farklı kasa kapanışı"),
        ("bildirim", "hata", "bildirim hatası"),
        ("bildirim", "bekliyor", "bekleyen bildirim"),
        ("katalog", "fiyatsiz", "fiyatsız ürün"),
    ];
    let mut y = Vec::new();
    for (bad, anahtar, ad) in IZLENEN {
        let s = bolum(simdi, bad).and_then(|b| sayi(b, anahtar));
        let o = bolum(onceki, bad).and_then(|b| sayi(b, anahtar));
        if let (Some(s), Some(o)) = (s, o) {
            if s != o {
                let yon = if s > o { "arttı" } else { "azaldı" };
                let seviye = if s > o && *anahtar != "acik" {
                    Seviye::Dikkat
                } else {
                    Seviye::Bilgi
                };
                y.push(Yorum::yeni(
                    seviye,
                    bad,
                    format!(
                        "Önceki rapora ({}) göre {ad} {o} → {s} ({yon}).",
                        onceki.bugun
                    ),
                ));
            }
        }
    }
    y
}

/// Yorumları metne çevirir.
pub fn yorum_metni(y: &[Yorum]) -> String {
    let mut s = String::from("YORUM\n");
    if y.is_empty() {
        s.push_str("  (yorum yok)\n");
    }
    for k in y {
        s.push_str(&format!(
            "  [{}] {}: {}\n",
            k.seviye.metin(),
            k.bolum,
            k.metin
        ));
    }
    s
}

pub fn yorum_json(y: &[Yorum]) -> Value {
    Value::Array(
        y.iter()
            .map(|k| json!({"seviye": k.seviye.metin(), "bolum": k.bolum, "metin": k.metin}))
            .collect(),
    )
}

/// Oturumda yönetici rolü var mı.
pub fn yonetici_mi(o: &taa_kimlik::Oturum) -> bool {
    o.roller.iter().any(|r| r == YONETICI_ROLU)
}

/// Önceki kayıtlı raporu (`<kök>/raporlar/rapor-*.json`, en yenisi) okur; yoksa None.
pub fn onceki_rapor(kok: &Path) -> Option<Rapor> {
    let klasor = kok.join("raporlar");
    let mut adlar: Vec<String> = std::fs::read_dir(&klasor)
        .ok()?
        .flatten()
        .filter_map(|g| g.file_name().to_str().map(str::to_string))
        .filter(|a| a.starts_with("rapor-") && a.ends_with(".json"))
        .collect();
    adlar.sort();
    let son = adlar.pop()?;
    let v: Value = serde_json::from_slice(&std::fs::read(klasor.join(son)).ok()?).ok()?;
    rapor_jsondan(v.get("rapor").unwrap_or(&v))
}

/// taa-rapor `Rapor::json()` çıktısını geri okur (yalnız karşılaştırma için gereken alanlar).
pub fn rapor_jsondan(v: &Value) -> Option<Rapor> {
    let bugun = v.get("bugun")?.as_str()?.to_string();
    let mut bolumler = Vec::new();
    for b in v.get("bolumler")?.as_array()? {
        let ad = b.get("ad")?.as_str()?.to_string();
        let durum = match b.get("durum").and_then(Value::as_str) {
            Some("var") => Durum::Var,
            Some("hata") => Durum::Hata,
            _ => Durum::Yok,
        };
        let satirlar = b
            .get("satirlar")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|s| {
                        Some((
                            s.get("anahtar")?.as_str()?.to_string(),
                            s.get("deger")?.as_str()?.to_string(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default();
        bolumler.push(Bolum {
            baslik: b
                .get("baslik")
                .and_then(Value::as_str)
                .unwrap_or(&ad)
                .to_string(),
            ad,
            durum,
            kaynak: String::new(),
            satirlar,
            uyarilar: Vec::new(),
        });
    }
    Some(Rapor { bugun, bolumler })
}
