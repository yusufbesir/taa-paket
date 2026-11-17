// Sahip: Yusuf. yazan: [Claude#kur-yerel-pencere 2026-11-15] değiştiren: [Claude#kurucu-arayuz 2026-11-17]
//! Yerel kurulum penceresi (yalnız Windows; Win32 + GDI, windows-sys). Tarayıcı ya da HTTP YOKTUR.
//! İnce çizim katmanı: kurulum işi ayrı iş parçacığında `ekran::Paylasim`'ı günceller; pencere 120 ms'de bir
//! anlık kopyasını çizer (çift tamponlu, `cizim.rs`), tuş ve tıklamaları `ekran::Komut` olarak gönderir.
//! Görünüm: üç tema (İlmek varsayılan, Kalıp, Mezura; KARAR-FABLE.md), açık/koyu/sistemi izle, özel vurgu,
//! hareketi azalt; tercih `<veri kökü>\kurucu\gorunum.json` (`taa_kur_tema::Gorunum`). Windows Yüksek Karşıtlık
//! açıksa ve kullanıcı tema seçmediyse Mezura; hareketi azaltın varsayılanı SPI_GETCLIENTAREAANIMATION'ın tersi.
//! Açılışta markalı yükleme ekranı (10 tik), ilk açılışta 3 sahneli tanıtım; başlıkta Ayarlar yan paneli.
//! Klavye: Enter = onay / Yeniden dene / Bitir, Esc = vazgeç / kapat, Yukarı/Aşağı + Boşluk = seçim, Sol/Sağ = kartlar,
//! Tab = Ayarlar düğmesi (panelde denetimler arası; Esc paneli kapatır). Yüksek DPI: SetProcessDPIAware + GetDpiForWindow.
//! DİKKAT: Windows'ta elle DENENMEDİ; Linux'ta yalnız `cargo check/clippy --target x86_64-pc-windows-gnu`.

use crate::cizim::{self, Cizer, Fontlar};
use crate::ekran::{Komut, Paylasim, Uc};
use crate::ilerleme::Asama;
use crate::kartlar::{self, Kart};
use crate::secim::Secim;
use crate::yerlesim::{self as y, AyarOge, Dik, Gecis, Tanitim, Yukleme};
use std::ffi::c_void;
use std::ptr::{null, null_mut};
use std::time::Instant;
use taa_kur_tema::{
    colorref, colorref_rgb, etkin_jetonlar, hataya_yakin, logo, vurgu_duzelt, Gorunum, Jetonlar, Mod, Tema, ILMEK_ACIK,
};
use windows_sys::Win32::Foundation::{ERROR_SUCCESS, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWA_USE_IMMERSIVE_DARK_MODE};
use windows_sys::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateBitmap, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, EndPaint,
    FillRect, GetDC, GetMonitorInfoW, GetStockObject, InvalidateRect, MonitorFromWindow, ReleaseDC, SelectObject,
    SetBkMode, BLACK_BRUSH, MONITORINFO, MONITOR_DEFAULTTONEAREST, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
};
use windows_sys::Win32::System::Console::{GetConsoleProcessList, GetConsoleWindow};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
use windows_sys::Win32::UI::Accessibility::{HCF_HIGHCONTRASTON, HIGHCONTRASTW};
use windows_sys::Win32::UI::Controls::Dialogs::{ChooseColorW, CC_FULLOPEN, CC_RGBINIT, CHOOSECOLORW};
use windows_sys::Win32::UI::HiDpi::GetDpiForWindow;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VK_DOWN, VK_ESCAPE, VK_LEFT, VK_RETURN, VK_RIGHT, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconIndirect, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, GetMessageW,
    GetWindowLongPtrW, LoadCursorW, LoadIconW, MessageBoxW, PostQuitMessage, RegisterClassW, SendMessageW, SetCursor,
    SetProcessDPIAware, SetTimer, SetWindowLongPtrW, SetWindowPos, ShowWindow, SystemParametersInfoW, TranslateMessage,
    CREATESTRUCTW, CW_USEDEFAULT, GWLP_USERDATA, HICON, HTCLIENT, ICONINFO, ICON_SMALL, IDC_ARROW, IDC_HAND,
    IDI_APPLICATION, IDYES, MB_ICONQUESTION, MB_YESNO, MINMAXINFO, MSG, SPI_GETCLIENTAREAANIMATION, SPI_GETHIGHCONTRAST,
    SWP_NOZORDER, SW_HIDE, SW_SHOW, WM_CLOSE, WM_DESTROY, WM_DPICHANGED, WM_ERASEBKGND, WM_GETMINMAXINFO, WM_KEYDOWN,
    WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL, WM_NCCREATE, WM_NCDESTROY, WM_PAINT, WM_SETCURSOR, WM_SETICON,
    WM_SETTINGCHANGE, WM_SIZE, WM_TIMER, WNDCLASSW, WS_OVERLAPPEDWINDOW,
};

