// Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]
//! Windows adlandırılmış boru ucu (`#[cfg(windows)]`). UYARI: bu dosya Linux'ta derlenemez; T3'te
//! yalnız `cargo check --target x86_64-pc-windows-gnu` ile tür denetiminden geçirildi, Windows'ta
//! ÇALIŞTIRILIP DOĞRULANMADI (CI `windows-latest` ilk gerçek sınavdır).
//!
//! Tasarım (mantık `boru_ortak.rs`'te, burada yalnız sistem çağrıları var):
//! * Sunucu: `CreateNamedPipeW` bayt kipi, `PIPE_REJECT_REMOTE_CLIENTS` (uzaktan bağlantı yok),
//!   ilk örnek `FILE_FLAG_FIRST_PIPE_INSTANCE` (ad başkasındaysa `Dolu`: kimlik sahtekârlığı
//!   engeli), güvenlik tanımlayıcısı yalnız geçerli kullanıcı (`acl_sddl`).
//! * İstemci: `CreateFileW`, `SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION` (sunucu istemciyi
//!   taklit edemez); bağlanınca `GetNamedPipeServerProcessId` kayıttaki pid ile karşılaştırılır,
//!   uyuşmazsa HİÇBİR ŞEY yazılmaz.
//! * Bütün G/Ç eşzamansız (`FILE_FLAG_OVERLAPPED`) ve toplam süreyle sınırlı (`Sure`); süre dolunca
//!   `CancelIoEx`. Bir bağlantı = bir çerçeve + bir onay (bağlantı başına bir iş parçacığı, en
//!   çok `EN_COK_BAGLANTI`).

use super::boru_ortak::{
    acl_sddl, boru_adi, cerceve_oku, cerceve_yaz, kullanici_kimligi_temizle, sunucu_isle,
    BoruHatasi, BoruUcu, Onay, Sure, YedekliTasiyici, BORU_ZAMAN_ASIMI,
};
use super::{GelenKutusu, IpcHata};
use std::collections::HashMap;
use std::ffi::{c_void, OsStr};
use std::io::{self, Read, Write};
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, LocalFree, ERROR_ACCESS_DENIED, ERROR_BROKEN_PIPE,
    ERROR_FILE_NOT_FOUND, ERROR_IO_PENDING, ERROR_OPERATION_ABORTED, ERROR_PIPE_BUSY,
    ERROR_PIPE_CONNECTED, GENERIC_READ, GENERIC_WRITE, HANDLE, INVALID_HANDLE_VALUE, WAIT_OBJECT_0,
    WAIT_TIMEOUT,
};
use windows_sys::Win32::Security::Authorization::{
    ConvertSidToStringSidW, ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    GetTokenInformation, TokenUser, PSECURITY_DESCRIPTOR, SECURITY_ATTRIBUTES, TOKEN_QUERY,
    TOKEN_USER,
};
use windows_sys::Win32::Storage::FileSystem::{
    CreateFileW, ReadFile, WriteFile, FILE_FLAG_FIRST_PIPE_INSTANCE, FILE_FLAG_OVERLAPPED,
    OPEN_EXISTING, PIPE_ACCESS_DUPLEX, SECURITY_IDENTIFICATION, SECURITY_SQOS_PRESENT,
};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe, GetNamedPipeServerProcessId,
    WaitNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};
use windows_sys::Win32::System::Threading::{
    CreateEventW, GetCurrentProcess, OpenProcessToken, WaitForSingleObject,
};
use windows_sys::Win32::System::IO::{CancelIoEx, GetOverlappedResult, OVERLAPPED};

/// Aynı anda işlenen en çok bağlantı; fazlası hemen kapatılır (istemci dosyaya düşer).
const EN_COK_BAGLANTI: usize = 16;
/// Kabul döngüsünün durdurma bayrağına bakma aralığı.
const KABUL_ARALIGI: Duration = Duration::from_millis(500);
/// Boru arabellek boyutu (çerçeve sınırından küçük olabilir; bayt akışıdır).
const ARABELLEK: u32 = 64 * 1024;

