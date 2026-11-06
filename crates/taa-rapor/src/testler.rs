// Sahip: Yusuf. yazan: [Claude#taa-rapor-T5 2026-11-06]
//! taa-rapor testleri: geçici kökte örnek alan dosyaları; sonunda silinir (yalnız testin kendi klasörü).

use super::*;

struct Ortam {
    kok: PathBuf,
}

impl Ortam {
    fn yeni() -> Ortam {
        Ortam {
            kok: taa_dosya::gecici_klasor().unwrap(),
        }
    }
    fn yaz(&self, goreli: &str, icerik: &str) -> PathBuf {
        let yol = self.kok.join(goreli);
        fs::create_dir_all(yol.parent().unwrap()).unwrap();
        fs::write(&yol, icerik).unwrap();
        yol
    }
}

impl Drop for Ortam {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.kok);
    }
}

fn deger<'a>(b: &'a Bolum, anahtar: &str) -> &'a str {
    b.satirlar
        .iter()
        .find(|(k, _)| k == anahtar)
        .map(|(_, v)| v.as_str())
        .unwrap_or_else(|| panic!("{} satırı yok: {:?}", anahtar, b.satirlar))
}

fn bolum<'a>(r: &'a Rapor, ad: &str) -> &'a Bolum {
    r.bolumler.iter().find(|b| b.ad == ad).unwrap()
}

const FIS_DEFTERI: &str = r#"{"tur":"fis","v":{"no":"F-1","tarih":"2026-11-01","musteri":"A","telefon":"","kalemler":[],"teslim":"2026-11-03","ucret_kurus":10000,"on_odeme_kurus":2500,"usta":"Ali","kargo":true,"durum":"bekliyor"}}
{"tur":"fis","v":{"no":"F-2","tarih":"2026-11-01","musteri":"B","telefon":"","kalemler":[],"teslim":"2026-11-06","ucret_kurus":5000,"on_odeme_kurus":0,"usta":"Veli","durum":"bekliyor"}}
{"tur":"fis","v":{"no":"F-3","tarih":"2026-11-01","musteri":"C","telefon":"","kalemler":[],"teslim":"2026-11-02","ucret_kurus":7000,"on_odeme_kurus":7000,"usta":"Ali","durum":"bekliyor"}}
{"tur":"durum","v":{"no":"F-3","durum":"teslim","tarih":"2026-11-02"}}
{"tur":"durum","v":{"no":"F-2","durum":"terzihanede","tarih":"2026-11-02"}}
{"tur":"teslim","v":{"no":"F-1","degisiklik":{"eski":"2026-11-03","yeni":"2026-11-04","neden":"x","kim":"y","tarih":"2026-11-02"},"saat":"17:00"}}
{"tur":"raf","v":{"no":"F-2","raf":"A1","kim":"k","tarih":"2026-11-03"}}
bu satır bozuk
"#;

#[test]
fn fis_bolumu_sayar_ve_bozuk_satiri_atlar() {
    let o = Ortam::yeni();
    o.yaz("fis-defteri/fisler.jsonl", FIS_DEFTERI);
    let k = Kaynaklar::veri_kokunden(&o.kok);
    let r = uret(&k, "2026-11-06").unwrap();
    let f = bolum(&r, "fis");
    assert_eq!(f.durum, Durum::Var);
    assert_eq!(deger(f, "toplam_fis"), "3");
    assert_eq!(deger(f, "acik"), "2");
    assert_eq!(deger(f, "bekliyor"), "1");
    assert_eq!(deger(f, "terzihanede"), "1");
    assert_eq!(deger(f, "teslim"), "1");
    // F-1 teslimi 2026-11-04'e alındı, hâlâ açık → geciken; F-2 bugün teslim.
    assert_eq!(deger(f, "geciken"), "1");
    assert_eq!(deger(f, "bugun_teslim"), "1");
    assert_eq!(deger(f, "kargo_bekleyen"), "1");
    assert_eq!(deger(f, "acik_kalan_ucret"), "125,00 TL");
    assert_eq!(deger(f, "usta_acik:Ali"), "1");
    assert_eq!(deger(f, "usta_acik:Veli"), "1");
    assert_eq!(f.uyarilar.len(), 1, "{:?}", f.uyarilar);
    assert!(f.uyarilar[0].starts_with("8. satır bozuk"));
    // Diğer bölümler kaynak yok.
    for ad in ["kasa", "bildirim", "katalog", "pano", "yedek"] {
        assert_eq!(bolum(&r, ad).durum, Durum::Yok, "{ad}");
    }
}

