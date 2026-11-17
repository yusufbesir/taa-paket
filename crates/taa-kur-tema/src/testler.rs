// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz 2026-11-17]
//! taa-kur-tema birim testleri.

use super::*;
use crate::logo::{ayristir, cokgenler, Parca};

fn paletler() -> Vec<(Tema, bool, Jetonlar)> {
    let mut v = Vec::new();
    for t in Tema::HEPSI {
        for koyu in [false, true] {
            v.push((t, koyu, t.jetonlar(koyu)));
        }
    }
    v
}

// --- Paletler ve kontrast ---

#[test]
fn metin_ve_durum_renkleri_her_yuzeyde_4_5() {
    for (t, koyu, j) in paletler() {
        for (ad, renk) in [
            ("metin", j.metin),
            ("soluk", j.soluk),
            ("basari", j.basari),
            ("uyari", j.uyari),
            ("hata", j.hata),
        ] {
            for (yad, y) in [("yuzey", j.yuzey), ("yuzey2", j.yuzey2), ("zemin", j.zemin)] {
                let k = kontrast(renk, y);
                assert!(k >= 4.5, "{} koyu={koyu}: {ad}/{yad} = {k:.2}", t.ad());
            }
        }
    }
}

#[test]
fn kenar_her_yuzeyde_3() {
    for (t, koyu, j) in paletler() {
        for y in [j.yuzey, j.yuzey2, j.zemin] {
            let k = kontrast(j.kenar, y);
            assert!(k >= 3.0, "{} koyu={koyu}: kenar {k:.2}", t.ad());
        }
    }
}

#[test]
fn mezura_metin_aaa() {
    for koyu in [false, true] {
        let j = Tema::Mezura.jetonlar(koyu);
        for y in [j.yuzey, j.yuzey2, j.zemin] {
            assert!(kontrast(j.metin, y) >= 7.0);
            assert!(kontrast(j.soluk, y) >= 7.0);
            assert!(kontrast(j.basari, y) >= 7.0);
        }
        assert!(kontrast(j.vurgu, j.vurgu_ustu) >= 7.0);
    }
}

#[test]
fn vurgu_ustu_okunur() {
    for (t, koyu, j) in paletler() {
        assert!(
            kontrast(j.vurgu, j.vurgu_ustu) >= 4.5,
            "{} koyu={koyu}",
            t.ad()
        );
    }
}

#[test]
fn karar_hexleri_birebir() {
    assert_eq!(Tema::Ilmek.jetonlar(false).vurgu, 0x6A2C5A);
    assert_eq!(Tema::Ilmek.jetonlar(true).vurgu_ustu, 0x2A1024);
    assert_eq!(Tema::Kalip.jetonlar(true).zemin, 0x101923);
    assert_eq!(Tema::Kalip.jetonlar(false).hata, 0xAD263E);
    assert_eq!(Tema::Mezura.jetonlar(false).vurgu, 0xF2C811);
    assert_eq!(Tema::Mezura.jetonlar(true).vurgu, 0xFFD600);
}

#[test]
fn kontrast_bilinen_degerler() {
    assert!((kontrast(0x000000, 0xFFFFFF) - 21.0).abs() < 1e-9);
    assert!((kontrast(0x777777, 0x777777) - 1.0).abs() < 1e-9);
    // KARAR §4.1: sarı beyaz üstünde ~1,4-1,6.
    assert!(kontrast(0xF2C811, 0xFFFFFF) < 2.0);
    assert_eq!(ustune_yazi(0xFFFFFF), 0x000000);
    assert_eq!(ustune_yazi(0x000000), 0xFFFFFF);
    assert_eq!(ustune_yazi(0x6A2C5A), 0xFFFFFF);
    assert_eq!(ustune_yazi(0xFFD600), 0x000000);
}

#[test]
fn ustune_yazi_her_renkte_4_5() {
    let mut r = 0u32;
    while r <= 0xFF_FFFF {
        assert!(kontrast(r, ustune_yazi(r)) >= 4.5, "{r:06X}");
        r += 0x010307;
    }
}

