//! taa-telegram: terzihane iç işlemleri için Telegram botu çekirdeği.
//! Bot yalnız izinli Telegram kullanıcılarına yanıt verir (`telegram.json` içindeki liste, anahtar bilgisi repoda durmaz).
//! Ağ kodu YOK: Telegram'ın webhook çağrısına yanıt gövdesi olarak `sendMessage` JSON'u üretir; dışarı istek atmaz.
//! Müşteri adı/telefonu bot mesajlarında yalnız MASKELİ görünür (ad: ilk/son harf; telefon: ilk 4 ve son 2 hane).
//! Özellik anahtarı `telegram-bot`: varsayılan KAPALI. Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-05].

use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use taa_fis::{Defter, Durum};

const OZELLIK: &str = "telegram-bot";

#[derive(Debug, Deserialize, Default)]
pub struct Ayar {
    /// Webhook adresindeki gizli parça: `/telegram/<gizli>`.
    #[serde(default)]
    pub gizli: String,
    /// Telegram kullanıcı kimliği -> görünen ad (imzada bu ad yazılır).
    #[serde(default)]
    pub kullanicilar: BTreeMap<String, String>,
}

impl Ayar {
    pub fn dizinden(dizin: &Path) -> Result<Ayar, String> {
        let s = std::fs::read_to_string(dizin.join("telegram.json"))
            .map_err(|_| "telegram.json yok (bot ayarlanmamış)".to_string())?;
        let a: Ayar =
            serde_json::from_str(&s).map_err(|e| format!("telegram.json geçersiz: {e}"))?;
        if a.gizli.len() < 16 {
            return Err("telegram.json: gizli en az 16 karakter olmalı".into());
        }
        Ok(a)
    }

    /// Sabit zamanlı karşılaştırma.
    pub fn gizli_dogru(&self, g: &str) -> bool {
        self.gizli.len() == g.len()
            && self
                .gizli
                .bytes()
                .zip(g.bytes())
                .fold(0u8, |d, (a, b)| d | (a ^ b))
                == 0
    }
}

pub struct Bot<'a> {
    pub dizin: &'a Path,
    pub ayar: &'a Ayar,
    /// Bugünün tarihi (ISO).
    pub bugun: &'a str,
}

