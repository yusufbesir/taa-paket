//! Müşteri ürün takibi (Yusuf 2026-11-26): müşteri fişteki karekodu okutup ürününün durumunu görür.
//!
//! Güvenlik ve KVKK:
//! - Mağaza sunucusu internete AÇILMAZ (gelen bağlantı yok). Durum değişince sunucu kaydı kendisi dışarı
//!   iter (Cloudflare ve Vercel'deki takip sayfalarına; ikisi birbirinin yedeği).
//! - Dışarı giden kayıt yalnız: takip kodu, durum, şube kodu, teslim günü/saati, kesim günü, fiş özeti, zaman. Müşteri adı, telefonu,
//!   ölçü, fiyat, fiş numarası, serbest metin GİTMEZ.
//!   Fiş özeti yalnız sabit anahtarlarla gider (ürün türü, katalog işlem kodu, varyant, kesim/teslim günü).
//! - Takip kodu fiş numarasından bu kuruluma özel gizli anahtarla (HMAC-SHA256) türetilir; tahmin edilemez,
//!   saklanması gerekmez. Anahtar `musteri-takip/anahtar` dosyasındadır, dışarı çıkmaz.
//! - Kayıtlar ed25519 ile imzalanır; bulut tarafı yalnız GENEL anahtarı bilir (sızsa bile kayıt yazılamaz).
//!   Özel anahtar `musteri-takip/imza.ozel` dosyasındadır, dışarı çıkmaz.
//! - Bulut tarafı teslimden 7 gün (iptalde 3 gün) sonra kaydı siler (`sil` alanı).
//!
//! Ayar: `musteri-takip.json` (yoksa özellik kapalı):
//! `{"acik":true,"sayfa":"https://taa-takip.example.workers.dev","yedek_sayfa":"https://taa-takip.vercel.app",
//!   "hedefler":["https://taa-takip.example.workers.dev/api/yaz","https://taa-takip.vercel.app/api/yaz"]}`
//! Gönderim arka planda (main.rs) `gonder_bekleyenler` ile yapılır; HTTP için yeni crate yok, `curl` kullanılır.
//! Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use taa_fis::{Durum, Fis};

/// Fişe basılan açıklama cümlesi (Yusuf 2026-11-26: ad/telefon açıklaması YOK).
pub const FIS_CUMLESI: &str = "Ürününüzün durumunu bu karekodu okutarak takip edebilirsiniz.";
const KLASOR: &str = "musteri-takip";
const GUN: u64 = 86_400;
/// Kod uzunluğu (base32, 5 bit/karakter): 16 karakter = 80 bit.
const KOD_UZUNLUGU: usize = 16;
pub(crate) const ALFABE: &[u8; 32] = b"abcdefghijkmnpqrstuvwxyz23456789"; // l, o, 0, 1 yok: okurken karışmasın

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Ayar {
    pub sayfa: String,
    pub yedek_sayfa: String,
    pub hedefler: Vec<String>,
}

impl Ayar {
    /// `musteri-takip.json` okunur; yoksa, bozuksa ya da `acik` değilse `None` (özellik kapalı).
    pub fn dizinden(dizin: &Path) -> Option<Ayar> {
        let m = std::fs::read_to_string(dizin.join("musteri-takip.json")).ok()?;
        let v: Value = serde_json::from_str(&m).ok()?;
        if v["acik"].as_bool() != Some(true) {
            return None;
        }
        let https = |s: &str| s.starts_with("https://") && !s.contains(char::is_whitespace);
        let sayfa = v["sayfa"]
            .as_str()
            .unwrap_or("")
            .trim_end_matches('/')
            .to_string();
        if !https(&sayfa) {
            return None;
        }
        let yedek_sayfa = v["yedek_sayfa"]
            .as_str()
            .unwrap_or("")
            .trim_end_matches('/')
            .to_string();
        let hedefler: Vec<String> = v["hedefler"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|h| h.as_str())
                    .filter(|h| https(h))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        Some(Ayar {
            sayfa,
            yedek_sayfa: if https(&yedek_sayfa) {
                yedek_sayfa
            } else {
                String::new()
            },
            hedefler,
        })
    }

    pub fn adres(&self, kod: &str) -> String {
        format!("{}/t/{kod}", self.sayfa)
    }

