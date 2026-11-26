// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS10 2026-11-26]
//! Akşam ciro şablonu: şubelerin her akşam doldurduğu Excel'in ("KASIM AYI CİRO ŞABLONU 2026 V2") aynısını
//! TAA Kasa kayıtlarından üretir. Ay başına bir dosya, gün başına bir sayfa (`01.11.2026` …), sayfa düzeni
//! şablonla aynıdır: A sütunu satır adları, her çekmece bir mağaza sütunu (şablonda OUTLET MAĞAZA / ANA
//! MAĞAZA), ardından TOPLAM DÖVİZ ve tarih başlıklı toplam sütunu; G sütununda "NASIL DOLDURULUR?".
//!
//! İki çekmeceli mağazada hücre adresleri şablonla birebir aynıdır (A1:E40). Formüller şablondakiyle aynı
//! hesaplanır ve önbellek değeri olarak programın hesabı yazılır; Excel açılışta yeniden hesaplar.
//!
//! Şablondan bilinçli ayrılan yerler (docs/TAA-KASA.md):
//! - Masraf satırlarında (18–26) mağaza sütunlarına masrafın hangi çekmeceden çıktığı yazılır (şablonda
//!   gri ve boş); E sütunu (şablonda olduğu gibi sarı) satır toplamıdır. Böylece mağaza toplamları kendi masrafını ve kasa açığını içerir.
//!   Şablon tüm masrafı (öteki mağazanın kasa açığı dahil) son mağazanın toplamına yazıyordu; iki mağaza
//!   toplamının toplamı yine GENEL TOPLAM'a eşittir.
//! - Formüller tablo adı (`Tablo…[[#This Row],…]`) yerine düz hücre başvurusuyla yazılır; sonuç aynıdır.
//! - Satır 9 (eski E-FATURA POS:2) boş bırakılır: iki POS terminali var (Yusuf, 2026-11-26); eski kayıtta tutar
//!   varsa "(ESKİ KAYIT)" etiketiyle yazılır. Satır numaraları ve formüller değişmez.
//! - Veri doğrulama (−1.000.000.000 alt sınırı) yazılmaz; tutarları program zaten denetler.

use crate::gunsonu::{
    DovizSatiri, GunSonuKaydi, KasaAyar, MasrafDurumu, MasrafGorunum, BILGI_ALANLARI, DOVIZLER,
    ESKI_EFATURA, KASA_ACIGI, ODEME_TURLERI,
};
use crate::xlsx::{sutun, Hucre, Sayfa, Stil};
use crate::zaman;
use std::collections::BTreeMap;

/// Şablondaki en az masraf satırı sayısı (18–26). Masraf eklendikçe satır artar; alttaki bölüm kayar.
pub const MASRAF_SATIRI: usize = 9;

/// Bir mağaza sütunu (bir çekmece).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CiroSutunu {
    pub ad: String,
    pub ciro: BTreeMap<String, i64>,
    /// Döviz kodu → yabancı para (cent).
    pub doviz: BTreeMap<String, i64>,
    pub not: String,
    /// Bu çekmecenin o gün gün sonu kaydı var mı.
    pub gun_sonu_var: bool,
}

/// Bir günün şablon verisi.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CiroGunu {
    pub tarih: String,
    pub baslik: String,
    pub havale_adi: String,
    pub sutunlar: Vec<CiroSutunu>,
    /// Döviz kodu → kur (onbinde).
    pub kurlar: BTreeMap<String, i64>,
    /// (açıklama, sütun başına tutar).
    pub masraflar: Vec<(String, Vec<i64>)>,
}

/// `2026-11-01` → `01.11.2026` (şablonun sayfa adı ve E1 başlığı).
pub fn tarih_tr(t: &str) -> String {
    let p: Vec<&str> = t.split('-').collect();
    if p.len() == 3 {
        format!("{}.{}.{}", p[2], p[1], p[0])
    } else {
        t.to_string()
    }
}

