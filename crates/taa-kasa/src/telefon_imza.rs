// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! Telefonla gün sonu imzası: Telefon Köprüsü onay akışının taa-kasa istemcisi.
//!
//! Akış (önerilen sözleşme `onay_iste@1` / `onay@1` / `onay_red@1`, kapsam `taa_kasa_gun_sonu`):
//! 1. Kasa programı gün sonu kaydından kanonik bir **özet metni** üretir ve SHA-256'sını alır.
//! 2. `onay_iste@1` isteği (tek kullanımlık istek kimliği, 60 sn geçerlilik) köprü kanalına verilir.
//! 3. Telefon özeti gösterir; kişi biyometrik doğrulamayla onaylar ya da reddeder.
//! 4. `onay@1` gelince istek kimliği, kapsam, süre ve özet **yeniden hesaplanarak** denetlenir; tutarlar
//!    istek gönderildikten sonra değiştiyse imza atılmaz. Geçerliyse `KasaDefteri::imzala` kuralları
//!    (kapanış listesi, giren ikinci imzayı atamaz, en çok iki imza, gönderim sonrası imza yok) aynen
//!    uygulanır ve imza `kanal: telefon`, `cihaz: <eşleşme_kimliği>:<pc_pin>`, `ozet` ile deftere yazılır.
//!
//! Köprü kanalı bir arayüzdür (`KopruKanali`). Gerçek kanal Windows adlandırılmış borusu
//! `\\.\pipe\TelefonKoprusuHizmet` olacaktır; borunun erişim listesinde taa-kasa.exe izni ve köprü tarafındaki
//! onay akışı (u2b-tls-kip2 dalı) birleşene kadar `BoruKanali` her istekte `KanalYok` döner ve ekran
//! 501 + yer tutucu mesajını göstermeye devam eder. Telefon imzasının (P-256) doğrulanması köprünün işidir;
//! bu istemci köprünün doğruladığı cevabı özet ve istek kimliğiyle bağlar.
//! Sözleşme taslağı kesinleşmediği için cevap çözümü esnektir: bilinmeyen alanlar yok sayılır, alan adlarının
//! bilinen eş anlamlıları kabul edilir.

use crate::gunsonu::{GunSonuKaydi, ImzaKanali, ImzaKaydi, KasaDefteri, Kisi, IMZA_ROLLERI};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const ISTEK_TURU: &str = "onay_iste@1";
pub const ONAY_TURU: &str = "onay@1";
pub const RED_TURU: &str = "onay_red@1";
pub const KAPSAM: &str = "taa_kasa_gun_sonu";
/// Özet metninin biçim sürümü; biçim değişirse artar (telefon aynı metni gösterir ve hash'ini doğrular).
pub const OZET_SURUMU: u32 = 1;
/// İsteğin geçerlilik süresi (sn); köprünün PC isteği nonce süresiyle aynı.
pub const GECERLILIK_SN: i64 = 60;
/// Telefon köprüsünün yerel hizmet borusu.
pub const BORU_ADI: &str = r"\\.\pipe\TelefonKoprusuHizmet";
/// Köprü bağlı değilken ekranın ve API'nin gösterdiği mesaj (HTTP 501).
pub const KOPRU_YOK_MESAJI: &str =
    "Telefonla imza köprüsü henüz bağlı değil; imza bu ekrandan (yer tutucu) atılır.";

/// Gün sonu tutarlarının kanonik özet metni: satır satır `anahtar=değer`, anahtarlar sözlük sırasında,
/// tutarlar kuruş cinsinden tam sayı. Kişi adı ve serbest not girmez (yalnız tutarlar ve kimlik).
pub fn ozet_metni(gs: &GunSonuKaydi) -> String {
    let mut s: Vec<(String, String)> = vec![
        ("kapsam".into(), KAPSAM.into()),
        ("surum".into(), OZET_SURUMU.to_string()),
        ("id".into(), gs.id.clone()),
        ("tarih".into(), gs.tarih.clone()),
        ("sube".into(), gs.sube.clone()),
        ("cekmece".into(), gs.cekmece.clone()),
        (
            "nakit_sayilan_kurus".into(),
            gs.nakit_sayilan_kurus.to_string(),
        ),
        ("devreden_kurus".into(), gs.devreden_kurus.to_string()),
        (
            "nebim_toplam_kurus".into(),
            gs.nebim_toplam_kurus.to_string(),
        ),
        (
            "ciro_toplam_kurus".into(),
            gs.karsilastirma.ciro_toplam_kurus.to_string(),
        ),
    ];
    for (k, v) in &gs.ciro {
        s.push((format!("ciro.{k}"), v.to_string()));
    }
    for d in &gs.doviz {
        s.push((format!("doviz.{}.tutar", d.kod), d.tutar.to_string()));
        s.push((
            format!("doviz.{}.kur_onbinde", d.kod),
            d.kur_onbinde.to_string(),
        ));
    }
    for n in &gs.noktalar {
        s.push((
            format!("nokta.{}.z_toplam_kurus", n.nokta),
            n.z_toplam_kurus.to_string(),
        ));
        for (p, v) in &n.pos {
            s.push((format!("nokta.{}.pos.{p}", n.nokta), v.to_string()));
        }
    }
    s.sort();
    let mut m = String::new();
    for (k, v) in s {
        // Satır sonu ve '=' anahtar/değer içinde yer alamaz (kanonik biçim bozulmasın).
        let k = k.replace(['\n', '\r', '='], "_");
        let v = v.replace(['\n', '\r'], " ");
        m.push_str(&k);
        m.push('=');
        m.push_str(&v);
        m.push('\n');
    }
    m
}

