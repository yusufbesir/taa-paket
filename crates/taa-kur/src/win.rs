// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05] değiştiren: [Claude#kurucu-arayuz 2026-11-17] (üç tema, Ayarlar, Geçme Temel logosu)
//! Win32 çizim katmanı (yalnız Windows): ince katman, mantık `pencere_model` ve `pencere_tema` içindedir.
//! Görünüm KARAR-FABLE'a göre üç tema (İlmek varsayılan, Kalıp, Mezura), açık/koyu/sistem modu, özel vurgu.
//! Düzen `pencere_tema::duzen` ile hesaplanır (etiketler GDI ile ölçülür, kesilmez); pencere GDI ile bellek
//! DC'sine çizilip tek BitBlt ile basılır. Düğmeler sahip-çizimli BUTTON (klavye, Tab, ekran okuyucu adı korunur),
//! liste gerçek ListView: satırlar ve başlık NM_CUSTOMDRAW ile tema renginde çizilir (LVS_EX_DOUBLEBUFFER).
//! Hareketler 120 ms tikli, 2-3 adımlı; "hareketi azalt" açıkken anlık. Kur / Kaldır / Güncelle / Rapor
//! davranışı ve onay soruları değişmedi. İşlemler arayüz iş parçacığında eşzamanlı çalışır.
//! DİKKAT: Windows'ta elle doğrulanmadı; Linux'ta yalnız tür denetimi (check/clippy) yapıldı.

use crate::pencere_model::{
    GuncelleyiciArayuzu, KaynaksizGuncelleyici, KomutGuncelleyici, PencereModel, SUTUNLAR,
};
use crate::pencere_tema::{
    dolgu_harmanla, durum_turu, duzen, gecis_adim_sayisi, gecis_karesi, hucre_kipi, odak_jetonu,
    olcekle, secili_bicim, serit_genisligi, sutun_genislikleri, vurgu_sec, yazi, yerlesim, Dik,
    DugmeTuru, Duzen, DuzenGirdisi, Ekran, GecisKaresi, Gorunus, Jeton, MetinKipi, Oge, Olcer,
    SatirAyirici, SistemDurumu, TabloKenari, YaziRolu, EN_AZ_BOYUT, HEDEF_BOYUT, PENCERE_BASLIGI,
    SON_KARE, TANITIM_SIFIRLANDI, VARSAYILAN_ONAY,
};
use crate::Kur;
use std::ffi::{c_void, OsString};
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use taa_kur_tema::{
    ara_renk, colorref, colorref_rgb, logo, Gorunum, Hareket, Jetonlar, Tema, Yazi,
};
use windows_sys::Win32::Foundation::{
    GlobalFree, ERROR_SUCCESS, HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM,
};
use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreatePen,
    CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, Ellipse, EndPaint, FillRect, GetDC,
    GetStockObject, GetTextExtentPoint32W, GetTextFaceW, GetTextMetricsW, InvalidateRect, Polygon,
    Polyline, RedrawWindow, ReleaseDC, RoundRect, SelectObject, SetBkMode, SetTextColor,
    CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET, DEFAULT_PITCH, DT_CALCRECT, DT_CENTER,
    DT_EDITCONTROL, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_PATH_ELLIPSIS, DT_SINGLELINE,
    DT_VCENTER, DT_WORDBREAK, HDC, HFONT, NULL_PEN, OUT_DEFAULT_PRECIS, PAINTSTRUCT, PS_SOLID,
    RDW_ALLCHILDREN, RDW_INVALIDATE, SRCCOPY, TEXTMETRICW, TRANSPARENT,
};
use windows_sys::Win32::System::DataExchange::{
    CloseClipboard, EmptyClipboard, OpenClipboard, SetClipboardData,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows_sys::Win32::System::Ole::CF_UNICODETEXT;
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows_sys::Win32::UI::Controls::Dialogs::{
    ChooseColorW, GetOpenFileNameW, CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW, OFN_FILEMUSTEXIST,
    OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows_sys::Win32::UI::Controls::{
    ImageList_Create, ImageList_Destroy, InitCommonControlsEx, SetWindowTheme, CDDS_ITEMPREPAINT,
    CDDS_PREPAINT, CDRF_DODEFAULT, CDRF_NOTIFYITEMDRAW, CDRF_SKIPDEFAULT, DRAWITEMSTRUCT,
    HIMAGELIST, ICC_LISTVIEW_CLASSES, ICC_STANDARD_CLASSES, ILC_COLOR32, INITCOMMONCONTROLSEX,
    LVCF_SUBITEM, LVCF_TEXT, LVCF_WIDTH, LVCOLUMNW, LVIF_TEXT, LVIR_BOUNDS, LVIS_SELECTED, LVITEMW,
    LVM_DELETEALLITEMS, LVM_GETCOLUMNWIDTH, LVM_GETHEADER, LVM_GETITEMRECT, LVM_GETITEMSTATE,
    LVM_GETNEXTITEM, LVM_INSERTCOLUMNW, LVM_INSERTITEMW, LVM_SETBKCOLOR, LVM_SETCOLUMNWIDTH,
    LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETIMAGELIST, LVM_SETITEMTEXTW, LVM_SETTEXTBKCOLOR,
    LVM_SETTEXTCOLOR, LVNI_SELECTED, LVN_ITEMCHANGED, LVSIL_SMALL, LVS_EX_DOUBLEBUFFER,
    LVS_EX_FULLROWSELECT, LVS_NOSORTHEADER, LVS_REPORT, LVS_SHAREIMAGELISTS, LVS_SHOWSELALWAYS,
    LVS_SINGLESEL, NMCUSTOMDRAW, NMHDR, NMLVCUSTOMDRAW, NM_CUSTOMDRAW, ODS_DISABLED, ODS_FOCUS,
    ODS_SELECTED, WC_LISTVIEWW,
};
use windows_sys::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{EnableWindow, GetFocus, SetFocus};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AdjustWindowRectEx, CallWindowProcW, CreateWindowExW, DefWindowProcW, DispatchMessageW,
    GetClientRect, GetMessageW, GetParent, GetSystemMetrics, GetWindowLongPtrW, IsDialogMessageW,
    IsWindowVisible, KillTimer, LoadCursorW, MessageBoxW, MoveWindow, PostMessageW,
    PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SendMessageW, SetProcessDPIAware,
    SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, SystemParametersInfoW, TranslateMessage,
    BN_CLICKED, BS_OWNERDRAW, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, GWLP_WNDPROC,
    HWND_BROADCAST, IDC_ARROW, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION, MB_OK,
    MB_YESNO, MINMAXINFO, MSG, SM_CXVSCROLL, SPI_GETCLIENTAREAANIMATION, SPI_GETHIGHCONTRAST,
    SWP_NOACTIVATE, SWP_NOZORDER, SW_HIDE, SW_SHOW, WM_COMMAND, WM_CREATE, WM_DESTROY,
    WM_DPICHANGED, WM_DRAWITEM, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_NCCREATE, WM_NOTIFY, WM_PAINT,
    WM_SETFONT, WM_SETTINGCHANGE, WM_SIZE, WM_SYSCOLORCHANGE, WM_THEMECHANGED, WM_TIMER, WNDCLASSW,
    WS_CHILD, WS_CLIPCHILDREN, WS_OVERLAPPEDWINDOW, WS_TABSTOP, WS_VISIBLE,
};

const ZAMANLAYICI: usize = 1;
/// Görünüm tercihi değişince açık pencereler arasında yayınlanan kayıtlı ileti.
const GORUNUM_ILETISI: &str = "TAA_KUR_GORUNUM_DEGISTI";

/// Liste yenilenirken gelen bildirimler (silme/ekleme) yok sayılır.
static YENILIYOR: AtomicBool = AtomicBool::new(false);
/// ListView'ın özgün pencere işlevi (başlık özel çizimi için alt sınıflama).
static LISTE_ESKI_PROC: AtomicIsize = AtomicIsize::new(0);

enum Eylem {
    Kur(PathBuf),
    Kaldir,
    Guncelle,
    Rapor,
}

/// Tema yazı rolleri için fontlar (Drop'ta silinir).
struct Fontlar {
    baslik: HFONT,
    bolum: HFONT,
    govde: HFONT,
    govde_kalin: HFONT,
    kucuk: HFONT,
    dugme: HFONT,
}

impl Fontlar {
    fn yap(t: Tema, dpi: u32) -> Fontlar {
        let ty = t.tipografi();
        let baslik_aile = if aile_var(ty.baslik_aile) {
            ty.baslik_aile
        } else {
            ty.aile
        };
        let f = |aile: &str, y: Yazi| font_yap(aile, y, dpi);
        Fontlar {
            baslik: f(baslik_aile, yazi(t, YaziRolu::Baslik)),
            bolum: f(baslik_aile, yazi(t, YaziRolu::Bolum)),
            govde: f(ty.aile, yazi(t, YaziRolu::Govde)),
            govde_kalin: f(ty.aile, yazi(t, YaziRolu::GovdeKalin)),
            kucuk: f(ty.aile, yazi(t, YaziRolu::Kucuk)),
            dugme: f(ty.aile, yazi(t, YaziRolu::Dugme)),
        }
    }

    fn rol(&self, r: YaziRolu) -> HFONT {
        match r {
            YaziRolu::Baslik => self.baslik,
            YaziRolu::Bolum => self.bolum,
            YaziRolu::Govde => self.govde,
            YaziRolu::GovdeKalin => self.govde_kalin,
            YaziRolu::Kucuk => self.kucuk,
            YaziRolu::Dugme => self.dugme,
        }
    }
}

impl Drop for Fontlar {
    fn drop(&mut self) {
        for f in [
            self.baslik,
            self.bolum,
            self.govde,
            self.govde_kalin,
            self.kucuk,
            self.dugme,
        ] {
            // SAFETY: CreateFontW ile yaratılan, artık seçili olmayan tutamak.
            unsafe { DeleteObject(f) };
        }
    }
}

fn font_yap(aile: &str, y: Yazi, dpi: u32) -> HFONT {
    let ad = genis(aile);
    // SAFETY: ad sıfır sonlu; dönen tutamak Fontlar::drop'ta silinir.
    unsafe {
        CreateFontW(
            -olcekle(y.px, dpi),
            0,
            0,
            0,
            y.agirlik,
            0,
            0,
            0,
            DEFAULT_CHARSET as u32,
            OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,
            CLEARTYPE_QUALITY as u32,
            DEFAULT_PITCH as u32,
            ad.as_ptr(),
        )
    }
}

/// Yazı ailesi kurulu mu? (GDI bilinmeyen adı sessizce başka aileyle değiştirir; seçilen yüz adı karşılaştırılır.)
fn aile_var(aile: &str) -> bool {
    let f = font_yap(
        aile,
        Yazi {
            px: 12,
            agirlik: 400,
        },
        96,
    );
    let mut ad = [0u16; 64];
    // SAFETY: ekran DC'si alınıp bırakılır; font seçilip geri alınır ve silinir.
    let n = unsafe {
        let dc = GetDC(null_mut());
        let eski = SelectObject(dc, f);
        let n = GetTextFaceW(dc, ad.len() as i32, ad.as_mut_ptr());
        SelectObject(dc, eski);
        ReleaseDC(null_mut(), dc);
        DeleteObject(f);
        n
    };
    let uz = ad.iter().position(|&c| c == 0).unwrap_or(n.max(0) as usize);
    String::from_utf16_lossy(&ad[..uz]).eq_ignore_ascii_case(aile)
}

/// GDI ölçer: `pencere_tema::duzen` etiketleri bununla ölçer.
struct GdiOlcer<'a> {
    dc: HDC,
    f: &'a Fontlar,
    tema: Tema,
    dpi: u32,
}

