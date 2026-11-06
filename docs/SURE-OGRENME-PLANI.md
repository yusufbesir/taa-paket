# TaaSüre: öğrenen süre tahmini (plan)
Sahip: Yusuf. İstek 2026-11-06: usta belirtilen süreden hızlı/yavaş yaparsa tahmin süreleri usta bazlı zamanla güncellensin. Bu bir TASARIM planıdır; kodlanmadı, onay bekliyor.

## Önce veri: şu an gerçek süre ölçülemiyor
Fiş durum geçmişi yalnız TARİH tutuyor (bekliyor/terzihanede/hazır). Dakika ölçmek için ustanın iş başında iki dokunuşu gerekir: **Başladı** ve **Bitti** (zaman damgalı, kalem başına). Bu olmadan öğrenme olmaz. Dokunuş telefonda ya da ustanın ekranında tek tuş; fişte zaten `imzalar` (usta imzası) mekanizması var, aynı yerden zaman damgası alınabilir.

## Yöntem (öneri)
- **Öğrenilen birim:** usta × işlem kodu (× varyant). Usta başka ustanın verisini etkilemez.
- **Örnek:** bir kalemin Başladı→Bitti süresi, ustanın mola/öğle/gün sonu aralıkları ve duraklatmalar çıkarılarak (çalışma dakikası). Duraklatma tuşu ya da vardiya saatleri ayarı gerekir.
- **Başlangıç değeri:** bugünkü süre tablosu (usta verisi). Öğrenme tabloyu ezmez, yanına yazar.
- **Güncelleme:** `yeni = (n·ölçülen_ortalama + k·tablo) / (n + k)` (küçültülmüş ortalama; az örnekle tablodan sapmaz). n = o usta×işlem örnek sayısı, k = tabloya güven ağırlığı (öneri k=5; ayarlanır). Zamanla n büyüdükçe usta kendi gerçek ortalamasına yaklaşır.
- **Güvenlik:** (1) en az örnek sayısı dolmadan (öneri 5) tahmine yansıtılmaz; (2) uç değerler (tablo süresinin ~3 katı, yarısı gibi mantıksız) örnekten dışlanır; ortalama yerine medyan da seçilebilir; (3) tek parçalı fişlerden gelen örnek tam ağırlıkta, çok kalemli fişlerde süre kalemlere tablo oranında dağıtılır ve yarım ağırlık alır; (4) "zor durum" (bel 4 cm üstü) ayrı örnek grubu, normal bel süresini bozmaz; (5) ani değişim (son 10 örnek ortalaması eskiden %X sapma) ustaya "süre güncellensin mi?" diye sorulur, otomatik değişmez.
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
