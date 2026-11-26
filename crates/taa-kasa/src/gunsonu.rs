// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! TAA Kasa gün sonu çekirdeği (ana plan §15, mağaza işleyişi planı §4.1 aşama 1: KS2a–KS10).
//!
//! Kavramlar:
//! - **Çekmece**: paranın durduğu yer. Nakit sayımı (açılış, ara sayım, devir, gün sonu) çekmece başınadır.
//! - **Kasa noktası**: bilgisayar + yazarkasa + POS cihazları. Z raporu ve POS gün sonu kasa noktası başınadır.
//!   Bir çekmeceye birden çok kasa noktası bağlanabilir (Esenyurt: 2 nokta, 1 çekmece). Mağaza toplamı
//!   çekmecelerden kendiliğinden toplanır.
//!
//! Kurallar (değişmez):
//! - Defterler yalnız eklemelidir (JSONL). **Silme yok**; düzeltme yalnız yönetici girişiyle, "iptal" kaydı ve
//!   (gerekirse) `duzeltir` alanı dolu yeni kayıtla yapılır. İz kalır.
//! - **Giren onaylayamaz**: ikinci kişi onayı ve masraf onayı kaydı giren kişiden başkası verir.
//! - Tek yazar TAA Kasa: `kasa.gunsonu@1` (bu defter) ve `kasa.masraf@1` (masraf defteri). Tutaa yalnız okur.
//! - Satış yapılmaz, yazarkasanın yerine geçilmez; tutarlar elle girilir, hiçbir satış sistemine bağlanılmaz.
//!
//! Alanlar Yusuf'un şubelerde kullanılan akşam Excel'i görülünce birebir uyarlanacaktır; bugünkü alanlar
//! planlardan alınmıştır (ödeme türleri, Z/POS/Nebim üçlü karşılaştırma, döviz, 7 maddelik kapanış listesi).

use crate::{sayim_toplam, zaman};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Gün sonu ve masraf özelliği (Ayarlar'dan kapatılabilir; kapalıyken yazılmaz, okunur).
pub const OZELLIK_GUNSONU: &str = "kasa-gunsonu";

/// Tutar üst sınırı (10 milyar TL); taşmayı ve yazım hatasını önler.
pub const TUTAR_EN_COK: i64 = 1_000_000_000_000;

/// Ödeme türleri (anahtar, ekranda görünen ad). Sıra ekranda ve Excel'de aynıdır.
pub const ODEME_TURLERI: [(&str, &str); 7] = [
    ("nakit", "Nakit"),
    ("ziraat_kart", "Ziraat kart"),
    ("iyzico_fiziksel", "İyzico fiziksel POS"),
    ("iyzico_sanal", "İyzico sanal POS"),
    ("qr", "QR ödeme"),
    ("mail_order", "Mail order"),
    ("havale", "Havale / EFT"),
];

/// Dövizler (kod, ad).
pub const DOVIZLER: [(&str, &str); 4] = [
    ("USD", "Dolar"),
    ("EUR", "Euro"),
    ("GBP", "Sterlin"),
    ("CHF", "İsviçre Frangı"),
];

/// Kapanış kontrol listesi (KS9, 7 madde).
pub const KAPANIS_LISTESI: [&str; 7] = [
    "Z raporu alındı",
    "Z raporu, POS ve satış sistemi karşılaştırıldı",
    "Çift doğrulama yapıldı (ikinci kişi)",
    "Kasa kilitlendi",
    "Kamera kaydı açık",
    "Masraf fişleri ayrıldı",
    "Ertesi gün bozukluğu hazırlandı",
];

// ---------------------------------------------------------------- kişi ve rol

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rol {
    Personel,
    Mudur,
    Yonetici,
}

impl Rol {
    pub fn coz(s: &str) -> Option<Rol> {
        match s {
            "personel" => Some(Rol::Personel),
            "mudur" => Some(Rol::Mudur),
            "yonetici" => Some(Rol::Yonetici),
            _ => None,
        }
    }
}

/// İşlemi yapan kişi. `dogrulandi = false`: ortak giriş (KM2) bağlanana kadar ad yalnız seçilmiştir, PIN yoktur.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kisi {
    pub ad: String,
    pub rol: Rol,
    #[serde(default)]
    pub dogrulandi: bool,
}

fn ayni_kisi(a: &str, b: &str) -> bool {
    a.trim().to_lowercase() == b.trim().to_lowercase()
}

