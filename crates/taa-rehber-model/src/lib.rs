//! taa-rehber-model: Rehber alan modeli (kişi, birim, görevlendirme, rol), doğrulayıcılar, bellek Deposu, şube süzgeci.
//! Ortak crate: TAA Rehber, TAA Eğitim, Tutaa ve TerziTakip okuyucuları buna bağlanır (docs/rehber-sozlesmesi.md).
//! Sahip: Yusuf. yazan: [Claude#rehber-model 2026-11-26]

pub mod model;

pub fn eposta_gecerli(s: &str) -> bool {
    let Some((yerel, alan)) = s.split_once('@') else {
        return false;
    };
    !yerel.is_empty()
        && !s.chars().any(char::is_whitespace)
        && !alan.contains('@')
        && alan.contains('.')
        && !alan.starts_with('.')
        && !alan.ends_with('.')
}
