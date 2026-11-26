# TAA Kasa — gün sonu programı

Program adı `taakasa`, exe `taa-kasa` (crates/taa-kasa-ekran). Çekirdek: crates/taa-kasa (`gunsonu`, `merkez`,
`sablon`, `xlsx`, `zaman`; eski `Defter` uyumlu). Fiyat Etiketi'ndeki "Kasa" ekranıyla ilgisi yoktur.

Tek yazar TAA Kasa'dır (`kasa.gunsonu@1`, `kasa.masraf@1`); Tutaa yalnız okur. Sözleşmeler:
docs/sozlesmeler/kasa.gunsonu.md, docs/sozlesmeler/kasa.masraf.md.

## Akış

1. **Açılış sayımı** (çekmece başına, günde bir). Beklenen = önceki gün sonunun devreden tutarı.
2. **Gün içi:** ara sayım ve vardiya devri (ikinci kişi zorunlu, giren olamaz), masraf girişi.
3. **Gün sonu sihirbazı (5 adım):** nakit sayımı → döviz → Z raporu / POS → Nebim ve ciro satırları (akşam ciro
   Excel'inin satırları, aynı sırayla) → kontrol listesi. Sağda canlı fark paneli.
4. **İkinci kişi onayı** ("giren onaylayamaz").
5. **Gönder:** paket (imzalı, cihaz anahtarı varsa) kuyruğa yazılır, merkeze teslim edilir; ağ yoksa kuyrukta bekler.

Fark varsa açıklama zorunlu; nakit ya da Z↔ciro tutmazsa "kasa ile ciro uyuşmuyor" uyarısı çıkar.

## Kurallar

- Kayıt yalnız eklenir (JSONL). Silme yok; düzeltme = yönetici iptali (gerekçeli) + `duzeltir` izli yeni kayıt.
- Masraf: eşik (varsayılan 2.000 TL) altını müdür, üstünü yönetici (merkez) onaylar; giren onaylayamaz.
- İptal, düzeltme, ayarlar ve matris yalnız yönetici girişiyle; Excel giriş yapmış kişiye açık.

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

- Z raporu ve Nebim, şablonun hangi toplamıyla karşılaştırılmalı? Şimdilik son GENEL TOPLAM (kredi çeki ve
  internet değişimi dahil). Z bunları içermiyorsa ilk GENEL TOPLAM'a (E28) çevrilir.
- NAKİT TL masraflar çıktıktan sonra kalan nakit mi (çekmece beklentisi = açılış + NAKİT TL varsayıldı)?
- KASA AÇIĞI / KASA FAZLASI elle mi girilsin, yoksa nakit sayım farkından mı doldurulsun? (Şimdilik elle.)
- E-FATURA POS:2 ve İYZİCO ÖDEMESİ'nin hangi POS/kanaldan geldiği; E-fatura için ayrı POS karşılaştırması gerekir mi?
- Masraf satırları 9 ile sınırlı; fazlası son satırda toplanıyor. Uygun mu?
- Masraf türü listesi (şimdilik örnek liste), kasa limiti, fark toleransı (varsayılan 0), gönderim son saati
  (varsayılan 23:00).
- Mağaza imza anahtarı ve sunucu kasa ucu.

## Akşam ciro Excel'i (şablon)

Kaynak: şubelerin "KASIM AYI CİRO ŞABLONU 2026" (V1) ve "… V2" dosyaları (Yusuf'un örnekleri; dosyalar depoya
konmadı, yalnız yapıları alındı). Kayıtlar ve Excel → "Akşam ciro Excel'i" aynı düzende aylık dosya üretir
(`/api/excel/ciro?ay=YYYY-AA`).

**Yapı (V1 ve V2 ortak):** ay başına bir dosya, gün başına bir sayfa (`01.11.2026` … `30.11.2026`), her sayfa
A1:E40 tablosu. A: satır adı; B, C: mağaza sütunları (OUTLET MAĞAZA, ANA MAĞAZA); D: TOPLAM DÖVİZ; E: tarih başlıklı
toplam.

| Satır | Ad | Toplama |
|---|---|---|
| 2 | NAKİT TL | E28 |
| 3–6 | DOLAR, EURO, STERLİN, İSVİÇRE FRANGI (adet; D = toplam döviz, E = TL karşılığı) | E28 |
| 7–9 | ZİRAAT BANKASI POS:1, İYZİCO POS:2, E-FATURA POS:2 | E28 |
| 10–11 | KREDİ KARTI +/-, KASA FAZLASI | bilgi |
| 12–15 | BANKAYA HAVALE: firma hesabı, İYZİCO ÖDEMESİ (E28), HAVALE +/- (bilgi) | |
| 16–27 | MASRAFLAR: KASA AÇIĞI (mağaza başına), 9 masraf satırı (açıklama A, tutar E), TOPLAM MASRAF = SUM(E17:E26) | E28 |
| 28 | GENEL TOPLAM = SUM(E2:E9, E13:E14, E27) | |
| 29–30 | KREDİ ÇEKİ, İNTERNET DEĞİŞİMİ | E31 |
| 31 | GENEL TOPLAM = E28 + E29 + E30 | |
| 32–33 | OUTLET / ANA MAĞAZA TOPLAMI | |
| 34–35 | döviz başlıkları ve GÜNLÜK DÖVİZ KURLARI (elle) | |
| 36–37 | mağaza başına döviz TL karşılığı (adet × kur) | |
| 38–40 | NOT, DEĞİŞİM, İADE (bilgi) | |

**V2'nin V1'den farkı:** satırlar, formüller ve sayfalar aynı. V2'de G sütununda "NASIL DOLDURULUR?" yardımı (sarı =
doldurulacak, gri = hesaplanır, Tab/Enter ile sıradaki kutu), sarı/gri renkler, sayfa koruması (yalnız sarı kutular
açık), B2'den bölme dondurma, yazdırma alanı A1:E40, giriş kutularında alt sınır doğrulaması, düzenli sütun
genişlikleri; İADE satırında V1'de toplam 0 sabitken V2'de `=B40+C40`. V1'de tablo biçimi (TableStyleMedium15) vardı.

