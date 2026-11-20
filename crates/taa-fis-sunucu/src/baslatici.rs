//! taa-terzi-takip: masaüstü başlatıcı. Sunucu bu bilgisayarda çalışmıyorsa yanındaki `taa-fis-sunucu`yu
//! başlatır, sonra TAA Terzi Takip'i Edge uygulama penceresinde (`--app`, ayrı profil) açar; Edge yoksa
//! varsayılan tarayıcı. Kullanım: taa-terzi-takip [--port 8787] [sunucuya geçecek diğer bağımsız değişkenler]
#![cfg_attr(windows, windows_subsystem = "windows")]

use taa_fis_sunucu::pencere;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let port: u16 = a
        .iter()
        .position(|x| x == "--port")
        .and_then(|i| a.get(i + 1))
        .and_then(|p| p.parse().ok())
        .unwrap_or(8787);
    if !pencere::sunucu_acik(port) {
        let ad = if cfg!(windows) {
            "taa-fis-sunucu.exe"
        } else {
            "taa-fis-sunucu"
        };
        let yol = std::env::current_exe()
            .ok()
            .and_then(|e| e.parent().map(|p| p.join(ad)))
            .filter(|p| p.is_file())
            .unwrap_or_else(|| ad.into());
        let mut k = std::process::Command::new(&yol);
        k.args(&a);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x0800_0000;
            k.creation_flags(CREATE_NO_WINDOW);
        }
        if let Err(e) = k.spawn() {
            eprintln!("Sunucu başlatılamadı ({}): {e}", yol.display());
            std::process::exit(1);
        }
        // Sunucu dinlemeye başlayana kadar en çok 5 sn bekle.
        for _ in 0..50 {
            if pencere::sunucu_acik(port) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
        }
    }
    if !pencere::ac(&pencere::adres(port)) {
        std::process::exit(2);
    }
}