#[test]
fn colorref_ve_hex() {
    assert_eq!(colorref(0x112233), 0x332211);
    assert_eq!(colorref_rgb(0x332211), 0x112233);
    assert_eq!(hex(0x6A2C5A), "#6A2C5A");
    assert_eq!(hex_oku("#6a2c5a"), Some(0x6A2C5A));
    assert_eq!(hex_oku("6A2C5A"), Some(0x6A2C5A));
    assert_eq!(hex_oku("#6A2C5"), None);
    assert_eq!(hex_oku("#GGGGGG"), None);
}

#[test]
fn ara_renk_harman() {
    assert_eq!(ara_renk(0x000000, 0xFFFFFF, 0.0), 0x000000);
    assert_eq!(ara_renk(0x000000, 0xFFFFFF, 1.0), 0xFFFFFF);
    assert_eq!(ara_renk(0x000000, 0xFFFFFF, 0.5), 0x808080);
    assert_eq!(ara_renk(0x102030, 0x102030, 0.3), 0x102030);
    assert_eq!(ara_renk(0x000000, 0xFFFFFF, 2.0), 0xFFFFFF);
}

// --- Vurgu düzeltme ---

#[test]
fn tema_renkleri_duzeltmesiz_gecer() {
    for (t, koyu, j) in paletler() {
        assert_eq!(
            vurgu_duzelt(t, koyu, j.vurgu),
            Ok((j.vurgu, false)),
            "{} koyu={koyu}",
            t.ad()
        );
    }
}

#[test]
fn acik_renk_acik_temada_koyulastirilir() {
    let (r, duz) = vurgu_duzelt(Tema::Ilmek, false, 0xF0E0F0).expect("düzeltilmeli");
    assert!(duz);
    let j = Tema::Ilmek.jetonlar(false);
    assert!(kontrast(r, j.yuzey) >= 3.0);
    assert!(kontrast(r, ustune_yazi(r)) >= 4.5);
    // Yalnız metin yönüne kaydı: orijinalden koyu.
    assert!(bagil_parlaklik(r) < bagil_parlaklik(0xF0E0F0));
}

#[test]
fn koyu_renk_koyu_temada_acilir_ya_da_reddedilir() {
    // İlmek koyu: koyu gri metin yönüne (açığa) kaydırılarak kurtarılır.
    let (r, duz) = vurgu_duzelt(Tema::Ilmek, true, 0x101010).expect("düzeltilmeli");
    assert!(duz);
    assert!(kontrast(r, Tema::Ilmek.jetonlar(true).yuzey) >= 3.0);
    // Mezura koyu: siyah, 12 adımda 7:1 yazı + 3:1 göstergeye ulaşamaz -> ret.
    match vurgu_duzelt(Tema::Mezura, true, 0x000000) {
        Err(Red::KontrastYetersiz { denenen }) => assert_ne!(denenen, 0x000000),
        diger => panic!("reddedilmeliydi: {diger:?}"),
    }
}

#[test]
fn mezura_acikta_sari_cerceveli_kabul() {
    // Mezura açıkta gösterge kuralı yok (hep 2 px kenar çerçeve); yalnız 7:1 yazı.
    assert!(Tema::Mezura.vurgu_cerceveli(false));
    assert_eq!(
        vurgu_duzelt(Tema::Mezura, false, 0xFFFF00),
        Ok((0xFFFF00, false))
    );
    // Saf kırmızı siyah/beyazla 7:1 veremez -> koyulaştırılır.
    let (r, duz) = vurgu_duzelt(Tema::Mezura, false, 0xFF0000).unwrap();
    assert!(duz && kontrast(r, ustune_yazi(r)) >= 7.0);
}

#[test]
fn hata_rengine_yakinlik() {
    assert!(hataya_yakin(Tema::Ilmek, false, 0xC0303A));
    assert!(hataya_yakin(Tema::Kalip, true, 0xFF8090));
    assert!(!hataya_yakin(Tema::Ilmek, false, 0x164DB5));
    assert!(!hataya_yakin(Tema::Ilmek, false, 0x6A2C5A)); // mürdüm bilerek uzak
    assert!(!hataya_yakin(Tema::Mezura, false, 0x777777)); // gri
}

