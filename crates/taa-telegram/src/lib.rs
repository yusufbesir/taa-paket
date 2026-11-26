//! taa-telegram: terzihane iç işlemleri için Telegram botu çekirdeği.
//! Bot yalnız ONAYLI Telegram hesaplarına yanıt verir (`hesaplar.jsonl`; onayı yalnız yönetici ekranı verir, bkz. `hesap`).
//! Eski `telegram.json` `kullanicilar` listesi geriye uyum için usta sayılır.
//! Çekirdekte ağ kodu YOK: güncelleme gelir, Telegram yöntem çağrıları (JSON) üretilir. Webhook yanıtı ya da
//! `yoklama` modülündeki uzun yoklama bunları taşır (dışarıdan kapı açılmaz).
//! Usta imzası: dayanak değişmeyen sayısal Telegram kimliğidir; kayda profil adı da yazılır (ad girmek gerekmez).
//! Müşteri adı/telefonu bot mesajlarında yalnız MASKELİ görünür; müşteri numarası Telegram'dan GEÇMEZ:
//! "Müşteriye WhatsApp'tan haber ver" düğmesi yerel sunucudaki tek kullanımlık adrese gider, o `wa.me`ye yönlendirir,
//! Gönder'e insan basar. Bu düğmeyi yalnız `magaza` rolündeki (mağaza telefonu) hesap görür.
//! Özellik anahtarı `telegram-bot`: varsayılan KAPALI. Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-05], TT12c: [Claude Sonnet 5.5, TT başlığı 2026-11-15].

pub mod hesap;
pub mod kargo;
mod kargo_bot;
pub mod yoklama;

use hesap::{HesapDurum, Hesaplar, Rol};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::path::Path;
use taa_fis::{Defter, Durum, Fis};

const OZELLIK: &str = "telegram-bot";

#[derive(Debug, Deserialize)]
pub struct Ayar {
    /// Webhook adresindeki gizli parça: `/telegram/<gizli>` (uzun yoklamada kullanılmaz).
    #[serde(default)]
    pub gizli: String,
    /// ESKİ: Telegram kullanıcı kimliği -> görünen ad. Geriye uyum: bu hesaplar usta sayılır.
    #[serde(default)]
    pub kullanicilar: BTreeMap<String, String>,
    /// Botun kullanıcı adı (`t.me/<bot_adi>?start=<fiş no>` kare kodu için; anahtar DEĞİL).
    #[serde(default)]
    pub bot_adi: String,
    /// Yerel sunucunun telefona görünen taban adresi (ör. https://sunucu.ts.net); WhatsApp yönlendirme düğmesi için.
    #[serde(default)]
    pub yonlendirme: String,
    /// Hesap başına dakikada en çok komut (Fable tasarım kararı, ölçüm değil; pilotta gözden geçirilir).
    #[serde(default = "varsayilan_hiz")]
    pub hiz_siniri: u32,
    /// Kargo özetinde alıcı adı ve telefon açık yazılsın mı? Varsayılan HAYIR (maskeli). Yusuf WhatsApp'a yapıştırmak için açabilir.
    #[serde(default)]
    pub kargo_tam_ozet: bool,
}

fn varsayilan_hiz() -> u32 {
    20
}

impl Ayar {
    pub fn dizinden(dizin: &Path) -> Result<Ayar, String> {
        let s = std::fs::read_to_string(dizin.join("telegram.json"))
            .map_err(|_| "telegram.json yok (bot ayarlanmamış)".to_string())?;
        let a: Ayar =
            serde_json::from_str(&s).map_err(|e| format!("telegram.json geçersiz: {e}"))?;
        if a.gizli.len() < 16 {
            return Err("telegram.json: gizli en az 16 karakter olmalı".into());
        }
        Ok(a)
    }

    /// Sabit zamanlı karşılaştırma.
    pub fn gizli_dogru(&self, g: &str) -> bool {
        self.gizli.len() == g.len()
            && self
                .gizli
                .bytes()
                .zip(g.bytes())
                .fold(0u8, |d, (a, b)| d | (a ^ b))
                == 0
    }
}

pub struct Bot<'a> {
    pub dizin: &'a Path,
    pub ayar: &'a Ayar,
    /// Bugünün tarihi (ISO).
    pub bugun: &'a str,
    /// Şimdi, unix saniye (kod ömrü, hız sınırı, imza saati). Saat dilimi: Türkiye UTC+3.
    pub simdi: u64,
}

/// Onaylı kullanıcının bu işlemdeki kimliği.
#[derive(Debug, Clone)]
pub struct Kimlik {
    pub tg_id: i64,
    pub profil_adi: String,
    pub usta_id: String,
    pub rol: Rol,
    pub sube: String,
}

impl Kimlik {
    /// Fiş defterindeki imza adı: sayısal kimlik + o anki profil adı.
    pub fn imza_adi(&self) -> String {
        format!("tg:{} {}", self.tg_id, self.profil_adi)
    }
    /// Kişisel usta imzasını yalnız `usta` rolü atar; mağaza telefonu ve yönetici atamaz (K8, K9).
    fn usta_mi(&self) -> bool {
        self.rol == Rol::Usta
    }
    fn magaza_mi(&self) -> bool {
        matches!(self.rol, Rol::Magaza | Rol::Yonetici)
    }
    /// K5: şube kapsamı. Fişin şubesi yazılıysa hesabın şubesiyle aynı olmalı; yönetici tüm şubeleri görür.
    /// Şubesi yazılmamış (eski) fişe denetim uygulanamaz, geçer; şubesiz hesap şubeli fişi göremez.
    fn sube_uygun(&self, f: &Fis) -> bool {
        self.rol == Rol::Yonetici || f.sube.is_empty() || f.sube.eq_ignore_ascii_case(&self.sube)
    }
}

/// Fişi bulur ve şube kapsamını denetler (K5). Başka şubenin fişi için fişin var olduğu bile söylenmez.
fn kapsamli_bul(d: &Defter, no: &str, k: &Kimlik) -> Result<Option<Fis>, String> {
    Ok(d.bul(no)?.filter(|f| k.sube_uygun(f)))
}

/// Fişteki kare kodun içeriği: botu fiş numarasıyla açar. Kodda ad/telefon YOK.
pub fn fis_baglantisi(bot_adi: &str, fis_no: &str) -> String {
    format!("https://t.me/{bot_adi}?start={fis_no}")
}