/// Windows boru taşıyıcısı: boru dener, olmazsa dosya tabanlı `GelenKutusu`'na düşer.
pub type BoruTasiyici = YedekliTasiyici<WindowsBoru>;

impl YedekliTasiyici<WindowsBoru> {
    /// Verilen dosya kutusuyla kurar (geçerli kullanıcının SID'i okunur).
    pub fn boruyla(kutu: GelenKutusu) -> Result<BoruTasiyici, IpcHata> {
        Ok(YedekliTasiyici::yeni(kutu, WindowsBoru::yeni()?))
    }

    /// Kök sırası `GelenKutusu::ortamdan` ile aynıdır.
    pub fn boruyla_ortamdan() -> Result<BoruTasiyici, IpcHata> {
        Self::boruyla(GelenKutusu::ortamdan()?)
    }
}

// ---------------------------------------------------------------------------------------------
// Küçük yardımcılar (her unsafe bloğu kendi içinde gerekçelidir)
// ---------------------------------------------------------------------------------------------

/// Son Win32 hatasını `io::Error` yapar.
fn son_hata() -> io::Error {
    // SAFETY: GetLastError argümansızdır ve her zaman güvenle çağrılır.
    io::Error::from_raw_os_error(unsafe { GetLastError() } as i32)
}

fn win_hata(kod: u32) -> io::Error {
    io::Error::from_raw_os_error(kod as i32)
}

/// UTF-16, sonu sıfırla biten metin (Win32 `PCWSTR`).
fn genis(s: &str) -> Vec<u16> {
    OsStr::new(s)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

/// Sahip olunan tutamaç; düşünce kapatılır.
struct Tutamac(HANDLE);

impl Drop for Tutamac {
    fn drop(&mut self) {
        // SAFETY: `self.0` bu yapı tarafından sahip olunan, geçerli bir tutamaçtır ve yalnız burada
        // bir kez kapatılır.
        unsafe { CloseHandle(self.0) };
    }
}

/// `LocalFree` ile serbest bırakılacak güvenlik tanımlayıcısı.
struct Guvenlik(PSECURITY_DESCRIPTOR);

impl Drop for Guvenlik {
    fn drop(&mut self) {
        // SAFETY: işaretçi `ConvertStringSecurityDescriptorToSecurityDescriptorW` tarafından
        // LocalAlloc ile ayrıldı ve yalnız burada bir kez serbest bırakılır.
        unsafe { LocalFree(self.0) };
    }
}

/// Geçerli sürecin kullanıcı SID'i (metin, `S-1-5-21-...`).
fn kullanici_sid() -> io::Result<String> {
    let mut belirtec: HANDLE = null_mut();
    // SAFETY: GetCurrentProcess sahte tutamaç döner (kapatılmaz); `belirtec` geçerli bir çıktı
    // işaretçisidir.
    if unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut belirtec) } == 0 {
        return Err(son_hata());
    }
    let belirtec = Tutamac(belirtec);
    let mut gerekli = 0u32;
    // SAFETY: boyut sorgusu; tampon null ve uzunluk 0 verilir, `gerekli` geçerli bir çıktıdır.
    // Başarısız olması (yetersiz tampon) beklenir; yalnız `gerekli` kullanılır.
    unsafe { GetTokenInformation(belirtec.0, TokenUser, null_mut(), 0, &mut gerekli) };
    if gerekli == 0 {
        return Err(son_hata());
    }
    // u64 dizisi: TOKEN_USER işaretçi hizası ister.
    let mut tampon = vec![0u64; (gerekli as usize).div_ceil(8)];
    // SAFETY: tampon `gerekli` baytı karşılar (u64 sayısı yukarı yuvarlandı) ve hizalıdır.
    let tamam = unsafe {
        GetTokenInformation(
            belirtec.0,
            TokenUser,
            tampon.as_mut_ptr().cast::<c_void>(),
            (tampon.len() * 8) as u32,
            &mut gerekli,
        )
    };
    if tamam == 0 {
        return Err(son_hata());
    }
    let mut metin: *mut u16 = null_mut();
    // SAFETY: tampon başarılı çağrıyla doldu; başında hizalı bir TOKEN_USER var ve `User.Sid`
    // tampon içini gösterir (tampon bu çağrı boyunca yaşar). `metin` geçerli bir çıktıdır.
    let tamam = unsafe {
        let kullanici = &*tampon.as_ptr().cast::<TOKEN_USER>();
        ConvertSidToStringSidW(kullanici.User.Sid, &mut metin)
    };
    if tamam == 0 || metin.is_null() {
        return Err(son_hata());
    }
    // SAFETY: `metin`, sıfır sonlu, LocalAlloc ile ayrılmış UTF-16 dizgisini gösterir; uzunluk
    // sıfıra kadar sayılır, kopyalandıktan sonra bir kez LocalFree edilir.
    let sid = unsafe {
        let mut uzunluk = 0usize;
        while *metin.add(uzunluk) != 0 {
            uzunluk += 1;
        }
        let s = String::from_utf16_lossy(std::slice::from_raw_parts(metin, uzunluk));
        LocalFree(metin.cast::<c_void>());
        s
    };
    Ok(sid)
}

