// Sahip: Yusuf. yazan: [Claude#kur-ekrani 2026-11-15]
//! Kurulum ilerlemesi: aşama, program adımları (indir, doğrula, kur, ayarla), yüzde ve olay günlüğü. Saf mantık;
//! G/Ç yok. `taa-guncelle` (TAA_ILERLEME=1) satırlarını çözer, arayüz için JSON üretir. Bkz. docs/KURULUM-EKRANI.md.

/// `taa-guncelle` makine satırı öneki.
pub const ONEK: &str = "TAA-ILERLEME";
/// Olay günlüğünde tutulan en çok olay (eskiler düşer).
const OLAY_SINIRI: usize = 200;
/// Genel yüzdede çekirdek dosyaların payı; kalan programlarındır.
const CEKIRDEK_PAYI: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Asama {
    Hazirlik,
    Cekirdek,
    Secim,
    Programlar,
    Bitti,
}

impl Asama {
    pub fn kod(self) -> &'static str {
        match self {
            Asama::Hazirlik => "hazirlik",
            Asama::Cekirdek => "cekirdek",
            Asama::Secim => "secim",
            Asama::Programlar => "programlar",
            Asama::Bitti => "bitti",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Adim {
    Indir,
    Dogrula,
    Kur,
    Ayarla,
}

impl Adim {
    pub const HEPSI: [Adim; 4] = [Adim::Indir, Adim::Dogrula, Adim::Kur, Adim::Ayarla];
    pub fn kod(self) -> &'static str {
        match self {
            Adim::Indir => "indir",
            Adim::Dogrula => "dogrula",
            Adim::Kur => "kur",
            Adim::Ayarla => "ayarla",
        }
    }
    pub fn etiket(self) -> &'static str {
        match self {
            Adim::Indir => "İndir",
            Adim::Dogrula => "Doğrula",
            Adim::Kur => "Kur",
            Adim::Ayarla => "Ayarla",
        }
    }
    /// Program içindeki ağırlık (toplam 100).
    pub fn agirlik(self) -> u32 {
        match self {
            Adim::Indir => 50,
            Adim::Dogrula => 15,
            Adim::Kur => 25,
            Adim::Ayarla => 10,
        }
    }
    fn sira(self) -> usize {
        Adim::HEPSI.iter().position(|a| *a == self).unwrap_or(0)
    }
    pub fn coz(s: &str) -> Option<Adim> {
        Adim::HEPSI.into_iter().find(|a| a.kod() == s)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Durum {
    Bekliyor,
    Suruyor,
    Tamam,
    Hata,
}

impl Durum {
    pub fn kod(self) -> &'static str {
        match self {
            Durum::Bekliyor => "bekliyor",
            Durum::Suruyor => "suruyor",
            Durum::Tamam => "tamam",
            Durum::Hata => "hata",
        }
    }
}

/// `TAA-ILERLEME\t<paket>\t<adım>\t<basladi|bitti>` satırının çözülmüş hali.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Isaret {
    pub paket: String,
    pub adim: Adim,
    pub bitti: bool,
}

/// Makine satırını çözer; satır bizim değilse ya da bozuksa `None` (o zaman konsola aynen geçirilir).
pub fn isaret_coz(satir: &str) -> Option<Isaret> {
    let mut p = satir.trim_end_matches(['\r', '\n']).split('\t');
    if p.next()? != ONEK {
        return None;
    }
    let paket = p.next()?.trim();
    let adim = Adim::coz(p.next()?.trim())?;
    let bitti = match p.next()?.trim() {
        "basladi" => false,
        "bitti" => true,
        _ => return None,
    };
    if paket.is_empty() || p.next().is_some() {
        return None;
    }
    Some(Isaret { paket: paket.to_string(), adim, bitti })
}

/// Arayüze giden ilerleme olayı: hangi program, hangi adım, genel yüzde ve okunur mesaj.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Olay {
    pub sira: u64,
    pub program: String,
    pub adim: Option<Adim>,
    pub yuzde: u8,
    pub mesaj: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramDurumu {
    pub ad: String,
    pub adimlar: [Durum; 4],
    pub hata: Option<String>,
    pub deneme: u32,
    /// Seçilmedi, bağımlılık olarak geldi (ör. Tailscale).
    pub bagimlilik: bool,
}

