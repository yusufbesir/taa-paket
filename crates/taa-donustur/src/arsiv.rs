//! TaaDön arşiv işlemleri: ZIP listele/aç/oluştur, GZIP sıkıştır/aç.
//! Sahip: Yusuf (TaaDön projesi). yazan: [Claude#taadon-rust-asama1 2026-11-04], [Claude#taadon-rust-asama2 2026-11-04] (ad: taa-donustur).
//! Değişiklik: Yusuf'un kuralları çerçevesinde. Kurallar src/motorlar/arsiv.js ile BİREBİR aynıdır
//! (guvenliYol, sınırlar, ZIP64 reddi, SAKLA listesi, acYazma "(n)" eki); biri değişirse öteki de değişmeli.
//! Bu modül hiçbir var olan dosyanın üzerine yazmaz; yalnız kendi yarım bıraktığı yeni çıktı dosyasını kaldırır.

use crate::{node_extname, uzanti_al, Hata, Ilerleme, Sonuc};
use serde::Serialize;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub const VARSAYILAN_MAKS_BOYUT: u64 = 2 * 1024 * 1024 * 1024; // 2 GiB
pub const VARSAYILAN_MAKS_DOSYA: u64 = 65535;
pub const ZIP_MAKS_GIRDI: usize = 65535;
pub const ZIP_MAKS_BAYT: u64 = 0xFFFF_FFFF;
const TAMPON: usize = 256 * 1024;

/// Zaten sıkıştırılmış biçimler: sıkıştırmadan sakla (arsiv.js SAKLA ile aynı).
pub const SAKLA: &[&str] = &[
    "zip", "gz", "jpg", "jpeg", "png", "webp", "avif", "gif", "mp3", "mp4", "mkv", "webm", "m4a",
    "aac", "ogg", "opus", "flac", "mov", "avi", "docx", "xlsx", "pptx", "7z", "rar", "bz2", "xz",
];

// ---------------------------------------------------------------------------
// ZIP yol güvenliği (zip-slip): arsiv.js guvenliYol ile aynı kural
// ---------------------------------------------------------------------------

fn ayrilmis_mi(t: &str) -> bool {
    // /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\..*)?$/i
    let bas = t.split('.').next().unwrap_or("").to_ascii_lowercase();
    match bas.as_str() {
        "con" | "prn" | "aux" | "nul" => true,
        s if s.len() == 4 && (s.starts_with("com") || s.starts_with("lpt")) => {
            matches!(s.as_bytes()[3], b'1'..=b'9')
        }
        _ => false,
    }
}