/// SDDL metninden güvenlik tanımlayıcısı üretir.
fn guvenlik_olustur(sddl: &str) -> io::Result<Guvenlik> {
    let sddl_w = genis(sddl);
    let mut sd: PSECURITY_DESCRIPTOR = null_mut();
    // SAFETY: `sddl_w` sıfır sonlu ve çağrı boyunca yaşar; `sd` geçerli çıktı işaretçisi, boyut
    // çıktısı istenmediği için null verilebilir.
    let tamam = unsafe {
        ConvertStringSecurityDescriptorToSecurityDescriptorW(
            sddl_w.as_ptr(),
            SDDL_REVISION_1,
            &mut sd,
            null_mut(),
        )
    };
    if tamam == 0 {
        return Err(son_hata());
    }
    Ok(Guvenlik(sd))
}

// ---------------------------------------------------------------------------------------------
// Süreli eşzamansız G/Ç
// ---------------------------------------------------------------------------------------------

/// Eşzamansız (overlapped) bir G/Ç işlemini başlatır ve `sure` dolana kadar bekler. Süre dolarsa
/// işlemi iptal eder ve iptalin bitmesini bekler (çağıranın arabellekleri ve `ov` işlem bitmeden
/// düşmez). `baslat` işlemi başlatan çağrıyı yapar ve BOOL döner.
fn sureli(
    tutamac: HANDLE,
    sure: &Sure,
    baslat: impl FnOnce(*mut OVERLAPPED) -> i32,
) -> io::Result<u32> {
    // SAFETY: tüm argümanlar null/varsayılan; elle sıfırlanabilir olay (manual reset).
    let olay = unsafe { CreateEventW(null(), 1, 0, null()) };
    if olay.is_null() {
        return Err(son_hata());
    }
    let olay = Tutamac(olay);
    // SAFETY: OVERLAPPED düz veri yapısıdır; sıfır başlangıç değeri belgelenen kullanımdır.
    let mut ov: OVERLAPPED = unsafe { std::mem::zeroed() };
    ov.hEvent = olay.0;
    if baslat(&mut ov) == 0 {
        // SAFETY: argümansız.
        let kod = unsafe { GetLastError() };
        if kod != ERROR_IO_PENDING {
            return Err(win_hata(kod));
        }
        // SAFETY: `olay.0` geçerli bir olay tutamacı.
        let bekleme = unsafe { WaitForSingleObject(olay.0, sure.kalan_ms()) };
        if bekleme != WAIT_OBJECT_0 {
            // Süre doldu (ya da bekleme hatası): işlemi iptal et ve bitmesini BEKLE; böylece `ov`
            // ve arabellek işlem sürerken serbest kalmaz.
            let neden = if bekleme == WAIT_TIMEOUT {
                io::Error::from(io::ErrorKind::TimedOut)
            } else {
                son_hata()
            };
            let mut n = 0u32;
            // SAFETY: `tutamac` ve `ov` işlemi başlatanla aynı; son argüman 1 = bitene kadar bekle.
            let sonuc = unsafe {
                CancelIoEx(tutamac, &ov);
                GetOverlappedResult(tutamac, &ov, &mut n, 1)
            };
            if sonuc != 0 {
                // İptalden hemen önce işlem tamamlanmış: sonucu kullan.
                return Ok(n);
            }
            return Err(neden);
        }
    }
    let mut n = 0u32;
    // SAFETY: işlem tamamlandı; bekleme yok (0).
    if unsafe { GetOverlappedResult(tutamac, &ov, &mut n, 0) } == 0 {
        return Err(son_hata());
    }
    Ok(n)
}

