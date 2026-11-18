// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz 2026-11-17]
//! taa-kur-tema: TAA Kurucu arayüzünün tema çekirdeği.
//!
//! Bağlayıcı kaynak: `arayuz-modernlesme/taa-kurucu/ekip/KARAR-FABLE.md` (3 tema) ve
//! `logo/LOGO-KARAR-FABLE.md` §2 (Geçme Temel). Bu crate yalnız veriyi ve hesabı taşır; çizim (GDI)
//! kullanan taraftadır (taa-kur, kurulum). Bağımlılık yoktur, yalnız std.
//!
//! - [`Tema`], [`Mod`], [`Jetonlar`]: altı palet (üç tema x açık/koyu), HEX'ler KARAR'dan birebir.
//! - [`Geometri`], [`Tipografi`], [`Hareket`]: tema başına ölçüler ve hareket karakteri.
//! - [`kontrast`], [`ustune_yazi`], [`vurgu_duzelt`]: WCAG 2.x kontrast koruması (KARAR §5).
//! - [`Gorunum`]: `<veri kökü>\kurucu\gorunum.json` tercih dosyası (elle JSON, atomik yazma).
//! - [`etkin_jetonlar`]: tercih + sistem durumu -> kullanılacak tema ve renkler.
//! - [`logo`]: Geçme Temel simgesinin çokgenleri ve SVG metni.

pub mod gorunum;
pub mod kontrast;
pub mod logo;
pub mod olcu;
pub mod simge;

pub use gorunum::{gorunum_yolu, veri_koku, Gorunum, GORUNUM_SURUM};
pub use kontrast::{
    ara_renk, bagil_parlaklik, hataya_yakin, kontrast, ustune_yazi, vurgu_duzelt,
    vurgu_metin_olabilir, Red, ADIM_ORANI, EN_COK_ADIM,
};
pub use olcu::{Geometri, Hareket, Tipografi, Yazi, KALIP_OTURMA_PX, TIK_MS, YUKLEME_TIK};

/// Üç tema. Sıra (ve [`Tema::sira`]) Ayarlar ekranındaki sıradır; varsayılan İlmek.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Tema {
    #[default]
    Ilmek,
    Kalip,
    Mezura,
}

impl Tema {
    /// Ayarlar sırasıyla tüm temalar.
    pub const HEPSI: [Tema; 3] = [Tema::Ilmek, Tema::Kalip, Tema::Mezura];

    /// Dosyada ve kodda kullanılan ASCII ad: `ilmek`, `kalip`, `mezura`.
    pub fn ad(self) -> &'static str {
        match self {
            Tema::Ilmek => "ilmek",
            Tema::Kalip => "kalip",
            Tema::Mezura => "mezura",
        }
    }

    /// Arayüzde görünen Türkçe ad.
    pub fn gorunen_ad(self) -> &'static str {
        match self {
            Tema::Ilmek => "İlmek",
            Tema::Kalip => "Kalıp",
            Tema::Mezura => "Mezura",
        }
    }

    /// 0, 1, 2 (dizi indisi olarak kullanılabilir).
    pub fn sira(self) -> usize {
        match self {
            Tema::Ilmek => 0,
            Tema::Kalip => 1,
            Tema::Mezura => 2,
        }
    }

    /// [`Tema::ad`]'ın tersi (büyük/küçük harf ve baştaki/sondaki boşluk önemsiz).
    pub fn addan(s: &str) -> Option<Tema> {
        let s = s.trim().to_ascii_lowercase();
        Tema::HEPSI.into_iter().find(|t| t.ad() == s)
    }

    /// Temanın açık ya da koyu paleti (KARAR §2.1, §3.1, §4.1).
    pub fn jetonlar(self, koyu: bool) -> Jetonlar {
        match (self, koyu) {
            (Tema::Ilmek, false) => ILMEK_ACIK,
            (Tema::Ilmek, true) => ILMEK_KOYU,
            (Tema::Kalip, false) => KALIP_ACIK,
            (Tema::Kalip, true) => KALIP_KOYU,
            (Tema::Mezura, false) => MEZURA_ACIK,
            (Tema::Mezura, true) => MEZURA_KOYU,
        }
    }

    /// Ölçüler (yarıçap, kenar, boşluk, satır/düğme yüksekliği).
    pub fn geometri(self) -> Geometri {
        olcu::geometri(self)
    }

    /// Yazı boyları ve ağırlıkları.
    pub fn tipografi(self) -> Tipografi {
        olcu::tipografi(self)
    }

    /// Geçiş hareketinin karakteri.
    pub fn hareket(self) -> Hareket {
        match self {
            Tema::Ilmek => Hareket::RenkGecisi,
            Tema::Kalip => Hareket::Oturma,
            Tema::Mezura => Hareket::Anlik,
        }
    }

    /// Tanıtım kartının otomatik dönme aralığı (ms): 6000 / 7200 / 8400.
    pub fn tanitim_ms(self) -> u32 {
        match self {
            Tema::Ilmek => 6000,
            Tema::Kalip => 7200,
            Tema::Mezura => 8400,
        }
    }

    /// Tanıtım kartı değişiminin adım sayısı (İlmek 3 adımlı solma, Kalıp 2 adımlı kayma, Mezura anlık).
    pub fn tanitim_adim(self) -> u8 {
        match self {
            Tema::Ilmek => 3,
            Tema::Kalip => 2,
            Tema::Mezura => 0,
        }
    }

    /// Bu temada vurgu dolgusu her zaman `kenar` çerçeveyle mi çizilmeli? (Mezura açık, KARAR §4.1 ve §5.5)
    pub fn vurgu_cerceveli(self, koyu: bool) -> bool {
        self == Tema::Mezura && !koyu
    }
}

