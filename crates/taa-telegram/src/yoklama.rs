//! Uzun yoklama (getUpdates) döngüsü: sunucu Telegram'a kendisi sorar, dışarıdan kapı açılmaz (alan adı yok).
//! Ağ işi `Aktarim` arayüzünün ardındadır; bu dosyadaki mantık sahte aktarımla test edilir.
//! Gerçek aktarım `CurlAktarim`: yeni bağımlılık YOK, `curl` kullanılır (taa-kuyruk ile aynı yol).
//! Bot anahtarı komut satırına/günlüğe/hata metnine yazılmaz (curl yapılandırması stdin'den verilir).
//! Gerçek Telegram'da DENENMEDİ. Sahip: Yusuf. yazan: [Claude Sonnet 5.5, TT başlığı 2026-11-15].

use crate::Bot;
use serde_json::{json, Value};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub trait Aktarim {
    /// Telegram Bot API yöntemini çağırır; `result` alanını döndürür.
    fn cagir(&self, yontem: &str, govde: &Value, bekle_sn: u32) -> Result<Value, String>;
}

pub struct Yoklayici<'a, A: Aktarim> {
    pub aktarim: &'a A,
    /// Bir sonraki getUpdates ofseti. Her turdan sonra `telegram-ofset.txt`e yazılır (K4): yeniden başlatmada
    /// aynı güncelleme iki kez işlenmez. Tekrar engeli yalnız buna dayanmaz; kalem imzası (fiş, kalem, tur) tektir.
    pub ofset: i64,
    pub dizin: PathBuf,
}

/// Kasa bilgisayarlarında değil, YALNIZ sunucuda ve TEK süreçte yoklama yapılır (K4). Kilit dosyası 120 sn içinde
/// yenilenmezse bayat sayılır (çöken süreç yeni sürecin önünü kesmez).
pub fn kilit_al(dizin: &Path, simdi: u64) -> Result<(), String> {
    let yol = dizin.join("telegram-yoklama.kilit");
    if let Ok(s) = std::fs::read_to_string(&yol) {
        if let Ok(t) = s.trim().parse::<u64>() {
            if simdi < t + 120 {
                return Err("yoklama başka bir süreçte çalışıyor".into());
            }
        }
    }
    std::fs::write(&yol, simdi.to_string()).map_err(|e| e.to_string())
}

impl<'a, A: Aktarim> Yoklayici<'a, A> {
    /// Kalıcı ofseti okuyarak kurar.
    pub fn kur(aktarim: &'a A, dizin: &Path) -> Self {
        let ofset = std::fs::read_to_string(dizin.join("telegram-ofset.txt"))
            .ok()
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0);
        Yoklayici {
            aktarim,
            ofset,
            dizin: dizin.to_path_buf(),
        }
    }
}

/// Hata metninden bot anahtarını çıkarır (günlüğe anahtar düşmez, K3).
pub fn arindir(metin: &str, anahtar: &str) -> String {
    if anahtar.is_empty() {
        metin.to_string()
    } else {
        metin.replace(anahtar, "<anahtar>")
    }
}

/// curl'e verilen komut satırı bağımsız değişkenleri: anahtar ve adres BURADA YOK (standart girdiden yapılandırma, `-K -`).
pub fn curl_argumanlari() -> [&'static str; 2] {
    ["-K", "-"]
}

impl<A: Aktarim> Yoklayici<'_, A> {
    /// Bir tur: güncellemeleri alır, yanıtları gönderir. (işlenen güncelleme, gönderim hatası) döner.
    pub fn tur(&mut self, bot: &Bot, bekle_sn: u32) -> Result<(usize, usize), String> {
        let r = self.aktarim.cagir(
            "getUpdates",
            &json!({"offset": self.ofset, "timeout": bekle_sn, "allowed_updates": ["message", "callback_query"]}),
            bekle_sn + 10,
        )?;
        let (mut islenen, mut hata) = (0, 0);
        for u in r.as_array().cloned().unwrap_or_default() {
            for mut eylem in bot.eylemler(&u) {
                let Some(yontem) = eylem["method"].as_str().map(str::to_string) else {
                    continue;
                };
                eylem.as_object_mut().map(|o| o.remove("method"));
                if self.aktarim.cagir(&yontem, &eylem, 15).is_err() {
                    hata += 1;
                }
            }
            if let Some(id) = u["update_id"].as_i64() {
                self.ofset = self.ofset.max(id + 1);
                let _ = std::fs::write(
                    self.dizin.join("telegram-ofset.txt"),
                    self.ofset.to_string(),
                );
            }
            islenen += 1;
        }
        Ok((islenen, hata))
    }
}

