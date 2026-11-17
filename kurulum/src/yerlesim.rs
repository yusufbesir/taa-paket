// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz 2026-11-17]
//! Kurulum penceresinin saf hesapları (Windows'a bağlı değil; Linux'ta birim testli).
//! Bağlayıcı tasarım: KARAR-FABLE.md (İlmek / Kalıp / Mezura). Çizim `pencere.rs` + `cizim.rs`'dedir;
//! burada yalnız sayılar vardır: tema başına yerleşim dikdörtgenleri, Mezura cetvel çentikleri, ilerleme
//! parça sayıları, yükleme ekranı ve tanıtım sahnesi durumu, geçiş hareketi kareleri, Ayarlar odak sırası,
//! akış düzeni (düğme / çip satırları) ve ekran metinleri. Tüm ölçüler 96 DPI mantıksal pikselden `dpi` ile ölçeklenir.

use crate::ilerleme::{Asama, Durum, Ilerleme};
use taa_kur_tema::{Hareket, Tema, KALIP_OTURMA_PX, YUKLEME_TIK};

/// Mantıksal pikseli (96 DPI) cihaz pikseline çevirir.
pub fn olcekle(v: i32, dpi: u32) -> i32 {
    v * dpi.max(96) as i32 / 96
}

/// Dikdörtgen (x, y, genişlik, yükseklik).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Dik {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Dik {
    pub const fn yeni(x: i32, y: i32, w: i32, h: i32) -> Dik {
        Dik { x, y, w, h }
    }
    pub fn sag(&self) -> i32 {
        self.x + self.w
    }
    pub fn alt(&self) -> i32 {
        self.y + self.h
    }
    pub fn bos(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }
    pub fn icinde(&self, px: i32, py: i32) -> bool {
        px >= self.x && px < self.sag() && py >= self.y && py < self.alt()
    }
    /// Her yandan `d` içeri.
    pub fn ic(&self, d: i32) -> Dik {
        Dik::yeni(self.x + d, self.y + d, (self.w - 2 * d).max(0), (self.h - 2 * d).max(0))
    }
    /// Yatay ve dikey farklı iç boşluk.
    pub fn ic2(&self, yatay: i32, dikey: i32) -> Dik {
        Dik::yeni(self.x + yatay, self.y + dikey, (self.w - 2 * yatay).max(0), (self.h - 2 * dikey).max(0))
    }
    pub fn kaydir(&self, dx: i32, dy: i32) -> Dik {
        Dik::yeni(self.x + dx, self.y + dy, self.w, self.h)
    }
    /// İki boş olmayan dikdörtgen üst üste biniyor mu? (yerleşim testleri)
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn kesisir(&self, o: &Dik) -> bool {
        !self.bos() && !o.bos() && self.x < o.sag() && o.x < self.sag() && self.y < o.alt() && o.y < self.alt()
    }
    /// `o` tamamen bunun içinde mi? (yerleşim testleri)
    #[cfg_attr(not(test), allow(dead_code))]
    pub fn icerir(&self, o: &Dik) -> bool {
        o.bos() || (o.x >= self.x && o.y >= self.y && o.sag() <= self.sag() && o.alt() <= self.alt())
    }
}

/// Ana ekranın bölgeleri. Boş (`h == 0`) bölge o temada kullanılmaz.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Yerlesim {
    /// Logo, "TAA Kurucu", sürüm ve Ayarlar düğmesi.
    pub baslik: Dik,
    /// Yalnız Mezura: büyük görev başlığı.
    pub gorev: Dik,
    /// Aşama şeridi (İlmek dikiş hattı, Kalıp sol ray, Mezura düz metin).
    pub asama: Dik,
    /// Genel ilerleme (İlmek teyel, Kalıp 20 bölme, Mezura cetvel).
    pub ilerleme: Dik,
    /// İş alanı: seçim, programlar, hata.
    pub is_alani: Dik,
    /// Sağ rehber sütunu (adım adım direktifler).
    pub rehber: Dik,
    /// Tanıtım (bekleme) kartı; kart yoksa boş.
    pub tanitim: Dik,
    /// Alt eylem çubuğu.
    pub alt: Dik,
    /// Dar pencere (Mezura küçük başlık, Kalıp/İlmek en az sütun genişlikleri).
    pub dar: bool,
}

/// Tanıtım kartı için ayrılacak en büyük pay (gövdenin yüzdesi): kart değişimi yerleşimi oynatmasın diye
/// yükseklik bütün kartların en uzununa göre bir kez hesaplanır, ama gövdeyi yutmaz.
const KART_PAYI_YUZDE: i32 = 55;
/// Mezura'da kart tam genişlikte bir alt bilgi şerididir; gövdeden daha az pay alır.
const MEZURA_KART_PAYI_YUZDE: i32 = 35;

/// Tanıtım kartının genişliği yalnız temaya ve pencereye bağlıdır (yüksekliğe değil); önce bu ölçülür.
pub fn kart_genisligi(tema: Tema, w: i32, h: i32, dpi: u32) -> i32 {
    let y = yerlesim(tema, w, h, dpi, 0);
    match tema {
        Tema::Ilmek => y.rehber.w,
        Tema::Kalip => y.is_alani.w,
        Tema::Mezura => y.alt.w,
    }
}

