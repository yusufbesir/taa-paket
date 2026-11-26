//! API düzeyi testler (saf işlevler; ağ yok). Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]

use super::*;
use crate::api::sorgu_coz;
use crate::durum::YONETICI_BOSTA_SN;
use serde_json::json;
use std::cell::Cell;
use std::path::PathBuf;

/// 2026-11-26 10:00:00 UTC
const T0: u64 = 1_795_687_200;

struct SahteOrtam {
    simdi: Cell<u64>,
    sayac: Cell<u64>,
}

impl SahteOrtam {
    fn yeni() -> Self {
        SahteOrtam {
            simdi: Cell::new(T0),
            sayac: Cell::new(1),
        }
    }
    fn ilerle(&self, sn: u64) {
        self.simdi.set(self.simdi.get() + sn);
    }
}

impl Ortam for SahteOrtam {
    fn simdi(&self) -> u64 {
        self.simdi.get()
    }
    fn rastgele(&self, b: &mut [u8]) -> Result<(), String> {
        let n = self.sayac.get();
        self.sayac.set(n + 1);
        for (i, x) in b.iter_mut().enumerate() {
            *x = (n as u8)
                .wrapping_mul(31)
                .wrapping_add(i as u8)
                .wrapping_add(7);
        }
        Ok(())
    }
}

/// Test kimliği: yalnız testlerde var; üretimde sahte giriş yoktur.
struct SahteKimlik(Result<KimlikBilgisi, KimlikRed>);

impl Kimlik for SahteKimlik {
    fn giris(&self, _: &str, _: &str) -> Result<KimlikBilgisi, KimlikRed> {
        self.0.clone()
    }
}

fn yonetici(izinler: &[&str]) -> Box<dyn Kimlik> {
    Box::new(SahteKimlik(Ok(KimlikBilgisi {
        kullanici: "test-yonetici".into(),
        izinler: izinler.iter().map(|s| s.to_string()).collect(),
    })))
}

struct Ortamlar {
    kok: PathBuf,
    o: SahteOrtam,
    d: Durum,
}

impl Drop for Ortamlar {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.kok);
    }
}

fn kur(demo: bool, sube: Option<&str>, kimlik: Box<dyn Kimlik>) -> Ortamlar {
    kur_kokte(taa_dosya::gecici_klasor().unwrap(), demo, sube, kimlik)
}

fn kur_kokte(kok: PathBuf, demo: bool, sube: Option<&str>, kimlik: Box<dyn Kimlik>) -> Ortamlar {
    let o = SahteOrtam::yeni();
    let d = Durum::yeni(
        Yapilandirma {
            kok: kok.clone(),
            sube_kodu: sube.map(Into::into),
            demo,
            kimlik,
        },
        &o,
    )
    .unwrap();
    Ortamlar { kok, o, d }
}

fn al(t: &mut Ortamlar, yol: &str, sorgu: &str) -> Yanit {
    let g = Value::Null;
    yonlendir(
        &mut t.d,
        &t.o,
        &Istek {
            yontem: "GET",
            yol,
            sorgu,
            govde: &g,
        },
    )
}

fn gonder(t: &mut Ortamlar, yol: &str, govde: Value) -> Yanit {
    yonlendir(
        &mut t.d,
        &t.o,
        &Istek {
            yontem: "POST",
            yol,
            sorgu: "",
            govde: &govde,
        },
    )
}

fn girisli(izinler: &[&str]) -> Ortamlar {
    let mut t = kur(true, Some(demo::SUBE_1), yonetici(izinler));
    let (k, _) = gonder(
        &mut t,
        "/api/yonetici/giris",
        json!({"kullanici": "x", "parola": "y"}),
    );
    assert_eq!(k, 200);
    t
}

fn adlar(v: &Value) -> Vec<String> {
    v["kisiler"]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k["ad"].as_str().unwrap().to_string())
        .collect()
}

// ------------------------------------------------------------ yönetici: varsayılan reddet

const YONETIM_UCLARI: &[(&str, &str)] = &[
    ("GET", "/api/yonetim/secenekler"),
    ("GET", "/api/yonetim/denetim"),
    ("POST", "/api/yonetim/kisi/ekle"),
    ("POST", "/api/yonetim/kisi/duzelt"),
    ("POST", "/api/yonetim/birim/ekle"),
    ("POST", "/api/yonetim/birim/duzelt"),
    ("POST", "/api/yonetim/gorevlendirme/ekle"),
];