/// Eşzamansız boru ucu; `Read`/`Write` uygular (çerçeveleme bunun üstünde çalışır).
struct BoruG {
    tutamac: Tutamac,
    sure: Sure,
    /// Sunucu ucu ise kapatmadan önce `DisconnectNamedPipe` çağrılır.
    sunucu: bool,
}

impl BoruG {
    fn yeni(tutamac: HANDLE, sure: Sure, sunucu: bool) -> BoruG {
        BoruG {
            tutamac: Tutamac(tutamac),
            sure,
            sunucu,
        }
    }
}

impl Read for BoruG {
    fn read(&mut self, tampon: &mut [u8]) -> io::Result<usize> {
        let uzunluk = tampon.len().min(u32::MAX as usize) as u32;
        let ptr = tampon.as_mut_ptr();
        let h = self.tutamac.0;
        // SAFETY: `ptr` en az `uzunluk` baytlık tamponu gösterir ve `sureli` dönmeden (işlem
        // bitmeden ya da iptal edilip bitmeden) tampon düşmez; `ov` sureli'nin yığınındadır.
        let sonuc = sureli(h, &self.sure, |ov| unsafe {
            ReadFile(h, ptr, uzunluk, null_mut(), ov)
        });
        match sonuc {
            Ok(n) => Ok(n as usize),
            // Karşı uç kapandı: akış sonu.
            Err(e) if e.raw_os_error() == Some(ERROR_BROKEN_PIPE as i32) => Ok(0),
            Err(e) if e.raw_os_error() == Some(ERROR_OPERATION_ABORTED as i32) => {
                Err(io::Error::from(io::ErrorKind::TimedOut))
            }
            Err(e) => Err(e),
        }
    }
}

