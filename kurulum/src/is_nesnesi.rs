//! Alt süreç yaşam bağı: Kur kapanırsa (pencere kapatma, çökme, görev sonlandırma) çalışan `taa-guncelle kur` da
//! sonlanır, yetim süreç kalmaz. Windows'ta "kapanınca öldür" iş nesnesi; başka sistemlerde işlem yok.
//! [Claude#havuz-sube-kur 2026-11-18]

use std::process::Child;

#[cfg(windows)]
mod win {
    use std::os::windows::io::AsRawHandle;
    use std::process::Child;
    use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation, SetInformationJobObject,
        JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };

    /// Tek süreçlik ömür boyu açık tutulan iş nesnesi; tutamaç bilerek kapatılmaz (süreç sonunda OS kapatır ve alt süreci öldürür).
    pub fn bagla(c: &Child) -> bool {
        unsafe {
            let job: HANDLE = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            if job.is_null() {
                return false;
            }
            let mut bilgi: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
            bilgi.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let tamam = SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                &bilgi as *const _ as *const _,
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            ) != 0
                && AssignProcessToJobObject(job, c.as_raw_handle() as HANDLE) != 0;
            if !tamam {
                CloseHandle(job);
            }
            tamam
        }
    }
}

/// Alt süreci iş nesnesine bağlar; başarısızlık kurulumu durdurmaz (yalnız yetim koruması yoktur).
pub fn bagla(c: &Child) {
    #[cfg(windows)]
    {
        if !win::bagla(c) {
            eprintln!("uyarı: alt süreç iş nesnesine bağlanamadı (Kur kapanırsa kurulum arkada sürebilir)");
        }
    }
    #[cfg(not(windows))]
    {
        let _ = c;
    }
}
