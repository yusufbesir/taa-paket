// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! taa-kur penceresinin platformdan bağımsız modeli (docs/T3-PENCERE.md).
//! Komut -> `Kur` çağrısı, satır modeli, düğme etkin/pasif kuralları ve Türkçe durum metinleri burada;
//! Win32 çizimi (`win.rs`) yalnız bu modeli çağırır. Hiçbir şey silinmez: kaldırma `Kur::kaldir` ile
//! `eski\` klasörüne taşımadır; veri alanlarına dokunulmaz.

use crate::kayit::surum_karsilastir;
use crate::{KaldirSonuc, Kur, KurHata, KurSonuc, ListeSatiri};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};

/// Liste sütun başlıkları (Win32 katmanı bunları kullanır).
pub const SUTUNLAR: [&str; 4] = ["Paket", "Kurulu sürüm", "Seçili sürüm", "Durum"];

/// Güncelleme kaynağı (taa-guncelle başka dalda yazılıyor; o gelince bu trait'e bağlanır).
pub trait GuncelleyiciArayuzu {
    /// Paketi güncellemeyi dener; başarıda ya da bilgi durumunda Türkçe tek satır mesaj, hatada neden + öneri.
    fn guncelle(&self, kur: &Kur, paket_adi: &str) -> Result<String, String>;
}

/// Varsayılan uygulama: güncelleme kaynağı yok.
#[derive(Debug, Default, Clone, Copy)]
pub struct KaynaksizGuncelleyici;

impl GuncelleyiciArayuzu for KaynaksizGuncelleyici {
    fn guncelle(&self, _kur: &Kur, _paket_adi: &str) -> Result<String, String> {
        Err("Güncelleme kaynağı ayarlanmadı. Öneri: güncelleme modülü (taa-guncelle) kurulunca etkinleşir.".into())
    }
}

/// Listedeki bir paket satırı (görüntülenecek metinler hazır).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Satir {
    pub ad: String,
    pub kurulu: String,
    pub secili: String,
    pub durum: String,
    /// Seçili sürüm var mı (kaldırma kuralı için).
    pub secili_var: bool,
}

impl Satir {
    /// Sütun sırası `SUTUNLAR` ile aynı.
    pub fn hucre(&self, sutun: usize) -> &str {
        match sutun {
            0 => &self.ad,
            1 => &self.kurulu,
            2 => &self.secili,
            _ => &self.durum,
        }
    }

    fn kayittan(s: &ListeSatiri) -> Satir {
        let en_yuksek = s
            .kurulu
            .iter()
            .max_by(|a, b| surum_karsilastir(a, b))
            .cloned();
        let durum = match (&s.secili, &en_yuksek) {
            (_, None) => "Kurulu değil".to_string(),
            (None, Some(_)) => "Seçili sürüm yok".to_string(),
            (Some(sec), Some(en)) if surum_karsilastir(sec, en) == Ordering::Less => {
                format!("Daha yeni sürüm kurulu ({en}), seçili değil")
            }
            (Some(_), Some(_)) if s.sabitle => "Hazır (sabit)".to_string(),
            (Some(_), Some(_)) => "Hazır".to_string(),
        };
        Satir {
            ad: s.ad.clone(),
            kurulu: if s.kurulu.is_empty() {
                "-".to_string()
            } else {
                s.kurulu.join(", ")
            },
            secili: s.secili.clone().unwrap_or_else(|| "-".to_string()),
            durum,
            secili_var: s.secili.is_some(),
        }
    }
}

/// Düğmelerin etkin olup olmadığı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DugmeDurumu {
    pub kur: bool,
    pub kaldir: bool,
    pub guncelle: bool,
}

/// Pencere durumu: satırlar, seçim ve alt durum satırı.
pub struct PencereModel {
    kur: Kur,
    guncelleyici: Box<dyn GuncelleyiciArayuzu>,
    satirlar: Vec<Satir>,
    secili: Option<usize>,
    durum: String,
}

