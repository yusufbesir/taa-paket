//! Teslim sepeti (Yusuf 2026-11-26: "teslim edilirken de teslim edilen ürünler zincirlenebilsin karekod zinciri ile
//! yani tek karekod ile de teslim edilebilsin, birden fazla karekod da tek teslimatta yapılabilsin").
//!
//! - Tek karekodla zincir: bağlı fişlerden birinin karekodu okutulunca (ya da no yazılınca) bütün bağlı fişler
//!   sepete gelir; arayüzde varsayılan hepsi seçilidir, tek tek çıkarılabilir.
//! - Çok karekodlu sepet: sırayla başka fişler (bağlı olmasalar da) okutulur ya da no yazılır. Aynı fiş ikinci kez
//!   okutulursa yok sayılır (fişsiz satır karekodla okutulursa "fişli"ye yükselir). Teslim edilmiş ya da iptal fiş
//!   sepete girmez; hazır olmayan fiş uyarıyla listelenir ama TESLİM EDİLMEZ (istisna yok).
//! - Tek teslimat: sepetin tamamı tek teslim alan bilgisiyle, tek karekod onayıyla (n = toplam ürün; sepette fişsiz
//!   fiş varsa i = 1) ya da tek cihaz imzasıyla tamamlanır. Her fişe ayrı `TeslimKaydi` yazılır; hepsinde ortak
//!   `teslim_id` ("T-YYYYAAGG-XXXXXX") vardır, `ana_fis` ilk okutulan fiştir. İmza BİR KEZ şifreli yazılır
//!   (`imzalar/<ana>-<n>.png.enc`), bütün kayıtlar aynı dosyaya ve özete başvurur. Şifreli kişisel bilgi bloğu her
//!   fiş için ayrı şifrelenir (aynı düz metin; aad = fiş no, bir fişin bloğu başka fişe taşınamaz).
//! - Ya hepsi ya hiçbiri: bütün denetimler (hazır, yetki, son 4, imza, karekod onayı, şifreleme) YAZMADAN önce
//!   yapılır; kayıtlar `Defter::teslim_et_toplu` ile tek yazımda eklenir. Yazım yine de hata verirse hiçbir fiş
//!   teslim sayılmaz, yeni imza dosyası `imzalar/artik/` altına alınır (silinmez), 500 `teslim_yazilamadi` döner ve
//!   `teslim-sepet.jsonl` günlüğüne (kişisel veri olmadan) yazılır. Defter yalnız eklemeli olduğu için "ters kayıt"
//!   yoktur; tek yazım, satırların yarısının yazılıp yarısının yazılmaması olasılığını en aza indirir.
//!
//! Fişli / fişsiz (karar: Yusuf gözetimsiz, denetçinin önerisi):
//! - Fiş karekodu OKUTULARAK sepete giren fiş "fişli"dir. Kanıt sunucuda yeniden doğrulanır: istemci okunan metni
//!   (`karekod`) gönderir. Müşteri nüshasındaki takip karekodu (`<bulut>/t/<kod>`, kod gizli anahtarla üretilir,
//!   tahmin edilemez) her zaman kanıttır. Personel karekodu (`.../telefon#fis=<no>&k=<kanıt>`; k = yerel gizli
//!   anahtarla HMAC, bkz. `musteri_takip::fis_kaniti`) yalnız müşteri takip AYARLI DEĞİLKEN ve k doğruysa kanıttır
//!   (o zaman müşteri nüshasında bu karekod basılıdır); takip ayarlıyken o karekod yalnız terzihane nüshasındadır,
//!   mağazada durur, kanıt sayılmaz. k'siz/yanlış k'li metin (eski basılı fiş, elle yazılmış "#fis=") fişsizdir.
//! - Sınır: tabanca klavye gibi yazar (okutma ile yazma ayırt edilemez); mağaza cihazı müşteri nüshasını yeniden
//!   yazdırabilir (2026-11-26'dan beri `yeniden-yazdir` yetkisine bağlı, bkz. `yeniden_basim`). Kanıt "karekod
//!   içeriği elde" demektir; mağaza içi güven sınırıdır.
//! - Eski fiş geçiş süresi (yönetici ayarı, varsayılan kapalı): kanıt mekanizmasından önce basılmış fişte kanıtsız
//!   karekod / elle no süre boyunca fişlidir (bkz. `eski_fis_gecis`; satırda `gecis`, günlükte `eski_fis_gecisi`).
//! - Elle no yazılarak ya da bağlı zincirden otomatik gelen ama karekodu okutulmayan fiş "fişsiz"dir.
//! - Sepette en az bir fişsiz fiş varsa: cihazın "fissiz-teslim" yetkisi, her fişsiz müşteri için telefonun son 4
//!   hanesi (aynı telefonlu fişlerde bir kez) ve imza ZORUNLU.
//! - Kasiyerin "fişli" işaretini elle koyma yolu yoktur: `okutuldu: true` karekod kanıtı yoksa yok sayılır.
//! - Bir teslimatta en çok `EN_COK_SEPET` (30) fiş; fazlası kırpılmaz, teslimde sepet tümden reddedilir (413).
//! - Eski `POST /api/fis/<no>/teslim-al` + `"bagli": [..]` isteği buraya çevrilir (bağlı fişler fişsiz).
//!
//! Uçlar (hepsi yetkili cihaz; mağaza PC ya da eşleşmiş telefon):
//! - `POST /api/teslim/karekod-coz {metin}`: okutulan/yazılan metnin fişi ve kanıtı.
//! - `POST /api/teslim/sepet/denetle {fisler, ekle?}`: sepeti çözer, `ekle` metnini (ve bağlı zincirini) ekler.
//! - `POST /api/teslim/sepet/onay-ac {fisler}`: sepet için karekod (belirteç sepet kimliğine bağlı).
//! - `POST /api/teslim/sepet/onay-durum {fisler, t}`: karekod onay durumu.
//! - `POST /api/teslim/sepet {fisler, teslim_alan?, iliski?, onay_belirteci?, imza_png_base64?, telefon_son4?}`.
//!   `fisler`: `[{no, okutuldu, karekod?}]`; `telefon_son4`: `{<no>: "1234"}` (gruptaki herhangi bir fiş no'su).
//!
//! Müşteri takip (bulut): her fiş ayrı ayrı "teslim" durumuna geçer (mevcut `kuyruga_yaz`); teslim alan bilgisi ve
//! teslim_id buluta gitmez.

use super::*;
use crate::teslim_al::{imza_coz_denetle, no_guvenli, sabit_esit, sha256_bayt};

/// Bir sepette en çok bu kadar fiş.
pub(crate) const EN_COK_SEPET: usize = 30;
/// Okutulan metnin en büyük uzunluğu.
const EN_UZUN_METIN: usize = 400;

#[cfg(test)]
thread_local! {
    /// Test: doğrulamadan sonra deftere yazım hatası benzetimi (yalnız bu iş parçacığında).
    pub(crate) static SEPET_YAZIM_HATASI: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Test: eski fiş geçişi günlüğü yazılamıyor benzetimi.
    pub(crate) static GECIS_GUNLUK_HATASI: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Okutulan metnin kanıt türü.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Kanit {
    /// Müşteri nüshasındaki takip karekodu (gizli anahtarlı kod).
    Takip,
    /// Personel karekodu (`#fis=<no>&k=<kanıt>`), kanıt parçası doğru.
    FisKarekodu,
    /// Personel karekodu ama kanıt parçası yok ya da yanlış (eski basılı fiş, ya da elle yazılmış "#fis=<no>").
    KarekodKanitsiz,
    /// Elle yazılan no (ya da zincirden gelen).
    Elle,
}

impl Kanit {
    pub(crate) fn ad(self) -> &'static str {
        match self {
            Kanit::Takip => "takip",
            Kanit::FisKarekodu => "fis-karekodu",
            Kanit::KarekodKanitsiz => "karekod-kanitsiz",
            Kanit::Elle => "elle",
        }
    }
}

/// Metinden fiş no adayı: "F-20261126-001", "F20261126001", "f.20261126.001" gibi.
fn f_deseni(m: &str) -> Option<String> {
    let b = m.as_bytes();
    for i in 0..b.len() {
        if !b[i].eq_ignore_ascii_case(&b'F') {
            continue;
        }
        let mut j = i + 1;
        if j < b.len() && !b[j].is_ascii_digit() {
            j += 1;
        }
        let gun = b.get(j..j + 8)?;
        if !gun.iter().all(u8::is_ascii_digit) {
            continue;
        }
        let mut k = j + 8;
        if k < b.len() && !b[k].is_ascii_digit() {
            k += 1;
        }
        let Some(sira) = b.get(k..k + 3) else {
            continue;
        };
        if sira.iter().all(u8::is_ascii_digit) && b.get(k + 3).is_none_or(|x| !x.is_ascii_digit()) {
            return Some(format!(
                "F-{}-{}",
                std::str::from_utf8(gun).ok()?,
                std::str::from_utf8(sira).ok()?
            ));
        }
    }
    None
}

/// Takip adresinden ("…/t/<kod>") ya da çıplak koddan kod.
fn takip_kodu_ayikla(m: &str) -> Option<&str> {
    if musteri_takip::kod_gecerli(m) {
        return Some(m);
    }
    let (_, son) = m.rsplit_once("/t/")?;
    let kod = son.split(['?', '#', '/', '&']).next().unwrap_or("");
    musteri_takip::kod_gecerli(kod).then_some(kod)
}

/// Doğrulanacak son 4: (grup (anahtar, no'lar), yazılan değer, fişteki telefon rakamları).
type Son4Degeri<'a> = (&'a (String, Vec<String>), String, Vec<u8>);

/// Bir sepet satırının iç hâli.
struct Satir {
    no: String,
    okutuldu: bool,
    kanit: Kanit,
    /// Kanıt olan okunmuş metin (yalnız okutulduysa; istemci sonraki isteklerde geri gönderir).
    karekod: String,
    /// Eski fiş geçiş süresiyle fişli sayıldı (bkz. `eski_fis_gecis`; teslimde günlüğe yazılır).
    gecis: bool,
    fis: taa_fis::Fis,
}

/// Çözülmüş sepet.
#[derive(Default)]
struct Sepet {
    satirlar: Vec<Satir>,
    /// Çift okutma / çift satır: yok sayıldı.
    yok_sayilan: Vec<String>,
    /// Sepete girmeyen: (no, neden) — fiş yok, teslim edilmiş, iptal, sepet sınırı aşıldı.
    reddedilen: Vec<(String, String)>,
    /// Sepet sınırını (`EN_COK_SEPET`) aşan fişler: sepete girmez; teslimde/karekodda sepet TÜMDEN reddedilir (413).
    fazla: Vec<String>,
    /// `okutuldu: true` denildi ama karekod kanıtı yok: fişsiz sayıldı.
    kanitsiz: Vec<String>,
}

impl Sepet {
    fn hazirlar(&self) -> impl Iterator<Item = &Satir> {
        self.satirlar.iter().filter(|s| hazir(&s.fis))
    }
}

fn hazir(f: &taa_fis::Fis) -> bool {
    f.durum == Durum::Hazir && f.teslim_kaydi.is_none()
}

fn urun_sayisi(f: &taa_fis::Fis) -> u32 {
    f.kalemler.iter().map(|k| k.adet.unwrap_or(1).max(1)).sum()
}

fn durum_adi(d: Durum) -> &'static str {
    match d {
        Durum::Bekliyor => "bekliyor",
        Durum::Terzihanede => "terzihanede",
        Durum::Hazir => "hazir",
        Durum::Teslim => "teslim",
        Durum::Iptal => "iptal",
    }
}

/// Sepet kimliği: sıralı fiş no'larının özeti. Karekod belirteci buna bağlanır; sepet değişirse onay geçmez.
pub(crate) fn sepet_kimligi(nolar: &[&str]) -> String {
    let mut v: Vec<&str> = nolar.to_vec();
    v.sort_unstable();
    format!("S-{}", &sha256_bayt(v.join(",").as_bytes())[..16])
}

/// Telefonun karşılaştırma biçimi: yalnız rakamlar, son 10 hane (0532… ile +90532… aynı sayılır).
fn tel_anahtari(t: &str) -> String {
    let r: Vec<u8> = t.bytes().filter(u8::is_ascii_digit).collect();
    let bas = r.len().saturating_sub(10);
    String::from_utf8(r[bas..].to_vec()).unwrap_or_default()
}

/// İstemcinin gönderdiği sepet satırı.
struct Girdi {
    no: String,
    okutuldu: bool,
    karekod: String,
}

