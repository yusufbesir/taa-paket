//! Karekodla teslim onayı (Yusuf 2026-11-26: "Fişsiz de teslim için müşteriden imza alınsın ama imza fiziksel
//! değil, qr kod ile olsun. Kimin telefonundan okunduğu yazsın loglarda, teslim tarihi ve saati yazsın. Teslim alanın
//! ismi telefon numarası da yazsın.").
//!
//! Akış (bulut sözleşmesi; bulut tarafı `bulut/` klasöründedir):
//! 1. Aç: mağaza sunucusu 20 karakterlik belirteç `t` üretir, iki buluta (birincil Cloudflare, yedek Vercel;
//!    `musteri-takip.json` adresleri) imzalı `POST /api/onay/ac` gönderir. Gövdede sunucunun X25519 GENEL anahtarı
//!    (`pk`) vardır. Ekranda `<birincil>/o/<t>` karekodu gösterilir (birincile ulaşılamadıysa yedek adres).
//! 2. Müşteri kendi telefonuyla okutur, bulut sayfasında ad soyad + telefon yazıp onaylar. Bulut, form verisini
//!    hemen `pk` ile şifreler (geçici X25519 + HKDF-SHA256 + AES-256-GCM, aad = t); düz metin bulutta saklanmaz.
//! 3. Oku: sunucu imzalı `GET /api/onay/<t>` ile durumu sorar; "onaylandi" ise şifreli bloğu kendi özel anahtarıyla
//!    çözer (yalnız bu sunucu çözebilir).
//! 4. Kapat: sunucu imzalı `POST /api/onay/kapat` ile bulut kaydını sildirir.
//!
//! İmza (Yusuf 2026-11-26: "telefon üzerinden de imza alınsın dokunmatik ekran ile — fişi yanında olmayan için"):
//! - Aç gövdesinde `"i": 1` (fişsiz teslim: müşteri karekod sayfasında parmakla imza çizer, imza ZORUNLU) ya da
//!   `"i": 0` (fişli: imza istenmez/isteğe bağlı). Eski bulut `i` alanını bilmiyorsa 0 sayar. İstemci karekodu
//!   `POST /api/fis/<no>/teslim-qr {"fisli": false}` ile açınca i = 1 olur (gövde yoksa fişli sayılır).
//! - Bulut, onaylanan düz metin JSON'a `"imza": "<base64 PNG>"` ekler (i = 1 ise zorunlu) ve hepsini birlikte
//!   şifreler. Sunucu çözünce PNG'yi doğrular (PNG imzası + IHDR, en çok 300 KB, kenar en çok 8192 px); teslimde
//!   imza yerelde ŞİFRELİ saklanır (`imzalar/<no>-<n>.png.enc`, AES-256-GCM yerel anahtar, aad = fiş no), kayda
//!   `imza_sha256` ve `imza_yontemi: "qr_imza"` yazılır. Karekoddan imza gelmezse (eski bulut) fişsiz teslim için
//!   mağaza cihazının ekranında imza ("cihaz_imza") alınır; imzasız fişsiz teslim 422 `imza_gerekli`.
//!
//! Güvenlik:
//! - Mağaza sunucusu internete AÇILMAZ; bütün bağlantılar sunucudan dışarı doğrudur (curl).
//! - X25519 özel anahtarı `musteri-takip/teslim-x25519.anahtar` dosyasındadır ("x25519-ozel:<hex>"), dışarı çıkmaz.
//! - Belirteç ve çözülen bilgi yalnız BELLEKTE, fiş + belirteç eşlemesiyle en çok 15 dk durur; teslim tamamlanınca
//!   silinir. Kalıcı kayıt yalnız şifreli teslim kaydıdır (bkz. `teslim_sifre`).
//! - Ad, telefon, cihaz, IP hiçbir günlüğe yazılmaz.
//! - Sunucu tek iş parçacıklıdır: 2 sn'lik durum sorgusu bulutu ARKA PLANDA okur, istek hemen yanıtlanır.
//!
//! Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

use super::*;
use crate::musteri_takip::{dosya_ya_da_olustur, hex, hex_coz, rastgele, ALFABE};
use crate::teslim_sifre::{base64_coz, cihaz_ozeti, tel_maskele, tel_normal};
use std::collections::BTreeMap;
use std::sync::Arc;

/// Belirteç uzunluğu (alfabe 32 harf: 20 karakter = 100 bit).
pub const BELIRTEC_UZUNLUGU: usize = 20;
/// Karekodun ömrü (sözleşme: sil <= ts + 900).
pub const ONAY_OMRU_SN: u64 = 15 * 60;
/// Bir fişe aynı anda en çok bu kadar geçerli karekod.
const FIS_BASINA_EN_COK: usize = 5;
/// HKDF bilgi dizesi (sözleşme).
pub const HKDF_BILGI: &[u8] = b"taa-teslim-onay-v1";
const ANAHTAR_DOSYASI: &str = "teslim-x25519.anahtar";
/// Bulut isteği zaman aşımı (sn). Aç isteği iki buluta paralel gider.
const BULUT_SURESI_SN: u32 = 8;
/// Şifreli onayın (imza dahil) en büyük boyu (bayt).
const EN_BUYUK_SIFRELI: usize = crate::teslim_al::EN_BUYUK_IMZA * 4 / 3 + 8 * 1024;
/// Bulut ile mağaza bilgisayarı saatleri arasında kabul edilen kayma (bulutun aç payıyla aynı).
const SAAT_PAYI_SN: u64 = 600;
/// Bulut saat farkı nedeniyle isteği reddettiğinde (aç: 400 "zaman geçersiz", oku: 401) ekranda gösterilen neden.
pub const SAAT_SORUNU: &str = "saat";
/// Şube kodu (bulut sözleşmesi: ^[A-Za-z0-9_-]{1,16}$).
pub fn sube_kodu_gecerli(s: &str) -> bool {
    (1..=16).contains(&s.len())
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

/// Onaydaki zaman (bulutun saati) karekodun açılış..silinme aralığında mı (saat kayması payıyla)?
pub fn onay_zamani_gecerli(zaman: u64, sil: u64) -> bool {
    let acilis = sil.saturating_sub(ONAY_OMRU_SN);
    zaman + SAAT_PAYI_SN >= acilis && zaman <= sil + SAAT_PAYI_SN
}

/// Bulut yanıtı saat farkından mı reddedildi? (onay.mjs: "zaman geçersiz" / "silinme zamanı geçersiz")
fn saat_reddi(kod: u16, govde: &str) -> bool {
    matches!(kod, 400 | 401) && govde.contains("zaman")
}

/// Bulutla konuşan taraf (testte sahte bulut kullanılır).
pub trait OnayAgi: Send + Sync {
    /// (HTTP kodu, yanıt gövdesi). Ağ hatasında kod 0.
    fn istek(
        &self,
        metot: &str,
        url: &str,
        basliklar: &[(&str, &str)],
        govde: Option<&str>,
    ) -> Result<(u16, String), String>;
}

/// `curl` ile (musteri_takip gönderim yardımcısı).
pub struct CurlOnayAgi;

impl OnayAgi for CurlOnayAgi {
    fn istek(
        &self,
        metot: &str,
        url: &str,
        basliklar: &[(&str, &str)],
        govde: Option<&str>,
    ) -> Result<(u16, String), String> {
        musteri_takip::curl_istek(metot, url, basliklar, govde, BULUT_SURESI_SN)
    }
}

/// Müşterinin onay formundan gelen (çözülmüş) bilgi. Yalnız bellekte.
#[derive(Clone, PartialEq, Eq)]
pub struct OnayBilgisi {
    pub ad: String,
    /// "+<rakamlar>"
    pub tel: String,
    /// Okutan telefonun user-agent'ı (en çok 200 karakter).
    pub ua: String,
    pub ip: String,
    /// Onay anı (bulutun saati), unix saniyesi.
    pub zaman: u64,
    pub dil: String,
    /// Müşterinin karekod sayfasında parmakla çizdiği imza (doğrulanmış PNG baytları); yoksa `None`.
    pub imza: Option<Vec<u8>>,
}

/// Hata iletilerinde ve test çıktısında kişisel veri ya da imza baytı görünmesin.
impl std::fmt::Debug for OnayBilgisi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OnayBilgisi")
            .field("zaman", &self.zaman)
            .field("imza", &self.imza.as_ref().map(|b| b.len()))
            .finish_non_exhaustive()
    }
}

