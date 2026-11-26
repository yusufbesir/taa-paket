<!-- Sahip: Yusuf. yazan: [Claude#taa-kasa-KS1 2026-11-26]. KS1 sözleşmesi; ciro alanları akşam ciro şablonuna (V2) birebir uyarlandı. -->
# kasa.gunsonu

- Tür: veri türü ve yetenek
- Sürüm: @1
- Sunan / tek yazar: **TAA Kasa** (`taakasa`; çekirdek `crates/taa-kasa/src/gunsonu.rs`). Başka program yazmaz.
- Okuyan: Tutaa (T6b kasa çift doğrulama tutanağı, T10 açılış/devir tutanağı), TAA Çatı (RP6 evrak matrisi). Okuyanlar yalnız okur;
  TAA Kasa kurulu değilse Tutaa'da bu tutanaklar görünmez, öteki tutanaklar etkilenmez (ince çizgi).
- Veri alanı: `kasa-gunsonu` (`<veri kökü>/kasa/gunsonu.jsonl`, yalnız eklemeli JSONL). Eski `kasa-defteri` (TaaKasaDefteri) aynen sürer.
- Ana plan §15 ve mağaza işleyişi planı §4.1'deki `ciro.sablon@1` adı bu türün ciro bölümüdür; ayrı bir tür açılmadı.

## Kavramlar
- **Çekmece** (`cekmece`): paranın durduğu yer. Açılış, ara sayım, devir ve gün sonu nakit sayımı çekmece başınadır.
- **Kasa noktası** (`nokta`): bilgisayar + yazarkasa + POS cihazları. Z raporu ve POS gün sonu kasa noktası başınadır.
  Bir çekmeceye birden çok kasa noktası bağlanır (Esenyurt: 2 nokta, 1 çekmece). Mağaza toplamı çekmecelerden kendiliğinden toplanır.

## Kayıt türleri (JSONL satırı: `{"tur": ..., "v": ...}`)
| tur | Alanlar | Not |
|---|---|---|
| `sayim` | `id` (`AC-`/`AS-`/`DV-` + `YYYYAAGG-<çekmece>-<sıra>`), `tur` (acilis, ara, devir), `tarih`, `cekmece`, `kisi`, `ikinci_kisi` (ikinci sayan ya da devralan), `adetler` {kuruş: adet}, `sayilan_kurus`, `beklenen_kurus`, `fark_kurus`, `limit_asildi`, `not`, `zaman`, `duzeltir` | Açılışta beklenen = önceki gün sonunun `devreden_kurus`'u. Ara sayım ve devir iki kişilidir. |
| `gun_sonu` | `id` (`GS-...`), `tarih`, `sube`, `cekmece`, `kisi`, `nakit_adetler`, `nakit_sayilan_kurus`, `noktalar` [{`nokta`, `z_toplam_kurus`, `pos` {pos_id: kuruş}}], `nebim_toplam_kurus`, `ciro` {alan: kuruş}, `doviz` [{`kod`, `tutar` (yabancı para kuruşu), `kur_onbinde`}], `devreden_kurus`, `kontrol` [7 × bool], `not`, `karsilastirma`, `zaman`, `duzeltir` | `ciro` alanları akşam ciro şablonunun satırlarıdır. Genel toplama girenler: `nakit` (NAKİT TL), `ziraat_pos` (ZİRAAT BANKASI POS:1), `iyzico_pos` (İYZİCO POS:2), `efatura_pos` (E-FATURA POS:2), `havale` (BANKAYA HAVALE, firma hesabı), `iyzico_odemesi`, `kredi_ceki`, `internet_degisimi`, `kasa_acigi` (KASA AÇIĞI). Bilgi satırları (toplama girmez, eksi olabilir): `kredi_karti_fark`, `kasa_fazlasi`, `havale_fark`, `degisim`, `iade`. Döviz: USD, EUR, GBP, CHF (kur elle). |
| `onay` | `id` (gün sonu), `kisi`, `onay` (ikinci_kisi, ertesi_gun), `zaman` | Giren onaylayamaz. Ertesi gün gözden geçirmesi müdür ya da yönetici. |
| `iptal` | `id`, `neden`, `kisi`, `zaman` | Yalnız yönetici girişiyle; neden zorunlu. **Silme yok.** |
| `gonderim` | `id`, `dosya`, `imzali`, `zaman` | Paket kuyruğa alındı. |

## Karşılaştırma (`karsilastirma`)
Satırlar (her biri `beklenen_kurus`, `bulunan_kurus`, `fark_kurus = bulunan − beklenen`):
1. Nakit: sayılan ↔ açılış + NAKİT TL. (Şablonda NAKİT TL masraflar çıktıktan sonra kalan nakittir; masraf genel toplama ayrıca eklenir.)
2. Z raporu toplamı (tüm kasa noktaları) ↔ genel toplam: şablonun son GENEL TOPLAM'ına çekmecenin payı (genel toplama giren alanlar + döviz TL karşılığı + çekmecenin masrafları). `ciro_toplam_kurus` budur.
3. Her POS ödeme türü için: POS gün sonları toplamı ↔ o türün cirosu (Ziraat ve İyzico ayrı satır).
4. Satış sistemi (Nebim) mağaza raporu toplamı ↔ genel toplam.

`fark_var`: herhangi bir satır mağaza toleransının (`fark_toleransi_kurus`, varsayılan 0) dışında; o zaman `not` zorunludur.
`kasa_ciro_uyusmuyor`: 1. ya da 2. satır tolerans dışında; Çatı'daki "kasa ile ciro uyuşmuyor" uyarısı budur.

## Kurallar
- Aynı gün ve çekmece için tek açılış ve tek gün sonu; ikincisi reddedilir, düzeltme yönetici girişiyle "iptal et ve yeniden gir"
  (eski kayıt `iptal` satırıyla kapanır, yeni kayıt `duzeltir` alanında eskisini gösterir). Geçersiz düzeltme eski kaydı iptal etmez.
- Gönderim ön koşulu: ikinci kişi onayı ve kapanış listesinin 7 maddesi.

## Gönderim paketi (merkeze)
`{"tur":"kasa.gunsonu@1","surum":1,"sube","tarih","cekmece","gun_sonu","sayimlar","masraflar","onaylar":{"ikinci_kisi","ertesi_gun"},"olusturma","imza"}`.
İmza: `taa-imza` `yayin_imzala` (ed25519, `imza` alanı hariç kanonik JSON). **Bugün cihaz anahtarıyla** (`<veri kökü>/cihaz/cihaz.key`);
mağaza (şube) anahtarı sözleşmesi bağlanınca şube imzasına geçer. Anahtar yoksa paket imzasız kuyruğa girer ve `imzali: false` yazılır.
Kuyruk `<veri kökü>/kasa/giden/`; teslim edilen `gonderilen/` altına taşınır (silinmez). Sunucu kasa ucu (Ş3) yok; bugünkü taşıyıcı
`merkez_klasoru` ayarındaki ortak klasördür.

## Matris (Çatı RP6)
`merkez::matris`: mağaza × gün → `geldi` | `gec` | `eksik` | `bekleniyor`, hücrede `kasa_ciro_uyusmuyor`. Son saat mağaza ayarı
(`gonderim_son_saat`, varsayılan 23:00, Türkiye saati); paketin `olusturma` anı esas alınır. Çatı paneline bağlanması Çatı'nın bu türü
`veri_okur` ile okumasını ister (ayrı adım).

## Hata kodları
API 401 (giriş yok), 403 (yetki: yönetici ya da müdür gerekli), 422 (kural: giren onaylayamaz, fark notu, tanımsız POS/ödeme türü…).

## Sürüm kuralı
Değişirse `@2` yeni dosya; eski `@1` en az bir sürüm daha yaşar (§18.3). 

## Akşam ciro Excel'i

`sablon::ciro_sablonu` aylık xlsx üretir: gün başına bir sayfa (`01.11.2026`), şablonla aynı satır/sütun düzeni ve formüller (iki çekmecede A1:E40 birebir). Ayrıntı ve şablondan ayrılan yerler: docs/TAA-KASA.md. Karşılaştırma aracı: `tools/kasa_ciro_karsilastir.py`.
