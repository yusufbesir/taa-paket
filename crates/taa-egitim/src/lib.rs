//! taa-egitim: TAA Eğitim çekirdeği (EG0; ana plan §4.7, mağaza işleyişi planı EG5).
//! Ders kataloğu (kasa eğitimi bir konudur), konuya ve göreve göre süzme, kişiye atama, "izledim" ve kaldığı saniye,
//! yalnız yöneticiye açık izleme özeti, yalnız-ekleme JSONL izleme günlüğü ve parça parça indirme için Range hesaplayıcı.
//! Ağ yok, saat yok (zamanı çağıran verir). Kişi verisi tutulmaz: yalnız rehberden gelen kişi kimliği saklanır;
//! rehbere yalnız `RehberOkur` ile salt okunur bakılır. İlk sürümde sınav yoktur.
//! Sahip: Yusuf. yazan: [Claude#egitim 2026-11-26]

pub mod aralik;
pub mod egitim;
pub mod gunluk;
pub mod katalog;
pub mod rehber;

pub use aralik::{aralik_coz, Aralik, AralikSonuc, ParcaPlani};
pub use egitim::{DersOzet, Durum, Egitim, KisiDersi, Ozet, OzetSatir};
pub use gunluk::{Gunluk, GunlukSatiri, Olay};
pub use katalog::{Ders, Katalog, KONU_KASA};
pub use rehber::{RehberOkur, RehberVeri};

pub const OZELLIK: &str = "egitim";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hata {
    /// Alan geçersiz (alan adı, neden).
    Gecersiz {
        alan: &'static str,
        neden: String,
    },
    DersYok(String),
    DersVar(String),
    /// Ders güncellemesinde sürüm artmadı.
    SurumGeri {
        ders: String,
        eski: u32,
        yeni: u32,
    },
    /// İzleme bildirimi dersin güncel sürümüne ait değil.
    SurumEski {
        ders: String,
        verilen: u32,
        guncel: u32,
    },
    /// Kaldığı saniye ders süresinden büyük.
    KonumAsimi {
        ders: String,
        saniye: u64,
        sure_sn: u64,
    },
    /// Kişi rehberde yok ya da etkin değil.
    KisiYok(String),
    /// İzleme özeti yalnız yönetici bayrağıyla istenebilir.
    YoneticiGerekli,
    /// Katalog ya da günlük sınırı aşıldı.
    Sinir(&'static str),
    /// Günlük dosyası hatası (G/Ç metni).
    Gunluk(String),
}

impl std::fmt::Display for Hata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Hata::Gecersiz { alan, neden } => write!(f, "{alan} geçersiz: {neden}"),
            Hata::DersYok(d) => write!(f, "ders yok: {d}"),
            Hata::DersVar(d) => write!(f, "ders zaten var: {d}"),
            Hata::SurumGeri { ders, eski, yeni } => {
                write!(f, "ders {ders}: sürüm artmalı (eski {eski}, yeni {yeni})")
            }
            Hata::SurumEski {
                ders,
                verilen,
                guncel,
            } => {
                write!(
                    f,
                    "ders {ders}: sürüm {verilen} eski, güncel sürüm {guncel}"
                )
            }
            Hata::KonumAsimi {
                ders,
                saniye,
                sure_sn,
            } => {
                write!(
                    f,
                    "ders {ders}: konum {saniye} sn süreyi ({sure_sn} sn) aşıyor"
                )
            }
            Hata::KisiYok(k) => write!(f, "kişi rehberde yok ya da etkin değil: {k}"),
            Hata::YoneticiGerekli => write!(f, "izleme özeti yalnız yöneticiye açık"),
            Hata::Sinir(s) => write!(f, "sınır aşıldı: {s}"),
            Hata::Gunluk(m) => write!(f, "günlük hatası: {m}"),
        }
    }
}

impl std::error::Error for Hata {}

pub(crate) fn gecersiz(alan: &'static str, neden: impl Into<String>) -> Hata {
    Hata::Gecersiz {
        alan,
        neden: neden.into(),
    }
}
