// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS3 2026-11-26]
//! Merkeze gönderim (KS3): çevrimdışı kuyruk, taşıyıcı ve merkez tarafında "geldi / eksik / geç" matrisi.
//!
//! Kuyruk: `<kasa>/giden/` imzalı paketleri bekletir; teslim edilen paket `<kasa>/gonderilen/` altına
//! taşınır (yeniden adlandırma; hiçbir dosya silinmez). Sunucuda kasa ucu henüz yok (Ş3); bugünkü taşıyıcı
//! `KlasorTasiyici`, paketi ortak bir klasöre (ör. merkezin paylaşılan klasörü) yazar. Sunucu ucu gelince
//! aynı `Tasiyici` arayüzüyle değiştirilir.

use crate::gunsonu::{
    GunSonuKaydi, ImzaKanali, ImzaKaydi, MasrafGorunum, SayimKaydi, IKINCI_IMZA_ATMADI,
    KAPANIS_LISTESI, ODEME_TURLERI,
};
use crate::telefon_imza::{ozet_sha256, CihazDurumu, CihazKaydi, MerkezKaydiYok};
use crate::xlsx::{Hucre, Sayfa};
use crate::zaman;
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Paketi bir yere teslim eden taşıyıcı.
pub trait Tasiyici {
    fn teslim(&self, ad: &str, bayt: &[u8]) -> Result<(), String>;
}

/// Paketi bir klasöre yazar (aynı adlı dosya varsa üzerine yazmaz, teslim edilmiş sayar: aynı paket).
pub struct KlasorTasiyici {
    pub hedef: PathBuf,
}

impl Tasiyici for KlasorTasiyici {
    fn teslim(&self, ad: &str, bayt: &[u8]) -> Result<(), String> {
        if !self.hedef.is_dir() {
            return Err(format!(
                "merkez klasörüne ulaşılamıyor: {}",
                self.hedef.display()
            ));
        }
        let y = self.hedef.join(ad);
        if y.exists() {
            return Ok(());
        }
        taa_dosya::uzerine_yaz_atomik(&y, bayt).map_err(|e| format!("merkeze yazılamadı: {e}"))
    }
}

fn ad_gecerli(ad: &str) -> bool {
    !ad.is_empty()
        && ad.len() <= 128
        && ad.ends_with(".json")
        && ad
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
        && !ad.contains("..")
}

/// Çevrimdışı gönderim kuyruğu.
pub struct Kuyruk {
    pub kok: PathBuf,
}

impl Kuyruk {
    pub fn giden(&self) -> PathBuf {
        self.kok.join("giden")
    }
    pub fn gonderilen(&self) -> PathBuf {
        self.kok.join("gonderilen")
    }

    /// Paketi kuyruğa alır (atomik yazım).
    pub fn ekle(&self, ad: &str, bayt: &[u8]) -> Result<(), String> {
        if !ad_gecerli(ad) {
            return Err(format!("paket adı geçersiz: {ad}"));
        }
        std::fs::create_dir_all(self.giden()).map_err(|e| e.to_string())?;
        taa_dosya::uzerine_yaz_atomik(&self.giden().join(ad), bayt).map_err(|e| e.to_string())
    }

