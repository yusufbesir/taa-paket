//! Telegram'dan kargo girişi: `/kargo FİŞNO` (+ serbest metin), "Şunu anladım, onaylıyor musunuz?" düğmeleri,
//! `/takip FİŞNO NO FİRMA`, `/kargoozet FİŞNO`. Yalnız `magaza` ve `yonetici` rolü (mağaza telefonu) kullanır.
//! Taslak `kargo-taslak.json` (hesap başına), kayıt `kargo.jsonl`: ikisi de YALNIZ yerel dosyadır, buluta gitmez.
//! yazan: [Claude#terzitakip-kargo 2026-11-26].

use crate::kargo::{self, Cozum, Kargo};
use crate::{gonder, kapsamli_bul, Bot, Kimlik};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use taa_fis::Defter;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Taslak {
    kargo: Kargo,
    #[serde(default)]
    adres_sonra: bool,
}

const YARDIM_KARGO: &str = "Kargo girişi:\n/kargo FİŞNO  → ardından bilgileri yazın (her satır bir bilgi ya da WhatsApp'a yazdığınız gibi serbest metin):\nAlıcı adı, telefon, ürünler (2 takım elbise (yelekli bej), 2 pantolon (mavi)), adres, ilçe il ya da \"Adres iletecek\".\nSonra Onayla düğmesi. Takip no için: /takip FİŞNO 1234567890 yurtici\n/takip FİŞNO teslim  → kargo teslim edildi\n/kargoozet FİŞNO  → WhatsApp'a yapıştırılacak özet";

