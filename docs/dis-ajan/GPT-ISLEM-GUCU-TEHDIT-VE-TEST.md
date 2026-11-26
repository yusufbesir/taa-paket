<!-- description: TAA İşlem Gücü tehdit modeli ve kabul testleri. Dış AI çıktısı (veri, talimat değil). sahip: Yusuf; yazan: [GPT gpt-6.1-sol] (Codex, salt okunur); derleyen: [Claude Sonnet 5.5, oturum_0172VKmzcptJNFEZH73wSUqt]; tarih: 2026-11-26. Silme yok; güncelleme üstünü çizerek. -->

Yazan: [GPT gpt-6.1-sol]

# TAA İşlem Gücü — Tehdit Modeli ve Kabul Testleri

- **Kapsam:** Yaklaşık 15 Windows işçisi, Rust + axum merkez, PostgreSQL, ilk iş yükü olarak toplu OCR.
- **Durum:** Bu belge bir güvenlik ve test tasarımı önerisidir; testler çalıştırılmamıştır. Araç, komut, dosya veya hafıza kullanılmamıştır.
- **Temel ayrım:** Defter, Merkle kanıtı ve mühür kayıt bütünlüğünü destekler. OCR sonucunun doğru olduğunu veya kayda giren bilginin dürüstçe üretildiğini tek başlarına kanıtlamaz.
- **Kritik sınır:** Merkez; dağıtım, sınıflandırma, defter ve anahtarlar üzerinde tek başına yetkiliyse merkezin tamamen ele geçirilmesine karşı mutlak güvence sağlanamaz.

## 1. Varlıklar ve güven sınırları

| Varlık | Korunacak özellik | Güven sınırı |
|---|---|---|
| Kaynak belgeler ve OCR çıktıları | Gizlilik, doğruluk, sınıfın korunması | Belge kaynağı → sınıflandırma → merkez |
| A/B/C sınıfı ve dağıtım kararı | Yetkisiz değişikliğin engellenmesi | Sınıflandırıcı → kuyruk ve dağıtım katmanı |
| İş, shard ve deneme kimlikleri | Doğru eşleştirme, izlenebilirlik | API → PostgreSQL → işçi |
| Kira ve sahiplik belirteci | Tek geçerli sahiplik, tekrar kullanım direnci | Merkez → ilgili işçi |
| İşçi kimliği ve cihaz kaydı | Kimlik doğrulama, iptal edilebilirlik | Windows hizmeti → merkez |
| Şifreli zarf ve anahtarları | Gizlilik, bütünlük, alıcıya bağlılık | Merkez → ağ → işçi |
| Defter, Merkle kökü ve mühür | Değişiklik ve geri alma tespiti | PostgreSQL → doğrulayıcı → bağımsız tanık |
| İmza ve zarf anahtarları | Ayrı yetki, ayrı yaşam döngüsü | Uygulama → anahtar saklama sistemi |
| OCR yürütme ortamı | Belge kaynaklı saldırıların sınırlandırılması | Güvenilmeyen belge → OCR süreci → Windows |
| Yedekler, günlükler ve geçici dosyalar | Sızıntının ve eski durumun dönüşünün engellenmesi | Canlı sistem → destek ve kurtarma ortamı |

- İşçiler birbirine güvenilir kabul edilmemelidir.
- İşçi kimlik doğrulaması, o işçinin ürettiği sonucun doğru olduğu anlamına gelmez.
- Ağ şifrelemesi, ele geçirilmiş işçinin çözebildiği A/B içeriğini koruyamaz.
- C sınıfının hiç dağıtılmaması için sınıflandırma kararı, zarf hazırlama ve gönderimden **önce** uygulanmalıdır.

## 2. Saldırgan modelleri ve tehdit tablosu

