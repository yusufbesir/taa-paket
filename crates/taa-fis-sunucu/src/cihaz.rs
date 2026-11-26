//! Eşleşmiş telefonlar: eklemeli JSONL kayıt; belirteç yalnız SHA256 özeti olarak saklanır.
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cihaz {
    pub id: String,
    pub ad: String,
    pub token_sha256: String,
    pub tarih: String,
    /// Kişisel telefon (kullanıcı girişi): fişi düzenleyen her zaman bu kişidir. Boş = mağaza telefonu.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kisi_id: String,
    /// Yalnız mağaza bilgisayarında (id "PC"), istek anında: oturumdaki kişi — yönetici oturumunun kişisi (ortak giriş
    /// kullanıcısı ya da "PC (PIN)") ya da ortak girişle giren kullanıcı (çakışan adlar "ortak:" önekli). Kaydedilmez, okunmaz (bkz. lib.rs `pc_kimi`).
    /// Günlüklerde ve kayıtlardaki "kim" alanında "PC" / "Mağaza PC" yerine bu yazılır.
    #[serde(skip)]
    pub oturum_kisi: String,
}

impl Cihaz {
    /// Fiş düzeltme ve teslim kaydındaki "kim": kişisel telefonda kişi kimliği, mağaza bilgisayarında oturumdaki
    /// kişi (varsa), yoksa cihaz adı (eski davranış).
    pub fn kayit_kimi(&self) -> String {
        if !self.kisi_id.is_empty() {
            self.kisi_id.clone()
        } else if !self.oturum_kisi.is_empty() {
            self.oturum_kisi.clone()
        } else {
            self.ad.clone()
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "tur", content = "v", rename_all = "lowercase")]
enum Kayit {
    Cihaz(Cihaz),
    Kapat {
        id: String,
        tarih: String,
    },
    /// Cihaz adı yalnız mağaza bilgisayarındaki programdan (yönetici) değişir.
    Ad {
        id: String,
        ad: String,
        tarih: String,
    },
    /// Yönetici bir cihaza yetki verir ya da geri alır (mağaza PC'si için id "PC").
    Yetki {
        id: String,
        yetki: String,
        ver: bool,
        tarih: String,
    },
}

/// Cihaza verilebilen yetkiler: (anahtar, açıklama, mağaza PC'sinde varsayılan, telefonda varsayılan).
pub const YETKILER: &[(&str, &str, bool, bool)] = &[
    (
        "duzelt",
        "Fiş düzeltme: sonradan işlem ekleme/çıkarma, kargo ↔ mağaza, teslim tarihi, fiş bağlama",
        true,
        false,
    ),
    (
        "fissiz-teslim",
        "Fişsiz teslim: müşteri fişiyle gelmediğinde telefonun son 4 hanesi ve imzayla teslim",
        true,
        false,
    ),
    // Yusuf 2026-11-26: "yalnızca yöneticiler ve mağaza müdürleri görebilsin. bu ayarları yine yönetici
    // değiştirebilsin". Varsayılan HER YERDE kapalı; açık görünüm şu yollarla gelir (bkz. lib.rs
    // `teslim_acik_yetkisi`): mağaza bilgisayarında yönetici (PIN) oturumu; kişisel telefonda rolü yönetici ya da
    // mağaza müdürü olan kişi (`TESLIM_ACIK_ROLLER`); yöneticinin bu yetkiyi verdiği cihaz ya da kişi. Yönetici
    // rol varsayılanını da kişi bazında geri alabilir. Her açık görüntüleme denetim günlüğüne yazılır.
    (
        "teslim-acik",
        "Teslim bilgisini açık görme: teslim alanın tam adı, telefonu, IP'si ve imzası (her görüntüleme günlüğe yazılır)",
        false,
        false,
    ),
    // Koordinatör kararı 2026-11-26 (geri alınabilir varsayılan): müşteri nüshasının YENİDEN basımı ve müşteri
    // nüshası karekodunun (kanıt parçalı personel karekodu) ekranda yeniden gösterimi. Varsayılan HER YERDE açık
    // (eski davranış); yönetici cihaz bazında kapatabilir. İlk basım ve terzihane nüshası etkilenmez. Her yeniden
    // basım `basim.jsonl` günlüğüne yazılır (bkz. lib.rs `musteri_nushasi_izni`).
    (
        "yeniden-yazdir",
        "Müşteri nüshasını yeniden yazdırma ve müşteri karekodunu ekranda yeniden gösterme (her yeniden basım günlüğe yazılır)",
        true,
        true,
    ),
];

/// "teslim-acik" yetkisini varsayılan olarak taşıyan kişi rolleri (kisi.rs ROLLER).
pub const TESLIM_ACIK_ROLLER: &[&str] = &["yonetici", "magaza-muduru"];
/// Rol varsayılanının geçtiği kişi kaynakları: rolü yöneticinin BU bilgisayarda verdiği kişiler ("" = Personel
/// ekranı, "ilk-kurulum"). TAA Çatı (unvan metninden tahmin) ve Telegram rolü kendiliğinden "teslim-acik" vermez.
pub const ROL_VARSAYILANI_KAYNAKLARI: &[&str] = &["", "ilk-kurulum"];

/// Bu rol + kaynak "teslim-acik" yetkisini varsayılan olarak taşır mı?
pub fn rol_varsayilani_acik(rol: &str, kaynak: &str) -> bool {
    TESLIM_ACIK_ROLLER.contains(&rol) && ROL_VARSAYILANI_KAYNAKLARI.contains(&kaynak)
}
/// Kişi bazında verilebilen yetkiler (cihazdan bağımsız; kişinin bütün kişisel telefonlarına geçer).
pub const KISI_YETKILERI: &[&str] = &["teslim-acik"];

/// Kişi yetkisi kaydının kimliği (cihaz kimlikleriyle karışmasın).
pub fn kisi_anahtari(kisi_id: &str) -> String {
    format!("kisi:{kisi_id}")
}

pub struct Kayitci {
    pub yol: PathBuf,
}

pub fn sha256_hex(s: &str) -> String {
    Sha256::digest(s.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// 32 bayt işletim sistemi rastgeleliği, onaltılık.
pub fn yeni_token() -> Result<String, String> {
    let mut b = [0u8; 32];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(b.iter().map(|x| format!("{x:02x}")).collect())
}

/// 6 haneli eşleştirme kodu.
pub fn yeni_kod() -> Result<String, String> {
    let mut b = [0u8; 4];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(format!("{:06}", u32::from_le_bytes(b) % 1_000_000))
}

/// 4 haneli ikinci eşleştirme kodu (telefonda görünür, yönetici bilgisayara yazar).
pub fn yeni_kod2() -> Result<String, String> {
    let mut b = [0u8; 4];
    getrandom::fill(&mut b).map_err(|e| format!("rastgele sayı alınamadı: {e}"))?;
    Ok(format!("{:04}", u32::from_le_bytes(b) % 10_000))
}

/// Sabit zamanlı karşılaştırma (belirteç özeti).
fn esit(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0u8, |d, (x, y)| d | (x ^ y)) == 0
}

impl Kayitci {
    fn oku(&self) -> Result<Vec<Kayit>, String> {
        taa_dosya::satirlari_oku(&self.yol)
            .map_err(|e| e.to_string())?
            .iter()
            .map(|l| serde_json::from_str(l).map_err(|e| format!("cihaz kaydı bozuk: {e}")))
            .collect()
    }

    pub fn aktifler(&self) -> Result<Vec<Cihaz>, String> {
        let k = self.oku()?;
        let kapali: Vec<&str> = k
            .iter()
            .filter_map(|x| {
                if let Kayit::Kapat { id, .. } = x {
                    Some(id.as_str())
                } else {
                    None
                }
            })
            .collect();
        Ok(k.iter()
            .filter_map(|x| {
                if let Kayit::Cihaz(c) = x {
                    Some(c.clone())
                } else {
                    None
                }
            })
            .filter(|c| !kapali.contains(&c.id.as_str()))
            .map(|mut c| {
                for x in &k {
                    if let Kayit::Ad { id, ad, .. } = x {
                        if *id == c.id {
                            c.ad = ad.clone();
                        }
                    }
                }
                c
            })
            .collect())
    }

    /// Cihaz adını değiştirir (yalnız yönetici, mağaza bilgisayarından çağrılır).
    pub fn ad_degistir(&self, id: &str, ad: &str, tarih: &str) -> Result<(), String> {
        let ad = ad.trim();
        if ad.is_empty() || ad.chars().count() > 40 || taa_fis::gorunmez_karakter_var(ad) {
            return Err("cihaz adı 1-40 karakter olmalı (görünmez karakter olmadan)".into());
        }
        if !self.aktifler()?.iter().any(|c| c.id == id) {
            return Err(format!("aktif cihaz yok: {id}"));
        }
        let s = serde_json::to_string(&Kayit::Ad {
            id: id.into(),
            ad: ad.into(),
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.yol, &s).map_err(|e| e.to_string())
    }

    /// Yetki ver/geri al. id: aktif cihaz ya da "PC" (mağaza bilgisayarı).
    pub fn yetki_ata(&self, id: &str, yetki: &str, ver: bool, tarih: &str) -> Result<(), String> {
        if !YETKILER.iter().any(|(k, ..)| *k == yetki) {
            return Err(format!("bilinmeyen yetki: {yetki}"));
        }
        if id != crate::PC_ID && !self.aktifler()?.iter().any(|c| c.id == id) {
            return Err(format!("aktif cihaz yok: {id}"));
        }
        let s = serde_json::to_string(&Kayit::Yetki {
            id: id.into(),
            yetki: yetki.into(),
            ver,
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.yol, &s).map_err(|e| e.to_string())
    }

    /// Kişiye (cihazdan bağımsız) yetki verir/geri alır; yalnız `KISI_YETKILERI`. Kişinin varlığını çağıran denetler.
    pub fn kisi_yetki_ata(
        &self,
        kisi_id: &str,
        yetki: &str,
        ver: bool,
        tarih: &str,
    ) -> Result<(), String> {
        if !KISI_YETKILERI.contains(&yetki) {
            return Err(format!("kişiye verilemeyen yetki: {yetki}"));
        }
        if !crate::kisi::kimlik_gecerli(kisi_id) {
            return Err("kişi kimliği geçersiz".into());
        }
        let s = serde_json::to_string(&Kayit::Yetki {
            id: kisi_anahtari(kisi_id),
            yetki: yetki.into(),
            ver,
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.yol, &s).map_err(|e| e.to_string())
    }

    /// Bu kimlik (cihaz, "PC" ya da `kisi_anahtari`) için yöneticinin yazdığı son yetki kaydı; yoksa `None`.
    pub fn yetki_kaydi(&self, id: &str, yetki: &str) -> Option<bool> {
        let mut v = None;
        for x in self.oku().ok()? {
            if let Kayit::Yetki {
                id: i,
                yetki: y,
                ver,
                ..
            } = x
            {
                if i == id && y == yetki {
                    v = Some(ver);
                }
            }
        }
        v
    }

    /// Cihazın yetkisi var mı? Son kayıt geçerli; kayıt yoksa varsayılan (PC: açık, telefon: kapalı).
    /// "teslim-acik" için asıl karar `Uygulama::teslim_acik_yetkisi`dedir (kişi/rol ve yönetici oturumu).
    pub fn yetki_var(&self, id: &str, yetki: &str) -> bool {
        let Some((_, _, pc, tel)) = YETKILER.iter().find(|(k, ..)| *k == yetki) else {
            return false;
        };
        let mut v = if id == crate::PC_ID { *pc } else { *tel };
        if let Ok(k) = self.oku() {
            for x in &k {
                if let Kayit::Yetki {
                    id: i,
                    yetki: y,
                    ver,
                    ..
                } = x
                {
                    if i == id && y == yetki {
                        v = *ver;
                    }
                }
            }
        }
        v
    }

    /// Cihazın açık yetkileri (liste ekranı için).
    pub fn yetkileri(&self, id: &str) -> Vec<&'static str> {
        YETKILER
            .iter()
            .map(|(k, ..)| *k)
            .filter(|k| self.yetki_var(id, k))
            .collect()
    }

    pub fn ekle(&self, ad: &str, token: &str, tarih: &str, kisi_id: &str) -> Result<Cihaz, String> {
        let ad = ad.trim();
        if ad.is_empty() || ad.chars().count() > 40 || taa_fis::gorunmez_karakter_var(ad) {
            return Err("telefon adı 1-40 karakter olmalı (görünmez karakter olmadan)".into());
        }
        let n = self
            .oku()?
            .iter()
            .filter(|k| matches!(k, Kayit::Cihaz(_)))
            .count()
            + 1;
        let c = Cihaz {
            id: format!("T-{n:03}"),
            ad: ad.into(),
            token_sha256: sha256_hex(token),
            tarih: tarih.into(),
            kisi_id: kisi_id.into(),
            oturum_kisi: String::new(),
        };
        let s = serde_json::to_string(&Kayit::Cihaz(c.clone())).map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.yol, &s).map_err(|e| e.to_string())?;
        Ok(c)
    }

    pub fn kapat(&self, id: &str, tarih: &str) -> Result<(), String> {
        if !self.aktifler()?.iter().any(|c| c.id == id) {
            return Err(format!("aktif telefon yok: {id}"));
        }
        let s = serde_json::to_string(&Kayit::Kapat {
            id: id.into(),
            tarih: tarih.into(),
        })
        .map_err(|e| e.to_string())?;
        taa_dosya::satir_ekle(&self.yol, &s).map_err(|e| e.to_string())
    }

    pub fn dogrula(&self, token: &str) -> Option<Cihaz> {
        let h = sha256_hex(token);
        self.aktifler()
            .ok()?
            .into_iter()
            .find(|c| esit(&c.token_sha256, &h))
    }
}