/// Bellekteki karekod kaydı.
pub(crate) struct OnayKaydi {
    no: String,
    /// Karekodu açan cihaz (yalnız o cihaz sorgular ve teslimi tamamlar).
    cihaz: String,
    /// Karekod fişsiz teslim için açıldı: bulut müşteriden imza ister ("i": 1).
    imza_gerekli: bool,
    sil: u64,
    /// Kaydın açıldığı bulut tabanları; karekoddaki adres ilk sıradadır.
    tabanlar: Vec<String>,
    bilgi: Option<OnayBilgisi>,
    hata: Option<String>,
    /// Son bulut okumasında ulaşılamadıysa nedeni (ekranda "bağlantı sorunu" uyarısı).
    baglanti: Option<String>,
    son_sorgu: u64,
    sorguda: bool,
}

pub(crate) type OnayHaritasi = Arc<Mutex<BTreeMap<String, OnayKaydi>>>;

pub fn belirtec_gecerli(t: &str) -> bool {
    t.len() == BELIRTEC_UZUNLUGU && t.bytes().all(|b| ALFABE.contains(&b))
}

/// Rastgele belirteç (256 = 8 x 32: alt 5 bit tekdüze dağılır).
pub fn yeni_belirtec() -> Result<String, String> {
    Ok(rastgele(BELIRTEC_UZUNLUGU)?
        .iter()
        .map(|b| ALFABE[(b & 31) as usize] as char)
        .collect())
}

/// Sunucunun uzun ömürlü X25519 özel anahtarı (yoksa oluşturulur, üzerine yazılmaz).
pub fn ozel_anahtar(dizin: &Path) -> Result<x25519_dalek::StaticSecret, String> {
    let yol = musteri_takip::klasor(dizin).join(ANAHTAR_DOSYASI);
    let yeni = !yol.exists();
    let m = dosya_ya_da_olustur(&yol, || Ok(format!("x25519-ozel:{}", hex(&rastgele(32)?))))?;
    #[cfg(unix)]
    if yeni {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&yol, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(not(unix))]
    let _ = yeni;
    let b: [u8; 32] = m
        .trim()
        .strip_prefix("x25519-ozel:")
        .and_then(hex_coz)
        .and_then(|v| v.try_into().ok())
        .ok_or("teslim X25519 anahtarı bozuk")?;
    Ok(x25519_dalek::StaticSecret::from(b))
}

/// Buluta verilen genel anahtar (64 onaltılık hane).
pub fn genel_anahtar(dizin: &Path) -> Result<String, String> {
    Ok(hex(
        x25519_dalek::PublicKey::from(&ozel_anahtar(dizin)?).as_bytes()
    ))
}

/// HKDF-SHA256(ikm = paylaşılan, salt = geçici_genel || pk, info = "taa-teslim-onay-v1") -> 32 bayt.
fn anahtar_tur(paylasilan: &[u8; 32], e: &[u8; 32], pk: &[u8; 32]) -> Result<[u8; 32], String> {
    let mut tuz = [0u8; 64];
    tuz[..32].copy_from_slice(e);
    tuz[32..].copy_from_slice(pk);
    let h = hkdf::Hkdf::<sha2::Sha256>::new(Some(&tuz), paylasilan);
    let mut k = [0u8; 32];
    h.expand(HKDF_BILGI, &mut k).map_err(|_| "hkdf")?;
    Ok(k)
}

/// Sözleşmedeki şifreyi çözer: e (geçici genel, hex 64), iv (hex 24), c (base64: şifreli + 16 bayt etiket),
/// aad = UTF-8 t. Düz metin baytlarını döndürür.
pub fn onay_coz(
    ozel: &x25519_dalek::StaticSecret,
    t: &str,
    e_hex: &str,
    iv_hex: &str,
    c_b64: &str,
) -> Result<Vec<u8>, String> {
    use aes_gcm::aead::{Aead, KeyInit, Payload};
    let e: [u8; 32] = hex_coz(e_hex)
        .and_then(|v| v.try_into().ok())
        .ok_or("geçici anahtar (e) bozuk")?;
    let iv = hex_coz(iv_hex)
        .filter(|v| v.len() == 12)
        .ok_or("iv bozuk")?;
    // İmza (base64 PNG, en çok 300 KB) da şifreli onayın içindedir: 300 KB * 4/3 + form alanları.
    let c = base64_coz(c_b64)
        .filter(|v| v.len() > 16 && v.len() <= EN_BUYUK_SIFRELI)
        .ok_or("şifreli metin bozuk")?;
    let pk = x25519_dalek::PublicKey::from(ozel);
    let paylasilan = ozel.diffie_hellman(&x25519_dalek::PublicKey::from(e));
    // Düşük mertebeli (sıfır) nokta: anahtar tahmin edilebilir olur; reddedilir.
    if !paylasilan.was_contributory() {
        return Err("geçici anahtar geçersiz".into());
    }
    let k = anahtar_tur(paylasilan.as_bytes(), &e, pk.as_bytes())?;
    let s = aes_gcm::Aes256Gcm::new_from_slice(&k).map_err(|_| "anahtar")?;
    s.decrypt(
        aes_gcm::Nonce::from_slice(&iv),
        Payload {
            msg: &c,
            aad: t.as_bytes(),
        },
    )
    .map_err(|_| "onay çözülemedi (kurcalanmış ya da başka sunucunun)".into())
}