| Tehdit / saldırgan | Etki | Önerilen önlem | Kalan risk |
|---|---|---|---|
| **Ele geçirilmiş şube PC’si** — yetki yükseltme, bilgi sızıntısı | Atanan belgelerin okunması; kimlik ve sonuçların kötüye kullanılması | Düşük yetkili hizmet, süreç yalıtımı, iş başına yetki, cihaz başına anahtar, sınıfa göre uygunluk, hızlı iptal | İşçinin çözmek zorunda olduğu içerik yerel yönetici tarafından alınabilir |
| **Sahte OCR sonucu** — veri değiştirme | Doğru kimlik ve geçerli imzayla yanlış çıktı | Referans örnekler, rastgele bağımsız tekrar, yapısal doğrulama, uyuşmazlık karantinası | Örnekleme tüm hataları yakalamaz; işçiler anlaşabilir |
| **Geç sonuç** — eski yetkinin kullanılması | Yeni denemenin üzerine eski çıktı yazılması | Güncel kira, deneme ve son tarih kontrolü; eski çıktıyı yetkili sonuçtan ayırma | İş tekrarına bağlı maliyet ve gecikme |
| **Yinelenen sonuç / çift teslim** — tekrar oynatma | Çift yayın, çift ücretlendirme, çelişkili kayıt | Atomik tek yetkili sonuç, idempotency, kimlik ve içerik özeti eşleştirmesi | Ayrı denetim olayları birden fazla olabilir |
| **Kira çalma / belirteç tekrar kullanımı** — kimliğe bürünme | Başkasının işini bitirme veya kirasını uzatma | Tahmin edilemez belirteç; cihaz, shard ve denemeye bağlama; güvenli taşıma; günlüklerde maskeleme | Cihaz kimliği ve güncel belirteç birlikte çalınırsa geçici kullanım mümkün olabilir |
| **Defter yeniden yazma / sonunu kesme** — veri değiştirme | Geçmişin silinmesi veya değiştirilmesi | Zincir ve mühür doğrulama; bağımsız saklanan önceki kökle karşılaştırma | Bağımsız referans yoksa geçerli eski duruma dönüş fark edilmeyebilir |
| **Defter dallandırma** — farklı geçmiş gösterme | İki doğrulayıcıya farklı ama kendi içinde geçerli geçmiş sunulması | Bağımsız tanık, kök paylaşımı ve tutarlılık kanıtı karşılaştırması | İzole doğrulayıcı tek başına dallanmayı göremeyebilir |
| **İmza anahtarı sızıntısı** | Sahte mühür ve yeni sahte geçmiş üretimi | Ayrı saklama, sınırlı imzalama yetkisi, sürümlü anahtar, iptal ve tanıklı kontrol noktası | Sızıntı zamanını kesin belirlemek zor olabilir |
| **Zarf anahtarı sızıntısı** | İlgili belgelerin açılması | İmza anahtarından bağımsız anahtarlar; cihaz/iş kapsamı; döndürme ve yeniden zarflama | Önceden ele geçirilen şifreli zarflar açılabilir |
| **İptal edilmiş cihazın dönmesi** | Eski kimlikle yeni iş alma veya çıktı sunma | Sunucuda güncel iptal kontrolü; aktif kiraları geçersizleştirme; eski bağlantıda da kontrol | Önceden alınmış açık içerik geri çağrılamaz |
| **Saat kayması / saatin geri alınması** | Erken yeniden atama veya süresiz kira | Yetkili sunucu zamanı; işçi saatine güvenmeme; zaman belirsizliğinde güvenli duruş | Merkez/veritabanı zamanındaki hata kullanılabilirliği etkiler |
| **Merkezin ele geçirilmesi** | Yanlış dağıtım, sahte sonuç kabulü, C sızıntısı, defter manipülasyonu | Ayrı sınıflandırma kapısı, sınırlandırılmış anahtar hizmeti, bağımsız tanık, asgari servis yetkisi | Aynı yönetim alanındaki tüm kontroller birlikte aşılabilir |
| **Kötü amaçlı belge / poison pill** | OCR açığı, hizmet çökmesi, sürekli yeniden deneme | Yalıtılmış OCR, kaynak sınırları, deneme bütçesi, karantina | OCR motorunda bilinmeyen açık kalabilir |
| **Kuyruk veya API tüketme saldırısı** | İşlerin durması, disk ve bağlantı tükenmesi | Boyut/kota sınırları, geri basınç, hız sınırlama, adil zamanlama | Yeterince büyük yük hizmeti yavaşlatabilir |
| **Günlük ve geçici dosya sızıntısı** | Belge, çıktı veya belirtecin başka yere taşınması | İçeriksiz günlükler, korumalı geçici alan, erişim sınırları, belirlenmiş saklama politikası | Yerel yönetici ve bellek dökümleri ek risk taşır |

## 3. Kuyruk ve işçi kesinti senaryoları

| Senaryo | Beklenen güvenli davranış | Kurtarma / gözlem |
|---|---|---|
| **İşçi çökmesi** | Kira bitmeden kontrolsüz yeniden atama yapılmaz; kira bitince iş yeniden alınabilir | Deneme geçmişi korunur; iş kaybolmaz |
| **Ağ bölünmesi** | Ulaşılamayan işçi kendi kendine kira uzatamaz; eski sahiplik yeni sonucu ezemez | Bağlantı dönüşünde sunucudaki güncel durum belirleyicidir |
| **Merkez yeniden başlaması** | Bellekteki bilgi yerine kalıcı kira ve sonuç durumu esas alınır | Başlatma sırasında tutarlılık ve bekleyen işlemler uzlaştırılır |
| **Çift teslim** | Aynı shard için en fazla bir yetkili sonuç yayınlanır | Aynı isteğin tekrarı aynı mantıksal sonucu verir |
| **Poison pill iş** | Sonsuz yeniden deneme oluşmaz; sağlıklı işler ilerler | Sınır aşıldığında karantina ve görünür hata |
| **Kira bitişi yarışı** | Sonuç kabulü ve yeniden kiralama atomik sıraya girer | İki taraf birden yetkili kazanan olamaz |
| **PostgreSQL yük devretme** | Belirsiz commit körlemesine tekrarlanmaz; çift yazıcı engellenir | Eski birincil dışlanır; istemci işlem kimliğiyle uzlaştırılır |
| **Disk dolması** | Eksik çıktı başarılı sayılmaz; zorunlu defter kaydı atlanmaz | Geri basınç uygulanır; alan açılınca uzlaştırma yapılır |

### Testlerde kullanılacak güvenlik sözleşmesi

