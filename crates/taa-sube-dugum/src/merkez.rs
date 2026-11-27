// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-sube-dugum 2026-11-27]
//! Merkeze arka plan gönderimi. Düğüm merkezin yerini tutmaz: aldığı her kayıt `taa_kuyruk::Kuyruk::gonder` ile
//! sırayla, aynı `no` üstünden merkeze yollanır. Merkez yoksa gönderim o kayıtta durur ve bekler; sıra bozulmaz.
//!
//! Bu modülün `taa-kuyruk` davranışına eklediği iki uyarlama (docs/WEB-ESITLE-SOZLESMESI.md §4.3):
//! - **401** kalıcı ret sayılmaz (süresi dolan oturum bütün kuyruğu reddettirmesin); kayıt bekler.
//! - **409** `cakisma` / `sube_uyusmaz` / `surum_eski` / `no_uyusmaz` kalıcıdır; kayıt `reddedildi.jsonl`e düşer ve
//!   kuyruğu tıkamaz. Öteki 409'lar (`taa-kuyruk`taki `tekrar`) eskisi gibi bekler.

use crate::{gunluk, Dugum, DugumHata};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use taa_kuyruk::{CurlGonderici, GonderHata, Gonderici, Imzalayici};

pub const BASLIK_NO: &str = "X-TAA-No";
pub const BASLIK_SUBE: &str = "X-TAA-Sube";
/// Merkezin kalıcı saydığı 409 kodları.
pub const KALICI_409: &[&str] = &["cakisma", "sube_uyusmaz", "surum_eski", "no_uyusmaz"];
/// Döngünün iki deneme arasındaki varsayılan bekleme süresi (saniye).
pub const ARALIK_SN_VARSAYILAN: u64 = 30;

/// Merkeze giden her isteğe `X-TAA-Sube` ve (POST gövdesinde `no` varsa) `X-TAA-No` başlığını ekler; verilmişse
/// cihaz imzalayıcısının başlıklarını da katar.
struct Basliklar {
    sube: String,
    cihaz: Option<Arc<dyn Imzalayici>>,
}

impl Imzalayici for Basliklar {
    fn basliklar(
        &self,
        yontem: &str,
        yol: &str,
        govde: &[u8],
    ) -> Result<Vec<(String, String)>, String> {
        let mut v = vec![(BASLIK_SUBE.to_string(), self.sube.clone())];
        if yontem == "POST" {
            let no = serde_json::from_slice::<Value>(govde)
                .ok()
                .and_then(|g| g.get("no").and_then(Value::as_str).map(str::to_string));
            if let Some(no) = no.filter(|n| taa_kuyruk::no_gecerli(n)) {
                v.push((BASLIK_NO.to_string(), no));
            }
        }
        if let Some(c) = &self.cihaz {
            v.extend(c.basliklar(yontem, yol, govde)?);
        }
        Ok(v)
    }
}

/// Merkez göndericisini düğüm başlıklarıyla donatır.
///
/// **Cihaz imza kancası:** `cihaz` olarak `taa_kuyruk::Imzalayici` verilir. `taa_cihaz::CihazAnahtari` bu arayüzü
/// gerçekler; bağlamak için çağıran taraf `Some(Arc::new(CihazAnahtari::yukle(kok)?))` verir. Bu crate `taa-cihaz`a
/// bağlı değildir ve imza üretmez: `None` verilirse istekler cihaz imzasız gider.
pub fn baslikli(g: CurlGonderici, sube: &str, cihaz: Option<Arc<dyn Imzalayici>>) -> CurlGonderici {
    g.imzalayici(Arc::new(Basliklar {
        sube: sube.to_string(),
        cihaz,
    }))
}

fn kalici_409(mesaj: &str) -> bool {
    let sade: String = mesaj.chars().filter(|c| !c.is_whitespace()).collect();
    sade.contains("HTTP409:")
        && KALICI_409
            .iter()
            .any(|k| sade.contains(&format!("\"hata\":\"{k}\"")))
}

/// Gönderim sonucunu sözleşmeye uyarlar (modül başındaki iki kural).
pub fn uyarla(r: Result<Value, GonderHata>) -> Result<Value, GonderHata> {
    match r {
        Err(GonderHata::Reddedildi { kod: 401, mesaj }) => Err(GonderHata::Ulasilamadi(format!(
            "HTTP 401: {mesaj}. Öneri: merkez oturumunu yenileyin; kayıtlar bekliyor."
        ))),
        Err(GonderHata::Ulasilamadi(m)) if kalici_409(&m) => Err(GonderHata::Reddedildi {
            kod: 409,
            mesaj: m.chars().take(300).collect(),
        }),
        r => r,
    }
}

/// [`uyarla`]yı her POST'a uygulayan sarmalayıcı.
pub struct Uyarlayan<'a>(pub &'a dyn Gonderici);

impl Gonderici for Uyarlayan<'_> {
    fn gonder(&self, uc: &str, govde: &Value) -> Result<Value, GonderHata> {
        uyarla(self.0.gonder(uc, govde))
    }
    fn al(&self, uc: &str) -> Result<Value, GonderHata> {
        self.0.al(uc)
    }
}

/// Bir gönderim turunun özeti.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TurOzeti {
    pub gonderilen: usize,
    pub reddedilen: usize,
    pub kalan: usize,
    pub gunluk_gonderilen: usize,
    /// Bu turda merkezden yanıt alındı mı.
    pub ulasti: bool,
    pub durma_nedeni: Option<String>,
}

