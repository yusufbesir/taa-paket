//! Usta puanı (0–10) ve QR okutmayla iş yükü ölçümü. Plan: arastirma/USTA-PUAN-CETVELI-2026-11-06.md.
//! Usta işi bitirince QR okutur (`Olay`); süre tutulmaz. Kazanılan dakika = Σ norm süre (okutma anında kaydedilir).
//! Yeniden iş / müşteri şikâyetini TEZGÂHTAR işaretler (`Isaret`); yönetici açıklamalı tutanak yazar (`Tutanak`).
//! KURALLAR (Yusuf 2026-11-06): usta kendi puanını GÖRMEZ, veriyi yalnız YÖNETİCİ görür (çağıran program
//! `takip.puan@1` iznini yalnız yöneticiye verir); gölge mod: hesap yapılır, gösterilmez.
//! Ağırlıklar ve hedef sabit değildir: `PuanAyari` dosyadan okunur, `gozden_gecir` veriye göre sınırlı ve
//! kayıtlı biçimde ayarlar. Özellik anahtarları `qr-uretim` ve `usta-puan` (ikisi de varsayılan KAPALI).
//! Prim/ücret formülü DEĞİLDİR. Sahip: Yusuf.

use crate::sure::{gun_no, gun_tarih, SureTablosu};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::path::{Path, PathBuf};

pub const OZELLIK: &str = "usta-puan";
pub const OZELLIK_QR: &str = "qr-uretim";
const DEFTER_DOSYASI: &str = "usta-puan.jsonl";
const AYAR_DOSYASI: &str = "puan-ayar.json";

// ---------------------------------------------------------------- kayıtlar

