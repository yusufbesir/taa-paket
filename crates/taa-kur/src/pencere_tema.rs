// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz 2026-11-17]
//! Paket penceresinin görünüm kararları (platformdan bağımsız, Linux'ta birim testli).
//! Bağlayıcı tasarım: arayuz-modernlesme/taa-kurucu/ekip/KARAR-FABLE.md (İlmek / Kalıp / Mezura).
//! Burada: tema -> yerleşim kuralları, ekran düzeni (dikdörtgenler; metinler ölçülerek yerleşir, etiket
//! kesilmez), durum işaretleri, seçili satır biçimi, geçiş kareleri ve Ayarlar işlemleri. `win.rs` yalnız
//! bu kararları GDI ile çizer. İş mantığı (Kur / Kaldır / Güncelle / Rapor) `pencere_model`dedir, değişmedi.

use crate::pencere_model::{Satir, SUTUNLAR};
use taa_kur_tema::{
    ara_renk, hataya_yakin, vurgu_duzelt, Gorunum, Hareket, Jetonlar, Mod, Tema, Yazi,
    KALIP_OTURMA_PX,
};

/// Pencere başlık çubuğu metni.
pub const PENCERE_BASLIGI: &str = "TAA Kurucu · Paketler";
/// Hedef ve en küçük pencere boyutu (96 DPI mantıksal piksel, KARAR §1).
pub const HEDEF_BOYUT: (i32, i32) = (1040, 700);
pub const EN_AZ_BOYUT: (i32, i32) = (860, 600);
/// Varsayılana dön onay sorusu.
pub const VARSAYILAN_ONAY: &str = "Görünüm tercihleri varsayılana dönsün mü?\nTema, görünüm modu, vurgu rengi ve hareket ayarı sıfırlanır. Paketlere ve kurulu programlara dokunulmaz.";
/// Tanıtım tercihi sıfırlanınca durum satırı.
pub const TANITIM_SIFIRLANDI: &str =
    "Tanıtım, TAA Kurucu bir sonraki açılışında yeniden gösterilecek.";
/// Yüksek Karşıtlık yüzünden Mezura seçildiğinde Ayarlar notu.
pub const YK_NOTU: &str = "Windows Yüksek Karşıtlık açık olduğu için Mezura kullanılıyor. Başka bir tema seçerseniz o tema kalır.";
/// Ayarlar ekranındaki satır etiketleri (sırasıyla).
pub const AYAR_ETIKETLERI: [&str; 5] =
    ["Tema", "Görünüm modu", "Vurgu rengi", "Hareket", "Tanıtım"];

/// Sıkışık Ayarlar ekranında Hareket ve Tanıtım satırlarının ortak etiketi.
pub const BIRLESIK_ETIKET: &str = "Hareket ve tanıtım";

/// Windows'tan okunan durum.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SistemDurumu {
    /// Uygulama teması koyu mu (AppsUseLightTheme = 0).
    pub koyu: bool,
    /// Yüksek Karşıtlık açık mı (SPI_GETHIGHCONTRAST).
    pub yuksek_karsitlik: bool,
    /// "Animasyonları göster" açık mı (SPI_GETCLIENTAREAANIMATION).
    pub animasyon_acik: bool,
}

impl Default for SistemDurumu {
    fn default() -> Self {
        SistemDurumu {
            koyu: false,
            yuksek_karsitlik: false,
            animasyon_acik: true,
        }
    }
}

/// Tercih + sistem durumundan çıkan, o an kullanılacak görünüm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Gorunus {
    pub tema: Tema,
    pub koyu: bool,
    pub j: Jetonlar,
    pub hareketi_azalt: bool,
    /// Tema kullanıcı seçimi değil, Yüksek Karşıtlık yüzünden Mezura.
    pub yk_ile_mezura: bool,
}

impl Gorunus {
    pub fn hesapla(g: &Gorunum, s: SistemDurumu) -> Gorunus {
        let (tema, j) = taa_kur_tema::etkin_jetonlar(g, s.koyu, s.yuksek_karsitlik);
        Gorunus {
            tema,
            koyu: g.modu.koyu_mu(s.koyu),
            j,
            hareketi_azalt: g.hareketi_azalt_mi(s.animasyon_acik),
            yk_ile_mezura: g.tema.is_none() && s.yuksek_karsitlik,
        }
    }
}

/// Renk jetonu adı (çizen taraf `renk` ile 0xRRGGBB alır).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Jeton {
    Zemin,
    Yuzey,
    Yuzey2,
    Kenar,
    Metin,
    Soluk,
    Vurgu,
    VurguUstu,
    Basari,
    Uyari,
    Hata,
}

impl Jeton {
    pub fn renk(self, j: &Jetonlar) -> u32 {
        match self {
            Jeton::Zemin => j.zemin,
            Jeton::Yuzey => j.yuzey,
            Jeton::Yuzey2 => j.yuzey2,
            Jeton::Kenar => j.kenar,
            Jeton::Metin => j.metin,
            Jeton::Soluk => j.soluk,
            Jeton::Vurgu => j.vurgu,
            Jeton::VurguUstu => j.vurgu_ustu,
            Jeton::Basari => j.basari,
            Jeton::Uyari => j.uyari,
            Jeton::Hata => j.hata,
        }
    }
}

/// İlmek renk geçişi: yalnız dolgu jetonları harmanlanır, metin jetonları doğrudan yeni değerdedir
/// (KARAR §1: alfa harmanı yalnız dolguya uygulanır, metne değil).
pub fn dolgu_harmanla(eski: &Jetonlar, yeni: &Jetonlar, t: f32) -> Jetonlar {
    let h = |a: u32, b: u32| ara_renk(a, b, t);
    Jetonlar {
        zemin: h(eski.zemin, yeni.zemin),
        yuzey: h(eski.yuzey, yeni.yuzey),
        yuzey2: h(eski.yuzey2, yeni.yuzey2),
        vurgu: h(eski.vurgu, yeni.vurgu),
        ..*yeni
    }
}

/// Hangi ekran gösteriliyor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum Ekran {
    #[default]
    Paketler,
    Ayarlar,
}

/// Paket eylemlerinin yeri (KARAR §2.4, §3.4, §4.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EylemYeri {
    /// İlmek: hepsi üst araç çubuğunda (Kur… dolu, diğerleri düz).
    UstAracCubugu,
    /// Kalıp: tablonun altındaki işlem çubuğunda; Rapor sağda.
    AltIslemCubugu,
    /// Mezura: Kur… büyük başlığın sağında, diğerleri tablonun altında; Rapor sağda.
    KurUstSagDigerleriAlt,
}

