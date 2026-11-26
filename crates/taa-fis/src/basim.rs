//! Düz metin ve ESC/POS baskı düzeni. lib.rs'den ayrıldı; davranış aynı.

use super::*;

pub fn para(k: i64) -> String {
    let tl = (k.unsigned_abs() / 100).to_string();
    let mut g = String::new();
    for (i, c) in tl.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            g.push('.');
        }
        g.push(c);
    }
    format!(
        "{}{},{:02}",
        if k < 0 { "-" } else { "" },
        g.chars().rev().collect::<String>(),
        k.unsigned_abs() % 100
    )
}

pub(crate) fn gun_metni(t: &str) -> String {
    format!("{}.{}.{}", &t[8..10], &t[5..7], &t[0..4])
}

pub(crate) fn sar(metin: &str, genislik: usize) -> Vec<String> {
    let mut satirlar = Vec::new();
    let mut cur = String::new();
    for kelime in metin.split_whitespace() {
        let kl = kelime.chars().count();
        if !cur.is_empty() && cur.chars().count() + 1 + kl > genislik {
            satirlar.push(std::mem::take(&mut cur));
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(kelime);
        while cur.chars().count() > genislik {
            let (a, b): (String, String) = (
                cur.chars().take(genislik).collect(),
                cur.chars().skip(genislik).collect(),
            );
            satirlar.push(a);
            cur = b;
        }
    }
    if !cur.is_empty() {
        satirlar.push(cur);
    }
    satirlar
}

/// Sabit genişlikli (örn. 42 sütun) düz metin fiş.
pub fn duz_metin(f: &Fis, genislik: usize) -> String {
    duz_metin_nusha(f, genislik, "")
}

/// Aynı fiş, üstte nüsha etiketiyle (örn. "TERZİHANE NÜSHASI", "MÜŞTERİ NÜSHASI").
pub fn duz_metin_nusha(f: &Fis, genislik: usize, nusha: &str) -> String {
    let g = genislik.max(24);
    let mut o = String::new();
    for (stil, m) in satirlar(f, nusha) {
        match stil {
            Stil::Ayrac => o.push_str(&"-".repeat(g)),
            Stil::Cift => o.push_str(&"=".repeat(g)),
            Stil::Ham => o.push_str(&m),
            Stil::Serit => {
                let ic = format!(" {m} ");
                let sol = g.saturating_sub(ic.chars().count()) / 2;
                o.push_str(&format!(
                    "{}{ic}{}",
                    "=".repeat(sol),
                    "=".repeat(g.saturating_sub(sol + ic.chars().count()))
                ));
            }
            _ => {
                for l in sar(&m, g) {
                    if stil.ortali() {
                        let bos = g.saturating_sub(l.chars().count()) / 2;
                        o.push_str(&" ".repeat(bos));
                    }
                    o.push_str(&l);
                    o.push('\n');
                }
                continue;
            }
        }
        o.push('\n');
    }
    o
}

/// Müşteri nüshasında teslim imza bloğu ve ürün nüshasında alt bilgi cümleleri basılmaz (kâğıt tasarrufu);
/// etiketsiz (tek) çıktıda ikisi de basılır.
pub(crate) fn musteri_nushasi(nusha: &str) -> bool {
    nusha.starts_with("MÜŞTERİ")
}
pub(crate) fn terzihane_nushasi(nusha: &str) -> bool {
    nusha.starts_with("TERZİHANE")
}

/// Fişteki bir satırın görünümü. Yazıcıda kalın, büyük (2x) ve küçük (Font B) yazı ile ortalama kullanılır;
/// düz metinde yalnız ortalama ve çizgiler yansır.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Stil {
    Normal,
    Kalin,
    OrtaKalin,
    OrtaKucuk,
    OrtaBuyuk,
    /// Ters baskı (siyah zemin, beyaz yazı) bölüm başlığı.
    Serit,
    /// Firma logosu (bitmap); logo kapalıysa metin olarak ortalı kalın basılır.
    Logo,
    Ham,
    Ayrac,
    Cift,
}

