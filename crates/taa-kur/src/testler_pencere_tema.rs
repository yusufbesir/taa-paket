// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz 2026-11-17]
//! `pencere_tema` birim testleri: tema -> yerleşim, durum işaretleri, seçili satır, geçişler, Ayarlar, düzen.

use super::*;
use taa_kur_tema::{kontrast, Hareket};

/// Sahte ölçer: karakter başına boyun ~%55'i (Segoe UI ortalamasına yakın), satır = boy x 1,4.
struct Sahte {
    dpi: u32,
}

impl Olcer for Sahte {
    fn genislik(&self, metin: &str, rol: YaziRolu) -> i32 {
        let px = olcekle(yazi(Tema::Mezura, rol).px.max(16), self.dpi);
        (metin.chars().count() as i32 * px * 55 + 99) / 100
    }
    fn satir(&self, rol: YaziRolu) -> i32 {
        olcekle(yazi(Tema::Mezura, rol).px.max(16), self.dpi) * 14 / 10
    }
}

fn satir(durum: &str) -> Satir {
    Satir {
        ad: "taa-ornek".into(),
        kurulu: "0.1.0".into(),
        secili: "0.1.0".into(),
        durum: durum.into(),
        secili_var: true,
    }
}

fn girdi<'a>(tema: Tema, ekran: Ekran, en: i32, boy: i32, dpi: u32) -> DuzenGirdisi<'a> {
    DuzenGirdisi {
        tema,
        dpi,
        en,
        boy,
        ekran,
        durum: "Kuruldu: taa-ornek 0.2.0 (seçili değil). Önceki içerik şuraya taşındı: D:\\Programlar\\TAA\\eski\\taa-ornek-0.1.0.",
        kok: "D:\\Programlar\\TAA",
        vurgu_notu: Some(("Okunabilirlik için renk biraz koyulaştırıldı.", Jeton::Soluk)),
        yk_notu: true,
        kare: SON_KARE,
    }
}

// --- Yerleşim kararları ---

#[test]
fn tema_basina_yerlesim_karari() {
    let i = yerlesim(Tema::Ilmek);
    assert_eq!(i.eylemler, EylemYeri::UstAracCubugu);
    assert!(i.ayarlar_baslikta && !i.sekmeler && !i.kok_tablodan_once);
    assert_eq!(i.satir_ayirici, SatirAyirici::Teyel);
    assert_eq!(i.tablo_kenari, TabloKenari::YuzeyKarti);

    let k = yerlesim(Tema::Kalip);
    assert_eq!(k.eylemler, EylemYeri::AltIslemCubugu);
    assert!(k.sekmeler && !k.ayarlar_baslikta && k.kok_tablodan_once && k.rapor_sagda);
    assert_eq!(k.tablo_kenari, TabloKenari::InceCerceve);

    let m = yerlesim(Tema::Mezura);
    assert_eq!(m.eylemler, EylemYeri::KurUstSagDigerleriAlt);
    assert!(m.buyuk_baslik && m.ayarlar_baslikta && !m.kok_tablodan_once);
    assert_eq!(m.tablo_kenari, TabloKenari::KalinUstAlt);
}

#[test]
fn baslik_metinleri() {
    assert_eq!(
        baslik_metni(Tema::Ilmek, Ekran::Paketler),
        "TAA Kurucu · Paketler"
    );
    assert_eq!(
        baslik_metni(Tema::Ilmek, Ekran::Ayarlar),
        "TAA Kurucu · Ayarlar"
    );
    assert!(baslik_metni(Tema::Kalip, Ekran::Paketler).starts_with("TAA Kurucu  v"));
    assert_eq!(buyuk_baslik_metni(Ekran::Paketler), "Paketler");
    assert_eq!(PENCERE_BASLIGI, "TAA Kurucu · Paketler");
}

// --- Durum sütunu ---