    pub fn yedek_adres(&self, kod: &str) -> Option<String> {
        (!self.yedek_sayfa.is_empty()).then(|| format!("{}/t/{kod}", self.yedek_sayfa))
    }
}

pub(crate) fn klasor(dizin: &Path) -> PathBuf {
    dizin.join(KLASOR)
}

pub(crate) fn rastgele(n: usize) -> Result<Vec<u8>, String> {
    let mut b = vec![0u8; n];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(b)
}

pub(crate) fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

pub(crate) fn hex_coz(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    if !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect()
}

/// Dosya yoksa yeni oluşturur (üzerine yazmaz); `uret` içeriği verir.
pub(crate) fn dosya_ya_da_olustur(
    yol: &Path,
    uret: impl FnOnce() -> Result<String, String>,
) -> Result<String, String> {
    if let Ok(m) = std::fs::read_to_string(yol) {
        return Ok(m);
    }
    if let Some(u) = yol.parent() {
        std::fs::create_dir_all(u).map_err(|e| format!("klasör açılamadı: {e}"))?;
    }
    let icerik = uret()?;
    use std::io::Write;
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(yol)
    {
        Ok(mut f) => {
            f.write_all(icerik.as_bytes())
                .map_err(|e| format!("yazılamadı: {e}"))?;
            Ok(icerik)
        }
        // Aynı anda başka iş parçacığı oluşturduysa onunkini kullan.
        Err(_) => std::fs::read_to_string(yol).map_err(|e| format!("okunamadı: {e}")),
    }
}

fn anahtar(dizin: &Path) -> Result<Vec<u8>, String> {
    let m = dosya_ya_da_olustur(&klasor(dizin).join("anahtar"), || Ok(hex(&rastgele(32)?)))?;
    hex_coz(&m)
        .filter(|b| b.len() == 32)
        .ok_or_else(|| "takip anahtarı bozuk".into())
}

pub(crate) fn hmac_sha256(anahtar: &[u8], veri: &[u8]) -> [u8; 32] {
    let mut k = [0u8; 64];
    if anahtar.len() > 64 {
        k[..32].copy_from_slice(&Sha256::digest(anahtar));
    } else {
        k[..anahtar.len()].copy_from_slice(anahtar);
    }
    let mut ic = Sha256::new();
    ic.update(k.map(|b| b ^ 0x36));
    ic.update(veri);
    let mut dis = Sha256::new();
    dis.update(k.map(|b| b ^ 0x5c));
    dis.update(ic.finalize());
    dis.finalize().into()
}

/// Fişin takip kodu: HMAC(anahtar, "takip:" + no), 16 karakter.
pub fn takip_kodu(dizin: &Path, fis_no: &str) -> Result<String, String> {
    Ok(kod_uret(&anahtar(dizin)?, fis_no))
}

/// Okutulan takip kodunun (müşteri nüshasındaki karekod) hangi fişe ait olduğunu bulur: anahtar bir kez okunur,
/// adayların kodu hesaplanıp karşılaştırılır. Kod geçersizse ya da hiçbir aday tutmazsa `None`.
/// Takip kodu gizli anahtarla üretildiği için tahmin edilemez: müşteri nüshasının elde olduğunun kanıtıdır.
pub(crate) fn takip_kodundan_fis<'a>(
    dizin: &Path,
    kod: &str,
    adaylar: impl IntoIterator<Item = &'a str>,
) -> Option<&'a str> {
    if !kod_gecerli(kod) {
        return None;
    }
    let a = anahtar(dizin).ok()?;
    adaylar
        .into_iter()
        .find(|no| crate::teslim_al::sabit_esit(kod_uret(&a, no).as_bytes(), kod.as_bytes()))
}

fn kod_uret(anahtar: &[u8], fis_no: &str) -> String {
    b32_ozet(anahtar, &format!("takip:{fis_no}"), KOD_UZUNLUGU)
}

/// HMAC(anahtar, veri) özetinin ilk `uzunluk` karakteri (32'lik alfabe, 5 bit/karakter).
fn b32_ozet(anahtar: &[u8], veri: &str, uzunluk: usize) -> String {
    let h = hmac_sha256(anahtar, veri.as_bytes());
    let mut s = String::with_capacity(uzunluk);
    let (mut tampon, mut bit) = (0u32, 0u32);
    for b in h {
        tampon = (tampon << 8) | b as u32;
        bit += 8;
        while bit >= 5 && s.len() < uzunluk {
            bit -= 5;
            s.push(ALFABE[((tampon >> bit) & 31) as usize] as char);
        }
    }
    s
}

