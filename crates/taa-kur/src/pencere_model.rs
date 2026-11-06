// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! taa-kur penceresinin platformdan bağımsız modeli (docs/T3-PENCERE.md).
//! Komut -> `Kur` çağrısı, satır modeli, düğme etkin/pasif kuralları ve Türkçe durum metinleri burada;
//! Win32 çizimi (`win.rs`) yalnız bu modeli çağırır. Hiçbir şey silinmez: kaldırma `Kur::kaldir` ile
//! `eski\` klasörüne taşımadır; veri alanlarına dokunulmaz.

use crate::kayit::surum_karsilastir;
use crate::{KaldirSonuc, Kur, KurHata, KurSonuc, ListeSatiri};
use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Liste sütun başlıkları (Win32 katmanı bunları kullanır).
pub const SUTUNLAR: [&str; 4] = ["Paket", "Kurulu sürüm", "Seçili sürüm", "Durum"];

/// Güncelleme kaynağı: pencere bu trait üzerinden günceller (`KomutGuncelleyici` taa-guncelle.exe'yi çalıştırır).
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

/// `taa-guncelle` komutunu çalıştıran güncelleyici: `taa-guncelle uygula --json --paket <ad>` (TAA_KOK = kök).
/// taa-kur, taa-guncelle'ye bağlanamaz (taa-guncelle taa-kur'u kullanır); bu yüzden komut olarak çağrılır.
/// Komut bulunamazsa pencere `KaynaksizGuncelleyici` ile açılır.
#[derive(Debug, Clone)]
pub struct KomutGuncelleyici {
    komut: PathBuf,
}

impl KomutGuncelleyici {
    pub fn yeni(komut: impl Into<PathBuf>) -> KomutGuncelleyici {
        KomutGuncelleyici {
            komut: komut.into(),
        }
    }

    pub fn komut(&self) -> &Path {
        &self.komut
    }

    /// Sırasıyla: çalışan programın yanındaki `taa-guncelle[.exe]`, sonra kökte seçili `taa-guncelle` paketinin
    /// `bin\taa-guncelle[.exe]` dosyası. Yoksa `None`.
    pub fn bul(kur: &Kur) -> Option<KomutGuncelleyici> {
        let ad = format!("taa-guncelle{}", std::env::consts::EXE_SUFFIX);
        let yanimda = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.join(&ad)));
        let depoda = kur
            .liste()
            .ok()
            .and_then(|l| l.into_iter().find(|s| s.ad == "taa-guncelle"))
            .and_then(|s| s.secili)
            .map(|surum| {
                kur.kok()
                    .join("depo")
                    .join("taa-guncelle")
                    .join(surum)
                    .join("bin")
                    .join(&ad)
            });
        [yanimda, depoda]
            .into_iter()
            .flatten()
            .find(|y| y.is_file())
            .map(KomutGuncelleyici::yeni)
    }
}

