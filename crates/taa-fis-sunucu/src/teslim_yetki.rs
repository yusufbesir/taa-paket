//! Teslim bilgisinin maskeli/açık görünümü ve "teslim-acik" yetkisi (Yusuf 2026-11-26: "daha sonra maskeli olarak
//! görünsün. yalnızca yöneticiler ve mağaza müdürleri görebilsin. bu ayarları yine yönetici değiştirebilsin.
//! yetkilendirilebilir bir karar olsun.").
//!
//! - Ekranlarda teslim bilgisi varsayılan MASKELİDİR: ad "M*** Y***", telefon son 4 hane dışında gizli, IP hiç
//!   gösterilmez, okutan cihazın yalnız özeti ("Android 14 · Chrome"). İmza görüntüsü maskeli görünümde YOKTUR.
//! - Açık görünüm (`GET /api/fis/<no>/teslim-acik`: tam ad, tam telefon, IP, ham user-agent; imza görüntüsü
//!   `GET /api/fis/<no>/imza`) yalnız `teslim_acik_yetkisi` olan istekte verilir:
//!   1. mağaza bilgisayarında yönetici (PIN) oturumu (yönetici kilidi kapalıysa bu bilgisayar yönetici sayılır);
//!   2. cihaza yöneticinin yazdığı "teslim-acik" kaydı (ver/geri al; mağaza bilgisayarı "PC" dahil);
//!   3. kişisel telefonda, kişiye yöneticinin yazdığı kayıt (`kisi:<id>`) — yalnız kişi hâlâ AÇIK (seçilebilir)
//!      listedeyse; kapatılan / listeden düşen kişinin eski kaydı geçersizdir (kapatınca kayıt da geri alınır);
//!   4. kayıt yoksa kişinin rolü: "yonetici" ya da "magaza-muduru" (cihaz::TESLIM_ACIK_ROLLER) varsayılan açık —
//!      YALNIZ rolü yöneticinin bu bilgisayarda verdiği kişide (kaynak "" ya da "ilk-kurulum"). TAA Çatı'dan
//!      (unvan metninden tahmin) ya da Telegram'dan gelen rol kendiliğinden yetki VERMEZ; yönetici kişi bazında
//!      açıkça verir (denetçi bulgusu 2026-11-26: "müdür yardımcısı", "bölge müdürü" yetki almasın).
//!
//!   Mağaza telefonu (kişisiz) ve diğer roller varsayılan kapalıdır.
//! - Her açık görüntüleme (ve imza görüntüleme) `teslim-ayrinti.jsonl` denetim günlüğüne yazılır: olay, fiş no,
//!   kim (cihaz kimliği / kişi kimliği / "PC"), hangi yetkiyle, tarih-saat. Günlükte teslim alanın kişisel verisi
//!   (ad, telefon, IP, cihaz) YOKTUR.
//!
//! Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

use super::*;

/// Açık görünüm yetkisinin kaynağı (denetim günlüğüne ve yönetim ekranına yazılır).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum AcikKaynak {
    /// Mağaza bilgisayarında yönetici oturumu.
    Yonetici,
    /// Cihaza verilen yetki kaydı.
    Cihaz,
    /// Kişiye verilen yetki kaydı.
    Kisi,
    /// Kişinin rolü (yönetici / mağaza müdürü).
    Rol,
}

impl AcikKaynak {
    pub(crate) fn ad(self) -> &'static str {
        match self {
            AcikKaynak::Yonetici => "yonetici-oturumu",
            AcikKaynak::Cihaz => "cihaz-yetkisi",
            AcikKaynak::Kisi => "kisi-yetkisi",
            AcikKaynak::Rol => "rol",
        }
    }
}

impl Uygulama {
    /// Kişi hâlâ açık (seçilebilir) listede ise kaydı; kapatılmış ya da listeden düşmüşse `None`.
    fn acik_kisi(&self, kisi_id: &str) -> Option<kisi::Kisi> {
        kisi::secilebilir(&self.dizin, &self.sube_kodu().unwrap_or_default())
            .into_iter()
            .find(|k| k.id == kisi_id)
    }

    /// Oturumdan bağımsız (kalıcı) "teslim-acik" kararı: cihaz kaydı > kişi kaydı > rol varsayılanı > kapalı.
    /// Kişi yolu (kayıt ve rol) yalnız kişi açık listede iken geçerlidir.
    pub(crate) fn teslim_acik_kalici(&self, cihaz_id: &str, kisi_id: &str) -> Option<AcikKaynak> {
        let k = self.cihazlar();
        if let Some(v) = k.yetki_kaydi(cihaz_id, "teslim-acik") {
            return v.then_some(AcikKaynak::Cihaz);
        }
        if kisi_id.is_empty() {
            return None;
        }
        let kisi = self.acik_kisi(kisi_id)?;
        if let Some(v) = k.yetki_kaydi(&cihaz::kisi_anahtari(kisi_id), "teslim-acik") {
            return v.then_some(AcikKaynak::Kisi);
        }
        cihaz::rol_varsayilani_acik(&kisi.rol, &kisi.kaynak).then_some(AcikKaynak::Rol)
    }

