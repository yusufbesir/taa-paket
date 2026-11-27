//! taa-yazdir testleri. Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-yazdir 2026-11-27]
//! Yazıcı sahtedir (gerçek yazıcıya hiçbir şey gitmez); ağ testleri yalnız 127.0.0.1'de, boş portta çalışır.
use super::*;
use std::sync::atomic::AtomicBool;

const KOKEN: &str = "https://taa-web.ornek";

#[derive(Default)]
struct SahteYazici {
    isler: Mutex<Vec<(String, Vec<u8>)>>,
    bozuk: bool,
}

impl YaziciYolu for Arc<SahteYazici> {
    fn yaz(&self, hedef: &str, baytlar: &[u8]) -> std::io::Result<()> {
        if self.bozuk {
            return Err(std::io::Error::other("kablo çıkık"));
        }
        self.isler
            .lock()
            .unwrap()
            .push((hedef.to_string(), baytlar.to_vec()));
        Ok(())
    }
}

fn ayar() -> Ayar {
    Ayar::coz(
        r#"{"izinli_kokenler":["https://TAA-Web.ornek/"],"en_cok_bayt":4096,
            "yazicilar":{"escpos":"tcp://192.0.2.1:9100"}}"#,
    )
    .unwrap()
}

fn hizmet() -> (Hizmet, Arc<SahteYazici>) {
    let y = Arc::new(SahteYazici::default());
    (
        Hizmet::ozel(ayar(), Box::new(Arc::clone(&y)), Box::new(DenetimKapali)),
        y,
    )
}

fn ist(metot: &str, hedef: &str, koken: Option<&str>, tur: &str, govde: &[u8]) -> Istek {
    let (yol, sorgu) = hedef.split_once('?').unwrap_or((hedef, ""));
    let mut basliklar = vec![("host".to_string(), "127.0.0.1:8797".to_string())];
    if let Some(k) = koken {
        basliklar.push(("origin".into(), k.into()));
    }
    if !tur.is_empty() {
        basliklar.push(("content-type".into(), tur.into()));
    }
    Istek {
        metot: metot.into(),
        yol: yol.into(),
        sorgu: sorgu.into(),
        basliklar,
        govde: govde.to_vec(),
        yerel: true,
    }
}

fn json_govde(y: &Yanit) -> Value {
    serde_json::from_slice(&y.govde).unwrap()
}

fn baslik<'a>(y: &'a Yanit, ad: &str) -> Option<&'a str> {
    y.basliklar
        .iter()
        .find(|(a, _)| a == ad)
        .map(|(_, d)| d.as_str())
}

#[test]
fn ayar_cozulur_ve_koken_duzeltilir() {
    let a = ayar();
    assert_eq!(a.port, VARSAYILAN_PORT);
    assert_eq!(a.izinli_kokenler, vec![KOKEN.to_string()]);
    assert_eq!(a.en_cok_bayt, 4096);
    assert!(!a.cihaz_imza_zorunlu);
    assert!(a.koken_izinli("HTTPS://taa-web.ornek"));
    assert!(!a.koken_izinli("https://taa-web.ornek.kotu.ornek"));
    assert!(!a.koken_izinli("http://taa-web.ornek"));
    assert!(Ayar::coz(&ornek_ayar()).is_ok());
}

#[test]
fn ayar_tehlikeli_degerleri_reddeder() {
    for kotu in [
        r#"{"izinli_kokenler":["*"]}"#,
        r#"{"izinli_kokenler":["null"]}"#,
        r#"{"izinli_kokenler":["https://*.ornek"]}"#,
        r#"{"izinli_kokenler":["https://a.ornek/yol"]}"#,
        r#"{"izinli_kokenler":["a.ornek"]}"#,
        r#"{}"#,
        r#"{"izinli_kokenler":[],"port":80}"#,
        r#"{"izinli_kokenler":[],"en_cok_bayt":0}"#,
        r#"{"izinli_kokenler":[],"en_cok_bayt":999999999}"#,
        r#"{"izinli_kokenler":[],"yazicilar":{"zebra":"x"}}"#,
        r#"{"izinli_kokenler":[],"yazicilar":{"sistem":"x"}}"#,
        r#"{"izinli_kokenler":[],"yazicilar":{"escpos":""}}"#,
        r#"{"izinli_kokenler":[],"cihaz_imza":"belki"}"#,
        r#"{"izinli_kokenler":[],"adres":"0.0.0.0"}"#,
        "[]",
        "bozuk",
    ] {
        assert!(Ayar::coz(kotu).is_err(), "kabul edilmemeliydi: {kotu}");
    }
}

