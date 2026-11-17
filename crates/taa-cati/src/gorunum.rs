// Sahip: Yusuf. yazan: [Claude#taa-cati-panel 2026-11-17]
//! TAA Çatı paneli görünüm tercihi: `<veri kökü>\cati\gorunum.json`.
//!
//! TAA Kurucu'nun (taa-kur-tema, #66) kavramıyla aynı: tercih tarayıcıda değil veri kökünde durur (Edge profili
//! silinse de kalır), dosya sürümlüdür, atomik yazılır, bozuk dosya varsayılan sayılır; özel vurgu rengi
//! okunabilirlik kuralına uydurulur (kontrast hesabı taa-kur-tema'dan ortak). Temalar Çatı'ya özgüdür
//! (Kule Gecesi, Defter, Taş); her temanın açık ve koyu kipi vardır (tur 3 ekip kararı, docs/S3-CATI-PANEL.md).

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use taa_kur_tema::{ara_renk, kontrast, ustune_yazi};

/// Dosya biçim sürümü.
pub const GORUNUM_SURUM: u32 = 1;
/// Vurgu, panel yüzeyi üstünde gösterge olarak en az bu kontrastı vermeli.
pub const GOSTERGE_ESIK: f64 = 3.0;
/// Vurgu dolgusu üstündeki yazı en az bu kontrastı vermeli.
pub const YAZI_ESIK: f64 = 4.5;
/// Düzeltme adımı (metin rengine doğru harman) ve en çok adım.
pub const ADIM: f32 = 0.04;
pub const EN_COK_ADIM: u32 = 12;

/// Çatı temaları. Sıra Ayarlar'daki sıradır; varsayılan Kule Gecesi.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Tema {
    #[default]
    Kule,
    Defter,
    Tas,
}

impl Tema {
    pub const HEPSI: [Tema; 3] = [Tema::Kule, Tema::Defter, Tema::Tas];

    pub fn ad(self) -> &'static str {
        match self {
            Tema::Kule => "kule",
            Tema::Defter => "defter",
            Tema::Tas => "tas",
        }
    }

    pub fn addan(s: &str) -> Option<Tema> {
        Tema::HEPSI.into_iter().find(|t| t.ad() == s.trim())
    }

    /// Kip "temaya göre" iken: Kule koyu, Defter ve Taş açık.
    pub fn dogal_koyu(self) -> bool {
        matches!(self, Tema::Kule)
    }

    /// Düzeltme için gereken jetonlar: (panel yüzeyi, metin, temanın vurgusu).
    pub fn jetonlar(self, koyu: bool) -> Jeton {
        match (self, koyu) {
            (Tema::Kule, true) => Jeton {
                panel: 0x151D2C,
                metin: 0xE8ECF4,
                vurgu: 0xF2A93B,
            },
            (Tema::Kule, false) => Jeton {
                panel: 0xFFFFFF,
                metin: 0x0E1420,
                vurgu: 0xB87408,
            },
            (Tema::Defter, false) => Jeton {
                panel: 0xFFFFFF,
                metin: 0x1C2A44,
                vurgu: 0x2B4C9B,
            },
            (Tema::Defter, true) => Jeton {
                panel: 0x18223D,
                metin: 0xF3EEE4,
                vurgu: 0x8FAAF0,
            },
            (Tema::Tas, false) => Jeton {
                panel: 0xECEDE7,
                metin: 0x23261F,
                vurgu: 0x4A5C24,
            },
            (Tema::Tas, true) => Jeton {
                panel: 0x272A24,
                metin: 0xE6E8DF,
                vurgu: 0xA9BF6A,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Jeton {
    pub panel: u32,
    pub metin: u32,
    pub vurgu: u32,
}

/// Kip: temaya göre (varsayılan), açık, koyu ya da Windows'un açık/koyu ayarını izle.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Kip {
    #[default]
    Tema,
    Acik,
    Koyu,
    Sistem,
}

impl Kip {
    pub const HEPSI: [Kip; 4] = [Kip::Tema, Kip::Acik, Kip::Koyu, Kip::Sistem];
    pub fn ad(self) -> &'static str {
        match self {
            Kip::Tema => "tema",
            Kip::Acik => "acik",
            Kip::Koyu => "koyu",
            Kip::Sistem => "sistem",
        }
    }
    pub fn addan(s: &str) -> Option<Kip> {
        Kip::HEPSI.into_iter().find(|k| k.ad() == s.trim())
    }
}

/// `#RRGGBB` → u32.
pub fn hex_oku(s: &str) -> Option<u32> {
    let h = s.trim().strip_prefix('#')?;
    if h.len() != 6 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(h, 16).ok()
}

pub fn hex_yaz(r: u32) -> String {
    format!("#{:06X}", r & 0xFF_FFFF)
}

/// Düzeltme sonucu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Duzeltme {
    /// Renk olduğu gibi uygun.
    Uygun(u32),
    /// Okunabilirlik için metin rengine doğru kaydırıldı.
    Duzeltildi(u32),
    /// 12 adımda da uymadı; temanın kendi rengi kullanılır.
    Reddedildi,
}

