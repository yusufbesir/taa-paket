//! `rehber.belge@1` (El Kitabı; docs/rehber-sozlesmesi.md §2.5): sürümlü konu kayıtları, doğrulayıcı, bellek deposu
//! ve Markdown alt kümesinin güvenli bloklara çevrilmesi. taa-rehber-model'de bu tür yoktur; model crate'i bu işin
//! dışındadır, bu yüzden tür panel crate'indedir.
//! Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use taa_rehber_model::model::{kimlik_gecerli, zaman_gecerli};

/// Gövde üst sınırı (bayt): 64 KiB.
pub const GOVDE_EN_COK: usize = 64 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BelgeHata {
    Gecersiz { alan: &'static str, neden: String },
    Bulunamadi(String),
}

impl std::fmt::Display for BelgeHata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BelgeHata::Gecersiz { alan, neden } => write!(f, "geçersiz alan {alan}: {neden}"),
            BelgeHata::Bulunamadi(k) => write!(f, "belge konusu bulunamadı: {k}"),
        }
    }
}

impl std::error::Error for BelgeHata {}

/// `rehber.belge@1` kaydı. Anahtar: `konu_id + surum`. Bilinmeyen alan reddedilir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BelgeKaydi {
    pub konu_id: String,
    pub surum: u64,
    pub baslik: String,
    pub govde: String,
    #[serde(default)]
    pub etiketler: Vec<String>,
    pub yayin: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub yenilik_notu: Option<String>,
}

fn gecersiz(alan: &'static str, neden: &str) -> BelgeHata {
    BelgeHata::Gecersiz {
        alan,
        neden: neden.into(),
    }
}

/// Betik/HTML işareti ya da dış görsel var mı: `<harf`, `</`, `<!`, `<?`, `![`, `javascript:`.
pub fn yasakli_icerik(govde: &str) -> bool {
    let k = govde.to_lowercase();
    if k.contains("![") || k.contains("javascript:") {
        return true;
    }
    let b = k.as_bytes();
    b.windows(2)
        .any(|w| w[0] == b'<' && (w[1].is_ascii_alphabetic() || matches!(w[1], b'/' | b'!' | b'?')))
}

impl BelgeKaydi {
    pub fn dogrula(&self) -> Result<(), BelgeHata> {
        if !kimlik_gecerli(&self.konu_id) {
            return Err(gecersiz("konu_id", "[a-z0-9][a-z0-9_-]{0,63} olmalı"));
        }
        if self.surum < 1 {
            return Err(gecersiz("surum", "en az 1 olmalı"));
        }
        let b = self.baslik.trim();
        if b.is_empty() || b.chars().count() > 160 || b.chars().any(char::is_control) {
            return Err(gecersiz("baslik", "1-160 karakter, tek satır olmalı"));
        }
        if self.govde.trim().is_empty() {
            return Err(gecersiz("govde", "boş olamaz"));
        }
        if self.govde.len() > GOVDE_EN_COK {
            return Err(gecersiz("govde", "64 KiB'ı aşamaz"));
        }
        if yasakli_icerik(&self.govde) {
            return Err(gecersiz("govde", "betik, HTML ve görsel bağlantısı olamaz"));
        }
        if self.etiketler.len() > 20
            || self.etiketler.iter().any(|e| {
                e.trim().is_empty() || e.chars().count() > 40 || e.chars().any(char::is_control)
            })
        {
            return Err(gecersiz(
                "etiketler",
                "en çok 20 etiket, her biri 1-40 karakter",
            ));
        }
        if !zaman_gecerli(&self.yayin) {
            return Err(gecersiz("yayin", "RFC 3339 UTC zaman olmalı"));
        }
        if let Some(n) = &self.yenilik_notu {
            if n.chars().count() > 300 || n.chars().any(|c| c.is_control() && c != '\n') {
                return Err(gecersiz("yenilik_notu", "en çok 300 karakter"));
            }
        }
        Ok(())
    }
}

