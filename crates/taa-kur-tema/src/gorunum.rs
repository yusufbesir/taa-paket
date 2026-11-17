// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz 2026-11-17]
//! Görünüm tercihleri: `<veri kökü>\kurucu\gorunum.json` (KARAR §5 "Kalıcı saklama").
//!
//! Düz JSON elle yazılır/okunur (serde yok). Okurken bilinmeyen anahtar yok sayılır; dosya yoksa
//! ya da bozuksa varsayılanlar döner, hata gösterilmez. Yazma atomiktir: geçici dosya + rename.
//! Kayıt defteri kullanılmaz.
//!
//! Not (KARAR §5): sihirbaz başka bir hesapla yönetici olarak yükseltilirse `%APPDATA%` o hesabın
//! profiline gider. Böyle durumda çağıran taraf etkileşimli kullanıcının yolunu bulup
//! [`Gorunum::yukle_yoldan`] / [`Gorunum::kaydet_yola`] ile o yolu vermelidir.

use std::io::Write;
use std::path::{Path, PathBuf};

use crate::{hex, hex_oku, Mod, Tema};

/// Dosya biçim sürümü.
pub const GORUNUM_SURUM: u32 = 1;

/// Görünüm tercihleri.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gorunum {
    /// Dosya biçim sürümü (yazarken her zaman [`GORUNUM_SURUM`]).
    pub surum: u32,
    /// Kullanıcının seçtiği tema; `None` = seçmedi (İlmek, Yüksek Karşıtlık açıksa Mezura).
    pub tema: Option<Tema>,
    /// Görünüm modu (dosyada `"mod"`); varsayılan Açık.
    pub modu: Mod,
    /// Tema başına özel vurgu (`Tema::sira()` indisli); `None` = temanın kendi rengi.
    pub ozel_vurgu: [Option<u32>; 3],
    /// `None` = Windows "animasyonları göster" ayarını izle; `Some` = kullanıcı seçimi.
    pub hareketi_azalt: Option<bool>,
    /// Tanıtım bir kez gösterildi mi? Varsayılana dönüşte korunur.
    pub tanitim_goruldu: bool,
}

impl Default for Gorunum {
    fn default() -> Self {
        Gorunum {
            surum: GORUNUM_SURUM,
            tema: None,
            modu: Mod::Acik,
            ozel_vurgu: [None; 3],
            hareketi_azalt: None,
            tanitim_goruldu: false,
        }
    }
}

impl Gorunum {
    /// Kullanılacak tema: seçim varsa o; yoksa Yüksek Karşıtlık açıkken Mezura, değilse İlmek.
    pub fn etkin_tema(&self, yuksek_karsitlik: bool) -> Tema {
        match self.tema {
            Some(t) => t,
            None if yuksek_karsitlik => Tema::Mezura,
            None => Tema::Ilmek,
        }
    }

    /// Hareket azaltılmalı mı? Kullanıcı seçmediyse Windows "animasyonları göster"in tersi
    /// (`SPI_GETCLIENTAREAANIMATION`).
    pub fn hareketi_azalt_mi(&self, sistem_animasyon_acik: bool) -> bool {
        self.hareketi_azalt.unwrap_or(!sistem_animasyon_acik)
    }

    /// O temanın özel vurgusu.
    pub fn ozel_vurgu(&self, tema: Tema) -> Option<u32> {
        self.ozel_vurgu[tema.sira()]
    }

    /// O temanın özel vurgusunu ayarlar (`None` = "Temanın rengine dön").
    pub fn ozel_vurgu_ata(&mut self, tema: Tema, renk: Option<u32>) {
        self.ozel_vurgu[tema.sira()] = renk.map(|r| r & 0xFF_FFFF);
    }

    /// "Varsayılana dön": yalnız görünüm alanları sıfırlanır; `tanitim_goruldu` korunur.
    pub fn varsayilana_don(&mut self) {
        let tanitim = self.tanitim_goruldu;
        *self = Gorunum {
            tanitim_goruldu: tanitim,
            ..Gorunum::default()
        };
    }

