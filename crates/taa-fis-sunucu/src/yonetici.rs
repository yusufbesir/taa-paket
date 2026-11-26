//! Yönetici kilidi: Telefonlar, Personel ve Usta puanı ekranları yönetici girişi ister.
//! Giriş (koordinatör kararı 2026-11-26): TAA ortak giriş kullanıcısı yönetici rolündeyse, o kişinin şifresi ortak
//! girişin `giris` akışıyla yeniden doğrulanarak; oturum o kişiye bağlıdır (ortak girişte kişi değişince geçersiz).
//! Geri uyum: yönetici PIN'i (bu sunucuda saklanmaz ve yazılmaz; TAA Çatı doğrular, bkz. `cati_pin`; günlükte "PC (PIN)").
//! Doğrulanınca rastgele bir oturum belirteci verilir (yalnız SHA256 özeti bellekte tutulur); 10 dakika işlem olmazsa
//! oturum kapanır. Tüm yönetim uçları ayrıca yalnız bu bilgisayardan çalışır.

use super::*;
use cati_pin::PinSonuc;

/// Ortak giriş kapalıyken / erişilemezken PIN ile açılan yönetici oturumunun günlükteki adı. PIN kişiyi belirlemez;
/// ortak girişte o an kim olursa olsun günlüğe yalnız bu yazılır (başkasının adı PIN'le günlüğe taşınamaz).
pub const PIN_KIMI: &str = "PC (PIN)";
/// Ortak giriş oturumunda yönetici sayılan rol (TAA Çatı rol adı). `yonetim-ayar.json` içindeki `yonetici_rolleri`
/// ile değiştirilebilir (bkz. `eski_fis_gecis::YonetimAyari`); boşsa bu kullanılır.
pub const YONETICI_ROLU: &str = "yonetici";
/// Bellekteki ortak giriş kullanıcısının arka planda taa-kimlik'ten yeniden sorulma aralığı (sn; main.rs).
pub const ORTAK_YENILEME_SN: u64 = 45;

/// Bir yönetici oturumu. Belirteç yalnız SHA256 özetiyle bellekte durur.
#[derive(Clone, Debug)]
pub(crate) struct YoneticiOturum {
    pub(crate) ozet: String,
    pub(crate) son: u64,
    /// Günlüklerdeki "kim": ortak giriş kullanıcısı (`OrtakKullanici::gunluk_adi`) ya da `PIN_KIMI`.
    pub(crate) kim: String,
    /// "ortak-giris" ya da "pin".
    pub(crate) kaynak: &'static str,
    /// Ortak girişle açıldıysa oturumu açan ortak kullanıcı. Ortak girişte bu kişi çıkınca ya da başka biri girince
    /// oturum geçersizdir (bkz. `Uygulama::oturum_gecerli`). PIN oturumunda boş.
    pub(crate) kullanici: String,
}

/// Bu bilgisayarda TAA ortak girişle giren kullanıcı (bellekte; `/api/giris/*` yanıtlarından ve arka planda
/// `ORTAK_YENILEME_SN`'de bir taa-kimlik'ten).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OrtakKullanici {
    pub(crate) kullanici: String,
    pub(crate) roller: Vec<String>,
}

/// Bellekteki ortak giriş kullanıcısı; ortak giriş uçlarını ayrı iş parçacığında yanıtlayan main.rs de günceller.
pub type OrtakTutucu = std::sync::Arc<Mutex<Option<OrtakKullanici>>>;

impl OrtakKullanici {
    /// `ortak_giris` oturum özetinden ({kullanici, roller, ...}); geçersizse `None`.
    pub(crate) fn ozetten(o: &serde_json::Value) -> Option<Self> {
        let k = o["kullanici"].as_str()?.trim();
        if k.is_empty() || k.chars().count() > 64 || k.chars().any(char::is_control) {
            return None;
        }
        Some(OrtakKullanici {
            kullanici: k.to_string(),
            roller: o["roller"]
                .as_array()
                .map(|l| {
                    l.iter()
                        .filter_map(|r| r.as_str().map(str::to_string))
                        .collect()
                })
                .unwrap_or_default(),
        })
    }

