// Sahip: Yusuf. yazan: [Claude#taa-kasa-T6b 2026-11-26]
//! Kasa Çift Doğrulama Tutanağı (KCD). Yusuf'un kararıyla (2026-11-26) tutanak ve raporlaması Tutaa'dan
//! TAA Kasa'ya taşındı: şablon `sablonlar/kasa-cift-dogrulama.json` (Tutaa'dan birebir), tutanak gün sonu
//! verisinden kendiliğinden dolar, A4 yazdırılabilir HTML olarak üretilir, imzalar TAA Kasa imza kurallarıyla.
//! Tutaa karşılıkları: şablon motoru (`sablon.js`: fark = fiili − erp, |fark| > 0,004 satırda gerekçe) →
//! `cift_dogrulama`; `paket.js` farktan sonucu seçme → `sonuc_sec`; `kasa-excel.js` Nebim Excel içe aktarma →
//! `xlsx_oku` + `sutun_oner` + `eslestir` + `sayi_oku`.
//!
//! Eşleme kuralı (ERP = Nebim sistem tutarı, fiili = sayım / cihaz gün sonu):
//! - Nakit (TL): ERP = NAKİT TL (masraf ve para üstü düşülmüş sistem nakdi); fiili = sayılan − açılış.
//!   Fark = kasa fazlası (+) / açığı (−).
//! - Ziraat Bankası POS: ERP = ZİRAAT BANKASI POS:1; fiili = Ziraat POS gün sonu.
//! - İyzico Fiziksel POS: ERP = İYZİCO POS:2 (+ eski kayıtlarda E-FATURA POS:2; satır kaldırıldı); fiili = bu
//!   cihazların gün sonu. QR ve İyzico cihazındaki tüm banka kartları buradadır.
//! - İyzico Sanal POS: ERP = İYZİCO ÖDEMESİ (link / sanal ödeme). TAA Kasa'da ayrı sayım yok; fiili = ERP.
//! - Havale / EFT: ERP = BANKAYA HAVALE (firma hesabı); fiili = havale + HAVALE +/-.
//! - Kredi Çeki: ERP = KREDİ ÇEKİ + mağaza değişimi (DEĞİŞİM − İNTERNET DEĞİŞİMİ). Nebim "Kredi Çeki"
//!   ödeme tipi değişim çekidir (25.11: Nebim kredi çeki = DEĞİŞİM). Fiili = ERP (belge ile).
//! - E-ticaret Değişimi: ERP = İNTERNET DEĞİŞİMİ; fiili = ERP.
//! - Diğer: döviz nakdin TL karşılığı (sayılır; Nebim'de TL dışı nakit). ERP verilmezse fiili = ERP.
//!
//! Hareket özetinden (Nebim) gelen tutar verilirse ERP o tutardır; verilmezse gün sonundaki ciro satırı
//! kullanılır. Fark = fiili − ERP (kuruş); farkı olan satırda gerekçe gün sonu notudur.

use crate::gunsonu::{
    imza_listesi, DovizSatiri, GunSonuKaydi, ImzaKaydi, KasaAyar, IKINCI_IMZA_ATMADI,
    KAPANIS_LISTESI,
};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::io::Read;

/// Tutaa'dan alınan şablon (kasa-cift-dogrulama.json, sürüm 2).
pub const SABLON_JSON: &str = include_str!("../sablonlar/kasa-cift-dogrulama.json");

/// Şablondan okunan tutanak metinleri.
#[derive(Debug, Clone)]
pub struct Sablon {
    pub baslik: String,
    pub dipnot: String,
    pub onek: String,
    pub kontroller: Vec<String>,
    pub kalemler: Vec<String>,
    pub roller: Vec<String>,
    pub sonuclar: Vec<String>,
}

pub fn sablon() -> Sablon {
    let v: Value = serde_json::from_str(SABLON_JSON).unwrap_or(Value::Null);
    let liste = |x: &Value| -> Vec<String> {
        x.as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|y| y.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default()
    };
    let alan = |id: &str| -> Value {
        v["bolumler"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|b| b["alanlar"].as_array().cloned().unwrap_or_default())
            .find(|a| a["id"] == id)
            .unwrap_or(Value::Null)
    };
    Sablon {
        baslik: v["baslik"]
            .as_str()
            .unwrap_or("KASA ÇİFT DOĞRULAMA TUTANAĞI")
            .into(),
        dipnot: v["hukuki"]["dipnot"].as_str().unwrap_or("").into(),
        onek: v["numara"]["onek"].as_str().unwrap_or("KCD").into(),
        kontroller: liste(&alan("kontroller")["secenekler"]),
        kalemler: liste(&v["tablo"]["sabitSatirlar"]),
        roller: v["imzalar"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|i| i["rol"].as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default(),
        sonuclar: liste(&alan("sonuc")["secenekler"]),
    }
}

/// Tutanak numarası: KCD-yıl-<gün sonu kaydı>, ör. KCD-2026-20261125-c1-1 (gün sonu başına tekil).
pub fn numara(gs: &GunSonuKaydi) -> String {
    format!(
        "{}-{}-{}",
        sablon().onek,
        gs.tarih.get(..4).unwrap_or("0000"),
        gs.id.trim_start_matches("GS-")
    )
}

/// Tutaa `paket.js` karşılığı: herhangi bir satırda |fark| > 0,004 TL (≥ 1 kuruş) ise "Fark tespit edilmiştir."
pub fn sonuc_sec(farklar: &[i64]) -> &'static str {
    if farklar.iter().any(|f| *f != 0) {
        SONUC_FARK_VAR
    } else {
        SONUC_FARK_YOK
    }
}