impl Stil {
    fn ortali(self) -> bool {
        matches!(
            self,
            Stil::OrtaKalin | Stil::OrtaKucuk | Stil::OrtaBuyuk | Stil::Serit | Stil::Logo
        )
    }
}

/// Fişin satırları (ham, sarılmamış). Fiyat/ödeme bilgisi basılmaz: ödeme alınmadan işlem yapılmaz.
pub(crate) fn satirlar(f: &Fis, nusha: &str) -> Vec<(Stil, String)> {
    if musteri_nushasi(nusha) && kisa_musteri_nushasi() {
        return musteri_satirlari(f, nusha);
    }
    use Stil::*;
    let mut v: Vec<(Stil, String)> = Vec::new();
    for (i, l) in baslik().lines().enumerate() {
        v.push((
            match i {
                0 if logo_kullan() => Logo,
                0 => OrtaKalin,
                _ => OrtaKucuk,
            },
            l.to_string(),
        ));
    }
    v.push((Cift, String::new()));
    v.push((OrtaKucuk, "TADİLAT FİŞİ".into()));
    v.push((OrtaBuyuk, f.no.clone()));
    if !nusha.is_empty() {
        v.push((OrtaKalin, nusha.to_string()));
    }
    v.push((Normal, format!("Fiş Tarihi: {}", gun_metni(&f.tarih))));
    v.push((Ayrac, String::new()));
    v.push((Kalin, format!("Müşteri Adı: {}", f.musteri)));
    v.push((Normal, format!("Telefonu: {}", f.telefon)));
    v.push((Ayrac, String::new()));
    v.push((Serit, "ÜRÜN TESLİM TARİHİ".into()));
    v.push((
        OrtaBuyuk,
        format!("{} {}", gun_metni(&f.teslim), f.teslim_saati),
    ));
    if let Some(gun) = gun_adi(&f.teslim) {
        v.push((OrtaKalin, gun.to_string()));
    }
    if !f.teslim_degisiklikleri.is_empty() {
        v.push((
            OrtaKucuk,
            format!(
                "(teslim tarihi {} kez değişti)",
                f.teslim_degisiklikleri.len()
            ),
        ));
    }
    if !f.prova.is_empty() {
        v.push((Normal, format!("Prova: {}", gun_metni(&f.prova))));
    }
    v.push((Ayrac, String::new()));
    v.push((Serit, "İŞLEM DETAYI".into()));
    for k in &f.kalemler {
        let mut satir = format!("* {}", kalem_ozeti(k));
        if let Some(x) = &k.varyant {
            satir.push_str(&format!(" [{x}]"));
        }
        if let Some(m) = k.olcu_mm {
            satir.push_str(&format!(" {} cm", cm_metni(m)));
        }
        if let Some(p) = k.pay_mm.filter(|p| *p > 0) {
            satir.push_str(&format!(" (pay {} cm)", cm_metni(p)));
        }
        if k.igneli {
            satir.push_str(" [İĞNELİ]");
        }
        if k.usta_onayli {
            satir.push_str(" [USTA ONAYLI]");
        }
        v.push((Normal, satir));
        if !k.not.is_empty() {
            v.push((Normal, format!("  ({})", k.not)));
        }
    }
    if !f.olcu_notu.is_empty() {
        v.push((Normal, format!("Ölçü: {}", f.olcu_notu)));
    }
    if f.kargo {
        v.push((Ayrac, String::new()));
        v.push((OrtaKalin, "*** KARGO YAPILACAK ***".into()));
        v.push((Normal, format!("Alıcı: {}", f.kargo_ad)));
        v.push((Normal, format!("Adres: {}", f.kargo_adres)));
        v.push((Normal, format!("Alıcı Tel: {}", f.kargo_telefon)));
        if !f.kargo_notu.is_empty() {
            v.push((Normal, format!("Kargo: {}", f.kargo_notu)));
        }
    }
    if !f.bagli.is_empty() {
        v.push((Ayrac, String::new()));
        v.push((OrtaKalin, "*** BİRLİKTE TESLİM ***".into()));
        v.push((Normal, format!("Bağlı fişler: {}", f.bagli.join(", "))));
    }
    if !f.duzeltmeler.is_empty() {
        v.push((Ayrac, String::new()));
        v.push((Kalin, "Sonradan yapılan değişiklikler:".into()));
        for d in &f.duzeltmeler {
            let ne = match d.ne.as_str() {
                "kalem-ekle" => "+",
                "kalem-cikar" => "-",
                _ => "*",
            };
            v.push((
                Normal,
                format!("{ne} {} ({})", d.ayrinti, gun_metni(&d.tarih)),
            ));
        }
    }
    v.push((Ayrac, String::new()));
    v.push((
        Normal,
        format!(
            "Sat. Dan.: {} | Ölçü Alan: {}",
            kisa_ad(&f.satis_danismani),
            kisa_ad(&f.olcu_alan)
        ),
    ));
    if !f.sube.is_empty() {
        v.push((Normal, format!("Şube: {}", f.sube)));
    }
    if !f.usta.is_empty() {
        v.push((Normal, format!("Usta: {}", kisa_ad(&f.usta))));
    }
    if !f.raf.is_empty() {
        v.push((Normal, format!("Raf/Stant: {}", f.raf)));
    }
    for i in &f.imzalar {
        v.push((
            Normal,
            format!(
                "İmza ({}): {} {}",
                i.rol,
                kisa_ad(&i.kim),
                gun_metni(&i.tarih)
            ),
        ));
    }
    // Teslim alan (mağaza/terzihane nüshası ve düz metin): "Teslim alan: Ayşe Ç. (yakını) - fişsiz, imzalı".
    // Tire yazıcıda da çıksın diye ASCII; CP857'de uzun tire yok.
    if let Some(t) = f.teslim_kaydi.as_ref().filter(|_| !musteri_nushasi(nusha)) {
        let mut ek = String::from(if t.fisli { "fişli" } else { "fişsiz" });
        if !t.imza_sha256.is_empty() {
            ek.push_str(", imzalı");
        }
        v.push((
            Normal,
            format!(
                "Teslim alan: {} ({}) - {ek}",
                kisa_ad(&t.teslim_alan),
                iliski_adi(&t.iliski).unwrap_or("?")
            ),
        ));
        if !t.ana_fis.is_empty() {
            // Bağlı fiş: teslim, ana fişle birlikte yapıldı (imza ve doğrulama ana fişte).
            v.push((Normal, format!("Birlikte teslim: {}", t.ana_fis)));
        }
    }
    if imza_blogu() && !musteri_nushasi(nusha) {
        v.push((Ayrac, String::new()));
        for l in [
            "Teslim Eden            Teslim Alan",
            "Tarih:                 Tarih:",
            "İmza:                  İmza:",
        ] {
            v.push((Ham, l.into()));
        }
    }
    v
}

