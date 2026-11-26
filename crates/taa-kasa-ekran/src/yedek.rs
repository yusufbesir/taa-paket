// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! Sunucuya şifreli kasa yedeği (`kasa.yedek@1`). Veri yerinde kalır (`%APPDATA%\TAA\veri\kasa`); yedek ek
//! güvencedir. Akış:
//! 1. `yedek_al`: kasa klasörünün üst düzey metin dosyaları (defterler, ayar; `yonetici-pin.json` HARİÇ) tek
//!    JSON demetinde toplanır, bellekte şifrelenir (taa-yedek "TAAY1", XChaCha20-Poly1305; düz metin diske
//!    yazılmaz) ve `<kasa>/yedek/giden/<damga>.taay` + manifest `<damga>.json` olarak kuyruğa alınır. Manifest
//!    şifreli dosyanın SHA-256'sını ve her dosyanın düz SHA-256'sını taşır (bütünlük).
//! 2. `YedekKuyrugu::bosalt`: her yedeğin SHA-256'sı göndermeden önce yeniden doğrulanır; tutmayan gönderilmez.
//!    Teslim edilen yedek SİLİNMEZ, `gonderilen`e taşınır. Çevrimdışıyken yedekler kuyrukta bekler (veri kaybı yok).
//! 3. Sunucu ucu henüz yok: `SunucuYok` yer tutucudur.
//!
//! Anahtar: `<veri kökü>/cihaz/yedek.key` (yoksa üretilir; üzerine yazılmaz). Anahtarın merkezde emanet
//! (escrow) tutulması açık konudur: anahtar kaybolursa sunucudaki yedek çözülemez.
//!
//! Şifre biçimi ve anahtar dosyası taa-yedek (`sifre.rs`) ile birebir aynıdır ("TAAY1" | 19 bayt ön ek | 64 KiB
//! parçalar, XChaCha20-Poly1305 akışı; `taa-yedek-anahtar:<64 hane>`), böylece sunucu tarafı taa-yedek ile çözebilir.
//! taa-yedek başka programın crate'i olduğu için bağımlılık alınmadı; kod burada kısa tutuldu.

use chacha20poly1305::aead::stream::{DecryptorBE32, EncryptorBE32};
use chacha20poly1305::{Key, KeyInit, XChaCha20Poly1305};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use taa_kasa::zaman;

pub const YEDEK_TURU: &str = "kasa.yedek@1";
pub const ICERIK_TURU: &str = "kasa.yedek.icerik@1";
pub const SIFRE_BICIMI: &str = "taa-yedek-taay1-xchacha20poly1305";
/// Yedeğe hiç girmeyen dosyalar (sır).
pub const HARIC: [&str; 1] = ["yonetici-pin.json"];
/// Sunucu ucu yokken dönen yer tutucu mesajı.
pub const SUNUCU_YOK: &str = "yedek sunucusu ucu henüz yok; şifreli yedek kuyrukta bekliyor";

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

fn sha256_hex(b: &[u8]) -> String {
    hex(&Sha256::digest(b))
}

// ---------------------------------------------------------------- şifre (taa-yedek "TAAY1" ile aynı biçim)

const ON_EK: &[u8; 5] = b"TAAY1";
const PARCA: usize = 64 * 1024;
const ETIKET: usize = 16;
const ANAHTAR_ON_EK: &str = "taa-yedek-anahtar:";

/// 32 baytlık yedek anahtarı. `Debug` içeriği göstermez.
pub struct Anahtar([u8; 32]);

impl std::fmt::Debug for Anahtar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Anahtar(***)")
    }
}

impl Anahtar {
    fn sifreleyici(&self) -> XChaCha20Poly1305 {
        XChaCha20Poly1305::new(Key::from_slice(&self.0))
    }
}