fn tum_yonetim_uclari_403(t: &mut Ortamlar) {
    for (yontem, yol) in YONETIM_UCLARI {
        let g = json!({"ad": "x", "eposta": "x@ornek.test"});
        let (k, v) = yonlendir(
            &mut t.d,
            &t.o,
            &Istek {
                yontem,
                yol,
                sorgu: "",
                govde: &g,
            },
        );
        assert_eq!(k, 403, "{yontem} {yol}: {v}");
        assert_eq!(v["kod"], "yetkisiz");
    }
}

#[test]
fn oturum_yokken_butun_yonetici_uclari_403() {
    let mut t = kur(true, Some(demo::SUBE_1), Box::new(KimlikYok));
    tum_yonetim_uclari_403(&mut t);
    // Bilinmeyen yönetim yolu da (yetkisiz) 403, 404 ile yol varlığı sızmaz.
    assert_eq!(al(&mut t, "/api/yonetim/yok", "").0, 403);
    // Kayıt hiç değişmedi.
    assert_eq!(t.d.denetim.len(), 0);
}

#[test]
fn kimlik_dogrulanamiyorsa_giris_403_ve_yonetici_acilmaz() {
    let mut t = kur(true, Some(demo::SUBE_1), Box::new(KimlikYok));
    let (k, v) = gonder(
        &mut t,
        "/api/yonetici/giris",
        json!({"kullanici": "admin", "parola": "gizli"}),
    );
    assert_eq!(k, 403, "{v}");
    assert_eq!(v["kod"], "dogrulanamadi");
    assert!(!v.to_string().contains("gizli"), "parola yanıtta yok");
    assert!(t.d.yonetici.is_none());
    tum_yonetim_uclari_403(&mut t);
}

#[test]
fn giris_girdisi_ve_red_kodlari() {
    let mut t = kur(
        true,
        None,
        Box::new(SahteKimlik(Err(KimlikRed::Reddedildi(
            "kullanıcı adı ve parolayı denetleyin".into(),
        )))),
    );
    assert_eq!(
        gonder(
            &mut t,
            "/api/yonetici/giris",
            json!({"kullanici": "", "parola": "x"})
        )
        .0,
        400
    );
    assert_eq!(
        gonder(&mut t, "/api/yonetici/giris", json!({"kullanici": "a"})).0,
        400
    );
    assert_eq!(
        gonder(
            &mut t,
            "/api/yonetici/giris",
            json!({"kullanici": "a", "parola": "b"})
        )
        .0,
        401
    );
    assert!(t.d.yonetici.is_none());
}

#[test]
fn giris_basarili_ama_rehber_yaz_izni_yoksa_403() {
    let mut t = kur(true, None, yonetici(&["rehber.tum.oku", "kimlik.yonetici"]));
    let (k, v) = gonder(
        &mut t,
        "/api/yonetici/giris",
        json!({"kullanici": "a", "parola": "b"}),
    );
    assert_eq!(k, 403, "{v}");
    assert!(
        t.d.yonetici.is_none(),
        "izin yoksa oturum açılmaz (rol adı tek başına yetmez)"
    );
    tum_yonetim_uclari_403(&mut t);
}

#[test]
fn giris_cikis_ve_bosta_kalma_suresi() {
    let mut t = girisli(&[IZIN_YAZ_]);
    let (k, v) = al(&mut t, "/api/durum", "");
    assert_eq!(k, 200);
    assert_eq!(v["yonetici"]["girisli"], true);
    assert_eq!(v["yonetici"]["yazma_izni"], true);
    assert_eq!(al(&mut t, "/api/yonetim/secenekler", "").0, 200);
    // Boşta kalma: süre dolunca oturum düşer.
    t.o.ilerle(YONETICI_BOSTA_SN - 1);
    assert_eq!(
        al(&mut t, "/api/yonetim/secenekler", "").0,
        200,
        "etkinlik süreyi yeniler"
    );
    t.o.ilerle(YONETICI_BOSTA_SN + 1);
    assert_eq!(al(&mut t, "/api/yonetim/secenekler", "").0, 403);
    assert_eq!(al(&mut t, "/api/durum", "").1["yonetici"]["girisli"], false);
    // Çıkış.
    let mut t = girisli(&[IZIN_YAZ_]);
    assert_eq!(gonder(&mut t, "/api/yonetici/cikis", json!({})).0, 200);
    assert_eq!(al(&mut t, "/api/yonetim/secenekler", "").0, 403);
}

const IZIN_YAZ_: &str = "rehber.yaz";

// ------------------------------------------------------------ yönetim formları

