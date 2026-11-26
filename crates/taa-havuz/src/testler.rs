// Sahip: Yusuf.
//! taa-havuz testleri: sahte sunucu (Gonderici) ile eşitleme, imleç, çevrimdışı okuma, dosya indirme/sürdürme.

use super::*;
use std::cell::{Cell, RefCell};

fn gecici() -> PathBuf {
    taa_dosya::gecici_klasor().unwrap()
}

fn sha_hex(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn yuzde_coz(s: &str) -> String {
    let b = s.as_bytes();
    let mut o = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            o.push(u8::from_str_radix(&s[i + 1..i + 3], 16).unwrap());
            i += 3;
        } else {
            o.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(o).unwrap()
}

/// Sunucu davranışı: `ogeler`/`olaylar`/`satirlar`/`icerik` uçlarını bellekte taklit eder.
#[derive(Default)]
struct Sahte {
    ogeler: RefCell<Vec<Value>>,
    olaylar: RefCell<Vec<Value>>,
    satirlar: RefCell<BTreeMap<i64, Vec<(String, Value)>>>,
    icerik: RefCell<BTreeMap<String, Vec<u8>>>,
    kopuk: Cell<bool>,
    /// Bu kadar ham istekten sonra ağ kopar (sürdürme testi).
    ham_hakki: Cell<Option<usize>>,
    bozuk_icerik: Cell<bool>,
    aranan_programlar: RefCell<Vec<String>>,
    ham_aralik: RefCell<Vec<(u64, u64)>>,
    sonralar: RefCell<Vec<String>>,
    satir_404: Cell<bool>,
}

fn sorgu(uc: &str) -> BTreeMap<String, String> {
    uc.split_once('?')
        .map(|(_, q)| q)
        .unwrap_or("")
        .split('&')
        .filter_map(|p| p.split_once('='))
        .map(|(k, v)| (k.to_string(), yuzde_coz(v)))
        .collect()
}

impl Sahte {
    #[allow(clippy::too_many_arguments)]
    fn oge_ekle(
        &self,
        id: i64,
        sira: i64,
        tur: &str,
        kip: &str,
        ad: &str,
        surum: i64,
        sha: Option<&str>,
        boyut: Option<usize>,
    ) {
        self.ogeler.borrow_mut().push(json!({
            "id": id, "sira": sira, "tur": tur, "kip": kip, "ad": ad, "surum_no": surum,
            "meta": {}, "sha256": sha, "boyut": boyut, "mime": null
        }));
        let olay_sira = self.olaylar.borrow().len() as i64 + 1;
        self.olaylar
            .borrow_mut()
            .push(json!({ "sira": olay_sira, "tip": "eklendi", "oge_id": id, "tur": tur }));
    }

    fn arsivle(&self, id: i64) {
        self.ogeler.borrow_mut().retain(|o| o["id"] != id);
        let olay_sira = self.olaylar.borrow().len() as i64 + 1;
        self.olaylar
            .borrow_mut()
            .push(json!({ "sira": olay_sira, "tip": "arsivlendi", "oge_id": id, "tur": "x" }));
    }

    fn dosya_oge(&self, id: i64, sira: i64, ad: &str, icerik: &[u8]) -> String {
        let sha = sha_hex(icerik);
        self.icerik
            .borrow_mut()
            .insert(sha.clone(), icerik.to_vec());
        self.oge_ekle(
            id,
            sira,
            "egitim.video",
            "dosya",
            ad,
            1,
            Some(&sha),
            Some(icerik.len()),
        );
        sha
    }

    fn kontrol(&self, uc: &str) -> Result<BTreeMap<String, String>, GonderHata> {
        if self.kopuk.get() {
            return Err(GonderHata::Ulasilamadi("ağ yok".into()));
        }
        let q = sorgu(uc);
        self.aranan_programlar
            .borrow_mut()
            .push(q.get("program").cloned().unwrap_or_default());
        Ok(q)
    }
}

impl Gonderici for Sahte {
    fn gonder(&self, _uc: &str, _govde: &Value) -> Result<Value, GonderHata> {
        unreachable!("istemci yalnız okur")
    }

    fn al(&self, uc: &str) -> Result<Value, GonderHata> {
        let q = self.kontrol(uc)?;
        let yol = uc.split('?').next().unwrap();
        let limit: usize = q["limit"].parse().unwrap();
        if yol == "/v1/havuz/ogeler" {
            let since: i64 = q["since"].parse().unwrap();
            let v: Vec<Value> = self
                .ogeler
                .borrow()
                .iter()
                .filter(|o| o["sira"].as_i64().unwrap() > since)
                .take(limit)
                .cloned()
                .collect();
            return Ok(json!({ "ogeler": v }));
        }
        if yol == "/v1/havuz/olaylar" {
            let since: i64 = q["since"].parse().unwrap();
            let v: Vec<Value> = self
                .olaylar
                .borrow()
                .iter()
                .filter(|o| o["sira"].as_i64().unwrap() > since)
                .take(limit)
                .cloned()
                .collect();
            return Ok(json!({ "olaylar": v }));
        }
        if let Some(r) = yol
            .strip_prefix("/v1/havuz/oge/")
            .and_then(|r| r.strip_suffix("/satirlar"))
        {
            if self.satir_404.get() {
                return Err(GonderHata::Reddedildi {
                    kod: 404,
                    mesaj: "Öğe yok".into(),
                });
            }
            let id: i64 = r.parse().unwrap();
            let tum = self.satirlar.borrow();
            let mut s: Vec<&(String, Value)> = tum[&id].iter().collect();
            s.sort_by(|a, b| a.0.cmp(&b.0));
            if let Some(sonra) = q.get("sonra") {
                self.sonralar.borrow_mut().push(sonra.clone());
                s.retain(|x| x.0.as_str() > sonra.as_str());
            }
            let sayfa: Vec<Value> = s
                .into_iter()
                .take(limit)
                .map(|(a, v)| json!({ "anahtar": a, "veri": v }))
                .collect();
            let son = sayfa.last().map(|x| x["anahtar"].clone());
            return Ok(json!({ "satirlar": sayfa, "son_anahtar": son }));
        }
        panic!("beklenmeyen uç: {uc}")
    }

    fn al_ham(&self, uc: &str, aralik: Option<(u64, u64)>) -> Result<Vec<u8>, GonderHata> {
        self.kontrol(uc)?;
        if let Some(h) = self.ham_hakki.get() {
            if h == 0 {
                return Err(GonderHata::Ulasilamadi("ağ koptu".into()));
            }
            self.ham_hakki.set(Some(h - 1));
        }
        let sha = uc.split('?').next().unwrap().rsplit('/').next().unwrap();
        let (a, b) = aralik.unwrap();
        self.ham_aralik.borrow_mut().push((a, b));
        let mut parca = self.icerik.borrow()[sha][a as usize..=b as usize].to_vec();
        if self.bozuk_icerik.get() {
            parca[0] ^= 0xff;
        }
        Ok(parca)
    }
}

fn rastgele(n: usize) -> Vec<u8> {
    (0..n)
        .map(|i| (i.wrapping_mul(2654435761) >> 7) as u8)
        .collect()
}

fn tablo_sahte(satir: usize) -> Sahte {
    let s = Sahte::default();
    s.oge_ekle(1, 10, "urun.liste", "tablo", "tum-urunler", 1, None, None);
    // anahtarlar yüzde kodlama gerektirir: boşluk, eğik çizgi, artı, yüzde, Türkçe harf
    s.satirlar.borrow_mut().insert(
        1,
        (0..satir)
            .map(|i| (format!("K{i:04} /+%ü"), json!({ "k": i })))
            .collect(),
    );
    s
}

#[test]
fn ilk_esitleme_tablo_sayfalar_ve_anahtar_kodlanir() {
    let kok = gecici();
    let s = tablo_sahte(1200);
    let mut h = Havuz::ac(&kok, "fiyatetiketi").unwrap();
    let o = h.esitle(&s).unwrap();
    assert_eq!(
        o,
        Ozet {
            yeni: 1,
            arsivlenen: 0,
            satir: 1200
        }
    );
    let satirlar = h.satirlar(1).unwrap();
    assert_eq!(satirlar.len(), 1200);
    assert_eq!(
        h.satir(1, "K0007 /+%ü").unwrap().unwrap(),
        json!({ "k": 7 })
    );
    assert_eq!(h.satir(1, "yok").unwrap(), None);
    // 500'lük sayfalar: ikinci ve üçüncü istek `sonra` ile gitti, sunucu kodu çözüp doğru anahtarı gördü
    assert_eq!(
        *s.sonralar.borrow(),
        vec!["K0499 /+%ü".to_string(), "K0999 /+%ü".to_string()]
    );
    assert!(
        s.aranan_programlar
            .borrow()
            .iter()
            .all(|p| p == "fiyatetiketi"),
        "her istek program ilgisini bildirir"
    );
    assert_eq!(h.ogeler(None).len(), 1);
    assert_eq!(h.bul("urun.liste", "tum-urunler").unwrap().id, 1);
    assert!(h.satirlar(99).is_err());
}

#[test]
fn imlec_kalici_ikinci_esitleme_bos_ve_yeni_gelenler_artimli() {
    let kok = gecici();
    let s = tablo_sahte(3);
    let mut h = Havuz::ac(&kok, "tutaa").unwrap();
    h.esitle(&s).unwrap();
    assert_eq!(h.imlecler(), (10, 1));
    // yeniden aç: imleç diskten gelir, yeni şey yok
    let mut h2 = Havuz::ac(&kok, "tutaa").unwrap();
    assert_eq!(h2.imlecler(), (10, 1));
    assert_eq!(h2.esitle(&s).unwrap(), Ozet::default());
    // sunucuda yeni sürüm gelir, eskisi arşivlenir
    s.oge_ekle(2, 11, "urun.liste", "tablo", "tum-urunler", 2, None, None);
    s.satirlar
        .borrow_mut()
        .insert(2, vec![("A".into(), json!(1)), ("B".into(), json!(2))]);
    s.arsivle(1);
    let o = h2.esitle(&s).unwrap();
    assert_eq!(
        o,
        Ozet {
            yeni: 1,
            arsivlenen: 1,
            satir: 2
        }
    );
    assert_eq!(h2.ogeler(None).len(), 1);
    assert_eq!(h2.bul("urun.liste", "tum-urunler").unwrap().surum_no, 2);
    assert!(h2.satirlar(1).is_err(), "arşivdeki öğe dizinde yok");
    assert!(
        h2.klasor().join("tablo/1.json").exists(),
        "eski satırlar silinmez"
    );
    // farklı program ayrı önbellek
    let h3 = Havuz::ac(&kok, "taarkod").unwrap();
    assert!(h3.ogeler(None).is_empty());
}

#[test]
fn cevrimdisi_okuma_ve_ag_hatasinda_onbellek_bozulmaz() {
    let kok = gecici();
    let s = tablo_sahte(5);
    let icerik = rastgele(3000);
    let sha = s.dosya_oge(2, 11, "egitim-1", &icerik);
    let mut h = Havuz::ac(&kok, "taarehber").unwrap();
    h.esitle(&s).unwrap();
    h.indir(&s, 2).unwrap();
    s.kopuk.set(true);
    let e = h.esitle(&s).unwrap_err();
    assert!(e.cevrimdisi_mi(), "{e}");
    // ağ yokken her şey okunur
    assert_eq!(h.satirlar(1).unwrap().len(), 5);
    let yol = h.dosya_yolu(2).unwrap();
    assert_eq!(fs::read(&yol).unwrap(), icerik);
    assert!(yol.ends_with(&sha));
    assert_eq!(h.imlecler(), (11, 2));
    // yeniden açılınca da aynı
    let h2 = Havuz::ac(&kok, "taarehber").unwrap();
    assert_eq!(h2.ogeler(None).len(), 2);
    assert!(h2.dosya_yolu(2).is_some());
    // indirilmiş dosya için ağsız `indir` yolu döndürür
    assert_eq!(h2.indir(&s, 2).unwrap(), yol);
}

#[test]
fn dosya_parca_parca_iner_ve_kesintide_surer() {
    let kok = gecici();
    let s = Sahte::default();
    let icerik = rastgele(2 * PARCA as usize + 300);
    s.dosya_oge(5, 7, "video", &icerik);
    let mut h = Havuz::ac(&kok, "taarehber").unwrap();
    h.esitle(&s).unwrap();
    assert!(h.dosya_yolu(5).is_none(), "dosyalar isteğe bağlı iner");
    // ilk parçadan sonra ağ kopar
    s.ham_hakki.set(Some(1));
    let e = h.indir(&s, 5).unwrap_err();
    assert!(e.cevrimdisi_mi());
    assert!(h.dosya_yolu(5).is_none(), "yarım dosya sunulmaz");
    // geri gelince kaldığı yerden sürer (baştan değil)
    s.ham_hakki.set(None);
    s.ham_aralik.borrow_mut().clear();
    let yol = h.indir(&s, 5).unwrap();
    assert_eq!(fs::read(&yol).unwrap(), icerik);
    assert_eq!(
        *s.ham_aralik.borrow(),
        vec![(PARCA, 2 * PARCA - 1), (2 * PARCA, 2 * PARCA + 299)]
    );
    assert!(
        !h.klasor()
            .join("icerik")
            .join(format!("{}.part", sha_hex(&icerik)))
            .exists(),
        "yarım dosya yerine taşındı"
    );
    assert_eq!(h.dosya_yolu(5), Some(yol));
}

#[test]
fn bozuk_icerik_reddedilir_ve_sifirlanir() {
    let kok = gecici();
    let s = Sahte::default();
    let icerik = rastgele(1500);
    let sha = s.dosya_oge(5, 7, "video", &icerik);
    let mut h = Havuz::ac(&kok, "taarehber").unwrap();
    h.esitle(&s).unwrap();
    s.bozuk_icerik.set(true);
    match h.indir(&s, 5) {
        Err(HavuzHata::Ozet { beklenen, bulunan }) => {
            assert_eq!(beklenen, sha);
            assert_ne!(bulunan, sha);
        }
        d => panic!("özet hatası beklenirdi: {d:?}"),
    }
    assert!(h.dosya_yolu(5).is_none());
    let part = h.klasor().join("icerik").join(format!("{sha}.part"));
    assert_eq!(
        fs::metadata(&part).unwrap().len(),
        0,
        "yarım dosya sıfırlandı"
    );
    // düzelince temiz inmesi
    s.bozuk_icerik.set(false);
    assert_eq!(fs::read(h.indir(&s, 5).unwrap()).unwrap(), icerik);
}

#[test]
fn arsivlenen_dosya_dizinden_duser_ama_dosya_kalir_yetim_sayilir() {
    let kok = gecici();
    let s = Sahte::default();
    let icerik = rastgele(400);
    s.dosya_oge(5, 7, "video", &icerik);
    let mut h = Havuz::ac(&kok, "taarehber").unwrap();
    h.esitle(&s).unwrap();
    let yol = h.indir(&s, 5).unwrap();
    assert!(h.yetimler().unwrap().is_empty());
    s.arsivle(5);
    assert_eq!(h.esitle(&s).unwrap().arsivlenen, 1);
    assert!(h.dosya_yolu(5).is_none());
    assert!(yol.exists(), "silme yok");
    assert_eq!(h.yetimler().unwrap(), vec![yol]);
}

#[test]
fn tablo_satirlari_alinirken_oge_kaybolursa_imlec_takilmaz() {
    let kok = gecici();
    let s = tablo_sahte(3);
    s.oge_ekle(2, 11, "fiyat.liste", "tablo", "fiyatlar", 1, None, None);
    s.satirlar
        .borrow_mut()
        .insert(2, vec![("A".into(), json!(1))]);
    s.satir_404.set(true);
    let mut h = Havuz::ac(&kok, "fiyatetiketi").unwrap();
    let o = h.esitle(&s).unwrap();
    assert_eq!(
        (o.yeni, h.imlecler().0),
        (0, 11),
        "görünmez öğe atlandı, imleç ilerledi"
    );
    assert!(h.ogeler(None).is_empty());
}

#[test]
fn gecersiz_program_bozuk_onbellek_ve_bozuk_yanit() {
    let kok = gecici();
    for p in ["", "Büyük", "a b", &"x".repeat(33)] {
        assert!(
            matches!(Havuz::ac(&kok, p), Err(HavuzHata::Gecersiz(_))),
            "{p:?}"
        );
    }
    // bozuk dizin.json sessizce ezilmez
    let dizin = kok.join(KLASOR).join("taadon");
    fs::create_dir_all(&dizin).unwrap();
    fs::write(dizin.join("dizin.json"), "{bozuk").unwrap();
    assert!(matches!(
        Havuz::ac(&kok, "taadon"),
        Err(HavuzHata::Bicim(_))
    ));
    assert_eq!(
        fs::read_to_string(dizin.join("dizin.json")).unwrap(),
        "{bozuk"
    );
    // sunucu yanıtı beklenen biçimde değilse hata (çökmez)
    struct Bozuk;
    impl Gonderici for Bozuk {
        fn gonder(&self, _: &str, _: &Value) -> Result<Value, GonderHata> {
            unreachable!()
        }
        fn al(&self, _: &str) -> Result<Value, GonderHata> {
            Ok(json!({ "x": 1 }))
        }
    }
    let mut h = Havuz::ac(&gecici(), "taadon").unwrap();
    assert!(matches!(h.esitle(&Bozuk), Err(HavuzHata::Bicim(_))));
    // ham GET desteklemeyen gönderici: yapılandırma hatası
    let s = Sahte::default();
    s.dosya_oge(5, 7, "v", &rastgele(10));
    let mut h = Havuz::ac(&gecici(), "taarehber").unwrap();
    h.esitle(&s).unwrap();
    struct Yalniz(Sahte);
    impl Gonderici for Yalniz {
        fn gonder(&self, _: &str, _: &Value) -> Result<Value, GonderHata> {
            unreachable!()
        }
        fn al(&self, uc: &str) -> Result<Value, GonderHata> {
            self.0.al(uc)
        }
    }
    assert!(matches!(
        h.indir(&Yalniz(s), 5),
        Err(HavuzHata::Gonder(GonderHata::Yapilandirma(_)))
    ));
}

#[test]
fn yuzde_kodlama() {
    assert_eq!(yuzde_kodla("ab-1_2.3"), "ab-1_2.3");
    assert_eq!(yuzde_kodla("a b/c+%"), "a%20b%2Fc%2B%25");
    assert_eq!(yuzde_kodla("ü"), "%C3%BC");
    assert!(taa_kuyruk::uc_sorgulu_gecerli(&format!(
        "/v1/havuz/oge/1/satirlar?sonra={}",
        yuzde_kodla("K 1/ü+%~")
    )));
}