/// Tema başına yerleşim (KARAR §2.4, §3.4, §4.4). `w`, `h`: istemci alanı (cihaz pikseli);
/// `kart_h`: ölçülen tanıtım kartı yüksekliği (cihaz pikseli, 0 = kart yok).
pub fn yerlesim(tema: Tema, w: i32, h: i32, dpi: u32, kart_h: i32) -> Yerlesim {
    let s = |v: i32| olcekle(v, dpi);
    let w = w.max(1);
    let h = h.max(1);
    match tema {
        Tema::Ilmek => {
            let m = s(24);
            let baslik = Dik::yeni(0, 0, w, s(60));
            let asama = Dik::yeni(m, baslik.alt(), w - 2 * m, s(56));
            let ilerleme = Dik::yeni(m, asama.alt() + s(4), w - 2 * m, s(44));
            let alt = Dik::yeni(0, h - s(72), w, s(72));
            let govde_ust = ilerleme.alt() + s(12);
            let govde_alt = (alt.y - s(8)).max(govde_ust);
            let govde_h = govde_alt - govde_ust;
            let ara = s(24);
            let dar = w - 2 * m - s(256) - ara < s(440);
            let rw = if dar { s(224) } else { s(256) };
            let sx = w - m - rw;
            let is_alani = Dik::yeni(m, govde_ust, (sx - ara - m).max(0), govde_h);
            let (rehber, tanitim) = sag_sutun(Dik::yeni(sx, govde_ust, rw, govde_h), kart_h, s(12));
            Yerlesim { baslik, gorev: Dik::default(), asama, ilerleme, is_alani, rehber, tanitim, alt, dar }
        }
        Tema::Kalip => {
            let baslik = Dik::yeni(0, 0, w, s(52));
            let alt = Dik::yeni(0, h - s(56), w, s(56));
            let ilerleme = Dik::yeni(0, alt.y - s(52), w, s(52));
            let ust = baslik.alt();
            let govde_h = (ilerleme.y - ust).max(0);
            let dar = w < s(940);
            let ray = if dar { s(168) } else { s(176) };
            let rw = if dar { s(208) } else { s(216) };
            let asama = Dik::yeni(0, ust, ray, govde_h);
            let rehber = Dik::yeni(w - rw, ust, rw, govde_h);
            let orta = Dik::yeni(ray, ust, (w - ray - rw).max(0), govde_h);
            let kh = kart_payi(kart_h, govde_h);
            let tanitim = if kh > 0 { Dik::yeni(orta.x, orta.alt() - kh, orta.w, kh) } else { Dik::default() };
            let is_alani = Dik::yeni(orta.x, orta.y, orta.w, orta.h - kh);
            Yerlesim { baslik, gorev: Dik::default(), asama, ilerleme, is_alani, rehber, tanitim, alt, dar }
        }
        Tema::Mezura => {
            let m = s(24);
            let dar = h < s(640) || w < s(940);
            let baslik = Dik::yeni(0, 0, w, s(52));
            let gorev = Dik::yeni(m, baslik.alt(), w - 2 * m, if dar { s(38) } else { s(46) });
            let asama = Dik::yeni(m, gorev.alt(), w - 2 * m, s(36));
            let ilerleme = Dik::yeni(m, asama.alt() + s(4), w - 2 * m, if dar { s(64) } else { s(72) });
            let alt = Dik::yeni(0, h - s(64), w, s(64));
            let govde_ust = ilerleme.alt() + s(12);
            let kalan = (alt.y - govde_ust).max(0);
            let kh = if kart_h > 0 { kart_h.min(kalan * MEZURA_KART_PAYI_YUZDE / 100) } else { 0 };
            let tanitim = if kh > 0 { Dik::yeni(0, alt.y - kh, w, kh) } else { Dik::default() };
            let govde_alt = if kh > 0 { tanitim.y } else { alt.y };
            let govde_h = (govde_alt - govde_ust - s(8)).max(0);
            let ara = s(20);
            let rw = if w - 2 * m - s(264) - ara >= s(480) { s(264) } else { s(240) };
            let rehber = Dik::yeni(w - m - rw, govde_ust, rw, govde_h);
            let is_alani = Dik::yeni(m, govde_ust, (rehber.x - ara - m).max(0), govde_h);
            Yerlesim { baslik, gorev, asama, ilerleme, is_alani, rehber, tanitim, alt, dar }
        }
    }
}

fn kart_payi(kart_h: i32, govde_h: i32) -> i32 {
    if kart_h <= 0 {
        0
    } else {
        kart_h.min(govde_h * KART_PAYI_YUZDE / 100)
    }
}

/// İlmek sağ sütunu: rehber üstte, tanıtım kartı altta (`ara` kadar boşlukla).
fn sag_sutun(sutun: Dik, kart_h: i32, ara: i32) -> (Dik, Dik) {
    let kh = kart_payi(kart_h, sutun.h);
    if kh == 0 {
        return (sutun, Dik::default());
    }
    let tanitim = Dik::yeni(sutun.x, sutun.alt() - kh, sutun.w, kh);
    let rehber = Dik::yeni(sutun.x, sutun.y, sutun.w, (tanitim.y - ara - sutun.y).max(0));
    (rehber, tanitim)
}

/// Ayarlar yan paneli: başlığın altında, sağa yaslı (akış sıfırlanmaz, iş alanının üstüne açılır).
pub fn ayar_paneli(w: i32, h: i32, dpi: u32, baslik_alt: i32) -> Dik {
    let s = |v: i32| olcekle(v, dpi);
    let pw = s(392).min((w - s(48)).max(s(240))).min(w);
    Dik::yeni(w - pw, baslik_alt, pw, (h - baslik_alt).max(0))
}

// ------------------------------------------------------------------ akış düzeni

