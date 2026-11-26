//! Eski teslim kayıtlarının şifrelenmesi (Yusuf 2026-11-26: "eski teslim kayıtları da şifrelensin"). Bir kezlik,
//! idempotent geçiş; sunucu açılışında (istek almadan önce) kendiliğinden çalışır. Defterin yeniden yazımı YALNIZ
//! açılışta yapılır: sunucu çalışırken yeniden yazmak, okuma ile yeniden adlandırma arasında eklenen satırı
//! (yeni fiş, durum, teslim) kaybettirebilirdi (denetçi bulgusu). Yönetici `POST /api/yonetim/teslim-gecis` ile
//! yalnız imza dosyalarının şifreli kopyasını yazdırabilir; `GET /api/yonetim/teslim-gecis` durumu verir
//! (masaüstü Yönetim ekranında uyarı kartı). Defter tarafı gerekiyorsa yanıt "yeniden başlatın" der.
//!
//! Ne yapar:
//! 1. İmzalar: `imzalar/<no>-<n>.png` (şifresiz) için `imzalar/<no>-<n>.png.enc` (yerel anahtar, AES-256-GCM,
//!    aad = fiş no) yazılır ve geri okunup doğrulanır. ŞİFRESİZ PNG SİLİNMEZ (kalıcı silme yok; Geri Dönüşüm
//!    Kutusu kuralı): `imzalar/silinecek-duz-imzalar.json` listesine yazılır, yönetici onaylayıp kendisi siler
//!    (Windows'ta Geri Dönüşüm Kutusu'na). Liste silinene kadar şifresiz imza diskte KALIR.
//! 2. Defter (`fisler.jsonl`, yalnız eklemeli): düz metin `teslim_alan` taşıyan teslim kaydı satırları şifreli
//!    biçime (`sifreli`, aad = o satırın fiş no'su) çevrilir; imza yolu ".png" olan satırlar şifreli kopyayı
//!    (".png.enc") gösterecek biçimde güncellenir. Bunun için defter BİR KEZ yeniden yazılır:
//!    a. önce defterin TAMAMI zaman damgalı yedeğe alınır: `yedek/fisler-<YYYYMMDD-HHMMSS>.jsonl.enc`. Yedek de
//!    düz metin içerdiğinden yerel anahtarla tek blok AES-256-GCM ile ŞİFRELİ yazılır (aad =
//!    "yedek:<dosya adı>") ve geri okunup doğrulanır; doğrulanamazsa defter değişmez;
//!    b. sonra defter atomik olarak (geçici dosya + yeniden adlandırma) yeniden yazılır; diğer satırlar bayt bayt
//!    aynı kalır.
//!    "Eski satırı geçersiz kılan yeni olay" yolu seçilmedi: düz metin dosyada kalırdı.
//! - Zincir (TT1): geçiş zincir başlatılmadan ÖNCE tamamlanmalıdır. Zincir başlatılmışsa yeniden yazım merkezdeki
//!   ucu bozar; bu yüzden zincir varken defter yeniden yazılmaz (`zincir_onayi` = false), rapor
//!   `zincir_onayi_gerekli` der; yönetici açıkça onaylarsa (`--teslim-gecis-zincir-onay` açılış seçeneği) yazılır.
//!   Bugün zincir üretim kodunda başlatılmıyor.
//! - İkinci çalıştırmada değişecek satır yoksa hiçbir şey yazılmaz, yedek alınmaz.
//! - Hiçbir günlüğe kişisel veri yazılmaz (yalnız sayılar).
//!
//! Kalan düz metin (bilerek): (a) yönetici silene kadar şifresiz imza PNG'leri (liste dosyası); (b) veri klasörü
//! dışındaki eski yedekler/kopyalar (bu program onlara erişmez); (c) atomik yeniden yazım eski dosyanın disk
//! bloklarını güvenle silmez (dosya sistemi düzeyi).
//!
//! Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

use super::*;
use crate::teslim_al::{imza_dosya_adi_coz, sha256_bayt};

/// Yönetici onayıyla silinecek şifresiz imza listesi (imzalar/ altında).
pub const SILINECEK_LISTESI: &str = "silinecek-duz-imzalar.json";

