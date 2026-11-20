//! Masaüstü pencere: TAA Terzi Takip, Edge'in uygulama kipinde (`--app`) ayrı bir profil klasörüyle açılır;
//! Edge yoksa varsayılan tarayıcı. Model: taa-cati `panel_main.rs` `pencere_ac`. Windows'a özgü kod
//! `cfg(windows)` arkasındadır; Linux'ta da derlenir (xdg-open).

use std::path::{Path, PathBuf};

/// Pencerenin açacağı adres (yalnız loopback; yönetim kuralı 127.0.0.1 ister).
pub fn adres(port: u16) -> String {
    format!("http://127.0.0.1:{port}/masaustu")
}

/// Edge'e verilecek bağımsız değişkenler (ayrı profil: çerezler ve ayarlar kişisel tarayıcıya karışmaz).
pub fn edge_argumanlari(adres: &str, profil: &Path) -> Vec<String> {
    vec![
        format!("--app={adres}"),
        format!("--user-data-dir={}", profil.display()),
        "--window-size=1280,820".into(),
        "--no-first-run".into(),
        "--no-default-browser-check".into(),
    ]
}

/// Pencere profil klasörü: `%LOCALAPPDATA%\TAA\terzitakip-pencere` (yoksa veri kökünün yanında).
pub fn profil_klasoru() -> PathBuf {
    std::env::var_os("LOCALAPPDATA")
        .filter(|k| !k.is_empty())
        .map(|k| PathBuf::from(k).join("TAA"))
        .unwrap_or_else(|| {
            crate::cati_pin::veri_koku()
                .parent()
                .map(Path::to_path_buf)
                .unwrap_or_default()
        })
        .join("terzitakip-pencere")
}

/// Sunucu bu portta yanıt veriyor mu? (127.0.0.1, 300 ms)
pub fn sunucu_acik(port: u16) -> bool {
    let a = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    std::net::TcpStream::connect_timeout(&a, std::time::Duration::from_millis(300)).is_ok()
}

/// Pencereyi açar. Başarıda true; hiçbir yol çalışmazsa false (adres ekrana yazılır).
pub fn ac(adres: &str) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        let profil = profil_klasoru();
        let _ = std::fs::create_dir_all(&profil);
        let adaylar = ["ProgramFiles(x86)", "ProgramFiles", "LOCALAPPDATA"]
            .iter()
            .filter_map(|e| std::env::var(e).ok())
            .map(|k| {
                Path::new(&k)
                    .join("Microsoft")
                    .join("Edge")
                    .join("Application")
                    .join("msedge.exe")
            });
        for edge in adaylar {
            if edge.is_file()
                && std::process::Command::new(&edge)
                    .args(edge_argumanlari(adres, &profil))
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()
                    .is_ok()
            {
                return true;
            }
        }
        std::process::Command::new("rundll32")
            .args(["url.dll,FileProtocolHandler", adres])
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .is_ok()
    }
    #[cfg(not(windows))]
    {
        if std::process::Command::new("xdg-open")
            .arg(adres)
            .spawn()
            .is_ok()
        {
            true
        } else {
            println!("{adres}");
            false
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn edge_argumanlari_ayri_profil_ve_uygulama_kipi() {
        let a = adres(8787);
        assert_eq!(a, "http://127.0.0.1:8787/masaustu");
        let v = edge_argumanlari(&a, Path::new("/tmp/p"));
        assert_eq!(v[0], "--app=http://127.0.0.1:8787/masaustu");
        assert!(v[1].starts_with("--user-data-dir=") && v[1].ends_with('p'));
        assert!(profil_klasoru().ends_with("terzitakip-pencere"));
    }
}
