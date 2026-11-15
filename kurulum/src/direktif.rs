// Sahip: Yusuf. yazan: [Claude#kur-yerel-pencere 2026-11-15]
//! Kurulumdan sonra bu PC'de yapılacak ayarlar: kısa, numaralı adımlar (rol, şube, güncelleme kaynağı, TAA Sunucu
//! adresi, cihaz onayı, Tailscale). Yapılabilenler kendiliğinden yapılır (kurulum programının yanındaki
//! `sunucu-yollari.json`, `sunucu-ca.pem`, `tailscale.anahtar` var olanı ezmeden kopyalanır); kalanlar adım olarak
//! gösterilir. Adım üretimi saf; kopyalama yalnız std::fs. Bkz. docs/KURULUM-EKRANI.md.

use std::fs;
use std::path::{Path, PathBuf};

/// Kurulum programının yanına konursa kendiliğinden kopyalanan dosyalar.
pub const YAN_YOLLAR: &str = "sunucu-yollari.json";
pub const YAN_CA: &str = "sunucu-ca.pem";
pub const YAN_TAILSCALE: &str = "tailscale.anahtar";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durum {
    /// Kurucu yaptı ya da zaten ayarlı.
    Yapildi,
    /// Kullanıcının yapması gereken adım.
    Yapilacak,
    /// Bilgi: yapılacak bir şey yok ya da başka yerde yapılır.
    Bilgi,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Direktif {
    pub baslik: String,
    pub ayrinti: String,
    pub durum: Durum,
}

fn d(baslik: &str, ayrinti: String, durum: Durum) -> Direktif {
    Direktif { baslik: baslik.to_string(), ayrinti, durum }
}

/// Bir yardımcı dosyanın durumu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dosya {
    #[default]
    Yok,
    /// Hedefte zaten vardı (dokunulmadı).
    Vardi,
    /// Kurulum programının yanından kopyalandı.
    Kopyalandi,
}

impl Dosya {
    fn var(self) -> bool {
        self != Dosya::Yok
    }
}

/// Adımları üretmek için bilinenler.
#[derive(Debug, Clone, Default)]
pub struct Girdi {
    /// "sube" | "sunucu"; None = henüz seçilmedi.
    pub rol: Option<String>,
    /// Seçilen şube (kod, ad).
    pub sube: Option<(String, String)>,
    pub kaynak_url: Option<String>,
    pub yollar: Dosya,
    pub ca: Dosya,
    pub tailscale_anahtar: Dosya,
    pub tailscale_kurulu: bool,
    /// Kurulamayan programlar (sunucu rolünde zorunlu paket adımı için).
    pub hatali: Vec<String>,
    /// Programlar hâlâ kuruluyor (paket adımı henüz işaretlenmez).
    pub kuruluyor: bool,
}