/// Çözülen onay JSON'unu denetler (bulut tarafı da denetler; burada yeniden, güvenmeden).
/// Ad kuralı bulutunkinin (onay.mjs adDogrula) üst kümesidir: harf + birleşik işaret + ZWNJ/ZWJ + boşluk .-'
/// (yön işaretleri ve diğer görünmez karakterler `ad_karakterleri_gecerli` içinde reddedilir); en az bir harf.
pub fn onay_bilgisi(duz: &[u8]) -> Result<OnayBilgisi, String> {
    let v: serde_json::Value = serde_json::from_slice(duz).map_err(|_| "onay JSON değil")?;
    let s = |k: &str| v[k].as_str().unwrap_or("").trim().to_string();
    let ad = s("ad").split_whitespace().collect::<Vec<_>>().join(" ");
    let n = ad.chars().count();
    if !(2..=60).contains(&n)
        || !taa_fis::ad_karakterleri_gecerli(&ad)
        || !ad.chars().any(char::is_alphabetic)
    {
        return Err("onaydaki ad geçersiz".into());
    }
    let tel = tel_normal(&s("tel"))
        .filter(|t| !t.is_empty())
        .ok_or("onaydaki telefon geçersiz")?;
    let ua: String = s("ua")
        .chars()
        .filter(|c| !c.is_control())
        .take(200)
        .collect();
    let ip = s("ip");
    if ip.len() > 64
        || !ip
            .bytes()
            .all(|b| b.is_ascii_hexdigit() || b == b'.' || b == b':')
    {
        return Err("onaydaki IP geçersiz".into());
    }
    let zaman = v["zaman"].as_u64().ok_or("onay zamanı yok")?;
    let dil: String = s("dil")
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-')
        .take(8)
        .collect();
    // İsteğe bağlı imza: base64 PNG (data: öneki kabul). Varsa geçerli olmalı; bozuksa onay reddedilir.
    let imza = match v["imza"].as_str().map(str::trim).filter(|x| !x.is_empty()) {
        None => None,
        Some(x) => Some(
            crate::teslim_al::imza_coz_denetle(x)
                .map_err(|(_, e)| format!("onaydaki imza geçersiz ({e})"))?,
        ),
    };
    Ok(OnayBilgisi {
        ad,
        tel,
        ua,
        ip,
        zaman,
        dil,
        imza,
    })
}

/// Bulut okuma sonucu.
enum Bulut {
    Bekliyor,
    Onaylandi { e: String, iv: String, c: String },
    Yok,
    Ulasilamadi(String),
}

/// İmzalı okuma: tabanları sırayla dener; bir tabanın "onaylandi" yanıtı yeterlidir.
fn bulut_oku(dizin: &Path, agi: &dyn OnayAgi, t: &str, tabanlar: &[String], ts: u64) -> Bulut {
    use ed25519_dalek::Signer;
    let Ok(k) = musteri_takip::imza_anahtari(dizin) else {
        return Bulut::Ulasilamadi("imza anahtarı okunamadı".into());
    };
    let imza = hex(&k.sign(format!("onay_oku:{t}:{ts}").as_bytes()).to_bytes());
    let ts_s = ts.to_string();
    let mut sonuc = Bulut::Ulasilamadi("bulut adresi yok".into());
    for b in tabanlar {
        let url = format!("{b}/api/onay/{t}");
        match agi.istek(
            "GET",
            &url,
            &[("x-taa-zaman", &ts_s), ("x-taa-imza", &imza)],
            None,
        ) {
            Ok((200, g)) => {
                let v: serde_json::Value = serde_json::from_str(&g).unwrap_or_default();
                match v["durum"].as_str() {
                    Some("onaylandi") => {
                        let a = |k: &str| v[k].as_str().unwrap_or("").to_string();
                        return Bulut::Onaylandi {
                            e: a("e"),
                            iv: a("iv"),
                            c: a("c"),
                        };
                    }
                    Some("bekliyor") => sonuc = Bulut::Bekliyor,
                    _ => {
                        if !matches!(sonuc, Bulut::Bekliyor) {
                            sonuc = Bulut::Ulasilamadi("bulut yanıtı anlaşılmadı".into());
                        }
                    }
                }
            }
            Ok((404, _)) => {
                if !matches!(sonuc, Bulut::Bekliyor) {
                    sonuc = Bulut::Yok;
                }
            }
            Ok((kod, g)) => {
                if matches!(sonuc, Bulut::Ulasilamadi(_)) {
                    sonuc = Bulut::Ulasilamadi(if saat_reddi(kod, &g) {
                        SAAT_SORUNU.to_string()
                    } else {
                        format!("HTTP {kod}")
                    });
                }
            }
            Err(e) => {
                if matches!(sonuc, Bulut::Ulasilamadi(_)) {
                    sonuc = Bulut::Ulasilamadi(e);
                }
            }
        }
    }
    sonuc
}

/// İmzalı kapat (bulut kaydını sildirir). Sonuç önemsenmez: bulut kaydı zaten en çok 15 dk yaşar.
fn bulut_kapat(dizin: &Path, agi: &dyn OnayAgi, t: &str, tabanlar: &[String], ts: u64) {
    let govde = json!({ "v": 1, "tur": "onay_kapat", "t": t, "ts": ts });
    let Ok((g, imza)) = musteri_takip::imzala(dizin, &govde) else {
        return;
    };
    for b in tabanlar {
        let _ = agi.istek(
            "POST",
            &format!("{b}/api/onay/kapat"),
            &[("x-taa-imza", &imza)],
            Some(&g),
        );
    }
}

