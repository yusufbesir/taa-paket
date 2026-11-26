//! Bildirim biçimleyici: Telegram metni ve e-posta konu/gövde. Kişisel veri maskelenir: metindeki e-posta adresleri
//! ve 7+ haneli sayı dizileri (telefon, TC kimlik, IBAN parçaları) gizlenir; ihbar edenin bıraktığı iletişim
//! bildirime YAZILMAZ (yalnız "bırakıldı" denir, içeriği yönetici ekranındadır). Ağ yok.
//! Sahip: Yusuf. yazan: [Claude#ihbar 2026-11-26]

use crate::kayit::Ihbar;

/// Telegram ileti sınırı (karakter).
pub const TELEGRAM_EN_COK: usize = 4096;
const EPOSTA_YER: &str = "[e-posta gizlendi]";
const NUMARA_YER: &str = "[numara gizlendi]";
const KESME: &str = "\n… (devamı yönetici ekranında)";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bildirim {
    pub telegram: String,
    pub eposta_konu: String,
    pub eposta_govde: String,
}

fn eposta_parca(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '.' | '_' | '%' | '+' | '-')
}

fn eposta_gizle(s: &str) -> String {
    let ch: Vec<char> = s.chars().collect();
    let mut cikti = String::new();
    let mut i = 0;
    while i < ch.len() {
        if ch[i] == '@' {
            // Yerel kısmı çıktıdan geri al.
            let mut sol = 0;
            for c in cikti.chars().rev() {
                if eposta_parca(c) {
                    sol += 1;
                } else {
                    break;
                }
            }
            let mut sag = i + 1;
            while sag < ch.len() && (ch[sag].is_alphanumeric() || ch[sag] == '.' || ch[sag] == '-')
            {
                sag += 1;
            }
            // Sondaki noktaları adrese katma.
            while sag > i + 1 && ch[sag - 1] == '.' {
                sag -= 1;
            }
            let alan: String = ch[i + 1..sag].iter().collect();
            if sol > 0 && alan.contains('.') && !alan.starts_with('.') {
                let kalan = cikti.chars().count() - sol;
                cikti = cikti.chars().take(kalan).collect();
                cikti.push_str(EPOSTA_YER);
                i = sag;
                continue;
            }
        }
        cikti.push(ch[i]);
        i += 1;
    }
    cikti
}

fn numara_ayrac(c: char) -> bool {
    matches!(c, ' ' | '-' | '+' | '(' | ')')
}

fn numara_gizle(s: &str) -> String {
    let ch: Vec<char> = s.chars().collect();
    let mut cikti = String::new();
    let mut i = 0;
    while i < ch.len() {
        if !ch[i].is_ascii_digit() {
            cikti.push(ch[i]);
            i += 1;
            continue;
        }
        let baslangic = i;
        let mut son_hane = i;
        let mut hane = 0;
        let mut k = i;
        while k < ch.len() && (ch[k].is_ascii_digit() || numara_ayrac(ch[k])) {
            if ch[k].is_ascii_digit() {
                hane += 1;
                son_hane = k;
            }
            k += 1;
        }
        if hane >= 7 {
            // Başta bitişik '+' çıktıdan alınır.
            if baslangic > 0 && ch[baslangic - 1] == '+' && cikti.ends_with('+') {
                cikti.pop();
            }
            cikti.push_str(NUMARA_YER);
        } else {
            cikti.extend(ch[baslangic..=son_hane].iter());
        }
        i = son_hane + 1;
    }
    cikti
}

/// E-posta adreslerini ve 7+ haneli sayı dizilerini gizler.
pub fn maskele(metin: &str) -> String {
    numara_gizle(&eposta_gizle(metin))
}

fn baslik(i: &Ihbar) -> String {
    let ilt = if i.anonim_mi() {
        "yok (anonim)"
    } else {
        "bırakıldı (yönetici ekranında görünür)"
    };
    format!(
        "Kategori: {}\nTakip no: {}\nTarih: {}\nİletişim: {}",
        i.kategori.etiket(),
        i.takip_no,
        i.tarih,
        ilt
    )
}

