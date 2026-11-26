//! Teslim alma ucu: ürünü teslim alan kişinin adı, ilişkisi ve imzası; müşteri fişiyle gelmediyse
//! (fişsiz teslim) telefonun son 4 hanesiyle doğrulama. Yusuf: "ürünü teslim alan kişi için imza alıyoruz,
//! bazen müşteri fişiyle gelmiyor".
//! - Fişsiz teslim "fissiz-teslim" yetkisi ister (varsayılan yalnız mağaza bilgisayarı).
//! - Son 4 hane hiçbir yere yazılmaz; kayıtta yalnız doğrulama yöntemi ("telefon-son4") kalır.
//! - Fiş başına 5 yanlışta 15 dk kilit (bellekte; yeniden başlatınca sıfırlanır).
//! - İmza PNG'si `imzalar/<no>-<n>.png` olarak atomik yazılır, SHA-256 özeti defterdeki kayda girer.
//! - Teslim alan bilgisi müşteri takip (bulut) kaydına GİTMEZ; bulut kaydı yalnız sabit anahtarlar taşır.
//! - Karekodla onay (2026-11-26, bkz. teslim_onay.rs): `onay_belirteci` verilirse ve müşteri kendi telefonundan
//!   onayladıysa teslim "qr" (fişsizde "qr+telefon-son4") doğrulamasıyla yapılır, imza PNG'si gerekmez. Karekod
//!   yoksa ekran imzası "telefonu olmayan müşteri" yedeği olarak kalır. Fişsiz teslimde ikisinden biri zorunlu.
//! - Teslim alanın adı, telefonu, cihazı (user-agent) ve IP'si deftere yalnız ŞİFRELİ yazılır (teslim_sifre.rs);
//!   `teslim_alan` düz metin alanı yeni kayıtlarda boştur.

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
    /// dosyası ve özeti) `ana_fis` atfıyla yazılır, müşteri takibine yazılır. (teslim edilenler, atlananlar)
    /// döndürür; atlanan (şifrelenemeyen ya da yazılamayan) bağlı fiş yanıtta ayrıca bildirilir.
    /// Şifreli blok fiş no'ya bağlı olduğundan (aad) her bağlı fiş için yeniden şifrelenir.
    pub(crate) fn birlikte_teslim_et(
        &self,
        d: &Defter,
        ana: &str,
        kayit: &taa_fis::TeslimKaydi,
        duz: &teslim_sifre::TeslimDuz,
        birlikte: &[String],
    ) -> (Vec<String>, Vec<String>) {
        let mut edilen = Vec::new();
        let mut atlanan = Vec::new();
        for n in birlikte {
            let Ok(sifreli) = teslim_sifre::sifrele(&self.dizin, n, duz) else {
                eprintln!("Bağlı fiş teslim edilemedi ({n}): teslim bilgisi şifrelenemedi");
                atlanan.push(n.clone());
                continue;
            };
            let k = taa_fis::TeslimKaydi {
                ana_fis: ana.to_string(),
                sifreli: Some(sifreli),
                ..kayit.clone()
            };
            match d.teslim_et(n, k) {
                Ok(()) => {
                    edilen.push(n.clone());
                    if let Ok(Some(f)) = d.bul(n) {
                        self.takibe_yaz(&f);
                    }
                }
                Err(e) => {
                    eprintln!("Bağlı fiş teslim edilemedi ({n}): {e}");
                    atlanan.push(n.clone());
                }
            }
        }
        (edilen, atlanan)
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
        // İmza önce çözülür (yanlış biçim 400/413), son 4 hane hakkı boşa harcanmasın. Karekod onayında imza
        // kullanılmaz (gönderilse de yok sayılır).
        let imza: Option<Vec<u8>> = match v["imza_png_base64"]
            .as_str()
            .filter(|s| !s.is_empty() && onay.is_none())
        {
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
            if imza.is_none() && onay.is_none() {
                return Yanit::hata(
                    422,
                    "fişsiz teslimde karekod onayı ya da teslim alanın imzası zorunlu",
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
                    let _ = std::fs::remove_file(y);
                }
                return Yanit::hata(500, &format!("teslim bilgisi şifrelenemedi: {e}"));
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
            kim: if c.kisi_id.is_empty() {
                c.ad.clone()
            } else {
                c.kisi_id.clone()
            },
            tarih: tarih_iso(s),
            saat: saat_iso(s),
            ana_fis: String::new(),
            sifreli: Some(sifreli),
            gorunen: None,
        };
        // Teslim kaydı ve Teslim durumu tek defter satırıdır: yarım kalmaz.
        if let Err(e) = d.teslim_et(no, kayit.clone()) {
            if let Some(y) = yazilan {
                let _ = std::fs::remove_file(y);
            }
            return Yanit::hata(422, &e);
        }
        if let Some(t) = &belirtec {
            self.qr_onayi_tuket(t);
        }
        let (birlikte, atlanan) = self.birlikte_teslim_et(d, no, &kayit, &duz, &birlikte);
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
                // Seçildiği hâlde teslim kaydı yazılamayan bağlı fişler (boşsa sorun yok).
                "birlikte_atlanan": atlanan,
                // Teslimi yapan cihaz adı zaten gördü; tam telefon yine gönderilmez (maskeli).
                "teslim_kaydi": t.map(|t| json!({
                    "ad": duz.ad,
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
    fn teslim_coz(&self, no: &str, t: &taa_fis::TeslimKaydi) -> Option<teslim_sifre::TeslimDuz> {
        let s = t.sifreli.as_ref()?;
        match teslim_sifre::coz(&self.dizin, no, s) {
            Ok(d) => Some(d),
            Err(e) => {
                eprintln!("Teslim kaydı çözülemedi ({no}): {e}");
                None
            }
        }
    }

    /// Basım/düz metin için: şifreli kayıttan adı çözüp yalnız bellekteki görünüme koyar.
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

    /// Fiş ayrıntısı için çözülmüş görünüm: ad, maskeli telefon, cihaz özeti, zaman, yöntem (tam telefon yok).
    pub(crate) fn teslim_goruntusu(
        &self,
        no: &str,
        t: &taa_fis::TeslimKaydi,
    ) -> Option<serde_json::Value> {
        let d = self.teslim_coz(no, t)?;
        Some(json!({
            "ad": d.ad,
            "teslim_alan": d.ad,
            "tel_maskeli": teslim_sifre::tel_maskele(&d.tel),
            "cihaz": teslim_sifre::cihaz_ozeti(&d.cihaz),
            "zaman": d.zaman,
            "yontem": d.yontem,
        }))
    }

    /// Yönetici görüntülemesi günlüğü: kişisel veri İÇERMEZ (yalnız olay, fiş no / sonuç sayısı, zaman).
    fn teslim_gunlugu(&self, satir: serde_json::Value) {
        let s = self.simdi();
        let mut v = satir;
        v["tarih"] = json!(tarih_iso(s));
        v["saat"] = json!(saat_iso(s));
        let _ = taa_dosya::satir_ekle(&self.dizin.join("teslim-ayrinti.jsonl"), &v.to_string());
        eprintln!("Teslim kaydı yönetici görüntülemesi: {v}");
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
            return Yanit::json(
                200,
                json!({ "sifreli": false, "teslim_alan": t.teslim_alan, "iliski": t.iliski, "tarih": t.tarih, "saat": t.saat }),
            );
        };
        let duz = match teslim_sifre::coz(&self.dizin, no, s) {
            Ok(x) => x,
            Err(e) => return Yanit::hata(500, &e),
        };
        self.teslim_gunlugu(json!({ "olay": "teslim_ayrintisi", "fis": no }));
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
        self.teslim_gunlugu(json!({ "olay": "teslim_ara", "ad_ile": !ad.is_empty(), "tel_ile": !tel.is_empty(), "sonuc": bulunan.len() }));
        Yanit::json(200, json!({ "sonuc": bulunan }))
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

    // ---------- Karekodla teslim onayı (sahte bulut, sahte kişi) ----------

    /// Bulut sözleşmesini taklit eden sahte bulut: imzaları doğrular, kayıtları bellekte tutar.
    struct SahteBulut {
        genel: ed25519_dalek::VerifyingKey,
        kayit: Mutex<BTreeMap<String, serde_json::Value>>,
        dusuk: Mutex<Vec<String>>,
        istekler: Mutex<Vec<String>>,
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
        fn musteri_onaylar(&self, taban: &str, t: &str, ad: &str, tel: &str) {
            let mut k = self.kayit.lock().unwrap();
            let r = k.get_mut(&format!("{taban}|{t}")).expect("bulut kaydı");
            // Onay anı: karekod açıldıktan 1 sn sonra (sunucu, zamanın ts..sil aralığında olduğunu denetler).
            let zaman = r["sil"].as_u64().unwrap() - crate::teslim_onay::ONAY_OMRU_SN + 1;
            let duz = json!({
                "ad": ad, "tel": tel,
                "ua": "Mozilla/5.0 (Linux; Android 14; SM-SAHTE) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Mobile Safari/537.36",
                "ip": "198.51.100.23", "zaman": zaman, "dil": "tr",
            })
            .to_string();
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
                    let a = format!("{taban}|{t}");
                    if k.contains_key(&a) {
                        return Ok((409, String::new()));
                    }
                    k.insert(a, json!({ "pk": v["pk"], "durum": "bekliyor", "sil": sil }));
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

    const A: &str = "https://a.example";
    const B: &str = "https://b.example";
    /// Sahte teslim alan (gerçek kişi değildir).
    const QR_AD: &str = "Zeynep Örnekoğlu";
    const QR_TEL: &str = "+905559998877";

    fn qr_uyg() -> (Uygulama, std::sync::Arc<SahteBulut>) {
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

    fn tum_dosyalarda(k: &Path, aranan: &[&str], bulunan: &mut Vec<String>) {
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
        let govde = json!({ "fisli": true, "onay_belirteci": t }).to_string();
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
            json!({ "fisli": true, "onay_belirteci": t, "iliski": "kendisi" }).to_string();
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-al"), &g_kendisi))
                .kod,
            422
        );
        let govde = json!({ "fisli": true, "onay_belirteci": t, "iliski": "yakini" }).to_string();
        // teslim: imza gerekmez, doğrulama "qr"
        let y = u.isle(&pc("POST", &format!("/api/fis/{no}/teslim-al"), &govde));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j = json_of(&y);
        assert_eq!(j["teslim_kaydi"]["dogrulama"], "qr");
        assert_eq!(j["teslim_kaydi"]["imzali"], false);
        assert_eq!(j["teslim_kaydi"]["ad"], QR_AD);
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
        // fiş ayrıntısı: mağaza PC'si çözülmüş görünümü alır (tam telefon yok)
        let y = u.isle(&pc("GET", &format!("/api/fis/{no}"), ""));
        let j = json_of(&y);
        assert_eq!(j["teslim_kaydi"]["ad"], QR_AD);
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
                "Teslim alan: Zeynep Ö. - QR onaylı, {gun} {}",
                &tk.saat[..5]
            )),
            "{m}"
        );
        assert!(!m.contains("8877") && !m.contains("Örnekoğlu"));
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
        let (t, _) = qr_ac(&u, &no);
        bulut.musteri_onaylar(A, &t, QR_AD, QR_TEL);
        assert_eq!(
            json_of(&u.isle(&pc("GET", &format!("/api/fis/{no}/teslim-qr/{t}"), "")))["durum"],
            "onaylandi"
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
        // doğru son 4: imzasız fişsiz teslim, doğrulama "qr+telefon-son4"
        let y = u.isle(&pc("POST", &yol, &g("4567")));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(json_of(&y)["teslim_kaydi"]["dogrulama"], "qr+telefon-son4");
        assert!(
            !u.dizin.join("imzalar").exists(),
            "QR teslimde imza dosyası yazılmaz"
        );
        let f = Defter { yol: &u.fis_yolu() }.bul(&no).unwrap().unwrap();
        let tk = f.teslim_kaydi.unwrap();
        assert!(!tk.fisli && tk.iliski == "yakini" && tk.imza_sha256.is_empty());
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
            "{}",
        ));
        assert_eq!(y.kod, 200);
        let t2 = json_of(&y)["t"].as_str().unwrap().to_string();
        bulut.musteri_onaylar(A, &t2, QR_AD, QR_TEL);
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
        let g = json!({ "fisli": true, "onay_belirteci": t }).to_string();
        let y = u.isle(&tel(token, "POST", &format!("/api/fis/{a}/teslim-al"), &g));
        assert_eq!(y.kod, 422);
        // başka fişin teslim-al'ında kullanılamaz
        assert_eq!(
            u.isle(&pc("POST", &format!("/api/fis/{b}/teslim-al"), &g))
                .kod,
            422
        );
        let g = json!({ "fisli": true, "onay_belirteci": bilinmeyen }).to_string();
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
        let g3 = json!({ "fisli": true, "onay_belirteci": t3 }).to_string();
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
        let g = json!({ "fisli": true, "onay_belirteci": t }).to_string();
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
        let g = json!({ "fisli": true, "onay_belirteci": t, "iliski": "diger" }).to_string();
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
        let g = json!({ "fisli": true, "onay_belirteci": t }).to_string();
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
        let g = json!({ "fisli": true, "onay_belirteci": t, "iliski": "yakini" }).to_string();
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
