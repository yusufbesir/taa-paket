//! JSON uçları: saf işlevler (`yonlendir`). Ağ, dosya ve saat dışarıdan (`Ortam`, `Kimlik`) gelir; bu yüzden testlidir.
//! Okuma uçları cihazın kapsamıyla çalışır (docs/rehber-sozlesmesi.md §5): görünürlük SUNUCUDA `taa_rehber_model`
//! görünümüyle uygulanır, istemciden kapsam alınmaz. Yönetici uçları (`/api/yonetim/*`) varsayılan REDDET: geçerli bir
//! yönetici oturumu ve `rehber.yaz` izni yoksa 403.
//! Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]

use crate::durum::{
    Durum, KimlikRed, Ortam, YoneticiOturum, IZIN_AMIR_OKU, IZIN_TUM_OKU, IZIN_YAZ,
};
use crate::gorunum::{self, Tema};
use crate::suzgec::{self, Suzgec, SuzgecHata};
use crate::{belge, ihbar, yonetim, zaman, Yanit};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use taa_rehber_model::model::{gorunum as gorunum_al, Bakis, Gorunum};

/// Bir isteğin işleyici için gereken parçaları.
#[derive(Debug, Clone, Copy)]
pub struct Istek<'a> {
    pub yontem: &'a str,
    pub yol: &'a str,
    pub sorgu: &'a str,
    pub govde: &'a Value,
}

/// `a=b&c=d` → harita; yüzde kodlaması ve `+` çözülür. Geçersiz UTF-8 çifti atlanır.
pub fn sorgu_coz(sorgu: &str) -> BTreeMap<String, String> {
    fn coz(s: &str) -> Option<String> {
        let b = s.as_bytes();
        let mut o = Vec::with_capacity(b.len());
        let mut i = 0;
        while i < b.len() {
            match b[i] {
                b'+' => o.push(b' '),
                b'%' => {
                    let h = s.get(i + 1..i + 3)?;
                    o.push(u8::from_str_radix(h, 16).ok()?);
                    i += 2;
                }
                c => o.push(c),
            }
            i += 1;
        }
        String::from_utf8(o).ok()
    }
    sorgu
        .split('&')
        .filter(|p| !p.is_empty())
        .filter_map(|p| {
            let (k, v) = p.split_once('=').unwrap_or((p, ""));
            Some((coz(k)?, coz(v)?))
        })
        .collect()
}

fn bulunamadi() -> Yanit {
    (404, json!({"hata": "bulunamadı", "kod": "bulunamadi"}))
}

fn yetkisiz() -> Yanit {
    (
        403,
        json!({"hata": "Bu işlem için yönetici girişi gerekir.", "kod": "yetkisiz"}),
    )
}

/// Okuma kapsamı: cihazın şubesi + (yönetici girişliyse) sunucunun verdiği okuma izinleri. Şube yalnız yapılandırmadan gelir.
fn bakis<'a>(d: &'a Durum, bugun: &'a str) -> Bakis<'a> {
    Bakis {
        sube_kodu: d.sube_kodu.as_deref(),
        tum_oku: d.yonetici_izinli(IZIN_TUM_OKU),
        amir_oku: d.yonetici_izinli(IZIN_AMIR_OKU),
        bugun,
        mudur_rolleri: &d.mudur_rolleri,
    }
}

fn gorunum_hesapla(d: &Durum, bugun: &str) -> Gorunum {
    gorunum_al(d.depo.veri(), &bakis(d, bugun))
}

fn hiyerarsi_izinli(d: &Durum) -> bool {
    d.yonetici_izinli(IZIN_AMIR_OKU) || d.yonetici_izinli(IZIN_TUM_OKU)
}

fn durum_json(d: &Durum) -> Value {
    let v = d.depo.veri();
    let kaynak = if d.demo {
        "ornek"
    } else if v.kisiler.is_empty() && v.birimler.is_empty() {
        "yok"
    } else {
        "bellek"
    };
    json!({
        "surum": env!("CARGO_PKG_VERSION"),
        "demo": d.demo,
        "veri_kaynagi": kaynak,
        "kapsam": {
            "sube_kodu": d.sube_kodu,
            "tum_oku": d.yonetici_izinli(IZIN_TUM_OKU),
            "amir_oku": d.yonetici_izinli(IZIN_AMIR_OKU),
        },
        "yonetici": {
            "girisli": d.yonetici.is_some(),
            "kullanici": d.yonetici.as_ref().map(|y| y.kullanici.as_str()),
            "yazma_izni": d.yonetici_izinli(IZIN_YAZ),
        },
        "yazma": {
            "kalici": false,
            "mesaj": "Kayıtlar yalnız bu pencere açıkken bellekte tutulur: sunucu yazma hizmeti (rehber.yaz@1) bu sürümde bağlı değil.",
        },
        "ihbar": {
            "iletim_bagli": false,
            "ayarli": d.ihbar.ayar.is_some(),
            "mesaj": "Gönderim henüz bağlı değil: ihbar bu bilgisayardaki deftere yazılır, yetkiliye henüz iletilmez.",
        },
        "tema": d.tema.kod(),
    })
}

