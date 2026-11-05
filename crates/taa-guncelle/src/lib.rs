// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! taa-guncelle: `yayin.json` kontrolü, indirip doğrulama (SHA256 + ed25519) ve uygulama (docs/T3-TASARIM.md §3).
//! Kural (taa-kur ile aynı): güvenilir anahtar varsa imzasız/yanlış imzalı `yayin.json` ve paket REDDEDİLİR;
//! hiç anahtar yoksa imza denetlenmez (kontrol çıktısında uyarı). Geri alma koruması: kayıtlı sürümden DÜŞÜK sürüm
//! reddedilir. Uygulama: indir → boyut+SHA256 → `Kur::kur(--sec)` → doğrula; başarısızlıkta önceki sürüme dönülür.
//! Delta yamaları şemada var ama T3'te kullanılmaz: her zaman `tam` dosya indirilir. Hiçbir dosya silinmez
//! (indirilen dosya iş bitince `eski/` altına taşınır). HTTP(S) kaynağı: `HttpKaynak` (curl ile; T4).

pub mod kaynak;
pub mod yayin;

#[cfg(test)]
mod testler;

pub use kaynak::{http_url_mu, HttpKaynak, Kaynak, YerelKaynak};
pub use yayin::{Yayin, YayinPaketi};

use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use taa_kur::kayit::surum_karsilastir;
use taa_kur::paket::sha256_dosya;
use taa_kur::{ImzaDurumu, Kur, KurHata};

