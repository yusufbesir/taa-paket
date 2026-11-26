<!-- Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-26]. Yusuf'un kararı (2026-11-26): veri %APPDATA%\TAA\veri'de kalır, sunucuya ayrıca şifreli yedek gider. -->
# kasa.yedek

- Tür: veri türü (şifreli yedek + manifest)
- Sürüm: @1
- Sunan / tek yazar: **TAA Kasa** (`taakasa`; `crates/taa-kasa-ekran/src/yedek.rs`).
- Okuyan: sunucu yedek ucu (henüz yok; `SunucuYok` yer tutucu). Geri yükleme `yedek_coz` ile.
- Tetik: her `/api/gonder` sonrası ve `POST /api/yedek` (elle). Durum: `GET /api/yedek`.
- Kuyruk: `<veri kökü>/kasa/yedek/giden/kasa-yedek-<unix sn>-<sıra>.taay` + aynı adlı `.json` manifest. Önce şifreli dosya,
  sonra manifest yazılır (manifest varsa çift tamdır). Teslim edilen çift `gonderilen`e taşınır; **hiçbir şey silinmez**.
  Çevrimdışıyken yedekler kuyrukta bekler.

## Şifreli dosya (`.taay`)
taa-yedek "TAAY1" biçimi (XChaCha20-Poly1305 akışı, 64 KiB parça, son parça işaretli). Anahtar `<veri kökü>/cihaz/yedek.key`
(yoksa üretilir, üzerine yazılmaz; yedeğe ve günlüğe yazılmaz). Düz içerik yalnız bellekte üretilir:
`{"tur":"kasa.yedek.icerik@1","sube","olusturma","dosyalar":[{"ad","sha256","icerik"}]}`; kasa klasörünün üst düzey
`.json`/`.jsonl` dosyaları, **`yonetici-pin.json` hariç**.

## Manifest
`{"tur":"kasa.yedek@1","surum":1,"sube","olusturma","dosya","boyut","sha256","sifre":"taa-yedek-taay1-xchacha20poly1305","dosyalar":[{"ad","boyut","sha256"}]}`

- `sha256`/`boyut`: şifreli dosyanın. Gönderimden önce yeniden doğrulanır; tutmayan gönderilmez, yerinde kalır.
- `dosyalar[].sha256`: düz dosyanın; çözümden sonra karşılaştırılır.

## Hata kodları
Yedek alınamaz ya da gönderilemezse gönderim yine tamamlanır; yanıtta `yedek.hata` döner.

## Açık
Anahtar emaneti (anahtar kaybolursa sunucudaki yedek çözülemez), sunucu ucu ve saklama süresi kararı.

## Sürüm kuralı
Değişirse `@2` yeni dosya; eski `@1` en az bir sürüm daha yaşar (§18.3).
