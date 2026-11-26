// Sahip: Yusuf. yazan: [Claude#egitim-panel 2026-11-26]
//! Panelin API'si: HTTP'den bağımsız, saf işlevlerle (`Panel::api`). Girdi: kullanıcı (oturumdan, yoksa None),
//! yöntem, yol, sorgu, JSON gövde, şimdi (ISO). Çıktı: (durum kodu, JSON).
//!
//! Kurallar:
//! - Oturum yoksa her uç 401 (yalnız `/api/oturum` ve `GET /api/gorunum` oturumsuz çalışır).
//! - Yönetici uçları (`/api/ozet`, `/api/yonetim`, `/api/atama*`) yönetici değilse 403 (varsayılan REDDET).
//! - Kişi kendi izleme kaydını yazar (`/api/konum`, `/api/izledim`); başkası adına kayıt ucu yoktur.
//! - Video yoksa (bağ yok ya da dosya yok) konum ve "izledim" kaydedilmez: dürüst boş durum.
//! - Her değişiklik önce JSONL günlüğe sonra belleğe işlenir (`taa-egitim`); günlük yazılamazsa 500, kayıp sessiz olmaz.

use crate::gorunum::{self, Tema};
use crate::guvenlik::{sorgu_degeri, yuzde_coz};
use crate::kimlik::Kullanici;
use crate::veri::{self, YuklenenKatalog};
use crate::video::{self, Karar, KaynakTuru, VideoDurumu, VideoKaynagi};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use taa_egitim::{Durum, Egitim, Gunluk, Hata, KisiDersi, Olay, RehberVeri};
use taa_rehber_model::model::{KisiDurum, Veri};

/// API yanıtı: (HTTP durum, JSON gövde).
pub type Yanit = (u16, Value);

/// Videonun HTTP'ye çevrilecek sonucu (bin dosyayı akıtır).
#[derive(Debug)]
pub enum VideoYaniti {
    Hata(Yanit),
    Dosya {
        yol: PathBuf,
        tur: &'static str,
        karar: Karar,
    },
}

/// Panelin tüm durumu. Tek iş parçacığında kullanılır (sunucu döngüsü sırayla işler).
pub struct Panel {
    pub egitim: Egitim,
    pub rehber: Veri,
    pub videolar: BTreeMap<String, VideoKaynagi>,
    pub konu_adlari: BTreeMap<String, String>,
    pub video_kok: PathBuf,
    pub gunluk: Option<Gunluk>,
    pub demo: bool,
    pub uyarilar: Vec<String>,
    pub tema: Tema,
    pub tema_yolu: Option<PathBuf>,
    /// Kişinin en son konum kaydettiği ders ("kaldığın yerden devam et" için).
    son_ders: BTreeMap<String, String>,
}

fn hata(durum: u16, mesaj: impl Into<String>, kod: u8) -> Yanit {
    (durum, json!({ "hata": mesaj.into(), "kod": kod }))
}

fn girdi(m: &str) -> Yanit {
    hata(400, format!("geçersiz girdi: {m}"), 2)
}

fn oturum_yok() -> Yanit {
    hata(401, "Oturum gerekli. Öneri: giriş yapın.", 5)
}

fn yonetici_gerekli() -> Yanit {
    hata(403, "Bu işlem yönetici yetkisi ister.", 7)
}

/// `taa-egitim` hatası → HTTP yanıtı.
pub fn egitim_hatasi(e: &Hata) -> Yanit {
    let (durum, kod) = match e {
        Hata::Gecersiz { .. } => (400, 2),
        Hata::DersYok(_) => (404, 2),
        Hata::DersVar(_) | Hata::SurumGeri { .. } | Hata::SurumEski { .. } => (409, 4),
        Hata::KonumAsimi { .. } => (400, 2),
        Hata::KisiYok(_) | Hata::YoneticiGerekli => (403, 7),
        Hata::Sinir(_) => (422, 4),
        Hata::Gunluk(_) => (500, 1),
    };
    let mesaj = match e {
        Hata::KisiYok(_) => format!("{e}. Öneri: rehberde etkin kayıt olmalı."),
        Hata::Gunluk(_) => format!("{e}. Öneri: veri kökü yazılabilir mi bakın."),
        _ => e.to_string(),
    };
    hata(durum, mesaj, kod)
}

fn s<'a>(g: &'a Value, ad: &str) -> Option<&'a str> {
    g.get(ad).and_then(Value::as_str)
}