#[test]
fn durum_isaret_ve_metin() {
    assert_eq!(durum_gorunen(&satir("Hazır")), "✓ Hazır");
    assert_eq!(durum_gorunen(&satir("Hazır (sabit)")), "✓ Hazır (sabit)");
    assert_eq!(
        durum_gorunen(&satir("Seçili sürüm yok")),
        "! Seçili sürüm yok"
    );
    assert_eq!(
        durum_gorunen(&satir("Daha yeni sürüm kurulu (0.2.0), seçili değil")),
        "! Daha yeni sürüm kurulu (0.2.0), seçili değil"
    );
    assert_eq!(durum_gorunen(&satir("Kurulu değil")), "○ Kurulu değil");
    assert_eq!(durum_turu(&satir("Hazır")).jeton(), Jeton::Basari);
    assert_eq!(durum_turu(&satir("Seçili sürüm yok")).jeton(), Jeton::Uyari);
    assert_eq!(durum_turu(&satir("Kurulu değil")).jeton(), Jeton::Soluk);
}

#[test]
fn durum_renkleri_secili_satirda_okunur() {
    // İşaret rengi seçili satır dolgusunda (yuzey2) ve yüzeyde >= 4,5:1.
    for t in Tema::HEPSI {
        for koyu in [false, true] {
            let j = t.jetonlar(koyu);
            for d in [DurumTuru::Tamam, DurumTuru::Uyari, DurumTuru::Bos] {
                let r = d.jeton().renk(&j);
                for zemin in [secili_bicim(t).dolgu.renk(&j), j.yuzey] {
                    assert!(kontrast(r, zemin) >= 4.5, "{} {koyu} {d:?}", t.ad());
                }
            }
        }
    }
}

#[test]
fn hucre_kipleri_kesmez() {
    assert_eq!(hucre_kipi(0), MetinKipi::TekKisalt);
    for s in 1..4 {
        assert_eq!(hucre_kipi(s), MetinKipi::Sarili);
    }
}

// --- Seçili satır ---

#[test]
fn secili_satir_bicimi() {
    let i = secili_bicim(Tema::Ilmek);
    assert_eq!(i.dolgu, Jeton::Yuzey2);
    assert!(i.kalin && i.seritler.is_empty() && i.isaret.is_none());

    let k = secili_bicim(Tema::Kalip);
    assert_eq!(k.seritler, vec![(3, Jeton::Vurgu)]);
    assert_eq!(serit_genisligi(Tema::Kalip), 3);

    let m = secili_bicim(Tema::Mezura);
    assert_eq!(m.dolgu, Jeton::Yuzey2);
    assert_eq!(serit_genisligi(Tema::Mezura), 6);
    assert_eq!(m.seritler[0].1, Jeton::Vurgu);
    assert_eq!(m.seritler[1].1, Jeton::VurguUstu);
    assert_eq!(m.isaret, Some("▸"));
}

#[test]
fn mezura_acikta_odak_sari_degil() {
    assert_eq!(odak_jetonu(Tema::Mezura, false), Jeton::Kenar);
    assert_eq!(odak_jetonu(Tema::Mezura, true), Jeton::Vurgu);
    assert_eq!(odak_jetonu(Tema::Ilmek, false), Jeton::Vurgu);
}

// --- Görünüş ---

#[test]
fn yuksek_karsitlik_ve_hareket() {
    let mut g = Gorunum::default();
    let s = SistemDurumu {
        yuksek_karsitlik: true,
        animasyon_acik: false,
        ..SistemDurumu::default()
    };
    let gs = Gorunus::hesapla(&g, s);
    assert_eq!(gs.tema, Tema::Mezura);
    assert!(gs.yk_ile_mezura);
    assert!(gs.hareketi_azalt, "animasyon kapalıysa hareket azaltılır");

    g.tema = Some(Tema::Kalip);
    g.hareketi_azalt = Some(false);
    let gs = Gorunus::hesapla(&g, s);
    assert_eq!(gs.tema, Tema::Kalip, "kullanıcı seçimine dokunulmaz");
    assert!(!gs.yk_ile_mezura && !gs.hareketi_azalt);

    let gs = Gorunus::hesapla(&Gorunum::default(), SistemDurumu::default());
    assert_eq!(gs.tema, Tema::Ilmek);
    assert!(!gs.koyu && !gs.hareketi_azalt);
    assert_eq!(gs.j, Tema::Ilmek.jetonlar(false));
}

