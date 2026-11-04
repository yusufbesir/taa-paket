//! taa-fis-sunucu: TaaFiş yerel sunucusu. Kullanım:
//!   taa-fis-sunucu [--veri KLASÖR] [--port 8787] [--yazici YOL] [--adres http://192.168.1.20:8787]
//! Yönetim (karekod, telefon listesi): bu bilgisayarda http://localhost:PORT/yonetim
use std::io::Read;
use std::path::PathBuf;
use taa_fis_sunucu::{Istek, Uygulama, GOVDE_SINIRI};

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
    let uyg = Uygulama::yeni(veri, arg("--yazici").map(PathBuf::from), taban.clone());
    let sunucu = match tiny_http::Server::http(format!("0.0.0.0:{port}")) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Sunucu açılamadı (port {port} dolu olabilir): {e}");
            std::process::exit(1);
        }
    };
    println!("TaaFiş sunucusu hazır. Telefon adresi: {taban}/telefon");
    println!("Eşleştirme için bu bilgisayarda açın: http://localhost:{port}/yonetim");
    for mut r in sunucu.incoming_requests() {
        let yerel = r
            .remote_addr()
            .map(|a| a.ip().is_loopback())
            .unwrap_or(false);
        let yetki = r
            .headers()
            .iter()
            .find(|h| h.field.equiv("Authorization"))
            .map(|h| h.value.to_string());
        let mut govde = Vec::new();
        let _ = r
            .as_reader()
            .take(GOVDE_SINIRI as u64 + 1)
            .read_to_end(&mut govde);
        let y = uyg.isle(&Istek {
            metot: r.method().as_str().to_string(),
            yol: r.url().to_string(),
            yetki,
            govde,
            yerel,
        });
        let mut cevap = tiny_http::Response::from_data(y.govde).with_status_code(y.kod);
        for (a, d) in [
            ("Content-Type", y.tur),
            ("X-Content-Type-Options", "nosniff"),
            ("Cache-Control", "no-store"),
        ] {
            if let Ok(h) = tiny_http::Header::from_bytes(a.as_bytes(), d.as_bytes()) {
                cevap.add_header(h);
            }
        }
        let _ = r.respond(cevap);
    }
}