fn durum_adi(d: Durum, atandi: bool) -> &'static str {
    match d {
        Durum::Izledi => "izledi",
        Durum::Basladi => "basladi",
        Durum::Atandi if atandi => "atandi",
        Durum::Atandi => "baslanmadi",
    }
}

fn yuzde(saniye: u64, sure_sn: u64) -> u8 {
    if sure_sn == 0 {
        return 0;
    }
    (saniye.min(sure_sn) * 100 / sure_sn) as u8
}

impl Panel {
    /// Paneli kurar. `gunluk_yolu` verilirse günlük açılır ve durum günlükten yeniden kurulur; verilmezse (demo)
    /// hiçbir şey diske yazılmaz.
    pub fn kur(
        yuk: YuklenenKatalog,
        rehber: Veri,
        video_kok: PathBuf,
        gunluk_yolu: Option<&Path>,
        demo: bool,
        tema_yolu: Option<PathBuf>,
        mut uyarilar: Vec<String>,
    ) -> Result<Panel, String> {
        let mut son_ders = BTreeMap::new();
        let (egitim, gunluk) = match gunluk_yolu {
            None => (Egitim::yeni(yuk.katalog), None),
            Some(yol) => {
                if let Some(k) = yol.parent() {
                    std::fs::create_dir_all(k)
                        .map_err(|e| format!("{} oluşturulamadı: {e}", k.display()))?;
                }
                let g = Gunluk::ac(yol).map_err(|e| e.to_string())?;
                let (satirlar, bozuk) = Gunluk::oku(yol).map_err(|e| e.to_string())?;
                if bozuk > 0 {
                    uyarilar.push(format!("İzleme günlüğünde {bozuk} bozuk satır atlandı."));
                }
                for sa in &satirlar {
                    if let Olay::Konum { kisi, ders, .. } = &sa.olay {
                        son_ders.insert(kisi.clone(), ders.clone());
                    }
                }
                let (e, atlanan) = Egitim::gunlukten(yuk.katalog, &satirlar);
                if atlanan > 0 {
                    uyarilar.push(format!(
                        "İzleme günlüğünde katalogda olmayan derse ait {atlanan} satır atlandı."
                    ));
                }
                (e, Some(g))
            }
        };
        let tema = tema_yolu.as_deref().map(gorunum::oku).unwrap_or_default();
        Ok(Panel {
            egitim,
            rehber,
            videolar: yuk.videolar,
            konu_adlari: yuk.konu_adlari,
            video_kok,
            gunluk,
            demo,
            uyarilar,
            tema,
            tema_yolu,
            son_ders,
        })
    }

    fn video_durumu(&self, ders: &str) -> VideoDurumu {
        video::durum(self.videolar.get(ders), &self.video_kok)
    }

    fn kisi_etkin(&self, id: &str) -> bool {
        self.rehber
            .kisiler
            .get(id)
            .is_some_and(|k| k.durum == KisiDurum::Etkin)
    }

    fn kisi_adi(&self, id: &str) -> String {
        self.rehber
            .kisiler
            .get(id)
            .map_or_else(|| id.to_string(), |k| k.ad.clone())
    }

    /// Oturum özeti (oturumsuz da çağrılabilir). Uyarılar yalnız girişliye gösterilir.
    pub fn oturum_ozeti(&self, k: Option<&Kullanici>) -> Value {
        let Some(k) = k else {
            return json!({ "girisli": false, "demo": self.demo });
        };
        let kisi = k.kisi_id();
        let etkin = kisi.as_deref().is_some_and(|i| self.kisi_etkin(i));
        json!({
            "girisli": true,
            "kullanici": k.ad,
            "yonetici": k.yonetici,
            "kisi_id": kisi,
            "kisi_etkin": etkin,
            "kisi_adi": kisi.as_deref().filter(|_| etkin).map(|i| self.kisi_adi(i)),
            "demo": self.demo,
            "uyarilar": self.uyarilar,
        })
    }