impl ProgramDurumu {
    fn yeni(ad: &str, bagimlilik: bool) -> Self {
        ProgramDurumu { ad: ad.to_string(), adimlar: [Durum::Bekliyor; 4], hata: None, deneme: 1, bagimlilik }
    }
    /// Program içi yüzde (0-100): tamam adım tam, süren adım yarım ağırlık.
    pub fn yuzde(&self) -> u32 {
        Adim::HEPSI
            .iter()
            .zip(self.adimlar.iter())
            .map(|(a, d)| match d {
                Durum::Tamam => a.agirlik(),
                Durum::Suruyor => a.agirlik() / 2,
                _ => 0,
            })
            .sum()
    }
    pub fn durum(&self) -> Durum {
        if self.adimlar.contains(&Durum::Hata) {
            Durum::Hata
        } else if self.adimlar.iter().all(|d| *d == Durum::Tamam) {
            Durum::Tamam
        } else if self.adimlar.iter().all(|d| *d == Durum::Bekliyor) {
            Durum::Bekliyor
        } else {
            Durum::Suruyor
        }
    }
}

#[derive(Debug, Clone)]
pub struct Ilerleme {
    pub asama: Asama,
    pub cekirdek_toplam: u32,
    pub cekirdek_yazilan: u32,
    pub programlar: Vec<ProgramDurumu>,
    pub olaylar: Vec<Olay>,
    pub surum: String,
    /// Kurucu yeniden dene / bitir komutunu bekliyor.
    pub komut_bekleniyor: bool,
    sayac: u64,
}

impl Ilerleme {
    pub fn yeni(surum: &str) -> Self {
        Ilerleme {
            asama: Asama::Hazirlik,
            cekirdek_toplam: 0,
            cekirdek_yazilan: 0,
            programlar: vec![],
            olaylar: vec![],
            surum: surum.to_string(),
            komut_bekleniyor: false,
            sayac: 0,
        }
    }

    fn olay(&mut self, program: &str, adim: Option<Adim>, mesaj: String) {
        self.sayac += 1;
        let yuzde = self.yuzde();
        self.olaylar.push(Olay { sira: self.sayac, program: program.to_string(), adim, yuzde, mesaj });
        if self.olaylar.len() > OLAY_SINIRI {
            let fazla = self.olaylar.len() - OLAY_SINIRI;
            self.olaylar.drain(..fazla);
        }
    }

    pub fn asama(&mut self, a: Asama, mesaj: &str) {
        self.asama = a;
        self.olay("", None, mesaj.to_string());
    }

    pub fn cekirdek_basla(&mut self, toplam: u32) {
        self.cekirdek_toplam = toplam;
        self.cekirdek_yazilan = 0;
        self.asama(Asama::Cekirdek, "TAA Kur dosyaları yazılıyor");
    }

    pub fn cekirdek_dosya(&mut self) {
        self.cekirdek_yazilan = (self.cekirdek_yazilan + 1).min(self.cekirdek_toplam.max(1));
    }

    /// Kurulacak programı listeye ekler (zaten varsa dokunmaz).
    pub fn program_ekle(&mut self, ad: &str, bagimlilik: bool) {
        if !self.programlar.iter().any(|p| p.ad == ad) {
            self.programlar.push(ProgramDurumu::yeni(ad, bagimlilik));
        }
    }

    fn program_mut(&mut self, ad: &str) -> &mut ProgramDurumu {
        if let Some(i) = self.programlar.iter().position(|p| p.ad == ad) {
            &mut self.programlar[i]
        } else {
            // Bağımlılık, onu isteyen (süren ya da sıradaki) programın önünde görünür.
            let i = self
                .programlar
                .iter()
                .position(|p| matches!(p.durum(), Durum::Suruyor | Durum::Bekliyor))
                .unwrap_or(self.programlar.len());
            self.programlar.insert(i, ProgramDurumu::yeni(ad, true));
            &mut self.programlar[i]
        }
    }

    /// `taa-guncelle` işaretini uygular. Bir adım başlarken öncekiler "tamam" sayılır
    /// (ör. paket depoda zaten varsa indir atlanır).
    pub fn isaret_uygula(&mut self, i: &Isaret) {
        let p = self.program_mut(&i.paket);
        let s = i.adim.sira();
        for d in p.adimlar.iter_mut().take(s) {
            if *d != Durum::Hata {
                *d = Durum::Tamam;
            }
        }
        p.adimlar[s] = if i.bitti { Durum::Tamam } else { Durum::Suruyor };
        let mesaj = format!("{}: {} {}", i.paket, i.adim.etiket(), if i.bitti { "tamam" } else { "sürüyor" });
        self.olay(&i.paket, Some(i.adim), mesaj);
    }

