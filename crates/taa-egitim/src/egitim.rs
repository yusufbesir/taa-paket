//! Eğitim durumu: kişiye atama, "izledim" ve kaldığı saniye, yalnız yöneticiye açık izleme özeti.
//! Kişi verisi tutulmaz: yalnız rehberden doğrulanan kişi kimliği saklanır. Silme ucu yoktur (atama iptali kayıt bırakır).
//! Her değişiklik önce günlüğe eklenir, ardından belleğe uygulanır; günlük yazılamazsa bellek değişmez.
//! Sahip: Yusuf. yazan: [Claude#egitim 2026-11-26]

use crate::gunluk::{Gunluk, GunlukSatiri, Olay};
use crate::katalog::{Ders, Katalog};
use crate::rehber::RehberOkur;
use crate::{gecersiz, Hata};
use std::collections::{BTreeMap, BTreeSet};
use taa_rehber_model::model::kimlik_gecerli;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durum {
    /// Atandı, henüz izlenmedi.
    Atandi,
    /// Kaldığı saniye var, bitmedi.
    Basladi,
    Izledi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Ilerleme {
    surum: u32,
    saniye: u64,
    izlendi: bool,
}

/// Bir kişinin kendi listesindeki satır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KisiDersi {
    pub ders_id: String,
    pub atandi: bool,
    pub durum: Durum,
    /// Devam noktası (saniye); izlendiyse ya da başlanmadıysa 0.
    pub saniye: u64,
    /// Kayıtlı izleme eski sürüme ait (ders güncellendi); geçersiz sayıldı.
    pub eski_surum: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OzetSatir {
    pub kisi_id: String,
    pub ders_id: String,
    pub durum: Durum,
    pub yuzde: u8,
    pub eski_surum: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DersOzet {
    pub ders_id: String,
    /// Etkin atamalı kişi sayısı.
    pub atanan: usize,
    pub basladi: usize,
    pub izledi: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ozet {
    pub satirlar: Vec<OzetSatir>,
    pub dersler: Vec<DersOzet>,
}

#[derive(Debug, Clone, Default)]
pub struct Egitim {
    pub katalog: Katalog,
    /// (kişi, ders) -> etkin mi. İptal kayıt bırakır.
    atamalar: BTreeMap<(String, String), bool>,
    ilerleme: BTreeMap<(String, String), Ilerleme>,
}

fn kimlik(alan: &'static str, s: &str) -> Result<(), Hata> {
    if kimlik_gecerli(s) {
        Ok(())
    } else {
        Err(gecersiz(alan, "[a-z0-9][a-z0-9_-]{0,63} olmalı"))
    }
}

impl Egitim {
    pub fn yeni(katalog: Katalog) -> Self {
        Self {
            katalog,
            ..Self::default()
        }
    }

    /// Günlükten durumu yeniden kurar. Katalogda olmayan derse ait satırlar atlanır; atlanan sayısı döner.
    /// Rehber yeniden denetlenmez (kayıt o gün geçerliydi).
    pub fn gunlukten(katalog: Katalog, satirlar: &[GunlukSatiri]) -> (Self, usize) {
        let mut e = Self::yeni(katalog);
        let mut atlanan = 0;
        for s in satirlar {
            let ders = match &s.olay {
                Olay::Atama { ders, .. }
                | Olay::AtamaIptal { ders, .. }
                | Olay::Konum { ders, .. }
                | Olay::Izledi { ders, .. } => ders,
            };
            if e.katalog.al(ders).is_none() {
                atlanan += 1;
                continue;
            }
            e.uygula(&s.olay);
        }
        (e, atlanan)
    }

    fn uygula(&mut self, olay: &Olay) -> bool {
        match olay {
            Olay::Atama { kisi, ders, .. } => {
                let e = self
                    .atamalar
                    .entry((kisi.clone(), ders.clone()))
                    .or_insert(false);
                !std::mem::replace(e, true)
            }
            Olay::AtamaIptal { kisi, ders, .. } => {
                match self.atamalar.get_mut(&(kisi.clone(), ders.clone())) {
                    Some(e) if *e => {
                        *e = false;
                        true
                    }
                    _ => false,
                }
            }
            Olay::Konum {
                kisi,
                ders,
                surum,
                saniye,
            } => {
                let a = self
                    .ilerleme
                    .entry((kisi.clone(), ders.clone()))
                    .or_insert(Ilerleme {
                        surum: *surum,
                        saniye: 0,
                        izlendi: false,
                    });
                if a.surum != *surum {
                    *a = Ilerleme {
                        surum: *surum,
                        saniye: *saniye,
                        izlendi: false,
                    };
                    return true;
                }
                let degisti = a.saniye != *saniye;
                a.saniye = *saniye;
                degisti
            }
            Olay::Izledi { kisi, ders, surum } => {
                let a = self
                    .ilerleme
                    .entry((kisi.clone(), ders.clone()))
                    .or_insert(Ilerleme {
                        surum: *surum,
                        saniye: 0,
                        izlendi: false,
                    });
                if a.surum != *surum {
                    *a = Ilerleme {
                        surum: *surum,
                        saniye: 0,
                        izlendi: true,
                    };
                    return true;
                }
                !std::mem::replace(&mut a.izlendi, true)
            }
        }
    }

    /// Olayı denetler, değiştirecekse günlüğe ekler, sonra uygular. Dönüş: durum değişti mi (değişmeyen olay günlüğe yazılmaz).
    pub fn isle(
        &mut self,
        olay: Olay,
        rehber: &dyn RehberOkur,
        zaman: &str,
        gunluk: Option<&mut Gunluk>,
    ) -> Result<bool, Hata> {
        let ders_id = match &olay {
            Olay::Atama { kisi, ders, atayan } | Olay::AtamaIptal { kisi, ders, atayan } => {
                kimlik("kisi", kisi)?;
                kimlik("atayan", atayan)?;
                if !rehber.kisi_etkin(kisi) {
                    return Err(Hata::KisiYok(kisi.clone()));
                }
                if !rehber.kisi_etkin(atayan) {
                    return Err(Hata::KisiYok(atayan.clone()));
                }
                ders
            }
            Olay::Konum { kisi, ders, .. } | Olay::Izledi { kisi, ders, .. } => {
                kimlik("kisi", kisi)?;
                if !rehber.kisi_etkin(kisi) {
                    return Err(Hata::KisiYok(kisi.clone()));
                }
                ders
            }
        };
        let d: &Ders = self
            .katalog
            .al(ders_id)
            .ok_or_else(|| Hata::DersYok(ders_id.clone()))?;
        match &olay {
            Olay::Konum { surum, saniye, .. } => {
                if *surum != d.surum {
                    return Err(Hata::SurumEski {
                        ders: d.id.clone(),
                        verilen: *surum,
                        guncel: d.surum,
                    });
                }
                if *saniye > d.sure_sn {
                    return Err(Hata::KonumAsimi {
                        ders: d.id.clone(),
                        saniye: *saniye,
                        sure_sn: d.sure_sn,
                    });
                }
            }
            Olay::Izledi { surum, .. } if *surum != d.surum => {
                return Err(Hata::SurumEski {
                    ders: d.id.clone(),
                    verilen: *surum,
                    guncel: d.surum,
                });
            }
            _ => {}
        }
        // Değişiklik olacak mı? Bellek kopyasında denenir; günlük başarısızsa gerçek durum dokunulmaz.
        let mut deneme = self.clone();
        if !deneme.uygula(&olay) {
            return Ok(false);
        }
        if let Some(g) = gunluk {
            g.ekle(zaman, olay)?;
        }
        *self = deneme;
        Ok(true)
    }

    pub fn ata(
        &mut self,
        kisi: &str,
        ders: &str,
        atayan: &str,
        rehber: &dyn RehberOkur,
        zaman: &str,
        gunluk: Option<&mut Gunluk>,
    ) -> Result<bool, Hata> {
        let o = Olay::Atama {
            kisi: kisi.into(),
            ders: ders.into(),
            atayan: atayan.into(),
        };
        self.isle(o, rehber, zaman, gunluk)
    }

    pub fn atamayi_iptal_et(
        &mut self,
        kisi: &str,
        ders: &str,
        atayan: &str,
        rehber: &dyn RehberOkur,
        zaman: &str,
        gunluk: Option<&mut Gunluk>,
    ) -> Result<bool, Hata> {
        let o = Olay::AtamaIptal {
            kisi: kisi.into(),
            ders: ders.into(),
            atayan: atayan.into(),
        };
        self.isle(o, rehber, zaman, gunluk)
    }

    /// Kişinin rehberdeki etkin görev kodlarından en az biri dersin görev etiketlerinde olan tüm dersleri atar
    /// (etiketsiz dersler atanmaz). Dönüş: yeni atanan ders kimlikleri.
    pub fn gorevden_ata(
        &mut self,
        kisi: &str,
        atayan: &str,
        rehber: &dyn RehberOkur,
        zaman: &str,
        mut gunluk: Option<&mut Gunluk>,
    ) -> Result<Vec<String>, Hata> {
        kimlik("kisi", kisi)?;
        if !rehber.kisi_etkin(kisi) {
            return Err(Hata::KisiYok(kisi.into()));
        }
        let roller = rehber.gorev_kodlari(kisi);
        let adaylar: Vec<String> = self
            .katalog
            .hepsi()
            .into_iter()
            .filter(|d| d.gorev_etiketleri.iter().any(|e| roller.contains(e)))
            .map(|d| d.id.clone())
            .collect();
        let mut yeni = Vec::new();
        for d in adaylar {
            if self.ata(kisi, &d, atayan, rehber, zaman, gunluk.as_deref_mut())? {
                yeni.push(d);
            }
        }
        Ok(yeni)
    }

    /// Kaldığı saniyeyi kaydeder (devam noktası).
    pub fn konum_kaydet(
        &mut self,
        kisi: &str,
        ders: &str,
        saniye: u64,
        rehber: &dyn RehberOkur,
        zaman: &str,
        gunluk: Option<&mut Gunluk>,
    ) -> Result<bool, Hata> {
        let surum = self
            .katalog
            .al(ders)
            .ok_or_else(|| Hata::DersYok(ders.into()))?
            .surum;
        let o = Olay::Konum {
            kisi: kisi.into(),
            ders: ders.into(),
            surum,
            saniye,
        };
        self.isle(o, rehber, zaman, gunluk)
    }

    /// "İzledim" işareti (dersin güncel sürümü için).
    pub fn izledim(
        &mut self,
        kisi: &str,
        ders: &str,
        rehber: &dyn RehberOkur,
        zaman: &str,
        gunluk: Option<&mut Gunluk>,
    ) -> Result<bool, Hata> {
        let surum = self
            .katalog
            .al(ders)
            .ok_or_else(|| Hata::DersYok(ders.into()))?
            .surum;
        let o = Olay::Izledi {
            kisi: kisi.into(),
            ders: ders.into(),
            surum,
        };
        self.isle(o, rehber, zaman, gunluk)
    }

    fn guncel(&self, kisi: &str, ders: &Ders) -> Option<&Ilerleme> {
        self.ilerleme
            .get(&(kisi.to_string(), ders.id.clone()))
            .filter(|a| a.surum == ders.surum)
    }

    /// Oynatıcının kaldığı yerden devam saniyesi; başlanmadıysa, izlendiyse ya da ders sürümü değiştiyse 0.
    pub fn devam_noktasi(&self, kisi: &str, ders: &str) -> u64 {
        let Some(d) = self.katalog.al(ders) else {
            return 0;
        };
        self.guncel(kisi, d)
            .filter(|a| !a.izlendi)
            .map_or(0, |a| a.saniye)
    }

    pub fn atandi_mi(&self, kisi: &str, ders: &str) -> bool {
        self.atamalar
            .get(&(kisi.to_string(), ders.to_string()))
            .copied()
            .unwrap_or(false)
    }

    fn durum(&self, kisi: &str, d: &Ders) -> (Durum, bool) {
        let kayit = self.ilerleme.get(&(kisi.to_string(), d.id.clone()));
        match kayit {
            Some(a) if a.surum == d.surum => {
                let durum = if a.izlendi {
                    Durum::Izledi
                } else if a.saniye > 0 {
                    Durum::Basladi
                } else {
                    Durum::Atandi
                };
                (durum, false)
            }
            Some(_) => (Durum::Atandi, true),
            None => (Durum::Atandi, false),
        }
    }

    /// Kişinin kendi listesi: etkin atamaları ve ilerlemesi olan dersler (kendi verisi; yönetici bayrağı gerekmez).
    pub fn kisi_dersleri(&self, kisi: &str) -> Vec<KisiDersi> {
        let mut ids: BTreeSet<&str> = BTreeSet::new();
        for ((k, d), etkin) in &self.atamalar {
            if k == kisi && *etkin {
                ids.insert(d);
            }
        }
        for (k, d) in self.ilerleme.keys() {
            if k == kisi {
                ids.insert(d);
            }
        }
        ids.into_iter()
            .filter_map(|id| self.katalog.al(id))
            .map(|d| {
                let (durum, eski) = self.durum(kisi, d);
                KisiDersi {
                    ders_id: d.id.clone(),
                    atandi: self.atandi_mi(kisi, &d.id),
                    durum,
                    saniye: self.devam_noktasi(kisi, &d.id),
                    eski_surum: eski,
                }
            })
            .collect()
    }

    /// İzleme özeti: yalnız `yonetici == true` ile; aksi halde `YoneticiGerekli`. Çıktıda yalnız kişi kimliği vardır.
    pub fn ozet(&self, yonetici: bool) -> Result<Ozet, Hata> {
        if !yonetici {
            return Err(Hata::YoneticiGerekli);
        }
        let mut cift: BTreeSet<(&str, &str)> = BTreeSet::new();
        for ((k, d), etkin) in &self.atamalar {
            if *etkin {
                cift.insert((k, d));
            }
        }
        for (k, d) in self.ilerleme.keys() {
            cift.insert((k, d));
        }
        let mut o = Ozet::default();
        let mut ders_say: BTreeMap<&str, DersOzet> = BTreeMap::new();
        for (k, did) in cift {
            let Some(d) = self.katalog.al(did) else {
                continue;
            };
            let (durum, eski) = self.durum(k, d);
            let yuzde = match (durum, self.guncel(k, d)) {
                (Durum::Izledi, _) => 100,
                (_, Some(a)) => (a.saniye.min(d.sure_sn) * 100 / d.sure_sn) as u8,
                _ => 0,
            };
            let s = ders_say.entry(did).or_insert_with(|| DersOzet {
                ders_id: did.to_string(),
                atanan: 0,
                basladi: 0,
                izledi: 0,
            });
            if self.atandi_mi(k, did) {
                s.atanan += 1;
            }
            match durum {
                Durum::Basladi => s.basladi += 1,
                Durum::Izledi => s.izledi += 1,
                Durum::Atandi => {}
            }
            o.satirlar.push(OzetSatir {
                kisi_id: k.to_string(),
                ders_id: did.to_string(),
                durum,
                yuzde,
                eski_surum: eski,
            });
        }
        o.dersler = ders_say.into_values().collect();
        Ok(o)
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::katalog::KONU_KASA;
    use crate::rehber::RehberVeri;
    use std::collections::HashMap;
    use taa_rehber_model::model::{
        EtkinPasif, GorevTur, GorevlendirmeKaydi, KisiDurum, KisiKaydi, Veri,
    };

    const Z: &str = "2026-11-26T09:00:00Z";

    #[derive(Default)]
    struct Sahte {
        kisiler: HashMap<String, Vec<String>>,
    }
    impl Sahte {
        fn kisi(mut self, id: &str, roller: &[&str]) -> Self {
            self.kisiler
                .insert(id.into(), roller.iter().map(|s| s.to_string()).collect());
            self
        }
    }
    impl RehberOkur for Sahte {
        fn kisi_etkin(&self, id: &str) -> bool {
            self.kisiler.contains_key(id)
        }
        fn gorev_kodlari(&self, id: &str) -> Vec<String> {
            self.kisiler.get(id).cloned().unwrap_or_default()
        }
    }

    fn ders(id: &str, konu: &str, et: &[&str], sure: u64) -> Ders {
        Ders {
            id: id.into(),
            baslik: format!("Ders {id}"),
            konu: konu.into(),
            gorev_etiketleri: et.iter().map(|s| s.to_string()).collect(),
            sure_sn: sure,
            surum: 1,
        }
    }

    fn kur() -> Egitim {
        let mut k = Katalog::yeni();
        k.ekle(ders("kasa-acilis", KONU_KASA, &["kasa-sorumlusu"], 600))
            .unwrap();
        k.ekle(ders("kasa-gun-sonu", KONU_KASA, &["kasa-sorumlusu"], 1000))
            .unwrap();
        k.ekle(ders("iade", "satis", &["satis-danismani"], 300))
            .unwrap();
        Egitim::yeni(k)
    }

    fn rehber() -> Sahte {
        Sahte::default()
            .kisi("ayse", &["kasa-sorumlusu"])
            .kisi("mehmet", &["satis-danismani"])
            .kisi("yonetici1", &[])
    }

    #[test]
    fn atama_ve_tekrar_atama_degistirmez() {
        let (mut e, r) = (kur(), rehber());
        assert!(e
            .ata("ayse", "kasa-acilis", "yonetici1", &r, Z, None)
            .unwrap());
        assert!(!e
            .ata("ayse", "kasa-acilis", "yonetici1", &r, Z, None)
            .unwrap());
        assert!(e.atandi_mi("ayse", "kasa-acilis"));
        assert!(!e.atandi_mi("ayse", "iade"));
    }

    #[test]
    fn rehberde_olmayan_kisi_ve_ders_reddedilir() {
        let (mut e, r) = (kur(), rehber());
        assert_eq!(
            e.ata("yok", "iade", "yonetici1", &r, Z, None),
            Err(Hata::KisiYok("yok".into()))
        );
        assert_eq!(
            e.ata("ayse", "iade", "yok", &r, Z, None),
            Err(Hata::KisiYok("yok".into()))
        );
        assert_eq!(
            e.ata("ayse", "yok", "yonetici1", &r, Z, None),
            Err(Hata::DersYok("yok".into()))
        );
        assert!(matches!(
            e.ata("Ayşe!", "iade", "yonetici1", &r, Z, None),
            Err(Hata::Gecersiz { .. })
        ));
        assert_eq!(
            e.konum_kaydet("yok", "iade", 5, &r, Z, None),
            Err(Hata::KisiYok("yok".into()))
        );
    }

    #[test]
    fn gorevden_atama_etiketle_eslesenleri_atar() {
        let (mut e, r) = (kur(), rehber());
        let yeni = e.gorevden_ata("ayse", "yonetici1", &r, Z, None).unwrap();
        assert_eq!(yeni, ["kasa-acilis", "kasa-gun-sonu"]);
        assert!(e
            .gorevden_ata("ayse", "yonetici1", &r, Z, None)
            .unwrap()
            .is_empty());
        assert_eq!(
            e.gorevden_ata("mehmet", "yonetici1", &r, Z, None).unwrap(),
            ["iade"]
        );
        assert!(e
            .gorevden_ata("yonetici1", "yonetici1", &r, Z, None)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn atama_iptali_kayit_birakir_ve_yeniden_atanabilir() {
        let (mut e, r) = (kur(), rehber());
        e.ata("ayse", "iade", "yonetici1", &r, Z, None).unwrap();
        e.konum_kaydet("ayse", "iade", 30, &r, Z, None).unwrap();
        assert!(e
            .atamayi_iptal_et("ayse", "iade", "yonetici1", &r, Z, None)
            .unwrap());
        assert!(!e
            .atamayi_iptal_et("ayse", "iade", "yonetici1", &r, Z, None)
            .unwrap());
        assert!(!e.atandi_mi("ayse", "iade"));
        // ilerleme silinmedi
        assert_eq!(e.devam_noktasi("ayse", "iade"), 30);
        let l = e.kisi_dersleri("ayse");
        assert_eq!(l.len(), 1);
        assert!(!l[0].atandi);
        assert!(e.ata("ayse", "iade", "yonetici1", &r, Z, None).unwrap());
    }

    #[test]
    fn konum_devam_noktasi_olur() {
        let (mut e, r) = (kur(), rehber());
        assert_eq!(e.devam_noktasi("ayse", "kasa-acilis"), 0);
        assert!(e
            .konum_kaydet("ayse", "kasa-acilis", 125, &r, Z, None)
            .unwrap());
        assert_eq!(e.devam_noktasi("ayse", "kasa-acilis"), 125);
        assert!(!e
            .konum_kaydet("ayse", "kasa-acilis", 125, &r, Z, None)
            .unwrap());
        e.konum_kaydet("ayse", "kasa-acilis", 40, &r, Z, None)
            .unwrap(); // geri sarma
        assert_eq!(e.devam_noktasi("ayse", "kasa-acilis"), 40);
        assert_eq!(e.devam_noktasi("mehmet", "kasa-acilis"), 0);
    }

    #[test]
    fn konum_sureyi_asamaz() {
        let (mut e, r) = (kur(), rehber());
        assert!(e.konum_kaydet("ayse", "iade", 300, &r, Z, None).is_ok());
        assert!(matches!(
            e.konum_kaydet("ayse", "iade", 301, &r, Z, None),
            Err(Hata::KonumAsimi { .. })
        ));
        assert_eq!(e.devam_noktasi("ayse", "iade"), 300);
    }

    #[test]
    fn izledim_isareti_devami_sifirlar() {
        let (mut e, r) = (kur(), rehber());
        e.konum_kaydet("ayse", "iade", 100, &r, Z, None).unwrap();
        assert!(e.izledim("ayse", "iade", &r, Z, None).unwrap());
        assert!(!e.izledim("ayse", "iade", &r, Z, None).unwrap());
        assert_eq!(e.devam_noktasi("ayse", "iade"), 0);
        let l = e.kisi_dersleri("ayse");
        assert_eq!(l[0].durum, Durum::Izledi);
        // izlendikten sonra yeniden izleme izlendi bayrağını silmez
        e.konum_kaydet("ayse", "iade", 10, &r, Z, None).unwrap();
        assert_eq!(e.kisi_dersleri("ayse")[0].durum, Durum::Izledi);
    }

    #[test]
    fn yeni_surum_eski_izlemeyi_gecersiz_kilar() {
        let (mut e, r) = (kur(), rehber());
        e.izledim("ayse", "iade", &r, Z, None).unwrap();
        e.konum_kaydet("mehmet", "iade", 50, &r, Z, None).unwrap();
        let mut v2 = ders("iade", "satis", &["satis-danismani"], 400);
        v2.surum = 2;
        e.katalog.yeni_surum(v2).unwrap();
        assert_eq!(e.devam_noktasi("mehmet", "iade"), 0);
        let l = e.kisi_dersleri("ayse");
        assert_eq!(l[0].durum, Durum::Atandi);
        assert!(l[0].eski_surum);
        // eski sürüme bildirim reddedilir
        let eski = Olay::Izledi {
            kisi: "ayse".into(),
            ders: "iade".into(),
            surum: 1,
        };
        assert!(matches!(
            e.isle(eski, &r, Z, None),
            Err(Hata::SurumEski {
                verilen: 1,
                guncel: 2,
                ..
            })
        ));
        // güncel sürümü izleyince yeniden geçerli
        e.izledim("ayse", "iade", &r, Z, None).unwrap();
        let l = e.kisi_dersleri("ayse");
        assert_eq!((l[0].durum, l[0].eski_surum), (Durum::Izledi, false));
    }

    #[test]
    fn ozet_yalniz_yoneticiye() {
        let (mut e, r) = (kur(), rehber());
        e.ata("ayse", "iade", "yonetici1", &r, Z, None).unwrap();
        assert_eq!(e.ozet(false), Err(Hata::YoneticiGerekli));
        assert!(e.ozet(true).is_ok());
    }

    #[test]
    fn ozet_sayilari_ve_yuzde() {
        let (mut e, r) = (kur(), rehber());
        for k in ["ayse", "mehmet"] {
            e.ata(k, "kasa-acilis", "yonetici1", &r, Z, None).unwrap();
        }
        e.konum_kaydet("ayse", "kasa-acilis", 300, &r, Z, None)
            .unwrap();
        e.izledim("mehmet", "kasa-acilis", &r, Z, None).unwrap();
        let o = e.ozet(true).unwrap();
        assert_eq!(o.satirlar.len(), 2);
        let a = o.satirlar.iter().find(|s| s.kisi_id == "ayse").unwrap();
        assert_eq!((a.durum, a.yuzde), (Durum::Basladi, 50));
        let m = o.satirlar.iter().find(|s| s.kisi_id == "mehmet").unwrap();
        assert_eq!((m.durum, m.yuzde), (Durum::Izledi, 100));
        assert_eq!(
            o.dersler,
            [DersOzet {
                ders_id: "kasa-acilis".into(),
                atanan: 2,
                basladi: 1,
                izledi: 1
            }]
        );
    }

    #[test]
    fn kisi_dersleri_atamayi_ve_ilerlemeyi_birlestirir() {
        let (mut e, r) = (kur(), rehber());
        e.ata("ayse", "kasa-acilis", "yonetici1", &r, Z, None)
            .unwrap();
        e.konum_kaydet("ayse", "iade", 60, &r, Z, None).unwrap(); // atanmamış ama izlemiş
        let l = e.kisi_dersleri("ayse");
        assert_eq!(l.len(), 2);
        let acilis = l.iter().find(|x| x.ders_id == "kasa-acilis").unwrap();
        assert!(acilis.atandi);
        assert_eq!(acilis.durum, Durum::Atandi);
        let iade = l.iter().find(|x| x.ders_id == "iade").unwrap();
        assert!(!iade.atandi);
        assert_eq!((iade.durum, iade.saniye), (Durum::Basladi, 60));
    }

    #[test]
    fn gunluk_yazilir_ve_durum_yeniden_kurulur() {
        let y = crate::gunluk::testler_gecici("egitim-yeniden");
        let (mut e, r) = (kur(), rehber());
        {
            let mut g = Gunluk::ac(&y).unwrap();
            e.ata("ayse", "kasa-acilis", "yonetici1", &r, Z, Some(&mut g))
                .unwrap();
            e.konum_kaydet("ayse", "kasa-acilis", 200, &r, Z, Some(&mut g))
                .unwrap();
            e.konum_kaydet("ayse", "kasa-acilis", 200, &r, Z, Some(&mut g))
                .unwrap(); // değişmedi: yazılmaz
            e.izledim("mehmet", "iade", &r, Z, Some(&mut g)).unwrap();
            e.atamayi_iptal_et("ayse", "kasa-acilis", "yonetici1", &r, Z, Some(&mut g))
                .unwrap();
            e.ata("ayse", "kasa-gun-sonu", "yonetici1", &r, Z, Some(&mut g))
                .unwrap();
        }
        let (satirlar, bozuk) = Gunluk::oku(&y).unwrap();
        assert_eq!((satirlar.len(), bozuk), (5, 0));
        let (yeni, atlanan) = Egitim::gunlukten(kur().katalog, &satirlar);
        assert_eq!(atlanan, 0);
        assert_eq!(yeni.ozet(true).unwrap(), e.ozet(true).unwrap());
        assert_eq!(yeni.devam_noktasi("ayse", "kasa-acilis"), 200);
        assert!(!yeni.atandi_mi("ayse", "kasa-acilis"));
        assert!(yeni.atandi_mi("ayse", "kasa-gun-sonu"));
        let _ = std::fs::remove_file(&y);
    }

    #[test]
    fn reddedilen_olay_gunluge_yazilmaz() {
        let y = crate::gunluk::testler_gecici("egitim-red");
        let (mut e, r) = (kur(), rehber());
        let mut g = Gunluk::ac(&y).unwrap();
        assert!(e
            .konum_kaydet("ayse", "iade", 9999, &r, Z, Some(&mut g))
            .is_err());
        assert!(e
            .ata("yok", "iade", "yonetici1", &r, Z, Some(&mut g))
            .is_err());
        assert_eq!(std::fs::metadata(&y).unwrap().len(), 0);
        let _ = std::fs::remove_file(&y);
    }

    #[test]
    fn gunluk_yazilamazsa_bellek_degismez() {
        let y = crate::gunluk::testler_gecici("egitim-zaman");
        let (mut e, r) = (kur(), rehber());
        let mut g = Gunluk::ac(&y).unwrap();
        assert!(e
            .ata("ayse", "iade", "yonetici1", &r, "bozuk-zaman", Some(&mut g))
            .is_err());
        assert!(!e.atandi_mi("ayse", "iade"));
        let _ = std::fs::remove_file(&y);
    }

    #[test]
    fn gunlukten_katalogda_olmayan_ders_atlanir() {
        let satirlar = vec![GunlukSatiri {
            sira: 1,
            zaman: Z.into(),
            olay: Olay::Izledi {
                kisi: "ayse".into(),
                ders: "silinmis".into(),
                surum: 1,
            },
        }];
        let (e, atlanan) = Egitim::gunlukten(kur().katalog, &satirlar);
        assert_eq!(atlanan, 1);
        assert!(e.ozet(true).unwrap().satirlar.is_empty());
    }

    fn kisi(id: &str, durum: KisiDurum) -> KisiKaydi {
        KisiKaydi {
            id: id.into(),
            ad: "Ad".into(),
            eposta: format!("{id}@ornek.test"),
            durum,
            pasif_tarihi: None,
            surum: 1,
            guncelleme: Z.into(),
        }
    }

    fn gorev(
        id: &str,
        kisi: &str,
        rol: &str,
        bas: &str,
        bitis: Option<&str>,
        durum: EtkinPasif,
    ) -> GorevlendirmeKaydi {
        GorevlendirmeKaydi {
            id: id.into(),
            kisi_id: kisi.into(),
            birim_id: "b1".into(),
            rol: rol.into(),
            gorev: None,
            amir_id: None,
            tur: GorevTur::Asil,
            vekil_edilen_id: None,
            durum,
            baslangic: bas.into(),
            bitis: bitis.map(String::from),
            surum: 1,
            guncelleme: Z.into(),
        }
    }

    #[test]
    fn rehber_veri_baglayicisi() {
        let mut v = Veri::default();
        v.kisiler
            .insert("ayse".into(), kisi("ayse", KisiDurum::Etkin));
        v.kisiler
            .insert("eski".into(), kisi("eski", KisiDurum::Pasif));
        for g in [
            gorev(
                "g1",
                "ayse",
                "kasa-sorumlusu",
                "2026-01-01",
                None,
                EtkinPasif::Etkin,
            ),
            gorev(
                "g2",
                "ayse",
                "satis-danismani",
                "2025-01-01",
                Some("2025-12-31"),
                EtkinPasif::Etkin,
            ),
            gorev(
                "g3",
                "ayse",
                "magaza-muduru",
                "2027-01-01",
                None,
                EtkinPasif::Etkin,
            ),
            gorev(
                "g4",
                "ayse",
                "finans",
                "2026-01-01",
                None,
                EtkinPasif::Pasif,
            ),
        ] {
            v.gorevlendirmeler.insert(g.id.clone(), g);
        }
        let rv = RehberVeri {
            veri: &v,
            bugun: "2026-11-26",
        };
        assert!(rv.kisi_etkin("ayse"));
        assert!(!rv.kisi_etkin("eski"));
        assert!(!rv.kisi_etkin("yok"));
        assert_eq!(rv.gorev_kodlari("ayse"), ["kasa-sorumlusu"]);
        // uçtan uca: rehber görevinden atama
        let mut e = kur();
        v.kisiler
            .insert("yonetici1".into(), kisi("yonetici1", KisiDurum::Etkin));
        let rv = RehberVeri {
            veri: &v,
            bugun: "2026-11-26",
        };
        assert_eq!(
            e.gorevden_ata("ayse", "yonetici1", &rv, Z, None)
                .unwrap()
                .len(),
            2
        );
        assert!(matches!(
            e.ata("eski", "iade", "yonetici1", &rv, Z, None),
            Err(Hata::KisiYok(_))
        ));
    }
}
