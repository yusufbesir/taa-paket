//! Okunur özet ve iki koşu karşılaştırması. Sahip: Yusuf. yazan: [Claude#olcum 2026-11-26]

use crate::olcum::OlcumRapor;
use crate::yontem::KOSU_FARKI_EN_COK_YUZDE;
use std::fmt::Write;

fn ope<T: std::fmt::Display>(x: Option<T>, birim: &str) -> String {
    match x {
        Some(v) => format!("{v}{birim}"),
        None => "bilinmiyor".to_string(),
    }
}

/// Türkçe okunur özet.
pub fn ozet(r: &OlcumRapor) -> String {
    let e = &r.envanter;
    let mut s = String::new();
    let _ = writeln!(s, "TAA ölçüm özeti ({}, araç {})", r.sema, r.arac_surumu);
    let _ = writeln!(s, "İşçi no: {}", e.isci_no);
    let _ = writeln!(
        s,
        "İşlemci: {} ({} mantıksal, {} fiziksel)",
        e.islemci.model,
        e.islemci.mantiksal,
        ope(e.islemci.fiziksel, "")
    );
    let _ = writeln!(
        s,
        "Bellek: toplam {}, boş {}",
        ope(e.bellek.toplam_mb, " MB"),
        ope(e.bellek.bos_mb, " MB")
    );
    let _ = writeln!(s, "Disk: {}, boş {}", e.disk.tur, ope(e.disk.bos_gb, " GB"));
    let _ = writeln!(
        s,
        "Sistem: {} (çekirdek/sürüm {}); güç: {}",
        e.sistem.surum, e.sistem.cekirdek, e.guc
    );
    let _ = writeln!(
        s,
        "İş türü: {} (OCR yerine sentetik); tekrar süresi {} ms",
        r.is_turu, r.sure_ms
    );
    let _ = writeln!(
        s,
        "Yöntem özeti: {}... kilit {}",
        &r.yontem_sha256[..16],
        if r.yontem_kilit_tutuyor {
            "tutuyor"
        } else {
            "TUTMUYOR"
        }
    );
    let _ = writeln!(
        s,
        "Pay  Hız ({})  Geçerli  Taşıma ms  Ön plan artış  En kötü ek ms  Sonuç",
        r.paylar
            .first()
            .map(|p| p.hiz_birimi.as_str())
            .unwrap_or("-")
    );
    for p in &r.paylar {
        let _ = writeln!(
            s,
            "%{:<3} {:<22} {}/{}      {:<9}  {:<13}  {:<13}  {}",
            p.pay_yuzde,
            ope(p.hiz_birim_saat, ""),
            p.gecerli_tekrar,
            p.tekrarlar.len(),
            p.tasima_ms,
            ope(p.on_plan.artis_yuzde, " %"),
            ope(p.on_plan.en_kotu_ek_ms, ""),
            match p.on_plan.gecti {
                Some(true) => "geçti",
                Some(false) => "GEÇMEDİ",
                None => "veri yok",
            }
        );
    }
    let _ = writeln!(
        s,
        "Ön planı etkilemeyen en yüksek pay: {}",
        ope(
            r.on_plani_etkilemeyen_en_yuksek_pay
                .map(|p| format!("%{p}")),
            ""
        )
    );
    let l = &r.limitler;
    let _ = writeln!(
        s,
        "Limit düzeneği: iş parçacığı {}, görev oranı {}, düşük öncelik {}, bellek sınırı {}, iş nesnesi {} (uygulanmayan, gerçek işçide zorunlu)",
        ev(l.is_parcacigi),
        ev(l.gorev_orani),
        ev(l.dusuk_oncelik),
        ev(l.bellek_ust_sinir),
        ev(l.is_nesnesi)
    );
    let _ = writeln!(s, "Toplanmayan: {}", e.toplanmadi.join(", "));
    for u in &r.uyarilar {
        let _ = writeln!(s, "UYARI: {u}");
    }
    let _ = writeln!(s, "Not: taşıma süresi bellek içi iskelettir; ön plan işlemi temsilîdir (gerçek kasa işlemi DG1b).");
    s
}

fn ev(b: bool) -> &'static str {
    if b {
        "var"
    } else {
        "yok"
    }
}

/// İki koşunun pay başına ortanca hız farkı (yüzde). Her satır: (pay, fark, eşik içinde mi).
pub fn karsilastir(a: &OlcumRapor, b: &OlcumRapor) -> Vec<(u32, Option<f64>, Option<bool>)> {
    a.paylar
        .iter()
        .map(|pa| {
            let hb = b
                .paylar
                .iter()
                .find(|pb| pb.pay_yuzde == pa.pay_yuzde)
                .and_then(|pb| pb.hiz_birim_saat);
            match (pa.hiz_birim_saat, hb) {
                (Some(x), Some(y)) if x > 0.0 || y > 0.0 => {
                    let fark = (y - x).abs() / x.max(y) * 100.0;
                    let f = (fark * 100.0).round() / 100.0;
                    (pa.pay_yuzde, Some(f), Some(f <= KOSU_FARKI_EN_COK_YUZDE))
                }
                _ => (pa.pay_yuzde, None, None),
            }
        })
        .collect()
}

pub fn karsilastirma_metni(a: &OlcumRapor, b: &OlcumRapor) -> String {
    let mut s = String::from("İki koşu hız farkı (ortanca, büyüğe göre yüzde)\n");
    for (pay, fark, tamam) in karsilastir(a, b) {
        let _ = writeln!(
            s,
            "%{pay}: {} {}",
            ope(fark, " %"),
            match tamam {
                Some(true) => "(sınır içinde)",
                Some(false) => "(SINIR AŞILDI)",
                None => "(veri yok)",
            }
        );
    }
    s
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::olcum::{olc, Ayar};

    fn hizli() -> OlcumRapor {
        olc(
            &Ayar {
                sure_ms: 100,
                isitma: 1,
                tekrar: 3,
                paylar: vec![5, 15],
                cekirdek: Some(4),
            },
            &mut |_| {},
        )
    }

    #[test]
    fn ozet_turkce_ve_alanlar_var() {
        let r = hizli();
        let s = ozet(&r);
        assert!(s.contains("TAA ölçüm özeti"));
        assert!(s.contains("İşçi no: isci-"));
        assert!(s.contains("%5"));
        assert!(s.contains("%15"));
        assert!(s.contains("iş nesnesi yok"));
        assert!(s.contains("Toplanmayan: gpu"));
    }

    #[test]
    fn karsilastirma_esik_ve_yok_veri() {
        let mut a = hizli();
        let mut b = a.clone();
        a.paylar[0].hiz_birim_saat = Some(1000.0);
        b.paylar[0].hiz_birim_saat = Some(950.0);
        a.paylar[1].hiz_birim_saat = Some(1000.0);
        b.paylar[1].hiz_birim_saat = Some(500.0);
        let k = karsilastir(&a, &b);
        assert_eq!(k[0].2, Some(true));
        assert_eq!(k[1].2, Some(false));
        assert_eq!(k[1].1, Some(50.0));
        b.paylar[0].hiz_birim_saat = None;
        assert_eq!(karsilastir(&a, &b)[0], (5, None, None));
        assert!(karsilastirma_metni(&a, &b).contains("SINIR AŞILDI"));
    }
}
