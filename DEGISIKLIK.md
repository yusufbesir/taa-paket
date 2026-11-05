<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T0 2026-11-05] -->
# Değişiklik günlüğü

## Yayınlanmamış

### T3-pencere

- `taa-kur-pencere` (yeni bin) ve `taa_kur::pencere_model`: paket listesi (ad, kurulu sürüm, seçili sürüm, durum), Kur / Kaldır / Güncelle düğmeleri ve alt durum satırı. Mantık platformdan bağımsız modelde (7 birim test, gerçek `Kur`, geçici kök); Win32 çizimi `#[cfg(windows)] win.rs` içinde ince katman.
- Güncelle düğmesi `GuncelleyiciArayuzu` trait'ine bağlı; varsayılan "Güncelleme kaynağı ayarlanmadı" der (taa-guncelle başka dalda).
- Kaldırma silmez: `Kur::kaldir` ile `eski\` klasörüne taşır; veri alanlarına dokunulmaz.
- Bağımlılık: yalnız Windows'ta `windows-sys 0.61` (Yusuf onaylı; `zip` zaten dolaylı olarak aynı sürümü çekiyordu), asgari özellikler. Windows'ta elle doğrulanmadı; tür denetimi `--target x86_64-pc-windows-gnu` ile yapıldı. `docs/T3-PENCERE.md` eklendi.
### T3-imza
- Yusuf onaylı yeni bağımlılıklar: `ed25519-dalek 2` (`default-features=false`, `std`) ve `toml` (`parse`). Başka bağımlılık, HTTP ve silme yok.
- `crates/taa-imza` eklendi: ed25519 anahtar üret/imzala/doğrula, hex, `ed25519:<hex>` genel anahtar dosyası, `taa-imza uret|imzala|dogrula` CLI. Özel anahtar yalnız verilen dosyadan okunur (0600, ezmez, günlüğe/`Debug`'a yazılmaz).
- `crates/taa-kur`: `paket.json` `imza` = `{alg, anahtar_kimlik, deger}` (imzalanan veri `icerik_sha256`). `Kur::yeni/ortamdan` `<kök>\anahtarlar\*.pub` okur; anahtar varken imzasız/yanlış imzalı paket `KurHata::Dogrulama` ile reddedilir, anahtar yokken eski davranış (+ `tani` uyarısı). Bozuk `.pub` imza denetimini kapatmaz. `Kur::imza_denetle`, `tani` alanları `uyarilar` ve `guvenilir_anahtarlar`. `kur.toml` (`kaynak_klasoru`, `kanal`) `toml` ile okunur (`kur_ayari_oku`).
- `crates/taa-guncelle` eklendi: `yayin.json` şeması, `Kaynak` trait'i + `YerelKaynak`, `kontrol` (zorunlu `en_az_surum`, geri alma koruması), `uygula` (SHA256 + imza, `kur --sec`, hatada önceki sürüme dönme; yama yalnız şemada, `tam`a düşülür), `taa-guncelle kontrol|uygula [--json]`.
- `tools/paket_uret.py` artık `imza` alanı yazmaz; hazır imza `--imza-dosya` ile alınır. `docs/T3-TASARIM.md` eklendi.
### T3-boru
- `taa-ipc`: Windows adlandırılmış boru taşıyıcısı (`BoruTasiyici`, `#[cfg(windows)]` `src/boru.rs`; `Tasiyici` trait'i, dosya tabanlı `GelenKutusu`'na yedekli). Yusuf onaylı tek yeni bağımlılık `windows-sys` (yalnız `[target.'cfg(windows)'.dependencies]`). Platformdan bağımsız parça `src/boru_ortak.rs` (boru adı `\\.\pipe\taa-<kullanici>-<program>`, 4 bayt uzunluk + JSON çerçeve, 256 KiB sınır, zaman aşımı, ACL = yalnız geçerli kullanıcı, `FIRST_PIPE_INSTANCE`, `YedekliTasiyici`) Linux'ta test edilir. `docs/ipc.md` "T3: Windows adlandırılmış boru" bölümü eklendi. DOĞRULANMADI: Windows kodu yalnız `cargo check --target x86_64-pc-windows-gnu` ile tür denetiminden geçti, Windows'ta çalıştırılmadı.

