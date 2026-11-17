// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz 2026-11-17] değiştiren: [Claude#kurucu-arayuz 2026-11-17] (program hatası sarılır)
//! Kurulum penceresinin GDI çizimi (yalnız Windows). Üç tema (KARAR-FABLE.md): İlmek (teyel -> dikiş, sağ sütun),
//! Kalıp (sol numaralı ray + çıt, alt 20 bölmeli çubuk), Mezura (büyük görev başlığı, cetvel, alt bilgi şeridi).
//! Gradyan, gölge, bitmap yok; her şey bellek DC'sine çizilir, `pencere.rs` tek BitBlt ile basar.
//! Renkler `0xRRGGBB` jetonlarıdır (`taa_kur_tema`); GDI'ye verilirken `colorref` ile çevrilir.
//! Saf yerleşim / durum hesapları `yerlesim.rs`'dedir (testli); burada yalnız çizim ve ölçüm vardır.
//! DİKKAT: Windows'ta elle DENENMEDİ; Linux'ta yalnız `cargo check/clippy --target x86_64-pc-windows-gnu`.

use crate::direktif::{Direktif, Durum as DDurum};
use crate::ekran::Paylasim;
use crate::ilerleme::{Adim, Asama, Durum, ProgramDurumu};
use crate::kartlar::{self, Kart};
use crate::pencere::{Hedef, PencereDurumu};
use crate::secim::Secim;
use crate::yerlesim::{self as y, AyarOge, Dik, Tanitim, Yerlesim, Yukleme};
use std::ptr::null_mut;
use taa_kur_tema::{ara_renk, colorref, etkin_jetonlar, simge, Geometri, Gorunum, Jetonlar, Mod, Tema, Tipografi};
use windows_sys::Win32::Foundation::POINT;
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontW, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, FillRect, GetDC, GetStockObject, GetTextFaceW,
    IntersectClipRect, Pie, Polygon, Polyline, ReleaseDC, RestoreDC, RoundRect, SaveDC, SelectObject, SetTextColor,
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DT_CALCRECT, DT_CENTER, DT_EDITCONTROL,
    DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, HDC, HFONT, NULL_BRUSH,
    OUT_DEFAULT_PRECIS, PS_SOLID,
};
use windows_sys::Win32::Foundation::RECT;

pub(crate) const TEK: u32 = DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS;
/// Ölçülerek yerleştirilen etiketler için: kesme yok.
pub(crate) const TEK_TAM: u32 = DT_SINGLELINE | DT_VCENTER;
pub(crate) const SARILI: u32 = DT_WORDBREAK | DT_EDITCONTROL;

fn genis(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn rc(d: Dik) -> RECT {
    RECT { left: d.x, top: d.y, right: d.sag(), bottom: d.alt() }
}

// ------------------------------------------------------------------ fontlar

#[derive(Clone, Copy)]
pub(crate) struct F {
    pub baslik: HFONT,
    pub baslik_dar: HFONT,
    pub bolum: HFONT,
    pub govde: HFONT,
    pub kalin: HFONT,
    pub kucuk: HFONT,
    pub kucuk_kalin: HFONT,
    pub dugme: HFONT,
    pub yuzde: HFONT,
    pub cetvel: HFONT,
    pub marka: HFONT,
}

/// Sahip olunan fontlar (Drop'ta silinir); çizimde kopyalanabilir tutamaklar (`.0`) kullanılır.
pub(crate) struct Fontlar(pub F);

/// Font yaratır; istenen aile sistemde yoksa (ör. Segoe UI Variable Display) `yedek` aileye düşer.
fn font(dpi: u32, px: i32, agirlik: i32, aile: &str, yedek: &str) -> HFONT {
    let h = -(px * dpi.max(96) as i32 / 96);
    let yap = |ad: &str| {
        let w = genis(ad);
        // SAFETY: ad sıfır sonlu; dönen tutamak Fontlar::drop'ta silinir.
        unsafe {
            CreateFontW(h, 0, 0, 0, agirlik, 0, 0, 0, DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32,
                CLIP_DEFAULT_PRECIS as u32, CLEARTYPE_QUALITY as u32, DEFAULT_PITCH as u32, w.as_ptr())
        }
    };
    let f = yap(aile);
    if aile == yedek {
        return f;
    }
    // GDI bilinmeyen aileyi sessizce başka yazı tipine eşler; seçilen yüzün adına bakılır.
    // SAFETY: ekran DC'si alınıp bırakılır; font yalnız ölçüm için seçilir ve geri alınır.
    let ayni = unsafe {
        let dc = GetDC(null_mut());
        let eski = SelectObject(dc, f);
        let mut ad = [0u16; 64];
        let n = GetTextFaceW(dc, ad.len() as i32, ad.as_mut_ptr());
        SelectObject(dc, eski);
        ReleaseDC(null_mut(), dc);
        n > 0 && String::from_utf16_lossy(&ad[..(n as usize).min(64)]).trim_end_matches('\0').eq_ignore_ascii_case(aile)
    };
    if ayni {
        f
    } else {
        // SAFETY: az önce yaratılan, seçili olmayan font.
        unsafe { DeleteObject(f) };
        yap(yedek)
    }
}

impl Fontlar {
    pub(crate) fn yap(dpi: u32, tema: Tema) -> Self {
        let t: Tipografi = tema.tipografi();
        let a = t.aile;
        let ba = t.baslik_aile;
        Fontlar(F {
            baslik: font(dpi, t.baslik.px, t.baslik.agirlik, ba, a),
            baslik_dar: font(dpi, t.baslik_dar.px, t.baslik_dar.agirlik, ba, a),
            bolum: font(dpi, t.bolum.px, t.bolum.agirlik, ba, a),
            govde: font(dpi, t.govde.px, t.govde.agirlik, a, a),
            kalin: font(dpi, t.govde.px, 600, a, a),
            kucuk: font(dpi, t.kucuk.px, t.kucuk.agirlik, a, a),
            kucuk_kalin: font(dpi, t.kucuk.px, 600, a, a),
            dugme: font(dpi, t.dugme.px, t.dugme.agirlik, a, a),
            yuzde: font(dpi, t.yuzde.px, t.yuzde.agirlik, a, a),
            cetvel: font(dpi, 14, 700, a, a),
            marka: font(dpi, 18, 600, ba, a),
        })
    }
}

impl Drop for Fontlar {
    fn drop(&mut self) {
        let t = self.0;
        for f in [t.baslik, t.baslik_dar, t.bolum, t.govde, t.kalin, t.kucuk, t.kucuk_kalin, t.dugme, t.yuzde, t.cetvel, t.marka] {
            // SAFETY: CreateFontW ile yaratılan, artık seçili olmayan tutamak.
            unsafe { DeleteObject(f) };
        }
    }
}

// ------------------------------------------------------------------ GDI yardımcıları

/// GDI yardımcıları: mantıksal birim (96 DPI) -> piksel, dolgu, çizgi, çokgen, metin.
pub(crate) struct Cizer {
    pub dc: HDC,
    pub dpi: i32,
}

/// Kırpma bölgesi; düşünce DC eski hâline döner.
pub(crate) struct Kirp<'a> {
    c: &'a Cizer,
    n: i32,
}

impl Drop for Kirp<'_> {
    fn drop(&mut self) {
        // SAFETY: SaveDC'nin döndürdüğü düzey.
        unsafe { RestoreDC(self.c.dc, self.n) };
    }
}

impl Cizer {
    pub(crate) fn px(&self, v: i32) -> i32 {
        v * self.dpi / 96
    }

    fn p1(&self) -> i32 {
        self.px(1).max(1)
    }

    pub(crate) fn kirp(&self, r: Dik) -> Kirp<'_> {
        // SAFETY: geçerli DC; Kirp düşünce RestoreDC.
        let n = unsafe {
            let n = SaveDC(self.dc);
            IntersectClipRect(self.dc, r.x, r.y, r.sag(), r.alt());
            n
        };
        Kirp { c: self, n }
    }

    pub(crate) fn doldur(&self, r: Dik, renk: u32) {
        if r.bos() {
            return;
        }
        // SAFETY: geçerli DC; fırça hemen silinir.
        unsafe {
            let b = CreateSolidBrush(colorref(renk));
            FillRect(self.dc, &rc(r), b);
            DeleteObject(b);
        }
    }

    /// Dikdörtgen çerçeve (içeri doğru `k` kalınlık), keskin köşe.
    pub(crate) fn cerceve(&self, r: Dik, k: i32, renk: u32) {
        let k = k.max(1);
        self.doldur(Dik::yeni(r.x, r.y, r.w, k), renk);
        self.doldur(Dik::yeni(r.x, r.alt() - k, r.w, k), renk);
        self.doldur(Dik::yeni(r.x, r.y, k, r.h), renk);
        self.doldur(Dik::yeni(r.sag() - k, r.y, k, r.h), renk);
    }

    /// Yuvarlak köşeli (yarıçap 0 ise keskin) kutu; `kenar` None ise çerçevesiz.
    pub(crate) fn kutu(&self, r: Dik, yaricap: i32, dolgu: u32, kenar: Option<(u32, i32)>) {
        if r.bos() {
            return;
        }
        if yaricap <= 0 {
            self.doldur(r, dolgu);
            if let Some((k, kk)) = kenar {
                self.cerceve(r, kk, k);
            }
            return;
        }
        let (kr, kk) = kenar.unwrap_or((dolgu, 1));
        // Kalın kalem kenarın ortasından çizer; kutunun içinde kalsın diye içeri alınır.
        let i = (kk - 1) / 2;
        let r2 = r.ic(i);
        // SAFETY: geçerli DC; seçilen nesneler geri seçilip silinir.
        unsafe {
            let kalem = CreatePen(PS_SOLID, kk.max(1), colorref(kr));
            let firca = CreateSolidBrush(colorref(dolgu));
            let ek = SelectObject(self.dc, kalem);
            let ef = SelectObject(self.dc, firca);
            RoundRect(self.dc, r2.x, r2.y, r2.sag(), r2.alt(), yaricap * 2, yaricap * 2);
            SelectObject(self.dc, ek);
            SelectObject(self.dc, ef);
            DeleteObject(kalem);
            DeleteObject(firca);
        }
    }

    /// İçi boş yuvarlak çerçeve (odak halkası).
    pub(crate) fn yuvarlak_cerceve(&self, r: Dik, yaricap: i32, k: i32, renk: u32) {
        if yaricap <= 0 {
            self.cerceve(r, k, renk);
            return;
        }
        let i = (k - 1) / 2;
        let r2 = r.ic(i);
        // SAFETY: geçerli DC; stok fırça silinmez, kalem silinir.
        unsafe {
            let kalem = CreatePen(PS_SOLID, k.max(1), colorref(renk));
            let ek = SelectObject(self.dc, kalem);
            let ef = SelectObject(self.dc, GetStockObject(NULL_BRUSH));
            RoundRect(self.dc, r2.x, r2.y, r2.sag(), r2.alt(), yaricap * 2, yaricap * 2);
            SelectObject(self.dc, ek);
            SelectObject(self.dc, ef);
            DeleteObject(kalem);
        }
    }

    pub(crate) fn daire(&self, cx: i32, cy: i32, r: i32, dolgu: u32, kenar: u32, kalinlik: i32) {
        self.kutu(Dik::yeni(cx - r, cy - r, 2 * r + 1, 2 * r + 1), r + 1, dolgu, Some((kenar, kalinlik)));
    }

    pub(crate) fn halka(&self, cx: i32, cy: i32, r: i32, renk: u32, kalinlik: i32) {
        self.yuvarlak_cerceve(Dik::yeni(cx - r, cy - r, 2 * r + 1, 2 * r + 1), r + 1, kalinlik, renk);
    }

    pub(crate) fn yatay(&self, x1: i32, x2: i32, yy: i32, k: i32, renk: u32) {
        if x2 > x1 {
            self.doldur(Dik::yeni(x1, yy - k / 2, x2 - x1, k.max(1)), renk);
        }
    }

    pub(crate) fn dikey(&self, xx: i32, y1: i32, y2: i32, k: i32, renk: u32) {
        if y2 > y1 {
            self.doldur(Dik::yeni(xx - k / 2, y1, k.max(1), y2 - y1), renk);
        }
    }

    /// İlmek teyeli: 8 px çizgi + 6 px boşluk, elle (PS_DASH kalın kalemde çalışmaz). Faz `x1`'e bağlıdır.
    pub(crate) fn teyel(&self, x1: i32, x2: i32, yy: i32, k: i32, renk: u32) {
        let parca = self.px(14).max(2);
        let cizgi = self.px(8).max(1);
        let mut x = x1;
        while x < x2 {
            self.yatay(x, (x + cizgi).min(x2), yy, k, renk);
            x += parca;
        }
    }

    pub(crate) fn cizgi(&self, noktalar: &[(i32, i32)], renk: u32, kalinlik: i32) {
        let p: Vec<POINT> = noktalar.iter().map(|(x, y)| POINT { x: *x, y: *y }).collect();
        // SAFETY: geçerli DC ve nokta dizisi.
        unsafe {
            let kalem = CreatePen(PS_SOLID, kalinlik.max(1), colorref(renk));
            let ek = SelectObject(self.dc, kalem);
            Polyline(self.dc, p.as_ptr(), p.len() as i32);
            SelectObject(self.dc, ek);
            DeleteObject(kalem);
        }
    }

    pub(crate) fn cokgen(&self, noktalar: &[(i32, i32)], renk: u32) {
        if noktalar.len() < 3 {
            return;
        }
        let p: Vec<POINT> = noktalar.iter().map(|(x, y)| POINT { x: *x, y: *y }).collect();
        // SAFETY: geçerli DC ve nokta dizisi; nesneler geri seçilip silinir.
        unsafe {
            let kalem = CreatePen(PS_SOLID, 1, colorref(renk));
            let firca = CreateSolidBrush(colorref(renk));
            let ek = SelectObject(self.dc, kalem);
            let ef = SelectObject(self.dc, firca);
            Polygon(self.dc, p.as_ptr(), p.len() as i32);
            SelectObject(self.dc, ek);
            SelectObject(self.dc, ef);
            DeleteObject(kalem);
            DeleteObject(firca);
        }
    }

    /// Dairenin sağ üst çeyreği dolu (◔).
    pub(crate) fn ceyrek(&self, cx: i32, cy: i32, r: i32, renk: u32) {
        // SAFETY: geçerli DC; nesneler geri seçilip silinir.
        unsafe {
            let kalem = CreatePen(PS_SOLID, 1, colorref(renk));
            let firca = CreateSolidBrush(colorref(renk));
            let ek = SelectObject(self.dc, kalem);
            let ef = SelectObject(self.dc, firca);
            Pie(self.dc, cx - r, cy - r, cx + r + 1, cy + r + 1, cx + r, cy, cx, cy - r);
            SelectObject(self.dc, ek);
            SelectObject(self.dc, ef);
            DeleteObject(kalem);
            DeleteObject(firca);
        }
    }

    /// Metin yazar; `bayrak` DT_*; dönen: kullanılan yükseklik (piksel).
    pub(crate) fn yazi(&self, metin: &str, font: HFONT, renk: u32, r: Dik, bayrak: u32) -> i32 {
        // Boş dizgide Vec işaretçisi sarkık olur (0x2); DrawTextW'ye hiç verilmez.
        if metin.is_empty() || r.bos() {
            return 0;
        }
        let w: Vec<u16> = metin.encode_utf16().collect();
        let mut rr = rc(r);
        // SAFETY: geçerli DC, font ve tampon.
        unsafe {
            let ef = SelectObject(self.dc, font);
            SetTextColor(self.dc, colorref(renk));
            let h = DrawTextW(self.dc, w.as_ptr(), w.len() as i32, &mut rr, bayrak | DT_NOPREFIX);
            SelectObject(self.dc, ef);
            h
        }
    }

    /// Sarılan metnin yüksekliği.
    pub(crate) fn olc(&self, metin: &str, font: HFONT, genislik: i32) -> i32 {
        if metin.is_empty() {
            return 0;
        }
        let w: Vec<u16> = metin.encode_utf16().collect();
        let mut r = RECT { left: 0, top: 0, right: genislik.max(1), bottom: 0 };
        // SAFETY: geçerli DC, font ve tampon; DT_CALCRECT çizmez.
        unsafe {
            let ef = SelectObject(self.dc, font);
            DrawTextW(self.dc, w.as_ptr(), w.len() as i32, &mut r, DT_CALCRECT | DT_WORDBREAK | DT_EDITCONTROL | DT_NOPREFIX);
            SelectObject(self.dc, ef);
        }
        r.bottom - r.top
    }

    /// Tek satırlık metnin genişliği.
    pub(crate) fn en(&self, metin: &str, font: HFONT) -> i32 {
        if metin.is_empty() {
            return 0;
        }
        let w: Vec<u16> = metin.encode_utf16().collect();
        let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: bkz. olc.
        unsafe {
            let ef = SelectObject(self.dc, font);
            DrawTextW(self.dc, w.as_ptr(), w.len() as i32, &mut r, DT_CALCRECT | DT_SINGLELINE | DT_NOPREFIX);
            SelectObject(self.dc, ef);
        }
        r.right - r.left
    }

    /// Onay işareti.
    pub(crate) fn tik(&self, cx: i32, cy: i32, r: i32, renk: u32) {
        let k = |v: i32| v * r / 10;
        self.cizgi(&[(cx - k(5), cy), (cx - k(1), cy + k(4)), (cx + k(5), cy - k(4))], renk, self.px(2));
    }
}

