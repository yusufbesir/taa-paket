//! taa-giris.rs v1 — kaynak: yusufbesir/taa-paket/crates/taa-fis-sunucu/src/ortak_giris.rs; Rust programları
//! kopyalar, elle düzenlemez (yalnız std + serde_json; programa özgü hiçbir şey içermez).
//!
//! TAA ortak giriş (KM1), yerel HTTP arayüzlü Rust programları için: `POST /api/giris/<islem>` uçlarının gövdesi.
//! Electron programlarındaki `araclar/js/taa-giris.js` köprüsünün karşılığı: aynı `taa-kimlik` aracını çalıştırır
//! (cihaz imzası, yedekli yollar, sunucu imzası doğrulaması, Windows Kimlik Bilgisi Yöneticisi orada), aynı durum
//! adlarını ve iletileri döndürür; ekran aynı `taa-giris-ekrani.js` dosyasıdır (sayfaya gömülür).
//! Program yalnız bu bilgisayardan ve kendi özel başlığıyla gelen istekleri buraya geçirmelidir.
//! Şifre yalnız aracın standart girdisine yazılır; hiçbir yere kaydedilmez, günlüğe ve hata iletisine girmez.
//! Yazan: [Claude#ortak-giris 2026-11-25].

use serde_json::json;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Araç bu sürede bitmezse durdurulur ve "sunucuya ulaşılamadı" sayılır (tek iş parçacıklı sunucular
/// için Electron köprüsündeki 30 sn'den kısa).
const SURE: Duration = Duration::from_secs(20);
const CIKTI_EN_COK: u64 = 64 * 1024;

/// Aracın yolunu bulan işlev; testler sahte araç verir.
pub type KimlikBulucu = fn() -> Option<PathBuf>;

fn ortam(ad: &str) -> Option<String> {
    std::env::var(ad).ok().filter(|s| !s.trim().is_empty())
}

/// TAA kurulum kökü. Yayın derlemesinde ortam değişkeni yok sayılır (Electron'daki `paketli` kuralı):
/// yalnız sabit kök (Windows'ta D:\Programlar\TAA; yazma izni yalnız yöneticide).
fn taa_kok() -> PathBuf {
    if cfg!(debug_assertions) {
        if let Some(k) = ortam("TAA_KOK") {
            return PathBuf::from(k);
        }
    }
    if cfg!(windows) {
        return PathBuf::from(r"D:\Programlar\TAA");
    }
    let veri = ortam("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(ortam("HOME").unwrap_or_else(|| ".".into()))
                .join(".local")
                .join("share")
        });
    veri.join("TAA")
}

/// Sürüm adı rakamla başlar, yalnız [0-9a-z.+-], en çok 32 karakter, `..` içermez (yol kaçışı olmaz).
fn surum_gecerli(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.as_bytes()[0].is_ascii_digit()
        && s.bytes()
            .all(|b| b.is_ascii_digit() || b.is_ascii_lowercase() || b"+.-".contains(&b))
        && !s.contains("..")
}

/// `taa-kimlik` aracı: (yalnız geliştirme derlemesinde TAA_KIMLIK) → `<kök>/kayit.json` seçili sürüm →
/// `<kök>/depo/taa-kimlik/<sürüm>/bin` → `<kök>/bin`.
pub fn kimlik_yolu() -> Option<PathBuf> {
    let exe = if cfg!(windows) {
        "taa-kimlik.exe"
    } else {
        "taa-kimlik"
    };
    if cfg!(debug_assertions) {
        if let Some(p) = ortam("TAA_KIMLIK") {
            let p = PathBuf::from(p);
            return p.is_file().then_some(p);
        }
    }
    let kok = taa_kok();
    let secili = std::fs::read_to_string(kok.join("kayit.json"))
        .ok()
        .and_then(|m| serde_json::from_str::<serde_json::Value>(&m).ok())
        .and_then(|v| {
            v["paketler"]["taa-kimlik"]["secili"]
                .as_str()
                .map(str::to_string)
        });
    if let Some(s) = secili.filter(|s| surum_gecerli(s)) {
        let y = kok
            .join("depo")
            .join("taa-kimlik")
            .join(s)
            .join("bin")
            .join(exe);
        if y.is_file() {
            return Some(y);
        }
    }
    let yanimda = kok.join("bin").join(exe);
    yanimda.is_file().then_some(yanimda)
}

