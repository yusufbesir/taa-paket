// Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]
//! Dış crate olmadan tarih/saat: Unix saniyesi ↔ yerel takvim (sabit fark; Türkiye UTC+3, yaz saati yok).

use std::time::{SystemTime, UNIX_EPOCH};

/// Türkiye saati (2016'dan beri sabit UTC+3).
pub const TURKIYE_DK: i32 = 180;

pub fn simdi_sn() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

/// Gün sayısı (1970-01-01'den) → (yıl, ay, gün). Howard Hinnant'ın algoritması.
pub fn gunden_takvim(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// (yıl, ay, gün) → gün sayısı.
pub fn takvimden_gun(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let mp = if m > 2 { m - 3 } else { m + 9 } as i64;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// Unix saniyesi → yerel "YYYY-AA-GG".
pub fn yerel_tarih(sn: i64, fark_dk: i32) -> String {
    let y = sn + fark_dk as i64 * 60;
    let (a, b, c) = gunden_takvim(y.div_euclid(86_400));
    format!("{a:04}-{b:02}-{c:02}")
}

/// Unix saniyesi → yerel "YYYY-AA-GG SS:DD".
pub fn yerel_zaman(sn: i64, fark_dk: i32) -> String {
    let y = sn + fark_dk as i64 * 60;
    let g = y.rem_euclid(86_400);
    format!(
        "{} {:02}:{:02}",
        yerel_tarih(sn, fark_dk),
        g / 3600,
        (g % 3600) / 60
    )
}

/// "YYYY-AA-GG" geçerli mi (ayın gün sayısı dahil).
pub fn tarih_gecerli(t: &str) -> bool {
    tarih_coz(t).is_some()
}

/// "YYYY-AA-GG" → gün sayısı.
pub fn tarih_coz(t: &str) -> Option<i64> {
    let b = t.as_bytes();
    if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
        return None;
    }
    if !t
        .char_indices()
        .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
    {
        return None;
    }
    let y: i64 = t[..4].parse().ok()?;
    let m: u32 = t[5..7].parse().ok()?;
    let d: u32 = t[8..10].parse().ok()?;
    if !(1..=12).contains(&m) || d == 0 {
        return None;
    }
    let g = takvimden_gun(y, m, d);
    (gunden_takvim(g) == (y, m, d)).then_some(g)
}

/// "SS:DD" → gün içindeki dakika.
pub fn saat_coz(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60 && s.len() == 5).then_some(h * 60 + m)
}

/// Yerel tarih + "SS:DD" → Unix saniyesi.
pub fn yerel_an(tarih: &str, saat: &str, fark_dk: i32) -> Option<i64> {
    let g = tarih_coz(tarih)?;
    let dk = saat_coz(saat)? as i64;
    Some(g * 86_400 + dk * 60 - fark_dk as i64 * 60)
}

/// Gün ekler/çıkarır.
pub fn gun_ekle(tarih: &str, n: i64) -> Option<String> {
    let g = tarih_coz(tarih)? + n;
    let (a, b, c) = gunden_takvim(g);
    Some(format!("{a:04}-{b:02}-{c:02}"))
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn takvim_gidis_donus() {
        assert_eq!(gunden_takvim(0), (1970, 1, 1));
        assert_eq!(takvimden_gun(2026, 11, 26), 20_783);
        for g in [-1000, 0, 11_016, 20_783, 50_000] {
            let (y, m, d) = gunden_takvim(g);
            assert_eq!(takvimden_gun(y, m, d), g);
        }
    }

    #[test]
    fn yerel_bicimler() {
        // 2026-11-26 21:30 UTC = 2026-11-27 00:30 TR.
        let sn = 20_783 * 86_400 + 21 * 3600 + 30 * 60;
        assert_eq!(yerel_tarih(sn, 0), "2026-11-26");
        assert_eq!(yerel_tarih(sn, TURKIYE_DK), "2026-11-27");
        assert_eq!(yerel_zaman(sn, TURKIYE_DK), "2026-11-27 00:30");
        assert_eq!(yerel_an("2026-11-27", "00:30", TURKIYE_DK), Some(sn));
    }

    #[test]
    fn dogrulama() {
        assert!(tarih_gecerli("2028-02-29"));
        assert!(!tarih_gecerli("2026-02-29"));
        assert!(!tarih_gecerli("2026-04-31"));
        assert!(!tarih_gecerli("26.11.2026"));
        assert_eq!(saat_coz("23:59"), Some(1439));
        assert_eq!(saat_coz("24:00"), None);
        assert_eq!(gun_ekle("2026-12-31", 1).as_deref(), Some("2027-01-01"));
        assert_eq!(gun_ekle("2026-03-01", -1).as_deref(), Some("2026-02-28"));
    }
}