fn surucu_harfli(n: &str) -> bool {
    let b = n.as_bytes();
    b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

/// ZIP girdi adını güvenli göreli parçalara çevirir.
/// Ok(None): yazılacak bir şey yok (ör. "./"). Err: arşiv reddedilmeli (Node ile aynı Türkçe metin).
pub fn guvenli_yol(ad: &str) -> Result<Option<Vec<String>>, String> {
    let n = ad.replace('\\', "/");
    if n.is_empty() || n.contains('\u{0}') {
        return Err("ZIP içinde geçersiz dosya adı var; arşiv reddedildi.".into());
    }
    if n.starts_with('/') || surucu_harfli(&n) {
        return Err(format!(
            "ZIP güvenlik ihlali: mutlak yol reddedildi ({ad})."
        ));
    }
    let mut parcalar = Vec::new();
    for p in n.split('/') {
        if p.is_empty() || p == "." {
            continue;
        }
        if p == ".." {
            return Err(format!(
                "ZIP güvenlik ihlali: üst klasöre çıkan yol reddedildi ({ad})."
            ));
        }
        let mut t: String = p
            .chars()
            .map(|c| {
                if matches!(c, ':' | '*' | '?' | '"' | '<' | '>' | '|') || (c as u32) <= 0x1f {
                    '_'
                } else {
                    c
                }
            })
            .collect();
        let kirp = t.trim_end_matches(['.', ' ']).len();
        t.truncate(kirp);
        if t.is_empty() {
            t.push('_');
        }
        if ayrilmis_mi(&t) {
            t.insert(0, '_');
        }
        parcalar.push(t);
    }
    if parcalar.is_empty() {
        return Ok(None);
    }
    Ok(Some(parcalar))
}

/// Parçaları kök altında birleştirir ve kök dışına çıkmadığını doğrular.
fn kok_altinda(kok: &Path, parcalar: &[String], ad: &str) -> Sonuc<PathBuf> {
    let mut h = kok.to_path_buf();
    for p in parcalar {
        h.push(p);
    }
    if !h.starts_with(kok) || h == kok {
        return Err(Hata::yeni(format!(
            "ZIP güvenlik ihlali: çıktı klasörü dışına çıkan yol reddedildi ({ad})."
        )));
    }
    Ok(h)
}

fn dizin_girdisi_mi(ad: &str) -> bool {
    ad.ends_with('/') || ad.ends_with('\\')
}

/// Yinelenen girdi adı hatası (rust.js bu metni JS yoluna geri düşme işareti sayar; değiştirirken oradaki deseni de güncelleyin).
pub const YINELENEN_AD_HATASI: &str =
    "ZIP yinelenen girdi adı içeriyor; arşiv Rust çekirdeğinde açılmadı.";

fn zip_ac_hatasi(e: zip::result::ZipError) -> Hata {
    let m = e.to_string();
    if m.to_lowercase().contains("password") {
        return Hata::yeni("Şifreli ZIP girdileri desteklenmiyor.");
    }
    Hata::yeni(format!("ZIP bozuk ya da okunamadı ({m})."))
}

fn arsivi_ac(zip_yolu: &Path) -> Sonuc<(zip::ZipArchive<BufReader<File>>, u64)> {
    let f = File::open(zip_yolu).map_err(|e| Hata::yeni(format!("ZIP dosyası açılamadı: {e}")))?;
    let boyut = f.metadata().map(|m| m.len()).unwrap_or(0);
    let a = zip::ZipArchive::new(BufReader::with_capacity(TAMPON, f)).map_err(zip_ac_hatasi)?;
    // Merkezi dizinde aynı ad birden çok kez: zip crate bunları sessizce tekilleştirir (girdi kaybı). Reddedilir;
    // Node köprüsü (src/motorlar/rust.js) bu metni tanıyıp JS yoluna (fflate, "(n)" ekiyle açar) geri düşer.
    // yazan: [Claude#taadon-rust-asama2 2026-11-04]
    if a.has_duplicate_names() {
        return Err(Hata::yeni(YINELENEN_AD_HATASI));
    }
    Ok((a, boyut))
}

// ---------------------------------------------------------------------------
// zip-listele
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ListeGirdisi {
    pub ad: String,
    pub boyut: u64,
    pub sikistirilmis: u64,
    pub dizin: bool,
    pub guvenli: bool,
    pub neden: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Liste {
    pub girdiler: Vec<ListeGirdisi>,
    pub girdi_sayisi: usize,
    pub toplam_boyut: u64,
    pub toplam_sikistirilmis: u64,
    pub guvenli: bool,
}

pub fn zip_listele(zip_yolu: &Path) -> Sonuc<Liste> {
    let (mut a, _) = arsivi_ac(zip_yolu)?;
    if a.len() > ZIP_MAKS_GIRDI {
        return Err(Hata::yeni(format!(
            "ZIP çok fazla girdi içeriyor ({} girdi, sınır {ZIP_MAKS_GIRDI}).",
            a.len()
        )));
    }
    let mut girdiler = Vec::with_capacity(a.len());
    let (mut toplam, mut sik) = (0u64, 0u64);
    for i in 0..a.len() {
        let f = a.by_index_raw(i).map_err(zip_ac_hatasi)?;
        let ad = f
            .name()
            .map(|s| s.into_owned())
            .unwrap_or_else(|_| String::from_utf8_lossy(f.name_raw()).into_owned());
        let (boyut, s) = (f.size(), f.compressed_size());
        if boyut >= ZIP_MAKS_BAYT || s >= ZIP_MAKS_BAYT {
            return Err(Hata::yeni(format!(
                "ZIP64 desteklenmiyor: bir girdi 4 GiB ve üzeri ({ad})."
            )));
        }
        toplam += boyut;
        sik += s;
        let (guvenli, neden) = match guvenli_yol(&ad) {
            Ok(_) => (true, None),
            Err(m) => (false, Some(m)),
        };
        girdiler.push(ListeGirdisi {
            dizin: dizin_girdisi_mi(&ad),
            ad,
            boyut,
            sikistirilmis: s,
            guvenli,
            neden,
        });
    }
    if toplam > VARSAYILAN_MAKS_BOYUT {
        return Err(Hata::yeni(format!(
            "ZIP açılmış boyutu sınırı aşıyor (sınır {VARSAYILAN_MAKS_BOYUT} bayt)."
        )));
    }
    let hepsi = girdiler.iter().all(|g| g.guvenli);
    Ok(Liste {
        girdi_sayisi: girdiler.len(),
        girdiler,
        toplam_boyut: toplam,
        toplam_sikistirilmis: sik,
        guvenli: hepsi,
    })
}

// ---------------------------------------------------------------------------
// zip-ac
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AcmaSonucu {
    pub klasor: String,
    pub dosya_sayisi: u64,
    pub toplam_bayt: u64,
}

/// arsiv.js acYazma: aynı ad varsa "ad (n).uz" ile benzersiz yeni dosya (üzerine yazma yok).
fn ac_yazma(yol: &Path) -> Sonuc<(File, PathBuf)> {
    let ad = yol
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let uz = node_extname(&ad).to_string();
    let govde = ad[..ad.len() - uz.len()].to_string();
    for n in 0..10000 {
        let aday = if n == 0 {
            yol.to_path_buf()
        } else {
            yol.with_file_name(format!("{govde} ({n}){uz}"))
        };
        match OpenOptions::new().write(true).create_new(true).open(&aday) {
            Ok(f) => return Ok((f, aday)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                return Err(Hata::yeni(format!(
                    "Dosya yazılamadı ({}): {e}",
                    aday.display()
                )))
            }
        }
    }
    Err(Hata::yeni("Dosya adı çakışması çözülemedi."))
}

fn boyut_asimi(maks: u64) -> Hata {
    Hata::yeni(format!(
        "ZIP açılmış boyutu sınırı aşıyor (sınır {maks} bayt)."
    ))
}

/// Girdi verisini okur, toplam sayaca ekler ve sınırı uygular; `yaz` varsa yazar.
fn akit(
    okur: &mut dyn Read,
    mut yaz: Option<&mut dyn Write>,
    toplam: &mut u64,
    maks: u64,
    tampon: &mut [u8],
) -> Sonuc<()> {
    loop {
        let n = match okur.read(tampon) {
            Ok(0) => return Ok(()),
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(Hata::yeni(format!("ZIP verisi çözülemedi: {e}"))),
        };
        *toplam += n as u64;
        if *toplam > maks {
            return Err(boyut_asimi(maks));
        }
        if let Some(w) = yaz.as_mut() {
            w.write_all(&tampon[..n])
                .map_err(|e| Hata::yeni(format!("Dosya yazılamadı: {e}")))?;
        }
    }
}

fn cok_dosya(maks: u64) -> Hata {
    Hata::yeni(format!("ZIP çok fazla dosya içeriyor (sınır {maks})."))
}

/// Güvenli çıkarma. Hedef klasör yoksa oluşturulur; gerçek yolu (canonicalize) dışına çıkılmaz.
/// Hata durumunda yarım çıkarılan içerik hedefte kalır (silinmez): çağıran taraf kendi geçici klasörünü kullanır.
pub fn zip_ac(
    zip_yolu: &Path,
    hedef: &Path,
    maks_bayt: u64,
    maks_dosya: u64,
    il: &mut Ilerleme,
) -> Sonuc<AcmaSonucu> {
    fs::create_dir_all(hedef)
        .map_err(|e| Hata::yeni(format!("Hedef klasör oluşturulamadı: {e}")))?;
    let kok =
        fs::canonicalize(hedef).map_err(|e| Hata::yeni(format!("Hedef klasör çözülemedi: {e}")))?;
    let (mut a, zip_boyut) = arsivi_ac(zip_yolu)?;
    let mut toplam = 0u64;
    let mut dosya_sayisi = 0u64;
    let mut islenen_sik = 0u64;
    let mut tampon = vec![0u8; TAMPON];
    for i in 0..a.len() {
        let mut f = a.by_index(i).map_err(zip_ac_hatasi)?;
        let ad = f
            .name()
            .map(|s| s.into_owned())
            .map_err(|_| Hata::yeni("ZIP içinde geçersiz dosya adı var; arşiv reddedildi."))?;
        let parcalar = guvenli_yol(&ad).map_err(Hata)?;
        islenen_sik += f.compressed_size();
        match (&parcalar, dizin_girdisi_mi(&ad)) {
            (Some(p), false) => {
                dosya_sayisi += 1;
                if dosya_sayisi > maks_dosya {
                    return Err(cok_dosya(maks_dosya));
                }
                if toplam.saturating_add(f.size()) > maks_bayt {
                    return Err(boyut_asimi(maks_bayt));
                }
                let hedef_yol = kok_altinda(&kok, p, &ad)?;
                let ust = hedef_yol.parent().unwrap_or(&kok).to_path_buf();
                fs::create_dir_all(&ust)
                    .map_err(|e| Hata::yeni(format!("Klasör oluşturulamadı: {e}")))?;
                // bağlantı (junction/symlink) üzerinden kök dışına çıkmayı engelle
                let ust_gercek = fs::canonicalize(&ust)
                    .map_err(|e| Hata::yeni(format!("Klasör çözülemedi: {e}")))?;
                if !ust_gercek.starts_with(&kok) {
                    return Err(Hata::yeni(format!(
                        "ZIP güvenlik ihlali: çıktı klasörü dışına çıkan yol reddedildi ({ad})."
                    )));
                }
                let (dosya, _yol) = ac_yazma(&hedef_yol)?;
                // sembolik bağlantı girdileri de düz dosya olarak (hedef metniyle) yazılır
                let mut w = BufWriter::with_capacity(TAMPON, dosya);
                akit(&mut f, Some(&mut w), &mut toplam, maks_bayt, &mut tampon)?;
                w.flush()
                    .map_err(|e| Hata::yeni(format!("Dosya yazılamadı: {e}")))?;
            }
            _ => {
                if let Some(p) = &parcalar {
                    dosya_sayisi += 1; // klasör girişleri de sayılır
                    if dosya_sayisi > maks_dosya {
                        return Err(cok_dosya(maks_dosya));
                    }
                    let d = kok_altinda(&kok, p, &ad)?;
                    fs::create_dir_all(&d)
                        .map_err(|e| Hata::yeni(format!("Klasör oluşturulamadı: {e}")))?;
                }
                // veri taşıyan dizin/atlanan girdiler: tüket, toplama say, sınırı uygula
                akit(&mut f, None, &mut toplam, maks_bayt, &mut tampon)?;
            }
        }
        il.oran(islenen_sik, zip_boyut);
    }
    Ok(AcmaSonucu {
        klasor: crate::gosterim_yolu(&kok),
        dosya_sayisi,
        toplam_bayt: toplam,
    })
}

// ---------------------------------------------------------------------------
// Yeni çıktı dosyası: var olanın üzerine yazmaz; başarısızlıkta yalnız kendi oluşturduğunu kaldırır
// ---------------------------------------------------------------------------

struct YeniCikti {
    yol: PathBuf,
    tamam: bool,
}

impl YeniCikti {
    fn olustur(yol: &Path) -> Sonuc<(Self, File)> {
        match OpenOptions::new().write(true).create_new(true).open(yol) {
            Ok(f) => Ok((
                YeniCikti {
                    yol: yol.to_path_buf(),
                    tamam: false,
                },
                f,
            )),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(Hata::yeni(format!(
                "Çıktı dosyası zaten var; üzerine yazılmaz: {}",
                yol.display()
            ))),
            Err(e) => Err(Hata::yeni(format!("Çıktı dosyası oluşturulamadı: {e}"))),
        }
    }
}

impl Drop for YeniCikti {
    fn drop(&mut self) {
        if !self.tamam {
            // yalnız bu çalıştırmada create_new ile oluşturulan yarım dosya
            let _ = fs::remove_file(&self.yol);
        }
    }
}

// ---------------------------------------------------------------------------
// zip-olustur
// ---------------------------------------------------------------------------

#[derive(Debug)]
struct Kayit {
    yol: PathBuf,
    ad: String,
    dizin: bool,
    boyut: u64,
    mtime: Option<SystemTime>,
}

/// arsiv.js girdileriTopla benzersiz(): kök adlar büyük/küçük harf duyarsız tekilleştirilir.
fn benzersiz(adlar: &mut std::collections::HashSet<String>, ad: &str) -> String {
    let mut aday = ad.to_string();
    let mut n = 1;
    while adlar.contains(&aday.to_lowercase()) {
        let (kok, uz) = match ad.rfind('.').filter(|&i| i > 0) {
            Some(i) if !ad[i..].contains('/') => (&ad[..i], &ad[i..]),
            _ => (ad, ""),
        };
        aday = format!("{kok} ({n}){uz}");
        n += 1;
    }
    adlar.insert(aday.to_lowercase());
    aday
}

fn gez(yol: &Path, zip_adi: String, kayitlar: &mut Vec<Kayit>, maks: u64) -> Sonuc<()> {
    if kayitlar.len() as u64 >= maks {
        return Err(Hata::yeni(format!("Çok fazla dosya (sınır {maks}).")));
    }
    let md = fs::symlink_metadata(yol)
        .map_err(|e| Hata::yeni(format!("Girdi okunamadı ({}): {e}", yol.display())))?;
    let ft = md.file_type();
    if ft.is_symlink() {
        return Ok(()); // sembolik bağlantılar izlenmez
    }
    if ft.is_dir() {
        let mut cocuklar = Vec::new();
        for g in fs::read_dir(yol).map_err(|e| Hata::yeni(format!("Klasör okunamadı: {e}")))? {
            let g = g.map_err(|e| Hata::yeni(format!("Klasör okunamadı: {e}")))?;
            cocuklar.push(g.file_name().to_string_lossy().into_owned());
        }
        if cocuklar.is_empty() {
            kayitlar.push(Kayit {
                yol: yol.to_path_buf(),
                ad: format!("{zip_adi}/"),
                dizin: true,
                boyut: 0,
                mtime: md.modified().ok(),
            });
        }
        for c in cocuklar {
            gez(&yol.join(&c), format!("{zip_adi}/{c}"), kayitlar, maks)?;
        }
    } else if ft.is_file() {
        kayitlar.push(Kayit {
            yol: yol.to_path_buf(),
            ad: zip_adi,
            dizin: false,
            boyut: md.len(),
            mtime: md.modified().ok(),
        });
    }
    Ok(())
}

/// arsiv.js zip64Gerekli: sınır aşılıyorsa Türkçe neden.
fn zip64_gerekli(kayitlar: &[Kayit]) -> Option<String> {
    if kayitlar.len() > ZIP_MAKS_GIRDI {
        return Some(format!(
            "{} girdi var (en çok {ZIP_MAKS_GIRDI})",
            kayitlar.len()
        ));
    }
    let mut toplam = 0u64;
    for k in kayitlar {
        let b = if k.dizin { 0 } else { k.boyut };
        if b >= ZIP_MAKS_BAYT {
            return Some(format!("bir dosya 4 GiB ve üzeri ({})", k.ad));
        }
        toplam += b;
    }
    if toplam >= ZIP_MAKS_BAYT {
        return Some("toplam boyut 4 GiB ve üzeri".into());
    }
    None
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OlusturmaSonucu {
    pub cikti: String,
    pub girdi_sayisi: usize,
    pub toplam_bayt: u64,
}

fn yaz_hatasi(e: impl std::fmt::Display) -> Hata {
    Hata::yeni(format!("ZIP yazılamadı: {e}"))
}

pub fn zip_olustur(
    cikti: &Path,
    girdiler: &[PathBuf],
    maks_dosya: u64,
    il: &mut Ilerleme,
) -> Sonuc<OlusturmaSonucu> {
    if girdiler.is_empty() {
        return Err(Hata::yeni("ZIP için en az bir girdi gerekli."));
    }
    let mut kayitlar = Vec::new();
    let mut adlar = std::collections::HashSet::new();
    for g in girdiler {
        let tam = std::path::absolute(g).unwrap_or_else(|_| g.clone());
        let taban = tam
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let kok_ad = benzersiz(&mut adlar, &taban);
        gez(&tam, kok_ad, &mut kayitlar, maks_dosya)?;
    }
    if let Some(neden) = zip64_gerekli(&kayitlar) {
        return Err(Hata::yeni(format!(
            "ZIP64 desteklenmiyor: {neden}. Daha az/küçük dosyayla ya da parçalara bölerek deneyin."
        )));
    }
    let toplam_bayt: u64 = kayitlar.iter().filter(|k| !k.dizin).map(|k| k.boyut).sum();
    let (mut koruma, dosya) = YeniCikti::olustur(cikti)?;
    let mut zw = zip::ZipWriter::new(BufWriter::with_capacity(TAMPON, dosya));
    let mut islenen = 0u64;
    let mut tampon = vec![0u8; TAMPON];
    for k in &kayitlar {
        let sakla = k.dizin || SAKLA.contains(&uzanti_al(Path::new(&k.ad)).as_str());
        let sec = zip::write::SimpleFileOptions::default()
            .compression_method(if sakla {
                zip::CompressionMethod::Stored
            } else {
                zip::CompressionMethod::Deflated
            })
            .compression_level(if sakla { None } else { Some(6) })
            .last_modified_time(zip_zamani(k.mtime))
            .large_file(false);
        if k.dizin {
            zw.add_directory(k.ad.clone(), sec).map_err(yaz_hatasi)?;
            continue;
        }
        zw.start_file(k.ad.clone(), sec).map_err(yaz_hatasi)?;
        let mut f = File::open(&k.yol)
            .map_err(|e| Hata::yeni(format!("Girdi açılamadı ({}): {e}", k.yol.display())))?;
        loop {
            let n = match f.read(&mut tampon) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(Hata::yeni(format!("Girdi okunamadı: {e}"))),
            };
            zw.write_all(&tampon[..n]).map_err(yaz_hatasi)?;
            islenen += n as u64;
            il.oran(islenen, toplam_bayt);
        }
    }
    let mut w = zw.finish().map_err(yaz_hatasi)?;
    w.flush().map_err(yaz_hatasi)?;
    drop(w);
    koruma.tamam = true;
    Ok(OlusturmaSonucu {
        cikti: crate::gosterim_yolu(cikti),
        girdi_sayisi: kayitlar.len(),
        toplam_bayt,
    })
}

/// Değiştirilme zamanını yerel saatle ZIP (DOS) zamanına çevirir; 1980 öncesi 1980-01-01.
fn zip_zamani(t: Option<SystemTime>) -> zip::DateTime {
    let varsayilan = zip::DateTime::default();
    let Some(t) = t else { return varsayilan };
    let Some((y, mo, d, h, mi, s)) = yerel_saat::coz(t) else {
        return varsayilan;
    };
    if y < 1980 {
        return varsayilan;
    }
    zip::DateTime::from_date_and_time(y.min(2107) as u16, mo, d, h, mi, s).unwrap_or(varsayilan)
}

#[cfg(windows)]
mod yerel_saat {
    use std::time::{SystemTime, UNIX_EPOCH};
    #[repr(C)]
    #[derive(Default, Clone, Copy)]
    #[allow(non_snake_case, clippy::upper_case_acronyms)]
    struct SYSTEMTIME {
        wYear: u16,
        wMonth: u16,
        wDayOfWeek: u16,
        wDay: u16,
        wHour: u16,
        wMinute: u16,
        wSecond: u16,
        wMilliseconds: u16,
    }
    #[repr(C)]
    #[allow(clippy::upper_case_acronyms)]
    struct FILETIME {
        dusuk: u32,
        yuksek: u32,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn FileTimeToSystemTime(ft: *const FILETIME, st: *mut SYSTEMTIME) -> i32;
        fn SystemTimeToTzSpecificLocalTime(
            tz: *const core::ffi::c_void,
            utc: *const SYSTEMTIME,
            yerel: *mut SYSTEMTIME,
        ) -> i32;
    }
    pub fn coz(t: SystemTime) -> Option<(i32, u8, u8, u8, u8, u8)> {
        let d = t.duration_since(UNIX_EPOCH).ok()?;
        let v = d.as_secs().checked_mul(10_000_000)?
            + u64::from(d.subsec_nanos() / 100)
            + 116_444_736_000_000_000;
        let ft = FILETIME {
            dusuk: v as u32,
            yuksek: (v >> 32) as u32,
        };
        let mut utc = SYSTEMTIME::default();
        let mut yerel = SYSTEMTIME::default();
        // SAFETY: işaretçiler geçerli yığın değişkenlerine işaret eder; işlevler yalnız bunları okur/yazar.
        unsafe {
            if FileTimeToSystemTime(&ft, &mut utc) == 0 {
                return None;
            }
            if SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc, &mut yerel) == 0 {
                yerel = utc;
            }
        }
        Some((
            yerel.wYear as i32,
            yerel.wMonth as u8,
            yerel.wDay as u8,
            yerel.wHour as u8,
            yerel.wMinute as u8,
            yerel.wSecond as u8,
        ))
    }
}

#[cfg(not(windows))]
mod yerel_saat {
    use std::time::{SystemTime, UNIX_EPOCH};
    /// Windows dışı: UTC (yerel saat dilimi kitaplıksız çözülemez).
    pub fn coz(t: SystemTime) -> Option<(i32, u8, u8, u8, u8, u8)> {
        let s = t.duration_since(UNIX_EPOCH).ok()?.as_secs() as i64;
        let (gun, kalan) = (s.div_euclid(86400), s.rem_euclid(86400));
        let z = gun + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let d = (doy - (153 * mp + 2) / 5 + 1) as u8;
        let m = if mp < 10 { mp + 3 } else { mp - 9 } as u8;
        let y = (yoe + era * 400 + i64::from(m <= 2)) as i32;
        Some((
            y,
            m,
            d,
            (kalan / 3600) as u8,
            (kalan % 3600 / 60) as u8,
            (kalan % 60) as u8,
        ))
    }
}

// ---------------------------------------------------------------------------
// GZIP
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GzipSonucu {
    pub cikti: String,
    pub toplam_bayt: u64,
}

fn oku_tekrarli(f: &mut dyn Read, tampon: &mut [u8], hata: &str) -> Sonuc<usize> {
    loop {
        match f.read(tampon) {
            Ok(n) => return Ok(n),
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(Hata::yeni(format!("{hata}: {e}"))),
        }
    }
}

pub fn gzip(girdi: &Path, cikti: &Path, il: &mut Ilerleme) -> Sonuc<GzipSonucu> {
    let md = fs::metadata(girdi).map_err(|e| Hata::yeni(format!("Girdi okunamadı: {e}")))?;
    if !md.is_file() {
        return Err(Hata::yeni(
            "GZIP yalnız dosyaları sıkıştırır; klasör için ZIP kullanın.",
        ));
    }
    let ad = girdi
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    // fflate ile aynı: başlıktaki ad her karakterin alt baytıdır (NUL olamaz)
    let ad_bayt: Vec<u8> = ad
        .chars()
        .map(|c| match (c as u32 & 0xff) as u8 {
            0 => b'_',
            b => b,
        })
        .collect();
    let mtime = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);
    let mut f = File::open(girdi).map_err(|e| Hata::yeni(format!("Girdi açılamadı: {e}")))?;
    let (mut koruma, dosya) = YeniCikti::olustur(cikti)?;
    let mut gz = flate2::GzBuilder::new()
        .filename(ad_bayt)
        .mtime(mtime)
        .write(
            BufWriter::with_capacity(TAMPON, dosya),
            flate2::Compression::new(6),
        );
    let mut tampon = vec![0u8; TAMPON];
    let mut islenen = 0u64;
    loop {
        let n = oku_tekrarli(&mut f, &mut tampon, "Girdi okunamadı")?;
        if n == 0 {
            break;
        }
        gz.write_all(&tampon[..n])
            .map_err(|e| Hata::yeni(format!("GZIP yazılamadı: {e}")))?;
        islenen += n as u64;
        il.oran(islenen, md.len());
    }
    let mut w = gz
        .finish()
        .map_err(|e| Hata::yeni(format!("GZIP yazılamadı: {e}")))?;
    w.flush()
        .map_err(|e| Hata::yeni(format!("GZIP yazılamadı: {e}")))?;
    drop(w);
    koruma.tamam = true;
    Ok(GzipSonucu {
        cikti: crate::gosterim_yolu(cikti),
        toplam_bayt: islenen,
    })
}

