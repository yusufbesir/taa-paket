//! Yönetici PIN'i doğrulaması: PIN bu sunucuda saklanmaz, TAA Çatı sunucusu doğrular.
//! Uç: `POST /v1/yonetici-pin/dogrula {program:"terzitakip", pin}`; olağan oturum + cihaz imzasıyla gider
//! (şube kimliği gönderilmez, Çatı oturumdan bilir). PIN hiçbir yere yazılmaz: günlüğe, hata iletisine, dosyaya.
//! curl gövdeyi stdin'den aldığı için PIN komut satırında da görünmez (taa-kuyruk `CurlGonderici`).

use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use taa_kuyruk::{GonderHata, Gonderici};

pub const DOGRULA_UCU: &str = "/v1/yonetici-pin/dogrula";
pub const PROGRAM: &str = "terzitakip";
pub const ULASILAMIYOR: &str = "TAA Çatı'ya ulaşılamıyor, yönetim ekranları açılamaz";
pub const BELIRLENMEMIS: &str = "Yönetici PIN'i TAA Çatı'dan belirlenmemiş";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PinSonuc {
    Gecerli,
    Yanlis,
    /// Çatı'da bu program için PIN yok.
    Belirlenmemis,
    /// Çok fazla yanlış deneme (HTTP 423).
    Kilitli {
        ileti: String,
        kalan_sn: u64,
    },
    /// Ağ, oturum ya da yapılandırma sorunu. İleti PIN içermez.
    Ulasilamadi(String),
}

/// PIN'i doğrulayan taraf. Testlerde sahte uygulanır.
pub trait PinDogrulayici: Send + Sync {
    fn dogrula(&self, pin: &str) -> PinSonuc;
}

/// Çatı bağlantısı kurulmamış sunucu: yönetim ekranları açılmaz.
pub struct Yapilandirilmamis;

impl PinDogrulayici for Yapilandirilmamis {
    fn dogrula(&self, _pin: &str) -> PinSonuc {
        PinSonuc::Ulasilamadi("TAA Çatı bağlantısı yapılandırılmamış".into())
    }
}

/// Gönderilecek gövde (PIN yalnız burada bulunur; gövde hiçbir yere yazılmaz).
pub fn govde(pin: &str) -> Value {
    json!({ "program": PROGRAM, "pin": pin })
}

/// Çatı yanıtını sonuca çevirir. Saf işlev; ağdan bağımsız sınanır.
pub fn yanit_coz(r: Result<Value, GonderHata>) -> PinSonuc {
    match r {
        Ok(v) => {
            if v["gecerli"].as_bool() == Some(true) {
                PinSonuc::Gecerli
            } else if v["belirlendi"].as_bool() == Some(false) {
                PinSonuc::Belirlenmemis
            } else if v["gecerli"].as_bool() == Some(false) {
                PinSonuc::Yanlis
            } else {
                PinSonuc::Ulasilamadi("TAA Çatı'dan anlaşılmayan yanıt".into())
            }
        }
        Err(GonderHata::Reddedildi { kod: 423, mesaj }) => {
            let v: Value = serde_json::from_str(&mesaj).unwrap_or_default();
            PinSonuc::Kilitli {
                ileti: v["ileti"]
                    .as_str()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or("Çok fazla yanlış deneme")
                    .to_string(),
                kalan_sn: v["kalan_sn"].as_u64().unwrap_or(60),
            }
        }
        Err(GonderHata::Reddedildi { kod: 401 | 403, .. }) => PinSonuc::Ulasilamadi(
            "TAA Çatı oturumu geçersiz; TAA Çatı'ya yeniden giriş yapın".into(),
        ),
        Err(GonderHata::Reddedildi { kod, .. }) => {
            PinSonuc::Ulasilamadi(format!("TAA Çatı isteği reddetti (HTTP {kod})"))
        }
        Err(GonderHata::Ulasilamadi(_)) => PinSonuc::Ulasilamadi(ULASILAMIYOR.into()),
        Err(GonderHata::Yapilandirma(m)) => PinSonuc::Ulasilamadi(m),
    }
}

/// Herhangi bir gönderici ile doğrular (sahte gönderici ile sınanır).
pub fn gonderici_ile(g: &dyn Gonderici, pin: &str) -> PinSonuc {
    yanit_coz(g.gonder(DOGRULA_UCU, &govde(pin)))
}

/// Gerçek istemci: TAA Çatı'nın kullandığı veri kökündeki oturum + cihaz anahtarı + yol listesi.
pub struct CatiPinIstemcisi {
    pub kok: PathBuf,
    pub sunucu: Option<String>,
    pub sertifika: Option<PathBuf>,
}