/// Fiş karekodu (personel karekodu, `…/telefon#fis=<no>&k=<kanıt>`) kanıt uzunluğu: 12 karakter = 60 bit.
pub(crate) const FIS_KANIT_UZUNLUGU: usize = 12;

/// Fiş karekodunun gizli anahtarlı kanıt parçası: HMAC(anahtar, "fis:" + no), 12 karakter. Anahtar YERELDİR
/// (`musteri-takip/anahtar`, takip/bulut ayarı olmasa da üretilir); fiş no'sundan tahmin edilemez. Teslim sepetinde
/// "fişli" sayılmanın kanıtı budur: elle yazılan "#fis=<no>" (k'siz ya da yanlış k) fişsizdir.
pub(crate) fn fis_kaniti(dizin: &Path, fis_no: &str) -> Result<String, String> {
    Ok(b32_ozet(
        &anahtar(dizin)?,
        &format!("fis:{fis_no}"),
        FIS_KANIT_UZUNLUGU,
    ))
}

/// Okunan `k` bu fişin kanıtı mı (sabit zamanlı karşılaştırma)?
pub(crate) fn fis_kaniti_dogru(dizin: &Path, fis_no: &str, k: &str) -> bool {
    if k.len() != FIS_KANIT_UZUNLUGU || !k.bytes().all(|b| ALFABE.contains(&b)) {
        return false;
    }
    fis_kaniti(dizin, fis_no)
        .is_ok_and(|b| crate::teslim_al::sabit_esit(b.as_bytes(), k.as_bytes()))
}

pub fn kod_gecerli(k: &str) -> bool {
    k.len() == KOD_UZUNLUGU && k.bytes().all(|b| ALFABE.contains(&b))
}

pub(crate) fn imza_anahtari(dizin: &Path) -> Result<ed25519_dalek::SigningKey, String> {
    let m = dosya_ya_da_olustur(&klasor(dizin).join("imza.ozel"), || {
        Ok(format!("ed25519-ozel:{}", hex(&rastgele(32)?)))
    })?;
    let b: [u8; 32] = m
        .trim()
        .strip_prefix("ed25519-ozel:")
        .and_then(hex_coz)
        .and_then(|v| v.try_into().ok())
        .ok_or("imza anahtarı bozuk")?;
    Ok(ed25519_dalek::SigningKey::from_bytes(&b))
}

/// Bulut tarafına verilecek GENEL anahtar (64 onaltılık hane). Gizli değildir.
pub fn genel_anahtar(dizin: &Path) -> Result<String, String> {
    Ok(hex(imza_anahtari(dizin)?.verifying_key().as_bytes()))
}

fn durum_adi(d: Durum) -> &'static str {
    match d {
        Durum::Bekliyor => "alindi",
        Durum::Terzihanede => "terzide",
        Durum::Hazir => "hazir",
        Durum::Teslim => "teslim",
        Durum::Iptal => "iptal",
    }
}

