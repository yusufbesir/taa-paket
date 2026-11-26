//! taa-fis-sunucu: TaaFiş yerel sunucusu. Kullanım:
//!   taa-fis-sunucu [--veri KLASÖR] [--port 8787] [--yazici YOL|tcp://IP[:9100]] [--adres http://192.168.1.20:8787]
//!                  [--aydinlatma-url https://...] [--kok TAA_VERI_KOKU] [--sunucu CATI_ADRESI] [--sertifika CA.pem]
//! Masaüstü program: bu bilgisayarda http://127.0.0.1:PORT/masaustu (ya da `taa-terzi-takip` başlatıcısı).
//! Yönetici PIN'i TAA Çatı'da doğrulanır: `--kok` (varsayılan TAA veri kökü) altındaki oturum, cihaz anahtarı ve
//! yol listesi kullanılır; `--sunucu`/`--sertifika` TAA Çatı'daki anlamıyla aynıdır.
//! Müşteri takip: `taa-fis-sunucu --veri KLASÖR --takip-genel-anahtar` bulut sayfasına verilecek GENEL anahtarı
//! yazar ve çıkar (özel anahtar ve takip anahtarı klasörde kalır). Ayar `musteri-takip.json` (bkz. musteri_takip.rs).
//! Teslim bilgisi şifre anahtarı: varsayılan `KLASÖR/teslim-sifre.anahtar`; `--teslim-anahtar YOL` ile veri klasörü
//! dışında (ayrı yedeklenen) bir dosyada tutulabilir. Anahtar yedekte veriden AYRI saklanmalıdır (bkz. teslim_sifre.rs).
use std::io::Read;
use std::path::PathBuf;
use taa_fis_sunucu::{cati_pin, kurulum, Istek, Uygulama, FOTO_SINIRI};

fn arg(ad: &str) -> Option<String> {
    let a: Vec<String> = std::env::args().collect();
    a.iter()
        .position(|x| x == ad)
        .and_then(|i| a.get(i + 1).cloned())
}

/// Yerel ağdaki adresi bulur (paket göndermeden: UDP "bağlantı" yalnız yol seçer).
fn yerel_ip() -> Option<String> {
    let s = std::net::UdpSocket::bind("0.0.0.0:0").ok()?;
    s.connect("192.0.2.1:80").ok()?;
    Some(s.local_addr().ok()?.ip().to_string())
}

