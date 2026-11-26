//! taa-rehber alan modeli (docs/rehber-sozlesmesi.md §2-§6): kişi, birim, görevlendirme, rol türleri, doğrulayıcılar,
//! bellekte Depo (sürümlü yazma, silme yok), şube süzgeci ve günlük için e-posta maskeleme.
//! Gerçek yazma hizmeti (`rehber.yaz@1`) ve havuz sunucuda; burada yalnız kural çekirdeği ve bellek içi taklit vardır.
//! Sahip: Yusuf. yazan: [Claude#rehber-model 2026-11-26]

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Sözleşme §6: en çok kişi sayısı.
pub const EN_COK_KISI: usize = 2000;

// ---------------------------------------------------------------- hata

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModelHata {
    KimlikGecersiz(String),
    AlanGecersiz {
        alan: &'static str,
        neden: String,
    },
    EpostaGecersiz(String),
    EpostaTekrar(String),
    KisiSiniri,
    PasifTarihiGerekli(String),
    SubeKoduGerekli(String),
    UstBirimYok {
        birim: String,
        ust: String,
    },
    BirimDongu(String),
    KisiYok(String),
    BirimYok(String),
    RolYok(String),
    RolPasif(String),
    AmirYok {
        gorevlendirme: String,
        amir: String,
    },
    AmirDongu(String),
    GorevlendirmeTekrar(String),
    AsilCakisma(String),
    KimlikVar {
        tur: &'static str,
        id: String,
    },
    Bulunamadi {
        tur: &'static str,
        id: String,
    },
    /// Beklenen sürüm güncel değil (sözleşme §3, 409 `cakisma`).
    Cakisma {
        tur: &'static str,
        id: String,
        beklenen: u64,
        guncel: u64,
    },
}

impl std::fmt::Display for ModelHata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use ModelHata::*;
        match self {
            KimlikGecersiz(i) => write!(f, "geçersiz kimlik: {i}"),
            AlanGecersiz { alan, neden } => write!(f, "geçersiz alan {alan}: {neden}"),
            EpostaGecersiz(e) => write!(f, "geçersiz e-posta: {}", eposta_maskele(e)),
            EpostaTekrar(e) => write!(f, "tekrar eden e-posta: {}", eposta_maskele(e)),
            KisiSiniri => write!(f, "en çok {EN_COK_KISI} kişi tutulabilir"),
            PasifTarihiGerekli(i) => write!(f, "{i}: pasif kişi için pasif_tarihi gerekir"),
            SubeKoduGerekli(i) => write!(f, "{i}: sube/magaza biriminde sube_kodu gerekir"),
            UstBirimYok { birim, ust } => write!(f, "{birim}: üst birim yok ({ust})"),
            BirimDongu(i) => write!(f, "birim döngüsü: {i}"),
            KisiYok(i) => write!(f, "kişi yok: {i}"),
            BirimYok(i) => write!(f, "birim yok: {i}"),
            RolYok(i) => write!(f, "rol yok: {i}"),
            RolPasif(i) => write!(f, "rol pasif, yeni atamada seçilemez: {i}"),
            AmirYok {
                gorevlendirme,
                amir,
            } => write!(f, "{gorevlendirme}: amir kişi yok ({amir})"),
            AmirDongu(i) => write!(f, "amir döngüsü: {i}"),
            GorevlendirmeTekrar(i) => write!(f, "{i}: aynı kişi+birim+rol+başlangıç zaten var"),
            AsilCakisma(i) => write!(
                f,
                "{i}: kişinin aynı anda en çok bir etkin asıl görevlendirmesi olur"
            ),
            KimlikVar { tur, id } => write!(f, "{tur} kimliği zaten var: {id}"),
            Bulunamadi { tur, id } => write!(f, "{tur} bulunamadı: {id}"),
            Cakisma {
                tur,
                id,
                beklenen,
                guncel,
            } => {
                write!(
                    f,
                    "cakisma: {tur} {id} beklenen sürüm {beklenen}, güncel {guncel}"
                )
            }
        }
    }
}

impl std::error::Error for ModelHata {}

// ---------------------------------------------------------------- yardımcı denetimler

/// `[a-z0-9][a-z0-9_-]{0,63}`
pub fn kimlik_gecerli(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= 64
        && (b[0].is_ascii_lowercase() || b[0].is_ascii_digit())
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_' || *c == b'-')
}

/// `[a-z][a-z0-9_-]{0,31}`
pub fn rol_kodu_gecerli(s: &str) -> bool {
    let b = s.as_bytes();
    !b.is_empty()
        && b.len() <= 32
        && b[0].is_ascii_lowercase()
        && b.iter()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == b'_' || *c == b'-')
}

/// `YYYY-AA-GG` (gerçek takvim günü).
pub fn tarih_gecerli(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let sayi = |a: &str| {
        a.parse::<u32>()
            .ok()
            .filter(|_| a.bytes().all(|c| c.is_ascii_digit()))
    };
    let (Some(y), Some(a), Some(g)) = (sayi(&s[0..4]), sayi(&s[5..7]), sayi(&s[8..10])) else {
        return false;
    };
    let artik = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let gun = match a {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 => {
            if artik {
                29
            } else {
                28
            }
        }
        _ => return false,
    };
    y >= 1 && (1..=gun).contains(&g)
}

/// RFC 3339 UTC: `YYYY-AA-GGTSS:DD:ss[.kesir]Z`.
pub fn zaman_gecerli(s: &str) -> bool {
    let Some(govde) = s.strip_suffix('Z') else {
        return false;
    };
    let Some((t, saat)) = govde.split_once('T') else {
        return false;
    };
    if !tarih_gecerli(t) {
        return false;
    }
    let (ana, kesir) = match saat.split_once('.') {
        Some((a, k)) => (a, Some(k)),
        None => (saat, None),
    };
    if let Some(k) = kesir {
        if k.is_empty() || !k.bytes().all(|c| c.is_ascii_digit()) {
            return false;
        }
    }
    let p: Vec<&str> = ana.split(':').collect();
    p.len() == 3
        && p.iter()
            .all(|x| x.len() == 2 && x.bytes().all(|c| c.is_ascii_digit()))
        && p[0] < "24"
        && p[1] < "60"
        && p[2] < "61"
}

fn metin_ok(alan: &'static str, s: &str, en_cok: usize) -> Result<(), ModelHata> {
    let n = s.chars().count();
    if s.trim().is_empty() || n > en_cok || s != s.trim() {
        return Err(ModelHata::AlanGecersiz {
            alan,
            neden: format!("1-{en_cok} karakter, baş/son boşluksuz olmalı"),
        });
    }
    Ok(())
}

fn istege_bagli_ok(alan: &'static str, s: &Option<String>, en_cok: usize) -> Result<(), ModelHata> {
    match s {
        Some(v) if v.chars().count() > en_cok => Err(ModelHata::AlanGecersiz {
            alan,
            neden: format!("en çok {en_cok} karakter"),
        }),
        _ => Ok(()),
    }
}

fn tarih_ok(alan: &'static str, s: &str) -> Result<(), ModelHata> {
    if tarih_gecerli(s) {
        Ok(())
    } else {
        Err(ModelHata::AlanGecersiz {
            alan,
            neden: "YYYY-AA-GG olmalı".into(),
        })
    }
}

fn zaman_ok(s: &str) -> Result<(), ModelHata> {
    if zaman_gecerli(s) {
        Ok(())
    } else {
        Err(ModelHata::AlanGecersiz {
            alan: "guncelleme",
            neden: "RFC 3339 UTC olmalı".into(),
        })
    }
}

fn surum_ok(s: u64) -> Result<(), ModelHata> {
    if s >= 1 {
        Ok(())
    } else {
        Err(ModelHata::AlanGecersiz {
            alan: "surum",
            neden: "en az 1 olmalı".into(),
        })
    }
}

