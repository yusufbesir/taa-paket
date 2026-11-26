//! Zaman yardımcıları: Unix saniyesinden `YYYY-AA-GG` ve RFC 3339 (UTC). Dış bağımlılık yok.
//! Sahip: Yusuf. yazan: [Claude#rehber-panel 2026-11-26]

/// Gün sayısından (1970-01-01 = 0) takvim günü; Howard Hinnant "civil_from_days" algoritması.
fn gunden(gun: i64) -> (i64, u32, u32) {
    let z = gun + 719_468;
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

/// `YYYY-AA-GG` (UTC).
pub fn tarih(sn: u64) -> String {
    let (y, m, d) = gunden((sn / 86_400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// `YYYY-AA-GGTSS:DD:SSZ` (UTC).
pub fn zaman(sn: u64) -> String {
    let g = sn % 86_400;
    format!(
        "{}T{:02}:{:02}:{:02}Z",
        tarih(sn),
        g / 3600,
        (g % 3600) / 60,
        g % 60
    )
}

#[cfg(test)]
mod testler {
    use super::*;
    use taa_rehber_model::model::{tarih_gecerli, zaman_gecerli};

    #[test]
    fn bilinen_tarihler() {
        assert_eq!(tarih(0), "1970-01-01");
        assert_eq!(tarih(951_782_400), "2000-02-29"); // artık gün
        assert_eq!(tarih(1_764_115_200), "2025-11-26");
        assert_eq!(
            zaman(1_764_115_200 + 3 * 3600 + 4 * 60 + 5),
            "2025-11-26T03:04:05Z"
        );
        assert_eq!(tarih(4_102_444_799), "2099-12-31");
    }

    #[test]
    fn model_biciminde() {
        for sn in [0u64, 86_399, 1_000_000_000, 1_795_000_000] {
            assert!(tarih_gecerli(&tarih(sn)), "{}", tarih(sn));
            assert!(zaman_gecerli(&zaman(sn)), "{}", zaman(sn));
        }
    }
}
