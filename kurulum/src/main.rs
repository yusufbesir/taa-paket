// Sahip: Yusuf. yazan: [Claude#taa-paket-kurulum 2026-11-09]
//! TAA Kur kurulum ve kaldırma programı (tek exe; adı "kaldir" ile başlıyorsa kaldırır). Yalnız standart kütüphane.
//! Kurar: taa-kur.exe, taa-kur-pencere.exe, taa-guncelle.exe, kur.toml (YOKSA), anahtarlar\*.pub (YOKSA), kaldir.exe.
//! Var olan kur.toml/anahtar dosyasına, depo\, eski\, kayit.json ve veri alanlarına DOKUNMAZ. Kaldırma yalnız kendi
//! yazdığı dosyaları siler; kurulu TAA programları (depo\) kalır. Sonra yayından program seçtirir (taa-guncelle kur).
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

include!(concat!(env!("OUT_DIR"), "/dosyalar.rs"));

const AD: &str = "TAA Kur";
#[cfg(windows)]
const KAYIT: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Uninstall\TAA-Kur";
const MANIFEST: &str = "kurulum-taa-kur.txt";
/// Kullanıcıya ait: paketle gelse de var olan dosya ezilmez.
const KORUNAN: &[&str] = &["kur.toml", "rol.txt"];
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

fn program_kur(hedef: &Path, ad: &str) -> bool {
    let exe = hedef.join(if cfg!(windows) { "taa-guncelle.exe" } else { "taa-guncelle" });
    match Command::new(exe).args(["kur", ad]).env("TAA_KOK", hedef).status() {
        Ok(s) if s.success() => true,
        _ => { eprintln!("UYARI: {ad} kurulamadı; sonra TAA Kur penceresinden ya da `taa-guncelle kur {ad}` ile deneyin."); false }
    }
}

/// Bu PC'nin rolü: `--rol sube|sunucu`, yoksa soru (sessizde "sube"). Seçim kökteki rol.txt'ye yazılır (var olan ezilmez).
fn rol_sec(a: &[String], hedef: &Path, sessiz: bool) -> String {
    let kayitli = fs::read_to_string(hedef.join("rol.txt")).ok().map(|r| r.trim().to_string()).filter(|r| !r.is_empty());
    let rol = match arg(a, "--rol") {
        Some(r) if matches!(r.as_str(), "sube" | "sunucu") => r,
        Some(r) => { eprintln!("UYARI: bilinmeyen rol '{r}' (sube ya da sunucu olmalı); şube sayıldı."); "sube".into() }
        None => match (kayitli, sessiz) {
            (Some(r), _) => r,
            (None, true) => "sube".into(),
            (None, false) => {
                println!("\nBu PC ne olacak?\n  1) Şube PC (varsayılan)\n  2) Sunucu PC (TAA Sunucu, veritabanı vb. kendiliğinden kurulur)");
                if oku_satir("Seçiminiz (Enter = 1): ") == "2" { "sunucu".into() } else { "sube".into() }
            }
        },
    };
    if !hedef.join("rol.txt").exists() { let _ = fs::write(hedef.join("rol.txt"), format!("{rol}\n")); }
    rol
}

