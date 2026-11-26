// Sahip: Yusuf. yazan: [Claude#taa-cati-raporlar 2026-11-26]
//! TAA Çatı > Raporlar uçları (yerel panel). Yusuf 2026-11-26: "TAA Çatı altında Raporlar bölümü olsun. Tüm
//! uygulamalardan rapor çekilebilsin. Bunu yalnızca yöneticiler yapabilsin."
//!
//! - Yetki: oturumda `yonetici` rolü şart (`taa_raporlar::yonetici_mi` ile aynı kural). Başka rol → 403,
//!   hiçbir veri okunmaz. Panel ayrıca menüde yalnız yöneticiye gösterir; asıl denetim burasıdır.
//! - Veri: yerel veri kökünden salt okunur (`taa_raporlar::uygulamalar`); sunucuya gitmez, PC'den çıkmaz.
//! - Çıktı: JSON (ekran) ve CSV (Excel açar; `;`, UTF-8 BOM). PDF: ekran yazdır (tarayıcı "PDF olarak kaydet").

use serde_json::{json, Value};
use std::path::Path;
use taa_raporlar::uygulamalar::{paket_raporu, Donem, UYGULAMALAR};
use taa_raporlar::{Esikler, YONETICI_ROLU};

/// (HTTP durum, içerik türü, gövde, indirme dosya adı).
pub type Cevap = (u16, &'static str, Vec<u8>, Option<String>);

const JSON_TUR: &str = "application/json; charset=utf-8";
const CSV_TUR: &str = "text/csv; charset=utf-8";
const XLSX_TUR: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";

fn hata(durum: u16, m: &str, kod: u8) -> Cevap {
    (
        durum,
        JSON_TUR,
        json!({"hata": m, "kod": kod}).to_string().into_bytes(),
        None,
    )
}

fn sorgu_degeri<'a>(sorgu: &'a str, ad: &str) -> Option<&'a str> {
    sorgu
        .split('&')
        .filter_map(|p| p.split_once('='))
        .find(|(k, _)| *k == ad)
        .map(|(_, v)| v)
}

/// Raporlar uçlarından biri mi.
pub fn raporlar_yolu(yol: &str) -> bool {
    yol == "/api/raporlar" || yol.starts_with("/api/raporlar/")
}

