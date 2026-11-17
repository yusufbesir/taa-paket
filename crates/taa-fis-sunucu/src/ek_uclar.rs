//! Masaüstü program ve yeni telefon sayfası için fiş yardımcıları: oturum bilgisi, kişi denetimi,
//! adların çözülmesi, sunucuda üretilen karekod (SVG) ve yazıcı denemesi.

use super::*;
use std::collections::BTreeMap;

/// Telefon "bağlı" sayılır: son bu kadar saniyede istek attıysa.
const BAGLI_SN: u64 = 120;

impl Uygulama {
    /// Fiş karekodunun içeriği: telefon sayfasında fişi açan adres.
    pub(crate) fn fis_karekod_adresi(&self, no: &str) -> String {
        format!("{}/telefon#fis={no}", self.taban_url.trim_end_matches('/'))
    }

    /// GET /api/oturum: giriş türü (pc | tel | kullanici) ve kişisel telefonda kişi.
    pub(crate) fn oturum(&self, c: &cihaz::Cihaz) -> Yanit {
        let sube = self.sube_kodu().unwrap_or_default();
        let giris = if c.id == PC_ID {
            "pc"
        } else if c.kisi_id.is_empty() {
            "tel"
        } else {
            "kullanici"
        };
        let kisi = (!c.kisi_id.is_empty()).then(|| {
            json!({
                "id": c.kisi_id,
                "ad": kisi::ad_coz(&self.dizin, &sube, &c.kisi_id).unwrap_or_else(|| c.kisi_id.clone()),
            })
        });
        Yanit::json(
            200,
            json!({
                "giris": giris,
                "cihaz": { "id": c.id, "ad": c.ad },
                "kisi": kisi,
                "sube": sube,
            }),
        )
    }

    /// GET /api/durum: alt çubuk için şube, yazıcı ve bağlı telefon sayısı.
    pub(crate) fn genel_durum(&self) -> Yanit {
        let simdi = self.simdi();
        let (sube, sube_hata) = match self.sube_kodu() {
            Ok(k) => (k, String::new()),
            Err(_) => (String::new(), self.sube_yok_iletisi()),
        };
        let telefonlar = self.cihazlar().aktifler().unwrap_or_default();
        let bagli = self
            .son_gorulme
            .lock()
            .map(|m| {
                telefonlar
                    .iter()
                    .filter(|c| {
                        m.get(&c.id)
                            .is_some_and(|t| simdi.saturating_sub(*t) < BAGLI_SN)
                    })
                    .count()
            })
            .unwrap_or(0);
        Yanit::json(
            200,
            json!({
                "sube": sube,
                "sube_hata": sube_hata,
                "yazici": self.yazici_yolu.is_some(),
                "telefon": telefonlar.len(),
                "bagli_telefon": bagli,
                "tarih": tarih_iso(simdi),
                "saat": saat_iso(simdi),
            }),
        )
    }

    /// Üç kişi alanı zorunlu ve kişi kaydında açık olmalı. Kişisel telefonda (kullanıcı girişi) düzenleyen
    /// her zaman giren kişidir; istemcinin gönderdiği değer yok sayılır. Fişe ad değil kimlik yazılır.
    pub(crate) fn kisileri_denetle(&self, g: &mut Girdi, c: &cihaz::Cihaz) -> Result<(), Yanit> {
        if !c.kisi_id.is_empty() {
            g.duzenleyen_id = c.kisi_id.clone();
        }
        let sube = self.sube_kodu().unwrap_or_default();
        let acik: Vec<String> = kisi::secilebilir(&self.dizin, &sube)
            .into_iter()
            .map(|k| k.id)
            .collect();
        if acik.is_empty() {
            return Err(Yanit::json(
                422,
                json!({ "hata": self.kisi_bos_iletisi(), "kisi_kaydi_bos": true }),
            ));
        }
        let mut eksik = Vec::new();
        for (id, ad) in [
            (&mut g.satis_danismani_id, "Satış danışmanı"),
            (&mut g.olcu_alan_id, "Ölçü alan"),
            (&mut g.duzenleyen_id, "Fişi düzenleyen"),
        ] {
            *id = id.trim().to_string();
            if !acik.contains(id) {
                eksik.push(ad);
            }
        }
        if !eksik.is_empty() {
            return Err(Yanit::json(
                422,
                json!({
                    "hata": format!("Kaydetmek için seçin: {}.", eksik.join(", ")),
                    "eksik": eksik,
                }),
            ));
        }
        g.satis_danismani.clear();
        g.olcu_alan.clear();
        Ok(())
    }

    /// Yazdırma ve düz metin için kimliklerden adları doldurur (eski fişlerde ad zaten yazılı).
    pub(crate) fn adlari_doldur(&self, f: &mut taa_fis::Fis) {
        let h = kisi::ad_haritasi(&self.dizin, &self.sube_kodu().unwrap_or_default());
        for (ad, id) in [
            (&mut f.satis_danismani, &f.satis_danismani_id),
            (&mut f.olcu_alan, &f.olcu_alan_id),
        ] {
            if let Some(a) = h.get(id) {
                *ad = a.clone();
            }
        }
    }
}

/// Fiş JSON'u: kimlikler korunur, ad alanları kişi kaydından doldurulur, `duzenleyen` adı eklenir.
pub(crate) fn fis_json(f: taa_fis::Fis, h: &BTreeMap<String, String>) -> serde_json::Value {
    let kalan = f.kalan_kurus();
    let ad = |id: &str| h.get(id).cloned();
    let dan = ad(&f.satis_danismani_id);
    let olc = ad(&f.olcu_alan_id);
    let duz = ad(&f.duzenleyen_id).unwrap_or_default();
    let mut v = json!(f);
    if let Some(a) = dan {
        v["satis_danismani"] = json!(a);
    }
    if let Some(a) = olc {
        v["olcu_alan"] = json!(a);
    }
    v["duzenleyen"] = json!(duz);
    v["kalan_kurus"] = json!(kalan);
    v
}

/// Karekodu tek renkli SVG olarak üretir (sessiz bölge 2 modül, `currentColor`; fişte siyah basılır).
pub fn karekod_svg(icerik: &str) -> Result<String, String> {
    let q =
        qrcode::QrCode::new(icerik.as_bytes()).map_err(|e| format!("karekod üretilemedi: {e}"))?;
    let w = q.width();
    let renkler = q.to_colors();
    let kenar = 2;
    let boy = w + 2 * kenar;
    let mut yol = String::new();
    for (n, r) in renkler.iter().enumerate() {
        if *r == qrcode::Color::Dark {
            let (x, y) = (n % w + kenar, n / w + kenar);
            yol.push_str(&format!("M{x} {y}h1v1h-1z"));
        }
    }
    Ok(format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {boy} {boy}\" shape-rendering=\"crispEdges\" role=\"img\" aria-label=\"Karekod\"><path fill=\"currentColor\" d=\"{yol}\"/></svg>"
    ))
}

/// Yazıcı denemesi: kısa ASCII metin, besleme ve kesim.
pub(crate) fn deneme_baytlari() -> Vec<u8> {
    let mut b = vec![0x1B, 0x40, 0x1B, 0x61, 1];
    b.extend_from_slice(b"TAA Terzi Takip\nYazici denemesi\n80 mm / 72 mm basilabilir alan\n");
    b.extend_from_slice(&tarih_iso(simdi()).into_bytes());
    b.extend_from_slice(b"\n\n\n\n");
    b.extend_from_slice(&[0x1D, 0x56, 0x42, 0]);
    b
}