    /// Rollerinden biri yönetici rolleri listesinde mi?
    pub(crate) fn yonetici(&self, roller: &[String]) -> bool {
        self.roller.iter().any(|r| roller.contains(r))
    }

    /// Günlükteki adı: kullanıcı adı; "PC", "PC (…)" ya da kişi/cihaz kimliğine benzeyen ("K-", "U-", "T-") bir ad
    /// günlükte başka şeyle karışmasın diye "ortak:" önekiyle yazılır.
    pub(crate) fn gunluk_adi(&self) -> String {
        let k = self.kullanici.as_str();
        let cakisir = k.eq_ignore_ascii_case(crate::PC_ID)
            || k.to_uppercase().starts_with("PC (")
            || ["K-", "U-", "T-"].iter().any(|o| k.starts_with(o))
            || k.starts_with("ortak:");
        if cakisir {
            format!("ortak:{k}")
        } else {
            k.to_string()
        }
    }
}

/// `/api/giris/<islem>` yanıtından bellekteki ortak giriş kullanıcısını günceller (çıkışta siler). Kişi değiştiyse
/// `true` (ortak girişle açılmış yönetici oturumları bir sonraki istekte geçersiz sayılır).
pub fn ortak_tutucu_guncelle(t: &OrtakTutucu, islem: &str, v: &serde_json::Value) -> bool {
    let yeni = match islem {
        "giris" if v["tamam"] == true => OrtakKullanici::ozetten(&v["oturum"]),
        "giris" => return false, // hatalı deneme süren oturumu değiştirmez
        "baslangic" => OrtakKullanici::ozetten(&v["oturum"]),
        "durum" => OrtakKullanici::ozetten(v),
        "cikis" => None,
        _ => return false,
    };
    ortak_tutucu_yaz(t, yeni)
}

fn ortak_tutucu_yaz(t: &OrtakTutucu, yeni: Option<OrtakKullanici>) -> bool {
    match t.lock() {
        Ok(mut o) => {
            let degisti = *o != yeni;
            *o = yeni;
            degisti
        }
        Err(_) => false,
    }
}

/// Arka plan yenilemesi (main.rs, `ORTAK_YENILEME_SN`'de bir): süren ortak oturumu taa-kimlik'ten yeniden sorar. Kişi
/// başka bir TAA programından çıktıysa günlüklere en çok bu süre kadar eski kişi yazılabilir.
pub fn ortak_yenile(t: &OrtakTutucu, bul: ortak_giris::KimlikBulucu) -> bool {
    let k = ortak_giris::oturum(bul, cati_pin::PROGRAM)
        .as_ref()
        .and_then(OrtakKullanici::ozetten);
    ortak_tutucu_yaz(t, k)
}

impl Uygulama {
    /// Ortak giriş kullanıcısını tutan paylaşılan kutu (main.rs'deki ortak giriş iş parçacığı ve yenileyici için).
    pub fn ortak_tutucu(&self) -> OrtakTutucu {
        self.ortak_kullanici.clone()
    }

    /// Ortak girişte yönetici sayılan roller (`yonetim-ayar.json` `yonetici_rolleri`, boşsa "yonetici").
    pub(crate) fn yonetici_rolleri(&self) -> Vec<String> {
        let r = eski_fis_gecis::YonetimAyari::oku(&self.dizin).yonetici_rolleri;
        if r.is_empty() {
            vec![YONETICI_ROLU.to_string()]
        } else {
            r
        }
    }

    /// Oturum hâlâ geçerli mi? Boşta süre aşılmamış olmalı; ortak girişle açıldıysa ortak girişte AYNI kişi hâlâ
    /// yönetici rolüyle oturuyor olmalı (kişi çıktıysa ya da başkası girdiyse oturum devralınamaz).
    fn oturum_gecerli(
        o: &YoneticiOturum,
        simdi: u64,
        ortak: &Option<OrtakKullanici>,
        roller: &[String],
    ) -> bool {
        if simdi.saturating_sub(o.son) >= YONETICI_BOSTA_SN {
            return false;
        }
        match o.kaynak {
            "ortak-giris" => ortak
                .as_ref()
                .is_some_and(|k| k.kullanici == o.kullanici && k.yonetici(roller)),
            _ => true,
        }
    }