#[test]
fn durum_hedefi_gostermez() {
    let (h, _) = hizmet();
    let y = h.isle(&ist("GET", "/durum", Some(KOKEN), "", b""));
    assert_eq!(y.kod, 200);
    let v = json_govde(&y);
    assert_eq!(v["hizmet"], "taa-yazdir");
    assert_eq!(v["suruculer"]["escpos"], "ayarli");
    assert_eq!(v["suruculer"]["argox-pplb"], "bagli-degil");
    assert_eq!(v["suruculer"]["sistem"], "bagli-degil");
    assert_eq!(v["cihaz_imza"], "kapali");
    assert!(!String::from_utf8_lossy(&y.govde).contains("192.0.2.1"));
    assert_eq!(baslik(&y, "Access-Control-Allow-Origin"), Some(KOKEN));
    // Kökensiz (yerel tanı) durum sorgusu serbest, CORS başlığı yok.
    let y = h.isle(&ist("GET", "/durum", None, "", b""));
    assert_eq!(y.kod, 200);
    assert_eq!(baslik(&y, "Access-Control-Allow-Origin"), None);
}

#[test]
fn izinsiz_koken_ve_kokensiz_yazdirma_reddedilir() {
    let (h, yazici) = hizmet();
    let g = br#"{"surucu":"escpos","veri":"x"}"#;
    for k in [Some("https://kotu.ornek"), Some("null"), None] {
        let y = h.isle(&ist("POST", "/yazdir", k, "application/json", g));
        assert_eq!(y.kod, 403, "{k:?}");
        assert_eq!(baslik(&y, "Access-Control-Allow-Origin"), None);
    }
    let y = h.isle(&ist("GET", "/durum", Some("https://kotu.ornek"), "", b""));
    assert_eq!(y.kod, 403);
    let y = h.isle(&ist(
        "OPTIONS",
        "/yazdir",
        Some("https://kotu.ornek"),
        "",
        b"",
    ));
    assert_eq!(y.kod, 403);
    assert!(yazici.isler.lock().unwrap().is_empty());
}

#[test]
fn disaridan_ve_yabanci_host_reddedilir() {
    let (h, yazici) = hizmet();
    let g = br#"{"surucu":"escpos","veri":"x"}"#;
    let mut i = ist("POST", "/yazdir", Some(KOKEN), "application/json", g);
    i.yerel = false;
    assert_eq!(h.isle(&i).kod, 403);
    for host in [
        "kotu.ornek",
        "kotu.ornek:8797",
        "192.168.1.5:8797",
        "127.0.0.1.kotu.ornek",
    ] {
        let mut i = ist("POST", "/yazdir", Some(KOKEN), "application/json", g);
        i.basliklar[0].1 = host.into();
        assert_eq!(h.isle(&i).kod, 403, "{host}");
    }
    let mut i = ist("GET", "/durum", None, "", b"");
    i.basliklar.clear();
    assert_eq!(h.isle(&i).kod, 403, "Host yoksa ret");
    let mut i = ist("GET", "/durum", None, "", b"");
    i.basliklar[0].1 = "localhost:8797".into();
    assert_eq!(h.isle(&i).kod, 200);
    assert!(yazici.isler.lock().unwrap().is_empty());
}

#[test]
fn on_ucus_istegi_yanitlanir() {
    let (h, _) = hizmet();
    let y = h.isle(&ist("OPTIONS", "/yazdir", Some(KOKEN), "", b""));
    assert_eq!(y.kod, 204);
    assert_eq!(baslik(&y, "Access-Control-Allow-Origin"), Some(KOKEN));
    assert_eq!(
        baslik(&y, "Access-Control-Allow-Private-Network"),
        Some("true")
    );
    assert!(baslik(&y, "Access-Control-Allow-Headers")
        .unwrap()
        .contains("Content-Type"));
    assert_eq!(baslik(&y, "Vary"), Some("Origin"));
}