struct Sonuc {
    kod: i32,
    cikti: Option<serde_json::Value>,
    hata: Option<serde_json::Value>,
}

/// Çıktıdaki son JSON satırı.
fn json_satir(metin: &[u8]) -> Option<serde_json::Value> {
    String::from_utf8_lossy(metin)
        .lines()
        .rev()
        .map(str::trim)
        .filter(|s| s.starts_with('{'))
        .find_map(|s| serde_json::from_str(s).ok())
}

/// Aracı çalıştırır; asla panik etmez. `girdi` (şifre) yalnız stdin'e yazılır. Araç yoksa kod -1.
fn calistir(bul: KimlikBulucu, args: &[&str], girdi: Option<&str>) -> Sonuc {
    let yok = Sonuc {
        kod: -1,
        cikti: None,
        hata: None,
    };
    let Some(exe) = bul() else { return yok };
    let mut k = Command::new(exe);
    k.args(args)
        .arg("--json")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        k.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    let Ok(mut p) = k.spawn() else { return yok };
    if let Some(mut s) = p.stdin.take() {
        let g: String = girdi
            .unwrap_or("")
            .chars()
            .filter(|c| *c != '\r' && *c != '\n')
            .collect();
        let _ = s.write_all(g.as_bytes());
        let _ = s.write_all(b"\n");
    }
    let oku = |r: Option<Box<dyn Read + Send>>| {
        std::thread::spawn(move || {
            let mut b = Vec::new();
            if let Some(r) = r {
                let _ = r.take(CIKTI_EN_COK).read_to_end(&mut b);
            }
            b
        })
    };
    let o = oku(p.stdout.take().map(|r| Box::new(r) as Box<dyn Read + Send>));
    let e = oku(p.stderr.take().map(|r| Box::new(r) as Box<dyn Read + Send>));
    let bas = Instant::now();
    let kod = loop {
        match p.try_wait() {
            Ok(Some(d)) => break d.code().unwrap_or(1),
            Ok(None) if bas.elapsed() < SURE => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = p.kill();
                let _ = p.wait();
                break 3;
            }
        }
    };
    Sonuc {
        kod,
        cikti: o.join().ok().and_then(|b| json_satir(&b)),
        hata: e.join().ok().and_then(|b| json_satir(&b)),
    }
}

fn ileti(durum: &str) -> &'static str {
    match durum {
        "arac_yok" => "Giriş aracı (taa-kimlik) bu bilgisayarda kurulu değil. TAA Kur ile kurun ya da yöneticinize haber verin.",
        "baglanti" => "Sunucuya ulaşılamadı. İnternet ya da ağ bağlantısını denetleyip yeniden deneyin.",
        "hatali" => "Kullanıcı adı ya da şifre hatalı.",
        "kilitli" => "Hesabınız kilitlendi. \"Yöneticiye başvur\" ile yöneticiden kilidi açmasını isteyin.",
        "sinir" => "Bu bilgisayardan çok fazla hatalı deneme yapıldı. Bir saat sonra yeniden deneyin.",
        "mesgul" => "Bu bilgisayardan önceki istek sürüyor. Birazdan yeniden deneyin.",
        "onay_bekliyor" => "Hesabınız yönetici onayı bekliyor. Yönetici onaylayınca yeniden deneyin.",
        "cihaz" => "Bu bilgisayar sunucuya kayıtlı ya da onaylı değil. Yöneticiden bilgisayarı onaylamasını isteyin.",
        "program_kapali" => "Bu program hesabınıza ya da şubenize açık değil. Yöneticinize başvurun.",
        "sube_yetkisiz" => "Bu bilgisayarın şubesinde yetkiniz yok. Kendi şubenizin bilgisayarından girin ya da yöneticinize başvurun.",
        "sube_cihazi" => "Başvuru yalnız mağaza bilgisayarından yapılır.",
        "girdi" => "Kullanıcı adını ve şifreyi doğru yazın.",
        "oturum_yok" => "Oturum yok. Giriş yapın.",
        _ => "Giriş yapılamadı. Yeniden deneyin; sürerse yöneticinize haber verin.",
    }
}

