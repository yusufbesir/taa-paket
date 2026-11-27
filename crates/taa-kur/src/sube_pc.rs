// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-kurulum 2026-11-27]
//! Şube bilgisayarı kurulum profili (PLAN-TAA-WEB-2026-11-27 §9, docs/SUBE-DUGUMU.md §9).
//!
//! Şube bilgisayarı o şubenin sunucusu gibi çalışır: telefonlar ve tarayıcılar ona yazar, o da her kaydı merkeze
//! taşır (aynı `no`). Merkezin yerini tutmaz. Bu profil o bilgisayarda şunları yapar:
//! 1. `taa-yazdir` (yazdırma köprüsü, 127.0.0.1) ve `taa-sube-dugum` (şube düğümü, yerel ağ, kapı 8788) ikililerini bulur,
//!    düğümü `--koken <merkez>` ile başlatır; yerel ağa (`--lan`) yalnız `--lan-ac` verilirse açar: düğümde cihaz
//!    doğrulaması henüz yok, bu yüzden varsayılan kapalıdır (yalnız 127.0.0.1),
//! 2. günlük klasörünü (`<veri kökü>/gunluk/`) açar; düğüm oraya `YYYY-AA-GG.jsonl` ekler, hiçbir şey silinmez,
//! 3. istemci yol listesini yazar: `[{ad:"merkez",url},{ad:"sube",url},{ad:"yerel"}]` (sıra: merkez → şube → yerel),
//! 4. `taa-yazdir` ayarında izinli kökenlere merkez ve şube adresini ekler,
//! 5. Windows açılış görevini kaydeder ve hizmetleri hemen başlatır.
//!
//! Silme yok: değişen ayar dosyasının eski hali `eski\` altına tarihli adla taşınır. Sistem çağrıları (`schtasks`,
//! `reg`, `netsh`, süreç başlatma) [`Sistem`] üstünden yapılır; testler sahte sistem verir.
//! Çatı şubeye kurulmaz: bu profil sunucu rolündeki bilgisayarda çalışmaz ve Çatı'ya dair hiçbir şey kurmaz.

use crate::{Kur, KurHata};
use serde_json::{json, Value};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::time::Duration;

type Sonuc<T> = Result<T, KurHata>;

/// Yazdırma köprüsü ikilisi (crates/taa-yazdir).
pub const YAZDIR: &str = "taa-yazdir";
/// Şube düğümü ikilisi (crates/taa-sube-dugum).
pub const DUGUM: &str = "taa-sube-dugum";
/// Şube düğümünün varsayılan kapısı (sözleşme).
pub const DUGUM_PORT: u16 = 8788;
/// `taa-yazdir` varsayılan kapısı (`taa_yazdir::VARSAYILAN_PORT` ile aynı; ayar dosyasında değiştirilebilir).
pub const YAZDIR_PORT: u16 = 8797;
/// Açılış görevinin adı (schtasks) ve Run kaydındaki değer adı.
pub const GOREV_ADI: &str = "TAA Sube PC";
/// Güvenlik duvarı kuralının adı.
pub const KURAL_ADI: &str = "TAA Sube Dugumu";
/// Kurulum kökünde tutulan profil dosyası (açılışta `baslat` bunu okur).
pub const PROFIL_DOSYASI: &str = "sube-pc.json";
/// `<veri kökü>/sube-pc/` altındaki istemci yol listesi.
pub const YOL_DOSYASI: &str = "istemci-yollari.json";
/// Açılışta pencere açmadan hizmetleri başlatan bayrak (`taa-kur-pencere --sube-pc-baslat`).
pub const BASLAT_BAYRAGI: &str = "--sube-pc-baslat";
const RUN_ANAHTARI: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

// ---------------------------------------------------------------- adres ve yol listesi

/// Adresin kökenini (`şema://ad[:kapı]`, küçük harf, yol yok) döndürür. Kullanıcı adı, sorgu ve parça kabul edilmez.
pub fn koken(ham: &str) -> Result<String, String> {
    let m = ham.trim();
    let (sema, kalan) = m
        .split_once("://")
        .ok_or_else(|| format!("adres http:// ya da https:// ile başlamalı: '{ham}'"))?;
    let sema = sema.to_ascii_lowercase();
    if sema != "http" && sema != "https" {
        return Err(format!(
            "adres http:// ya da https:// ile başlamalı: '{ham}'"
        ));
    }
    let ana = kalan.split('/').next().unwrap_or("").to_ascii_lowercase();
    let gecerli = !ana.is_empty()
        && ana.len() <= 255
        && !ana.starts_with(':')
        && ana
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':'));
    if !gecerli || kalan.contains(['?', '#', '@']) {
        return Err(format!("adres okunamadı: '{ham}'"));
    }
    if let Some((_, kapi)) = ana.rsplit_once(':') {
        if kapi.parse::<u16>().ok().filter(|k| *k > 0).is_none() {
            return Err(format!("kapı numarası geçersiz: '{ham}'"));
        }
    }
    Ok(format!("{sema}://{ana}"))
}

/// Yerel ağ IPv4 adresi mi: 10/8, 172.16/12, 192.168/16.
pub fn yerel_ag_ipv4(ip: Ipv4Addr) -> bool {
    let o = ip.octets();
    o[0] == 10 || (o[0] == 172 && (16..=31).contains(&o[1])) || (o[0] == 192 && o[1] == 168)
}