/// Bir günün şablon verisini kayıtlardan çıkarır. İptal edilen gün sonları ve reddedilen/iptal edilen
/// masraflar girmez (masraf toplamıyla aynı kural).
pub fn ciro_gunu(
    ayar: &KasaAyar,
    tarih: &str,
    gun_sonlari: &[GunSonuKaydi],
    masraflar: &[MasrafGorunum],
) -> CiroGunu {
    let mut g = CiroGunu {
        tarih: tarih.to_string(),
        baslik: [ayar.excel_baslik.trim(), ayar.sube.trim()]
            .into_iter()
            .find(|x| !x.is_empty())
            .unwrap_or("MAĞAZA")
            .to_string(),
        havale_adi: if ayar.havale_hesap_adi.trim().is_empty() {
            "FİRMA HESABINA HAVALE".into()
        } else {
            ayar.havale_hesap_adi.trim().to_string()
        },
        ..Default::default()
    };
    for c in &ayar.cekmeceler {
        let mut s = CiroSutunu {
            ad: c.ad.clone(),
            ..Default::default()
        };
        // Düzeltmede eski kayıt iptal edilmiştir; yine de birden çok varsa en sonuncusu geçerlidir.
        if let Some(k) = gun_sonlari
            .iter()
            .rfind(|k| k.tarih == tarih && k.cekmece == c.id)
        {
            s.gun_sonu_var = true;
            s.ciro = k.ciro.clone();
            s.not = k.not.clone();
            for d in &k.doviz {
                *s.doviz.entry(d.kod.clone()).or_default() += d.tutar;
                if d.kur_onbinde > 0 {
                    g.kurlar.entry(d.kod.clone()).or_insert(d.kur_onbinde);
                }
            }
        }
        g.sutunlar.push(s);
    }
    let n = g.sutunlar.len();
    for m in masraflar.iter().filter(|m| {
        m.kayit.tarih == tarih
            && matches!(m.durum, MasrafDurumu::Bekliyor | MasrafDurumu::Onaylandi)
    }) {
        let Some(i) = ayar.cekmeceler.iter().position(|c| c.id == m.kayit.cekmece) else {
            continue;
        };
        let mut tutar = vec![0; n];
        tutar[i] = m.kayit.tutar_kurus;
        g.masraflar
            .push((crate::gunsonu::masraf_kalemi(&m.kayit), tutar));
    }
    g
}

impl CiroGunu {
    fn c(&self, i: usize, k: &str) -> i64 {
        self.sutunlar[i].ciro.get(k).copied().unwrap_or(0)
    }

    fn doviz_tl(&self, i: usize, kod: &str) -> i64 {
        DovizSatiri {
            kod: kod.into(),
            tutar: self.sutunlar[i].doviz.get(kod).copied().unwrap_or(0),
            kur_onbinde: self.kurlar.get(kod).copied().unwrap_or(0),
        }
        .tl_kurus()
    }

    fn masraf(&self, i: usize) -> i64 {
        self.masraflar.iter().map(|(_, v)| v[i]).sum()
    }

    /// Mağaza sütununun toplamı (şablondaki "… TOPLAMI").
    pub fn sutun_toplami(&self, i: usize) -> i64 {
        let ozel = ["kredi_ceki", "internet_degisimi"];
        ODEME_TURLERI
            .iter()
            .filter(|(k, _)| !ozel.contains(k))
            .map(|(k, _)| self.c(i, k))
            .sum::<i64>()
            + self.c(i, ESKI_EFATURA)
            + DOVIZLER
                .iter()
                .map(|(k, _)| self.doviz_tl(i, k))
                .sum::<i64>()
            + self.masraf(i)
            + ozel.iter().map(|k| self.c(i, k)).sum::<i64>()
    }

    /// Şablondaki ilk GENEL TOPLAM (E28): nakit, döviz, POS'lar, havale, İyzico ödemesi, toplam masraf.
    pub fn genel_toplam_ara(&self) -> i64 {
        (0..self.sutunlar.len())
            .map(|i| {
                self.sutun_toplami(i) - self.c(i, "kredi_ceki") - self.c(i, "internet_degisimi")
            })
            .sum()
    }

    /// Şablondaki son GENEL TOPLAM (E31) = E28 + kredi çeki + internet değişimi.
    pub fn genel_toplam(&self) -> i64 {
        (0..self.sutunlar.len())
            .map(|i| self.sutun_toplami(i))
            .sum()
    }

    /// Excel'deki NAKİT TL: sistem nakdi − kasa açığı (açık MASRAFLAR'da ayrıca yazılır).
    fn nakit_excel(&self, i: usize) -> i64 {
        self.c(i, "nakit") - self.c(i, KASA_ACIGI.0)
    }