#[test]
fn kisi_birim_gorevlendirme_ekleme_ve_gorunurluk() {
    let mut t = girisli(&[IZIN_YAZ_]);
    let (k, v) = gonder(
        &mut t,
        "/api/yonetim/kisi/ekle",
        json!({"ad": "ÖRNEK Yeni Kişi", "eposta": "Yeni.Kisi@ORNEK.test"}),
    );
    assert_eq!(k, 201, "{v}");
    let kisi_id = v["id"].as_str().unwrap().to_string();
    assert!(taa_rehber_model::model::kimlik_gecerli(&kisi_id));
    assert_eq!(v["surum"], 1);
    // Görevlendirmesi yokken şube cihazında görünmez.
    let (_, l) = al(&mut t, "/api/kisiler", "");
    assert!(!adlar(&l).contains(&"ÖRNEK Yeni Kişi".to_string()));
    // Birim ekle (şube 1'in altında mağaza).
    let (k, b) = gonder(
        &mut t,
        "/api/yonetim/birim/ekle",
        json!({
            "ad": "ÖRNEK Yeni Mağaza", "tur": "magaza", "ust_birim_id": "ornek-sube-1",
            "sube_kodu": demo::SUBE_1, "ortak_eposta": "yeni.magaza@ornek.test"
        }),
    );
    assert_eq!(k, 201, "{b}");
    let birim_id = b["id"].as_str().unwrap().to_string();
    // Görevlendirme: tarih varsayılan bugün.
    let (k, g) = gonder(
        &mut t,
        "/api/yonetim/gorevlendirme/ekle",
        json!({
            "kisi_id": kisi_id, "birim_id": birim_id, "rol": "uzman", "gorev": "Yeni uzman"
        }),
    );
    assert_eq!(k, 201, "{g}");
    let (_, l) = al(&mut t, "/api/kisiler", "ara=yeni+uzman");
    assert_eq!(adlar(&l), vec!["ÖRNEK Yeni Kişi".to_string()], "{l}");
    assert_eq!(
        l["kisiler"][0]["eposta"], "yeni.kisi@ornek.test",
        "e-posta küçük harfe çevrilir"
    );
    // Seçenekler: e-posta yok.
    let (_, s) = al(&mut t, "/api/yonetim/secenekler", "");
    assert!(
        !s.to_string().contains("@"),
        "seçeneklerde e-posta yok: {s}"
    );
    assert!(s["kisiler"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x["id"] == kisi_id.as_str()));
    // Denetim izi: e-posta ve ad yok, id var.
    let (_, dn) = al(&mut t, "/api/yonetim/denetim", "");
    let satirlar = dn["satirlar"].as_array().unwrap();
    assert!(satirlar.len() >= 4, "{dn}");
    assert!(satirlar
        .iter()
        .any(|x| x.as_str().unwrap().contains(&format!("id={kisi_id}"))));
    assert!(!dn.to_string().contains("ornek.test") && !dn.to_string().contains("Yeni Kişi"));
}

#[test]
fn gecersiz_form_girdileri() {
    let mut t = girisli(&[IZIN_YAZ_]);
    let kotu = |t: &mut Ortamlar, yol: &str, g: Value, beklenen: u16| {
        let (k, v) = gonder(t, yol, g);
        assert_eq!(k, beklenen, "{yol}: {v}");
        assert!(v["hata"].is_string());
    };
    kotu(
        &mut t,
        "/api/yonetim/kisi/ekle",
        json!({"ad": " ", "eposta": "a@b.co"}),
        400,
    );
    kotu(
        &mut t,
        "/api/yonetim/kisi/ekle",
        json!({"ad": "A", "eposta": 5}),
        400,
    );
    kotu(
        &mut t,
        "/api/yonetim/kisi/ekle",
        json!({"ad": "A", "eposta": "yanlis"}),
        422,
    );
    kotu(
        &mut t,
        "/api/yonetim/birim/ekle",
        json!({"ad": "B", "tur": "yok"}),
        400,
    );
    kotu(
        &mut t,
        "/api/yonetim/birim/ekle",
        json!({"ad": "B", "tur": "sube"}),
        422,
    ); // sube_kodu gerekli
    kotu(
        &mut t,
        "/api/yonetim/birim/ekle",
        json!({"ad": "B", "tur": "departman", "ust_birim_id": "olmayan"}),
        422,
    );
    kotu(
        &mut t,
        "/api/yonetim/gorevlendirme/ekle",
        json!({"kisi_id": "olmayan", "birim_id": "ornek-sube-1", "rol": "uzman"}),
        422,
    );
    kotu(
        &mut t,
        "/api/yonetim/gorevlendirme/ekle",
        json!({"kisi_id": "ornek-kisi-ayse", "birim_id": "ornek-sube-1", "rol": "olmayan-rol"}),
        422,
    );
    kotu(
        &mut t,
        "/api/yonetim/gorevlendirme/ekle",
        json!({"kisi_id": "ornek-kisi-ayse", "birim_id": "ornek-sube-1", "rol": "uzman", "tur": "x"}),
        400,
    );
    kotu(
        &mut t,
        "/api/yonetim/gorevlendirme/ekle",
        json!({"kisi_id": "ornek-kisi-ayse", "birim_id": "ornek-sube-1", "rol": "uzman", "baslangic": "2026-13-45"}),
        422,
    );
    kotu(
        &mut t,
        "/api/yonetim/kisi/duzelt",
        json!({"id": "yok", "ad": "A", "eposta": "a@b.co", "beklenen_surum": 1}),
        404,
    );
    kotu(
        &mut t,
        "/api/yonetim/kisi/duzelt",
        json!({"id": "ornek-kisi-ayse", "ad": "A", "eposta": "a@b.co"}),
        400,
    );
    // Hiçbiri kaydı bozmadı.
    assert!(t.d.depo.veri().dogrula().is_ok());
}

