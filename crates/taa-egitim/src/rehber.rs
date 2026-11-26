//! Rehbere salt okunur bakış. Eğitim kişi verisi (ad, e-posta) tutmaz; yalnız kimliğe ve görev kodlarına bakar.
//! Sahip: Yusuf. yazan: [Claude#egitim 2026-11-26]

use taa_rehber_model::model::{EtkinPasif, KisiDurum, Veri};

/// Rehberden okunan en az bilgi. Yazma ucu yoktur.
pub trait RehberOkur {
    /// Kişi rehberde var ve etkin mi?
    fn kisi_etkin(&self, id: &str) -> bool;
    /// Kişinin bugün etkin görevlerinin rol kodları (tekrarsız, sıralı).
    fn gorev_kodlari(&self, id: &str) -> Vec<String>;
}

/// `taa_rehber_model::model::Veri` üzerinde `RehberOkur`. `bugun`: `YYYY-AA-GG`.
pub struct RehberVeri<'a> {
    pub veri: &'a Veri,
    pub bugun: &'a str,
}

impl RehberOkur for RehberVeri<'_> {
    fn kisi_etkin(&self, id: &str) -> bool {
        self.veri
            .kisiler
            .get(id)
            .is_some_and(|k| k.durum == KisiDurum::Etkin)
    }

    fn gorev_kodlari(&self, id: &str) -> Vec<String> {
        let mut r: Vec<String> = self
            .veri
            .gorevlendirmeler
            .values()
            .filter(|g| {
                g.kisi_id == id
                    && g.durum == EtkinPasif::Etkin
                    && g.baslangic.as_str() <= self.bugun
                    && g.bitis.as_deref().is_none_or(|b| b >= self.bugun)
            })
            .map(|g| g.rol.clone())
            .collect();
        r.sort();
        r.dedup();
        r
    }
}