type Sonuc<T> = Result<T, KurHata>;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Guncelleme {
    pub ad: String,
    pub kurulu: String,
    pub yeni: String,
    /// Kurulu sürüm `en_az_surum` altında: güncelleme zorunlu (sabitlenmiş olsa bile uygulanır).
    pub zorunlu: bool,
    /// Paket sabitlenmiş: zorunlu değilse `uygula` atlar.
    pub sabitli: bool,
    pub indirme_boyutu: u64,
    /// Kurulu sürümden yama var mı (bilgi; T3 yine de `tam` dosyayı indirir).
    pub yama_var: bool,
    /// Yeni sürüm zaten depoda kurulu (seçili değil): `uygula` indirmez, seçip doğrular.
    #[serde(default)]
    pub zaten_kurulu: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Reddedilen {
    pub ad: String,
    pub kurulu: String,
    pub yayinda: String,
    pub neden: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct KontrolSonuc {
    pub kanal: String,
    /// "dogrulandi" ya da "denetlenmedi" (güvenilir anahtar yok).
    pub imza: String,
    pub guncellenebilir: Vec<Guncelleme>,
    /// Zaten yayındaki sürümde olan paketler.
    pub guncel: Vec<String>,
    pub reddedilen: Vec<Reddedilen>,
    pub uyarilar: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UygulaSatiri {
    pub ad: String,
    pub onceki: String,
    pub yeni: String,
    /// "guncellendi", "secildi" (zaten kuruluydu, indirme yok), "atlandi" ya da "hata".
    pub durum: String,
    pub mesaj: String,
    /// Hata sonrası önceki sürüm yeniden seçildi mi.
    pub geri_donuldu: bool,
    /// Hatanın çıkış kodu (taa-kur ile aynı: 2 doğrulama, 4 yok, 5 bozuk, 1 diğer); hata yoksa 0.
    #[serde(default)]
    pub kod: i32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UygulaSonuc {
    pub kontrol: KontrolSonuc,
    pub satirlar: Vec<UygulaSatiri>,
}

impl UygulaSonuc {
    pub fn hata_var(&self) -> bool {
        self.satirlar.iter().any(|s| s.durum == "hata")
    }

    /// CLI çıkış kodu: hata yoksa 0; varsa ilk hatalı satırın kodu (kontrol aşaması hataları zaten `Err` ile çıkar).
    pub fn cikis_kodu(&self) -> i32 {
        self.satirlar
            .iter()
            .find(|s| s.durum == "hata")
            .map(|s| if s.kod == 0 { 1 } else { s.kod })
            .unwrap_or(0)
    }
}

fn yayin_yukle(kur: &Kur, kaynak: &dyn Kaynak) -> Sonuc<(Yayin, ImzaDurumu)> {
    let bayt = kaynak.yayin_oku()?;
    let (y, ham) = Yayin::coz(&bayt)?;
    let durum = kur.imza_denetle(
        "Yayın dosyası (yayin.json)",
        &taa_imza::kanonik_bayt(&ham),
        ham.get("imza"),
    )?;
    Ok((y, durum))
}

/// Kur kaydı ile `yayin.json` karşılaştırması. Yalnız kurulu ve seçili paketler değerlendirilir; yazma yapmaz.
pub fn kontrol(kur: &Kur, kaynak: &dyn Kaynak, kanal: &str) -> Sonuc<KontrolSonuc> {
    kontrol_paket(kur, kaynak, kanal, None)
}

/// `kontrol` ile aynı; `yalniz` verilirse yalnız o paket değerlendirilir (pencere "Güncelle" düğmesi).
pub fn kontrol_paket(
    kur: &Kur,
    kaynak: &dyn Kaynak,
    kanal: &str,
    yalniz: Option<&str>,
) -> Sonuc<KontrolSonuc> {
    let (yayin, durum) = yayin_yukle(kur, kaynak)?;
    let mut s = KontrolSonuc {
        kanal: kanal.into(),
        imza: match durum {
            ImzaDurumu::Dogrulandi(_) => "dogrulandi".into(),
            ImzaDurumu::Denetlenmedi => "denetlenmedi".into(),
        },
        guncellenebilir: vec![],
        guncel: vec![],
        reddedilen: vec![],
        uyarilar: vec![],
    };
    if durum == ImzaDurumu::Denetlenmedi {
        s.uyarilar.push(
            "Güvenilir anahtar yok (anahtarlar/*.pub): yayin.json ve paket imzaları denetlenmiyor."
                .into(),
        );
    }
    for satir in kur.liste()? {
        if yalniz.is_some_and(|y| y != satir.ad) {
            continue;
        }
        let Some(secili) = satir.secili.clone() else {
            continue;
        };
        // Kanalda bu paketin en yüksek sürümü.
        let Some(aday) = yayin
            .paketler
            .iter()
            .filter(|p| p.ad == satir.ad && p.kanal == kanal)
            .max_by(|a, b| surum_karsilastir(&a.surum, &b.surum))
        else {
            continue;
        };
        match surum_karsilastir(&aday.surum, &secili) {
            std::cmp::Ordering::Less => s.reddedilen.push(Reddedilen {
                ad: satir.ad.clone(),
                kurulu: secili,
                yayinda: aday.surum.clone(),
                neden: "yayındaki sürüm kurulu sürümden düşük (geri alma koruması)".into(),
            }),
            std::cmp::Ordering::Equal => s.guncel.push(satir.ad.clone()),
            std::cmp::Ordering::Greater => {
                let zorunlu = aday
                    .en_az_surum
                    .as_deref()
                    .is_some_and(|e| surum_karsilastir(&secili, e).is_lt());
                s.guncellenebilir.push(Guncelleme {
                    ad: satir.ad.clone(),
                    yama_var: aday.yamalar.iter().any(|y| y.kaynak_surum == secili),
                    zaten_kurulu: satir.kurulu.contains(&aday.surum),
                    kurulu: secili,
                    yeni: aday.surum.clone(),
                    zorunlu,
                    sabitli: satir.sabitle,
                    indirme_boyutu: aday.tam.boyut,
                });
            }
        }
    }
    Ok(s)
}

/// `kontrol` + her güncellenebilir paket için indir/doğrula/kur. Paket başına hata diğerlerini durdurmaz;
/// sonuç satırlarında bildirilir. Kontrol aşamasındaki hata (imza, kaynak) tümünü durdurur.
pub fn uygula(kur: &Kur, kaynak: &dyn Kaynak, kanal: &str) -> Sonuc<UygulaSonuc> {
    uygula_paket(kur, kaynak, kanal, None)
}

/// `uygula` ile aynı; `yalniz` verilirse yalnız o paket güncellenir.
pub fn uygula_paket(
    kur: &Kur,
    kaynak: &dyn Kaynak,
    kanal: &str,
    yalniz: Option<&str>,
) -> Sonuc<UygulaSonuc> {
    let kontrol_sonucu = kontrol_paket(kur, kaynak, kanal, yalniz)?;
    let (yayin, _) = yayin_yukle(kur, kaynak)?;
    let mut satirlar = Vec::new();
    for g in &kontrol_sonucu.guncellenebilir {
        let mut satir = UygulaSatiri {
            ad: g.ad.clone(),
            onceki: g.kurulu.clone(),
            yeni: g.yeni.clone(),
            durum: "guncellendi".into(),
            mesaj: "tamam".into(),
            geri_donuldu: false,
            kod: 0,
        };
        if g.sabitli && !g.zorunlu {
            satir.durum = "atlandi".into();
            satir.mesaj = "paket sabitlenmiş (taa-kur sabitle); zorunlu güncelleme değil".into();
            satirlar.push(satir);
            continue;
        }
        let Some(p) = yayin
            .paketler
            .iter()
            .find(|p| p.ad == g.ad && p.surum == g.yeni && p.kanal == kanal)
        else {
            satir.durum = "hata".into();
            satir.mesaj = "yayında paket bulunamadı".into();
            satirlar.push(satir);
            continue;
        };
        let sonuc = if g.zaten_kurulu {
            satir.durum = "secildi".into();
            satir.mesaj = format!(
                "{} zaten kuruluydu; indirilmeden seçildi ve doğrulandı",
                g.yeni
            );
            kuruluyu_sec(kur, &p.ad, &p.surum, &g.kurulu)
        } else {
            paketi_uygula(kur, kaynak, p, &g.kurulu)
        };
        if let Err((e, geri)) = sonuc {
            satir.durum = "hata".into();
            satir.mesaj = e.mesaj().to_string();
            satir.geri_donuldu = geri;
            satir.kod = e.cikis_kodu();
        }
        satirlar.push(satir);
    }
    Ok(UygulaSonuc {
        kontrol: kontrol_sonucu,
        satirlar,
    })
}

/// Yeni sürüm depoda zaten varsa indirme yapılmaz: seçilir ve doğrulanır; doğrulanamazsa önceki sürüme dönülür.
fn kuruluyu_sec(kur: &Kur, ad: &str, surum: &str, onceki: &str) -> Result<(), (KurHata, bool)> {
    kur.sec(ad, surum).map_err(|e| (e, false))?;
    let hata = match kur.dogrula(Some(ad)) {
        Ok(sorunlar) if sorunlar.is_empty() => return Ok(()),
        Ok(sorunlar) => KurHata::Bozuk(format!(
            "{ad} {surum} depoda kurulu ama doğrulanamadı ({} sorun). Öneri: `taa-kur kaldir {ad}` sonra yeniden güncelleyin.",
            sorunlar.len()
        )),
        Err(e) => e,
    };
    let geri = geri_don(kur, ad, onceki);
    let ek = if geri {
        format!(" Önceki sürüme ({onceki}) dönüldü.")
    } else {
        String::new()
    };
    Err((hata.yeniden(format!("{}{ek}", tek(hata.mesaj()))), geri))
}

fn sayac() -> u64 {
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

/// Tek paketi indirir, doğrular, kurar. Hata: (hata, önceki sürüme dönüldü mü).
fn paketi_uygula(
    kur: &Kur,
    kaynak: &dyn Kaynak,
    p: &YayinPaketi,
    onceki: &str,
) -> Result<(), (KurHata, bool)> {
    if !p.tam.url.to_ascii_lowercase().ends_with(".zip") {
        return Err((
            KurHata::Dogrulama(format!(
                "{} için tam paket .zip olmalı. Öneri: yayıncıdan zip paket isteyin.",
                p.ad
            )),
            false,
        ));
    }
    let indirilen = indirme_yolu(kur, &p.ad, &p.surum).map_err(|e| (e, false))?;
    let sonuc = indir_ve_kur(kur, kaynak, p, onceki, &indirilen);
    // Başarılı indirme eski\ altında kalır (silme yok); başarısız/bozuk olanlar eski\indirilen-hatali\ altına ayrılır.
    if let Err(e) = eskiye_tasi(kur, &indirilen, sonuc.is_err()) {
        return sonuc.and(Err((e, false)));
    }
    sonuc
}

fn indirme_yolu(kur: &Kur, ad: &str, surum: &str) -> Sonuc<PathBuf> {
    let dizin = kur.kok().join("indirilen");
    fs::create_dir_all(&dizin)?;
    Ok(dizin.join(format!(
        "{ad}-{surum}-{}-{}.zip",
        std::process::id(),
        sayac()
    )))
}

fn indir_ve_kur(
    kur: &Kur,
    kaynak: &dyn Kaynak,
    p: &YayinPaketi,
    onceki: &str,
    dosya: &Path,
) -> Result<(), (KurHata, bool)> {
    let h = |e: KurHata| (e, false);
    let n = kaynak.getir(&p.tam.url, dosya, p.tam.boyut).map_err(h)?;
    if n != p.tam.boyut {
        return Err(h(KurHata::Dogrulama(format!(
            "{} {} indirilen boyut ({n}) yayin.json'daki boyutla ({}) uyuşmuyor. Öneri: yayını yeniden indirin.",
            p.ad, p.surum, p.tam.boyut
        ))));
    }
    let (sha, _) = sha256_dosya(dosya).map_err(|e| h(e.into()))?;
    if sha != p.tam.sha256 {
        return Err(h(KurHata::Dogrulama(format!(
            "{} {} indirilen dosyanın SHA256 değeri yayin.json ile uyuşmuyor. Öneri: yeniden indirin; sürerse yayıncıya bildirin.",
            p.ad, p.surum
        ))));
    }
    // Paket imzası (varsa güvenilir anahtarla) Kur::kur içinde denetlenir.
    let kurma = kur.kur(dosya, true);
    let hata = match kurma {
        Ok(s) if s.ad == p.ad && s.surum == p.surum && s.secili => match kur.dogrula(Some(&p.ad)) {
            Ok(sorunlar) if sorunlar.is_empty() => return Ok(()),
            Ok(sorunlar) => KurHata::Bozuk(format!(
                "{} {} kurulumdan sonra doğrulanamadı ({} sorun).",
                p.ad,
                p.surum,
                sorunlar.len()
            )),
            Err(e) => e,
        },
        Ok(s) => KurHata::Dogrulama(format!(
            "Zip içindeki paket ({} {}) yayin.json'daki ({} {}) ile uyuşmuyor.",
            s.ad, s.surum, p.ad, p.surum
        )),
        Err(e) => e,
    };
    let geri = geri_don(kur, &p.ad, onceki);
    let ek = if geri {
        format!(" Önceki sürüme ({onceki}) dönüldü.")
    } else {
        String::new()
    };
    Err((hata.yeniden(format!("{}{ek}", tek(hata.mesaj()))), geri))
}

fn tek(m: &str) -> String {
    m.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Seçili sürümü `onceki`ye döndürür. Zaten seçiliyse (kur hiç değiştirmediyse) true döner.
fn geri_don(kur: &Kur, ad: &str, onceki: &str) -> bool {
    let secili = || {
        kur.liste()
            .ok()
            .and_then(|l| l.into_iter().find(|s| s.ad == ad))
            .and_then(|s| s.secili)
    };
    if secili().as_deref() == Some(onceki) {
        return true;
    }
    if kur.geri_don(ad).is_ok() && secili().as_deref() == Some(onceki) {
        return true;
    }
    kur.sec(ad, onceki).is_ok() && secili().as_deref() == Some(onceki)
}

/// İndirilen dosyayı silmez: `<kök>/eski/indirilen-<ad>` altına taşır; `hatali` ise `<kök>/eski/indirilen-hatali/`.
fn eskiye_tasi(kur: &Kur, dosya: &Path, hatali: bool) -> Sonuc<()> {
    if !dosya.exists() {
        return Ok(());
    }
    let dizin = if hatali {
        kur.kok().join("eski").join("indirilen-hatali")
    } else {
        kur.kok().join("eski")
    };
    fs::create_dir_all(&dizin)?;
    let ad = dosya
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut hedef = dizin.join(format!("indirilen-{ad}"));
    let mut n = 2;
    while hedef.exists() {
        hedef = dizin.join(format!("indirilen-{n}-{ad}"));
        n += 1;
    }
    fs::rename(dosya, hedef)?;
    Ok(())
}