#[test]
fn sistem_modu_koyuyu_izler() {
    let g = Gorunum {
        modu: Mod::Sistem,
        ..Gorunum::default()
    };
    let koyu = SistemDurumu {
        koyu: true,
        ..SistemDurumu::default()
    };
    assert!(Gorunus::hesapla(&g, koyu).koyu);
    assert!(!Gorunus::hesapla(&g, SistemDurumu::default()).koyu);
}

// --- Geçişler ---

#[test]
fn gecis_kareleri() {
    assert_eq!(gecis_adim_sayisi(Hareket::RenkGecisi, false), 2);
    assert_eq!(gecis_adim_sayisi(Hareket::Oturma, false), 3);
    for h in [Hareket::RenkGecisi, Hareket::Oturma, Hareket::Anlik] {
        assert_eq!(
            gecis_adim_sayisi(h, true),
            0,
            "hareketi azalt: animasyon yok"
        );
        assert_eq!(gecis_karesi(h, 0), SON_KARE);
        assert_eq!(gecis_karesi(h, h.adim() + 1), SON_KARE);
    }
    assert_eq!(gecis_karesi(Hareket::RenkGecisi, 1).renk_orani, 0.5);
    assert_eq!(gecis_karesi(Hareket::RenkGecisi, 2).renk_orani, 1.0);
    let kayma: Vec<i32> = (1..=3)
        .map(|n| gecis_karesi(Hareket::Oturma, n).kayma_px)
        .collect();
    assert_eq!(kayma, vec![8, 3, 0]);
    assert_eq!(gecis_karesi(Hareket::Anlik, 1).cizgi_orani, 0.5);
    assert_eq!(gecis_karesi(Hareket::Anlik, 1).renk_orani, 1.0);
}

#[test]
fn renk_gecisi_yalniz_dolgu() {
    let a = Tema::Ilmek.jetonlar(false);
    let b = Tema::Ilmek.jetonlar(true);
    let h = dolgu_harmanla(&a, &b, 0.5);
    assert_eq!(h.metin, b.metin);
    assert_eq!(h.soluk, b.soluk);
    assert_eq!(h.basari, b.basari);
    assert_ne!(h.zemin, a.zemin);
    assert_ne!(h.zemin, b.zemin);
    assert_eq!(dolgu_harmanla(&a, &b, 1.0), b);
}

// --- Ayarlar ---

#[test]
fn vurgu_secimi_ve_notlar() {
    // Uygun renk: not yok, olduğu gibi saklanır.
    let mut g = Gorunum::default();
    assert_eq!(vurgu_sec(&mut g, Tema::Ilmek, false, 0x164DB5), None);
    assert_eq!(g.ozel_vurgu(Tema::Ilmek), Some(0x164DB5));
    assert_eq!(g.ozel_vurgu(Tema::Kalip), None, "tema başına ayrı");

    // Açık sarı İlmek açıkta okunmaz: koyulaştırılır, not gösterilir.
    let mut g = Gorunum::default();
    let not = vurgu_sec(&mut g, Tema::Ilmek, false, 0xFFE680).expect("not");
    assert!(not.0.contains("koyulaştırıldı"), "{}", not.0);
    let saklanan = g.ozel_vurgu(Tema::Ilmek).unwrap();
    assert_ne!(saklanan, 0xFFE680);
    assert!(kontrast(saklanan, Tema::Ilmek.jetonlar(false).yuzey) >= 3.0);

    // Hataya yakın kırmızı: uyarı notu.
    let mut g = Gorunum::default();
    let not = vurgu_sec(&mut g, Tema::Kalip, false, 0xB0203A).expect("not");
    assert!(not.0.contains("hata rengine"), "{}", not.0);
    assert_eq!(not.1, Jeton::Uyari);
}

