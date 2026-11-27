<!-- Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W0-kabuk 2026-11-27] -->
# TAA Web kabuğu (`web/kabuk/`) ve çekirdeği (`web/cekirdek/`)

Plan: `PLAN-TAA-WEB-2026-11-27.md` W0. Tek PWA; her program `web/moduller/<ad>/` altında bir modüldür.
Bağımlılık yok (saf ES modülleri). Derleme adımı yok: `web/` klasörü olduğu gibi sunulur, adres `…/kabuk/`.

## Dosyalar
| Dosya | İş |
|---|---|
| `../cekirdek/taa-cekirdek.js` | `acDB`, `Giden` (giden kutusu), `Onbellek`; IndexedDB, yoksa bellek içi sürücü |
| `index.html`, `kabuk.css`, `tema.js` | Sayfa iskeleti, İlmek renkleri (`--tk-*`), açılışta tema |
| `uygulama.js` | Giriş, modül yükleyici, hash rotası, çekmeceler, hesap paneli, gösterge, çıkış uyarısı |
| `yardimci.js` | DOM'suz saf işlevler (Node'da test edilir) |
| `sw.js` | Kabuk + çekirdek + modül dosyalarını önbelleğe alır; `/v1/` uçlarına dokunmaz |
| `moduller.json` | Kabuğun okuduğu modül listesi (kaynak: `../moduller/LISTE.json`) |
| `manifest.webmanifest`, `simge*.{svg,png}` | Yüklenebilir uygulama bilgisi |

## Çekirdek kuralları (taa-kuyruk ile aynı)
- `giden.ekle({program, uc, govde[, no]})` -> `{no, durum: 'bekliyor'}`. `no` = `cihaz-sayaç-zaman`. Aynı `no` ikinci kez eklenmez (`zatenVar: true`).
- `giden.gonder({sunucu, oturum, fetch})`: sırayla `POST <sunucu><uc>`; başlıklar `Authorization: Bearer`, `X-TAA-No`; gövdede `no` yoksa eklenir.
  2xx -> `gonderildi`; 4xx -> `reddedildi` (yeniden denenmez); ağ hatası / zaman aşımı / 5xx / 408 / 429 -> o kayıtta durur, sıra bozulmaz.
  **401 bilerek ret sayılmaz, durdurur** (süresi dolan oturum bütün kutuyu kalıcı reddetmesin).
- `giden.liste(durum)`, `giden.say()`; `onbellek.oku(anahtar)`, `onbellek.yaz(anahtar, deger, surum)` (eski sürüm yeniyi ezmez).
- Olay: `taa-giden-degisti` (`window` ve çekirdeğin `olaylar` nesnesi; diğer sekmelere BroadcastChannel ile).
- Silme işlevi yoktur. Veritabanı: `taa-<sube>-<kullanici>`; şube/kullanıcı başına ayrı.

## Modül yazana
- `web/moduller/<ad>/index.js` `{ad, baslik, ac(kok, baglam)}` dışa aktarır. `ac` bir kapatma işlevi döndürebilir.
- `baglam = {giden: {ekle, liste, say, gonder}, onbellek: {oku, okuKayit, yaz}, yetki(izin), tema: {ad, kip, koyu}, kullanici: {ad, sube}}`.
  Oturum anahtarı modüle verilmez; gönderimi kabuk yapar.
- Renk ve ölçüler `--tk-*` CSS değişkenleridir; tema değişince `window` üstünde `taa-tema-degisti` olayı gelir.
- Rota `#/<ad>/...`; alt rota değişiminde modül yeniden açılmaz, `hashchange` olayını modül dinler.
- Yeni modül: `moduller.json` içine `{ad, baslik, aciklama?, izin?, dosyalar[]}` ekleyin (`dosyalar` `web/` köküne göre). Kabuk dosyası eklerseniz `sw.js` içindeki `KABUK` listesine de yazın ve `SURUM`'u artırın.

## Oturum
- `POST /v1/giris {kullanici, parola[, kod]} -> {oturum, sube_id, roller[]}` (docs/T7-KIMLIK.md).
- Oturum anahtarı yalnız `sessionStorage` içindedir (sekme kapanınca gider). Profil (kullanıcı, şube, roller; anahtar yok) `localStorage` içindedir ve son girişten sonra 7 gün çevrimdışı açılışa izin verir.
- Çıkış: bekleyen kayıt varsa uyarı penceresi (Şimdi gönder / Vazgeç / Göndermeden çık). Sekme kapatılırken de tarayıcı uyarır. Yerel veritabanı silinmez.

## Test
```
node --test "web/test/*.test.js"
```
Çekirdek bellek içi sürücüyle, kabuğun saf işlevleri ve dosya tutarlılığı Node'da sınanır. IndexedDB sürücüsü, service worker ve
arayüz Node'da sınanamaz; 2026-11-27'de gerçek Chrome ile elle yazılmış geçici bir duman testiyle (giriş, çevrimdışı ekleme,
çevrimdışı yeniden açılış, 503'te durma, sırayla gönderim, 422 reddi, çıkış uyarısı) doğrulandı. Bu test depoda değildir
(Playwright harici bağımlılık olurdu).
