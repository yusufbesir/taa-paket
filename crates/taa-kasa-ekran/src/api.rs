// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! TAA Kasa API'si (yalnız yerel ekran çağırır). Her yazma işlemi çekirdekteki kuralları uygular:
//! giren onaylayamaz, düzeltme/iptal yalnız yönetici girişiyle, silme yok.

use crate::{pin_gecerli, pin_ozeti, rastgele_hex};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use taa_kasa::gunsonu::{
    GunSonuGirdi, ImzaKanali, KasaAyar, KasaDefteri, Kisi, MasrafGirdi, OnayTuru, Rol, SayimGirdi,
    SayimTuru, BILGI_ALANLARI, DOVIZLER, KAPANIS_LISTESI, KASA_ACIGI, ODEME_TURLERI,
};
use taa_kasa::merkez::{self, KlasorTasiyici, Kuyruk};
use taa_kasa::sablon;
use taa_kasa::{zaman, DEGERLER};

/// Yönetici oturumu süresi (Terzi Takip ile aynı: 10 dk, her işlemde uzar).
pub const YONETICI_SURESI: Duration = Duration::from_secs(10 * 60);
const PIN_DENEME: u32 = 5;
const PIN_KILIT: Duration = Duration::from_secs(5 * 60);

pub enum Cevap {
    Json(u16, Value),
    /// Sayfada gösterilen HTML (A4 tutanak; indirme değil).
    Html(String),
    Dosya {
        ad: String,
        tur: &'static str,
        bayt: Vec<u8>,
    },
    Kapat,
}