#[test]
fn reddedilen_renk_saklanmaz() {
    for t in Tema::HEPSI {
        for koyu in [false, true] {
            for renk in [0x000000, 0x777777, 0xFFFFFF, 0xFFFF00, 0x00FF00, 0x808000] {
                let mut g = Gorunum::default();
                let not = vurgu_sec(&mut g, t, koyu, renk);
                match vurgu_duzelt(t, koyu, renk) {
                    Ok((r, _)) => assert_eq!(g.ozel_vurgu(t), Some(r)),
                    Err(_) => {
                        assert_eq!(g.ozel_vurgu(t), None);
                        assert_eq!(not.map(|n| n.1), Some(Jeton::Uyari));
                    }
                }
            }
        }
    }
}

#[test]
fn oge_kimlikleri_benzersiz() {
    let mut gorulen = std::collections::HashSet::new();
    for o in Oge::HEPSI {
        assert!(gorulen.insert(o.kimlik()));
        assert_eq!(Oge::kimlikten(o.kimlik()), Some(o));
        assert!(!o.etiket(Ekran::Paketler).is_empty());
    }
    assert_eq!(Oge::kimlikten(0), None);
    assert_eq!(Oge::kimlikten(9999), None);
    assert_eq!(Oge::Kur.tur(), DugmeTuru::Birincil);
    assert_eq!(Oge::Kur.etiket(Ekran::Paketler), "Kur…");
    assert_eq!(Oge::Ayarlar.etiket(Ekran::Ayarlar), "Paketlere dön");
}

// --- Düzen ---

fn denetle(t: Tema, ekran: Ekran, en: i32, boy: i32, dpi: u32) -> Duzen {
    let o = Sahte { dpi };
    let d = duzen(&girdi(t, ekran, en, boy, dpi), &o);
    let pencere = Dik::yeni(0, 0, en, boy);
    let geo = t.geometri();
    let ad = format!("{} {ekran:?} {en}x{boy}@{dpi}", t.ad());
    for b in &d.dugmeler {
        assert!(
            b.alan.icinde(&pencere),
            "{ad}: {:?} pencere dışında {:?}",
            b.oge,
            b.alan
        );
        if b.satir == 1 {
            let metin = o.genislik(&b.oge.etiket(ekran), YaziRolu::Dugme)
                + olcekle(gosterge_genisligi(b.oge.tur()), dpi);
            assert!(
                b.alan.en >= metin + 2 * olcekle(geo.dugme_yatay_ic, dpi),
                "{ad}: {:?} etiketi kesilir",
                b.oge
            );
            assert!(b.alan.en >= olcekle(geo.dugme_en_az, dpi));
        }
        for c in &d.dugmeler {
            if b.oge != c.oge {
                assert!(
                    !b.alan.kesisir(&c.alan),
                    "{ad}: {:?} ile {:?} üst üste",
                    b.oge,
                    c.oge
                );
            }
        }
        if let Some(l) = d.liste {
            assert!(!b.alan.kesisir(&l), "{ad}: {:?} listenin üstünde", b.oge);
        }
    }
    for m in &d.metinler {
        assert!(
            m.alan.icinde(&pencere),
            "{ad}: metin dışarıda {:?}",
            m.metin
        );
        for b in &d.dugmeler {
            assert!(
                !m.alan.kesisir(&b.alan),
                "{ad}: {:?} metni {:?} düğmesiyle çakışıyor",
                m.metin,
                b.oge
            );
        }
    }
    d
}

#[test]
fn tum_temalarda_duzen_tutarli() {
    for t in Tema::HEPSI {
        for ekran in [Ekran::Paketler, Ekran::Ayarlar] {
            for (en, boy) in [EN_AZ_BOYUT, HEDEF_BOYUT] {
                for dpi in [96, 120, 144] {
                    // %125 ve %150'de pencere de ölçeklenir.
                    denetle(t, ekran, olcekle(en, dpi), olcekle(boy, dpi), dpi);
                }
            }
        }
    }
}

