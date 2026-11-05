<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05] -->
# taa-ipc: programlar arası iletişim (T1)

Kaynak tasarım: `docs/T1-TASARIM.md` §2. Crate: `crates/taa-ipc` (program = "ortak", gerektirir = taa-dosya). Yetenekler programlara aittir; taa-ipc yalnız taşır.

## Kısa kullanım
```rust
use serde_json::json;
use taa_ipc::{GelenKutusu, Tasiyici, Zarf};

let kutu = GelenKutusu::ortamdan()?;             // TAA_IPC_KOK -> APPDATA/TAA/ipc -> ~/.local/share/TAA/ipc
kutu.kayit_ol("taarkod", &["kod.uret@1".to_string()])?;   // sunan program

// İstemci (tutaa)
let hedef = kutu.yetenek_bul("kod.uret@1")?;     // Ok(None) ise kabuk düğmeyi gizler; Err(Belirsiz) ise kullanıcıya sorar
let istek = Zarf::istek("tutaa", "taarkod", "kod.uret@1", json!({"urun": 7}));
let id = kutu.gonder(&istek)?;                   // yetenek yoksa Err(YetenekYok)

// Sunucu (taarkod)
for z in kutu.al("taarkod") { kutu.yanitla(&z, json!({"kod": "ABC"}))?; }

// İstemci yanıtı alır (bir kez)
let yanit = kutu.yanit_al("tutaa", &id);

// Olay: yayın ve abone (abone son okuduğu id'yi kendisi tutar)
kutu.olay_yayinla("taarkod", "veri.degisti@1", json!({}))?;
let yeniler = kutu.olaylar("tutaa", &son_id);    // kendi yayınladıkları gelmez
```
`Tasiyici` trait'i taşıyıcıdan bağımsızdır; T2'de adlandırılmış boru aynı trait'le gelecektir.

## Dosya düzeni
```
<kok>/kayit/<program>.json          # program, yetenekler, pid, zaman (atomik yazılır)
<kok>/<program>/gelen/<id>.json     # bekleyen istek
<kok>/<program>/yanit/<id>.json     # isteği atan programa gelen yanıt (ad = istek id'si)
<kok>/<program>/islenen/<id>.json   # yanıtlanan istek / alınan yanıt (silinmez, taşınır)
<kok>/<program>/islenen/bozuk/      # bozuk zarf dosyaları (bir kez günlüğe düşer, yeniden teslim edilmez)
<kok>/olaylar/<YYYYMMDD>.jsonl      # eklemeli olay günlüğü
<kok>/gunluk.jsonl                  # atlanan bozuk dosya/satır kayıtları
```
Zarf: `{"v":1,"id":"<pid>-<sayac>-<nanos>","kaynak","hedef","tur":"istek|yanit|olay","yetenek","yuk","zaman":"RFC3339 UTC","yanit_id"}`. Olayda `hedef` = `herkes`, `yetenek` = olay adı.

## Sınırlar
- Zarf en çok 1 MiB: gönderimde `ZarfBuyuk` döner ve dosya yazılmaz; okumada büyük dosya atlanır.
- Adlar (yol kaçışı engeli; nokta ve `@` program adında yoktur):
  - `program_adi_gecerli`: `[a-z0-9-]`, harfle başlar, en çok 32 karakter. `kayit`, `olaylar`, `gunluk`, `islenen`, `gelen`, `yanit` program adı olarak ayrılmıştır.
  - `yetenek_adi_gecerli`: `ad.fiil@N` (`ad`/`fiil`: `[a-z0-9-]`, harfle başlar; `N`: rakam), en çok 64 karakter. Olay adları da bu biçimdedir.
  - `kimlik_gecerli` (zarf `id`, `yanit_id`): `[a-z0-9-]`, harf/rakamla başlar, en çok 64 karakter.
