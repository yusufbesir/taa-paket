// Sahip: Yusuf. yazan: [Claude#egitim-panel 2026-11-26]
//! Panelin kullanıcı bilgisi. Kaynak `taa-kimlik` oturumu ve yetki belgesidir (panel_main.rs okur); burada yalnız
//! saf karar vardır. Yönetici işlemleri varsayılan REDDET: aşağıdakilerin HEPSİ gerekir, biri eksikse yönetici değil.
//!  1. Yetki belgesi okunabilmiş ve süresi geçmemiş.
//!  2. Belge aynı kullanıcıya ait (başka kullanıcının eski belgesi sayılmaz).
//!  3. Belgenin imzası sunucu genel anahtarıyla doğrulanmış (`dogrulandi`); anahtar kurulu değilse yönetici yok.
//!  4. Belgenin `izinler` listesinde `kimlik.yonetici` var (rol adı tek başına izin vermez).

use taa_rehber_model::model::kimlik_gecerli;

/// Yönetici işlemleri (video atama, izleme özeti) için gereken izin adı (`kimlik.yonetici@1` yeteneği).
pub const YONETICI_IZNI: &str = "kimlik.yonetici";

/// Saklı yetki belgesinden panelin ihtiyacı olan alanlar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YetkiBilgisi {
    pub kullanici: String,
    pub izinler: Vec<String>,
    /// İmza sunucu genel anahtarıyla doğrulandı mı.
    pub dogrulandi: bool,
    /// ISO bitiş zamanı; boşsa süresiz sayılmaz (reddedilir).
    pub gecerlilik: String,
}

/// Oturum açmış kullanıcı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Kullanici {
    /// Giriş adı (oturumdan).
    pub ad: String,
    pub yonetici: bool,
}

impl Kullanici {
    /// Rehberdeki kişi kimliği. Eşleştirme kuralı: giriş adı küçük harfe çevrilip rehber kimliği biçimine
    /// (`[a-z0-9][a-z0-9_-]{0,63}`) uyuyorsa o kimliktir; uymuyorsa kişi eşleşmez (None). AÇIK: giriş adı ile rehber
    /// kimliği arasındaki gerçek eşleme sözleşmesi henüz yok (BENIOKU.md).
    pub fn kisi_id(&self) -> Option<String> {
        let k = self.ad.to_lowercase();
        kimlik_gecerli(&k).then_some(k)
    }
}

/// Oturum ve (varsa) yetki belgesinden kullanıcı kurar. `simdi` ISO.
pub fn kullanici_olustur(
    oturum_kullanici: &str,
    yetki: Option<&YetkiBilgisi>,
    simdi: &str,
) -> Kullanici {
    let yonetici = yetki.is_some_and(|y| {
        y.kullanici == oturum_kullanici
            && y.dogrulandi
            && !y.gecerlilik.is_empty()
            && simdi < y.gecerlilik.as_str()
            && y.izinler.iter().any(|i| i == YONETICI_IZNI)
    });
    Kullanici {
        ad: oturum_kullanici.to_string(),
        yonetici,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    const SIMDI: &str = "2026-11-26T10:00:00Z";

    fn yetki() -> YetkiBilgisi {
        YetkiBilgisi {
            kullanici: "ayse".into(),
            izinler: vec![YONETICI_IZNI.into(), "egitim.izle".into()],
            dogrulandi: true,
            gecerlilik: "2026-11-29T10:00:00Z".into(),
        }
    }

    #[test]
    fn tam_yetki_yonetici_yapar() {
        assert!(kullanici_olustur("ayse", Some(&yetki()), SIMDI).yonetici);
    }

    #[test]
    fn belge_yoksa_yonetici_degil() {
        assert!(!kullanici_olustur("ayse", None, SIMDI).yonetici);
    }

    #[test]
    fn imza_dogrulanmadiysa_yonetici_degil() {
        let mut y = yetki();
        y.dogrulandi = false;
        assert!(!kullanici_olustur("ayse", Some(&y), SIMDI).yonetici);
    }

    #[test]
    fn baska_kullanicinin_belgesi_sayilmaz() {
        assert!(!kullanici_olustur("mehmet", Some(&yetki()), SIMDI).yonetici);
    }

    #[test]
    fn suresi_gecen_ya_da_bos_belge_reddedilir() {
        let mut y = yetki();
        y.gecerlilik = "2026-11-26T10:00:00Z".into();
        assert!(!kullanici_olustur("ayse", Some(&y), SIMDI).yonetici);
        y.gecerlilik = String::new();
        assert!(!kullanici_olustur("ayse", Some(&y), SIMDI).yonetici);
    }

    #[test]
    fn izin_yoksa_rol_adi_yetmez() {
        let mut y = yetki();
        y.izinler = vec!["yonetici".into(), "her.sey".into()];
        assert!(!kullanici_olustur("ayse", Some(&y), SIMDI).yonetici);
    }

    #[test]
    fn kisi_kimligi_eslesmesi() {
        let k = |a: &str| Kullanici {
            ad: a.into(),
            yonetici: false,
        };
        assert_eq!(k("Ayse").kisi_id().as_deref(), Some("ayse"));
        assert_eq!(k("ayse.yilmaz").kisi_id(), None);
        assert_eq!(k("").kisi_id(), None);
        assert_eq!(k("a b").kisi_id(), None);
    }
}