/// Hata iletisinin içine gömülü sunucu gövdesi ({"hata": kod, "ileti": "..."}).
fn sunucu_govdesi(m: &str) -> Option<serde_json::Value> {
    let (i, j) = (m.find('{')?, m.rfind('}')?);
    (j > i)
        .then(|| serde_json::from_str::<serde_json::Value>(&m[i..=j]).ok())
        .flatten()
        .filter(|v| v.is_object())
}

fn durum_bul(r: &Sonuc) -> &'static str {
    if r.kod == -1 {
        return "arac_yok";
    }
    let bos = json!({});
    let h = r.hata.as_ref().unwrap_or(&bos);
    let http = h["http"].as_u64().unwrap_or(0);
    let m = h["hata"].as_str().unwrap_or("");
    let g = sunucu_govdesi(m).unwrap_or_else(|| json!({}));
    let kod = g["hata"].as_str().unwrap_or("");
    match http {
        423 => return "kilitli",
        429 if kod == "cihaz_mesgul"
            || m.contains("cihaz_mesgul")
            || m.contains("önceki istek") =>
        {
            return "mesgul"
        }
        429 => return "sinir",
        403 if kod == "hesap_onay_bekliyor" || m.contains("onayı bekliyor") => {
            return "onay_bekliyor"
        }
        403 if kod == "sube_yetkisiz" => return "sube_yetkisiz",
        403 if kod == "sube_cihazi_gerekli" => return "sube_cihazi",
        403 => return "cihaz",
        401 if kod.starts_with("cihaz") => return "cihaz",
        401 => return "hatali",
        _ => {}
    }
    match r.kod {
        3 => "baglanti",
        2 => "girdi",
        5 if m.contains("cihaz") => "cihaz",
        5 => "oturum_yok",
        _ => "bilinmiyor",
    }
}

fn red(durum: &str) -> serde_json::Value {
    json!({ "tamam": false, "durum": durum, "ileti": ileti(durum) })
}

fn liste(v: &serde_json::Value) -> serde_json::Value {
    if v.is_array() {
        v.clone()
    } else {
        json!([])
    }
}

/// Program listede değilse (liste yok ya da boşsa da) kapalı.
fn program_acik(oturum: &serde_json::Value, program: &str) -> bool {
    oturum["programlar"]
        .as_array()
        .is_some_and(|l| l.iter().any(|p| p.as_str() == Some(program)))
}

fn oturum_ozeti(o: &serde_json::Value, uyari: bool) -> serde_json::Value {
    let mut v = json!({
        "kullanici": o["kullanici"],
        "sube_id": o["sube_id"],
        "roller": liste(&o["roller"]),
        "izinler": liste(&o["izinler"]),
        "programlar": liste(&o["programlar"]),
    });
    if uyari {
        v["uyari_sayisi"] = json!(o["uyari_sayisi"].as_i64().unwrap_or(0));
    }
    v
}

/// taa-kimlik `kullanici_gecerli` ile aynı kural: boş değil, en çok 64, boşluk/denetim yok, "-" ile başlamaz.
fn kullanici_gecerli(k: &str) -> bool {
    !k.is_empty()
        && !k.starts_with('-')
        && k.chars().count() <= 64
        && !k.chars().any(|c| c.is_whitespace() || c.is_control())
}

/// Süren oturum; bu programa kapalıysa ya da yoksa null.
fn durum(bul: KimlikBulucu, program: &str) -> serde_json::Value {
    let r = calistir(bul, &["durum"], None);
    match r.cikti {
        Some(o) if r.kod == 0 => {
            let v = oturum_ozeti(&o, false);
            if program_acik(&v, program) {
                v
            } else {
                serde_json::Value::Null
            }
        }
        _ => serde_json::Value::Null,
    }
}