#[test]
fn ilmek_eylemler_ustte_kok_altta() {
    let d = denetle(Tema::Ilmek, Ekran::Paketler, 1040, 700, 96);
    let liste = d.liste.unwrap();
    for o in [Oge::Kur, Oge::Kaldir, Oge::Guncelle, Oge::Rapor] {
        assert!(d.dugme(o).unwrap().alan.alt() <= liste.y, "{o:?}");
    }
    assert!(d.dugme(Oge::Kopyala).unwrap().alan.y >= liste.alt());
    assert!(d.dugme(Oge::Ayarlar).is_some());
    assert!(d.dugme(Oge::SekmePaketler).is_none());
    let kur = d.dugme(Oge::Kur).unwrap().alan;
    let kaldir = d.dugme(Oge::Kaldir).unwrap().alan;
    assert_eq!(kur.y, kaldir.y);
    assert!(kur.x < kaldir.x);
}

#[test]
fn kalip_sekmeler_kok_once_eylemler_altta() {
    let d = denetle(Tema::Kalip, Ekran::Paketler, 1040, 700, 96);
    let liste = d.liste.unwrap();
    assert!(d.dugme(Oge::SekmePaketler).is_some() && d.dugme(Oge::SekmeAyarlar).is_some());
    assert!(d.dugme(Oge::Ayarlar).is_none(), "Kalıp'ta Ayarlar sekmedir");
    assert!(d.dugme(Oge::Kopyala).unwrap().alan.alt() <= liste.y);
    for o in [Oge::Kur, Oge::Kaldir, Oge::Guncelle, Oge::Rapor] {
        assert!(d.dugme(o).unwrap().alan.y >= liste.alt(), "{o:?}");
    }
    // Rapor sağa yaslı.
    let rapor = d.dugme(Oge::Rapor).unwrap().alan;
    assert!(rapor.x > d.dugme(Oge::Guncelle).unwrap().alan.sag());
    assert!(1040 - rapor.sag() < 40);
}

#[test]
fn mezura_kur_ust_sagda_digerleri_altta() {
    let d = denetle(Tema::Mezura, Ekran::Paketler, 1040, 700, 96);
    let liste = d.liste.unwrap();
    let kur = d.dugme(Oge::Kur).unwrap().alan;
    assert!(kur.alt() <= liste.y);
    assert_eq!(kur.sag(), 1040 - 16);
    for o in [Oge::Kaldir, Oge::Guncelle, Oge::Rapor] {
        assert!(d.dugme(o).unwrap().alan.y >= liste.alt(), "{o:?}");
    }
    assert!(d
        .metinler
        .iter()
        .any(|m| m.metin == "Paketler" && m.rol == YaziRolu::Baslik));
    let kopyala = d.dugme(Oge::Kopyala).unwrap().alan;
    assert!(
        kopyala.y > d.dugme(Oge::Kaldir).unwrap().alan.y,
        "kök klasör eylemlerin altında"
    );
}