impl Write for BoruG {
    fn write(&mut self, veri: &[u8]) -> io::Result<usize> {
        let uzunluk = veri.len().min(u32::MAX as usize) as u32;
        let ptr = veri.as_ptr();
        let h = self.tutamac.0;
        // SAFETY: `ptr` en az `uzunluk` baytı gösterir ve `sureli` dönene kadar yaşar (bkz. Read).
        let sonuc = sureli(h, &self.sure, |ov| unsafe {
            WriteFile(h, ptr, uzunluk, null_mut(), ov)
        });
        match sonuc {
            Ok(n) => Ok(n as usize),
            Err(e) if e.raw_os_error() == Some(ERROR_OPERATION_ABORTED as i32) => {
                Err(io::Error::from(io::ErrorKind::TimedOut))
            }
            Err(e) => Err(e),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        // Bayt kipinde her WriteFile tamamlanınca veri borudadır; FlushFileBuffers sınırsız
        // bloklayabileceğinden kullanılmaz (karşı uç okuyana kadar beklerdi).
        Ok(())
    }
}

impl Drop for BoruG {
    fn drop(&mut self) {
        if self.sunucu {
            // SAFETY: geçerli sunucu tutamacı; bağlı değilse çağrı hata döner, önemsiz.
            unsafe { DisconnectNamedPipe(self.tutamac.0) };
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Sunucu
// ---------------------------------------------------------------------------------------------

/// Bir boru örneği oluşturur. `ilk`: ilk örnek (ad başkasındaysa `ERROR_ACCESS_DENIED`).
fn ornek_olustur(ad: &[u16], sa: &SECURITY_ATTRIBUTES, ilk: bool) -> io::Result<BoruG> {
    let mut acma = PIPE_ACCESS_DUPLEX | FILE_FLAG_OVERLAPPED;
    if ilk {
        acma |= FILE_FLAG_FIRST_PIPE_INSTANCE;
    }
    // SAFETY: `ad` sıfır sonlu; `sa` geçerli bir SECURITY_ATTRIBUTES (içindeki tanımlayıcı çağıran
    // tarafından çağrı boyunca canlı tutulur). Kopyalanır, işaretçi saklanmaz.
    let h = unsafe {
        CreateNamedPipeW(
            ad.as_ptr(),
            acma,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
            PIPE_UNLIMITED_INSTANCES,
            ARABELLEK,
            ARABELLEK,
            0,
            sa,
        )
    };
    if h == INVALID_HANDLE_VALUE {
        return Err(son_hata());
    }
    Ok(BoruG::yeni(h, Sure::yeni(BORU_ZAMAN_ASIMI), true))
}

/// Bir istemci bağlanana ya da `durdur` işaretlenene kadar bekler. `true`: bağlandı.
fn baglanti_bekle(g: &BoruG, durdur: &AtomicBool) -> io::Result<bool> {
    loop {
        if durdur.load(Ordering::Relaxed) {
            return Ok(false);
        }
        let sure = Sure::yeni(KABUL_ARALIGI);
        let h = g.tutamac.0;
        // SAFETY: geçerli sunucu tutamacı ve `sureli`nin yığındaki OVERLAPPED'ı.
        let sonuc = sureli(h, &sure, |ov| unsafe { ConnectNamedPipe(h, ov) });
        match sonuc {
            Ok(_) => return Ok(true),
            // İstemci, ConnectNamedPipe çağrısından önce bağlanmıştı.
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_CONNECTED as i32) => return Ok(true),
            // Süre/iptal: durdurma bayrağına bakıp yeniden dene.
            Err(e)
                if e.kind() == io::ErrorKind::TimedOut
                    || e.raw_os_error() == Some(ERROR_OPERATION_ABORTED as i32) => {}
            Err(e) => return Err(e),
        }
    }
}

/// Ham tutamaç ve güvenlik tanımlayıcısı iş parçacığına taşınırken sarmalayıcı.
struct Tasinan<T>(T);
// SAFETY: sarılan tutamaç/tanımlayıcı yalnız TEK iş parçacığına devredilir, eşzamanlı paylaşılmaz;
// Win32 tutamaçları iş parçacıkları arasında kullanılabilir.
unsafe impl<T> Send for Tasinan<T> {}

impl<T> Tasinan<T> {
    /// İçeriği alır. Kapanışlarda alanı doğrudan desenle açmak yerine bu yöntem kullanılır:
    /// (edition 2021'de ayrık yakalama sarmalayıcıyı atlayıp `Send` olmayan alanı yakalardı.)
    fn ac(self) -> T {
        self.0
    }
}

/// Tek bağlantıyı işler: çerçeve oku, `sunucu_isle`, onay yaz, istemcinin kapanmasını bekle.
fn baglanti_isle(mut g: BoruG, kutu: &GelenKutusu, program: &str) {
    let onay = match cerceve_oku(&mut g) {
        Ok(govde) => sunucu_isle(kutu, program, &govde),
        Err(e) => Onay::red(&IpcHata::from(e)),
    };
    let Ok(onay_govdesi) = serde_json::to_vec(&onay) else {
        return;
    };
    if cerceve_yaz(&mut g, &onay_govdesi).is_ok() {
        // İstemci onayı okuyup kapatana (akış sonu) ya da süre dolana kadar bekle; sonra
        // `Drop` bağlantıyı keser. Erken kesmek onayın kaybolmasına yol açar.
        let mut bos = [0u8; 1];
        let _ = g.read(&mut bos);
    }
}

fn sunucu_dongusu(
    ad: Vec<u16>,
    ilk: Tasinan<(BoruG, Guvenlik)>,
    kutu: GelenKutusu,
    program: String,
    durdur: Arc<AtomicBool>,
) {
    let (mut simdiki, guvenlik) = ilk.ac();
    let sa = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: guvenlik.0,
        bInheritHandle: 0,
    };
    let aktif = Arc::new(AtomicUsize::new(0));
    loop {
        match baglanti_bekle(&simdiki, &durdur) {
            Ok(true) => {}
            Ok(false) => return,
            Err(e) => {
                kutu.gunluk(
                    "boru_baglanti_hatasi",
                    &kutu.kok().join(&program),
                    &e.to_string(),
                );
                // Bu örnek bozuk; yenisini aşağıda kuracağız.
            }
        }
        // Yeni istemciler için bir sonraki örneği HEMEN kur (bağlantı işlenirken ad boş kalmasın).
        let sonraki = loop {
            match ornek_olustur(&ad, &sa, false) {
                Ok(g) => break g,
                Err(e) => {
                    kutu.gunluk(
                        "boru_ornek_hatasi",
                        &kutu.kok().join(&program),
                        &e.to_string(),
                    );
                    if durdur.load(Ordering::Relaxed) {
                        return;
                    }
                    thread::sleep(KABUL_ARALIGI);
                }
            }
        };
        let baglanan = std::mem::replace(&mut simdiki, sonraki);
        if aktif.load(Ordering::Relaxed) >= EN_COK_BAGLANTI {
            // Aşırı yük: kapat (Drop); istemci dosyaya düşer.
            continue;
        }
        aktif.fetch_add(1, Ordering::Relaxed);
        let (kutu2, program2, aktif2) = (kutu.clone(), program.clone(), Arc::clone(&aktif));
        let tasinan = Tasinan(baglanan);
        let basladi = thread::Builder::new()
            .name(format!("taa-boru-{program}"))
            .spawn(move || {
                let g = tasinan.ac();
                baglanti_isle(g, &kutu2, &program2);
                aktif2.fetch_sub(1, Ordering::Relaxed);
            });
        if basladi.is_err() {
            aktif.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

// ---------------------------------------------------------------------------------------------
// BoruUcu
// ---------------------------------------------------------------------------------------------

/// Windows boru ucu: geçerli kullanıcının SID'iyle adlandırılmış borular.
pub struct WindowsBoru {
    sid: String,
    kullanici: String,
    sunucular: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl WindowsBoru {
    /// Geçerli kullanıcının SID'ini okuyup kurar.
    pub fn yeni() -> Result<WindowsBoru, IpcHata> {
        let sid =
            kullanici_sid().map_err(|e| IpcHata::Io(format!("kullanıcı SID'i okunamadı: {e}")))?;
        let kullanici = kullanici_kimligi_temizle(&sid)?;
        // SDDL kararı baştan doğrulanır (geçersiz SID erken hata verir).
        acl_sddl(&sid)?;
        Ok(WindowsBoru {
            sid,
            kullanici,
            sunucular: Mutex::new(HashMap::new()),
        })
    }
}

impl Drop for WindowsBoru {
    fn drop(&mut self) {
        // Kabul döngüleri en geç KABUL_ARALIGI içinde durur (beklenmez).
        if let Ok(m) = self.sunucular.lock() {
            for d in m.values() {
                d.store(true, Ordering::Relaxed);
            }
        }
    }
}

/// Boruya istemci olarak bağlanır; süre içinde meşgulse bekler.
fn baglan(ad: &[u16], sure: &Sure) -> Result<Tutamac, BoruHatasi> {
    loop {
        // SAFETY: `ad` sıfır sonlu. Güvenlik özniteliği ve şablon null. SQOS bayrağı: sunucu
        // istemci kimliğini yalnız tanımlama düzeyinde görür, taklit edemez.
        let h = unsafe {
            CreateFileW(
                ad.as_ptr(),
                GENERIC_READ | GENERIC_WRITE,
                0,
                null(),
                OPEN_EXISTING,
                FILE_FLAG_OVERLAPPED | SECURITY_SQOS_PRESENT | SECURITY_IDENTIFICATION,
                null_mut(),
            )
        };
        if h != INVALID_HANDLE_VALUE {
            return Ok(Tutamac(h));
        }
        // SAFETY: argümansız.
        match unsafe { GetLastError() } {
            ERROR_FILE_NOT_FOUND => return Err(BoruHatasi::Yok),
            ERROR_PIPE_BUSY => {
                if sure.kalan().is_none() {
                    return Err(BoruHatasi::ZamanAsimi);
                }
                // SAFETY: `ad` sıfır sonlu; kalan süre kadar bir örneğin boşalmasını bekler.
                unsafe { WaitNamedPipeW(ad.as_ptr(), sure.kalan_ms()) };
            }
            // Erişim reddi vb.: başka kullanıcı/yetki sorunu; çağıran dosyaya düşer.
            kod => return Err(BoruHatasi::Io(win_hata(kod).to_string())),
        }
    }
}

impl BoruUcu for WindowsBoru {
    fn degis(
        &self,
        hedef: &str,
        beklenen_pid: u32,
        istek: &[u8],
        zaman_asimi: Duration,
    ) -> Result<Vec<u8>, BoruHatasi> {
        let ad = boru_adi(&self.kullanici, hedef).map_err(|e| BoruHatasi::Io(e.to_string()))?;
        let ad_w = genis(&ad);
        let sure = Sure::yeni(zaman_asimi);
        let tutamac = baglan(&ad_w, &sure)?;

        // Sunucu kimliği: ad başka bir süreçteyse (kayıtla uyuşmuyorsa) hiçbir şey yazılmaz.
        let mut sunucu_pid = 0u32;
        // SAFETY: geçerli istemci tutamacı ve geçerli çıktı işaretçisi.
        if unsafe { GetNamedPipeServerProcessId(tutamac.0, &mut sunucu_pid) } == 0 {
            return Err(BoruHatasi::Io(son_hata().to_string()));
        }
        if sunucu_pid != beklenen_pid {
            return Err(BoruHatasi::KimlikUyusmadi);
        }

        // BoruG, Tutamac'ın sahipliğini alır (Drop kapatır).
        let mut g = BoruG {
            tutamac,
            sure,
            sunucu: false,
        };
        cerceve_yaz(&mut g, istek).map_err(|e| match e {
            super::boru_ortak::CerceveHata::ZamanAsimi => BoruHatasi::ZamanAsimi,
            diger => BoruHatasi::Io(diger.to_string()),
        })?;
        cerceve_oku(&mut g).map_err(|e| match e {
            super::boru_ortak::CerceveHata::ZamanAsimi => BoruHatasi::ZamanAsimi,
            diger => BoruHatasi::Io(diger.to_string()),
        })
    }

    fn sunucu_baslat(&self, program: &str, kutu: &GelenKutusu) -> Result<(), BoruHatasi> {
        let mut sunucular = self
            .sunucular
            .lock()
            .map_err(|_| BoruHatasi::Io("kilit zehirli".into()))?;
        if sunucular
            .get(program)
            .is_some_and(|d| !d.load(Ordering::Relaxed))
        {
            return Ok(());
        }
        let ad = boru_adi(&self.kullanici, program).map_err(|e| BoruHatasi::Io(e.to_string()))?;
        let ad_w = genis(&ad);
        let sddl = acl_sddl(&self.sid).map_err(|e| BoruHatasi::Io(e.to_string()))?;
        let guvenlik = guvenlik_olustur(&sddl).map_err(|e| BoruHatasi::Io(e.to_string()))?;
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: guvenlik.0,
            bInheritHandle: 0,
        };
        // İlk örnek: ad başkasındaysa ERROR_ACCESS_DENIED (ya da PIPE_BUSY) -> Dolu.
        let ilk = match ornek_olustur(&ad_w, &sa, true) {
            Ok(g) => g,
            Err(e)
                if e.raw_os_error() == Some(ERROR_ACCESS_DENIED as i32)
                    || e.raw_os_error() == Some(ERROR_PIPE_BUSY as i32) =>
            {
                return Err(BoruHatasi::Dolu)
            }
            Err(e) => return Err(BoruHatasi::Io(e.to_string())),
        };
        let durdur = Arc::new(AtomicBool::new(false));
        let tasinan = Tasinan((ilk, guvenlik));
        let (kutu2, program2, durdur2) = (kutu.clone(), program.to_string(), Arc::clone(&durdur));
        thread::Builder::new()
            .name(format!("taa-boru-kabul-{program}"))
            .spawn(move || sunucu_dongusu(ad_w, tasinan, kutu2, program2, durdur2))
            .map_err(|e| BoruHatasi::Io(e.to_string()))?;
        sunucular.insert(program.to_string(), durdur);
        Ok(())
    }
}