/// `roller`: giriş yapmış oturumun rolleri (boşsa giriş yok sayılır).
pub fn yanit(
    kok: &Path,
    fis_klasoru: Option<&Path>,
    roller: Option<&[String]>,
    yol: &str,
    sorgu: &str,
    bugun: &str,
) -> Cevap {
    let Some(roller) = roller else {
        return hata(401, "Raporlar için giriş yapın.", 5);
    };
    if !roller.iter().any(|r| r == YONETICI_ROLU) {
        return hata(
            403,
            "Raporlar yalnız yöneticiye açıktır. Bu hesapta yönetici rolü yok.",
            7,
        );
    }
    let uygulama = sorgu_degeri(sorgu, "uygulama").filter(|u| !u.is_empty());
    let tarih = sorgu_degeri(sorgu, "tarih")
        .filter(|t| !t.is_empty())
        .unwrap_or(bugun);
    if !taa_rapor::tarih::tarih_gecerli(tarih) {
        return hata(400, "tarih YYYY-AA-GG olmalı", 2);
    }
    match yol {
        "/api/raporlar/liste" => {
            let liste: Vec<Value> = UYGULAMALAR
                .iter()
                .map(|u| json!({"kimlik": u.kimlik, "ad": u.ad, "aciklama": u.aciklama}))
                .collect();
            (
                200,
                JSON_TUR,
                json!({
                    "uygulamalar": liste,
                    "usta_puani_acik": taa_ayar::ozellik_acik("usta-puan", false),
                })
                .to_string()
                .into_bytes(),
                None,
            )
        }
        "/api/raporlar" | "/api/raporlar/csv" | "/api/raporlar/xlsx" => {
            let secim = sorgu_degeri(sorgu, "donem").unwrap_or("");
            let donem = match Donem::coz(
                secim,
                sorgu_degeri(sorgu, "bas").unwrap_or(""),
                sorgu_degeri(sorgu, "bit").unwrap_or(""),
                tarih,
            ) {
                Ok(d) => d,
                Err(m) => return hata(400, &m, 2),
            };
            let esikler = match Esikler::kokten(kok) {
                Ok(e) => e,
                Err(m) => return hata(409, &m, 2),
            };
            let r = match paket_raporu(kok, fis_klasoru, tarih, &esikler, uygulama, &donem) {
                Ok(r) => r,
                Err(m) => return hata(400, &m, 2),
            };
            let ad = |uzanti: &str| {
                format!(
                    "taa-rapor-{}-{}.{uzanti}",
                    uygulama.unwrap_or("hepsi"),
                    tarih
                )
            };
            if yol.ends_with("/csv") {
                (200, CSV_TUR, r.csv().into_bytes(), Some(ad("csv")))
            } else if yol.ends_with("/xlsx") {
                (200, XLSX_TUR, r.xlsx(), Some(ad("xlsx")))
            } else {
                (200, JSON_TUR, r.json().to_string().into_bytes(), None)
            }
        }
        _ => hata(404, "bilinmeyen rapor ucu", 2),
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn gecici(ad: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("taa-rapor-api-{}-{ad}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }
    fn rol(r: &[&str]) -> Vec<String> {
        r.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn yalniz_yonetici() {
        let d = gecici("yetki");
        let t = "2026-11-26";
        for y in [
            "/api/raporlar",
            "/api/raporlar/csv",
            "/api/raporlar/xlsx",
            "/api/raporlar/liste",
        ] {
            assert_eq!(yanit(&d, None, None, y, "", t).0, 401, "{y}");
        }
        for r in ["personel", "tezgahtar", "yonetici2", ""] {
            let rl = rol(&[r]);
            for y in [
                "/api/raporlar",
                "/api/raporlar/csv",
                "/api/raporlar/xlsx",
                "/api/raporlar/liste",
            ] {
                let c = yanit(&d, None, Some(&rl), y, "", t);
                assert_eq!(c.0, 403, "{r} {y}");
                assert!(
                    !String::from_utf8_lossy(&c.2).contains("uygulamalar"),
                    "veri sızmamalı"
                );
            }
        }
        let y = rol(&["personel", "yonetici"]);
        assert_eq!(yanit(&d, None, Some(&y), "/api/raporlar", "", t).0, 200);
    }

    #[test]
    fn tum_uygulamalar_ve_tek_uygulama() {
        let d = gecici("uyg");
        std::fs::create_dir_all(d.join("kasa-defteri")).unwrap();
        std::fs::write(d.join("kasa-defteri/kasa.jsonl"), "").unwrap();
        std::fs::create_dir_all(d.join("kimlik")).unwrap();
        std::fs::write(d.join("kimlik/k.jsonl"), "{\"kisi\":\"gizli-ad\"}\n").unwrap();
        let y = rol(&["yonetici"]);
        let t = "2026-11-26";
        let c = yanit(&d, None, Some(&y), "/api/raporlar", "", t);
        let v: Value = serde_json::from_slice(&c.2).unwrap();
        assert_eq!(
            v["uygulamalar"].as_array().unwrap().len(),
            UYGULAMALAR.len()
        );
        assert!(
            !String::from_utf8_lossy(&c.2).contains("gizli-ad"),
            "kişisel veri içeriği raporda olmamalı"
        );
        let c = yanit(&d, None, Some(&y), "/api/raporlar", "uygulama=kasa", t);
        let v: Value = serde_json::from_slice(&c.2).unwrap();
        assert_eq!(v["uygulamalar"].as_array().unwrap().len(), 1);
        assert_eq!(v["uygulamalar"][0]["kimlik"], "kasa");
        assert_eq!(
            yanit(&d, None, Some(&y), "/api/raporlar", "uygulama=yok", t).0,
            400
        );
        assert_eq!(
            yanit(&d, None, Some(&y), "/api/raporlar", "tarih=2026-13-99x", t).0,
            400
        );
    }

    #[test]
    fn csv_excel_icin_ve_formul_enjeksiyonu_yok() {
        let d = gecici("csv");
        let y = rol(&["yonetici"]);
        let c = yanit(
            &d,
            None,
            Some(&y),
            "/api/raporlar/csv",
            "uygulama=paket",
            "2026-11-26",
        );
        assert_eq!(c.0, 200);
        let m = String::from_utf8(c.2.clone()).unwrap();
        assert!(m.starts_with('\u{feff}'));
        assert!(m.contains("uygulama;bolum;durum;anahtar;deger"));
        assert_eq!(c.3.as_deref(), Some("taa-rapor-paket-2026-11-26.csv"));
        assert!(!m.contains(d.to_str().unwrap()), "dosya yolu sızmamalı");
    }

    #[test]
    fn xlsx_donem_ve_kasa_kurali() {
        let d = gecici("xlsx");
        std::fs::create_dir_all(d.join("kasa-defteri")).unwrap();
        std::fs::write(
            d.join("kasa-defteri/kasa.jsonl"),
            "{\"tur\":\"kapanis\",\"v\":{\"id\":\"a\",\"tarih\":\"2026-11-25\",\"sayilan_kurus\":100000,\"fark_kurus\":-500}}\n",
        )
        .unwrap();
        let y = rol(&["yonetici"]);
        let t = "2026-11-26";
        let c = yanit(
            &d,
            None,
            Some(&y),
            "/api/raporlar/xlsx",
            "uygulama=kasa&donem=7",
            t,
        );
        assert_eq!(c.0, 200);
        assert_eq!(&c.2[..2], b"PK");
        assert_eq!(c.3.as_deref(), Some("taa-rapor-kasa-2026-11-26.xlsx"));
        let c = yanit(
            &d,
            None,
            Some(&y),
            "/api/raporlar",
            "uygulama=kasa&donem=7",
            t,
        );
        let v: Value = serde_json::from_slice(&c.2).unwrap();
        let u = &v["uygulamalar"][0];
        assert!(u["yorumlar"]
            .to_string()
            .contains("5 günde kasa kapanışı girilmemiş"));
        assert_eq!(u["grafikler"][0]["noktalar"][0]["deger"], 1000.0);
        assert_eq!(v["donem"]["bas"], "2026-11-20");
        for kotu in [
            "donem=x",
            "donem=ozel&bas=2026-01-01&bit=2027-12-31",
            "donem=ozel&bas=2026-11-05&bit=2026-11-01",
        ] {
            assert_eq!(
                yanit(&d, None, Some(&y), "/api/raporlar", kotu, t).0,
                400,
                "{kotu}"
            );
        }
    }
}