    /// Bir dersin ekran kaydı.
    fn ders_kaydi(&self, id: &str, kd: Option<&KisiDersi>) -> Option<Value> {
        let d = self.egitim.katalog.al(id)?;
        let vd = self.video_durumu(id);
        let kaynak = self.videolar.get(id);
        let (tur, src) = match (vd, kaynak.map(|k| (k.dogrula(), k))) {
            (VideoDurumu::Hazir, Some((Ok(KaynakTuru::Yerel), _))) => {
                (Some("yerel"), Some(format!("/video/{id}")))
            }
            (VideoDurumu::Hazir, Some((Ok(KaynakTuru::Url), k))) => (Some("url"), k.url.clone()),
            _ => (None, None),
        };
        let (atandi, durum, saniye, y, eski) = match kd {
            Some(x) => (
                x.atandi,
                durum_adi(x.durum, x.atandi),
                x.saniye,
                if x.durum == Durum::Izledi {
                    100
                } else {
                    yuzde(x.saniye, d.sure_sn)
                },
                x.eski_surum,
            ),
            None => (false, "baslanmadi", 0, 0, false),
        };
        Some(json!({
            "id": d.id,
            "baslik": d.baslik,
            "konu": d.konu,
            "konu_ad": veri::konu_adi(&d.konu, &self.konu_adlari),
            "gorevler": d.gorev_etiketleri.iter()
                .map(|g| json!({"kod": g, "ad": veri::gorev_adi(g, &self.rehber)}))
                .collect::<Vec<_>>(),
            "sure_sn": d.sure_sn,
            "surum": d.surum,
            "video": vd.ad(),
            "kaynak": tur,
            "src": src,
            "atandi": atandi,
            "durum": durum,
            "saniye": saniye,
            "yuzde": y,
            "eski_surum": eski,
        }))
    }

    fn videolar_listesi(&self, k: &Kullanici, sorgu: &str) -> Yanit {
        let suz = |ad: &str| {
            sorgu_degeri(sorgu, ad)
                .and_then(yuzde_coz)
                .filter(|v| !v.is_empty())
        };
        let (konu, gorev) = (suz("konu"), suz("gorev"));
        let kisi = k.kisi_id().filter(|i| self.kisi_etkin(i));
        let benim: BTreeMap<String, KisiDersi> = kisi
            .as_deref()
            .map(|i| {
                self.egitim
                    .kisi_dersleri(i)
                    .into_iter()
                    .map(|d| (d.ders_id.clone(), d))
                    .collect()
            })
            .unwrap_or_default();
        let liste: Vec<Value> = self
            .egitim
            .katalog
            .suz(konu.as_deref(), gorev.as_deref())
            .iter()
            .filter_map(|d| self.ders_kaydi(&d.id, benim.get(&d.id)))
            .collect();
        let konular: Vec<Value> = self
            .egitim
            .katalog
            .konular()
            .iter()
            .map(|id| json!({"kod": id, "ad": veri::konu_adi(id, &self.konu_adlari)}))
            .collect();
        let mut gorevler: Vec<&str> = self
            .egitim
            .katalog
            .hepsi()
            .iter()
            .flat_map(|d| d.gorev_etiketleri.iter().map(String::as_str))
            .collect();
        gorevler.sort_unstable();
        gorevler.dedup();
        let gorevler: Vec<Value> = gorevler
            .iter()
            .map(|g| json!({"kod": g, "ad": veri::gorev_adi(g, &self.rehber)}))
            .collect();
        (
            200,
            json!({
                "videolar": liste,
                "konular": konular,
                "gorevler": gorevler,
                "devam": kisi.as_deref().and_then(|i| self.devam_dersi(i, &benim)),
                "katalog_bos": self.egitim.katalog.is_empty(),
            }),
        )
    }

    /// "Kaldığın yerden devam et": son konum yazılan ders yarımsa o; değilse yarım kalan ilk ders.
    fn devam_dersi(&self, kisi: &str, benim: &BTreeMap<String, KisiDersi>) -> Option<Value> {
        let yarim = |id: &str| {
            benim
                .get(id)
                .is_some_and(|d| d.durum == Durum::Basladi && d.saniye > 0)
                && self.video_durumu(id) == VideoDurumu::Hazir
        };
        let id = self
            .son_ders
            .get(kisi)
            .filter(|i| yarim(i))
            .cloned()
            .or_else(|| benim.keys().find(|i| yarim(i)).cloned())?;
        let d = self.egitim.katalog.al(&id)?;
        Some(json!({"id": id, "baslik": d.baslik, "saniye": benim[&id].saniye}))
    }