    /// Belirteçli geçerli oturum (süre yenilenmeden). Kayıt/günlük için "kim" okunurken kullanılır.
    fn yonetici_oturumu(&self, i: &Istek) -> Option<YoneticiOturum> {
        let t = i.yonetici.as_deref().filter(|t| !t.is_empty())?;
        let h = cihaz::sha256_hex(t);
        let simdi = self.simdi();
        let ortak = self.ortak_kullanici();
        let roller = self.yonetici_rolleri();
        let mut v = self.yonetici_oturum.lock().ok()?;
        v.retain(|o| Self::oturum_gecerli(o, simdi, &ortak, &roller));
        v.iter().find(|o| o.ozet == h).cloned()
    }

    /// Belirteç geçerliyse son işlem zamanını yeniler.
    fn yonetici_gecerli(&self, i: &Istek) -> bool {
        let Some(t) = i.yonetici.as_deref().filter(|t| !t.is_empty()) else {
            return false;
        };
        let h = cihaz::sha256_hex(t);
        let simdi = self.simdi();
        let ortak = self.ortak_kullanici();
        let roller = self.yonetici_rolleri();
        let Ok(mut v) = self.yonetici_oturum.lock() else {
            return false;
        };
        v.retain(|o| Self::oturum_gecerli(o, simdi, &ortak, &roller));
        match v.iter_mut().find(|o| o.ozet == h) {
            Some(o) => {
                o.son = simdi;
                true
            }
            None => false,
        }
    }

    /// Ortak girişle açılmış ama artık geçersiz (kişi çıktı / değişti) yönetici oturumlarını siler.
    fn yonetici_oturumlarini_temizle(&self) {
        let simdi = self.simdi();
        let ortak = self.ortak_kullanici();
        let roller = self.yonetici_rolleri();
        if let Ok(mut v) = self.yonetici_oturum.lock() {
            v.retain(|o| Self::oturum_gecerli(o, simdi, &ortak, &roller));
        }
    }

    /// Bellekteki ortak giriş kullanıcısı.
    pub(crate) fn ortak_kullanici(&self) -> Option<OrtakKullanici> {
        self.ortak_kullanici.lock().ok().and_then(|o| o.clone())
    }

    /// `/api/giris/<islem>` yanıtından bellekteki ortak giriş kullanıcısını günceller (çıkışta siler); kişi
    /// değiştiyse ortak girişle açılmış yönetici oturumları hemen kapanır.
    pub(crate) fn ortak_kullanici_guncelle(&self, islem: &str, v: &serde_json::Value) {
        if ortak_tutucu_guncelle(&self.ortak_kullanici, islem, v) {
            self.yonetici_oturumlarini_temizle();
        }
    }

    /// Mağaza bilgisayarında oturumdaki kişi: geçerli yönetici oturumunun "kim"i, yoksa ortak giriş kullanıcısı.
    /// İkisi de yoksa `None` (günlükte "PC" kalır).
    pub(crate) fn pc_kimi(&self, i: &Istek) -> Option<String> {
        if let Some(o) = self.yonetici_oturumu(i) {
            return Some(o.kim);
        }
        self.ortak_kullanici().map(|o| o.gunluk_adi())
    }

