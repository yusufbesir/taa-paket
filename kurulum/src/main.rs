// Sahip: Yusuf. yazan: [Claude#taa-paket-kurulum 2026-11-09]
//! TAA Kur kurulum ve kaldırma programı (tek exe; adı "kaldir" ile başlıyorsa kaldırır). Yalnız standart kütüphane.
//! Kurar: taa-kur.exe, taa-kur-pencere.exe, taa-guncelle.exe, kur.toml (YOKSA), anahtarlar\*.pub (YOKSA), kaldir.exe.
//! Var olan kur.toml/anahtar dosyasına, depo\, eski\, kayit.json ve veri alanlarına DOKUNMAZ. Kaldırma yalnız kendi
//! yazdığı dosyaları siler; kurulu TAA programları (depo\) kalır. Sonra yayından program seçtirir (taa-guncelle kur).
//! Etkileşimli kurulum Windows'ta kendi yerel penceresinde yürür (Win32 + GDI; tarayıcı ya da HTTP YOK):
//! aşamalar, program adımları, bekleme kartları ve şube için adım adım ayar direktifleri (docs/KURULUM-EKRANI.md).
//! `--sessiz` ve `--arayuzsuz` iken pencere açılmaz; seçimler konsolda sorulur (sessizde hiç sorulmaz).
mod direktif;
// Ekran modellerinin bir kısmını yalnız yerel pencere (Windows) kullanır; Linux'ta testli saf mantık olarak derlenir.
#[cfg_attr(not(windows), allow(dead_code))]
mod ekran;
#[cfg_attr(not(windows), allow(dead_code))]
mod ilerleme;
#[cfg_attr(not(windows), allow(dead_code))]
mod kartlar;
#[cfg(windows)]
mod pencere;
#[cfg_attr(not(windows), allow(dead_code))]
mod secim;
mod subeler;

use ekran::{Ekran, Komut};
use ilerleme::Asama;
use secim::{Secenek, Secim};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

include!(concat!(env!("OUT_DIR"), "/dosyalar.rs"));

const AD: &str = "TAA Kur";
#[cfg(windows)]
const KAYIT: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\TAA-Kur";
const MANIFEST: &str = "kurulum-taa-kur.txt";
/// Kullanıcıya ait: paketle gelse de var olan dosya ezilmez.
const KORUNAN: &[&str] = &["kur.toml", "rol.txt", "sube.txt"];
const KORUNAN_ONEK: &str = "anahtarlar/";

#[cfg(windows)]
fn konsol_utf8() { extern "system" { fn SetConsoleOutputCP(k: u32) -> i32; } unsafe { SetConsoleOutputCP(65001); } }
#[cfg(not(windows))]
fn konsol_utf8() {}

fn varsayilan_hedef() -> PathBuf {
    if cfg!(windows) {
        if Path::new(r"D:\").exists() { return PathBuf::from(r"D:\Programlar\TAA"); }
        if let Ok(p) = std::env::var("LOCALAPPDATA") { return PathBuf::from(p).join("Programs").join("TAA"); }
    }
    PathBuf::from("TAA")
}

fn sor(soru: &str, sessiz: bool) -> bool {
    if sessiz { return true; }
    print!("{soru} (E/H, Enter = Evet): ");
    let _ = io::stdout().flush();
    let mut s = String::new();
    let _ = io::stdin().read_line(&mut s);
    !matches!(s.trim().to_lowercase().as_str(), "h" | "hayir" | "hayır" | "n" | "no")
}

fn oku_satir(istem: &str) -> String {
    print!("{istem}");
    let _ = io::stdout().flush();
    let mut s = String::new();
    let _ = io::stdin().read_line(&mut s);
    s.trim().to_string()
}

fn bekle(sessiz: bool) { if !sessiz { oku_satir("Kapatmak için Enter'a basın…"); } }

fn arg(a: &[String], k: &str) -> Option<String> { a.iter().position(|x| x == k).and_then(|i| a.get(i + 1)).cloned() }

#[cfg(windows)]
fn reg(args: &[&str]) -> bool {
    use std::os::windows::process::CommandExt;
    Command::new("reg").args(args).creation_flags(0x0800_0000).status().map(|s| s.success()).unwrap_or(false)
}

#[cfg(windows)]
fn kisayol_olustur(hedef: &Path) -> Vec<String> {
    use std::os::windows::process::CommandExt;
    let exe = hedef.join("taa-kur-pencere.exe");
    let mut yollar = vec![];
    for klasor in ["Desktop", "Programs"] {
        let ps = format!(
            "[Console]::OutputEncoding=[System.Text.Encoding]::UTF8; $d=[Environment]::GetFolderPath('{klasor}'); $y=Join-Path $d '{AD}.lnk'; $s=(New-Object -ComObject WScript.Shell).CreateShortcut($y); $s.TargetPath='{exe}'; $s.WorkingDirectory='{dir}'; $s.Description='{AD}'; $s.Save(); Write-Output $y",
            exe = exe.display(), dir = hedef.display());
        if let Ok(o) = Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", &ps]).creation_flags(0x0800_0000).output() {
            if o.status.success() { let y = String::from_utf8_lossy(&o.stdout).trim().to_string(); if !y.is_empty() { yollar.push(y); } }
        }
    }
    yollar
}
#[cfg(not(windows))]
fn kisayol_olustur(_: &Path) -> Vec<String> { vec![] }

