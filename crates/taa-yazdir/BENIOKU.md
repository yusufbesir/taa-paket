<!-- Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-yazdir 2026-11-27] -->
# taa-yazdir: tarayıcıdan yazıcıya köprü

Plan: `PLAN-TAA-WEB-2026-11-27.md` §4. Tarayıcı Argox ya da Epson'a ham komut gönderemez; bu küçük hizmet şube
PC'sinde çalışır ve TAA Web sayfasından gelen işi yazıcıya iletir. Kurmak istemeyen tarayıcının kendi yazdırmasını kullanır.

## Güvenlik kuralları (kodda zorunlu, ayarla gevşetilemez)
- Yalnız `127.0.0.1` dinlenir. Dinleme adresi ayar değildir; `0.0.0.0` seçeneği yoktur.
- Karşı uç geri döngü değilse ve `Host` başlığı `127.0.0.1` / `localhost` değilse 403.
- `POST /yazdir` yalnız ayardaki `izinli_kokenler` listesindeki `Origin` ile kabul edilir. `*`, `null`, yol içeren köken ayara yazılamaz.
- Gövde sınırı `en_cok_bayt` (varsayılan 1 MiB, en çok 8 MiB); aşan gövde okunmadan 413. Başlıklar 16 KiB.
- ESC/POS içeriğinde para çekmecesi açma komutu (`1B 70`, `10 14 01`) varsa iş basılmaz: 400 `cekmece_red`
  (`docs/sozlesmeler/yazdir.sun.md` Kural 12).
- `/durum` yazıcı yolunu ya da IP'sini göstermez.

## Ayar
`<veri kökü>\yazdir\ayar.json` (ya da `--ayar <dosya>`). Örnek: `taa-yazdir ornek-ayar`.

| Alan | Anlam |
|---|---|
| `port` | 1024-65535, varsayılan 8797 |
| `izinli_kokenler` | Zorunlu liste. TAA Web'in açıldığı adres(ler), örnek `https://taa-web.ornek`. Boşsa hiçbir sayfa yazdıramaz |
| `en_cok_bayt` | Gövde sınırı |
| `yazicilar` | `{"argox-pplb": "<hedef>", "escpos": "<hedef>"}`. Hedef: yazılabilir yol (Windows paylaşılan yazıcı yolu, `LPT1`) ya da `tcp://IP[:9100]` |
| `cihaz_imza` | `kapali` (varsayılan) ya da `zorunlu` |

## Uçlar
- `GET /durum` -> `{tamam, hizmet, surum, en_cok_bayt, suruculer: {ad: "ayarli"|"bagli-degil"}, cihaz_imza, yazdirilan, hatali}`
- `POST /yazdir`
  - `Content-Type: application/json`: `{"surucu": "argox-pplb"|"escpos"|"sistem", "veri": "<metin>", "kodlama": "utf8"|"bayt"}`.
    `utf8` (varsayılan): metnin UTF-8 baytları gider. `bayt`: her karakter U+0000-U+00FF tek bayttır (ikili komutlar için; base64 yok).
  - `Content-Type: application/octet-stream` (ya da `text/plain`): gövde olduğu gibi gider; sürücü `/yazdir?surucu=escpos`.
  - Yanıt: 200 `{tamam: true, surucu, bayt}`; 400 bozuk iş; 401 imza; 403 köken/Host; 413 boyut;
    501 `{hata: "henüz bağlı değil"}` (hedefi ayarlanmamış sürücü ve `sistem`); 502 yazıcıya yazılamadı.
- `OPTIONS` ön uçuş isteği yanıtlanır (`Access-Control-Allow-Private-Network: true` dahil).

Tarayıcı örneği:
```js
await fetch('http://127.0.0.1:8797/yazdir', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ surucu: 'argox-pplb', veri: 'N\nA50,50,0,3,1,1,N,"TAA"\nP1\n' }),
});
```

## Cihaz imzası (kanca)
`CihazDenetimi` trait'i `POST /yazdir` isteğinin geçtiği doğrulama noktasıdır. `cihaz_imza: "zorunlu"` olursa
`<veri kökü>\cihaz\cihaz.pub` yüklenir ve mevcut `taa_cihaz::dogrula` çağrılır (başlıklar `X-TAA-Cihaz`, `X-TAA-Zaman`,
`X-TAA-Imza`; mesaj biçimi `taa_cihaz::imza_mesaji`). Yeni bir imza biçimi tanımlanmadı. Varsayılan kapalıdır, çünkü
tarayıcı sayfası bu bilgisayarın cihaz özel anahtarına erişemez; isteği kimin imzalayacağı kararı verilmedi (aşağıda).

## Test
```
cargo test -p taa-yazdir
cargo clippy -p taa-yazdir --all-targets -- -D warnings
```
Yazıcı sahtedir. Ağ testleri yalnız 127.0.0.1'de boş portta çalışır. Gerçek Argox / Epson ile deneme YAPILMADI.

## Açık noktalar
1. **`yazdir.sun@1` ile ilişki.** `docs/sozlesmeler/yazdir.sun.md` Kural 16 "yazıcıya giden tek yol ortak yazdırma hizmetidir"
   der; o hizmet (EB2) depoda henüz yok. Bu köprü bugün yazıcıya doğrudan yazar; EB2 yazılınca işi `is_id` ile
   `yazdir.sun@1`'e devretmeli (çift basmama defteri, yazıcı kaydı, eşteki yazıcı o hizmette). Şimdilik `is_id` ve defter yok:
   aynı isteği iki kez gönderen sayfa iki kez basar.
2. **Gönderme kodu kopyası.** `yaziciya_yaz` işlevi `taa-fis-sunucu`dakiyle aynıdır; ortak crate program crate'ine
   bağlanamadığı ve mevcut dosyalara dokunulmadığı için buraya alındı. Ortak bir crate'e taşınıp iki yerden çağrılmalı.
3. **Windows yazıcı adıyla ham gönderme yok.** Depoda winspool (RAW iş) yolu bulunmadı. USB'ye bağlı Argox için yazıcının
   Windows'ta paylaşılması ve hedefe paylaşım yolunun yazılması gerekir; bu yol gerçek cihazda denenmedi.
4. **`sistem` sürücüsü** bağlı değil (501). PDF/A4 basımı `yazdir.sun@1` işidir.
5. **Cihaz imzasını kim atar?** Seçenekler: TAA Web masaüstü kabuğu imzalar; ya da köken listesi yeterli sayılır. Yusuf kararı gerekir.
6. **Çekmece taraması** bayt düzeyindedir; ESC/POS görüntü (raster) verisinde aynı baytlar rastlantıyla geçerse iş
   reddedilir. Fiş içeriği üreten modül bunu bilmeli; gerekirse tarama komut çözümleyerek yapılır.
7. **Safari** `https` sayfadan `http://127.0.0.1` isteğini engelleyebilir (karışık içerik). Chrome ve Firefox geri döngüye
   izin verir. Gerçek tarayıcılarda denenmedi.
8. **Çalışma biçimi.** Açılışta başlatma ve `taa-kur` paketi (program.toml, kısayol) bu adımda yapılmadı.
9. `Cargo.lock` dosyasına cargo `taa-yazdir` girdisini ekledi (yeni dış crate yok).
