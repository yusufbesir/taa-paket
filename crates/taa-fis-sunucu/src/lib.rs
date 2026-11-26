//! taa-fis-sunucu: TaaFiş yerel ağ sunucusunun mantığı (HTTP'den bağımsız, test edilebilir).
//! Güvenlik: dışarıya açık port yok (yalnız mağaza ağı); telefon, bilgisayarda üretilen tek kullanımlık
//! 6 haneli kodla eşleşir (10 dk, 5 yanlışta 60 sn kilit); belirteç yalnız SHA256 özetiyle saklanır;
//! yönetim sayfası ve kod üretimi yalnız yerel bilgisayardan; yönetici telefonu kapatabilir.
//! Özellik anahtarları: "fis-sunucu", "telefon-eslestirme" (varsayılan açık). Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-04].

pub mod cati_pin;
pub mod cihaz;
mod ek_uclar;
mod hesap_uclari;
pub mod kisi;
pub mod kurulum;
pub mod musteri_takip;
pub mod ortak_giris;
pub mod pencere;
pub mod sayfalar;
mod teslim_al;
mod yonetici;

pub use ek_uclar::karekod_svg;
use ek_uclar::{deneme_baytlari, fis_json};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use taa_fis::{Defter, Durum, Girdi};

pub const GOVDE_SINIRI: usize = 64 * 1024;
/// Ürün fotoğrafı (JPEG) için gövde sınırı.
pub const FOTO_SINIRI: usize = 3 * 1024 * 1024;
const EN_COK_FOTO: usize = 6;
const KOD_OMRU: u64 = 300;
const KILIT_SURESI: u64 = 60;
const EN_COK_YANLIS: u32 = 5;
/// Yönetici oturumu bu kadar saniye işlem olmazsa kapanır.
pub const YONETICI_BOSTA_SN: u64 = 600;
/// Mağaza bilgisayarının sözde cihaz kimliği (masaüstü program, yalnız bu bilgisayardan).
pub const PC_ID: &str = "PC";

#[derive(Default)]
pub struct Istek {
    pub metot: String,
    pub yol: String,
    pub yetki: Option<String>,
    pub govde: Vec<u8>,
    /// İstek bu bilgisayardan (loopback adresten ve loopback `Host` başlığıyla) geldi mi?
    pub yerel: bool,
    /// `X-TAA-Masaustu: 1` başlığı: masaüstü programın kendi isteği. Özel başlık başka sitelerin
    /// tarayıcı üzerinden istek atmasını engeller (CORS ön denetimi yanıtlanmaz).
    pub masaustu: bool,
    /// `X-TAA-Yonetici` başlığındaki yönetici oturum belirteci.
    pub yonetici: Option<String>,
}

pub struct Yanit {
    pub kod: u16,
    pub tur: &'static str,
    pub govde: Vec<u8>,
}

impl Yanit {
    fn json(kod: u16, v: serde_json::Value) -> Self {
        Yanit {
            kod,
            tur: "application/json; charset=utf-8",
            govde: v.to_string().into_bytes(),
        }
    }
    fn hata(kod: u16, m: &str) -> Self {
        Self::json(kod, json!({ "hata": m }))
    }
    fn html(s: String) -> Self {
        Yanit {
            kod: 200,
            tur: "text/html; charset=utf-8",
            govde: s.into_bytes(),
        }
    }
    fn tur(tur: &'static str, govde: impl Into<Vec<u8>>) -> Self {
        Yanit {
            kod: 200,
            tur,
            govde: govde.into(),
        }
    }
    /// Eski adresler yeni masaüstü programına yönlenir (yalnız yerel istekler buraya gelir).
    fn yonlendir(adres: &str) -> Self {
        Self::html(format!(
            "<!doctype html><meta charset=utf-8><meta http-equiv=refresh content=\"0;url={adres}\"><title>TAA Terzi Takip</title><p><a href=\"{adres}\">TAA Terzi Takip</a></p>"
        ))
    }
}

pub struct Uygulama {
    pub dizin: PathBuf,
    /// Yazıcıya ham baytların yazılacağı yol (Windows'ta paylaşılan yazıcı `\\localhost\ADI`, Linux'ta aygıt dosyası).
    pub yazici_yolu: Option<PathBuf>,
    /// QR'a gömülecek taban adres (örn. `http://192.168.1.20:8787`).
    pub taban_url: String,
    /// Müşteri nüshasındaki aydınlatma QR'ının adresi. Müşterinin telefonu mağaza ağı dışında olduğundan
    /// genelde şirketin herkese açık sayfası olmalı; yoksa `taban_url/aydinlatma` (yalnız mağaza Wi-Fi'ında açılır).
    pub aydinlatma_url: Option<String>,
    /// Saati ileri alır (yalnız test/deneme; kod süresi ve kilit sınanır).
    pub ileri_sn: std::sync::atomic::AtomicU64,
    /// Yönetim ekranları (Telefonlar, Personel, Usta puanı) yönetici PIN'i ister. Kapalıysa yalnız
    /// "bu bilgisayardan" kuralı kalır (eski davranış; testlerde kullanılır).
    pub yonetici_kilidi: bool,
    /// PIN'i doğrulayan taraf (TAA Çatı istemcisi).
    pub pin: Box<dyn cati_pin::PinDogrulayici>,
    /// Kişi listesi ve şube kodunu TAA Çatı'dan çeken taraf (bkz. `kurulum`).
    pub cati: std::sync::Arc<dyn kurulum::CatiKaynagi>,
    /// TAA programlarının ortak veri kökü (TAA Kur `sube.json`'u buraya yazar). Testlerde None.
    /// TAA ortak giriş aracı (taa-kimlik) bulucusu; testler sahte araç verir.
    pub kimlik_bulucu: ortak_giris::KimlikBulucu,
    pub kok: Option<PathBuf>,
    /// (eşleştirme kodu, bitiş, telefonun bağlanacağı kişi)
    kodlar: Mutex<Vec<(String, u64, String)>>,
    yanlis: Mutex<(u32, u64)>,
    /// Yönetici oturumları: (belirteç özeti, son işlem).
    yonetici_oturum: Mutex<Vec<(String, u64)>>,
    /// Cihaz kimliği -> son görülme (bellekte; yeniden başlatınca sıfırlanır).
    son_gorulme: Mutex<std::collections::BTreeMap<String, u64>>,
    /// Birinci kodu girmiş, ikinci kodu bekleyen telefonlar (çifte kod eşleştirme).
    bekleyen: Mutex<Vec<Bekleyen>>,
    yanlis2: Mutex<u32>,
}

/// Çifte kod eşleştirmede ikinci kodu bekleyen telefon. Kodlar yalnız bellekte durur, günlüğe yazılmaz.
struct Bekleyen {
    id: String,
    ad: String,
    /// Kodla bağlanacak kişi (boşsa mağaza telefonu).
    kisi_id: String,
    kod2: String,
    son: u64,
    /// Yönetici onayından sonra telefonun bir kez alacağı belirteç ve cihaz kimliği.
    tamam: Option<(String, String)>,
}

/// Unix saniyesi (UTC).
pub fn simdi() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Unix saniyesinden Türkiye (UTC+3) takvim günü, YYYY-AA-GG.
pub fn saat_iso(epoch: u64) -> String {
    let sn = (epoch + 3 * 3600) % 86400;
    format!("{:02}:{:02}:{:02}", sn / 3600, sn % 3600 / 60, sn % 60)
}