/// Dışarı gidecek kayıt. Kişisel veri İÇERMEZ (testle denetlenir).
pub fn kayit(dizin: &Path, f: &Fis, simdi: u64) -> Result<Value, String> {
    let sil = match f.durum {
        Durum::Teslim => simdi + 7 * GUN,
        Durum::Iptal => simdi + 3 * GUN,
        // Teslim edilmeyen ürün en geç 120 gün sonra silinir (14 gün kuralı + pay).
        _ => simdi + 120 * GUN,
    };
    // Fişin sayfada (müşterinin dilinde) gösterilecek özeti (Yusuf 2026-11-26 03:42). Yalnız SABİT
    // anahtarlar gider: ürün türü, katalog işlem kodu, varyant. Serbest metin, not, ölçü, fiyat GİTMEZ;
    // böylece bulut sayfası her şeyi kendi sözlüğüyle çevirebilir ve kişisel veri sızmaz.
    let kalemler: Vec<Value> = f
        .kalemler
        .iter()
        .take(20)
        .map(|k| {
            let mut v = json!({ "u": urun_anahtari(&k.urun) });
            if let Some(kod) = k.islem_kodu.as_deref().filter(|c| islem_kodu_gecerli(c)) {
                v["i"] = json!(kod);
            }
            if let Some(var) = k.varyant.as_deref().and_then(varyant_anahtari) {
                v["v"] = json!(var);
            }
            // Renk yalnız sabit listedeki anahtar olarak, adet 2-20 ise gider (2026-11-26 04:31).
            if let Some(r) = k.renk.as_deref().filter(|r| taa_fis::renk_adi(r).is_some()) {
                v["r"] = json!(r);
            }
            if let Some(a) = k.adet.filter(|a| (2..=20).contains(a)) {
                v["a"] = json!(a);
            }
            v
        })
        .collect();
    let saat = if f.teslim_saati.len() == 5 && f.teslim_saati.as_bytes()[2] == b':' {
        f.teslim_saati.clone()
    } else {
        String::new()
    };
    let mut k = json!({
        "v": 2,
        "k": takip_kodu(dizin, &f.no)?,
        "d": durum_adi(f.durum),
        "s": f.sube,
        "t": f.teslim,
        "g": f.tarih,
        "h": saat,
        "u": kalemler,
        "ts": simdi,
        "sil": sil,
    });
    // Bağlı (birlikte teslim) fiş sayısı; fiş numaraları gitmez.
    if !f.bagli.is_empty() {
        k["b"] = json!(f.bagli.len().min(20));
    }
    // Kargo: yalnız "kargoya hazırlanıyor" bilgisi; firma/takip no Telegram kargo kaydından eklenir.
    // Ad, telefon, açık adres GİTMEZ.
    if f.kargo && !matches!(f.durum, Durum::Teslim | Durum::Iptal) {
        k["v"] = json!(3);
        k["kg"] = json!({ "d": "hazirlaniyor", "f": "diger", "n": "", "t": "", "y": "" });
    }
    Ok(k)
}

/// Ürün adını sabit bir anahtara çevirir; bilinmeyen ürün "diger" olur (serbest metin dışarı çıkmaz).
fn urun_anahtari(urun: &str) -> &'static str {
    let a: String = urun
        .trim()
        .chars()
        .map(|c| match c {
            'İ' | 'I' | 'ı' => 'i',
            'Ş' | 'ş' => 's',
            'Ğ' | 'ğ' => 'g',
            'Ü' | 'ü' => 'u',
            'Ö' | 'ö' => 'o',
            'Ç' | 'ç' => 'c',
            c => c.to_ascii_lowercase(),
        })
        .collect();
    const BILINEN: [(&str, &str); 16] = [
        ("pantolon", "pantolon"),
        ("ceket", "ceket"),
        ("gomlek", "gomlek"),
        ("yelek", "yelek"),
        ("takim", "takim"),
        ("takim elbise", "takim"),
        ("etek", "etek"),
        ("elbise", "elbise"),
        ("mont", "mont"),
        ("palto", "palto"),
        ("kaban", "palto"),
        ("bluz", "bluz"),
        ("kot", "kot"),
        ("jean", "kot"),
        ("tisort", "tisort"),
        ("trenckot", "palto"),
    ];
    BILINEN
        .iter()
        .find(|(k, _)| *k == a)
        .map(|(_, v)| *v)
        .unwrap_or("diger")
}

/// Katalog işlem kodu biçimi (ör. "pant-paca-kisalt"): yalnız küçük harf, rakam, tire.
fn islem_kodu_gecerli(k: &str) -> bool {
    !k.is_empty()
        && k.len() <= 40
        && k.bytes()
            .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
}

fn varyant_anahtari(v: &str) -> Option<&'static str> {
    match v.trim() {
        "Düz paça" => Some("duz"),
        "Duble paça" => Some("duble"),
        _ => None,
    }
}

/// Kaydı imzalar: (gövde, imza hex). Gövde tam bu baytlarla gönderilir, imza bu baytların üzerindedir.
pub fn imzala(dizin: &Path, k: &Value) -> Result<(String, String), String> {
    use ed25519_dalek::Signer;
    let govde = serde_json::to_string(k).map_err(|e| e.to_string())?;
    let imza = imza_anahtari(dizin)?.sign(govde.as_bytes());
    Ok((govde, hex(&imza.to_bytes())))
}

fn bekleyen_yolu(dizin: &Path) -> PathBuf {
    klasor(dizin).join("bekleyen.json")
}

