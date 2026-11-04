//! TaaDön Rust çekirdeği (kitaplık kısmı): güvenli ZIP/GZIP işlemleri ve çıktı imza denetimi.
//! Sahip: Yusuf (TaaDön projesi). yazan: [Claude#taadon-rust-asama1 2026-11-04], [Claude#taadon-rust-asama2 2026-11-04] (ad: taa-donustur).
//! Değişiklik: Yusuf'un kuralları (ORTAK-AJAN-KURALLARI.md) çerçevesinde; kurallar Node'daki
//! src/motorlar/arsiv.js ve src/donustur.js (CIKTI_IMZALARI) ile birebir aynı tutulmalıdır.

pub mod arsiv;
pub mod imza;

use std::io::Write;
use std::path::Path;

/// Kullanıcıya gösterilecek Türkçe hata metni.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hata(pub String);

impl Hata {
    pub fn yeni(m: impl Into<String>) -> Self {
        Hata(m.into())
    }
}

impl std::fmt::Display for Hata {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

pub type Sonuc<T> = Result<T, Hata>;

/// stderr'e `ILERLEME <yuzde>` satırları yazar (yalnız değer değişince).
pub struct Ilerleme {
    etkin: bool,
    son: i64,
}

impl Ilerleme {
    pub fn yeni(etkin: bool) -> Self {
        Ilerleme { etkin, son: -1 }
    }
    pub fn kapali() -> Self {
        Self::yeni(false)
    }
    /// islenen/toplam oranını 0..=95 aralığına çevirip bildirir (Node ile aynı tavan).
    pub fn oran(&mut self, islenen: u64, toplam: u64) {
        if toplam == 0 {
            return;
        }
        let y = ((islenen as f64 / toplam as f64) * 95.0).floor() as i64;
        self.bildir(y.clamp(0, 95));
    }
    pub fn bildir(&mut self, yuzde: i64) {
        if !self.etkin || yuzde == self.son {
            return;
        }
        self.son = yuzde;
        let mut e = std::io::stderr().lock();
        let _ = writeln!(e, "ILERLEME {yuzde}");
        let _ = e.flush();
    }
}

/// Node `path.extname` benzeri: son noktadan sonraki uzantı (noktalı), baştaki noktalar sayılmaz.
pub fn node_extname(ad: &str) -> &str {
    let govde = ad.trim_start_matches('.');
    let bas = ad.len() - govde.len();
    match govde.rfind('.') {
        Some(j) => &ad[bas + j..],
        None => "",
    }
}

/// Node `yollar.uzantiAl`: noktasız, küçük harf uzantı.
pub fn uzanti_al(yol: &Path) -> String {
    let ad = yol
        .file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    node_extname(&ad).trim_start_matches('.').to_lowercase()
}

/// Windows `\?\` önekini kullanıcıya gösterilecek yoldan kaldırır.
pub fn gosterim_yolu(yol: &Path) -> String {
    let s = yol.to_string_lossy().into_owned();
    if let Some(r) = s.strip_prefix(r"\?\UNC\") {
        return format!(r"\{r}");
    }
    if let Some(r) = s.strip_prefix(r"\?\") {
        return r.to_string();
    }
    s
}
