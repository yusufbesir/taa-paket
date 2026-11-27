<!-- Sahip: Yusuf. yazan: [Claude#taa-kasa-tk-sozlesme 2026-11-27]. Telefon Köprüsü (TK) boru kabulü için TAA paket bildirimi, imza, açık anahtar ve makine geneli kök kaydı. Kaynaktan doğrulandı. -->
# TK için paket doğrulama: bildirim, imza, açık anahtar, kök

TK hizmeti borudan bağlanan istemciyi fail-closed kabul eder: bildirim yoksa red. Aşağıdakiler depodaki koddan
doğrulandı. Kaynaklar: `crates/taa-kur/src/{lib.rs,paket.rs}`, `crates/taa-imza/src/lib.rs`, `tools/paket_uret.py`,
`kurulum/src/{main.rs,makine_kok.rs}`.

## (a) Paket bildirimi ve imza

- **Dosya:** `paket.json` (UTF-8 JSON, tek nesne), sürüm klasörünün kökünde.
- **Alanlar:** `sema`, `ad`, `surum`, `arayuz`, `program`, `gerektirir`, `sunar`, `ister`, `veri_sahibi`, `veri_okur`,
  `dosyalar`, `icerik_sha256`, `imza`.
- **Exe özeti:** `dosyalar[]` içindeki `{ "yol": "bin/taa-kasa.exe", "sha256": "<64 hane küçük hex>" }` kaydı. Değer exe
  baytlarının SHA-256'sıdır. `yol` paket köküne görelidir, ayırıcısı `/`.
- **İçerik özeti:** `icerik_sha256` şöyle hesaplanır:
  1. `dosyalar[]` kayıtları `yol`'a göre UTF-8 bayt sırasına dizilir.
  2. Her kayıt için `yol + "\n" + sha256 + "\n"` yan yana eklenir.
  3. Bu baytların SHA-256'sı alınır ve küçük hex yazılır.

  Kaynak: `paket.rs` `icerik_hesapla`, `tools/paket_uret.py`.
- **İmza:** ayrı bir imza dosyası YOKTUR. İmza `paket.json` içindeki `imza` alanında durur:
  `{ "alg": "ed25519", "anahtar_kimlik": "<16 hane hex>", "deger": "<128 hane hex>" }`.
  - İmzalanan veri `icerik_sha256` metninin UTF-8 baytlarıdır (64 ASCII karakter). JSON'un kendisi imzalanmaz.
  - `anahtar_kimlik`, genel anahtarın ilk 8 baytının hex'idir. Anahtar seçmek için kullanılır, güven tam anahtardan gelir.
  - Doğrulama ed25519 `verify_strict` ile yapılır (taa-imza `GenelAnahtar::dogrula`).
- **Doğrulama sırası (TK için önerilen):**
  1. `paket.json` okunur.
  2. `imza.alg == "ed25519"` olmalı; `anahtar_kimlik` ile sabitlenmiş anahtar bulunur.
  3. İmza `icerik_sha256` metni üzerinde doğrulanır.
  4. `dosyalar[]` üzerinden `icerik_sha256` yeniden hesaplanır ve eşit olmalı.
  5. Borunun öbür ucundaki sürecin exe'si okunur, SHA-256'sı `bin/taa-kasa.exe` kaydıyla eşit olmalı.
  6. Exe yolu `<kök>\depo\taa-kasa-ekran\<sürüm>\bin\taa-kasa.exe` olmalı.
- **TAA açık anahtarı:**
  - Biçim: tek satır `ed25519:<64 hane hex>` (32 bayt); `#` ile başlayan satırlar yorumdur.
  - Yer: taa-kur güvendiği anahtarları `<kök>\anahtarlar\*.pub` dosyalarından okur. Kurulum bu dosyaları yalnız yoksa yazar.
  - **Üretim anahtarı henüz yok.** Depoda yalnız test/geliştirme anahtarları üretilir.
  - **TK için öneri:** kökteki `anahtarlar\` klasörüne güvenmeyin. O klasör kökle aynı yazma iznini taşır ve saldırgan kendi
    anahtarını koyabilir. Üretim açık anahtarını TK hizmetine sabitleyin (derleme sabiti ya da hizmetin kendi korumalı
    ayarı). Anahtar çıkınca `ed25519:<hex>` metni ayrıca iletilecek.
- **Authenticode yok.** exe kod imzalı değil. Kaldırma kaydındaki yayıncı adı `TAA`.

## (b) Kurulumdan sonra bildirim ve imza yerinde mi

Evet, ikisi de yerinde kalır:

- `taa-kur kur` paketi hazırlık klasörüne kopyalar (`kopyala`) ve `<kök>\depo\<ad>\<sürüm>\` adıyla yerine taşır.
- `paket.json` (imza içinde) bayt bayt aynı kalır; her dosyanın sha256'sı kopyalanırken denetlenir.
- Sürüm klasöründe yalnız `paket.json` ve `bin\` bulunur.
- Bunu test ediyor: `crates/taa-kur/src/testler_imza.rs` → `kurulu_surum_klasorunde_imzali_paket_json_kalir`. Test şunları
  denetler:
  - Bayt eşitliği.
  - `bin/taa-kasa.exe` sha256'sının kurulu exe ile eşleşmesi.
  - İmzanın güvenilir anahtarla doğrulanması.
  - Ayrı imza dosyası olmaması.
- Seçili sürüm `<kök>\kayit.json` içinde `paketler.<ad>.secili` alanındadır. Eski sürümler `depo\` altında yan yana durabilir.

## (c) Makine geneli kök (TK `TAA_KOK`'u yok sayar)

Kurulum (`kurulum/src/makine_kok.rs`, `main.rs` `makine_kok_yaz`) iki yere yazar.

**1. `HKLM\SOFTWARE\TAA\Kur` (güvenilir kaynak).** Bu anahtarı yalnız yönetici yazabilir. Değerlerin hepsi REG_SZ:

| Değer | Örnek |
|---|---|
| `Kok` | `D:\Programlar\TAA` |
| `KayitJson` | `D:\Programlar\TAA\kayit.json` |
| `Depo` | `D:\Programlar\TAA\depo` |
| `Sema` | `1` |

**2. `%ProgramData%\TAA\kur-kok.json` (yalnız ipucu).** Aynı bilgileri `sema`, `kok`, `kayit_json`, `depo`,
`kurulum_surumu` ve `guvenilir_kaynak` alanlarıyla taşır. ProgramData'ya sıradan kullanıcı da yazabilir, bu yüzden
güvenlik kararı bu dosyaya dayanmamalı.

Davranış:

- Kayıt `--kisayolsuz` iken de yazılır.
- HKLM yazılamazsa (kurulum yönetici olarak çalışmadıysa) kurulum durmaz. Uyarı verir: "Kurulumu bir kez sağ tık →
  Yönetici olarak çalıştır ile yeniden çalıştırın". TK o zamana kadar kökü bulamaz, yani red (fail-closed).
- Kaldırmada HKLM anahtarı yalnız `Kok` bu kurulumun köküyse silinir; ipucu dosyası da yalnız kendi kökünü gösteriyorsa silinir.
- Bu değişiklik Windows'ta denenmedi. Linux'ta saf mantığın birim testleri var (`makine_kok::testler`) ve Windows hedefi için
  clippy temiz.
- Varsayılan kök dışındaysa kullanıcı ortamına `TAA_KOK` yazılmaya devam eder. Bu taa-kur içindir, TK onu kullanmaz.

## Kısa yanıt metni (TK'ya)

> (a) Bildirim, sürüm klasöründeki `paket.json` dosyasıdır. Exe özeti `dosyalar[]` içinde `yol: "bin/taa-kasa.exe"` kaydının
> `sha256` alanındadır (SHA-256, küçük hex). Ayrı imza dosyası yok. İmza `paket.json` → `imza` alanında:
> `{alg:"ed25519", anahtar_kimlik:16 hex (genel anahtarın ilk 8 baytı), deger:128 hex}`. İmzalanan veri `icerik_sha256`
> metnidir; bu, sıralı `yol\nsha256\n` satırlarının SHA-256'sıdır. Doğrulama `verify_strict` ile. Açık anahtar biçimi
> `ed25519:<64 hex>`; taa-kur `<kök>\anahtarlar\*.pub` dosyalarını okur. Üretim anahtarı henüz yok. TK anahtarı kendisi
> sabitlemeli, kökteki klasöre güvenmemeli. Authenticode yok.
> (b) Evet, `paket.json` (imzasıyla) `<kök>\depo\taa-kasa-ekran\<sürüm>\` altında bayt bayt aynı kalır. Testle doğrulandı.
> (c) Kurulum `HKLM\SOFTWARE\TAA\Kur` altına `Kok`, `KayitJson`, `Depo` ve `Sema` değerlerini yazar (yönetici gerekir;
> yetki yoksa uyarı verir). Ayrıca ipucu olarak `%ProgramData%\TAA\kur-kok.json` yazar. TK yalnız HKLM'ye güvenmeli.