/// Geçişin sonucu (yalnız sayılar ve dosya adları; kişisel veri yok).
#[derive(Debug, Default, Clone, PartialEq, Eq, serde::Serialize)]
pub struct GecisRaporu {
    /// Düz metin teslim bilgisi şifrelenen defter satırı.
    pub defter_sifrelenen: usize,
    /// Yeniden yazılan defter satırı (şifrelenen + yalnız imza yolu güncellenen).
    pub defter_yeniden_yazilan: usize,
    /// Bu çalıştırmada şifreli kopyası yazılan imza.
    pub imza_sifrelenen: usize,
    /// Şifreli kopyası doğrulanmış, yönetici onayıyla silinmeyi bekleyen şifresiz imza dosyaları (göreli yol).
    pub duz_imza_bekleyen: Vec<String>,
    /// Şifreli kopyası tutmayan ya da adı tanınmayan şifresiz imzalar (dokunulmadı).
    pub sorunlu_imza: Vec<String>,
    /// Alınan şifreli defter yedeği (göreli yol), defter yeniden yazıldıysa.
    pub yedek: Option<String>,
    /// Defter zinciri başlatılmış olduğu için yeniden yazım yapılmadı (yöneticinin açık onayı gerekir).
    pub zincir_onayi_gerekli: bool,
}

fn zaman_damgasi(epoch: u64) -> String {
    format!(
        "{}-{}",
        tarih_iso(epoch).replace('-', ""),
        saat_iso(epoch).replace(':', "")
    )
}

/// Şifresiz imzaların şifreli kopyasını yazar/doğrular; silinme listesini günceller.
fn imzalari_sifrele(dizin: &Path, simdi: u64, r: &mut GecisRaporu) -> Result<(), String> {
    let klasor = dizin.join("imzalar");
    let Ok(okuyucu) = std::fs::read_dir(&klasor) else {
        return Ok(());
    };
    let mut adlar: Vec<String> = okuyucu
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|a| a.ends_with(".png"))
        .collect();
    adlar.sort();
    let mut liste = Vec::new();
    for ad in adlar {
        let Some((no, _)) = imza_dosya_adi_coz(&ad) else {
            r.sorunlu_imza.push(format!("imzalar/{ad}"));
            continue;
        };
        let duz = std::fs::read(klasor.join(&ad)).map_err(|e| format!("imza okunamadı: {e}"))?;
        let hedef = klasor.join(format!("{ad}.enc"));
        if hedef.exists() {
            let tutar = std::fs::read(&hedef)
                .ok()
                .and_then(|b| teslim_sifre::bayt_coz(dizin, no.as_bytes(), &b).ok())
                .is_some_and(|b| b == duz);
            if !tutar {
                r.sorunlu_imza.push(format!("imzalar/{ad}"));
                continue;
            }
        } else {
            let sifreli = teslim_sifre::bayt_sifrele(dizin, no.as_bytes(), &duz)?;
            taa_dosya::uzerine_yaz_atomik(&hedef, &sifreli)
                .map_err(|e| format!("şifreli imza yazılamadı: {e}"))?;
            let geri = std::fs::read(&hedef).map_err(|e| e.to_string())?;
            if teslim_sifre::bayt_coz(dizin, no.as_bytes(), &geri)? != duz {
                return Err("şifreli imza doğrulanamadı".into());
            }
            r.imza_sifrelenen += 1;
        }
        r.duz_imza_bekleyen.push(format!("imzalar/{ad}"));
        liste.push(json!({
            "dosya": format!("imzalar/{ad}"),
            "sifreli_kopya": format!("imzalar/{ad}.enc"),
            "sha256": sha256_bayt(&duz),
        }));
    }
    let liste_yolu = klasor.join(SILINECEK_LISTESI);
    if !liste.is_empty() || liste_yolu.exists() {
        let v = json!({
            "aciklama": "Bu şifresiz imza dosyalarının şifreli kopyası (.png.enc) yazıldı ve doğrulandı. Program onları SİLMEZ: yönetici onaylayıp kendisi siler (Windows'ta Geri Dönüşüm Kutusu'na). Silinene kadar şifresiz imza diskte kalır.",
            "guncelleme": format!("{} {}", tarih_iso(simdi), saat_iso(simdi)),
            "dosyalar": liste,
        });
        let metin = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
        taa_dosya::uzerine_yaz_atomik(&liste_yolu, metin.as_bytes())
            .map_err(|e| format!("silinecek listesi yazılamadı: {e}"))?;
    }
    Ok(())
}