/// Özet metninin SHA-256'sı, küçük harf onaltılık.
pub fn ozet_sha256(gs: &GunSonuKaydi) -> String {
    hex(&Sha256::digest(ozet_metni(gs).as_bytes()))
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// `onay_iste@1`: kasa programından köprüye giden imza isteği.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OnayIstegi {
    pub tur: String,
    pub kapsam: String,
    /// Tek kullanımlık istek kimliği (nonce); cevap bununla eşleşir.
    pub istek_kimligi: String,
    pub gun_sonu_id: String,
    /// 1: kasa sorumlusu, 2: mağaza sorumlusu / vekili.
    pub sira: u8,
    pub rol: String,
    pub rol_adi: String,
    /// Ekranda imzalayacak olarak seçilen kişi (cihaz-kişi bağı TAA Merkez'e gelene kadar).
    pub istenen_kisi: String,
    pub ozet_sha256: String,
    /// Telefonda gösterilecek metin; telefon hash'ini kendisi hesaplayıp `ozet_sha256` ile karşılaştırır.
    pub ozet_metni: String,
    pub olusturma: i64,
    pub son_gecerlilik: i64,
}

/// Köprünün bildirdiği cihaz kimliği: eşleşme kimliği + telefon sertifikasının SHA-256 sabitlemesi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CihazKimligi {
    pub eslesme_kimligi: String,
    pub pc_pin: String,
}

impl CihazKimligi {
    /// Defterdeki `cihaz` alanı: `eslesme_kimligi:pc_pin`.
    pub fn defter_metni(&self) -> String {
        format!("{}:{}", self.eslesme_kimligi.trim(), self.pc_pin.trim())
    }
}

/// Köprüden gelen cevap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KopruCevabi {
    /// `onay@1`
    Onay {
        istek_kimligi: String,
        kapsam: String,
        ozet_sha256: String,
        cihaz: CihazKimligi,
        /// Telefonun bildirdiği kişi (boşsa istenen kişi kabul edilir).
        kisi: String,
        /// Kişi telefonda biyometrik/PIN ile doğrulandı mı (Keystore + BiometricPrompt planlı).
        kisi_dogrulandi: bool,
        zaman: i64,
    },
    /// `onay_red@1`
    Red {
        istek_kimligi: String,
        neden: String,
    },
}

fn alan<'a>(v: &'a Value, adlar: &[&str]) -> Option<&'a Value> {
    adlar.iter().find_map(|a| v.get(*a))
}
fn metin(v: &Value, adlar: &[&str]) -> String {
    alan(v, adlar)
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string()
}

