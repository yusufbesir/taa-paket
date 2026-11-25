// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz coPLkm 2026-11-17]
// Windows hedefinde exe'lere (taa-kur, taa-kur-pencere) HD Geçme Temel simgesi gömülür: 16–256 px, alfa kanallı.
// Kaynak nesnesi taa-kur-tema ile üretilir (rc.exe / windres gerekmez); bağlayıcıya nesne dosyası olarak verilir.
use std::{env, fs, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../taa-kur-tema/src/simge.rs");
    println!("cargo:rerun-if-changed=../taa-kur-tema/src/logo.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
    let out = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR"));
    let nesne = out.join(if msvc {
        "taa-kurucu-simge.obj"
    } else {
        "taa-kurucu-simge.o"
    });
    let veri = taa_kur_tema::simge::rsrc_nesnesi(&taa_kur_tema::simge::ICO_BOYUTLARI);
    fs::write(&nesne, veri).expect("simge kaynağı yazılamadı");
    println!("cargo:rustc-link-arg-bins={}", nesne.display());
}