/// Görünüm modu. Varsayılan bilerek Açık (KARAR §5: 15 şubede aynı görünüm).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Mod {
    #[default]
    Acik,
    Koyu,
    Sistem,
}

impl Mod {
    pub const HEPSI: [Mod; 3] = [Mod::Acik, Mod::Koyu, Mod::Sistem];

    /// Dosyadaki ad: `acik`, `koyu`, `sistem`.
    pub fn ad(self) -> &'static str {
        match self {
            Mod::Acik => "acik",
            Mod::Koyu => "koyu",
            Mod::Sistem => "sistem",
        }
    }

    /// Ayarlar ekranındaki etiket.
    pub fn gorunen_ad(self) -> &'static str {
        match self {
            Mod::Acik => "Açık",
            Mod::Koyu => "Koyu",
            Mod::Sistem => "Sistemi izle",
        }
    }

    pub fn addan(s: &str) -> Option<Mod> {
        let s = s.trim().to_ascii_lowercase();
        Mod::HEPSI.into_iter().find(|m| m.ad() == s)
    }

    /// Koyu palet mi kullanılacak? `Sistem` ise Windows'un uygulama modu (`sistem_koyu`) izlenir.
    pub fn koyu_mu(self, sistem_koyu: bool) -> bool {
        match self {
            Mod::Acik => false,
            Mod::Koyu => true,
            Mod::Sistem => sistem_koyu,
        }
    }
}

/// Bir paletin on bir jetonu. Renkler `0xRRGGBB`; GDI için [`colorref`] ya da [`Jetonlar::cr`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Jetonlar {
    pub zemin: u32,
    pub yuzey: u32,
    pub yuzey2: u32,
    pub kenar: u32,
    pub metin: u32,
    pub soluk: u32,
    pub vurgu: u32,
    pub vurgu_ustu: u32,
    pub basari: u32,
    pub uyari: u32,
    pub hata: u32,
}

impl Jetonlar {
    /// Jetonu GDI COLORREF'e (0x00BBGGRR) çevirir: `j.cr(j.metin)`.
    pub fn cr(&self, rgb: u32) -> u32 {
        colorref(rgb)
    }