/// Yeni rastgele anahtarı YENİ dosya olarak yazar (var olan ezilmez; Unix'te 0600).
pub fn anahtar_uret(yol: &Path) -> Result<(), String> {
    let mut a = [0u8; 32];
    getrandom::fill(&mut a).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    let mut oz = std::fs::OpenOptions::new();
    oz.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        oz.mode(0o600);
    }
    if let Some(d) = yol.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    let mut f = oz
        .open(yol)
        .map_err(|e| format!("yedek anahtarı yazılamadı: {e}"))?;
    f.write_all(format!("{ANAHTAR_ON_EK}{}\n", hex(&a)).as_bytes())
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())
}

pub fn anahtar_oku(yol: &Path) -> Result<Anahtar, String> {
    let m = std::fs::read_to_string(yol).map_err(|e| format!("yedek anahtarı okunamadı: {e}"))?;
    let h = m
        .lines()
        .map(str::trim)
        .find(|s| !s.is_empty() && !s.starts_with('#'))
        .and_then(|s| s.strip_prefix(ANAHTAR_ON_EK))
        .filter(|h| h.len() == 64 && h.is_ascii())
        .ok_or("yedek anahtarı biçimi bozuk")?;
    let mut o = [0u8; 32];
    for (i, c) in o.iter_mut().enumerate() {
        *c = u8::from_str_radix(&h[2 * i..2 * i + 2], 16)
            .map_err(|_| "yedek anahtarı biçimi bozuk")?;
    }
    Ok(Anahtar(o))
}

/// Bellekte şifreler (düz metin diske yazılmaz).
pub fn sifrele(veri: &[u8], a: &Anahtar) -> Result<Vec<u8>, String> {
    let mut on = [0u8; 19];
    getrandom::fill(&mut on).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    let mut c = Vec::with_capacity(veri.len() + 24 + (veri.len() / PARCA + 1) * ETIKET);
    c.extend_from_slice(ON_EK);
    c.extend_from_slice(&on);
    let mut enc = EncryptorBE32::from_aead(a.sifreleyici(), (&on).into());
    let mut parcalar = veri.chunks(PARCA).peekable();
    if parcalar.peek().is_none() {
        c.extend(
            enc.encrypt_last(&[][..])
                .map_err(|_| "şifreleme başarısız")?,
        );
        return Ok(c);
    }
    while let Some(p) = parcalar.next() {
        // Tam dolu son parça da "son" işaretlenir (taa-yedek ile aynı).
        if parcalar.peek().is_none() {
            c.extend(enc.encrypt_last(p).map_err(|_| "şifreleme başarısız")?);
            break;
        }
        c.extend(enc.encrypt_next(p).map_err(|_| "şifreleme başarısız")?);
    }
    Ok(c)
}

/// `sifrele` çıktısını (ya da taa-yedek `.taay` dosyasını) bellekte çözer.
pub fn coz(veri: &[u8], a: &Anahtar) -> Result<Vec<u8>, String> {
    let hata = || "yedek çözülemedi: anahtar yanlış ya da dosya bozuk/kesik".to_string();
    if veri.len() < 24 + ETIKET || &veri[..5] != ON_EK {
        return Err(hata());
    }
    let mut dec = DecryptorBE32::from_aead(a.sifreleyici(), (&veri[5..24]).into());
    let mut d = Vec::with_capacity(veri.len());
    let mut parcalar = veri[24..].chunks(PARCA + ETIKET).peekable();
    while let Some(p) = parcalar.next() {
        if parcalar.peek().is_none() {
            d.extend(dec.decrypt_last(p).map_err(|_| hata())?);
            return Ok(d);
        }
        d.extend(dec.decrypt_next(p).map_err(|_| hata())?);
    }
    Err(hata())
}

/// Sunucuya yedek gönderen uç.
pub trait YedekSunucusu {
    fn gonder(&self, manifest: &Value, sifreli: &[u8]) -> Result<(), String>;
}

/// Yer tutucu: sunucu yedek ucu (sözleşme `kasa.yedek@1`) bağlanana kadar gönderilmez.
#[derive(Debug, Clone, Copy, Default)]
pub struct SunucuYok;

impl YedekSunucusu for SunucuYok {
    fn gonder(&self, _manifest: &Value, _sifreli: &[u8]) -> Result<(), String> {
        Err(SUNUCU_YOK.into())
    }
}