/// "Ayşe Çelik" -> "A**e Ç***k": her sözcüğün ilk ve son harfi görünür, ortası yıldız.
pub fn maske_ad(ad: &str) -> String {
    ad.split_whitespace()
        .map(|w| {
            let c: Vec<char> = w.chars().collect();
            match c.len() {
                0 => String::new(),
                1 => "*".into(),
                2 => format!("{}*", c[0]),
                n => format!("{}{}{}", c[0], "*".repeat(n - 2), c[n - 1]),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// "05321234567" -> "0532*****67": ilk hane + alan kodu ve son 2 hane görünür.
pub fn maske_tel(t: &str) -> String {
    let c: Vec<char> = t.chars().filter(char::is_ascii_digit).collect();
    if c.len() < 8 {
        return "*".repeat(c.len());
    }
    let (bas, son) = (4, 2);
    format!(
        "{}{}{}",
        c[..bas].iter().collect::<String>(),
        "*".repeat(c.len() - bas - son),
        c[c.len() - son..].iter().collect::<String>()
    )
}

const YARDIM: &str = "Komutlar:\n/durum FİŞNO\n/terzihanede FİŞNO\n/hazir FİŞNO\n/raf FİŞNO A-12\n/imza FİŞNO\n/bugun (bugün teslim)\n/geciken\n/kargo (kargo bekleyenler)\n/kuru (kuru temizleme)\n/ozet\n/yardim";

impl Bot<'_> {
    /// Telegram güncellemesine webhook yanıtı üretir; yanıtlanacak bir şey yoksa `None`.
    pub fn yanit(&self, guncelleme: &Value) -> Option<Value> {
        if !taa_ayar::ozellik_acik(OZELLIK, false) {
            return None;
        }
        let m = guncelleme.get("message")?;
        let sohbet = m["chat"]["id"].as_i64()?;
        let metin = m["text"].as_str()?;
        let kim = m["from"]["id"].as_i64()?;
        let cevap = match self.ayar.kullanicilar.get(&kim.to_string()) {
            Some(ad) => self.komut(metin, ad),
            None => format!(
                "Bu botu kullanma izniniz yok. Telegram kimliğiniz: {kim}. Yönetici sizi telegram.json listesine eklemeli."
            ),
        };
        let cevap: String = cevap.chars().take(3500).collect();
        Some(json!({"method": "sendMessage", "chat_id": sohbet, "text": cevap}))
    }

    fn komut(&self, metin: &str, ad: &str) -> String {
        let mut p = metin.split_whitespace();
        let k = p
            .next()
            .unwrap_or("")
            .split('@')
            .next()
            .unwrap_or("")
            .to_lowercase();
        let (a1, a2) = (p.next().unwrap_or(""), p.next().unwrap_or(""));
        let yol = self.dizin.join("fisler.jsonl");
        let d = Defter { yol: &yol };
        let r = match k.as_str() {
            "/start" | "/yardim" | "/yardım" => Ok(YARDIM.to_string()),
            "/durum" => d
                .bul(a1)
                .and_then(|f| f.ok_or_else(|| format!("fiş yok: {a1}")))
                .map(|f| {
                    format!(
                        "{}: {:?}\nMüşteri: {} Tel: {}\n{}\nTeslim: {} {}\nRaf: {}\nİmza: {}",
                        f.no,
                        f.durum,
                        maske_ad(&f.musteri),
                        maske_tel(&f.telefon),
                        f.kalemler
                            .iter()
                            .map(|k| format!("{} - {}", k.urun, k.islem))
                            .collect::<Vec<_>>()
                            .join("; "),
                        f.teslim,
                        f.teslim_saati,
                        if f.raf.is_empty() { "—" } else { &f.raf },
                        if f.imzalar.is_empty() {
                            "—".to_string()
                        } else {
                            f.imzalar
                                .iter()
                                .map(|i| format!("{} ({})", i.kim, i.rol))
                                .collect::<Vec<_>>()
                                .join(", ")
                        },
                    )
                }),
            "/bugun" | "/bugün" => d.fisler().map(|v| {
                let l: Vec<String> = v
                    .iter()
                    .filter(|f| {
                        f.teslim == self.bugun
                            && matches!(
                                f.durum,
                                Durum::Bekliyor | Durum::Terzihanede | Durum::Hazir
                            )
                    })
                    .map(|f| {
                        format!(
                            "{} {:?} {} {}",
                            f.no,
                            f.durum,
                            f.teslim_saati,
                            maske_ad(&f.musteri)
                        )
                    })
                    .collect();
                if l.is_empty() {
                    "Bugün teslim edilecek fiş yok.".into()
                } else {
                    l.join("\n")
                }
            }),
            "/geciken" => d.geciken(self.bugun).map(|v| {
                if v.is_empty() {
                    "Geciken fiş yok.".into()
                } else {
                    v.iter()
                        .map(|f| format!("{} teslim {}", f.no, f.teslim))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            }),
            "/kargo" | "/kuru" | "/ozet" => taa_takip::Takip::defterden(&d).map(|t| match k.as_str() {
                "/ozet" => {
                    let o = t.ozet(self.bugun);
                    format!(
                        "Bekliyor {} | Terzihanede {} | Hazır {}\nGeciken {} | Bugün teslim {}\nKargo bekleyen {} | Kuru temizleme {}",
                        o.bekliyor, o.terzihanede, o.hazir, o.geciken, o.bugun_teslim, o.kargo_bekleyen, o.kuru_temizleme
                    )
                }
                ad => {
                    let v = if ad == "/kargo" { t.kargo_bekleyen() } else { t.kuru_temizleme() };
                    if v.is_empty() {
                        "Liste boş.".into()
                    } else {
                        // KVKK: bot mesajında yalnız maskeli ad; kargo adresi/telefonu gösterilmez
                        v.iter()
                            .map(|f| format!("{} {} teslim {}", f.no, maske_ad(&f.musteri), f.teslim))
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                }
            }),
            "/terzihanede" => d
                .durum_ata(a1, Durum::Terzihanede, self.bugun)
                .map(|_| format!("{a1}: terzihanede")),
            "/hazir" | "/hazır" => self.hazir(&d, a1),
            "/raf" => d
                .raf_ata(a1, a2, ad, self.bugun)
                .map(|_| format!("{a1}: raf {}", a2.to_uppercase())),
            "/imza" => d
                .imzala(a1, ad, "usta", self.bugun)
                .map(|_| format!("{a1}: {ad} imzaladı")),
            _ => Ok(format!("Anlamadım. {YARDIM}")),
        };
        r.unwrap_or_else(|e| format!("Hata: {e}"))
    }

    fn hazir(&self, d: &Defter, no: &str) -> Result<String, String> {
        let f = d.bul(no)?.ok_or_else(|| format!("fiş yok: {no}"))?;
        if f.durum == Durum::Bekliyor {
            d.durum_ata(no, Durum::Terzihanede, self.bugun)?;
        }
        d.durum_ata(no, Durum::Hazir, self.bugun)?;
        let b = taa_bildirim::hazir_bildir(self.dizin, no, &f.telefon, f.bildirim_izni, self.bugun);
        Ok(format!("{no}: hazır. Müşteri bildirimi: {b}"))
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use taa_fis::Girdi;

    fn kur() -> (std::path::PathBuf, String) {
        let d = taa_dosya::gecici_klasor().unwrap();
        let y = d.join("fisler.jsonl");
        let g: Girdi = serde_json::from_str(
            r#"{"satis_danismani":"Mert","olcu_alan":"K","musteri":"Ayşe Çelik","telefon":"05321234567","kalemler":[{"urun":"Pantolon","islem":"paça kısalt"}],"teslim":"2099-01-01","ucret_kurus":100,"bildirim_izni":true,"tarih":"2026-11-05"}"#,
        )
        .unwrap();
        let f = Defter { yol: &y }.ekle(g).unwrap();
        (d, f.no)
    }

    fn ayar() -> Ayar {
        Ayar {
            gizli: "0123456789abcdef".into(),
            kullanicilar: [("42".to_string(), "Usta Ahmet".to_string())].into(),
        }
    }

    fn mesaj(kim: i64, metin: &str) -> Value {
        json!({"message": {"chat": {"id": 7}, "from": {"id": kim}, "text": metin}})
    }

    #[test]
    fn akis() {
        std::env::set_var("TAA_OZELLIK_TELEGRAM_BOT", "1");
        let (d, no) = kur();
        let a = ayar();
        let b = Bot {
            dizin: &d,
            ayar: &a,
            bugun: "2026-11-05",
        };
        let y = |kim, m: &str| {
            b.yanit(&mesaj(kim, m)).unwrap()["text"]
                .as_str()
                .unwrap()
                .to_string()
        };
        // yetkisiz
        assert!(y(99, "/durum x").contains("izniniz yok") && y(99, "/durum x").contains("99"));
        // durum: müşteri adı/telefonu görünmez
        let t = y(42, &format!("/durum {no}"));
        assert!(t.contains("paça kısalt") && !t.contains("Ayşe") && !t.contains("1234"));
        assert!(t.contains("A**e Ç***k") && t.contains("0532*****67"), "{t}");
        assert!(y(42, &format!("/raf {no} a-12")).contains("A-12"));
        assert!(y(42, &format!("/imza {no}")).contains("Usta Ahmet"));
        assert!(y(42, &format!("/imza {no}")).starts_with("Hata"));
        let h = y(42, &format!("/hazir {no}"));
        assert!(h.contains("kuyruğa yazıldı"), "{h}");
        assert!(y(42, &format!("/durum {no}")).contains("Hazir"));
        assert!(y(42, "/hazir F-yok").starts_with("Hata"));
        assert!(y(42, "/bugun").contains("yok"));
        // takip komutları: kargo/kuru listesi boş, özet sayıları var, ad maskeli
        assert_eq!(y(42, "/kargo"), "Liste boş.");
        assert_eq!(y(42, "/kuru"), "Liste boş.");
        assert!(y(42, "/ozet").contains("Hazır 1"));
        assert!(y(42, "/xyz").contains("Komutlar"));
        // yanıt biçimi
        let r = b.yanit(&mesaj(42, "/yardim")).unwrap();
        assert_eq!(
            (r["method"].as_str(), r["chat_id"].as_i64()),
            (Some("sendMessage"), Some(7))
        );
        // metin olmayan güncelleme yanıtsız
        assert!(b.yanit(&json!({"edited_message": {}})).is_none());
    }

    #[test]
    fn maskeler() {
        assert_eq!(maske_ad("Ali Öz"), "A*i Ö*");
        assert_eq!(maske_ad("Ece"), "E*e");
        assert_eq!(maske_tel("0532 123 45 67"), "0532*****67");
        assert_eq!(maske_tel("+90 532 123 45 67"), "9053******67");
        assert_eq!(maske_tel("12"), "**");
    }

    #[test]
    fn ayar_dogrulama() {
        let d = taa_dosya::gecici_klasor().unwrap();
        assert!(Ayar::dizinden(&d).is_err());
        std::fs::write(d.join("telegram.json"), r#"{"gizli":"kisa"}"#).unwrap();
        assert!(Ayar::dizinden(&d).is_err());
        std::fs::write(
            d.join("telegram.json"),
            r#"{"gizli":"0123456789abcdef","kullanicilar":{"1":"A"}}"#,
        )
        .unwrap();
        let a = Ayar::dizinden(&d).unwrap();
        assert!(
            a.gizli_dogru("0123456789abcdef")
                && !a.gizli_dogru("0123456789abcdee")
                && !a.gizli_dogru("x")
        );
    }
}
