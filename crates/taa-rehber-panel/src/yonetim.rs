//! Yönetici formlarının sunucu tarafı: kişi, birim, görevlendirme ekleme ve (sürüm denetimli) düzeltme.
//! Her yazma `taa_rehber_model::model::Depo` üzerinden gider (ya hep ya hiç, silme yok). Beklenen sürüm güncel değilse
//! 409 `cakisma` döner (sözleşme §3). Kimlikler (`id`) sunucuda üretilir; istemciden alınmaz.
//!
//! DÜRÜST SINIR: gerçek yazar sunucudaki `rehber.yaz@1` hizmetidir ve bu sürümde bağlı değildir; yazılanlar yalnız bu
//! sürecin belleğindedir (pencere kapanınca gider). Yetki denetimi (`rehber.yaz`) yönlendiricidedir; bu modüle yalnız
//! yönetici oturumu doğrulanmış istekler gelir.
//! Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]

use crate::durum::{yeni_kimlik, Durum, Ortam};
use crate::suzgec::{tur_etiketi, tur_kodu};
use crate::{zaman, Yanit};
use serde_json::{json, Value};
use taa_rehber_model::model::*;

fn hata(durum: u16, kod: &str, m: impl Into<String>) -> Yanit {
    (durum, json!({"hata": m.into(), "kod": kod}))
}

fn alan_hatasi(alan: &str, m: &str) -> Yanit {
    (400, json!({"hata": m, "kod": "gecersiz", "alan": alan}))
}

/// Model hatası → HTTP. Sürüm çakışması ve tekrar eden kayıt 409; yok olan kayıt 404; kural ihlali 422.
pub fn model_hatasi(e: ModelHata) -> Yanit {
    let m = e.to_string();
    match e {
        ModelHata::Cakisma {
            guncel, beklenen, ..
        } => (
            409,
            json!({"hata": m, "kod": "cakisma", "guncel_surum": guncel, "beklenen_surum": beklenen}),
        ),
        ModelHata::KimlikVar { .. }
        | ModelHata::EpostaTekrar(_)
        | ModelHata::GorevlendirmeTekrar(_)
        | ModelHata::AsilCakisma(_) => hata(409, "tekrar", m),
        ModelHata::Bulunamadi { .. } => hata(404, "bulunamadi", m),
        _ => hata(422, "gecersiz", m),
    }
}

/// Zorunlu metin alanı (kırpılmış, boş olamaz).
fn gerekli(g: &Value, ad: &str) -> Result<String, Yanit> {
    match g.get(ad) {
        Some(Value::String(s)) if !s.trim().is_empty() => Ok(s.trim().to_string()),
        Some(Value::String(_)) | None | Some(Value::Null) => {
            Err(alan_hatasi(ad, &format!("{ad} gerekli")))
        }
        Some(_) => Err(alan_hatasi(ad, &format!("{ad} metin olmalı"))),
    }
}

/// İsteğe bağlı metin: yok/boş/null → None.
fn istege_bagli(g: &Value, ad: &str) -> Result<Option<String>, Yanit> {
    match g.get(ad) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok((!s.trim().is_empty()).then(|| s.trim().to_string())),
        Some(_) => Err(alan_hatasi(ad, &format!("{ad} metin olmalı"))),
    }
}

fn beklenen(g: &Value) -> Result<u64, Yanit> {
    g.get("beklenen_surum")
        .and_then(Value::as_u64)
        .ok_or_else(|| alan_hatasi("beklenen_surum", "beklenen_surum gerekli (tam sayı)"))
}

fn birim_turu(s: &str) -> Option<BirimTur> {
    match s {
        "sube" => Some(BirimTur::Sube),
        "magaza" => Some(BirimTur::Magaza),
        "departman" => Some(BirimTur::Departman),
        "merkez" => Some(BirimTur::Merkez),
        _ => None,
    }
}

fn gorev_turu(s: &str) -> Option<GorevTur> {
    match s {
        "asil" => Some(GorevTur::Asil),
        "gecici" => Some(GorevTur::Gecici),
        "vekil" => Some(GorevTur::Vekil),
        _ => None,
    }
}

fn kimlik_uret(o: &dyn Ortam, onek: &str) -> Result<String, Yanit> {
    yeni_kimlik(o, onek).map_err(|m| hata(500, "sistem", m))
}