#[test]
fn surum_cakismasi_409() {
    let mut t = girisli(&[IZIN_YAZ_]);
    let g = |s: u64, ad: &str| json!({"id": "ornek-kisi-ayse", "ad": ad, "eposta": "ayse.sube1@ornek.test", "beklenen_surum": s});
    let (k, v) = gonder(&mut t, "/api/yonetim/kisi/duzelt", g(1, "ÖRNEK Ayşe Yeni"));
    assert_eq!(k, 200, "{v}");
    assert_eq!(v["surum"], 2);
    // İkinci yönetici eski sürümle gelir: 409, güncel sürüm bildirilir, kayıt değişmez.
    let (k, v) = gonder(&mut t, "/api/yonetim/kisi/duzelt", g(1, "ÖRNEK Ayşe Eski"));
    assert_eq!(k, 409, "{v}");
    assert_eq!(v["kod"], "cakisma");
    assert_eq!(v["guncel_surum"], 2);
    assert_eq!(v["beklenen_surum"], 1);
    assert_eq!(
        t.d.depo.veri().kisiler["ornek-kisi-ayse"].ad,
        "ÖRNEK Ayşe Yeni"
    );
    // Doğru sürümle geçer.
    assert_eq!(
        gonder(&mut t, "/api/yonetim/kisi/duzelt", g(2, "ÖRNEK Ayşe Üç")).0,
        200
    );
    // Birim için de aynı.
    let b = |s: u64| json!({"id": "ornek-magaza-1", "ad": "ÖRNEK Mağaza 1b", "beklenen_surum": s});
    assert_eq!(gonder(&mut t, "/api/yonetim/birim/duzelt", b(1)).0, 200);
    let (k, v) = gonder(&mut t, "/api/yonetim/birim/duzelt", b(1));
    assert_eq!((k, v["kod"].as_str()), (409, Some("cakisma")));
    // Sayı olmayan sürüm 400.
    assert_eq!(
        gonder(
            &mut t,
            "/api/yonetim/birim/duzelt",
            json!({"id": "ornek-magaza-1", "ad": "x", "beklenen_surum": "1"})
        )
        .0,
        400
    );
}

#[test]
fn tekrar_eden_kayitlar_409() {
    let mut t = girisli(&[IZIN_YAZ_]);
    // Etkin kişiler arasında e-posta benzersiz.
    let (k, v) = gonder(
        &mut t,
        "/api/yonetim/kisi/ekle",
        json!({"ad": "Kopya", "eposta": "AYSE.sube1@ornek.test"}),
    );
    assert_eq!((k, v["kod"].as_str()), (409, Some("tekrar")), "{v}");
    assert!(
        !v["hata"].as_str().unwrap().contains("ayse.sube1"),
        "e-posta maskeli"
    );
    // Aynı kişiye ikinci etkin asıl görevlendirme.
    let (k, v) = gonder(
        &mut t,
        "/api/yonetim/gorevlendirme/ekle",
        json!({
            "kisi_id": "ornek-kisi-ayse", "birim_id": "ornek-magaza-1", "rol": "uzman", "tur": "asil"
        }),
    );
    assert_eq!(k, 409, "{v}");
    // Geçici görevlendirme ise olur.
    let (k, v) = gonder(
        &mut t,
        "/api/yonetim/gorevlendirme/ekle",
        json!({
            "kisi_id": "ornek-kisi-ayse", "birim_id": "ornek-magaza-1", "rol": "uzman", "tur": "gecici", "bitis": "2026-12-31"
        }),
    );
    assert_eq!(k, 201, "{v}");
}

