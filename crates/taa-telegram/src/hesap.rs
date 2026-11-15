//! Telegram hesabı ile usta/mağaza kaydı eşlemesi (TT12c, Fable denetimi K1, K6, K10).
//! Akış: YÖNETİCİ bir usta kaydı için kısa ömürlü tek kullanımlık bağlama kodu üretir; usta kodu bota yazar;
//! kod geçerliyse hesap o ustaya bağlanır ve "yönetici onayı bekliyor" olur; son onayı yalnız yönetici ekranı verir.
//! Geçerli kod göndermeyen hesap için HİÇBİR ŞEY kaydedilmez (yabancıların Telegram kimliği/profil adı saklanmaz).
//! Dayanak Telegram'ın değişmeyen sayısal kimliğidir (`tg_id`); profil adı yalnız görünen, GÜVENİLMEZ metindir (temizlenir, kısaltılır).
//! Yalnız eklemeli JSONL (`hesaplar.jsonl`, `baglama-kodlari.jsonl`), silme yok. Veriler çalışan kişisel verisidir: yalnız sunucuda, yalnız yönetici görür.
//! Sahip: Yusuf. yazan: [Claude Sonnet 5.5, TT başlığı 2026-11-15].

use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rol {
    Usta,
    /// Satış/tezgâh personeli: fiş açar, durum sorar, raf yazar; usta imzası ve WhatsApp düğmesi YOK.
    Personel,
    /// Mağaza telefonu: ortak hesap; yalnız "müşteriye haber ver" ve "gönderdim"; usta imzası ATAMAZ (K8).
    Magaza,
    Yonetici,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum HesapDurum {
    /// Kod ile bağlandı, yönetici son onayı bekleniyor.
    OnayBekliyor,
    Onayli,
    Askida,
    Kaldirildi,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hesap {
    pub tg_id: i64,
    /// Usta/personel kimliği: rastgele yerel `U-…`; addan ya da Telegram kimliğinden türetilmez (KM2'de eşlenir).
    pub usta_id: String,
    pub rol: Rol,
    pub sube: String,
    pub durum: HesapDurum,
    /// Bağlama anındaki profil adı (temizlenmiş; görünen ad, kimlik dayanağı DEĞİL).
    pub profil_adi: String,
    pub saat: String,
    pub onaylayan: String,
    pub sira: u64,
    /// Yöneticinin kendi notu (ör. "kol ustası"); yalnız yönetici ekranında görünür, imzaya girmez.
    #[serde(default)]
    pub not: String,
    /// Bağlanınca telefonda ve yönetici ekranında AYNI görünen 4 haneli doğrulama sayısı: yönetici onaylamadan önce
    /// kişinin telefonundaki sayıyla karşılaştırır (yanlış kişiye QR gitmesine karşı).
    #[serde(default)]
    pub dogrulama: String,
}

/// Yönetici QR'ının içeriği: kare kodu okutan telefonda Telegram bu bağlantıyla botu açar.
pub fn baglama_baglantisi(bot_adi: &str, kod: &str) -> String {
    format!("https://t.me/{bot_adi}?start=baglan-{kod}")
}

/// Profil adı güvenilmez metindir: denetim karakterleri çıkar, boşluklar sadeleşir, en çok 40 karakter.
pub fn profil_temizle(s: &str) -> String {
    let t: String = s
        .chars()
        .filter(|c| {
            !c.is_control()
                && !matches!(*c, '\u{200B}'..='\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}' | '\u{FEFF}')
        })
        .collect();
    let t = t.split_whitespace().collect::<Vec<_>>().join(" ");
    let t: String = t.chars().take(40).collect();
    if t.is_empty() {
        "?".into()
    } else {
        t
    }
}

fn dogrulama_sayisi() -> Result<String, String> {
    let mut b = [0u8; 2];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(format!("{:04}", u16::from_le_bytes(b) % 10_000))
}

/// Yeni rastgele usta kimliği (`U-` + 8 onaltılık hane).
pub fn yeni_usta_id() -> Result<String, String> {
    let mut b = [0u8; 4];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(format!(
        "U-{}",
        b.iter().map(|x| format!("{x:02x}")).collect::<String>()
    ))
}

/// "1234-5678", "12345678" -> "12345678" (8 hane değilse None).
pub fn kod_ayikla(metin: &str) -> Option<String> {
    let k: String = metin
        .trim()
        .chars()
        .filter(|c| *c != '-' && *c != ' ')
        .collect();
    (k.len() == 8 && k.chars().all(|c| c.is_ascii_digit())).then_some(k)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct KodKaydi {
    kod: String,
    usta_id: String,
    rol: Rol,
    sube: String,
    #[serde(default)]
    not: String,
    olusturan: String,
    /// Unix saniye.
    bitis: u64,
    kullanildi: bool,
}

pub struct Hesaplar<'a> {
    pub dizin: &'a Path,
}

impl Hesaplar<'_> {
    fn hesap_yolu(&self) -> std::path::PathBuf {
        self.dizin.join("hesaplar.jsonl")
    }
    fn kod_yolu(&self) -> std::path::PathBuf {
        self.dizin.join("baglama-kodlari.jsonl")
    }

    fn satirlar(&self) -> Result<Vec<Hesap>, String> {
        taa_dosya::satirlari_oku(&self.hesap_yolu())
            .map_err(|e| e.to_string())?
            .iter()
            .map(|l| serde_json::from_str(l).map_err(|e| format!("hesap satırı bozuk: {e}")))
            .collect()
    }

    /// Her `tg_id` için son satır (güncel durum).
    pub fn guncel(&self) -> Result<Vec<Hesap>, String> {
        let mut v: Vec<Hesap> = Vec::new();
        for h in self.satirlar()? {
            match v.iter_mut().find(|x| x.tg_id == h.tg_id) {
                Some(x) => *x = h,
                None => v.push(h),
            }
        }
        Ok(v)
    }

    pub fn bul(&self, tg_id: i64) -> Result<Option<Hesap>, String> {
        Ok(self.guncel()?.into_iter().find(|h| h.tg_id == tg_id))
    }

    fn yaz(&self, h: &Hesap) -> Result<(), String> {
        let s = serde_json::to_string(h).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.hesap_yolu(), &s).map_err(|e| e.to_string())
    }

    fn sonraki(&self) -> Result<u64, String> {
        Ok(self.satirlar()?.len() as u64 + 1)
    }

    fn kodlar(&self) -> Result<Vec<KodKaydi>, String> {
        taa_dosya::satirlari_oku(&self.kod_yolu())
            .map_err(|e| e.to_string())?
            .iter()
            .map(|l| serde_json::from_str(l).map_err(|e| format!("kod satırı bozuk: {e}")))
            .collect()
    }

    fn usta_bagli_mi(&self, usta_id: &str, haric_tg: i64) -> Result<bool, String> {
        Ok(self.guncel()?.iter().any(|x| {
            x.tg_id != haric_tg
                && x.usta_id == usta_id
                && matches!(x.durum, HesapDurum::Onayli | HesapDurum::OnayBekliyor)
        }))
    }

    #[allow(clippy::too_many_arguments)]
    /// YÖNETİCİ ekranı çağırır. `usta_id` boşsa yeni rastgele kimlik üretilir. Dönen: (bağlama kodu, usta_id).
    /// Kod 8 haneli rastgele sayıdır, `omur_sn` saniye geçerlidir, tek kullanımlıktır.
    pub fn kod_uret(
        &self,
        usta_id: &str,
        rol: Rol,
        sube: &str,
        not: &str,
        yonetici: &str,
        simdi: u64,
        omur_sn: u64,
    ) -> Result<(String, String), String> {
        if yonetici.trim().is_empty() {
            return Err("kodu üreten yönetici kaydı gerekli".into());
        }
        let usta_id = if usta_id.trim().is_empty() {
            yeni_usta_id()?
        } else {
            usta_id.trim().to_string()
        };
        if self.usta_bagli_mi(&usta_id, 0)? {
            return Err(format!(
                "{usta_id} zaten bir hesaba bağlı; önce o hesabı askıya alın"
            ));
        }
        let mut b = [0u8; 4];
        getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
        let kod = format!("{:08}", u32::from_le_bytes(b) % 100_000_000);
        let k = KodKaydi {
            kod: kod.clone(),
            usta_id: usta_id.clone(),
            rol,
            sube: sube.into(),
            not: not.trim().chars().take(40).collect(),
            olusturan: yonetici.trim().into(),
            bitis: simdi + omur_sn,
            kullanildi: false,
        };
        let s = serde_json::to_string(&k).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.kod_yolu(), &s).map_err(|e| e.to_string())?;
        Ok((kod, usta_id))
    }

    /// Usta kodu bota yazınca: kod geçerliyse hesap bağlanır (`OnayBekliyor`). Geçersizse hiçbir şey yazılmaz.
    pub fn kod_ile_bagla(
        &self,
        tg_id: i64,
        profil_adi: &str,
        kod: &str,
        simdi: u64,
        saat: &str,
    ) -> Result<Hesap, String> {
        let k = self
            .kodlar()?
            .into_iter()
            .rev()
            .find(|k| k.kod == kod)
            .ok_or("kod geçersiz")?;
        if k.kullanildi {
            return Err("kod geçersiz".into());
        }
        if simdi > k.bitis {
            return Err("kodun süresi dolmuş; yöneticiden yenisini isteyin".into());
        }
        if self.usta_bagli_mi(&k.usta_id, tg_id)? {
            return Err("bu usta kaydı başka hesaba bağlı".into());
        }
        let h = Hesap {
            tg_id,
            usta_id: k.usta_id.clone(),
            rol: k.rol,
            sube: k.sube.clone(),
            not: k.not.clone(),
            dogrulama: dogrulama_sayisi()?,
            durum: HesapDurum::OnayBekliyor,
            profil_adi: profil_temizle(profil_adi),
            saat: saat.into(),
            onaylayan: String::new(),
            sira: self.sonraki()?,
        };
        self.yaz(&h)?;
        let kullanildi = KodKaydi {
            kullanildi: true,
            ..k
        };
        let s = serde_json::to_string(&kullanildi).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.kod_yolu(), &s).map_err(|e| e.to_string())?;
        Ok(h)
    }

    /// Yönetici son onayı (yönetici ekranı çağırır; Telegram'dan onay YOK).
    pub fn onayla(&self, tg_id: i64, yonetici: &str, saat: &str) -> Result<Hesap, String> {
        if yonetici.trim().is_empty() {
            return Err("onaylayan yönetici gerekli".into());
        }
        let mut h = self.bul(tg_id)?.ok_or("bu hesap bağlanmamış")?;
        if h.durum != HesapDurum::OnayBekliyor {
            return Err("onay bekleyen hesap değil".into());
        }
        h.durum = HesapDurum::Onayli;
        h.onaylayan = yonetici.trim().into();
        h.saat = saat.into();
        h.sira = self.sonraki()?;
        self.yaz(&h)?;
        Ok(h)
    }

    pub fn durum_ata(
        &self,
        tg_id: i64,
        durum: HesapDurum,
        yonetici: &str,
        saat: &str,
    ) -> Result<(), String> {
        let mut h = self.bul(tg_id)?.ok_or("hesap yok")?;
        h.durum = durum;
        h.onaylayan = yonetici.into();
        h.saat = saat.into();
        h.sira = self.sonraki()?;
        self.yaz(&h)
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kodlu_baglama_akisi() {
        let d = taa_dosya::gecici_klasor().unwrap();
        let h = Hesaplar { dizin: &d };
        // kodsuz/yanlış kodla hiçbir şey kaydedilmez
        assert!(h.kod_ile_bagla(5, "Yabancı", "12345678", 100, "g").is_err());
        assert!(!d.join("hesaplar.jsonl").exists());
        let (kod, uid) = h
            .kod_uret("", Rol::Usta, "Esenyurt", "kol ustası", "Y-1", 1000, 600)
            .unwrap();
        assert!(uid.starts_with("U-") && kod.len() == 8);
        // süresi dolunca olmaz
        assert!(h
            .kod_ile_bagla(5, "Ahmet", &kod, 1601, "g")
            .unwrap_err()
            .contains("süresi"));
        let b = h
            .kod_ile_bagla(5, "Ahmet\u{202e}\n Y", &kod, 1500, "2026-11-15")
            .unwrap();
        assert_eq!(b.durum, HesapDurum::OnayBekliyor);
        assert_eq!(b.profil_adi, "Ahmet Y");
        assert_eq!(b.not, "kol ustası");
        assert_eq!(b.dogrulama.len(), 4);
        assert_eq!(
            baglama_baglantisi("bot", &kod),
            format!("https://t.me/bot?start=baglan-{kod}")
        );
        // tek kullanımlık
        assert!(h.kod_ile_bagla(6, "Başka", &kod, 1500, "g").is_err());
        assert!(h.bul(6).unwrap().is_none());
        // yönetici son onayı
        assert!(h.onayla(5, "", "g").is_err());
        let o = h.onayla(5, "Y-1", "2026-11-15").unwrap();
        assert_eq!(o.durum, HesapDurum::Onayli);
        assert!(h.onayla(5, "Y-1", "g").is_err());
        // aynı usta kimliği ikinci etkin hesaba verilemez
        assert!(h
            .kod_uret(&uid, Rol::Usta, "E", "", "Y-1", 1000, 600)
            .is_err());
        h.durum_ata(5, HesapDurum::Askida, "Y-1", "g").unwrap();
        assert!(h
            .kod_uret(&uid, Rol::Usta, "E", "", "Y-1", 1000, 600)
            .is_ok());
    }

    #[test]
    fn yardimcilar() {
        assert_eq!(kod_ayikla("1234-5678").as_deref(), Some("12345678"));
        assert!(kod_ayikla("1234567").is_none() && kod_ayikla("abcd1234").is_none());
        assert_eq!(profil_temizle("  \u{0}\u{202e}"), "?");
        assert_eq!(profil_temizle(&"a".repeat(80)).len(), 40);
    }
}
