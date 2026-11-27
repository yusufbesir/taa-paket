// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26] değiştiren: [Claude#taa-kasa-tk-ek3 2026-11-27]
//! Telefonla gün sonu imzası: Telefon Köprüsü onay akışının taa-kasa istemcisi.
//!
//! Sözleşme: TK deposu `taa-kasa-onay` 5c9b06d, "Ek 3" (koordinatörün aktardığı alan listesine göre; belge bu
//! depodan okunmadı). Kapsam `taa_kasa_gun_sonu`.
//! 1. Kasa gün sonu kaydından **özet metni** (biçim sürümü 2, en çok 1 KiB) üretir; `ozet` = SHA-256, 64 küçük hex.
//! 2. Boruya YALNIZ `{kapsam, baslik, ozet, ozet_metni}` verilir (istek en çok 2048 bayt). `nonce`, `bitis`, `pc_adi`,
//!    `pc_imza` alanlarını TK hizmeti ekler; kasa nonce ÜRETMEZ. Hizmetin ürettiği `nonce` ve `bitis` borunun
//!    kabul cevabında döner ([`IstekKabulu`]); red/onay bununla eşlenir.
//! 3. `onay@1` cevabı katı biçimdedir ([`OnayCevabi`]): `tur="onay"`, `surum=1`, `kapsam`, `ozet`, `nonce`, `bitis`,
//!    `sayac`, `imza` (ASN.1 DER ECDSA P-256, küçük hex). BAŞKA ALAN YOK: bilinmeyen ya da eksik alan → RED
//!    (geçersiz, imzasız). `kapsam`/`ozet`/`nonce` istekle bayt bayt aynı; `şimdi < bitis ≤ istenen bitis`;
//!    `sayac` kasada saklanan son sayaçtan kesin büyük (tekrar → red).
//! 4. `onay_red@1`: `tur="onay_red"`, `surum=1`, `nonce`, `neden` (6 neden); red imzasızdır. Her redde gün sonu
//!    imzasız kalır ve neden ekranda Türkçe gösterilir.
//! 5. Onay kabul edilince özet kayıttan **yeniden** hesaplanır; tutarlar arada değiştiyse imza atılmaz. Cihaz/kişi
//!    bağını kasa kendi eşleşme kaydıyla kurar ([`EslesmeKaydi`]); cevapta cihaz/kişi alanı yoktur.
//!
//! Telefon imzasının kriptografik doğrulaması: imzalanan baytların tanımı (Ek 3) ve telefonun açık anahtarı
//! (eşleşme kaydı) bu depoda yok; P-256 kitaplığı da çalışma alanında yok. Bugün imza yalnız DER biçimi olarak
//! denetlenir, deftere yazılır ve [`EslesmeKaydi::imza_dogrula`] yer tutucusu `yapilmadi` döner.
//!
//! Köprü kanalı bir arayüzdür (`KopruKanali`). Gerçek kanal Windows adlandırılmış borusu
//! `\\.\pipe\TelefonKoprusuHizmet` olacaktır; bağlanana kadar `BoruKanali` her istekte `KanalYok` döner (API 501).