/// Bulut okumasını yapar ve bellekteki kaydı günceller (arka planda ya da testte eşzamanlı çağrılır).
fn sorgula_ve_guncelle(
    dizin: PathBuf,
    agi: Arc<dyn OnayAgi>,
    harita: OnayHaritasi,
    t: String,
    tabanlar: Vec<String>,
    ts: u64,
    sil: u64,
) {
    let sonuc = bulut_oku(&dizin, agi.as_ref(), &t, &tabanlar, ts);
    let baglanti = match &sonuc {
        Bulut::Ulasilamadi(e) => Some(e.clone()),
        _ => None,
    };
    let mut kapat = false;
    let guncel = match sonuc {
        Bulut::Onaylandi { e, iv, c } => {
            match ozel_anahtar(&dizin)
                .and_then(|k| onay_coz(&k, &t, &e, &iv, &c))
                .and_then(|d| onay_bilgisi(&d))
                .and_then(|b| {
                    if onay_zamani_gecerli(b.zaman, sil) {
                        Ok(b)
                    } else {
                        Err(
                            "onay zamanı karekodun süresi dışında (bulut ya da bilgisayar saati?)"
                                .into(),
                        )
                    }
                }) {
                Ok(b) => {
                    kapat = true;
                    Ok(Some(b))
                }
                Err(e) => Err(e),
            }
        }
        _ => Ok(None),
    };
    if let Ok(mut h) = harita.lock() {
        if let Some(k) = h.get_mut(&t) {
            k.sorguda = false;
            k.baglanti = baglanti;
            match guncel {
                Ok(Some(b)) => k.bilgi = Some(b),
                Ok(None) => {}
                Err(e) => {
                    // Ad/telefon günlüğe yazılmaz; yalnız olay.
                    eprintln!("Teslim onayı çözülemedi ({}): {e}", k.no);
                    k.hata = Some(e);
                }
            }
        }
    }
    if kapat {
        bulut_kapat(&dizin, agi.as_ref(), &t, &tabanlar, ts);
    }
}

fn zaman_metni(epoch: u64) -> String {
    let t = tarih_iso(epoch);
    format!(
        "{}.{}.{} {}",
        &t[8..10],
        &t[5..7],
        &t[0..4],
        &saat_iso(epoch)[..5]
    )
}

/// Karekoddaki telefon fişteki telefonla aynı mı? "ayni" | "farkli" | "yok" (fişte geçerli telefon yok).
/// Numara yanıtta açılmaz; karşılaştırma normalize "+<rakamlar>" biçiminde yapılır.
pub(crate) fn fis_telefonu_karsilastir(d: &Defter, no: &str, qr_tel: &str) -> &'static str {
    let fis_tel = match d.bul(no) {
        Ok(Some(f)) => tel_normal(&f.telefon).unwrap_or_default(),
        _ => String::new(),
    };
    if fis_tel.is_empty() || qr_tel.is_empty() {
        "yok"
    } else if fis_tel == qr_tel {
        "ayni"
    } else {
        "farkli"
    }
}

/// Ekrana giden görünüm (teslimi yapan kasiyer kimliği kontrol eder): tam telefon yok, yalnız maskeli; imza
/// görüntüsü gönderilmez, yalnız geldi mi.
fn bilgi_json(b: &OnayBilgisi) -> serde_json::Value {
    json!({
        "ad": b.ad,
        "tel_maskeli": tel_maskele(&b.tel),
        "cihaz": cihaz_ozeti(&b.ua),
        "zaman": b.zaman,
        "zaman_metni": zaman_metni(b.zaman),
        "imzali": b.imza.is_some(),
    })
}

/// Süresi geçen karekodları bellekten siler ve buluttaki kayıtlarını da kapattırır (arka planda; sonuç
/// önemsenmez). Böylece Vercel Blob'da (kendi silme zamanı yok) şifreli kayıt, sunucu açıksa ve internete
/// ulaşıyorsa en geç bir sonraki taramada (dakikada bir, bkz. `OnayTemizleyici`) silinir.
fn sureli_temizle(
    dizin: &Path,
    agi: &Arc<dyn OnayAgi>,
    h: &mut BTreeMap<String, OnayKaydi>,
    simdi: u64,
    arka_plan: bool,
) {
    let mut giden: Vec<(String, Vec<String>)> = Vec::new();
    h.retain(|t, k| {
        if k.sil > simdi {
            return true;
        }
        if k.bilgi.is_none() {
            // Onaylanmış olanlar okunurken zaten kapatıldı; bekleyen/hatalı kayıtlar burada kapatılır.
            giden.push((t.clone(), k.tabanlar.clone()));
        }
        false
    });
    if giden.is_empty() {
        return;
    }
    let (dizin, agi) = (dizin.to_path_buf(), agi.clone());
    let is = move || {
        for (t, tabanlar) in giden {
            bulut_kapat(&dizin, agi.as_ref(), &t, &tabanlar, simdi);
        }
    };
    if arka_plan {
        std::thread::spawn(is);
    } else {
        is();
    }
}

/// Dakikalık tarayıcı (main.rs): istek gelmese de süresi geçen karekodların bulut kaydını kapattırır.
pub struct OnayTemizleyici {
    dizin: PathBuf,
    agi: Arc<dyn OnayAgi>,
    harita: OnayHaritasi,
}

impl OnayTemizleyici {
    pub fn calistir(&self) {
        let simdi = crate::simdi();
        if let Ok(mut h) = self.harita.lock() {
            sureli_temizle(&self.dizin, &self.agi, &mut h, simdi, true);
        }
    }
}

impl Uygulama {
    fn onay_temizle(&self, h: &mut BTreeMap<String, OnayKaydi>) {
        sureli_temizle(
            &self.dizin,
            &self.onay_agi,
            h,
            self.simdi(),
            self.onay_arka_plan,
        );
    }

    /// main.rs'deki dakikalık temizlik için.
    pub fn onay_temizleyici(&self) -> OnayTemizleyici {
        OnayTemizleyici {
            dizin: self.dizin.clone(),
            agi: self.onay_agi.clone(),
            harita: self.onaylar.clone(),
        }
    }