/// Tablo çevresi.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabloKenari {
    /// İlmek: kenarsız, yuvarlak köşeli yüzey kartı (yüzey farkıyla ayrılır).
    YuzeyKarti,
    /// Kalıp: 1 px `kenar` çerçeve.
    InceCerceve,
    /// Mezura: yalnız üstte ve altta 2 px çizgi.
    KalinUstAlt,
}

/// Liste satırları arası ayırıcı.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SatirAyirici {
    /// İlmek imzası: 1 px teyel (8 px çizgi + 6 px boşluk, `kenar`).
    Teyel,
    /// Kalıp ve Mezura: 1 px düz çizgi.
    Cizgi,
}

/// Tema başına yerleşim kararları.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Yerlesim {
    pub eylemler: EylemYeri,
    /// Kalıp: üstte `Paketler | Ayarlar` sekmeleri.
    pub sekmeler: bool,
    /// İlmek ve Mezura: başlık satırının sağında `Ayarlar` düğmesi.
    pub ayarlar_baslikta: bool,
    /// Mezura: büyük ekran başlığı (`Paketler` / `Ayarlar`) ve altında 4 px çizgi.
    pub buyuk_baslik: bool,
    /// Kalıp: kök klasör bandı tablodan önce; diğerlerinde tablodan sonra.
    pub kok_tablodan_once: bool,
    /// Rapor düğmesi satırın sağında mı (Kalıp, Mezura).
    pub rapor_sagda: bool,
    pub tablo_kenari: TabloKenari,
    pub satir_ayirici: SatirAyirici,
    /// Kalıp: kök klasör bandı ve işlem çubuğu 1 px çerçeveli yüzey bantlarıdır.
    pub bantlar_cerceveli: bool,
}

pub fn yerlesim(t: Tema) -> Yerlesim {
    match t {
        Tema::Ilmek => Yerlesim {
            eylemler: EylemYeri::UstAracCubugu,
            sekmeler: false,
            ayarlar_baslikta: true,
            buyuk_baslik: false,
            kok_tablodan_once: false,
            rapor_sagda: false,
            tablo_kenari: TabloKenari::YuzeyKarti,
            satir_ayirici: SatirAyirici::Teyel,
            bantlar_cerceveli: false,
        },
        Tema::Kalip => Yerlesim {
            eylemler: EylemYeri::AltIslemCubugu,
            sekmeler: true,
            ayarlar_baslikta: false,
            buyuk_baslik: false,
            kok_tablodan_once: true,
            rapor_sagda: true,
            tablo_kenari: TabloKenari::InceCerceve,
            satir_ayirici: SatirAyirici::Cizgi,
            bantlar_cerceveli: true,
        },
        Tema::Mezura => Yerlesim {
            eylemler: EylemYeri::KurUstSagDigerleriAlt,
            sekmeler: false,
            ayarlar_baslikta: true,
            buyuk_baslik: true,
            kok_tablodan_once: false,
            rapor_sagda: true,
            tablo_kenari: TabloKenari::KalinUstAlt,
            satir_ayirici: SatirAyirici::Cizgi,
            bantlar_cerceveli: false,
        },
    }
}

/// Başlık satırı metni (logonun sağında).
pub fn baslik_metni(t: Tema, ekran: Ekran) -> String {
    match t {
        Tema::Ilmek => match ekran {
            Ekran::Paketler => "TAA Kurucu · Paketler".to_string(),
            Ekran::Ayarlar => "TAA Kurucu · Ayarlar".to_string(),
        },
        // Kalıp'ta ekranı sekmeler, Mezura'da büyük başlık söyler.
        Tema::Kalip | Tema::Mezura => format!("TAA Kurucu  v{}", env!("CARGO_PKG_VERSION")),
    }
}

/// Mezura büyük başlığı.
pub fn buyuk_baslik_metni(ekran: Ekran) -> &'static str {
    match ekran {
        Ekran::Paketler => "Paketler",
        Ekran::Ayarlar => "Ayarlar",
    }
}

// --- Durum sütunu ---

/// Satır durumunun türü: her zaman işaret + metin (yalnız renkle anlatım yok).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DurumTuru {
    /// "Hazır", "Hazır (sabit)".
    Tamam,
    /// "Seçili sürüm yok", "Daha yeni sürüm kurulu (X), seçili değil".
    Uyari,
    /// "Kurulu değil".
    Bos,
}

impl DurumTuru {
    pub fn isaret(self) -> &'static str {
        match self {
            DurumTuru::Tamam => "✓",
            DurumTuru::Uyari => "!",
            DurumTuru::Bos => "○",
        }
    }

    pub fn jeton(self) -> Jeton {
        match self {
            DurumTuru::Tamam => Jeton::Basari,
            DurumTuru::Uyari => Jeton::Uyari,
            DurumTuru::Bos => Jeton::Soluk,
        }
    }
}

pub fn durum_turu(s: &Satir) -> DurumTuru {
    if s.durum.starts_with("Hazır") {
        DurumTuru::Tamam
    } else if s.durum == "Kurulu değil" {
        DurumTuru::Bos
    } else {
        DurumTuru::Uyari
    }
}

/// Durum sütununda gösterilen metin: "✓ Hazır", "! Seçili sürüm yok", "○ Kurulu değil".
pub fn durum_gorunen(s: &Satir) -> String {
    format!("{} {}", durum_turu(s).isaret(), s.durum)
}

/// Hücre metninin sığmadığında ne olacağı.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetinKipi {
    /// Tek satır; sığmazsa "…" (yalnız paket adı).
    TekKisalt,
    /// Tek satır, kısaltma yok (ölçülerek yer ayrıldı).
    Tek,
    /// Sözcükten sarılır (en çok iki-üç satır), kesilmez.
    Sarili,
    /// Klasör yolu: ortadan "…" (tam metin Kopyala ile alınır).
    YolKisalt,
}

/// Liste hücresi kipi: paket adı kısaltılabilir; sürümler ve durum sarılır, kesilmez.
pub fn hucre_kipi(sutun: usize) -> MetinKipi {
    match sutun {
        0 => MetinKipi::TekKisalt,
        _ => MetinKipi::Sarili,
    }
}