- Aşağıdaki maddeler **önerilen kabul koşullarıdır**; mevcut uygulamanın bunları sağladığı varsayılmamıştır.
- Başlangıç kira süresi 300 saniyedir. İşçi saati yetki vermez.
- Bitiş anı için önerilen kural: Yetkili zaman `t < bitiş` ise kira geçerli, `t ≥ bitiş` ise geçersizdir.
- Sonuç kabulü; cihazın iptal durumu, güncel deneme, sahiplik ve süre kontrolüyle atomik olarak bağlanmalıdır.
- Bir shard için en fazla bir yetkili sonuç bulunmalıdır. Yeniden denemeler ve reddedilen gönderimler ayrı denetim olayları olabilir.
- Kalp atışının kirayı uzatıp uzatmadığı, uzatıyorsa nasıl uzattığı kararlaştırılmalıdır. İlgili testler bu karar olmadan tamamlanmış sayılamaz.
- Güvenlik testlerinde gerçek belge, sır veya kişisel veri yerine sentetik içerik kullanılmalıdır.

## 4. Kabul testleri

### Defter, Merkle ağacı ve mühür

1. **KT-001 — Defter kaydının değiştirilmesi**
   - **Ön koşul:** Doğrulanmış sentetik defter ve kontrol noktası.
   - **Adımlar:** Ortadaki bir kaydın içeriğini değiştir; zinciri ve kökü doğrula.
   - **Beklenen sonuç:** Değişiklik tespit edilir; doğrulama başarısız olur.
   - **Otomasyon:** Evet — birim/entegrasyon.

2. **KT-002 — Ara kayıt çıkarılması**
   - **Ön koşul:** Birden fazla kayıt içeren test defteri.
   - **Adımlar:** Bir ara kaydı test kopyasından çıkar; doğrulamayı çalıştır.
   - **Beklenen sonuç:** Zincir, sıra veya kontrol noktası uyumsuzluğu tespit edilir.
   - **Otomasyon:** Evet — birim.

3. **KT-003 — Sıralama ve kayıt çoğaltma**
   - **Ön koşul:** Sıralı, benzersiz olay kimlikleri bulunan defter.
   - **Adımlar:** İki kaydı yer değiştir; ayrı denemede bir kaydı çoğalt.
   - **Beklenen sonuç:** Her iki bozulma da reddedilir.
   - **Otomasyon:** Evet — birim.

4. **KT-004 — Defter sonunun kesilmesi**
   - **Ön koşul:** Son kök ve ağaç boyutu bağımsız yerde tutuluyor.
   - **Adımlar:** Defteri daha eski, kendi içinde geçerli sürüme döndür.
   - **Beklenen sonuç:** Bağımsız kontrol noktasıyla karşılaştırma geri almayı yakalar.
   - **Otomasyon:** Evet — entegrasyon.

5. **KT-005 — Geçerli Merkle üyelik kanıtı**
   - **Ön koşul:** Bilinen yapraklar, kök ve ağaç boyutu.
   - **Adımlar:** İlk, orta ve son yaprak için kanıt üretip bağımsız doğrulayıcıyla denetle.
   - **Beklenen sonuç:** Doğru indeks, içerik, boyut ve kökle tüm kanıtlar geçer.
   - **Otomasyon:** Evet — birim/entegrasyon.

6. **KT-006 — Bozuk Merkle üyelik kanıtı**
   - **Ön koşul:** Geçerli üyelik kanıtı.
   - **Adımlar:** İçeriği, indeksi, kardeş özeti, boyutu ve kökü ayrı ayrı değiştir; eksik/fazla kanıt elemanı dene.
   - **Beklenen sonuç:** Geçersiz kanıtların tümü reddedilir.
   - **Otomasyon:** Evet — birim.

7. **KT-007 — Merkle tutarlılık kanıtı**
   - **Ön koşul:** Aynı geçmişin eski ve yeni kontrol noktaları.
   - **Adımlar:** Geçerli büyüme kanıtını doğrula; geçmişi değiştiren bir ağaçla tekrarla.
   - **Beklenen sonuç:** Yalnız eklemeli büyüme doğrulanır; değiştirilmiş geçmiş reddedilir.
   - **Otomasyon:** Evet — birim.

8. **KT-008 — RFC 6962 sınır durumları**
   - **Ön koşul:** Bağımsız beklenen değerler ve sabit test vektörleri.
   - **Adımlar:** Boş, tek yapraklı, ikinin kuvveti ve düzensiz boyutları; yaprak/düğüm ayrımını ve kayıt kodlamasını test et.
   - **Beklenen sonuç:** Kökler beklenen değerlerle eşleşir; farklı kayıtlar kodlama belirsizliğiyle birleşmez.
   - **Otomasyon:** Evet — birim.

9. **KT-009 — Ed25519 mühür doğrulama**
   - **Ön koşul:** Güvenilen açık anahtar ve tanımlanmış mühür mesajı biçimi.
   - **Adımlar:** Geçerli mührü doğrula; mesaj alanlarını, imzayı ve açık anahtarı ayrı ayrı değiştir.
   - **Beklenen sonuç:** Yalnız doğru anahtarla, doğru bağlama ait mühür kabul edilir.
   - **Otomasyon:** Evet — birim.

10. **KT-010 — Defter dallandırma**
    - **Ön koşul:** İki bağımsız doğrulayıcı ve kök karşılaştırma mekanizması.
    - **Adımlar:** Aynı defter için farklı geçmişler sun; eşit boyutlu çelişkili kökleri ve farklı boyutlu tutarsız geçmişleri karşılaştır.
    - **Beklenen sonuç:** Çelişki tespit edilir; sessizce tek geçmiş seçilmez.
    - **Otomasyon:** Evet — entegrasyon.