    /// POST /api/fis/<no>/teslim-qr {fisli?}: belirteç üretir, buluta "aç" gönderir, karekodu döndürür.
    /// `fisli: false` ise karekod sayfası müşteriden imza ister (aç gövdesinde "i": 1); gövde yoksa fişli (i = 0).
    pub(crate) fn teslim_qr_ac(&self, d: &Defter, no: &str, c: &cihaz::Cihaz, i: &Istek) -> Yanit {
        let istek: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let imza_gerekli = istek["fisli"].as_bool() == Some(false);
        let f = match d.bul(no) {
            Ok(Some(f)) => f,
            Ok(None) => return Yanit::hata(404, "fiş yok"),
            Err(e) => return Yanit::hata(500, &e),
        };
        if f.durum != Durum::Hazir {
            return Yanit::hata(409, "yalnız hazır fiş teslim edilir");
        }
        if f.teslim_kaydi.is_some() {
            return Yanit::hata(409, "bu fişin teslim kaydı zaten var");
        }
        let Some(ayar) = musteri_takip::Ayar::dizinden(&self.dizin) else {
            return Yanit::json(
                409,
                json!({
                    "hata": "qr_kapali",
                    "ileti": "Karekodla onay için müşteri takip (bulut) ayarı gerekli. Ekranda imza kullanın.",
                }),
            );
        };
        let sube = match self.sube_kodu() {
            Ok(s) if sube_kodu_gecerli(&s) => s,
            Ok(_) => {
                return Yanit::hata(
                    409,
                    "şube kodu karekod için geçersiz (yalnız A-Z, 0-9, _ ve -; en çok 16). Ekranda imza kullanın.",
                )
            }
            Err(e) => return Yanit::hata(409, &e),
        };
        let simdi = self.simdi();
        {
            let Ok(mut h) = self.onaylar.lock() else {
                return Yanit::hata(500, "kilit hatası");
            };
            self.onay_temizle(&mut h);
            if h.values().filter(|k| k.no == no).count() >= FIS_BASINA_EN_COK {
                return Yanit::hata(
                    429,
                    "bu fiş için çok fazla karekod açıldı; birkaç dakika sonra yeniden deneyin",
                );
            }
        }
        let t = match yeni_belirtec() {
            Ok(t) => t,
            Err(e) => return Yanit::hata(500, &e),
        };
        let pk = match genel_anahtar(&self.dizin) {
            Ok(p) => p,
            Err(e) => return Yanit::hata(500, &e),
        };
        let n: u32 = f
            .kalemler
            .iter()
            .map(|k| k.adet.unwrap_or(1).max(1))
            .sum::<u32>()
            .clamp(1, 50);
        let sil = simdi + ONAY_OMRU_SN;
        let govde = json!({
            "v": 1, "tur": "onay_ac", "t": t, "ts": simdi, "sil": sil,
            "s": sube, "n": n, "pk": pk, "i": u8::from(imza_gerekli),
        });
        let (g, imza) = match musteri_takip::imzala(&self.dizin, &govde) {
            Ok(x) => x,
            Err(e) => return Yanit::hata(500, &e),
        };
        let mut tabanlar = vec![ayar.sayfa.clone()];
        if !ayar.yedek_sayfa.is_empty() {
            tabanlar.push(ayar.yedek_sayfa.clone());
        }
        // İki buluta paralel (tek iş parçacıklı sunucu en çok bir zaman aşımı kadar bekler).
        let agi = self.onay_agi.as_ref();
        let sonuc: Vec<Result<(u16, String), String>> = std::thread::scope(|k| {
            let isler: Vec<_> = tabanlar
                .iter()
                .map(|b| {
                    let (g, imza) = (&g, &imza);
                    k.spawn(move || {
                        agi.istek(
                            "POST",
                            &format!("{b}/api/onay/ac"),
                            &[("x-taa-imza", imza)],
                            Some(g),
                        )
                    })
                })
                .collect();
            isler
                .into_iter()
                .map(|i| i.join().unwrap_or_else(|_| Err("iş parçacığı".into())))
                .collect()
        });
        let acik: Vec<String> = tabanlar
            .iter()
            .zip(&sonuc)
            .filter(|(_, s)| matches!(s, Ok((200..=299, _))))
            .map(|(b, _)| b.clone())
            .collect();
        let saat_sorunu = sonuc
            .iter()
            .any(|s| matches!(s, Ok((k, g)) if saat_reddi(*k, g)));
        // Karekod birincil adresi gösterir; birincile ulaşılamadıysa yedeği.
        let Some(taban) = acik.first().cloned() else {
            if saat_sorunu {
                return Yanit::json(
                    502,
                    json!({
                        "hata": "saat_farki",
                        "ileti": "Karekod hazırlanamadı: bu bilgisayarın saati yanlış görünüyor (bulut isteği reddetti). Windows saatini ve saat dilimini denetleyin (Ayarlar > Saat ve dil > Saati otomatik ayarla). Bu arada ekranda imza kullanın.",
                    }),
                );
            }
            return Yanit::json(
                502,
                json!({
                    "hata": "bulut_yok",
                    "ileti": "Karekod hazırlanamadı (internet bağlantısı?). Telefonu olmayan müşteri için ekranda imza kullanın.",
                }),
            );
        };
        let url = format!("{taban}/o/{t}");
        let svg = match karekod_svg(&url) {
            Ok(s) => s,
            Err(e) => return Yanit::hata(500, &e),
        };
        // Okumada önce karekoddaki bulut, sonra diğeri (müşterinin onayı karekoddakine yazılır).
        let mut sira = acik.clone();
        for b in &tabanlar {
            if !sira.contains(b) {
                sira.push(b.clone());
            }
        }
        if let Ok(mut h) = self.onaylar.lock() {
            h.insert(
                t.clone(),
                OnayKaydi {
                    no: no.to_string(),
                    cihaz: c.id.clone(),
                    imza_gerekli,
                    sil,
                    tabanlar: sira,
                    bilgi: None,
                    hata: None,
                    baglanti: None,
                    son_sorgu: 0,
                    sorguda: false,
                },
            );
        }
        Yanit::json(
            200,
            json!({ "t": t, "url": url, "qr_svg": svg, "sil": sil, "kalan_sn": ONAY_OMRU_SN, "imza_gerekli": imza_gerekli }),
        )
    }