const SINIF: &str = "TaaKurKurulum";
const ZAMANLAYICI: usize = 1;
/// Mantıksal (96 DPI) pencere boyutu ve en küçük boyut.
const GENISLIK: i32 = 1040;
const YUKSEKLIK: i32 = 700;
const EN_AZ: (i32, i32) = (860, 600);

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

/// Windows Yüksek Karşıtlık açık mı (SPI_GETHIGHCONTRAST)?
fn yuksek_karsitlik() -> bool {
    let mut hc = HIGHCONTRASTW { cbSize: std::mem::size_of::<HIGHCONTRASTW>() as u32, dwFlags: 0, lpszDefaultScheme: null_mut() };
    // SAFETY: doğru boyutlu yapı; sistem yalnız okur/yazar.
    let ok = unsafe { SystemParametersInfoW(SPI_GETHIGHCONTRAST, hc.cbSize, &mut hc as *mut HIGHCONTRASTW as *mut c_void, 0) };
    ok != 0 && hc.dwFlags & HCF_HIGHCONTRASTON != 0
}

/// Windows "animasyonları göster" açık mı (SPI_GETCLIENTAREAANIMATION)? Okunamazsa açık sayılır.
fn sistem_animasyon() -> bool {
    let mut v: i32 = 1;
    // SAFETY: BOOL boyutunda yazılabilir alan.
    let ok = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut v as *mut i32 as *mut c_void, 0) };
    ok == 0 || v != 0
}

