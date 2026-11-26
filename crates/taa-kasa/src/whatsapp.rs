// Sahip: Yusuf. yazan: [Claude#taa-kasa-WA 2026-11-26]
//! Akşam WhatsApp özeti: mağazaların her akşam WhatsApp'tan gönderdiği düz yazı, gün sonu verisinden.
//! Biçim Yusuf'un örnek metninden birebir alınmıştır (satır sırası, "MERKEZ MAĞAZA gg.aa.yyyy" başlığı,
//! "1.234,56 TL" / sıfır için "0 TL", "Nakit: x TL yy$(z TL)", masraf kalemleri parantez içinde, son satır
//! "MAĞAZA GENEL TOPLAM"). Dosya değil yazıdır; ekran kopyalar.
//!
//! GENEL TOPLAM = ciro şablonundaki son GENEL TOPLAM (`ciro_toplami`): nakit + döviz TL + kredi kartı
//! (Ziraat + İyzico + E-fatura POS) + havale + İyzico ödemesi + masraf + internet değişimi + kredi çeki.
//! Kasa fazlası/açığı, kredi kartı fazlası ve havale (+, -) bilgi satırıdır, toplama girmez.

use crate::gunsonu::{ciro_toplami, DovizSatiri};
use crate::sablon::{buyuk_tr, tarih_tr};
use std::collections::BTreeMap;

/// Bir çekmecenin (mağaza sütununun) özet verisi.
#[derive(Debug, Clone, Default)]
pub struct MagazaOzeti {
    /// Ayarlardaki çekmece adı (ör. "Merkez", "İstanbul Outlet").
    pub ad: String,
    pub ciro: BTreeMap<String, i64>,
    pub doviz: Vec<DovizSatiri>,
    /// (kalem, kuruş): reddedilmemiş, iptal edilmemiş masraflar; kalem notu dahil (`masraf_kalemi`).
    pub masraflar: Vec<(String, i64)>,
    /// Satır açıklamaları (ör. `havale_fark` nedeni); satırın sonunda parantez içinde yazılır.
    pub aciklamalar: BTreeMap<String, String>,
}

/// "1.234,56 TL"; sıfır "0 TL".
pub fn tl(k: i64) -> String {
    if k == 0 {
        "0 TL".into()
    } else {
        crate::kurus_metni(k)
    }
}

/// Kısa tutar: tam lira ise kuruşsuz ("190 TL"), değilse "190,50 TL" (masraf kalemleri).
fn kisa_tl(k: i64) -> String {
    if k % 100 == 0 {
        let s = crate::kurus_metni(k);
        s.replacen(",00 TL", " TL", 1)
    } else {
        crate::kurus_metni(k)
    }
}

fn doviz_isareti(kod: &str) -> &str {
    match kod {
        "USD" => "$",
        "EUR" => "€",
        "GBP" => "£",
        _ => kod,
    }
}

/// Yabancı para tutarı (cent): tam ise "965", değilse "965,50".
fn doviz_tutari(cent: i64) -> String {
    let s = crate::kurus_metni(cent);
    let s = s.trim_end_matches(" TL");
    s.strip_suffix(",00").unwrap_or(s).to_string()
}

/// Mağaza başlığı: çekmece adı büyük harfle, sonunda "MAĞAZA" yoksa eklenir.
pub fn baslik(ad: &str) -> String {
    let b = buyuk_tr(ad.trim());
    if b.ends_with("MAĞAZA") {
        b
    } else {
        format!("{b} MAĞAZA")
    }
}

/// Bir mağazanın genel toplamı (şablon formülü).
pub fn genel_toplam(m: &MagazaOzeti) -> i64 {
    ciro_toplami(&m.ciro, &m.doviz, m.masraflar.iter().map(|x| x.1).sum())
}

/// Bir mağaza bölümü (başlıktan GENEL TOPLAM satırına kadar).
fn kalin(s: String, k: bool) -> String {
    if k {
        format!("*{s}*")
    } else {
        s
    }
}