#[test]
fn yonetici_okuma_izinleri_kapsami_yalniz_izinle_genisletir() {
    // Yalnız yazma izni: okuma kapsamı cihazın şubesi olarak kalır.
    let mut t = girisli(&[IZIN_YAZ_]);
    let (_, l) = al(&mut t, "/api/kisiler", "");
    assert!(!adlar(&l).contains(&"ÖRNEK Pasif Kişi".to_string()));
    assert!(!adlar(&l).contains(&"ÖRNEK Can Şube2".to_string()));
    assert_eq!(al(&mut t, "/api/durum", "").1["kapsam"]["tum_oku"], false);
    // rehber.tum.oku ile tam liste (pasif dahil) ve hiyerarşi ağacı.
    let mut t = girisli(&[IZIN_YAZ_, "rehber.tum.oku"]);
    let (_, l) = al(&mut t, "/api/kisiler", "");
    assert!(adlar(&l).contains(&"ÖRNEK Pasif Kişi".to_string()));
    assert!(adlar(&l).contains(&"ÖRNEK Can Şube2".to_string()));
    let (_, a) = al(&mut t, "/api/agac", "");
    assert!(
        a["dugumler"]
            .as_array()
            .unwrap()
            .iter()
            .any(|n| n["grup"] == false),
        "hiyerarşi (iç içe) ağaç: {a}"
    );
}

// ------------------------------------------------------------ okuma uçları ve şube süzgeci

#[test]
fn sube_suzgeci_sunucuda_kapsami_asamaz() {
    let mut t = kur(true, Some(demo::SUBE_1), Box::new(KimlikYok));
    let (_, s) = al(&mut t, "/api/suzgecler", "");
    let subeler = s["subeler"].as_array().unwrap();
    assert!(
        subeler.iter().any(|x| x["id"] == "ornek-sube-2"),
        "şubelerin adı ortak listede görünür (§5.3)"
    );
    // Başka şube süzgeci yalnız daraltır: S2'nin sıradan çalışanı gelmez.
    let (k, l) = al(&mut t, "/api/kisiler", "sube=ornek-sube-2");
    assert_eq!(k, 200);
    assert!(!adlar(&l).contains(&"ÖRNEK Can Şube2".to_string()), "{l}");
    // İstemcinin uydurduğu sorgu parametreleri (kapsam, tum, sube_kodu) yok sayılır.
    for sorgu in [
        "tum=1",
        "tum_oku=true",
        "sube_kodu=ORNEK-S2",
        "kapsam=tum",
        "amir_oku=1",
    ] {
        let (k, l) = al(&mut t, "/api/kisiler", sorgu);
        assert_eq!(k, 200);
        assert!(
            !adlar(&l).contains(&"ÖRNEK Can Şube2".to_string()),
            "{sorgu}: {l}"
        );
        assert!(
            !adlar(&l).contains(&"ÖRNEK Pasif Kişi".to_string()),
            "{sorgu}"
        );
        assert!(
            l["kisiler"]
                .as_array()
                .unwrap()
                .iter()
                .all(|x| x["gorevler"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|g| g["amir_ad"].is_null())),
            "{sorgu}"
        );
    }
    // Bilinmeyen değer 400.
    assert_eq!(al(&mut t, "/api/kisiler", "sube=yok").0, 400);
    assert_eq!(al(&mut t, "/api/kisiler", "gorev=yok").0, 400);
    assert_eq!(al(&mut t, "/api/kisiler", "birim=yok").0, 400);
    // Kendi şubesi süzgeci kendi kişilerini getirir; görev süzgeci birlikte çalışır.
    let (_, l) = al(
        &mut t,
        "/api/kisiler",
        "sube=ornek-sube-1&gorev=kasa-sorumlusu",
    );
    assert_eq!(adlar(&l), vec!["ÖRNEK Ayşe Şube1".to_string()]);
    // Türkçe arama (yüzde kodlu).
    let (_, l) = al(&mut t, "/api/kisiler", "ara=%C5%9Eube1");
    assert!(adlar(&l).contains(&"ÖRNEK Ayşe Şube1".to_string()), "{l}");
    // Çok uzun arama reddedilir.
    assert_eq!(
        al(&mut t, "/api/kisiler", &format!("ara={}", "a".repeat(300))).0,
        400
    );
}

