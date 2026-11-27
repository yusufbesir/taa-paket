// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! `taa-kur-pencere`: taa-kur için küçük Win32 penceresi (docs/T3-PENCERE.md). Windows dışında yalnız mesaj verir.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(windows)]
fn main() -> std::process::ExitCode {
    taa_hata::kur_varsayilan("taa-kur", env!("CARGO_PKG_VERSION"));
    // Açılış görevi (şube bilgisayarı profili): pencere açmadan taa-yazdir ve taa-sube-dugum'u başlatır.
    // [Claude Fable 5.2 #taa-web-W2-kurulum 2026-11-27]
    if std::env::args().any(|a| a == taa_kur::sube_pc::BASLAT_BAYRAGI) {
        return std::process::ExitCode::from(taa_kur::sube_pc::acilista_baslat() as u8);
    }
    std::process::ExitCode::from(taa_kur::win::calistir() as u8)
}

#[cfg(not(windows))]
fn main() -> std::process::ExitCode {
    eprintln!(
        "Bu pencere yalnız Windows'ta çalışır. Öneri: komut satırı için `taa-kur liste` kullanın."
    );
    std::process::ExitCode::from(1)
}