impl KopruCevabi {
    /// Esnek çözüm: `tur` "onay@1" / "onay" ya da "onay_red@1" / "red"; bilinmeyen alanlar yok sayılır.
    /// Cihaz kimliği düz (`eslesme_kimligi`, `pc_pin`) ya da `cihaz` nesnesi içinde gelebilir.
    pub fn jsondan(v: &Value) -> Result<KopruCevabi, String> {
        let tur = metin(v, &["tur", "type"]);
        let istek_kimligi = metin(v, &["istek_kimligi", "nonce", "istek"]);
        if istek_kimligi.is_empty() {
            return Err("cevapta istek kimliği yok".into());
        }
        let ana = tur.split('@').next().unwrap_or("");
        match ana {
            "onay" | "uk_onay" => {
                let c = v.get("cihaz").filter(|c| c.is_object()).unwrap_or(v);
                let cihaz = CihazKimligi {
                    eslesme_kimligi: metin(c, &["eslesme_kimligi", "eslesme"]),
                    pc_pin: metin(c, &["pc_pin", "pin"]),
                };
                if cihaz.eslesme_kimligi.is_empty() || cihaz.pc_pin.is_empty() {
                    return Err("onay cevabında cihaz kimliği eksik".into());
                }
                Ok(KopruCevabi::Onay {
                    istek_kimligi,
                    kapsam: metin(v, &["kapsam"]),
                    ozet_sha256: metin(v, &["ozet_sha256", "ozet"]).to_lowercase(),
                    cihaz,
                    kisi: metin(v, &["kisi", "onaylayan"]),
                    kisi_dogrulandi: alan(v, &["kisi_dogrulandi", "biyometrik"])
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                    zaman: alan(v, &["zaman"]).and_then(Value::as_i64).unwrap_or(0),
                })
            }
            "onay_red" | "red" | "uk_onay_red" => Ok(KopruCevabi::Red {
                istek_kimligi,
                neden: metin(v, &["neden", "aciklama"]),
            }),
            _ => Err(format!("bilinmeyen cevap türü: {tur}")),
        }
    }
}

/// TAA Merkez'in cihaz-kişi bağı ve iptal kaydının denetim sonucu. Bağ ve iptal TAA Merkez'de tutulur; kasa
/// programı ve merkez paket okuyucusu yalnız bu sonucu kullanır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CihazDurumu {
    /// Cihaz bu kişiye bağlı ve iptal edilmemiş.
    Gecerli,
    /// Cihaz başka bir kişiye bağlı.
    BaskaKisi,
    /// Cihaz iptal edildi (kayıp, işten ayrılma).
    Iptal,
    /// Merkez kaydı yok ya da ulaşılamıyor.
    Bilinmiyor,
}

impl CihazDurumu {
    pub fn kod(&self) -> &'static str {
        match self {
            CihazDurumu::Gecerli => "gecerli",
            CihazDurumu::BaskaKisi => "baska_kisi",
            CihazDurumu::Iptal => "iptal",
            CihazDurumu::Bilinmiyor => "bilinmiyor",
        }
    }
}

/// TAA Merkez cihaz kaydı arayüzü (`cihaz`: `eslesme_kimligi:pc_pin`).
pub trait CihazKaydi {
    fn denetle(&self, cihaz: &str, kisi: &str) -> CihazDurumu;
}

/// Yer tutucu: TAA Merkez cihaz kaydı bağlanana kadar her cihaz `Bilinmiyor`. Bilinmiyor imzayı engellemez,
/// imza kaydına `cihaz_dogrulama: "bilinmiyor"` yazılır ve merkezde "doğrulanamadı" görünür.
#[derive(Debug, Clone, Copy, Default)]
pub struct MerkezKaydiYok;

impl CihazKaydi for MerkezKaydiYok {
    fn denetle(&self, _cihaz: &str, _kisi: &str) -> CihazDurumu {
        CihazDurumu::Bilinmiyor
    }
}

/// Kanal hatası: `Yok` köprü kurulu/izinli değil (ekran 501 gösterir), `Hata` geçici iletişim hatası.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KanalHatasi {
    Yok(String),
    Hata(String),
}

/// Telefon Köprüsü kanalı. Gerçek uygulama adlandırılmış boru üzerinden JSON satırları taşır.
pub trait KopruKanali {
    /// İsteği köprüye iletir.
    fn gonder(&mut self, istek: &OnayIstegi) -> Result<(), KanalHatasi>;
    /// Cevap geldiyse döner; gelmediyse `None`.
    fn yokla(&mut self, istek_kimligi: &str) -> Result<Option<Value>, KanalHatasi>;
}

/// Yer tutucu boru kanalı: köprü onay akışı birleşip borunun erişim listesine taa-kasa.exe eklenene kadar
/// her çağrıda `KanalYok` döner. Boruya hiçbir şey yazmaz.
#[derive(Debug, Clone)]
pub struct BoruKanali {
    pub boru: String,
}

impl Default for BoruKanali {
    fn default() -> Self {
        BoruKanali {
            boru: BORU_ADI.into(),
        }
    }
}

