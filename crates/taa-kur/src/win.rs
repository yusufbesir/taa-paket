// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! Win32 çizim katmanı (yalnız Windows): ince katman, mantık `pencere_model` içindedir.
//! Düzen: üstte liste (ListView), altında Kur / Kaldır / Güncelle düğmeleri, en altta durum satırı.
//! İşlemler arayüz iş parçacığında eşzamanlı çalışır (küçük paketler; T3 sonrası ayrı iş parçacığı düşünülebilir).
//! DİKKAT: Windows'ta elle doğrulanmadı; Linux'ta yalnız tür denetimi yapıldı.

use crate::pencere_model::{KaynaksizGuncelleyici, PencereModel, SUTUNLAR};
use crate::Kur;
use std::ffi::OsString;
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{GetStockObject, COLOR_WINDOW, DEFAULT_GUI_FONT};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Controls::Dialogs::{
    GetOpenFileNameW, OFN_FILEMUSTEXIST, OFN_NOCHANGEDIR, OFN_PATHMUSTEXIST, OPENFILENAMEW,
};
use windows_sys::Win32::UI::Controls::{
    InitCommonControlsEx, ICC_LISTVIEW_CLASSES, INITCOMMONCONTROLSEX, LVCF_SUBITEM, LVCF_TEXT,
    LVCF_WIDTH, LVCOLUMNW, LVIF_TEXT, LVITEMW, LVM_DELETEALLITEMS, LVM_GETNEXTITEM,
    LVM_INSERTCOLUMNW, LVM_INSERTITEMW, LVM_SETEXTENDEDLISTVIEWSTYLE, LVM_SETITEMTEXTW,
    LVNI_SELECTED, LVN_ITEMCHANGED, LVS_EX_FULLROWSELECT, LVS_REPORT, LVS_SHOWSELALWAYS,
    LVS_SINGLESEL, NMHDR, WC_LISTVIEWW,
};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::EnableWindow;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetMessageW,
    GetWindowLongPtrW, LoadCursorW, MessageBoxW, MoveWindow, PostQuitMessage, RegisterClassW,
    SendMessageW, SetWindowLongPtrW, SetWindowTextW, ShowWindow, TranslateMessage, BN_CLICKED,
    BS_PUSHBUTTON, CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, IDC_ARROW, IDYES, MB_ICONERROR,
    MB_ICONQUESTION, MB_YESNO, MSG, SW_SHOW, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_NCCREATE,
    WM_NOTIFY, WM_SETFONT, WM_SIZE, WNDCLASSW, WS_CHILD, WS_EX_CLIENTEDGE, WS_OVERLAPPEDWINDOW,
    WS_TABSTOP, WS_VISIBLE,
};

const KIMLIK_KUR: isize = 101;
const KIMLIK_KALDIR: isize = 102;
const KIMLIK_GUNCELLE: isize = 103;

/// Liste yenilenirken gelen bildirimler (silme/ekleme) yok sayılır.
static YENILIYOR: AtomicBool = AtomicBool::new(false);

enum Eylem {
    Kur(PathBuf),
    Kaldir,
    Guncelle,
}