use crate::gunsonu::{
    GunSonuKaydi, ImzaKanali, ImzaKaydi, KasaDefteri, Kisi, TelefonKaniti, IMZA_ROLLERI,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

pub const ISTEK_TURU: &str = "onay_iste@1";
/// `onay@1` cevabının `tur` değeri (sürüm ayrı `surum` alanında).
pub const ONAY_TURU: &str = "onay";
/// `onay_red@1` cevabının `tur` değeri.
pub const RED_TURU: &str = "onay_red";
/// Cevapların `surum` değeri.
pub const CEVAP_SURUMU: i64 = 1;
pub const KAPSAM: &str = "taa_kasa_gun_sonu";
/// Özet metninin biçim sürümü (telefonda gösterilen ve hash'lenen metin). Sürüm 1 = tam ayrıntı
/// ([`ayrinti_metni`]); sürüm 2 = kısa okunur özet + ayrıntının SHA-256'sı (1 KiB sınırı için).
pub const OZET_SURUMU: u32 = 2;
/// Özet metni üst sınırı (bayt, UTF-8).
pub const OZET_METNI_EN_COK_BAYT: usize = 1024;
/// Boru isteği üst sınırı (bayt, JSON UTF-8).
pub const ISTEK_EN_COK_BAYT: usize = 2048;
/// Sürüm 2 metin alanlarının (şube, tarih, çekmece, id) en çok bayt sayısı (karakter sınırında kesilir).
pub const METIN_ALANI_EN_COK_BAYT: usize = 48;
/// İsteğin geçerlilik süresi (sn); TK hizmetinin `bitis` değeri en çok `şimdi + 60 + SAAT_PAYI_SN` kabul edilir.
pub const GECERLILIK_SN: i64 = 60;
/// Hizmetle kasa arasındaki saat kayması payı (sn).
pub const SAAT_PAYI_SN: i64 = 5;
/// `baslik` alanının en çok karakter sayısı (TK sözleşmesi).
pub const BASLIK_EN_COK: usize = 80;
/// Telefon köprüsünün yerel hizmet borusu.
pub const BORU_ADI: &str = r"\\.\pipe\TelefonKoprusuHizmet";
/// Köprü bağlı değilken ekranın ve API'nin gösterdiği mesaj (HTTP 501).
pub const KOPRU_YOK_MESAJI: &str =
    "Telefonla imza köprüsü henüz bağlı değil; imza bu ekrandan (yer tutucu) atılır.";

/// Özet biçim sürümü 1 = tam ayrıntı metni (sürüm 2'de `ayrinti_sha256` ile bağlanır): satır satır `anahtar=değer`, anahtarlar sözlük sırasında,
/// tutarlar kuruş cinsinden tam sayı. Kişi adı ve serbest not girmez (yalnız tutarlar ve kimlik).
pub fn ayrinti_metni(gs: &GunSonuKaydi) -> String {
    let mut s: Vec<(String, String)> = vec![
        ("kapsam".into(), KAPSAM.into()),
        ("surum".into(), "1".into()),
        ("id".into(), gs.id.clone()),
        ("tarih".into(), gs.tarih.clone()),
        ("sube".into(), gs.sube.clone()),
        ("cekmece".into(), gs.cekmece.clone()),
        (
            "nakit_sayilan_kurus".into(),
            gs.nakit_sayilan_kurus.to_string(),
        ),
        ("devreden_kurus".into(), gs.devreden_kurus.to_string()),
        (
            "nebim_toplam_kurus".into(),
            gs.nebim_toplam_kurus.to_string(),
        ),
        (
            "ciro_toplam_kurus".into(),
            gs.karsilastirma.ciro_toplam_kurus.to_string(),
        ),
    ];
    for (k, v) in &gs.ciro {
        s.push((format!("ciro.{k}"), v.to_string()));
    }
    for d in &gs.doviz {
        s.push((format!("doviz.{}.tutar", d.kod), d.tutar.to_string()));
        s.push((
            format!("doviz.{}.kur_onbinde", d.kod),
            d.kur_onbinde.to_string(),
        ));
    }
    for n in &gs.noktalar {
        s.push((
            format!("nokta.{}.z_toplam_kurus", n.nokta),
            n.z_toplam_kurus.to_string(),
        ));
        for (p, v) in &n.pos {
            s.push((format!("nokta.{}.pos.{p}", n.nokta), v.to_string()));
        }
    }
    s.sort();
    let mut m = String::new();
    for (k, v) in s {
        // Satır sonu ve '=' anahtar/değer içinde yer alamaz (kanonik biçim bozulmasın).
        let k = k.replace(['\n', '\r', '='], "_");
        let v = v.replace(['\n', '\r'], " ");
        m.push_str(&k);
        m.push('=');
        m.push_str(&v);
        m.push('\n');
    }
    m
}

/// Ayrıntı metninin (sürüm 1) SHA-256'sı. Sürüm 1 özetiyle imzalanmış eski kayıtlar bununla doğrulanır.
pub fn ayrinti_sha256(gs: &GunSonuKaydi) -> String {
    hex(&Sha256::digest(ayrinti_metni(gs).as_bytes()))
}

/// Metin alanını tek satıra indirir ve en çok `METIN_ALANI_EN_COK_BAYT` bayta, karakter sınırında keser.
fn kisa_alan(s: &str) -> String {
    let s = s.replace(['\n', '\r'], " ");
    if s.len() <= METIN_ALANI_EN_COK_BAYT {
        return s;
    }
    let mut n = METIN_ALANI_EN_COK_BAYT;
    while !s.is_char_boundary(n) {
        n -= 1;
    }
    s[..n].to_string()
}

/// Telefonda gösterilen ve hash'lenen özet metni (biçim sürümü 2). Sabit sırada, okunur toplamlar ve tüm
/// ayrıntıyı bağlayan `ayrinti_sha256` (sürüm 1 metninin SHA-256'sı). Kasa noktası, POS ve döviz sayısı ne
/// olursa olsun sınırlıdır (en çok ~600 bayt); 1 KiB'ı aşamaz. Kişi adı ve not girmez.
pub fn ozet_metni(gs: &GunSonuKaydi) -> String {
    let z: i128 = gs.noktalar.iter().map(|n| n.z_toplam_kurus as i128).sum();
    let pos: i128 = gs
        .noktalar
        .iter()
        .flat_map(|n| n.pos.values())
        .map(|v| *v as i128)
        .sum();
    let satirlar: [(&str, String); 15] = [
        ("kapsam", KAPSAM.into()),
        ("surum", OZET_SURUMU.to_string()),
        ("sube", kisa_alan(&gs.sube)),
        ("tarih", kisa_alan(&gs.tarih)),
        ("cekmece", kisa_alan(&gs.cekmece)),
        ("id", kisa_alan(&gs.id)),
        ("nakit_sayilan_kurus", gs.nakit_sayilan_kurus.to_string()),
        ("devreden_kurus", gs.devreden_kurus.to_string()),
        ("nebim_toplam_kurus", gs.nebim_toplam_kurus.to_string()),
        (
            "ciro_toplam_kurus",
            gs.karsilastirma.ciro_toplam_kurus.to_string(),
        ),
        ("z_toplam_kurus", z.to_string()),
        ("pos_toplam_kurus", pos.to_string()),
        ("nokta_adet", gs.noktalar.len().to_string()),
        ("doviz_adet", gs.doviz.len().to_string()),
        ("ayrinti_sha256", ayrinti_sha256(gs)),
    ];
    let mut m = String::new();
    for (k, v) in satirlar {
        m.push_str(k);
        m.push('=');
        m.push_str(&v);
        m.push('\n');
    }
    m
}

/// Özet metninin (sürüm 2) SHA-256'sı, küçük harf onaltılık: istekteki ve defterdeki `ozet`.
pub fn ozet_sha256(gs: &GunSonuKaydi) -> String {
    hex(&Sha256::digest(ozet_metni(gs).as_bytes()))
}

/// Defterdeki/paketteki `ozet` bu gün sonuna ait mi: sürüm 2 ya da (eski kayıt) sürüm 1.
pub fn ozet_eslesir(ozet: &str, gs: &GunSonuKaydi) -> bool {
    ozet == ozet_sha256(gs) || ozet == ayrinti_sha256(gs)
}

/// Telefonda bildirim başlığı: "Gün sonu · <şube> · <tarih>", en çok 80 karakter (Unicode karakter sayısı).
/// Aşarsa 79 karakter + "…" olur. Satır sonları boşluğa çevrilir.
pub fn baslik(gs: &GunSonuKaydi) -> String {
    let b =
        format!("Gün sonu · {} · {}", gs.sube.trim(), gs.tarih.trim()).replace(['\n', '\r'], " ");
    if b.chars().count() <= BASLIK_EN_COK {
        return b;
    }
    let mut k: String = b.chars().take(BASLIK_EN_COK - 1).collect();
    k.push('…');
    k
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

/// 64 hane küçük harf onaltılık mı.
pub fn hex64_mi(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

/// `onay_iste@1`: kasa programından boruya giden istek. YALNIZ bu dört alan; `nonce`, `bitis`, `pc_adi`,
/// `pc_imza` TK hizmeti tarafından eklenir.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OnayIstegi {
    pub kapsam: String,
    /// Telefon bildirim başlığı (en çok 80 karakter).
    pub baslik: String,
    /// `ozet_metni`'nin SHA-256'sı, 64 hane küçük harf onaltılık.
    pub ozet: String,
    /// Telefonda gösterilecek metin (en çok 1 KiB); telefon hash'ini kendisi hesaplayıp `ozet` ile karşılaştırır.
    pub ozet_metni: String,
}

impl OnayIstegi {
    /// Boruya yazılacak JSON baytları; sınırlar (metin 1 KiB, istek 2048 bayt) aşılırsa hata.
    pub fn bayt(&self) -> Result<Vec<u8>, String> {
        if self.ozet_metni.len() > OZET_METNI_EN_COK_BAYT {
            return Err(format!(
                "özet metni {} bayt; en çok {OZET_METNI_EN_COK_BAYT}",
                self.ozet_metni.len()
            ));
        }
        let b = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if b.len() > ISTEK_EN_COK_BAYT {
            return Err(format!(
                "istek {} bayt; en çok {ISTEK_EN_COK_BAYT}",
                b.len()
            ));
        }
        Ok(b)
    }
}

/// Borunun istek kabul cevabı: TK hizmetinin ürettiği tek kullanımlık `nonce` ve isteğin `bitis` anı.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IstekKabulu {
    pub nonce: String,
    pub bitis: i64,
}

impl IstekKabulu {
    /// `nonce` 64 küçük hex, `bitis` tamsayı ve `şimdi < bitis ≤ şimdi + 60 + saat payı` olmalı.
    pub fn jsondan(v: &Value, simdi: i64) -> Result<IstekKabulu, String> {
        let nonce = v
            .get("nonce")
            .and_then(Value::as_str)
            .filter(|n| hex64_mi(n))
            .ok_or("hizmet cevabında geçerli nonce yok (64 hane küçük hex)")?;
        let bitis = v
            .get("bitis")
            .and_then(Value::as_i64)
            .ok_or("hizmet cevabında bitis yok")?;
        if bitis <= simdi || bitis > simdi + GECERLILIK_SN + SAAT_PAYI_SN {
            return Err(format!(
                "hizmetin bitis değeri geçersiz ({} sn sonra)",
                bitis - simdi
            ));
        }
        Ok(IstekKabulu {
            nonce: nonce.into(),
            bitis,
        })
    }
}

/// `onay_red@1` nedenleri (TK sözleşmesi). Hepsinde gün sonu imzasız kalır.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RedNedeni {
    /// Kişi telefonda "Reddet" dedi.
    Kullanici,
    /// Biyometrik/PIN doğrulaması başarısız ya da iptal.
    Biyometri,
    /// Telefonda süre doldu.
    ZamanAsimi,
    /// Köprüde onay kipi kapalı.
    KipKapali,
    /// Eşleşmede bu kapsam (`taa_kasa_gun_sonu`) izinli değil.
    KapsamYok,
    /// Telefonun hesapladığı özet istekteki `ozet` ile uyuşmadı.
    OzetUyusmaz,
    /// Sözleşmede olmayan neden; yine red sayılır.
    Bilinmeyen(String),
}