/// İstemci yol listesi (sözleşme): sıra her zaman merkez → şube → yerel. Adresi bilinmeyen yol listeye girmez;
/// `yerel` (cihazın kendi kutusu) her zaman sondadır. Merkez yalnız https olabilir.
pub fn yol_listesi(merkez: Option<&str>, sube: Option<&str>) -> Result<Value, String> {
    let mut v = vec![];
    if let Some(m) = merkez {
        let k = koken(m)?;
        if !k.starts_with("https://") {
            return Err(format!("merkez adresi https:// olmalı: '{m}'"));
        }
        v.push(json!({"ad": "merkez", "url": k}));
    }
    if let Some(s) = sube {
        v.push(json!({"ad": "sube", "url": koken(s)?}));
    }
    v.push(json!({"ad": "yerel"}));
    Ok(Value::Array(v))
}

/// Merkez adresi: veri kökündeki ulaşım yolu listesinin (taa-yol, `yol/sunucu-yollari.json`) ilk yolu.
pub fn merkez_adresi(veri_kok: &Path) -> Option<String> {
    let y = taa_yol::yollar_oku(veri_kok).ok()??;
    y.yollar.first().and_then(|y| koken(&y.taban).ok())
}

/// Bu bilgisayarın yerel ağ adresi. Paket göndermez: UDP soketi yalnız "hangi arayüzden çıkılır" diye sorar.
/// Yerel ağ adresi değilse (doğrudan internete bağlı, yalnız VPN vb.) `None`.
pub fn yerel_ag_adresi() -> Option<Ipv4Addr> {
    for hedef in ["192.168.255.254:9", "10.255.255.254:9", "172.31.255.254:9"] {
        let Ok(s) = UdpSocket::bind("0.0.0.0:0") else {
            continue;
        };
        if s.connect(hedef).is_err() {
            continue;
        }
        if let Ok(SocketAddr::V4(a)) = s.local_addr() {
            if yerel_ag_ipv4(*a.ip()) {
                return Some(*a.ip());
            }
        }
    }
    None
}

// ---------------------------------------------------------------- hizmetler ve komutlar

/// Başlatılacak bir hizmet.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hizmet {
    pub ad: &'static str,
    pub exe: PathBuf,
    pub argumanlar: Vec<String>,
    pub port: u16,
}

fn metin(p: &Path) -> String {
    p.display().to_string()
}

/// `taa-yazdir --kok <veri kökü> calis` (crates/taa-yazdir/src/main.rs).
pub fn yazdir_argumanlari(veri_kok: &Path) -> Vec<String> {
    vec!["--kok".into(), metin(veri_kok), "calis".into()]
}

/// Düğümün komut satırı girdileri (crates/taa-sube-dugum/src/main.rs).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DugumAyari {
    /// Kapı; varsayılandan (8788) farklıysa yazılır.
    pub port: u16,
    /// Şube bilgisayarının yerel ağ adresi. Yoksa düğüm yalnız 127.0.0.1'de dinler (telefonlar ulaşamaz).
    pub lan: Option<Ipv4Addr>,
    /// Düğüme ulaşılan ek ad (şube adresi IP yerine adla yazıldıysa).
    pub ad: Option<String>,
    /// Düğüme yazmasına izin verilen başka köken: merkezden açılan TAA Web.
    pub koken: Option<String>,
    /// Merkez adresi; yalnız ulaşım yolu listesi (taa-yol) yokken verilir, varsa düğüm listeyi kendisi kullanır.
    pub merkez: Option<String>,
}

/// `taa-sube-dugum calis --kok <veri kökü> [--port N] [--lan IPv4] [--ad ad] [--koken köken] [--merkez adres]`.
/// Merkez oturum anahtarı komut satırına yazılmaz (düğüm `TAA_OTURUM` ya da `--oturum-dosyasi` ister; kurulum vermez).
pub fn dugum_argumanlari(veri_kok: &Path, a: &DugumAyari) -> Vec<String> {
    let mut v = vec!["calis".to_string(), "--kok".into(), metin(veri_kok)];
    let mut ekle = |ad: &str, deger: Option<String>| {
        if let Some(d) = deger {
            v.push(ad.to_string());
            v.push(d);
        }
    };
    ekle("--port", (a.port != DUGUM_PORT).then(|| a.port.to_string()));
    ekle("--lan", a.lan.map(|i| i.to_string()));
    ekle("--ad", a.ad.clone());
    ekle("--koken", a.koken.clone());
    ekle("--merkez", a.merkez.clone());
    v
}

/// Kökenin ana bilgisayar kısmı (kapısız).
fn koken_ana(k: &str) -> Option<&str> {
    let kalan = k.split_once("://")?.1;
    Some(kalan.rsplit_once(':').map(|(a, _)| a).unwrap_or(kalan))
}

/// Şube adresi yerel ağ IPv4'ü olarak yazıldıysa o adres.
pub fn sube_ip(sube: Option<&str>) -> Option<Ipv4Addr> {
    sube.and_then(koken_ana)
        .and_then(|a| a.parse::<Ipv4Addr>().ok())
        .filter(|i| yerel_ag_ipv4(*i))
}

/// Profildeki adreslerden düğüm ayarı. `lan` düğümün dinleyeceği yerel ağ adresidir (bkz. `sube_pc_lan`); şube
/// adresi adla yazıldıysa `--ad` olur. Merkez kökeni düğüme yazabilsin diye `--koken` olarak verilir.
pub fn dugum_ayari(
    port: u16,
    merkez: Option<&str>,
    sube: Option<&str>,
    lan: Option<Ipv4Addr>,
    yol_listesi_var: bool,
) -> DugumAyari {
    DugumAyari {
        port,
        lan,
        ad: sube
            .and_then(koken_ana)
            .filter(|a| a.parse::<Ipv4Addr>().is_err())
            .map(str::to_string),
        koken: merkez.map(str::to_string),
        merkez: merkez.filter(|_| !yol_listesi_var).map(str::to_string),
    }
}

