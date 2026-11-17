<!-- Sahip: Yusuf. yazan: [Claude#taa-cati-panel 2026-11-17] -->
# TAA Çatı paneli (`taa-cati-panel`)

TAA Çatı'nın grafik yönetim ekranı. Komut satırı `taa-cati` değişmedi; panel aynı `taa_cati::Cati` kitaplığını
kullanır, sunucuya giden her istek aynı kurallarla gider (cihaz imzası + Bearer, varsayılan reddet, 15 dk yeniden
doğrulama, silme yok). TAA Sunucu hizmetine dokunulmaz.

## Çalışma biçimi
- `taa-cati-panel` yalnız `127.0.0.1` üzerinde rastgele bir kapıda yerel sunucu açar ve Edge'i uygulama penceresi
  (`--app`, adres çubuğu yok) olarak başlatır. Edge yoksa varsayılan tarayıcı açılır.
- Pencere kapanınca (3 dk istek gelmezse) panel kendiliğinden kapanır; Ayarlar → "Paneli kapat" hemen kapatır.
- Oturum `taa-kimlik` deposundadır: panelden giriş yapmak `taa-kimlik giris` ile aynıdır (cihaz imzalı giriş,
  sunucu yanıt imzası doğrulanır, çevrimdışı yetki belgesi alınır). Çıkış yapınca kayıt silinir.
- Seçenekler: `--sunucu`, `--kok`, `--sertifika` (taa-cati ile aynı), `--pencere-acma` (adresi yazar).

## Ekranlar (izne göre görünür)
| Ekran | İzin | İşler |
|---|---|---|
| Genel bakış | — | sunucu sağlığı (`/v1/saglik`), bekleyen cihazlar, son olaylar, oturum |
| Cihazlar | `cihaz.yonet` | bekleyen/onaylı/iptal süzgeci, arama, onayla (istenen şubeyle), iptal (satır içi onay) |
| Kullanıcılar | `kullanici.yonet` | liste, ekle (rol + şube), parola değiştir, hesabı kapat (silme yok) |
| Şubeler | `kullanici.yonet` | liste, ekle |
| Program yetkileri | `program.yonet` | program × şube anahtarları; soluk anahtar genel kuraldan gelir; role özel kurallar komut satırında |
| Yönetici PIN'leri | `program.yonet` | program başına genel PIN ve şubeye özel PIN; belirle/değiştir/kaldır (taze parola). PIN yalnız buradan belirlenir; programlar (TerziTakip, Tutaa…) sunucuda yalnız doğrular |
| Günlük | `gunluk.gor` | olaylar, işleme göre süzgeç, daha eski olaylar |
| Ayarlar > Görünüm | — | 3 tema × açık/koyu (Temaya göre/Açık/Koyu/Sistem), temaya özel vurgu rengi, hareket (Sistem/Azalt/Normal), tanıtımı yeniden göster, varsayılana dön; çıkış, paneli kapat |

Yazma işleminde sunucu 15 dk penceresi isterse (428) parola penceresi açılır ve işlem bir kez yeniden denenir.

## Tasarım
Ekip turu v2 (Fable 5.2, GPT-6 Astra, Gemini 3.1 Pro), `/mnt/project-files/arayuz-modernlesme/taa-sunucu/`.
- **Kule Gecesi** (varsayılan): koyu, sol menü, amber yalnız "senden iş bekleyen" için. 120 ms.
- **Defter**: açık, üst sekmeler, başlıklar Georgia, mürekkep laciverti vurgu. 160 ms.
- **Taş**: mat taş, 92 px etiketli ray, 44 px dokunma hedefleri, zeytin vurgu. 200 ms.
- Logo: **Üç Çatı** (merkez, şube, cihaz), Yusuf'un seçimi; 16 px için ayrı çizim tasarım klasöründe.
- Yazı tipleri Windows'ta hazır olanlar (Segoe UI Variable, Georgia, Bahnschrift, Cascadia Mono); dış kaynak yok.
- Her temanın açık ve koyu hâli (6 palet, ekip turu 3, Fable paletleri; metin ≥ 7:1, ikincil metin ≥ 4,5:1, vurgu panelde ≥ 3:1).
- Görünüm tercihi `<veri kökü>\cati\gorunum.json` (sürümlü, atomik yazım; `taa_cati::gorunum`). Kavramlar TAA Kurucu ile
  ortak; kontrast hesabı `taa-kur-tema` crate'inden. Özel vurgu kaydedilirken değil, her kip için çizilirken denetlenir:
  okunabilirse aynen, yetmiyorsa metin rengine %4 adımlarla yaklaştırılır (en çok 12 adım), olmuyorsa temanın rengi kullanılır
  ve ekranda not çıkar. Yerel uçlar (oturumdan önce, çerezle): `GET/POST /api/gorunum`, `POST /api/gorunum/sifirla`.
- İlk açılışta 3 sahnelik tanıtım (cihaz onayı, yetki, günlük); Kule yukarı süzülür (120 ms), Defter sayfa çevirir (160 ms),
  Taş yumuşak geçer (200 ms). Atla/İleri/Başla, Esc atlar; bitince `tanitim_goruldu` yazılır.
- Hareket: "Sistem" `prefers-reduced-motion`'a uyar, "Azalt" her zaman kapatır, "Normal" sistem ayarını geçersiz kılar.

## Güvenlik (yerel sunucu)
- Yalnız 127.0.0.1; `Host` tam olarak `127.0.0.1:<kapı>` olmalı (DNS yeniden bağlamaya karşı), değilse 421.
- İki ayrı 32 baytlık rastgele belirteç: açılış bağlantısı tek seferlik (ikinci kullanım 403); çerez belirteci
  (`HttpOnly; SameSite=Strict`, kalıcı değil) adreste hiç görünmez. Her istek sabit süreli karşılaştırmayla denetlenir.
- POST: başka `Origin` 403, JSON olmayan gövde 415 (form ile CSRF ve CORS'suz basit istekler kapalı), gövde ≤ 64 KB (413).
- Yanıt başlıkları: CSP `default-src 'none'` (yalnız kendi betiği/biçemi), `frame-ancestors 'none'`, `X-Frame-Options: DENY`,
  `nosniff`, `no-referrer`, `no-store`, COOP/CORP `same-origin`.
- Sunucudan gelen her metin ekrana kaçışlanarak (esc) yazılır. Parola yalnız istek süresince bellekte; yanıtta dönmez.

## Doğrulama
- Linux: `cargo test -p taa-cati` (panel çekirdeği 12 test: belirteç, Host/çerez/köken/içerik türü denetimi, CSP,
  uç eşlemesi, 400/404, varsayılan reddet, 428, parola sızmaz), clippy `-D warnings`.
- Canlı duman testi (Linux, sahte kök): çerezsiz 401, yanlış Host 421, yanlış/ikinci açılış 403, başka köken 403,
  text/plain 415, büyük gövde 413, oturum yokken kod 5, `/api/kapat` süreci kapatır.
- Windows: `cargo clippy --target x86_64-pc-windows-gnu --release -D warnings` temiz. **Windows'ta elle denenmedi.**

## Yönetici PIN'i (sunucu sözleşmesi, tutaa `sunucu/src/yonetici_pin.rs`)
- `GET /v1/yonetici-pin` → `[{program, sube_id, belirlendi, guncellendi, guncelleyen}]` (PIN ve karma hiç dönmez).
- `POST /v1/yonetici-pin {program, sube_id?, pin}`, `POST /v1/yonetici-pin/kaldir {program, sube_id?}`: `program.yonet` + taze parola.
- `POST /v1/yonetici-pin/dogrula {program, pin}` (programlar): şube cihazdan; 200 `{gecerli, belirlendi}`;
  cihaz+program başına 5 yanlış = 5 dk kilit, 423 `{hata:"kilitli", kalan_sn}`.
- PIN 4–8 rakam; 0000 ve 1234/9876 gibi düz sıralar reddedilir (Çatı da aynı kuralı önceden uygular). argon2id ile saklanır.
  Program adları sunucudaki program yetkisi adlarıdır (ör. `terzitakip`).

## Sınırlar
- Kasa onayları ekranı yok (Çatı kitaplığında kasa uçları yok; ayrı adım).
- Role özel program yetkisi ve rol kaldırma yalnız komut satırında.
- İşlemler sırayla işlenir (tek iş parçacığı); yavaş sunucu yanıtı sırasında diğer istekler bekler.