impl RedNedeni {
    pub fn coz(kod: &str) -> RedNedeni {
        match kod.trim() {
            "kullanici" => RedNedeni::Kullanici,
            "biyometri" => RedNedeni::Biyometri,
            "zaman_asimi" => RedNedeni::ZamanAsimi,
            "kip_kapali" => RedNedeni::KipKapali,
            "kapsam_yok" => RedNedeni::KapsamYok,
            "ozet_uyusmaz" => RedNedeni::OzetUyusmaz,
            k => RedNedeni::Bilinmeyen(k.chars().take(80).collect()),
        }
    }
    pub fn kod(&self) -> &str {
        match self {
            RedNedeni::Kullanici => "kullanici",
            RedNedeni::Biyometri => "biyometri",
            RedNedeni::ZamanAsimi => "zaman_asimi",
            RedNedeni::KipKapali => "kip_kapali",
            RedNedeni::KapsamYok => "kapsam_yok",
            RedNedeni::OzetUyusmaz => "ozet_uyusmaz",
            RedNedeni::Bilinmeyen(k) => k,
        }
    }
    /// İmza ekranında gösterilen Türkçe açıklama (ne oldu + ne yapılmalı).
    pub fn mesaj(&self) -> String {
        match self {
            RedNedeni::Kullanici => "İmza telefonda reddedildi. Gün sonu imzasız kaldı; tutarları kontrol edip yeniden imza isteyin.".into(),
            RedNedeni::Biyometri => "Telefonda parmak izi / yüz / PIN doğrulaması geçmedi. Gün sonu imzasız kaldı; yeniden deneyin.".into(),
            RedNedeni::ZamanAsimi => "Telefonda 60 saniye içinde onay verilmedi. Gün sonu imzasız kaldı; yeniden imza isteyin.".into(),
            RedNedeni::KipKapali => "Telefon Köprüsü'nde onay kipi kapalı. Gün sonu imzasız kaldı; köprüde onay kipini açıp yeniden deneyin.".into(),
            RedNedeni::KapsamYok => "Bu telefona gün sonu imzası izni verilmemiş (kapsam yok). Gün sonu imzasız kaldı; eşleştirmede TAA Kasa gün sonu iznini açın.".into(),
            RedNedeni::OzetUyusmaz => "Telefondaki özet kasadaki özetle uyuşmadı. Gün sonu imzasız kaldı; ekranı yenileyip yeniden imza isteyin.".into(),
            RedNedeni::Bilinmeyen(k) if k.is_empty() => "Telefon imzayı reddetti (neden bildirilmedi). Gün sonu imzasız kaldı.".into(),
            RedNedeni::Bilinmeyen(k) => format!("Telefon imzayı reddetti (neden: {k}). Gün sonu imzasız kaldı."),
        }
    }
}