// ------------------------------------------------------------------ çizim bağlamı

/// Bir karede kullanılan tema bilgisi (kopyalanabilir; `PencereDurumu` ödünç alınmadan kullanılır).
#[derive(Clone, Copy)]
pub(crate) struct Ctx {
    pub tema: Tema,
    pub koyu: bool,
    pub j: Jetonlar,
    pub g: Geometri,
    pub f: F,
    pub azalt: bool,
    pub tik: u64,
    pub dpi: u32,
}

impl Ctx {
    pub(crate) fn yap(d: &PencereDurumu) -> Ctx {
        let mut j = d.j;
        // İlmek geçişi: dolgular eski -> %50 -> yeni; metin renkleri hemen yeni (alfa metne uygulanmaz).
        if let Some(t) = d.gecis.and_then(|g| g.harman()) {
            let e = d.eski_j;
            j.zemin = ara_renk(e.zemin, j.zemin, t);
            j.yuzey = ara_renk(e.yuzey, j.yuzey, t);
            j.yuzey2 = ara_renk(e.yuzey2, j.yuzey2, t);
            j.kenar = ara_renk(e.kenar, j.kenar, t);
            j.vurgu = ara_renk(e.vurgu, j.vurgu, t);
        }
        Ctx { tema: d.tema, koyu: d.koyu, j, g: d.tema.geometri(), f: d.fontlar.0, azalt: d.azalt(), tik: d.tik, dpi: d.dpi.max(96) }
    }

    /// İnce çizgi / küçük gösterge rengi: vurgu; Mezura açıkta sarı tek başına gösterge olamaz, metin rengi.
    fn gosterge(&self) -> u32 {
        gosterge(self.tema, self.koyu, &self.j)
    }
}

fn gosterge(tema: Tema, koyu: bool, j: &Jetonlar) -> u32 {
    if tema.vurgu_cerceveli(koyu) {
        j.metin
    } else {
        j.vurgu
    }
}

/// Vurgu dolgusu; Mezura açıkta her zaman 2 px `kenar` çerçeveli (KARAR §4.1, §5.5).
fn vurgu_dolgu(c: &Cizer, tema: Tema, koyu: bool, j: &Jetonlar, r: Dik, yaricap: i32) {
    let kenar = if tema.vurgu_cerceveli(koyu) { Some((j.kenar, c.px(2))) } else { None };
    c.kutu(r, yaricap, j.vurgu, kenar);
}

/// Görünüm tercihinden başka bir temanın jetonları (Ayarlar önizlemesi; özel vurgu dahil).
fn tema_jetonlari(g: &Gorunum, tema: Tema, koyu: bool) -> Jetonlar {
    let mut g2 = g.clone();
    g2.tema = Some(tema);
    g2.modu = if koyu { Mod::Koyu } else { Mod::Acik };
    etkin_jetonlar(&g2, koyu, false).1
}

fn dugme_gen(c: &Cizer, x: &Ctx, etiket: &str) -> i32 {
    (c.en(etiket, x.f.dugme) + 2 * c.px(x.g.dugme_yatay_ic)).max(c.px(x.g.dugme_en_az))
}

/// Klavye odağı: 2 px; Mezura'da 2 px boşluk + çift çerçeve.
fn odak_ciz(c: &Cizer, x: &Ctx, r: Dik, yaricap: i32) {
    let k = c.px(2);
    if x.tema == Tema::Mezura {
        c.cerceve(r.ic(-2 * k), k, if x.koyu { x.j.vurgu } else { x.j.zemin });
        c.cerceve(r.ic(-3 * k), k, x.j.metin);
    } else {
        c.yuvarlak_cerceve(r.ic(-k - c.p1()), yaricap + k, k, x.gosterge());
    }
}

/// Durum işareti (○ Bekliyor, ◔ Sürüyor, ✓ Tamam, ! Hata) elle çizilir; yazı tipine güvenilmez.
fn isaret(c: &Cizer, x: &Ctx, cx: i32, cy: i32, r: i32, d: Durum) {
    let j = &x.j;
    match d {
        Durum::Bekliyor => c.halka(cx, cy, r, j.kenar, c.px(2)),
        Durum::Suruyor => {
            c.halka(cx, cy, r, x.gosterge(), c.px(2));
            c.ceyrek(cx, cy, r, x.gosterge());
        }
        Durum::Tamam => {
            c.daire(cx, cy, r, j.basari, j.basari, 1);
            c.tik(cx, cy, r, taa_kur_tema::ustune_yazi(j.basari));
        }
        Durum::Hata => {
            c.daire(cx, cy, r, j.hata, j.hata, 1);
            let u = taa_kur_tema::ustune_yazi(j.hata);
            let k = c.px(2).max(2);
            c.doldur(Dik::yeni(cx - k / 2, cy - r * 6 / 10, k, r * 7 / 10), u);
            c.doldur(Dik::yeni(cx - k / 2, cy + r * 3 / 10, k, k), u);
        }
    }
}

fn durum_rengi(j: &Jetonlar, d: Durum) -> u32 {
    match d {
        Durum::Bekliyor => j.soluk,
        Durum::Suruyor => j.metin,
        Durum::Tamam => j.basari,
        Durum::Hata => j.hata,
    }
}

/// İşaret + metin; dönen genişlik.
fn durum_etiketi(c: &Cizer, x: &Ctx, xx: i32, cy: i32, d: Durum, font: HFONT) -> i32 {
    let r = c.px(7);
    isaret(c, x, xx + r, cy, r, d);
    let m = y::durum_metni(d);
    let w = c.en(m, font);
    c.yazi(m, font, durum_rengi(&x.j, d), Dik::yeni(xx + 2 * r + c.px(6), cy - c.px(12), w + c.px(2), c.px(24)), TEK_TAM);
    2 * r + c.px(6) + w
}

// ------------------------------------------------------------------ düğme

#[allow(clippy::too_many_arguments)]
fn dugme_ciz(c: &Cizer, d: &mut PencereDurumu, x: &Ctx, r: Dik, etiket: &str, birincil: bool, hedef: Hedef, secili: bool) {
    let ust = d.uzerinde == Some(hedef);
    let odak = d.odakli_hedef() == Some(hedef);
    let j = x.j;
    let rad = c.px(x.g.dugme_yaricap);
    let yazi_r = if birincil {
        let dolgu = if ust { ara_renk(j.vurgu, j.metin, 0.15) } else { j.vurgu };
        let kenar = if x.tema == Tema::Mezura { Some((j.kenar, c.px(2))) } else { None };
        c.kutu(r, rad, dolgu, kenar);
        j.vurgu_ustu
    } else {
        let k = if x.tema == Tema::Mezura { c.px(2) } else { 1 };
        let dolgu = if ust || secili {
            j.yuzey2
        } else if x.tema == Tema::Ilmek {
            j.zemin
        } else {
            j.yuzey
        };
        c.kutu(r, rad, dolgu, Some((j.kenar, k)));
        j.metin
    };
    let ic = c.px(8);
    if c.en(etiket, x.f.dugme) <= r.w - 2 * ic {
        c.yazi(etiket, x.f.dugme, yazi_r, r, TEK_TAM | DT_CENTER);
    } else {
        // Etiket kesilmez: iki satıra iner.
        let hh = c.olc(etiket, x.f.dugme, r.w - 2 * ic);
        c.yazi(etiket, x.f.dugme, yazi_r, Dik::yeni(r.x + ic, r.y + (r.h - hh) / 2, r.w - 2 * ic, hh), SARILI | DT_CENTER);
    }
    if odak {
        odak_ciz(c, x, r, rad);
    }
    d.hedefler.push((r, hedef));
}

// ------------------------------------------------------------------ ana çizim

pub(crate) fn ciz(c: &Cizer, d: &mut PencereDurumu, p: &Paylasim, w: i32, h: i32) {
    let x = Ctx::yap(d);
    d.hedefler.clear();
    d.kart_alani = Dik::default();
    c.doldur(Dik::yeni(0, 0, w, h), x.j.zemin);
    if let Some(yk) = d.yukleme {
        yukleme_ciz(c, &x, yk, w, h, yk.baslatiliyor_mu(hazir_mi(p)));
        return;
    }
    let kart_var = kart_gosterilir(p) && !d.kartlar.is_empty();
    let kh = if kart_var { kart_h_al(c, d, &x, w, h) } else { 0 };
    let l = y::yerlesim(x.tema, w, h, d.dpi, kh);
    match x.tema {
        Tema::Ilmek => ilmek_ust(c, &x, p, &l),
        Tema::Kalip => kalip_ust(c, &x, p, &l),
        Tema::Mezura => mezura_ust(c, d, &x, p, &l),
    }
    baslik_ciz(c, d, &x, p, l.baslik);
    govde_ciz(c, d, &x, p, &l);
    rehber_ciz(c, &x, &p.direktifler, &l);
    if kart_var {
        kart_ciz(c, d, &x, &l);
    }
    alt_ciz(c, d, &x, p, l.alt);
    if d.ayarlar_acik {
        ayar_paneli_ciz(c, d, &x, y::ayar_paneli(w, h, d.dpi, l.baslik.alt()));
    }
    if let Some(t) = d.tanitim {
        tanitim_ciz(c, d, &x, t, w, h);
    }
}

/// Kurulum ilk ekranını hazırladı mı? (yükleme ekranı bundan önce kalkmaz)
pub(crate) fn hazir_mi(p: &Paylasim) -> bool {
    p.secim.is_some() || p.hata.is_some() || p.cikis.is_some() || p.ilerleme.asama != Asama::Hazirlik || !p.ilerleme.programlar.is_empty()
}

/// Bekleme kartı yalnız beklerken: seçimde, hatada ve bitince gizli.
fn kart_gosterilir(p: &Paylasim) -> bool {
    p.hata.is_none() && p.secim.is_none() && p.ilerleme.asama != Asama::Bitti
}

fn kart_h_al(c: &Cizer, d: &mut PencereDurumu, x: &Ctx, w: i32, h: i32) -> i32 {
    let kw = y::kart_genisligi(x.tema, w, h, d.dpi);
    let anahtar = (x.tema, kw, d.dpi);
    if let Some((a, v)) = d.kart_h {
        if a == anahtar {
            return v;
        }
    }
    let v = d.kartlar.iter().map(|k| kart_olc(c, x, k, kw).toplam).max().unwrap_or(0);
    d.kart_h = Some((anahtar, v));
    v
}

fn logo_ciz(c: &Cizer, tema: Tema, koyu: bool, j: &Jetonlar, boyut: i32, lx: i32, ly: i32) {
    // T hiçbir zaman vurgu rengiyle boyanmaz; Mezura sarısı açık zemine doğrudan basılmaz (koyu kare üstünde).
    // HD: kenar yumuşatmalı tarama + AlphaBlend (GDI Polygon köşeleri piksele yuvarlayıp tırtıklı çiziyordu).
    let mut cizim = simge::LogoCizimi::yeni(boyut.max(1) as u32, j.metin, j.vurgu);
    if tema == Tema::Mezura && !koyu {
        cizim.ana = j.zemin;
        cizim.karo = Some(simge::Karo { renk: j.metin, yaricap: 0.0, pay: 0.0 });
    }
    // SAFETY: geçerli bellek DC'si (çift tampon).
    unsafe { crate::hd::logo_bas(c.dc, lx, ly, &cizim) };
}

fn kenar_payi(c: &Cizer, x: &Ctx) -> i32 {
    c.px(if x.tema == Tema::Kalip { 16 } else { 24 })
}

fn baslik_ciz(c: &Cizer, d: &mut PencereDurumu, x: &Ctx, p: &Paylasim, b: Dik) {
    let j = x.j;
    let m = kenar_payi(c, x);
    if x.tema == Tema::Kalip {
        c.doldur(b, j.yuzey);
        c.yatay(0, b.w, b.alt() - 1, 1, j.kenar);
    }
    let ls = c.px(28);
    logo_ciz(c, x.tema, x.koyu, &j, ls, m, b.y + (b.h - ls) / 2);
    let tx = m + ls + c.px(10);
    let mw = c.en("TAA Kurucu", x.f.marka);
    c.yazi("TAA Kurucu", x.f.marka, j.metin, Dik::yeni(tx, b.y, mw + c.px(2), b.h), TEK_TAM);
    let et = "Ayarlar";
    let bw = dugme_gen(c, x, et);
    let bh = c.px(x.g.dugme).min(b.h - c.px(12));
    let br = Dik::yeni(b.w - m - bw, b.y + (b.h - bh) / 2, bw, bh);
    let acik = d.ayarlar_acik;
    dugme_ciz(c, d, x, br, et, false, Hedef::Ayarlar, acik);
    let ix = tx + mw + c.px(14);
    // Klasör yolu "…" ile kısalabilir (KARAR §1: kesme yalnız paket adı ve klasör yolunda).
    c.yazi(&format!("sürüm {} · {}", p.ilerleme.surum, p.hedef), x.f.kucuk, j.soluk,
        Dik::yeni(ix, b.y, (br.x - c.px(16) - ix).max(0), b.h), TEK);
}

// ------------------------------------------------------------------ tema imzaları (önizleme ve yüklemede de kullanılır)