/// Defter yedeğini şifreli yazar ve doğrular; göreli yolu döndürür.
fn yedek_yaz(dizin: &Path, simdi: u64, eski: &[u8]) -> Result<String, String> {
    let klasor = dizin.join(teslim_sifre::YEDEK_KLASORU);
    std::fs::create_dir_all(&klasor).map_err(|e| format!("yedek klasörü açılamadı: {e}"))?;
    let kok = format!("fisler-{}", zaman_damgasi(simdi));
    let ad = (1..1000)
        .map(|n| {
            if n == 1 {
                format!("{kok}.jsonl.enc")
            } else {
                format!("{kok}-{n}.jsonl.enc")
            }
        })
        .find(|a| !klasor.join(a).exists())
        .ok_or("yedek adı bulunamadı")?;
    let aad = format!("yedek:{ad}");
    let sifreli = teslim_sifre::bayt_sifrele(dizin, aad.as_bytes(), eski)?;
    let yol = klasor.join(&ad);
    taa_dosya::uzerine_yaz_atomik(&yol, &sifreli).map_err(|e| format!("yedek yazılamadı: {e}"))?;
    let geri = std::fs::read(&yol).map_err(|e| format!("yedek okunamadı: {e}"))?;
    if teslim_sifre::bayt_coz(dizin, aad.as_bytes(), &geri)? != eski {
        return Err("yedek doğrulanamadı; defter değiştirilmedi".into());
    }
    Ok(format!("{}/{ad}", teslim_sifre::YEDEK_KLASORU))
}

/// Şifreli defter yedeğini çözer (geri yükleme için; yedek dosyasının veri klasörüne göre yolu).
pub fn yedek_coz(dizin: &Path, goreli: &str) -> Result<Vec<u8>, String> {
    let ad = goreli
        .strip_prefix(&format!("{}/", teslim_sifre::YEDEK_KLASORU))
        .filter(|a| !a.contains(['/', '\\']) && a.ends_with(".jsonl.enc"))
        .ok_or("yedek yolu geçersiz")?;
    let b = std::fs::read(dizin.join(teslim_sifre::YEDEK_KLASORU).join(ad))
        .map_err(|e| format!("yedek okunamadı: {e}"))?;
    teslim_sifre::bayt_coz(dizin, format!("yedek:{ad}").as_bytes(), &b)
}

/// Kayıttaki ".png" imzanın şifreli kopyası var, çözülüyor ve özeti kayıttakiyle aynı mı?
fn sifreli_kopya_tutar(dizin: &Path, no: &str, k: &taa_fis::TeslimKaydi) -> bool {
    let sahip = if k.ana_fis.is_empty() {
        no
    } else {
        k.ana_fis.as_str()
    };
    let Some(ad) = k
        .imza_dosyasi
        .strip_prefix("imzalar/")
        .filter(|a| imza_dosya_adi_coz(a).is_some_and(|(n, _)| n == sahip))
    else {
        return false;
    };
    std::fs::read(dizin.join("imzalar").join(format!("{ad}.enc")))
        .ok()
        .and_then(|b| teslim_sifre::imza_coz(dizin, sahip, &k.teslim_id, &b).ok())
        .is_some_and(|png| sha256_bayt(&png) == k.imza_sha256)
}

/// Yalnız imza dosyalarının şifreli kopyasını yazar (sunucu çalışırken güvenli; defterde değişiklik yok).
pub fn imzalari_calistir(dizin: &Path, simdi: u64) -> Result<GecisRaporu, String> {
    let mut r = GecisRaporu::default();
    imzalari_sifrele(dizin, simdi, &mut r)?;
    Ok(r)
}

