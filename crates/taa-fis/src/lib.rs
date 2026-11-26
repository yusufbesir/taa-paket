//! taa-fis (TaaFiş): terzi fişi modeli, eklemeli defter (JSONL), durum takibi, termal (ESC/POS) ve düz metin çıktı.
//! Telefon sayfası, QR eşleştirme, OCR ve ağ sunucusu bu crate'in DIŞINDADIR (sonraki aşamalar); burada saf çekirdek var.
//! ESC/POS çıktısı gerçek yazıcıda denenmedi; yazıcı kod sayfası CP857 (Türkçe) varsayılır.
//! Defter silmez: durum değişikliği yeni satırdır. Özellik anahtarı: "fis" (varsayılan açık).
//! Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

use serde::{Deserialize, Serialize};

pub const OZELLIK: &str = "fis";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Durum {
    Bekliyor,
    /// Ürün terzihaneye verildi.
    Terzihanede,
    Hazir,
    Teslim,
    Iptal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kalem {
    /// Ürün (ceket, pantolon...).
    pub urun: String,
    /// Yapılacak işlem (paça kısalt, bel daralt...). Katalogdan seçildiyse katalog adı.
    pub islem: String,
    #[serde(default)]
    pub not: String,
    /// Katalog işlem kodu (taa-terzi); yoksa serbest metin işlem.
    #[serde(default)]
    pub islem_kodu: Option<String>,
    /// Düz paça / duble paça gibi seçenek.
    #[serde(default)]
    pub varyant: Option<String>,
    /// Yapılacak ölçü, milimetre (2,5 cm = 25).
    #[serde(default)]
    pub olcu_mm: Option<i32>,
    /// İçeride bırakılan pay, milimetre (tekrar uzatma için).
    #[serde(default)]
    pub pay_mm: Option<i32>,
    /// Sınır aşımında usta onayı verildi mi?
    #[serde(default)]
    pub usta_onayli: bool,
    /// Ölçü iğneyle işaretlendi (cm bilinmiyor).
    #[serde(default)]
    pub igneli: bool,
    /// Ürün rengi: `RENKLER` anahtarı (siyah, bej...). Müşteri takip sayfasında çevrilerek görünür.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub renk: Option<String>,
    /// Aynı üründen kaç adet (ör. 2 pantolon). Boş = 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adet: Option<u32>,
}

/// Fişte sonradan yapılan düzeltme (kalem ekleme/çıkarma, teslim şekli, fiş bağlama). Defterde kalır, silinmez.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Duzeltme {
    /// kalem-ekle, kalem-cikar, teslim-sekli, bagla, bag-coz
    pub ne: String,
    pub ayrinti: String,
    /// Yapan cihaz/kişi (cihaz adı ya da kişi kimliği).
    pub kim: String,
    pub tarih: String,
}

/// Renk anahtarı ve fişe basılan Türkçe adı. Anahtar değişmez (bulut sayfası çevirisi buna bağlı).
pub const RENKLER: &[(&str, &str)] = &[
    ("siyah", "Siyah"),
    ("lacivert", "Lacivert"),
    ("antrasit", "Antrasit"),
    ("gri", "Gri"),
    ("acik-gri", "Açık gri"),
    ("fume", "Füme"),
    ("bej", "Bej"),
    ("krem", "Krem"),
    ("haki", "Haki"),
    ("kahverengi", "Kahverengi"),
    ("taba", "Taba"),
    ("mavi", "Mavi"),
    ("acik-mavi", "Açık mavi"),
    ("beyaz", "Beyaz"),
    ("bordo", "Bordo"),
    ("yesil", "Yeşil"),
    ("mor", "Mor"),
    ("pembe", "Pembe"),
    ("kirmizi", "Kırmızı"),
    ("sari", "Sarı"),
    ("turuncu", "Turuncu"),
    ("desenli", "Desenli"),
];

/// Renk anahtarından Türkçe ad.
pub fn renk_adi(anahtar: &str) -> Option<&'static str> {
    RENKLER.iter().find(|(k, _)| *k == anahtar).map(|(_, a)| *a)
}

/// Personel adını fişe kısa basar: ilk ad + soyadın baş harfi ("Ahmet Selim Yılmaz" -> "Ahmet Y.").
/// Tek kelimelik adlar olduğu gibi kalır.
pub fn kisa_ad(ad: &str) -> String {
    let k: Vec<&str> = ad.split_whitespace().collect();
    match k.as_slice() {
        [] => String::new(),
        [tek] => (*tek).to_string(),
        [ilk, .., son] => {
            let bas: String = son
                .chars()
                .next()
                .map(|c| c.to_uppercase().collect())
                .unwrap_or_default();
            format!("{ilk} {bas}.")
        }
    }
}

/// Tek kalemi denetler (fiş yazılırken ve sonradan kalem eklenirken aynı kural).
pub fn kalem_denetle(
    k: &Kalem,
    katalog: &taa_terzi::Katalog,
    serbest_islem: bool,
    olcu_zorunlu: bool,
) -> Result<Vec<String>, String> {
    let mut uyarilar = Vec::new();
    match &k.islem_kodu {
        Some(kod) => uyarilar.extend(katalog.kontrol(
            kod,
            k.olcu_mm,
            k.varyant.as_deref(),
            k.usta_onayli,
            olcu_zorunlu,
            k.igneli,
        )?),
        None if !serbest_islem => {
            return Err("işlem listeden seçilmeli (serbest işlem kapalı)".into())
        }
        None => {}
    }
    if matches!(k.pay_mm, Some(p) if p < 0) {
        return Err("pay negatif olamaz".into());
    }
    if k.urun.trim().is_empty() || k.islem.trim().is_empty() {
        return Err("her kalemde ürün ve işlem yazılmalı".into());
    }
    if let Some(r) = &k.renk {
        if renk_adi(r).is_none() {
            return Err(format!("renk listede yok: {r}"));
        }
    }
    if matches!(k.adet, Some(a) if a == 0 || a > 20) {
        return Err("adet 1-20 olmalı".into());
    }
    Ok(uyarilar)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeslimDegisti {
    pub eski: String,
    pub yeni: String,
    pub neden: String,
    pub tarih: String,
}

/// Telefonla atılan imza: kim (eşleşmiş telefonun kullanıcısı), rol (usta, teslim...), tarih.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Imza {
    pub kim: String,
    pub rol: String,
    pub tarih: String,
    /// İmza saati (SS:DD:SS) ve cihaz kimliği; eski satırlarda boş.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub saat: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub cihaz: String,
    /// "dogrulanmis" (Telegram kimliği) ya da "beyan" (listeden seçilen kişi); eski satırlarda boş.
    /// Beyan usta performansına ve puana sayılmaz.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kimlik_duzeyi: String,
    /// Kayıtta ad değil kişi kimliği tutulur (havuz kişi kaydı); eski satırlarda boş.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub okutan_kisi_id: String,
}

/// Ürünü teslim alanın ilişkisi: anahtar ve fişe basılan Türkçe adı.
pub const TESLIM_ILISKILERI: &[(&str, &str)] = &[
    ("kendisi", "kendisi"),
    ("yakini", "yakını"),
    ("diger", "diğer"),
];

/// Teslim alan kişinin şifreli bilgisi (Yusuf 2026-11-26: "bu bilgiler veri tabanında şifreli olarak tutulsun").
/// `c` = AES-256-GCM(yerel anahtar, aad = fiş no) ile şifrelenmiş JSON {ad, tel, cihaz, ip, zaman, yontem}.
/// Şifreleme/çözme sunucudadır (taa-fis-sunucu `teslim_sifre`); bu crate yalnız biçimi taşır ve denetler.
/// `ad_indeks`/`tel_indeks`: kör indeks (HMAC-SHA256, ilk 16 onaltılık hane); arama içindir, geri çevrilemez.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeslimSifreli {
    pub surum: u8,
    /// 12 baytlık IV, onaltılık (24 hane).
    pub iv: String,
    /// Şifreli metin + 16 baytlık etiket, base64.
    pub c: String,
    pub ad_indeks: String,
    /// Telefon yoksa (ekran imzası yolu) boş.
    #[serde(default)]
    pub tel_indeks: String,
}

impl TeslimSifreli {
    /// Biçim denetimi (anahtarsız; içerik doğrulaması çözmede GCM etiketiyle yapılır).
    pub fn gecerli(&self) -> bool {
        let hex = |s: &str, n: usize| {
            s.len() == n
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        self.surum == 1
            && hex(&self.iv, 24)
            && self.c.len() >= 24
            && self.c.len() <= 8192
            && self
                .c
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/' || b == b'=')
            && hex(&self.ad_indeks, 16)
            && (self.tel_indeks.is_empty() || hex(&self.tel_indeks, 16))
    }
}

/// Şifresi çözülmüş, yalnız bellekte duran görünüm (basım ve ekran için). Defterde YAZILMAZ (`serde(skip)`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TeslimGorunen {
    /// Teslim alanın adı soyadı (basımda yalnız maskeli "A*** S***" biçimi kullanılır).
    pub ad: String,
    /// "qr" ya da "ekran-imza".
    pub yontem: String,
}