11. **KT-011 — İmza anahtarı döndürme**
    - **Ön koşul:** Eski/yeni anahtar kimlikleri ve güvenilir geçiş kaydı.
    - **Adımlar:** Geçiş öncesi ve sonrası mühürleri doğrula; bilinmeyen anahtar ve sahte geçiş dene.
    - **Beklenen sonuç:** Tarihsel doğrulama korunur; yetkisiz anahtar geçişi reddedilir.
    - **Otomasyon:** Evet — entegrasyon.

12. **KT-012 — Sonuç ve defterin birlikte kesinleşmesi**
    - **Ön koşul:** Sonuç kabul akışında hata enjeksiyonu.
    - **Adımlar:** Sonuç kaydı, zorunlu defter olayı ve yayın adımları arasında süreci durdur; yeniden başlat.
    - **Beklenen sonuç:** Başarılı görünen sonuç zorunlu denetim kaydından ayrı kalmaz; yarım işlemler uzlaştırılır.
    - **Otomasyon:** Evet — entegrasyon/kaos.

### Kira, kalp atışı ve yarışlar

13. **KT-013 — Eşzamanlı kira alma**
    - **Ön koşul:** Tek hazır shard ve çok sayıda yetkili işçi.
    - **Adımlar:** Aynı anda kira talep et.
    - **Beklenen sonuç:** Yalnız bir güncel sahip oluşur; diğerleri aynı yetkiyi alamaz.
    - **Otomasyon:** Evet — entegrasyon.

14. **KT-014 — Beş dakikalık kira sınırı**
    - **Ön koşul:** Kontrol edilebilir yetkili zaman; kalp atışı yok.
    - **Adımlar:** Kirayı oluştur; 300 saniyenin hemen öncesinde, tam sınırda ve sonrasında ayrı sonuç gönderimleri dene.
    - **Beklenen sonuç:** Önerilen sözleşmede yalnız sınır öncesi gönderim kabul edilebilir.
    - **Otomasyon:** Evet — birim/entegrasyon.

15. **KT-015 — 30 ve 60 saniyelik kalp atışı**
    - **Ön koşul:** Kalp atışı ve yenileme sözleşmesi kararlaştırılmış.
    - **Adımlar:** Ayrı koşularda 30 ve 60 saniye aralıklarla geçerli kalp atışı gönder.
    - **Beklenen sonuç:** Gereksiz iş kaybı oluşmaz; süre yalnız kararlaştırılan kuralla değişir.
    - **Otomasyon:** Evet — entegrasyon.

16. **KT-016 — Süresi dolmuş kiraya kalp atışı**
    - **Ön koşul:** Süresi dolmuş veya yeniden atanmış kira.
    - **Adımlar:** Eski belirteçle kalp atışı gönder.
    - **Beklenen sonuç:** Eski kira dirilmez; yeni sahibin süresi etkilenmez.
    - **Otomasyon:** Evet — entegrasyon.

17. **KT-017 — Belirtecin bağlam dışı kullanılması**
    - **Ön koşul:** İki cihaz, iki shard ve farklı denemeler.
    - **Adımlar:** Geçerli belirteci başka cihaz, shard veya denemeyle kullan.
    - **Beklenen sonuç:** Tüm bağlam uyuşmazlıkları reddedilir.
    - **Otomasyon:** Evet — entegrasyon.

18. **KT-018 — Eski belirtecin yeniden kullanılması**
    - **Ön koşul:** Aynı shard için yeni deneme ve yeni belirteç.
    - **Adımlar:** Önceki belirteçle sonuç ve kalp atışı gönder.
    - **Beklenen sonuç:** Hiçbiri güncel durumu değiştiremez.
    - **Otomasyon:** Evet — entegrasyon.

19. **KT-019 — Geç sonucun yeni sonucu ezmesi**
    - **Ön koşul:** İlk deneme zaman aşımına uğramış, sonraki deneme tamamlanmış.
    - **Adımlar:** İlk denemenin sonucunu gönder.
    - **Beklenen sonuç:** Yetkili çıktı değişmez; geç gönderim açıkça sınıflandırılır.
    - **Otomasyon:** Evet — entegrasyon.

20. **KT-020 — Yinelenen sonuç gönderimi**
    - **Ön koşul:** Kabul edilmiş sonuç ve idempotency kimliği.
    - **Adımlar:** Aynı isteği tekrarla; sonra aynı kimlikle farklı içerik gönder.
    - **Beklenen sonuç:** Aynı içerik tek mantıksal sonuç verir; farklı içerik çatışma olarak reddedilir.
    - **Otomasyon:** Evet — entegrasyon.

21. **KT-021 — Kira bitişi yarışı**
    - **Ön koşul:** Sonuç kabulü ve yeniden kiralama için eşzamanlılık kontrolü.
    - **Adımlar:** İşlemleri bitiş sınırında farklı sıralarla yarıştır.
    - **Beklenen sonuç:** Atomik sıraya göre tek yetkili kazanan oluşur.
    - **Otomasyon:** Evet — entegrasyon/kaos.

22. **KT-022 — Saat kayması**
    - **Ön koşul:** İşçi ve yetkili zaman ayrı kontrol ediliyor.
    - **Adımlar:** İşçi saatini ileri/geri değiştir; ayrıca yetkili zamanda geri sıçrama oluştur.
    - **Beklenen sonuç:** İşçi saati yetkiyi etkilemez; yetkili zaman belirsizliği eski sahipliği yeniden geçerli yapmaz.
    - **Otomasyon:** Evet — entegrasyon/kaos.

