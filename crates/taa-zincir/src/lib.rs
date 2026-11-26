//! taa-zincir: yalnız-ekleme özet zinciri, Merkle kökü ve imzalı mühür (DG2a; docs/dagitim-sozlesmesi.md).
//! Kayıt özeti, kendi içeriği ile bir önceki özetin SHA-256'sıdır; araya kayıt sokma, silme, değiştirme ve sıra
//! değişikliği zincir doğrulamasında yakalanır. Mühür (sayı, son özet, Merkle kökü, zaman) ed25519 ile imzalanır;
//! kısaltılmış ya da uzatılmış defter mühürle eşleşmez. Elle şifreleme yok: yalnız sha2 ve taa-imza.
//! Sahip: Yusuf. yazan: [Claude#dg2a-islem-gucu 2026-11-26]

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use taa_imza::{GenelAnahtar, Imza, OzelAnahtar};

pub const OZELLIK: &str = "zincir";
/// İlk kaydın `onceki` değeri: 64 sıfır.
pub const KOK_ONCEKI: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kayit {
    pub sira: u64,
    pub onceki: String,
    pub zaman: String,
    pub tur: String,
    pub veri: Value,
    pub ozet: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hata {
    /// Zincirin `sira` numaralı kaydında sorun var.
    Zincir { sira: u64, neden: String },
    /// Mühür geçersiz ya da defterle eşleşmiyor.
    Muhur(String),
}

impl std::fmt::Display for Hata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Hata::Zincir { sira, neden } => write!(f, "zincir bozuk (kayıt {sira}): {neden}"),
            Hata::Muhur(m) => write!(f, "mühür geçersiz: {m}"),
        }
    }
}

fn hex(b: &[u8]) -> String {
    taa_imza::hex_kodla(b)
}

fn sha(parcalar: &[&[u8]]) -> [u8; 32] {
    let mut h = Sha256::new();
    for p in parcalar {
        h.update(p);
    }
    h.finalize().into()
}

/// Kayıt özeti: SHA-256(kanonik{sira, onceki, zaman, tur, veri}).
pub fn kayit_ozeti(sira: u64, onceki: &str, zaman: &str, tur: &str, veri: &Value) -> String {
    let v = json!({"sira": sira, "onceki": onceki, "zaman": zaman, "tur": tur, "veri": veri});
    hex(&sha(&[&taa_imza::kanonik_bayt(&v)]))
}

/// Merkle yaprağı (RFC 6962 ön eki 0x00): SHA-256(0x00 || kayıt özeti baytları).
fn yaprak(ozet_hex: &str) -> [u8; 32] {
    let b = taa_imza::hex_coz(ozet_hex).unwrap_or_default();
    sha(&[&[0x00], &b])
}

fn dugum(sol: &[u8; 32], sag: &[u8; 32]) -> [u8; 32] {
    sha(&[&[0x01], sol, sag])
}