pub fn tarih_iso(epoch: u64) -> String {
    let gun = ((epoch + 3 * 3600) / 86400) as i64;
    let z = gun + 719468;
    let era = z.div_euclid(146097);
    let doe = z.rem_euclid(146097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

fn kapali(ad: &str) -> Option<Yanit> {
    if taa_ayar::ozellik_acik(ad, true) {
        None
    } else {
        Some(Yanit::hata(503, &taa_ayar::kapali_mesaji(ad)))
    }
}

/// Puan hesabı girdileri: ayar, süre tablosu, defter kayıtları, fiş no -> teslim tarihi.
type PuanGirdisi = (
    taa_takip::PuanAyari,
    taa_takip::SureTablosu,
    Vec<taa_takip::Kayit>,
    std::collections::BTreeMap<String, String>,
);

impl Uygulama {
    pub fn yeni(dizin: PathBuf, yazici_yolu: Option<PathBuf>, taban_url: String) -> Self {
        Uygulama {
            dizin,
            yazici_yolu,
            taban_url,
            aydinlatma_url: None,
            ileri_sn: std::sync::atomic::AtomicU64::new(0),
            yonetici_kilidi: true,
            pin: Box::new(cati_pin::Yapilandirilmamis),
            cati: std::sync::Arc::new(kurulum::Yapilandirilmamis),
            kok: None,
            kimlik_bulucu: ortak_giris::kimlik_yolu,
            kodlar: Mutex::new(Vec::new()),
            yanlis: Mutex::new((0, 0)),
            yonetici_oturum: Mutex::new(Vec::new()),
            son_gorulme: Mutex::new(Default::default()),
            bekleyen: Mutex::new(Vec::new()),
            yanlis2: Mutex::new(0),
        }
    }

    fn simdi(&self) -> u64 {
        simdi() + self.ileri_sn.load(std::sync::atomic::Ordering::SeqCst)
    }

    fn cihazlar(&self) -> cihaz::Kayitci {
        cihaz::Kayitci {
            yol: self.dizin.join("cihazlar.jsonl"),
        }
    }

    fn fis_yolu(&self) -> PathBuf {
        self.dizin.join("fisler.jsonl")
    }

    /// Bu sunucunun şube kodu. Şube kimliği sunucuda belirlenir; istemcinin beyanına güvenilmez
    /// (docs/sube-sozlesmesi.md §2 kural 1). Sıra: bu klasördeki `sube.json` (İlk kurulum, yönetici ya da TAA Çatı
    /// cihaz kaydı yazar), yoksa TAA veri kökündeki `sube.json` (TAA Kur yazar). Biçim: {"kod":"MRK01"}.
    fn sube_kodu(&self) -> Result<String, String> {
        kurulum::sube_oku(&self.dizin, self.kok.as_deref())
            .map(|(k, _)| k)
            .ok_or_else(|| "şube kodu ayarlanmamış: fiş yazılamaz".to_string())
    }

    fn yeni_kod(&self, kisi_id: &str) -> Result<String, String> {
        let simdi = self.simdi();
        let kod = cihaz::yeni_kod()?;
        let mut k = self.kodlar.lock().map_err(|_| "kilit hatası")?;
        k.retain(|(_, son, _)| *son > simdi);
        k.push((kod.clone(), simdi + KOD_OMRU, kisi_id.to_string()));
        Ok(kod)
    }

    /// Kod doğruysa tüketir ve kodla bağlanacak kişiyi (boşsa mağaza telefonu) döndürür.
    fn kod_tuket(&self, kod: &str) -> Result<String, String> {
        let simdi = self.simdi();
        let mut y = self.yanlis.lock().map_err(|_| "kilit hatası")?;
        if y.1 > simdi {
            return Err(format!(
                "çok fazla yanlış deneme; {} saniye bekleyin",
                y.1 - simdi
            ));
        }
        let mut k = self.kodlar.lock().map_err(|_| "kilit hatası")?;
        k.retain(|(_, son, _)| *son > simdi);
        if let Some(i) = k.iter().position(|(c, _, _)| c == kod) {
            let (_, _, kisi) = k.remove(i);
            *y = (0, 0);
            Ok(kisi)
        } else {
            y.0 += 1;
            if y.0 >= EN_COK_YANLIS {
                *y = (0, simdi + KILIT_SURESI);
            }
            Err("kod yanlış ya da süresi dolmuş".into())
        }
    }

    pub fn isle(&self, i: &Istek) -> Yanit {
        let foto = i.metot == "POST" && i.yol.split('?').next().unwrap_or("").ends_with("/foto");
        // Teslim alma ucunda imza PNG'si base64 gelir: sınır daha geniş (bkz. teslim_al.rs).
        let teslim_al_ucu = i.metot == "POST"
            && i.yol
                .split('?')
                .next()
                .unwrap_or("")
                .ends_with("/teslim-al");
        let sinir = if foto {
            FOTO_SINIRI
        } else if teslim_al_ucu {
            teslim_al::GOVDE_SINIRI_TESLIM_AL
        } else {
            GOVDE_SINIRI
        };
        if i.govde.len() > sinir {
            return Yanit::hata(413, "istek çok büyük");
        }
        let (yol, sorgu) = i.yol.split_once('?').unwrap_or((i.yol.as_str(), ""));
        let parca: Vec<&str> = yol.trim_matches('/').split('/').collect();
        if let Some(k) = kapali("fis-sunucu") {
            return k;
        }
        match (i.metot.as_str(), parca.as_slice()) {
            ("GET", ["telefon"]) => Yanit::html(sayfalar::telefon()),
            ("GET", [""]) => {
                if i.yerel {
                    Yanit::yonlendir("/masaustu")
                } else {
                    Yanit::html(sayfalar::telefon())
                }
            }
            ("GET", ["masaustu"]) => {
                if !i.yerel {
                    return Yanit::hata(403, "masaüstü program yalnız bu bilgisayardan açılır");
                }
                Yanit::html(sayfalar::masaustu())
            }
            ("GET", ["manifest.json"]) => {
                Yanit::tur("application/manifest+json", sayfalar::MANIFEST.as_bytes())
            }
            ("GET", ["simge.svg"]) => Yanit::tur("image/svg+xml", sayfalar::SIMGE.as_bytes()),
            // Chromium/Edge geliştirici araçları açıkken bu adresi kendiliğinden ister (otomatik çalışma klasörü);
            // 404 konsolda "Failed to load resource" satırı bırakıyordu. Boş yanıt: klasör önerilmez.
            ("GET", [".well-known", "appspecific", "com.chrome.devtools.json"]) => {
                Yanit::json(200, json!({}))
            }
            ("GET", ["api", "kurulum"]) => self.kurulum_durum(i),
            ("POST", ["api", "kurulum"]) => self.ilk_kurulum_yap(i),
            ("POST", ["api", "kurulum", "esitle"]) => self.kurulum_esitle(i),
            ("POST", ["api", "yonetim", "sube"]) => self.yonetim_sube(i),
            ("POST", ["api", "eslestir"]) => self.eslestir(i),
            ("GET", ["api", "eslestir", id]) => self.eslestir_durumu(id),
            ("POST", ["api", "yonetim", "eslestir", "onayla"]) => self.eslestir_onayla(i),
            ("GET", ["yonetim"]) => {
                if !i.yerel {
                    return Yanit::hata(403, "yönetim yalnız bu bilgisayardan açılır");
                }
                Yanit::yonlendir("/masaustu#telefonlar")
            }
            ("POST", ["api", "giris", islem]) => {
                // Ortak giriş yalnız bu bilgisayardaki masaüstü programdan (telefonlar eşleştirme belirteciyle çalışır).
                if !i.yerel || !i.masaustu {
                    return Yanit::hata(403, "yalnız bu bilgisayardaki programdan");
                }
                match ortak_giris::yanitla(self.kimlik_bulucu, cati_pin::PROGRAM, islem, &i.govde) {
                    Some(v) => Yanit::json(200, v),
                    None => Yanit::hata(404, "bulunamadı"),
                }
            }
            ("POST", ["api", "yonetici", "giris"]) => self.yonetici_giris(i),
            ("POST", ["api", "yonetici", "cikis"]) => self.yonetici_cikis(i),
            ("GET", ["api", "yonetici", "durum"]) => self.yonetici_durum(i),
            ("GET", ["api", "yonetim", "eslestirme"]) => self.eslestirme_kodu(i, sorgu),
            ("GET", ["api", "yonetim", "cihazlar"]) => self.cihaz_listesi(i),
            ("GET", ["api", "yonetim", "kisiler"]) => self.kisi_listesi_yonetim(i),
            ("POST", ["api", "yonetim", "kisi"]) => self.kisi_ekle(i),
            ("POST", ["api", "yonetim", "kisi", id, "kapat"]) => self.kisi_kapat(i, id),
            ("POST", ["api", "yonetim", "cihaz", id, "ad"]) => self.cihaz_ad(i, id),
            ("POST", ["api", "yonetim", "cihaz", id, "yetki"]) => self.cihaz_yetki(i, id),
            ("GET", ["api", "yonetim", "yetkiler"]) => self.yetki_listesi(i),
            ("POST", ["api", "yonetim", "cihaz", id, "kapat"]) => {
                if let Some(y) = self.yonetim_izni(i) {
                    return y;
                }
                match self.cihazlar().kapat(id, &tarih_iso(self.simdi())) {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true })),
                    Err(e) => Yanit::hata(404, &e),
                }
            }
            ("GET", ["yonetim", "puan"]) => {
                if !i.yerel {
                    return Yanit::hata(403, "yalnız bu bilgisayardan");
                }
                Yanit::yonlendir("/masaustu#puan")
            }
            ("GET", ["api", "yonetim", "isaretler"]) => self.puan_isaretleri(i),
            ("POST", ["api", "yonetim", "tutanak"]) => self.puan_tutanak(i),
            ("GET", ["api", "yonetim", "puan", bas, bit]) => self.puan_raporu(i, bas, bit),
            ("POST", ["api", "yonetim", "puan", "gozden-gecir", bas, bit]) => {
                self.puan_gozden_gecir(i, bas, bit)
            }
            ("GET", ["aydinlatma"]) => Yanit::html(
                std::fs::read_to_string(self.dizin.join("aydinlatma.html")).unwrap_or_else(|_| {
                    "<!doctype html><meta charset=utf-8><title>Aydınlatma</title><body style=\"font-family:sans-serif;max-width:40em;margin:1em auto;padding:0 1em\"><h1>Kişisel Verilerin Korunması Aydınlatma Metni</h1><p>Aydınlatma metni henüz yüklenmedi. Şirket KVKK sorumlusu metni veri klasörüne <code>aydinlatma.html</code> olarak koymalıdır.</p></body>".into()
                }),
            ),
            ("POST", ["telegram", gizli]) => self.telegram(gizli, i),
            ("GET", ["wa", belirtec]) => self.wa_sayfasi(belirtec),
            ("POST", ["api", "yonetim", "baglama-kodu"]) => self.baglama_kodu(i),
            ("GET", ["yonetim", "personel"]) => {
                if !i.yerel {
                    return Yanit::hata(403, "yalnız bu bilgisayardan");
                }
                Yanit::yonlendir("/masaustu#personel")
            }
            ("GET", ["api", "yonetim", "hesaplar"]) => self.hesap_listesi(i),
            ("POST", ["api", "yonetim", "hesap", tg, islem]) => self.hesap_islemi(i, tg, islem),
            ("GET" | "POST", ["api", ..]) => self.yetkili(i, &parca),
            _ => Yanit::hata(404, "bulunamadı"),
        }
    }

    /// Fişe eklenmiş fotoğraf sayısı. Fiş no yalnızca harf/rakam/tire ise sayılır (yol kaçışına karşı).
    fn foto_sayisi(&self, no: &str) -> usize {
        if no.is_empty() || !no.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            return 0;
        }
        (1..=EN_COK_FOTO)
            .filter(|n| {
                self.dizin
                    .join("fotolar")
                    .join(format!("{no}-{n}.jpg"))
                    .exists()
            })
            .count()
    }

    /// Fiş hazır olunca bilgilendirme kuyruğuna yazar; sonucu kısa metin olarak döndürür.
    /// Müşteri takip sayfasına gidecek durumu kuyruğa yazar (özellik kapalıysa bir şey yapmaz).
    /// Hata fişi engellemez: yalnız günlüğe yazılır, gönderim arka planda yeniden denenir.
    fn takibe_yaz(&self, f: &taa_fis::Fis) {
        if let Err(e) = musteri_takip::kuyruga_yaz(&self.dizin, f, self.simdi()) {
            eprintln!("Müşteri takip kuyruğuna yazılamadı: {e}");
        }
    }

    /// Müşteri nüshasına basılacak takip adresleri (özellik kapalıysa `None`).
    pub(crate) fn takip_adresleri(&self, no: &str) -> Option<(String, Option<String>)> {
        let a = musteri_takip::Ayar::dizinden(&self.dizin)?;
        let kod = musteri_takip::takip_kodu(&self.dizin, no).ok()?;
        Some((a.adres(&kod), a.yedek_adres(&kod)))
    }

    fn hazir_bildirimi(&self, d: &Defter, no: &str) -> serde_json::Value {
        let Ok(Some(f)) = d.bul(no) else {
            return json!(null);
        };
        let takip = self.takip_adresleri(no).map(|(a, _)| a);
        json!(taa_bildirim::hazir_bildir_takipli(
            &self.dizin,
            no,
            &f.telefon,
            f.bildirim_izni,
            &tarih_iso(self.simdi()),
            takip.as_deref()
        ))
    }

    /// Telegram webhook'u: adreste gizli parça olmalı; özellik kapalıyken ya da gizli yanlışken 404 (varlığı belli etmez).
    fn telegram(&self, gizli: &str, i: &Istek) -> Yanit {
        if !taa_ayar::ozellik_acik("telegram-bot", false) {
            return Yanit::hata(404, "bulunamadı");
        }
        let Ok(ayar) = taa_telegram::Ayar::dizinden(&self.dizin) else {
            return Yanit::hata(404, "bulunamadı");
        };
        if !ayar.gizli_dogru(gizli) {
            return Yanit::hata(404, "bulunamadı");
        }
        let Ok(g) = serde_json::from_slice::<serde_json::Value>(&i.govde) else {
            return Yanit::hata(400, "geçersiz güncelleme");
        };
        let bugun = tarih_iso(self.simdi());
        let bot = taa_telegram::Bot {
            dizin: &self.dizin,
            ayar: &ayar,
            bugun: &bugun,
            simdi: self.simdi(),
        };
        match bot.yanit(&g) {
            Some(y) => Yanit::json(200, y),
            None => Yanit::json(200, json!({})),
        }
    }

    fn eslestir(&self, i: &Istek) -> Yanit {
        if let Some(k) = kapali("telefon-eslestirme") {
            return k;
        }
        let v: serde_json::Value = match serde_json::from_slice(&i.govde) {
            Ok(v) => v,
            Err(_) => return Yanit::hata(400, "geçersiz istek"),
        };
        let (kod, ad) = (
            v["kod"].as_str().unwrap_or(""),
            v["ad"].as_str().unwrap_or(""),
        );
        let kisi_id = match self.kod_tuket(kod) {
            Ok(k) => k,
            Err(e) => return Yanit::hata(403, &e),
        };
        let ad = ad.trim();
        if ad.is_empty() {
            return Yanit::hata(400, "telefon adı yazılmalı");
        }
        let (id, kod2) = match (cihaz::yeni_token(), cihaz::yeni_kod2()) {
            (Ok(t), Ok(k)) => (t[..16].to_string(), k),
            (Err(e), _) | (_, Err(e)) => return Yanit::hata(500, &e),
        };
        let simdi = self.simdi();
        let mut b = match self.bekleyen.lock() {
            Ok(b) => b,
            Err(_) => return Yanit::hata(500, "kilit hatası"),
        };
        b.retain(|x| x.son > simdi);
        b.push(Bekleyen {
            id: id.clone(),
            ad: ad.into(),
            kisi_id,
            kod2: kod2.clone(),
            son: simdi + KOD_OMRU,
            tamam: None,
        });
        Yanit::json(200, json!({ "id": id, "kod2": kod2 }))
    }

    /// Telefon ikinci kodun onaylanıp onaylanmadığını yoklar; belirteç yalnız bir kez verilir.
    fn eslestir_durumu(&self, id: &str) -> Yanit {
        let simdi = self.simdi();
        let mut b = match self.bekleyen.lock() {
            Ok(b) => b,
            Err(_) => return Yanit::hata(500, "kilit hatası"),
        };
        b.retain(|x| x.son > simdi);
        match b.iter().position(|x| x.id == id) {
            None => Yanit::hata(
                410,
                "eşleştirme iptal oldu ya da süresi doldu; baştan başlayın",
            ),
            Some(n) if b[n].tamam.is_none() => Yanit::json(200, json!({ "durum": "bekliyor" })),
            Some(n) => {
                let x = b.remove(n);
                let (token, cid) = x.tamam.unwrap_or_default();
                Yanit::json(200, json!({ "durum": "tamam", "token": token, "id": cid }))
            }
        }
    }

    /// Yönetici, telefonda görünen ikinci kodu bu bilgisayarda (masaüstü "Telefonlar" ekranı) yazar.
    /// Yalnız yerel ve yönetici oturumuyla. 5 yanlışta bekleyenler iptal.
    pub(crate) fn eslestir_onayla(&self, i: &Istek) -> Yanit {
        if !i.yerel {
            return Yanit::hata(403, "eşleştirme yalnız bu bilgisayardan onaylanır");
        }
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let kod2 = v["kod2"].as_str().unwrap_or("").trim().to_string();
        let simdi = self.simdi();
        let (mut b, mut y) = match (self.bekleyen.lock(), self.yanlis2.lock()) {
            (Ok(b), Ok(y)) => (b, y),
            _ => return Yanit::hata(500, "kilit hatası"),
        };
        b.retain(|x| x.son > simdi);
        let Some(n) = b.iter().position(|x| x.tamam.is_none() && x.kod2 == kod2) else {
            *y += 1;
            if *y >= EN_COK_YANLIS {
                *y = 0;
                b.clear();
                return Yanit::hata(
                    403,
                    "5 yanlış deneme: eşleştirme iptal edildi, telefonda baştan başlayın",
                );
            }
            return Yanit::hata(403, "ikinci kod yanlış ya da süresi dolmuş");
        };
        *y = 0;
        let token = match cihaz::yeni_token() {
            Ok(t) => t,
            Err(e) => return Yanit::hata(500, &e),
        };
        // Cihaz adını yönetici bu ekranda verir; telefonun önerdiği ad yalnız ön değerdir.
        if let Some(ad) = v["ad"].as_str().map(str::trim).filter(|a| !a.is_empty()) {
            b[n].ad = ad.chars().take(40).collect();
        }
        match self
            .cihazlar()
            .ekle(&b[n].ad, &token, &tarih_iso(simdi), &b[n].kisi_id)
        {
            Ok(c) => {
                // Kayıt: kim, hangi cihaz, şube, saat. Kodlar yazılmaz.
                let sube = self.sube_kodu().unwrap_or_default();
                let satir = json!({"olay": "eslestirme", "cihaz": c.id, "ad": b[n].ad, "kisi": b[n].kisi_id, "sube": sube, "tarih": tarih_iso(simdi), "saat": saat_iso(simdi)}).to_string();
                let _ = taa_dosya::satir_ekle(&self.dizin.join("eslestirme.jsonl"), &satir);
                b[n].tamam = Some((token, c.id.clone()));
                Yanit::json(200, json!({ "tamam": true, "cihaz": c.id, "ad": b[n].ad }))
            }
            Err(e) => Yanit::hata(400, &e),
        }
    }

    /// TaaTerziTakip listeleri (salt okunur): özet, kargo, kuru-temizleme, geciken, bugun, usta.
    fn takip(&self, d: &taa_fis::Defter, ad: &str) -> Yanit {
        let bugun = tarih_iso(self.simdi());
        let t = match taa_takip::Takip::defterden(d) {
            Ok(t) => t,
            Err(e) => return Yanit::hata(409, &e),
        };
        let liste = |v: Vec<&taa_fis::Fis>| {
            let satirlar: Vec<_> = v
                .iter()
                .map(|f| {
                    json!({
                        "no": f.no, "musteri": f.musteri, "telefon": f.telefon,
                        "durum": f.durum, "teslim": f.teslim, "teslim_saati": f.teslim_saati,
                        "usta": f.usta, "raf": f.raf,
                        "urunler": f.kalemler.iter().map(|k| format!("{} - {}", k.urun, k.islem)).collect::<Vec<_>>(),
                        "kargo_ad": f.kargo_ad, "kargo_adres": f.kargo_adres, "kargo_telefon": f.kargo_telefon,
                    })
                })
                .collect();
            Yanit::json(200, json!({ "sayi": satirlar.len(), "fisler": satirlar }))
        };
        match ad {
            "ozet" => Yanit::json(200, json!(t.ozet(&bugun))),
            "kargo" => liste(t.kargo_bekleyen()),
            "kuru-temizleme" => liste(t.kuru_temizleme()),
            "geciken" => liste(t.geciken(&bugun)),
            "bugun" => liste(t.bugun_teslim(&bugun)),
            "usta" => Yanit::json(200, json!({ "ustalar": t.usta_performansi(&bugun) })),
            "usta-is" => match taa_takip::Zorluk::dizinden(&self.dizin) {
                Ok(z) => Yanit::json(200, json!({ "ustalar": t.usta_is_raporu(&z) })),
                Err(e) => Yanit::hata(409, &e),
            },
            _ => Yanit::hata(404, "bilinmeyen takip listesi"),
        }
    }

    fn puan_hata(e: String) -> Yanit {
        if e.contains("kapatılmış") {
            Yanit::hata(503, &e)
        } else {
            Yanit::hata(422, &e)
        }
    }

    /// Usta QR okutur: fişin (belirtilmezse tüm) kalemleri "bitti" olarak kaydedilir. Norm süre okutma anında yazılır.
    /// Gövde: {"kalemler":[0,1], "zor":[0]}; hepsi isteğe bağlı. Usta = okutan telefonun adı.
    fn okut(&self, d: &Defter, no: &str, cihaz_ad: &str, i: &Istek) -> Yanit {
        let f = match d.bul(no) {
            Ok(Some(f)) => f,
            Ok(None) => return Yanit::hata(404, "fiş yok"),
            Err(e) => return Yanit::hata(500, &e),
        };
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let idx = |anahtar: &str| -> Option<Vec<usize>> {
            v[anahtar].as_array().map(|a| {
                a.iter()
                    .filter_map(|x| x.as_u64().map(|n| n as usize))
                    .collect()
            })
        };
        let kalemler = idx("kalemler").unwrap_or_else(|| (0..f.kalemler.len()).collect());
        let zor = idx("zor").unwrap_or_default();
        // Yusuf kararı (2026-11-07): işi yapan usta okutur; puan satırı okutan telefonun adıdır (elle ad ya da fişteki usta kullanılmaz).
        let usta = cihaz_ad;
        let sure = match taa_takip::SureTablosu::dizinden(&self.dizin) {
            Ok(s) => s,
            Err(e) => return Yanit::hata(409, &e),
        };
        let defter = taa_takip::PuanDefteri::dizinden(&self.dizin);
        let simdi = self.simdi();
        let tarih = tarih_iso(simdi);
        let yerel = simdi + 3 * 3600; // tarih_iso ile aynı saat dilimi (UTC+3)
        let saat = format!("{:02}:{:02}", yerel % 86_400 / 3600, yerel % 3600 / 60);
        let mut okutulan = Vec::new();
        let mut zaten = Vec::new();
        for n in kalemler {
            let Some(k) = f.kalemler.get(n) else {
                return Yanit::hata(422, &format!("{} numaralı kalem yok", n + 1));
            };
            let olay = taa_takip::Olay {
                usta: usta.to_string(),
                fis_no: f.no.clone(),
                kalem_no: n,
                islem_kodu: k.islem_kodu.clone(),
                norm_dk: sure.kalem_dk(k, zor.contains(&n)),
                tarih: tarih.clone(),
                saat: saat.clone(),
            };
            match defter.olay_ekle(olay) {
                Ok(()) => okutulan.push(n),
                Err(e) if e.contains("zaten okutulmuş") => zaten.push(n),
                Err(e) => return Self::puan_hata(e),
            }
        }
        Yanit::json(200, json!({ "okutulan": okutulan, "zaten": zaten }))
    }

    /// Tezgâhtar yeniden iş / müşteri şikâyeti işaretler. Usta fişteki ustadır; tutanağı yönetici yazar.
    fn isaret(&self, d: &Defter, no: &str, cihaz_ad: &str, i: &Istek) -> Yanit {
        let f = match d.bul(no) {
            Ok(Some(f)) => f,
            Ok(None) => return Yanit::hata(404, "fiş yok"),
            Err(e) => return Yanit::hata(500, &e),
        };
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let tur = match v["tur"].as_str() {
            Some("yeniden_is") => taa_takip::IsaretTuru::YenidenIs,
            Some("sikayet") => taa_takip::IsaretTuru::Sikayet,
            _ => return Yanit::hata(422, "tur yeniden_is ya da sikayet olmalı"),
        };
        let kalem = v["kalem"].as_u64().unwrap_or(0) as usize;
        if kalem >= f.kalemler.len() {
            return Yanit::hata(422, "kalem numarası geçersiz");
        }
        // Usta: önce okutan usta (QR kaydı), yoksa fişteki usta.
        let defter = taa_takip::PuanDefteri::dizinden(&self.dizin);
        let okutan = defter.oku().ok().and_then(|k| {
            k.into_iter().find_map(|x| match x {
                taa_takip::Kayit::Olay(o) if o.fis_no == f.no && o.kalem_no == kalem => {
                    Some(o.usta)
                }
                _ => None,
            })
        });
        let usta = okutan.unwrap_or_else(|| f.usta.clone());
        if usta.trim().is_empty() {
            return Yanit::hata(
                422,
                "bu kalemde usta bilinmiyor (okutulmamış ve fişte usta yok)",
            );
        }
        let isaret = taa_takip::Isaret {
            id: 0,
            fis_no: f.no.clone(),
            kalem_no: kalem,
            usta,
            tur,
            neden: v["neden"].as_str().unwrap_or("").to_string(),
            tezgah: cihaz_ad.to_string(),
            tarih: tarih_iso(self.simdi()),
            tutanak: None,
        };
        match defter.isaretle(isaret) {
            Ok(id) => Yanit::json(200, json!({ "id": id })),
            Err(e) => Self::puan_hata(e),
        }
    }

    /// Yönetici tutanağı (yalnız bu bilgisayardan). Gövde: {isaret_id, sonuc: usta_kaynakli|usta_kaynakli_degil|kismen, yuzde, agir, aciklama, yonetici}.
    fn puan_tutanak(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let v: serde_json::Value = match serde_json::from_slice(&i.govde) {
            Ok(v) => v,
            Err(e) => return Yanit::hata(400, &format!("geçersiz gövde: {e}")),
        };
        let sonuc = match v["sonuc"].as_str() {
            Some("usta_kaynakli") => taa_takip::Sonuc::UstaKaynakli,
            Some("usta_kaynakli_degil") => taa_takip::Sonuc::UstaKaynakliDegil,
            Some("kismen") => taa_takip::Sonuc::Kismen {
                yuzde: v["yuzde"].as_u64().unwrap_or(0).min(255) as u8,
            },
            _ => {
                return Yanit::hata(
                    422,
                    "sonuc usta_kaynakli, usta_kaynakli_degil ya da kismen olmalı",
                )
            }
        };
        let t = taa_takip::Tutanak {
            sonuc,
            agir: v["agir"].as_bool().unwrap_or(false),
            aciklama: v["aciklama"].as_str().unwrap_or("").to_string(),
            yonetici: v["yonetici"].as_str().unwrap_or("").to_string(),
            tarih: tarih_iso(self.simdi()),
        };
        let defter = taa_takip::PuanDefteri::dizinden(&self.dizin);
        match defter.tutanak_yaz(v["isaret_id"].as_u64().unwrap_or(0), t) {
            Ok(()) => Yanit::json(200, json!({ "tamam": true })),
            Err(e) => Yanit::hata(422, &e),
        }
    }

    /// Tutanak bekleyenler ve yazılmış tutanaklar (yalnız bu bilgisayardan, `usta-puan` açıkken).
    fn puan_isaretleri(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        if !taa_ayar::ozellik_acik("usta-puan", false) {
            return Yanit::hata(503, &taa_ayar::kapali_mesaji("usta-puan"));
        }
        match taa_takip::PuanDefteri::dizinden(&self.dizin).oku() {
            Ok(k) => Yanit::json(200, json!({ "isaretler": taa_takip::puan_isaretleri(&k) })),
            Err(e) => Yanit::hata(500, &e),
        }
    }

    fn puan_girdisi(&self) -> Result<PuanGirdisi, Yanit> {
        let ayar = taa_takip::PuanAyari::dizinden(&self.dizin).map_err(|e| Yanit::hata(409, &e))?;
        let sure =
            taa_takip::SureTablosu::dizinden(&self.dizin).map_err(|e| Yanit::hata(409, &e))?;
        let kayitlar = taa_takip::PuanDefteri::dizinden(&self.dizin)
            .oku()
            .map_err(|e| Yanit::hata(500, &e))?;
        let defter_yolu = self.fis_yolu();
        let d = Defter { yol: &defter_yolu };
        let teslimler = d
            .fisler()
            .map_err(|e| Yanit::hata(500, &e))?
            .into_iter()
            .map(|f| (f.no, f.teslim))
            .collect();
        Ok((ayar, sure, kayitlar, teslimler))
    }

    /// Yönetici raporu: puanlar, yorumlar ve anomaliler. Yalnız bu bilgisayardan; usta ekranına ASLA gitmez.
    fn puan_raporu(&self, i: &Istek, bas: &str, bit: &str) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let (ayar, sure, kayitlar, teslimler) = match self.puan_girdisi() {
            Ok(g) => g,
            Err(y) => return y,
        };
        let puanlar = match taa_takip::puan_hesapla(&ayar, &sure, &kayitlar, &teslimler, bas, bit) {
            Ok(p) => p,
            Err(e) => return Self::puan_hata(e),
        };
        let bugun = tarih_iso(self.simdi());
        let anomaliler = taa_takip::anomaliler(&ayar, &sure, &kayitlar, &bugun).unwrap_or_default();
        Yanit::json(
            200,
            json!({ "ayar": ayar, "puanlar": puanlar, "anomaliler": anomaliler }),
        )
    }

    /// Veriye göre ayar önerisi; `otomatik` açıksa uygular, kaydeder ve deftere yazar.
    fn puan_gozden_gecir(&self, i: &Istek, bas: &str, bit: &str) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let (ayar, sure, kayitlar, teslimler) = match self.puan_girdisi() {
            Ok(g) => g,
            Err(y) => return y,
        };
        let puanlar = match taa_takip::puan_hesapla(&ayar, &sure, &kayitlar, &teslimler, bas, bit) {
            Ok(p) => p,
            Err(e) => return Self::puan_hata(e),
        };
        let olay_sayisi = kayitlar
            .iter()
            .filter(|k| matches!(k, taa_takip::Kayit::Olay(_)))
            .count();
        let ilk = kayitlar.iter().find_map(|k| match k {
            taa_takip::Kayit::Olay(o) => Some(o.tarih.clone()),
            _ => None,
        });
        let bugun = tarih_iso(self.simdi());
        let veri_gun = ilk.map_or(0, |t| taa_takip::gun_farki(&t, &bugun));
        match taa_takip::gozden_gecir(&ayar, &puanlar, olay_sayisi, veri_gun, &kayitlar, &bugun) {
            None => Yanit::json(200, json!({ "degisti": false })),
            Some(o) => {
                if let Err(e) = o.yeni.kaydet(&self.dizin) {
                    return Yanit::hata(500, &e);
                }
                if let Err(e) = taa_takip::PuanDefteri::dizinden(&self.dizin)
                    .ayar_degisti(&bugun, o.degisiklikler.clone())
                {
                    return Yanit::hata(500, &e);
                }
                Yanit::json(
                    200,
                    json!({ "degisti": true, "degisiklikler": o.degisiklikler }),
                )
            }
        }
    }

    /// Fiş düzeltme yetkisi: varsayılan yalnız mağaza bilgisayarı; yönetici her doğrulanmış cihaza verip geri alabilir.
    fn duzeltme_izni(&self, cihaz: &cihaz::Cihaz) -> Option<Yanit> {
        if self.cihazlar().yetki_var(&cihaz.id, "duzelt") {
            return None;
        }
        Some(Yanit::json(
            403,
            json!({
                "hata": "yetki_yok",
                "ileti": "Bu cihazın fiş düzeltme yetkisi yok. Yönetici, mağaza bilgisayarındaki Telefonlar ekranından yetki verebilir.",
            }),
        ))
    }

    fn yetkili(&self, i: &Istek, parca: &[&str]) -> Yanit {
        let token = i
            .yetki
            .as_deref()
            .and_then(|h| h.strip_prefix("Bearer "))
            .unwrap_or("");
        let cihaz = if token.is_empty() && i.yerel && i.masaustu {
            // Mağaza bilgisayarındaki masaüstü program: belirteç yerine "bu bilgisayar" kuralı.
            cihaz::Cihaz {
                id: PC_ID.into(),
                ad: "Mağaza PC".into(),
                token_sha256: String::new(),
                tarih: String::new(),
                kisi_id: String::new(),
            }
        } else {
            match self.cihazlar().dogrula(token) {
                Some(c) => c,
                None => return Yanit::hata(401, "yetkisiz telefon"),
            }
        };
        if let Ok(mut m) = self.son_gorulme.lock() {
            m.insert(cihaz.id.clone(), self.simdi());
        }
        let defter_yolu = self.fis_yolu();
        let d = Defter { yol: &defter_yolu };
        match (i.metot.as_str(), parca) {
            ("GET", ["api", "oturum"]) => self.oturum(&cihaz),
            ("GET", ["api", "kisiler"]) => Yanit::json(
                200,
                json!({ "kisiler": kisi::secilebilir(&self.dizin, &self.sube_kodu().unwrap_or_default()) }),
            ),
            ("GET", ["api", "durum"]) => self.genel_durum(),
            ("GET", ["api", "fis", no, "karekod"]) => match d.bul(no) {
                Ok(Some(f)) => match karekod_svg(&self.fis_karekod_adresi(&f.no)) {
                    Ok(svg) => Yanit::tur("image/svg+xml", svg),
                    Err(e) => Yanit::hata(500, &e),
                },
                Ok(None) => Yanit::hata(404, "fiş yok"),
                Err(e) => Yanit::hata(500, &e),
            },
            ("POST", ["api", "yazici", "deneme"]) => {
                let Some(yazici) = &self.yazici_yolu else {
                    return Yanit::hata(
                        409,
                        "yazıcı ayarlı değil (sunucuyu --yazici ile başlatın)",
                    );
                };
                match yaziciya_yaz(yazici, &deneme_baytlari()) {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true })),
                    Err(e) => Yanit::hata(502, &format!("yazıcıya yazılamadı: {e}")),
                }
            }
            ("POST", ["api", "fis"]) => {
                let mut g: Girdi = match serde_json::from_slice(&i.govde) {
                    Ok(g) => g,
                    Err(e) => return Yanit::hata(400, &format!("geçersiz fiş: {e}")),
                };
                if let Err(y) = self.kisileri_denetle(&mut g, &cihaz) {
                    return y;
                }
                g.tarih = tarih_iso(self.simdi());
                // Şube kodu sunucudan gelir; istemci başka bir şube beyan ederse reddedilir (409).
                let sube = match self.sube_kodu() {
                    Ok(k) => k,
                    Err(_) => {
                        return Yanit::json(
                            503,
                            json!({ "hata": self.sube_yok_iletisi(), "sube_yok": true }),
                        )
                    }
                };
                if !g.sube.trim().is_empty() && g.sube.trim() != sube {
                    return Yanit::hata(409, "sube_uyusmaz: fiş bu sunucunun şubesine yazılır");
                }
                g.sube = sube;
                match d.ekle_uyarili(g) {
                    Ok((f, uyarilar)) => {
                        self.takibe_yaz(&f);
                        Yanit::json(200, json!({ "no": f.no, "uyarilar": uyarilar }))
                    }
                    Err(e) => Yanit::hata(422, &e),
                }
            }
            ("GET", ["api", "renkler"]) => Yanit::json(
                200,
                json!({ "renkler": taa_fis::RENKLER
                    .iter()
                    .map(|(k, a)| json!({ "anahtar": k, "ad": a }))
                    .collect::<Vec<_>>() }),
            ),
            ("GET", ["api", "islemler"]) => match taa_terzi::Katalog::dizinden(&self.dizin) {
                Ok(k) => Yanit::json(200, json!(k)),
                Err(e) => Yanit::hata(500, &e),
            },
            ("POST", ["api", "fis", no, "kalem"])
            | ("POST", ["api", "fis", no, "kalem", _, "cikar"])
            | ("POST", ["api", "fis", no, "teslim-sekli"])
            | ("POST", ["api", "fis", no, "bagla"])
            | ("POST", ["api", "fis", no, "bag-coz"]) => {
                if let Some(y) = self.duzeltme_izni(&cihaz) {
                    return y;
                }
                let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
                let kim = if cihaz.kisi_id.is_empty() {
                    cihaz.ad.clone()
                } else {
                    cihaz.kisi_id.clone()
                };
                let tarih = tarih_iso(self.simdi());
                let s = |k: &str| v[k].as_str().unwrap_or("").to_string();
                let sonuc: Result<serde_json::Value, String> = match parca {
                    [_, _, _, "kalem"] => {
                        match serde_json::from_value::<taa_fis::Kalem>(v["kalem"].clone()) {
                            Ok(k) => d
                                .kalem_ekle(no, k, &kim, &tarih)
                                .map(|u| json!({ "uyarilar": u })),
                            Err(e) => Err(format!("geçersiz kalem: {e}")),
                        }
                    }
                    [_, _, _, "kalem", sira, _] => match sira.parse::<usize>() {
                        Ok(n) => {
                            // İstemci gördüğü kalemi (ürün/işlem) de gönderir: arada başka cihaz kalem
                            // çıkardıysa sıra kaymış olabilir, yanlış kalem çıkarılmasın (409).
                            let beklenen = (s("urun"), s("islem"));
                            let kaymis = (!beklenen.0.is_empty() || !beklenen.1.is_empty())
                                && d.bul(no).ok().flatten().is_some_and(|f| {
                                    f.kalemler.get(n).is_none_or(|k| {
                                        k.urun != beklenen.0 || k.islem != beklenen.1
                                    })
                                });
                            if kaymis {
                                return Yanit::hata(
                                    409,
                                    "fiş bu arada değişti; listeyi yenileyip yeniden deneyin",
                                );
                            }
                            d.kalem_cikar(no, n, &s("neden"), &kim, &tarih)
                                .map(|_| json!({}))
                        }
                        Err(_) => Err("kalem sırası sayı olmalı".into()),
                    },
                    [_, _, _, "teslim-sekli"] => d
                        .teslim_sekli(
                            no,
                            v["kargo"].as_bool().unwrap_or(false),
                            &s("kargo_ad"),
                            &s("kargo_adres"),
                            &s("kargo_telefon"),
                            &s("kargo_notu"),
                            &s("neden"),
                            &kim,
                            &tarih,
                        )
                        .map(|_| json!({})),
                    [_, _, _, "bagla"] => d.bagla(no, &s("diger"), &kim, &tarih).map(|_| json!({})),
                    _ => d.bag_coz(no, &kim, &tarih).map(|_| json!({})),
                };
                match sonuc {
                    Ok(mut j) => {
                        if let Ok(Some(f)) = d.bul(no) {
                            self.takibe_yaz(&f);
                        }
                        j["tamam"] = json!(true);
                        Yanit::json(200, j)
                    }
                    Err(e) => Yanit::hata(422, &e),
                }
            }
            ("POST", ["api", "fis", no, "teslim"]) => {
                let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
                if let Some(y) = self.duzeltme_izni(&cihaz) {
                    return y;
                }
                match d.teslim_degistir(
                    no,
                    v["yeni"].as_str().unwrap_or(""),
                    v["saat"].as_str().unwrap_or("17:00"),
                    v["neden"].as_str().unwrap_or(""),
                    &tarih_iso(self.simdi()),
                ) {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true })),
                    Err(e) => Yanit::hata(422, &e),
                }
            }
            ("GET", ["api", "fisler"]) => match d.fisler() {
                Ok(f) => {
                    let h = kisi::ad_haritasi(&self.dizin, &self.sube_kodu().unwrap_or_default());
                    let v: Vec<_> = f.into_iter().map(|f| fis_json(f, &h)).collect();
                    Yanit::json(200, json!({ "fisler": v }))
                }
                Err(e) => Yanit::hata(500, &e),
            },
            ("POST", ["api", "fis", no, "durum"]) => {
                let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
                let yeni = match v["durum"].as_str() {
                    Some("terzihanede") => Durum::Terzihanede,
                    Some("hazir") => Durum::Hazir,
                    Some("teslim") => Durum::Teslim,
                    Some("iptal") => Durum::Iptal,
                    _ => return Yanit::hata(400, "durum: terzihanede, hazir, teslim ya da iptal"),
                };
                // Teslim bu uçtan yapılmaz: teslim alan adı, ilişki, fişsizde son 4 + imza ve fişsiz teslim
                // yetkisi atlanmasın diye istemci POST /api/fis/<no>/teslim-al'a yönlendirilir (teslim_al.rs).
                if yeni == Durum::Teslim {
                    return Yanit::json(
                        409,
                        json!({
                            "hata": "teslim_al_gerekli",
                            "ileti": "Teslim, teslim alan bilgisiyle yapılır: teslim kutusunu kullanın.",
                        }),
                    );
                }
                match d.durum_ata(no, yeni, &tarih_iso(self.simdi())) {
                    Ok(()) => {
                        if let Ok(Some(f)) = d.bul(no) {
                            self.takibe_yaz(&f);
                        }
                        // Hazır + müşteri izni varsa hazır mesajı kuyruğa yazılır (yalnız fiş no, KVKK).
                        let mut mesaj = json!(null);
                        if yeni == Durum::Hazir {
                            mesaj = self.hazir_bildirimi(&d, no);
                        }
                        Yanit::json(200, json!({ "tamam": true, "bildirim": mesaj }))
                    }
                    Err(e) => Yanit::hata(409, &e),
                }
            }
            ("GET", ["api", "fis", no, "metin"]) => match d.bul(no) {
                Ok(Some(mut f)) => Yanit {
                    kod: 200,
                    tur: "text/plain; charset=utf-8",
                    govde: {
                        self.adlari_doldur(&mut f);
                        taa_fis::duz_metin(&f, 42).into_bytes()
                    },
                },
                Ok(None) => Yanit::hata(404, "fiş yok"),
                Err(e) => Yanit::hata(500, &e),
            },
            ("GET", ["api", "takip", ad]) => self.takip(&d, ad),
            ("GET", ["api", "bildirimler"]) => {
                let k = taa_bildirim::Kuyruk {
                    yol: &self.dizin.join("bildirimler.jsonl"),
                };
                match k.mesajlar() {
                    Ok(m) => Yanit::json(200, json!({ "mesajlar": m })),
                    Err(e) => Yanit::hata(500, &e),
                }
            }
            ("GET", ["api", "fis", no]) => match d.bul(no) {
                Ok(Some(f)) => {
                    let h = kisi::ad_haritasi(&self.dizin, &self.sube_kodu().unwrap_or_default());
                    let mut v = fis_json(f, &h);
                    v["foto_adet"] = json!(self.foto_sayisi(no));
                    Yanit::json(200, v)
                }
                Ok(None) => Yanit::hata(404, "fiş yok"),
                Err(e) => Yanit::hata(500, &e),
            },
            ("POST", ["api", "fis", no, "raf"]) => {
                let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
                match d.raf_ata(
                    no,
                    v["raf"].as_str().unwrap_or(""),
                    &cihaz.ad,
                    &tarih_iso(self.simdi()),
                ) {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true })),
                    Err(e) => Yanit::hata(422, &e),
                }
            }
            ("POST", ["api", "fis", no, "imza"]) => {
                let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
                // Mağaza telefonu/bilgisayarı usta imzası atamaz; okutan kişi seçilmeden imza olmaz.
                // Kişi kimliği havuz kişi kaydından gelecek (KM2); şimdilik biçimi denetlenir.
                let rol = v["rol"].as_str().unwrap_or("teslim-eden");
                // Kişisel telefonda (kullanıcı girişi) imzalayan her zaman giren kişidir.
                let okutan = if cihaz.kisi_id.is_empty() {
                    v["okutan"].as_str().unwrap_or("").trim()
                } else {
                    cihaz.kisi_id.as_str()
                };
                if okutan.is_empty() {
                    return Yanit::hata(400, "okutan personel seçilmeli");
                }
                if okutan.len() > 40
                    || !okutan
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                {
                    return Yanit::hata(400, "okutan kişi kimliği geçersiz");
                }
                let s = self.simdi();
                match d.imzala_kisili(
                    no,
                    &cihaz.ad,
                    rol,
                    &tarih_iso(s),
                    &saat_iso(s),
                    &cihaz.id,
                    "beyan",
                    okutan,
                ) {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true, "kimlik_duzeyi": "beyan" })),
                    Err(e) => Yanit::hata(422, &e),
                }
            }
            ("POST", ["api", "fis", no, "foto"]) => {
                if !taa_ayar::ozellik_acik("foto-eki", true) {
                    return Yanit::hata(409, &taa_ayar::kapali_mesaji("foto-eki"));
                }
                if !i.govde.starts_with(&[0xFF, 0xD8, 0xFF]) {
                    return Yanit::hata(400, "yalnız JPEG fotoğraf kabul edilir");
                }
                match d.bul(no) {
                    Ok(Some(_)) => {}
                    Ok(None) => return Yanit::hata(404, "fiş yok"),
                    Err(e) => return Yanit::hata(500, &e),
                }
                let n = self.foto_sayisi(no);
                if n >= EN_COK_FOTO {
                    return Yanit::hata(409, "bir fişe en çok 6 fotoğraf eklenir");
                }
                let klasor = self.dizin.join("fotolar");
                let yaz = std::fs::create_dir_all(&klasor).and_then(|_| {
                    taa_dosya::uzerine_yaz_atomik(
                        &klasor.join(format!("{no}-{}.jpg", n + 1)),
                        &i.govde,
                    )
                });
                match yaz {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true, "sira": n + 1 })),
                    Err(e) => Yanit::hata(500, &format!("fotoğraf kaydedilemedi: {e}")),
                }
            }
            ("GET", ["api", "fis", no, "foto"]) => {
                Yanit::json(200, json!({ "adet": self.foto_sayisi(no) }))
            }
            ("GET", ["api", "fis", no, "foto", n]) => {
                let n: usize = n.parse().unwrap_or(0);
                if n == 0 || n > self.foto_sayisi(no) {
                    return Yanit::hata(404, "fotoğraf yok");
                }
                match std::fs::read(self.dizin.join("fotolar").join(format!("{no}-{n}.jpg"))) {
                    Ok(b) => Yanit {
                        kod: 200,
                        tur: "image/jpeg",
                        govde: b,
                    },
                    Err(_) => Yanit::hata(404, "fotoğraf yok"),
                }
            }
            ("POST", ["api", "fis", no, "teslim-al"]) => self.teslim_al(&d, no, &cihaz, i),
            ("GET", ["api", "fis", no, "imza"]) => self.teslim_imzasi(&d, no, &cihaz),
            ("POST", ["api", "fis", no, "okut"]) => {
                if cihaz.id == PC_ID {
                    return Yanit::hata(403, "iş bitti okutması ustanın telefonundan yapılır");
                }
                self.okut(&d, no, &cihaz.ad, i)
            }
            ("POST", ["api", "fis", no, "isaret"]) => self.isaret(&d, no, &cihaz.ad, i),
            ("POST", ["api", "fis", no, "yazdir"]) => {
                let mut f = match d.bul(no) {
                    Ok(Some(f)) => f,
                    Ok(None) => return Yanit::hata(404, "fiş yok"),
                    Err(e) => return Yanit::hata(500, &e),
                };
                let Some(yazici) = &self.yazici_yolu else {
                    return Yanit::hata(
                        409,
                        "yazıcı ayarlı değil (sunucuyu --yazici ile başlatın)",
                    );
                };
                self.adlari_doldur(&mut f);
                let nusha: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
                let baytlar = if nusha["nusha"].as_u64() == Some(1) {
                    taa_fis::escpos(&f, 42)
                } else {
                    let qr = self.fis_karekod_adresi(&f.no);
                    let ayd = self.aydinlatma_url.clone().unwrap_or_else(|| {
                        format!("{}/aydinlatma", self.taban_url.trim_end_matches('/'))
                    });
                    let takip = self.takip_adresleri(&f.no);
                    let t = takip.as_ref().map(|(a, y)| taa_fis::Takip {
                        adres: a,
                        yedek: y.as_deref(),
                        cumle: musteri_takip::FIS_CUMLESI,
                    });
                    taa_fis::escpos_nushalar_takipli(&f, 42, &qr, Some(&ayd), t.as_ref())
                };
                match yaziciya_yaz(yazici, &baytlar) {
                    Ok(()) => Yanit::json(200, json!({ "tamam": true })),
                    Err(e) => Yanit::hata(502, &format!("yazıcıya yazılamadı: {e}")),
                }
            }
            _ => Yanit::hata(404, "bulunamadı"),
        }
    }
}

