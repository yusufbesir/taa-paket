//! İlk kurulum ve TAA Çatı eşitlemesi (kişi listesi ve şube kodu).
//!
//! Kişilerin ve şubenin asıl kaynağı TAA Çatı/TAA Sunucu'dur: kişiler havuzdaki `rehber.kisi`,
//! `rehber.birim` ve `rehber.gorevlendirme` tablolarından (docs/rehber-sozlesmesi.md; `terzitakip` okuyucudur),
//! şube kodu cihaz kaydından (`POST /v1/cihaz/durum` → `sube_id`, gerekirse `GET /v1/sube`) gelir.
//! Sunucu açılışta ve `ESITLEME_ARALIGI_SN` saniyede bir eşitler; sonuç bu klasörde önbelleğe yazılır
//! (`cati-kisiler.json`, `cati.json`); Çatı yokken son önbellek kullanılır (Fable M5, H9: "beyan" düzeyi).
//!
//! İlk kurulum (yalnız bu bilgisayardaki masaüstü program, loopback + `X-TAA-Masaustu`): bu sunucu TAA Çatı'dan
//! hiç kişi listesi almadıysa, ilk kurulum daha önce yapılmadıysa ve kişi kaydı boşsa (ya da şube ayarlı değilse)
//! şube ve kişiler TEK istekte bir kez girilir. Girilince `ilk-kurulum.json` yazılır ve yol kapanır; sonraki
//! değişiklik yönetici girişi (TAA Çatı PIN) ister. Çatı listesi bir kez alınınca yol kalıcı olarak kapanır ve
//! Çatı'daki kişiler esas alınır (yerel kayıt seçimden düşer, eski fişlerde adı çözülmeye devam eder).
//! Güvenlik gerekçesi DEGISIKLIK.md'de; kısaca: yönetici yetkisi vermez, var olan kaydı değiştiremez,
//! tek kullanımlıktır ve Çatı bağlantısıyla geçersizleşir.

use super::*;
use crate::kisi::{Kisi, Kisiler};
use serde_json::Value;

/// Gömülü 15 şube (TAA Kur ile aynı kaynak: `kurulum/subeler.txt`, kod<TAB>slug<TAB>ad<TAB>il).
const SUBELER_TXT: &str = include_str!("../../../kurulum/subeler.txt");
/// Periyodik eşitleme aralığı (saniye).
pub const ESITLEME_ARALIGI_SN: u64 = 600;
const CATI_DURUM: &str = "cati.json";
const CATI_KISILER: &str = "cati-kisiler.json";
const ILK_KURULUM: &str = "ilk-kurulum.json";
const EN_COK_ILK_KISI: usize = 30;
/// Havuz önbelleğinin program adı (docs/havuz-turleri.toml okuyucu listesi).
pub const HAVUZ_PROGRAMI: &str = "terzitakip";

/// (kod, ad) listesi.
pub fn subeler() -> Vec<(String, String)> {
    SUBELER_TXT
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
        .filter_map(|l| {
            let p: Vec<&str> = l.trim_end_matches('\r').split('\t').collect();
            (p.len() >= 3 && taa_fis::sube_kodu_gecerli(p[0].trim()))
                .then(|| (p[0].trim().to_string(), p[2].trim().to_string()))
        })
        .collect()
}

/// TAA Çatı'dan alınan veri.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CatiVerisi {
    /// Cihaz kaydındaki şube kodu (merkez cihazında ya da bilinmiyorsa None).
    pub sube: Option<String>,
    /// Bu şubede etkin görevlendirmesi olan kişiler (görünen adla).
    pub kisiler: Vec<Kisi>,
}

/// Çatı'dan veri çeken taraf. Testlerde sahte uygulanır.
pub trait CatiKaynagi: Send + Sync {
    /// `sube_yerel`: bu bilgisayarda ayarlı şube (cihaz kaydı şube vermezse süzgeç için). `bugun`: YYYY-AA-GG.
    fn cek(&self, sube_yerel: Option<&str>, bugun: &str) -> Result<CatiVerisi, String>;
}

/// Çatı bağlantısı yapılandırılmamış (testler, `Uygulama::yeni` varsayılanı).
pub struct Yapilandirilmamis;

