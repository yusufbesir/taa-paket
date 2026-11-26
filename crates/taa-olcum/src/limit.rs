//! Limit düzeneği: gerçek işçidekiyle aynı kavramlar. Sahip: Yusuf. yazan: [Claude#olcum 2026-11-26]
//! Uygulanan: iş parçacığı sayısı sınırı ve görev oranı (duty cycle). Uygulanmayan (rapora açıkça yazılır):
//! iş nesnesi kesin işlemci üst sınırı (Windows, yer tutucu), düşük öncelik, bellek üst sınırı.

use serde::{Deserialize, Serialize};

/// Bir payın düzeneği: kaç iş parçacığı, her birinin görev oranı.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Duzenek {
    pub pay_yuzde: u32,
    pub parcacik: usize,
    /// 0 < oran <= 1: her iş parçacığının çalışma süresi payı.
    pub gorev_orani: f64,
}

/// Mantıksal çekirdek ve paydan düzenek: toplam kullanım = cekirdek x pay / 100.
pub fn duzenek_hesapla(cekirdek: usize, pay_yuzde: u32) -> Duzenek {
    let cekirdek = cekirdek.max(1);
    let hedef = cekirdek as f64 * f64::from(pay_yuzde) / 100.0;
    let parcacik = (hedef.ceil() as usize).clamp(1, cekirdek);
    let gorev_orani = (hedef / parcacik as f64).clamp(0.0001, 1.0);
    Duzenek {
        pay_yuzde,
        parcacik,
        gorev_orani,
    }
}

/// Hangi limit türleri bu yapıda gerçekten uygulanıyor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UygulananLimitler {
    pub is_parcacigi: bool,
    pub gorev_orani: bool,
    pub dusuk_oncelik: bool,
    pub bellek_ust_sinir: bool,
    pub is_nesnesi: bool,
}

pub fn uygulanan_limitler() -> UygulananLimitler {
    UygulananLimitler {
        is_parcacigi: true,
        gorev_orani: true,
        dusuk_oncelik: false,
        bellek_ust_sinir: false,
        is_nesnesi: is_nesnesi::UYGULANDI,
    }
}

/// Windows iş nesnesi (Job Object) için yer tutucu. Gerçek uygulama (JOBOBJECT_CPU_RATE_CONTROL_INFORMATION,
/// sert üst sınır) DG1b öncesi eklenecek; Windows'ta denenmedi.
#[cfg(windows)]
pub mod is_nesnesi {
    pub const UYGULANDI: bool = false;
    /// Yer tutucu: şimdilik hiçbir şey yapmaz, uygulanmadığını bildirir.
    pub fn uygula(_pay_yuzde: u32) -> Result<(), String> {
        Err("Windows iş nesnesi limiti henüz yok (yer tutucu)".to_string())
    }
}

/// Windows dışında iş nesnesi kavramı yoktur.
#[cfg(not(windows))]
pub mod is_nesnesi {
    pub const UYGULANDI: bool = false;
    pub fn uygula(_pay_yuzde: u32) -> Result<(), String> {
        Err("iş nesnesi yalnız Windows'ta vardır".to_string())
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn duzenek_toplam_kullanim_hedefte() {
        for (c, p) in [(4usize, 5u32), (4, 15), (8, 10), (16, 12), (1, 15), (2, 8)] {
            let d = duzenek_hesapla(c, p);
            let toplam = d.parcacik as f64 * d.gorev_orani;
            let hedef = c as f64 * f64::from(p) / 100.0;
            assert!(d.parcacik >= 1 && d.parcacik <= c);
            assert!(d.gorev_orani > 0.0 && d.gorev_orani <= 1.0);
            assert!(
                (toplam - hedef).abs() < 1e-9 || d.gorev_orani == 0.0001 || toplam > hedef,
                "{c} {p}"
            );
        }
    }

    #[test]
    fn tek_cekirdek_tavani_asmaz() {
        let d = duzenek_hesapla(1, 15);
        assert_eq!(d.parcacik, 1);
        assert!((d.gorev_orani - 0.15).abs() < 1e-9);
    }

    #[test]
    fn is_nesnesi_yer_tutucu_hata_verir() {
        assert!(is_nesnesi::uygula(10).is_err());
        assert!(!uygulanan_limitler().is_nesnesi);
    }
}
