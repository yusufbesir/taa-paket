// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-kurulum 2026-11-27]
//! Şube bilgisayarı profili (PLAN-TAA-WEB-2026-11-27 §9): şube rolündeki PC'ye `taa-yazdir` ve `taa-sube-dugum`
//! kurulur ve başlatılır; yol listesi (merkez → şube → yerel) ve günlük klasörü hazırlanır. İşi kurulan
//! `taa-kur sube-pc kur` yapar; burada yalnız karar (kurulsun mu), komut satırı ve çıktının özeti vardır (saf, testli).
//! Çatı şubeye kurulmaz: şube rolünde Çatı paketleri seçilemez.

use std::path::Path;
use std::process::Command;

/// Seçimin yazıldığı dosya (kurulum kökünde; var olan ezilmez).
pub const SECIM_DOSYASI: &str = "sube-pc.txt";
/// Profilin kurulu olduğunu gösteren dosya (`taa-kur sube-pc kur` yazar).
pub const PROFIL_DOSYASI: &str = "sube-pc.json";

/// Profil kurulumunun sonucu (kurulum ekranındaki adım bundan üretilir).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SubePc {
    /// Henüz karar verilmedi (şube rolü değil ya da adım gelmedi): ekranda adım gösterilmez.
    #[default]
    Sorulmadi,
    /// Kullanıcı bu PC'nin şube bilgisayarı olmasını istemedi.
    Istenmedi,
    /// Kuruldu: yol sırası ve `taa-kur`un uyarıları.
    Kuruldu { yol_sirasi: String, uyarilar: Vec<String> },
    /// Kurulamadı: neden.
    Olmadi(String),
}

/// Çatı paketi mi (şube rolünde kurulmaz). Ad küçük/büyük harfe ve `taa-` önekine bakmadan karşılaştırılır.
pub fn cati_paketi(ad: &str) -> bool {
    let a = ad.trim().to_lowercase();
    let a = a.strip_prefix("taa-").unwrap_or(&a);
    a == "cati" || a.starts_with("cati-") || a == "taacati"
}

/// `--sube-pc evet|hayir` değeri ya da kayıtlı seçim: `Some(true/false)`; tanınmıyorsa `None`.
pub fn secim_coz(deger: &str) -> Option<bool> {
    match deger.trim().to_lowercase().as_str() {
        "evet" | "e" | "1" => Some(true),
        "hayir" | "hayır" | "h" | "0" => Some(false),
        _ => None,
    }
}

/// `taa-kur sube-pc kur` bağımsız değişkenleri: kurulum programına verilen `--merkez`, `--sube-adres`,
/// `--dugum-port` aynen geçer; `--sube-lan evet` düğümü telefonlara (yerel ağa) açar (varsayılan kapalı: düğümde cihaz
/// doğrulaması henüz yok).
pub fn komut_argumanlari(merkez: Option<&str>, sube_adres: Option<&str>, port: Option<&str>, lan_ac: bool) -> Vec<String> {
    let mut v = vec!["sube-pc".to_string(), "kur".to_string()];
    if lan_ac {
        v.push("--lan-ac".to_string());
    }
    for (ad, deger) in [("--merkez", merkez), ("--sube-adres", sube_adres), ("--port", port)] {
        if let Some(d) = deger.map(str::trim).filter(|d| !d.is_empty()) {
            v.push(ad.to_string());
            v.push(d.to_string());
        }
    }
    v
}

/// `taa-kur sube-pc kur` çıktısından özet: çıkış 0 ise yol sırası ve uyarılar, değilse hata satırı.
pub fn cikti_ozeti(kod: Option<i32>, stdout: &str, stderr: &str) -> SubePc {
    if kod != Some(0) {
        let neden = stderr.lines().chain(stdout.lines()).map(str::trim).find(|l| !l.is_empty()).unwrap_or("taa-kur hata verdi");
        return SubePc::Olmadi(neden.to_string());
    }
    let bul = |onek: &'static str| stdout.lines().filter_map(move |l| l.trim().strip_prefix(onek)).map(|s| s.trim().to_string());
    let mut uyarilar: Vec<String> = bul("UYARI:").collect();
    // Başlatılamayan hizmet ve kurulamayan açılış kaydı da uyarıdır.
    for l in stdout.lines().map(str::trim) {
        if (l.starts_with("Hizmet:") && l.contains("baslatilamadi")) || (l.starts_with("Açılış kaydı:") && l.contains("yapilamadi")) || (l.starts_with("Güvenlik duvarı:") && l.contains("yapilamadi")) {
            uyarilar.push(l.to_string());
        }
    }
    SubePc::Kuruldu { yol_sirasi: bul("Yol sırası:").next().unwrap_or_default(), uyarilar }
}