### T2-A4 (güvenlik sertleştirme; Opus incelemesi, 0 yüksek / 9 orta / 7 düşük)
- `taa-veri.js`: `eskiyeTasi` dışa açıldı (programlar bozuk alan dosyasını `<alan>/eski/` altına alır). `docs/veri-alani.md`: `rehber` + `TAA_VERI_KOK` kuralı kodla hizalandı.

- `araclar/js/taa-veri.js` v2: `'wx'` + `fsync` ile atomik yazım; ilk `SAHIP.toml` yazımı yarışsız (`wx`, yeniden okuma); `oneriler()` yalnız düz dosya, ≤256 KiB, zarf doğrulaması (`sema`, `oneren`, `degisiklik`), kimlik dosya adından (yük içindeki `id` yok sayılır); `altYol` ayrılmış üst adları harfe duyarsız, Windows aygıt adlarını, sonda nokta/boşluğu ve `:` reddeder; `oneriKapat`/`eskiyeTasi` hedef adı `wx` ile ayırır (ezme yok); bilinmeyen `SAHIP.toml` anahtarları korunur; `goc` `ekler` (ör. `.bak` da `.tasindi`). Programlar kopyayı güncellemeli.
- `crates/taa-veri`: ilk `SAHIP.toml` `create_new` ile (yarışta yeniden okur); `yol_denetle` harfe duyarsız ayrılmış adlar, Windows aygıt adları, sonda nokta/boşluk, denetim karakterleri.
- `crates/taa-kur`: `yol_dogrula` Windows aygıt adları ve sonda nokta/boşluk reddi; zip yinelenen ad denetimi harfe duyarsız; `kur_zip` geçici klasörü `create_dir` ile alır (boş ve yarışsız).
- Açık (T3): sahip tablosunun çalışma anında kitaplığa gömülmesi (sahte `gecici`/sahte asıl sahip), alan içi sembolik bağ denetimi, SAHIP.toml çözücüleri için ortak test kümesi, `eski/` boyut uyarısı (`docs/T2-TASARIM.md` §6).


### T2-A3

- `tools/sozlesme_denetle.py` yeni program modu: `--program <program.toml> [--surum-dosyasi <package.json|Cargo.toml>] [--paket-kok <taa-paket>]` (zorunlu alanlar, ad ve alan adı biçimi, `gerektirir` yalnız `taa-*`, `sunar`/`ister` biçimi ve tekilliği, sürüm eşitliği, `ayar_alani`, `docs/veri-sahipleri.toml` ile sahip/okur çapraz denetimi; `[veri.<alan>] gecici = true` geçici sahip uyarı verir). Mevcut paket denetimi değişmedi; 13 yeni test.
- `docs/veri-sahipleri.toml` (paket genelinde veri alanı sahip tablosu) eklendi.
- `docs/program-sozlesmesi.md`: "Taslak örnekler" yerine dört programın kesin değerleri (tutaa, taadon, taarkod, fiyatetiketi), taarehber "ayrı başlık, taslak"; geçici sahip notasyonu belgelendi.
- `araclar/js/taa-veri.js` (bağımlılıksız CommonJS veri alanı kitaplığı, programlar kopyalar) ve `araclar/js/test/taa-veri.test.js` eklendi; CI'ya `js` işi (Node 22, `node --test`) eklendi.

### T2-A1

- `crates/taa-veri` eklendi (program = ortak, gerektirir = taa-dosya; yeni Cargo bağımlılığı yok): veri alanı sahipliği (`ac_sahip`, `ac_gecici_sahip`, `ac_okur`), atomik yazım, öneri kutusu (`oner`, `oneriler`, `oneri_kapat`), `SAHIP.toml` düz TOML alt kümesi çözücü/yazıcı (hata satır numaralı). Kodda silme yok; devralınan `SAHIP.toml` ve kapatılan öneriler `eski/` altına taşınır.
- `docs/veri-alani.md` (veri alanı sözleşmesi: kök, SAHIP.toml, 6 kural, hata adları, öneri biçimi, göç kuralı) ve `docs/veri-sahipleri.toml` (paket genelinde sahip tablosu) eklendi.
- README crate tablosunda `taa-veri` "var"; `docs/TEK-CATI.md` §3'e T2 durum notu eklendi.

T1 (kurulum yöneticisi ve iletişim; yeni Cargo bağımlılığı yok).

