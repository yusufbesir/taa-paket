// Sahip: Yusuf. yazan: [Claude#taa-kuyruk-T6 2026-11-06]
//! UTC zaman biçimi (dış bağımlılık yok; taa-veri/zaman.rs ile aynı hesap).

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

/// RFC3339 UTC; verilen saniye UNIX epoch'tan beri.
pub fn bicimle(saniye: i64) -> String {
    let gun = saniye.div_euclid(86_400);
    let kalan = saniye.rem_euclid(86_400);
    let (y, a, g) = gunden_tarih(gun);
    let (s, d, sn) = (kalan / 3600, (kalan % 3600) / 60, kalan % 60);
    format!("{y:04}-{a:02}-{g:02}T{s:02}:{d:02}:{sn:02}Z")
}

/// `YYYY-AA-GGTSS:DD:SSZ` metnini UNIX saniyesine çevirir; biçim bozuksa None.
pub fn coz(iso: &str) -> Option<i64> {
    let b = iso.as_bytes();
    if b.len() != 20
        || b[4] != b'-'
        || b[7] != b'-'
        || b[10] != b'T'
        || b[13] != b':'
        || b[16] != b':'
        || b[19] != b'Z'
    {
        return None;
    }
    let n = |a: usize, u: usize| iso.get(a..a + u)?.parse::<i64>().ok();
    let (y, a, g, s, d, sn) = (
        n(0, 4)?,
        n(5, 2)?,
        n(8, 2)?,
        n(11, 2)?,
        n(14, 2)?,
        n(17, 2)?,
    );
    if !(1..=12).contains(&a) || !(1..=31).contains(&g) || s > 23 || d > 59 || sn > 60 {
        return None;
    }
    // days_from_civil (Howard Hinnant)
    let y2 = if a <= 2 { y - 1 } else { y };
    let era = y2.div_euclid(400);
    let yoe = y2 - era * 400;
    let mp = if a > 2 { a - 3 } else { a + 9 };
    let doy = (153 * mp + 2) / 5 + g - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let gun = era * 146_097 + doe - 719_468;
    Some(gun * 86_400 + s * 3600 + d * 60 + sn)
}

pub fn simdi() -> String {
    let s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    bicimle(s)
}
