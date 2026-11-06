# TaaSüre: öğrenen süre tahmini (plan)
Sahip: Yusuf. İstek 2026-11-06: usta belirtilen süreden hızlı/yavaş yaparsa tahmin süreleri usta bazlı zamanla güncellensin. Bu bir TASARIM planıdır; kodlanmadı, onay bekliyor.

> GÜNCELLEME 2026-11-06 22:59 (Yusuf): Başladı/Bitti tuşu yerine usta QR okutur; kazanılan dakika = adet × norm süre (`taa-takip::puan`, docs/USTA-PUAN.md). Gerçek süre ölçümü isteğe bağlı sonraya kaldı. Ağırlık/hedefler veri geldikçe `gozden_gecir` ile sınırlı ayarlanır.

## Önce veri: şu an gerçek süre ölçülemiyor
Fiş durum geçmişi yalnız TARİH tutuyor (bekliyor/terzihanede/hazır). Dakika ölçmek için ustanın iş başında iki dokunuşu gerekir: **Başladı** ve **Bitti** (zaman damgalı, kalem başına). Bu olmadan öğrenme olmaz. Dokunuş telefonda ya da ustanın ekranında tek tuş; fişte zaten `imzalar` (usta imzası) mekanizması var, aynı yerden zaman damgası alınabilir.

## Yöntem (öneri)
- **Öğrenilen birim:** usta × işlem kodu (× varyant). Usta başka ustanın verisini etkilemez.
- **Örnek:** bir kalemin Başladı→Bitti süresi, ustanın mola/öğle/gün sonu aralıkları ve duraklatmalar çıkarılarak (çalışma dakikası). Duraklatma tuşu ya da vardiya saatleri ayarı gerekir.
- **Başlangıç değeri:** bugünkü süre tablosu (usta verisi). Öğrenme tabloyu ezmez, yanına yazar.
- **Güncelleme (şartname SADE-10 "EK — TaaSüre kendini eğitir" esas alındı):** üstel ağırlıklı ortalama: `yeni = (1-a)·eski + a·ölçülen`, a = yeni ölçümün etkisi (şartname önerisi %20–30; kesin oran Yusuf/usta ile, sayı uydurulmaz). Başlangıç = ustanın verdiği sabit süre. ~~Küçültülmüş ortalama (n·ort + k·tablo)~~ pasif 2026-11-06 21:55, yerine üstel ortalama + en az N ölçüm eşiği (aynı güvence).
- **İş ve bekleme ayrı:** iş süresi = Başladı→Bitti; bekleme = fişe girdi→Başladı. İkisi ayrı öğrenilir; teslim tahmini = kuyruk bekleme + iş süresi (usta hızlı ama iş birikmiş olabilir).
- **Aralık:** tek tarih yerine "Perşembe, en geç Cuma" (ortalama + sapma); belirsizlik büyükse geniş söyler. Ölçüm az ise "elle seç".
- **"Bu ölçümü sayma":** kullanıcı kesinti/unutulan tuş ölçümünü dışlayabilir; ölçümler listelenir.
- **Güvenlik:** (1) en az N ölçüm dolmadan (N Yusuf/usta ile; öneri 5) sabit süre kullanılır; (2) uç değerler (tablo süresinin ~3 katı, yarısı gibi mantıksız) örnekten dışlanır; ortalama yerine medyan da seçilebilir; (3) tek parçalı fişlerden gelen örnek tam ağırlıkta, çok kalemli fişlerde süre kalemlere tablo oranında dağıtılır ve yarım ağırlık alır; (4) "zor durum" (bel 4 cm üstü) ayrı örnek grubu, normal bel süresini bozmaz; (5) ani değişim (son 10 örnek ortalaması eskiden %X sapma) ustaya "süre güncellensin mi?" diye sorulur, otomatik değişmez.
- **Gösterim:** tahmin gerekçesinde "Ali bu işi ortalama 12 dk yapıyor (tablo 15, 23 örnek)" yazılır. Öğrenilen değer Ayarlar'da görülür, ustaya/yöneticiye sıfırlama ve elle düzeltme hakkı verilir.
- **Kapanış:** açılıp kapanabilir (`teslim-ogrenme`, varsayılan KAPALI; veri birikince açılır). Kapalıyken yalnız tablo kullanılır.

## Yapılmayacaklar (kural)
- Usta karşılaştırma/puan yok: öğrenilen süreler yalnız TARİH TAHMİNİ içindir, performans raporuna girmez, ustalar birbirine karşı sıralanmaz (Yusuf kararı: usta karşılaştırma kapalı).
- Ölçüm ustanın baskısı için kullanılmaz; ustaların bilgisi ve onayıyla açılır (veri sahibi usta).
- Sayı uydurulmaz: k, en az örnek ve sapma eşiği öneridir, Yusuf/usta onayıyla kesinleşir.

## Alternatifler
1. **Basit (önerim):** yukarıdaki küçültülmüş ortalama. Anlaşılır, açıklanabilir, az veriyle çalışır.
2. **Çarpan:** usta başına tek "hız çarpanı" (ör. 0,9×). Çok az veriyle çalışır ama işlemler arası farkı (paçada hızlı, ceketle yavaş) kaçırır.
3. **Makine öğrenmesi:** gereksiz; veri az ve açıklanabilirlik şart.

## İşler (sırayla, ayrı PR)
1. Başladı/Bitti/Duraklat olayları fiş defterine (`taa-fis`, zaman damgalı, geriye uyumlu).
2. Örnek toplama + usta×işlem istatistiği (`taa-takip::sure_ogren`), sanal testlerle (sahte zaman damgaları).
3. Tahminde öğrenilen değeri kullanma + gerekçe metni + ayar anahtarı.
4. Ayarlar ekranı: görüntüle, sıfırla, elle düzelt.

## Doğrulama
Python ile sahte geçmiş verilerle simülasyon: öğrenme oranı (a) ve N seçimi için; Rust çekirdek tahmini hesaplar. Günlük kapasite 510 dk (8,5 saat, molasız; Yusuf 2026-11-06) iş dakikası olarak hesaba girer. Başladı/Bitti olayı fiş defterine eklenecek (1. iş).