/// `onay@1` cevabı (katı biçim).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OnayCevabi {
    pub kapsam: String,
    pub ozet: String,
    pub nonce: String,
    pub bitis: i64,
    pub sayac: u64,
    /// ASN.1 DER ECDSA P-256 imzası, küçük hex.
    pub imza: String,
}

/// Köprüden gelen cevap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KopruCevabi {
    Onay(OnayCevabi),
    Red { nonce: String, neden: RedNedeni },
}

const ONAY_ALANLARI: [&str; 8] = [
    "tur", "surum", "kapsam", "ozet", "nonce", "bitis", "sayac", "imza",
];
const RED_ALANLARI: [&str; 4] = ["tur", "surum", "nonce", "neden"];

fn alanlar_tam(o: &serde_json::Map<String, Value>, izinli: &[&str]) -> Result<(), String> {
    if let Some(k) = o.keys().find(|k| !izinli.contains(&k.as_str())) {
        return Err(format!(
            "cevapta tanınmayan alan: {}",
            k.chars().take(40).collect::<String>()
        ));
    }
    if let Some(k) = izinli.iter().find(|k| !o.contains_key(**k)) {
        return Err(format!("cevapta {k} alanı yok"));
    }
    Ok(())
}

fn metin_al<'a>(o: &'a serde_json::Map<String, Value>, k: &str) -> Result<&'a str, String> {
    o.get(k)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{k} metin değil"))
}

/// ASN.1 DER ECDSA imzası mı: `30 L 02 lr r 02 ls s`, kısa uzunluk biçimi, r ve s 1–33 bayt, pozitif ve
/// gereksiz baştaki sıfır yok. (P-256 imzası en çok 72 bayt.)
pub fn der_ecdsa_mi(b: &[u8]) -> bool {
    fn tamsayi(b: &[u8]) -> Option<(usize, &[u8])> {
        if b.len() < 2 || b[0] != 0x02 {
            return None;
        }
        let n = b[1] as usize;
        if n == 0 || n > 33 || b.len() < 2 + n {
            return None;
        }
        let v = &b[2..2 + n];
        if v[0] & 0x80 != 0 || (n > 1 && v[0] == 0 && v[1] & 0x80 == 0) {
            return None;
        }
        Some((2 + n, &b[2 + n..]))
    }
    if b.len() < 8
        || b.len() > 72
        || b[0] != 0x30
        || b[1] as usize != b.len() - 2
        || b[1] & 0x80 != 0
    {
        return false;
    }
    match tamsayi(&b[2..]).and_then(|(_, kalan)| tamsayi(kalan)) {
        Some((_, kalan)) => kalan.is_empty(),
        None => false,
    }
}

