//! taa-olcum: DG1a ölçüm aracı. Bilgisayarın envanterini (rastgele işçi numarasıyla, kişi/bilgisayar adı/seri yok) ve pay
//! basamaklarında (yüzde 5, 8, 10, 12, 15; ayrı ayrı, ısınma sonrası ortanca-3) hız ile ön plan gecikmesini ölçer.
//! İş yükü şimdilik OCR değil, sentetik CPU işidir; eşikler `docs/olcum/YONTEM.md` içinde ölçümden önce kilitlidir.
//! Linux ve Windows'ta derlenir (cfg); Windows'ta denenmedi, iş nesnesi limiti yer tutucudur.
//! Sahip: Yusuf. yazan: [Claude#olcum 2026-11-26]

pub mod envanter;
pub mod is;
pub mod limit;
pub mod olcum;
pub mod rapor;
pub mod yontem;

pub const OZELLIK: &str = "olcum";
