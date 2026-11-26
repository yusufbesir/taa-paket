# TAA Kasa — gün sonu programı

Program adı `taakasa`, exe `taa-kasa` (crates/taa-kasa-ekran). Çekirdek: crates/taa-kasa (`gunsonu`, `merkez`,
`xlsx`, `zaman`; eski `Defter` uyumlu). Fiyat Etiketi'ndeki "Kasa" ekranıyla ilgisi yoktur.

Tek yazar TAA Kasa'dır (`kasa.gunsonu@1`, `kasa.masraf@1`); Tutaa yalnız okur. Sözleşmeler:
docs/sozlesmeler/kasa.gunsonu.md, docs/sozlesmeler/kasa.masraf.md.

## Akış

1. **Açılış sayımı** (çekmece başına, günde bir). Beklenen = önceki gün sonunun devreden tutarı.
2. **Gün içi:** ara sayım ve vardiya devri (ikinci kişi zorunlu, giren olamaz), masraf girişi.
3. **Gün sonu sihirbazı (5 adım):** nakit sayımı → ödeme türleri (nakit, Ziraat kart, İyzico fiziksel/sanal, QR,
   mail order, havale) → döviz → Z raporu / POS / Nebim → kontrol listesi. Sağda canlı fark paneli.
4. **İkinci kişi onayı** ("giren onaylayamaz").
5. **Gönder:** paket (imzalı, cihaz anahtarı varsa) kuyruğa yazılır, merkeze teslim edilir; ağ yoksa kuyrukta bekler.

Fark varsa açıklama zorunlu; nakit ya da Z↔ciro tutmazsa "kasa ile ciro uyuşmuyor" uyarısı çıkar.

## Kurallar

- Kayıt yalnız eklenir (JSONL). Silme yok; düzeltme = yönetici iptali (gerekçeli) + `duzeltir` izli yeni kayıt.
- Masraf: eşik (varsayılan 2.000 TL) altını müdür, üstünü yönetici (merkez) onaylar; giren onaylayamaz.
- İptal, düzeltme, ayarlar, Excel ve matris yalnız yönetici girişiyle.

## Yer tutucular (henüz bağlanmadı)

| Konu | Şu an | Bağlanınca |
|---|---|---|
| Personel girişi | Ayarlar'daki listeden ad seçilir, PIN yok (ekranda açıkça yazar) | KM2 personel PIN'i / ortak giriş (`taa-giris.rs`) |
| Yönetici girişi | Geçici yerel PIN (tuzlu SHA-256 ×100.000, 5 hata → 5 dk kilit, oturum 10 dk) | KM1 `kimlik.yonetici@1` / Çatı PIN doğrulaması |
| Mağaza imzası | Cihaz anahtarı (`cihaz/cihaz.key`) varsa onunla, yoksa imzasız | Mağaza anahtarı |
| Merkeze teslim | Ayarlardaki `merkez_klasoru`na dosya kopyası | Sunucu kasa ucu (Ş3) |
| Fiş fotoğrafı | Yalnız dosya yolu | Dosya ekleme |
| Çatı paneli | Ayrı pencere | Çatı sekmesi |

Ortak giriş neden bağlanmadı: depoda `taa-fis-sunucu/src/ortak_giris.rs` (taa-giris.rs v1) var, ama sunucuda
`taakasa` program izni tanımlı değil (giriş `program_kapali` döner) ve rol eşlemesi (personel/müdür/yönetici)
henüz kararlaştırılmadı. Bağlanması ayrı iş olarak bırakıldı.

## Açık sorular

- Akşam Excel örneği: alanlar örnekle birebir eşleştirilecek (iskelet plandan kuruldu).
- Masraf türü listesi (şimdilik örnek liste), kasa limiti, fark toleransı (varsayılan 0), gönderim son saati
  (varsayılan 23:00).
- Mağaza imza anahtarı ve sunucu kasa ucu.

## Tasarım

Arayüz önerisi bir dış tasarım modelinden (yalnız sırsız genel özet gönderilerek) alındı: aşamaya göre değişen tek
ana düğme, 5 adımlı sihirbaz + yapışkan fark paneli, numpad odaklı giriş, zümrüt paleti, farkın renk + simge + işaretle
gösterimi (✓ ▲ ▼), "Sil" sözcüğünün hiç kullanılmaması.

## Çalıştırma

`taa-kasa [--kok <veri kökü>] [--pencere-acma]`. Veri kökü: `TAA_VERI_KOK`, yoksa `%APPDATA%\TAA\veri`.
Kayıtlar `<kök>/kasa/` altında (`gunsonu.jsonl`, `masraf.jsonl`, `ayar.json`, `giden/`, `gonderilen/`).