#[test]
fn json_utf8_ve_bayt_kodlamasi() {
    let (h, yazici) = hizmet();
    let y = h.isle(&ist(
        "POST",
        "/yazdir",
        Some(KOKEN),
        "application/json; charset=utf-8",
        r#"{"surucu":"escpos","veri":"Şube\n"}"#.as_bytes(),
    ));
    assert_eq!(y.kod, 200, "{}", String::from_utf8_lossy(&y.govde));
    assert_eq!(json_govde(&y)["bayt"], "Şube\n".len());
    // ESC @ (1B 40) + 0xFF: 'bayt' kodlamasında her karakter tek bayt.
    let y = h.isle(&ist(
        "POST",
        "/yazdir",
        Some(KOKEN),
        "application/json",
        br#"{"surucu":"escpos","kodlama":"bayt","veri":"\u001b@\u00ff\u0000"}"#,
    ));
    assert_eq!(y.kod, 200);
    let isler = yazici.isler.lock().unwrap();
    assert_eq!(
        isler[0],
        (
            "tcp://192.0.2.1:9100".to_string(),
            "Şube\n".as_bytes().to_vec()
        )
    );
    assert_eq!(isler[1].1, vec![0x1b, 0x40, 0xff, 0x00]);
    drop(isler);
    let v = json_govde(&h.isle(&ist("GET", "/durum", None, "", b"")));
    assert_eq!(v["yazdirilan"], 2);
}

#[test]
fn ham_govde_surucu_sorgudan() {
    let (h, yazici) = hizmet();
    let ham = [0x1bu8, 0x40, 0x80, 0xfe, 0x0a];
    let y = h.isle(&ist(
        "POST",
        "/yazdir?surucu=escpos",
        Some(KOKEN),
        "application/octet-stream",
        &ham,
    ));
    assert_eq!(y.kod, 200);
    assert_eq!(yazici.isler.lock().unwrap()[0].1, ham);
    let y = h.isle(&ist(
        "POST",
        "/yazdir",
        Some(KOKEN),
        "application/octet-stream",
        &ham,
    ));
    assert_eq!(y.kod, 400, "sürücü yoksa ret");
}