/// TAA Çatı ile aynı oturum + cihaz anahtarı + yol listesiyle imzalı gönderici kurar (ağa dokunmaz).
/// PIN doğrulaması ve kişi/şube eşitlemesi (`kurulum`) aynı göndericiyi kullanır.
pub fn cati_gonderici(
    kok: &Path,
    sunucu: Option<&str>,
    sertifika: Option<&Path>,
) -> Result<taa_kuyruk::CurlGonderici, String> {
    let h = taa_yol::hedef_coz_curl(kok, sunucu, sertifika)
        .map_err(|e| format!("TAA Çatı sunucu adresi bulunamadı: {e}"))?;
    let o = taa_kimlik::varsayilan_depo(kok)
        .oku(&h.anahtar)
        .map_err(|_| "TAA Çatı oturumu yok; önce TAA Çatı'ya giriş yapın".to_string())?;
    let cihaz = taa_kimlik::cihaz_imzalayici(kok)
        .map_err(|e| format!("cihaz anahtarı okunamadı: {e}"))?
        .ok_or_else(|| "bu bilgisayarın cihaz anahtarı yok".to_string())?;
    o.gonderici_taban(&h.taban, h.sertifika.as_deref(), Some(cihaz))
        .map(|g| g.baglanti_suresi(5))
        .map_err(|e| e.to_string())
}

impl CatiPinIstemcisi {
    fn gonderici(&self) -> Result<taa_kuyruk::CurlGonderici, String> {
        cati_gonderici(&self.kok, self.sunucu.as_deref(), self.sertifika.as_deref())
    }
}

impl PinDogrulayici for CatiPinIstemcisi {
    fn dogrula(&self, pin: &str) -> PinSonuc {
        match self.gonderici() {
            Ok(g) => gonderici_ile(&g, pin),
            Err(m) => PinSonuc::Ulasilamadi(m),
        }
    }
}

/// TAA programlarının ortak veri kökü: `TAA_VERI_KOK`, yoksa Windows'ta `%APPDATA%\TAA\veri`,
/// diğerlerinde `$XDG_DATA_HOME/TAA/veri` ya da `~/.local/share/TAA/veri`.
pub fn veri_koku() -> PathBuf {
    if let Some(k) = std::env::var_os("TAA_VERI_KOK").filter(|k| !k.is_empty()) {
        return PathBuf::from(k);
    }
    if cfg!(windows) {
        if let Some(a) = std::env::var_os("APPDATA") {
            return PathBuf::from(a).join("TAA").join("veri");
        }
    }
    if let Some(x) = std::env::var_os("XDG_DATA_HOME").filter(|k| !k.is_empty()) {
        return PathBuf::from(x).join("TAA").join("veri");
    }
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".local/share/TAA/veri"))
        .unwrap_or_else(|| PathBuf::from("TAA-veri"))
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::sync::Mutex;

    struct Sahte {
        yanit: Result<Value, GonderHata>,
        gelen: Mutex<Vec<(String, Value)>>,
    }

    impl Gonderici for Sahte {
        fn gonder(&self, uc: &str, govde: &Value) -> Result<Value, GonderHata> {
            self.gelen.lock().unwrap().push((uc.into(), govde.clone()));
            self.yanit.clone()
        }
    }

    fn sahte(y: Result<Value, GonderHata>) -> Sahte {
        Sahte {
            yanit: y,
            gelen: Mutex::new(Vec::new()),
        }
    }

    #[test]
    fn istek_bicimi_ve_yanitlar() {
        let s = sahte(Ok(json!({"gecerli": true, "belirlendi": true})));
        assert_eq!(gonderici_ile(&s, "4821"), PinSonuc::Gecerli);
        let g = s.gelen.lock().unwrap();
        assert_eq!(g[0].0, "/v1/yonetici-pin/dogrula");
        assert_eq!(g[0].1, json!({"program": "terzitakip", "pin": "4821"}));
        assert!(g[0].1.get("sube_id").is_none());
        drop(g);
        let s = sahte(Ok(json!({"gecerli": false, "belirlendi": true})));
        assert_eq!(gonderici_ile(&s, "1"), PinSonuc::Yanlis);
        let s = sahte(Ok(json!({"gecerli": false, "belirlendi": false})));
        assert_eq!(gonderici_ile(&s, "1"), PinSonuc::Belirlenmemis);
    }

    #[test]
    fn kilit_ve_ulasilamama() {
        let s = sahte(Err(GonderHata::Reddedildi {
            kod: 423,
            mesaj: r#"{"hata":"kilitli","ileti":"5 yanlış deneme","kalan_sn":42}"#.into(),
        }));
        assert_eq!(
            gonderici_ile(&s, "0000"),
            PinSonuc::Kilitli {
                ileti: "5 yanlış deneme".into(),
                kalan_sn: 42
            }
        );
        let s = sahte(Err(GonderHata::Ulasilamadi("zaman aşımı".into())));
        assert_eq!(
            gonderici_ile(&s, "0000"),
            PinSonuc::Ulasilamadi(ULASILAMIYOR.into())
        );
        // Hiçbir ileti PIN'i içermez.
        for r in [
            Err(GonderHata::Reddedildi {
                kod: 401,
                mesaj: "x".into(),
            }),
            Err(GonderHata::Reddedildi {
                kod: 400,
                mesaj: "9876".into(),
            }),
            Ok(json!({})),
        ] {
            let m = format!("{:?}", gonderici_ile(&sahte(r), "9876"));
            assert!(!m.contains("9876"), "{m}");
        }
    }

    #[test]
    fn yapilandirilmamis_ulasilamaz() {
        assert!(matches!(
            Yapilandirilmamis.dogrula("1234"),
            PinSonuc::Ulasilamadi(_)
        ));
    }
}
