//! taa-takip: TaaTerziTakip raporları. Veri yerel fiş defterinden (`fisler.jsonl`) okunur; Google Sheets ve bulut YOK.
//! Salt okunur: defteri değiştirmez, yazma işleri `taa-fis` içindedir.
//! Usta performansı SAYI verir (biten, zamanında, geciken); iş raporu işlem başına adet ve zorluk puanı verir.
//! Zorluk puanları düzenlenebilir başlangıç değerleridir (zorluk.json). Prim formülü/oranı YOKTUR.
//! Özellik anahtarı `fis-takip`: varsayılan AÇIK. Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-05].

mod zorluk;
pub use zorluk::Zorluk;

use serde::Serialize;
use std::collections::BTreeMap;
use taa_fis::{Defter, Durum, Fis};

const OZELLIK: &str = "fis-takip";

/// Türkçe büyük/küçük harf farkını (İ/I) gözeterek küçültür.
fn kucuk(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'İ' => vec!['i'],
            'I' => vec!['ı'],
            c => c.to_lowercase().collect(),
        })
        .collect()
}

/// Fiş hâlâ işlemde mi (teslim edilmedi, iptal edilmedi)?
fn acik(f: &Fis) -> bool {
    matches!(f.durum, Durum::Bekliyor | Durum::Terzihanede | Durum::Hazir)
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Ozet {
    pub bekliyor: usize,
    pub terzihanede: usize,
    pub hazir: usize,
    pub geciken: usize,
    pub bugun_teslim: usize,
    pub kargo_bekleyen: usize,
    pub kuru_temizleme: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UstaSatiri {
    pub usta: String,
    /// Hazır/teslim durumuna geçmiş fiş sayısı.
    pub biten: usize,
    /// Hazır işareti teslim tarihinden önce ya da o gün atılanlar.
    pub zamaninda: usize,
    pub gec: usize,
    /// Hâlâ işlemde olan ve teslim tarihi geçmiş fişler.
    pub geciken_acik: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct IslemSayisi {
    /// Katalog işlem kodu; serbest yazılan işlemde "serbest".
    pub kod: String,
    pub ad: String,
    pub adet: usize,
    /// Tek işlemin zorluk puanı (1–5).
    pub puan: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct UstaIsRaporu {
    pub usta: String,
    /// Biten fişlerdeki toplam işlem (kalem) adedi.
    pub islem_adedi: usize,
    /// Her işlemin adedi x zorluk puanı toplamı.
    pub agirlikli_puan: u32,
    pub islemler: Vec<IslemSayisi>,
}

pub struct Takip {
    fisler: Vec<Fis>,
    /// (fiş no, durum, tarih), defter sırasıyla.
    gecmis: Vec<(String, Durum, String)>,
}

impl Takip {
    pub fn defterden(d: &Defter) -> Result<Takip, String> {
        if !taa_ayar::ozellik_acik(OZELLIK, true) {
            return Err(taa_ayar::kapali_mesaji(OZELLIK));
        }
        Ok(Takip {
            fisler: d.fisler()?,
            gecmis: d.durum_gecmisi()?,
        })
    }

    /// Kargo işaretli ve henüz teslim/iptal olmayan fişler (kargoya verilmesi gerekenler).
    pub fn kargo_bekleyen(&self) -> Vec<&Fis> {
        self.fisler.iter().filter(|f| f.kargo && acik(f)).collect()
    }

    /// Kalemlerinden biri kuru temizleme olan, işlemdeki fişler (dışarıya gönderilen işler için takip).
    pub fn kuru_temizleme(&self) -> Vec<&Fis> {
        self.fisler
            .iter()
            .filter(|f| {
                acik(f)
                    && f.kalemler.iter().any(|k| {
                        let m = kucuk(&format!("{} {}", k.urun, k.islem));
                        m.contains("kuru temiz")
                    })
            })
            .collect()
    }

    pub fn geciken(&self, bugun: &str) -> Vec<&Fis> {
        self.fisler
            .iter()
            .filter(|f| acik(f) && f.teslim.as_str() < bugun)
            .collect()
    }

    pub fn bugun_teslim(&self, bugun: &str) -> Vec<&Fis> {
        self.fisler
            .iter()
            .filter(|f| acik(f) && f.teslim == bugun)
            .collect()
    }

    pub fn ozet(&self, bugun: &str) -> Ozet {
        let say = |d: Durum| self.fisler.iter().filter(|f| f.durum == d).count();
        Ozet {
            bekliyor: say(Durum::Bekliyor),
            terzihanede: say(Durum::Terzihanede),
            hazir: say(Durum::Hazir),
            geciken: self.geciken(bugun).len(),
            bugun_teslim: self.bugun_teslim(bugun).len(),
            kargo_bekleyen: self.kargo_bekleyen().len(),
            kuru_temizleme: self.kuru_temizleme().len(),
        }
    }

    /// Ustanın hangi işlemden kaç tane bitirdiği ve zorluğa göre ağırlıklı puanı.
    /// Biten = en az bir kez "Hazır" ya da "Teslim" olmuş fiş (iptal sayılmaz). Puan prim DEĞİLDİR.
    pub fn usta_is_raporu(&self, z: &Zorluk) -> Vec<UstaIsRaporu> {
        let mut m: BTreeMap<String, BTreeMap<String, IslemSayisi>> = BTreeMap::new();
        for f in &self.fisler {
            let biten = self
                .gecmis
                .iter()
                .any(|(no, d, _)| *no == f.no && matches!(d, Durum::Hazir | Durum::Teslim));
            if !biten {
                continue;
            }
            let usta = if f.usta.trim().is_empty() {
                "(atanmamış)".to_string()
            } else {
                f.usta.trim().to_string()
            };
            let islemler = m.entry(usta).or_default();
            for k in &f.kalemler {
                let (kod, ad) = match &k.islem_kodu {
                    Some(c) => (c.clone(), k.islem.clone()),
                    None => ("serbest".to_string(), format!("{} - {}", k.urun, k.islem)),
                };
                let anahtar = format!("{kod}|{ad}");
                islemler
                    .entry(anahtar)
                    .or_insert(IslemSayisi {
                        puan: z.puan(k.islem_kodu.as_deref()),
                        kod,
                        ad,
                        adet: 0,
                    })
                    .adet += 1;
            }
        }
        m.into_iter()
            .map(|(usta, islemler)| {
                let islemler: Vec<IslemSayisi> = islemler.into_values().collect();
                UstaIsRaporu {
                    usta,
                    islem_adedi: islemler.iter().map(|i| i.adet).sum(),
                    agirlikli_puan: islemler.iter().map(|i| i.adet as u32 * i.puan).sum(),
                    islemler,
                }
            })
            .collect()
    }

    /// Fişte atanmış ustalara göre sayılar. Usta atanmamış fişler "(atanmamış)" altında toplanır.
    pub fn usta_performansi(&self, bugun: &str) -> Vec<UstaSatiri> {
        let mut m: BTreeMap<String, UstaSatiri> = BTreeMap::new();
        for f in &self.fisler {
            let ad = if f.usta.trim().is_empty() {
                "(atanmamış)".to_string()
            } else {
                f.usta.trim().to_string()
            };
            let s = m.entry(ad.clone()).or_insert(UstaSatiri {
                usta: ad,
                biten: 0,
                zamaninda: 0,
                gec: 0,
                geciken_acik: 0,
            });
            // ilk "Hazır" (ya da doğrudan Teslim) tarihi bitiş kabul edilir
            let bitis = self
                .gecmis
                .iter()
                .find(|(no, d, _)| *no == f.no && matches!(d, Durum::Hazir | Durum::Teslim))
                .map(|(_, _, t)| t.as_str());
            if let Some(t) = bitis {
                s.biten += 1;
                if t <= f.teslim.as_str() {
                    s.zamaninda += 1;
                } else {
                    s.gec += 1;
                }
            } else if acik(f) && f.teslim.as_str() < bugun {
                s.geciken_acik += 1;
            }
        }
        m.into_values().collect()
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use taa_fis::{Girdi, Kalem};

    fn kalem(urun: &str, islem: &str) -> Kalem {
        Kalem {
            urun: urun.into(),
            islem: islem.into(),
            not: String::new(),
            islem_kodu: None,
            varyant: None,
            olcu_mm: None,
            pay_mm: None,
            usta_onayli: false,
            igneli: false,
        }
    }

    fn girdi(musteri: &str, teslim: &str, usta: &str, kalemler: Vec<Kalem>, kargo: bool) -> Girdi {
        Girdi {
            tarih: "2026-11-01".into(),
            musteri: musteri.into(),
            telefon: "05321234567".into(),
            kalemler,
            olcu_notu: String::new(),
            prova: String::new(),
            teslim: teslim.into(),
            teslim_saati: "17:00".into(),
            ucret_kurus: 0,
            on_odeme_kurus: 0,
            usta: usta.into(),
            satis_danismani: "Mert".into(),
            olcu_alan: "Kadir Y.".into(),
            kargo,
            kargo_notu: String::new(),
            kargo_ad: if kargo {
                "Ali Veli".into()
            } else {
                String::new()
            },
            kargo_adres: if kargo {
                "Atatürk Mah. 5. Sk. No:3 Çankaya/Ankara".into()
            } else {
                String::new()
            },
            kargo_telefon: if kargo {
                "05329998877".into()
            } else {
                String::new()
            },
            bildirim_izni: false,
        }
    }

    #[test]
    fn listeler_ve_usta_sayilari() {
        let dir = taa_dosya::gecici_klasor().unwrap();
        let y = dir.join("d.jsonl");
        let d = Defter { yol: &y };
        // 1: zamanında biten (usta Hasan); 2: geç biten (Hasan); 3: açık geciken (Ayten, kuru temizleme);
        // 4: kargo bekleyen; 5: teslim edilmiş kargo (listede olmaz)
        let f1 = d
            .ekle(girdi(
                "Ayşe Çelik",
                "2026-11-10",
                "Hasan",
                vec![kalem("Ceket", "paça kısalt")],
                false,
            ))
            .unwrap();
        let f2 = d
            .ekle(girdi(
                "Mehmet Kaya",
                "2026-11-03",
                "Hasan",
                vec![kalem("Pantolon", "bel daralt")],
                false,
            ))
            .unwrap();
        let _f3 = d
            .ekle(girdi(
                "Zeynep Er",
                "2026-11-02",
                "Ayten",
                vec![kalem("Palto", "Kuru Temizleme")],
                false,
            ))
            .unwrap();
        let f4 = d
            .ekle(girdi(
                "Can Tekin",
                "2026-11-12",
                "Ayten",
                vec![kalem("Gömlek", "kol kısalt")],
                true,
            ))
            .unwrap();
        let f5 = d
            .ekle(girdi(
                "Deniz Ak",
                "2026-11-12",
                "Ayten",
                vec![kalem("Gömlek", "kol kısalt")],
                true,
            ))
            .unwrap();
        d.durum_ata(&f1.no, Durum::Hazir, "2026-11-08").unwrap();
        d.durum_ata(&f2.no, Durum::Hazir, "2026-11-05").unwrap(); // teslim 11-03: geç
        for st in [Durum::Hazir, Durum::Teslim] {
            d.durum_ata(&f5.no, st, "2026-11-06").unwrap();
        }
        let t = Takip::defterden(&d).unwrap();
        assert_eq!(t.kargo_bekleyen().len(), 1);
        assert_eq!(t.kargo_bekleyen()[0].no, f4.no);
        assert_eq!(t.kuru_temizleme().len(), 1);
        let g: Vec<&str> = t
            .geciken("2026-11-06")
            .iter()
            .map(|f| f.teslim.as_str())
            .collect();
        assert_eq!(g, ["2026-11-03", "2026-11-02"]); // f2 hazır ama teslim edilmedi: hâlâ açık
        let o = t.ozet("2026-11-06");
        assert_eq!(
            (o.hazir, o.bekliyor, o.kargo_bekleyen, o.kuru_temizleme),
            (2, 2, 1, 1)
        );
        let u = t.usta_performansi("2026-11-06");
        let h = u.iter().find(|x| x.usta == "Hasan").unwrap();
        assert_eq!((h.biten, h.zamaninda, h.gec, h.geciken_acik), (2, 1, 1, 0));
        let a = u.iter().find(|x| x.usta == "Ayten").unwrap();
        assert_eq!((a.biten, a.zamaninda, a.geciken_acik), (1, 1, 1));
        // iş raporu: yalnız biten fişler; serbest işlem varsayılan puan (2) alır
        let r = t.usta_is_raporu(&Zorluk::varsayilan());
        let ra = r.iter().find(|x| x.usta == "Ayten").unwrap();
        assert_eq!((ra.islem_adedi, ra.agirlikli_puan), (1, 2));
        let rh = r.iter().find(|x| x.usta == "Hasan").unwrap();
        assert_eq!(rh.islem_adedi, 2);
        assert_eq!(rh.agirlikli_puan, 4);
    }

    #[test]
    fn turkce_kucuk_harf() {
        assert_eq!(kucuk("KURU TEMİZLEME"), "kuru temizleme");
        assert_eq!(kucuk("Iğdır"), "ığdır");
    }

    #[test]
    fn usta_is_raporu_zorluk_agirligi() {
        let z = Zorluk::varsayilan();
        assert_eq!(z.puan(Some("cek-omuz-kol-kisalt")), 5);
        assert_eq!(z.puan(Some("yok")), 2);
        assert_eq!(z.puan(None), 2);
        assert!(Zorluk::json_den(r#"{"varsayilan_puan":9,"puanlar":{}}"#).is_err());
        assert!(Zorluk::json_den("bozuk").is_err());
        let ozel = Zorluk::json_den(r#"{"varsayilan_puan":1,"puanlar":{"x":4}}"#).unwrap();
        assert_eq!(ozel.puan(Some("x")), 4);
        assert_eq!(ozel.puan(None), 1);
    }
}