    fn atamalarim(&self, k: &Kullanici) -> Yanit {
        let Some(kisi) = k.kisi_id().filter(|i| self.kisi_etkin(i)) else {
            return (200, json!({"atamalar": [], "kisi_etkin": false}));
        };
        let atamalar: Vec<Value> = self
            .egitim
            .kisi_dersleri(&kisi)
            .iter()
            .filter(|d| d.atandi)
            .filter_map(|d| self.ders_kaydi(&d.ders_id, Some(d)))
            .collect();
        (200, json!({"atamalar": atamalar, "kisi_etkin": true}))
    }

    /// Oturum sahibinin rehber kimliği (etkin olmalı) ya da hata yanıtı.
    fn etkin_kisi(&self, k: &Kullanici) -> Result<String, Yanit> {
        let Some(id) = k.kisi_id() else {
            return Err(hata(
                403,
                "Giriş adı rehber kimliğiyle eşleşmiyor. Öneri: yöneticiden rehber kaydını sorun.",
                7,
            ));
        };
        if !self.kisi_etkin(&id) {
            return Err(hata(
                403,
                "Rehberde etkin kaydınız yok. Öneri: yöneticiden rehber kaydını sorun.",
                7,
            ));
        }
        Ok(id)
    }

    fn video_hazir_mi(&self, ders: &str) -> Result<(), Yanit> {
        if self.egitim.katalog.al(ders).is_none() {
            return Err(hata(404, format!("ders yok: {ders}"), 2));
        }
        match self.video_durumu(ders) {
            VideoDurumu::Hazir => Ok(()),
            VideoDurumu::Yok => Err(hata(409, "Bu derse bağlı video yok; kayıt tutulmaz.", 4)),
            VideoDurumu::DosyaYok => Err(hata(409, "Video dosyası bulunamadı; kayıt tutulmaz.", 4)),
        }
    }

    fn konum(&mut self, k: &Kullanici, g: &Value, simdi: &str) -> Yanit {
        let kisi = match self.etkin_kisi(k) {
            Ok(i) => i,
            Err(y) => return y,
        };
        let Some(ders) = s(g, "ders") else {
            return girdi("ders gerekli");
        };
        let Some(saniye) = g.get("saniye").and_then(Value::as_u64) else {
            return girdi("saniye negatif olmayan tam sayı olmalı");
        };
        if let Err(y) = self.video_hazir_mi(ders) {
            return y;
        }
        // Katalogdaki süre gerçek videodan kısaysa son saniyeler hata vermesin: ders süresine kırpılır.
        let sure = self.egitim.katalog.al(ders).map_or(0, |d| d.sure_sn);
        let saniye = saniye.min(sure);
        let r = RehberVeri {
            veri: &self.rehber,
            bugun: crate::zaman::gun_kismi(simdi),
        };
        match self
            .egitim
            .konum_kaydet(&kisi, ders, saniye, &r, simdi, self.gunluk.as_mut())
        {
            Ok(degisti) => {
                self.son_ders.insert(kisi, ders.to_string());
                (
                    200,
                    json!({"tamam": true, "kaydedildi": degisti, "saniye": saniye}),
                )
            }
            Err(e) => egitim_hatasi(&e),
        }
    }

    fn izledim(&mut self, k: &Kullanici, g: &Value, simdi: &str) -> Yanit {
        let kisi = match self.etkin_kisi(k) {
            Ok(i) => i,
            Err(y) => return y,
        };
        let Some(ders) = s(g, "ders") else {
            return girdi("ders gerekli");
        };
        if let Err(y) = self.video_hazir_mi(ders) {
            return y;
        }
        let r = RehberVeri {
            veri: &self.rehber,
            bugun: crate::zaman::gun_kismi(simdi),
        };
        match self
            .egitim
            .izledim(&kisi, ders, &r, simdi, self.gunluk.as_mut())
        {
            Ok(yeni) => (200, json!({"tamam": true, "yeni": yeni})),
            Err(e) => egitim_hatasi(&e),
        }
    }

    // ------------------------------------------------------------------ yönetici uçları

