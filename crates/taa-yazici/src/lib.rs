//! taa-yazici (Yazıcı Sağlık): yazıcı listesini (PowerShell CSV çıktısı) okuyup Türkçe tanı üretir.
//! Çözümleme ve tanı saf işlevdir (her platformda test edilir); `windows_listesi` yalnız Windows'ta çalışır
//! ve YALNIZ OKUR (Get-Printer). Hiçbir yazıcı ayarına/kuyruğa dokunmaz.
//! Özellik anahtarı: "yazici-saglik" (varsayılan açık). Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

pub const OZELLIK: &str = "yazici-saglik";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Yazici {
    pub ad: String,
    pub surucu: String,
    /// Windows PrinterStatus metni (Normal, Offline, PaperOut, Error...).
    pub durum: String,
    pub bekleyen_is: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Seviye {
    Tamam,
    Uyari,
    Hata,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bulgu {
    pub yazici: String,
    pub seviye: Seviye,
    pub mesaj: String,
}

/// `Get-Printer | Select Name,DriverName,PrinterStatus,JobCount | ConvertTo-Csv -NoTypeInformation` çıktısı.
pub fn csv_coz(metin: &str) -> Result<Vec<Yazici>, String> {
    let satirlar = taa_nebim::csv_satirlari(metin.trim_start_matches('\u{feff}'), ',');
    let (bas, govde) = satirlar.split_first().ok_or("çıktı boş")?;
    let i = |ad: &str| {
        bas.iter()
            .position(|b| b.eq_ignore_ascii_case(ad))
            .ok_or(format!("'{ad}' sütunu yok"))
    };
    let (n, d, s, j) = (
        i("Name")?,
        i("DriverName")?,
        i("PrinterStatus")?,
        i("JobCount")?,
    );
    Ok(govde
        .iter()
        .filter(|r| r.len() > n.max(d).max(s).max(j))
        .map(|r| Yazici {
            ad: r[n].clone(),
            surucu: r[d].clone(),
            durum: r[s].clone(),
            bekleyen_is: r[j].trim().parse().unwrap_or(0),
        })
        .collect())
}

pub fn tani(yaziciler: &[Yazici]) -> Vec<Bulgu> {
    let mut c = Vec::new();
    if yaziciler.is_empty() {
        c.push(Bulgu {
            yazici: String::new(),
            seviye: Seviye::Hata,
            mesaj: "Hiç yazıcı bulunamadı".into(),
        });
    }
    for y in yaziciler {
        let mut ekle = |s: Seviye, m: String| {
            c.push(Bulgu {
                yazici: y.ad.clone(),
                seviye: s,
                mesaj: m,
            })
        };
        match y.durum.to_lowercase().as_str() {
            "normal" | "idle" | "0" | "" => {}
            "offline" => ekle(
                Seviye::Hata,
                "Yazıcı çevrimdışı: kablo/güç ve USB bağlantısını kontrol edin".into(),
            ),
            "paperout" => ekle(
                Seviye::Uyari,
                "Kağıt/etiket bitti: ruloyu değiştirin".into(),
            ),
            "papernotjam" | "paperjam" => ekle(Seviye::Hata, "Kağıt sıkışması".into()),
            "error" => ekle(
                Seviye::Hata,
                "Yazıcı hata bildiriyor: sürücü ve kapağı kontrol edin".into(),
            ),
            "tonerlow" => ekle(Seviye::Uyari, "Mürekkep/şerit azalıyor".into()),
            diger => ekle(Seviye::Uyari, format!("Bilinmeyen durum: {diger}")),
        }
        if y.surucu.trim().is_empty() {
            ekle(Seviye::Hata, "Sürücü kurulu görünmüyor".into());
        }
        if y.bekleyen_is >= 10 {
            ekle(
                Seviye::Hata,
                format!(
                    "Kuyrukta {} iş takılı: kuyruğu yönetici temizlemeli",
                    y.bekleyen_is
                ),
            );
        } else if y.bekleyen_is >= 3 {
            ekle(
                Seviye::Uyari,
                format!("Kuyrukta {} iş bekliyor", y.bekleyen_is),
            );
        }
    }
    c
}

pub fn en_kotu(b: &[Bulgu]) -> Seviye {
    b.iter().map(|x| x.seviye).max().unwrap_or(Seviye::Tamam)
}

/// Windows'ta PowerShell ile salt okunur liste alır; başka sistemlerde hata verir.
pub fn windows_listesi() -> Result<Vec<Yazici>, String> {
    if !taa_ayar::ozellik_acik(OZELLIK, true) {
        return Err(taa_ayar::kapali_mesaji(OZELLIK));
    }
    if !cfg!(windows) {
        return Err("yazıcı listesi yalnız Windows'ta alınır".into());
    }
    let o = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", "Get-Printer | Select-Object Name,DriverName,PrinterStatus,JobCount | ConvertTo-Csv -NoTypeInformation"])
        .output()
        .map_err(|e| format!("PowerShell çalışmadı: {e}"))?;
    if !o.status.success() {
        return Err("Get-Printer başarısız".into());
    }
    csv_coz(&String::from_utf8_lossy(&o.stdout))
}

#[cfg(test)]
mod testler {
    use super::*;

    const CIKTI: &str = "\"Name\",\"DriverName\",\"PrinterStatus\",\"JobCount\"\r\n\"Argox OS-214\",\"Argox OS-214 PPLA\",\"Normal\",\"0\"\r\n\"Fis\",\"\",\"Offline\",\"12\"\r\n\"Etiket2\",\"X\",\"PaperOut\",\"4\"\r\n";

    #[test]
    fn cozer() {
        let y = csv_coz(CIKTI).unwrap();
        assert_eq!(y.len(), 3);
        assert_eq!(y[1].bekleyen_is, 12);
    }

    #[test]
    fn taniyi_uretir() {
        let y = csv_coz(CIKTI).unwrap();
        let b = tani(&y);
        assert!(!b.iter().any(|x| x.yazici == "Argox OS-214"));
        assert_eq!(b.iter().filter(|x| x.yazici == "Fis").count(), 3); // offline + sürücü + kuyruk
        assert!(b
            .iter()
            .any(|x| x.yazici == "Etiket2" && x.seviye == Seviye::Uyari));
        assert_eq!(en_kotu(&b), Seviye::Hata);
    }

    #[test]
    fn bos_ve_eksik() {
        assert_eq!(en_kotu(&tani(&[])), Seviye::Hata);
        assert!(csv_coz("a,b\n1,2\n").is_err());
        assert_eq!(en_kotu(&[]), Seviye::Tamam);
    }

    #[test]
    fn windows_disinda_hata() {
        if !cfg!(windows) {
            assert!(windows_listesi().is_err());
        }
    }
}