    /// Günlükteki "kim"in kaynağı (ayrı alan): "ortak-giris" (ortak girişle açılmış yönetici oturumu), "pin"
    /// (PIN'le açılmış yönetici oturumu), "ortak-oturum" (yönetici oturumu yok, ortak giriş kullanıcısı) ya da "yok".
    pub(crate) fn kim_kaynagi(&self, i: &Istek) -> &'static str {
        if let Some(o) = self.yonetici_oturumu(i) {
            return o.kaynak;
        }
        if self.ortak_kullanici().is_some() {
            "ortak-oturum"
        } else {
            "yok"
        }
    }

    /// Yönetim olaylarının (yetki atama, teslim ayrıntısı, ayar) günlüğündeki "kim".
    pub(crate) fn yonetici_kimi(&self, i: &Istek) -> String {
        self.pc_kimi(i).unwrap_or_else(|| crate::PC_ID.to_string())
    }

    /// Yönetim uçlarının kapısı: yalnız bu bilgisayar ve (kilit açıksa) geçerli yönetici oturumu.
    pub(crate) fn yonetim_izni(&self, i: &Istek) -> Option<Yanit> {
        if !i.yerel {
            return Some(Yanit::hata(403, "yalnız bu bilgisayardan"));
        }
        if !self.yonetici_kilidi || self.yonetici_gecerli(i) {
            return None;
        }
        Some(Yanit::json(
            401,
            json!({ "hata": "yonetici_gerekli", "ileti": "Yönetici girişi gerekli" }),
        ))
    }

    /// Yeni yönetici oturumu açar, girişi günlüğe yazar ve yanıtı döndürür.
    fn yonetici_oturumu_ac(&self, kim: String, kaynak: &'static str, kullanici: String) -> Yanit {
        let t = match cihaz::yeni_token() {
            Ok(t) => t,
            Err(e) => return Yanit::hata(500, &e),
        };
        let simdi = self.simdi();
        if let Ok(mut o) = self.yonetici_oturum.lock() {
            o.retain(|x| simdi.saturating_sub(x.son) < YONETICI_BOSTA_SN);
            o.push(YoneticiOturum {
                ozet: cihaz::sha256_hex(&t),
                son: simdi,
                kim: kim.clone(),
                kaynak,
                kullanici,
            });
        }
        let satir = json!({"olay": "yonetici_giris", "kim": kim, "kaynak": kaynak, "tarih": tarih_iso(simdi), "saat": saat_iso(simdi)}).to_string();
        let _ = taa_dosya::satir_ekle(&self.dizin.join("eslestirme.jsonl"), &satir);
        Yanit::json(
            200,
            json!({ "gecerli": true, "belirlendi": true, "belirtec": t, "bosta_sn": YONETICI_BOSTA_SN, "kim": kim, "kaynak": kaynak }),
        )
    }

    /// Reddedilen ortak giriş denemesini yönetim günlüğüne yazar (şifre yazılmaz).
    fn yonetici_giris_red(&self, kullanici: &str, neden: &str) {
        let s = self.simdi();
        let satir = json!({"olay": "yonetici_giris_red", "kaynak": "ortak-giris", "kullanici": kullanici, "neden": neden, "tarih": tarih_iso(s), "saat": saat_iso(s)}).to_string();
        let _ = taa_dosya::satir_ekle(&self.dizin.join("eslestirme.jsonl"), &satir);
    }

    /// POST /api/yonetici/giris {ortak:true, parola} ya da {pin}.
    /// - `ortak`: bu bilgisayardaki TAA ortak giriş oturumu (taa-kimlik) sorulur; kullanıcı TAA Çatı'da yönetici
    ///   rolündeyse (`yonetici_rolleri`) KİŞİ YENİDEN DOĞRULANIR: o kullanıcının şifresi istenir ve ortak girişin
    ///   "giris" akışıyla (aynı kullanıcı için taa-kimlik) denetlenir. Doğruysa oturum o kişiyle açılır (günlüklerde kişi
    ///   yazılır). Ortak girişte biri oturuyor diye bilgisayarın başındaki herkes yönetici olamaz. Oturum yoksa 409
    ///   `ortak_oturum_yok`, rol yoksa 403 `yonetici_degil`, şifre yoksa 400 `parola_gerekli`, şifre yanlışsa 401
    ///   (ekran PIN'e düşebilir). Ortak girişte bu kişi çıkarsa ya da başka biri girerse oturum kendiliğinden kapanır.
    /// - `pin` (geri uyum; ortak giriş kapalı ya da erişilemezken): PIN TAA Çatı'ya gider; günlükte yalnız "PC (PIN)".
    pub(crate) fn yonetici_giris(&self, i: &Istek) -> Yanit {
        if !i.yerel || !i.masaustu {
            return Yanit::hata(403, "yalnız bu bilgisayardaki programdan");
        }
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        if v["ortak"] == true {
            let o = ortak_giris::oturum(self.kimlik_bulucu, cati_pin::PROGRAM);
            let k = o.as_ref().and_then(OrtakKullanici::ozetten);
            if ortak_tutucu_yaz(&self.ortak_kullanici, k.clone()) {
                self.yonetici_oturumlarini_temizle();
            }
            let roller = self.yonetici_rolleri();
            let k = match k {
                None => {
                    return Yanit::json(
                        409,
                        json!({ "hata": "ortak_oturum_yok", "ileti": "Bu bilgisayarda TAA ortak girişle açılmış bir oturum yok. Ortak girişle girin ya da yönetici PIN'ini kullanın." }),
                    )
                }
                Some(k) if !k.yonetici(&roller) => {
                    return Yanit::json(
                        403,
                        json!({ "hata": "yonetici_degil", "kullanici": k.kullanici, "ileti": format!("{} yönetici rolünde değil. Yönetici hesabıyla ortak girişe girin ya da yönetici PIN'ini kullanın.", k.kullanici) }),
                    )
                }
                Some(k) => k,
            };
            let parola = v["parola"].as_str().unwrap_or("");
            if parola.is_empty() {
                return Yanit::json(
                    400,
                    json!({ "hata": "parola_gerekli", "kullanici": k.kullanici, "ileti": format!("Kimliği doğrulamak için {} hesabının şifresini yazın.", k.kullanici) }),
                );
            }
            // Aynı kullanıcı için ortak giriş "giris" akışı (taa-kimlik): şifre yalnız aracın standart girdisine gider.
            let govde = json!({ "kullanici": k.kullanici, "parola": parola }).to_string();
            let r = ortak_giris::yanitla(
                self.kimlik_bulucu,
                cati_pin::PROGRAM,
                "giris",
                govde.as_bytes(),
            )
            .unwrap_or_default();
            if r["tamam"] != true {
                let durum = r["durum"].as_str().unwrap_or("bilinmiyor").to_string();
                self.yonetici_giris_red(&k.kullanici, &durum);
                return Yanit::json(
                    401,
                    json!({ "hata": "parola_hatali", "durum": durum, "ileti": r["ileti"].as_str().unwrap_or("Şifre doğrulanamadı.") }),
                );
            }
            let k2 = OrtakKullanici::ozetten(&r["oturum"]);
            if ortak_tutucu_yaz(&self.ortak_kullanici, k2.clone()) {
                self.yonetici_oturumlarini_temizle();
            }
            return match k2 {
                Some(k2) if k2.kullanici == k.kullanici && k2.yonetici(&roller) => {
                    self.yonetici_oturumu_ac(k2.gunluk_adi(), "ortak-giris", k2.kullanici)
                }
                _ => {
                    self.yonetici_giris_red(&k.kullanici, "rol_ya_da_kisi_degisti");
                    Yanit::json(
                        403,
                        json!({ "hata": "yonetici_degil", "ileti": "Doğrulanan hesap yönetici rolünde değil." }),
                    )
                }
            };
        }
        let pin = v["pin"].as_str().unwrap_or("");
        if !(4..=8).contains(&pin.len()) || !pin.bytes().all(|b| b.is_ascii_digit()) {
            return Yanit::hata(400, "PIN 4-8 rakam olmalı");
        }
        match self.pin.dogrula(pin) {
            // PIN kişiyi belirlemez: günlükte yalnız "PC (PIN)" (ortak girişteki kişinin adı eklenmez).
            PinSonuc::Gecerli => {
                self.yonetici_oturumu_ac(PIN_KIMI.to_string(), "pin", String::new())
            }
            PinSonuc::Yanlis => Yanit::json(
                200,
                json!({ "gecerli": false, "belirlendi": true, "ileti": "PIN yanlış" }),
            ),
            PinSonuc::Belirlenmemis => Yanit::json(
                200,
                json!({ "gecerli": false, "belirlendi": false, "ileti": cati_pin::BELIRLENMEMIS }),
            ),
            PinSonuc::Kilitli { ileti, kalan_sn } => Yanit::json(
                423,
                json!({ "hata": "kilitli", "ileti": ileti, "kalan_sn": kalan_sn }),
            ),
            PinSonuc::Ulasilamadi(ayrinti) => Yanit::json(
                503,
                json!({ "hata": "cati_ulasilamiyor", "ileti": cati_pin::ULASILAMIYOR, "ayrinti": ayrinti }),
            ),
        }
    }

    /// POST /api/yonetici/cikis: oturumu kapatır.
    pub(crate) fn yonetici_cikis(&self, i: &Istek) -> Yanit {
        if let Some(t) = i.yonetici.as_deref() {
            let h = cihaz::sha256_hex(t);
            if let Ok(mut o) = self.yonetici_oturum.lock() {
                o.retain(|x| x.ozet != h);
            }
        }
        Yanit::json(200, json!({ "tamam": true }))
    }

    /// GET /api/yonetici/durum: oturum açık mı? (süreyi yeniler)
    pub(crate) fn yonetici_durum(&self, i: &Istek) -> Yanit {
        if !i.yerel {
            return Yanit::hata(403, "yalnız bu bilgisayardan");
        }
        let acik = !self.yonetici_kilidi || self.yonetici_gecerli(i);
        let oturum = self.yonetici_oturumu(i);
        let ortak = self.ortak_kullanici();
        let roller = self.yonetici_rolleri();
        Yanit::json(
            200,
            json!({
                "acik": acik,
                "kilit": self.yonetici_kilidi,
                "bosta_sn": YONETICI_BOSTA_SN,
                // Oturumdaki kişi (günlüklere yazılan) ve ortak giriş kullanıcısı (giriş ekranı "Ortak girişle aç"
                // düğmesini buna göre gösterir).
                "kim": oturum.as_ref().map(|o| o.kim.clone()),
                "kaynak": oturum.as_ref().map(|o| o.kaynak),
                "ortak": ortak.map(|o| json!({ "kullanici": o.kullanici, "yonetici": o.yonetici(&roller) })),
            }),
        )
    }

    /// GET /api/yonetim/eslestirme?kisi=K-…: tek kullanımlık 6 haneli kod + karekod. `kisi` verilirse telefon
    /// o kişinin kişisel telefonu olur (kullanıcı girişi: fişi düzenleyen her zaman o kişi).
    pub(crate) fn eslestirme_kodu(&self, i: &Istek, sorgu: &str) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        if let Some(k) = kapali("telefon-eslestirme") {
            return k;
        }
        let kisi_id = sorgu
            .split('&')
            .find_map(|p| p.strip_prefix("kisi="))
            .unwrap_or("")
            .trim();
        if !kisi_id.is_empty() {
            let sube = self.sube_kodu().unwrap_or_default();
            if !kisi::secilebilir(&self.dizin, &sube)
                .iter()
                .any(|k| k.id == kisi_id)
            {
                return Yanit::hata(422, "kişi kayıtta yok ya da kapatılmış");
            }
        }
        let kod = match self.yeni_kod(kisi_id) {
            Ok(k) => k,
            Err(e) => return Yanit::hata(500, &e),
        };
        let url = format!("{}/telefon?kod={kod}", self.taban_url.trim_end_matches('/'));
        match ek_uclar::karekod_svg(&url) {
            Ok(svg) => Yanit::json(
                200,
                json!({ "kod": kod, "url": url, "qr_svg": svg, "gecerli_sn": KOD_OMRU, "kisi": kisi_id }),
            ),
            Err(e) => Yanit::hata(500, &e),
        }
    }

    /// GET /api/yonetim/cihazlar: eşleşmiş telefonlar, son görülme ve bağlı kişi.
    pub(crate) fn cihaz_listesi(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let sube = self.sube_kodu().unwrap_or_default();
        let h = kisi::ad_haritasi(&self.dizin, &sube);
        let gorulme = self
            .son_gorulme
            .lock()
            .map(|m| m.clone())
            .unwrap_or_default();
        let simdi = self.simdi();
        let v: Vec<_> = self
            .cihazlar()
            .aktifler()
            .unwrap_or_default()
            .into_iter()
            .map(|c| {
                let son = gorulme.get(&c.id).copied();
                json!({
                    "id": c.id, "ad": c.ad, "tarih": c.tarih,
                    // "teslim-acik" kişi/rol kuralıyla (etkin karar).
                    "yetkiler": self.etkin_yetkiler(None, &c),
                    "kisi_id": c.kisi_id,
                    "kisi": h.get(&c.kisi_id).cloned().unwrap_or_default(),
                    "son_gorulme_sn": son.map(|t| simdi.saturating_sub(t)),
                })
            })
            .collect();
        // İkinci kodun onayını bekleyen telefon sayısı (kodlar dönmez; yalnız sayı).
        let bekleyen = self
            .bekleyen
            .lock()
            .map(|b| {
                b.iter()
                    .filter(|x| x.tamam.is_none() && x.son > simdi)
                    .count()
            })
            .unwrap_or(0);
        Yanit::json(200, json!({ "cihazlar": v, "bekleyen": bekleyen }))
    }

    /// GET /api/yonetim/kisiler: yerel kişi kaydı + Telegram hesapları (yönetici görür).
    pub(crate) fn kisi_listesi_yonetim(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let sube = self.sube_kodu().unwrap_or_default();
        let hesaplar = taa_telegram::hesap::Hesaplar { dizin: &self.dizin }
            .guncel()
            .unwrap_or_default();
        Yanit::json(
            200,
            json!({
                "kisiler": kisi::secilebilir(&self.dizin, &sube),
                "hesaplar": hesaplar,
                "cati_esas": crate::kurulum::cati_bagli(&self.dizin),
            }),
        )
    }

    /// POST /api/yonetim/kisi {ad, rol}. TAA Çatı listesi esassa yerel ekleme yapılmaz (tek kimlik, Fable M6).
    pub(crate) fn kisi_ekle(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        if crate::kurulum::cati_bagli(&self.dizin) {
            return Yanit::hata(
                409,
                "kişiler TAA Çatı'dan gelir; TAA Rehber'de bu şubeye görevlendirin",
            );
        }
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        // Personel adı ve soyadı birlikte yazılır (fişe "Ad S." basılır).
        if v["ad"].as_str().unwrap_or("").split_whitespace().count() < 2 {
            return Yanit::hata(422, "personelin adı ve soyadı birlikte yazılmalı");
        }
        match kisi::Kisiler::dizinden(&self.dizin).ekle(
            v["ad"].as_str().unwrap_or(""),
            v["rol"].as_str().unwrap_or("personel"),
            &tarih_iso(self.simdi()),
        ) {
            Ok(k) => Yanit::json(200, json!(k)),
            Err(e) => Yanit::hata(422, &e),
        }
    }

    /// POST /api/yonetim/kisi/<id>/kapat
    pub(crate) fn kisi_kapat(&self, i: &Istek, id: &str) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        match kisi::Kisiler::dizinden(&self.dizin).kapat(id, &tarih_iso(self.simdi())) {
            Ok(()) => {
                // Kapatılan kişinin "teslim-acik" kaydı da geri alınır (karar ayrıca açık listeye bakar; bu kayıt
                // kişi yeniden açılırsa yetkinin kendiliğinden geri gelmemesi içindir).
                if self
                    .cihazlar()
                    .yetki_kaydi(&cihaz::kisi_anahtari(id), "teslim-acik")
                    == Some(true)
                {
                    let _ = self.cihazlar().kisi_yetki_ata(
                        id,
                        "teslim-acik",
                        false,
                        &tarih_iso(self.simdi()),
                    );
                }
                Yanit::json(200, json!({ "tamam": true }))
            }
            Err(e) => Yanit::hata(404, &e),
        }
    }

    /// POST /api/yonetim/cihaz/<id>/ad {ad}: cihaz adı yalnız bu bilgisayardaki programdan, yönetici oturumuyla.
    pub(crate) fn cihaz_ad(&self, i: &Istek, id: &str) -> Yanit {
        if !i.masaustu {
            return Yanit::hata(
                403,
                "cihaz adı yalnız mağaza bilgisayarındaki programdan değişir",
            );
        }
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        match self.cihazlar().ad_degistir(
            id,
            v["ad"].as_str().unwrap_or(""),
            &tarih_iso(self.simdi()),
        ) {
            Ok(()) => Yanit::json(200, json!({ "tamam": true })),
            Err(e) => Yanit::hata(422, &e),
        }
    }

    /// GET /api/yonetim/yetkiler: yetki listesi + mağaza PC'si ve her cihazın açık yetkileri.
    pub(crate) fn yetki_listesi(&self, i: &Istek) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let c = self.cihazlar();
        let tanim: Vec<_> = cihaz::YETKILER
            .iter()
            .map(|(k, a, pc, tel)| json!({ "anahtar": k, "aciklama": a, "pc_varsayilan": pc, "telefon_varsayilan": tel }))
            .collect();
        // Yetkiler oturumdan bağımsız (kalıcı) karardır: "teslim-acik" cihaz kaydı > kişi kaydı > rol.
        let pc = cihaz::Cihaz {
            id: crate::PC_ID.into(),
            ad: "Mağaza bilgisayarı".into(),
            ..Default::default()
        };
        let mut cihazlar = vec![
            json!({ "id": crate::PC_ID, "ad": "Mağaza bilgisayarı", "yetkiler": self.etkin_yetkiler(None, &pc) }),
        ];
        for x in c.aktifler().unwrap_or_default() {
            cihazlar.push(json!({
                "id": x.id, "ad": x.ad, "kisi_id": x.kisi_id,
                "yetkiler": self.etkin_yetkiler(None, &x),
                "teslim_acik_kaynagi": self.teslim_acik_kalici(&x.id, &x.kisi_id).map(|k| k.ad()),
            }));
        }
        Yanit::json(
            200,
            json!({
                "yetkiler": tanim,
                "cihazlar": cihazlar,
                // Kişi bazında "teslim-acik" (rol varsayılanı: yönetici, mağaza müdürü).
                "kisiler": self.kisi_yetki_listesi(),
                "teslim_acik_rolleri": cihaz::TESLIM_ACIK_ROLLER,
            }),
        )
    }

    /// POST /api/yonetim/cihaz/<id>/yetki {yetki, ver}: yönetici yetki verir/geri alır.
    pub(crate) fn cihaz_yetki(&self, i: &Istek, id: &str) -> Yanit {
        if let Some(y) = self.yonetim_izni(i) {
            return y;
        }
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let Some(ver) = v["ver"].as_bool() else {
            return Yanit::hata(400, "ver: true ya da false");
        };
        match self.cihazlar().yetki_ata(
            id,
            v["yetki"].as_str().unwrap_or(""),
            ver,
            &tarih_iso(self.simdi()),
        ) {
            Ok(()) => {
                let satir = json!({"olay": "yetki", "cihaz": id, "yetki": v["yetki"], "ver": ver, "kim": self.yonetici_kimi(i), "kim_kaynak": self.kim_kaynagi(i), "tarih": tarih_iso(self.simdi()), "saat": saat_iso(self.simdi())}).to_string();
                let _ = taa_dosya::satir_ekle(&self.dizin.join("eslestirme.jsonl"), &satir);
                Yanit::json(
                    200,
                    json!({ "tamam": true, "yetkiler": self.cihazlar().yetkileri(id) }),
                )
                // Not: "teslim-acik" listede etkin karar olarak `yetki_listesi`nden okunur.
            }
            Err(e) => Yanit::hata(422, &e),
        }
    }
}