    fn ozet(&self) -> Yanit {
        let o = match self.egitim.ozet(true) {
            Ok(o) => o,
            Err(e) => return egitim_hatasi(&e),
        };
        let baslik = |id: &str| {
            self.egitim
                .katalog
                .al(id)
                .map_or_else(|| id.to_string(), |d| d.baslik.clone())
        };
        let mut satirlar: Vec<(String, String, Value)> = o
            .satirlar
            .iter()
            .map(|r| {
                let (kad, dbas) = (self.kisi_adi(&r.kisi_id), baslik(&r.ders_id));
                let atandi = self.egitim.atandi_mi(&r.kisi_id, &r.ders_id);
                let v = json!({
                    "kisi_id": r.kisi_id,
                    "kisi_ad": kad,
                    "ders_id": r.ders_id,
                    "ders_baslik": dbas,
                    "atandi": atandi,
                    "durum": durum_adi(r.durum, atandi),
                    "yuzde": r.yuzde,
                    "eski_surum": r.eski_surum,
                });
                (kad.to_lowercase(), dbas.to_lowercase(), v)
            })
            .collect();
        satirlar.sort_by(|a, b| (&a.0, &a.1).cmp(&(&b.0, &b.1)));
        let dersler: Vec<Value> = o
            .dersler
            .iter()
            .map(|d| {
                json!({
                    "ders_id": d.ders_id,
                    "ders_baslik": baslik(&d.ders_id),
                    "atanan": d.atanan,
                    "basladi": d.basladi,
                    "izledi": d.izledi,
                })
            })
            .collect();
        (
            200,
            json!({
                "satirlar": satirlar.into_iter().map(|x| x.2).collect::<Vec<_>>(),
                "dersler": dersler,
                "bos": o.satirlar.is_empty(),
            }),
        )
    }

    fn yonetim(&self) -> Yanit {
        let mut kisiler: Vec<(String, Value)> = self
            .rehber
            .kisiler
            .values()
            .filter(|k| k.durum == KisiDurum::Etkin)
            .map(|k| (k.ad.to_lowercase(), json!({"id": k.id, "ad": k.ad})))
            .collect();
        kisiler.sort_by(|a, b| a.0.cmp(&b.0));
        let dersler: Vec<Value> = self
            .egitim
            .katalog
            .hepsi()
            .iter()
            .map(
                |d| json!({"id": d.id, "baslik": d.baslik, "video": self.video_durumu(&d.id).ad()}),
            )
            .collect();
        (
            200,
            json!({"kisiler": kisiler.into_iter().map(|x| x.1).collect::<Vec<_>>(), "dersler": dersler}),
        )
    }

    fn atama(&mut self, k: &Kullanici, g: &Value, iptal: bool, simdi: &str) -> Yanit {
        let atayan = match self.etkin_kisi(k) {
            Ok(i) => i,
            Err(y) => return y,
        };
        let (Some(kisi), Some(ders)) = (s(g, "kisi"), s(g, "ders")) else {
            return girdi("kisi ve ders gerekli");
        };
        let r = RehberVeri {
            veri: &self.rehber,
            bugun: crate::zaman::gun_kismi(simdi),
        };
        let sonuc = if iptal {
            self.egitim
                .atamayi_iptal_et(kisi, ders, &atayan, &r, simdi, self.gunluk.as_mut())
        } else {
            self.egitim
                .ata(kisi, ders, &atayan, &r, simdi, self.gunluk.as_mut())
        };
        match sonuc {
            Ok(degisti) => {
                let mesaj = match (iptal, degisti) {
                    (false, true) => "Atama yapıldı.",
                    (false, false) => "Bu atama zaten var.",
                    (true, true) => "Atama iptal edildi (kayıt günlükte kalır).",
                    (true, false) => "Bu kişiye bu eğitim atanmış değil.",
                };
                (200, json!({"tamam": true, "yeni": degisti, "mesaj": mesaj}))
            }
            Err(e) => egitim_hatasi(&e),
        }
    }

    fn gorevden_ata(&mut self, k: &Kullanici, g: &Value, simdi: &str) -> Yanit {
        let atayan = match self.etkin_kisi(k) {
            Ok(i) => i,
            Err(y) => return y,
        };
        let Some(kisi) = s(g, "kisi") else {
            return girdi("kisi gerekli");
        };
        let r = RehberVeri {
            veri: &self.rehber,
            bugun: crate::zaman::gun_kismi(simdi),
        };
        match self
            .egitim
            .gorevden_ata(kisi, &atayan, &r, simdi, self.gunluk.as_mut())
        {
            Ok(yeni) => {
                let mesaj = if yeni.is_empty() {
                    "Görevine uyan yeni bir eğitim yok.".to_string()
                } else {
                    format!("{} eğitim atandı.", yeni.len())
                };
                (200, json!({"tamam": true, "yeni": yeni, "mesaj": mesaj}))
            }
            Err(e) => egitim_hatasi(&e),
        }
    }

    // ------------------------------------------------------------------ görünüm

