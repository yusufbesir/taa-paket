//! Gönderici arayüzü. Bu crate ağa ÇIKMAZ: gerçek Telegram/e-posta göndericisi bu trait'i ayrı bir katmanda uygular
//! (bot anahtarını `Ayar::bot_anahtari()` ile alır). Test için `SahteGonderici` vardır.
//! Sahip: Yusuf. yazan: [Claude#ihbar 2026-11-26]

use crate::ayar::Ayar;
use crate::bicim::Bildirim;
use crate::defter::Teslim;
use std::sync::Mutex;

pub trait Gonderici {
    /// `sohbet`: ayardaki hedef sohbet (Yusuf'un hesabı). Başarısızlık metni sır içermemelidir.
    fn telegram_gonder(&self, sohbet: &str, metin: &str) -> Result<(), String>;
    fn eposta_gonder(&self, kime: &str, konu: &str, govde: &str) -> Result<(), String>;
}

/// Hata metnindeki sırları siler ve kısaltır (defterde saklanır).
pub(crate) fn hata_temizle(h: &str, sirlar: &[&str]) -> String {
    let mut s = h.to_string();
    for sir in sirlar.iter().filter(|x| !x.is_empty()) {
        s = s.replace(sir, "[gizli]");
    }
    let s: String = s.chars().filter(|c| !c.is_control()).take(200).collect();
    s
}

/// Her kanalı ayara göre dener; biri başarısız olsa da öbürü denenir. Kanal ayarda yoksa `Kapali`.
pub fn teslim_et(g: &dyn Gonderici, ayar: &Ayar, b: &Bildirim) -> (Teslim, Teslim) {
    let sirlar = [
        ayar.bot_anahtari(),
        ayar.telegram_sohbet(),
        ayar.eposta_alici(),
    ];
    let telegram = if ayar.telegram_acik() {
        match g.telegram_gonder(ayar.telegram_sohbet(), &b.telegram) {
            Ok(()) => Teslim::Gitti,
            Err(e) => Teslim::Hata(hata_temizle(&e, &sirlar)),
        }
    } else {
        Teslim::Kapali
    };
    let eposta = if ayar.eposta_acik() {
        match g.eposta_gonder(ayar.eposta_alici(), &b.eposta_konu, &b.eposta_govde) {
            Ok(()) => Teslim::Gitti,
            Err(e) => Teslim::Hata(hata_temizle(&e, &sirlar)),
        }
    } else {
        Teslim::Kapali
    };
    (telegram, eposta)
}

/// Sahte gönderici: gönderilenleri bellekte tutar; istenirse hata döner. Ağ yok.
#[derive(Default)]
pub struct SahteGonderici {
    pub telegram_hata: Option<String>,
    pub eposta_hata: Option<String>,
    telegramlar: Mutex<Vec<(String, String)>>,
    epostalar: Mutex<Vec<(String, String, String)>>,
}

impl SahteGonderici {
    pub fn telegram_hatali(h: &str) -> Self {
        SahteGonderici {
            telegram_hata: Some(h.into()),
            ..Default::default()
        }
    }
    pub fn eposta_hatali(h: &str) -> Self {
        SahteGonderici {
            eposta_hata: Some(h.into()),
            ..Default::default()
        }
    }
    pub fn telegramlar(&self) -> Vec<(String, String)> {
        self.telegramlar.lock().unwrap().clone()
    }
    pub fn epostalar(&self) -> Vec<(String, String, String)> {
        self.epostalar.lock().unwrap().clone()
    }
}

impl Gonderici for SahteGonderici {
    fn telegram_gonder(&self, sohbet: &str, metin: &str) -> Result<(), String> {
        if let Some(h) = &self.telegram_hata {
            return Err(h.clone());
        }
        self.telegramlar
            .lock()
            .unwrap()
            .push((sohbet.into(), metin.into()));
        Ok(())
    }
    fn eposta_gonder(&self, kime: &str, konu: &str, govde: &str) -> Result<(), String> {
        if let Some(h) = &self.eposta_hata {
            return Err(h.clone());
        }
        self.epostalar
            .lock()
            .unwrap()
            .push((kime.into(), konu.into(), govde.into()));
        Ok(())
    }
}
