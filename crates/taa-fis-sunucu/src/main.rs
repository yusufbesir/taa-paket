//! taa-fis-sunucu: TaaFiş yerel sunucusu. Kullanım:
//!   taa-fis-sunucu [--veri KLASÖR] [--port 8787] [--yazici YOL|tcp://IP[:9100]] [--adres http://192.168.1.20:8787]
//!                  [--aydinlatma-url https://...] [--kok TAA_VERI_KOKU] [--sunucu CATI_ADRESI] [--sertifika CA.pem]
//! Masaüstü program: bu bilgisayarda http://127.0.0.1:PORT/masaustu (ya da `taa-terzi-takip` başlatıcısı).
//! Yönetici PIN'i TAA Çatı'da doğrulanır: `--kok` (varsayılan TAA veri kökü) altındaki oturum, cihaz anahtarı ve
//! yol listesi kullanılır; `--sunucu`/`--sertifika` TAA Çatı'daki anlamıyla aynıdır.
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
    let port: u16 = arg("--port").and_then(|p| p.parse().ok()).unwrap_or(8787);
    let veri = PathBuf::from(arg("--veri").unwrap_or_else(|| "taa-fis-veri".into()));
    let taban = arg("--adres").unwrap_or_else(|| {
        format!(
            "http://{}:{port}",
            yerel_ip().unwrap_or_else(|| "localhost".into())
        )
    });
    let mut uyg = Uygulama::yeni(veri, arg("--yazici").map(PathBuf::from), taban.clone());
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
