// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! Güncelleme kaynağı soyutlaması: yerel klasör (`YerelKaynak`) ve HTTP(S) (`HttpKaynak`, T4).
//! HTTP için yeni crate yok: indirme `curl` komutuyla yapılır (Windows 10 1803+ ve CI'da hazır).

use crate::yayin::MAKS_YAYIN_BAYT;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use taa_kur::paket::guvenli_coz;
use taa_kur::KurHata;

pub trait Kaynak {
    /// `yayin.json` ham baytları.
    fn yayin_oku(&self) -> Result<Vec<u8>, KurHata>;
    /// `url` ile gösterilen dosyayı `hedef` yoluna YENİ dosya olarak yazar (var olan ezilmez); yazılan bayt sayısını döner.
    /// Gerçek uzunluk `beklenen_boyut`u aşarsa durur ve hata verir (şişirilmiş indirmeye karşı).
    fn getir(&self, url: &str, hedef: &Path, beklenen_boyut: u64) -> Result<u64, KurHata>;
}

/// Klasör kaynağı: `<klasor>/yayin.json` ve `url` olarak klasöre göreli yollar.
#[derive(Debug, Clone)]
pub struct YerelKaynak {
    klasor: PathBuf,
}

impl YerelKaynak {
    pub fn yeni(klasor: impl Into<PathBuf>) -> YerelKaynak {
        YerelKaynak {
            klasor: klasor.into(),
        }
    }
}

impl Kaynak for YerelKaynak {
    fn yayin_oku(&self) -> Result<Vec<u8>, KurHata> {
        let yol = self.klasor.join("yayin.json");
        let m = fs::metadata(&yol).map_err(|e| {
            KurHata::Yok(format!(
                "yayin.json bulunamadı ({}): {e}. Öneri: kaynak klasörünü ve kur.toml'daki kaynak_klasoru değerini denetleyin.",
                yol.display()
            ))
        })?;
        if !m.is_file() || m.len() > MAKS_YAYIN_BAYT {
            return Err(KurHata::Dogrulama(
                "yayin.json normal dosya değil ya da çok büyük. Öneri: yayıncıdan yeniden isteyin."
                    .into(),
            ));
        }
        Ok(fs::read(&yol)?)
    }

    fn getir(&self, url: &str, hedef: &Path, beklenen_boyut: u64) -> Result<u64, KurHata> {
        // Yol kaçışı, mutlak yol ve sembolik bağ reddedilir (paket yolu kurallarıyla aynı).
        let kaynak = guvenli_coz(&self.klasor, url).map_err(|m| {
            KurHata::Dogrulama(format!("Kaynak yolu güvenli değil ({m}). Öneri: yayin.json url alanı kaynak klasörüne göreli olmalı."))
        })?;
        let mut oku = fs::File::open(&kaynak)
            .map_err(|e| {
                KurHata::Yok(format!(
                    "{url} kaynakta okunamadı: {e}. Öneri: yayın klasörünü denetleyin."
                ))
            })?
            .take(beklenen_boyut.saturating_add(1));
        let mut yaz = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(hedef)?;
        let n = std::io::copy(&mut oku, &mut yaz)?;
        yaz.flush()?;
        yaz.sync_all()?;
        if n > beklenen_boyut {
            return Err(KurHata::Dogrulama(format!(
                "{url} yayin.json'daki boyuttan ({beklenen_boyut} bayt) büyük. Öneri: yayını yeniden üretin."
            )));
        }
        Ok(n)
    }
}

/// HTTP(S) kaynağı: `<taban>/yayin.json` ve `url` olarak tabana göreli ya da mutlak `http(s)://` adresler.
/// Aktarım `curl` ile: `--fail` (4xx/5xx hata), `--location` (GitHub Releases yönlendirmesi), `--max-filesize`
/// (bildirilen boyut + 1; aşılırsa indirme durur) ve `--proto =http,https`. Yerel ağ (127.0.0.1/localhost) için
/// vekil sunucu atlanır. Çıkış kodu/hata metni Türkçe tek cümleye çevrilir.
#[derive(Debug, Clone)]
pub struct HttpKaynak {
    taban: String,
}

/// Saniye cinsinden en uzun indirme süresi (curl `--max-time`).
const EN_UZUN_SURE_SN: u64 = 600;

pub fn http_url_mu(s: &str) -> bool {
    let k = s.trim().to_ascii_lowercase();
    k.starts_with("http://") || k.starts_with("https://")
}

