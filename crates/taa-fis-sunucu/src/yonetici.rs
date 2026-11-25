//! Yönetici kilidi: Telefonlar, Personel ve Usta puanı ekranları yönetici PIN'i ister.
//! PIN bu sunucuda saklanmaz ve yazılmaz; TAA Çatı doğrular (bkz. `cati_pin`). Doğrulanınca rastgele bir
//! oturum belirteci verilir (yalnız SHA256 özeti bellekte tutulur); 10 dakika işlem olmazsa oturum kapanır.
//! Tüm yönetim uçları ayrıca yalnız bu bilgisayardan çalışır.

use super::*;
use cati_pin::PinSonuc;

impl Uygulama {
    /// Belirteç geçerliyse son işlem zamanını yeniler.
    fn yonetici_gecerli(&self, i: &Istek) -> bool {
        let Some(t) = i.yonetici.as_deref().filter(|t| !t.is_empty()) else {
            return false;
        };
        let h = cihaz::sha256_hex(t);
        let simdi = self.simdi();
        let Ok(mut v) = self.yonetici_oturum.lock() else {
            return false;
        };
        v.retain(|(_, son)| simdi.saturating_sub(*son) < YONETICI_BOSTA_SN);
        match v.iter_mut().find(|(o, _)| *o == h) {
            Some(o) => {
                o.1 = simdi;
                true
            }
            None => false,
        }
    }

    /// Yönetim uçlarının kapısı: yalnız bu bilgisayar ve (kilit açıksa) geçerli yönetici oturumu.
    pub(crate) fn yonetim_izni(&self, i: &Istek) -> Option<Yanit> {
        if !i.yerel {
            return Some(Yanit::hata(403, "yalnız bu bilgisayardan"));
        }
        if !self.yonetici_kilidi || self.yonetici_gecerli(i) {
            return None;
        }
        Some(Yanit::json(
            401,
            json!({ "hata": "yonetici_gerekli", "ileti": "Yönetici girişi gerekli" }),
        ))
    }

    /// POST /api/yonetici/giris {pin}: PIN TAA Çatı'ya gider; yanıt olduğu gibi ekrana yansır.
    pub(crate) fn yonetici_giris(&self, i: &Istek) -> Yanit {
        if !i.yerel || !i.masaustu {
            return Yanit::hata(403, "yalnız bu bilgisayardaki programdan");
        }
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let pin = v["pin"].as_str().unwrap_or("");
        if !(4..=8).contains(&pin.len()) || !pin.bytes().all(|b| b.is_ascii_digit()) {
            return Yanit::hata(400, "PIN 4-8 rakam olmalı");
        }
        match self.pin.dogrula(pin) {
            PinSonuc::Gecerli => {
                let t = match cihaz::yeni_token() {
                    Ok(t) => t,
                    Err(e) => return Yanit::hata(500, &e),
                };
                let simdi = self.simdi();
                if let Ok(mut o) = self.yonetici_oturum.lock() {
                    o.retain(|(_, son)| simdi.saturating_sub(*son) < YONETICI_BOSTA_SN);
                    o.push((cihaz::sha256_hex(&t), simdi));
                }
                Yanit::json(
                    200,
                    json!({ "gecerli": true, "belirlendi": true, "belirtec": t, "bosta_sn": YONETICI_BOSTA_SN }),
                )
            }
            PinSonuc::Yanlis => Yanit::json(
                200,
                json!({ "gecerli": false, "belirlendi": true, "ileti": "PIN yanlış" }),
            ),
            PinSonuc::Belirlenmemis => Yanit::json(
                200,
                json!({ "gecerli": false, "belirlendi": false, "ileti": cati_pin::BELIRLENMEMIS }),
            ),
            PinSonuc::Kilitli { ileti, kalan_sn } => Yanit::json(
                423,
                json!({ "hata": "kilitli", "ileti": ileti, "kalan_sn": kalan_sn }),
            ),
            PinSonuc::Ulasilamadi(ayrinti) => Yanit::json(
                503,
                json!({ "hata": "cati_ulasilamiyor", "ileti": cati_pin::ULASILAMIYOR, "ayrinti": ayrinti }),
            ),
        }
    }

    /// POST /api/yonetici/cikis: oturumu kapatır.
    pub(crate) fn yonetici_cikis(&self, i: &Istek) -> Yanit {
        if let Some(t) = i.yonetici.as_deref() {
            let h = cihaz::sha256_hex(t);
            if let Ok(mut o) = self.yonetici_oturum.lock() {
                o.retain(|(x, _)| *x != h);
            }
        }
        Yanit::json(200, json!({ "tamam": true }))
    }

    /// GET /api/yonetici/durum: oturum açık mı? (süreyi yeniler)
    pub(crate) fn yonetici_durum(&self, i: &Istek) -> Yanit {
        if !i.yerel {
            return Yanit::hata(403, "yalnız bu bilgisayardan");
        }
        Yanit::json(
            200,
            json!({
                "acik": !self.yonetici_kilidi || self.yonetici_gecerli(i),
                "kilit": self.yonetici_kilidi,
                "bosta_sn": YONETICI_BOSTA_SN,
            }),
        )
    }