/// Düğümün dinleyeceği adres: şube adresindeki IP bu bilgisayardaysa o; değilse (adres değişmiş ya da adla
/// yazılmış) o anki yerel ağ adresi. Bu bilgisayarda olmayan adres verilmez: düğüm o adresi dinleyemez ve hiç açılmaz.
pub fn lan_sec(sube: Option<&str>, sistem: &dyn Sistem) -> Option<Ipv4Addr> {
    sube_ip(sube)
        .filter(|ip| sistem.adres_bu_bilgisayarda(*ip))
        .or_else(|| sistem.yerel_ag_adresi())
}

fn tirnakla(s: &str) -> String {
    format!("\"{s}\"")
}

/// Açılışta çalışacak komut: pencere açmayan `taa-kur-pencere --sube-pc-baslat`.
pub fn gorev_komutu(kok: &Path) -> String {
    format!(
        "{} {BASLAT_BAYRAGI}",
        tirnakla(&metin(&kok.join("taa-kur-pencere.exe")))
    )
}

fn dizgeler(a: &[&str]) -> Vec<String> {
    a.iter().map(|s| s.to_string()).collect()
}

/// `schtasks`: kullanıcı oturum açınca, yükseltilmemiş yetkiyle çalışan görev (taa-dns `zamanla` ile aynı araç).
pub fn schtasks_kur(komut: &str) -> Vec<String> {
    dizgeler(&[
        "/Create", "/TN", GOREV_ADI, "/SC", "ONLOGON", "/RL", "LIMITED", "/TR", komut, "/F",
    ])
}

/// `schtasks` yönetici ister ve reddedilirse: kullanıcının Run kaydı (yönetici gerekmez).
pub fn run_kaydi_kur(komut: &str) -> Vec<String> {
    dizgeler(&[
        "add",
        RUN_ANAHTARI,
        "/v",
        GOREV_ADI,
        "/t",
        "REG_SZ",
        "/d",
        komut,
        "/f",
    ])
}

/// Güvenlik duvarı kuralı (docs/SUBE-DUGUMU.md §7 madde 2): gelen yön, yalnız düğüm exe'si, tek TCP kapısı,
/// yalnız Özel profil, uzak adres yalnız yerel alt ağ.
pub fn guvenlik_duvari_kur(exe: &Path, port: u16) -> Vec<String> {
    vec![
        "advfirewall".into(),
        "firewall".into(),
        "add".into(),
        "rule".into(),
        format!("name={KURAL_ADI}"),
        "dir=in".into(),
        "action=allow".into(),
        format!("program={}", metin(exe)),
        "protocol=TCP".into(),
        format!("localport={port}"),
        "profile=private".into(),
        "remoteip=localsubnet".into(),
    ]
}

/// İşletim sistemi çağrıları. Testler sahte uygulama verir; gerçek uygulama [`GercekSistem`].
pub trait Sistem {
    /// Komutu çalıştırır: (başarılı mı, çıktı).
    fn calistir(&self, program: &str, argumanlar: &[String]) -> Result<(bool, String), String>;
    /// Hizmeti arka planda (penceresiz) başlatır; çıktısı `cikti` dosyasına eklenir. Süreç kimliğini döndürür.
    fn baslat(&self, exe: &Path, argumanlar: &[String], cikti: &Path) -> Result<u32, String>;
    /// Bu bilgisayarda kapı dinleniyor mu.
    fn port_acik(&self, port: u16) -> bool;
    /// Bu bilgisayarın yerel ağ adresi (bkz. [`yerel_ag_adresi`]).
    fn yerel_ag_adresi(&self) -> Option<Ipv4Addr> {
        yerel_ag_adresi()
    }
    /// Adres bu bilgisayarın arayüzlerinden birinde mi (o adrese bağlanılabiliyorsa evet; paket gönderilmez).
    fn adres_bu_bilgisayarda(&self, ip: Ipv4Addr) -> bool {
        UdpSocket::bind(SocketAddr::new(IpAddr::V4(ip), 0)).is_ok()
    }
    /// Görev ve güvenlik duvarı araçları var mı (yalnız Windows).
    fn windows(&self) -> bool {
        cfg!(windows)
    }
}

pub struct GercekSistem;

impl Sistem for GercekSistem {
    fn calistir(&self, program: &str, argumanlar: &[String]) -> Result<(bool, String), String> {
        let mut c = std::process::Command::new(program);
        c.args(argumanlar);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        let o = c
            .output()
            .map_err(|e| format!("{program} çalıştırılamadı: {e}"))?;
        let m = format!(
            "{}{}",
            String::from_utf8_lossy(&o.stdout),
            String::from_utf8_lossy(&o.stderr)
        );
        Ok((o.status.success(), m.trim().to_string()))
    }