    /// Şablon sayfası.
    pub fn sayfa(&self) -> Sayfa {
        let n = self.sutunlar.len().max(1);
        let sc: Vec<String> = (1..=n).map(sutun).collect();
        let e = sutun(n + 2);
        let gen = n + 4; // toplam sütunundan sonra boş F, sonra G (yardım)
                         // Masraf satırları: en az 9, masraf eklendikçe artar; alttaki satırlar `k` kadar kayar.
        let ms = self.masraflar.len().max(MASRAF_SATIRI);
        let k = ms - MASRAF_SATIRI;
        let r_mson = 17 + ms; // son masraf satırı (şablonda 26)
        let r_tm = 27 + k;
        let r_gt = 28 + k;
        let r_kc = 29 + k;
        let r_id = 30 + k;
        let r_gt2 = 31 + k;
        let r_toplam = 32 + k;
        let r_dbas = r_toplam + n;
        let r_kur = r_dbas + 1;
        let r_tl0 = r_kur + 1;
        let r_not = r_tl0 + n;
        let r_degisim = r_not + 1;
        let r_iade = r_degisim + 1;
        // Döviz TL satırı: şablonda son mağaza önce (36: ANA, 37: OUTLET).
        let r_tl = |i: usize| r_tl0 + (n - 1 - i);
        let kur_sut = |j: usize| sutun(1 + j);
        let topla = |r: usize| -> String {
            sc.iter()
                .map(|c| format!("{c}{r}"))
                .collect::<Vec<_>>()
                .join("+")
        };

        let mut satirlar: Vec<Vec<Hucre>> = vec![Vec::new(); r_iade];
        let mut koy = |r: usize, c: usize, h: Hucre| {
            let s = &mut satirlar[r - 1];
            if s.len() <= c {
                s.resize(c + 1, Hucre::Bos);
            }
            s[c] = h;
        };
        // Tablo alanı çerçeveli (A1:E…).
        for r in 1..=r_iade {
            for c in 0..=n + 2 {
                koy(r, c, Hucre::Bos.stil(Stil::Cerceve));
            }
        }
        let etiket = |m: &str| Hucre::from(m).stil(Stil::Etiket);
        let tl = |k: i64| Hucre::Ondalik(k, 2).stil(Stil::GirisTl);

        // 1: başlık
        koy(1, 0, Hucre::from(buyuk_tr(&self.baslik)).stil(Stil::Baslik));
        for (i, s) in self.sutunlar.iter().enumerate() {
            koy(1, 1 + i, Hucre::from(buyuk_tr(&s.ad)).stil(Stil::Baslik));
        }
        koy(1, n + 1, Hucre::from("TOPLAM DÖVİZ").stil(Stil::Baslik));
        koy(
            1,
            n + 2,
            Hucre::from(tarih_tr(&self.tarih)).stil(Stil::TarihBaslik),
        );
        koy(
            1,
            gen,
            Hucre::from("NASIL DOLDURULUR?").stil(Stil::YardimBaslik),
        );
        koy(
            2,
            gen,
            Hucre::from("Sarı kutuları doldurun").stil(Stil::Yardim),
        );
        koy(
            3,
            gen,
            Hucre::from("Gri/renkli kutular otomatik hesaplanır").stil(Stil::YardimGri),
        );
        koy(
            4,
            gen,
            Hucre::from("Tab / Enter: sıradaki sarı kutuya geçer").stil(Stil::YardimBeyaz),
        );
        koy(
            5,
            gen,
            Hucre::from("Bu dosya TAA Kasa kayıtlarından üretildi").stil(Stil::YardimBeyaz),
        );

        // Mağaza sütunlarına giriş + E'ye satır toplamı.
        let ciro_satiri =
            |koy: &mut dyn FnMut(usize, usize, Hucre), r: usize, ad: &str, k: &str| {
                koy(r, 0, etiket(ad));
                let mut t = 0;
                for i in 0..self.sutunlar.len() {
                    let v = self.c(i, k);
                    t += v;
                    koy(r, 1 + i, tl(v));
                }
                koy(r, n + 2, Hucre::Formul(topla(r), t).stil(Stil::HesapTl));
            };

        {
            koy(2, 0, etiket(ad("nakit")));
            let mut t = 0;
            for i in 0..self.sutunlar.len() {
                let v = self.nakit_excel(i);
                t += v;
                koy(2, 1 + i, tl(v));
            }
            koy(2, n + 2, Hucre::Formul(topla(2), t).stil(Stil::HesapTl));
        }
        // 3–6: döviz
        let giris_doviz = [
            Stil::GirisUsd,
            Stil::GirisEur,
            Stil::GirisGbp,
            Stil::GirisChf,
        ];
        let hesap_doviz = [
            Stil::HesapUsd,
            Stil::HesapEur,
            Stil::HesapGbp,
            Stil::HesapChf,
        ];
        let doviz_adlari = ["DOLAR", "EURO", "STERLİN", "İSVİÇRE FRANGI"];
        for (j, (kod, _)) in DOVIZLER.iter().enumerate() {
            let r = 3 + j;
            koy(r, 0, etiket(doviz_adlari[j]));
            let mut adet = 0;
            for (i, s) in self.sutunlar.iter().enumerate() {
                let v = s.doviz.get(*kod).copied().unwrap_or(0);
                adet += v;
                koy(r, 1 + i, Hucre::Ondalik(v, 2).stil(giris_doviz[j]));
            }
            koy(r, n + 1, Hucre::Formul(topla(r), adet).stil(hesap_doviz[j]));
            let ks = kur_sut(j);
            let f = (0..n)
                .rev()
                .map(|i| format!("{ks}{}", r_tl(i)))
                .collect::<Vec<_>>()
                .join("+");
            let t: i64 = (0..self.sutunlar.len())
                .map(|i| self.doviz_tl(i, kod))
                .sum();
            koy(r, n + 2, Hucre::Formul(f, t).stil(Stil::HesapTl));
        }
        ciro_satiri(&mut koy, 7, ad("ziraat_pos"), "ziraat_pos");
        ciro_satiri(&mut koy, 8, ad("iyzico_pos"), "iyzico_pos");
        // 9: eski E-FATURA POS:2 satırı. Kaldırıldı; satır boş kalır, formüller (E28 = E2:E9) aynen durur.
        // Eski kayıtta tutar varsa okunur ve "(eski kayıt)" etiketiyle yazılır.
        let eski = (0..self.sutunlar.len()).any(|i| self.c(i, ESKI_EFATURA) != 0);
        ciro_satiri(
            &mut koy,
            9,
            if eski {
                "E-FATURA POS:2 (ESKİ KAYIT)"
            } else {
                ""
            },
            ESKI_EFATURA,
        );
        ciro_satiri(&mut koy, 10, ad("kredi_karti_fark"), "kredi_karti_fark");
        ciro_satiri(&mut koy, 11, ad("kasa_fazlasi"), "kasa_fazlasi");
        // 12: BANKAYA HAVALE bölümü
        for c in 0..=n + 1 {
            koy(12, c, Hucre::Bos.stil(Stil::Bolum));
        }
        koy(12, 0, Hucre::from("BANKAYA HAVALE").stil(Stil::Bolum));
        koy(12, n + 2, Hucre::from("TUTAR").stil(Stil::BolumOrta));
        ciro_satiri(&mut koy, 13, &buyuk_tr(&self.havale_adi), "havale");
        ciro_satiri(&mut koy, 14, ad("iyzico_odemesi"), "iyzico_odemesi");
        ciro_satiri(&mut koy, 15, ad("havale_fark"), "havale_fark");
        // 16: MASRAFLAR bölümü
        for c in 0..=n + 2 {
            koy(16, c, Hucre::Bos.stil(Stil::Bolum));
        }
        koy(16, 0, Hucre::from("MASRAFLAR").stil(Stil::Bolum));
        ciro_satiri(&mut koy, 17, KASA_ACIGI.1, KASA_ACIGI.0);
        for j in 0..ms {
            let r = 18 + j;
            let (ad, v) = self
                .masraflar
                .get(j)
                .cloned()
                .unwrap_or_else(|| (String::new(), vec![0; self.sutunlar.len()]));
            koy(r, 0, Hucre::from(ad).stil(Stil::GirisMetin));
            for (i, t) in v.iter().enumerate() {
                koy(r, 1 + i, Hucre::Ondalik(*t, 2).stil(Stil::HesapTl));
            }
            koy(r, n + 1, Hucre::Bos.stil(Stil::HesapBos));
            // Şablonda E18:E26 elle girilen (sarı) tutardır; program satır toplamını yazar.
            koy(r, n + 2, tl(v.iter().sum()));
        }
        let toplam_masraf: i64 = (0..self.sutunlar.len())
            .map(|i| self.c(i, KASA_ACIGI.0) + self.masraf(i))
            .sum();
        for c in 0..=n + 1 {
            koy(r_tm, c, Hucre::Bos.stil(Stil::ToplamMasraf));
        }
        koy(
            r_tm,
            0,
            Hucre::from("TOPLAM MASRAF").stil(Stil::ToplamMasraf),
        );
        koy(
            r_tm,
            n + 2,
            Hucre::Formul(format!("SUM({e}17:{e}{r_mson})"), toplam_masraf)
                .stil(Stil::ToplamMasrafTutar),
        );
        for c in 0..=n + 1 {
            koy(r_gt, c, Hucre::Bos.stil(Stil::GenelToplam));
        }
        koy(r_gt, 0, Hucre::from("GENEL TOPLAM").stil(Stil::GenelToplam));
        koy(
            r_gt,
            n + 2,
            Hucre::Formul(
                format!("SUM({e}2:{e}9,{e}13:{e}14,{e}{r_tm})"),
                self.genel_toplam_ara(),
            )
            .stil(Stil::GenelToplamTutar),
        );
        ciro_satiri(&mut koy, r_kc, ad("kredi_ceki"), "kredi_ceki");
        ciro_satiri(&mut koy, r_id, ad("internet_degisimi"), "internet_degisimi");
        for c in 0..=n + 1 {
            koy(r_gt2, c, Hucre::Bos.stil(Stil::GenelToplam2));
        }
        koy(
            r_gt2,
            0,
            Hucre::from("GENEL TOPLAM").stil(Stil::GenelToplam2),
        );
        koy(
            r_gt2,
            n + 2,
            Hucre::Formul(
                format!("{e}{r_gt}+{e}{r_kc}+{e}{r_id}"),
                self.genel_toplam(),
            )
            .stil(Stil::GenelToplam2Tutar),
        );
        // Mağaza toplamları
        for (i, s) in self.sutunlar.iter().enumerate() {
            let r = r_toplam + i;
            let x = &sc[i];
            for c in 0..=n + 1 {
                koy(r, c, Hucre::Bos.stil(Stil::MagazaToplam));
            }
            koy(
                r,
                0,
                Hucre::from(format!("{} TOPLAMI", buyuk_tr(&s.ad))).stil(Stil::MagazaToplam),
            );
            let tlr = r_tl(i);
            koy(
                r,
                n + 2,
                Hucre::Formul(
                    format!(
                        "SUM({x}2,{x}7:{x}9,{x}13:{x}14,{x}17:{x}{r_mson},{x}{r_kc}:{x}{r_id},B{tlr}:E{tlr})"
                    ),
                    self.sutun_toplami(i),
                )
                .stil(Stil::MagazaToplamTutar),
            );
        }
        // Döviz başlıkları ve günlük kur
        for (j, ad) in doviz_adlari.iter().enumerate() {
            koy(r_dbas, 1 + j, Hucre::from(*ad).stil(Stil::BolumOrta));
            let kur = self.kurlar.get(DOVIZLER[j].0).copied().unwrap_or(0);
            koy(r_kur, 1 + j, Hucre::Ondalik(kur, 4).stil(Stil::GirisKur));
        }
        koy(
            r_kur,
            0,
            Hucre::from("GÜNLÜK DÖVİZ KURLARI ($/€)").stil(Stil::Bolum),
        );
        koy(r_dbas, 0, Hucre::Bos.stil(Stil::Bolum));
        for (i, s) in self.sutunlar.iter().enumerate() {
            let r = r_tl(i);
            koy(
                r,
                0,
                // Şablondaki yazımla aynı: ilk mağazada "DÖVİZ KURU İLE", ötekilerde "DÖVİZ KURU".
                etiket(&format!(
                    "{} DÖVİZ KURU {}TL KARŞILIĞI",
                    buyuk_tr(&s.ad),
                    if i == 0 { "İLE " } else { "" }
                )),
            );
            for (j, (kod, _)) in DOVIZLER.iter().enumerate() {
                let ks = kur_sut(j);
                koy(
                    r,
                    1 + j,
                    Hucre::Formul(
                        format!("{}{}*{ks}{r_kur}", sc[i], 3 + j),
                        self.doviz_tl(i, kod),
                    )
                    .stil(Stil::HesapTl),
                );
            }
        }
        // Not
        koy(r_not, 0, etiket("NOT:"));
        let not = self
            .sutunlar
            .iter()
            .filter(|s| !s.not.trim().is_empty())
            .map(|s| format!("{}: {}", s.ad, s.not.trim()))
            .collect::<Vec<_>>()
            .join(" · ");
        for c in 1..=n + 2 {
            koy(r_not, c, Hucre::Bos.stil(Stil::GirisMetin));
        }
        koy(r_not, 1, Hucre::from(not).stil(Stil::GirisMetin));
        ciro_satiri(&mut koy, r_degisim, ad("degisim"), "degisim");
        ciro_satiri(&mut koy, r_iade, ad("iade"), "iade");

        let mut genislikler = vec![52.0];
        genislikler.extend(std::iter::repeat_n(24.0, n + 2));
        genislikler.extend([3.0, 34.0]);
        Sayfa {
            ad: tarih_tr(&self.tarih),
            satirlar,
            genislikler,
            dondur: Some("B2".into()),
            koru: true,
            yazdirma_alani: Some(format!("$A$1:${e}${r_iade}")),
        }
    }
}