fn hex_coz(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2)
        || !s
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

impl KopruCevabi {
    /// Katı çözüm (TK Ek 3). Bilinmeyen ya da eksik alan, yanlış tür/sürüm, biçimsiz değer → hata (= red).
    pub fn jsondan(v: &Value) -> Result<KopruCevabi, String> {
        let o = v.as_object().ok_or("cevap JSON nesnesi değil")?;
        let tur = metin_al(o, "tur")?;
        if o.get("surum").and_then(Value::as_i64) != Some(CEVAP_SURUMU) {
            return Err("cevap sürümü 1 değil".into());
        }
        match tur {
            ONAY_TURU => {
                alanlar_tam(o, &ONAY_ALANLARI)?;
                let ozet = metin_al(o, "ozet")?;
                let nonce = metin_al(o, "nonce")?;
                if !hex64_mi(ozet) || !hex64_mi(nonce) {
                    return Err("ozet ve nonce 64 hane küçük hex olmalı".into());
                }
                let bitis = o
                    .get("bitis")
                    .and_then(Value::as_i64)
                    .ok_or("bitis tamsayı değil")?;
                let sayac = o
                    .get("sayac")
                    .and_then(Value::as_u64)
                    .ok_or("sayac negatif olmayan tamsayı değil")?;
                let imza = metin_al(o, "imza")?;
                if !hex_coz(imza).is_some_and(|b| der_ecdsa_mi(&b)) {
                    return Err("imza ASN.1 DER ECDSA (küçük hex) değil".into());
                }
                Ok(KopruCevabi::Onay(OnayCevabi {
                    kapsam: metin_al(o, "kapsam")?.into(),
                    ozet: ozet.into(),
                    nonce: nonce.into(),
                    bitis,
                    sayac,
                    imza: imza.into(),
                }))
            }
            RED_TURU => {
                alanlar_tam(o, &RED_ALANLARI)?;
                let nonce = metin_al(o, "nonce")?;
                if !hex64_mi(nonce) {
                    return Err("nonce 64 hane küçük hex olmalı".into());
                }
                Ok(KopruCevabi::Red {
                    nonce: nonce.into(),
                    neden: RedNedeni::coz(metin_al(o, "neden")?),
                })
            }
            t => Err(format!(
                "bilinmeyen cevap türü: {}",
                t.chars().take(40).collect::<String>()
            )),
        }
    }
}

/// Telefon imzasının kasa tarafı denetim sonucu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImzaDenetimi {
    /// Eşleşmiş telefonun açık anahtarıyla doğrulandı.
    Dogrulandi,
    /// Doğrulanamadı (eşleşme kaydı / imzalanan bayt tanımı yok). İmzayı engellemez; kayda `yapilmadi` yazılır.
    Yapilmadi,
    /// İmza tutmadı: red.
    Gecersiz,
}

impl ImzaDenetimi {
    pub fn kod(&self) -> &'static str {
        match self {
            ImzaDenetimi::Dogrulandi => "dogrulandi",
            ImzaDenetimi::Yapilmadi => "yapilmadi",
            ImzaDenetimi::Gecersiz => "gecersiz",
        }
    }
}

/// Kasanın kendi TK eşleşme kaydı: cihaz kimliği ve telefonun açık anahtarı (P-256). Cevapta cihaz/kişi alanı
/// olmadığı için bağ buradan kurulur.
pub trait EslesmeKaydi {
    /// Bu PC'nin eşleşmiş telefonunun kimliği (defterdeki `cihaz`); kayıt yoksa `None`.
    fn cihaz(&self) -> Option<String>;
    /// Onay imzasını doğrular.
    fn imza_dogrula(&self, onay: &OnayCevabi) -> ImzaDenetimi;
}

/// Yer tutucu: eşleşme kaydı yok. Cihaz bilinmez, imza doğrulanamaz (`yapilmadi`).
#[derive(Debug, Clone, Copy, Default)]
pub struct EslesmeKaydiYok;

impl EslesmeKaydi for EslesmeKaydiYok {
    fn cihaz(&self) -> Option<String> {
        None
    }
    fn imza_dogrula(&self, _onay: &OnayCevabi) -> ImzaDenetimi {
        ImzaDenetimi::Yapilmadi
    }
}

