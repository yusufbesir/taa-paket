//! Telegram hesap bağlama (yönetici) uçları ve tek kullanımlık WhatsApp yönlendirme sayfası (TT12c).
//! Yönetici uçları YALNIZ bu bilgisayardan çalışır (telefon yetkisiyle de olmaz). Çalışan kişisel verisi: yalnız yönetici görür.
//! Yazan: [Claude Sonnet 5.5, TT başlığı 2026-11-15].

use super::*;
use taa_telegram::hesap::{HesapDurum, Hesaplar, Rol};

fn kuculuk(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

impl Uygulama {
    /// GET /wa/<belirtec>: belirteç tek kullanımlıktır; müşteri numarası Telegram'dan geçmez, yalnız bu yerel ağdan gider.
    pub(crate) fn wa_sayfasi(&self, belirtec: &str) -> Yanit {
        match taa_telegram::wa_yonlendir(&self.dizin, belirtec, &tarih_iso(self.simdi())) {
            Ok(url) => Yanit::html(format!(
                "<!doctype html><meta charset=utf-8><meta name=viewport content=\"width=device-width,initial-scale=1\"><meta http-equiv=refresh content=\"0;url={u}\"><title>WhatsApp</title><body style=\"font-family:sans-serif;text-align:center;padding:2em\"><p>WhatsApp açılıyor…</p><p><a style=\"display:inline-block;padding:1em 2em;font-size:1.3em;background:#25d366;color:#fff;border-radius:.5em;text-decoration:none\" href=\"{u}\">WhatsApp'ı aç</a></p><p>Mesaj hazır gelir; Gönder'e siz basın.</p></body>",
                u = kuculuk(&url)
            )),
            Err(e) => {
                let mut y = Yanit::html(format!(
                    "<!doctype html><meta charset=utf-8><title>Bağlantı</title><body style=\"font-family:sans-serif;padding:2em\"><p>{}</p></body>",
                    kuculuk(&e)
                ));
                y.kod = 410;
                y
            }
        }
    }

    /// POST /api/yonetim/baglama-kodu {usta_id?, rol, sube, yonetici, omur_dk?}
    pub(crate) fn baglama_kodu(&self, i: &Istek) -> Yanit {
        if let Some(r) = self.yonetim_izni(i) {
            return r;
        }
        let Ok(v) = serde_json::from_slice::<serde_json::Value>(&i.govde) else {
            return Yanit::hata(400, "geçersiz istek");
        };
        let rol = match v["rol"].as_str().unwrap_or("usta") {
            "usta" => Rol::Usta,
            "personel" => Rol::Personel,
            "magaza" => Rol::Magaza,
            "yonetici" => Rol::Yonetici,
            _ => return Yanit::hata(422, "rol: usta, personel, magaza ya da yonetici"),
        };
        let omur = v["omur_dk"].as_u64().unwrap_or(10).clamp(1, 60) * 60;
        let simdi = self.simdi();
        let h = Hesaplar { dizin: &self.dizin };
        match h.kod_uret(
            v["usta_id"].as_str().unwrap_or(""),
            rol,
            v["sube"].as_str().unwrap_or(""),
            v["not"].as_str().unwrap_or(""),
            // Oturumdaki kişi biliniyorsa o (istemci beyanı yalnız yedek).
            &self
                .pc_kimi(i)
                .unwrap_or_else(|| v["yonetici"].as_str().unwrap_or("").to_string()),
            simdi,
            omur,
        ) {
            Ok((kod, usta_id)) => {
                // Yönetici QR'ı: telefonda Telegram botu `start=baglan-<kod>` ile açılır. Bot adı telegram.json'dadır.
                let bot_adi = taa_telegram::Ayar::dizinden(&self.dizin)
                    .map(|a| a.bot_adi)
                    .unwrap_or_default();
                let (baglanti, qr) = if bot_adi.is_empty() {
                    (String::new(), String::new())
                } else {
                    let b = taa_telegram::hesap::baglama_baglantisi(&bot_adi, &kod);
                    let svg = qrcode::QrCode::new(b.as_bytes())
                        .map(|q| {
                            q.render::<qrcode::render::svg::Color>()
                                .min_dimensions(260, 260)
                                .build()
                        })
                        .unwrap_or_default();
                    (b, svg)
                };
                Yanit::json(
                    200,
                    json!({"kod": kod, "usta_id": usta_id, "gecerli_sn": omur, "baglanti": baglanti, "qr_svg": qr}),
                )
            }
            Err(e) => Yanit::hata(422, &e),
        }
    }

    /// GET /api/yonetim/hesaplar
    pub(crate) fn hesap_listesi(&self, i: &Istek) -> Yanit {
        if let Some(r) = self.yonetim_izni(i) {
            return r;
        }
        match (Hesaplar { dizin: &self.dizin }).guncel() {
            Ok(v) => Yanit::json(200, json!({ "hesaplar": v })),
            Err(e) => Yanit::hata(500, &e),
        }
    }

    /// POST /api/yonetim/hesap/<tg_id>/onayla|askiya|kaldir  {yonetici}
    pub(crate) fn hesap_islemi(&self, i: &Istek, tg: &str, islem: &str) -> Yanit {
        if let Some(r) = self.yonetim_izni(i) {
            return r;
        }
        let Ok(tg_id) = tg.parse::<i64>() else {
            return Yanit::hata(400, "geçersiz hesap");
        };
        // Oturumdaki kişi biliniyorsa o (istemci beyanı yalnız yedek).
        let yonetici = self.pc_kimi(i).unwrap_or_else(|| {
            serde_json::from_slice::<serde_json::Value>(&i.govde)
                .ok()
                .and_then(|v| v["yonetici"].as_str().map(str::to_string))
                .unwrap_or_default()
        });
        if yonetici.trim().is_empty() {
            return Yanit::hata(422, "yönetici gerekli");
        }
        let (h, gun) = (Hesaplar { dizin: &self.dizin }, tarih_iso(self.simdi()));
        let r = match islem {
            "onayla" => h.onayla(tg_id, &yonetici, &gun).map(|_| ()),
            "askiya" => h.durum_ata(tg_id, HesapDurum::Askida, &yonetici, &gun),
            "kaldir" => h.durum_ata(tg_id, HesapDurum::Kaldirildi, &yonetici, &gun),
            "yeniden-ac" => h.durum_ata(tg_id, HesapDurum::Onayli, &yonetici, &gun),
            _ => return Yanit::hata(404, "bulunamadı"),
        };
        match r {
            Ok(()) => Yanit::json(200, json!({"tamam": true})),
            Err(e) => Yanit::hata(422, &e),
        }
    }
}
