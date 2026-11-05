<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05]. T3 "imza ve güncelleme" tasarımı (kısa). -->
# T3 tasarımı: ed25519 imza, güvenilir anahtarlar, güncelleme

Kaynak: docs/TEK-CATI.md §9, docs/T1-TASARIM.md, docs/T2-TASARIM.md. Yusuf onaylı yeni bağımlılıklar: `ed25519-dalek 2` (`default-features=false`, `std`) ve `toml` (yalnız `parse`). Başka bağımlılık, HTTP ve silme yok (`eski\` kuralı sürer). Ağ kaynağı T4'e bırakıldı.

## 1. `taa-imza` (program = ortak)
- **Biçimler:** genel anahtar dosyası `ed25519:<64 hane hex>` (`#` yorum satırı serbest); özel anahtar dosyası `ed25519-ozel:<64 hane hex>`; imza `{"alg":"ed25519","anahtar_kimlik":<genel anahtarın ilk 8 baytı, 16 hane>,"deger":<128 hane hex>}`. Kimlik yalnız eşleştirme içindir; güvenlik tam anahtar doğrulamasından gelir (`verify_strict`).
- **Özel anahtar:** yalnız çağıranın verdiği dosyadan okunur (`--ozel <dosya>`); depoya, günlüğe, hata iletisine, `Debug` çıktısına yazılmaz. `uret` dosyayı yeni oluşturur (var olanı ezmez), Unix'te 0600; okurken başkalarının erişebildiği dosya için uyarı verilir (Windows'ta izin denetlenemez, genel uyarı). Rastgelelik ek crate olmadan: Unix `/dev/urandom`, Windows `RtlGenRandom`.
- **CLI:** `taa-imza uret --ozel <dosya> --genel <dosya>` · `taa-imza imzala <dosya> --ozel <dosya> [--tur ham|paket|yayin] [--cikti <dosya>]` · `taa-imza dogrula <dosya> --genel <dosya> [--tur ...] [--imza <dosya>]`. `paket`: `icerik_sha256` metnini imzalar, `imza` alanını yerinde yazar (eski dosya `<dosya>.once-N` kopyası). `yayin`: `imza` hariç kanonik JSON (anahtarlar sıralı, boşluksuz) üzerinde. `ham`: dosya baytları, imza JSON'u stdout ya da `--cikti`.
- **Zip paketi doğrudan imzalama (2026-11-05):** `taa-imza imzala <paket.zip> --ozel k` (`.zip` → `--tur paket` varsayılır): kökteki `paket.json` zip içinde yerinde imzalanır, öteki girdiler bayt bayt kopyalanır, eski zip `<zip>.once-N` olarak yanında kalır (silinmez). `taa-imza dogrula <paket.zip> --genel g` de zipten okur. Argümansız/`yardim`/`--help` yardım basar (çıkış 0). Windows'ta özel anahtar dosyasının izni denetlenmez ve uyarı basılmaz (yardım metninde yazar).
- **Zip paket imzalama akışı (paket_uret ile, eski yol):** `tools/paket_uret.py ... --zip x.zip` (imzasız) → `icerik_sha256` metni dosyaya (sonda satır sonu olmadan) → `taa-imza imzala icerik.txt --ozel k --cikti imza.json` → aynı komut `--imza-dosya imza.json` ile yeniden (zip deterministik). `paket_uret.py` imza alanını kendisi yazmaz.