impl CatiKaynagi for Yapilandirilmamis {
    fn cek(&self, _: Option<&str>, _: &str) -> Result<CatiVerisi, String> {
        Err("TAA Çatı bağlantısı yapılandırılmamış".into())
    }
}

/// Gerçek istemci: TAA Çatı'nın oturumu + cihaz anahtarı (bkz. `cati_pin::cati_gonderici`).
pub struct CatiIstemcisi {
    pub kok: PathBuf,
    pub sunucu: Option<String>,
    pub sertifika: Option<PathBuf>,
}

impl CatiKaynagi for CatiIstemcisi {
    fn cek(&self, sube_yerel: Option<&str>, bugun: &str) -> Result<CatiVerisi, String> {
        use taa_kuyruk::Gonderici;
        let g =
            cati_pin::cati_gonderici(&self.kok, self.sunucu.as_deref(), self.sertifika.as_deref())?;
        // Şube: cihaz kaydından (imzalı). Hata kişi eşitlemesini durdurmaz.
        let sube = g
            .gonder("/v1/cihaz/durum", &json!({}))
            .ok()
            .and_then(|v| sube_coz(&v, || g.al("/v1/sube").ok()));
        let mut h = taa_havuz::Havuz::ac(&self.kok, HAVUZ_PROGRAMI).map_err(|e| e.to_string())?;
        h.esitle(&g)
            .map_err(|e| format!("TAA Çatı kişi listesi alınamadı: {e}"))?;
        let satirlar = |tur: &str| -> Vec<Value> {
            h.ogeler(None)
                .into_iter()
                .filter(|o| o.tur.split('@').next() == Some(tur) && o.kip == "tablo")
                .flat_map(|o| {
                    h.satirlar(o.id)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|(_, v)| v)
                })
                .collect()
        };
        let Some(s) = sube.clone().or_else(|| sube_yerel.map(str::to_string)) else {
            return Err("şube bilinmiyor; kişi listesi şubeye göre süzülemedi".into());
        };
        let kisiler = rehberden_kisiler(
            &satirlar("rehber.kisi"),
            &satirlar("rehber.birim"),
            &satirlar("rehber.gorevlendirme"),
            &s,
            bugun,
        );
        Ok(CatiVerisi { sube, kisiler })
    }
}

/// `POST /v1/cihaz/durum` yanıtından şube kodu. `sube_id` kod biçimindeyse doğrudan; sayıysa `GET /v1/sube`
/// listesinden koda çevrilir. Onaylı olmayan cihazda ya da merkez cihazında (`null`) None.
pub fn sube_coz(durum: &Value, sube_listesi: impl FnOnce() -> Option<Value>) -> Option<String> {
    if durum["durum"].as_str() != Some("onayli") {
        return None;
    }
    match &durum["sube_id"] {
        Value::String(k) if taa_fis::sube_kodu_gecerli(k.trim()) => Some(k.trim().to_string()),
        Value::Number(n) => {
            let id = n.as_i64()?;
            sube_listesi()?
                .as_array()?
                .iter()
                .find(|s| s["id"].as_i64() == Some(id))
                .and_then(|s| s["kod"].as_str())
                .map(str::trim)
                .filter(|k| taa_fis::sube_kodu_gecerli(k))
                .map(str::to_string)
        }
        _ => None,
    }
}

/// Görünen ad (Fable M4: mağaza ekranı ve basılı fiş): "Ad Soyad" → "Ad S.".
pub fn gorunen_ad(ad: &str) -> String {
    let p: Vec<&str> = ad.split_whitespace().collect();
    match p.as_slice() {
        [] => String::new(),
        [tek] => (*tek).to_string(),
        [bas @ .., son] => format!(
            "{} {}.",
            bas.join(" "),
            son.chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_default()
        ),
    }
}

/// Görevlendirme rolünden (iletişim rolü ve unvan) fişteki rol; yalnız gösterim içindir, yetki vermez.
fn rol_esle(rol: &str, gorev: &str) -> &'static str {
    let m = format!("{rol} {gorev}").to_lowercase();
    if m.contains("usta") || m.contains("terzi") {
        "usta"
    } else if m.contains("danis")
        || m.contains("danış")
        || m.contains("satis")
        || m.contains("satış")
    {
        "danisman"
    } else if m.contains("mudur")
        || m.contains("müdür")
        || m.contains("yonetici")
        || m.contains("yönetici")
    {
        "yonetici"
    } else {
        "personel"
    }
}