/// Ürünü teslim alan kişinin kaydı (fişli/fişsiz teslim, karekod onayı ya da ekran imzası). Defterde kalır, silinmez.
/// Fişsiz teslimde doğrulama telefonun son 4 hanesiyle yapılır; son 4 hane SAKLANMAZ, yalnız yöntemi yazılır.
/// Yeni kayıtlarda kişisel veri (ad, telefon, cihaz, IP) yalnız `sifreli` alanındadır; `teslim_alan` boş kalır
/// (eski kayıtlar için alan durur). Bu kayıt müşteri takip (bulut) kaydına gitmez.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeslimKaydi {
    /// Teslim alanın adı soyadı (yalnız eski kayıtlar; yenilerde boş, ad `sifreli` içindedir).
    #[serde(default)]
    pub teslim_alan: String,
    /// `TESLIM_ILISKILERI` anahtarı: kendisi, yakini, diger.
    pub iliski: String,
    /// Müşteri fişiyle mi geldi?
    pub fisli: bool,
    /// "fis" ya da "telefon-son4".
    pub dogrulama: String,
    /// İmza PNG'sinin SHA-256 özeti (onaltılık); imza alınmadıysa boş.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub imza_sha256: String,
    /// Veri klasörüne göre imza dosyası (yeni: "imzalar/F-20261126-001-1.png.enc", yerel anahtarla AES-256-GCM
    /// şifreli; eski kayıtlarda ".png"); imza yoksa boş. `imza_sha256` ŞİFRESİZ PNG'nin özetidir.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub imza_dosyasi: String,
    /// İmzanın nereden alındığı: "qr_imza" (müşteri karekod sayfasında kendi telefonuna parmakla çizdi) ya da
    /// "cihaz_imza" (mağaza telefonu/tablet ya da bilgisayar ekranındaki imza alanı). İmza yoksa boş.
    /// Kişisel veri değildir (yalnız yöntem).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub imza_yontemi: String,
    /// Teslimi yapan cihaz/kişi (kişi kimliği ya da cihaz adı).
    pub kim: String,
    pub tarih: String,
    /// SS:DD:SS
    #[serde(default)]
    pub saat: String,
    /// Bağlı fiş birlikte teslim edildiyse teslimin yapıldığı ana fiş no (imza ve doğrulama onda);
    /// fişin kendi teslimiyse boş.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub ana_fis: String,
    /// Şifreli teslim alan bilgisi (yeni kayıtlar).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sifreli: Option<TeslimSifreli>,
    /// Ortak teslim kimliği ("T-20261126-K7Q2MX"): aynı teslimatta (teslim sepeti ya da bağlı fişlerle birlikte)
    /// teslim edilen bütün fişlerin kayıtlarında aynıdır. Kişisel veri değildir. Eski kayıtlarda boş.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub teslim_id: String,
    /// Sunucunun çözüp doldurduğu görünüm; diske ve JSON'a hiç yazılmaz.
    #[serde(skip)]
    pub gorunen: Option<TeslimGorunen>,
}

impl TeslimKaydi {
    /// Karekodla (müşterinin kendi telefonundan) onaylanmış teslim mi?
    pub fn qr_onayli(&self) -> bool {
        self.dogrulama == "qr" || self.dogrulama == "qr+telefon-son4"
    }
}

/// İmza yöntemleri (TeslimKaydi.imza_yontemi).
pub const IMZA_YONTEMLERI: &[&str] = &["qr_imza", "cihaz_imza"];

