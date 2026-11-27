// Sahip: Yusuf. yazan: [Claude#taa-kasa-tk-sozlesme 2026-11-27]
//! Makine geneli kök kaydı. Telefon Köprüsü (TK) hizmeti `TAA_KOK` ortam değişkenini YOK SAYAR; kökü ve
//! kayit.json yerini buradan okur. Kurulum iki yere yazar:
//! 1. `HKLM\SOFTWARE\TAA\Kur` → `Kok`, `KayitJson`, `Depo`, `Sema` (REG_SZ). Yalnız yönetici yazabilir; GÜVENİLİR kaynak budur.
//! 2. `%ProgramData%\TAA\kur-kok.json` → aynı bilgi, JSON. ProgramData'ya sıradan kullanıcı da yazabildiği için
//!    yalnız İPUCUDUR (tanılama/okuma kolaylığı); güvenlik kararı HKLM'ye dayanmalı.
//!
//! HKLM yazılamazsa (kurulum yönetici olarak çalışmadıysa) kurulum durmaz, uyarı verir: "bir kez yönetici olarak çalıştırın".
//! Kaldırmada HKLM kaydı yalnız `Kok` bu kuruluma aitse silinir (başka köke kurulmuş TAA'nın kaydı ezilmez/silinmez).
use std::path::Path;

/// Makine geneli kayıt anahtarı (reg.exe biçimi).
pub const ANAHTAR: &str = r"HKLM\SOFTWARE\TAA\Kur";
/// İpucu dosyası, `%ProgramData%` altında göreli.
pub const IPUCU_GORELI: [&str; 2] = ["TAA", "kur-kok.json"];
/// Kayıt biçimi sürümü (alan eklenirse artar).
pub const SEMA: &str = "1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MakineKok {
    pub kok: String,
    pub kayit_json: String,
    pub depo: String,
}

/// Kurulum hedefinden kayıt değerlerini hesaplar (taa-kur ile aynı yerleşim: `<kök>\kayit.json`, `<kök>\depo`).
pub fn hesapla(hedef: &Path) -> MakineKok {
    MakineKok {
        kok: hedef.display().to_string(),
        kayit_json: hedef.join("kayit.json").display().to_string(),
        depo: hedef.join("depo").display().to_string(),
    }
}

/// `reg.exe` argümanları: her değer için bir `reg add` çağrısı.
pub fn reg_komutlari(m: &MakineKok) -> Vec<Vec<String>> {
    [
        ("Kok", m.kok.as_str()),
        ("KayitJson", m.kayit_json.as_str()),
        ("Depo", m.depo.as_str()),
        ("Sema", SEMA),
    ]
    .iter()
    .map(|(ad, v)| {
        ["add", ANAHTAR, "/v", ad, "/t", "REG_SZ", "/d", v, "/f"]
            .iter()
            .map(|s| s.to_string())
            .collect()
    })
    .collect()
}