**Program çıktısı:** V2 düzeni (renkler, koruma, dondurma, yazdırma alanı, G yardım sütunu, sayı biçimleri).
Her çekmece bir mağaza sütunu, sütun başlığı çekmece adı; A1 = Ayarlar'daki "Ciro Excel'i başlığı"; firma havale
satırının adı Ayarlar'dan. İki çekmecede hücre adresleri şablonla aynıdır; LibreOffice ile yeniden hesaplatılınca
program değerleriyle 30 sayfada fark çıkmadı. Şablondan ayrılan yerler:
- Masraf satırlarında B/C'ye masrafın çıktığı çekmece yazılır (şablonda gri ve boş); mağaza toplamı kendi masrafını
  ve kasa açığını içerir. Şablonda tüm masraf (outlet'in kasa açığı dahil) ANA MAĞAZA TOPLAMI'na yazılıyordu
  (`E33` içinde `E27`); iki mağaza toplamının toplamı yine GENEL TOPLAM'dır.
- Formüller tablo adıyla değil düz hücre başvurusuyla yazılır; veri doğrulama yazılmaz; G5'e "TAA Kasa
  kayıtlarından üretildi" notu eklenir.
- Kur, o gün kuru giren ilk çekmeceden alınır (şablonda tek kur satırı var).

## Karşılaştırma yöntemi

`tools/kasa_ciro_karsilastir.py dolu.xlsx program.xlsx [--gun 26.11.2026] [--json]`: şubenin doldurduğu Excel'i
programın aynı ay için ürettiği Excel'le satır adına göre (hücre adresine göre değil) karşılaştırır, farkları
satır × sütun olarak listeler; masraf satırları sırasız (toplam olarak) karşılaştırılır. Yalnız standart kitaplık.

Dolu sayfa / dolu Excel gelince: o günün kayıtları programa girilir (ya da test verisi olarak kurulur), program
Excel'i üretilir, araçla farklar çıkarılır. "Mağaza hareket özeti" (Nebim) ve "alınan ödemeler listesi" (POS/banka)
biçimleri görülünce aynı araca okuyucu eklenecek: hareket özeti toplamı ↔ `nebim_toplam_kurus` ve GENEL TOPLAM,
ödeme listesi ödeme türüne göre toplanıp ↔ ilgili ciro satırları (POS:1, POS:2, E-fatura, havale, İyzico ödemesi).

## Tasarım

Arayüz önerisi bir dış tasarım modelinden (yalnız sırsız genel özet gönderilerek) alındı: aşamaya göre değişen tek
ana düğme, 5 adımlı sihirbaz + yapışkan fark paneli, numpad odaklı giriş, zümrüt paleti, farkın renk + simge + işaretle
gösterimi (✓ ▲ ▼), "Sil" sözcüğünün hiç kullanılmaması.

## Çalıştırma

`taa-kasa [--kok <veri kökü>] [--pencere-acma]`. Veri kökü: `TAA_VERI_KOK`, yoksa `%APPDATA%\TAA\veri`.
Kayıtlar `<kök>/kasa/` altında (`gunsonu.jsonl`, `masraf.jsonl`, `ayar.json`, `giden/`, `gonderilen/`).
