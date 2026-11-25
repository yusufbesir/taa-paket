//! taa-bildirim: müşteriye "ürününüz hazır" mesajı kuyruğu.
//! KVKK: mesajda yalnız fiş numarası ve "hazır" bilgisi olur; müşteri adı/telefonu METNE yazılmaz.
//! Telefon yalnız gönderim adresi olarak yerel kuyrukta durur. Müşteri bilgilendirme izni vermediyse kuyruğa yazılmaz.
//! Gerçek gönderim (WhatsApp Business API) bir `Gonderici` ile yapılır; bu crate gönderim yapmaz, kuyruğu tutar.
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-05].

use serde::{Deserialize, Serialize};
use std::path::Path;

const OZELLIK: &str = "bildirim-whatsapp";
pub const VARSAYILAN_SABLON: &str =
    "Tadilat fişiniz {no} hazır. Ürününüzü fişinizle birlikte teslim alabilirsiniz.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Durum {
    Bekliyor,
    Gonderildi,
    Hata(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mesaj {
    pub id: String,
    pub fis_no: String,
    /// Uluslararası biçim, ör. 905321234567.
    pub telefon: String,
    pub metin: String,
    pub tarih: String,
    pub durum: Durum,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "lowercase")]
enum Kayit {
    Mesaj(Mesaj),
    Sonuc {
        id: String,
        durum: Durum,
        tarih: String,
    },
}

/// Gerçek gönderici (WhatsApp Business API vb.) bu arayüzü uygular.
pub trait Gonderici {
    fn gonder(&self, m: &Mesaj) -> Result<(), String>;
}

/// Şablonda yalnız `{no}` ve `{takip}` (müşteri takip sayfası adresi) yer tutucularına izin verilir
/// (KVKK: başka kişisel veri girmesin).
pub fn sablon_gecerli(s: &str) -> Result<(), String> {
    if !s.contains("{no}") {
        return Err("şablonda {no} (fiş numarası) bulunmalı".into());
    }
    let kalan = s.replace("{no}", "").replace("{takip}", "");
    if let Some(i) = kalan.find('{') {
        let son = kalan[i..]
            .find('}')
            .map(|j| i + j + 1)
            .unwrap_or(kalan.len());
        return Err(format!(
            "şablonda yalnız {{no}} ve {{takip}} kullanılabilir (KVKK); bulunan: {}",
            &kalan[i..son]
        ));
    }
    if s.chars().count() > 300 {
        return Err("şablon 300 karakterden uzun olamaz".into());
    }
    Ok(())
}

pub fn hazir_mesaji(sablon: &str, fis_no: &str) -> Result<String, String> {
    hazir_mesaji_takipli(sablon, fis_no, None)
}

/// Takip adresi verilirse `{takip}` yerine konur; şablonda `{takip}` yoksa sona kısa bir cümleyle eklenir.
/// Adres verilmezse `{takip}` geçen cümle parçası bırakılmaz (yer tutucu boşalır).
pub fn hazir_mesaji_takipli(
    sablon: &str,
    fis_no: &str,
    takip: Option<&str>,
) -> Result<String, String> {
    sablon_gecerli(sablon)?;
    let m = sablon.replace("{no}", fis_no);
    Ok(match takip {
        Some(t) if m.contains("{takip}") => m.replace("{takip}", t),
        Some(t) => format!("{m} Ürününüzün durumunu buradan izleyebilirsiniz: {t}"),
        None => m.replace("{takip}", "").trim().to_string(),
    })
}

/// `05321234567` / `+90 532 123 45 67` -> `905321234567`.
pub fn wa_telefon(t: &str) -> Result<String, String> {
    let r: String = t.chars().filter(char::is_ascii_digit).collect();
    let r = r.strip_prefix("90").map(str::to_string).unwrap_or(r);
    let r = r.strip_prefix('0').map(str::to_string).unwrap_or(r);
    if r.len() == 10 && r.starts_with('5') {
        Ok(format!("90{r}"))
    } else {
        Err("telefon geçerli bir cep numarası değil".into())
    }
}

pub struct Kuyruk<'a> {
    pub yol: &'a Path,
}