    /// JSON metni (girintili, sonunda satır sonu).
    pub fn json(&self) -> String {
        let renk = |r: Option<u32>| match r {
            Some(r) => format!("\"{}\"", hex(r)),
            None => "null".to_string(),
        };
        let tema = match self.tema {
            Some(t) => format!("\"{}\"", t.ad()),
            None => "null".to_string(),
        };
        let azalt = match self.hareketi_azalt {
            Some(b) => b.to_string(),
            None => "null".to_string(),
        };
        let mut s = String::new();
        s.push_str("{\n");
        s.push_str(&format!("  \"surum\": {},\n", GORUNUM_SURUM));
        s.push_str(&format!("  \"tema\": {},\n", tema));
        s.push_str(&format!("  \"mod\": \"{}\",\n", self.modu.ad()));
        s.push_str("  \"ozel_vurgu\": {");
        for (i, t) in Tema::HEPSI.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!(
                "\"{}\": {}",
                t.ad(),
                renk(self.ozel_vurgu[t.sira()])
            ));
        }
        s.push_str("},\n");
        s.push_str(&format!("  \"hareketi_azalt\": {},\n", azalt));
        s.push_str(&format!(
            "  \"tanitim_goruldu\": {}\n",
            self.tanitim_goruldu
        ));
        s.push_str("}\n");
        s
    }

    /// JSON metninden okur. Bozuk metin = varsayılan; bilinmeyen anahtar ve tanınmayan değer
    /// yok sayılır (o alan varsayılan kalır).
    pub fn json_oku(metin: &str) -> Gorunum {
        let mut g = Gorunum::default();
        let alanlar = match json::ayristir(metin) {
            Some(json::Deger::Nesne(a)) => a,
            _ => return g,
        };
        for (k, d) in &alanlar {
            match (k.as_str(), d) {
                ("surum", json::Deger::Sayi(n)) if *n >= 0.0 && *n <= u32::MAX as f64 => {
                    g.surum = *n as u32
                }
                ("tema", json::Deger::Metin(s)) => g.tema = Tema::addan(s),
                ("mod", json::Deger::Metin(s)) => g.modu = Mod::addan(s).unwrap_or_default(),
                ("ozel_vurgu", json::Deger::Nesne(v)) => {
                    for (tk, td) in v {
                        if let (Some(t), json::Deger::Metin(h)) = (Tema::addan(tk), td) {
                            g.ozel_vurgu[t.sira()] = hex_oku(h);
                        }
                    }
                }
                ("hareketi_azalt", json::Deger::Bool(b)) => g.hareketi_azalt = Some(*b),
                ("tanitim_goruldu", json::Deger::Bool(b)) => g.tanitim_goruldu = *b,
                _ => {}
            }
        }
        g
    }

    /// Dosyadan okur; dosya yoksa/okunamazsa/bozuksa varsayılan.
    pub fn yukle_yoldan(yol: &Path) -> Gorunum {
        match std::fs::read(yol) {
            Ok(b) => Gorunum::json_oku(&String::from_utf8_lossy(&b)),
            Err(_) => Gorunum::default(),
        }
    }

    /// Dosyaya atomik yazar: aynı klasörde geçici dosya, diske boşalt, sonra yeniden adlandır.
    /// Klasör yoksa oluşturulur.
    pub fn kaydet_yola(&self, yol: &Path) -> std::io::Result<()> {
        if let Some(k) = yol.parent() {
            if !k.as_os_str().is_empty() {
                std::fs::create_dir_all(k)?;
            }
        }
        let mut ad = yol
            .file_name()
            .map(|a| a.to_os_string())
            .unwrap_or_default();
        ad.push(format!(".{}.gecici", std::process::id()));
        let gecici = yol.with_file_name(ad);
        let sonuc = (|| {
            let mut f = std::fs::File::create(&gecici)?;
            f.write_all(self.json().as_bytes())?;
            f.sync_all()?;
            drop(f);
            std::fs::rename(&gecici, yol)
        })();
        if sonuc.is_err() {
            let _ = std::fs::remove_file(&gecici);
        }
        sonuc
    }

    /// Varsayılan yoldan ([`gorunum_yolu`]) okur.
    pub fn yukle() -> Gorunum {
        match gorunum_yolu() {
            Some(y) => Gorunum::yukle_yoldan(&y),
            None => Gorunum::default(),
        }
    }

    /// Varsayılan yola yazar.
    pub fn kaydet(&self) -> std::io::Result<()> {
        match gorunum_yolu() {
            Some(y) => self.kaydet_yola(&y),
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "TAA veri kökü bulunamadı (TAA_VERI_KOK ve APPDATA tanımsız)",
            )),
        }
    }
}

