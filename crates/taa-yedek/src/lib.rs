//! taa-yedek: seçilen klasörleri zaman damgalı yedek klasörüne kopyalar, SHA256 bildirimi (manifest.json) yazar,
//! doğrular ve geri yükler. Kaynağı değiştirmez, hiçbir dosyayı silmez veya üzerine yazmaz.
//! Özellik anahtarı: "yedek" (varsayılan açık). Zamanlama programın işidir (bu crate tek seferlik çalışır).
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

pub const OZELLIK: &str = "yedek";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Kayit {
    /// Yedek klasörüne göre yol ("/" ayraçlı).
    pub yol: String,
    pub boyut: u64,
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Manifest {
    pub kayitlar: Vec<Kayit>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Sorun {
    Eksik(String),
    Bozuk(String),
}

fn kapali() -> Result<(), String> {
    if taa_ayar::ozellik_acik(OZELLIK, true) {
        Ok(())
    } else {
        Err(taa_ayar::kapali_mesaji(OZELLIK))
    }
}

pub fn sha256_dosya(yol: &Path) -> std::io::Result<String> {
    let mut f = fs::File::open(yol)?;
    let mut h = Sha256::new();
    let mut tampon = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut tampon)?;
        if n == 0 {
            break;
        }
        h.update(&tampon[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{b:02x}")).collect())
}

fn gez(dizin: &Path, cikti: &mut Vec<PathBuf>) -> std::io::Result<()> {
    let mut ogeler: Vec<_> = fs::read_dir(dizin)?.flatten().collect();
    ogeler.sort_by_key(|o| o.file_name());
    for o in ogeler {
        let t = o.file_type()?;
        if t.is_symlink() {
            continue; // bağlantılar izlenmez (döngü ve dışarı çıkma riski)
        }
        if t.is_dir() {
            gez(&o.path(), cikti)?;
        } else if t.is_file() {
            cikti.push(o.path());
        }
    }
    Ok(())
}

/// Kaynakları `hedef/<damga>/k<i>_<ad>/...` altına yedekler; yedek klasörünü döndürür.
pub fn yedekle(kaynaklar: &[PathBuf], hedef: &Path, damga: &str) -> Result<PathBuf, String> {
    kapali()?;
    let yedek = hedef.join(damga);
    if yedek.exists() {
        return Err(format!("yedek klasörü zaten var: {}", yedek.display()));
    }
    let mut manifest = Manifest::default();
    for (i, kaynak) in kaynaklar.iter().enumerate() {
        if !kaynak.is_dir() {
            return Err(format!("kaynak klasör değil: {}", kaynak.display()));
        }
        // Hedef kaynağın içindeyse sonsuz kopyalamayı önle.
        if let (Ok(h), Ok(k)) = (std::path::absolute(hedef), std::path::absolute(kaynak)) {
            if h.starts_with(&k) {
                return Err("yedek hedefi kaynağın içinde olamaz".into());
            }
        }
        let ad = kaynak
            .file_name()
            .map(|a| a.to_string_lossy().to_string())
            .unwrap_or_default();
        let ust = format!("k{i}_{ad}");
        let mut dosyalar = Vec::new();
        gez(kaynak, &mut dosyalar).map_err(|e| e.to_string())?;
        for d in dosyalar {
            let goreli = d.strip_prefix(kaynak).map_err(|e| e.to_string())?;
            let ham = goreli
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect::<Vec<_>>()
                .join("/");
            let hedef_yol = yedek.join(&ust).join(goreli);
            fs::create_dir_all(hedef_yol.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::copy(&d, &hedef_yol).map_err(|e| format!("{}: {e}", d.display()))?;
            manifest.kayitlar.push(Kayit {
                yol: format!("{ust}/{ham}"),
                boyut: fs::metadata(&hedef_yol).map_err(|e| e.to_string())?.len(),
                sha256: sha256_dosya(&hedef_yol).map_err(|e| e.to_string())?,
            });
        }
    }
    fs::create_dir_all(&yedek).map_err(|e| e.to_string())?;
    let j = serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?;
    taa_dosya::uzerine_yaz_atomik(&yedek.join("manifest.json"), &j).map_err(|e| e.to_string())?;
    Ok(yedek)
}

fn manifest_oku(yedek: &Path) -> Result<Manifest, String> {
    let b =
        fs::read(yedek.join("manifest.json")).map_err(|e| format!("manifest okunamadı: {e}"))?;
    serde_json::from_slice(&b).map_err(|e| e.to_string())
}

/// Yedeği bildirime göre doğrular; sorun listesi boşsa sağlamdır.
pub fn dogrula(yedek: &Path) -> Result<Vec<Sorun>, String> {
    kapali()?;
    let m = manifest_oku(yedek)?;
    let mut sorunlar = Vec::new();
    for k in &m.kayitlar {
        let yol = yedek.join(&k.yol);
        match sha256_dosya(&yol) {
            Err(_) => sorunlar.push(Sorun::Eksik(k.yol.clone())),
            Ok(h) if h != k.sha256 => sorunlar.push(Sorun::Bozuk(k.yol.clone())),
            Ok(_) => {}
        }
    }
    Ok(sorunlar)
}

/// Yedeği `hedef` altına geri yükler. Var olan dosyaya dokunmaz; atlananları döndürür.
/// Önce doğrulama yapılır; bozuk yedek geri yüklenmez.
pub fn geri_yukle(yedek: &Path, hedef: &Path) -> Result<Vec<String>, String> {
    kapali()?;
    let sorunlar = dogrula(yedek)?;
    if !sorunlar.is_empty() {
        return Err(format!(
            "yedek doğrulanamadı ({} sorun); geri yükleme yapılmadı",
            sorunlar.len()
        ));
    }
    let m = manifest_oku(yedek)?;
    let mut atlanan = Vec::new();
    for k in &m.kayitlar {
        if k.yol.split('/').any(|p| p == ".." || p.is_empty()) {
            return Err(format!("güvensiz yol: {}", k.yol));
        }
        let h = hedef.join(&k.yol);
        if h.exists() {
            atlanan.push(k.yol.clone());
            continue;
        }
        fs::create_dir_all(h.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::copy(yedek.join(&k.yol), &h).map_err(|e| e.to_string())?;
    }
    Ok(atlanan)
}

#[cfg(test)]
mod testler {
    use super::*;

    fn gecici(ad: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("taa-yedek-{ad}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn tam_dongu() {
        let t = gecici("dongu");
        let kaynak = t.join("veri");
        fs::create_dir_all(kaynak.join("alt")).unwrap();
        fs::write(kaynak.join("a.txt"), "merhaba").unwrap();
        fs::write(kaynak.join("alt/b.txt"), "dünya").unwrap();
        let yedek = yedekle(&[kaynak], &t.join("yedekler"), "2026-11-04").unwrap();
        assert!(dogrula(&yedek).unwrap().is_empty());
        // geri yükle: ilk seferde yazılır, ikincide atlanır
        let hedef = t.join("geri");
        assert!(geri_yukle(&yedek, &hedef).unwrap().is_empty());
        assert_eq!(
            fs::read_to_string(hedef.join("k0_veri/alt/b.txt")).unwrap(),
            "dünya"
        );
        assert_eq!(geri_yukle(&yedek, &hedef).unwrap().len(), 2);
        let _ = fs::remove_dir_all(&t);
    }

    #[test]
    fn bozulmayi_yakalar() {
        let t = gecici("bozuk");
        let kaynak = t.join("v");
        fs::create_dir_all(&kaynak).unwrap();
        fs::write(kaynak.join("a.txt"), "x").unwrap();
        let yedek = yedekle(&[kaynak], &t.join("y"), "d1").unwrap();
        fs::write(yedek.join("k0_v/a.txt"), "değişti").unwrap();
        assert_eq!(
            dogrula(&yedek).unwrap(),
            vec![Sorun::Bozuk("k0_v/a.txt".into())]
        );
        assert!(geri_yukle(&yedek, &t.join("g")).is_err());
        let _ = fs::remove_dir_all(&t);
    }

    #[test]
    fn hedef_icerde_olamaz() {
        let t = gecici("ic");
        let kaynak = t.join("v");
        fs::create_dir_all(&kaynak).unwrap();
        assert!(yedekle(std::slice::from_ref(&kaynak), &kaynak.join("y"), "d").is_err());
        let _ = fs::remove_dir_all(&t);
    }

    #[test]
    fn ayni_damga_tekrar_edilmez() {
        let t = gecici("damga");
        let kaynak = t.join("v");
        fs::create_dir_all(&kaynak).unwrap();
        yedekle(std::slice::from_ref(&kaynak), &t.join("y"), "d").unwrap();
        assert!(yedekle(&[kaynak], &t.join("y"), "d").is_err());
        let _ = fs::remove_dir_all(&t);
    }
}