    /// GET /api/fis/<no>/teslim-qr/<t>: onay durumu. Bulut arka planda okunur; yanıt hemen döner.
    /// Onaylandıysa karekoddaki telefon fişteki telefonla karşılaştırılır (numara açılmadan): "fis_telefonu":
    /// "ayni" | "farkli" | "yok" (fişte geçerli telefon yok). "farkli" ise teslimde ilişki (yakını/diğer) zorunludur.
    pub(crate) fn teslim_qr_durum(&self, d: &Defter, no: &str, t: &str, c: &cihaz::Cihaz) -> Yanit {
        if !belirtec_gecerli(t) {
            return Yanit::hata(404, "karekod bulunamadı");
        }
        let simdi = self.simdi();
        let is = {
            let Ok(mut h) = self.onaylar.lock() else {
                return Yanit::hata(500, "kilit hatası");
            };
            match h.get(t) {
                Some(k) if k.no == no && k.cihaz == c.id => {}
                _ => return Yanit::hata(404, "karekod bulunamadı"),
            }
            if h.get(t).is_some_and(|k| k.sil <= simdi) {
                // Süresi geçti: bellekten silinir, bulut kaydı da kapattırılır.
                self.onay_temizle(&mut h);
                return Yanit::json(
                    410,
                    json!({ "durum": "suresi_doldu", "ileti": "Karekodun süresi doldu; yeni karekod oluşturun." }),
                );
            }
            let Some(k) = h.get_mut(t) else {
                return Yanit::hata(404, "karekod bulunamadı");
            };
            if k.bilgi.is_some() || k.hata.is_some() || k.sorguda || simdi < k.son_sorgu + 1 {
                None
            } else {
                k.sorguda = true;
                k.son_sorgu = simdi;
                Some((k.tabanlar.clone(), k.sil))
            }
        };
        if let Some((tabanlar, sil)) = is {
            let (dizin, agi, harita, t2) = (
                self.dizin.clone(),
                self.onay_agi.clone(),
                self.onaylar.clone(),
                t.to_string(),
            );
            if self.onay_arka_plan {
                std::thread::spawn(move || {
                    sorgula_ve_guncelle(dizin, agi, harita, t2, tabanlar, simdi, sil)
                });
            } else {
                sorgula_ve_guncelle(dizin, agi, harita, t2, tabanlar, simdi, sil);
            }
        }
        let (bilgi, hata, baglanti, kalan, imza_gerekli) = {
            let Ok(h) = self.onaylar.lock() else {
                return Yanit::hata(500, "kilit hatası");
            };
            let Some(k) = h.get(t) else {
                return Yanit::hata(404, "karekod bulunamadı");
            };
            (
                k.bilgi.clone(),
                k.hata.clone(),
                k.baglanti.clone(),
                k.sil.saturating_sub(simdi),
                k.imza_gerekli,
            )
        };
        if let Some(e) = hata {
            return Yanit::json(
                422,
                json!({ "durum": "hatali", "ileti": format!("Onay doğrulanamadı: {e}. Yeni karekod oluşturun."), "kalan_sn": kalan }),
            );
        }
        match bilgi {
            Some(b) => {
                let mut v = bilgi_json(&b);
                v["durum"] = json!("onaylandi");
                v["kalan_sn"] = json!(kalan);
                v["fis_telefonu"] = json!(fis_telefonu_karsilastir(d, no, &b.tel));
                v["imza_gerekli"] = json!(imza_gerekli);
                Yanit::json(200, v)
            }
            None => Yanit::json(
                200,
                json!({
                    "durum": "bekliyor",
                    "kalan_sn": kalan,
                    "imza_gerekli": imza_gerekli,
                    "baglanti_sorunu": baglanti.is_some(),
                    "saat_sorunu": baglanti.as_deref() == Some(SAAT_SORUNU),
                }),
            ),
        }
    }

    /// teslim-al için: belirteç bu fişe ve cihaza ait, süresi geçmemiş ve onaylanmış olmalı.
    pub(crate) fn qr_onayi(
        &self,
        no: &str,
        t: &str,
        c: &cihaz::Cihaz,
    ) -> Result<OnayBilgisi, Yanit> {
        let yok = || {
            Yanit::json(
                422,
                json!({ "hata": "onay_yok", "ileti": "Karekod onayı bulunamadı ya da süresi doldu; yeni karekod oluşturun." }),
            )
        };
        if !belirtec_gecerli(t) {
            return Err(yok());
        }
        let simdi = self.simdi();
        let h = self
            .onaylar
            .lock()
            .map_err(|_| Yanit::hata(500, "kilit hatası"))?;
        match h.get(t) {
            Some(k) if k.no == no && k.cihaz == c.id && k.sil > simdi => match &k.bilgi {
                Some(b) => Ok(b.clone()),
                None => Err(Yanit::json(
                    409,
                    json!({ "hata": "onay_bekliyor", "ileti": "Müşteri henüz karekodla onaylamadı." }),
                )),
            },
            _ => Err(yok()),
        }
    }

    /// Teslim tamamlandı: belirteç ve çözülen bilgi bellekten silinir (tek kullanım).
    pub(crate) fn qr_onayi_tuket(&self, t: &str) {
        if let Ok(mut h) = self.onaylar.lock() {
            h.remove(t);
        }
    }
}

#[cfg(test)]
pub(crate) mod testler {
    use super::*;
    use aes_gcm::aead::{Aead, KeyInit, Payload};

    /// Tarayıcı/bulut tarafının şifrelemesi (sözleşme; testte Rust ile yapılır).
    pub(crate) fn sozlesme_sifrele(
        pk_hex: &str,
        t: &str,
        duz: &[u8],
        gecici: [u8; 32],
        iv: [u8; 12],
    ) -> (String, String, String) {
        let pk: [u8; 32] = hex_coz(pk_hex).unwrap().try_into().unwrap();
        let es = x25519_dalek::StaticSecret::from(gecici);
        let e = x25519_dalek::PublicKey::from(&es);
        let p = es.diffie_hellman(&x25519_dalek::PublicKey::from(pk));
        let k = anahtar_tur(p.as_bytes(), e.as_bytes(), &pk).unwrap();
        let c = aes_gcm::Aes256Gcm::new_from_slice(&k)
            .unwrap()
            .encrypt(
                aes_gcm::Nonce::from_slice(&iv),
                Payload {
                    msg: duz,
                    aad: t.as_bytes(),
                },
            )
            .unwrap();
        (hex(e.as_bytes()), hex(&iv), b64(&c))
    }

    pub(crate) fn b64(b: &[u8]) -> String {
        const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut s = String::new();
        for p in b.chunks(3) {
            let n = (p[0] as u32) << 16
                | (*p.get(1).unwrap_or(&0) as u32) << 8
                | *p.get(2).unwrap_or(&0) as u32;
            for i in 0..4 {
                s.push(if i <= p.len() {
                    A[(n >> (18 - 6 * i) & 63) as usize] as char
                } else {
                    '='
                });
            }
        }
        s
    }

