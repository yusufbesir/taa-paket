<!-- Sahip: Yusuf. yazan: [Claude#kurucu-arayuz 2026-11-17] -->
# TAA Kurucu temaları (`crates/taa-kur-tema`)

Bağlayıcı tasarım kararı: `arayuz-modernlesme/taa-kurucu/ekip/KARAR-FABLE.md`. Logo kararı:
`logo/LOGO-KARAR-FABLE.md` §2 (Geçme Temel). Bu crate yalnız veri ve hesap taşır, çizim yapmaz;
`taa-kur` ve `kurulum/` GDI ile çizer. Bağımlılığı yoktur (yalnız std), Linux'ta test edilir.
`kurulum/` ayrı çalışma alanı olduğu için path bağımlılığıyla kullanır; bu yüzden `Cargo.toml`
`workspace.*` alanı kullanmaz.

## Temalar

| Tema | Ruh | Hareket | Tanıtım kartı |
|---|---|---|---|
| **İlmek** (varsayılan) | Sıcak krem + mürdüm, yuvarlak, ferah | 2 adımlı renk geçişi | 6 sn |
| **Kalıp** | Soğuk mavi-gri, kompakt, numaralı ray | 3 adımlı 8 px oturma | 7,2 sn |
| **Mezura** | Siyah/beyaz + mezura sarısı, büyük yazı, AAA | Anlık | 8,4 sn |

Her temanın açık ve koyu paleti vardır. Varsayılan mod bilerek **Açık**'tır (`Sistemi izle` değil).
Windows Yüksek Karşıtlık açıksa ve kullanıcı tema seçmemişse tema Mezura olur.

## Jetonlar nerede

- `src/lib.rs`: `ILMEK_ACIK`, `ILMEK_KOYU`, `KALIP_ACIK`, `KALIP_KOYU`, `MEZURA_ACIK`, `MEZURA_KOYU`
  (on bir jeton: zemin, yuzey, yuzey2, kenar, metin, soluk, vurgu, vurgu_ustu, basari, uyari, hata).
  Renkler `0xRRGGBB`; GDI için `colorref()` (BGR).
- `src/olcu.rs`: `Geometri` (yarıçap, kenar, odak, boşluk ölçeği, satır/düğme yüksekliği, düğme
  genişliği hesabı), `Tipografi` (rol başına px + ağırlık), `Hareket`, `TIK_MS` (120),
  `YUKLEME_TIK` (10).
- Kullanılacak renkler her zaman `etkin_jetonlar(&gorunum, sistem_koyu, yuksek_karsitlik)` ile alınır;
  sabitlere doğrudan gitmeyin (özel vurgu ve Yüksek Karşıtlık o işlevde uygulanır).

## Tercih dosyası

`<veri kökü>\kurucu\gorunum.json`; veri kökü `TAA_VERI_KOK`, yoksa `%APPDATA%\TAA\veri`
(taa-yol / taa-cihaz ile aynı kural). Kayıt defteri kullanılmaz.

```json
{
  "surum": 1,
  "tema": "ilmek",
  "mod": "acik",
  "ozel_vurgu": {"ilmek": "#6A2C5A", "kalip": null, "mezura": null},
  "hareketi_azalt": null,
  "tanitim_goruldu": true
}
```

- `tema: null` = kullanıcı seçmedi. `hareketi_azalt: null` = Windows "animasyonları göster"
  (`SPI_GETCLIENTAREAANIMATION`) izlenir.
- Okuma hoşgörülüdür: dosya yok ya da bozuksa varsayılanlar, bilinmeyen anahtar yok sayılır, hata
  gösterilmez. Yazma atomiktir (geçici dosya + yeniden adlandırma).
- `varsayilana_don()` yalnız görünüm alanlarını sıfırlar; `tanitim_goruldu` korunur.
- Yönetici olarak başka hesapla yükseltilmiş süreçte `%APPDATA%` yanlış profile gider: etkileşimli
  kullanıcının yolunu bulup `yukle_yoldan` / `kaydet_yola` kullanın.

## Kontrast koruması

- Testler her palette `metin`, `soluk`, `basari`, `uyari`, `hata` için üç yüzeyde (zemin, yuzey,
  yuzey2) >= 4,5:1, `kenar` için >= 3:1, Mezura'da metin için >= 7:1 doğrular.
- Özel vurgu (`vurgu_duzelt`): `vurgu_ustu` siyah ya da beyazdan kontrastı yüksek olan
  (Mezura'da >= 7:1 şart). Vurgu `yuzey` üstünde >= 3:1 vermeli; vermiyorsa `metin` yönüne %4'lük
  adımlarla en çok 12 kez kaydırılır, yine olmazsa reddedilir ve temanın rengi kalır. Düzeltildiyse
  arayüz "Okunabilirlik için renk biraz koyulaştırıldı." notunu gösterir.
- Mezura açıkta sarı hiçbir zaman metin ya da tek başına gösterge olmaz; hep 2 px `kenar`
  çerçeveyle dolgu olarak çizilir (`Tema::vurgu_cerceveli`). Bu yüzden orada 3:1 gösterge kuralı
  atlanır.
- `hataya_yakin`: seçilen renk `hata` tonuna 20°'den yakınsa uyarı notu (rengi reddetmez).
- `vurgu_metin_olabilir`: vurgu renkli bağlantı 4,5:1 vermiyorsa `metin` + alt çizgi kullanılır.

## Logo (Geçme Temel)

`src/logo.rs`: blok T (`ANA_PATH`, ana renk) alttaki U yuvaya (`VURGU_PATH`, vurgu) iner; ikisi
değmez. `cokgenler(boyut_px, x, y)` GDI `Polygon` için tamsayı köşeler verir; `SVG` tek dosya simge
metni, `svg(ana, vurgu)` renkleri sabit sürüm. Kurallar: T hiçbir zaman vurgu rengiyle boyanmaz;
Mezura sarısı açık zemine doğrudan basılmaz (koyu karo üstünde ya da tümü siyah). Diskteki `.svg`
dosyalarında c2pa metadata olduğu için kaynak olarak bu path'ler kullanılır.

## Yeni tema eklemek

1. `Tema` sayısına yeni değer ekleyin; `HEPSI`, `ad`, `gorunen_ad`, `sira`, `hareket`,
   `tanitim_ms`, `tanitim_adim` eşleşmelerini tamamlayın (derleyici eksikleri gösterir).
2. `lib.rs`'e açık ve koyu `Jetonlar` sabitlerini, `olcu.rs`'e `geometri` ve `tipografi` dallarını
   ekleyin.
3. `Gorunum::ozel_vurgu` dizisinin boyunu tema sayısına çıkarın.
4. `cargo test -p taa-kur-tema`: kontrast testleri yeni paleti otomatik denetler; geçmezse renkleri
   düzeltin, testi gevşetmeyin.
5. Önce KARAR belgesine yazılır, sonra koda; bu belgedeki tabloyu güncelleyin.

## HD logo ve simgeler (2026-11-17, Yusuf: "logo piksel piksel duruyor")
[Claude#kurucu-arayuz coPLkm]
- `taa_kur_tema::simge::tara`: Geçme Temel kenar yumuşatmalı taranır (16×16 alt örnek; 256 px'te 4×4). 48 px altında köşeler piksele oturtulur (keskin), üstünde gerçek oran + yumuşak kenar. Çıktı önceden çarpılmış 32 bit BGRA; pencerelerde `AlphaBlend` ile basılır (kurulum `hd.rs`, taa-kur `win_hd.rs`). Pencere her karede çizdiği için `tara_onbellekli`.
- Pencere simgeleri alfa kanallı ve sistemin istediği boyutta üretilir (`GetSystemMetricsForDpi`: %150'de 48/24 px), ekran ölçeği değişince (`WM_DPICHANGED`) yeniden üretilir. taa-kur penceresinin önceden simgesi yoktu.
- İki pencere de monitör başına DPI farkındalığı v2 bildirir (yoksa sistem DPI'si): ölçek değişince Windows pencereyi bitmap olarak büyütüp bulanıklaştırmaz.
- Exe dosya simgesi: `build.rs` (kurulum ve taa-kur) `simge::rsrc_nesnesi` ile 16, 20, 24, 32, 40, 48, 64, 256 px'lik RT_ICON/RT_GROUP_ICON içeren `.rsrc` COFF nesnesi üretip bağlayıcıya verir (rc.exe / windres gerekmez). Gezgin, Başlat menüsü kısayolu ve görev çubuğu bu simgeyi kullanır.