/// Yedek kuyruğu (`<kasa>/yedek`).
pub struct YedekKuyrugu {
    pub kok: PathBuf,
}

fn manifest_adi_gecerli(ad: &str) -> bool {
    ad.len() <= 64
        && ad.starts_with("kasa-yedek-")
        && ad.ends_with(".json")
        && ad
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.'))
        && !ad.contains("..")
}

impl YedekKuyrugu {
    pub fn kasa_icin(kasa: &Path) -> YedekKuyrugu {
        YedekKuyrugu {
            kok: kasa.join("yedek"),
        }
    }
    pub fn giden(&self) -> PathBuf {
        self.kok.join("giden")
    }
    pub fn gonderilen(&self) -> PathBuf {
        self.kok.join("gonderilen")
    }

    fn listele(k: &Path) -> Vec<String> {
        let mut v: Vec<String> = std::fs::read_dir(k)
            .map(|d| {
                d.filter_map(Result::ok)
                    .filter_map(|e| e.file_name().into_string().ok())
                    .filter(|a| manifest_adi_gecerli(a))
                    .collect()
            })
            .unwrap_or_default();
        v.sort();
        v
    }

    /// Bekleyen yedeklerin manifest adları (eskiden yeniye).
    pub fn bekleyenler(&self) -> Vec<String> {
        Self::listele(&self.giden())
    }

    pub fn gonderilenler(&self) -> Vec<String> {
        Self::listele(&self.gonderilen())
    }

    /// Bekleyen bir yedeğin manifestini ve şifreli baytlarını okur, bütünlüğü (boyut + SHA-256) doğrular.
    pub fn oku_dogrula(&self, manifest_adi: &str) -> Result<(Value, Vec<u8>), String> {
        oku_dogrula(&self.giden(), manifest_adi)
    }

    /// Bekleyenleri sunucuya göndermeye çalışır. Bütünlüğü tutmayan yedek gönderilmez, yerinde bırakılır.
    /// Dönüş: (gönderilen sayısı, ilk hata). Ağ hatasında durulur (sıra korunur).
    pub fn bosalt(&self, s: &dyn YedekSunucusu) -> (usize, Option<String>) {
        let mut n = 0;
        let mut ilk_hata = None;
        for ad in self.bekleyenler() {
            let (m, bayt) = match self.oku_dogrula(&ad) {
                Ok(x) => x,
                Err(e) => {
                    ilk_hata.get_or_insert(e);
                    continue;
                }
            };
            if let Err(e) = s.gonder(&m, &bayt) {
                return (n, Some(ilk_hata.unwrap_or(e)));
            }
            let dosya = m["dosya"].as_str().unwrap_or("").to_string();
            let tasi = std::fs::create_dir_all(self.gonderilen())
                .and_then(|_| {
                    std::fs::rename(self.giden().join(&dosya), self.gonderilen().join(&dosya))
                })
                .and_then(|_| std::fs::rename(self.giden().join(&ad), self.gonderilen().join(&ad)));
            if let Err(e) = tasi {
                return (
                    n,
                    Some(format!("yedek gönderildi ama kuyruktan taşınamadı: {e}")),
                );
            }
            n += 1;
        }
        (n, ilk_hata)
    }
}

fn oku_dogrula(klasor: &Path, manifest_adi: &str) -> Result<(Value, Vec<u8>), String> {
    if !manifest_adi_gecerli(manifest_adi) {
        return Err(format!("yedek manifest adı geçersiz: {manifest_adi}"));
    }
    let m: Value = std::fs::read(klasor.join(manifest_adi))
        .map_err(|e| format!("{manifest_adi}: {e}"))
        .and_then(|b| serde_json::from_slice(&b).map_err(|e| format!("{manifest_adi}: {e}")))?;
    if m["tur"] != YEDEK_TURU {
        return Err(format!("{manifest_adi}: yedek manifesti değil"));
    }
    let dosya = m["dosya"].as_str().unwrap_or("");
    let beklenen_dosya = manifest_adi.trim_end_matches(".json").to_string() + ".taay";
    if dosya != beklenen_dosya {
        return Err(format!(
            "{manifest_adi}: şifreli dosya adı manifestle tutmuyor"
        ));
    }
    let bayt = std::fs::read(klasor.join(dosya)).map_err(|e| format!("{dosya}: {e}"))?;
    if m["boyut"].as_u64() != Some(bayt.len() as u64) || m["sha256"] != sha256_hex(&bayt) {
        return Err(format!(
            "{dosya}: bütünlük tutmuyor (SHA-256/boyut); gönderilmedi"
        ));
    }
    Ok((m, bayt))
}