/// Program seçimi: önce bu rolde ZORUNLU olanlar kendiliğinden kurulur; sonra `--programlar a,b` (sessiz),
/// `--programsiz` (seçim yok) ya da gruplu numara listesi. `zorunlu_roller` dolu paketler rol-özeldir: yalnız o rollerde
/// (kendiliğinden) kurulur, başka rolde listede görünmez.
fn programlari_sec(a: &[String], hedef: &Path, sessiz: bool) {
    let Some(k) = katalog(hedef) else {
        if let Some(l) = arg(a, "--programlar") { for ad in l.split(',').map(str::trim).filter(|s| !s.is_empty()) { program_kur(hedef, ad); } }
        else if !sessiz { println!("\nProgram listesi şimdi alınamadı (internet ya da yayın kaynağı). Sonra TAA Kur penceresinden kurabilirsiniz."); }
        return;
    };
    let rol = rol_sec(a, hedef, sessiz);
    let zorunlu: Vec<&Satir> = k.iter().filter(|s| s.zorunlu.iter().any(|r| *r == rol)).collect();
    if !zorunlu.is_empty() {
        println!("\nBu PC ({rol}) için zorunlu paketler kurulacak:");
        for s in &zorunlu { println!("  - {} {}{}", s.ad, s.surum, if s.kurulu != "-" { " (zaten kurulu)" } else { "" }); }
        for s in zorunlu.iter().filter(|s| s.kurulu == "-") { program_kur(hedef, &s.ad); }
    }
    if a.iter().any(|x| x == "--programsiz") { return; }
    if let Some(l) = arg(a, "--programlar") { for ad in l.split(',').map(str::trim).filter(|s| !s.is_empty()) { if !zorunlu.iter().any(|z| z.ad == ad) { if k.iter().any(|s| s.ad == ad && !s.zorunlu.is_empty()) { eprintln!("UYARI: {ad} yalnız {} rolünde kurulur; atlandı.", k.iter().find(|s| s.ad == ad).map(|s| s.zorunlu.join("/")).unwrap_or_default()); } else { program_kur(hedef, ad); } } } return; }
    if sessiz { return; }
    let secilebilir: Vec<&Satir> = k.iter().filter(|s| s.kurulu == "-" && s.zorunlu.is_empty()).collect();
    if secilebilir.is_empty() { println!("\nSeçilebilecek başka program yok (hepsi kurulu ya da zorunlu)."); return; }
    println!("\nİsteğe bağlı programlar (seçmedikleriniz kurulmaz):");
    let mut gruplar: Vec<&str> = secilebilir.iter().map(|s| s.grup.as_str()).collect();
    gruplar.sort(); gruplar.dedup();
    let mut sira: Vec<&Satir> = vec![];
    for g in gruplar {
        println!(" [{g}]");
        for s in secilebilir.iter().filter(|s| s.grup == g) {
            sira.push(s);
            println!("  {}) {} {}{}", sira.len(), s.ad, s.surum, if s.aciklama.is_empty() { String::new() } else { format!(" - {}", s.aciklama) });
        }
    }
    let cevap = oku_satir("Kurulacakların numaralarını yazın (örn. 1,3), hepsi için H, atlamak için Enter: ");
    let secim: Vec<&str> = if cevap.eq_ignore_ascii_case("h") { sira.iter().map(|s| s.ad.as_str()).collect() }
        else { cevap.split([',', ' ']).filter_map(|n| n.trim().parse::<usize>().ok()).filter_map(|n| sira.get(n.wrapping_sub(1)).map(|s| s.ad.as_str())).collect() };
    for ad in secim { program_kur(hedef, ad); }
}

fn kur(a: &[String]) -> i32 {
    let sessiz = a.iter().any(|x| x == "--sessiz");
    println!("{AD} {SURUM} kurulumu");
    let hedef = match arg(a, "--hedef").or_else(|| arg(a, "--kok")) {
        Some(h) => PathBuf::from(h),
        None if sessiz => varsayilan_hedef(),
        None => {
            let v = varsayilan_hedef();
            let c = oku_satir(&format!("Kurulum klasörü (Enter = {}): ", v.display()));
            if c.is_empty() { v } else { PathBuf::from(c.trim_matches('"')) }
        }
    };
    println!("Kurulum klasörü: {}", hedef.display());
    if !sor("Kurulsun mu?", sessiz) { println!("Vazgeçildi."); return 0; }
    let mut liste: Vec<String> = vec![];
    let mut atlanan = 0;
    for (ad, veri) in DOSYALAR {
        let yol = ad.split('/').fold(hedef.clone(), |p, x| p.join(x));
        if korunan_mi(ad) && yol.exists() { atlanan += 1; continue; }
        if let Err(e) = yaz(&yol, veri) { eprintln!("HATA: {e}"); bekle(sessiz); return 1; }
        liste.push(ad.to_string());
    }
    match std::env::current_exe().and_then(fs::read) {
        Ok(v) => { if let Err(e) = yaz(&hedef.join("kaldir.exe"), &v) { eprintln!("HATA: {e}"); bekle(sessiz); return 1; } liste.push("kaldir.exe".into()); }
        Err(e) => { eprintln!("HATA: kaldırma programı yazılamadı: {e}"); bekle(sessiz); return 1; }
    }
    let kisayolsuz = a.iter().any(|x| x == "--kisayolsuz");
    let kisayollar = if kisayolsuz { vec![] } else { kisayol_olustur(&hedef) };
    if !kisayolsuz { kayit_yaz(&hedef); kok_ortami(&hedef); }
    let mut m = liste.join("\n");
    for k in &kisayollar { m.push_str(&format!("\n@{k}")); }
    m.push('\n');
    if let Err(e) = yaz(&hedef.join(MANIFEST), m.as_bytes()) { eprintln!("HATA: {e}"); bekle(sessiz); return 1; }
    println!("Kurulum tamam: {} dosya yazıldı{}.", liste.len(), if atlanan > 0 { format!(", {atlanan} mevcut ayar/anahtar dosyasına dokunulmadı") } else { String::new() });
    if !kisayollar.is_empty() { println!("Masaüstü ve Başlat menüsü kısayolları oluşturuldu."); }
    programlari_sec(a, &hedef, sessiz);
    if cfg!(windows) && !sessiz && sor("TAA Kur şimdi açılsın mı?", false) { let _ = Command::new(hedef.join("taa-kur-pencere.exe")).env("TAA_KOK", &hedef).spawn(); }
    0
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
    let kod = if a.iter().any(|x| x == "--kaldir") || ad.starts_with("kaldir") { kaldir(&a) } else { kur(&a) };
    std::process::exit(kod);
}