/// Rol ve bilinenlere göre adımlar (en çok 6, sıra sabit).
pub fn uret(g: &Girdi) -> Vec<Direktif> {
    let mut v = vec![];
    let sunucu = g.rol.as_deref() == Some("sunucu");
    v.push(match g.rol.as_deref() {
        Some("sunucu") => d("Rol: Sunucu PC", "rol.txt yazıldı; sunucu paketleri kendiliğinden kurulur.".into(), Durum::Yapildi),
        Some(_) => d("Rol: Şube PC", "rol.txt yazıldı.".into(), Durum::Yapildi),
        None => d("Rolü seçin", "Kurulum ekranında Şube PC ya da Sunucu PC'yi seçin.".into(), Durum::Yapilacak),
    });
    if sunucu {
        v.push(if g.kuruluyor {
            d("Sunucu paketleri", "Kuruluyor; bitince burada işaretlenir.".into(), Durum::Bilgi)
        } else if g.hatali.is_empty() {
            d("Sunucu paketleri", "Seçilen ve zorunlu paketler kuruldu.".into(), Durum::Yapildi)
        } else {
            d(
                "Sunucu paketleri",
                format!("Kurulamayanlar: {}. Yeniden deneyin ya da TAA Kur penceresinden kurun.", g.hatali.join(", ")),
                Durum::Yapilacak,
            )
        });
    } else {
        v.push(match &g.sube {
            Some((kod, ad)) => d(&format!("Şube: {ad}"), format!("sube.txt yazıldı (kod: {kod})."), Durum::Yapildi),
            None => d("Şubeyi seçin", "Kurulum ekranında şubenizi seçin; kod sube.txt'ye yazılır.".into(), Durum::Yapilacak),
        });
    }
    v.push(match &g.kaynak_url {
        Some(u) => d("Güncelleme kaynağı", format!("kur.toml: {u}"), Durum::Yapildi),
        None => d(
            "Güncelleme kaynağını yazın",
            "Kurulum klasöründeki kur.toml dosyasına kaynak_url = \"https://…\" satırını ekleyin.".into(),
            Durum::Yapilacak,
        ),
    });
    if sunucu {
        v.push(d(
            "Şubelere sunucu adresini verin",
            "taa-yol ekle --ad dogrudan --taban https://<bu sunucunun adresi> ile sunucu-yollari.json hazırlayın; dosyayı \
             şube kurulum programının yanına koyun, şube kurulumu kendiliğinden kopyalar."
                .into(),
            Durum::Yapilacak,
        ));
        v.push(d(
            "Şube cihazlarını onaylayın",
            "Şube PC'leri kayıt olunca TAA Çatı'da bekleyen cihazları onaylayın.".into(),
            Durum::Yapilacak,
        ));
        v.push(d(
            "VPN",
            "Sunucu PC'de WireGuard ayrı kurulur; şubelere Tailscale paketle birlikte gelir.".into(),
            Durum::Bilgi,
        ));
        return v;
    }
    let ca = match g.ca {
        Dosya::Kopyalandi => " sunucu-ca.pem de kopyalandı.",
        _ => "",
    };
    v.push(match g.yollar {
        Dosya::Kopyalandi => d("TAA Sunucu adresi", format!("sunucu-yollari.json kopyalandı.{ca}"), Durum::Yapildi),
        Dosya::Vardi => d("TAA Sunucu adresi", format!("sunucu-yollari.json zaten var.{ca}"), Durum::Yapildi),
        Dosya::Yok => d(
            "TAA Sunucu adresini girin",
            format!(
                "Merkezden alınan {YAN_YOLLAR} dosyasını kurulum programının yanına koyup kurulumu yeniden çalıştırın \
                 ya da: taa-yol ekle --ad dogrudan --taban https://<sunucu adresi>"
            ),
            Durum::Yapilacak,
        ),
    });
    let kod = g.sube.as_ref().map(|(k, _)| k.as_str()).unwrap_or("<şube kodu>");
    v.push(d(
        "Cihaz onayı",
        format!(
            "taa-cihaz uret, ardından taa-cihaz kayit --sube {kod}. Merkezdeki yönetici TAA Çatı'dan onaylayınca \
             bu PC sunucuya bağlanır.{}",
            if g.yollar.var() { "" } else { " (Önce sunucu adresi gerekir.)" }
        ),
        Durum::Yapilacak,
    ));
    v.push(match (g.tailscale_kurulu, g.tailscale_anahtar) {
        (true, _) => d("Tailscale", "Kurulu; ağa katılım paket ayarıyla yapılır.".into(), Durum::Yapildi),
        (false, Dosya::Kopyalandi) | (false, Dosya::Vardi) => d(
            "Tailscale",
            "Anahtar hazır; Tailscale gerektiren paket kurulunca ağa kendiliğinden katılır.".into(),
            Durum::Yapildi,
        ),
        (false, Dosya::Yok) => d(
            "Tailscale anahtarı",
            format!(
                "Merkezden alınan {YAN_TAILSCALE} dosyasını kurulum programının yanına koyun (ya da kurulum \
                 klasöründe gizli\\ altına); Tailscale paketle birlikte kurulur ve ağa katılır."
            ),
            Durum::Yapilacak,
        ),
    });
    v
}

/// `kur.toml` içinden `kaynak_url` değeri (boş değilse). Basit satır okuyucu: `kaynak_url = "…"`.
pub fn kaynak_url_oku(toml: &str) -> Option<String> {
    toml.lines().find_map(|l| {
        let (a, b) = l.split_once('=')?;
        if a.trim() != "kaynak_url" {
            return None;
        }
        let v = b.split('#').next()?.trim().trim_matches('"').trim();
        (!v.is_empty()).then(|| v.to_string())
    })
}

