//! Müşteri nüshasının yeniden basımı yetkiye bağlı (koordinatör kararı 2026-11-26, geri alınabilir varsayılan).
//!
//! Neden: müşteri nüshasındaki karekod fişli teslimin kanıtıdır (`teslim_sepet`). Mağaza cihazı müşteri nüshasını
//! yeniden yazdırabilirse kanıt mağaza içinde üretilebilir. Yönetici bunu cihaz bazında kapatabilir.
//!
//! - Yetki: `cihaz::YETKILER` içindeki "yeniden-yazdir"; varsayılan HER YERDE AÇIK (mağaza bilgisayarı ve telefonlar
//!   eskisi gibi yazdırır). Yönetici Telefonlar ekranındaki yetki anahtarlarından cihaz bazında kapatır/açar.
//! - "Verildi": bu fişin müşteri nüshası daha önce verildiyse (yazıcıya basıldı; Windows yazdırma penceresi için
//!   karekod `?basim=1` ile alındı; yetkisi kapalı cihaz karekodu ekranda gördü — bkz. aşağı) `basim.jsonl` günlüğünde
//!   kayıt vardır. Bu özellikten (ve fişli kanıttan) ÖNCE basılmış fişler (`eski_fis_gecis::YonetimAyari::eski_fis`)
//!   basım kaydı olmasa da "verilmiş" sayılır: yetkisi kapalı cihaz onların müşteri nüshasını/karekodunu hiç alamaz.
//!   Basım yapılıp günlük yazılamadıysa fiş bellekte "verildi" işaretlenir (yeniden başlatılana dek). İlk basım
//!   serbesttir.
//! - Yetki kapalıyken: müşteri nüshalı basım (`POST /api/fis/<no>/yazdir`, `nusha` verilmeden) ve müşteri nüshası
//!   karekodunun (kanıt parçalı personel karekodu, `GET /api/fis/<no>/karekod`) yeniden verilmesi 403 `yetki_yok`.
//!   Terzihane nüshası (`{"nusha":1}`; karekodunda yalnız fiş no var, kanıt değildir) etkilenmez.
//! - Yetki kapalı cihazda karekod ucu: `?basim=1` olmasa da her gösterim müşteri nüshasının VERİLMESİ sayılır (günlüğe
//!   `basim`, yöntem `karekod`; yazılamazsa karekod verilmez). Tek istisna önizleme: cihaz fişi bu süreçte KENDİSİ
//!   oluşturduysa, oluşturmadan sonraki `ONIZLEME_SN` boyunca karekodu günlüksüz görebilir (yeni fiş ekranı). Yetki
//!   kapalı cihazda ilk yazıcı basımı ÖNCE günlüğe yazılır (yazılamazsa basılmaz); yazıcı hatasında `basim_iptal`.
//! - Günlük (`basim.jsonl`, kişisel veri yok): her basım (`basim` / `yeniden_basim`), yeniden gösterim
//!   (`karekod_gosterim`), iptal (`basim_iptal`) ve reddedilen deneme (`yeniden_basim_reddedildi`): fiş no, nüsha,
//!   yöntem, kim, tarih, saat.
//! - Sınır: müşteri takip ayarlıyken takip bağlantısı da kanıttır. Hazır bildirimi kuyruğundaki (`GET /api/bildirimler`)
//!   takip bağlantısı yetkisi kapalı cihaza maskeli verilir (bkz. lib.rs). Müşterinin kendisine giden bildirim
//!   (WhatsApp) bağlantıyı taşır; o müşteri tarafıdır.
//!
//! Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

use super::*;

/// Basım günlüğü (veri klasöründe).
pub const GUNLUK: &str = "basim.jsonl";
pub const YETKI: &str = "yeniden-yazdir";
/// Yetkisi kapalı cihazın kendi oluşturduğu fişin karekodunu günlüksüz görebileceği süre (sn).
pub const ONIZLEME_SN: u64 = 600;