fn girdileri_oku(v: &serde_json::Value) -> Result<Vec<Girdi>, Yanit> {
    let Some(a) = v["fisler"].as_array() else {
        return Err(Yanit::hata(400, "fisler dizisi bekleniyor"));
    };
    if a.len() > EN_COK_SEPET * 2 {
        return Err(Yanit::hata(413, "sepette çok fazla satır"));
    }
    let mut g = Vec::with_capacity(a.len());
    for x in a {
        // Kısa biçim: yalnız fiş no (fişsiz sayılır).
        let (no, okutuldu, karekod) = match x {
            serde_json::Value::String(s) => (s.trim().to_string(), false, String::new()),
            serde_json::Value::Object(_) => (
                x["no"].as_str().unwrap_or("").trim().to_string(),
                x["okutuldu"].as_bool().unwrap_or(false),
                x["karekod"]
                    .as_str()
                    .unwrap_or("")
                    .trim()
                    .chars()
                    .take(EN_UZUN_METIN)
                    .collect(),
            ),
            _ => {
                return Err(Yanit::hata(
                    400,
                    "fisler: {no, okutuldu, karekod?} nesneleri",
                ))
            }
        };
        if !no_guvenli(&no) {
            return Err(Yanit::hata(400, "fiş no geçersiz"));
        }
        g.push(Girdi {
            no,
            okutuldu,
            karekod,
        });
    }
    Ok(g)
}

impl Uygulama {
    fn takip_ayarli(&self) -> bool {
        musteri_takip::Ayar::dizinden(&self.dizin).is_some()
    }