/// Geçişi çalıştırır (idempotent). YALNIZ sunucu açılışında, istek almadan önce çağrılmalı. Hata olursa defter
/// değişmemiş olur (yedek doğrulanmadan yazılmaz; okuma sonrası defter değiştiyse yazılmaz). `zincir_onayi`:
/// defter zinciri (TT1) başlatılmışsa yeniden yazım için yöneticinin açık onayı.
pub fn calistir(dizin: &Path, simdi: u64, zincir_onayi: bool) -> Result<GecisRaporu, String> {
    let mut r = GecisRaporu::default();
    imzalari_sifrele(dizin, simdi, &mut r)?;
    let yol = dizin.join("fisler.jsonl");
    let d = Defter { yol: &yol };
    if !zincir_onayi && d.zincir_ucu()?.is_some() && durum(dizin)?["gecis_gerekli"] == true {
        r.zincir_onayi_gerekli = true;
        return Ok(r);
    }
    let mut sifrelenen = 0usize;
    let mut yedek = None;
    let n = d.teslim_kayitlarini_yeniden_yaz(
        |no, k| {
            let mut y = k.clone();
            let mut degisti = false;
            if y.sifreli.is_none() && !y.teslim_alan.trim().is_empty() {
                let duz = teslim_sifre::TeslimDuz {
                    ad: y
                        .teslim_alan
                        .split_whitespace()
                        .collect::<Vec<_>>()
                        .join(" "),
                    yontem: if y.imza_sha256.is_empty() {
                        "elle"
                    } else {
                        "ekran-imza"
                    }
                    .into(),
                    ..Default::default()
                };
                y.sifreli = Some(teslim_sifre::sifrele(dizin, no, &y.teslim_id, &duz)?);
                y.teslim_alan = String::new();
                sifrelenen += 1;
                degisti = true;
            }
            // İmza yolu: şifreli kopya çözülüp özeti kayıtla tutuyorsa ".png.enc" gösterir (kurcalanmışsa dokunulmaz).
            if y.imza_dosyasi.ends_with(".png") && sifreli_kopya_tutar(dizin, no, &y) {
                y.imza_dosyasi.push_str(".enc");
                degisti = true;
            }
            if degisti && !y.imza_sha256.is_empty() && y.imza_yontemi.is_empty() {
                // Geçiş öncesi imzalar yalnız ekrandan alınırdı (karekod yolunda imza yoktu).
                y.imza_yontemi = "cihaz_imza".into();
            }
            Ok(degisti.then_some(y))
        },
        |eski| {
            yedek = Some(yedek_yaz(dizin, simdi, eski)?);
            Ok(())
        },
    )?;
    r.defter_yeniden_yazilan = n;
    r.defter_sifrelenen = sifrelenen;
    r.yedek = yedek;
    Ok(r)
}

/// Geçiş gerekiyor mu? (düz metin teslim satırı, ".png" gösteren satır, şifresiz imza dosyası sayıları)
pub fn durum(dizin: &Path) -> Result<serde_json::Value, String> {
    let yol = dizin.join("fisler.jsonl");
    let fisler = Defter { yol: &yol }.fisler().unwrap_or_default();
    let duz_satir = fisler
        .iter()
        .filter_map(|f| f.teslim_kaydi.as_ref())
        .filter(|t| t.sifreli.is_none() && !t.teslim_alan.trim().is_empty())
        .count();
    let png_yolu = fisler
        .iter()
        .filter_map(|f| f.teslim_kaydi.as_ref())
        .filter(|t| t.imza_dosyasi.ends_with(".png"))
        .count();
    let duz_imza: Vec<String> = std::fs::read_dir(dizin.join("imzalar"))
        .map(|r| {
            let mut v: Vec<String> = r
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|a| a.ends_with(".png"))
                .map(|a| format!("imzalar/{a}"))
                .collect();
            v.sort();
            v
        })
        .unwrap_or_default();
    Ok(json!({
        "duz_metin_teslim_satiri": duz_satir,
        "png_gosteren_satir": png_yolu,
        "duz_imza_dosyalari": duz_imza,
        "silinecek_listesi": format!("imzalar/{SILINECEK_LISTESI}"),
        "gecis_gerekli": duz_satir > 0 || png_yolu > 0,
    }))
}