/// Müşteri nüshası cüzdana sığacak kadar kısa (Yusuf 2026-11-26): logo bitmap'i, adres satırları, ölçüler,
/// personel ve imza basılmaz; ürünler tek satır özet. `fis-kisa-musteri` ayarı kapatılırsa eski uzun düzen basılır.
pub(crate) fn kisa_musteri_nushasi() -> bool {
    taa_ayar::ozellik_acik("fis-kisa-musteri", true)
}

pub(crate) fn musteri_satirlari(f: &Fis, nusha: &str) -> Vec<(Stil, String)> {
    use Stil::*;
    let mut v: Vec<(Stil, String)> = Vec::new();
    let b = baslik();
    let satirlar_b: Vec<&str> = b.lines().collect();
    if let Some(ilk) = satirlar_b.first() {
        v.push((OrtaKalin, ilk.to_string()));
    }
    if satirlar_b.len() > 1 {
        if let Some(son) = satirlar_b.last() {
            v.push((OrtaKucuk, son.to_string()));
        }
    }
    v.push((OrtaBuyuk, f.no.clone()));
    v.push((OrtaKucuk, format!("{} · {}", nusha, gun_metni(&f.tarih))));
    v.push((Ayrac, String::new()));
    v.push((Kalin, format!("Müşteri: {}", f.musteri)));
    let gun = gun_adi(&f.teslim)
        .map(|g| format!(" {g}"))
        .unwrap_or_default();
    v.push((
        Kalin,
        format!("Teslim: {}{gun} {}", gun_metni(&f.teslim), f.teslim_saati),
    ));
    for k in &f.kalemler {
        v.push((Normal, format!("* {}", kalem_ozeti(k))));
    }
    if f.kargo {
        v.push((OrtaKalin, "KARGO İLE GÖNDERİLECEK".into()));
    }
    if !f.bagli.is_empty() {
        v.push((Normal, format!("Birlikte teslim: {}", f.bagli.join(", "))));
    }
    v
}