/// Seçili liste satırının biçimi (KARAR: İlmek yuzey2 + kalın; Kalıp 3 px sol vurgu şeridi;
/// Mezura yuzey2 + 6 px sarı-siyah şerit + ▸).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeciliBicim {
    pub dolgu: Jeton,
    pub kalin: bool,
    /// Soldan sağa şerit parçaları (genişlik px @96 DPI, renk).
    pub seritler: Vec<(i32, Jeton)>,
    /// Paket adının önüne konan işaret.
    pub isaret: Option<&'static str>,
}

pub fn secili_bicim(t: Tema) -> SeciliBicim {
    match t {
        Tema::Ilmek => SeciliBicim {
            dolgu: Jeton::Yuzey2,
            kalin: true,
            seritler: Vec::new(),
            isaret: None,
        },
        Tema::Kalip => SeciliBicim {
            dolgu: Jeton::Yuzey2,
            kalin: false,
            seritler: vec![(3, Jeton::Vurgu)],
            isaret: None,
        },
        Tema::Mezura => SeciliBicim {
            dolgu: Jeton::Yuzey2,
            kalin: true,
            seritler: vec![(4, Jeton::Vurgu), (2, Jeton::VurguUstu)],
            isaret: Some("▸"),
        },
    }
}

/// Seçili satır şeritlerinin toplam genişliği (96 DPI).
pub fn serit_genisligi(t: Tema) -> i32 {
    secili_bicim(t).seritler.iter().map(|s| s.0).sum()
}

/// Klavye odağı rengi: Mezura açıkta sarı tek başına gösterge olamaz, `kenar` kullanılır.
pub fn odak_jetonu(t: Tema, koyu: bool) -> Jeton {
    if t.vurgu_cerceveli(koyu) {
        Jeton::Kenar
    } else {
        Jeton::Vurgu
    }
}

// --- Geçişler ---

/// Bir geçiş karesinde uygulanacak değerler.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GecisKaresi {
    /// Eski renkten yeniye oran (İlmek).
    pub renk_orani: f32,
    /// İçeriğin sağdan kayması, 96 DPI px (Kalıp).
    pub kayma_px: i32,
    /// Mezura büyük başlık alt çizgisinin uzunluk oranı.
    pub cizgi_orani: f32,
}

pub const SON_KARE: GecisKaresi = GecisKaresi {
    renk_orani: 1.0,
    kayma_px: 0,
    cizgi_orani: 1.0,
};

/// Geçişin tik sayısı; hareketi azalt açıkken 0 (anlık).
pub fn gecis_adim_sayisi(h: Hareket, hareketi_azalt: bool) -> u8 {
    if hareketi_azalt {
        0
    } else {
        h.adim()
    }
}

/// `adim_no` 1'den başlar; adım sayısını aşınca son kare.
pub fn gecis_karesi(h: Hareket, adim_no: u8) -> GecisKaresi {
    let n = h.adim();
    if adim_no == 0 || adim_no > n {
        return SON_KARE;
    }
    let oran = adim_no as f32 / n as f32;
    match h {
        Hareket::RenkGecisi => GecisKaresi {
            renk_orani: oran,
            ..SON_KARE
        },
        Hareket::Oturma => GecisKaresi {
            kayma_px: KALIP_OTURMA_PX[(adim_no as usize - 1).min(KALIP_OTURMA_PX.len() - 1)],
            ..SON_KARE
        },
        Hareket::Anlik => GecisKaresi {
            cizgi_orani: oran,
            ..SON_KARE
        },
    }
}

// --- Ayarlar işlemleri ---

/// Vurgu rengi seçimi: kontrast koruması (KARAR §5) uygulanır, tercihe yazılır.
/// Dönen not (varsa) Ayarlar ekranında vurgu satırının altında gösterilir.
pub fn vurgu_sec(g: &mut Gorunum, tema: Tema, koyu: bool, secilen: u32) -> Option<(String, Jeton)> {
    match vurgu_duzelt(tema, koyu, secilen) {
        Err(red) => Some((
            format!("{red} Temanın rengi kullanılmaya devam ediyor."),
            Jeton::Uyari,
        )),
        Ok((renk, duzeltildi)) => {
            g.ozel_vurgu_ata(tema, Some(renk));
            let mut notlar = Vec::new();
            if duzeltildi {
                notlar.push(if koyu {
                    "Okunabilirlik için renk biraz açıldı."
                } else {
                    "Okunabilirlik için renk biraz koyulaştırıldı."
                });
            }
            let yakin = hataya_yakin(tema, koyu, renk);
            if yakin {
                notlar.push("Bu renk hata rengine çok yakın; durum işaretleriyle karışabilir.");
            }
            if notlar.is_empty() {
                None
            } else {
                Some((
                    notlar.join(" "),
                    if yakin { Jeton::Uyari } else { Jeton::Soluk },
                ))
            }
        }
    }
}

// --- Düzen (yerleşim hesabı) ---

/// Dikdörtgen (aygıt pikseli).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Dik {
    pub x: i32,
    pub y: i32,
    pub en: i32,
    pub boy: i32,
}

impl Dik {
    pub const fn yeni(x: i32, y: i32, en: i32, boy: i32) -> Dik {
        Dik { x, y, en, boy }
    }
    pub fn sag(&self) -> i32 {
        self.x + self.en
    }
    pub fn alt(&self) -> i32 {
        self.y + self.boy
    }
    pub fn kesisir(&self, o: &Dik) -> bool {
        self.x < o.sag() && o.x < self.sag() && self.y < o.alt() && o.y < self.alt()
    }
    pub fn icinde(&self, o: &Dik) -> bool {
        self.x >= o.x && self.y >= o.y && self.sag() <= o.sag() && self.alt() <= o.alt()
    }
    fn kaydir(&mut self, dx: i32) {
        self.x += dx;
    }
}

/// Yazı rolü (çizen taraf fontu seçer).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum YaziRolu {
    /// Ekran başlığı (Mezura büyük başlığı).
    Baslik,
    /// Bölüm başlığı (başlık satırı, Ayarlar "Görünüm").
    Bolum,
    Govde,
    GovdeKalin,
    Kucuk,
    Dugme,
}