/// Genişlikleri verilen öğeleri `alan` genişliğinde soldan sağa dizer, sığmayanı alt satıra indirir.
/// Dönen: her öğe için (satır, x). Tek başına sığmayan öğe kendi satırına konur (çizen iki satıra sarar).
pub fn satira_dagit(genislikler: &[i32], alan: i32, ara: i32) -> Vec<(usize, i32)> {
    let mut v = Vec::with_capacity(genislikler.len());
    let (mut satir, mut x) = (0usize, 0i32);
    for &g in genislikler {
        if x > 0 && x + g > alan {
            satir += 1;
            x = 0;
        }
        v.push((satir, x));
        x += g + ara;
    }
    v
}

// ------------------------------------------------------------------ ilerleme parçaları

/// İlmek teyeli: parça = 14 px (8 px çizgi + 6 px boşluk). Hattaki parça sayısı (en az 1).
pub fn teyel_parcalari(genislik_px: i32, dpi: u32) -> i32 {
    (genislik_px / olcekle(14, dpi).max(1)).max(1)
}

/// Yüzdeye göre dikişe dönmüş parça sayısı (yuvarlama aşağı; %100'de hepsi).
pub fn dikili_parca(toplam: i32, yuzde: u8) -> i32 {
    toplam * yuzde.min(100) as i32 / 100
}

/// Kalıp bölmeli çubuğu: 20 hücre, her biri %5.
pub const KALIP_HUCRE: i32 = 20;

/// Kalıp çubuğunda dolu hücre sayısı.
pub fn dolu_hucre(yuzde: u8) -> i32 {
    yuzde.min(100) as i32 / (100 / KALIP_HUCRE)
}

/// Mezura cetvelinde bir çentik.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Centik {
    pub yuzde: u8,
    /// %10 çentikleri uzundur (12 px) ve altında sayı vardır; ötekiler 5 px.
    pub uzun: bool,
}

/// Mezura cetvel çentikleri (KARAR §4.5, §7.4): her %2'de kısa, her %10'da uzun çentik; %2 aralığı
/// 8 px'in altına düşerse yalnız %5 ve %10 çentikleri çizilir.
pub fn cetvel_centikleri(genislik_px: i32, dpi: u32) -> Vec<Centik> {
    let adim: u8 = if genislik_px / 50 >= olcekle(8, dpi) { 2 } else { 5 };
    (0..=100u8).step_by(adim as usize).map(|y| Centik { yuzde: y, uzun: y % 10 == 0 }).collect()
}

/// Cetvel üzerinde bir yüzdenin x konumu.
pub fn cetvel_x(x0: i32, genislik: i32, yuzde: u8) -> i32 {
    x0 + genislik * yuzde.min(100) as i32 / 100
}

// ------------------------------------------------------------------ yükleme ekranı

/// Markalı yükleme ekranı: 10 tik (1,2 sn), gerçek hazırlıkla paralel; hazırlık uzarsa son karede
/// "Başlatılıyor…" kalır, animasyon döngüye girmez (KARAR §1).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Yukleme {
    pub tik: u8,
}

impl Yukleme {
    pub fn tikle(&mut self) {
        self.tik = (self.tik + 1).min(YUKLEME_TIK);
    }
    /// Çizilecek kare (0..=10). Hareketi azaltta hep son kare.
    pub fn kare(&self, azalt: bool) -> u8 {
        if azalt {
            YUKLEME_TIK
        } else {
            self.tik.min(YUKLEME_TIK)
        }
    }
    /// Yükleme ekranı kalkabilir mi? (10 tik doldu ve kurulum ilk ekranını hazırladı)
    pub fn bitti_mi(&self, hazir: bool) -> bool {
        self.tik >= YUKLEME_TIK && hazir
    }
    pub fn baslatiliyor_mu(&self, hazir: bool) -> bool {
        self.tik >= YUKLEME_TIK && !hazir
    }
}

/// İlmek yüklemesi: 140 px hattın 10 parçasından kaçı dikildi; son karede uçta düğüm belirir.
pub fn yukleme_ilmek(kare: u8) -> (i32, bool) {
    (kare.min(YUKLEME_TIK) as i32, kare >= YUKLEME_TIK)
}

/// Kalıp yüklemesi: üç parça 3'er tik arayla tabana oturur. `None` = parça henüz görünmüyor,
/// `Some(px)` = tabandan yukarı kaydırma (8 -> 3 -> 0).
pub fn yukleme_kalip_parca(parca: usize, kare: u8) -> Option<i32> {
    let bas = 3 * parca as i32 + 1;
    let k = kare as i32;
    if k < bas {
        None
    } else {
        Some(KALIP_OTURMA_PX[((k - bas) as usize).min(2)])
    }
}

/// Kalıp yüklemesi son karesi: köşe nişanları ve "Kuruluma hazır".
pub fn yukleme_kalip_hazir(kare: u8) -> bool {
    kare >= YUKLEME_TIK
}

/// Mezura yüklemesi: sarı bant yüzdesi (her tik %10) ve imleç (son karede).
pub fn yukleme_mezura(kare: u8) -> (u8, bool) {
    (kare.min(YUKLEME_TIK) * 10, kare >= YUKLEME_TIK)
}

// ------------------------------------------------------------------ tanıtım (ilk açılış)

/// Tanıtım sahnesinin hareket kare sayısı (her sahne kendi içinde en çok 4 tik oynar, sonra durur).
pub const SAHNE_TIK: u8 = 4;
/// Sahne sayısı.
pub const SAHNE_SAYISI: u8 = 3;