impl Uygulama {
    /// GET /api/yonetim/teslim-gecis: geçiş durumu (yalnız sayılar ve dosya adları).
    pub(crate) fn teslim_gecis_durumu(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        match durum(&self.dizin) {
            Ok(v) => Yanit::json(200, v),
            Err(e) => Yanit::hata(500, &e),
        }
    }

    /// POST /api/yonetim/teslim-gecis: yalnız imza dosyalarının şifreli kopyasını yazar (idempotent). Defterin
    /// yeniden yazımı sunucu çalışırken yapılmaz (eklenen satır kaybolabilirdi); gerekiyorsa yanıt
    /// `yeniden_baslat: true` der: program yeniden açılınca açılış geçişi yapar.
    pub(crate) fn teslim_gecis_calistir(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        match imzalari_calistir(&self.dizin, self.simdi()) {
            Ok(r) => {
                let gerekli = durum(&self.dizin)
                    .map(|v| v["gecis_gerekli"] == true)
                    .unwrap_or(false);
                Yanit::json(
                    200,
                    json!({ "tamam": true, "rapor": r, "yeniden_baslat": gerekli }),
                )
            }
            Err(e) => Yanit::hata(500, &format!("imza geçişi yapılamadı: {e}")),
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    const KISILER: &str = r#"{"tur":"ekle","v":{"id":"K-test0001","ad":"Deneme Danışman","rol":"danisman","tarih":"2026-11-01"}}"#;

    /// Eski biçimde (geçiş öncesi) bir defter ve şifresiz imza kurar: düz metin ad, ".png" imza.
    fn eski_kurulum() -> (PathBuf, String, String, Vec<u8>) {
        let d = taa_dosya::gecici_klasor().unwrap();
        std::fs::write(d.join("sube.json"), r#"{"kod":"MRK01"}"#).unwrap();
        std::fs::write(d.join("kisiler.jsonl"), KISILER).unwrap();
        let yol = d.join("fisler.jsonl");
        let df = Defter { yol: &yol };
        let g: taa_fis::Girdi = serde_json::from_str(
            r#"{"satis_danismani_id":"K-test0001","olcu_alan_id":"K-test0001","duzenleyen_id":"K-test0001","musteri":"Deneme Müşteri","telefon":"0532 000 45 67","kalemler":[{"urun":"Pantolon","islem":"paça kısalt"}],"tarih":"2026-11-01","sube":"MRK01","teslim":"2026-11-05","ucret_kurus":0,"on_odeme_kurus":0}"#,
        )
        .unwrap();
        let a = df.ekle(g.clone()).unwrap().no;
        let b = df.ekle(g).unwrap().no;
        // Sahte PNG (8 bayt imza + IHDR + sahte gövde).
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&400u32.to_be_bytes());
        png.extend_from_slice(&150u32.to_be_bytes());
        png.extend_from_slice(b"\x08\x06\0\0\0\0\0\0\0ESKI-DENEME-IMZA");
        std::fs::create_dir_all(d.join("imzalar")).unwrap();
        std::fs::write(d.join("imzalar").join(format!("{a}-1.png")), &png).unwrap();
        // Eski biçim satırlar (düz metin ad; teslim_et artık bu biçimi kabul etmediği için doğrudan yazılır).
        let satir = |no: &str, ad: &str, imza: bool| {
            let mut k = json!({
                "teslim_alan": ad, "iliski": "yakini", "fisli": !imza,
                "dogrulama": if imza { "telefon-son4" } else { "fis" },
                "kim": "PC", "tarih": "2026-11-05", "saat": "10:00:00",
            });
            if imza {
                k["imza_sha256"] = json!(sha256_bayt(&png));
                k["imza_dosyasi"] = json!(format!("imzalar/{no}-1.png"));
            }
            json!({ "tur": "durum", "v": { "no": no, "durum": "hazir", "tarih": "2026-11-04" } })
                .to_string()
                + "\n"
                + &json!({ "tur": "teslimkaydi", "v": { "no": no, "kayit": k } }).to_string()
        };
        let mut m = std::fs::read_to_string(&yol).unwrap();
        m.push_str(&satir(&a, "Eski Örnekkişi Bir", true));
        m.push('\n');
        m.push_str(&satir(&b, "Eski Örnekkişi İki", false));
        m.push('\n');
        std::fs::write(&yol, m).unwrap();
        (d, a, b, png)
    }

    fn icerir(k: &Path, aranan: &[&str]) -> Vec<String> {
        let mut bulunan = Vec::new();
        fn tara(k: &Path, aranan: &[&str], b: &mut Vec<String>) {
            for e in std::fs::read_dir(k).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    tara(&p, aranan, b);
                } else {
                    let v = std::fs::read(&p).unwrap();
                    for a in aranan {
                        if v.windows(a.len()).any(|w| w == a.as_bytes()) {
                            b.push(format!("{} içinde {a}", p.display()));
                        }
                    }
                }
            }
        }
        tara(k, aranan, &mut bulunan);
        bulunan
    }