#[cfg(windows)]
fn kayit_yaz(hedef: &Path) {
    let kal = hedef.join("kaldir.exe").display().to_string();
    let uz = hedef.display().to_string();
    for (ad, tur, v) in [("DisplayName", "REG_SZ", AD.to_string()), ("DisplayVersion", "REG_SZ", SURUM.to_string()), ("Publisher", "REG_SZ", "TAA".to_string()),
        ("InstallLocation", "REG_SZ", uz), ("UninstallString", "REG_SZ", format!("\"{kal}\"")), ("NoModify", "REG_DWORD", "1".into()), ("NoRepair", "REG_DWORD", "1".into())] {
        reg(&["add", KAYIT, "/v", ad, "/t", tur, "/d", &v, "/f"]);
    }
}
#[cfg(not(windows))]
fn kayit_yaz(_: &Path) {}

/// Kök varsayılan yerde değilse kullanıcı ortamına TAA_KOK yazılır (taa-kur ve programlar kökü bulsun).
#[cfg(windows)]
fn kok_ortami(hedef: &Path) {
    if hedef != Path::new(r"D:\Programlar\TAA") { reg(&["add", r"HKCU\Environment", "/v", "TAA_KOK", "/t", "REG_SZ", "/d", &hedef.display().to_string(), "/f"]); }
}
#[cfg(not(windows))]
fn kok_ortami(_: &Path) {}

fn yaz(yol: &Path, veri: &[u8]) -> Result<(), String> {
    if let Some(k) = yol.parent() { fs::create_dir_all(k).map_err(|e| format!("{}: {e}", k.display()))?; }
    fs::write(yol, veri).map_err(|e| format!("{} yazılamadı ({e}). Program açıksa kapatıp yeniden deneyin.", yol.display()))?;
    #[cfg(unix)]
    if yol.extension().is_none() { use std::os::unix::fs::PermissionsExt; let _ = fs::set_permissions(yol, fs::Permissions::from_mode(0o755)); }
    Ok(())
}

fn korunan_mi(ad: &str) -> bool { KORUNAN.contains(&ad) || ad.starts_with(KORUNAN_ONEK) }

/// Katalog satırı (`taa-guncelle liste --duz`): ad, yayındaki, kurulu ("-" = kurulu değil), grup, zorunlu roller, açıklama.
struct Satir { ad: String, surum: String, kurulu: String, grup: String, zorunlu: Vec<String>, aciklama: String }

fn katalog(hedef: &Path) -> Option<Vec<Satir>> {
    let exe = hedef.join(if cfg!(windows) { "taa-guncelle.exe" } else { "taa-guncelle" });
    let o = Command::new(exe).args(["liste", "--duz"]).env("TAA_KOK", hedef).output().ok()?;
    if !o.status.success() { return None; }
    let m = String::from_utf8_lossy(&o.stdout);
    Some(m.lines().filter_map(|l| {
        let mut p = l.split('\t');
        let (ad, surum, kurulu) = (p.next()?.to_string(), p.next()?.to_string(), p.next()?.to_string());
        let grup = p.next().filter(|g| *g != "-").unwrap_or("Diğer").to_string();
        let zorunlu = p.next().filter(|z| *z != "-").map(|z| z.split(',').map(str::to_string).collect()).unwrap_or_default();
        let aciklama = p.next().filter(|a| *a != "-").unwrap_or("").to_string();
        Some(Satir { ad, surum, kurulu, grup, zorunlu, aciklama })
    }).collect())
}