/// Rehber tablolarından bu şubenin seçilebilir kişileri. Saf işlev (ağsız sınanır).
/// Kural (docs/rehber-sozlesmesi.md §2, §5): kişi `etkin`; birimi `sube`/`magaza` türünde, `sube_kodu` bu şube ve
/// `etkin`; görevlendirme `etkin`, başlamış ve bitmemiş. Merkez/departman birimlerindeki kişiler alınmaz.
pub fn rehberden_kisiler(
    kisiler: &[Value],
    birimler: &[Value],
    gorevler: &[Value],
    sube: &str,
    bugun: &str,
) -> Vec<Kisi> {
    let metin = |v: &Value, a: &str| v[a].as_str().unwrap_or("").trim().to_string();
    let birim: Vec<String> = birimler
        .iter()
        .filter(|b| matches!(b["tur"].as_str(), Some("sube" | "magaza")))
        .filter(|b| metin(b, "sube_kodu") == sube && metin(b, "durum") == "etkin")
        .map(|b| metin(b, "id"))
        .collect();
    let mut v: Vec<Kisi> = Vec::new();
    for g in gorevler {
        let bas = metin(g, "baslangic");
        let bit = metin(g, "bitis");
        if metin(g, "durum") != "etkin"
            || !birim.contains(&metin(g, "birim_id"))
            || (!bas.is_empty() && bas.as_str() > bugun)
            || (!bit.is_empty() && bit.as_str() < bugun)
        {
            continue;
        }
        let kid = metin(g, "kisi_id");
        if v.iter().any(|k| k.id == kid) || !kisi::kimlik_gecerli(&kid) {
            continue;
        }
        let Some(k) = kisiler
            .iter()
            .find(|k| metin(k, "id") == kid && metin(k, "durum") == "etkin")
        else {
            continue;
        };
        let ad = metin(k, "ad");
        if ad.is_empty() {
            continue;
        }
        v.push(Kisi {
            id: kid,
            ad,
            rol: rol_esle(&metin(g, "rol"), &metin(g, "gorev")).into(),
            tarih: String::new(),
            kaynak: "cati".into(),
        });
    }
    // Gerçek ad yalnız yöneticiye (M4): listede görünen ad; şube içinde çakışırsa tam ad kalır.
    let kisa: Vec<String> = v.iter().map(|k| gorunen_ad(&k.ad)).collect();
    for (i, k) in v.iter_mut().enumerate() {
        if kisa.iter().filter(|x| **x == kisa[i]).count() == 1 {
            k.ad = kisa[i].clone();
        }
    }
    v.sort_by_key(|k| k.ad.to_lowercase());
    v
}

fn json_oku(yol: &Path) -> Value {
    std::fs::read(yol)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(|| json!({}))
}

fn json_yaz(yol: &Path, v: &Value) -> Result<(), String> {
    taa_dosya::uzerine_yaz_atomik(yol, v.to_string().as_bytes())
        .map_err(|e| format!("{} yazılamadı: {e}", yol.display()))
}

/// `cati.json`: {bagli, ilk_baglanti, son_esitleme, son_deneme, son_hata, sube_cati}.
pub fn cati_durumu(dizin: &Path) -> Value {
    json_oku(&dizin.join(CATI_DURUM))
}

/// Bu sunucu TAA Çatı'dan (boş olmayan) kişi listesi en az bir kez aldı mı? Alındıysa Çatı esastır.
pub fn cati_bagli(dizin: &Path) -> bool {
    cati_durumu(dizin)["bagli"].as_bool() == Some(true)
}

/// Önbellekteki Çatı kişileri (bağlı olsun olmasın; eski fişlerin adı için).
pub fn cati_onbellegi(dizin: &Path) -> Vec<Kisi> {
    serde_json::from_value(json_oku(&dizin.join(CATI_KISILER))["kisiler"].clone())
        .unwrap_or_default()
}