// ---------------------------------------------------------------- ayarlar

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PosCihazi {
    pub id: String,
    pub ad: String,
    /// Bu POS'un gün sonu hangi ödeme türüyle karşılaştırılır (ör. `ziraat_kart`).
    pub odeme_turu: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KasaNoktasi {
    pub id: String,
    pub ad: String,
    #[serde(default)]
    pub poslar: Vec<PosCihazi>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cekmece {
    pub id: String,
    pub ad: String,
    pub noktalar: Vec<KasaNoktasi>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PersonelKaydi {
    pub ad: String,
    pub rol: Rol,
}

/// Mağaza ayarları (yönetici değiştirir). İlk değerler plandan; kesin değerleri Yusuf verir (§6 madde 4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct KasaAyar {
    pub surum: u32,
    pub sube: String,
    pub cekmeceler: Vec<Cekmece>,
    /// Gün içi kasa limiti (KS2b). Merkez için 2.500 TL; öteki mağazalarda yönetici girer.
    pub kasa_limiti_kurus: Option<i64>,
    /// Bu tutar ve üstü masraf merkez onayı ister (KS6; 2.000 TL).
    pub masraf_merkez_esigi_kurus: i64,
    /// Fark toleransı (mağaza bazlı; varsayılan 0).
    pub fark_toleransi_kurus: i64,
    pub masraf_turleri: Vec<String>,
    /// Gün sonu bu saatten sonra merkeze ulaşırsa "geç" sayılır (yerel saat, ertesi sabaha kadar).
    pub gonderim_son_saat: String,
    pub saat_farki_dk: i32,
    /// Ortak giriş (KM2) bağlanana kadar yer tutucu kişi listesi.
    pub personel: Vec<PersonelKaydi>,
    /// Sunucu kasa ucu (Ş3) gelene kadar paketlerin teslim edildiği ortak klasör (boşsa kuyrukta bekler).
    pub merkez_klasoru: Option<String>,
}

impl Default for KasaAyar {
    fn default() -> Self {
        KasaAyar {
            surum: 1,
            sube: String::new(),
            cekmeceler: vec![Cekmece {
                id: "c1".into(),
                ad: "Çekmece 1".into(),
                noktalar: vec![KasaNoktasi {
                    id: "k1".into(),
                    ad: "Kasa noktası 1".into(),
                    poslar: vec![
                        PosCihazi {
                            id: "ziraat".into(),
                            ad: "Ziraat POS".into(),
                            odeme_turu: "ziraat_kart".into(),
                        },
                        PosCihazi {
                            id: "iyzico".into(),
                            ad: "İyzico POS".into(),
                            odeme_turu: "iyzico_fiziksel".into(),
                        },
                    ],
                }],
            }],
            kasa_limiti_kurus: None,
            masraf_merkez_esigi_kurus: 200_000,
            fark_toleransi_kurus: 0,
            // Masraf türleri listesi Yusuf'tan beklenir (G6); yer tutucu.
            masraf_turleri: vec![
                "Kırtasiye".into(),
                "Temizlik".into(),
                "Kargo".into(),
                "Yemek".into(),
                "Diğer".into(),
            ],
            gonderim_son_saat: "23:00".into(),
            saat_farki_dk: zaman::TURKIYE_DK,
            personel: vec![],
            merkez_klasoru: None,
        }
    }
}

fn kimlik_gecerli(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

impl KasaAyar {
    pub fn dogrula(&self) -> Result<(), String> {
        if self.cekmeceler.is_empty() {
            return Err("en az bir çekmece gerekli".into());
        }
        let mut idler = Vec::new();
        for c in &self.cekmeceler {
            if !kimlik_gecerli(&c.id) {
                return Err(format!("çekmece kimliği geçersiz: {:?}", c.id));
            }
            if c.noktalar.is_empty() {
                return Err(format!("{} için en az bir kasa noktası gerekli", c.ad));
            }
            for n in &c.noktalar {
                if !kimlik_gecerli(&n.id) {
                    return Err(format!("kasa noktası kimliği geçersiz: {:?}", n.id));
                }
                for p in &n.poslar {
                    if !kimlik_gecerli(&p.id)
                        || !ODEME_TURLERI.iter().any(|(k, _)| *k == p.odeme_turu)
                    {
                        return Err(format!("POS tanımı geçersiz: {}", p.ad));
                    }
                }
                idler.push(n.id.clone());
            }
            idler.push(c.id.clone());
        }
        let mut s = idler.clone();
        s.sort();
        s.dedup();
        if s.len() != idler.len() {
            return Err("çekmece ve kasa noktası kimlikleri tekil olmalı".into());
        }
        if self.masraf_merkez_esigi_kurus <= 0 || self.fark_toleransi_kurus < 0 {
            return Err("eşik ve tolerans negatif olamaz".into());
        }
        if zaman::saat_coz(&self.gonderim_son_saat).is_none() {
            return Err("gönderim son saati SS:DD olmalı".into());
        }
        Ok(())
    }

    pub fn cekmece(&self, id: &str) -> Option<&Cekmece> {
        self.cekmeceler.iter().find(|c| c.id == id)
    }

    pub fn yukle(yol: &Path) -> Result<KasaAyar, String> {
        match std::fs::read(yol) {
            Ok(b) => serde_json::from_slice(&b).map_err(|e| format!("ayar dosyası bozuk: {e}")),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(KasaAyar::default()),
            Err(e) => Err(format!("ayar okunamadı: {e}")),
        }
    }

    pub fn kaydet(&self, yol: &Path) -> Result<(), String> {
        self.dogrula()?;
        if let Some(k) = yol.parent() {
            std::fs::create_dir_all(k).map_err(|e| e.to_string())?;
        }
        let b = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        taa_dosya::uzerine_yaz_atomik(yol, &b).map_err(|e| e.to_string())
    }
}

// ---------------------------------------------------------------- kayıtlar

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SayimTuru {
    Acilis,
    Ara,
    Devir,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SayimKaydi {
    pub id: String,
    pub tur: SayimTuru,
    pub tarih: String,
    pub cekmece: String,
    pub kisi: String,
    /// İkinci sayan (ara sayım) ya da devralan (devir).
    pub ikinci_kisi: Option<String>,
    pub adetler: BTreeMap<i64, u32>,
    pub sayilan_kurus: i64,
    /// Açılış: bir önceki gün sonunda ertesi güne devreden nakit. Ara/devir: o ana kadarki beklenen nakit.
    pub beklenen_kurus: Option<i64>,
    pub fark_kurus: Option<i64>,
    pub limit_asildi: bool,
    pub not: String,
    pub zaman: i64,
    pub duzeltir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DovizSatiri {
    pub kod: String,
    /// Yabancı para, kuruş (cent) cinsinden.
    pub tutar: i64,
    /// Günlük kur, 1 birim = kur_onbinde / 10000 TL.
    pub kur_onbinde: i64,
}

impl DovizSatiri {
    /// TL karşılığı (kuruş), yarım yukarı yuvarlanır.
    pub fn tl_kurus(&self) -> i64 {
        let c = self.tutar as i128 * self.kur_onbinde as i128;
        ((c + 5_000) / 10_000) as i64
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoktaGunSonu {
    pub nokta: String,
    pub z_toplam_kurus: i64,
    /// POS kimliği → gün sonu tutarı.
    pub pos: BTreeMap<String, i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FarkSatiri {
    pub ad: String,
    pub beklenen_kurus: i64,
    pub bulunan_kurus: i64,
    pub fark_kurus: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Karsilastirma {
    pub satirlar: Vec<FarkSatiri>,
    pub ciro_toplam_kurus: i64,
    /// Herhangi bir fark tolerans dışında.
    pub fark_var: bool,
    /// "Kasa ile ciro uyuşmuyor" uyarısı: nakit ya da Z ↔ ciro satırı tolerans dışında.
    pub kasa_ciro_uyusmuyor: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GunSonuKaydi {
    pub id: String,
    pub tarih: String,
    pub sube: String,
    pub cekmece: String,
    pub kisi: String,
    pub nakit_adetler: BTreeMap<i64, u32>,
    pub nakit_sayilan_kurus: i64,
    pub noktalar: Vec<NoktaGunSonu>,
    pub nebim_toplam_kurus: i64,
    pub ciro: BTreeMap<String, i64>,
    pub doviz: Vec<DovizSatiri>,
    /// Ertesi güne çekmecede bırakılan nakit (bozukluk); ertesi günün açılışında beklenen tutardır.
    pub devreden_kurus: i64,
    pub kontrol: Vec<bool>,
    pub not: String,
    pub karsilastirma: Karsilastirma,
    pub zaman: i64,
    pub duzeltir: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OnayTuru {
    /// Çift doğrulama: ikinci kişi aynı gün onaylar (giren olamaz).
    IkinciKisi,
    /// Ertesi gün müdür gözden geçirmesi (KS9).
    ErtesiGun,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "snake_case")]
pub enum GunKaydi {
    Sayim(SayimKaydi),
    GunSonu(GunSonuKaydi),
    Onay {
        id: String,
        kisi: String,
        onay: OnayTuru,
        zaman: i64,
    },
    Iptal {
        id: String,
        neden: String,
        kisi: String,
        zaman: i64,
    },
    Gonderim {
        id: String,
        dosya: String,
        imzali: bool,
        zaman: i64,
    },
}

// ---------------------------------------------------------------- masraf kayıtları (kasa.masraf@1)

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnayDuzeyi {
    /// Eşik altı: mağaza müdürü.
    Mudur,
    /// Eşik ve üstü: talep formu ve merkez onayı.
    Merkez,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasrafKaydi {
    pub id: String,
    pub tarih: String,
    pub cekmece: String,
    pub kisi: String,
    pub tur: String,
    pub tutar_kurus: i64,
    pub aciklama: String,
    /// İsteğe bağlı fiş fotoğrafı (veri kökü altındaki göreli dosya adı).
    pub foto: Option<String>,
    pub onay_duzeyi: OnayDuzeyi,
    pub zaman: i64,
    pub duzeltir: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "snake_case")]
pub enum MasrafKayit {
    Masraf(MasrafKaydi),
    Karar {
        id: String,
        kisi: String,
        onay: bool,
        not: String,
        zaman: i64,
    },
    Iptal {
        id: String,
        neden: String,
        kisi: String,
        zaman: i64,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MasrafDurumu {
    Bekliyor,
    Onaylandi,
    Reddedildi,
    Iptal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MasrafGorunum {
    #[serde(flatten)]
    pub kayit: MasrafKaydi,
    pub durum: MasrafDurumu,
    pub karar_veren: Option<String>,
}

// ---------------------------------------------------------------- girdi yapıları

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct SayimGirdi {
    pub tarih: String,
    pub cekmece: String,
    pub ikinci_kisi: Option<String>,
    pub adetler: BTreeMap<i64, u32>,
    pub not: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct GunSonuGirdi {
    pub tarih: String,
    pub cekmece: String,
    pub nakit_adetler: BTreeMap<i64, u32>,
    pub noktalar: Vec<NoktaGunSonu>,
    pub nebim_toplam_kurus: i64,
    pub ciro: BTreeMap<String, i64>,
    pub doviz: Vec<DovizSatiri>,
    pub devreden_kurus: i64,
    pub kontrol: Vec<bool>,
    pub not: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct MasrafGirdi {
    pub tarih: String,
    pub cekmece: String,
    pub tur: String,
    pub tutar_kurus: i64,
    pub aciklama: String,
    pub foto: Option<String>,
}

fn tutar_denetle(ad: &str, t: i64) -> Result<(), String> {
    if !(0..=TUTAR_EN_COK).contains(&t) {
        return Err(format!("{ad}: tutar 0 ile 10 milyar TL arasında olmalı"));
    }
    Ok(())
}

// ---------------------------------------------------------------- karşılaştırma (saf işlev)

/// Gün sonu karşılaştırması (KS7). `acilis_kurus`: o günün açılış sayımı; `masraf_kurus`: çekmeceden çıkan
/// (reddedilmemiş, iptal edilmemiş) masraflar.
pub fn karsilastir(
    ayar: &KasaAyar,
    girdi: &GunSonuGirdi,
    nakit_sayilan: i64,
    acilis_kurus: i64,
    masraf_kurus: i64,
) -> Karsilastirma {
    let cir = |k: &str| girdi.ciro.get(k).copied().unwrap_or(0);
    let doviz_tl: i64 = girdi.doviz.iter().map(DovizSatiri::tl_kurus).sum();
    let ciro_toplam: i64 = girdi.ciro.values().sum::<i64>() + doviz_tl;
    let z: i64 = girdi.noktalar.iter().map(|n| n.z_toplam_kurus).sum();
    let mut satirlar = vec![
        FarkSatiri {
            ad: "Nakit: sayılan ↔ beklenen (açılış + nakit ciro − masraf)".into(),
            beklenen_kurus: acilis_kurus + cir("nakit") - masraf_kurus,
            bulunan_kurus: nakit_sayilan,
            fark_kurus: 0,
        },
        FarkSatiri {
            ad: "Z raporu toplamı ↔ ciro toplamı".into(),
            beklenen_kurus: z,
            bulunan_kurus: ciro_toplam,
            fark_kurus: 0,
        },
    ];
    // POS cihazları ödeme türüne göre toplanır (Ziraat ve İyzico ayrı satır).
    let mut pos_tur: BTreeMap<String, (String, i64)> = BTreeMap::new();
    if let Some(c) = ayar.cekmece(&girdi.cekmece) {
        for n in &c.noktalar {
            let ng = girdi.noktalar.iter().find(|x| x.nokta == n.id);
            for p in &n.poslar {
                let t = ng.and_then(|x| x.pos.get(&p.id)).copied().unwrap_or(0);
                let e = pos_tur
                    .entry(p.odeme_turu.clone())
                    .or_insert_with(|| (p.ad.clone(), 0));
                e.1 += t;
            }
        }
    }
    for (tur, (ad, toplam)) in &pos_tur {
        let tur_ad = ODEME_TURLERI
            .iter()
            .find(|(k, _)| k == tur)
            .map(|(_, a)| *a)
            .unwrap_or(tur);
        satirlar.push(FarkSatiri {
            ad: format!("{ad} gün sonu ↔ {tur_ad} cirosu"),
            beklenen_kurus: *toplam,
            bulunan_kurus: cir(tur),
            fark_kurus: 0,
        });
    }
    satirlar.push(FarkSatiri {
        ad: "Satış sistemi (Nebim) toplamı ↔ ciro toplamı".into(),
        beklenen_kurus: girdi.nebim_toplam_kurus,
        bulunan_kurus: ciro_toplam,
        fark_kurus: 0,
    });
    for s in &mut satirlar {
        s.fark_kurus = s.bulunan_kurus - s.beklenen_kurus;
    }
    let dis = |s: &FarkSatiri| s.fark_kurus.abs() > ayar.fark_toleransi_kurus;
    Karsilastirma {
        fark_var: satirlar.iter().any(dis),
        kasa_ciro_uyusmuyor: satirlar.iter().take(2).any(dis),
        ciro_toplam_kurus: ciro_toplam,
        satirlar,
    }
}

// ---------------------------------------------------------------- defter

/// Gün sonu ve masraf defterleri. `kok`: `<veri kökü>/kasa`.
pub struct KasaDefteri {
    pub kok: PathBuf,
}

/// Bir gün ve çekmece için toplu görünüm (ekranın üç eylemi buradan seçilir).
#[derive(Debug, Clone, Serialize)]
pub struct GunDurumu {
    pub tarih: String,
    pub cekmece: String,
    pub acilis: Option<SayimKaydi>,
    pub sayimlar: Vec<SayimKaydi>,
    pub masraflar: Vec<MasrafGorunum>,
    pub gun_sonu: Option<GunSonuKaydi>,
    pub ikinci_onay: Option<String>,
    pub ertesi_gun_onay: Option<String>,
    pub gonderildi: bool,
    pub beklenen_nakit_kurus: i64,
    pub limit_asildi: bool,
    /// Sıradaki iş: "acilis", "gun_ici", "onay", "gonder", "tamam".
    pub asama: String,
}

fn ozellik_acik() -> Result<(), String> {
    if taa_ayar::ozellik_acik(OZELLIK_GUNSONU, true) {
        Ok(())
    } else {
        Err(taa_ayar::kapali_mesaji(OZELLIK_GUNSONU))
    }
}

impl KasaDefteri {
    pub fn yeni(kok: impl Into<PathBuf>) -> Self {
        KasaDefteri { kok: kok.into() }
    }
    pub fn gun_yolu(&self) -> PathBuf {
        self.kok.join("gunsonu.jsonl")
    }
    pub fn masraf_yolu(&self) -> PathBuf {
        self.kok.join("masraf.jsonl")
    }

    fn hazirla(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.kok).map_err(|e| format!("kasa klasörü açılamadı: {e}"))
    }

    pub fn gun_kayitlari(&self) -> Result<Vec<GunKaydi>, String> {
        oku(&self.gun_yolu())
    }

    pub fn masraf_kayitlari(&self) -> Result<Vec<MasrafKayit>, String> {
        oku(&self.masraf_yolu())
    }

    fn gun_ekle(&self, k: &GunKaydi) -> Result<(), String> {
        self.hazirla()?;
        let s = serde_json::to_string(k).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.gun_yolu(), &s).map_err(|e| e.to_string())
    }

    fn masraf_ekle_satir(&self, k: &MasrafKayit) -> Result<(), String> {
        self.hazirla()?;
        let s = serde_json::to_string(k).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.masraf_yolu(), &s).map_err(|e| e.to_string())
    }

    fn iptaller(k: &[GunKaydi]) -> Vec<String> {
        k.iter()
            .filter_map(|x| match x {
                GunKaydi::Iptal { id, .. } => Some(id.clone()),
                _ => None,
            })
            .collect()
    }

    /// Geçerli (iptal edilmemiş) sayımlar.
    pub fn sayimlar(&self) -> Result<Vec<SayimKaydi>, String> {
        let k = self.gun_kayitlari()?;
        let ip = Self::iptaller(&k);
        Ok(k.into_iter()
            .filter_map(|x| match x {
                GunKaydi::Sayim(s) if !ip.contains(&s.id) => Some(s),
                _ => None,
            })
            .collect())
    }

    /// Geçerli (iptal edilmemiş) gün sonları.
    pub fn gun_sonlari(&self) -> Result<Vec<GunSonuKaydi>, String> {
        let k = self.gun_kayitlari()?;
        let ip = Self::iptaller(&k);
        Ok(k.into_iter()
            .filter_map(|x| match x {
                GunKaydi::GunSonu(s) if !ip.contains(&s.id) => Some(s),
                _ => None,
            })
            .collect())
    }

    /// Masraflar ve durumları.
    pub fn masraflar(&self) -> Result<Vec<MasrafGorunum>, String> {
        let k = self.masraf_kayitlari()?;
        let mut v: Vec<MasrafGorunum> = Vec::new();
        for x in &k {
            match x {
                MasrafKayit::Masraf(m) => v.push(MasrafGorunum {
                    kayit: m.clone(),
                    durum: MasrafDurumu::Bekliyor,
                    karar_veren: None,
                }),
                MasrafKayit::Karar { id, kisi, onay, .. } => {
                    if let Some(g) = v.iter_mut().find(|g| &g.kayit.id == id) {
                        if g.durum == MasrafDurumu::Bekliyor {
                            g.durum = if *onay {
                                MasrafDurumu::Onaylandi
                            } else {
                                MasrafDurumu::Reddedildi
                            };
                            g.karar_veren = Some(kisi.clone());
                        }
                    }
                }
                MasrafKayit::Iptal { id, .. } => {
                    if let Some(g) = v.iter_mut().find(|g| &g.kayit.id == id) {
                        g.durum = MasrafDurumu::Iptal;
                    }
                }
            }
        }
        Ok(v)
    }

    fn sira(&self, on_ek: &str) -> Result<usize, String> {
        let say = |id: &str| id.starts_with(on_ek);
        let g = self
            .gun_kayitlari()?
            .iter()
            .filter(|k| match k {
                GunKaydi::Sayim(s) => say(&s.id),
                GunKaydi::GunSonu(s) => say(&s.id),
                _ => false,
            })
            .count();
        let m = self
            .masraf_kayitlari()?
            .iter()
            .filter(|k| matches!(k, MasrafKayit::Masraf(m) if say(&m.id)))
            .count();
        Ok(g + m + 1)
    }

    fn ortak_denetim(
        ayar: &KasaAyar,
        tarih: &str,
        cekmece: &str,
        kisi: &Kisi,
    ) -> Result<(), String> {
        ozellik_acik()?;
        if !zaman::tarih_gecerli(tarih) {
            return Err("tarih YYYY-AA-GG olmalı".into());
        }
        if ayar.cekmece(cekmece).is_none() {
            return Err(format!("tanımsız çekmece: {cekmece}"));
        }
        if kisi.ad.trim().is_empty() {
            return Err("işlemi yapan kişi gerekli (giriş yapın)".into());
        }
        Ok(())
    }

    /// Çekmecedeki beklenen nakit: açılış + (o güne kadar sayım yoksa) − masraflar. Ara sayım ve devir için bilgi amaçlı.
    fn beklenen_nakit(&self, tarih: &str, cekmece: &str) -> Result<(Option<i64>, i64), String> {
        let acilis = self
            .sayimlar()?
            .into_iter()
            .rfind(|s| s.tur == SayimTuru::Acilis && s.tarih == tarih && s.cekmece == cekmece)
            .map(|s| s.sayilan_kurus);
        let masraf = self.masraf_toplami(tarih, cekmece)?;
        Ok((acilis, masraf))
    }

    /// Çekmeceden çıkan masraf toplamı (reddedilen ve iptal edilen hariç).
    pub fn masraf_toplami(&self, tarih: &str, cekmece: &str) -> Result<i64, String> {
        Ok(self
            .masraflar()?
            .iter()
            .filter(|m| m.kayit.tarih == tarih && m.kayit.cekmece == cekmece)
            .filter(|m| matches!(m.durum, MasrafDurumu::Bekliyor | MasrafDurumu::Onaylandi))
            .map(|m| m.kayit.tutar_kurus)
            .sum())
    }

    /// Açılış, ara sayım ya da devir sayımı (KS2a–KS2c).
    pub fn sayim_ekle(
        &self,
        ayar: &KasaAyar,
        tur: SayimTuru,
        g: &SayimGirdi,
        kisi: &Kisi,
    ) -> Result<SayimKaydi, String> {
        Self::ortak_denetim(ayar, &g.tarih, &g.cekmece, kisi)?;
        let ikinci = g
            .ikinci_kisi
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        match tur {
            SayimTuru::Ara | SayimTuru::Devir => {
                let Some(i) = ikinci else {
                    return Err(if tur == SayimTuru::Devir {
                        "devralan kişi gerekli".into()
                    } else {
                        "ara sayım iki kişiyle yapılır: ikinci kişi gerekli".into()
                    });
                };
                if ayni_kisi(i, &kisi.ad) {
                    return Err("ikinci kişi işlemi girenden farklı olmalı".into());
                }
            }
            SayimTuru::Acilis => {
                if self.sayimlar()?.iter().any(|s| {
                    s.tur == SayimTuru::Acilis && s.tarih == g.tarih && s.cekmece == g.cekmece
                }) {
                    return Err("bu gün ve çekmece için açılış zaten var; düzeltme yönetici girişiyle yapılır".into());
                }
            }
        }
        let sayilan = sayim_toplam(&g.adetler)?;
        let beklenen = match tur {
            SayimTuru::Acilis => {
                // Bir önceki kapanışta ertesi güne devreden nakit.
                self.gun_sonlari()?
                    .into_iter()
                    .filter(|s| s.cekmece == g.cekmece && s.tarih < g.tarih)
                    .max_by(|a, b| a.tarih.cmp(&b.tarih).then(a.zaman.cmp(&b.zaman)))
                    .map(|s| s.devreden_kurus)
            }
            _ => {
                let (ac, masraf) = self.beklenen_nakit(&g.tarih, &g.cekmece)?;
                ac.map(|a| a - masraf)
            }
        };
        let fark = beklenen.map(|b| sayilan - b);
        if fark.is_some_and(|f| f.abs() > ayar.fark_toleransi_kurus) && g.not.trim().is_empty() {
            return Err(format!(
                "{} fark var; açıklama notu yazın",
                crate::kurus_metni(fark.unwrap_or(0))
            ));
        }
        let on = match tur {
            SayimTuru::Acilis => "AC",
            SayimTuru::Ara => "AS",
            SayimTuru::Devir => "DV",
        };
        let on_ek = format!("{on}-{}-{}-", g.tarih.replace('-', ""), g.cekmece);
        let s = SayimKaydi {
            id: format!("{on_ek}{}", self.sira(&on_ek)?),
            tur,
            tarih: g.tarih.clone(),
            cekmece: g.cekmece.clone(),
            kisi: kisi.ad.trim().into(),
            ikinci_kisi: ikinci.map(String::from),
            adetler: g.adetler.clone(),
            sayilan_kurus: sayilan,
            beklenen_kurus: beklenen,
            fark_kurus: fark,
            limit_asildi: ayar.kasa_limiti_kurus.is_some_and(|l| sayilan > l),
            not: g.not.trim().into(),
            zaman: zaman::simdi_sn(),
            duzeltir: None,
        };
        self.gun_ekle(&GunKaydi::Sayim(s.clone()))?;
        Ok(s)
    }

    /// Masraf girişi (KS6). Eşik altı müdür, eşik ve üstü merkez onayı bekler.
    pub fn masraf_ekle(
        &self,
        ayar: &KasaAyar,
        g: &MasrafGirdi,
        kisi: &Kisi,
    ) -> Result<MasrafKaydi, String> {
        Self::ortak_denetim(ayar, &g.tarih, &g.cekmece, kisi)?;
        tutar_denetle("masraf", g.tutar_kurus)?;
        if g.tutar_kurus == 0 {
            return Err("masraf tutarı sıfır olamaz".into());
        }
        if g.tur.trim().is_empty() || !ayar.masraf_turleri.iter().any(|t| t == g.tur.trim()) {
            return Err("masraf türü listeden seçilmeli".into());
        }
        if g.aciklama.trim().is_empty() {
            return Err("masraf açıklaması gerekli".into());
        }
        if let Some(f) = &g.foto {
            if f.contains("..") || f.starts_with('/') || f.contains('\\') || f.contains(':') {
                return Err("fiş fotoğrafı yolu geçersiz".into());
            }
        }
        let on_ek = format!("MS-{}-{}-", g.tarih.replace('-', ""), g.cekmece);
        let m = MasrafKaydi {
            id: format!("{on_ek}{}", self.sira(&on_ek)?),
            tarih: g.tarih.clone(),
            cekmece: g.cekmece.clone(),
            kisi: kisi.ad.trim().into(),
            tur: g.tur.trim().into(),
            tutar_kurus: g.tutar_kurus,
            aciklama: g.aciklama.trim().into(),
            foto: g.foto.clone(),
            onay_duzeyi: if g.tutar_kurus >= ayar.masraf_merkez_esigi_kurus {
                OnayDuzeyi::Merkez
            } else {
                OnayDuzeyi::Mudur
            },
            zaman: zaman::simdi_sn(),
            duzeltir: None,
        };
        self.masraf_ekle_satir(&MasrafKayit::Masraf(m.clone()))?;
        Ok(m)
    }

    /// Masraf onayı ya da reddi. Giren onaylayamaz; müdür düzeyi için müdür ya da yönetici, merkez düzeyi için
    /// yalnız yönetici (merkez).
    pub fn masraf_karar(&self, id: &str, onay: bool, not: &str, kisi: &Kisi) -> Result<(), String> {
        ozellik_acik()?;
        let m = self
            .masraflar()?
            .into_iter()
            .find(|m| m.kayit.id == id)
            .ok_or_else(|| format!("masraf yok: {id}"))?;
        if m.durum != MasrafDurumu::Bekliyor {
            return Err("bu masraf için karar verilmiş ya da iptal edilmiş".into());
        }
        if ayni_kisi(&m.kayit.kisi, &kisi.ad) {
            return Err("giren onaylayamaz: kararı başka biri vermeli".into());
        }
        let yetkili = match m.kayit.onay_duzeyi {
            OnayDuzeyi::Mudur => matches!(kisi.rol, Rol::Mudur | Rol::Yonetici),
            OnayDuzeyi::Merkez => kisi.rol == Rol::Yonetici,
        };
        if !yetkili {
            return Err(match m.kayit.onay_duzeyi {
                OnayDuzeyi::Mudur => "bu masrafı mağaza müdürü onaylar".into(),
                OnayDuzeyi::Merkez => "eşik üstü masrafı merkez (yönetici girişi) onaylar".into(),
            });
        }
        if !onay && not.trim().is_empty() {
            return Err("ret nedeni yazın".into());
        }
        self.masraf_ekle_satir(&MasrafKayit::Karar {
            id: id.into(),
            kisi: kisi.ad.trim().into(),
            onay,
            not: not.trim().into(),
            zaman: zaman::simdi_sn(),
        })
    }

    /// Gün sonu (KS7, KS8, KS9). Fark tolerans dışındaysa not zorunlu.
    pub fn gun_sonu_ekle(
        &self,
        ayar: &KasaAyar,
        g: &GunSonuGirdi,
        kisi: &Kisi,
    ) -> Result<GunSonuKaydi, String> {
        self.gun_sonu_yaz(ayar, g, kisi, None)
    }

    fn gun_sonu_yaz(
        &self,
        ayar: &KasaAyar,
        g: &GunSonuGirdi,
        kisi: &Kisi,
        duzeltir: Option<&str>,
    ) -> Result<GunSonuKaydi, String> {
        let s = self.gun_sonu_hazirla(ayar, g, kisi, duzeltir)?;
        self.gun_ekle(&GunKaydi::GunSonu(s.clone()))?;
        Ok(s)
    }

    /// Gün sonu kaydını doğrular ve hazırlar; yazmaz.
    fn gun_sonu_hazirla(
        &self,
        ayar: &KasaAyar,
        g: &GunSonuGirdi,
        kisi: &Kisi,
        duzeltir: Option<&str>,
    ) -> Result<GunSonuKaydi, String> {
        Self::ortak_denetim(ayar, &g.tarih, &g.cekmece, kisi)?;
        if duzeltir.is_none()
            && self
                .gun_sonlari()?
                .iter()
                .any(|s| s.tarih == g.tarih && s.cekmece == g.cekmece)
        {
            return Err(
                "bu gün ve çekmece için gün sonu zaten var; düzeltme yönetici girişiyle yapılır"
                    .into(),
            );
        }
        let cek = ayar.cekmece(&g.cekmece).ok_or("tanımsız çekmece")?;
        for n in &g.noktalar {
            let tanim = cek
                .noktalar
                .iter()
                .find(|x| x.id == n.nokta)
                .ok_or_else(|| format!("bu çekmeceye bağlı olmayan kasa noktası: {}", n.nokta))?;
            tutar_denetle("Z raporu", n.z_toplam_kurus)?;
            for (p, t) in &n.pos {
                if !tanim.poslar.iter().any(|x| &x.id == p) {
                    return Err(format!("tanımsız POS: {p}"));
                }
                tutar_denetle("POS gün sonu", *t)?;
            }
        }
        for k in g.ciro.keys() {
            if !ODEME_TURLERI.iter().any(|(a, _)| a == k) {
                return Err(format!("bilinmeyen ödeme türü: {k}"));
            }
        }
        for (k, t) in &g.ciro {
            tutar_denetle(k, *t)?;
        }
        for d in &g.doviz {
            if !DOVIZLER.iter().any(|(k, _)| *k == d.kod) {
                return Err(format!("bilinmeyen döviz: {}", d.kod));
            }
            tutar_denetle("döviz", d.tutar)?;
            if d.tutar > 0 && !(1..=100_000_000).contains(&d.kur_onbinde) {
                return Err(format!("{} için günlük kur girin", d.kod));
            }
        }
        tutar_denetle("Nebim toplamı", g.nebim_toplam_kurus)?;
        tutar_denetle("devreden nakit", g.devreden_kurus)?;
        let nakit = sayim_toplam(&g.nakit_adetler)?;
        if g.devreden_kurus > nakit {
            return Err("ertesi güne devreden nakit sayılan nakitten büyük olamaz".into());
        }
        let mut kontrol = g.kontrol.clone();
        kontrol.resize(KAPANIS_LISTESI.len(), false);
        let (acilis, masraf) = self.beklenen_nakit(&g.tarih, &g.cekmece)?;
        let k = karsilastir(ayar, g, nakit, acilis.unwrap_or(0), masraf);
        if k.fark_var && g.not.trim().is_empty() {
            return Err("karşılaştırmada fark var; açıklama notu yazın".into());
        }
        let on_ek = format!("GS-{}-{}-", g.tarih.replace('-', ""), g.cekmece);
        let s = GunSonuKaydi {
            id: format!("{on_ek}{}", self.sira(&on_ek)?),
            tarih: g.tarih.clone(),
            sube: ayar.sube.clone(),
            cekmece: g.cekmece.clone(),
            kisi: kisi.ad.trim().into(),
            nakit_adetler: g.nakit_adetler.clone(),
            nakit_sayilan_kurus: nakit,
            noktalar: g.noktalar.clone(),
            nebim_toplam_kurus: g.nebim_toplam_kurus,
            ciro: g.ciro.clone(),
            doviz: g.doviz.iter().filter(|d| d.tutar > 0).cloned().collect(),
            devreden_kurus: g.devreden_kurus,
            kontrol,
            not: g.not.trim().into(),
            karsilastirma: k,
            zaman: zaman::simdi_sn(),
            duzeltir: duzeltir.map(String::from),
        };
        Ok(s)
    }

    /// Kaydetmeden karşılaştırma (ekrandaki canlı fark paneli için).
    pub fn gun_sonu_onizle(
        &self,
        ayar: &KasaAyar,
        g: &GunSonuGirdi,
    ) -> Result<Karsilastirma, String> {
        let nakit = sayim_toplam(&g.nakit_adetler)?;
        let (acilis, masraf) = self.beklenen_nakit(&g.tarih, &g.cekmece)?;
        Ok(karsilastir(ayar, g, nakit, acilis.unwrap_or(0), masraf))
    }

    /// Kayıt onayı. Giren onaylayamaz; ertesi gün gözden geçirmesini müdür ya da yönetici yapar.
    pub fn onayla(&self, id: &str, tur: OnayTuru, kisi: &Kisi) -> Result<(), String> {
        ozellik_acik()?;
        let k = self.gun_kayitlari()?;
        let gs = self
            .gun_sonlari()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| format!("geçerli gün sonu yok: {id}"))?;
        if ayni_kisi(&gs.kisi, &kisi.ad) {
            return Err("giren onaylayamaz: onayı ikinci bir kişi vermeli".into());
        }
        if tur == OnayTuru::ErtesiGun && !matches!(kisi.rol, Rol::Mudur | Rol::Yonetici) {
            return Err("ertesi gün gözden geçirmesini mağaza müdürü yapar".into());
        }
        if k.iter()
            .any(|x| matches!(x, GunKaydi::Onay { id: i, onay, .. } if i == id && *onay == tur))
        {
            return Err("bu onay zaten verilmiş".into());
        }
        self.gun_ekle(&GunKaydi::Onay {
            id: id.into(),
            kisi: kisi.ad.trim().into(),
            onay: tur,
            zaman: zaman::simdi_sn(),
        })
    }

    /// İptal (silmez). Yalnız yönetici girişiyle; neden zorunlu. Sayım, gün sonu ya da masraf olabilir.
    pub fn iptal(&self, id: &str, neden: &str, kisi: &Kisi) -> Result<(), String> {
        ozellik_acik()?;
        if kisi.rol != Rol::Yonetici {
            return Err("düzeltme ve iptal yalnız yönetici girişiyle yapılır".into());
        }
        if neden.trim().is_empty() {
            return Err("iptal nedeni yazın".into());
        }
        let z = zaman::simdi_sn();
        if id.starts_with("MS-") {
            let m = self.masraflar()?;
            if !m
                .iter()
                .any(|x| x.kayit.id == id && x.durum != MasrafDurumu::Iptal)
            {
                return Err(format!("geçerli masraf yok: {id}"));
            }
            return self.masraf_ekle_satir(&MasrafKayit::Iptal {
                id: id.into(),
                neden: neden.trim().into(),
                kisi: kisi.ad.trim().into(),
                zaman: z,
            });
        }
        let var = self.sayimlar()?.iter().any(|s| s.id == id)
            || self.gun_sonlari()?.iter().any(|s| s.id == id);
        if !var {
            return Err(format!("geçerli kayıt yok: {id}"));
        }
        self.gun_ekle(&GunKaydi::Iptal {
            id: id.into(),
            neden: neden.trim().into(),
            kisi: kisi.ad.trim().into(),
            zaman: z,
        })
    }

    /// Gün sonu düzeltmesi: eskisi iptal edilir, yerine `duzeltir` alanı dolu yeni kayıt yazılır (iz kalır).
    pub fn gun_sonu_duzelt(
        &self,
        ayar: &KasaAyar,
        eski_id: &str,
        g: &GunSonuGirdi,
        neden: &str,
        yonetici: &Kisi,
    ) -> Result<GunSonuKaydi, String> {
        let eski = self
            .gun_sonlari()?
            .into_iter()
            .find(|s| s.id == eski_id)
            .ok_or_else(|| format!("geçerli gün sonu yok: {eski_id}"))?;
        if eski.tarih != g.tarih || eski.cekmece != g.cekmece {
            return Err("düzeltme aynı gün ve çekmece için olmalı".into());
        }
        if yonetici.rol != Rol::Yonetici {
            return Err("düzeltme yalnız yönetici girişiyle yapılır".into());
        }
        if neden.trim().is_empty() {
            return Err("düzeltme nedeni yazın".into());
        }
        // Önce yeni kayıt doğrulanır (yazmadan); geçersizse eski kayıt iptal edilmez.
        let yeni = self.gun_sonu_hazirla(ayar, g, yonetici, Some(eski_id))?;
        self.iptal(eski_id, neden, yonetici)?;
        self.gun_ekle(&GunKaydi::GunSonu(yeni.clone()))?;
        Ok(yeni)
    }

    /// Gönderim kaydı (paket kuyruğa alındı).
    pub fn gonderim_kaydet(&self, id: &str, dosya: &str, imzali: bool) -> Result<(), String> {
        self.gun_ekle(&GunKaydi::Gonderim {
            id: id.into(),
            dosya: dosya.into(),
            imzali,
            zaman: zaman::simdi_sn(),
        })
    }

    fn onay_kisi(k: &[GunKaydi], id: &str, tur: OnayTuru) -> Option<String> {
        k.iter().find_map(|x| match x {
            GunKaydi::Onay {
                id: i, onay, kisi, ..
            } if i == id && *onay == tur => Some(kisi.clone()),
            _ => None,
        })
    }

    /// Gün ve çekmece durumu.
    pub fn gun_durumu(
        &self,
        ayar: &KasaAyar,
        tarih: &str,
        cekmece: &str,
    ) -> Result<GunDurumu, String> {
        let k = self.gun_kayitlari()?;
        let sayimlar: Vec<SayimKaydi> = self
            .sayimlar()?
            .into_iter()
            .filter(|s| s.tarih == tarih && s.cekmece == cekmece)
            .collect();
        let acilis = sayimlar
            .iter()
            .rfind(|s| s.tur == SayimTuru::Acilis)
            .cloned();
        let masraflar: Vec<MasrafGorunum> = self
            .masraflar()?
            .into_iter()
            .filter(|m| m.kayit.tarih == tarih && m.kayit.cekmece == cekmece)
            .collect();
        let gun_sonu = self
            .gun_sonlari()?
            .into_iter()
            .rfind(|s| s.tarih == tarih && s.cekmece == cekmece);
        let (ikinci, ertesi, gonderildi) = match &gun_sonu {
            Some(g) => (
                Self::onay_kisi(&k, &g.id, OnayTuru::IkinciKisi),
                Self::onay_kisi(&k, &g.id, OnayTuru::ErtesiGun),
                k.iter()
                    .any(|x| matches!(x, GunKaydi::Gonderim { id, .. } if id == &g.id)),
            ),
            None => (None, None, false),
        };
        let masraf = self.masraf_toplami(tarih, cekmece)?;
        let beklenen = acilis.as_ref().map(|a| a.sayilan_kurus).unwrap_or(0) - masraf;
        let son_sayim = sayimlar.last().map(|s| s.sayilan_kurus).unwrap_or(0);
        let asama = if acilis.is_none() && gun_sonu.is_none() {
            "acilis"
        } else if gun_sonu.is_none() {
            "gun_ici"
        } else if ikinci.is_none() {
            "onay"
        } else if !gonderildi {
            "gonder"
        } else {
            "tamam"
        };
        Ok(GunDurumu {
            tarih: tarih.into(),
            cekmece: cekmece.into(),
            limit_asildi: ayar.kasa_limiti_kurus.is_some_and(|l| son_sayim > l),
            acilis,
            sayimlar,
            masraflar,
            gun_sonu,
            ikinci_onay: ikinci,
            ertesi_gun_onay: ertesi,
            gonderildi,
            beklenen_nakit_kurus: beklenen,
            asama: asama.into(),
        })
    }

    /// Gönderime hazır mı: ikinci kişi onayı ve kapanış listesinin tamamı.
    pub fn gonderime_hazir(&self, ayar: &KasaAyar, id: &str) -> Result<GunSonuKaydi, String> {
        let gs = self
            .gun_sonlari()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or_else(|| format!("geçerli gün sonu yok: {id}"))?;
        let d = self.gun_durumu(ayar, &gs.tarih, &gs.cekmece)?;
        if d.ikinci_onay.is_none() {
            return Err("önce ikinci kişi onayı (çift doğrulama) gerekli".into());
        }
        if gs.kontrol.iter().filter(|x| **x).count() < KAPANIS_LISTESI.len() {
            return Err("kapanış kontrol listesinin 7 maddesi de işaretlenmeli".into());
        }
        Ok(gs)
    }

    /// `kasa.gunsonu@1` gönderim paketi (imzasız gövde; imzayı program ekler).
    pub fn gonderim_paketi(&self, ayar: &KasaAyar, id: &str) -> Result<Value, String> {
        let gs = self.gonderime_hazir(ayar, id)?;
        let d = self.gun_durumu(ayar, &gs.tarih, &gs.cekmece)?;
        Ok(json!({
            "tur": "kasa.gunsonu@1",
            "surum": 1,
            "sube": ayar.sube,
            "tarih": gs.tarih,
            "cekmece": gs.cekmece,
            "gun_sonu": gs,
            "sayimlar": d.sayimlar,
            "masraflar": d.masraflar,
            "onaylar": {"ikinci_kisi": d.ikinci_onay, "ertesi_gun": d.ertesi_gun_onay},
            "olusturma": zaman::simdi_sn(),
        }))
    }

    /// Günlük kontrol formu (KS10): yapılan işlemlerden kendiliğinden dolar.
    pub fn gunluk_form(d: &GunDurumu) -> Vec<(String, bool, String)> {
        let gs = d.gun_sonu.as_ref();
        let devir = d
            .sayimlar
            .iter()
            .filter(|s| s.tur == SayimTuru::Devir)
            .count();
        let ara = d
            .sayimlar
            .iter()
            .filter(|s| s.tur == SayimTuru::Ara)
            .count();
        let bekleyen = d
            .masraflar
            .iter()
            .filter(|m| m.durum == MasrafDurumu::Bekliyor)
            .count();
        vec![
            (
                "Açılış sayımı".into(),
                d.acilis.is_some(),
                d.acilis
                    .as_ref()
                    .map(|a| format!("{} · {}", a.kisi, crate::kurus_metni(a.sayilan_kurus)))
                    .unwrap_or_else(|| "yapılmadı".into()),
            ),
            (
                "Gün içi sayım".into(),
                ara > 0,
                format!("{ara} ara sayım, {devir} devir"),
            ),
            (
                "Masraflar karara bağlandı".into(),
                bekleyen == 0,
                format!("{} masraf, {bekleyen} onay bekliyor", d.masraflar.len()),
            ),
            (
                "Gün sonu girildi".into(),
                gs.is_some(),
                gs.map(|g| g.kisi.clone())
                    .unwrap_or_else(|| "girilmedi".into()),
            ),
            (
                "Karşılaştırma farksız".into(),
                gs.is_some_and(|g| !g.karsilastirma.fark_var),
                gs.map(|g| {
                    if g.karsilastirma.fark_var {
                        "fark var, not yazıldı".into()
                    } else {
                        "fark yok".to_string()
                    }
                })
                .unwrap_or_default(),
            ),
            (
                "Kapanış listesi tamam".into(),
                gs.is_some_and(|g| g.kontrol.iter().all(|x| *x)),
                gs.map(|g| format!("{}/7", g.kontrol.iter().filter(|x| **x).count()))
                    .unwrap_or_default(),
            ),
            (
                "Çift doğrulama".into(),
                d.ikinci_onay.is_some(),
                d.ikinci_onay.clone().unwrap_or_else(|| "bekliyor".into()),
            ),
            (
                "Merkeze gönderildi".into(),
                d.gonderildi,
                if d.gonderildi {
                    "kuyrukta ya da teslim"
                } else {
                    "gönderilmedi"
                }
                .into(),
            ),
        ]
    }
}

fn oku<T: for<'de> Deserialize<'de>>(yol: &Path) -> Result<Vec<T>, String> {
    taa_dosya::satirlari_oku(yol)
        .map_err(|e| e.to_string())?
        .iter()
        .map(|l| serde_json::from_str(l).map_err(|e| format!("defter satırı bozuk: {e}")))
        .collect()
}

// ---------------------------------------------------------------- mağaza toplamı

/// Mağaza toplamı: çekmecelerin gün sonları kendiliğinden toplanır.
pub fn magaza_toplami(gun_sonlari: &[GunSonuKaydi], tarih: &str) -> Value {
    let g: Vec<&GunSonuKaydi> = gun_sonlari.iter().filter(|s| s.tarih == tarih).collect();
    let mut ciro: BTreeMap<String, i64> = BTreeMap::new();
    for s in &g {
        for (k, v) in &s.ciro {
            *ciro.entry(k.clone()).or_default() += v;
        }
    }
    json!({
        "tarih": tarih,
        "cekmece_sayisi": g.len(),
        "nakit_sayilan_kurus": g.iter().map(|s| s.nakit_sayilan_kurus).sum::<i64>(),
        "z_toplam_kurus": g.iter().flat_map(|s| s.noktalar.iter()).map(|n| n.z_toplam_kurus).sum::<i64>(),
        "ciro_toplam_kurus": g.iter().map(|s| s.karsilastirma.ciro_toplam_kurus).sum::<i64>(),
        "ciro": ciro,
        "kasa_ciro_uyusmuyor": g.iter().any(|s| s.karsilastirma.kasa_ciro_uyusmuyor),
    })
}

#[cfg(test)]
#[path = "testler_gunsonu.rs"]
mod testler;