/// İlk açılış tanıtımı: 3 sahne, otomatik ilerlemez (İleri / Atla, sonda Başla).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Tanitim {
    pub sahne: u8,
    pub tik: u8,
    pub bitti: bool,
}

impl Tanitim {
    pub fn yeni() -> Tanitim {
        Tanitim::default()
    }
    pub fn son_mu(&self) -> bool {
        self.sahne + 1 >= SAHNE_SAYISI
    }
    /// İleri; son sahnede "Başla" = bitir. Dönen: tanıtım bitti mi.
    pub fn ileri(&mut self) -> bool {
        if self.son_mu() {
            self.bitti = true;
        } else {
            self.sahne += 1;
            self.tik = 0;
        }
        self.bitti
    }
    pub fn geri(&mut self) {
        if self.sahne > 0 {
            self.sahne -= 1;
            self.tik = 0;
        }
    }
    pub fn atla(&mut self) {
        self.bitti = true;
    }
    pub fn tikle(&mut self) {
        self.tik = (self.tik + 1).min(SAHNE_TIK);
    }
    /// Çizilecek kare (0..=4); hareketi azaltta son kare.
    pub fn kare(&self, azalt: bool) -> u8 {
        if azalt {
            SAHNE_TIK
        } else {
            self.tik
        }
    }
    pub fn dugme(&self) -> &'static str {
        if self.son_mu() {
            "Başla"
        } else {
            "İleri"
        }
    }
}

/// Sahne başlığı ve açıklaması (başlıklar KARAR §2.6, §3.6, §4.6'daki tema metinleri).
pub fn sahne_metni(tema: Tema, sahne: u8) -> (&'static str, &'static str) {
    match (tema, sahne.min(2)) {
        (Tema::Ilmek, 0) => ("Bilgisayarın görevini seçin.", "Şube PC mi, Sunucu PC mi? Seçiminize göre gereken her şey sırayla bağlanır."),
        (Tema::Ilmek, 1) => ("Mağazanıza uygun programlar hazır.", "Zorunlu programlar kendiliğinden kurulur; isteğe bağlıları siz işaretlersiniz."),
        (Tema::Ilmek, _) => ("Kurulumdan sonra da yanınızda.", "Programlar TAA Kur'da paket listesi olarak durur; güncelleme ve kaldırma oradan yapılır."),
        (Tema::Kalip, 0) => ("Görevi ve şubeyi belirleyin.", "Rol ve şube bir kez seçilir; kurulum planı buna göre çıkarılır."),
        (Tema::Kalip, 1) => ("Programları tek yerden kurun.", "Her program dört adımdan geçer: İndir, Doğrula, Kur, Ayarla."),
        (Tema::Kalip, _) => ("Sürümleri düzenli tutun.", "Kurulu ve seçili sürüm yan yana görünür; yenisi geldiğinde Güncelle yeterlidir."),
        (Tema::Mezura, 0) => ("Bir kez seçin.", "Rolü ve şubeyi seçin; aynı soru bir daha sorulmaz."),
        (Tema::Mezura, 1) => ("Her adımı görün.", "İndir, Doğrula, Kur, Ayarla: her adım işaret ve yazıyla gösterilir."),
        (Tema::Mezura, _) => ("Güvenle tamamlayın.", "Sonunda kaç programın hazır olduğu yazılır; sorun varsa açıkça söylenir."),
    }
}

// ------------------------------------------------------------------ geçiş hareketi

/// Tema ya da ekran değişiminde oynayan kısa geçiş (KARAR §2.6, §3.6, §4.6). Hareketi azaltta hiç başlamaz.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gecis {
    pub hareket: Hareket,
    pub tik: u8,
}

impl Gecis {
    pub fn baslat(hareket: Hareket, azalt: bool) -> Option<Gecis> {
        if azalt {
            None
        } else {
            Some(Gecis { hareket, tik: 0 })
        }
    }
    /// Bir tik ilerletir; dönen: geçiş sürüyor mu.
    pub fn ilerlet(&mut self) -> bool {
        self.tik = self.tik.saturating_add(1);
        !self.bitti()
    }
    pub fn bitti(&self) -> bool {
        self.tik >= self.hareket.adim()
    }
    /// İlmek: eski -> %50 harman -> yeni. Harman oranı (yeni rengin payı); geçiş yoksa `None`.
    pub fn harman(&self) -> Option<f32> {
        match self.hareket {
            Hareket::RenkGecisi if !self.bitti() => Some(0.5),
            _ => None,
        }
    }
    /// Kalıp: yeni panelin sağdan kaydırması (mantıksal px): 8 -> 3 -> 0.
    pub fn kaydirma(&self) -> i32 {
        match self.hareket {
            Hareket::Oturma if !self.bitti() => KALIP_OTURMA_PX[(self.tik as usize).min(2)],
            _ => 0,
        }
    }
    /// Mezura: etkin aşama alt çizgisinin oranı (%50 -> %100).
    pub fn cizgi_orani(&self) -> f32 {
        match self.hareket {
            Hareket::Anlik if !self.bitti() => 0.5,
            _ => 1.0,
        }
    }
}

/// Tanıtım (bekleme) kartı değişiminin bir karesi.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KartKare {
    /// Kart yazıları çizilsin mi? (İlmek solmada ara karelerde yalnız dolgu çizilir: alfa metne uygulanmaz.)
    pub metin: bool,
    /// Kart dolgusunun `yuzey2`'ye doğru harman oranı.
    pub dolgu: f32,
    /// Yatay kaydırma (mantıksal px; Kalıp).
    pub kaydirma: i32,
}