/// Çatı esassa seçilebilir kişiler (önbellekten); değilse None (yerel kayıt kullanılır).
pub fn cati_kisileri(dizin: &Path) -> Option<Vec<Kisi>> {
    cati_bagli(dizin).then(|| cati_onbellegi(dizin))
}

/// Bir eşitleme turu: sonuç dosyalara yazılır. Hata durumunda önbellek bozulmaz (son liste geçerli kalır).
pub fn esitle(
    dizin: &Path,
    kok: Option<&Path>,
    kaynak: &dyn CatiKaynagi,
    simdi: u64,
) -> Result<String, String> {
    let _ = std::fs::create_dir_all(dizin);
    let mut d = cati_durumu(dizin);
    if !d.is_object() {
        d = json!({});
    }
    let zaman = format!("{} {}", tarih_iso(simdi), saat_iso(simdi));
    d["son_deneme"] = json!(zaman);
    let yerel = sube_oku(dizin, kok).map(|(k, _)| k);
    let sonuc = kaynak.cek(yerel.as_deref(), &tarih_iso(simdi));
    let ozet = match sonuc {
        Err(e) => {
            d["son_hata"] = json!(e);
            json_yaz(&dizin.join(CATI_DURUM), &d)?;
            return Err(e);
        }
        Ok(c) => {
            let mut notlar = Vec::new();
            if let Some(s) = &c.sube {
                d["sube_cati"] = json!(s);
                match &yerel {
                    None => {
                        sube_yaz(dizin, s, "cati", simdi)?;
                        notlar.push(format!("şube TAA Çatı'dan alındı: {s}"));
                    }
                    Some(y) if y != s => notlar.push(format!(
                        "şube uyuşmuyor (TAA Çatı: {s}, bu bilgisayar: {y}); yönetici düzeltmeli"
                    )),
                    _ => {}
                }
            }
            if c.kisiler.is_empty() {
                d["son_hata"] = json!(
                    "TAA Çatı'daki kişi listesi bu şube için boş (TAA Rehber'de görevlendirme yok)"
                );
                notlar.push("Çatı kişi listesi boş; yerel kayıt kullanılmaya devam ediyor".into());
            } else {
                json_yaz(
                    &dizin.join(CATI_KISILER),
                    &json!({ "tarih": zaman, "sube": c.sube, "kisiler": c.kisiler }),
                )?;
                if d["bagli"].as_bool() != Some(true) {
                    d["ilk_baglanti"] = json!(zaman);
                }
                d["bagli"] = json!(true);
                d["son_esitleme"] = json!(zaman);
                d["son_hata"] = json!("");
                notlar.push(format!("{} kişi TAA Çatı'dan alındı", c.kisiler.len()));
            }
            notlar.join("; ")
        }
    };
    json_yaz(&dizin.join(CATI_DURUM), &d)?;
    Ok(ozet)
}

/// Şube kodu ve kaynağı: önce bu klasördeki `sube.json`, yoksa TAA veri kökündeki (TAA Kur'un yazdığı) `sube.json`.
pub(crate) fn sube_oku(dizin: &Path, kok: Option<&Path>) -> Option<(String, String)> {
    let oku = |yol: PathBuf, varsayilan: &str| -> Option<(String, String)> {
        let v = json_oku(&yol);
        let k = v["kod"].as_str()?.trim().to_string();
        taa_fis::sube_kodu_gecerli(&k).then(|| {
            let kaynak = v["kaynak"]
                .as_str()
                .filter(|s| !s.is_empty())
                .unwrap_or(varsayilan);
            (k, kaynak.to_string())
        })
    };
    oku(dizin.join("sube.json"), "dosya")
        .or_else(|| kok.and_then(|k| oku(k.join("sube.json"), "taa-kur")))
}

pub(crate) fn sube_yaz(dizin: &Path, kod: &str, kaynak: &str, simdi: u64) -> Result<(), String> {
    if !taa_fis::sube_kodu_gecerli(kod) {
        return Err("şube kodu 3 büyük harf + 2 rakam olmalı (örn. MRK01)".into());
    }
    let _ = std::fs::create_dir_all(dizin);
    json_yaz(
        &dizin.join("sube.json"),
        &json!({ "kod": kod, "kaynak": kaynak, "tarih": tarih_iso(simdi) }),
    )
}