/// İlmek: `x1`-`bitis` dikiş (düz vurgu), `bitis`-`x2` teyel (kesik kenar).
fn teyel_hat(c: &Cizer, j: &Jetonlar, x1: i32, x2: i32, yy: i32, bitis: i32, k: i32) {
    let b = bitis.min(x2).max(x1);
    c.yatay(x1, b, yy, k, j.vurgu);
    c.teyel(b, x2, yy, k, j.kenar);
}

/// Kalıp: 20 bölmeli çubuk, 2 px aralık.
fn bolmeli_ciz(c: &Cizer, j: &Jetonlar, r: Dik, dolu: i32) {
    let ara = c.px(2).max(1);
    let hw = (r.w - ara * (y::KALIP_HUCRE - 1)) / y::KALIP_HUCRE;
    if hw <= 0 {
        return;
    }
    for i in 0..y::KALIP_HUCRE {
        let h = Dik::yeni(r.x + i * (hw + ara), r.y, hw, r.h);
        if i < dolu {
            c.doldur(h, j.vurgu);
        } else {
            c.kutu(h, 0, j.yuzey2, Some((j.kenar, 1)));
        }
    }
}

/// Kalıp çıtı: hücrenin sağ kenarında iş alanına bakan dolu üçgen.
fn cit(c: &Cizer, renk: u32, rx: i32, cy: i32, boy: i32) {
    c.cokgen(&[(rx, cy - boy), (rx + boy, cy), (rx, cy + boy)], renk);
}

/// Mezura cetveli: üstte imleç payı (10 px), 14 px sarı bant (2 px kenar çerçeveli), 2 px taban, çentikler,
/// %10'ların altında sayı. `sayi_font` None ise sayı yazılmaz (önizleme).
#[allow(clippy::too_many_arguments)]
fn cetvel_ciz(c: &Cizer, tema: Tema, koyu: bool, j: &Jetonlar, r: Dik, yuzde: u8, imlec: bool, sayi_font: Option<HFONT>, dpi: u32) {
    let pay = match sayi_font {
        Some(f) => c.en("100", f) / 2 + c.px(2),
        None => c.px(2),
    };
    let x0 = r.x + pay;
    let rw = (r.w - 2 * pay).max(1);
    let ust = r.y + c.px(10);
    let bant = Dik::yeni(x0, ust, rw * yuzde.min(100) as i32 / 100, c.px(14));
    if bant.w > 0 {
        c.doldur(bant, j.vurgu);
        c.cerceve(bant, c.px(2), j.kenar);
    }
    let taban = bant.alt();
    c.doldur(Dik::yeni(x0, taban, rw, c.px(2)), j.kenar);
    let cy = taban + c.px(2);
    for ct in y::cetvel_centikleri(rw, dpi) {
        let cx = y::cetvel_x(x0, rw - c.p1(), ct.yuzde);
        let boy = c.px(if ct.uzun { 12 } else { 5 });
        c.doldur(Dik::yeni(cx, cy, c.p1(), boy), j.kenar);
        if let (true, Some(f)) = (ct.uzun, sayi_font) {
            let s = ct.yuzde.to_string();
            let sw = c.en(&s, f);
            c.yazi(&s, f, j.metin, Dik::yeni(cx - sw / 2, cy + boy + c.px(1), sw + c.px(2), c.px(18)), TEK_TAM);
        }
    }
    if imlec {
        let ix = y::cetvel_x(x0, rw, yuzde);
        let renk = gosterge(tema, koyu, j);
        c.cokgen(&[(ix - c.px(5), ust - c.px(10)), (ix + c.px(5), ust - c.px(10)), (ix, ust)], renk);
    }
}

// ------------------------------------------------------------------ İlmek

fn ilmek_ust(c: &Cizer, x: &Ctx, p: &Paylasim, l: &Yerlesim) {
    let j = x.j;
    let i = &p.ilerleme;
    let simdi = i.asama.sira();
    let bitti = i.asama == Asama::Bitti;
    // Aşama dikiş hattı: bitmiş aralık düz vurgu, kalan teyel; düğüm: tamam dolu, etkin çift halka, bekleyen boş.
    let a = l.asama;
    let r = c.px(6);
    let ny = a.y + c.px(14);
    let n = Asama::HEPSI.len() as i32;
    let nx = |k: i32| a.x + r + k * (a.w - 2 * r) / (n - 1);
    for k in 0..n - 1 {
        let (x1, x2) = (nx(k) + r + c.px(4), nx(k + 1) - r - c.px(4));
        if (k + 1) as usize <= simdi {
            c.yatay(x1, x2, ny, c.px(2), j.vurgu);
        } else {
            c.teyel(x1, x2, ny, c.px(2), j.kenar);
        }
    }
    for (k, asama) in Asama::HEPSI.iter().enumerate() {
        let cx = nx(k as i32);
        let etkin = k == simdi && !bitti;
        if k < simdi || (k == simdi && bitti) {
            c.daire(cx, ny, r, j.vurgu, j.vurgu, 1);
        } else if etkin {
            // Yavaş halka nabzı (960 ms, iki ton arası); hareketi azaltta sabit.
            let ton = ara_renk(j.vurgu, ara_renk(j.vurgu, j.zemin, 0.55), y::nabiz(x.tik, x.azalt));
            c.halka(cx, ny, r + c.px(2), ton, c.px(2));
            c.daire(cx, ny, r - c.px(2), j.vurgu, j.vurgu, 1);
        } else {
            c.daire(cx, ny, r, j.zemin, j.kenar, c.px(2));
        }
        let et = asama.etiket();
        let font = if etkin { x.f.kalin } else { x.f.govde };
        let ew = c.en(et, font) + c.px(2);
        let ex = (cx - ew / 2).min(a.sag() - ew).max(a.x);
        c.yazi(et, font, if k <= simdi { j.metin } else { j.soluk }, Dik::yeni(ex, ny + r + c.px(6), ew, c.px(24)), TEK_TAM);
    }
    // Genel ilerleme: yüzde arttıkça teyel parçaları tek tek dikişe döner (parça 14 px).
    let pr = l.ilerleme;
    let yuzde = i.yuzde();
    let et = "Genel kurulum";
    let lw = c.en(et, x.f.kucuk);
    c.yazi(et, x.f.kucuk, j.soluk, Dik::yeni(pr.x, pr.y, lw + c.px(2), c.px(22)), TEK_TAM);
    let pw = c.en("%100", x.f.yuzde);
    c.yazi(&format!("%{yuzde}"), x.f.yuzde, j.metin, Dik::yeni(pr.sag() - pw, pr.y - c.px(2), pw, c.px(26)), TEK_TAM | DT_RIGHT);
    let x1 = pr.x + lw + c.px(16);
    let x2 = pr.sag() - pw - c.px(16);
    let toplam = y::teyel_parcalari(x2 - x1, x.dpi);
    let dikili = y::dikili_parca(toplam, yuzde);
    let bitis = if dikili >= toplam { x2 } else { x1 + dikili * c.px(14) };
    teyel_hat(c, &j, x1, x2, pr.y + c.px(11), bitis, c.px(2));
    c.yazi(i.son_mesaj(), x.f.kucuk, j.soluk, Dik::yeni(pr.x, pr.y + c.px(24), pr.w, c.px(20)), TEK);
}

// ------------------------------------------------------------------ Kalıp

fn kalip_ust(c: &Cizer, x: &Ctx, p: &Paylasim, l: &Yerlesim) {
    let j = x.j;
    let i = &p.ilerleme;
    let simdi = i.asama.sira();
    let bitti = i.asama == Asama::Bitti;
    // Sol ray: numaralı beş hücre; etkin hücrede solda 3 px vurgu şeridi ve sağ kenarda 8 px çıt.
    let a = l.asama;
    c.doldur(a, j.yuzey);
    c.dikey(a.sag() - 1, a.y, a.alt(), 1, j.kenar);
    c.dikey(l.rehber.x, l.rehber.y, l.rehber.alt(), 1, j.kenar);
    let hh = c.px(48);
    let nw = c.en("00", x.f.yuzde);
    for (k, asama) in Asama::HEPSI.iter().enumerate() {
        let h = Dik::yeni(a.x, a.y + c.px(8) + k as i32 * hh, a.w - 1, hh);
        if h.alt() > a.alt() {
            break;
        }
        let etkin = k == simdi;
        let tamam = k < simdi || (etkin && bitti);
        if etkin {
            c.doldur(h, j.yuzey2);
            c.doldur(Dik::yeni(h.x, h.y, c.px(3), h.h), j.vurgu);
            cit(c, j.vurgu, a.sag(), h.y + h.h / 2, c.px(8));
        }
        let renk = if etkin { j.metin } else { j.soluk };
        c.yazi(&format!("{:02}", k + 1), x.f.yuzde, renk, Dik::yeni(h.x + c.px(14), h.y, nw + c.px(2), h.h), TEK_TAM);
        let tx = h.x + c.px(14) + nw + c.px(10);
        let isaret_w = if tamam { c.px(22) } else { 0 };
        let tw = h.sag() - tx - c.px(10) - isaret_w;
        let et = asama.etiket();
        let font = if etkin { x.f.kalin } else { x.f.govde };
        let er = if etkin || tamam { j.metin } else { j.soluk };
        if c.en(et, font) <= tw {
            c.yazi(et, font, er, Dik::yeni(tx, h.y, tw, h.h), TEK_TAM);
        } else {
            let eh = c.olc(et, font, tw);
            c.yazi(et, font, er, Dik::yeni(tx, h.y + (h.h - eh) / 2, tw, eh), SARILI);
        }
        if tamam {
            isaret(c, x, h.sag() - c.px(18), h.y + h.h / 2, c.px(6), Durum::Tamam);
        }
    }
    // Alt bölmeli ilerleme bandı: 20 hücre (her biri %5), sabit genişlikte yüzde.
    let pr = l.ilerleme;
    c.yatay(0, pr.w, pr.y, 1, j.kenar);
    let ic = pr.ic2(c.px(16), c.px(8));
    let yuzde = i.yuzde();
    let et = "Genel kurulum";
    let lw = c.en(et, x.f.kucuk_kalin);
    c.yazi(et, x.f.kucuk_kalin, j.metin, Dik::yeni(ic.x, ic.y, lw + c.px(2), c.px(20)), TEK_TAM);
    let pw = c.en("%100", x.f.yuzde);
    c.yazi(&format!("%{yuzde}"), x.f.yuzde, j.metin, Dik::yeni(ic.sag() - pw, ic.y - c.px(2), pw, c.px(24)), TEK_TAM | DT_RIGHT);
    let x1 = ic.x + lw + c.px(16);
    let x2 = ic.sag() - pw - c.px(16);
    bolmeli_ciz(c, &j, Dik::yeni(x1, ic.y + c.px(3), x2 - x1, c.px(14)), y::dolu_hucre(yuzde));
    c.yazi(i.son_mesaj(), x.f.kucuk, j.soluk, Dik::yeni(ic.x, ic.y + c.px(22), ic.w, c.px(18)), TEK);
}

// ------------------------------------------------------------------ Mezura

fn mezura_ust(c: &Cizer, d: &PencereDurumu, x: &Ctx, p: &Paylasim, l: &Yerlesim) {
    let j = x.j;
    let i = &p.ilerleme;
    // Büyük görev başlığı (en küçük pencerede 26 px); sığmazsa bölüm boyunda iki satır.
    let gb = y::gorev_basligi(i.asama, p.secim.as_ref().map(|s| s.baslik.as_str()), p.hata.is_some(), i.komut_bekleniyor);
    let g = l.gorev;
    let font = if l.dar { x.f.baslik_dar } else { x.f.baslik };
    if c.en(&gb, font) <= g.w {
        c.yazi(&gb, font, j.metin, g, TEK_TAM);
    } else {
        c.yazi(&gb, x.f.bolum, j.metin, g, SARILI);
    }
    // Aşamalar düz metin; etkin aşamanın altında 4 px kalın çizgi (geçişte %50 -> %100 uzar).
    let a = l.asama;
    let simdi = i.asama.sira();
    let ara = c.px(28);
    let mut xx = a.x;
    let oran = d.gecis.map(|g| g.cizgi_orani()).unwrap_or(1.0);
    for (k, asama) in Asama::HEPSI.iter().enumerate() {
        let et = asama.etiket();
        let etkin = k == simdi;
        let font = if etkin { x.f.kalin } else { x.f.govde };
        let ew = c.en(et, font);
        let renk = if k <= simdi { j.metin } else { j.soluk };
        c.yazi(et, font, renk, Dik::yeni(xx, a.y, ew + c.px(2), a.h - c.px(8)), TEK_TAM);
        if etkin {
            let uz = ((ew as f32) * oran) as i32;
            c.doldur(Dik::yeni(xx, a.alt() - c.px(6), uz, c.px(4)), x.gosterge());
        }
        xx += ew + ara;
    }
    // Tam genişlikte cetvel ve büyük yüzde.
    let pr = l.ilerleme;
    let yuzde = i.yuzde();
    let pw = c.en("%100", x.f.yuzde);
    c.yazi(&format!("%{yuzde}"), x.f.yuzde, j.metin, Dik::yeni(pr.x, pr.y + c.px(2), pw + c.px(2), c.px(40)), TEK_TAM);
    let cetvel = Dik::yeni(pr.x + pw + c.px(16), pr.y, pr.w - pw - c.px(16), pr.h);
    cetvel_ciz(c, x.tema, x.koyu, &j, cetvel, yuzde, true, Some(x.f.cetvel), x.dpi);
    // 2 px çizgiler: cetvelin altı, rehber ayırıcısı, şerit ve alt çubuk.
    c.doldur(Dik::yeni(0, l.ilerleme.alt() + c.px(4), l.baslik.w, c.px(2)), j.kenar);
    let sinir = if l.tanitim.bos() { l.alt.y } else { l.tanitim.y };
    c.doldur(Dik::yeni(l.rehber.x - c.px(11), l.ilerleme.alt() + c.px(4), c.px(2), sinir - l.ilerleme.alt() - c.px(4)), j.kenar);
}

// ------------------------------------------------------------------ gövde

fn govde_ciz(c: &Cizer, d: &mut PencereDurumu, x: &Ctx, p: &Paylasim, l: &Yerlesim) {
    let a0 = match x.tema {
        Tema::Ilmek => {
            c.kutu(l.is_alani, c.px(x.g.kart_yaricap), x.j.yuzey, None);
            l.is_alani.ic(c.px(x.g.kart_ic_bosluk))
        }
        Tema::Kalip => l.is_alani.ic(c.px(16)),
        Tema::Mezura => l.is_alani,
    };
    // Kalıp geçişi: yeni panel 8 px sağdan 3 adımda oturur.
    let dx = d.gecis.map(|g| c.px(g.kaydirma())).unwrap_or(0);
    let a = a0.kaydir(dx, 0);
    let _k = c.kirp(l.is_alani);
    if let Some(hata) = &p.hata {
        hata_ciz(c, x, hata, a);
    } else if let Some(s) = &p.secim {
        secim_ciz(c, d, x, s, a, x.tema != Tema::Mezura);
    } else {
        programlar_ciz(c, x, p, a);
    }
}

