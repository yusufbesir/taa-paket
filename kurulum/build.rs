// Kurulum paketine gömülecek dosyalar TAA_KUR_PAKET klasöründen alınır (tools/kur_paketle.sh hazırlar).
use std::{env, fs, path::{Path, PathBuf}};

fn topla(kok: &Path, k: &Path, out: &mut Vec<String>) {
    let mut e: Vec<PathBuf> = fs::read_dir(k).unwrap().map(|x| x.unwrap().path()).collect();
    e.sort();
    for p in e {
        if p.is_dir() { topla(kok, &p, out); }
        else { out.push(p.strip_prefix(kok).unwrap().to_string_lossy().replace('\\', "/")); }
    }
}

fn main() {
    println!("cargo:rerun-if-env-changed=TAA_KUR_PAKET");
    let paket = env::var("TAA_KUR_PAKET").unwrap_or_else(|_| panic!("TAA_KUR_PAKET ortam değişkeni gerekli (tools/kur_paketle.sh kullanın)"));
    let kok = PathBuf::from(&paket).canonicalize().expect("TAA_KUR_PAKET klasörü yok");
    println!("cargo:rerun-if-changed={}", kok.display());
    let mut liste = vec![];
    topla(&kok, &kok, &mut liste);
    let mut kod = String::from("pub const DOSYALAR: &[(&str, &[u8])] = &[\n");
    for r in &liste { kod.push_str(&format!("    ({:?}, include_bytes!({:?})),\n", r, kok.join(r).to_string_lossy())); }
    kod.push_str("];\n");
    let surum = env::var("TAA_KUR_SURUM").unwrap_or_else(|_| env::var("CARGO_PKG_VERSION").unwrap());
    kod.push_str(&format!("pub const SURUM: &str = {surum:?};\n"));
    fs::write(PathBuf::from(env::var("OUT_DIR").unwrap()).join("dosyalar.rs"), kod).unwrap();
    // HD exe simgesi (16–256 px, alfa kanallı Geçme Temel). [Claude#kurucu-arayuz coPLkm 2026-11-17]
    println!("cargo:rerun-if-changed=../crates/taa-kur-tema/src/simge.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let msvc = env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc");
        let nesne = PathBuf::from(env::var("OUT_DIR").unwrap())
            .join(if msvc { "taa-kurucu-simge.obj" } else { "taa-kurucu-simge.o" });
        fs::write(&nesne, taa_kur_tema::simge::rsrc_nesnesi(&taa_kur_tema::simge::ICO_BOYUTLARI)).unwrap();
        println!("cargo:rustc-link-arg-bins={}", nesne.display());
    }
}