    /// Program başarıyla bitti (çıkış kodu 0): kalan adımlar tamam.
    pub fn program_tamam(&mut self, ad: &str) {
        let p = self.program_mut(ad);
        p.adimlar = [Durum::Tamam; 4];
        p.hata = None;
        self.olay(ad, None, format!("{ad} kuruldu"));
    }

    /// Program başarısız: süren adım (yoksa ilk bekleyen) hata olur.
    pub fn program_hata(&mut self, ad: &str, mesaj: &str) {
        let p = self.program_mut(ad);
        let i = p
            .adimlar
            .iter()
            .position(|d| *d == Durum::Suruyor)
            .or_else(|| p.adimlar.iter().position(|d| *d == Durum::Bekliyor))
            .unwrap_or(0);
        p.adimlar[i] = Durum::Hata;
        let m = if mesaj.trim().is_empty() { "kurulamadı".to_string() } else { mesaj.trim().to_string() };
        p.hata = Some(m.clone());
        let adim = Adim::HEPSI[i];
        self.olay(ad, Some(adim), format!("{ad}: {} başarısız: {m}", adim.etiket()));
    }

    /// Yeniden deneme: adımlar sıfırlanır, deneme sayısı artar. Program yoksa ya da hatalı değilse `false`.
    pub fn yeniden_dene(&mut self, ad: &str) -> bool {
        let Some(p) = self.programlar.iter_mut().find(|p| p.ad == ad) else { return false };
        if p.durum() != Durum::Hata {
            return false;
        }
        p.adimlar = [Durum::Bekliyor; 4];
        p.hata = None;
        p.deneme += 1;
        let n = p.deneme;
        self.olay(ad, None, format!("{ad}: yeniden deneniyor ({n}. deneme)"));
        true
    }

    pub fn hatalilar(&self) -> Vec<String> {
        self.programlar.iter().filter(|p| p.durum() == Durum::Hata).map(|p| p.ad.clone()).collect()
    }

    /// Genel yüzde (0-100).
    pub fn yuzde(&self) -> u8 {
        if self.asama == Asama::Bitti && self.hatalilar().is_empty() {
            return 100;
        }
        let cekirdek = match self.asama {
            Asama::Hazirlik => 0,
            Asama::Cekirdek if self.cekirdek_toplam > 0 => CEKIRDEK_PAYI * self.cekirdek_yazilan / self.cekirdek_toplam,
            Asama::Cekirdek => 0,
            _ => CEKIRDEK_PAYI,
        };
        let programlar = if self.programlar.is_empty() {
            if matches!(self.asama, Asama::Bitti) { 100 - CEKIRDEK_PAYI } else { 0 }
        } else {
            let toplam: u32 = self.programlar.iter().map(ProgramDurumu::yuzde).sum();
            (100 - CEKIRDEK_PAYI) * toplam / (100 * self.programlar.len() as u32)
        };
        (cekirdek + programlar).min(100) as u8
    }

