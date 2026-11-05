<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T2 2026-11-05] -->
# Veri alanı sözleşmesi

Kaynak: `docs/T2-TASARIM.md` §1 ve §1.1. Uygulama: `crates/taa-veri` (Rust), `araclar/js/taa-veri.js` (Node; ayrı iş). Paket genelinde alan sahipleri `docs/veri-sahipleri.toml` dosyasındadır.

Bir veri alanı, birden çok programın paylaştığı ama **tek bir programın yazdığı** bir klasördür. Okuyanlar alana yazmaz; değişiklik isterse öneri bırakır.

## 1. Kök ve düzen

Kök: `TAA_VERI_KOK` ortam değişkeni; yoksa Windows'ta `%APPDATA%\TAA\veri`, diğer sistemlerde `$XDG_DATA_HOME/TAA/veri` (`XDG_DATA_HOME` yoksa `~/.local/share/TAA/veri`).

**İstisna:** `rehber` alanı mevcut konumunda kalır: `%APPDATA%\TAA\rehber\` (`rehber.json` orada; başka başlığın kuralı). `TAA_VERI_KOK` `rehber` konumunu DEĞİŞTİRMEZ (kod ve `taa-veri` testleri böyle): yalıtım için testler `APPDATA` / `XDG_DATA_HOME` değişkenlerini geçici klasöre yönlendirir. Windows dışı eski konum `~/.config/TAA/rehber` Tutaa tarafından bir kez göçürülür (kopya + `.tasindi`).

Alan adı: `[a-z0-9-]`, en çok 32 karakter. Program adı (`sahip`, `oneren`) aynı kurala uyar.

```
<kok>/<alan>/SAHIP.toml           # sahip kaydı (düz TOML alt kümesi)
<kok>/<alan>/<veri dosyaları>     # yalnız sahip yazar, atomik (geçici dosya + yeniden adlandırma)
<kok>/<alan>/oneriler/<id>.json   # okuyanların değişiklik önerileri (okuyan yalnız buraya yazar)
<kok>/<alan>/oneriler/eski/       # sahibin kapattığı (uygulanan ya da reddedilen) öneriler
<kok>/<alan>/eski/<ad>-<YYYYMMDD-HHMMSS>  # üzerine yazılan SAHIP.toml / taşınan dosyalar; silme yok
```

`SAHIP.toml`, `oneriler` ve `eski` adları alan içinde ayrılmıştır; sahip bu adlarla veri dosyası yazamaz.

### SAHIP.toml biçimi

Düz alt küme: `anahtar = "metin"` | `anahtar = true|false` | `anahtar = 123`. Tablo, dizi, çok satırlı metin yoktur. `#` ile yorum satırı (değerden sonra da yazılabilir). Anahtar tekrarı hatadır. Metinde `\"`, `\\`, `\n`, `\t`, `\r` kaçışları geçerlidir.

```toml
sema = 1
alan = "etiket-kuyrugu"
sahip = "fiyatetiketi"      # program adı (program.toml `ad`)
gecici = false              # true: geçici sahip; asıl sahip kurulunca devralır
yazan_surum = "1.33.0"
son_yazim = "2026-11-05T14:00:00Z"
```

`sema` (şu an 1), `alan` (klasör adıyla aynı) ve `sahip` zorunludur; `gecici` yoksa `false`. Bilinmeyen anahtarlar korunur.

## 2. Kurallar