#[test]
fn bozuk_isler_400() {
    let (h, yazici) = hizmet();
    for (tur, g) in [
        ("application/json", r#"{"surucu":"zebra","veri":"x"}"#),
        ("application/json", r#"{"surucu":"escpos"}"#),
        ("application/json", r#"{"veri":"x"}"#),
        ("application/json", r#"{"surucu":"escpos","veri":""}"#),
        (
            "application/json",
            r#"{"surucu":"escpos","veri":"x","kodlama":"base64"}"#,
        ),
        (
            "application/json",
            r#"{"surucu":"escpos","veri":"Ş","kodlama":"bayt"}"#,
        ),
        ("application/json", "bozuk"),
        ("application/xml", "<x/>"),
        ("", "x"),
    ] {
        let y = h.isle(&ist("POST", "/yazdir", Some(KOKEN), tur, g.as_bytes()));
        assert_eq!(y.kod, 400, "{tur} {g}");
        assert_eq!(json_govde(&y)["tamam"], false);
    }
    assert!(yazici.isler.lock().unwrap().is_empty());
}

#[test]
fn bagli_olmayan_surucu_501() {
    let (h, yazici) = hizmet();
    for s in ["sistem", "argox-pplb"] {
        let g = format!(r#"{{"surucu":"{s}","veri":"N\nP1\n"}}"#);
        let y = h.isle(&ist(
            "POST",
            "/yazdir",
            Some(KOKEN),
            "application/json",
            g.as_bytes(),
        ));
        assert_eq!(y.kod, 501, "{s}");
        assert_eq!(json_govde(&y)["hata"], "henüz bağlı değil");
    }
    assert!(yazici.isler.lock().unwrap().is_empty());
}

#[test]
fn yazici_hatasi_502_ve_sayilir() {
    let y = Arc::new(SahteYazici {
        bozuk: true,
        ..Default::default()
    });
    let h = Hizmet::ozel(ayar(), Box::new(y), Box::new(DenetimKapali));
    let r = h.isle(&ist(
        "POST",
        "/yazdir",
        Some(KOKEN),
        "application/json",
        br#"{"surucu":"escpos","veri":"x"}"#,
    ));
    assert_eq!(r.kod, 502);
    assert!(json_govde(&r)["hata"]
        .as_str()
        .unwrap()
        .contains("kablo çıkık"));
    assert_eq!(
        json_govde(&h.isle(&ist("GET", "/durum", None, "", b"")))["hatali"],
        1
    );
}

#[test]
fn yol_ve_yontem() {
    let (h, _) = hizmet();
    assert_eq!(h.isle(&ist("GET", "/", None, "", b"")).kod, 404);
    assert_eq!(
        h.isle(&ist("GET", "/yazdir", Some(KOKEN), "", b"")).kod,
        405
    );
    assert_eq!(
        h.isle(&ist("POST", "/durum", Some(KOKEN), "", b"")).kod,
        405
    );
}

#[test]
fn cihaz_imzasi_kancasi_taa_cihaz_ile_dogrular() {
    let kok = taa_dosya::gecici_klasor().unwrap();
    let anahtar = taa_cihaz::CihazAnahtari::uret(&kok, Some("deneme-pc")).unwrap();
    let denetim = CihazImzasi::kokten(&kok).unwrap().saatle(|| 1_800_000_000);
    let yazici = Arc::new(SahteYazici::default());
    let h = Hizmet::ozel(ayar(), Box::new(Arc::clone(&yazici)), Box::new(denetim));
    let g = br#"{"surucu":"escpos","veri":"x"}"#;
    let imzali = |zaman: i64, imzalanan: &[u8]| {
        let mut i = ist("POST", "/yazdir", Some(KOKEN), "application/json", g);
        i.basliklar.push(("x-taa-cihaz".into(), anahtar.kimlik()));
        i.basliklar.push(("x-taa-zaman".into(), zaman.to_string()));
        i.basliklar.push((
            "x-taa-imza".into(),
            anahtar.imzala("POST", "/yazdir", zaman, imzalanan),
        ));
        i
    };
    // İmzasız: 401.
    assert_eq!(
        h.isle(&ist("POST", "/yazdir", Some(KOKEN), "application/json", g))
            .kod,
        401
    );
    // Başka gövdeye atılmış imza: 401.
    assert_eq!(h.isle(&imzali(1_800_000_000, b"baska")).kod, 401);
    // Zaman payı dışında: 401.
    assert_eq!(h.isle(&imzali(1_800_000_000 - 3600, g)).kod, 401);
    assert!(yazici.isler.lock().unwrap().is_empty());
    // Doğru imza: yazdırılır.
    assert_eq!(h.isle(&imzali(1_800_000_010, g)).kod, 200);
    assert_eq!(yazici.isler.lock().unwrap().len(), 1);
    // İmza zorunluyken de /durum imzasız okunur ve kipi gösterir.
    assert_eq!(
        json_govde(&h.isle(&ist("GET", "/durum", None, "", b"")))["cihaz_imza"],
        "zorunlu"
    );
}

fn oku(ham: &[u8], en_cok: usize) -> Result<Istek, OkumaHata> {
    istek_oku(&mut &ham[..], en_cok, true)
}

#[test]
fn istek_cozumleme() {
    let i = oku(b"POST /yazdir?surucu=escpos HTTP/1.1\r\nHost: 127.0.0.1:8797\r\nORIGIN:  https://a.ornek \r\nContent-Length: 3\r\n\r\nabcFAZLA", 100).unwrap();
    assert_eq!(
        (i.metot.as_str(), i.yol.as_str(), i.sorgu.as_str()),
        ("POST", "/yazdir", "surucu=escpos")
    );
    assert_eq!(i.baslik("origin"), Some("https://a.ornek"));
    assert_eq!(i.govde, b"abc");
    let i = oku(b"GET /durum HTTP/1.1\r\nHost: localhost\r\n\r\n", 100).unwrap();
    assert!(i.govde.is_empty());
}

#[test]
fn istek_sinirlari_ve_bozuk_girdi() {
    let kod = |ham: &[u8]| oku(ham, 10).unwrap_err().0;
    assert_eq!(
        kod(b"POST /yazdir HTTP/1.1\r\nContent-Length: 11\r\n\r\n"),
        413
    );
    assert_eq!(
        kod(b"POST /yazdir HTTP/1.1\r\nContent-Length: 99999999999999999999999\r\n\r\n"),
        413
    );
    assert_eq!(kod(b"POST /yazdir HTTP/1.1\r\nHost: x\r\n\r\n"), 411);
    assert_eq!(
        kod(b"POST /yazdir HTTP/1.1\r\nContent-Length: 5\r\n\r\nab"),
        400
    );
    assert_eq!(
        kod(b"POST /yazdir HTTP/1.1\r\nContent-Length: 2\r\nContent-Length: 3\r\n\r\nabc"),
        400
    );
    assert_eq!(
        kod(b"POST /yazdir HTTP/1.1\r\nContent-Length: -1\r\n\r\n"),
        400
    );
    assert_eq!(
        kod(b"POST /yazdir HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n"),
        501
    );
    assert_eq!(
        kod(b"POST /yazdir HTTP/1.1\r\nHost : x\r\nContent-Length: 0\r\n\r\n"),
        400
    );
    assert_eq!(kod(b"POST /yazdir HTTP/1.1\r\nbozuk\r\n\r\n"), 400);
    assert_eq!(kod(b"GET durum HTTP/1.1\r\n\r\n"), 400);
    assert_eq!(kod(b"GET /durum HTTP/2\r\n\r\n"), 400);
    assert_eq!(kod(b"GET /durum\r\n\r\n"), 400);
    assert_eq!(kod(b"GET /durum HTTP/1.1\r\nHost: x"), 400);
    let mut buyuk = b"GET /durum HTTP/1.1\r\nX: ".to_vec();
    buyuk.extend(std::iter::repeat_n(b'a', BASLIK_EN_COK + 10));
    buyuk.extend(b"\r\n\r\n");
    assert_eq!(kod(&buyuk), 431);
}

#[test]
fn yanit_basliga_satir_sonu_sizdirmaz() {
    let y =
        Yanit::hata(400, "x").baslikla("Access-Control-Allow-Origin", "https://a\r\nSet-Cookie: x");
    let mut c = Vec::new();
    yanit_yaz(&mut c, &y).unwrap();
    let m = String::from_utf8(c).unwrap();
    assert!(!m.contains("Set-Cookie"));
    assert!(m.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    assert!(m.contains("Connection: close\r\n"));
}

/// Gerçek soket: 127.0.0.1'de boş portta dinler, istek atar.
fn ag_istegi(port: u16, ham: &[u8]) -> String {
    let mut s = TcpStream::connect((Ipv4Addr::LOCALHOST, port)).unwrap();
    s.set_read_timeout(Some(Duration::from_secs(10))).unwrap();
    s.write_all(ham).unwrap();
    let mut c = Vec::new();
    let _ = s.read_to_end(&mut c);
    String::from_utf8_lossy(&c).into_owned()
}

#[test]
fn ag_uzerinden_yalniz_geri_dongu() {
    let dinleyici = dinle(0).unwrap();
    let adres = dinleyici.local_addr().unwrap();
    assert!(
        adres.ip().is_loopback(),
        "dinleme adresi geri döngü olmalı: {adres}"
    );
    assert_eq!(adres.ip().to_string(), "127.0.0.1");
    let port = adres.port();
    let yazici = Arc::new(SahteYazici::default());
    let h = Arc::new(Hizmet::ozel(
        ayar(),
        Box::new(Arc::clone(&yazici)),
        Box::new(DenetimKapali),
    ));
    let dur = Arc::new(AtomicBool::new(false));
    let (h2, dur2) = (Arc::clone(&h), Arc::clone(&dur));
    let is = std::thread::spawn(move || calistir(h2, dinleyici, dur2));

    let y = ag_istegi(
        port,
        format!("GET /durum HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n\r\n").as_bytes(),
    );
    assert!(y.starts_with("HTTP/1.1 200 OK\r\n"), "{y}");
    assert!(y.contains("\"hizmet\":\"taa-yazdir\""));

    let g = r#"{"surucu":"escpos","veri":"merhaba\n"}"#;
    let y = ag_istegi(
        port,
        format!("POST /yazdir HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: {KOKEN}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{g}", g.len()).as_bytes(),
    );
    assert!(y.starts_with("HTTP/1.1 200 OK\r\n"), "{y}");
    assert!(y.contains(&format!("Access-Control-Allow-Origin: {KOKEN}\r\n")));
    assert_eq!(yazici.isler.lock().unwrap()[0].1, b"merhaba\n");

    // Sınırı aşan gövde (ayar: 4096) okunmadan 413.
    let buyuk = vec![b'a'; 5000];
    let mut ham = format!("POST /yazdir?surucu=escpos HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: {KOKEN}\r\nContent-Type: application/octet-stream\r\nContent-Length: {}\r\n\r\n", buyuk.len()).into_bytes();
    ham.extend(&buyuk);
    let y = ag_istegi(port, &ham);
    assert!(y.starts_with("HTTP/1.1 413 "), "{y}");
    assert_eq!(yazici.isler.lock().unwrap().len(), 1);

    // İzinsiz köken ağ üzerinden de 403.
    let y = ag_istegi(
        port,
        format!("POST /yazdir HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: https://kotu.ornek\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{g}", g.len()).as_bytes(),
    );
    assert!(y.starts_with("HTTP/1.1 403 "), "{y}");
    assert!(!y.contains("Access-Control-Allow-Origin"));

    dur.store(true, Ordering::SeqCst);
    let _ = TcpStream::connect((Ipv4Addr::LOCALHOST, port));
    is.join().unwrap();
}

/// Gerçek gönderme yolu: yerel bir TCP "yazıcı" (127.0.0.1) ve bir dosya hedefiyle denenir.
#[test]
fn gercek_yol_tcp_ve_dosya() {
    let sahte = TcpListener::bind((Ipv4Addr::LOCALHOST, 0)).unwrap();
    let port = sahte.local_addr().unwrap().port();
    let is = std::thread::spawn(move || {
        let (mut s, _) = sahte.accept().unwrap();
        let mut c = Vec::new();
        s.read_to_end(&mut c).unwrap();
        c
    });
    GercekYazici
        .yaz(&format!("tcp://127.0.0.1:{port}"), &[0x1b, 0x40, 0xff])
        .unwrap();
    assert_eq!(is.join().unwrap(), vec![0x1b, 0x40, 0xff]);

    let d = taa_dosya::gecici_klasor().unwrap();
    let hedef = d.join("yazici-yerine-dosya.bin");
    // Hedef yoksa oluşturulmaz (yazıcı yolu var olmalı): hata döner.
    assert!(GercekYazici.yaz(&hedef.to_string_lossy(), b"x").is_err());
    std::fs::write(&hedef, b"").unwrap();
    GercekYazici
        .yaz(&hedef.to_string_lossy(), b"N\nP1\n")
        .unwrap();
    assert_eq!(std::fs::read(&hedef).unwrap(), b"N\nP1\n");
}

/// yazdir.sun Kural 12: tarayıcıdan gelen ESC/POS içeriği çekmece açamaz.
#[test]
fn cekmece_komutu_reddedilir() {
    let (h, yazici) = hizmet();
    for ham in [
        &[0x1b, 0x40, 0x1b, 0x70, 0x00, 0x19, 0xfa][..],
        &[b'a', 0x10, 0x14, 0x01, 0x00, 0x05][..],
    ] {
        let y = h.isle(&ist(
            "POST",
            "/yazdir?surucu=escpos",
            Some(KOKEN),
            "application/octet-stream",
            ham,
        ));
        assert_eq!(y.kod, 400);
        assert_eq!(json_govde(&y)["hata"], "cekmece_red");
    }
    assert!(yazici.isler.lock().unwrap().is_empty());
    // ESC @ ve DLE EOT (durum sorgusu) çekmece komutu değildir.
    assert!(!cekmece_komutu_var(&[
        0x1b, 0x40, 0x10, 0x04, 0x01, 0x10, 0x14, 0x02
    ]));
    let y = h.isle(&ist(
        "POST",
        "/yazdir?surucu=escpos",
        Some(KOKEN),
        "application/octet-stream",
        &[0x1b, 0x40, b'x'],
    ));
    assert_eq!(y.kod, 200);
}
