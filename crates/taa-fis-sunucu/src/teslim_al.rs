//! Teslim alma ucu: ürünü teslim alan kişinin adı, ilişkisi ve imzası; müşteri fişiyle gelmediyse
//! (fişsiz teslim) telefonun son 4 hanesiyle doğrulama. Yusuf: "ürünü teslim alan kişi için imza alıyoruz,
//! bazen müşteri fişiyle gelmiyor".
//! - Fişsiz teslim "fissiz-teslim" yetkisi ister (varsayılan yalnız mağaza bilgisayarı).
//! - Son 4 hane hiçbir yere yazılmaz; kayıtta yalnız doğrulama yöntemi ("telefon-son4") kalır.
//! - Fiş başına 5 yanlışta 15 dk kilit (bellekte; yeniden başlatınca sıfırlanır).
//! - İmza PNG'si `imzalar/<no>-<n>.png` olarak atomik yazılır, SHA-256 özeti defterdeki kayda girer.
//! - Teslim alan bilgisi müşteri takip (bulut) kaydına GİTMEZ; bulut kaydı yalnız sabit anahtarlar taşır.

use super::*;
use std::collections::BTreeMap;

/// teslim-al isteğinin gövde sınırı (imza PNG'si base64 olarak gelir).
pub(crate) const GOVDE_SINIRI_TESLIM_AL: usize = 512 * 1024;
/// Çözülmüş imza PNG'si en çok bu kadar bayt.
const EN_BUYUK_IMZA: usize = 300 * 1024;
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
fn no_guvenli(no: &str) -> bool {
    !no.is_empty() && no.len() <= 40 && no.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// Sabit zamanlı karşılaştırma (uzunluk eşitse her baytı gezer).
fn sabit_esit(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

/// Standart base64 (RFC 4648) çözücü; isteğe bağlı `data:image/png;base64,` öneki ve boşluklar kabul edilir.
fn base64_coz(s: &str) -> Result<Vec<u8>, String> {
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
fn png_gecerli(b: &[u8]) -> bool {
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

/// İmzayı görebilen cihaz: mağaza bilgisayarı ya da "fissiz-teslim" / "duzelt" yetkisi olan cihaz.
/// Islak imza hassas kişisel veridir; usta ve kişisel telefonlar varsayılan olarak göremez.
pub(crate) fn imza_gorebilir(u: &Uygulama, c: &cihaz::Cihaz) -> bool {
    c.id == PC_ID || {
        let k = u.cihazlar();
        k.yetki_var(&c.id, "fissiz-teslim") || k.yetki_var(&c.id, "duzelt")
    }
}

fn sha256_bayt(b: &[u8]) -> String {
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
    fn son4_kilitli(&self, no: &str) -> Option<u64> {
        let simdi = self.simdi();
        let m = SON4_YANLIS.lock().ok()?;
        match m.get(&self.son4_anahtari(no)) {
            Some((_, bitis)) if *bitis > simdi => Some(bitis - simdi),
            _ => None,
        }
    }

    /// Yanlış denemeyi sayar; kalan hakkı döndürür (0 = şimdi kilitlendi).
    fn son4_yanlis(&self, no: &str) -> u32 {
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

    fn son4_sifirla(&self, no: &str) {
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

    /// Seçilen bağlı fişleri de teslim eder: her birine ana fişin teslim kaydı (aynı teslim alan, imza
    /// dosyası ve özeti) `ana_fis` atfıyla yazılır, müşteri takibine yazılır. Teslim edilenleri döndürür.
    pub(crate) fn birlikte_teslim_et(
        &self,
        d: &Defter,
        ana: &str,
        kayit: &taa_fis::TeslimKaydi,
        birlikte: &[String],
    ) -> Vec<String> {
        let mut edilen = Vec::new();
        for n in birlikte {
            let k = taa_fis::TeslimKaydi {
                ana_fis: ana.to_string(),
                ..kayit.clone()
            };
            if d.teslim_et(n, k).is_ok() {
                edilen.push(n.clone());
                if let Ok(Some(f)) = d.bul(n) {
                    self.takibe_yaz(&f);
                }
            }
        }
        edilen
    }

    /// POST /api/fis/<no>/teslim-al
    pub(crate) fn teslim_al(&self, d: &Defter, no: &str, c: &cihaz::Cihaz, i: &Istek) -> Yanit {
        let v: serde_json::Value = match serde_json::from_slice(&i.govde) {
            Ok(v @ serde_json::Value::Object(_)) => v,
            _ => return Yanit::hata(400, "geçersiz istek (JSON nesnesi bekleniyor)"),
        };
        if !no_guvenli(no) {
            return Yanit::hata(400, "fiş no geçersiz");
        }
        let Some(fisli) = v["fisli"].as_bool() else {
            return Yanit::hata(400, "fisli alanı true/false olmalı");
        };
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
        let teslim_alan = v["teslim_alan"].as_str().unwrap_or("");
        let iliski = v["iliski"].as_str().unwrap_or("");
        if let Err(e) = taa_fis::teslim_alan_denetle(teslim_alan, iliski) {
            return Yanit::hata(422, &e);
        }
        // İmza önce çözülür (yanlış biçim 400/413), son 4 hane hakkı boşa harcanmasın.
        let imza: Option<Vec<u8>> = match v["imza_png_base64"].as_str().filter(|s| !s.is_empty()) {
            None => None,
            Some(s) => {
                let b = match base64_coz(s) {
                    Ok(b) => b,
                    Err(e) => return Yanit::hata(400, &e),
                };
                if b.len() > EN_BUYUK_IMZA {
                    return Yanit::hata(413, "imza en çok 300 KB olmalı");
                }
                if !png_gecerli(&b) {
                    return Yanit::hata(400, "imza PNG olmalı");
                }
                Some(b)
            }
        };
        if !fisli {
            if imza.is_none() {
                return Yanit::hata(422, "fişsiz teslimde teslim alanın imzası zorunlu");
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
        // Bağlı fiş sorusu: durum ucuyla aynı (409 bagli_teslim; "bagli" listesiyle yeniden gönderilir).
        let birlikte = match self.bagli_teslim_secimi(d, no, &v) {
            Ok(b) => b,
            Err(y) => return y,
        };
        // İmza dosyası (atomik). Kayıt yazılamazsa dosya geri silinir.
        let mut imza_sha256 = String::new();
        let mut imza_dosyasi = String::new();
        let mut yazilan: Option<PathBuf> = None;
        if let Some(b) = &imza {
            let klasor = self.dizin.join("imzalar");
            let Some(n) =
                (1..=EN_COK_IMZA).find(|n| !klasor.join(format!("{no}-{n}.png")).exists())
            else {
                return Yanit::hata(409, "bu fişe çok fazla imza dosyası yazılmış");
            };
            let ad = format!("{no}-{n}.png");
            let yol = klasor.join(&ad);
            if let Err(e) = std::fs::create_dir_all(&klasor)
                .and_then(|_| taa_dosya::uzerine_yaz_atomik(&yol, b))
            {
                return Yanit::hata(500, &format!("imza kaydedilemedi: {e}"));
            }
            imza_sha256 = sha256_bayt(b);
            imza_dosyasi = format!("imzalar/{ad}");
            yazilan = Some(yol);
        }
        let s = self.simdi();
        let kayit = taa_fis::TeslimKaydi {
            teslim_alan: teslim_alan.into(),
            iliski: iliski.into(),
            fisli,
            dogrulama: if fisli { "fis" } else { "telefon-son4" }.into(),
            imza_sha256,
            imza_dosyasi,
            kim: if c.kisi_id.is_empty() {
                c.ad.clone()
            } else {
                c.kisi_id.clone()
            },
            tarih: tarih_iso(s),
            saat: saat_iso(s),
            ana_fis: String::new(),
        };
        // Teslim kaydı ve Teslim durumu tek defter satırıdır: yarım kalmaz.
        if let Err(e) = d.teslim_et(no, kayit.clone()) {
            if let Some(y) = yazilan {
                let _ = std::fs::remove_file(y);
            }
            return Yanit::hata(422, &e);
        }
        let birlikte = self.birlikte_teslim_et(d, no, &kayit, &birlikte);
        if let Ok(Some(f)) = d.bul(no) {
            // Bulut kaydı yalnız sabit anahtarlar taşır; teslim alan bilgisi gitmez.
            self.takibe_yaz(&f);
        }
        let t = d.bul(no).ok().flatten().and_then(|f| f.teslim_kaydi);
        Yanit::json(
            200,
            json!({
                "tamam": true,
                "birlikte_teslim": birlikte,
                "teslim_kaydi": t.map(|t| json!({
                    "teslim_alan": t.teslim_alan,
                    "iliski": t.iliski,
                    "fisli": t.fisli,
                    "dogrulama": t.dogrulama,
                    "imzali": !t.imza_sha256.is_empty(),
                    "tarih": t.tarih,
                    "saat": t.saat,
                })),
            }),
        )
    }

    /// GET /api/fis/<no>/imza: teslim imzasının PNG'si (özeti kayıtla eşleşmeli).
    /// Yalnız mağaza bilgisayarı ve fişsiz teslim / düzeltme yetkisi olan cihazlar görür (403 yetki_yok).
    pub(crate) fn teslim_imzasi(&self, d: &Defter, no: &str, c: &cihaz::Cihaz) -> Yanit {
        if !imza_gorebilir(self, c) {
            return Yanit::json(
                403,
                json!({
                    "hata": "yetki_yok",
                    "ileti": "Bu cihaz teslim imzalarını göremez. Yönetici, Telefonlar ekranından yetki verebilir.",
                }),
            );
        }
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
        // Kayıttaki dosya adı yalnız imzalar/<no>-<n>.png biçimindeyse okunur; bağlı fişte imza ana fişindir.
        let sahip = if t.ana_fis.is_empty() {
            no
        } else {
            t.ana_fis.as_str()
        };
        if !no_guvenli(sahip) {
            return Yanit::hata(404, "imza yok");
        }
        let Some(ad) = t
            .imza_dosyasi
            .strip_prefix("imzalar/")
            .filter(|a| a.starts_with(&format!("{sahip}-")))
            .and_then(|a| a.strip_suffix(".png").map(|k| (a, k)))
            .filter(|(_, k)| no_guvenli(k))
            .map(|(a, _)| a)
        else {
            return Yanit::hata(404, "imza yok");
        };
        match std::fs::read(self.dizin.join("imzalar").join(ad)) {
            Ok(b) if sabit_esit(sha256_bayt(&b).as_bytes(), t.imza_sha256.as_bytes()) => Yanit {
                kod: 200,
                tur: "image/png",
                govde: b,
            },
            Ok(_) => Yanit::hata(409, "imza dosyası kayıttaki özetle uyuşmuyor"),
            Err(_) => Yanit::hata(404, "imza yok"),
        }
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    /// Deneme kişileri (gerçek kişi değildir).
    const KISILER: &str = concat!(
        r#"{"tur":"ekle","v":{"id":"K-test0001","ad":"Deneme Danışman","rol":"danisman","tarih":"2026-11-01"}}"#,
        "\n",
        r#"{"tur":"ekle","v":{"id":"K-test0002","ad":"Deneme Usta","rol":"usta","tarih":"2026-11-01"}}"#,
        "\n"
    );
    /// Sahte fiş: telefonun son 4 hanesi 4567.
    const FIS: &str = r#"{"satis_danismani_id":"K-test0001","olcu_alan_id":"K-test0002","duzenleyen_id":"K-test0001","musteri":"Deneme Müşteri","telefon":"0532 000 45 67","kalemler":[{"urun":"Pantolon","islem":"paça kısalt"}],"teslim":"2099-01-01","ucret_kurus":50000,"on_odeme_kurus":10000}"#;

    fn uyg() -> Uygulama {
        let d = taa_dosya::gecici_klasor().unwrap();
        std::fs::write(d.join("sube.json"), r#"{"kod":"MRK01"}"#).unwrap();
        std::fs::write(d.join("kisiler.jsonl"), KISILER).unwrap();
        let mut u = Uygulama::yeni(d, None, "http://192.168.1.20:8787".into());
        u.yonetici_kilidi = false;
        u
    }

    fn pc(metot: &str, yol: &str, govde: &str) -> Istek {
        Istek {
            metot: metot.into(),
            yol: yol.into(),
            govde: govde.as_bytes().to_vec(),
            yerel: true,
            masaustu: true,
            ..Default::default()
        }
    }

    fn tel(token: &str, metot: &str, yol: &str, govde: &str) -> Istek {
        Istek {
            metot: metot.into(),
            yol: yol.into(),
            yetki: Some(format!("Bearer {token}")),
            govde: govde.as_bytes().to_vec(),
            ..Default::default()
        }
    }

    fn json_of(y: &Yanit) -> serde_json::Value {
        serde_json::from_slice(&y.govde).unwrap_or_default()
    }

    /// Hazır durumda bir sahte fiş açar.
    fn hazir_fis(u: &Uygulama) -> String {
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

    /// Sahte PNG başı: 8 baytlık imza + IHDR (en x boy) + sahte gövde. Görüntü olarak açılması gerekmez.
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

    /// Sahte imza (base64).
    fn imza_b64() -> String {
        base64_kodla(&sahte_png(400, 150))
    }

    fn base64_kodla(b: &[u8]) -> String {
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
        assert_eq!(u.isle(&pc("POST", &yol, &fissiz("4567", None))).kod, 422);
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
        assert_eq!(t.imza_dosyasi, format!("imzalar/{no}-1.png"));
        // imza geri okunur, özet tutar
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
        // düz metinde "Ad S." biçimi, soyad tam basılmaz
        let m = String::from_utf8(
            u.isle(&pc("GET", &format!("/api/fis/{no}/metin"), ""))
                .govde,
        )
        .unwrap()
        .replace('\n', " ");
        assert!(
            m.contains("Teslim alan: Deneme K. (yakını) - fişsiz, imzalı"),
            "{m}"
        );
        assert!(!m.contains("Yakın Kişi"));
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
        // yetki verilince telefondan fişsiz teslim olur, imzayı görür
        u.cihazlar()
            .yetki_ata(&c.id, "fissiz-teslim", true, "2026-11-26")
            .unwrap();
        let y = u.isle(&tel(token, "POST", &yol, &fissiz("4567", Some(&imza))));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let imza_yolu = format!("/api/fis/{no}/imza");
        assert_eq!(u.isle(&tel(token, "GET", &imza_yolu, "")).kod, 200);
        // yetki geri alınınca: imza 403, yeni fişte fişsiz teslim 403
        u.cihazlar()
            .yetki_ata(&c.id, "fissiz-teslim", false, "2026-11-26")
            .unwrap();
        assert_eq!(u.isle(&tel(token, "GET", &imza_yolu, "")).kod, 403);
        let no2 = hazir_fis(&u);
        let y = u.isle(&tel(
            token,
            "POST",
            &format!("/api/fis/{no2}/teslim-al"),
            &fissiz("4567", Some(&imza)),
        ));
        assert_eq!(y.kod, 403);
        // düzeltme yetkisi olan telefon da imzayı görür
        u.cihazlar()
            .yetki_ata(&c.id, "duzelt", true, "2026-11-26")
            .unwrap();
        assert_eq!(u.isle(&tel(token, "GET", &imza_yolu, "")).kod, 200);
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
        // dosya değiştirilirse özet tutmaz: 409
        let dosya = u.dizin.join("imzalar").join(format!("{no}-1.png"));
        std::fs::write(&dosya, sahte_png(401, 150)).unwrap();
        assert_eq!(u.isle(&pc("GET", &imza_yolu, "")).kod, 409);
        // kayıttaki dosya adı kurcalanırsa (yol kaçışı) okunmaz: 404
        let defter = std::fs::read_to_string(u.fis_yolu()).unwrap();
        let eski = format!("imzalar/{no}-1.png");
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
            r#"{"fisli":true,"teslim_alan":"Deneme Müşteri","iliski":"kendisi","telefon_son4":"9999"}"#,
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
        assert!(!u.dizin.join("imzalar").join(format!("{a}-1.png")).exists());
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
        assert!(m.contains("Teslim alan: Deneme K. (yakını)"), "{m}");
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
}
