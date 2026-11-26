<!-- Sahip: Yusuf. yazan: [Claude#egitim-panel 2026-11-26] -->
# taa-egitim-panel

TAA Eğitim'in yerel grafik paneli (bin `taa-egitim-panel`). `taa-cati-panel` ile aynı kalıp: 127.0.0.1'de rastgele kapı, Edge `--app` penceresi, pencere kapanınca (3 dk sessizlik) kendiliğinden kapanma. Tasarım kaynağı: `docs/tasarim/GPT-REHBER-EGITIM-TASARIM.md` (Mezura dili; açık, koyu, tek renkli tema; hesap menüsü sırası kullanıcı, çizgi, Yönetici girişi, çizgi, Ayarlar / tema; üst çubuk tema rengini alır; açılış animasyonundan sonra giriş). Tek HTML dosyası, dış kaynak yok.

## Ekranlar
- **Eğitimler**: konu ve görev süzgeci (kasa eğitimi bir konudur), video listesi (atandı, başladı %, izledi, video yok / dosya yok etiketleri), "Kaldığın yerden devam et".
- **Oynatıcı**: HTML5 `<video>`, kaynak yerel dosya (`/video/<ders>`, Range 200/206/416, `taa_egitim::aralik_coz`) ya da https adresi. Kaldığı saniye 10 sn'de bir, duraklatınca ve sayfa kapanırken `taa-egitim` günlüğüne yazılır; açılışta o saniyeden devam eder. "İzledim" elle işarettir. Video yoksa ya da dosya bulunamazsa dürüst boş durum gösterilir; konum ve "izledim" kaydedilmez. Video üretilmez.
- **Atamalarım**: oturum sahibine atanan eğitimler.
- **İzleme Özeti** ve **+ Atama** (yalnız yönetici): eğitim ve kişi başına durum, atama (kişi + eğitim, ya da "görevine göre ata"), atamayı iptal (kayıt günlükte kalır).

## Çalıştırma
`taa-egitim-panel [--sunucu <adres>] [--kok <veri kökü>] [--sertifika <pem>] [--katalog <json>] [--rehber <json>] [--video-klasoru <klasör>] [--demo] [--pencere-acma]`

Veri kökü altındaki dosyalar (varsayılanlar): `egitim/katalog.json`, `rehber/rehber.json` (rehberin salt okunur önbelleği), `egitim/video/` (yerel videolar), `egitim/izleme.jsonl` (yalnız-ekleme izleme günlüğü), `egitim/gorunum.json` (tema). Katalog yoksa liste boş görünür; rehber yoksa kimse etkin sayılmaz ve ekranda uyarı çıkar.

`katalog.json`: `{"dersler": [<taa_egitim::Ders>...], "videolar": {"<ders-id>": {"yol": "kasa/acilis.mp4"} | {"url": "https://..."}}, "konu_adlari": {"kasa-egitimi": "Kasa eğitimi"}}`. Yol göreli, en çok 3 parça, mp4/m4v/webm/ogv; url yalnız https.

`--demo`: ÖRNEK etiketli katalog ve rehber, bellekte; diske hiçbir şey yazılmaz; video yok. Oturum sahibi rehberde yoksa bellekte ÖRNEK kişi olarak eklenir. Demo yönetici yetkisi vermez.

## Kimlik ve yetki (varsayılan REDDET)
- Giriş `taa-kimlik` ortak girişidir (cihaz imzalı, sunucu yanıt imzası doğrulanır, oturum depoya yazılır). Sahte ya da kodlanmış giriş yoktur.
- Yönetici uçları (`/api/ozet`, `/api/yonetim`, `/api/atama*`) yalnız şu dört koşul birlikte sağlanırsa açılır: yetki belgesi okunabiliyor ve süresi geçmemiş; belge aynı kullanıcıya ait; imza sunucu genel anahtarıyla doğrulanmış (anahtar kurulu değilse yönetici yok); `izinler` içinde `kimlik.yonetici` var. Aksi halde 403 (`kimlik::kullanici_olustur`).
- Atayan kişinin rehberde etkin olması gerekir (`taa-egitim` kuralı). Kişi yalnız kendi konum ve "izledim" kaydını yazar.
- Yerel sunucu: `Host` tam `127.0.0.1:<kapı>`, tek seferlik açılış bağlantısı, `HttpOnly; SameSite=Strict` çerez, POST'ta `Origin` ve JSON gövde, gövde en çok 64 KB, sıkı CSP (video için yalnız kendi sunucusu ve https).

## Test
`cargo test -p taa-egitim-panel` (42 test: güvenlik denetimleri, kimlik kararı, Range kararı, video yol/URL kuralları, katalog ve rehber yükleme, API uçları: oturumsuz 401, yönetici kapısı 403, süzgeç, konum/devam/izledim, atama/iptal/özet, günlükten yeniden kurma, demo, tema). Canlı duman testi (Linux, sahte kök): açılış çerezi, Range 206/416, konum yazımı, çerezsiz 401, başka köken 403, yönetici ucu 403, tema kalıcılığı; Chrome başsız ekran görüntüsü alındı.

## Henüz yok / açık
- Giriş adı ile rehber kişi kimliği eşlemesi: giriş adı küçük harfe çevrilip rehber kimliğiyle (`[a-z0-9][a-z0-9_-]{0,63}`) birebir eşleşiyor. Gerçek eşleme sözleşmesi (Çatı kullanıcısı ile rehber kişisi) yok; eşleşmeyen kullanıcı izleme kaydı yazamaz ve atama alamaz.
- `kimlik.yonetici` izin adı `rehber-sozlesmesi.md` §1'deki `kimlik.yonetici@1` yeteneğinden türetildi; sunucunun gerçek izin adı doğrulanmadı. Yetki belgesi yerelden okunur (3 gün çevrimdışı kuralı); her istekte sunucuya sorulmaz.
- Ders ekleme/düzenleme ekranı yok; katalog dosyadan okunur (katalog yazarı ve mağaza işleyişi EG5 kapsamı). Atama ve izleme günlüğü yerel `egitim/izleme.jsonl` dosyasıdır (sunucu tarafı EG1 yok; tek yazar varsayımı, dosya kilidi yok, iki panel aynı günlüğe yazmamalı).
- "İzledim" otomatik değil (yüzde kuralı verilmedi). Ders süresi kataloğa göre kırpılır; gerçek video süresi farklıysa katalog düzeltilmeli.
- Giriş ekranı ve oturum bağı (curl ile sunucuya giden kısım) Linux'ta ağsız denendi: gerçek sunucuyla ve Windows/Edge'de elle denenmedi. Altyazı ve yazılı döküm (tasarım belgesi üretim videoları için ister) yok; video dosyaları ve içerik Yusuf'tan beklenir.
- Ekran kontrastı ve klavye akışı tasarım belgesinin değerlerine göre yazıldı; ekran okuyucuyla denenmedi.