#[test]
fn kapsam_yoksa_yalniz_merkez_ve_bos_depo() {
    let mut t = kur(true, None, Box::new(KimlikYok));
    // --demo ama --sube yok: demo şubesi atanır (ilk örnek şube).
    assert_eq!(
        al(&mut t, "/api/durum", "").1["kapsam"]["sube_kodu"],
        demo::SUBE_1
    );
    // Demo olmayan kurulum: veri yok, kapsam yok, boş liste, dürüst durum.
    let mut b = kur(false, None, Box::new(KimlikYok));
    let (_, d) = al(&mut b, "/api/durum", "");
    assert_eq!(
        (d["demo"].clone(), d["veri_kaynagi"].clone()),
        (json!(false), json!("yok"))
    );
    let (k, l) = al(&mut b, "/api/kisiler", "");
    assert_eq!(k, 200);
    assert_eq!(l["toplam"], 0);
    assert_eq!(al(&mut b, "/api/agac", "").1["dugumler"], json!([]));
    assert_eq!(al(&mut b, "/api/el-kitabi", "").1["konular"], json!([]));
    assert_eq!(al(&mut b, "/api/durum", "").1["yazma"]["kalici"], false);
}

#[test]
fn demo_olmayan_kurulumda_gercek_ornek_veri_yok() {
    let b = kur(false, Some("X"), Box::new(KimlikYok));
    assert!(b.d.depo.veri().kisiler.is_empty());
    assert!(b.d.belgeler.konular().is_empty());
}

#[test]
fn el_kitabi_liste_oku_ve_surumler() {
    let mut t = kur(true, Some(demo::SUBE_1), Box::new(KimlikYok));
    let (k, l) = al(&mut t, "/api/el-kitabi", "");
    assert_eq!(k, 200);
    assert_eq!(l["sema"], "rehber.belge@1");
    let konular = l["konular"].as_array().unwrap();
    assert_eq!(konular.len(), 2);
    let b = konular
        .iter()
        .find(|x| x["konu_id"] == "ornek-baslangic")
        .unwrap();
    assert_eq!(b["surum"], 2);
    let (k, o) = al(&mut t, "/api/el-kitabi/oku", "konu=ornek-baslangic");
    assert_eq!(k, 200);
    assert_eq!(o["surum"], 2);
    assert_eq!(o["surumler"], json!([1, 2]));
    assert_eq!(o["bloklar"][0]["tur"], "baslik");
    let (_, eski) = al(&mut t, "/api/el-kitabi/oku", "konu=ornek-baslangic&surum=1");
    assert_eq!(eski["surum"], 1);
    assert_eq!(al(&mut t, "/api/el-kitabi/oku", "konu=yok").0, 404);
    assert_eq!(
        al(&mut t, "/api/el-kitabi/oku", "konu=ornek-baslangic&surum=9").0,
        404
    );
    assert_eq!(
        al(&mut t, "/api/el-kitabi/oku", "konu=ornek-baslangic&surum=x").0,
        400
    );
    assert_eq!(al(&mut t, "/api/el-kitabi/oku", "").0, 400);
    // Okuma yanıtında ham gövde (HTML olabilecek metin) yok, yalnız bloklar.
    assert!(o.get("govde").is_none());
}

// ------------------------------------------------------------ ihbar

fn ihbar_defteri(t: &Ortamlar) -> String {
    std::fs::read_to_string(&t.d.ihbar.defter_yolu).unwrap_or_default()
}

#[test]
fn ihbar_varsayilan_anonim_ve_iletisim_yok_sayilir() {
    let mut t = kur(true, None, Box::new(KimlikYok));
    let (k, v) = gonder(
        &mut t,
        "/api/ihbar",
        json!({
            "kategori": "usulsuzluk", "metin": "Örnek ihbar metni", "iletisim": "gizli.kisi@ornek.test"
        }),
    );
    assert_eq!(k, 201, "{v}");
    assert_eq!(v["anonim"], true, "anonim varsayılan");
    assert_eq!(v["iletildi"], false);
    assert!(v["takip_no"].as_str().unwrap().starts_with("IH-"));
    let m = v["mesaj"].as_str().unwrap();
    assert!(m.contains("henüz bağlı değil"), "dürüst durum: {m}");
    let d = ihbar_defteri(&t);
    assert!(d.contains("Örnek ihbar metni"));
    assert!(
        !d.contains("gizli.kisi"),
        "anonimken iletişim saklanmaz: {d}"
    );
    // Anonim kapalıyken iletişim kaydedilir (isteğe bağlı).
    let (k, v) = gonder(
        &mut t,
        "/api/ihbar",
        json!({
            "kategori": "diger", "metin": "ikinci", "anonim": false, "iletisim": "kisi@ornek.test"
        }),
    );
    assert_eq!((k, v["anonim"].clone()), (201, json!(false)), "{v}");
    assert!(ihbar_defteri(&t).contains("kisi@ornek.test"));
    // İletişimsiz ve anonim kapalı: yine geçerli (iletişim isteğe bağlı).
    assert_eq!(
        gonder(
            &mut t,
            "/api/ihbar",
            json!({"kategori": "diger", "metin": "üçüncü", "anonim": false})
        )
        .0,
        201
    );
}

