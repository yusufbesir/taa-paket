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
use taa_raporlar::uygulamalar::{paket_raporu, UYGULAMALAR};
use taa_raporlar::{Esikler, YONETICI_ROLU};

/// (HTTP durum, içerik türü, gövde, indirme dosya adı).
pub type Cevap = (u16, &'static str, String, Option<String>);

const JSON_TUR: &str = "application/json; charset=utf-8";
const CSV_TUR: &str = "text/csv; charset=utf-8";

fn hata(durum: u16, m: &str, kod: u8) -> Cevap {
    (
        durum,
        JSON_TUR,
        json!({"hata": m, "kod": kod}).to_string(),
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
                .to_string(),
                None,
            )
        }
        "/api/raporlar" | "/api/raporlar/csv" => {
            let esikler = match Esikler::kokten(kok) {
                Ok(e) => e,
                Err(m) => return hata(409, &m, 2),
            };
            let r = match paket_raporu(kok, fis_klasoru, tarih, &esikler, uygulama) {
                Ok(r) => r,
                Err(m) => return hata(400, &m, 2),
            };
            if yol.ends_with("/csv") {
                let ad = format!("taa-rapor-{}-{}.csv", uygulama.unwrap_or("hepsi"), tarih);
                (200, CSV_TUR, r.csv(), Some(ad))
            } else {
                (200, JSON_TUR, r.json().to_string(), None)
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
        assert_eq!(yanit(&d, None, None, "/api/raporlar", "", t).0, 401);
        for r in ["personel", "tezgahtar", "yonetici2", ""] {
            let rl = rol(&[r]);
            for y in ["/api/raporlar", "/api/raporlar/csv", "/api/raporlar/liste"] {
                let c = yanit(&d, None, Some(&rl), y, "", t);
                assert_eq!(c.0, 403, "{r} {y}");
                assert!(!c.2.contains("uygulamalar"), "veri sızmamalı");
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
        let v: Value = serde_json::from_str(&c.2).unwrap();
        assert_eq!(
            v["uygulamalar"].as_array().unwrap().len(),
            UYGULAMALAR.len()
        );
        assert!(
            !c.2.contains("gizli-ad"),
            "kişisel veri içeriği raporda olmamalı"
        );
        let c = yanit(&d, None, Some(&y), "/api/raporlar", "uygulama=kasa", t);
        let v: Value = serde_json::from_str(&c.2).unwrap();
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
        assert!(c.2.starts_with('\u{feff}'));
        assert!(c.2.contains("uygulama;bolum;durum;anahtar;deger"));
        assert_eq!(c.3.as_deref(), Some("taa-rapor-paket-2026-11-26.csv"));
        assert!(!c.2.contains(d.to_str().unwrap()), "dosya yolu sızmamalı");
    }
}
