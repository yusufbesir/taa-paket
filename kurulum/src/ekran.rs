// Sahip: Yusuf. yazan: [Claude#kur-yerel-pencere 2026-11-15]
//! Kurulum işi ile ekran arasındaki köprü (platformdan bağımsız). Kurulum ayrı iş parçacığında çalışır ve ortak
//! durumu (`Paylasim`) günceller; yerel pencere (yalnız Windows, `pencere.rs`) bu durumu çizer ve komut gönderir.
//! Pencere yoksa (Linux, `--arayuzsuz`, `--sessiz`) seçimler konsolda sorulur. Tarayıcı ya da HTTP yoktur.

use crate::direktif::Direktif;
use crate::ilerleme::Ilerleme;
use crate::secim::Secim;
use std::io::{self, Write};
use std::sync::{mpsc, Arc, Mutex};

/// Pencereden kurulum işine giden komut.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Komut {
    YenidenDene(String),
    Bitir,
    /// Seçim ekranında onay: seçilen sıra numaraları.
    Secim(Vec<usize>),
    /// Seçim ekranında vazgeç / atla (Esc).
    Vazgec,
    /// Bitti ekranında "TAA Kur'u aç": aç ve bitir.
    TaaKurAc,
}

/// Pencerenin çizdiği her şey.
#[derive(Debug, Clone)]
pub struct Paylasim {
    pub ilerleme: Ilerleme,
    /// Doluysa sol alanda seçim ekranı gösterilir; kurulum cevap bekler.
    pub secim: Option<Secim>,
    pub direktifler: Vec<Direktif>,
    pub hedef: String,
    /// Kurulumu durduran hata (pencerede kırmızı kutu; Bitir beklenir).
    pub hata: Option<String>,
    /// Kurulum işi bitti (çıkış kodu); pencere kapanabilir.
    pub cikis: Option<i32>,
}

/// Pencere tarafının tuttuğu uç: ortak durum + komut göndericisi.
#[derive(Clone)]
pub struct Uc {
    pub durum: Arc<Mutex<Paylasim>>,
    pub komut: mpsc::Sender<Komut>,
}

impl Uc {
    pub fn gonder(&self, k: Komut) {
        let _ = self.komut.send(k);
    }
}

/// Kurulum işinin tuttuğu uç.
pub struct Ekran {
    pub durum: Arc<Mutex<Paylasim>>,
    komut_rx: mpsc::Receiver<Komut>,
    /// Yerel pencere açık: seçimler pencerede, konsolda soru yok.
    pub pencere: bool,
}

impl Ekran {
    /// Ekran ve pencereye verilecek uç.
    pub fn yeni(surum: &str, hedef: &str, pencere: bool) -> (Ekran, Uc) {
        let durum = Arc::new(Mutex::new(Paylasim {
            ilerleme: Ilerleme::yeni(surum),
            secim: None,
            direktifler: vec![],
            hedef: hedef.to_string(),
            hata: None,
            cikis: None,
        }));
        let (tx, rx) = mpsc::channel();
        (Ekran { durum: Arc::clone(&durum), komut_rx: rx, pencere }, Uc { durum, komut: tx })
    }

    /// İlerleme üzerinde işlem (kilit zehirlenmişse sessizce atlanır: ekran kurulumu durdurmaz).
    pub fn g<R>(&self, f: impl FnOnce(&mut Ilerleme) -> R) -> Option<R> {
        self.durum.lock().ok().map(|mut d| f(&mut d.ilerleme))
    }

    pub fn p<R>(&self, f: impl FnOnce(&mut Paylasim) -> R) -> Option<R> {
        self.durum.lock().ok().map(|mut d| f(&mut d))
    }

    /// Pencereden komut bekler (pencere kanalı kapanırsa `Bitir`).
    pub fn komut_bekle(&self) -> Komut {
        self.komut_rx.recv().unwrap_or(Komut::Bitir)
    }

    /// Seçim yaptırır. Pencerede: seçim ekranı, Enter/Devam ile onay, Esc ile vazgeç (`None`).
    /// Konsolda: numaralı soru. Bu sırada gelen başka komutlar (ör. eski yeniden dene) yok sayılır.
    pub fn sec(&self, s: Secim) -> Option<Vec<usize>> {
        if !self.pencere {
            return Some(konsol_sec(&s));
        }
        self.p(|d| d.secim = Some(s));
        let sonuc = loop {
            match self.komut_bekle() {
                Komut::Secim(v) => break Some(v),
                Komut::Vazgec | Komut::Bitir => break None,
                _ => continue,
            }
        };
        self.p(|d| d.secim = None);
        sonuc
    }
}

fn konsol_sec(s: &Secim) -> Vec<usize> {
    println!("\n{}", s.baslik);
    if !s.aciklama.is_empty() {
        println!("{}", s.aciklama);
    }
    let mut grup = "";
    for (i, x) in s.secenekler.iter().enumerate() {
        if x.grup != grup && !x.grup.is_empty() {
            println!(" [{}]", x.grup);
        }
        grup = &x.grup;
        let ac = if x.aciklama.is_empty() { String::new() } else { format!(" - {}", x.aciklama) };
        println!("  {}) {}{ac}", i + 1, x.etiket);
    }
    let istem = if s.coklu {
        "Numaraları yazın (örn. 1,3), hepsi için H, atlamak için Enter: ".to_string()
    } else {
        format!("Seçiminiz (Enter = {}): ", s.imlec + 1)
    };
    print!("{istem}");
    let _ = io::stdout().flush();
    let mut c = String::new();
    let _ = io::stdin().read_line(&mut c);
    s.konsol_cevabi(&c)
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::secim::Secenek;

    #[test]
    fn pencere_secimi_kanal_uzerinden() {
        let (ekran, uc) = Ekran::yeni("1.0", "D:/TAA", true);
        let s = Secim::tek("Rol", "", vec![Secenek::yeni("Şube", ""), Secenek::yeni("Sunucu", "")], 0);
        let uc2 = uc.clone();
        let t = std::thread::spawn(move || {
            // Seçim ekranı görünene kadar bekle, sonra eski bir komut ve onay gönder.
            loop {
                if uc2.durum.lock().unwrap().secim.is_some() {
                    break;
                }
                std::thread::yield_now();
            }
            uc2.gonder(Komut::YenidenDene("x".into()));
            uc2.gonder(Komut::Secim(vec![1]));
        });
        assert_eq!(ekran.sec(s.clone()), Some(vec![1]));
        t.join().unwrap();
        assert!(uc.durum.lock().unwrap().secim.is_none(), "seçimden sonra ekran temizlenir");
        uc.gonder(Komut::Vazgec);
        assert_eq!(ekran.sec(s), None);
        drop(uc);
        assert_eq!(ekran.komut_bekle(), Komut::Bitir, "pencere kapanınca bitir");
    }

    #[test]
    fn ilerleme_ve_paylasim_erisimi() {
        let (ekran, uc) = Ekran::yeni("2.0", "X", false);
        ekran.g(|i| i.program_ekle("tutaa", false));
        ekran.p(|p| p.cikis = Some(0));
        let d = uc.durum.lock().unwrap();
        assert_eq!((d.ilerleme.programlar.len(), d.cikis, d.hedef.as_str()), (1, Some(0), "X"));
    }
}