/// "Yapılan kontroller" kutuları gün sonundan kendiliğinden işaretlenir:
/// 1 ERP ↔ fiili: gün sonu karşılaştırması yapıldı (her zaman); 2 Z / POS / Nebim: kapanış listesi 2. madde;
/// 3 belgeler: kapanış listesi "Masraf fişleri ayrıldı"; 4 iki yetkili birlikte saydı: iki imza var;
/// 5 havale / sanal POS ekstreyle: HAVALE +/- girildi ya da sıfır doğrulandı (havale satırı varsa) — TAA Kasa'da
/// ayrı ekstre eşleştirmesi yok, bu yüzden yalnız havale satırı boşsa da işaretlenmez.
pub fn kontroller(gs: &GunSonuKaydi, imzalar: &[ImzaKaydi]) -> Vec<bool> {
    let k = |i: usize| gs.kontrol.get(i).copied().unwrap_or(false);
    let fis = KAPANIS_LISTESI
        .iter()
        .position(|m| m.starts_with("Masraf fişleri"))
        .map(k)
        .unwrap_or(false);
    vec![true, k(1), fis, imzalar.len() >= 2, false]
}

/// Tutaa KCD şablonunun 8 sabit satırı (sabit kod, Tutaa'daki ad), Tutaa'daki sırayla.
pub const TUTAA_KALEMLERI: [(&str, &str); 8] = [
    ("nakit_tl", "Nakit (TL)"),
    ("ziraat_pos", "Ziraat Bankası POS"),
    ("iyzico_fiziksel_pos", "İyzico Fiziksel POS"),
    ("iyzico_sanal_pos", "İyzico Sanal POS"),
    ("havale_eft", "Havale / EFT"),
    ("kredi_ceki", "Kredi Çeki"),
    ("eticaret_degisimi", "E-ticaret Değişimi"),
    ("diger", "Diğer"),
];

pub const SONUC_FARK_YOK: &str = "Fark yoktur.";
pub const SONUC_FARK_VAR: &str = "Fark tespit edilmiştir.";

/// Bir gün sonunun POS gün sonu toplamları, ödeme türüne göre (`ziraat_pos`, `iyzico_pos`, ...).
fn pos_toplamlari(ayar: &KasaAyar, gs: &GunSonuKaydi) -> BTreeMap<String, i64> {
    let mut m = BTreeMap::new();
    if let Some(c) = ayar.cekmece(&gs.cekmece) {
        for n in &c.noktalar {
            let Some(ng) = gs.noktalar.iter().find(|x| x.nokta == n.id) else {
                continue;
            };
            for p in &n.poslar {
                if let Some(t) = ng.pos.get(&p.id) {
                    *m.entry(p.odeme_turu.clone()).or_insert(0) += t;
                }
            }
        }
    }
    m
}

/// Satır başına (ERP, fiili, fiili kaynağı) — hareket özeti ERP'si uygulanmadan önce.
pub fn kalem_tutarlari(
    ayar: &KasaAyar,
    gs: &GunSonuKaydi,
    acilis_kurus: i64,
) -> Vec<(&'static str, i64, i64, &'static str)> {
    let c = |k: &str| gs.ciro.get(k).copied().unwrap_or(0);
    let pos = pos_toplamlari(ayar, gs);
    let p = |k: &str| pos.get(k).copied().unwrap_or(0);
    let doviz: i64 = gs.doviz.iter().map(DovizSatiri::tl_kurus).sum();
    let ceki = c("kredi_ceki") + c("degisim") - c("internet_degisimi");
    vec![
        (
            "nakit_tl",
            c("nakit"),
            gs.nakit_sayilan_kurus - acilis_kurus,
            "sayim",
        ),
        (
            "ziraat_pos",
            c("ziraat_pos"),
            p("ziraat_pos"),
            "pos_gun_sonu",
        ),
        (
            "iyzico_fiziksel_pos",
            c("iyzico_pos") + c("efatura_pos"),
            p("iyzico_pos") + p("efatura_pos"),
            "pos_gun_sonu",
        ),
        (
            "iyzico_sanal_pos",
            c("iyzico_odemesi"),
            c("iyzico_odemesi"),
            "sistem",
        ),
        (
            "havale_eft",
            c("havale"),
            c("havale") + c("havale_fark"),
            "banka_hesabi",
        ),
        ("kredi_ceki", ceki, ceki, "belge"),
        (
            "eticaret_degisimi",
            c("internet_degisimi"),
            c("internet_degisimi"),
            "belge",
        ),
        ("diger", doviz, doviz, "sayim"),
    ]
}

/// Tutaa KCD okuma görünümü. `erp`: hareket özetinden (Nebim) gelen satır tutarları (Tutaa kalem koduyla);
/// verilmeyen satırda gün sonundaki ciro değeri kullanılır.
pub fn cift_dogrulama(
    ayar: &KasaAyar,
    gs: &GunSonuKaydi,
    acilis_kurus: i64,
    imzalar: &[ImzaKaydi],
    erp: Option<&BTreeMap<String, i64>>,
) -> Value {
    cift_dogrulama_islem(ayar, gs, acilis_kurus, imzalar, erp, "")
}