impl Olcer for GdiOlcer<'_> {
    fn genislik(&self, metin: &str, rol: YaziRolu) -> i32 {
        let w: Vec<u16> = metin.encode_utf16().collect();
        let mut s = SIZE { cx: 0, cy: 0 };
        // SAFETY: dc geçerli; font seçilip geri alınır; w çağrı süresince yaşar.
        unsafe {
            let eski = SelectObject(self.dc, self.f.rol(rol));
            GetTextExtentPoint32W(self.dc, w.as_ptr(), w.len() as i32, &mut s);
            SelectObject(self.dc, eski);
        }
        s.cx
    }

    fn satir(&self, rol: YaziRolu) -> i32 {
        // SAFETY: dc geçerli; TEXTMETRICW yazılabilir.
        let h = unsafe {
            let eski = SelectObject(self.dc, self.f.rol(rol));
            let mut tm: TEXTMETRICW = std::mem::zeroed();
            GetTextMetricsW(self.dc, &mut tm);
            SelectObject(self.dc, eski);
            tm.tmHeight + tm.tmExternalLeading
        };
        if rol == YaziRolu::Govde {
            h.max(olcekle(self.tema.tipografi().govde_satir, self.dpi))
        } else {
            h
        }
    }
}

struct Durum {
    model: PencereModel,
    gorunum: Gorunum,
    sistem: SistemDurumu,
    gs: Gorunus,
    /// O karede çizilen renkler (İlmek geçişinde harman).
    renk: Jetonlar,
    kare: GecisKaresi,
    /// Süren geçiş: (hareket, adım no, eski renkler).
    gecis: Option<(Hareket, u8, Jetonlar)>,
    ekran: Ekran,
    dpi: u32,
    fontlar: Option<Fontlar>,
    imaj: HIMAGELIST,
    pencere: HWND,
    liste: HWND,
    baslik: HWND,
    dugmeler: Vec<(Oge, HWND)>,
    duzen: Duzen,
    vurgu_notu: Option<(String, Jeton)>,
    ozel_renkler: [u32; 16],
    gorunum_iletisi: u32,
}

impl Durum {
    fn s(&self, v: i32) -> i32 {
        olcekle(v, self.dpi)
    }

    fn dugme(&self, oge: Oge) -> HWND {
        self.dugmeler
            .iter()
            .find(|(o, _)| *o == oge)
            .map(|(_, h)| *h)
            .unwrap_or(null_mut())
    }

    fn cr(&self, j: Jeton) -> u32 {
        colorref(j.renk(&self.renk))
    }
}

fn genis(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn mesaj_kutusu(sahip: HWND, metin: &str, baslik: &str, bayrak: u32) -> i32 {
    let m = genis(metin);
    let b = genis(baslik);
    // SAFETY: m ve b sıfır sonlu, çağrı süresince yaşıyor.
    unsafe { MessageBoxW(sahip, m.as_ptr(), b.as_ptr(), bayrak) }
}

/// Sistem uygulama teması koyu mu (HKCU ...\Themes\Personalize\AppsUseLightTheme = 0).
fn sistem_koyu() -> bool {
    let anahtar = genis(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize");
    let deger = genis("AppsUseLightTheme");
    let mut v: u32 = 1;
    let mut boy: u32 = 4;
    // SAFETY: sıfır sonlu dizgiler ve yazılabilir u32; RRF_RT_REG_DWORD 4 bayt yazar.
    let s = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            anahtar.as_ptr(),
            deger.as_ptr(),
            RRF_RT_REG_DWORD,
            null_mut(),
            &mut v as *mut u32 as *mut c_void,
            &mut boy,
        )
    };
    s == ERROR_SUCCESS && v == 0
}

fn sistem_oku() -> SistemDurumu {
    let mut yk: HIGHCONTRASTW = unsafe { std::mem::zeroed() };
    yk.cbSize = std::mem::size_of::<HIGHCONTRASTW>() as u32;
    let mut anim: i32 = 1;
    // SAFETY: yapılar yazılabilir ve doğru boyutta.
    let (yk_ok, anim_ok) = unsafe {
        (
            SystemParametersInfoW(
                SPI_GETHIGHCONTRAST,
                yk.cbSize,
                &mut yk as *mut _ as *mut c_void,
                0,
            ),
            SystemParametersInfoW(
                SPI_GETCLIENTAREAANIMATION,
                0,
                &mut anim as *mut i32 as *mut c_void,
                0,
            ),
        )
    };
    SistemDurumu {
        koyu: sistem_koyu(),
        yuksek_karsitlik: yk_ok != 0 && (yk.dwFlags & HCF_HIGHCONTRASTON) != 0,
        animasyon_acik: anim_ok == 0 || anim != 0,
    }
}

/// Giriş noktası: pencereyi açar, ileti döngüsünü çalıştırır; çıkış kodu döner.
pub fn calistir() -> i32 {
    let kur = match Kur::ortamdan() {
        Ok(k) => k,
        Err(h) => {
            mesaj_kutusu(null_mut(), h.mesaj(), "TAA Kurucu", MB_ICONERROR);
            return h.cikis_kodu();
        }
    };
    let guncelleyici: Box<dyn GuncelleyiciArayuzu> = match KomutGuncelleyici::bul(&kur) {
        Some(g) => Box::new(g),
        None => Box::new(KaynaksizGuncelleyici),
    };
    // Kurucu ile aynı tercih dosyası (%APPDATA%\TAA\veri\kurucu\gorunum.json); yoksa varsayılanlar.
    let gorunum = Gorunum::yukle();
    let sistem = sistem_oku();
    let gs = Gorunus::hesapla(&gorunum, sistem);
    let ileti_adi = genis(GORUNUM_ILETISI);
    // SAFETY: DPI farkındalığı pencere açılmadan önce bildirilir; ileti adı sıfır sonlu.
    let (gorunum_iletisi, sistem_dpi) = unsafe {
        SetProcessDPIAware();
        (
            RegisterWindowMessageW(ileti_adi.as_ptr()),
            GetDpiForSystem().max(96),
        )
    };
    let durum = Box::new(Durum {
        model: PencereModel::yeni(kur, guncelleyici),
        gorunum,
        sistem,
        gs,
        renk: gs.j,
        kare: SON_KARE,
        gecis: None,
        ekran: Ekran::Paketler,
        dpi: sistem_dpi,
        fontlar: None,
        imaj: 0,
        pencere: null_mut(),
        liste: null_mut(),
        baslik: null_mut(),
        dugmeler: Vec::new(),
        duzen: Duzen::default(),
        vurgu_notu: None,
        ozel_renkler: [0x00FF_FFFF; 16],
        gorunum_iletisi,
    });
    let durum_ptr = Box::into_raw(durum);
    // SAFETY: Win32 çağrıları; durum_ptr pencere yıkılana kadar geçerli, WM_DESTROY'da geri alınır.
    unsafe {
        let ic = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_LISTVIEW_CLASSES | ICC_STANDARD_CLASSES,
        };
        InitCommonControlsEx(&ic);
        let ornek = GetModuleHandleW(null());
        let sinif = genis("TaaKurPencere");
        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(pencere_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: ornek,
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            // Arka plan WM_PAINT'te tema rengiyle çizilir (titreme olmasın diye fırça yok).
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: sinif.as_ptr(),
        };
        if RegisterClassW(&wc) == 0 {
            drop(Box::from_raw(durum_ptr));
            mesaj_kutusu(
                null_mut(),
                "Pencere sınıfı kaydedilemedi.",
                "TAA Kurucu",
                MB_ICONERROR,
            );
            return 1;
        }
        let baslik = genis(PENCERE_BASLIGI);
        let mut r = RECT {
            left: 0,
            top: 0,
            right: olcekle(HEDEF_BOYUT.0, sistem_dpi),
            bottom: olcekle(HEDEF_BOYUT.1, sistem_dpi),
        };
        AdjustWindowRectEx(&mut r, WS_OVERLAPPEDWINDOW, 0, 0);
        let pencere = CreateWindowExW(
            0,
            sinif.as_ptr(),
            baslik.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            r.right - r.left,
            r.bottom - r.top,
            null_mut(),
            null_mut(),
            ornek,
            durum_ptr as *const _,
        );
        if pencere.is_null() {
            drop(Box::from_raw(durum_ptr));
            mesaj_kutusu(null_mut(), "Pencere açılamadı.", "TAA Kurucu", MB_ICONERROR);
            return 1;
        }
        ShowWindow(pencere, SW_SHOW);
        let mut ileti: MSG = std::mem::zeroed();
        while GetMessageW(&mut ileti, null_mut(), 0, 0) > 0 {
            // Tab / ok tuşlarıyla düğmeler arasında gezinme.
            if IsDialogMessageW(pencere, &ileti) == 0 {
                TranslateMessage(&ileti);
                DispatchMessageW(&ileti);
            }
        }
        // WM_DESTROY içinde durum_ptr geri alınmıştır.
        ileti.wParam as i32
    }
}