/// TAA ortak veri kökü: `TAA_VERI_KOK`, yoksa Windows'ta `%APPDATA%\TAA\veri`
/// (Linux'ta test için `XDG_DATA_HOME/TAA/veri` ya da `~/.local/share/TAA/veri`).
/// taa-yol / taa-cihaz ile aynı kural.
pub fn veri_koku() -> Option<PathBuf> {
    let ev = |a: &str| std::env::var(a).ok().filter(|s| !s.trim().is_empty());
    if let Some(k) = ev("TAA_VERI_KOK") {
        return Some(PathBuf::from(k));
    }
    if cfg!(windows) {
        return ev("APPDATA").map(|a| PathBuf::from(a).join("TAA").join("veri"));
    }
    if let Some(x) = ev("XDG_DATA_HOME") {
        return Some(PathBuf::from(x).join("TAA").join("veri"));
    }
    ev("HOME").map(|h| PathBuf::from(h).join(".local/share/TAA/veri"))
}

/// `<veri kökü>\kurucu\gorunum.json`.
pub fn gorunum_yolu() -> Option<PathBuf> {
    veri_koku().map(|k| k.join("kurucu").join("gorunum.json"))
}

/// Küçük, yalnız bu dosya için JSON okuyucu (RFC 8259 alt kümesi değil, tamamı; derinlik sınırlı).
pub(crate) mod json {
    #[derive(Clone, Debug, PartialEq)]
    pub enum Deger {
        Bos,
        Bool(bool),
        Sayi(f64),
        Metin(String),
        Dizi(Vec<Deger>),
        Nesne(Vec<(String, Deger)>),
    }

    const EN_DERIN: usize = 32;

    pub fn ayristir(s: &str) -> Option<Deger> {
        let s = s.strip_prefix('\u{FEFF}').unwrap_or(s);
        let mut o = Okuyucu {
            b: s.as_bytes(),
            i: 0,
        };
        let d = o.deger(0)?;
        o.bosluk();
        if o.i == o.b.len() {
            Some(d)
        } else {
            None
        }
    }