23. **KT-023 — Aktif kiralarla merkez yeniden başlatma**
    - **Ön koşul:** Geçerli ve süresi dolmak üzere olan kiralar.
    - **Adımlar:** Merkezi yeniden başlat; kalp atışı ve sonuç gönder.
    - **Beklenen sonuç:** Kalıcı sahiplik korunur; kiralar başlangıçtan itibaren tekrar 300 saniyeye çekilmez.
    - **Otomasyon:** Evet — entegrasyon/kaos.

### Zarf, anahtar ayrımı ve veri sınıfları

24. **KT-024 — A/B zarfının uçtan uca doğrulanması**
    - **Ön koşul:** Seçilmiş zarf biçimi, yetkili alıcı ve sentetik A/B belge.
    - **Adımlar:** Zarflama, taşıma, açma ve içerik doğrulama akışını çalıştır.
    - **Beklenen sonuç:** Yalnız hedef alıcı doğru içeriği açar; ağda açık belge görülmez.
    - **Otomasyon:** Evet — entegrasyon.

25. **KT-025 — Zarf ve üstveri değiştirme**
    - **Ön koşul:** Doğrulanmış şifreli zarf.
    - **Adımlar:** Şifreli içerik, bütünlük etiketi, alıcı, sınıf, iş ve shard bağlarını ayrı ayrı değiştir.
    - **Beklenen sonuç:** Her değişiklik reddedilir; doğrulanmamış içerik OCR’ye verilmez.
    - **Otomasyon:** Evet — birim/entegrasyon.

26. **KT-026 — İmza ve şifreleme anahtarlarının ayrılığı**
    - **Ön koşul:** Ayrı anahtar kimlikleri ve servis yetkileri.
    - **Adımlar:** Aynı anahtarı iki role atamayı; imza yetkisiyle zarf açmayı ve zarf yetkisiyle mühür üretmeyi dene.
    - **Beklenen sonuç:** Yapılandırma ve yetki düzeyinde rol karışması engellenir.
    - **Otomasyon:** Evet — birim/entegrasyon.

27. **KT-027 — Yanlış cihazın zarfı açması**
    - **Ön koşul:** Bir cihaza hazırlanmış zarf ve başka cihazın kimliği/anahtarı.
    - **Adımlar:** İkinci cihazdan indirme ve çözme dene.
    - **Beklenen sonuç:** Yetkisiz indirme ve çözme başarısız olur.
    - **Otomasyon:** Evet — entegrasyon.

28. **KT-028 — Tek anahtar sızıntısının sınırı**
    - **Ön koşul:** Tamamen sentetik anahtarlarla test ortamı.
    - **Adımlar:** Yalnız imza anahtarını bilen tarafla zarf açmayı; yalnız zarf anahtarını bilen tarafla mühür üretmeyi dene.
    - **Beklenen sonuç:** Bir anahtarın ele geçirilmesi diğer işlevi sağlamaz.
    - **Otomasyon:** Evet — birim/entegrasyon.

29. **KT-029 — C sınıfının hiç ağa çıkmaması**
    - **Ön koşul:** C olarak işaretli, benzersiz sentetik işaretleyicili belge; tüm ilgili çıkış kanalları gözleniyor.
    - **Adımlar:** Normal dağıtım, yeniden deneme, hata, destek ve telemetri yollarını tetikle.
    - **Beklenen sonuç:** C belge, türevi veya dağıtım zarfı yasaklanan hiçbir ağ sınırını geçmez.
    - **Otomasyon:** Evet — entegrasyon.

30. **KT-030 — Sınıf kapısını aşma**
    - **Ön koşul:** Güvenilir sınıflandırma kaydı ve yetkisiz istemci.
    - **Adımlar:** Eksik/bilinmeyen sınıf gönder; C etiketini A/B olarak değiştir; doğrudan kuyruk ekleme yolunu dene.
    - **Beklenen sonuç:** Güvenilir onay olmadan dağıtım gerçekleşmez; hata halinde dağıtım kapalı kalır.
    - **Otomasyon:** Evet — entegrasyon.

31. **KT-031 — Günlük ve telemetri sızıntısı**
    - **Ön koşul:** Sentetik içerik ve sahte belirteç işaretleyicileri.
    - **Adımlar:** Başarı, hata, zaman aşımı ve ayrıştırma hatalarını tetikle; günlük ve telemetriyi incele.
    - **Beklenen sonuç:** Açık belge, OCR metni, özel anahtar veya tam sahiplik belirteci bulunmaz.
    - **Otomasyon:** Evet — entegrasyon.

32. **KT-032 — İşçi geçici verisinin korunması**
    - **Ön koşul:** Belirlenmiş geçici alan ve saklama politikası.
    - **Adımlar:** Normal bitiş, çökme ve yeniden başlatmayı dene; düşük yetkili başka kullanıcıdan erişmeye çalış.
    - **Beklenen sonuç:** Yetkisiz erişim engellenir; kalıntılar politika kapsamında yönetilir; otomatik olarak “güvenli silindi” iddiası üretilmez.
    - **Otomasyon:** Evet — entegrasyon.

### Cihaz kimliği ve iptal