/// Konu başına sürüm zinciri; eski sürüm okunur kalır, silme ucu yoktur.
#[derive(Debug, Clone, Default)]
pub struct BelgeDeposu {
    konular: BTreeMap<String, Vec<BelgeKaydi>>,
}

impl BelgeDeposu {
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Yeni sürüm yayımlar (ilk yayın sürüm 1). Dönen değer yeni sürümdür.
    pub fn yayinla(
        &mut self,
        konu_id: &str,
        baslik: &str,
        govde: &str,
        etiketler: &[&str],
        yenilik_notu: Option<&str>,
        zaman: &str,
    ) -> Result<u64, BelgeHata> {
        let surum = self.konular.get(konu_id).map_or(1, |v| v.len() as u64 + 1);
        let k = BelgeKaydi {
            konu_id: konu_id.into(),
            surum,
            baslik: baslik.trim().into(),
            govde: govde.into(),
            etiketler: etiketler.iter().map(|e| (*e).to_string()).collect(),
            yayin: zaman.into(),
            yenilik_notu: yenilik_notu.map(Into::into),
        };
        k.dogrula()?;
        self.konular.entry(konu_id.into()).or_default().push(k);
        Ok(surum)
    }

    /// Her konunun en son sürümü, konu kimliğine göre sıralı.
    pub fn konular(&self) -> Vec<&BelgeKaydi> {
        self.konular.values().filter_map(|v| v.last()).collect()
    }

    /// `surum` boşsa en son sürüm.
    pub fn oku(&self, konu_id: &str, surum: Option<u64>) -> Result<&BelgeKaydi, BelgeHata> {
        let v = self
            .konular
            .get(konu_id)
            .ok_or_else(|| BelgeHata::Bulunamadi(konu_id.into()))?;
        match surum {
            None => v.last(),
            Some(s) => v.iter().find(|k| k.surum == s),
        }
        .ok_or_else(|| BelgeHata::Bulunamadi(format!("{konu_id}@{}", surum.unwrap_or(0))))
    }

    pub fn surumler(&self, konu_id: &str) -> Vec<u64> {
        self.konular
            .get(konu_id)
            .map(|v| v.iter().map(|k| k.surum).collect())
            .unwrap_or_default()
    }
}

/// Okuma bloğu: ekran yalnız `textContent` ile yazar; ham HTML hiçbir yoldan sayfaya girmez.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "tur", rename_all = "lowercase")]
pub enum Blok {
    Baslik { duzey: u8, metin: String },
    Paragraf { metin: String },
    Liste { maddeler: Vec<String> },
}

/// Markdown alt kümesi: `#`..`###` başlık, `- `/`* ` liste, boş satırla ayrılan paragraf. Satır içi biçim sade metindir.
pub fn bloklar(govde: &str) -> Vec<Blok> {
    let mut cikti = Vec::new();
    let mut par: Vec<String> = Vec::new();
    let mut liste: Vec<String> = Vec::new();
    fn par_bosalt(par: &mut Vec<String>, cikti: &mut Vec<Blok>) {
        if !par.is_empty() {
            cikti.push(Blok::Paragraf {
                metin: par.join(" "),
            });
            par.clear();
        }
    }
    fn liste_bosalt(liste: &mut Vec<String>, cikti: &mut Vec<Blok>) {
        if !liste.is_empty() {
            cikti.push(Blok::Liste {
                maddeler: std::mem::take(liste),
            });
        }
    }
    for satir in govde.replace("\r\n", "\n").lines() {
        let s = satir.trim();
        if s.is_empty() {
            par_bosalt(&mut par, &mut cikti);
            liste_bosalt(&mut liste, &mut cikti);
            continue;
        }
        let diyez = s.chars().take_while(|c| *c == '#').count();
        if (1..=3).contains(&diyez) && s[diyez..].starts_with(' ') {
            par_bosalt(&mut par, &mut cikti);
            liste_bosalt(&mut liste, &mut cikti);
            cikti.push(Blok::Baslik {
                duzey: diyez as u8,
                metin: s[diyez..].trim().to_string(),
            });
        } else if let Some(m) = s.strip_prefix("- ").or_else(|| s.strip_prefix("* ")) {
            par_bosalt(&mut par, &mut cikti);
            liste.push(m.trim().to_string());
        } else {
            liste_bosalt(&mut liste, &mut cikti);
            par.push(s.to_string());
        }
    }
    par_bosalt(&mut par, &mut cikti);
    liste_bosalt(&mut liste, &mut cikti);
    cikti
}