/// İlk kurulumun açık olup olmadığı ve kapalıysa nedeni.
pub(crate) enum IlkKurulum {
    Acik { kisi_bos: bool, sube_yok: bool },
    Kapali(&'static str),
}

impl Uygulama {
    pub(crate) fn ilk_kurulum(&self) -> IlkKurulum {
        if cati_bagli(&self.dizin) {
            return IlkKurulum::Kapali("cati");
        }
        if self.dizin.join(ILK_KURULUM).exists() {
            return IlkKurulum::Kapali("yapildi");
        }
        let sube = self.sube_kodu().ok();
        let kisi_bos = kisi::secilebilir(&self.dizin, sube.as_deref().unwrap_or("")).is_empty();
        let sube_yok = sube.is_none();
        if kisi_bos || sube_yok {
            IlkKurulum::Acik { kisi_bos, sube_yok }
        } else {
            IlkKurulum::Kapali("gerek-yok")
        }
    }

    /// Kişi kaydı boşken fiş neden kaydedilemiyor ve ne yapılmalı (ekranda ve 422 yanıtında aynı metin).
    pub(crate) fn kisi_bos_iletisi(&self) -> String {
        let neden = "Kişi kaydı boş: fişte satış danışmanı, ölçü alan ve düzenleyen listeden seçilmek zorunda olduğu için fiş kaydedilemez.";
        let ne = match self.ilk_kurulum() {
            IlkKurulum::Acik { .. } => "Bu bilgisayar henüz TAA Çatı'ya bağlanmadı: mağaza bilgisayarındaki TAA Terzi Takip programında İlk kurulum ekranından kişileri bir kez ekleyin.",
            IlkKurulum::Kapali("cati") => "Kişiler TAA Çatı'dan gelir: yönetici TAA Rehber'de kişileri bu şubeye görevlendirmeli; liste bağlantı gelince kendiliğinden yüklenir.",
            _ => "Yönetici, Personel ve usta ekranından (TAA Çatı yönetici PIN'i ile) kişileri eklemeli ya da TAA Çatı bağlantısı kurulmalı.",
        };
        format!("{neden} {ne}")
    }

    pub(crate) fn sube_yok_iletisi(&self) -> String {
        let ne = match self.ilk_kurulum() {
            IlkKurulum::Acik { .. } => "Mağaza bilgisayarında İlk kurulum ekranından şubeyi seçin.",
            _ => "Yönetici, Personel ve usta ekranından şubeyi ayarlamalı; TAA Çatı bağlantısı kurulunca şube cihaz kaydından gelir.",
        };
        format!("Şube kodu ayarlanmamış: fişin hangi şubeye ait olduğu bilinmeden fiş kaydedilemez. {ne}")
    }

    /// GET /api/kurulum: ilk kurulum, şube ve TAA Çatı eşitleme durumu (yalnız bu bilgisayar).
    pub(crate) fn kurulum_durum(&self, i: &Istek) -> Yanit {
        if !i.yerel {
            return Yanit::hata(403, "yalnız bu bilgisayardan");
        }
        let (acik, neden, kisi_bos, sube_yok) = match self.ilk_kurulum() {
            IlkKurulum::Acik { kisi_bos, sube_yok } => (true, "", kisi_bos, sube_yok),
            IlkKurulum::Kapali(n) => (false, n, false, false),
        };
        let sube = sube_oku(&self.dizin, self.kok.as_deref());
        let c = cati_durumu(&self.dizin);
        let kisi_sayisi = kisi::secilebilir(
            &self.dizin,
            sube.as_ref().map(|s| s.0.as_str()).unwrap_or(""),
        )
        .len();
        Yanit::json(
            200,
            json!({
                "ilk_kurulum_acik": acik,
                "kapali_neden": neden,
                "kisi_bos": kisi_bos || kisi_sayisi == 0,
                "sube_yok": sube_yok || sube.is_none(),
                "kisi_sayisi": kisi_sayisi,
                "sube": sube.as_ref().map(|s| s.0.clone()),
                "sube_kaynak": sube.map(|s| s.1),
                "subeler": subeler().into_iter().map(|(k, a)| json!({"kod": k, "ad": a})).collect::<Vec<_>>(),
                "cati": {
                    "bagli": c["bagli"].as_bool() == Some(true),
                    "son_esitleme": c["son_esitleme"],
                    "son_deneme": c["son_deneme"],
                    "son_hata": c["son_hata"],
                    "sube": c["sube_cati"],
                },
                "kisi_bos_iletisi": if kisi_sayisi == 0 { self.kisi_bos_iletisi() } else { String::new() },
            }),
        )
    }