impl Uygulama {
    /// Bu fişin müşteri nüshası daha önce verildi mi? Basım günlüğünde (basım − iptal) > 0, bellekte işaretli ya da
    /// fiş, fişli kanıt mekanizmasından önce basılmış (basım kaydı olmasa da müşteriye verilmiştir).
    pub(crate) fn musteri_nushasi_verildi(&self, f: &taa_fis::Fis) -> bool {
        if eski_fis_gecis::YonetimAyari::oku(&self.dizin).eski_fis(f) {
            return true;
        }
        if self
            .basim_verildi_bellek
            .lock()
            .is_ok_and(|b| b.contains(&f.no))
        {
            return true;
        }
        let sayi: i64 = taa_dosya::satirlari_oku(&self.dizin.join(GUNLUK))
            .unwrap_or_default()
            .iter()
            .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
            .filter(|v| v["fis"] == f.no.as_str() && v["musteri"] == true)
            .map(|v| match v["olay"].as_str() {
                Some("basim" | "yeniden_basim") => 1,
                Some("basim_iptal") => -1,
                _ => 0,
            })
            .sum();
        sayi > 0
    }

    /// Yeni fiş oluşturuldu (önizleme penceresi için; yalnız bellekte).
    pub(crate) fn yeni_fis_kaydet(&self, no: &str, c: &cihaz::Cihaz) {
        let s = self.simdi();
        if let Ok(mut m) = self.yeni_fisler.lock() {
            m.retain(|_, (_, t)| s.saturating_sub(*t) < ONIZLEME_SN);
            m.insert(no.to_string(), (c.id.clone(), s));
        }
    }

    /// Cihaz bu fişi kendisi, son `ONIZLEME_SN` içinde oluşturdu mu?
    pub(crate) fn onizleme_serbest(&self, no: &str, c: &cihaz::Cihaz) -> bool {
        let s = self.simdi();
        self.yeni_fisler.lock().is_ok_and(|m| {
            m.get(no)
                .is_some_and(|(id, t)| *id == c.id && s.saturating_sub(*t) < ONIZLEME_SN)
        })
    }

    /// Basım yapıldı ama günlük yazılamadı: fiş bu süreç boyunca "verildi" sayılır.
    pub(crate) fn verildi_bellege(&self, no: &str) {
        if let Ok(mut b) = self.basim_verildi_bellek.lock() {
            b.insert(no.to_string());
        }
    }

    /// Basım günlüğüne satır ekler (tarih/saat eklenir). Yazılamazsa `Err`.
    pub(crate) fn basim_gunlugu(&self, mut v: serde_json::Value) -> Result<(), String> {
        let s = self.simdi();
        v["tarih"] = json!(tarih_iso(s));
        v["saat"] = json!(saat_iso(s));
        taa_dosya::satir_ekle(&self.dizin.join(GUNLUK), &v.to_string()).map_err(|e| {
            eprintln!("Basım günlüğü yazılamadı: {e}");
            e.to_string()
        })
    }

    /// Müşteri nüshasını (ya da karekodunu) vermeden önce: daha önce verildiyse "yeniden-yazdir" yetkisi gerekir.
    /// `Ok(yeniden)`; yetki yoksa reddi günlüğe yazar ve 403 `yetki_yok` döner.
    pub(crate) fn musteri_nushasi_izni(
        &self,
        f: &taa_fis::Fis,
        c: &cihaz::Cihaz,
        islem: &str,
    ) -> Result<bool, Yanit> {
        let no = f.no.as_str();
        let yeniden = self.musteri_nushasi_verildi(f);
        if yeniden && !self.cihazlar().yetki_var(&c.id, YETKI) {
            let _ = self.basim_gunlugu(json!({
                "olay": "yeniden_basim_reddedildi", "fis": no, "islem": islem,
                "cihaz": c.id, "kim": Self::denetim_kimi(c),
            }));
            return Err(Yanit::json(
                403,
                json!({
                    "hata": "yetki_yok",
                    "ileti": "Bu fişin müşteri nüshası daha önce verildi; bu cihazın müşteri nüshasını yeniden yazdırma yetkisi yok. Terzihane nüshası yazdırılabilir. Yönetici, Telefonlar ekranından yetki verebilir.",
                    "terzihane_yazdirilabilir": true,
                }),
            ));
        }
        Ok(yeniden)
    }

    /// Müşteri nüshası verildi: günlüğe `basim` ya da `yeniden_basim` yazar.
    pub(crate) fn musteri_nushasi_verildi_yaz(
        &self,
        no: &str,
        c: &cihaz::Cihaz,
        yeniden: bool,
        yontem: &str,
    ) -> Result<(), String> {
        self.basim_gunlugu(json!({
            "olay": if yeniden { "yeniden_basim" } else { "basim" },
            "fis": no, "musteri": true, "yontem": yontem,
            "cihaz": c.id, "kim": Self::denetim_kimi(c),
        }))
    }
}