/// TAA Merkez'in cihaz-kişi bağı ve iptal kaydının denetim sonucu. Bağ ve iptal TAA Merkez'de tutulur; kasa
/// programı ve merkez paket okuyucusu yalnız bu sonucu kullanır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CihazDurumu {
    /// Cihaz bu kişiye bağlı ve iptal edilmemiş.
    Gecerli,
    /// Cihaz başka bir kişiye bağlı.
    BaskaKisi,
    /// Cihaz iptal edildi (kayıp, işten ayrılma).
    Iptal,
    /// Cihaz bu kişiye bağlı ama kişi TAA Merkez'de yönetici değil (imza ve onay yetkisi yok).
    YetkisizKisi,
    /// Merkez kaydı yok ya da ulaşılamıyor.
    Bilinmiyor,
}

impl CihazDurumu {
    pub fn kod(&self) -> &'static str {
        match self {
            CihazDurumu::Gecerli => "gecerli",
            CihazDurumu::BaskaKisi => "baska_kisi",
            CihazDurumu::Iptal => "iptal",
            CihazDurumu::YetkisizKisi => "yetkisiz_kisi",
            CihazDurumu::Bilinmiyor => "bilinmiyor",
        }
    }
}

/// TAA Merkez cihaz kaydı arayüzü (`cihaz`: `eslesme_kimligi:pc_pin`).
pub trait CihazKaydi {
    fn denetle(&self, cihaz: &str, kisi: &str) -> CihazDurumu;
}

/// Yer tutucu: TAA Merkez cihaz kaydı bağlanana kadar her cihaz `Bilinmiyor`. Bilinmiyor imzayı engellemez,
/// imza kaydına `cihaz_dogrulama: "bilinmiyor"` yazılır ve merkezde "doğrulanamadı" görünür.
#[derive(Debug, Clone, Copy, Default)]
pub struct MerkezKaydiYok;

impl CihazKaydi for MerkezKaydiYok {
    fn denetle(&self, _cihaz: &str, _kisi: &str) -> CihazDurumu {
        CihazDurumu::Bilinmiyor
    }
}

/// Kanal hatası: `Yok` köprü kurulu/izinli değil (ekran 501 gösterir), `Hata` geçici iletişim hatası.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KanalHatasi {
    Yok(String),
    Hata(String),
}

/// Telefon Köprüsü kanalı. Gerçek uygulama adlandırılmış boru üzerinden JSON satırları taşır.
pub trait KopruKanali {
    /// İsteği (yalnız dört alan) boruya yazar; hizmetin kabul cevabını (`nonce`, `bitis`) döner.
    fn gonder(&mut self, istek: &OnayIstegi) -> Result<Value, KanalHatasi>;
    /// Bu nonce için cevap geldiyse döner; gelmediyse `None`.
    fn yokla(&mut self, nonce: &str) -> Result<Option<Value>, KanalHatasi>;
}

/// Yer tutucu boru kanalı: köprü onay akışı birleşip borunun erişim listesine taa-kasa.exe eklenene kadar
/// her çağrıda `KanalYok` döner. Boruya hiçbir şey yazmaz.
#[derive(Debug, Clone)]
pub struct BoruKanali {
    pub boru: String,
}

impl Default for BoruKanali {
    fn default() -> Self {
        BoruKanali {
            boru: BORU_ADI.into(),
        }
    }
}

impl KopruKanali for BoruKanali {
    fn gonder(&mut self, _istek: &OnayIstegi) -> Result<Value, KanalHatasi> {
        Err(KanalHatasi::Yok(KOPRU_YOK_MESAJI.into()))
    }
    fn yokla(&mut self, _nonce: &str) -> Result<Option<Value>, KanalHatasi> {
        Err(KanalHatasi::Yok(KOPRU_YOK_MESAJI.into()))
    }
}

/// İmza oturumunun durumu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImzaDurumu {
    /// İstek telefonda, cevap bekleniyor.
    Bekliyor,
    /// Onaylandı ve deftere yazıldı.
    Onaylandi(Box<ImzaKaydi>),
    /// Telefon `onay_red@1` gönderdi; gün sonu imzasız kaldı.
    Reddedildi { neden: RedNedeni },
    /// 60 sn içinde cevap gelmedi (kasa tarafı süre); gün sonu imzasız kaldı.
    ZamanAsimi,
    /// Köprü bağlı değil (API 501 döner).
    KanalYok { neden: String },
    /// Cevap geldi ama geçersiz (özet değişti, nonce tutmuyor, bilinmeyen alan, sayaç tekrarı, defter kuralı vb.); imza atılmadı.
    Gecersiz { neden: String },
}