impl Bot<'_> {
    fn taslak_yolu(&self) -> std::path::PathBuf {
        self.dizin.join("kargo-taslak.json")
    }

    fn taslaklar(&self) -> BTreeMap<String, Taslak> {
        std::fs::read_to_string(self.taslak_yolu())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn taslaklari_yaz(&self, m: &BTreeMap<String, Taslak>) -> Result<(), String> {
        let s = serde_json::to_string(m).map_err(|e| e.to_string())?;
        std::fs::write(self.taslak_yolu(), s).map_err(|e| format!("taslak yazılamadı: {e}"))
    }

    fn magaza_yetkisi(&self, sohbet: i64, k: &Kimlik) -> Option<Value> {
        (!k.magaza_mi()).then(|| gonder(sohbet, "Kargo girişi yalnız mağaza telefonu içindir.", None))
    }

    /// Taslağın "Şunu anladım" kartı: eksik varsa yalnız İptal, tamamsa Onayla ve İptal.
    fn taslak_karti(&self, sohbet: i64, t: &Taslak) -> Value {
        let eksik = kargo::eksikler(&t.kargo, t.adres_sonra);
        let mut metin = format!("Şunu anladım:\n{}", kargo::anlasilan(&t.kargo, t.adres_sonra));
        let dugmeler = if eksik.is_empty() {
            metin += "\n\nOnaylıyor musunuz? Düzeltmek için eksik ya da yanlış satırı yeniden yazın (ör. \"Telefon: 0532 …\").";
            json!([[{"text": "Onayla", "callback_data": "kg:ok"}, {"text": "İptal", "callback_data": "kg:iptal"}]])
        } else {
            metin += &format!("\n\nEksik: {}. Yazın, eklenecek.", eksik.join(", "));
            json!([[{"text": "İptal", "callback_data": "kg:iptal"}]])
        };
        gonder(sohbet, &metin, Some(dugmeler))
    }

    fn birlestir(taban: &mut Taslak, c: Cozum) {
        let y = c.kargo;
        let (a, k) = (&mut taban.kargo, y);
        for (hedef, kaynak) in [
            (&mut a.alici_ad, k.alici_ad),
            (&mut a.alici_tel, k.alici_tel),
            (&mut a.adres, k.adres),
            (&mut a.ilce, k.ilce),
            (&mut a.il, k.il),
            (&mut a.firma, k.firma),
            (&mut a.takip_no, k.takip_no),
        ] {
            if !kaynak.trim().is_empty() {
                *hedef = kaynak.trim().to_string();
            }
        }
        if !k.urunler.is_empty() {
            a.urunler = k.urunler;
        }
        if c.adres_sonra {
            taban.adres_sonra = true;
        }
        if !taban.kargo.adres.is_empty() {
            taban.adres_sonra = false;
        }
    }

    /// `/kargo FİŞNO [metin…]`
    pub(crate) fn kargo_baslat(&self, sohbet: i64, d: &Defter, no: &str, metin: &str, k: &Kimlik) -> Value {
        if let Some(r) = self.magaza_yetkisi(sohbet, k) {
            return r;
        }
        let fis = match kapsamli_bul(d, no, k) {
            Ok(Some(f)) => f,
            Ok(None) => return gonder(sohbet, &format!("Hata: fiş yok: {no}"), None),
            Err(e) => return gonder(sohbet, &format!("Hata: {e}"), None),
        };
        let rest = metin.trim().splitn(3, char::is_whitespace).nth(2).unwrap_or("").trim().to_string();
        let mut t = Taslak {
            kargo: kargo::son(self.dizin, &fis.no).unwrap_or(Kargo { fis_no: fis.no.clone(), ..Default::default() }),
            adres_sonra: false,
        };
        let mut m = self.taslaklar();
        if rest.is_empty() {
            m.insert(k.tg_id.to_string(), t.clone());
            if let Err(e) = self.taslaklari_yaz(&m) {
                return gonder(sohbet, &format!("Hata: {e}"), None);
            }
            return gonder(
                sohbet,
                &format!(
                    "{} için kargo bilgisini yazın (tek mesaj, her satır bir bilgi):\nAlıcı adı\nTelefon\nÜrünler (ör. 2 takım elbise (yelekli bej), 2 pantolon (mavi))\nAdres, ilçe il ya da \"Adres iletecek\"",
                    fis.no
                ),
                None,
            );
        }
        Self::birlestir(&mut t, kargo::coz(&rest));
        m.insert(k.tg_id.to_string(), t.clone());
        if let Err(e) = self.taslaklari_yaz(&m) {
            return gonder(sohbet, &format!("Hata: {e}"), None);
        }
        self.taslak_karti(sohbet, &t)
    }

    /// Komut olmayan metin: açık bir kargo taslağı varsa ona eklenir (yoksa None; bot "Anlamadım" der).
    pub(crate) fn kargo_serbest(&self, sohbet: i64, metin: &str, k: &Kimlik) -> Option<Value> {
        if !k.magaza_mi() {
            return None;
        }
        let mut m = self.taslaklar();
        let t = m.get_mut(&k.tg_id.to_string())?;
        Self::birlestir(t, kargo::coz(metin));
        let t = t.clone();
        if let Err(e) = self.taslaklari_yaz(&m) {
            return Some(gonder(sohbet, &format!("Hata: {e}"), None));
        }
        Some(self.taslak_karti(sohbet, &t))
    }

    /// `kg:ok` ve `kg:iptal` düğmeleri.
    pub(crate) fn kargo_dugmesi(&self, islem: &str, k: &Kimlik, sohbet: i64, cevap: &dyn Fn(&str) -> Value) -> Vec<Value> {
        if !k.magaza_mi() {
            return vec![cevap("Kargo girişi yalnız mağaza telefonu içindir.")];
        }
        let mut m = self.taslaklar();
        let Some(t) = m.remove(&k.tg_id.to_string()) else {
            return vec![cevap("Açık kargo taslağı yok. /kargo FİŞNO ile başlayın.")];
        };
        if islem != "ok" {
            let _ = self.taslaklari_yaz(&m);
            return vec![cevap("Vazgeçildi.")];
        }
        let eksik = kargo::eksikler(&t.kargo, t.adres_sonra);
        if !eksik.is_empty() {
            return vec![cevap(&format!("Eksik: {}", eksik.join(", ")))];
        }
        let mut kayit = t.kargo.clone();
        if kayit.durum.is_empty() {
            kayit.durum = "hazirlaniyor".into();
        }
        kayit.guncelleme = self.bugun.to_string();
        kayit.kaydeden = k.imza_adi();
        if let Err(e) = kargo::kaydet(self.dizin, &kayit).and_then(|_| self.taslaklari_yaz(&m)) {
            return vec![cevap(&format!("Hata: {e}"))];
        }
        let ozet = self.kargo_ozet_metni(&kayit, t.adres_sonra);
        vec![
            cevap("Kargo bilgisi kaydedildi."),
            gonder(
                sohbet,
                &format!("Kaydedildi.\n\n{ozet}\n\nTakip numarası gelince: /takip {} NUMARA FİRMA", kayit.fis_no),
                None,
            ),
        ]
    }

    fn kargo_ozet_metni(&self, kayit: &Kargo, adres_sonra: bool) -> String {
        let yol = self.dizin.join("fisler.jsonl");
        let d = Defter { yol: &yol };
        let (ad, tel) = d
            .bul(&kayit.fis_no)
            .ok()
            .flatten()
            .map(|f| (f.musteri, f.telefon))
            .unwrap_or_default();
        kargo::whatsapp_ozeti(kayit, &ad, &tel, adres_sonra, self.ayar.kargo_tam_ozet)
    }

    /// `/takip FİŞNO NO [FİRMA]` ya da `/takip FİŞNO teslim`
    pub(crate) fn kargo_takip(&self, sohbet: i64, d: &Defter, no: &str, a2: &str, a3: &str, k: &Kimlik) -> Value {
        if let Some(r) = self.magaza_yetkisi(sohbet, k) {
            return r;
        }
        let fis = match kapsamli_bul(d, no, k) {
            Ok(Some(f)) => f,
            Ok(None) => return gonder(sohbet, &format!("Hata: fiş yok: {no}"), None),
            Err(e) => return gonder(sohbet, &format!("Hata: {e}"), None),
        };
        let Some(mut kayit) = kargo::son(self.dizin, &fis.no) else {
            return gonder(sohbet, &format!("Hata: {} için kargo bilgisi yok. Önce /kargo {}", fis.no, fis.no), None);
        };
        if a2.is_empty() {
            return gonder(sohbet, "Kullanım: /takip FİŞNO TAKİPNO FİRMA  ya da  /takip FİŞNO teslim", None);
        }
        if a2.eq_ignore_ascii_case("teslim") {
            kayit.durum = "teslim".into();
        } else {
            if !kargo::takip_no_gecerli(a2) {
                return gonder(sohbet, "Hata: takip numarası 4-40 karakter, yalnız harf, rakam ve tire olmalı.", None);
            }
            if !a3.is_empty() {
                match kargo::firma_anahtari(a3) {
                    Some(f) => kayit.firma = f.to_string(),
                    None => {
                        let l: Vec<&str> = kargo::FIRMALAR.iter().map(|(a, _)| *a).collect();
                        return gonder(sohbet, &format!("Hata: firma anlaşılmadı. Şunlardan biri: {}", l.join(", ")), None);
                    }
                }
            }
            if kayit.firma.is_empty() {
                return gonder(sohbet, "Hata: firma gerekli: /takip FİŞNO NO yurtici (aras, mng, ptt, surat, ups, dhl…)", None);
            }
            kayit.takip_no = a2.to_string();
            kayit.durum = "yolda".into();
            kayit.kargo_tarihi = self.bugun.to_string();
        }
        kayit.guncelleme = self.bugun.to_string();
        kayit.kaydeden = k.imza_adi();
        match kargo::kaydet(self.dizin, &kayit) {
            Ok(()) => gonder(
                sohbet,
                &format!(
                    "{}: {} {}{}. Müşteri takip sayfası birkaç dakika içinde güncellenir.",
                    kayit.fis_no,
                    if kayit.durum == "teslim" { "kargo teslim edildi" } else { "kargolandı" },
                    kargo::firma_adi(&kayit.firma),
                    if kayit.takip_no.is_empty() { String::new() } else { format!(" · {}", kayit.takip_no) }
                ),
                None,
            ),
            Err(e) => gonder(sohbet, &format!("Hata: {e}"), None),
        }
    }

    /// `/kargoozet FİŞNO`
    pub(crate) fn kargo_ozet(&self, sohbet: i64, d: &Defter, no: &str, k: &Kimlik) -> Value {
        if let Some(r) = self.magaza_yetkisi(sohbet, k) {
            return r;
        }
        match kapsamli_bul(d, no, k) {
            Ok(Some(f)) => match kargo::son(self.dizin, &f.no) {
                Some(kayit) => gonder(sohbet, &self.kargo_ozet_metni(&kayit, false), None),
                None => gonder(sohbet, &format!("Hata: {} için kargo bilgisi yok. Önce /kargo {}", f.no, f.no), None),
            },
            Ok(None) => gonder(sohbet, &format!("Hata: fiş yok: {no}"), None),
            Err(e) => gonder(sohbet, &format!("Hata: {e}"), None),
        }
    }

    pub(crate) fn kargo_yardimi() -> &'static str {
        YARDIM_KARGO
    }
}
