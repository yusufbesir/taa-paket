// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! CLI uçtan uca: kur.toml'dan kaynak okuma, `kontrol --json`, bozuk kur.toml tek cümle hata.

use std::fs;
use std::process::Command;

fn calistir(kok: &std::path::Path, args: &[&str]) -> (i32, String, String) {
    let o = Command::new(env!("CARGO_BIN_EXE_taa-guncelle"))
        .args(args)
        .env("TAA_KOK", kok)
        .output()
        .unwrap();
    (
        o.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

#[test]
fn kontrol_json_kur_toml_kaynagiyla() {
    let t = std::env::temp_dir().join(format!("taa-guncelle-cli-{}", std::process::id()));
    let (kok, kaynak) = (t.join("kok"), t.join("kaynak"));
    fs::create_dir_all(&kok).unwrap();
    fs::create_dir_all(&kaynak).unwrap();
    fs::write(kaynak.join("yayin.json"), r#"{"sema":1,"paketler":[]}"#).unwrap();
    fs::write(
        kok.join("kur.toml"),
        format!(
            "kaynak_klasoru = {:?}\nkanal = \"kararli\"\n",
            kaynak.to_string_lossy()
        ),
    )
    .unwrap();
    let (kod, cikti, hata) = calistir(&kok, &["kontrol", "--json"]);
    assert_eq!(kod, 0, "{hata}");
    let v: serde_json::Value = serde_json::from_str(cikti.trim()).unwrap();
    assert_eq!(v["kanal"], "kararli");
    assert_eq!(v["imza"], "denetlenmedi");

    // bozuk kur.toml: tek cümle hata, çıkış 2
    fs::write(kok.join("kur.toml"), "kanal = \n").unwrap();
    let (kod, _, hata) = calistir(&kok, &["kontrol"]);
    assert_eq!(kod, 2);
    assert!(
        hata.contains("bozuk TOML") && hata.trim().lines().count() == 1,
        "{hata}"
    );

    // kaynak yok
    fs::rename(kok.join("kur.toml"), kok.join("kur.toml.yedek")).unwrap();
    let (kod, _, hata) = calistir(&kok, &["uygula"]);
    assert_eq!(kod, 2);
    assert!(hata.contains("kaynak"), "{hata}");
    let (kod, _, _) = calistir(&kok, &["bilinmeyen"]);
    assert_eq!(kod, 2);
}