/// Komut çıktısını tek satır Türkçe mesaja çevirir (komuttan bağımsız; testlenebilir).
/// `stdout`: `uygula --json` çıktısı; `stderr`: hata durumunda `{"hata":..,"kod":..}`.
pub fn guncelle_ciktisi_yorumla(
    ad: &str,
    basarili: bool,
    stdout: &[u8],
    stderr: &[u8],
) -> Result<String, String> {
    let cikti = String::from_utf8_lossy(stdout);
    let sonuc: Option<serde_json::Value> = serde_json::from_str(cikti.trim()).ok();
    if let Some(satir) = sonuc
        .as_ref()
        .and_then(|v| v.get("satirlar"))
        .and_then(|s| s.as_array())
        .and_then(|a| {
            a.iter()
                .find(|r| r.get("ad").and_then(|x| x.as_str()) == Some(ad))
        })
    {
        let al = |k: &str| {
            satir
                .get(k)
                .and_then(|x| x.as_str())
                .unwrap_or("")
                .to_string()
        };
        let (durum, onceki, yeni, mesaj) = (al("durum"), al("onceki"), al("yeni"), al("mesaj"));
        let geri = satir
            .get("geri_donuldu")
            .and_then(|x| x.as_bool())
            .unwrap_or(false);
        return match durum.as_str() {
            "guncellendi" => Ok(format!("{ad} güncellendi: {onceki} -> {yeni}.")),
            "secildi" => Ok(format!(
                "{ad} güncellendi: {onceki} -> {yeni} (zaten kuruluydu, indirilmedi)."
            )),
            "atlandi" => Ok(format!("{ad} güncellenmedi: {mesaj}")),
            _ => Err(format!(
                "{ad} {onceki} -> {yeni}: {mesaj}{}",
                if geri {
                    " (önceki sürüm seçili)"
                } else {
                    ""
                }
            )),
        };
    }
    if let Some(k) = sonuc.as_ref().and_then(|v| v.get("kontrol")) {
        let icerir = |alan: &str| {
            k.get(alan)
                .and_then(|x| x.as_array())
                .is_some_and(|a| a.iter().any(|x| x.as_str() == Some(ad)))
        };
        if icerir("guncel") {
            return Ok(format!("{ad} güncel; yayında daha yeni sürüm yok."));
        }
        if let Some(r) = k
            .get("reddedilen")
            .and_then(|x| x.as_array())
            .and_then(|a| {
                a.iter()
                    .find(|r| r.get("ad").and_then(|x| x.as_str()) == Some(ad))
            })
        {
            let neden = r.get("neden").and_then(|x| x.as_str()).unwrap_or("");
            return Err(format!("{ad} reddedildi: {neden}."));
        }
        if basarili {
            return Ok(format!(
                "{ad} için yayında kayıt yok. Öneri: paketin kanalını ve yayin.json'u denetleyin."
            ));
        }
    }
    let hata = String::from_utf8_lossy(stderr);
    let hata_metin: Option<String> = serde_json::from_str::<serde_json::Value>(hata.trim())
        .ok()
        .and_then(|v| v.get("hata").and_then(|h| h.as_str()).map(String::from));
    let hata_metin = hata_metin.unwrap_or_else(|| {
        let h = hata.trim();
        if h.is_empty() {
            "taa-guncelle beklenmeyen çıktı verdi. Öneri: komut satırından `taa-guncelle kontrol` çalıştırın.".into()
        } else {
            h.to_string()
        }
    });
    Err(hata_metin)
}