33. **KT-033 — İptal edilmiş cihazın yeni iş alması**
    - **Ön koşul:** Kayıtlı cihaz iptal edilmiş.
    - **Adımlar:** Yeni bağlantı ve mevcut bağlantı üzerinden iş talep et.
    - **Beklenen sonuç:** İki yolda da iş ve yeni zarf verilmez.
    - **Otomasyon:** Evet — entegrasyon.

34. **KT-034 — Aktif kira sırasında iptal**
    - **Ön koşul:** Aktif kira ve sonuç kabulünde kontrol edilebilir yarış.
    - **Adımlar:** Cihazı iptal et; kalp atışı ve sonuç kabulünü iptalle yarıştır.
    - **Beklenen sonuç:** İptalden sonra kesinleşen işlemler cihazı yetkili saymaz; eski kira yenilenmez.
    - **Otomasyon:** Evet — entegrasyon.

35. **KT-035 — Çevrimdışı cihazın geri dönmesi**
    - **Ön koşul:** Cihaz çevrimdışıyken iptal edilmiş; eski kimlik ve zarf bağlantıları mevcut.
    - **Adımlar:** Yeniden bağlan; eski bağlantıları, belirteçleri ve sonuçları kullan.
    - **Beklenen sonuç:** Sunucu güncel iptali uygular; eski yetkiler canlanmaz.
    - **Otomasyon:** Evet — entegrasyon.

36. **KT-036 — Yetkisiz kayıt ve cihaz kopyası**
    - **Ön koşul:** Cihaz kayıt yöntemi ve kimlik saklama politikası kararlaştırılmış.
    - **Adımlar:** Onaysız cihaz kaydet; kopyalanmış kimlikle eşzamanlı bağlantı kur.
    - **Beklenen sonuç:** Onaysız kayıt reddedilir; kopya kimlik için belirlenen tespit/engelleme politikası çalışır.
    - **Otomasyon:** Evet — entegrasyon.

### Kesintiler ve kurtarma

37. **KT-037 — OCR sırasında işçi çökmesi**
    - **Ön koşul:** İşlenmekte olan shard ve başka sağlıklı işçi.
    - **Adımlar:** İşçiyi test ortamında durdur; kira bitişini ve yeniden atamayı izle.
    - **Beklenen sonuç:** İş kaybolmaz; yarım çıktı yayınlanmaz; başka işçi tamamlayabilir.
    - **Otomasyon:** Evet — kaos.

38. **KT-038 — Ağ bölünmesi ve dönüş**
    - **Ön koşul:** Aktif işçi ve yeniden atama yapabilecek merkez.
    - **Adımlar:** Bağlantıyı kira süresinden uzun kes; yeniden atamadan sonra bağlantıyı aç.
    - **Beklenen sonuç:** Eski işçinin dönüşü yeni sahipliği veya sonucu bozamaz.
    - **Otomasyon:** Evet — kaos.

39. **KT-039 — Teslim alındı yanıtının kaybolması**
    - **Ön koşul:** Sonuç gönderimi ve yanıt kaybı enjeksiyonu.
    - **Adımlar:** Sonuç kesinleştikten sonra yanıtı düşür; işçinin aynı isteği tekrarlamasını sağla.
    - **Beklenen sonuç:** Tek yetkili sonuç ve tek mantıksal yayın oluşur.
    - **Otomasyon:** Evet — entegrasyon/kaos.

40. **KT-040 — Poison pill ve yeniden deneme bütçesi**
    - **Ön koşul:** Sürekli hata veren sentetik belge; deneme bütçesi kararlaştırılmış.
    - **Adımlar:** İşin tekrarlarını ve aynı kuyruktaki sağlıklı işleri izle.
    - **Beklenen sonuç:** Sorunlu iş bütçe sonunda karantinaya alınır; sağlıklı işler ilerler.
    - **Otomasyon:** Evet — entegrasyon.

41. **KT-041 — PostgreSQL yük devretme**
    - **Ön koşul:** Üretimle eşdeğer çoğaltma ve eski birincili dışlama düzeni.
    - **Adımlar:** Kira/sonuç commit’i sırasında bağlantı kaybı ve yük devretme oluştur; eski birincili geri getir.
    - **Beklenen sonuç:** Çift yazıcı oluşmaz; belirsiz işlemler uzlaştırılır; onaylı veri kaybı RPO sözleşmesini aşmaz.
    - **Otomasyon:** Evet — kaos.

42. **KT-042 — Merkez diskinin dolması**
    - **Ön koşul:** Ayrılmış test hacimleri ve doluluk enjeksiyonu.
    - **Adımlar:** Veritabanı, çıktı veya zorunlu defter yazımı sırasında alanı tüket.
    - **Beklenen sonuç:** Eksik işlem başarılı görünmez; geri basınç ve görünür hata oluşur.
    - **Otomasyon:** Evet — kaos.

43. **KT-043 — İşçi diskinin dolması**
    - **Ön koşul:** OCR geçici alanı sınırlandırılmış işçi.
    - **Adımlar:** Zarf indirme, açma ve çıktı üretme aşamalarında alanı tüket.
    - **Beklenen sonuç:** Kesilmiş çıktı kabul edilmez; hizmet hata bildirir ve kontrolsüz tekrar yapmaz.
    - **Otomasyon:** Evet — kaos.

44. **KT-044 — Eski yedekten geri yükleme**
    - **Ön koşul:** Bağımsız güncel kontrol noktası ve daha eski yedek.
    - **Adımlar:** Yedeği ayrı ortama yükle; cihaz iptali, kira ve defter durumunu doğrula.
    - **Beklenen sonuç:** Geri alma fark edilir; eski kimlik ve kiralar uzlaştırılmadan hizmete açılmaz.
    - **Otomasyon:** Evet — entegrasyon/kaos.

