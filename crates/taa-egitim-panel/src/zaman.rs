// Sahip: Yusuf. yazan: [Claude#egitim-panel 2026-11-26]
//! Saat yardımcıları. `taa-egitim` saati bilmez (zamanı çağıran verir); panel burada RFC 3339 UTC metni üretir.

use std::time::{SystemTime, UNIX_EPOCH};

/// Unix saniyesi → `YYYY-AA-GGTSS:DD:SSZ` (UTC, 1970'ten sonrası; negatif değer 1970-01-01'e sabitlenir).
pub fn iso_unix(sn: i64) -> String {
    let sn = sn.max(0);
    let gun = sn / 86_400;
    let gunici = sn % 86_400;
    // Howard Hinnant "civil_from_days" algoritması.
    let z = gun + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        gunici / 3600,
        gunici % 3600 / 60,
        gunici % 60
    )
}

/// Şu an (UTC, RFC 3339). Saat okunamazsa 1970 başı (günlük yine geçerli biçimde yazılır).
pub fn simdi_iso() -> String {
    let sn = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    iso_unix(sn)
}

/// `YYYY-AA-GG` kısmı (görevlendirme tarihleri için).
pub fn gun_kismi(iso: &str) -> &str {
    iso.get(..10).unwrap_or(iso)
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn bilinen_anlar() {
        assert_eq!(iso_unix(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_unix(951_782_400), "2000-02-29T00:00:00Z");
        assert_eq!(iso_unix(1_700_000_000), "2023-11-14T22:13:20Z");
        assert_eq!(iso_unix(-5), "1970-01-01T00:00:00Z");
    }

    #[test]
    fn simdi_gecerli_bicimde() {
        let s = simdi_iso();
        assert!(taa_rehber_model::model::zaman_gecerli(&s), "{s}");
        assert_eq!(gun_kismi(&s).len(), 10);
        assert_eq!(gun_kismi("2026-11-26T09:00:00Z"), "2026-11-26");
    }
}