    fn baslat(&self, exe: &Path, argumanlar: &[String], cikti: &Path) -> Result<u32, String> {
        use std::process::Stdio;
        if let Some(u) = cikti.parent() {
            std::fs::create_dir_all(u).map_err(|e| format!("{}: {e}", u.display()))?;
        }
        let dosya = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(cikti)
            .map_err(|e| format!("{}: {e}", cikti.display()))?;
        let hata = dosya
            .try_clone()
            .map_err(|e| format!("{}: {e}", cikti.display()))?;
        let mut c = std::process::Command::new(exe);
        c.args(argumanlar)
            .stdin(Stdio::null())
            .stdout(dosya)
            .stderr(hata);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
        }
        c.spawn()
            .map(|s| s.id())
            .map_err(|e| format!("{} başlatılamadı: {e}", exe.display()))
    }

    fn port_acik(&self, port: u16) -> bool {
        let mut adresler = vec![IpAddr::V4(Ipv4Addr::LOCALHOST)];
        // Düğüm yalnız yerel ağ arayüzünü dinleyebilir; o adres de denenir.
        if let Some(ip) = yerel_ag_adresi() {
            adresler.push(IpAddr::V4(ip));
        }
        adresler.into_iter().any(|a| {
            TcpStream::connect_timeout(&SocketAddr::new(a, port), Duration::from_millis(400))
                .is_ok()
        })
    }
}

// ---------------------------------------------------------------- profil

/// `sube-pc kur` seçenekleri.
#[derive(Debug, Clone, Default)]
pub struct Secenek {
    /// Merkez adresi (https). Verilmezse veri kökündeki ulaşım yolu listesinden alınır.
    pub merkez: Option<String>,
    /// Şube bilgisayarının telefonlara görünen adresi. Verilmezse `http://<yerel ağ IP>:<kapı>`.
    pub sube_adresi: Option<String>,
    /// Düğüm kapısı; `None` → 8788.
    pub port: Option<u16>,
    /// Açılış görevi kaydedilmesin.
    pub gorevsiz: bool,
    /// Hizmetler şimdi başlatılmasın.
    pub baslatma: bool,
    /// Güvenlik duvarı kuralı kurulmasın.
    pub duvarsiz: bool,
    /// Düğüm yerel ağa açılsın (`--lan`): telefonlar yazabilir. Varsayılan KAPALI: düğümde cihaz/kişi doğrulaması
    /// henüz yok (docs/SUBE-DUGUMU.md §13.6 madde 1); açılınca şube ağındaki her cihaz kayıt yazabilir.
    pub lan_ac: bool,
}

fn exe_adi(ad: &str) -> String {
    if cfg!(windows) {
        format!("{ad}.exe")
    } else {
        ad.to_string()
    }
}

fn simdi() -> (String, String) {
    crate::zaman::simdi()
}

fn diger(e: impl std::fmt::Display) -> KurHata {
    KurHata::Diger(e.to_string())
}

/// Var olan dosyanın yerine yenisi yazılacaksa eskisini `<klasör>/eski/<ad>-<damga>.<uzantı>` olarak yana taşır.
fn eskiye_tasi(yol: &Path) -> Sonuc<Option<PathBuf>> {
    if !yol.exists() {
        return Ok(None);
    }
    let klasor = yol.parent().unwrap_or(Path::new(".")).join("eski");
    std::fs::create_dir_all(&klasor)?;
    let govde = yol.file_stem().and_then(|s| s.to_str()).unwrap_or("dosya");
    let uzanti = yol.extension().and_then(|s| s.to_str()).unwrap_or("json");
    let damga = simdi().1;
    let mut hedef = klasor.join(format!("{govde}-{damga}.{uzanti}"));
    let mut n = 1;
    while hedef.exists() {
        n += 1;
        hedef = klasor.join(format!("{govde}-{damga}-{n}.{uzanti}"));
    }
    std::fs::rename(yol, &hedef)?;
    Ok(Some(hedef))
}

/// İçerik aynıysa dokunmaz; farklıysa eskisini `eski\` altına taşıyıp yenisini yazar. Dönüş: (yazıldı mı, eski kopya).
fn degistiyse_yaz(yol: &Path, icerik: &str) -> Sonuc<(bool, Option<PathBuf>)> {
    if std::fs::read_to_string(yol).ok().as_deref() == Some(icerik) {
        return Ok((false, None));
    }
    let eski = eskiye_tasi(yol)?;
    taa_dosya::uzerine_yaz_atomik(yol, icerik.as_bytes()).map_err(diger)?;
    Ok((true, eski))
}

fn guzel(v: &Value) -> String {
    let mut s = serde_json::to_string_pretty(v).unwrap_or_default();
    s.push('\n');
    s
}

/// `taa-yazdir` ayarı: dosya yoksa en küçük geçerli ayar yazılır (yazıcı hedefi kullanıcıya kalır); varsa yalnız
/// eksik kökenler eklenir, öteki alanlara dokunulmaz. Bozuk dosyaya dokunulmaz, hata döner.
/// Dönüş: (ayar dosyası, eklenen kökenler, köprünün kapısı).
fn yazdir_ayari(veri_kok: &Path, kokenler: &[String]) -> Sonuc<(PathBuf, Vec<String>, u16)> {
    let yol = veri_kok.join("yazdir").join("ayar.json");
    let mut v: Value = match std::fs::read_to_string(&yol) {
        Ok(m) => serde_json::from_str(&m).map_err(|e| {
            KurHata::Bozuk(format!(
                "taa-yazdir ayarı okunamadı ({}): {e}. Öneri: dosyayı düzeltin ya da adını değiştirip kurulumu yeniden çalıştırın.",
                yol.display()
            ))
        })?,
        Err(_) => json!({
            "aciklama": "taa-yazdir ayarı. izinli_kokenler: TAA Web'in açıldığı adresler (kurulum yazdı). yazicilar: \"argox-pplb\" / \"escpos\" için dosya ya da paylaşım yolu veya tcp://IP[:9100] yazın.",
            "port": YAZDIR_PORT,
            "izinli_kokenler": [],
            "yazicilar": {},
            "cihaz_imza": "kapali"
        }),
    };
    let nesne = v.as_object_mut().ok_or_else(|| {
        KurHata::Bozuk(format!(
            "taa-yazdir ayarı bir JSON nesnesi değil: {}",
            yol.display()
        ))
    })?;
    let port = nesne
        .get("port")
        .and_then(Value::as_u64)
        .and_then(|p| u16::try_from(p).ok())
        .unwrap_or(YAZDIR_PORT);
    let liste = nesne
        .entry("izinli_kokenler")
        .or_insert_with(|| json!([]))
        .as_array_mut()
        .ok_or_else(|| {
            KurHata::Bozuk(format!(
                "taa-yazdir ayarında izinli_kokenler liste değil: {}",
                yol.display()
            ))
        })?;
    let mut eklenen = vec![];
    for k in kokenler {
        let var = liste.iter().any(|x| {
            x.as_str()
                .is_some_and(|s| s.trim().trim_end_matches('/').eq_ignore_ascii_case(k))
        });
        if !var {
            liste.push(json!(k));
            eklenen.push(k.clone());
        }
    }
    degistiyse_yaz(&yol, &guzel(&v))?;
    Ok((yol, eklenen, port))
}