impl Duzeltme {
    pub fn renk(self, tema_rengi: u32) -> u32 {
        match self {
            Duzeltme::Uygun(r) | Duzeltme::Duzeltildi(r) => r,
            Duzeltme::Reddedildi => tema_rengi,
        }
    }
}

fn uygun(j: &Jeton, r: u32) -> bool {
    kontrast(r, j.panel) >= GOSTERGE_ESIK && kontrast(ustune_yazi(r), r) >= YAZI_ESIK
}

/// Özel vurguyu temanın kipine uydurur (Kurucu KARAR §5 ile aynı kural): panel üstünde ≥ 3:1, üstündeki
/// yazı (siyah/beyazdan iyisi) ≥ 4,5:1. Uymazsa %4'lük adımlarla metin rengine kaydırılır (en çok 12).
pub fn vurgu_duzelt(tema: Tema, koyu: bool, renk: u32) -> Duzeltme {
    let j = tema.jetonlar(koyu);
    if uygun(&j, renk) {
        return Duzeltme::Uygun(renk);
    }
    for i in 1..=EN_COK_ADIM {
        let aday = ara_renk(renk, j.metin, ADIM * i as f32);
        if uygun(&j, aday) {
            return Duzeltme::Duzeltildi(aday);
        }
    }
    Duzeltme::Reddedildi
}

/// Görünüm tercihleri.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Gorunum {
    pub tema: Tema,
    pub kip: Kip,
    /// Tema başına özel vurgu (sıra: Kule, Defter, Taş); `None` = temanın rengi.
    pub ozel_vurgu: [Option<u32>; 3],
    /// `None` = Windows/tarayıcı "hareketi azalt" ayarını izle.
    pub hareketi_azalt: Option<bool>,
    /// İlk açılış tanıtımı gösterildi mi? "Varsayılana dön" bunu korur.
    pub tanitim_goruldu: bool,
}

fn sira(t: Tema) -> usize {
    match t {
        Tema::Kule => 0,
        Tema::Defter => 1,
        Tema::Tas => 2,
    }
}

impl Gorunum {
    pub fn ozel_vurgu(&self, t: Tema) -> Option<u32> {
        self.ozel_vurgu[sira(t)]
    }

    pub fn ozel_vurgu_ata(&mut self, t: Tema, r: Option<u32>) {
        self.ozel_vurgu[sira(t)] = r;
    }

    /// Görünüm alanları sıfırlanır; tanıtım bilgisi korunur.
    pub fn varsayilana_don(&mut self) {
        let t = self.tanitim_goruldu;
        *self = Gorunum::default();
        self.tanitim_goruldu = t;
    }

    /// Dosyadaki biçim.
    pub fn json(&self) -> Value {
        let v = |t: Tema| self.ozel_vurgu(t).map(hex_yaz);
        json!({
            "surum": GORUNUM_SURUM,
            "tema": self.tema.ad(),
            "kip": self.kip.ad(),
            "ozel_vurgu": { "kule": v(Tema::Kule), "defter": v(Tema::Defter), "tas": v(Tema::Tas) },
            "hareketi_azalt": self.hareketi_azalt,
            "tanitim_goruldu": self.tanitim_goruldu,
        })
    }

    /// Bozuk metin = varsayılan; tanınmayan değer o alanı varsayılan bırakır.
    pub fn json_oku(metin: &str) -> Gorunum {
        let Ok(v) = serde_json::from_str::<Value>(metin) else {
            return Gorunum::default();
        };
        let mut g = Gorunum::default();
        g.guncelle(&v);
        g.tanitim_goruldu = v
            .get("tanitim_goruldu")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        g
    }