### Kötü niyetli iş, sonuç ve servis davranışı

45. **KT-045 — Kuyruk ve API kaynak tüketimi**
    - **Ön koşul:** Kararlaştırılmış boyut, hız ve kaynak sınırları.
    - **Adımlar:** Büyük istek, çok sayıda kira talebi ve yavaş bağlantılar üret.
    - **Beklenen sonuç:** Sınırlar uygulanır; kaynak kullanımı sınırlı kalır; sağlıklı işçiler belirlenen hizmet düzeyinde ilerler.
    - **Otomasyon:** Evet — yük/entegrasyon.

46. **KT-046 — OCR süreç yalıtımı**
    - **Ön koşul:** Zararsız saldırı benzetimleri ve kaynak sınırları.
    - **Adımlar:** Aşırı bellek/CPU tüketimi, beklenmeyen dosya erişimi ve dış bağlantı denemesi oluştur.
    - **Beklenen sonuç:** OCR sınırları aşamaz; merkez, hizmet kimliği ve diğer işler korunur.
    - **Otomasyon:** Evet — entegrasyon/kaos.

47. **KT-047 — Geçerli kimlikle sahte sonuç**
    - **Ön koşul:** Referans çıktılı sentetik OCR kümesi ve sonuç doğrulama politikası.
    - **Adımlar:** Geçerli kirayla yapısal olarak düzgün ama yanlış çıktı gönder.
    - **Beklenen sonuç:** Zorunlu doğrulanan örneklerde hata yakalanır; uyuşmazlık karantinaya gider; kimlik doğrulaması doğruluk kanıtı sayılmaz.
    - **Otomasyon:** Evet — entegrasyon.

48. **KT-048 — API yetkilendirmesi ve taşıma güvenliği**
    - **Ön koşul:** İşçi, yönetici ve yetkisiz istemci kimlikleri.
    - **Adımlar:** Başkasının işine eriş; yönetim uçlarını çağır; geçersiz sunucu/istemci kimliği ve güvenli olmayan taşıma dene.
    - **Beklenen sonuç:** Yetkisiz erişim ve güvensiz bağlantı reddedilir; geri dönüş yolu açılmaz.
    - **Otomasyon:** Evet — entegrasyon.

49. **KT-049 — Merkezin ele geçirilmesi tatbikatı**
    - **Ön koşul:** Merkez dışındaki güven sınırları ve savunulabilir güvenlik iddiaları belgelenmiş.
    - **Adımlar:** Merkez yetkisiyle C dağıtımı, yetkisiz imzalama ve çelişkili defter sunmayı dene.
    - **Beklenen sonuç:** Bağımsız kontroller vaat edilen sınırları korur; aşılan sınırlar açık bulgu olur.
    - **Otomasyon:** Kısmen — entegrasyon ve güvenlik tatbikatı.

50. **KT-050 — Anahtar sızıntısından kurtarma**
    - **Ön koşul:** Sentetik anahtarlar ve onaylı olay müdahale prosedürü.
    - **Adımlar:** Anahtarı ele geçirilmiş ilan et; kullanımını durdur; döndür; eski kimlikle yeni işlem dene.
    - **Beklenen sonuç:** Eski anahtar yeni yetki sağlamaz; tarihsel mühürler ve olası belge maruziyeti ayrı değerlendirilir.
    - **Otomasyon:** Kısmen — entegrasyon ve tatbikat.

51. **KT-051 — Eksik veya yanlış çıktı nesnesi**
    - **Ön koşul:** Çıktı referansı, beklenen boyut ve içerik özeti.
    - **Adımlar:** Nesneyi eksik, kesilmiş veya farklı içerikle sun; kabul sonrasında değiştirmeyi dene.
    - **Beklenen sonuç:** Bozuk çıktı yayınlanmaz; kabul edilmiş içerik özetiyle uyuşmazlık tespit edilir.
    - **Otomasyon:** Evet — entegrasyon.

52. **KT-052 — Yol, komut ve sorgu girdileri**
    - **Ön koşul:** Dosya adı, belge üstverisi ve iş parametreleri için saldırgan girdileri.
    - **Adımlar:** Yol aşımı, komut ayırıcıları, sorgu parçaları ve beklenmeyen Unicode değerleri gönder.
    - **Beklenen sonuç:** Yetkisiz dosya erişimi, komut çalıştırma veya sorgu yapısı değişimi oluşmaz.
    - **Otomasyon:** Evet — birim/entegrasyon.

53. **KT-053 — Sağlıksız işçinin kuyruğu tutması**
    - **Ön koşul:** Sürekli kira alıp sonuç üretmeyen bir işçi ve sağlıklı işçiler.
    - **Adımlar:** Davranışı birkaç kira döngüsü boyunca sürdür.
    - **Beklenen sonuç:** İşçi kota ve sağlık politikasıyla sınırlandırılır; işler sonsuza kadar tutulmaz.
    - **Otomasyon:** Evet — entegrasyon/kaos.

