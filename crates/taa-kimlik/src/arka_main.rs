// Sahip: Yusuf. yazan: [Claude#hata-bildirimi 2026-11-18]
//! Zamanlanmış görev başlatıcısı: Windows'ta pencere açmaz (GUI alt sistemi); yanındaki `taa-kimlik` ile
//! `gonder` komutunu gizli konsolda çalıştırır. Argümanlar olduğu gibi `taa-kimlik gonder`e geçer.
//! Çıkış kodu önemsenmez (sunucuya ulaşılamazsa sonraki turda yeniden denenir).
#![cfg_attr(windows, windows_subsystem = "windows")]

use std::process::Command;

fn main() {
    let exe = match std::env::current_exe() {
        Ok(e) => e,
        Err(_) => return,
    };
    let ad = if cfg!(windows) {
        "taa-kimlik.exe"
    } else {
        "taa-kimlik"
    };
    let hedef = exe.with_file_name(ad);
    let mut k = Command::new(hedef);
    k.arg("gonder").args(std::env::args().skip(1));
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        k.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let _ = k.status();
}