/// Ustanın bir kalemi QR ile bitirdiği an. `norm_dk` okutma anındaki norm süredir (sonradan tablo değişse de eski kayıt değişmez).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Olay {
    pub usta: String,
    pub fis_no: String,
    pub kalem_no: usize,
    #[serde(default)]
    pub islem_kodu: Option<String>,
    /// Norm dakika; süresi tanımsız işte None (kazanılan dakikaya girmez).
    #[serde(default)]
    pub norm_dk: Option<u32>,
    /// YYYY-AA-GG
    pub tarih: String,
    /// SS:DD
    pub saat: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IsaretTuru {
    YenidenIs,
    Sikayet,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Isaret {
    #[serde(default)]
    pub id: u64,
    pub fis_no: String,
    pub kalem_no: usize,
    pub usta: String,
    pub tur: IsaretTuru,
    pub neden: String,
    /// İşareti atan tezgâhtar.
    pub tezgah: String,
    pub tarih: String,
    /// Yönetici tutanağı (son yazılan geçerli).
    #[serde(default)]
    pub tutanak: Option<Tutanak>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sonuc {
    UstaKaynakli,
    UstaKaynakliDegil,
    /// Ustanın payı yüzde olarak (1–99).
    Kismen {
        yuzde: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tutanak {
    pub sonuc: Sonuc,
    /// Ağır hata (kumaş hasarı, yanlış ölçü): o dönem kalite bileşenini sınırlar.
    #[serde(default)]
    pub agir: bool,
    pub aciklama: String,
    pub yonetici: String,
    pub tarih: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kayit", rename_all = "snake_case")]
pub enum Kayit {
    Olay(Olay),
    Isaret(Isaret),
    Tutanak {
        isaret_id: u64,
        tutanak: Tutanak,
    },
    /// İzin/rapor günü: mevcut dakikadan düşer.
    Izin {
        usta: String,
        tarih: String,
        neden: String,
        yonetici: String,
    },
    AyarDegisti {
        tarih: String,
        degisiklikler: Vec<String>,
    },
}

/// İki YYYY-AA-GG tarihi arasındaki gün farkı (b − a); geçersizse 0.
pub fn gun_farki(a: &str, b: &str) -> i64 {
    match (gun_no(a), gun_no(b)) {
        (Some(x), Some(y)) => y - x,
        _ => 0,
    }
}

fn tarih_gecerli(t: &str) -> bool {
    gun_no(t).is_some()
}

fn saat_dk(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    if h.len() != 2 || m.len() != 2 {
        return None;
    }
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

/// Salt eklemeli (append-only) JSONL defteri. Hiçbir kayıt silinmez ya da değiştirilmez; düzeltme yeni kayıttır.
pub struct PuanDefteri {
    yol: PathBuf,
}

impl PuanDefteri {
    pub fn dizinden(dizin: &Path) -> PuanDefteri {
        PuanDefteri {
            yol: dizin.join(DEFTER_DOSYASI),
        }
    }

    pub fn oku(&self) -> Result<Vec<Kayit>, String> {
        let metin = match std::fs::read_to_string(&self.yol) {
            Ok(m) => m,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
            Err(e) => return Err(format!("usta puan defteri okunamadı: {e}")),
        };
        let mut v = Vec::new();
        for (i, satir) in metin.lines().enumerate() {
            if satir.trim().is_empty() {
                continue;
            }
            v.push(
                serde_json::from_str(satir)
                    .map_err(|e| format!("usta puan defteri {}. satır bozuk: {e}", i + 1))?,
            );
        }
        Ok(v)
    }

    fn ekle(&self, k: &Kayit) -> Result<(), String> {
        let satir = serde_json::to_string(k).map_err(|e| e.to_string())?;
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.yol)
            .map_err(|e| format!("usta puan defteri açılamadı: {e}"))?;
        writeln!(f, "{satir}").map_err(|e| format!("usta puan defterine yazılamadı: {e}"))
    }

    /// QR okutma kaydı. Aynı fiş + kalem + işlem iki kez okutulamaz.
    pub fn olay_ekle(&self, o: Olay) -> Result<(), String> {
        if !taa_ayar::ozellik_acik(OZELLIK_QR, false) {
            return Err(taa_ayar::kapali_mesaji(OZELLIK_QR));
        }
        if o.usta.trim().is_empty() || o.fis_no.trim().is_empty() {
            return Err("usta ve fiş no boş olamaz".into());
        }
        if !tarih_gecerli(&o.tarih) || saat_dk(&o.saat).is_none() {
            return Err("tarih YYYY-AA-GG, saat SS:DD olmalı".into());
        }
        let mevcut = self.oku()?;
        let tekrar = mevcut.iter().any(|k| {
            matches!(k, Kayit::Olay(e) if e.fis_no == o.fis_no && e.kalem_no == o.kalem_no && e.islem_kodu == o.islem_kodu)
        });
        if tekrar {
            return Err("bu kalem zaten okutulmuş".into());
        }
        self.ekle(&Kayit::Olay(o))
    }

    /// Tezgâhtar işareti. Dönen değer işaret numarası (tutanak buna yazılır).
    pub fn isaretle(&self, mut i: Isaret) -> Result<u64, String> {
        if !taa_ayar::ozellik_acik(OZELLIK_QR, false) {
            return Err(taa_ayar::kapali_mesaji(OZELLIK_QR));
        }
        if i.tezgah.trim().is_empty() || i.neden.trim().is_empty() || i.usta.trim().is_empty() {
            return Err("tezgâhtar, usta ve neden boş olamaz".into());
        }
        if !tarih_gecerli(&i.tarih) {
            return Err("tarih YYYY-AA-GG olmalı".into());
        }
        let mevcut = self.oku()?;
        let id = mevcut
            .iter()
            .filter_map(|k| match k {
                Kayit::Isaret(x) => Some(x.id),
                _ => None,
            })
            .max()
            .unwrap_or(0)
            + 1;
        i.id = id;
        i.tutanak = None;
        self.ekle(&Kayit::Isaret(i))?;
        Ok(id)
    }

    /// Yönetici tutanağı. Açıklama zorunlu; sonradan yazılan tutanak öncekinin yerine geçer (eskisi defterde kalır).
    pub fn tutanak_yaz(&self, isaret_id: u64, t: Tutanak) -> Result<(), String> {
        if t.aciklama.trim().is_empty() || t.yonetici.trim().is_empty() {
            return Err("tutanakta açıklama ve yönetici adı zorunlu".into());
        }
        if let Sonuc::Kismen { yuzde } = t.sonuc {
            if !(1..=99).contains(&yuzde) {
                return Err("kısmen sonucunda yüzde 1 ile 99 arasında olmalı".into());
            }
        }
        if !tarih_gecerli(&t.tarih) {
            return Err("tarih YYYY-AA-GG olmalı".into());
        }
        let var = self
            .oku()?
            .iter()
            .any(|k| matches!(k, Kayit::Isaret(x) if x.id == isaret_id));
        if !var {
            return Err(format!("{isaret_id} numaralı işaret yok"));
        }
        self.ekle(&Kayit::Tutanak {
            isaret_id,
            tutanak: t,
        })
    }

    pub fn izin_ekle(
        &self,
        usta: &str,
        tarih: &str,
        neden: &str,
        yonetici: &str,
    ) -> Result<(), String> {
        if usta.trim().is_empty() || yonetici.trim().is_empty() || !tarih_gecerli(tarih) {
            return Err("usta, yönetici ve geçerli tarih gerekli".into());
        }
        self.ekle(&Kayit::Izin {
            usta: usta.into(),
            tarih: tarih.into(),
            neden: neden.into(),
            yonetici: yonetici.into(),
        })
    }

    pub fn ayar_degisti(&self, tarih: &str, degisiklikler: Vec<String>) -> Result<(), String> {
        self.ekle(&Kayit::AyarDegisti {
            tarih: tarih.into(),
            degisiklikler,
        })
    }
}

// ---------------------------------------------------------------- ayar

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Agirliklar {
    pub verimlilik: f64,
    pub kalite: f64,
    pub musteri: f64,
    pub zamaninda: f64,
    pub duzen: f64,
}

impl Agirliklar {
    fn dizi(&self) -> [f64; 5] {
        [
            self.verimlilik,
            self.kalite,
            self.musteri,
            self.zamaninda,
            self.duzen,
        ]
    }
    fn diziden(a: [f64; 5]) -> Agirliklar {
        Agirliklar {
            verimlilik: a[0],
            kalite: a[1],
            musteri: a[2],
            zamaninda: a[3],
            duzen: a[4],
        }
    }
}

/// Alt/üst sınırlar: verimlilik ve kalite 0,20–0,40; diğerleri 0,05–0,30.
const SINIR: [(f64, f64); 5] = [
    (0.20, 0.40),
    (0.20, 0.40),
    (0.05, 0.30),
    (0.05, 0.30),
    (0.05, 0.30),
];
const AD: [&str; 5] = ["verimlilik", "kalite", "müşteri", "zamanında", "düzen"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PuanAyari {
    pub agirlik: Agirliklar,
    /// Verimlilik hedefi (kazanılan/mevcut dakika). İşlem bazlı hedef yoksa bu kullanılır.
    pub hedef_oran: f64,
    #[serde(default)]
    pub hedef_islem: BTreeMap<String, f64>,
    /// Küçük örnek düzeltmesi: ekip ortalamasından "C" hayali iş.
    pub c: f64,
    /// Tek usta varsa kullanılan önsel bileşen değeri.
    pub onsel: f64,
    /// İlk işten bu kadar gün puanlanmaz (deneme dönemi).
    pub deneme_gun: i64,
    /// Beklemede (tutanaksız) işaretin ağırlığı.
    pub beklemede_carpan: f64,
    /// true: `gozden_gecir` önerileri uygulanabilir; false: ayarlar yalnız elle değişir.
    pub otomatik: bool,
    /// İki otomatik ayar arasındaki en az gün.
    pub ayar_araligi_gun: i64,
}

impl Default for PuanAyari {
    fn default() -> Self {
        PuanAyari {
            agirlik: Agirliklar {
                verimlilik: 0.35,
                kalite: 0.35,
                musteri: 0.15,
                zamaninda: 0.10,
                duzen: 0.05,
            },
            hedef_oran: 0.85,
            hedef_islem: BTreeMap::new(),
            c: 30.0,
            onsel: 0.85,
            deneme_gun: 28,
            beklemede_carpan: 0.5,
            otomatik: true,
            ayar_araligi_gun: 28,
        }
    }
}

impl PuanAyari {
    pub fn dogrula(&self) -> Result<(), String> {
        let a = self.agirlik.dizi();
        if (a.iter().sum::<f64>() - 1.0).abs() > 1e-6 {
            return Err("ağırlıkların toplamı 1 olmalı".into());
        }
        for (i, w) in a.iter().enumerate() {
            if *w < SINIR[i].0 - 1e-9 || *w > SINIR[i].1 + 1e-9 {
                return Err(format!(
                    "{} ağırlığı {}–{} arasında olmalı",
                    AD[i], SINIR[i].0, SINIR[i].1
                ));
            }
        }
        let hedef_ok = |h: f64| (0.3..=1.0).contains(&h);
        if !hedef_ok(self.hedef_oran) || self.hedef_islem.values().any(|h| !hedef_ok(*h)) {
            return Err("hedef oran 0,30 ile 1,00 arasında olmalı".into());
        }
        if self.c < 0.0 || !(0.0..=1.0).contains(&self.onsel) || self.deneme_gun < 0 {
            return Err("c, önsel ya da deneme günü geçersiz".into());
        }
        if !(0.0..=1.0).contains(&self.beklemede_carpan) || self.ayar_araligi_gun < 1 {
            return Err("beklemede çarpanı 0–1, ayar aralığı en az 1 gün olmalı".into());
        }
        Ok(())
    }

    pub fn json_den(s: &str) -> Result<PuanAyari, String> {
        let a: PuanAyari =
            serde_json::from_str(s).map_err(|e| format!("puan ayarı geçersiz: {e}"))?;
        a.dogrula()?;
        Ok(a)
    }

    /// `dizin/puan-ayar.json` varsa onu, yoksa varsayılanı yükler. Bozuk dosya hata verir.
    pub fn dizinden(dizin: &Path) -> Result<PuanAyari, String> {
        match std::fs::read_to_string(dizin.join(AYAR_DOSYASI)) {
            Ok(s) => Self::json_den(&s),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("puan ayarı okunamadı: {e}")),
        }
    }

    /// Önce geçici dosyaya yazıp yeniden adlandırır (yarım dosya kalmaz).
    pub fn kaydet(&self, dizin: &Path) -> Result<(), String> {
        self.dogrula()?;
        let metin = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        let gecici = dizin.join(format!("{AYAR_DOSYASI}.tmp"));
        std::fs::write(&gecici, metin).map_err(|e| format!("puan ayarı yazılamadı: {e}"))?;
        std::fs::rename(&gecici, dizin.join(AYAR_DOSYASI))
            .map_err(|e| format!("puan ayarı yerine konamadı: {e}"))
    }

    fn hedef(&self, kod: Option<&str>) -> f64 {
        kod.and_then(|k| self.hedef_islem.get(k).copied())
            .unwrap_or(self.hedef_oran)
    }
}

// ---------------------------------------------------------------- hesap

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Bilesenler {
    pub verimlilik: f64,
    pub kalite: f64,
    pub musteri: f64,
    pub zamaninda: f64,
    pub duzen: f64,
}

impl Bilesenler {
    fn dizi(&self) -> [f64; 5] {
        [
            self.verimlilik,
            self.kalite,
            self.musteri,
            self.zamaninda,
            self.duzen,
        ]
    }
    fn diziden(a: [f64; 5]) -> Bilesenler {
        Bilesenler {
            verimlilik: a[0],
            kalite: a[1],
            musteri: a[2],
            zamaninda: a[3],
            duzen: a[4],
        }
    }
}

/// YALNIZ yönetici görür. Usta ekranına bu yapı ASLA verilmez.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct UstaPuan {
    pub usta: String,
    /// None: deneme dönemi (puan gösterilmez/hesaplanmaz).
    pub puan: Option<f64>,
    pub deneme: bool,
    pub is_adedi: usize,
    pub kazanilan_dk: f64,
    pub mevcut_dk: u32,
    /// Ceza sonrası kazanılan / mevcut.
    pub verimlilik_orani: f64,
    pub hedef: f64,
    /// Küçük örnek düzeltmesinden SONRAKİ bileşenler (puana giren).
    pub bilesenler: Bilesenler,
    /// Düzeltme öncesi bileşenler (ayar öğrenmesi bunlara bakar).
    pub ham: Bilesenler,
    /// Tutanaksız (beklemede) işaret sayısı.
    pub beklemede: usize,
    pub suresiz_is: usize,
    pub yorumlar: Vec<String>,
}

struct Gorunum {
    olaylar: Vec<Olay>,
    isaretler: Vec<Isaret>,
    izinler: BTreeSet<(String, i64)>,
    son_ayar: Option<String>,
}

fn birlestir(kayitlar: &[Kayit]) -> Gorunum {
    let mut g = Gorunum {
        olaylar: vec![],
        isaretler: vec![],
        izinler: BTreeSet::new(),
        son_ayar: None,
    };
    for k in kayitlar {
        match k {
            Kayit::Olay(o) => g.olaylar.push(o.clone()),
            Kayit::Isaret(i) => g.isaretler.push(i.clone()),
            Kayit::Tutanak { isaret_id, tutanak } => {
                if let Some(i) = g.isaretler.iter_mut().find(|i| i.id == *isaret_id) {
                    i.tutanak = Some(tutanak.clone());
                }
            }
            Kayit::Izin { usta, tarih, .. } => {
                if let Some(n) = gun_no(tarih) {
                    g.izinler.insert((usta.clone(), n));
                }
            }
            Kayit::AyarDegisti { tarih, .. } => {
                if g.son_ayar.as_deref().is_none_or(|s| s < tarih.as_str()) {
                    g.son_ayar = Some(tarih.clone());
                }
            }
        }
    }
    g
}

/// Tüm işaretler, tutanaklarıyla birleşik (yönetici ekranı için). Özellik denetimi çağıranda.
pub fn isaretler(kayitlar: &[Kayit]) -> Vec<Isaret> {
    birlestir(kayitlar).isaretler
}

fn aralik(bas: &str, bit: &str) -> Result<(i64, i64), String> {
    match (gun_no(bas), gun_no(bit)) {
        (Some(b), Some(e)) if b <= e => Ok((b, e)),
        _ => Err("başlangıç ve bitiş YYYY-AA-GG olmalı, başlangıç bitişten sonra olamaz".into()),
    }
}

/// İşaretin puana giren ağırlığı: tutanaksız yarım, usta kaynaklı tam, kısmen yüzde, kaynaklı değil sıfır.
fn isaret_agirligi(i: &Isaret, ayar: &PuanAyari) -> f64 {
    match &i.tutanak {
        None => ayar.beklemede_carpan,
        Some(t) => match t.sonuc {
            Sonuc::UstaKaynakli => 1.0,
            Sonuc::UstaKaynakliDegil => 0.0,
            Sonuc::Kismen { yuzde } => f64::from(yuzde) / 100.0,
        },
    }
}

/// Aynı gün içinde peş peşe (aralık ≤ 2 dk) en az 6 okutma = toplu okutma. Dönen: toplu sayılan olay sayısı.
fn toplu_say(saatler: &mut [u32]) -> usize {
    saatler.sort_unstable();
    let mut toplam = 0;
    let mut kume = 1usize;
    for i in 1..=saatler.len() {
        if i < saatler.len() && saatler[i] - saatler[i - 1] <= 2 {
            kume += 1;
        } else {
            if kume >= 6 {
                toplam += kume;
            }
            kume = 1;
        }
    }
    toplam
}

fn calisma_gunu(sure: &SureTablosu, g: &Gorunum, usta: &str, gun: i64) -> bool {
    !sure.kapali_mi(gun) && !g.izinler.contains(&(usta.to_string(), gun))
}

/// Mevcut dakika. Aralığın SON günü, ustanın o gün hiç okutması yoksa sayılmaz (gün henüz bitmemiş olabilir;
/// bugünü kapsayan raporda yarım gün verimliliği yapay düşürmesin). `son_gun_olay`: o gün okutma var mı.
fn mevcut_dk(
    sure: &SureTablosu,
    g: &Gorunum,
    usta: &str,
    bas: i64,
    bit: i64,
    ilk: i64,
    son_gun_olay: bool,
) -> Result<u32, String> {
    let gunluk = sure
        .gunluk_dakika
        .ok_or("sure.json içinde gunluk_dakika tanımsız; puan hesaplanamaz")?;
    let n = (bas.max(ilk)..=bit)
        .filter(|d| (*d != bit || son_gun_olay) && calisma_gunu(sure, g, usta, *d))
        .count() as u32;
    Ok(n * gunluk)
}

struct Ham {
    usta: String,
    n: usize,
    kazanilan: f64,
    mevcut: u32,
    oran: f64,
    hedef: f64,
    b: [f64; 5],
    beklemede: usize,
    suresiz: usize,
    ilk: i64,
    toplu: usize,
    agir: bool,
}

/// Tüm ustaların puanını hesaplar. `teslimler`: fiş no -> söz verilen teslim tarihi (zamanında bileşeni için).
/// Özellik kapalıysa (`usta-puan`) hata verir.
pub fn hesapla(
    ayar: &PuanAyari,
    sure: &SureTablosu,
    kayitlar: &[Kayit],
    teslimler: &BTreeMap<String, String>,
    bas: &str,
    bit: &str,
) -> Result<Vec<UstaPuan>, String> {
    if !taa_ayar::ozellik_acik(OZELLIK, false) {
        return Err(taa_ayar::kapali_mesaji(OZELLIK));
    }
    hesapla_cekirdek(ayar, sure, kayitlar, teslimler, bas, bit)
}

fn hesapla_cekirdek(
    ayar: &PuanAyari,
    sure: &SureTablosu,
    kayitlar: &[Kayit],
    teslimler: &BTreeMap<String, String>,
    bas: &str,
    bit: &str,
) -> Result<Vec<UstaPuan>, String> {
    ayar.dogrula()?;
    let (b0, b1) = aralik(bas, bit)?;
    let g = birlestir(kayitlar);
    let ilk_gunler: BTreeMap<&str, i64> = g.olaylar.iter().fold(BTreeMap::new(), |mut m, o| {
        if let Some(d) = gun_no(&o.tarih) {
            let e = m.entry(o.usta.as_str()).or_insert(d);
            *e = (*e).min(d);
        }
        m
    });
    let norm_of: BTreeMap<(&str, usize), u32> = g
        .olaylar
        .iter()
        .filter_map(|o| o.norm_dk.map(|n| ((o.fis_no.as_str(), o.kalem_no), n)))
        .collect();

    let mut hamlar: Vec<Ham> = Vec::new();
    for (usta, ilk) in &ilk_gunler {
        let ol: Vec<&Olay> = g
            .olaylar
            .iter()
            .filter(|o| o.usta == *usta && gun_no(&o.tarih).is_some_and(|d| (b0..=b1).contains(&d)))
            .collect();
        if ol.is_empty() {
            continue;
        }
        let n = ol.len();
        let brut: f64 = ol.iter().filter_map(|o| o.norm_dk).map(f64::from).sum();
        let suresiz = ol.iter().filter(|o| o.norm_dk.is_none()).count();

        let isr: Vec<&Isaret> = g
            .isaretler
            .iter()
            .filter(|i| i.usta == *usta && gun_no(&i.tarih).is_some_and(|d| (b0..=b1).contains(&d)))
            .collect();
        let mut yeniden = 0.0;
        let mut sikayet = 0.0;
        let mut ceza = 0.0;
        let mut agir = false;
        let mut beklemede = 0;
        for i in &isr {
            let w = isaret_agirligi(i, ayar);
            if i.tutanak.is_none() {
                beklemede += 1;
            }
            let dk = norm_of
                .get(&(i.fis_no.as_str(), i.kalem_no))
                .copied()
                .unwrap_or(0) as f64;
            match i.tur {
                IsaretTuru::YenidenIs => {
                    yeniden += w;
                    ceza += w * dk * 2.0;
                }
                IsaretTuru::Sikayet => {
                    sikayet += w;
                    ceza += w * dk * 3.0;
                }
            }
            if i.tutanak
                .as_ref()
                .is_some_and(|t| t.agir && t.sonuc != Sonuc::UstaKaynakliDegil)
            {
                agir = true;
            }
        }
        let net = (brut - ceza).max(0.0);
        let son_gun_olay = ol.iter().any(|o| gun_no(&o.tarih) == Some(b1));
        let mevcut = mevcut_dk(sure, &g, usta, b0, b1, *ilk, son_gun_olay)?;
        let oran = if mevcut == 0 {
            0.0
        } else {
            net / f64::from(mevcut)
        };

        // Hedef: işlemlerin norm dakikasına göre ağırlıklı ortalama.
        let (ts, tw) = ol.iter().fold((0.0, 0.0), |(s, w), o| {
            let dk = f64::from(o.norm_dk.unwrap_or(0));
            (s + dk * ayar.hedef(o.islem_kodu.as_deref()), w + dk)
        });
        let hedef = if tw > 0.0 { ts / tw } else { ayar.hedef_oran };
        let v = (oran / hedef).clamp(0.0, 1.0);

        let k = (1.0 - yeniden / n as f64).clamp(0.0, 1.0);
        let k = if agir { k.min(0.6) } else { k };
        let m = (1.0 - sikayet / n as f64).clamp(0.0, 1.0);

        let zaman: Vec<bool> = ol
            .iter()
            .filter_map(|o| {
                teslimler
                    .get(&o.fis_no)
                    .map(|t| o.tarih.as_str() <= t.as_str())
            })
            .collect();
        let z = if zaman.is_empty() {
            ayar.onsel
        } else {
            zaman.iter().filter(|b| **b).count() as f64 / zaman.len() as f64
        };

        let mut gunler: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
        for o in &ol {
            if let Some(dk) = saat_dk(&o.saat) {
                gunler.entry(o.tarih.as_str()).or_default().push(dk);
            }
        }
        let toplu: usize = gunler.values_mut().map(|v| toplu_say(v)).sum();
        let d = (1.0 - toplu as f64 / n as f64).clamp(0.0, 1.0);

        hamlar.push(Ham {
            usta: (*usta).to_string(),
            n,
            kazanilan: net,
            mevcut,
            oran,
            hedef,
            b: [v, k, m, z, d],
            beklemede,
            suresiz,
            ilk: *ilk,
            toplu,
            agir,
        });
    }

    // Ekip ortalaması (iş sayısına göre ağırlıklı); tek usta varsa önsel değer.
    let toplam_n: f64 = hamlar.iter().map(|h| h.n as f64).sum();
    let ort: [f64; 5] = if hamlar.len() < 2 {
        [ayar.onsel; 5]
    } else {
        let mut o = [0.0; 5];
        for h in &hamlar {
            for (i, x) in o.iter_mut().enumerate() {
                *x += h.b[i] * h.n as f64 / toplam_n;
            }
        }
        o
    };
    let w = ayar.agirlik.dizi();
    let mut cikti = Vec::new();
    for h in hamlar {
        let n = h.n as f64;
        let mut duz = [0.0; 5];
        for (i, x) in duz.iter_mut().enumerate() {
            *x = (ayar.c * ort[i] + n * h.b[i]) / (ayar.c + n);
        }
        let deneme = b1 - h.ilk < ayar.deneme_gun;
        let puan = (!deneme).then(|| {
            let ham: f64 = (0..5).map(|i| w[i] * duz[i]).sum();
            (ham * 1000.0).round() / 100.0
        });
        let mut yorumlar = Vec::new();
        if deneme {
            yorumlar.push(format!(
                "Deneme dönemi: ilk okutmadan {} gün geçmeden puan verilmez.",
                ayar.deneme_gun
            ));
        }
        if h.b[0] < 0.8 {
            yorumlar.push(format!(
                "Verimlilik düşük: kazanılan {:.0} dk / mevcut {} dk (%{:.0}), hedef %{:.0}.",
                h.kazanilan,
                h.mevcut,
                h.oran * 100.0,
                h.hedef * 100.0
            ));
        }
        if h.b[1] < 0.9 {
            yorumlar.push(format!("Yeniden iş yüksek: kalite bileşeni {:.2}.", h.b[1]));
        }
        if h.agir {
            yorumlar.push("Ağır hata tutanağı var: kalite bileşeni en çok 0,60.".into());
        }
        if h.b[2] < 0.95 {
            yorumlar.push(format!("Müşteri şikâyeti etkisi: bileşen {:.2}.", h.b[2]));
        }
        if h.toplu > 0 {
            yorumlar.push(format!(
                "{} iş toplu okutulmuş (peş peşe 2 dk aralıkla 6+); gerçek bitiş zamanı belirsiz.",
                h.toplu
            ));
        }
        if h.beklemede > 0 {
            yorumlar.push(format!(
                "{} işaret tutanak bekliyor (yarım ağırlıkla sayıldı).",
                h.beklemede
            ));
        }
        if h.suresiz > 0 {
            yorumlar.push(format!(
                "{} işin norm süresi tanımsız, kazanılan dakikaya girmedi.",
                h.suresiz
            ));
        }
        cikti.push(UstaPuan {
            usta: h.usta,
            puan,
            deneme,
            is_adedi: h.n,
            kazanilan_dk: h.kazanilan,
            mevcut_dk: h.mevcut,
            verimlilik_orani: h.oran,
            hedef: h.hedef,
            bilesenler: Bilesenler::diziden(duz),
            ham: Bilesenler::diziden(h.b),
            beklemede: h.beklemede,
            suresiz_is: h.suresiz,
            yorumlar,
        });
    }
    Ok(cikti)
}

// ---------------------------------------------------------------- anomali

/// Ani değişim uyarısı. YALNIZ yönetici raporuna gider; ustaya söylenmez.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Anomali {
    pub usta: String,
    pub tur: String,
    pub mesaj: String,
}

fn ort_std(v: &[f64]) -> (f64, f64) {
    let n = v.len() as f64;
    let m = v.iter().sum::<f64>() / n;
    let var = v.iter().map(|x| (x - m).powi(2)).sum::<f64>() / n;
    (m, var.sqrt())
}

pub fn anomaliler(
    ayar: &PuanAyari,
    sure: &SureTablosu,
    kayitlar: &[Kayit],
    bugun: &str,
) -> Result<Vec<Anomali>, String> {
    if !taa_ayar::ozellik_acik(OZELLIK, false) {
        return Err(taa_ayar::kapali_mesaji(OZELLIK));
    }
    anomali_cekirdek(ayar, sure, kayitlar, bugun)
}

fn anomali_cekirdek(
    ayar: &PuanAyari,
    sure: &SureTablosu,
    kayitlar: &[Kayit],
    bugun: &str,
) -> Result<Vec<Anomali>, String> {
    let bugun_no = gun_no(bugun).ok_or("bugün YYYY-AA-GG olmalı")?;
    let gunluk = f64::from(
        sure.gunluk_dakika
            .ok_or("sure.json içinde gunluk_dakika tanımsız")?,
    );
    let g = birlestir(kayitlar);
    let ustalar: BTreeSet<&str> = g.olaylar.iter().map(|o| o.usta.as_str()).collect();
    let mut cikti = Vec::new();
    for usta in ustalar {
        let ilk = g
            .olaylar
            .iter()
            .filter(|o| o.usta == usta)
            .filter_map(|o| gun_no(&o.tarih))
            .min()
            .unwrap_or(bugun_no);
        let gunluk_kazanc = |d: i64| -> f64 {
            let t = gun_tarih(d);
            g.olaylar
                .iter()
                .filter(|o| o.usta == usta && o.tarih == t)
                .filter_map(|o| o.norm_dk)
                .map(f64::from)
                .sum::<f64>()
                / gunluk
        };
        let seri = |a: i64, b: i64| -> Vec<f64> {
            (a.max(ilk)..=b)
                .filter(|d| calisma_gunu(sure, &g, usta, *d))
                .map(gunluk_kazanc)
                .collect()
        };
        // Bugün henüz bitmemiş olabilir: seriler dünle biter.
        let son = seri(bugun_no - 7, bugun_no - 1);
        let onceki = seri(bugun_no - 97, bugun_no - 8);
        if son.len() >= 3 && onceki.len() >= 10 {
            let (m0, s0) = ort_std(&onceki);
            let (m1, _) = ort_std(&son);
            let s0 = s0.max(0.05);
            let z = (m1 - m0) / s0;
            if z <= -2.0 {
                cikti.push(Anomali {
                    usta: usta.into(),
                    tur: "verimlilik-dusus".into(),
                    mesaj: format!(
                        "{usta}: son 7 günde günlük kazanılan dakika oranı %{:.0} (önceki 90 gün ortalaması %{:.0}). Ani düşüş.",
                        m1 * 100.0,
                        m0 * 100.0
                    ),
                });
            }
        }
        // Yeniden iş oranı: son 14 gün, önceki 90 güne göre.
        let sayim = |a: i64, b: i64| -> (f64, f64) {
            let n = g
                .olaylar
                .iter()
                .filter(|o| {
                    o.usta == usta && gun_no(&o.tarih).is_some_and(|d| (a..=b).contains(&d))
                })
                .count() as f64;
            let y: f64 = g
                .isaretler
                .iter()
                .filter(|i| {
                    i.usta == usta
                        && i.tur == IsaretTuru::YenidenIs
                        && gun_no(&i.tarih).is_some_and(|d| (a..=b).contains(&d))
                })
                .map(|i| isaret_agirligi(i, ayar))
                .sum();
            (n, y)
        };
        let (n14, y14) = sayim(bugun_no - 13, bugun_no);
        let (n90, y90) = sayim(bugun_no - 103, bugun_no - 14);
        if n14 > 0.0 && n90 >= 30.0 && y14 >= 3.0 {
            let r14 = y14 / n14;
            let r90 = (y90 / n90).max(0.02);
            if r14 > 2.0 * r90 {
                cikti.push(Anomali {
                    usta: usta.into(),
                    tur: "yeniden-is-artis".into(),
                    mesaj: format!(
                        "{usta}: son 14 günde yeniden iş oranı %{:.1} (önceki 90 gün %{:.1}). Ani artış.",
                        r14 * 100.0,
                        r90 * 100.0
                    ),
                });
            }
        }
        // Toplu okutma (son 7 gün).
        let mut gunler: BTreeMap<&str, Vec<u32>> = BTreeMap::new();
        for o in g.olaylar.iter().filter(|o| {
            o.usta == usta
                && gun_no(&o.tarih).is_some_and(|d| (bugun_no - 6..=bugun_no).contains(&d))
        }) {
            if let Some(dk) = saat_dk(&o.saat) {
                gunler.entry(o.tarih.as_str()).or_default().push(dk);
            }
        }
        let toplu: usize = gunler.values_mut().map(|v| toplu_say(v)).sum();
        if toplu > 0 {
            cikti.push(Anomali {
                usta: usta.into(),
                tur: "toplu-okutma".into(),
                mesaj: format!("{usta}: son 7 günde {toplu} iş toplu okutulmuş (sahte/gecikmiş okutma şüphesi)."),
            });
        }
        // Norm gözden geçir: 30 gün ortalama %115 üstü.
        let s30 = seri(bugun_no - 30, bugun_no - 1);
        if s30.len() >= 10 && s30.iter().sum::<f64>() / s30.len() as f64 > 1.15 {
            cikti.push(Anomali {
                usta: usta.into(),
                tur: "norm-gozden-gecir".into(),
                mesaj: format!("{usta}: 30 gün ortalaması norm süresinin %115 üstünde; norm tablosu gözden geçirilmeli."),
            });
        }
    }
    Ok(cikti)
}

// ---------------------------------------------------------------- kendini ayarlama

#[derive(Debug, Clone, PartialEq)]
pub struct AyarOnerisi {
    pub yeni: PuanAyari,
    pub degisiklikler: Vec<String>,
}

fn medyan(v: &mut [f64]) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

/// Veriye göre ayar önerisi. Sınırlı ve kayıtlıdır; çağıran uygular (`otomatik` ise) ve `PuanDefteri::ayar_degisti` ile kaydeder.
/// - Hedef oran: deneme dışı ustaların verimlilik oranı medyanının 1,1 katına doğru %20 adım, adım en çok 0,05, sınır 0,60–0,95.
/// - Ağırlıklar (en az 3 usta): bileşenin ustalar arası ayrışma payına doğru %10 adım, adım en çok 0,03, sınırlar içinde.
/// - Veri azsa (< 200 okutma ya da < 28 gün) ya da son ayardan beri aralık dolmadıysa öneri yok.
pub fn gozden_gecir(
    ayar: &PuanAyari,
    puanlar: &[UstaPuan],
    toplam_olay: usize,
    veri_gun: i64,
    kayitlar: &[Kayit],
    bugun: &str,
) -> Option<AyarOnerisi> {
    if !ayar.otomatik || toplam_olay < 200 || veri_gun < 28 {
        return None;
    }
    let bugun_no = gun_no(bugun)?;
    if let Some(son) = birlestir(kayitlar).son_ayar.as_deref().and_then(gun_no) {
        if bugun_no - son < ayar.ayar_araligi_gun {
            return None;
        }
    }
    let aktif: Vec<&UstaPuan> = puanlar.iter().filter(|p| !p.deneme).collect();
    if aktif.len() < 2 {
        return None;
    }
    let mut yeni = ayar.clone();
    let mut degisiklikler = Vec::new();

    let mut oranlar: Vec<f64> = aktif.iter().map(|p| p.verimlilik_orani).collect();
    let hedef = (medyan(&mut oranlar) * 1.1).clamp(0.60, 0.95);
    let adim = (0.2 * (hedef - ayar.hedef_oran)).clamp(-0.05, 0.05);
    if adim.abs() >= 0.005 {
        yeni.hedef_oran = ((ayar.hedef_oran + adim) * 1000.0).round() / 1000.0;
        degisiklikler.push(format!(
            "hedef oran {:.3} -> {:.3} (ustaların medyan verimliliği %{:.0})",
            ayar.hedef_oran,
            yeni.hedef_oran,
            medyan(&mut oranlar) * 100.0
        ));
    }

    if aktif.len() >= 3 {
        let mut std = [0.0; 5];
        for (i, s) in std.iter_mut().enumerate() {
            let v: Vec<f64> = aktif.iter().map(|p| p.ham.dizi()[i]).collect();
            *s = ort_std(&v).1;
        }
        let toplam: f64 = std.iter().sum();
        if toplam > 1e-9 {
            let w = ayar.agirlik.dizi();
            let mut y = w;
            for i in 0..5 {
                let pay = std[i] / toplam;
                y[i] = w[i] + (0.1 * (pay - w[i])).clamp(-0.03, 0.03);
                y[i] = y[i].clamp(SINIR[i].0, SINIR[i].1);
            }
            // Toplamı 1'e getir: farkı sınırı aşmayan bileşenlere paylaştır.
            for _ in 0..8 {
                let fark = 1.0 - y.iter().sum::<f64>();
                if fark.abs() < 1e-9 {
                    break;
                }
                let serbest: Vec<usize> = (0..5)
                    .filter(|i| {
                        if fark > 0.0 {
                            y[*i] < SINIR[*i].1 - 1e-9
                        } else {
                            y[*i] > SINIR[*i].0 + 1e-9
                        }
                    })
                    .collect();
                if serbest.is_empty() {
                    break;
                }
                let pay = fark / serbest.len() as f64;
                for i in serbest {
                    y[i] = (y[i] + pay).clamp(SINIR[i].0, SINIR[i].1);
                }
            }
            for v in y.iter_mut() {
                *v = (*v * 1000.0).round() / 1000.0;
            }
            // Yuvarlama artığını en büyük bileşene ekle.
            let artik = 1.0 - y.iter().sum::<f64>();
            let en = (0..5)
                .max_by(|a, b| y[*a].partial_cmp(&y[*b]).unwrap())
                .unwrap_or(0);
            y[en] = ((y[en] + artik) * 1000.0).round() / 1000.0;
            let degisti: Vec<String> = (0..5)
                .filter(|i| (y[*i] - w[*i]).abs() >= 0.005)
                .map(|i| format!("{} ağırlığı {:.3} -> {:.3}", AD[i], w[i], y[i]))
                .collect();
            if !degisti.is_empty() {
                yeni.agirlik = Agirliklar::diziden(y);
                degisiklikler.extend(degisti);
            }
        }
    }
    if degisiklikler.is_empty() || yeni.dogrula().is_err() {
        return None;
    }
    Some(AyarOnerisi {
        yeni,
        degisiklikler,
    })
}

// ---------------------------------------------------------------- testler

#[cfg(test)]
mod testler {
    use super::*;

    fn sure() -> SureTablosu {
        SureTablosu::json_den(
            r#"{"sureler_dk":{"a":30,"b":5},"gunluk_dakika":510,"kapali_gunler":["cumartesi","pazar"]}"#,
        )
        .unwrap()
    }

    fn ayar0() -> PuanAyari {
        PuanAyari {
            c: 0.0,
            deneme_gun: 0,
            ..PuanAyari::default()
        }
    }

    fn olay(usta: &str, no: usize, kod: &str, dk: u32, tarih: &str, saat: &str) -> Kayit {
        Kayit::Olay(Olay {
            usta: usta.into(),
            fis_no: format!("F{no}"),
            kalem_no: 0,
            islem_kodu: Some(kod.into()),
            norm_dk: Some(dk),
            tarih: tarih.into(),
            saat: saat.into(),
        })
    }

    /// 2026-11-02 (pzt) – 11-06 (cum): 5 gün = 2550 dk. 70×30 + 15×5 = 2175 dk (%85,3).
    fn hafta(usta: &str) -> Vec<Kayit> {
        let gunler = [
            "2026-11-02",
            "2026-11-03",
            "2026-11-04",
            "2026-11-05",
            "2026-11-06",
        ];
        (0..85)
            .map(|i| {
                let (kod, dk) = if i < 70 { ("a", 30) } else { ("b", 5) };
                let dakika = 9 * 60 + (i % 60) * 7;
                olay(
                    usta,
                    i,
                    kod,
                    dk,
                    gunler[i % 5],
                    &format!("{:02}:{:02}", dakika / 60, dakika % 60),
                )
            })
            .collect()
    }

    #[test]
    fn hedefe_ulasan_usta_tam_puan_alir() {
        let r = hesapla_cekirdek(
            &ayar0(),
            &sure(),
            &hafta("Ali"),
            &BTreeMap::new(),
            "2026-11-02",
            "2026-11-06",
        )
        .unwrap();
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].mevcut_dk, 2550);
        assert!((r[0].kazanilan_dk - 2175.0).abs() < 1e-9);
        // zamanında bileşeni teslim bilinmediği için önsel (0,85): 10×(0,35+0,35+0,15+0,10×0,85+0,05)
        assert_eq!(r[0].puan, Some(9.85));
    }

    #[test]
    fn tutanaksiz_isaret_yarim_tutanakli_tam_kaynakli_degil_sifir() {
        let mut k = hafta("Ali");
        let teslim: BTreeMap<String, String> = (0..85)
            .map(|i| (format!("F{i}"), "2026-11-30".into()))
            .collect();
        let temiz = hesapla_cekirdek(&ayar0(), &sure(), &k, &teslim, "2026-11-02", "2026-11-06")
            .unwrap()[0]
            .puan;
        assert_eq!(temiz, Some(10.0));
        k.push(Kayit::Isaret(Isaret {
            id: 1,
            fis_no: "F0".into(),
            kalem_no: 0,
            usta: "Ali".into(),
            tur: IsaretTuru::YenidenIs,
            neden: "dikiş".into(),
            tezgah: "Ayşe".into(),
            tarih: "2026-11-05".into(),
            tutanak: None,
        }));
        let beklemede =
            hesapla_cekirdek(&ayar0(), &sure(), &k, &teslim, "2026-11-02", "2026-11-06").unwrap();
        // k = 1 - 0,5/85; ceza = 0,5×30×2 = 30 dk -> oran 2145/2550 = 0,84118 -> v = 0,98962
        let beklenen = 10.0 * (0.35 * (2145.0 / 2550.0 / 0.85) + 0.35 * (1.0 - 0.5 / 85.0) + 0.30);
        assert!(
            (beklemede[0].puan.unwrap() - beklenen).abs() < 0.006,
            "{:?}",
            beklemede[0].puan
        );
        assert_eq!(beklemede[0].beklemede, 1);

        let mut tam = k.clone();
        tam.push(Kayit::Tutanak {
            isaret_id: 1,
            tutanak: Tutanak {
                sonuc: Sonuc::UstaKaynakli,
                agir: false,
                aciklama: "dikiş atladı".into(),
                yonetici: "Yusuf".into(),
                tarih: "2026-11-06".into(),
            },
        });
        let p_tam = hesapla_cekirdek(&ayar0(), &sure(), &tam, &teslim, "2026-11-02", "2026-11-06")
            .unwrap()[0]
            .puan
            .unwrap();
        assert!(p_tam < beklemede[0].puan.unwrap());

        let mut degil = k;
        degil.push(Kayit::Tutanak {
            isaret_id: 1,
            tutanak: Tutanak {
                sonuc: Sonuc::UstaKaynakliDegil,
                agir: true,
                aciklama: "müşteri yanlış ölçü verdi".into(),
                yonetici: "Yusuf".into(),
                tarih: "2026-11-06".into(),
            },
        });
        let r = hesapla_cekirdek(
            &ayar0(),
            &sure(),
            &degil,
            &teslim,
            "2026-11-02",
            "2026-11-06",
        )
        .unwrap();
        assert_eq!(
            r[0].puan,
            Some(10.0),
            "usta kaynaklı değilse puana yazılmaz, ağır olsa bile"
        );
    }

    #[test]
    fn agir_hata_kaliteyi_sinirlar() {
        let mut k = hafta("Ali");
        k.push(Kayit::Isaret(Isaret {
            id: 1,
            fis_no: "F0".into(),
            kalem_no: 0,
            usta: "Ali".into(),
            tur: IsaretTuru::YenidenIs,
            neden: "kumaş".into(),
            tezgah: "Ayşe".into(),
            tarih: "2026-11-05".into(),
            tutanak: None,
        }));
        k.push(Kayit::Tutanak {
            isaret_id: 1,
            tutanak: Tutanak {
                sonuc: Sonuc::UstaKaynakli,
                agir: true,
                aciklama: "kumaş yandı".into(),
                yonetici: "Yusuf".into(),
                tarih: "2026-11-06".into(),
            },
        });
        let r = hesapla_cekirdek(
            &ayar0(),
            &sure(),
            &k,
            &BTreeMap::new(),
            "2026-11-02",
            "2026-11-06",
        )
        .unwrap();
        assert!((r[0].ham.kalite - 0.6).abs() < 1e-9);
    }

    #[test]
    fn deneme_doneminde_puan_yok() {
        let ayar = PuanAyari {
            deneme_gun: 28,
            ..ayar0()
        };
        let r = hesapla_cekirdek(
            &ayar,
            &sure(),
            &hafta("Yeni"),
            &BTreeMap::new(),
            "2026-11-02",
            "2026-11-06",
        )
        .unwrap();
        assert!(r[0].deneme && r[0].puan.is_none());
        assert!(r[0].yorumlar[0].contains("Deneme dönemi"));
    }

    #[test]
    fn kucuk_ornek_ekip_ortalamasina_yaklasir() {
        // Ali 85 iş (iyi), Veli 2 iş, hiç kazanç yok gibi düşük verimlilik: düzeltme Veli'yi ortalamaya çeker.
        let mut k = hafta("Ali");
        k.push(olay("Veli", 900, "b", 5, "2026-11-02", "10:00"));
        k.push(olay("Veli", 901, "b", 5, "2026-11-03", "10:00"));
        let ayar = PuanAyari { c: 30.0, ..ayar0() };
        let r = hesapla_cekirdek(
            &ayar,
            &sure(),
            &k,
            &BTreeMap::new(),
            "2026-11-02",
            "2026-11-06",
        )
        .unwrap();
        let veli = r.iter().find(|p| p.usta == "Veli").unwrap();
        assert!(veli.ham.verimlilik < 0.1);
        assert!(
            veli.bilesenler.verimlilik > 0.6,
            "ortalamaya çekilmeli: {}",
            veli.bilesenler.verimlilik
        );
    }

    #[test]
    fn toplu_okutma_duzeni_dusurur_ve_yorum_yazar() {
        let mut k: Vec<Kayit> = (0..10)
            .map(|i| olay("Ali", i, "a", 30, "2026-11-02", &format!("17:{:02}", i)))
            .collect();
        k.push(olay("Ali", 99, "a", 30, "2026-11-03", "09:00"));
        let r = hesapla_cekirdek(
            &ayar0(),
            &sure(),
            &k,
            &BTreeMap::new(),
            "2026-11-02",
            "2026-11-06",
        )
        .unwrap();
        assert!((r[0].ham.duzen - (1.0 - 10.0 / 11.0)).abs() < 1e-9);
        assert!(r[0].yorumlar.iter().any(|y| y.contains("toplu okutulmuş")));
    }

    #[test]
    fn bugun_bitmemis_gun_mevcut_dakikayi_dusurmez() {
        // Aralık 11-02..11-06 (pzt-cum). Usta 11-06'da henüz okutma yapmamış: o gün sayılmaz (4 gün).
        let k: Vec<Kayit> = hafta("Ali")
            .into_iter()
            .filter(|x| !matches!(x, Kayit::Olay(o) if o.tarih == "2026-11-06"))
            .collect();
        let r = hesapla_cekirdek(
            &ayar0(),
            &sure(),
            &k,
            &BTreeMap::new(),
            "2026-11-02",
            "2026-11-06",
        )
        .unwrap();
        assert_eq!(r[0].mevcut_dk, 4 * 510);
        // Son günde okutma varsa o gün sayılır.
        let r = hesapla_cekirdek(
            &ayar0(),
            &sure(),
            &hafta("Ali"),
            &BTreeMap::new(),
            "2026-11-02",
            "2026-11-06",
        )
        .unwrap();
        assert_eq!(r[0].mevcut_dk, 5 * 510);
    }

    #[test]
    fn izin_gunu_mevcut_dakikadan_duser() {
        let mut k = hafta("Ali");
        k.push(Kayit::Izin {
            usta: "Ali".into(),
            tarih: "2026-11-04".into(),
            neden: "rapor".into(),
            yonetici: "Yusuf".into(),
        });
        let r = hesapla_cekirdek(
            &ayar0(),
            &sure(),
            &k,
            &BTreeMap::new(),
            "2026-11-02",
            "2026-11-06",
        )
        .unwrap();
        assert_eq!(r[0].mevcut_dk, 4 * 510);
    }

    #[test]
    fn ozellik_kapaliyken_hesap_reddedilir() {
        assert!(hesapla(
            &ayar0(),
            &sure(),
            &[],
            &BTreeMap::new(),
            "2026-11-02",
            "2026-11-06"
        )
        .is_err());
        assert!(anomaliler(&ayar0(), &sure(), &[], "2026-11-06").is_err());
    }

    #[test]
    fn defter_gidis_donus_ve_kurallar() {
        let d = std::env::temp_dir().join(format!("taa-puan-defter-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let def = PuanDefteri::dizinden(&d);
        let o = Olay {
            usta: "Ali".into(),
            fis_no: "F1".into(),
            kalem_no: 0,
            islem_kodu: Some("a".into()),
            norm_dk: Some(30),
            tarih: "2026-11-02".into(),
            saat: "10:00".into(),
        };
        // Özellik varsayılan KAPALI: önce reddedilir, sonra açılınca kabul edilir (bu env yalnız bu testte kullanılır).
        assert!(def.olay_ekle(o.clone()).unwrap_err().contains("kapatılmış"));
        std::env::set_var("TAA_OZELLIK_QR_URETIM", "1");
        def.olay_ekle(o.clone()).unwrap();
        assert!(def.olay_ekle(o).unwrap_err().contains("zaten okutulmuş"));
        let id = def
            .isaretle(Isaret {
                id: 0,
                fis_no: "F1".into(),
                kalem_no: 0,
                usta: "Ali".into(),
                tur: IsaretTuru::Sikayet,
                neden: "boy kısa".into(),
                tezgah: "Ayşe".into(),
                tarih: "2026-11-03".into(),
                tutanak: None,
            })
            .unwrap();
        assert_eq!(id, 1);
        let t = |a: &str| Tutanak {
            sonuc: Sonuc::Kismen { yuzde: 50 },
            agir: false,
            aciklama: a.into(),
            yonetici: "Yusuf".into(),
            tarih: "2026-11-04".into(),
        };
        assert!(
            def.tutanak_yaz(id, t("  ")).is_err(),
            "açıklamasız tutanak olmaz"
        );
        assert!(
            def.tutanak_yaz(99, t("x")).is_err(),
            "olmayan işarete tutanak olmaz"
        );
        def.tutanak_yaz(id, t("yarı yarıya ustanın hatası"))
            .unwrap();
        let k = def.oku().unwrap();
        assert_eq!(k.len(), 3);
        assert_eq!(
            birlestir(&k).isaretler[0].tutanak.as_ref().unwrap().sonuc,
            Sonuc::Kismen { yuzde: 50 }
        );
        std::env::remove_var("TAA_OZELLIK_QR_URETIM");
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn ani_dusus_yalniz_yonetici_anomalisi_olur() {
        // 11 iş günü 0,85/0,95 dalgalı, sonra 7 gün (5 iş günü) çok düşük.
        let mut k = Vec::new();
        let mut no = 0;
        let basla = gun_no("2026-10-12").unwrap(); // pazartesi
        for gun in 0..30 {
            let n = basla + gun;
            let t = gun_tarih(n);
            let dk = if gun < 21 {
                if gun % 2 == 0 {
                    435
                } else {
                    485
                } // %85, %95
            } else {
                200
            };
            if sure().kapali_mi(n) {
                continue;
            }
            no += 1;
            k.push(olay("Ali", no, "a", dk, &t, "11:00"));
        }
        let r = anomali_cekirdek(&PuanAyari::default(), &sure(), &k, "2026-11-11").unwrap();
        assert!(r.iter().any(|a| a.tur == "verimlilik-dusus"), "{r:?}");
    }

    fn sahte_puan(usta: &str, oran: f64, b: [f64; 5]) -> UstaPuan {
        UstaPuan {
            usta: usta.into(),
            puan: Some(8.0),
            deneme: false,
            is_adedi: 100,
            kazanilan_dk: 0.0,
            mevcut_dk: 0,
            verimlilik_orani: oran,
            hedef: 0.85,
            bilesenler: Bilesenler::diziden(b),
            ham: Bilesenler::diziden(b),
            beklemede: 0,
            suresiz_is: 0,
            yorumlar: vec![],
        }
    }

    #[test]
    fn otomatik_ayar_sinirli_toplam_bir_ve_aralikli() {
        let ayar = PuanAyari::default();
        let p = vec![
            sahte_puan("A", 0.60, [0.7, 0.99, 1.0, 0.9, 1.0]),
            sahte_puan("B", 0.66, [0.8, 0.90, 1.0, 0.8, 1.0]),
            sahte_puan("C", 0.72, [0.9, 0.80, 1.0, 1.0, 1.0]),
        ];
        // veri yetersiz
        assert!(gozden_gecir(&ayar, &p, 100, 60, &[], "2026-12-01").is_none());
        assert!(gozden_gecir(&ayar, &p, 500, 10, &[], "2026-12-01").is_none());
        let o = gozden_gecir(&ayar, &p, 500, 60, &[], "2026-12-01").expect("öneri");
        o.yeni.dogrula().unwrap();
        // medyan 0,66×1,1 = 0,726; adım 0,2×(0,726−0,85) = −0,0248
        assert!(
            (o.yeni.hedef_oran - 0.825).abs() < 0.002,
            "{}",
            o.yeni.hedef_oran
        );
        for (a, b) in ayar.agirlik.dizi().iter().zip(o.yeni.agirlik.dizi()) {
            assert!((a - b).abs() <= 0.031, "adım sınırı aşıldı");
        }
        assert!(!o.degisiklikler.is_empty());
        // son ayardan beri 28 gün dolmadı
        let k = vec![Kayit::AyarDegisti {
            tarih: "2026-11-20".into(),
            degisiklikler: vec![],
        }];
        assert!(gozden_gecir(&ayar, &p, 500, 60, &k, "2026-12-01").is_none());
        assert!(gozden_gecir(&ayar, &p, 500, 60, &k, "2026-12-20").is_some());
        // kapalıysa öneri yok
        let kapali = PuanAyari {
            otomatik: false,
            ..ayar
        };
        assert!(gozden_gecir(&kapali, &p, 500, 60, &[], "2026-12-01").is_none());
    }

    #[test]
    fn ayar_dogrulama_ve_dosya() {
        assert!(PuanAyari::default().dogrula().is_ok());
        let mut a = PuanAyari::default();
        a.agirlik.verimlilik = 0.5;
        assert!(a.dogrula().is_err());
        let a = PuanAyari {
            hedef_oran: 1.5,
            ..PuanAyari::default()
        };
        assert!(a.dogrula().is_err());
        let d = std::env::temp_dir().join(format!("taa-puan-ayar-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        assert_eq!(PuanAyari::dizinden(&d).unwrap(), PuanAyari::default());
        let mut b = PuanAyari::default();
        b.hedef_islem.insert("a".into(), 0.75);
        b.kaydet(&d).unwrap();
        assert_eq!(PuanAyari::dizinden(&d).unwrap(), b);
        std::fs::write(d.join("puan-ayar.json"), "{bozuk").unwrap();
        assert!(PuanAyari::dizinden(&d).is_err());
        std::fs::remove_dir_all(&d).ok();
    }
}
