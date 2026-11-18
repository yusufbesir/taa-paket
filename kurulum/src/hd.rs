// Sahip: Yusuf. yazan: [Claude#kurucu-arayuz coPLkm 2026-11-17]
//! HD logo basma: taa_kur_tema::simge ile kenar yumuşatmalı taranmış logoyu 32 bit DIB'e koyup
//! `AlphaBlend` ile basar; pencere simgesini de aynı taramadan alfa kanallı üretir.

use std::ffi::c_void;
use std::ptr::null_mut;
use taa_kur_tema::simge::{tara, tara_onbellekli, uygulama_simgesi, LogoCizimi};
use windows_sys::Win32::Graphics::Gdi::{
    AlphaBlend, CreateBitmap, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, SelectObject, AC_SRC_ALPHA,
    AC_SRC_OVER, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, BLENDFUNCTION, DIB_RGB_COLORS, HBITMAP, HDC,
};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::HiDpi::{
    GetSystemMetricsForDpi, SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, DestroyIcon, GetSystemMetrics, SendMessageW, SetProcessDPIAware, HICON, ICONINFO, ICON_BIG,
    ICON_SMALL, SM_CXICON, SM_CXSMICON, WM_SETICON,
};

/// Önceden çarpılmış BGRA pikselleri tutan üstten-alta 32 bit DIB.
unsafe fn dib(n: i32, piks: &[u32]) -> HBITMAP {
    let mut bi: BITMAPINFO = std::mem::zeroed();
    bi.bmiHeader = BITMAPINFOHEADER {
        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
        biWidth: n,
        biHeight: -n,
        biPlanes: 1,
        biBitCount: 32,
        biCompression: BI_RGB,
        ..std::mem::zeroed()
    };
    let mut bits: *mut c_void = null_mut();
    let h = CreateDIBSection(null_mut(), &bi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
    if h.is_null() || bits.is_null() {
        return null_mut();
    }
    std::ptr::copy_nonoverlapping(piks.as_ptr(), bits as *mut u32, piks.len());
    h
}

/// Logoyu `(x, y)` sol üst köşesine kenar yumuşatmalı basar. Başarısızsa hiçbir şey çizmez.
///
/// # Safety
/// `dc` geçerli bir aygıt bağlamı olmalı.
pub unsafe fn logo_bas(dc: HDC, x: i32, y: i32, c: &LogoCizimi) {
    let n = c.boyut as i32;
    if n <= 0 {
        return;
    }
    let piks = tara_onbellekli(c);
    let bmp = dib(n, &piks);
    if bmp.is_null() {
        return;
    }
    let kaynak = CreateCompatibleDC(dc);
    let eski = SelectObject(kaynak, bmp);
    let bf = BLENDFUNCTION { BlendOp: AC_SRC_OVER as u8, BlendFlags: 0, SourceConstantAlpha: 255, AlphaFormat: AC_SRC_ALPHA as u8 };
    AlphaBlend(dc, x, y, n, n, kaynak, 0, 0, n, n, bf);
    SelectObject(kaynak, eski);
    DeleteObject(bmp);
    DeleteDC(kaynak);
}

/// Alfa kanallı simge (görev çubuğu, Alt+Tab, başlık). Yaratılamazsa boş tutamak.
///
/// # Safety
/// Win32 çağrıları; dönen simge pencere ömrü boyunca tutulur.
pub unsafe fn simge_yap(c: &LogoCizimi) -> HICON {
    let n = c.boyut as i32;
    let renk = dib(n, &tara(c));
    if renk.is_null() {
        return null_mut();
    }
    // Maske sıfır: saydamlık 32 bit rengin alfasından gelir.
    let maske_satir = (((n + 15) / 16) * 2) as usize;
    let sifir = vec![0u8; maske_satir * n as usize];
    let maske = CreateBitmap(n, n, 1, 1, sifir.as_ptr() as *const c_void);
    let ii = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: maske, hbmColor: renk };
    let h = CreateIconIndirect(&ii);
    DeleteObject(renk);
    if !maske.is_null() {
        DeleteObject(maske);
    }
    h
}

/// Pencere sınıfı için büyük uygulama simgesi (sistem DPI'si boyutunda).
///
/// # Safety
/// Win32 çağrısı.
pub unsafe fn sinif_simgesi() -> HICON {
    let n = GetSystemMetrics(SM_CXICON);
    simge_yap(&uygulama_simgesi(if n > 0 { n as u32 } else { 32 }))
}

/// Pencerenin büyük ve küçük simgesini `dpi` için yeniden üretir (açılışta ve WM_DPICHANGED'de):
/// %150'de 48/24 px, %200'de 64/32 px … Windows küçük simgeyi büyütüp pikselleştirmez.
///
/// # Safety
/// `hwnd` geçerli pencere olmalı.
pub unsafe fn simgeleri_ayarla(hwnd: HWND, dpi: u32) {
    for (tur, olcu, yedek) in [(ICON_BIG, SM_CXICON, 32), (ICON_SMALL, SM_CXSMICON, 16)] {
        let n = GetSystemMetricsForDpi(olcu, dpi.max(96));
        let h = simge_yap(&uygulama_simgesi(if n > 0 { n as u32 } else { yedek * dpi.max(96) / 96 }));
        if h.is_null() {
            continue;
        }
        let eski = SendMessageW(hwnd, WM_SETICON, tur as usize, h as isize);
        if eski != 0 {
            DestroyIcon(eski as HICON);
        }
    }
}

/// Monitör başına DPI farkındalığı (v2): pencere ekran ölçeği değişince Windows tarafından bitmap
/// olarak büyütülmez (bulanık / piksel piksel görünüm); kendi DPI'sinde yeniden çizilir. Eski
/// Windows'ta sistem DPI farkındalığına düşer.
///
/// # Safety
/// Pencere yaratılmadan önce bir kez çağrılmalı.
pub unsafe fn dpi_farkindaligi() {
    if SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) == 0 {
        SetProcessDPIAware();
    }
}