    /// Kısmi güncelleme (yalnız gelen alanlar). Ekrandan gelen istek ve dosya okuma aynı yolu kullanır.
    /// `ozel_vurgu` nesnesi ya da tek `vurgu` (seçili tema için) kabul edilir; `null` = temanın rengine dön.
    pub fn guncelle(&mut self, v: &Value) {
        if let Some(t) = v.get("tema").and_then(Value::as_str).and_then(Tema::addan) {
            self.tema = t;
        }
        if let Some(k) = v.get("kip").and_then(Value::as_str).and_then(Kip::addan) {
            self.kip = k;
        }
        if let Some(o) = v.get("ozel_vurgu").and_then(Value::as_object) {
            for t in Tema::HEPSI {
                if let Some(d) = o.get(t.ad()) {
                    self.ozel_vurgu_ata(t, d.as_str().and_then(hex_oku));
                }
            }
        }
        if let Some(d) = v.get("vurgu") {
            let t = self.tema;
            self.ozel_vurgu_ata(t, d.as_str().and_then(hex_oku));
        }
        if let Some(d) = v.get("hareketi_azalt") {
            self.hareketi_azalt = d.as_bool();
        }
        if v.get("tanitim_goruldu").and_then(Value::as_bool) == Some(true) {
            self.tanitim_goruldu = true;
        }
    }

    /// Ekrana giden özet: tercih + her tema için açık/koyu kipte kullanılacak vurgu ve düzeltme durumu.
    pub fn ozet(&self) -> Value {
        let mut vurgular = serde_json::Map::new();
        for t in Tema::HEPSI {
            let mut m = serde_json::Map::new();
            for koyu in [false, true] {
                let tema_rengi = t.jetonlar(koyu).vurgu;
                let (renk, durum) = match self.ozel_vurgu(t).map(|r| vurgu_duzelt(t, koyu, r)) {
                    None => (tema_rengi, "tema"),
                    Some(Duzeltme::Uygun(r)) => (r, "uygun"),
                    Some(Duzeltme::Duzeltildi(r)) => (r, "duzeltildi"),
                    Some(Duzeltme::Reddedildi) => (tema_rengi, "reddedildi"),
                };
                m.insert(
                    if koyu { "koyu" } else { "acik" }.into(),
                    json!({ "renk": hex_yaz(renk), "yazi": hex_yaz(ustune_yazi(renk)), "durum": durum }),
                );
            }
            vurgular.insert(t.ad().into(), Value::Object(m));
        }
        let mut v = self.json();
        v["vurgular"] = Value::Object(vurgular);
        v
    }

    pub fn yukle_yoldan(yol: &Path) -> Gorunum {
        std::fs::read_to_string(yol)
            .map(|m| Gorunum::json_oku(&m))
            .unwrap_or_default()
    }

    /// Atomik yazma: aynı klasörde geçici dosya, diske boşalt, sonra yeniden adlandır.
    pub fn kaydet_yola(&self, yol: &Path) -> std::io::Result<()> {
        use std::io::Write;
        if let Some(k) = yol.parent() {
            std::fs::create_dir_all(k)?;
        }
        let gecici = yol.with_extension("json.yaz");
        {
            let mut f = std::fs::File::create(&gecici)?;
            let metin = serde_json::to_string_pretty(&self.json()).unwrap_or_default();
            f.write_all(metin.as_bytes())?;
            f.write_all(b"\n")?;
            f.sync_all()?;
        }
        std::fs::rename(&gecici, yol)
    }
}