/// Adı maskeler (Yusuf 2026-11-26: "daha sonra maskeli olarak görünsün"): her kelimenin yalnız ilk harfi kalır,
/// ardından "***". "Mehmet Yılmaz" -> "M*** Y***". Ekranda (yetkisiz görünüm) ve basılı nüshalarda kullanılır.
pub fn ad_maskele(ad: &str) -> String {
    ad.split_whitespace()
        .filter_map(|k| k.chars().next())
        .map(|c| match c {
            // Türkçe: noktalı küçük i -> İ (genel büyük harf kuralı "I" verirdi).
            'i' => "İ***".to_string(),
            c => format!("{}***", c.to_uppercase()),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// İlişki anahtarının fişe basılan adı.
pub fn iliski_adi(anahtar: &str) -> Option<&'static str> {
    TESLIM_ILISKILERI
        .iter()
        .find(|(k, _)| *k == anahtar)
        .map(|(_, a)| *a)
}

/// Kişi adı karakterleri: harf, birleşik işaret (Arapça hareke, Hintçe virama/ünlü işaretleri: Unicode M sınıfı),
/// Farsça/Hint yazılarındaki sıfır genişlikli birleştirmez/birleştirir (U+200C ZWNJ, U+200D ZWJ), boşluk, tire,
/// kesme işareti, nokta. Çift yönlü yazı işaretleri (U+200E/F, U+202A..E, U+2066..9), diğer sıfır genişlikli
/// karakterler (U+200B, U+2060, U+FEFF), rakamlar ve HTML işaretleri reddedilir (ad ekranda/fişte yanıltıcı görünmesin).
/// Bulut onay sayfasının kuralı (`bulut/musteri-takip/ortak/onay.mjs` adDogrula: `[\p{L}\p{M} .'\-ZWNJ ZWJ]`) bunun
/// alt kümesidir: bulutun kabul ettiği her ad burada da geçer (karekod onayı kasada "ad geçersiz"e düşmez).
pub fn ad_karakterleri_gecerli(ad: &str) -> bool {
    use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory};
    ad.chars().all(|c| {
        c.is_alphabetic()
            || c.general_category_group() == GeneralCategoryGroup::Mark
            || matches!(c, ' ' | '-' | '\'' | '.' | '’' | '\u{200C}' | '\u{200D}')
    })
}

/// Görünmez biçim karakteri var mı (çift yönlü yazı işaretleri, sıfır genişlikli karakterler, BOM)?
/// Rakam ve işaret serbest olan alanlarda (cihaz adı vb.) kullanılır.
pub fn gorunmez_karakter_var(s: &str) -> bool {
    s.chars().any(|c| {
        c.is_control()
            || matches!(c,
                '\u{00AD}' | '\u{061C}' | '\u{180E}' | '\u{200B}'..='\u{200F}'
                | '\u{202A}'..='\u{202E}' | '\u{2060}'..='\u{206F}' | '\u{FEFF}'
                | '\u{FFF9}'..='\u{FFFB}')
    })
}

/// Teslim alan adını ve ilişkiyi denetler (sunucu dosya yazmadan önce de çağırır). Ad soyad en az iki kelime.
pub fn teslim_alan_denetle(teslim_alan: &str, iliski: &str) -> Result<(), String> {
    let ad = teslim_alan.trim();
    if ad.split_whitespace().count() < 2 {
        return Err("teslim alanın adı ve soyadı yazılmalı".into());
    }
    if ad.chars().count() > 80 {
        return Err("teslim alanın adı en çok 80 harf olmalı".into());
    }
    if !ad_karakterleri_gecerli(ad) {
        return Err(
            "teslim alanın adında yalnız harf, boşluk, tire, kesme işareti ve nokta olabilir"
                .into(),
        );
    }
    if iliski_adi(iliski).is_none() {
        return Err("ilişki: kendisi, yakini ya da diger olmalı".into());
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fis {
    pub no: String,
    pub tarih: String,
    pub musteri: String,
    pub telefon: String,
    pub kalemler: Vec<Kalem>,
    #[serde(default)]
    pub olcu_notu: String,
    #[serde(default)]
    pub prova: String,
    pub teslim: String,
    /// HH:MM
    #[serde(default = "varsayilan_saat")]
    pub teslim_saati: String,
    pub ucret_kurus: i64,
    pub on_odeme_kurus: i64,
    #[serde(default)]
    pub usta: String,
    #[serde(default)]
    pub satis_danismani: String,
    #[serde(default)]
    pub olcu_alan: String,
    /// Kişi kaydındaki kimlikler (ör. "K-1a2b3c4d" ya da "U-…"). Yeni fişte ad yerine bunlar saklanır;
    /// eski satırlarda boştur ve ad alanları geçerlidir.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub satis_danismani_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub olcu_alan_id: String,
    /// Fişi düzenleyen kişinin kimliği (fişe basılmaz).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub duzenleyen_id: String,
    /// Kargo ile gönderilecek mi? Fişte büyük yazılır (not kâğıdı düşebildiği için).
    #[serde(default)]
    pub kargo: bool,
    #[serde(default)]
    pub kargo_notu: String,
    /// Kargo işaretliyse zorunlu: alıcı ad-soyad, açık adres, alıcı telefonu.
    #[serde(default)]
    pub kargo_ad: String,
    #[serde(default)]
    pub kargo_adres: String,
    #[serde(default)]
    pub kargo_telefon: String,
    /// Müşteri "hazır" bilgilendirmesine izin verdi mi? (KVKK; varsayılan hayır)
    #[serde(default)]
    pub bildirim_izni: bool,
    #[serde(default)]
    pub teslim_degisiklikleri: Vec<TeslimDegisti>,
    pub durum: Durum,
    /// Hazır ürünün asılı olduğu stant/raf kodu (QR okutularak yazılır).
    #[serde(default)]
    pub raf: String,
    #[serde(default)]
    pub imzalar: Vec<Imza>,
    /// Fişin yazıldığı şube (kısa ad ya da kod). Eski satırlarda boş; boş = şube bilinmiyor.
    #[serde(default)]
    pub sube: String,
    /// Sonradan yapılan düzeltmeler (defterden türetilir).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub duzeltmeler: Vec<Duzeltme>,
    /// Birlikte teslim edilecek bağlı fişler (aynı anda alışveriş yapanlar; defterden türetilir).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bagli: Vec<String>,
    /// Ürünü teslim alan kişi (defterden türetilir; eski fişlerde yok).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teslim_kaydi: Option<TeslimKaydi>,
}

fn varsayilan_saat() -> String {
    "17:00".into()
}

impl Fis {
    pub fn kalan_kurus(&self) -> i64 {
        self.ucret_kurus - self.on_odeme_kurus
    }
}

/// Yeni fiş girdisi (numara ve durum defter tarafından verilir).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Girdi {
    #[serde(default)]
    pub tarih: String,
    /// Fişi yazan şube; boş bırakılırsa fiş şubesiz kaydolur.
    #[serde(default)]
    pub sube: String,
    pub musteri: String,
    pub telefon: String,
    pub kalemler: Vec<Kalem>,
    #[serde(default)]
    pub olcu_notu: String,
    #[serde(default)]
    pub prova: String,
    pub teslim: String,
    #[serde(default = "varsayilan_saat")]
    pub teslim_saati: String,
    pub ucret_kurus: i64,
    #[serde(default)]
    pub on_odeme_kurus: i64,
    #[serde(default)]
    pub usta: String,
    #[serde(default)]
    pub satis_danismani: String,
    #[serde(default)]
    pub olcu_alan: String,
    /// Kişi kaydındaki kimlikler (ör. "K-1a2b3c4d" ya da "U-…"). Yeni fişte ad yerine bunlar saklanır;
    /// eski satırlarda boştur ve ad alanları geçerlidir.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub satis_danismani_id: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub olcu_alan_id: String,
    /// Fişi düzenleyen kişinin kimliği (fişe basılmaz).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub duzenleyen_id: String,
    /// Kargo ile gönderilecek mi? Fişte büyük yazılır (not kâğıdı düşebildiği için).
    #[serde(default)]
    pub kargo: bool,
    #[serde(default)]
    pub kargo_notu: String,
    /// Kargo işaretliyse zorunlu: alıcı ad-soyad, açık adres, alıcı telefonu.
    #[serde(default)]
    pub kargo_ad: String,
    #[serde(default)]
    pub kargo_adres: String,
    #[serde(default)]
    pub kargo_telefon: String,
    /// Müşteri "hazır" bilgilendirmesine izin verdi mi? (KVKK; varsayılan hayır)
    #[serde(default)]
    pub bildirim_izni: bool,
}

pub fn tarih_gecerli(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && t.char_indices()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && matches!(t[5..7].parse::<u32>(), Ok(1..=12))
        && matches!(t[8..10].parse::<u32>(), Ok(1..=31))
}

/// Yalnız rakamlar kalır; 10 hane (5xx...) ya da 0 ile 11 hane kabul edilir. "05321234567" biçimine çevirir.
pub fn telefon_duzelt(t: &str) -> Result<String, String> {
    let r: String = t.chars().filter(char::is_ascii_digit).collect();
    let r = r
        .strip_prefix("90")
        .filter(|x| x.len() == 10)
        .map(str::to_string)
        .unwrap_or(r);
    match r.len() {
        10 if r.starts_with('5') => Ok(format!("0{r}")),
        11 if r.starts_with("05") => Ok(r),
        _ => Err(format!("geçersiz cep telefonu: '{t}'")),
    }
}

/// Şube kodu: 3 büyük harf (şube öneki) + 2 rakam (sıra), ör. `MRK01`, `PND01`, `ANK02`.
/// Önekler `docs/sube-sozlesmesi.md` §3'teki `onek` sütunudur; sıra numarası aynı şehirde ikinci
/// şubeye ve taşınmalara yer açar (Yusuf kararı 2026-11-15 21:37).
pub fn sube_kodu_gecerli(k: &str) -> bool {
    let b = k.as_bytes();
    b.len() == 5
        && b[..3].iter().all(u8::is_ascii_uppercase)
        && b[3..].iter().all(u8::is_ascii_digit)
        && &k[3..] != "00"
}

/// Fiş girdisini denetler; telefonu düzeltilmiş halde ve kalem uyarılarıyla döndürür.
pub fn dogrula(
    g: &Girdi,
    katalog: &taa_terzi::Katalog,
    serbest_islem: bool,
    olcu_zorunlu: bool,
) -> Result<(String, Vec<String>), String> {
    if !sube_kodu_gecerli(g.sube.trim()) {
        return Err("şube kodu olmadan fiş yazılamaz (3 büyük harf + 2 rakam; ör. MRK01)".into());
    }
    if g.musteri.trim().is_empty() {
        return Err("müşteri adı gerekli".into());
    }
    let dolu = |ad: &str, id: &str| !ad.trim().is_empty() || !id.trim().is_empty();
    if !dolu(&g.satis_danismani, &g.satis_danismani_id) || !dolu(&g.olcu_alan, &g.olcu_alan_id) {
        return Err("satış danışmanı ve ölçü alan kişi yazılmalı".into());
    }
    if !saat_gecerli(&g.teslim_saati) {
        return Err("teslim saati SS:DD olmalı".into());
    }
    if g.kalemler.is_empty() {
        return Err("her kalemde ürün ve işlem yazılmalı (en az bir kalem)".into());
    }
    let mut uyarilar = Vec::new();
    for k in &g.kalemler {
        uyarilar.extend(kalem_denetle(k, katalog, serbest_islem, olcu_zorunlu)?);
    }
    if !tarih_gecerli(&g.tarih)
        || !tarih_gecerli(&g.teslim)
        || (!g.prova.is_empty() && !tarih_gecerli(&g.prova))
    {
        return Err("tarihler YYYY-AA-GG olmalı".into());
    }
    if g.teslim < g.tarih || (!g.prova.is_empty() && (g.prova < g.tarih || g.prova > g.teslim)) {
        return Err(
            "prova/teslim tarihi fiş tarihinden önce olamaz; prova teslimden sonra olamaz".into(),
        );
    }
    if g.ucret_kurus < 0 || g.on_odeme_kurus < 0 || g.on_odeme_kurus > g.ucret_kurus {
        return Err("ücret/ön ödeme geçersiz (ön ödeme ücretten fazla olamaz)".into());
    }
    Ok((telefon_duzelt(&g.telefon)?, uyarilar))
}

pub fn saat_gecerli(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 5
        && b[2] == b':'
        && matches!(s[0..2].parse::<u32>(), Ok(0..=23))
        && matches!(s[3..5].parse::<u32>(), Ok(0..=59))
}

mod basim;
mod defter;
#[cfg(test)]
mod sanal_baski;
pub use basim::*;
pub use defter::*;

#[cfg(test)]
mod testler {
    use super::*;

    fn kalem(urun: &str, islem: &str, not: &str) -> Kalem {
        Kalem {
            urun: urun.into(),
            islem: islem.into(),
            not: not.into(),
            islem_kodu: None,
            varyant: None,
            olcu_mm: None,
            pay_mm: None,
            usta_onayli: false,
            igneli: false,
            renk: None,
            adet: None,
        }
    }

    fn kat() -> taa_terzi::Katalog {
        taa_terzi::Katalog::varsayilan()
    }

    fn girdi() -> Girdi {
        Girdi {
            sube: "MRK01".into(),
            tarih: "2026-11-04".into(),
            musteri: " Ayşe Çelik ".into(),
            telefon: "+90 532 123 45 67".into(),
            kalemler: vec![
                kalem("Pantolon", "paça kısalt", "2 cm"),
                kalem("Ceket", "kol kısalt", ""),
            ],
            olcu_notu: "bel 84".into(),
            prova: "2026-11-07".into(),
            teslim: "2026-11-10".into(),
            teslim_saati: "17:00".into(),
            ucret_kurus: 85000,
            on_odeme_kurus: 20000,
            usta: "Hasan".into(),
            satis_danismani: "Mert".into(),
            olcu_alan: "Kadir Y.".into(),
            satis_danismani_id: String::new(),
            olcu_alan_id: String::new(),
            duzenleyen_id: String::new(),
            kargo: false,
            kargo_notu: String::new(),
            kargo_ad: String::new(),
            kargo_adres: String::new(),
            kargo_telefon: String::new(),
            bildirim_izni: false,
        }
    }

    fn yol() -> std::path::PathBuf {
        taa_dosya::gecici_klasor().unwrap().join("fis.jsonl")
    }

    #[test]
    fn takip_karekodu_yalniz_musteri_nushasinda() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let f = Defter { yol: &y }.ekle(girdi()).unwrap();
        let personel = "http://x/telefon#fis=PERSONEL";
        let t = Takip {
            adres: "https://takip.example/t/abcdefghjkmnpqrs",
            yedek: Some("https://yedek.example/t/abcdefghjkmnpqrs"),
            cumle: "Ürününüzün durumunu bu karekodu okutarak takip edebilirsiniz.",
        };
        let b = escpos_nushalar_takipli(&f, 42, personel, None, Some(&t));
        let sayi = |hedef: &[u8]| b.windows(hedef.len()).filter(|w| *w == hedef).count();
        assert_eq!(
            sayi(t.adres.as_bytes()),
            1,
            "takip karekodu yalnız müşteri nüshasında"
        );
        assert_eq!(
            sayi(personel.as_bytes()),
            1,
            "personel karekodu yalnız terzihane nüshasında"
        );
        assert_eq!(sayi(b"yedek.example"), 1);
        let cumle: Vec<u8> = "Ürününüzün durumunu".chars().map(cp857).collect();
        assert_eq!(sayi(&cumle), 1);
        // takip yoksa eski davranış: iki nüshada da personel karekodu
        let c = escpos_nushalar_takipli(&f, 42, personel, None, None);
        assert_eq!(
            c.windows(personel.len())
                .filter(|w| *w == personel.as_bytes())
                .count(),
            2
        );
    }

    #[test]
    fn aydinlatma_yalniz_musteri_nushasinda() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let f = Defter { yol: &y }.ekle(girdi()).unwrap();
        let url = "http://x/aydinlatma";
        let b = escpos_nushalar_tam(&f, 42, "http://x/telefon#fis=F", Some(url));
        let sayi = |hedef: &[u8]| b.windows(hedef.len()).filter(|w| *w == hedef).count();
        assert_eq!(sayi(url.as_bytes()), 1); // yalnız müşteri nüshası
        assert_eq!(
            sayi(
                "Kişisel verileriniz"
                    .chars()
                    .map(cp857)
                    .collect::<Vec<_>>()
                    .as_slice()
            ),
            1
        );
        // verilmezse hiçbir nüshada yok
        let c = escpos_nushalar_tam(&f, 42, "q", None);
        assert!(!c.windows(19).any(|w| w == url.as_bytes()));
        assert_eq!(
            c.windows(4).filter(|w| *w == [0x1D, 0x56, 0x42, 0]).count(),
            2
        );
    }

    #[test]
    fn raf_ve_imza() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let d = Defter { yol: &y };
        let f = d.ekle(girdi()).unwrap();
        assert!(d.raf_ata(&f.no, "", "Ali", "2026-11-05").is_err());
        assert!(d.raf_ata("yok", "A-1", "Ali", "2026-11-05").is_err());
        d.raf_ata(&f.no, "a-12", "Ali", "2026-11-05").unwrap();
        d.raf_ata(&f.no, "B-3", "Ali", "2026-11-06").unwrap();
        d.imzala(&f.no, "Usta Ahmet", "usta", "2026-11-05").unwrap();
        assert!(d.imzala(&f.no, "Usta Ahmet", "usta", "2026-11-05").is_err());
        assert!(d.imzala(&f.no, "", "usta", "2026-11-05").is_err());
        assert!(d.imzala(&f.no, "X", "patron", "2026-11-05").is_err());
        let g = d.bul(&f.no).unwrap().unwrap();
        assert_eq!(g.raf, "B-3");
        assert_eq!(g.imzalar.len(), 1);
        let m = duz_metin(&g, 42);
        assert!(m.contains("Raf/Stant: B-3") && m.contains("İmza (usta): Usta A.")); // personel fişte "Ad S."
        let q = escpos_nushalar_qr(&g, 42, "http://x/telefon#fis=F");
        assert!(q.windows(22).any(|w| w == b"http://x/telefon#fis=F"));
        for st in [Durum::Terzihanede, Durum::Hazir] {
            d.durum_ata(&f.no, st, "2026-11-06").unwrap();
        }
        teslim_et_deneme(&d, &f.no, "2026-11-06");
        assert!(d.raf_ata(&f.no, "C-1", "Ali", "2026-11-07").is_err());
    }

    #[test]
    fn kargo_fiste_basilir() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let d = Defter { yol: &y };
        let mut g = girdi();
        g.kargo = true;
        g.kargo_notu = "Zile kapıda ödeme yok".into();
        assert!(d.ekle(g.clone()).unwrap_err().contains("ad ve soyad"));
        g.kargo_ad = "Ayşe Çelik".into();
        assert!(d.ekle(g.clone()).unwrap_err().contains("açık adres"));
        g.kargo_adres = "Atatürk Mah. 5. Sk. No:3 Çankaya/Ankara".into();
        assert!(d.ekle(g.clone()).unwrap_err().contains("telefon"));
        g.kargo_telefon = "0532 111 22 33".into();
        let f = d.ekle(g).unwrap();
        for e in [
            "TERZİHANE NÜSHASI 1",
            "TERZİHANE NÜSHASI 2",
            "MÜŞTERİ NÜSHASI",
        ] {
            let m = duz_metin_nusha(&f, 42, e);
            if e.starts_with("MÜŞTERİ") {
                // kısa müşteri nüshası: adres yok, kargo uyarısı var
                assert!(
                    m.contains("KARGO İLE GÖNDERİLECEK") && !m.contains("Çankaya"),
                    "{e}"
                );
            } else {
                assert!(
                    m.contains("KARGO YAPILACAK") && m.contains("Çankaya"),
                    "{e}"
                );
            }
        }
        let f2 = d.ekle(girdi()).unwrap();
        assert!(!duz_metin(&f2, 42).contains("KARGO"));
    }

    #[test]
    fn beyan_imzasi_kisi_ister_ve_usta_olamaz() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let d = Defter { yol: &y };
        let f = d.ekle(girdi()).unwrap();
        assert!(d
            .imzala_kisili(
                &f.no,
                "magaza",
                "teslim-eden",
                "2026-11-15",
                "10:00:00",
                "tel-1",
                "beyan",
                ""
            )
            .is_err());
        assert!(d
            .imzala_kisili(
                &f.no,
                "magaza",
                "usta",
                "2026-11-15",
                "10:00:00",
                "tel-1",
                "beyan",
                "k-7"
            )
            .is_err());
        assert!(d
            .imzala_kisili(
                &f.no,
                "magaza",
                "teslim-eden",
                "2026-11-15",
                "10:00:00",
                "tel-1",
                "x",
                "k-7"
            )
            .is_err());
        d.imzala_kisili(
            &f.no,
            "magaza",
            "teslim-eden",
            "2026-11-15",
            "10:00:00",
            "tel-1",
            "beyan",
            "k-7",
        )
        .unwrap();
        let im = &d.bul(&f.no).unwrap().unwrap().imzalar;
        assert_eq!(im[0].kimlik_duzeyi, "beyan");
        assert_eq!(im[0].okutan_kisi_id, "k-7");
    }

    #[test]
    fn sube_kodu_olmadan_fis_yazilamaz() {
        let y = yol();
        let d = Defter { yol: &y };
        for kotu in [
            "", "  ", "X", "Istanbul", "1ankara", "ankara-", "a b", "çankaya",
        ] {
            let mut g = girdi();
            g.sube = kotu.into();
            let e = d.ekle(g).unwrap_err();
            assert!(e.contains("şube kodu"), "{kotu:?}: {e}");
        }
        assert!(!y.exists(), "reddedilen fiş deftere yazılmamalı");
        assert!(sube_kodu_gecerli("MRK01") && sube_kodu_gecerli("VAN02"));
        // Basılı fişte şube kodu görünür.
        let f = d.ekle(girdi()).unwrap();
        assert!(duz_metin(&f, 42).contains("Şube: MRK01"));
    }

    #[test]
    fn sube_ve_imza_ayrintisi_eski_satirla_uyumlu() {
        let y = yol();
        let d = Defter { yol: &y };
        let mut g = girdi();
        g.sube = " MRK01 ".into();
        let f = d.ekle(g).unwrap();
        assert_eq!(f.sube, "MRK01");
        d.imzala_ayrintili(&f.no, "tg:1", "usta", "2026-11-15", "17:30:05", "cihaz-a")
            .unwrap();
        let k = d.bul(&f.no).unwrap().unwrap();
        assert_eq!(k.imzalar[0].saat, "17:30:05");
        assert_eq!(k.imzalar[0].cihaz, "cihaz-a");
        // Eski biçim (sube, saat, cihaz yok) hâlâ okunur.
        let eski = r#"{"tur":"imza","v":{"no":"F-1","imza":{"kim":"a","rol":"usta","tarih":"2026-11-01"}}}"#;
        let k: defter::Kayit = serde_json::from_str(eski).unwrap();
        assert!(matches!(k, defter::Kayit::Imza { .. }));
    }

    #[test]
    fn zincir_kismi_degisikligi_yakalar() {
        let y = yol();
        let d = Defter { yol: &y };
        d.ekle(girdi()).unwrap(); // eski satır: zincire girmez
        assert!(d.zincir_ucu().unwrap().is_none());
        d.zincir_baslat("2026-11-15").unwrap();
        assert!(d.zincir_baslat("2026-11-15").is_err());
        let f = d.ekle(girdi()).unwrap();
        let uc1 = d.zincir_ucu().unwrap().unwrap();
        assert_eq!(uc1.sayi, 2);
        assert!(d.zincir_dogrula(&uc1).unwrap());
        d.imzala(&f.no, "tg:1", "usta", "2026-11-15").unwrap();
        let uc2 = d.zincir_ucu().unwrap().unwrap();
        assert_ne!(uc1.ozet, uc2.ozet);
        assert!(
            d.zincir_dogrula(&uc1).unwrap(),
            "eski uç yeni zincirde durur"
        );
        // Zincirdeki bir satır değiştirilirse eski uç doğrulanamaz.
        let metin = std::fs::read_to_string(&y).unwrap();
        std::fs::write(&y, metin.replacen("tg:1", "tg:9", 1)).unwrap();
        assert!(!d.zincir_dogrula(&uc2).unwrap());
        // Kısalmış defter de doğrulanmaz.
        assert!(!d
            .zincir_dogrula(&ZincirUcu {
                sayi: 99,
                ozet: "x".into()
            })
            .unwrap());
    }

    #[test]
    fn telefon() {
        assert_eq!(telefon_duzelt("+90 532 123 45 67").unwrap(), "05321234567");
        assert_eq!(telefon_duzelt("0532 123 4567").unwrap(), "05321234567");
        assert_eq!(telefon_duzelt("5321234567").unwrap(), "05321234567");
        assert!(telefon_duzelt("12345").is_err());
        assert!(telefon_duzelt("02121234567").is_err());
    }

    #[test]
    fn dogrulama() {
        let mut g = girdi();
        assert!(dogrula(&g, &kat(), true, true).is_ok());
        g.on_odeme_kurus = 90000;
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.teslim = "2026-11-01".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.prova = "2026-11-12".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.kalemler.clear();
        assert!(dogrula(&g, &kat(), true, true).is_err());
        let mut g = girdi();
        g.musteri = "  ".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
    }

    #[test]
    fn defter_dongusu() {
        let y = yol();
        let d = Defter { yol: &y };
        let f1 = d.ekle(girdi()).unwrap();
        let f2 = d.ekle(girdi()).unwrap();
        assert_eq!(
            (f1.no.as_str(), f2.no.as_str()),
            ("F-20261104-001", "F-20261104-002")
        );
        assert_eq!(f1.musteri, "Ayşe Çelik");
        assert_eq!(f1.kalan_kurus(), 65000);
        assert!(d.durum_ata(&f1.no, Durum::Teslim, "2026-11-05").is_err()); // Hazir'dan geçmeli
        d.durum_ata(&f1.no, Durum::Hazir, "2026-11-08").unwrap();
        // teslim yalnız teslim kaydıyla (durum_ata Teslim'e geçirmez)
        assert!(d.durum_ata(&f1.no, Durum::Teslim, "2026-11-09").is_err());
        teslim_et_deneme(&d, &f1.no, "2026-11-09");
        assert!(d.durum_ata(&f1.no, Durum::Iptal, "2026-11-09").is_err());
        assert_eq!(d.bul(&f1.no).unwrap().unwrap().durum, Durum::Teslim);
        assert_eq!(d.geciken("2026-11-11").unwrap().len(), 1);
        assert_eq!(d.geciken("2026-11-10").unwrap().len(), 0);
        assert_eq!(taa_dosya::satirlari_oku(&y).unwrap().len(), 4); // hiçbir şey silinmedi
    }

    #[test]
    fn metin_ve_termal() {
        let f = Fis {
            sube: String::new(),
            no: "F-20261104-001".into(),
            tarih: "2026-11-04".into(),
            durum: Durum::Bekliyor,
            raf: String::new(),
            imzalar: Vec::new(),
            musteri: "Ayşe Çelik".into(),
            telefon: "05321234567".into(),
            kalemler: girdi().kalemler,
            olcu_notu: "".into(),
            prova: "".into(),
            teslim: "2026-11-10".into(),
            teslim_saati: "17:00".into(),
            ucret_kurus: 85000,
            on_odeme_kurus: 20000,
            usta: "".into(),
            satis_danismani: "Mert".into(),
            olcu_alan: "Kadir Y.".into(),
            satis_danismani_id: String::new(),
            olcu_alan_id: String::new(),
            duzenleyen_id: String::new(),
            kargo: false,
            kargo_notu: String::new(),
            kargo_ad: String::new(),
            kargo_adres: String::new(),
            kargo_telefon: String::new(),
            bildirim_izni: false,
            teslim_degisiklikleri: Vec::new(),
            duzeltmeler: Vec::new(),
            bagli: Vec::new(),
            teslim_kaydi: None,
        };
        let m = duz_metin(&f, 42);
        assert!(m.contains("TADİLAT FİŞİ") && m.contains("F-20261104-001"));
        assert!(m.contains("ÜRÜN TESLİM TARİHİ") && m.contains("10.11.2026 17:00"));
        assert!(m.lines().all(|l| l.chars().count() <= 42));
        // ödeme alınmadan işlem yapılmaz: fiyat basılmaz
        assert!(!m.contains("KALAN") && !m.contains(" TL"));
        let b = escpos(&f, 42);
        assert_eq!(&b[..5], &[0x1B, 0x40, 0x1B, 0x74, 13]);
        assert!(b.contains(&0x98)); // İ -> CP857
        assert!(b.ends_with(&[0x1D, 0x56, 0x42, 0]));
    }

    #[test]
    fn para_ve_sarma() {
        assert_eq!(para(123456), "1.234,56");
        assert_eq!(para(5), "0,05");
        assert!(sar(&"a".repeat(100), 42).iter().all(|l| l.len() <= 42));
    }

    fn katalogsuz_girdi() -> Girdi {
        let mut g = girdi();
        g.kalemler = vec![Kalem {
            islem_kodu: Some("pant-bel-daralt".into()),
            olcu_mm: Some(30),
            ..kalem("Pantolon", "Bel daraltma", "")
        }];
        g
    }

    #[test]
    fn katalog_kalemi_ve_sinir() {
        let mut g = katalogsuz_girdi();
        assert!(dogrula(&g, &kat(), false, true).is_ok());
        g.kalemler[0].olcu_mm = Some(60);
        assert!(dogrula(&g, &kat(), false, true)
            .unwrap_err()
            .contains("Usta onayı"));
        g.kalemler[0].usta_onayli = true;
        assert_eq!(dogrula(&g, &kat(), false, true).unwrap().1.len(), 1);
        g.kalemler[0].olcu_mm = None; // unutulan ölçü
        assert!(dogrula(&g, &kat(), false, true)
            .unwrap_err()
            .contains("ölçü"));
    }

    #[test]
    fn serbest_islem_kapatilabilir_ve_zorunlu_isimler() {
        let g = girdi(); // serbest metin kalemler
        assert!(dogrula(&g, &kat(), true, true).is_ok());
        assert!(dogrula(&g, &kat(), false, true)
            .unwrap_err()
            .contains("listeden"));
        let mut g = girdi();
        g.olcu_alan = " ".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
        // Kişi kaydı kimliği adın yerini tutar (yeni fişlerde ad boş, kimlik dolu).
        g.olcu_alan_id = "K-0000abcd".into();
        g.satis_danismani = String::new();
        g.satis_danismani_id = "K-0000abce".into();
        assert!(dogrula(&g, &kat(), true, true).is_ok());
        let mut g = girdi();
        g.teslim_saati = "25:00".into();
        assert!(dogrula(&g, &kat(), true, true).is_err());
    }

    #[test]
    fn teslim_degisikligi_izlenir() {
        let y = yol();
        let d = Defter { yol: &y };
        let f = d.ekle(girdi()).unwrap();
        assert!(d
            .teslim_degistir(&f.no, "2026-11-12", "17:00", " ", "2026-11-05")
            .is_err());
        assert!(d
            .teslim_degistir(&f.no, "2026-11-01", "17:00", "x", "2026-11-05")
            .is_err());
        d.teslim_degistir(&f.no, "2026-11-12", "18:30", "müşteri istedi", "2026-11-05")
            .unwrap();
        let g = d.bul(&f.no).unwrap().unwrap();
        assert_eq!(
            (g.teslim.as_str(), g.teslim_saati.as_str()),
            ("2026-11-12", "18:30")
        );
        assert_eq!(g.teslim_degisiklikleri[0].eski, "2026-11-10");
        assert!(duz_metin(&g, 42).contains("1 kez değişti"));
        d.durum_ata(&f.no, Durum::Terzihanede, "2026-11-05")
            .unwrap();
        d.durum_ata(&f.no, Durum::Hazir, "2026-11-06").unwrap();
        teslim_et_deneme(&d, &f.no, "2026-11-12");
        assert!(d
            .teslim_degistir(&f.no, "2026-11-20", "17:00", "x", "2026-11-13")
            .is_err());
    }

    #[test]
    fn eski_defter_satiri_okunur() {
        // yeni alanlardan önce yazılmış bir satır
        let y = yol();
        let eski = r#"{"tur":"fis","v":{"no":"F-20260101-001","tarih":"2026-01-01","musteri":"A","telefon":"05321234567","kalemler":[{"urun":"Ceket","islem":"kol"}],"teslim":"2026-01-05","ucret_kurus":0,"on_odeme_kurus":0,"durum":"bekliyor"}}"#;
        taa_dosya::satir_ekle(&y, eski).unwrap();
        let f = Defter { yol: &y }.bul("F-20260101-001").unwrap().unwrap();
        assert_eq!(f.teslim_saati, "17:00");
        assert!(f.kalemler[0].olcu_mm.is_none());
    }

    #[test]
    fn uc_nusha_ve_cm() {
        assert_eq!(cm_metni(25), "2,5");
        assert_eq!(cm_metni(40), "4");
        let y = yol();
        let d = Defter { yol: &y };
        let f = d.ekle(katalogsuz_girdi()).unwrap();
        let m = duz_metin(&f, 42);
        assert!(
            m.contains("Bel daraltma 3 cm")
                && m.contains("Sat. Dan.: Mert")
                && m.contains("Ölçü Alan: Kadir Y.")
        );
        let b = escpos_nushalar(&f, 42);
        assert_eq!(
            b.windows(4).filter(|w| *w == [0x1D, 0x56, 0x42, 0]).count(),
            2
        );
        assert!(String::from_utf8_lossy(&b).contains("mesul"));
    }

    #[test]
    fn kagit_tasarrufu_duzeni() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let f = Defter { yol: &y }.ekle(girdi()).unwrap();
        assert_eq!(nusha_etiketleri(2).len(), 2);
        assert_eq!(nusha_etiketleri(3).len(), 3);
        let t = duz_metin_nusha(&f, 42, "TERZİHANE NÜSHASI");
        let m = duz_metin_nusha(&f, 42, "MÜŞTERİ NÜSHASI");
        assert!(t.contains("Teslim Eden") && !m.contains("Teslim Eden"));
        assert!(t.contains("Fiş Tarihi: 04.11.2026"));
        assert!(t.contains("Sat. Dan.: Mert | Ölçü Alan: Kadir Y."));
        assert!(t.lines().all(|l| l.chars().count() <= 42));
        // alt bilgi cümleleri yalnız müşteri nüshasında basılır
        let b = escpos_nushalar(&f, 42);
        let yer = |hedef: &str| {
            let h: Vec<u8> = hedef.chars().map(cp857).collect();
            b.windows(h.len()).filter(|w| *w == h.as_slice()).count()
        };
        assert_eq!(yer("mesul değiliz"), 1);
    }

    #[test]
    fn logo_gun_adi_ve_serit() {
        assert_eq!(gun_adi("2026-11-10"), Some("Salı"));
        assert_eq!(gun_adi("2026-11-05"), Some("Perşembe"));
        assert_eq!(gun_adi("bozuk"), None);
        // logo bitmap'i: başlık + 72 bayt x yükseklik, 80 mm genişliğe sığar
        let h = LOGO[0] as usize | (LOGO[1] as usize) << 8;
        assert!(h > 20 && h < 200 && LOGO.len() == 2 + 72 * h);
        let l = logo_baytlari();
        assert_eq!(&l[..6], &[0x1D, 0x76, 0x30, 0, 72, 0]);
        assert!(LOGO[2..].iter().any(|b| *b != 0)); // siyah nokta var
        let dir = taa_dosya::gecici_klasor().unwrap();
        let f = Defter {
            yol: &dir.join("d.jsonl"),
        }
        .ekle(girdi())
        .unwrap();
        let b = escpos_nusha_tam(&f, 42, "TERZİHANE NÜSHASI", "q", None);
        assert!(b.windows(l.len()).any(|w| w == l.as_slice()));
        // kısa müşteri nüshasında logo bitmap'i yok (cüzdan boyu)
        let mb = escpos_nusha_tam(&f, 42, "MÜŞTERİ NÜSHASI", "q", None);
        assert!(!mb.windows(l.len()).any(|w| w == l.as_slice()));
        let m = duz_metin_nusha(&f, 42, "");
        assert!(
            m.contains("TERZİ ADEM ALTUN")
                && m.contains("Salı")
                && m.contains("= ÜRÜN TESLİM TARİHİ =")
        );
    }

    #[test]
    fn sonradan_duzeltme_baglama_ve_kisa_musteri_nushasi() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let d = Defter { yol: &y };
        let mut g = girdi();
        g.kalemler[0].renk = Some("siyah".into());
        g.kalemler[0].adet = Some(2);
        g.satis_danismani = "Mert Can Yılmaz".into();
        let a = d.ekle(g.clone()).unwrap();
        let b = d.ekle(girdi()).unwrap();
        let c = d.ekle(girdi()).unwrap();
        // renk listede olmalı
        let mut kotu = kalem("Ceket", "ütü", "");
        kotu.renk = Some("gokkusagi".into());
        assert!(d
            .kalem_ekle(&a.no, kotu, "Mağaza PC", "2026-11-05")
            .is_err());
        // sonradan ütü eklenir, bir kalem çıkarılır
        d.kalem_ekle(&a.no, kalem("Ceket", "ütü", ""), "Mağaza PC", "2026-11-05")
            .unwrap();
        d.kalem_cikar(&a.no, 1, "müşteri vazgeçti", "Mağaza PC", "2026-11-05")
            .unwrap();
        assert!(d
            .kalem_cikar(&a.no, 9, "", "Mağaza PC", "2026-11-05")
            .is_err());
        // mağazadan teslim -> kargo (eksik bilgi reddedilir) -> kargo iptal
        assert!(d
            .teslim_sekli(&a.no, true, "Ayşe", "", "", "", "", "PC", "2026-11-05")
            .is_err());
        d.teslim_sekli(
            &a.no,
            true,
            "Ayşe Çelik",
            "Atatürk Mah. 5. Sk. No:3 Büyükçekmece/İstanbul",
            "0532 111 22 33",
            "",
            "müşteri istedi",
            "PC",
            "2026-11-05",
        )
        .unwrap();
        assert!(d.bul(&a.no).unwrap().unwrap().kargo);
        d.teslim_sekli(
            &a.no,
            false,
            "",
            "",
            "",
            "",
            "kargo iptal",
            "PC",
            "2026-11-06",
        )
        .unwrap();
        let f = d.bul(&a.no).unwrap().unwrap();
        assert!(!f.kargo && f.kargo_adres.is_empty());
        assert_eq!(f.kalemler.len(), 2);
        assert_eq!(f.kalemler[1].islem, "ütü");
        assert_eq!(f.duzeltmeler.len(), 4);
        // bağlama: a-b, sonra b-c => üçü bir grup
        assert!(d.bagla(&a.no, &a.no, "PC", "2026-11-06").is_err());
        d.bagla(&a.no, &b.no, "PC", "2026-11-06").unwrap();
        assert!(d.bagla(&b.no, &a.no, "PC", "2026-11-06").is_err());
        d.bagla(&b.no, &c.no, "PC", "2026-11-06").unwrap();
        let fa = d.bul(&a.no).unwrap().unwrap();
        assert_eq!(fa.bagli.len(), 2);
        assert!(d.bul(&c.no).unwrap().unwrap().bagli.contains(&a.no));
        d.bag_coz(&c.no, "PC", "2026-11-06").unwrap();
        assert_eq!(d.bul(&a.no).unwrap().unwrap().bagli, vec![b.no.clone()]);
        assert!(d.bul(&c.no).unwrap().unwrap().bagli.is_empty());
        // baskı: mağaza nüshası ayrıntılı, müşteri nüshası kısa
        let f = d.bul(&a.no).unwrap().unwrap();
        let t = duz_metin_nusha(&f, 42, "TERZİHANE NÜSHASI");
        assert!(t.contains("2 x Pantolon (Siyah) - paça kısalt") && t.contains("Sonradan yapılan"));
        assert!(t.contains("Mert Y.") && !t.contains("Yılmaz"));
        assert!(t.contains("BİRLİKTE TESLİM") && t.contains(&b.no));
        let m = duz_metin_nusha(&f, 42, "MÜŞTERİ NÜSHASI");
        assert!(m.contains("2 x Pantolon (Siyah)") && m.contains("Birlikte teslim"));
        assert!(!m.contains("Sat. Dan.") && !m.contains("Ölçü:") && !m.contains("Turgut Özal"));
        assert!(
            m.lines().count() < t.lines().count() / 2 + 4,
            "müşteri nüshası kısa:\n{m}"
        );
        // teslim edilen fiş düzeltilemez
        d.durum_ata(&a.no, Durum::Hazir, "2026-11-07").unwrap();
        teslim_et_deneme(&d, &a.no, "2026-11-07");
        assert!(d
            .kalem_ekle(&a.no, kalem("Ceket", "ütü", ""), "PC", "2026-11-07")
            .is_err());
        assert_eq!(kisa_ad("ali"), "ali");
        assert_eq!(kisa_ad("Muhammed Yusuf Beşir"), "Muhammed B.");
    }

    fn teslim_kaydi_ornek(fisli: bool) -> TeslimKaydi {
        TeslimKaydi {
            teslim_alan: " Zeynep   Kaya ".into(),
            iliski: "yakini".into(),
            fisli,
            dogrulama: if fisli { "fis" } else { "telefon-son4" }.into(),
            imza_sha256: "ab".repeat(32),
            imza_dosyasi: "imzalar/F-20261104-001-1.png.enc".into(),
            imza_yontemi: "cihaz_imza".into(),
            kim: "PC".into(),
            tarih: "2026-11-09".into(),
            saat: "10:00:00".into(),
            ana_fis: String::new(),
            sifreli: None,
            teslim_id: String::new(),
            gorunen: None,
        }
    }

    /// Sahte şifreli blok (biçimce geçerli; içerik anlamsız).
    fn sahte_sifreli() -> TeslimSifreli {
        TeslimSifreli {
            surum: 1,
            iv: "00".repeat(12),
            c: "QUJDREVGR0hJSktMTU5PUFFSU1RVVldY".into(),
            ad_indeks: "0123456789abcdef".into(),
            tel_indeks: "fedcba9876543210".into(),
        }
    }

    /// Toplu teslim (teslim sepeti): ya hepsi ya hiçbiri; ortak teslim_id; aynı fiş iki kez olmaz.
    #[test]
    fn toplu_teslim_hepsi_ya_da_hicbiri() {
        let y = yol();
        let d = Defter { yol: &y };
        let a = d.ekle(girdi()).unwrap();
        let b = d.ekle(girdi()).unwrap();
        let c = d.ekle(girdi()).unwrap();
        d.durum_ata(&a.no, Durum::Hazir, "2026-11-26").unwrap();
        d.durum_ata(&b.no, Durum::Hazir, "2026-11-26").unwrap();
        // c hazır değil
        let tid = "T-20261126-ABCD23";
        let k = |ana: &str| TeslimKaydi {
            teslim_id: tid.into(),
            ana_fis: ana.into(),
            ..teslim_kaydi_ornek(true)
        };
        assert!(teslim_id_gecerli(tid));
        assert!(!teslim_id_gecerli("T-2026-AB"));
        assert!(!teslim_id_gecerli("T-20261126-ab12"));
        let once = std::fs::read(&y).unwrap();
        // hazır olmayan bir fiş: hiçbiri yazılmaz
        let e = d
            .teslim_et_toplu(vec![
                (a.no.clone(), k("")),
                (b.no.clone(), k(&a.no)),
                (c.no.clone(), k(&a.no)),
            ])
            .unwrap_err();
        assert!(e.contains(&c.no), "{e}");
        assert_eq!(std::fs::read(&y).unwrap(), once);
        // aynı fiş iki kez; farklı teslim_id; boş teslim_id: reddedilir
        assert!(d
            .teslim_et_toplu(vec![(a.no.clone(), k("")), (a.no.clone(), k(""))])
            .is_err());
        let mut farkli = k(&a.no);
        farkli.teslim_id = "T-20261126-ZZZZ99".into();
        assert!(d
            .teslim_et_toplu(vec![(a.no.clone(), k("")), (b.no.clone(), farkli)])
            .is_err());
        let mut bos = k("");
        bos.teslim_id.clear();
        assert!(d.teslim_et_toplu(vec![(a.no.clone(), bos)]).is_err());
        // doğrulama geçti ama yazım hatası: hiçbir satır yazılmaz
        TOPLU_YAZIM_HATASI.with(|h| h.set(true));
        assert!(d
            .teslim_et_toplu(vec![(a.no.clone(), k("")), (b.no.clone(), k(&a.no))])
            .is_err());
        TOPLU_YAZIM_HATASI.with(|h| h.set(false));
        assert_eq!(std::fs::read(&y).unwrap(), once);
        // geçerli: ikisi de teslim, aynı teslim_id; ana fiş ilkinde boş
        d.teslim_et_toplu(vec![(a.no.clone(), k("")), (b.no.clone(), k(&a.no))])
            .unwrap();
        let fa = d.bul(&a.no).unwrap().unwrap();
        let fb = d.bul(&b.no).unwrap().unwrap();
        assert_eq!(fa.durum, Durum::Teslim);
        assert_eq!(fb.durum, Durum::Teslim);
        let (ta, tb) = (fa.teslim_kaydi.unwrap(), fb.teslim_kaydi.unwrap());
        assert_eq!(ta.teslim_id, tid);
        assert_eq!(tb.teslim_id, tid);
        assert_eq!(ta.ana_fis, "");
        assert_eq!(tb.ana_fis, a.no);
        // ikinci kez: zaten teslim
        assert!(d.teslim_et_toplu(vec![(a.no.clone(), k(""))]).is_err());
    }

    #[test]
    fn sifreli_ve_qr_teslim_kaydi() {
        let y = yol();
        let d = Defter { yol: &y };
        let f = d.ekle(girdi()).unwrap();
        d.durum_ata(&f.no, Durum::Hazir, "2026-11-26").unwrap();
        let qr = |fisli: bool| TeslimKaydi {
            teslim_alan: String::new(),
            iliski: "kendisi".into(),
            fisli,
            dogrulama: if fisli { "qr" } else { "qr+telefon-son4" }.into(),
            imza_sha256: String::new(),
            imza_dosyasi: String::new(),
            imza_yontemi: String::new(),
            kim: "PC".into(),
            tarih: "2026-11-26".into(),
            saat: "14:32:09".into(),
            ana_fis: String::new(),
            sifreli: Some(sahte_sifreli()),
            teslim_id: String::new(),
            gorunen: None,
        };
        // qr şifresiz olamaz; şifreli kayıtta düz metin ad olamaz; bozuk şifreli blok reddedilir
        let mut k = qr(false);
        k.sifreli = None;
        assert!(d.teslim_et(&f.no, k).is_err());
        let mut k = qr(false);
        k.teslim_alan = "Zeynep Kaya".into();
        assert!(d.teslim_et(&f.no, k).is_err());
        let mut k = qr(false);
        k.sifreli.as_mut().unwrap().iv = "zz".into();
        assert!(d.teslim_et(&f.no, k).is_err());
        let mut k = qr(true);
        k.dogrulama = "qr+telefon-son4".into();
        assert!(d.teslim_et(&f.no, k).is_err());
        // şifreli ekran imzası yolu: fişsizde imza yine zorunlu
        let mut k = teslim_kaydi_ornek(false);
        k.teslim_alan = String::new();
        k.sifreli = Some(sahte_sifreli());
        k.imza_sha256 = String::new();
        k.imza_dosyasi = String::new();
        assert!(d.teslim_et(&f.no, k).is_err());
        // fişsiz QR: imza artık ZORUNLU (karekod sayfasında qr_imza ya da ekranda cihaz_imza)
        assert!(d
            .teslim_et(&f.no, qr(false))
            .unwrap_err()
            .contains("imzası zorunlu"));
        let imzali_qr = || {
            let mut k = qr(false);
            k.imza_sha256 = "cd".repeat(32);
            k.imza_dosyasi = format!("imzalar/{}-1.png.enc", f.no);
            k.imza_yontemi = "qr_imza".into();
            k
        };
        // imza yöntemi bilinmiyor / boş: reddedilir
        let mut k = imzali_qr();
        k.imza_yontemi = "parmak".into();
        assert!(d.teslim_et(&f.no, k).is_err());
        let mut k = imzali_qr();
        k.imza_yontemi = String::new();
        assert!(d.teslim_et(&f.no, k).is_err());
        // qr_imza karekodsuz teslimde olmaz
        let mut k = teslim_kaydi_ornek(false);
        k.imza_yontemi = "qr_imza".into();
        assert!(d.teslim_et(&f.no, k).is_err());
        // imzasız kayıtta yöntem yazılamaz
        let mut k = qr(true);
        k.imza_yontemi = "cihaz_imza".into();
        assert!(d.teslim_et(&f.no, k).is_err());
        d.teslim_et(&f.no, imzali_qr()).unwrap();
        let mut g = d.bul(&f.no).unwrap().unwrap();
        assert_eq!(g.durum, Durum::Teslim);
        let t = g.teslim_kaydi.clone().unwrap();
        assert!(t.qr_onayli() && t.teslim_alan.is_empty() && t.gorunen.is_none());
        // çözülmemiş: "(şifreli)"; çözülmüş görünüm: "Ad S. - QR onaylı, tarih saat"
        let m = duz_metin(&g, 42).replace('\n', " ");
        assert!(
            m.contains("Teslim alan: (şifreli) - QR onaylı, imzalı, 26.11.2026 14:32"),
            "{m}"
        );
        g.teslim_kaydi.as_mut().unwrap().gorunen = Some(TeslimGorunen {
            ad: "Zeynep Ayşe Kaya".into(),
            yontem: "qr".into(),
        });
        let m = duz_metin(&g, 42).replace('\n', " ");
        assert!(
            m.contains("Teslim alan: Z*** A*** K*** - QR onaylı, imzalı, 26.11.2026 14:32"),
            "{m}"
        );
        assert!(!m.contains("Kaya") && !m.contains("Zeynep"));
        // görünüm diske/JSON'a yazılmaz
        let j = serde_json::to_string(&g).unwrap();
        assert!(!j.contains("Zeynep") && !j.contains("gorunen"), "{j}");
        let defter = std::fs::read_to_string(&y).unwrap();
        assert!(!defter.contains("Zeynep"));
        // eski kayıt (sifreli alanı yok) okunur
        let eski: TeslimKaydi = serde_json::from_str(
            r#"{"teslim_alan":"Eski Kişi","iliski":"kendisi","fisli":true,"dogrulama":"fis","kim":"PC","tarih":"2026-11-01"}"#,
        )
        .unwrap();
        assert!(eski.sifreli.is_none() && !eski.qr_onayli());
    }

    /// Deneme teslimi: fişli, imzasız, sahte ad.
    fn teslim_et_deneme(d: &Defter, no: &str, tarih: &str) {
        let mut k = teslim_kaydi_ornek(true);
        k.imza_sha256 = String::new();
        k.imza_dosyasi = String::new();
        k.imza_yontemi = String::new();
        k.tarih = tarih.into();
        d.teslim_et(no, k).unwrap();
    }

    #[test]
    fn teslim_kaydi_fissiz_ve_baski() {
        let y = yol();
        let d = Defter { yol: &y };
        let f = d.ekle(girdi()).unwrap();
        // hazır olmayan fişe teslim kaydı yazılmaz
        assert!(d.teslim_et(&f.no, teslim_kaydi_ornek(false)).is_err());
        d.durum_ata(&f.no, Durum::Hazir, "2026-11-08").unwrap();
        // kurallar: ad soyad, ilişki, doğrulama yöntemi, fişsizde imza
        let mut k = teslim_kaydi_ornek(false);
        k.teslim_alan = "Zeynep".into();
        assert!(d.teslim_et(&f.no, k).is_err());
        let mut k = teslim_kaydi_ornek(false);
        k.iliski = "komsu".into();
        assert!(d.teslim_et(&f.no, k).is_err());
        let mut k = teslim_kaydi_ornek(false);
        k.dogrulama = "fis".into();
        assert!(d.teslim_et(&f.no, k).is_err());
        let mut k = teslim_kaydi_ornek(false);
        k.imza_sha256 = String::new();
        k.imza_dosyasi = String::new();
        assert!(d.teslim_et(&f.no, k).is_err());
        let mut k = teslim_kaydi_ornek(false);
        k.imza_sha256 = "xyz".into();
        assert!(d.teslim_et(&f.no, k).is_err());
        // ad: çift yönlü yazı işareti, sıfır genişlikli karakter, HTML ve rakam reddedilir
        for ad in [
            "Zeynep \u{202E}Kaya",
            "Zey\u{200B}nep Kaya",
            "Zeynep <b>Kaya</b>",
            "Zeynep Kaya2",
        ] {
            let mut k = teslim_kaydi_ornek(false);
            k.teslim_alan = ad.into();
            assert!(d.teslim_et(&f.no, k).is_err(), "{ad}");
        }
        assert!(teslim_alan_denetle("Ayşe O'Neil-Çelik Jr.", "kendisi").is_ok());
        // Hintçe (virama U+094D), Farsça (ZWNJ U+200C) ve harekeli Arapça sahte adlar geçer (bulut kuralıyla aynı);
        // yön işaretleri ve U+200B yine reddedilir.
        for ad in [
            "राहुल शर्मा",
            "علی\u{200C}رضا محمدی",
            "مُحَمَّد عَلِي",
            "Ayşe Deneme",
            "Zoë Ünlü-Çelik",
        ] {
            assert!(ad_karakterleri_gecerli(ad), "{ad}");
        }
        for ad in [
            "Zey\u{200E}nep",
            "Zey\u{2066}nep",
            "Zey\u{FEFF}nep",
            "Zey\u{200B}nep",
            "Zey\u{00AD}nep",
        ] {
            assert!(!ad_karakterleri_gecerli(ad), "{ad:?}");
        }
        // bağlı fiş kaydında ana fiş, fişin kendisi olamaz
        let mut k = teslim_kaydi_ornek(false);
        k.ana_fis = f.no.clone();
        assert!(d.teslim_et(&f.no, k).is_err());
        d.teslim_et(&f.no, teslim_kaydi_ornek(false)).unwrap();
        assert!(d.teslim_et(&f.no, teslim_kaydi_ornek(false)).is_err()); // bir kez
        let g = d.bul(&f.no).unwrap().unwrap();
        // kayıt satırı aynı zamanda Teslim geçişi (tek satır, atomik)
        assert_eq!(g.durum, Durum::Teslim);
        assert!(d.durum_ata(&f.no, Durum::Iptal, "2026-11-09").is_err());
        let gecmis = d.durum_gecmisi().unwrap();
        assert_eq!(
            gecmis
                .iter()
                .filter(|(n, st, _)| *n == f.no && *st == Durum::Teslim)
                .count(),
            1
        );
        let t = g.teslim_kaydi.clone().unwrap();
        assert_eq!(t.teslim_alan, "Zeynep Kaya");
        assert!(!t.fisli && t.dogrulama == "telefon-son4");
        // 42 sütunda satır sarılabilir; satırlar birleştirilip aranır.
        let m = duz_metin(&g, 42).replace('\n', " ");
        assert!(
            m.contains("Teslim alan: Z*** K*** (yakını) - fişsiz, imzalı"),
            "{m}"
        );
        assert!(!m.contains("Kaya") && !m.contains("Zeynep"));
        // müşteri nüshasında basılmaz
        assert!(!duz_metin_nusha(&g, 42, "MÜŞTERİ NÜSHASI").contains("Teslim alan:"));
        // eski satırlar (teslim kaydı yok) okunur, alan yazılmaz
        let mut eski = g.clone();
        eski.teslim_kaydi = None;
        assert!(!serde_json::to_string(&eski)
            .unwrap()
            .contains("teslim_kaydi"));
    }

    #[test]
    fn teslim_kaydi_fisli_imzasiz() {
        let y = yol();
        let d = Defter { yol: &y };
        let f = d.ekle(girdi()).unwrap();
        d.durum_ata(&f.no, Durum::Hazir, "2026-11-08").unwrap();
        let mut k = teslim_kaydi_ornek(true);
        k.dogrulama = "telefon-son4".into();
        assert!(d.teslim_et(&f.no, k).is_err());
        let mut k = teslim_kaydi_ornek(true);
        k.iliski = "kendisi".into();
        k.teslim_alan = "Ayşe Çelik".into();
        k.imza_sha256 = String::new();
        k.imza_dosyasi = String::new();
        k.imza_yontemi = String::new();
        d.teslim_et(&f.no, k).unwrap();
        let g = d.bul(&f.no).unwrap().unwrap();
        assert!(duz_metin(&g, 42).contains("Teslim alan: A*** Ç*** (kendisi) - fişli\n"));
    }

    #[test]
    fn ad_maskeleme() {
        assert_eq!(ad_maskele("Mehmet Yılmaz"), "M*** Y***");
        assert_eq!(ad_maskele("  ışık   ince "), "I*** İ***");
        assert_eq!(ad_maskele("علی\u{200C}رضا محمدی"), "ع*** م***");
        assert_eq!(ad_maskele(""), "");
    }

    /// Bir kezlik geçiş: yalnız dönüştürülen teslim satırı değişir, diğerleri bayt bayt aynı kalır;
    /// yedek çağrısı başarısızsa defter değişmez; ikinci çalıştırma hiçbir şey yazmaz.
    #[test]
    fn teslim_kayitlari_yeniden_yazilir() {
        let y = yol();
        let d = Defter { yol: &y };
        let f = d.ekle(girdi()).unwrap();
        let f2 = d.ekle(girdi()).unwrap();
        d.durum_ata(&f.no, Durum::Hazir, "2026-11-08").unwrap();
        d.teslim_et(&f.no, teslim_kaydi_ornek(true)).unwrap();
        let once = std::fs::read(&y).unwrap();
        let donustur = |_: &str, k: &TeslimKaydi| -> Result<Option<TeslimKaydi>, String> {
            if k.teslim_alan.is_empty() {
                return Ok(None);
            }
            Ok(Some(TeslimKaydi {
                teslim_alan: String::new(),
                sifreli: Some(sahte_sifreli()),
                ..k.clone()
            }))
        };
        // yedek alınamazsa dokunulmaz
        let e = d
            .teslim_kayitlarini_yeniden_yaz(donustur, |_| Err("yedek yok".into()))
            .unwrap_err();
        assert_eq!(e, "yedek yok");
        assert_eq!(std::fs::read(&y).unwrap(), once);
        let mut yedek = Vec::new();
        let n = d
            .teslim_kayitlarini_yeniden_yaz(donustur, |b| {
                yedek = b.to_vec();
                Ok(())
            })
            .unwrap();
        assert_eq!(n, 1);
        assert_eq!(yedek, once);
        let sonra = std::fs::read_to_string(&y).unwrap();
        assert!(!sonra.contains("Zeynep"), "{sonra}");
        let once_m = String::from_utf8(once.clone()).unwrap();
        let (a, b): (Vec<&str>, Vec<&str>) = (once_m.lines().collect(), sonra.lines().collect());
        assert_eq!(a.len(), b.len());
        assert_eq!(a.iter().zip(&b).filter(|(x, y)| x != y).count(), 1);
        let t = d.bul(&f.no).unwrap().unwrap().teslim_kaydi.unwrap();
        assert!(t.teslim_alan.is_empty() && t.sifreli.is_some());
        assert_eq!(d.bul(&f2.no).unwrap().unwrap().durum, Durum::Bekliyor);
        // ikinci çalıştırma: değişiklik yok, yedek istenmez
        let n = d
            .teslim_kayitlarini_yeniden_yaz(donustur, |_| panic!("yedek istenmemeli"))
            .unwrap();
        assert_eq!(n, 0);
    }
}