/// curl yapılandırma satırı için kaçış (`\` ve `"`).
fn kacis(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// curl yapılandırması (stdin'e verilir; anahtar argv'de görünmez).
pub fn curl_yapilandirma(anahtar: &str, yontem: &str, govde: &Value, sure: u32) -> String {
    format!(
        "url = \"https://api.telegram.org/bot{}/{}\"\nsilent\nshow-error\nmax-time = {}\nheader = \"Content-Type: application/json\"\ndata = \"{}\"\n",
        kacis(anahtar),
        kacis(yontem),
        sure,
        kacis(&govde.to_string())
    )
}

pub struct CurlAktarim {
    pub anahtar: String,
}

impl Aktarim for CurlAktarim {
    fn cagir(&self, yontem: &str, govde: &Value, bekle_sn: u32) -> Result<Value, String> {
        let mut c = Command::new("curl")
            .args(curl_argumanlari())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|_| "curl çalıştırılamadı".to_string())?;
        c.stdin
            .take()
            .ok_or("curl girdisi yok")?
            .write_all(curl_yapilandirma(&self.anahtar, yontem, govde, bekle_sn).as_bytes())
            .map_err(|_| "curl girdisi yazılamadı".to_string())?;
        let o = c
            .wait_with_output()
            .map_err(|_| "curl beklenemedi".to_string())?;
        let v: Value = serde_json::from_slice(&o.stdout)
            .map_err(|_| "Telegram yanıtı okunamadı (ağ kesik olabilir)".to_string())?;
        if v["ok"].as_bool() == Some(true) {
            Ok(v["result"].clone())
        } else {
            Err(arindir(
                &format!(
                    "Telegram hatası: {}",
                    v["description"].as_str().unwrap_or("bilinmiyor")
                ),
                &self.anahtar,
            ))
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::cell::RefCell;

    struct Sahte {
        gelen: RefCell<Vec<Value>>,
        giden: RefCell<Vec<(String, Value)>>,
    }
    impl Aktarim for Sahte {
        fn cagir(&self, yontem: &str, govde: &Value, _: u32) -> Result<Value, String> {
            if yontem == "getUpdates" {
                return Ok(Value::Array(self.gelen.borrow_mut().drain(..).collect()));
            }
            self.giden.borrow_mut().push((yontem.into(), govde.clone()));
            Ok(json!({}))
        }
    }

    #[test]
    fn anahtar_komut_satirinda_ve_hatada_yok() {
        let c = curl_yapilandirma("123:ABC", "sendMessage", &json!({"text": "a\"b"}), 15);
        assert!(c.contains("bot123:ABC/sendMessage") && c.contains("a\\\\\\\"b"));
        assert!(c.contains("max-time = 15"));
        // süreç parametrelerinde anahtar aranır (K3)
        assert!(curl_argumanlari()
            .iter()
            .all(|a| !a.contains("123:ABC") && !a.contains("telegram")));
        assert!(!arindir("hata: bot123:ABC/x", "123:ABC").contains("123:ABC"));
    }

    #[test]
    fn tur_ofset_kalici_ve_kilit() {
        std::env::set_var("TAA_OZELLIK_TELEGRAM_BOT", "1");
        let d = taa_dosya::gecici_klasor().unwrap();
        let ayar = crate::Ayar {
            gizli: String::new(),
            kullanicilar: Default::default(),
            bot_adi: String::new(),
            yonlendirme: String::new(),
            hiz_siniri: 20,
        };
        let bot = Bot {
            dizin: &d,
            ayar: &ayar,
            bugun: "2026-11-15",
            simdi: 1_000_000,
        };
        let a = Sahte {
            gelen: RefCell::new(vec![
                json!({"update_id": 41, "message": {"chat": {"id": 7}, "from": {"id": 5}, "text": "/yardim"}}),
            ]),
            giden: RefCell::new(vec![]),
        };
        let mut y = Yoklayici::kur(&a, &d);
        assert_eq!(y.tur(&bot, 1).unwrap(), (1, 0));
        assert_eq!(y.ofset, 42);
        assert_eq!(a.giden.borrow().len(), 1);
        assert!(a.giden.borrow()[0].1.get("method").is_none());
        // yeniden başlatma: ofset dosyadan gelir
        assert_eq!(Yoklayici::kur(&a, &d).ofset, 42);
        // tek süreç kilidi
        assert!(kilit_al(&d, 1000).is_ok());
        assert!(kilit_al(&d, 1050).is_err());
        assert!(kilit_al(&d, 1200).is_ok());
    }
}