fn hata_ciz(c: &Cizer, x: &Ctx, hata: &str, a: Dik) {
    let j = x.j;
    let pad = c.px(20);
    let iw = a.w - 2 * pad;
    let hh = c.olc(hata, x.f.govde, iw);
    let bas = "Kurulum tamamlanamadı";
    let bh = c.olc(bas, x.f.bolum, iw - c.px(26));
    let kutu = Dik::yeni(a.x, a.y, a.w, hh + bh + c.px(48));
    let k = if x.tema == Tema::Mezura { c.px(2) } else { 1 };
    c.kutu(kutu, c.px(x.g.kart_yaricap.min(10)), ara_renk(j.yuzey, j.hata, 0.08), Some((j.hata, k)));
    isaret(c, x, a.x + pad + c.px(9), a.y + pad + bh / 2, c.px(9), Durum::Hata);
    c.yazi(bas, x.f.bolum, j.hata, Dik::yeni(a.x + pad + c.px(26), a.y + pad, iw - c.px(26), bh), SARILI);
    c.yazi(hata, x.f.govde, j.metin, Dik::yeni(a.x + pad, a.y + pad + bh + c.px(8), iw, hh), SARILI);
}

fn secim_ciz(c: &Cizer, d: &mut PencereDurumu, x: &Ctx, s: &Secim, a: Dik, baslik: bool) {
    let j = x.j;
    let gen = a.w;
    let mut yy = a.y;
    if baslik {
        let bh = c.olc(&s.baslik, x.f.bolum, gen);
        c.yazi(&s.baslik, x.f.bolum, j.metin, Dik::yeni(a.x, yy, gen, bh), SARILI);
        yy += bh + c.px(8);
    }
    if !s.aciklama.is_empty() {
        let hh = c.olc(&s.aciklama, x.f.govde, gen);
        c.yazi(&s.aciklama, x.f.govde, j.soluk, Dik::yeni(a.x, yy, gen, hh), SARILI);
        yy += hh + c.px(14);
    }
    let n = s.secenekler.len();
    if n == 0 {
        return;
    }
    // İlmek: isteğe bağlı program grupları hap biçimli çiplerle seçilir (ILHAM-TURU-CHROME).
    let gruplar = s.gruplar();
    if x.tema == Tema::Ilmek && s.coklu && !gruplar.is_empty() {
        let ipucu = "Grupları tek dokunuşla seçin:";
        let ih = c.olc(ipucu, x.f.kucuk, gen);
        c.yazi(ipucu, x.f.kucuk, j.soluk, Dik::yeni(a.x, yy, gen, ih), SARILI);
        yy += ih + c.px(6);
        let ch = c.px(30);
        let etiketler: Vec<(String, usize, usize)> = gruplar
            .iter()
            .map(|g| {
                let (on, top) = s.grup_sayisi(g);
                (format!("{g} {on}/{top}"), on, top)
            })
            .collect();
        let genislikler: Vec<i32> =
            etiketler.iter().map(|(e, _, _)| (c.en(e, x.f.kucuk_kalin) + c.px(14 + 8 + 28)).min(gen)).collect();
        let yer = y::satira_dagit(&genislikler, gen, c.px(8));
        let mut satirlar = 0;
        for (gi, ((et, on, top), (satir, cx))) in etiketler.iter().zip(yer).enumerate() {
            satirlar = satirlar.max(satir + 1);
            let r = Dik::yeni(a.x + cx, yy + satir as i32 * (ch + c.px(8)), genislikler[gi], ch);
            let ust = d.uzerinde == Some(Hedef::Grup(gi));
            let rad = c.px(x.g.cip_yaricap);
            let ix = r.x + c.px(14) + c.px(7);
            let cy = r.y + ch / 2;
            let yazi_r = if on == top {
                c.kutu(r, rad, if ust { ara_renk(j.vurgu, j.metin, 0.15) } else { j.vurgu }, None);
                c.tik(ix, cy, c.px(7), j.vurgu_ustu);
                j.vurgu_ustu
            } else if *on > 0 {
                c.kutu(r, rad, ara_renk(j.yuzey, j.vurgu, if ust { 0.22 } else { 0.14 }), Some((j.vurgu, 1)));
                c.yatay(ix - c.px(5), ix + c.px(5), cy, c.px(2), j.vurgu);
                j.metin
            } else {
                c.kutu(r, rad, if ust { ara_renk(j.yuzey2, j.kenar, 0.15) } else { j.yuzey2 }, Some((j.kenar, 1)));
                c.halka(ix, cy, c.px(6), j.kenar, c.p1());
                j.metin
            };
            let tx = ix + c.px(7) + c.px(8);
            c.yazi(et, x.f.kucuk_kalin, yazi_r, Dik::yeni(tx, r.y, r.sag() - tx - c.px(10), ch), TEK);
            d.hedefler.push((r, Hedef::Grup(gi)));
        }
        yy += satirlar as i32 * (ch + c.px(8)) + c.px(6);
    }
    // Çok seçenekte (ör. 15 şube) ızgara; sütun sayısı etiketler sığacak kadar düşürülür (etiket kesilmez).
    let gruplu = s.secenekler.iter().any(|x| !x.grup.is_empty());
    let ara = c.px(10);
    let gosterge_w = c.px(22 + 8 + 14);
    let mut sut: usize = if n > 6 && !gruplu { if n > 12 { 3 } else { 2 } } else { 1 };
    let en_genis = s.secenekler.iter().map(|o| c.en(&o.etiket, x.f.govde)).max().unwrap_or(0);
    while sut > 1 && en_genis > (gen - ara * (sut as i32 - 1)) / sut as i32 - gosterge_w - c.px(12) {
        sut -= 1;
    }
    let kw = (gen - ara * (sut as i32 - 1)) / sut as i32;
    let tw = kw - gosterge_w - c.px(12);
    let aciklamali = sut == 1 && s.secenekler.iter().any(|x| !x.aciklama.is_empty());
    let font_et = if aciklamali { x.f.kalin } else { x.f.govde };
    let mut kh = c.px(if sut > 1 { x.g.satir - 6 } else { x.g.satir });
    for o in &s.secenekler {
        let mut h = c.olc(&o.etiket, font_et, tw) + c.px(16);
        if aciklamali {
            h += c.olc(&o.aciklama, x.f.kucuk, tw) + c.px(4);
        }
        kh = kh.max(h);
    }
    let grup_h = c.px(26);
    let satirlar = n.div_ceil(sut);
    let sigan = (((a.alt() - yy) + ara) / (kh + ara)).max(1) as usize;
    let imlec_satir = s.imlec / sut;
    let bas = if satirlar <= sigan { 0 } else { imlec_satir.saturating_sub(sigan.saturating_sub(1)).min(satirlar - sigan) };
    let mut onceki_grup = "";
    let rad = c.px(x.g.dugme_yaricap);
    for (k, o) in s.secenekler.iter().enumerate() {
        let satir = k / sut;
        if satir < bas {
            onceki_grup = &o.grup;
            continue;
        }
        if sut == 1 && o.grup != onceki_grup && !o.grup.is_empty() {
            if yy + grup_h > a.alt() {
                break;
            }
            c.yazi(&o.grup, x.f.kucuk_kalin, j.soluk, Dik::yeni(a.x, yy, gen, grup_h), TEK);
            yy += grup_h;
        }
        onceki_grup = &o.grup;
        let (kx, ky) = (a.x + (k % sut) as i32 * (kw + ara), yy);
        if ky + kh > a.alt() {
            c.yazi("…", x.f.govde, j.soluk, Dik::yeni(a.x, ky, gen, c.px(20)), TEK);
            break;
        }
        let rr = Dik::yeni(kx, ky, kw, kh);
        let secili = k == s.imlec;
        let ust = d.uzerinde == Some(Hedef::Secenek(k));
        match x.tema {
            Tema::Mezura => {
                let dolgu = if secili || ust { j.yuzey2 } else { j.yuzey };
                c.kutu(rr, 0, dolgu, Some((if secili { j.metin } else { j.kenar }, c.px(2))));
                if secili {
                    // Seçili satır: solda 6 px sarı-siyah şerit (yalnız renge dayanmaz: kalın çerçeve de var).
                    let serit = Dik::yeni(rr.x, rr.y, c.px(6), rr.h);
                    c.doldur(serit, j.vurgu);
                    c.cerceve(serit, c.px(2), j.kenar);
                }
            }
            _ => {
                let dolgu = if secili {
                    ara_renk(j.yuzey, j.vurgu, 0.10)
                } else if ust {
                    ara_renk(j.yuzey, j.kenar, 0.12)
                } else {
                    j.yuzey
                };
                let kenar = if secili { (x.gosterge(), c.px(2)) } else { (j.kenar, 1) };
                c.kutu(rr, rad, dolgu, Some(kenar));
            }
        }
        // Gösterge: tek seçimde daire, çoklu seçimde kare.
        let gx = kx + c.px(22);
        let gy = ky + kh / 2;
        let gr = c.px(8);
        if s.coklu {
            let isaretli = s.isaretli.get(k).copied().unwrap_or(false);
            let kr = Dik::yeni(gx - gr, gy - gr, 2 * gr, 2 * gr);
            let krad = c.px(x.g.etiket_yaricap.min(3));
            if isaretli {
                vurgu_dolgu(c, x.tema, x.koyu, &j, kr, krad);
                c.tik(gx, gy, gr, j.vurgu_ustu);
            } else {
                c.kutu(kr, krad, j.yuzey, Some((j.kenar, if x.tema == Tema::Mezura { c.px(2) } else { 1 })));
            }
        } else if secili {
            c.halka(gx, gy, gr, x.gosterge(), c.px(2));
            c.daire(gx, gy, gr / 2, x.gosterge(), x.gosterge(), 1);
        } else {
            c.daire(gx, gy, gr, j.yuzey, j.kenar, if x.tema == Tema::Mezura { c.px(2) } else { 1 });
        }
        let tx = gx + gr + c.px(14);
        if aciklamali && !o.aciklama.is_empty() {
            let eh = c.olc(&o.etiket, font_et, tw);
            let ah = c.olc(&o.aciklama, x.f.kucuk, tw);
            let ty = ky + (kh - eh - ah - c.px(4)) / 2;
            c.yazi(&o.etiket, font_et, j.metin, Dik::yeni(tx, ty, tw, eh), SARILI);
            c.yazi(&o.aciklama, x.f.kucuk, j.soluk, Dik::yeni(tx, ty + eh + c.px(4), tw, ah), SARILI);
        } else {
            let eh = c.olc(&o.etiket, font_et, tw);
            c.yazi(&o.etiket, font_et, j.metin, Dik::yeni(tx, ky + (kh - eh) / 2, tw, eh), SARILI);
        }
        d.hedefler.push((rr, Hedef::Secenek(k)));
        if k % sut == sut - 1 || k + 1 == n {
            yy += kh + ara;
        }
    }
}

fn program_ek(pd: &ProgramDurumu) -> String {
    let mut v = vec![];
    if pd.bagimlilik {
        v.push("bağımlılık".to_string());
    }
    if pd.deneme > 1 {
        v.push(format!("{}. deneme", pd.deneme));
    }
    v.join(" · ")
}