impl KopruKanali for BoruKanali {
    fn gonder(&mut self, _istek: &OnayIstegi) -> Result<(), KanalHatasi> {
        Err(KanalHatasi::Yok(KOPRU_YOK_MESAJI.into()))
    }
    fn yokla(&mut self, _istek_kimligi: &str) -> Result<Option<Value>, KanalHatasi> {
        Err(KanalHatasi::Yok(KOPRU_YOK_MESAJI.into()))
    }
}

/// İmza oturumunun durumu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImzaDurumu {
    /// İstek telefonda, cevap bekleniyor.
    Bekliyor,
    /// Onaylandı ve deftere yazıldı.
    Onaylandi(ImzaKaydi),
    Reddedildi {
        neden: String,
    },
    ZamanAsimi,
    /// Köprü bağlı değil (API 501 döner).
    KanalYok {
        neden: String,
    },
    /// Cevap geldi ama geçersiz (özet değişti, istek kimliği tutmuyor, defter kuralı vb.); imza atılmadı.
    Gecersiz {
        neden: String,
    },
}

impl ImzaDurumu {
    pub fn bitti(&self) -> bool {
        !matches!(self, ImzaDurumu::Bekliyor)
    }
    /// Ekran/API için kısa kod.
    pub fn kod(&self) -> &'static str {
        match self {
            ImzaDurumu::Bekliyor => "bekliyor",
            ImzaDurumu::Onaylandi(_) => "onaylandi",
            ImzaDurumu::Reddedildi { .. } => "reddedildi",
            ImzaDurumu::ZamanAsimi => "zaman_asimi",
            ImzaDurumu::KanalYok { .. } => "kanal_yok",
            ImzaDurumu::Gecersiz { .. } => "gecersiz",
        }
    }
}

/// Bir gün sonu imzası için telefon oturumu.
#[derive(Debug, Clone)]
pub struct TelefonImzaOturumu {
    pub istek: OnayIstegi,
    pub durum: ImzaDurumu,
    kisi: Kisi,
}

impl TelefonImzaOturumu {
    /// İsteği hazırlar ve kanala verir. `istek_kimligi` çağıran tarafından güvenli rastgele üretilir
    /// (örn. taa-kasa-ekran `rastgele_hex`). Kapanış listesi ve imza kuralları istek açılmadan denetlenir;
    /// kural ihlalinde telefon hiç rahatsız edilmez.
    pub fn baslat(
        defter: &KasaDefteri,
        kanal: &mut dyn KopruKanali,
        gun_sonu_id: &str,
        kisi: &Kisi,
        istek_kimligi: &str,
        simdi: i64,
    ) -> Result<TelefonImzaOturumu, String> {
        if istek_kimligi.trim().len() < 16 {
            return Err("istek kimliği en az 16 karakter olmalı".into());
        }
        if kisi.ad.trim().is_empty() {
            return Err("imzalayan kişiyi seçin".into());
        }
        let gs = defter
            .gun_sonlari()?
            .into_iter()
            .find(|s| s.id == gun_sonu_id)
            .ok_or_else(|| format!("geçerli gün sonu yok: {gun_sonu_id}"))?;
        let sira = defter.imza_on_denetim(&gs, &kisi.ad)?;
        let (rol, rol_adi) = IMZA_ROLLERI[sira as usize - 1];
        let istek = OnayIstegi {
            tur: ISTEK_TURU.into(),
            kapsam: KAPSAM.into(),
            istek_kimligi: istek_kimligi.trim().into(),
            gun_sonu_id: gs.id.clone(),
            sira,
            rol: rol.into(),
            rol_adi: rol_adi.into(),
            istenen_kisi: kisi.ad.trim().into(),
            ozet_sha256: ozet_sha256(&gs),
            ozet_metni: ozet_metni(&gs),
            olusturma: simdi,
            son_gecerlilik: simdi + GECERLILIK_SN,
        };
        let durum = match kanal.gonder(&istek) {
            Ok(()) => ImzaDurumu::Bekliyor,
            Err(KanalHatasi::Yok(n)) => ImzaDurumu::KanalYok { neden: n },
            Err(KanalHatasi::Hata(n)) => ImzaDurumu::Gecersiz {
                neden: format!("istek köprüye iletilemedi: {n}"),
            },
        };
        Ok(TelefonImzaOturumu {
            istek,
            durum,
            kisi: kisi.clone(),
        })
    }

    /// Cevabı yoklar ve durumu ilerletir (merkez cihaz kaydı yer tutucu). Bitmiş oturumda hiçbir şey yapmaz.
    pub fn ilerle(
        &mut self,
        defter: &KasaDefteri,
        kanal: &mut dyn KopruKanali,
        simdi: i64,
    ) -> &ImzaDurumu {
        self.ilerle_denetle(defter, kanal, &MerkezKaydiYok, simdi)
    }

