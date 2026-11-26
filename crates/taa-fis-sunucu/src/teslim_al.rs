//! Teslim alma ucu: ürünü teslim alan kişinin adı, ilişkisi ve imzası; müşteri fişiyle gelmediyse
//! (fişsiz teslim) telefonun son 4 hanesiyle doğrulama. Yusuf: "ürünü teslim alan kişi için imza alıyoruz,
//! bazen müşteri fişiyle gelmiyor".
//! - Fişsiz teslim "fissiz-teslim" yetkisi ister (varsayılan yalnız mağaza bilgisayarı).
//! - Son 4 hane hiçbir yere yazılmaz; kayıtta yalnız doğrulama yöntemi ("telefon-son4") kalır.
//! - Fiş başına 5 yanlışta 15 dk kilit (bellekte; yeniden başlatınca sıfırlanır).
//! - İmza PNG'si `imzalar/<no>-<n>.png.enc` olarak yerel anahtarla ŞİFRELİ (AES-256-GCM, aad = fiş no) ve atomik
//!   yazılır; şifresiz PNG'nin SHA-256 özeti ve imza yöntemi ("qr_imza" / "cihaz_imza") defterdeki kayda girer.
//! - Fişsiz teslimde İMZA ZORUNLU (Yusuf 2026-11-26: "telefon üzerinden de imza alınsın dokunmatik ekran ile —
//!   fişi yanında olmayan için"): karekod sayfasında müşterinin kendi telefonunda (qr_imza) ya da mağaza
//!   cihazının ekranında (cihaz_imza). İmza yoksa 422 `imza_gerekli`. Fişli teslimde imza isteğe bağlıdır.
//! - Teslim alan bilgisi müşteri takip (bulut) kaydına GİTMEZ; bulut kaydı yalnız sabit anahtarlar taşır.
//! - Karekodla onay (2026-11-26, bkz. teslim_onay.rs): `onay_belirteci` verilirse ve müşteri kendi telefonundan
//!   onayladıysa teslim "qr" (fişsizde "qr+telefon-son4") doğrulamasıyla yapılır. Karekod onayında imza buluttan
//!   (onay içinde) gelir; gelmediyse cihaz imzası alınabilir. Karekod yoksa ekran imzası "telefonu olmayan müşteri"
//!   yedeğidir.
//! - Teslim alanın adı, telefonu, cihazı (user-agent) ve IP'si deftere yalnız ŞİFRELİ yazılır (teslim_sifre.rs);
//!   `teslim_alan` düz metin alanı yeni kayıtlarda boştur.

use super::*;
use std::collections::BTreeMap;

/// teslim-al isteğinin gövde sınırı (imza PNG'si base64 olarak gelir).
pub(crate) const GOVDE_SINIRI_TESLIM_AL: usize = 512 * 1024;
/// Çözülmüş imza PNG'si en çok bu kadar bayt (cihaz ve karekod imzası için aynı).
pub(crate) const EN_BUYUK_IMZA: usize = 300 * 1024;
/// Bir fişe en çok kaç imza dosyası yazılabilir (dosya adı sırası).
const EN_COK_IMZA: usize = 20;
const SON4_EN_COK_YANLIS: u32 = 5;
const SON4_KILIT_SN: u64 = 15 * 60;
const PNG_IMZASI: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
/// İmza görüntüsünün en büyük eni/boyu (piksel).
const EN_BUYUK_IMZA_KENARI: u32 = 8192;

/// (veri klasörü | fiş no) -> (yanlış sayısı, kilit bitişi). Bellekte; diske yazılmaz.
static SON4_YANLIS: Mutex<BTreeMap<String, (u32, u64)>> = Mutex::new(BTreeMap::new());