/// Yedeğe giren dosyalar: kasa klasörünün üst düzey `.json`/`.jsonl` dosyaları, `HARIC` dışında (ada göre sıralı).
pub fn yedek_dosyalari(kasa: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(kasa)
        .map(|d| {
            d.filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.is_file())
                .filter(|p| p.extension().is_some_and(|x| x == "json" || x == "jsonl"))
                .filter(|p| {
                    p.file_name()
                        .and_then(|a| a.to_str())
                        .is_some_and(|a| !HARIC.contains(&a))
                })
                .collect()
        })
        .unwrap_or_default();
    v.sort();
    v
}

/// Kasa klasörünün şifreli yedeğini kuyruğa alır. Dönüş: manifest. Anahtar yoksa üretilir.
pub fn yedek_al(kasa: &Path, anahtar_yolu: &Path, sube: &str) -> Result<Value, String> {
    if !anahtar_yolu.is_file() {
        anahtar_uret(anahtar_yolu)?;
    }
    let anahtar = anahtar_oku(anahtar_yolu)?;
    let mut dosyalar = Vec::new();
    let mut liste = Vec::new();
    for y in yedek_dosyalari(kasa) {
        let ad = y
            .file_name()
            .and_then(|a| a.to_str())
            .unwrap_or_default()
            .to_string();
        let b = std::fs::read(&y).map_err(|e| format!("{ad}: {e}"))?;
        let icerik = String::from_utf8(b).map_err(|_| format!("{ad}: UTF-8 metin değil"))?;
        let h = sha256_hex(icerik.as_bytes());
        liste.push(json!({"ad": ad, "boyut": icerik.len(), "sha256": h}));
        dosyalar.push(json!({"ad": ad, "sha256": h, "icerik": icerik}));
    }
    let simdi = zaman::simdi_sn();
    let demet = json!({"tur": ICERIK_TURU, "sube": sube, "olusturma": simdi, "dosyalar": dosyalar});
    let sifreli = sifrele(demet.to_string().as_bytes(), &anahtar)?;
    let k = YedekKuyrugu::kasa_icin(kasa);
    std::fs::create_dir_all(k.giden()).map_err(|e| e.to_string())?;
    // Damga: zaman + sıra (aynı saniyede ikinci yedek ayrı ad alır; var olan dosyanın üzerine yazılmaz).
    let mut sira = 1;
    let damga = loop {
        let d = format!("kasa-yedek-{simdi}-{sira}");
        let var = [k.giden(), k.gonderilen()].iter().any(|kl| {
            kl.join(format!("{d}.json")).exists() || kl.join(format!("{d}.taay")).exists()
        });
        if !var {
            break d;
        }
        sira += 1;
    };
    let dosya = format!("{damga}.taay");
    let m = json!({
        "tur": YEDEK_TURU,
        "surum": 1,
        "sube": sube,
        "olusturma": simdi,
        "dosya": dosya,
        "boyut": sifreli.len(),
        "sha256": sha256_hex(&sifreli),
        "sifre": SIFRE_BICIMI,
        "dosyalar": liste,
    });
    // Önce şifreli dosya, sonra manifest: manifest varsa çift tamdır.
    taa_dosya::uzerine_yaz_atomik(&k.giden().join(&dosya), &sifreli).map_err(|e| e.to_string())?;
    taa_dosya::uzerine_yaz_atomik(
        &k.giden().join(format!("{damga}.json")),
        serde_json::to_string_pretty(&m)
            .map_err(|e| e.to_string())?
            .as_bytes(),
    )
    .map_err(|e| e.to_string())?;
    Ok(m)
}