    struct Okuyucu<'a> {
        b: &'a [u8],
        i: usize,
    }

    impl Okuyucu<'_> {
        fn bosluk(&mut self) {
            while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
                self.i += 1;
            }
        }

        fn bak(&self) -> Option<u8> {
            self.b.get(self.i).copied()
        }

        fn sabit(&mut self, k: &str, d: Deger) -> Option<Deger> {
            if self.b[self.i..].starts_with(k.as_bytes()) {
                self.i += k.len();
                Some(d)
            } else {
                None
            }
        }

        fn deger(&mut self, derinlik: usize) -> Option<Deger> {
            if derinlik > EN_DERIN {
                return None;
            }
            self.bosluk();
            match self.bak()? {
                b'{' => self.nesne(derinlik),
                b'[' => self.dizi(derinlik),
                b'"' => self.metin().map(Deger::Metin),
                b't' => self.sabit("true", Deger::Bool(true)),
                b'f' => self.sabit("false", Deger::Bool(false)),
                b'n' => self.sabit("null", Deger::Bos),
                b'-' | b'0'..=b'9' => self.sayi(),
                _ => None,
            }
        }

        fn nesne(&mut self, derinlik: usize) -> Option<Deger> {
            self.i += 1;
            let mut v = Vec::new();
            self.bosluk();
            if self.bak()? == b'}' {
                self.i += 1;
                return Some(Deger::Nesne(v));
            }
            loop {
                self.bosluk();
                if self.bak()? != b'"' {
                    return None;
                }
                let k = self.metin()?;
                self.bosluk();
                if self.bak()? != b':' {
                    return None;
                }
                self.i += 1;
                let d = self.deger(derinlik + 1)?;
                v.push((k, d));
                self.bosluk();
                match self.bak()? {
                    b',' => self.i += 1,
                    b'}' => {
                        self.i += 1;
                        return Some(Deger::Nesne(v));
                    }
                    _ => return None,
                }
            }
        }

        fn dizi(&mut self, derinlik: usize) -> Option<Deger> {
            self.i += 1;
            let mut v = Vec::new();
            self.bosluk();
            if self.bak()? == b']' {
                self.i += 1;
                return Some(Deger::Dizi(v));
            }
            loop {
                v.push(self.deger(derinlik + 1)?);
                self.bosluk();
                match self.bak()? {
                    b',' => self.i += 1,
                    b']' => {
                        self.i += 1;
                        return Some(Deger::Dizi(v));
                    }
                    _ => return None,
                }
            }
        }

        fn hex4(&mut self) -> Option<u32> {
            let p = self.b.get(self.i..self.i + 4)?;
            let s = std::str::from_utf8(p).ok()?;
            let n = u32::from_str_radix(s, 16).ok()?;
            self.i += 4;
            Some(n)
        }

        fn metin(&mut self) -> Option<String> {
            self.i += 1; // "
            let mut s: Vec<u8> = Vec::new();
            loop {
                let c = self.bak()?;
                self.i += 1;
                match c {
                    b'"' => return String::from_utf8(s).ok(),
                    b'\\' => {
                        let e = self.bak()?;
                        self.i += 1;
                        let ch = match e {
                            b'"' => '"',
                            b'\\' => '\\',
                            b'/' => '/',
                            b'b' => '\u{8}',
                            b'f' => '\u{C}',
                            b'n' => '\n',
                            b'r' => '\r',
                            b't' => '\t',
                            b'u' => {
                                let u = self.hex4()?;
                                if (0xD800..0xDC00).contains(&u) {
                                    if self.b.get(self.i..self.i + 2) != Some(b"\\u") {
                                        return None;
                                    }
                                    self.i += 2;
                                    let alt = self.hex4()?;
                                    if !(0xDC00..0xE000).contains(&alt) {
                                        return None;
                                    }
                                    char::from_u32(0x10000 + ((u - 0xD800) << 10) + (alt - 0xDC00))?
                                } else {
                                    char::from_u32(u)?
                                }
                            }
                            _ => return None,
                        };
                        let mut tmp = [0u8; 4];
                        s.extend_from_slice(ch.encode_utf8(&mut tmp).as_bytes());
                    }
                    0x00..=0x1F => return None,
                    _ => s.push(c),
                }
            }
        }

        fn sayi(&mut self) -> Option<Deger> {
            let bas = self.i;
            while let Some(c) = self.bak() {
                if c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.' | b'e' | b'E') {
                    self.i += 1;
                } else {
                    break;
                }
            }
            let s = std::str::from_utf8(&self.b[bas..self.i]).ok()?;
            s.parse::<f64>()
                .ok()
                .filter(|n| n.is_finite())
                .map(Deger::Sayi)
        }
    }
}