#[test]
fn vurgu_metin_kurali() {
    assert!(vurgu_metin_olabilir(&Tema::Ilmek.jetonlar(false)));
    assert!(!vurgu_metin_olabilir(&Tema::Mezura.jetonlar(false)));
    assert!(vurgu_metin_olabilir(&Tema::Mezura.jetonlar(true)));
}

// --- Görünüm tercihi ---

#[test]
fn json_gidis_donus() {
    let mut g = Gorunum {
        tema: Some(Tema::Kalip),
        modu: Mod::Sistem,
        hareketi_azalt: Some(true),
        tanitim_goruldu: true,
        ..Gorunum::default()
    };
    g.ozel_vurgu_ata(Tema::Ilmek, Some(0x123456));
    g.ozel_vurgu_ata(Tema::Mezura, Some(0xABCDEF));
    let metin = g.json();
    assert!(metin.contains("\"mod\": \"sistem\""));
    assert!(metin.contains("\"ilmek\": \"#123456\""));
    assert_eq!(Gorunum::json_oku(&metin), g);
    let v = Gorunum::default();
    assert_eq!(Gorunum::json_oku(&v.json()), v);
}

#[test]
fn bozuk_json_varsayilan() {
    for s in [
        "",
        "{",
        "nul",
        "[1,2]",
        "{\"tema\": \"kalip\",}",
        "{\"tema\": \"kalip\"} fazla",
        "\u{0}\u{1}",
        "{\"tema\": \"kal\nip\"}",
    ] {
        assert_eq!(Gorunum::json_oku(s), Gorunum::default(), "{s:?}");
    }
    // Derinlik sınırı: taşma yok, varsayılan.
    let derin = "[".repeat(10_000);
    assert_eq!(Gorunum::json_oku(&derin), Gorunum::default());
}

#[test]
fn bilinmeyen_anahtar_ve_deger_yok_sayilir() {
    let s = "\u{FEFF}{\"yeni_alan\": {\"a\": [1, 2, {\"b\": null}]}, \"tema\": \"MEZURA\", \
             \"mod\": \"mor\", \"ozel_vurgu\": {\"kalip\": \"#0000FF\", \"ilmek\": \"bozuk\", \
             \"baska\": \"#111111\"}, \"hareketi_azalt\": \"evet\", \"tanitim_goruldu\": true, \
             \"not\": \"\\u0130lmek \\ud83e\\uddf5\"}";
    let g = Gorunum::json_oku(s);
    assert_eq!(g.tema, Some(Tema::Mezura));
    assert_eq!(g.modu, Mod::Acik);
    assert_eq!(g.ozel_vurgu(Tema::Kalip), Some(0x0000FF));
    assert_eq!(g.ozel_vurgu(Tema::Ilmek), None);
    assert_eq!(g.hareketi_azalt, None);
    assert!(g.tanitim_goruldu);
}

#[test]
fn varsayilana_don_tanitimi_korur() {
    let mut g = Gorunum {
        tema: Some(Tema::Mezura),
        modu: Mod::Koyu,
        ozel_vurgu: [Some(1), Some(2), Some(3)],
        hareketi_azalt: Some(false),
        tanitim_goruldu: true,
        ..Gorunum::default()
    };
    g.varsayilana_don();
    assert_eq!(
        g,
        Gorunum {
            tanitim_goruldu: true,
            ..Gorunum::default()
        }
    );
}

#[test]
fn dosyaya_atomik_yazma_ve_okuma() {
    let kok = std::env::temp_dir().join(format!("taa-kur-tema-test-{}", std::process::id()));
    let yol = kok.join("kurucu").join("gorunum.json");
    assert_eq!(Gorunum::yukle_yoldan(&yol), Gorunum::default()); // yok
    let g = Gorunum {
        tema: Some(Tema::Ilmek),
        modu: Mod::Koyu,
        ..Gorunum::default()
    };
    g.kaydet_yola(&yol).unwrap();
    assert_eq!(Gorunum::yukle_yoldan(&yol), g);
    let g2 = Gorunum {
        tanitim_goruldu: true,
        ..g.clone()
    };
    g2.kaydet_yola(&yol).unwrap(); // üzerine yazma
    assert_eq!(Gorunum::yukle_yoldan(&yol), g2);
    let artik: Vec<_> = std::fs::read_dir(yol.parent().unwrap()).unwrap().collect();
    assert_eq!(artik.len(), 1, "geçici dosya kalmamalı");
    std::fs::write(&yol, b"\xFF\xFE bozuk").unwrap();
    assert_eq!(Gorunum::yukle_yoldan(&yol), Gorunum::default());
    let _ = std::fs::remove_dir_all(&kok);
}