/// `taa-guncelle kur <ad>`: stderr'deki TAA-ILERLEME satırları ekrana işlenir, öteki satırlar konsola aynen geçer.
fn program_kur(hedef: &Path, ad: &str, ekran: &Ekran) -> bool {
    let exe = hedef.join(if cfg!(windows) { "taa-guncelle.exe" } else { "taa-guncelle" });
    ekran.g(|d| d.program_ekle(ad, false));
    let mut son_hata = String::new();
    let sonuc = Command::new(exe).args(["kur", ad]).env("TAA_KOK", hedef).env("TAA_ILERLEME", "1").stderr(Stdio::piped()).spawn().and_then(|mut c| {
        if let Some(e) = c.stderr.take() {
            let mut r = io::BufReader::new(e);
            let mut b = Vec::new();
            while matches!(r.read_until(b'\n', &mut b), Ok(n) if n > 0) {
                let satir = String::from_utf8_lossy(&b).trim_end().to_string();
                b.clear();
                match ilerleme::isaret_coz(&satir) {
                    Some(i) => { ekran.g(|d| d.isaret_uygula(&i)); }
                    None if !satir.is_empty() => { eprintln!("{satir}"); son_hata = satir; }
                    None => {}
                }
            }
        }
        c.wait()
    });
    match sonuc {
        Ok(s) if s.success() => { ekran.g(|d| d.program_tamam(ad)); true }
        Ok(_) => {
            ekran.g(|d| d.program_hata(ad, &son_hata));
            eprintln!("UYARI: {ad} kurulamadı; sonra TAA Kur penceresinden ya da `taa-guncelle kur {ad}` ile deneyin.");
            false
        }
        Err(e) => {
            ekran.g(|d| d.program_hata(ad, &format!("taa-guncelle çalıştırılamadı: {e}")));
            eprintln!("UYARI: {ad} kurulamadı (taa-guncelle çalıştırılamadı: {e}).");
            false
        }
    }
}

/// Listeyi önce ekrana kaydeder (sırada görünsünler), sonra sırayla kurar.
fn programlari_kur(hedef: &Path, adlar: &[String], ekran: &Ekran) {
    if adlar.is_empty() { return; }
    ekran.g(|d| { for a in adlar { d.program_ekle(a, false); } d.asama(Asama::Programlar, "Programlar kuruluyor"); });
    for a in adlar { program_kur(hedef, a, ekran); }
}

fn satir_oku(yol: &Path) -> Option<String> {
    fs::read_to_string(yol).ok().map(|r| r.trim().to_string()).filter(|r| !r.is_empty())
}

/// Bu PC'nin rolü: `--rol sube|sunucu`, kayıtlı rol.txt, yoksa seçim (pencerede ya da konsolda; sessizde "sube").
/// Seçim kökteki rol.txt'ye yazılır (var olan ezilmez).
fn rol_sec(a: &[String], hedef: &Path, sessiz: bool, ekran: &Ekran) -> String {
    let rol = match arg(a, "--rol") {
        Some(r) if matches!(r.as_str(), "sube" | "sunucu") => r,
        Some(r) => { eprintln!("UYARI: bilinmeyen rol '{r}' (sube ya da sunucu olmalı); şube sayıldı."); "sube".into() }
        None => match (satir_oku(&hedef.join("rol.txt")), sessiz) {
            (Some(r), _) => r,
            (None, true) => "sube".into(),
            (None, false) => {
                let s = Secim::tek("Bu PC ne olacak?", "Şube PC'lerinde mağaza programları, Sunucu PC'de TAA Sunucu ve veritabanı paketleri kurulur.", vec![
                    Secenek::yeni("Şube PC", "Mağazada kullanılan bilgisayar (çoğu kurulum)."),
                    Secenek::yeni("Sunucu PC", "Merkezdeki sunucu; sunucu paketleri kendiliğinden kurulur."),
                ], 0).dugmeler("Devam", "Şube PC");
                if ekran.sec(s) == Some(vec![1]) { "sunucu".into() } else { "sube".into() }
            }
        },
    };
    if !hedef.join("rol.txt").exists() { let _ = fs::write(hedef.join("rol.txt"), format!("{rol}\n")); }
    rol
}