- Çift dosya yok: `gonder` ve `yanitla` hedef dosya zaten varsa yazmaz, `IdCakismasi` döner ("(1)" eki alınmaz). Yanıtlanamayan istek `gelen/` altında kalır.
- `al` yalnız adı `<id>.json` olan ve `zarf.id` dosya adına eşit, türü `istek`, hedefi o program olan dosyaları teslim eder; uyuşmazsa günlüğe yazılır ve `islenen/` altına taşınır.
- `yanit_al` yanıtı yalnız şu koşullarda kabul eder: tür `yanit`, hedef = alan program, `yanit_id` = istek kimliği ve yanıtın `kaynak`ı = isteğin `hedef`i (istek hedefin `gelen/` ya da `islenen/` klasöründe aranır). Aksi halde `Gecersiz` sayılır, günlüğe yazılır, dosya `islenen/` altına taşınır ve `None` döner.
- Bozuk JSON, geçersiz alan ya da sınır aşımı: `al` içinde dosya `islenen/bozuk/` altına taşınır, bir kez `gunluk.jsonl`'a yazılır, okuma sürer; dosya silinmez.
- `kayit_ol`: kayıt aynı program için başka bir pid taşıyorsa ve zamanı 10 dakikadan yeniyse üzerine yazmaz, `KayitDolu` döner; bayat (>= 10 dk) ya da okunamayan kayıt üzerine yazılır. Aynı pid kendi kaydını yenileyebilir.
- `yetenek_bul`: yetenek kayıtlı değilse `Ok(None)`; tek program sunuyorsa onu döner; birden çok program sunuyorsa HİÇBİRİNİ seçmez, `Err(Belirsiz { yetenek, programlar })` döner (kabuk kullanıcıya sorar).
- `gonder`: hedef yeteneği sunmuyorsa `YetenekYok`.
- `olaylar`: yalnız son 7 günün (bugün dahil) `olaylar/<YYYYMMDD>.jsonl` dosyalarını okur. `son` boşsa bu pencerenin tamamı, `son` bulunamazsa yalnız bugünün dosyası döner. Her dosyadan en çok 10.000 satır okunur (en yeni satırlar tutulur); fazlası kesilir ve `olay_satir_siniri` olarak `gunluk.jsonl`'a yazılır. Bozuk satır atlanır.
- Yoklama tabanlıdır (dosya taraması); bildirim yoktur.
- Eşzamanlı yazım güvenlidir: id'ler süreç içinde sayaçla, süreçler arası pid ile ayrışır; yeni dosyalar geçici dosya + sert bağlantıyla (varsa ezmeden) yerleştirilir, kayıt `taa_dosya::uzerine_yaz_atomik` ile yazılır.

## Güvenlik sınırı
Dosya tabanlı taşıyıcı, aynı kullanıcı haklarıyla çalışan süreçlere karşı kimlik DOĞRULAMAZ. Aynı kullanıcı altındaki herhangi bir süreç `kayit/`, `gelen/` ya da `yanit/` altına dosya yazabilir; yukarıdaki denetimler (ad biçimi, çift dosya reddi, yanıt kaynağı/kimlik eşleşmesi, kayıt tazeliği, belirsiz yetenek) yanlışlıkla çakışmayı ve basit sahteciliği zorlaştırır ama kötü niyetli bir yerel süreci durdurmaz. Bu sınır T2'de adlandırılmış boru (`Boru`) ve `taa-kimlik` belirteciyle kapanır.

## T2: Boru
Adlandırılmış boru (`Boru`, `interprocess` ya da `windows-sys`) aynı `Tasiyici` trait'iyle eklenecek; bağımlılık Yusuf onayı bekler (T1-TASARIM §4, soru 2). Dosya tabanlı `GelenKutusu` yedek yol olarak kalır.

## Test
`cargo test -p taa-ipc` (birim: `src/lib.rs`; bütünleşik senaryolar: `tests/senaryo.rs`).