pub fn gunzip(girdi: &Path, cikti: &Path, maks_bayt: u64, il: &mut Ilerleme) -> Sonuc<GzipSonucu> {
    let f = File::open(girdi).map_err(|e| Hata::yeni(format!("Girdi açılamadı: {e}")))?;
    let boyut = f.metadata().map(|m| m.len()).unwrap_or(0);
    if boyut == 0 {
        return Err(Hata::yeni("GZIP dosyası bozuk ya da eksik."));
    }
    let sayac = SayacOkur {
        ic: BufReader::with_capacity(TAMPON, f),
        okunan: 0,
    };
    let mut dec = flate2::read::MultiGzDecoder::new(sayac);
    let (mut koruma, dosya) = YeniCikti::olustur(cikti)?;
    let mut w = BufWriter::with_capacity(TAMPON, dosya);
    let mut tampon = vec![0u8; TAMPON];
    let mut toplam = 0u64;
    loop {
        let n = match dec.read(&mut tampon) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return Err(Hata::yeni("GZIP dosyası bozuk ya da eksik.")),
        };
        toplam += n as u64;
        if toplam > maks_bayt {
            return Err(Hata::yeni(format!(
                "GZIP açılmış boyutu sınırı aşıyor (sınır {maks_bayt} bayt)."
            )));
        }
        w.write_all(&tampon[..n])
            .map_err(|e| Hata::yeni(format!("Dosya yazılamadı: {e}")))?;
        il.oran(dec.get_ref().okunan, boyut);
    }
    w.flush()
        .map_err(|e| Hata::yeni(format!("Dosya yazılamadı: {e}")))?;
    drop(w);
    koruma.tamam = true;
    Ok(GzipSonucu {
        cikti: crate::gosterim_yolu(cikti),
        toplam_bayt: toplam,
    })
}

struct SayacOkur<R> {
    ic: R,
    okunan: u64,
}

impl<R: Read> Read for SayacOkur<R> {
    fn read(&mut self, b: &mut [u8]) -> io::Result<usize> {
        let n = self.ic.read(b)?;
        self.okunan += n as u64;
        Ok(n)
    }
}