/// Form seçenekleri: etkin kişi/birim/rol. Yazma izni sahibi kayıt seçebilsin diye kimlik ve ADI görür; e-posta yok.
pub fn secenekler(d: &Durum) -> Value {
    let v = d.depo.veri();
    json!({
        "kisiler": v.kisiler.values().filter(|k| k.durum == KisiDurum::Etkin)
            .map(|k| json!({"id": k.id, "ad": k.ad, "surum": k.surum})).collect::<Vec<_>>(),
        "birimler": v.birimler.values().filter(|b| b.durum == BirimDurum::Etkin)
            .map(|b| json!({"id": b.id, "ad": b.ad, "tur": tur_kodu(b.tur), "tur_etiket": tur_etiketi(b.tur), "surum": b.surum}))
            .collect::<Vec<_>>(),
        "roller": v.roller.values().filter(|r| r.durum == EtkinPasif::Etkin)
            .map(|r| json!({"kod": r.kod, "ad": r.ad})).collect::<Vec<_>>(),
    })
}

pub fn kisi_ekle(d: &mut Durum, o: &dyn Ortam, g: &Value) -> Yanit {
    let (ad, eposta) = match (gerekli(g, "ad"), gerekli(g, "eposta")) {
        (Ok(a), Ok(e)) => (a, e),
        (Err(y), _) | (_, Err(y)) => return y,
    };
    let id = match kimlik_uret(o, "kisi") {
        Ok(i) => i,
        Err(y) => return y,
    };
    let z = zaman::zaman(o.simdi());
    let k = KisiKaydi {
        id: id.clone(),
        ad,
        eposta,
        durum: KisiDurum::Etkin,
        pasif_tarihi: None,
        surum: 1,
        guncelleme: z.clone(),
    };
    match d.depo.kisi_ekle(k, &z) {
        Ok(s) => {
            d.denetim_yaz(&z, "rehber.kisi", "ekle", &id);
            (201, json!({"tamam": true, "id": id, "surum": s}))
        }
        Err(e) => model_hatasi(e),
    }
}

pub fn kisi_duzelt(d: &mut Durum, o: &dyn Ortam, g: &Value) -> Yanit {
    let (id, ad, eposta, bek) = match (
        gerekli(g, "id"),
        gerekli(g, "ad"),
        gerekli(g, "eposta"),
        beklenen(g),
    ) {
        (Ok(i), Ok(a), Ok(e), Ok(b)) => (i, a, e, b),
        (Err(y), ..) | (_, Err(y), ..) | (_, _, Err(y), _) | (_, _, _, Err(y)) => return y,
    };
    let Some(mut k) = d.depo.veri().kisiler.get(&id).cloned() else {
        return hata(404, "bulunamadi", "kişi bulunamadı");
    };
    k.ad = ad;
    k.eposta = eposta;
    let z = zaman::zaman(o.simdi());
    match d.depo.kisi_yaz(k, bek, &z) {
        Ok(s) => {
            d.denetim_yaz(&z, "rehber.kisi", "duzelt", &id);
            (200, json!({"tamam": true, "id": id, "surum": s}))
        }
        Err(e) => model_hatasi(e),
    }
}

/// Birim alanlarını gövdeden okur (yalnız gövdede bulunan isteğe bağlı alanlar değişir; boş değer alanı temizler).
fn birim_alanlari(b: &mut BirimKaydi, g: &Value) -> Result<(), Yanit> {
    if g.get("ust_birim_id").is_some() {
        b.ust_birim_id = istege_bagli(g, "ust_birim_id")?;
    }
    if g.get("sube_kodu").is_some() {
        b.sube_kodu = istege_bagli(g, "sube_kodu")?;
    }
    if g.get("ortak_eposta").is_some() {
        b.ortak_eposta = istege_bagli(g, "ortak_eposta")?;
    }
    Ok(())
}

