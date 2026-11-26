//! ÖRNEK veri (yalnız `--demo` bayrağıyla). Her ad ve adres uydurmadır ("ÖRNEK" etiketli, `ornek.test` alanı);
//! gerçek kişi, şube ya da adres içermez (docs/rehber-sozlesmesi.md §6: örnek veri uydurma kişilerle üretilir).
//! Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]

use crate::belge::BelgeDeposu;
use std::collections::BTreeSet;
use taa_rehber_model::model::*;

/// Örnek şube kodları (gerçek `subeler.kod` değerleri değildir).
pub const SUBE_1: &str = "ORNEK-S1";
pub const SUBE_2: &str = "ORNEK-S2";
/// "Mağaza müdürü/sorumlusu" sayılan örnek rol kodu (AÇIK-R7: gerçek liste docs/rehber-rolleri.toml'a bağlanacak).
pub const ROL_MUDUR: &str = "magaza-muduru";

fn kisi(id: &str, ad: &str, eposta: &str) -> KisiKaydi {
    KisiKaydi {
        id: id.into(),
        ad: ad.into(),
        eposta: eposta.into(),
        durum: KisiDurum::Etkin,
        pasif_tarihi: None,
        surum: 1,
        guncelleme: String::new(),
    }
}

fn birim(
    id: &str,
    ad: &str,
    tur: BirimTur,
    ust: Option<&str>,
    sube: Option<&str>,
    ortak: &str,
) -> BirimKaydi {
    BirimKaydi {
        id: id.into(),
        ad: ad.into(),
        tur,
        ust_birim_id: ust.map(Into::into),
        sube_kodu: sube.map(Into::into),
        ortak_eposta: Some(ortak.into()),
        durum: BirimDurum::Etkin,
        surum: 1,
        guncelleme: String::new(),
    }
}

#[allow(clippy::too_many_arguments)]
fn gorev(
    id: &str,
    kisi: &str,
    birim: &str,
    rol: &str,
    unvan: &str,
    amir: Option<&str>,
    bitis: Option<&str>,
) -> GorevlendirmeKaydi {
    GorevlendirmeKaydi {
        id: id.into(),
        kisi_id: kisi.into(),
        birim_id: birim.into(),
        rol: rol.into(),
        gorev: Some(unvan.into()),
        amir_id: amir.map(Into::into),
        tur: GorevTur::Asil,
        vekil_edilen_id: None,
        durum: EtkinPasif::Etkin,
        baslangic: "2026-01-01".into(),
        bitis: bitis.map(Into::into),
        surum: 1,
        guncelleme: String::new(),
    }
}

/// Örnek rehber deposu ve "müdür/sorumlu" rol kümesi.
pub fn ornek_depo(zaman: &str) -> (Depo, BTreeSet<String>) {
    let mut d = Depo::yeni();
    let roller = [
        (ROL_MUDUR, "Mağaza müdürü"),
        ("kasa-sorumlusu", "Kasa sorumlusu"),
        ("uzman", "Uzman"),
        ("merkez-yetkilisi", "Merkez yetkilisi"),
    ];
    for (kod, ad) in roller {
        d.rol_ekle(RolKaydi {
            kod: kod.into(),
            ad: ad.into(),
            aciklama: Some("ÖRNEK rol".into()),
            durum: EtkinPasif::Etkin,
        })
        .expect("örnek rol");
    }
    let birimler = [
        birim(
            "ornek-merkez",
            "ÖRNEK Genel Merkez",
            BirimTur::Merkez,
            None,
            None,
            "merkez@ornek.test",
        ),
        birim(
            "ornek-idari",
            "ÖRNEK İdari Birim",
            BirimTur::Departman,
            Some("ornek-merkez"),
            None,
            "idari@ornek.test",
        ),
        birim(
            "ornek-sube-1",
            "ÖRNEK Şube 1",
            BirimTur::Sube,
            Some("ornek-merkez"),
            Some(SUBE_1),
            "sube1@ornek.test",
        ),
        birim(
            "ornek-sube-2",
            "ÖRNEK Şube 2",
            BirimTur::Sube,
            Some("ornek-merkez"),
            Some(SUBE_2),
            "sube2@ornek.test",
        ),
        birim(
            "ornek-magaza-1",
            "ÖRNEK Mağaza 1",
            BirimTur::Magaza,
            Some("ornek-sube-1"),
            Some(SUBE_1),
            "magaza1@ornek.test",
        ),
        birim(
            "ornek-magaza-2",
            "ÖRNEK Mağaza 2",
            BirimTur::Magaza,
            Some("ornek-sube-2"),
            Some(SUBE_2),
            "magaza2@ornek.test",
        ),
    ];
    for b in birimler {
        d.birim_ekle(b, zaman).expect("örnek birim");
    }
    let kisiler = [
        kisi(
            "ornek-kisi-merkez",
            "ÖRNEK Merkez Yetkilisi",
            "merkez.yetkili@ornek.test",
        ),
        kisi(
            "ornek-kisi-idari",
            "ÖRNEK İdari Personel",
            "idari.personel@ornek.test",
        ),
        kisi(
            "ornek-kisi-ayse",
            "ÖRNEK Ayşe Şube1",
            "ayse.sube1@ornek.test",
        ),
        kisi(
            "ornek-kisi-mudur1",
            "ÖRNEK Müdür Şube1",
            "mudur.sube1@ornek.test",
        ),
        kisi(
            "ornek-kisi-ali",
            "ÖRNEK Ali Mağaza1",
            "ali.magaza1@ornek.test",
        ),
        kisi("ornek-kisi-can", "ÖRNEK Can Şube2", "can.sube2@ornek.test"),
        kisi(
            "ornek-kisi-mudur2",
            "ÖRNEK Müdür Şube2",
            "mudur.sube2@ornek.test",
        ),
    ];
    for k in kisiler {
        d.kisi_ekle(k, zaman).expect("örnek kişi");
    }
    let mut pasif = kisi(
        "ornek-kisi-pasif",
        "ÖRNEK Pasif Kişi",
        "pasif.kisi@ornek.test",
    );
    pasif.durum = KisiDurum::Pasif;
    pasif.pasif_tarihi = Some("2026-06-30".into());
    d.kisi_ekle(pasif, zaman).expect("örnek pasif kişi");
    let gorevler = [
        gorev(
            "ornek-g-1",
            "ornek-kisi-merkez",
            "ornek-merkez",
            "merkez-yetkilisi",
            "Merkez yetkilisi",
            None,
            None,
        ),
        gorev(
            "ornek-g-2",
            "ornek-kisi-idari",
            "ornek-idari",
            "uzman",
            "İdari uzman",
            None,
            None,
        ),
        gorev(
            "ornek-g-3",
            "ornek-kisi-mudur1",
            "ornek-magaza-1",
            ROL_MUDUR,
            "Mağaza müdürü",
            None,
            None,
        ),
        gorev(
            "ornek-g-4",
            "ornek-kisi-ayse",
            "ornek-sube-1",
            "kasa-sorumlusu",
            "Kasa sorumlusu",
            Some("ornek-kisi-mudur1"),
            None,
        ),
        gorev(
            "ornek-g-5",
            "ornek-kisi-ali",
            "ornek-magaza-1",
            "uzman",
            "Satış uzmanı",
            Some("ornek-kisi-mudur1"),
            None,
        ),
        gorev(
            "ornek-g-6",
            "ornek-kisi-mudur2",
            "ornek-magaza-2",
            ROL_MUDUR,
            "Mağaza müdürü",
            None,
            None,
        ),
        gorev(
            "ornek-g-7",
            "ornek-kisi-can",
            "ornek-sube-2",
            "uzman",
            "Şube uzmanı",
            Some("ornek-kisi-mudur2"),
            None,
        ),
        gorev(
            "ornek-g-8",
            "ornek-kisi-pasif",
            "ornek-sube-1",
            "uzman",
            "Eski uzman",
            None,
            Some("2026-06-30"),
        ),
    ];
    for g in gorevler {
        d.gorevlendirme_ekle(g, zaman).expect("örnek görevlendirme");
    }
    (d, BTreeSet::from([ROL_MUDUR.to_string()]))
}

