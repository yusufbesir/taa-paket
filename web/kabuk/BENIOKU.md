<!-- Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W0-kabuk 2026-11-27] -->
# TAA Web kabuğu (`web/kabuk/`) ve çekirdeği (`web/cekirdek/`)

Plan: `PLAN-TAA-WEB-2026-11-27.md` W0. Tek PWA; her program `web/moduller/<ad>/` altında bir modüldür.
Bağımlılık yok (saf ES modülleri). Derleme adımı yok: `web/` klasörü olduğu gibi sunulur, adres `…/kabuk/`.

## Dosyalar
| Dosya | İş |
|---|---|
| `../cekirdek/taa-cekirdek.js` | `acDB`, `Giden` (giden kutusu), `Onbellek`, `Gunluk` (cihaz günlüğü); IndexedDB, yoksa bellek içi sürücü |
| `../cekirdek/yol.js` | Gönderim yolları: merkez → şube bilgisayarı → bu cihaz; yol hatasını günlüğe yazar, günlüğü merkeze kopyalar |
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

## Gönderim yolları (W2, Yusuf 11-27; yazan: [Claude Fable 5.2 #taa-web-W2-yol 2026-11-27])
Kural: çevrimiçiyken cihaz **doğrudan merkeze** gönderir. Sıra: **merkez → şube bilgisayarı → bu cihaz**.
- Ayar biçimi (kabuk `localStorage` `taa-web-yollar`; kurulum da aynı biçimi yazabilir):
  `[{ad:'merkez',url},{ad:'sube',url[,sube]},{ad:'yerel'}]`. Sıra ayardan bağımsızdır; `yerel` hep vardır.
  Kabukta merkez her zaman kabuğun kendi kökenidir. Ayar ekranı: Hesap paneli → "Bağlantı yolları"
  (şube bilgisayarı adresi ve isteğe bağlı şube kodu). Şema yazılmazsa `http://`, kapı yazılmazsa `8788` eklenir.
- `giden.gonder({yollar, oturum, sube, cevrimici})`: önce merkez. Merkeze **ulaşılamazsa** (ağ hatası, zaman aşımı,
  5xx / 408 / 429) kalan kayıtlar aynı sırayla şube düğümüne (`taa-sube-dugum`, `POST <şube>/v1/<uç>`) gider; o da
  yoksa kayıt cihazda `bekliyor` kalır. Merkez yanıt veriyorsa (401 dahil) ya da oturum yoksa şubeye **geçilmez**.
  Sonuç: eski alanlar + `yol` (`merkez` / `sube` / `yerel` / `null`), `denemeler[]`, durunca `durmaTuru`.
- Şube yolunda: `Authorization` **gönderilmez** (oturum anahtarı şube ağına çıkmaz), `X-TAA-Sube` ve aynı `X-TAA-No` gider.
  Düğümün yanıtı 2xx ise kayıt `gonderildi` olur (`gonderim.yol = 'sube'`); merkeze düğüm taşır.
  Düğümün 409 `cakisma` yanıtı kalıcı rettir (şube bilgisayarındaki hal geçerli); öteki 4xx ret sayılmaz, kayıt merkezi bekler.
  Düğümün 256 KiB sınırını aşan kayıt (fotoğraf) düğüme gönderilmez; aynı programın o turdaki sonraki kayıtları da bekler
  (program içi sıra korunur), başka programların kayıtları gider.
- `giden.gonder({sunucu, oturum})` eskisi gibidir (tek yol; sonuç biçimi ve kayıt alanları değişmedi).
- **Cihaz günlüğü** (`db.gunluk`, IndexedDB `gunluk` deposu; veritabanı sürümü 2): yalnız ekleme, silme yok.
  `yaz({olay, yol?, ayrinti?})`, `liste({sonra, enCok})`, `say()`, `kopyaIsareti()`, `kopyalandi(sira)`.
  Yazılan olaylar: `yol.basarisiz`, `yol.duzeldi`, `yol.kullanildi`, `yol.yerel`, `yol.ayar`. Aynı neden sürerken yeni satır
  yazılmaz (sayılır, düzelince `bastirilan_tekrar` ile yazılır). Kayıt içeriği ve oturum anahtarı günlüğe girmez.
  Merkeze ulaşınca kopyalanmamış satırlar `POST <merkez>/v1/gunluk/cihaz` ile gönderilir (**öneri uç**; merkezde karşılığı
  doğrulanmadı). Satırlar cihazda kalır, yalnız kopya işareti ilerler. Kopya başarısızsa 10 dakika yeniden denenmez.
- Güvenlik politikası (`index.html`): `connect-src 'self' http://*:8788 https://*:8788`. Şube adresi bu yüzden 8788
  kapısında olmak zorundadır. Kabuk `https` kökeninden sunulurken düz `http://` şube adresini tarayıcı karışık içerik
  sayıp engeller (bkz. `docs/SUBE-DUGUMU.md` §13.6 madde 2); bu durumda şube yolu "başarısız" olarak günlüğe düşer ve
  kayıt cihazda bekler. Düğüme HTTPS gelene kadar şube yolu yalnız `http` kökenli kabukta çalışır.

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
Yol kuralları `web/test/yol.test.js` içindedir (sahte ağ, bellek içi sürücü).
2026-11-27 (W2 yol + Tutaa): gerçek Chrome (başsız) ile depoda OLMAYAN geçici bir duman testi yapıldı: sahte merkez +
8788'de sahte şube düğümü (hem 127.0.0.1 hem yerel ağ adresiyle), `http` kökeni. Görülenler: IndexedDB sürüm 2 ve `gunluk`
deposu, yol ayarı, merkez kapalıyken tutanak ve imzanın şubeye oturum anahtarı olmadan gitmesi, büyük fotoğrafın
küçültülüp `ocr_bekliyor` yazılması ve düğüme gönderilmeyip beklemesi, merkez açılınca doğrudan merkeze gitmesi, günlüğün
kopyalanması. **Denenmedi:** gerçek `taa-sube-dugum`, gerçek taa-sunucu, `https` kökeni (karışık içerik), telefon
tarayıcısı / Safari, service worker güncellemesi (v1 → v2), eski sürümden açık kalan sekmeyle veritabanı yükseltmesi.
Çekirdek bellek içi sürücüyle, kabuğun saf işlevleri ve dosya tutarlılığı Node'da sınanır. IndexedDB sürücüsü, service worker ve
arayüz Node'da sınanamaz; 2026-11-27'de gerçek Chrome ile elle yazılmış geçici bir duman testiyle (giriş, çevrimdışı ekleme,
çevrimdışı yeniden açılış, 503'te durma, sırayla gönderim, 422 reddi, çıkış uyarısı) doğrulandı. Bu test depoda değildir
(Playwright harici bağımlılık olurdu).