## 2. taa-kur: güvenilir anahtar kuralı
`Kur::yeni(kok)` / `Kur::ortamdan()` `<kök>\anahtarlar\*.pub` dosyalarını güvenilir anahtar olarak okur.
| Durum | Sonuç |
|---|---|
| güvenilir anahtar var, paket imzası geçerli | kurulur |
| anahtar var, imzasız / `null` | `KurHata::Dogrulama` (reddedilir) |
| anahtar var, imza bozuk, biçim yanlış, algoritma ≠ ed25519 | `KurHata::Dogrulama` |
| anahtar var, imza başka (güvenilmeyen) anahtarla ya da kimlik taklidi | `KurHata::Dogrulama` |
| hiç anahtar yok | eski davranış: imza yok sayılır; `tani` çıktısında uyarı (`uyarilar`) |
| `.pub` dosyası bozuk | imza denetimi KAPANMAZ: `kur` `KurHata::Bozuk`, `tani` sorun bildirir |
İmzalanan veri `icerik_sha256` metnidir (dosya listesini ve SHA256'ları kapsar). Doğrulama `kur` içinde, hiçbir şey yazılmadan önce yapılır. `Kur::imza_denetle` aynı kuralı `yayin.json` için de sunar. `tani --json`: `uyarilar`, `guvenilir_anahtarlar` alanları eklendi.

## 3. `taa-guncelle` (kütüphane + CLI)
- **`yayin.json`** (PLAN-TAA-PAKET §16-17 özeti): `{"sema":1,"paketler":[{ad,surum,kanal,arayuz,gerektirir,tam:{url,sha256,boyut},yamalar:[{kaynak_surum,url,sha256,boyut}],en_az_surum}],"imza":{..}}`. Doğrulama: ad/sürüm/kanal kuralları, sha256 hex, boyut 1..2 GiB, tekrar yok, `en_az_surum ≤ surum`. `yamalar` şemada var, T3'te kullanılmaz (her zaman `tam`).
- **`Kaynak` trait'i:** `yayin_oku()`, `getir(url, hedef, beklenen_boyut)`. `YerelKaynak` klasörden okur; `url` klasöre göreli, yol kaçışı/mutlak yol/sembolik bağ reddedilir, `hedef` yeni dosya olarak yazılır, gerçek uzunluk beklenenden büyükse durur. HTTP kaynağı T4'te aynı trait'i uygular.
- **`kontrol`:** yalnız kurulu ve seçili paketler; kanalın en yüksek sürümü. Yayındaki sürüm > kurulu → güncellenebilir (`zorunlu` = kurulu < `en_az_surum`); eşit → güncel; **düşük → reddedilen (geri alma koruması)**. `yayin.json` imzası `taa-kur` ile aynı kurala bağlı (anahtar varsa zorunlu).
- **`uygula`:** paket başına indir (`<kök>\indirilen\`) → boyut + SHA256 → `Kur::kur(--sec)` (paket imzası burada) → `dogrula` ve ad/sürüm eşleşmesi. Herhangi bir adım seçimi değiştirdikten sonra başarısız olursa önceki sürüme `geri_don`/`sec` ile dönülür (`geri_donuldu`). Sabitli paket atlanır, `zorunlu` ise uygulanır. İndirilen dosya `eski\indirilen-*` altına taşınır, silinmez. Bir paketin hatası diğerlerini durdurmaz; en az bir hata varsa çıkış 1.
- **CLI:** `taa-guncelle kontrol|uygula [--json] [--kaynak <klasor>] [--kanal <ad>]`; kaynak/kanal yoksa `kur.toml`'dan.

### 3.1 Yayın üretimi (`tools/yayin_uret.py`, T4)
`python3 tools/yayin_uret.py --cikti yayin.json [--kanal kararli] [--taban-url <url>] [--onceki eski.json] [--en-az-surum ad=x.y.z] <paket.zip> ...`:
her zip'in kökteki `paket.json`'undan ad/sürüm/arayüz/gerektirir, dosyadan SHA256 ve boyut alınır; `url` zip adı (tabana göreli) ya da `--taban-url` ile mutlak.
`--onceki` ile eski kayıtlar korunur, aynı (ad, sürüm, kanal) değiştirilir. Çıktı imzasızdır; `taa-imza imzala yayin.json --ozel <anahtar> --tur yayin` ile imzalanır.
İmzasız zip uyarı verir (`--imzali-zorunlu` ile hata). Var olan çıktı `.once-N` olarak saklanır, silinmez. Yayın akışı: zip'leri `taa-imza` ile imzala → `yayin_uret.py` → `taa-imza imzala --tur yayin` → zip'ler + `yayin.json` aynı klasöre/sürüme (GitHub Releases: `kaynak_url = .../releases/latest/download`).

## 4. `kur.toml` (taa-kur `ayar` modülü, `toml` crate)
`<kök>\kur.toml`: `kaynak_klasoru = "D:/Kaynak"`, `kanal = "kararli"` (varsayılan). Dosya yoksa varsayılanlar; bozuk TOML ya da geçersiz değer tek cümlelik Türkçe hata (`KurHata::Dogrulama`, satır numarası ile). Bilinmeyen anahtarlar yok sayılır.

## 5. Sınırlar ve riskler
Özel anahtar yönetimi (yedek, döndürme, iptal listesi) kapsam dışı; anahtar döndürme `anahtarlar\` klasörüne yeni `.pub` eklemekle yapılır. `kayit.json`/`secili.json` imzası (TEK-CATI §9) bu aşamada yok: iç `ozet` bütünlük denetimi sürer. Windows rastgelelik yolu (`RtlGenRandom`) bu ortamda derlenip çalıştırılamadı. `yayin.json` tazelik/yeniden oynatma koruması (sürüm düşürme dışında, ör. süre sonu) T4.