    /// Arayüz için JSON. `sonra`: bu sıradan sonraki olaylar (0 = hepsi).
    pub fn json(&self, sonra: u64) -> String {
        let mut s = String::with_capacity(512);
        s.push('{');
        s.push_str(&format!(
            "\"surum\":{},\"asama\":\"{}\",\"yuzde\":{},\"komut_bekleniyor\":{},\"cekirdek\":{{\"toplam\":{},\"yazilan\":{}}},",
            metin(&self.surum),
            self.asama.kod(),
            self.yuzde(),
            self.komut_bekleniyor,
            self.cekirdek_toplam,
            self.cekirdek_yazilan
        ));
        s.push_str("\"adimlar\":[");
        for (i, a) in Adim::HEPSI.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            s.push_str(&format!("{{\"kod\":\"{}\",\"etiket\":{}}}", a.kod(), metin(a.etiket())));
        }
        s.push_str("],\"programlar\":[");
        for (i, p) in self.programlar.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let adimlar: Vec<String> = p.adimlar.iter().map(|d| format!("\"{}\"", d.kod())).collect();
            s.push_str(&format!(
                "{{\"ad\":{},\"durum\":\"{}\",\"yuzde\":{},\"adimlar\":[{}],\"hata\":{},\"deneme\":{},\"bagimlilik\":{}}}",
                metin(&p.ad),
                p.durum().kod(),
                p.yuzde(),
                adimlar.join(","),
                p.hata.as_deref().map(metin).unwrap_or_else(|| "null".into()),
                p.deneme,
                p.bagimlilik
            ));
        }
        s.push_str("],\"olaylar\":[");
        let mut ilk = true;
        for o in self.olaylar.iter().filter(|o| o.sira > sonra) {
            if !ilk {
                s.push(',');
            }
            ilk = false;
            s.push_str(&format!(
                "{{\"sira\":{},\"program\":{},\"adim\":{},\"yuzde\":{},\"mesaj\":{}}}",
                o.sira,
                metin(&o.program),
                o.adim.map(|a| format!("\"{}\"", a.kod())).unwrap_or_else(|| "null".into()),
                o.yuzde,
                metin(&o.mesaj)
            ));
        }
        s.push_str("]}");
        s
    }
}