    #[test]
    fn eski_kayitlar_sifrelenir_yedek_sifreli_ve_idempotent() {
        let (d, a, b, png) = eski_kurulum();
        let once = std::fs::read(d.join("fisler.jsonl")).unwrap();
        let st = durum(&d).unwrap();
        assert_eq!(st["duz_metin_teslim_satiri"], 2);
        assert_eq!(st["png_gosteren_satir"], 1);
        assert_eq!(st["gecis_gerekli"], true);
        let r = calistir(&d, 1_795_000_000, false).unwrap();
        assert_eq!(r.defter_sifrelenen, 2);
        assert_eq!(r.defter_yeniden_yazilan, 2);
        assert_eq!(r.imza_sifrelenen, 1);
        assert_eq!(r.duz_imza_bekleyen, vec![format!("imzalar/{a}-1.png")]);
        assert!(r.sorunlu_imza.is_empty());
        let yedek = r.yedek.clone().unwrap();
        assert!(yedek.starts_with("yedek/fisler-") && yedek.ends_with(".jsonl.enc"));
        // yedek şifreli; çözünce eski defterin aynısı
        assert_eq!(yedek_coz(&d, &yedek).unwrap(), once);
        // defterde ve yedekte düz metin ad yok; şifresiz imza yalnız silinme listesindeki dosyada
        let bulunan = icerir(&d, &["Örnekkişi", "rnekki", "ESKI-DENEME-IMZA"]);
        assert_eq!(bulunan.len(), 1, "{bulunan:?}");
        assert!(
            bulunan[0].contains(&format!("{a}-1.png içinde ESKI")),
            "{bulunan:?}"
        );
        // kayıtlar çözülür; imza yolu şifreli kopyayı gösterir, yöntem cihaz_imza
        let df = Defter {
            yol: &d.join("fisler.jsonl"),
        };
        let ta = df.bul(&a).unwrap().unwrap().teslim_kaydi.unwrap();
        assert!(ta.teslim_alan.is_empty());
        assert_eq!(ta.imza_dosyasi, format!("imzalar/{a}-1.png.enc"));
        assert_eq!(ta.imza_yontemi, "cihaz_imza");
        let duz = teslim_sifre::coz(&d, &a, &ta.teslim_id, ta.sifreli.as_ref().unwrap()).unwrap();
        assert_eq!(
            (duz.ad.as_str(), duz.yontem.as_str()),
            ("Eski Örnekkişi Bir", "ekran-imza")
        );
        let tb = df.bul(&b).unwrap().unwrap().teslim_kaydi.unwrap();
        let duz = teslim_sifre::coz(&d, &b, &tb.teslim_id, tb.sifreli.as_ref().unwrap()).unwrap();
        assert_eq!(
            (duz.ad.as_str(), duz.yontem.as_str()),
            ("Eski Örnekkişi İki", "elle")
        );
        // şifreli kopya doğru; şifresiz dosya silinmedi, listede
        let enc = std::fs::read(d.join("imzalar").join(format!("{a}-1.png.enc"))).unwrap();
        assert_eq!(teslim_sifre::bayt_coz(&d, a.as_bytes(), &enc).unwrap(), png);
        assert!(d.join("imzalar").join(format!("{a}-1.png")).exists());
        let liste: serde_json::Value = serde_json::from_slice(
            &std::fs::read(d.join("imzalar").join(SILINECEK_LISTESI)).unwrap(),
        )
        .unwrap();
        assert_eq!(liste["dosyalar"][0]["dosya"], format!("imzalar/{a}-1.png"));
        // ikinci çalıştırma: hiçbir şey yazılmaz, yeni yedek yok
        let sonra = std::fs::read(d.join("fisler.jsonl")).unwrap();
        let r2 = calistir(&d, 1_795_000_100, false).unwrap();
        assert_eq!(
            (
                r2.defter_yeniden_yazilan,
                r2.imza_sifrelenen,
                r2.yedek.clone()
            ),
            (0, 0, None)
        );
        assert_eq!(r2.duz_imza_bekleyen.len(), 1);
        assert_eq!(std::fs::read(d.join("fisler.jsonl")).unwrap(), sonra);
        assert_eq!(
            std::fs::read_dir(d.join("yedek")).unwrap().count(),
            1,
            "tek yedek"
        );
        assert_eq!(durum(&d).unwrap()["gecis_gerekli"], false);
        // yönetici şifresiz PNG'yi sildikten sonra (Geri Dönüşüm Kutusu) imza şifreli kopyadan okunur
        std::fs::remove_file(d.join("imzalar").join(format!("{a}-1.png"))).unwrap();
        let r3 = calistir(&d, 1_795_000_200, false).unwrap();
        assert!(r3.duz_imza_bekleyen.is_empty());
        let liste: serde_json::Value = serde_json::from_slice(
            &std::fs::read(d.join("imzalar").join(SILINECEK_LISTESI)).unwrap(),
        )
        .unwrap();
        assert!(liste["dosyalar"].as_array().unwrap().is_empty());
        assert!(icerir(&d, &["Örnekkişi", "ESKI-DENEME-IMZA"]).is_empty());
    }