fn main() {
    taa_hata::kur_varsayilan("taa-fis-sunucu", env!("CARGO_PKG_VERSION"));
    let port: u16 = arg("--port").and_then(|p| p.parse().ok()).unwrap_or(8787);
    let veri = PathBuf::from(arg("--veri").unwrap_or_else(|| "taa-fis-veri".into()));
    let taban = arg("--adres").unwrap_or_else(|| {
        format!(
            "http://{}:{port}",
            yerel_ip().unwrap_or_else(|| "localhost".into())
        )
    });
    if std::env::args().any(|a| a == "--takip-genel-anahtar") {
        match taa_fis_sunucu::musteri_takip::genel_anahtar(&veri) {
            Ok(g) => {
                println!("{g}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Takip anahtarı okunamadı: {e}");
                std::process::exit(1);
            }
        }
    }
    // Müşteri takip: bekleyen durumlar dakikada bir bulut sayfalarına gönderilir (ayar yoksa bir şey yapmaz).
    {
        let dizin = veri.clone();
        std::thread::spawn(move || loop {
            let simdi = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            if let Err(e) = taa_fis_sunucu::musteri_takip::kargo_degisenleri_kuyruga_al(&dizin, simdi) {
                eprintln!("Müşteri takip kargo bilgisi kuyruğa alınamadı: {e}");
            }
            let g = taa_fis_sunucu::musteri_takip::CurlGonderici;
            if let Err(e) = taa_fis_sunucu::musteri_takip::gonder_bekleyenler(&dizin, &g) {
                eprintln!("Müşteri takip gönderimi yapılamadı: {e}");
            }
            std::thread::sleep(std::time::Duration::from_secs(60));
        });
    }
    // Şube eşitleme: site okunur, şube tablosu değiştiyse (ya da 14 günde bir) bulutlara imzalı itilir. İlk tur 1 dk sonra, sonra 6 saatte bir.
    {
        let dizin = veri.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(60));
            loop {
                let simdi = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let g = taa_fis_sunucu::musteri_takip::CurlGonderici;
                match taa_fis_sunucu::sube_esitle::esitle(&dizin, simdi, &taa_fis_sunucu::sube_esitle::site_getir, &g) {
                    Ok(s) if s.degisti => eprintln!("Şube tablosu güncellendi: {} şube, {} hedefe itildi", s.subeler, s.itilen),
                    Ok(_) => {}
                    Err(e) => eprintln!("Şube eşitleme yapılamadı: {e}"),
                }
                std::thread::sleep(std::time::Duration::from_secs(6 * 3600));
            }
        });
    }
    // Teslim bilgisi şifre anahtarı veri klasörü dışında tutulabilir (ayrı yedeklenir; bkz. teslim_sifre).
    if let Some(y) = arg("--teslim-anahtar") {
        if let Err(e) = taa_fis_sunucu::teslim_sifre::anahtar_yeri_ayarla(PathBuf::from(y)) {
            eprintln!("Teslim anahtarı yeri ayarlanamadı: {e}");
            std::process::exit(1);
        }
    }
    // Eski teslim kayıtlarının şifrelenmesi (bir kezlik, idempotent; bkz. teslim_gecis). Önce defterin şifreli
    // yedeği alınır; hata olursa defter değişmez ve sunucu yine açılır. Günlüğe yalnız sayılar yazılır.
    // Defter zinciri (TT1) başlatılmışsa yeniden yazım yalnız yöneticinin açık onayıyla (`--teslim-gecis-zincir-onay`).
    let zincir_onayi = std::env::args().any(|a| a == "--teslim-gecis-zincir-onay");
    match taa_fis_sunucu::teslim_gecis::calistir(&veri, taa_fis_sunucu::simdi(), zincir_onayi) {
        Ok(r) => {
            if r.zincir_onayi_gerekli {
                eprintln!("Teslim geçişi: defter zinciri başlatılmış; eski teslim satırları yeniden yazılmadı. Yönetici onaylıyorsa programı --teslim-gecis-zincir-onay ile açın (merkezdeki zincir ucu yenilenmeli).");
            }
            if r.defter_yeniden_yazilan > 0 || r.imza_sifrelenen > 0 {
                eprintln!(
                    "Teslim geçişi: {} defter satırı yeniden yazıldı ({} düz metin şifrelendi), {} imza şifrelendi; şifreli yedek: {}",
                    r.defter_yeniden_yazilan,
                    r.defter_sifrelenen,
                    r.imza_sifrelenen,
                    r.yedek.as_deref().unwrap_or("-")
                );
            }
            if !r.duz_imza_bekleyen.is_empty() {
                eprintln!(
                    "Teslim geçişi: {} şifresiz imza dosyası yönetici onayıyla silinmeyi bekliyor (imzalar/{}).",
                    r.duz_imza_bekleyen.len(),
                    taa_fis_sunucu::teslim_gecis::SILINECEK_LISTESI
                );
            }
        }
        Err(e) => eprintln!("Teslim geçişi yapılamadı (defter değişmedi): {e}"),
    }
    let mut uyg = Uygulama::yeni(veri, arg("--yazici").map(PathBuf::from), taban.clone());
    // Karekod onayı: süresi geçen karekodların bulut kaydı dakikada bir kapattırılır (Vercel Blob'da kendi silme
    // zamanı yok; şifreli kayıt 15 dk + en çok 1 dk içinde silinir, sunucu açık ve internete ulaşıyorsa).
    {
        let temizleyici = uyg.onay_temizleyici();
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(60));
            temizleyici.calistir();
        });
    }
    uyg.aydinlatma_url = arg("--aydinlatma-url");
    let kok = arg("--kok")
        .map(PathBuf::from)
        .unwrap_or_else(cati_pin::veri_koku);
    uyg.pin = Box::new(cati_pin::CatiPinIstemcisi {
        kok: kok.clone(),
        sunucu: arg("--sunucu"),
        sertifika: arg("--sertifika").map(PathBuf::from),
    });
    let cati: std::sync::Arc<dyn kurulum::CatiKaynagi> =
        std::sync::Arc::new(kurulum::CatiIstemcisi {
            kok: kok.clone(),
            sunucu: arg("--sunucu"),
            sertifika: arg("--sertifika").map(PathBuf::from),
        });
    uyg.cati = cati.clone();
    uyg.kok = Some(kok.clone());
    // Kişi listesi ve şube: açılışta ve periyodik olarak TAA Çatı'dan; ulaşılamazsa son önbellek kalır.
    {
        let dizin = uyg.dizin.clone();
        std::thread::spawn(move || loop {
            match kurulum::esitle(&dizin, Some(&kok), cati.as_ref(), taa_fis_sunucu::simdi()) {
                Ok(o) => println!("TAA Çatı eşitlemesi: {o}"),
                Err(e) => {
                    eprintln!("TAA Çatı eşitlemesi yapılamadı (son önbellek kullanılıyor): {e}")
                }
            }
            std::thread::sleep(std::time::Duration::from_secs(kurulum::ESITLEME_ARALIGI_SN));
        });
    }
    let sunucu = match tiny_http::Server::http(format!("0.0.0.0:{port}")) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Sunucu açılamadı (port {port} dolu olabilir): {e}");
            std::process::exit(1);
        }
    };
    println!("TaaFiş sunucusu hazır. Telefon adresi: {taban}/telefon");
    println!("Masaüstü program (bu bilgisayarda): http://127.0.0.1:{port}/masaustu");
    for mut r in sunucu.incoming_requests() {
        let baslik = |ad: &str| {
            r.headers()
                .iter()
                .find(|h| h.field.as_str().as_str().eq_ignore_ascii_case(ad))
                .map(|h| h.value.to_string())
        };
        // Yerel: loopback adresten VE Host başlığı loopback (DNS yeniden bağlama saldırısına karşı) VE önde
        // bir aracı (Caddy, Tailscale Serve) yok: aracı her bağlantıyı 127.0.0.1'den gösterir (Fable E1); aracının
        // eklediği başlıklardan biri varsa istek yerel sayılmaz.
        let host_yerel = baslik("Host").is_some_and(|h| host_loopback(&h));
        let aracidan = [
            "X-Forwarded-For",
            "Forwarded",
            "X-Real-IP",
            "Tailscale-User-Login",
        ]
        .iter()
        .any(|a| baslik(a).is_some());
        let yerel = host_yerel
            && !aracidan
            && r.remote_addr()
                .map(|a| a.ip().is_loopback())
                .unwrap_or(false);
        let yetki = baslik("Authorization");
        let masaustu = baslik("X-TAA-Masaustu").as_deref() == Some("1");
        let yonetici = baslik("X-TAA-Yonetici");
        let mut govde = Vec::new();
        let _ = r
            .as_reader()
            .take(FOTO_SINIRI as u64 + 1)
            .read_to_end(&mut govde);
        // Ortak giriş (taa-kimlik aracı; ağa çıkabilir, en çok birkaç sn) ayrı iş parçacığında: tek iş parçacıklı
        // sunucu bu sürede telefon isteklerini bekletmesin (Fable E2). Kapı aynı: yalnız bu bilgisayardaki program.
        if r.method() == &tiny_http::Method::Post && yerel && masaustu {
            if let Some(islem) = r.url().strip_prefix("/api/giris/").map(str::to_string) {
                let bul = uyg.kimlik_bulucu;
                std::thread::spawn(move || {
                    let y = match taa_fis_sunucu::ortak_giris::yanitla(
                        bul,
                        taa_fis_sunucu::cati_pin::PROGRAM,
                        &islem,
                        &govde,
                    ) {
                        Some(v) => (200, v.to_string()),
                        None => (404, r#"{"hata":"bulunamadı"}"#.to_string()),
                    };
                    let mut cevap = tiny_http::Response::from_string(y.1).with_status_code(y.0);
                    for (a, d) in [
                        ("Content-Type", "application/json; charset=utf-8"),
                        ("X-Content-Type-Options", "nosniff"),
                        ("Cache-Control", "no-store"),
                        ("X-Frame-Options", "DENY"),
                        ("Referrer-Policy", "no-referrer"),
                    ] {
                        if let Ok(h) = tiny_http::Header::from_bytes(a.as_bytes(), d.as_bytes()) {
                            cevap.add_header(h);
                        }
                    }
                    let _ = r.respond(cevap);
                });
                continue;
            }
        }
        let y = uyg.isle(&Istek {
            metot: r.method().as_str().to_string(),
            yol: r.url().to_string(),
            yetki,
            govde,
            yerel,
            masaustu,
            yonetici,
        });
        let mut cevap = tiny_http::Response::from_data(y.govde).with_status_code(y.kod);
        for (a, d) in [
            ("Content-Type", y.tur),
            ("X-Content-Type-Options", "nosniff"),
            ("Cache-Control", "no-store"),
            ("X-Frame-Options", "DENY"),
            ("Referrer-Policy", "no-referrer"),
        ] {
            if let Ok(h) = tiny_http::Header::from_bytes(a.as_bytes(), d.as_bytes()) {
                cevap.add_header(h);
            }
        }
        let _ = r.respond(cevap);
    }
}

/// `Host` başlığı 127.0.0.1, localhost ya da [::1] mi (port olsun olmasın)?
fn host_loopback(h: &str) -> bool {
    let h = h.trim().to_ascii_lowercase();
    let ad = if let Some(r) = h.strip_prefix('[') {
        r.split(']').next().unwrap_or("")
    } else {
        h.split(':').next().unwrap_or("")
    };
    matches!(ad, "127.0.0.1" | "localhost" | "::1")
}
