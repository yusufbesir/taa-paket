<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T0 2026-11-05] -->
# Tek çatı (plan §18, depo gerçeğine uyarlanmış)

Kaynak: `PLAN-TAA-PAKET.md` §18 + `docs/ENVANTER.md`. Çelişkide envanter kazanır; her sapmanın nedeni yazılı.

## 1. Kural (değişmedi)
Bir program başka bir programa zorunlu bağımlı olmaz; yalnız ortak çekirdeğe bağlıdır. Programlar arası ilişki ya isteğe bağlı yetenek (`ister`) ya da tek sahipli paylaşılan veridir. Bir program kaldırılınca diğerleri açılır; en çok bir düğme gizlenir veya liste "son kopya, salt okunur" olur.

**Ortak çekirdek (program = ortak):** taa-ayar, taa-dosya, taa-nebim, taa-terzi (T0 kararı: veri yazmaz, saf kural motoru). **TaaFiş ailesi tek program (`taafis`)**: taa-fis, taa-fis-sunucu, taa-bildirim, taa-takip, taa-telegram (T0 kararı; neden: taa-fis-sunucu bunlara sert Cargo bağımlı. TaaTerziTakip'i ayrı program yapmak için T2'de taa-fis-sunucu'da Cargo `features` ile isteğe bağlı hale getirilir, sonra `program` alanı ayrılır). taa-katalog `ortak` kalır ama `urunler` alanının sahibi olduğu için sahiplik programı kayıtta tutulur (açık soru).

## 2. Depodaki 20 crate (plan "21" diyor; 21. bulunamadı)
taa-ayar, taa-dosya, taa-nebim, taa-katalog, taa-donustur, taa-yedek, taa-pdf, taa-rehber, taa-yazici, taa-tara, taa-pano, taa-kasa, taa-fis, taa-fis-sunucu, taa-terzi, taa-kombin, taa-bildirim, taa-takip, taa-telegram, taa-subesor.
Planlı ama yok: `taa-kur` (T1), `taa-ipc` (T1), `taa-kimlik`, `taa-veri` (§18.5'te anılıyor, kod yok).

## 3. Kurulum yöneticisi `taa-kur`
§18.2 geçerli; uyarlamalar:
- Kök sabit `D:\Programlar\TAA\` yerine **çalışma zamanında çözülen kök** (varsayılan `%LOCALAPPDATA%\Programs\TAA\`, kullanıcı değiştirebilir; seçim `kayit.json`'da). Neden: envanter sabit `D:\` yollarını temizlik listesine aldı; Tutaa zaten `%LOCALAPPDATA%\Programs\Tutaa` kullanıyor. (Açık soru: Yusuf `D:\` istiyorsa kayıtta varsayılan değer olur, kodda sabit olmaz.)
- Düzen: `TAA\depo\<ad>\<surum>\`, `TAA\programlar\<ad>\secili.json`, `TAA\eski\`, `TAA\kayit.json` (`taa-dosya::uzerine_yaz_atomik`). Komutlar: `liste | kur | kaldir | sec | geri-don | dogrula | tani`.
- Kaldır = `eski\<ad>-<surum>-<tarih>\` altına taşı; veri alanına dokunulmaz. Silme yok.
- `taa-kur` yokken programlar tek başına çalışır (crate'ler statik bağlı).
- T1'de yapıldı: klasör paketi, SHA256; zip ve imza T2.

## 4. Sözleşme (`modul.toml`, 20 crate)
Mevcut alanlar korunur (`ad, surum, arayuz, gerektirir, ozellikler, program, butce`); eklenir:
```toml
sunar       = ["urunler.ara@1"]      # ad.fiil@N, paket genelinde benzersiz
ister       = ["bildirim.gonder@1"]  # isteğe bağlı; yoksa düğme/uç kapalı
veri_sahibi = ["urunler"]            # yalnız bu crate yazar
veri_okur   = ["nebim-aktarim"]      # salt okunur
ayar_alani  = "moduller.taa-katalog"
```
- `gerektirir`: yalnız `taa-*` crate adı, Cargo.toml `[dependencies]` ile **birebir** (dev-bağımlılık hariç). Envanterdeki üç eksik (taa-fis-sunucu, taa-telegram, taa-yedek) T0'da düzeltilir.
- `ozellikler`: koddaki `OZELLIK` / `ozellik_acik("...")` anahtarlarıyla aynı liste (denetim PLANLI, T1; T0 betiğinde yok).
- CI `tools/sozlesme_denetle.py` (T0'da var): §18.3 (1)-(4) + gerektirir = Cargo `taa-*` bağımlılıkları birebir + ince çizgi (gerektirir hedefi ortak ya da aynı program) + her `veri_sahibi` alanı paket genelinde tek + ad/klasör/Cargo name eşitliği. Yok (planlı): arayuz→DEGISIKLIK "kırıcı" satırı, ozellikler = kod anahtarları.

## 5. Sürüm uyumu
§18.4 geçerli. Durum: 20/20 crate Cargo = modul.toml. Tutaa'da üç farklı sürüm (2.4.5 / 2.5.0 / 2.0.0) → `uyumluluk.toml`'a girmeden tek kaynak seçilmeli. Tüm veri dosyaları şemasız: her alana `SAHIP.toml` ile `sema = 1` başlangıcı; mevcut dosyalar "sema yok = 1" kabul edilir (göç gerekmez).

## 6. Veri sahipliği (envantere göre düzeltilmiş)
Hedef konum: `%APPDATA%\TAA\veri\<alan>\` + `SAHIP.toml`. Bugün tüm crate verisi çağıranın verdiği dizinde (taa-fis-sunucu `--veri`, varsayılan göreli `taa-fis-veri`).

| Alan | Sahip (tek yazar) | Okuyanlar | Bugünkü gerçek yol | Sahip yokken |
|---|---|---|---|---|
| urunler | taa-katalog | (repo içinde yok; TaaRkod/Fiyat Etiketi planlı) | `<yol>/katalog.json`, `<dizin>/eski/` | son kopya salt okunur; Nebim CSV'den yeniden kur |
| nebim-aktarim | taa-nebim (dış kaynak bağdaştırıcısı; dosyayı Nebim yazar) | taa-katalog | `<klasör>/*.csv` | içe aktarma düğmesi gizli |
| fis-defteri | taa-fis | taa-fis-sunucu, taa-telegram, taa-takip | `<veri>/fisler.jsonl` | takip/telegram boş liste |
| cihazlar | taa-fis-sunucu | — | `<veri>/cihazlar.jsonl` | telefon eşleştirme kapalı |
| fis-fotolari | taa-fis-sunucu | — | `<veri>/fotolar/` | foto eki gizli |
| aydinlatma | taa-fis-sunucu | — | `<veri>/aydinlatma.html` (elle) | `--aydinlatma-url` |
| terzi-kurallari | taa-terzi | taa-fis | gömülü `islemler.json` + `<defter dizini>/terzi-kurallari.json` | gömülü varsayılan |
| zorluk | taa-takip | taa-fis-sunucu | `<veri>/zorluk.json` (elle) | gömülü varsayılan |
| bildirim-kuyrugu | taa-bildirim | taa-fis-sunucu | `<veri>/bildirimler.jsonl`, `bildirim-sablonu.txt` | bildirim düğmesi gizli |
| telegram-ayar (sır) | taa-telegram | — | `<veri>/telegram.json` | bot kapalı |
| kasa-defteri | taa-kasa | — | çağıranın `.jsonl` yolu | — |
| subesor-defteri | taa-subesor | — | çağıranın `.jsonl` yolu | — |
| yedekler | taa-yedek | — | `<hedef>/<damga>/` + `manifest.json` | — |
| pano-kartlari | taa-pano (şema sahibi; üretici belirsiz) | — | `<klasör>/*.pano.json` | pano boş |
| rehber | taa-rehber | (Tutaa, TaaDön planlı) | **yok** (yalnız bellekte CSV) | elle yazım |
| belgeler | taa-donustur | (Tutaa planlı) | **yok** (çıktı kullanıcı yolunda) | ek düğmesi gizli |
| ayar-ortak | taa-ayar (geçici; T1'de taa-kur) | tüm crate'ler | `TAA_AYAR` ortamının gösterdiği JSON | varsayılanlar |

§18.5 taslağından sapmalar ve nedenleri:
1. **urunler**: taslak "Tutaa (`taa-veri`)". Envanter: `taa-veri` yok, Tutaa'da merkez API istemcisi bulunamadı; tek yazar taa-katalog. → sahip taa-katalog. Tutaa merkezi gelirse sahiplik devri kayıtla (tek yazar kuralı korunur).
2. **kimlik**: taslakta Tutaa (`taa-kimlik`). Kod yok → bu depoda alan tanımlanmadı; Tutaa sözleşmesine bırakıldı (açık soru).
3. **etiket-kuyrugu**: TaaRkod envanteri gelmedi; bu depoda crate yok → tabloya alınmadı, program.toml (T2) ile girer.
4. **rehber**: sahip taa-rehber doğru, ama `rehber.json` kodda yok; T2'de yazılacak. `%APPDATA%\TAA\rehber` yerine `%APPDATA%\TAA\veri\rehber\` (tek düzen).
5. **belgeler**: taa-donustur kayıt tutmuyor; alan ayrıldı, kod T2.
6. **ayar-ortak**: taslak `taa-kur`; o crate yok, bugün yalnız okuyan taa-ayar var ve yazanı yok. Geçici sahip taa-ayar; T1'de taa-kur'a devir.
7. **Yeni alanlar** (taslakta yoktu, envanterde var): nebim-aktarim, fis-defteri, cihazlar, fis-fotolari, aydinlatma, terzi-kurallari, zorluk, bildirim-kuyrugu, telegram-ayar, kasa-defteri, subesor-defteri, yedekler, pano-kartlari.
8. **olay-gunlugu**: paylaşılan alan değil (her program kendi dosyası) → sahiplik tablosunda yok.

Tek yazar ihlalleri ve çözüm:
- fis-defteri: taa-telegram `durum_ata` ile yazıyor → taa-telegram `veri_okur` + `ister fis.guncelle@1`; yazımı taa-fis API'si yalnız taafis süreci içinde yapar (Telegram webhook zaten taa-fis-sunucu içinde).
- bildirim-kuyrugu: yazım yalnız taa-bildirim `bildirim.gonder@1` üzerinden; tek süreç (taa-fis-sunucu) içinde sıralı; `M-{n}` yerine çakışmasız kimlik (T1).

## 7. İş akışı
§18.6 geçerli. T0 çıktısı: bu iki belge + 20 modul.toml hizalaması + `sozlesme_denetle.py`. Denetim hedefi "20/20" (21 değil, ta ki 21. crate netleşene kadar).

## 9. Güvenlik notları (T0 güvenlik incelemesi, T1'de karşılanacak)
- IPC: adlandırılmış boru ACL'i yalnız geçerli kullanıcı; her istek `taa-kimlik` belirteci ister; `sunar` ucu çağıranın programını ve rolünü doğrular; bütçe/hız sınırı. Kötü niyetli yerel süreç senaryosu T1 tasarımında zorunlu madde.
- taa-kur: `kur`/`sec` yalnız imzalı (ed25519) ve SHA256'sı `yayin.json` ile uyuşan paketi etkinleştirir; `dogrula` = depo içeriğinin SHA256'sını kayıt ile karşılaştırır. `kayit.json`/`secili.json` imzalı özetle korunur (kullanıcı haklarıyla çalışan süreç değiştirirse `dogrula` fark eder, çalıştırma reddedilir).
- Veri alanı: `%APPDATA%\TAA\veri\<alan>` için kilit dosyası + tek yazar; `telegram-ayar` gibi sır içeren alanlar DPAPI ile şifreli; `bildirim-kuyrugu` telefon numarası saklama süresi Yusuf kararı (KVKK).
- CI: `permissions: contents: read` eklendi; action'ların SHA'ya sabitlenmesi T1.
