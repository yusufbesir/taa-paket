// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! TAA Çatı'ya tek imzalı gönderim raporu (`kasa.tek-imza@1`). Mağazada ikinci imza atılmadan gün sonu
//! gönderildiğinde kasa operasyonu DURMAZ; yalnız Çatı'ya bildirilmek üzere rapor kuyruğa alınır
//! (`<kasa>/cati/giden`). Çatı'nın rapor ucu henüz yok: `CatiUcuYok` yer tutucudur, rapor kuyrukta bekler,
//! uç gelince aynı kuyruk boşaltılır (silinmez, `gonderilen`e taşınır). Raporda tutar yoktur.

use crate::gunsonu::IKINCI_IMZA_ATMADI;
use crate::merkez::{Kuyruk, Tasiyici};
use crate::zaman;
use serde_json::{json, Value};
use std::path::Path;

pub const RAPOR_TURU: &str = "kasa.tek-imza@1";

/// Rapor ucu yokken dönen yer tutucu mesajı.
pub const CATI_UCU_YOK: &str = "TAA Çatı rapor ucu henüz yok; tek imza raporu kuyrukta bekliyor";

/// Çatı rapor kuyruğu (`<kasa>/cati`).
pub fn cati_kuyrugu(kasa: &Path) -> Kuyruk {
    Kuyruk {
        kok: kasa.join("cati"),
    }
}

/// Yer tutucu taşıyıcı: Çatı rapor ucu (sözleşme `kasa.tek-imza@1`) bağlanana kadar teslim etmez.
#[derive(Debug, Clone, Copy, Default)]
pub struct CatiUcuYok;

impl Tasiyici for CatiUcuYok {
    fn teslim(&self, _ad: &str, _bayt: &[u8]) -> Result<(), String> {
        Err(CATI_UCU_YOK.into())
    }
}

/// Gönderim paketi tek imzalıysa (ikinci imza atılmadı) Çatı raporunu üretir; iki imzalıysa `None`.
pub fn tek_imza_raporu(paket: &Value) -> Option<Value> {
    let ikinci = &paket["imzalar"][1];
    if ikinci["imzalamadi"] != true {
        return None;
    }
    let i = &paket["imzalar"][0];
    Some(json!({
        "tur": RAPOR_TURU,
        "surum": 1,
        "sube": paket["sube"],
        "tarih": paket["tarih"],
        "cekmece": paket["cekmece"],
        "gun_sonu_id": paket["gun_sonu"]["id"],
        "imzalayan": {
            "kisi": i["kisi"], "kisi_rol": i["kisi_rol"], "kanal": i["kanal"], "zaman": i["zaman"],
        },
        "ikinci_imza_durumu": IKINCI_IMZA_ATMADI,
        "gerekce": paket["ikinci_imza_gerekcesi"],
        "olusturma": zaman::simdi_sn(),
    }))
}

/// Paket tek imzalıysa raporu Çatı kuyruğuna alır; dönen değer kuyruktaki dosya adıdır. Aynı gün sonu için
/// yeniden çağrılırsa aynı ad yeniden yazılır (tek rapor).
pub fn raporla(kasa: &Path, paket: &Value) -> Result<Option<String>, String> {
    let Some(r) = tek_imza_raporu(paket) else {
        return Ok(None);
    };
    let id = r["gun_sonu_id"].as_str().unwrap_or("");
    if id.is_empty() {
        return Err("tek imza raporu: gün sonu kimliği yok".into());
    }
    let ad = format!("tek-imza-{id}.json");
    cati_kuyrugu(kasa).ekle(&ad, r.to_string().as_bytes())?;
    Ok(Some(ad))
}

#[cfg(test)]
mod testler {
    use super::*;

    fn paket(tek: bool) -> Value {
        json!({
            "tur": "kasa.gunsonu@1", "sube": "SB1", "tarih": "2026-11-26", "cekmece": "c1",
            "gun_sonu": {"id": "GS-20261126-c1-1", "ciro": {"nakit": 100}},
            "imzalar": [
                {"sira": 1, "kisi": "Müdür A", "kisi_rol": "mudur", "kanal": "ekran", "zaman": 1, "durum": "imzaladi"},
                if tek { json!({"sira": 2, "imzalamadi": true, "durum": "imza_atmadi"}) }
                else { json!({"sira": 2, "kisi": "Müdür B", "imzalamadi": false, "durum": "imzaladi"}) }
            ],
            "ikinci_imza_gerekcesi": if tek { json!("Vekil atanmadı: izin günü") } else { Value::Null },
        })
    }

    #[test]
    fn tek_imzali_gonderim_cati_kuyruguna_raporlanir_tutar_yazilmaz() {
        assert!(tek_imza_raporu(&paket(false)).is_none());
        let r = tek_imza_raporu(&paket(true)).unwrap();
        assert_eq!(r["tur"], RAPOR_TURU);
        assert_eq!(r["imzalayan"]["kisi_rol"], "mudur");
        assert_eq!(r["gerekce"], "Vekil atanmadı: izin günü");
        assert!(!r.to_string().contains("ciro"), "raporda tutar yok");
        let kasa = taa_dosya::gecici_klasor().unwrap().join("kasa-cati");
        assert_eq!(raporla(&kasa, &paket(false)).unwrap(), None);
        let ad = raporla(&kasa, &paket(true)).unwrap().unwrap();
        assert_eq!(ad, "tek-imza-GS-20261126-c1-1.json");
        let k = cati_kuyrugu(&kasa);
        // Uç yok: rapor kuyrukta bekler, kasa işlemi durmaz.
        assert_eq!(k.bosalt(&CatiUcuYok), (0, Some(CATI_UCU_YOK.into())));
        assert_eq!(k.bekleyenler(), vec![ad]);
    }
}