/// Şube PC'sinde şube: `--sube <kod>` (MRK01 biçimi; eski slug de kabul), kayıtlı sube.txt, yoksa gömülü 15 şubeden seçim
/// (sessizde sorulmaz). Kayıtlı eski slug koda çevrilir. Kod `sube.txt`'ye ve fiş sunucusunun okuduğu
/// `<veri kökü>/sube.json`'a ({"kod":"MRK01"}) yazılır. "Sonra" denirse adım direktif olarak kalır.
fn sube_sec(a: &[String], hedef: &Path, sessiz: bool, ekran: &Ekran) {
    let liste = subeler::gomulu();
    let kod = if let Some(kayit) = satir_oku(&hedef.join("sube.txt")) {
        match subeler::koda_cevir(&liste, &kayit) {
            Some(k) => Some(k),
            None => { eprintln!("UYARI: kayıtlı şube '{kayit}' tanınmadı; şube yeniden seçilecek."); None }
        }
    } else { None };
    let kod = match kod {
        Some(k) => Some(k),
        None => match arg(a, "--sube") {
            Some(k) => match subeler::koda_cevir(&liste, &k) {
                Some(c) => Some(c),
                None => { eprintln!("UYARI: geçersiz şube kodu '{k}' (3 büyük harf + 2 rakam, örn. MRK01); şube seçilmedi."); None }
            },
            None if sessiz || liste.is_empty() => None,
            None => {
                let s = Secim::tek("Hangi şube?", "Bu PC'nin bulunduğu şubeyi seçin. Listede yoksa Sonra deyin; adım aşağıda kalır.",
                    liste.iter().map(|x| Secenek::yeni(x.ad.trim_end_matches(" Mağaza").replace(" Mağaza ", " ").trim(), &x.il)).collect(), 0).dugmeler("Devam", "Sonra");
                ekran.sec(s).and_then(|v| v.first().and_then(|i| liste.get(*i)).map(|x| x.kod.clone()))
            }
        },
    };
    if let Some(k) = kod {
        let _ = fs::write(hedef.join("sube.txt"), format!("{k}\n"));
        if let Some(j) = subeler::sube_json(&k) {
            let kok = veri_koku(hedef);
            let _ = fs::create_dir_all(&kok);
            let _ = fs::write(kok.join("sube.json"), j);
        }
    }
}