#[test]
fn ihbar_dogrulama_ve_hiz_siniri() {
    let mut t = kur(true, None, Box::new(KimlikYok));
    let (k, v) = gonder(
        &mut t,
        "/api/ihbar",
        json!({"kategori": "yok", "metin": "m"}),
    );
    assert_eq!((k, v["alan"].as_str()), (400, Some("kategori")));
    assert_eq!(
        gonder(&mut t, "/api/ihbar", json!({"kategori": "diger"})).0,
        400
    );
    assert_eq!(
        gonder(
            &mut t,
            "/api/ihbar",
            json!({"kategori": "diger", "metin": "   "})
        )
        .0,
        400
    );
    assert_eq!(
        gonder(
            &mut t,
            "/api/ihbar",
            json!({"kategori": "diger", "metin": "x".repeat(4001)})
        )
        .0,
        400
    );
    assert_eq!(ihbar_defteri(&t), "", "geçersiz girdi deftere girmez");
    // Varsayılan hız sınırı: kaynak başına 3. taa-ihbar sırası gereği geçersiz metin de belirteç harcar
    // (yukarıdaki boş ve aşırı uzun metin iki belirteç aldı); kategorisi geçersiz istekler harcamaz.
    for i in 0..1 {
        assert_eq!(
            gonder(
                &mut t,
                "/api/ihbar",
                json!({"kategori": "diger", "metin": format!("m{i}")})
            )
            .0,
            201
        );
    }
    let (k, v) = gonder(
        &mut t,
        "/api/ihbar",
        json!({"kategori": "diger", "metin": "dördüncü"}),
    );
    assert_eq!(k, 429, "{v}");
    assert!(v["bekle_sn"].as_u64().unwrap() > 0);
    // Süre geçince yeniden denenebilir.
    t.o.ilerle(3601);
    assert_eq!(
        gonder(
            &mut t,
            "/api/ihbar",
            json!({"kategori": "diger", "metin": "sonra"})
        )
        .0,
        201
    );
}

#[test]
fn ihbar_kategorileri() {
    let mut t = kur(true, None, Box::new(KimlikYok));
    let (k, v) = al(&mut t, "/api/ihbar/kategoriler", "");
    assert_eq!(k, 200);
    assert_eq!(v["kategoriler"].as_array().unwrap().len(), 5);
    assert_eq!(v["iletim_bagli"], false);
}

#[test]
fn ihbar_ayarli_iletim_bagli_degil_durumu_ve_sir_sizmaz() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    std::fs::create_dir_all(kok.join("ihbar")).unwrap();
    std::fs::write(
        kok.join("ihbar").join("ihbar.json"),
        r#"{"telegram_bot_anahtari":"BOT-SIRRI-9","telegram_sohbet":"555","eposta_alici":"alici@ornek.test","eposta_gonderen":"bot@ornek.test"}"#,
    )
    .unwrap();
    let mut t = kur_kokte(kok, true, None, Box::new(KimlikYok));
    assert_eq!(al(&mut t, "/api/durum", "").1["ihbar"]["ayarli"], true);
    let (k, v) = gonder(
        &mut t,
        "/api/ihbar",
        json!({"kategori": "taciz", "metin": "ayarlı hat"}),
    );
    assert_eq!(k, 201, "{v}");
    assert_eq!(v["iletildi"], false);
    assert_eq!(
        (v["telegram"].as_str(), v["eposta"].as_str()),
        (Some("hata"), Some("hata"))
    );
    assert!(v["mesaj"].as_str().unwrap().contains("henüz bağlı değil"));
    let d = ihbar_defteri(&t);
    assert!(d.contains("ayarlı hat"), "ihbar kaybolmaz");
    for sir in ["BOT-SIRRI-9", "555", "alici@ornek.test"] {
        assert!(
            !d.contains(sir) && !v.to_string().contains(sir),
            "{sir} sızdı"
        );
    }
    assert!(
        d.contains("gönderim henüz bağlı değil"),
        "teslimat durumu deftere yazılır"
    );
}

