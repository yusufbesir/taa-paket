<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T0 2026-11-05] -->
# Değişiklik günlüğü

## Yayınlanmamış

### Terzi Takip: main birleştirmesi (çifte kod eşleştirme yeni ekranlarda, taa-hata)

- **Çifte kod eşleştirme (PR #68) yeni arayüze taşındı:** eski `/yonetim` sayfası masaüstü **Telefonlar** ekranına yönlenir; o ekranda (yönetici PIN'iyle açılır) karekod + 6 haneli KOD 1, altında "Telefonda görünen 4 haneli kodu yazın" alanı ve **Onayla** düğmesi, sonuç iletisi ve süre bilgisi var. Bekleyen telefon olunca ekran bunu yazar ve alana kaydırır (`/api/yonetim/cihazlar` artık `bekleyen` sayısını döner; kod dönmez). Telefon sitesinde KOD 1 girilince belirteç verilmez; KOD 2 büyük gösterilir, 5 dk geri sayım, `GET /api/eslestir/<id>` yoklanır, `tamam` gelince belirteç `taa_tok`'a yazılır ve "Ana Ekrana Ekle" yönlendirmesi çıkar; 410'da "Eşleştirme iptal oldu ya da 5 dakikalık süre doldu…" iletisiyle ilk adıma döner.
- **Güvenlik:** `POST /api/yonetim/eslestir/onayla` artık yalnız yerel değil, yönetici oturumu da ister (PIN kilidi açıkken oturumsuz 401). Kişiye bağlı KOD 1 (kişisel telefon) çifte kodda da korunur: cihaz onayda o kişiye bağlanır; `eslestirme.jsonl`'a kişi kimliği yazılır, kodlar ve belirteç yazılmaz.
- **taa-hata (PR #74):** `main.rs` başında `taa_hata::kur_varsayilan`, `Cargo.toml` ve `modul.toml`'da `taa-hata` (taa-havuz bağımlılığı da duruyor). Android uygulaması eşleştirmeyi kendisi yapmıyor (WebView'de `/telefon?kod=` açar), değişmedi.
- Testler: `taa-fis-sunucu` 45 birim testi (yeni: `cifte_kod_belirtec_yalniz_yonetici_onayindan_sonra`; onaysız/oturumsuz/uzaktan belirteç yok, onaydan sonra bir kez, kişiye bağlanma, süre dolunca 410).

### Terzi Takip: ilk kurulum, TAA Çatı kişi/şube eşitlemesi, konsol hatası (Windows denemesi)

- **Kök neden 1 (kişi kaydı boşken fiş yok):** kişi eklemenin tek yolu yönetici girişiydi, yönetici girişi TAA Çatı PIN doğrulaması istiyordu; Çatı'ya ulaşılamayan ya da henüz bağlanmamış mağaza PC'sinde ilk gün fiş yazmak olanaksızdı. **Çözüm:** (a) kişilerin asıl kaynağı TAA Çatı/havuzdur (`rehber.kisi`, `rehber.birim`, `rehber.gorevlendirme`; `terzitakip` okuyucu, docs/rehber-sozlesmesi.md): sunucu açılışta ve 10 dakikada bir `taa-havuz` ile eşitler, bu şubede etkin görevlendirmesi olan kişileri görünen adla (M4: "Ad S.") `cati-kisiler.json` önbelleğine yazar; Çatı yokken son önbellek kullanılır (M5, H9). Boş olmayan Çatı listesi bir kez alındıysa **Çatı esastır**: yerel kayıt seçimden düşer (eski fişlerde adı çözülür), yönetici yerel kişi ekleyemez (409; tek kimlik M6). (b) **İlk kurulum** ekranı (`/api/kurulum`): yalnız bu bilgisayardaki masaüstü program (loopback adres + loopback `Host` + `X-TAA-Masaustu`, önde aracı başlığı yok), Çatı'dan hiç kişi listesi alınmamışsa, ilk kurulum yapılmamışsa ve kişi kaydı boşsa (ya da şube ayarlı değilse) şube ve kişiler TEK istekte bir kez girilir; `ilk-kurulum.json` (ad içermez) yazılınca yol kapanır. Kişi kaydı boşken Yeni fiş ekranı ve 422 yanıtı nedeni ve ne yapılacağını yazar.
- **Güvenlik gerekçesi (ilk kurulum):** yönetici yetkisi vermez (telefonlar, puan, kişi kapatma, şube değiştirme yine PIN ister); var olan hiçbir kaydı değiştiremez, yalnız boş kayda ekler; tek kullanımlıktır ve Çatı listesi gelince kalıcı olarak kapanır (H9 "yönetici işlemleri çevrimdışı yapılamaz" ile çelişmez: çalışan bir mağazada değil, hiç bağlanmamış boş bir PC'de bir kerelik önyüklemedir; sube-sozlesmesi §4.1 "paketsiz programda yerel yol sürer" ile aynı mantık). Parola, varsayılan hesap ya da gizli yönetici yok (H10). Girilen kişiler `beyan` düzeyindedir (M3) ve Çatı bağlanınca Çatı'dakiler esas alınır. E1 için ek önlem: `X-Forwarded-For`, `Forwarded`, `X-Real-IP` ya da `Tailscale-User-Login` başlığı taşıyan istek hiçbir uçta "yerel" sayılmaz (aracı arkasından ilk kuruluma ve yönetime erişim yok). Dürüst sınır: veri klasörüne dosya düzeyinde erişen biri `ilk-kurulum.json`'u silebilir; bu zaten `kisiler.jsonl`'u doğrudan düzenleyebilecek erişimdir, HTTP arayüzü bunu açmaz.
- **Kök neden 2 (sube.json yok):** TAA Kur şubeyi TAA veri kökündeki `sube.json`'a yazıyor, sunucu ise yalnız `--veri` klasöründekine (varsayılan çalışma klasörüne göre `taa-fis-veri`) bakıyordu; ekrandan ayarlanacak yol da yoktu. **Çözüm:** sıra `--veri/sube.json` → `<veri kökü>/sube.json` (TAA Kur) → TAA Çatı cihaz kaydı (`POST /v1/cihaz/durum` `sube_id`, sayıysa `GET /v1/sube` ile koda çevrilir; yalnız yerel şube boşsa yazılır, farklıysa kendiliğinden değişmez, uyarı olarak kalır). İlk kurulumda 15 şubeden seçim (TAA Kur ile aynı `kurulum/subeler.txt`); ayarlandıktan sonra yalnız yönetici değiştirir (`POST /api/yonetim/sube`, Personel ve usta ekranı).
- **Kök neden 3 (açılışta konsolda hata satırı):** Chromium/Edge geliştirici araçları açıkken loopback sitelerden `/.well-known/appspecific/com.chrome.devtools.json` ister; sunucu 404 dönüyordu, konsolda "Failed to load resource: 404" satırı çıkıyordu (Playwright + açık geliştirici araçlarıyla yerelde yeniden üretildi). Artık boş `{}` döner (çalışma klasörü önerilmez). Temiz klasörle açılışta konsol hatası 0.
- Testler: `taa-fis-sunucu` 43 birim testi (ilk kurulum yalnız yerel ve bir kez, Çatı listesi gelince kapanır, boş kayıt iletisi, şube TAA Kur kökünden, rehber süzgeci, eşitleme önbelleği, geliştirici araçları adresi). **Sınır:** gerçek TAA Sunucu'da havuz `rehber.*` uçları ve `/v1/cihaz/durum` şube alanı bu oturumda denenmedi; sahte kaynakla sınandı.

### Terzi Takip Android uygulaması (mobil/terzi-takip-android)

- Mağaza telefonları için yerel Kotlin + WebView kabuğu (`com.taa.terzitakip`, "Terzi Takip" 1.0.0; minSdk 26, targetSdk 37; AGP 9.5.1, Gradle 9.6.1 sarmalayıcısı). Telefon sitesini (`<sunucu>/telefon`) açar; gezinti yalnız kayıtlı sunucu kökünde, düz http yalnız yerel ağ IP'lerinde (10/8, 172.16/12, 192.168/16, 100.64/10), `.ts.net` yalnız https; kendi imzalı sertifikaya güven yalnız debug.
- İlk kurulum: eşleştirme karekodu (`/telefon?kod=`) okutulunca adres ve kod kendiliğinden dolar; adres SharedPreferences'ta. Gizli Sunucu ayarı: başlığa uzun basma. Açılışta sunucuya ulaşılamazsa markalı "Yeniden dene" ekranı. Android 12 SplashScreen ve uyarlanabilir/tek renk Yaka simgesi. Fotoğraf için dosya seçicide kamera.
- Yerel karekod okuyucu (CameraX + ML Kit bundled) ve kökene bağlı JS köprüsü `window.TaaAndroid` (`karekodOku()` → `taa-karekod` olayı). `telefon.html` köprü varsa Kod okut ve eşleştirme ekranında onu kullanır; yoksa davranış aynı.
- Release imza anahtarı depoya konmaz (`keystore.properties` / `TAA_IMZA_*`); Windows için `derle.ps1` (APK → `dist\TerziTakip-1.0.0.apk`, silmez). **Sınır:** bulut ortamında Android SDK indirilemediği için hiç derlenmedi ve telefonda denenmedi; ayrıntı README "Riskler".

### Terzi Takip: yeni arayüz (masaüstü program, telefon sitesi, 80 mm fiş, yönetici kilidi)

- **Masaüstü program `/masaustu`** (yalnız bu bilgisayar: loopback + Host + `X-TAA-Masaustu` başlığı): Ana sayfa, Yeni fiş (canlı 80 mm önizleme), Fişler (toplu işlem, okutarak seç, 10 sn geri al / Ctrl+Z), Fiş aç (F2), Telefonlar, Personel ve usta, Usta puanı, Ayarlar. Altı tema (Koçan varsayılan, Tebeşir, Askı, Çözgü, Çivit, Keten), ton, yazı boyutu, kontrast denetimli vurgu rengi, varsayılana dön, yazıcı kipi. Açılış ≤1 sn, 3 sahnelik tanıtım, hareketi azaltta animasyon yok, kısayollar ve barkod tabancası algılama. Dış kaynak yok; sayfalar `src/sayfa/` altında, `include_str!` ile gömülü, tema jetonları telefonla ortak (`tema.css`, `ortak.js`).
- **Başlatıcı `taa-terzi-takip`**: sunucu kapalıysa açar, Edge `--app` (ayrı profil) penceresi, yoksa varsayılan tarayıcı.
- **Telefon sitesi `/telefon`** yeniden yazıldı: altı tema, eşleştirme (`?kod=`), `taa_tok` / `taa_kuyruk` korunur (çevrimdışı kuyruk), kamera ile karekod (BarcodeDetector; yoksa elle), fiş sayfası (durum, raf, imza, okut, işaret, foto, teslim tarihi), PWA simgesi Yaka.
- **Kişi alanları**: Satış danışmanı, Ölçü alan, Fişi düzenleyen zorunlu, her fişte boş başlar; son 3 kişi çipleri, "Danışmanla aynı / Ölçü alanla aynı". Fişe ad değil kimlik yazılır (`*_id`), sunucu üçünü de kişi kaydına karşı denetler (422, eksik alanlar adıyla). Kişisel telefonda düzenleyen ve imzalayan sunucuda giren kişiye sabitlenir. Düzenleyen fişe basılmaz. Yerel kişi kaydı `kisiler.jsonl` + onaylı Telegram hesapları.
- **80 mm fiş**: müşteri nüshası (Danışman + Ölçü alan) ve ürün üstü nüsha (Ölçü alan + Usta + Kalan), 72 mm basılabilir alan, karekod sunucuda SVG (`GET /api/fis/<no>/karekod`), tarayıcı baskısında nüsha başına ölçülmüş `@page`; mevcut yazıcı yolu da kullanılır.
- **Yönetici kilidi**: Telefonlar, Personel ve Usta puanı yönetici PIN'i ister; PIN TAA Çatı `POST /v1/yonetici-pin/dogrula` ile sunucu tarafında doğrulanır, saklanmaz ve günlüğe yazılmaz; 10 dk boşta oturum kapanır; Çatı'ya ulaşılamazsa yönetim ekranları açılmaz.
- Yeni uçlar: `/api/oturum`, `/api/kisiler`, `/api/durum`, `/api/fis/<no>/karekod`, `/api/yazici/deneme`, `/api/yonetici/{giris,cikis,durum}`, `/api/yonetim/{eslestirme,cihazlar,kisiler,kisi,kisi/<id>/kapat}`. Eski `/yonetim*` sayfaları masaüstü ekranlarına yönlenir.
- Bağımlılık: `taa-kimlik`, `taa-kuyruk`, `taa-yol` (yerel crate'ler; yeni dış crate yok). **Sınır:** Çatı istemcisi yalnız sahte gönderici ile sınandı; gerçek Çatı'ya karşı denenmedi.

### TAA Çatı: açık/koyu temalar, Görünüm ayarı, tanıtım ve yönetici PIN'leri

- Görünüm (TAA Kurucu'nun yeni kavramlarıyla): 3 tema × açık/koyu (6 palet), `cati/gorunum.json` (sürümlü, atomik), temaya özel vurgu rengi ve okunabilirlik düzeltmesi (`taa-kur-tema` kontrast hesabı), hareket Sistem/Azalt/Normal, ilk açılışta 3 sahnelik tanıtım. Tercih artık tarayıcıda değil veri kökünde. Logo Üç Çatı olarak kaldı.
- Yeni ekran "Yönetici PIN'leri" (Yusuf 2026-11-17: yönetici PIN'i yalnız TAA Çatı'dan belirlenir): program başına genel ve şubeye özel PIN. Sunucu uçları tutaa deposunda (PR #41).
- Bağımlılık: `taa-cati` → `taa-kur-tema` (yerel crate; yeni dış crate yok).

### TAA Çatı: grafik yönetim paneli (`taa-cati-panel`)

- Yeni bin `taa-cati-panel` (crate `taa-cati`): 127.0.0.1'de rastgele kapıda yerel sunucu + Edge uygulama penceresi. Ekranlar: Genel bakış, Cihazlar, Kullanıcılar, Şubeler, Program yetkileri, Günlük, Ayarlar (izne göre görünür). 3 tema (Kule Gecesi varsayılan, Defter, Taş; düzen de değişir), vurgu rengi, hareketi azalt, varsayılana dön; Üç Çatı logosu ve yükleme ekranı. Panelden giriş `taa-kimlik giris` ile aynı akış. Ayrıntı ve güvenlik: docs/S3-CATI-PANEL.md.
- Güvenlik: Host denetimi, tek seferlik açılış belirteci + HttpOnly/SameSite=Strict çerez, Origin ve JSON zorunluluğu, sıkı CSP, gövde sınırı. Komut satırı `taa-cati` ve TAA Sunucu değişmedi.
- Bağımlılık: `tiny_http` 0.12 ve `getrandom` 0.3 (ikisi de çalışma alanında zaten kullanılıyor; yeni dış crate yok).
- Windows'ta elle denenmedi; Windows hedefi için clippy temiz.

### Terzi Takip: fiş defteri ileri uyumluluğu ve zincir özeti (TT1)

- **Şube kodu zorunlu (Yusuf 2026-11-15 21:30):** şube kodu olmadan fiş yazılamaz. `taa_fis::sube_kodu_gecerli` (Yusuf'un seçtiği biçim: 3 büyük harf önek + 2 haneli sıra numarası, ör. `MRK01`, `PND01`; önek `docs/sube-sozlesmesi.md`'deki `onek` değeridir, §3'teki `kod` slug biçimiyle uzlaştırılması sözleşme sahibi başlığa kalır); `Defter::ekle` boş ya da geçersiz kodu reddeder ve deftere hiçbir şey yazmaz. Basılı fişte "Şube: <kod>" satırı çıkar. Sunucu (`taa-fis-sunucu`) şube kodunu `sube.json` ({"kod":"..."}) dosyasından alır, istemcinin beyanına güvenmez: dosya yoksa ya da geçersizse 503, istemci başka şube beyan ederse 409 `sube_uyusmaz`. Telegram hesap bağlama kodu da şube kodu ister (yönetici hesabı hariç). **Dürüst sınır:** bu, fişin şube koduyla damgalanmasıdır; belgeyi şube imza anahtarıyla (ed25519) imzalama Ş3 işidir ve henüz yoktur. **Geçiş:** `sube.json` dosyasını TAA Kur yazacak; o zamana kadar el ile konmalıdır, yoksa yeni fiş yazılamaz. Eski şubesiz fişler okunur.
- **İmzaya kimlik düzeyi ve okutan kişi (Fable M1–M7):** `Imza.kimlik_duzeyi` (`dogrulanmis` / `beyan`) ve `okutan_kisi_id` (kayıtta ad yok, kişi kimliği); `Defter::imzala_kisili`: beyan için kişi seçimi zorunlu, beyan ile usta imzası atılamaz. Eski satırlar değişmeden okunur. Mağaza telefonu personel seçimi ekranı sonraki adım.
- **Mağaza telefonu imzası (Yusuf 17:52):** web `POST /api/fis/<no>/imza` artık `okutan` (personel kimliği) ister (yoksa 400, biçim geçersizse 400); `usta` rolüyle imza atılamaz (422); kayıt `beyan` düzeyinde ve `okutan_kisi_id` ile yazılır. **Sınır:** personel listesi (havuz KM2) henüz yok; kimliğin listede/etkin/aynı şubede olduğu sunucuda DENETLENMİYOR, yalnız biçimi denetlenir. Web sayfasına okutan alanı eklendi (listeden seçim ekranı KM2 ile).
- **Telefon eşleştirme: çifte kod (Yusuf 2026-11-17, Fable ekinin 1–7. maddeleri):** (1) yönetim sayfasında karekod + 6 haneli KOD 1 (tek kullanımlık, 5 dk); (2) telefon KOD 1'i girince belirteç VERİLMEZ, telefonda 4 haneli KOD 2 görünür (`/api/eslestir` artık `{id,kod2}` döner); (3) yönetici KOD 2'yi bu bilgisayarın yönetim sayfasına yazar (`POST /api/yonetim/eslestir/onayla`, yalnız yerel); (4) telefon `GET /api/eslestir/<id>` ile yoklar, belirteç bir kez verilir, sunucuda yalnız özeti durur; (5) 5 yanlış KOD 2'de bekleyen eşleştirmeler iptal; kodlar yalnız bellekte, günlüğe yazılmaz; `eslestirme.jsonl`'a cihaz, ad, şube, saat yazılır; (7) eşleşince "Ana Ekrana Ekle" yönlendirmesi. **Sınırlar:** (a) yönetici girişi/PIN (KM2) henüz yok, bu bilgisayardaki yönetim sayfası loopback ile korunuyor; (b) "yalnız mağaza ağında" için kaynak adres denetimi TT14 işi, `/api/eslestir` şimdilik ağ aralığı denetlemiyor; (c) hangi personele/kişiye bağlandığı KM2 kişi kaydı gelince eklenecek; (d) "Personeli kapat" KM2'de, "Cihazı kapat" mevcut.
- Şube kapsamı (K5, Telegram botu): usta, personel ve mağaza hesapları yalnız kendi şubelerinin fişini görür, imzalar, rafa koyar, hazır yapar ve listeler; başka şubenin fişi için "fiş yok" denir. Yönetici tüm şubeleri görür. Şubesi yazılmamış eski fişe denetim uygulanamaz, geçer. `Takip::sinirla` özet ve listelerin şubeye göre daraltılmasını sağlar. Not: fiş yazan uçların `Girdi.sube` alanını doldurması ayrı adımdır; doldurulana kadar yeni fişler de şubesiz kalır ve denetime takılmaz.
- `Fis.sube` ve `Girdi.sube` eklendi; eski satırlarda boş (`serde(default)`), okuma bozulmaz. `Imza` satırına isteğe bağlı `saat` ve `cihaz` alanları eklendi (boşsa dosyaya yazılmaz); `Defter::imzala_ayrintili` bunları yazar, `imzala` aynı kaldı.
- Zincir özeti: `Defter::zincir_baslat` bir "zincir başlangıcı" satırı yazar; ondan sonraki her satır sha-256 ile zincire girer. `zincir_ucu()` (satır sayısı + özet) ve `zincir_dogrula(uc)` ile merkeze gönderilmiş uç hâlâ defterde mi denetlenir. Eski satırlar yeniden yazılmaz ve zincire girmez.
- Sınır: zincir kısmi değişikliği yakalar; dosyanın tamamını yeniden yazan birini tek başına durdurmaz, bu yüzden uç eşitlemede (TT3) merkeze gönderilir. Eski sürümde çalışan program zincir başlangıcı satırını bilmez ve defteri "bozuk" sayar: zincir, tüm şube bilgisayarları güncellendikten sonra başlatılmalıdır.
- Yeni bağımlılık yok (sha2 çalışma alanında zaten vardı).

### Terzi Takip: personel ve usta QR ile eklenir (TT12c devamı)

- Yönetici `GET /yonetim/personel` ekranında (yalnız sunucu bilgisayarı) rol (usta, personel, mağaza telefonu), şube ve kendine özel not seçip kişiye özel, tek kullanımlık, 10 dk geçerli QR üretir. Kişi QR'ı telefon kamerasıyla okutunca Telegram `t.me/<bot>?start=baglan-<kod>` ile açılır ve hesap bağlanır; telefonda ve yönetici ekranında AYNI 4 haneli doğrulama sayısı görünür, yönetici sayıyı karşılaştırıp son onayı verir. İmzanın dayanağı Telegram'ın değişmeyen kullanıcı numarası, profil adı da kayda yazılır. Kodu elle yazmak yedek yol olarak kalır. Yeni rol `personel` (fiş görür, raf yazar; usta imzası ve WhatsApp düğmesi yok).

### Terzi Takip: Telegram botu (TT12c)

- `taa-telegram`: fiş kare koduyla bot açılır (`t.me/<bot>?start=<fiş no>`, kodda ad/telefon yok); fiş maskeli gösterilir, usta her KALEM için "Bitti" der, "Bitti mi? Evet/Vazgeç" onayından sonra imzalanır (anahtar: fiş, kalem, tur; yeniden iş yeni tur). Tüm kalemler imzalanınca fiş hazır olur ve müşteri bildirimi kuyruğa yazılır. İmza olayı `imza-olaylari.jsonl`'e `taa-takip` Olay alan adlarıyla yazılır (usta = rastgele `U-` kimliği, tg_id, o anki profil adı; @kullanıcı adı alınmaz).
- Hesap bağlama (Fable denetimi K1): yönetici kısa ömürlü tek kullanımlık 8 haneli kod üretir, usta bota yazar, hesap bağlanır, yönetici son onayı verir; kodsuz hesap için hiçbir şey kaydedilmez. Hesap başına dakikada 20 komut (ayar), saatte 5 yanlış kod.
- "Müşteriye WhatsApp'tan haber ver" ve "Gönderdim" düğmeleri yalnız `magaza` rolünde; mağaza hesabı usta imzası atamaz. Düğme yerel sunucudaki tek kullanımlık, o güne ait `/wa/<belirtec>` adresine gider; `wa.me` bağlantısı (numara + hazır metin) yalnız yerel ağda üretilir, Telegram'dan müşteri numarası geçmez.
- `yoklama` (uzun yoklama): `Aktarim` arayüzü, kalıcı ofset, tek süreç kilidi, `curl -K -` ile bot anahtarı komut satırında/hata metninde yok. Gerçek Telegram'da DENENMEDİ, `CurlAktarim` yalnız birim testlerle (sahte aktarım) sınandı. Webhook yolu korunur.
- `taa-fis-sunucu`: `GET /wa/<belirtec>`, yalnız bu bilgisayardan `POST /api/yonetim/baglama-kodu`, `GET /api/yonetim/hesaplar`, `POST /api/yonetim/hesap/<tg_id>/onayla|askiya|kaldir`. Yeni bağımlılık yok (`getrandom` zaten çalışma alanında).
- Henüz YOK: şube kapsamı denetimi (K5, `Fis.sube` TT1'de gelince), yönetici ekranı düğmeleri, fişe bot kare kodu basımı (TT12d), yönetici adına işaretleme (K9), zincir özeti (TT1).

### Kurulum ekranı: yerel pencere (tarayıcısız)

- `kurulum/`: tarayıcı + yerel HTTP ekranı KALDIRILDI (`arayuz.rs`, `arayuz.html`, HTTP testleri). Yerine kurucunun kendi Win32 + GDI penceresi (`pencere.rs`, yalnız Windows; `windows-sys` 0.61, Yusuf'un onaylı bağımlılığı, yeni crate yok). Onay, rol, şube (gömülü 15 şube, `kurulum/subeler.txt` → `<kök>\sube.txt`), isteğe bağlı programlar pencerede seçilir; bekleme kartları; sağda adım adım şube ayar direktifleri (`direktif.rs`): kurulum programının yanındaki `sunucu-yollari.json`, `sunucu-ca.pem`, `tailscale.anahtar` var olanı ezmeden yerine kopyalanır, kalanlar numaralı adım. Açık/koyu (sistem teması), yüksek DPI, klavye, kapatma onayı. `--sessiz`/`--arayuzsuz` iken pencere yok; `--sube <kod>` eklendi. Saf mantık platformdan bağımsız ve testli (`secim.rs`, `ekran.rs`, `subeler.rs`, `direktif.rs`; 22 test).
- Gerçek Windows'ta DENENMEDİ; Linux'ta wine 9 + Xvfb ile sahte `taa-guncelle` üzerinden uçtan uca (klavye, fare, hata + yeniden dene, koyu tema, kapatma onayı) denendi.

### Kurulum ekranı (ilk sürüm, tarayıcılı; yerini yerel pencere aldı)

- `kurulum/` (TAA-Kur-Kurulum.exe): tarayıcıda ilerleme ekranı (std-only yerel HTTP, yalnız 127.0.0.1, Host denetimi, komutlar jetonlu). Aşamalar, program başına İndir/Doğrula/Kur/Ayarla, yüzde, hata + Yeniden dene / Bitir; bekleme kartları (8 TAA programı + rehber notları, `kurulum/kartlar.txt`); tek vurgu rengi, açık/koyu, renk seçici, klavye, `prefers-reduced-motion`. `--arayuzsuz` kapatır, `--sessiz` iken kapalı (`--arayuz` açar). Saf mantık `ilerleme.rs`, `kartlar.rs`, `arayuz.rs` (16 birim test; çalıştırma: `TAA_KUR_PAKET=<boş klasör> cargo test --manifest-path kurulum/Cargo.toml`). Yeni bağımlılık yok. `docs/KURULUM-EKRANI.md`.
- `taa-guncelle`: `TAA_ILERLEME=1` iken stderr'e `TAA-ILERLEME<TAB>paket<TAB>adım<TAB>basladi|bitti` satırları (`ilerleme_satiri`, 1 test). Ortam yoksa davranış aynı.
- Windows'ta ÇALIŞTIRILMADI; yalnız `cargo clippy --target x86_64-pc-windows-gnu` tür denetimi. Linux'ta sahte `taa-guncelle` ile uçtan uca ve başsız Chrome ekran görüntüsüyle denendi.

### Usta puanı (QR)

- `taa-takip::puan` (yeni): QR okutma olayları, tezgâhtar işareti + yönetici tutanağı, 0–10 usta puanı (verimlilik, kalite, müşteri, zamanında, düzen), küçük örnek düzeltmesi, deneme dönemi, ani değişim uyarıları (yalnız yönetici), kural tabanlı yorum ve veriye göre sınırlı kendini ayarlama (`gozden_gecir`). Özellik anahtarları `qr-uretim` ve `usta-puan`, ikisi de varsayılan KAPALI. Usta puanı göremez; izin `takip.puan@1` yalnız yöneticiye verilmelidir (çağıran program uygular). 13 birim test. `docs/USTA-PUAN.md`. 
- `taa-fis-sunucu`: `POST /api/fis/{no}/okut`, `POST /api/fis/{no}/isaret` (telefon), yönetici uçları `POST /api/yonetim/tutanak`, `GET /api/yonetim/puan/{bas}/{bit}`, `POST /api/yonetim/puan/gozden-gecir/{bas}/{bit}` (yalnız sunucu bilgisayarı). Fiş sayfasına "İşi bitirdim (okut)" ve "Yeniden iş / şikâyet işaretle" düğmeleri. Yeni bağımlılık yok. Ayrıntı `docs/USTA-PUAN.md`.

### Usta puanı ekranı

- `taa-fis-sunucu`: yönetici puan ekranı `/yonetim/puan` (yalnız sunucu bilgisayarı): puanlar, yorumlar, ani değişim uyarıları, işaretler ve tutanak formu, ayar gözden geçirme düğmesi; `GET /api/yonetim/isaretler`. Yeni bağımlılık yok.

### T3-pencere

- `taa-kur-pencere` (yeni bin) ve `taa_kur::pencere_model`: paket listesi (ad, kurulu sürüm, seçili sürüm, durum), Kur / Kaldır / Güncelle düğmeleri ve alt durum satırı. Mantık platformdan bağımsız modelde (7 birim test, gerçek `Kur`, geçici kök); Win32 çizimi `#[cfg(windows)] win.rs` içinde ince katman.
- Güncelle düğmesi `GuncelleyiciArayuzu` trait'ine bağlı; varsayılan "Güncelleme kaynağı ayarlanmadı" der (taa-guncelle başka dalda).
- Kaldırma silmez: `Kur::kaldir` ile `eski\` klasörüne taşır; veri alanlarına dokunulmaz.
- Bağımlılık: yalnız Windows'ta `windows-sys 0.61` (Yusuf onaylı; `zip` zaten dolaylı olarak aynı sürümü çekiyordu), asgari özellikler. Windows'ta elle doğrulanmadı; tür denetimi `--target x86_64-pc-windows-gnu` ile yapıldı. `docs/T3-PENCERE.md` eklendi.
### T3-imza
- Yusuf onaylı yeni bağımlılıklar: `ed25519-dalek 2` (`default-features=false`, `std`) ve `toml` (`parse`). Başka bağımlılık, HTTP ve silme yok.
- `crates/taa-imza` eklendi: ed25519 anahtar üret/imzala/doğrula, hex, `ed25519:<hex>` genel anahtar dosyası, `taa-imza uret|imzala|dogrula` CLI. Özel anahtar yalnız verilen dosyadan okunur (0600, ezmez, günlüğe/`Debug`'a yazılmaz).
- `crates/taa-kur`: `paket.json` `imza` = `{alg, anahtar_kimlik, deger}` (imzalanan veri `icerik_sha256`). `Kur::yeni/ortamdan` `<kök>\anahtarlar\*.pub` okur; anahtar varken imzasız/yanlış imzalı paket `KurHata::Dogrulama` ile reddedilir, anahtar yokken eski davranış (+ `tani` uyarısı). Bozuk `.pub` imza denetimini kapatmaz. `Kur::imza_denetle`, `tani` alanları `uyarilar` ve `guvenilir_anahtarlar`. `kur.toml` (`kaynak_klasoru`, `kanal`) `toml` ile okunur (`kur_ayari_oku`).
- `crates/taa-guncelle` eklendi: `yayin.json` şeması, `Kaynak` trait'i + `YerelKaynak`, `kontrol` (zorunlu `en_az_surum`, geri alma koruması), `uygula` (SHA256 + imza, `kur --sec`, hatada önceki sürüme dönme; yama yalnız şemada, `tam`a düşülür), `taa-guncelle kontrol|uygula [--json]`.
- `tools/paket_uret.py` artık `imza` alanı yazmaz; hazır imza `--imza-dosya` ile alınır. `docs/T3-TASARIM.md` eklendi.
### T3-boru
- `taa-ipc`: Windows adlandırılmış boru taşıyıcısı (`BoruTasiyici`, `#[cfg(windows)]` `src/boru.rs`; `Tasiyici` trait'i, dosya tabanlı `GelenKutusu`'na yedekli). Yusuf onaylı tek yeni bağımlılık `windows-sys` (yalnız `[target.'cfg(windows)'.dependencies]`). Platformdan bağımsız parça `src/boru_ortak.rs` (boru adı `\\.\pipe\taa-<kullanici>-<program>`, 4 bayt uzunluk + JSON çerçeve, 256 KiB sınır, zaman aşımı, ACL = yalnız geçerli kullanıcı, `FIRST_PIPE_INSTANCE`, `YedekliTasiyici`) Linux'ta test edilir. `docs/ipc.md` "T3: Windows adlandırılmış boru" bölümü eklendi. DOĞRULANMADI: Windows kodu yalnız `cargo check --target x86_64-pc-windows-gnu` ile tür denetiminden geçti, Windows'ta çalıştırılmadı.

### T2-A4 (güvenlik sertleştirme; Opus incelemesi, 0 yüksek / 9 orta / 7 düşük)
- `taa-veri.js`: `eskiyeTasi` dışa açıldı (programlar bozuk alan dosyasını `<alan>/eski/` altına alır). `docs/veri-alani.md`: `rehber` + `TAA_VERI_KOK` kuralı kodla hizalandı.

- `araclar/js/taa-veri.js` v2: `'wx'` + `fsync` ile atomik yazım; ilk `SAHIP.toml` yazımı yarışsız (`wx`, yeniden okuma); `oneriler()` yalnız düz dosya, ≤256 KiB, zarf doğrulaması (`sema`, `oneren`, `degisiklik`), kimlik dosya adından (yük içindeki `id` yok sayılır); `altYol` ayrılmış üst adları harfe duyarsız, Windows aygıt adlarını, sonda nokta/boşluğu ve `:` reddeder; `oneriKapat`/`eskiyeTasi` hedef adı `wx` ile ayırır (ezme yok); bilinmeyen `SAHIP.toml` anahtarları korunur; `goc` `ekler` (ör. `.bak` da `.tasindi`). Programlar kopyayı güncellemeli.
- `crates/taa-veri`: ilk `SAHIP.toml` `create_new` ile (yarışta yeniden okur); `yol_denetle` harfe duyarsız ayrılmış adlar, Windows aygıt adları, sonda nokta/boşluk, denetim karakterleri.
- `crates/taa-kur`: `yol_dogrula` Windows aygıt adları ve sonda nokta/boşluk reddi; zip yinelenen ad denetimi harfe duyarsız; `kur_zip` geçici klasörü `create_dir` ile alır (boş ve yarışsız).
- Açık (T3): sahip tablosunun çalışma anında kitaplığa gömülmesi (sahte `gecici`/sahte asıl sahip), alan içi sembolik bağ denetimi, SAHIP.toml çözücüleri için ortak test kümesi, `eski/` boyut uyarısı (`docs/T2-TASARIM.md` §6).


### T2-A3

- `tools/sozlesme_denetle.py` yeni program modu: `--program <program.toml> [--surum-dosyasi <package.json|Cargo.toml>] [--paket-kok <taa-paket>]` (zorunlu alanlar, ad ve alan adı biçimi, `gerektirir` yalnız `taa-*`, `sunar`/`ister` biçimi ve tekilliği, sürüm eşitliği, `ayar_alani`, `docs/veri-sahipleri.toml` ile sahip/okur çapraz denetimi; `[veri.<alan>] gecici = true` geçici sahip uyarı verir). Mevcut paket denetimi değişmedi; 13 yeni test.
- `docs/veri-sahipleri.toml` (paket genelinde veri alanı sahip tablosu) eklendi.
- `docs/program-sozlesmesi.md`: "Taslak örnekler" yerine dört programın kesin değerleri (tutaa, taadon, taarkod, fiyatetiketi), taarehber "ayrı başlık, taslak"; geçici sahip notasyonu belgelendi.
- `araclar/js/taa-veri.js` (bağımlılıksız CommonJS veri alanı kitaplığı, programlar kopyalar) ve `araclar/js/test/taa-veri.test.js` eklendi; CI'ya `js` işi (Node 22, `node --test`) eklendi.

### T2-A1

- `crates/taa-veri` eklendi (program = ortak, gerektirir = taa-dosya; yeni Cargo bağımlılığı yok): veri alanı sahipliği (`ac_sahip`, `ac_gecici_sahip`, `ac_okur`), atomik yazım, öneri kutusu (`oner`, `oneriler`, `oneri_kapat`), `SAHIP.toml` düz TOML alt kümesi çözücü/yazıcı (hata satır numaralı). Kodda silme yok; devralınan `SAHIP.toml` ve kapatılan öneriler `eski/` altına taşınır.
- `docs/veri-alani.md` (veri alanı sözleşmesi: kök, SAHIP.toml, 6 kural, hata adları, öneri biçimi, göç kuralı) ve `docs/veri-sahipleri.toml` (paket genelinde sahip tablosu) eklendi.
- README crate tablosunda `taa-veri` "var"; `docs/TEK-CATI.md` §3'e T2 durum notu eklendi.

T1 (kurulum yöneticisi ve iletişim; yeni Cargo bağımlılığı yok).

- `taa-kur` (klasör paketi kurulumu, sürüm seçme, geri dönme, SHA256 doğrulama; zip ve imza T2) ve `taa-ipc` (dosya tabanlı gelen kutusu taşıyıcısı) crate'leri eklendi.
- `tools/paket_uret.py` (+ testi) eklendi: `modul.toml` + Cargo sürümü -> `paket.json`; `icerik_sha256` tanımı `paket.json`'a `icerik_sha256_tanim` olarak yazılır ve `docs/T1-TASARIM.md` §1.2'de kesinleştirildi.
- `tools/sozlesme_denetle.py`: iki yeni denetim (hata): `gerektirir` grafında döngü; istenen arayüz ana sürümü hedef modülün `arayuz` değerine eşit olmalı. Testler eklendi.
- README crate tablosu: `taa-kur` ve `taa-ipc` "var".
- `docs/TEK-CATI.md` §3'e T1 durum notu eklendi.

### T2-A2

- `taa-kur kur <paket.zip>`: bağımsız değişken `.zip` ile bitiyorsa zip `<kök>/gecici-<pid>/` altına açılır (yol kuralları klasör paketiyle aynı; en çok 10.000 girdi, açılmış toplam en çok 2 GiB; bildirilen boyutlar toplanıp sınır aşılırsa hiçbir şey açılmaz; sembolik bağ ve yinelenen ad reddedilir; `paket.json` zipin kökünde olmalı), sonra mevcut klasör kurulumu çalışır; geçici klasör iş bitince (hata dahil) `eski/` altına taşınır, silinmez. `zip` bağımlılığı `taa-donustur` ile aynı sürüm ve özelliklerle (yeni sürüm ya da özellik yok).
- `paket.json` şemasına ayrılmış `imza` alanı (`null`; Rust yok sayar, doğrulamaz; T3 imza). `tools/paket_uret.py --zip <cikti.zip>` aynı içeriği zip olarak üretir; testler eklendi.
- `taa-kur tani --json`: `veri_kok` ve `veri_alanlari` (`TAA_VERI_KOK` ya da platform varsayılanı altındaki her `<alan>/SAHIP.toml` için `{alan, sahip, gecici, sahip_kurulu}`); bozuk SAHIP.toml tanı `sorunlar` listesine girer. SAHIP.toml düz TOML alt kümesi taa-kur içindeki küçük yerel çözücüyle okunur (`taa-veri`'ye bağımlılık yok).
- `taa-katalog`: `kaydet` artık `fs::remove_file` kullanmaz; eski dosya `eski/<epoch>-<ad>` olarak `rename` ile taşınır (olmazsa kopyalanır ve eski dosya `<ad>.tasindi` adını alır); `eski/` altındaki dosya hiçbir zaman ezilmez. Test eklendi.

T0 (tek çatı, veri ve yapı temizliği). Kırıcı değişiklik yok; yalnız ek alanlar, belge ve denetim.

- 20 `modul.toml` dosyasına `sunar`, `ister`, `veri_sahibi`, `veri_okur`, `ayar_alani` alanları eklendi (mevcut alanlar korundu).
- `tools/sozlesme_denetle.py` (+ testi) eklendi; CI'ya sözleşme denetimi adımı eklendi.
- `docs/ENVANTER.md` (veri konumları, ayar dosyaları, sürümler) eklendi.
- `docs/TEK-CATI.md` (plan §18'in depo gerçeğine uyarlanmış hali) ve `docs/program-sozlesmesi.md` (program.toml taslağı) eklendi.
- `docs/sozlesmeler/` altında yetenek sözleşmeleri eklendi (bilinmeyen alanlar "belirsiz").
- `uyumluluk.toml` (test edilmiş küme taslağı) eklendi.
- Karar: `taa-terzi` program=ortak; `taa-bildirim`, `taa-takip`, `taa-telegram` program=taafis (TaaFiş ailesi tek program). Ayrı TaaTerziTakip programı T2'de Cargo features ile.
- Denetim betiğine: gerektirir = Cargo bağımlılıkları birebir; ince çizgi (başka programın crate'ine zorunlu bağımlılık hata). CI'ya `permissions: contents: read`.
- `docs/modul-sozlesmesi.md` yeni alanlarla güncellendi; README "Modül sözleşmesi" bölümüne bağlantılar eklendi.

## Teslim tarihi önerisi (TaaSüre)
- taa-takip: `sure` modülü; usta kuyruğuna ve `sure.json` sürelerine göre tahmini teslim tarihi önerir, tablo yoksa "tahmin yok, elle seç". Özellik anahtarı `teslim-oneri`, yeni yetenek `teslim.oneri@1`. Fiş ekranına düğme bağlama ayrı adım.
