<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05] -->
# T3: taa-kur penceresi ("bu program ne?")

`taa-kur-pencere` (Windows): kurulu paketleri gösterir, Kur / Kaldır / Güncelle düğmeleri verir. Komut satırı `taa-kur` değişmedi; pencere aynı `taa_kur::Kur` kitaplığını çağırır.

## Ekran düzeni
```
+--------------------------------------------------------------------+
| TAA Kur — paketler                                          [_][□][x]|
+--------------------------------------------------------------------+
| Paket        | Kurulu sürüm  | Seçili sürüm | Durum                 |
|--------------+---------------+--------------+-----------------------|
| taa-ayar     | 0.1.0         | 0.1.0        | Hazır                 |
| taa-ornek    | 0.1.0, 0.2.0  | 0.1.0        | Daha yeni sürüm kurulu|
|              |               |              |  (0.2.0), seçili değil|
+--------------------------------------------------------------------+
| [ Kur... ]  [ Kaldır ]  [ Güncelle ]                               |
| Kuruldu: taa-ornek 0.2.0 (seçili değil).   [Kök: D:\Programlar\TAA]|
+--------------------------------------------------------------------+
```
Durum metinleri: "Hazır", "Hazır (sabit)", "Seçili sürüm yok", "Kurulu değil", "Daha yeni sürüm kurulu (X), seçili değil".

## Düğme kuralları
| Düğme | Etkin olduğu durum | İş |
|---|---|---|
| Kur... | her zaman | dosya seçici (`paket.zip` ya da klasör paketinin `paket.json` dosyası) -> `Kur::kur(yol, false)` |
| Kaldır | seçili satırın seçili sürümü varsa | onay sorusu -> `Kur::kaldir(ad, None)`; içerik silinmez, `eski\` klasörüne taşınır; veri alanlarına dokunulmaz |
| Güncelle | bir satır seçiliyse | onay sorusu → `GuncelleyiciArayuzu::guncelle`. `KomutGuncelleyici`: `taa-guncelle uygula --json --paket <ad>` (TAA_KOK kök; konsol penceresi açılmaz); komut exe'nin yanında ya da depodaki seçili `taa-guncelle` paketinde aranır. Bulunamazsa `KaynaksizGuncelleyici`: "Güncelleme kaynağı ayarlanmadı" |

Durum satırı her işlemin sonucunu (ya da hata + öneriyi) Türkçe tek satır yazar; hata varsa kayıt değişmez.

## Mimari
- `src/pencere_model.rs`: platformdan bağımsız (komut -> `Kur`, satır modeli, düğme kuralları, metinler, `GuncelleyiciArayuzu`). Linux'ta birim testli (geçici kök, gerçek `Kur`).
- `src/win.rs` (`#[cfg(windows)]`): ince Win32 katmanı, `windows-sys` doğrudan (ListView, düğmeler, `GetOpenFileNameW`, `MessageBoxW`). Yeni GUI bağımlılığı yok.
- `src/pencere_main.rs`: bin `taa-kur-pencere`; Windows dışında "Bu pencere yalnız Windows'ta çalışır" deyip 1 ile çıkar. `main.rs` değişmedi.
- Güncelleyici: `win::calistir` `KomutGuncelleyici::bul` ile taa-guncelle komutunu arar; JSON çıktısı `guncelle_ciktisi_yorumla` ile tek satır Türkçe mesaja çevrilir (güncellendi / güncel / atlandı / reddedildi / hata + önceki sürüm seçili). Kaynak `kur.toml` (`kaynak_url` ya da `kaynak_klasoru`) ile ayarlanır.

## Sınırlar
- Windows'ta elle doğrulanmadı; yalnız `cargo check/clippy --target x86_64-pc-windows-gnu` tür denetimi yapıldı.
- İşlemler arayüz iş parçacığında eşzamanlı (büyük zip kurulurken pencere kısa süre yanıtsız kalabilir).
- Klasör seçici yok; klasör paketi için `paket.json` seçilir.