/// `{kod: {"kayit": {...}, "gonderildi": {hedef: ts}}}`
fn bekleyen_oku(dizin: &Path) -> BTreeMap<String, Value> {
    std::fs::read_to_string(bekleyen_yolu(dizin))
        .ok()
        .and_then(|m| serde_json::from_str(&m).ok())
        .unwrap_or_default()
}

fn bekleyen_yaz(dizin: &Path, b: &BTreeMap<String, Value>) -> Result<(), String> {
    let yol = bekleyen_yolu(dizin);
    std::fs::create_dir_all(klasor(dizin)).map_err(|e| e.to_string())?;
    let gecici = yol.with_extension("json.yeni");
    std::fs::write(&gecici, serde_json::to_vec(b).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    std::fs::rename(&gecici, &yol).map_err(|e| e.to_string())
}

static KILIT: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Fişin güncel durumunu gönderim kuyruğuna yazar (özellik kapalıysa hiçbir şey yapmaz).
pub fn kuyruga_yaz(dizin: &Path, f: &Fis, simdi: u64) -> Result<(), String> {
    if Ayar::dizinden(dizin).is_none() {
        return Ok(());
    }
    let k = kayit(dizin, f, simdi)?;
    let kod = k["k"].as_str().unwrap_or_default().to_string();
    let _g = KILIT.lock().map_err(|_| "kilit hatası")?;
    let mut b = bekleyen_oku(dizin);
    b.insert(kod, json!({ "kayit": k, "gonderildi": {} }));
    bekleyen_yaz(dizin, &b)
}

/// HTTP gönderici (testte sahtesi kullanılır).
pub trait Gonderici {
    /// 2xx: Ok. Kalıcı ret (4xx): Err((true, ileti)). Geçici (ağ, 5xx): Err((false, ileti)).
    fn gonder(&self, url: &str, govde: &str, imza: &str) -> Result<(), (bool, String)>;
}

/// `curl` ile tek HTTP isteği (Windows 10+ ve Linux'ta hazır). Gövde varsa stdin'den verilir (komut satırında
/// görünmez). (HTTP kodu, yanıt gövdesi) döner; ağ hatasında kod 0 ve gövde curl'ün hata iletisidir.
/// Başlık değerleri yalnız imza/zaman gibi sabit karakterli değerlerdir (kişisel veri başlıkta gitmez).
pub(crate) fn curl_istek(
    metot: &str,
    url: &str,
    basliklar: &[(&str, &str)],
    govde: Option<&str>,
    sure_sn: u32,
) -> Result<(u16, String), String> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    let mut c = Command::new("curl");
    c.args(["-sS", "-o", "-", "-w", "\n%{http_code}", "--max-time"])
        .arg(sure_sn.to_string())
        .args(["-X", metot]);
    if govde.is_some() {
        c.args(["-H", "Content-Type: application/json"]);
    }
    for (a, d) in basliklar {
        c.args(["-H", &format!("{a}: {d}")]);
    }
    if govde.is_some() {
        c.args(["--data-binary", "@-"]);
    }
    c.arg(url)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let mut p = c.spawn().map_err(|e| format!("curl çalışmadı: {e}"))?;
    if let Some(mut g) = p.stdin.take() {
        if let Some(b) = govde {
            let _ = g.write_all(b.as_bytes());
        }
    }
    let o = p.wait_with_output().map_err(|e| e.to_string())?;
    let cikti = String::from_utf8_lossy(&o.stdout);
    let (yanit, kod) = cikti.rsplit_once('\n').unwrap_or(("", &cikti));
    let kod: u16 = kod.trim().parse().unwrap_or(0);
    if kod == 0 {
        return Ok((0, String::from_utf8_lossy(&o.stderr).trim().to_string()));
    }
    Ok((kod, yanit.to_string()))
}

/// `curl` ile gönderir. Gövde stdin'den verilir.
pub struct CurlGonderici;

impl Gonderici for CurlGonderici {
    fn gonder(&self, url: &str, govde: &str, imza: &str) -> Result<(), (bool, String)> {
        let (kod, yanit) = curl_istek("POST", url, &[("X-TAA-Imza", imza)], Some(govde), 15)
            .map_err(|e| (false, e))?;
        match kod {
            200..=299 => Ok(()),
            400..=499 => Err((true, format!("HTTP {kod}"))),
            0 => Err((false, yanit)),
            _ => Err((false, format!("HTTP {kod}"))),
        }
    }
}