impl ImzaDurumu {
    pub fn bitti(&self) -> bool {
        !matches!(self, ImzaDurumu::Bekliyor)
    }
    /// Bitti ama imza atılmadı mı (her red, zaman aşımı, geçersiz cevap, kanal yok).
    pub fn imzasiz_bitti(&self) -> bool {
        self.bitti() && !matches!(self, ImzaDurumu::Onaylandi(_))
    }
    /// Red nedeni kodu (yalnız `Reddedildi`).
    pub fn red_nedeni(&self) -> Option<&str> {
        match self {
            ImzaDurumu::Reddedildi { neden } => Some(neden.kod()),
            _ => None,
        }
    }
    /// İmza ekranında gösterilen Türkçe mesaj.
    pub fn mesaj(&self) -> String {
        match self {
            ImzaDurumu::Bekliyor => "Telefonda onay bekleniyor (60 saniye).".into(),
            ImzaDurumu::Onaylandi(i) => format!("{} telefonla imzaladı.", i.kisi),
            ImzaDurumu::Reddedildi { neden } => neden.mesaj(),
            ImzaDurumu::ZamanAsimi => RedNedeni::ZamanAsimi.mesaj(),
            ImzaDurumu::KanalYok { neden } => neden.clone(),
            ImzaDurumu::Gecersiz { neden } => {
                format!("Telefon onayı kabul edilmedi: {neden}. Gün sonu imzasız kaldı.")
            }
        }
    }
    /// Ekran/API için kısa kod.
    pub fn kod(&self) -> &'static str {
        match self {
            ImzaDurumu::Bekliyor => "bekliyor",
            ImzaDurumu::Onaylandi(_) => "onaylandi",
            ImzaDurumu::Reddedildi { .. } => "reddedildi",
            ImzaDurumu::ZamanAsimi => "zaman_asimi",
            ImzaDurumu::KanalYok { .. } => "kanal_yok",
            ImzaDurumu::Gecersiz { .. } => "gecersiz",
        }
    }
}

/// Bir gün sonu imzası için telefon oturumu.
#[derive(Debug, Clone)]
pub struct TelefonImzaOturumu {
    /// Boruya giden istek (dört alan).
    pub istek: OnayIstegi,
    pub gun_sonu_id: String,
    /// 1: kasa sorumlusu, 2: mağaza sorumlusu / vekili.
    pub sira: u8,
    pub rol: String,
    /// TK hizmetinin ürettiği nonce (kabul edilmediyse boş).
    pub nonce: String,
    /// TK hizmetinin bildirdiği bitiş anı (Unix sn; kabul edilmediyse 0).
    pub bitis: i64,
    pub durum: ImzaDurumu,
    kisi: Kisi,
}

impl TelefonImzaOturumu {
    /// İsteği hazırlar ve kanala verir. Nonce'u kasa üretmez: hizmetin kabul cevabından alınır. Kapanış listesi
    /// ve imza kuralları istek açılmadan denetlenir; kural ihlalinde ya da boyut sınırı aşılırsa telefon hiç
    /// rahatsız edilmez.
    pub fn baslat(
        defter: &KasaDefteri,
        kanal: &mut dyn KopruKanali,
        gun_sonu_id: &str,
        kisi: &Kisi,
        simdi: i64,
    ) -> Result<TelefonImzaOturumu, String> {
        if kisi.ad.trim().is_empty() {
            return Err("imzalayan kişiyi seçin".into());
        }
        let gs = defter
            .gun_sonlari()?
            .into_iter()
            .find(|s| s.id == gun_sonu_id)
            .ok_or_else(|| format!("geçerli gün sonu yok: {gun_sonu_id}"))?;
        let sira = defter.imza_on_denetim(&gs, kisi)?;
        let istek = OnayIstegi {
            kapsam: KAPSAM.into(),
            baslik: baslik(&gs),
            ozet: ozet_sha256(&gs),
            ozet_metni: ozet_metni(&gs),
        };
        istek.bayt()?;
        let mut o = TelefonImzaOturumu {
            istek,
            gun_sonu_id: gs.id.clone(),
            sira,
            rol: IMZA_ROLLERI[sira as usize - 1].0.into(),
            nonce: String::new(),
            bitis: 0,
            durum: ImzaDurumu::Bekliyor,
            kisi: kisi.clone(),
        };
        o.durum = match kanal.gonder(&o.istek) {
            Ok(v) => match IstekKabulu::jsondan(&v, simdi) {
                Ok(k) => {
                    o.nonce = k.nonce;
                    o.bitis = k.bitis;
                    ImzaDurumu::Bekliyor
                }
                Err(e) => ImzaDurumu::Gecersiz { neden: e },
            },
            Err(KanalHatasi::Yok(n)) => ImzaDurumu::KanalYok { neden: n },
            Err(KanalHatasi::Hata(n)) => ImzaDurumu::Gecersiz {
                neden: format!("istek köprüye iletilemedi: {n}"),
            },
        };
        Ok(o)
    }

    /// Cevabı yoklar ve durumu ilerletir (merkez cihaz kaydı ve eşleşme kaydı yer tutucu).
    pub fn ilerle(
        &mut self,
        defter: &KasaDefteri,
        kanal: &mut dyn KopruKanali,
        simdi: i64,
    ) -> &ImzaDurumu {
        self.ilerle_denetle(defter, kanal, &MerkezKaydiYok, &EslesmeKaydiYok, simdi)
    }

