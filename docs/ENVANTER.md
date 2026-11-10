<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T0 2026-11-05] -->
# TAA paket envanteri (T0, salt okunur tarama sonucu)

Kaynak: 10 envanter raporu (Haiku) + mimar doğrulaması (Cargo.toml / modul.toml / `OZELLIK` sabitleri, depo e747130).
Raporların ilk 6'sı tam elde; 7. rapor (dosya-donusturucu / taadon) kesik, 8-10 (TaaRkod, Argox Fiyat Etiketi ve biri belirsiz) bu turda **elde değil** → ilgili satırlar "belirsiz".

**Önemli düzeltme:** `crates/` altında **20** crate var (plan "21" diyor). 21. crate bulunamadı → belirsiz (planlı `taa-kur` mı, `taa-ipc` mi, sayım hatası mı Yusuf'a soruldu).

## 1. Ortak çekirdek

| Crate | Sürüm (Cargo = modul) | Veri | Yetenek | Sorun |
|---|---|---|---|---|
| taa-ayar | 0.1.0 = 0.1.0 | `TAA_AYAR` ortamının gösterdiği JSON `{"ozellikler":{ad:bool}}`; yazanı yok; `TAA_OZELLIK_<AD>` önce gelir (lib.rs:17-49) | ozellik_acik, json_den, kapali_mesaji | Bozuk/okunamayan dosyada sessizce varsayılana döner; şema sürümü yok; modul `ozellik-anahtari` kodda karşılıksız; test `set_var` yarışı |
| taa-dosya | 0.2.0 = 0.2.0 | Çağıranın yolu: JSONL `satir_ekle` (sürüm alanı yok), atomik yazım `.taa-<pid>-<n>.tmp`, `temp_dir()/taa-*` | bicim_algila, atomik_yaz, uzerine_yaz_atomik, gecici_klasor, satir_ekle, satirlari_oku | Süreçler arası kilit yok; atomik_yaz hard_link başarısızsa TOCTOU (lib.rs:108-115); yalnız kendi .tmp dosyasını siler |

## 2. Bileşenler (crate'ler)

| Crate | Sürüm | program | Veri konumu (yazar) | Yetenek | Sorun |
|---|---|---|---|---|---|
| taa-donustur | 1.6.0 = 1.6.0 | taadon | Çıktı zip/gz (CLI argümanı, `create_new`, "ad (n)" eki); girdi salt okunur | CLI: surum, imza, zip-listele/ac/olustur, gzip, gunzip; stdout tek satır JSON | Kural Node `src/donustur.js` CIKTI_IMZALARI ile çift tutuluyor (doğrulanmadı); `butce.acilis_ms=800` ölçüm kaynağı yok |
| taa-nebim | 0.1.0 = 0.1.0 | ortak | Okur: `<klasör>/*.csv` (Nebim dış süreç yazar; UTF-8/1254 varsayımı) | urunleri_oku, klasorden_oku, en_yeni_csv, csv_satirlari, fiyat_kurus | modul `csv` kodda yok (kod `nebim-alici`); xlsx yok; kodlama sessiz yanlış okuyabilir |
| taa-katalog | 0.1.0 = 0.1.0 | ortak | `<yol>/katalog.json` + `<dizin>/eski/<epoch>-<ad>` (tek yazar `Katalog::kaydet`) | ara (barkod/kod/ad), gecerli_fiyat, fiyat_farki, nebimden_kur | **fs::remove_file** lib.rs:109 (kopyadan sonra eski dosya kaldırılıyor; silme yasağı, Yusuf onayı); `eski/` sınırsız birikir; modul `fiyat-fark` kodda yok; şema sürümü yok |
| taa-yedek | 0.1.0 = 0.1.0 | taayedek | `<hedef>/<damga>/k<i>_<ad>/…` + `manifest.json` | yedekle, dogrula, geri_yukle (ezmez), sha256_dosya | modul `gerektirir` taa-dosya eksik; modul `dogrula/geri-yukle` kodda yok (kod `yedek`); manifest şemasız |
| taa-pdf | 0.1.0 = 0.1.0 | taapdf | Çağıranın çıktı yolu (atomik, "(n)") | bellekten, sec, birlestir, sikistir, kaydet | modul `birlestir/sec/sikistir`, kod `pdf-araclari`; çağıran program belirsiz |
| taa-rehber | 0.1.0 = 0.1.0 | taarehber | Kalıcı veri **yok**; çağıranın verdiği CSV metni | csv_oku, dogrula, hiyerarsi_metni, eposta_gecerli | `rehber.json` (§18.5) kodda yok; modul `hiyerarsi` kodda yok |
| taa-yazici | 0.1.0 = 0.1.0 | ortak | Kalıcı veri yok; PowerShell `Get-Printer` anlık | windows_listesi, csv_coz, tani | modul `tani`, kod `yazici-saglik`; `program=ortak` ama tüketen yok; PATH'ten `powershell` |
| taa-tara | 0.1.0 = 0.1.0 | taatara | Çağıranın çıktı PDF yolu (atomik) | jpeg_bilgi, jpeglerden_pdf, dosyalardan_pdf, ocr (yer tutucu) | modul `jpeg-pdf`, kod `tara` |
| taa-pano | 0.1.0 = 0.1.0 | taapano | Okur: `<klasör>/*.pano.json` (yazan **belirsiz**) | klasorden, Pano::metin | Üretici yok; `durum` doğrulanmıyor; şemasız |
| taa-kasa | 0.1.0 = 0.1.0 | taakasa | `<yol>.jsonl` kasa defteri (eklemeli) | kapanislar, kapanis_ekle, iptal, sayim_toplam | Tüketen program ve dosya adı belirsiz; şemasız |
| taa-fis | 0.1.0 = 0.1.0 | taafis | `<veri>/fisler.jsonl` (Defter) | ekle(_uyarili), raf_ata, imzala, teslim_degistir, durum_ata, escpos | **İki program yazıyor** (taa-fis-sunucu ve taa-telegram, kilitsiz); `terzi-kurallari.json` yolu defter klasörüne bağlı; 9 özellik anahtarı modul.toml'da yok; `TAA_FIS_BASLIK`, `TAA_FIS_NUSHA` |
| taa-fis-sunucu | 0.1.0 = 0.1.0 | taafis | `<veri>/cihazlar.jsonl`, `<veri>/fotolar/{no}-{n}.jpg`; okur `aydinlatma.html`, `zorluk.json`, `bildirimler.jsonl`; `--veri` varsayılanı **göreli** `taa-fis-veri` | CLI (--veri --port 8787 --yazici --adres --aydinlatma-url), HTTP uçları, ESC/POS yazıcı | modul `gerektirir` eksik: taa-bildirim, taa-telegram, taa-takip; başka programların crate'lerine sert bağlı |
| taa-terzi | 0.1.0 = 0.1.0 | taaterzi | Gömülü `src/islemler.json`; geçersiz kılma `<defter dizini>/terzi-kurallari.json` (elle) | varsayilan, json_den, dizinden, bul, kontrol | İki kural kaynağı; özellik anahtarı yok (`islem-katalogu`, `sinir-denetimi` kodda karşılıksız) |
| taa-kombin | 0.1.0 = 0.1.0 | taakombin | Veri yok | oner, normalize, takim_renkleri | — |
| taa-bildirim | 0.1.0 = 0.1.0 | taabildirim | `<dizin>/bildirimler.jsonl` (eklemeli), `<dizin>/bildirim-sablonu.txt` (elle) | hazir_bildir, Kuyruk (mesajlar, bekleyenler, isle) | `M-{n}` kimliği kilitsiz → çift kimlik; telefon düz metin, saklama süresi yok (KVKK); bozuk satır tüm okumayı durdurur; gerçek Gonderici yok |
| taa-takip | 0.1.0 = 0.1.0 | taatakip | Okur `fisler.jsonl`; `<dizin>/zorluk.json` (elle, gömülü varsayılan); yazar `<dizin>/usta-puan.jsonl` (eklemeli), `<dizin>/puan-ayar.json` | Takip (geciken, kargo, ozet, usta raporu), Zorluk, puan (PuanDefteri, PuanAyari, puan_hesapla, anomaliler, gozden_gecir) | — |
| taa-telegram | 0.1.0 = 0.1.0 | taatelegram | `<dizin>/telegram.json` (gizli anahtar; elle); `fisler.jsonl`a durum yazar | Ayar::dizinden, Bot::yanit (ağ yok) | modul `gerektirir` eksik: taa-takip, taa-dosya; fiş defterine ikinci yazar; sır dosyası |
| taa-subesor | 0.1.0 = 0.1.0 | taasubesor | Çağıranın verdiği JSONL defter | sor, yanitla, ozetler | `S-{n}` kilitsiz; dosya adı tanımsız |

## 3. Depo dışı programlar (salt okunur)

| Program | Sürüm | Veri konumu | Sorun |
|---|---|---|---|
| Tutaa / TAARNAK (`/home/user/tutaa`) | masaustu 2.4.5, tutaa-paketi/masaustu 2.5.0, lock 2.0.0 (**uyumsuz**) | `%APPDATA%\TAARNAK\{kayitlar, yedekler\yedek-<no>.json, kayit-kopyasi, yerel-ayar.json, imza-gunlugu.log, imza-anahtari.json, ocr-gecici}`; geri düşüş `Belgelerim\TAARNAK\kayitlar`; ayrıca `Belgelerim\TAARKOD\kayitlar`, `~\TAARKOD\kayitlar`; kurulum `%LOCALAPPDATA%\Programs\Tutaa` | Ad karmaşası (TAARNAK/Tutaa/TAARKOD); aynı tutanak 3 yerde; sabit `D:\Programlar\El-Yazisi-OCR\venv\Scripts\python.exe` (main.js:400, motor/motor.json); motor.json başka iş parçacığına ait; merkez API istemcisi ve `kimlik` kodda yok; modul.toml yok |
| TaaDön (dosya-donusturucu) | Rust çekirdek 1.6.0 | belirsiz (rapor kesik) | belirsiz |
| TaaRkod, Argox Fiyat Etiketi | belirsiz | belirsiz (rapor yok) | `etiket-kuyrugu` doğrulanamadı |
| LED Ekran Video (`/mnt/project-files/led-ekran-video/`, eklendi 2026-11-07) | program.toml 0.2.0 (Cargo 0.1.0, eşitlenecek) | Veri alanı yok; çıktı MP4/HAP kullanıcının seçtiği dosya; kurulum `D:\Programlar\LED-Ekran-Video\` | Tek PC; `ffmpeg` PATH'te gerekli; kendi deposu ve Windows derlemesi henüz yok |

## 4. Ortam değişkenleri
`TAA_AYAR`, `TAA_OZELLIK_<AD>` (büyük harf, `-`→`_`; 0/1/acik/kapali/evet/hayir), `TAA_FIS_BASLIK`, `TAA_FIS_NUSHA`, Tutaa'da `TUTANAK_MOTOR_KOMUT`, `TUTANAK_TEST`, `TUTANAK_MOTOR_ONISIT`.
Koddaki özellik anahtarları: kasa-defteri, fis-raf, fis-imza, serbest-islem, olcu-zorunlu(kapalı), kargo-alani, fis-logo, fis-aydinlatma, foto-eki, telegram-bot(kapalı), nebim-alici, katalog, yedek, pdf-araclari, rehber, yazici-saglik, tara, pano, kombin, bildirim-whatsapp, fis-takip, teslim-oneri, subesor.

## Temizlik listesi

**Yinelenen veri**
1. Nebim CSV ↔ `katalog.json` ↔ `eski/` birikimi (temizlik yok; silme yasağı nedeniyle arşiv politikası gerekir).
2. Tutaa tutanağı: `kayitlar` + `yedekler\yedek-<no>.json` + `kayit-kopyasi` (üç kopya).
3. Tutaa iki kopya kaynak ağacı (kök `masaustu/` ve `tutaa-paketi/masaustu/`).
4. Terzi kuralları: gömülü `islemler.json` + `terzi-kurallari.json`.
5. Arşiv/imza kuralları: taa-donustur ↔ Node `src/donustur.js` (doğrulanmadı).

**İki yazarlı alan**
1. `fisler.jsonl`: taa-fis-sunucu (taafis) + taa-telegram (taatelegram), kilitsiz.
2. `bildirimler.jsonl`: taa-bildirim API'sine taa-fis-sunucu ve taa-telegram ayrı yollardan çağırır; `M-{n}` yarışı.
3. `taa-subesor` defteri: `S-{n}` yarışı (çok süreç açarsa).
4. `motor/motor.json`: Tutaa ve El-Yazisi-OCR iş parçacığı.

**Sabit yol**
1. `D:\Programlar\El-Yazisi-OCR\venv\Scripts\python.exe` (Tutaa main.js:400, motor/motor.json:1).
2. `D:\Programlar\TAARNAK\` (DEVAM.md) ve plan §18.2 `D:\Programlar\TAA\taa-kur\`.
3. Göreli varsayılan `taa-fis-veri` (taa-fis-sunucu main.rs:24; çalışma dizinine bağlı).
4. Kodda sabit dosya adları (dizin çağırandan): fisler.jsonl, cihazlar.jsonl, bildirimler.jsonl, bildirim-sablonu.txt, zorluk.json, telegram.json, terzi-kurallari.json, aydinlatma.html.
5. `%APPDATA%\TAARNAK` / `TAARKOD` çift ad.

**Şemasız dosya** (hiçbirinde `sema`/sürüm alanı yok)
`TAA_AYAR` JSON, tüm `satir_ekle` JSONL'leri (fisler, cihazlar, bildirimler, kasa, subesor), katalog.json, yedek manifest.json, *.pano.json, zorluk.json, telegram.json, terzi-kurallari.json, Tutaa yedek JSON, yerel-ayar.json, motor.json.

**modul.toml ↔ kod uyumsuzlukları**
- `gerektirir` eksik: taa-fis-sunucu (taa-bildirim, taa-telegram, taa-takip), taa-telegram (taa-takip, taa-dosya), taa-yedek (taa-dosya).
- `ozellikler` koddaki anahtarla uyuşmuyor: taa-ayar, taa-nebim, taa-katalog, taa-yedek, taa-pdf, taa-rehber, taa-yazici, taa-tara, taa-terzi, taa-fis (9 anahtar eksik), taa-fis-sunucu (foto-eki, telegram-bot eksik).
- Sürüm: 20/20 crate Cargo = modul.toml (uyumlu). Uyumsuzluk yalnız Tutaa package.json'larında.

**Silme kodu (Yusuf onayı)**: taa-katalog lib.rs:109 `fs::remove_file` (kopyadan sonra). Diğerleri yalnız kendi geçici dosyası/test.

## Programlar arası zorunlu bağımlılık (ihlaller, §18.1)

| # | İhlal | Kanıt | Düzeltme önerisi |
|---|---|---|---|
| 1 | taa-fis-sunucu (taafis) → taa-telegram (taatelegram), taa-takip (taatakip), taa-bildirim (taabildirim) sert Cargo bağımlılığı | taa-fis-sunucu/Cargo.toml | Cargo `features` ile isteğe bağlı yap; çalışma zamanında `ister` yeteneği yoksa uç kapalı |
| 2 | taa-telegram → taa-fis, taa-bildirim, taa-takip (üç ayrı programın crate'i) | taa-telegram/Cargo.toml | taa-telegram'ı taafis programının modülü say (program alanı) ya da yazımı `fis.guncelle@1` yeteneğine çevir |
| 3 | taa-takip (taatakip) → taa-fis (taafis) | taa-takip/Cargo.toml | Okuma için taa-fis'in salt okunur kayıt tiplerini kullan; `veri_okur = fis-defteri` |
| 4 | taa-fis (taafis) → taa-terzi (taaterzi) | taa-fis/Cargo.toml | taa-terzi'yi `ortak` say (veri yok, saf kural motoru) — Yusuf kararı |
| 5 | taa-katalog/taa-rehber/taa-yazici → taa-nebim | Cargo.toml | taa-nebim `ortak`; ihlal değil |
| 6 | Tutaa → El-Yazisi-OCR venv (`D:\`, harici proje) | main.js:400-405 | Yol `motor.json`/ayar alanından; yoksa tarayıcı OCR yedeği; OCR düğmesi gizlenir |

Not: Rust crate'leri statik bağlandığından 1-4 çalışma zamanı program bağımlılığı değil, **derleme düzeyinde** bağlılıktır; ancak bir program "kaldırıldığında" kodu diğerinde yaşamaya devam eder ve aynı veriye ikinci yazar olur (bkz. iki yazarlı alan 1-2). Bu yüzden ihlal sayıldı.
