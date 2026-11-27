<!-- Sahip: Yusuf. yazan: [Claude#taa-kasa-KS2 2026-11-27]. Telefon Köprüsü onay_iste@1 / onay@1 için gün sonu özetinin kanonik biçimi (SABİT). -->
# kasa.telefon-ozet

- Tür: biçim (Telefon Köprüsü `onay_iste@1` / `onay@1` alanı `ozet`)
- Sürüm: özet biçimi `surum=1` (`telefon_imza::OZET_SURUMU`). Biçim değişirse sürüm artar; eski sürüm en az bir sürüm daha kabul edilir.
- Tek kaynak: `crates/taa-kasa/src/telefon_imza.rs` → `ozet_metni`, `ozet_sha256`.
- Çapraz test vektörleri: `crates/taa-kasa/testdata/telefon-ozet-vektorleri.json` (uydurma veri). Rust testi
  `ozet_bicimi_sabit_capraz_test_vektorleri` ve bağımsız bir Python uygulaması aynı sonucu verir.

## Özetlenen baytlar
`ozet = SHA-256(UTF-8(özet metni))`, 32 bayt. Kasa isteğinde (`ozet`, TK sözleşmesi Ek 2) 64 haneli **küçük harf onaltılık** yazılır ve `ozet_metni` ile birlikte gider;
köprü/telefon tarafındaki 32 bayt bu hex'in çözülmüş hâlidir.

Özet metni JSON **değildir**; satır satır `anahtar=değer\n` düz metindir:

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

## Örnek (vektör `v1-sade`, uydurma)
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
`v3-bos-ciro` → `6a8dafdbc87927a91bc3f65b7c8da02142177da52cf64b8773c450e056dabbf5` (tam girişler JSON dosyasında).

## Telefonun doğrulaması
Kasa isteği (`onay_iste@1`) özet metnini de taşır (`ozet_metni`). Telefon metnin SHA-256'sını kendisi hesaplar, `ozet` ile
karşılaştırır ve kullanıcıya metni (ya da `baslik`) gösterir. Kasa onay gelince özeti kayıttan yeniden hesaplar; gün sonu
arada değiştiyse ya da özet tutmazsa imza atılmaz.