struct Durum {
    model: PencereModel,
    liste: HWND,
    kur: HWND,
    kaldir: HWND,
    guncelle: HWND,
    durum: HWND,
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

/// Giriş noktası: pencereyi açar, ileti döngüsünü çalıştırır; çıkış kodu döner.
pub fn calistir() -> i32 {
    let kur = match Kur::ortamdan() {
        Ok(k) => k,
        Err(h) => {
            mesaj_kutusu(null_mut(), h.mesaj(), "TAA Kur", MB_ICONERROR);
            return h.cikis_kodu();
        }
    };
    let durum = Box::new(Durum {
        model: PencereModel::yeni(kur, Box::new(KaynaksizGuncelleyici)),
        liste: null_mut(),
        kur: null_mut(),
        kaldir: null_mut(),
        guncelle: null_mut(),
        durum: null_mut(),
    });
    let durum_ptr = Box::into_raw(durum);
    // SAFETY: Win32 çağrıları; durum_ptr pencere yıkılana kadar geçerli, sonda geri alınır.
    unsafe {
        let ic = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_LISTVIEW_CLASSES,
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
            hbrBackground: (COLOR_WINDOW + 1) as usize as _,
            lpszMenuName: null(),
            lpszClassName: sinif.as_ptr(),
        };
        if RegisterClassW(&wc) == 0 {
            drop(Box::from_raw(durum_ptr));
            mesaj_kutusu(
                null_mut(),
                "Pencere sınıfı kaydedilemedi.",
                "TAA Kur",
                MB_ICONERROR,
            );
            return 1;
        }
        let baslik = genis("TAA Kur — paketler");
        let pencere = CreateWindowExW(
            0,
            sinif.as_ptr(),
            baslik.as_ptr(),
            WS_OVERLAPPEDWINDOW | WS_VISIBLE,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            720,
            440,
            null_mut(),
            null_mut(),
            ornek,
            durum_ptr as *const _,
        );
        if pencere.is_null() {
            drop(Box::from_raw(durum_ptr));
            mesaj_kutusu(null_mut(), "Pencere açılamadı.", "TAA Kur", MB_ICONERROR);
            return 1;
        }
        ShowWindow(pencere, SW_SHOW);
        let mut ileti: MSG = std::mem::zeroed();
        while GetMessageW(&mut ileti, null_mut(), 0, 0) > 0 {
            TranslateMessage(&ileti);
            DispatchMessageW(&ileti);
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
    ex_stil: u32,
    sinif: *const u16,
    metin: &str,
    stil: u32,
    kimlik: isize,
    ust: HWND,
    ornek: windows_sys::Win32::Foundation::HMODULE,
) -> HWND {
    let m = genis(metin);
    let h = CreateWindowExW(
        ex_stil,
        sinif,
        m.as_ptr(),
        WS_CHILD | WS_VISIBLE | stil,
        0,
        0,
        10,
        10,
        ust,
        kimlik as _,
        ornek,
        null(),
    );
    SendMessageW(h, WM_SETFONT, GetStockObject(DEFAULT_GUI_FONT) as usize, 1);
    h
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
                let ornek = GetModuleHandleW(null());
                d.liste = cocuk(
                    WS_EX_CLIENTEDGE,
                    WC_LISTVIEWW,
                    "",
                    WS_TABSTOP | LVS_REPORT | LVS_SINGLESEL | LVS_SHOWSELALWAYS,
                    0,
                    h,
                    ornek,
                );
                SendMessageW(
                    d.liste,
                    LVM_SETEXTENDEDLISTVIEWSTYLE,
                    0,
                    LVS_EX_FULLROWSELECT as isize,
                );
                let genislikler = [170, 140, 110, 260];
                for (i, b) in SUTUNLAR.iter().enumerate() {
                    let mut yazi = genis(b);
                    let mut s: LVCOLUMNW = std::mem::zeroed();
                    s.mask = LVCF_TEXT | LVCF_WIDTH | LVCF_SUBITEM;
                    s.cx = genislikler[i];
                    s.pszText = yazi.as_mut_ptr();
                    s.iSubItem = i as i32;
                    SendMessageW(d.liste, LVM_INSERTCOLUMNW, i, &s as *const _ as isize);
                }
                let btn = "BUTTON\0".encode_utf16().collect::<Vec<u16>>();
                d.kur = cocuk(
                    0,
                    btn.as_ptr(),
                    "Kur...",
                    WS_TABSTOP | BS_PUSHBUTTON as u32,
                    KIMLIK_KUR,
                    h,
                    ornek,
                );
                d.kaldir = cocuk(
                    0,
                    btn.as_ptr(),
                    "Kaldır",
                    WS_TABSTOP | BS_PUSHBUTTON as u32,
                    KIMLIK_KALDIR,
                    h,
                    ornek,
                );
                d.guncelle = cocuk(
                    0,
                    btn.as_ptr(),
                    "Güncelle",
                    WS_TABSTOP | BS_PUSHBUTTON as u32,
                    KIMLIK_GUNCELLE,
                    h,
                    ornek,
                );
                let st = "STATIC\0".encode_utf16().collect::<Vec<u16>>();
                d.durum = cocuk(0, st.as_ptr(), "", 0, 0, h, ornek);
                liste_doldur(d);
                dugmeleri_guncelle(d);
                durum_yaz(d);
            }
            0
        }
        WM_SIZE => {
            if let Some(d) = durum_al(h) {
                yerlestir(h, d);
            }
            0
        }
        WM_NOTIFY => {
            if YENILIYOR.load(Ordering::Relaxed) {
                return 0;
            }
            let nm = &*(l as *const NMHDR);
            if let Some(d) = durum_al(h) {
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
                // Modal pencereler (dosya seçici, onay) sırasında iç içe ileti gelebilir: durum referansı
                // pencere açılmadan önce bırakılır, sonra yeniden alınır.
                let eylem = match kimlik {
                    KIMLIK_KUR => dosya_sec(h).map(Eylem::Kur),
                    KIMLIK_KALDIR => {
                        let soru = durum_al(h).and_then(|d| d.model.kaldir_onay_metni());
                        soru.filter(|s| {
                            mesaj_kutusu(h, s, "TAA Kur", MB_YESNO | MB_ICONQUESTION) == IDYES
                        })
                        .map(|_| Eylem::Kaldir)
                    }
                    KIMLIK_GUNCELLE => Some(Eylem::Guncelle),
                    _ => None,
                };
                if let (Some(eylem), Some(d)) = (eylem, durum_al(h)) {
                    match eylem {
                        Eylem::Kur(yol) => {
                            d.model.kur_paketi(&yol);
                        }
                        Eylem::Kaldir => {
                            d.model.kaldir();
                        }
                        Eylem::Guncelle => {
                            d.model.guncelle();
                        }
                    }
                    liste_doldur(d);
                    dugmeleri_guncelle(d);
                    durum_yaz(d);
                }
            }
            0
        }
        WM_DESTROY => {
            let p = GetWindowLongPtrW(h, GWLP_USERDATA) as *mut Durum;
            SetWindowLongPtrW(h, GWLP_USERDATA, 0);
            if !p.is_null() {
                drop(Box::from_raw(p));
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(h, ileti, w, l),
    }
}

unsafe fn yerlestir(h: HWND, d: &Durum) {
    let mut r: RECT = std::mem::zeroed();
    GetClientRect(h, &mut r);
    let (g, y) = (r.right - r.left, r.bottom - r.top);
    let kenar = 8;
    let durum_y = 22;
    let dugme_y = 28;
    let liste_y = (y - kenar * 3 - durum_y - dugme_y).max(40);
    MoveWindow(d.liste, kenar, kenar, (g - 2 * kenar).max(100), liste_y, 1);
    let dy = kenar * 2 + liste_y;
    MoveWindow(d.kur, kenar, dy, 100, dugme_y, 1);
    MoveWindow(d.kaldir, kenar + 108, dy, 100, dugme_y, 1);
    MoveWindow(d.guncelle, kenar + 216, dy, 100, dugme_y, 1);
    MoveWindow(
        d.durum,
        kenar,
        dy + dugme_y + kenar,
        (g - 2 * kenar).max(100),
        durum_y,
        1,
    );
}

unsafe fn liste_doldur(d: &mut Durum) {
    YENILIYOR.store(true, Ordering::Relaxed);
    SendMessageW(d.liste, LVM_DELETEALLITEMS, 0, 0);
    for (i, s) in d.model.satirlar().iter().enumerate() {
        for sutun in 0..SUTUNLAR.len() {
            let mut yazi = genis(s.hucre(sutun));
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
}

unsafe fn dugmeleri_guncelle(d: &Durum) {
    let k = d.model.dugmeler();
    EnableWindow(d.kur, k.kur as i32);
    EnableWindow(d.kaldir, k.kaldir as i32);
    EnableWindow(d.guncelle, k.guncelle as i32);
}

unsafe fn durum_yaz(d: &Durum) {
    let metin = format!("{}   [{}]", d.model.durum(), d.model.kok_metni());
    let w = genis(&metin);
    SetWindowTextW(d.durum, w.as_ptr());
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