54. **KT-054 — Zarf anahtarı döndürme ve benzersizlik**
    - **Ön koşul:** Seçilmiş şifreleme algoritması ve anahtar/nonce üretim sözleşmesi.
    - **Adımlar:** Eşzamanlı zarflama, yeniden başlatma ve döndürme yap; eski/yeni zarfları denetle.
    - **Beklenen sonuç:** Algoritmanın benzersizlik şartları korunur; anahtar kimlikleri karışmaz; imza anahtarı döndürmeden etkilenmez.
    - **Otomasyon:** Evet — birim/entegrasyon.

## 5. Öncelik sırası ve açık sorular

### İlk uygulanacak 10 test

1. **KT-029 — C sınıfının hiç ağa çıkmaması:** Dağıtım yasağının uçtan uca kanıtı.
2. **KT-030 — Sınıf kapısını aşma:** Etiket değiştirerek yasağın delinmesini engelleme.
3. **KT-026 — Anahtar ayrılığı:** Tek sızıntının iki güvenlik işlevini birden yıkmasını önleme.
4. **KT-025 — Zarf ve üstveri değiştirme:** Belge ve iş bağlarının korunması.
5. **KT-021 — Kira bitişi yarışı:** Eşzamanlılıkta tek yetkili sonuç.
6. **KT-034 — Aktif kira sırasında iptal:** Ele geçirilmiş cihazın etkisini durdurma.
7. **KT-012 — Sonuç ve defterin birlikte kesinleşmesi:** Denetimsiz başarı oluşmaması.
8. **KT-047 — Geçerli kimlikle sahte sonuç:** OCR doğruluğunun ayrıca sınanması.
9. **KT-004 — Defter sonunun kesilmesi:** Geçerli eski geçmişe dönüşün tespiti.
10. **KT-041 — PostgreSQL yük devretme:** Kesinti sırasında sahiplik ve sonuç güvenliğinin korunması.

### Karar gerektiren açık sorular

- **C sınırı:** “Hiç ağa çıkmaz” merkez–işçi trafiğiyle mi sınırlı, yoksa yedek, telemetri, destek paketi ve bütün uzak depolama da yasak mı? Hangi üstveriler kapsama giriyor?
- **Sınıflandırma:** A/B/C kararını kim veriyor? Merkezden bağımsız, değiştirilemez veya ayrıca doğrulanan bir sınıflandırma kaynağı var mı?
- **Kira yenileme:** Her geçerli kalp atışı kirayı yeniden 300 saniyeye mi uzatıyor? Toplam çalışma süresinin üst sınırı olacak mı?
- **Zaman:** Kira için hangi zaman kaynağı yetkili? Veritabanı yük devretmesinde veya saat geri sıçramasında hangi güvenli duruş uygulanacak?
- **Cihaz iptali:** İptal merkezde kesinleştiğinde aktif kiralar hemen geçersiz mi? Kabul edilmiş ama henüz yayınlanmamış sonuçlar nasıl ele alınacak?
- **Cihaz kimliği:** Kimlik doğrulama yöntemi nedir? Özel anahtar donanım destekli ve dışarı aktarılamaz mı olacak? Klonlanmış kimlikten ne ölçüde korunma bekleniyor?
- **Zarf tasarımı:** Hangi doğrulanmış şifreleme biçimi kullanılacak? Alıcı, sınıf, iş, shard ve deneme hangi kriptografik bağlarla korunacak?
- **Anahtar yerleşimi:** İmza ve zarf anahtarları yalnız farklı değerler mi, yoksa farklı erişim yetkileri ve saklama sınırlarıyla da ayrılacak mı?
- **Mühür kapsamı:** İmzalanan mesaj; defter kimliği, ağaç boyutu, kök, sürüm ve anahtar kimliğinden hangilerini kapsayacak?
- **Bağımsız tanık:** Kökleri kim, nerede ve hangi sıklıkla saklayacak? Merkezden bağımsız erişim ve yönetim sağlanacak mı?
- **OCR doğruluğu:** Hangi hata oranı kabul edilecek? Hangi işler bağımsız tekrarlanacak? Uyuşmazlıkta yetkili sonucu kim belirleyecek?
- **PostgreSQL:** Çoğaltma modeli, RPO/RTO ve eski birincili dışlama yöntemi nedir? Başarı yanıtı verilmiş veri kaybı kabul ediliyor mu?
- **Çıktı saklama:** Çıktılar PostgreSQL’de mi, ayrı nesne deposunda mı? Değişmezlik ve defterle uzlaştırma nasıl sağlanacak?
- **İşçi sınırları:** Windows hizmeti hangi hesapla çalışacak? OCR’nin dosya sistemi, ağ, CPU, bellek ve süre yetkileri neler olacak?
- **Operasyon sınırları:** Deneme bütçesi, karantina, kuyruk kotası, işçi kotası ve ölçülebilir hizmet hedefleri nedir?
- **Geçici veri:** Açık içerik ne kadar tutulacak? Temizlik, çökme sonrası kalıntı ve bellek dökümleri için hangi politika uygulanacak?

aday not: Defter bütünlüğü ve cihaz kimliği, OCR sonucunun doğruluğundan ayrı kabul koşullarıyla sınanmalı. (kaynak: kullanıcı mimarisi ve bu yanıttaki tehdit analizi)

aday not: C dağıtım yasağı, kira atomikliği, aktif cihaz iptali ve bağımsız defter kontrol noktası ilk güvenlik kapıları olarak önerildi; uygulanmış veya doğrulanmış değildir. (kaynak: bu yanıttaki KT-029, KT-021, KT-034 ve KT-004)