fn profil_oku(kok: &Path) -> Sonuc<Value> {
    let yol = kok.join(PROFIL_DOSYASI);
    let m = std::fs::read_to_string(&yol).map_err(|_| {
        KurHata::Yok(format!(
            "Şube bilgisayarı profili kurulu değil ({} yok). Öneri: `taa-kur sube-pc kur` çalıştırın.",
            yol.display()
        ))
    })?;
    serde_json::from_str(&m)
        .map_err(|e| KurHata::Bozuk(format!("{} okunamadı: {e}", yol.display())))
}

fn profilden(p: &Value) -> Sonuc<(PathBuf, u16)> {
    let veri = p["veri_kok"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| KurHata::Bozuk(format!("{PROFIL_DOSYASI} içinde veri_kok yok.")))?;
    let port = p["dugum_port"]
        .as_u64()
        .and_then(|p| u16::try_from(p).ok())
        .unwrap_or(DUGUM_PORT);
    Ok((veri, port))
}

impl Kur {
    /// Hizmet ikilisini bulur: önce kurulum kökü (`<kök>\<ad>.exe`), sonra paket deposundaki seçili sürüm
    /// (`<kök>\depo\<ad>\<sürüm>\<ad>.exe`).
    pub fn sube_pc_ikili(&self, ad: &str) -> Option<PathBuf> {
        let dosya = exe_adi(ad);
        let kokte = self.kok.join(&dosya);
        if kokte.is_file() {
            return Some(kokte);
        }
        let secili = self
            .liste()
            .ok()?
            .into_iter()
            .find(|s| s.ad == ad)
            .and_then(|s| s.secili)?;
        let depoda = self.kok.join("depo").join(ad).join(secili).join(&dosya);
        depoda.is_file().then_some(depoda)
    }

    fn sube_pc_hizmetler(
        &self,
        veri_kok: &Path,
        dugum: &DugumAyari,
        yazdir_port: u16,
    ) -> Vec<Hizmet> {
        let mut v = vec![];
        if let Some(exe) = self.sube_pc_ikili(YAZDIR) {
            v.push(Hizmet {
                ad: YAZDIR,
                exe,
                argumanlar: yazdir_argumanlari(veri_kok),
                port: yazdir_port,
            });
        }
        if let Some(exe) = self.sube_pc_ikili(DUGUM) {
            v.push(Hizmet {
                ad: DUGUM,
                exe,
                argumanlar: dugum_argumanlari(veri_kok, dugum),
                port: dugum.port,
            });
        }
        v
    }

    /// Profilden (sube-pc.json) düğüm ayarı; yerel ağ adresi başlatma anında yeniden sorulur.
    fn sube_pc_dugum_ayari(
        profil: &Value,
        veri: &Path,
        port: u16,
        sistem: &dyn Sistem,
    ) -> DugumAyari {
        dugum_ayari(
            port,
            profil["merkez"].as_str(),
            profil["sube"].as_str(),
            profil["lan_acik"]
                .as_bool()
                .unwrap_or(false)
                .then(|| lan_sec(profil["sube"].as_str(), sistem))
                .flatten(),
            matches!(taa_yol::yollar_oku(veri), Ok(Some(_))),
        )
    }

    fn sube_pc_yazdir_portu(veri_kok: &Path) -> u16 {
        std::fs::read_to_string(veri_kok.join("yazdir").join("ayar.json"))
            .ok()
            .and_then(|m| serde_json::from_str::<Value>(&m).ok())
            .and_then(|v| v["port"].as_u64())
            .and_then(|p| u16::try_from(p).ok())
            .unwrap_or(YAZDIR_PORT)
    }

    /// Şube bilgisayarı profilini kurar (veri kökü ortamdan).
    pub fn sube_pc_kur(&self, secenek: &Secenek, sistem: &dyn Sistem) -> Sonuc<Value> {
        self.sube_pc_kur_veri_ile(crate::veri::veri_kok_ortamdan(), secenek, sistem)
    }

