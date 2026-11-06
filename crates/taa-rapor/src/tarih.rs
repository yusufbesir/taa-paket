// Sahip: Yusuf. yazan: [Claude#taa-rapor-T5 2026-11-06]
//! Bugünün tarihi (UTC, YYYY-AA-GG) dış bağımlılık olmadan; taa-kur `zaman.rs` ile aynı takvim hesabı.

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

/// UNIX saniyesinden "YYYY-AA-GG".
pub fn tarih_metni(saniye: i64) -> String {
    let (y, a, g) = gunden_tarih(saniye.div_euclid(86_400));
    format!("{y:04}-{a:02}-{g:02}")
}

pub fn bugun() -> String {
    let s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    tarih_metni(s)
}

/// YYYY-AA-GG biçim denetimi (ay 1-12, gün 1-31).
pub fn tarih_gecerli(t: &str) -> bool {
    let b = t.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && t.char_indices()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
        && matches!(t[5..7].parse::<u32>(), Ok(1..=12))
        && matches!(t[8..10].parse::<u32>(), Ok(1..=31))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn bilinen_gunler() {
        assert_eq!(tarih_metni(0), "1970-01-01");
        assert_eq!(tarih_metni(1_709_164_800), "2024-02-29");
        assert_eq!(tarih_metni(1_762_387_200), "2025-11-06");
        assert!(tarih_gecerli("2026-11-06"));
        assert!(!tarih_gecerli("2026-13-06"));
        assert!(!tarih_gecerli("06.11.2026"));
        assert_eq!(bugun().len(), 10);
    }
}