/// Program seçimi: önce bu rolde ZORUNLU olanlar kendiliğinden kurulur; sonra `--programlar a,b` (sessiz),
/// `--programsiz` (seçim yok) ya da gruplu çoklu seçim (pencerede ya da konsolda). `zorunlu_roller` dolu paketler
/// rol-özeldir: yalnız o rollerde (kendiliğinden) kurulur, başka rolde listede görünmez.
fn programlari_sec(a: &[String], hedef: &Path, rol: &str, sessiz: bool, ekran: &Ekran) {
    let Some(k) = katalog(hedef) else {
        if let Some(l) = arg(a, "--programlar") { programlari_kur(hedef, &l.split(',').map(str::trim).filter(|s| !s.is_empty()).map(str::to_string).collect::<Vec<_>>(), ekran); }
        else if !sessiz {
            println!("\nProgram listesi şimdi alınamadı (internet ya da yayın kaynağı). Sonra TAA Kur penceresinden kurabilirsiniz.");
            ekran.g(|d| d.asama(Asama::Secim, "Program listesi alınamadı; programları sonra TAA Kur'dan kurabilirsiniz"));
        }
        return;
    };
    let zorunlu: Vec<&Satir> = k.iter().filter(|s| s.zorunlu.iter().any(|z| z == rol)).collect();
    if !zorunlu.is_empty() {
        println!("\nBu PC ({rol}) için zorunlu paketler kurulacak:");
        for s in &zorunlu { println!("  - {} {}{}", s.ad, s.surum, if s.kurulu != "-" { " (zaten kurulu)" } else { "" }); }
        programlari_kur(hedef, &zorunlu.iter().filter(|s| s.kurulu == "-").map(|s| s.ad.clone()).collect::<Vec<_>>(), ekran);
    }
    if a.iter().any(|x| x == "--programsiz") { return; }
    if let Some(l) = arg(a, "--programlar") {
        let mut plan = vec![];
        for ad in l.split(',').map(str::trim).filter(|s| !s.is_empty()) { if !zorunlu.iter().any(|z| z.ad == ad) { if k.iter().any(|s| s.ad == ad && !s.zorunlu.is_empty()) { eprintln!("UYARI: {ad} yalnız {} rolünde kurulur; atlandı.", k.iter().find(|s| s.ad == ad).map(|s| s.zorunlu.join("/")).unwrap_or_default()); } else { plan.push(ad.to_string()); } } }
        programlari_kur(hedef, &plan, ekran);
        return;
    }
    if sessiz { return; }
    let mut secilebilir: Vec<&Satir> = k.iter().filter(|s| s.kurulu == "-" && s.zorunlu.is_empty()).collect();
    if secilebilir.is_empty() { println!("\nSeçilebilecek başka program yok (hepsi kurulu ya da zorunlu)."); return; }
    secilebilir.sort_by(|x, y| x.grup.cmp(&y.grup));
    ekran.g(|d| d.asama(Asama::Secim, "Program seçimi bekleniyor"));
    let secenekler = secilebilir.iter().map(|s| Secenek {
        etiket: format!("{} {}", s.ad, s.surum), aciklama: s.aciklama.clone(), grup: s.grup.clone(),
    }).collect();
    let s = Secim::coklu("İsteğe bağlı programlar", "Kurulacakları işaretleyin (Boşluk ya da tıklama). Seçmedikleriniz kurulmaz; sonra TAA Kur'dan da kurabilirsiniz.", secenekler)
        .dugmeler("Kur", "Hiçbiri");
    let secim: Vec<String> = ekran.sec(s).unwrap_or_default().into_iter().filter_map(|i| secilebilir.get(i).map(|s| s.ad.clone())).collect();
    programlari_kur(hedef, &secim, ekran);
}

/// Kurulum programının bulunduğu klasör (yanına konan sunucu-yollari.json vb. buradan alınır).
fn yan_klasor() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf))
}

fn veri_koku(hedef: &Path) -> PathBuf {
    direktif::veri_kok(|k| std::env::var(k).ok(), cfg!(windows)).unwrap_or_else(|| hedef.join("veri"))
}

fn tailscale_kurulu(ekran: &Ekran) -> bool {
    let pakette = ekran.g(|d| d.programlar.iter().any(|p| p.ad.eq_ignore_ascii_case("tailscale") && p.durum() == ilerleme::Durum::Tamam)).unwrap_or(false);
    let sistemde = cfg!(windows) && std::env::var("ProgramFiles").map(|p| Path::new(&p).join("Tailscale").join("tailscale.exe").exists()).unwrap_or(false);
    pakette || sistemde
}

/// Bilinenleri (rol.txt, sube.txt, kur.toml, kopyalanan yan dosyalar, kurulanlar) toplayıp adımları ekrana yazar.
fn direktifleri_guncelle(hedef: &Path, oto: &direktif::Otomatik, ekran: &Ekran) {
    let sube = satir_oku(&hedef.join("sube.txt")).map(|k| {
        let ad = subeler::bul(&subeler::gomulu(), &k).map(|s| s.ad.clone()).unwrap_or_else(|| k.clone());
        (k, ad)
    });
    let g = direktif::Girdi {
        rol: satir_oku(&hedef.join("rol.txt")),
        sube,
        kaynak_url: fs::read_to_string(hedef.join("kur.toml")).ok().and_then(|t| direktif::kaynak_url_oku(&t)),
        yollar: oto.yollar,
        ca: oto.ca,
        tailscale_anahtar: oto.tailscale_anahtar,
        tailscale_kurulu: tailscale_kurulu(ekran),
        hatali: ekran.g(|d| d.hatalilar()).unwrap_or_default(),
        kuruluyor: ekran.g(|d| d.asama != Asama::Bitti).unwrap_or(false),
    };
    let v = direktif::uret(&g);
    ekran.p(|d| d.direktifler = v);
}