/// Logo bitmap'i: `fis-logo` ayarı açıksa ve başlık özelleştirilmemişse (özel başlıkta farklı firma adı olabilir).
pub(crate) fn logo_kullan() -> bool {
    taa_ayar::ozellik_acik("fis-logo", true)
        && std::env::var("TAA_FIS_BASLIK").map_or(true, |v| v.trim().is_empty())
}

/// 80 mm yazıcı için 576 nokta genişliğinde 1 bit logo (assets/logo-uret.py üretir): [yükseklik u16 LE] + satır başına 72 bayt.
pub(crate) static LOGO: &[u8] = include_bytes!("../assets/logo-576.bin");

/// ESC/POS ham bitmap (GS v 0) komutu olarak logo; ortalama bitmap içindeki beyaz dolguyla yapılır.
pub(crate) fn logo_baytlari() -> Vec<u8> {
    let h = LOGO[0] as usize | (LOGO[1] as usize) << 8;
    let mut b = vec![0x1D, 0x76, 0x30, 0, 72, 0, (h & 255) as u8, (h >> 8) as u8];
    b.extend(&LOGO[2..2 + 72 * h]);
    b
}

/// "2026-11-10" -> "Salı" (Zeller). Geçersiz tarihte None.
pub(crate) fn gun_adi(t: &str) -> Option<&'static str> {
    let y: i32 = t.get(0..4)?.parse().ok()?;
    let m: i32 = t.get(5..7)?.parse().ok()?;
    let d: i32 = t.get(8..10)?.parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let (yy, mm) = if m < 3 { (y - 1, m + 12) } else { (y, m) };
    let k = yy % 100;
    let j = yy / 100;
    let h = (d + 13 * (mm + 1) / 5 + k + k / 4 + j / 4 + 5 * j) % 7; // 0 = Cumartesi
    Some(
        [
            "Cumartesi",
            "Pazar",
            "Pazartesi",
            "Salı",
            "Çarşamba",
            "Perşembe",
            "Cuma",
        ][h as usize],
    )
}

/// Fiş başlığı (firma adı, adres, destek hattı). `TAA_FIS_BASLIK` ile (satırlar `|` ile) değiştirilir.
pub(crate) fn baslik() -> String {
    match std::env::var("TAA_FIS_BASLIK") {
        Ok(v) if !v.trim().is_empty() => v.replace('|', "\n"),
        _ => "TERZİ ADEM ALTUN\nTurgut Özal Mah. 68. Sk. No:12, E-5 Karayolu Yan Yol Üzeri\nEsenyurt / İstanbul - Destek: 0212 690 02 02 - 9:30-21:30".into(),
    }
}

/// Teslim Eden / Teslim Alan tarih-imza alanları (`fis-imza` ayarıyla kapatılır).
pub(crate) fn imza_blogu() -> bool {
    taa_ayar::ozellik_acik("fis-imza", true)
}

/// 25 mm -> "2,5"; 40 mm -> "4".
pub fn cm_metni(mm: i32) -> String {
    if mm % 10 == 0 {
        (mm / 10).to_string()
    } else {
        format!("{},{}", mm / 10, (mm % 10).abs())
    }
}

pub(crate) fn cp857(c: char) -> u8 {
    match c {
        'Ç' => 0x80,
        'ü' => 0x81,
        'ç' => 0x87,
        'ı' => 0x8D,
        'İ' => 0x98,
        'Ö' => 0x99,
        'Ü' => 0x9A,
        'ö' => 0x94,
        'Ğ' => 0xA6,
        'ğ' => 0xA7,
        'Ş' => 0x9E,
        'ş' => 0x9F,
        c if c.is_ascii() => c as u8,
        _ => b'?',
    }
}