pub fn yazi(t: Tema, r: YaziRolu) -> Yazi {
    let ty = t.tipografi();
    match r {
        YaziRolu::Baslik => ty.baslik,
        YaziRolu::Bolum => ty.bolum,
        YaziRolu::Govde => ty.govde,
        YaziRolu::GovdeKalin => Yazi {
            px: ty.govde.px,
            agirlik: if t == Tema::Ilmek { 600 } else { 700 },
        },
        YaziRolu::Kucuk => ty.kucuk,
        YaziRolu::Dugme => ty.dugme,
    }
}

/// Metin ölçer (çizen taraf GDI ile, testler sahte ölçüyle verir). Değerler aygıt pikselidir.
pub trait Olcer {
    /// Metnin tek satırdaki genişliği.
    fn genislik(&self, metin: &str, rol: YaziRolu) -> i32;
    /// Bir satırın yüksekliği.
    fn satir(&self, rol: YaziRolu) -> i32;
}

/// Düğme türü (çizim biçimi).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DugmeTuru {
    /// Vurgu dolgulu (Kur…).
    Birincil,
    /// Düz (diğer eylemler).
    Duz,
    /// Kalıp sekmesi.
    Sekme,
    /// Tema seçimi: içinde canlı önizleme.
    TemaSecimi,
    /// Radyo (görünüm modu).
    Radyo,
    /// Onay kutusu (Hareketi azalt).
    Onay,
}

/// Penceredeki düğmeler (hepsi sahip-çizimli BUTTON denetimi; klavye ve ekran okuyucu için).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Oge {
    Kur,
    Kaldir,
    Guncelle,
    Rapor,
    /// Başlıktaki Ayarlar / Paketlere dön (İlmek, Mezura).
    Ayarlar,
    Kopyala,
    SekmePaketler,
    SekmeAyarlar,
    Tema(Tema),
    Mod(Mod),
    RenkSec,
    RenkSifirla,
    HareketiAzalt,
    TanitimTekrar,
    Varsayilan,
}

impl Oge {
    pub const HEPSI: [Oge; 19] = [
        Oge::Kur,
        Oge::Kaldir,
        Oge::Guncelle,
        Oge::Rapor,
        Oge::Ayarlar,
        Oge::Kopyala,
        Oge::SekmePaketler,
        Oge::SekmeAyarlar,
        Oge::Tema(Tema::Ilmek),
        Oge::Tema(Tema::Kalip),
        Oge::Tema(Tema::Mezura),
        Oge::Mod(Mod::Acik),
        Oge::Mod(Mod::Koyu),
        Oge::Mod(Mod::Sistem),
        Oge::RenkSec,
        Oge::RenkSifirla,
        Oge::HareketiAzalt,
        Oge::TanitimTekrar,
        Oge::Varsayilan,
    ];

    /// Denetim kimliği (WM_COMMAND).
    pub fn kimlik(self) -> isize {
        101 + Oge::HEPSI.iter().position(|o| *o == self).unwrap_or(0) as isize
    }

    pub fn kimlikten(k: isize) -> Option<Oge> {
        usize::try_from(k - 101)
            .ok()
            .and_then(|i| Oge::HEPSI.get(i).copied())
    }

    pub fn tur(self) -> DugmeTuru {
        match self {
            Oge::Kur => DugmeTuru::Birincil,
            Oge::SekmePaketler | Oge::SekmeAyarlar => DugmeTuru::Sekme,
            Oge::Tema(_) => DugmeTuru::TemaSecimi,
            Oge::Mod(_) => DugmeTuru::Radyo,
            Oge::HareketiAzalt => DugmeTuru::Onay,
            _ => DugmeTuru::Duz,
        }
    }

    pub fn etiket(self, ekran: Ekran) -> String {
        match self {
            Oge::Kur => "Kur…".into(),
            Oge::Kaldir => "Kaldır".into(),
            Oge::Guncelle => "Güncelle".into(),
            Oge::Rapor => "Rapor".into(),
            Oge::Ayarlar => match ekran {
                Ekran::Paketler => "Ayarlar".into(),
                Ekran::Ayarlar => "Paketlere dön".into(),
            },
            Oge::Kopyala => "Kopyala".into(),
            Oge::SekmePaketler => "Paketler".into(),
            Oge::SekmeAyarlar => "Ayarlar".into(),
            Oge::Tema(t) => t.gorunen_ad().into(),
            Oge::Mod(m) => m.gorunen_ad().into(),
            Oge::RenkSec => "Renk seç…".into(),
            Oge::RenkSifirla => "Temanın rengine dön".into(),
            Oge::HareketiAzalt => "Hareketi azalt".into(),
            Oge::TanitimTekrar => "Tanıtımı tekrar göster".into(),
            Oge::Varsayilan => "Varsayılana dön".into(),
        }
    }

    /// Seçili / işaretli görünür mü (sekme, tema, mod, onay kutusu).
    pub fn secili(self, g: &Gorunum, gs: &Gorunus, ekran: Ekran) -> bool {
        match self {
            Oge::SekmePaketler => ekran == Ekran::Paketler,
            Oge::SekmeAyarlar => ekran == Ekran::Ayarlar,
            Oge::Tema(t) => gs.tema == t,
            Oge::Mod(m) => g.modu == m,
            Oge::HareketiAzalt => gs.hareketi_azalt,
            _ => false,
        }
    }
}

/// Düğme göstergesi (radyo dairesi, onay kutusu, tema ✓) için etiket dışında ayrılan genişlik (96 DPI).
pub fn gosterge_genisligi(t: DugmeTuru) -> i32 {
    match t {
        DugmeTuru::Radyo | DugmeTuru::Onay => 26,
        DugmeTuru::TemaSecimi => 22,
        _ => 0,
    }
}

/// Tema önizlemesi boyutu (KARAR §5: 160x100). Ayarlar pencereye sığmazsa sıkışık ölçü kullanılır.
pub const ONIZLEME: (i32, i32) = (160, 100);
pub const ONIZLEME_SIKISIK: (i32, i32) = (112, 70);

/// Yerleşmiş düğme.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct YerDugme {
    pub oge: Oge,
    pub alan: Dik,
    /// Etiket satır sayısı (sığmayan etiket iki satıra iner).
    pub satir: u8,
    /// Arkasındaki yüzey (düğme köşeleri bu renkle doldurulur).
    pub arka: Jeton,
}

/// Çizilecek metin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Metin {
    pub metin: String,
    pub alan: Dik,
    pub rol: YaziRolu,
    pub jeton: Jeton,
    pub kip: MetinKipi,
}