#[cfg(test)]
mod testler {
    use super::*;

    const Z: &str = "2026-11-26T10:00:00Z";

    #[test]
    fn surum_zinciri_ve_eski_surum_okunur() {
        let mut d = BelgeDeposu::yeni();
        assert_eq!(
            d.yayinla("baslangic", "Başlangıç", "# A\nmetin", &["a"], None, Z),
            Ok(1)
        );
        assert_eq!(
            d.yayinla("baslangic", "Başlangıç 2", "yeni", &[], Some("Yeni not"), Z),
            Ok(2)
        );
        assert_eq!(d.oku("baslangic", None).unwrap().surum, 2);
        assert_eq!(d.oku("baslangic", Some(1)).unwrap().baslik, "Başlangıç");
        assert_eq!(d.surumler("baslangic"), vec![1, 2]);
        assert!(d.oku("baslangic", Some(3)).is_err());
        assert!(d.oku("yok", None).is_err());
        assert_eq!(d.konular().len(), 1);
        assert_eq!(d.konular()[0].surum, 2);
    }

    #[test]
    fn dogrulayici_yasaklari() {
        let mut d = BelgeDeposu::yeni();
        for kotu in [
            "x <script>alert(1)</script>",
            "<b>kalın</b>",
            "![gorsel](http://dis.test/a.png)",
            "[tıkla](javascript:alert(1))",
            "a </div>",
            "   ",
        ] {
            assert!(
                d.yayinla("k", "Başlık", kotu, &[], None, Z).is_err(),
                "{kotu}"
            );
        }
        assert!(
            d.yayinla("K", "Başlık", "iyi", &[], None, Z).is_err(),
            "büyük harfli kimlik"
        );
        assert!(d.yayinla("k", "", "iyi", &[], None, Z).is_err());
        assert!(d.yayinla("k", "B", "iyi", &[], None, "dün").is_err());
        assert!(d
            .yayinla("k", "B", &"x".repeat(GOVDE_EN_COK + 1), &[], None, Z)
            .is_err());
        // "a < b" HTML değildir.
        assert!(d.yayinla("k", "B", "3 < 5 doğrudur", &[], None, Z).is_ok());
        assert!(d.konular().len() == 1);
    }

    #[test]
    fn bilinmeyen_alan_reddedilir() {
        let j = r#"{"konu_id":"a","surum":1,"baslik":"b","govde":"g","yayin":"2026-11-26T10:00:00Z","ek":1}"#;
        assert!(serde_json::from_str::<BelgeKaydi>(j).is_err());
    }

    #[test]
    fn markdown_bloklari() {
        let b = bloklar("# Baş\nilk satır\nikinci satır\n\n- bir\n- iki\n\n## Alt\nson");
        assert_eq!(
            b,
            vec![
                Blok::Baslik {
                    duzey: 1,
                    metin: "Baş".into()
                },
                Blok::Paragraf {
                    metin: "ilk satır ikinci satır".into()
                },
                Blok::Liste {
                    maddeler: vec!["bir".into(), "iki".into()]
                },
                Blok::Baslik {
                    duzey: 2,
                    metin: "Alt".into()
                },
                Blok::Paragraf {
                    metin: "son".into()
                },
            ]
        );
        // dört diyez başlık sayılmaz
        assert_eq!(
            bloklar("#### x"),
            vec![Blok::Paragraf {
                metin: "#### x".into()
            }]
        );
        assert!(bloklar("").is_empty());
    }
}