/// `islem`: "Yapılan İşlem" alanı (kamera incelemesi / müşteri ile iletişim / tahsilat / tutanak).
pub fn cift_dogrulama_islem(
    ayar: &KasaAyar,
    gs: &GunSonuKaydi,
    acilis_kurus: i64,
    imzalar: &[ImzaKaydi],
    erp: Option<&BTreeMap<String, i64>>,
    islem: &str,
) -> Value {
    let mut kalemler = Vec::new();
    let mut toplam = 0i64;
    let mut mutlak = 0i64;
    let mut farkli: Vec<&str> = Vec::new();
    for (kod, e, fiili, kaynak) in kalem_tutarlari(ayar, gs, acilis_kurus) {
        let ad = TUTAA_KALEMLERI
            .iter()
            .find(|(k, _)| *k == kod)
            .map(|(_, a)| *a)
            .unwrap_or(kod);
        let e = erp.and_then(|m| m.get(kod)).copied().unwrap_or(e);
        // ERP verildiyse ve fiili sistemden geliyorsa fiili de ERP değil gün sonu değeridir (fark görünür).
        let fark = fiili - e;
        toplam += fark;
        mutlak += fark.abs();
        if fark != 0 {
            farkli.push(ad);
        }
        kalemler.push(json!({
            "kod": kod, "ad": ad, "erp_kurus": e, "fiili_kurus": fiili, "fark_kurus": fark,
            "gerekce": if fark != 0 { gs.not.as_str() } else { "" },
            "fiili_kaynagi": kaynak,
        }));
    }
    let sb = sablon();
    let kont = kontroller(gs, imzalar);
    let farklar: Vec<i64> = kalemler
        .iter()
        .filter_map(|k| k["fark_kurus"].as_i64())
        .collect();
    json!({
        "sema": "taa-kasa.cift_dogrulama@1",
        "numara": numara(gs),
        "baslik": sb.baslik,
        "sube": gs.sube,
        "tarih": gs.tarih,
        "cekmece": gs.cekmece,
        "gun_sonu_id": gs.id,
        "kalemler": kalemler,
        "toplam_fark_kurus": toplam,
        "fark_tutar_kurus": mutlak,
        "fark_kalem": farkli.join(", "),
        "neden": if farkli.is_empty() { "" } else { gs.not.as_str() },
        "sonuc": sonuc_sec(&farklar),
        "islem": islem,
        "kontroller": sb.kontroller.iter().zip(kont.iter().chain(std::iter::repeat(&false)))
            .map(|(m, y)| json!({"metin": m, "yapildi": y})).collect::<Vec<_>>(),
        "imzalar": imza_listesi(imzalar),
    })
}

// ---------------------------------------------------------------- A4 tutanak (yazdırılabilir HTML)