/// Dolgulu / çerçeveli kutu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kutu {
    pub alan: Dik,
    pub dolgu: Option<Jeton>,
    /// (renk, kalınlık px)
    pub kenar: Option<(Jeton, i32)>,
    pub yaricap: i32,
}

/// Bir ekranın tam düzeni.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Duzen {
    pub logo: Dik,
    pub kutular: Vec<Kutu>,
    pub metinler: Vec<Metin>,
    pub dugmeler: Vec<YerDugme>,
    /// Liste denetimi (yalnız Paketler ekranı).
    pub liste: Option<Dik>,
    /// Vurgu rengi örneği (yalnız Ayarlar).
    pub renk_ornegi: Option<Dik>,
}

impl Duzen {
    pub fn dugme(&self, oge: Oge) -> Option<&YerDugme> {
        self.dugmeler.iter().find(|d| d.oge == oge)
    }
}

/// Düzen girdisi.
#[derive(Clone, Debug)]
pub struct DuzenGirdisi<'a> {
    pub tema: Tema,
    pub dpi: u32,
    pub en: i32,
    pub boy: i32,
    pub ekran: Ekran,
    pub durum: &'a str,
    pub kok: &'a str,
    pub vurgu_notu: Option<(&'a str, Jeton)>,
    pub yk_notu: bool,
    pub kare: GecisKaresi,
}

/// 96 DPI mantıksal pikseli aygıt pikseline çevirir.
pub fn olcekle(v: i32, dpi: u32) -> i32 {
    (v * dpi as i32 + 48) / 96
}

/// Kenar boşluğu: temanın boşluk ölçeğinde 12'den büyük-eşit ilk değer (İlmek 16, Kalıp 12, Mezura 16).
fn dis_bosluk(t: Tema) -> i32 {
    let g = t.geometri();
    g.bosluk.iter().copied().find(|b| *b >= 12).unwrap_or(16)
}

/// Sarılı metnin satır sayısı (en az 1, en çok `en_cok`).
fn satir_sayisi(o: &dyn Olcer, metin: &str, rol: YaziRolu, en: i32, en_cok: i32) -> i32 {
    if en <= 0 {
        return 1;
    }
    let mut satir = 1;
    let mut x = 0;
    let bosluk = o.genislik(" ", rol);
    for s in metin.split_whitespace() {
        let w = o.genislik(s, rol);
        if x > 0 && x + bosluk + w > en {
            satir += 1;
            x = w;
        } else {
            x += if x > 0 { bosluk + w } else { w };
        }
    }
    satir.clamp(1, en_cok)
}

/// Akış içindeki bir öğe: düğme ya da sabit genişlikli alan (renk örneği).
#[derive(Clone, Copy)]
enum Parca {
    Dugme(Oge),
    Alan(i32),
}

struct Ciz<'a> {
    t: Tema,
    dpi: u32,
    ekran: Ekran,
    o: &'a dyn Olcer,
    /// Tema önizlemesi ölçüsü (96 DPI).
    onizleme: (i32, i32),
}

impl Ciz<'_> {
    fn s(&self, v: i32) -> i32 {
        olcekle(v, self.dpi)
    }

    fn dugme_yuksekligi(&self) -> i32 {
        self.s(self.t.geometri().dugme)
    }

    /// Düğmenin doğal ölçüsü: metin ölçülür + 2 x iç boşluk (en az 96); `en_cok` aşılırsa iki satır.
    fn dugme_olcusu(&self, oge: Oge, en_cok: i32) -> (i32, i32, u8) {
        let g = self.t.geometri();
        let tur = oge.tur();
        let metin = self.o.genislik(&oge.etiket(self.ekran), YaziRolu::Dugme)
            + self.s(gosterge_genisligi(tur));
        let ic = self.s(g.dugme_yatay_ic);
        let mut en = (metin + 2 * ic).max(self.s(g.dugme_en_az));
        let mut boy = self.dugme_yuksekligi();
        if tur == DugmeTuru::TemaSecimi {
            en = en.max(self.s(self.onizleme.0 + 16));
            boy = self.s(self.onizleme.1 + 8 * 3) + self.o.satir(YaziRolu::Dugme);
        }
        let mut satir = 1;
        if en > en_cok {
            en = en_cok.max(1);
            satir = 2;
            boy += self.o.satir(YaziRolu::Dugme);
        }
        (en, boy, satir)
    }

    /// Soldan sağa akış; sağ grup aynı satıra sığmazsa yeni satırda sağa yaslanır.
    /// Döner: (düğmeler, alanlar, bitiş y).
    fn akis(
        &self,
        sol: &[Parca],
        sag: &[Parca],
        x0: i32,
        x1: i32,
        y0: i32,
        arka: Jeton,
    ) -> (Vec<YerDugme>, Vec<Dik>, i32) {
        let ara = self.s(8);
        let en_cok = (x1 - x0).max(1);
        let olc = |p: &Parca| -> (i32, i32, u8) {
            match p {
                Parca::Dugme(oge) => self.dugme_olcusu(*oge, en_cok),
                Parca::Alan(w) => (self.s(*w), self.dugme_yuksekligi(), 1),
            }
        };
        let mut dugmeler = Vec::new();
        let mut alanlar = Vec::new();
        let mut koy = |p: &Parca, d: Dik, satir: u8| match p {
            Parca::Dugme(oge) => dugmeler.push(YerDugme {
                oge: *oge,
                alan: d,
                satir,
                arka,
            }),
            Parca::Alan(_) => alanlar.push(d),
        };
        let (mut x, mut y, mut satir_boy) = (x0, y0, 0);
        for p in sol {
            let (w, h, n) = olc(p);
            if x > x0 && x + w > x1 {
                x = x0;
                y += satir_boy + ara;
                satir_boy = 0;
            }
            koy(p, Dik::yeni(x, y, w, h), n);
            x += w + ara;
            satir_boy = satir_boy.max(h);
        }
        if !sag.is_empty() {
            let olculer: Vec<_> = sag.iter().map(olc).collect();
            let toplam: i32 =
                olculer.iter().map(|o| o.0).sum::<i32>() + ara * (olculer.len() as i32 - 1);
            if x > x0 && x + toplam > x1 {
                y += satir_boy + ara;
                satir_boy = 0;
            }
            if toplam <= en_cok {
                let mut sx = x1 - toplam;
                for (p, (w, h, n)) in sag.iter().zip(&olculer) {
                    koy(p, Dik::yeni(sx, y, *w, *h), *n);
                    sx += w + ara;
                    satir_boy = satir_boy.max(*h);
                }
            } else {
                // Sağ grup tek satıra bile sığmıyor: soldan akar.
                let mut sx = x0;
                for (p, (w, h, n)) in sag.iter().zip(&olculer) {
                    if sx > x0 && sx + w > x1 {
                        sx = x0;
                        y += satir_boy + ara;
                        satir_boy = 0;
                    }
                    koy(p, Dik::yeni(sx, y, *w, *h), *n);
                    sx += w + ara;
                    satir_boy = satir_boy.max(*h);
                }
            }
        }
        (dugmeler, alanlar, y + satir_boy)
    }
}