fn programlar_ciz(c: &Cizer, x: &Ctx, p: &Paylasim, a: Dik) {
    let j = x.j;
    let i = &p.ilerleme;
    let gen = a.w;
    let mut yy = a.y;
    if i.asama == Asama::Bitti {
        // Bitti özeti: hata varsa başarı gibi davranmaz.
        let hatali = !i.hatalilar().is_empty();
        let oz = y::bitti_ozeti(i);
        let r = c.px(9);
        let oh = c.olc(&oz, x.f.bolum, gen - 2 * r - c.px(10)).max(2 * r);
        isaret(c, x, a.x + r, yy + oh / 2, r, if hatali { Durum::Hata } else { Durum::Tamam });
        c.yazi(&oz, x.f.bolum, if hatali { j.hata } else { j.basari }, Dik::yeni(a.x + 2 * r + c.px(10), yy, gen - 2 * r - c.px(10), oh), SARILI);
        yy += oh + c.px(14);
    }
    if i.programlar.is_empty() {
        let m = match i.asama {
            Asama::Hazirlik => "Kurulum başlamak üzere.".to_string(),
            Asama::Cekirdek => format!("TAA Kur dosyaları yazılıyor ({}/{}).", i.cekirdek_yazilan, i.cekirdek_toplam),
            Asama::Bitti => "Program kurulmadı. Programları sonra TAA Kur penceresinden kurabilirsiniz.".into(),
            _ => "Seçilen programlar burada adım adım görünür: İndir, Doğrula, Kur, Ayarla.".into(),
        };
        let hh = c.olc(&m, x.f.govde, gen);
        c.yazi(&m, x.f.govde, j.soluk, Dik::yeni(a.x, yy, gen, hh), SARILI);
        return;
    }
    // Adım genişliği: en uzun adım etiketi + işaret (etiket kesilmez).
    let adim_en = Adim::HEPSI.iter().map(|ad| c.en(ad.etiket(), x.f.kucuk)).max().unwrap_or(0);
    let adim_w = adim_en + c.px(14 + 6 + 14);
    let durum_w = c.en("Bekliyor", x.f.kucuk) + c.px(26);
    let iki_satir = x.tema != Tema::Ilmek || gen - 4 * adim_w < c.px(150) + durum_w;
    let kalip_2x2 = x.tema == Tema::Kalip && (gen - 3 * c.px(6)) / 4 < adim_w + c.px(8);
    let hata_yuk = |h: &str| -> i32 { c.olc(h, x.f.kucuk, gen).max(c.px(20)) };
    let satir_h = |pd: &ProgramDurumu| -> i32 {
        // Hata iletisi kesilmez: genişlikte sarılarak ölçülür, satır gerektiği kadar uzar.
        let hata = pd.hata.as_deref().map_or(0, |h| hata_yuk(h) + c.px(2));
        match x.tema {
            Tema::Ilmek => c.px(x.g.satir) + if iki_satir { c.px(24) } else { 0 } + hata,
            Tema::Kalip => c.px(28) + if kalip_2x2 { 2 * c.px(36) } else { c.px(36) } + c.px(8) + hata,
            Tema::Mezura => c.px(28) + c.px(30) + c.px(10) + hata,
        }
    };
    // Süren program görünür kalsın (yoksa ilk hatalı).
    let odak = i
        .programlar
        .iter()
        .position(|x| x.durum() == Durum::Suruyor)
        .or_else(|| i.programlar.iter().position(|x| x.durum() == Durum::Hata))
        .unwrap_or(0);
    let mut bas = 0;
    loop {
        let toplam: i32 = i.programlar[bas..=odak.max(bas)].iter().map(satir_h).sum();
        if yy + toplam <= a.alt() || bas >= odak {
            break;
        }
        bas += 1;
    }
    for (k, pd) in i.programlar.iter().enumerate().skip(bas) {
        let sh = satir_h(pd);
        if yy + sh > a.alt() {
            let kalan = i.programlar.len() - k;
            c.yazi(&format!("+{kalan} program daha"), x.f.kucuk, j.soluk, Dik::yeni(a.x, yy, gen, c.px(20)), TEK);
            break;
        }
        let dur = pd.durum();
        let ek = program_ek(pd);
        match x.tema {
            Tema::Ilmek => {
                let ust_h = c.px(x.g.satir);
                let cy = yy + ust_h / 2;
                let dw = c.en(y::durum_metni(dur), x.f.kucuk) + c.px(26);
                let adimlar_w = if iki_satir { 0 } else { 4 * adim_w };
                let ad_w = (gen - adimlar_w - dw - c.px(16)).max(c.px(60));
                // Paket adı "…" ile kısalabilir (KARAR §1).
                let aw = c.en(&pd.ad, x.f.kalin).min(ad_w);
                c.yazi(&pd.ad, x.f.kalin, j.metin, Dik::yeni(a.x, cy - c.px(12), aw, c.px(24)), TEK);
                let dx = a.x + aw + c.px(12);
                let w2 = durum_etiketi(c, x, dx, cy, dur, x.f.kucuk);
                if !ek.is_empty() {
                    let ex = dx + w2 + c.px(10);
                    c.yazi(&ek, x.f.kucuk, j.soluk, Dik::yeni(ex, cy - c.px(10), (a.x + gen - adimlar_w - ex).max(0), c.px(20)), TEK);
                }
                let (ax0, ay) = if iki_satir { (a.x, yy + ust_h - c.px(6)) } else { (a.x + gen - adimlar_w, cy) };
                for (n, adim) in Adim::HEPSI.iter().enumerate() {
                    let ax = ax0 + n as i32 * adim_w;
                    isaret(c, x, ax + c.px(7), ay, c.px(6), pd.adimlar[n]);
                    let renk = if pd.adimlar[n] == Durum::Bekliyor { j.soluk } else { durum_rengi(&j, pd.adimlar[n]) };
                    c.yazi(adim.etiket(), x.f.kucuk, renk, Dik::yeni(ax + c.px(20), ay - c.px(10), adim_w - c.px(20), c.px(20)), TEK_TAM);
                }
            }
            Tema::Kalip => {
                let no = format!("{:02}", k + 1);
                let nw = c.en(&no, x.f.kalin);
                c.yazi(&no, x.f.kalin, j.soluk, Dik::yeni(a.x, yy, nw + c.px(2), c.px(26)), TEK_TAM);
                let dw = c.en(y::durum_metni(dur), x.f.kucuk) + c.px(26);
                let ax = a.x + nw + c.px(10);
                let ad_w = (gen - (ax - a.x) - dw - c.px(12)).max(c.px(40));
                let aw = c.en(&pd.ad, x.f.kalin).min(ad_w);
                c.yazi(&pd.ad, x.f.kalin, j.metin, Dik::yeni(ax, yy, aw, c.px(26)), TEK);
                if !ek.is_empty() {
                    let ex = ax + aw + c.px(8);
                    c.yazi(&ek, x.f.kucuk, j.soluk, Dik::yeni(ex, yy, (a.sag() - dw - c.px(8) - ex).max(0), c.px(26)), TEK);
                }
                durum_etiketi(c, x, a.sag() - dw, yy + c.px(13), dur, x.f.kucuk);
                let ara = c.px(6);
                let (sut, hw) = if kalip_2x2 { (2, (gen - ara) / 2) } else { (4, (gen - 3 * ara) / 4) };
                for (n, adim) in Adim::HEPSI.iter().enumerate() {
                    let hx = a.x + (n % sut) as i32 * (hw + ara);
                    let hy = yy + c.px(28) + (n / sut) as i32 * c.px(36);
                    let h = Dik::yeni(hx, hy, hw, c.px(30));
                    let ad = pd.adimlar[n];
                    let (dolgu, kenar) = match ad {
                        Durum::Suruyor => (ara_renk(j.yuzey2, j.vurgu, 0.12), j.vurgu),
                        Durum::Hata => (j.yuzey, j.hata),
                        Durum::Tamam => (j.yuzey, j.kenar),
                        Durum::Bekliyor => (j.yuzey2, j.kenar),
                    };
                    c.kutu(h, c.px(x.g.etiket_yaricap), dolgu, Some((kenar, 1)));
                    isaret(c, x, h.x + c.px(14), h.y + h.h / 2, c.px(6), ad);
                    c.yazi(adim.etiket(), x.f.kucuk, if ad == Durum::Bekliyor { j.soluk } else { j.metin },
                        Dik::yeni(h.x + c.px(26), h.y, h.w - c.px(30), h.h), TEK_TAM);
                    if ad == Durum::Suruyor {
                        // Etkin işlemde üç küçük hücre sırayla yanar (360 ms); azaltta sabit vurgu.
                        let yanan = y::kalip_hucre_dongusu(x.tik, x.azalt);
                        for q in 0..3 {
                            let kx = h.sag() - c.px(8) - (3 - q) * c.px(7);
                            let renk = match yanan {
                                Some(z) if z == q as usize => j.vurgu,
                                Some(_) => j.kenar,
                                None => j.vurgu,
                            };
                            c.doldur(Dik::yeni(kx, h.y + h.h / 2 - c.px(2), c.px(5), c.px(5)), renk);
                        }
                    }
                }
            }
            Tema::Mezura => {
                let bas = format!("{} — {}", pd.ad, y::durum_metni(dur));
                c.yazi(&bas, x.f.kalin, if dur == Durum::Hata { j.hata } else { j.metin }, Dik::yeni(a.x, yy, gen, c.px(26)), TEK);
                let mut sx = a.x;
                let sy = yy + c.px(28) + c.px(15);
                for (n, adim) in Adim::HEPSI.iter().enumerate() {
                    let r = c.px(7);
                    isaret(c, x, sx + r, sy, r, pd.adimlar[n]);
                    let et = adim.etiket();
                    let ew = c.en(et, x.f.govde);
                    if sx + 2 * r + c.px(6) + ew > a.sag() {
                        break;
                    }
                    c.yazi(et, x.f.govde, if pd.adimlar[n] == Durum::Bekliyor { j.soluk } else { j.metin },
                        Dik::yeni(sx + 2 * r + c.px(6), sy - c.px(12), ew + c.px(2), c.px(24)), TEK_TAM);
                    sx += 2 * r + c.px(6) + ew + c.px(22);
                }
                if !ek.is_empty() {
                    c.yazi(&ek, x.f.kucuk, j.soluk, Dik::yeni(sx, sy - c.px(10), (a.sag() - sx).max(0), c.px(20)), TEK);
                }
            }
        }
        if let Some(h) = &pd.hata {
            let hh = hata_yuk(h);
            c.yazi(h, x.f.kucuk, j.hata, Dik::yeni(a.x, yy + sh - hh - c.px(6), gen, hh), SARILI);
        }
        yy += sh;
        if k + 1 < i.programlar.len() {
            match x.tema {
                // İlmek: liste satır ayırıcıları 1 px teyel.
                Tema::Ilmek => c.teyel(a.x, a.sag(), yy - c.px(4), 1, j.kenar),
                _ => c.yatay(a.x, a.sag(), yy - c.px(4), 1, j.kenar),
            }
        }
    }
}

// ------------------------------------------------------------------ rehber

fn rehber_ciz(c: &Cizer, x: &Ctx, v: &[Direktif], l: &Yerlesim) {
    let j = x.j;
    let a = match x.tema {
        Tema::Ilmek => {
            // Sağ sütun tek yüzey: rehber üstte, tanıtım kartı altta; araları teyel.
            let alt = if l.tanitim.bos() { l.rehber.alt() } else { l.tanitim.alt() };
            c.kutu(Dik::yeni(l.rehber.x, l.rehber.y, l.rehber.w, alt - l.rehber.y), c.px(x.g.kart_yaricap), j.yuzey, None);
            if !l.tanitim.bos() {
                c.teyel(l.rehber.x + c.px(16), l.rehber.sag() - c.px(16), l.tanitim.y - c.px(6), 1, j.kenar);
            }
            l.rehber.ic(c.px(16))
        }
        Tema::Kalip => {
            c.doldur(Dik::yeni(l.rehber.x + 1, l.rehber.y, l.rehber.w - 1, l.rehber.h), j.yuzey);
            l.rehber.ic(c.px(12))
        }
        Tema::Mezura => l.rehber,
    };
    if a.h < c.px(60) {
        return;
    }
    let gen = a.w;
    let bh = c.olc("Adım adım", x.f.bolum, gen);
    c.yazi("Adım adım", x.f.bolum, j.metin, Dik::yeni(a.x, a.y, gen, bh), SARILI);
    let mut yy = a.y + bh + c.px(2);
    let alt_b = "Bu PC'de yapılacak ayarlar; yapılanlar işaretli.";
    let ah = c.olc(alt_b, x.f.kucuk, gen);
    c.yazi(alt_b, x.f.kucuk, j.soluk, Dik::yeni(a.x, yy, gen, ah), SARILI);
    yy += ah + c.px(12);
    let rr = c.px(10);
    let tx = a.x + 2 * rr + c.px(10);
    let tw = a.sag() - tx;
    for (n, it) in v.iter().enumerate() {
        let th = c.olc(&it.baslik, x.f.kalin, tw);
        if yy + th > a.alt() {
            c.yazi("…", x.f.kucuk, j.soluk, Dik::yeni(tx, yy, tw, c.px(16)), TEK);
            break;
        }
        let cy = yy + c.px(10);
        let cx = a.x + rr;
        let kutu = Dik::yeni(cx - rr, cy - rr, 2 * rr + 1, 2 * rr + 1);
        let kare = x.tema != Tema::Ilmek;
        let kk = if x.tema == Tema::Mezura { c.px(2) } else { c.px(2).min(2) };
        match it.durum {
            DDurum::Yapildi => isaret(c, x, cx, cy, rr, Durum::Tamam),
            DDurum::Yapilacak => {
                if kare {
                    c.kutu(kutu, c.px(x.g.etiket_yaricap), j.yuzey, Some((x.gosterge(), kk)));
                } else {
                    c.daire(cx, cy, rr, j.yuzey, x.gosterge(), kk);
                }
                c.yazi(&(n + 1).to_string(), x.f.kucuk_kalin, j.metin, kutu, TEK_TAM | DT_CENTER);
            }
            DDurum::Bilgi => {
                if kare {
                    c.kutu(kutu, c.px(x.g.etiket_yaricap), j.yuzey, Some((j.kenar, kk)));
                } else {
                    c.daire(cx, cy, rr, j.yuzey, j.kenar, kk);
                }
                c.yazi("i", x.f.kucuk_kalin, j.soluk, kutu, TEK_TAM | DT_CENTER);
            }
        }
        // Tamamlanan madde alfa ile soldurulmaz: soluk jetonu + tik.
        let bas_r = if it.durum == DDurum::Yapildi { j.soluk } else { j.metin };
        c.yazi(&it.baslik, x.f.kalin, bas_r, Dik::yeni(tx, yy, tw, th), SARILI | DT_LEFT);
        let ah = c.olc(&it.ayrinti, x.f.kucuk, tw);
        let kalan = a.alt() - (yy + th);
        c.yazi(&it.ayrinti, x.f.kucuk, j.soluk, Dik::yeni(tx, yy + th, tw, ah.min(kalan.max(0))), SARILI | DT_END_ELLIPSIS);
        yy += th + ah + c.px(12);
    }
}

// ------------------------------------------------------------------ tanıtım (bekleme) kartı

struct KartOlcu {
    ad_h: i32,
    ozet_h: i32,
    not_h: i32,
    sab_h: i32,
    iw: i32,
    /// Kalıp iki sütun: sağ sütunun x payı (0 = tek sütun).
    sag_x: i32,
    toplam: i32,
}

fn kart_olc(c: &Cizer, x: &Ctx, k: &Kart, gen: i32) -> KartOlcu {
    let f = &x.f;
    match x.tema {
        Tema::Ilmek => {
            let iw = gen - 2 * c.px(16);
            let ad_h = c.olc(&k.ad, f.bolum, iw);
            let ozet_h = c.olc(&k.ozet, f.govde, iw);
            let not_h = c.olc(&k.not, f.kucuk, iw);
            let sab_h = c.olc(&k.sablon, f.kucuk, iw - c.px(20));
            let mut t = c.px(12) + c.px(28) + ad_h + c.px(4) + ozet_h + c.px(12);
            if not_h > 0 {
                t += c.px(10 + 18) + not_h;
            }
            if sab_h > 0 {
                t += c.px(10 + 18) + sab_h + c.px(16);
            }
            KartOlcu { ad_h, ozet_h, not_h, sab_h, iw, sag_x: 0, toplam: t }
        }
        Tema::Kalip => {
            let ic = gen - 2 * c.px(16);
            let iki = ic >= c.px(420);
            let (iw, sw, sag_x) = if iki {
                let lw = ic * 55 / 100;
                (lw, ic - lw - c.px(16), lw + c.px(16))
            } else {
                (ic, ic, 0)
            };
            let ad_h = c.olc(&k.ad, f.bolum, iw);
            let ozet_h = c.olc(&k.ozet, f.govde, iw);
            let not_h = c.olc(&k.not, f.kucuk, sw);
            let sab_h = c.olc(&k.sablon, f.kucuk, sw - c.px(16));
            let sol = ad_h + c.px(4) + ozet_h;
            let mut sag = 0;
            if not_h > 0 {
                sag += c.px(18) + not_h + c.px(8);
            }
            if sab_h > 0 {
                sag += c.px(18) + sab_h + c.px(12);
            }
            let govde = if iki { sol.max(sag) } else { sol + c.px(10) + sag };
            KartOlcu { ad_h, ozet_h, not_h, sab_h, iw, sag_x, toplam: c.px(12) + c.px(28) + govde + c.px(12) }
        }
        Tema::Mezura => {
            let iw = gen - 2 * c.px(24) - mezura_kart_denetim_w(c, x) - c.px(16);
            let bas = format!("Program tanıtımı — {}", k.ad);
            let ad_h = c.olc(&bas, f.kalin, iw);
            let ozet_h = c.olc(&k.ozet, f.govde, iw);
            let not_h = c.olc(&mezura_not(k), f.kucuk, iw);
            KartOlcu { ad_h, ozet_h, not_h, sab_h: 0, iw, sag_x: 0, toplam: c.px(12) + ad_h + c.px(4) + ozet_h + c.px(4) + not_h + c.px(12) }
        }
    }
}

fn mezura_not(k: &Kart) -> String {
    match (k.not.is_empty(), k.sablon.is_empty()) {
        (false, false) => format!("Not: {}  ·  Örnek şablon: {}", k.not, k.sablon),
        (false, true) => format!("Not: {}", k.not),
        (true, false) => format!("Örnek şablon: {}", k.sablon),
        (true, true) => String::new(),
    }
}

fn mezura_kart_denetim_w(c: &Cizer, x: &Ctx) -> i32 {
    2 * c.px(40) + c.px(8) + dugme_gen(c, x, "Duraklat").max(dugme_gen(c, x, "Sürdür")) + c.px(8)
}

fn kart_ok(c: &Cizer, d: &mut PencereDurumu, x: &Ctx, r: Dik, isaret: &str, hedef: Hedef) {
    let j = x.j;
    let ust = d.uzerinde == Some(hedef);
    if x.tema == Tema::Mezura {
        c.kutu(r, 0, if ust { j.yuzey2 } else { j.yuzey }, Some((j.kenar, c.px(2))));
    } else if ust {
        c.kutu(r, c.px(x.g.etiket_yaricap), j.yuzey2, None);
    }
    c.yazi(isaret, x.f.bolum, j.metin, r, TEK_TAM | DT_CENTER);
    d.hedefler.push((r, hedef));
}