    /// Bu istek teslim bilgisinin AÇIK hâlini görebilir mi? (yönetici oturumu dahil)
    pub(crate) fn teslim_acik_yetkisi(&self, i: &Istek, c: &cihaz::Cihaz) -> Option<AcikKaynak> {
        if c.id == PC_ID && i.yerel && self.yonetim_izni(i).is_none() {
            return Some(AcikKaynak::Yonetici);
        }
        self.teslim_acik_kalici(&c.id, &c.kisi_id)
    }

    /// Maskeli özeti görebilen cihaz: mağaza bilgisayarı, fişsiz teslim / düzeltme ya da teslim-acik yetkisi olan.
    /// Diğerleri (ör. usta telefonu) yalnız "şifreli" işaretini görür.
    pub(crate) fn teslim_ozet_gorebilir(&self, i: &Istek, c: &cihaz::Cihaz) -> bool {
        c.id == PC_ID || {
            let k = self.cihazlar();
            k.yetki_var(&c.id, "fissiz-teslim")
                || k.yetki_var(&c.id, "duzelt")
                || self.teslim_acik_yetkisi(i, c).is_some()
        }
    }

    /// Cihazın etkin yetkileri (oturum ekranı ve yönetim listesi): "teslim-acik" kişi/rol kuralıyla hesaplanır.
    pub(crate) fn etkin_yetkiler(&self, i: Option<&Istek>, c: &cihaz::Cihaz) -> Vec<&'static str> {
        let mut v: Vec<&'static str> = self
            .cihazlar()
            .yetkileri(&c.id)
            .into_iter()
            .filter(|y| *y != "teslim-acik")
            .collect();
        let acik = match i {
            Some(i) => self.teslim_acik_yetkisi(i, c).is_some(),
            None => self.teslim_acik_kalici(&c.id, &c.kisi_id).is_some(),
        };
        if acik {
            v.push("teslim-acik");
        }
        v
    }

    /// Denetim günlüğündeki "kim": cihaz kimliği (+ kişisel telefonda kişi kimliği). Kişisel veri değildir.
    pub(crate) fn denetim_kimi(c: &cihaz::Cihaz) -> String {
        if c.kisi_id.is_empty() {
            c.id.clone()
        } else {
            format!("{} / {}", c.id, c.kisi_id)
        }
    }

    /// Maskeli görünüm (her teslim kaydı için; şifreli ise çözülüp maskelenir, eski düz metin kayıtta da ad maskelenir).
    pub(crate) fn teslim_maskeli(&self, no: &str, t: &taa_fis::TeslimKaydi) -> serde_json::Value {
        let mut v = json!({ "maskeli": true });
        if let Some(d) = self.teslim_coz(no, t) {
            v["ad"] = json!(taa_fis::ad_maskele(&d.ad));
            v["teslim_alan"] = json!(taa_fis::ad_maskele(&d.ad));
            v["tel_maskeli"] = json!(teslim_sifre::tel_maskele(&d.tel));
            v["cihaz"] = json!(teslim_sifre::cihaz_ozeti(&d.cihaz));
            v["zaman"] = json!(d.zaman);
            v["yontem"] = json!(d.yontem);
        } else if !t.teslim_alan.is_empty() {
            v["ad"] = json!(taa_fis::ad_maskele(&t.teslim_alan));
            v["teslim_alan"] = json!(taa_fis::ad_maskele(&t.teslim_alan));
        }
        v
    }

    /// GET /api/fis/<no>/teslim-acik: teslim bilgisinin AÇIK hâli (tam ad, tam telefon, IP, ham user-agent).
    /// Yetki `teslim_acik_yetkisi`; her başarılı görüntüleme denetim günlüğüne yazılır (kişisel veri olmadan).
    /// Mağaza bilgisayarında oturum yoksa 401 yonetici_gerekli (ekran PIN ister); diğer cihazda 403 yetki_yok.
    pub(crate) fn teslim_acik(&self, d: &Defter, no: &str, c: &cihaz::Cihaz, i: &Istek) -> Yanit {
        let Some(kaynak) = self.teslim_acik_yetkisi(i, c) else {
            if c.id == PC_ID {
                return Yanit::json(
                    401,
                    json!({ "hata": "yonetici_gerekli", "ileti": "Teslim bilgisinin açık hâli için yönetici girişi gerekli." }),
                );
            }
            return Yanit::json(
                403,
                json!({
                    "hata": "yetki_yok",
                    "ileti": "Bu cihaz teslim bilgisini yalnız maskeli görebilir. Yönetici, mağaza bilgisayarındaki Telefonlar ekranından yetki verebilir.",
                }),
            );
        };
        let f = match d.bul(no) {
            Ok(Some(f)) => f,
            Ok(None) => return Yanit::hata(404, "fiş yok"),
            Err(e) => return Yanit::hata(500, &e),
        };
        let Some(t) = f.teslim_kaydi.as_ref() else {
            return Yanit::hata(404, "bu fişin teslim kaydı yok");
        };
        let fis_tel = teslim_sifre::tel_normal(&f.telefon).unwrap_or_default();
        let mut v = json!({
            "iliski": t.iliski,
            "fisli": t.fisli,
            "dogrulama": t.dogrulama,
            "imza_yontemi": t.imza_yontemi,
            "imzali": !t.imza_sha256.is_empty(),
            "kim": t.kim,
            "tarih": t.tarih,
            "saat": t.saat,
            "ana_fis": t.ana_fis,
            "sifreli": t.sifreli.is_some(),
        });
        match t.sifreli.as_ref() {
            Some(s) => {
                let duz = match teslim_sifre::coz(&self.dizin, no, s) {
                    Ok(x) => x,
                    Err(e) => return Yanit::hata(500, &e),
                };
                v["ad"] = json!(duz.ad);
                v["tel"] = json!(duz.tel);
                v["fis_telefonu"] = json!(if duz.tel.is_empty() || fis_tel.is_empty() {
                    "yok"
                } else if duz.tel == fis_tel {
                    "ayni"
                } else {
                    "farkli"
                });
                v["ip"] = json!(duz.ip);
                v["cihaz"] = json!(duz.cihaz);
                v["cihaz_ozeti"] = json!(teslim_sifre::cihaz_ozeti(&duz.cihaz));
                v["zaman"] = json!(duz.zaman);
                v["yontem"] = json!(duz.yontem);
            }
            // Geçişten önceki eski düz metin kayıt (geçiş çalışınca kalmaz).
            None => v["ad"] = json!(t.teslim_alan),
        }
        if let Err(y) = self.teslim_gunlugu(json!({
            "olay": "teslim_acik",
            "fis": no,
            "kim": Self::denetim_kimi(c),
            "yetki": kaynak.ad(),
        })) {
            return y;
        }
        Yanit::json(200, v)
    }

    /// POST /api/yonetim/kisi/<id>/yetki {yetki:"teslim-acik", ver}: yönetici kişiye yetki verir/geri alır
    /// (rol varsayılanını geri almak dahil). Yalnız bu bilgisayardan ve yönetici oturumuyla.
    pub(crate) fn kisi_yetki(&self, i: &Istek, id: &str) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let Some(ver) = v["ver"].as_bool() else {
            return Yanit::hata(400, "ver: true ya da false");
        };
        let yetki = v["yetki"].as_str().unwrap_or("");
        let sube = self.sube_kodu().unwrap_or_default();
        if !kisi::secilebilir(&self.dizin, &sube)
            .iter()
            .any(|k| k.id == id)
        {
            return Yanit::hata(404, "açık kişi yok");
        }
        let s = self.simdi();
        match self
            .cihazlar()
            .kisi_yetki_ata(id, yetki, ver, &tarih_iso(s))
        {
            Ok(()) => {
                let satir = json!({"olay": "kisi_yetki", "kisi": id, "yetki": yetki, "ver": ver, "tarih": tarih_iso(s), "saat": saat_iso(s)}).to_string();
                let _ = taa_dosya::satir_ekle(&self.dizin.join("eslestirme.jsonl"), &satir);
                Yanit::json(
                    200,
                    json!({ "tamam": true, "teslim_acik": self.teslim_acik_kalici("", id).is_some() }),
                )
            }
            Err(e) => Yanit::hata(422, &e),
        }
    }

    /// Yönetim listesi için kişiler ve "teslim-acik" durumu (kaynağıyla).
    pub(crate) fn kisi_yetki_listesi(&self) -> Vec<serde_json::Value> {
        let sube = self.sube_kodu().unwrap_or_default();
        let k = self.cihazlar();
        kisi::secilebilir(&self.dizin, &sube)
            .into_iter()
            .map(|x| {
                let kayit = k.yetki_kaydi(&cihaz::kisi_anahtari(&x.id), "teslim-acik");
                let rol_acik = cihaz::rol_varsayilani_acik(&x.rol, &x.kaynak);
                let acik = kayit.unwrap_or(rol_acik);
                json!({
                    "id": x.id, "ad": x.ad, "rol": x.rol, "rol_kaynagi": x.kaynak,
                    "teslim_acik": acik,
                    "kaynak": match kayit { Some(_) => "yonetici", None if rol_acik => "rol", None => "varsayilan" },
                    "rol_varsayilani": rol_acik,
                })
            })
            .collect()
    }
}