    #[test]
    fn sifrele_coz_gidis_donus() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let ozel = ozel_anahtar(&d).unwrap();
        let pk = genel_anahtar(&d).unwrap();
        assert_eq!(pk.len(), 64);
        // aynı dosyadan aynı anahtar
        assert_eq!(genel_anahtar(&d).unwrap(), pk);
        let m = std::fs::read_to_string(d.join("musteri-takip").join(ANAHTAR_DOSYASI)).unwrap();
        assert!(m.starts_with("x25519-ozel:"));
        let t = yeni_belirtec().unwrap();
        assert!(belirtec_gecerli(&t), "{t}");
        let duz = br#"{"ad":"Deneme Kisi","tel":"+905320004567","ua":"Deneme/1.0","ip":"203.0.113.9","zaman":1795000000,"dil":"tr"}"#;
        let (e, iv, c) = sozlesme_sifrele(&pk, &t, duz, [7u8; 32], [9u8; 12]);
        assert_eq!(onay_coz(&ozel, &t, &e, &iv, &c).unwrap(), duz.to_vec());
        // başka belirteçle (aad) çözülmez
        let t2 = yeni_belirtec().unwrap();
        assert!(onay_coz(&ozel, &t2, &e, &iv, &c).is_err());
        // başka sunucunun anahtarıyla çözülmez
        let d2 = taa_dosya::gecici_klasor().unwrap();
        assert!(onay_coz(&ozel_anahtar(&d2).unwrap(), &t, &e, &iv, &c).is_err());
        // sıfır (düşük mertebeli) geçici anahtar reddedilir
        assert!(onay_coz(&ozel, &t, &"00".repeat(32), &iv, &c).is_err());
        let b = onay_bilgisi(duz).unwrap();
        assert_eq!(b.tel, "+905320004567");
        assert_eq!(b.zaman, 1_795_000_000);
        // geçersiz içerik
        assert!(onay_bilgisi(br#"{"ad":"X","tel":"+905320004567","zaman":1}"#).is_err());
        assert!(onay_bilgisi(br#"{"ad":"Deneme <b>","tel":"+905320004567","zaman":1}"#).is_err());
        assert!(onay_bilgisi(br#"{"ad":"Deneme Kisi","tel":"123","zaman":1}"#).is_err());
        assert!(onay_bilgisi(br#"{"ad":"Deneme Kisi","tel":"+905320004567"}"#).is_err());
    }

    /// Sabit test vektörü: node:crypto ile üretildi (X25519, HKDF-SHA256, AES-256-GCM; üretim betiği aşağıdaki
    /// yorumda). Sunucu özel anahtarı = 0x11 * 32, geçici özel = 0x22 * 32, iv = 0x33 * 12.
    #[test]
    fn node_test_vektoru() {
        let ozel = x25519_dalek::StaticSecret::from([0x11u8; 32]);
        assert_eq!(
            hex(x25519_dalek::PublicKey::from(&ozel).as_bytes()),
            VEKTOR_PK
        );
        let duz = onay_coz(&ozel, VEKTOR_T, VEKTOR_E, VEKTOR_IV, VEKTOR_C).unwrap();
        assert_eq!(String::from_utf8(duz).unwrap(), VEKTOR_DUZ);
        // Rust şifrelemesi aynı vektörü üretir (iki taraf birebir aynı sözleşme).
        let (e, iv, c) = sozlesme_sifrele(
            VEKTOR_PK,
            VEKTOR_T,
            VEKTOR_DUZ.as_bytes(),
            [0x22u8; 32],
            [0x33u8; 12],
        );
        assert_eq!(
            (e.as_str(), iv.as_str(), c.as_str()),
            (VEKTOR_E, VEKTOR_IV, VEKTOR_C)
        );
    }

    #[test]
    fn sube_kodu_ve_onay_zamani_kurallari() {
        for k in ["MRK01", "S_1-a", "ABCDEFGHIJKLMNOP"] {
            assert!(sube_kodu_gecerli(k), "{k}");
        }
        for k in ["", "MRK 01", "ŞUBE1", "ABCDEFGHIJKLMNOPQ", "a/b"] {
            assert!(!sube_kodu_gecerli(k), "{k}");
        }
        let sil = 1_795_000_900;
        assert!(onay_zamani_gecerli(1_795_000_000, sil));
        assert!(onay_zamani_gecerli(1_795_000_000 - 600, sil));
        assert!(onay_zamani_gecerli(sil + 600, sil));
        assert!(!onay_zamani_gecerli(1_795_000_000 - 601, sil));
        assert!(!onay_zamani_gecerli(sil + 601, sil));
        assert!(saat_reddi(401, r#"{"hata":"zaman geçersiz"}"#));
        assert!(saat_reddi(400, r#"{"hata":"silinme zamanı geçersiz"}"#));
        assert!(!saat_reddi(401, r#"{"hata":"imza geçersiz"}"#));
    }

    /// Bulutun GERÇEK kodundan (ortak/onay.mjs onayIsle → sifrele, WebCrypto X25519) üretilmiş sabit vektörler:
    /// sahte bir depo ile POST /o/<t> çalıştırıldı (simdi = 1795000000, ua "Mozilla/5.0 (Linux; Android 14;
    /// SM-DENEME) Chrome/120"), kayda yazılan {e, iv, c} buraya kondu. Sunucu özel anahtarı = 0x11 * 32 (VEKTOR_PK).
    /// Hintçe (virama), Farsça (ZWNJ) ve harekeli Arapça sahte adlar: bulutta "teşekkür" gören müşteri kasada da
    /// geçmeli (denetçi bulgusu: eskiden "onaydaki ad geçersiz" dönüyordu). IPv6 adresi de geçer.
    #[test]
    fn bulut_webcrypto_vektorleri_ve_cok_dilli_adlar() {
        let ozel = x25519_dalek::StaticSecret::from([0x11u8; 32]);
        for (t, e, iv, c, ad, tel, ip, dil) in BULUT_VEKTORLERI {
            let duz = onay_coz(&ozel, t, e, iv, c).unwrap_or_else(|h| panic!("{t}: {h}"));
            let b = onay_bilgisi(&duz).unwrap_or_else(|h| panic!("{t}: {h}"));
            assert_eq!(
                (b.ad.as_str(), b.tel.as_str(), b.ip.as_str(), b.dil.as_str()),
                (*ad, *tel, *ip, *dil),
                "{t}"
            );
            assert_eq!(b.zaman, 1_795_000_000);
            assert_eq!(
                b.ua,
                "Mozilla/5.0 (Linux; Android 14; SM-DENEME) Chrome/120"
            );
            // açılış..silinme aralığında (sil = simdi + 600)
            assert!(onay_zamani_gecerli(b.zaman, 1_795_000_600));
        }
        // aynı adlar düz JSON'dan da geçer; yön işaretli ad geçmez
        for ad in ["राहुल शर्मा", "علی\u{200C}رضا محمدی", "مُحَمَّد عَلِي"]
        {
            let j = json!({ "ad": ad, "tel": "+905320004567", "ua": "", "ip": "", "zaman": 1, "dil": "tr" });
            assert!(onay_bilgisi(j.to_string().as_bytes()).is_ok(), "{ad}");
        }
        for ad in [
            "Deneme \u{202E}Kişi",
            "Deneme\u{200B} Kişi",
            "\u{200C}\u{200C}",
        ] {
            let j = json!({ "ad": ad, "tel": "+905320004567", "ua": "", "ip": "", "zaman": 1, "dil": "tr" });
            assert!(onay_bilgisi(j.to_string().as_bytes()).is_err(), "{ad:?}");
        }
    }

    /// (t, e, iv, c, beklenen ad (NFC), tel, ip, dil). Üretim betiği: bulut/musteri-takip/test/dene6.mjs ile aynı
    /// depo düzeni; sahte veridir.
    #[allow(clippy::type_complexity)]
    const BULUT_VEKTORLERI: &[(&str, &str, &str, &str, &str, &str, &str, &str)] = &[
        (
            "abcdefghijkmnpqrst23",
            "5f7754d521e6526f22bd51f69fa8dcd5da46d19ca859662ba1ca4ed56f7b8353",
            "0d40f962c0a0c2cf9d909229",
            "xetahKry+DRXYCBv9SZX+ZkGEGwHWDZRn0EI0NURuqDOwKmLT7KhIXCFgB7DU6EVXeSPVyoANTfsNRKfyVNaxTDDk5LzdDLmhPefF/SHBu4B3OMaGiC9KRpR/JkKw35hk/4vCgtZGU7BdSA+hdYoFDIVCOpak9Ie43JxxwXoxdB3+E8LYe4g23H/vPWyT4AiwIRx0MQZqdgX51DgkWrYvQMPSAjHHfRox+fBRIrmXM/pTJn9c318gzOABEM=",
            "\u{930}\u{93e}\u{939}\u{941}\u{932} \u{936}\u{930}\u{94d}\u{92e}\u{93e}",
            "+905320004567",
            "203.0.113.9",
            "hi",
        ),
        (
            "bcdefghijkmnpqrst234",
            "c94ed627ad78ec5a6dc856df1a14fb024ba86338d54b928afbbac6380c2a8b28",
            "2215e0b023008d3295d4f326",
            "zABBvyG1u+rN0usI3noAl8ldCihcNBRcTgm2oN6HwqpWOaENNXWhwQ3/7Eh7pOFB9WMpO113nIb0CpTv2bxqLAot6UK6ma813Z8ieVfeBXUDhWCLZPxhzVg7bB1s2HB0cb7xp8H6JBeZFFPXyeka9VND24tVWKLffITs/TCiFMW8hyT8M9w9D0XFEjk5sBLPI8snrMiDJRhvDlS9LC3CHSFIhWIOnszB1y6LXUaCeLSsto53CK3I2Q==",
            "\u{639}\u{644}\u{6cc}\u{200c}\u{631}\u{636}\u{627} \u{645}\u{62d}\u{645}\u{62f}\u{6cc}",
            "+989120001234",
            "2001:db8::17",
            "fa",
        ),
        (
            "cdefghijkmnpqrst2345",
            "edfceae9cb11be95c36a6ae7d4fe15b21787ce56821fe2342fdb0ac9f5d7c40f",
            "38c2d7e36cf01a5605114787",
            "gC3lnFNxDDsSzyQVVChhHNF0DIayYnU3OV4rybBoXYFZ4a3gz4PIY4O5hZgjY578pU1gS1slr7zh0TvkkQhGm0CnRZLOkxJafpjGWtr1v6h4Jf4N2Qss9yOlnsU+ZjLb3m3T2C3tva1cQ2X182ZLwzn3fxnI784K4cGp6BoIn5Vij501AHDLQgG4CoFf242ncVUdxqAvOdJLVoxj7lmDSx3gBab16fe5uDW29ZXjHukJ7TyKx3MI9qYz",
            "\u{645}\u{64f}\u{62d}\u{64e}\u{645}\u{64e}\u{651}\u{62f} \u{639}\u{64e}\u{644}\u{650}\u{64a}",
            "+971500001234",
            "198.51.100.23",
            "ar",
        ),
    ];

    // Vektörü üreten node:crypto betiği (node 22; çıktısı aşağıdaki sabitler):
    // import crypto from 'node:crypto';
    // const pkcs8 = (b) => crypto.createPrivateKey({ key: Buffer.concat([Buffer.from('302e020100300506032b656e04220420', 'hex'), b]), format: 'der', type: 'pkcs8' });
    // const raw = (k) => Buffer.from(crypto.createPublicKey(k).export({ format: 'jwk' }).x, 'base64url');
    // const sunucu = pkcs8(Buffer.alloc(32, 0x11)), gecici = pkcs8(Buffer.alloc(32, 0x22));
    // const pk = raw(sunucu), e = raw(gecici);
    // const t = 'abcdefghijkmnpqrst23';
    // const duz = JSON.stringify({ ad: 'Deneme Kişi', tel: '+905320004567', ua: 'Mozilla/5.0 (Linux; Android 14; SM-DENEME) Chrome/120', ip: '203.0.113.9', zaman: 1795000000, dil: 'tr' });
    // const ortak = crypto.diffieHellman({ privateKey: gecici, publicKey: crypto.createPublicKey(sunucu) });
    // const anahtar = Buffer.from(crypto.hkdfSync('sha256', ortak, Buffer.concat([e, pk]), Buffer.from('taa-teslim-onay-v1'), 32));
    // const iv = Buffer.alloc(12, 0x33);
    // const s = crypto.createCipheriv('aes-256-gcm', anahtar, iv); s.setAAD(Buffer.from(t, 'utf8'));
    // const c = Buffer.concat([s.update(duz, 'utf8'), s.final(), s.getAuthTag()]);
    // console.log(`    const VEKTOR_PK: &str = "${pk.toString('hex')}";
    //     const VEKTOR_T: &str = "${t}";
    //     const VEKTOR_E: &str = "${e.toString('hex')}";
    //     const VEKTOR_IV: &str = "${iv.toString('hex')}";
    //     const VEKTOR_C: &str = "${c.toString('base64')}";
    //     const VEKTOR_DUZ: &str = ${JSON.stringify(duz).replace(/[^\x00-\x7f]/g, ch => '\\u{' + ch.codePointAt(0).toString(16) + '}')};`);
    const VEKTOR_PK: &str = "7b4e909bbe7ffe44c465a220037d608ee35897d31ef972f07f74892cb0f73f13";
    const VEKTOR_T: &str = "abcdefghijkmnpqrst23";
    const VEKTOR_E: &str = "0faa684ed28867b97f4a6a2dee5df8ce974e76b7018e3f22a1c4cf2678570f20";
    const VEKTOR_IV: &str = "333333333333333333333333";
    const VEKTOR_C: &str = "J9Q+Y3ArQEGjub6DaiTlzJxqyqy+I/a2vTa3ASndk3a9Gn7tL7NmjJqpGfKyMUcQJpbIYWyf3dKkQzhQ454X68k7Ldg7uO+EphUNLSIC4AjvN1Z5EtegsTOqlsrOWMgLKl/FMFc5XUsn3MG2SjGfIhzJh36uvhqNd3I7TqmuqGH4wPF/qKsaMO9KbfPI2KshblKv+lwkJ88KoBuMGMWKIUGeN4NLmxiczw==";
    const VEKTOR_DUZ: &str = "{\"ad\":\"Deneme Ki\u{15f}i\",\"tel\":\"+905320004567\",\"ua\":\"Mozilla/5.0 (Linux; Android 14; SM-DENEME) Chrome/120\",\"ip\":\"203.0.113.9\",\"zaman\":1795000000,\"dil\":\"tr\"}";
}