/// JSON metin değeri (tırnaklı, kaçışlı). `<` da kaçılır: çıktı HTML içine gömülse bile güvenli.
pub fn metin(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            '<' => o.push_str("\\u003c"),
            '>' => o.push_str("\\u003e"),
            '&' => o.push_str("\\u0026"),
            c if (c as u32) < 0x20 || c == '\u{2028}' || c == '\u{2029}' => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

#[cfg(test)]
mod testler {
    use super::*;

    fn isr(p: &str, a: &str, d: &str) -> Isaret {
        isaret_coz(&format!("{ONEK}\t{p}\t{a}\t{d}")).unwrap()
    }

    #[test]
    fn isaret_cozumu() {
        assert_eq!(
            isaret_coz("TAA-ILERLEME\ttutaa\tindir\tbasladi\r\n"),
            Some(Isaret { paket: "tutaa".into(), adim: Adim::Indir, bitti: false })
        );
        assert_eq!(isaret_coz("TAA-ILERLEME\ttutaa\tayarla\tbitti").unwrap().adim, Adim::Ayarla);
        for kotu in [
            "",
            "Kuruldu: tutaa 1.0",
            "TAA-ILERLEME\ttutaa\tindir",
            "TAA-ILERLEME\t\tindir\tbasladi",
            "TAA-ILERLEME\ttutaa\tucur\tbasladi",
            "TAA-ILERLEME\ttutaa\tindir\tbelki",
            "TAA-ILERLEME\ttutaa\tindir\tbitti\tfazla",
            "XTAA-ILERLEME\ttutaa\tindir\tbitti",
        ] {
            assert_eq!(isaret_coz(kotu), None, "{kotu:?}");
        }
    }

    #[test]
    fn agirliklar_yuz_eder() {
        assert_eq!(Adim::HEPSI.iter().map(|a| a.agirlik()).sum::<u32>(), 100);
    }

    #[test]
    fn adim_akisi_ve_yuzde() {
        let mut i = Ilerleme::yeni("1.0");
        assert_eq!(i.yuzde(), 0);
        i.cekirdek_basla(4);
        i.cekirdek_dosya();
        i.cekirdek_dosya();
        assert_eq!(i.yuzde(), 5);
        i.asama(Asama::Programlar, "programlar");
        i.program_ekle("tutaa", false);
        i.program_ekle("taarkod", false);
        assert_eq!(i.yuzde(), 10);
        i.isaret_uygula(&isr("tutaa", "indir", "basladi"));
        assert_eq!(i.programlar[0].durum(), Durum::Suruyor);
        assert_eq!(i.programlar[0].yuzde(), 25);
        i.isaret_uygula(&isr("tutaa", "indir", "bitti"));
        i.isaret_uygula(&isr("tutaa", "dogrula", "bitti"));
        assert_eq!(i.programlar[0].yuzde(), 65);
        i.program_tamam("tutaa");
        assert_eq!(i.programlar[0].durum(), Durum::Tamam);
        assert_eq!(i.yuzde(), 10 + 45);
        i.program_tamam("taarkod");
        assert_eq!(i.yuzde(), 100);
        let son = i.olaylar.last().unwrap();
        assert_eq!(son.yuzde, 100);
        assert_eq!(son.program, "taarkod");
    }

    #[test]
    fn sonraki_adim_oncekileri_tamamlar_ve_bagimlilik_eklenir() {
        let mut i = Ilerleme::yeni("1.0");
        i.program_ekle("tutaa", false);
        i.isaret_uygula(&isr("tailscale", "kur", "basladi"));
        let t = i.programlar.iter().find(|p| p.ad == "tailscale").unwrap();
        assert!(t.bagimlilik);
        assert_eq!(t.adimlar, [Durum::Tamam, Durum::Tamam, Durum::Suruyor, Durum::Bekliyor]);
        assert_eq!(i.programlar[0].ad, "tailscale", "bağımlılık isteyenin önünde");
        assert!(!i.programlar[1].bagimlilik);
    }

    #[test]
    fn hata_ve_yeniden_dene() {
        let mut i = Ilerleme::yeni("1.0");
        i.program_ekle("taadon", false);
        assert!(!i.yeniden_dene("taadon"), "hatasız program yeniden denenmez");
        assert!(!i.yeniden_dene("yok"));
        i.isaret_uygula(&isr("taadon", "dogrula", "basladi"));
        i.program_hata("taadon", "SHA256 uyuşmuyor\n");
        let p = &i.programlar[0];
        assert_eq!(p.durum(), Durum::Hata);
        assert_eq!(p.adimlar[1], Durum::Hata);
        assert_eq!(p.hata.as_deref(), Some("SHA256 uyuşmuyor"));
        assert_eq!(i.hatalilar(), vec!["taadon".to_string()]);
        i.asama(Asama::Bitti, "bitti");
        assert!(i.yuzde() < 100, "hata varken %100 gösterilmez");
        assert!(i.yeniden_dene("taadon"));
        let p = &i.programlar[0];
        assert_eq!((p.durum(), p.deneme, p.hata.clone()), (Durum::Bekliyor, 2, None));
        // Hiç işaret gelmeden hata: ilk adım hatalı olur, boş mesaj düzgün metne döner.
        i.program_hata("taadon", "  ");
        assert_eq!(i.programlar[0].adimlar[0], Durum::Hata);
        assert_eq!(i.programlar[0].hata.as_deref(), Some("kurulamadı"));
    }

    #[test]
    fn programsiz_bitti_yuz() {
        let mut i = Ilerleme::yeni("1.0");
        i.cekirdek_basla(0);
        assert_eq!(i.yuzde(), 0);
        i.asama(Asama::Bitti, "bitti");
        assert_eq!(i.yuzde(), 100);
    }

    #[test]
    fn olay_siniri_ve_sonraki_olaylar() {
        let mut i = Ilerleme::yeni("1.0");
        for _ in 0..(OLAY_SINIRI + 25) {
            i.asama(Asama::Programlar, "x");
        }
        assert_eq!(i.olaylar.len(), OLAY_SINIRI);
        assert_eq!(i.olaylar.last().unwrap().sira, (OLAY_SINIRI + 25) as u64);
        let j = i.json((OLAY_SINIRI + 23) as u64);
        assert_eq!(j.matches("\"sira\":").count(), 2);
    }

    #[test]
    fn json_bicimi_ve_kacis() {
        let mut i = Ilerleme::yeni("0.1\"x");
        i.program_ekle("a<b>", false);
        i.program_hata("a<b>", "yol \"C:\\x\"\tbitti");
        let j = i.json(0);
        assert!(j.starts_with("{\"surum\":\"0.1\\\"x\",\"asama\":\"hazirlik\""));
        assert!(j.contains("\"ad\":\"a\\u003cb\\u003e\""));
        assert!(j.contains("\"hata\":\"yol \\\"C:\\\\x\\\"\\tbitti\""));
        assert!(j.contains("\"adimlar\":[\"hata\",\"bekliyor\",\"bekliyor\",\"bekliyor\"]"));
        assert!(!j.contains('<'));
        assert_eq!(metin("\u{1}\u{2028}"), "\"\\u0001\\u2028\"");
        // Ayraçlar dengeli (kaba yapı denetimi).
        assert_eq!(j.matches('{').count(), j.matches('}').count());
        assert_eq!(j.matches('[').count(), j.matches(']').count());
    }
}