/// `k`: başlık ve toplam satırları WhatsApp kalın biçiminde (*…*).
pub fn bolum(m: &MagazaOzeti, tarih: &str, k: bool) -> String {
    let c = |k: &str| m.ciro.get(k).copied().unwrap_or(0);
    let mut s = kalin(format!("{} {}", baslik(&m.ad), tarih_tr(tarih)), k) + "\n";
    let mut nakit = format!("Nakit: {}", tl(c("nakit")));
    for d in m.doviz.iter().filter(|d| d.tutar != 0) {
        nakit.push_str(&format!(
            " {}{}({})",
            doviz_tutari(d.tutar),
            doviz_isareti(&d.kod),
            tl(d.tl_kurus())
        ));
    }
    s.push_str(&nakit);
    s.push('\n');
    let kart = c("ziraat_pos") + c("iyzico_pos") + c("efatura_pos");
    let masraf: i64 = m.masraflar.iter().map(|x| x.1).sum();
    for (ad, anahtar, t) in [
        ("Kasa Fazlası", "kasa_fazlasi", c("kasa_fazlasi")),
        ("Kasa Açığı", "kasa_acigi", c("kasa_acigi")),
        ("Kredi Kartı", "", kart),
        (
            "Kredi Kartı Fazlası",
            "kredi_karti_fark",
            c("kredi_karti_fark"),
        ),
        ("Havale", "havale", c("havale")),
        ("Havale(+, -)", "havale_fark", c("havale_fark")),
        ("Iyzico Ödemesi", "iyzico_odemesi", c("iyzico_odemesi")),
    ] {
        match m.aciklamalar.get(anahtar).filter(|a| !a.trim().is_empty()) {
            Some(a) => s.push_str(&format!("{ad}: {} ({})\n", tl(t), a.trim())),
            None => s.push_str(&format!("{ad}: {}\n", tl(t))),
        }
    }
    s.push_str(&format!("Masraf: {}\n", tl(masraf)));
    if !m.masraflar.is_empty() {
        let k: Vec<String> = m
            .masraflar
            .iter()
            .map(|(a, t)| format!("{} {}", a.trim(), kisa_tl(*t)))
            .collect();
        s.push_str(&format!("({})\n", k.join("\n")));
    }
    s.push_str(&format!(
        "İnternet Değişimi: {}\n",
        tl(c("internet_degisimi"))
    ));
    s.push_str(&format!("Kredi çeki: {}\n", tl(c("kredi_ceki"))));
    s.push('\n');
    s.push_str(&kalin(format!("GENEL TOPLAM: {}", tl(genel_toplam(m))), k));
    s.push('\n');
    s
}

/// Tüm mağazaların özeti ve en altta MAĞAZA GENEL TOPLAM. `k`: kalın başlıklar (Ayarlar).
pub fn ozet(magazalar: &[MagazaOzeti], tarih: &str, k: bool) -> String {
    let mut s = String::new();
    for m in magazalar {
        s.push_str(&bolum(m, tarih, k));
        s.push('\n');
    }
    let t: i64 = magazalar.iter().map(genel_toplam).sum();
    s.push_str(&kalin(format!("MAĞAZA GENEL TOPLAM: {}", tl(t)), k));
    s
}

#[cfg(test)]
mod testler {
    use super::*;

    fn ciro(v: &[(&str, i64)]) -> BTreeMap<String, i64> {
        v.iter().map(|(k, t)| (k.to_string(), *t)).collect()
    }

