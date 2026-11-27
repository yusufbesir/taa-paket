// Sahip: Yusuf. yazan: [Claude Sonnet 5.5 #taa-web-cihaz 2026-11-27]
// Tarayıcı (WebCrypto Ed25519, web/cekirdek/cihaz.js) tarafından üretilmiş sabit bir imza, Rust doğrulayıcısından geçmeli.
use taa_cihaz::*;
#[test]
fn js_imzasi_rustta_dogrulanir() {
    let g = taa_imza::GenelAnahtar::coz(
        "ed25519:6cdbf6788b87969cae21d85458a616a9dd5ed64b1a19d4e55a36a69caebbbc03",
    )
    .expect("anahtar");
    dogrula(&g, "POST", "/v1/kasa/gun?x=1", 1795000000, br#"{"no":"k-1"}"#, "40e25f76749c6be26d60a5ec0c3a456f8b2f4ffc77eefbd91a0962877fe3cbe1d3a58499bb7727a990767b2f4d5d7914b22337e10bbcb1713ac1bbf372c32008", 1795000000).expect("imza");
}
