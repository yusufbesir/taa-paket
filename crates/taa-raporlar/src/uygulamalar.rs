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

/// Bir uygulamanın raporu: bölümler + yorumlar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UygulamaRaporu {
    pub kimlik: String,
    pub ad: String,
    pub aciklama: String,
    pub bolumler: Vec<Bolum>,
    pub yorumlar: Vec<Yorum>,
}

/// Tüm uygulamaların raporu (tek tarihli).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaketRaporu {
    pub bugun: String,
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
    let m = std::fs::read_to_string(klasor.join("SAHIP.toml")).ok()?;
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
    let Ok(girdiler) = std::fs::read_dir(&klasor) else {
        return yok("");
    };
    let (mut dosya, mut kayit, mut bayt, mut son) = (0u64, 0u64, 0u64, 0i64);
    let mut uyarilar = vec![];
    for g in girdiler.flatten() {
        let ad_g = g.file_name().to_string_lossy().to_string();
        let Ok(meta) = g.metadata() else { continue };
        if !meta.is_file() || ad_g == "SAHIP.toml" {
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
pub fn usta_puani_bolumu(fis_klasoru: Option<&Path>, bugun: &str) -> Bolum {
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
    let Some(gun) = crate::tarih_gunu(bugun) else {
        b.durum = Durum::Hata;
        b.uyarilar.push("bugün tarihi çözülemedi".into());
        return b;
    };
    let bas = tarih::tarih_metni((gun - 30) * 86_400);
    let girdi = (|| -> Result<_, String> {
        let ayar = taa_takip::PuanAyari::dizinden(dizin)?;
        let sure = taa_takip::SureTablosu::dizinden(dizin)?;
        let kayitlar = taa_takip::PuanDefteri::dizinden(dizin).oku()?;
        let mut teslimler = std::collections::BTreeMap::new();
        if let Ok(s) = std::fs::read_to_string(dizin.join("fisler.jsonl")) {
            for satir in s.lines().filter(|l| !l.trim().is_empty()) {
                let Ok(k) = serde_json::from_str::<Value>(satir) else {
                    continue;
                };
                let v = k.get("v").unwrap_or(&Value::Null);
                let no = v.get("no").and_then(Value::as_str).unwrap_or("");
                match k.get("tur").and_then(Value::as_str) {
                    Some("fis") => {
                        teslimler.insert(
                            no.to_string(),
                            v.get("teslim")
                                .and_then(Value::as_str)
                                .unwrap_or("")
                                .to_string(),
                        );
                    }
                    Some("teslim") => {
                        if let Some(y) = v.pointer("/degisiklik/yeni").and_then(Value::as_str) {
                            teslimler.insert(no.to_string(), y.to_string());
                        }
                    }
                    _ => {}
                }
            }
        }
        Ok((ayar, sure, kayitlar, teslimler))
    })();
    let (ayar, sure, kayitlar, teslimler) = match girdi {
        Ok(g) => g,
        Err(e) => {
            b.durum = Durum::Hata;
            b.uyarilar.push(e);
            return b;
        }
    };
    match taa_takip::puan_hesapla(&ayar, &sure, &kayitlar, &teslimler, &bas, bugun) {
        Ok(puanlar) => {
            b.durum = Durum::Var;
            b.satirlar
                .push(("dönem".into(), format!("{bas} – {bugun}")));
            for p in puanlar {
                let deger = match p.puan {
                    Some(x) => format!("{x:.1} / 10 · {} iş", p.is_adedi),
                    None => format!("deneme dönemi · {} iş", p.is_adedi),
                };
                b.satirlar.push((p.usta, deger));
            }
        }
        Err(e) => {
            b.durum = Durum::Hata;
            b.uyarilar.push(e);
        }
    }
    b
}

/// Tek uygulamanın raporu. `rapor`: `taa_rapor::uret` çıktısı (bir kez üretilip paylaşılır).
pub fn uygulama_raporu(
    kok: &Path,
    fis_klasoru: Option<&Path>,
    t: &UygulamaTanim,
    rapor: &Rapor,
    tum_yorumlar: &[Yorum],
) -> UygulamaRaporu {
    let mut bolumler: Vec<Bolum> = t
        .bolumler
        .iter()
        .filter_map(|ad| rapor.bolumler.iter().find(|b| b.ad == *ad).cloned())
        .collect();
    for a in t.alanlar {
        bolumler.push(alan_ozeti(kok, a, t.kisisel_alanlar.contains(a)));
    }
    if t.usta_puani {
        bolumler.push(usta_puani_bolumu(fis_klasoru, &rapor.bugun));
    }
    let mut yorumlar: Vec<Yorum> = tum_yorumlar
        .iter()
        .filter(|y| t.bolumler.contains(&y.bolum.as_str()))
        .cloned()
        .collect();
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
    }
}

/// Tüm uygulamalar (ya da `yalniz` ile seçilen biri) için rapor üretir.
pub fn paket_raporu(
    kok: &Path,
    fis_klasoru: Option<&Path>,
    bugun: &str,
    esikler: &Esikler,
    yalniz: Option<&str>,
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
        uygulamalar: secili
            .into_iter()
            .map(|t| uygulama_raporu(kok, fis_klasoru, t, &rapor, &yorumlar))
            .collect(),
    })
}

fn hucre(s: &str) -> String {
    // Excel'de formül olarak çalışmasın: =, +, -, @ ile başlayanın önüne kesme konur.
    let s = if s.starts_with(['=', '+', '-', '@']) {
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
    pub fn json(&self) -> Value {
        json!({
            "bugun": self.bugun,
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
            })).collect::<Vec<_>>(),
        })
    }

    /// Excel'in doğrudan açtığı CSV (UTF-8 BOM, `;`): `uygulama;bolum;durum;anahtar;deger`.
    /// Veri kaynağı yollarını içermez (yalnız değerler).
    pub fn csv(&self) -> String {
        let mut s = String::from("\u{feff}uygulama;bolum;durum;anahtar;deger\r\n");
        s.push_str(&format!("rapor;;var;tarih;{}\r\n", self.bugun));
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
        let b = usta_puani_bolumu(None, "2026-11-26");
        if !taa_ayar::ozellik_acik("usta-puan", false) {
            assert_eq!(b.durum, Durum::Yok);
            assert!(b.uyarilar[0].contains("kapalı"));
            assert!(b.satirlar.is_empty());
        }
    }

    #[test]
    fn bos_kokte_tum_uygulamalar_calisir() {
        let d = gecici("bos");
        let r = paket_raporu(&d, None, "2026-11-26", &Esikler::default(), None).unwrap();
        assert_eq!(r.uygulamalar.len(), UYGULAMALAR.len());
        let vidyo = r.uygulamalar.iter().find(|u| u.kimlik == "vidyo").unwrap();
        assert!(vidyo.yorumlar[0].metin.contains("yazmaz"));
        assert!(paket_raporu(&d, None, "2026-11-26", &Esikler::default(), Some("yok")).is_err());
    }

    #[test]
    fn csv_formul_ve_ayrac_kacisi() {
        assert_eq!(hucre("=1+1"), "'=1+1");
        assert_eq!(hucre("a;b"), "\"a;b\"");
        assert_eq!(hucre("düz"), "düz");
    }
}