unsafe fn durum_al<'a>(h: HWND) -> Option<&'a mut Durum> {
    let p = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Durum;
    if p.is_null() {
        None
    } else {
        Some(&mut *p)
    }
}

unsafe fn cocuk(
    sinif: *const u16,
    metin: &str,
    stil: u32,
    kimlik: isize,
    ust: HWND,
    ornek: windows_sys::Win32::Foundation::HMODULE,
) -> HWND {
    let m = genis(metin);
    CreateWindowExW(
        0,
        sinif,
        m.as_ptr(),
        WS_CHILD | stil,
        0,
        0,
        10,
        10,
        ust,
        kimlik as _,
        ornek,
        null(),
    )
}

unsafe extern "system" fn pencere_proc(h: HWND, ileti: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    match ileti {
        WM_NCCREATE => {
            let cs = &*(l as *const CREATESTRUCTW);
            SetWindowLongPtrW(h, GWLP_USERDATA, cs.lpCreateParams as isize);
            DefWindowProcW(h, ileti, w, l)
        }
        WM_CREATE => {
            if let Some(d) = durum_al(h) {
                olustur(h, d);
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            if let Some(d) = durum_al(h) {
                boya(h, d);
            } else {
                return DefWindowProcW(h, ileti, w, l);
            }
            0
        }
        WM_SIZE => {
            if let Some(d) = durum_al(h) {
                yerlestir(d);
            }
            0
        }
        WM_GETMINMAXINFO => {
            if let Some(d) = durum_al(h) {
                let mut r = RECT {
                    left: 0,
                    top: 0,
                    right: d.s(EN_AZ_BOYUT.0),
                    bottom: d.s(EN_AZ_BOYUT.1),
                };
                AdjustWindowRectEx(&mut r, WS_OVERLAPPEDWINDOW, 0, 0);
                let mm = &mut *(l as *mut MINMAXINFO);
                mm.ptMinTrackSize = POINT {
                    x: r.right - r.left,
                    y: r.bottom - r.top,
                };
            }
            0
        }
        WM_DPICHANGED => {
            if let Some(d) = durum_al(h) {
                d.dpi = (((w >> 16) & 0xFFFF) as u32).max(96);
                let r = &*(l as *const RECT);
                SetWindowPos(
                    h,
                    null_mut(),
                    r.left,
                    r.top,
                    r.right - r.left,
                    r.bottom - r.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
                gorunus_uygula(d, false);
            }
            0
        }
        WM_SETTINGCHANGE | WM_THEMECHANGED | WM_SYSCOLORCHANGE => {
            // Sistemi izle modu, Yüksek Karşıtlık ve "animasyonları göster" yeniden değerlendirilir.
            if let Some(d) = durum_al(h) {
                let yeni = sistem_oku();
                if yeni != d.sistem {
                    d.sistem = yeni;
                    gorunus_uygula(d, false);
                }
            }
            DefWindowProcW(h, ileti, w, l)
        }
        WM_TIMER => {
            if let Some(d) = durum_al(h) {
                gecis_ilerlet(d);
            }
            0
        }
        WM_DRAWITEM => {
            if let Some(d) = durum_al(h) {
                dugme_ciz(d, &*(l as *const DRAWITEMSTRUCT));
            }
            1
        }
        WM_NOTIFY => {
            let nm = &*(l as *const NMHDR);
            if let Some(d) = durum_al(h) {
                if nm.hwndFrom == d.liste && nm.code == NM_CUSTOMDRAW {
                    return satir_ozel_cizim(d, &mut *(l as *mut NMLVCUSTOMDRAW));
                }
                if YENILIYOR.load(Ordering::Relaxed) {
                    return 0;
                }
                if nm.hwndFrom == d.liste && nm.code == LVN_ITEMCHANGED {
                    let sec =
                        SendMessageW(d.liste, LVM_GETNEXTITEM, usize::MAX, LVNI_SELECTED as isize);
                    d.model
                        .sec_satir(if sec >= 0 { Some(sec as usize) } else { None });
                    dugmeleri_guncelle(d);
                }
            }
            0
        }
        WM_COMMAND => {
            let kimlik = (w & 0xFFFF) as isize;
            let bildirim = ((w >> 16) & 0xFFFF) as u32;
            if bildirim == BN_CLICKED {
                if let Some(oge) = Oge::kimlikten(kimlik) {
                    komut(h, oge);
                }
            }
            0
        }
        WM_DESTROY => {
            KillTimer(h, ZAMANLAYICI);
            let p = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Durum;
            SetWindowLongPtrW(h, GWLP_USERDATA, 0);
            if !p.is_null() {
                let d = Box::from_raw(p);
                if d.imaj != 0 {
                    ImageList_Destroy(d.imaj);
                }
                drop(d);
            }
            PostQuitMessage(0);
            0
        }
        _ => {
            if let Some(d) = durum_al(h) {
                if d.gorunum_iletisi != 0 && ileti == d.gorunum_iletisi {
                    // Başka bir TAA penceresi görünüm tercihini değiştirdi (kendi yayınımız değilse yeniden oku).
                    if w as HWND != h {
                        d.gorunum = Gorunum::yukle();
                        gorunus_uygula(d, false);
                    }
                    return 0;
                }
            }
            DefWindowProcW(h, ileti, w, l)
        }
    }
}

unsafe fn olustur(h: HWND, d: &mut Durum) {
    d.pencere = h;
    d.dpi = GetDpiForWindow(h).max(96);
    let ornek = GetModuleHandleW(null());
    d.liste = cocuk(
        WC_LISTVIEWW,
        "",
        // LVS_SHAREIMAGELISTS: görüntü listesinin sahibi biziz (WM_DESTROY'da ve yeniden kurulumda yıkılır);
        // ListView kendi yıkılışında onu bir daha yok etmesin.
        WS_TABSTOP
            | WS_VISIBLE
            | LVS_REPORT
            | LVS_SINGLESEL
            | LVS_SHOWSELALWAYS
            | LVS_NOSORTHEADER
            | LVS_SHAREIMAGELISTS,
        0,
        h,
        ornek,
    );
    SendMessageW(
        d.liste,
        LVM_SETEXTENDEDLISTVIEWSTYLE,
        0,
        (LVS_EX_FULLROWSELECT | LVS_EX_DOUBLEBUFFER) as isize,
    );
    for (i, b) in SUTUNLAR.iter().enumerate() {
        let mut yazi = genis(b);
        let mut s: LVCOLUMNW = std::mem::zeroed();
        s.mask = LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM;
        s.cx = d.s([180, 150, 130, 260][i]);
        s.pszText = yazi.as_mut_ptr();
        s.iSubItem = i as i32;
        SendMessageW(d.liste, LVM_INSERTCOLUMNW, i, &s as *const _ as isize);
    }
    d.baslik = SendMessageW(d.liste, LVM_GETHEADER, 0, 0) as HWND;
    // Başlık özel çizimi başlığın üstüne (ListView'a) gider: ListView alt sınıflanır.
    let eski = SetWindowLongPtrW(d.liste, GWLP_WNDPROC, liste_proc as *const () as isize);
    LISTE_ESKI_PROC.store(eski, Ordering::Relaxed);
    let btn = genis("BUTTON");
    for oge in Oge::HEPSI {
        let hb = cocuk(
            btn.as_ptr(),
            &oge.etiket(Ekran::Paketler),
            WS_TABSTOP | BS_OWNERDRAW as u32,
            oge.kimlik(),
            h,
            ornek,
        );
        d.dugmeler.push((oge, hb));
    }
    liste_doldur(d);
    dugmeleri_guncelle(d);
    gorunus_uygula(d, false);
    SetFocus(d.liste);
}

/// Tercih + sistem durumundan görünüşü yeniden hesaplar ve uygular; `animasyon` ise tema hareketiyle geçer.
unsafe fn gorunus_uygula(d: &mut Durum, animasyon: bool) {
    let eski = d.renk;
    d.gs = Gorunus::hesapla(&d.gorunum, d.sistem);
    let adim = gecis_adim_sayisi(d.gs.tema.hareket(), d.gs.hareketi_azalt);
    if animasyon && adim > 0 {
        d.gecis = Some((d.gs.tema.hareket(), 1, eski));
        SetTimer(d.pencere, ZAMANLAYICI, taa_kur_tema::TIK_MS, None);
    } else {
        d.gecis = None;
        KillTimer(d.pencere, ZAMANLAYICI);
    }
    d.fontlar = Some(Fontlar::yap(d.gs.tema, d.dpi));
    if let Some(f) = &d.fontlar {
        // Başlık satırı kalın yazıyla ölçülür; satırlar özel çizimde kendi fontlarıyla çizilir.
        SendMessageW(d.liste, WM_SETFONT, f.govde_kalin as usize, 0);
    }
    // Satır yüksekliği: 1 px genişliğinde görüntü listesiyle (ListView başka yol tanımaz).
    let yeni = ImageList_Create(1, d.s(d.gs.tema.geometri().satir), ILC_COLOR32, 1, 1);
    SendMessageW(d.liste, LVM_SETIMAGELIST, LVSIL_SMALL as usize, yeni);
    if d.imaj != 0 {
        ImageList_Destroy(d.imaj);
    }
    d.imaj = yeni;
    // Koyu modda kaydırma çubukları ve başlık çubuğu da koyu.
    let tema_adi = genis(if d.gs.koyu {
        "DarkMode_Explorer"
    } else {
        "Explorer"
    });
    SetWindowTheme(d.liste, tema_adi.as_ptr(), null());
    let koyu: i32 = d.gs.koyu as i32;
    DwmSetWindowAttribute(
        d.pencere,
        DWMWA_USE_IMMERSIVE_DARK_MODE as u32,
        &koyu as *const i32 as *const c_void,
        4,
    );
    kare_uygula(d);
}

/// Geçiş karesini (renk, kayma, çizgi) hesaplar, yerleştirir ve her şeyi yeniden çizdirir.
unsafe fn kare_uygula(d: &mut Durum) {
    match d.gecis {
        Some((h, n, eski)) => {
            d.kare = gecis_karesi(h, n);
            d.renk = if h == Hareket::RenkGecisi {
                dolgu_harmanla(&eski, &d.gs.j, d.kare.renk_orani)
            } else {
                d.gs.j
            };
        }
        None => {
            d.kare = SON_KARE;
            d.renk = d.gs.j;
        }
    }
    SendMessageW(d.liste, LVM_SETBKCOLOR, 0, d.cr(Jeton::Yuzey) as isize);
    SendMessageW(d.liste, LVM_SETTEXTBKCOLOR, 0, d.cr(Jeton::Yuzey) as isize);
    SendMessageW(d.liste, LVM_SETTEXTCOLOR, 0, d.cr(Jeton::Metin) as isize);
    yerlestir(d);
}

unsafe fn gecis_ilerlet(d: &mut Durum) {
    if let Some((h, n, eski)) = d.gecis {
        if n >= h.adim() {
            d.gecis = None;
            KillTimer(d.pencere, ZAMANLAYICI);
        } else {
            d.gecis = Some((h, n + 1, eski));
        }
    } else {
        KillTimer(d.pencere, ZAMANLAYICI);
    }
    kare_uygula(d);
}

/// Ekran değişimi (Paketler <-> Ayarlar): tema hareketiyle (Kalıp oturma, Mezura alt çizgi).
unsafe fn ekran_degistir(d: &mut Durum, ekran: Ekran) {
    if d.ekran == ekran {
        return;
    }
    d.ekran = ekran;
    for (oge, hb) in &d.dugmeler {
        let m = genis(&oge.etiket(ekran));
        windows_sys::Win32::UI::WindowsAndMessaging::SetWindowTextW(*hb, m.as_ptr());
    }
    gorunus_uygula(d, true);
    // Odak görünmeyen bir denetimde kalmasın.
    let odak = GetFocus();
    if odak.is_null() || IsWindowVisible(odak) == 0 {
        let hedef = match ekran {
            Ekran::Paketler => d.liste,
            Ekran::Ayarlar => d.dugme(Oge::Tema(d.gs.tema)),
        };
        SetFocus(hedef);
    }
}

/// Düzeni hesaplar, denetimleri yerleştirir, gösterir/gizler ve yeniden çizdirir.
unsafe fn yerlestir(d: &mut Durum) {
    let Some(f) = &d.fontlar else {
        return;
    };
    let mut r: RECT = std::mem::zeroed();
    GetClientRect(d.pencere, &mut r);
    let (en, boy) = (r.right - r.left, r.bottom - r.top);
    if en <= 0 || boy <= 0 {
        return;
    }
    let dc = GetDC(d.pencere);
    let o = GdiOlcer {
        dc,
        f,
        tema: d.gs.tema,
        dpi: d.dpi,
    };
    let durum = d.model.durum().to_string();
    let kok = d.model.kok_yolu().display().to_string();
    let gr = DuzenGirdisi {
        tema: d.gs.tema,
        dpi: d.dpi,
        en,
        boy,
        ekran: d.ekran,
        durum: &durum,
        kok: &kok,
        vurgu_notu: d.vurgu_notu.as_ref().map(|(m, j)| (m.as_str(), *j)),
        yk_notu: d.gs.yk_ile_mezura,
        kare: d.kare,
    };
    let yeni = duzen(&gr, &o);
    let sutunlar = yeni.liste.map(|l| {
        sutun_genislikleri(
            d.gs.tema,
            d.dpi,
            l.en - GetSystemMetrics(SM_CXVSCROLL) - d.s(2),
            &o,
        )
    });
    ReleaseDC(d.pencere, dc);
    for (oge, hb) in &d.dugmeler {
        match yeni.dugme(*oge) {
            Some(yd) => {
                MoveWindow(*hb, yd.alan.x, yd.alan.y, yd.alan.en, yd.alan.boy, 0);
                ShowWindow(*hb, SW_SHOW);
            }
            None => {
                ShowWindow(*hb, SW_HIDE);
            }
        }
    }
    match yeni.liste {
        Some(l) => {
            MoveWindow(d.liste, l.x, l.y, l.en, l.boy, 0);
            ShowWindow(d.liste, SW_SHOW);
            if let Some(g) = sutunlar {
                for (i, w) in g.iter().enumerate() {
                    SendMessageW(d.liste, LVM_SETCOLUMNWIDTH, i, *w as isize);
                }
            }
        }
        None => {
            ShowWindow(d.liste, SW_HIDE);
        }
    }
    d.duzen = yeni;
    RedrawWindow(
        d.pencere,
        null(),
        null_mut(),
        RDW_INVALIDATE | RDW_ALLCHILDREN,
    );
}

fn dik_rect(d: Dik) -> RECT {
    RECT {
        left: d.x,
        top: d.y,
        right: d.x + d.en,
        bottom: d.y + d.boy,
    }
}

/// Dolu (isteğe bağlı yuvarlak köşeli) dikdörtgen.
unsafe fn dolgu(dc: HDC, r: RECT, renk: u32, yaricap: i32) {
    let firca = CreateSolidBrush(renk);
    if yaricap <= 0 {
        FillRect(dc, &r, firca);
    } else {
        let eski_f = SelectObject(dc, firca);
        let eski_k = SelectObject(dc, GetStockObject(NULL_PEN));
        // NULL_PEN ile RoundRect sağ/alt kenarı bir piksel içeride doldurur: +1.
        RoundRect(
            dc,
            r.left,
            r.top,
            r.right + 1,
            r.bottom + 1,
            2 * yaricap,
            2 * yaricap,
        );
        SelectObject(dc, eski_k);
        SelectObject(dc, eski_f);
    }
    DeleteObject(firca);
}

fn iceri(r: RECT, k: i32) -> RECT {
    RECT {
        left: r.left + k,
        top: r.top + k,
        right: r.right - k,
        bottom: r.bottom - k,
    }
}

/// Çerçeveli kutu: dış kenar rengiyle, içi dolguyla (köşeler yarıçapla).
unsafe fn cerceveli(dc: HDC, r: RECT, kenar: u32, k: i32, ic: Option<u32>, yaricap: i32) {
    dolgu(dc, r, kenar, yaricap);
    if let Some(c) = ic {
        dolgu(dc, iceri(r, k), c, (yaricap - k).max(0));
    }
}

/// Çerçeve halkası (içi boş): odak göstergesi.
unsafe fn halka(dc: HDC, r: RECT, renk: u32, k: i32) {
    let firca = CreateSolidBrush(renk);
    for p in [
        RECT {
            bottom: r.top + k,
            ..r
        },
        RECT {
            top: r.bottom - k,
            ..r
        },
        RECT {
            right: r.left + k,
            ..r
        },
        RECT {
            left: r.right - k,
            ..r
        },
    ] {
        FillRect(dc, &p, firca);
    }
    DeleteObject(firca);
}

/// Yatay çizgi: düz ya da teyel (8 px çizgi + 6 px boşluk; faz `x0`'dan sabit, titremez).
unsafe fn yatay_cizgi(
    dc: HDC,
    x0: i32,
    x1: i32,
    y: i32,
    k: i32,
    renk: u32,
    teyel: Option<(i32, i32)>,
) {
    let firca = CreateSolidBrush(renk);
    match teyel {
        None => {
            let r = RECT {
                left: x0,
                top: y,
                right: x1,
                bottom: y + k,
            };
            FillRect(dc, &r, firca);
        }
        Some((cizgi, bosluk)) => {
            let mut x = x0;
            while x < x1 {
                let r = RECT {
                    left: x,
                    top: y,
                    right: (x + cizgi).min(x1),
                    bottom: y + k,
                };
                FillRect(dc, &r, firca);
                x += cizgi + bosluk;
            }
        }
    }
    DeleteObject(firca);
}

unsafe fn metin_yaz(dc: HDC, metin: &str, r: RECT, font: HFONT, renk: u32, bayrak: u32) {
    // Boş metinde Vec::as_ptr() sarkık işaretçi olur; Win32'ye hiç verme.
    if metin.is_empty() {
        return;
    }
    let w: Vec<u16> = metin.encode_utf16().collect();
    let mut r = r;
    let eski = SelectObject(dc, font);
    SetTextColor(dc, renk);
    SetBkMode(dc, TRANSPARENT as i32);
    DrawTextW(dc, w.as_ptr(), w.len() as i32, &mut r, bayrak | DT_NOPREFIX);
    SelectObject(dc, eski);
}

/// Sarılı metni dikeyde ortalar (önce DT_CALCRECT ile ölçer).
unsafe fn sarili_ortali(dc: HDC, metin: &str, r: RECT, font: HFONT, renk: u32, yatay: u32) {
    // Boş metinde Vec::as_ptr() sarkık işaretçi olur; Win32'ye hiç verme.
    if metin.is_empty() {
        return;
    }
    let w: Vec<u16> = metin.encode_utf16().collect();
    let mut olc = r;
    let eski = SelectObject(dc, font);
    DrawTextW(
        dc,
        w.as_ptr(),
        w.len() as i32,
        &mut olc,
        DT_CALCRECT | DT_WORDBREAK | DT_EDITCONTROL | DT_NOPREFIX | yatay,
    );
    SelectObject(dc, eski);
    let h = (olc.bottom - olc.top).min(r.bottom - r.top);
    let top = r.top + ((r.bottom - r.top) - h) / 2;
    metin_yaz(
        dc,
        metin,
        RECT {
            top,
            bottom: top + h,
            ..r
        },
        font,
        renk,
        DT_WORDBREAK | DT_EDITCONTROL | yatay,
    );
}

fn kip_bayragi(k: MetinKipi) -> u32 {
    match k {
        MetinKipi::TekKisalt => DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS | DT_LEFT,
        MetinKipi::Tek => DT_SINGLELINE | DT_VCENTER | DT_LEFT,
        MetinKipi::Sarili => DT_WORDBREAK | DT_EDITCONTROL | DT_LEFT,
        MetinKipi::YolKisalt => DT_SINGLELINE | DT_VCENTER | DT_PATH_ELLIPSIS | DT_LEFT,
    }
}

/// Geçme Temel logosu: ana parça `metin`, vurgu parçası `vurgu` (Mezura açıkta `kenar` çerçeveli).
unsafe fn logo_ciz(dc: HDC, alan: Dik, j: &Jetonlar, cerceveli: bool) {
    for (parca, noktalar) in logo::cokgenler(alan.en, alan.x, alan.y) {
        let (dolgu_rengi, kalem) = match parca {
            logo::Parca::Ana => (j.metin, None),
            logo::Parca::Vurgu => (j.vurgu, cerceveli.then_some(j.kenar)),
        };
        let pts: Vec<POINT> = noktalar.iter().map(|&(x, y)| POINT { x, y }).collect();
        let firca = CreateSolidBrush(colorref(dolgu_rengi));
        let eski_f = SelectObject(dc, firca);
        let k = kalem.map(|c| CreatePen(PS_SOLID, 1, colorref(c)));
        let eski_k = match k {
            Some(p) => SelectObject(dc, p),
            None => SelectObject(dc, GetStockObject(NULL_PEN)),
        };
        Polygon(dc, pts.as_ptr(), pts.len() as i32);
        SelectObject(dc, eski_k);
        SelectObject(dc, eski_f);
        if let Some(p) = k {
            DeleteObject(p);
        }
        DeleteObject(firca);
    }
}

/// Pencere arka planı, başlık, bantlar, tablo çerçevesi, metinler (çift tampon, tek BitBlt).
unsafe fn boya(h: HWND, d: &mut Durum) {
    let mut ps: PAINTSTRUCT = std::mem::zeroed();
    let hdc = BeginPaint(h, &mut ps);
    let mut alan: RECT = std::mem::zeroed();
    GetClientRect(h, &mut alan);
    let (w, hh) = (alan.right.max(1), alan.bottom.max(1));
    let mdc = CreateCompatibleDC(hdc);
    let bmp = CreateCompatibleBitmap(hdc, w, hh);
    let eski = SelectObject(mdc, bmp);
    SetBkMode(mdc, TRANSPARENT as i32);
    dolgu(mdc, alan, d.cr(Jeton::Zemin), 0);
    for k in &d.duzen.kutular {
        let r = dik_rect(k.alan);
        match (k.kenar, k.dolgu) {
            (Some((kj, kk)), ic) => cerceveli(mdc, r, d.cr(kj), kk, ic.map(|j| d.cr(j)), k.yaricap),
            (None, Some(j)) => dolgu(mdc, r, d.cr(j), k.yaricap),
            (None, None) => {}
        }
    }
    logo_ciz(
        mdc,
        d.duzen.logo,
        &d.renk,
        d.gs.tema.vurgu_cerceveli(d.gs.koyu),
    );
    if let Some(f) = &d.fontlar {
        for m in &d.duzen.metinler {
            metin_yaz(
                mdc,
                &m.metin,
                dik_rect(m.alan),
                f.rol(m.rol),
                d.cr(m.jeton),
                kip_bayragi(m.kip),
            );
        }
    }
    if let Some(o) = d.duzen.renk_ornegi {
        // Vurgu örneği: her temada kenar çerçeveli (Mezura açıkta zorunlu, diğerlerinde okunurluk için).
        let k = d.s(d.gs.tema.geometri().kenar.max(1));
        cerceveli(
            mdc,
            dik_rect(o),
            d.cr(Jeton::Kenar),
            k,
            Some(d.cr(Jeton::Vurgu)),
            d.s(d.gs.tema.geometri().etiket_yaricap),
        );
    }
    BitBlt(hdc, 0, 0, w, hh, mdc, 0, 0, SRCCOPY);
    SelectObject(mdc, eski);
    DeleteObject(bmp);
    DeleteDC(mdc);
    EndPaint(h, &ps);
}

/// Sahip-çizimli düğme (bellek DC'sinde çizilip basılır).
unsafe fn dugme_ciz(d: &Durum, di: &DRAWITEMSTRUCT) {
    let Some(oge) = Oge::kimlikten(di.CtlID as isize) else {
        return;
    };
    let Some(f) = &d.fontlar else {
        return;
    };
    let yd = d.duzen.dugme(oge);
    let satir = yd.map(|y| y.satir).unwrap_or(1);
    let arka = yd.map(|y| y.arka).unwrap_or(Jeton::Zemin);
    let r0 = di.rcItem;
    let (w, h) = ((r0.right - r0.left).max(1), (r0.bottom - r0.top).max(1));
    let mdc = CreateCompatibleDC(di.hDC);
    let bmp = CreateCompatibleBitmap(di.hDC, w, h);
    let eski = SelectObject(mdc, bmp);
    let r = RECT {
        left: 0,
        top: 0,
        right: w,
        bottom: h,
    };
    let t = d.gs.tema;
    let koyu = d.gs.koyu;
    let geo = t.geometri();
    let j = &d.renk;
    let yc = d.s(geo.dugme_yaricap);
    let kenar = d.s(geo.kenar).max(1);
    let basili = di.itemState & ODS_SELECTED != 0;
    let pasif = di.itemState & ODS_DISABLED != 0;
    let odakta = di.itemState & ODS_FOCUS != 0;
    let secili = oge.secili(&d.gorunum, &d.gs, d.ekran);
    let cerceveli_vurgu = t.vurgu_cerceveli(koyu);
    let cr = |rgb: u32| colorref(rgb);
    dolgu(mdc, r, cr(arka.renk(j)), 0);
    let etiket = oge.etiket(d.ekran);
    let mut metin_rengi = if pasif { j.soluk } else { j.metin };
    let mut metin_alani = iceri(r, d.s(4));
    let tur = oge.tur();
    match tur {
        DugmeTuru::Birincil => {
            if pasif {
                dolgu(mdc, r, cr(j.yuzey2), yc);
            } else {
                let renk = if basili {
                    ara_renk(j.vurgu, j.metin, 0.15)
                } else {
                    j.vurgu
                };
                if cerceveli_vurgu {
                    cerceveli(mdc, r, cr(j.kenar), d.s(2), Some(cr(renk)), yc);
                } else {
                    dolgu(mdc, r, cr(renk), yc);
                }
                metin_rengi = j.vurgu_ustu;
            }
        }
        DugmeTuru::Duz | DugmeTuru::Radyo | DugmeTuru::Onay => {
            let ic = if basili { j.yuzey2 } else { j.yuzey };
            if pasif {
                dolgu(mdc, r, cr(ic), yc);
            } else {
                cerceveli(mdc, r, cr(j.kenar), kenar, Some(cr(ic)), yc);
            }
            if matches!(tur, DugmeTuru::Radyo | DugmeTuru::Onay) {
                let g = d.s(14);
                let x = d.s(geo.dugme_yatay_ic);
                let y = (h - g) / 2;
                let kutu = RECT {
                    left: x,
                    top: y,
                    right: x + g,
                    bottom: y + g,
                };
                let isaret = if cerceveli_vurgu { j.metin } else { j.vurgu };
                if tur == DugmeTuru::Radyo {
                    daire(mdc, kutu, cr(j.kenar), cr(ic), d.s(2));
                    if secili {
                        daire(mdc, iceri(kutu, d.s(4)), cr(isaret), cr(isaret), 0);
                    }
                } else if secili {
                    cerceveli(
                        mdc,
                        kutu,
                        cr(j.kenar),
                        d.s(2),
                        Some(cr(j.vurgu)),
                        d.s(2).min(yc),
                    );
                    tik(mdc, kutu, cr(j.vurgu_ustu), d.s(2));
                } else {
                    cerceveli(mdc, kutu, cr(j.kenar), d.s(2), Some(cr(ic)), d.s(2).min(yc));
                }
                metin_alani.left = x + g + d.s(10);
                metin_alani.right = w - d.s(geo.dugme_yatay_ic) + d.s(4);
            }
        }
        DugmeTuru::Sekme => {
            if basili {
                dolgu(mdc, r, cr(j.yuzey2), yc);
            }
            if secili {
                let cizgi = if cerceveli_vurgu { j.kenar } else { j.vurgu };
                dolgu(
                    mdc,
                    RECT {
                        top: h - d.s(3),
                        ..r
                    },
                    cr(cizgi),
                    0,
                );
            } else {
                metin_rengi = j.soluk;
            }
        }
        DugmeTuru::TemaSecimi => {
            if let Oge::Tema(onizlenen) = oge {
                let (kc, kk) = if secili {
                    (if cerceveli_vurgu { j.kenar } else { j.vurgu }, d.s(2))
                } else {
                    (j.kenar, kenar)
                };
                let ic = if basili { j.yuzey2 } else { j.yuzey };
                cerceveli(mdc, r, cr(kc), kk, Some(cr(ic)), d.s(geo.kart_yaricap));
                let p = d.s(8);
                let etiket_boy = (h - p * 3).min(d.s(28));
                let on = RECT {
                    left: p,
                    top: p,
                    right: w - p,
                    bottom: h - p * 2 - etiket_boy + p,
                };
                onizleme_ciz(d, mdc, on, onizlenen, koyu);
                metin_alani = RECT {
                    left: p,
                    top: on.bottom,
                    right: w - p,
                    bottom: h - p / 2,
                };
            }
        }
    }
    let font = f.dugme;
    let yazilacak = if tur == DugmeTuru::TemaSecimi && secili {
        format!("{etiket} ✓")
    } else {
        etiket
    };
    let yatay = if matches!(tur, DugmeTuru::Radyo | DugmeTuru::Onay) {
        DT_LEFT
    } else {
        DT_CENTER
    };
    if satir > 1 {
        sarili_ortali(mdc, &yazilacak, metin_alani, font, cr(metin_rengi), yatay);
    } else {
        metin_yaz(
            mdc,
            &yazilacak,
            metin_alani,
            font,
            cr(metin_rengi),
            DT_SINGLELINE | DT_VCENTER | yatay,
        );
    }
    if odakta {
        // Klavye odağı her temada görünür (2 px); Mezura'da boşluklu çift çerçeve.
        let renk = cr(odak_jetonu(t, koyu).renk(j));
        let k = d.s(geo.odak);
        halka(mdc, r, renk, k);
        if geo.odak_bosluk > 0 {
            halka(mdc, iceri(r, k + d.s(geo.odak_bosluk)), renk, k);
        }
    }
    BitBlt(di.hDC, r0.left, r0.top, w, h, mdc, 0, 0, SRCCOPY);
    SelectObject(mdc, eski);
    DeleteObject(bmp);
    DeleteDC(mdc);
}

unsafe fn daire(dc: HDC, r: RECT, kenar: u32, ic: u32, k: i32) {
    let firca = CreateSolidBrush(ic);
    let kalem = if k > 0 {
        CreatePen(PS_SOLID, k, kenar)
    } else {
        CreatePen(PS_SOLID, 1, ic)
    };
    let ef = SelectObject(dc, firca);
    let ek = SelectObject(dc, kalem);
    Ellipse(dc, r.left, r.top, r.right, r.bottom);
    SelectObject(dc, ek);
    SelectObject(dc, ef);
    DeleteObject(kalem);
    DeleteObject(firca);
}

/// Onay işareti (Polyline).
unsafe fn tik(dc: HDC, r: RECT, renk: u32, k: i32) {
    let (w, h) = (r.right - r.left, r.bottom - r.top);
    let pts = [
        POINT {
            x: r.left + w * 2 / 10,
            y: r.top + h / 2,
        },
        POINT {
            x: r.left + w * 4 / 10,
            y: r.top + h * 7 / 10,
        },
        POINT {
            x: r.left + w * 8 / 10,
            y: r.top + h * 3 / 10,
        },
    ];
    let kalem = CreatePen(PS_SOLID, k.max(1), renk);
    let ek = SelectObject(dc, kalem);
    Polyline(dc, pts.as_ptr(), pts.len() as i32);
    SelectObject(dc, ek);
    DeleteObject(kalem);
}

/// Tema önizlemesi (gerçek çizim işlevleriyle, bitmap yok): zemin, logo, kart, satırlar, birincil düğme, imza.
unsafe fn onizleme_ciz(d: &Durum, dc: HDC, r: RECT, t: Tema, koyu: bool) {
    let g = Gorunum {
        tema: Some(t),
        modu: if koyu {
            taa_kur_tema::Mod::Koyu
        } else {
            taa_kur_tema::Mod::Acik
        },
        ..d.gorunum.clone()
    };
    let (_, j) = taa_kur_tema::etkin_jetonlar(&g, koyu, false);
    let geo = t.geometri();
    let cr = colorref;
    // Önizleme ölçüleri 160 px genişliğe göre tasarlandı; sıkışık ölçüde orantılı küçülür.
    let w = r.right - r.left;
    let u = |v: i32| v * w.max(1) / 160;
    dolgu(dc, r, cr(j.zemin), d.s(geo.etiket_yaricap));
    let lg = u(14).max(6);
    logo_ciz(
        dc,
        Dik::yeni(r.left + u(8), r.top + u(6), lg, lg),
        &j,
        t.vurgu_cerceveli(koyu),
    );
    // Başlık çizgisi.
    dolgu(
        dc,
        RECT {
            left: r.left + u(8) + lg + u(6),
            top: r.top + u(6) + lg / 2 - u(2),
            right: r.left + u(80),
            bottom: r.top + u(6) + lg / 2 + u(2),
        },
        cr(j.metin),
        0,
    );
    // Birincil düğme.
    let dg = RECT {
        left: r.right - u(44),
        top: r.top + u(4),
        right: r.right - u(8),
        bottom: r.top + u(4) + lg + u(4),
    };
    if t.vurgu_cerceveli(koyu) {
        cerceveli(dc, dg, cr(j.kenar), 2, Some(cr(j.vurgu)), 0);
    } else {
        dolgu(dc, dg, cr(j.vurgu), u(geo.dugme_yaricap) / 2);
    }
    // Kart / tablo.
    let kart = RECT {
        left: r.left + u(8),
        top: r.top + u(28),
        right: r.right - u(8),
        bottom: r.bottom - u(6),
    };
    match yerlesim(t).tablo_kenari {
        TabloKenari::YuzeyKarti => dolgu(dc, kart, cr(j.yuzey), u(geo.kart_yaricap) / 2),
        TabloKenari::InceCerceve => cerceveli(dc, kart, cr(j.kenar), 1, Some(cr(j.yuzey)), 2),
        TabloKenari::KalinUstAlt => {
            dolgu(dc, kart, cr(j.yuzey), 0);
            yatay_cizgi(dc, kart.left, kart.right, kart.top, 2, cr(j.kenar), None);
            yatay_cizgi(
                dc,
                kart.left,
                kart.right,
                kart.bottom - 2,
                2,
                cr(j.kenar),
                None,
            );
        }
    }
    let satir_h = ((kart.bottom - kart.top) / 3).max(4);
    let bicim = secili_bicim(t);
    for i in 0..3 {
        let y = kart.top + 2 + i * satir_h;
        let sr = RECT {
            left: kart.left + 2,
            top: y,
            right: kart.right - 2,
            bottom: (y + satir_h - 1).min(kart.bottom - 2),
        };
        let mut x = sr.left + u(4);
        if i == 1 {
            dolgu(dc, sr, cr(bicim.dolgu.renk(&j)), 0);
            let mut sx = sr.left;
            for (sw, sj) in &bicim.seritler {
                let ww = u(*sw).max(1);
                dolgu(
                    dc,
                    RECT {
                        left: sx,
                        right: sx + ww,
                        ..sr
                    },
                    cr(sj.renk(&j)),
                    0,
                );
                sx += ww;
            }
            x = x.max(sx + u(3));
        }
        let ty = (sr.top + sr.bottom) / 2;
        dolgu(
            dc,
            RECT {
                left: x,
                top: ty - 1,
                right: x + u(50),
                bottom: ty + 2,
            },
            cr(j.soluk),
            0,
        );
        // Durum işareti yerine küçük kare (basari).
        dolgu(
            dc,
            RECT {
                left: sr.right - u(30),
                top: ty - 2,
                right: sr.right - u(24),
                bottom: ty + 3,
            },
            cr(j.basari),
            0,
        );
        if i < 2 {
            let teyel = (yerlesim(t).satir_ayirici == SatirAyirici::Teyel).then_some((4, 3));
            yatay_cizgi(dc, sr.left, sr.right, sr.bottom, 1, cr(j.kenar), teyel);
        }
    }
}

/// ListView satırlarının özel çizimi (NM_CUSTOMDRAW): tema renkleri, seçili satır biçimi, işaretli durum.
unsafe fn satir_ozel_cizim(d: &Durum, cd: &mut NMLVCUSTOMDRAW) -> LRESULT {
    match cd.nmcd.dwDrawStage {
        CDDS_PREPAINT => CDRF_NOTIFYITEMDRAW as LRESULT,
        CDDS_ITEMPREPAINT => {
            satir_ciz(d, cd.nmcd.hdc, cd.nmcd.dwItemSpec);
            CDRF_SKIPDEFAULT as LRESULT
        }
        _ => CDRF_DODEFAULT as LRESULT,
    }
}

unsafe fn satir_ciz(d: &Durum, dc: HDC, i: usize) {
    let Some(s) = d.model.satirlar().get(i) else {
        return;
    };
    let Some(f) = &d.fontlar else {
        return;
    };
    let mut r = RECT {
        left: LVIR_BOUNDS as i32,
        top: 0,
        right: 0,
        bottom: 0,
    };
    SendMessageW(d.liste, LVM_GETITEMRECT, i, &mut r as *mut RECT as isize);
    let mut ic: RECT = std::mem::zeroed();
    GetClientRect(d.liste, &mut ic);
    let sag = r.right.max(ic.right);
    let t = d.gs.tema;
    let j = &d.renk;
    let secili = SendMessageW(d.liste, LVM_GETITEMSTATE, i, LVIS_SELECTED as isize) as u32
        & LVIS_SELECTED
        != 0;
    let bicim = secili_bicim(t);
    let satir = RECT { right: sag, ..r };
    let zemin = if secili { bicim.dolgu.renk(j) } else { j.yuzey };
    dolgu(dc, satir, colorref(zemin), 0);
    if secili {
        let mut x = r.left;
        for (w, sj) in &bicim.seritler {
            let ww = d.s(*w);
            dolgu(
                dc,
                RECT {
                    left: x,
                    right: x + ww,
                    ..satir
                },
                colorref(sj.renk(j)),
                0,
            );
            x += ww;
        }
    }
    // Satır ayırıcı: İlmek teyel, diğerleri düz (Kalıp açık ince, Mezura kenar).
    let ayirici = match yerlesim(t).satir_ayirici {
        SatirAyirici::Teyel => (j.kenar, Some((d.s(8), d.s(6)))),
        SatirAyirici::Cizgi if t == Tema::Mezura => (j.kenar, None),
        SatirAyirici::Cizgi => (j.yuzey2, None),
    };
    yatay_cizgi(
        dc,
        r.left,
        sag,
        r.bottom - 1,
        1,
        colorref(ayirici.0),
        ayirici.1,
    );
    let font = if secili && bicim.kalin {
        f.govde_kalin
    } else {
        f.govde
    };
    let pad = d.s(12);
    let mut x = r.left;
    for sutun in 0..SUTUNLAR.len() {
        let gen = SendMessageW(d.liste, LVM_GETCOLUMNWIDTH, sutun, 0) as i32;
        let mut alan = RECT {
            left: x + pad,
            top: r.top + d.s(2),
            right: x + gen - pad,
            bottom: r.bottom - d.s(2),
        };
        if sutun == 0 {
            alan.left += d.s(serit_genisligi(t));
        }
        match sutun {
            0 => {
                let ad = match (secili, bicim.isaret) {
                    (true, Some(i)) => format!("{i} {}", s.ad),
                    _ => s.ad.clone(),
                };
                metin_yaz(
                    dc,
                    &ad,
                    alan,
                    font,
                    colorref(j.metin),
                    kip_bayragi(hucre_kipi(0)),
                );
            }
            3 => {
                // İşaret + metin: işaret durum renginde ve kalın, metin `metin` renginde.
                let tur = durum_turu(s);
                let isaret = format!("{} ", tur.isaret());
                let iw = olc_genislik(dc, f.govde_kalin, &isaret);
                metin_yaz(
                    dc,
                    &isaret,
                    alan,
                    f.govde_kalin,
                    colorref(tur.jeton().renk(j)),
                    DT_SINGLELINE | DT_VCENTER | DT_LEFT,
                );
                sarili_ortali(
                    dc,
                    &s.durum,
                    RECT {
                        left: alan.left + iw,
                        ..alan
                    },
                    font,
                    colorref(j.metin),
                    DT_LEFT,
                );
            }
            _ => sarili_ortali(dc, s.hucre(sutun), alan, font, colorref(j.metin), DT_LEFT),
        }
        x += gen;
    }
    if secili && GetFocus() == d.liste {
        halka(
            dc,
            iceri(satir, 0),
            colorref(odak_jetonu(t, d.gs.koyu).renk(j)),
            d.s(2),
        );
    }
}

unsafe fn olc_genislik(dc: HDC, font: HFONT, metin: &str) -> i32 {
    // Boş metinde Vec::as_ptr() sarkık işaretçi olur; Win32'ye hiç verme.
    if metin.is_empty() {
        return 0;
    }
    let w: Vec<u16> = metin.encode_utf16().collect();
    let mut s = SIZE { cx: 0, cy: 0 };
    let eski = SelectObject(dc, font);
    GetTextExtentPoint32W(dc, w.as_ptr(), w.len() as i32, &mut s);
    SelectObject(dc, eski);
    s.cx
}

/// ListView alt sınıfı: başlık denetiminin NM_CUSTOMDRAW bildirimi ListView'a gelir; tema renginde çizilir.
unsafe extern "system" fn liste_proc(h: HWND, ileti: u32, w: WPARAM, l: LPARAM) -> LRESULT {
    let eski: windows_sys::Win32::UI::WindowsAndMessaging::WNDPROC =
        std::mem::transmute::<isize, _>(LISTE_ESKI_PROC.load(Ordering::Relaxed));
    if ileti == WM_NOTIFY {
        let nm = &*(l as *const NMHDR);
        if nm.code == NM_CUSTOMDRAW {
            if let Some(d) = durum_al(GetParent(h)) {
                if nm.hwndFrom == d.baslik {
                    return baslik_ozel_cizim(d, &mut *(l as *mut NMCUSTOMDRAW));
                }
            }
        }
    }
    CallWindowProcW(eski, h, ileti, w, l)
}

unsafe fn baslik_ozel_cizim(d: &Durum, cd: &mut NMCUSTOMDRAW) -> LRESULT {
    let t = d.gs.tema;
    let j = &d.renk;
    let zemin = if t == Tema::Kalip { j.yuzey2 } else { j.yuzey };
    match cd.dwDrawStage {
        CDDS_PREPAINT => {
            dolgu(cd.hdc, cd.rc, colorref(zemin), 0);
            CDRF_NOTIFYITEMDRAW as LRESULT
        }
        CDDS_ITEMPREPAINT => {
            let r = cd.rc;
            dolgu(cd.hdc, r, colorref(zemin), 0);
            let i = cd.dwItemSpec;
            let mut alan = RECT {
                left: r.left + d.s(12),
                right: r.right - d.s(12),
                ..r
            };
            if i == 0 {
                alan.left += d.s(serit_genisligi(t));
            }
            if let (Some(f), Some(b)) = (&d.fontlar, SUTUNLAR.get(i)) {
                let renk = if t == Tema::Ilmek { j.soluk } else { j.metin };
                metin_yaz(
                    cd.hdc,
                    b,
                    alan,
                    f.govde_kalin,
                    colorref(renk),
                    DT_SINGLELINE | DT_VCENTER | DT_LEFT,
                );
            }
            match t {
                Tema::Ilmek => yatay_cizgi(
                    cd.hdc,
                    r.left,
                    r.right,
                    r.bottom - 1,
                    1,
                    colorref(j.kenar),
                    Some((d.s(8), d.s(6))),
                ),
                Tema::Kalip => {
                    yatay_cizgi(
                        cd.hdc,
                        r.left,
                        r.right,
                        r.bottom - 1,
                        1,
                        colorref(j.kenar),
                        None,
                    );
                    dolgu(
                        cd.hdc,
                        RECT {
                            left: r.right - 1,
                            ..r
                        },
                        colorref(j.kenar),
                        0,
                    );
                }
                Tema::Mezura => {
                    let k = d.s(2);
                    yatay_cizgi(
                        cd.hdc,
                        r.left,
                        r.right,
                        r.bottom - k,
                        k,
                        colorref(j.kenar),
                        None,
                    )
                }
            }
            CDRF_SKIPDEFAULT as LRESULT
        }
        _ => CDRF_DODEFAULT as LRESULT,
    }
}

/// Düğme komutları. Modal pencereler (dosya seçici, onay, renk seçici) sırasında iç içe ileti gelebilir:
/// durum referansı pencere açılmadan önce bırakılır, sonra yeniden alınır.
unsafe fn komut(h: HWND, oge: Oge) {
    match oge {
        Oge::Kur | Oge::Kaldir | Oge::Guncelle | Oge::Rapor => paket_komutu(h, oge),
        Oge::Ayarlar => {
            if let Some(d) = durum_al(h) {
                let hedef = match d.ekran {
                    Ekran::Paketler => Ekran::Ayarlar,
                    Ekran::Ayarlar => Ekran::Paketler,
                };
                ekran_degistir(d, hedef);
            }
        }
        Oge::SekmePaketler => {
            if let Some(d) = durum_al(h) {
                ekran_degistir(d, Ekran::Paketler);
            }
        }
        Oge::SekmeAyarlar => {
            if let Some(d) = durum_al(h) {
                ekran_degistir(d, Ekran::Ayarlar);
            }
        }
        Oge::Kopyala => {
            if let Some(d) = durum_al(h) {
                let yol = d.model.kok_yolu().display().to_string();
                let metin = if panoya_yaz(h, &yol) {
                    format!("Kök klasör panoya kopyalandı: {yol}")
                } else {
                    "Pano kullanılamadı. Öneri: biraz sonra yeniden deneyin.".to_string()
                };
                d.model.durum_ata(metin);
                yerlestir(d);
            }
        }
        Oge::Tema(t) => ayar_degistir(h, true, |d| {
            d.gorunum.tema = Some(t);
            d.vurgu_notu = None;
        }),
        Oge::Mod(m) => ayar_degistir(h, true, |d| {
            d.gorunum.modu = m;
            d.vurgu_notu = None;
        }),
        Oge::HareketiAzalt => ayar_degistir(h, false, |d| {
            d.gorunum.hareketi_azalt = Some(!d.gs.hareketi_azalt);
        }),
        Oge::RenkSifirla => ayar_degistir(h, true, |d| {
            let t = d.gs.tema;
            d.gorunum.ozel_vurgu_ata(t, None);
            d.vurgu_notu = None;
        }),
        Oge::RenkSec => {
            let (baslangic, mut ozel) = match durum_al(h) {
                Some(d) => (d.gs.j.vurgu, d.ozel_renkler),
                None => return,
            };
            let secilen = renk_sec(h, baslangic, &mut ozel);
            if let Some(d) = durum_al(h) {
                d.ozel_renkler = ozel;
            }
            if let Some(renk) = secilen {
                ayar_degistir(h, true, |d| {
                    let (t, koyu) = (d.gs.tema, d.gs.koyu);
                    d.vurgu_notu = vurgu_sec(&mut d.gorunum, t, koyu, renk);
                });
            }
        }
        Oge::TanitimTekrar => ayar_degistir(h, false, |d| {
            // Bu pencerede tanıtım yok: yalnız tercih sıfırlanır, TAA Kurucu bir sonraki açılışta gösterir.
            d.gorunum.tanitim_goruldu = false;
            d.model.durum_ata(TANITIM_SIFIRLANDI);
        }),
        Oge::Varsayilan => {
            if mesaj_kutusu(h, VARSAYILAN_ONAY, "TAA Kurucu", MB_YESNO | MB_ICONQUESTION) == IDYES {
                ayar_degistir(h, true, |d| {
                    d.gorunum.varsayilana_don();
                    d.vurgu_notu = None;
                    d.model.durum_ata("Görünüm tercihleri varsayılana döndü.");
                });
            }
        }
    }
}

/// Görünüm tercihini değiştirir: anında uygular, kaydeder (atomik), diğer TAA pencerelerine duyurur.
unsafe fn ayar_degistir(h: HWND, animasyon: bool, degistir: impl FnOnce(&mut Durum)) {
    let Some(d) = durum_al(h) else {
        return;
    };
    degistir(d);
    match d.gorunum.kaydet() {
        Ok(()) => {
            PostMessageW(HWND_BROADCAST, d.gorunum_iletisi, h as usize, 0);
        }
        Err(e) => d.model.durum_ata(format!(
            "Görünüm tercihi kaydedilemedi ({e}). Seçim bu pencerede geçerli. Öneri: %APPDATA%\\TAA\\veri klasörüne yazma iznini denetleyin."
        )),
    }
    gorunus_uygula(d, animasyon);
}

unsafe fn paket_komutu(h: HWND, oge: Oge) {
    let eylem = match oge {
        Oge::Kur => dosya_sec(h).map(Eylem::Kur),
        Oge::Kaldir => {
            let soru = durum_al(h).and_then(|d| d.model.kaldir_onay_metni());
            soru.filter(|s| mesaj_kutusu(h, s, "TAA Kurucu", MB_YESNO | MB_ICONQUESTION) == IDYES)
                .map(|_| Eylem::Kaldir)
        }
        Oge::Guncelle => {
            // İndirip kuracağı için önce sorulur; seçili satır yoksa soru yok, model mesaj verir.
            let soru = durum_al(h).and_then(|d| d.model.guncelle_onay_metni());
            match soru {
                None => Some(Eylem::Guncelle),
                Some(s) => (mesaj_kutusu(h, &s, "TAA Kurucu", MB_YESNO | MB_ICONQUESTION) == IDYES)
                    .then_some(Eylem::Guncelle),
            }
        }
        // Salt okunur; soru yok. Rapor metni bilgi kutusunda gösterilir, CSV kök\raporlar'a yazılır.
        Oge::Rapor => Some(Eylem::Rapor),
        _ => None,
    };
    let Some(eylem) = eylem else {
        return;
    };
    let rapor_metni = match durum_al(h) {
        None => return,
        Some(d) => match eylem {
            Eylem::Kur(yol) => {
                d.model.kur_paketi(&yol);
                None
            }
            Eylem::Kaldir => {
                d.model.kaldir();
                None
            }
            Eylem::Guncelle => {
                d.model.guncelle();
                None
            }
            Eylem::Rapor => Some(d.model.rapor()),
        },
    };
    if let Some(d) = durum_al(h) {
        liste_doldur(d);
        dugmeleri_guncelle(d);
        yerlestir(d);
    }
    if let Some(metin) = rapor_metni {
        mesaj_kutusu(h, &metin, "TAA paket raporu", MB_OK | MB_ICONINFORMATION);
    }
}

unsafe fn liste_doldur(d: &mut Durum) {
    YENILIYOR.store(true, Ordering::Relaxed);
    SendMessageW(d.liste, LVM_DELETEALLITEMS, 0, 0);
    for (i, s) in d.model.satirlar().iter().enumerate() {
        for sutun in 0..SUTUNLAR.len() {
            // Durum sütununun erişilebilir metni de işaretli ("✓ Hazır"); görünen çizim özel çizimdedir.
            let metin = if sutun == 3 {
                crate::pencere_tema::durum_gorunen(s)
            } else {
                s.hucre(sutun).to_string()
            };
            let mut yazi = genis(&metin);
            let mut oge: LVITEMW = std::mem::zeroed();
            oge.mask = LVIF_TEXT;
            oge.iItem = i as i32;
            oge.iSubItem = sutun as i32;
            oge.pszText = yazi.as_mut_ptr();
            if sutun == 0 {
                SendMessageW(d.liste, LVM_INSERTITEMW, 0, &oge as *const _ as isize);
            } else {
                SendMessageW(d.liste, LVM_SETITEMTEXTW, i, &oge as *const _ as isize);
            }
        }
    }
    YENILIYOR.store(false, Ordering::Relaxed);
    // Seçim yenilemeden sonra modelde kalmışsa listede de seçili göster (düğme kuralları modeldedir).
    if let Some(i) = d.model.secili_indeks() {
        let mut oge: LVITEMW = std::mem::zeroed();
        oge.stateMask = LVIS_SELECTED;
        oge.state = LVIS_SELECTED;
        YENILIYOR.store(true, Ordering::Relaxed);
        SendMessageW(
            d.liste,
            windows_sys::Win32::UI::Controls::LVM_SETITEMSTATE,
            i,
            &oge as *const _ as isize,
        );
        YENILIYOR.store(false, Ordering::Relaxed);
    }
}

unsafe fn dugmeleri_guncelle(d: &Durum) {
    let k = d.model.dugmeler();
    EnableWindow(d.dugme(Oge::Kur), k.kur as i32);
    EnableWindow(d.dugme(Oge::Kaldir), k.kaldir as i32);
    EnableWindow(d.dugme(Oge::Guncelle), k.guncelle as i32);
    EnableWindow(d.dugme(Oge::Rapor), k.rapor as i32);
    for o in [Oge::Kur, Oge::Kaldir, Oge::Guncelle, Oge::Rapor] {
        InvalidateRect(d.dugme(o), null(), 0);
    }
}

/// Metni panoya yazar (CF_UNICODETEXT).
unsafe fn panoya_yaz(sahip: HWND, metin: &str) -> bool {
    let w = genis(metin);
    let bayt = w.len() * 2;
    if OpenClipboard(sahip) == 0 {
        return false;
    }
    EmptyClipboard();
    let bellek = GlobalAlloc(GMEM_MOVEABLE, bayt);
    let mut tamam = false;
    if !bellek.is_null() {
        let p = GlobalLock(bellek) as *mut u16;
        if !p.is_null() {
            std::ptr::copy_nonoverlapping(w.as_ptr(), p, w.len());
            GlobalUnlock(bellek);
            // Başarıda bellek panonun olur; başarısızlıkta biz bırakırız.
            tamam = !SetClipboardData(CF_UNICODETEXT as u32, bellek).is_null();
        }
        if !tamam {
            GlobalFree(bellek);
        }
    }
    CloseClipboard();
    tamam
}

/// Windows renk seçici; seçilen renk 0xRRGGBB.
fn renk_sec(sahip: HWND, baslangic: u32, ozel: &mut [u32; 16]) -> Option<u32> {
    // SAFETY: CHOOSECOLORW alanları geçerli; ozel 16 COLORREF, çağrı süresince yaşar.
    unsafe {
        let mut cc: CHOOSECOLORW = std::mem::zeroed();
        cc.lStructSize = std::mem::size_of::<CHOOSECOLORW>() as u32;
        cc.hwndOwner = sahip;
        cc.rgbResult = colorref(baslangic);
        cc.lpCustColors = ozel.as_mut_ptr();
        cc.Flags = CC_RGBINIT | CC_FULLOPEN;
        if ChooseColorW(&mut cc) == 0 {
            return None;
        }
        Some(colorref_rgb(cc.rgbResult))
    }
}

/// Paket dosyası seçtirir (`paket.zip` ya da klasör paketinin `paket.json` dosyası).
fn dosya_sec(sahip: HWND) -> Option<PathBuf> {
    let filtre: Vec<u16> = "Paket (*.zip;paket.json)\0*.zip;paket.json\0Tüm dosyalar\0*.*\0\0"
        .encode_utf16()
        .collect();
    let mut tampon = vec![0u16; 32768];
    // SAFETY: OPENFILENAMEW alanları geçerli tamponlara işaret eder; çağrı süresince yaşarlar.
    unsafe {
        let mut o: OPENFILENAMEW = std::mem::zeroed();
        o.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        o.hwndOwner = sahip;
        o.lpstrFilter = filtre.as_ptr();
        o.lpstrFile = tampon.as_mut_ptr();
        o.nMaxFile = tampon.len() as u32;
        o.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR;
        if GetOpenFileNameW(&mut o) == 0 {
            return None;
        }
    }
    let uzunluk = tampon.iter().position(|&c| c == 0).unwrap_or(tampon.len());
    let yol = OsString::from_wide(&tampon[..uzunluk]);
    Some(PathBuf::from(yol))
}