/// Kasa okuyucusu için: kare kod metninden (bağlantı ya da düz numara) fiş numarasını ayıklar.
pub fn baslangic_kodu(metin: &str) -> Option<String> {
    let m = metin.trim();
    let no = match m.split_once("start=") {
        Some((_, k)) => k.split(['&', '#', ' ']).next().unwrap_or(""),
        None => m,
    };
    (!no.is_empty()
        && no.len() <= 40
        && no
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'))
    .then(|| no.to_string())
}

/// "Ayşe Çelik" -> "A**e Ç***k": her sözcüğün ilk ve son harfi görünür, ortası yıldız.
pub fn maske_ad(ad: &str) -> String {
    ad.split_whitespace()
        .map(|w| {
            let c: Vec<char> = w.chars().collect();
            match c.len() {
                0 => String::new(),
                1 => "*".into(),
                2 => format!("{}*", c[0]),
                n => format!("{}{}{}", c[0], "*".repeat(n - 2), c[n - 1]),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// "05321234567" -> "0532*****67": ilk hane + alan kodu ve son 2 hane görünür.
pub fn maske_tel(t: &str) -> String {
    let c: Vec<char> = t.chars().filter(char::is_ascii_digit).collect();
    if c.len() < 8 {
        return "*".repeat(c.len());
    }
    let (bas, son) = (4, 2);
    format!(
        "{}{}{}",
        c[..bas].iter().collect::<String>(),
        "*".repeat(c.len() - bas - son),
        c[c.len() - son..].iter().collect::<String>()
    )
}

fn url_kodla(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            _ => format!("%{b:02X}"),
        })
        .collect()
}

#[derive(Serialize, Deserialize)]
struct WaKayit {
    belirtec: String,
    mesaj_id: String,
    tg_id: i64,
    gun: String,
    kullanildi: bool,
}

fn wa_kayitlari(dizin: &Path) -> Result<Vec<WaKayit>, String> {
    taa_dosya::satirlari_oku(&dizin.join("wa-belirtecler.jsonl"))
        .map_err(|e| e.to_string())?
        .iter()
        .map(|l| serde_json::from_str(l).map_err(|e| format!("wa satırı bozuk: {e}")))
        .collect()
}

fn wa_belirtec_uret(dizin: &Path, mesaj_id: &str, tg_id: i64, gun: &str) -> Result<String, String> {
    let mut b = [0u8; 16];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    let belirtec: String = b.iter().map(|x| format!("{x:02x}")).collect();
    let k = WaKayit {
        belirtec: belirtec.clone(),
        mesaj_id: mesaj_id.into(),
        tg_id,
        gun: gun.into(),
        kullanildi: false,
    };
    let s = serde_json::to_string(&k).map_err(|e| e.to_string())?;
    taa_dosya::satir_ekle(&dizin.join("wa-belirtecler.jsonl"), &s).map_err(|e| e.to_string())?;
    Ok(belirtec)
}

/// Yerel sunucudaki `/wa/<belirtec>` adresi bunu çağırır: belirteç tek kullanımlık ve yalnız o gün geçerlidir.
/// Döndürdüğü `wa.me` bağlantısı hazır metni ve müşteri numarasını taşır; numara Telegram'dan hiç geçmez.
pub fn wa_yonlendir(dizin: &Path, belirtec: &str, bugun: &str) -> Result<String, String> {
    let kayitlar = wa_kayitlari(dizin)?;
    let k = kayitlar
        .iter()
        .rev()
        .find(|k| k.belirtec == belirtec)
        .ok_or("bağlantı geçersiz")?;
    if k.kullanildi {
        return Err("bu bağlantı daha önce kullanıldı; Telegram'dan yeniden isteyin".into());
    }
    if k.gun != bugun {
        return Err("bağlantının süresi doldu; Telegram'dan yeniden isteyin".into());
    }
    let q = taa_bildirim::Kuyruk {
        yol: &dizin.join("bildirimler.jsonl"),
    };
    let m = q
        .mesajlar()?
        .into_iter()
        .find(|m| m.id == k.mesaj_id)
        .ok_or("mesaj bulunamadı")?;
    let yeni = WaKayit {
        belirtec: belirtec.into(),
        mesaj_id: k.mesaj_id.clone(),
        tg_id: k.tg_id,
        gun: k.gun.clone(),
        kullanildi: true,
    };
    let s = serde_json::to_string(&yeni).map_err(|e| e.to_string())?;
    taa_dosya::satir_ekle(&dizin.join("wa-belirtecler.jsonl"), &s).map_err(|e| e.to_string())?;
    Ok(format!(
        "https://wa.me/{}?text={}",
        m.telefon,
        url_kodla(&m.metin)
    ))
}

const YARDIM: &str = "Komutlar:\n/durum FİŞNO\n/terzihanede FİŞNO\n/hazir FİŞNO\n/raf FİŞNO A-12\n/imza FİŞNO\n/bugun (bugün teslim)\n/geciken\n/kargo (kargo bekleyenler)
/kargo FİŞNO (kargo bilgisi gir)
/takip FİŞNO NO FİRMA
/kargoozet FİŞNO\n/kuru (kuru temizleme)\n/ozet\n/yardim\nFişin kare kodunu okutursanız fiş açılır.";

fn gonder(sohbet: i64, metin: &str, dugmeler: Option<Value>) -> Value {
    let metin: String = metin.chars().take(3500).collect();
    match dugmeler {
        Some(d) => {
            json!({"method": "sendMessage", "chat_id": sohbet, "text": metin, "reply_markup": {"inline_keyboard": d}})
        }
        None => json!({"method": "sendMessage", "chat_id": sohbet, "text": metin}),
    }
}

fn ad_goster(from: &Value) -> String {
    let ad = [from["first_name"].as_str(), from["last_name"].as_str()]
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>()
        .join(" ");
    if !ad.trim().is_empty() {
        ad
    } else {
        from["username"].as_str().unwrap_or("?").to_string()
    }
}

fn fis_ozeti(f: &Fis) -> String {
    format!(
        "{}: {:?}\nMüşteri: {} Tel: {}\n{}\nTeslim: {} {}\nRaf: {}\nİmza: {}",
        f.no,
        f.durum,
        maske_ad(&f.musteri),
        maske_tel(&f.telefon),
        f.kalemler
            .iter()
            .map(|k| format!("{} - {}", k.urun, k.islem))
            .collect::<Vec<_>>()
            .join("; "),
        f.teslim,
        f.teslim_saati,
        if f.raf.is_empty() { "—" } else { &f.raf },
        if f.imzalar.is_empty() {
            "—".to_string()
        } else {
            f.imzalar
                .iter()
                .map(|i| format!("{} ({})", i.kim, i.rol))
                .collect::<Vec<_>>()
                .join(", ")
        },
    )
}

/// Bellekte tutulan sayaçlar (kalıcı DEĞİL, kayıt yazmaz): hız sınırı ve yanlış kod denemesi.
fn sayac(tur: &str, tg: i64, pencere: u64, sinir: u32) -> bool {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    type Sayac = Mutex<HashMap<(String, i64), (u64, u32)>>;
    static S: OnceLock<Sayac> = OnceLock::new();
    let mut m = S
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let e = m.entry((tur.to_string(), tg)).or_insert((pencere, 0));
    if e.0 != pencere {
        *e = (pencere, 0);
    }
    e.1 += 1;
    e.1 <= sinir
}

/// Kalem başına imza (K2): anahtar (fiş, kalem, tur). Alan adları `taa-takip` Olay ile aynıdır; ek alanlar tg_id, profil_adi, tur, kaynak.
fn imza_olaylari(dizin: &Path) -> Vec<Value> {
    taa_dosya::satirlari_oku(&dizin.join("imza-olaylari.jsonl"))
        .unwrap_or_default()
        .iter()
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

/// Yeniden iş (tezgâhtar işareti) kalem için yeni tur açar.
fn acilan_tur(dizin: &Path, no: &str, kalem: usize) -> usize {
    1 + taa_takip::PuanDefteri::dizinden(dizin)
        .oku()
        .unwrap_or_default()
        .iter()
        .filter(|k| matches!(k, taa_takip::Kayit::Isaret(i) if i.fis_no == no && i.kalem_no == kalem && i.tur == taa_takip::IsaretTuru::YenidenIs))
        .count()
}

fn imzali_tur(dizin: &Path, no: &str, kalem: usize) -> usize {
    imza_olaylari(dizin)
        .iter()
        .filter(|o| o["fis_no"] == no && o["kalem_no"].as_u64() == Some(kalem as u64))
        .count()
}

fn bekleyen_kalemler(dizin: &Path, f: &Fis) -> Vec<usize> {
    (0..f.kalemler.len())
        .filter(|&i| imzali_tur(dizin, &f.no, i) < acilan_tur(dizin, &f.no, i))
        .collect()
}

fn saat_metni(simdi: u64) -> String {
    let dk = ((simdi + 3 * 3600) / 60) % 1440;
    format!("{:02}:{:02}", dk / 60, dk % 60)
}

impl Bot<'_> {
    /// Webhook için tek yanıt: eylemlerin sonuncusu (düğme basışında `answerCallbackQuery` düşer, ileti kalır).
    pub fn yanit(&self, guncelleme: &Value) -> Option<Value> {
        self.eylemler(guncelleme).into_iter().last()
    }

    /// Bir güncellemenin doğurduğu Telegram yöntem çağrıları (sırayla gönderilir).
    pub fn eylemler(&self, g: &Value) -> Vec<Value> {
        if !taa_ayar::ozellik_acik(OZELLIK, false) {
            return vec![];
        }
        if let Some(m) = g.get("message") {
            return self.mesaj(m).into_iter().collect();
        }
        if let Some(c) = g.get("callback_query") {
            return self.dugme(c, g["update_id"].as_i64());
        }
        vec![]
    }

    /// Kimliği çözer; Err içindeki metin kullanıcıya gösterilir. Bilinmeyen hesap için HİÇBİR ŞEY kaydedilmez (K1).
    fn kimlik(&self, from: &Value) -> Result<Kimlik, String> {
        let tg_id = from["id"].as_i64().ok_or("kimlik yok")?;
        let profil = hesap::profil_temizle(&ad_goster(from));
        let h = Hesaplar { dizin: self.dizin };
        let kimlik = match h.bul(tg_id)? {
            Some(x) if x.durum == HesapDurum::Onayli => Kimlik {
                tg_id,
                profil_adi: profil,
                usta_id: x.usta_id,
                rol: x.rol,
                sube: x.sube,
            },
            Some(x) if x.durum == HesapDurum::OnayBekliyor => {
                return Err("Hesabınız bağlandı, yönetici onayı bekleniyor.".into())
            }
            Some(_) => return Err("Bu hesap kapalı. Yöneticiye başvurun.".into()),
            None if self.ayar.kullanicilar.contains_key(&tg_id.to_string()) => Kimlik {
                tg_id,
                profil_adi: profil,
                usta_id: format!("L-{tg_id}"),
                rol: Rol::Usta,
                sube: String::new(),
            },
            None => {
                return Err("Bu bot yalnız onaylı çalışanlar içindir. Yöneticinizden bağlama kodu isteyin ve buraya yazın.".into())
            }
        };
        if !sayac("hiz", tg_id, self.simdi / 60, self.ayar.hiz_siniri) {
            return Err("Çok hızlı komut gönderdiniz, bir dakika bekleyin.".into());
        }
        Ok(kimlik)
    }

    /// Henüz bağlanmamış hesap bir kod yazarsa bağlamayı dener. Yanlış denemeler hesap başına saatte 5 ile sınırlıdır.
    fn baglama_denemesi(&self, from: &Value, metin: &str) -> Option<String> {
        let tg_id = from["id"].as_i64()?;
        let kod = hesap::kod_ayikla(metin)?;
        if !sayac("kod", tg_id, self.simdi / 3600, 5) {
            return Some(
                "Çok fazla deneme. Yöneticinizle konuşun, bir saat sonra tekrar deneyin.".into(),
            );
        }
        let h = Hesaplar { dizin: self.dizin };
        match h.kod_ile_bagla(tg_id, &ad_goster(from), &kod, self.simdi, self.bugun) {
            Ok(x) => Some(format!(
                "Bağlandı. Doğrulama sayınız: {}. Yöneticinin ekranındaki sayıyla aynı olmalı; yönetici onayından sonra bot çalışır.",
                x.dogrulama
            )),
            Err(e) => Some(format!("Kod kabul edilmedi: {e}")),
        }
    }

    fn mesaj(&self, m: &Value) -> Option<Value> {
        let sohbet = m["chat"]["id"].as_i64()?;
        let metin = m["text"].as_str()?;
        m["from"]["id"].as_i64()?;
        // Kayıtlı olmayan hesap yalnız bağlama kodu yazabilir.
        let h = Hesaplar { dizin: self.dizin };
        let tg = m["from"]["id"].as_i64()?;
        if h.bul(tg).ok().flatten().is_none()
            && !self.ayar.kullanicilar.contains_key(&tg.to_string())
        {
            if let Some(c) = self.baglama_denemesi(
                &m["from"],
                metin
                    .trim_start_matches("/start")
                    .trim()
                    .trim_start_matches("baglan-")
                    .trim_start_matches("/baglan")
                    .trim(),
            ) {
                return Some(gonder(sohbet, &c, None));
            }
        }
        match self.kimlik(&m["from"]) {
            Ok(k) => Some(self.komut(sohbet, metin, &k)),
            Err(e) => Some(gonder(sohbet, &e, None)),
        }
    }

    /// Fiş kartının düğmeleri: usta için bekleyen her kalem, mağaza için WhatsApp.
    fn dugmeler(&self, f: &Fis, k: &Kimlik) -> Option<Value> {
        let mut satirlar = vec![];
        if k.usta_mi() && matches!(f.durum, Durum::Bekliyor | Durum::Terzihanede | Durum::Hazir) {
            for i in bekleyen_kalemler(self.dizin, f) {
                let kalem = &f.kalemler[i];
                let ad: String = format!("{} - {}", kalem.urun, kalem.islem)
                    .chars()
                    .take(40)
                    .collect();
                satirlar.push(json!([{"text": format!("Bitti: {ad}"), "callback_data": format!("k:{}:{i}", f.no)}]));
            }
        }
        if k.magaza_mi() && f.durum == Durum::Hazir {
            satirlar.push(json!([{"text": "Müşteriye WhatsApp'tan haber ver", "callback_data": format!("w:{}", f.no)}]));
        }
        (!satirlar.is_empty()).then_some(Value::Array(satirlar))
    }

    fn komut(&self, sohbet: i64, metin: &str, k: &Kimlik) -> Value {
        let mut p = metin.split_whitespace();
        let kom = p
            .next()
            .unwrap_or("")
            .split('@')
            .next()
            .unwrap_or("")
            .to_lowercase();
        let (a1, a2) = (p.next().unwrap_or(""), p.next().unwrap_or(""));
        let yol = self.dizin.join("fisler.jsonl");
        let d = Defter { yol: &yol };
        if kom == "/start" && !a1.is_empty() {
            return match kapsamli_bul(&d, a1, k) {
                Ok(Some(f)) => gonder(sohbet, &fis_ozeti(&f), self.dugmeler(&f, k)),
                Ok(None) => gonder(sohbet, &format!("Hata: fiş yok: {a1}"), None),
                Err(e) => gonder(sohbet, &format!("Hata: {e}"), None),
            };
        }
        // Kargo girişi (yalnız mağaza telefonu): /kargo FİŞNO [metin], /takip, /kargoozet; komut olmayan metin açık taslağa eklenir.
        if kom == "/kargo" && !a1.is_empty() {
            return self.kargo_baslat(sohbet, &d, a1, metin, k);
        }
        if kom == "/takip" {
            return self.kargo_takip(sohbet, &d, a1, a2, p.next().unwrap_or(""), k);
        }
        if kom == "/kargoozet" || kom == "/kargoözet" {
            return self.kargo_ozet(sohbet, &d, a1, k);
        }
        if kom == "/kargoyardim" || kom == "/kargoyardım" {
            return gonder(sohbet, Self::kargo_yardimi(), None);
        }
        if !kom.starts_with('/') {
            if let Some(y) = self.kargo_serbest(sohbet, metin, k) {
                return y;
            }
        }
        let r = match kom.as_str() {
            "/start" | "/yardim" | "/yardım" => Ok(YARDIM.to_string()),
            "/durum" => kapsamli_bul(&d, a1, k)
                .and_then(|f| f.ok_or_else(|| format!("fiş yok: {a1}")))
                .map(|f| fis_ozeti(&f)),
            "/bugun" | "/bugün" => d.fisler().map(|v| {
                let l: Vec<String> = v
                    .iter()
                    .filter(|f| {
                        k.sube_uygun(f)
                            && f.teslim == self.bugun
                            && matches!(
                                f.durum,
                                Durum::Bekliyor | Durum::Terzihanede | Durum::Hazir
                            )
                    })
                    .map(|f| {
                        format!(
                            "{} {:?} {} {}",
                            f.no,
                            f.durum,
                            f.teslim_saati,
                            maske_ad(&f.musteri)
                        )
                    })
                    .collect();
                if l.is_empty() {
                    "Bugün teslim edilecek fiş yok.".into()
                } else {
                    l.join("\n")
                }
            }),
            "/geciken" => d.geciken(self.bugun).map(|v| {
                let v: Vec<_> = v.into_iter().filter(|f| k.sube_uygun(f)).collect();
                if v.is_empty() {
                    "Geciken fiş yok.".into()
                } else {
                    v.iter()
                        .map(|f| format!("{} teslim {}", f.no, f.teslim))
                        .collect::<Vec<_>>()
                        .join("\n")
                }
            }),
            "/kargo" | "/kuru" | "/ozet" => taa_takip::Takip::defterden(&d).map(|t| t.sinirla(|f| k.sube_uygun(f))).map(|t| match kom.as_str() {
                "/ozet" => {
                    let o = t.ozet(self.bugun);
                    format!(
                        "Bekliyor {} | Terzihanede {} | Hazır {}\nGeciken {} | Bugün teslim {}\nKargo bekleyen {} | Kuru temizleme {}",
                        o.bekliyor, o.terzihanede, o.hazir, o.geciken, o.bugun_teslim, o.kargo_bekleyen, o.kuru_temizleme
                    )
                }
                ad => {
                    let v = if ad == "/kargo" { t.kargo_bekleyen() } else { t.kuru_temizleme() };
                    if v.is_empty() {
                        "Liste boş.".into()
                    } else {
                        // KVKK: bot mesajında yalnız maskeli ad; kargo adresi/telefonu gösterilmez
                        v.iter()
                            .map(|f| format!("{} {} teslim {}", f.no, maske_ad(&f.musteri), f.teslim))
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                }
            }),
            "/terzihanede" => kapsamli_bul(&d, a1, k)
                .and_then(|f| f.ok_or_else(|| format!("fiş yok: {a1}")))
                .and_then(|_| d.durum_ata(a1, Durum::Terzihanede, self.bugun))
                .map(|_| format!("{a1}: terzihanede")),
            "/hazir" | "/hazır" => kapsamli_bul(&d, a1, k)
                .and_then(|f| f.ok_or_else(|| format!("fiş yok: {a1}")))
                .and_then(|_| self.hazir(&d, a1)),
            "/raf" => kapsamli_bul(&d, a1, k)
                .and_then(|f| f.ok_or_else(|| format!("fiş yok: {a1}")))
                .and_then(|_| d.raf_ata(a1, a2, &k.imza_adi(), self.bugun))
                .map(|_| format!("{a1}: raf {}", a2.to_uppercase())),
            // /imza: kalem seçtirip onay isteyen akışı başlatır (K7); doğrudan imza atmaz.
            "/imza" => return self.imza_baslat(sohbet, &d, a1, k),
            _ => Ok(format!("Anlamadım. {YARDIM}")),
        };
        gonder(sohbet, &r.unwrap_or_else(|e| format!("Hata: {e}")), None)
    }

    fn imza_baslat(&self, sohbet: i64, d: &Defter, no: &str, k: &Kimlik) -> Value {
        if !k.usta_mi() {
            return gonder(sohbet, "Hata: imza yalnız usta hesabıyla atılır.", None);
        }
        match kapsamli_bul(d, no, k) {
            Ok(Some(f)) => match self.dugmeler(&f, k) {
                Some(dg) => gonder(sohbet, &fis_ozeti(&f), Some(dg)),
                None => gonder(sohbet, "İmzalanacak kalem yok.", None),
            },
            Ok(None) => gonder(sohbet, &format!("Hata: fiş yok: {no}"), None),
            Err(e) => gonder(sohbet, &format!("Hata: {e}"), None),
        }
    }

    /// Kalem imzası (K2): anahtar (fiş, kalem, tur). Olay `imza-olaylari.jsonl`'e Olay alan adlarıyla yazılır.
    /// Fiş defterine imza satırı da düşer (kim = sayısal kimlik + profil adı; çift satır hatası yok sayılır).
    fn kalem_imzala(
        &self,
        d: &Defter,
        no: &str,
        kalem: usize,
        k: &Kimlik,
        guncelleme: Option<i64>,
    ) -> Result<(), String> {
        if !k.usta_mi() {
            return Err("imza yalnız usta hesabıyla atılır".into());
        }
        // K5: şube kapsamı (sunucu tarafında; kare koddaki fiş numarası tahmin edilebilir).
        let f = kapsamli_bul(d, no, k)?.ok_or_else(|| format!("fiş yok: {no}"))?;
        let kal = f.kalemler.get(kalem).ok_or("kalem yok")?;
        if !matches!(f.durum, Durum::Bekliyor | Durum::Terzihanede | Durum::Hazir) {
            return Err("bu fiş artık imzalanamaz".into());
        }
        let tur = imzali_tur(self.dizin, no, kalem);
        if tur >= acilan_tur(self.dizin, no, kalem) {
            return Err("bu kalem zaten imzalanmış".into());
        }
        let olay = json!({"usta": k.usta_id, "fis_no": no, "kalem_no": kalem, "islem_kodu": kal.islem_kodu,
            "norm_dk": null, "tarih": self.bugun, "saat": saat_metni(self.simdi),
            "tur": tur + 1, "tg_id": k.tg_id, "profil_adi": k.profil_adi, "sube": k.sube,
            "kaynak": "telegram", "guncelleme_id": guncelleme});
        taa_dosya::satir_ekle(&self.dizin.join("imza-olaylari.jsonl"), &olay.to_string())
            .map_err(|e| e.to_string())?;
        let _ = d.imzala(no, &k.imza_adi(), "usta", self.bugun);
        Ok(())
    }

    fn hazir(&self, d: &Defter, no: &str) -> Result<String, String> {
        let f = d.bul(no)?.ok_or_else(|| format!("fiş yok: {no}"))?;
        if f.durum == Durum::Bekliyor {
            d.durum_ata(no, Durum::Terzihanede, self.bugun)?;
        }
        d.durum_ata(no, Durum::Hazir, self.bugun)?;
        let b = taa_bildirim::hazir_bildir(self.dizin, no, &f.telefon, f.bildirim_izni, self.bugun);
        Ok(format!("{no}: hazır. Müşteri bildirimi: {b}"))
    }

    fn dugme(&self, c: &Value, guncelleme: Option<i64>) -> Vec<Value> {
        let (Some(cid), Some(sohbet), Some(veri)) = (
            c["id"].as_str(),
            c["message"]["chat"]["id"].as_i64(),
            c["data"].as_str(),
        ) else {
            return vec![];
        };
        let cevap =
            |t: &str| json!({"method": "answerCallbackQuery", "callback_query_id": cid, "text": t});
        let k = match self.kimlik(&c["from"]) {
            Ok(k) => k,
            Err(e) => {
                return vec![
                    json!({"method": "answerCallbackQuery", "callback_query_id": cid, "text": e, "show_alert": true}),
                ]
            }
        };
        let yol = self.dizin.join("fisler.jsonl");
        let d = Defter { yol: &yol };
        let mut p = veri.split(':');
        let tur = p.next().unwrap_or("");
        let (a1, a2) = (p.next().unwrap_or(""), p.next().unwrap_or(""));
        match tur {
            // k: kalem seçildi -> "Bitti mi?" onayı (K7)
            "k" => self.onay_iste(&d, a1, a2.parse().ok(), &k, sohbet, cevap),
            // y: onaylandı -> imza
            "y" => self.imza_onaylandi(&d, a1, a2.parse().ok(), &k, sohbet, guncelleme, cevap),
            "v" => vec![cevap("Vazgeçildi.")],
            "kg" => self.kargo_dugmesi(a1, &k, sohbet, &cevap),
            "w" => self.wa_dugmesi(&d, a1, &k, sohbet, cevap),
            "g" => {
                if !k.magaza_mi() {
                    return vec![cevap("Bu düğme yalnız mağaza telefonu içindir.")];
                }
                let kuyruk = taa_bildirim::Kuyruk {
                    yol: &self.dizin.join("bildirimler.jsonl"),
                };
                match kuyruk.sonuc(a1, taa_bildirim::Durum::Gonderildi, self.bugun) {
                    Ok(()) => vec![
                        cevap("Kaydedildi: gönderildi."),
                        gonder(
                            sohbet,
                            &format!("{a1}: müşteriye WhatsApp'tan gönderildi olarak işaretlendi."),
                            None,
                        ),
                    ],
                    Err(e) => vec![cevap(&format!("Hata: {e}"))],
                }
            }
            _ => vec![cevap("Bilinmeyen düğme.")],
        }
    }

    fn onay_iste(
        &self,
        d: &Defter,
        no: &str,
        kalem: Option<usize>,
        k: &Kimlik,
        sohbet: i64,
        cevap: impl Fn(&str) -> Value,
    ) -> Vec<Value> {
        if !k.usta_mi() {
            return vec![cevap("İmza yalnız usta hesabıyla atılır.")];
        }
        let (Ok(Some(f)), Some(i)) = (kapsamli_bul(d, no, k), kalem) else {
            return vec![cevap("Fiş ya da kalem bulunamadı.")];
        };
        let Some(kal) = f.kalemler.get(i) else {
            return vec![cevap("Kalem bulunamadı.")];
        };
        let dg = json!([[
            {"text": "Evet, bitti", "callback_data": format!("y:{no}:{i}")},
            {"text": "Vazgeç", "callback_data": "v"}
        ]]);
        vec![
            cevap("Onay bekleniyor."),
            gonder(
                sohbet,
                &format!(
                    "{no}: {} - {}\nBitti mi? Onaylarsanız bu iş sizin adınıza imzalanır.",
                    kal.urun, kal.islem
                ),
                Some(dg),
            ),
        ]
    }

    #[allow(clippy::too_many_arguments)]
    fn imza_onaylandi(
        &self,
        d: &Defter,
        no: &str,
        kalem: Option<usize>,
        k: &Kimlik,
        sohbet: i64,
        guncelleme: Option<i64>,
        cevap: impl Fn(&str) -> Value,
    ) -> Vec<Value> {
        let Some(i) = kalem else {
            return vec![cevap("Kalem bulunamadı.")];
        };
        if let Err(e) = self.kalem_imzala(d, no, i, k, guncelleme) {
            return vec![cevap(&format!("Hata: {e}"))];
        }
        let mut metin = format!("{no}: kalem {} bitti, {} imzaladı.", i + 1, k.profil_adi);
        // Tüm kalemler imzalanınca fiş "hazır" olur ve müşteri bildirimi kuyruğa yazılır.
        if let Ok(Some(f)) = d.bul(no) {
            if f.durum != Durum::Hazir && bekleyen_kalemler(self.dizin, &f).is_empty() {
                match self.hazir(d, no) {
                    Ok(s) => metin.push_str(&format!("\nTüm kalemler bitti. {s}")),
                    Err(e) => metin.push_str(&format!("\nDurum güncellenemedi: {e}")),
                }
            }
        }
        vec![cevap("İmzalandı."), gonder(sohbet, &metin, None)]
    }

    fn wa_dugmesi(
        &self,
        d: &Defter,
        no: &str,
        k: &Kimlik,
        sohbet: i64,
        cevap: impl Fn(&str) -> Value,
    ) -> Vec<Value> {
        if !k.magaza_mi() {
            return vec![cevap("Bu düğme yalnız mağaza telefonu içindir.")];
        }
        let f = match kapsamli_bul(d, no, k) {
            Ok(Some(f)) if f.durum == Durum::Hazir => f,
            _ => return vec![cevap("Fiş hazır değil.")],
        };
        if !f.bildirim_izni {
            return vec![cevap(
                "Müşteri bilgilendirme izni vermemiş: aranacaklar listesine alın.",
            )];
        }
        if self.ayar.yonlendirme.trim().is_empty() {
            return vec![cevap("Yönlendirme adresi ayarlanmamış; kasadaki \"Gönderilecek mesajlar\" listesinden gönderin.")];
        }
        taa_bildirim::hazir_bildir(self.dizin, no, &f.telefon, true, self.bugun);
        let kuyruk = taa_bildirim::Kuyruk {
            yol: &self.dizin.join("bildirimler.jsonl"),
        };
        let m = match kuyruk.mesajlar() {
            Ok(v) => v.into_iter().find(|m| m.fis_no == no),
            Err(_) => None,
        };
        let Some(m) = m else {
            return vec![cevap(
                "Mesaj kuyrukta yok (telefon geçersiz olabilir): aranacaklar listesine alın.",
            )];
        };
        let belirtec = match wa_belirtec_uret(self.dizin, &m.id, k.tg_id, self.bugun) {
            Ok(b) => b,
            Err(e) => return vec![cevap(&format!("Hata: {e}"))],
        };
        let adres = format!(
            "{}/wa/{belirtec}",
            self.ayar.yonlendirme.trim_end_matches('/')
        );
        let dugme = json!([
            [{"text": "WhatsApp'ı aç", "url": adres}],
            [{"text": "Gönderdim", "callback_data": format!("g:{}", m.id)}]
        ]);
        vec![
            cevap("Bağlantı hazır."),
            gonder(sohbet, &format!("{no}: WhatsApp'ı açın, mesaj hazır gelir, Gönder'e siz basın; sonra \"Gönderdim\"e dokunun. Bağlantı tek kullanımlıktır ve bugün geçerlidir."), Some(dugme)),
        ]
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use taa_fis::Girdi;

    fn kur() -> (std::path::PathBuf, String) {
        let d = taa_dosya::gecici_klasor().unwrap();
        let y = d.join("fisler.jsonl");
        let g: Girdi = serde_json::from_str(
            r#"{"sube":"MRK01","satis_danismani":"Mert","olcu_alan":"K","musteri":"Ayşe Çelik","telefon":"05321234567","kalemler":[{"urun":"Pantolon","islem":"paça kısalt"},{"urun":"Ceket","islem":"kol kısalt"}],"teslim":"2099-01-01","ucret_kurus":100,"bildirim_izni":true,"tarih":"2026-11-05"}"#,
        )
        .unwrap();
        let f = Defter { yol: &y }.ekle(g).unwrap();
        (d, f.no)
    }

    fn ayar() -> Ayar {
        Ayar {
            gizli: "0123456789abcdef".into(),
            kullanicilar: Default::default(),
            bot_adi: "taa_terzi_bot".into(),
            yonlendirme: "https://sunucu.example/".into(),
            hiz_siniri: 100_000,
            kargo_tam_ozet: false,
        }
    }

    fn msg(kim: i64, metin: &str) -> Value {
        json!({"message": {"chat": {"id": 7}, "from": {"id": kim, "first_name": "Ahmet", "username": "ahmet_x"}, "text": metin}})
    }
    fn tus(kim: i64, veri: &str) -> Value {
        json!({"update_id": 9, "callback_query": {"id": "c1", "from": {"id": kim, "first_name": "Ahmet"}, "message": {"chat": {"id": 7}}, "data": veri}})
    }
    fn metinler(v: &[Value]) -> String {
        v.iter()
            .map(|x| format!("{}{}", x["text"].as_str().unwrap_or(""), x["reply_markup"]))
            .collect::<Vec<_>>()
            .join("|")
    }

    /// Hesabı yönetici koduyla bağlayıp onaylar.
    fn baglan(b: &Bot, d: &Path, kim: i64, rol: Rol) {
        let h = Hesaplar { dizin: d };
        let (kod, _) = h
            .kod_uret("", rol, "MRK01", "", "Y-1", b.simdi, 600)
            .unwrap();
        let y = b.eylemler(&msg(kim, &kod));
        assert!(metinler(&y).contains("Bağlandı"), "{y:?}");
        h.onayla(kim, "Y-1", b.bugun).unwrap();
    }

    #[test]
    fn tam_akis() {
        std::env::set_var("TAA_OZELLIK_TELEGRAM_BOT", "1");
        let (d, no) = kur();
        let a = ayar();
        let b = Bot {
            dizin: &d,
            ayar: &a,
            bugun: "2026-11-05",
            simdi: 1_000_000,
        };
        // K1: bağlanmamış hesap sabit metin alır, hiçbir şey kaydedilmez
        let y = b.eylemler(&msg(99, "/durum x"));
        assert!(metinler(&y).contains("onaylı çalışanlar") && !metinler(&y).contains("99"));
        assert!(!d.join("hesaplar.jsonl").exists());
        // yanlış kod yine kayıt yazmaz
        assert!(metinler(&b.eylemler(&msg(99, "00000000"))).contains("kabul edilmedi"));
        assert!(!d.join("hesaplar.jsonl").exists());
        // usta: kodla bağla, onay beklerken bot çalışmaz
        let h = Hesaplar { dizin: &d };
        let (kod, _) = h
            .kod_uret("", Rol::Usta, "MRK01", "", "Y-1", b.simdi, 600)
            .unwrap();
        assert!(metinler(&b.eylemler(&msg(42, &kod))).contains("Bağlandı"));
        assert!(metinler(&b.eylemler(&msg(42, "/durum x"))).contains("onayı bekleniyor"));
        h.onayla(42, "Y-1", "2026-11-05").unwrap();
        // fiş kare koduyla açılır: maskeli özet + her kalem için düğme
        let t = metinler(&b.eylemler(&msg(42, &format!("/start {no}"))));
        assert!(t.contains("A**e Ç***k") && t.contains("0532*****67") && !t.contains("Ayşe"));
        assert!(t.contains(&format!("k:{no}:0")) && t.contains(&format!("k:{no}:1")));
        // K7: kalem seçince onay istenir, imza henüz yok
        let o = metinler(&b.eylemler(&tus(42, &format!("k:{no}:0"))));
        assert!(o.contains("Bitti mi?") && o.contains(&format!("y:{no}:0")));
        assert!(!d.join("imza-olaylari.jsonl").exists());
        // onay: 1. kalem imzalanır, fiş hâlâ hazır değil
        let s = metinler(&b.eylemler(&tus(42, &format!("y:{no}:0"))));
        assert!(
            s.contains("kalem 1 bitti") && !s.contains("Tüm kalemler"),
            "{s}"
        );
        // K2: aynı kalem ikinci kez imzalanamaz; ikinci kalem imzalanabilir
        assert!(metinler(&b.eylemler(&tus(42, &format!("y:{no}:0")))).contains("zaten imzalanmış"));
        let s = metinler(&b.eylemler(&tus(42, &format!("y:{no}:1"))));
        assert!(
            s.contains("Tüm kalemler bitti") && s.contains("kuyruğa yazıldı"),
            "{s}"
        );
        let ol: Vec<Value> = taa_dosya::satirlari_oku(&d.join("imza-olaylari.jsonl"))
            .unwrap()
            .iter()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(ol.len(), 2);
        assert_eq!(ol[0]["tg_id"], 42);
        assert_eq!(ol[0]["fis_no"], no.as_str());
        assert!(ol[0]["usta"].as_str().unwrap().starts_with("U-"));
        assert!(ol[0].get("kullanici_adi").is_none() && !ol[0].to_string().contains("ahmet_x"));
        // usta WhatsApp düğmesini göremez/kullanamaz
        assert!(metinler(&b.eylemler(&tus(42, &format!("w:{no}")))).contains("yalnız mağaza"));
        // mağaza telefonu: usta imzası atamaz, WhatsApp akışı çalışır
        baglan(&b, &d, 77, Rol::Magaza);
        let t = metinler(&b.eylemler(&msg(77, &format!("/start {no}"))));
        assert!(t.contains(&format!("w:{no}")) && !t.contains("Bitti:"));
        assert!(metinler(&b.eylemler(&tus(77, &format!("y:{no}:0")))).contains("yalnız usta"));
        let w = b.eylemler(&tus(77, &format!("w:{no}")));
        let wm = metinler(&w);
        assert!(
            wm.contains("https://sunucu.example/wa/") && wm.contains("g:M-1"),
            "{wm}"
        );
        // müşteri numarası Telegram'a giden hiçbir yöntemde yok
        assert!(!serde_json::to_string(&w).unwrap().contains("905321234567"));
        let belirtec = wm
            .split("/wa/")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .to_string();
        let url = wa_yonlendir(&d, &belirtec, "2026-11-05").unwrap();
        assert!(url.starts_with("https://wa.me/905321234567?text=") && url.contains("F-"));
        // tek kullanımlık, yalnız o gün
        assert!(wa_yonlendir(&d, &belirtec, "2026-11-05")
            .unwrap_err()
            .contains("kullanıldı"));
        assert!(wa_yonlendir(&d, "yok", "2026-11-05").is_err());
        // "gönderdim"
        assert!(metinler(&b.eylemler(&tus(77, "g:M-1"))).contains("gönderildi"));
        let q = taa_bildirim::Kuyruk {
            yol: &d.join("bildirimler.jsonl"),
        };
        assert_eq!(
            q.mesajlar().unwrap()[0].durum,
            taa_bildirim::Durum::Gonderildi
        );
        // yabancı hesap düğmeye basamaz
        let y = b.eylemler(&tus(1234, &format!("y:{no}:0")));
        assert_eq!(y[0]["show_alert"], true);
        // metin olmayan güncelleme yanıtsız
        assert!(b.yanit(&json!({"edited_message": {}})).is_none());
    }

    #[test]
    fn sube_kapsami_baska_subenin_fisini_gostermez_ve_imzalatmaz() {
        std::env::set_var("TAA_OZELLIK_TELEGRAM_BOT", "1");
        let (d, no) = kur();
        // Fişi Bursa şubesine yaz (kur() şubesiz yazar; satırı değiştirmeden yeni fiş ekle).
        let y = d.join("fisler.jsonl");
        let g: Girdi = serde_json::from_str(
            r#"{"sube":"BRS01","satis_danismani":"Mert","olcu_alan":"K","musteri":"Veli Kaya","telefon":"05321234568","kalemler":[{"urun":"Pantolon","islem":"paça kısalt"}],"teslim":"2099-01-01","ucret_kurus":100,"bildirim_izni":true,"tarih":"2026-11-05"}"#,
        )
        .unwrap();
        let bursa = Defter { yol: &y }.ekle(g).unwrap();
        assert_eq!(bursa.sube, "BRS01");
        let a = ayar();
        let b = Bot {
            dizin: &d,
            ayar: &a,
            bugun: "2026-11-05",
            simdi: 1_000_000,
        };
        baglan(&b, &d, 42, Rol::Usta); // hesap şubesi MRK01
                                       // Başka şubenin fişi: var olduğu bile söylenmez, düğme çıkmaz, imza atılamaz.
        let t = metinler(&b.eylemler(&msg(42, &format!("/start {}", bursa.no))));
        assert!(t.contains("fiş yok") && !t.contains("Veli"), "{t}");
        let t = metinler(&b.eylemler(&tus(42, &format!("y:{}:0", bursa.no))));
        assert!(t.contains("fiş yok") || t.contains("bulunamadı"), "{t}");
        assert!(!d.join("imza-olaylari.jsonl").exists());
        // Listeler de yalnız kendi şubesini gösterir (şubesiz eski fiş geçer).
        let l = metinler(&b.eylemler(&msg(42, "/geciken")));
        assert!(!l.contains(&bursa.no), "{l}");
        // Şubesiz (eski) fiş denetime takılmaz.
        assert!(metinler(&b.eylemler(&msg(42, &format!("/start {no}")))).contains("A**e"));
    }

    #[test]
    fn qr_baglantisi_ile_baglama_personel() {
        std::env::set_var("TAA_OZELLIK_TELEGRAM_BOT", "1");
        let (d, no) = kur();
        let a = ayar();
        let b = Bot {
            dizin: &d,
            ayar: &a,
            bugun: "2026-11-05",
            simdi: 4_000_000,
        };
        let h = Hesaplar { dizin: &d };
        let (kod, _) = h
            .kod_uret("", Rol::Personel, "MRK01", "tezgâh", "Y-1", b.simdi, 600)
            .unwrap();
        // QR: telefonda Telegram `/start baglan-<kod>` ile açılır
        let y = metinler(&b.eylemler(&msg(55, &format!("/start baglan-{kod}"))));
        let hs = h.bul(55).unwrap().unwrap();
        assert!(
            y.contains(&format!("Doğrulama sayınız: {}", hs.dogrulama)),
            "{y}"
        );
        assert_eq!(hs.durum, HesapDurum::OnayBekliyor);
        h.onayla(55, "Y-1", "2026-11-05").unwrap();
        // personel fişi görür ama usta imzası ve WhatsApp düğmesi yok
        let t = metinler(&b.eylemler(&msg(55, &format!("/start {no}"))));
        assert!(t.contains("Pantolon") && !t.contains("Bitti:") && !t.contains("w:"));
        assert!(metinler(&b.eylemler(&tus(55, &format!("y:{no}:0")))).contains("yalnız usta"));
        assert!(metinler(&b.eylemler(&tus(55, &format!("w:{no}")))).contains("yalnız mağaza"));
    }

    #[test]
    fn wa_belirteci_gun_degisince_gecersiz() {
        std::env::set_var("TAA_OZELLIK_TELEGRAM_BOT", "1");
        let (d, no) = kur();
        let a = ayar();
        let b = Bot {
            dizin: &d,
            ayar: &a,
            bugun: "2026-11-05",
            simdi: 2_000_000,
        };
        let f = Defter {
            yol: &d.join("fisler.jsonl"),
        };
        f.durum_ata(&no, Durum::Hazir, "2026-11-05").unwrap();
        baglan(&b, &d, 78, Rol::Magaza);
        let w = metinler(&b.eylemler(&tus(78, &format!("w:{no}"))));
        let bel = w
            .split("/wa/")
            .nth(1)
            .unwrap()
            .split('"')
            .next()
            .unwrap()
            .to_string();
        assert!(wa_yonlendir(&d, &bel, "2026-11-06")
            .unwrap_err()
            .contains("süresi"));
    }

    #[test]
    fn hiz_siniri_ve_kod_deneme_siniri() {
        std::env::set_var("TAA_OZELLIK_TELEGRAM_BOT", "1");
        let (d, _) = kur();
        let mut a = ayar();
        a.hiz_siniri = 2;
        let b = Bot {
            dizin: &d,
            ayar: &a,
            bugun: "2026-11-05",
            simdi: 3_000_000,
        };
        baglan(&b, &d, 88, Rol::Usta);
        let mut son = String::new();
        for _ in 0..5 {
            son = metinler(&b.eylemler(&msg(88, "/yardim")));
        }
        assert!(son.contains("Çok hızlı"), "{son}");
        // yanlış kod: saatte en çok 5 deneme
        let mut sonuc = String::new();
        for _ in 0..7 {
            sonuc = metinler(&b.eylemler(&msg(89, "11112222")));
        }
        assert!(sonuc.contains("Çok fazla deneme"), "{sonuc}");
    }

    #[test]
    fn baslangic_kodu_ayiklama() {
        assert_eq!(fis_baglantisi("bot", "F-1"), "https://t.me/bot?start=F-1");
        assert_eq!(
            baslangic_kodu("https://t.me/bot?start=F-20261115-0001").as_deref(),
            Some("F-20261115-0001")
        );
        assert_eq!(baslangic_kodu("F-5").as_deref(), Some("F-5"));
        assert!(baslangic_kodu("../../x").is_none() && baslangic_kodu("").is_none());
    }

    #[test]
    fn maskeler() {
        assert_eq!(maske_ad("Ali Öz"), "A*i Ö*");
        assert_eq!(maske_ad("Ece"), "E*e");
        assert_eq!(maske_tel("0532 123 45 67"), "0532*****67");
        assert_eq!(maske_tel("+90 532 123 45 67"), "9053******67");
        assert_eq!(maske_tel("12"), "**");
    }

    #[test]
    fn ayar_dogrulama() {
        let d = taa_dosya::gecici_klasor().unwrap();
        assert!(Ayar::dizinden(&d).is_err());
        std::fs::write(d.join("telegram.json"), r#"{"gizli":"kisa"}"#).unwrap();
        assert!(Ayar::dizinden(&d).is_err());
        std::fs::write(
            d.join("telegram.json"),
            r#"{"gizli":"0123456789abcdef","kullanicilar":{"1":"A"}}"#,
        )
        .unwrap();
        let a = Ayar::dizinden(&d).unwrap();
        assert_eq!(a.hiz_siniri, 20);
        assert!(
            a.gizli_dogru("0123456789abcdef")
                && !a.gizli_dogru("0123456789abcdee")
                && !a.gizli_dogru("x")
        );
    }

    #[test]
    fn kargo_girisi_telegramdan() {
        std::env::set_var("TAA_OZELLIK_TELEGRAM_BOT", "1");
        let (d, no) = kur();
        let a = ayar();
        let b = Bot { dizin: &d, ayar: &a, bugun: "2026-11-26", simdi: 1_795_700_000 };
        baglan(&b, &d, 11, Rol::Magaza);
        baglan(&b, &d, 22, Rol::Usta);
        // Usta kargo giremez.
        let y = b.eylemler(&msg(22, &format!("/kargo {no}")));
        assert!(metinler(&y).contains("yalnız mağaza telefonu"), "{y:?}");
        // Eksik bilgiyle Onayla çalışmaz: yalnız İptal düğmesi gelir.
        let y = b.eylemler(&msg(11, &format!("/kargo {no}
2 pantolon")));
        let m = metinler(&y);
        assert!(m.contains("Eksik") && !m.contains("kg:ok") && m.contains("kg:iptal"), "{m}");
        let y = b.eylemler(&tus(11, "kg:iptal"));
        assert!(metinler(&y).contains("Vazgeçildi"));
        // Mağaza: komut + serbest metin tek mesajda, Yusuf'un WhatsApp biçimi.
        let metin = format!("/kargo {no}
Ayşe Yılmaz
0532 111 22 33
2 takım elbise (yelekli bej ve haki kruvaze), 2 pantolon (mavi ve siyah)
Büyükçekmece İstanbul");
        let y = b.eylemler(&msg(11, &metin));
        let m = metinler(&y);
        assert!(m.contains("Şunu anladım") && m.contains("Ayşe Yılmaz") && m.contains("2 pantolon (mavi ve siyah)") && m.contains("kg:ok"), "{m}");
        // Onay öncesi kayıt yok.
        assert!(kargo::son(&d, &no).is_none());
        let y = b.eylemler(&tus(11, "kg:ok"));
        let m = metinler(&y);
        assert!(m.contains("Kaydedildi"), "{m}");
        // Varsayılan özet maskeli: tam ad/telefon bot mesajında yok.
        assert!(!m.contains("Ayşe Yılmaz") || m.contains("Satın alan"), "{m}");
        let k = kargo::son(&d, &no).unwrap();
        assert_eq!((k.ilce.as_str(), k.il.as_str(), k.durum.as_str()), ("Büyükçekmece", "İstanbul", "hazirlaniyor"));
        assert_eq!(k.urunler.len(), 2);
        // Takip numarası.
        let y = b.eylemler(&msg(11, &format!("/takip {no} 1234567890 Yurtiçi")));
        assert!(metinler(&y).contains("kargolandı"), "{y:?}");
        let k = kargo::son(&d, &no).unwrap();
        assert_eq!((k.firma.as_str(), k.takip_no.as_str(), k.durum.as_str(), k.kargo_tarihi.as_str()), ("yurtici", "1234567890", "yolda", "2026-11-26"));
        // Geçersiz takip no ve bilinmeyen firma reddedilir.
        assert!(metinler(&b.eylemler(&msg(11, &format!("/takip {no} a b")))).contains("takip numarası"));
        assert!(metinler(&b.eylemler(&msg(11, &format!("/takip {no} 12345678 bilinmeyenfirma")))).contains("firma anlaşılmadı"));
        // Teslim.
        b.eylemler(&msg(11, &format!("/takip {no} teslim")));
        assert_eq!(kargo::son(&d, &no).unwrap().durum, "teslim");
        // Başka şubenin fişi ve olmayan fiş.
        assert!(metinler(&b.eylemler(&msg(11, "/kargo YOK-1 Ali"))).contains("fiş yok"));
    }
}
