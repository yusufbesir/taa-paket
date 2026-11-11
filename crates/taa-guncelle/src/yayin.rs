// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! `yayin.json` şeması (PLAN-TAA-PAKET §16-17 özeti) ve doğrulaması.
//! `{"sema":1,"paketler":[{ad,surum,kanal,arayuz,gerektirir,tam:{url,sha256,boyut},yamalar:[{kaynak_surum,url,sha256,boyut}],en_az_surum}],"imza":{..}}`
//! İmza: `imza` alanı hariç kanonik JSON baytları üzerinde (`taa_imza::kanonik_bayt`).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use taa_kur::paket::{ad_gecerli, surum_gecerli, MAKS_TOPLAM_BAYT};
use taa_kur::KurHata;

pub const MAKS_YAYIN_BAYT: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DosyaBilgi {
    pub url: String,
    pub sha256: String,
    pub boyut: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Yama {
    pub kaynak_surum: String,
    pub url: String,
    pub sha256: String,
    pub boyut: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct YayinPaketi {
    pub ad: String,
    pub surum: String,
    pub kanal: String,
    pub arayuz: String,
    #[serde(default)]
    pub gerektirir: BTreeMap<String, String>,
    pub tam: DosyaBilgi,
    /// Delta yamaları: şemada var, T3'te kullanılmaz (her zaman `tam` dosyaya düşülür).
    #[serde(default)]
    pub yamalar: Vec<Yama>,
    /// Bu sürümün altındaki kurulumlar güncellenmek ZORUNDA (güvenlik düzeltmesi vb.).
    #[serde(default)]
    pub en_az_surum: Option<String>,
    /// Kurucu listesinde gösterilen grup (örn. "Çatı", "İş programları").
    #[serde(default)]
    pub grup: Option<String>,
    /// Kurucu listesinde gösterilen kısa açıklama.
    #[serde(default)]
    pub aciklama: Option<String>,
    /// Seçim listesinde başta işaretli gelir (zorunlu değildir).
    #[serde(default)]
    pub varsayilan_secili: bool,
    /// Bu roldeki PC'lerde seçilemez, kendiliğinden kurulur (örn. ["sunucu"]; şube PC'sinde boş).
    #[serde(default)]
    pub zorunlu_roller: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Yayin {
    pub sema: u32,
    pub paketler: Vec<YayinPaketi>,
    #[serde(default)]
    pub imza: Option<serde_json::Value>,
}

fn sha_gecerli(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|c| matches!(c, b'0'..=b'9' | b'a'..=b'f'))
}

fn hata(m: String) -> KurHata {
    KurHata::Dogrulama(format!(
        "yayin.json geçersiz: {m}. Öneri: yayıncıdan yeni yayin.json isteyin."
    ))
}

impl Yayin {
    /// Ham baytları ayrıştırır ve şemayı doğrular; imza denetimi için ham JSON değerini de döner (imza burada DENETLENMEZ).
    pub fn coz(bayt: &[u8]) -> Result<(Yayin, serde_json::Value), KurHata> {
        if bayt.len() as u64 > MAKS_YAYIN_BAYT {
            return Err(hata("dosya çok büyük".into()));
        }
        let ham: serde_json::Value =
            serde_json::from_slice(bayt).map_err(|e| hata(format!("JSON bozuk ({e})")))?;
        let y: Yayin = serde_json::from_value(ham.clone())
            .map_err(|e| hata(format!("şema uyuşmuyor ({e})")))?;
        y.dogrula().map_err(hata)?;
        Ok((y, ham))
    }

    fn dogrula(&self) -> Result<(), String> {
        if self.sema != 1 {
            return Err(format!("desteklenmeyen şema {}", self.sema));
        }
        let mut gorulen = BTreeSet::new();
        for p in &self.paketler {
            if !ad_gecerli(&p.ad) {
                return Err(format!("geçersiz paket adı {:?}", p.ad));
            }
            if !surum_gecerli(&p.surum) {
                return Err(format!("{} için geçersiz sürüm {:?}", p.ad, p.surum));
            }
            if !ad_gecerli(&p.kanal) {
                return Err(format!("{} için geçersiz kanal {:?}", p.ad, p.kanal));
            }
            if p.arayuz.is_empty() {
                return Err(format!("{} için arayuz boş", p.ad));
            }
            if !gorulen.insert((p.ad.as_str(), p.surum.as_str(), p.kanal.as_str())) {
                return Err(format!(
                    "{} {} ({}) iki kez listelenmiş",
                    p.ad, p.surum, p.kanal
                ));
            }
            dosya_dogrula(&p.ad, "tam", &p.tam.url, &p.tam.sha256, p.tam.boyut)?;
            for y in &p.yamalar {
                if !surum_gecerli(&y.kaynak_surum) {
                    return Err(format!(
                        "{} yamasında geçersiz kaynak_surum {:?}",
                        p.ad, y.kaynak_surum
                    ));
                }
                dosya_dogrula(&p.ad, "yama", &y.url, &y.sha256, y.boyut)?;
            }
            for r in &p.zorunlu_roller {
                if !ad_gecerli(r) {
                    return Err(format!("{} için geçersiz rol {r:?}", p.ad));
                }
            }
            if let Some(e) = &p.en_az_surum {
                if !surum_gecerli(e) {
                    return Err(format!("{} için geçersiz en_az_surum {e:?}", p.ad));
                }
                if taa_kur::kayit::surum_karsilastir(e, &p.surum).is_gt() {
                    return Err(format!(
                        "{} için en_az_surum ({e}) yayın sürümünden ({}) büyük",
                        p.ad, p.surum
                    ));
                }
            }
        }
        Ok(())
    }
}

fn dosya_dogrula(ad: &str, tur: &str, url: &str, sha: &str, boyut: u64) -> Result<(), String> {
    if url.trim().is_empty() {
        return Err(format!("{ad} {tur} url boş"));
    }
    if !sha_gecerli(sha) {
        return Err(format!("{ad} {tur} sha256 geçersiz"));
    }
    if boyut == 0 || boyut > MAKS_TOPLAM_BAYT {
        return Err(format!(
            "{ad} {tur} boyut 1..={MAKS_TOPLAM_BAYT} aralığında olmalı"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod rol_testleri {
    use super::*;

    fn json(ek: &str) -> Vec<u8> {
        format!(
            r#"{{"sema":1,"paketler":[{{"ad":"taa-sunucu","surum":"1.0.0","kanal":"kararli","arayuz":"a.b@1",
            "tam":{{"url":"x.zip","sha256":"{}","boyut":10}}{ek}}}]}}"#,
            "a".repeat(64)
        )
        .into_bytes()
    }

    #[test]
    fn secimli_kurucu_alanlari_okunur_ve_eskisi_bozulmaz() {
        let (y, _) = Yayin::coz(&json("")).unwrap();
        assert!(y.paketler[0].zorunlu_roller.is_empty() && !y.paketler[0].varsayilan_secili);
        let ek = r#","grup":"Çatı","aciklama":"Merkez sunucu","varsayilan_secili":true,"zorunlu_roller":["sunucu"]"#;
        let (y, _) = Yayin::coz(&json(ek)).unwrap();
        assert_eq!(y.paketler[0].zorunlu_roller, vec!["sunucu".to_string()]);
        assert_eq!(y.paketler[0].grup.as_deref(), Some("Çatı"));
        assert!(Yayin::coz(&json(r#","zorunlu_roller":["Büyük Rol"]"#)).is_err());
    }
}