/// Baytları yazıcıya gönderir. Hedef `tcp://IP[:9100]` ise ağ (Ethernet) yazıcısıdır;
/// aksi halde dosya/paylaşım yoludur (USB yazıcı: `\\\\localhost\\ADI` ya da `/dev/usb/lp0`).
pub fn yaziciya_yaz(hedef: &Path, baytlar: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let h = hedef.to_string_lossy();
    if let Some(adres) = h.strip_prefix("tcp://") {
        use std::net::ToSocketAddrs;
        let adres = if adres.contains(':') {
            adres.to_string()
        } else {
            format!("{adres}:9100")
        };
        let a = adres
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| std::io::Error::other("yazıcı adresi çözülemedi"))?;
        let mut t = std::net::TcpStream::connect_timeout(&a, std::time::Duration::from_secs(5))?;
        t.set_write_timeout(Some(std::time::Duration::from_secs(10)))?;
        t.write_all(baytlar)?;
        t.flush()
    } else {
        std::fs::OpenOptions::new()
            .write(true)
            .open(hedef)
            .and_then(|mut f| f.write_all(baytlar))
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn uyg() -> Uygulama {
        let d = taa_dosya::gecici_klasor().unwrap();
        std::fs::write(d.join("sube.json"), r#"{"kod":"MRK01"}"#).unwrap();
        std::fs::write(d.join("kisiler.jsonl"), KISILER).unwrap();
        let mut u = Uygulama::yeni(d, None, "http://192.168.1.20:8787".into());
        u.yonetici_kilidi = false;
        u
    }

    /// Deneme kişileri (gerçek kişi değildir).
    const KISILER: &str = concat!(
        r#"{"tur":"ekle","v":{"id":"K-test0001","ad":"Deneme Danışman","rol":"danisman","tarih":"2026-11-01"}}"#,
        "\n",
        r#"{"tur":"ekle","v":{"id":"K-test0002","ad":"Deneme Usta","rol":"usta","tarih":"2026-11-01"}}"#,
        "\n"
    );
    const KISI_ALANLARI: &str = r#""satis_danismani_id":"K-test0001","olcu_alan_id":"K-test0002","duzenleyen_id":"K-test0001""#;

    fn ist(metot: &str, yol: &str, yetki: Option<&str>, govde: &str, yerel: bool) -> Istek {
        Istek {
            metot: metot.into(),
            yol: yol.into(),
            yetki: yetki.map(|t| format!("Bearer {t}")),
            govde: govde.as_bytes().to_vec(),
            yerel,
            ..Default::default()
        }
    }

    fn kod_al(u: &Uygulama) -> String {
        let y = u.isle(&ist("GET", "/api/yonetim/eslestirme", None, "", true));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let v: serde_json::Value = serde_json::from_slice(&y.govde).unwrap();
        v["kod"].as_str().unwrap().to_string()
    }

    fn esles(u: &Uygulama) -> String {
        let k = kod_al(u);
        kodla_esles(u, &k, "Ayşe'nin telefonu")
    }

    /// Çifte kod akışının tamamı: KOD 1 -> {id,kod2} -> yönetici onayı -> belirteç (bir kez).
    fn kodla_esles(u: &Uygulama, k: &str, ad: &str) -> String {
        let y = u.isle(&ist(
            "POST",
            "/api/eslestir",
            None,
            &format!(r#"{{"kod":"{k}","ad":"{ad}"}}"#),
            false,
        ));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j = serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap();
        assert!(j["token"].is_null(), "tek kodla belirteç verilmez");
        let (id, kod2) = (j["id"].as_str().unwrap(), j["kod2"].as_str().unwrap());
        // onaydan önce telefon beklemede
        let d = u.isle(&ist("GET", &format!("/api/eslestir/{id}"), None, "", false));
        assert!(String::from_utf8_lossy(&d.govde).contains("bekliyor"));
        // yerel olmayan istek onaylayamaz
        let o = u.isle(&ist(
            "POST",
            "/api/yonetim/eslestir/onayla",
            None,
            &format!(r#"{{"kod2":"{kod2}"}}"#),
            false,
        ));
        assert_eq!(o.kod, 403);
        let o = u.isle(&ist(
            "POST",
            "/api/yonetim/eslestir/onayla",
            None,
            &format!(r#"{{"kod2":"{kod2}"}}"#),
            true,
        ));
        assert_eq!(o.kod, 200, "{}", String::from_utf8_lossy(&o.govde));
        let d = u.isle(&ist("GET", &format!("/api/eslestir/{id}"), None, "", false));
        let t = serde_json::from_slice::<serde_json::Value>(&d.govde).unwrap()["token"]
            .as_str()
            .unwrap()
            .to_string();
        // belirteç yalnız bir kez verilir
        assert_eq!(
            u.isle(&ist("GET", &format!("/api/eslestir/{id}"), None, "", false))
                .kod,
            410
        );
        t
    }

    const FIS: &str = r#"{"satis_danismani_id":"K-test0001","olcu_alan_id":"K-test0002","duzenleyen_id":"K-test0001","musteri":"Ayşe Çelik","telefon":"0532 123 45 67","kalemler":[{"urun":"Pantolon","islem":"paça kısalt"}],"teslim":"2099-01-01","ucret_kurus":50000,"on_odeme_kurus":10000}"#;

    #[test]
    fn tarih() {
        assert_eq!(tarih_iso(0), "1970-01-01");
        assert_eq!(tarih_iso(1_730_757_600), "2024-11-05"); // 2024-11-04 22:00 UTC = 05 Kasım 01:00 TR
        assert_eq!(tarih_iso(951_782_400), "2000-02-29");
    }

    #[test]
    fn yonetim_yalniz_yerel() {
        let u = uyg();
        assert_eq!(u.isle(&ist("GET", "/yonetim", None, "", false)).kod, 403);
        assert_eq!(u.isle(&ist("GET", "/masaustu", None, "", false)).kod, 403);
        assert_eq!(
            u.isle(&ist("GET", "/api/yonetim/eslestirme", None, "", false))
                .kod,
            403
        );
        // Eski adres masaüstü programa yönlenir.
        let y = u.isle(&ist("GET", "/yonetim", None, "", true));
        assert!(String::from_utf8(y.govde)
            .unwrap()
            .contains("/masaustu#telefonlar"));
        let y = u.isle(&ist("GET", "/api/yonetim/eslestirme", None, "", true));
        assert_eq!(y.kod, 200);
        let v: serde_json::Value = serde_json::from_slice(&y.govde).unwrap();
        assert!(v["qr_svg"].as_str().unwrap().contains("<svg"));
        assert!(v["url"]
            .as_str()
            .unwrap()
            .starts_with("http://192.168.1.20:8787/telefon?kod="));
        let m = u.isle(&ist("GET", "/masaustu", None, "", true));
        assert_eq!(m.kod, 200);
        assert!(String::from_utf8(m.govde).unwrap().contains("<html"));
    }

    #[test]
    fn fis_sube_kodu_sunucudan_gelir_ve_zorunludur() {
        let u = uyg();
        let t = esles(&u);
        // İstemci başka şube beyan ederse 409 ve hiçbir şey yazılmaz.
        let baska = FIS.replacen('{', r#"{"sube":"BRS01","#, 1);
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), &baska, false));
        assert_eq!(y.kod, 409, "{}", String::from_utf8_lossy(&y.govde));
        assert!(!u.dizin.join("fisler.jsonl").exists());
        // Aynı şubeyi beyan etmek serbest; kayıt sunucunun şube koduyla yazılır.
        let ayni = FIS.replacen('{', r#"{"sube":"MRK01","#, 1);
        assert_eq!(
            u.isle(&ist("POST", "/api/fis", Some(&t), &ayni, false)).kod,
            200
        );
        let l = u.isle(&ist("GET", "/api/fisler", Some(&t), "", false));
        let v: serde_json::Value = serde_json::from_slice(&l.govde).unwrap();
        assert_eq!(v["fisler"][0]["sube"], "MRK01");
        // sube.json yoksa fiş yazılamaz (503).
        std::fs::remove_file(u.dizin.join("sube.json")).unwrap();
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false));
        assert_eq!(y.kod, 503);
    }

    #[test]
    fn eslestirme_ve_fis_akisi() {
        let u = uyg();
        // yetkisiz
        assert_eq!(u.isle(&ist("GET", "/api/fisler", None, "", false)).kod, 401);
        assert_eq!(
            u.isle(&ist("GET", "/api/fisler", Some("sahte"), "", false))
                .kod,
            401
        );
        let t = esles(&u);
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let no = serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        let l = u.isle(&ist("GET", "/api/fisler", Some(&t), "", false));
        assert!(String::from_utf8(l.govde).unwrap().contains(&no));
        assert_eq!(
            u.isle(&ist(
                "POST",
                &format!("/api/fis/{no}/durum"),
                Some(&t),
                r#"{"durum":"teslim"}"#,
                false
            ))
            .kod,
            409
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                &format!("/api/fis/{no}/durum"),
                Some(&t),
                r#"{"durum":"hazir"}"#,
                false
            ))
            .kod,
            200
        );
        let m = u.isle(&ist(
            "GET",
            &format!("/api/fis/{no}/metin"),
            Some(&t),
            "",
            false,
        ));
        assert!(String::from_utf8(m.govde).unwrap().contains("Ayşe Çelik"));
        // geçersiz fiş
        assert_eq!(
            u.isle(&ist(
                "POST",
                "/api/fis",
                Some(&t),
                r#"{"musteri":"","telefon":"1","kalemler":[],"teslim":"x","ucret_kurus":1}"#,
                false
            ))
            .kod,
            422
        );
        assert_eq!(
            u.isle(&ist("POST", "/api/fis", Some(&t), "bozuk", false))
                .kod,
            400
        );
    }

    #[test]
    fn kod_tek_kullanimlik_ve_sureli() {
        let u = uyg();
        let k = kod_al(&u);
        let gov = format!(r#"{{"kod":"{k}","ad":"T1"}}"#);
        assert_eq!(
            u.isle(&ist("POST", "/api/eslestir", None, &gov, false)).kod,
            200
        );
        assert_eq!(
            u.isle(&ist("POST", "/api/eslestir", None, &gov, false)).kod,
            403
        ); // ikinci kullanım
        let k2 = kod_al(&u);
        u.ileri_sn
            .fetch_add(601, std::sync::atomic::Ordering::SeqCst);
        let gov2 = format!(r#"{{"kod":"{k2}","ad":"T2"}}"#);
        assert_eq!(
            u.isle(&ist("POST", "/api/eslestir", None, &gov2, false))
                .kod,
            403
        ); // süresi doldu
    }

    #[test]
    fn cifte_kod_yanlis_ikinci_kod_iptal_eder() {
        let u = uyg();
        let k = kod_al(&u);
        let y = u.isle(&ist(
            "POST",
            "/api/eslestir",
            None,
            &format!(r#"{{"kod":"{k}","ad":"x"}}"#),
            false,
        ));
        assert_eq!(y.kod, 200);
        let j = serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap();
        let id = j["id"].as_str().unwrap().to_string();
        let kod2 = j["kod2"].as_str().unwrap().to_string();
        assert_eq!(kod2.len(), 4);
        // birinci kod tek kullanımlık
        let yine = u.isle(&ist(
            "POST",
            "/api/eslestir",
            None,
            &format!(r#"{{"kod":"{k}","ad":"y"}}"#),
            false,
        ));
        assert_eq!(yine.kod, 403);
        let yanlis = if kod2 == "0000" { "0001" } else { "0000" };
        for _ in 0..5 {
            let o = u.isle(&ist(
                "POST",
                "/api/yonetim/eslestir/onayla",
                None,
                &format!(r#"{{"kod2":"{yanlis}"}}"#),
                true,
            ));
            assert_eq!(o.kod, 403);
        }
        // 5 yanlıştan sonra bekleyen eşleştirme iptal: doğru kod bile geçmez, telefon 410 alır
        let o = u.isle(&ist(
            "POST",
            "/api/yonetim/eslestir/onayla",
            None,
            &format!(r#"{{"kod2":"{kod2}"}}"#),
            true,
        ));
        assert_eq!(o.kod, 403);
        assert_eq!(
            u.isle(&ist("GET", &format!("/api/eslestir/{id}"), None, "", false))
                .kod,
            410
        );
        // kodlar kayda yazılmaz
        let log = std::fs::read_to_string(u.dizin.join("eslestirme.jsonl")).unwrap_or_default();
        assert!(!log.contains(&kod2) && !log.contains(&k));
    }

    #[test]
    fn yanlis_kod_kilitler() {
        let u = uyg();
        let k = kod_al(&u);
        let yanlis = if k == "000000" { "000001" } else { "000000" };
        for _ in 0..5 {
            assert_eq!(
                u.isle(&ist(
                    "POST",
                    "/api/eslestir",
                    None,
                    &format!(r#"{{"kod":"{yanlis}","ad":"x"}}"#),
                    false
                ))
                .kod,
                403
            );
        }
        // doğru kod bile kilit süresince reddedilir
        let y = u.isle(&ist(
            "POST",
            "/api/eslestir",
            None,
            &format!(r#"{{"kod":"{k}","ad":"x"}}"#),
            false,
        ));
        assert_eq!(y.kod, 403);
        assert!(String::from_utf8_lossy(&y.govde).contains("bekleyin"));
    }

    #[test]
    fn cihaz_kapatilinca_erisim_biter() {
        let u = uyg();
        let t = esles(&u);
        assert_eq!(
            u.isle(&ist("GET", "/api/fisler", Some(&t), "", false)).kod,
            200
        );
        let id = u.cihazlar().aktifler().unwrap()[0].id.clone();
        assert_eq!(
            u.isle(&ist(
                "POST",
                &format!("/api/yonetim/cihaz/{id}/kapat"),
                None,
                "",
                false
            ))
            .kod,
            403
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                &format!("/api/yonetim/cihaz/{id}/kapat"),
                None,
                "",
                true
            ))
            .kod,
            200
        );
        assert_eq!(
            u.isle(&ist("GET", "/api/fisler", Some(&t), "", false)).kod,
            401
        );
    }

    #[test]
    fn yazdirma() {
        let mut u = uyg();
        let t = esles(&u);
        let no = serde_json::from_slice::<serde_json::Value>(
            &u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false)).govde,
        )
        .unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        let yol = format!("/api/fis/{no}/yazdir");
        assert_eq!(u.isle(&ist("POST", &yol, Some(&t), "", false)).kod, 409); // yazıcı yok
        let p = u.dizin.join("yazici.bin");
        std::fs::write(&p, b"").unwrap();
        u.yazici_yolu = Some(p.clone());
        assert_eq!(u.isle(&ist("POST", &yol, Some(&t), "", false)).kod, 200);
        let b = std::fs::read(&p).unwrap();
        assert_eq!(&b[..2], &[0x1B, 0x40]);
        assert_eq!(
            b.windows(4).filter(|w| *w == [0x1D, 0x56, 0x42, 0]).count(),
            2
        ); // 1 terzihane + 1 müşteri
        u.yazici_yolu = Some(u.dizin.join("yok").join("x"));
        assert_eq!(u.isle(&ist("POST", &yol, Some(&t), "", false)).kod, 502);
    }

    /// Sanal yazıcı: yerel TCP dinleyici, ESC/POS baytlarını alır; CP857 ters çevrilip Türkçe karakterler doğrulanır.
    #[test]
    fn sanal_ag_yazicisi_turkce() {
        use std::io::Read;
        let dinle = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let port = dinle.local_addr().unwrap().port();
        let alici = std::thread::spawn(move || {
            let (mut c, _) = dinle.accept().unwrap();
            let mut v = Vec::new();
            c.read_to_end(&mut v).unwrap();
            v
        });
        let mut u = uyg();
        let t = esles(&u);
        let govde = FIS.replace("Ayşe Çelik", "Şükrü Ğüleç İÖ ığöçş");
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), &govde, false));
        let no = serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        u.yazici_yolu = Some(PathBuf::from(format!("tcp://127.0.0.1:{port}")));
        let yol = format!("/api/fis/{no}/yazdir");
        assert_eq!(u.isle(&ist("POST", &yol, Some(&t), "", false)).kod, 200);
        let baytlar = alici.join().unwrap();
        // CP857 -> metin
        let metin: String = baytlar
            .iter()
            .map(|b| match b {
                0x80 => 'Ç',
                0x81 => 'ü',
                0x87 => 'ç',
                0x8D => 'ı',
                0x98 => 'İ',
                0x99 => 'Ö',
                0x9A => 'Ü',
                0x94 => 'ö',
                0xA6 => 'Ğ',
                0xA7 => 'ğ',
                0x9E => 'Ş',
                0x9F => 'ş',
                b if b.is_ascii() => *b as char,
                _ => '\u{FFFD}',
            })
            .collect();
        assert!(metin.contains("Şükrü Ğüleç İÖ ığöçş"), "{metin}");
        assert!(metin.contains("TERZİHANE NÜSHASI") && metin.contains("MÜŞTERİ NÜSHASI"));
        assert!(!metin.contains('?') || !metin.contains("Müşteri Adı: ?"));
    }

    #[test]
    fn qr_okutma_isaret_tutanak_ve_puan_raporu() {
        let u = uyg();
        let t = esles(&u);
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false));
        let no = serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        let tel = |yol: &str, g: &str| u.isle(&ist("POST", yol, Some(&t), g, false));
        // Özellikler varsayılan kapalı.
        assert_eq!(tel(&format!("/api/fis/{no}/okut"), "{}").kod, 503);
        std::env::set_var("TAA_OZELLIK_QR_URETIM", "1");
        std::env::set_var("TAA_OZELLIK_USTA_PUAN", "1");
        let y = tel(&format!("/api/fis/{no}/okut"), "{}");
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let v: serde_json::Value = serde_json::from_slice(&y.govde).unwrap();
        assert_eq!(v["okutulan"], serde_json::json!([0]));
        // İkinci okutma reddedilmez ama "zaten" olarak döner.
        let v: serde_json::Value =
            serde_json::from_slice(&tel(&format!("/api/fis/{no}/okut"), "{}").govde).unwrap();
        assert_eq!(v["zaten"], serde_json::json!([0]));
        assert_eq!(
            tel(&format!("/api/fis/{no}/okut"), r#"{"kalemler":[7]}"#).kod,
            422
        );
        // Tezgâh işareti: usta, kalemi okutan telefonun adıdır.
        assert_eq!(
            tel(
                &format!("/api/fis/{no}/isaret"),
                r#"{"kalem":0,"tur":"x","neden":"a"}"#
            )
            .kod,
            422
        );
        let y = tel(
            &format!("/api/fis/{no}/isaret"),
            r#"{"kalem":0,"tur":"yeniden_is","neden":"dikiş"}"#,
        );
        assert_eq!(y.kod, 200);
        // Yönetici işlemleri yalnız yerel; telefon yetkisiyle de olmaz.
        let tut = r#"{"isaret_id":1,"sonuc":"kismen","yuzde":50,"aciklama":"yarı yarıya","yonetici":"Yusuf"}"#;
        assert_eq!(
            u.isle(&ist("POST", "/api/yonetim/tutanak", Some(&t), tut, false))
                .kod,
            403
        );
        assert_eq!(
            u.isle(&ist(
                "GET",
                "/api/yonetim/puan/2026-11-01/2026-11-30",
                Some(&t),
                "",
                false
            ))
            .kod,
            403
        );
        assert_eq!(
            u.isle(&ist("POST", "/api/yonetim/tutanak", None, tut, true))
                .kod,
            200
        );
        let bos =
            r#"{"isaret_id":1,"sonuc":"kismen","yuzde":50,"aciklama":" ","yonetici":"Yusuf"}"#;
        assert_eq!(
            u.isle(&ist("POST", "/api/yonetim/tutanak", None, bos, true))
                .kod,
            422
        );
        // Rapor: gömülü sure.json'da gunluk_dakika var; aralık 2099'a uzandığı için deneme dönemi biter, puan sayıdır.
        let r = u.isle(&ist(
            "GET",
            "/api/yonetim/puan/2026-01-01/2099-12-31",
            None,
            "",
            true,
        ));
        assert_eq!(r.kod, 200, "{}", String::from_utf8_lossy(&r.govde));
        let v: serde_json::Value = serde_json::from_slice(&r.govde).unwrap();
        assert_eq!(v["puanlar"][0]["usta"], "Ayşe'nin telefonu");
        assert!(
            v["puanlar"][0]["puan"].is_number(),
            "aralık deneme dönemini aşıyor"
        );
        let l = u.isle(&ist("GET", "/api/yonetim/isaretler", None, "", true));
        assert_eq!(l.kod, 200);
        let v: serde_json::Value = serde_json::from_slice(&l.govde).unwrap();
        assert_eq!(v["isaretler"][0]["tutanak"]["yonetici"], "Yusuf");
        assert_eq!(
            u.isle(&ist("GET", "/api/yonetim/isaretler", Some(&t), "", false))
                .kod,
            403
        );
        assert_eq!(
            u.isle(&ist("GET", "/yonetim/puan", Some(&t), "", false))
                .kod,
            403
        );
        assert_eq!(
            u.isle(&ist("GET", "/yonetim/puan", None, "", true)).kod,
            200
        );
        std::env::remove_var("TAA_OZELLIK_QR_URETIM");
        std::env::remove_var("TAA_OZELLIK_USTA_PUAN");
    }

    #[test]
    fn raf_imza_ve_qr_adresi() {
        let u = uyg();
        let t = esles(&u);
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false));
        let no = serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        let p = |yol: &str, g: &str| u.isle(&ist("POST", yol, Some(&t), g, false)).kod;
        assert_eq!(p(&format!("/api/fis/{no}/raf"), r#"{"raf":"a-12"}"#), 200);
        assert_eq!(p(&format!("/api/fis/{no}/raf"), r#"{"raf":""}"#), 422);
        let im = format!("/api/fis/{no}/imza");
        // okutan seçilmeden ya da usta rolüyle imza olmaz
        assert_eq!(p(&im, r#"{"rol":"teslim-eden"}"#), 400);
        assert_eq!(p(&im, r#"{"rol":"teslim-eden","okutan":"k 7!"}"#), 400);
        assert_eq!(p(&im, r#"{"rol":"usta","okutan":"k-7"}"#), 422);
        assert_eq!(p(&im, r#"{"rol":"teslim-eden","okutan":"k-7"}"#), 200);
        assert_eq!(p(&im, r#"{"rol":"teslim-eden","okutan":"k-7"}"#), 422);
        let g = u.isle(&ist("GET", &format!("/api/fis/{no}"), Some(&t), "", false));
        let j: serde_json::Value = serde_json::from_slice(&g.govde).unwrap();
        assert_eq!(j["raf"], "A-12");
        assert_eq!(j["imzalar"][0]["kim"], "Ayşe'nin telefonu");
        assert_eq!(
            u.isle(&ist("GET", "/api/fis/F-yok", Some(&t), "", false))
                .kod,
            404
        );
    }

    #[test]
    fn hazir_mesaji_kuyruga_yazilir() {
        let u = uyg();
        let t = esles(&u);
        let izinli = FIS.replace(r#""ucret_kurus""#, r#""bildirim_izni":true,"ucret_kurus""#);
        let kur = |g: &str| {
            let y = u.isle(&ist("POST", "/api/fis", Some(&t), g, false));
            serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap()["no"]
                .as_str()
                .unwrap()
                .to_string()
        };
        let (a, b) = (kur(&izinli), kur(FIS));
        for no in [&a, &b] {
            for st in ["terzihanede", "hazir"] {
                let y = u.isle(&ist(
                    "POST",
                    &format!("/api/fis/{no}/durum"),
                    Some(&t),
                    &format!(r#"{{"durum":"{st}"}}"#),
                    false,
                ));
                assert_eq!(y.kod, 200);
            }
        }
        let l = u.isle(&ist("GET", "/api/bildirimler", Some(&t), "", false));
        let j: serde_json::Value = serde_json::from_slice(&l.govde).unwrap();
        let m = j["mesajlar"].as_array().unwrap();
        assert_eq!(m.len(), 1); // yalnız izin veren müşteri
        let metin = m[0]["metin"].as_str().unwrap();
        assert!(metin.contains(a.as_str()) && !metin.contains("Ayşe") && !metin.contains("0532"));
        assert_eq!(m[0]["telefon"], "905321234567");
    }

    #[test]
    fn hesap_baglama_uclari_ve_wa_sayfasi() {
        let u = uyg();
        let kod = r#"{"rol":"personel","sube":"MRK01","not":"tezgâh","yonetici":"Y-1"}"#;
        // telefon yetkisiyle ya da yerel olmayan istekle olmaz
        assert_eq!(
            u.isle(&ist("POST", "/api/yonetim/baglama-kodu", None, kod, false))
                .kod,
            403
        );
        assert_eq!(
            u.isle(&ist("GET", "/api/yonetim/hesaplar", None, "", false))
                .kod,
            403
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                "/api/yonetim/baglama-kodu",
                None,
                r#"{"rol":"x","yonetici":"Y"}"#,
                true
            ))
            .kod,
            422
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                "/api/yonetim/baglama-kodu",
                None,
                r#"{"rol":"usta"}"#,
                true
            ))
            .kod,
            422
        );
        let y = u.isle(&ist("POST", "/api/yonetim/baglama-kodu", None, kod, true));
        assert_eq!(y.kod, 200);
        let v: serde_json::Value = serde_json::from_slice(&y.govde).unwrap();
        assert_eq!(v["kod"].as_str().unwrap().len(), 8);
        assert!(v["usta_id"].as_str().unwrap().starts_with("U-"));
        assert_eq!(v["baglanti"], ""); // telegram.json yok: QR üretilmez
        std::fs::write(
            u.dizin.join("telegram.json"),
            r#"{"gizli":"0123456789abcdef","bot_adi":"taa_bot"}"#,
        )
        .unwrap();
        let y = u.isle(&ist("POST", "/api/yonetim/baglama-kodu", None, kod, true));
        let v: serde_json::Value = serde_json::from_slice(&y.govde).unwrap();
        assert!(v["baglanti"]
            .as_str()
            .unwrap()
            .starts_with("https://t.me/taa_bot?start=baglan-"));
        assert!(v["qr_svg"].as_str().unwrap().contains("<svg"));
        assert_eq!(
            u.isle(&ist("GET", "/yonetim/personel", None, "", false))
                .kod,
            403
        );
        assert_eq!(
            u.isle(&ist("GET", "/yonetim/personel", None, "", true)).kod,
            200
        );
        // onay bekleyen hesap yokken onay 422
        assert_eq!(
            u.isle(&ist(
                "POST",
                "/api/yonetim/hesap/5/onayla",
                None,
                r#"{"yonetici":"Y"}"#,
                true
            ))
            .kod,
            422
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                "/api/yonetim/hesap/5/onayla",
                None,
                r#"{"yonetici":"Y"}"#,
                false
            ))
            .kod,
            403
        );
        // geçersiz WhatsApp belirteci
        let w = u.isle(&ist("GET", "/wa/yok", None, "", false));
        assert_eq!(w.kod, 410);
    }

    #[test]
    fn aydinlatma_sayfasi() {
        let u = uyg();
        let y = u.isle(&ist("GET", "/aydinlatma", None, "", false));
        assert_eq!(y.kod, 200);
        assert!(String::from_utf8(y.govde)
            .unwrap()
            .contains("henüz yüklenmedi"));
        std::fs::write(u.dizin.join("aydinlatma.html"), "<p>Şirket metni</p>").unwrap();
        let y = u.isle(&ist("GET", "/aydinlatma", None, "", false));
        assert!(String::from_utf8(y.govde).unwrap().contains("Şirket metni"));
    }

    #[test]
    fn telegram_webhook() {
        std::env::set_var("TAA_OZELLIK_TELEGRAM_BOT", "1");
        let u = uyg();
        let g = r#"{"message":{"chat":{"id":7},"from":{"id":42},"text":"/yardim"}}"#;
        // ayar yok -> 404
        assert_eq!(
            u.isle(&ist("POST", "/telegram/0123456789abcdef", None, g, false))
                .kod,
            404
        );
        std::fs::write(
            u.dizin.join("telegram.json"),
            r#"{"gizli":"0123456789abcdef","kullanicilar":{"42":"Usta Ahmet"}}"#,
        )
        .unwrap();
        assert_eq!(
            u.isle(&ist("POST", "/telegram/yanlis-gizli-xxxxx", None, g, false))
                .kod,
            404
        );
        let y = u.isle(&ist("POST", "/telegram/0123456789abcdef", None, g, false));
        assert_eq!(y.kod, 200);
        let j: serde_json::Value = serde_json::from_slice(&y.govde).unwrap();
        assert_eq!(j["method"], "sendMessage");
        assert!(j["text"].as_str().unwrap().contains("Komutlar"));
    }

    #[test]
    fn sayfalar_ve_sinirlar() {
        let u = uyg();
        let y = u.isle(&ist("GET", "/telefon", None, "", false));
        assert!(String::from_utf8(y.govde).unwrap().contains("<html"));
        assert_eq!(u.isle(&ist("GET", "/bilinmeyen", None, "", false)).kod, 404);
        let buyuk = "x".repeat(GOVDE_SINIRI + 1);
        assert_eq!(
            u.isle(&ist("POST", "/api/eslestir", None, &buyuk, false))
                .kod,
            413
        );
        assert_eq!(
            sayfalar::kacis("<a href=\"x\">&'"),
            "&lt;a href=&quot;x&quot;&gt;&amp;&#39;"
        );
    }

    #[test]
    fn foto_eki() {
        let u = uyg();
        let t = esles(&u);
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let no = serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        let jpg = vec![0xFF, 0xD8, 0xFF, 0xE0, 1, 2, 3];
        let ist = |m: &str, y: String, g: Vec<u8>| Istek {
            metot: m.into(),
            yol: y,
            yetki: Some(format!("Bearer {t}")),
            govde: g,
            yerel: false,
            ..Default::default()
        };
        let y = u.isle(&ist("POST", format!("/api/fis/{no}/foto"), jpg.clone()));
        assert_eq!(y.kod, 200);
        assert_eq!(
            u.isle(&ist("POST", format!("/api/fis/{no}/foto"), b"yok".to_vec()))
                .kod,
            400
        );
        assert_eq!(
            u.isle(&ist("POST", "/api/fis/F-yok-1/foto".into(), jpg.clone()))
                .kod,
            404
        );
        let g = u.isle(&ist("GET", format!("/api/fis/{no}/foto/1"), vec![]));
        assert_eq!((g.kod, g.govde), (200, jpg));
        assert_eq!(
            u.isle(&ist("GET", format!("/api/fis/{no}/foto/2"), vec![]))
                .kod,
            404
        );
    }

    #[test]
    fn oturum_yetkileri_ve_renkler() {
        let u = uyg();
        // mağaza PC'si: düzeltme varsayılan açık
        let o = json_of(&u.isle(&pc("GET", "/api/oturum", "")));
        assert!(o["yetkiler"]
            .as_array()
            .unwrap()
            .iter()
            .any(|y| y == "duzelt"));
        // telefon: varsayılan kapalı, yönetici verince açık
        let t = esles(&u);
        let o = json_of(&u.isle(&ist("GET", "/api/oturum", Some(&t), "", false)));
        assert!(!o["yetkiler"]
            .as_array()
            .unwrap()
            .iter()
            .any(|y| y == "duzelt"));
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/cihaz/T-001/yetki",
                r#"{"yetki":"duzelt","ver":true}"#
            ))
            .kod,
            200
        );
        let o = json_of(&u.isle(&ist("GET", "/api/oturum", Some(&t), "", false)));
        assert!(o["yetkiler"]
            .as_array()
            .unwrap()
            .iter()
            .any(|y| y == "duzelt"));
        // renk listesi: anahtar + Türkçe ad; belirteçsiz uzak istek reddedilir
        let r = u.isle(&ist("GET", "/api/renkler", Some(&t), "", false));
        assert_eq!(r.kod, 200);
        let r = json_of(&r);
        let liste = r["renkler"].as_array().unwrap();
        assert_eq!(liste.len(), taa_fis::RENKLER.len());
        assert_eq!(liste[0]["anahtar"], "siyah");
        assert_eq!(liste[0]["ad"], "Siyah");
        assert_eq!(
            u.isle(&ist("GET", "/api/renkler", None, "", false)).kod,
            401
        );
    }

    #[test]
    fn islemler_ve_teslim_degisikligi() {
        let u = uyg();
        let t = esles(&u);
        let k = u.isle(&ist("GET", "/api/islemler", Some(&t), "", false));
        assert_eq!(k.kod, 200);
        assert!(String::from_utf8(k.govde)
            .unwrap()
            .contains("pant-paca-kisalt"));
        let no = serde_json::from_slice::<serde_json::Value>(
            &u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false)).govde,
        )
        .unwrap()["no"]
            .as_str()
            .unwrap()
            .to_string();
        let yol = format!("/api/fis/{no}/teslim");
        // telefonun varsayılan olarak düzeltme yetkisi yok; yönetici verir
        assert_eq!(
            u.isle(&ist(
                "POST",
                &yol,
                Some(&t),
                r#"{"yeni":"2099-02-01","saat":"18:00","neden":"x"}"#,
                false
            ))
            .kod,
            403
        );
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/cihaz/T-001/yetki",
                r#"{"yetki":"duzelt","ver":true}"#
            ))
            .kod,
            200
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                &yol,
                Some(&t),
                r#"{"yeni":"2099-02-01"}"#,
                false
            ))
            .kod,
            422
        ); // neden yok
        assert_eq!(
            u.isle(&ist(
                "POST",
                &yol,
                Some(&t),
                r#"{"yeni":"2099-02-01","saat":"18:00","neden":"müşteri istedi"}"#,
                false
            ))
            .kod,
            200
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                &format!("/api/fis/{no}/durum"),
                Some(&t),
                r#"{"durum":"terzihanede"}"#,
                false
            ))
            .kod,
            200
        );
        let l = String::from_utf8(
            u.isle(&ist("GET", "/api/fisler", Some(&t), "", false))
                .govde,
        )
        .unwrap();
        assert!(l.contains("terzihanede") && l.contains("müşteri istedi"));
    }

    #[test]
    fn katalog_siniri_sunucuda_calisir() {
        let u = uyg();
        let t = esles(&u);
        let fis = |olcu: i32, onay: bool| {
            format!(
                r#"{{"satis_danismani_id":"K-test0001","olcu_alan_id":"K-test0002","duzenleyen_id":"K-test0001","musteri":"A B","telefon":"05321234567","kalemler":[{{"urun":"Pantolon","islem":"Bel daraltma","islem_kodu":"pant-bel-daralt","olcu_mm":{olcu},"usta_onayli":{onay}}}],"teslim":"2099-01-01","ucret_kurus":0}}"#
            )
        };
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), &fis(60, false), false));
        assert_eq!(y.kod, 422);
        assert!(String::from_utf8_lossy(&y.govde).contains("Usta onayı"));
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), &fis(60, true), false));
        assert_eq!(y.kod, 200);
        assert!(String::from_utf8_lossy(&y.govde).contains("doğrulanmadı"));
    }

    #[test]
    fn olcu_cm_istege_bagli_igneli() {
        let u = uyg();
        let t = esles(&u);
        let fis = r#"{"satis_danismani_id":"K-test0001","olcu_alan_id":"K-test0002","duzenleyen_id":"K-test0001","musteri":"A B","telefon":"05321234567","kalemler":[{"urun":"Pantolon","islem":"Bel daraltma","islem_kodu":"pant-bel-daralt","igneli":true},{"urun":"Pantolon","islem":"Paça kısaltma","islem_kodu":"pant-paca-kisalt","varyant":"Düz paça","igneli":true}],"teslim":"2099-01-01","ucret_kurus":0}"#;
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), fis, false));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert!(String::from_utf8_lossy(&y.govde).contains("denetlenemedi"));
    }

    #[test]
    fn takip_listeleri() {
        let u = uyg();
        let t = esles(&u);
        for (yol, anahtar) in [
            ("/api/takip/ozet", "kargo_bekleyen"),
            ("/api/takip/kargo", "fisler"),
            ("/api/takip/kuru-temizleme", "fisler"),
            ("/api/takip/geciken", "fisler"),
            ("/api/takip/usta", "ustalar"),
            ("/api/takip/usta-is", "ustalar"),
        ] {
            let y = u.isle(&ist("GET", yol, Some(&t), "", false));
            assert_eq!(y.kod, 200, "{yol}");
            let j: serde_json::Value = serde_json::from_slice(&y.govde).unwrap();
            assert!(j.get(anahtar).is_some(), "{yol}");
        }
        assert_eq!(
            u.isle(&ist("GET", "/api/takip/yok", Some(&t), "", false))
                .kod,
            404
        );
        // belirteçsiz erişim yok
        assert_eq!(
            u.isle(&ist("GET", "/api/takip/ozet", None, "", false)).kod,
            401
        );
    }

    fn pc(metot: &str, yol: &str, govde: &str) -> Istek {
        Istek {
            masaustu: true,
            ..ist(metot, yol, None, govde, true)
        }
    }

    fn json_of(y: &Yanit) -> serde_json::Value {
        serde_json::from_slice(&y.govde).unwrap_or_default()
    }

    #[test]
    fn kisi_alanlari_zorunlu_ve_kimlik_saklanir() {
        let u = uyg();
        let t = esles(&u);
        // Üçü de eksik: hata eksik alanları adıyla söyler.
        let bos = FIS.replace(KISI_ALANLARI, r#""satis_danismani":"Ad","olcu_alan":"Ad""#);
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), &bos, false));
        assert_eq!(y.kod, 422);
        let v = json_of(&y);
        assert_eq!(
            v["eksik"],
            serde_json::json!(["Satış danışmanı", "Ölçü alan", "Fişi düzenleyen"])
        );
        assert!(v["hata"]
            .as_str()
            .unwrap()
            .starts_with("Kaydetmek için seçin:"));
        // Kayıtta olmayan kimlik kabul edilmez.
        let yok = FIS.replace(
            r#""duzenleyen_id":"K-test0001""#,
            r#""duzenleyen_id":"K-yok00001""#,
        );
        let v = json_of(&u.isle(&ist("POST", "/api/fis", Some(&t), &yok, false)));
        assert_eq!(v["eksik"], serde_json::json!(["Fişi düzenleyen"]));
        assert!(!u.dizin.join("fisler.jsonl").exists());
        // Doğru fiş: kayıtta ad değil kimlik; okurken ad çözülür.
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false));
        assert_eq!(y.kod, 200);
        let satir = std::fs::read_to_string(u.dizin.join("fisler.jsonl")).unwrap();
        assert!(satir.contains("K-test0002") && !satir.contains("Deneme Usta"));
        let no = json_of(&y)["no"].as_str().unwrap().to_string();
        let f = json_of(&u.isle(&ist("GET", &format!("/api/fis/{no}"), Some(&t), "", false)));
        assert_eq!(f["olcu_alan"], "Deneme Usta");
        assert_eq!(f["satis_danismani"], "Deneme Danışman");
        assert_eq!(f["duzenleyen"], "Deneme Danışman");
        assert_eq!(f["kalan_kurus"], 40000);
        let l = json_of(&u.isle(&ist("GET", "/api/fisler", Some(&t), "", false)));
        assert_eq!(l["fisler"][0]["olcu_alan"], "Deneme Usta");
        // Kişi listesi telefon yetkisiyle okunur.
        let k = json_of(&u.isle(&ist("GET", "/api/kisiler", Some(&t), "", false)));
        assert_eq!(k["kisiler"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn kisisel_telefonda_duzenleyen_oturumdan_gelir() {
        let u = uyg();
        let y = u.isle(&ist(
            "GET",
            "/api/yonetim/eslestirme?kisi=K-test0002",
            None,
            "",
            true,
        ));
        assert_eq!(y.kod, 200);
        let kod = json_of(&y)["kod"].as_str().unwrap().to_string();
        assert_eq!(
            u.isle(&ist(
                "GET",
                "/api/yonetim/eslestirme?kisi=K-yok",
                None,
                "",
                true
            ))
            .kod,
            422
        );
        let t = kodla_esles(&u, &kod, "Kişisel");
        let o = json_of(&u.isle(&ist("GET", "/api/oturum", Some(&t), "", false)));
        assert_eq!(o["giris"], "kullanici");
        assert_eq!(o["kisi"]["ad"], "Deneme Usta");
        // İstemci başka düzenleyen gönderse de kayıt giren kişidir.
        let y = u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false));
        assert_eq!(y.kod, 200);
        let no = json_of(&y)["no"].as_str().unwrap().to_string();
        let f = json_of(&u.isle(&ist("GET", &format!("/api/fis/{no}"), Some(&t), "", false)));
        assert_eq!(f["duzenleyen_id"], "K-test0002");
        // İmzada da imzalayan giren kişidir.
        let y = u.isle(&ist(
            "POST",
            &format!("/api/fis/{no}/imza"),
            Some(&t),
            r#"{"rol":"teslim-eden","okutan":"K-test0001"}"#,
            false,
        ));
        assert_eq!(y.kod, 200);
        let f = json_of(&u.isle(&ist("GET", &format!("/api/fis/{no}"), Some(&t), "", false)));
        assert_eq!(f["imzalar"][0]["okutan_kisi_id"], "K-test0002");
        // Mağaza telefonu "tel" girişidir.
        let t2 = esles(&u);
        assert_eq!(
            json_of(&u.isle(&ist("GET", "/api/oturum", Some(&t2), "", false)))["giris"],
            "tel"
        );
    }

    #[test]
    fn magaza_pc_yalniz_yerel_ve_baslikli() {
        let u = uyg();
        // Başlıksız ya da uzaktan: telefon belirteci gerekir.
        assert_eq!(u.isle(&ist("GET", "/api/fisler", None, "", true)).kod, 401);
        let mut uzak = pc("GET", "/api/fisler", "");
        uzak.yerel = false;
        assert_eq!(u.isle(&uzak).kod, 401);
        assert_eq!(u.isle(&pc("GET", "/api/fisler", "")).kod, 200);
        assert_eq!(
            json_of(&u.isle(&pc("GET", "/api/oturum", "")))["giris"],
            "pc"
        );
        let y = u.isle(&pc("POST", "/api/fis", FIS));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let no = json_of(&y)["no"].as_str().unwrap().to_string();
        // Karekod sunucuda SVG olarak üretilir, içerik fişin telefon adresidir.
        let k = u.isle(&pc("GET", &format!("/api/fis/{no}/karekod"), ""));
        assert_eq!((k.kod, k.tur), (200, "image/svg+xml"));
        assert!(String::from_utf8(k.govde).unwrap().starts_with("<svg"));
        assert_eq!(u.isle(&pc("GET", "/api/fis/F-yok/karekod", "")).kod, 404);
        // Okutma ustanın telefonundan yapılır.
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{no}/okut"), "{}"))
                .kod,
            403
        );
        let d = json_of(&u.isle(&pc("GET", "/api/durum", "")));
        assert_eq!(d["sube"], "MRK01");
        assert_eq!(d["yazici"], false);
        assert_eq!(u.isle(&pc("POST", "/api/yazici/deneme", "")).kod, 409);
        assert_eq!(u.isle(&ist("GET", "/simge.svg", None, "", false)).kod, 200);
        assert!(
            String::from_utf8(u.isle(&ist("GET", "/manifest.json", None, "", false)).govde)
                .unwrap()
                .contains("/simge.svg")
        );
    }

    struct SahtePin(std::sync::Mutex<Vec<String>>);
    impl cati_pin::PinDogrulayici for SahtePin {
        fn dogrula(&self, pin: &str) -> cati_pin::PinSonuc {
            self.0.lock().unwrap().push(pin.into());
            match pin {
                "4821" => cati_pin::PinSonuc::Gecerli,
                "9999" => cati_pin::PinSonuc::Belirlenmemis,
                "5555" => cati_pin::PinSonuc::Kilitli {
                    ileti: "Çok fazla yanlış deneme".into(),
                    kalan_sn: 290,
                },
                "7777" => cati_pin::PinSonuc::Ulasilamadi("ağ yok".into()),
                _ => cati_pin::PinSonuc::Yanlis,
            }
        }
    }

    #[test]
    fn cifte_kod_belirtec_yalniz_yonetici_onayindan_sonra() {
        let mut u = uyg();
        u.yonetici_kilidi = true;
        u.pin = Box::new(SahtePin(Default::default()));
        let gir = json_of(&u.isle(&pc("POST", "/api/yonetici/giris", r#"{"pin":"4821"}"#)));
        let b = gir["belirtec"].as_str().unwrap().to_string();
        let yon = |metot: &str, yol: &str, govde: &str| Istek {
            yonetici: Some(b.clone()),
            ..pc(metot, yol, govde)
        };
        // Telefonlar ekranı: kişiye bağlı KOD 1 + karekod.
        let y = u.isle(&yon("GET", "/api/yonetim/eslestirme?kisi=K-test0002", ""));
        assert_eq!(y.kod, 200);
        let e = json_of(&y);
        assert_eq!(e["gecerli_sn"], 300);
        let kod = e["kod"].as_str().unwrap().to_string();
        // Telefon KOD 1'i girer: belirteç yok, yalnız {id, kod2}.
        let y = u.isle(&ist(
            "POST",
            "/api/eslestir",
            None,
            &format!(r#"{{"kod":"{kod}","ad":"Usta telefonu"}}"#),
            false,
        ));
        assert_eq!(y.kod, 200);
        let j = json_of(&y);
        assert!(j.get("token").is_none() && j.get("belirtec").is_none());
        let (id, kod2) = (
            j["id"].as_str().unwrap().to_string(),
            j["kod2"].as_str().unwrap().to_string(),
        );
        assert_eq!(kod2.len(), 4);
        let durum =
            |u: &Uygulama| u.isle(&ist("GET", &format!("/api/eslestir/{id}"), None, "", false));
        let d = durum(&u);
        assert_eq!(
            (d.kod, json_of(&d)["durum"].clone()),
            (200, json!("bekliyor"))
        );
        assert!(json_of(&d).get("token").is_none());
        // Yönetici oturumu olmadan (yerel olsa bile) onay yok, belirteç yok.
        let govde = format!(r#"{{"kod2":"{kod2}"}}"#);
        assert_eq!(
            u.isle(&pc("POST", "/api/yonetim/eslestir/onayla", &govde))
                .kod,
            401
        );
        assert_eq!(json_of(&durum(&u))["durum"], "bekliyor");
        assert!(u.cihazlar().aktifler().unwrap().is_empty());
        let c = json_of(&u.isle(&yon("GET", "/api/yonetim/cihazlar", "")));
        assert_eq!(c["bekleyen"], 1);
        assert!(!c.to_string().contains(&kod2));
        // Uzaktan oturumla da olmaz.
        let mut uzak = yon("POST", "/api/yonetim/eslestir/onayla", &govde);
        uzak.yerel = false;
        assert_eq!(u.isle(&uzak).kod, 403);
        // Yönetici onaylar: telefon belirteci bir kez alır, cihaz kişiye bağlanır.
        let o = u.isle(&yon("POST", "/api/yonetim/eslestir/onayla", &govde));
        assert_eq!(o.kod, 200, "{}", String::from_utf8_lossy(&o.govde));
        assert_eq!(json_of(&o)["ad"], "Usta telefonu");
        let d = json_of(&durum(&u));
        assert_eq!(d["durum"], "tamam");
        let t = d["token"].as_str().unwrap().to_string();
        assert_eq!(durum(&u).kod, 410);
        let ot = json_of(&u.isle(&ist("GET", "/api/oturum", Some(&t), "", false)));
        assert_eq!(ot["kisi"]["ad"], "Deneme Usta");
        let c = json_of(&u.isle(&yon("GET", "/api/yonetim/cihazlar", "")));
        assert_eq!(c["cihazlar"][0]["kisi_id"], "K-test0002");
        // Kayıtta kodlar ve belirteç yok, kişi var.
        let log = std::fs::read_to_string(u.dizin.join("eslestirme.jsonl")).unwrap();
        assert!(log.contains("K-test0002"));
        assert!(!log.contains(&kod2) && !log.contains(&kod) && !log.contains(&t));
        // Süre: 5 dakika sonra bekleyen düşer (telefon 410 alır).
        let k = json_of(&u.isle(&yon("GET", "/api/yonetim/eslestirme", "")))["kod"]
            .as_str()
            .unwrap()
            .to_string();
        let j = json_of(&u.isle(&ist(
            "POST",
            "/api/eslestir",
            None,
            &format!(r#"{{"kod":"{k}","ad":"Geç"}}"#),
            false,
        )));
        u.ileri_sn.store(301, std::sync::atomic::Ordering::SeqCst);
        let gec = format!("/api/eslestir/{}", j["id"].as_str().unwrap());
        assert_eq!(u.isle(&ist("GET", &gec, None, "", false)).kod, 410);
    }

    #[test]
    fn yonetici_kilidi_pin_cati_dan_ve_bosta_kapanir() {
        let mut u = uyg();
        u.yonetici_kilidi = true;
        u.pin = Box::new(SahtePin(Default::default()));
        let giris = |pin: &str| {
            u.isle(&pc(
                "POST",
                "/api/yonetici/giris",
                &format!(r#"{{"pin":"{pin}"}}"#),
            ))
        };
        // Kilitliyken yönetim uçları 401.
        for yol in [
            "/api/yonetim/cihazlar",
            "/api/yonetim/kisiler",
            "/api/yonetim/eslestirme",
            "/api/yonetim/hesaplar",
            "/api/yonetim/isaretler",
        ] {
            assert_eq!(u.isle(&pc("GET", yol, "")).kod, 401, "{yol}");
        }
        assert_eq!(giris("12").kod, 400);
        assert_eq!(giris("12a4").kod, 400);
        let y = giris("0000");
        assert_eq!(
            (y.kod, json_of(&y)["gecerli"].clone()),
            (200, serde_json::json!(false))
        );
        let y = giris("9999");
        assert_eq!(json_of(&y)["belirlendi"], false);
        assert_eq!(
            json_of(&y)["ileti"],
            "Yönetici PIN'i TAA Çatı'dan belirlenmemiş"
        );
        let y = giris("5555");
        assert_eq!(y.kod, 423);
        assert_eq!(json_of(&y)["kalan_sn"], 290);
        let y = giris("7777");
        assert_eq!(y.kod, 503);
        assert_eq!(
            json_of(&y)["ileti"],
            "TAA Çatı'ya ulaşılamıyor, yönetim ekranları açılamaz"
        );
        // Uzaktan ya da başlıksız giriş yok (PIN Çatı'ya hiç gitmez).
        let mut uzak = pc("POST", "/api/yonetici/giris", r#"{"pin":"4821"}"#);
        uzak.yerel = false;
        assert_eq!(u.isle(&uzak).kod, 403);
        let y = giris("4821");
        assert_eq!(y.kod, 200);
        let b = json_of(&y)["belirtec"].as_str().unwrap().to_string();
        let yon = |yol: &str| Istek {
            yonetici: Some(b.clone()),
            ..pc("GET", yol, "")
        };
        assert_eq!(u.isle(&yon("/api/yonetim/cihazlar")).kod, 200);
        assert_eq!(json_of(&u.isle(&yon("/api/yonetici/durum")))["acik"], true);
        // Kişi ekle / kapat.
        let k = u.isle(&Istek {
            yonetici: Some(b.clone()),
            ..pc(
                "POST",
                "/api/yonetim/kisi",
                r#"{"ad":"Yeni Kişi","rol":"usta"}"#,
            )
        });
        assert_eq!(k.kod, 200);
        let id = json_of(&k)["id"].as_str().unwrap().to_string();
        assert_eq!(
            u.isle(&Istek {
                yonetici: Some(b.clone()),
                ..pc("POST", &format!("/api/yonetim/kisi/{id}/kapat"), "")
            })
            .kod,
            200
        );
        // Belirteç uzaktan geçmez.
        let mut uzak = yon("/api/yonetim/cihazlar");
        uzak.yerel = false;
        assert_eq!(u.isle(&uzak).kod, 403);
        // 10 dk işlem olmazsa kapanır.
        u.ileri_sn
            .fetch_add(YONETICI_BOSTA_SN + 1, std::sync::atomic::Ordering::SeqCst);
        assert_eq!(u.isle(&yon("/api/yonetim/cihazlar")).kod, 401);
        // Çıkış.
        let b2 = json_of(&giris("4821"))["belirtec"]
            .as_str()
            .unwrap()
            .to_string();
        let ist2 = |yol: &str, m: &str| Istek {
            yonetici: Some(b2.clone()),
            ..pc(m, yol, "")
        };
        assert_eq!(u.isle(&ist2("/api/yonetim/cihazlar", "GET")).kod, 200);
        u.isle(&ist2("/api/yonetici/cikis", "POST"));
        assert_eq!(u.isle(&ist2("/api/yonetim/cihazlar", "GET")).kod, 401);
    }

    /// Kişi kaydı ve sube.json olmadan açılan sunucu (Windows'taki temiz kurulum).
    fn bos_uyg() -> Uygulama {
        let d = taa_dosya::gecici_klasor().unwrap();
        let mut u = Uygulama::yeni(d, None, "http://192.168.1.20:8787".into());
        u.yonetici_kilidi = false;
        u
    }

    const FIS_KISISIZ: &str = r#"{"musteri":"Ayşe Çelik","telefon":"0532 123 45 67","kalemler":[{"urun":"Pantolon","islem":"paça kısalt"}],"teslim":"2099-01-01","ucret_kurus":50000,"on_odeme_kurus":10000}"#;

    #[test]
    fn bos_kayitta_neden_ve_ne_yapilacagi_yazar() {
        let u = bos_uyg();
        let y = u.isle(&pc("POST", "/api/fis", FIS_KISISIZ));
        assert_eq!(y.kod, 422);
        let v = json_of(&y);
        assert_eq!(v["kisi_kaydi_bos"], true);
        let m = v["hata"].as_str().unwrap();
        assert!(
            m.contains("Kişi kaydı boş") && m.contains("İlk kurulum"),
            "{m}"
        );
        // Şube yokken durum çubuğu nedeni ve yolu söyler.
        let d = json_of(&u.isle(&pc("GET", "/api/durum", "")));
        assert!(d["sube_hata"].as_str().unwrap().contains("İlk kurulum"));
    }

    #[test]
    fn ilk_kurulum_yalniz_yerel_program_ve_bir_kez() {
        let u = bos_uyg();
        let govde = r#"{"sube":"MRK01","kisiler":[{"ad":"Deneme Danışman","rol":"danisman"},{"ad":"Deneme Usta","rol":"usta"},{"ad":"Deneme Personel","rol":"personel"}]}"#;
        // Telefon ya da ağdan gelen istek: 403; masaüstü başlığı yoksa (başka site) 403.
        assert_eq!(
            u.isle(&ist("POST", "/api/kurulum", None, govde, false)).kod,
            403
        );
        assert_eq!(
            u.isle(&ist("POST", "/api/kurulum", None, govde, true)).kod,
            403
        );
        assert_eq!(
            u.isle(&ist("GET", "/api/kurulum", None, "", false)).kod,
            403
        );
        let d = json_of(&u.isle(&pc("GET", "/api/kurulum", "")));
        assert_eq!(d["ilk_kurulum_acik"], true);
        assert_eq!(d["subeler"].as_array().unwrap().len(), 15);
        // Listede olmayan şube ve hatalı kişi: hiçbir şey yazılmaz.
        let y = u.isle(&pc(
            "POST",
            "/api/kurulum",
            r#"{"sube":"XYZ99","kisiler":[{"ad":"A","rol":"usta"}]}"#,
        ));
        assert_eq!(y.kod, 422);
        let y = u.isle(&pc(
            "POST",
            "/api/kurulum",
            r#"{"sube":"MRK01","kisiler":[{"ad":"A","rol":"usta"},{"ad":" ","rol":"usta"}]}"#,
        ));
        assert_eq!(y.kod, 422);
        assert!(!u.dizin.join("kisiler.jsonl").exists() && !u.dizin.join("sube.json").exists());
        // Geçerli kurulum: şube + 3 kişi.
        let y = u.isle(&pc("POST", "/api/kurulum", govde));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let k = json_of(&y)["kisiler"].as_array().unwrap().clone();
        assert_eq!(k.len(), 3);
        assert_eq!(k[0]["kaynak"], "ilk-kurulum");
        // İz dosyasında ad yok.
        let iz = std::fs::read_to_string(u.dizin.join("ilk-kurulum.json")).unwrap();
        assert!(!iz.contains("Deneme"));
        // Yol kapandı: ikinci deneme 409, kişi eklenmez.
        let y = u.isle(&pc(
            "POST",
            "/api/kurulum",
            r#"{"kisiler":[{"ad":"Başka Biri","rol":"yonetici"}]}"#,
        ));
        assert_eq!(y.kod, 409);
        assert_eq!(
            json_of(&u.isle(&pc("GET", "/api/kisiler", "")))["kisiler"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
        assert_eq!(
            json_of(&u.isle(&pc("GET", "/api/kurulum", "")))["kapali_neden"],
            "yapildi"
        );
        // Fiş artık kaydedilir.
        let id = |n: usize| k[n]["id"].as_str().unwrap().to_string();
        let fis = FIS_KISISIZ.replacen(
            '{',
            &format!(
                r#"{{"satis_danismani_id":"{}","olcu_alan_id":"{}","duzenleyen_id":"{}","#,
                id(0),
                id(1),
                id(2)
            ),
            1,
        );
        let y = u.isle(&pc("POST", "/api/fis", &fis));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        // Şubeyi sonradan yalnız yönetici değiştirir.
        let mut kilitli = bos_uyg();
        kilitli.dizin = u.dizin.clone();
        kilitli.yonetici_kilidi = true;
        assert_eq!(
            kilitli
                .isle(&pc("POST", "/api/yonetim/sube", r#"{"kod":"PND01"}"#))
                .kod,
            401
        );
        assert_eq!(
            u.isle(&pc("POST", "/api/yonetim/sube", r#"{"kod":"PND99"}"#))
                .kod,
            422
        );
        assert_eq!(
            u.isle(&pc("POST", "/api/yonetim/sube", r#"{"kod":"PND01"}"#))
                .kod,
            200
        );
        assert_eq!(
            json_of(&u.isle(&pc("GET", "/api/durum", "")))["sube"],
            "PND01"
        );
    }

    #[test]
    fn cati_listesi_alininca_ilk_kurulum_kapanir_ve_cati_esas() {
        struct Sahte;
        impl kurulum::CatiKaynagi for Sahte {
            fn cek(&self, _: Option<&str>, _: &str) -> Result<kurulum::CatiVerisi, String> {
                Ok(kurulum::CatiVerisi {
                    sube: Some("ANK01".into()),
                    kisiler: vec![kisi::Kisi {
                        id: "k-cati-1".into(),
                        ad: "Çatı K.".into(),
                        rol: "danisman".into(),
                        tarih: String::new(),
                        kaynak: "cati".into(),
                    }],
                })
            }
        }
        let mut u = bos_uyg();
        u.cati = std::sync::Arc::new(Sahte);
        assert_eq!(
            u.isle(&ist("POST", "/api/kurulum/esitle", None, "", true))
                .kod,
            403
        );
        let y = u.isle(&pc("POST", "/api/kurulum/esitle", ""));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let d = json_of(&u.isle(&pc("GET", "/api/kurulum", "")));
        assert_eq!(d["ilk_kurulum_acik"], false);
        assert_eq!(d["kapali_neden"], "cati");
        assert_eq!(d["sube"], "ANK01");
        assert_eq!(d["cati"]["bagli"], true);
        let y = u.isle(&pc(
            "POST",
            "/api/kurulum",
            r#"{"kisiler":[{"ad":"Yerel","rol":"usta"}]}"#,
        ));
        assert_eq!(y.kod, 409);
        // Yönetici de yerel kişi ekleyemez: tek kimlik Çatı'dan.
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/kisi",
                r#"{"ad":"Yerel","rol":"usta"}"#
            ))
            .kod,
            409
        );
        let l = json_of(&u.isle(&pc("GET", "/api/kisiler", "")));
        assert_eq!(l["kisiler"][0]["id"], "k-cati-1");
        assert_eq!(l["kisiler"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn ortak_giris_yalniz_bu_bilgisayardaki_programdan() {
        fn yok() -> Option<PathBuf> {
            None
        }
        let mut u = uyg();
        u.kimlik_bulucu = yok;
        let gonder = |yerel: bool, masaustu: bool, islem: &str| {
            let y = u.isle(&Istek {
                metot: "POST".into(),
                yol: format!("/api/giris/{islem}"),
                govde: b"{}".to_vec(),
                yerel,
                masaustu,
                ..Default::default()
            });
            (
                y.kod,
                serde_json::from_slice::<serde_json::Value>(&y.govde).unwrap_or_default(),
            )
        };
        assert_eq!(
            gonder(true, true, "baslangic"),
            (200, json!({"arac": false, "oturum": null}))
        );
        assert_eq!(
            gonder(false, true, "baslangic").0,
            403,
            "telefondan/ağdan olmaz"
        );
        assert_eq!(
            gonder(true, false, "baslangic").0,
            403,
            "başlıksız (başka site) olmaz"
        );
        assert_eq!(gonder(true, true, "bilinmeyen").0, 404);
    }

    #[test]
    fn gelistirici_araclari_adresi_404_vermez() {
        let u = bos_uyg();
        let y = u.isle(&ist(
            "GET",
            "/.well-known/appspecific/com.chrome.devtools.json",
            None,
            "",
            true,
        ));
        assert_eq!(y.kod, 200);
        assert_eq!(json_of(&y), json!({}));
    }

    #[test]
    fn duzeltme_yetkisi_baglama_ve_bagli_teslim() {
        let u = uyg();
        let t = esles(&u);
        let fis = |yerel_pc: bool| {
            let y = if yerel_pc {
                u.isle(&pc("POST", "/api/fis", FIS))
            } else {
                u.isle(&ist("POST", "/api/fis", Some(&t), FIS, false))
            };
            assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
            json_of(&y)["no"].as_str().unwrap().to_string()
        };
        let a = fis(true);
        let b = fis(false);
        let kalem = r#"{"kalem":{"urun":"Ceket","islem":"ütü","renk":"bej"}}"#;
        // telefon düzeltemez (varsayılan), mağaza PC düzeltir
        let y = u.isle(&ist(
            "POST",
            &format!("/api/fis/{a}/kalem"),
            Some(&t),
            kalem,
            false,
        ));
        assert_eq!(y.kod, 403);
        assert_eq!(json_of(&y)["hata"], "yetki_yok");
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{a}/kalem"), kalem))
                .kod,
            200
        );
        // kargo alıcı adında çift yönlü yazı işareti reddedilir
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{a}/teslim-sekli"),
                "{\"kargo\":true,\"kargo_ad\":\"Deneme \u{202E}Alıcı\",\"kargo_adres\":\"Deneme Mah. 1. Sk. No:1 Büyükçekmece/İstanbul\",\"kargo_telefon\":\"0532 000 00 00\"}")).kod,
            422
        );
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{a}/teslim-sekli"),
                r#"{"kargo":true,"kargo_ad":"Deneme Alıcı","kargo_adres":"Deneme Mah. 1. Sk. No:1 Büyükçekmece/İstanbul","kargo_telefon":"0532 000 00 00"}"#)).kod,
            200
        );
        assert_eq!(
            u.isle(&pc(
                "POST",
                &format!("/api/fis/{a}/teslim-sekli"),
                r#"{"kargo":false,"neden":"kargo iptal"}"#
            ))
            .kod,
            200
        );
        assert_eq!(
            u.isle(&pc(
                "POST",
                &format!("/api/fis/{a}/bagla"),
                &format!(r#"{{"diger":"{b}"}}"#)
            ))
            .kod,
            200
        );
        let f = json_of(&u.isle(&pc("GET", &format!("/api/fis/{a}"), "")));
        assert_eq!(f["bagli"][0], b.as_str());
        assert_eq!(f["kalemler"].as_array().unwrap().len(), 2);
        // kalem çıkarmada istemcinin gördüğü kalem tutmazsa (sıra kaymış) 409, kalem çıkmaz
        assert_eq!(
            u.isle(&pc(
                "POST",
                &format!("/api/fis/{a}/kalem/1/cikar"),
                r#"{"neden":"deneme","urun":"Pantolon","islem":"paça kısalt"}"#
            ))
            .kod,
            409
        );
        assert_eq!(
            json_of(&u.isle(&pc("GET", &format!("/api/fis/{a}"), "")))["kalemler"]
                .as_array()
                .unwrap()
                .len(),
            2
        );
        assert!(f["duzeltmeler"].as_array().unwrap().len() >= 4);
        // yönetici yetkiyi verir, sonra geri alır; mağaza PC'sinden de alınabilir
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/cihaz/T-001/yetki",
                r#"{"yetki":"duzelt","ver":true}"#
            ))
            .kod,
            200
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                &format!("/api/fis/{b}/kalem"),
                Some(&t),
                kalem,
                false
            ))
            .kod,
            200
        );
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/cihaz/T-001/yetki",
                r#"{"yetki":"duzelt","ver":false}"#
            ))
            .kod,
            200
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                &format!("/api/fis/{b}/kalem"),
                Some(&t),
                kalem,
                false
            ))
            .kod,
            403
        );
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/cihaz/PC/yetki",
                r#"{"yetki":"duzelt","ver":false}"#
            ))
            .kod,
            200
        );
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{a}/kalem"), kalem))
                .kod,
            403
        );
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/cihaz/PC/yetki",
                r#"{"yetki":"duzelt","ver":true}"#
            ))
            .kod,
            200
        );
        // yetki/ad yönetimi telefondan yapılamaz; ad yalnız masaüstü programdan
        assert_eq!(
            u.isle(&ist(
                "POST",
                "/api/yonetim/cihaz/T-001/yetki",
                Some(&t),
                r#"{"yetki":"duzelt","ver":true}"#,
                false
            ))
            .kod,
            403
        );
        assert_eq!(
            u.isle(&ist(
                "POST",
                "/api/yonetim/cihaz/T-001/ad",
                None,
                r#"{"ad":"X"}"#,
                true
            ))
            .kod,
            403
        );
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/cihaz/T-001/ad",
                "{\"ad\":\"Müdür \u{202E}telefonu\"}"
            ))
            .kod,
            422
        );
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/cihaz/T-001/ad",
                r#"{"ad":"Müdür telefonu"}"#
            ))
            .kod,
            200
        );
        let l = json_of(&u.isle(&pc("GET", "/api/yonetim/yetkiler", "")));
        assert_eq!(l["cihazlar"][1]["ad"], "Müdür telefonu");
        assert_eq!(l["cihazlar"][0]["yetkiler"][0], "duzelt");
        // bağlı teslim: ikisi de hazır; teslimde sorulur, seçilen birlikte teslim edilir
        for n in [&a, &b] {
            assert_eq!(
                u.isle(&pc(
                    "POST",
                    &format!("/api/fis/{n}/durum"),
                    r#"{"durum":"hazir"}"#
                ))
                .kod,
                200
            );
        }
        // durum ucu teslim etmez (teslim-al gerekir)
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{a}/durum"),
            r#"{"durum":"teslim"}"#,
        ));
        assert_eq!(y.kod, 409);
        assert_eq!(json_of(&y)["hata"], "teslim_al_gerekli");
        let teslim = r#""fisli":true,"teslim_alan":"Deneme Müşteri","iliski":"kendisi""#;
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{a}/teslim-al"),
            &format!("{{{teslim}}}"),
        ));
        assert_eq!(y.kod, 409);
        assert_eq!(json_of(&y)["soru"], "bagli_teslim");
        assert_eq!(json_of(&y)["bagli"][0]["no"], b.as_str());
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{a}/teslim-al"),
            &format!(r#"{{{teslim},"bagli":["{b}"]}}"#),
        ));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(
            json_of(&u.isle(&pc("GET", &format!("/api/fis/{b}"), "")))["durum"],
            "teslim"
        );
        // personel ad soyad birlikte
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/kisi",
                r#"{"ad":"Ali","rol":"personel"}"#
            ))
            .kod,
            422
        );
    }
}