/// Ekran düzenini hesaplar. Tüm etiketler ölçülür; düğmeler hiçbir zaman ölçülen etiketten dar olmaz.
pub fn duzen(gr: &DuzenGirdisi, o: &dyn Olcer) -> Duzen {
    let c = Ciz {
        t: gr.tema,
        dpi: gr.dpi,
        ekran: gr.ekran,
        o,
        onizleme: ONIZLEME,
    };
    let t = gr.tema;
    let yk = yerlesim(t);
    let geo = t.geometri();
    let dis = c.s(dis_bosluk(t));
    let ara = c.s(8);
    let dh = c.dugme_yuksekligi();
    let (x0, x1) = (dis, (gr.en - dis).max(dis + 1));
    let mut d = Duzen::default();

    // Başlık satırı: logo + başlık metni (+ sağda Ayarlar).
    let mut y = dis;
    let logo = c.s(if t == Tema::Mezura { 32 } else { 28 });
    d.logo = Dik::yeni(x0, y + (dh - logo) / 2, logo, logo);
    let mut baslik_sag = x1;
    if yk.ayarlar_baslikta {
        let (w, h, n) = c.dugme_olcusu(Oge::Ayarlar, x1 - x0);
        d.dugmeler.push(YerDugme {
            oge: Oge::Ayarlar,
            alan: Dik::yeni(x1 - w, y, w, h),
            satir: n,
            arka: Jeton::Zemin,
        });
        baslik_sag = x1 - w - ara;
    }
    let bx = x0 + logo + c.s(12);
    d.metinler.push(Metin {
        metin: baslik_metni(t, gr.ekran),
        alan: Dik::yeni(bx, y, (baslik_sag - bx).max(1), dh),
        rol: YaziRolu::Bolum,
        jeton: Jeton::Metin,
        kip: MetinKipi::Tek,
    });
    y += dh + ara;

    // Kalıp: sekmeler.
    if yk.sekmeler {
        let (dg, _, son) = c.akis(
            &[
                Parca::Dugme(Oge::SekmePaketler),
                Parca::Dugme(Oge::SekmeAyarlar),
            ],
            &[],
            x0,
            x1,
            y,
            Jeton::Zemin,
        );
        d.dugmeler.extend(dg);
        y = son;
        d.kutular.push(Kutu {
            alan: Dik::yeni(x0, y, x1 - x0, c.s(geo.kenar)),
            dolgu: Some(Jeton::Kenar),
            kenar: None,
            yaricap: 0,
        });
        y += c.s(12);
    }

    // Gövde başlangıcı (Kalıp oturma kayması buradan sonrakilere uygulanır).
    let govde = (d.kutular.len(), d.metinler.len(), d.dugmeler.len());

    // Mezura: büyük başlık (+ Paketler'de sağda Kur…).
    if yk.buyuk_baslik {
        let metin = buyuk_baslik_metni(gr.ekran);
        let sh = o.satir(YaziRolu::Baslik);
        let mut satir_boy = sh.max(dh);
        let mut baslik_en = x1 - x0;
        if gr.ekran == Ekran::Paketler {
            let (w, h, n) = c.dugme_olcusu(Oge::Kur, x1 - x0);
            d.dugmeler.push(YerDugme {
                oge: Oge::Kur,
                alan: Dik::yeni(x1 - w, y + (satir_boy - h).max(0) / 2, w, h),
                satir: n,
                arka: Jeton::Zemin,
            });
            satir_boy = satir_boy.max(h);
            baslik_en = x1 - x0 - w - ara;
        }
        d.metinler.push(Metin {
            metin: metin.to_string(),
            alan: Dik::yeni(x0, y, baslik_en.max(1), satir_boy),
            rol: YaziRolu::Baslik,
            jeton: Jeton::Metin,
            kip: MetinKipi::Tek,
        });
        // Etkin ekranın 4 px alt çizgisi (geçişte %50 -> %100 uzar).
        let mw = o.genislik(metin, YaziRolu::Baslik);
        let cizgi = ((mw as f32) * gr.kare.cizgi_orani.clamp(0.0, 1.0)).round() as i32;
        let cy = y + (satir_boy + sh) / 2 + c.s(2);
        d.kutular.push(Kutu {
            alan: Dik::yeni(x0, cy, cizgi.max(1), c.s(4)),
            // Her iki varyantta `metin`: sarı açıkta tek başına gösterge olamaz (§4.1).
            dolgu: Some(Jeton::Metin),
            kenar: None,
            yaricap: 0,
        });
        y = cy + c.s(4) + c.s(12);
    }

    // Alttan: durum satırı.
    let durum_en = x1 - x0;
    let durum_satir = satir_sayisi(o, gr.durum, YaziRolu::Govde, durum_en, 3);
    let durum_boy = durum_satir * o.satir(YaziRolu::Govde);
    let durum_y = gr.boy - dis - durum_boy;
    d.metinler.push(Metin {
        metin: gr.durum.to_string(),
        alan: Dik::yeni(x0, durum_y, durum_en, durum_boy),
        rol: YaziRolu::Govde,
        jeton: Jeton::Soluk,
        kip: MetinKipi::Sarili,
    });
    let mut alt = durum_y - ara;

    match gr.ekran {
        Ekran::Paketler => paketler_govdesi(&c, gr, &mut d, x0, x1, y, &mut alt),
        Ekran::Ayarlar => {
            let taban = d.clone();
            let son = ayarlar_govdesi(&c, gr, &mut d, x0, x1, y, false);
            if son > alt {
                // Sığmadı (dar pencere, büyük yazı): sıkışık önizleme ve dar satır aralığı.
                d = taban;
                let sik = Ciz {
                    onizleme: ONIZLEME_SIKISIK,
                    ..c
                };
                ayarlar_govdesi(&sik, gr, &mut d, x0, x1, y, true);
            }
        }
    }

    // Kalıp oturma: gövde sağdan kayarak yerine oturur.
    let kayma = c.s(gr.kare.kayma_px);
    if kayma != 0 {
        for k in &mut d.kutular[govde.0..] {
            k.alan.kaydir(kayma);
        }
        for m in &mut d.metinler[govde.1..] {
            m.alan.kaydir(kayma);
        }
        for b in &mut d.dugmeler[govde.2..] {
            b.alan.kaydir(kayma);
        }
        if let Some(l) = &mut d.liste {
            l.kaydir(kayma);
        }
        if let Some(r) = &mut d.renk_ornegi {
            r.kaydir(kayma);
        }
    }
    d
}