pub fn birim_ekle(d: &mut Durum, o: &dyn Ortam, g: &Value) -> Yanit {
    let ad = match gerekli(g, "ad") {
        Ok(a) => a,
        Err(y) => return y,
    };
    let Some(tur) = g.get("tur").and_then(Value::as_str).and_then(birim_turu) else {
        return alan_hatasi("tur", "tür: sube, magaza, departman ya da merkez olmalı");
    };
    let id = match kimlik_uret(o, "birim") {
        Ok(i) => i,
        Err(y) => return y,
    };
    let z = zaman::zaman(o.simdi());
    let mut b = BirimKaydi {
        id: id.clone(),
        ad,
        tur,
        ust_birim_id: None,
        sube_kodu: None,
        ortak_eposta: None,
        durum: BirimDurum::Etkin,
        surum: 1,
        guncelleme: z.clone(),
    };
    if let Err(y) = birim_alanlari(&mut b, g) {
        return y;
    }
    match d.depo.birim_ekle(b, &z) {
        Ok(s) => {
            d.denetim_yaz(&z, "rehber.birim", "ekle", &id);
            (201, json!({"tamam": true, "id": id, "surum": s}))
        }
        Err(e) => model_hatasi(e),
    }
}

pub fn birim_duzelt(d: &mut Durum, o: &dyn Ortam, g: &Value) -> Yanit {
    let (id, ad, bek) = match (gerekli(g, "id"), gerekli(g, "ad"), beklenen(g)) {
        (Ok(i), Ok(a), Ok(b)) => (i, a, b),
        (Err(y), ..) | (_, Err(y), _) | (_, _, Err(y)) => return y,
    };
    let Some(mut b) = d.depo.veri().birimler.get(&id).cloned() else {
        return hata(404, "bulunamadi", "birim bulunamadı");
    };
    b.ad = ad;
    if let Err(y) = birim_alanlari(&mut b, g) {
        return y;
    }
    let z = zaman::zaman(o.simdi());
    match d.depo.birim_yaz(b, bek, &z) {
        Ok(s) => {
            d.denetim_yaz(&z, "rehber.birim", "duzelt", &id);
            (200, json!({"tamam": true, "id": id, "surum": s}))
        }
        Err(e) => model_hatasi(e),
    }
}

pub fn gorevlendirme_ekle(d: &mut Durum, o: &dyn Ortam, g: &Value) -> Yanit {
    let (kisi_id, birim_id, rol) = match (
        gerekli(g, "kisi_id"),
        gerekli(g, "birim_id"),
        gerekli(g, "rol"),
    ) {
        (Ok(k), Ok(b), Ok(r)) => (k, b, r),
        (Err(y), ..) | (_, Err(y), _) | (_, _, Err(y)) => return y,
    };
    let tur = match g.get("tur") {
        None | Some(Value::Null) => GorevTur::Asil,
        Some(Value::String(s)) => match gorev_turu(s) {
            Some(t) => t,
            None => return alan_hatasi("tur", "tür: asil, gecici ya da vekil olmalı"),
        },
        Some(_) => return alan_hatasi("tur", "tür metin olmalı"),
    };
    let z = zaman::zaman(o.simdi());
    let baslangic = match istege_bagli(g, "baslangic") {
        Ok(Some(b)) => b,
        Ok(None) => zaman::tarih(o.simdi()),
        Err(y) => return y,
    };
    let (gorev, bitis, amir_id, vekil) = match (
        istege_bagli(g, "gorev"),
        istege_bagli(g, "bitis"),
        istege_bagli(g, "amir_id"),
        istege_bagli(g, "vekil_edilen_id"),
    ) {
        (Ok(a), Ok(b), Ok(c), Ok(v)) => (a, b, c, v),
        (Err(y), ..) | (_, Err(y), ..) | (_, _, Err(y), _) | (_, _, _, Err(y)) => return y,
    };
    let id = match kimlik_uret(o, "gorev") {
        Ok(i) => i,
        Err(y) => return y,
    };
    let kayit = GorevlendirmeKaydi {
        id: id.clone(),
        kisi_id,
        birim_id,
        rol,
        gorev,
        amir_id,
        tur,
        vekil_edilen_id: vekil,
        durum: EtkinPasif::Etkin,
        baslangic,
        bitis,
        surum: 1,
        guncelleme: z.clone(),
    };
    match d.depo.gorevlendirme_ekle(kayit, &z) {
        Ok(s) => {
            d.denetim_yaz(&z, "rehber.gorevlendirme", "ekle", &id);
            (201, json!({"tamam": true, "id": id, "surum": s}))
        }
        Err(e) => model_hatasi(e),
    }
}