impl HttpKaynak {
    /// `taban`: `https://sunucu/yol` (sondaki `/` önemsiz). `http://`/`https://` dışı reddedilir.
    pub fn yeni(taban: &str) -> Result<HttpKaynak, KurHata> {
        let t = taban.trim();
        if !http_url_mu(t) || t.contains(char::is_whitespace) {
            return Err(KurHata::Dogrulama(format!(
                "Kaynak adresi http:// ya da https:// ile başlamalı ({t}). Öneri: kur.toml kaynak_url değerini düzeltin."
            )));
        }
        Ok(HttpKaynak {
            taban: t.trim_end_matches('/').to_string(),
        })
    }

    pub fn taban(&self) -> &str {
        &self.taban
    }

    fn tam_url(&self, url: &str) -> Result<String, KurHata> {
        let u = url.trim();
        if u.is_empty() || u.contains(char::is_whitespace) {
            return Err(KurHata::Dogrulama(
                "yayin.json url alanı boş ya da boşluk içeriyor. Öneri: yayını yeniden üretin."
                    .into(),
            ));
        }
        if http_url_mu(u) {
            return Ok(u.to_string());
        }
        if u.contains("://") || u.starts_with('/') || u.starts_with('\\') {
            return Err(KurHata::Dogrulama(format!(
                "yayin.json url alanı ({u}) http(s) ya da tabana göreli olmalı. Öneri: yayını yeniden üretin."
            )));
        }
        Ok(format!("{}/{}", self.taban, u))
    }

    fn komut(url: &str, en_fazla: u64) -> Command {
        let mut c = Command::new("curl");
        c.args([
            "--fail",
            "--silent",
            "--show-error",
            "--location",
            "--max-redirs",
            "5",
        ])
        .args(["--proto", "=http,https", "--proto-redir", "=http,https"])
        .args(["--max-time", &EN_UZUN_SURE_SN.to_string()])
        .args(["--max-filesize", &en_fazla.to_string()])
        .args(["--user-agent", "taa-guncelle"]);
        if yerel_mi(url) {
            c.args(["--noproxy", "*"]);
        }
        c.arg(url);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW: pencere uygulamasından çağrılınca konsol açılmaz
        }
        c
    }

    fn hata(url: &str, cikis: Option<i32>, stderr: &[u8]) -> KurHata {
        let m = String::from_utf8_lossy(stderr)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let m = if m.is_empty() {
            "ayrıntı yok".to_string()
        } else {
            m
        };
        match cikis {
            Some(22) => KurHata::Yok(format!(
                "{url} sunucuda bulunamadı ya da reddedildi ({m}). Öneri: kaynak adresini ve yayını denetleyin."
            )),
            Some(63) => KurHata::Dogrulama(format!(
                "{url} yayin.json'daki boyuttan büyük ({m}). Öneri: yayını yeniden üretin."
            )),
            Some(6) | Some(7) | Some(28) | Some(35) | Some(60) => KurHata::Diger(format!(
                "{url} adresine ulaşılamadı ({m}). Öneri: ağ bağlantısını denetleyip yeniden deneyin."
            )),
            Some(k) => KurHata::Diger(format!(
                "{url} indirilemedi (curl çıkış {k}: {m}). Öneri: yeniden deneyin; sürerse adresi denetleyin."
            )),
            None => KurHata::Diger(format!(
                "{url} indirilirken curl kesildi ({m}). Öneri: yeniden deneyin."
            )),
        }
    }
}

fn yerel_mi(url: &str) -> bool {
    let k = url.to_ascii_lowercase();
    let govde = k
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let sunucu = govde.split(['/', '?', '#']).next().unwrap_or("");
    let sunucu = sunucu.rsplit('@').next().unwrap_or(sunucu);
    sunucu.starts_with("127.0.0.1")
        || sunucu.starts_with("localhost")
        || sunucu.starts_with("[::1]")
}

fn curl_yok(e: std::io::Error) -> KurHata {
    KurHata::Diger(format!(
        "curl çalıştırılamadı ({e}). Öneri: Windows 10 1803+ ile gelir; yoksa curl'ü kurun ya da --kaynak ile yerel klasör verin."
    ))
}

