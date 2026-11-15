// Sahip: Yusuf. yazan: [Claude#kur-yerel-pencere 2026-11-15]
//! Yerel kurulum penceresi (yalnız Windows; Win32 + GDI, windows-sys). Tarayıcı ya da HTTP YOKTUR.
//! İnce çizim katmanı: kurulum işi ayrı iş parçacığında `ekran::Paylasim`'ı günceller; pencere 120 ms'de bir
//! anlık kopyasını çizer (çift tamponlu), tuş ve tıklamaları `ekran::Komut` olarak gönderir.
//! Görsel dil: tek vurgu rengi, nötr griler, bol boşluk, ince çizgi, gölgesiz; açık/koyu sistem temasından
//! (kayıt defteri AppsUseLightTheme), yüksek DPI (SetProcessDPIAware + GetDpiForWindow + WM_DPICHANGED).
//! Klavye: Enter = onay / Yeniden dene / Bitir, Esc = vazgeç / kapat, Yukarı/Aşağı + Boşluk = seçim, Sol/Sağ = kartlar.
//! DİKKAT: Windows'ta elle DENENMEDİ; Linux'ta yalnız `cargo check/clippy --target x86_64-pc-windows-gnu`.

use crate::direktif::{Direktif, Durum as DDurum};
use crate::ekran::{Komut, Paylasim, Uc};
use crate::ilerleme::{Adim, Asama, Durum};
use crate::kartlar::{self, Kart};
use crate::secim::Secim;
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::time::Instant;
use windows_sys::Win32::Foundation::{COLORREF, ERROR_SUCCESS, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen, CreateSolidBrush, DeleteDC,
    DeleteObject, DrawTextW, EndPaint, FillRect, GetMonitorInfoW, InvalidateRect, MonitorFromWindow, Polyline,
    RoundRect, SelectObject, SetBkMode, SetTextColor, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
    DEFAULT_PITCH, DT_CALCRECT, DT_CENTER, DT_EDITCONTROL, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_RIGHT,
    DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK, FW_NORMAL, FW_SEMIBOLD, HDC, HFONT, MONITORINFO,
    MONITOR_DEFAULTTONEAREST, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID, SRCCOPY, TRANSPARENT,
};
use windows_sys::Win32::System::Console::{GetConsoleProcessList, GetConsoleWindow};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    VK_DOWN, VK_ESCAPE, VK_LEFT, VK_RETURN, VK_RIGHT, VK_SPACE, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW, GetWindowLongPtrW,
    LoadCursorW, LoadIconW, MessageBoxW, PostQuitMessage, RegisterClassW, SetCursor, SetProcessDPIAware, SetTimer,
    SetWindowLongPtrW, SetWindowPos, ShowWindow, TranslateMessage, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA,
    HTCLIENT, IDC_ARROW, IDC_HAND, IDI_APPLICATION, IDYES, MB_ICONQUESTION, MB_YESNO, MINMAXINFO, MSG, SWP_NOZORDER,
    SW_HIDE, SW_SHOW, WM_CLOSE, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_KEYDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SETCURSOR, WM_SETTINGCHANGE,
    WM_SIZE, WM_TIMER, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

const SINIF: &str = "TaaKurKurulum";
const ZAMANLAYICI: usize = 1;
const TIK_MS: u32 = 120;
/// Mantıksal (96 DPI) pencere boyutu ve en küçük boyut.
const GENISLIK: i32 = 1040;
const YUKSEKLIK: i32 = 700;
const EN_AZ: (i32, i32) = (860, 600);

const fn rgb(r: u8, g: u8, b: u8) -> COLORREF {
    r as u32 | (g as u32) << 8 | (b as u32) << 16
}

/// İki rengi karıştırır (t: 0 = a, 255 = b).
fn karis(a: COLORREF, b: COLORREF, t: u32) -> COLORREF {
    let k = |s: u32| -> u32 {
        let x = (a >> s) & 0xFF;
        let y = (b >> s) & 0xFF;
        (x * (255 - t) + y * t) / 255
    };
    k(0) | k(8) << 8 | k(16) << 16
}

#[derive(Clone, Copy)]
struct Renkler {
    zemin: COLORREF,
    yuzey: COLORREF,
    kenar: COLORREF,
    metin: COLORREF,
    soluk: COLORREF,
    vurgu: COLORREF,
    vurgu_ustu: COLORREF,
    hata: COLORREF,
}

impl Renkler {
    fn tema(koyu: bool) -> Self {
        if koyu {
            Renkler {
                zemin: rgb(0x16, 0x16, 0x17),
                yuzey: rgb(0x1F, 0x1F, 0x21),
                kenar: rgb(0x33, 0x33, 0x36),
                metin: rgb(0xEC, 0xEC, 0xEE),
                soluk: rgb(0x9C, 0x9C, 0xA3),
                vurgu: rgb(0x6E, 0x93, 0xF5),
                vurgu_ustu: rgb(0x10, 0x14, 0x20),
                hata: rgb(0xF0, 0x6A, 0x6A),
            }
        } else {
            Renkler {
                zemin: rgb(0xF6, 0xF6, 0xF4),
                yuzey: rgb(0xFF, 0xFF, 0xFF),
                kenar: rgb(0xE4, 0xE3, 0xDF),
                metin: rgb(0x1E, 0x1E, 0x20),
                soluk: rgb(0x6B, 0x6B, 0x70),
                vurgu: rgb(0x2B, 0x5C, 0xD6),
                vurgu_ustu: rgb(0xFF, 0xFF, 0xFF),
                hata: rgb(0xC0, 0x32, 0x32),
            }
        }
    }
}