/// Türkçe büyük harf (i → İ, ı → I).
pub fn buyuk_tr(s: &str) -> String {
    s.replace('i', "İ").replace('ı', "I").to_uppercase()
}

/// Ciro alanının şablondaki adı.
fn ad(k: &'static str) -> &'static str {
    ODEME_TURLERI
        .iter()
        .chain(BILGI_ALANLARI.iter())
        .find(|(a, _)| *a == k)
        .map(|(_, b)| *b)
        .unwrap_or(k)
}

/// Ayın günleri (`2026-11` → 2026-11-01 … 2026-11-30).
pub fn ayin_gunleri(ay: &str) -> Result<Vec<String>, String> {
    let hata = || format!("ay YYYY-AA olmalı: {ay:?}");
    let (y, m) = ay.split_once('-').ok_or_else(hata)?;
    let y: i64 = y.parse().map_err(|_| hata())?;
    let m: u32 = m.parse().map_err(|_| hata())?;
    if !(1..=12).contains(&m) || !(2000..=2100).contains(&y) {
        return Err(hata());
    }
    let bas = zaman::takvimden_gun(y, m, 1);
    let son = if m == 12 {
        zaman::takvimden_gun(y + 1, 1, 1)
    } else {
        zaman::takvimden_gun(y, m + 1, 1)
    };
    Ok((bas..son)
        .map(|g| {
            let (y, m, d) = zaman::gunden_takvim(g);
            format!("{y:04}-{m:02}-{d:02}")
        })
        .collect())
}

/// Aylık ciro şablonu (xlsx): ayın her günü için bir sayfa; kaydı olmayan gün boş şablondur.
pub fn ciro_sablonu(
    ayar: &KasaAyar,
    ay: &str,
    gun_sonlari: &[GunSonuKaydi],
    masraflar: &[MasrafGorunum],
) -> Result<Vec<u8>, String> {
    let sayfalar: Vec<Sayfa> = ayin_gunleri(ay)?
        .iter()
        .map(|t| ciro_gunu(ayar, t, gun_sonlari, masraflar).sayfa())
        .collect();
    Ok(crate::xlsx::xlsx_uret(&sayfalar))
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::gunsonu::{Cekmece, MasrafKaydi, OnayDuzeyi};
    use serde_json::json;

    fn ayar() -> KasaAyar {
        let mut a = KasaAyar {
            sube: "Deneme Şubesi".into(),
            ..KasaAyar::default()
        };
        let n = a.cekmeceler[0].noktalar.clone();
        a.cekmeceler = vec![
            Cekmece {
                id: "outlet".into(),
                ad: "Outlet Mağaza".into(),
                noktalar: n.clone(),
            },
            Cekmece {
                id: "ana".into(),
                ad: "Ana Mağaza".into(),
                noktalar: n
                    .into_iter()
                    .map(|mut x| {
                        x.id = "k2".into();
                        x
                    })
                    .collect(),
            },
        ];
        a
    }

    fn gs(cekmece: &str, ciro: serde_json::Value, doviz: serde_json::Value) -> GunSonuKaydi {
        serde_json::from_value(json!({
            "id": format!("GS-{cekmece}"), "tarih": "2026-11-26", "sube": "D", "cekmece": cekmece,
            "kisi": "A", "nakit_adetler": {}, "nakit_sayilan_kurus": 0, "noktalar": [],
            "nebim_toplam_kurus": 0, "ciro": ciro, "doviz": doviz, "devreden_kurus": 0,
            "kontrol": [], "not": format!("{cekmece} notu"),
            "karsilastirma": {"satirlar": [], "ciro_toplam_kurus": 0, "fark_var": false,
                              "kasa_ciro_uyusmuyor": false},
            "zaman": 0, "duzeltir": null
        }))
        .unwrap()
    }

    fn masraf(id: &str, cekmece: &str, tutar: i64, durum: MasrafDurumu) -> MasrafGorunum {
        MasrafGorunum {
            kayit: MasrafKaydi {
                id: id.into(),
                tarih: "2026-11-26".into(),
                cekmece: cekmece.into(),
                kisi: "A".into(),
                tur: "Kargo".into(),
                tutar_kurus: tutar,
                aciklama: String::new(),
                not: String::new(),
                foto: None,
                onay_duzeyi: OnayDuzeyi::Mudur,
                zaman: 0,
                duzeltir: None,
            },
            durum,
            karar_veren: None,
        }
    }

    fn ornek() -> CiroGunu {
        let g = vec![
            gs(
                "outlet",
                json!({"nakit": 100_000, "ziraat_pos": 50_000, "iyzico_pos": 25_000,
                       "kredi_karti_fark": -1_000, "havale": 30_000, "kasa_acigi": 500,
                       "kredi_ceki": 2_000, "iade": 1_500}),
                json!([{"kod": "USD", "tutar": 1_000, "kur_onbinde": 400_000}]),
            ),
            gs(
                "ana",
                json!({"nakit": 200_000, "iyzico_odemesi": 10_000, "internet_degisimi": 3_000}),
                json!([{"kod": "EUR", "tutar": 500, "kur_onbinde": 455_000}]),
            ),
        ];
        let m = vec![
            masraf("MS-1", "ana", 15_000, MasrafDurumu::Onaylandi),
            masraf("MS-2", "outlet", 8_000, MasrafDurumu::Bekliyor),
            masraf("MS-3", "ana", 99_900, MasrafDurumu::Reddedildi),
        ];
        ciro_gunu(&ayar(), "2026-11-26", &g, &m)
    }

    #[test]
    fn eski_efatura_kaydi_okunur_ve_toplama_girer() {
        let g = vec![gs(
            "outlet",
            json!({"nakit": 1_000, "efatura_pos": 500, "kredi_ceki": -200}),
            json!([]),
        )];
        let c = ciro_gunu(&ayar(), "2026-11-26", &g, &[]);
        assert_eq!(c.sutun_toplami(0), 1_000 + 500 - 200);
        let b = crate::xlsx::xlsx_uret(&[c.sayfa()]);
        let x = String::from_utf8_lossy(&b);
        assert!(x.contains("E-FATURA POS:2 (ESKİ KAYIT)"));
        assert!(
            x.contains("<c r=\"B29\" s=\"6\"><v>-2.00</v></c>"),
            "eksi kredi çeki yazılır"
        );
    }

    #[test]
    fn toplamlar_sablon_formulune_uyar() {
        let g = ornek();
        // Outlet: 1000 (sistem nakdi; 5 TL açık bunun içinde) +500+250 (iki POS)+300 + 400 (10 USD) + 80 masraf
        // + 20 kredi çeki = 2.550
        assert_eq!(g.sutun_toplami(0), 255_000);
        // Ana: 2000+100 + 227,50 (5 EUR × 45,5) + 150 masraf + 30 internet = 2.507,50
        assert_eq!(g.sutun_toplami(1), 250_750);
        assert_eq!(g.genel_toplam(), 505_750);
        assert_eq!(g.genel_toplam_ara(), 505_750 - 2_000 - 3_000);
        assert_eq!(g.masraflar.len(), 2, "reddedilen masraf girmez");
    }

    #[test]
    fn sayfa_duzeni_iki_magazada_sablonla_ayni() {
        let g = ornek();
        let b = crate::xlsx::xlsx_uret(&[g.sayfa()]);
        let x = String::from_utf8_lossy(&b);
        for parca in [
            "name=\"26.11.2026\"",
            ">DENEME ŞUBESİ<",
            "width=\"3\"",
            "r=\"B1\" t=\"inlineStr\" s=\"3\"><is><t xml:space=\"preserve\">OUTLET MAĞAZA<",
            "r=\"C1\" t=\"inlineStr\" s=\"3\"><is><t xml:space=\"preserve\">ANA MAĞAZA<",
            ">TOPLAM DÖVİZ<",
            "r=\"E1\" t=\"inlineStr\" s=\"4\"><is><t xml:space=\"preserve\">26.11.2026<",
            "r=\"A2\" t=\"inlineStr\" s=\"5\"><is><t xml:space=\"preserve\">NAKİT TL<",
            // NAKİT TL = sistem nakdi − kasa açığı (açık MASRAFLAR'da)
            "<c r=\"B2\" s=\"6\"><v>995.00</v></c>",
            "<c r=\"E2\" s=\"11\"><f>B2+C2</f><v>2995.00</v></c>",
            "<c r=\"D3\" s=\"12\"><f>B3+C3</f><v>10.00</v></c>",
            "<c r=\"E3\" s=\"11\"><f>B36+B37</f><v>400.00</v></c>",
            ">ZİRAAT BANKASI POS:1<",
            "<c r=\"B10\" s=\"6\"><v>-10.00</v></c>",
            ">FİRMA HESABINA HAVALE<",
            ">KASA AÇIĞI<",
            "<c r=\"E27\" s=\"18\"><f>SUM(E17:E26)</f><v>235.00</v></c>",
            "<c r=\"E28\" s=\"20\"><f>SUM(E2:E9,E13:E14,E27)</f><v>5007.50</v></c>",
            "<c r=\"E31\" s=\"22\"><f>E28+E29+E30</f><v>5057.50</v></c>",
            "<f>SUM(B2,B7:B9,B13:B14,B17:B26,B29:B30,B37:E37)</f><v>2550.00</v>",
            "<f>SUM(C2,C7:C9,C13:C14,C17:C26,C29:C30,B36:E36)</f><v>2507.50</v>",
            ">OUTLET MAĞAZA TOPLAMI<",
            ">ANA MAĞAZA DÖVİZ KURU TL KARŞILIĞI<",
            ">OUTLET MAĞAZA DÖVİZ KURU İLE TL KARŞILIĞI<",
            "<c r=\"E18\" s=\"6\"><v>150.00</v></c>",
            "<c r=\"C35\" s=\"27\"><v>45.5000</v></c>",
            "<c r=\"C36\" s=\"11\"><f>C4*C35</f><v>227.50</v></c>",
            "<c r=\"B37\" s=\"11\"><f>B3*B35</f><v>400.00</v></c>",
            "r=\"A38\" t=\"inlineStr\" s=\"5\"><is><t xml:space=\"preserve\">NOT:<",
            "r=\"A40\" t=\"inlineStr\" s=\"5\"><is><t xml:space=\"preserve\">İADE<",
            "r=\"G1\" t=\"inlineStr\" s=\"28\"><is><t xml:space=\"preserve\">NASIL DOLDURULUR?<",
            "state=\"frozen\"",
            "<sheetProtection sheet=\"1\"",
            "'26.11.2026'!$A$1:$E$40",
            "fullCalcOnLoad=\"1\"",
        ] {
            assert!(x.contains(parca), "eksik: {parca}");
        }
        assert!(!x.contains("<c r=\"A41\""), "şablon 40 satır");
        // E-FATURA POS kaldırıldı: 9. satır etiketsiz, formül yerinde (satır numaraları değişmedi).
        assert!(!x.contains("E-FATURA"), "E-FATURA POS satırı yazılmaz");
        assert!(x.contains("<c r=\"E9\""), "9. satır formülü durur");
    }

    #[test]
    fn ay_dosyasi_her_gun_bir_sayfa() {
        assert_eq!(ayin_gunleri("2026-11").unwrap().len(), 30);
        assert_eq!(ayin_gunleri("2028-02").unwrap().len(), 29);
        assert_eq!(ayin_gunleri("2026-12").unwrap()[30], "2026-12-31");
        assert!(ayin_gunleri("2026-13").is_err());
        let b = ciro_sablonu(&ayar(), "2026-11", &[], &[]).unwrap();
        let x = String::from_utf8_lossy(&b);
        assert!(x.contains("name=\"01.11.2026\"") && x.contains("name=\"30.11.2026\""));
        assert!(!x.contains("name=\"31.11.2026\""));
    }

    #[test]
    fn masraf_eklendikce_satir_artar_ve_alt_bolum_kayar() {
        let m: Vec<MasrafGorunum> = (0..12)
            .map(|i| masraf(&format!("MS-{i}"), "outlet", 100, MasrafDurumu::Onaylandi))
            .collect();
        let g = ciro_gunu(&ayar(), "2026-11-26", &[], &m);
        assert_eq!(g.masraflar.len(), 12, "birleştirme yok");
        assert_eq!(g.genel_toplam(), 1_200);
        let b = crate::xlsx::xlsx_uret(&[g.sayfa()]);
        let x = String::from_utf8_lossy(&b);
        for parca in [
            "<c r=\"E29\" s=\"6\"><v>1.00</v></c>",
            "r=\"A30\" t=\"inlineStr\" s=\"17\"><is><t xml:space=\"preserve\">TOPLAM MASRAF<",
            "<f>SUM(E17:E29)</f><v>12.00</v>",
            "<f>SUM(E2:E9,E13:E14,E30)</f><v>12.00</v>",
            "<f>E31+E32+E33</f><v>12.00</v>",
            "<f>SUM(B2,B7:B9,B13:B14,B17:B29,B32:B33,B40:E40)</f><v>12.00</v>",
            "r=\"A43\" t=\"inlineStr\" s=\"5\"><is><t xml:space=\"preserve\">İADE<",
            "'26.11.2026'!$A$1:$E$43",
        ] {
            assert!(x.contains(parca), "eksik: {parca}");
        }
    }
}