/// ESC/POS baytları: CP857, metin, fiş numarası QR kodu, kesme. Yazıcıya olduğu gibi gönderilir.
pub fn escpos(f: &Fis, genislik: usize) -> Vec<u8> {
    escpos_nusha(f, genislik, "")
}

/// Kâğıt fişteki düzen: 2 terzihane nüshası + 1 müşteri nüshası, her biri ayrı kesilir.
pub fn escpos_nushalar(f: &Fis, genislik: usize) -> Vec<u8> {
    escpos_nushalar_qr(f, genislik, &f.no)
}

/// Aynısı; QR içeriği `qr` (ör. `http://sunucu:8787/telefon#fis=F-...`: telefon kamerası fişi doğrudan açar).
pub fn escpos_nushalar_qr(f: &Fis, genislik: usize, qr: &str) -> Vec<u8> {
    escpos_nushalar_tam(f, genislik, qr, None)
}

/// Aynısı; `aydinlatma` verilirse (ve `fis-aydinlatma` ayarı açıksa) MÜŞTERİ nüshasına kısa aydınlatma
/// cümlesi ve ayrıntılı metne giden ikinci bir QR basılır.
pub fn escpos_nushalar_tam(
    f: &Fis,
    genislik: usize,
    qr: &str,
    aydinlatma: Option<&str>,
) -> Vec<u8> {
    escpos_nushalar_takipli(f, genislik, qr, aydinlatma, None)
}

/// Müşteri takip karekodu (Yusuf 2026-11-26): MÜŞTERİ nüshasında personel karekodu yerine takip sayfasının
/// adresi basılır; üstünde kısa açıklama, altında (varsa) yedek adres yazılır. Terzihane nüshası değişmez.
#[derive(Debug, Clone, Copy)]
pub struct Takip<'a> {
    pub adres: &'a str,
    pub yedek: Option<&'a str>,
    pub cumle: &'a str,
}

pub fn escpos_nushalar_takipli(
    f: &Fis,
    genislik: usize,
    qr: &str,
    aydinlatma: Option<&str>,
    takip: Option<&Takip>,
) -> Vec<u8> {
    let mut b = Vec::new();
    for etiket in nusha_etiketleri(nusha_sayisi()) {
        b.extend(escpos_nusha_takipli(
            f, genislik, etiket, qr, aydinlatma, takip,
        ));
    }
    b
}

/// Basılacak nüsha sayısı: varsayılan 2 (terzihane + müşteri; takip programda tutulur).
/// `TAA_FIS_NUSHA=3` eski düzeni (2 terzihane + 1 müşteri) geri getirir.
pub(crate) fn nusha_sayisi() -> usize {
    match std::env::var("TAA_FIS_NUSHA")
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some("3") => 3,
        _ => 2,
    }
}

pub(crate) fn nusha_etiketleri(n: usize) -> Vec<&'static str> {
    if n >= 3 {
        vec![
            "TERZİHANE NÜSHASI 1",
            "TERZİHANE NÜSHASI 2",
            "MÜŞTERİ NÜSHASI",
        ]
    } else {
        vec!["TERZİHANE NÜSHASI", "MÜŞTERİ NÜSHASI"]
    }
}

pub fn escpos_nusha(f: &Fis, genislik: usize, nusha: &str) -> Vec<u8> {
    escpos_nusha_qr(f, genislik, nusha, &f.no)
}

pub fn escpos_nusha_qr(f: &Fis, genislik: usize, nusha: &str, qr: &str) -> Vec<u8> {
    escpos_nusha_tam(f, genislik, nusha, qr, None)
}

/// Müşteri nüshasındaki kısa aydınlatma cümlesi. Metin şirketin KVKK sorumlusu tarafından onaylanmalıdır.
pub const AYDINLATMA_CUMLESI: &str = "Kişisel verileriniz, tadilat işleminin yürütülmesi ve ürününüzün hazır olduğunun bildirilmesi amacıyla işlenir. Ayrıntılı aydınlatma metni için aşağıdaki QR kodu okutunuz.";