/// Sistem uygulama teması koyu mu (HKCU ...\Themes\Personalize\AppsUseLightTheme = 0).
fn sistem_koyu() -> bool {
    let anahtar = genis(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let deger = genis("AppsUseLightTheme");
    let mut v: u32 = 1;
    let mut boy: u32 = 4;
    // SAFETY: sıfır sonlu dizgiler ve yazılabilir u32; RRF_RT_REG_DWORD 4 bayt yazar.
    let s = unsafe {
        RegGetValueW(HKEY_CURRENT_USER, anahtar.as_ptr(), deger.as_ptr(), RRF_RT_REG_DWORD, null_mut(), &mut v as *mut u32 as *mut c_void, &mut boy)
    };
    s == ERROR_SUCCESS && v == 0
}

fn genis(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[derive(Clone, Copy)]
struct F {
    baslik: HFONT,
    alt: HFONT,
    kalin: HFONT,
    govde: HFONT,
    kucuk: HFONT,
    etiket: HFONT,
}

/// Sahip olunan fontlar (Drop'ta silinir); çizimde kopyalanabilir tutamaklar (`.0`) kullanılır.
struct Fontlar(F);

impl Fontlar {
    fn yap(dpi: u32) -> Self {
        let f = |px: i32, agirlik: u32| {
            let ad = genis("Segoe UI");
            let h = -(px * dpi as i32 / 96);
            // SAFETY: ad sıfır sonlu; dönen tutamak Drop'ta silinir.
            unsafe {
                CreateFontW(h, 0, 0, 0, agirlik as i32, 0, 0, 0, DEFAULT_CHARSET as u32, OUT_DEFAULT_PRECIS as u32,
                    CLIP_DEFAULT_PRECIS as u32, CLEARTYPE_QUALITY as u32, DEFAULT_PITCH as u32, ad.as_ptr())
            }
        };
        Fontlar(F {
            baslik: f(24, FW_SEMIBOLD),
            alt: f(17, FW_SEMIBOLD),
            kalin: f(14, FW_SEMIBOLD),
            govde: f(14, FW_NORMAL),
            kucuk: f(12, FW_NORMAL),
            etiket: f(11, FW_SEMIBOLD),
        })
    }
}

impl Drop for Fontlar {
    fn drop(&mut self) {
        let t = self.0;
        for f in [t.baslik, t.alt, t.kalin, t.govde, t.kucuk, t.etiket] {
            // SAFETY: CreateFontW ile yaratılan, artık seçili olmayan tutamak.
            unsafe { DeleteObject(f) };
        }
    }
}

/// Tıklanabilir alanlar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hedef {
    Onay,
    Vazgec,
    Secenek(usize),
    YenidenDene,
    Bitir,
    TaaKurAc,
    KartGeri,
    KartIleri,
}

struct PencereDurumu {
    uc: Uc,
    kartlar: Vec<Kart>,
    kart: usize,
    kart_zamani: Instant,
    kart_alani: RECT,
    dpi: u32,
    renk: Renkler,
    koyu: bool,
    fontlar: Fontlar,
    hedefler: Vec<(RECT, Hedef)>,
    uzerinde: Option<Hedef>,
    fare: (i32, i32),
    baslangic: Instant,
}

/// Açık pencere; `dongu` ileti döngüsünü çalıştırır.
pub struct Pencere {
    hwnd: HWND,
}

impl Pencere {
    /// Pencereyi yaratıp gösterir. Hata: kurulum konsolda sürer.
    pub fn ac(uc: Uc) -> Result<Pencere, String> {
        // SAFETY: Win32 çağrıları; durum kutusu WM_NCDESTROY'da geri alınır.
        unsafe {
            SetProcessDPIAware();
            let ornek = GetModuleHandleW(null());
            let sinif = genis(SINIF);
            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(pencere_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: ornek,
                hIcon: LoadIconW(null_mut(), IDI_APPLICATION),
                hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                hbrBackground: null_mut(),
                lpszMenuName: null(),
                lpszClassName: sinif.as_ptr(),
            };
            if RegisterClassW(&wc) == 0 {
                return Err("pencere sınıfı kaydedilemedi".into());
            }
            let koyu = sistem_koyu();
            let durum = Box::new(PencereDurumu {
                uc,
                kartlar: kartlar::gomulu(),
                kart: 0,
                kart_zamani: Instant::now(),
                kart_alani: RECT { left: 0, top: 0, right: 0, bottom: 0 },
                dpi: 96,
                renk: Renkler::tema(koyu),
                koyu,
                fontlar: Fontlar::yap(96),
                hedefler: vec![],
                uzerinde: None,
                fare: (-1, -1),
                baslangic: Instant::now(),
            });
            let baslik = genis("TAA Kur kurulumu");
            let hwnd = CreateWindowExW(0, sinif.as_ptr(), baslik.as_ptr(), WS_OVERLAPPEDWINDOW, CW_USEDEFAULT,
                CW_USEDEFAULT, GENISLIK, YUKSEKLIK, null_mut(), null_mut(), ornek, Box::into_raw(durum) as *const c_void);
            if hwnd.is_null() {
                return Err("pencere yaratılamadı".into());
            }
            boyutla_ortala(hwnd);
            koyu_baslik(hwnd, koyu);
            ShowWindow(hwnd, SW_SHOW);
            SetTimer(hwnd, ZAMANLAYICI, TIK_MS, None);
            konsolu_gizle();
            Ok(Pencere { hwnd })
        }
    }

    /// İleti döngüsü: pencere kapanınca döner.
    pub fn dongu(self) {
        let _ = self.hwnd;
        // SAFETY: standart Win32 ileti döngüsü.
        unsafe {
            let mut m: MSG = std::mem::zeroed();
            while GetMessageW(&mut m, null_mut(), 0, 0) > 0 {
                TranslateMessage(&m);
                DispatchMessageW(&m);
            }
        }
    }
}

/// Kurucu çift tıklamayla açıldıysa (konsol yalnız bizim) konsol penceresini gizler; terminalden çalıştırılınca dokunmaz.
unsafe fn konsolu_gizle() {
    let mut l = [0u32; 4];
    if GetConsoleProcessList(l.as_mut_ptr(), l.len() as u32) == 1 {
        let k = GetConsoleWindow();
        if !k.is_null() {
            ShowWindow(k, SW_HIDE);
        }
    }
}

unsafe fn koyu_baslik(hwnd: HWND, koyu: bool) {
    let v: i32 = koyu as i32;
    DwmSetWindowAttribute(hwnd, DWMWA_USE_IMMERSIVE_DARK_MODE as u32, &v as *const i32 as *const c_void, 4);
}

/// DPI'ye göre boyutlandırır ve çalışma alanının ortasına koyar.
unsafe fn boyutla_ortala(hwnd: HWND) {
    let dpi = GetDpiForWindow(hwnd).max(96) as i32;
    let (w, h) = (GENISLIK * dpi / 96, YUKSEKLIK * dpi / 96);
    let mut mi: MONITORINFO = std::mem::zeroed();
    mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
    let (mut x, mut y) = (CW_USEDEFAULT, CW_USEDEFAULT);
    if GetMonitorInfoW(MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST), &mut mi) != 0 {
        let c = mi.rcWork;
        x = c.left + ((c.right - c.left) - w).max(0) / 2;
        y = c.top + ((c.bottom - c.top) - h).max(0) / 2;
    }
    SetWindowPos(hwnd, null_mut(), x, y, w, h, SWP_NOZORDER);
}