    fn listele(k: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(k)
            .map(|d| {
                d.filter_map(Result::ok)
                    .filter_map(|e| e.file_name().into_string().ok())
                    .filter(|a| ad_gecerli(a))
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }

    pub fn bekleyenler(&self) -> Vec<String> {
        Self::listele(&self.giden())
    }

    pub fn gonderilenler(&self) -> Vec<String> {
        Self::listele(&self.gonderilen())
    }

    /// Bekleyenleri teslim etmeye çalışır. Dönüş: (teslim edilen sayısı, ilk hata).
    pub fn bosalt(&self, t: &dyn Tasiyici) -> (usize, Option<String>) {
        let mut n = 0;
        for ad in self.bekleyenler() {
            let y = self.giden().join(&ad);
            let bayt = match std::fs::read(&y) {
                Ok(b) => b,
                Err(e) => return (n, Some(e.to_string())),
            };
            if let Err(e) = t.teslim(&ad, &bayt) {
                return (n, Some(e));
            }
            if let Err(e) = std::fs::create_dir_all(self.gonderilen())
                .and_then(|_| std::fs::rename(&y, self.gonderilen().join(&ad)))
            {
                return (
                    n,
                    Some(format!("teslim edildi ama kuyruktan taşınamadı: {e}")),
                );
            }
            n += 1;
        }
        (n, None)
    }
}

// ---------------------------------------------------------------- matris

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatrisDurumu {
    Geldi,
    Gec,
    Eksik,
    /// Henüz süre dolmadı.
    Bekleniyor,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatrisHucre {
    pub tarih: String,
    pub durum: MatrisDurumu,
    pub kasa_ciro_uyusmuyor: bool,
    pub cekmece_sayisi: usize,
    /// Gelen paketlerden en az birinde ikinci imza yok ("ikinci imza atmadı").
    pub ikinci_imza_atmadi: bool,
    /// İkinci imza atılmayan paketlerin gerekçeleri.
    pub ikinci_imza_gerekceleri: Vec<String>,
    /// Telefonla imza merkezde doğrulanamadı ya da sorunlu (özet tutmuyor, cihaz iptal / başka kişide).
    pub telefon_imza_sorunlari: Vec<String>,
    /// Merkez cihaz kaydı olmadığı için doğrulanamayan telefon imzası sayısı.
    pub telefon_imza_dogrulanamadi: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct MatrisSatiri {
    pub sube: String,
    pub hucreler: Vec<MatrisHucre>,
    /// Son `pencere_gun` günde ikinci imzasız gönderilen gün sayısı (`tek_imza_uyarisi_uygula`).
    pub tek_imza_gun_sayisi: usize,
    /// Sayı ayardaki eşiğe ulaştı: merkez ekranında uyarı.
    pub tek_imza_uyarisi: bool,
}

/// Merkeze ulaşmış bir paketin özeti.
#[derive(Debug, Clone, Default)]
pub struct GelenPaket {
    pub sube: String,
    pub tarih: String,
    pub alinma_sn: i64,
    pub kasa_ciro_uyusmuyor: bool,
    pub ikinci_imza_atmadi: bool,
    pub ikinci_imza_gerekcesi: Option<String>,
    pub telefon_imza_sorunlari: Vec<String>,
    pub telefon_imza_dogrulanamadi: usize,
}

/// Paketteki telefonla imzaları merkezde denetler: imzanın onayladığı özet paketteki gün sonundan yeniden
/// hesaplanır; cihaz-kişi bağı ve iptal TAA Merkez cihaz kaydından sorulur. Dönen: (sorunlar, doğrulanamayan).
pub fn telefon_imzalarini_denetle(p: &Value, kayit: &dyn CihazKaydi) -> (Vec<String>, usize) {
    let mut sorun = Vec::new();
    let mut bilinmiyor = 0;
    let gs: Option<GunSonuKaydi> = serde_json::from_value(p["gun_sonu"].clone()).ok();
    for i in p["imzalar"].as_array().into_iter().flatten() {
        if i["kanal"] != "telefon" {
            continue;
        }
        let sira = i["sira"].as_i64().unwrap_or(0);
        let kisi = i["kisi"].as_str().unwrap_or("");
        let cihaz = i["cihaz"].as_str().unwrap_or("");
        match (i["ozet"].as_str(), &gs) {
            (Some(o), Some(g)) if o == ozet_sha256(g) => {}
            (Some(_), Some(_)) => sorun.push(format!("{sira}. imza: özet gün sonuyla tutmuyor")),
            _ => sorun.push(format!("{sira}. imza: özet yok")),
        }
        if cihaz.is_empty() {
            sorun.push(format!("{sira}. imza: cihaz kimliği yok"));
            continue;
        }
        match kayit.denetle(cihaz, kisi) {
            CihazDurumu::Gecerli => {}
            CihazDurumu::Bilinmiyor => bilinmiyor += 1,
            CihazDurumu::Iptal => sorun.push(format!("{sira}. imza: cihaz iptal edilmiş")),
            CihazDurumu::BaskaKisi => sorun.push(format!("{sira}. imza: cihaz başka kişiye bağlı")),
        }
    }
    (sorun, bilinmiyor)
}

/// Merkez klasöründeki paketleri okur (bozuk ya da tanınmayan dosyalar atlanır, sayısı döner). TAA Merkez cihaz
/// kaydı bağlanana kadar telefon imzaları `MerkezKaydiYok` ile denetlenir.
pub fn gelenleri_oku(klasor: &Path) -> (Vec<GelenPaket>, usize) {
    gelenleri_oku_denetle(klasor, &MerkezKaydiYok)
}

/// `gelenleri_oku`, TAA Merkez cihaz kaydıyla.
pub fn gelenleri_oku_denetle(klasor: &Path, kayit: &dyn CihazKaydi) -> (Vec<GelenPaket>, usize) {
    let mut v = Vec::new();
    let mut bozuk = 0;
    let Ok(d) = std::fs::read_dir(klasor) else {
        return (v, 0);
    };
    for e in d.filter_map(Result::ok) {
        let ad = e.file_name().to_string_lossy().to_string();
        if !ad_gecerli(&ad) {
            continue;
        }
        let alinma = e
            .metadata()
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let p: Option<Value> = std::fs::read(e.path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        match p.filter(|p| p["tur"] == "kasa.gunsonu@1") {
            Some(p) => v.push({
                let (telefon_imza_sorunlari, telefon_imza_dogrulanamadi) =
                    telefon_imzalarini_denetle(&p, kayit);
                GelenPaket {
                    sube: p["sube"].as_str().unwrap_or("").to_string(),
                    tarih: p["tarih"].as_str().unwrap_or("").to_string(),
                    // Mağazada imzalanıp kuyruğa alındığı an esas alınır (ağ kesintisi "geç" saydırmaz);
                    // alan yoksa dosyanın merkeze yazıldığı an.
                    alinma_sn: p["olusturma"].as_i64().unwrap_or(alinma),
                    kasa_ciro_uyusmuyor: p["gun_sonu"]["karsilastirma"]["kasa_ciro_uyusmuyor"]
                        .as_bool()
                        .unwrap_or(false),
                    ikinci_imza_atmadi: p["ikinci_imza_durumu"] == IKINCI_IMZA_ATMADI,
                    ikinci_imza_gerekcesi: p["ikinci_imza_gerekcesi"].as_str().map(str::to_string),
                    telefon_imza_sorunlari,
                    telefon_imza_dogrulanamadi,
                }
            }),
            None => bozuk += 1,
        }
    }
    (v, bozuk)
}

/// Mağaza × gün matrisi. Gün sonu `tarih`'in ertesi günü değil, aynı günün `son_saat`'ine kadar beklenir;
/// sonra gelen "geç", hiç gelmeyen "eksik" sayılır. `simdi_sn` süre dolmadıysa "bekleniyor" der.
pub fn matris(
    gelenler: &[GelenPaket],
    subeler: &[String],
    tarihler: &[String],
    son_saat: &str,
    fark_dk: i32,
    simdi_sn: i64,
) -> Vec<MatrisSatiri> {
    subeler
        .iter()
        .map(|s| MatrisSatiri {
            sube: s.clone(),
            tek_imza_gun_sayisi: 0,
            tek_imza_uyarisi: false,
            hucreler: tarihler
                .iter()
                .map(|t| {
                    let p: Vec<&GelenPaket> = gelenler
                        .iter()
                        .filter(|g| &g.sube == s && &g.tarih == t)
                        .collect();
                    let son = zaman::yerel_an(t, son_saat, fark_dk).unwrap_or(i64::MAX);
                    let durum = if p.is_empty() {
                        if simdi_sn <= son {
                            MatrisDurumu::Bekleniyor
                        } else {
                            MatrisDurumu::Eksik
                        }
                    } else if p.iter().map(|g| g.alinma_sn).min().unwrap_or(0) <= son {
                        MatrisDurumu::Geldi
                    } else {
                        MatrisDurumu::Gec
                    };
                    MatrisHucre {
                        tarih: t.clone(),
                        durum,
                        kasa_ciro_uyusmuyor: p.iter().any(|g| g.kasa_ciro_uyusmuyor),
                        cekmece_sayisi: p.len(),
                        ikinci_imza_atmadi: p.iter().any(|g| g.ikinci_imza_atmadi),
                        ikinci_imza_gerekceleri: p
                            .iter()
                            .filter_map(|g| g.ikinci_imza_gerekcesi.clone())
                            .collect(),
                        telefon_imza_sorunlari: p
                            .iter()
                            .flat_map(|g| g.telefon_imza_sorunlari.clone())
                            .collect(),
                        telefon_imza_dogrulanamadi: p
                            .iter()
                            .map(|g| g.telefon_imza_dogrulanamadi)
                            .sum(),
                    }
                })
                .collect(),
        })
        .collect()
}

/// Sık tek imzalı gönderim uyarısı: her şube için `bitis` dahil son `pencere_gun` günde ikinci imzasız gönderilen
/// gün sayısı; `esik`e ulaşırsa uyarı. Ayar: `KasaAyar.tek_imza_uyarisi`.
pub fn tek_imza_uyarisi_uygula(
    satirlar: &mut [MatrisSatiri],
    gelenler: &[GelenPaket],
    bitis: &str,
    pencere_gun: u32,
    esik: u32,
) {
    let Some(bas) = zaman::gun_ekle(bitis, 1 - i64::from(pencere_gun.max(1))) else {
        return;
    };
    for s in satirlar.iter_mut() {
        let mut gunler: Vec<&str> = gelenler
            .iter()
            .filter(|g| g.sube == s.sube && g.ikinci_imza_atmadi)
            .filter(|g| g.tarih.as_str() >= bas.as_str() && g.tarih.as_str() <= bitis)
            .map(|g| g.tarih.as_str())
            .collect();
        gunler.sort();
        gunler.dedup();
        s.tek_imza_gun_sayisi = gunler.len();
        s.tek_imza_uyarisi = esik > 0 && gunler.len() >= esik as usize;
    }
}

// ---------------------------------------------------------------- Excel

fn evet(b: bool) -> Hucre {
    Hucre::Metin(if b { "Evet" } else { "Hayır" }.into())
}

/// Gün sonları, masraflar ve sayımlardan xlsx üretir (üç sayfa).
pub fn excel(
    sube: &str,
    gun_sonlari: &[GunSonuKaydi],
    masraflar: &[MasrafGorunum],
    sayimlar: &[SayimKaydi],
    imzalar: &BTreeMap<String, Vec<ImzaKaydi>>,
    gerekceler: &BTreeMap<String, String>,
) -> Vec<u8> {
    let mut b: Vec<Hucre> = [
        "Şube",
        "Tarih",
        "Çekmece",
        "Kayıt",
        "Giren",
        "Sayılan nakit",
    ]
    .iter()
    .map(|s| Hucre::from(*s))
    .collect();
    b.extend(ODEME_TURLERI.iter().map(|(_, a)| Hucre::from(*a)));
    for s in [
        "Döviz (TL)",
        "Ciro toplamı",
        "Z raporu toplamı",
        "POS toplamı",
        "Nebim toplamı",
        "Ertesi güne devreden",
        "Fark var",
        "Kasa ile ciro uyuşmuyor",
        "Kapanış listesi",
        "1. imza (Kasa Sorumlusu)",
        "2. imza (Mağaza Sorumlusu / Vekili)",
        "Not",
        "Düzeltir",
    ] {
        b.push(s.into());
    }
    let mut gs = vec![b];
    for g in gun_sonlari {
        let mut r: Vec<Hucre> = vec![
            sube.into(),
            g.tarih.clone().into(),
            g.cekmece.clone().into(),
            g.id.clone().into(),
            g.kisi.clone().into(),
            Hucre::Kurus(g.nakit_sayilan_kurus),
        ];
        r.extend(
            ODEME_TURLERI
                .iter()
                .map(|(k, _)| Hucre::Kurus(g.ciro.get(*k).copied().unwrap_or(0))),
        );
        r.push(Hucre::Kurus(g.doviz.iter().map(|d| d.tl_kurus()).sum()));
        r.push(Hucre::Kurus(g.karsilastirma.ciro_toplam_kurus));
        r.push(Hucre::Kurus(
            g.noktalar.iter().map(|n| n.z_toplam_kurus).sum(),
        ));
        r.push(Hucre::Kurus(
            g.noktalar.iter().flat_map(|n| n.pos.values()).sum(),
        ));
        r.push(Hucre::Kurus(g.nebim_toplam_kurus));
        r.push(Hucre::Kurus(g.devreden_kurus));
        r.push(evet(g.karsilastirma.fark_var));
        r.push(evet(g.karsilastirma.kasa_ciro_uyusmuyor));
        r.push(
            format!(
                "{}/{}",
                g.kontrol.iter().filter(|x| **x).count(),
                KAPANIS_LISTESI.len()
            )
            .into(),
        );
        let im = imzalar.get(&g.id).map(Vec::as_slice).unwrap_or(&[]);
        for n in 0..2 {
            r.push(
                match im.get(n) {
                    Some(i) => format!(
                        "{} ({})",
                        i.kisi,
                        match i.kanal {
                            ImzaKanali::Telefon => "telefon",
                            ImzaKanali::Ekran => "ekran",
                        }
                    ),
                    None if n == 1 => match gerekceler.get(&g.id) {
                        Some(x) => format!("{IKINCI_IMZA_ATMADI} (gerekçe: {x})"),
                        None => IKINCI_IMZA_ATMADI.to_string(),
                    },
                    None => "imza atmadı".to_string(),
                }
                .into(),
            );
        }
        r.push(g.not.clone().into());
        r.push(g.duzeltir.clone().unwrap_or_default().into());
        gs.push(r);
    }
    let mut ms = vec![[
        "Tarih",
        "Çekmece",
        "Kayıt",
        "Giren",
        "Tür",
        "Tutar",
        "Açıklama",
        "Onay düzeyi",
        "Durum",
        "Karar veren",
    ]
    .iter()
    .map(|s| Hucre::from(*s))
    .collect::<Vec<_>>()];
    for m in masraflar {
        ms.push(vec![
            m.kayit.tarih.clone().into(),
            m.kayit.cekmece.clone().into(),
            m.kayit.id.clone().into(),
            m.kayit.kisi.clone().into(),
            m.kayit.tur.clone().into(),
            Hucre::Kurus(m.kayit.tutar_kurus),
            m.kayit.aciklama.clone().into(),
            serde_json::to_value(m.kayit.onay_duzeyi)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default()
                .into(),
            serde_json::to_value(m.durum)
                .ok()
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or_default()
                .into(),
            m.karar_veren.clone().unwrap_or_default().into(),
        ]);
    }
    let mut ss = vec![[
        "Tarih",
        "Çekmece",
        "Kayıt",
        "Tür",
        "Sayan",
        "İkinci kişi / devralan",
        "Sayılan",
        "Beklenen",
        "Fark",
        "Limit aşıldı",
        "Not",
    ]
    .iter()
    .map(|s| Hucre::from(*s))
    .collect::<Vec<_>>()];
    for s in sayimlar {
        ss.push(vec![
            s.tarih.clone().into(),
            s.cekmece.clone().into(),
            s.id.clone().into(),
            match s.tur {
                crate::gunsonu::SayimTuru::Acilis => "Açılış",
                crate::gunsonu::SayimTuru::Ara => "Ara sayım",
                crate::gunsonu::SayimTuru::Devir => "Vardiya devri",
            }
            .into(),
            s.kisi.clone().into(),
            s.ikinci_kisi.clone().unwrap_or_default().into(),
            Hucre::Kurus(s.sayilan_kurus),
            s.beklenen_kurus.map(Hucre::Kurus).unwrap_or(Hucre::Bos),
            s.fark_kurus.map(Hucre::Kurus).unwrap_or(Hucre::Bos),
            evet(s.limit_asildi),
            s.not.clone().into(),
        ]);
    }
    crate::xlsx::xlsx_uret(&[
        Sayfa {
            ad: "Gün sonu".into(),
            satirlar: gs,
            ..Default::default()
        },
        Sayfa {
            ad: "Masraf".into(),
            satirlar: ms,
            ..Default::default()
        },
        Sayfa {
            ad: "Sayımlar".into(),
            satirlar: ss,
            ..Default::default()
        },
    ])
}