/// Şifreli yedeği çözer ve her dosyanın SHA-256'sını manifestle karşılaştırır (geri yükleme ve denetim için).
/// Dönüş: (dosya adı, içerik) listesi. Diske yazmaz.
pub fn yedek_coz(
    manifest: &Value,
    sifreli: &[u8],
    anahtar: &Anahtar,
) -> Result<Vec<(String, String)>, String> {
    if manifest["sha256"] != sha256_hex(sifreli) {
        return Err("şifreli yedeğin SHA-256'sı manifestle tutmuyor".into());
    }
    let duz = coz(sifreli, anahtar)?;
    let d: Value = serde_json::from_slice(&duz).map_err(|e| format!("yedek içeriği: {e}"))?;
    if d["tur"] != ICERIK_TURU {
        return Err("yedek içeriği tanınmadı".into());
    }
    let mut v = Vec::new();
    for f in d["dosyalar"].as_array().into_iter().flatten() {
        let ad = f["ad"].as_str().unwrap_or("").to_string();
        let icerik = f["icerik"].as_str().unwrap_or("").to_string();
        let h = sha256_hex(icerik.as_bytes());
        let listede = manifest["dosyalar"]
            .as_array()
            .into_iter()
            .flatten()
            .any(|x| x["ad"] == ad.as_str() && x["sha256"] == h.as_str());
        if f["sha256"] != h.as_str() || !listede {
            return Err(format!("{ad}: dosya özeti tutmuyor"));
        }
        v.push((ad, icerik));
    }
    Ok(v)
}

#[cfg(test)]
mod testler {
    use super::*;
    use std::cell::RefCell;

    #[derive(Default)]
    struct SahteSunucu {
        alinan: RefCell<Vec<(Value, Vec<u8>)>>,
        kapali: bool,
    }
    impl YedekSunucusu for SahteSunucu {
        fn gonder(&self, m: &Value, b: &[u8]) -> Result<(), String> {
            if self.kapali {
                return Err("bağlantı yok".into());
            }
            self.alinan.borrow_mut().push((m.clone(), b.to_vec()));
            Ok(())
        }
    }

    fn kasa(ad: &str) -> (PathBuf, PathBuf) {
        let k = taa_dosya::gecici_klasor()
            .unwrap()
            .join(format!("kasa-yedek-{ad}"));
        let kasa = k.join("kasa");
        std::fs::create_dir_all(&kasa).unwrap();
        // Anonim test verisi.
        std::fs::write(kasa.join("gunsonu.jsonl"), "{\"tur\":\"ornek\"}\n").unwrap();
        std::fs::write(kasa.join("ayar.json"), "{\"sube\":\"SB1\"}").unwrap();
        std::fs::write(kasa.join("yonetici-pin.json"), "{\"ozet\":\"gizli\"}").unwrap();
        (kasa, k.join("cihaz").join("yedek.key"))
    }

    #[test]
    fn bellekte_sifrele_coz_parca_siniri_yanlis_anahtar_kesik() {
        let a = Anahtar([7; 32]);
        for n in [0, 1, PARCA - 1, PARCA, PARCA + 1, 3 * PARCA] {
            let veri: Vec<u8> = (0..n).map(|i| (i % 251) as u8).collect();
            let s = sifrele(&veri, &a).unwrap();
            assert_eq!(&s[..5], ON_EK);
            assert_eq!(coz(&s, &a).unwrap(), veri, "{n}");
        }
        let s = sifrele(b"anonim test", &a).unwrap();
        assert!(coz(&s, &Anahtar([8; 32])).is_err(), "yanlış anahtar");
        let mut bozuk = s.clone();
        let son = bozuk.len() - 1;
        bozuk[son] ^= 1;
        assert!(coz(&bozuk, &a).is_err(), "bozuk bayt");
        assert!(coz(&s[..s.len() - 1], &a).is_err(), "kesik");
        // Tam parça sınırında kesilen akış (son parça eksik) çözülmez.
        let uzun = sifrele(&vec![1u8; 2 * PARCA], &a).unwrap();
        assert!(coz(&uzun[..24 + PARCA + ETIKET], &a).is_err());
    }