fn tamam(v: Value) -> Cevap {
    Cevap::Json(200, v)
}
fn hata(durum: u16, m: impl Into<String>) -> Cevap {
    Cevap::Json(durum, json!({"hata": m.into()}))
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Gorunum {
    /// "sistem" | "acik" | "koyu"
    pub tema: String,
    /// null: sistem, true: azalt
    pub hareket_azalt: Option<bool>,
    pub tanitim_goruldu: bool,
}

#[derive(Serialize, Deserialize)]
struct YoneticiPin {
    ad: String,
    tuz: String,
    ozet: String,
}

pub struct Uygulama {
    /// `<veri kökü>`
    pub kok: PathBuf,
    pub kisi: Option<Kisi>,
    yonetici: Option<(Kisi, Instant)>,
    pin_hata: u32,
    pin_kilit: Option<Instant>,
}

impl Uygulama {
    pub fn yeni(kok: PathBuf) -> Self {
        Uygulama {
            kok,
            kisi: None,
            yonetici: None,
            pin_hata: 0,
            pin_kilit: None,
        }
    }

    fn kasa(&self) -> PathBuf {
        self.kok.join("kasa")
    }
    fn defter(&self) -> KasaDefteri {
        KasaDefteri::yeni(self.kasa())
    }
    fn kuyruk(&self) -> Kuyruk {
        Kuyruk { kok: self.kasa() }
    }
    fn ayar_yolu(&self) -> PathBuf {
        self.kasa().join("ayar.json")
    }
    fn ayar(&self) -> Result<KasaAyar, String> {
        KasaAyar::yukle(&self.ayar_yolu())
    }

    /// Geçerli yönetici (süresi dolmamışsa; her çağrı süreyi uzatır).
    fn yonetici(&mut self) -> Option<Kisi> {
        match &mut self.yonetici {
            Some((k, son)) if son.elapsed() < YONETICI_SURESI => {
                *son = Instant::now();
                Some(k.clone())
            }
            _ => {
                self.yonetici = None;
                None
            }
        }
    }

    /// İşlemi yapan kişi: yönetici girişi açıksa yönetici, değilse oturumdaki personel.
    fn islem_kisisi(&mut self) -> Option<Kisi> {
        self.yonetici().or_else(|| self.kisi.clone())
    }

    fn bugun(&self, ayar: &KasaAyar) -> String {
        zaman::yerel_tarih(zaman::simdi_sn(), ayar.saat_farki_dk)
    }

    pub fn istek(&mut self, yontem: &str, yol: &str, sorgu: &str, govde: &Value) -> Cevap {
        let ayar = match self.ayar() {
            Ok(a) => a,
            Err(e) => return hata(500, e),
        };
        let s = |ad: &str| {
            govde
                .get(ad)
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string()
        };
        match (yontem, yol) {
            ("GET", "/api/durum") => self.durum(&ayar, sorgu),
            ("POST", "/api/giris") => self.giris(&ayar, &s("ad")),
            ("POST", "/api/cikis") => {
                self.kisi = None;
                self.yonetici = None;
                tamam(json!({"tamam": true}))
            }
            ("POST", "/api/yonetici") => self.yonetici_giris(&s("ad"), &s("pin"), &s("pin_yeni")),
            ("POST", "/api/yonetici/kapat") => {
                self.yonetici = None;
                tamam(json!({"tamam": true}))
            }
            ("GET", "/api/gorunum") => tamam(json!(self.gorunum_oku())),
            ("POST", "/api/gorunum") => self.gorunum_yaz(govde),
            ("POST", "/api/sayim") => self.sayim(&ayar, govde),
            ("POST", "/api/masraf") => self.yaz(govde, |d, k, g| {
                let m: MasrafGirdi =
                    serde_json::from_value(g.clone()).map_err(|e| e.to_string())?;
                d.masraf_ekle(&ayar, &m, k).map(|x| json!(x))
            }),
            ("POST", "/api/masraf/karar") => self.yaz(govde, |d, k, g| {
                let onay = g
                    .get("onay")
                    .and_then(Value::as_bool)
                    .ok_or("onay alanı gerekli")?;
                let id = g.get("id").and_then(Value::as_str).unwrap_or("");
                let not = g.get("not").and_then(Value::as_str).unwrap_or("");
                d.masraf_karar(id, onay, not, k)
                    .map(|_| json!({"tamam": true}))
            }),
            ("POST", "/api/gunsonu/onizle") => match girdi::<GunSonuGirdi>(govde) {
                Ok(g) => match self.defter().gun_sonu_onizle(&ayar, &g) {
                    Ok(k) => tamam(json!(k)),
                    Err(e) => hata(422, e),
                },
                Err(e) => hata(400, e),
            },
            ("POST", "/api/gunsonu") => self.yaz(govde, |d, k, g| {
                let gi: GunSonuGirdi =
                    serde_json::from_value(g.clone()).map_err(|e| e.to_string())?;
                d.gun_sonu_ekle(&ayar, &gi, k).map(|x| json!(x))
            }),
            ("POST", "/api/onay") => self.yaz(govde, |d, k, g| {
                let id = g.get("id").and_then(Value::as_str).unwrap_or("");
                let tur = match g.get("tur").and_then(Value::as_str) {
                    Some("ertesi_gun") => OnayTuru::ErtesiGun,
                    _ => OnayTuru::IkinciKisi,
                };
                d.onayla(id, tur, k).map(|_| json!({"tamam": true}))
            }),
            ("POST", "/api/imza") => {
                // Telefonla imza: istek Telefon Köprüsü kanalına (`telefon_imza`) verilir. Gerçek boru kanalı
                // bağlanana kadar `BoruKanali` "kanal yok" döner ve yanıt 501 + yer tutucu mesajıdır; kişi
                // ekranda kendi adıyla imzalar. Kural ihlali (giren ikinci imza vb.) köprüden önce 422 döner.
                if s("kanal") == "telefon" {
                    return self.telefon_imza(govde);
                }
                self.yaz(govde, |d, k, g| {
                    let id = g.get("id").and_then(Value::as_str).unwrap_or("");
                    d.imzala(id, ImzaKanali::Ekran, None, k).map(|x| json!(x))
                })
            }
            ("POST", "/api/whatsapp") => {
                let tarih = s("tarih");
                if !zaman::tarih_gecerli(&tarih) {
                    return hata(400, "tarih YYYY-AA-GG olmalı");
                }
                let canli = match govde.get("girdi").filter(|v| !v.is_null()) {
                    Some(v) => match girdi::<GunSonuGirdi>(v) {
                        Ok(g) => Some(g),
                        Err(e) => return hata(400, e),
                    },
                    None => None,
                };
                match self.defter().whatsapp_ozeti(&ayar, &tarih, canli.as_ref()) {
                    Ok(m) => tamam(json!({"metin": m})),
                    Err(e) => hata(422, e),
                }
            }
            ("POST", "/api/gerekce") => self.yaz(govde, |d, k, g| {
                let id = g.get("id").and_then(Value::as_str).unwrap_or("");
                let gr = g.get("gerekce").and_then(Value::as_str).unwrap_or("");
                d.ikinci_imza_gerekcesi(id, gr, k)
                    .map(|_| json!({"tamam": true}))
            }),
            ("GET", "/api/tutanak") => {
                if self.kisi.is_none() && self.yonetici().is_none() {
                    return hata(401, "Tutanak için önce giriş yapın.");
                }
                let id = crate::sorgu_degeri(sorgu, "id").unwrap_or("").to_string();
                match self.defter().tutanak(&ayar, &id) {
                    Ok(cd) => {
                        let sube = if ayar.excel_baslik.trim().is_empty() {
                            ayar.sube.clone()
                        } else {
                            format!("{} · {}", ayar.excel_baslik.trim(), ayar.sube)
                        };
                        let an = zaman::yerel_zaman(zaman::simdi_sn(), ayar.saat_farki_dk);
                        if crate::sorgu_degeri(sorgu, "bicim") == Some("json") {
                            tamam(cd)
                        } else {
                            Cevap::Html(taa_kasa::tutanak::tutanak_html(&cd, &sube, &an))
                        }
                    }
                    Err(e) => hata(422, e),
                }
            }
            ("POST", "/api/tutanak") => {
                // Nebim kasa Excel'i (Tutaa kasa-excel.js karşılığı) base64 olarak gelir; ERP sütununa yazılır.
                let mut eslesme = None;
                let erp = match govde.get("excel_b64").and_then(Value::as_str) {
                    Some(b) => {
                        match crate::base64_coz(b).and_then(|x| taa_kasa::tutanak::excel_erp(&x)) {
                            Ok(e) => {
                                let d = e.degerler.clone();
                                eslesme = Some(
                                    json!({"eslesmeyen": e.eslesmeyen, "yok_olan": e.yok_olan, "degerler": e.degerler}),
                                );
                                Some(d)
                            }
                            Err(e) => return hata(422, e),
                        }
                    }
                    None => None,
                };
                let islem = govde.get("islem").and_then(Value::as_str).map(String::from);
                self.yaz(govde, |d, k, g| {
                    let id = g.get("id").and_then(Value::as_str).unwrap_or("");
                    d.tutanak_kaydet(id, erp, islem.as_deref(), k)?;
                    Ok(json!({"tamam": true, "eslesme": eslesme}))
                })
            }
            ("POST", "/api/gonder") => self.gonder(&ayar, &s("id")),
            ("POST", "/api/kuyruk/bosalt") => self.bosalt(&ayar),
            ("POST", "/api/iptal") => match self.yonetici() {
                Some(y) => match self.defter().iptal(&s("id"), &s("neden"), &y) {
                    Ok(()) => tamam(json!({"tamam": true})),
                    Err(e) => hata(422, e),
                },
                None => hata(403, "İptal yalnız yönetici girişiyle yapılır."),
            },
            ("POST", "/api/gunsonu/duzelt") => match self.yonetici() {
                Some(y) => {
                    match girdi::<GunSonuGirdi>(govde.get("girdi").unwrap_or(&Value::Null)) {
                        Ok(g) => match self.defter().gun_sonu_duzelt(
                            &ayar,
                            &s("eski_id"),
                            &g,
                            &s("neden"),
                            &y,
                        ) {
                            Ok(x) => tamam(json!(x)),
                            Err(e) => hata(422, e),
                        },
                        Err(e) => hata(400, e),
                    }
                }
                None => hata(403, "Düzeltme yalnız yönetici girişiyle yapılır."),
            },
            ("GET", "/api/ayar") => tamam(json!(ayar)),
            ("POST", "/api/ayar") => {
                if self.yonetici().is_none() {
                    return hata(403, "Ayarları yalnız yönetici değiştirir.");
                }
                match serde_json::from_value::<KasaAyar>(govde.clone()) {
                    Ok(a) => match a.kaydet(&self.ayar_yolu()) {
                        Ok(()) => tamam(json!(a)),
                        Err(e) => hata(422, e),
                    },
                    Err(e) => hata(400, format!("ayar biçimi: {e}")),
                }
            }
            ("GET", "/api/kayitlar") => self.kayitlar(&ayar, sorgu),
            ("GET", "/api/excel" | "/api/excel/ciro")
                if self.kisi.is_none() && self.yonetici().is_none() =>
            {
                hata(401, "Excel için önce giriş yapın.")
            }
            ("GET", "/api/excel") => self.excel(&ayar, sorgu),
            ("GET", "/api/excel/ciro") => self.ciro_excel(&ayar, sorgu),
            ("GET", "/api/matris") => self.matris(&ayar, sorgu),
            ("POST", "/api/kapat") => Cevap::Kapat,
            _ => hata(404, "bilinmeyen istek"),
        }
    }

    /// Ortak yazma kalıbı: giriş zorunlu.
    fn yaz(
        &mut self,
        govde: &Value,
        f: impl FnOnce(&KasaDefteri, &Kisi, &Value) -> Result<Value, String>,
    ) -> Cevap {
        let Some(k) = self.islem_kisisi() else {
            return hata(401, "Önce giriş yapın.");
        };
        match f(&self.defter(), &k, govde) {
            Ok(v) => tamam(v),
            Err(e) => hata(422, e),
        }
    }

    fn telefon_imza(&mut self, govde: &Value) -> Cevap {
        use taa_kasa::telefon_imza::{BoruKanali, ImzaDurumu, TelefonImzaOturumu};
        let Some(k) = self.islem_kisisi() else {
            return hata(401, "Önce giriş yapın.");
        };
        let id = govde.get("id").and_then(Value::as_str).unwrap_or("");
        let nonce = match rastgele_hex(16) {
            Ok(n) => n,
            Err(e) => return hata(500, e),
        };
        let mut kanal = BoruKanali::default();
        match TelefonImzaOturumu::baslat(
            &self.defter(),
            &mut kanal,
            id,
            &k,
            &nonce,
            zaman::simdi_sn(),
        ) {
            Err(e) => hata(422, e),
            Ok(o) => match o.durum {
                ImzaDurumu::KanalYok { neden } => hata(501, neden),
                // Gerçek kanal bağlandığında: istek telefonda, ekran istek kimliğiyle durumu yoklar.
                d => tamam(
                    json!({"durum": d.kod(), "istek_kimligi": o.istek.istek_kimligi,
                    "son_gecerlilik": o.istek.son_gecerlilik}),
                ),
            },
        }
    }

    fn sayim(&mut self, ayar: &KasaAyar, govde: &Value) -> Cevap {
        let tur = match govde.get("tur").and_then(Value::as_str) {
            Some("acilis") => SayimTuru::Acilis,
            Some("ara") => SayimTuru::Ara,
            Some("devir") => SayimTuru::Devir,
            _ => return hata(400, "sayım türü: acilis, ara ya da devir"),
        };
        self.yaz(govde, |d, k, g| {
            let gi: SayimGirdi = serde_json::from_value(g.clone()).map_err(|e| e.to_string())?;
            d.sayim_ekle(ayar, tur, &gi, k).map(|x| json!(x))
        })
    }

    fn giris(&mut self, ayar: &KasaAyar, ad: &str) -> Cevap {
        if ad.is_empty() || ad.chars().count() > 60 {
            return hata(400, "Adınızı seçin.");
        }
        let rol = if ayar.personel.is_empty() {
            // İlk kurulum: liste boşken herkes personel olarak girer; yönetici listeyi Ayarlar'dan doldurur.
            Rol::Personel
        } else {
            match ayar
                .personel
                .iter()
                .find(|p| p.ad.trim().eq_ignore_ascii_case(ad))
            {
                Some(p) => p.rol,
                None => {
                    return hata(
                        403,
                        "Bu ad personel listesinde yok. Yönetici Ayarlar'dan ekler.",
                    )
                }
            }
        };
        let k = Kisi {
            ad: ad.into(),
            rol,
            dogrulandi: false,
        };
        self.kisi = Some(k.clone());
        tamam(json!({"kisi": k}))
    }

    fn pin_yolu(&self) -> PathBuf {
        self.kasa().join("yonetici-pin.json")
    }

    fn yonetici_giris(&mut self, ad: &str, pin: &str, pin_yeni: &str) -> Cevap {
        if let Some(t) = self.pin_kilit {
            if t.elapsed() < PIN_KILIT {
                return hata(
                    429,
                    "Çok sayıda yanlış PIN. 5 dakika sonra yeniden deneyin.",
                );
            }
            self.pin_kilit = None;
            self.pin_hata = 0;
        }
        let kayit: Option<YoneticiPin> = std::fs::read(self.pin_yolu())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        let ad_kayit = match kayit {
            None => {
                // İlk kullanım: geçici yerel PIN belirlenir.
                if pin_yeni.is_empty() {
                    return Cevap::Json(200, json!({"kurulum": true}));
                }
                if ad.is_empty() || !pin_gecerli(pin_yeni) {
                    return hata(400, "Yönetici adı ve 6–12 haneli PIN gerekli.");
                }
                let tuz = match rastgele_hex(16) {
                    Ok(t) => t,
                    Err(e) => return hata(500, e),
                };
                let k = YoneticiPin {
                    ad: ad.into(),
                    ozet: pin_ozeti(&tuz, pin_yeni),
                    tuz,
                };
                let yaz = std::fs::create_dir_all(self.kasa())
                    .map_err(|e| e.to_string())
                    .and_then(|_| serde_json::to_vec(&k).map_err(|e| e.to_string()))
                    .and_then(|b| {
                        taa_dosya::uzerine_yaz_atomik(&self.pin_yolu(), &b)
                            .map_err(|e| e.to_string())
                    });
                if let Err(e) = yaz {
                    return hata(500, format!("PIN kaydedilemedi: {e}"));
                }
                k.ad
            }
            Some(k) => {
                if !crate::esit(&pin_ozeti(&k.tuz, pin), &k.ozet) {
                    self.pin_hata += 1;
                    if self.pin_hata >= PIN_DENEME {
                        self.pin_kilit = Some(Instant::now());
                    }
                    return hata(401, "PIN yanlış.");
                }
                self.pin_hata = 0;
                k.ad
            }
        };
        let y = Kisi {
            ad: ad_kayit,
            rol: Rol::Yonetici,
            dogrulandi: true,
        };
        self.yonetici = Some((y.clone(), Instant::now()));
        tamam(json!({"yonetici": y, "sure_sn": YONETICI_SURESI.as_secs()}))
    }

    fn gorunum_yolu(&self) -> PathBuf {
        self.kasa().join("gorunum.json")
    }
    fn gorunum_oku(&self) -> Gorunum {
        std::fs::read(self.gorunum_yolu())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_else(|| Gorunum {
                tema: "sistem".into(),
                ..Default::default()
            })
    }
    fn gorunum_yaz(&self, govde: &Value) -> Cevap {
        let mut g = self.gorunum_oku();
        if let Some(t) = govde.get("tema").and_then(Value::as_str) {
            if !matches!(t, "sistem" | "acik" | "koyu") {
                return hata(400, "tema: sistem, acik ya da koyu");
            }
            g.tema = t.into();
        }
        if let Some(h) = govde.get("hareket_azalt") {
            g.hareket_azalt = h.as_bool();
        }
        if let Some(t) = govde.get("tanitim_goruldu").and_then(Value::as_bool) {
            g.tanitim_goruldu = t;
        }
        let yaz = std::fs::create_dir_all(self.kasa())
            .map_err(|e| e.to_string())
            .and_then(|_| serde_json::to_vec(&g).map_err(|e| e.to_string()))
            .and_then(|b| {
                taa_dosya::uzerine_yaz_atomik(&self.gorunum_yolu(), &b).map_err(|e| e.to_string())
            });
        match yaz {
            Ok(()) => tamam(json!(g)),
            Err(e) => hata(500, e),
        }
    }

    fn durum(&mut self, ayar: &KasaAyar, sorgu: &str) -> Cevap {
        let tarih = crate::sorgu_degeri(sorgu, "tarih")
            .filter(|t| zaman::tarih_gecerli(t))
            .map(String::from)
            .unwrap_or_else(|| self.bugun(ayar));
        let cekmece = crate::sorgu_degeri(sorgu, "cekmece")
            .filter(|c| ayar.cekmece(c).is_some())
            .map(String::from)
            .unwrap_or_else(|| ayar.cekmeceler[0].id.clone());
        let d = self.defter();
        let gd = match d.gun_durumu(ayar, &tarih, &cekmece) {
            Ok(g) => g,
            Err(e) => return hata(500, e),
        };
        let form: Vec<Value> = KasaDefteri::gunluk_form(&gd)
            .into_iter()
            .map(|(a, b, c)| json!({"ad": a, "tamam": b, "ayrinti": c}))
            .collect();
        let yon = self.yonetici();
        let pin_kurulu = self.pin_yolu().is_file();
        tamam(json!({
            "bugun": self.bugun(ayar),
            "tarih": tarih,
            "cekmece": cekmece,
            "sube": ayar.sube,
            "cekmeceler": ayar.cekmeceler,
            "personel": ayar.personel.iter().map(|p| json!({"ad": p.ad, "rol": p.rol})).collect::<Vec<_>>(),
            "masraf_turleri": ayar.masraf_turleri,
            "masraf_merkez_esigi_kurus": ayar.masraf_merkez_esigi_kurus,
            "kasa_limiti_kurus": ayar.kasa_limiti_kurus,
            "kisi": self.kisi,
            "yonetici": yon,
            "yonetici_pin_kurulu": pin_kurulu,
            "ortak_giris_bagli": false,
            "gun": gd,
            "form": form,
            "kuyruk": {"bekleyen": self.kuyruk().bekleyenler().len(), "merkez_klasoru": ayar.merkez_klasoru.is_some()},
            "sabitler": {
                "degerler": DEGERLER,
                "odeme_turleri": ODEME_TURLERI.iter().map(|(k, a)| {
                    let a = if *k == "havale" && !ayar.havale_hesap_adi.trim().is_empty() {
                        format!("Bankaya havale: {}", ayar.havale_hesap_adi.trim())
                    } else {
                        a.to_string()
                    };
                    json!([k, a])
                }).collect::<Vec<_>>(),
                "kasa_acigi": [KASA_ACIGI.0, KASA_ACIGI.1],
                "bilgi_alanlari": BILGI_ALANLARI.iter().map(|(k, a)| json!([k, a])).collect::<Vec<_>>(),
                "dovizler": DOVIZLER.iter().map(|(k, a)| json!([k, a])).collect::<Vec<_>>(),
                "kapanis_listesi": KAPANIS_LISTESI,
                "kapanis_adimlari": taa_kasa::gunsonu::KAPANIS_ADIMLARI,
            },
        }))
    }

    /// Gönder: paket hazırlanır, cihaz anahtarı varsa imzalanır, kuyruğa alınır, merkeze ulaşılabiliyorsa teslim edilir.
    fn gonder(&mut self, ayar: &KasaAyar, id: &str) -> Cevap {
        if self.islem_kisisi().is_none() {
            return hata(401, "Önce giriş yapın.");
        }
        let d = self.defter();
        let mut p = match d.gonderim_paketi(ayar, id) {
            Ok(p) => p,
            Err(e) => return hata(422, e),
        };
        // Şube imzası: mağaza anahtarı sözleşmesi (Ş-sözleşme) bağlanana kadar cihaz anahtarı kullanılır.
        let anahtar = self.kok.join("cihaz").join("cihaz.key");
        let imzali = match taa_imza::OzelAnahtar::dosyadan(&anahtar) {
            Ok((o, _)) => taa_imza::yayin_imzala(&mut p, &o).is_ok(),
            Err(_) => false,
        };
        let ad = format!("{id}.json");
        if let Err(e) = self.kuyruk().ekle(&ad, p.to_string().as_bytes()) {
            return hata(500, e);
        }
        if let Err(e) = d.gonderim_kaydet(id, &ad, imzali) {
            return hata(500, e);
        }
        let (n, h) = self.teslim(ayar);
        tamam(
            json!({"kuyrukta": true, "imzali": imzali, "teslim": n, "hata": h,
            "bekleyen": self.kuyruk().bekleyenler().len()}),
        )
    }

    fn teslim(&self, ayar: &KasaAyar) -> (usize, Option<String>) {
        match &ayar.merkez_klasoru {
            Some(k) => self.kuyruk().bosalt(&KlasorTasiyici {
                hedef: PathBuf::from(k),
            }),
            None => (
                0,
                Some("Merkez bağlantısı tanımlı değil; paket kuyrukta bekliyor.".into()),
            ),
        }
    }

    fn bosalt(&self, ayar: &KasaAyar) -> Cevap {
        let (n, h) = self.teslim(ayar);
        tamam(json!({"teslim": n, "hata": h, "bekleyen": self.kuyruk().bekleyenler().len()}))
    }

    fn aralik(&self, ayar: &KasaAyar, sorgu: &str) -> (String, String) {
        let bit = crate::sorgu_degeri(sorgu, "bit")
            .filter(|t| zaman::tarih_gecerli(t))
            .map(String::from)
            .unwrap_or_else(|| self.bugun(ayar));
        let bas = crate::sorgu_degeri(sorgu, "bas")
            .filter(|t| zaman::tarih_gecerli(t))
            .map(String::from)
            .unwrap_or_else(|| zaman::gun_ekle(&bit, -30).unwrap_or(bit.clone()));
        (bas, bit)
    }

    fn kayitlar(&self, ayar: &KasaAyar, sorgu: &str) -> Cevap {
        let (bas, bit) = self.aralik(ayar, sorgu);
        let d = self.defter();
        let ic = |t: &str| t >= bas.as_str() && t <= bit.as_str();
        let r = (|| -> Result<Value, String> {
            Ok(json!({
                "bas": bas, "bit": bit,
                "gun_sonlari": d.gun_sonlari()?.into_iter().filter(|x| ic(&x.tarih)).collect::<Vec<_>>(),
                "masraflar": d.masraflar()?.into_iter().filter(|x| ic(&x.kayit.tarih)).collect::<Vec<_>>(),
                "sayimlar": d.sayimlar()?.into_iter().filter(|x| ic(&x.tarih)).collect::<Vec<_>>(),
                "magaza": taa_kasa::gunsonu::magaza_toplami(&d.gun_sonlari()?, &bit),
            }))
        })();
        match r {
            Ok(v) => tamam(v),
            Err(e) => hata(500, e),
        }
    }

    fn excel(&self, ayar: &KasaAyar, sorgu: &str) -> Cevap {
        let (bas, bit) = self.aralik(ayar, sorgu);
        let d = self.defter();
        let ic = |t: &str| t >= bas.as_str() && t <= bit.as_str();
        let r = (|| -> Result<Vec<u8>, String> {
            let g: Vec<_> = d
                .gun_sonlari()?
                .into_iter()
                .filter(|x| ic(&x.tarih))
                .collect();
            let m: Vec<_> = d
                .masraflar()?
                .into_iter()
                .filter(|x| ic(&x.kayit.tarih))
                .collect();
            let s: Vec<_> = d.sayimlar()?.into_iter().filter(|x| ic(&x.tarih)).collect();
            Ok(merkez::excel(
                &ayar.sube,
                &g,
                &m,
                &s,
                &d.imza_haritasi()?,
                &d.gerekce_haritasi()?,
            ))
        })();
        match r {
            Ok(b) => Cevap::Dosya {
                ad: format!(
                    "taa-kasa-{}-{bas}-{bit}.xlsx",
                    if ayar.sube.is_empty() {
                        "sube"
                    } else {
                        &ayar.sube
                    }
                ),
                tur: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                bayt: b,
            },
            Err(e) => hata(500, e),
        }
    }

    /// Akşam ciro şablonu (aylık, gün başına sayfa): şubelerin akşam Excel'iyle aynı düzen.
    fn ciro_excel(&self, ayar: &KasaAyar, sorgu: &str) -> Cevap {
        let ay = crate::sorgu_degeri(sorgu, "ay")
            .map(String::from)
            .unwrap_or_else(|| {
                zaman::yerel_tarih(zaman::simdi_sn(), ayar.saat_farki_dk)[..7].to_string()
            });
        let d = self.defter();
        let r = (|| -> Result<Vec<u8>, String> {
            sablon::ciro_sablonu(ayar, &ay, &d.gun_sonlari()?, &d.masraflar()?)
        })();
        match r {
            Ok(b) => Cevap::Dosya {
                ad: format!("ciro-sablonu-{ay}.xlsx"),
                tur: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                bayt: b,
            },
            Err(e) => hata(422, e),
        }
    }

    /// Merkez matrisi (yönetici). Sunucu kasa ucu gelene kadar merkez klasöründeki paketlerden.
    fn matris(&mut self, ayar: &KasaAyar, sorgu: &str) -> Cevap {
        if self.yonetici().is_none() {
            return hata(403, "Merkez matrisi yönetici girişiyle görünür.");
        }
        let Some(k) = &ayar.merkez_klasoru else {
            return hata(422, "Merkez klasörü tanımlı değil (Ayarlar).");
        };
        let gun: i64 = crate::sorgu_degeri(sorgu, "gun")
            .and_then(|g| g.parse().ok())
            .unwrap_or(7)
            .clamp(1, 62);
        let bugun = self.bugun(ayar);
        let tarihler: Vec<String> = (0..gun)
            .rev()
            .filter_map(|i| zaman::gun_ekle(&bugun, -i))
            .collect();
        let (gelen, bozuk) = merkez::gelenleri_oku(std::path::Path::new(k));
        let mut subeler: Vec<String> = gelen.iter().map(|g| g.sube.clone()).collect();
        if !ayar.sube.is_empty() {
            subeler.push(ayar.sube.clone());
        }
        subeler.sort();
        subeler.dedup();
        let mut m = merkez::matris(
            &gelen,
            &subeler,
            &tarihler,
            &ayar.gonderim_son_saat,
            ayar.saat_farki_dk,
            zaman::simdi_sn(),
        );
        merkez::tek_imza_uyarisi_uygula(
            &mut m,
            &gelen,
            &bugun,
            ayar.tek_imza_uyarisi.pencere_gun,
            ayar.tek_imza_uyarisi.esik,
        );
        tamam(
            json!({"tarihler": tarihler, "satirlar": m, "okunamayan": bozuk,
            "tek_imza_uyarisi": ayar.tek_imza_uyarisi,
            "not": "Şube listesi şimdilik merkeze paket gönderen şubelerden çıkar; tam liste Çatı'daki şube kaydından gelecek."}),
        )
    }
}

fn girdi<T: for<'de> Deserialize<'de>>(v: &Value) -> Result<T, String> {
    serde_json::from_value(v.clone()).map_err(|e| format!("girdi biçimi: {e}"))
}

#[cfg(test)]
mod testler {
    use super::*;

    fn uyg() -> Uygulama {
        Uygulama::yeni(taa_dosya::gecici_klasor().unwrap())
    }
    fn json_of(c: Cevap) -> (u16, Value) {
        match c {
            Cevap::Json(d, v) => (d, v),
            _ => panic!("json bekleniyordu"),
        }
    }

    #[test]
    fn girissiz_yazilmaz_ve_akis() {
        let mut u = uyg();
        let (d, _) = json_of(u.istek(
            "POST",
            "/api/sayim",
            "",
            &json!({"tur":"acilis","tarih":"2026-11-26","cekmece":"c1","adetler":{"10000":5}}),
        ));
        assert_eq!(d, 401);
        let (d, _) = json_of(u.istek("POST", "/api/giris", "", &json!({"ad":"Ayşe"})));
        assert_eq!(d, 200);
        let (d, v) = json_of(u.istek(
            "POST",
            "/api/sayim",
            "",
            &json!({"tur":"acilis","tarih":"2026-11-26","cekmece":"c1","adetler":{"10000":5}}),
        ));
        assert_eq!(d, 200, "{v}");
        let (_, v) = json_of(u.istek("GET", "/api/durum", "tarih=2026-11-26", &Value::Null));
        assert_eq!(v["gun"]["asama"], "gun_ici");
        assert_eq!(v["ortak_giris_bagli"], false);
        // Personel iptal edemez.
        let (d, _) = json_of(u.istek(
            "POST",
            "/api/iptal",
            "",
            &json!({"id":"AC-20261126-c1-1","neden":"x"}),
        ));
        assert_eq!(d, 403);
        // Ayar yazmak yönetici ister.
        let (d, _) = json_of(u.istek("POST", "/api/ayar", "", &json!({})));
        assert_eq!(d, 403);
    }

    #[test]
    fn imza_ve_whatsapp_uclari() {
        let mut u = uyg();
        u.istek("POST", "/api/giris", "", &json!({"ad":"Ayşe"}));
        let t = "2026-11-25";
        u.istek(
            "POST",
            "/api/sayim",
            "",
            &json!({"tur":"acilis","tarih":t,"cekmece":"c1","adetler":{"10000":1}}),
        );
        let g = json!({"tarih":t,"cekmece":"c1","nakit_adetler":{"10000":3},"ciro":{"nakit":20000},
            "nebim_toplam_kurus":20000,"noktalar":[],"not":"Z ayrı girilecek","kontrol":[true,true,true,true,true,true,true]});
        // Canlı özet: kaydedilmemiş girdiden.
        let (d, v) = json_of(u.istek("POST", "/api/whatsapp", "", &json!({"tarih":t,"girdi":g})));
        assert_eq!(d, 200, "{v}");
        assert!(v["metin"].as_str().unwrap().contains("Nakit: 200,00 TL"));
        let (d, gs) = json_of(u.istek("POST", "/api/gunsonu", "", &g));
        assert_eq!(d, 200, "{gs}");
        let id = gs["id"].as_str().unwrap().to_string();
        // Telefon köprüsü yok: açıkça reddedilir.
        let (d, v) = json_of(u.istek("POST", "/api/imza", "", &json!({"id":id,"kanal":"telefon"})));
        assert_eq!(d, 501);
        assert!(
            v.to_string()
                .contains(taa_kasa::telefon_imza::KOPRU_YOK_MESAJI),
            "{v}"
        );
        let (d, v) = json_of(u.istek("POST", "/api/imza", "", &json!({"id":id,"kanal":"ekran"})));
        assert_eq!((d, v["sira"].as_i64()), (200, Some(1)));
        let (_, v) = json_of(u.istek("GET", "/api/durum", "tarih=2026-11-25", &Value::Null));
        assert_eq!(v["gun"]["asama"], "gonder");
        assert_eq!(v["gun"]["ikinci_imza_durumu"], "ikinci imza atmadı");
        // Tek imzayla gönderim gerekçe ister.
        let (d, _) = json_of(u.istek("POST", "/api/gerekce", "", &json!({"id":id,"gerekce":"x"})));
        assert_eq!(d, 422);
        let (d, _) = json_of(u.istek(
            "POST",
            "/api/gerekce",
            "",
            &json!({"id":id,"gerekce":"ikinci kişi izinli"}),
        ));
        assert_eq!(d, 200);
        let (_, v) = json_of(u.istek("GET", "/api/durum", "tarih=2026-11-25", &Value::Null));
        assert_eq!(v["gun"]["ikinci_imza_gerekcesi"], "ikinci kişi izinli");
        // Giren ikinci imzayı atamaz; telefonla da istek köprüye gitmeden reddedilir.
        let (d, _) = json_of(u.istek("POST", "/api/imza", "", &json!({"id":id,"kanal":"ekran"})));
        assert_eq!(d, 422);
        let (d, _) = json_of(u.istek("POST", "/api/imza", "", &json!({"id":id,"kanal":"telefon"})));
        assert_eq!(d, 422);
    }

    #[test]
    fn tutanak_a4_ve_excel_ice_aktarma() {
        let mut u = uyg();
        assert!(matches!(
            u.istek("GET", "/api/tutanak", "id=x", &Value::Null),
            Cevap::Json(401, _)
        ));
        u.istek("POST", "/api/giris", "", &json!({"ad":"Ayşe"}));
        let t = "2026-11-25";
        u.istek(
            "POST",
            "/api/sayim",
            "",
            &json!({"tur":"acilis","tarih":t,"cekmece":"c1","adetler":{"10000":1}}),
        );
        let g = json!({"tarih":t,"cekmece":"c1","nakit_adetler":{"10000":3},"ciro":{"nakit":20000},
            "nebim_toplam_kurus":20000,"noktalar":[],"not":"Z ayrı","kontrol":[true,true,true,true,true,true,true]});
        let (_, gs) = json_of(u.istek("POST", "/api/gunsonu", "", &g));
        let id = gs["id"].as_str().unwrap().to_string();
        let html = match u.istek("GET", "/api/tutanak", &format!("id={id}"), &Value::Null) {
            Cevap::Html(h) => h,
            _ => panic!("html bekleniyordu"),
        };
        assert!(html.contains("TASLAK") && html.contains("KCD-2026-"));
        // Nebim Excel'i: ERP sütunu içe aktarılır.
        use taa_kasa::xlsx::{Hucre, Sayfa};
        let x = taa_kasa::xlsx::xlsx_uret(&[Sayfa {
            ad: "N".into(),
            satirlar: vec![
                vec![Hucre::from("Kalem"), Hucre::from("ERP")],
                vec![Hucre::from("Nakit (TL)"), Hucre::Kurus(19_000)],
            ],
            ..Default::default()
        }]);
        let (d, v) = json_of(u.istek(
            "POST",
            "/api/tutanak",
            "",
            &json!({"id":id,"excel_b64":crate::base64_kodla(&x),"islem":"kamera incelendi"}),
        ));
        assert_eq!(d, 200, "{v}");
        let (_, cd) = json_of(u.istek(
            "GET",
            "/api/tutanak",
            &format!("id={id}&bicim=json"),
            &Value::Null,
        ));
        assert_eq!(cd["kalemler"][0]["erp_kurus"], 19_000);
        assert_eq!(cd["kalemler"][0]["fark_kurus"], 1_000);
        assert_eq!(cd["sonuc"], "Fark tespit edilmiştir.");
        assert_eq!(cd["islem"], "kamera incelendi");
    }

    #[test]
    fn yonetici_pin_kurulum_dogrulama_kilit() {
        let mut u = uyg();
        let (_, v) = json_of(u.istek("POST", "/api/yonetici", "", &json!({"ad":"Yusuf"})));
        assert_eq!(v["kurulum"], true);
        let (d, _) = json_of(u.istek(
            "POST",
            "/api/yonetici",
            "",
            &json!({"ad":"Yusuf","pin_yeni":"12"}),
        ));
        assert_eq!(d, 400);
        let (d, _) = json_of(u.istek(
            "POST",
            "/api/yonetici",
            "",
            &json!({"ad":"Yusuf","pin_yeni":"482913"}),
        ));
        assert_eq!(d, 200);
        let dosya = std::fs::read_to_string(u.kok.join("kasa/yonetici-pin.json")).unwrap();
        assert!(!dosya.contains("482913"), "PIN düz metin yazılmaz");
        u.istek("POST", "/api/yonetici/kapat", "", &json!({}));
        for _ in 0..5 {
            let (d, _) = json_of(u.istek("POST", "/api/yonetici", "", &json!({"pin":"000000"})));
            assert_eq!(d, 401);
        }
        let (d, _) = json_of(u.istek("POST", "/api/yonetici", "", &json!({"pin":"482913"})));
        assert_eq!(d, 429, "kilitli");
    }

    #[test]
    fn excel_dosya_doner() {
        let mut u = uyg();
        assert!(matches!(
            u.istek("GET", "/api/excel/ciro", "ay=2026-11", &Value::Null),
            Cevap::Json(401, _)
        ));
        u.istek("POST", "/api/giris", "", &json!({"ad":"Ayşe"}));
        match u.istek(
            "GET",
            "/api/excel",
            "bas=2026-11-01&bit=2026-11-26",
            &Value::Null,
        ) {
            Cevap::Dosya { ad, bayt, .. } => {
                assert!(ad.ends_with(".xlsx"));
                assert_eq!(&bayt[..2], b"PK");
            }
            _ => panic!("dosya bekleniyordu"),
        }
        match u.istek("GET", "/api/excel/ciro", "ay=2026-11", &Value::Null) {
            Cevap::Dosya { ad, bayt, .. } => {
                assert_eq!(ad, "ciro-sablonu-2026-11.xlsx");
                let x = String::from_utf8_lossy(&bayt);
                assert!(x.contains("name=\"30.11.2026\""));
            }
            _ => panic!("ciro şablonu bekleniyordu"),
        }
        assert!(matches!(
            u.istek("GET", "/api/excel/ciro", "ay=2026-13", &Value::Null),
            Cevap::Json(422, _)
        ));
    }
}