impl Kuyruk<'_> {
    fn kayitlar(&self) -> Result<Vec<Kayit>, String> {
        taa_dosya::satirlari_oku(self.yol)
            .map_err(|e| e.to_string())?
            .iter()
            .map(|l| serde_json::from_str(l).map_err(|e| format!("kuyruk satırı bozuk: {e}")))
            .collect()
    }

    /// Mesajlar, güncel durumlarıyla.
    pub fn mesajlar(&self) -> Result<Vec<Mesaj>, String> {
        let mut m: Vec<Mesaj> = Vec::new();
        for k in self.kayitlar()? {
            match k {
                Kayit::Mesaj(x) => m.push(x),
                Kayit::Sonuc { id, durum, .. } => {
                    if let Some(x) = m.iter_mut().find(|x| x.id == id) {
                        x.durum = durum;
                    }
                }
            }
        }
        Ok(m)
    }

    pub fn bekleyenler(&self) -> Result<Vec<Mesaj>, String> {
        Ok(self
            .mesajlar()?
            .into_iter()
            .filter(|m| m.durum == Durum::Bekliyor)
            .collect())
    }

    /// Hazır mesajını kuyruğa yazar. Aynı fiş için ikinci kez yazılmaz (`Ok(None)`).
    pub fn hazir_ekle(
        &self,
        fis_no: &str,
        telefon: &str,
        sablon: &str,
        tarih: &str,
    ) -> Result<Option<Mesaj>, String> {
        self.hazir_ekle_takipli(fis_no, telefon, sablon, tarih, None)
    }

    pub fn hazir_ekle_takipli(
        &self,
        fis_no: &str,
        telefon: &str,
        sablon: &str,
        tarih: &str,
        takip: Option<&str>,
    ) -> Result<Option<Mesaj>, String> {
        if !taa_ayar::ozellik_acik(OZELLIK, true) {
            return Err(taa_ayar::kapali_mesaji(OZELLIK));
        }
        if self.mesajlar()?.iter().any(|m| m.fis_no == fis_no) {
            return Ok(None);
        }
        let m = Mesaj {
            id: format!("M-{}", self.mesajlar()?.len() + 1),
            fis_no: fis_no.into(),
            telefon: wa_telefon(telefon)?,
            metin: hazir_mesaji_takipli(sablon, fis_no, takip)?,
            tarih: tarih.into(),
            durum: Durum::Bekliyor,
        };
        let s = serde_json::to_string(&Kayit::Mesaj(m.clone())).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())?;
        Ok(Some(m))
    }

    pub fn sonuc(&self, id: &str, durum: Durum, tarih: &str) -> Result<(), String> {
        let s = serde_json::to_string(&Kayit::Sonuc {
            id: id.into(),
            durum,
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(self.yol, &s).map_err(|e| e.to_string())
    }

    /// Bekleyenleri `g` ile gönderir; sonucu deftere yazar. (gönderilen, hata) sayısı döner.
    pub fn isle(&self, g: &dyn Gonderici, tarih: &str) -> Result<(usize, usize), String> {
        let (mut ok, mut hata) = (0, 0);
        for m in self.bekleyenler()? {
            match g.gonder(&m) {
                Ok(()) => {
                    self.sonuc(&m.id, Durum::Gonderildi, tarih)?;
                    ok += 1;
                }
                Err(e) => {
                    self.sonuc(&m.id, Durum::Hata(e), tarih)?;
                    hata += 1;
                }
            }
        }
        Ok((ok, hata))
    }
}

/// Fiş "hazır" olunca: izin varsa mesajı kuyruğa yazar; sonucu kısa metin olarak döndürür.
/// Şablon `dizin/bildirim-sablonu.txt` varsa o, yoksa varsayılandır; kuyruk `dizin/bildirimler.jsonl`.
pub fn hazir_bildir(dizin: &Path, fis_no: &str, telefon: &str, izin: bool, tarih: &str) -> String {
    hazir_bildir_takipli(dizin, fis_no, telefon, izin, tarih, None)
}

/// Aynısı; `takip` müşteri takip sayfasının adresidir (mesaja eklenir).
pub fn hazir_bildir_takipli(
    dizin: &Path,
    fis_no: &str,
    telefon: &str,
    izin: bool,
    tarih: &str,
    takip: Option<&str>,
) -> String {
    if !izin {
        return "müşteri bilgilendirme izni vermemiş".into();
    }
    let sablon = std::fs::read_to_string(dizin.join("bildirim-sablonu.txt"))
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| VARSAYILAN_SABLON.to_string());
    let yol = dizin.join("bildirimler.jsonl");
    match (Kuyruk { yol: &yol }).hazir_ekle_takipli(fis_no, telefon, &sablon, tarih, takip) {
        Ok(Some(m)) => format!("hazır mesajı kuyruğa yazıldı ({})", m.id),
        Ok(None) => "hazır mesajı zaten kuyrukta".into(),
        Err(e) => e,
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    struct Sahte(bool);
    impl Gonderici for Sahte {
        fn gonder(&self, _: &Mesaj) -> Result<(), String> {
            if self.0 {
                Ok(())
            } else {
                Err("ağ yok".into())
            }
        }
    }

    #[test]
    fn sablon_kvkk() {
        assert!(sablon_gecerli(VARSAYILAN_SABLON).is_ok());
        assert!(sablon_gecerli("Merhaba {ad}, {no} hazır").is_err());
        assert!(sablon_gecerli("{telefon} {no}").is_err());
        assert!(sablon_gecerli("fişsiz").is_err());
        let m = hazir_mesaji(VARSAYILAN_SABLON, "F-1").unwrap();
        assert!(m.contains("F-1") && !m.contains('{'));
    }

    #[test]
    fn telefon_biçimi() {
        assert_eq!(wa_telefon("0532 123 45 67").unwrap(), "905321234567");
        assert_eq!(wa_telefon("+90 532 123 45 67").unwrap(), "905321234567");
        assert!(wa_telefon("0212 123 45 67").is_err());
        assert!(wa_telefon("123").is_err());
    }

    #[test]
    fn kuyruk_akisi() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let y = d.join("bildirim.jsonl");
        let k = Kuyruk { yol: &y };
        let m = k
            .hazir_ekle("F-1", "05321234567", VARSAYILAN_SABLON, "2026-11-05")
            .unwrap()
            .unwrap();
        assert!(!m.metin.contains("Ayşe") && m.telefon == "905321234567");
        // aynı fiş ikinci kez yazılmaz
        assert!(k
            .hazir_ekle("F-1", "05321234567", VARSAYILAN_SABLON, "2026-11-05")
            .unwrap()
            .is_none());
        assert_eq!(k.bekleyenler().unwrap().len(), 1);
        assert_eq!(k.isle(&Sahte(false), "2026-11-05").unwrap(), (0, 1));
        assert!(matches!(k.mesajlar().unwrap()[0].durum, Durum::Hata(_)));
        // hatalı mesaj yeniden denenmez (elle yeniden kuyruğa alınmalı)
        assert_eq!(k.bekleyenler().unwrap().len(), 0);
        k.hazir_ekle("F-2", "05321234568", VARSAYILAN_SABLON, "2026-11-05")
            .unwrap();
        assert_eq!(k.isle(&Sahte(true), "2026-11-05").unwrap(), (1, 0));
        assert_eq!(k.mesajlar().unwrap()[1].durum, Durum::Gonderildi);
    }

    #[test]
    fn takip_adresi_mesaja_eklenir() {
        let t = "https://takip.example/t/abcdefghjkmnpqrs";
        let m = hazir_mesaji_takipli(VARSAYILAN_SABLON, "F-1", Some(t)).unwrap();
        assert!(
            m.starts_with("Tadilat fişiniz F-1 hazır.") && m.ends_with(t),
            "{m}"
        );
        let m = hazir_mesaji_takipli("Fiş {no} hazır. Durum: {takip}", "F-2", Some(t)).unwrap();
        assert_eq!(m, format!("Fiş F-2 hazır. Durum: {t}"));
        assert_eq!(
            hazir_mesaji_takipli("Fiş {no} hazır. {takip}", "F-3", None).unwrap(),
            "Fiş F-3 hazır."
        );
        assert!(sablon_gecerli("Fiş {no} {ad}").is_err());
    }
}