/// `POST /api/giris/<islem>` gövdesi (islem: baslangic, giris, basvur, durum, cikis). Yanıt her zaman 200 ile
/// gönderilecek JSON'dur; bilinmeyen işlemde None (404). `program`: sunucudaki program yetkisi adı.
pub fn yanitla(
    bul: KimlikBulucu,
    program: &str,
    islem: &str,
    govde: &[u8],
) -> Option<serde_json::Value> {
    {
        let v: serde_json::Value = serde_json::from_slice(govde).unwrap_or_else(|_| json!({}));
        let yaz = |k: &str| v[k].as_str().unwrap_or("").to_string();
        let sonuc = match islem {
            "baslangic" => match bul() {
                None => json!({ "arac": false, "oturum": null }),
                Some(_) => json!({ "arac": true, "oturum": durum(bul, program) }),
            },
            "durum" => durum(bul, program),
            "giris" => {
                let k = yaz("kullanici").trim().to_string();
                let parola = yaz("parola");
                if !kullanici_gecerli(&k) || parola.is_empty() || parola.contains(['\r', '\n']) {
                    red("girdi")
                } else {
                    let r = calistir(bul, &["giris", "--kullanici", &k], Some(&parola));
                    match &r.cikti {
                        Some(o) if r.kod == 0 => {
                            let oturum = oturum_ozeti(o, true);
                            // Çıkış yapılmaz: ortak oturum başka programlarda sürer, yalnız bu program reddedilir.
                            if program_acik(&oturum, program) {
                                json!({ "tamam": true, "oturum": oturum })
                            } else {
                                red("program_kapali")
                            }
                        }
                        _ => red(durum_bul(&r)),
                    }
                }
            }
            "basvur" => {
                let k = yaz("kullanici").trim().to_string();
                let yeni = yaz("yeniParola");
                if !kullanici_gecerli(&k) || yeni.contains(['\r', '\n']) {
                    red("girdi")
                } else {
                    let r = calistir(bul, &["basvur", "--kullanici", &k], Some(&yeni));
                    let sayi = r
                        .cikti
                        .as_ref()
                        .and_then(|c| c["dogrulama_sayisi"].as_str())
                        .filter(|s| s.len() == 2 && s.bytes().all(|b| b.is_ascii_digit()))
                        .map(str::to_string);
                    match sayi {
                        Some(s) if r.kod == 0 => json!({
                            "tamam": true, "dogrulama_sayisi": s,
                            "tur": if yeni.is_empty() { "kilit" } else { "sifre" },
                            "ileti": "Başvurunuz yöneticiye iletildi. Yöneticiye bu iki haneli sayıyı söyleyin; onaylanınca giriş yapabilirsiniz.",
                        }),
                        _ => {
                            // Sunucunun şifre kuralı iletisi kişiye aynen gösterilir; şifrenin kendisi iletide yoktur.
                            let h = r.hata.clone().unwrap_or_default();
                            let g = (h["http"].as_u64() == Some(400))
                                .then(|| sunucu_govdesi(h["hata"].as_str().unwrap_or("")))
                                .flatten();
                            match g {
                                Some(g) if g["hata"] == "parola" && g["ileti"].is_string() => {
                                    json!({ "tamam": false, "durum": "parola", "ileti": g["ileti"] })
                                }
                                _ => red(durum_bul(&r)),
                            }
                        }
                    }
                }
            }
            "cikis" => json!({ "tamam": calistir(bul, &["cikis"], None).kod == 0 }),
            _ => return None,
        };
        Some(sonuc)
    }
}

#[cfg(test)]
mod testler {
    use super::*;

    fn sonuc(kod: i32, hata: serde_json::Value) -> Sonuc {
        Sonuc {
            kod,
            cikti: None,
            hata: Some(hata),
        }
    }

    #[test]
    fn surum_adi_yol_kacisina_izin_vermez() {
        assert!(surum_gecerli("1.4.0") && surum_gecerli("2.0.0-rc.1+b"));
        for k in [
            "",
            "..",
            "1/../x",
            "a1",
            "1\\x",
            "1..2",
            "1.0 ",
            &"1".repeat(33),
        ] {
            assert!(!surum_gecerli(k), "{k:?}");
        }
    }