fn kimlik_ok(s: &str) -> Result<(), ModelHata> {
    if kimlik_gecerli(s) {
        Ok(())
    } else {
        Err(ModelHata::KimlikGecersiz(s.to_string()))
    }
}

// ---------------------------------------------------------------- günlük maskeleme (§6)

/// `ayse@alan.com` -> `a***@alan.com`. Biçimsiz değer tamamen `***` olur (sızıntı yok).
pub fn eposta_maskele(e: &str) -> String {
    match e.split_once('@') {
        Some((yerel, alan)) if !alan.is_empty() => match yerel.chars().next() {
            Some(c) => format!("{c}***@{alan}"),
            None => format!("***@{alan}"),
        },
        _ => "***".to_string(),
    }
}

/// Günlük satırı: ad yazılmaz, `id` yazılır, e-posta maskelidir.
pub fn gunluk_kisi(k: &KisiKaydi) -> String {
    format!("kisi id={} eposta={}", k.id, eposta_maskele(&k.eposta))
}

// ---------------------------------------------------------------- türler

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KisiDurum {
    Etkin,
    Pasif,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BirimTur {
    Sube,
    Magaza,
    Departman,
    Merkez,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BirimDurum {
    Etkin,
    Pasif,
    Arsiv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GorevTur {
    Asil,
    Gecici,
    Vekil,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EtkinPasif {
    Etkin,
    Pasif,
}

/// `rehber.kisi@1`: yalnız kimlik ve iletişim. Görev, rol, amir, telefon alanı yoktur; bilinmeyen alan reddedilir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KisiKaydi {
    pub id: String,
    pub ad: String,
    pub eposta: String,
    pub durum: KisiDurum,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pasif_tarihi: Option<String>,
    pub surum: u64,
    pub guncelleme: String,
}

/// `rehber.birim@1`
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BirimKaydi {
    pub id: String,
    pub ad: String,
    pub tur: BirimTur,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ust_birim_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sube_kodu: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ortak_eposta: Option<String>,
    pub durum: BirimDurum,
    pub surum: u64,
    pub guncelleme: String,
}

/// `rehber.gorevlendirme@1` ("kim nerede görevli")
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GorevlendirmeKaydi {
    pub id: String,
    pub kisi_id: String,
    pub birim_id: String,
    pub rol: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gorev: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amir_id: Option<String>,
    pub tur: GorevTur,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vekil_edilen_id: Option<String>,
    pub durum: EtkinPasif,
    pub baslangic: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bitis: Option<String>,
    pub surum: u64,
    pub guncelleme: String,
}

/// `rehber.rol@1` (iletişim rolü; erişim yetkisi DEĞİLDİR). Anahtar: `kod`. Sözleşmede `surum` alanı yoktur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RolKaydi {
    pub kod: String,
    pub ad: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aciklama: Option<String>,
    pub durum: EtkinPasif,
}

// ---------------------------------------------------------------- kayıt doğrulayıcıları

impl KisiKaydi {
    pub fn dogrula(&self) -> Result<(), ModelHata> {
        kimlik_ok(&self.id)?;
        metin_ok("ad", &self.ad, 120)?;
        if !crate::eposta_gecerli(&self.eposta) {
            return Err(ModelHata::EpostaGecersiz(self.eposta.clone()));
        }
        match (self.durum, &self.pasif_tarihi) {
            (KisiDurum::Pasif, None) => return Err(ModelHata::PasifTarihiGerekli(self.id.clone())),
            (KisiDurum::Pasif, Some(t)) => tarih_ok("pasif_tarihi", t)?,
            (KisiDurum::Etkin, Some(_)) => {
                return Err(ModelHata::AlanGecersiz {
                    alan: "pasif_tarihi",
                    neden: "etkin kişide olmaz".into(),
                })
            }
            (KisiDurum::Etkin, None) => {}
        }
        surum_ok(self.surum)?;
        zaman_ok(&self.guncelleme)
    }
}

impl BirimKaydi {
    pub fn dogrula(&self) -> Result<(), ModelHata> {
        kimlik_ok(&self.id)?;
        metin_ok("ad", &self.ad, 120)?;
        if let Some(u) = &self.ust_birim_id {
            kimlik_ok(u)?;
        }
        if matches!(self.tur, BirimTur::Sube | BirimTur::Magaza)
            && self
                .sube_kodu
                .as_deref()
                .map(str::trim)
                .unwrap_or("")
                .is_empty()
        {
            return Err(ModelHata::SubeKoduGerekli(self.id.clone()));
        }
        if let Some(e) = &self.ortak_eposta {
            if !crate::eposta_gecerli(e) {
                return Err(ModelHata::EpostaGecersiz(e.clone()));
            }
        }
        surum_ok(self.surum)?;
        zaman_ok(&self.guncelleme)
    }
}

impl GorevlendirmeKaydi {
    pub fn dogrula(&self) -> Result<(), ModelHata> {
        kimlik_ok(&self.id)?;
        kimlik_ok(&self.kisi_id)?;
        kimlik_ok(&self.birim_id)?;
        if !rol_kodu_gecerli(&self.rol) {
            return Err(ModelHata::AlanGecersiz {
                alan: "rol",
                neden: "[a-z][a-z0-9_-]{0,31} olmalı".into(),
            });
        }
        istege_bagli_ok("gorev", &self.gorev, 120)?;
        for k in [&self.amir_id, &self.vekil_edilen_id].into_iter().flatten() {
            kimlik_ok(k)?;
        }
        tarih_ok("baslangic", &self.baslangic)?;
        if let Some(b) = &self.bitis {
            tarih_ok("bitis", b)?;
            if b.as_str() < self.baslangic.as_str() {
                return Err(ModelHata::AlanGecersiz {
                    alan: "bitis",
                    neden: "başlangıçtan önce olamaz".into(),
                });
            }
        }
        surum_ok(self.surum)?;
        zaman_ok(&self.guncelleme)
    }

    /// Tarih aralığı bugünü kapsıyor ve durum etkin mi.
    fn aktif(&self, bugun: &str) -> bool {
        self.durum == EtkinPasif::Etkin
            && self.baslangic.as_str() <= bugun
            && self.bitis.as_deref().is_none_or(|b| b >= bugun)
    }

    /// İki satırın tarih aralıkları kesişiyor mu (kapalı aralık; boş bitis = sonsuz).
    fn kesisir(&self, o: &Self) -> bool {
        let a_bit_sonra = self
            .bitis
            .as_deref()
            .is_none_or(|b| b >= o.baslangic.as_str());
        let b_bit_sonra = o
            .bitis
            .as_deref()
            .is_none_or(|b| b >= self.baslangic.as_str());
        a_bit_sonra && b_bit_sonra
    }
}

impl RolKaydi {
    pub fn dogrula(&self) -> Result<(), ModelHata> {
        if !rol_kodu_gecerli(&self.kod) {
            return Err(ModelHata::AlanGecersiz {
                alan: "kod",
                neden: "[a-z][a-z0-9_-]{0,31} olmalı".into(),
            });
        }
        metin_ok("ad", &self.ad, 60)?;
        istege_bagli_ok("aciklama", &self.aciklama, 200)
    }
}

// ---------------------------------------------------------------- veri kümesi ve bütünlük

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Veri {
    pub kisiler: BTreeMap<String, KisiKaydi>,
    pub birimler: BTreeMap<String, BirimKaydi>,
    pub gorevlendirmeler: BTreeMap<String, GorevlendirmeKaydi>,
    pub roller: BTreeMap<String, RolKaydi>,
}

impl Veri {
    /// Tüm bütünlük hatalarını toplar (boş = temiz).
    pub fn hatalar(&self) -> Vec<ModelHata> {
        let mut h = Vec::new();
        let mut ekle = |r: Result<(), ModelHata>| {
            if let Err(e) = r {
                h.push(e)
            }
        };
        for k in self.kisiler.values() {
            ekle(k.dogrula());
        }
        for b in self.birimler.values() {
            ekle(b.dogrula());
        }
        for g in self.gorevlendirmeler.values() {
            ekle(g.dogrula());
        }
        for r in self.roller.values() {
            ekle(r.dogrula());
        }
        if self.kisiler.len() > EN_COK_KISI {
            h.push(ModelHata::KisiSiniri);
        }
        // e-posta benzersizliği: yalnız etkin kişiler arasında
        let mut gorulen = BTreeSet::new();
        for k in self
            .kisiler
            .values()
            .filter(|k| k.durum == KisiDurum::Etkin)
        {
            let e = k.eposta.to_lowercase();
            if !gorulen.insert(e.clone()) {
                h.push(ModelHata::EpostaTekrar(e));
            }
        }
        // birim: üst birim var mı, döngü yok
        for b in self.birimler.values() {
            if let Some(u) = &b.ust_birim_id {
                if !self.birimler.contains_key(u) {
                    h.push(ModelHata::UstBirimYok {
                        birim: b.id.clone(),
                        ust: u.clone(),
                    });
                }
            }
            let (mut cur, mut adim) = (b.ust_birim_id.as_deref(), 0);
            while let Some(c) = cur {
                if c == b.id {
                    h.push(ModelHata::BirimDongu(b.id.clone()));
                    break;
                }
                adim += 1;
                if adim > self.birimler.len() {
                    break; // başka bir döngüde; o birim kendi turunda raporlanır
                }
                cur = self.birimler.get(c).and_then(|x| x.ust_birim_id.as_deref());
            }
        }
        // görevlendirme: başvurular
        let mut anahtarlar = BTreeSet::new();
        for g in self.gorevlendirmeler.values() {
            if !self.kisiler.contains_key(&g.kisi_id) {
                h.push(ModelHata::KisiYok(g.kisi_id.clone()));
            }
            if !self.birimler.contains_key(&g.birim_id) {
                h.push(ModelHata::BirimYok(g.birim_id.clone()));
            }
            if !self.roller.contains_key(&g.rol) {
                h.push(ModelHata::RolYok(g.rol.clone()));
            }
            for (k, amir) in [(&g.amir_id, true), (&g.vekil_edilen_id, false)] {
                if let Some(k) = k {
                    if !self.kisiler.contains_key(k) {
                        h.push(if amir {
                            ModelHata::AmirYok {
                                gorevlendirme: g.id.clone(),
                                amir: k.clone(),
                            }
                        } else {
                            ModelHata::KisiYok(k.clone())
                        });
                    }
                }
            }
            if g.durum == EtkinPasif::Etkin
                && !anahtarlar.insert((
                    g.kisi_id.as_str(),
                    g.birim_id.as_str(),
                    g.rol.as_str(),
                    g.baslangic.as_str(),
                ))
            {
                h.push(ModelHata::GorevlendirmeTekrar(g.id.clone()));
            }
        }
        // bir kişiye aynı anda en çok bir etkin asıl görevlendirme
        let asiller: Vec<&GorevlendirmeKaydi> = self
            .gorevlendirmeler
            .values()
            .filter(|g| g.durum == EtkinPasif::Etkin && g.tur == GorevTur::Asil)
            .collect();
        for (i, a) in asiller.iter().enumerate() {
            if asiller[i + 1..]
                .iter()
                .any(|b| b.kisi_id == a.kisi_id && a.kesisir(b))
            {
                h.push(ModelHata::AsilCakisma(a.kisi_id.clone()));
            }
        }
        // amir döngüsü (kişi -> amir kenarları, yalnız etkin satırlar)
        let mut kenar: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        for g in self
            .gorevlendirmeler
            .values()
            .filter(|g| g.durum == EtkinPasif::Etkin)
        {
            if let Some(a) = &g.amir_id {
                kenar
                    .entry(g.kisi_id.as_str())
                    .or_default()
                    .insert(a.as_str());
            }
        }
        for &bas in kenar.keys() {
            let mut gez: BTreeSet<&str> = BTreeSet::new();
            let mut yigin: Vec<&str> = kenar[bas].iter().copied().collect();
            let mut dongu = false;
            while let Some(x) = yigin.pop() {
                if x == bas {
                    dongu = true;
                    break;
                }
                if gez.insert(x) {
                    if let Some(s) = kenar.get(x) {
                        yigin.extend(s.iter().copied());
                    }
                }
            }
            if dongu {
                h.push(ModelHata::AmirDongu(bas.to_string()));
            }
        }
        h
    }

    pub fn dogrula(&self) -> Result<(), ModelHata> {
        match self.hatalar().into_iter().next() {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
}

// ---------------------------------------------------------------- Depo (bellekte, sürümlü yazma, silme yok)

fn surum_denetle(tur: &'static str, id: &str, beklenen: u64, guncel: u64) -> Result<(), ModelHata> {
    if beklenen != guncel {
        return Err(ModelHata::Cakisma {
            tur,
            id: id.to_string(),
            beklenen,
            guncel,
        });
    }
    Ok(())
}

/// Bellekte taklit: her yazma önce kopyada uygulanır, bütünlük geçerse kabul edilir (ya hep ya hiç).
/// Silme ucu bilerek yoktur (§4): kişi/birim/rol `pasif`, görevlendirme `bitis`/`pasif`.
#[derive(Debug, Clone, Default)]
pub struct Depo {
    veri: Veri,
}

impl Depo {
    pub fn yeni() -> Self {
        Self::default()
    }

    pub fn veri(&self) -> &Veri {
        &self.veri
    }

    fn degistir<T>(
        &mut self,
        f: impl FnOnce(&mut Veri) -> Result<T, ModelHata>,
    ) -> Result<T, ModelHata> {
        let mut kopya = self.veri.clone();
        let sonuc = f(&mut kopya)?;
        kopya.dogrula()?;
        self.veri = kopya;
        Ok(sonuc)
    }

    // ---- rol (sürümsüz)
    pub fn rol_ekle(&mut self, r: RolKaydi) -> Result<(), ModelHata> {
        self.degistir(|v| {
            if v.roller.contains_key(&r.kod) {
                return Err(ModelHata::KimlikVar {
                    tur: "rehber.rol",
                    id: r.kod,
                });
            }
            v.roller.insert(r.kod.clone(), r);
            Ok(())
        })
    }

    pub fn rol_pasiflestir(&mut self, kod: &str) -> Result<(), ModelHata> {
        self.degistir(|v| {
            let r = v.roller.get_mut(kod).ok_or_else(|| ModelHata::Bulunamadi {
                tur: "rehber.rol",
                id: kod.into(),
            })?;
            r.durum = EtkinPasif::Pasif;
            Ok(())
        })
    }

    // ---- kişi
    /// Yeni kişi: sürüm 1, `guncelleme` = `zaman`; e-posta küçük harfe çevrilir. Dönen değer yeni sürümdür.
    pub fn kisi_ekle(&mut self, mut k: KisiKaydi, zaman: &str) -> Result<u64, ModelHata> {
        k.surum = 1;
        k.guncelleme = zaman.to_string();
        k.eposta = k.eposta.to_lowercase();
        self.degistir(|v| {
            if v.kisiler.contains_key(&k.id) {
                return Err(ModelHata::KimlikVar {
                    tur: "rehber.kisi",
                    id: k.id,
                });
            }
            v.kisiler.insert(k.id.clone(), k);
            Ok(1)
        })
    }

    /// Güncelleme: `beklenen` güncel sürümle aynı değilse `Cakisma`; başarıda sürüm bir artar.
    pub fn kisi_yaz(
        &mut self,
        mut k: KisiKaydi,
        beklenen: u64,
        zaman: &str,
    ) -> Result<u64, ModelHata> {
        k.eposta = k.eposta.to_lowercase();
        self.degistir(|v| {
            let mevcut = v.kisiler.get(&k.id).ok_or_else(|| ModelHata::Bulunamadi {
                tur: "rehber.kisi",
                id: k.id.clone(),
            })?;
            surum_denetle("rehber.kisi", &k.id, beklenen, mevcut.surum)?;
            k.surum = mevcut.surum + 1;
            k.guncelleme = zaman.to_string();
            let s = k.surum;
            v.kisiler.insert(k.id.clone(), k);
            Ok(s)
        })
    }

    /// Silme yerine pasifleştirme (§4).
    pub fn kisi_pasiflestir(
        &mut self,
        id: &str,
        beklenen: u64,
        tarih: &str,
        zaman: &str,
    ) -> Result<u64, ModelHata> {
        let mut k = self
            .veri
            .kisiler
            .get(id)
            .cloned()
            .ok_or_else(|| ModelHata::Bulunamadi {
                tur: "rehber.kisi",
                id: id.into(),
            })?;
        k.durum = KisiDurum::Pasif;
        k.pasif_tarihi = Some(tarih.to_string());
        self.kisi_yaz(k, beklenen, zaman)
    }

    // ---- birim
    pub fn birim_ekle(&mut self, mut b: BirimKaydi, zaman: &str) -> Result<u64, ModelHata> {
        b.surum = 1;
        b.guncelleme = zaman.to_string();
        self.degistir(|v| {
            if v.birimler.contains_key(&b.id) {
                return Err(ModelHata::KimlikVar {
                    tur: "rehber.birim",
                    id: b.id,
                });
            }
            v.birimler.insert(b.id.clone(), b);
            Ok(1)
        })
    }

    pub fn birim_yaz(
        &mut self,
        mut b: BirimKaydi,
        beklenen: u64,
        zaman: &str,
    ) -> Result<u64, ModelHata> {
        self.degistir(|v| {
            let mevcut = v.birimler.get(&b.id).ok_or_else(|| ModelHata::Bulunamadi {
                tur: "rehber.birim",
                id: b.id.clone(),
            })?;
            surum_denetle("rehber.birim", &b.id, beklenen, mevcut.surum)?;
            b.surum = mevcut.surum + 1;
            b.guncelleme = zaman.to_string();
            let s = b.surum;
            v.birimler.insert(b.id.clone(), b);
            Ok(s)
        })
    }

    /// Kapanan birim `pasif` ya da `arsiv` olur; silinmez.
    pub fn birim_pasiflestir(
        &mut self,
        id: &str,
        beklenen: u64,
        durum: BirimDurum,
        zaman: &str,
    ) -> Result<u64, ModelHata> {
        let mut b = self
            .veri
            .birimler
            .get(id)
            .cloned()
            .ok_or_else(|| ModelHata::Bulunamadi {
                tur: "rehber.birim",
                id: id.into(),
            })?;
        b.durum = durum;
        self.birim_yaz(b, beklenen, zaman)
    }

    // ---- görevlendirme
    /// Yeni satırda rol etkin olmalıdır (pasif rol yeni atamada seçilemez).
    pub fn gorevlendirme_ekle(
        &mut self,
        mut g: GorevlendirmeKaydi,
        zaman: &str,
    ) -> Result<u64, ModelHata> {
        g.surum = 1;
        g.guncelleme = zaman.to_string();
        self.degistir(|v| {
            if v.gorevlendirmeler.contains_key(&g.id) {
                return Err(ModelHata::KimlikVar {
                    tur: "rehber.gorevlendirme",
                    id: g.id,
                });
            }
            rol_etkin_mi(v, &g.rol)?;
            v.gorevlendirmeler.insert(g.id.clone(), g);
            Ok(1)
        })
    }

    pub fn gorevlendirme_yaz(
        &mut self,
        mut g: GorevlendirmeKaydi,
        beklenen: u64,
        zaman: &str,
    ) -> Result<u64, ModelHata> {
        self.degistir(|v| {
            let mevcut = v
                .gorevlendirmeler
                .get(&g.id)
                .ok_or_else(|| ModelHata::Bulunamadi {
                    tur: "rehber.gorevlendirme",
                    id: g.id.clone(),
                })?;
            surum_denetle("rehber.gorevlendirme", &g.id, beklenen, mevcut.surum)?;
            if mevcut.rol != g.rol {
                rol_etkin_mi(v, &g.rol)?;
            }
            g.surum = mevcut.surum + 1;
            g.guncelleme = zaman.to_string();
            let s = g.surum;
            v.gorevlendirmeler.insert(g.id.clone(), g);
            Ok(s)
        })
    }

    /// Görev bitince `bitis` dolar; satır geçmişte okunur kalır.
    pub fn gorevlendirme_bitir(
        &mut self,
        id: &str,
        beklenen: u64,
        bitis: &str,
        zaman: &str,
    ) -> Result<u64, ModelHata> {
        let mut g =
            self.veri
                .gorevlendirmeler
                .get(id)
                .cloned()
                .ok_or_else(|| ModelHata::Bulunamadi {
                    tur: "rehber.gorevlendirme",
                    id: id.into(),
                })?;
        g.bitis = Some(bitis.to_string());
        self.gorevlendirme_yaz(g, beklenen, zaman)
    }

    /// Yanlış girilen satır `pasif` olur (silinmez).
    pub fn gorevlendirme_pasiflestir(
        &mut self,
        id: &str,
        beklenen: u64,
        zaman: &str,
    ) -> Result<u64, ModelHata> {
        let mut g =
            self.veri
                .gorevlendirmeler
                .get(id)
                .cloned()
                .ok_or_else(|| ModelHata::Bulunamadi {
                    tur: "rehber.gorevlendirme",
                    id: id.into(),
                })?;
        g.durum = EtkinPasif::Pasif;
        self.gorevlendirme_yaz(g, beklenen, zaman)
    }

    /// Şube süzgeci (§5) uygulanmış görünüm.
    pub fn gorunum(&self, b: &Bakis) -> Gorunum {
        gorunum(&self.veri, b)
    }
}

fn rol_etkin_mi(v: &Veri, kod: &str) -> Result<(), ModelHata> {
    match v.roller.get(kod) {
        None => Err(ModelHata::RolYok(kod.into())),
        Some(r) if r.durum != EtkinPasif::Etkin => Err(ModelHata::RolPasif(kod.into())),
        Some(_) => Ok(()),
    }
}

// ---------------------------------------------------------------- şube süzgeci (§5)

/// Okuyanın kapsamı. Şube cihazdan türetilir; istemci beyanı buraya taşınmamalıdır.
#[derive(Debug, Clone)]
pub struct Bakis<'a> {
    /// Cihazın şubesi. `None` ya da boş = kapsam yok: yalnız merkez (§5 farkı, "boş kapsam herkes değil").
    pub sube_kodu: Option<&'a str>,
    /// `rehber.tum.oku` (merkez rolü): tüm liste, pasif kişiler dahil.
    pub tum_oku: bool,
    /// `rehber.amir.oku`: `amir_id` görünür.
    pub amir_oku: bool,
    /// Bugünün tarihi (`YYYY-AA-GG`); "etkin görevlendirme" bununla belirlenir.
    pub bugun: &'a str,
    /// "Mağaza müdürü/sorumlusu" sayılan rol kodları (docs/rehber-rolleri.toml, AÇIK-R7).
    pub mudur_rolleri: &'a BTreeSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GorunurGorev {
    /// Yalnız sınırlı görünümde değilse (şubenin kendi birimi ya da `tum_oku`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gorevlendirme_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub birim_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gorev: Option<String>,
    /// Yalnız `amir_oku` iken.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amir_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tur: Option<GorevTur>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub baslangic: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitis: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GorunurKisi {
    /// Sınırlı görünümde (merkez/departman, mağaza müdürü) kimlik verilmez.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub ad: String,
    pub eposta: String,
    pub gorevler: Vec<GorunurGorev>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GorunurBirim {
    pub id: String,
    pub ad: String,
    pub tur: BirimTur,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ortak_eposta: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct Gorunum {
    pub kisiler: Vec<GorunurKisi>,
    pub birimler: Vec<GorunurBirim>,
    /// `rehber.rol@1` kişisel değildir, tamamı görünür.
    pub roller: Vec<RolKaydi>,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Seviye {
    Mudur,
    Merkez,
    Tam,
}

pub fn gorunum(v: &Veri, b: &Bakis) -> Gorunum {
    let roller: Vec<RolKaydi> = v.roller.values().cloned().collect();
    let birim_gor = |x: &BirimKaydi| GorunurBirim {
        id: x.id.clone(),
        ad: x.ad.clone(),
        tur: x.tur,
        ortak_eposta: x.ortak_eposta.clone(),
    };
    let amir = |g: &GorevlendirmeKaydi| if b.amir_oku { g.amir_id.clone() } else { None };

    if b.tum_oku {
        let kisiler = v
            .kisiler
            .values()
            .map(|k| GorunurKisi {
                id: Some(k.id.clone()),
                ad: k.ad.clone(),
                eposta: k.eposta.clone(),
                gorevler: v
                    .gorevlendirmeler
                    .values()
                    .filter(|g| g.kisi_id == k.id)
                    .map(|g| tam_gorev(g, amir(g)))
                    .collect(),
            })
            .collect();
        return Gorunum {
            kisiler,
            birimler: v.birimler.values().map(birim_gor).collect(),
            roller,
        };
    }

    let sube = b.sube_kodu.map(str::trim).filter(|s| !s.is_empty());
    let mut birimler = Vec::new();
    for x in v.birimler.values().filter(|x| x.durum == BirimDurum::Etkin) {
        let gorunur = match (sube, x.tur) {
            (_, BirimTur::Merkez) => true,
            (Some(_), BirimTur::Departman | BirimTur::Sube | BirimTur::Magaza) => true,
            (None, _) => false,
        };
        if gorunur {
            birimler.push(birim_gor(x));
        }
    }

    // satır başına görünürlük seviyesi; kişi için en geniş seviye
    let mut toplam: BTreeMap<&str, Vec<(Seviye, &GorevlendirmeKaydi)>> = BTreeMap::new();
    for g in v.gorevlendirmeler.values().filter(|g| g.aktif(b.bugun)) {
        let (Some(k), Some(u)) = (v.kisiler.get(&g.kisi_id), v.birimler.get(&g.birim_id)) else {
            continue;
        };
        if k.durum != KisiDurum::Etkin || u.durum != BirimDurum::Etkin {
            continue;
        }
        let kovan = matches!(u.tur, BirimTur::Sube | BirimTur::Magaza);
        let seviye = match sube {
            Some(s) if kovan && u.sube_kodu.as_deref() == Some(s) => Some(Seviye::Tam),
            Some(_) if matches!(u.tur, BirimTur::Merkez | BirimTur::Departman) => {
                Some(Seviye::Merkez)
            }
            Some(_) if kovan && b.mudur_rolleri.contains(&g.rol) => Some(Seviye::Mudur),
            None if u.tur == BirimTur::Merkez => Some(Seviye::Merkez),
            _ => None,
        };
        if let Some(s) = seviye {
            toplam.entry(k.id.as_str()).or_default().push((s, g));
        }
    }
    let mut kisiler = Vec::new();
    for (kid, satirlar) in toplam {
        let k = &v.kisiler[kid];
        let tam_var = satirlar.iter().any(|(s, _)| *s == Seviye::Tam);
        let gorevler = satirlar
            .iter()
            .map(|(s, g)| match s {
                Seviye::Tam => tam_gorev(g, amir(g)),
                Seviye::Merkez => GorunurGorev {
                    gorevlendirme_id: None,
                    birim_id: Some(g.birim_id.clone()),
                    rol: Some(g.rol.clone()),
                    gorev: g.gorev.clone(),
                    amir_id: None,
                    tur: None,
                    baslangic: None,
                    bitis: None,
                },
                Seviye::Mudur => GorunurGorev {
                    gorevlendirme_id: None,
                    birim_id: None,
                    rol: None,
                    gorev: g.gorev.clone(),
                    amir_id: None,
                    tur: None,
                    baslangic: None,
                    bitis: None,
                },
            })
            .collect();
        kisiler.push(GorunurKisi {
            id: tam_var.then(|| k.id.clone()),
            ad: k.ad.clone(),
            eposta: k.eposta.clone(),
            gorevler,
        });
    }
    Gorunum {
        kisiler,
        birimler,
        roller,
    }
}

fn tam_gorev(g: &GorevlendirmeKaydi, amir: Option<String>) -> GorunurGorev {
    GorunurGorev {
        gorevlendirme_id: Some(g.id.clone()),
        birim_id: Some(g.birim_id.clone()),
        rol: Some(g.rol.clone()),
        gorev: g.gorev.clone(),
        amir_id: amir,
        tur: Some(g.tur),
        baslangic: Some(g.baslangic.clone()),
        bitis: g.bitis.clone(),
    }
}

// ---------------------------------------------------------------- testler

#[cfg(test)]
mod testler {
    use super::*;

    const Z: &str = "2026-11-26T10:00:00Z";
    const BUGUN: &str = "2026-11-26";

    fn kisi(id: &str, eposta: &str) -> KisiKaydi {
        KisiKaydi {
            id: id.into(),
            ad: format!("Ad {id}"),
            eposta: eposta.into(),
            durum: KisiDurum::Etkin,
            pasif_tarihi: None,
            surum: 1,
            guncelleme: Z.into(),
        }
    }

    fn birim(id: &str, tur: BirimTur, sube: Option<&str>) -> BirimKaydi {
        BirimKaydi {
            id: id.into(),
            ad: format!("Birim {id}"),
            tur,
            ust_birim_id: None,
            sube_kodu: sube.map(Into::into),
            ortak_eposta: Some(format!("{id}@ornek.test")),
            durum: BirimDurum::Etkin,
            surum: 1,
            guncelleme: Z.into(),
        }
    }

    fn gorev(id: &str, kisi: &str, birim: &str, rol: &str) -> GorevlendirmeKaydi {
        GorevlendirmeKaydi {
            id: id.into(),
            kisi_id: kisi.into(),
            birim_id: birim.into(),
            rol: rol.into(),
            gorev: Some("Unvan".into()),
            amir_id: None,
            tur: GorevTur::Asil,
            vekil_edilen_id: None,
            durum: EtkinPasif::Etkin,
            baslangic: "2026-01-01".into(),
            bitis: None,
            surum: 1,
            guncelleme: Z.into(),
        }
    }

    fn rol(kod: &str) -> RolKaydi {
        RolKaydi {
            kod: kod.into(),
            ad: kod.into(),
            aciklama: None,
            durum: EtkinPasif::Etkin,
        }
    }

    fn depo() -> Depo {
        let mut d = Depo::yeni();
        for r in ["mudur", "satis", "yonetici"] {
            d.rol_ekle(rol(r)).unwrap();
        }
        d.birim_ekle(birim("merkez1", BirimTur::Merkez, None), Z)
            .unwrap();
        d.birim_ekle(birim("muhasebe", BirimTur::Departman, None), Z)
            .unwrap();
        d.birim_ekle(birim("sube-a", BirimTur::Sube, Some("A01")), Z)
            .unwrap();
        d.birim_ekle(birim("sube-b", BirimTur::Sube, Some("B01")), Z)
            .unwrap();
        d.birim_ekle(birim("outlet-b", BirimTur::Magaza, Some("B01")), Z)
            .unwrap();
        d
    }

    /// Şube süzgeci senaryosu: A şubesinde ayse (satış), B şubesinde veli (satış) ve mudur-b (müdür),
    /// merkezde ali, muhasebede fatma.
    fn dolu() -> Depo {
        let mut d = depo();
        for (id, ep) in [
            ("ayse", "ayse@ornek.test"),
            ("veli", "veli@ornek.test"),
            ("murat", "murat@ornek.test"),
            ("ali", "ali@ornek.test"),
            ("fatma", "fatma@ornek.test"),
        ] {
            d.kisi_ekle(kisi(id, ep), Z).unwrap();
        }
        d.gorevlendirme_ekle(gorev("g1", "ayse", "sube-a", "satis"), Z)
            .unwrap();
        d.gorevlendirme_ekle(gorev("g2", "veli", "sube-b", "satis"), Z)
            .unwrap();
        d.gorevlendirme_ekle(gorev("g3", "murat", "outlet-b", "mudur"), Z)
            .unwrap();
        let mut g = gorev("g4", "ali", "merkez1", "yonetici");
        g.amir_id = Some("fatma".into());
        d.gorevlendirme_ekle(g, Z).unwrap();
        d.gorevlendirme_ekle(gorev("g5", "fatma", "muhasebe", "satis"), Z)
            .unwrap();
        d
    }

    fn bakis<'a>(sube: Option<&'a str>, mudur: &'a BTreeSet<String>) -> Bakis<'a> {
        Bakis {
            sube_kodu: sube,
            tum_oku: false,
            amir_oku: false,
            bugun: BUGUN,
            mudur_rolleri: mudur,
        }
    }

    fn mudurler() -> BTreeSet<String> {
        ["mudur".to_string()].into_iter().collect()
    }

    // ---- şema

    #[test]
    fn bilinmeyen_ve_yasak_alan_reddedilir() {
        let j = r#"{"id":"k1","ad":"A","eposta":"a@b.co","durum":"etkin","surum":1,"guncelleme":"2026-11-26T10:00:00Z"}"#;
        assert!(serde_json::from_str::<KisiKaydi>(j).is_ok());
        for yasak in ["telefon", "gorev", "roller", "amir_id", "dogum_tarihi"] {
            let j2 = j.replace("\"surum\":1", &format!("\"{yasak}\":\"x\",\"surum\":1"));
            assert!(
                serde_json::from_str::<KisiKaydi>(&j2).is_err(),
                "{yasak} reddedilmeli"
            );
        }
        // zorunlu alan eksik
        assert!(serde_json::from_str::<KisiKaydi>(r#"{"id":"k1"}"#).is_err());
        // bilinmeyen sabit değer
        assert!(serde_json::from_str::<KisiKaydi>(&j.replace("etkin", "silindi")).is_err());
    }

    #[test]
    fn diger_turler_bilinmeyen_alani_reddeder() {
        let b = r#"{"id":"b","ad":"B","tur":"magaza","sube_kodu":"A01","durum":"etkin","surum":1,"guncelleme":"2026-11-26T10:00:00Z","x":1}"#;
        assert!(serde_json::from_str::<BirimKaydi>(b).is_err());
        let r = r#"{"kod":"satis","ad":"Satış","durum":"etkin","yetki":"yonetici"}"#;
        assert!(serde_json::from_str::<RolKaydi>(r).is_err());
        let g = serde_json::to_string(&gorev("g", "k", "b", "satis")).unwrap();
        assert!(serde_json::from_str::<GorevlendirmeKaydi>(&g).is_ok());
        assert!(
            serde_json::from_str::<GorevlendirmeKaydi>(&g.replace("\"id\"", "\"x\":1,\"id\""))
                .is_err()
        );
    }

    #[test]
    fn json_gidis_donus() {
        let k = kisi("k1", "a@b.co");
        let s = serde_json::to_string(&k).unwrap();
        assert!(!s.contains("pasif_tarihi"));
        assert_eq!(serde_json::from_str::<KisiKaydi>(&s).unwrap(), k);
    }

    // ---- doğrulayıcılar

    #[test]
    fn kimlik_ve_tarih_ve_zaman_bicimi() {
        assert!(kimlik_gecerli("a1_b-2"));
        assert!(!kimlik_gecerli(""));
        assert!(!kimlik_gecerli("-a"));
        assert!(!kimlik_gecerli("A1"));
        assert!(!kimlik_gecerli(&"a".repeat(65)));
        assert!(kimlik_gecerli(&"a".repeat(64)));
        assert!(tarih_gecerli("2028-02-29"));
        assert!(!tarih_gecerli("2026-02-29"));
        assert!(!tarih_gecerli("2026-13-01"));
        assert!(!tarih_gecerli("2026-1-01"));
        assert!(zaman_gecerli("2026-11-26T10:00:00Z"));
        assert!(zaman_gecerli("2026-11-26T10:00:00.123Z"));
        assert!(!zaman_gecerli("2026-11-26 10:00:00Z"));
        assert!(!zaman_gecerli("2026-11-26T25:00:00Z"));
        assert!(!zaman_gecerli("2026-11-26T10:00:00+03:00"));
    }

    #[test]
    fn kisi_alanlari() {
        assert!(kisi("k1", "a@b.co").dogrula().is_ok());
        assert!(matches!(
            kisi("K1", "a@b.co").dogrula(),
            Err(ModelHata::KimlikGecersiz(_))
        ));
        assert!(matches!(
            kisi("k1", "bozuk").dogrula(),
            Err(ModelHata::EpostaGecersiz(_))
        ));
        let mut k = kisi("k1", "a@b.co");
        k.ad = "  ".into();
        assert!(k.dogrula().is_err());
        k = kisi("k1", "a@b.co");
        k.durum = KisiDurum::Pasif;
        assert!(matches!(k.dogrula(), Err(ModelHata::PasifTarihiGerekli(_))));
        k.pasif_tarihi = Some("2026-11-26".into());
        assert!(k.dogrula().is_ok());
        k.durum = KisiDurum::Etkin; // etkin kişide pasif_tarihi olmaz
        assert!(k.dogrula().is_err());
        k = kisi("k1", "a@b.co");
        k.surum = 0;
        assert!(k.dogrula().is_err());
    }

    #[test]
    fn birim_sube_kodu_zorunlu() {
        assert!(birim("b", BirimTur::Sube, None).dogrula().is_err());
        assert!(birim("b", BirimTur::Magaza, Some(" ")).dogrula().is_err());
        assert!(birim("b", BirimTur::Departman, None).dogrula().is_ok());
    }

    #[test]
    fn eposta_benzersiz_yalniz_etkinler_arasinda() {
        let mut d = Depo::yeni();
        d.kisi_ekle(kisi("k1", "Ayse@Ornek.test"), Z).unwrap();
        // büyük/küçük harf farkı aynı adres sayılır
        let e = d.kisi_ekle(kisi("k2", "ayse@ornek.test"), Z).unwrap_err();
        assert!(matches!(e, ModelHata::EpostaTekrar(_)));
        assert!(!d.veri().kisiler.contains_key("k2"));
        // pasifleşince adres yeniden kullanılabilir
        d.kisi_pasiflestir("k1", 1, "2026-11-26", Z).unwrap();
        d.kisi_ekle(kisi("k2", "ayse@ornek.test"), Z).unwrap();
    }

    #[test]
    fn kisi_siniri_2000() {
        let mut v = Veri::default();
        for i in 0..=EN_COK_KISI {
            let k = kisi(&format!("k{i}"), &format!("k{i}@ornek.test"));
            v.kisiler.insert(k.id.clone(), k);
        }
        assert!(v.hatalar().contains(&ModelHata::KisiSiniri));
        v.kisiler.remove("k0");
        assert!(!v.hatalar().contains(&ModelHata::KisiSiniri));
    }

    #[test]
    fn birim_dongusu_ve_yetim_ust() {
        let mut d = depo();
        let mut b = d.veri().birimler["sube-a"].clone();
        b.ust_birim_id = Some("sube-a".into()); // kendine
        assert!(matches!(
            d.birim_yaz(b, 1, Z),
            Err(ModelHata::BirimDongu(_))
        ));
        let mut a = d.veri().birimler["outlet-b"].clone();
        a.ust_birim_id = Some("sube-b".into());
        d.birim_yaz(a, 1, Z).unwrap();
        let mut c = d.veri().birimler["sube-b"].clone();
        c.ust_birim_id = Some("outlet-b".into()); // iki halkalı döngü
        assert!(matches!(
            d.birim_yaz(c, 1, Z),
            Err(ModelHata::BirimDongu(_))
        ));
        let mut y = d.veri().birimler["sube-a"].clone();
        y.ust_birim_id = Some("yok".into());
        assert!(matches!(
            d.birim_yaz(y, 1, Z),
            Err(ModelHata::UstBirimYok { .. })
        ));
    }

    #[test]
    fn amir_dongusu_ve_yetim_amir() {
        let mut d = dolu();
        // ali'nin amiri fatma; fatma'nın amirini ali yaparsak döngü
        let mut g = d.veri().gorevlendirmeler["g5"].clone();
        g.amir_id = Some("ali".into());
        assert!(matches!(
            d.gorevlendirme_yaz(g, 1, Z),
            Err(ModelHata::AmirDongu(_))
        ));
        let mut kendi = d.veri().gorevlendirmeler["g5"].clone();
        kendi.amir_id = Some("fatma".into());
        assert!(matches!(
            d.gorevlendirme_yaz(kendi, 1, Z),
            Err(ModelHata::AmirDongu(_))
        ));
        let mut yetim = d.veri().gorevlendirmeler["g5"].clone();
        yetim.amir_id = Some("hayalet".into());
        assert!(matches!(
            d.gorevlendirme_yaz(yetim, 1, Z),
            Err(ModelHata::AmirYok { .. })
        ));
        // reddedilen yazma durumu değiştirmedi
        assert_eq!(d.veri().gorevlendirmeler["g5"].surum, 1);
    }

    #[test]
    fn bir_etkin_asil_gecici_ve_vekil_birlikte_durabilir() {
        let mut d = dolu();
        let mut ikinci = gorev("g6", "ayse", "sube-b", "satis");
        assert!(matches!(
            d.gorevlendirme_ekle(ikinci.clone(), Z),
            Err(ModelHata::AsilCakisma(_))
        ));
        ikinci.tur = GorevTur::Gecici;
        ikinci.bitis = Some("2026-12-31".into());
        d.gorevlendirme_ekle(ikinci, Z).unwrap();
        let mut vekil = gorev("g7", "ayse", "sube-b", "mudur");
        vekil.tur = GorevTur::Vekil;
        vekil.vekil_edilen_id = Some("murat".into());
        d.gorevlendirme_ekle(vekil, Z).unwrap();
    }

    #[test]
    fn asil_devri_tarih_kesismezse_olur() {
        let mut d = dolu();
        d.gorevlendirme_bitir("g1", 1, "2026-06-30", Z).unwrap();
        let mut yeni = gorev("g8", "ayse", "sube-b", "satis");
        yeni.baslangic = "2026-07-01".into();
        d.gorevlendirme_ekle(yeni, Z).unwrap();
        // eskiye kesişen asıl hâlâ reddedilir
        let mut kesisen = gorev("g9", "ayse", "sube-a", "satis");
        kesisen.baslangic = "2026-09-01".into();
        assert!(matches!(
            d.gorevlendirme_ekle(kesisen, Z),
            Err(ModelHata::AsilCakisma(_))
        ));
    }

    #[test]
    fn gorevlendirme_basvurulari_ve_tekrar() {
        let mut d = dolu();
        assert!(matches!(
            d.gorevlendirme_ekle(gorev("x1", "yok", "sube-a", "satis"), Z),
            Err(ModelHata::KisiYok(_))
        ));
        assert!(matches!(
            d.gorevlendirme_ekle(gorev("x2", "ayse", "yok", "satis"), Z),
            Err(ModelHata::BirimYok(_))
        ));
        assert!(matches!(
            d.gorevlendirme_ekle(gorev("x3", "ayse", "sube-a", "bilinmez"), Z),
            Err(ModelHata::RolYok(_))
        ));
        let mut t = gorev("x4", "veli", "sube-b", "satis");
        t.tur = GorevTur::Gecici;
        assert!(matches!(
            d.gorevlendirme_ekle(t, Z),
            Err(ModelHata::GorevlendirmeTekrar(_))
        ));
        let mut ters = gorev("x5", "veli", "sube-b", "mudur");
        ters.bitis = Some("2025-12-31".into());
        assert!(d.gorevlendirme_ekle(ters, Z).is_err());
    }

    #[test]
    fn pasif_rol_yeni_atamada_secilemez_eski_kayitta_kalir() {
        let mut d = dolu();
        d.rol_pasiflestir("satis").unwrap();
        assert!(d.veri().dogrula().is_ok()); // eski kayıt bozulmadı
        let mut g = gorev("y1", "ali", "merkez1", "satis");
        g.tur = GorevTur::Gecici;
        assert!(matches!(
            d.gorevlendirme_ekle(g, Z),
            Err(ModelHata::RolPasif(_))
        ));
        assert!(matches!(
            d.rol_ekle(rol("satis")),
            Err(ModelHata::KimlikVar { .. })
        ));
    }

    // ---- Depo: sürüm ve silme yok

    #[test]
    fn surum_artar_eski_surum_cakisma() {
        let mut d = Depo::yeni();
        assert_eq!(d.kisi_ekle(kisi("k1", "a@b.co"), Z).unwrap(), 1);
        let mut k = d.veri().kisiler["k1"].clone();
        k.ad = "Yeni Ad".into();
        assert_eq!(d.kisi_yaz(k.clone(), 1, "2026-11-26T11:00:00Z").unwrap(), 2);
        assert_eq!(d.veri().kisiler["k1"].surum, 2);
        assert_eq!(d.veri().kisiler["k1"].guncelleme, "2026-11-26T11:00:00Z");
        // ikinci yönetici hâlâ eski sürümle yazarsa 409
        k.ad = "Başka Ad".into();
        let e = d.kisi_yaz(k, 1, Z).unwrap_err();
        assert_eq!(
            e,
            ModelHata::Cakisma {
                tur: "rehber.kisi",
                id: "k1".into(),
                beklenen: 1,
                guncel: 2
            }
        );
        assert_eq!(d.veri().kisiler["k1"].ad, "Yeni Ad");
    }

    #[test]
    fn cakisma_birim_ve_gorevlendirmede_de() {
        let mut d = dolu();
        let b = d.veri().birimler["sube-a"].clone();
        d.birim_yaz(b.clone(), 1, Z).unwrap();
        assert!(matches!(
            d.birim_yaz(b, 1, Z),
            Err(ModelHata::Cakisma { .. })
        ));
        let g = d.veri().gorevlendirmeler["g1"].clone();
        d.gorevlendirme_yaz(g.clone(), 1, Z).unwrap();
        assert!(matches!(
            d.gorevlendirme_yaz(g, 1, Z),
            Err(ModelHata::Cakisma { .. })
        ));
        assert!(matches!(
            d.gorevlendirme_bitir("g1", 1, "2026-12-01", Z),
            Err(ModelHata::Cakisma { .. })
        ));
    }

    #[test]
    fn var_olmayani_yazmak_ve_var_olani_eklemek() {
        let mut d = Depo::yeni();
        assert!(matches!(
            d.kisi_yaz(kisi("k1", "a@b.co"), 1, Z),
            Err(ModelHata::Bulunamadi { .. })
        ));
        d.kisi_ekle(kisi("k1", "a@b.co"), Z).unwrap();
        assert!(matches!(
            d.kisi_ekle(kisi("k1", "c@d.co"), Z),
            Err(ModelHata::KimlikVar { .. })
        ));
    }

    #[test]
    fn silme_yok_pasiflestirme_ve_bitis_gecmiste_okunur() {
        let mut d = dolu();
        let once = d.veri().kisiler.len();
        d.kisi_pasiflestir("ayse", 1, "2026-11-26", Z).unwrap();
        d.gorevlendirme_bitir("g2", 1, "2026-11-01", Z).unwrap();
        d.gorevlendirme_pasiflestir("g3", 1, Z).unwrap();
        d.birim_pasiflestir("outlet-b", 1, BirimDurum::Arsiv, Z)
            .unwrap();
        assert_eq!(d.veri().kisiler.len(), once);
        assert_eq!(d.veri().kisiler["ayse"].durum, KisiDurum::Pasif);
        assert_eq!(
            d.veri().gorevlendirmeler["g2"].bitis.as_deref(),
            Some("2026-11-01")
        );
        assert_eq!(d.veri().gorevlendirmeler["g3"].durum, EtkinPasif::Pasif);
        assert_eq!(d.veri().birimler["outlet-b"].durum, BirimDurum::Arsiv);
        // bitmiş görev geçmişte durur ama bugünkü süzgeçte görünmez
        let m = mudurler();
        let g = d.gorunum(&bakis(Some("B01"), &m));
        assert!(g.kisiler.iter().all(|k| k.eposta != "veli@ornek.test"));
    }

    // ---- şube süzgeci

    fn epostalar(g: &Gorunum) -> Vec<&str> {
        g.kisiler.iter().map(|k| k.eposta.as_str()).collect()
    }

    #[test]
    fn sube_pcsi_kendi_subesini_gorur_digerini_gormez() {
        let d = dolu();
        let m = mudurler();
        let g = d.gorunum(&bakis(Some("A01"), &m));
        let e = epostalar(&g);
        assert!(e.contains(&"ayse@ornek.test")); // kendi şubesi
        assert!(!e.contains(&"veli@ornek.test")); // B şubesinin sıradan kişisi
        let ayse = g
            .kisiler
            .iter()
            .find(|k| k.eposta == "ayse@ornek.test")
            .unwrap();
        assert_eq!(ayse.id.as_deref(), Some("ayse"));
        assert_eq!(ayse.gorevler[0].baslangic.as_deref(), Some("2026-01-01"));
    }

    #[test]
    fn merkez_departman_magaza_ortak_adresleri_ve_mudur() {
        let d = dolu();
        let m = mudurler();
        let g = d.gorunum(&bakis(Some("A01"), &m));
        // 2: merkez/departman kişileri yalnız ad, eposta, gorev, rol (kimlik yok)
        let ali = g
            .kisiler
            .iter()
            .find(|k| k.eposta == "ali@ornek.test")
            .unwrap();
        assert_eq!(ali.id, None);
        assert_eq!(ali.gorevler[0].rol.as_deref(), Some("yonetici"));
        assert_eq!(ali.gorevler[0].gorevlendirme_id, None);
        assert_eq!(ali.gorevler[0].baslangic, None);
        assert!(g.kisiler.iter().any(|k| k.eposta == "fatma@ornek.test"));
        // 3: tüm şube/mağaza/merkez/departman ortak adresleri
        let b: Vec<&str> = g.birimler.iter().map(|b| b.id.as_str()).collect();
        for id in ["merkez1", "muhasebe", "sube-a", "sube-b", "outlet-b"] {
            assert!(b.contains(&id), "{id}");
        }
        assert!(g.birimler.iter().all(|b| b.ortak_eposta.is_some()));
        // 4: başka şubenin mağaza müdürü yalnız ad, eposta, gorev
        let murat = g
            .kisiler
            .iter()
            .find(|k| k.eposta == "murat@ornek.test")
            .unwrap();
        assert_eq!(murat.id, None);
        assert_eq!(murat.gorevler[0].gorev.as_deref(), Some("Unvan"));
        assert_eq!(murat.gorevler[0].rol, None);
        assert_eq!(murat.gorevler[0].birim_id, None);
        // 5: roller tamamı
        assert_eq!(g.roller.len(), 3);
    }

    #[test]
    fn amir_id_yalniz_izinle_gorunur() {
        let d = dolu();
        let m = mudurler();
        // ali merkez birimde: sınırlı görünümde amir hiç yok
        let g = d.gorunum(&bakis(Some("A01"), &m));
        assert!(g
            .kisiler
            .iter()
            .flat_map(|k| &k.gorevler)
            .all(|x| x.amir_id.is_none()));
        // kendi şubesinde amirli satır: izin yokken yok, izinle var
        let mut d2 = d.clone();
        let mut x = d2.veri().gorevlendirmeler["g1"].clone();
        x.amir_id = Some("fatma".into());
        d2.gorevlendirme_yaz(x, 1, Z).unwrap();
        let yok = d2.gorunum(&bakis(Some("A01"), &m));
        assert!(yok
            .kisiler
            .iter()
            .flat_map(|k| &k.gorevler)
            .all(|x| x.amir_id.is_none()));
        let mut b = bakis(Some("A01"), &m);
        b.amir_oku = true;
        let var = d2.gorunum(&b);
        let ayse = var
            .kisiler
            .iter()
            .find(|k| k.eposta == "ayse@ornek.test")
            .unwrap();
        assert_eq!(ayse.gorevler[0].amir_id.as_deref(), Some("fatma"));
        let s = serde_json::to_string(&yok).unwrap();
        assert!(!s.contains("amir_id"));
    }

    #[test]
    fn bos_kapsam_yalniz_merkez() {
        let d = dolu();
        let m = mudurler();
        for sube in [None, Some(""), Some("  ")] {
            let g = d.gorunum(&bakis(sube, &m));
            assert_eq!(epostalar(&g), vec!["ali@ornek.test"], "{sube:?}");
            assert!(g.birimler.iter().all(|b| b.tur == BirimTur::Merkez));
            assert!(!g.birimler.is_empty());
        }
    }

    #[test]
    fn tum_oku_pasifleri_de_gorur_digerleri_gormez() {
        let mut d = dolu();
        d.kisi_pasiflestir("veli", 1, "2026-11-26", Z).unwrap();
        let m = mudurler();
        let g = d.gorunum(&bakis(Some("B01"), &m));
        assert!(!epostalar(&g).contains(&"veli@ornek.test"));
        let mut b = bakis(None, &m);
        b.tum_oku = true;
        let t = d.gorunum(&b);
        assert_eq!(t.kisiler.len(), 5);
        assert!(epostalar(&t).contains(&"veli@ornek.test"));
        assert_eq!(t.birimler.len(), 5);
    }

    #[test]
    fn magaza_kendi_subesinden_sayilir() {
        let d = dolu();
        let m = mudurler();
        // outlet-b birimi B01'e bağlı: B cihazı müdürü tam görür (kimlikle)
        let g = d.gorunum(&bakis(Some("B01"), &m));
        let murat = g
            .kisiler
            .iter()
            .find(|k| k.eposta == "murat@ornek.test")
            .unwrap();
        assert_eq!(murat.id.as_deref(), Some("murat"));
    }

    // ---- maskeleme

    #[test]
    fn eposta_maskeleme() {
        assert_eq!(
            eposta_maskele("ayse@terziademaltun.com"),
            "a***@terziademaltun.com"
        );
        assert_eq!(eposta_maskele("@alan.com"), "***@alan.com");
        assert_eq!(eposta_maskele("bozuk"), "***");
        assert_eq!(eposta_maskele("x@"), "***");
        assert_eq!(eposta_maskele("şule@alan.com"), "ş***@alan.com");
        let k = kisi("k1", "ayse@ornek.test");
        let s = gunluk_kisi(&k);
        assert_eq!(s, "kisi id=k1 eposta=a***@ornek.test");
        assert!(!s.contains("ayse") && !s.contains("Ad k1"));
    }

    #[test]
    fn hata_iletilerinde_acik_eposta_yok() {
        let e = ModelHata::EpostaTekrar("ayse@ornek.test".into()).to_string();
        assert!(!e.contains("ayse@"), "{e}");
        let e = ModelHata::EpostaGecersiz("ayse@ornek".into()).to_string();
        assert!(!e.contains("ayse@"), "{e}");
    }
}
