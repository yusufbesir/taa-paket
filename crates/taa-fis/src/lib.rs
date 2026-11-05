//! taa-fis (TaaFiş): terzi fişi modeli, eklemeli defter (JSONL), durum takibi, termal (ESC/POS) ve düz metin çıktı.
//! Telefon sayfası, QR eşleştirme, OCR ve ağ sunucusu bu crate'in DIŞINDADIR (sonraki aşamalar); burada saf çekirdek var.
//! ESC/POS çıktısı gerçek yazıcıda denenmedi; yazıcı kod sayfası CP857 (Türkçe) varsayılır.
//! Defter silmez: durum değişikliği yeni satırdır. Özellik anahtarı: "fis" (varsayılan açık).
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const OZELLIK: &str = "fis";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Durum {
    Bekliyor,
    /// Ürün terzihaneye verildi.
    Terzihanede,
    Hazir,
    Teslim,
    Iptal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kalem {
    /// Ürün (ceket, pantolon...).
    pub urun: String,
    /// Yapılacak işlem (paça kısalt, bel daralt...). Katalogdan seçildiyse katalog adı.
    pub islem: String,
    #[serde(default)]
    pub not: String,
    /// Katalog işlem kodu (taa-terzi); yoksa serbest metin işlem.
    #[serde(default)]
    pub islem_kodu: Option<String>,
    /// Düz paça / duble paça gibi seçenek.
    #[serde(default)]
    pub varyant: Option<String>,
    /// Yapılacak ölçü, milimetre (2,5 cm = 25).
    #[serde(default)]
    pub olcu_mm: Option<i32>,
    /// İçeride bırakılan pay, milimetre (tekrar uzatma için).
    #[serde(default)]
    pub pay_mm: Option<i32>,
    /// Sınır aşımında usta onayı verildi mi?
    #[serde(default)]
    pub usta_onayli: bool,
    /// Ölçü iğneyle işaretlendi (cm bilinmiyor).
    #[serde(default)]
    pub igneli: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeslimDegisti {
    pub eski: String,
    pub yeni: String,
    pub neden: String,
    pub tarih: String,
}

/// Telefonla atılan imza: kim (eşleşmiş telefonun kullanıcısı), rol (usta, teslim...), tarih.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Imza {
    pub kim: String,
    pub rol: String,
    pub tarih: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fis {
    pub no: String,
    pub tarih: String,
    pub musteri: String,
    pub telefon: String,
    pub kalemler: Vec<Kalem>,
    #[serde(default)]
    pub olcu_notu: String,
    #[serde(default)]
    pub prova: String,
    pub teslim: String,
    /// HH:MM
    #[serde(default = "varsayilan_saat")]
    pub teslim_saati: String,
    pub ucret_kurus: i64,
    pub on_odeme_kurus: i64,
    #[serde(default)]
    pub usta: String,
    #[serde(default)]
    pub satis_danismani: String,
    #[serde(default)]
    pub olcu_alan: String,
    /// Kargo ile gönderilecek mi? Fişte büyük yazılır (not kâğıdı düşebildiği için).
    #[serde(default)]
    pub kargo: bool,
    #[serde(default)]
    pub kargo_notu: String,
    /// Kargo işaretliyse zorunlu: alıcı ad-soyad, açık adres, alıcı telefonu.
    #[serde(default)]
    pub kargo_ad: String,
    #[serde(default)]
    pub kargo_adres: String,
    #[serde(default)]
    pub kargo_telefon: String,
    /// Müşteri "hazır" bilgilendirmesine izin verdi mi? (KVKK; varsayılan hayır)
    #[serde(default)]
    pub bildirim_izni: bool,
    #[serde(default)]
    pub teslim_degisiklikleri: Vec<TeslimDegisti>,
    pub durum: Durum,
    /// Hazır ürünün asılı olduğu stant/raf kodu (QR okutularak yazılır).
    #[serde(default)]
    pub raf: String,
    #[serde(default)]
    pub imzalar: Vec<Imza>,
}

fn varsayilan_saat() -> String {
    "17:00".into()
}

impl Fis {
    pub fn kalan_kurus(&self) -> i64 {
        self.ucret_kurus - self.on_odeme_kurus
    }
}

/// Yeni fiş girdisi (numara ve durum defter tarafından verilir).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Girdi {
    #[serde(default)]
    pub tarih: String,
    pub musteri: String,
    pub telefon: String,
    pub kalemler: Vec<Kalem>,
    #[serde(default)]
    pub olcu_notu: String,
    #[serde(default)]
    pub prova: String,
    pub teslim: String,
    #[serde(default = "varsayilan_saat")]
    pub teslim_saati: String,
    pub ucret_kurus: i64,
    #[serde(default)]
    pub on_odeme_kurus: i64,
    #[serde(default)]
    pub usta: String,
    #[serde(default)]
    pub satis_danismani: String,
    #[serde(default)]
    pub olcu_alan: String,
    /// Kargo ile gönderilecek mi? Fişte büyük yazılır (not kâğıdı düşebildiği için).
    #[serde(default)]
    pub kargo: bool,
    #[serde(default)]
    pub kargo_notu: String,
    /// Kargo işaretliyse zorunlu: alıcı ad-soyad, açık adres, alıcı telefonu.
    #[serde(default)]
    pub kargo_ad: String,
    #[serde(default)]
    pub kargo_adres: String,
    #[serde(default)]
    pub kargo_telefon: String,
    /// Müşteri "hazır" bilgilendirmesine izin verdi mi? (KVKK; varsayılan hayır)
    #[serde(default)]
    pub bildirim_izni: bool,
}

pub fn tarih_gecerli(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && t.char_indices()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && matches!(t[5..7].parse::<u32>(), Ok(1..=12))
        && matches!(t[8..10].parse::<u32>(), Ok(1..=31))
}

/// Yalnız rakamlar kalır; 10 hane (5xx...) ya da 0 ile 11 hane kabul edilir. "05321234567" biçimine çevirir.
pub fn telefon_duzelt(t: &str) -> Result<String, String> {
    let r: String = t.chars().filter(char::is_ascii_digit).collect();
    let r = r
        .strip_prefix("90")
        .filter(|x| x.len() == 10)
        .map(str::to_string)
        .unwrap_or(r);
    match r.len() {
        10 if r.starts_with('5') => Ok(format!("0{r}")),
        11 if r.starts_with("05") => Ok(r),
        _ => Err(format!("geçersiz cep telefonu: '{t}'")),
    }
}

/// Fiş girdisini denetler; telefonu düzeltilmiş halde ve kalem uyarılarıyla döndürür.
pub fn dogrula(
    g: &Girdi,
    katalog: &taa_terzi::Katalog,
    serbest_islem: bool,
    olcu_zorunlu: bool,
) -> Result<(String, Vec<String>), String> {
    if g.musteri.trim().is_empty() {
        return Err("müşteri adı gerekli".into());
    }
    if g.satis_danismani.trim().is_empty() || g.olcu_alan.trim().is_empty() {
        return Err("satış danışmanı ve ölçü alan kişi yazılmalı".into());
    }
    if !saat_gecerli(&g.teslim_saati) {
        return Err("teslim saati SS:DD olmalı".into());
    }
    let mut uyarilar = Vec::new();
    for k in &g.kalemler {
        match &k.islem_kodu {
            Some(kod) => uyarilar.extend(katalog.kontrol(
                kod,
                k.olcu_mm,
                k.varyant.as_deref(),
                k.usta_onayli,
                olcu_zorunlu,
                k.igneli,
            )?),
            None if !serbest_islem => {
                return Err("işlem listeden seçilmeli (serbest işlem kapalı)".into())
            }
            None => {}
        }
        if matches!(k.pay_mm, Some(p) if p < 0) {
            return Err("pay negatif olamaz".into());
        }
    }
    if g.kalemler.is_empty()
        || g.kalemler
            .iter()
            .any(|k| k.urun.trim().is_empty() || k.islem.trim().is_empty())
    {
        return Err("her kalemde ürün ve işlem yazılmalı (en az bir kalem)".into());
    }
    if !tarih_gecerli(&g.tarih)
        || !tarih_gecerli(&g.teslim)
        || (!g.prova.is_empty() && !tarih_gecerli(&g.prova))
    {
        return Err("tarihler YYYY-AA-GG olmalı".into());
    }
    if g.teslim < g.tarih || (!g.prova.is_empty() && (g.prova < g.tarih || g.prova > g.teslim)) {
        return Err(
            "prova/teslim tarihi fiş tarihinden önce olamaz; prova teslimden sonra olamaz".into(),
        );
    }
    if g.ucret_kurus < 0 || g.on_odeme_kurus < 0 || g.on_odeme_kurus > g.ucret_kurus {
        return Err("ücret/ön ödeme geçersiz (ön ödeme ücretten fazla olamaz)".into());
    }
    Ok((telefon_duzelt(&g.telefon)?, uyarilar))
}

pub fn saat_gecerli(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && matches!(s[0..2].parse::<u32>(), Ok(0..=23))
        && matches!(s[3..5].parse::<u32>(), Ok(0..=59))
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "lowercase")]
#[allow(clippy::large_enum_variant)]
enum Kayit {
    Fis(Fis),
    Durum {
        no: String,
        durum: Durum,
        tarih: String,
    },
    Teslim {
        no: String,
        degisiklik: TeslimDegisti,
        saat: String,
    },
    Raf {
        no: String,
        raf: String,
        kim: String,
        tarih: String,
    },
    Imza {
        no: String,
        imza: Imza,
    },
}