/// Fiş no yalnız harf/rakam/tire ise dosya adında kullanılır (yol kaçışına karşı).
pub(crate) fn no_guvenli(no: &str) -> bool {
    !no.is_empty() && no.len() <= 40 && no.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Sabit zamanlı karşılaştırma (uzunluk eşitse her baytı gezer).
pub(crate) fn sabit_esit(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Standart base64 (RFC 4648) çözücü; isteğe bağlı `data:image/png;base64,` öneki ve boşluklar kabul edilir.
pub(crate) fn base64_coz(s: &str) -> Result<Vec<u8>, String> {
    let s = s
        .trim()
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(s.trim());
    let deger = |c: u8| -> Option<u32> {
        Some(match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => return None,
        } as u32)
    };
    let temiz: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    let govde = temiz
        .strip_suffix(b"==")
        .or_else(|| temiz.strip_suffix(b"="))
        .unwrap_or(&temiz);
    if govde.len() % 4 == 1 || (temiz.len() != govde.len() && !temiz.len().is_multiple_of(4)) {
        return Err("imza base64 değil".into());
    }
    let mut cikti = Vec::with_capacity(govde.len() * 3 / 4);
    for parca in govde.chunks(4) {
        let mut n: u32 = 0;
        for (i, c) in parca.iter().enumerate() {
            n |= deger(*c).ok_or("imza base64 değil")? << (18 - 6 * i);
        }
        let b = n.to_be_bytes();
        cikti.extend_from_slice(&b[1..parca.len()]);
    }
    Ok(cikti)
}

/// PNG mi: 8 baytlık tam imza ve ardından IHDR parçası (uzunluk 13, makul en/boy).
/// Yalnız ilk baytlara bakmak keyfi verinin "imza" diye saklanmasına izin verirdi.
pub(crate) fn png_gecerli(b: &[u8]) -> bool {
    if b.len() < 33 || b[..8] != PNG_IMZASI {
        return false;
    }
    let u32_be = |i: usize| u32::from_be_bytes([b[i], b[i + 1], b[i + 2], b[i + 3]]);
    let (uzunluk, tur, en, boy) = (u32_be(8), &b[12..16], u32_be(16), u32_be(20));
    uzunluk == 13
        && tur == b"IHDR"
        && (1..=EN_BUYUK_IMZA_KENARI).contains(&en)
        && (1..=EN_BUYUK_IMZA_KENARI).contains(&boy)
}

/// İmza dosyası adı "<no>-<n>.png.enc" (yeni) ya da "<no>-<n>.png" (eski, geçiş öncesi) ise (sahip fiş no, n).
pub(crate) fn imza_dosya_adi_coz(ad: &str) -> Option<(&str, &str)> {
    let kok = ad
        .strip_suffix(".png.enc")
        .or_else(|| ad.strip_suffix(".png"))?;
    let (no, n) = kok.rsplit_once('-')?;
    (no_guvenli(no) && !n.is_empty() && n.len() <= 3 && n.bytes().all(|b| b.is_ascii_digit()))
        .then_some((no, n))
}

/// Base64 imzayı çözer ve denetler: ≤300 KB, PNG imzası + IHDR, sonra İÇERİK (imza_png: PNG gerçekten çözülür,
/// CRC/boy tutmalı, yeterince koyu piksel olmalı). Hata: (HTTP kodu, ileti); boş ya da çözülemeyen imza 422.
pub(crate) fn imza_coz_denetle(s: &str) -> Result<Vec<u8>, (u16, String)> {
    let b = base64_coz(s).map_err(|e| (400, e))?;
    if b.len() > EN_BUYUK_IMZA {
        return Err((413, "imza en çok 300 KB olmalı".into()));
    }
    if !png_gecerli(&b) {
        return Err((400, "imza PNG olmalı".into()));
    }
    crate::imza_png::imza_yeterli(&b).map_err(|e| (422, format!("imza geçersiz: {e}")))?;
    Ok(b)
}

pub(crate) fn sha256_bayt(b: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

impl Uygulama {
    fn son4_anahtari(&self, no: &str) -> String {
        format!("{}|{no}", self.dizin.display())
    }

    /// Kilitliyse kalan saniye.
    pub(crate) fn son4_kilitli(&self, no: &str) -> Option<u64> {
        let simdi = self.simdi();
        let m = SON4_YANLIS.lock().ok()?;
        match m.get(&self.son4_anahtari(no)) {
            Some((_, bitis)) if *bitis > simdi => Some(bitis - simdi),
            _ => None,
        }
    }

    /// Yanlış denemeyi sayar; kalan hakkı döndürür (0 = şimdi kilitlendi).
    pub(crate) fn son4_yanlis(&self, no: &str) -> u32 {
        let simdi = self.simdi();
        let Ok(mut m) = SON4_YANLIS.lock() else {
            return 0;
        };
        let k = m.entry(self.son4_anahtari(no)).or_insert((0, 0));
        if k.1 != 0 && k.1 <= simdi {
            *k = (0, 0); // kilit süresi doldu: sayaç baştan
        }
        k.0 += 1;
        if k.0 >= SON4_EN_COK_YANLIS {
            *k = (SON4_EN_COK_YANLIS, simdi + SON4_KILIT_SN);
            0
        } else {
            SON4_EN_COK_YANLIS - k.0
        }
    }

    pub(crate) fn son4_sifirla(&self, no: &str) {
        if let Ok(mut m) = SON4_YANLIS.lock() {
            m.remove(&self.son4_anahtari(no));
        }
    }

    /// Teslimde bağlı fiş sorusu (durum ucu ve teslim-al aynı kuralı kullanır).
    /// `v["bagli"]` yoksa ve açık bağlı fiş varsa 409 `bagli_teslim` sorusu döner; liste verilmişse
    /// seçilenler hazır olmalı. Ok: birlikte teslim edilecek fiş numaraları.
    pub(crate) fn bagli_teslim_secimi(
        &self,
        d: &Defter,
        no: &str,
        v: &serde_json::Value,
    ) -> Result<Vec<String>, Yanit> {
        let mut birlikte: Vec<String> = Vec::new();
        let Ok(Some(f)) = d.bul(no) else {
            return Ok(birlikte);
        };
        let acik: Vec<taa_fis::Fis> = f
            .bagli
            .iter()
            .filter_map(|n| d.bul(n).ok().flatten())
            .filter(|x| matches!(x.durum, Durum::Bekliyor | Durum::Terzihanede | Durum::Hazir))
            .collect();
        match v.get("bagli").and_then(|b| b.as_array()) {
            None if !acik.is_empty() => {
                let liste: Vec<_> = acik
                    .iter()
                    .map(|x| json!({ "no": x.no, "musteri": x.musteri, "durum": x.durum, "hazir": x.durum == Durum::Hazir }))
                    .collect();
                return Err(Yanit::json(
                    409,
                    json!({
                        "soru": "bagli_teslim",
                        "ileti": "Bu fişe bağlı başka fişler var. Onlar da şimdi teslim edilsin mi?",
                        "bagli": liste,
                    }),
                ));
            }
            Some(sec) => {
                for n in sec.iter().filter_map(|x| x.as_str()) {
                    match acik.iter().find(|x| x.no == n) {
                        Some(x) if x.durum == Durum::Hazir => birlikte.push(n.to_string()),
                        Some(_) => {
                            return Err(Yanit::hata(
                                409,
                                &format!("{n} henüz hazır değil; birlikte teslim edilemez"),
                            ))
                        }
                        None => {
                            return Err(Yanit::hata(
                                409,
                                &format!("{n} bu fişe bağlı açık bir fiş değil"),
                            ))
                        }
                    }
                }
            }
            None => {}
        }
        Ok(birlikte)
    }

    /// POST /api/fis/<no>/teslim-al
    /// Fişli/fişsiz (2026-11-26, teslim sepeti kararı): `fisli: true` yalnız `fis_karekodu` (okutulan karekod metni)
    /// bu fişin karekod kanıtıysa geçerlidir (bkz. teslim_sepet.rs: müşteri nüshasının takip karekodu; takip ayarlı
    /// değilse personel karekodu). Kanıt yoksa istek FİŞSİZ kurallarıyla (yetki + son 4 + imza) değerlendirilir ve
    /// hata yanıtlarına `"fis_karekodu_yok": true` eklenir. Kasiyerin "fişli" işaretini elle koyma yolu yoktur.
    pub(crate) fn teslim_al(&self, d: &Defter, no: &str, c: &cihaz::Cihaz, i: &Istek) -> Yanit {
        let v: serde_json::Value = match serde_json::from_slice(&i.govde) {
            Ok(v @ serde_json::Value::Object(_)) => v,
            _ => return Yanit::hata(400, "geçersiz istek (JSON nesnesi bekleniyor)"),
        };
        if !no_guvenli(no) {
            return Yanit::hata(400, "fiş no geçersiz");
        }
        let Some(fisli_istenen) = v["fisli"].as_bool() else {
            return Yanit::hata(400, "fisli alanı true/false olmalı");
        };
        let (kanitli, gecisle) =
            self.fis_karekodu_kaniti(d, no, v["fis_karekodu"].as_str().unwrap_or(""));
        let fisli = fisli_istenen && kanitli;
        // Fişli sayılması yalnız eski fiş geçiş süresinden geliyorsa günlüğe (teslim kaydından hemen önce) yazılır.
        let gecis = fisli && gecisle;
        // Bağlı fişlerle birlikte teslim ("bagli": [..] dolu): istek iç olarak teslim sepetine çevrilir. Bağlı fişlerin
        // karekodu okutulmadığı için FİŞSİZDİR (D kararı): fişsiz teslim yetkisi, telefon son 4 ve imza zorunlu;
        // yazım da sepetteki gibi ya hepsi ya hiçbiri. (Eskiden ana fişin kaydı fisli=true kopyalanıyordu.)
        let bagli_dolu = v
            .get("bagli")
            .and_then(|b| b.as_array())
            .is_some_and(|b| !b.is_empty());
        let y = if bagli_dolu {
            self.bagli_sepetle_teslim(d, no, c, &v)
        } else {
            self.teslim_al_ic(d, no, c, &v, fisli, gecis)
        };
        if fisli_istenen && !fisli && matches!(y.kod, 403 | 422 | 429) {
            // Fişli istendi ama karekod kanıtı yok: neden fişsiz kuralların istendiği açıkça yazılsın.
            if let Ok(mut j) = serde_json::from_slice::<serde_json::Value>(&y.govde) {
                j["fis_karekodu_yok"] = json!(true);
                let eski = j["ileti"]
                    .as_str()
                    .or_else(|| j["hata"].as_str())
                    .unwrap_or("")
                    .to_string();
                j["ileti"] = json!(format!(
                    "Fiş karekodu okutulmadı: fişsiz teslim kuralları geçerli (yetki, telefon son 4, imza). {eski}"
                ));
                return Yanit::json(y.kod, j);
            }
        }
        y
    }

    /// teslim-al + `"bagli": [..]`: seçim eski uçtaki gibi denetlenir (bağlı ve hazır olmalı; 409 iletileri aynı),
    /// sonra istek `teslim_sepet_ic`'e verilir: ana fiş `fisli`/`fis_karekodu` ile (kanıt sunucuda yeniden
    /// doğrulanır), bağlı fişler `okutuldu: false` (fişsiz). Eski tek `telefon_son4` metni sepette tek telefon grubu
    /// varsa ona uygulanır; farklı telefonlu fişlerde `{<no>: "1234"}` nesnesi gerekir (422 `son4_gerekli` hangi
    /// fişler için olduğunu söyler). Tek fişe açılmış karekod belirteci bütün sepeti onaylamaz: 409 `sepet_gerekli`.
    fn bagli_sepetle_teslim(
        &self,
        d: &Defter,
        no: &str,
        c: &cihaz::Cihaz,
        v: &serde_json::Value,
    ) -> Yanit {
        let birlikte = match self.bagli_teslim_secimi(d, no, v) {
            Ok(b) => b,
            Err(y) => return y,
        };
        if v["onay_belirteci"].as_str().is_some_and(|t| !t.is_empty()) {
            return Yanit::json(
                409,
                json!({
                    "hata": "sepet_gerekli",
                    "ileti": "Bu karekod yalnız bu fiş içindir. Bağlı fişlerle birlikte karekodlu teslim teslim sepetinden yapılır (POST /api/teslim/sepet/onay-ac).",
                }),
            );
        }
        let mut fisler = vec![json!({
            "no": no,
            "okutuldu": v["fisli"].as_bool().unwrap_or(false),
            "karekod": v["fis_karekodu"].as_str().unwrap_or(""),
        })];
        fisler.extend(
            birlikte
                .iter()
                .map(|b| json!({ "no": b, "okutuldu": false })),
        );
        let mut g = v.clone();
        if let Some(o) = g.as_object_mut() {
            for k in ["bagli", "fisli", "fis_karekodu"] {
                o.remove(k);
            }
            o.insert("fisler".into(), json!(fisler));
        }
        let mut y = self.teslim_sepet_ic(d, c, &g);
        if y.kod == 200 {
            if let Ok(mut j) = serde_json::from_slice::<serde_json::Value>(&y.govde) {
                // Eski yanıtla uyum: atlanan bağlı fiş yoktur (ya hepsi ya hiçbiri).
                j["birlikte_atlanan"] = json!([]);
                y = Yanit::json(200, j);
            }
        }
        y
    }

    fn teslim_al_ic(
        &self,
        d: &Defter,
        no: &str,
        c: &cihaz::Cihaz,
        v: &serde_json::Value,
        fisli: bool,
        gecis: bool,
    ) -> Yanit {
        if !fisli && !self.cihazlar().yetki_var(&c.id, "fissiz-teslim") {
            return Yanit::json(
                403,
                json!({
                    "hata": "yetki_yok",
                    "ileti": "Bu cihazın fişsiz teslim yetkisi yok. Yönetici, mağaza bilgisayarındaki Telefonlar ekranından yetki verebilir.",
                }),
            );
        }
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
        // Karekod onayı: belirteç verildiyse bu fişe/cihaza ait ve onaylanmış olmalı (ad, telefon, cihaz ondan gelir).
        let belirtec = v["onay_belirteci"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let onay = match &belirtec {
            Some(t) => match self.qr_onayi(no, t, c) {
                Ok(b) => Some(b),
                Err(y) => return y,
            },
            None => None,
        };
        // Karekoddaki telefon (müşterinin kendi beyanı, doğrulanmaz) fişteki numaradan farklıysa teslim alan
        // başkası olabilir: ilişki (yakını/diğer) açıkça seçilmeli.
        if onay.is_some()
            && crate::teslim_onay::fis_telefonu_karsilastir(
                d,
                no,
                &onay.as_ref().map(|b| b.tel.clone()).unwrap_or_default(),
            ) == "farkli"
            && !matches!(v["iliski"].as_str(), Some("yakini") | Some("diger"))
        {
            return Yanit::json(
                422,
                json!({
                    "hata": "iliski_gerekli",
                    "ileti": "Karekodda yazılan telefon fişteki numaradan farklı. Teslim alanın müşteriye yakınlığını seçin (yakını ya da diğer).",
                }),
            );
        }
        let iliski = v["iliski"]
            .as_str()
            .unwrap_or(if onay.is_some() { "kendisi" } else { "" });
        let teslim_alan: String = match &onay {
            Some(b) => {
                if taa_fis::iliski_adi(iliski).is_none() {
                    return Yanit::hata(422, "ilişki: kendisi, yakini ya da diger olmalı");
                }
                b.ad.clone()
            }
            None => {
                let a = v["teslim_alan"].as_str().unwrap_or("");
                if let Err(e) = taa_fis::teslim_alan_denetle(a, iliski) {
                    return Yanit::hata(422, &e);
                }
                a.split_whitespace().collect::<Vec<_>>().join(" ")
            }
        };
        // İmza önce çözülür (yanlış biçim 400/413), son 4 hane hakkı boşa harcanmasın. Kaynak sırası: karekod
        // onayının içindeki imza (müşteri kendi telefonunda çizdi: "qr_imza"), yoksa bu cihazın ekranındaki imza
        // alanı ("cihaz_imza"). Karekod imzası varken gönderilen cihaz imzası yok sayılır.
        let qr_imza = onay.as_ref().and_then(|b| b.imza.clone());
        let imza: Option<(Vec<u8>, &'static str)> = match qr_imza {
            Some(b) => Some((b, "qr_imza")),
            None => match v["imza_png_base64"].as_str().filter(|s| !s.is_empty()) {
                None => None,
                Some(s) => match imza_coz_denetle(s) {
                    Ok(b) => Some((b, "cihaz_imza")),
                    // Boş tuval / çözülemeyen PNG: imza yok sayılmaz, açıkça reddedilir (fişli teslimde de).
                    Err((422, e)) => {
                        return Yanit::json(
                            422,
                            json!({
                                "hata": "imza_gerekli",
                                "ileti": format!("İmza okunamadı ya da boş ({e}). Teslim alana yeniden imzalatın."),
                            }),
                        )
                    }
                    Err((kod, e)) => return Yanit::hata(kod, &e),
                },
            },
        };
        if !fisli {
            if imza.is_none() {
                return Yanit::json(
                    422,
                    json!({
                        "hata": "imza_gerekli",
                        "ileti": if onay.is_some() {
                            "Fişsiz teslimde imza zorunlu. Müşterinin imzası karekod sayfasından gelmedi; ekranda imzalatın."
                        } else {
                            "Fişsiz teslimde imza zorunlu. Teslim alana ekranda (dokunmatik ekran, kalem ya da fare) imzalatın."
                        },
                    }),
                );
            }
            if let Some(kalan) = self.son4_kilitli(no) {
                return Yanit::json(
                    429,
                    json!({
                        "hata": "son4_kilitli",
                        "ileti": format!("Çok fazla yanlış deneme; {} dakika sonra yeniden deneyin.", kalan.div_ceil(60)),
                        "kalan_sn": kalan,
                    }),
                );
            }
            let son4 = v["telefon_son4"].as_str().unwrap_or("");
            if son4.len() != 4 || !son4.bytes().all(|b| b.is_ascii_digit()) {
                return Yanit::hata(422, "telefon_son4 tam 4 rakam olmalı");
            }
            let rakamlar: Vec<u8> = f.telefon.bytes().filter(|b| b.is_ascii_digit()).collect();
            if rakamlar.len() < 4 {
                return Yanit::hata(422, "fişte doğrulanabilir telefon yok");
            }
            if !sabit_esit(son4.as_bytes(), &rakamlar[rakamlar.len() - 4..]) {
                let kalan = self.son4_yanlis(no);
                return Yanit::json(
                    422,
                    json!({
                        "hata": "son4_yanlis",
                        "ileti": if kalan == 0 {
                            "Telefonun son 4 hanesi yanlış. Bu fiş 15 dakika kilitlendi.".to_string()
                        } else {
                            format!("Telefonun son 4 hanesi yanlış. Kalan deneme: {kalan}.")
                        },
                        "kalan_hak": kalan,
                        "kilitli": kalan == 0,
                    }),
                );
            }
            self.son4_sifirla(no);
        }
        // Bağlı fiş sorusu: durum ucuyla aynı (409 bagli_teslim). "bagli" dolu istekler buraya gelmez, teslim sepetine
        // çevrilir (bagli_sepetle_teslim); "bagli": [] ise bağlılar şimdi teslim edilmez.
        if let Err(y) = self.bagli_teslim_secimi(d, no, v) {
            return y;
        }
        // İmza dosyası: yerel anahtarla şifreli, atomik. Kayıt yazılamazsa (yeni yazılmış, henüz hiçbir kayda
        // bağlanmamış) şifreli dosya SİLİNMEZ, `imzalar/artik/` altına taşınır (kalıcı silme kuralı).
        let mut imza_sha256 = String::new();
        let mut imza_dosyasi = String::new();
        let mut imza_yontemi = String::new();
        let mut yazilan: Option<PathBuf> = None;
        if let Some((b, yontem)) = &imza {
            match self.imza_yaz(no, b) {
                Ok((goreli, yol)) => {
                    imza_sha256 = sha256_bayt(b);
                    imza_dosyasi = goreli;
                    imza_yontemi = (*yontem).to_string();
                    yazilan = Some(yol);
                }
                Err(y) => return y,
            }
        }
        let s = self.simdi();
        // Kişisel bilgi yalnız şifreli alana gider: ad, telefon, okutan cihaz (user-agent), IP, zaman, yöntem.
        let duz = match &onay {
            Some(b) => teslim_sifre::TeslimDuz {
                ad: teslim_alan.clone(),
                tel: b.tel.clone(),
                cihaz: b.ua.clone(),
                ip: b.ip.clone(),
                zaman: b.zaman,
                yontem: "qr".into(),
            },
            None => teslim_sifre::TeslimDuz {
                ad: teslim_alan.clone(),
                zaman: s,
                yontem: if imza.is_some() { "ekran-imza" } else { "elle" }.into(),
                ..Default::default()
            },
        };
        let sifreli = match teslim_sifre::sifrele(&self.dizin, no, &duz) {
            Ok(x) => x,
            Err(e) => {
                if let Some(y) = &yazilan {
                    self.imza_artigi_kenara(y);
                }
                return Yanit::hata(500, &format!("teslim bilgisi şifrelenemedi: {e}"));
            }
        };
        let teslim_id = match self.yeni_teslim_id(d) {
            Ok(t) => t,
            Err(e) => {
                if let Some(y) = &yazilan {
                    self.imza_artigi_kenara(y);
                }
                return Yanit::hata(500, &e);
            }
        };
        let dogrulama = match (fisli, onay.is_some()) {
            (true, false) => "fis",
            (true, true) => "qr",
            (false, false) => "telefon-son4",
            (false, true) => "qr+telefon-son4",
        };
        let kayit = taa_fis::TeslimKaydi {
            teslim_alan: String::new(),
            iliski: iliski.into(),
            fisli,
            dogrulama: dogrulama.into(),
            imza_sha256,
            imza_dosyasi,
            imza_yontemi,
            kim: c.kayit_kimi(),
            tarih: tarih_iso(s),
            saat: saat_iso(s),
            ana_fis: String::new(),
            sifreli: Some(sifreli),
            teslim_id,
            gorunen: None,
        };
        // Eski fiş geçişiyle fişli: günlük teslim kaydından HEMEN ÖNCE (tüm denetimler geçti) yazılır; yazılamazsa teslim
        // yapılmaz. Kayıt sonra başarısız olursa aynı teslim_id ile hata satırı düşülür.
        if gecis {
            if let Err(e) = self.sepet_gunlugu_yaz(json!({
                "olay": "eski_fis_gecisi", "asama": "teslim-al", "teslim_id": kayit.teslim_id,
                "fisler": [no], "kim": Self::denetim_kimi(c),
            })) {
                if let Some(y) = &yazilan {
                    self.imza_artigi_kenara(y);
                }
                return Yanit::json(
                    500,
                    json!({ "hata": "gunluk_yazilamadi", "ileti": format!("Eski fiş geçişi günlüğe yazılamadı; teslim edilmedi ({e}). Yeniden deneyin.") }),
                );
            }
        }
        // Teslim kaydı ve Teslim durumu tek defter satırıdır: yarım kalmaz.
        if let Err(e) = d.teslim_et(no, kayit.clone()) {
            if let Some(y) = &yazilan {
                self.imza_artigi_kenara(y);
            }
            if gecis {
                let _ = self.sepet_gunlugu_yaz(json!({
                    "olay": "eski_fis_gecisi_hata", "asama": "teslim-al", "teslim_id": kayit.teslim_id,
                    "fisler": [no], "kim": Self::denetim_kimi(c), "hata": e,
                }));
            }
            return Yanit::hata(422, &e);
        }
        if let Some(t) = &belirtec {
            self.qr_onayi_tuket(t);
        }
        if let Ok(Some(f)) = d.bul(no) {
            // Bulut kaydı yalnız sabit anahtarlar taşır; teslim alan bilgisi gitmez.
            self.takibe_yaz(&f);
        }
        let t = d.bul(no).ok().flatten().and_then(|f| f.teslim_kaydi);
        Yanit::json(
            200,
            json!({
                "tamam": true,
                "teslim_id": kayit.teslim_id,
                // Eski yanıtla uyum: bağlı fişlerle birlikte teslim artık teslim sepetinden yapılır (ya hepsi ya hiçbiri).
                "birlikte_teslim": [],
                "birlikte_atlanan": [],
                // Teslimden SONRA bilgi maskelidir (ad "M*** Y***", telefon son 4); açık hâl yalnız teslim-acik
                // yetkisiyle `GET /api/fis/<no>/teslim-acik`.
                "teslim_kaydi": t.map(|t| json!({
                    "ad": taa_fis::ad_maskele(&duz.ad),
                    "maskeli": true,
                    "imza_yontemi": t.imza_yontemi,
                    "tel_maskeli": teslim_sifre::tel_maskele(&duz.tel),
                    "cihaz": teslim_sifre::cihaz_ozeti(&duz.cihaz),
                    "yontem": duz.yontem,
                    "iliski": t.iliski,
                    "fisli": t.fisli,
                    "dogrulama": t.dogrulama,
                    "imzali": !t.imza_sha256.is_empty(),
                    "sifreli": t.sifreli.is_some(),
                    "tarih": t.tarih,
                    "saat": t.saat,
                })),
            }),
        )
    }

    /// Şifreli teslim bilgisini çözer (fiş no'ya bağlı). Eski kayıtta (şifresiz) `None`.
    pub(crate) fn teslim_coz(
        &self,
        no: &str,
        t: &taa_fis::TeslimKaydi,
    ) -> Option<teslim_sifre::TeslimDuz> {
        let s = t.sifreli.as_ref()?;
        match teslim_sifre::coz(&self.dizin, no, s) {
            Ok(d) => Some(d),
            Err(e) => {
                eprintln!("Teslim kaydı çözülemedi ({no}): {e}");
                None
            }
        }
    }

    /// Basım/düz metin için: şifreli kayıttan adı çözüp yalnız bellekteki görünüme koyar (basım adı MASKELER).
    pub(crate) fn teslim_gorunen_doldur(&self, f: &mut taa_fis::Fis) {
        let no = f.no.clone();
        if let Some(t) = f.teslim_kaydi.as_mut() {
            if let Some(d) = self.teslim_coz(&no, t) {
                t.gorunen = Some(taa_fis::TeslimGorunen {
                    ad: d.ad,
                    yontem: d.yontem,
                });
            }
        }
    }

    /// Açık görüntüleme denetim günlüğü: kişisel veri İÇERMEZ (yalnız olay, fiş no / sonuç sayısı, kim, zaman).
    /// Günlük yazılamazsa (disk dolu, dosya kilitli) `Err(500 yanıtı)` döner ve çağıran açık veriyi VERMEZ:
    /// "her görüntüleme günlüğe yazılsın" kuralı kayıtsız görüntülemeye izin vermez.
    pub(crate) fn teslim_gunlugu(&self, satir: serde_json::Value) -> Result<(), Yanit> {
        let s = self.simdi();
        let mut v = satir;
        v["tarih"] = json!(tarih_iso(s));
        v["saat"] = json!(saat_iso(s));
        if let Err(e) =
            taa_dosya::satir_ekle(&self.dizin.join("teslim-ayrinti.jsonl"), &v.to_string())
        {
            eprintln!("Teslim denetim günlüğü yazılamadı, açık veri verilmedi: {e}");
            return Err(Yanit::json(
                500,
                json!({ "hata": "gunluk_yazilamadi", "ileti": "Denetim günlüğü yazılamadı; açık bilgi gösterilmedi." }),
            ));
        }
        eprintln!("Teslim kaydı açık görüntüleme: {v}");
        Ok(())
    }

    /// GET /api/yonetim/teslim/<no>: şifreli teslim kaydının TAM görünümü (tam telefon, IP, ham user-agent).
    /// Yalnız bu bilgisayardan ve (kilit açıksa) yönetici oturumuyla; her görüntüleme `teslim-ayrinti.jsonl`
    /// günlüğüne kişisel veri içermeyen bir satırla yazılır. Anlaşmazlıkta kanıt için.
    pub(crate) fn teslim_ayrintisi(&self, i: &Istek, no: &str) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        if !no_guvenli(no) {
            return Yanit::hata(404, "fiş yok");
        }
        let yol = self.fis_yolu();
        let d = Defter { yol: &yol };
        let f = match d.bul(no) {
            Ok(Some(f)) => f,
            Ok(None) => return Yanit::hata(404, "fiş yok"),
            Err(e) => return Yanit::hata(500, &e),
        };
        let Some(t) = f.teslim_kaydi.as_ref() else {
            return Yanit::hata(404, "bu fişin teslim kaydı yok");
        };
        let Some(s) = t.sifreli.as_ref() else {
            if let Err(y) = self.teslim_gunlugu(
                json!({ "olay": "teslim_ayrintisi", "fis": no, "kim": self.yonetici_kimi(i), "yetki": "yonetici-oturumu" }),
            ) {
            return y;
        }
            return Yanit::json(
                200,
                json!({ "sifreli": false, "teslim_alan": t.teslim_alan, "iliski": t.iliski, "tarih": t.tarih, "saat": t.saat }),
            );
        };
        let duz = match teslim_sifre::coz(&self.dizin, no, s) {
            Ok(x) => x,
            Err(e) => return Yanit::hata(500, &e),
        };
        if let Err(y) = self.teslim_gunlugu(
            json!({ "olay": "teslim_ayrintisi", "fis": no, "kim": self.yonetici_kimi(i), "yetki": "yonetici-oturumu" }),
        ) {
            return y;
        }
        let fis_tel = teslim_sifre::tel_normal(&f.telefon).unwrap_or_default();
        Yanit::json(
            200,
            json!({
                "sifreli": true,
                "ad": duz.ad,
                "tel": duz.tel,
                "fis_telefonu": if duz.tel.is_empty() || fis_tel.is_empty() { "yok" } else if duz.tel == fis_tel { "ayni" } else { "farkli" },
                "ip": duz.ip,
                "cihaz": duz.cihaz,
                "cihaz_ozeti": teslim_sifre::cihaz_ozeti(&duz.cihaz),
                "zaman": duz.zaman,
                "yontem": duz.yontem,
                "imza_yontemi": t.imza_yontemi,
                "iliski": t.iliski,
                "fisli": t.fisli,
                "dogrulama": t.dogrulama,
                "kim": t.kim,
                "tarih": t.tarih,
                "saat": t.saat,
                "ana_fis": t.ana_fis,
            }),
        )
    }

    /// POST /api/yonetim/teslim-ara {ad?, tel?}: kör indeksle teslim kaydı arar (değerler URL'de gitmez).
    /// Yanıtta yalnız fiş no, tarih, saat, yöntem; ad/telefon dönmez. Yetki ve günlük `teslim_ayrintisi` gibi.
    pub(crate) fn teslim_ara(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let ad = v["ad"].as_str().unwrap_or("").trim();
        let tel = v["tel"].as_str().unwrap_or("").trim();
        if ad.is_empty() && tel.is_empty() {
            return Yanit::hata(400, "ad ya da telefon yazın");
        }
        if !tel.is_empty() && teslim_sifre::tel_normal(tel).is_none_or(|t| t.is_empty()) {
            return Yanit::hata(422, "telefon geçersiz");
        }
        let (ai, ti) = match teslim_sifre::indeksler(&self.dizin, ad, tel) {
            Ok(x) => x,
            Err(e) => return Yanit::hata(500, &e),
        };
        let yol = self.fis_yolu();
        let d = Defter { yol: &yol };
        let fisler = match d.fisler() {
            Ok(f) => f,
            Err(e) => return Yanit::hata(500, &e),
        };
        let bulunan: Vec<serde_json::Value> = fisler
            .iter()
            .filter_map(|f| {
                let t = f.teslim_kaydi.as_ref()?;
                let s = t.sifreli.as_ref()?;
                let ad_tutar = ad.is_empty() || s.ad_indeks == ai;
                let tel_tutar = tel.is_empty() || (!s.tel_indeks.is_empty() && s.tel_indeks == ti);
                (ad_tutar && tel_tutar).then(|| {
                    json!({ "no": f.no, "tarih": t.tarih, "saat": t.saat, "dogrulama": t.dogrulama, "ana_fis": t.ana_fis })
                })
            })
            .collect();
        if let Err(y) = self.teslim_gunlugu(json!({ "olay": "teslim_ara", "ad_ile": !ad.is_empty(), "tel_ile": !tel.is_empty(), "sonuc": bulunan.len() })) {
            return y;
        }
        Yanit::json(200, json!({ "sonuc": bulunan }))
    }

    /// İmzayı `imzalar/<no>-<n>.png.enc` olarak şifreli ve atomik yazar: (kayıttaki göreli yol, tam yol).
    /// n, ne şifreli ne eski şifresiz dosyası olan ilk sıradır.
    pub(crate) fn imza_yaz(&self, no: &str, png: &[u8]) -> Result<(String, PathBuf), Yanit> {
        if !no_guvenli(no) {
            return Err(Yanit::hata(400, "fiş no geçersiz"));
        }
        let klasor = self.dizin.join("imzalar");
        let Some(n) = (1..=EN_COK_IMZA).find(|n| {
            !klasor.join(format!("{no}-{n}.png.enc")).exists()
                && !klasor.join(format!("{no}-{n}.png")).exists()
        }) else {
            return Err(Yanit::hata(409, "bu fişe çok fazla imza dosyası yazılmış"));
        };
        let sifreli = teslim_sifre::bayt_sifrele(&self.dizin, no.as_bytes(), png)
            .map_err(|e| Yanit::hata(500, &format!("imza şifrelenemedi: {e}")))?;
        let ad = format!("{no}-{n}.png.enc");
        let yol = klasor.join(&ad);
        std::fs::create_dir_all(&klasor)
            .and_then(|_| taa_dosya::uzerine_yaz_atomik(&yol, &sifreli))
            .map_err(|e| Yanit::hata(500, &format!("imza kaydedilemedi: {e}")))?;
        Ok((format!("imzalar/{ad}"), yol))
    }

    /// Hiçbir kayda bağlanamamış (teslim kaydı yazılamadı) yeni şifreli imza dosyasını `imzalar/artik/` altına
    /// taşır: kalıcı silinmez, sonraki imza aynı sırayı kullanabilir. Taşınamazsa yerinde kalır (yalnız uyarı);
    /// şifreli olduğu için düz metin içermez. Yönetici dilerse elle siler.
    pub(crate) fn imza_artigi_kenara(&self, yol: &Path) {
        let Some(ad) = yol.file_name().and_then(|a| a.to_str()) else {
            return;
        };
        let klasor = self.dizin.join("imzalar").join("artik");
        let hedef = klasor.join(format!("{}-{ad}", self.simdi()));
        if let Err(e) = std::fs::create_dir_all(&klasor).and_then(|_| std::fs::rename(yol, &hedef))
        {
            eprintln!("Bağlanmamış imza dosyası kenara alınamadı ({ad}): {e}");
        }
    }

    /// Kayıttaki imzayı okur, çözer ve özetini doğrular. Kayıt ".png" (geçiş öncesi) gösteriyorsa önce şifreli
    /// kopyası (".png.enc"), yoksa eski şifresiz dosya okunur. Hata: (HTTP kodu, ileti).
    pub(crate) fn imza_oku(
        &self,
        no: &str,
        t: &taa_fis::TeslimKaydi,
    ) -> Result<Vec<u8>, (u16, &'static str)> {
        // Bağlı fişte imza ana fişindir (dosya adı ve aad ana fiş no).
        let sahip = if t.ana_fis.is_empty() {
            no
        } else {
            t.ana_fis.as_str()
        };
        let ad = t
            .imza_dosyasi
            .strip_prefix("imzalar/")
            .filter(|a| imza_dosya_adi_coz(a).is_some_and(|(n, _)| n == sahip))
            .ok_or((404, "imza yok"))?;
        let klasor = self.dizin.join("imzalar");
        let enc = if ad.ends_with(".enc") {
            ad.to_string()
        } else {
            format!("{ad}.enc")
        };
        let png = match std::fs::read(klasor.join(&enc)) {
            Ok(b) => teslim_sifre::bayt_coz(&self.dizin, sahip.as_bytes(), &b)
                .map_err(|_| (409, "imza dosyası çözülemedi (anahtar ya da fiş uyuşmuyor)"))?,
            Err(_) if !ad.ends_with(".enc") => {
                std::fs::read(klasor.join(ad)).map_err(|_| (404, "imza yok"))?
            }
            Err(_) => return Err((404, "imza yok")),
        };
        if !sabit_esit(sha256_bayt(&png).as_bytes(), t.imza_sha256.as_bytes()) {
            return Err((409, "imza dosyası kayıttaki özetle uyuşmuyor"));
        }
        Ok(png)
    }

    /// GET /api/fis/<no>/imza: teslim imzasının PNG'si (sunucuda çözülür, özeti kayıtla eşleşmeli).
    /// Yalnız "teslim-acik" yetkisiyle (imza da teslim bilgisinin açık hâlidir); her görüntüleme denetim
    /// günlüğüne yazılır. Mağaza bilgisayarında oturum yoksa 401, diğer cihazda 403 yetki_yok.
    pub(crate) fn teslim_imzasi(&self, d: &Defter, no: &str, c: &cihaz::Cihaz, i: &Istek) -> Yanit {
        let Some(kaynak) = self.teslim_acik_yetkisi(i, c) else {
            if c.id == PC_ID {
                return Yanit::json(
                    401,
                    json!({ "hata": "yonetici_gerekli", "ileti": "İmzayı görmek için yönetici girişi gerekli." }),
                );
            }
            return Yanit::json(
                403,
                json!({
                    "hata": "yetki_yok",
                    "ileti": "Bu cihaz teslim imzalarını göremez. Yönetici, Telefonlar ekranından yetki verebilir.",
                }),
            );
        };
        if !no_guvenli(no) {
            return Yanit::hata(404, "imza yok");
        }
        let t = match d.bul(no) {
            Ok(Some(f)) => f.teslim_kaydi,
            Ok(None) => return Yanit::hata(404, "fiş yok"),
            Err(e) => return Yanit::hata(500, &e),
        };
        let Some(t) = t.filter(|t| !t.imza_dosyasi.is_empty()) else {
            return Yanit::hata(404, "imza yok");
        };
        match self.imza_oku(no, &t) {
            Ok(b) => {
                if let Err(y) = self.teslim_gunlugu(json!({
                    "olay": "imza_goruntuleme",
                    "fis": no,
                    "kim": Self::denetim_kimi(c),
                    "yetki": kaynak.ad(),
                })) {
                    return y;
                }
                Yanit {
                    kod: 200,
                    tur: "image/png",
                    govde: b,
                }
            }
            Err((kod, e)) => Yanit::hata(kod, e),
        }
    }
}

#[cfg(test)]
pub(crate) mod testler {
    use super::*;

    /// Deneme kişileri (gerçek kişi değildir).
    const KISILER: &str = concat!(
        r#"{"tur":"ekle","v":{"id":"K-test0001","ad":"Deneme Danışman","rol":"danisman","tarih":"2026-11-01"}}"#,
        "\n",
        r#"{"tur":"ekle","v":{"id":"K-test0002","ad":"Deneme Usta","rol":"usta","tarih":"2026-11-01"}}"#,
        "\n"
    );
    /// Sahte fiş: telefonun son 4 hanesi 4567.
    pub(crate) const FIS: &str = r#"{"satis_danismani_id":"K-test0001","olcu_alan_id":"K-test0002","duzenleyen_id":"K-test0001","musteri":"Deneme Müşteri","telefon":"0532 000 45 67","kalemler":[{"urun":"Pantolon","islem":"paça kısalt"}],"teslim":"2099-01-01","ucret_kurus":50000,"on_odeme_kurus":10000}"#;

    pub(crate) fn uyg() -> Uygulama {
        let d = taa_dosya::gecici_klasor().unwrap();
        std::fs::write(d.join("sube.json"), r#"{"kod":"MRK01"}"#).unwrap();
        std::fs::write(d.join("kisiler.jsonl"), KISILER).unwrap();
        let mut u = Uygulama::yeni(d, None, "http://192.168.1.20:8787".into());
        u.yonetici_kilidi = false;
        u
    }

    pub(crate) fn pc(metot: &str, yol: &str, govde: &str) -> Istek {
        Istek {
            metot: metot.into(),
            yol: yol.into(),
            govde: govde.as_bytes().to_vec(),
            yerel: true,
            masaustu: true,
            ..Default::default()
        }
    }

    pub(crate) fn tel(token: &str, metot: &str, yol: &str, govde: &str) -> Istek {
        Istek {
            metot: metot.into(),
            yol: yol.into(),
            yetki: Some(format!("Bearer {token}")),
            govde: govde.as_bytes().to_vec(),
            ..Default::default()
        }
    }

    pub(crate) fn json_of(y: &Yanit) -> serde_json::Value {
        serde_json::from_slice(&y.govde).unwrap_or_default()
    }

    /// Hazır durumda bir sahte fiş açar.
    pub(crate) fn hazir_fis(u: &Uygulama) -> String {
        let y = u.isle(&pc("POST", "/api/fis", FIS));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let no = json_of(&y)["no"].as_str().unwrap().to_string();
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no}/durum"),
            r#"{"durum":"hazir"}"#,
        ));
        assert_eq!(y.kod, 200);
        no
    }

    /// Yalnız başlıklı sahte PNG (8 bayt imza + IHDR + çöp): başlık denetimini geçer, içerik denetimini GEÇMEZ.
    fn sahte_png(en: u32, boy: u32) -> Vec<u8> {
        let mut b = PNG_IMZASI.to_vec();
        b.extend_from_slice(&13u32.to_be_bytes());
        b.extend_from_slice(b"IHDR");
        b.extend_from_slice(&en.to_be_bytes());
        b.extend_from_slice(&boy.to_be_bytes());
        b.extend_from_slice(&[8, 6, 0, 0, 0, 0, 0, 0, 0]); // derinlik, renk, ..., CRC (sahte)
        b.extend_from_slice(b"DENEME-IMZA");
        b
    }

    /// Gerçek küçük imza PNG'si (gri, çizgili; gerçek zlib + CRC).
    fn imza_png(en: u32, boy: u32) -> Vec<u8> {
        crate::imza_png::deneme_png(en, boy, 0, true)
    }

    /// Sahte imza (base64).
    pub(crate) fn imza_b64() -> String {
        base64_kodla(&imza_png(400, 150))
    }

    pub(crate) fn base64_kodla(b: &[u8]) -> String {
        const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut s = String::new();
        for p in b.chunks(3) {
            let n = (p[0] as u32) << 16
                | (*p.get(1).unwrap_or(&0) as u32) << 8
                | *p.get(2).unwrap_or(&0) as u32;
            for i in 0..4 {
                if i <= p.len() {
                    s.push(A[(n >> (18 - 6 * i) & 63) as usize] as char);
                } else {
                    s.push('=');
                }
            }
        }
        s
    }

    /// Fişin karekod kanıtı (okutulan metin): müşteri takip ayarlıysa müşteri nüshasındaki takip karekodu, değilse
    /// personel karekodu.
    pub(crate) fn fis_kk(u: &Uygulama, no: &str) -> String {
        match musteri_takip::Ayar::dizinden(&u.dizin) {
            Some(a) => a.adres(&musteri_takip::takip_kodu(&u.dizin, no).unwrap()),
            None => u.fis_karekod_adresi(no),
        }
    }

    fn fissiz(son4: &str, imza: Option<&str>) -> String {
        let mut v = json!({
            "fisli": false,
            "teslim_alan": "Deneme Yakın Kişi",
            "iliski": "yakini",
            "telefon_son4": son4,
        });
        if let Some(i) = imza {
            v["imza_png_base64"] = json!(i);
        }
        v.to_string()
    }

    #[test]
    fn base64_cozucu() {
        for n in 0..10 {
            let b: Vec<u8> = (0..n).map(|i| (i * 37 + 5) as u8).collect();
            assert_eq!(base64_coz(&base64_kodla(&b)).unwrap(), b);
        }
        assert_eq!(base64_coz("data:image/png;base64,QUJD").unwrap(), b"ABC");
        assert!(base64_coz("QU*D").is_err());
        assert!(base64_coz("Q").is_err());
    }

    #[test]
    fn fissiz_teslim_yetki_ve_dogrulama() {
        let u = uyg();
        let no = hazir_fis(&u);
        let yol = format!("/api/fis/{no}/teslim-al");
        let imza = imza_b64();
        // eşleşmiş telefon: fişsiz teslim yetkisi varsayılan kapalı
        let token = "deneme-belirtec-0001";
        u.cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        let y = u.isle(&tel(token, "POST", &yol, &fissiz("4567", Some(&imza))));
        assert_eq!(y.kod, 403);
        assert_eq!(json_of(&y)["hata"], "yetki_yok");
        // PNG olmayan imza 400, imzasız fişsiz 422
        let png_degil = base64_kodla(b"GIF89a-deneme");
        assert_eq!(
            u.isle(&pc("POST", &yol, &fissiz("4567", Some(&png_degil))))
                .kod,
            400
        );
        assert_eq!(
            u.isle(&pc("POST", &yol, &fissiz("4567", Some("%%%")))).kod,
            400
        );
        let y = u.isle(&pc("POST", &yol, &fissiz("4567", None)));
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "imza_gerekli");
        // biçim hatası (4 rakam değil) 422, hak yemez
        assert_eq!(
            u.isle(&pc("POST", &yol, &fissiz("45a7", Some(&imza)))).kod,
            422
        );
        // tek kelimelik ad 422
        let tek = fissiz("4567", Some(&imza)).replace("Deneme Yakın Kişi", "Deneme");
        assert_eq!(u.isle(&pc("POST", &yol, &tek)).kod, 422);
        // yanlış son 4: 422 ve kalan hak
        let y = u.isle(&pc("POST", &yol, &fissiz("1111", Some(&imza))));
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["kalan_hak"], 4);
        // doğru son 4 + imza: 200; kayıt yazılır, durum teslim
        let y = u.isle(&pc("POST", &yol, &fissiz("4567", Some(&imza))));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j = json_of(&y);
        assert_eq!(j["teslim_kaydi"]["dogrulama"], "telefon-son4");
        assert_eq!(j["teslim_kaydi"]["imzali"], true);
        let d = Defter { yol: &u.fis_yolu() };
        let f = d.bul(&no).unwrap().unwrap();
        assert_eq!(f.durum, Durum::Teslim);
        let t = f.teslim_kaydi.clone().unwrap();
        assert_eq!(t.imza_dosyasi, format!("imzalar/{no}-1.png.enc"));
        assert_eq!(t.imza_yontemi, "cihaz_imza");
        // imza diskte şifreli (yerel anahtar, aad = fiş no); şifresiz PNG yazılmaz
        let enc = std::fs::read(u.dizin.join("imzalar").join(format!("{no}-1.png.enc"))).unwrap();
        assert!(enc.starts_with(teslim_sifre::DOSYA_BASLIGI));
        assert!(!enc.windows(11).any(|w| w == b"DENEME-IMZA"));
        assert!(!u.dizin.join("imzalar").join(format!("{no}-1.png")).exists());
        // imza geri okunur (sunucuda çözülür), özet tutar
        let g = u.isle(&pc("GET", &format!("/api/fis/{no}/imza"), ""));
        assert_eq!(g.kod, 200);
        assert_eq!(g.tur, "image/png");
        assert_eq!(sha256_bayt(&g.govde), t.imza_sha256);
        // imza yetkisiz telefona kapalı (403), belirteçsiz okunmaz
        let y = u.isle(&tel(token, "GET", &format!("/api/fis/{no}/imza"), ""));
        assert_eq!(y.kod, 403);
        assert_eq!(json_of(&y)["hata"], "yetki_yok");
        assert_eq!(
            u.isle(&tel("yanlis", "GET", &format!("/api/fis/{no}/imza"), ""))
                .kod,
            401
        );
        // ikinci kez teslim alınamaz
        assert_eq!(
            u.isle(&pc("POST", &yol, &fissiz("4567", Some(&imza)))).kod,
            409
        );
        // düz metinde ad MASKELİ ("D*** Y*** K***"), ad hiç basılmaz
        let m = String::from_utf8(
            u.isle(&pc("GET", &format!("/api/fis/{no}/metin"), ""))
                .govde,
        )
        .unwrap()
        .replace('\n', " ");
        assert!(
            m.contains("Teslim alan: D*** Y*** K*** (yakını) - fişsiz, imzalı"),
            "{m}"
        );
        assert!(!m.contains("Yakın Kişi") && !m.contains("Deneme Y"));
        // son 4 hane hiçbir dosyaya yazılmaz (telefonun kendisi fiş satırında durur; teslim kaydında yok)
        let defter = std::fs::read_to_string(u.fis_yolu()).unwrap();
        let satir = defter
            .lines()
            .find(|l| l.contains("teslimkaydi"))
            .expect("teslim kaydı satırı");
        assert!(!satir.contains("4567") && !satir.contains("telefon_son4"));
        fn tara(k: &Path, bulunan: &mut Vec<String>) {
            for e in std::fs::read_dir(k).unwrap().flatten() {
                let p = e.path();
                if p.is_dir() {
                    tara(&p, bulunan);
                } else if std::fs::read(&p)
                    .unwrap()
                    .windows(12)
                    .any(|w| w == b"telefon_son4")
                {
                    bulunan.push(p.display().to_string());
                }
            }
        }
        let mut bulunan = Vec::new();
        tara(&u.dizin, &mut bulunan);
        assert!(bulunan.is_empty(), "son 4 alanı dosyada: {bulunan:?}");
        // bulut kaydına teslim alan bilgisi gitmez (yalnız sabit anahtarlar)
        let bulut = musteri_takip::kayit(&u.dizin, &f, 0).unwrap().to_string();
        assert!(
            !bulut.contains("Yakın") && !bulut.contains("teslim_alan") && !bulut.contains("imza")
        );
    }

    #[test]
    fn png_ihdr_denetimi() {
        assert!(png_gecerli(&sahte_png(1, 1)));
        assert!(!png_gecerli(&PNG_IMZASI)); // IHDR yok
        assert!(!png_gecerli(&sahte_png(0, 10)));
        assert!(!png_gecerli(&sahte_png(10, EN_BUYUK_IMZA_KENARI + 1)));
        let mut b = sahte_png(10, 10);
        b[12..16].copy_from_slice(b"IDAT");
        assert!(!png_gecerli(&b));
        // yalnız 4 sihirli bayt (eski denetim) artık yetmez
        let mut eski = vec![0x89, b'P', b'N', b'G'];
        eski.extend_from_slice(&[0u8; 40]);
        assert!(!png_gecerli(&eski));
        let u = uyg();
        let no = hazir_fis(&u);
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no}/teslim-al"),
            &fissiz("4567", Some(&base64_kodla(&eski))),
        ));
        assert_eq!(y.kod, 400);
        // Başlığı doğru ama içi sahte / boş beyaz tuval / tek nokta: 422 imza_gerekli, teslim olmaz
        // (denetçi bulgusu: içerik denetimi yalnız tarayıcıdaydı).
        let nokta = crate::imza_png::gri_png(600, 240, |x, y| {
            if (295..300).contains(&x) && (100..105).contains(&y) {
                0
            } else {
                255
            }
        });
        for (ad, png) in [
            ("sahte", sahte_png(400, 150)),
            ("bos-gri", crate::imza_png::deneme_png(600, 240, 0, false)),
            ("bos-rgba", crate::imza_png::deneme_png(600, 240, 6, false)),
            ("nokta", nokta),
        ] {
            let y = u.isle(&pc(
                "POST",
                &format!("/api/fis/{no}/teslim-al"),
                &fissiz("4567", Some(&base64_kodla(&png))),
            ));
            assert_eq!(y.kod, 422, "{ad}: {}", String::from_utf8_lossy(&y.govde));
            assert_eq!(json_of(&y)["hata"], "imza_gerekli", "{ad}");
        }
        // fişli teslimde de boş imza gönderilirse reddedilir (yok sayılmaz)
        let mut v: serde_json::Value = serde_json::from_str(&fissiz("4567", None)).unwrap();
        v["fisli"] = json!(true);
        v["imza_png_base64"] = json!(base64_kodla(&crate::imza_png::deneme_png(
            600, 240, 0, false
        )));
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no}/teslim-al"),
            &v.to_string(),
        ));
        assert_eq!(json_of(&y)["hata"], "imza_gerekli");
        let f = Defter { yol: &u.fis_yolu() }.bul(&no).unwrap().unwrap();
        assert!(f.teslim_kaydi.is_none(), "hiçbiri teslim etmemeli");
        // gerçek imza geçer (RGBA yedeği de)
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no}/teslim-al"),
            &fissiz(
                "4567",
                Some(&base64_kodla(&crate::imza_png::deneme_png(
                    600, 240, 6, true,
                ))),
            ),
        ));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
    }

    #[test]
    fn durum_ucu_teslim_etmez() {
        let u = uyg();
        let no = hazir_fis(&u);
        let token = "deneme-belirtec-0003";
        u.cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        // yetkisiz telefon /durum ile teslim edemez; PC de edemez: teslim-al gerekir
        for i in [
            tel(
                token,
                "POST",
                &format!("/api/fis/{no}/durum"),
                r#"{"durum":"teslim"}"#,
            ),
            pc(
                "POST",
                &format!("/api/fis/{no}/durum"),
                r#"{"durum":"teslim"}"#,
            ),
        ] {
            let y = u.isle(&i);
            assert_eq!(y.kod, 409);
            assert_eq!(json_of(&y)["hata"], "teslim_al_gerekli");
        }
        let d = Defter { yol: &u.fis_yolu() };
        let f = d.bul(&no).unwrap().unwrap();
        assert_eq!(f.durum, Durum::Hazir);
        assert!(f.teslim_kaydi.is_none());
    }

    #[test]
    fn yetkili_telefon_fissiz_teslim_ve_imza() {
        let u = uyg();
        let no = hazir_fis(&u);
        let token = "deneme-belirtec-0004";
        let c = u
            .cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        let yol = format!("/api/fis/{no}/teslim-al");
        let imza = imza_b64();
        // fişsiz teslim yetkisi verilince telefondan fişsiz teslim olur; imzayı GÖRMEZ (teslim-acik ayrı yetki)
        u.cihazlar()
            .yetki_ata(&c.id, "fissiz-teslim", true, "2026-11-26")
            .unwrap();
        let y = u.isle(&tel(token, "POST", &yol, &fissiz("4567", Some(&imza))));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        // teslimden sonra yanıt da maskeli
        assert_eq!(json_of(&y)["teslim_kaydi"]["ad"], "D*** Y*** K***");
        let imza_yolu = format!("/api/fis/{no}/imza");
        let acik_yolu = format!("/api/fis/{no}/teslim-acik");
        assert_eq!(u.isle(&tel(token, "GET", &imza_yolu, "")).kod, 403);
        assert_eq!(u.isle(&tel(token, "GET", &acik_yolu, "")).kod, 403);
        // maskeli özet görünür (ad maskeli, açık düğmesi yok)
        let j = json_of(&u.isle(&tel(token, "GET", &format!("/api/fis/{no}"), "")));
        assert_eq!(j["teslim_kaydi"]["ad"], "D*** Y*** K***");
        assert_eq!(j["teslim_kaydi"]["acik_yetki"], false);
        // yönetici "teslim-acik" verir: imza ve açık görünüm; geri alınca kapanır
        assert_eq!(
            u.isle(&pc(
                "POST",
                &format!("/api/yonetim/cihaz/{}/yetki", c.id),
                r#"{"yetki":"teslim-acik","ver":true}"#
            ))
            .kod,
            200
        );
        assert_eq!(u.isle(&tel(token, "GET", &imza_yolu, "")).kod, 200);
        let y = u.isle(&tel(token, "GET", &acik_yolu, ""));
        assert_eq!(y.kod, 200);
        assert_eq!(json_of(&y)["ad"], "Deneme Yakın Kişi");
        assert_eq!(json_of(&y)["imza_yontemi"], "cihaz_imza");
        let o = json_of(&u.isle(&tel(token, "GET", "/api/oturum", "")));
        assert!(o["yetkiler"]
            .as_array()
            .unwrap()
            .iter()
            .any(|y| y == "teslim-acik"));
        u.cihazlar()
            .yetki_ata(&c.id, "teslim-acik", false, "2026-11-26")
            .unwrap();
        assert_eq!(u.isle(&tel(token, "GET", &imza_yolu, "")).kod, 403);
        // fişsiz yetki geri alınınca yeni fişte fişsiz teslim 403
        u.cihazlar()
            .yetki_ata(&c.id, "fissiz-teslim", false, "2026-11-26")
            .unwrap();
        let no2 = hazir_fis(&u);
        let y = u.isle(&tel(
            token,
            "POST",
            &format!("/api/fis/{no2}/teslim-al"),
            &fissiz("4567", Some(&imza)),
        ));
        assert_eq!(y.kod, 403);
        // denetim günlüğü: kim (cihaz), hangi fiş, hangi yetkiyle; kişisel veri yok
        let g = std::fs::read_to_string(u.dizin.join("teslim-ayrinti.jsonl")).unwrap();
        assert!(
            g.contains("\"olay\":\"teslim_acik\"") && g.contains("\"olay\":\"imza_goruntuleme\"")
        );
        assert!(g.contains(&c.id) && g.contains(&no) && g.contains("cihaz-yetkisi"));
        assert!(!g.contains("Deneme") && !g.contains("4567"), "{g}");
    }

    /// Rol varsayılanı: yönetici ve mağaza müdürünün kişisel telefonu açık görür; personel görmez. Yönetici kişi
    /// bazında geri alabilir ya da verebilir. Bilgisayarda yönetici oturumu gerekir.
    #[test]
    fn teslim_acik_rol_kisi_ve_yonetici_oturumu() {
        let mut u = uyg();
        std::fs::write(
            u.dizin.join("kisiler.jsonl"),
            concat!(
                r#"{"tur":"ekle","v":{"id":"K-test0001","ad":"Deneme Danışman","rol":"danisman","tarih":"2026-11-01"}}"#, "\n",
                r#"{"tur":"ekle","v":{"id":"K-test0002","ad":"Deneme Usta","rol":"usta","tarih":"2026-11-01"}}"#, "\n",
                r#"{"tur":"ekle","v":{"id":"K-test0003","ad":"Deneme Müdür","rol":"magaza-muduru","tarih":"2026-11-01"}}"#, "\n",
            ),
        )
        .unwrap();
        let no = hazir_fis(&u);
        assert_eq!(
            u.isle(&pc(
                "POST",
                &format!("/api/fis/{no}/teslim-al"),
                &fissiz("4567", Some(&imza_b64()))
            ))
            .kod,
            200
        );
        let (tm, tp) = ("deneme-belirtec-m001", "deneme-belirtec-p001");
        let m = u
            .cihazlar()
            .ekle("Müdür tel", tm, "2026-11-26", "K-test0003")
            .unwrap();
        u.cihazlar()
            .ekle("Danışman tel", tp, "2026-11-26", "K-test0001")
            .unwrap();
        let acik = format!("/api/fis/{no}/teslim-acik");
        assert_eq!(
            u.isle(&tel(tm, "GET", &acik, "")).kod,
            200,
            "mağaza müdürü rol varsayılanı"
        );
        assert_eq!(
            u.isle(&tel(tp, "GET", &acik, "")).kod,
            403,
            "danışman varsayılan kapalı"
        );
        // yetki listesi kişileri ve kaynağı gösterir
        let l = json_of(&u.isle(&pc("GET", "/api/yonetim/yetkiler", "")));
        let k3 = l["kisiler"]
            .as_array()
            .unwrap()
            .iter()
            .find(|k| k["id"] == "K-test0003")
            .unwrap()
            .clone();
        assert_eq!(
            (k3["teslim_acik"].clone(), k3["kaynak"].clone()),
            (json!(true), json!("rol"))
        );
        let mc = l["cihazlar"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["id"] == m.id.as_str())
            .unwrap()
            .clone();
        assert_eq!(mc["teslim_acik_kaynagi"], "rol");
        // yönetici müdürden geri alır, danışmana verir (kişi bazında)
        for (id, ver) in [("K-test0003", false), ("K-test0001", true)] {
            let y = u.isle(&pc(
                "POST",
                &format!("/api/yonetim/kisi/{id}/yetki"),
                &json!({"yetki":"teslim-acik","ver":ver}).to_string(),
            ));
            assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        }
        assert_eq!(u.isle(&tel(tm, "GET", &acik, "")).kod, 403);
        assert_eq!(u.isle(&tel(tp, "GET", &acik, "")).kod, 200);
        // kişi kapatılınca kişi kaydı geçersiz ve geri alınmış olur (eşleşmiş telefon açık kalsa da)
        let y = u.isle(&pc("POST", "/api/yonetim/kisi/K-test0001/kapat", ""));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(
            u.isle(&tel(tp, "GET", &acik, "")).kod,
            403,
            "kapatılan kişi"
        );
        assert_eq!(
            u.cihazlar()
                .yetki_kaydi(&cihaz::kisi_anahtari("K-test0001"), "teslim-acik"),
            Some(false)
        );
        // kişi listeden düşmüş ama kayıt kalmışsa da (ör. Çatı listesi esas olunca) karar kapalı
        u.cihazlar()
            .kisi_yetki_ata("K-test0002", "teslim-acik", true, "2026-11-26")
            .unwrap();
        assert!(u.teslim_acik_kalici("", "K-test0002").is_some());
        std::fs::write(
            u.dizin.join("kisiler.jsonl"),
            std::fs::read_to_string(u.dizin.join("kisiler.jsonl")).unwrap()
                + r#"{"tur":"kapat","v":{"id":"K-test0002","tarih":"2026-11-26"}}"#
                + "\n",
        )
        .unwrap();
        assert!(u.teslim_acik_kalici("", "K-test0002").is_none());
        // bilinmeyen kişi / kişiye verilemeyen yetki
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/kisi/K-yok00/yetki",
                r#"{"yetki":"teslim-acik","ver":true}"#
            ))
            .kod,
            404
        );
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/kisi/K-test0003/yetki",
                r#"{"yetki":"duzelt","ver":true}"#
            ))
            .kod,
            422
        );
        // bilgisayar: yönetici kilidi açıkken oturumsuz 401 (PIN istenir), maskeli özet yine görünür
        u.yonetici_kilidi = true;
        assert_eq!(u.isle(&pc("GET", &acik, "")).kod, 401);
        assert_eq!(
            u.isle(&pc("GET", &format!("/api/fis/{no}/imza"), "")).kod,
            401
        );
        let j = json_of(&u.isle(&pc("GET", &format!("/api/fis/{no}"), "")));
        assert_eq!(j["teslim_kaydi"]["ad"], "D*** Y*** K***");
        assert_eq!(j["teslim_kaydi"]["acik_yetki"], true);
        assert!(j["teslim_kaydi"]["ip"].is_null());
        // kişi yetkisini yalnız yönetici değiştirir
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/kisi/K-test0003/yetki",
                r#"{"yetki":"teslim-acik","ver":true}"#
            ))
            .kod,
            401
        );
    }

    #[test]
    fn denetim_gunlugu_yazilamazsa_acik_veri_verilmez() {
        let u = uyg();
        let no = hazir_fis(&u);
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no}/teslim-al"),
            &fissiz("4567", Some(&imza_b64())),
        ));
        assert_eq!(y.kod, 200);
        // günlük dosyası yerine klasör: satır eklenemez (disk dolu / kilitli dosya benzeri)
        let g = u.dizin.join("teslim-ayrinti.jsonl");
        let _ = std::fs::rename(&g, u.dizin.join("eski-gunluk.jsonl"));
        std::fs::create_dir_all(&g).unwrap();
        for yol in [
            format!("/api/fis/{no}/teslim-acik"),
            format!("/api/fis/{no}/imza"),
            format!("/api/yonetim/teslim/{no}"),
        ] {
            let y = u.isle(&pc("GET", &yol, ""));
            assert_eq!(y.kod, 500, "{yol}");
            let govde = String::from_utf8_lossy(&y.govde).to_string();
            assert!(govde.contains("gunluk_yazilamadi"), "{yol}: {govde}");
            assert!(!govde.contains("Deneme Yakın"), "{yol}: açık ad sızdı");
        }
        let y = u.isle(&pc(
            "POST",
            "/api/yonetim/teslim-ara",
            r#"{"ad":"Deneme Yakın Kişi"}"#,
        ));
        assert_eq!(y.kod, 500);
    }

    #[test]
    fn imza_dosyasi_kurcalanirsa() {
        let u = uyg();
        let no = hazir_fis(&u);
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no}/teslim-al"),
            &fissiz("4567", Some(&imza_b64())),
        ));
        assert_eq!(y.kod, 200);
        let imza_yolu = format!("/api/fis/{no}/imza");
        assert_eq!(u.isle(&pc("GET", &imza_yolu, "")).kod, 200);
        let dosya = u.dizin.join("imzalar").join(format!("{no}-1.png.enc"));
        // başka bir (geçerli şifreli) imzayla değiştirilirse özet tutmaz: 409
        let baska =
            teslim_sifre::bayt_sifrele(&u.dizin, no.as_bytes(), &imza_png(401, 150)).unwrap();
        std::fs::write(&dosya, baska).unwrap();
        assert_eq!(u.isle(&pc("GET", &imza_yolu, "")).kod, 409);
        // başka fişin anahtar bağlamıyla (aad) şifrelenmiş imza çözülmez: 409
        let yabanci =
            teslim_sifre::bayt_sifrele(&u.dizin, b"F-20990101-999", &imza_png(400, 150)).unwrap();
        std::fs::write(&dosya, yabanci).unwrap();
        assert_eq!(u.isle(&pc("GET", &imza_yolu, "")).kod, 409);
        // kayıttaki dosya adı kurcalanırsa (yol kaçışı) okunmaz: 404
        let defter = std::fs::read_to_string(u.fis_yolu()).unwrap();
        let eski = format!("imzalar/{no}-1.png.enc");
        assert!(defter.contains(&eski));
        std::fs::write(u.fis_yolu(), defter.replace(&eski, "imzalar/../sube.json")).unwrap();
        assert_eq!(u.isle(&pc("GET", &imza_yolu, "")).kod, 404);
    }

    #[test]
    fn telefonu_kisa_fis_fissiz_teslim_edilemez() {
        let u = uyg();
        // telefon alanında 4'ten az rakam olan sahte fiş doğrudan deftere yazılır (giriş denetimi bunu reddeder)
        let no = hazir_fis(&u);
        let defter = std::fs::read_to_string(u.fis_yolu()).unwrap();
        let bas = defter.find("\"telefon\":\"").unwrap() + 12;
        let son = bas + defter[bas..].find('"').unwrap();
        let yeni = format!("{}12{}", &defter[..bas], &defter[son..]);
        std::fs::write(u.fis_yolu(), yeni).unwrap();
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no}/teslim-al"),
            &fissiz("0012", Some(&imza_b64())),
        ));
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "fişte doğrulanabilir telefon yok");
    }

    #[test]
    fn son4_bes_yanlista_kilit() {
        let u = uyg();
        let no = hazir_fis(&u);
        let yol = format!("/api/fis/{no}/teslim-al");
        let imza = imza_b64();
        for kalan in (0..5).rev() {
            let y = u.isle(&pc("POST", &yol, &fissiz("0000", Some(&imza))));
            assert_eq!(y.kod, 422);
            assert_eq!(json_of(&y)["kalan_hak"], kalan);
        }
        // kilitli: doğru hane de kabul edilmez
        let y = u.isle(&pc("POST", &yol, &fissiz("4567", Some(&imza))));
        assert_eq!(y.kod, 429);
        assert_eq!(json_of(&y)["hata"], "son4_kilitli");
        // 15 dk sonra açılır
        u.ileri_sn
            .fetch_add(SON4_KILIT_SN + 1, std::sync::atomic::Ordering::SeqCst);
        let y = u.isle(&pc("POST", &yol, &fissiz("4567", Some(&imza))));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
    }

    /// Karar (2026-11-26, teslim sepeti): eski `fisli: true` alanı karekod kanıtı (`fis_karekodu`) yoksa fişsiz
    /// kurallarıyla değerlendirilir; elle "fişli" işaretleme yolu yoktur.
    #[test]
    fn fisli_istegi_karekodsuz_fissiz_sayilir() {
        let u = uyg();
        let no = hazir_fis(&u);
        let yol = format!("/api/fis/{no}/teslim-al");
        let token = "deneme-belirtec-0011";
        u.cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        let g = r#"{"fisli":true,"teslim_alan":"Deneme Müşteri","iliski":"kendisi"}"#;
        // yetkisiz telefon: fişsiz yetkisi istenir
        let y = u.isle(&tel(token, "POST", &yol, g));
        assert_eq!(y.kod, 403);
        assert_eq!(json_of(&y)["fis_karekodu_yok"], true);
        // mağaza PC: imza ve son 4 istenir
        let y = u.isle(&pc("POST", &yol, g));
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "imza_gerekli");
        assert_eq!(json_of(&y)["fis_karekodu_yok"], true);
        assert!(json_of(&y)["ileti"]
            .as_str()
            .unwrap()
            .starts_with("Fiş karekodu okutulmadı"));
        // başka fişin karekodu kanıt değildir
        let diger = hazir_fis(&u);
        let g2 = json!({"fisli":true,"fis_karekodu":fis_kk(&u, &diger),"teslim_alan":"Deneme Müşteri","iliski":"kendisi"});
        assert_eq!(u.isle(&pc("POST", &yol, &g2.to_string())).kod, 422);
        // son 4 + imza ile fişsiz olarak geçer, kayıt fişsiz yazılır
        let g3 = json!({"fisli":true,"teslim_alan":"Deneme Müşteri","iliski":"kendisi","telefon_son4":"4567","imza_png_base64":imza_b64()});
        let y = u.isle(&pc("POST", &yol, &g3.to_string()));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(json_of(&y)["teslim_kaydi"]["fisli"], false);
        assert_eq!(json_of(&y)["teslim_kaydi"]["dogrulama"], "telefon-son4");
        assert!(taa_fis::teslim_id_gecerli(
            json_of(&y)["teslim_id"].as_str().unwrap()
        ));
        // takip ayarlıyken personel karekodu (terzihane nüshası) kanıt değildir, takip karekodu kanıttır
        let (u, _) = qr_uyg();
        let no = hazir_fis(&u);
        let yol = format!("/api/fis/{no}/teslim-al");
        let personel = json!({"fisli":true,"fis_karekodu":format!("http://192.168.1.20:8787/telefon#fis={no}"),"teslim_alan":"Deneme Müşteri","iliski":"kendisi"});
        assert_eq!(u.isle(&pc("POST", &yol, &personel.to_string())).kod, 422);
        let takip = json!({"fisli":true,"fis_karekodu":fis_kk(&u, &no),"teslim_alan":"Deneme Müşteri","iliski":"kendisi"});
        let y = u.isle(&pc("POST", &yol, &takip.to_string()));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(json_of(&y)["teslim_kaydi"]["dogrulama"], "fis");
    }

    #[test]
    fn fisli_teslim_yetkisiz_ve_imzasiz() {
        let u = uyg();
        let no = hazir_fis(&u);
        let token = "deneme-belirtec-0002";
        u.cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        // GET imza: kayıt yok -> 404
        assert_eq!(
            u.isle(&pc("GET", &format!("/api/fis/{no}/imza"), "")).kod,
            404
        );
        let y = u.isle(&tel(
            token,
            "POST",
            &format!("/api/fis/{no}/teslim-al"),
            &json!({"fisli":true,"fis_karekodu":fis_kk(&u, &no),"teslim_alan":"Deneme Müşteri","iliski":"kendisi","telefon_son4":"9999"}).to_string(),
        ));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j = json_of(&y);
        assert_eq!(j["teslim_kaydi"]["dogrulama"], "fis");
        assert_eq!(j["teslim_kaydi"]["imzali"], false);
        assert_eq!(
            u.isle(&pc("GET", &format!("/api/fis/{no}/imza"), "")).kod,
            404
        );
        // fisli alanı zorunlu, geçersiz fiş no
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-al"), "{}"))
                .kod,
            400
        );
        assert_eq!(
            u.isle(&pc("POST", "/api/fis/F..x/teslim-al", r#"{"fisli":true}"#))
                .kod,
            400
        );
    }

    #[test]
    fn bagli_fis_sorusu_teslim_alda_da() {
        let u = uyg();
        let a = hazir_fis(&u);
        let b = hazir_fis(&u);
        assert_eq!(
            u.isle(&pc(
                "POST",
                &format!("/api/fis/{a}/bagla"),
                &format!(r#"{{"diger":"{b}"}}"#)
            ))
            .kod,
            200
        );
        let yol = format!("/api/fis/{a}/teslim-al");
        let imza = imza_b64();
        let y = u.isle(&pc("POST", &yol, &fissiz("4567", Some(&imza))));
        assert_eq!(y.kod, 409);
        let j = json_of(&y);
        assert_eq!(j["soru"], "bagli_teslim");
        assert_eq!(j["bagli"][0]["no"], b.as_str());
        // soru sırasında imza dosyası yazılmaz
        assert!(!u.dizin.join("imzalar").exists());
        let mut v: serde_json::Value = serde_json::from_str(&fissiz("4567", Some(&imza))).unwrap();
        v["bagli"] = json!([b]);
        let y = u.isle(&pc("POST", &yol, &v.to_string()));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(json_of(&y)["birlikte_teslim"], json!([b]));
        let d = Defter { yol: &u.fis_yolu() };
        let fb = d.bul(&b).unwrap().unwrap();
        assert_eq!(fb.durum, Durum::Teslim);
        // bağlı fişe de aynı teslim kaydı ana fiş atfıyla yazılır (aynı imza)
        let ta = d.bul(&a).unwrap().unwrap().teslim_kaydi.unwrap();
        let tb = fb.teslim_kaydi.clone().unwrap();
        assert_eq!(tb.ana_fis, a);
        assert_eq!(
            (
                tb.teslim_alan.as_str(),
                tb.fisli,
                tb.imza_sha256.as_str(),
                tb.imza_dosyasi.as_str()
            ),
            (
                ta.teslim_alan.as_str(),
                ta.fisli,
                ta.imza_sha256.as_str(),
                ta.imza_dosyasi.as_str()
            )
        );
        assert!(ta.ana_fis.is_empty());
        let m = String::from_utf8(u.isle(&pc("GET", &format!("/api/fis/{b}/metin"), "")).govde)
            .unwrap()
            .replace('\n', " ");
        assert!(m.contains("Teslim alan: D*** Y*** K*** (yakını)"), "{m}");
        assert!(m.contains(&format!("Birlikte teslim: {a}")), "{m}");
        // bağlı fişin imzası (ana fişin dosyası) okunur
        let g = u.isle(&pc("GET", &format!("/api/fis/{b}/imza"), ""));
        assert_eq!(g.kod, 200);
        assert_eq!(sha256_bayt(&g.govde), ta.imza_sha256);
    }

    #[test]
    fn buyuk_imza_govde_siniri() {
        let u = uyg();
        let no = hazir_fis(&u);
        // 300 KB'ı aşan imza 413 (gövde sınırı teslim-al için geniş)
        let mut b = sahte_png(400, 150);
        b.resize(EN_BUYUK_IMZA + 1, 0);
        let govde = fissiz("4567", Some(&base64_kodla(&b)));
        assert!(govde.len() > GOVDE_SINIRI && govde.len() < GOVDE_SINIRI_TESLIM_AL);
        let y = u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-al"), &govde));
        assert_eq!(y.kod, 413);
        assert_eq!(json_of(&y)["hata"], "imza en çok 300 KB olmalı");
    }

    // ---------- Karekodla teslim onayı (sahte bulut, sahte kişi) ----------

    /// Bulut sözleşmesini taklit eden sahte bulut: imzaları doğrular, kayıtları bellekte tutar.
    pub(crate) struct SahteBulut {
        genel: ed25519_dalek::VerifyingKey,
        pub(crate) kayit: Mutex<BTreeMap<String, serde_json::Value>>,
        dusuk: Mutex<Vec<String>>,
        pub(crate) istekler: Mutex<Vec<String>>,
        /// Bulut, mağaza saatini yanlış bulur (aç: 400, oku: 401 "zaman geçersiz").
        saat_yanlis: std::sync::atomic::AtomicBool,
    }

    impl SahteBulut {
        fn yeni(dizin: &Path) -> std::sync::Arc<Self> {
            let g: [u8; 32] = musteri_takip::hex_coz(&musteri_takip::genel_anahtar(dizin).unwrap())
                .unwrap()
                .try_into()
                .unwrap();
            std::sync::Arc::new(SahteBulut {
                genel: ed25519_dalek::VerifyingKey::from_bytes(&g).unwrap(),
                kayit: Default::default(),
                dusuk: Default::default(),
                istekler: Default::default(),
                saat_yanlis: Default::default(),
            })
        }
        fn imza_dogru(&self, metin: &str, imza: &str) -> bool {
            use ed25519_dalek::Verifier;
            let Some(i) = musteri_takip::hex_coz(imza).and_then(|v| <[u8; 64]>::try_from(v).ok())
            else {
                return false;
            };
            self.genel
                .verify(metin.as_bytes(), &ed25519_dalek::Signature::from_bytes(&i))
                .is_ok()
        }
        /// Müşterinin bulut sayfasında onaylaması (tarayıcı şifrelemesi, sözleşmeye göre).
        pub(crate) fn musteri_onaylar(&self, taban: &str, t: &str, ad: &str, tel: &str) {
            self.musteri_onaylar_imzali(taban, t, ad, tel, None)
        }
        /// Müşteri karekod sayfasında parmakla imza da çizer ("imza": base64 PNG, düz metinle birlikte şifrelenir).
        pub(crate) fn musteri_onaylar_imzali(
            &self,
            taban: &str,
            t: &str,
            ad: &str,
            tel: &str,
            imza: Option<&str>,
        ) {
            let mut k = self.kayit.lock().unwrap();
            let r = k.get_mut(&format!("{taban}|{t}")).expect("bulut kaydı");
            // Onay anı: karekod açıldıktan 1 sn sonra (sunucu, zamanın ts..sil aralığında olduğunu denetler).
            let zaman = r["sil"].as_u64().unwrap() - crate::teslim_onay::ONAY_OMRU_SN + 1;
            let mut duz = json!({
                "ad": ad, "tel": tel,
                "ua": "Mozilla/5.0 (Linux; Android 14; SM-SAHTE) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Mobile Safari/537.36",
                "ip": "198.51.100.23", "zaman": zaman, "dil": "tr",
            });
            if let Some(i) = imza {
                duz["imza"] = json!(i);
            }
            let duz = duz.to_string();
            let (e, iv, c) = crate::teslim_onay::testler::sozlesme_sifrele(
                r["pk"].as_str().unwrap(),
                t,
                duz.as_bytes(),
                [5u8; 32],
                [6u8; 12],
            );
            r["durum"] = json!("onaylandi");
            r["e"] = json!(e);
            r["iv"] = json!(iv);
            r["c"] = json!(c);
        }
    }

    impl crate::teslim_onay::OnayAgi for SahteBulut {
        fn istek(
            &self,
            metot: &str,
            url: &str,
            basliklar: &[(&str, &str)],
            govde: Option<&str>,
        ) -> Result<(u16, String), String> {
            self.istekler
                .lock()
                .unwrap()
                .push(format!("{metot} {url} {}", govde.unwrap_or("")));
            let (taban, yol) = url.split_once("/api/onay/").ok_or("adres")?;
            if self.dusuk.lock().unwrap().iter().any(|d| d == taban) {
                return Err("ağ yok".into());
            }
            if self.saat_yanlis.load(std::sync::atomic::Ordering::SeqCst) && yol != "kapat" {
                let kod = if metot == "GET" { 401 } else { 400 };
                return Ok((kod, r#"{"hata":"zaman geçersiz"}"#.into()));
            }
            let baslik = |a: &str| {
                basliklar
                    .iter()
                    .find(|(k, _)| k.eq_ignore_ascii_case(a))
                    .map(|(_, d)| d.to_string())
                    .unwrap_or_default()
            };
            let mut k = self.kayit.lock().unwrap();
            match (metot, yol) {
                ("POST", "ac") | ("POST", "kapat") => {
                    let g = govde.unwrap_or("");
                    if !self.imza_dogru(g, &baslik("x-taa-imza")) {
                        return Ok((401, String::new()));
                    }
                    let v: serde_json::Value = serde_json::from_str(g).unwrap();
                    let t = v["t"].as_str().unwrap().to_string();
                    assert!(crate::teslim_onay::belirtec_gecerli(&t));
                    if yol == "kapat" {
                        assert_eq!(v["tur"], "onay_kapat");
                        k.remove(&format!("{taban}|{t}"));
                        return Ok((200, "{}".into()));
                    }
                    assert_eq!(v["tur"], "onay_ac");
                    assert_eq!(v["v"], 1);
                    let (ts, sil) = (v["ts"].as_u64().unwrap(), v["sil"].as_u64().unwrap());
                    assert!(ts < sil && sil <= ts + 900);
                    assert_eq!(v["pk"].as_str().unwrap().len(), 64);
                    assert!((1..=50).contains(&v["n"].as_u64().unwrap()));
                    assert_eq!(v["s"], "MRK01");
                    // sözleşme: "i" her zaman var (fişsiz 1, fişli 0)
                    assert!(v["i"] == 0 || v["i"] == 1, "{v}");
                    let a = format!("{taban}|{t}");
                    if k.contains_key(&a) {
                        return Ok((409, String::new()));
                    }
                    k.insert(
                        a,
                        json!({ "pk": v["pk"], "durum": "bekliyor", "sil": sil, "i": v["i"] }),
                    );
                    Ok((200, "{}".into()))
                }
                ("GET", t) => {
                    let ts = baslik("x-taa-zaman");
                    if !self.imza_dogru(&format!("onay_oku:{t}:{ts}"), &baslik("x-taa-imza")) {
                        return Ok((401, String::new()));
                    }
                    match k.get(&format!("{taban}|{t}")) {
                        Some(r) if r["durum"] == "onaylandi" => Ok((
                            200,
                            json!({ "durum": "onaylandi", "e": r["e"], "iv": r["iv"], "c": r["c"] })
                                .to_string(),
                        )),
                        Some(_) => Ok((200, r#"{"durum":"bekliyor"}"#.into())),
                        None => Ok((404, String::new())),
                    }
                }
                _ => Ok((404, String::new())),
            }
        }
    }

    pub(crate) const A: &str = "https://a.example";
    const B: &str = "https://b.example";
    /// Sahte teslim alan (gerçek kişi değildir).
    pub(crate) const QR_AD: &str = "Zeynep Örnekoğlu";
    pub(crate) const QR_TEL: &str = "+905559998877";

    pub(crate) fn qr_uyg() -> (Uygulama, std::sync::Arc<SahteBulut>) {
        let mut u = uyg();
        std::fs::write(
            u.dizin.join("musteri-takip.json"),
            r#"{"acik":true,"sayfa":"https://a.example","yedek_sayfa":"https://b.example","hedefler":[]}"#,
        )
        .unwrap();
        let b = SahteBulut::yeni(&u.dizin);
        u.onay_agi = b.clone();
        u.onay_arka_plan = false;
        (u, b)
    }

    /// Karekod açar: (t, url).
    fn qr_ac(u: &Uygulama, no: &str) -> (String, String) {
        let y = u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-qr"), "{}"));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j = json_of(&y);
        assert!(j["qr_svg"].as_str().unwrap().starts_with("<svg"));
        assert_eq!(j["kalan_sn"], 900);
        (
            j["t"].as_str().unwrap().to_string(),
            j["url"].as_str().unwrap().to_string(),
        )
    }

    pub(crate) fn tum_dosyalarda(k: &Path, aranan: &[&str], bulunan: &mut Vec<String>) {
        for e in std::fs::read_dir(k).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                tum_dosyalarda(&p, aranan, bulunan);
            } else {
                let b = std::fs::read(&p).unwrap();
                for a in aranan {
                    if b.windows(a.len()).any(|w| w == a.as_bytes()) {
                        bulunan.push(format!("{} içinde {a}", p.display()));
                    }
                }
            }
        }
    }

    #[test]
    fn qr_ile_fisli_teslim_ve_sifreli_kayit() {
        let (u, bulut) = qr_uyg();
        let no = hazir_fis(&u);
        let (t, url) = qr_ac(&u, &no);
        assert_eq!(url, format!("{A}/o/{t}"));
        // iki buluta da açıldı
        assert!(bulut
            .kayit
            .lock()
            .unwrap()
            .contains_key(&format!("{A}|{t}")));
        assert!(bulut
            .kayit
            .lock()
            .unwrap()
            .contains_key(&format!("{B}|{t}")));
        let durum = format!("/api/fis/{no}/teslim-qr/{t}");
        let y = u.isle(&pc("GET", &durum, ""));
        assert_eq!(y.kod, 200);
        assert_eq!(json_of(&y)["durum"], "bekliyor");
        // onay gelmeden teslim tamamlanamaz
        let govde = json!({ "fisli": true, "fis_karekodu": fis_kk(&u, &no), "onay_belirteci": t })
            .to_string();
        let y = u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-al"), &govde));
        assert_eq!(y.kod, 409);
        assert_eq!(json_of(&y)["hata"], "onay_bekliyor");
        // müşteri kendi telefonundan onaylar
        bulut.musteri_onaylar(A, &t, QR_AD, QR_TEL);
        u.ileri_sn.fetch_add(2, std::sync::atomic::Ordering::SeqCst);
        let y = u.isle(&pc("GET", &durum, ""));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j = json_of(&y);
        assert_eq!(j["durum"], "onaylandi");
        assert_eq!(j["ad"], QR_AD);
        assert_eq!(j["tel_maskeli"], "+••••••••8877");
        assert_eq!(j["cihaz"], "Android 14 · SM-SAHTE · Chrome");
        let onay_zamani = j["zaman"].as_u64().unwrap();
        assert!(onay_zamani > 1_795_000_000);
        // karekoddaki telefon fişteki numaradan farklı (numara açılmadan bildirilir)
        assert_eq!(j["fis_telefonu"], "farkli");
        assert!(
            !String::from_utf8_lossy(&y.govde).contains("9998877"),
            "tam telefon ekrana gelmez"
        );
        // bulut kaydı kapatıldı (iki buluttan da silindi)
        assert!(bulut.kayit.lock().unwrap().is_empty());
        // telefon farklıyken ilişki seçilmeden (ya da "kendisi" ile) teslim edilemez
        let y = u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-al"), &govde));
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "iliski_gerekli");
        let g_kendisi =
            json!({ "fisli": true, "fis_karekodu": fis_kk(&u, &no), "onay_belirteci": t, "iliski": "kendisi" }).to_string();
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-al"), &g_kendisi))
                .kod,
            422
        );
        let govde = json!({ "fisli": true, "fis_karekodu": fis_kk(&u, &no), "onay_belirteci": t, "iliski": "yakini" }).to_string();
        // teslim: imza gerekmez, doğrulama "qr"
        let y = u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-al"), &govde));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j = json_of(&y);
        assert_eq!(j["teslim_kaydi"]["dogrulama"], "qr");
        assert_eq!(j["teslim_kaydi"]["imzali"], false);
        // teslimden sonra maskeli
        assert_eq!(j["teslim_kaydi"]["ad"], "Z*** Ö***");
        let d = Defter { yol: &u.fis_yolu() };
        let f = d.bul(&no).unwrap().unwrap();
        assert_eq!(f.durum, Durum::Teslim);
        let tk = f.teslim_kaydi.clone().unwrap();
        assert!(tk.teslim_alan.is_empty() && tk.imza_dosyasi.is_empty());
        let s = tk.sifreli.clone().unwrap();
        assert_eq!(s.surum, 1);
        // kör indeks: aynı ad/telefon aynı indeks
        assert_eq!(
            teslim_sifre::indeksler(&u.dizin, "zeynep  örnekoğlu", "0555 999 88 77").unwrap(),
            (s.ad_indeks.clone(), s.tel_indeks.clone())
        );
        // çözülen kayıt: ad, telefon, okutan cihaz, IP, onay zamanı, yöntem
        let duz = teslim_sifre::coz(&u.dizin, &no, &s).unwrap();
        assert_eq!(
            (
                duz.ad.as_str(),
                duz.tel.as_str(),
                duz.ip.as_str(),
                duz.zaman,
                duz.yontem.as_str()
            ),
            (QR_AD, QR_TEL, "198.51.100.23", onay_zamani, "qr")
        );
        assert!(duz.cihaz.contains("SM-SAHTE"));
        // defterde ve veri klasörünün HİÇBİR dosyasında ad/telefon/IP/cihaz düz metin yok
        let mut bulunan = Vec::new();
        tum_dosyalarda(
            &u.dizin,
            &[
                "Zeynep",
                "Örnekoğlu",
                "rnekog",
                "9998877",
                "198.51.100",
                "SM-SAHTE",
            ],
            &mut bulunan,
        );
        assert!(bulunan.is_empty(), "düz metin kişisel veri: {bulunan:?}");
        // fiş ayrıntısı: mağaza PC'si MASKELİ görünümü alır (ad "Z*** Ö***", tam telefon ve IP yok)
        let y = u.isle(&pc("GET", &format!("/api/fis/{no}"), ""));
        let j = json_of(&y);
        assert_eq!(j["teslim_kaydi"]["ad"], "Z*** Ö***");
        assert!(j["teslim_kaydi"]["ip"].is_null());
        assert!(!String::from_utf8_lossy(&y.govde).contains("Zeynep"));
        assert!(!String::from_utf8_lossy(&y.govde).contains("198.51.100"));
        // açık görünüm ayrı uçtan (yönetici kilidi kapalı: bu bilgisayar yönetici sayılır) ve günlüğe yazılır
        let ya = json_of(&u.isle(&pc("GET", &format!("/api/fis/{no}/teslim-acik"), "")));
        assert_eq!(
            (ya["ad"].clone(), ya["tel"].clone(), ya["ip"].clone()),
            (json!(QR_AD), json!(QR_TEL), json!("198.51.100.23"))
        );
        assert_eq!(j["teslim_kaydi"]["tel_maskeli"], "+••••••••8877");
        assert_eq!(j["teslim_kaydi"]["yontem"], "qr");
        assert_eq!(j["teslim_kaydi"]["sifreli"], true);
        assert!(!y.govde.windows(7).any(|w| w == b"9998877"));
        assert!(
            j["teslim_kaydi"]["ad_indeks"].is_null(),
            "şifreli blok istemciye gitmez"
        );
        // yetkisiz telefon: yalnız "şifreli" işareti, ad yok
        let token = "deneme-belirtec-0q01";
        u.cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        let y = u.isle(&tel(token, "GET", &format!("/api/fis/{no}"), ""));
        let j = json_of(&y);
        assert_eq!(j["teslim_kaydi"]["sifreli"], true);
        assert!(j["teslim_kaydi"]["ad"].is_null());
        assert!(!String::from_utf8_lossy(&y.govde).contains("Zeynep"));
        let y = u.isle(&tel(token, "GET", "/api/fisler", ""));
        assert!(!String::from_utf8_lossy(&y.govde).contains("Zeynep"));
        // mağaza nüshası metni: "Ad S. - QR onaylı, tarih saat"; telefon basılmaz
        let m = String::from_utf8(
            u.isle(&pc("GET", &format!("/api/fis/{no}/metin"), ""))
                .govde,
        )
        .unwrap()
        .replace('\n', " ");
        let gun = tk.tarih.split('-').rev().collect::<Vec<_>>().join(".");
        assert!(
            m.contains(&format!(
                "Teslim alan: Z*** Ö*** - QR onaylı, {gun} {}",
                &tk.saat[..5]
            )),
            "{m}"
        );
        assert!(!m.contains("8877") && !m.contains("Örnekoğlu") && !m.contains("Zeynep"));
        // yetkisiz telefonda ad çözülmez
        let m = String::from_utf8(
            u.isle(&tel(token, "GET", &format!("/api/fis/{no}/metin"), ""))
                .govde,
        )
        .unwrap();
        assert!(!m.contains("Zeynep"), "{m}");
        // belirteç tek kullanımlık
        let no2 = hazir_fis(&u);
        let y = u.isle(&pc("POST", &format!("/api/fis/{no2}/teslim-al"), &govde));
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "onay_yok");
        // giden isteklerde kişisel veri yok (aç/oku/kapat yalnız belirteç, şube, sayı, genel anahtar taşır)
        for i in bulut.istekler.lock().unwrap().iter() {
            assert!(
                !i.contains("Zeynep") && !i.contains("9998877") && !i.contains(&no),
                "{i}"
            );
        }
    }

    #[test]
    fn qr_ile_fissiz_teslim_son4_ve_yetki() {
        let (u, bulut) = qr_uyg();
        let no = hazir_fis(&u);
        // fişsiz karekod: aç gövdesinde "i": 1 (müşteri karekod sayfasında imza çizer)
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no}/teslim-qr"),
            r#"{"fisli":false}"#,
        ));
        assert_eq!(y.kod, 200);
        assert_eq!(json_of(&y)["imza_gerekli"], true);
        let t = json_of(&y)["t"].as_str().unwrap().to_string();
        assert_eq!(bulut.kayit.lock().unwrap()[&format!("{A}|{t}")]["i"], 1);
        let musteri_imzasi = base64_kodla(&imza_png(600, 220));
        bulut.musteri_onaylar_imzali(A, &t, QR_AD, QR_TEL, Some(&musteri_imzasi));
        let j = json_of(&u.isle(&pc("GET", &format!("/api/fis/{no}/teslim-qr/{t}"), "")));
        assert_eq!(j["durum"], "onaylandi");
        assert_eq!(
            (j["imzali"].clone(), j["imza_gerekli"].clone()),
            (json!(true), json!(true))
        );
        assert!(
            j.get("imza").is_none(),
            "imza görüntüsü durum yanıtına konmaz"
        );
        let yol = format!("/api/fis/{no}/teslim-al");
        let g = |son4: &str| {
            json!({ "fisli": false, "onay_belirteci": t, "telefon_son4": son4, "iliski": "yakini" })
                .to_string()
        };
        // son 4 doğrulaması sürer (yanlış: 422, hak sayılır)
        let y = u.isle(&pc("POST", &yol, &g("1111")));
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "son4_yanlis");
        // doğru son 4: imza karekoddan gelir (qr_imza), doğrulama "qr+telefon-son4"
        let y = u.isle(&pc("POST", &yol, &g("4567")));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j = json_of(&y);
        assert_eq!(j["teslim_kaydi"]["dogrulama"], "qr+telefon-son4");
        assert_eq!(j["teslim_kaydi"]["imza_yontemi"], "qr_imza");
        let f = Defter { yol: &u.fis_yolu() }.bul(&no).unwrap().unwrap();
        let tk = f.teslim_kaydi.unwrap();
        assert!(!tk.fisli && tk.iliski == "yakini");
        assert_eq!(tk.imza_yontemi, "qr_imza");
        assert_eq!(tk.imza_dosyasi, format!("imzalar/{no}-1.png.enc"));
        assert_eq!(tk.imza_sha256, sha256_bayt(&imza_png(600, 220)));
        // imza yerelde şifreli; sunucuda çözülüp yetkili cihaza gösterilir
        let enc = std::fs::read(u.dizin.join(&tk.imza_dosyasi)).unwrap();
        assert!(!enc.windows(11).any(|w| w == b"DENEME-IMZA"));
        let g2 = u.isle(&pc("GET", &format!("/api/fis/{no}/imza"), ""));
        assert_eq!(g2.kod, 200);
        assert_eq!(g2.govde, imza_png(600, 220));
        // düz metin hiçbir dosyada yok (imza dahil)
        let mut bulunan = Vec::new();
        tum_dosyalarda(
            &u.dizin,
            &["Zeynep", "9998877", "198.51.100", "DENEME-IMZA"],
            &mut bulunan,
        );
        assert!(bulunan.is_empty(), "{bulunan:?}");

        // fişsiz karekod, bulut imza GÖNDERMEDİ (eski bulut): imzasız 422 imza_gerekli; cihaz imzasıyla olur
        let no3 = hazir_fis(&u);
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no3}/teslim-qr"),
            r#"{"fisli":false}"#,
        ));
        let t3 = json_of(&y)["t"].as_str().unwrap().to_string();
        bulut.musteri_onaylar(A, &t3, QR_AD, QR_TEL);
        let j = json_of(&u.isle(&pc("GET", &format!("/api/fis/{no3}/teslim-qr/{t3}"), "")));
        assert_eq!(
            (j["imzali"].clone(), j["imza_gerekli"].clone()),
            (json!(false), json!(true))
        );
        let mut g3 = json!({ "fisli": false, "onay_belirteci": t3, "telefon_son4": "4567", "iliski": "yakini" });
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no3}/teslim-al"),
            &g3.to_string(),
        ));
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "imza_gerekli");
        g3["imza_png_base64"] = json!(imza_b64());
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no3}/teslim-al"),
            &g3.to_string(),
        ));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(json_of(&y)["teslim_kaydi"]["imza_yontemi"], "cihaz_imza");

        // bozuk imzalı onay (PNG değil): onay "hatali", teslimde kullanılamaz
        let no4 = hazir_fis(&u);
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no4}/teslim-qr"),
            r#"{"fisli":false}"#,
        ));
        let t4 = json_of(&y)["t"].as_str().unwrap().to_string();
        bulut.musteri_onaylar_imzali(A, &t4, QR_AD, QR_TEL, Some(&base64_kodla(b"GIF89a-deneme")));
        let y = u.isle(&pc("GET", &format!("/api/fis/{no4}/teslim-qr/{t4}"), ""));
        assert_eq!(y.kod, 422);
        assert!(json_of(&y)["ileti"].as_str().unwrap().contains("imza"));

        // fişli karekod: "i": 0; gövdesiz eski istemci de fişli sayılır
        let no5 = hazir_fis(&u);
        let (t5, _) = qr_ac(&u, &no5);
        assert_eq!(bulut.kayit.lock().unwrap()[&format!("{A}|{t5}")]["i"], 0);
        let y = u.isle(&pc("POST", &format!("/api/fis/{no5}/teslim-qr"), ""));
        assert_eq!(y.kod, 200);
        assert_eq!(json_of(&y)["imza_gerekli"], false);

        // fişsiz teslim yetkisi olmayan telefon karekod onayıyla da fişsiz teslim edemez
        let no2 = hazir_fis(&u);
        let token = "deneme-belirtec-0q02";
        u.cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        let y = u.isle(&tel(
            token,
            "POST",
            &format!("/api/fis/{no2}/teslim-qr"),
            r#"{"fisli":false}"#,
        ));
        assert_eq!(y.kod, 200);
        let t2 = json_of(&y)["t"].as_str().unwrap().to_string();
        bulut.musteri_onaylar_imzali(A, &t2, QR_AD, QR_TEL, Some(&musteri_imzasi));
        let y = u.isle(&tel(
            token,
            "GET",
            &format!("/api/fis/{no2}/teslim-qr/{t2}"),
            "",
        ));
        assert_eq!(json_of(&y)["durum"], "onaylandi");
        let g2 =
            json!({ "fisli": false, "onay_belirteci": t2, "telefon_son4": "4567" }).to_string();
        let y = u.isle(&tel(
            token,
            "POST",
            &format!("/api/fis/{no2}/teslim-al"),
            &g2,
        ));
        assert_eq!(y.kod, 403);
        // fişsiz teslimde ne karekod ne imza: 422
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no2}/teslim-al"),
            &fissiz("4567", None),
        ));
        assert_eq!(y.kod, 422);
    }

    /// Fişli karekod teslimde müşteri imza da çizdiyse saklanır (isteğe bağlı), yöntem qr_imza.
    #[test]
    fn qr_fisli_istege_bagli_imza() {
        let (u, bulut) = qr_uyg();
        let no = hazir_fis(&u);
        let (t, _) = qr_ac(&u, &no);
        bulut.musteri_onaylar_imzali(A, &t, QR_AD, "+905320004567", Some(&imza_b64()));
        u.isle(&pc("GET", &format!("/api/fis/{no}/teslim-qr/{t}"), ""));
        let g = json!({ "fisli": true, "fis_karekodu": fis_kk(&u, &no), "onay_belirteci": t })
            .to_string();
        let y = u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-al"), &g));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let tk = Defter { yol: &u.fis_yolu() }
            .bul(&no)
            .unwrap()
            .unwrap()
            .teslim_kaydi
            .unwrap();
        assert_eq!(
            (tk.dogrulama.as_str(), tk.imza_yontemi.as_str()),
            ("qr", "qr_imza")
        );
        let m = String::from_utf8(
            u.isle(&pc("GET", &format!("/api/fis/{no}/metin"), ""))
                .govde,
        )
        .unwrap();
        assert!(m.replace('\n', " ").contains("QR onaylı, imzalı"), "{m}");
    }

    #[test]
    fn qr_yanlis_belirtec_ve_sure() {
        let (u, bulut) = qr_uyg();
        let a = hazir_fis(&u);
        let b = hazir_fis(&u);
        let (t, _) = qr_ac(&u, &a);
        // bilinmeyen / biçimsiz belirteç ve başka fişin belirteci reddedilir
        let bilinmeyen = "abcdefghijkmnpqrst23";
        for yol in [
            format!("/api/fis/{a}/teslim-qr/{bilinmeyen}"),
            format!("/api/fis/{a}/teslim-qr/KISA"),
            format!("/api/fis/{b}/teslim-qr/{t}"),
        ] {
            assert_eq!(u.isle(&pc("GET", &yol, "")).kod, 404, "{yol}");
        }
        bulut.musteri_onaylar(A, &t, QR_AD, QR_TEL);
        // başka cihaz bu belirteci sorgulayamaz ve teslimde kullanamaz
        let token = "deneme-belirtec-0q03";
        u.cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        assert_eq!(
            u.isle(&tel(
                token,
                "GET",
                &format!("/api/fis/{a}/teslim-qr/{t}"),
                ""
            ))
            .kod,
            404
        );
        assert_eq!(
            json_of(&u.isle(&pc("GET", &format!("/api/fis/{a}/teslim-qr/{t}"), "")))["durum"],
            "onaylandi"
        );
        let g = json!({ "fisli": true, "fis_karekodu": fis_kk(&u, &a), "onay_belirteci": t })
            .to_string();
        let y = u.isle(&tel(token, "POST", &format!("/api/fis/{a}/teslim-al"), &g));
        assert_eq!(y.kod, 422);
        // başka fişin teslim-al'ında kullanılamaz
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{b}/teslim-al"), &g))
                .kod,
            422
        );
        let g =
            json!({ "fisli": true, "fis_karekodu": fis_kk(&u, &a), "onay_belirteci": bilinmeyen })
                .to_string();
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{a}/teslim-al"), &g))
                .kod,
            422
        );
        // kurcalanmış şifreli onay: "hatali", teslimde kullanılamaz
        let (t3, _) = qr_ac(&u, &b);
        bulut.musteri_onaylar(A, &t3, QR_AD, QR_TEL);
        {
            let mut k = bulut.kayit.lock().unwrap();
            let r = k.get_mut(&format!("{A}|{t3}")).unwrap();
            let c = r["c"].as_str().unwrap().to_string();
            let ilk = if c.starts_with('A') { "B" } else { "A" };
            r["c"] = json!(format!("{ilk}{}", &c[1..]));
        }
        let y = u.isle(&pc("GET", &format!("/api/fis/{b}/teslim-qr/{t3}"), ""));
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["durum"], "hatali");
        let g3 = json!({ "fisli": true, "fis_karekodu": fis_kk(&u, &b), "onay_belirteci": t3 })
            .to_string();
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{b}/teslim-al"), &g3))
                .kod,
            409
        );
        // 15 dk sonra süresi dolar (410) ve teslimde kullanılamaz
        u.ileri_sn
            .fetch_add(ONAY_SN + 1, std::sync::atomic::Ordering::SeqCst);
        let y = u.isle(&pc("GET", &format!("/api/fis/{a}/teslim-qr/{t}"), ""));
        assert_eq!(y.kod, 410);
        assert_eq!(json_of(&y)["durum"], "suresi_doldu");
        let g = json!({ "fisli": true, "fis_karekodu": fis_kk(&u, &a), "onay_belirteci": t })
            .to_string();
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{a}/teslim-al"), &g))
                .kod,
            422
        );
    }
    const ONAY_SN: u64 = crate::teslim_onay::ONAY_OMRU_SN;

    #[test]
    fn qr_birincil_dusukse_yedek_ve_ayar_yoksa_kapali() {
        let u0 = uyg();
        let no0 = hazir_fis(&u0);
        let y = u0.isle(&pc("POST", &format!("/api/fis/{no0}/teslim-qr"), "{}"));
        assert_eq!(y.kod, 409);
        assert_eq!(json_of(&y)["hata"], "qr_kapali");
        let (u, bulut) = qr_uyg();
        let no = hazir_fis(&u);
        bulut.dusuk.lock().unwrap().push(A.into());
        let (t, url) = qr_ac(&u, &no);
        assert_eq!(
            url,
            format!("{B}/o/{t}"),
            "birincile ulaşılamayınca yedek adres"
        );
        bulut.musteri_onaylar(B, &t, QR_AD, QR_TEL);
        let y = u.isle(&pc("GET", &format!("/api/fis/{no}/teslim-qr/{t}"), ""));
        assert_eq!(json_of(&y)["durum"], "onaylandi");
        // iki bulut da düşükse karekod açılmaz (502), ekran imzası yedeği kalır
        bulut.dusuk.lock().unwrap().push(B.into());
        let no2 = hazir_fis(&u);
        let y = u.isle(&pc("POST", &format!("/api/fis/{no2}/teslim-qr"), "{}"));
        assert_eq!(y.kod, 502);
        assert_eq!(json_of(&y)["hata"], "bulut_yok");
        // teslim edilmiş fiş için karekod açılmaz
        bulut.dusuk.lock().unwrap().clear();
        let g = json!({ "fisli": true, "fis_karekodu": fis_kk(&u, &no), "onay_belirteci": t, "iliski": "diger" }).to_string();
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-al"), &g))
                .kod,
            200
        );
        let y = u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-qr"), "{}"));
        assert_eq!(y.kod, 409);
    }

    #[test]
    fn ekran_imzasi_yolunda_ad_sifreli() {
        let u = uyg();
        let no = hazir_fis(&u);
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{no}/teslim-al"),
            &fissiz("4567", Some(&imza_b64())),
        ));
        assert_eq!(y.kod, 200);
        let defter = std::fs::read_to_string(u.fis_yolu()).unwrap();
        let satir = defter.lines().find(|l| l.contains("teslimkaydi")).unwrap();
        assert!(
            !satir.contains("Yakın") && !satir.contains("Deneme"),
            "{satir}"
        );
        let tk = Defter { yol: &u.fis_yolu() }
            .bul(&no)
            .unwrap()
            .unwrap()
            .teslim_kaydi
            .unwrap();
        let d = teslim_sifre::coz(&u.dizin, &no, tk.sifreli.as_ref().unwrap()).unwrap();
        assert_eq!(
            (d.ad.as_str(), d.yontem.as_str(), d.tel.as_str()),
            ("Deneme Yakın Kişi", "ekran-imza", "")
        );
    }

    /// Karekoddaki telefon fiştekiyle aynıysa ilişki seçmeden ("kendisi") teslim edilir.
    #[test]
    fn qr_ayni_telefon_ve_birlikte_atlanan_alani() {
        let (u, bulut) = qr_uyg();
        let no = hazir_fis(&u);
        let (t, _) = qr_ac(&u, &no);
        bulut.musteri_onaylar(A, &t, QR_AD, "+905320004567");
        let j = json_of(&u.isle(&pc("GET", &format!("/api/fis/{no}/teslim-qr/{t}"), "")));
        assert_eq!(j["durum"], "onaylandi");
        assert_eq!(j["fis_telefonu"], "ayni");
        let g = json!({ "fisli": true, "fis_karekodu": fis_kk(&u, &no), "onay_belirteci": t })
            .to_string();
        let y = u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-al"), &g));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j = json_of(&y);
        assert_eq!(j["teslim_kaydi"]["iliski"], "kendisi");
        assert_eq!(j["birlikte_atlanan"], json!([]));
    }

    /// Yönetici: tam telefon, IP ve ham user-agent yalnız bu bilgisayardan + yönetici oturumuyla görülür; kör
    /// indeksle arama; her görüntüleme kişisel veri içermeyen satırla günlüğe yazılır.
    #[test]
    fn yonetici_teslim_ayrintisi_ve_kor_indeks_aramasi() {
        let (u, bulut) = qr_uyg();
        let no = hazir_fis(&u);
        let (t, _) = qr_ac(&u, &no);
        bulut.musteri_onaylar(A, &t, QR_AD, QR_TEL);
        u.isle(&pc("GET", &format!("/api/fis/{no}/teslim-qr/{t}"), ""));
        let g = json!({ "fisli": true, "fis_karekodu": fis_kk(&u, &no), "onay_belirteci": t, "iliski": "yakini" }).to_string();
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-al"), &g))
                .kod,
            200
        );
        let yol = format!("/api/yonetim/teslim/{no}");
        // uzaktan (telefon / ağ) görülemez
        let mut uzak = pc("GET", &yol, "");
        uzak.yerel = false;
        assert_eq!(u.isle(&uzak).kod, 403);
        let token = "deneme-belirtec-0q09";
        u.cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        assert_eq!(u.isle(&tel(token, "GET", &yol, "")).kod, 403);
        let y = u.isle(&pc("GET", &yol, ""));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j = json_of(&y);
        assert_eq!(j["tel"], QR_TEL);
        assert_eq!(j["ip"], "198.51.100.23");
        assert!(j["cihaz"].as_str().unwrap().contains("SM-SAHTE"));
        assert_eq!(j["fis_telefonu"], "farkli");
        assert_eq!(j["iliski"], "yakini");
        // kör indeksle arama: telefon biçimi farklı da olsa bulur; yanıtta ad/telefon yok
        let y = u.isle(&pc(
            "POST",
            "/api/yonetim/teslim-ara",
            r#"{"tel":"0555 999 88 77"}"#,
        ));
        assert_eq!(y.kod, 200);
        let j = json_of(&y);
        assert_eq!(j["sonuc"].as_array().unwrap().len(), 1);
        assert_eq!(j["sonuc"][0]["no"], no.as_str());
        assert!(!String::from_utf8_lossy(&y.govde).contains("9998877"));
        let y = u.isle(&pc(
            "POST",
            "/api/yonetim/teslim-ara",
            r#"{"ad":"zeynep ÖRNEKOĞLU","tel":"+905559998877"}"#,
        ));
        assert_eq!(json_of(&y)["sonuc"].as_array().unwrap().len(), 1);
        let y = u.isle(&pc(
            "POST",
            "/api/yonetim/teslim-ara",
            r#"{"tel":"0555 000 00 00"}"#,
        ));
        assert!(json_of(&y)["sonuc"].as_array().unwrap().is_empty());
        assert_eq!(
            u.isle(&pc("POST", "/api/yonetim/teslim-ara", "{}")).kod,
            400
        );
        // yönetici kilidi açıkken oturumsuz 401
        let mut u2 = u;
        u2.yonetici_kilidi = true;
        assert_eq!(u2.isle(&pc("GET", &yol, "")).kod, 401);
        // günlükte kişisel veri yok, olaylar var
        let gunluk = std::fs::read_to_string(u2.dizin.join("teslim-ayrinti.jsonl")).unwrap();
        assert!(gunluk.contains("teslim_ayrintisi") && gunluk.contains("teslim_ara"));
        for yasak in ["Zeynep", "9998877", "198.51.100", "SM-SAHTE"] {
            assert!(!gunluk.contains(yasak), "{yasak}: {gunluk}");
        }
    }

    /// Bilgisayar saati yanlışsa bulut reddeder: "bağlantı yok" yerine saat iletisi.
    #[test]
    fn qr_saat_farki_iletisi() {
        let (u, bulut) = qr_uyg();
        let no = hazir_fis(&u);
        let (t, _) = qr_ac(&u, &no);
        bulut
            .saat_yanlis
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let j = json_of(&u.isle(&pc("GET", &format!("/api/fis/{no}/teslim-qr/{t}"), "")));
        assert_eq!(j["durum"], "bekliyor");
        assert_eq!(j["saat_sorunu"], true);
        let no2 = hazir_fis(&u);
        let y = u.isle(&pc("POST", &format!("/api/fis/{no2}/teslim-qr"), "{}"));
        assert_eq!(y.kod, 502);
        let j = json_of(&y);
        assert_eq!(j["hata"], "saat_farki");
        assert!(j["ileti"].as_str().unwrap().contains("saat"));
    }

    /// Süresi geçen (onaylanmamış) karekodun bulut kaydı kapattırılır (Vercel'de kendi silme zamanı yok).
    #[test]
    fn suresi_gecen_karekod_buluttan_silinir() {
        let (u, bulut) = qr_uyg();
        let a = hazir_fis(&u);
        let (t, _) = qr_ac(&u, &a);
        assert_eq!(bulut.kayit.lock().unwrap().len(), 2);
        u.ileri_sn
            .fetch_add(ONAY_SN + 1, std::sync::atomic::Ordering::SeqCst);
        let y = u.isle(&pc("GET", &format!("/api/fis/{a}/teslim-qr/{t}"), ""));
        assert_eq!(y.kod, 410);
        assert!(
            bulut.kayit.lock().unwrap().is_empty(),
            "iki buluttaki kayıt da kapatılmalı"
        );
        // başka bir fişin karekodu açılırken de süresi geçenler taranır
        let b = hazir_fis(&u);
        let (t2, _) = qr_ac(&u, &b);
        u.ileri_sn
            .fetch_add(ONAY_SN + 1, std::sync::atomic::Ordering::SeqCst);
        let c = hazir_fis(&u);
        let (t3, _) = qr_ac(&u, &c);
        let k = bulut.kayit.lock().unwrap();
        assert!(!k.keys().any(|x| x.ends_with(&t2)), "{k:?}");
        assert!(k.keys().any(|x| x.ends_with(&t3)));
    }

    /// Onaydaki zaman karekodun açılış..silinme aralığı dışındaysa onay kabul edilmez.
    #[test]
    fn qr_onay_zamani_aralik_disi_reddedilir() {
        let (u, bulut) = qr_uyg();
        let no = hazir_fis(&u);
        let (t, _) = qr_ac(&u, &no);
        bulut.musteri_onaylar(A, &t, QR_AD, QR_TEL);
        // kaydı eski bir zamanla yeniden şifrele
        {
            let mut k = bulut.kayit.lock().unwrap();
            let r = k.get_mut(&format!("{A}|{t}")).unwrap();
            let duz = json!({ "ad": QR_AD, "tel": QR_TEL, "ua": "", "ip": "", "zaman": 1_000_000_000u64, "dil": "tr" }).to_string();
            let (e, iv, c) = crate::teslim_onay::testler::sozlesme_sifrele(
                r["pk"].as_str().unwrap(),
                &t,
                duz.as_bytes(),
                [5u8; 32],
                [6u8; 12],
            );
            r["e"] = json!(e);
            r["iv"] = json!(iv);
            r["c"] = json!(c);
        }
        let y = u.isle(&pc("GET", &format!("/api/fis/{no}/teslim-qr/{t}"), ""));
        assert_eq!(y.kod, 422);
        assert!(json_of(&y)["ileti"].as_str().unwrap().contains("zaman"));
    }
}