    fn gorunum_yaz(&mut self, g: &Value) -> Yanit {
        let Some(t) = s(g, "tema").and_then(Tema::adindan) else {
            return girdi("tema: acik, koyu ya da tek olmalı");
        };
        if let Some(y) = &self.tema_yolu {
            if let Err(e) = gorunum::yaz(y, t) {
                return hata(
                    500,
                    format!("Görünüm kaydedilemedi: {e}. Öneri: veri kökü yazılabilir mi bakın."),
                    2,
                );
            }
        }
        self.tema = t;
        (200, t.ozet())
    }

    // ------------------------------------------------------------------ yönlendirme

    /// Bir API isteğini işler. `yol` sorgusuz yoldur; `sorgu` `?` sonrasıdır.
    pub fn api(
        &mut self,
        k: Option<&Kullanici>,
        yontem: &str,
        yol: &str,
        sorgu: &str,
        govde: &Value,
        simdi: &str,
    ) -> Yanit {
        // `--demo`: oturum sahibi rehberde yoksa bellekte ÖRNEK kişi olarak eklenir.
        if self.demo {
            if let Some(id) = k.and_then(Kullanici::kisi_id) {
                veri::demo_oturum_sahibi_ekle(&mut self.rehber, &id);
            }
        }
        // Oturumsuz uçlar.
        match (yontem, yol) {
            ("GET", "/api/oturum") => return (200, self.oturum_ozeti(k)),
            ("GET", "/api/gorunum") => return (200, self.tema.ozet()),
            _ => {}
        }
        let Some(k) = k else {
            return if yol.starts_with("/api/") {
                oturum_yok()
            } else {
                hata(404, "uç yok", 2)
            };
        };
        let yonetici_uc = matches!(
            yol,
            "/api/ozet"
                | "/api/yonetim"
                | "/api/atama"
                | "/api/atama/iptal"
                | "/api/atama/gorevden"
        );
        if yonetici_uc && !k.yonetici {
            return yonetici_gerekli();
        }
        match (yontem, yol) {
            ("GET", "/api/videolar") => self.videolar_listesi(k, sorgu),
            ("GET", "/api/atamalarim") => self.atamalarim(k),
            ("POST", "/api/konum") => self.konum(k, govde, simdi),
            ("POST", "/api/izledim") => self.izledim(k, govde, simdi),
            ("GET", "/api/ozet") => self.ozet(),
            ("GET", "/api/yonetim") => self.yonetim(),
            ("POST", "/api/atama") => self.atama(k, govde, false, simdi),
            ("POST", "/api/atama/iptal") => self.atama(k, govde, true, simdi),
            ("POST", "/api/atama/gorevden") => self.gorevden_ata(k, govde, simdi),
            ("POST", "/api/gorunum") => self.gorunum_yaz(govde),
            (
                _,
                "/api/videolar" | "/api/atamalarim" | "/api/ozet" | "/api/yonetim" | "/api/gorunum",
            )
            | (
                _,
                "/api/konum"
                | "/api/izledim"
                | "/api/atama"
                | "/api/atama/iptal"
                | "/api/atama/gorevden",
            ) => hata(405, "yöntem desteklenmiyor", 2),
            _ => hata(404, "uç yok", 2),
        }
    }

    // ------------------------------------------------------------------ video

    /// `/video/<ders>`: oturum şart; yalnız yerel kaynaklı, dosyası var olan dersler. `range` istek başlığıdır.
    pub fn video_hazirla(
        &self,
        k: Option<&Kullanici>,
        ders: &str,
        range: Option<&str>,
    ) -> VideoYaniti {
        if k.is_none() {
            return VideoYaniti::Hata(oturum_yok());
        }
        let yok = || VideoYaniti::Hata(hata(404, "Video bulunamadı.", 2));
        let Some(kaynak) = self.videolar.get(ders) else {
            return yok();
        };
        let Some(goreli) = kaynak.yol.as_deref() else {
            return yok();
        };
        let Some(yol) = video::yerel_yol(&self.video_kok, goreli) else {
            return yok();
        };
        let Some(tur) = video::icerik_turu(goreli) else {
            return yok();
        };
        let Ok(meta) = std::fs::metadata(&yol) else {
            return yok();
        };
        if !meta.is_file() {
            return yok();
        }
        VideoYaniti::Dosya {
            yol,
            tur,
            karar: video::karar(range, meta.len()),
        }
    }
}
