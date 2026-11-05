// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! `taa-kur-pencere`: taa-kur için küçük Win32 penceresi (docs/T3-PENCERE.md). Windows dışında yalnız mesaj verir.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(taa_kur::win::calistir() as u8)
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!(
        "Bu pencere yalnız Windows'ta çalışır. Öneri: komut satırı için `taa-kur liste` kullanın."
    );
    std::process::ExitCode::from(1)
}