/// Kök klasör satırı: "Kök klasör: …" + sağda Kopyala. Döner: satırın yüksekliği.
fn kok_satiri(
    c: &Ciz,
    gr: &DuzenGirdisi,
    d: &mut Duzen,
    x0: i32,
    x1: i32,
    y: i32,
    arka: Jeton,
) -> i32 {
    let (w, h, n) = c.dugme_olcusu(Oge::Kopyala, x1 - x0);
    d.dugmeler.push(YerDugme {
        oge: Oge::Kopyala,
        alan: Dik::yeni(x1 - w, y, w, h),
        satir: n,
        arka,
    });
    d.metinler.push(Metin {
        metin: format!("Kök klasör: {}", gr.kok),
        alan: Dik::yeni(x0, y, (x1 - w - c.s(8) - x0).max(1), h),
        rol: YaziRolu::Govde,
        jeton: Jeton::Metin,
        kip: MetinKipi::YolKisalt,
    });
    h
}

fn tablo(c: &Ciz, d: &mut Duzen, x0: i32, x1: i32, y0: i32, y1: i32) {
    let t = c.t;
    let yk = yerlesim(t);
    let geo = t.geometri();
    let alan = Dik::yeni(x0, y0, x1 - x0, (y1 - y0).max(c.s(40)));
    match yk.tablo_kenari {
        TabloKenari::YuzeyKarti => {
            d.kutular.push(Kutu {
                alan,
                dolgu: Some(Jeton::Yuzey),
                kenar: None,
                yaricap: c.s(geo.kart_yaricap),
            });
            let (ix, iy) = (c.s(geo.kart_yaricap / 2), c.s(geo.kart_yaricap / 2));
            d.liste = Some(Dik::yeni(
                alan.x + ix,
                alan.y + iy,
                alan.en - 2 * ix,
                alan.boy - 2 * iy,
            ));
        }
        TabloKenari::InceCerceve => {
            let k = c.s(geo.kenar).max(1);
            d.kutular.push(Kutu {
                alan,
                dolgu: Some(Jeton::Yuzey),
                kenar: Some((Jeton::Kenar, k)),
                yaricap: c.s(geo.kart_yaricap),
            });
            let ic = k + c.s(1);
            d.liste = Some(Dik::yeni(
                alan.x + ic,
                alan.y + ic,
                alan.en - 2 * ic,
                alan.boy - 2 * ic,
            ));
        }
        TabloKenari::KalinUstAlt => {
            let k = c.s(geo.kenar);
            for yy in [alan.y, alan.alt() - k] {
                d.kutular.push(Kutu {
                    alan: Dik::yeni(alan.x, yy, alan.en, k),
                    dolgu: Some(Jeton::Kenar),
                    kenar: None,
                    yaricap: 0,
                });
            }
            d.liste = Some(Dik::yeni(alan.x, alan.y + k, alan.en, alan.boy - 2 * k));
        }
    }
}

fn paketler_govdesi(
    c: &Ciz,
    gr: &DuzenGirdisi,
    d: &mut Duzen,
    x0: i32,
    x1: i32,
    mut y: i32,
    alt: &mut i32,
) {
    let yk = yerlesim(c.t);
    let ara = c.s(8);
    let pd = c.s(6);
    let dugme = |o: Oge| Parca::Dugme(o);
    match yk.eylemler {
        EylemYeri::UstAracCubugu => {
            let (dg, _, son) = c.akis(
                &[
                    dugme(Oge::Kur),
                    dugme(Oge::Kaldir),
                    dugme(Oge::Guncelle),
                    dugme(Oge::Rapor),
                ],
                &[],
                x0,
                x1,
                y,
                Jeton::Zemin,
            );
            d.dugmeler.extend(dg);
            y = son + ara;
            // Altta: kök klasör satırı.
            let h = c.dugme_yuksekligi();
            let ky = *alt - h;
            kok_satiri(c, gr, d, x0, x1, ky, Jeton::Zemin);
            *alt = ky - ara;
            tablo(c, d, x0, x1, y, *alt);
        }
        EylemYeri::AltIslemCubugu => {
            // Kök klasör bandı tablodan önce (çerçeveli yüzey bandı).
            let h = c.dugme_yuksekligi();
            let bant = Dik::yeni(x0, y, x1 - x0, h + 2 * pd);
            d.kutular.push(Kutu {
                alan: bant,
                dolgu: Some(Jeton::Yuzey),
                kenar: Some((Jeton::Kenar, c.s(1).max(1))),
                yaricap: c.s(c.t.geometri().kart_yaricap),
            });
            kok_satiri(c, gr, d, x0 + pd * 2, x1 - pd, y + pd, Jeton::Yuzey);
            y = bant.alt() + ara;
            // İşlem çubuğu altta: önce yüksekliği ölçülür.
            let sol = [dugme(Oge::Kur), dugme(Oge::Kaldir), dugme(Oge::Guncelle)];
            let sag = [dugme(Oge::Rapor)];
            let (_, _, olcu_son) = c.akis(&sol, &sag, x0 + pd, x1 - pd, 0, Jeton::Yuzey);
            let bant_boy = olcu_son + 2 * pd;
            let by = *alt - bant_boy;
            d.kutular.push(Kutu {
                alan: Dik::yeni(x0, by, x1 - x0, bant_boy),
                dolgu: Some(Jeton::Yuzey),
                kenar: Some((Jeton::Kenar, c.s(1).max(1))),
                yaricap: c.s(c.t.geometri().kart_yaricap),
            });
            let (dg, _, _) = c.akis(&sol, &sag, x0 + pd, x1 - pd, by + pd, Jeton::Yuzey);
            d.dugmeler.extend(dg);
            *alt = by - ara;
            tablo(c, d, x0, x1, y, *alt);
        }
        EylemYeri::KurUstSagDigerleriAlt => {
            // Kur… büyük başlıkta yerleşti. Altta: durum, üstünde kök, üstünde eylemler.
            let h = c.dugme_yuksekligi();
            let ky = *alt - h;
            kok_satiri(c, gr, d, x0, x1, ky, Jeton::Zemin);
            *alt = ky - ara;
            let sol = [dugme(Oge::Kaldir), dugme(Oge::Guncelle)];
            let sag = [dugme(Oge::Rapor)];
            let (_, _, olcu_son) = c.akis(&sol, &sag, x0, x1, 0, Jeton::Zemin);
            let ey = *alt - olcu_son;
            let (dg, _, _) = c.akis(&sol, &sag, x0, x1, ey, Jeton::Zemin);
            d.dugmeler.extend(dg);
            *alt = ey - c.s(12);
            tablo(c, d, x0, x1, y, *alt);
        }
    }
}