fn sinirla(baslik: &str, govde: &str, en_cok: usize) -> String {
    let tam = format!("{baslik}\n\n{govde}");
    if tam.chars().count() <= en_cok {
        return tam;
    }
    let pay = en_cok.saturating_sub(baslik.chars().count() + 2 + KESME.chars().count());
    let kisa: String = govde.chars().take(pay).collect();
    format!("{baslik}\n\n{kisa}{KESME}")
}

pub fn bildirim(i: &Ihbar) -> Bildirim {
    let maske = maskele(&i.metin);
    let ust = baslik(i);
    let telegram = sinirla(&format!("İHBAR\n{ust}"), &maske, TELEGRAM_EN_COK);
    let eposta_konu = format!("[İhbar] {} {}", i.kategori.etiket(), i.takip_no);
    let eposta_govde = format!(
        "{ust}\n\n{maske}\n\n--\nMetindeki e-posta ve telefon benzeri numaralar gizlenmiştir; özgün metin ve iletişim \
         bilgisi yalnız yönetici ekranındadır. Bu ileti ihbar edenin kimliğini içermez."
    );
    Bildirim {
        telegram,
        eposta_konu,
        eposta_govde,
    }
}

#[cfg(test)]
mod testler {
    use super::*;
    use crate::kayit::{Girdi, Kategori};

    fn ihbar(metin: &str, iletisim: Option<&str>) -> Ihbar {
        Ihbar::olustur(
            Girdi {
                kategori: Kategori::Hirsizlik,
                metin: metin.into(),
                iletisim: iletisim.map(str::to_string),
            },
            "2026-11-26",
            "IH-ABCDEF0123".into(),
        )
        .unwrap()
    }

    #[test]
    fn eposta_maskelenir() {
        let m = maskele("ali.veli@ornek.com.tr yazdı, sonra b@c.d de");
        assert_eq!(m, "[e-posta gizlendi] yazdı, sonra [e-posta gizlendi] de");
        // Alan adı noktasız ise adres sayılmaz.
        assert_eq!(maskele("a@b"), "a@b");
        assert_eq!(maskele("Bitti@sonunda."), "Bitti@sonunda.");
    }

    #[test]
    fn numara_maskelenir_kisa_sayi_kalir() {
        assert_eq!(maskele("0532 123 45 67 aradı"), "[numara gizlendi] aradı");
        assert_eq!(maskele("+90 532 123 45 67"), "[numara gizlendi]");
        assert_eq!(maskele("TC 12345678901 var"), "TC [numara gizlendi] var");
        assert_eq!(
            maskele("3 adet, saat 14 30, 12 kişi"),
            "3 adet, saat 14 30, 12 kişi"
        );
        assert_eq!(maskele("fiş 123456 hazır"), "fiş 123456 hazır");
    }

    #[test]
    fn telegram_ve_eposta_icerigi() {
        let i = ihbar(
            "Kasadan 05321234567 numaralı kişi para aldı, ali@x.com",
            Some("0555 000 00 00"),
        );
        let b = bildirim(&i);
        assert!(b.telegram.contains("IH-ABCDEF0123"));
        assert!(b.telegram.contains("Hırsızlık"));
        assert!(b.telegram.contains("bırakıldı"));
        for s in [&b.telegram, &b.eposta_govde, &b.eposta_konu] {
            assert!(!s.contains("0532"), "{s}");
            assert!(!s.contains("ali@x.com"));
            assert!(!s.contains("0555"));
        }
        assert!(b.eposta_konu.starts_with("[İhbar]"));
        assert!(!b.eposta_konu.contains('\n'));
    }

    #[test]
    fn anonim_baslik() {
        let b = bildirim(&ihbar("metin", None));
        assert!(b.telegram.contains("yok (anonim)"));
    }

    #[test]
    fn telegram_siniri() {
        let i = ihbar(&"a".repeat(4000), Some("iletişim"));
        let b = bildirim(&i);
        assert!(b.telegram.chars().count() <= TELEGRAM_EN_COK);
        assert!(b.telegram.contains("devamı yönetici ekranında"));
        // E-posta gövdesi kesilmez.
        assert!(b.eposta_govde.contains(&"a".repeat(4000)));
    }
}