    /// `ilerle`, TAA Merkez cihaz kaydı ve kasanın eşleşme kaydıyla.
    pub fn ilerle_denetle(
        &mut self,
        defter: &KasaDefteri,
        kanal: &mut dyn KopruKanali,
        kayit: &dyn CihazKaydi,
        eslesme: &dyn EslesmeKaydi,
        simdi: i64,
    ) -> &ImzaDurumu {
        if self.durum.bitti() {
            return &self.durum;
        }
        let cevap = match kanal.yokla(&self.nonce) {
            Ok(c) => c,
            Err(KanalHatasi::Yok(n)) => {
                self.durum = ImzaDurumu::KanalYok { neden: n };
                return &self.durum;
            }
            // Geçici hata: süre dolana kadar beklemeye devam.
            Err(KanalHatasi::Hata(_)) => None,
        };
        // Bitiş anında ya da sonra gelen onay kabul edilmez (şimdi < bitis).
        if simdi >= self.bitis {
            self.durum = ImzaDurumu::ZamanAsimi;
            return &self.durum;
        }
        if let Some(v) = cevap {
            self.durum = self.cevabi_isle(defter, kayit, eslesme, &v, simdi);
        }
        &self.durum
    }

    fn cevabi_isle(
        &self,
        defter: &KasaDefteri,
        kayit: &dyn CihazKaydi,
        eslesme: &dyn EslesmeKaydi,
        v: &Value,
        simdi: i64,
    ) -> ImzaDurumu {
        let gecersiz = |n: &str| ImzaDurumu::Gecersiz { neden: n.into() };
        let c = match KopruCevabi::jsondan(v) {
            Ok(c) => c,
            Err(e) => return gecersiz(&e),
        };
        let o = match c {
            KopruCevabi::Red { nonce, neden } => {
                if nonce != self.nonce {
                    return gecersiz("cevap başka bir isteğe ait");
                }
                return ImzaDurumu::Reddedildi { neden };
            }
            KopruCevabi::Onay(o) => o,
        };
        if o.nonce != self.nonce {
            return gecersiz("cevap başka bir isteğe ait (nonce tutmuyor)");
        }
        if o.kapsam != self.istek.kapsam {
            return gecersiz("cevabın kapsamı istekle aynı değil");
        }
        if o.ozet != self.istek.ozet {
            return gecersiz("telefonda onaylanan özet istenen özetle aynı değil");
        }
        if !(simdi < o.bitis && o.bitis <= self.bitis) {
            return gecersiz("onayın bitiş anı geçersiz (süresi dolmuş ya da istenenden geç)");
        }
        match defter.telefon_sayaci() {
            Ok(Some(son)) if o.sayac <= son => {
                return gecersiz("onay sayacı tekrar ya da geri (yeniden oynatma)")
            }
            Ok(_) => {}
            Err(e) => return gecersiz(&format!("son sayaç okunamadı: {e}")),
        }
        let imza_denetimi = eslesme.imza_dogrula(&o);
        if imza_denetimi == ImzaDenetimi::Gecersiz {
            return gecersiz("telefon imzası eşleşmiş telefonun anahtarıyla doğrulanmadı");
        }
        // Biçimce ve bağlamca geçerli onay: sayaç ilerler (imza sonradan defter kuralına takılsa da tekrar
        // kullanılamaz).
        if let Err(e) = defter.telefon_sayaci_yaz(o.sayac) {
            return gecersiz(&format!("sayaç yazılamadı: {e}"));
        }
        // Tutarlar istekten sonra değiştiyse (düzeltme/iptal) imza atılmaz.
        let guncel = match defter
            .gun_sonlari()
            .ok()
            .and_then(|v| v.into_iter().find(|s| s.id == self.gun_sonu_id))
        {
            Some(g) => g,
            None => return gecersiz("gün sonu artık geçerli değil (iptal ya da düzeltme)"),
        };
        if ozet_sha256(&guncel) != self.istek.ozet {
            return gecersiz("gün sonu tutarları istek gönderildikten sonra değişti");
        }
        let cihaz = eslesme.cihaz();
        let dd = match &cihaz {
            Some(c) => kayit.denetle(c, &self.kisi.ad),
            None => CihazDurumu::Bilinmiyor,
        };
        match dd {
            CihazDurumu::Iptal => return gecersiz("cihaz TAA Merkez'de iptal edilmiş"),
            CihazDurumu::BaskaKisi => {
                return gecersiz("cihaz TAA Merkez'de başka bir kişiye bağlı")
            }
            CihazDurumu::YetkisizKisi => {
                return gecersiz("kişi TAA Merkez'de yönetici değil; imza yetkisi yok")
            }
            CihazDurumu::Gecerli | CihazDurumu::Bilinmiyor => {}
        }
        let kanit = TelefonKaniti {
            nonce: o.nonce.clone(),
            sayac: o.sayac,
            bitis: o.bitis,
            imza: o.imza.clone(),
            imza_dogrulama: imza_denetimi.kod().into(),
        };
        match defter.imzala_ic(
            &self.gun_sonu_id,
            ImzaKanali::Telefon,
            cihaz.as_deref(),
            Some(&self.istek.ozet),
            Some(dd.kod()),
            Some(&kanit),
            &self.kisi,
        ) {
            Ok(i) => ImzaDurumu::Onaylandi(Box::new(i)),
            Err(e) => gecersiz(&e),
        }
    }
}

#[cfg(test)]
#[path = "testler_telefon_imza.rs"]
mod testler;
