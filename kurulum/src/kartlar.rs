// Sahip: Yusuf. yazan: [Claude#kur-ekrani 2026-11-15] değiştiren: [Claude#kurucu-arayuz 2026-11-17]
//! Bekleme kartları: `kurulum/kartlar.txt` verisini çözer; yerel kurulum penceresi bekleme sırasında döndürür. Saf mantık.

/// Exe'ye gömülen kart verisi.
pub const VERI: &str = include_str!("../kartlar.txt");

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Kart {
    pub kimlik: String,
    pub tur: String,
    pub ad: String,
    pub ozet: String,
    pub not: String,
    pub sablon: String,
}

/// Kart dosyasını çözer. Hata: satır numarası ve neden.
pub fn coz(veri: &str) -> Result<Vec<Kart>, String> {
    let mut kartlar: Vec<Kart> = vec![];
    for (no, ham) in veri.lines().enumerate() {
        let satir = ham.trim();
        let no = no + 1;
        if satir.is_empty() || satir.starts_with('#') {
            continue;
        }
        if let Some(k) = satir.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            let k = k.trim();
            if k.is_empty() || !k.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
                return Err(format!("{no}. satır: geçersiz kart kimliği [{k}]"));
            }
            if kartlar.iter().any(|x| x.kimlik == k) {
                return Err(format!("{no}. satır: [{k}] iki kez var"));
            }
            kartlar.push(Kart { kimlik: k.to_string(), tur: "program".into(), ..Kart::default() });
            continue;
        }
        let Some((a, d)) = satir.split_once('=') else {
            return Err(format!("{no}. satır: 'anahtar = değer' bekleniyordu"));
        };
        let Some(kart) = kartlar.last_mut() else {
            return Err(format!("{no}. satır: [kimlik] satırından önce değer"));
        };
        let d = d.trim().to_string();
        match a.trim() {
            "tur" if d == "program" || d == "rehber" => kart.tur = d,
            "tur" => return Err(format!("{no}. satır: tur program ya da rehber olmalı")),
            "ad" => kart.ad = d,
            "ozet" => kart.ozet = d,
            "not" => kart.not = d,
            "sablon" => kart.sablon = d,
            x => return Err(format!("{no}. satır: bilinmeyen anahtar '{x}'")),
        }
    }
    if let Some(k) = kartlar.iter().find(|k| k.ad.is_empty() || k.ozet.is_empty()) {
        return Err(format!("[{}] kartında ad ve ozet zorunlu", k.kimlik));
    }
    Ok(kartlar)
}

/// Gömülü kartlar; bozuksa boş liste (arayüz kartsız çalışır, kurulum durmaz).
pub fn gomulu() -> Vec<Kart> {
    coz(VERI).unwrap_or_default()
}

/// Kartların eski sabit dönme aralığı (ms). Pencere artık temanın süresini kullanır
/// (`taa_kur_tema::Tema::tanitim_ms`: İlmek 6 sn, Kalıp 7,2 sn, Mezura 8,4 sn); sabit konsol/yedek için kalır.
#[allow(dead_code)]
pub const DONUS_MS: u64 = 9000;

/// Kart sırası: ileri/geri bir adım, başa/sona sarar. Kart yoksa 0.
pub fn sonraki(simdiki: usize, adet: usize, ileri: bool) -> usize {
    if adet == 0 {
        return 0;
    }
    let s = simdiki % adet;
    if ileri { (s + 1) % adet } else { (s + adet - 1) % adet }
}

/// Kart türünün ekrandaki küçük başlığı (büyük harfli etiket yok, KARAR §2.2).
pub fn tur_etiketi(tur: &str) -> &'static str {
    if tur == "rehber" { "Not" } else { "Program tanıtımı" }
}

#[cfg(test)]
mod testler {
    use super::*;

    #[test]
    fn gomulu_kartlar_gecerli_ve_tum_programlar_var() {
        let k = coz(VERI).expect("kartlar.txt çözülmeli");
        let adlar: Vec<&str> = k.iter().filter(|k| k.tur == "program").map(|k| k.ad.as_str()).collect();
        for beklenen in [
            "TAA Etiket",
            "Tutaa",
            "TaaRkod",
            "TaaDön",
            "TaaFiş / TaaTerziTakip",
            "TAA Rehber",
            "TAA Veri Tabanı Deposu",
            "TAA Video Yapıcı",
        ] {
            assert!(adlar.contains(&beklenen), "{beklenen} kartı yok");
        }
        for p in k.iter().filter(|k| k.tur == "program") {
            assert!(!p.not.is_empty() && !p.sablon.is_empty(), "{}: not ve şablon olmalı", p.kimlik);
            // 1-2 cümle: kartın kısa kalması için kaba sınır.
            assert!(p.ozet.chars().count() <= 220, "{}: özet çok uzun", p.kimlik);
        }
        assert!(k.iter().any(|k| k.tur == "rehber"));
        assert_eq!(gomulu().len(), k.len());
    }

    #[test]
    fn cozum_ve_hatalar() {
        let k = coz("# yorum\n\n[a-1]\nad = A\nozet = Ö = eşittir\n[b]\ntur = rehber\nad=B\nozet=x\n").unwrap();
        assert_eq!(k.len(), 2);
        assert_eq!(k[0].ozet, "Ö = eşittir");
        assert_eq!((k[0].tur.as_str(), k[1].tur.as_str()), ("program", "rehber"));
        assert!(coz("ad = A").unwrap_err().contains("önce"));
        assert!(coz("[A]\nad=x\nozet=y").unwrap_err().contains("kimliği"));
        assert!(coz("[a]\nad=x\nozet=y\n[a]\nad=x\nozet=y").unwrap_err().contains("iki kez"));
        assert!(coz("[a]\nad=x\nozet=y\nrenk=mavi").unwrap_err().contains("bilinmeyen"));
        assert!(coz("[a]\nad=x\nozet=y\ntur=oyun").unwrap_err().contains("tur"));
        assert!(coz("[a]\nad=x").unwrap_err().contains("zorunlu"));
        assert!(coz("[a]\nad x").unwrap_err().contains("bekleniyordu"));
    }

    #[test]
    fn kart_donusu() {
        assert_eq!(sonraki(0, 3, true), 1);
        assert_eq!(sonraki(2, 3, true), 0);
        assert_eq!(sonraki(0, 3, false), 2);
        assert_eq!(sonraki(7, 3, false), 0);
        assert_eq!(sonraki(5, 0, true), 0);
        assert_eq!((tur_etiketi("rehber"), tur_etiketi("program")), ("Not", "Program tanıtımı"));
    }
}