/// Kurulan `taa-kur` ile profili kurar. `veri_kok` açıkça verilir (kurulumun kullandığı veri köküyle aynı olsun diye).
pub fn kur(hedef: &Path, veri_kok: &Path, argumanlar: &[String]) -> SubePc {
    let exe = hedef.join(if cfg!(windows) { "taa-kur.exe" } else { "taa-kur" });
    let mut c = Command::new(&exe);
    c.args(argumanlar).env("TAA_KOK", hedef).env("TAA_VERI_KOK", veri_kok);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    match c.output() {
        Ok(o) => cikti_ozeti(o.status.code(), &String::from_utf8_lossy(&o.stdout), &String::from_utf8_lossy(&o.stderr)),
        Err(e) => SubePc::Olmadi(format!("{} çalıştırılamadı: {e}", exe.display())),
    }
}

/// Kaldırmadan önce açılış kaydını kaldırır (yoksa kayıt silinmiş exe'yi gösterirdi). Dosya ve veri silmez.
pub fn acilis_kaydini_kaldir(hedef: &Path) {
    if !hedef.join(PROFIL_DOSYASI).exists() {
        return;
    }
    let exe = hedef.join(if cfg!(windows) { "taa-kur.exe" } else { "taa-kur" });
    let mut c = Command::new(exe);
    c.args(["sube-pc", "gorev-kaldir"]).env("TAA_KOK", hedef);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000);
    }
    let _ = c.output();
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn cati_subeye_kurulmaz() {
        for ad in ["taa-cati", "TAA-Cati", "cati", "taa-cati-panel", "cati-panel", "taacati"] {
            assert!(cati_paketi(ad), "{ad}");
        }
        for ad in ["taa-kasa", "tutaa", "taa-catisma", "terzi-takip", "taa-sube-dugum", "taa-yazdir"] {
            assert!(!cati_paketi(ad), "{ad}");
        }
    }

    #[test]
    fn secim_degerleri() {
        assert_eq!(secim_coz("evet\n"), Some(true));
        assert_eq!(secim_coz("Hayır"), Some(false));
        assert_eq!(secim_coz("hayir"), Some(false));
        assert_eq!(secim_coz("belki"), None);
    }

    #[test]
    fn komut_satiri() {
        assert_eq!(komut_argumanlari(None, None, None, false), ["sube-pc", "kur"]);
        assert_eq!(
            komut_argumanlari(Some("https://merkez.ornek.com"), Some(" http://192.168.1.20:8788 "), Some(""), true),
            ["sube-pc", "kur", "--lan-ac", "--merkez", "https://merkez.ornek.com", "--sube-adres", "http://192.168.1.20:8788"]
        );
        assert_eq!(komut_argumanlari(None, None, Some("9000"), false), ["sube-pc", "kur", "--port", "9000"]);
    }

    #[test]
    fn cikti_ozetlenir() {
        let cikti = "Şube bilgisayarı profili kuruldu.\nYol sırası: merkez (https://m) → sube (http://192.168.1.20:8788) → yerel\n\
                     Açılış kaydı: kuruldu (gorev)\nGüvenlik duvarı: yapilamadi. Yönetici olarak çalıştırın: netsh …\n\
                     Hizmet: taa-yazdir (kapı 8797) baslatildi\nHizmet: taa-sube-dugum (kapı 8788) baslatilamadi: dosya yok\n\
                     UYARI: Şube adresi IP ile yazıldı.\n";
        match cikti_ozeti(Some(0), cikti, "") {
            SubePc::Kuruldu { yol_sirasi, uyarilar } => {
                assert_eq!(yol_sirasi, "merkez (https://m) → sube (http://192.168.1.20:8788) → yerel");
                assert_eq!(uyarilar.len(), 3, "{uyarilar:?}");
                assert!(uyarilar[0].starts_with("Şube adresi"));
                assert!(uyarilar.iter().any(|u| u.contains("taa-sube-dugum")));
                assert!(uyarilar.iter().any(|u| u.contains("Güvenlik duvarı")));
            }
            x => panic!("{x:?}"),
        }
        assert_eq!(
            cikti_ozeti(Some(4), "", "\ntaa-yazdir ve taa-sube-dugum bu bilgisayarda bulunamadı.\n"),
            SubePc::Olmadi("taa-yazdir ve taa-sube-dugum bu bilgisayarda bulunamadı.".into())
        );
        assert_eq!(cikti_ozeti(None, "", ""), SubePc::Olmadi("taa-kur hata verdi".into()));
    }

    #[test]
    fn taa_kur_yoksa_olmadi() {
        let t = std::env::temp_dir().join(format!("taa-kur-subepc-{}", std::process::id()));
        std::fs::create_dir_all(&t).unwrap();
        assert!(matches!(kur(&t, &t.join("veri"), &komut_argumanlari(None, None, None, false)), SubePc::Olmadi(_)));
        // Profil yokken açılış kaydı kaldırma hiçbir şey çalıştırmaz (panik yok).
        acilis_kaydini_kaldir(&t);
        let _ = std::fs::remove_dir_all(&t);
    }
}