fn direktifleri_yaz(ekran: &Ekran) {
    let v = ekran.p(|d| d.direktifler.clone()).unwrap_or_default();
    if v.is_empty() { return; }
    println!("\nKurulumdan sonra bu PC'de:");
    for (i, x) in v.iter().enumerate() {
        let d = match x.durum { direktif::Durum::Yapildi => "tamam", direktif::Durum::Yapilacak => "yapılacak", direktif::Durum::Bilgi => "bilgi" };
        println!("  {}. [{d}] {}: {}", i + 1, x.baslik, x.ayrinti);
    }
}

/// Kurulumu durduran hata: konsola yazar; pencerede hata kutusu gösterilir ve Bitir beklenir.
fn hata_bitir(ekran: &Ekran, mesaj: &str, sessiz: bool) -> i32 {
    eprintln!("HATA: {mesaj}");
    if ekran.pencere {
        ekran.p(|d| { d.hata = Some(mesaj.to_string()); d.ilerleme.komut_bekleniyor = true; });
        while !matches!(ekran.komut_bekle(), Komut::Bitir | Komut::TaaKurAc) {}
    } else {
        bekle(sessiz);
    }
    1
}

fn taa_kur_ac(hedef: &Path) {
    let _ = Command::new(hedef.join(if cfg!(windows) { "taa-kur-pencere.exe" } else { "taa-kur-pencere" })).env("TAA_KOK", hedef).spawn();
}

/// Kurulum klasörü: `--hedef`/`--kok`, yoksa varsayılan; yalnız konsol kurulumunda (pencere yok, sessiz değil) sorulur.
fn hedef_belirle(a: &[String], sor_konsolda: bool) -> PathBuf {
    match arg(a, "--hedef").or_else(|| arg(a, "--kok")) {
        Some(h) => PathBuf::from(h),
        None if !sor_konsolda => varsayilan_hedef(),
        None => {
            let v = varsayilan_hedef();
            let c = oku_satir(&format!("Kurulum klasörü (Enter = {}): ", v.display()));
            if c.is_empty() { v } else { PathBuf::from(c.trim_matches('"')) }
        }
    }
}