impl Kaynak for HttpKaynak {
    fn yayin_oku(&self) -> Result<Vec<u8>, KurHata> {
        let url = format!("{}/yayin.json", self.taban);
        let c = HttpKaynak::komut(&url, MAKS_YAYIN_BAYT)
            .output()
            .map_err(curl_yok)?;
        if !c.status.success() {
            return Err(HttpKaynak::hata(&url, c.status.code(), &c.stderr));
        }
        if c.stdout.len() as u64 > MAKS_YAYIN_BAYT {
            return Err(KurHata::Dogrulama(
                "yayin.json çok büyük. Öneri: yayıncıdan yeniden isteyin.".into(),
            ));
        }
        Ok(c.stdout)
    }

    fn getir(&self, url: &str, hedef: &Path, beklenen_boyut: u64) -> Result<u64, KurHata> {
        let tam = self.tam_url(url)?;
        if hedef.exists() {
            return Err(KurHata::Dogrulama(format!(
                "{} zaten var; üzerine yazılmaz. Öneri: indirilen/ klasörünü denetleyin.",
                hedef.display()
            )));
        }
        let c = HttpKaynak::komut(&tam, beklenen_boyut.saturating_add(1))
            .arg("--output")
            .arg(hedef)
            .output()
            .map_err(curl_yok)?;
        if !c.status.success() {
            return Err(HttpKaynak::hata(&tam, c.status.code(), &c.stderr));
        }
        let n = fs::metadata(hedef).map(|m| m.len()).map_err(|e| {
            KurHata::Diger(format!(
                "{tam} indirildi ama dosya okunamadı ({e}). Öneri: indirilen/ klasörünün izinlerini denetleyin."
            ))
        })?;
        if n > beklenen_boyut {
            return Err(KurHata::Dogrulama(format!(
                "{tam} yayin.json'daki boyuttan ({beklenen_boyut} bayt) büyük. Öneri: yayını yeniden üretin."
            )));
        }
        Ok(n)
    }
}

/// Yedekli kaynak: sıradaki kaynaklar (ör. GitHub Releases, sunucu üstü Tailscale/Caddy adresi, yerel klasör)
/// sırayla denenir; biri düşerse sessizce sıradakine geçilir, çalışan kaynak hatırlanır. İndirilen paket yine
/// SHA256 + ed25519 ile doğrulandığından hangi kaynaktan geldiği güveni değiştirmez.
/// Yarım kalan indirme silinmez, `<hedef>.kismi<N>` adıyla yana taşınır.
pub struct YedekliKaynak {
    adaylar: Vec<(String, Box<dyn Kaynak>)>,
    aktif: std::cell::Cell<usize>,
}

impl YedekliKaynak {
    pub fn yeni(adaylar: Vec<(String, Box<dyn Kaynak>)>) -> Result<YedekliKaynak, KurHata> {
        if adaylar.is_empty() {
            return Err(KurHata::Dogrulama(
                "Güncelleme kaynağı yok. Öneri: kur.toml'a kaynak_url / kaynak_klasoru yazın."
                    .into(),
            ));
        }
        Ok(YedekliKaynak {
            adaylar,
            aktif: std::cell::Cell::new(0),
        })
    }

    pub fn aktif_ad(&self) -> &str {
        &self.adaylar[self.aktif.get()].0
    }

    fn dene<T>(&self, mut f: impl FnMut(&dyn Kaynak) -> Result<T, KurHata>) -> Result<T, KurHata> {
        let n = self.adaylar.len();
        let bas = self.aktif.get();
        let mut hatalar = Vec::new();
        for i in 0..n {
            let idx = (bas + i) % n;
            let (ad, k) = &self.adaylar[idx];
            match f(k.as_ref()) {
                Ok(v) => {
                    self.aktif.set(idx);
                    return Ok(v);
                }
                Err(e) => hatalar.push(format!("{ad}: {e}")),
            }
        }
        Err(KurHata::Diger(format!(
            "Tüm güncelleme kaynakları denendi, hiçbiri çalışmadı ({}). Öneri: ağ bağlantısını denetleyin.",
            hatalar.join(" | ")
        )))
    }
}

impl Kaynak for YedekliKaynak {
    fn yayin_oku(&self) -> Result<Vec<u8>, KurHata> {
        self.dene(|k| k.yayin_oku())
    }

    fn getir(&self, url: &str, hedef: &Path, beklenen_boyut: u64) -> Result<u64, KurHata> {
        let mut say = 0u32;
        self.dene(|k| {
            if hedef.exists() {
                say += 1;
                let mut yan = hedef.as_os_str().to_owned();
                yan.push(format!(".kismi{say}"));
                fs::rename(hedef, PathBuf::from(yan))?;
            }
            k.getir(url, hedef, beklenen_boyut)
        })
    }
}