    #[test]
    fn durumlar_js_kopru_ile_ayni() {
        let c = |kod, h| durum_bul(&sonuc(kod, h));
        assert_eq!(
            durum_bul(&Sonuc {
                kod: -1,
                cikti: None,
                hata: None
            }),
            "arac_yok"
        );
        assert_eq!(c(4, json!({"http": 423})), "kilitli");
        assert_eq!(
            c(
                4,
                json!({"http": 429, "hata": "HTTP 429 {\"hata\":\"cihaz_mesgul\"}"})
            ),
            "mesgul"
        );
        assert_eq!(c(4, json!({"http": 429})), "sinir");
        assert_eq!(
            c(
                4,
                json!({"http": 403, "hata": "x {\"hata\":\"hesap_onay_bekliyor\"}"})
            ),
            "onay_bekliyor"
        );
        assert_eq!(
            c(
                4,
                json!({"http": 403, "hata": "x {\"hata\":\"sube_yetkisiz\"}"})
            ),
            "sube_yetkisiz"
        );
        assert_eq!(c(4, json!({"http": 403})), "cihaz");
        assert_eq!(
            c(
                4,
                json!({"http": 401, "hata": "x {\"hata\":\"cihaz_bilinmiyor\"}"})
            ),
            "cihaz"
        );
        assert_eq!(c(4, json!({"http": 401})), "hatali");
        assert_eq!(c(3, json!({})), "baglanti");
        assert_eq!(c(5, json!({"hata": "kayıtlı oturum yok"})), "oturum_yok");
        assert_eq!(c(1, json!({})), "bilinmiyor");
    }

    #[test]
    fn bos_ya_da_eksik_program_listesi_kapali() {
        assert!(!program_acik(&json!({}), "terzitakip"));
        assert!(!program_acik(&json!({"programlar": []}), "terzitakip"));
        assert!(!program_acik(
            &json!({"programlar": ["tutaa"]}),
            "terzitakip"
        ));
        assert!(program_acik(
            &json!({"programlar": ["tutaa", "terzitakip"]}),
            "terzitakip"
        ));
    }

    // ---- Uçtan uca: sahte taa-kimlik aracıyla (yalnız Unix; kabuk betiği) ----
    #[cfg(unix)]
    fn sahte_dizin() -> PathBuf {
        std::env::temp_dir().join(format!("taa-giris-sahte-kimlik-{}", std::process::id()))
    }
    #[cfg(unix)]
    fn sahte_bul() -> Option<PathBuf> {
        Some(sahte_dizin().join("taa-kimlik"))
    }
    fn arac_yok() -> Option<PathBuf> {
        None
    }

    fn post(bul: KimlikBulucu, islem: &str, govde: serde_json::Value) -> serde_json::Value {
        yanitla(bul, "terzitakip", islem, govde.to_string().as_bytes())
            .unwrap_or_else(|| json!("YOK"))
    }

    #[test]
    fn kullanici_kurali_taa_kimlik_ile_ayni() {
        assert!(kullanici_gecerli("ayse-k"));
        assert!(!kullanici_gecerli("-ayse"));
        assert!(!kullanici_gecerli("--sunucu"));
        assert!(!kullanici_gecerli("a b"));
        assert!(!kullanici_gecerli("a\u{7}"));
        assert!(!kullanici_gecerli(""));
        assert_eq!(
            post(arac_yok, "giris", json!({"kullanici": "-x", "parola": "p"}))["durum"],
            "girdi"
        );
    }

    #[test]
    fn arac_yoksa_eski_davranis() {
        assert_eq!(
            post(arac_yok, "baslangic", json!({})),
            json!({"arac": false, "oturum": null})
        );
        assert_eq!(
            post(arac_yok, "giris", json!({"kullanici": "a", "parola": "b"}))["durum"],
            "arac_yok"
        );
        assert_eq!(post(arac_yok, "bilinmeyen", json!({})), json!("YOK"));
    }