fn json_metin(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// `%ProgramData%\TAA\kur-kok.json` içeriği (UTF-8, tek nesne).
pub fn ipucu_json(m: &MakineKok, kurulum_surumu: &str) -> String {
    format!(
        "{{\n  \"sema\": {},\n  \"kok\": {},\n  \"kayit_json\": {},\n  \"depo\": {},\n  \"kurulum_surumu\": {},\n  \"guvenilir_kaynak\": {}\n}}\n",
        SEMA,
        json_metin(&m.kok),
        json_metin(&m.kayit_json),
        json_metin(&m.depo),
        json_metin(kurulum_surumu),
        json_metin(ANAHTAR)
    )
}

/// `reg query <anahtar> /v <ad>` çıktısından değeri çıkarır.
pub fn reg_sorgu_degeri(cikti: &str, ad: &str) -> Option<String> {
    for satir in cikti.lines() {
        let mut p = satir.split_whitespace();
        if p.next().map(|x| x.eq_ignore_ascii_case(ad)) != Some(true) {
            continue;
        }
        let tur = p.next()?;
        if !tur.starts_with("REG_") {
            continue;
        }
        // Değer boşluk içerebilir: türden sonraki kısmı olduğu gibi al.
        let i = satir.find(tur)? + tur.len();
        return Some(satir[i..].trim().to_string());
    }
    None
}

/// Windows yolları büyük/küçük harf duyarsız; sondaki ayırıcı yok sayılır.
fn yol_esit(a: &str, b: &str) -> bool {
    let n = |s: &str| {
        s.trim()
            .trim_end_matches(['\\', '/'])
            .replace('/', "\\")
            .to_lowercase()
    };
    n(a) == n(b)
}

/// Kaldırmada makine kaydı silinsin mi: yalnız kayıtlı kök bu kurulumun köküyse.
pub fn silinmeli_mi(kayitli_kok: Option<&str>, hedef: &Path) -> bool {
    kayitli_kok.is_some_and(|k| yol_esit(k, &hedef.display().to_string()))
}

/// İpucu dosyası bu kökün mü (kaldırmada yalnız kendi yazdığımızı sileriz).
pub fn ipucu_bu_kokun_mu(icerik: &str, hedef: &Path) -> bool {
    let satir = format!("\"kok\": {}", json_metin(&hedef.display().to_string()));
    icerik
        .lines()
        .any(|l| l.trim().trim_end_matches(',') == satir)
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn degerler_kok_kayit_ve_depo() {
        let m = hesapla(&PathBuf::from("ornek-kok"));
        assert_eq!(m.kok, "ornek-kok");
        assert!(m.kayit_json.ends_with("kayit.json") && m.kayit_json.starts_with("ornek-kok"));
        assert!(m.depo.ends_with("depo"));
    }

    #[test]
    fn reg_komutlari_hklm_ve_dort_deger() {
        let m = MakineKok {
            kok: r"D:\Programlar\TAA".into(),
            kayit_json: r"D:\Programlar\TAA\kayit.json".into(),
            depo: r"D:\Programlar\TAA\depo".into(),
        };
        let k = reg_komutlari(&m);
        assert_eq!(k.len(), 4);
        for c in &k {
            assert_eq!(c[0], "add");
            assert_eq!(c[1], r"HKLM\SOFTWARE\TAA\Kur");
            assert_eq!(c[5], "REG_SZ");
            assert_eq!(c.last().unwrap(), "/f");
        }
        let adlar: Vec<&str> = k.iter().map(|c| c[3].as_str()).collect();
        assert_eq!(adlar, ["Kok", "KayitJson", "Depo", "Sema"]);
        assert_eq!(k[1][7], r"D:\Programlar\TAA\kayit.json");
    }

    #[test]
    fn ipucu_json_gecerli_ve_ters_bolu_kacisli() {
        let m = MakineKok {
            kok: r#"C:\Kok "x"\TAA"#.into(),
            kayit_json: r"C:\K\kayit.json".into(),
            depo: r"C:\K\depo".into(),
        };
        let j = ipucu_json(&m, "1.2.3");
        assert!(j.contains(r#""kok": "C:\\Kok \"x\"\\TAA""#), "{j}");
        assert!(j.contains(r#""kayit_json": "C:\\K\\kayit.json""#));
        assert!(j.contains(r#""guvenilir_kaynak": "HKLM\\SOFTWARE\\TAA\\Kur""#));
        assert!(j.contains("\"sema\": 1"));
        // Denge: süslü parantez ve tırnak sayısı.
        assert_eq!(j.matches('{').count(), 1);
        assert_eq!(j.matches('}').count(), 1);
    }

    #[test]
    fn reg_sorgu_ciktisi_ayristirilir() {
        let c = "\r\nHKEY_LOCAL_MACHINE\\SOFTWARE\\TAA\\Kur\r\n    Kok    REG_SZ    D:\\Program Dosyalari\\TAA\r\n\r\n";
        assert_eq!(
            reg_sorgu_degeri(c, "Kok").as_deref(),
            Some(r"D:\Program Dosyalari\TAA")
        );
        assert_eq!(reg_sorgu_degeri(c, "KayitJson"), None);
        assert_eq!(reg_sorgu_degeri("", "Kok"), None);
    }

    #[test]
    fn kaldirma_yalniz_kendi_kokunu_siler() {
        let h = PathBuf::from(r"D:\Programlar\TAA");
        assert!(silinmeli_mi(Some(r"d:\programlar\taa\"), &h));
        assert!(silinmeli_mi(Some(r"D:\Programlar\TAA"), &h));
        assert!(!silinmeli_mi(Some(r"E:\Baska\TAA"), &h));
        assert!(!silinmeli_mi(None, &h));
    }

    #[test]
    fn ipucu_yalniz_kendi_kokuysa_silinir() {
        let h = PathBuf::from(r"D:\Programlar\TAA");
        let j = ipucu_json(&hesapla(&h), "1");
        assert!(ipucu_bu_kokun_mu(&j, &h));
        assert!(!ipucu_bu_kokun_mu(&j, &PathBuf::from(r"E:\TAA")));
        assert!(!ipucu_bu_kokun_mu("", &h));
    }
}