unsafe fn durum_al(hwnd: HWND) -> Option<&'static mut PencereDurumu> {
    let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PencereDurumu;
    p.as_mut()
}

fn kopya(d: &PencereDurumu) -> Option<Paylasim> {
    d.uc.durum.lock().ok().map(|p| p.clone())
}

fn icinde(r: &RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

/// Enter: seçimde onay; hata varken yeniden dene; bitmişse bitir.
fn enter(d: &mut PencereDurumu) {
    let Ok(mut p) = d.uc.durum.lock() else { return };
    if let Some(s) = p.secim.take() {
        drop(p);
        d.uc.gonder(Komut::Secim(s.sonuc()));
    } else if p.hata.is_some() {
        p.ilerleme.komut_bekleniyor = false;
        drop(p);
        d.uc.gonder(Komut::Bitir);
    } else if p.ilerleme.komut_bekleniyor {
        p.ilerleme.komut_bekleniyor = false;
        let hatali = p.ilerleme.hatalilar();
        drop(p);
        if hatali.is_empty() {
            d.uc.gonder(Komut::Bitir);
        } else {
            for a in hatali {
                d.uc.gonder(Komut::YenidenDene(a));
            }
        }
    }
}

/// Esc / Vazgeç: seçimde vazgeç; bekleme ya da hata ekranında bitir; kurulum sürerken kapatma sorusu.
fn vazgec(hwnd: HWND, d: &mut PencereDurumu) {
    let Ok(mut p) = d.uc.durum.lock() else { return };
    if p.secim.take().is_some() {
        drop(p);
        d.uc.gonder(Komut::Vazgec);
    } else if p.ilerleme.komut_bekleniyor || p.hata.is_some() {
        p.ilerleme.komut_bekleniyor = false;
        drop(p);
        d.uc.gonder(Komut::Bitir);
    } else {
        drop(p);
        kapat_sor(hwnd, d);
    }
}

/// Kapatma: bitmişse kapanır; bekleme ekranında Bitir; ilk onay ekranında Vazgeç; kurulum sürerken sorar.
fn kapat_sor(hwnd: HWND, d: &mut PencereDurumu) {
    let (bitti, bekliyor, hos_geldin) = match d.uc.durum.lock() {
        Ok(p) => (
            p.cikis.is_some(),
            p.ilerleme.komut_bekleniyor || p.hata.is_some(),
            p.secim.is_some() && p.ilerleme.asama == Asama::Hazirlik,
        ),
        Err(_) => (true, false, false),
    };
    if bitti {
        // SAFETY: kendi penceremiz.
        unsafe { DestroyWindow(hwnd) };
    } else if bekliyor {
        d.uc.gonder(Komut::Bitir);
    } else if hos_geldin {
        if let Ok(mut p) = d.uc.durum.lock() {
            p.secim = None;
        }
        d.uc.gonder(Komut::Vazgec);
    } else {
        let m = genis("Kurulum sürüyor. Pencere kapatılırsa kurulum yarıda kalır.\n\nKapatılsın mı?");
        let b = genis("TAA Kur kurulumu");
        // SAFETY: sıfır sonlu dizgiler; kendi penceremiz.
        unsafe {
            if MessageBoxW(hwnd, m.as_ptr(), b.as_ptr(), MB_YESNO | MB_ICONQUESTION) == IDYES {
                DestroyWindow(hwnd);
            }
        }
    }
}

fn secim_degistir(d: &PencereDurumu, f: impl FnOnce(&mut Secim)) -> bool {
    match d.uc.durum.lock() {
        Ok(mut p) => match p.secim.as_mut() {
            Some(s) => {
                f(s);
                true
            }
            None => false,
        },
        Err(_) => false,
    }
}

fn kart_cevir(d: &mut PencereDurumu, ileri: bool) {
    d.kart = kartlar::sonraki(d.kart, d.kartlar.len(), ileri);
    d.kart_zamani = Instant::now();
}

fn tikla(hwnd: HWND, d: &mut PencereDurumu, x: i32, y: i32) {
    let Some(h) = d.hedefler.iter().find(|(r, _)| icinde(r, x, y)).map(|(_, h)| *h) else { return };
    match h {
        Hedef::Onay | Hedef::YenidenDene => enter(d),
        Hedef::Bitir => {
            if let Ok(mut p) = d.uc.durum.lock() {
                p.ilerleme.komut_bekleniyor = false;
            }
            d.uc.gonder(Komut::Bitir);
        }
        Hedef::TaaKurAc => {
            if let Ok(mut p) = d.uc.durum.lock() {
                p.ilerleme.komut_bekleniyor = false;
            }
            d.uc.gonder(Komut::TaaKurAc);
        }
        Hedef::Vazgec => vazgec(hwnd, d),
        Hedef::Secenek(i) => {
            secim_degistir(d, |s| s.tikla(i));
        }
        Hedef::KartGeri => kart_cevir(d, false),
        Hedef::KartIleri => kart_cevir(d, true),
    }
}

unsafe extern "system" fn pencere_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lp as *const CREATESTRUCTW;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize);
        if let Some(d) = durum_al(hwnd) {
            d.dpi = GetDpiForWindow(hwnd).max(96);
            d.fontlar = Fontlar::yap(d.dpi);
        }
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    if msg == WM_NCDESTROY {
        let p = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut PencereDurumu;
        if !p.is_null() {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(p));
        }
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    let Some(d) = durum_al(hwnd) else { return DefWindowProcW(hwnd, msg, wp, lp) };
    match msg {
        WM_PAINT => {
            boya(hwnd, d);
            0
        }
        WM_ERASEBKGND => 1,
        WM_SIZE => {
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_TIMER => {
            let bitti = d.uc.durum.lock().map(|p| p.cikis.is_some()).unwrap_or(true);
            if bitti {
                DestroyWindow(hwnd);
                return 0;
            }
            let durgun = icinde(&d.kart_alani, d.fare.0, d.fare.1);
            if !durgun && d.kart_zamani.elapsed().as_millis() as u64 >= kartlar::DONUS_MS {
                kart_cevir(d, true);
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_KEYDOWN => {
            let k = wp as u16;
            if k == VK_RETURN {
                enter(d);
            } else if k == VK_ESCAPE {
                vazgec(hwnd, d);
            } else if k == VK_UP {
                secim_degistir(d, Secim::yukari);
            } else if k == VK_DOWN {
                secim_degistir(d, Secim::asagi);
            } else if k == VK_SPACE {
                secim_degistir(d, Secim::degistir);
            } else if k == VK_LEFT {
                kart_cevir(d, false);
            } else if k == VK_RIGHT {
                kart_cevir(d, true);
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_MOUSEWHEEL => {
            let delta = ((wp >> 16) & 0xFFFF) as u16 as i16;
            if delta > 0 {
                secim_degistir(d, Secim::yukari);
            } else if delta < 0 {
                secim_degistir(d, Secim::asagi);
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_MOUSEMOVE => {
            let (x, y) = ((lp & 0xFFFF) as u16 as i16 as i32, ((lp >> 16) & 0xFFFF) as u16 as i16 as i32);
            d.fare = (x, y);
            let h = d.hedefler.iter().find(|(r, _)| icinde(r, x, y)).map(|(_, h)| *h);
            if h != d.uzerinde {
                d.uzerinde = h;
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_SETCURSOR => {
            if (lp & 0xFFFF) as u32 == HTCLIENT && d.uzerinde.is_some() {
                SetCursor(LoadCursorW(null_mut(), IDC_HAND));
                return 1;
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_LBUTTONUP => {
            let (x, y) = ((lp & 0xFFFF) as u16 as i16 as i32, ((lp >> 16) & 0xFFFF) as u16 as i16 as i32);
            tikla(hwnd, d, x, y);
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_CLOSE => {
            kapat_sor(hwnd, d);
            0
        }
        WM_DPICHANGED => {
            d.dpi = ((wp >> 16) & 0xFFFF) as u32;
            d.fontlar = Fontlar::yap(d.dpi.max(96));
            let r = &*(lp as *const RECT);
            SetWindowPos(hwnd, null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER);
            0
        }
        WM_GETMINMAXINFO => {
            let mm = &mut *(lp as *mut MINMAXINFO);
            let s = d.dpi.max(96) as i32;
            mm.ptMinTrackSize = POINT { x: EN_AZ.0 * s / 96, y: EN_AZ.1 * s / 96 };
            0
        }
        WM_SETTINGCHANGE => {
            let koyu = sistem_koyu();
            if koyu != d.koyu {
                d.koyu = koyu;
                d.renk = Renkler::tema(koyu);
                koyu_baslik(hwnd, koyu);
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

// ------------------------------------------------------------------ çizim

/// GDI yardımcıları: mantıksal birim (96 DPI) → piksel, metin, yuvarlak dikdörtgen, daire, çizgi.
struct Cizer {
    dc: HDC,
    dpi: i32,
}

impl Cizer {
    fn px(&self, v: i32) -> i32 {
        v * self.dpi / 96
    }

    fn doldur(&self, r: RECT, renk: COLORREF) {
        // SAFETY: geçerli DC; fırça hemen silinir.
        unsafe {
            let b = CreateSolidBrush(renk);
            FillRect(self.dc, &r, b);
            DeleteObject(b);
        }
    }

    /// Yuvarlak köşeli dikdörtgen; `kenar` None ise çerçevesiz.
    fn kutu(&self, r: RECT, yaricap: i32, dolgu: COLORREF, kenar: Option<(COLORREF, i32)>) {
        // SAFETY: geçerli DC; seçilen nesneler geri seçilip silinir.
        unsafe {
            let (kr, kk) = kenar.unwrap_or((dolgu, 1));
            let kalem = CreatePen(PS_SOLID, kk.max(1), kr);
            let firca = CreateSolidBrush(dolgu);
            let ek = SelectObject(self.dc, kalem);
            let ef = SelectObject(self.dc, firca);
            RoundRect(self.dc, r.left, r.top, r.right, r.bottom, yaricap * 2, yaricap * 2);
            SelectObject(self.dc, ek);
            SelectObject(self.dc, ef);
            DeleteObject(kalem);
            DeleteObject(firca);
        }
    }

    fn daire(&self, cx: i32, cy: i32, r: i32, dolgu: COLORREF, kenar: COLORREF, kalinlik: i32) {
        self.kutu(RECT { left: cx - r, top: cy - r, right: cx + r + 1, bottom: cy + r + 1 }, r + 1, dolgu, Some((kenar, kalinlik)));
    }

    fn cizgi(&self, noktalar: &[(i32, i32)], renk: COLORREF, kalinlik: i32) {
        let p: Vec<POINT> = noktalar.iter().map(|(x, y)| POINT { x: *x, y: *y }).collect();
        // SAFETY: geçerli DC ve nokta dizisi.
        unsafe {
            let kalem = CreatePen(PS_SOLID, kalinlik.max(1), renk);
            let ek = SelectObject(self.dc, kalem);
            Polyline(self.dc, p.as_ptr(), p.len() as i32);
            SelectObject(self.dc, ek);
            DeleteObject(kalem);
        }
    }

    /// Onay işareti (daire içinde).
    fn tik(&self, cx: i32, cy: i32, r: i32, renk: COLORREF) {
        let k = |v: i32| v * r / 10;
        self.cizgi(&[(cx - k(5), cy), (cx - k(1), cy + k(4)), (cx + k(5), cy - k(4))], renk, self.px(2));
    }

    fn carpi(&self, cx: i32, cy: i32, r: i32, renk: COLORREF) {
        let k = r * 4 / 10;
        self.cizgi(&[(cx - k, cy - k), (cx + k + 1, cy + k + 1)], renk, self.px(2));
        self.cizgi(&[(cx + k, cy - k), (cx - k - 1, cy + k + 1)], renk, self.px(2));
    }

    /// Metin yazar; `bayrak` DT_*; dönen: kullanılan yükseklik (piksel).
    fn yazi(&self, metin: &str, font: HFONT, renk: COLORREF, mut r: RECT, bayrak: u32) -> i32 {
        // Boş dizgide Vec işaretçisi sarkık olur (0x2); DrawTextW'ye hiç verilmez.
        if metin.is_empty() {
            return 0;
        }
        let w: Vec<u16> = metin.encode_utf16().collect();
        // SAFETY: geçerli DC, font ve tampon.
        unsafe {
            let ef = SelectObject(self.dc, font);
            SetTextColor(self.dc, renk);
            let h = DrawTextW(self.dc, w.as_ptr(), w.len() as i32, &mut r, bayrak | DT_NOPREFIX);
            SelectObject(self.dc, ef);
            h
        }
    }

    /// Sarılan metnin yüksekliği.
    fn olc(&self, metin: &str, font: HFONT, genislik: i32) -> i32 {
        // Boş dizgide Vec işaretçisi sarkık olur (0x2); DrawTextW'ye hiç verilmez.
        if metin.is_empty() {
            return 0;
        }
        let w: Vec<u16> = metin.encode_utf16().collect();
        let mut r = RECT { left: 0, top: 0, right: genislik, bottom: 0 };
        // SAFETY: geçerli DC, font ve tampon; DT_CALCRECT çizmez.
        unsafe {
            let ef = SelectObject(self.dc, font);
            DrawTextW(self.dc, w.as_ptr(), w.len() as i32, &mut r, DT_CALCRECT | DT_WORDBREAK | DT_EDITCONTROL | DT_NOPREFIX);
            SelectObject(self.dc, ef);
        }
        r.bottom - r.top
    }

    /// Tek satırlık metnin genişliği.
    fn en(&self, metin: &str, font: HFONT) -> i32 {
        // Boş dizgide Vec işaretçisi sarkık olur (0x2); DrawTextW'ye hiç verilmez.
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
}

fn dr(x: i32, y: i32, w: i32, h: i32) -> RECT {
    RECT { left: x, top: y, right: x + w, bottom: y + h }
}

const TEK: u32 = DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS;
const SARILI: u32 = DT_WORDBREAK | DT_EDITCONTROL;

unsafe fn boya(hwnd: HWND, d: &mut PencereDurumu) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    let mut alan = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    GetClientRect(hwnd, &mut alan);
    let (w, h) = (alan.right.max(1), alan.bottom.max(1));
    let mdc = CreateCompatibleDC(hdc);
    let bmp = CreateCompatibleBitmap(hdc, w, h);
    let eski = SelectObject(mdc, bmp);
    SetBkMode(mdc, TRANSPARENT as i32);
    if let Some(p) = kopya(d) {
        ciz(&Cizer { dc: mdc, dpi: d.dpi.max(96) as i32 }, d, &p, w, h);
    }
    BitBlt(hdc, 0, 0, w, h, mdc, 0, 0, SRCCOPY);
    SelectObject(mdc, eski);
    DeleteObject(bmp);
    DeleteDC(mdc);
    EndPaint(hwnd, &ps);
}

fn ciz(c: &Cizer, d: &mut PencereDurumu, p: &Paylasim, w: i32, h: i32) {
    let r = d.renk;
    let f = d.fontlar.0;
    d.hedefler.clear();
    c.doldur(dr(0, 0, w, h), r.zemin);
    let m = c.px(36);
    let i = &p.ilerleme;

    // Başlık ve sürüm.
    c.yazi("TAA Kur", f.baslik, r.metin, dr(m, c.px(24), c.px(300), c.px(36)), TEK);
    c.yazi(&format!("Kurulum · sürüm {} · {}", i.surum, p.hedef), f.kucuk, r.soluk, dr(w - m - c.px(460), c.px(24), c.px(460), c.px(36)), TEK | DT_RIGHT);

    // Aşama şeridi.
    let mut x = m;
    let y = c.px(76);
    let simdi = i.asama.sira();
    for (n, a) in Asama::HEPSI.iter().enumerate() {
        let (nokta, yazi_rengi) = if n < simdi || (n == simdi && i.asama == Asama::Bitti) {
            (r.vurgu, r.metin)
        } else if n == simdi {
            (r.vurgu, r.vurgu)
        } else {
            (r.kenar, r.soluk)
        };
        c.daire(x + c.px(4), y, c.px(4), nokta, nokta, 1);
        let et = a.etiket();
        let font = if n == simdi { f.kalin } else { f.govde };
        let en = c.en(et, font);
        c.yazi(et, font, yazi_rengi, dr(x + c.px(14), y - c.px(10), en + c.px(4), c.px(20)), TEK);
        x += c.px(14) + en + c.px(28);
    }

    // Genel ilerleme.
    let sag_w = c.px(if w < c.px(960) { 320 } else { 360 });
    let bosluk = c.px(32);
    let sol_w = w - 2 * m - sag_w - bosluk;
    let yuzde = i.yuzde() as i32;
    let ust = c.px(104);
    c.yazi("Genel ilerleme", f.kucuk, r.soluk, dr(m, ust, c.px(200), c.px(18)), TEK);
    c.yazi(&format!("%{yuzde}"), f.kalin, r.metin, dr(m + sol_w - c.px(80), ust, c.px(80), c.px(18)), TEK | DT_RIGHT);
    let cubuk = dr(m, ust + c.px(24), sol_w, c.px(6));
    c.kutu(cubuk, c.px(3), r.kenar, None);
    if yuzde > 0 {
        let dolu = (sol_w * yuzde / 100).max(c.px(6));
        let renk = if i.hatalilar().is_empty() { r.vurgu } else { r.hata };
        c.kutu(dr(m, ust + c.px(24), dolu, c.px(6)), c.px(3), renk, None);
    }
    c.yazi(i.son_mesaj(), f.kucuk, r.soluk, dr(m, ust + c.px(38), sol_w, c.px(18)), TEK);

    // Gövde.
    let govde_ust = c.px(168);
    let alt_cubuk = h - c.px(72);
    let sol = dr(m, govde_ust, sol_w, alt_cubuk - govde_ust - c.px(16));
    d.kart_alani = dr(0, 0, 0, 0);
    if let Some(hata) = &p.hata {
        hata_ciz(c, d, hata, sol);
    } else if let Some(s) = &p.secim {
        // Seçim sırasında kullanıcı bekleme yapmıyor: kart gösterilmez, sol alan seçimindir.
        secim_ciz(c, d, s, sol);
    } else {
        // Bekleme: program adımları üstte, dönen kart sol alanın altında (bitince kart kalkar, sonuç tam görünür).
        let kart_ust = if i.asama == Asama::Bitti { sol.bottom + c.px(24) } else { kart_ciz(c, d, sol) };
        programlar_ciz(c, d, p, RECT { bottom: kart_ust - c.px(24), ..sol });
    }
    // Sağ sütun: adım adım ayar direktifleri (tam yükseklik).
    let sag_x = m + sol_w + bosluk;
    direktifler_ciz(c, d, &p.direktifler, dr(sag_x, govde_ust, sag_w, alt_cubuk - govde_ust - c.px(16)));

    // Alt çubuk: ipucu ve düğmeler.
    c.doldur(dr(0, alt_cubuk, w, 1), r.kenar);
    let (ipucu, dugmeler): (&str, Vec<(&str, Hedef, bool)>) = if p.hata.is_some() {
        ("Enter: Bitir", vec![("Bitir", Hedef::Bitir, true)])
    } else if let Some(s) = &p.secim {
        let ip = if s.coklu { "Yukarı/Aşağı ok: gez · Boşluk: işaretle · Enter: onayla · Esc: vazgeç" } else if s.secenekler.is_empty() { "Enter: onayla · Esc: vazgeç" } else { "Yukarı/Aşağı ok: seç · Enter: onayla · Esc: vazgeç" };
        (ip, vec![(s.vazgec.as_str(), Hedef::Vazgec, false), (s.onay.as_str(), Hedef::Onay, true)])
    } else if i.komut_bekleniyor && !i.hatalilar().is_empty() {
        ("Enter: Yeniden dene · Esc: Bitir", vec![("Bitir", Hedef::Bitir, false), ("Yeniden dene", Hedef::YenidenDene, true)])
    } else if i.komut_bekleniyor {
        ("Enter: Bitir", vec![("TAA Kur'u aç", Hedef::TaaKurAc, false), ("Bitir", Hedef::Bitir, true)])
    } else {
        ("Kurulum sürüyor · Sol/Sağ ok: kartlar", vec![])
    };
    c.yazi(ipucu, f.kucuk, r.soluk, dr(m, alt_cubuk, w / 2, h - alt_cubuk), TEK);
    let mut bx = w - m;
    let by = alt_cubuk + (h - alt_cubuk - c.px(36)) / 2;
    for (et, hedef, birincil) in dugmeler.iter().rev() {
        let bw = c.en(et, f.kalin) + c.px(40);
        bx -= bw;
        let rr = dr(bx, by, bw, c.px(36));
        let ust_ = d.uzerinde == Some(*hedef);
        if *birincil {
            let renk = if ust_ { karis(r.vurgu, r.metin, 40) } else { r.vurgu };
            c.kutu(rr, c.px(8), renk, None);
            c.yazi(et, f.kalin, r.vurgu_ustu, rr, TEK | DT_CENTER);
        } else {
            let dolgu = if ust_ { karis(r.zemin, r.kenar, 160) } else { r.zemin };
            c.kutu(rr, c.px(8), dolgu, Some((r.kenar, 1)));
            c.yazi(et, f.kalin, r.metin, rr, TEK | DT_CENTER);
        }
        d.hedefler.push((rr, *hedef));
        bx -= c.px(12);
    }
}

fn hata_ciz(c: &Cizer, d: &PencereDurumu, hata: &str, a: RECT) {
    let (r, f) = (d.renk, d.fontlar.0);
    let pad = c.px(20);
    let iw = a.right - a.left - 2 * pad;
    let hh = c.olc(hata, f.govde, iw);
    let kutu = dr(a.left, a.top, a.right - a.left, hh + c.px(68));
    c.kutu(kutu, c.px(10), karis(r.yuzey, r.hata, 24), Some((karis(r.kenar, r.hata, 120), 1)));
    c.yazi("Kurulum tamamlanamadı", f.alt, r.hata, dr(a.left + pad, a.top + pad, iw, c.px(24)), TEK);
    c.yazi(hata, f.govde, r.metin, dr(a.left + pad, a.top + pad + c.px(32), iw, hh), SARILI);
}

fn secim_ciz(c: &Cizer, d: &mut PencereDurumu, s: &Secim, a: RECT) {
    let (r, f) = (d.renk, d.fontlar.0);
    let gen = a.right - a.left;
    let mut y = a.top;
    c.yazi(&s.baslik, f.alt, r.metin, dr(a.left, y, gen, c.px(26)), TEK);
    y += c.px(34);
    if !s.aciklama.is_empty() {
        let hh = c.olc(&s.aciklama, f.govde, gen);
        c.yazi(&s.aciklama, f.govde, r.soluk, dr(a.left, y, gen, hh), SARILI);
        y += hh + c.px(18);
    }
    let n = s.secenekler.len();
    if n == 0 {
        return;
    }
    // Çok seçenekte (ör. 15 şube) ızgara: 7-12 seçenek iki, daha çoğu üç sütun; hücrede yalnız etiket.
    let iki = n > 6 && s.secenekler.iter().all(|x| x.grup.is_empty());
    let sut: usize = if !iki { 1 } else if n > 12 { 3 } else { 2 };
    let ara = c.px(10);
    let kw = (gen - ara * (sut as i32 - 1)) / sut as i32;
    let kh = c.px(if iki { 40 } else if s.secenekler.iter().any(|x| !x.aciklama.is_empty()) { 58 } else { 44 });
    let grup_h = c.px(26);
    // Görünür pencere: imleç hep görünsün.
    let satirlar = n.div_ceil(sut);
    let sigan = (((a.bottom - y) + ara) / (kh + ara)).max(1) as usize;
    let imlec_satir = s.imlec / sut;
    let bas = if satirlar <= sigan { 0 } else { imlec_satir.saturating_sub(sigan.saturating_sub(1)).min(satirlar - sigan) };
    let mut onceki_grup = "";
    for (k, x) in s.secenekler.iter().enumerate() {
        let satir = k / sut;
        if satir < bas {
            onceki_grup = &x.grup;
            continue;
        }
        if !iki && x.grup != onceki_grup && !x.grup.is_empty() {
            if y + grup_h > a.bottom {
                break;
            }
            c.yazi(&x.grup.to_uppercase(), f.etiket, r.soluk, dr(a.left, y, gen, grup_h), TEK);
            y += grup_h;
        }
        onceki_grup = &x.grup;
        let (kx, ky) = (a.left + (k % sut) as i32 * (kw + ara), y);
        if ky + kh > a.bottom {
            c.yazi("…", f.govde, r.soluk, dr(a.left, ky, gen, c.px(20)), TEK);
            break;
        }
        let rr = dr(kx, ky, kw, kh);
        let secili = k == s.imlec;
        let ust_ = d.uzerinde == Some(Hedef::Secenek(k));
        let dolgu = if secili { karis(r.yuzey, r.vurgu, 22) } else if ust_ { karis(r.yuzey, r.kenar, 90) } else { r.yuzey };
        let kenar = if secili { (r.vurgu, c.px(2)) } else { (r.kenar, 1) };
        c.kutu(rr, c.px(8), dolgu, Some(kenar));
        // Gösterge: tek seçimde daire, çoklu seçimde kare.
        let gx = kx + c.px(22);
        let gy = ky + kh / 2;
        let gr = c.px(8);
        if s.coklu {
            let isaretli = s.isaretli.get(k).copied().unwrap_or(false);
            let kr = dr(gx - gr, gy - gr, 2 * gr, 2 * gr);
            if isaretli {
                c.kutu(kr, c.px(3), r.vurgu, None);
                c.tik(gx, gy, gr, r.vurgu_ustu);
            } else {
                c.kutu(kr, c.px(3), r.yuzey, Some((r.soluk, 1)));
            }
        } else if secili {
            c.daire(gx, gy, gr, r.yuzey, r.vurgu, c.px(2));
            c.daire(gx, gy, gr / 2, r.vurgu, r.vurgu, 1);
        } else {
            c.daire(gx, gy, gr, r.yuzey, r.soluk, 1);
        }
        let tx = gx + gr + c.px(14);
        let tw = kx + kw - tx - c.px(12);
        if !iki && !x.aciklama.is_empty() {
            c.yazi(&x.etiket, f.kalin, r.metin, dr(tx, ky + c.px(9), tw, c.px(20)), TEK);
            c.yazi(&x.aciklama, f.kucuk, r.soluk, dr(tx, ky + c.px(30), tw, c.px(18)), TEK);
        } else {
            c.yazi(&x.etiket, f.govde, r.metin, dr(tx, ky, tw, kh), TEK);
        }
        d.hedefler.push((rr, Hedef::Secenek(k)));
        if k % sut == sut - 1 || k + 1 == n {
            y += kh + ara;
        }
    }
}

fn programlar_ciz(c: &Cizer, d: &PencereDurumu, p: &Paylasim, a: RECT) {
    let (r, f) = (d.renk, d.fontlar.0);
    let i = &p.ilerleme;
    let gen = a.right - a.left;
    let mut y = a.top;
    if i.programlar.is_empty() {
        let m = match i.asama {
            Asama::Hazirlik => "Kurulum başlamak üzere.".to_string(),
            Asama::Cekirdek => format!("TAA Kur dosyaları yazılıyor ({}/{}).", i.cekirdek_yazilan, i.cekirdek_toplam),
            Asama::Bitti => "Program kurulmadı. Programları sonra TAA Kur penceresinden kurabilirsiniz.".into(),
            _ => "Seçilen programlar burada adım adım görünür: İndir, Doğrula, Kur, Ayarla.".into(),
        };
        let hh = c.olc(&m, f.govde, gen);
        c.yazi(&m, f.govde, r.soluk, dr(a.left, y, gen, hh), SARILI);
        return;
    }
    let adim_w = c.px(if gen < c.px(560) { 76 } else { 86 });
    let adimlar_w = adim_w * 4;
    let ad_w = gen - adimlar_w - c.px(12);
    let satir_h = |pd: &crate::ilerleme::ProgramDurumu| if pd.hata.is_some() { c.px(62) } else { c.px(42) };
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
        if y + toplam <= a.bottom || bas >= odak {
            break;
        }
        bas += 1;
    }
    let nabiz = {
        let t = (d.baslangic.elapsed().as_millis() % 1400) as f32 / 1400.0;
        (((t * std::f32::consts::TAU).sin() + 1.0) * 0.5 * 160.0) as u32
    };
    for (k, pd) in i.programlar.iter().enumerate().skip(bas) {
        let sh = satir_h(pd);
        if y + sh > a.bottom {
            let kalan = i.programlar.len() - k;
            c.yazi(&format!("+{kalan} program daha"), f.kucuk, r.soluk, dr(a.left, y, gen, c.px(20)), TEK);
            break;
        }
        // Ad ve (aynı satırda) durum etiketi.
        let ad_font = f.kalin;
        let aen = c.en(&pd.ad, ad_font).min(ad_w);
        c.yazi(&pd.ad, ad_font, r.metin, dr(a.left, y + c.px(10), aen, c.px(22)), TEK);
        let mut etiketler = vec![];
        if pd.bagimlilik {
            etiketler.push("bağımlılık".to_string());
        }
        if pd.deneme > 1 {
            etiketler.push(format!("{}. deneme", pd.deneme));
        }
        let durum_metni = match pd.durum() {
            Durum::Bekliyor => "sırada",
            Durum::Suruyor => "sürüyor",
            Durum::Tamam => "kuruldu",
            Durum::Hata => "kurulamadı",
        };
        etiketler.insert(0, durum_metni.to_string());
        let ex = a.left + aen + c.px(10);
        c.yazi(&etiketler.join(" · "), f.kucuk, if pd.durum() == Durum::Hata { r.hata } else { r.soluk },
            dr(ex, y + c.px(11), (a.left + ad_w - ex).max(0), c.px(20)), TEK);
        // Adımlar.
        for (j, adim) in Adim::HEPSI.iter().enumerate() {
            let ax = a.left + ad_w + c.px(12) + j as i32 * adim_w;
            let cy = y + c.px(21);
            let rr = c.px(7);
            let dur = pd.adimlar[j];
            let (yazi_r, font) = match dur {
                Durum::Bekliyor => {
                    c.daire(ax + rr, cy, rr, r.zemin, r.kenar, c.px(2));
                    (r.soluk, f.kucuk)
                }
                Durum::Suruyor => {
                    let ic = karis(r.zemin, r.vurgu, 60 + nabiz);
                    c.daire(ax + rr, cy, rr, ic, r.vurgu, c.px(2));
                    (r.vurgu, f.etiket)
                }
                Durum::Tamam => {
                    c.daire(ax + rr, cy, rr, r.vurgu, r.vurgu, 1);
                    c.tik(ax + rr, cy, rr, r.vurgu_ustu);
                    (r.metin, f.kucuk)
                }
                Durum::Hata => {
                    c.daire(ax + rr, cy, rr, r.hata, r.hata, 1);
                    c.carpi(ax + rr, cy, rr, r.vurgu_ustu);
                    (r.hata, f.etiket)
                }
            };
            c.yazi(adim.etiket(), font, yazi_r, dr(ax + 2 * rr + c.px(6), cy - c.px(10), adim_w - 2 * rr - c.px(6), c.px(20)), TEK);
        }
        if let Some(h) = &pd.hata {
            c.yazi(h, f.kucuk, r.hata, dr(a.left, y + c.px(36), gen, c.px(18)), TEK);
        }
        y += sh;
        if k + 1 < i.programlar.len() {
            c.doldur(dr(a.left, y - 1, gen, 1), r.kenar);
        }
    }
}

/// Bekleme kartı `a` alanının altına yaslanır; dönen: kartın üst kenarı.
fn kart_ciz(c: &Cizer, d: &mut PencereDurumu, alan: RECT) -> i32 {
    let (r, f) = (d.renk, d.fontlar.0);
    let gen = alan.right - alan.left;
    let Some(k) = d.kartlar.get(d.kart % d.kartlar.len().max(1)).cloned() else {
        return alan.bottom;
    };
    let pad = c.px(20);
    let iw = gen - 2 * pad;
    let ozet_h = c.olc(&k.ozet, f.govde, iw);
    let not_h = if k.not.is_empty() { 0 } else { c.olc(&k.not, f.kucuk, iw) };
    let sab_h = if k.sablon.is_empty() { 0 } else { c.olc(&k.sablon, f.kucuk, iw - c.px(24)) };
    let mut hh = pad + c.px(18) + c.px(8) + c.px(26) + ozet_h + pad;
    if not_h > 0 {
        hh += c.px(14) + c.px(16) + not_h;
    }
    if sab_h > 0 {
        hh += c.px(14) + c.px(16) + sab_h + c.px(20);
    }
    let a = RECT { top: alan.bottom - hh, ..alan };
    let kutu = dr(a.left, a.top, gen, hh);
    d.kart_alani = kutu;
    c.kutu(kutu, c.px(12), r.yuzey, Some((r.kenar, 1)));
    let x = a.left + pad;
    let mut y = a.top + pad;
    c.yazi(kartlar::tur_etiketi(&k.tur), f.etiket, r.vurgu, dr(x, y, iw, c.px(18)), TEK);
    // Sayaç ve oklar.
    let n = d.kartlar.len();
    let ok_w = c.px(26);
    let geri = dr(a.right - pad - 2 * ok_w, y - c.px(4), ok_w, c.px(26));
    let ileri = dr(a.right - pad - ok_w, y - c.px(4), ok_w, c.px(26));
    for (rr, hedef, isaret) in [(geri, Hedef::KartGeri, "‹"), (ileri, Hedef::KartIleri, "›")] {
        if d.uzerinde == Some(hedef) {
            c.kutu(rr, c.px(6), karis(r.yuzey, r.kenar, 160), None);
        }
        c.yazi(isaret, f.alt, r.soluk, rr, TEK | DT_CENTER);
        d.hedefler.push((rr, hedef));
    }
    c.yazi(&format!("{} / {n}", d.kart % n + 1), f.kucuk, r.soluk, dr(geri.left - c.px(70), y, c.px(64), c.px(18)), TEK | DT_RIGHT);
    y += c.px(26);
    c.yazi(&k.ad, f.alt, r.metin, dr(x, y, iw, c.px(26)), TEK);
    y += c.px(30);
    c.yazi(&k.ozet, f.govde, r.metin, dr(x, y, iw, ozet_h), SARILI);
    y += ozet_h;
    if not_h > 0 {
        y += c.px(14);
        c.yazi("Not", f.etiket, r.soluk, dr(x, y, iw, c.px(16)), TEK);
        y += c.px(16);
        c.yazi(&k.not, f.kucuk, r.soluk, dr(x, y, iw, not_h), SARILI);
        y += not_h;
    }
    if sab_h > 0 {
        y += c.px(14);
        c.yazi("Örnek şablon", f.etiket, r.soluk, dr(x, y, iw, c.px(16)), TEK);
        y += c.px(18);
        let sk = dr(x, y, iw, sab_h + c.px(18));
        c.kutu(sk, c.px(6), karis(r.yuzey, r.zemin, 200), Some((r.kenar, 1)));
        c.yazi(&k.sablon, f.kucuk, r.metin, dr(x + c.px(12), y + c.px(9), iw - c.px(24), sab_h), SARILI);
    }
    kutu.top
}

fn direktifler_ciz(c: &Cizer, d: &PencereDurumu, v: &[Direktif], a: RECT) {
    let (r, f) = (d.renk, d.fontlar.0);
    if v.is_empty() || a.bottom - a.top < c.px(60) {
        return;
    }
    let gen = a.right - a.left;
    c.yazi("Kurulumdan sonra", f.kalin, r.metin, dr(a.left, a.top, gen, c.px(20)), TEK);
    c.yazi("Bu PC'de yapılacak ayarlar; yapılanlar işaretli.", f.kucuk, r.soluk, dr(a.left, a.top + c.px(20), gen, c.px(18)), TEK);
    let mut y = a.top + c.px(48);
    let rr = c.px(10);
    let tx = a.left + 2 * rr + c.px(12);
    let tw = a.right - tx;
    for (n, x) in v.iter().enumerate() {
        let ah = c.olc(&x.ayrinti, f.kucuk, tw);
        let sh = c.px(20) + ah;
        if y + c.px(20) > a.bottom {
            c.yazi("…", f.kucuk, r.soluk, dr(tx, y, tw, c.px(16)), TEK);
            break;
        }
        let cy = y + c.px(10);
        let cx = a.left + rr;
        match x.durum {
            DDurum::Yapildi => {
                c.daire(cx, cy, rr, r.vurgu, r.vurgu, 1);
                c.tik(cx, cy, rr, r.vurgu_ustu);
            }
            DDurum::Yapilacak => {
                c.daire(cx, cy, rr, r.zemin, r.vurgu, c.px(2));
                c.yazi(&(n + 1).to_string(), f.etiket, r.vurgu, dr(cx - rr, cy - rr, 2 * rr + 1, 2 * rr + 1), TEK | DT_CENTER);
            }
            DDurum::Bilgi => {
                c.daire(cx, cy, rr, r.zemin, r.kenar, c.px(2));
                c.yazi("i", f.etiket, r.soluk, dr(cx - rr, cy - rr, 2 * rr + 1, 2 * rr + 1), TEK | DT_CENTER);
            }
        }
        let bas_r = if x.durum == DDurum::Yapildi { r.soluk } else { r.metin };
        c.yazi(&x.baslik, f.kalin, bas_r, dr(tx, y, tw, c.px(20)), TEK | DT_LEFT);
        let kalan = a.bottom - (y + c.px(20));
        c.yazi(&x.ayrinti, f.kucuk, r.soluk, dr(tx, y + c.px(20), tw, ah.min(kalan.max(0))), SARILI | DT_END_ELLIPSIS);
        y += sh + c.px(12);
    }
}