/// Bir tur: biten günlerin özetini kuyruğa ekler, bekleyen kayıtları sırayla merkeze yollar, sonra günlük
/// özetlerini yollar, sürüm sayacını ve günlüğü günceller. Merkez yoksa hiçbir şey kaybolmaz, tur `ulasti = false` döner.
pub fn tur(d: &Dugum, g: &dyn Gonderici) -> Result<TurOzeti, DugumHata> {
    let g = Uyarlayan(g);
    gunluk::ozetleri_kuyrukla(d, &gunluk::bugun())?;
    let o = d.kuyruk().gonder(&g, 0)?;
    let mut ozet = TurOzeti {
        gonderilen: o.gonderilen,
        reddedilen: o.reddedilen,
        kalan: o.kalan,
        durma_nedeni: o.durma_nedeni,
        ..TurOzeti::default()
    };
    ozet.ulasti = ozet.gonderilen + ozet.reddedilen > 0;
    if ozet.durma_nedeni.is_none() {
        let go = d.gunluk_kuyruk().gonder(&g, 0)?;
        ozet.gunluk_gonderilen = go.gonderilen;
        ozet.ulasti |= go.gonderilen + go.reddedilen > 0;
        if !ozet.ulasti && go.durma_nedeni.is_none() {
            // Gönderilecek bir şey yoktu: merkez ayakta mı diye yokla (taa-yol ile aynı ölçüt).
            ozet.ulasti = taa_yol::yanit_ulasildi_mi(&g.al(taa_yol::SAGLIK_UCU));
        }
    }
    if ozet.ulasti {
        d.merkez_goruldu()?;
    }
    for olay in d.surum_isle()? {
        match olay.olay {
            "gonderildi" => d.gunluk(
                "merkez.gonderildi",
                json!({"no": olay.no, "surum": olay.surum}),
            ),
            "reddedildi" => d.gunluk(
                "merkez.reddetti",
                json!({"no": olay.no, "surum": olay.surum}),
            ),
            _ => {}
        }
    }
    Ok(ozet)
}

/// Arka plan döngüsü: `dur` işaretlenene kadar tur atar; turlar arasında `aralik` kadar ya da yeni kayıt gelene
/// kadar bekler. `kur` her turda göndericiyi yeniden kurar (yol listesi değişmiş olabilir); `None` dönerse merkez
/// yapılandırılmamıştır, kayıtlar birikir. Merkez durumunun değişimi (ulaşıldı / ulaşılamadı) günlüğe bir kez yazılır.
pub fn dongu(
    d: &Dugum,
    kur: &dyn Fn() -> Option<Box<dyn Gonderici>>,
    aralik: Duration,
    dur: &AtomicBool,
) {
    let mut onceki: Option<bool> = None;
    while !dur.load(Ordering::SeqCst) {
        let (ulasti, neden) = match kur() {
            None => (false, Some("merkez adresi yapılandırılmamış".to_string())),
            Some(g) => match tur(d, g.as_ref()) {
                Ok(o) => (o.ulasti, o.durma_nedeni),
                Err(e) => (false, Some(e.to_string())),
            },
        };
        if onceki != Some(ulasti) {
            if ulasti {
                d.gunluk("merkez.ulasildi", json!({}));
            } else {
                let neden: String = neden.unwrap_or_default().chars().take(300).collect();
                d.gunluk("merkez.ulasilamadi", json!({"neden": neden}));
            }
            onceki = Some(ulasti);
        }
        if dur.load(Ordering::SeqCst) {
            break;
        }
        d.bekle(aralik);
    }
}

/// Merkez göndericisini kurar. Öncelik: `acik_merkez` (`--merkez` / `TAA_SUNUCU`) → `taa-yol` yol listesi
/// (`<kök>/yol/sunucu-yollari.json`; yollar sırayla denenir). İkisi de yoksa `None`: kayıtlar düğümde birikir.
/// `oturum` (Bearer) verilirse her isteğe eklenir; bu crate onu hiçbir yere yazmaz. `cihaz`: bkz. [`baslikli`].
pub fn gonderici_kur(
    kok: &std::path::Path,
    sube: &str,
    acik_merkez: Option<&str>,
    sertifika: Option<&std::path::Path>,
    oturum: Option<&str>,
    cihaz: Option<Arc<dyn Imzalayici>>,
) -> Result<Option<Box<dyn Gonderici>>, String> {
    let yap =
        |taban: &str, ca: Option<std::path::PathBuf>| -> Result<Box<dyn Gonderici>, GonderHata> {
            let mut g = CurlGonderici::yeni(taban)?.baglanti_suresi(taa_yol::DENEME_BAGLANTI_SN);
            if let Some(o) = oturum.map(str::trim).filter(|o| !o.is_empty()) {
                g = g.oturum(o)?;
            }
            if let Some(c) = ca {
                g = g.sertifika(&c);
            }
            Ok(Box::new(baslikli(g, sube, cihaz.clone())))
        };
    if let Some(m) = acik_merkez.map(str::trim).filter(|m| !m.is_empty()) {
        return yap(m, sertifika.map(std::path::Path::to_path_buf))
            .map(Some)
            .map_err(|e| e.to_string());
    }
    let fabrika = |y: &taa_yol::Yol, ca: Option<std::path::PathBuf>| {
        yap(&y.taban, sertifika.map(std::path::Path::to_path_buf).or(ca))
    };
    match taa_yol::yedekli_kur(kok, &fabrika) {
        Ok(Some(g)) => Ok(Some(Box::new(g))),
        Ok(None) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}
