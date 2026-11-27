//! Kilitli yöntem değerleri (docs/olcum/YONTEM.md ile birebir). Sahip: Yusuf. yazan: [Claude#olcum 2026-11-26]
//! Belge ve bu sabitler birlikte değişir; `kilit_tutuyor` testi ikisini bağlar.

use sha2::{Digest, Sha256};

/// Yöntem belgesinin metni (derleme anında gömülür; rapora özeti yazılır).
pub const YONTEM_METNI: &str = include_str!("../../../docs/olcum/YONTEM.md");

/// Kilitlenen belge özeti. Opus yöntemi kesinleştirirse belge ile birlikte güncellenir.
pub const YONTEM_SHA256_KILIT: &str =
    "040a9942bec494d492e5056c4ac3fe3753b9407fa0a84072cb67703928ad0fa7";

/// Ölçülen pay basamakları (yüzde); her biri ayrı ölçülür.
pub const PAY_BASAMAKLARI: [u32; 5] = [5, 8, 10, 12, 15];
/// Isınma tekrarı (atılır).
pub const ISINMA_TEKRAR: u32 = 1;
/// Geçerli tekrar sayısı (ortanca-3).
pub const OLCUM_TEKRAR: usize = 3;
/// Geçersiz tekrar yerine en çok yapılacak ek deneme.
pub const EK_DENEME: u32 = 3;
/// Varsayılan tekrar süresi (ms).
pub const VARSAYILAN_SURE_MS: u64 = 2000;
/// Ortanca gecikme artışı üst sınırı (yüzde).
pub const ORTANCA_ARTIS_EN_COK_YUZDE: f64 = 10.0;
/// Hiçbir işlemin tabana göre ek gecikmesi bunu aşamaz (ms).
pub const TEK_ISLEM_EK_GECIKME_MS: f64 = 200.0;
/// Koşular arası ortanca hız sapması üst sınırı (yüzde).
pub const KOSU_FARKI_EN_COK_YUZDE: f64 = 10.0;

/// Belgenin SHA-256 özeti (onaltılık).
pub fn yontem_ozeti() -> String {
    ozet_hex(YONTEM_METNI.as_bytes())
}

pub fn ozet_hex(b: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(b);
    h.finalize().iter().map(|x| format!("{x:02x}")).collect()
}

/// Gömülü belge kilitle eşleşiyor mu.
pub fn kilit_tutuyor() -> bool {
    yontem_ozeti() == YONTEM_SHA256_KILIT
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn kilit_tutuyor_belge_degismemis() {
        assert_eq!(
            yontem_ozeti(),
            YONTEM_SHA256_KILIT,
            "YONTEM.md değişti: kilidi onayla ve güncelle"
        );
        assert!(kilit_tutuyor());
    }

    #[test]
    fn belge_sabitleri_icerir() {
        assert!(YONTEM_METNI.contains("**5, 8, 10, 12, 15**"));
        assert!(YONTEM_METNI.contains("en çok %10"));
        assert!(YONTEM_METNI.contains("200 ms"));
        assert!(YONTEM_METNI.contains("3 geçerli tekrar"));
        assert_eq!(PAY_BASAMAKLARI, [5, 8, 10, 12, 15]);
        assert_eq!(OLCUM_TEKRAR, 3);
        assert_eq!(ORTANCA_ARTIS_EN_COK_YUZDE, 10.0);
        assert_eq!(TEK_ISLEM_EK_GECIKME_MS, 200.0);
    }
}