1. **Tek yazar.** Alan dosyalarını yalnız `sahip` yazar. Okuyan hiçbir zaman alan dosyası yazmaz; değişiklik isterse `oneriler/` altına `{"sema":1,"oneren":"taarkod","zaman":"…","degisiklik":{…}}` bırakır. Sahip öneriyi uygular ya da reddeder; her iki durumda dosyayı `oneriler/eski/`ye taşır. Öneri en çok 256 KiB olabilir.
2. **Açılış.** Sahip alanı açarken: `SAHIP.toml` yoksa yazar; varsa ve `sahip` kendisiyse günceller (`yazan_surum`, `son_yazim`); varsa ve `sahip` başka programsa: `gecici = true` ise **devralır** (eski kayıt `eski/`ye taşınır), değilse **`BaskaSahip` hatası** verir. Program bu durumda alanı salt okunur açar ve kullanıcıya söyler. Geçici sahip olarak açan program başka bir sahibi (geçici de olsa) devralamaz. İki asıl sahip CI'da yasaktır (`tools/sozlesme_denetle.py`).
3. **Sahip yokken.** `SAHIP.toml` var ama sahip program kurulu değilse (taa-kur `kayit.json` içinde yoksa) okuyanlar **son kopyayı salt okunur** kullanır; arayüzde "sahip kurulu değil" notu görünür.
4. **Değişiklik bildirimi.** Sahip yazdıktan sonra `taa-ipc` ile `veri.degisti@1` olayı yayınlar (`yuk: {"alan":…}`). IPC yoksa okuyan dosya zamanına bakar.
5. **Göç (yedekli).** Program ilk açılışta eski konumdaki dosyayı alana **kopyalar**, eskisini `<eski ad>.tasindi` olarak **yeniden adlandırır** (silme yok). Alan yazılamıyorsa eski konumla çalışmaya devam eder ve günlüğe yazar. Göç bir kez yapılır; `.tasindi` varsa tekrarlanmaz.
6. **Kişisel veri.** Kişisel veri alanlarında (`rehber`, `kimlik`) dışa aktarma yetki ister; günlükte maskeleme uygulanır (plan §15).

## 3. Hata adları (`taa-veri`)

| Hata | Anlam |
|---|---|
| `BaskaSahip { alan, sahip }` | Asıl sahip başka program; alan salt okunur açılmalı. |
| `SahipYok` | Okunmak istenen alanda `SAHIP.toml` yok. |
| `GecersizAd(ad)` | Alan ya da program adı `[a-z0-9-]`, 1..=32 kuralına uymuyor. |
| `GecersizToml { satir, neden }` | Bozuk `SAHIP.toml`; `satir` 1'den başlar, satıra bağlanamayan hatalarda (eksik anahtar vb.) 0. |
| `YolDisari(yol)` | Dosya yolu `..`, mutlak yol, `\`, `:` ya da ayrılmış ad içeriyor. |
| `OneriBuyuk(bayt)` | Öneri 256 KiB sınırını aşıyor. |
| `Io(hata)` | Dosya sistemi hatası. |

## 4. Kitaplık API'si (`crates/taa-veri`)

`VeriKok::ortamdan()`, `VeriKok::yeni(yol)`, `kok.ac_sahip(alan, program, surum)`, `kok.ac_gecici_sahip(alan, program, surum)`, `kok.ac_okur(alan, program)`; `Sahip::yaz(dosya, &[u8])` (atomik; `..`/mutlak yol reddi), `Sahip::oku`, `Sahip::oneriler()`, `Sahip::oneri_kapat(id)`; `Okur::oku(dosya)`, `Okur::oner(Value) -> id`, `Okur::sahip() -> SahipKaydi`, `Okur::sahip_kurulu(&kayit_json_yolu)`; `sahip_toml_coz`, `sahip_toml_yaz`. Okuyanın API'sinde alan dosyası yazan işlev yoktur. Kodda silme işlevi kullanılmaz.

## 5. Güvenlik notları

- Alan ve dosya adları yol kaçışına karşı denetlenir; alanın dışına çıkan yol yazılmaz ve okunmaz.
- Öneri dosyaları güvenilmez girdidir: sahip `degisiklik` içeriğini doğrulamadan uygulamaz; boyut sınırı 256 KiB, JSON olmayan ya da zarfı eksik dosyalar listede görünmez.
- `gecici` bayrağı yalnız devralmayı kolaylaştırır; sahte `gecici = true` yazan bir program alanı kaptırabilir. Bu yüzden `docs/veri-sahipleri.toml` tablosu ve CI denetimi asıl sahibi belirler.