    /// Okutulan/yazılan metnin fişi ve kanıtı. Fiş bu şubede yoksa `None`.
    fn metin_coz(&self, fisler: &[taa_fis::Fis], metin: &str) -> Option<(String, Kanit)> {
        let m = metin.trim();
        if m.is_empty() || m.len() > EN_UZUN_METIN {
            return None;
        }
        if let Some(kod) = takip_kodu_ayikla(m) {
            if self.takip_ayarli() {
                if let Some(no) = musteri_takip::takip_kodundan_fis(
                    &self.dizin,
                    kod,
                    fisler.iter().map(|f| f.no.as_str()),
                ) {
                    return Some((no.to_string(), Kanit::Takip));
                }
            }
            // Çıplak 16 harflik kod başka bir şey değildir; adres ise aşağıda fiş no aranmaz.
            return None;
        }
        if let Some((_, son)) = m.split_once("#fis=") {
            let no: String = son
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '-')
                .collect();
            if !fisler.iter().any(|f| f.no == no) {
                return None;
            }
            // Gizli anahtarlı kanıt parçası (`&k=`): yalnız o tutarsa karekod okutulmuş sayılır. "#fis=<no>" metni
            // gizli bir şey içermez, elle yazılabilir; k'siz (eski basılı) fiş fişsiz muamelesi görür.
            let k: String = son
                .split('&')
                .skip(1)
                .find_map(|p| p.strip_prefix("k="))
                .unwrap_or("")
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect();
            let kanit = if musteri_takip::fis_kaniti_dogru(&self.dizin, &no, &k) {
                Kanit::FisKarekodu
            } else {
                Kanit::KarekodKanitsiz
            };
            return Some((no, kanit));
        }
        let aday = if let Some(no) = f_deseni(m) {
            no
        } else if (1..=3).contains(&m.len()) && m.bytes().all(|b| b.is_ascii_digit()) {
            // Kısa yazım: bugünün fişi ("7" -> F-<bugün>-007).
            format!("F-{}-{:0>3}", tarih_iso(self.simdi()).replace('-', ""), m)
        } else {
            m.to_string()
        };
        fisler
            .iter()
            .any(|f| f.no == aday)
            .then_some((aday, Kanit::Elle))
    }

    /// Kanıt "fişli" sayılır mı? Takip kodu her zaman; kanıt parçası doğru personel karekodu yalnız takip ayarlı
    /// değilken (takip ayarlıyken o karekod terzihane nüshasındadır, mağazada durur).
    pub(crate) fn kanit_fisli(&self, k: Kanit) -> bool {
        match k {
            Kanit::Takip => true,
            Kanit::FisKarekodu => !self.takip_ayarli(),
            Kanit::KarekodKanitsiz | Kanit::Elle => false,
        }
    }

    /// teslim-al için: `fis_karekodu` metni bu fişin karekod kanıtı mı? `(fişli, geçişle)`: ikincisi, fişli sayılması
    /// yalnız eski fiş geçiş süresinden geliyorsa `true` (günlük, teslim kaydından hemen önce yazılır; burada değil).
    pub(crate) fn fis_karekodu_kaniti(&self, d: &Defter, no: &str, metin: &str) -> (bool, bool) {
        if metin.trim().is_empty() {
            return (false, false);
        }
        let Ok(fisler) = d.fisler() else {
            return (false, false);
        };
        match self.metin_coz(&fisler, metin) {
            Some((n, k)) if n == no => {
                if self.kanit_fisli(k) {
                    return (true, false);
                }
                let gecis = fisler
                    .iter()
                    .find(|f| f.no == no)
                    .is_some_and(|f| self.gecis_fisli(k, f));
                (gecis, gecis)
            }
            _ => (false, false),
        }
    }

    /// Girdileri sepete çevirir: fiş bulunur, çift satır birleşir, kanıt yeniden doğrulanır.
    fn sepet_coz(&self, fisler: &[taa_fis::Fis], girdiler: Vec<Girdi>) -> Sepet {
        let mut s = Sepet::default();
        for g in girdiler {
            let Some(f) = fisler.iter().find(|f| f.no == g.no) else {
                if !s.reddedilen.iter().any(|(n, _)| *n == g.no) {
                    s.reddedilen
                        .push((g.no, "bu şubede böyle bir fiş yok".into()));
                }
                continue;
            };
            let kanit = if g.karekod.is_empty() {
                None
            } else {
                match self.metin_coz(fisler, &g.karekod) {
                    Some((n, k)) if n == g.no => Some(k),
                    _ => None,
                }
            };
            let normal = kanit.is_some_and(|k| self.kanit_fisli(k));
            // Eski fiş geçiş süresi açıksa, kanıt mekanizmasından önce basılmış fişte kanıtsız karekod / elle no fişlidir.
            let gecis = !normal && kanit.is_some_and(|k| self.gecis_fisli(k, f));
            let okutuldu = normal || gecis;
            if g.okutuldu && !okutuldu && !s.kanitsiz.contains(&g.no) {
                s.kanitsiz.push(g.no.clone());
            }
            if let Some(var) = s.satirlar.iter_mut().find(|x| x.no == g.no) {
                s.yok_sayilan.push(g.no.clone());
                if okutuldu && !var.okutuldu {
                    var.okutuldu = true;
                    var.gecis = gecis;
                    var.kanit = kanit.unwrap_or(Kanit::Elle);
                    var.karekod = g.karekod;
                }
                continue;
            }
            if matches!(f.durum, Durum::Teslim | Durum::Iptal) || f.teslim_kaydi.is_some() {
                let neden = if f.durum == Durum::Iptal {
                    "iptal edilmiş fiş teslim edilemez"
                } else {
                    "bu fiş zaten teslim edilmiş"
                };
                if !s.reddedilen.iter().any(|(n, _)| *n == g.no) {
                    s.reddedilen.push((g.no, neden.into()));
                }
                continue;
            }
            if s.satirlar.len() >= EN_COK_SEPET {
                // Sessiz kırpma yok: fazlası bildirilir, teslimde sepet tümden reddedilir.
                if !s.fazla.contains(&g.no) {
                    s.reddedilen.push((
                        g.no.clone(),
                        format!("sepet sınırı aşıldı (bir teslimatta en çok {EN_COK_SEPET} fiş)"),
                    ));
                    s.fazla.push(g.no);
                }
                continue;
            }
            s.satirlar.push(Satir {
                no: g.no,
                okutuldu,
                kanit: kanit.unwrap_or(Kanit::Elle),
                karekod: if okutuldu { g.karekod } else { String::new() },
                gecis,
                fis: f.clone(),
            });
        }
        s
    }

    /// Fişsiz hazır satırların telefon grupları: (anahtar fiş no, gruptaki no'lar). Aynı telefonlu fişlerde son 4
    /// bir kez sorulur.
    fn son4_gruplari(s: &Sepet) -> Vec<(String, Vec<String>)> {
        let mut g: Vec<(String, String, Vec<String>)> = Vec::new();
        for x in s.hazirlar().filter(|x| !x.okutuldu) {
            let t = tel_anahtari(&x.fis.telefon);
            match g.iter_mut().find(|(k, _, _)| !t.is_empty() && *k == t) {
                Some((_, _, l)) => l.push(x.no.clone()),
                None => g.push((t, x.no.clone(), vec![x.no.clone()])),
            }
        }
        g.into_iter().map(|(_, a, l)| (a, l)).collect()
    }

    fn satir_json(&self, x: &Satir) -> serde_json::Value {
        let h = hazir(&x.fis);
        let mut v = json!({
            "no": x.no,
            "musteri": x.fis.musteri,
            "urun_sayisi": urun_sayisi(&x.fis),
            "durum": durum_adi(x.fis.durum),
            "hazir": h,
            "okutuldu": x.okutuldu,
            "kanit": x.kanit.ad(),
            "bagli": x.fis.bagli,
            "gecis": x.gecis,
        });
        if x.okutuldu {
            v["karekod"] = json!(x.karekod);
        }
        if !h {
            v["uyari"] = json!(format!(
                "Hazır değil ({}); teslim edilmez",
                durum_adi(x.fis.durum)
            ));
        }
        v
    }

    /// Sepetin istemciye giden görünümü.
    fn sepet_json(&self, s: &Sepet, c: &cihaz::Cihaz) -> serde_json::Value {
        let hazir_nolar: Vec<&str> = s.hazirlar().map(|x| x.no.as_str()).collect();
        let fissiz: Vec<&str> = s
            .hazirlar()
            .filter(|x| !x.okutuldu)
            .map(|x| x.no.as_str())
            .collect();
        let gruplar: Vec<_> = Self::son4_gruplari(s)
            .into_iter()
            .map(|(a, l)| {
                let musteri = s
                    .satirlar
                    .iter()
                    .find(|x| x.no == a)
                    .map(|x| x.fis.musteri.clone())
                    .unwrap_or_default();
                json!({ "anahtar": a, "fisler": l, "musteri": musteri })
            })
            .collect();
        let mut uyarilar: Vec<String> = Vec::new();
        for x in s.satirlar.iter().filter(|x| !hazir(&x.fis)) {
            uyarilar.push(format!(
                "{} hazır değil ({}); teslim edilmeyecek.",
                x.no,
                durum_adi(x.fis.durum)
            ));
        }
        for (n, e) in &s.reddedilen {
            uyarilar.push(format!("{n}: {e}; sepete eklenmedi."));
        }
        let gecisli: Vec<&str> = s
            .satirlar
            .iter()
            .filter(|x| x.gecis)
            .map(|x| x.no.as_str())
            .collect();
        if !gecisli.is_empty() {
            uyarilar.push(format!(
                "Eski fiş geçiş süresi: {} kanıtsız karekod / elle no ile fişli sayıldı (günlüğe yazılır).",
                gecisli.join(", ")
            ));
        }
        if !s.kanitsiz.is_empty() {
            uyarilar.push(format!(
                "Fiş karekodu okutulmadığı için fişsiz sayıldı: {}.",
                s.kanitsiz.join(", ")
            ));
        }
        let yetki = self.cihazlar().yetki_var(&c.id, "fissiz-teslim");
        if !fissiz.is_empty() && !yetki {
            uyarilar.push("Sepette fişsiz fiş var ve bu cihazın fişsiz teslim yetkisi yok.".into());
        }
        json!({
            "fisler": s.satirlar.iter().map(|x| self.satir_json(x)).collect::<Vec<_>>(),
            "toplam_urun": s.hazirlar().map(|x| urun_sayisi(&x.fis)).sum::<u32>(),
            "hazir_sayisi": hazir_nolar.len(),
            "fissiz": fissiz,
            "fissiz_var": !fissiz.is_empty(),
            "fissiz_yetki": yetki,
            "imza_zorunlu": !fissiz.is_empty(),
            "son4_gruplari": gruplar,
            "sepet_kimligi": if hazir_nolar.is_empty() { String::new() } else { sepet_kimligi(&hazir_nolar) },
            "yok_sayilan": s.yok_sayilan,
            "reddedilen": s.reddedilen.iter().map(|(n, e)| json!({ "no": n, "neden": e })).collect::<Vec<_>>(),
            "kanitsiz": s.kanitsiz,
            "uyarilar": uyarilar,
            "en_cok": EN_COK_SEPET,
            "fazla": s.fazla,
            "dolu": s.satirlar.len() >= EN_COK_SEPET,
        })
    }

    /// POST /api/teslim/karekod-coz {metin}: fiş aç / tabanca için okutulan metnin fişi ve kanıtı.
    pub(crate) fn karekod_coz(&self, d: &Defter, i: &Istek) -> Yanit {
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let metin = v["metin"].as_str().unwrap_or("");
        let fisler = match d.fisler() {
            Ok(f) => f,
            Err(e) => return Yanit::hata(500, &e),
        };
        match self.metin_coz(&fisler, metin) {
            Some((no, k)) => {
                let fisli = self.kanit_fisli(k);
                let gecis = !fisli
                    && fisler
                        .iter()
                        .find(|f| f.no == no)
                        .is_some_and(|f| self.gecis_fisli(k, f));
                Yanit::json(
                    200,
                    json!({
                        "no": no,
                        "kanit": k.ad(),
                        "okutuldu": fisli || gecis,
                        "gecis": gecis,
                        "not": if gecis { "Eski fiş (kanıt mekanizmasından önce basılmış): geçiş süresi boyunca fişli sayılır." } else { match k {
                            Kanit::Takip => "Müşteri nüshasının karekodu okutuldu (fişli).",
                            Kanit::FisKarekodu if fisli => "Fiş karekodu okutuldu (fişli).",
                            Kanit::FisKarekodu => "Bu, terzihane nüshasının karekodu: müşterinin fişi sayılmaz (fişsiz).",
                            Kanit::KarekodKanitsiz => "Karekodda kanıt parçası yok ya da yanlış (eski basılı fiş ya da elle yazılmış): fişsiz.",
                            Kanit::Elle => "Numara yazıldı: fiş okutulmadı (fişsiz).",
                        } },
                    }),
                )
            }
            None => Yanit::json(
                404,
                json!({ "hata": "fis_bulunamadi", "ileti": "Bu kod bu şubedeki bir fişe ait değil." }),
            ),
        }
    }

    /// POST /api/teslim/sepet/denetle {fisler, ekle?}: sepeti çözer; `ekle` (okutulan/yazılan metin) varsa o fişi ve
    /// bağlı zincirini (karekodu okutulmadıkları için fişsiz) ekler.
    pub(crate) fn sepet_denetle(&self, d: &Defter, c: &cihaz::Cihaz, i: &Istek) -> Yanit {
        let v: serde_json::Value = match serde_json::from_slice(&i.govde) {
            Ok(v @ serde_json::Value::Object(_)) => v,
            _ => return Yanit::hata(400, "geçersiz istek (JSON nesnesi bekleniyor)"),
        };
        let mut girdiler = if v["fisler"].is_null() {
            Vec::new()
        } else {
            match girdileri_oku(&v) {
                Ok(g) => g,
                Err(y) => return y,
            }
        };
        let fisler = match d.fisler() {
            Ok(f) => f,
            Err(e) => return Yanit::hata(500, &e),
        };
        let mut eklenen: Vec<String> = Vec::new();
        let mut ek_uyari: Option<String> = None;
        let mut zaten = false;
        let mut kanit_notu = String::new();
        if let Some(m) = v["ekle"].as_str().filter(|m| !m.trim().is_empty()) {
            match self.metin_coz(&fisler, m) {
                None => {
                    ek_uyari = Some(format!(
                        "\"{}\" bu şubedeki bir fişe ait değil.",
                        m.trim().chars().take(40).collect::<String>()
                    ))
                }
                Some((no, k)) => {
                    let gecis = !self.kanit_fisli(k)
                        && fisler
                            .iter()
                            .find(|f| f.no == no)
                            .is_some_and(|f| self.gecis_fisli(k, f));
                    let fisli = self.kanit_fisli(k) || gecis;
                    if gecis {
                        kanit_notu = format!(
                            "{no}: eski fiş (kanıt mekanizmasından önce basılmış); geçiş süresi boyunca fişli sayıldı."
                        );
                    } else if k == Kanit::FisKarekodu && !fisli {
                        kanit_notu = format!(
                            "{no}: terzihane nüshasının karekodu okutuldu; müşterinin fişi sayılmaz (fişsiz)."
                        );
                    } else if k == Kanit::KarekodKanitsiz {
                        kanit_notu = format!(
                            "{no}: karekodda kanıt parçası yok ya da yanlış (eski basılı fiş ya da elle yazılmış); fişsiz sayıldı."
                        );
                    }
                    zaten = girdiler.iter().any(|g| g.no == no);
                    if !zaten {
                        eklenen.push(no.clone());
                    }
                    // Okunan metin her durumda verilir: sepet_coz kanıtı yeniden doğrular, fişli değilse metni
                    // satırda tutmaz ama kanıt türünü (ör. karekod-kanitsiz) gösterir.
                    girdiler.push(Girdi {
                        no: no.clone(),
                        okutuldu: fisli,
                        karekod: m.trim().chars().take(EN_UZUN_METIN).collect(),
                    });
                    // Bağlı zincir: bütün bağlı fişler gelir (karekodları okutulmadı: fişsiz).
                    if let Some(f) = fisler.iter().find(|f| f.no == no) {
                        for b in &f.bagli {
                            if !girdiler.iter().any(|g| g.no == *b) {
                                if fisler.iter().any(|x| {
                                    x.no == *b && !matches!(x.durum, Durum::Teslim | Durum::Iptal)
                                }) {
                                    eklenen.push(b.clone());
                                }
                                girdiler.push(Girdi {
                                    no: b.clone(),
                                    okutuldu: false,
                                    karekod: String::new(),
                                });
                            }
                        }
                    }
                }
            }
        }
        let s = self.sepet_coz(&fisler, girdiler);
        let mut j = self.sepet_json(&s, c);
        // Zincirden gelen teslim edilmiş/iptal bağlı fişler sessizce atlanır (uyarı kalabalığı olmasın).
        j["eklenen"] = json!(eklenen
            .iter()
            .filter(|n| s.satirlar.iter().any(|x| x.no == **n))
            .collect::<Vec<_>>());
        j["zaten_sepette"] = json!(zaten);
        if ek_uyari.is_none() && eklenen.first().is_some_and(|n| s.fazla.contains(n)) {
            ek_uyari = Some(format!(
                "Sepet dolu: bir teslimatta en çok {EN_COK_SEPET} fiş. Önce bu sepeti teslim edin."
            ));
        }
        if let Some(u) = ek_uyari {
            j["ekle_hatasi"] = json!(u);
        }
        if !kanit_notu.is_empty() {
            j["kanit_notu"] = json!(kanit_notu);
        }
        Yanit::json(200, j)
    }

    /// Teslim/karekod için sepet: hepsi hazır olmalı (değilse 409, hiçbiri teslim edilmez).
    fn sepet_teslim_icin(&self, d: &Defter, v: &serde_json::Value) -> Result<Sepet, Yanit> {
        let girdiler = girdileri_oku(v)?;
        if girdiler.is_empty() {
            return Err(Yanit::hata(400, "sepet boş"));
        }
        let fisler = d.fisler().map_err(|e| Yanit::hata(500, &e))?;
        let s = self.sepet_coz(&fisler, girdiler);
        if !s.fazla.is_empty() {
            return Err(Yanit::json(
                413,
                json!({
                    "hata": "sepet_cok_buyuk",
                    "ileti": format!("Bir teslimatta en çok {EN_COK_SEPET} fiş teslim edilir; sepette {} fiş var. Sepeti bölün; hiçbir fiş teslim edilmedi.", s.satirlar.len() + s.fazla.len()),
                    "en_cok": EN_COK_SEPET,
                    "fazla": s.fazla,
                }),
            ));
        }
        if !s.reddedilen.is_empty() {
            return Err(Yanit::json(
                409,
                json!({
                    "hata": "sepet_gecersiz",
                    "ileti": "Sepette teslim edilemeyecek fiş var; hiçbir fiş teslim edilmedi.",
                    "reddedilen": s.reddedilen.iter().map(|(n, e)| json!({ "no": n, "neden": e })).collect::<Vec<_>>(),
                }),
            ));
        }
        let hazir_degil: Vec<_> = s
            .satirlar
            .iter()
            .filter(|x| !hazir(&x.fis))
            .map(|x| json!({ "no": x.no, "durum": durum_adi(x.fis.durum) }))
            .collect();
        if !hazir_degil.is_empty() {
            return Err(Yanit::json(
                409,
                json!({
                    "hata": "hazir_degil",
                    "ileti": "Yalnız hazır fişler teslim edilir. Hazır olmayanları sepetten çıkarın; hiçbir fiş teslim edilmedi.",
                    "fisler": hazir_degil,
                }),
            ));
        }
        Ok(s)
    }

    /// Karekoddaki telefonun sepetteki fiş telefonlarıyla karşılaştırması.
    fn sepet_tel_karsilastir(s: &Sepet, qr_tel: &str) -> &'static str {
        let fis_tel: Vec<String> = s
            .satirlar
            .iter()
            .map(|x| teslim_sifre::tel_normal(&x.fis.telefon).unwrap_or_default())
            .filter(|t| !t.is_empty())
            .collect();
        if fis_tel.is_empty() || qr_tel.is_empty() {
            return "yok";
        }
        let ayni = fis_tel.iter().filter(|t| *t == qr_tel).count();
        if ayni == fis_tel.len() {
            "ayni"
        } else if ayni == 0 {
            "farkli"
        } else {
            "kismen"
        }
    }

    /// POST /api/teslim/sepet/onay-ac {fisler}: sepetin tamamı için karekod. n = toplam ürün, sepette fişsiz fiş
    /// varsa müşteriden imza istenir (i = 1). Belirteç sepet kimliğine ve cihaza bağlıdır.
    pub(crate) fn sepet_onay_ac(&self, d: &Defter, c: &cihaz::Cihaz, i: &Istek) -> Yanit {
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let s = match self.sepet_teslim_icin(d, &v) {
            Ok(s) => s,
            Err(y) => return y,
        };
        let nolar: Vec<&str> = s.satirlar.iter().map(|x| x.no.as_str()).collect();
        let kimlik = sepet_kimligi(&nolar);
        let n: u32 = s.satirlar.iter().map(|x| urun_sayisi(&x.fis)).sum();
        let fissiz = s.satirlar.iter().any(|x| !x.okutuldu);
        let mut y = self.onay_ac_ic(&kimlik, n, fissiz, c);
        if y.kod == 200 {
            if let Ok(mut j) = serde_json::from_slice::<serde_json::Value>(&y.govde) {
                j["sepet_kimligi"] = json!(kimlik);
                j["toplam_urun"] = json!(n);
                y = Yanit::json(200, j);
            }
        }
        y
    }

    /// POST /api/teslim/sepet/onay-durum {fisler, t}: sepet karekodunun onay durumu.
    pub(crate) fn sepet_onay_durum(&self, d: &Defter, c: &cihaz::Cihaz, i: &Istek) -> Yanit {
        let v: serde_json::Value = serde_json::from_slice(&i.govde).unwrap_or_default();
        let s = match self.sepet_teslim_icin(d, &v) {
            Ok(s) => s,
            Err(y) => return y,
        };
        let nolar: Vec<&str> = s.satirlar.iter().map(|x| x.no.as_str()).collect();
        let kimlik = sepet_kimligi(&nolar);
        let t = v["t"].as_str().unwrap_or("");
        self.onay_durum_ic(&kimlik, t, c, &|tel| Self::sepet_tel_karsilastir(&s, tel))
    }

    /// Yeni ortak teslim kimliği: "T-YYYYAAGG-XXXXXX" (rastgele 6 karakter, l/o/0/1 yok); defterde varsa yeniden.
    pub(crate) fn yeni_teslim_id(&self, d: &Defter) -> Result<String, String> {
        let gun = tarih_iso(self.simdi()).replace('-', "");
        let kullanilan: Vec<String> = d
            .fisler()?
            .into_iter()
            .filter_map(|f| f.teslim_kaydi.map(|t| t.teslim_id))
            .filter(|t| !t.is_empty())
            .collect();
        for _ in 0..8 {
            let r: String = musteri_takip::rastgele(6)?
                .iter()
                .map(|b| musteri_takip::ALFABE[(b & 31) as usize].to_ascii_uppercase() as char)
                .collect();
            let t = format!("T-{gun}-{r}");
            if !kullanilan.contains(&t) {
                return Ok(t);
            }
        }
        Err("teslim kimliği üretilemedi".into())
    }

    /// Kişisel veri İÇERMEYEN sepet günlüğü (teslim_id, fiş no'ları, sonuç, kim). Yazılamazsa yalnız uyarı.
    fn sepet_gunlugu(&self, v: serde_json::Value) {
        let _ = self.sepet_gunlugu_yaz(v);
    }

    /// Teslim sepeti günlüğüne yazar; yazılamazsa `Err` (zorunlu satırlar için: eski fiş geçişi).
    pub(crate) fn sepet_gunlugu_yaz(&self, v: serde_json::Value) -> Result<(), String> {
        let s = self.simdi();
        let mut v = v;
        v["tarih"] = json!(tarih_iso(s));
        v["saat"] = json!(saat_iso(s));
        #[cfg(test)]
        let deneme = GECIS_GUNLUK_HATASI.with(|h| h.get()) && v["olay"] == "eski_fis_gecisi";
        #[cfg(not(test))]
        let deneme = false;
        let r = if deneme {
            Err("deneme: günlük yazılamadı".to_string())
        } else {
            taa_dosya::satir_ekle(&self.dizin.join("teslim-sepet.jsonl"), &v.to_string())
                .map_err(|e| e.to_string())
        };
        match &r {
            Err(e) => eprintln!("Teslim sepeti günlüğü yazılamadı: {e}"),
            Ok(()) => eprintln!("Teslim sepeti: {v}"),
        }
        r
    }

    /// POST /api/teslim/sepet: sepetin tamamını TEK teslimatta teslim eder (bkz. modül açıklaması).
    pub(crate) fn teslim_sepet(&self, d: &Defter, c: &cihaz::Cihaz, i: &Istek) -> Yanit {
        match serde_json::from_slice(&i.govde) {
            Ok(v @ serde_json::Value::Object(_)) => self.teslim_sepet_ic(d, c, &v),
            _ => Yanit::hata(400, "geçersiz istek (JSON nesnesi bekleniyor)"),
        }
    }

    /// Sepet teslimi (gövde çözülmüş). Eski `teslim-al` + `"bagli"` isteği de buraya çevrilir.
    pub(crate) fn teslim_sepet_ic(
        &self,
        d: &Defter,
        c: &cihaz::Cihaz,
        v: &serde_json::Value,
    ) -> Yanit {
        let v = v.clone();
        let s = match self.sepet_teslim_icin(d, &v) {
            Ok(s) => s,
            Err(y) => return y,
        };
        let nolar: Vec<String> = s.satirlar.iter().map(|x| x.no.clone()).collect();
        let fissiz: Vec<&str> = s
            .satirlar
            .iter()
            .filter(|x| !x.okutuldu)
            .map(|x| x.no.as_str())
            .collect();
        if !fissiz.is_empty() && !self.cihazlar().yetki_var(&c.id, "fissiz-teslim") {
            return Yanit::json(
                403,
                json!({
                    "hata": "yetki_yok",
                    "ileti": "Sepette fiş karekodu okutulmamış (fişsiz) fiş var ve bu cihazın fişsiz teslim yetkisi yok. Müşteri nüshasının karekodunu okutun ya da mağaza bilgisayarından teslim edin.",
                    "fissiz": fissiz,
                }),
            );
        }
        // Karekod onayı: belirteç sepet kimliğine ve bu cihaza bağlı olmalı.
        let refs: Vec<&str> = nolar.iter().map(String::as_str).collect();
        let kimlik = sepet_kimligi(&refs);
        let belirtec = v["onay_belirteci"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let onay = match &belirtec {
            Some(t) => match self.qr_onayi(&kimlik, t, c) {
                Ok(b) => Some(b),
                Err(y) => return y,
            },
            None => None,
        };
        if let Some(b) = &onay {
            if matches!(Self::sepet_tel_karsilastir(&s, &b.tel), "farkli" | "kismen")
                && !matches!(v["iliski"].as_str(), Some("yakini") | Some("diger"))
            {
                return Yanit::json(
                    422,
                    json!({
                        "hata": "iliski_gerekli",
                        "ileti": "Karekodda yazılan telefon sepetteki fişlerin (bazılarının) numarasından farklı. Teslim alanın müşteriye yakınlığını seçin (yakını ya da diğer).",
                    }),
                );
            }
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
        // İmza (son 4 hakkı boşa gitmesin diye önce): karekod imzası, yoksa cihaz ekranı.
        let imza: Option<(Vec<u8>, &'static str)> = match onay.as_ref().and_then(|b| b.imza.clone())
        {
            Some(b) => Some((b, "qr_imza")),
            None => match v["imza_png_base64"].as_str().filter(|s| !s.is_empty()) {
                None => None,
                Some(x) => match imza_coz_denetle(x) {
                    Ok(b) => Some((b, "cihaz_imza")),
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
        if !fissiz.is_empty() {
            if imza.is_none() {
                return Yanit::json(
                    422,
                    json!({
                        "hata": "imza_gerekli",
                        "ileti": "Sepette fişsiz fiş var: imza zorunlu. Teslim alana ekranda imzalatın (ya da karekod sayfasında imzalasın).",
                        "fissiz": fissiz,
                    }),
                );
            }
            if let Err(y) = self.sepet_son4_dogrula(&s, &v) {
                return y;
            }
        }
        let teslim_id = match self.yeni_teslim_id(d) {
            Ok(t) => t,
            Err(e) => return Yanit::hata(500, &e),
        };
        let ana = nolar[0].clone();
        let zaman = self.simdi();
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
                zaman,
                yontem: if imza.is_some() { "ekran-imza" } else { "elle" }.into(),
                ..Default::default()
            },
        };
        // Şifreleme yazımdan ÖNCE: biri bile şifrelenemezse hiçbir şey yazılmaz.
        let mut sifreliler = Vec::with_capacity(nolar.len());
        for n in &nolar {
            match teslim_sifre::sifrele(&self.dizin, n, &duz) {
                Ok(x) => sifreliler.push(x),
                Err(e) => return Yanit::hata(500, &format!("teslim bilgisi şifrelenemedi: {e}")),
            }
        }
        // İmza BİR KEZ, ana fişin adıyla şifreli yazılır; bütün kayıtlar ona başvurur.
        let mut imza_sha256 = String::new();
        let mut imza_dosyasi = String::new();
        let mut imza_yontemi = String::new();
        let mut yazilan: Option<PathBuf> = None;
        if let Some((b, yontem)) = &imza {
            match self.imza_yaz(&ana, b) {
                Ok((goreli, yol)) => {
                    imza_sha256 = sha256_bayt(b);
                    imza_dosyasi = goreli;
                    imza_yontemi = (*yontem).to_string();
                    yazilan = Some(yol);
                }
                Err(y) => return y,
            }
        }
        let kim = c.kayit_kimi();
        let kayitlar: Vec<(String, taa_fis::TeslimKaydi)> = s
            .satirlar
            .iter()
            .zip(sifreliler)
            .map(|(x, sifreli)| {
                let dogrulama = match (x.okutuldu, onay.is_some()) {
                    (true, false) => "fis",
                    (true, true) => "qr",
                    (false, false) => "telefon-son4",
                    (false, true) => "qr+telefon-son4",
                };
                (
                    x.no.clone(),
                    taa_fis::TeslimKaydi {
                        teslim_alan: String::new(),
                        iliski: iliski.into(),
                        fisli: x.okutuldu,
                        dogrulama: dogrulama.into(),
                        imza_sha256: imza_sha256.clone(),
                        imza_dosyasi: imza_dosyasi.clone(),
                        imza_yontemi: imza_yontemi.clone(),
                        kim: kim.clone(),
                        tarih: tarih_iso(zaman),
                        saat: saat_iso(zaman),
                        ana_fis: if x.no == ana {
                            String::new()
                        } else {
                            ana.clone()
                        },
                        sifreli: Some(sifreli),
                        teslim_id: teslim_id.clone(),
                        gorunen: None,
                    },
                )
            })
            .collect();
        // Eski fiş geçişiyle fişli sayılan fişler: günlük teslim kaydından HEMEN ÖNCE yazılır; yazılamazsa teslim
        // yapılmaz (geçişle fişli teslim günlüksüz olmasın). Kayıt sonra başarısız olursa hata satırı aynı teslim_id'li.
        let gecisli: Vec<&str> = s
            .satirlar
            .iter()
            .filter(|x| x.gecis)
            .map(|x| x.no.as_str())
            .collect();
        if !gecisli.is_empty() {
            if let Err(e) = self.sepet_gunlugu_yaz(json!({
                "olay": "eski_fis_gecisi", "asama": "teslim", "teslim_id": teslim_id, "fisler": gecisli,
                "kim": Self::denetim_kimi(c),
            })) {
                if let Some(y) = &yazilan {
                    self.imza_artigi_kenara(y);
                }
                return Yanit::json(
                    500,
                    json!({
                        "hata": "gunluk_yazilamadi",
                        "ileti": format!("Eski fiş geçişi günlüğe yazılamadı; hiçbir fiş teslim edilmedi ({e}). Yeniden deneyin."),
                        "teslim_edilen": [],
                        "fisler": nolar,
                    }),
                );
            }
        }
        #[cfg(test)]
        let deneme_hatasi = SEPET_YAZIM_HATASI.with(|h| h.get());
        #[cfg(not(test))]
        let deneme_hatasi = false;
        let yazim = if deneme_hatasi {
            Err("deneme: yazım hatası".to_string())
        } else {
            d.teslim_et_toplu(kayitlar)
        };
        if let Err(e) = yazim {
            if let Some(y) = &yazilan {
                self.imza_artigi_kenara(y);
            }
            self.sepet_gunlugu(json!({
                "olay": "teslim_sepet_hata", "teslim_id": teslim_id, "fisler": nolar,
                "kim": Self::denetim_kimi(c), "hata": e,
            }));
            return Yanit::json(
                500,
                json!({
                    "hata": "teslim_yazilamadi",
                    "ileti": format!("Teslim kaydedilemedi; hiçbir fiş teslim edilmedi ({e}). Yeniden deneyin."),
                    "teslim_edilen": [],
                    "fisler": nolar,
                }),
            );
        }
        if let Some(t) = &belirtec {
            self.qr_onayi_tuket(t);
        }
        // Müşteri takip: her fiş ayrı "teslim" durumuna geçer (kişisel veri gitmez).
        let mut satirlar = Vec::with_capacity(nolar.len());
        for n in &nolar {
            if let Ok(Some(f)) = d.bul(n) {
                self.takibe_yaz(&f);
                if let Some(t) = &f.teslim_kaydi {
                    satirlar.push(json!({
                        "no": f.no,
                        "musteri": f.musteri,
                        "urun_sayisi": urun_sayisi(&f),
                        "fisli": t.fisli,
                        "dogrulama": t.dogrulama,
                        "ana_fis": t.ana_fis,
                        "teslim_id": t.teslim_id,
                        "imzali": !t.imza_sha256.is_empty(),
                        "tarih": t.tarih,
                        "saat": t.saat,
                    }));
                }
            }
        }
        self.sepet_gunlugu(json!({
            "olay": "teslim_sepet", "teslim_id": teslim_id, "fisler": nolar, "ana_fis": ana,
            "fissiz": fissiz, "kim": Self::denetim_kimi(c),
        }));
        Yanit::json(
            200,
            json!({
                "tamam": true,
                "teslim_id": teslim_id,
                "ana_fis": ana,
                "fisler": satirlar,
                "yok_sayilan": s.yok_sayilan,
                "kanitsiz": s.kanitsiz,
                // Eski teslim-al yanıtıyla uyum (bildirim metni için).
                "birlikte_teslim": nolar[1..],
                "teslim_kaydi": {
                    "ad": taa_fis::ad_maskele(&duz.ad),
                    "maskeli": true,
                    "imza_yontemi": imza_yontemi,
                    "tel_maskeli": teslim_sifre::tel_maskele(&duz.tel),
                    "cihaz": teslim_sifre::cihaz_ozeti(&duz.cihaz),
                    "yontem": duz.yontem,
                    "iliski": iliski,
                    "fisli": fissiz.is_empty(),
                    "dogrulama": if onay.is_some() { "qr" } else if fissiz.is_empty() { "fis" } else { "telefon-son4" },
                    "imzali": imza.is_some(),
                    "sifreli": true,
                    "tarih": tarih_iso(zaman),
                    "saat": saat_iso(zaman),
                },
            }),
        )
    }

    /// Fişsiz fişlerin telefon son 4 doğrulaması: her telefon grubu için bir kez. Önce bütün grupların biçimi ve
    /// kilidi denetlenir, sonra değerler; yanlışta gruptaki her fişin sayacı artar (yeniden gruplayıp hak kazanılmaz).
    fn sepet_son4_dogrula(&self, s: &Sepet, v: &serde_json::Value) -> Result<(), Yanit> {
        let gruplar = Self::son4_gruplari(s);
        let tek = v["telefon_son4"].as_str();
        let harita = v["telefon_son4"].as_object();
        let mut degerler: Vec<Son4Degeri> = Vec::new();
        for g in &gruplar {
            let (anahtar, l) = g;
            for n in l {
                if let Some(kalan) = self.son4_kilitli(n) {
                    return Err(Yanit::json(
                        429,
                        json!({
                            "hata": "son4_kilitli",
                            "fis": n,
                            "ileti": format!("{n}: çok fazla yanlış deneme; {} dakika sonra yeniden deneyin.", kalan.div_ceil(60)),
                            "kalan_sn": kalan,
                        }),
                    ));
                }
            }
            let deger = match (harita, tek) {
                (Some(h), _) => l
                    .iter()
                    .find_map(|n| h.get(n).and_then(|x| x.as_str()))
                    .unwrap_or("")
                    .to_string(),
                (None, Some(t)) if gruplar.len() == 1 => t.to_string(),
                _ => String::new(),
            };
            if deger.len() != 4 || !deger.bytes().all(|b| b.is_ascii_digit()) {
                return Err(Yanit::json(
                    422,
                    json!({
                        "hata": "son4_gerekli",
                        "fis": anahtar,
                        "fisler": l,
                        "ileti": format!("{} için müşteri telefonunun son 4 hanesi (tam 4 rakam) gerekli.", l.join(", ")),
                    }),
                ));
            }
            let fis = s
                .satirlar
                .iter()
                .find(|x| x.no == *anahtar)
                .map(|x| x.fis.telefon.clone())
                .unwrap_or_default();
            let rakamlar: Vec<u8> = fis.bytes().filter(u8::is_ascii_digit).collect();
            if rakamlar.len() < 4 {
                return Err(Yanit::json(
                    422,
                    json!({ "hata": "telefon_yok", "fis": anahtar, "ileti": format!("{anahtar}: fişte doğrulanabilir telefon yok; fişsiz teslim edilemez.") }),
                ));
            }
            degerler.push((g, deger, rakamlar));
        }
        for ((anahtar, l), deger, rakamlar) in degerler {
            if !sabit_esit(deger.as_bytes(), &rakamlar[rakamlar.len() - 4..]) {
                let kalan = l.iter().map(|n| self.son4_yanlis(n)).min().unwrap_or(0);
                return Err(Yanit::json(
                    422,
                    json!({
                        "hata": "son4_yanlis",
                        "fis": anahtar,
                        "fisler": l,
                        "ileti": if kalan == 0 {
                            format!("{}: telefonun son 4 hanesi yanlış. Bu fiş(ler) 15 dakika kilitlendi.", l.join(", "))
                        } else {
                            format!("{}: telefonun son 4 hanesi yanlış. Kalan deneme: {kalan}.", l.join(", "))
                        },
                        "kalan_hak": kalan,
                        "kilitli": kalan == 0,
                    }),
                ));
            }
        }
        for (_, l) in &gruplar {
            for n in l {
                self.son4_sifirla(n);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::teslim_al::testler::{
        fis_kk, hazir_fis, imza_b64, json_of, pc, qr_uyg, tel, tum_dosyalarda, uyg, A, FIS, QR_AD,
        QR_TEL,
    };

    /// Başka bir sahte müşteri (telefonun son 4 hanesi 2233).
    fn baska_fis(u: &Uygulama) -> String {
        let g = FIS
            .replace("0532 000 45 67", "0533 111 22 33")
            .replace("Deneme Müşteri", "Deneme Başka");
        let y = u.isle(&pc("POST", "/api/fis", &g));
        assert_eq!(y.kod, 200);
        let no = json_of(&y)["no"].as_str().unwrap().to_string();
        assert_eq!(
            u.isle(&pc(
                "POST",
                &format!("/api/fis/{no}/durum"),
                r#"{"durum":"hazir"}"#
            ))
            .kod,
            200
        );
        no
    }

    fn bekleyen_fis(u: &Uygulama) -> String {
        let y = u.isle(&pc("POST", "/api/fis", FIS));
        json_of(&y)["no"].as_str().unwrap().to_string()
    }

    fn denetle(u: &Uygulama, fisler: &serde_json::Value, ekle: &str) -> serde_json::Value {
        let y = u.isle(&pc(
            "POST",
            "/api/teslim/sepet/denetle",
            &json!({ "fisler": fisler, "ekle": ekle }).to_string(),
        ));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        json_of(&y)
    }

    /// Denetle yanıtındaki satırları teslim isteğinin `fisler` biçimine çevirir (yalnız hazır olanlar).
    fn gonderilecek(j: &serde_json::Value) -> serde_json::Value {
        json!(j["fisler"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x["hazir"] == true)
            .map(|x| json!({ "no": x["no"], "okutuldu": x["okutuldu"], "karekod": x["karekod"].as_str().unwrap_or("") }))
            .collect::<Vec<_>>())
    }

    fn teslim(u: &Uygulama, govde: serde_json::Value) -> Yanit {
        u.isle(&pc("POST", "/api/teslim/sepet", &govde.to_string()))
    }

    fn fis(u: &Uygulama, no: &str) -> taa_fis::Fis {
        let y = u.fis_yolu();
        Defter { yol: &y }.bul(no).unwrap().unwrap()
    }

    fn bagla(u: &Uygulama, a: &str, b: &str) {
        let y = u.fis_yolu();
        Defter { yol: &y }.bagla(a, b, "PC", "2026-11-26").unwrap();
    }

    fn imza_dosyalari(u: &Uygulama) -> Vec<String> {
        std::fs::read_dir(u.dizin.join("imzalar"))
            .map(|r| {
                r.flatten()
                    .filter(|e| e.path().is_file())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    #[test]
    fn metin_cozumu_ve_kanit() {
        assert_eq!(
            f_deseni("F-20261126-001").as_deref(),
            Some("F-20261126-001")
        );
        assert_eq!(f_deseni("f20261126007").as_deref(), Some("F-20261126-007"));
        assert_eq!(f_deseni("F-2026112-001"), None);
        assert_eq!(
            takip_kodu_ayikla("https://a.example/t/abcdefghijkmnpqr?x"),
            Some("abcdefghijkmnpqr")
        );
        let u = uyg();
        let no = hazir_fis(&u);
        // takip ayarı yok: personel karekodu müşteri nüshasındadır, kanıttır; elle yazılan değildir
        let j = json_of(&u.isle(&pc(
            "POST",
            "/api/teslim/karekod-coz",
            &json!({ "metin": fis_kk(&u, &no) }).to_string(),
        )));
        assert_eq!(j["no"], no.as_str());
        assert_eq!(j["okutuldu"], true);
        let j = json_of(&u.isle(&pc(
            "POST",
            "/api/teslim/karekod-coz",
            &json!({ "metin": no }).to_string(),
        )));
        assert_eq!(j["okutuldu"], false);
        assert_eq!(j["kanit"], "elle");
        // Denetçi bulgusu: "#fis=<no>" gizli bir şey içermez, elle yazılabilir. Kanıt parçası (k) yoksa ya da
        // yanlışsa (eski basılı fiş, uydurma) fişsizdir.
        for metin in [
            format!("#fis={no}"),
            format!("http://192.168.1.20:8787/telefon#fis={no}"),
            format!("http://192.168.1.20:8787/telefon#fis={no}&k=abcdefghijkm"),
            format!("http://192.168.1.20:8787/telefon#fis={no}&k="),
        ] {
            let j = json_of(&u.isle(&pc(
                "POST",
                "/api/teslim/karekod-coz",
                &json!({ "metin": metin }).to_string(),
            )));
            assert_eq!(j["no"], no.as_str(), "{metin}");
            assert_eq!(j["kanit"], "karekod-kanitsiz", "{metin}");
            assert_eq!(j["okutuldu"], false, "{metin}");
        }
        // başka fişin kanıtı bu fişe taşınamaz
        let diger = hazir_fis(&u);
        let k_diger = u.fis_karekod_adresi(&diger);
        let tasinan = k_diger.replace(&diger, &no);
        let j = json_of(&u.isle(&pc(
            "POST",
            "/api/teslim/karekod-coz",
            &json!({ "metin": tasinan }).to_string(),
        )));
        assert_eq!(j["kanit"], "karekod-kanitsiz");
        let y = u.isle(&pc(
            "POST",
            "/api/teslim/karekod-coz",
            r#"{"metin":"F-20000101-999"}"#,
        ));
        assert_eq!(y.kod, 404);
        // takip ayarlı: müşteri nüshasının takip karekodu kanıttır, personel karekodu (terzihane nüshası) değildir
        let (u, _) = qr_uyg();
        let no = hazir_fis(&u);
        let j = json_of(&u.isle(&pc(
            "POST",
            "/api/teslim/karekod-coz",
            &json!({ "metin": fis_kk(&u, &no) }).to_string(),
        )));
        assert_eq!(j["no"], no.as_str());
        assert_eq!(j["kanit"], "takip");
        assert_eq!(j["okutuldu"], true);
        let personel = u.fis_karekod_adresi(&no);
        assert!(personel.contains("&k="), "{personel}");
        let j = json_of(&u.isle(&pc(
            "POST",
            "/api/teslim/karekod-coz",
            &json!({ "metin": personel }).to_string(),
        )));
        assert_eq!(j["kanit"], "fis-karekodu");
        assert_eq!(j["okutuldu"], false);
        // uydurma takip kodu: fiş bulunmaz
        let y = u.isle(&pc(
            "POST",
            "/api/teslim/karekod-coz",
            r#"{"metin":"https://a.example/t/abcdefghijkmnpqr"}"#,
        ));
        assert_eq!(y.kod, 404);
    }

    /// Bağlı zincir: tek karekod okutulunca bütün bağlı fişler sepete gelir; okutulan fişli, diğerleri fişsiz.
    /// Tek teslimatta: ortak teslim_id, ana fiş ilk okutulan, imza bir kez yazılır, bütün kayıtlar ona başvurur.
    #[test]
    fn bagli_zincir_tek_karekod_tek_imza_ortak_teslim_id() {
        let u = uyg();
        let (a, b, c) = (hazir_fis(&u), hazir_fis(&u), hazir_fis(&u));
        bagla(&u, &a, &b);
        bagla(&u, &b, &c);
        let j = denetle(&u, &json!([]), &fis_kk(&u, &a));
        let nolar: Vec<&str> = j["fisler"]
            .as_array()
            .unwrap()
            .iter()
            .map(|x| x["no"].as_str().unwrap())
            .collect();
        assert_eq!(nolar, vec![a.as_str(), b.as_str(), c.as_str()]);
        assert_eq!(j["fisler"][0]["okutuldu"], true);
        assert_eq!(j["fisler"][1]["okutuldu"], false);
        assert_eq!(j["fisler"][2]["kanit"], "elle");
        assert_eq!(j["toplam_urun"], 3);
        assert_eq!(j["fissiz_var"], true);
        assert_eq!(j["imza_zorunlu"], true);
        // aynı telefon: son 4 bir kez
        assert_eq!(j["son4_gruplari"].as_array().unwrap().len(), 1);
        assert_eq!(j["son4_gruplari"][0]["anahtar"], b.as_str());
        let fisler = gonderilecek(&j);
        let imza = imza_b64();
        let y = teslim(
            &u,
            json!({
                "fisler": fisler, "teslim_alan": "Deneme Yakın Kişi", "iliski": "yakini",
                "imza_png_base64": imza, "telefon_son4": { b.clone(): "4567" },
            }),
        );
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let r = json_of(&y);
        let tid = r["teslim_id"].as_str().unwrap().to_string();
        assert!(taa_fis::teslim_id_gecerli(&tid), "{tid}");
        assert_eq!(r["ana_fis"], a.as_str());
        assert_eq!(r["fisler"].as_array().unwrap().len(), 3);
        let (fa, fb, fc) = (fis(&u, &a), fis(&u, &b), fis(&u, &c));
        for f in [&fa, &fb, &fc] {
            assert_eq!(f.durum, Durum::Teslim);
            let t = f.teslim_kaydi.as_ref().unwrap();
            assert_eq!(t.teslim_id, tid);
            assert_eq!(
                t.imza_dosyasi,
                fa.teslim_kaydi.as_ref().unwrap().imza_dosyasi
            );
            assert_eq!(t.imza_sha256, fa.teslim_kaydi.as_ref().unwrap().imza_sha256);
            assert_eq!(t.imza_yontemi, "cihaz_imza");
            assert!(t.sifreli.is_some());
            assert!(t.teslim_alan.is_empty());
        }
        let (ta, tb) = (fa.teslim_kaydi.unwrap(), fb.teslim_kaydi.unwrap());
        assert_eq!(ta.ana_fis, "");
        assert!(ta.fisli);
        assert_eq!(ta.dogrulama, "fis");
        assert_eq!(tb.ana_fis, a);
        assert!(!tb.fisli);
        assert_eq!(tb.dogrulama, "telefon-son4");
        assert_eq!(fc.teslim_kaydi.unwrap().ana_fis, a);
        // imza tek dosya, ana fişin adıyla; bağlı fişlerden de okunur (özet tutar)
        let dosyalar = imza_dosyalari(&u);
        assert_eq!(dosyalar, vec![format!("{a}-1.png.enc")]);
        for n in [&a, &b, &c] {
            let y = u.isle(&pc("GET", &format!("/api/fis/{n}/imza"), ""));
            assert_eq!(y.kod, 200, "{n}: {}", String::from_utf8_lossy(&y.govde));
            assert_eq!(y.tur, "image/png");
        }
        // şifreli blokta düz metin yok: ad ve imza düz hâli hiçbir dosyada geçmez
        let mut bulunan = Vec::new();
        tum_dosyalarda(&u.dizin, &["Deneme Yakın Kişi", "Yakın"], &mut bulunan);
        assert!(bulunan.is_empty(), "{bulunan:?}");
        // her şifreli blok kendi fişine bağlı (aad): başka fişin bloğu çözülmez
        let sa = fis(&u, &a).teslim_kaydi.unwrap().sifreli.unwrap();
        assert!(teslim_sifre::coz(&u.dizin, &a, &sa).is_ok());
        assert!(teslim_sifre::coz(&u.dizin, &b, &sa).is_err());
        // sepet günlüğü: kişisel veri yok, teslim_id var
        let g = std::fs::read_to_string(u.dizin.join("teslim-sepet.jsonl")).unwrap();
        assert!(g.contains(&tid) && !g.contains("Deneme Yakın"));
        // müşteri takip kaydı ayarsız: kuyruk dosyası yazılmaz (kişisel veri gitmez)
        assert!(!u.dizin.join("musteri-takip").join("bekleyen.json").exists());
    }

    /// Bağlı olmayan, farklı müşterilerin fişleri tek teslimatta; hepsi okutuldu: imza ve son 4 gerekmez.
    /// Eski fiş geçiş süresi (koordinatör kararı 2026-11-26): kapalıyken eski fişin kanıtsız karekodu / elle no'su
    /// fişsizdir; açıkken (yönetici ayarı, gün) fişli sayılır ve günlüğe yazılır; süre bitince kendiliğinden kapanır.
    /// Yeni (kanıt mekanizmasından sonra basılmış) fiş geçişle fişli olmaz.
    #[test]
    fn eski_fis_gecis_suresi() {
        let u = uyg();
        let (a, b) = (hazir_fis(&u), baska_fis(&u));
        // a eski fiş: başlangıç bugün, son eski fiş a (b ondan sonra basıldı).
        let bugun = tarih_iso(u.simdi());
        std::fs::write(
            u.dizin.join(crate::eski_fis_gecis::AYAR_DOSYASI),
            json!({ "fisli_kanit_baslangic": bugun, "fisli_kanit_son_eski_fis": a }).to_string(),
        )
        .unwrap();
        let eski_kk = format!("http://192.168.1.20:8787/telefon#fis={a}");
        // Kapalı (0 gün): kanıtsız karekod fişsiz.
        let j = denetle(&u, &json!([]), &eski_kk);
        assert_eq!(j["fissiz_var"], true);
        assert_eq!(j["fisler"][0]["gecis"], false);
        // Ayar yalnız yönetici oturumuyla, bu bilgisayardan; sınır 90 gün.
        let mut uzak = pc("POST", "/api/yonetim/ayar", r#"{"eski_fis_gecis_gun":3}"#);
        uzak.yerel = false;
        assert_eq!(u.isle(&uzak).kod, 403);
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/ayar",
                r#"{"eski_fis_gecis_gun":91}"#
            ))
            .kod,
            422
        );
        let y = u.isle(&pc(
            "POST",
            "/api/yonetim/ayar",
            r#"{"eski_fis_gecis_gun":3}"#,
        ));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let v = json_of(&y);
        assert_eq!(v["eski_fis_gecis_acik"], true);
        assert_eq!(v["eski_fis_gecis_kalan_gun"], 3);
        assert_eq!(
            v["fisli_kanit_baslangic"],
            json!(bugun),
            "başlangıç değişmez"
        );
        // Açık: eski fiş (kanıtsız karekod ve elle no) fişli; yeni fiş (b) elle no ile fişsiz kalır.
        let j = denetle(&u, &json!([]), &eski_kk);
        assert_eq!(j["fisler"][0]["okutuldu"], true);
        assert_eq!(j["fisler"][0]["gecis"], true);
        assert_eq!(j["fissiz_var"], false);
        let j = denetle(&u, &gonderilecek(&j), &b);
        assert_eq!(j["fissiz"], json!([b]));
        let k = json_of(&u.isle(&pc(
            "POST",
            "/api/teslim/karekod-coz",
            &json!({ "metin": a }).to_string(),
        )));
        assert_eq!(
            (k["okutuldu"].clone(), k["gecis"].clone()),
            (json!(true), json!(true))
        );
        // Teslim: a fişli (kanıtsız karekodla) — günlükte eski_fis_gecisi.
        let j = denetle(&u, &json!([]), &eski_kk);
        let y = teslim(
            &u,
            json!({ "fisler": gonderilecek(&j), "teslim_alan": "Deneme Müşteri", "iliski": "kendisi" }),
        );
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert!(fis(&u, &a).teslim_kaydi.unwrap().fisli);
        let g = std::fs::read_to_string(u.dizin.join("teslim-sepet.jsonl")).unwrap();
        assert!(g.contains("eski_fis_gecisi") && g.contains(&a), "{g}");
        let yg = std::fs::read_to_string(u.dizin.join("eslestirme.jsonl")).unwrap();
        assert!(
            yg.contains("eski_fis_gecis_gun") && yg.contains("\"kim\":\"PC\""),
            "{yg}"
        );
        // Süre bitince kendiliğinden kapanır: başka bir eski fiş (c) yeniden fişsiz.
        let c = hazir_fis(&u);
        let mut ay = crate::eski_fis_gecis::YonetimAyari::oku(&u.dizin);
        ay.fisli_kanit_son_eski_fis = c.clone();
        std::fs::write(
            u.dizin.join(crate::eski_fis_gecis::AYAR_DOSYASI),
            serde_json::to_string(&ay).unwrap(),
        )
        .unwrap();
        let kk_c = format!("http://192.168.1.20:8787/telefon#fis={c}");
        assert_eq!(denetle(&u, &json!([]), &kk_c)["fisler"][0]["gecis"], true);
        u.ileri_sn
            .store(3 * 86_400 + 1, std::sync::atomic::Ordering::SeqCst);
        let j = denetle(&u, &json!([]), &kk_c);
        assert_eq!(j["fisler"][0]["okutuldu"], false);
        assert_eq!(j["fissiz_var"], true);
        let v = json_of(&u.isle(&pc("GET", "/api/yonetim/ayar", "")));
        assert_eq!(v["eski_fis_gecis_acik"], false);
        assert_eq!(
            v["eski_fis_gecis_gun"], 3,
            "ayar kalır, yalnız karar kapanır"
        );
    }

    /// Bulgu (orta): geçiş kullanımının günlüğü bağlayıcı. Reddedilen teslim günlüğe "kullanım" olarak düşmez; satır
    /// teslimden hemen önce "kim" ile yazılır (teslim-al ve sepet); günlük yazılamazsa teslim yapılmaz.
    #[test]
    fn eski_fis_gecisi_gunlugu_baglayici() {
        let u = uyg();
        let (a, b) = (hazir_fis(&u), hazir_fis(&u));
        std::fs::write(
            u.dizin.join(crate::eski_fis_gecis::AYAR_DOSYASI),
            json!({ "fisli_kanit_baslangic": tarih_iso(u.simdi()), "fisli_kanit_son_eski_fis": b })
                .to_string(),
        )
        .unwrap();
        assert_eq!(
            u.isle(&pc(
                "POST",
                "/api/yonetim/ayar",
                r#"{"eski_fis_gecis_gun":2}"#
            ))
            .kod,
            200
        );
        let gunluk =
            || std::fs::read_to_string(u.dizin.join("teslim-sepet.jsonl")).unwrap_or_default();
        let kk = |n: &str| format!("http://192.168.1.20:8787/telefon#fis={n}");
        let teslim_al = |n: &str, ek: serde_json::Value| {
            let mut g = json!({ "fisli": true, "fis_karekodu": kk(n), "iliski": "kendisi" });
            for (k, v) in ek.as_object().unwrap() {
                g[k] = v.clone();
            }
            u.isle(&pc(
                "POST",
                &format!("/api/fis/{n}/teslim-al"),
                &g.to_string(),
            ))
        };
        // Reddedilen teslim (teslim alan yok): günlükte geçiş kullanımı yok.
        let y = teslim_al(&a, json!({}));
        assert_ne!(y.kod, 200);
        assert!(!gunluk().contains("eski_fis_gecisi"), "{}", gunluk());
        // Günlük yazılamıyor: teslim yapılmaz.
        GECIS_GUNLUK_HATASI.with(|h| h.set(true));
        let y = teslim_al(&a, json!({ "teslim_alan": "Deneme Müşteri" }));
        GECIS_GUNLUK_HATASI.with(|h| h.set(false));
        assert_eq!(
            (y.kod, json_of(&y)["hata"].clone()),
            (500, json!("gunluk_yazilamadi"))
        );
        assert!(fis(&u, &a).teslim_kaydi.is_none());
        // Başarılı teslim-al: satır "kim" ve teslim_id ile.
        let y = teslim_al(&a, json!({ "teslim_alan": "Deneme Müşteri" }));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert!(fis(&u, &a).teslim_kaydi.unwrap().fisli);
        let tid = json_of(&y)["teslim_id"].as_str().unwrap().to_string();
        let satir = gunluk()
            .lines()
            .find(|l| l.contains("eski_fis_gecisi") && l.contains("teslim-al"))
            .unwrap()
            .to_string();
        assert!(
            satir.contains(r#""kim":"PC""#) && satir.contains(&tid) && satir.contains(&a),
            "{satir}"
        );
        // Sepet yolunda da: günlük yazılamazsa hiçbir fiş teslim edilmez.
        let j = denetle(&u, &json!([]), &kk(&b));
        assert_eq!(j["fisler"][0]["gecis"], true);
        GECIS_GUNLUK_HATASI.with(|h| h.set(true));
        let y = teslim(
            &u,
            json!({ "fisler": gonderilecek(&j), "teslim_alan": "Deneme Müşteri", "iliski": "kendisi" }),
        );
        GECIS_GUNLUK_HATASI.with(|h| h.set(false));
        assert_eq!(
            (y.kod, json_of(&y)["hata"].clone()),
            (500, json!("gunluk_yazilamadi"))
        );
        assert!(fis(&u, &b).teslim_kaydi.is_none());
        let y = teslim(
            &u,
            json!({ "fisler": gonderilecek(&j), "teslim_alan": "Deneme Müşteri", "iliski": "kendisi" }),
        );
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert!(gunluk().lines().any(|l| l.contains("eski_fis_gecisi")
            && l.contains(r#""asama":"teslim""#)
            && l.contains(&b)
            && l.contains(r#""kim":"PC""#)));
    }

    #[test]
    fn bagli_olmayan_cok_fis_hepsi_fisli() {
        let u = uyg();
        let (a, b) = (hazir_fis(&u), baska_fis(&u));
        let j = denetle(&u, &json!([]), &fis_kk(&u, &a));
        let j = denetle(&u, &gonderilecek(&j), &fis_kk(&u, &b));
        assert_eq!(j["fisler"].as_array().unwrap().len(), 2);
        assert_eq!(j["fissiz_var"], false);
        assert_eq!(j["imza_zorunlu"], false);
        assert_eq!(j["eklenen"], json!([b]));
        let y = teslim(
            &u,
            json!({ "fisler": gonderilecek(&j), "teslim_alan": "Deneme Müşteri", "iliski": "kendisi" }),
        );
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let r = json_of(&y);
        assert_eq!(r["teslim_kaydi"]["imzali"], false);
        let (ta, tb) = (
            fis(&u, &a).teslim_kaydi.unwrap(),
            fis(&u, &b).teslim_kaydi.unwrap(),
        );
        assert_eq!(ta.teslim_id, tb.teslim_id);
        assert!(ta.fisli && tb.fisli);
        assert_eq!(tb.ana_fis, a);
        assert!(imza_dosyalari(&u).is_empty());
        // eşleşmiş telefon, fişsiz teslim yetkisi olmadan da tamamen fişli sepeti teslim eder
        let (c, d) = (hazir_fis(&u), baska_fis(&u));
        let token = "deneme-belirtec-0101";
        u.cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        let g = json!({
            "fisler": [{ "no": c, "okutuldu": true, "karekod": fis_kk(&u, &c) }, { "no": d, "okutuldu": true, "karekod": fis_kk(&u, &d) }],
            "teslim_alan": "Deneme Müşteri", "iliski": "kendisi",
        });
        let y = u.isle(&tel(token, "POST", "/api/teslim/sepet", &g.to_string()));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
    }

    /// Çift okutma yok sayılır; fişsiz satır sonra karekodla okutulursa fişliye yükselir.
    #[test]
    fn cift_okutma_yok_sayilir() {
        let u = uyg();
        let a = hazir_fis(&u);
        let j = denetle(&u, &json!([]), &a); // elle yazıldı: fişsiz
        assert_eq!(j["fisler"][0]["okutuldu"], false);
        let j = denetle(&u, &gonderilecek(&j), &a); // aynı no yeniden
        assert_eq!(j["zaten_sepette"], true);
        assert_eq!(j["fisler"].as_array().unwrap().len(), 1);
        assert_eq!(j["yok_sayilan"], json!([a]));
        let j = denetle(&u, &gonderilecek(&j), &fis_kk(&u, &a)); // şimdi karekod
        assert_eq!(j["fisler"].as_array().unwrap().len(), 1);
        assert_eq!(j["fisler"][0]["okutuldu"], true);
        // teslim isteğinde çift satır tek kayıt olur
        let karekod = fis_kk(&u, &a);
        let y = teslim(
            &u,
            json!({
                "fisler": [{ "no": a, "okutuldu": true, "karekod": karekod }, { "no": a, "okutuldu": true, "karekod": karekod }],
                "teslim_alan": "Deneme Müşteri", "iliski": "kendisi",
            }),
        );
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(json_of(&y)["yok_sayilan"], json!([a]));
        assert_eq!(json_of(&y)["fisler"].as_array().unwrap().len(), 1);
        // teslim edilmiş fiş sepete girmez
        let j = denetle(&u, &json!([]), &a);
        assert!(j["fisler"].as_array().unwrap().is_empty());
        assert_eq!(j["reddedilen"][0]["no"], a.as_str());
    }

    /// Hazır olmayan fiş: listede uyarılı; teslim isteğinde varsa HİÇBİRİ teslim edilmez. İptal sepete girmez.
    #[test]
    fn hazir_olmayan_red_ve_iptal() {
        let u = uyg();
        let (a, b) = (hazir_fis(&u), bekleyen_fis(&u));
        bagla(&u, &a, &b);
        let j = denetle(&u, &json!([]), &fis_kk(&u, &a));
        assert_eq!(j["fisler"].as_array().unwrap().len(), 2);
        assert_eq!(j["fisler"][1]["hazir"], false);
        assert!(j["fisler"][1]["uyari"]
            .as_str()
            .unwrap()
            .contains("Hazır değil"));
        assert_eq!(j["hazir_sayisi"], 1);
        assert_eq!(
            j["fissiz_var"], false,
            "hazır olmayan fişsiz sayılmaz (teslim edilmez)"
        );
        let karekod = fis_kk(&u, &a);
        let y = teslim(
            &u,
            json!({
                "fisler": [{ "no": a, "okutuldu": true, "karekod": karekod }, { "no": b, "okutuldu": false }],
                "teslim_alan": "Deneme Müşteri", "iliski": "kendisi",
            }),
        );
        assert_eq!(y.kod, 409);
        assert_eq!(json_of(&y)["hata"], "hazir_degil");
        assert_eq!(json_of(&y)["fisler"][0]["no"], b.as_str());
        assert_eq!(
            fis(&u, &a).durum,
            Durum::Hazir,
            "hazır olan da teslim edilmedi"
        );
        // iptal fiş sepete girmez; teslim isteğinde 409
        let c = hazir_fis(&u);
        assert_eq!(
            u.isle(&pc(
                "POST",
                &format!("/api/fis/{c}/durum"),
                r#"{"durum":"iptal"}"#
            ))
            .kod,
            200
        );
        let j = denetle(&u, &json!([]), &c);
        assert!(j["fisler"].as_array().unwrap().is_empty());
        assert!(j["reddedilen"][0]["neden"]
            .as_str()
            .unwrap()
            .contains("iptal"));
        let y = teslim(
            &u,
            json!({ "fisler": [{ "no": a, "okutuldu": true, "karekod": karekod }, c], "teslim_alan": "Deneme Müşteri", "iliski": "kendisi" }),
        );
        assert_eq!(y.kod, 409);
        assert_eq!(json_of(&y)["hata"], "sepet_gecersiz");
        assert_eq!(fis(&u, &a).durum, Durum::Hazir);
    }

    /// Fişli + fişsiz karışık sepet: imza ve her fişsiz müşteri için son 4 zorunlu (aynı telefonda bir kez).
    #[test]
    fn karisik_sepette_son4_ve_imza_zorunlu() {
        let u = uyg();
        let (a, b, c) = (hazir_fis(&u), baska_fis(&u), baska_fis(&u));
        let fisler = json!([
            { "no": a, "okutuldu": true, "karekod": fis_kk(&u, &a) },
            { "no": b, "okutuldu": false },
            // "okutuldu" denildi ama kanıt yok: fişsiz sayılır
            { "no": c, "okutuldu": true },
        ]);
        let temel =
            json!({ "fisler": fisler, "teslim_alan": "Deneme Yakın Kişi", "iliski": "yakini" });
        let ile = |ek: serde_json::Value| {
            let mut g = temel.clone();
            for (k, v) in ek.as_object().unwrap() {
                g[k] = v.clone();
            }
            g
        };
        // imza yok
        let y = teslim(&u, temel.clone());
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "imza_gerekli");
        assert_eq!(json_of(&y)["fissiz"], json!([b, c]));
        // imza var, son 4 yok
        let imza = imza_b64();
        let y = teslim(&u, ile(json!({ "imza_png_base64": imza })));
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "son4_gerekli");
        // b ve c aynı telefon: tek grup, gruptaki herhangi bir no ile
        assert_eq!(json_of(&y)["fisler"], json!([b, c]));
        // yanlış son 4
        let y = teslim(
            &u,
            ile(json!({ "imza_png_base64": imza, "telefon_son4": { c.clone(): "4567" } })),
        );
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "son4_yanlis");
        assert_eq!(json_of(&y)["kalan_hak"], 4);
        assert_eq!(fis(&u, &a).durum, Durum::Hazir);
        // doğru
        let y = teslim(
            &u,
            ile(json!({ "imza_png_base64": imza, "telefon_son4": { c.clone(): "2233" } })),
        );
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(json_of(&y)["kanitsiz"], json!([c]));
        assert!(fis(&u, &a).teslim_kaydi.unwrap().fisli);
        assert!(!fis(&u, &c).teslim_kaydi.unwrap().fisli);
        // iki farklı telefonlu fişsiz grup: ikisinin de son 4'ü gerekir
        let (d, e) = (hazir_fis(&u), baska_fis(&u));
        let g = json!({ "fisler": [d, e], "teslim_alan": "Deneme Yakın Kişi", "iliski": "yakini", "imza_png_base64": imza, "telefon_son4": { d.clone(): "4567" } });
        let y = teslim(&u, g);
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "son4_gerekli");
        assert_eq!(json_of(&y)["fis"], e.as_str());
        let g = json!({ "fisler": [d, e], "teslim_alan": "Deneme Yakın Kişi", "iliski": "yakini", "imza_png_base64": imza, "telefon_son4": { d.clone(): "4567", e.clone(): "2233" } });
        assert_eq!(teslim(&u, g).kod, 200);
    }

    /// Yetkisiz cihazda (eşleşmiş telefon, fissiz-teslim yok) sepette fişsiz fiş varsa red; hiçbiri teslim edilmez.
    #[test]
    fn yetkisiz_cihazda_fissiz_red() {
        let u = uyg();
        let (a, b) = (hazir_fis(&u), hazir_fis(&u));
        let token = "deneme-belirtec-0102";
        u.cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        let g = json!({
            "fisler": [{ "no": a, "okutuldu": true, "karekod": fis_kk(&u, &a) }, { "no": b, "okutuldu": false }],
            "teslim_alan": "Deneme Müşteri", "iliski": "kendisi", "imza_png_base64": imza_b64(), "telefon_son4": "4567",
        });
        let y = u.isle(&tel(token, "POST", "/api/teslim/sepet", &g.to_string()));
        assert_eq!(y.kod, 403);
        assert_eq!(json_of(&y)["hata"], "yetki_yok");
        assert_eq!(json_of(&y)["fissiz"], json!([b]));
        assert_eq!(fis(&u, &a).durum, Durum::Hazir);
        // denetle de uyarır
        let y = u.isle(&tel(
            token,
            "POST",
            "/api/teslim/sepet/denetle",
            &json!({ "fisler": [b] }).to_string(),
        ));
        let j = json_of(&y);
        assert_eq!(j["fissiz_yetki"], false);
        assert!(j["uyarilar"].to_string().contains("yetkisi yok"));
        // aynı sepet mağaza bilgisayarından (yetkili) geçer; tek grup olduğu için düz son 4 de olur
        assert_eq!(teslim(&u, g).kod, 200);
    }

    /// Kısmi hata: doğrulama geçti ama deftere yazılamadı. Hiçbir fiş teslim sayılmaz, imza dosyası kenara alınır
    /// (silinmez), bulut kuyruğuna yazılmaz, günlüğe hata satırı düşer; yeniden deneme başarılı olur.
    #[test]
    fn kismi_hata_hicbiri_yazilmaz() {
        let u = uyg();
        let (a, b) = (hazir_fis(&u), hazir_fis(&u));
        let g = json!({
            "fisler": [a, b], "teslim_alan": "Deneme Müşteri", "iliski": "kendisi",
            "imza_png_base64": imza_b64(), "telefon_son4": "4567",
        });
        let once = std::fs::read(u.fis_yolu()).unwrap();
        SEPET_YAZIM_HATASI.with(|h| h.set(true));
        let y = teslim(&u, g.clone());
        SEPET_YAZIM_HATASI.with(|h| h.set(false));
        assert_eq!(y.kod, 500);
        let j = json_of(&y);
        assert_eq!(j["hata"], "teslim_yazilamadi");
        assert_eq!(j["teslim_edilen"], json!([]));
        assert_eq!(
            std::fs::read(u.fis_yolu()).unwrap(),
            once,
            "defter değişmedi"
        );
        assert_eq!(fis(&u, &a).durum, Durum::Hazir);
        assert_eq!(fis(&u, &b).durum, Durum::Hazir);
        assert!(
            imza_dosyalari(&u).is_empty(),
            "bağlanmamış imza ana klasörde kalmaz"
        );
        assert_eq!(
            std::fs::read_dir(u.dizin.join("imzalar").join("artik"))
                .unwrap()
                .count(),
            1
        );
        let gunluk = std::fs::read_to_string(u.dizin.join("teslim-sepet.jsonl")).unwrap();
        assert!(gunluk.contains("teslim_sepet_hata") && !gunluk.contains("Deneme Müşteri"));
        // yeniden deneme
        let y = teslim(&u, g);
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(imza_dosyalari(&u), vec![format!("{a}-1.png.enc")]);
    }

    /// Sepet karekodu: n = toplam ürün, fişsiz varsa imza istenir; belirteç sepet kimliğine bağlı (sepet değişirse
    /// geçmez); müşterinin karekod imzası bütün kayıtlara bir kez yazılır; bulut kaydına kişisel veri gitmez.
    #[test]
    fn sepet_karekod_onayi() {
        let (u, bulut) = qr_uyg();
        let (a, b, c) = (hazir_fis(&u), hazir_fis(&u), hazir_fis(&u));
        let fisler = json!([{ "no": a, "okutuldu": true, "karekod": fis_kk(&u, &a) }, { "no": b, "okutuldu": false }]);
        let y = u.isle(&pc(
            "POST",
            "/api/teslim/sepet/onay-ac",
            &json!({ "fisler": fisler }).to_string(),
        ));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j = json_of(&y);
        let t = j["t"].as_str().unwrap().to_string();
        assert_eq!(j["imza_gerekli"], true);
        assert_eq!(j["toplam_urun"], 2);
        assert_eq!(j["sepet_kimligi"], sepet_kimligi(&[&b, &a]));
        assert_eq!(bulut.kayit.lock().unwrap()[&format!("{A}|{t}")]["i"], 1);
        assert!(bulut
            .istekler
            .lock()
            .unwrap()
            .iter()
            .any(|x| x.contains(&t) && x.contains(r#""n":2"#)));
        // müşteri imzalı onaylar (telefon fiştekinden farklı)
        let imza = imza_b64();
        bulut.musteri_onaylar_imzali(A, &t, QR_AD, QR_TEL, Some(&imza));
        let y = u.isle(&pc(
            "POST",
            "/api/teslim/sepet/onay-durum",
            &json!({ "fisler": fisler, "t": t }).to_string(),
        ));
        let j = json_of(&y);
        assert_eq!(j["durum"], "onaylandi", "{j}");
        assert_eq!(j["fis_telefonu"], "farkli");
        assert_eq!(j["imzali"], true);
        // başka sepet (c eklendi) bu belirteci kullanamaz
        let baska = json!([{ "no": a, "okutuldu": true, "karekod": fis_kk(&u, &a) }, { "no": b }, { "no": c }]);
        let y = teslim(
            &u,
            json!({ "fisler": baska, "onay_belirteci": t, "iliski": "yakini", "telefon_son4": "4567" }),
        );
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "onay_yok");
        // telefon farklı: ilişki seçilmeli
        let y = teslim(
            &u,
            json!({ "fisler": fisler, "onay_belirteci": t, "telefon_son4": { b.clone(): "4567" } }),
        );
        assert_eq!(json_of(&y)["hata"], "iliski_gerekli");
        let y = teslim(
            &u,
            json!({ "fisler": fisler, "onay_belirteci": t, "iliski": "yakini", "telefon_son4": { b.clone(): "4567" } }),
        );
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let (ta, tb) = (
            fis(&u, &a).teslim_kaydi.unwrap(),
            fis(&u, &b).teslim_kaydi.unwrap(),
        );
        assert_eq!(ta.dogrulama, "qr");
        assert_eq!(tb.dogrulama, "qr+telefon-son4");
        assert_eq!(ta.imza_yontemi, "qr_imza");
        assert_eq!(ta.imza_dosyasi, tb.imza_dosyasi);
        assert_eq!(ta.teslim_id, tb.teslim_id);
        assert_eq!(imza_dosyalari(&u), vec![format!("{a}-1.png.enc")]);
        // belirteç tek kullanımlık
        let y = teslim(
            &u,
            json!({ "fisler": [c], "onay_belirteci": t, "iliski": "yakini" }),
        );
        assert_eq!(y.kod, 422);
        // bulut kuyruğu: her fiş ayrı "teslim" kaydı; ad, telefon, teslim_id gitmez
        let kuyruk = std::fs::read_to_string(u.dizin.join("musteri-takip").join("bekleyen.json"))
            .unwrap_or_default();
        assert!(!kuyruk.is_empty());
        assert!(
            !kuyruk.contains("Zeynep")
                && !kuyruk.contains("9998877")
                && !kuyruk.contains(&ta.teslim_id)
        );
        let mut bulunan = Vec::new();
        tum_dosyalarda(&u.dizin, &["Zeynep", "9998877"], &mut bulunan);
        assert!(bulunan.is_empty(), "{bulunan:?}");
    }

    /// Denetçi bulgusu (yüksek): eski uç `teslim-al` + `"bagli": [B]` artık sepet akışına çevrilir. Bağlı B'nin
    /// karekodu okutulmadı: fişsizdir; yetkisiz telefon 403 alır, hiçbir fiş teslim edilmez. Yetkili cihazda imza
    /// ve B'nin müşterisi için son 4 ister; kayıt fisli=false / "telefon-son4" yazılır, ortak teslim_id ile.
    #[test]
    fn eski_uc_bagli_fissiz_sayilir_ve_yetkisiz_red() {
        let u = uyg();
        let a = hazir_fis(&u);
        let b = baska_fis(&u);
        bagla(&u, &a, &b);
        let token = "deneme-belirtec-0103";
        u.cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        let yol = format!("/api/fis/{a}/teslim-al");
        let g = json!({
            "fisli": true, "fis_karekodu": fis_kk(&u, &a), "bagli": [b],
            "teslim_alan": "Deneme Müşteri", "iliski": "kendisi",
        });
        let y = u.isle(&tel(token, "POST", &yol, &g.to_string()));
        assert_eq!(y.kod, 403, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(json_of(&y)["hata"], "yetki_yok");
        assert_eq!(json_of(&y)["fissiz"], json!([b]));
        assert_eq!(fis(&u, &a).durum, Durum::Hazir);
        assert_eq!(fis(&u, &b).durum, Durum::Hazir);
        // yetkili cihaz (PC): imzasız 422, son 4'süz 422; tam istekte ikisi birlikte, B fişsiz
        let y = u.isle(&pc("POST", &yol, &g.to_string()));
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "imza_gerekli");
        let mut g2 = g.clone();
        g2["imza_png_base64"] = json!(imza_b64());
        let y = u.isle(&pc("POST", &yol, &g2.to_string()));
        assert_eq!(y.kod, 422);
        assert_eq!(json_of(&y)["hata"], "son4_gerekli");
        assert_eq!(json_of(&y)["fisler"], json!([b]));
        // ana fişin son 4'ü (4567) B için geçmez
        g2["telefon_son4"] = json!({ b.clone(): "4567" });
        let y = u.isle(&pc("POST", &yol, &g2.to_string()));
        assert_eq!(json_of(&y)["hata"], "son4_yanlis");
        g2["telefon_son4"] = json!({ b.clone(): "2233" });
        let y = u.isle(&pc("POST", &yol, &g2.to_string()));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        let j = json_of(&y);
        assert_eq!(j["birlikte_teslim"], json!([b]));
        assert_eq!(j["birlikte_atlanan"], json!([]));
        let (ta, tb) = (
            fis(&u, &a).teslim_kaydi.unwrap(),
            fis(&u, &b).teslim_kaydi.unwrap(),
        );
        assert!(ta.fisli);
        assert_eq!(ta.dogrulama, "fis");
        assert!(!tb.fisli);
        assert_eq!(tb.dogrulama, "telefon-son4");
        assert_eq!(tb.ana_fis, a);
        assert!(!tb.imza_sha256.is_empty());
        assert_eq!(ta.imza_sha256, tb.imza_sha256);
        assert_eq!(ta.teslim_id, tb.teslim_id);
        // tek fişe açılmış karekod belirteci bağlı fişlerle kullanılamaz
        let (c, d2) = (hazir_fis(&u), hazir_fis(&u));
        bagla(&u, &c, &d2);
        let y = u.isle(&pc(
            "POST",
            &format!("/api/fis/{c}/teslim-al"),
            &json!({ "fisli": false, "bagli": [d2], "onay_belirteci": "abcdefghijkmnpqrstuvw" })
                .to_string(),
        ));
        assert_eq!(y.kod, 409);
        assert_eq!(json_of(&y)["hata"], "sepet_gerekli");
    }

    /// Denetçi bulgusu (yüksek): takip ayarı YOKKEN "#fis=<no>" elle yazılarak fişli kanıtı uydurulamaz.
    #[test]
    fn elle_yazilan_fis_karekodu_fissiz() {
        let u = uyg();
        let no = hazir_fis(&u);
        let token = "deneme-belirtec-0104";
        u.cihazlar()
            .ekle("Deneme telefon", token, "2026-11-26", "")
            .unwrap();
        let uydurma = format!("#fis={no}");
        let y = u.isle(&tel(
            token,
            "POST",
            "/api/teslim/sepet/denetle",
            &json!({ "fisler": [], "ekle": uydurma }).to_string(),
        ));
        let j = json_of(&y);
        assert_eq!(j["fisler"][0]["okutuldu"], false);
        assert_eq!(j["fisler"][0]["kanit"], "karekod-kanitsiz");
        assert!(j["kanit_notu"].as_str().unwrap().contains("kanıt parçası"));
        let g = json!({
            "fisler": [{ "no": no, "okutuldu": true, "karekod": uydurma }],
            "teslim_alan": "Deneme Müşteri", "iliski": "kendisi",
        });
        let y = u.isle(&tel(token, "POST", "/api/teslim/sepet", &g.to_string()));
        assert_eq!(y.kod, 403);
        assert_eq!(json_of(&y)["hata"], "yetki_yok");
        // gerçek basılı karekod (k doğru) fişlidir
        let g = json!({
            "fisler": [{ "no": no, "okutuldu": true, "karekod": u.fis_karekod_adresi(&no) }],
            "teslim_alan": "Deneme Müşteri", "iliski": "kendisi",
        });
        let y = u.isle(&tel(token, "POST", "/api/teslim/sepet", &g.to_string()));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(fis(&u, &no).teslim_kaydi.unwrap().dogrulama, "fis");
    }

    /// Denetçi bulgusu (orta): 30'u aşan sepet sessizce kırpılmaz; teslimde ve karekodda tümden reddedilir (413),
    /// denetlede fazlası reddedilen/uyarı olarak bildirilir.
    #[test]
    fn sepet_siniri_asilinca_tumden_red() {
        let u = uyg();
        let nolar: Vec<String> = (0..EN_COK_SEPET + 2).map(|_| hazir_fis(&u)).collect();
        let fisler: Vec<_> = nolar
            .iter()
            .map(|n| json!({ "no": n, "okutuldu": true, "karekod": fis_kk(&u, n) }))
            .collect();
        let j = denetle(&u, &json!(fisler), "");
        assert_eq!(j["fisler"].as_array().unwrap().len(), EN_COK_SEPET);
        assert_eq!(j["fazla"].as_array().unwrap().len(), 2);
        assert_eq!(j["dolu"], true);
        assert!(j["uyarilar"].to_string().contains("sepet sınırı aşıldı"));
        // dolu sepete okutma: eklenmez, açıkça söylenir
        let j2 = denetle(
            &u,
            &json!(fisler[..EN_COK_SEPET]),
            &fis_kk(&u, &nolar[EN_COK_SEPET]),
        );
        assert_eq!(j2["eklenen"], json!([]));
        assert!(j2["ekle_hatasi"].as_str().unwrap().contains("Sepet dolu"));
        let y = teslim(
            &u,
            json!({ "fisler": fisler, "teslim_alan": "Deneme Müşteri", "iliski": "kendisi" }),
        );
        assert_eq!(y.kod, 413, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(json_of(&y)["hata"], "sepet_cok_buyuk");
        assert!(nolar.iter().all(|n| fis(&u, n).durum == Durum::Hazir));
        let y = u.isle(&pc(
            "POST",
            "/api/teslim/sepet/onay-ac",
            &json!({ "fisler": fisler }).to_string(),
        ));
        assert_eq!(y.kod, 413);
        // tam 30 geçer
        let y = teslim(
            &u,
            json!({ "fisler": fisler[..EN_COK_SEPET], "teslim_alan": "Deneme Müşteri", "iliski": "kendisi" }),
        );
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(
            json_of(&y)["fisler"].as_array().unwrap().len(),
            EN_COK_SEPET
        );
    }

    /// Denetçi bulgusu (düşük): karekod onayında ürün sayısı 50'ye sıkıştırılmaz; fazlası 409 `qr_cok_urun`.
    #[test]
    fn karekod_onayi_50_urunu_asamaz() {
        let (u, bulut) = qr_uyg();
        let g = FIS.replace(
            r#"{"urun":"Pantolon","islem":"paça kısalt"}"#,
            r#"{"urun":"Pantolon","islem":"paça kısalt","adet":20},{"urun":"Ceket","islem":"kol kısalt","adet":20}"#,
        );
        let mut nolar = Vec::new();
        for _ in 0..2 {
            let y = u.isle(&pc("POST", "/api/fis", &g));
            assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
            let no = json_of(&y)["no"].as_str().unwrap().to_string();
            u.isle(&pc(
                "POST",
                &format!("/api/fis/{no}/durum"),
                r#"{"durum":"hazir"}"#,
            ));
            nolar.push(no);
        }
        assert_eq!(urun_sayisi(&fis(&u, &nolar[0])), 40);
        let fisler: Vec<_> = nolar
            .iter()
            .map(|n| json!({ "no": n, "okutuldu": true, "karekod": fis_kk(&u, n) }))
            .collect();
        let y = u.isle(&pc(
            "POST",
            "/api/teslim/sepet/onay-ac",
            &json!({ "fisler": fisler }).to_string(),
        ));
        assert_eq!(y.kod, 409, "{}", String::from_utf8_lossy(&y.govde));
        assert_eq!(json_of(&y)["hata"], "qr_cok_urun");
        assert_eq!(json_of(&y)["toplam_urun"], 80);
        assert!(bulut.istekler.lock().unwrap().is_empty());
        // tek fiş (40 ürün) karekodla olur
        let y = u.isle(&pc(
            "POST",
            "/api/teslim/sepet/onay-ac",
            &json!({ "fisler": fisler[..1] }).to_string(),
        ));
        assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
    }
}
