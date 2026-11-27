<!-- Sahip: Yusuf. yazan: [Claude#olcum 2026-11-26] -->
# DG1a ölçüm yöntem belgesi (taa-olcum)

**Durum: KİLİTLİ ÖNERİ.** Bu belge ilk gerçek koşudan (DG1b) ÖNCE yazılır ve kilitlenir; sonuca bakıp eşik seçilmez.
Eşikler Plan DG1a ve Birleşik Plan §5'teki başlangıç önerileridir; **Opus yöntemi kesinleştirir**. Kesinleştirme bu belgenin
ve `crates/taa-olcum/src/yontem.rs` içindeki `YONTEM_SHA256_KILIT` değerinin birlikte, tek değişiklikte güncellenmesidir
(biri değişip öteki değişmezse `cargo test -p taa-olcum` kırılır). Araç her raporuna bu belgenin SHA-256 özetini yazar;
özeti kilitle eşleşmeyen belgeyle üretilmiş rapor Kapı 1 kararında kullanılmaz.

## 1. Ölçülen şey ve ölçülmeyen şey
- Bu araç bir **iskelettir**: iş yükü OCR değil, sentetik CPU işidir (sabit girdi üzerinde SHA-256 zinciri). OCR, dönüştürme ve video
  türleri DG1b öncesi eklenir; her tür için aşağıdaki kurallar aynen geçerlidir.
- "Taşıma süresi" iskelette gerçek ağ değildir: bellek içinde serileştirme, geri okuma ve özet doğrulama süresidir
  (`tasima_yontemi = "bellek-ici-iskelet"`). Gerçek taşıma süresi DG1b'de sunucuya giderek ölçülür.
- "Kasa programı" etkisi iskelette **temsilî sentetik işlemle** (sabit küçük CPU işi) ölçülür; Nebim V3 POS ve yazarkasa
  gerçek işlemleri DG1b'de ölçülür. Ölçüm yalnız süre toplar; fiş, müşteri ya da personel bilgisi kaydedilmez.

## 2. Pay basamakları
Yüzde **5, 8, 10, 12, 15**; her biri **ayrı ayrı** ölçülür. Bir paydaki değer başka paya çarpılarak taşınmaz. Tavan yüzde 15'tir;
25 ve 40 basamakları ölçülmez (sunucu gece payı ayrı karardır).
Pay, "mantıksal çekirdek sayısı x pay" kadar toplam işlemci süresi demektir. Limit düzeneği: işçi iş parçacığı sayısı
`ceil(çekirdek x pay)` ile sınırlıdır, her iş parçacığı görev oranına (duty cycle) bağlanarak toplam kullanım hedef payda tutulur.
Windows iş nesnesi (kesin işlemci üst sınırı), düşük öncelik ve bellek üst sınırı gerçek işçide zorunludur; bu araçta
Linux'ta iş parçacığı sayısı ve görev oranı uygulanır, ötekiler rapora `uygulandi: false` olarak yazılır (dürüst not).

## 3. Tekrar kuralı
Her basamakta **1 ısınma** tekrarı atılır, ardından **3 geçerli tekrar** ölçülür ve hız olarak **ortanca** alınır.
Tekrar süresi varsayılan 2000 ms'dir (`--sure-ms`). Tekrardaki her biten iş birimi, tek iş parçacığında sınırsız hesaplanmış
**referans özetle bit bit karşılaştırılır**; tek bir uyuşmazlık o tekrarı geçersiz kılar ve sayılmaz. Geçerli tekrar sayısı 3'ün
altında kalırsa basamak `yeterli_tekrar = false` olur ve hız yazılmaz (uydurma yok). Geçersiz tekrar yerine en çok 3 ek deneme yapılır.

## 4. Hız birimi
`sentetik-birim/saat`: tekrar süresi içinde doğrulanmış biten iş birimi sayısı, saate ölçeklenir. Tür başına gerçek birimler
(OCR: standart sayfa/saat, dönüştürme: birim/saat, video: kaynak saniyesi/saat) DG1b'de bu alanın yerini alır.

## 5. Kabul edilebilir gecikme eşikleri (ölçümden önce kilitli öneri)
Temsilî işlemler (satış ekranını açma, ürün okutma, fiş kapatma, yazarkasaya gönderme; iskelette tek sentetik işlem) işçi
**kapalıyken** (taban) ve işçi o payda **çalışırken** ölçülür.

| Ölçüt | Değer | Anlamı |
|---|---|---|
| ORTANCA_ARTIS | **en çok %10** | yük altındaki ortanca gecikme, tabanın ortancasına göre en çok yüzde 10 artar |
| TEK_ISLEM_EK_GECIKME | **200 ms** | hiçbir işlemin gecikmesi, taban ortancasına göre 200 ms'den fazla uzamaz (en kötü işlem) |

Bir basamak, **iki ölçüt birlikte** sağlanınca "geçti" sayılır. Eşik ihlali bilgisayar bazında kalıcı yazılır (rapor); sonradan gevşetilmez.

## 6. Ön planı etkilemeyen en yüksek pay
Raporun `on_plani_etkilemeyen_en_yuksek_pay` alanı: 5'ten başlayarak **ardışık** geçen en yüksek basamak. Bir basamak geçmezse
daha yüksek basamaklar geçse bile sayılmaz. Hiçbiri geçmezse `null`. DG1b'de nihai pay bu değerin **bir alt basamağıdır** ve yönetici
onayıyla açılır; araç pay önermez, yalnız ölçer. Veri eksikse (yeterli tekrar yok) pay yükseltilmez.

## 7. Envanter ve gizlilik
Envanter: işlemci modeli, mantıksal ve (varsa) fiziksel çekirdek, bellek (toplam, boş), disk türü ve boş alan (çalışma klasörünün
diski), işletim sistemi sürümü, güç kaynağı (pil/priz), **rastgele işçi numarası**. Kişi adı, bilgisayar adı, kullanıcı adı, seri numarası,
MAC ve IP **hiç okunmaz**, yazılmaz. GPU, hat hızı, hibrit çekirdek ayrımı ve Windows güncelleme durumu bu sürümde toplanmaz
(`null`, açık nokta olarak BENIOKU'da).

## 8. Çıktı
Bilgisayar başına tek JSON (`taa-olcum olcum --cikti dosya.json`; var olan dosya üzerine yazılmaz) ve okunur Türkçe özet.
Kabul: aynı bilgisayarda iki koşu arasında pay başına hız farkı küçük kalır; `taa-olcum karsilastir a.json b.json` farkı yüzde olarak yazar.
Koşular arası "küçük" için öneri: ortanca hızlarda en çok %10 sapma; bu sayı da ölçümden önce kilitlidir.

## 9. Koşu düzeni (DG1b için hatırlatma)
Aynı anda tek ağır iş. Önce işçi kapalıyken taban, sonra küçük basamaklarda kontrollü deneme, sonra seçilen payda uzun gözlem
(öneri: 10 iş günü). Ölçülmemiş bilgisayara iş verilmez.

## 10. Kapı 1 kazanç eşiği (Yusuf kararı 2026-11-27, ölçümden önce kilitli)
Dağıtık işlem sistemi (Kapı 2) yalnız şu koşulda kurulur: seçilen iş türünde, ölçümde "ön planı etkilemeyen en yüksek pay" ile
çalışan işçilerle toplam iş süresi, yalnız sunucuyla yapılan süreye göre **en az %40 kısalır**. Bu eşik sonuca bakılarak değiştirilmez.