    /// `sube_pc_kur` ile aynı; veri kökü verilir (testler sahte kök kullanır).
    pub fn sube_pc_kur_veri_ile(
        &self,
        veri_kok: Option<PathBuf>,
        secenek: &Secenek,
        sistem: &dyn Sistem,
    ) -> Sonuc<Value> {
        let veri = veri_kok.ok_or_else(|| {
            KurHata::Cozum(
                "Veri kökü bulunamadı (TAA_VERI_KOK ya da APPDATA). Öneri: TAA_VERI_KOK verin."
                    .into(),
            )
        })?;
        // Çatı ve sunucu paketleri şubeye kurulmaz; bu profil de sunucu bilgisayarında çalışmaz.
        let rol = std::fs::read_to_string(self.kok.join("rol.txt"))
            .ok()
            .map(|r| r.trim().to_string())
            .filter(|r| !r.is_empty());
        if rol.as_deref() == Some("sunucu") {
            return Err(KurHata::Dogrulama(
                "Bu bilgisayarın rolü sunucu (rol.txt). Şube bilgisayarı profili yalnız şube bilgisayarına kurulur."
                    .into(),
            ));
        }
        let port = secenek.port.unwrap_or(DUGUM_PORT);
        if port < 1024 {
            return Err(KurHata::Dogrulama(
                "Düğüm kapısı 1024-65535 arasında olmalı.".into(),
            ));
        }
        let (yazdir, dugum) = (self.sube_pc_ikili(YAZDIR), self.sube_pc_ikili(DUGUM));
        if yazdir.is_none() && dugum.is_none() {
            return Err(KurHata::Yok(format!(
                "{YAZDIR} ve {DUGUM} bu bilgisayarda bulunamadı ({} ya da depo\\ altında). Öneri: kurulum paketinin bu ikilileri içerdiğini denetleyin ya da `taa-guncelle kur {DUGUM}` çalıştırın.",
                self.kok.display()
            )));
        }
        let eksik: Vec<&str> = [(YAZDIR, &yazdir), (DUGUM, &dugum)]
            .iter()
            .filter(|(_, e)| e.is_none())
            .map(|(a, _)| *a)
            .collect();

        // Adresler ve yol listesi.
        let merkez = match &secenek.merkez {
            Some(m) => Some(koken(m).map_err(KurHata::Dogrulama)?),
            None => merkez_adresi(&veri),
        };
        let sube_adresi = match &secenek.sube_adresi {
            Some(s) => Some(koken(s).map_err(KurHata::Dogrulama)?),
            // Yerel ağ kapalıyken düğüme yalnız bu bilgisayardan ulaşılır.
            None if !secenek.lan_ac => Some(format!("http://127.0.0.1:{port}")),
            None => sistem
                .yerel_ag_adresi()
                .map(|ip| format!("http://{ip}:{port}")),
        };
        let yollar =
            yol_listesi(merkez.as_deref(), sube_adresi.as_deref()).map_err(KurHata::Dogrulama)?;

        // Günlük klasörü: düğüm buraya YYYY-AA-GG.jsonl ekler. Var olan hiçbir dosyaya dokunulmaz.
        let gunluk = veri.join("gunluk");
        std::fs::create_dir_all(&gunluk)?;
        let alan = veri.join("sube-pc");
        std::fs::create_dir_all(&alan)?;
        let yol_dosyasi = alan.join(YOL_DOSYASI);
        let (_, eski_yollar) = degistiyse_yaz(&yol_dosyasi, &guzel(&yollar))?;

        // taa-yazdir ayarı: TAA Web'in açılabileceği kökenler (merkez ve şube) izinli olmalı.
        let kokenler: Vec<String> = merkez.iter().chain(sube_adresi.iter()).cloned().collect();
        let (yazdir_ayar, eklenen_kokenler, yazdir_port) = yazdir_ayari(&veri, &kokenler)?;

        // Profil: açılıştaki `baslat` bunu okur.
        let profil = json!({
            "sema": 1,
            "veri_kok": metin(&veri),
            "dugum_port": port,
            "merkez": merkez,
            "sube": sube_adresi,
            "lan_acik": secenek.lan_ac,
            "yol_dosyasi": metin(&yol_dosyasi),
            "gunluk": metin(&gunluk),
        });
        degistiyse_yaz(&self.kok.join(PROFIL_DOSYASI), &guzel(&profil))?;
        let dugum_ayar = Self::sube_pc_dugum_ayari(&profil, &veri, port, sistem);
        let hizmetler = self.sube_pc_hizmetler(&veri, &dugum_ayar, yazdir_port);

        // Güvenlik duvarı kuralı (yönetici ister; kurulamazsa adım olarak bildirilir).
        let duvar = match (&dugum, secenek.duvarsiz, sistem.windows()) {
            (None, _, _) => json!({"durum": "atlandi", "ayrinti": format!("{DUGUM} yok")}),
            _ if !secenek.lan_ac => {
                json!({"durum": "atlandi", "ayrinti": "düğüm yerel ağa açık değil (--lan-ac verilmedi)"})
            }
            (_, true, _) => json!({"durum": "atlandi", "ayrinti": "--duvarsiz"}),
            (_, _, false) => {
                json!({"durum": "atlandi", "ayrinti": "yalnız Windows'ta kurulur"})
            }
            (Some(exe), false, true) => {
                match sistem.calistir("netsh", &guvenlik_duvari_kur(exe, port)) {
                    Ok((true, _)) => json!({"durum": "kuruldu", "kural": KURAL_ADI}),
                    Ok((false, m)) | Err(m) => json!({
                        "durum": "yapilamadi",
                        "ayrinti": m,
                        "oneri": format!("Yönetici olarak çalıştırın: netsh {}", guvenlik_duvari_kur(exe, port).join(" ")),
                    }),
                }
            }
        };

        // Açılış görevi: önce schtasks (oturum açılışı), reddedilirse kullanıcının Run kaydı.
        let komut = gorev_komutu(&self.kok);
        let gorev = if secenek.gorevsiz {
            json!({"durum": "atlandi", "ayrinti": "--gorevsiz"})
        } else if !sistem.windows() {
            json!({"durum": "atlandi", "ayrinti": "yalnız Windows'ta kaydedilir", "komut": komut})
        } else {
            match sistem.calistir("schtasks", &schtasks_kur(&komut)) {
                Ok((true, _)) => json!({"durum": "kuruldu", "tur": "gorev", "komut": komut}),
                ilk => {
                    let neden = match ilk {
                        Ok((_, m)) | Err(m) => m,
                    };
                    match sistem.calistir("reg", &run_kaydi_kur(&komut)) {
                        Ok((true, _)) => {
                            json!({"durum": "kuruldu", "tur": "run-kaydi", "komut": komut, "gorev_neden_olmadi": neden})
                        }
                        Ok((false, m)) | Err(m) => {
                            json!({"durum": "yapilamadi", "ayrinti": format!("schtasks: {neden}; reg: {m}"), "komut": komut})
                        }
                    }
                }
            }
        };

        let baslatma = if secenek.baslatma {
            json!([])
        } else {
            Value::Array(Self::hizmetleri_baslat(&hizmetler, &alan, sistem))
        };

        let mut uyarilar =
            Self::sube_pc_uyarilari(merkez.as_deref(), sube_adresi.as_deref(), &eksik);
        if !secenek.lan_ac {
            if dugum.is_some() {
                uyarilar.push(format!("Şube düğümü yalnız bu bilgisayardan ulaşılır (127.0.0.1:{port}); telefonlar yazamaz. Düğümde cihaz doğrulaması henüz yok; doğrulama bağlanınca ya da şube ağına güveniliyorsa `taa-kur sube-pc kur --lan-ac` ile yerel ağa açın."));
            }
        } else {
            uyarilar.push("Şube düğümü yerel ağa açık ve cihaz doğrulaması yok: şube ağındaki her cihaz kayıt yazabilir. Yalnız güvenilen (misafire kapalı) şube ağında kullanın.".to_string());
            if let Some(ip) = sube_ip(sube_adresi.as_deref()) {
                if dugum_ayar.lan != Some(ip) {
                    uyarilar.push(format!(
                        "Şube adresindeki {ip} bu bilgisayarın adresi değil: telefonlar şube bilgisayarına ulaşamaz. Öneri: doğru adresle `taa-kur sube-pc kur --lan-ac --sube-adres http://<IP>:{port}` çalıştırın."
                    ));
                }
            }
            if dugum.is_some() && dugum_ayar.lan.is_none() {
                uyarilar.push("Yerel ağ adresi olmadığı için şube düğümü yalnız bu bilgisayardan ulaşılır (127.0.0.1); telefonlar bağlanamaz.".to_string());
            }
        }
        let (zaman, _) = simdi();
        let ozet = json!({
            "veri_kok": metin(&veri),
            "gunluk": metin(&gunluk),
            "yol_dosyasi": metin(&yol_dosyasi),
            "yollar": yollar,
            "eski_yol_dosyasi": eski_yollar.as_deref().map(metin),
            "yazdir_ayar": metin(&yazdir_ayar),
            "eklenen_kokenler": eklenen_kokenler,
            "ikililer": {
                YAZDIR: yazdir.as_deref().map(metin),
                DUGUM: dugum.as_deref().map(metin),
            },
            "eksik": eksik,
            "dugum_port": port,
            "guvenlik_duvari": duvar,
            "gorev": gorev,
            "hizmetler": baslatma,
            "uyarilar": uyarilar,
        });
        // Kurulum izi: yalnız ekleme, silinmez.
        let satir = json!({"zaman": zaman, "olay": "sube-pc.kur", "ozet": ozet});
        taa_dosya::satir_ekle(&alan.join("kurulum.jsonl"), &satir.to_string()).map_err(diger)?;
        Ok(ozet)
    }