#[test]
fn kasa_bildirim_katalog_pano_yedek_bolumleri() {
    let o = Ortam::yeni();
    o.yaz(
        "kasa-defteri/kasa.jsonl",
        r#"{"tur":"kapanis","v":{"id":"K-1","tarih":"2026-11-04","sube":"M","kullanici":"u","adetler":{},"sayilan_kurus":100000,"beklenen_kurus":100000,"fark_kurus":0,"not":""}}
{"tur":"kapanis","v":{"id":"K-2","tarih":"2026-11-05","sube":"M","kullanici":"u","adetler":{},"sayilan_kurus":99500,"beklenen_kurus":100000,"fark_kurus":-500,"not":""}}
{"tur":"kapanis","v":{"id":"K-3","tarih":"2026-11-05","sube":"M","kullanici":"u","adetler":{},"sayilan_kurus":1,"beklenen_kurus":1,"fark_kurus":0,"not":"yanlış"}}
{"tur":"iptal","v":{"id":"K-3","neden":"yanlış","kullanici":"u"}}
"#,
    );
    o.yaz(
        "bildirim-kuyrugu/bildirimler.jsonl",
        r#"{"tur":"mesaj","v":{"id":"M-1","fis_no":"F-1","telefon":"905321234567","metin":"x","tarih":"t","durum":"Bekliyor"}}
{"tur":"mesaj","v":{"id":"M-2","fis_no":"F-2","telefon":"905321234567","metin":"x","tarih":"t","durum":"Bekliyor"}}
{"tur":"sonuc","v":{"id":"M-1","durum":"Gonderildi","tarih":"t"}}
{"tur":"sonuc","v":{"id":"M-2","durum":{"Hata":"ağ yok"},"tarih":"t"}}
{"tur":"mesaj","v":{"id":"M-3","fis_no":"F-3","telefon":"905321234567","metin":"x","tarih":"t","durum":"Bekliyor"}}
"#,
    );
    o.yaz(
        "urunler/katalog.json",
        r#"{"kaynak":"nebim-2026-11-05.csv","urunler":{"A|1":{"kod":"A","barkod":"1","ad":"Gömlek","renk":"","beden":"","fiyat":10000,"indirimli":8000},"B|2":{"kod":"B","barkod":"2","ad":"Pantolon","renk":"","beden":"","fiyat":0,"indirimli":null}}}"#,
    );
    o.yaz(
        "pano-kartlari/etiket.pano.json",
        r#"{"baslik":"Etiket kuyruğu","deger":"3","durum":"uyari"}"#,
    );
    o.yaz("pano-kartlari/bozuk.pano.json", "{");
    o.yaz(
        "yedekler/20261105-010000/manifest.json",
        r#"{"kayitlar":[{"yol":"k0_a/x","boyut":10,"sha256":"0"},{"yol":"k0_a/y","boyut":5,"sha256":"0"}]}"#,
    );
    o.yaz(
        "yedekler/20261104-010000/manifest.json",
        r#"{"kayitlar":[]}"#,
    );
    o.yaz("yedekler/not-a-backup/dosya.txt", "x");
    let k = Kaynaklar::veri_kokunden(&o.kok);
    let r = uret(&k, "2026-11-06").unwrap();

    let ka = bolum(&r, "kasa");
    assert_eq!(deger(ka, "kapanis"), "2");
    assert_eq!(deger(ka, "iptal"), "1");
    assert_eq!(deger(ka, "son_kapanis"), "2026-11-05");
    assert_eq!(deger(ka, "son_sayilan"), "995,00 TL");
    assert_eq!(deger(ka, "son_fark"), "-5,00 TL");
    assert_eq!(deger(ka, "farkli_kapanis"), "1");
    assert_eq!(deger(ka, "toplam_fark"), "-5,00 TL");

    let b = bolum(&r, "bildirim");
    assert_eq!(deger(b, "toplam"), "3");
    assert_eq!(deger(b, "bekliyor"), "1");
    assert_eq!(deger(b, "gonderildi"), "1");
    assert_eq!(deger(b, "hata"), "1");

    let kt = bolum(&r, "katalog");
    assert_eq!(deger(kt, "kaynak_dosya"), "nebim-2026-11-05.csv");
    assert_eq!(deger(kt, "urun"), "2");
    assert_eq!(deger(kt, "indirimli"), "1");
    assert_eq!(deger(kt, "fiyatsiz"), "1");

    let p = bolum(&r, "pano");
    assert_eq!(deger(p, "kart"), "2");
    assert_eq!(deger(p, "kart:Etiket kuyruğu"), "3 [uyari]");
    assert_eq!(p.uyarilar.len(), 1);
    assert!(p.uyarilar[0].starts_with("bozuk.pano.json"));

    let y = bolum(&r, "yedek");
    assert_eq!(deger(y, "yedek"), "2");
    assert_eq!(deger(y, "son_yedek"), "20261105-010000");
    assert_eq!(deger(y, "son_dosya"), "2");
    assert_eq!(deger(y, "son_boyut_bayt"), "15");
}