/// Hata mesajını tek satıra indirir (CLI ile aynı biçim).
fn tek_satir(m: &str) -> String {
    m.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn hata_metni(h: &KurHata) -> String {
    tek_satir(h.mesaj())
}

/// Dosya seçiciden gelen yolu paket yoluna çevirir: `paket.json` seçildiyse üst klasör (klasör paketi),
/// başka dosya (örn. `.zip`) olduğu gibi.
pub fn paket_yolu_coz(secilen: &Path) -> PathBuf {
    let ad_paket_json = secilen
        .file_name()
        .map(|a| a.to_string_lossy().eq_ignore_ascii_case("paket.json"))
        .unwrap_or(false);
    if ad_paket_json {
        if let Some(ust) = secilen.parent() {
            return ust.to_path_buf();
        }
    }
    secilen.to_path_buf()
}

impl PencereModel {
    pub fn yeni(kur: Kur, guncelleyici: Box<dyn GuncelleyiciArayuzu>) -> PencereModel {
        let mut m = PencereModel {
            kur,
            guncelleyici,
            satirlar: Vec::new(),
            secili: None,
            durum: String::new(),
        };
        m.yenile();
        if m.durum.is_empty() {
            m.durum = m.hazir_metni();
        }
        m
    }

    fn hazir_metni(&self) -> String {
        if self.satirlar.is_empty() {
            "Kurulu paket yok. Öneri: «Kur» ile bir paket seçin.".to_string()
        } else {
            format!("{} paket listelendi.", self.satirlar.len())
        }
    }

    pub fn kok_metni(&self) -> String {
        format!("Kök: {}", self.kur.kok().display())
    }

    pub fn satirlar(&self) -> &[Satir] {
        &self.satirlar
    }

    pub fn durum(&self) -> &str {
        &self.durum
    }

    pub fn secili_indeks(&self) -> Option<usize> {
        self.secili
    }

    pub fn secili_satir(&self) -> Option<&Satir> {
        self.secili.and_then(|i| self.satirlar.get(i))
    }

    /// Listeyi kayıttan yeniden okur; seçim paket adına göre korunur. Hata durum satırına yazılır.
    pub fn yenile(&mut self) {
        let onceki = self.secili_satir().map(|s| s.ad.clone());
        match self.kur.liste() {
            Ok(l) => {
                self.satirlar = l.iter().map(Satir::kayittan).collect();
                self.secili = onceki.and_then(|ad| self.satirlar.iter().position(|s| s.ad == ad));
            }
            Err(h) => {
                self.satirlar.clear();
                self.secili = None;
                self.durum = format!("Liste okunamadı: {}", hata_metni(&h));
            }
        }
    }

    /// Satır seçimi (liste denetiminden). Sınır dışı indeks seçimi kaldırır.
    pub fn sec_satir(&mut self, indeks: Option<usize>) {
        self.secili = indeks.filter(|i| *i < self.satirlar.len());
    }

    /// Düğme kuralları: Kur her zaman; Kaldır yalnız seçili sürümü olan satırda; Güncelle seçili satırda.
    pub fn dugmeler(&self) -> DugmeDurumu {
        let s = self.secili_satir();
        DugmeDurumu {
            kur: true,
            kaldir: s.is_some_and(|s| s.secili_var),
            guncelle: s.is_some(),
        }
    }

    /// Kaldırma onay metni; seçili uygun satır yoksa `None`. İçeriğin silinmeyeceğini söyler.
    pub fn kaldir_onay_metni(&self) -> Option<String> {
        let s = self.secili_satir().filter(|s| s.secili_var)?;
        Some(format!(
            "{} paketinin seçili sürümü ({}) kaldırılacak.\nİçerik silinmez, kök klasördeki eski\\ klasörüne taşınır; veri alanlarına dokunulmaz.\nDevam edilsin mi?",
            s.ad, s.secili
        ))
    }

    /// Kur düğmesi: seçilen dosya/klasör paketi kurulur. Durum metni döner (aynı metin `durum()` olur).
    pub fn kur_paketi(&mut self, secilen: &Path) -> &str {
        let yol = paket_yolu_coz(secilen);
        let metin = match self.kur.kur(&yol, false) {
            Ok(s) => kur_metni(&s),
            Err(h) => format!("Kurulamadı: {}", hata_metni(&h)),
        };
        self.yenile();
        self.durum = metin;
        &self.durum
    }

    /// Kaldır düğmesi (onay arayüzde alınmıştır). Seçili satırın seçili sürümü `eski\`ye taşınır.
    pub fn kaldir(&mut self) -> &str {
        let metin = match self
            .secili_satir()
            .filter(|s| s.secili_var)
            .map(|s| s.ad.clone())
        {
            None => {
                "Kaldırılacak paket seçilmedi. Öneri: listeden seçili sürümü olan bir paket seçin."
                    .to_string()
            }
            Some(ad) => match self.kur.kaldir(&ad, None) {
                Ok(s) => kaldir_metni(&s),
                Err(h) => format!("Kaldırılamadı: {}", hata_metni(&h)),
            },
        };
        self.yenile();
        self.durum = metin;
        &self.durum
    }

    /// Güncelle düğmesi: `GuncelleyiciArayuzu`'na iletilir.
    pub fn guncelle(&mut self) -> &str {
        let metin = match self.secili_satir().map(|s| s.ad.clone()) {
            None => "Güncellenecek paket seçilmedi. Öneri: listeden bir paket seçin.".to_string(),
            Some(ad) => match self.guncelleyici.guncelle(&self.kur, &ad) {
                Ok(m) => tek_satir(&m),
                Err(m) => format!("Güncellenemedi: {}", tek_satir(&m)),
            },
        };
        self.yenile();
        self.durum = metin;
        &self.durum
    }
}

fn kur_metni(s: &KurSonuc) -> String {
    let eski = s
        .eski
        .as_ref()
        .map(|e| format!(" Önceki içerik şuraya taşındı: {}.", e.display()))
        .unwrap_or_default();
    format!(
        "Kuruldu: {} {} ({}).{eski}",
        s.ad,
        s.surum,
        if s.secili {
            "seçili"
        } else {
            "seçili değil"
        }
    )
}

fn kaldir_metni(s: &KaldirSonuc) -> String {
    let mut m = format!(
        "Kaldırıldı: {} {} (içerik şuraya taşındı: {}).",
        s.ad,
        s.surum,
        s.eski.display()
    );
    if let Some(y) = &s.yeni_secili {
        m.push_str(&format!(" Seçili sürüm: {y}."));
    }
    if !s.salt_okunur.is_empty() {
        m.push_str(&format!(
            " Veri alanları salt okunur olacak paketler: {}.",
            s.salt_okunur.join(", ")
        ));
    }
    for u in &s.uyarilar {
        m.push_str(&format!(" Uyarı: {u}"));
    }
    tek_satir(&m)
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::paket::{icerik_hesapla, sha256_bayt, DosyaKaydi};
    use std::fs;

    struct Ortam {
        taban: PathBuf,
        kok: PathBuf,
    }

    impl Ortam {
        fn yeni() -> Ortam {
            let taban = taa_dosya::gecici_klasor().unwrap();
            let kok = taban.join("kok");
            Ortam { taban, kok }
        }

        fn model(&self) -> PencereModel {
            PencereModel::yeni(
                Kur::yeni(&self.kok).unwrap(),
                Box::new(KaynaksizGuncelleyici),
            )
        }

        fn paket(&self, ad: &str, surum: &str) -> PathBuf {
            let k = self.taban.join("paketler").join(format!("{ad}-{surum}"));
            fs::create_dir_all(&k).unwrap();
            let icerik = format!("{ad} {surum}");
            fs::write(k.join("veri.txt"), &icerik).unwrap();
            let liste = vec![DosyaKaydi {
                yol: "veri.txt".into(),
                sha256: sha256_bayt(icerik.as_bytes()),
            }];
            let p = serde_json::json!({
                "sema": 1, "ad": ad, "surum": surum, "arayuz": "1", "program": "ortak",
                "gerektirir": {}, "sunar": [], "ister": [], "veri_sahibi": [], "veri_okur": [],
                "dosyalar": liste, "icerik_sha256": icerik_hesapla(&liste),
            });
            fs::write(k.join("paket.json"), serde_json::to_vec_pretty(&p).unwrap()).unwrap();
            k
        }
    }

    impl Drop for Ortam {
        fn drop(&mut self) {
            // Yalnız testin kendi geçici klasörü (diğer taa-kur testleriyle aynı kural).
            let _ = fs::remove_dir_all(&self.taban);
        }
    }

    #[test]
    fn bos_kokte_liste_bos_ve_dugmeler() {
        let o = Ortam::yeni();
        let m = o.model();
        assert!(m.satirlar().is_empty());
        assert!(m.durum().contains("Kurulu paket yok"));
        assert_eq!(
            m.dugmeler(),
            DugmeDurumu {
                kur: true,
                kaldir: false,
                guncelle: false
            }
        );
    }

    #[test]
    fn kur_liste_kaldir_gercek_kur_ile() {
        let o = Ortam::yeni();
        let mut m = o.model();
        let p1 = o.paket("taa-ornek", "0.1.0");
        let d = m.kur_paketi(&p1).to_string();
        assert!(d.starts_with("Kuruldu: taa-ornek 0.1.0 (seçili)"), "{d}");
        assert_eq!(m.satirlar().len(), 1);
        let s = &m.satirlar()[0];
        assert_eq!(
            (s.ad.as_str(), s.kurulu.as_str(), s.secili.as_str()),
            ("taa-ornek", "0.1.0", "0.1.0")
        );
        assert_eq!(s.durum, "Hazır");
        // seçim yokken Kaldır/Güncelle pasif
        assert_eq!(
            m.dugmeler(),
            DugmeDurumu {
                kur: true,
                kaldir: false,
                guncelle: false
            }
        );
        m.sec_satir(Some(0));
        assert_eq!(
            m.dugmeler(),
            DugmeDurumu {
                kur: true,
                kaldir: true,
                guncelle: true
            }
        );
        assert!(m.kaldir_onay_metni().unwrap().contains("silinmez"));
        let d = m.kaldir().to_string();
        assert!(d.starts_with("Kaldırıldı: taa-ornek 0.1.0"), "{d}");
        // içerik silinmedi, eski\ altına taşındı
        let eski = o.kok.join("eski");
        let adlar: Vec<String> = fs::read_dir(&eski)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        assert!(
            adlar.iter().any(|a| a.starts_with("taa-ornek-0.1.0-")),
            "{adlar:?}"
        );
        assert!(m.satirlar().iter().all(|s| !s.secili_var));
    }

    #[test]
    fn ikinci_surum_ve_durum_metinleri() {
        let o = Ortam::yeni();
        let mut m = o.model();
        m.kur_paketi(&o.paket("taa-ornek", "0.1.0"));
        m.kur_paketi(&o.paket("taa-ornek", "0.2.0"));
        let s = &m.satirlar()[0];
        assert_eq!(s.kurulu, "0.1.0, 0.2.0");
        assert_eq!(s.secili, "0.1.0");
        assert!(
            s.durum.contains("Daha yeni sürüm kurulu (0.2.0)"),
            "{}",
            s.durum
        );
    }

    #[test]
    fn paket_json_secilirse_ust_klasor_kurulur() {
        let o = Ortam::yeni();
        let mut m = o.model();
        let k = o.paket("taa-ornek", "0.1.0");
        let d = m.kur_paketi(&k.join("paket.json")).to_string();
        assert!(d.starts_with("Kuruldu: taa-ornek 0.1.0"), "{d}");
        assert_eq!(
            paket_yolu_coz(Path::new("a/Paket.JSON")),
            PathBuf::from("a")
        );
        assert_eq!(
            paket_yolu_coz(Path::new("a/x.zip")),
            PathBuf::from("a/x.zip")
        );
    }

    #[test]
    fn hatalar_turkce_tek_satir_ve_kayit_degismez() {
        let o = Ortam::yeni();
        let mut m = o.model();
        let d = m.kur_paketi(&o.taban.join("yok-klasor")).to_string();
        assert!(d.starts_with("Kurulamadı: "), "{d}");
        assert!(!d.contains('\n'));
        assert!(m.satirlar().is_empty());
        // seçim yokken Kaldır/Güncelle güvenli
        assert!(m.kaldir().starts_with("Kaldırılacak paket seçilmedi"));
        assert!(m.guncelle().starts_with("Güncellenecek paket seçilmedi"));
    }

    #[test]
    fn varsayilan_guncelleyici_kaynak_ayarlanmadi_der() {
        let o = Ortam::yeni();
        let mut m = o.model();
        m.kur_paketi(&o.paket("taa-ornek", "0.1.0"));
        m.sec_satir(Some(0));
        let d = m.guncelle().to_string();
        assert!(d.contains("Güncelleme kaynağı ayarlanmadı"), "{d}");
        // güncelleme başarısızlığı kurulu paketi etkilemez
        assert_eq!(m.satirlar().len(), 1);
        assert_eq!(m.secili_indeks(), Some(0), "seçim ada göre korunur");
    }

    #[test]
    fn ozel_guncelleyici_trait_uzerinden_cagrilir() {
        struct Sahte;
        impl GuncelleyiciArayuzu for Sahte {
            fn guncelle(&self, _k: &Kur, ad: &str) -> Result<String, String> {
                Ok(format!("{ad} güncel.\n"))
            }
        }
        let o = Ortam::yeni();
        let mut m = PencereModel::yeni(Kur::yeni(&o.kok).unwrap(), Box::new(Sahte));
        m.kur_paketi(&o.paket("taa-ornek", "0.1.0"));
        m.sec_satir(Some(0));
        assert_eq!(m.guncelle(), "taa-ornek güncel.");
        m.sec_satir(Some(9));
        assert_eq!(m.secili_indeks(), None);
    }
}