/// Bekleyenleri her hedefe gönderir. Tüm hedeflere ulaşan kayıt kuyruktan düşer. (gönderilen, kalan) döner.
pub fn gonder_bekleyenler(dizin: &Path, g: &dyn Gonderici) -> Result<(usize, usize), String> {
    let Some(ayar) = Ayar::dizinden(dizin) else {
        return Ok((0, 0));
    };
    let _g = KILIT.lock().map_err(|_| "kilit hatası")?;
    let mut b = bekleyen_oku(dizin);
    let mut gonderilen = 0;
    let mut kalici_hata = false;
    for e in b.values_mut() {
        let Ok((govde, imza)) = imzala(dizin, &e["kayit"]) else {
            continue;
        };
        for h in &ayar.hedefler {
            if e["gonderildi"][h].is_number() {
                continue;
            }
            match g.gonder(h, &govde, &imza) {
                Ok(()) => {
                    e["gonderildi"][h] = json!(1);
                    gonderilen += 1;
                }
                Err((true, ileti)) => {
                    // Kalıcı ret: yeniden denenmez, hata kaydına düşer.
                    e["gonderildi"][h] = json!(-1);
                    e["hata"] = json!(ileti);
                    kalici_hata = true;
                }
                Err((false, _)) => {}
            }
        }
    }
    b.retain(|_, e| !ayar.hedefler.iter().all(|h| e["gonderildi"][h].is_number()));
    let kalan = b.len();
    bekleyen_yaz(dizin, &b)?;
    if kalici_hata {
        eprintln!(
            "Müşteri takip: bir hedef kaydı reddetti (imza/ayar?). Ayrıntı: {KLASOR}/bekleyen.json"
        );
    }
    Ok((gonderilen, kalan))
}

#[cfg(test)]
mod testler {
    use super::*;

