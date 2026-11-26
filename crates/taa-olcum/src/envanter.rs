//! Donanım ve sistem envanteri. Sahip: Yusuf. yazan: [Claude#olcum 2026-11-26]
//! Kişi adı, bilgisayar adı, kullanıcı adı, seri numarası, MAC ve IP OKUNMAZ. Kimlik rastgele işçi numarasıdır.
//! Linux: /proc ve /sys. Windows: PowerShell (CIM) çıktısı; Windows'ta denenmedi.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Islemci {
    pub model: String,
    pub mantiksal: usize,
    /// Bilinmiyorsa null.
    pub fiziksel: Option<usize>,
    /// Hibrit (P/E) çekirdek bilgisi bu sürümde toplanmaz.
    pub hibrit: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Bellek {
    pub toplam_mb: Option<u64>,
    pub bos_mb: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Disk {
    /// "ssd" | "hdd" | "bilinmiyor"
    pub tur: String,
    pub bos_gb: Option<f64>,
    pub kaynak: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sistem {
    /// "linux" | "windows" | diğer
    pub aile: String,
    pub surum: String,
    pub cekirdek: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Envanter {
    pub isci_no: String,
    pub islemci: Islemci,
    pub bellek: Bellek,
    pub disk: Disk,
    pub sistem: Sistem,
    /// "pil" | "priz" | "bilinmiyor"
    pub guc: String,
    /// Bu sürümde toplanmayan alanlar (açık).
    pub toplanmadi: Vec<String>,
}

pub fn toplanmayanlar() -> Vec<String> {
    [
        "gpu",
        "hat_hizi",
        "hibrit_cekirdek",
        "windows_guncelleme_durumu",
    ]
    .iter()
    .map(|s| (*s).to_string())
    .collect()
}

/// Rastgele işçi numarası: "isci-" + 32 onaltılık. Linux'ta /dev/urandom; yoksa (Windows) std'nin işletim sistemi
/// tohumlu `RandomState`i, zaman ve süreç numarası SHA-256 ile karıştırılır. Kişisel bilgi katılmaz (süreç numarası
/// yalnız karışıma girer, çıktıdan geri çıkarılamaz).
pub fn rastgele_isci_no() -> String {
    let mut ham = [0u8; 16];
    let mut dolu = false;
    #[cfg(unix)]
    {
        use std::io::Read;
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            dolu = f.read_exact(&mut ham).is_ok();
        }
    }
    if !dolu {
        use std::collections::hash_map::RandomState;
        use std::hash::{BuildHasher, Hasher};
        let mut h = Sha256::new();
        for _ in 0..4 {
            let mut x = RandomState::new().build_hasher();
            x.write_u64(std::process::id() as u64);
            h.update(x.finish().to_le_bytes());
        }
        if let Ok(t) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
            h.update(t.as_nanos().to_le_bytes());
        }
        ham.copy_from_slice(&h.finalize()[..16]);
    }
    let hex: String = ham.iter().map(|b| format!("{b:02x}")).collect();
    format!("isci-{hex}")
}

// ---- Ayrıştırıcılar (saf, her platformda test edilir) ----

/// /proc/cpuinfo: ilk işlemci modeli.
pub fn cpuinfo_model(metin: &str) -> Option<String> {
    for anahtar in ["model name", "Model", "Hardware", "cpu model"] {
        for satir in metin.lines() {
            if let Some((k, v)) = satir.split_once(':') {
                if k.trim() == anahtar && !v.trim().is_empty() {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

/// /sys/.../thread_siblings_list değerlerinden benzersiz fiziksel çekirdek sayısı.
pub fn fiziksel_cekirdek(siblings: &[String]) -> Option<usize> {
    let k: BTreeSet<&str> = siblings
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    if k.is_empty() {
        None
    } else {
        Some(k.len())
    }
}

/// /proc/meminfo: (toplam MB, kullanılabilir MB).
pub fn meminfo_mb(metin: &str) -> (Option<u64>, Option<u64>) {
    let al = |anahtar: &str| -> Option<u64> {
        metin.lines().find_map(|s| {
            let (k, v) = s.split_once(':')?;
            if k.trim() != anahtar {
                return None;
            }
            let kb: u64 = v.split_whitespace().next()?.parse().ok()?;
            Some(kb / 1024)
        })
    };
    (al("MemTotal"), al("MemAvailable"))
}

/// /etc/os-release: PRETTY_NAME.
pub fn os_release_ad(metin: &str) -> Option<String> {
    metin.lines().find_map(|s| {
        let v = s.strip_prefix("PRETTY_NAME=")?;
        Some(v.trim().trim_matches('"').to_string())
    })
}

/// `df -Pk <yol>` çıktısı: (aygıt, boş KB).
pub fn df_ayristir(metin: &str) -> Option<(String, u64)> {
    let satir = metin.lines().nth(1)?;
    let alanlar: Vec<&str> = satir.split_whitespace().collect();
    if alanlar.len() < 4 {
        return None;
    }
    let bos: u64 = alanlar[3].parse().ok()?;
    Some((alanlar[0].to_string(), bos))
}

/// Aygıt yolundan blok aygıtı adı: /dev/nvme0n1p2 -> nvme0n1, /dev/sda1 -> sda, /dev/vda -> vda.
pub fn blok_adi(aygit: &str) -> Option<String> {
    let ad = aygit.strip_prefix("/dev/")?;
    if ad.is_empty() || ad.contains('/') {
        return None;
    }
    if ad.starts_with("nvme") || ad.starts_with("mmcblk") {
        // Bölüm eki "p<rakamlar>"; yoksa aygıtın kendisi (nvme0n1, mmcblk0).
        if let Some(i) = ad.rfind('p') {
            if i + 1 < ad.len() && ad[i + 1..].chars().all(|c| c.is_ascii_digit()) {
                return Some(ad[..i].to_string());
            }
        }
        return Some(ad.to_string());
    }
    Some(
        ad.trim_end_matches(|c: char| c.is_ascii_digit())
            .to_string(),
    )
}

/// rotational dosyası içeriğinden tür.
pub fn disk_turu(rotational: &str) -> &'static str {
    match rotational.trim() {
        "0" => "ssd",
        "1" => "hdd",
        _ => "bilinmiyor",
    }
}

/// Windows PowerShell çıktısı: `anahtar=değer` satırları.
pub fn anahtar_deger(metin: &str) -> std::collections::BTreeMap<String, String> {
    metin
        .lines()
        .filter_map(|s| {
            let (k, v) = s.split_once('=')?;
            Some((k.trim().to_string(), v.trim().to_string()))
        })
        .collect()
}

/// Windows MediaType metninden tür.
pub fn windows_disk_turu(medya: &str) -> &'static str {
    match medya.trim().to_ascii_lowercase().as_str() {
        "ssd" => "ssd",
        "hdd" => "hdd",
        _ => "bilinmiyor",
    }
}

// ---- Toplama ----

#[cfg(target_os = "linux")]
fn oku(yol: &str) -> Option<String> {
    std::fs::read_to_string(yol).ok()
}

#[cfg(target_os = "linux")]
pub fn topla() -> Envanter {
    let mantiksal = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    let cpuinfo = oku("/proc/cpuinfo").unwrap_or_default();
    let mut siblings = Vec::new();
    for i in 0..4096usize {
        match oku(&format!(
            "/sys/devices/system/cpu/cpu{i}/topology/thread_siblings_list"
        )) {
            Some(s) => siblings.push(s),
            None => {
                if i > 0
                    && !std::path::Path::new(&format!("/sys/devices/system/cpu/cpu{i}")).exists()
                {
                    break;
                }
            }
        }
    }
    let (toplam_mb, bos_mb) = meminfo_mb(&oku("/proc/meminfo").unwrap_or_default());
    // Disk: çalışma klasörünün bulunduğu dosya sistemi.
    let (disk_tur, bos_gb) = match std::process::Command::new("df").args(["-Pk", "."]).output() {
        Ok(o) if o.status.success() => match df_ayristir(&String::from_utf8_lossy(&o.stdout)) {
            Some((aygit, bos_kb)) => {
                let tur = blok_adi(&aygit)
                    .and_then(|b| oku(&format!("/sys/block/{b}/queue/rotational")))
                    .map(|r| disk_turu(&r).to_string())
                    .unwrap_or_else(|| "bilinmiyor".to_string());
                (tur, Some(bos_kb as f64 / 1024.0 / 1024.0))
            }
            None => ("bilinmiyor".to_string(), None),
        },
        _ => ("bilinmiyor".to_string(), None),
    };
    let guc = linux_guc();
    Envanter {
        isci_no: rastgele_isci_no(),
        islemci: Islemci {
            model: cpuinfo_model(&cpuinfo).unwrap_or_else(|| "bilinmiyor".to_string()),
            mantiksal,
            fiziksel: fiziksel_cekirdek(&siblings),
            hibrit: None,
        },
        bellek: Bellek { toplam_mb, bos_mb },
        disk: Disk {
            tur: disk_tur,
            bos_gb: bos_gb.map(yuvarla1),
            kaynak: "calisma-klasoru".to_string(),
        },
        sistem: Sistem {
            aile: "linux".to_string(),
            surum: os_release_ad(&oku("/etc/os-release").unwrap_or_default())
                .unwrap_or_else(|| "bilinmiyor".to_string()),
            cekirdek: oku("/proc/sys/kernel/osrelease")
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "bilinmiyor".to_string()),
        },
        guc,
        toplanmadi: toplanmayanlar(),
    }
}

#[cfg(target_os = "linux")]
fn linux_guc() -> String {
    let Ok(d) = std::fs::read_dir("/sys/class/power_supply") else {
        return "bilinmiyor".to_string();
    };
    let mut pil_var = false;
    let mut priz_bagli = false;
    let mut gordu = false;
    for e in d.flatten() {
        let yol = e.path();
        let tur = std::fs::read_to_string(yol.join("type")).unwrap_or_default();
        gordu = true;
        match tur.trim() {
            "Battery" => {
                pil_var = true;
                let durum = std::fs::read_to_string(yol.join("status")).unwrap_or_default();
                if durum.trim() == "Discharging" {
                    return "pil".to_string();
                }
            }
            "Mains" | "USB"
                if std::fs::read_to_string(yol.join("online"))
                    .map(|s| s.trim() == "1")
                    .unwrap_or(false) =>
            {
                priz_bagli = true;
            }
            _ => {}
        }
    }
    if priz_bagli || pil_var {
        "priz".to_string()
    } else if gordu {
        "bilinmiyor".to_string()
    } else {
        // Güç kaynağı kaydı yok: genellikle masaüstü/sunucu; yine de uydurmayız.
        "bilinmiyor".to_string()
    }
}

#[cfg(windows)]
pub fn topla() -> Envanter {
    // Windows'ta denenmedi. Yalnız std + PowerShell (CIM); bilgisayar adı, kullanıcı adı, seri okunmaz.
    let betik = "$o=Get-CimInstance Win32_OperatingSystem;$c=Get-CimInstance Win32_Processor|Select-Object -First 1;\
        'os_ad='+$o.Caption;'os_surum='+$o.Version;'bellek_toplam_kb='+$o.TotalVisibleMemorySize;\
        'bellek_bos_kb='+$o.FreePhysicalMemory;'islemci='+$c.Name;'fiziksel='+$c.NumberOfCores;\
        $d=Get-PhysicalDisk|Select-Object -First 1;'disk_tur='+$d.MediaType;\
        'bos_bayt='+(Get-PSDrive -Name (Get-Location).Drive.Name).Free;\
        'pil='+@(Get-CimInstance Win32_Battery).Count";
    let cikti = std::process::Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", betik])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    let m = anahtar_deger(&cikti);
    let say = |k: &str| m.get(k).and_then(|v| v.parse::<u64>().ok());
    let mantiksal = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    Envanter {
        isci_no: rastgele_isci_no(),
        islemci: Islemci {
            model: m
                .get("islemci")
                .cloned()
                .filter(|s| !s.is_empty())
                .or_else(|| std::env::var("PROCESSOR_IDENTIFIER").ok())
                .unwrap_or_else(|| "bilinmiyor".to_string()),
            mantiksal,
            fiziksel: say("fiziksel").map(|n| n as usize),
            hibrit: None,
        },
        bellek: Bellek {
            toplam_mb: say("bellek_toplam_kb").map(|k| k / 1024),
            bos_mb: say("bellek_bos_kb").map(|k| k / 1024),
        },
        disk: Disk {
            tur: windows_disk_turu(m.get("disk_tur").map(String::as_str).unwrap_or("")).to_string(),
            bos_gb: say("bos_bayt").map(|b| yuvarla1(b as f64 / 1024.0 / 1024.0 / 1024.0)),
            kaynak: "calisma-klasoru".to_string(),
        },
        sistem: Sistem {
            aile: "windows".to_string(),
            surum: format!(
                "{} {}",
                m.get("os_ad").cloned().unwrap_or_default(),
                m.get("os_surum").cloned().unwrap_or_default()
            )
            .trim()
            .to_string(),
            cekirdek: m
                .get("os_surum")
                .cloned()
                .unwrap_or_else(|| "bilinmiyor".to_string()),
        },
        guc: match say("pil") {
            Some(0) => "priz".to_string(),
            Some(_) => "pil".to_string(),
            None => "bilinmiyor".to_string(),
        },
        toplanmadi: toplanmayanlar(),
    }
}

#[cfg(not(any(target_os = "linux", windows)))]
pub fn topla() -> Envanter {
    let mantiksal = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    Envanter {
        isci_no: rastgele_isci_no(),
        islemci: Islemci {
            model: "bilinmiyor".to_string(),
            mantiksal,
            fiziksel: None,
            hibrit: None,
        },
        bellek: Bellek {
            toplam_mb: None,
            bos_mb: None,
        },
        disk: Disk {
            tur: "bilinmiyor".to_string(),
            bos_gb: None,
            kaynak: "calisma-klasoru".to_string(),
        },
        sistem: Sistem {
            aile: std::env::consts::OS.to_string(),
            surum: "bilinmiyor".to_string(),
            cekirdek: "bilinmiyor".to_string(),
        },
        guc: "bilinmiyor".to_string(),
        toplanmadi: toplanmayanlar(),
    }
}

fn yuvarla1(x: f64) -> f64 {
    (x * 10.0).round() / 10.0
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn isci_no_rastgele_ve_bicimli() {
        let a = rastgele_isci_no();
        let b = rastgele_isci_no();
        assert_ne!(a, b);
        assert!(a.starts_with("isci-") && a.len() == 5 + 32);
        assert!(a[5..].chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn cpuinfo_ve_meminfo() {
        let c = "processor\t: 0\nmodel name\t: Örnek İşlemci X 3.0GHz\ncpu cores\t: 4\n";
        assert_eq!(cpuinfo_model(c).as_deref(), Some("Örnek İşlemci X 3.0GHz"));
        assert_eq!(cpuinfo_model("yok: yok"), None);
        let m = "MemTotal:       16384000 kB\nMemFree: 100 kB\nMemAvailable:    8192000 kB\n";
        assert_eq!(meminfo_mb(m), (Some(16000), Some(8000)));
        assert_eq!(meminfo_mb(""), (None, None));
    }

    #[test]
    fn fiziksel_cekirdek_benzersiz_sayar() {
        let s: Vec<String> = ["0,4", "1,5", "2,6", "3,7", "0,4", "1,5", "2,6", "3,7"]
            .iter()
            .map(|x| x.to_string())
            .collect();
        assert_eq!(fiziksel_cekirdek(&s), Some(4));
        assert_eq!(fiziksel_cekirdek(&[]), None);
    }

    #[test]
    fn df_ve_blok_adi() {
        let d = "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/nvme0n1p2 100000 50000 12345678 50% /\n";
        assert_eq!(
            df_ayristir(d),
            Some(("/dev/nvme0n1p2".to_string(), 12345678))
        );
        assert_eq!(df_ayristir("x"), None);
        assert_eq!(blok_adi("/dev/nvme0n1p2").as_deref(), Some("nvme0n1"));
        assert_eq!(blok_adi("/dev/nvme0n1").as_deref(), Some("nvme0n1"));
        assert_eq!(blok_adi("/dev/sda1").as_deref(), Some("sda"));
        assert_eq!(blok_adi("/dev/vda").as_deref(), Some("vda"));
        assert_eq!(blok_adi("/dev/mmcblk0p1").as_deref(), Some("mmcblk0"));
        assert_eq!(blok_adi("overlay"), None);
        assert_eq!(disk_turu("0\n"), "ssd");
        assert_eq!(disk_turu("1"), "hdd");
        assert_eq!(disk_turu(""), "bilinmiyor");
    }

    #[test]
    fn os_release_ve_windows_ayristirma() {
        assert_eq!(
            os_release_ad("NAME=X\nPRETTY_NAME=\"Örnek Linux 1.0\"\n").as_deref(),
            Some("Örnek Linux 1.0")
        );
        let m = anahtar_deger(
            "os_ad=Windows 11 Pro\nbellek_toplam_kb=16777216\nçöp satır\ndisk_tur=SSD\n",
        );
        assert_eq!(m.get("os_ad").map(String::as_str), Some("Windows 11 Pro"));
        assert_eq!(
            m.get("bellek_toplam_kb").map(String::as_str),
            Some("16777216")
        );
        assert_eq!(windows_disk_turu("SSD"), "ssd");
        assert_eq!(windows_disk_turu("Unspecified"), "bilinmiyor");
    }

    #[test]
    fn topla_makul_ve_kisisel_alan_yok() {
        let e = topla();
        assert!(e.islemci.mantiksal >= 1);
        assert!(!e.islemci.model.is_empty());
        assert!(e.isci_no.starts_with("isci-"));
        let json = serde_json::to_string(&e).unwrap();
        for yasak in [
            "hostname",
            "bilgisayar_adi",
            "kullanici",
            "seri",
            "mac",
            "ip",
        ] {
            assert!(!json.contains(&format!("\"{yasak}\"")), "{yasak}");
        }
        // Ortam değişkenlerinden kullanıcı/bilgisayar adı sızmamış olmalı.
        for ad in ["HOSTNAME", "COMPUTERNAME", "USER", "USERNAME"] {
            if let Ok(v) = std::env::var(ad) {
                if v.len() >= 6 {
                    assert!(!json.contains(&v), "{ad} değeri çıktıya sızdı");
                }
            }
        }
        assert!(e.toplanmadi.contains(&"gpu".to_string()));
    }
}
