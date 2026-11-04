<!-- TAA modül sözleşmesi belgesi. Sahip: Yusuf. yazan: [Claude#taa-paket-A0 2026-11-04]. Değişiklik: Yusuf'un onayıyla. -->
# modul.toml sözleşmesi

Her TAA modülü (crate veya program bileşeni) kök klasöründe bir `modul.toml` taşır.

| Alan | Tür | Açıklama |
|---|---|---|
| `ad` | metin | Modül adı (crate adıyla aynı). |
| `surum` | metin | SemVer; Cargo.toml `version` ile aynı tutulur. |
| `arayuz` | metin | Arayüz ana sürümü ("1", "2" ...). Modülün dışa sunduğu sözleşmenin sürümü. |
| `gerektirir` | tablo | Gerekli modüller: `{ "taa-dosya" = "1" }` (ad = en düşük arayüz ana sürümü). Boşsa `{}`. |
| `ozellikler` | liste | Modülün sunduğu yetenek adları (örn. `["arsiv", "imza"]`). |
| `program` | metin | Bağlı program (örn. `taadon`); birden çok programca paylaşılıyorsa `ortak`. |
| `butce` | tablo | İsteğe bağlı performans bütçeleri (örn. `{ acilis_ms = 800 }`). |

## Uyumluluk kuralı

- `arayuz` ana sürümü değişmedikçe eski ve yeni modül sürümleri birlikte çalışmak zorundadır
  (yalnız ekleme; var olan davranış/alan kaldırılmaz veya anlamı değiştirilmez).
- Uyumsuz değişiklik `arayuz` ana sürümünü artırır; bu durumda eski ve yeni arayüz bir süre
  yan yana sunulur, tüketiciler kendi hızında geçer.
- `surum` her yayında artar; `arayuz` yalnız uyumsuz değişimde artar.

## Örnek

```toml
ad = "taa-dosya"
surum = "0.1.0"
arayuz = "1"
gerektirir = {}
ozellikler = ["bicim", "atomik-yazma", "gecici"]
program = "ortak"
```