    /// GET /api/yonetim/eslestirme?kisi=K-…: tek kullanımlık 6 haneli kod + karekod. `kisi` verilirse telefon
    /// o kişinin kişisel telefonu olur (kullanıcı girişi: fişi düzenleyen her zaman o kişi).
    pub(crate) fn eslestirme_kodu(&self, i: &Istek, sorgu: &str) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        if let Some(k) = kapali("telefon-eslestirme") {
            return k;
        }
        let kisi_id = sorgu
            .split('&')
            .find_map(|p| p.strip_prefix("kisi="))
            .unwrap_or("")
            .trim();
        if !kisi_id.is_empty() {
            let sube = self.sube_kodu().unwrap_or_default();
            if !kisi::secilebilir(&self.dizin, &sube)
                .iter()
                .any(|k| k.id == kisi_id)
            {
                return Yanit::hata(422, "kişi kayıtta yok ya da kapatılmış");
            }
        }
        let kod = match self.yeni_kod(kisi_id) {
            Ok(k) => k,
            Err(e) => return Yanit::hata(500, &e),
        };
        let url = format!("{}/telefon?kod={kod}", self.taban_url.trim_end_matches('/'));
        match ek_uclar::karekod_svg(&url) {
            Ok(svg) => Yanit::json(
                200,
                json!({ "kod": kod, "url": url, "qr_svg": svg, "gecerli_sn": KOD_OMRU, "kisi": kisi_id }),
            ),
            Err(e) => Yanit::hata(500, &e),
        }
    }

    /// GET /api/yonetim/cihazlar: eşleşmiş telefonlar, son görülme ve bağlı kişi.
    pub(crate) fn cihaz_listesi(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let sube = self.sube_kodu().unwrap_or_default();
        let h = kisi::ad_haritasi(&self.dizin, &sube);
        let gorulme = self
            .son_gorulme
            .lock()
            .map(|m| m.clone())
            .unwrap_or_default();
        let simdi = self.simdi();
        let v: Vec<_> = self
            .cihazlar()
            .aktifler()
            .unwrap_or_default()
            .into_iter()
            .map(|c| {
                let son = gorulme.get(&c.id).copied();
                json!({
                    "id": c.id, "ad": c.ad, "tarih": c.tarih,
                    "kisi_id": c.kisi_id,
                    "kisi": h.get(&c.kisi_id).cloned().unwrap_or_default(),
                    "son_gorulme_sn": son.map(|t| simdi.saturating_sub(t)),
                })
            })
            .collect();
        // İkinci kodun onayını bekleyen telefon sayısı (kodlar dönmez; yalnız sayı).
        let bekleyen = self
            .bekleyen
            .lock()
            .map(|b| {
                b.iter()
                    .filter(|x| x.tamam.is_none() && x.son > simdi)
                    .count()
            })
            .unwrap_or(0);
        Yanit::json(200, json!({ "cihazlar": v, "bekleyen": bekleyen }))
    }

    /// GET /api/yonetim/kisiler: yerel kişi kaydı + Telegram hesapları (yönetici görür).
    pub(crate) fn kisi_listesi_yonetim(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let sube = self.sube_kodu().unwrap_or_default();
        let hesaplar = taa_telegram::hesap::Hesaplar { dizin: &self.dizin }
            .guncel()
            .unwrap_or_default();
        Yanit::json(
            200,
            json!({
                "kisiler": kisi::secilebilir(&self.dizin, &sube),
                "hesaplar": hesaplar,
                "cati_esas": crate::kurulum::cati_bagli(&self.dizin),
            }),
        )
    }

    /// POST /api/yonetim/kisi {ad, rol}. TAA Çatı listesi esassa yerel ekleme yapılmaz (tek kimlik, Fable M6).
    pub(crate) fn kisi_ekle(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        if crate::kurulum::cati_bagli(&self.dizin) {
            return Yanit::hata(
                409,
                "kişiler TAA Çatı'dan gelir; TAA Rehber'de bu şubeye görevlendirin",
            );
        }
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        match kisi::Kisiler::dizinden(&self.dizin).ekle(
            v["ad"].as_str().unwrap_or(""),
            v["rol"].as_str().unwrap_or("personel"),
            &tarih_iso(self.simdi()),
        ) {
            Ok(k) => Yanit::json(200, json!(k)),
            Err(e) => Yanit::hata(422, &e),
        }
    }

    /// POST /api/yonetim/kisi/<id>/kapat
    pub(crate) fn kisi_kapat(&self, i: &Istek, id: &str) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        match kisi::Kisiler::dizinden(&self.dizin).kapat(id, &tarih_iso(self.simdi())) {
            Ok(()) => Yanit::json(200, json!({ "tamam": true })),
            Err(e) => Yanit::hata(404, &e),
        }
    }
}