fn suzgec_hatasi(e: SuzgecHata) -> Yanit {
    (400, json!({"hata": e.to_string(), "kod": "gecersiz"}))
}

fn kisiler(d: &Durum, bugun: &str, q: &BTreeMap<String, String>) -> Yanit {
    let al = |ad: &str| {
        q.get(ad)
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .map(str::to_string)
    };
    let s = Suzgec {
        ara: q.get("ara").cloned().unwrap_or_default(),
        sube: al("sube"),
        gorev: al("gorev"),
        birim: al("birim"),
    };
    // Aşırı uzun arama dizisi işlenmez (kaynak tüketimi).
    if s.ara.chars().count() > 200 {
        return (400, json!({"hata": "arama çok uzun", "kod": "gecersiz"}));
    }
    let g = gorunum_hesapla(d, bugun);
    match suzgec::kisiler(d.depo.veri(), &g, &s, hiyerarsi_izinli(d), bugun) {
        Ok(v) => (200, v),
        Err(e) => suzgec_hatasi(e),
    }
}

fn el_kitabi_listesi(d: &Durum) -> Yanit {
    let konular: Vec<Value> = d
        .belgeler
        .konular()
        .iter()
        .map(|k| {
            json!({
                "konu_id": k.konu_id, "baslik": k.baslik, "surum": k.surum, "yayin": k.yayin,
                "yenilik_notu": k.yenilik_notu, "etiketler": k.etiketler,
            })
        })
        .collect();
    (200, json!({"sema": "rehber.belge@1", "konular": konular}))
}

fn el_kitabi_oku(d: &Durum, q: &BTreeMap<String, String>) -> Yanit {
    let Some(konu) = q.get("konu").filter(|k| !k.is_empty()) else {
        return (400, json!({"hata": "konu gerekli", "kod": "gecersiz"}));
    };
    let surum = match q.get("surum").filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => match s.parse::<u64>() {
            Ok(n) => Some(n),
            Err(_) => {
                return (
                    400,
                    json!({"hata": "surum tam sayı olmalı", "kod": "gecersiz"}),
                )
            }
        },
    };
    match d.belgeler.oku(konu, surum) {
        Ok(k) => (
            200,
            json!({
                "konu_id": k.konu_id, "surum": k.surum, "surumler": d.belgeler.surumler(&k.konu_id),
                "baslik": k.baslik, "yayin": k.yayin, "yenilik_notu": k.yenilik_notu, "etiketler": k.etiketler,
                "bloklar": belge::bloklar(&k.govde),
            }),
        ),
        Err(_) => (
            404,
            json!({"hata": "Bölüm açılamadı: konu ya da sürüm yok.", "kod": "bulunamadi"}),
        ),
    }
}

fn yonetici_giris(d: &mut Durum, o: &dyn Ortam, g: &Value) -> Yanit {
    let kullanici = g
        .get("kullanici")
        .and_then(Value::as_str)
        .map(str::trim)
        .unwrap_or("");
    let parola = g.get("parola").and_then(Value::as_str).unwrap_or("");
    if kullanici.is_empty() || parola.is_empty() {
        return (
            400,
            json!({"hata": "Kullanıcı adı ve parola gerekli.", "kod": "gecersiz"}),
        );
    }
    match d.kimlik.giris(kullanici, parola) {
        Ok(b) if b.izinler.iter().any(|i| i == IZIN_YAZ) => {
            let simdi = o.simdi();
            d.yonetici = Some(YoneticiOturum {
                kullanici: b.kullanici.clone(),
                izinler: b.izinler,
                son_etkinlik: simdi,
            });
            d.denetim_yaz(&zaman::zaman(simdi), "yonetici", "giris", "-");
            (200, json!({"girisli": true, "kullanici": b.kullanici}))
        }
        Ok(_) => {
            d.yonetici = None;
            (
                403,
                json!({"hata": "Bu hesapta rehber yazma izni (rehber.yaz) yok; yönetici işlemleri açılmadı.", "kod": "yetkisiz"}),
            )
        }
        Err(KimlikRed::Gecersiz(m)) => (400, json!({"hata": m, "kod": "gecersiz"})),
        Err(KimlikRed::Reddedildi(m)) => (401, json!({"hata": m, "kod": "reddedildi"})),
        // Doğrulanamayan kimlik yönetici sayılmaz (varsayılan REDDET).
        Err(KimlikRed::Dogrulanamadi(m)) => (
            403,
            json!({"hata": format!("Kimlik doğrulanamadı, yönetici işlemleri kapalı: {m}"), "kod": "dogrulanamadi"}),
        ),
    }
}