#[test]
fn hareketi_azalt_sistemi_izler() {
    let g = Gorunum::default();
    assert!(g.hareketi_azalt_mi(false));
    assert!(!g.hareketi_azalt_mi(true));
    let g = Gorunum {
        hareketi_azalt: Some(false),
        ..Gorunum::default()
    };
    assert!(!g.hareketi_azalt_mi(false));
}

// --- Etkin jetonlar ---

#[test]
fn etkin_jetonlar_mantigi() {
    let g = Gorunum::default();
    assert_eq!(etkin_jetonlar(&g, true, false), (Tema::Ilmek, ILMEK_ACIK)); // mod Açık
    assert_eq!(etkin_jetonlar(&g, false, true), (Tema::Mezura, MEZURA_ACIK)); // YK, seçim yok

    let g = Gorunum {
        tema: Some(Tema::Kalip),
        modu: Mod::Sistem,
        ..Gorunum::default()
    };
    assert_eq!(etkin_jetonlar(&g, true, true), (Tema::Kalip, KALIP_KOYU)); // seçim YK'yı ezer
    assert_eq!(etkin_jetonlar(&g, false, false), (Tema::Kalip, KALIP_ACIK));

    // Özel vurgu: düzeltilmiş hâl + yeniden hesaplanan vurgu_ustu.
    let mut g = Gorunum {
        modu: Mod::Acik,
        ..Gorunum::default()
    };
    g.ozel_vurgu_ata(Tema::Ilmek, Some(0xF0E0F0));
    let (_, j) = etkin_jetonlar(&g, false, false);
    let (beklenen, _) = vurgu_duzelt(Tema::Ilmek, false, 0xF0E0F0).unwrap();
    assert_eq!(j.vurgu, beklenen);
    assert_eq!(j.vurgu_ustu, ustune_yazi(beklenen));
    assert_eq!(j.metin, ILMEK_ACIK.metin);
    // Başka temanın özel vurgusu bu temayı etkilemez.
    let (_, j) = etkin_jetonlar(
        &Gorunum {
            tema: Some(Tema::Kalip),
            ..g.clone()
        },
        false,
        false,
    );
    assert_eq!(j, KALIP_ACIK);
    // Reddedilen renk: temanın rengi kalır.
    let mut g = Gorunum {
        tema: Some(Tema::Mezura),
        modu: Mod::Koyu,
        ..Gorunum::default()
    };
    g.ozel_vurgu_ata(Tema::Mezura, Some(0x000000));
    assert_eq!(
        etkin_jetonlar(&g, false, false),
        (Tema::Mezura, MEZURA_KOYU)
    );
}

#[test]
fn tema_ve_mod_adlari() {
    for t in Tema::HEPSI {
        assert_eq!(Tema::addan(t.ad()), Some(t));
        assert_eq!(Tema::HEPSI[t.sira()], t);
    }
    assert_eq!(Tema::Ilmek.gorunen_ad(), "İlmek");
    assert_eq!(Tema::Kalip.gorunen_ad(), "Kalıp");
    for m in Mod::HEPSI {
        assert_eq!(Mod::addan(m.ad()), Some(m));
    }
    assert_eq!(Tema::default(), Tema::Ilmek);
    assert_eq!(Mod::default(), Mod::Acik);
}

// --- Ölçü ve hareket ---

