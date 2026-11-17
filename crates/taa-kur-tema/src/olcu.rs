// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz 2026-11-17]
//! Tema başına geometri, tipografi ve hareket (KARAR §2.2-2.6, §3.2-3.6, §4.2-4.6).
//! Tüm ölçüler 96 DPI mantıksal pikseldir; çizen taraf DPI ile ölçekler.

use crate::Tema;

/// Zamanlayıcı tiki (ms). Hareketler 2-3 adımlıdır, akıcılık taklit edilmez.
pub const TIK_MS: u32 = 120;
/// Yükleme ekranı 1,2 sn = 10 tik.
pub const YUKLEME_TIK: u8 = 10;
/// Kalıp geçişinde yeni panelin sağdan oturma kaydırmaları (px), her tik bir değer.
pub const KALIP_OTURMA_PX: [i32; 3] = [8, 3, 0];

/// Geçiş hareketinin karakteri.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hareket {
    /// İlmek: içerik yerinde, renk 2 adımda açılır (eski -> %50 harman -> yeni; 240 ms).
    RenkGecisi,
    /// Kalıp: yeni panel 8 px sağdan 3 adımda oturur (8 -> 3 -> 0; 360 ms).
    Oturma,
    /// Mezura: içerik anında değişir; yalnız etkin aşama alt çizgisi 2 adımda uzar (%50 -> %100).
    Anlik,
}

impl Hareket {
    /// Geçişin tik sayısı (hareketi azalt açıkken çizen taraf 0 kabul eder).
    pub fn adim(self) -> u8 {
        match self {
            Hareket::RenkGecisi => 2,
            Hareket::Oturma => 3,
            Hareket::Anlik => 2,
        }
    }

    /// Geçişin toplam süresi (ms).
    pub fn sure_ms(self) -> u32 {
        self.adim() as u32 * TIK_MS
    }
}

/// Bir yazı rolü: boy (px) ve ağırlık (400 Regular, 600 Semibold, 700 Bold).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Yazi {
    pub px: i32,
    pub agirlik: i32,
}

const fn y(px: i32, agirlik: i32) -> Yazi {
    Yazi { px, agirlik }
}

/// Yazı rolleri.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Tipografi {
    /// Başlık ailesi (İlmek: "Segoe UI Variable Display", yoksa [`Tipografi::aile`]).
    pub baslik_aile: &'static str,
    /// Gövde ailesi: her temada "Segoe UI".
    pub aile: &'static str,
    /// Ekran başlığı.
    pub baslik: Yazi,
    /// En küçük pencerede ekran başlığı (yalnız Mezura'da farklı: 26).
    pub baslik_dar: Yazi,
    /// Bölüm başlığı.
    pub bolum: Yazi,
    /// Gövde.
    pub govde: Yazi,
    /// Gövde satır yüksekliği.
    pub govde_satir: i32,
    /// Küçük yazı.
    pub kucuk: Yazi,
    /// Düğme etiketi.
    pub dugme: Yazi,
    /// Genel yüzde / hücre numarası.
    pub yuzde: Yazi,
}

/// Ölçüler.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Geometri {
    /// Kart / panel köşe yarıçapı.
    pub kart_yaricap: i32,
    /// Düğme ve seçim alanı yarıçapı.
    pub dugme_yaricap: i32,
    /// Etiket / hücre yarıçapı.
    pub etiket_yaricap: i32,
    /// Hap biçimli seçim çipi yarıçapı (İlmek'te program grupları, ILHAM-TURU-CHROME).
    pub cip_yaricap: i32,
    /// Kontrol kenar kalınlığı.
    pub kenar: i32,
    /// İç ayırıcı kalınlığı.
    pub ayirici: i32,
    /// Odak çerçevesi kalınlığı (her temada 2).
    pub odak: i32,
    /// Odak çerçevesi ile kontrol arası boşluk (Mezura: 2 px boşluk + çift çerçeve).
    pub odak_bosluk: i32,
    /// Boşluk ölçeği (küçükten büyüğe).
    pub bosluk: &'static [i32],
    /// Kart / panel iç boşluğu.
    pub kart_ic_bosluk: i32,
    /// Liste satırı yüksekliği.
    pub satir: i32,
    /// Düğme yüksekliği.
    pub dugme: i32,
    /// Düğme yatay iç boşluğu; genişlik = ölçülen metin + 2 x bu (en az [`Geometri::dugme_en_az`]).
    pub dugme_yatay_ic: i32,
    /// En dar düğme genişliği.
    pub dugme_en_az: i32,
    /// Sağ rehber sütunu genişliği ve en azı.
    pub rehber_genislik: i32,
    pub rehber_en_az: i32,
}

