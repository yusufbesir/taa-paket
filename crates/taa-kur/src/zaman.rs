// Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05]
//! UTC zaman biçimleri (dış bağımlılık yok).

use std::time::{SystemTime, UNIX_EPOCH};

fn gunden_tarih(gun: i64) -> (i64, u32, u32) {
    let z = gun + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let g = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let a = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if a <= 2 { y + 1 } else { y }, a, g)
}

/// (RFC3339 UTC, "YYYYMMDD-HHMMSS") döner; verilen saniye UNIX epoch'tan beri.
pub fn bicimle(saniye: i64) -> (String, String) {
    let gun = saniye.div_euclid(86_400);
    let kalan = saniye.rem_euclid(86_400);
    let (y, a, g) = gunden_tarih(gun);
    let (s, d, sn) = (kalan / 3600, (kalan % 3600) / 60, kalan % 60);
    (
        format!("{y:04}-{a:02}-{g:02}T{s:02}:{d:02}:{sn:02}Z"),
        format!("{y:04}{a:02}{g:02}-{s:02}{d:02}{sn:02}"),
    )
}

/// UNIX epoch'tan beri saniye.
pub fn unix_saniye() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

pub fn simdi() -> (String, String) {
    bicimle(unix_saniye())
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn bilinen_zamanlar() {
        assert_eq!(bicimle(0).0, "1970-01-01T00:00:00Z");
        assert_eq!(bicimle(1_000_000_000).0, "2001-09-09T01:46:40Z");
        assert_eq!(bicimle(1_000_000_000).1, "20010909-014640");
        assert_eq!(bicimle(1_709_164_800).0, "2024-02-29T00:00:00Z");
    }
}
