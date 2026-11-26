<!-- Sahip: Yusuf. yazan: [Claude#taa-kasa-KS1 2026-11-26]. KS1 sözleşmesi; ciro alanları akşam ciro şablonuna (V2) birebir uyarlandı. güncelleme: [Claude#taa-kasa-T6b 2026-11-26] imza, çift doğrulama tutanağı (Tutaa'dan taşındı), açıklamalar. [Claude#taa-kasa-KS2 2026-11-26] telefonla imza istemcisi, imza `ozet`. -->
# kasa.gunsonu

- Tür: veri türü ve yetenek
- Sürüm: @1
- Sunan / tek yazar: **TAA Kasa** (`taakasa`; çekirdek `crates/taa-kasa/src/gunsonu.rs`). Başka program yazmaz.
- Okuyan: TAA Çatı (RP6 evrak matrisi). **Kasa çift doğrulama tutanağı artık TAA Kasa'da üretilir** (Yusuf'un kararı,
  2026-11-26: ana plandaki T6b iptal; Tutaa yalnız eski kâğıt/eski tutanak kayıtlarını tutar). Tutaa T10 (açılış/devir) okuyabilir;
  okuyanlar yalnız okur.
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
| `gun_sonu` | `id` (`GS-...`), `tarih`, `sube`, `cekmece`, `kisi`, `nakit_adetler`, `nakit_sayilan_kurus`, `noktalar` [{`nokta`, `z_toplam_kurus`, `pos` {pos_id: kuruş}}], `nebim_toplam_kurus`, `ciro` {alan: kuruş}, `doviz` [{`kod`, `tutar` (yabancı para kuruşu), `kur_onbinde`}], `devreden_kurus`, `kontrol` [7 × bool], `not`, `aciklamalar` {alan: metin}, `karsilastirma`, `zaman`, `duzeltir` | `ciro` alanları akşam ciro şablonunun satırlarıdır. Genel toplama girenler: `nakit` (NAKİT TL = Nebim/sistem nakdi, masraf ve para üstü düşülmüş), `ziraat_pos`, `iyzico_pos`, `havale` (firma hesabı), `iyzico_odemesi`, `kredi_ceki`, `internet_degisimi`, döviz TL karşılığı ve masraflar. `kasa_fazlasi` / `kasa_acigi` elle girilmez: nakit sayım farkından yazılır (açık MASRAFLAR'ın ilk satırıdır, toplamı değiştirmez). Bilgi satırları (toplama girmez, eksi olabilir): `kredi_karti_fark`, `kasa_fazlasi`, `havale_fark`, `degisim`, `iade`. `aciklamalar` yalnız `havale_fark` ve `kredi_karti_fark` için; o satırda fark varsa zorunlu (ör. "… iadesine istinaden fark"). Döviz: USD, EUR, GBP, CHF (kur elle). |
| `onay` | `id` (gün sonu), `kisi`, `onay` (ikinci_kisi, ertesi_gun), `zaman` | Ertesi gün gözden geçirmesi müdür ya da yönetici. `ikinci_kisi` onayı eski defterlerde vardır; artık imzadır (`imza`). |
| `imza` | `id` (gün sonu), `sira` (1, 2), `rol` (`kasa_sorumlusu`, `magaza_sorumlusu`), `kisi`, `zaman`, `kanal` (`telefon` \| `ekran`), `cihaz` (telefonla imzada zorunlu, `eslesme_kimligi:pc_pin`), `ozet` (telefonla imzada onaylanan özetin SHA-256'sı; isteğe bağlı), `cihaz_dogrulama`, `kisi_rol` (`mudur` \| `yonetici`; eski kayıtta yok) | Yalnız yöneticiler (mağaza müdürü, yönetici) imzalar ve onaylar. İmza isteği yalnız kapanış listesi tamamlanınca açılır. En çok iki imza; aynı kişi iki kez imzalayamaz; ikinci imzayı giren atamaz. Gönderilmiş güne imza eklenmez. |
| `tutanak` | `id` (gün sonu), `erp` {kalem kodu: kuruş}, `islem`, `kisi`, `zaman` | Çift doğrulama tutanağı ekleri: Nebim kasa Excel'inden içe aktarılan ERP sütunu ve "Yapılan İşlem". Son kayıt geçerli. |
| `iptal` | `id`, `neden`, `kisi`, `zaman` | Yalnız yönetici girişiyle; neden zorunlu. **Silme yok.** |
| (not) | `ciro.efatura_pos` 2026-11-26'da kaldırıldı: yeni kayıtta 0 dışı değer reddedilir, eski kayıtlarda okunur ve toplama girer. `ciro.kredi_ceki` eksi olabilir. | |
| `gerekce` | `id` (gün sonu), `gerekce` (5–300 karakter), `kisi`, `zaman` | İkinci imza atılmayacaksa gönderimden önce zorunlu; son kayıt geçerli. Yalnız tek imzalı, gönderilmemiş güne yazılır. |
| `gonderim` | `id`, `dosya`, `imzali`, `zaman` | Paket kuyruğa alındı. |

## Karşılaştırma (`karsilastirma`)
Satırlar (her biri `beklenen_kurus`, `bulunan_kurus`, `fark_kurus = bulunan − beklenen`):
1. Nakit: sayılan ↔ açılış + NAKİT TL. (Şablonda NAKİT TL masraflar çıktıktan sonra kalan nakittir; masraf genel toplama ayrıca eklenir.)
2. Z raporu toplamı (tüm kasa noktaları) ↔ genel toplam + DEĞİŞİM − İNTERNET DEĞİŞİMİ + İADE − KREDİ ÇEKİ (kredi çeki kalemi 27.09 Z raporlarıyla koşullu doğrulandı; `KasaAyar.z_formulu` ile ayarlanır). Genel toplam: şablonun son GENEL TOPLAM'ına çekmecenin payı (genel toplama giren alanlar + döviz TL karşılığı + çekmecenin masrafları); `ciro_toplam_kurus` budur. (25.11.2026 dolu dosyalarında kuruşu kuruşuna doğrulandı: Z = Nebim "Peşin Satış Normal", genel toplam = Nebim "Peşin Satış Toplam".)
3. Her POS ödeme türü için: POS gün sonları toplamı ↔ o türün cirosu (Ziraat ve İyzico ayrı satır).
4. Satış sistemi (Nebim) mağaza raporu toplamı ↔ genel toplam.

`fark_var`: herhangi bir satır mağaza toleransının (`fark_toleransi_kurus`, varsayılan 0) dışında; o zaman `not` zorunludur.
`kasa_ciro_uyusmuyor`: 1. ya da 2. satır tolerans dışında; Çatı'daki "kasa ile ciro uyuşmuyor" uyarısı budur.

## Kurallar
- Aynı gün ve çekmece için tek açılış ve tek gün sonu; ikincisi reddedilir, düzeltme yönetici girişiyle "iptal et ve yeniden gir"
  (eski kayıt `iptal` satırıyla kapanır, yeni kayıt `duzeltir` alanında eskisini gösterir). Geçersiz düzeltme eski kaydı iptal etmez.
- Tek imzalı gönderimde ayrıca `kasa.tek-imza@1` raporu TAA Çatı kuyruğuna alınır (engellemez). Her gönderimden sonra şifreli yedek `kasa.yedek@1` kuyruğa alınır.
- Gönderim ön koşulu: kapanış listesinin 7 maddesi, **en az bir imza** ve tek imzada **gerekçe**. İlk imzadan sonra `tutanak` kaydı (ekler) eklenmez.
- İkinci imza yoksa gün yine kapanır ve gerekçeyle gönderilir;
  kayıtta, pakette (`ikinci_imza_durumu`), Excel'de, tutanakta, ekranda ve matriste "ikinci imza atmadı" yazar.
- Telefonla imza: istemci `telefon_imza` (Telefon Köprüsü `onay_iste@1` / `onay@1` / `onay_red@1`, kapsam `taa_kasa_gun_sonu`,
  özet = gün sonu tutarlarının kanonik metninin SHA-256'sı, 60 sn). Köprü borusu bağlanana kadar API `kanal: telefon` isteğine
  501 döner ve imza ekrandan (yer tutucu) atılır. Ayrıntı: docs/TAA-KASA.md "Telefonla imza".

## Gönderim paketi (merkeze)
`{"tur":"kasa.gunsonu@1","surum":1,"sube","tarih","cekmece","gun_sonu","sayimlar","masraflar","onaylar":{"ikinci_kisi","ertesi_gun"},"imzalar","ikinci_imza_durumu","ikinci_imza_gerekcesi","cift_dogrulama","olusturma","imza"}`.

- `imzalar`: iki rol de yazılır: `{sira, rol, rol_adi, kisi, zaman, kanal, cihaz, ozet, cihaz_dogrulama, kisi_rol, durum: "imzaladi"|"imza_atmadi", imzalamadi}`;
  atılmamış ikinci imzada `not: "ikinci imza atmadı"` ve varsa `gerekce`. `cift_dogrulama.ikinci_imza_gerekcesi` aynı değer.
- `cift_dogrulama` (`taa-kasa.cift_dogrulama@1`, Kasa Çift Doğrulama Tutanağı; şablon Tutaa'dan alındı:
  `crates/taa-kasa/sablonlar/kasa-cift-dogrulama.json`): `numara` (KCD-yıl-<gün sonu>), `baslik`, `sube`, `tarih`, `cekmece`,
  `gun_sonu_id`, `kalemler` [8 × `{kod, ad, erp_kurus, fiili_kurus, fark_kurus, gerekce, fiili_kaynagi}`], `toplam_fark_kurus`,
  `fark_tutar_kurus` (mutlak toplam), `fark_kalem`, `neden`, `islem`, `sonuc` ("Fark yoktur." / "Fark tespit edilmiştir."),
  `kontroller` [5 × `{metin, yapildi}`], `imzalar` (yukarıdaki gibi).

### Çift doğrulama: 8 sabit kalem ve eşleme
| kod | Tutanak satırı | ERP (Nebim) | Fiili |
|---|---|---|---|
| `nakit_tl` | Nakit (TL) | NAKİT TL | sayılan − açılış (fark = kasa fazlası/açığı) |
| `ziraat_pos` | Ziraat Bankası POS | ZİRAAT BANKASI POS:1 | Ziraat POS gün sonu |
| `iyzico_fiziksel_pos` | İyzico Fiziksel POS | İYZİCO POS:2 (eski kayıtta + E-FATURA POS:2) | İyzico cihazı gün sonu (QR ve tüm banka kartları) |
| `iyzico_sanal_pos` | İyzico Sanal POS | İYZİCO ÖDEMESİ | = ERP (ayrı sayım yok) |
| `havale_eft` | Havale / EFT | BANKAYA HAVALE | havale + HAVALE +/- |
| `kredi_ceki` | Kredi Çeki | KREDİ ÇEKİ + DEĞİŞİM − İNTERNET DEĞİŞİMİ | = ERP (belge) |
| `eticaret_degisimi` | E-ticaret Değişimi | İNTERNET DEĞİŞİMİ | = ERP (belge) |
| `diger` | Diğer | döviz TL karşılığı | döviz sayımı |

Fark = fiili − ERP (kuruş); farkı olan satırda gerekçe gün sonu notudur; herhangi bir satırda fark varsa sonuç kendiliğinden
"Fark tespit edilmiştir." Nebim kasa Excel'i içe aktarılırsa (`tutanak` kaydı) ERP o değerdir. Kontrol kutuları gün sonundan
işaretlenir (1 her zaman; 2 kapanış listesi 2. madde; 3 masraf fişleri; 4 iki imza; 5 ekstre eşleştirmesi programda yok, boş).
A4 çıktı: `GET /api/tutanak?id=` (yazdırılabilir HTML; imza yoksa TASLAK).
İmza: `taa-imza` `yayin_imzala` (ed25519, `imza` alanı hariç kanonik JSON). **Bugün cihaz anahtarıyla** (`<veri kökü>/cihaz/cihaz.key`);
mağaza (şube) anahtarı sözleşmesi bağlanınca şube imzasına geçer. Anahtar yoksa paket imzasız kuyruğa girer ve `imzali: false` yazılır.
Kuyruk `<veri kökü>/kasa/giden/`; teslim edilen `gonderilen/` altına taşınır (silinmez). Sunucu kasa ucu (Ş3) yok; bugünkü taşıyıcı
`merkez_klasoru` ayarındaki ortak klasördür.

## Matris (Çatı RP6)
`merkez::matris`: mağaza × gün → `geldi` | `gec` | `eksik` | `bekleniyor`, hücrede `kasa_ciro_uyusmuyor`, `ikinci_imza_atmadi`, `ikinci_imza_gerekceleri`, `telefon_imza_sorunlari`, `telefon_imza_dogrulanamadi`; satırda `tek_imza_gun_sayisi`, `tek_imza_uyarisi` (ayar `tek_imza_uyarisi`: 14 günde 3). Son saat mağaza ayarı
(`gonderim_son_saat`, varsayılan 23:00, Türkiye saati); paketin `olusturma` anı esas alınır. Çatı paneline bağlanması Çatı'nın bu türü
`veri_okur` ile okumasını ister (ayrı adım).

## Hata kodları
API 401 (giriş yok), 403 (yetki: yönetici ya da müdür gerekli), 422 (kural: giren onaylayamaz, fark notu, tanımsız POS/ödeme türü…).

## Sürüm kuralı
Değişirse `@2` yeni dosya; eski `@1` en az bir sürüm daha yaşar (§18.3). 

## Akşam ciro Excel'i

`sablon::ciro_sablonu` aylık xlsx üretir: gün başına bir sayfa (`01.11.2026`), şablonla aynı satır/sütun düzeni ve formüller (iki çekmecede A1:E40 birebir). Ayrıntı ve şablondan ayrılan yerler: docs/TAA-KASA.md. Karşılaştırma aracı: `tools/kasa_ciro_karsilastir.py`.

## WhatsApp özeti

`whatsapp::ozet` (API `POST /api/whatsapp {tarih, girdi?}`): Yusuf'un akşam metni biçiminde düz yazı; aynı kayıtlardan (Excel ve
tutanakla aynı veri). Mağaza adı çekmece adından ("MERKEZ MAĞAZA gg.aa.yyyy"); sıfır "0 TL"; masraf kalemleri tek tek
(kalem, tutar, not); `havale_fark` açıklaması satırın sonunda parantez içinde; `whatsapp_kalin` ayarıyla başlık ve toplamlar
*kalın*. GENEL TOPLAM = `ciro_toplami` (kasa fazlası, kredi kartı ve havale +/- girmez).