/// Kurulum akışı (pencerede ayrı iş parçacığında çalışır).
fn kur(a: &[String], hedef: &Path, ekran: &Ekran) -> i32 {
    let sessiz = a.iter().any(|x| x == "--sessiz");
    println!("Kurulum klasörü: {}", hedef.display());
    if ekran.pencere {
        let s = Secim::tek("TAA Kur kurulsun mu?", &format!(
            "Kurulum klasörü: {}\n\nTAA Kur, güncelleyici ve kaldırma programı kurulur; sonra bu PC'nin rolü ve şubesi sorulur. \
             Var olan ayar, anahtar ve verilere dokunulmaz.", hedef.display()), vec![], 0).dugmeler("Kur", "Vazgeç");
        if ekran.sec(s).is_none() { println!("Vazgeçildi."); return 0; }
    } else if !sor("Kurulsun mu?", sessiz) { println!("Vazgeçildi."); return 0; }
    ekran.g(|d| d.cekirdek_basla(DOSYALAR.len() as u32 + 1));
    let mut liste: Vec<String> = vec![];
    let mut atlanan = 0;
    for (ad, veri) in DOSYALAR {
        let yol = ad.split('/').fold(hedef.to_path_buf(), |p, x| p.join(x));
        ekran.g(|d| d.cekirdek_dosya());
        if korunan_mi(ad) && yol.exists() { atlanan += 1; continue; }
        if let Err(e) = yaz(&yol, veri) { return hata_bitir(ekran, &e, sessiz); }
        liste.push(ad.to_string());
    }
    match std::env::current_exe().and_then(fs::read) {
        Ok(v) => { if let Err(e) = yaz(&hedef.join("kaldir.exe"), &v) { return hata_bitir(ekran, &e, sessiz); } liste.push("kaldir.exe".into()); ekran.g(|d| d.cekirdek_dosya()); }
        Err(e) => return hata_bitir(ekran, &format!("kaldırma programı yazılamadı: {e}"), sessiz),
    }
    let kisayolsuz = a.iter().any(|x| x == "--kisayolsuz");
    let kisayollar = if kisayolsuz { vec![] } else { kisayol_olustur(hedef) };
    if !kisayolsuz { kayit_yaz(hedef); kok_ortami(hedef); }
    let mut m = liste.join("\n");
    for k in &kisayollar { m.push_str(&format!("\n@{k}")); }
    m.push('\n');
    if let Err(e) = yaz(&hedef.join(MANIFEST), m.as_bytes()) { return hata_bitir(ekran, &e, sessiz); }
    println!("Kurulum tamam: {} dosya yazıldı{}.", liste.len(), if atlanan > 0 { format!(", {atlanan} mevcut ayar/anahtar dosyasına dokunulmadı") } else { String::new() });
    if !kisayollar.is_empty() {
        println!("Masaüstü ve Başlat menüsü kısayolları oluşturuldu.");
        ekran.g(|d| d.asama(Asama::Cekirdek, "Masaüstü ve Başlat menüsüne TAA Kur kısayolu eklendi"));
    }
    // Otomatik ayarlar: kurulum programının yanındaki yardımcı dosyalar (var olan ezilmez).
    let veri = veri_koku(hedef);
    let oto = direktif::yan_dosyalari_kopyala(yan_klasor().as_deref(), hedef, &veri);
    direktifleri_guncelle(hedef, &oto, ekran);
    ekran.g(|d| d.asama(Asama::Secim, "Rol ve şube seçimi"));
    let rol = rol_sec(a, hedef, sessiz, ekran);
    direktifleri_guncelle(hedef, &oto, ekran);
    if rol == "sube" { sube_sec(a, hedef, sessiz, ekran); }
    direktifleri_guncelle(hedef, &oto, ekran);
    programlari_sec(a, hedef, &rol, sessiz, ekran);
    ekran.g(|d| d.asama(Asama::Bitti, "Kurulum bitti"));
    direktifleri_guncelle(hedef, &oto, ekran);
    direktifleri_yaz(ekran);
    if ekran.pencere {
        // Bitti ekranı: hata varsa Yeniden dene; Bitir ya da "TAA Kur'u aç" ile kapanır.
        loop {
            ekran.g(|d| d.komut_bekleniyor = true);
            match ekran.komut_bekle() {
                Komut::YenidenDene(ad) => {
                    if ekran.g(|d| { let t = d.yeniden_dene(&ad); if t { d.komut_bekleniyor = false; d.asama(Asama::Programlar, "Yeniden deneniyor"); } t }).unwrap_or(false) {
                        program_kur(hedef, &ad, ekran);
                        ekran.g(|d| d.asama(Asama::Bitti, "Kurulum bitti"));
                        direktifleri_guncelle(hedef, &oto, ekran);
                    }
                }
                Komut::TaaKurAc => { taa_kur_ac(hedef); break; }
                Komut::Bitir => break,
                Komut::Secim(_) | Komut::Vazgec => {}
            }
        }
        ekran.g(|d| d.komut_bekleniyor = false);
    } else if cfg!(windows) && !sessiz && sor("TAA Kur şimdi açılsın mı?", false) {
        taa_kur_ac(hedef);
    }
    0
}

/// Kurulum girişi: Windows'ta etkileşimli kurulum yerel pencerede (kurulum ayrı iş parçacığında), aksi halde konsolda.
fn kur_baslat(a: &[String]) -> i32 {
    let sessiz = a.iter().any(|x| x == "--sessiz");
    let pencere_iste = cfg!(windows) && !sessiz && !a.iter().any(|x| x == "--arayuzsuz");
    println!("{AD} {SURUM} kurulumu");
    let hedef = hedef_belirle(a, !sessiz && !pencere_iste);
    #[allow(unused_mut)]
    let (mut ekran, uc) = Ekran::yeni(SURUM, &hedef.display().to_string(), pencere_iste);
    #[cfg(windows)]
    if pencere_iste {
        match pencere::Pencere::ac(uc.clone()) {
            Ok(p) => {
                let (a2, h2) = (a.to_vec(), hedef.clone());
                let is = std::thread::spawn(move || {
                    let k = kur(&a2, &h2, &ekran);
                    ekran.p(|d| d.cikis = Some(k));
                    k
                });
                p.dongu();
                // Pencere kurulum bitmeden kapatıldıysa (kullanıcı onayladı) kurulum yarıda bırakılır.
                if uc.durum.lock().map(|d| d.cikis.is_none()).unwrap_or(true) { println!("Kurulum yarıda bırakıldı."); return 1; }
                return is.join().unwrap_or(1);
            }
            Err(e) => {
                eprintln!("UYARI: kurulum penceresi açılamadı ({e}); kurulum konsolda sürüyor.");
                ekran.pencere = false;
            }
        }
    }
    drop(uc);
    kur(a, &hedef, &ekran)
}

