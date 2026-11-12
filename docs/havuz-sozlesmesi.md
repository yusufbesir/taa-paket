<!-- Sahip: Yusuf. yazan: [Claude#ortak-veri-havuzu 2026-11-12] -->
# Ortak veri havuzu sözleşmesi

Plan: `planlar/PLAN-ORTAK-VERI-HAVUZU-2026-11-12.md`. Tür kataloğu: `docs/havuz-turleri.toml`. Denetim: `tools/havuz_denetle.py`.

Havuz, TAA Sunucu PostgreSQL'indeki çok programlı, sürümlü içerik deposudur. Yerel "veri alanları" (`docs/veri-alani.md`: rehber, kimlik, etiket-kuyrugu, cihaz, yol) havuza taşınmaz.

## Görünürlük
Bir kayıt bir programda şu üç koşul birlikte sağlanırsa görünür (sunucu hesaplar):
1. Tür, programın `havuz_okur` listesinde ya da `havuz_ilgi` ailesindedir.
2. Kullanıcı/cihazın Çatı izni `havuz.<tür>.oku` vardır (varsayılan reddet).
3. Kaydın `sube_kapsam` alanı boşsa herkes, doluysa yalnız o şubeler görür.

`X-TAA-Program` yalnız ilgi süzgecidir, yetki değildir.

## program.toml alanları (isteğe bağlı)
| Alan | Anlam |
|---|---|
| `havuz_okur` | Okunan türler, `tür@N` (örn. `"urun.liste@1"`). Her biri katalogda ve türün `okuyanlar` listesinde olmalı. |
| `havuz_ilgi` | Okunan tür aileleri (örn. `"egitim"`); ailedeki yeni türler otomatik görünür. |
| `havuz_yazar` | Yazılan türler; türün `yazanlar` listesinde olmalı. |

## Kurallar
- `kip = "tablo"` türünün tek yazarı vardır; `kip = "dosya"` türünde birden çok yazar olabilir.
- Dosyalar içerik adresli (sha256) depoda durur; PostgreSQL üst veriyi tutar.
- Silme yok; "arşivle" durumu vardır. Kalıcı silme yalnız Yusuf kararıyla.
- Tür adı `aile.ad` (küçük harf, tire serbest); tür sürümü katalogda, `@N` ile istenir. Eski sürüm en az bir sürüm daha yaşar.
- Yeni tür eklemek: `docs/havuz-turleri.toml`'a yazılır, `python3 tools/havuz_denetle.py` temiz çıkmalıdır.

## Denetim
`python3 tools/havuz_denetle.py` kataloğu doğrular; `--program program.toml modul.toml ...` ile programın havuz bildirimlerini katalogla karşılaştırır. Hata varsa çıkış 1.