/// Örnek El Kitabı: iki konu, biri iki sürümlü.
pub fn ornek_belgeler(zaman: &str) -> BelgeDeposu {
    let mut b = BelgeDeposu::yeni();
    b.yayinla(
        "ornek-baslangic",
        "ÖRNEK Başlangıç",
        "# Başlangıç\nBu konu örnek bir El Kitabı sayfasıdır.\n\n## Kişi bulma\n- Üstteki aramaya ad, görev, şube ya da birim yazın.\n- Satıra tıklayınca ayrıntı açılır.\n\n## Adres kopyalama\nAyrıntıdaki düğme adresi panoya kopyalar; Rehber e-posta göndermez, alıcıyı her zaman siz seçersiniz.",
        &["örnek", "başlangıç"],
        None,
        zaman,
    )
    .expect("örnek belge");
    b.yayinla(
        "ornek-baslangic",
        "ÖRNEK Başlangıç",
        "# Başlangıç\nBu konu örnek bir El Kitabı sayfasıdır (ikinci sürüm).\n\n## Kişi bulma\n- Üstteki aramaya ad, görev, şube ya da birim yazın.\n- Şube ve görev süzgeçleri aramayla birlikte çalışır.\n- Satıra tıklayınca ayrıntı açılır.\n\n## Adres kopyalama\nAyrıntıdaki düğme adresi panoya kopyalar.",
        &["örnek", "başlangıç"],
        Some("Şube ve görev süzgeci eklendi."),
        zaman,
    )
    .expect("örnek belge");
    b.yayinla(
        "ornek-ihbar",
        "ÖRNEK İhbar hattı",
        "# İhbar hattı\nİhbar varsayılan olarak anonimdir.\n\n- İletişim bilgisi isteğe bağlıdır ve yalnız anonim seçimi kapatılınca sorulur.\n- Tam anonimlik garanti edilmez: metnin içeriği kişiyi ele verebilir.",
        &["örnek", "ihbar"],
        None,
        zaman,
    )
    .expect("örnek belge");
    b
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn ornek_veri_butunlugu_gecer_ve_etiketli() {
        let (d, m) = ornek_depo("2026-11-26T09:00:00Z");
        assert!(d.veri().dogrula().is_ok());
        assert!(m.contains(ROL_MUDUR));
        assert!(d
            .veri()
            .kisiler
            .values()
            .all(|k| k.ad.starts_with("ÖRNEK") && k.eposta.ends_with("@ornek.test")));
        assert!(d
            .veri()
            .birimler
            .values()
            .all(|b| b.ad.starts_with("ÖRNEK")));
        let b = ornek_belgeler("2026-11-26T09:00:00Z");
        assert_eq!(b.konular().len(), 2);
        assert!(b.konular().iter().all(|k| k.baslik.starts_with("ÖRNEK")));
    }
}