fn kac(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn tutar(k: i64) -> String {
    crate::kurus_metni(k).trim_end_matches(" TL").to_string()
}

/// A4 tutanak (Tutaa genel şablon çıktısına benzer düzen: başlık, numara ve tarih-saat sağ üstte, numaralı
/// bölümler, kalem tablosu ve toplam, kontroller, fark detayı, dipnot, iki imza kutusu). Hiç imza yoksa
/// "TASLAK" yazar; atılmamış ikinci imza kutusunda "İKİNCİ İMZA ATMADI" yazar. Yazdırma tarayıcıdan.
pub fn tutanak_html(cd: &Value, sube_adi: &str, uretim: &str) -> String {
    let sb = sablon();
    let mut h = String::new();
    let taslak = cd["imzalar"]
        .as_array()
        .is_none_or(|a| a.iter().all(|i| i["imzalamadi"] == true));
    h.push_str(r#"<!doctype html><html lang="tr"><head><meta charset="utf-8"><title>"#);
    h.push_str(&kac(cd["numara"].as_str().unwrap_or("")));
    h.push_str(r#"</title><style>
@page{size:A4;margin:16mm 14mm}*{box-sizing:border-box}body{font:11pt/1.4 "Segoe UI",Arial,sans-serif;color:#111;margin:0}
.sayfa{max-width:182mm;margin:0 auto;padding:8px}.ust{display:flex;justify-content:space-between;align-items:flex-start;font-size:9.5pt}
h1{text-align:center;font-size:15pt;letter-spacing:.04em;margin:10px 0 14px}h2{font-size:11pt;margin:14px 0 6px;border-bottom:1px solid #999}
table{width:100%;border-collapse:collapse;font-size:10pt}th,td{border:1px solid #666;padding:4px 6px;vertical-align:top}th{background:#eee}
td.s{text-align:right;font-variant-numeric:tabular-nums;white-space:nowrap}tfoot td{font-weight:700}.kutu{font-family:monospace}
.dipnot{font-size:9pt;color:#333;margin-top:12px;text-align:justify}.imzalar{display:grid;grid-template-columns:1fr 1fr;gap:14px;margin-top:16px}
.imza{border:1px solid #666;padding:8px;min-height:34mm}.imza b{display:block}.yok{color:#a00;font-weight:700}.gerekce{margin-top:4px;font-size:9pt}.taslak{color:#a00;font-weight:700;text-align:center}
@media print{.yazdir{display:none}}</style></head><body><div class="sayfa">"#);
    h.push_str(&format!(
        r#"<div class="ust"><div>{}</div><div style="text-align:right">No: <b>{}</b><br>Tarih: {}<br>Düzenlenme: {}</div></div>"#,
        kac(sube_adi),
        kac(cd["numara"].as_str().unwrap_or("")),
        kac(&crate::sablon::tarih_tr(cd["tarih"].as_str().unwrap_or(""))),
        kac(uretim)
    ));
    h.push_str(&format!("<h1>{}</h1>", kac(&sb.baslik)));
    if taslak {
        h.push_str(r#"<p class="taslak">TASLAK — imza bekleniyor (en az bir imza zorunlu)</p>"#);
    }
    h.push_str(&format!(
        "<h2>1. Şube Bilgisi</h2><table><tr><th style=\"width:35%\">Şube Adı</th><td>{}</td></tr><tr><th>Çekmece / gün sonu</th><td>{} · {}</td></tr></table>",
        kac(sube_adi),
        kac(cd["cekmece"].as_str().unwrap_or("")),
        kac(cd["gun_sonu_id"].as_str().unwrap_or(""))
    ));
    h.push_str("<h2>2. Kalem Bazında Doğrulama</h2><table><thead><tr><th>Kalem</th><th>ERP (Nebim) Bakiyesi</th><th>Fiili Sayım</th><th>Fark (+/-)</th><th>Fark gerekçesi</th></tr></thead><tbody>");
    let (mut te, mut tf) = (0i64, 0i64);
    for k in cd["kalemler"].as_array().into_iter().flatten() {
        let e = k["erp_kurus"].as_i64().unwrap_or(0);
        let f = k["fiili_kurus"].as_i64().unwrap_or(0);
        te += e;
        tf += f;
        h.push_str(&format!(
            r#"<tr><td>{}</td><td class="s">{}</td><td class="s">{}</td><td class="s">{}</td><td>{}</td></tr>"#,
            kac(k["ad"].as_str().unwrap_or("")),
            tutar(e),
            tutar(f),
            tutar(k["fark_kurus"].as_i64().unwrap_or(0)),
            kac(k["gerekce"].as_str().unwrap_or(""))
        ));
    }
    h.push_str(&format!(
        r#"</tbody><tfoot><tr><td>Toplam</td><td class="s">{}</td><td class="s">{}</td><td class="s">{}</td><td></td></tr></tfoot></table>"#,
        tutar(te),
        tutar(tf),
        tutar(tf - te)
    ));
    h.push_str("<h2>3. Yapılan Kontroller ve Sonuç</h2>");
    for k in cd["kontroller"].as_array().into_iter().flatten() {
        h.push_str(&format!(
            r#"<div><span class="kutu">{}</span> {}</div>"#,
            if k["yapildi"] == true { "[X]" } else { "[ ]" },
            kac(k["metin"].as_str().unwrap_or(""))
        ));
    }
    let sonuc = cd["sonuc"].as_str().unwrap_or("");
    h.push_str("<p><b>Sonuç:</b> ");
    for s in &sb.sonuclar {
        h.push_str(&format!(
            r#"<span class="kutu">{}</span> {} &nbsp; "#,
            if s == sonuc { "[X]" } else { "[ ]" },
            kac(s)
        ));
    }
    h.push_str("</p>");
    h.push_str(&format!(
        "<h2>4. Fark Detayı (fark varsa)</h2><table><tr><th style=\"width:35%\">Fark Tutarı (TL)</th><td>{}</td></tr><tr><th>Hangi Kalemde</th><td>{}</td></tr><tr><th>Olası Neden / Açıklama</th><td>{}</td></tr><tr><th>Yapılan İşlem</th><td>{}</td></tr></table>",
        if sonuc == SONUC_FARK_VAR { tutar(cd["fark_tutar_kurus"].as_i64().unwrap_or(0)) } else { String::new() },
        kac(cd["fark_kalem"].as_str().unwrap_or("")),
        kac(cd["neden"].as_str().unwrap_or("")),
        kac(cd["islem"].as_str().unwrap_or(""))
    ));
    h.push_str(&format!(
        r#"<p class="dipnot">{}</p><div class="imzalar">"#,
        kac(&sb.dipnot)
    ));
    for i in cd["imzalar"].as_array().into_iter().flatten() {
        let rol = i["rol_adi"].as_str().unwrap_or("");
        if i["imzalamadi"] == true {
            let m = if i["sira"] == 2 {
                IKINCI_IMZA_ATMADI
            } else {
                "imza atmadı"
            };
            let gerekce = match i["gerekce"].as_str() {
                Some(g) => format!(r#"<div class="gerekce">Gerekçe: {}</div>"#, kac(g)),
                None => String::new(),
            };
            h.push_str(&format!(
                r#"<div class="imza"><b>{}</b><div class="yok">{}</div>{}</div>"#,
                kac(rol),
                kac(&crate::sablon::buyuk_tr(m)),
                gerekce
            ));
        } else {
            let kanal = match i["kanal"].as_str() {
                Some("telefon") => format!(
                    "telefonla imzalandı (cihaz {})",
                    i["cihaz"].as_str().unwrap_or("?")
                ),
                _ => "ekranda imzalandı".to_string(),
            };
            h.push_str(&format!(
                r#"<div class="imza"><b>{}</b>Ad Soyad: {}<br>Zaman: {}<br>{}</div>"#,
                kac(rol),
                kac(i["kisi"].as_str().unwrap_or("")),
                kac(&crate::zaman::yerel_zaman(
                    i["zaman"].as_i64().unwrap_or(0),
                    crate::zaman::TURKIYE_DK
                )),
                kac(&kanal)
            ));
        }
    }
    h.push_str(r#"</div><p class="yazdir"><button onclick="print()">Yazdır / PDF</button></p></div></body></html>"#);
    h
}

// ---------------------------------------------------------------- Nebim kasa Excel içe aktarma (kasa-excel.js)

const AZAMI_DOSYA: usize = 5 * 1024 * 1024;
const AZAMI_ACIK: u64 = 20 * 1024 * 1024;
const AZAMI_SATIR: usize = 5000;
const AZAMI_SUTUN: usize = 60;

/// Türkçe harfleri sadeleştirip küçük harfe indirir (eşleştirme anahtarı).
pub fn norm(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        let c = match c {
            'ı' | 'İ' | 'I' => 'i',
            'ş' | 'Ş' => 's',
            'ğ' | 'Ğ' => 'g',
            'ü' | 'Ü' => 'u',
            'ö' | 'Ö' => 'o',
            'ç' | 'Ç' => 'c',
            c => c.to_ascii_lowercase(),
        };
        o.push(if c.is_ascii_alphanumeric() { c } else { ' ' });
    }
    o.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// tr-TR sayı ("1.234,56", "1234.5", "-12") → kuruş.
pub fn sayi_oku(s: &str) -> Option<i64> {
    let t: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if t.is_empty() {
        return None;
    }
    let t = if t.contains(',') {
        t.replace('.', "").replace(',', ".")
    } else {
        t
    };
    let n: f64 = t.parse().ok()?;
    n.is_finite().then(|| (n * 100.0).round() as i64)
}

/// Başlık adından sütun önerisi (sütun adı varsayılmaz; bulunamazsa `None`).
pub fn sutun_oner(baslik: &[String], kelimeler: &[&str]) -> Option<usize> {
    baslik.iter().position(|b| {
        let n = norm(b);
        !n.is_empty() && kelimeler.iter().any(|k| n.contains(k))
    })
}

/// Eşleştirme sonucu: kalem kodu → kuruş, eşleşmeyen dosya adları (en çok 10), dosyada olmayan kalemler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Eslesme {
    pub degerler: BTreeMap<String, i64>,
    pub eslesmeyen: Vec<String>,
    pub yok_olan: Vec<String>,
}

/// Dosya satırlarını 8 kalemle eşler (önce tam ad, sonra içerme). Aynı kalem birden çok satırdaysa toplanır.
pub fn eslestir(
    satirlar: &[Vec<String>],
    kalem_sutun: usize,
    deger_sutun: usize,
    ilk_veri: usize,
) -> Eslesme {
    let anahtar: Vec<String> = TUTAA_KALEMLERI.iter().map(|(_, a)| norm(a)).collect();
    let mut degerler: BTreeMap<String, i64> = BTreeMap::new();
    let mut eslesmeyen = Vec::new();
    for r in satirlar.iter().skip(ilk_veri) {
        let ad = norm(r.get(kalem_sutun).map(String::as_str).unwrap_or(""));
        let Some(d) = r.get(deger_sutun).and_then(|v| sayi_oku(v)) else {
            continue;
        };
        if ad.is_empty() {
            continue;
        }
        let j = anahtar.iter().position(|a| *a == ad).or_else(|| {
            anahtar
                .iter()
                .position(|a| ad.contains(a.as_str()) || a.contains(ad.as_str()))
        });
        match j {
            Some(j) => {
                *degerler
                    .entry(TUTAA_KALEMLERI[j].0.to_string())
                    .or_insert(0) += d
            }
            None => eslesmeyen.push(
                r.get(kalem_sutun)
                    .cloned()
                    .unwrap_or_default()
                    .chars()
                    .take(60)
                    .collect(),
            ),
        }
    }
    eslesmeyen.truncate(10);
    let yok_olan = TUTAA_KALEMLERI
        .iter()
        .filter(|(k, _)| !degerler.contains_key(*k))
        .map(|(_, a)| a.to_string())
        .collect();
    Eslesme {
        degerler,
        eslesmeyen,
        yok_olan,
    }
}

fn xml_coz(s: &str) -> String {
    let mut o = s
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'");
    while let Some(i) = o.find("&#") {
        let Some(j) = o[i..].find(';') else { break };
        let n = &o[i + 2..i + j];
        let c = if let Some(h) = n.strip_prefix('x') {
            u32::from_str_radix(h, 16).ok()
        } else {
            n.parse().ok()
        };
        let r: String = c
            .and_then(char::from_u32)
            .map(String::from)
            .unwrap_or_default();
        o.replace_range(i..i + j + 1, &r);
    }
    o.replace("&amp;", "&")
}

/// `<t ...>metin</t>` parçalarını birleştirir.
fn t_metni(x: &str) -> String {
    let mut o = String::new();
    let mut k = x;
    while let Some(i) = k.find("<t") {
        let after = &k[i + 2..];
        if !(after.starts_with('>') || after.starts_with(' ')) {
            k = after;
            continue;
        }
        let Some(a) = after.find('>') else { break };
        let ic = &after[a + 1..];
        let Some(b) = ic.find("</t>") else { break };
        o.push_str(&xml_coz(&ic[..b]));
        k = &ic[b + 4..];
    }
    o
}

fn oznitelik<'a>(etiket: &'a str, ad: &str) -> Option<&'a str> {
    let i = etiket.find(&format!(" {ad}=\""))? + ad.len() + 3;
    let j = etiket[i..].find('"')?;
    Some(&etiket[i..i + j])
}

/// Güvenilmez xlsx (Nebim ya da doldurulmuş sayım şablonu) → satırlar (ilk sayfa). Sınırlar: 5 MB dosya,
/// 20 MB açılmış, 5000 satır, 60 sütun. Değerler yalnız metin olarak döner.
pub fn xlsx_oku(b: &[u8]) -> Result<Vec<Vec<String>>, String> {
    if b.len() > AZAMI_DOSYA {
        return Err("dosya 5 MB'den büyük".into());
    }
    let mut z = zip::ZipArchive::new(std::io::Cursor::new(b))
        .map_err(|_| "xlsx dosyası okunamadı".to_string())?;
    let mut oku = |ad: &str| -> Result<Option<String>, String> {
        let mut f = match z.by_name(ad) {
            Ok(f) => f,
            Err(_) => return Ok(None),
        };
        if f.size() > AZAMI_ACIK {
            return Err("xlsx içeriği çok büyük".into());
        }
        let mut s = String::new();
        f.by_ref()
            .take(AZAMI_ACIK)
            .read_to_string(&mut s)
            .map_err(|_| "xlsx içeriği bozuk".to_string())?;
        Ok(Some(s))
    };
    let mut ortak = Vec::new();
    if let Some(ss) = oku("xl/sharedStrings.xml")? {
        let mut k = ss.as_str();
        while let Some(i) = k.find("<si") {
            let Some(j) = k[i..].find("</si>") else { break };
            ortak.push(t_metni(&k[i..i + j]));
            k = &k[i + j + 5..];
        }
    }
    let sayfa = oku("xl/worksheets/sheet1.xml")?.ok_or("xlsx içinde sayfa yok")?;
    let mut satirlar = Vec::new();
    let mut k = sayfa.as_str();
    while let Some(i) = k.find("<row") {
        if satirlar.len() >= AZAMI_SATIR {
            break;
        }
        let Some(j) = k[i..].find("</row>").or_else(|| k[i..].find("/>")) else {
            break;
        };
        let satir = &k[i..i + j];
        k = &k[i + j + 1..];
        let mut r: Vec<String> = Vec::new();
        let mut c = satir;
        while let Some(a) = c.find("<c") {
            let rest = &c[a..];
            let Some(e) = rest.find('>') else { break };
            let etiket = &rest[..e];
            let (ic, son) = if etiket.ends_with('/') {
                ("", e + 1)
            } else {
                match rest.find("</c>") {
                    Some(x) => (&rest[e + 1..x], x + 4),
                    None => break,
                }
            };
            c = &rest[son..];
            let ref_ = oznitelik(etiket, "r").unwrap_or("");
            let mut no = 0usize;
            for ch in ref_.chars().take_while(|x| x.is_ascii_uppercase()) {
                no = no * 26 + (ch as usize - 64);
            }
            if no == 0 || no > AZAMI_SUTUN {
                continue;
            }
            let v = match oznitelik(etiket, "t") {
                Some("s") => t_icerik(ic, "v")
                    .and_then(|x| x.trim().parse::<usize>().ok())
                    .and_then(|n| ortak.get(n).cloned())
                    .unwrap_or_default(),
                Some("inlineStr") => t_metni(ic),
                _ => t_icerik(ic, "v").map(|x| xml_coz(&x)).unwrap_or_default(),
            };
            if r.len() < no {
                r.resize(no, String::new());
            }
            r[no - 1] = v;
        }
        satirlar.push(r);
    }
    Ok(satirlar)
}

fn t_icerik(x: &str, ad: &str) -> Option<String> {
    let i = x.find(&format!("<{ad}>"))? + ad.len() + 2;
    let j = x[i..].find(&format!("</{ad}>"))?;
    Some(x[i..i + j].to_string())
}

/// İçe aktarılan Excel'den tutanağın ERP sütunu: başlık satırından kalem ve tutar sütunu önerilir
/// (kalem / ödeme; erp / bakiye / tutar / toplam); öneri yoksa ilk iki sütun.
pub fn excel_erp(b: &[u8]) -> Result<Eslesme, String> {
    let s = xlsx_oku(b)?;
    let bas = s.first().cloned().unwrap_or_default();
    let ks = sutun_oner(&bas, &["kalem", "odeme"]).unwrap_or(0);
    let ds = sutun_oner(&bas, &["erp", "bakiye", "fiili", "tutar", "toplam"]).unwrap_or(1);
    let ilk = usize::from(
        ks != 0 || ds != 1 || bas.iter().any(|x| sayi_oku(x).is_none() && !x.is_empty()),
    );
    Ok(eslestir(&s, ks, ds, ilk))
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::gunsonu::{ImzaKanali, Karsilastirma, NoktaGunSonu};

    fn kayit() -> (KasaAyar, GunSonuKaydi) {
        let a = KasaAyar {
            sube: "S1".into(),
            ..KasaAyar::default()
        };
        let mut ciro = BTreeMap::new();
        for (k, v) in [
            ("nakit", 100_000),
            ("ziraat_pos", 50_000),
            ("iyzico_pos", 70_000),
            ("efatura_pos", 5_000),
            ("havale", 30_000),
            ("iyzico_odemesi", 8_000),
            ("degisim", 12_000),
            ("internet_degisimi", 2_000),
            ("havale_fark", -1_000),
            ("kasa_fazlasi", 500),
        ] {
            ciro.insert(k.to_string(), v);
        }
        let mut pos = BTreeMap::new();
        pos.insert("ziraat".to_string(), 50_000);
        pos.insert("iyzico".to_string(), 75_000);
        let gs = GunSonuKaydi {
            id: "GS-1".into(),
            tarih: "2026-11-25".into(),
            sube: "S1".into(),
            cekmece: "c1".into(),
            kisi: "A".into(),
            nakit_adetler: BTreeMap::new(),
            nakit_sayilan_kurus: 120_500,
            noktalar: vec![NoktaGunSonu {
                nokta: "k1".into(),
                z_toplam_kurus: 0,
                pos,
            }],
            nebim_toplam_kurus: 0,
            ciro,
            doviz: vec![DovizSatiri {
                kod: "USD".into(),
                tutar: 1_000,
                kur_onbinde: 420_000,
            }],
            devreden_kurus: 0,
            kontrol: vec![true; 7],
            not: "havale ertesi gün geldi".into(),
            aciklamalar: BTreeMap::new(),
            karsilastirma: Karsilastirma {
                satirlar: vec![],
                ciro_toplam_kurus: 0,
                fark_var: true,
                kasa_ciro_uyusmuyor: false,
            },
            zaman: 0,
            duzeltir: None,
        };
        (a, gs)
    }

    #[test]
    fn sekiz_satir_eslemesi_ve_sonuc() {
        let (a, gs) = kayit();
        let imza = ImzaKaydi {
            id: "GS-1".into(),
            sira: 1,
            rol: "kasa_sorumlusu".into(),
            kisi: "A".into(),
            zaman: 5,
            kanal: ImzaKanali::Telefon,
            cihaz: Some("tel-1".into()),
            ozet: None,
            cihaz_dogrulama: None,
            kisi_rol: None,
        };
        let v = cift_dogrulama(&a, &gs, 20_000, &[imza], None);
        let k = v["kalemler"].as_array().unwrap();
        assert_eq!(k.len(), 8);
        let satir = |kod: &str| k.iter().find(|x| x["kod"] == kod).unwrap().clone();
        // Nakit: fiili = sayılan − açılış; fark = kasa fazlası.
        assert_eq!(satir("nakit_tl")["fiili_kurus"], 100_500);
        assert_eq!(satir("nakit_tl")["fark_kurus"], 500);
        // E-fatura POS:2 İyzico fiziksel POS'a düşer.
        assert_eq!(satir("iyzico_fiziksel_pos")["erp_kurus"], 75_000);
        assert_eq!(satir("iyzico_fiziksel_pos")["fark_kurus"], 0);
        assert_eq!(satir("iyzico_sanal_pos")["erp_kurus"], 8_000);
        assert_eq!(satir("havale_eft")["fark_kurus"], -1_000);
        // Kredi çeki = mağaza değişimi; internet değişimi E-ticaret satırında.
        assert_eq!(satir("kredi_ceki")["erp_kurus"], 10_000);
        assert_eq!(satir("eticaret_degisimi")["erp_kurus"], 2_000);
        assert_eq!(satir("diger")["erp_kurus"], 42_000);
        assert_eq!(v["sonuc"], SONUC_FARK_VAR);
        assert_eq!(v["toplam_fark_kurus"], -500);
        assert_eq!(v["fark_tutar_kurus"], 1_500);
        assert_eq!(satir("havale_eft")["gerekce"], "havale ertesi gün geldi");
        assert_eq!(satir("ziraat_pos")["gerekce"], "");
        let im = v["imzalar"].as_array().unwrap();
        assert_eq!(im[0]["rol_adi"], "Kasa Sorumlusu");
        assert_eq!(im[0]["cihaz"], "tel-1");
        assert_eq!(im[1]["rol_adi"], "Mağaza Sorumlusu / Vekili");
        assert_eq!(im[1]["imzalamadi"], true);
        assert_eq!(im[1]["not"], "ikinci imza atmadı");
    }

    #[test]
    fn sablon_tutaa_ile_ayni_ve_a4_tutanak() {
        let sb = sablon();
        let ad: Vec<&str> = TUTAA_KALEMLERI.iter().map(|(_, a)| *a).collect();
        assert_eq!(sb.kalemler, ad, "8 sabit satır Tutaa şablonuyla birebir");
        assert_eq!(
            sb.roller,
            vec!["Kasa Sorumlusu", "Mağaza Sorumlusu / Vekili"]
        );
        assert_eq!(sb.kontroller.len(), 5);
        assert_eq!(sb.sonuclar, vec![SONUC_FARK_YOK, SONUC_FARK_VAR]);
        let (a, gs) = kayit();
        assert_eq!(numara(&gs), "KCD-2026-1");
        let taslak = tutanak_html(
            &cift_dogrulama(&a, &gs, 20_000, &[], None),
            "Şube <1>",
            "26.11.2026",
        );
        assert!(taslak.contains("TASLAK") && taslak.contains("Şube &lt;1&gt;"));
        assert!(taslak.contains("İKİNCİ İMZA ATMADI"));
        let imza = ImzaKaydi {
            id: "GS-1".into(),
            sira: 1,
            rol: "kasa_sorumlusu".into(),
            kisi: "A".into(),
            zaman: 0,
            kanal: ImzaKanali::Telefon,
            cihaz: Some("tel-1".into()),
            ozet: None,
            cihaz_dogrulama: None,
            kisi_rol: None,
        };
        let h = tutanak_html(
            &cift_dogrulama(&a, &gs, 20_000, &[imza], None),
            "Şube",
            "26.11.2026",
        );
        assert!(!h.contains("TASLAK"));
        assert!(h.contains("KASA ÇİFT DOĞRULAMA TUTANAĞI") && h.contains("@page{size:A4"));
        assert!(
            h.contains("telefonla imzalandı (cihaz tel-1)") && h.contains("İKİNCİ İMZA ATMADI")
        );
        assert!(h.contains("[X]</span> Fark tespit edilmiştir."));
        assert!(h.contains("Nakit (TL)") && h.contains("1.005,00"));
    }

    #[test]
    fn sonuc_sec_ve_excel_eslestirme() {
        assert_eq!(sonuc_sec(&[0, 0]), SONUC_FARK_YOK);
        assert_eq!(sonuc_sec(&[0, -1]), SONUC_FARK_VAR);
        assert_eq!(sayi_oku("2.000,25"), Some(200_025));
        assert_eq!(sayi_oku("1234.5"), Some(123_450));
        assert_eq!(sayi_oku("abc"), None);
        assert_eq!(norm("ZIRAAT bankası POS"), "ziraat bankasi pos");
        let b = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
        assert_eq!(
            sutun_oner(&b(&["Ödeme Türü", "Fiili Sayım"]), &["fiili"]),
            Some(1)
        );
        assert_eq!(sutun_oner(&b(&["A", "B"]), &["fiili", "erp"]), None);
        let s = vec![
            b(&["Kalem", "ERP"]),
            b(&["nakit", "1234,5"]),
            b(&["ZIRAAT bankasi pos", "2.000,25"]),
            b(&["İyzico Sanal POS", "10"]),
            b(&["İyzico Sanal POS", "5"]),
            b(&["Bilinmeyen & <b>", "7"]),
            b(&["Diğer", "abc"]),
        ];
        let e = eslestir(&s, 0, 1, 1);
        assert_eq!(e.degerler["nakit_tl"], 123_450);
        assert_eq!(e.degerler["ziraat_pos"], 200_025);
        assert_eq!(e.degerler["iyzico_sanal_pos"], 1_500);
        assert_eq!(e.eslesmeyen, vec!["Bilinmeyen & <b>".to_string()]);
        assert!(
            e.yok_olan.contains(&"Diğer".to_string())
                && e.yok_olan.contains(&"Havale / EFT".to_string())
        );
    }

    #[test]
    fn xlsx_gidis_donus_ve_bozuk_dosya() {
        // TAA Kasa'nın kendi xlsx yazıcısıyla şablon yazılır, içe aktarıcı geri okur.
        use crate::xlsx::{Hucre, Sayfa};
        let mut satir = vec![vec![
            Hucre::from("Kalem"),
            Hucre::from("ERP (Nebim) Bakiyesi"),
        ]];
        satir.push(vec![Hucre::from("Nakit (TL)"), Hucre::Kurus(150_050)]);
        satir.push(vec![
            Hucre::from("Havale / EFT & <x>"),
            Hucre::Kurus(20_000),
        ]);
        let b = crate::xlsx::xlsx_uret(&[Sayfa {
            ad: "Sayım".into(),
            satirlar: satir,
            ..Default::default()
        }]);
        let s = xlsx_oku(&b).unwrap();
        assert_eq!(s[0][0], "Kalem");
        assert_eq!(s[2][0], "Havale / EFT & <x>");
        let e = excel_erp(&b).unwrap();
        assert_eq!(e.degerler["nakit_tl"], 150_050);
        assert_eq!(e.degerler["havale_eft"], 20_000);
        assert!(xlsx_oku(&[1, 2, 3, 4, 5]).unwrap_err().contains("xlsx"));
        assert!(xlsx_oku(&vec![0u8; AZAMI_DOSYA + 1])
            .unwrap_err()
            .contains("5 MB"));
        assert!(xlsx_oku(&b[..b.len() - 30]).is_err());
    }

    #[test]
    fn hareket_ozeti_erp_degeri_kullanilir_ve_farksiz_sonuc() {
        let (a, mut gs) = kayit();
        gs.ciro.insert("havale_fark".into(), 0);
        gs.nakit_sayilan_kurus = 120_000;
        let v = cift_dogrulama(&a, &gs, 20_000, &[], None);
        assert_eq!(v["sonuc"], SONUC_FARK_YOK);
        assert_eq!(v["fark_kalem"], "");
        let mut erp = BTreeMap::new();
        erp.insert("ziraat_pos".to_string(), 49_000);
        let v = cift_dogrulama(&a, &gs, 20_000, &[], Some(&erp));
        let z = &v["kalemler"][1];
        assert_eq!(
            (z["erp_kurus"].as_i64(), z["fark_kurus"].as_i64()),
            (Some(49_000), Some(1_000))
        );
        assert_eq!(v["sonuc"], SONUC_FARK_VAR);
        assert_eq!(v["fark_kalem"], "Ziraat Bankası POS");
    }
}