    /// `ilerle`, TAA Merkez cihaz kaydıyla: iptal edilmiş ya da başka kişiye bağlı cihazın onayı geçersizdir.
    pub fn ilerle_denetle(
        &mut self,
        defter: &KasaDefteri,
        kanal: &mut dyn KopruKanali,
        kayit: &dyn CihazKaydi,
        simdi: i64,
    ) -> &ImzaDurumu {
        if self.durum.bitti() {
            return &self.durum;
        }
        let cevap = match kanal.yokla(&self.istek.istek_kimligi) {
            Ok(c) => c,
            Err(KanalHatasi::Yok(n)) => {
                self.durum = ImzaDurumu::KanalYok { neden: n };
                return &self.durum;
            }
            // Geçici hata: süre dolana kadar beklemeye devam.
            Err(KanalHatasi::Hata(_)) => None,
        };
        // Süre isteğin son geçerlilik anına göre denetlenir; geç gelen onay kabul edilmez.
        if simdi > self.istek.son_gecerlilik {
            self.durum = ImzaDurumu::ZamanAsimi;
            return &self.durum;
        }
        if let Some(v) = cevap {
            self.durum = self.cevabi_isle(defter, kayit, &v);
        }
        &self.durum
    }

    fn cevabi_isle(&self, defter: &KasaDefteri, kayit: &dyn CihazKaydi, v: &Value) -> ImzaDurumu {
        let gecersiz = |n: &str| ImzaDurumu::Gecersiz { neden: n.into() };
        let c = match KopruCevabi::jsondan(v) {
            Ok(c) => c,
            Err(e) => return gecersiz(&e),
        };
        match c {
            KopruCevabi::Red {
                istek_kimligi,
                neden,
            } => {
                if istek_kimligi != self.istek.istek_kimligi {
                    return gecersiz("cevap başka bir isteğe ait");
                }
                ImzaDurumu::Reddedildi {
                    neden: if neden.is_empty() {
                        "telefonda reddedildi".into()
                    } else {
                        neden
                    },
                }
            }
            KopruCevabi::Onay {
                istek_kimligi,
                kapsam,
                ozet_sha256: ozet,
                cihaz,
                kisi,
                ..
            } => {
                if istek_kimligi != self.istek.istek_kimligi {
                    return gecersiz("cevap başka bir isteğe ait");
                }
                if !kapsam.is_empty() && kapsam != KAPSAM {
                    return gecersiz("cevabın kapsamı gün sonu değil");
                }
                if ozet != self.istek.ozet_sha256 {
                    return gecersiz("telefonda onaylanan özet istenen özetle aynı değil");
                }
                if !kisi.is_empty() && kisi.to_lowercase() != self.istek.istenen_kisi.to_lowercase()
                {
                    return gecersiz("telefondaki kişi ekranda seçilen kişiyle aynı değil");
                }
                // Tutarlar istekten sonra değiştiyse (düzeltme/iptal) imza atılmaz.
                let guncel = match defter
                    .gun_sonlari()
                    .ok()
                    .and_then(|v| v.into_iter().find(|s| s.id == self.istek.gun_sonu_id))
                {
                    Some(g) => g,
                    None => return gecersiz("gün sonu artık geçerli değil (iptal ya da düzeltme)"),
                };
                if ozet_sha256(&guncel) != self.istek.ozet_sha256 {
                    return gecersiz("gün sonu tutarları istek gönderildikten sonra değişti");
                }
                let cihaz_metni = cihaz.defter_metni();
                let dd = kayit.denetle(&cihaz_metni, &self.kisi.ad);
                match dd {
                    CihazDurumu::Iptal => return gecersiz("cihaz TAA Merkez'de iptal edilmiş"),
                    CihazDurumu::BaskaKisi => {
                        return gecersiz("cihaz TAA Merkez'de başka bir kişiye bağlı")
                    }
                    CihazDurumu::Gecerli | CihazDurumu::Bilinmiyor => {}
                }
                match defter.imzala_ic(
                    &self.istek.gun_sonu_id,
                    ImzaKanali::Telefon,
                    Some(&cihaz_metni),
                    Some(&self.istek.ozet_sha256),
                    Some(dd.kod()),
                    &self.kisi,
                ) {
                    Ok(i) => ImzaDurumu::Onaylandi(i),
                    Err(e) => gecersiz(&e),
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "testler_telefon_imza.rs"]
mod testler;