fn genis(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Tıklanabilir alanlar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Hedef {
    Onay,
    Vazgec,
    Secenek(usize),
    YenidenDene,
    Bitir,
    TaaKurAc,
    KartGeri,
    KartIleri,
    KartDuraklat,
    /// İlmek hap çipi: grup sırası (`Secim::gruplar`).
    Grup(usize),
    Ayarlar,
    Ayar(AyarOge),
    TanitimIleri,
    TanitimAtla,
    /// Üstteki katmanın boş yüzeyi (alttakine tıklama geçmesin diye).
    Bos,
}

pub(crate) struct PencereDurumu {
    pub uc: Uc,
    pub kartlar: Vec<Kart>,
    pub kart: usize,
    pub kart_zamani: Instant,
    pub kart_alani: Dik,
    /// Kart değişiminin karesi (0'dan başlar; tema adım sayısına ulaşınca durgun).
    pub kart_tik: u8,
    pub kart_duraklat: bool,
    /// Ölçülen kart yüksekliği önbelleği: (tema, genişlik, dpi) -> yükseklik.
    pub kart_h: Option<((Tema, i32, u32), i32)>,
    pub dpi: u32,
    pub gorunum: Gorunum,
    pub sistem_koyu: bool,
    pub yuksek_karsitlik: bool,
    pub sistem_animasyon: bool,
    pub tema: Tema,
    pub koyu: bool,
    pub j: Jetonlar,
    /// İlmek renk geçişi için önceki jetonlar.
    pub eski_j: Jetonlar,
    pub fontlar: Fontlar,
    /// Tıklama alanları; sonra çizilen üsttedir (ters sırada aranır).
    pub hedefler: Vec<(Dik, Hedef)>,
    pub uzerinde: Option<Hedef>,
    pub fare: (i32, i32),
    pub tik: u64,
    pub yukleme: Option<Yukleme>,
    pub tanitim: Option<Tanitim>,
    pub gecis: Option<Gecis>,
    /// Ekran değişimini (geçiş hareketi için) yakalayan anahtar: aşama, seçim başlığı, hata.
    pub ekran_anahtari: (usize, Option<String>, bool),
    pub ayarlar_acik: bool,
    pub ayar_odak: Option<AyarOge>,
    pub ayar_kaydir: i32,
    pub ayar_paneli: Dik,
    /// Vurgu rengi notu (metin, uyarı mı).
    pub ayar_notu: Option<(String, bool)>,
    /// Ana ekranda Tab ile Ayarlar düğmesi odakta.
    pub ayarlar_odak: bool,
}

impl PencereDurumu {
    pub(crate) fn azalt(&self) -> bool {
        self.gorunum.hareketi_azalt_mi(self.sistem_animasyon)
    }

    /// Klavye odağındaki hedef (odak halkası çizilir).
    pub(crate) fn odakli_hedef(&self) -> Option<Hedef> {
        if self.ayarlar_acik {
            self.ayar_odak.map(Hedef::Ayar)
        } else if self.ayarlar_odak {
            Some(Hedef::Ayarlar)
        } else {
            None
        }
    }
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
            let buyuk = logo_simgesi(32);
            let wc = WNDCLASSW {
                style: 0,
                lpfnWndProc: Some(pencere_proc),
                cbClsExtra: 0,
                cbWndExtra: 0,
                hInstance: ornek,
                hIcon: if buyuk.is_null() { LoadIconW(null_mut(), IDI_APPLICATION) } else { buyuk },
                hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                hbrBackground: null_mut(),
                lpszMenuName: null(),
                lpszClassName: sinif.as_ptr(),
            };
            if RegisterClassW(&wc) == 0 {
                return Err("pencere sınıfı kaydedilemedi".into());
            }
            let gorunum = Gorunum::yukle();
            let (sk, yk, sa) = (sistem_koyu(), yuksek_karsitlik(), sistem_animasyon());
            let (tema, j) = etkin_jetonlar(&gorunum, sk, yk);
            let koyu = gorunum.modu.koyu_mu(sk);
            let durum = Box::new(PencereDurumu {
                uc,
                kartlar: kartlar::gomulu(),
                kart: 0,
                kart_zamani: Instant::now(),
                kart_alani: Dik::default(),
                kart_tik: u8::MAX,
                kart_duraklat: false,
                kart_h: None,
                dpi: 96,
                gorunum,
                sistem_koyu: sk,
                yuksek_karsitlik: yk,
                sistem_animasyon: sa,
                tema,
                koyu,
                j,
                eski_j: j,
                fontlar: Fontlar::yap(96, tema),
                hedefler: vec![],
                uzerinde: None,
                fare: (-1, -1),
                tik: 0,
                yukleme: Some(Yukleme::default()),
                tanitim: None,
                gecis: None,
                ekran_anahtari: (0, None, false),
                ayarlar_acik: false,
                ayar_odak: None,
                ayar_kaydir: 0,
                ayar_paneli: Dik::default(),
                ayar_notu: None,
                ayarlar_odak: false,
            });
            let baslik = genis("TAA Kur kurulumu");
            let hwnd = CreateWindowExW(0, sinif.as_ptr(), baslik.as_ptr(), WS_OVERLAPPEDWINDOW, CW_USEDEFAULT,
                CW_USEDEFAULT, GENISLIK, YUKSEKLIK, null_mut(), null_mut(), ornek, Box::into_raw(durum) as *const c_void);
            if hwnd.is_null() {
                return Err("pencere yaratılamadı".into());
            }
            let kucuk = logo_simgesi(16);
            if !kucuk.is_null() {
                SendMessageW(hwnd, WM_SETICON, ICON_SMALL as usize, kucuk as isize);
            }
            boyutla_ortala(hwnd);
            koyu_baslik(hwnd, koyu);
            ShowWindow(hwnd, SW_SHOW);
            SetTimer(hwnd, ZAMANLAYICI, taa_kur_tema::TIK_MS, None);
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

/// Geçme Temel logosundan çalışma zamanında küçük pencere simgesi (krem kare üstünde; İlmek açık renkleri).
/// Yaratılamazsa boş tutamak döner, sınıf varsayılan simgeyi kullanır.
unsafe fn logo_simgesi(boyut: i32) -> HICON {
    let ekran = GetDC(null_mut());
    if ekran.is_null() {
        return null_mut();
    }
    let dc = CreateCompatibleDC(ekran);
    let renk = CreateCompatibleBitmap(ekran, boyut, boyut);
    let maske = CreateBitmap(boyut, boyut, 1, 1, null());
    let j = ILMEK_ACIK;
    let eski = SelectObject(dc, renk);
    let c = Cizer { dc, dpi: 96 };
    c.doldur(Dik::yeni(0, 0, boyut, boyut), j.zemin);
    for (parca, nk) in logo::cokgenler(boyut, 0, 0) {
        c.cokgen(&nk, if parca == logo::Parca::Ana { j.metin } else { j.vurgu });
    }
    // Maske tümüyle siyah: simge tamamen opak (kare karo).
    SelectObject(dc, maske);
    let r = RECT { left: 0, top: 0, right: boyut, bottom: boyut };
    FillRect(dc, &r, GetStockObject(BLACK_BRUSH));
    SelectObject(dc, eski);
    let ii = ICONINFO { fIcon: 1, xHotspot: 0, yHotspot: 0, hbmMask: maske, hbmColor: renk };
    let h = CreateIconIndirect(&ii);
    DeleteObject(renk);
    DeleteObject(maske);
    DeleteDC(dc);
    ReleaseDC(null_mut(), ekran);
    h
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

// ------------------------------------------------------------------ görünüm

/// Tercih + sistem durumundan temayı ve renkleri yeniden kurar; `gecis` ise temanın geçiş hareketi oynar.
unsafe fn gorunumu_uygula(hwnd: HWND, d: &mut PencereDurumu, gecis: bool) {
    let (tema, j) = etkin_jetonlar(&d.gorunum, d.sistem_koyu, d.yuksek_karsitlik);
    let koyu = d.gorunum.modu.koyu_mu(d.sistem_koyu);
    if tema == d.tema && j == d.j && koyu == d.koyu {
        return;
    }
    if gecis {
        d.eski_j = d.j;
        d.gecis = Gecis::baslat(tema.hareket(), d.azalt());
    }
    if tema != d.tema {
        d.fontlar = Fontlar::yap(d.dpi.max(96), tema);
        d.kart_h = None;
    }
    if koyu != d.koyu {
        koyu_baslik(hwnd, koyu);
    }
    d.tema = tema;
    d.j = j;
    d.koyu = koyu;
    InvalidateRect(hwnd, null(), 0);
}

fn kaydet(d: &PencereDurumu) {
    // Kayıt başarısız olursa (salt okunur profil vb.) görünüm yine uygulanır; kurulum durmaz.
    let _ = d.gorunum.kaydet();
}

fn tanitim_bitir(d: &mut PencereDurumu) {
    d.tanitim = None;
    if !d.gorunum.tanitim_goruldu {
        d.gorunum.tanitim_goruldu = true;
        kaydet(d);
    }
}

/// Pencere dışı kalıcı (modal) işler: ödünç alınan durum bırakıldıktan sonra çalışır.
enum Modal {
    Renk,
    Varsayilan,
}

/// Ayarlar denetimini çalıştırır (tıklama, Enter, Boşluk). Seçim anında uygulanır ve kaydedilir.
unsafe fn ayar_etkinlestir(hwnd: HWND, d: &mut PencereDurumu, o: AyarOge) -> Option<Modal> {
    d.ayar_odak = Some(o);
    match o {
        AyarOge::Kapat => {
            d.ayarlar_acik = false;
            d.ayar_odak = None;
        }
        AyarOge::Tema(i) => {
            d.gorunum.tema = Some(Tema::HEPSI[i.min(2)]);
            d.ayar_notu = None;
            gorunumu_uygula(hwnd, d, true);
            kaydet(d);
        }
        AyarOge::Mod(i) => {
            d.gorunum.modu = Mod::HEPSI[i.min(2)];
            gorunumu_uygula(hwnd, d, true);
            kaydet(d);
        }
        AyarOge::RenkSec => return Some(Modal::Renk),
        AyarOge::RengeDon => {
            d.gorunum.ozel_vurgu_ata(d.tema, None);
            d.ayar_notu = None;
            gorunumu_uygula(hwnd, d, true);
            kaydet(d);
        }
        AyarOge::HareketAzalt => {
            let simdi = d.azalt();
            d.gorunum.hareketi_azalt = Some(!simdi);
            if !simdi {
                d.gecis = None;
            }
            kaydet(d);
        }
        AyarOge::TanitimTekrar => {
            d.ayarlar_acik = false;
            d.ayar_odak = None;
            d.tanitim = Some(Tanitim::yeni());
        }
        AyarOge::Varsayilan => return Some(Modal::Varsayilan),
    }
    None
}

/// Modal iletişim kutuları. Kutu açıkken pencere iletileri (zamanlayıcı, boyama) sürer; bu yüzden durum
/// burada yeniden alınır, çağıran taraf ödünç almayı bırakmış olmalıdır.
unsafe fn modal_calistir(hwnd: HWND, m: Modal) {
    match m {
        Modal::Renk => {
            let Some(d) = durum_al(hwnd) else { return };
            let (tema, koyu, simdiki) = (d.tema, d.koyu, d.j.vurgu);
            let mut ozel = [colorref(0xFFFFFF); 16];
            for (k, t) in Tema::HEPSI.iter().enumerate() {
                if let Some(r) = d.gorunum.ozel_vurgu(*t) {
                    ozel[k] = colorref(r);
                }
            }
            let mut cc: CHOOSECOLORW = std::mem::zeroed();
            cc.lStructSize = std::mem::size_of::<CHOOSECOLORW>() as u32;
            cc.hwndOwner = hwnd;
            cc.rgbResult = colorref(simdiki);
            cc.lpCustColors = ozel.as_mut_ptr();
            cc.Flags = CC_RGBINIT | CC_FULLOPEN;
            if ChooseColorW(&mut cc) == 0 {
                return;
            }
            let secilen = colorref_rgb(cc.rgbResult);
            let Some(d) = durum_al(hwnd) else { return };
            match vurgu_duzelt(tema, koyu, secilen) {
                Ok((_, duzeltildi)) => {
                    d.gorunum.ozel_vurgu_ata(tema, Some(secilen));
                    let mut not = String::new();
                    if duzeltildi {
                        not.push_str("Okunabilirlik için renk biraz koyulaştırıldı.");
                    }
                    let yakin = hataya_yakin(tema, koyu, secilen);
                    if yakin {
                        if !not.is_empty() {
                            not.push(' ');
                        }
                        not.push_str("Bu renk hata rengine çok yakın; hata durumlarıyla karışabilir.");
                    }
                    d.ayar_notu = if not.is_empty() { None } else { Some((not, yakin)) };
                }
                Err(e) => d.ayar_notu = Some((e.to_string(), true)),
            }
            gorunumu_uygula(hwnd, d, true);
            kaydet(d);
        }
        Modal::Varsayilan => {
            let m = genis("Görünüm tercihleri varsayılana dönsün mü?\n\nTema, görünüm modu, vurgu rengi ve hareket ayarı sıfırlanır. Rol, şube ve program seçimlerine dokunulmaz.");
            let b = genis("TAA Kur kurulumu");
            if MessageBoxW(hwnd, m.as_ptr(), b.as_ptr(), MB_YESNO | MB_ICONQUESTION) != IDYES {
                return;
            }
            let Some(d) = durum_al(hwnd) else { return };
            d.gorunum.varsayilana_don();
            d.ayar_notu = None;
            gorunumu_uygula(hwnd, d, true);
            kaydet(d);
        }
    }
    InvalidateRect(hwnd, null(), 0);
}

// ------------------------------------------------------------------ akış (değişmedi)

fn icinde(r: &Dik, x: i32, y: i32) -> bool {
    r.icinde(x, y)
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
    d.kart_tik = 0;
}

fn hedef_bul(d: &PencereDurumu, x: i32, y: i32) -> Option<Hedef> {
    d.hedefler.iter().rev().find(|(r, _)| icinde(r, x, y)).map(|(_, h)| *h)
}

unsafe fn tikla(hwnd: HWND, d: &mut PencereDurumu, x: i32, y: i32) -> Option<Modal> {
    let h = hedef_bul(d, x, y)?;
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
        Hedef::Grup(i) => {
            secim_degistir(d, |s| {
                if let Some(g) = s.gruplar().get(i).cloned() {
                    s.grup_degistir(&g);
                }
            });
        }
        Hedef::KartGeri => kart_cevir(d, false),
        Hedef::KartIleri => kart_cevir(d, true),
        Hedef::KartDuraklat => d.kart_duraklat = !d.kart_duraklat,
        Hedef::Ayarlar => {
            d.ayarlar_acik = !d.ayarlar_acik;
            d.ayar_odak = None;
            d.ayar_kaydir = 0;
        }
        Hedef::Ayar(o) => return ayar_etkinlestir(hwnd, d, o),
        Hedef::TanitimIleri => {
            if let Some(t) = d.tanitim.as_mut() {
                if t.ileri() {
                    tanitim_bitir(d);
                }
            }
        }
        Hedef::TanitimAtla => {
            if let Some(t) = d.tanitim.as_mut() {
                t.atla();
            }
            tanitim_bitir(d);
        }
        Hedef::Bos => {}
    }
    None
}

/// Klavye. Tanıtım ve Ayarlar paneli açıkken tuşlar onlarındır; ana ekranda akış tuşları değişmedi.
unsafe fn tus(hwnd: HWND, d: &mut PencereDurumu, k: u16) -> Option<Modal> {
    if d.yukleme.is_some() {
        return None;
    }
    if let Some(t) = d.tanitim.as_mut() {
        if k == VK_RETURN || k == VK_RIGHT || k == VK_SPACE {
            if t.ileri() {
                tanitim_bitir(d);
            }
        } else if k == VK_LEFT {
            t.geri();
        } else if k == VK_ESCAPE {
            t.atla();
            tanitim_bitir(d);
        }
        return None;
    }
    if d.ayarlar_acik {
        let geri = GetKeyState(VK_SHIFT as i32) < 0;
        if k == VK_TAB {
            d.ayar_odak = Some(y::ayar_sonraki(d.ayar_odak, geri));
        } else if k == VK_DOWN {
            d.ayar_odak = Some(y::ayar_sonraki(d.ayar_odak, false));
        } else if k == VK_UP {
            d.ayar_odak = Some(y::ayar_sonraki(d.ayar_odak, true));
        } else if k == VK_LEFT || k == VK_RIGHT {
            if let Some(o) = d.ayar_odak {
                let yeni = y::ayar_grupta(o, k == VK_RIGHT);
                if yeni != o {
                    return ayar_etkinlestir(hwnd, d, yeni);
                }
            }
        } else if k == VK_RETURN || k == VK_SPACE {
            if let Some(o) = d.ayar_odak {
                return ayar_etkinlestir(hwnd, d, o);
            }
        } else if k == VK_ESCAPE {
            // Odak Ayarlar düğmesine dönmez: ardından gelen Enter yine akışın onayıdır.
            d.ayarlar_acik = false;
            d.ayar_odak = None;
        }
        return None;
    }
    if k == VK_TAB {
        d.ayarlar_odak = !d.ayarlar_odak;
        return None;
    }
    if d.ayarlar_odak && (k == VK_RETURN || k == VK_SPACE) {
        d.ayarlar_acik = true;
        d.ayarlar_odak = false;
        d.ayar_odak = Some(AyarOge::Kapat);
        d.ayar_kaydir = 0;
        return None;
    }
    d.ayarlar_odak = false;
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
    None
}

/// Zamanlayıcı tiki: yükleme, tanıtım, geçiş, kart dönüşü.
unsafe fn zamanlayici(hwnd: HWND, d: &mut PencereDurumu) {
    d.tik = d.tik.wrapping_add(1);
    let (bitti, hazir, anahtar) = match d.uc.durum.lock() {
        Ok(p) => (
            p.cikis.is_some(),
            cizim::hazir_mi(&p),
            (p.ilerleme.asama.sira(), p.secim.as_ref().map(|s| s.baslik.clone()), p.hata.is_some()),
        ),
        Err(_) => (true, true, (0, None, false)),
    };
    if bitti {
        DestroyWindow(hwnd);
        return;
    }
    let azalt = d.azalt();
    if let Some(yk) = d.yukleme.as_mut() {
        yk.tikle();
        if yk.bitti_mi(hazir) {
            d.yukleme = None;
            d.ekran_anahtari = anahtar.clone();
            if !d.gorunum.tanitim_goruldu {
                d.tanitim = Some(Tanitim::yeni());
            }
        }
    }
    if let Some(t) = d.tanitim.as_mut() {
        t.tikle();
    }
    if let Some(g) = d.gecis.as_mut() {
        if !g.ilerlet() {
            d.gecis = None;
        }
    }
    if d.yukleme.is_none() && anahtar != d.ekran_anahtari {
        d.ekran_anahtari = anahtar;
        if d.gecis.is_none() {
            // Ekran değişimi: İlmek'te içerik yerinde, renkler zeminden açılır; Kalıp oturur; Mezura alt çizgi uzar.
            let mut e = d.j;
            e.yuzey = e.zemin;
            e.yuzey2 = e.zemin;
            e.kenar = taa_kur_tema::ara_renk(e.kenar, e.zemin, 0.5);
            e.vurgu = taa_kur_tema::ara_renk(e.vurgu, e.zemin, 0.5);
            d.eski_j = e;
            d.gecis = Gecis::baslat(d.tema.hareket(), azalt);
        }
    }
    d.kart_tik = d.kart_tik.saturating_add(1);
    let durgun = icinde(&d.kart_alani, d.fare.0, d.fare.1) || d.kart_duraklat || azalt || d.ayarlar_acik || d.tanitim.is_some();
    if durgun {
        // Üstüne gelince / duraklatılınca durur; bırakınca süre baştan sayılır.
        d.kart_zamani = Instant::now();
    } else if d.kart_zamani.elapsed().as_millis() as u64 >= d.tema.tanitim_ms() as u64 {
        kart_cevir(d, true);
    }
    InvalidateRect(hwnd, null(), 0);
}

unsafe extern "system" fn pencere_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let cs = lp as *const CREATESTRUCTW;
        SetWindowLongPtrW(hwnd, GWLP_USERDATA, (*cs).lpCreateParams as isize);
        if let Some(d) = durum_al(hwnd) {
            d.dpi = GetDpiForWindow(hwnd).max(96);
            d.fontlar = Fontlar::yap(d.dpi, d.tema);
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
    if durum_al(hwnd).is_none() {
        return DefWindowProcW(hwnd, msg, wp, lp);
    }
    match msg {
        WM_PAINT => {
            if let Some(d) = durum_al(hwnd) {
                boya(hwnd, d);
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_SIZE => {
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_TIMER => {
            if let Some(d) = durum_al(hwnd) {
                zamanlayici(hwnd, d);
            }
            0
        }
        WM_KEYDOWN => {
            let modal = durum_al(hwnd).and_then(|d| tus(hwnd, d, wp as u16));
            if let Some(m) = modal {
                modal_calistir(hwnd, m);
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_MOUSEWHEEL => {
            if let Some(d) = durum_al(hwnd) {
                let delta = ((wp >> 16) & 0xFFFF) as u16 as i16;
                // Tekerlek konumu ekran koordinatıdır; panel açıkken panel kaydırılır.
                if d.ayarlar_acik {
                    d.ayar_kaydir -= delta as i32 * d.dpi as i32 / 96 / 3;
                    d.ayar_odak = None;
                } else if delta > 0 {
                    secim_degistir(d, Secim::yukari);
                } else if delta < 0 {
                    secim_degistir(d, Secim::asagi);
                }
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_MOUSEMOVE => {
            if let Some(d) = durum_al(hwnd) {
                let (x, y) = ((lp & 0xFFFF) as u16 as i16 as i32, ((lp >> 16) & 0xFFFF) as u16 as i16 as i32);
                d.fare = (x, y);
                let h = hedef_bul(d, x, y).filter(|h| *h != Hedef::Bos);
                if h != d.uzerinde {
                    d.uzerinde = h;
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_SETCURSOR => {
            if (lp & 0xFFFF) as u32 == HTCLIENT && durum_al(hwnd).is_some_and(|d| d.uzerinde.is_some()) {
                SetCursor(LoadCursorW(null_mut(), IDC_HAND));
                return 1;
            }
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        WM_LBUTTONUP => {
            let (x, y) = ((lp & 0xFFFF) as u16 as i16 as i32, ((lp >> 16) & 0xFFFF) as u16 as i16 as i32);
            let modal = durum_al(hwnd).and_then(|d| {
                d.ayarlar_odak = false;
                tikla(hwnd, d, x, y)
            });
            if let Some(m) = modal {
                modal_calistir(hwnd, m);
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_CLOSE => {
            if let Some(d) = durum_al(hwnd) {
                kapat_sor(hwnd, d);
            }
            0
        }
        WM_DPICHANGED => {
            if let Some(d) = durum_al(hwnd) {
                d.dpi = ((wp >> 16) & 0xFFFF) as u32;
                d.fontlar = Fontlar::yap(d.dpi.max(96), d.tema);
                d.kart_h = None;
            }
            let r = &*(lp as *const RECT);
            SetWindowPos(hwnd, null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER);
            0
        }
        WM_GETMINMAXINFO => {
            let mm = &mut *(lp as *mut MINMAXINFO);
            let s = durum_al(hwnd).map(|d| d.dpi).unwrap_or(96).max(96) as i32;
            mm.ptMinTrackSize = POINT { x: EN_AZ.0 * s / 96, y: EN_AZ.1 * s / 96 };
            0
        }
        WM_SETTINGCHANGE => {
            if let Some(d) = durum_al(hwnd) {
                d.sistem_koyu = sistem_koyu();
                d.yuksek_karsitlik = yuksek_karsitlik();
                d.sistem_animasyon = sistem_animasyon();
                gorunumu_uygula(hwnd, d, false);
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
        cizim::ciz(&Cizer { dc: mdc, dpi: d.dpi.max(96) as i32 }, d, &p, w, h);
    }
    BitBlt(hdc, 0, 0, w, h, mdc, 0, 0, SRCCOPY);
    SelectObject(mdc, eski);
    DeleteObject(bmp);
    DeleteDC(mdc);
    EndPaint(hwnd, &ps);
}