pub struct Defter<'a> {
    pub yol: &'a Path,
}

fn kapali() -> Result<(), String> {
    if taa_ayar::ozellik_acik(OZELLIK, true) {
        Ok(())
    } else {
        Err(taa_ayar::kapali_mesaji(OZELLIK))
    }
}

impl Defter<'_> {
    fn kayitlar(&self) -> Result<Vec<Kayit>, String> {
        taa_dosya::satirlari_oku(self.yol)
            .map_err(|e| e.to_string())?
            .iter()
            .map(|l| serde_json::from_str(l).map_err(|e| format!("defter satırı bozuk: {e}")))
            .collect()
    }

    /// Fişlerin güncel durumları (defter sırasıyla).
    pub fn fisler(&self) -> Result<Vec<Fis>, String> {
        let mut f: Vec<Fis> = Vec::new();
        for k in self.kayitlar()? {
            match k {
                Kayit::Fis(x) => f.push(x),
                Kayit::Durum { no, durum, .. } => {
                    if let Some(x) = f.iter_mut().find(|x| x.no == no) {
                        x.durum = durum;
                    }
                }
                Kayit::Teslim {
                    no,
                    degisiklik,
                    saat,
                } => {
                    if let Some(x) = f.iter_mut().find(|x| x.no == no) {
                        x.teslim = degisiklik.yeni.clone();
                        x.teslim_saati = saat;
                        x.teslim_degisiklikleri.push(degisiklik);
                    }
                }
                Kayit::Raf { no, raf, .. } => {
                    if let Some(x) = f.iter_mut().find(|x| x.no == no) {
                        x.raf = raf;
                    }
                }
                Kayit::Imza { no, imza } => {
                    if let Some(x) = f.iter_mut().find(|x| x.no == no) {
                        x.imzalar.push(imza);
                    }
                }
            }
        }
        Ok(f)
    }

    /// Ürünün stant/raf yerini yazar (QR okutulunca). Eski yer silinmez, defterde kalır.
    pub fn raf_ata(&self, no: &str, raf: &str, kim: &str, tarih: &str) -> Result<(), String> {
        kapali()?;
        if !taa_ayar::ozellik_acik("fis-raf", true) {
            return Err(taa_ayar::kapali_mesaji("fis-raf"));
        }
        let raf = raf.trim().to_uppercase();
        if raf.is_empty()
            || raf.chars().count() > 20
            || !raf
                .chars()
                .all(|c| c.is_alphanumeric() || c == '-' || c == ' ')
        {
            return Err("stant/raf kodu 1-20 harf, rakam ya da tire olmalı (örn. A-12)".into());
        }
        let f = self.bul(no)?.ok_or("fiş yok")?;
        if matches!(f.durum, Durum::Teslim | Durum::Iptal) {
            return Err("teslim edilmiş ya da iptal fişe raf yazılamaz".into());
        }
        let s = serde_json::to_string(&Kayit::Raf {
            no: no.into(),
            raf,
            kim: kim.trim().into(),
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    /// Telefondaki kullanıcı adına imza atar (ör. usta işi bitirince). Aynı kişi aynı rolde bir kez imzalar.
    pub fn imzala(&self, no: &str, kim: &str, rol: &str, tarih: &str) -> Result<(), String> {
        kapali()?;
        if !taa_ayar::ozellik_acik("fis-imza", true) {
            return Err(taa_ayar::kapali_mesaji("fis-imza"));
        }
        let (kim, rol) = (kim.trim(), rol.trim());
        if kim.is_empty() || !["usta", "teslim-eden", "teslim-alan"].contains(&rol) {
            return Err("imza: kullanıcı ve rol (usta, teslim-eden, teslim-alan) gerekli".into());
        }
        let f = self.bul(no)?.ok_or("fiş yok")?;
        if f.durum == Durum::Iptal {
            return Err("iptal fiş imzalanamaz".into());
        }
        if f.imzalar.iter().any(|i| i.kim == kim && i.rol == rol) {
            return Err(format!("{kim} bu fişi '{rol}' olarak zaten imzaladı"));
        }
        let s = serde_json::to_string(&Kayit::Imza {
            no: no.into(),
            imza: Imza {
                kim: kim.into(),
                rol: rol.into(),
                tarih: tarih.into(),
            },
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    pub fn bul(&self, no: &str) -> Result<Option<Fis>, String> {
        Ok(self.fisler()?.into_iter().find(|f| f.no == no))
    }

    pub fn ekle(&self, g: Girdi) -> Result<Fis, String> {
        self.ekle_uyarili(g).map(|(f, _)| f)
    }

    /// Fişi ekler; kalem uyarılarını da döndürür. Kurallar `terzi-kurallari.json` (defterin klasöründe) ya da gömülü katalogdur.
    pub fn ekle_uyarili(&self, g: Girdi) -> Result<(Fis, Vec<String>), String> {
        kapali()?;
        let katalog = taa_terzi::Katalog::dizinden(self.yol.parent().unwrap_or(Path::new(".")))?;
        let (telefon, uyarilar) = dogrula(
            &g,
            &katalog,
            taa_ayar::ozellik_acik("serbest-islem", true),
            taa_ayar::ozellik_acik("olcu-zorunlu", false),
        )?;
        let kargo = g.kargo && taa_ayar::ozellik_acik("kargo-alani", true);
        let mut kargo_telefon = String::new();
        if kargo {
            if g.kargo_ad.split_whitespace().count() < 2 {
                return Err("kargo: alıcının ad ve soyadı yazılmalı".into());
            }
            if g.kargo_adres.trim().chars().count() < 15 {
                return Err("kargo: açık adres (mahalle, sokak, no, ilçe/il) yazılmalı".into());
            }
            kargo_telefon = telefon_duzelt(&g.kargo_telefon)
                .map_err(|_| "kargo: alıcı telefon numarası geçerli değil".to_string())?;
        }
        let gun = g.tarih.replace('-', "");
        let sira = self.fisler()?.iter().filter(|f| f.tarih == g.tarih).count() + 1;
        let fis = Fis {
            no: format!("F-{gun}-{sira:03}"),
            tarih: g.tarih,
            musteri: g.musteri.trim().into(),
            telefon,
            kalemler: g.kalemler,
            olcu_notu: g.olcu_notu.trim().into(),
            prova: g.prova,
            teslim: g.teslim,
            teslim_saati: g.teslim_saati,
            ucret_kurus: g.ucret_kurus,
            on_odeme_kurus: g.on_odeme_kurus,
            usta: g.usta.trim().into(),
            satis_danismani: g.satis_danismani.trim().into(),
            olcu_alan: g.olcu_alan.trim().into(),
            kargo,
            kargo_notu: g.kargo_notu.trim().into(),
            kargo_ad: if kargo {
                g.kargo_ad.trim().into()
            } else {
                String::new()
            },
            kargo_adres: if kargo {
                g.kargo_adres.trim().into()
            } else {
                String::new()
            },
            kargo_telefon,
            bildirim_izni: g.bildirim_izni,
            teslim_degisiklikleri: Vec::new(),
            durum: Durum::Bekliyor,
            raf: String::new(),
            imzalar: Vec::new(),
        };
        let s = serde_json::to_string(&Kayit::Fis(fis.clone())).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())?;
        Ok((fis, uyarilar))
    }

    /// Teslim tarihini/saatini değiştirir; eski değer silinmez, neden zorunludur.
    pub fn teslim_degistir(
        &self,
        no: &str,
        yeni: &str,
        yeni_saat: &str,
        neden: &str,
        tarih: &str,
    ) -> Result<(), String> {
        kapali()?;
        let f = self.bul(no)?.ok_or_else(|| format!("fiş yok: {no}"))?;
        if matches!(f.durum, Durum::Teslim | Durum::Iptal) {
            return Err("teslim edilmiş ya da iptal fişin teslim tarihi değiştirilemez".into());
        }
        if !tarih_gecerli(yeni) || !tarih_gecerli(tarih) || !saat_gecerli(yeni_saat) {
            return Err("tarih YYYY-AA-GG, saat SS:DD olmalı".into());
        }
        if yeni < f.tarih.as_str() {
            return Err("yeni teslim tarihi fiş tarihinden önce olamaz".into());
        }
        if neden.trim().is_empty() {
            return Err("değişiklik nedeni yazılmalı".into());
        }
        let s = serde_json::to_string(&Kayit::Teslim {
            no: no.into(),
            degisiklik: TeslimDegisti {
                eski: f.teslim,
                yeni: yeni.into(),
                neden: neden.trim().into(),
                tarih: tarih.into(),
            },
            saat: yeni_saat.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    /// Durumu ilerletir: Bekliyor -> (Terzihanede) -> Hazir -> Teslim; Iptal Teslim'den önce. Geri dönüş yok.
    pub fn durum_ata(&self, no: &str, yeni: Durum, tarih: &str) -> Result<(), String> {
        kapali()?;
        let f = self.bul(no)?.ok_or_else(|| format!("fiş yok: {no}"))?;
        let izin = matches!(
            (f.durum, yeni),
            (Durum::Bekliyor, Durum::Terzihanede | Durum::Hazir)
                | (Durum::Terzihanede, Durum::Hazir)
                | (Durum::Hazir, Durum::Teslim)
                | (
                    Durum::Bekliyor | Durum::Terzihanede | Durum::Hazir,
                    Durum::Iptal
                )
        );
        if !izin {
            return Err(format!(
                "{:?} durumundan {:?} durumuna geçilemez",
                f.durum, yeni
            ));
        }
        if !tarih_gecerli(tarih) {
            return Err("tarih YYYY-AA-GG olmalı".into());
        }
        let s = serde_json::to_string(&Kayit::Durum {
            no: no.into(),
            durum: yeni,
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    /// Teslim tarihi geçmiş ve teslim edilmemiş fişler.
    pub fn geciken(&self, bugun: &str) -> Result<Vec<Fis>, String> {
        Ok(self
            .fisler()?
            .into_iter()
            .filter(|f| {
                matches!(f.durum, Durum::Bekliyor | Durum::Terzihanede | Durum::Hazir)
                    && f.teslim.as_str() < bugun
            })
            .collect())
    }
}

pub fn para(k: i64) -> String {
    let tl = (k.unsigned_abs() / 100).to_string();
    let mut g = String::new();
    for (i, c) in tl.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            g.push('.');
        }
        g.push(c);
    }
    format!(
        "{}{},{:02}",
        if k < 0 { "-" } else { "" },
        g.chars().rev().collect::<String>(),
        k.unsigned_abs() % 100
    )
}

fn gun_metni(t: &str) -> String {
    format!("{}.{}.{}", &t[8..10], &t[5..7], &t[0..4])
}

fn sar(metin: &str, genislik: usize) -> Vec<String> {
    let mut satirlar = Vec::new();
    let mut cur = String::new();
    for kelime in metin.split_whitespace() {
        let kl = kelime.chars().count();
        if !cur.is_empty() && cur.chars().count() + 1 + kl > genislik {
            satirlar.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(kelime);
        while cur.chars().count() > genislik {
            let (a, b): (String, String) = (
                cur.chars().take(genislik).collect(),
                cur.chars().skip(genislik).collect(),
            );
            satirlar.push(a);
            cur = b;
        }
    }
    if !cur.is_empty() {
        satirlar.push(cur);
    }
    satirlar
}

/// Sabit genişlikli (örn. 42 sütun) düz metin fiş.
pub fn duz_metin(f: &Fis, genislik: usize) -> String {
    duz_metin_nusha(f, genislik, "")
}

/// Aynı fiş, üstte nüsha etiketiyle (örn. "TERZİHANE NÜSHASI", "MÜŞTERİ NÜSHASI").
pub fn duz_metin_nusha(f: &Fis, genislik: usize, nusha: &str) -> String {
    let g = genislik.max(24);
    let mut o = String::new();
    for (stil, m) in satirlar(f, nusha) {
        match stil {
            Stil::Ayrac => o.push_str(&"-".repeat(g)),
            Stil::Cift => o.push_str(&"=".repeat(g)),
            Stil::Ham => o.push_str(&m),
            _ => {
                for l in sar(&m, g) {
                    if stil.ortali() {
                        let bos = g.saturating_sub(l.chars().count()) / 2;
                        o.push_str(&" ".repeat(bos));
                    }
                    o.push_str(&l);
                    o.push('\n');
                }
                continue;
            }
        }
        o.push('\n');
    }
    o
}

/// Müşteri nüshasında teslim imza bloğu ve ürün nüshasında alt bilgi cümleleri basılmaz (kâğıt tasarrufu);
/// etiketsiz (tek) çıktıda ikisi de basılır.
fn musteri_nushasi(nusha: &str) -> bool {
    nusha.starts_with("MÜŞTERİ")
}
fn terzihane_nushasi(nusha: &str) -> bool {
    nusha.starts_with("TERZİHANE")
}

/// Fişteki bir satırın görünümü. Yazıcıda kalın, büyük (2x) ve küçük (Font B) yazı ile ortalama kullanılır;
/// düz metinde yalnız ortalama ve çizgiler yansır.
#[derive(Clone, Copy, PartialEq)]
enum Stil {
    Normal,
    Kalin,
    OrtaKalin,
    OrtaKucuk,
    OrtaBuyuk,
    Ham,
    Ayrac,
    Cift,
}

impl Stil {
    fn ortali(self) -> bool {
        matches!(self, Stil::OrtaKalin | Stil::OrtaKucuk | Stil::OrtaBuyuk)
    }
}

/// Fişin satırları (ham, sarılmamış). Fiyat/ödeme bilgisi basılmaz: ödeme alınmadan işlem yapılmaz.
fn satirlar(f: &Fis, nusha: &str) -> Vec<(Stil, String)> {
    use Stil::*;
    let mut v: Vec<(Stil, String)> = Vec::new();
    for (i, l) in baslik().lines().enumerate() {
        v.push((if i == 0 { OrtaKalin } else { OrtaKucuk }, l.to_string()));
    }
    v.push((Cift, String::new()));
    v.push((OrtaKucuk, "TADİLAT FİŞİ".into()));
    v.push((OrtaBuyuk, f.no.clone()));
    if !nusha.is_empty() {
        v.push((OrtaKalin, nusha.to_string()));
    }
    v.push((Normal, format!("Fiş Tarihi: {}", gun_metni(&f.tarih))));
    v.push((Ayrac, String::new()));
    v.push((Kalin, format!("Müşteri Adı: {}", f.musteri)));
    v.push((Normal, format!("Telefonu: {}", f.telefon)));
    v.push((Ayrac, String::new()));
    v.push((OrtaKucuk, "ÜRÜN TESLİM TARİHİ".into()));
    v.push((
        OrtaBuyuk,
        format!("{} {}", gun_metni(&f.teslim), f.teslim_saati),
    ));
    if !f.teslim_degisiklikleri.is_empty() {
        v.push((
            OrtaKucuk,
            format!(
                "(teslim tarihi {} kez değişti)",
                f.teslim_degisiklikleri.len()
            ),
        ));
    }
    if !f.prova.is_empty() {
        v.push((Normal, format!("Prova: {}", gun_metni(&f.prova))));
    }
    v.push((Ayrac, String::new()));
    v.push((Kalin, "İŞLEM DETAYI".into()));
    for k in &f.kalemler {
        let mut satir = format!("* {} - {}", k.urun, k.islem);
        if let Some(x) = &k.varyant {
            satir.push_str(&format!(" [{x}]"));
        }
        if let Some(m) = k.olcu_mm {
            satir.push_str(&format!(" {} cm", cm_metni(m)));
        }
        if let Some(p) = k.pay_mm.filter(|p| *p > 0) {
            satir.push_str(&format!(" (pay {} cm)", cm_metni(p)));
        }
        if k.igneli {
            satir.push_str(" [İĞNELİ]");
        }
        if k.usta_onayli {
            satir.push_str(" [USTA ONAYLI]");
        }
        v.push((Normal, satir));
        if !k.not.is_empty() {
            v.push((Normal, format!("  ({})", k.not)));
        }
    }
    if !f.olcu_notu.is_empty() {
        v.push((Normal, format!("Ölçü: {}", f.olcu_notu)));
    }
    if f.kargo {
        v.push((Ayrac, String::new()));
        v.push((OrtaKalin, "*** KARGO YAPILACAK ***".into()));
        v.push((Normal, format!("Alıcı: {}", f.kargo_ad)));
        v.push((Normal, format!("Adres: {}", f.kargo_adres)));
        v.push((Normal, format!("Alıcı Tel: {}", f.kargo_telefon)));
        if !f.kargo_notu.is_empty() {
            v.push((Normal, format!("Kargo: {}", f.kargo_notu)));
        }
    }
    v.push((Ayrac, String::new()));
    v.push((
        Normal,
        format!(
            "Sat. Dan.: {} | Ölçü Alan: {}",
            f.satis_danismani, f.olcu_alan
        ),
    ));
    if !f.usta.is_empty() {
        v.push((Normal, format!("Usta: {}", f.usta)));
    }
    if !f.raf.is_empty() {
        v.push((Normal, format!("Raf/Stant: {}", f.raf)));
    }
    for i in &f.imzalar {
        v.push((
            Normal,
            format!("İmza ({}): {} {}", i.rol, i.kim, gun_metni(&i.tarih)),
        ));
    }
    if imza_blogu() && !musteri_nushasi(nusha) {
        v.push((Ayrac, String::new()));
        for l in [
            "Teslim Eden            Teslim Alan",
            "Tarih:                 Tarih:",
            "İmza:                  İmza:",
        ] {
            v.push((Ham, l.into()));
        }
    }
    v
}

/// Fiş başlığı (firma adı, adres, destek hattı). `TAA_FIS_BASLIK` ile (satırlar `|` ile) değiştirilir.
fn baslik() -> String {
    match std::env::var("TAA_FIS_BASLIK") {
        Ok(v) if !v.trim().is_empty() => v.replace('|', "\n"),
        _ => "TERZİ ADEM ALTUN İSTANBUL\nTurgut Özal Mah. 68. Sk. No:12, E-5 Karayolu Yan Yol Üzeri\nEsenyurt / İstanbul - Destek: 0212 690 02 02 - 9:30-21:30".into(),
    }
}

/// Teslim Eden / Teslim Alan tarih-imza alanları (`fis-imza` ayarıyla kapatılır).
fn imza_blogu() -> bool {
    taa_ayar::ozellik_acik("fis-imza", true)
}

/// 25 mm -> "2,5"; 40 mm -> "4".
pub fn cm_metni(mm: i32) -> String {
    if mm % 10 == 0 {
        (mm / 10).to_string()
    } else {
        format!("{},{}", mm / 10, (mm % 10).abs())
    }
}

fn cp857(c: char) -> u8 {
    match c {
        'Ç' => 0x80,
        'ü' => 0x81,
        'ç' => 0x87,
        'ı' => 0x8D,
        'İ' => 0x98,
        'Ö' => 0x99,
        'Ü' => 0x9A,
        'ö' => 0x94,
        'Ğ' => 0xA6,
        'ğ' => 0xA7,
        'Ş' => 0x9E,
        'ş' => 0x9F,
        c if c.is_ascii() => c as u8,
        _ => b'?',
    }
}

/// ESC/POS baytları: CP857, metin, fiş numarası QR kodu, kesme. Yazıcıya olduğu gibi gönderilir.
pub fn escpos(f: &Fis, genislik: usize) -> Vec<u8> {
    escpos_nusha(f, genislik, "")
}

/// Kâğıt fişteki düzen: 2 terzihane nüshası + 1 müşteri nüshası, her biri ayrı kesilir.
pub fn escpos_nushalar(f: &Fis, genislik: usize) -> Vec<u8> {
    escpos_nushalar_qr(f, genislik, &f.no)
}

/// Aynısı; QR içeriği `qr` (ör. `http://sunucu:8787/telefon#fis=F-...`: telefon kamerası fişi doğrudan açar).
pub fn escpos_nushalar_qr(f: &Fis, genislik: usize, qr: &str) -> Vec<u8> {
    escpos_nushalar_tam(f, genislik, qr, None)
}

/// Aynısı; `aydinlatma` verilirse (ve `fis-aydinlatma` ayarı açıksa) MÜŞTERİ nüshasına kısa aydınlatma
/// cümlesi ve ayrıntılı metne giden ikinci bir QR basılır.
pub fn escpos_nushalar_tam(
    f: &Fis,
    genislik: usize,
    qr: &str,
    aydinlatma: Option<&str>,
) -> Vec<u8> {
    let mut b = Vec::new();
    for etiket in nusha_etiketleri(nusha_sayisi()) {
        b.extend(escpos_nusha_tam(f, genislik, etiket, qr, aydinlatma));
    }
    b
}

/// Basılacak nüsha sayısı: varsayılan 2 (terzihane + müşteri; takip programda tutulur).
/// `TAA_FIS_NUSHA=3` eski düzeni (2 terzihane + 1 müşteri) geri getirir.
fn nusha_sayisi() -> usize {
    match std::env::var("TAA_FIS_NUSHA")
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some("3") => 3,
        _ => 2,
    }
}

fn nusha_etiketleri(n: usize) -> Vec<&'static str> {
    if n >= 3 {
        vec![
            "TERZİHANE NÜSHASI 1",
            "TERZİHANE NÜSHASI 2",
            "MÜŞTERİ NÜSHASI",
        ]
    } else {
        vec!["TERZİHANE NÜSHASI", "MÜŞTERİ NÜSHASI"]
    }
}

pub fn escpos_nusha(f: &Fis, genislik: usize, nusha: &str) -> Vec<u8> {
    escpos_nusha_qr(f, genislik, nusha, &f.no)
}

pub fn escpos_nusha_qr(f: &Fis, genislik: usize, nusha: &str, qr: &str) -> Vec<u8> {
    escpos_nusha_tam(f, genislik, nusha, qr, None)
}

/// Müşteri nüshasındaki kısa aydınlatma cümlesi. Metin şirketin KVKK sorumlusu tarafından onaylanmalıdır.
pub const AYDINLATMA_CUMLESI: &str = "Kişisel verileriniz, tadilat işleminin yürütülmesi ve ürününüzün hazır olduğunun bildirilmesi amacıyla işlenir. Ayrıntılı aydınlatma metni için aşağıdaki QR kodu okutunuz.";

fn qr_baytlari(veri: &[u8]) -> Vec<u8> {
    // QR (model 2, boyut 5, düzeltme L), ortalı
    let mut b = vec![0x1B, 0x61, 1];
    b.extend([0x1D, 0x28, 0x6B, 4, 0, 0x31, 0x41, 0x32, 0]);
    b.extend([0x1D, 0x28, 0x6B, 3, 0, 0x31, 0x43, 5]);
    b.extend([0x1D, 0x28, 0x6B, 3, 0, 0x31, 0x45, 0x30]);
    let n = veri.len() + 3;
    b.extend([
        0x1D,
        0x28,
        0x6B,
        (n % 256) as u8,
        (n / 256) as u8,
        0x31,
        0x50,
        0x30,
    ]);
    b.extend(veri);
    b.extend([0x1D, 0x28, 0x6B, 3, 0, 0x31, 0x51, 0x30]);
    b.extend([0x1B, 0x61, 0]);
    b
}

pub fn escpos_nusha_tam(
    f: &Fis,
    genislik: usize,
    nusha: &str,
    qr: &str,
    aydinlatma: Option<&str>,
) -> Vec<u8> {
    const KUCUK: usize = 60; // Font B: 80 mm kâğıtta 64 sütun sığar, 60 güvenli sınır
    let g = genislik.max(24);
    let mut b = vec![0x1B, 0x40, 0x1B, 0x74, 13]; // başlat, kod sayfası 13 = CP857
    for (stil, m) in satirlar(f, nusha) {
        match stil {
            Stil::Ayrac => b.extend(std::iter::repeat_n(0xC4, g)), // ─
            Stil::Cift => b.extend(std::iter::repeat_n(0xCD, g)),  // ═
            Stil::Ham => b.extend(m.chars().map(cp857)),           // boşlukları olduğu gibi
            _ => {
                let (genis, ayar): (usize, &[u8]) = match stil {
                    Stil::Kalin => (g, &[0x1B, 0x45, 1]),
                    Stil::OrtaKalin => (g, &[0x1B, 0x61, 1, 0x1B, 0x45, 1]),
                    Stil::OrtaKucuk => (KUCUK, &[0x1B, 0x61, 1, 0x1B, 0x4D, 1]),
                    Stil::OrtaBuyuk => (g / 2, &[0x1B, 0x61, 1, 0x1B, 0x45, 1, 0x1D, 0x21, 0x11]),
                    _ => (g, &[]),
                };
                b.extend(ayar);
                b.extend(sar(&m, genis).join("\n").chars().map(cp857));
                b.extend(*b"\n");
                // ayarları sıfırla: hizalama sola, kalın kapalı, Font A, boyut 1x
                b.extend([0x1B, 0x61, 0, 0x1B, 0x45, 0, 0x1B, 0x4D, 0, 0x1D, 0x21, 0]);
                continue;
            }
        }
        b.extend(*b"\n");
    }
    if !terzihane_nushasi(nusha) {
        b.extend([0x1B, 0x4D, 1]);
        for c in [
            "Ürünlerinizi Saat 17:00'den itibaren teslim alabilirsiniz.",
            "14 Gün içinde alınmayan ürünlerden mesul değiliz.",
            "Ürünlerinizi teslim alırken tadilat fişinizi yanınızda bulundurmayı unutmayınız.",
        ] {
            b.extend(sar(c, KUCUK).join("\n").chars().map(cp857));
            b.extend(*b"\n");
        }
        b.extend([0x1B, 0x4D, 0]);
    }
    b.extend(qr_baytlari(qr.as_bytes()));
    b.extend(*b"\n");
    if let Some(url) = aydinlatma {
        if nusha.starts_with("MÜŞTERİ") && taa_ayar::ozellik_acik("fis-aydinlatma", true) {
            b.extend([0x1B, 0x4D, 1]); // küçük yazı (Font B)
            b.extend(
                format!("{}\n", sar(AYDINLATMA_CUMLESI, KUCUK).join("\n"))
                    .chars()
                    .map(cp857),
            );
            b.extend([0x1B, 0x4D, 0]);
            b.extend(qr_baytlari(url.as_bytes()));
            b.extend(*b"\n");
        }
    }
    b.extend([0x1D, 0x56, 0x42, 0]); // kâğıdı kesme konumuna ilerletip kısmi kes (ek boş satır gerekmez)
    b
}

#[cfg(test)]
mod testler {
    use super::*;

    fn kalem(urun: &str, islem: &str, not: &str) -> Kalem {
        Kalem {
            urun: urun.into(),
            islem: islem.into(),
            not: not.into(),
            islem_kodu: None,
            varyant: None,
            olcu_mm: None,
            pay_mm: None,
            usta_onayli: false,
            igneli: false,
        }
    }

    fn kat() -> taa_terzi::Katalog {
        taa_terzi::Katalog::varsayilan()
    }

    fn girdi() -> Girdi {
        Girdi {
            tarih: "2026-11-04".into(),
            musteri: " Ayşe Çelik ".into(),
            telefon: "+90 532 123 45 67".into(),
            kalemler: vec![
                kalem("Pantolon", "paça kısalt", "2 cm"),
                kalem("Ceket", "kol kısalt", ""),
            ],
            olcu_notu: "bel 84".into(),
            prova: "2026-11-07".into(),
            teslim: "2026-11-10".into(),
            teslim_saati: "17:00".into(),
            ucret_kurus: 85000,
            on_odeme_kurus: 20000,
            usta: "Hasan".into(),
            satis_danismani: "Mert".into(),
            olcu_alan: "Kadir Y.".into(),
            kargo: false,
            kargo_notu: String::new(),
            kargo_ad: String::new(),
            kargo_adres: String::new(),
            kargo_telefon: String::new(),
            bildirim_izni: false,
        }
    }

    fn yol() -> std::path::PathBuf {
        taa_dosya::gecici_klasor().unwrap().join("fis.jsonl")
    }

    #[test]
    fn aydinlatma_yalniz_musteri_nushasinda() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let f = Defter { yol: &y }.ekle(girdi()).unwrap();
        let url = "http://x/aydinlatma";
        let b = escpos_nushalar_tam(&f, 42, "http://x/telefon#fis=F", Some(url));
        let sayi = |hedef: &[u8]| b.windows(hedef.len()).filter(|w| *w == hedef).count();
        assert_eq!(sayi(url.as_bytes()), 1); // yalnız müşteri nüshası
        assert_eq!(
            sayi(
                "Kişisel verileriniz"
                    .chars()
                    .map(cp857)
                    .collect::<Vec<_>>()
                    .as_slice()
            ),
            1
        );
        // verilmezse hiçbir nüshada yok
        let c = escpos_nushalar_tam(&f, 42, "q", None);
        assert!(!c.windows(19).any(|w| w == url.as_bytes()));
        assert_eq!(
            c.windows(4).filter(|w| *w == [0x1D, 0x56, 0x42, 0]).count(),
            2
        );
    }

    #[test]
    fn raf_ve_imza() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let d = Defter { yol: &y };
        let f = d.ekle(girdi()).unwrap();
        assert!(d.raf_ata(&f.no, "", "Ali", "2026-11-05").is_err());
        assert!(d.raf_ata("yok", "A-1", "Ali", "2026-11-05").is_err());
        d.raf_ata(&f.no, "a-12", "Ali", "2026-11-05").unwrap();
        d.raf_ata(&f.no, "B-3", "Ali", "2026-11-06").unwrap();
        d.imzala(&f.no, "Usta Ahmet", "usta", "2026-11-05").unwrap();
        assert!(d.imzala(&f.no, "Usta Ahmet", "usta", "2026-11-05").is_err());
        assert!(d.imzala(&f.no, "", "usta", "2026-11-05").is_err());
        assert!(d.imzala(&f.no, "X", "patron", "2026-11-05").is_err());
        let g = d.bul(&f.no).unwrap().unwrap();
        assert_eq!(g.raf, "B-3");
        assert_eq!(g.imzalar.len(), 1);
        let m = duz_metin(&g, 42);
        assert!(m.contains("Raf/Stant: B-3") && m.contains("İmza (usta): Usta Ahmet"));
        let q = escpos_nushalar_qr(&g, 42, "http://x/telefon#fis=F");
        assert!(q.windows(22).any(|w| w == b"http://x/telefon#fis=F"));
        for st in [Durum::Terzihanede, Durum::Hazir, Durum::Teslim] {
            d.durum_ata(&f.no, st, "2026-11-06").unwrap();
        }
        assert!(d.raf_ata(&f.no, "C-1", "Ali", "2026-11-07").is_err());
    }

    #[test]
    fn kargo_fiste_basilir() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let d = Defter { yol: &y };
        let mut g = girdi();
        g.kargo = true;
        g.kargo_notu = "Zile kapıda ödeme yok".into();
        assert!(d.ekle(g.clone()).unwrap_err().contains("ad ve soyad"));
        g.kargo_ad = "Ayşe Çelik".into();
        assert!(d.ekle(g.clone()).unwrap_err().contains("açık adres"));
        g.kargo_adres = "Atatürk Mah. 5. Sk. No:3 Çankaya/Ankara".into();
        assert!(d.ekle(g.clone()).unwrap_err().contains("telefon"));
        g.kargo_telefon = "0532 111 22 33".into();
        let f = d.ekle(g).unwrap();
        for e in [
            "TERZİHANE NÜSHASI 1",
            "TERZİHANE NÜSHASI 2",
            "MÜŞTERİ NÜSHASI",
        ] {
            let m = duz_metin_nusha(&f, 42, e);
            assert!(
                m.contains("KARGO YAPILACAK") && m.contains("Çankaya"),
                "{e}"
            );
        }
        let f2 = d.ekle(girdi()).unwrap();
        assert!(!duz_metin(&f2, 42).contains("KARGO"));
    }

    #[test]
    fn telefon() {
        assert_eq!(telefon_duzelt("+90 532 123 45 67").unwrap(), "05321234567");
        assert_eq!(telefon_duzelt("0532 123 4567").unwrap(), "05321234567");
        assert_eq!(telefon_duzelt("5321234567").unwrap(), "05321234567");
        assert!(telefon_duzelt("12345").is_err());
        assert!(telefon_duzelt("02121234567").is_err());
    }

    #[test]
    fn dogrulama() {
        let mut g = girdi();
        assert!(dogrula(&g, &kat(), true, true).is_ok());
        g.on_odeme_kurus = 90000;
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.teslim = "2026-11-01".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.prova = "2026-11-12".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.kalemler.clear();
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.musteri = "  ".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
    }

    #[test]
    fn defter_dongusu() {
        let y = yol();
        let d = Defter { yol: &y };
        let f1 = d.ekle(girdi()).unwrap();
        let f2 = d.ekle(girdi()).unwrap();
        assert_eq!(
            (f1.no.as_str(), f2.no.as_str()),
            ("F-20261104-001", "F-20261104-002")
        );
        assert_eq!(f1.musteri, "Ayşe Çelik");
        assert_eq!(f1.kalan_kurus(), 65000);
        assert!(d.durum_ata(&f1.no, Durum::Teslim, "2026-11-05").is_err()); // Hazir'dan geçmeli
        d.durum_ata(&f1.no, Durum::Hazir, "2026-11-08").unwrap();
        d.durum_ata(&f1.no, Durum::Teslim, "2026-11-09").unwrap();
        assert!(d.durum_ata(&f1.no, Durum::Iptal, "2026-11-09").is_err());
        assert_eq!(d.bul(&f1.no).unwrap().unwrap().durum, Durum::Teslim);
        assert_eq!(d.geciken("2026-11-11").unwrap().len(), 1);
        assert_eq!(d.geciken("2026-11-10").unwrap().len(), 0);
        assert_eq!(taa_dosya::satirlari_oku(&y).unwrap().len(), 4); // hiçbir şey silinmedi
    }

    #[test]
    fn metin_ve_termal() {
        let f = Fis {
            no: "F-20261104-001".into(),
            tarih: "2026-11-04".into(),
            durum: Durum::Bekliyor,
            raf: String::new(),
            imzalar: Vec::new(),
            musteri: "Ayşe Çelik".into(),
            telefon: "05321234567".into(),
            kalemler: girdi().kalemler,
            olcu_notu: "".into(),
            prova: "".into(),
            teslim: "2026-11-10".into(),
            teslim_saati: "17:00".into(),
            ucret_kurus: 85000,
            on_odeme_kurus: 20000,
            usta: "".into(),
            satis_danismani: "Mert".into(),
            olcu_alan: "Kadir Y.".into(),
            kargo: false,
            kargo_notu: String::new(),
            kargo_ad: String::new(),
            kargo_adres: String::new(),
            kargo_telefon: String::new(),
            bildirim_izni: false,
            teslim_degisiklikleri: Vec::new(),
        };
        let m = duz_metin(&f, 42);
        assert!(m.contains("TADİLAT FİŞİ") && m.contains("F-20261104-001"));
        assert!(m.contains("ÜRÜN TESLİM TARİHİ") && m.contains("10.11.2026 17:00"));
        assert!(m.lines().all(|l| l.chars().count() <= 42));
        // ödeme alınmadan işlem yapılmaz: fiyat basılmaz
        assert!(!m.contains("KALAN") && !m.contains(" TL"));
        let b = escpos(&f, 42);
        assert_eq!(&b[..5], &[0x1B, 0x40, 0x1B, 0x74, 13]);
        assert!(b.contains(&0x98)); // İ -> CP857
        assert!(b.ends_with(&[0x1D, 0x56, 0x42, 0]));
    }

    #[test]
    fn para_ve_sarma() {
        assert_eq!(para(123456), "1.234,56");
        assert_eq!(para(5), "0,05");
        assert!(sar(&"a".repeat(100), 42).iter().all(|l| l.len() <= 42));
    }

    fn katalogsuz_girdi() -> Girdi {
        let mut g = girdi();
        g.kalemler = vec![Kalem {
            islem_kodu: Some("pant-bel-daralt".into()),
            olcu_mm: Some(30),
            ..kalem("Pantolon", "Bel daraltma", "")
        }];
        g
    }

    #[test]
    fn katalog_kalemi_ve_sinir() {
        let mut g = katalogsuz_girdi();
        assert!(dogrula(&g, &kat(), false, true).is_ok());
        g.kalemler[0].olcu_mm = Some(60);
        assert!(dogrula(&g, &kat(), false, true)
            .unwrap_err()
            .contains("Usta onayı"));
        g.kalemler[0].usta_onayli = true;
        assert_eq!(dogrula(&g, &kat(), false, true).unwrap().1.len(), 1);
        g.kalemler[0].olcu_mm = None; // unutulan ölçü
        assert!(dogrula(&g, &kat(), false, true)
            .unwrap_err()
            .contains("ölçü"));
    }

    #[test]
    fn serbest_islem_kapatilabilir_ve_zorunlu_isimler() {
        let g = girdi(); // serbest metin kalemler
        assert!(dogrula(&g, &kat(), true, true).is_ok());
        assert!(dogrula(&g, &kat(), false, true)
            .unwrap_err()
            .contains("listeden"));
        let mut g = girdi();
        g.olcu_alan = " ".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.teslim_saati = "25:00".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
    }

    #[test]
    fn teslim_degisikligi_izlenir() {
        let y = yol();
        let d = Defter { yol: &y };
        let f = d.ekle(girdi()).unwrap();
        assert!(d
            .teslim_degistir(&f.no, "2026-11-12", "17:00", " ", "2026-11-05")
            .is_err());
        assert!(d
            .teslim_degistir(&f.no, "2026-11-01", "17:00", "x", "2026-11-05")
            .is_err());
        d.teslim_degistir(&f.no, "2026-11-12", "18:30", "müşteri istedi", "2026-11-05")
            .unwrap();
        let g = d.bul(&f.no).unwrap().unwrap();
        assert_eq!(
            (g.teslim.as_str(), g.teslim_saati.as_str()),
            ("2026-11-12", "18:30")
        );
        assert_eq!(g.teslim_degisiklikleri[0].eski, "2026-11-10");
        assert!(duz_metin(&g, 42).contains("1 kez değişti"));
        d.durum_ata(&f.no, Durum::Terzihanede, "2026-11-05")
            .unwrap();
        d.durum_ata(&f.no, Durum::Hazir, "2026-11-06").unwrap();
        d.durum_ata(&f.no, Durum::Teslim, "2026-11-12").unwrap();
        assert!(d
            .teslim_degistir(&f.no, "2026-11-20", "17:00", "x", "2026-11-13")
            .is_err());
    }

    #[test]
    fn eski_defter_satiri_okunur() {
        // yeni alanlardan önce yazılmış bir satır
        let y = yol();
        let eski = r#"{"tur":"fis","v":{"no":"F-20260101-001","tarih":"2026-01-01","musteri":"A","telefon":"05321234567","kalemler":[{"urun":"Ceket","islem":"kol"}],"teslim":"2026-01-05","ucret_kurus":0,"on_odeme_kurus":0,"durum":"bekliyor"}}"#;
        taa_dosya::satir_ekle(&y, eski).unwrap();
        let f = Defter { yol: &y }.bul("F-20260101-001").unwrap().unwrap();
        assert_eq!(f.teslim_saati, "17:00");
        assert!(f.kalemler[0].olcu_mm.is_none());
    }

    #[test]
    fn uc_nusha_ve_cm() {
        assert_eq!(cm_metni(25), "2,5");
        assert_eq!(cm_metni(40), "4");
        let y = yol();
        let d = Defter { yol: &y };
        let f = d.ekle(katalogsuz_girdi()).unwrap();
        let m = duz_metin(&f, 42);
        assert!(
            m.contains("Bel daraltma 3 cm")
                && m.contains("Sat. Dan.: Mert")
                && m.contains("Ölçü Alan: Kadir Y.")
        );
        let b = escpos_nushalar(&f, 42);
        assert_eq!(
            b.windows(4).filter(|w| *w == [0x1D, 0x56, 0x42, 0]).count(),
            2
        );
        assert!(String::from_utf8_lossy(&b).contains("mesul"));
    }

    #[test]
    fn kagit_tasarrufu_duzeni() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let f = Defter { yol: &y }.ekle(girdi()).unwrap();
        assert_eq!(nusha_etiketleri(2).len(), 2);
        assert_eq!(nusha_etiketleri(3).len(), 3);
        let t = duz_metin_nusha(&f, 42, "TERZİHANE NÜSHASI");
        let m = duz_metin_nusha(&f, 42, "MÜŞTERİ NÜSHASI");
        assert!(t.contains("Teslim Eden") && !m.contains("Teslim Eden"));
        assert!(t.contains("Fiş Tarihi: 04.11.2026"));
        assert!(t.contains("Sat. Dan.: Mert | Ölçü Alan: Kadir Y."));
        assert!(t.lines().all(|l| l.chars().count() <= 42));
        // alt bilgi cümleleri yalnız müşteri nüshasında basılır
        let b = escpos_nushalar(&f, 42);
        let yer = |hedef: &str| {
            let h: Vec<u8> = hedef.chars().map(cp857).collect();
            b.windows(h.len()).filter(|w| *w == h.as_slice()).count()
        };
        assert_eq!(yer("mesul değiliz"), 1);
    }
}