/// Ayarlar gövdesi; döner: son öğenin alt kenarı.
fn ayarlar_govdesi(
    c: &Ciz,
    gr: &DuzenGirdisi,
    d: &mut Duzen,
    x0: i32,
    x1: i32,
    mut y: i32,
    sikisik: bool,
) -> i32 {
    let ara = c.s(8);
    let satir_arasi = if sikisik { c.s(6) } else { ara + c.s(4) };
    let o = c.o;
    // Bölüm başlığı. Mezura'da büyük başlık zaten "Ayarlar"; sığmazsa bölüm başlığı atlanır.
    if !(sikisik && yerlesim(c.t).buyuk_baslik) {
        let bh = o.satir(YaziRolu::Bolum);
        d.metinler.push(Metin {
            metin: "Görünüm".into(),
            alan: Dik::yeni(x0, y, x1 - x0, bh),
            rol: YaziRolu::Bolum,
            jeton: Jeton::Metin,
            kip: MetinKipi::Tek,
        });
        y += bh + ara;
    }
    // Etiket sütunu: en uzun etiket ölçülür (kesilmez).
    let etiket_en = AYAR_ETIKETLERI
        .iter()
        .chain(sikisik.then_some(&BIRLESIK_ETIKET))
        .map(|e| o.genislik(e, YaziRolu::Govde))
        .max()
        .unwrap_or(0)
        + c.s(24);
    let kx = (x0 + etiket_en).min(x1 - c.s(96));
    let dh = c.dugme_yuksekligi();
    let dugme = |o: Oge| Parca::Dugme(o);
    let mut satirlar: Vec<(&str, Vec<Parca>)> = vec![
        (
            AYAR_ETIKETLERI[0],
            Tema::HEPSI.iter().map(|t| dugme(Oge::Tema(*t))).collect(),
        ),
        (
            AYAR_ETIKETLERI[1],
            Mod::HEPSI.iter().map(|m| dugme(Oge::Mod(*m))).collect(),
        ),
        (
            AYAR_ETIKETLERI[2],
            vec![
                Parca::Alan(48),
                dugme(Oge::RenkSec),
                dugme(Oge::RenkSifirla),
            ],
        ),
        (AYAR_ETIKETLERI[3], vec![dugme(Oge::HareketiAzalt)]),
        (
            AYAR_ETIKETLERI[4],
            vec![dugme(Oge::TanitimTekrar), dugme(Oge::Varsayilan)],
        ),
    ];
    if sikisik {
        // Sığmayan pencerede Hareket ve Tanıtım tek satırda (düğmeler yine ölçülür, gerekirse sarılır).
        let tanitim = satirlar.pop().map(|s| s.1).unwrap_or_default();
        if let Some(son) = satirlar.last_mut() {
            son.0 = BIRLESIK_ETIKET;
            son.1.extend(tanitim);
        }
    }
    let kucuk = o.satir(YaziRolu::Kucuk);
    for (i, (etiket, parcalar)) in satirlar.iter().enumerate() {
        d.metinler.push(Metin {
            metin: (*etiket).to_string(),
            alan: Dik::yeni(x0, y, kx - x0, dh),
            rol: YaziRolu::Govde,
            jeton: Jeton::Metin,
            kip: MetinKipi::Tek,
        });
        let (dg, alanlar, son) = c.akis(parcalar, &[], kx, x1, y, Jeton::Zemin);
        d.dugmeler.extend(dg);
        if let Some(a) = alanlar.first() {
            d.renk_ornegi = Some(*a);
        }
        y = son;
        let not = match i {
            0 if gr.yk_notu => Some((YK_NOTU, Jeton::Soluk)),
            2 => gr.vurgu_notu,
            _ => None,
        };
        if let Some((metin, jeton)) = not {
            let n = satir_sayisi(o, metin, YaziRolu::Kucuk, x1 - kx, 3);
            y += c.s(4);
            d.metinler.push(Metin {
                metin: metin.to_string(),
                alan: Dik::yeni(kx, y, x1 - kx, n * kucuk),
                rol: YaziRolu::Kucuk,
                jeton,
                kip: MetinKipi::Sarili,
            });
            y += n * kucuk;
        }
        y += satir_arasi;
    }
    y - satir_arasi
}

/// Liste sütun genişlikleri: başlıklar ölçülür; son sütun (Durum) kalan genişliği alır.
pub fn sutun_genislikleri(t: Tema, dpi: u32, toplam: i32, o: &dyn Olcer) -> [i32; 4] {
    let ic = olcekle(24, dpi) + olcekle(serit_genisligi(t), dpi);
    let taban = [180, 150, 130];
    let mut g = [0i32; 4];
    for i in 0..3 {
        g[i] = olcekle(taban[i], dpi).max(o.genislik(SUTUNLAR[i], YaziRolu::GovdeKalin) + ic);
    }
    let kalan = toplam - g[0] - g[1] - g[2];
    g[3] = kalan
        .max(o.genislik(SUTUNLAR[3], YaziRolu::GovdeKalin) + ic)
        .max(olcekle(200, dpi));
    g
}

#[cfg(test)]
#[path = "testler_pencere_tema.rs"]
mod testler;