impl Geometri {
    /// Ölçülen metin genişliğinden düğme genişliği (etiket kesilmez).
    pub fn dugme_genisligi(&self, metin_px: i32) -> i32 {
        (metin_px + 2 * self.dugme_yatay_ic).max(self.dugme_en_az)
    }
}

pub(crate) fn tipografi(t: Tema) -> Tipografi {
    match t {
        Tema::Ilmek => Tipografi {
            baslik_aile: "Segoe UI Variable Display",
            aile: "Segoe UI",
            baslik: y(26, 600),
            baslik_dar: y(26, 600),
            bolum: y(18, 600),
            govde: y(15, 400),
            govde_satir: 22,
            kucuk: y(13, 400),
            dugme: y(15, 600),
            // KARAR'da ayrı yüzde rolü yok; bölüm başlığı ölçüsü kullanılır.
            yuzde: y(18, 600),
        },
        Tema::Kalip => Tipografi {
            baslik_aile: "Segoe UI",
            aile: "Segoe UI",
            baslik: y(22, 700),
            baslik_dar: y(22, 700),
            bolum: y(16, 600),
            govde: y(14, 400),
            govde_satir: 20,
            kucuk: y(13, 400),
            // KARAR'da ayrı düğme rolü yok; gövde boyu Semibold (vurgu ağırlıkla kurulur).
            dugme: y(14, 600),
            yuzde: y(18, 700),
        },
        Tema::Mezura => Tipografi {
            baslik_aile: "Segoe UI",
            aile: "Segoe UI",
            baslik: y(30, 700),
            baslik_dar: y(26, 700),
            bolum: y(20, 600),
            govde: y(16, 400),
            govde_satir: 24,
            kucuk: y(14, 400),
            dugme: y(16, 700),
            yuzde: y(28, 700),
        },
    }
}

pub(crate) fn geometri(t: Tema) -> Geometri {
    match t {
        Tema::Ilmek => Geometri {
            kart_yaricap: 14,
            dugme_yaricap: 10,
            etiket_yaricap: 6,
            cip_yaricap: 10,
            kenar: 1,
            ayirici: 1,
            odak: 2,
            odak_bosluk: 0,
            bosluk: &[4, 8, 16, 24, 32],
            kart_ic_bosluk: 24,
            satir: 48,
            dugme: 44,
            dugme_yatay_ic: 20,
            dugme_en_az: 96,
            rehber_genislik: 256,
            rehber_en_az: 224,
        },
        Tema::Kalip => Geometri {
            kart_yaricap: 4,
            dugme_yaricap: 4,
            etiket_yaricap: 2,
            cip_yaricap: 4,
            kenar: 1,
            ayirici: 1,
            odak: 2,
            odak_bosluk: 0,
            bosluk: &[4, 8, 12, 16, 24],
            kart_ic_bosluk: 12,
            satir: 40,
            dugme: 36,
            dugme_yatay_ic: 16,
            dugme_en_az: 96,
            rehber_genislik: 216,
            rehber_en_az: 208,
        },
        Tema::Mezura => Geometri {
            kart_yaricap: 0,
            dugme_yaricap: 0,
            etiket_yaricap: 0,
            cip_yaricap: 0,
            kenar: 2,
            ayirici: 1,
            odak: 2,
            odak_bosluk: 2,
            bosluk: &[8, 16, 24, 32],
            kart_ic_bosluk: 16,
            satir: 52,
            dugme: 48,
            dugme_yatay_ic: 20,
            dugme_en_az: 96,
            rehber_genislik: 264,
            rehber_en_az: 240,
        },
    }
}