impl GuncelleyiciArayuzu for KomutGuncelleyici {
    fn guncelle(&self, kur: &Kur, paket_adi: &str) -> Result<String, String> {
        let mut c = Command::new(&self.komut);
        c.args(["uygula", "--json", "--paket", paket_adi])
            .env("TAA_KOK", kur.kok());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let cikti = c.output().map_err(|e| {
            format!(
                "{} çalıştırılamadı ({e}). Öneri: taa-guncelle paketini kurun.",
                self.komut.display()
            )
        })?;
        guncelle_ciktisi_yorumla(
            paket_adi,
            cikti.status.success(),
            &cikti.stdout,
            &cikti.stderr,
        )
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
    /// Rapor her zaman alınabilir (veri yoksa bölümler "yok" görünür).
    pub rapor: bool,
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
            rapor: true,
        }
    }

    /// Rapor düğmesi: paket raporunu üretir, kök\raporlar altına CSV kaydeder; pencerede gösterilecek metni döner.
    /// Durum satırına CSV yolu (ya da hata) yazılır. Kaydetme başarısız olsa bile metin döner.
    pub fn rapor(&mut self) -> String {
        match self.kur.rapor(None) {
            Err(h) => {
                self.durum = format!("Rapor alınamadı: {}", hata_metni(&h));
                self.durum.clone()
            }
            Ok(r) => {
                self.durum = match self.kur.rapor_kaydet(&r) {
                    Ok(y) => format!("Rapor CSV olarak kaydedildi: {}", y.display()),
                    Err(h) => format!("Rapor gösterildi, CSV kaydedilemedi: {}", hata_metni(&h)),
                };
                r.metin()
            }
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

    /// Güncelleme onay metni; seçili satır yoksa `None` (o durumda `guncelle` yalnız uyarı verir).
    pub fn guncelle_onay_metni(&self) -> Option<String> {
        let s = self.secili_satir()?;
        Some(format!(
            "{} paketi için güncelleme kaynağına bakılacak; yeni sürüm varsa indirilip kurulacak ve seçilecek.\nÖnceki sürüm silinmez (geri dönülebilir).\nDevam edilsin mi?",
            s.ad
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
                guncelle: false,
                rapor: true
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
                guncelle: false,
                rapor: true
            }
        );
        m.sec_satir(Some(0));
        assert_eq!(
            m.dugmeler(),
            DugmeDurumu {
                kur: true,
                kaldir: true,
                guncelle: true,
                rapor: true
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
        assert!(m
            .guncelle_onay_metni()
            .unwrap()
            .starts_with("taa-ornek paketi"));
        // güncelleme başarısızlığı kurulu paketi etkilemez
        assert_eq!(m.satirlar().len(), 1);
        assert_eq!(m.secili_indeks(), Some(0), "seçim ada göre korunur");
        m.sec_satir(None);
        assert!(m.guncelle_onay_metni().is_none());
    }

    #[test]
    fn komut_ciktisi_yorumlanir() {
        let y = |ok: bool, out: &str, err: &str| {
            guncelle_ciktisi_yorumla("a", ok, out.as_bytes(), err.as_bytes())
        };
        let guncellendi = r#"{"kontrol":{"guncellenebilir":[{"ad":"a"}],"guncel":[],"reddedilen":[]},"satirlar":[{"ad":"a","onceki":"1.0.0","yeni":"1.1.0","durum":"guncellendi","mesaj":"tamam","geri_donuldu":false}]}"#;
        assert_eq!(
            y(true, guncellendi, "").unwrap(),
            "a güncellendi: 1.0.0 -> 1.1.0."
        );
        let secildi = r#"{"kontrol":{},"satirlar":[{"ad":"a","onceki":"1.0.0","yeni":"1.1.0","durum":"secildi","mesaj":"x"}]}"#;
        assert!(y(true, secildi, "").unwrap().contains("zaten kuruluydu"));
        let atlandi = r#"{"kontrol":{},"satirlar":[{"ad":"a","onceki":"1.0.0","yeni":"1.1.0","durum":"atlandi","mesaj":"paket sabitlenmiş"}]}"#;
        assert_eq!(
            y(true, atlandi, "").unwrap(),
            "a güncellenmedi: paket sabitlenmiş"
        );
        let hata = r#"{"kontrol":{},"satirlar":[{"ad":"a","onceki":"1.0.0","yeni":"1.1.0","durum":"hata","mesaj":"SHA256 uyuşmuyor","geri_donuldu":true}]}"#;
        let e = y(false, hata, "").unwrap_err();
        assert!(
            e.contains("SHA256") && e.contains("önceki sürüm seçili"),
            "{e}"
        );
        let guncel =
            r#"{"kontrol":{"guncellenebilir":[],"guncel":["a"],"reddedilen":[]},"satirlar":[]}"#;
        assert!(y(true, guncel, "").unwrap().starts_with("a güncel"));
        let red = r#"{"kontrol":{"guncel":[],"reddedilen":[{"ad":"a","neden":"geri alma koruması"}]},"satirlar":[]}"#;
        assert!(y(true, red, "").unwrap_err().contains("geri alma"));
        let yok = r#"{"kontrol":{"guncel":[],"reddedilen":[]},"satirlar":[]}"#;
        assert!(y(true, yok, "").unwrap().contains("yayında kayıt yok"));
        // stderr JSON hatası ve düz metin
        let e = y(
            false,
            "",
            r#"{"hata":"Güncelleme kaynağı belirtilmedi: --kaynak verin","kod":2}"#,
        )
        .unwrap_err();
        assert!(e.starts_with("Güncelleme kaynağı belirtilmedi"), "{e}");
        assert!(y(false, "", "bozuk").unwrap_err().contains("bozuk"));
        assert!(y(false, "", "").unwrap_err().contains("beklenmeyen"));
    }

    #[test]
    fn komut_guncelleyici_bulma_ve_calistirma() {
        let o = Ortam::yeni();
        let kur = Kur::yeni(&o.kok).unwrap();
        // ne exe yanında ne depoda: None
        assert!(KomutGuncelleyici::bul(&kur).is_none());
        // depoda seçili taa-guncelle paketi: bin/taa-guncelle bulunur
        let mut m = o.model();
        m.kur_paketi(&o.paket("taa-guncelle", "0.1.0"));
        let bin = o
            .kok
            .join("depo")
            .join("taa-guncelle")
            .join("0.1.0")
            .join("bin");
        std::fs::create_dir_all(&bin).unwrap();
        let yol = bin.join(format!("taa-guncelle{}", std::env::consts::EXE_SUFFIX));
        std::fs::write(
            &yol,
            "#!/bin/sh\necho '{\"kontrol\":{\"guncel\":[\"taa-ornek\"]},\"satirlar\":[]}'\n",
        )
        .unwrap();
        let g = KomutGuncelleyici::bul(&kur).expect("depodaki komut bulunmalı");
        assert_eq!(g.komut(), yol);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&yol, std::fs::Permissions::from_mode(0o755)).unwrap();
            let mut m = PencereModel::yeni(Kur::yeni(&o.kok).unwrap(), Box::new(g));
            m.kur_paketi(&o.paket("taa-ornek", "0.1.0"));
            let i = m
                .satirlar()
                .iter()
                .position(|s| s.ad == "taa-ornek")
                .unwrap();
            m.sec_satir(Some(i));
            assert!(
                m.guncelle().starts_with("taa-ornek güncel"),
                "{}",
                m.durum()
            );
        }
        // çalıştırılamayan komut: tek satır hata, kurulu paketler etkilenmez
        let mut m = PencereModel::yeni(
            Kur::yeni(&o.kok).unwrap(),
            Box::new(KomutGuncelleyici::yeni(o.taban.join("yok-boyle-komut"))),
        );
        m.sec_satir(Some(0));
        assert!(m.guncelle().starts_with("Güncellenemedi:"));
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

#[cfg(test)]
mod testler_rapor {
    //! Rapor düğmesi: veri kökü olmadan da metin döner; sahte veri kökü ile fiş sayısı görünür; CSV kök\raporlar'a yazılır.
    use super::*;
    use std::fs;

    #[test]
    fn rapor_dugmesi_metin_ve_csv() {
        let taban = taa_dosya::gecici_klasor().unwrap();
        let kur = Kur::yeni(taban.join("kok")).unwrap();
        let mut m = PencereModel::yeni(kur, Box::new(KaynaksizGuncelleyici));
        assert!(m.dugmeler().rapor);
        let metin = m.rapor();
        assert!(metin.starts_with("TAA paket raporu — "), "{metin}");
        assert!(
            m.durum().starts_with("Rapor CSV olarak kaydedildi: "),
            "{}",
            m.durum()
        );
        let raporlar = taban.join("kok").join("raporlar");
        let dosyalar: Vec<_> = fs::read_dir(&raporlar).unwrap().flatten().collect();
        assert_eq!(dosyalar.len(), 1);
        let icerik = fs::read_to_string(dosyalar[0].path()).unwrap();
        assert!(icerik.starts_with("\u{feff}bolum;durum;anahtar;deger"));
        assert!(icerik.contains("fis;yok;kaynak;"));

        // Sahte veri kökü ile (Kur::rapor_veri_ile) fiş bölümü dolu.
        let veri = taban.join("veri");
        fs::create_dir_all(veri.join("fis-defteri")).unwrap();
        fs::write(
            veri.join("fis-defteri").join("fisler.jsonl"),
            "{\"tur\":\"fis\",\"v\":{\"no\":\"F-1\",\"teslim\":\"2026-01-01\",\"ucret_kurus\":100,\"on_odeme_kurus\":0,\"durum\":\"bekliyor\"}}\n",
        )
        .unwrap();
        let kur = Kur::yeni(taban.join("kok")).unwrap();
        let r = kur.rapor_veri_ile(Some(veri), Some("2026-11-06")).unwrap();
        let fis = r.bolumler.iter().find(|b| b.ad == "fis").unwrap();
        assert_eq!(fis.durum, taa_rapor::Durum::Var);
        assert!(fis
            .satirlar
            .contains(&("geciken".to_string(), "1".to_string())));
        assert!(kur.rapor_veri_ile(None, Some("kötü")).is_err());
        let _ = fs::remove_dir_all(&taban);
    }
}