    /// Anonim tutarlar; biçim Yusuf'un örnek metniyle aynı.
    #[test]
    fn bicim_ve_toplamlar() {
        let a = MagazaOzeti {
            ad: "Merkez".into(),
            ciro: ciro(&[
                ("nakit", 1_000_050),
                ("kasa_fazlasi", 1_215),
                ("ziraat_pos", 2_000_000),
                ("iyzico_pos", 3_000_025),
                ("havale", 500_000),
                ("iyzico_odemesi", 100_000),
                ("internet_degisimi", 19_990),
            ]),
            doviz: vec![DovizSatiri {
                kod: "USD".into(),
                tutar: 10_000,
                kur_onbinde: 420_000,
            }],
            masraflar: vec![("Ekmek".into(), 10_000), ("Nalbur".into(), 5_050)],
            aciklamalar: BTreeMap::new(),
        };
        let b = MagazaOzeti {
            ad: "İstanbul Outlet".into(),
            ciro: ciro(&[
                ("nakit", 300_000),
                ("ziraat_pos", 200_000),
                ("havale", 100_000),
            ]),
            ..Default::default()
        };
        let s = ozet(&[a.clone(), b.clone()], "2026-11-25", false);
        let beklenen = "MERKEZ MAĞAZA 25.11.2026
Nakit: 10.000,50 TL 100$(4.200,00 TL)
Kasa Fazlası: 12,15 TL
Kasa Açığı: 0 TL
Kredi Kartı: 50.000,25 TL
Kredi Kartı Fazlası: 0 TL
Havale: 5.000,00 TL
Havale(+, -): 0 TL
Iyzico Ödemesi: 1.000,00 TL
Masraf: 150,50 TL
(Ekmek 100 TL
Nalbur 50,50 TL)
İnternet Değişimi: 199,90 TL
Kredi çeki: 0 TL

GENEL TOPLAM: 70.551,15 TL

İSTANBUL OUTLET MAĞAZA 25.11.2026
Nakit: 3.000,00 TL
Kasa Fazlası: 0 TL
Kasa Açığı: 0 TL
Kredi Kartı: 2.000,00 TL
Kredi Kartı Fazlası: 0 TL
Havale: 1.000,00 TL
Havale(+, -): 0 TL
Iyzico Ödemesi: 0 TL
Masraf: 0 TL
İnternet Değişimi: 0 TL
Kredi çeki: 0 TL

GENEL TOPLAM: 6.000,00 TL

MAĞAZA GENEL TOPLAM: 76.551,15 TL";
        assert_eq!(s, beklenen);
        // GENEL TOPLAM = şablon formülü: kasa fazlası girmez, masraf ve döviz girer.
        assert_eq!(
            genel_toplam(&a),
            1_000_050 + 420_000 + 5_000_025 + 500_000 + 100_000 + 15_050 + 19_990
        );
    }

    /// 12.10 örneğindeki yeni noktalar (anonim tutarlarla): havale farkı açıklaması, EUR, notlu masraf
    /// kalemleri, kalın biçim. Havale farkı ve kasa fazlası toplama girmez; döviz ve masraf girer.
    #[test]
    fn havale_farki_aciklamasi_euro_notlu_masraf_kalin() {
        let mut ac = BTreeMap::new();
        ac.insert(
            "havale_fark".to_string(),
            "örnek iade belgesine istinaden".to_string(),
        );
        let m = MagazaOzeti {
            ad: "Merkez".into(),
            ciro: ciro(&[
                ("nakit", 100_000),
                ("havale", 50_000),
                ("havale_fark", 20_000),
                ("kasa_fazlasi", 210),
            ]),
            doviz: vec![DovizSatiri {
                kod: "EUR".into(),
                tutar: 2_000,
                kur_onbinde: 500_000,
            }],
            masraflar: vec![
                ("Ekmek".into(), 1_900),
                ("Ekmek".into(), 200),
                ("Nalbur (depo, yetkili)".into(), 10_000),
            ],
            aciklamalar: ac,
        };
        let s = bolum(&m, "2026-10-12", true);
        assert!(s.starts_with("*MERKEZ MAĞAZA 12.10.2026*\n"));
        assert!(s.contains("Nakit: 1.000,00 TL 20€(1.000,00 TL)\n"));
        assert!(s.contains("Havale(+, -): 200,00 TL (örnek iade belgesine istinaden)\n"));
        assert!(s.contains(
            "Masraf: 121,00 TL\n(Ekmek 19 TL\nEkmek 2 TL\nNalbur (depo, yetkili) 100 TL)\n"
        ));
        // 1.000 + 1.000 (EUR) + 500 + 121 masraf; havale farkı ve kasa fazlası girmez.
        assert!(s.ends_with("*GENEL TOPLAM: 2.621,00 TL*\n"), "{s}");
        assert!(ozet(&[m], "2026-10-12", true).ends_with("*MAĞAZA GENEL TOPLAM: 2.621,00 TL*"));
    }

    #[test]
    fn baslik_magaza_eki_yinelenmez() {
        assert_eq!(baslik("Merkez Mağaza"), "MERKEZ MAĞAZA");
        assert_eq!(baslik("istanbul outlet"), "İSTANBUL OUTLET MAĞAZA");
    }
}