/// `hedef` yoksa `kaynak`ı kopyalar (üst klasörü açar). Var olan dosyaya dokunmaz.
pub fn kopyala_yoksa(kaynak: Option<&Path>, hedef: &Path) -> Dosya {
    if hedef.exists() {
        return Dosya::Vardi;
    }
    let Some(k) = kaynak.filter(|k| k.is_file()) else { return Dosya::Yok };
    if let Some(u) = hedef.parent() {
        if fs::create_dir_all(u).is_err() {
            return Dosya::Yok;
        }
    }
    match fs::copy(k, hedef) {
        Ok(_) => Dosya::Kopyalandi,
        Err(_) => Dosya::Yok,
    }
}

/// Otomatik yapılanların sonucu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Otomatik {
    pub yollar: Dosya,
    pub ca: Dosya,
    pub tailscale_anahtar: Dosya,
}

/// Kurulum programının yanındaki (`yan`) yardımcı dosyaları yerine kopyalar:
/// `sunucu-yollari.json` → `<veri kökü>\yol\`, `sunucu-ca.pem` → `<veri kökü>\anahtarlar\`,
/// `tailscale.anahtar` → `<kök>\gizli\`. Anahtar içeriği hiçbir yere yazdırılmaz.
pub fn yan_dosyalari_kopyala(yan: Option<&Path>, kok: &Path, veri_kok: &Path) -> Otomatik {
    let k = |ad: &str| yan.map(|y| y.join(ad));
    Otomatik {
        yollar: kopyala_yoksa(k(YAN_YOLLAR).as_deref(), &veri_kok.join("yol").join(YAN_YOLLAR)),
        ca: kopyala_yoksa(k(YAN_CA).as_deref(), &veri_kok.join("anahtarlar").join(YAN_CA)),
        tailscale_anahtar: kopyala_yoksa(k(YAN_TAILSCALE).as_deref(), &kok.join("gizli").join(YAN_TAILSCALE)),
    }
}

/// Veri kökü (taa-yol, taa-cihaz ile aynı kural): TAA_VERI_KOK, Windows'ta %APPDATA%\TAA\veri,
/// öteki sistemlerde XDG_DATA_HOME/TAA/veri ya da ~/.local/share/TAA/veri.
pub fn veri_kok(ortam: impl Fn(&str) -> Option<String>, windows: bool) -> Option<PathBuf> {
    let ev = |a: &str| ortam(a).filter(|s| !s.trim().is_empty());
    if let Some(k) = ev("TAA_VERI_KOK") {
        return Some(PathBuf::from(k));
    }
    if windows {
        return ev("APPDATA").map(|a| PathBuf::from(a).join("TAA").join("veri"));
    }
    if let Some(x) = ev("XDG_DATA_HOME") {
        return Some(PathBuf::from(x).join("TAA").join("veri"));
    }
    ev("HOME").map(|h| PathBuf::from(h).join(".local/share/TAA/veri"))
}

#[cfg(test)]
mod testler {
    use super::*;