/// Kısa müşteri nüshasının tek satırlık kuralı.
pub const KISA_KURAL: &str = "17:00'den itibaren fişinizle teslim alabilirsiniz. 14 gün içinde alınmayan ürünlerden mesul değiliz.";
/// Kısa müşteri nüshasında aydınlatma (karekodsuz). Metin KVKK sorumlusunca onaylanmalıdır.
pub const KISA_AYDINLATMA: &str =
    "Kişisel verileriniz yalnız tadilat ve teslim bilgilendirmesi için işlenir. Aydınlatma metni:";

pub(crate) fn qr_baytlari(veri: &[u8]) -> Vec<u8> {
    qr_baytlari_boyut(veri, 5)
}

/// QR (model 2, düzeltme L), ortalı; boyut = modül nokta sayısı (4 küçük, 5 normal).
pub(crate) fn qr_baytlari_boyut(veri: &[u8], boyut: u8) -> Vec<u8> {
    let mut b = vec![0x1B, 0x61, 1];
    b.extend([0x1D, 0x28, 0x6B, 4, 0, 0x31, 0x41, 0x32, 0]);
    b.extend([0x1D, 0x28, 0x6B, 3, 0, 0x31, 0x43, boyut]);
    b.extend([0x1D, 0x28, 0x6B, 3, 0, 0x31, 0x45, 0x30]);
    let n = veri.len() + 3;
    b.extend([
        0x1D,
        0x28,
        0x6B,
        (n % 256) as u8,
        (n / 256) as u8,
        0x31,
        0x50,
        0x30,
    ]);
    b.extend(veri);
    b.extend([0x1D, 0x28, 0x6B, 3, 0, 0x31, 0x51, 0x30]);
    b.extend([0x1B, 0x61, 0]);
    b
}

pub fn escpos_nusha_tam(
    f: &Fis,
    genislik: usize,
    nusha: &str,
    qr: &str,
    aydinlatma: Option<&str>,
) -> Vec<u8> {
    escpos_nusha_takipli(f, genislik, nusha, qr, aydinlatma, None)
}

