<!-- Sahip: Yusuf. yazan: [Claude#taa-kasa-KS6 2026-11-26]. KS6 sözleşmesi (ilk sürüm). -->
# kasa.masraf

- Tür: veri türü ve yetenek
- Sürüm: @1
- Sunan / tek yazar: **TAA Kasa** (`taakasa`). Okuyan: TAA Çatı (RP8 finans kuyruğu, ayrı adım). Okuyanlar yazmaz.
- Veri alanı: `kasa-masraf` (`<veri kökü>/kasa/masraf.jsonl`, yalnız eklemeli).

## Kayıt türleri
| tur | Alanlar |
|---|---|
| `masraf` | `id` (`MS-YYYYAAGG-<çekmece>-<sıra>`), `tarih`, `cekmece`, `kisi`, `tur` (mağaza ayarındaki listeden), `tutar_kurus` (> 0; tutarı olmayan kalem reddedilir), `aciklama` (kalem adı, ör. Ekmek), `not` (isteğe bağlı: nereye/kime; eski kayıtlarda yok), `foto` (isteğe bağlı, veri kökü altında göreli ad), `onay_duzeyi` (mudur, merkez), `zaman`, `duzeltir` |
| `karar` | `id`, `kisi`, `onay` (bool), `not` (retçe zorunlu), `zaman` |
| `iptal` | `id`, `neden`, `kisi`, `zaman` (yalnız yönetici) |

## Kurallar
- Eşik: `masraf_merkez_esigi_kurus` (varsayılan 2.000 TL). Altı → mağaza müdürü (ya da yönetici) onaylar; eşik ve üstü → yalnız merkez (yönetici).
- **Giren onaylayamaz** (müdür olsa bile). Bir masrafa tek karar verilir.
- Bekleyen ve onaylı masraflar gün sonu genel toplamına ve ciro Excel'inin masraf satırlarına girer (her kalem ayrı satır: "kalem (not)"; en az 9 satır, kalem arttıkça satır eklenir, alttaki satırlar ve formüller kayar); WhatsApp özetinde her kalem tutarıyla ayrı yazılır; reddedilen ve iptal edilen girmez.
- Masraf türleri listesi Yusuf'tan beklenir (G6); bugünkü liste yer tutucudur (Kırtasiye, Temizlik, Kargo, Yemek, Diğer).
- Fiş fotoğrafı alanı sözleşmede var; ekranda dosya ekleme henüz bağlanmadı.

## Sürüm kuralı
Değişirse `@2` yeni dosya; eski `@1` en az bir sürüm daha yaşar.
