<!-- Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-27] değiştiren: [Claude#taa-kasa-tk-ek3 2026-11-27]. Telefon Köprüsü onay_iste@1 / onay@1 için gün sonu özetinin kanonik biçimi (SABİT). -->
# kasa.telefon-ozet

- Tür: biçim (Telefon Köprüsü `onay_iste@1` / `onay@1` alanı `ozet`)
- Sürüm: geçerli özet biçimi **`surum=2`** (`telefon_imza::OZET_SURUMU`, 2026-11-27). Sürüm 1 = tam ayrıntı metni; sürüm 2'de
  bu metnin SHA-256'sı `ayrinti_sha256` satırıyla bağlanır. Eski (sürüm 1) özetle imzalanmış kayıtlar merkezde kabul edilir
  (`telefon_imza::ozet_eslesir`).
- Neden: TK Ek 3'te `ozet_metni` en çok 1 KiB, istek en çok 2048 bayt. Sürüm 1 metni kasa noktası, POS ve döviz sayısıyla
  büyür; uydurma ölçümle 4 kasa noktası × 2 POS + 3 döviz + 13 ciro satırında ~1.070 bayt, 4 nokta × 4 POS + 6 dövizde
  ~1.480 bayt (vektör `v4-…`: 1.210 bayt). Sürüm 2 bu sayılardan bağımsızdır (en çok ~600 bayt).
- Tek kaynak: `crates/taa-kasa/src/telefon_imza.rs` → `ozet_metni`, `ozet_sha256` (sürüm 2); `ayrinti_metni`, `ayrinti_sha256` (sürüm 1).
- Çapraz test vektörleri: `crates/taa-kasa/testdata/telefon-ozet-vektorleri.json` (uydurma veri). Rust testi
  `ozet_bicimi_sabit_capraz_test_vektorleri` ve bağımsız bir Python uygulaması aynı sonucu verir.

## Özetlenen baytlar
`ozet = SHA-256(UTF-8(özet metni sürüm 2))`, 32 bayt. Kasa isteğinde (`ozet`, TK sözleşmesi Ek 3) 64 haneli **küçük harf onaltılık** yazılır ve `ozet_metni` ile birlikte gider;
köprü/telefon tarafındaki 32 bayt bu hex'in çözülmüş hâlidir.

## Sürüm 2 (geçerli): `ozet_metni`
JSON değildir; **sabit sırada** 15 satır `anahtar=değer\n` (LF, son satır dahil; CR/BOM yok):

```
kapsam=taa_kasa_gun_sonu
surum=2
sube=<metin>
tarih=<metin>
cekmece=<metin>
id=<metin>
nakit_sayilan_kurus=<tam sayı>
devreden_kurus=<tam sayı>
nebim_toplam_kurus=<tam sayı>
ciro_toplam_kurus=<tam sayı>
z_toplam_kurus=<tüm kasa noktalarının Z toplamı>
pos_toplam_kurus=<tüm noktaların tüm POS toplamı>
nokta_adet=<kasa noktası sayısı>
doviz_adet=<döviz satırı sayısı>
ayrinti_sha256=<sürüm 1 ayrıntı metninin SHA-256'sı, 64 küçük hex>
```

- Metin alanları (`sube`, `tarih`, `cekmece`, `id`): `\n`, `\r` → boşluk; en çok 48 bayt, UTF-8 karakter sınırında kesilir
  (tam değer `ayrinti_sha256` ile bağlıdır).
- Tutarlar kuruş, ondalık tam sayı (eksi için `-`). Kişi adı, not, açıklama girmez.
- `ayrinti_sha256` ciro kalemlerini, dövizleri, nokta/POS ayrıntısını bağlar: telefon okunur toplamları gösterir; tek bir
  kuruş farkı da özeti değiştirir.

## Sürüm 1: ayrıntı metni (`ayrinti_metni`)
Sürüm 2'deki `ayrinti_sha256` bu metnin özetidir; biçimi değişmez. JSON **değildir**; satır satır `anahtar=değer\n` düz metindir:

1. Alanlar (gün sonu kaydı `kasa.gunsonu@1` `gun_sonu` nesnesinden):
   - `kapsam` = `taa_kasa_gun_sonu`, `surum` = `1`
   - `id`, `tarih` (YYYY-AA-GG), `sube`, `cekmece`
   - `nakit_sayilan_kurus`, `devreden_kurus`, `nebim_toplam_kurus`, `ciro_toplam_kurus` (= `karsilastirma.ciro_toplam_kurus`)
   - her ciro alanı: `ciro.<anahtar>` (bilgi satırları dahil, ör. `ciro.degisim`, `ciro.kasa_fazlasi`; eksi olabilir)
   - her döviz: `doviz.<KOD>.tutar`, `doviz.<KOD>.kur_onbinde`
   - her kasa noktası: `nokta.<nokta>.z_toplam_kurus` ve her POS: `nokta.<nokta>.pos.<pos_id>`
2. Değerler: tutarlar kuruş (döviz tutarı yabancı para kuruşu, kur ×10.000) cinsinden **ondalık tam sayı**; eksi için başta `-`,
   baştaki sıfır yok, binlik ayırıcı yok. Metin alanları olduğu gibi (UTF-8).
3. Sıralama: (anahtar, değer) çiftleri **UTF-8 bayt sırasına** göre artan. (Örn. `.` < `_` olduğu için `ciro.*` satırları
   `ciro_toplam_kurus`'tan önce; `nokta.k1.pos.*` satırları `nokta.k1.z_toplam_kurus`'tan önce gelir.)
4. Kaçış (sıralamadan sonra): anahtardaki `\n`, `\r`, `=` → `_`; değerdeki `\n`, `\r` → boşluk. Her satır `\n` (LF) ile biter,
   son satır dahil; CR yok, BOM yok.
5. Kişi adı, serbest not, açıklamalar, kontrol listesi, zaman ve sayım adetleri **girmez**.

## Sürüm 1 örneği (vektör `v1-sade`, uydurma)
```
cekmece=c1
ciro.nakit=20000
ciro.ziraat_pos=10000
ciro_toplam_kurus=30000
devreden_kurus=5000
id=GS-20260115-c1-1
kapsam=taa_kasa_gun_sonu
nakit_sayilan_kurus=30000
nebim_toplam_kurus=30000
nokta.k1.pos.ziraat=10000
nokta.k1.z_toplam_kurus=30000
sube=SB1
surum=1
tarih=2026-01-15
```
SHA-256: `ab1b4529d01f2ba5430d6828e67ad3b7f1bd0b6c9b8029b0ca79f3ca16eae0e0`

Diğer vektörler: `v2-eksi-kredi-ceki-doviz-iki-nokta` → `ca0cd2b1f9485ce8cef96ae611cae598c61aca0d4ef090e3c09e202fc561836c`,
`v3-bos-ciro` → `6a8dafdbc87927a91bc3f65b7c8da02142177da52cf64b8773c450e056dabbf5` (tam girişler JSON dosyasında). Bu sürüm 1
değerleri (`beklenen_metin`, `beklenen_sha256`) korunur.

## Sürüm 2 örneği (vektör `v1-sade`, uydurma)
```
kapsam=taa_kasa_gun_sonu
surum=2
sube=SB1
tarih=2026-01-15
cekmece=c1
id=GS-20260115-c1-1
nakit_sayilan_kurus=30000
devreden_kurus=5000
nebim_toplam_kurus=30000
ciro_toplam_kurus=30000
z_toplam_kurus=30000
pos_toplam_kurus=10000
nokta_adet=1
doviz_adet=0
ayrinti_sha256=ab1b4529d01f2ba5430d6828e67ad3b7f1bd0b6c9b8029b0ca79f3ca16eae0e0
```

## Sürüm 2 vektörleri (`beklenen_metin_v2`, `beklenen_sha256_v2`)
- `v1-sade` → `325dc9c9f261080ef9082bb6691df90d4ac265065744f0403d2458ca7062d470`
- `v2-eksi-kredi-ceki-doviz-iki-nokta` → `b83c1b963620490bb840c05c08cc2ffff8c7d6b440f30bb1660216ad6e5123ac`
- `v3-bos-ciro` → `5b82b61b13e849474c3bd037c59a8defbcc050ed3d32fdf4e12e674c809f9fb1`
- `v4-cok-nokta-cok-doviz-1kib-ustu` (sürüm 1 metni 1.210 bayt, sürüm 2 metni 342 bayt) →
  `13e6b35ca54570606d7acfdedb824e98ffd1030e183daf920ecb21b30047a64a`

## Telefonun doğrulaması
Kasa isteği (`onay_iste@1`) özet metnini de taşır (`ozet_metni`). Telefon metnin SHA-256'sını kendisi hesaplar, `ozet` ile
karşılaştırır ve kullanıcıya metni (ya da `baslik`) gösterir. Kasa onay gelince özeti kayıttan yeniden hesaplar; gün sonu
arada değiştiyse ya da özet tutmazsa imza atılmaz.