    /// Şifreli kopyası kurcalanmış imza: dokunulmaz, sorunlu olarak bildirilir; kayıt yolu ".png" kalır, ad yine
    /// şifrelenir. Anahtar yokken şifreli dosya varsa (anahtar kaybı) yeni anahtar üretilmez, defter değişmez.
    #[test]
    fn uyusmayan_sifreli_kopya_ve_anahtar_kaybi() {
        let (d, a, _, _) = eski_kurulum();
        let bozuk = d.join("imzalar").join(format!("{a}-1.png.enc"));
        std::fs::write(&bozuk, b"TAAENC1\nbozuk-bozuk-bozuk-bozuk-bozuk-bozuk").unwrap();
        let once = std::fs::read(d.join("fisler.jsonl")).unwrap();
        assert!(calistir(&d, 1_795_000_000, false).is_err());
        assert_eq!(std::fs::read(d.join("fisler.jsonl")).unwrap(), once);
        assert!(!d.join(teslim_sifre::ANAHTAR_DOSYASI).exists());
        // anahtar varken: sorunlu imza bildirilir, imza yolu .png kalır, ad şifrelenir
        std::fs::remove_file(&bozuk).unwrap();
        teslim_sifre::sifrele(&d, "F-0", "", &teslim_sifre::TeslimDuz::default()).unwrap();
        std::fs::write(&bozuk, b"TAAENC1\nbozuk-bozuk-bozuk-bozuk-bozuk-bozuk").unwrap();
        let r = calistir(&d, 1_795_000_000, false).unwrap();
        assert_eq!(r.sorunlu_imza, vec![format!("imzalar/{a}-1.png")]);
        assert_eq!(r.defter_sifrelenen, 2);
        let df = Defter {
            yol: &d.join("fisler.jsonl"),
        };
        let t = df.bul(&a).unwrap().unwrap().teslim_kaydi.unwrap();
        assert_eq!(t.imza_dosyasi, format!("imzalar/{a}-1.png"));
        assert!(t.teslim_alan.is_empty() && t.sifreli.is_some());
    }