// ------------------------------------------------------------ görünüm, durum, yönlendirme

#[test]
fn tema_kalici_ve_gecersiz_reddedilir() {
    let mut t = kur(true, None, Box::new(KimlikYok));
    assert_eq!(al(&mut t, "/api/gorunum", "").1["tema"], "acik");
    let (k, v) = gonder(&mut t, "/api/gorunum", json!({"tema": "koyu"}));
    assert_eq!((k, v["kalici"].clone()), (200, json!(true)));
    assert_eq!(al(&mut t, "/api/durum", "").1["tema"], "koyu");
    assert_eq!(
        gonder(&mut t, "/api/gorunum", json!({"tema": "mor"})).0,
        400
    );
    assert_eq!(gonder(&mut t, "/api/gorunum", json!({})).0, 400);
    // Yeni açılış (aynı kök) kayıtlı temayı okur.
    let tema = crate::gorunum::yukle(&t.d.gorunum_yolu);
    assert_eq!(tema.kod(), "koyu");
    assert!(arayuz_html(tema).contains("data-theme=\"koyu\""));
}

#[test]
fn bilinmeyen_yollar_404() {
    let mut t = kur(true, None, Box::new(KimlikYok));
    assert_eq!(al(&mut t, "/api/yok", "").0, 404);
    assert_eq!(al(&mut t, "/disari", "").0, 404);
    // Yanlış yöntem.
    assert_eq!(gonder(&mut t, "/api/kisiler", json!({})).0, 404);
    assert_eq!(al(&mut t, "/api/ihbar", "").0, 404);
}

#[test]
fn sorgu_cozucu() {
    let q = sorgu_coz("ara=M%C3%BCd%C3%BCr+Bey&sube=a%20b&bos=&yalniz&bozuk=%ZZ&utf=%FF");
    assert_eq!(q["ara"], "Müdür Bey");
    assert_eq!(q["sube"], "a b");
    assert_eq!(q["bos"], "");
    assert_eq!(q["yalniz"], "");
    assert!(!q.contains_key("bozuk"), "geçersiz yüzde kodu atlanır");
    assert!(!q.contains_key("utf"), "geçersiz UTF-8 atlanır");
    assert!(sorgu_coz("").is_empty());
}

// ------------------------------------------------------------ gömülü ekran

#[test]
fn ekran_tek_dosya_dis_kaynak_yok() {
    let h = ARAYUZ_HTML;
    assert!(h.contains("__TEMA__"));
    for yasak in [
        "src=\"http",
        "href=\"http",
        "src='http",
        "@import",
        "url(http",
        "url(//",
        "<script src",
        "fonts.googleapis",
        "cdn.",
    ] {
        assert!(!h.contains(yasak), "dış kaynak izi: {yasak}");
    }
    assert!(!h.contains("innerHTML"), "veri HTML olarak yazılmaz");
    assert!(h.contains("<title>TAA Rehber</title>"));
    assert!(h.contains("İçeriğe geç"));
    assert!(h.contains("aria-live=\"polite\""));
    assert!(h.contains("prefers-reduced-motion"));
    for tema in ["acik", "koyu", "tek"] {
        assert!(arayuz_html(crate::gorunum::Tema::koddan(tema).unwrap())
            .contains(&format!("data-theme=\"{tema}\"")));
    }
    assert!(SIMGE_SVG.starts_with("<svg"));
}

#[test]
fn hesap_menusu_sirasi_kullanici_cizgi_yonetici_cizgi_ayarlar() {
    let h = ARAYUZ_HTML;
    let menu = &h[h.find("id=\"hesap-menu\"").unwrap()..];
    let menu = &menu[..menu.find("</div>").unwrap()];
    let konum = |s: &str| menu.find(s).unwrap_or_else(|| panic!("menüde yok: {s}"));
    let kullanici = konum("id=\"hesap-ad\"");
    let c1 = menu.find("<hr").unwrap();
    let yon = konum("id=\"yonetici-dugme\"");
    let c2 = menu[c1 + 3..].find("<hr").unwrap() + c1 + 3;
    let ayar = konum("id=\"tema-ac\"");
    assert!(
        kullanici < c1 && c1 < yon && yon < c2 && c2 < ayar,
        "{kullanici} {c1} {yon} {c2} {ayar}"
    );
}