/// Kart değişiminin `tik`. karesi (İlmek 3 adımlı solma, Kalıp 2 adımlı yer değişimi, Mezura anlık).
/// `tik >= adım` ya da hareketi azalt: durgun kart.
pub fn kart_karesi(tema: Tema, tik: u8, azalt: bool) -> KartKare {
    let durgun = KartKare { metin: true, dolgu: 0.0, kaydirma: 0 };
    if azalt || tik >= tema.tanitim_adim() {
        return durgun;
    }
    match tema {
        Tema::Ilmek => KartKare { metin: false, dolgu: if tik == 0 { 0.66 } else { 0.33 }, kaydirma: 0 },
        Tema::Kalip => KartKare { metin: true, dolgu: 0.0, kaydirma: KALIP_OTURMA_PX[(tik as usize).min(2)] },
        Tema::Mezura => durgun,
    }
}

/// İlmek etkin düğüm nabzı: 8 tikte (960 ms) iki ton arası gidip gelir. Dönen: ikinci tonun payı 0..1.
pub fn nabiz(tik: u64, azalt: bool) -> f32 {
    if azalt {
        return 0.0;
    }
    let t = (tik % 8) as f32;
    if t <= 4.0 {
        t / 4.0
    } else {
        (8.0 - t) / 4.0
    }
}

/// Kalıp: etkin işlemde sırayla yanan üç küçük hücreden hangisi yanıyor (360 ms döngü). Azaltta `None`.
pub fn kalip_hucre_dongusu(tik: u64, azalt: bool) -> Option<usize> {
    if azalt {
        None
    } else {
        Some((tik % 3) as usize)
    }
}

// ------------------------------------------------------------------ Ayarlar odak sırası

/// Ayarlar panelinin klavyeyle gezilen denetimleri (Tab sırası).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AyarOge {
    Kapat,
    Tema(usize),
    Mod(usize),
    RenkSec,
    RengeDon,
    HareketAzalt,
    TanitimTekrar,
    Varsayilan,
}

pub const AYAR_SIRASI: [AyarOge; 12] = [
    AyarOge::Kapat,
    AyarOge::Tema(0),
    AyarOge::Tema(1),
    AyarOge::Tema(2),
    AyarOge::Mod(0),
    AyarOge::Mod(1),
    AyarOge::Mod(2),
    AyarOge::RenkSec,
    AyarOge::RengeDon,
    AyarOge::HareketAzalt,
    AyarOge::TanitimTekrar,
    AyarOge::Varsayilan,
];

/// Tab (ileri) / Shift+Tab (geri): sıradaki denetim; başa/sona sarar. Odak yoksa ilk (geri: son).
pub fn ayar_sonraki(simdiki: Option<AyarOge>, geri: bool) -> AyarOge {
    let n = AYAR_SIRASI.len();
    match simdiki.and_then(|o| AYAR_SIRASI.iter().position(|x| *x == o)) {
        None if geri => AYAR_SIRASI[n - 1],
        None => AYAR_SIRASI[0],
        Some(i) if geri => AYAR_SIRASI[(i + n - 1) % n],
        Some(i) => AYAR_SIRASI[(i + 1) % n],
    }
}

/// Sol/Sağ ok: tema ya da mod grubunda komşu seçenek (gruplar 3'lü, sarar). Başka denetimde aynısı.
pub fn ayar_grupta(simdiki: AyarOge, sag: bool) -> AyarOge {
    let k = |i: usize| if sag { (i + 1) % 3 } else { (i + 2) % 3 };
    match simdiki {
        AyarOge::Tema(i) => AyarOge::Tema(k(i)),
        AyarOge::Mod(i) => AyarOge::Mod(k(i)),
        x => x,
    }
}

// ------------------------------------------------------------------ ekran metinleri

/// Durumun ekrandaki metni (her zaman işaretle birlikte çizilir: ○ ◔ ✓ !).
pub fn durum_metni(d: Durum) -> &'static str {
    match d {
        Durum::Bekliyor => "Bekliyor",
        Durum::Suruyor => "Sürüyor",
        Durum::Tamam => "Tamam",
        Durum::Hata => "Hata",
    }
}

/// Bitti ekranının özeti: hata varsa başarı gibi davranmaz ("6 program hazır, 1 program tamamlanamadı.").
pub fn bitti_ozeti(i: &Ilerleme) -> String {
    let tamam = i.programlar.iter().filter(|p| p.durum() == Durum::Tamam).count();
    let hata = i.programlar.iter().filter(|p| p.durum() == Durum::Hata).count();
    if i.programlar.is_empty() {
        "TAA Kur kuruldu.".into()
    } else if hata == 0 {
        format!("{tamam} program hazır.")
    } else {
        format!("{tamam} program hazır, {hata} program tamamlanamadı.")
    }
}