    fn gecici(ad: &str) -> PathBuf {
        let p = std::env::temp_dir().join(format!("taa-kur-direktif-{ad}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn durumlar(v: &[Direktif]) -> Vec<Durum> {
        v.iter().map(|x| x.durum).collect()
    }

    #[test]
    fn sube_rolu_adimlari() {
        let mut g = Girdi { rol: Some("sube".into()), ..Girdi::default() };
        let v = uret(&g);
        assert_eq!(v.len(), 6);
        assert_eq!(
            durumlar(&v),
            vec![Durum::Yapildi, Durum::Yapilacak, Durum::Yapilacak, Durum::Yapilacak, Durum::Yapilacak, Durum::Yapilacak]
        );
        assert!(v[2].ayrinti.contains("kur.toml") && v[5].ayrinti.contains("tailscale.anahtar"));
        assert!(v[4].ayrinti.contains("<şube kodu>") && v[4].ayrinti.contains("Önce sunucu adresi"));
        g.sube = Some(("ankara".into(), "Ankara Mağaza".into()));
        g.kaynak_url = Some("https://ornek/yayin".into());
        g.yollar = Dosya::Kopyalandi;
        g.ca = Dosya::Kopyalandi;
        g.tailscale_anahtar = Dosya::Vardi;
        let v = uret(&g);
        assert_eq!(v[1].baslik, "Şube: Ankara Mağaza");
        assert!(v[3].ayrinti.contains("sunucu-ca.pem de kopyalandı"));
        assert!(v[4].ayrinti.contains("--sube ankara") && !v[4].ayrinti.contains("Önce"));
        assert_eq!(
            durumlar(&v),
            vec![Durum::Yapildi, Durum::Yapildi, Durum::Yapildi, Durum::Yapildi, Durum::Yapilacak, Durum::Yapildi],
            "cihaz onayı her zaman elle"
        );
    }

    #[test]
    fn rol_secilmemis_ve_sunucu() {
        let v = uret(&Girdi::default());
        assert_eq!((v[0].durum, v[0].baslik.as_str()), (Durum::Yapilacak, "Rolü seçin"));
        let mut g = Girdi { rol: Some("sunucu".into()), ..Girdi::default() };
        let v = uret(&g);
        assert_eq!(v.len(), 6);
        assert_eq!(v[1].durum, Durum::Yapildi);
        assert!(v.iter().any(|x| x.baslik.contains("sunucu adresini verin")));
        assert_eq!(v[5].durum, Durum::Bilgi);
        assert!(!v.iter().any(|x| x.baslik.starts_with("Şube:") || x.baslik.starts_with("Şubeyi")), "sunucuda şube adımı yok");
        g.hatali = vec!["taa-sunucu".into()];
        assert!(uret(&g)[1].ayrinti.contains("taa-sunucu"));
        g.kuruluyor = true;
        assert_eq!(uret(&g)[1].durum, Durum::Bilgi, "kurulum sürerken paket adımı işaretlenmez");
    }

    #[test]
    fn kaynak_url_okuma() {
        assert_eq!(kaynak_url_oku("# x\nkaynak_url = \"https://a/b\" # yorum\n").as_deref(), Some("https://a/b"));
        assert_eq!(kaynak_url_oku("kaynak_url_yedek = [\"x\"]\nkanal = \"k\""), None);
        assert_eq!(kaynak_url_oku("kaynak_url = \"\""), None);
    }

    #[test]
    fn yan_dosyalar_kopyalanir_var_olan_ezilmez() {
        let t = gecici("yan");
        let (yan, kok, veri) = (t.join("yan"), t.join("kok"), t.join("veri"));
        fs::create_dir_all(&yan).unwrap();
        fs::write(yan.join(YAN_YOLLAR), "{\"yollar\":[]}").unwrap();
        fs::write(yan.join(YAN_TAILSCALE), "tskey-gizli").unwrap();
        fs::create_dir_all(veri.join("anahtarlar")).unwrap();
        fs::write(veri.join("anahtarlar").join(YAN_CA), "eski").unwrap();
        fs::write(yan.join(YAN_CA), "yeni").unwrap();
        let o = yan_dosyalari_kopyala(Some(&yan), &kok, &veri);
        assert_eq!(o, Otomatik { yollar: Dosya::Kopyalandi, ca: Dosya::Vardi, tailscale_anahtar: Dosya::Kopyalandi });
        assert_eq!(fs::read_to_string(veri.join("yol").join(YAN_YOLLAR)).unwrap(), "{\"yollar\":[]}");
        assert_eq!(fs::read_to_string(veri.join("anahtarlar").join(YAN_CA)).unwrap(), "eski");
        assert_eq!(fs::read_to_string(kok.join("gizli").join(YAN_TAILSCALE)).unwrap(), "tskey-gizli");
        // İkinci çalıştırma: hepsi zaten var.
        let o = yan_dosyalari_kopyala(Some(&yan), &kok, &veri);
        assert_eq!((o.yollar, o.tailscale_anahtar), (Dosya::Vardi, Dosya::Vardi));
        // Yan klasör yoksa: yok.
        assert_eq!(yan_dosyalari_kopyala(None, &t.join("k2"), &t.join("v2")), Otomatik::default());
        let _ = fs::remove_dir_all(&t);
    }

    #[test]
    fn veri_koku_kurali() {
        let o = |m: &'static [(&'static str, &'static str)]| {
            move |a: &str| m.iter().find(|(k, _)| *k == a).map(|(_, v)| v.to_string())
        };
        assert_eq!(veri_kok(o(&[("TAA_VERI_KOK", "/v"), ("APPDATA", "/a")]), true), Some(PathBuf::from("/v")));
        assert_eq!(veri_kok(o(&[("TAA_VERI_KOK", " "), ("APPDATA", "/a")]), true), Some(PathBuf::from("/a/TAA/veri")));
        assert_eq!(veri_kok(o(&[("HOME", "/h")]), false), Some(PathBuf::from("/h/.local/share/TAA/veri")));
        assert_eq!(veri_kok(o(&[]), true), None);
    }
}