/// `<veri kökü>/cati/gorunum.json`.
pub fn gorunum_yolu(kok: &Path) -> PathBuf {
    kok.join("cati").join("gorunum.json")
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn temalarin_kendi_vurgusu_kurala_uyar() {
        for t in Tema::HEPSI {
            for koyu in [false, true] {
                let j = t.jetonlar(koyu);
                assert_eq!(
                    vurgu_duzelt(t, koyu, j.vurgu),
                    Duzeltme::Uygun(j.vurgu),
                    "{t:?} koyu={koyu}"
                );
                assert!(kontrast(j.metin, j.panel) >= 7.0, "{t:?} koyu={koyu} metin");
            }
        }
    }

    #[test]
    fn soluk_vurgu_duzeltilir_ya_da_reddedilir() {
        // Açık zeminde açık sarı okunmaz: koyulaştırılır.
        match vurgu_duzelt(Tema::Defter, false, 0xFFE680) {
            Duzeltme::Duzeltildi(r) => assert!(kontrast(r, 0xFFFFFF) >= GOSTERGE_ESIK),
            d => panic!("beklenmedik: {d:?}"),
        }
        // Koyu zeminde çok koyu lacivert: açılır.
        match vurgu_duzelt(Tema::Kule, true, 0x1A2238) {
            Duzeltme::Duzeltildi(r) | Duzeltme::Uygun(r) => {
                assert!(kontrast(r, 0x151D2C) >= GOSTERGE_ESIK)
            }
            Duzeltme::Reddedildi => {}
        }
        // Panelle aynı renk 12 adımda kurtarılamazsa temanın rengi kalır.
        let d = vurgu_duzelt(Tema::Tas, false, 0xECEDE7);
        assert_eq!(d.renk(0x4A5C24) == 0x4A5C24, d == Duzeltme::Reddedildi);
    }

    #[test]
    fn json_gidis_donus_ve_bozuk_dosya() {
        let mut g = Gorunum {
            tema: Tema::Tas,
            kip: Kip::Sistem,
            hareketi_azalt: Some(true),
            tanitim_goruldu: true,
            ..Default::default()
        };
        g.ozel_vurgu_ata(Tema::Defter, Some(0x3366CC));
        let m = g.json().to_string();
        assert_eq!(Gorunum::json_oku(&m), g);
        assert_eq!(Gorunum::json_oku("{bozuk"), Gorunum::default());
        // Tanınmayan değer alanı varsayılan bırakır.
        let x =
            Gorunum::json_oku(r##"{"tema":"mor","kip":"koyu","ozel_vurgu":{"kule":"kırmızı"}}"##);
        assert_eq!(x.tema, Tema::Kule);
        assert_eq!(x.kip, Kip::Koyu);
        assert_eq!(x.ozel_vurgu(Tema::Kule), None);
    }

    #[test]
    fn kismi_guncelleme_ve_varsayilana_don() {
        let mut g = Gorunum::default();
        g.guncelle(&json!({"tema": "defter", "vurgu": "#335599"}));
        assert_eq!(g.tema, Tema::Defter);
        assert_eq!(g.ozel_vurgu(Tema::Defter), Some(0x335599));
        g.guncelle(&json!({"vurgu": null}));
        assert_eq!(g.ozel_vurgu(Tema::Defter), None);
        g.guncelle(&json!({"tanitim_goruldu": true, "hareketi_azalt": false}));
        g.varsayilana_don();
        assert_eq!(g.tema, Tema::Kule);
        assert_eq!(g.hareketi_azalt, None);
        assert!(g.tanitim_goruldu);
        // tanitim_goruldu yalnız true yapılabilir (geri almak "Tanıtımı yeniden göster" ile ekranda).
        g.guncelle(&json!({"tanitim_goruldu": false}));
        assert!(g.tanitim_goruldu);
    }

    #[test]
    fn ozet_her_tema_ve_kip_icin_renk_verir() {
        let mut g = Gorunum::default();
        g.ozel_vurgu_ata(Tema::Defter, Some(0xFFE680));
        let o = g.ozet();
        assert_eq!(o["vurgular"]["kule"]["koyu"]["renk"], "#F2A93B");
        assert_eq!(o["vurgular"]["kule"]["koyu"]["durum"], "tema");
        assert_ne!(o["vurgular"]["defter"]["acik"]["durum"], "uygun");
    }

    #[test]
    fn atomik_kayit() {
        let d = std::env::temp_dir().join(format!("cati-gorunum-{}", std::process::id()));
        let yol = gorunum_yolu(&d);
        let g = Gorunum {
            tema: Tema::Defter,
            ..Default::default()
        };
        g.kaydet_yola(&yol).unwrap();
        assert_eq!(Gorunum::yukle_yoldan(&yol), g);
        assert!(!yol.with_extension("json.yaz").exists());
        assert_eq!(
            Gorunum::yukle_yoldan(&d.join("yok.json")),
            Gorunum::default()
        );
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn hex() {
        assert_eq!(hex_oku("#a1B2c3"), Some(0xA1B2C3));
        assert_eq!(hex_oku("a1b2c3"), None);
        assert_eq!(hex_oku("#12345"), None);
        assert_eq!(hex_yaz(0x0A0B0C), "#0A0B0C");
    }
}