#[test]
fn ayarlar_ekrani_ogeleri() {
    for t in Tema::HEPSI {
        let d = denetle(t, Ekran::Ayarlar, 860, 600, 96);
        assert!(d.liste.is_none());
        assert!(d.renk_ornegi.is_some());
        for o in Oge::HEPSI {
            let olmali = match o {
                Oge::Kur | Oge::Kaldir | Oge::Guncelle | Oge::Rapor | Oge::Kopyala => false,
                Oge::Ayarlar => t != Tema::Kalip,
                Oge::SekmePaketler | Oge::SekmeAyarlar => t == Tema::Kalip,
                _ => true,
            };
            assert_eq!(d.dugme(o).is_some(), olmali, "{} {o:?}", t.ad());
        }
        // Tema düğmeleri önizleme taşır, en az sıkışık önizleme kadar geniş.
        let tema = d.dugme(Oge::Tema(Tema::Ilmek)).unwrap().alan;
        assert!(tema.en >= ONIZLEME_SIKISIK.0 && tema.boy >= ONIZLEME_SIKISIK.1);
        // Notlar gösterilir.
        assert!(d.metinler.iter().any(|m| m.metin == YK_NOTU));
        assert!(d
            .metinler
            .iter()
            .any(|m| m.metin.contains("koyulaştırıldı")));
        // Hiçbir şey durum satırının üstüne taşmaz.
        let durum = d
            .metinler
            .iter()
            .find(|m| m.metin.starts_with("Kuruldu"))
            .unwrap();
        for b in &d.dugmeler {
            assert!(
                b.alan.alt() <= durum.alan.y,
                "{} {:?} durum satırına taşıyor",
                t.ad(),
                b.oge
            );
        }
    }
    // Geniş pencerede tam önizleme (160x100).
    let d = denetle(Tema::Ilmek, Ekran::Ayarlar, 1040, 700, 96);
    assert!(d.dugme(Oge::Tema(Tema::Ilmek)).unwrap().alan.en >= ONIZLEME.0);
}

#[test]
fn dar_pencerede_uzun_etiket_iki_satir() {
    let o = Sahte { dpi: 96 };
    let d = duzen(&girdi(Tema::Mezura, Ekran::Ayarlar, 300, 900, 96), &o);
    let b = d.dugme(Oge::TanitimTekrar).unwrap();
    assert_eq!(b.satir, 2, "sığmayan etiket iki satıra iner");
    assert!(b.alan.boy > olcekle(Tema::Mezura.geometri().dugme, 96));
}

#[test]
fn kalip_oturma_yalniz_govdeyi_kaydirir() {
    let o = Sahte { dpi: 96 };
    let mut g = girdi(Tema::Kalip, Ekran::Paketler, 1040, 700, 96);
    let d0 = duzen(&g, &o);
    g.kare = gecis_karesi(Hareket::Oturma, 1);
    let d1 = duzen(&g, &o);
    assert_eq!(d1.liste.unwrap().x, d0.liste.unwrap().x + 8);
    assert_eq!(
        d1.dugme(Oge::Kur).unwrap().alan.x,
        d0.dugme(Oge::Kur).unwrap().alan.x + 8
    );
    // Başlık ve sekmeler yerinde.
    assert_eq!(d1.logo, d0.logo);
    assert_eq!(d1.dugme(Oge::SekmeAyarlar), d0.dugme(Oge::SekmeAyarlar));
}

#[test]
fn mezura_alt_cizgisi_uzar() {
    let o = Sahte { dpi: 96 };
    let mut g = girdi(Tema::Mezura, Ekran::Paketler, 1040, 700, 96);
    let tam = duzen(&g, &o);
    g.kare = gecis_karesi(Hareket::Anlik, 1);
    let yari = duzen(&g, &o);
    let cizgi = |d: &Duzen| {
        d.kutular
            .iter()
            .find(|k| k.alan.boy == 4 && k.dolgu == Some(Jeton::Metin))
            .unwrap()
            .alan
            .en
    };
    assert_eq!(cizgi(&tam), o.genislik("Paketler", YaziRolu::Baslik));
    assert!((cizgi(&yari) * 2 - cizgi(&tam)).abs() <= 1);
}

#[test]
fn sutunlar_basliklari_kesmez_ve_doldurur() {
    let o = Sahte { dpi: 96 };
    for t in Tema::HEPSI {
        let g = sutun_genislikleri(t, 96, 1000, &o);
        assert_eq!(g.iter().sum::<i32>(), 1000);
        for (i, b) in SUTUNLAR.iter().enumerate() {
            assert!(g[i] > o.genislik(b, YaziRolu::GovdeKalin));
        }
        // Dar listede Durum sütunu en az 200 px kalır (yatay kaydırma çıkar, kesilmez).
        let dar = sutun_genislikleri(t, 96, 500, &o);
        assert!(dar[3] >= 200);
    }
}