    #[cfg(unix)]
    #[test]
    fn giris_akisi_sahte_aracla() {
        use std::os::unix::fs::PermissionsExt;
        let dz = sahte_dizin();
        let _ = std::fs::remove_dir_all(&dz);
        std::fs::create_dir_all(&dz).unwrap();
        let betik = r#"#!/bin/sh
D="$(dirname "$0")"
echo "$@" >> "$D/argumanlar"
IFS= read -r GIRDI
case "$1" in
  durum)
    if [ -f "$D/oturum" ]; then cat "$D/oturum"; exit 0; fi
    echo '{"hata":"kayıtlı oturum yok. Öneri: taa-kimlik giris"}' >&2; exit 5 ;;
  giris)
    if [ "$GIRDI" != "Dogru-sifre1" ]; then echo '{"http":401,"hata":"giriş reddedildi (401): {\"hata\":\"giris\"}"}' >&2; exit 4; fi
    if [ "$3" = "kasiyer" ]; then P='["tutaa"]'; else P='["terzitakip","tutaa"]'; fi
    echo "{\"kullanici\":\"$3\",\"sube_id\":1,\"roller\":[\"personel\"],\"izinler\":[],\"programlar\":$P}" > "$D/oturum"
    echo "{\"kullanici\":\"$3\",\"sube_id\":1,\"roller\":[\"personel\"],\"izinler\":[],\"programlar\":$P,\"uyari_sayisi\":2}"; exit 0 ;;
  basvur) echo '{"durum":"iletildi","dogrulama_sayisi":"42"}'; exit 0 ;;
  cikis) rm -f "$D/oturum"; echo '{"durum":"cikis"}'; exit 0 ;;
esac
exit 1
"#;
        let yol = dz.join("taa-kimlik");
        std::fs::write(&yol, betik).unwrap();
        std::fs::set_permissions(&yol, std::fs::Permissions::from_mode(0o700)).unwrap();
        let u = sahte_bul;

        assert_eq!(
            post(u, "baslangic", json!({})),
            json!({"arac": true, "oturum": null})
        );
        let v = post(u, "giris", json!({"kullanici": "ayse", "parola": "yanlis"}));
        assert_eq!(
            (v["tamam"].clone(), v["durum"].clone()),
            (json!(false), json!("hatali"))
        );
        assert_eq!(
            post(u, "giris", json!({"kullanici": "ayse", "parola": "a\nb"}))["durum"],
            "girdi"
        );
        let v = post(
            u,
            "giris",
            json!({"kullanici": "ayse", "parola": "Dogru-sifre1"}),
        );
        assert_eq!(v["tamam"], true);
        assert_eq!(v["oturum"]["uyari_sayisi"], 2);
        let v = post(u, "baslangic", json!({}));
        assert_eq!(
            v["oturum"]["kullanici"], "ayse",
            "süren oturum ekranı açtırmaz"
        );
        // Programı kapalı kişi: giriş reddedilir, ortak oturum kapatılmaz (başka programlarda sürer).
        let v = post(
            u,
            "giris",
            json!({"kullanici": "kasiyer", "parola": "Dogru-sifre1"}),
        );
        assert_eq!(v["durum"], "program_kapali");
        assert!(dz.join("oturum").exists(), "ret çıkış yapmaz");
        assert_eq!(
            post(u, "durum", json!({})),
            serde_json::Value::Null,
            "kapalı programda süren oturum yok sayılır"
        );
        let v = post(u, "basvur", json!({"kullanici": "ayse", "yeniParola": ""}));
        assert_eq!(
            (v["dogrulama_sayisi"].clone(), v["tur"].clone()),
            (json!("42"), json!("kilit"))
        );
        assert_eq!(post(u, "cikis", json!({})), json!({"tamam": true}));
        assert!(!dz.join("oturum").exists());
        // Şifre komut satırına hiç girmez (yalnız standart girdi).
        let args = std::fs::read_to_string(dz.join("argumanlar")).unwrap();
        assert!(
            !args.contains("Dogru-sifre1") && !args.contains("yanlis"),
            "{args}"
        );
        assert!(args.lines().all(|l| l.ends_with("--json")));
        let _ = std::fs::remove_dir_all(&dz);
    }
}