    fn gecici() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "taa-musteri-takip-{}-{}",
            std::process::id(),
            hex(&rastgele(4).unwrap())
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn ayarla(d: &Path) {
        std::fs::write(
            d.join("musteri-takip.json"),
            r#"{"acik":true,"sayfa":"https://a.example/","yedek_sayfa":"https://b.example",
               "hedefler":["https://a.example/api/yaz","https://b.example/api/yaz","http://duz.example/x"]}"#,
        )
        .unwrap();
    }

    fn fis(no: &str, durum: Durum) -> Fis {
        let s = format!(
            r#"{{"no":"{no}","tarih":"2026-11-26","musteri":"Ayşe Yılmaz","telefon":"05321234567",
               "kalemler":[{{"urun":"Ceket","islem":"kol"}}],"teslim":"2026-11-29","ucret_kurus":45000,
               "on_odeme_kurus":0,"durum":"bekliyor","sube":"MRK01"}}"#
        );
        let mut f: Fis = serde_json::from_str(&s).unwrap();
        f.durum = durum;
        f
    }

    #[test]
    fn hmac_bilinen_deger() {
        // RFC 4231 test 2
        let h = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex(&h),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn kod_sabit_tahmin_edilemez_ve_kuruluma_ozel() {
        let (d1, d2) = (gecici(), gecici());
        let a = takip_kodu(&d1, "F-20261126-001").unwrap();
        assert_eq!(a, takip_kodu(&d1, "F-20261126-001").unwrap());
        assert!(kod_gecerli(&a), "{a}");
        assert_ne!(a, takip_kodu(&d1, "F-20261126-002").unwrap());
        assert_ne!(a, takip_kodu(&d2, "F-20261126-001").unwrap());
        assert!(!a.contains("F-") && !a.contains("2026"));
    }

    #[test]
    fn ayar_yoksa_kapali_ve_duz_http_atlanir() {
        let d = gecici();
        assert!(Ayar::dizinden(&d).is_none());
        kuyruga_yaz(&d, &fis("F-1", Durum::Bekliyor), 1).unwrap();
        assert!(!bekleyen_yolu(&d).exists());
        ayarla(&d);
        let a = Ayar::dizinden(&d).unwrap();
        assert_eq!(a.sayfa, "https://a.example");
        assert_eq!(a.hedefler.len(), 2, "düz http hedef atlanmalı");
        assert_eq!(a.adres("abc"), "https://a.example/t/abc");
        assert_eq!(a.yedek_adres("abc").unwrap(), "https://b.example/t/abc");
    }

    #[test]
    fn kayitta_kisisel_veri_yok_ve_imza_dogrulanir() {
        use ed25519_dalek::Verifier;
        let d = gecici();
        let f = fis("F-20261126-007", Durum::Hazir);
        let k = kayit(&d, &f, 1_000).unwrap();
        let (govde, imza) = imzala(&d, &k).unwrap();
        for yasak in [
            "Ayşe",
            "Yılmaz",
            "0532",
            "5321234567",
            "kol",
            "45000",
            "F-20261126-007",
        ] {
            assert!(
                !govde.contains(yasak),
                "dışarı giden kayıtta {yasak} var: {govde}"
            );
        }
        assert_eq!(k["d"], "hazir");
        assert_eq!(k["s"], "MRK01");
        assert_eq!(k["t"], "2026-11-29");
        assert_eq!(
            k["u"],
            json!([{"u": "ceket"}]),
            "serbest metin işlem dışarı gitmemeli"
        );
        let mut f2 = fis("F-2", Durum::Bekliyor);
        f2.kalemler[0].urun = "Pantolon".into();
        f2.kalemler[0].islem_kodu = Some("pant-paca-kisalt".into());
        f2.kalemler[0].varyant = Some("Duble paça".into());
        f2.kalemler[0].olcu_mm = Some(25);
        f2.kalemler[0].not = "Ayşe Hanım acele".into();
        let k2 = kayit(&d, &f2, 1_000).unwrap();
        assert_eq!(
            k2["u"],
            json!([{"u": "pantolon", "i": "pant-paca-kisalt", "v": "duble"}])
        );
        let g2 = serde_json::to_string(&k2).unwrap();
        assert!(!g2.contains("Ayşe") && !g2.contains("acele"), "{g2}");
        assert_eq!(urun_anahtari("GÖMLEK"), "gomlek");
        assert_eq!(urun_anahtari("Kız çocuk elbisesi"), "diger");
        let g: [u8; 32] = hex_coz(&genel_anahtar(&d).unwrap())
            .unwrap()
            .try_into()
            .unwrap();
        let g = ed25519_dalek::VerifyingKey::from_bytes(&g).unwrap();
        let i: [u8; 64] = hex_coz(&imza).unwrap().try_into().unwrap();
        assert!(g
            .verify(govde.as_bytes(), &ed25519_dalek::Signature::from_bytes(&i))
            .is_ok());
        let teslim = kayit(&d, &fis("F-1", Durum::Teslim), 1_000).unwrap();
        assert_eq!(teslim["sil"], 1_000 + 7 * GUN);
    }

    struct Sahte {
        cevap: std::collections::HashMap<String, Result<(), (bool, String)>>,
        gelen: std::sync::Mutex<Vec<String>>,
    }

    impl Gonderici for Sahte {
        fn gonder(&self, url: &str, govde: &str, _imza: &str) -> Result<(), (bool, String)> {
            self.gelen.lock().unwrap().push(format!("{url} {govde}"));
            self.cevap.get(url).cloned().unwrap_or(Ok(()))
        }
    }

    #[test]
    fn iki_hedef_biri_dusukse_yalniz_o_yeniden_denenir() {
        let d = gecici();
        ayarla(&d);
        kuyruga_yaz(&d, &fis("F-1", Durum::Bekliyor), 10).unwrap();
        let mut s = Sahte {
            cevap: [(
                "https://b.example/api/yaz".to_string(),
                Err((false, "ağ yok".into())),
            )]
            .into(),
            gelen: Default::default(),
        };
        assert_eq!(gonder_bekleyenler(&d, &s).unwrap(), (1, 1));
        s.cevap.clear();
        s.gelen.lock().unwrap().clear();
        assert_eq!(gonder_bekleyenler(&d, &s).unwrap(), (1, 0));
        let gelen = s.gelen.lock().unwrap();
        assert_eq!(gelen.len(), 1);
        assert!(gelen[0].starts_with("https://b.example/api/yaz"));
        drop(gelen);
        // Yeni durum aynı kodu günceller (tek kayıt).
        kuyruga_yaz(&d, &fis("F-1", Durum::Terzihanede), 20).unwrap();
        kuyruga_yaz(&d, &fis("F-1", Durum::Hazir), 30).unwrap();
        let b = bekleyen_oku(&d);
        assert_eq!(b.len(), 1);
        assert_eq!(b.values().next().unwrap()["kayit"]["d"], "hazir");
    }
}