    /// POST /api/kurulum {sube?, kisiler:[{ad, rol}]}: ilk kurulum, tek istekte, bir kez.
    pub(crate) fn ilk_kurulum_yap(&self, i: &Istek) -> Yanit {
        if !i.yerel || !i.masaustu {
            return Yanit::hata(
                403,
                "ilk kurulum yalnız bu bilgisayardaki programdan yapılır",
            );
        }
        let (kisi_bos, sube_yok) = match self.ilk_kurulum() {
            IlkKurulum::Acik { kisi_bos, sube_yok } => (kisi_bos, sube_yok),
            IlkKurulum::Kapali(n) => {
                let ileti = match n {
                    "cati" => "Bu bilgisayar TAA Çatı'ya bağlandı; kişiler ve şube TAA Çatı'dan gelir. Değişiklik yönetici girişiyle yapılır.",
                    "yapildi" => "İlk kurulum daha önce yapıldı. Değişiklik yalnız yönetici girişiyle (TAA Çatı PIN) yapılır.",
                    _ => "Kişi kaydı ve şube zaten ayarlı. Değişiklik yalnız yönetici girişiyle (TAA Çatı PIN) yapılır.",
                };
                return Yanit::json(409, json!({ "hata": "ilk_kurulum_kapali", "ileti": ileti }));
            }
        };
        let v: Value = match serde_json::from_slice(&i.govde) {
            Ok(v) => v,
            Err(_) => return Yanit::hata(400, "geçersiz istek"),
        };
        let istenen_sube = v["sube"].as_str().unwrap_or("").trim().to_string();
        let mevcut_sube = self.sube_kodu().ok();
        if sube_yok && istenen_sube.is_empty() {
            return Yanit::hata(422, "şube seçilmeli");
        }
        if !istenen_sube.is_empty() {
            if !subeler().iter().any(|(k, _)| *k == istenen_sube) {
                return Yanit::hata(422, "şube listede yok");
            }
            if let Some(m) = &mevcut_sube {
                if *m != istenen_sube {
                    return Yanit::hata(409, "şube zaten ayarlı; yalnız yönetici değiştirebilir");
                }
            }
        }
        let liste = v["kisiler"].as_array().cloned().unwrap_or_default();
        if kisi_bos && liste.is_empty() {
            return Yanit::hata(422, "en az bir kişi eklenmeli");
        }
        if !kisi_bos && !liste.is_empty() {
            return Yanit::hata(
                409,
                "kişi kaydı boş değil; kişiler yalnız yönetici girişiyle eklenir",
            );
        }
        if liste.len() > EN_COK_ILK_KISI {
            return Yanit::hata(422, "ilk kurulumda en çok 30 kişi eklenir");
        }
        // Önce hepsi denetlenir; biri hatalıysa hiçbir şey yazılmaz.
        let mut temiz = Vec::new();
        for (n, k) in liste.iter().enumerate() {
            let rol = k["rol"].as_str().unwrap_or("personel");
            match Kisiler::denetle(k["ad"].as_str().unwrap_or(""), rol) {
                Ok(ad) => temiz.push((ad, rol.to_string())),
                Err(e) => return Yanit::hata(422, &format!("{}. kişi: {e}", n + 1)),
            }
        }
        let simdi = self.simdi();
        if mevcut_sube.is_none() {
            if let Err(e) = sube_yaz(&self.dizin, &istenen_sube, "ilk-kurulum", simdi) {
                return Yanit::hata(500, &e);
            }
        }
        let kayit = Kisiler::dizinden(&self.dizin);
        let mut eklenen = Vec::new();
        for (ad, rol) in temiz {
            match kayit.ekle_kaynakli(&ad, &rol, &tarih_iso(simdi), "ilk-kurulum") {
                Ok(k) => eklenen.push(k),
                Err(e) => return Yanit::hata(500, &e),
            }
        }
        // Adlar bu dosyaya yazılmaz (yalnız sayı); yol bundan sonra kapalıdır.
        let iz = json!({
            "tarih": format!("{} {}", tarih_iso(simdi), saat_iso(simdi)),
            "sube": self.sube_kodu().ok(),
            "kisi_sayisi": eklenen.len(),
        });
        if let Err(e) = json_yaz(&self.dizin.join(ILK_KURULUM), &iz) {
            return Yanit::hata(500, &e);
        }
        Yanit::json(
            200,
            json!({ "tamam": true, "sube": self.sube_kodu().ok(), "kisiler": eklenen }),
        )
    }