#[test]
fn fis_klasoru_kok_dosyalarinin_yerine_gecer_ve_okunamayan_hata_bolumu_olur() {
    let o = Ortam::yeni();
    o.yaz("fis-defteri/fisler.jsonl", "");
    let sunucu = o.kok.join("sunucu-veri");
    o.yaz("sunucu-veri/fisler.jsonl", FIS_DEFTERI);
    o.yaz("sunucu-veri/bildirimler.jsonl", "");
    let mut k = Kaynaklar::veri_kokunden(&o.kok);
    assert!(k.bildirim.is_none());
    k.fis_klasoru_ekle(&sunucu);
    assert_eq!(
        k.fis.as_deref(),
        Some(sunucu.join("fisler.jsonl").as_path())
    );
    assert!(k.bildirim.is_some());
    let r = uret(&k, "2026-11-06").unwrap();
    assert_eq!(deger(bolum(&r, "fis"), "toplam_fis"), "3");
    assert_eq!(deger(bolum(&r, "bildirim"), "toplam"), "0");

    // Okunamayan kaynak: bölüm "hata", rapor yine üretilir.
    let bos = Kaynaklar {
        katalog: Some(o.kok.join("yok.json")),
        pano: Some(o.kok.join("yok-klasor")),
        ..Kaynaklar::default()
    };
    let r = uret(&bos, "2026-11-06").unwrap();
    assert_eq!(bolum(&r, "katalog").durum, Durum::Hata);
    assert_eq!(bolum(&r, "pano").durum, Durum::Hata);
    assert!(bolum(&r, "katalog").uyarilar[0].starts_with("okunamadı"));
    assert!(uret(&bos, "6.11.2026").is_err());
}

#[test]
fn ciktilar_metin_json_csv() {
    let o = Ortam::yeni();
    o.yaz("fis-defteri/fisler.jsonl", FIS_DEFTERI);
    o.yaz(
        "pano-kartlari/a.pano.json",
        r#"{"baslik":"Nokta;virgül \"tırnak\"","deger":"1"}"#,
    );
    let r = uret(&Kaynaklar::veri_kokunden(&o.kok), "2026-11-06").unwrap();
    let m = r.metin();
    assert!(m.starts_with("TAA paket raporu — 2026-11-06\n"));
    assert!(m.contains("[Fiş defteri] "));
    assert!(m.contains("  toplam_fis: 3\n"));
    assert!(m.contains("[Kasa defteri] kaynak yok\n"));
    assert!(m.contains("  ! 8. satır bozuk"));

    let j = r.json();
    assert_eq!(j["bugun"], "2026-11-06");
    assert_eq!(j["bolumler"][0]["ad"], "fis");
    assert_eq!(j["bolumler"][0]["durum"], "var");
    assert_eq!(j["bolumler"][0]["satirlar"][0]["anahtar"], "toplam_fis");
    assert_eq!(j["bolumler"][0]["satirlar"][0]["deger"], "3");
    assert_eq!(j["bolumler"][1]["durum"], "yok");

    let c = r.csv();
    assert!(c.starts_with("\u{feff}bolum;durum;anahtar;deger\r\n"));
    assert!(c.contains("fis;var;toplam_fis;3\r\n"));
    assert!(c.contains("kasa;yok;kaynak;\r\n"));
    assert!(c.contains("pano;var;\"kart:Nokta;virgül \"\"tırnak\"\"\";1 [tamam]\r\n"));
    assert!(c.contains("fis;var;uyari;8. satır bozuk"));
}

#[test]
fn kurus_metni_bicimi() {
    assert_eq!(kurus_metni(0), "0,00 TL");
    assert_eq!(kurus_metni(5), "0,05 TL");
    assert_eq!(kurus_metni(123_456), "1.234,56 TL");
    assert_eq!(kurus_metni(-500), "-5,00 TL");
    assert_eq!(kurus_metni(100_000_000), "1.000.000,00 TL");
}