    /// (ad, renk) çiftleri; test, döküm ve önizleme için.
    pub fn liste(&self) -> [(&'static str, u32); 11] {
        [
            ("zemin", self.zemin),
            ("yuzey", self.yuzey),
            ("yuzey2", self.yuzey2),
            ("kenar", self.kenar),
            ("metin", self.metin),
            ("soluk", self.soluk),
            ("vurgu", self.vurgu),
            ("vurgu_ustu", self.vurgu_ustu),
            ("basari", self.basari),
            ("uyari", self.uyari),
            ("hata", self.hata),
        ]
    }
}

/// `0xRRGGBB` -> GDI COLORREF `0x00BBGGRR`.
pub const fn colorref(rgb: u32) -> u32 {
    let r = (rgb >> 16) & 0xFF;
    let g = (rgb >> 8) & 0xFF;
    let b = rgb & 0xFF;
    (b << 16) | (g << 8) | r
}

/// GDI COLORREF `0x00BBGGRR` -> `0xRRGGBB` (ChooseColor sonucu için).
pub const fn colorref_rgb(cr: u32) -> u32 {
    colorref(cr)
}

/// `0xRRGGBB` -> `"#RRGGBB"`.
pub fn hex(rgb: u32) -> String {
    format!("#{:06X}", rgb & 0xFF_FFFF)
}

/// `"#RRGGBB"` ya da `"RRGGBB"` -> `0xRRGGBB`.
pub fn hex_oku(s: &str) -> Option<u32> {
    let s = s.trim();
    let s = s.strip_prefix('#').unwrap_or(s);
    if s.len() != 6 || !s.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    u32::from_str_radix(s, 16).ok()
}

// --- Paletler (KARAR-FABLE.md, birebir) ---

pub const ILMEK_ACIK: Jetonlar = Jetonlar {
    zemin: 0xF6F1E7,
    yuzey: 0xFFFDF8,
    yuzey2: 0xEBE3D3,
    kenar: 0x7A7166,
    metin: 0x2A2320,
    soluk: 0x5C5148,
    vurgu: 0x6A2C5A,
    vurgu_ustu: 0xFFFFFF,
    basari: 0x1F6338,
    uyari: 0x7A4E00,
    hata: 0xA3262F,
};

pub const ILMEK_KOYU: Jetonlar = Jetonlar {
    zemin: 0x1B1618,
    yuzey: 0x251E21,
    yuzey2: 0x31282C,
    kenar: 0x94858C,
    metin: 0xF3ECEE,
    soluk: 0xC2B4BA,
    vurgu: 0xE3A9D2,
    vurgu_ustu: 0x2A1024,
    basari: 0x9FDAAE,
    uyari: 0xE8BE74,
    hata: 0xFFA6A8,
};

pub const KALIP_ACIK: Jetonlar = Jetonlar {
    zemin: 0xEDF2F8,
    yuzey: 0xFFFFFF,
    yuzey2: 0xDDE7F2,
    kenar: 0x61748C,
    metin: 0x17283D,
    soluk: 0x40536B,
    vurgu: 0x164DB5,
    vurgu_ustu: 0xFFFFFF,
    basari: 0x146445,
    uyari: 0x7D5000,
    hata: 0xAD263E,
};

pub const KALIP_KOYU: Jetonlar = Jetonlar {
    zemin: 0x101923,
    yuzey: 0x182534,
    yuzey2: 0x223448,
    kenar: 0x7890AB,
    metin: 0xF1F6FC,
    soluk: 0xB4C5D9,
    vurgu: 0x92BBFF,
    vurgu_ustu: 0x10223C,
    basari: 0x80D4AE,
    uyari: 0xFFD17B,
    hata: 0xFF9DAF,
};

pub const MEZURA_ACIK: Jetonlar = Jetonlar {
    zemin: 0xFFFFFF,
    yuzey: 0xFFFFFF,
    yuzey2: 0xEFEFEF,
    kenar: 0x1A1A1A,
    metin: 0x101010,
    soluk: 0x444444,
    vurgu: 0xF2C811,
    vurgu_ustu: 0x141414,
    basari: 0x0F5A30,
    uyari: 0x6B4200,
    hata: 0x9B1C2D,
};

pub const MEZURA_KOYU: Jetonlar = Jetonlar {
    zemin: 0x080808,
    yuzey: 0x141414,
    yuzey2: 0x262626,
    kenar: 0xE0E0E0,
    metin: 0xFFFFFF,
    soluk: 0xD4D4D4,
    vurgu: 0xFFD600,
    vurgu_ustu: 0x000000,
    basari: 0x9EE6B4,
    uyari: 0xFFD58C,
    hata: 0xFFB4B4,
};

/// Tercih ve sistem durumundan kullanılacak tema ve jetonlar.
///
/// - Tema: kullanıcı seçtiyse o; seçmediyse Windows Yüksek Karşıtlık açıkken Mezura, değilse İlmek.
/// - Koyu/açık: [`Mod::koyu_mu`] (`Sistem` ise `sistem_koyu`).
/// - O tema için özel vurgu varsa [`vurgu_duzelt`] ile düzeltilmiş hâli kullanılır ve `vurgu_ustu`
///   yeniden hesaplanır ([`ustune_yazi`]); renk reddedilirse temanın kendi rengi kalır.
pub fn etkin_jetonlar(g: &Gorunum, sistem_koyu: bool, yuksek_karsitlik: bool) -> (Tema, Jetonlar) {
    let tema = g.etkin_tema(yuksek_karsitlik);
    let koyu = g.modu.koyu_mu(sistem_koyu);
    let mut j = tema.jetonlar(koyu);
    if let Some(renk) = g.ozel_vurgu[tema.sira()] {
        if let Ok((r, _)) = vurgu_duzelt(tema, koyu, renk) {
            j.vurgu = r;
            j.vurgu_ustu = ustune_yazi(r);
        }
    }
    (tema, j)
}

#[cfg(test)]
mod testler;