    /// POST /api/kurulum/esitle: TAA Çatı'dan şimdi eşitle (yalnız bu bilgisayardaki program).
    pub(crate) fn kurulum_esitle(&self, i: &Istek) -> Yanit {
        if !i.yerel || !i.masaustu {
            return Yanit::hata(403, "yalnız bu bilgisayardaki programdan");
        }
        match esitle(
            &self.dizin,
            self.kok.as_deref(),
            self.cati.as_ref(),
            self.simdi(),
        ) {
            Ok(o) => Yanit::json(200, json!({ "tamam": true, "ozet": o })),
            Err(e) => Yanit::json(503, json!({ "hata": "cati_ulasilamiyor", "ileti": e })),
        }
    }

    /// POST /api/yonetim/sube {kod}: şube kodunu yönetici değiştirir.
    pub(crate) fn yonetim_sube(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let v: Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let kod = v["kod"].as_str().unwrap_or("").trim();
        if !subeler().iter().any(|(k, _)| k == kod) {
            return Yanit::hata(422, "şube listede yok");
        }
        match sube_yaz(&self.dizin, kod, "yonetici", self.simdi()) {
            Ok(()) => Yanit::json(200, json!({ "tamam": true, "sube": kod })),
            Err(e) => Yanit::hata(422, &e),
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use serde_json::json;

    #[test]
    fn subeler_gomulu_on_bes() {
        let s = subeler();
        assert_eq!(s.len(), 15);
        assert!(s.iter().any(|(k, _)| k == "MRK01"));
    }

    #[test]
    fn gorunen_ad_kisaltir() {
        assert_eq!(gorunen_ad("Ayşe Nur Kaya"), "Ayşe Nur K.");
        assert_eq!(gorunen_ad("Deneme"), "Deneme");
        assert_eq!(gorunen_ad("  ali   şen "), "ali Ş.");
    }

    #[test]
    fn sube_cihaz_durumundan() {
        assert_eq!(
            sube_coz(&json!({"durum":"onayli","sube_id":"PND01"}), || None),
            Some("PND01".into())
        );
        let liste = || Some(json!([{"id":3,"kod":"BRS01"},{"id":4,"kod":"ANK01"}]));
        assert_eq!(
            sube_coz(&json!({"durum":"onayli","sube_id":4}), liste),
            Some("ANK01".into())
        );
        assert_eq!(
            sube_coz(&json!({"durum":"bekliyor","sube_id":4}), liste),
            None
        );
        assert_eq!(
            sube_coz(&json!({"durum":"onayli","sube_id":null}), liste),
            None
        );
    }

    #[test]
    fn rehber_suzgeci() {
        let kisiler = [
            json!({"id":"kisi-1","ad":"Ayşe Kaya","durum":"etkin"}),
            json!({"id":"kisi-2","ad":"Mehmet Demir","durum":"etkin"}),
            json!({"id":"kisi-3","ad":"Eski Personel","durum":"pasif"}),
            json!({"id":"kisi-4","ad":"Merkez Kişi","durum":"etkin"}),
            json!({"id":"kisi-5","ad":"Başka Şube","durum":"etkin"}),
        ];
        let birimler = [
            json!({"id":"b1","tur":"sube","sube_kodu":"MRK01","durum":"etkin"}),
            json!({"id":"b2","tur":"departman","durum":"etkin"}),
            json!({"id":"b3","tur":"sube","sube_kodu":"PND01","durum":"etkin"}),
        ];
        let g = |k: &str, b: &str, rol: &str, bit: &str| json!({"kisi_id":k,"birim_id":b,"rol":rol,"durum":"etkin","baslangic":"2026-01-01","bitis":bit});
        let gorevler = [
            g("kisi-1", "b1", "satis-danismani", ""),
            g("kisi-2", "b1", "terzi", ""),
            g("kisi-3", "b1", "personel", ""),
            g("kisi-4", "b2", "muhasebe", ""),
            g("kisi-5", "b3", "personel", ""),
            g("kisi-1", "b1", "eski", "2026-02-01"),
        ];
        let l = rehberden_kisiler(&kisiler, &birimler, &gorevler, "MRK01", "2026-11-17");
        let ozet: Vec<(&str, &str, &str)> = l
            .iter()
            .map(|k| (k.id.as_str(), k.ad.as_str(), k.rol.as_str()))
            .collect();
        assert_eq!(
            ozet,
            vec![
                ("kisi-1", "Ayşe K.", "danisman"),
                ("kisi-2", "Mehmet D.", "usta")
            ]
        );
        assert!(l.iter().all(|k| k.kaynak == "cati"));
    }

    struct Sahte(Result<CatiVerisi, String>);
    impl CatiKaynagi for Sahte {
        fn cek(&self, _: Option<&str>, _: &str) -> Result<CatiVerisi, String> {
            self.0.clone()
        }
    }

    #[test]
    fn esitleme_onbellek_ve_sube() {
        let d = taa_dosya::gecici_klasor().unwrap();
        // Ulaşılamadı: bağlı sayılmaz, önbellek yok.
        assert!(esitle(&d, None, &Sahte(Err("ağ yok".into())), 1_800_000_000).is_err());
        assert!(!cati_bagli(&d));
        assert_eq!(cati_durumu(&d)["son_hata"], "ağ yok");
        // Boş liste: şube yazılır ama Çatı esas sayılmaz.
        esitle(
            &d,
            None,
            &Sahte(Ok(CatiVerisi {
                sube: Some("PND01".into()),
                kisiler: vec![],
            })),
            1_800_000_000,
        )
        .unwrap();
        assert!(!cati_bagli(&d));
        assert_eq!(sube_oku(&d, None), Some(("PND01".into(), "cati".into())));
        // Dolu liste: Çatı esas, önbellek yazılır.
        let k = Kisi {
            id: "k1".into(),
            ad: "Ayşe K.".into(),
            rol: "danisman".into(),
            tarih: String::new(),
            kaynak: "cati".into(),
        };
        esitle(
            &d,
            None,
            &Sahte(Ok(CatiVerisi {
                sube: Some("BRS01".into()),
                kisiler: vec![k.clone()],
            })),
            1_800_000_000,
        )
        .unwrap();
        assert!(cati_bagli(&d));
        assert_eq!(cati_kisileri(&d), Some(vec![k.clone()]));
        // Yerel şube Çatı'dakinden farklıysa kendiliğinden değişmez.
        assert_eq!(sube_oku(&d, None).unwrap().0, "PND01");
        // Sonra Çatı'ya ulaşılamazsa son önbellek kullanılır.
        assert!(esitle(&d, None, &Sahte(Err("ağ yok".into())), 1_800_000_600).is_err());
        assert!(cati_bagli(&d));
        assert_eq!(kisi::secilebilir(&d, "PND01"), vec![k]);
    }

    #[test]
    fn sube_taa_kur_kokunden() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let kok = taa_dosya::gecici_klasor().unwrap();
        assert_eq!(sube_oku(&d, Some(&kok)), None);
        std::fs::write(kok.join("sube.json"), "{\"kod\":\"IZM01\"}\n").unwrap();
        assert_eq!(
            sube_oku(&d, Some(&kok)),
            Some(("IZM01".into(), "taa-kur".into()))
        );
        std::fs::write(d.join("sube.json"), r#"{"kod":"VAN01"}"#).unwrap();
        assert_eq!(sube_oku(&d, Some(&kok)).unwrap().0, "VAN01");
    }
}