/// Mezura'nın büyük görev başlığı (ve öteki temalarda durum özeti).
pub fn gorev_basligi(asama: Asama, secim_basligi: Option<&str>, hata: bool, komut_bekleniyor: bool) -> String {
    if hata {
        return "Kurulum tamamlanamadı".into();
    }
    if let Some(b) = secim_basligi {
        return b.to_string();
    }
    match asama {
        Asama::Hazirlik => "Kurulum hazırlanıyor".into(),
        Asama::Cekirdek => "TAA Kur dosyaları yazılıyor".into(),
        Asama::Secim => "Seçimler hazırlanıyor".into(),
        Asama::Programlar => "Programlar kuruluyor".into(),
        Asama::Bitti if komut_bekleniyor => "Kurulum bitti".into(),
        Asama::Bitti => "Kurulum bitiyor".into(),
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    const BOYUTLAR: [(i32, i32, u32); 5] = [
        // 860x600 pencerenin istemci alanı (çerçeve ve başlık çubuğu düşülmüş), 96 DPI.
        (844, 561, 96),
        (1024, 661, 96),
        // %150 ve %200 DPI'da en küçük pencere.
        (1266, 842, 144),
        (1688, 1122, 192),
        (1920, 1080, 96),
    ];

    fn bolgeler(y: &Yerlesim) -> Vec<(&'static str, Dik)> {
        vec![
            ("baslik", y.baslik),
            ("gorev", y.gorev),
            ("asama", y.asama),
            ("ilerleme", y.ilerleme),
            ("is_alani", y.is_alani),
            ("rehber", y.rehber),
            ("tanitim", y.tanitim),
            ("alt", y.alt),
        ]
    }

    #[test]
    fn uc_tema_en_kucuk_pencerede_tasmadan_sigar() {
        for tema in Tema::HEPSI {
            for (w, h, dpi) in BOYUTLAR {
                for kart in [0, olcekle(150, dpi), olcekle(900, dpi)] {
                    let y = yerlesim(tema, w, h, dpi, kart);
                    let istemci = Dik::yeni(0, 0, w, h);
                    let b = bolgeler(&y);
                    for (ad, d) in &b {
                        assert!(istemci.icerir(d), "{tema:?} {w}x{h}@{dpi} kart {kart}: {ad} taşıyor: {d:?}");
                        assert!(d.w >= 0 && d.h >= 0, "{ad} eksi boyut");
                    }
                    for i in 0..b.len() {
                        for j in i + 1..b.len() {
                            assert!(!b[i].1.kesisir(&b[j].1), "{tema:?} {w}x{h}@{dpi}: {} ile {} çakışıyor", b[i].0, b[j].0);
                        }
                    }
                    let s = |v: i32| olcekle(v, dpi);
                    assert!(y.is_alani.w >= s(400), "{tema:?} {w}x{h}: iş alanı çok dar ({})", y.is_alani.w);
                    assert!(y.is_alani.h >= s(150), "{tema:?} {w}x{h} kart {kart}: iş alanı çok kısa ({})", y.is_alani.h);
                    assert!(y.rehber.w >= s(208));
                    if kart > 0 {
                        assert!(!y.tanitim.bos(), "kart alanı ayrılmalı");
                    } else {
                        assert!(y.tanitim.bos());
                    }
                }
            }
        }
    }

    #[test]
    fn tema_yerlesimleri_karar_olculerinde() {
        // Kalıp 860 px'te: 168 + orta + 208 (KARAR §3.4); geniş pencerede 176 / 216.
        let k = yerlesim(Tema::Kalip, 844, 561, 96, 0);
        assert_eq!((k.asama.w, k.rehber.w), (168, 208));
        assert_eq!(k.asama.x, 0);
        assert!(k.ilerleme.y > k.is_alani.y, "Kalıp ilerlemesi altta");
        let k = yerlesim(Tema::Kalip, 1024, 661, 96, 0);
        assert_eq!((k.asama.w, k.rehber.w), (176, 216));
        // İlmek: sağ sütun 256 (dar pencerede 224), ilerleme üstte.
        let i = yerlesim(Tema::Ilmek, 1024, 661, 96, 0);
        assert_eq!(i.rehber.w, 256);
        assert!(i.ilerleme.y < i.is_alani.y);
        assert_eq!(yerlesim(Tema::Ilmek, 844, 561, 96, 0).rehber.w, 256, "860 px pencerede de 256 sığar");
        assert_eq!(yerlesim(Tema::Ilmek, 700, 561, 96, 0).rehber.w, 224, "daha darda en az 224");
        // İlmek kartı sağ sütunun altında, rehberin altında.
        let i = yerlesim(Tema::Ilmek, 1024, 661, 96, 160);
        assert_eq!((i.tanitim.x, i.tanitim.w, i.tanitim.h), (i.rehber.x, i.rehber.w, 160));
        assert!(i.tanitim.y > i.rehber.alt());
        // Mezura: rehber 264 (en az 240), kart tam genişlikte şerit, büyük görev başlığı var.
        let m = yerlesim(Tema::Mezura, 1200, 760, 96, 70);
        assert_eq!(m.rehber.w, 264);
        assert_eq!((m.tanitim.x, m.tanitim.w), (0, 1200));
        assert!(!m.gorev.bos() && !m.dar);
        let m = yerlesim(Tema::Mezura, 844, 561, 96, 0);
        assert!(m.dar);
        assert_eq!(m.rehber.w, 264);
        assert_eq!(yerlesim(Tema::Mezura, 760, 561, 96, 0).rehber.w, 240, "daha darda en az 240");
        // Kart genişliği yüksekliğe bağlı değil.
        assert_eq!(kart_genisligi(Tema::Kalip, 844, 561, 96), k_orta(844));
        assert_eq!(kart_genisligi(Tema::Mezura, 844, 561, 96), 844);
    }

    fn k_orta(w: i32) -> i32 {
        w - 168 - 208
    }

    #[test]
    fn kart_yuksekligi_sabit_ve_sinirli() {
        // Aynı kart yüksekliği her çağrıda aynı yerleşimi verir (kart değişimi yerleşimi oynatmaz).
        let a = yerlesim(Tema::Ilmek, 900, 600, 96, 170);
        let b = yerlesim(Tema::Ilmek, 900, 600, 96, 170);
        assert_eq!(a, b);
        // Çok uzun kart gövdeyi yutmaz.
        let c = yerlesim(Tema::Kalip, 844, 561, 96, 5000);
        assert!(c.tanitim.h <= (c.tanitim.h + c.is_alani.h) * KART_PAYI_YUZDE / 100 + 1);
    }

    #[test]
    fn ayar_paneli_pencereye_sigar() {
        for (w, h, dpi) in BOYUTLAR {
            let p = ayar_paneli(w, h, dpi, olcekle(52, dpi));
            assert!(Dik::yeni(0, 0, w, h).icerir(&p));
            assert_eq!(p.sag(), w);
            assert!(p.w >= olcekle(240, dpi));
        }
        assert_eq!(ayar_paneli(1200, 700, 96, 52).w, 392);
    }

    #[test]
    fn akis_duzeni() {
        assert_eq!(satira_dagit(&[100, 100, 100], 250, 10), vec![(0, 0), (0, 110), (1, 0)]);
        assert_eq!(satira_dagit(&[400, 50], 300, 8), vec![(0, 0), (1, 0)], "tek başına sığmayan kendi satırında");
        assert!(satira_dagit(&[], 100, 4).is_empty());
    }

    #[test]
    fn cetvel_centik_sayisi() {
        // Geniş cetvel: her %2 (51 çentik), 11'i uzun.
        let c = cetvel_centikleri(700, 96);
        assert_eq!(c.len(), 51);
        assert_eq!(c.iter().filter(|x| x.uzun).count(), 11);
        assert_eq!((c[0].yuzde, c[50].yuzde), (0, 100));
        // %2 aralığı 8 px'in altında (400/50 = 8 sınırda, 399 altında): yalnız %5 ve %10.
        assert_eq!(cetvel_centikleri(400, 96).len(), 51);
        let d = cetvel_centikleri(399, 96);
        assert_eq!(d.len(), 21);
        assert_eq!(d.iter().filter(|x| x.uzun).count(), 11);
        assert!(d.iter().all(|x| x.yuzde % 5 == 0));
        // DPI: aynı fiziksel genişlik aynı kararı verir.
        assert_eq!(cetvel_centikleri(800, 192).len(), 51);
        assert_eq!(cetvel_centikleri(798, 192).len(), 21);
        assert_eq!(cetvel_x(10, 200, 50), 110);
        assert_eq!(cetvel_x(10, 200, 250), 210);
    }

    #[test]
    fn ilerleme_parcalari() {
        assert_eq!(teyel_parcalari(140, 96), 10);
        assert_eq!(teyel_parcalari(280, 192), 10);
        assert_eq!(teyel_parcalari(5, 96), 1);
        assert_eq!(dikili_parca(10, 62), 6);
        assert_eq!(dikili_parca(10, 100), 10);
        assert_eq!(dikili_parca(10, 250), 10);
        assert_eq!(dolu_hucre(62), 12, "KARAR §3.4 krokisi: %62 = 12 hücre");
        assert_eq!(dolu_hucre(4), 0);
        assert_eq!(dolu_hucre(100), KALIP_HUCRE);
    }

    #[test]
    fn yukleme_ekrani() {
        let mut y = Yukleme::default();
        assert_eq!(y.kare(true), 10, "hareketi azaltta son kare");
        assert!(!y.bitti_mi(true));
        for _ in 0..9 {
            y.tikle();
        }
        assert!(!y.bitti_mi(true) && !y.baslatiliyor_mu(false));
        y.tikle();
        y.tikle();
        assert_eq!(y.tik, 10, "döngüye girmez");
        assert!(y.bitti_mi(true));
        assert!(y.baslatiliyor_mu(false), "hazırlık uzarsa Başlatılıyor… kalır");
        assert_eq!(yukleme_ilmek(3), (3, false));
        assert_eq!(yukleme_ilmek(10), (10, true));
        assert_eq!(yukleme_kalip_parca(0, 0), None);
        assert_eq!(yukleme_kalip_parca(0, 1), Some(8));
        assert_eq!(yukleme_kalip_parca(0, 2), Some(3));
        assert_eq!(yukleme_kalip_parca(0, 3), Some(0));
        assert_eq!(yukleme_kalip_parca(1, 3), None);
        assert_eq!(yukleme_kalip_parca(1, 4), Some(8));
        assert_eq!(yukleme_kalip_parca(2, 9), Some(0));
        assert!(!yukleme_kalip_hazir(9) && yukleme_kalip_hazir(10));
        assert_eq!(yukleme_mezura(4), (40, false));
        assert_eq!(yukleme_mezura(10), (100, true));
    }

    #[test]
    fn tanitim_sahneleri() {
        let mut t = Tanitim::yeni();
        assert_eq!((t.sahne, t.dugme()), (0, "İleri"));
        for _ in 0..9 {
            t.tikle();
        }
        assert_eq!(t.kare(false), SAHNE_TIK, "otomatik ilerlemez, sahnede durur");
        assert_eq!(t.sahne, 0);
        assert!(!t.ileri());
        assert_eq!((t.sahne, t.tik), (1, 0));
        t.geri();
        assert_eq!(t.sahne, 0);
        t.geri();
        assert_eq!(t.sahne, 0);
        t.ileri();
        t.ileri();
        assert_eq!((t.sahne, t.dugme()), (2, "Başla"));
        assert!(t.ileri());
        let mut a = Tanitim::yeni();
        a.atla();
        assert!(a.bitti);
        assert_eq!(Tanitim::yeni().kare(true), SAHNE_TIK);
        for tema in Tema::HEPSI {
            for s in 0..3 {
                let (b, a) = sahne_metni(tema, s);
                assert!(!b.is_empty() && !a.is_empty());
            }
        }
        assert_eq!(sahne_metni(Tema::Ilmek, 0).0, "Bilgisayarın görevini seçin.");
        assert_eq!(sahne_metni(Tema::Kalip, 1).0, "Programları tek yerden kurun.");
        assert_eq!(sahne_metni(Tema::Mezura, 2).0, "Güvenle tamamlayın.");
    }

    #[test]
    fn gecis_kareleri() {
        assert!(Gecis::baslat(Hareket::Oturma, true).is_none(), "hareketi azaltta geçiş yok");
        let mut k = Gecis::baslat(Hareket::Oturma, false).unwrap();
        let mut v = vec![k.kaydirma()];
        while k.ilerlet() {
            v.push(k.kaydirma());
        }
        v.push(k.kaydirma());
        assert_eq!(v, vec![8, 3, 0, 0]);
        let mut i = Gecis::baslat(Hareket::RenkGecisi, false).unwrap();
        assert_eq!(i.harman(), Some(0.5));
        assert!(i.ilerlet());
        assert!(!i.ilerlet());
        assert_eq!(i.harman(), None);
        let mut m = Gecis::baslat(Hareket::Anlik, false).unwrap();
        assert_eq!((m.cizgi_orani(), m.kaydirma(), m.harman()), (0.5, 0, None));
        m.ilerlet();
        m.ilerlet();
        assert_eq!(m.cizgi_orani(), 1.0);
    }

    #[test]
    fn kart_degisimi_ve_surekli_hareket() {
        assert!(!kart_karesi(Tema::Ilmek, 0, false).metin, "İlmek solmasında metne alfa yok: ara karede çizilmez");
        assert!(kart_karesi(Tema::Ilmek, 3, false).metin);
        assert_eq!(kart_karesi(Tema::Kalip, 0, false).kaydirma, 8);
        assert_eq!(kart_karesi(Tema::Kalip, 2, false).kaydirma, 0);
        assert_eq!(kart_karesi(Tema::Mezura, 0, false), KartKare { metin: true, dolgu: 0.0, kaydirma: 0 });
        assert!(kart_karesi(Tema::Ilmek, 0, true).metin, "hareketi azaltta durgun kart");
        assert_eq!(nabiz(0, false), 0.0);
        assert_eq!(nabiz(4, false), 1.0);
        assert_eq!(nabiz(6, false), 0.5);
        assert_eq!(nabiz(4, true), 0.0);
        assert_eq!(kalip_hucre_dongusu(4, false), Some(1));
        assert_eq!(kalip_hucre_dongusu(4, true), None);
    }

    #[test]
    fn ayar_odak_sirasi() {
        assert_eq!(ayar_sonraki(None, false), AyarOge::Kapat);
        assert_eq!(ayar_sonraki(None, true), AyarOge::Varsayilan);
        assert_eq!(ayar_sonraki(Some(AyarOge::Kapat), false), AyarOge::Tema(0));
        assert_eq!(ayar_sonraki(Some(AyarOge::Varsayilan), false), AyarOge::Kapat);
        assert_eq!(ayar_sonraki(Some(AyarOge::Kapat), true), AyarOge::Varsayilan);
        assert_eq!(ayar_sonraki(Some(AyarOge::Mod(2)), false), AyarOge::RenkSec);
        // Tab ile her denetime bir kez uğranır.
        let mut o = AyarOge::Kapat;
        let mut gorulen = vec![o];
        for _ in 1..AYAR_SIRASI.len() {
            o = ayar_sonraki(Some(o), false);
            assert!(!gorulen.contains(&o));
            gorulen.push(o);
        }
        assert_eq!(ayar_grupta(AyarOge::Tema(2), true), AyarOge::Tema(0));
        assert_eq!(ayar_grupta(AyarOge::Mod(0), false), AyarOge::Mod(2));
        assert_eq!(ayar_grupta(AyarOge::RenkSec, true), AyarOge::RenkSec);
    }

    #[test]
    fn ekran_metinleri() {
        let mut i = Ilerleme::yeni("1.0");
        assert_eq!(bitti_ozeti(&i), "TAA Kur kuruldu.");
        for ad in ["a", "b", "c"] {
            i.program_ekle(ad, false);
        }
        i.program_tamam("a");
        i.program_tamam("b");
        i.program_tamam("c");
        assert_eq!(bitti_ozeti(&i), "3 program hazır.");
        i.yeniden_dene("c");
        i.program_hata("c", "ağ yok");
        assert_eq!(bitti_ozeti(&i), "2 program hazır, 1 program tamamlanamadı.");
        assert_eq!(durum_metni(Durum::Suruyor), "Sürüyor");
        assert_eq!(gorev_basligi(Asama::Programlar, None, false, false), "Programlar kuruluyor");
        assert_eq!(gorev_basligi(Asama::Secim, Some("Hangi şube?"), false, false), "Hangi şube?");
        assert_eq!(gorev_basligi(Asama::Secim, Some("x"), true, false), "Kurulum tamamlanamadı");
        assert_eq!(gorev_basligi(Asama::Bitti, None, false, true), "Kurulum bitti");
    }
}