#[test]
fn olcu_ve_hareket_karardan() {
    assert_eq!(Tema::Ilmek.geometri().kart_yaricap, 14);
    assert_eq!(Tema::Kalip.geometri().satir, 40);
    assert_eq!(Tema::Mezura.geometri().dugme, 48);
    assert_eq!(Tema::Mezura.geometri().kenar, 2);
    assert_eq!(
        Tema::Ilmek.tipografi().baslik,
        Yazi {
            px: 26,
            agirlik: 600
        }
    );
    assert_eq!(
        Tema::Mezura.tipografi().yuzde,
        Yazi {
            px: 28,
            agirlik: 700
        }
    );
    assert_eq!(Tema::Kalip.tipografi().govde_satir, 20);
    assert_eq!(Tema::HEPSI.map(|t| t.tanitim_ms()), [6000, 7200, 8400]);
    assert_eq!(Tema::Ilmek.hareket(), Hareket::RenkGecisi);
    assert_eq!(Hareket::Oturma.sure_ms(), 360);
    assert_eq!(Hareket::RenkGecisi.sure_ms(), 240);
    assert_eq!(YUKLEME_TIK as u32 * TIK_MS, 1200);
    let g = Tema::Ilmek.geometri();
    assert_eq!(g.dugme_genisligi(10), 96);
    assert_eq!(g.dugme_genisligi(100), 140);
}

// --- Logo ---

#[test]
fn logo_ayristirma() {
    let ana = ayristir(logo::ANA_PATH).unwrap();
    assert_eq!(ana.len(), 1);
    assert_eq!(
        ana[0],
        vec![
            (6.0, 6.0),
            (42.0, 6.0),
            (42.0, 15.0),
            (30.0, 15.0),
            (30.0, 33.0),
            (18.0, 33.0),
            (18.0, 15.0),
            (6.0, 15.0)
        ]
    );
    let vurgu = ayristir(logo::VURGU_PATH).unwrap();
    assert_eq!(vurgu.len(), 1);
    assert_eq!(vurgu[0].len(), 8);
    assert_eq!(vurgu[0][7], (6.0, 42.0));
    // Genel: L, örtük L, virgül, çok alt yol, ondalık.
    let g = ayristir("M0,0 L10 0 10 10Z M 1.5 2 3 4 V9 H-1").unwrap();
    assert_eq!(
        g,
        vec![
            vec![(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)],
            vec![(1.5, 2.0), (3.0, 4.0), (3.0, 9.0), (-1.0, 9.0)]
        ]
    );
    assert!(ayristir("M0 0 C1 1 2 2 3 3").is_err());
    assert!(ayristir("m0 0").is_err());
    assert!(ayristir("M0").is_err());
}

#[test]
fn logo_olcekleme() {
    let c = cokgenler(48, 0, 0);
    assert_eq!(c.len(), 2);
    assert_eq!(c[0].0, Parca::Ana);
    assert_eq!(c[1].0, Parca::Vurgu);
    assert_eq!(c[0].1.len(), 8);
    assert_eq!(c[1].1.len(), 8);
    assert_eq!(c[0].1[0], (6, 6));

    // 16 px, (100, 200) konumunda: 6 birim = 2 px.
    let c = cokgenler(16, 100, 200);
    assert_eq!(c[0].1[0], (102, 202));
    assert_eq!(c[0].1[1], (114, 202));
    assert_eq!(c[1].1[6], (114, 214)); // (42,42)
                                       // 96 px: iki kat.
    let c = cokgenler(96, 0, 0);
    assert_eq!(c[0].1[4], (60, 66)); // (30,33)
                                     // T ile yuva değmez: T'nin altı (33) yuva tabanından (36) yukarıda.
    let t_alt = c[0].1.iter().map(|p| p.1).max().unwrap();
    let yuva_ic_taban = c[1].1[2].1; // (12,36)
    assert!(t_alt < yuva_ic_taban);
}

#[test]
fn logo_svg_metni() {
    assert!(logo::SVG.contains(logo::ANA_PATH));
    assert!(logo::SVG.contains(logo::VURGU_PATH));
    assert!(logo::SVG.contains("var(--vurgu, currentColor)"));
    let s = logo::svg(0x1B1618, 0x6A2C5A);
    assert!(s.contains("fill=\"#1B1618\" d=\"M6 6H42V15H30V33H18V15H6Z\""));
    assert!(s.contains("fill=\"#6A2C5A\" d=\"M6 21H12V36H36V21H42V42H6Z\""));
    assert!(s.starts_with("<svg") && s.trim_end().ends_with("</svg>"));
}