fn kart_ciz(c: &Cizer, d: &mut PencereDurumu, x: &Ctx, l: &Yerlesim) {
    let j = x.j;
    let alan = l.tanitim;
    if alan.bos() {
        return;
    }
    let n = d.kartlar.len();
    let Some(k) = d.kartlar.get(d.kart % n.max(1)).cloned() else { return };
    d.kart_alani = alan;
    let kk = y::kart_karesi(x.tema, d.kart_tik, x.azalt);
    let dx = c.px(kk.kaydirma);
    let o = kart_olc(c, x, &k, alan.w);
    let etiket = kartlar::tur_etiketi(&k.tur);
    let sayac = format!("{} / {n}", d.kart % n + 1);
    match x.tema {
        Tema::Ilmek | Tema::Kalip => {
            if x.tema == Tema::Kalip {
                c.doldur(alan, j.yuzey);
                c.yatay(alan.x, alan.sag(), alan.y, 1, j.kenar);
            }
            let a = alan.ic2(c.px(16), c.px(12));
            if kk.dolgu > 0.0 {
                c.kutu(alan.ic(c.px(4)), c.px(x.g.etiket_yaricap), ara_renk(j.yuzey, j.yuzey2, kk.dolgu), None);
            }
            // Üst satır: tür, sayaç, ‹ ›.
            let ok = c.px(28);
            let geri = Dik::yeni(a.sag() - 2 * ok, a.y, ok, ok);
            let ileri = Dik::yeni(a.sag() - ok, a.y, ok, ok);
            kart_ok(c, d, x, geri, "‹", Hedef::KartGeri);
            kart_ok(c, d, x, ileri, "›", Hedef::KartIleri);
            let sw = c.en(&sayac, x.f.kucuk);
            c.yazi(&sayac, x.f.kucuk, j.soluk, Dik::yeni(geri.x - sw - c.px(8), a.y, sw + c.px(2), ok), TEK_TAM);
            let ew = c.en(etiket, x.f.kucuk_kalin);
            c.yazi(etiket, x.f.kucuk_kalin, j.soluk, Dik::yeni(a.x, a.y, ew.min(geri.x - sw - c.px(16) - a.x).max(0), ok), TEK);
            if !kk.metin {
                return;
            }
            let _k = c.kirp(alan);
            let x0 = a.x + dx;
            let mut yy = a.y + ok;
            let alt = a.alt();
            c.yazi(&k.ad, x.f.bolum, j.metin, Dik::yeni(x0, yy, o.iw, o.ad_h), SARILI);
            yy += o.ad_h + c.px(4);
            c.yazi(&k.ozet, x.f.govde, j.metin, Dik::yeni(x0, yy, o.iw, o.ozet_h.min(alt - yy)), SARILI);
            yy += o.ozet_h;
            let (sx, sw2, mut sy) = if o.sag_x > 0 {
                (x0 + o.sag_x, a.w - o.sag_x, a.y + ok)
            } else {
                (x0, o.iw, yy + c.px(10))
            };
            if o.not_h > 0 && sy + c.px(18) + o.not_h <= alt {
                c.yazi("Not", x.f.kucuk_kalin, j.soluk, Dik::yeni(sx, sy, sw2, c.px(18)), TEK);
                sy += c.px(18);
                c.yazi(&k.not, x.f.kucuk, j.soluk, Dik::yeni(sx, sy, sw2, o.not_h), SARILI);
                sy += o.not_h + c.px(10);
            }
            if o.sab_h > 0 && sy + c.px(18) + o.sab_h + c.px(16) <= alt {
                c.yazi("Örnek şablon", x.f.kucuk_kalin, j.soluk, Dik::yeni(sx, sy, sw2, c.px(18)), TEK);
                sy += c.px(18);
                let kutu = Dik::yeni(sx, sy, sw2, o.sab_h + c.px(16));
                c.kutu(kutu, c.px(x.g.etiket_yaricap), j.yuzey2, None);
                c.yazi(&k.sablon, x.f.kucuk, j.metin, kutu.ic2(c.px(10), c.px(8)), SARILI);
            }
        }
        Tema::Mezura => {
            // Alt bilgi şeridi: iki 2 px yatay çizgi arası; ‹ › ve duraklat hep görünür; değişim anlık.
            c.doldur(Dik::yeni(alan.x, alan.y, alan.w, c.px(2)), j.kenar);
            let a = alan.ic2(c.px(24), c.px(12));
            let ow = c.px(40);
            let dw = dugme_gen(c, x, "Duraklat").max(dugme_gen(c, x, "Sürdür"));
            let bh = c.px(40);
            let ileri = Dik::yeni(a.sag() - ow, a.y, ow, bh);
            let geri = Dik::yeni(ileri.x - c.px(8) - ow, a.y, ow, bh);
            let dur = Dik::yeni(geri.x - c.px(8) - dw, a.y, dw, bh);
            let duraklat = d.kart_duraklat;
            let et = if duraklat { "Sürdür" } else { "Duraklat" };
            dugme_ciz(c, d, x, dur, et, false, Hedef::KartDuraklat, duraklat);
            kart_ok(c, d, x, geri, "‹", Hedef::KartGeri);
            kart_ok(c, d, x, ileri, "›", Hedef::KartIleri);
            let sw = c.en(&sayac, x.f.kucuk);
            c.yazi(&sayac, x.f.kucuk, j.soluk, Dik::yeni(ileri.sag() - sw, a.y + bh + c.px(4), sw + c.px(2), c.px(18)), TEK_TAM);
            let _k = c.kirp(alan);
            let mut yy = a.y;
            let alt = alan.alt() - c.px(4);
            let bas = format!("Program tanıtımı — {}", k.ad);
            c.yazi(&bas, x.f.kalin, j.metin, Dik::yeni(a.x, yy, o.iw, o.ad_h), SARILI);
            yy += o.ad_h + c.px(4);
            c.yazi(&k.ozet, x.f.govde, j.metin, Dik::yeni(a.x, yy, o.iw, o.ozet_h.min(alt - yy)), SARILI);
            yy += o.ozet_h + c.px(4);
            if o.not_h > 0 && yy + o.not_h <= alt {
                c.yazi(&mezura_not(&k), x.f.kucuk, j.soluk, Dik::yeni(a.x, yy, o.iw, o.not_h), SARILI);
            }
        }
    }
}

// ------------------------------------------------------------------ alt çubuk

fn alt_ciz(c: &Cizer, d: &mut PencereDurumu, x: &Ctx, p: &Paylasim, a: Dik) {
    let j = x.j;
    let i = &p.ilerleme;
    match x.tema {
        Tema::Ilmek => {}
        Tema::Kalip => c.yatay(0, a.w, a.y, 1, j.kenar),
        Tema::Mezura => c.doldur(Dik::yeni(0, a.y, a.w, c.px(2)), j.kenar),
    }
    let (ipucu, dugmeler): (&str, Vec<(&str, Hedef, bool)>) = if p.hata.is_some() {
        ("Enter: Bitir", vec![("Bitir", Hedef::Bitir, true)])
    } else if let Some(s) = &p.secim {
        let ip = if s.coklu {
            "Yukarı/Aşağı ok: gez · Boşluk: işaretle · Enter: onayla · Esc: vazgeç"
        } else if s.secenekler.is_empty() {
            "Enter: onayla · Esc: vazgeç"
        } else {
            "Yukarı/Aşağı ok: seç · Enter: onayla · Esc: vazgeç"
        };
        (ip, vec![(s.vazgec.as_str(), Hedef::Vazgec, false), (s.onay.as_str(), Hedef::Onay, true)])
    } else if i.komut_bekleniyor && !i.hatalilar().is_empty() {
        ("Enter: Yeniden dene · Esc: Bitir", vec![("Bitir", Hedef::Bitir, false), ("Yeniden dene", Hedef::YenidenDene, true)])
    } else if i.komut_bekleniyor {
        ("Enter: Bitir", vec![("TAA Kur'u aç", Hedef::TaaKurAc, false), ("Bitir", Hedef::Bitir, true)])
    } else {
        // Kurulum sürerken yanıltıcı "Devam et" gösterilmez.
        ("Kurulum sürüyor · Sol/Sağ ok: kartlar · Tab: Ayarlar", vec![])
    };
    let m = kenar_payi(c, x);
    let bh = c.px(x.g.dugme);
    let by = a.y + (a.h - bh) / 2;
    let mut bx = a.w - m;
    for (et, hedef, birincil) in dugmeler.iter().rev() {
        let bw = dugme_gen(c, x, et).min(a.w / 2);
        bx -= bw;
        dugme_ciz(c, d, x, Dik::yeni(bx, by, bw, bh), et, *birincil, *hedef, false);
        bx -= c.px(12);
    }
    let iw = (bx - m - c.px(12)).max(0);
    let ih = c.olc(ipucu, x.f.kucuk, iw).min(a.h);
    c.yazi(ipucu, x.f.kucuk, j.soluk, Dik::yeni(m, a.y + (a.h - ih) / 2, iw, ih), SARILI);
}

// ------------------------------------------------------------------ önizleme (Ayarlar)

/// Bir temanın küçük canlı önizlemesi; o temanın kendi imza çizimleriyle (teyel, çıt + bölmeli çubuk, cetvel).
fn onizleme_ciz(c: &Cizer, t: Tema, koyu: bool, j: &Jetonlar, r: Dik, dpi: u32) {
    let _k = c.kirp(r);
    c.doldur(r, j.zemin);
    let s = |v: i32| c.px(v);
    match t {
        Tema::Ilmek => {
            let yy = r.y + s(12);
            let (x1, x3) = (r.x + s(12), r.sag() - s(12));
            let x2 = (x1 + x3) / 2;
            c.yatay(x1, x2, yy, s(2), j.vurgu);
            c.teyel(x2 + s(6), x3, yy, s(2), j.kenar);
            c.daire(x1, yy, s(4), j.vurgu, j.vurgu, 1);
            c.halka(x2, yy, s(4), j.vurgu, s(2));
            c.daire(x3, yy, s(4), j.zemin, j.kenar, 1);
            let kart = Dik::yeni(r.x + s(8), r.y + s(24), r.w - s(16), r.h - s(30));
            c.kutu(kart, s(7), j.yuzey, None);
            c.kutu(Dik::yeni(kart.x + s(8), kart.y + s(8), kart.w / 2, s(4)), s(2), j.metin, None);
            c.kutu(Dik::yeni(kart.x + s(8), kart.y + s(16), kart.w * 2 / 3, s(3)), s(1), j.soluk, None);
            c.kutu(Dik::yeni(kart.sag() - s(30), kart.alt() - s(14), s(22), s(9)), s(4), j.vurgu, None);
        }
        Tema::Kalip => {
            let ray = Dik::yeni(r.x, r.y, s(24), r.h);
            c.doldur(ray, j.yuzey);
            c.dikey(ray.sag(), r.y, r.alt(), 1, j.kenar);
            for q in 0..3 {
                let h = Dik::yeni(ray.x, r.y + s(6) + q * s(14), ray.w, s(12));
                if q == 1 {
                    c.doldur(h, j.yuzey2);
                    c.doldur(Dik::yeni(h.x, h.y, s(2), h.h), j.vurgu);
                    cit(c, j.vurgu, ray.sag(), h.y + h.h / 2, s(4));
                }
                c.doldur(Dik::yeni(h.x + s(6), h.y + s(4), s(10), s(3)), if q == 1 { j.metin } else { j.soluk });
            }
            let ic = Dik::yeni(ray.sag() + s(8), r.y + s(6), r.sag() - ray.sag() - s(14), r.h - s(26));
            c.kutu(ic, s(2), j.yuzey, Some((j.kenar, 1)));
            c.doldur(Dik::yeni(ic.x + s(6), ic.y + s(6), ic.w / 2, s(4)), j.metin);
            bolmeli_ciz(c, j, Dik::yeni(ray.sag() + s(8), r.alt() - s(12), r.sag() - ray.sag() - s(14), s(6)), 12);
        }
        Tema::Mezura => {
            c.doldur(Dik::yeni(r.x + s(8), r.y + s(8), r.w / 2, s(6)), j.metin);
            c.doldur(Dik::yeni(r.x, r.y + s(20), r.w, s(2)), j.kenar);
            let cet = Dik::yeni(r.x + s(6), r.y + s(26), r.w - s(12), r.h - s(28));
            cetvel_ciz(c, t, koyu, j, cet, 60, true, None, dpi);
        }
    }
}

// ------------------------------------------------------------------ Ayarlar paneli