    fn sube_pc_uyarilari(merkez: Option<&str>, sube: Option<&str>, eksik: &[&str]) -> Vec<String> {
        let mut u = vec![];
        if merkez.is_none() {
            u.push("Merkez adresi bilinmiyor: yol listesine merkez yazılmadı. Öneri: sunucu-yollari.json kurulduktan sonra (taa-kur yol) `taa-kur sube-pc kur` yeniden çalıştırın ya da --merkez https://… verin.".to_string());
        }
        match sube {
            None => u.push("Bu bilgisayarın yerel ağ adresi bulunamadı: yol listesine şube yazılmadı. Öneri: --sube-adres http://<bu bilgisayarın IP'si>:8788 verin.".to_string()),
            Some(s) if sube_ip(Some(s)).is_some() => u.push(format!("Şube adresi {s} IP ile yazıldı: modemde bu bilgisayara sabit IP (DHCP ayırma) verin, yoksa adres değişince telefonlar şube bilgisayarını bulamaz.")),
            Some(_) => {}
        }
        if sube.is_some_and(|s| !s.starts_with("http://")) {
            u.push("Şube adresi https: taa-sube-dugum bu sürümde yalnız düz HTTP konuşur; önüne sertifikalı bir aktarıcı konmadıysa telefonlar bu adrese ulaşamaz.".to_string());
        }
        for e in eksik {
            u.push(format!(
                "{e} bulunamadı; kurulunca `taa-kur sube-pc kur` yeniden çalıştırın."
            ));
        }
        u
    }