fn kaldir(a: &[String]) -> i32 {
    let sessiz = a.iter().any(|x| x == "--sessiz");
    let hedef = arg(a, "--hedef").map(PathBuf::from).unwrap_or_else(|| std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.to_path_buf())).unwrap_or_else(varsayilan_hedef));
    let man = match fs::read_to_string(hedef.join(MANIFEST)) { Ok(m) => m, Err(_) => { eprintln!("HATA: {} içinde {MANIFEST} yok; kurulu TAA Kur bulunamadı.", hedef.display()); bekle(sessiz); return 1; } };
    println!("{AD} kaldırılacak: {}", hedef.display());
    println!("Kurulu TAA programlarına (depo\\), verilerine, kur.toml ve anahtarlara dokunulmaz; yalnız kurucu dosyaları silinir.");
    if !sor("Kaldırılsın mı?", sessiz) { println!("Vazgeçildi."); return 0; }
    let kendi = std::env::current_exe().ok().and_then(|e| e.file_name().map(|n| n.to_string_lossy().to_lowercase()));
    let mut hata = false;
    let mut kisayollar = vec![];
    let mut dosyalar = vec![];
    for s in man.lines().map(str::trim).filter(|s| !s.is_empty()) { if let Some(k) = s.strip_prefix('@') { kisayollar.push(k.to_string()) } else { dosyalar.push(s.to_string()) } }
    for k in &kisayollar { let _ = fs::remove_file(k); }
    let mut kaldir_sonra = false;
    for d in &dosyalar {
        // Kullanıcının ayar ve anahtarları kurulumda yazılmış olsa bile silinmez.
        if korunan_mi(d) { continue; }
        let yol = d.split('/').fold(hedef.clone(), |p, x| p.join(x));
        if kendi.as_deref() == yol.file_name().map(|n| n.to_string_lossy().to_lowercase()).as_deref() && yol.parent() == Some(hedef.as_path()) { kaldir_sonra = true; continue; }
        if yol.exists() { if let Err(e) = fs::remove_file(&yol) { eprintln!("Silinemedi: {} ({e}). Program açıksa kapatın.", yol.display()); hata = true; } }
    }
    let _ = fs::remove_file(hedef.join(MANIFEST));
    kayit_sil();
    if hata { eprintln!("Bazı dosyalar silinemedi; programı kapatıp kaldırmayı yeniden çalıştırın."); bekle(sessiz); return 1; }
    println!("Kaldırma tamam.");
    #[cfg(windows)]
    if kaldir_sonra {
        use std::os::windows::process::CommandExt;
        // Çalışan exe kendini silemez; kısa gecikmeyle cmd siler (klasör dolu olabilir: rmdir yalnız boşsa siler).
        let komut = format!("/c \"ping -n 3 127.0.0.1 >nul & del /f /q \"{}\" & rmdir \"{}\"\"", hedef.join("kaldir.exe").display(), hedef.display());
        let _ = Command::new("cmd").raw_arg(komut).creation_flags(0x0800_0000).spawn();
    }
    #[cfg(not(windows))]
    if kaldir_sonra { let _ = fs::remove_file(hedef.join("kaldir.exe")); let _ = fs::remove_dir(&hedef); }
    bekle(sessiz);
    0
}

#[cfg(windows)]
fn kayit_sil() { reg(&["delete", KAYIT, "/f"]); }
#[cfg(not(windows))]
fn kayit_sil() {}

fn main() {
    konsol_utf8();
    let a: Vec<String> = std::env::args().collect();
    let ad = a.first().map(|p| Path::new(p).file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default()).unwrap_or_default();
    let kod = if a.iter().any(|x| x == "--kaldir") || ad.starts_with("kaldir") { kaldir(&a) } else { kur_baslat(&a) };
    std::process::exit(kod);
}
