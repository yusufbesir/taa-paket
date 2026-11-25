// Sahip: Yusuf. yazan: [Claude#dinamik-adres 2026-11-25]
//! Zamanlanmış görev başlatıcısı: Windows'ta pencere açmaz; yanındaki `taa-dns` ile `guncelle` komutunu gizli
//! konsolda çalıştırır. Argümanlar olduğu gibi geçer. Çıkış kodu önemsenmez (sonraki turda yeniden denenir).
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::process::Command;

fn main() {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let ad = if cfg!(windows) {
        "taa-dns.exe"
    } else {
        "taa-dns"
    };
    let mut k = Command::new(exe.with_file_name(ad));
    k.arg("guncelle").args(std::env::args().skip(1));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        k.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let _ = k.status();
}