fn gorunum_yaz(d: &mut Durum, g: &Value) -> Yanit {
    let Some(t) = g.get("tema").and_then(Value::as_str).and_then(Tema::koddan) else {
        return (
            400,
            json!({"hata": "tema: acik, koyu ya da tek olmalı", "kod": "gecersiz"}),
        );
    };
    d.tema = t;
    match gorunum::kaydet(&d.gorunum_yolu, t) {
        Ok(()) => (200, json!({"tema": t.kod(), "kalici": true})),
        Err(_) => (
            200,
            json!({"tema": t.kod(), "kalici": false, "uyari": "Tercih bu cihazda saklanamadı; yalnız bu oturumda geçerli."}),
        ),
    }
}

/// Yönetici uçları. Oturum yok / izin yok / süre dolmuş → 403 (varsayılan REDDET).
fn yonetim_ucu(d: &mut Durum, o: &dyn Ortam, i: &Istek) -> Yanit {
    if !d.yonetici_izinli(IZIN_YAZ) {
        return yetkisiz();
    }
    let simdi = o.simdi();
    if let Some(y) = d.yonetici.as_mut() {
        y.son_etkinlik = simdi;
    }
    match (i.yontem, i.yol) {
        ("GET", "/api/yonetim/secenekler") => (200, yonetim::secenekler(d)),
        ("GET", "/api/yonetim/denetim") => (200, json!({"satirlar": d.denetim})),
        ("POST", "/api/yonetim/kisi/ekle") => yonetim::kisi_ekle(d, o, i.govde),
        ("POST", "/api/yonetim/kisi/duzelt") => yonetim::kisi_duzelt(d, o, i.govde),
        ("POST", "/api/yonetim/birim/ekle") => yonetim::birim_ekle(d, o, i.govde),
        ("POST", "/api/yonetim/birim/duzelt") => yonetim::birim_duzelt(d, o, i.govde),
        ("POST", "/api/yonetim/gorevlendirme/ekle") => yonetim::gorevlendirme_ekle(d, o, i.govde),
        _ => bulunamadi(),
    }
}

/// Tüm API yönlendirmesi.
pub fn yonlendir(d: &mut Durum, o: &dyn Ortam, i: &Istek) -> Yanit {
    let simdi = o.simdi();
    d.yonetici_suresi_kontrol(simdi);
    let bugun = zaman::tarih(simdi);
    let q = sorgu_coz(i.sorgu);
    match (i.yontem, i.yol) {
        ("GET", "/api/durum") => (200, durum_json(d)),
        ("GET", "/api/suzgecler") => (200, suzgec::suzgecler(&gorunum_hesapla(d, &bugun))),
        ("GET", "/api/agac") => {
            let g = gorunum_hesapla(d, &bugun);
            (
                200,
                json!({"dugumler": suzgec::agac(d.depo.veri(), &g, hiyerarsi_izinli(d))}),
            )
        }
        ("GET", "/api/kisiler") => kisiler(d, &bugun, &q),
        ("GET", "/api/el-kitabi") => el_kitabi_listesi(d),
        ("GET", "/api/el-kitabi/oku") => el_kitabi_oku(d, &q),
        ("GET", "/api/ihbar/kategoriler") => (200, ihbar::kategoriler()),
        ("POST", "/api/ihbar") => ihbar::gonder(d, o, i.govde),
        ("POST", "/api/yonetici/giris") => yonetici_giris(d, o, i.govde),
        ("POST", "/api/yonetici/cikis") => {
            d.yonetici = None;
            (200, json!({"girisli": false}))
        }
        ("GET", "/api/gorunum") => (200, json!({"tema": d.tema.kod()})),
        ("POST", "/api/gorunum") => gorunum_yaz(d, i.govde),
        (_, yol) if yol.starts_with("/api/yonetim/") => yonetim_ucu(d, o, i),
        _ => bulunamadi(),
    }
}