    /// Zincir (TT1) başlatılmışsa yönetici onayı olmadan defter yeniden yazılmaz; onayla yazılır.
    #[test]
    fn zincir_varken_onay_ister() {
        let (d, _, _, _) = eski_kurulum();
        let df = Defter {
            yol: &d.join("fisler.jsonl"),
        };
        df.zincir_baslat("2026-11-26").unwrap();
        let once = std::fs::read(d.join("fisler.jsonl")).unwrap();
        let r = calistir(&d, 1_795_000_000, false).unwrap();
        assert!(r.zincir_onayi_gerekli);
        assert_eq!(r.defter_yeniden_yazilan, 0);
        assert_eq!(std::fs::read(d.join("fisler.jsonl")).unwrap(), once);
        let r = calistir(&d, 1_795_000_100, true).unwrap();
        assert!(!r.zincir_onayi_gerekli);
        assert_eq!(r.defter_sifrelenen, 2);
    }

    /// Okuma ile yeniden adlandırma arasında deftere satır eklenirse yeniden yazım vazgeçer (satır kaybolmaz).
    #[test]
    fn gecis_sirasinda_eklenen_satir_kaybolmaz() {
        let (d, _, _, _) = eski_kurulum();
        let yol = d.join("fisler.jsonl");
        let df = Defter { yol: &yol };
        let e = df.teslim_kayitlarini_yeniden_yaz(
            |_, k| {
                let mut y = k.clone();
                y.kim = "DENEME".into();
                Ok(Some(y))
            },
            |_| {
                // yedek alınırken başka istek satır ekler
                taa_dosya::satir_ekle(&yol, r#"{"tur":"bos-deneme"}"#).map_err(|e| e.to_string())
            },
        );
        assert!(e.unwrap_err().contains("değişti"));
        let son = std::fs::read_to_string(&yol).unwrap();
        assert!(son.contains("bos-deneme") && !son.contains("DENEME\""));
    }

    #[test]
    fn uc_yonetici_ister() {
        let (d, _, _, _) = eski_kurulum();
        let mut u = Uygulama::yeni(d, None, "http://127.0.0.1:8787".into());
        u.yonetici_kilidi = true;
        let ist = |metot: &str| Istek {
            metot: metot.into(),
            yol: "/api/yonetim/teslim-gecis".into(),
            yerel: true,
            masaustu: true,
            ..Default::default()
        };
        assert_eq!(u.isle(&ist("POST")).kod, 401);
        u.yonetici_kilidi = false;
        let y = u.isle(&ist("GET"));
        assert_eq!(y.kod, 200);
        // geçiş öncesi bile eski düz metin ad listelerde/ayrıntıda açık gitmez (maskeli)
        let mut l = ist("GET");
        l.yol = "/api/fisler".into();
        let y = u.isle(&l);
        assert_eq!(y.kod, 200);
        let m = String::from_utf8_lossy(&y.govde).to_string();
        assert!(
            !m.contains("Örnekkişi") && m.contains("E*** Ö*** B***"),
            "{m}"
        );
        // Çalışırken uç defteri YENİDEN YAZMAZ (eklenen satır kaybolmasın); yalnız imzaları şifreler ve
        // "yeniden başlatın" der.
        let once = std::fs::read(u.dizin.join("fisler.jsonl")).unwrap();
        let y = u.isle(&ist("POST"));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j: serde_json::Value = serde_json::from_slice(&y.govde).unwrap();
        assert_eq!(j["rapor"]["defter_sifrelenen"], 0);
        assert_eq!(j["rapor"]["imza_sifrelenen"], 1);
        assert_eq!(j["yeniden_baslat"], true);
        assert_eq!(std::fs::read(u.dizin.join("fisler.jsonl")).unwrap(), once);
        assert!(!String::from_utf8_lossy(&y.govde).contains("Örnekkişi"));
        let mut uzak = ist("GET");
        uzak.yerel = false;
        assert_eq!(u.isle(&uzak).kod, 403);
    }
}