    fn hizmetleri_baslat(hizmetler: &[Hizmet], alan: &Path, sistem: &dyn Sistem) -> Vec<Value> {
        hizmetler
            .iter()
            .map(|h| {
                if sistem.port_acik(h.port) {
                    return json!({"ad": h.ad, "durum": "zaten_calisiyor", "port": h.port});
                }
                let cikti = alan.join("cikti").join(format!("{}.log", h.ad));
                match sistem.baslat(&h.exe, &h.argumanlar, &cikti) {
                    Ok(pid) => {
                        json!({"ad": h.ad, "durum": "baslatildi", "port": h.port, "surec": pid, "cikti": metin(&cikti)})
                    }
                    Err(e) => json!({"ad": h.ad, "durum": "baslatilamadi", "port": h.port, "ayrinti": e}),
                }
            })
            .collect()
    }

    /// Kurulu profilin hizmetlerini başlatır (açılış görevi bunu çağırır). Çalışan hizmet yeniden başlatılmaz.
    pub fn sube_pc_baslat(&self, sistem: &dyn Sistem) -> Sonuc<Value> {
        let profil = profil_oku(&self.kok)?;
        let (veri, port) = profilden(&profil)?;
        let dugum_ayar = Self::sube_pc_dugum_ayari(&profil, &veri, port, sistem);
        let hizmetler =
            self.sube_pc_hizmetler(&veri, &dugum_ayar, Self::sube_pc_yazdir_portu(&veri));
        std::fs::create_dir_all(veri.join("gunluk"))?;
        let alan = veri.join("sube-pc");
        std::fs::create_dir_all(&alan)?;
        let sonuc = Self::hizmetleri_baslat(&hizmetler, &alan, sistem);
        let satir = json!({"zaman": simdi().0, "olay": "sube-pc.baslat", "hizmetler": sonuc});
        taa_dosya::satir_ekle(&alan.join("kurulum.jsonl"), &satir.to_string()).map_err(diger)?;
        Ok(json!({"hizmetler": sonuc}))
    }

    /// Profilin durumu: adresler, ikililer, hizmetler çalışıyor mu, açılış kaydı var mı. Hiçbir şey değiştirmez.
    pub fn sube_pc_durum(&self, sistem: &dyn Sistem) -> Sonuc<Value> {
        let profil = profil_oku(&self.kok)?;
        let (veri, port) = profilden(&profil)?;
        let dugum_ayar = Self::sube_pc_dugum_ayari(&profil, &veri, port, sistem);
        let hizmetler: Vec<Value> = self
            .sube_pc_hizmetler(&veri, &dugum_ayar, Self::sube_pc_yazdir_portu(&veri))
            .iter()
            .map(|h| json!({"ad": h.ad, "port": h.port, "calisiyor": sistem.port_acik(h.port), "exe": metin(&h.exe)}))
            .collect();
        let gorev = if !sistem.windows() {
            json!("bilinmiyor (yalnız Windows)")
        } else if matches!(
            sistem.calistir("schtasks", &dizgeler(&["/Query", "/TN", GOREV_ADI])),
            Ok((true, _))
        ) {
            json!("gorev")
        } else if matches!(
            sistem.calistir("reg", &dizgeler(&["query", RUN_ANAHTARI, "/v", GOREV_ADI])),
            Ok((true, _))
        ) {
            json!("run-kaydi")
        } else {
            json!("yok")
        };
        let yollar = std::fs::read_to_string(veri.join("sube-pc").join(YOL_DOSYASI))
            .ok()
            .and_then(|m| serde_json::from_str::<Value>(&m).ok());
        let eksik: Vec<&str> = [YAZDIR, DUGUM]
            .into_iter()
            .filter(|a| self.sube_pc_ikili(a).is_none())
            .collect();
        Ok(json!({
            "profil": profil,
            "yollar": yollar,
            "hizmetler": hizmetler,
            "eksik": eksik,
            "acilis_kaydi": gorev,
            "gunluk_var": veri.join("gunluk").is_dir(),
        }))
    }

    /// Açılış kaydını kaldırır (bilgisayar başka işe verilirken). Dosya, ayar, günlük ve veri SİLİNMEZ; çalışan
    /// hizmet durdurulmaz (bir sonraki açılışta başlamaz).
    pub fn sube_pc_gorev_kaldir(&self, sistem: &dyn Sistem) -> Sonuc<Value> {
        if !sistem.windows() {
            return Err(KurHata::Dogrulama(
                "Açılış kaydı yalnız Windows'ta vardır.".into(),
            ));
        }
        let g = sistem
            .calistir("schtasks", &dizgeler(&["/Delete", "/TN", GOREV_ADI, "/F"]))
            .map(|(ok, _)| ok)
            .unwrap_or(false);
        let r = sistem
            .calistir(
                "reg",
                &dizgeler(&["delete", RUN_ANAHTARI, "/v", GOREV_ADI, "/f"]),
            )
            .map(|(ok, _)| ok)
            .unwrap_or(false);
        Ok(json!({"gorev_kaldirildi": g, "run_kaydi_kaldirildi": r}))
    }
}

/// `taa-kur-pencere --sube-pc-baslat`: açılışta pencere açmadan hizmetleri başlatır. Çıkış kodu döner.
pub fn acilista_baslat() -> i32 {
    match Kur::ortamdan().and_then(|k| k.sube_pc_baslat(&GercekSistem)) {
        Ok(_) => 0,
        Err(e) => e.cikis_kodu(),
    }
}