- `taa-kur` (klasör paketi kurulumu, sürüm seçme, geri dönme, SHA256 doğrulama; zip ve imza T2) ve `taa-ipc` (dosya tabanlı gelen kutusu taşıyıcısı) crate'leri eklendi.
- `tools/paket_uret.py` (+ testi) eklendi: `modul.toml` + Cargo sürümü -> `paket.json`; `icerik_sha256` tanımı `paket.json`'a `icerik_sha256_tanim` olarak yazılır ve `docs/T1-TASARIM.md` §1.2'de kesinleştirildi.
- `tools/sozlesme_denetle.py`: iki yeni denetim (hata): `gerektirir` grafında döngü; istenen arayüz ana sürümü hedef modülün `arayuz` değerine eşit olmalı. Testler eklendi.
- README crate tablosu: `taa-kur` ve `taa-ipc` "var".
- `docs/TEK-CATI.md` §3'e T1 durum notu eklendi.

### T2-A2

- `taa-kur kur <paket.zip>`: bağımsız değişken `.zip` ile bitiyorsa zip `<kök>/gecici-<pid>/` altına açılır (yol kuralları klasör paketiyle aynı; en çok 10.000 girdi, açılmış toplam en çok 2 GiB; bildirilen boyutlar toplanıp sınır aşılırsa hiçbir şey açılmaz; sembolik bağ ve yinelenen ad reddedilir; `paket.json` zipin kökünde olmalı), sonra mevcut klasör kurulumu çalışır; geçici klasör iş bitince (hata dahil) `eski/` altına taşınır, silinmez. `zip` bağımlılığı `taa-donustur` ile aynı sürüm ve özelliklerle (yeni sürüm ya da özellik yok).
- `paket.json` şemasına ayrılmış `imza` alanı (`null`; Rust yok sayar, doğrulamaz; T3 imza). `tools/paket_uret.py --zip <cikti.zip>` aynı içeriği zip olarak üretir; testler eklendi.
- `taa-kur tani --json`: `veri_kok` ve `veri_alanlari` (`TAA_VERI_KOK` ya da platform varsayılanı altındaki her `<alan>/SAHIP.toml` için `{alan, sahip, gecici, sahip_kurulu}`); bozuk SAHIP.toml tanı `sorunlar` listesine girer. SAHIP.toml düz TOML alt kümesi taa-kur içindeki küçük yerel çözücüyle okunur (`taa-veri`'ye bağımlılık yok).
- `taa-katalog`: `kaydet` artık `fs::remove_file` kullanmaz; eski dosya `eski/<epoch>-<ad>` olarak `rename` ile taşınır (olmazsa kopyalanır ve eski dosya `<ad>.tasindi` adını alır); `eski/` altındaki dosya hiçbir zaman ezilmez. Test eklendi.

T0 (tek çatı, veri ve yapı temizliği). Kırıcı değişiklik yok; yalnız ek alanlar, belge ve denetim.

- 20 `modul.toml` dosyasına `sunar`, `ister`, `veri_sahibi`, `veri_okur`, `ayar_alani` alanları eklendi (mevcut alanlar korundu).
- `tools/sozlesme_denetle.py` (+ testi) eklendi; CI'ya sözleşme denetimi adımı eklendi.
- `docs/ENVANTER.md` (veri konumları, ayar dosyaları, sürümler) eklendi.
- `docs/TEK-CATI.md` (plan §18'in depo gerçeğine uyarlanmış hali) ve `docs/program-sozlesmesi.md` (program.toml taslağı) eklendi.
- `docs/sozlesmeler/` altında yetenek sözleşmeleri eklendi (bilinmeyen alanlar "belirsiz").
- `uyumluluk.toml` (test edilmiş küme taslağı) eklendi.
- Karar: `taa-terzi` program=ortak; `taa-bildirim`, `taa-takip`, `taa-telegram` program=taafis (TaaFiş ailesi tek program). Ayrı TaaTerziTakip programı T2'de Cargo features ile.
- Denetim betiğine: gerektirir = Cargo bağımlılıkları birebir; ince çizgi (başka programın crate'ine zorunlu bağımlılık hata). CI'ya `permissions: contents: read`.
- `docs/modul-sozlesmesi.md` yeni alanlarla güncellendi; README "Modül sözleşmesi" bölümüne bağlantılar eklendi.