/// RFC 6962 Merkle ağacı kökü. Boş liste: SHA-256("") (RFC 6962).
fn kok_hesapla(y: &[[u8; 32]]) -> [u8; 32] {
    match y.len() {
        0 => sha(&[]),
        1 => y[0],
        n => {
            let mut k = 1usize;
            while k * 2 < n {
                k *= 2;
            }
            dugum(&kok_hesapla(&y[..k]), &kok_hesapla(&y[k..]))
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Muhur {
    pub sayi: u64,
    pub son_ozet: String,
    pub merkle_kok: String,
    pub zaman: String,
    pub imza: Imza,
}

fn muhur_bayt(sayi: u64, son_ozet: &str, kok: &str, zaman: &str) -> Vec<u8> {
    taa_imza::kanonik_bayt(&json!({
        "tur": "taa-zincir-muhur@1", "sayi": sayi, "son_ozet": son_ozet, "merkle_kok": kok, "zaman": zaman
    }))
}

/// Yalnız-ekleme defter (bellekte; kalıcılık çağıranın işidir: kayıtlar JSONL olarak yazılıp `kayitlardan` ile yüklenir).
#[derive(Debug, Clone, Default)]
pub struct Defter {
    kayitlar: Vec<Kayit>,
}

impl Defter {
    pub fn yeni() -> Self {
        Self::default()
    }

    /// Dışarıdan yüklenen kayıtlar zincir doğrulamasından geçmeden kabul edilmez.
    pub fn kayitlardan(kayitlar: Vec<Kayit>) -> Result<Self, Hata> {
        let d = Defter { kayitlar };
        d.dogrula()?;
        Ok(d)
    }

    pub fn len(&self) -> usize {
        self.kayitlar.len()
    }

    pub fn is_empty(&self) -> bool {
        self.kayitlar.is_empty()
    }

    pub fn kayitlar(&self) -> &[Kayit] {
        &self.kayitlar
    }

    pub fn son_ozet(&self) -> String {
        self.kayitlar
            .last()
            .map(|k| k.ozet.clone())
            .unwrap_or_else(|| KOK_ONCEKI.to_string())
    }

    /// Yeni kayıt ekler; `zaman` çağıranın verdiği RFC 3339 UTC metnidir (saat kaynağı çağırandadır, test edilebilir).
    pub fn ekle(&mut self, zaman: &str, tur: &str, veri: Value) -> &Kayit {
        let sira = self.kayitlar.len() as u64 + 1;
        let onceki = self.son_ozet();
        let ozet = kayit_ozeti(sira, &onceki, zaman, tur, &veri);
        self.kayitlar.push(Kayit {
            sira,
            onceki,
            zaman: zaman.to_string(),
            tur: tur.to_string(),
            veri,
            ozet,
        });
        self.kayitlar.last().expect("az önce eklendi")
    }

    /// Tüm zinciri baştan doğrular: sıra 1..n, önceki bağlantı ve özet.
    pub fn dogrula(&self) -> Result<(), Hata> {
        let mut onceki = KOK_ONCEKI.to_string();
        for (i, k) in self.kayitlar.iter().enumerate() {
            let beklenen = i as u64 + 1;
            if k.sira != beklenen {
                return Err(Hata::Zincir { sira: k.sira, neden: format!("sıra {beklenen} olmalıydı") });
            }
            if k.onceki != onceki {
                return Err(Hata::Zincir { sira: k.sira, neden: "önceki özet bağlantısı kopuk".into() });
            }
            if kayit_ozeti(k.sira, &k.onceki, &k.zaman, &k.tur, &k.veri) != k.ozet {
                return Err(Hata::Zincir { sira: k.sira, neden: "özet içerikle uyuşmuyor".into() });
            }
            onceki = k.ozet.clone();
        }
        Ok(())
    }

    /// İlk `n` kaydın Merkle kökü (onaltılık).
    pub fn merkle_kok(&self, n: usize) -> String {
        let n = n.min(self.kayitlar.len());
        let y: Vec<[u8; 32]> = self.kayitlar[..n].iter().map(|k| yaprak(&k.ozet)).collect();
        hex(&kok_hesapla(&y))
    }

    /// Defterin şu anki durumunu imzalar.
    pub fn muhurle(&self, o: &OzelAnahtar, zaman: &str) -> Muhur {
        let kok = self.merkle_kok(self.len());
        let son = self.son_ozet();
        let imza = o.imzala(&muhur_bayt(self.len() as u64, &son, &kok, zaman));
        Muhur { sayi: self.len() as u64, son_ozet: son, merkle_kok: kok, zaman: zaman.to_string(), imza }
    }

    /// Mührü doğrular: imza geçerli olmalı ve mührün söylediği ilk `sayi` kayıt bu defterde aynı kökü vermeli.
    /// Defter mühürden uzunsa (sonradan eklenmiş kayıtlar) geçerlidir; kısaysa (silinmiş kayıt) reddedilir.
    pub fn muhur_dogrula(&self, g: &GenelAnahtar, m: &Muhur) -> Result<(), Hata> {
        g.dogrula(&muhur_bayt(m.sayi, &m.son_ozet, &m.merkle_kok, &m.zaman), &m.imza)
            .map_err(|e| Hata::Muhur(format!("imza: {e:?}")))?;
        self.dogrula()?;
        let n = m.sayi as usize;
        if self.len() < n {
            return Err(Hata::Muhur(format!("defter mühürden kısa ({} < {n}): kayıt silinmiş", self.len())));
        }
        let son = if n == 0 { KOK_ONCEKI.to_string() } else { self.kayitlar[n - 1].ozet.clone() };
        if son != m.son_ozet {
            return Err(Hata::Muhur("son özet defterle uyuşmuyor".into()));
        }
        if self.merkle_kok(n) != m.merkle_kok {
            return Err(Hata::Muhur("Merkle kökü defterle uyuşmuyor".into()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn anahtar() -> OzelAnahtar {
        let d = std::env::temp_dir().join(format!("taa-zincir-test-{}-{:?}", std::process::id(), std::thread::current().id()));
        let _ = std::fs::create_dir_all(&d);
        let (o, g) = (d.join("ozel"), d.join("genel"));
        let _ = std::fs::remove_file(&o);
        let _ = std::fs::remove_file(&g);
        taa_imza::anahtar_uret(&o, &g).unwrap();
        OzelAnahtar::dosyadan(&o).unwrap().0
    }

    fn dolu(n: usize) -> Defter {
        let mut d = Defter::yeni();
        for i in 0..n {
            d.ekle(&format!("2026-11-26T02:00:{i:02}Z"), "is.basladi", json!({"is": i}));
        }
        d
    }

    #[test]
    fn sha256_bilinen_yanit() {
        assert_eq!(hex(&sha(&[b"abc"])), "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
    }

    #[test]
    fn merkle_rfc6962_bos_ve_tek() {
        // RFC 6962: boş ağacın kökü SHA-256("").
        assert_eq!(dolu(0).merkle_kok(0), "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855");
        let d = dolu(1);
        assert_eq!(d.merkle_kok(1), hex(&yaprak(&d.kayitlar()[0].ozet)));
    }

    #[test]
    fn merkle_uc_yaprak_rfc_bolumu() {
        let d = dolu(3);
        let y: Vec<_> = d.kayitlar().iter().map(|k| yaprak(&k.ozet)).collect();
        let beklenen = dugum(&dugum(&y[0], &y[1]), &y[2]);
        assert_eq!(d.merkle_kok(3), hex(&beklenen));
    }

    #[test]
    fn temiz_zincir_dogrulanir_ve_bagli_kalir() {
        let d = dolu(5);
        assert!(d.dogrula().is_ok());
        assert_eq!(d.kayitlar()[0].onceki, KOK_ONCEKI);
        assert_eq!(d.kayitlar()[3].onceki, d.kayitlar()[2].ozet);
    }

    #[test]
    fn icerik_degisikligi_yakalanir() {
        let mut k = dolu(4).kayitlar().to_vec();
        k[1].veri = json!({"is": 99});
        assert!(matches!(Defter::kayitlardan(k), Err(Hata::Zincir { sira: 2, .. })));
    }

    #[test]
    fn silme_yakalanir() {
        let mut k = dolu(4).kayitlar().to_vec();
        k.remove(1);
        assert!(Defter::kayitlardan(k).is_err());
    }

    #[test]
    fn araya_kayit_sokma_yakalanir() {
        let d = dolu(3);
        let mut k = d.kayitlar().to_vec();
        let sahte = Kayit {
            sira: 2,
            onceki: k[0].ozet.clone(),
            zaman: "2026-11-26T02:00:00Z".into(),
            tur: "sahte".into(),
            veri: json!({}),
            ozet: kayit_ozeti(2, &k[0].ozet, "2026-11-26T02:00:00Z", "sahte", &json!({})),
        };
        k.insert(1, sahte);
        assert!(Defter::kayitlardan(k).is_err());
    }

    #[test]
    fn muhur_dogrulanir_ve_uzayan_defter_gecerli() {
        let o = anahtar();
        let g = o.genel();
        let mut d = dolu(3);
        let m = d.muhurle(&o, "2026-11-26T03:00:00Z");
        assert!(d.muhur_dogrula(&g, &m).is_ok());
        d.ekle("2026-11-26T03:01:00Z", "is.bitti", json!({}));
        assert!(d.muhur_dogrula(&g, &m).is_ok());
    }

    #[test]
    fn kisalan_defter_muhurle_uyusmaz() {
        let o = anahtar();
        let g = o.genel();
        let d = dolu(4);
        let m = d.muhurle(&o, "2026-11-26T03:00:00Z");
        let kisa = Defter::kayitlardan(d.kayitlar()[..3].to_vec()).unwrap();
        assert!(matches!(kisa.muhur_dogrula(&g, &m), Err(Hata::Muhur(_))));
    }

    #[test]
    fn yeniden_yazilmis_defter_muhurle_uyusmaz() {
        let o = anahtar();
        let g = o.genel();
        let d = dolu(3);
        let m = d.muhurle(&o, "2026-11-26T03:00:00Z");
        // Aynı uzunlukta, tutarlı ama farklı geçmiş (zinciri baştan yeniden üretme saldırısı).
        let mut baska = Defter::yeni();
        for i in 0..3 {
            baska.ekle(&format!("2026-11-26T02:00:{i:02}Z"), "is.basladi", json!({"is": i + 100}));
        }
        assert!(baska.dogrula().is_ok());
        assert!(matches!(baska.muhur_dogrula(&g, &m), Err(Hata::Muhur(_))));
    }

    #[test]
    fn muhur_alani_degistirilirse_imza_dusur() {
        let o = anahtar();
        let g = o.genel();
        let d = dolu(3);
        let mut m = d.muhurle(&o, "2026-11-26T03:00:00Z");
        m.sayi = 2;
        assert!(matches!(d.muhur_dogrula(&g, &m), Err(Hata::Muhur(_))));
    }

    #[test]
    fn baska_anahtarla_imza_reddedilir() {
        let o = anahtar();
        let d = dolu(2);
        let m = d.muhurle(&o, "2026-11-26T03:00:00Z");
        let dir = std::env::temp_dir().join(format!("taa-zincir-ikinci-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let (oy, gy) = (dir.join("o"), dir.join("g"));
        let _ = std::fs::remove_file(&oy);
        let _ = std::fs::remove_file(&gy);
        let baska = taa_imza::anahtar_uret(&oy, &gy).unwrap();
        assert!(d.muhur_dogrula(&baska, &m).is_err());
    }
}