pub fn escpos_nusha_takipli(
    f: &Fis,
    genislik: usize,
    nusha: &str,
    qr: &str,
    aydinlatma: Option<&str>,
    takip: Option<&Takip>,
) -> Vec<u8> {
    const KUCUK: usize = 60; // Font B: 80 mm kâğıtta 64 sütun sığar, 60 güvenli sınır
    let g = genislik.max(24);
    let mut b = vec![0x1B, 0x40, 0x1B, 0x74, 13]; // başlat, kod sayfası 13 = CP857
    for (stil, m) in satirlar(f, nusha) {
        match stil {
            Stil::Ayrac => b.extend(std::iter::repeat_n(0xC4, g)), // ─
            Stil::Cift => b.extend(std::iter::repeat_n(0xCD, g)),  // ═
            Stil::Ham => b.extend(m.chars().map(cp857)),           // boşlukları olduğu gibi
            Stil::Logo => {
                b.extend(logo_baytlari());
                continue; // bitmap kendi yüksekliğini ilerletir, ek satır boşluğu istemez
            }
            Stil::Serit => {
                // ters baskı: siyah zemin, beyaz yazı; satır genişliğine ortalanıp doldurulur
                let ic = format!(" {m} ");
                let sol = g.saturating_sub(ic.chars().count()) / 2;
                let sag = g.saturating_sub(sol + ic.chars().count());
                b.extend([0x1D, 0x42, 1, 0x1B, 0x45, 1]);
                b.extend(" ".repeat(sol).chars().map(cp857));
                b.extend(ic.chars().map(cp857));
                b.extend(" ".repeat(sag).chars().map(cp857));
                b.extend([0x1D, 0x42, 0, 0x1B, 0x45, 0]);
            }
            _ => {
                let (genis, ayar): (usize, &[u8]) = match stil {
                    Stil::Kalin => (g, &[0x1B, 0x45, 1]),
                    Stil::OrtaKalin => (g, &[0x1B, 0x61, 1, 0x1B, 0x45, 1]),
                    Stil::OrtaKucuk => (KUCUK, &[0x1B, 0x61, 1, 0x1B, 0x4D, 1]),
                    Stil::OrtaBuyuk => (g / 2, &[0x1B, 0x61, 1, 0x1B, 0x45, 1, 0x1D, 0x21, 0x11]),
                    _ => (g, &[]),
                };
                b.extend(ayar);
                b.extend(sar(&m, genis).join("\n").chars().map(cp857));
                b.extend(*b"\n");
                // ayarları sıfırla: hizalama sola, kalın kapalı, Font A, boyut 1x
                b.extend([0x1B, 0x61, 0, 0x1B, 0x45, 0, 0x1B, 0x4D, 0, 0x1D, 0x21, 0]);
                continue;
            }
        }
        b.extend(*b"\n");
    }
    let kisa = musteri_nushasi(nusha) && kisa_musteri_nushasi();
    if kisa {
        b.extend([0x1B, 0x61, 1, 0x1B, 0x4D, 1]);
        b.extend(sar(KISA_KURAL, KUCUK).join("\n").chars().map(cp857));
        b.extend(*b"\n");
        b.extend([0x1B, 0x4D, 0, 0x1B, 0x61, 0]);
    } else if !terzihane_nushasi(nusha) {
        b.extend([0x1B, 0x4D, 1]);
        for c in [
            "Ürünlerinizi Saat 17:00'den itibaren teslim alabilirsiniz.",
            "14 Gün içinde alınmayan ürünlerden mesul değiliz.",
            "Ürünlerinizi teslim alırken tadilat fişinizi yanınızda bulundurmayı unutmayınız.",
        ] {
            b.extend(sar(c, KUCUK).join("\n").chars().map(cp857));
            b.extend(*b"\n");
        }
        b.extend([0x1B, 0x4D, 0]);
    }
    match takip.filter(|_| nusha.starts_with("MÜŞTERİ")) {
        Some(t) => {
            b.extend([0x1B, 0x61, 1, 0x1B, 0x45, 1]); // ortala, kalın
            b.extend(sar(t.cumle, g).join("\n").chars().map(cp857));
            b.extend(*b"\n");
            b.extend([0x1B, 0x45, 0, 0x1B, 0x61, 0]);
            b.extend(qr_baytlari_boyut(
                t.adres.as_bytes(),
                if kisa { 4 } else { 5 },
            ));
            b.extend(*b"\n");
            if let Some(y) = t.yedek {
                b.extend([0x1B, 0x61, 1, 0x1B, 0x4D, 1]); // ortala, küçük yazı
                b.extend(
                    sar(&format!("Açılmazsa: {y}"), KUCUK)
                        .join("\n")
                        .chars()
                        .map(cp857),
                );
                b.extend(*b"\n");
                b.extend([0x1B, 0x4D, 0, 0x1B, 0x61, 0]);
            }
        }
        None => {
            b.extend(qr_baytlari(qr.as_bytes()));
            b.extend(*b"\n");
        }
    }
    if let Some(url) = aydinlatma {
        if kisa && taa_ayar::ozellik_acik("fis-aydinlatma", true) {
            // Kısa nüshada ikinci karekod yok: tek küçük satır.
            b.extend([0x1B, 0x61, 1, 0x1B, 0x4D, 1]);
            b.extend(
                sar(&format!("{KISA_AYDINLATMA} {url}"), KUCUK)
                    .join("\n")
                    .chars()
                    .map(cp857),
            );
            b.extend(*b"\n");
            b.extend([0x1B, 0x4D, 0, 0x1B, 0x61, 0]);
        } else if nusha.starts_with("MÜŞTERİ") && taa_ayar::ozellik_acik("fis-aydinlatma", true)
        {
            b.extend([0x1B, 0x4D, 1]); // küçük yazı (Font B)
            b.extend(
                format!("{}\n", sar(AYDINLATMA_CUMLESI, KUCUK).join("\n"))
                    .chars()
                    .map(cp857),
            );
            b.extend([0x1B, 0x4D, 0]);
            b.extend(qr_baytlari(url.as_bytes()));
            b.extend(*b"\n");
        }
    }
    b.extend([0x1D, 0x56, 0x42, 0]); // kâğıdı kesme konumuna ilerletip kısmi kes (ek boş satır gerekmez)
    b
}