    #[test]
    fn sifreli_yedek_cozulur_pin_haric_ve_dosya_ozetleri_tutar() {
        let (kasa, anahtar) = kasa("cozum");
        let m = yedek_al(&kasa, &anahtar, "SB1").unwrap();
        assert_eq!(m["tur"], YEDEK_TURU);
        let adlar: Vec<&str> = m["dosyalar"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["ad"].as_str().unwrap())
            .collect();
        assert_eq!(adlar, ["ayar.json", "gunsonu.jsonl"], "PIN yedeğe girmez");
        let k = YedekKuyrugu::kasa_icin(&kasa);
        let ad = k.bekleyenler().pop().unwrap();
        let (m2, bayt) = k.oku_dogrula(&ad).unwrap();
        assert_eq!(m2, m);
        // Düz metin şifreli dosyada görünmez.
        assert!(!String::from_utf8_lossy(&bayt).contains("ornek"));
        let a = anahtar_oku(&anahtar).unwrap();
        let d = yedek_coz(&m, &bayt, &a).unwrap();
        assert_eq!(
            d[1],
            ("gunsonu.jsonl".into(), "{\"tur\":\"ornek\"}\n".into())
        );
        // Anahtar üzerine yazılmaz: ikinci yedek aynı anahtarla çözülür ve ayrı ad alır.
        let m3 = yedek_al(&kasa, &anahtar, "SB1").unwrap();
        assert_ne!(m3["dosya"], m["dosya"]);
        assert_eq!(k.bekleyenler().len(), 2);
    }

    #[test]
    fn butunlugu_tutmayan_yedek_gonderilmez() {
        let (kasa, anahtar) = kasa("butunluk");
        let m = yedek_al(&kasa, &anahtar, "SB1").unwrap();
        let k = YedekKuyrugu::kasa_icin(&kasa);
        let yol = k.giden().join(m["dosya"].as_str().unwrap());
        let mut b = std::fs::read(&yol).unwrap();
        b[30] ^= 1;
        std::fs::write(&yol, &b).unwrap();
        let s = SahteSunucu::default();
        let (n, h) = k.bosalt(&s);
        assert_eq!(n, 0);
        assert!(h.unwrap().contains("bütünlük"));
        assert!(s.alinan.borrow().is_empty());
        assert_eq!(k.bekleyenler().len(), 1, "bozuk yedek yerinde kalır");
        let a = anahtar_oku(&anahtar).unwrap();
        assert!(yedek_coz(&m, &b, &a).is_err());
    }

    #[test]
    fn cevrimdisi_kuyrukta_bekler_baglaninca_gider_silinmez() {
        let (kasa, anahtar) = kasa("kuyruk");
        yedek_al(&kasa, &anahtar, "SB1").unwrap();
        yedek_al(&kasa, &anahtar, "SB1").unwrap();
        let k = YedekKuyrugu::kasa_icin(&kasa);
        // Uç yok ve bağlantı yok: hiçbir şey kaybolmaz.
        assert_eq!(k.bosalt(&SunucuYok), (0, Some(SUNUCU_YOK.into())));
        let kapali = SahteSunucu {
            kapali: true,
            ..Default::default()
        };
        assert_eq!(k.bosalt(&kapali).0, 0);
        assert_eq!(k.bekleyenler().len(), 2);
        let s = SahteSunucu::default();
        assert_eq!(k.bosalt(&s), (2, None));
        assert!(k.bekleyenler().is_empty());
        assert_eq!(k.gonderilenler().len(), 2, "gönderilen silinmez, taşınır");
        let (m, b) = &s.alinan.borrow()[0];
        assert_eq!(m["sha256"], sha256_hex(b));
        assert!(oku_dogrula(&k.gonderilen(), &k.gonderilenler()[0]).is_ok());
    }
}