enum AP {
    Yazi(String, HFONT, u32, bool),
    Onizleme(usize, i32),
    Radyo(usize),
    Ornek,
    Dugme(AyarOge, &'static str),
    Kutu,
}

fn ayar_paneli_ciz(c: &Cizer, d: &mut PencereDurumu, x: &Ctx, panel: Dik) {
    let j = x.j;
    match x.tema {
        Tema::Ilmek => {
            c.doldur(panel, j.yuzey);
            c.dikey(panel.x, panel.y, panel.alt(), 1, j.kenar);
        }
        Tema::Kalip => {
            c.doldur(panel, j.yuzey);
            c.dikey(panel.x, panel.y, panel.alt(), 1, j.kenar);
        }
        Tema::Mezura => {
            c.doldur(panel, j.yuzey);
            c.doldur(Dik::yeni(panel.x, panel.y, c.px(2), panel.h), j.kenar);
        }
    }
    d.hedefler.push((panel, Hedef::Bos));
    let pad = c.px(20);
    let iw = panel.w - 2 * pad;
    let x0 = panel.x + pad;
    let bh = c.px(x.g.dugme);
    let mut v: Vec<(AP, Dik)> = vec![];
    let mut yy = 0;
    // Başlık satırı.
    let kw = dugme_gen(c, x, "Kapat");
    v.push((AP::Dugme(AyarOge::Kapat, "Kapat"), Dik::yeni(panel.sag() - pad - kw, yy, kw, bh)));
    v.push((AP::Yazi("Ayarlar".into(), x.f.bolum, j.metin, false), Dik::yeni(x0, yy, iw - kw - c.px(8), bh)));
    yy += bh + c.px(14);
    let bolum = |v: &mut Vec<(AP, Dik)>, yy: &mut i32, et: &str| {
        v.push((AP::Yazi(et.into(), x.f.kucuk_kalin, j.soluk, false), Dik::yeni(x0, *yy, iw, c.px(20))));
        *yy += c.px(24);
    };
    bolum(&mut v, &mut yy, "Tema");
    let ara = c.px(10);
    let pw = (iw - 2 * ara) / 3;
    let ph = (pw * 5 / 8).max(c.px(56));
    for k in 0..3 {
        v.push((AP::Onizleme(k, ph), Dik::yeni(x0 + k as i32 * (pw + ara), yy, pw, ph + c.px(28))));
    }
    yy += ph + c.px(28) + c.px(14);
    bolum(&mut v, &mut yy, "Görünüm modu");
    let rg: Vec<i32> = Mod::HEPSI.iter().map(|m| c.px(20 + 8 + 12) + c.en(m.gorunen_ad(), x.f.govde)).collect();
    let rh = c.px(34);
    let mut satir = 0;
    for (k, (s, rx)) in y::satira_dagit(&rg, iw, c.px(8)).into_iter().enumerate() {
        satir = satir.max(s + 1);
        v.push((AP::Radyo(k), Dik::yeni(x0 + rx, yy + s as i32 * (rh + c.px(6)), rg[k].min(iw), rh)));
    }
    yy += satir as i32 * (rh + c.px(6)) + c.px(8);
    bolum(&mut v, &mut yy, "Vurgu rengi");
    let og = [bh, dugme_gen(c, x, "Renk seç…"), dugme_gen(c, x, "Temanın rengine dön")];
    let og: Vec<i32> = og.iter().map(|w| (*w).min(iw)).collect();
    let mut satir = 0;
    for (k, (s, rx)) in y::satira_dagit(&og, iw, c.px(8)).into_iter().enumerate() {
        satir = satir.max(s + 1);
        let r = Dik::yeni(x0 + rx, yy + s as i32 * (bh + c.px(8)), og[k], bh);
        v.push((
            match k {
                0 => AP::Ornek,
                1 => AP::Dugme(AyarOge::RenkSec, "Renk seç…"),
                _ => AP::Dugme(AyarOge::RengeDon, "Temanın rengine dön"),
            },
            r,
        ));
    }
    yy += satir as i32 * (bh + c.px(8));
    if let Some((not, uyari)) = d.ayar_notu.clone() {
        let nh = c.olc(&not, x.f.kucuk, iw);
        v.push((AP::Yazi(not, x.f.kucuk, if uyari { j.uyari } else { j.soluk }, true), Dik::yeni(x0, yy, iw, nh)));
        yy += nh + c.px(6);
    }
    yy += c.px(8);
    bolum(&mut v, &mut yy, "Hareket");
    v.push((AP::Kutu, Dik::yeni(x0, yy, iw, rh)));
    yy += rh;
    if d.gorunum.hareketi_azalt.is_none() {
        let not = "Windows'un animasyon ayarı izleniyor.";
        let nh = c.olc(not, x.f.kucuk, iw);
        v.push((AP::Yazi(not.into(), x.f.kucuk, j.soluk, true), Dik::yeni(x0, yy, iw, nh)));
        yy += nh;
    }
    yy += c.px(14);
    bolum(&mut v, &mut yy, "Tanıtım");
    let tg: Vec<i32> = [dugme_gen(c, x, "Tanıtımı tekrar göster"), dugme_gen(c, x, "Varsayılana dön")].iter().map(|w| (*w).min(iw)).collect();
    let mut satir = 0;
    for (k, (s, rx)) in y::satira_dagit(&tg, iw, c.px(8)).into_iter().enumerate() {
        satir = satir.max(s + 1);
        let r = Dik::yeni(x0 + rx, yy + s as i32 * (bh + c.px(8)), tg[k], bh);
        v.push((if k == 0 { AP::Dugme(AyarOge::TanitimTekrar, "Tanıtımı tekrar göster") } else { AP::Dugme(AyarOge::Varsayilan, "Varsayılana dön") }, r));
    }
    yy += satir as i32 * (bh + c.px(8));
    let toplam = yy + pad;

    // Kaydırma: odaktaki denetim görünür kalsın.
    let ust = panel.y + c.px(16);
    let gorunur = panel.h - c.px(16);
    if let Some(o) = d.ayar_odak {
        let hedef = v.iter().find(|(p, _)| match p {
            AP::Dugme(a, _) => *a == o,
            AP::Onizleme(k, _) => o == AyarOge::Tema(*k),
            AP::Radyo(k) => o == AyarOge::Mod(*k),
            AP::Kutu => o == AyarOge::HareketAzalt,
            _ => false,
        });
        if let Some((_, r)) = hedef {
            if r.y - c.px(8) < d.ayar_kaydir {
                d.ayar_kaydir = r.y - c.px(8);
            } else if r.alt() + c.px(8) > d.ayar_kaydir + gorunur {
                d.ayar_kaydir = r.alt() + c.px(8) - gorunur;
            }
        }
    }
    d.ayar_kaydir = d.ayar_kaydir.clamp(0, (toplam - gorunur).max(0));
    d.ayar_paneli = panel;
    let dy = ust - d.ayar_kaydir;
    let _k = c.kirp(panel);
    let gorunum = d.gorunum.clone();
    for (parca, r0) in v {
        let r = r0.kaydir(0, dy);
        if r.alt() < panel.y || r.y > panel.alt() {
            continue;
        }
        match parca {
            AP::Yazi(t, font, renk, sarili) => {
                c.yazi(&t, font, renk, r, if sarili { SARILI } else { TEK_TAM });
            }
            AP::Dugme(o, et) => dugme_ciz(c, d, x, r, et, false, Hedef::Ayar(o), false),
            AP::Onizleme(k, ph) => {
                let t = Tema::HEPSI[k];
                let kutu = Dik::yeni(r.x, r.y, r.w, ph);
                let jt = tema_jetonlari(&gorunum, t, x.koyu);
                onizleme_ciz(c, t, x.koyu, &jt, kutu, x.dpi);
                let secili = t == x.tema;
                let ust_ = d.uzerinde == Some(Hedef::Ayar(AyarOge::Tema(k)));
                if secili {
                    c.cerceve(kutu, c.px(2), x.gosterge());
                } else {
                    c.cerceve(kutu, 1, if ust_ { j.metin } else { j.kenar });
                }
                let ad = t.gorunen_ad();
                let font = if secili { x.f.kalin } else { x.f.govde };
                let aw = c.en(ad, font);
                let ay = kutu.alt() + c.px(4);
                if secili {
                    isaret(c, x, r.x + c.px(7), ay + c.px(11), c.px(7), Durum::Tamam);
                }
                let ax = r.x + if secili { c.px(20) } else { 0 };
                c.yazi(ad, font, j.metin, Dik::yeni(ax, ay, aw.min(r.sag() - ax) + c.px(2), c.px(22)), TEK);
                if d.ayar_odak == Some(AyarOge::Tema(k)) {
                    odak_ciz(c, x, r, 0);
                }
                d.hedefler.push((r, Hedef::Ayar(AyarOge::Tema(k))));
            }
            AP::Radyo(k) => {
                let m = Mod::HEPSI[k];
                let secili = gorunum.modu == m;
                let gr = c.px(8);
                let (gx, gy) = (r.x + c.px(4) + gr, r.y + r.h / 2);
                if secili {
                    c.halka(gx, gy, gr, x.gosterge(), c.px(2));
                    c.daire(gx, gy, gr / 2, x.gosterge(), x.gosterge(), 1);
                } else {
                    c.daire(gx, gy, gr, j.yuzey, j.kenar, if x.tema == Tema::Mezura { c.px(2) } else { 1 });
                }
                c.yazi(m.gorunen_ad(), x.f.govde, j.metin, Dik::yeni(gx + gr + c.px(8), r.y, r.sag() - gx - gr - c.px(8), r.h), TEK_TAM);
                if d.ayar_odak == Some(AyarOge::Mod(k)) {
                    odak_ciz(c, x, r, c.px(x.g.dugme_yaricap));
                }
                d.hedefler.push((r, Hedef::Ayar(AyarOge::Mod(k))));
            }
            AP::Ornek => {
                vurgu_dolgu(c, x.tema, x.koyu, &j, r, c.px(x.g.etiket_yaricap));
                if !x.tema.vurgu_cerceveli(x.koyu) {
                    c.yuvarlak_cerceve(r, c.px(x.g.etiket_yaricap), 1, j.kenar);
                }
            }
            AP::Kutu => {
                let isaretli = x.azalt;
                let gr = c.px(9);
                let kr = Dik::yeni(r.x + c.px(4), r.y + r.h / 2 - gr, 2 * gr, 2 * gr);
                if isaretli {
                    vurgu_dolgu(c, x.tema, x.koyu, &j, kr, c.px(x.g.etiket_yaricap.min(3)));
                    c.tik(kr.x + gr, kr.y + gr, gr, j.vurgu_ustu);
                } else {
                    c.kutu(kr, c.px(x.g.etiket_yaricap.min(3)), j.yuzey, Some((j.kenar, if x.tema == Tema::Mezura { c.px(2) } else { 1 })));
                }
                let et = "Hareketi azalt";
                c.yazi(et, x.f.govde, j.metin, Dik::yeni(kr.sag() + c.px(10), r.y, r.sag() - kr.sag() - c.px(10), r.h), TEK_TAM);
                let ew = kr.w + c.px(14) + c.en(et, x.f.govde) + c.px(8);
                let hr = Dik::yeni(r.x, r.y, ew.min(r.w), r.h);
                if d.ayar_odak == Some(AyarOge::HareketAzalt) {
                    odak_ciz(c, x, hr, c.px(x.g.dugme_yaricap));
                }
                d.hedefler.push((hr, Hedef::Ayar(AyarOge::HareketAzalt)));
            }
        }
    }
}

// ------------------------------------------------------------------ yükleme ekranı

fn yukleme_ciz(c: &Cizer, x: &Ctx, yk: Yukleme, w: i32, h: i32, baslatiliyor: bool) {
    let j = x.j;
    let kare = yk.kare(x.azalt);
    let ls = c.px(56);
    let blok = ls + c.px(12) + c.px(40) + c.px(20) + c.px(64) + c.px(28);
    let ust = (h - blok).max(0) / 2;
    logo_ciz(c, x.tema, x.koyu, &j, ls, (w - ls) / 2, ust);
    let bas = "TAA Kurucu";
    c.yazi(bas, x.f.baslik, j.metin, Dik::yeni(0, ust + ls + c.px(12), w, c.px(40)), TEK_TAM | DT_CENTER);
    let my = ust + ls + c.px(12) + c.px(40) + c.px(20);
    match x.tema {
        Tema::Ilmek => {
            // 140 px teyel hattı: her tikte bir parça dikişe döner, son tikte uçta düğüm belirir.
            let l = c.px(140);
            let x1 = (w - l) / 2;
            let (n, dugum) = y::yukleme_ilmek(kare);
            let yy = my + c.px(16);
            teyel_hat(c, &j, x1, x1 + l, yy, x1 + n * l / 10, c.px(2));
            if dugum {
                c.daire(x1 + l, yy, c.px(6), j.vurgu, j.vurgu, 1);
            }
        }
        Tema::Kalip => {
            // Taban sabit; üç kalıp parçası 3'er tik arayla oturur; son tikte köşe nişanları.
            let l = c.px(180);
            let x1 = (w - l) / 2;
            let taban = my + c.px(44);
            c.yatay(x1, x1 + l, taban, c.px(2), j.kenar);
            let olc = [(48, 26), (40, 34), (56, 20)];
            let top_w: i32 = olc.iter().map(|(pw, _)| c.px(*pw)).sum::<i32>() + 2 * c.px(10);
            let mut px_ = (w - top_w) / 2;
            for (k, (pw, ph)) in olc.iter().enumerate() {
                let (pw, ph) = (c.px(*pw), c.px(*ph));
                if let Some(off) = y::yukleme_kalip_parca(k, kare) {
                    let r = Dik::yeni(px_, taban - c.px(1) - ph - c.px(off), pw, ph);
                    if k == 1 {
                        c.kutu(r, c.px(2), j.vurgu, None);
                    } else {
                        c.kutu(r, c.px(2), j.yuzey2, Some((j.kenar, 1)));
                    }
                    let no = format!("{:02}", k + 1);
                    c.yazi(&no, x.f.kucuk_kalin, if k == 1 { j.vurgu_ustu } else { j.metin }, r, TEK_TAM | DT_CENTER);
                }
                px_ += pw + c.px(10);
            }
            if y::yukleme_kalip_hazir(kare) {
                let alan = Dik::yeni(x1 - c.px(12), my - c.px(4), l + c.px(24), c.px(60));
                let a = c.px(9);
                for (cx, cy) in [(alan.x, alan.y), (alan.sag(), alan.y), (alan.x, alan.alt()), (alan.sag(), alan.alt())] {
                    c.yatay(cx - a / 2, cx + a / 2 + 1, cy, c.p1(), j.kenar);
                    c.dikey(cx, cy - a / 2, cy + a / 2 + 1, c.p1(), j.kenar);
                }
                c.yazi("Kuruluma hazır", x.f.kucuk, j.soluk, Dik::yeni(0, alan.alt() + c.px(6), w, c.px(20)), TEK_TAM | DT_CENTER);
            }
        }
        Tema::Mezura => {
            // 200 px cetvel hazır; sarı bant her tikte %10 uzar, son tikte imleç yerine oturur.
            let l = c.px(200);
            let (bant, imlec) = y::yukleme_mezura(kare);
            cetvel_ciz(c, x.tema, x.koyu, &j, Dik::yeni((w - l) / 2, my, l, c.px(60)), bant, imlec, Some(x.f.cetvel), x.dpi);
        }
    }
    if baslatiliyor {
        c.yazi("Başlatılıyor…", x.f.kucuk, j.soluk, Dik::yeni(0, my + c.px(64) + c.px(24), w, c.px(20)), TEK_TAM | DT_CENTER);
    }
}

// ------------------------------------------------------------------ ilk açılış tanıtımı

fn tanitim_ciz(c: &Cizer, d: &mut PencereDurumu, x: &Ctx, t: Tanitim, w: i32, h: i32) {
    let j = x.j;
    let tum = Dik::yeni(0, 0, w, h);
    c.doldur(tum, j.zemin);
    d.hedefler.push((tum, Hedef::Bos));
    let m = c.px(32);
    let ls = c.px(28);
    logo_ciz(c, x.tema, x.koyu, &j, ls, m, c.px(20));
    let mw = c.en("TAA Kurucu", x.f.marka);
    c.yazi("TAA Kurucu", x.f.marka, j.metin, Dik::yeni(m + ls + c.px(10), c.px(20), mw + c.px(2), ls), TEK_TAM);
    let sayac = format!("{} / {}", t.sahne + 1, y::SAHNE_SAYISI);
    let sw = c.en(&sayac, x.f.kucuk_kalin);
    c.yazi(&sayac, x.f.kucuk_kalin, j.soluk, Dik::yeni(w - m - sw, c.px(20), sw + c.px(2), ls), TEK_TAM);
    // Düğmeler: Atla / İleri (sonda Başla); otomatik ilerlemez.
    let bh = c.px(x.g.dugme);
    let alt_y = h - c.px(24) - bh;
    let ileri = t.dugme();
    let bw1 = dugme_gen(c, x, ileri);
    let bw2 = dugme_gen(c, x, "Atla");
    let r1 = Dik::yeni(w - m - bw1, alt_y, bw1, bh);
    let r2 = Dik::yeni(r1.x - c.px(12) - bw2, alt_y, bw2, bh);
    dugme_ciz(c, d, x, r2, "Atla", false, Hedef::TanitimAtla, false);
    dugme_ciz(c, d, x, r1, ileri, true, Hedef::TanitimIleri, false);
    let ip = "Enter: İleri · Esc: Atla";
    let iw = (r2.x - m - c.px(12)).max(0);
    let ih = c.olc(ip, x.f.kucuk, iw).min(bh);
    c.yazi(ip, x.f.kucuk, j.soluk, Dik::yeni(m, alt_y + (bh - ih) / 2, iw, ih), SARILI);
    // İçerik.
    let (bas, acik) = y::sahne_metni(x.tema, t.sahne);
    let cw = (w - 2 * m).min(c.px(640));
    let cx = (w - cw) / 2;
    let bas_h = c.olc(bas, x.f.baslik, cw);
    let acik_h = c.olc(acik, x.f.govde, cw);
    let ust = c.px(64);
    let alt_sinir = alt_y - c.px(20);
    let ill_h = (alt_sinir - ust - c.px(30) - c.px(20) - bas_h - c.px(10) - acik_h).clamp(c.px(60), c.px(240));
    let et = "Tanıtım örneği";
    let ew = c.en(et, x.f.kucuk_kalin) + c.px(20);
    let er = Dik::yeni(cx, ust, ew, c.px(24));
    c.kutu(er, c.px(x.g.etiket_yaricap), j.yuzey2, if x.tema == Tema::Mezura { Some((j.kenar, c.px(2))) } else { None });
    c.yazi(et, x.f.kucuk_kalin, j.metin, er, TEK_TAM | DT_CENTER);
    // Sahne göstergesi (tema imzasıyla): İlmek düğümler, Kalıp numaralar, Mezura alt çizgi.
    sahne_gostergesi(c, x, t.sahne, Dik::yeni(cx + cw - c.px(120), ust, c.px(120), c.px(24)));
    let ill = Dik::yeni(cx, ust + c.px(30), cw, ill_h);
    let adlar: Vec<String> = d.kartlar.iter().filter(|k| k.tur == "program").take(3).map(|k| k.ad.clone()).collect();
    sahne_ciz(c, x, t.sahne, t.kare(x.azalt), ill, &adlar);
    let by = ill.alt() + c.px(20);
    c.yazi(bas, x.f.baslik, j.metin, Dik::yeni(cx, by, cw, bas_h), SARILI);
    c.yazi(acik, x.f.govde, j.soluk, Dik::yeni(cx, by + bas_h + c.px(10), cw, acik_h.min(alt_sinir - by - bas_h)), SARILI);
}

fn sahne_gostergesi(c: &Cizer, x: &Ctx, sahne: u8, r: Dik) {
    let j = x.j;
    match x.tema {
        Tema::Ilmek => {
            let cy = r.y + r.h / 2;
            let xs = [r.x + c.px(20), r.x + c.px(60), r.x + c.px(100)];
            for q in 0..2 {
                if (q as u8) < sahne {
                    c.yatay(xs[q] + c.px(6), xs[q + 1] - c.px(6), cy, c.px(2), j.vurgu);
                } else {
                    c.teyel(xs[q] + c.px(6), xs[q + 1] - c.px(6), cy, c.px(2), j.kenar);
                }
            }
            for (q, px_) in xs.iter().enumerate() {
                let q = q as u8;
                if q < sahne {
                    c.daire(*px_, cy, c.px(5), j.vurgu, j.vurgu, 1);
                } else if q == sahne {
                    c.halka(*px_, cy, c.px(6), j.vurgu, c.px(2));
                    c.daire(*px_, cy, c.px(3), j.vurgu, j.vurgu, 1);
                } else {
                    c.daire(*px_, cy, c.px(5), j.zemin, j.kenar, c.px(2));
                }
            }
        }
        Tema::Kalip => {
            for q in 0..3u8 {
                let h = Dik::yeni(r.x + q as i32 * c.px(40), r.y, c.px(36), r.h);
                if q == sahne {
                    c.doldur(h, j.yuzey2);
                    c.doldur(Dik::yeni(h.x, h.y, c.px(3), h.h), j.vurgu);
                }
                c.yazi(&format!("{:02}", q + 1), x.f.kucuk_kalin, if q == sahne { j.metin } else { j.soluk }, h, TEK_TAM | DT_CENTER);
            }
        }
        Tema::Mezura => {
            for q in 0..3u8 {
                let h = Dik::yeni(r.x + q as i32 * c.px(40), r.y, c.px(36), r.h);
                let s = (q + 1).to_string();
                c.yazi(&s, if q == sahne { x.f.kalin } else { x.f.govde }, if q <= sahne { j.metin } else { j.soluk }, h, TEK_TAM | DT_CENTER);
                if q == sahne {
                    c.doldur(Dik::yeni(h.x + c.px(8), h.alt() - c.px(2), h.w - c.px(16), c.px(4)), x.gosterge());
                }
            }
        }
    }
}

/// Tanıtım sahnelerinin çizimi (KARAR §2.6, §3.6, §4.6). `kare` 0..=4; hareketi azaltta 4 (son kare).
fn sahne_ciz(c: &Cizer, x: &Ctx, sahne: u8, kare: u8, ill: Dik, adlar: &[String]) {
    let j = x.j;
    let s = |v: i32| c.px(v);
    let ad = |k: usize| adlar.get(k).cloned().unwrap_or_else(|| ["TAA Etiket", "Tutaa", "TaaRkod"][k.min(2)].to_string());
    match x.tema {
        Tema::Ilmek => c.kutu(ill, s(x.g.kart_yaricap), j.yuzey, None),
        Tema::Kalip => c.kutu(ill, s(x.g.kart_yaricap), j.yuzey, Some((j.kenar, 1))),
        Tema::Mezura => c.kutu(ill, 0, j.zemin, Some((j.kenar, s(2)))),
    }
    let _k = c.kirp(ill);
    let a = ill.ic2(s(28), s(20));
    let k = kare as i32;
    match (x.tema, sahne) {
        (Tema::Ilmek, 0) => {
            // İki rol alanı arası teyel dikilir.
            let bw = (a.w / 3).min(s(180));
            let bh = s(60);
            let by = a.y + (a.h - bh) / 2;
            let sol = Dik::yeni(a.x, by, bw, bh);
            let sag = Dik::yeni(a.sag() - bw, by, bw, bh);
            c.kutu(sol, s(10), ara_renk(j.yuzey, j.vurgu, 0.10), Some((j.vurgu, s(2))));
            c.kutu(sag, s(10), j.yuzey, Some((j.kenar, 1)));
            c.yazi("Şube PC", x.f.kalin, j.metin, sol, TEK_TAM | DT_CENTER);
            c.yazi("Sunucu PC", x.f.kalin, j.metin, sag, TEK_TAM | DT_CENTER);
            let (x1, x2) = (sol.sag() + s(10), sag.x - s(10));
            teyel_hat(c, &j, x1, x2, by + bh / 2, x1 + (x2 - x1) * k / 4, s(2));
        }
        (Tema::Ilmek, 1) => {
            // Üç satır görünür, zorunlu satır kilit işareti alır.
            let rh = s(40);
            let ust = a.y + (a.h - 3 * rh - 2 * s(8)) / 2;
            for q in 0..3 {
                if k <= q {
                    continue;
                }
                let r = Dik::yeni(a.x, ust + q * (rh + s(8)), a.w, rh);
                c.kutu(r, s(10), j.yuzey, Some((j.kenar, 1)));
                let gr = s(8);
                let kr = Dik::yeni(r.x + s(14), r.y + rh / 2 - gr, 2 * gr, 2 * gr);
                if q < 2 {
                    c.kutu(kr, s(3), j.vurgu, None);
                    c.tik(kr.x + gr, kr.y + gr, gr, j.vurgu_ustu);
                } else {
                    c.kutu(kr, s(3), j.yuzey, Some((j.kenar, 1)));
                }
                c.yazi(&ad(q as usize), x.f.govde, j.metin, Dik::yeni(kr.sag() + s(12), r.y, r.w / 2, rh), TEK);
                if q == 0 && k >= 4 {
                    // Kilit: gövde + kulp.
                    let lx = r.sag() - s(90);
                    let ly = r.y + rh / 2;
                    c.yuvarlak_cerceve(Dik::yeni(lx + s(2), ly - s(9), s(10), s(10)), s(4), s(2), j.soluk);
                    c.kutu(Dik::yeni(lx, ly - s(3), s(14), s(10)), s(2), j.soluk, None);
                    c.yazi("Zorunlu", x.f.kucuk, j.soluk, Dik::yeni(lx + s(20), r.y, s(70), rh), TEK);
                }
            }
        }
        (Tema::Ilmek, _) => {
            // Satırlar paket listesi düzenine oturur.
            let dx = s((4 - k).max(0) * 6);
            let rh = s(40);
            c.yazi("Paket", x.f.kucuk_kalin, j.soluk, Dik::yeni(a.x + dx, a.y, a.w / 2, s(20)), TEK);
            c.yazi("Durum", x.f.kucuk_kalin, j.soluk, Dik::yeni(a.sag() - s(120) + dx, a.y, s(120), s(20)), TEK);
            for q in 0..3 {
                let yy = a.y + s(24) + q * rh;
                c.yazi(&ad(q as usize), x.f.govde, j.metin, Dik::yeni(a.x + dx, yy, a.w / 2, rh), TEK);
                isaret(c, x, a.sag() - s(120) + dx + s(7), yy + rh / 2, s(7), Durum::Tamam);
                c.yazi("Hazır", x.f.govde, j.basari, Dik::yeni(a.sag() - s(98) + dx, yy, s(98), rh), TEK);
                if q < 2 {
                    c.teyel(a.x, a.sag(), yy + rh, 1, j.kenar);
                }
            }
        }
        (Tema::Kalip, 0) => {
            // İki modül ızgaraya oturur (8 -> 3 -> 0 px; ikincisi bir tik sonra).
            let mw = (a.w - s(16)) / 2;
            let mh = s(80).min(a.h);
            let my = a.y + (a.h - mh) / 2;
            let bilgi = [("01", "Görev", "Şube PC"), ("02", "Şube", "Örnek Şube")];
            for (q, (no, et, deger)) in bilgi.iter().enumerate() {
                let bas = q as i32;
                if k < bas {
                    continue;
                }
                let off = s(taa_kur_tema::KALIP_OTURMA_PX[((k - bas) as usize).min(2)]);
                let r = Dik::yeni(a.x + q as i32 * (mw + s(16)) + off, my, mw, mh);
                c.kutu(r, s(4), j.yuzey, Some((j.kenar, 1)));
                c.yazi(no, x.f.yuzde, j.metin, Dik::yeni(r.x + s(12), r.y + s(8), s(40), s(26)), TEK_TAM);
                c.yazi(et, x.f.kucuk_kalin, j.soluk, Dik::yeni(r.x + s(56), r.y + s(8), r.w - s(64), s(26)), TEK);
                c.yazi(deger, x.f.bolum, j.metin, Dik::yeni(r.x + s(12), r.y + s(38), r.w - s(24), s(30)), TEK);
            }
        }
        (Tema::Kalip, 1) => {
            // Dört adım hücresi soldan sağa belirir.
            let hw = (a.w - 3 * s(8)) / 4;
            let hh = s(48);
            let hy = a.y + (a.h - hh) / 2;
            let durumlar = [Durum::Tamam, Durum::Tamam, Durum::Suruyor, Durum::Bekliyor];
            for (q, adim) in Adim::HEPSI.iter().enumerate() {
                if k <= q as i32 {
                    continue;
                }
                let r = Dik::yeni(a.x + q as i32 * (hw + s(8)), hy, hw, hh);
                let dur = durumlar[q];
                c.kutu(r, s(2), if dur == Durum::Suruyor { ara_renk(j.yuzey2, j.vurgu, 0.12) } else { j.yuzey2 }, Some((if dur == Durum::Suruyor { j.vurgu } else { j.kenar }, 1)));
                isaret(c, x, r.x + s(16), r.y + hh / 2, s(7), dur);
                c.yazi(adim.etiket(), x.f.govde, j.metin, Dik::yeni(r.x + s(30), r.y, r.w - s(34), hh), TEK_TAM);
            }
        }
        (Tema::Kalip, _) => {
            // İki sürüm sütunu görünür, Güncelle işaretlenir.
            let sut = [a.w * 40 / 100, a.w * 30 / 100, a.w * 30 / 100];
            let basliklar = ["Paket", "Kurulu sürüm", "Seçili sürüm"];
            let satirlar = [(ad(1), "1.4.0", "1.5.0"), (ad(2), "2.1.0", "2.1.0")];
            let rh = s(34);
            let mut sx = a.x;
            for (q, b) in basliklar.iter().enumerate() {
                c.yazi(b, x.f.kucuk_kalin, j.soluk, Dik::yeni(sx + s(8), a.y, sut[q] - s(16), rh), if q == 0 { TEK } else { TEK | DT_RIGHT });
                sx += sut[q];
            }
            c.yatay(a.x, a.sag(), a.y + rh, 1, j.kenar);
            for (n, (p_, ku, se)) in satirlar.iter().enumerate() {
                let yy = a.y + rh * (n as i32 + 1);
                if n == 0 {
                    c.doldur(Dik::yeni(a.x, yy + 1, s(3), rh - 1), j.vurgu);
                }
                c.yazi(p_, x.f.govde, j.metin, Dik::yeni(a.x + s(8), yy, sut[0] - s(16), rh), TEK);
                c.yazi(ku, x.f.govde, j.metin, Dik::yeni(a.x + sut[0] + s(8), yy, sut[1] - s(16), rh), TEK | DT_RIGHT);
                c.yazi(se, x.f.govde, j.metin, Dik::yeni(a.x + sut[0] + sut[1] + s(8), yy, sut[2] - s(16), rh), TEK | DT_RIGHT);
                c.yatay(a.x, a.sag(), yy + rh, 1, j.kenar);
            }
            let et = "Güncelle";
            let bw = dugme_gen(c, x, et);
            let br = Dik::yeni(a.sag() - bw, a.y + 3 * rh + s(12), bw, s(x.g.dugme));
            if br.alt() <= a.alt() + s(16) {
                if k >= 2 {
                    c.kutu(br, s(4), j.vurgu, None);
                    c.yazi(et, x.f.dugme, j.vurgu_ustu, br, TEK_TAM | DT_CENTER);
                } else {
                    c.kutu(br, s(4), j.yuzey, Some((j.kenar, 1)));
                    c.yazi(et, x.f.dugme, j.metin, br, TEK_TAM | DT_CENTER);
                }
            }
        }
        (Tema::Mezura, 0) => {
            // Rol ve şube örneği.
            let rh = s(56).min((a.h - s(12)) / 2);
            for (q, (et, deger)) in [("Rol", "Şube PC"), ("Şube", "Örnek Şube")].iter().enumerate() {
                let r = Dik::yeni(a.x, a.y + q as i32 * (rh + s(12)), a.w, rh);
                c.kutu(r, 0, j.yuzey, Some((j.kenar, s(2))));
                c.yazi(et, x.f.govde, j.soluk, Dik::yeni(r.x + s(16), r.y, s(80), rh), TEK);
                c.yazi(deger, x.f.bolum, j.metin, Dik::yeni(r.x + s(100), r.y, r.w - s(116), rh), TEK);
            }
        }
        (Tema::Mezura, 1) => {
            // Dört adım sırayla ✓ olur.
            let hw = a.w / 4;
            let cy = a.y + a.h / 2;
            for (q, adim) in Adim::HEPSI.iter().enumerate() {
                let dur = if k > q as i32 { Durum::Tamam } else { Durum::Bekliyor };
                let xx = a.x + q as i32 * hw;
                isaret(c, x, xx + s(10), cy, s(9), dur);
                c.yazi(adim.etiket(), x.f.bolum, if dur == Durum::Tamam { j.metin } else { j.soluk }, Dik::yeni(xx + s(26), cy - s(16), hw - s(28), s(32)), TEK_TAM);
            }
        }
        (Tema::Mezura, _) => {
            // Sonuç özeti ve "TAA Kur'u aç".
            let oz = "6 program hazır.";
            isaret(c, x, a.x + s(10), a.y + s(16), s(10), Durum::Tamam);
            c.yazi(oz, x.f.bolum, j.basari, Dik::yeni(a.x + s(28), a.y, a.w - s(28), s(32)), TEK);
            cetvel_ciz(c, x.tema, x.koyu, &j, Dik::yeni(a.x, a.y + s(40), a.w, s(40)), 100, true, None, x.dpi);
            let et = "TAA Kur'u aç";
            let bw = dugme_gen(c, x, et);
            let br = Dik::yeni(a.sag() - bw, a.alt() - s(x.g.dugme), bw, s(x.g.dugme));
            if br.y > a.y + s(84) {
                c.kutu(br, 0, j.vurgu, Some((j.kenar, s(2))));
                c.yazi(et, x.f.dugme, j.vurgu_ustu, br, TEK_TAM | DT_CENTER);
            }
        }
    }
}
