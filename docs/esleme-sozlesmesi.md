<!-- Sahip: Yusuf. yazan: [Claude#sozlesme-EB0 2026-11-15]. Durum: SÖZLEŞME EKİ (EB0), Fable denetimi bekler. Değişiklik: Yusuf'un onayıyla. -->
# Eşleme sözleşmesi: aynı mağazada birden çok bilgisayar (EB0)

Kaynak: `planlar/PLAN-TAA-PAKET-ANA-2026-11-15.md` §16 (Yusuf 2026-11-15 15:19 UTC), §15 (TAA Kasa), §14 (çekme modeli, "şube bilgisayarında açık kapı olmaz"), §15 sonu Yusuf kararı (iki hal saklanır, insan seçer).
Bağlı belgeler: `docs/sube-sozlesmesi.md` (Ş0), `docs/sube-tehdit-modeli.md`, `docs/sube-izolasyon-testleri.md`, `docs/rehber-sozlesmesi.md`, `docs/modul-sozlesmesi.md`, `docs/ipc.md`, `docs/T7-KIMLIK.md`, `docs/S2-CIHAZ.md`. Yetenek: `docs/sozlesmeler/yazdir.sun.md`.
Bu belge Ş0'ın **ekidir**: Ş0'da yazan her kural geçerlidir; yalnız aşağıda adı geçen yerler genişler. Değişmesi gereken Ş0 satırları §1.5'te "DEĞİŞİKLİK ÖNERİSİ" olarak yazılıdır; Ş0 belgeleri bu görevde **düzenlenmedi**.

**Temel ilke:** tasarım Esenyurt'a özel değildir. Her mağazada 1 ya da daha çok bilgisayar olabilir. **Tek bilgisayarlı mağazada hiçbir şey değişmez:** eş yoktur, yerel ağ kapısı açılmaz, numara aralığı gerekmez, yazdırma doğrudan yereldedir.

## 1. Mağaza ve cihaz ayrımı

### 1.1 Terimler
| Terim | Anlam |
|---|---|
| Mağaza | Bu belgede **şube ile aynı şeydir**: 15 şubeden biri, kimliği `subeler.kod` (Ş0 §3). Esenyurt Outlet ayrı mağaza değildir, `istanbul-merkez` altındadır. AÇIK-E1 (rehberdeki `tur = magaza` ile ad çakışması) |
| Mağaza bilgisayarı | Bir mağazaya Çatı'dan bağlanmış onaylı cihaz (`taa_cihazlar.sube_id = <mağaza>`). Bir mağazanın 1..n bilgisayarı olabilir |
| Eş bilgisayar (eş) | **Aynı mağazaya bağlı, onaylı, iptal edilmemiş** öteki bilgisayar. Eşlik Çatı'daki kayıttan türer; bilgisayar kendini eş ilan edemez. Merkez cihazı (`sube_id = null`) hiçbir mağazanın eşi değildir |
| Eş listesi | Bir mağazanın onaylı bilgisayarları ve açık cihaz anahtarları; sunucu imzalı, her eşte önbellekte (§5.2) |
| Yazıcı sahibi | Bir yazıcının bağlı olduğu ve o yazıcı için yazdırma hizmeti sunan eş (`yazdir.sun@1`) |
| Sıra tutan | Mağazanın numaralı belge sırasını tutan tek eş (§3) |
| Eş kapısı | Eşlerin yerel ağda birbirine bağlandığı tek kapı (§5). Yazdırma ve eşleme aynı kapıyı kullanır |

### 1.2 Neye kim sahiptir
| Şey | Sahibi | Not |
|---|---|---|
| Şube kimliği (`kod`), önek, şube adı | **Mağaza** | Tüm eşlerde aynı; Ş0 §3 |
| Şube imzası (belgeye "bu mağazanın belgesi" damgası) | **Mağaza** | Doğrulama mağazanın geçerli anahtar kümesiyle yapılır (§1.3) |
| Şube imza anahtarı (özel) | **Her eşin kendisi** | Her eş kendi anahtarını üretir; özel anahtar o bilgisayardan çıkmaz (Ş0 §7.1 aynen) |
| Cihaz anahtarı (ed25519, istek imzası) | **Her bilgisayarın kendisi** | taa-cihaz; S2-CIHAZ |
| Çevrimdışı yönetici belgesi | Kullanıcı + **tek cihaz** | Ş0 §4.1 aynen; her eş için ayrı alınır |
| İşlem kayıtları (fiş, fiş olayı, tutanak, ciro, kasa, masraf) | **Mağaza** | Kaydı hangi eş yazdıysa `kaynak_cihaz` alanında durur |
| Yerel defter, kuyruk, önbellek | Her bilgisayarın kendisi | DPAPI ile sarılı (Ş0 §8) |

### 1.3 Şube imzası birden çok bilgisayarda
- Bir mağazada **birden çok geçerli şube imza anahtarı** olabilir: eş başına bir tane. Her biri ayrı başvurulur, Çatı'dan ayrı onaylanır, ayrı iptal edilir.
- Anahtar paylaşılmaz, bilgisayarlar arasında kopyalanmaz. "Mağazaya ait" demek: belge mağaza adına imzalanır ve mağazanın geçerli anahtar kümesindeki herhangi biriyle doğrulanır.
- Ş0 §7.3 kabul şartı aynen kalır, yalnız madde 2 "o şubenin geçerli anahtarlarından biriyle" olur. Madde 1 ("aynı şubeye bağlı onaylı cihazdan") eşin, ötekinin imzaladığı belgeyi merkeze iletmesine izin verir; sunucu `no` ile tek kayıt üretir (Ş0 §9.1).
- Bir eşin anahtarının iptali ötekileri etkilemez.

### 1.4 Veri görünürlüğü
1. Eşler **aynı mağaza verisini** görür: bir bilgisayarda açılan fiş, tutanak, kasa sayımı, ciro ve masraf ötekinde de görünür (§4).
2. **Başka mağazanın verisi yine görünmez.** Eş listesi yalnız aynı `sube_id`'den oluşur; eşten gelen başka şube damgalı kayıt reddedilir ve deftere yazılır (T30).
3. Ayarlar, şablonlar, fiyat listesi ve rehber eşten alınmaz; her bilgisayar havuzdan ve sunucudan kendisi alır (Ş0 §6 iki kanal aynen). Eş kapısı havuz kanalı değildir.
4. Kişi yetkisi değişmez: kişinin o mağazada rolü varsa her eşte aynı izinlerle çalışır (Ş0 §4.3). Eş kapısı kişi yetkisi vermez; yalnız cihazlar arasıdır.
5. KVKK notu: müşteri verisinin kopyası artık mağazadaki her eşte durur. Her kopya DPAPI ile sarılıdır; PC devri ve çalınma kuralları her eş için ayrı işler.

### 1.5 Ş0 "şubede tek PC" varsayımları: değişenler ve DEĞİŞİKLİK ÖNERİSİ
Seçim: Ş0 belgeleri (`sube-sozlesmesi.md`, `sube-tehdit-modeli.md`, `sube-izolasyon-testleri.md`) bu görevde **düzenlenmedi**; tüm düzeltmeler aşağıda öneri olarak yazıldı. Yusuf onayından sonra tek PR ile işlenmesi önerilir.

**`docs/sube-sozlesmesi.md`**
| # | Yer | Bugünkü varsayım | DEĞİŞİKLİK ÖNERİSİ |
|---|---|---|---|
| D1 | §1 "Şube PC'si" | "Şubedeki tek kayıtlı bilgisayar (15 şube × 1 PC)" | "Şube bilgisayarı: şubeye bağlı onaylı bilgisayarlardan biri (1 ya da daha çok). Aynı şubenin bilgisayarları eştir (`docs/esleme-sozlesmesi.md`)." |
| D2 | §1 "Merkez Mağaza" | Sunucu PC'siyle ilişkisi belirsiz | "… Esenyurt'ta sunucudan ayrı 2 kasa bilgisayarı vardır (ana plan §16.1)." AÇIK-S2 kapanır |
| D3 | §1 "Şube imza anahtarı" | Tek anahtar | "… her eş bilgisayarın kendi şube imza anahtarı vardır; hepsi şube adına geçerlidir." |
| D4 | §2 kural 3 | "şube PC'si sunucu olmadan çalışır" | "her şube bilgisayarı sunucu ve eşi olmadan çalışır" |
| D5 | §3 önek tablosu | "Aşağıdaki önekler öneridir (AÇIK-S1)", sütun "(öneri)" | "Önekler kesindir (Yusuf 2026-11-15, ana plan §16.1); pilot başladıktan sonra değişmez." Sütun başlığı `onek`. AÇIK-S1 kapanır |
| D6 | §4 madde 1 | Tek cihaz varsayılır | Ek cümle: "Bir şubeye birden çok cihaz bağlanabilir; Çatı şube başına cihaz listesini gösterir (EB1)." |
| D7 | §4 madde 7 (PC devri) | Tek PC | Ek: "Devredilen PC eski şubenin eş listesinden düşer; elindeki yedek numara aralığının kullanılmamış kısmı atlandı notuyla kapanır (`esleme-sozlesmesi` §3)." |
| D8 | §5 tablo "Fiş, fiş olayı" | "Sahibi şube PC'si … merkeze yalnız şube PC'si yazar" | "Sahibi şube; şubenin onaylı bilgisayarlarından herhangi biri yazar, kayıtta `kaynak_cihaz` durur. Telefonlar yalnız kayıtlı telefon sunucusu eşe bağlanır (AÇIK-E8)." |
| D9 | §5 tablo "Şube imza anahtarı kaydı" | "Şube PC'si başvurur" | "Her şube bilgisayarı kendi anahtarıyla başvurur" |
| D10 | §6 | İki kanal | Ek satır: "Eş kanalı (aynı şubenin bilgisayarları arası, yerel ağ) yalnız o şubenin işlem kayıtlarını, yazdırma işlerini ve numara isteklerini taşır; havuz içeriği taşımaz." |
| D11 | §7.1 "Saklama" | "Şube PC'sinde üretilir" | "Her şube bilgisayarında ayrı üretilir; …" (geri kalanı aynen) |
| D12 | §7.3 madde 2 | "o şubenin geçerli şube imza anahtarıyla" | "o şubenin geçerli şube imza anahtarlarından biriyle" |
| D13 | §7.5 iptal | Tek anahtar | Ek: "Bir bilgisayarın anahtarının iptali şubenin öteki anahtarlarını etkilemez." |
| D14 | §8 son madde | "o cihaza aittir" | Ek: "Aynı şubenin işlem kayıtları eşlemeyle her eşte de bulunur; her kopya DPAPI ile sarılıdır." |
| D15 | §9.4 | Eş ve yazıcı uçları yok | EB1 uçları (öneri) eklenir: §7.1 tablosu |
| D16 | §10 | "Sıra: … cihaz sıra numarası" | Ek: "Cihaz sıra numarası her cihaz için ayrıdır; eşten gelen çakışma da aynı kuralla iki hal olarak saklanır (`esleme-sozlesmesi` §4.3)." |

**`docs/sube-tehdit-modeli.md`**
| # | Yer | DEĞİŞİKLİK ÖNERİSİ |
|---|---|---|
| D17 | §1 Varlıklar | V12 eş listesi (sunucu imzalı), V13 yazdırma kuyruğu, V14 numara sırası ve yedek aralıklar eklenir |
| D18 | §3 Saldırı yüzeyi | "Şube PC'si" maddesine "eş kapısı (yalnız çok bilgisayarlı mağazada, yalnız yerel ağ)" eklenir |
| D19 | §4 | T23–T30 eklenir (bu belge §5.4) |
| D20 | §8 | AÇIK-S1 ve AÇIK-S2 "kapandı (ana plan §16.1)" diye işaretlenir |

**`docs/sube-izolasyon-testleri.md`**
| # | Yer | DEĞİŞİKLİK ÖNERİSİ |
|---|---|---|
| D21 | §1 Düzenek | A şubesinde **iki** onaylı cihaz (A1, A2) olur; B'de bir cihaz kalır (tek bilgisayarlı durum da sınanır) |
| D22 | §4 Matris | Yeni satırlar: `sube_yazicilari`, `numara_araliklari`, eş listesi ucu (okuma, yazma, liste, sayım, günlük, hata) |
| D23 | §5 | Ek senaryolar 13–16: `izolasyon_es_baska_magaza`, `izolasyon_es_listesi_capraz`, `izolasyon_yazici_capraz`, `izolasyon_numara_capraz` (bu belge §6.1) |

## 2. Ortak yazıcı ve `yazdir.sun@1`
Ayrıntılı sözleşme: `docs/sozlesmeler/yazdir.sun.md`. Özet:
- Programlar (TAA Etiket, Tutaa, Terzi Takip, TAA Kasa) yazdırmayı **her zaman kendi bilgisayarlarındaki yazdırma hizmetine** `taa-ipc` ile ister. Hizmet, yazıcı kaydına bakar: yazıcı bu bilgisayardaysa basar, eşteyse işi eş kapısından yazıcı sahibine iletir. Kullanıcı fark görmez.
- Yazıcı türleri: `etiket`, `fis`, `a4`. Hangi yazıcının hangi eşe bağlı olduğu kurulumda kaydedilir, Çatı'dan değiştirilir (EB1, EB4).
- Her işin **tek kimliği** vardır; aynı kimlik ikinci kez basılmaz.
- Yazıcı sahibi kapalıysa iş kuyrukta bekler, ekranda açıkça yazıcının bağlı olduğu bilgisayarın kapalı olduğu ve işin kaybolmadığı yazar (bağlayıcı metin `yazdir.sun.md` "Ekran mesajları").
- Sunucu ve internet gerekmez; yalnız mağazanın yerel ağı gerekir.
- Windows yazıcı paylaşımı yalnız A4 için yedek yoldur; etiket ve fiş için asıl yol bu hizmettir (ana plan §16.4).

## 3. Belge numarası (fiş, tutanak)
### 3.1 Biçim
- `<ONEK>-<6 hane>`, ör. `MRK-000123`. Önek mağazanın kodundan gelir (Ş0 §3 tablosu, **AÇIK-S1 kesinleşti**, ana plan §16.1). Önek elle seçilmez, cihazın onaylı şubesinden türer.
- Her numaralı belge türünün (fiş, tutanak) kendi sırası vardır. Tür listesi Ş4 ve Ş5'te kesinleşir.

### 3.2 Sıra tutan
- Mağazada her belge türünün sırasını **tek eş** tutar: sıra tutan. Varsayılan: mağazaya ilk kurulan bilgisayar. Değişiklik yalnız Çatı'dan (EB1).
- Öteki eş numarayı sıra tutandan eş kapısıyla ister (`numara.al`, istek kimliğiyle; aynı istek kimliği aynı numarayı döner, yeniden deneme çift numara üretmez).
- Sıra tutan verdiği her numarayı (kime, ne zaman, istek kimliği) kendi defterine yazar; defter eşlemeyle öteki eşe ve merkeze gider.
- Tek bilgisayarlı mağazada bilgisayarın kendisi sıra tutandır; bugünkü davranış aynen sürer.

### 3.3 Yedek aralık (sıra tutana ulaşılamazsa)
- Sıra tutan, her eşe **önceden** kendi yedek aralığını verir; aralık sıra tutanın defterine ve merkeze yazılır. Öneri (Ş4'te kesinleşir, AÇIK-E5): ana sıra `000001–799999`; yedek bölge `800000–999999`, eş başına 1.000'lik bloklar.
- Eş, sıra tutana ulaşamadığında (yanıt yok ya da 3 sn zaman aşımı) yedek aralığından sıradaki numarayı kullanır; ekranda küçük bir "yedek numara" işareti görünür, iş durmaz.
- Bağlantı dönünce eş kullandığı yedek numaraları bildirir. Blok dolmak üzereyken (%80) yeni blok alır; blok yalnız sıra tutana bağlıyken verilir.
- Hiç bağlantı yokken yedek aralık biterse eş yeni numara **üretmez**; ekranda "Numara aralığı bitti. Öteki bilgisayarı açın ya da yöneticiyi arayın." yazar. Fiş taslak olarak saklanır, numara bağlantı dönünce verilir.

### 3.4 Tekillik ve atlama
- Numara her durumda tekildir: ana sıra yalnız sıra tutandan, yedek numara yalnız o eşe verilmiş bloktan çıkar. Sunucu aynı `no` için farklı içerik görürse Ş0 §10 gereği 409 `cakisma` ile iki hali saklar.
- **Sıra yalnız kesintide atlayabilir:** sıra tutan numarayı verdi ama yanıt eşe ulaşmadıysa ya da yedek blokta kullanılmamış numara kalıp blok kapandıysa. Her atlama deftere `numara.atlandi` olayı olarak yazılır (tür, aralık, neden, cihaz, zaman) ve Çatı'da görünür. Sessiz atlama yoktur.
- Ana sıra ile yedek numaralar zaman sırasında karışık görünebilir (yedek numara daha sonra yazılmış olabilir); sıralama belge tarihine göre yapılır, numaraya göre değil.

### 3.5 Sıra tutan devri
- Sıra tutan bilgisayar bozulursa ya da değişirse devir **yalnız Çatı'dan** yapılır; otomatik seçim yoktur (iki bilgisayar aynı anda sıra tutan olamaz).
- Yeni sıra tutanın başlangıcı: bilinen en büyük ana numara (eş defterleri + sunucu) + güvenlik payı. Aradaki numaralar `numara.atlandi` olarak yazılır. Güvenlik payı değeri AÇIK-E5.

## 4. Eşleme (soyut protokol)
Bayt düzeyi biçim EB3'te kesinleşir; bu bölüm davranışı bağlar.

### 4.1 Kayıt kimliği ve sürüm
Her eşlenen kayıt şunları taşır: `kimlik` (uuid), `sube_kodu`, `kaynak_cihaz` (cihaz kimliği), `cihaz_sira` (o cihazın artan sayacı, Ş0 §4.1), `surum`, `onceki_ozet` (önceki sürümün sha256'sı; ilk sürümde boş), `ozet`. Ş0 §2 kural 4 aynen: silme ve yerinde değiştirme yok, değişiklik yeni sürüm ya da olaydır.

### 4.2 İki yol
1. **Eşten eşe, yerel ağda doğrudan:** her eş, öteki eşin her cihaz için son gördüğü `cihaz_sira`'dan sonrasını ister ("bende Kasa-1'den 418'e kadar var"). Yeni kayıt olunca eş kısa bir "yeni var" bildirimi yollar; karşı taraf çeker. Bağlantı yoksa birikir, dönünce kaldığı yerden sürer.
2. **Merkeze, her zamanki gibi:** her eş kendi kuyruğunu sunucuya gönderir (T6, T7). Eş ayrıca ötekinden aldığı ama merkezde makbuzu olmayan kaydı iletebilir (§1.3); sunucu `no`/`olay_id` ile tek kayıt üretir.
3. Yerel ağ yoksa ama internet varsa eş, öteki eşin kayıtlarını sunucudan (kendi şubesinin liste uçları, Ş0 §9.4) alabilir. Aynı kayıt iki yoldan gelirse `kimlik + surum + ozet` ile tekilleşir.
- Eşleme yalnız işlem kayıtlarını taşır (fiş, fiş olayı, tutanak, tutanak eki, ciro, kasa gün sonu, masraf, numara defteri, yazdırma iş durumu). Havuz, ayar, rehber ve anahtarlar taşınmaz.
- Eşten gelen her kayıt alınmadan önce denetlenir: `sube_kodu` = kendi şubesi, `kaynak_cihaz` eş listesinde, şema geçerli. Uymayan kayıt uygulanmaz, `es.red` olayıyla deftere yazılır.

### 4.3 Aynı kayıt iki bilgisayarda değişirse
- Aynı `onceki_ozet`'ten iki farklı yeni sürüm doğarsa bu **çakışmadır**. Yusuf kararı uygulanır: **iki hal de saklanır, hangisinin geçerli olacağını insan seçer; otomatik birleştirme ve "son yazan kazanır" yoktur** (ana plan §15 sonu, Ş0 §10).
- Çakışma kaydı her iki eşte ve merkezde aynı biçimde görünür. Seçimi şube müdürü ya da yönetici yapar; seçim de bir olaydır (`cakisma.secim`) ve iki yöne eşlenir. Seçim yapılana kadar kayıt ekranda "iki hali var" işaretiyle durur, iki hal yan yana gösterilir.
- İki kişi aynı çakışmayı farklı seçerse yeni bir çakışma doğar; aynı kuralla çözülür.
- Ekleme tipi olaylar (fiş olayı, not, masraf satırı) birbirini ezmez, ikisi de eklenir; çakışma yalnız aynı kaydın sürümünde doğar.

### 4.4 "Şu an öteki bilgisayarda açık" uyarısı
- Bir kayıt düzenleme için açılınca eş, ötekine `acik` bildirimi yollar (kayıt kimliği, bilgisayar adı, kullanıcının görünen adı, zaman). Bildirim 30 sn'de bir yenilenir; 2 dk yenilenmezse düşer.
- Öteki bilgisayarda aynı kayıt açılınca: "Bu kayıt şu an <Kasa-1> bilgisayarında açık (<ad>). Değiştirirseniz kaydın iki hali oluşabilir." Uyarı **kilit değildir**; kullanıcı devam edebilir.
- Dürüst sınır: yerel ağ kopukken uyarı gelmez; o durumda çakışma §4.3 ile yakalanır.

### 4.5 Programlara özel notlar
- **TAA Kasa:** gün sonu bir mağaza ve gün için tek kayıttır; iki eşten girilirse çakışma olur, insan seçer. Esenyurt'ta iki ayrı kasa çekmecesi varsa gün sonunun cihaz başına mı mağaza başına mı tutulacağı AÇIK-E6 (KS1'i etkiler).
- **Ciro:** mağaza ve gün başına bir kayıt (Ş0 §5); aynı kural.
- **Telefonlar:** telefon yerel sunucusu (taa-fis-sunucu) mağazada tek eşte çalışır; telefon fişleri o eşin defterine yazılır ve eşlenir. Hangi eş: AÇIK-E8.

## 5. Yerel ağ güvenliği
### 5.1 Eş kapısı kuralları
1. **Yalnız çok bilgisayarlı mağazada açılır.** Eş listesinde başka bilgisayar yoksa kapı dinlemez.
2. **Yalnız mağaza ağı:** kapı yalnız yerel ağ arayüzünde dinler. TAA VPN / Tailscale arayüzünde (100.64.0.0/10) ve genel ağ profilinde dinlemez. Bağlanan adres yerel alt ağda değilse bağlantı kimlik denetimine geçmeden kapanır.
3. **Yalnız aynı mağazanın kayıtlı eşi:** iki uç da birbirinin açık cihaz anahtarını kendi eş listesinde bulmalıdır (karşılıklı kimlik doğrulama). Bağlantı el sıkışması cihaz anahtarıyla, alan ayrımı etiketi `TAA-ES-v1` ile imzalanır; böylece eş imzası sunucu istek imzası ya da belge imzası yerine kullanılamaz (Ş0 §7.2 ilkesi).
4. **Şifreli bağlantı:** tüm trafik şifreli; oturum anahtarı her bağlantıda yeniden üretilir (ileri gizlilik); karşı tarafın anahtarı eş listesindeki anahtara sabitlenir. Düz metin kip yoktur. Kitaplık seçimi (ör. karşılıklı sertifikalı TLS 1.3 ya da Noise) yeni bağımlılıktır, Yusuf onayı ister: AÇIK-E9.
5. **Mesaj tekrarı reddedilir:** her mesajda oturum içi artan sayaç.
6. **İnternete ve başka mağazaya kapalı:** başka mağazanın onaylı cihazı da (eş listesinde olmadığı için) reddedilir. Modemde kapı yönlendirmesi yapılmaz, TAA Kur yapmaz.
7. **Güvenlik duvarı kuralını TAA Kur kurar (EB4):** gelen yön, yalnız eş hizmetinin exe'si, yalnız tek sabit TCP kapısı (numara EB2'de seçilir), yalnız Özel ağ profili, uzak adres yalnız yerel alt ağ. Ağ profili "Genel" ise TAA Kur kuralı kurmaz ve "Bu ağ Genel olarak işaretli; eş bilgisayar bağlantısı kapalı kalacak" der. Eş çıkarılınca kural kaldırılır.
8. **Hız ve boyut sınırları:** eş başına iş/istek sınırları `yazdir.sun.md` ve §3'te; aşımda istek reddedilir, deftere yazılır.

### 5.2 Eş listesi
- Kaynak: sunucu. Liste mağaza kodu, cihaz kimlikleri, açık anahtarlar, cihaz adları, rol işaretleri (yazıcı sahibi, sıra tutan, telefon sunucusu), liste sürümü ve zamanından oluşur; sunucu anahtarıyla imzalıdır.
- Her eş listeyi önbellekte tutar; imzası tutmayan liste kullanılmaz (önceki geçerli liste sürer).
- Çatı'dan cihaz iptal edilince yeni liste yayımlanır; eşler bir sonraki sunucu bağlantısında alır ve iptal edilen cihazla bağlantıyı keser.
- **Dürüst sınır:** mağaza internetten tamamen kopuksa iptal edilen bir cihaz, eşlerin listesi yenilenene kadar yerel ağda eş sayılmaya devam eder (Ş0'daki 7 günlük çevrimdışı gecikme ile aynı türden). Öneri: 7 günden eski liste ile eş kapısı yalnız okuma ve yazdırmaya izin verir; numara bloğu vermez (AÇIK-E10).

### 5.3 "Şube bilgisayarında açık kapı olmaz" kuralına dar istisna
Ana plan §14 kuralı ("Şube bilgisayarında dışarıya açık kapı olmaz") geçerli kalır. Eş kapısı bilinçli ve dar bir istisnadır (ana plan §16.2 madde 6), şu sınırların **hepsi** birlikte sağlanırsa:
- yalnız çok bilgisayarlı mağazada, yalnız yerel ağ arayüzünde, yalnız Özel profilde;
- yalnız aynı mağazanın eş listesindeki cihaza, karşılıklı kimlik ve şifreli bağlantıyla;
- yalnız bu belgede sayılan işler (yazdırma, eşleme, numara, açık bildirimi); başka uç yoktur;
- internet, VPN ve başka mağaza yönünden kapalıdır.
Bunlardan biri sağlanamıyorsa kapı açılmaz; mağaza tek bilgisayarlı gibi çalışır (yazdırma yalnız yerel, eşleme yalnız merkez yoluyla).
Not: Ş0 tehdit modeli §3'teki telefon eşleştirme yerel sunucusu zaten yerel ağda dinleyen bir kapıdır. İkisinin tek kapıda mı ayrı mı duracağı AÇIK-E8.

### 5.4 Tehdit modeli eki
| # | Tehdit | Saldırgan | Önlem | Test adı | Kalan risk |
|---|---|---|---|---|---|
| T23 | **Sahte eş cihaz**: yerel ağda kendini eş gibi tanıtan bilgisayar | S1, S4 | Karşılıklı kimlik; eş listesi sunucu imzalı; `TAA-ES-v1` etiketli el sıkışma; iptal edilen cihaz listeden düşer | `es_kayitsiz_cihaz_red`, `es_baska_magaza_cihazi_red`, `es_iptal_cihaz_red`, `es_liste_imza_bozuk_red`, `es_imza_etiket_karisma_red` | Kopuk mağazada iptal gecikmesi (§5.2) |
| T24 | **Yerel ağda dinleme / araya girme** | S4 | Şifreli bağlantı, ileri gizlilik, anahtar sabitleme; düz metin kip yok | `es_baglanti_sifresiz_red`, `es_araya_girme_red`, `es_ag_dokumu_duz_metin_yok`, `es_mesaj_tekrar_red` | Trafik miktarı ve zamanı görünür (içerik görünmez) |
| T25 | **Yazdırma kuyruğuna sahte iş** | S1, S4, S7 | Eşten yalnız kimliği doğrulanmış eş; her iş kim, hangi cihaz, hangi belge ile deftere; eş başına hız ve kuyruk sınırı; içerik türü ve boyut sınırı | `yazdir_kayitsiz_es_red`, `yazdir_hiz_siniri`, `yazdir_kuyruk_dolu`, `yazdir_bicim_red`, `yazdir_is_deftere_yazilir` | Aynı bilgisayarda aynı kullanıcıyla çalışan kötü niyetli süreç yerel IPC'den iş gönderebilir (`docs/ipc.md` güvenlik sınırı) |
| T26 | **Numara aralığı tüketme**: bozuk ya da kötü niyetli eşin sürekli numara/blok istemesi | S2, S7 | Blok yalnız önceki blok %80 dolunca; eş başına numara isteği hız sınırı; her verilen numara ve blok defterde; atlama eşiği aşılınca Çatı uyarısı | `numara_blok_tukenme_siniri`, `numara_istek_hiz_siniri`, `numara_atlama_deftere_yazilir`, `numara_ayni_istek_ayni_no` | Sınır içinde kalan yavaş tüketme; Çatı uyarısıyla görülür |
| T27 | Eş kapısına internetten, VPN'den ya da başka alt ağdan erişim | S6, S1 | Yalnız yerel arayüz; güvenlik duvarı Özel profil + yerel alt ağ; uygulama içi adres denetimi | `es_kapi_tailscale_kapali`, `es_kapi_genel_profil_kapali`, `es_kapi_alt_ag_disi_red`, `es_tek_pc_kapi_kapali` | Mağaza modemine izinsiz bağlanan cihaz kimlik denetimine kadar ulaşır (T23 ile durur) |
| T28 | Eşleme ile başka mağaza verisinin gelmesi ya da gitmesi | S2 | Kayıt şube damgası denetlenir; başka şube damgalı kayıt uygulanmaz; PC devrinde eş listesinden düşme | `es_baska_sube_damgali_kayit_red`, `es_pc_devri_baglanti_kesilir` | — |
| T29 | Çift basma (bütünlük) | — | Tek iş kimliği; yazıcı sahibinde basılan işler defteri; "basılıyor" iken çöküşte otomatik yeniden basma yok | `yazdir_cift_basma_yok`, `yazdir_yeniden_gonderim_tek_baski`, `yazdir_cokus_belirsiz_durum` | Çöküşte "basıldı mı" belirsiz kalabilir; kullanıcıya sorulur |
| T30 | Eşten gelen kaydın sahte sürümle ezilmesi | S2 | Eklemeli kayıt; aynı öncülden iki sürüm = çakışma, iki hal saklanır; otomatik seçim yok | `es_cakisma_iki_hal_saklanir`, `es_son_yazan_kazanmaz` | Şube müdürü yanlış hali seçebilir (insan kararı, denetim izinde) |

Saldırgan kodları (S1–S7) Ş0 tehdit modeli §2'dendir.

## 6. Kabul testleri
### 6.1 Otomatik (EB1–EB4 bunları geçirmeden birleşmez)
1. `tek_pc_degismez`: tek bilgisayarlı mağazada kapı açılmaz, yazdırma doğrudan yerel, numara bugünkü gibi.
2. `cok_cihaz_kayit`: bir şubeye iki cihaz onaylanır; eş listesi iki cihazı içerir; merkez cihazı listede yoktur.
3. `sube_imza_iki_anahtar`: A1 ve A2 anahtarlarının ikisiyle imzalı belge kabul edilir; A1 anahtarı iptal edilince A2 imzası geçer, A1 imzası reddedilir.
4. `es_belge_iletimi`: A2, A1'in imzaladığı belgeyi iletir; sunucu tek kayıt üretir.
5. `numara_tekil_iki_es`: iki eş aynı anda 1.000 numara ister; tekrar yok.
6. `numara_yedek_aralik`: sıra tutan kapalıyken eş yedek aralıktan numara verir; bağlantı dönünce bildirir; atlama deftere yazılır.
7. `numara_aralik_bitti`: yedek aralık biterse numara üretilmez, taslak saklanır, mesaj gösterilir.
8. `numara_ayni_istek_ayni_no`: yeniden denenen istek aynı numarayı alır.
9. `yazdir_*`: `docs/sozlesmeler/yazdir.sun.md` §Kabul testleri.
10. `es_cakisma_iki_hal_saklanir`, `es_son_yazan_kazanmaz`, `es_cakisma_secim_eslenir`.
11. `es_acik_uyarisi`: kayıt A1'de açıkken A2'de uyarı görünür; 2 dk sonra düşer; uyarı düzenlemeyi engellemez.
12. Tehdit testleri: §5.4 tablosundaki tüm test adları.
13. İzolasyon ekleri: `izolasyon_es_baska_magaza` (B cihazı A'nın eş kapısına bağlanamaz), `izolasyon_es_listesi_capraz` (A, B'nin eş listesini alamaz: 404), `izolasyon_yazici_capraz` (A, B'nin yazıcı kaydını göremez, değiştiremez), `izolasyon_numara_capraz` (A, B'nin numara defterini göremez).

### 6.2 EB5 gerçek deneme (Esenyurt, iki bilgisayar; P0 öncesi)
Ön koşul: ön pilot önce tek bilgisayarlı bir mağazada yapılır, Esenyurt hemen ardından (ana plan §16.4). Gerçek yazıcıyla denenmeden "çalışıyor" denmez. Sonuç `docs/olcum/EB5-<tarih>.md`.
| # | Senaryo | Beklenen |
|---|---|---|
| G1 | İki bilgisayardan art arda fiş açma | Numaralar tekil, önek `MRK`; ikisi de iki ekranda görünür |
| G2 | Yazıcısız bilgisayardan fiş, etiket ve A4 yazdırma | Doğru yazıcıdan çıkar; kullanıcı fark görmez |
| G3 | Yazıcılı bilgisayar kapalıyken yazdırma | Ekranda "yazıcının bağlı olduğu bilgisayar kapalı" mesajı; iş kuyrukta; bilgisayar açılınca **bir kez** basılır |
| G4 | Yazıcılı bilgisayar kapalıyken çalışmaya devam (fiş, kasa) | Öteki bilgisayar çalışır; sıra tutan kapalıysa yedek numara kullanılır |
| G5 | Ağ kablosu çekilir (iki bilgisayar arası kesinti) 10 dk, sonra takılır | İkisi de çalışır; dönünce kayıtlar eşlenir; atlama varsa defterde |
| G6 | İnternet kesik, yerel ağ var | Yazdırma ve eşleme sürer; merkez kuyruğu birikir, sonra gider |
| G7 | Aynı fişte iki bilgisayarda aynı anda değişiklik | "Öteki bilgisayarda açık" uyarısı; ısrar edilirse iki hal saklanır, müdür seçer |
| G8 | Yazdır tuşuna çift basma; gönderim sırasında ağ kopması | Tek baskı |
| G9 | Başka mağazaya kayıtlı bir test bilgisayarı aynı ağa takılıp bağlanmayı dener | Reddedilir, deftere yazılır |
| G10 | Çatı'dan fiş yazıcısının sahibini öteki bilgisayara almak | Yeni işler yeni sahipten çıkar; bekleyen işler kaybolmaz, çift basılmaz |
| G11 | Yazıcıda kağıt biter | Gönderen ekranda Türkçe hata; iş "hata" durumunda, yeniden denenebilir |
| G12 | Güvenlik duvarı: kapı internetten ve Tailscale adresinden denenir | Kapalı |

## 7. Bağımlılık ve arayüz notları (EB1–EB4)
Sıra: EB0 → (Ş2, KM1 genişler) → EB1 → EB2 ve EB3 → EB4 → EB5.

### 7.1 EB1: sunucu, bir mağazaya birden çok cihaz ve yazıcı kaydı
- `taa_cihazlar`: aynı `sube_id` için çok satır; yeni alanlar (öneri): `gorunen_ad`, `roller` (`yazici_sahibi`, `sira_tutan`, `telefon_sunucusu`), `son_yerel_adres` (bilgi amaçlı).
- Yeni tablolar (öneri): `sube_yazicilari` (`yazici_id`, `sube_id`, `tur` = `etiket|fis|a4`, `cihaz_id`, `baglanti`, `ad`, `model`, `durum`, `surum`), `numara_araliklari` (`sube_id`, `belge_turu`, `cihaz_id`, `bas`, `son`, `durum`), `numara_olaylari` (verildi, atlandı). Hepsi şubeli; RLS ve izolasyon matrisi satırı zorunlu (D22).
- Uçlar (öneri, adlar Ş3 ile kesinleşir):

| Uç | İzin | Not |
|---|---|---|
| `GET /v1/sube/esler` | şube cihazı | Sunucu imzalı eş listesi (§5.2); yalnız istek şubesi |
| `GET /v1/sube/yazicilar` | şube cihazı | Yazıcı kaydı; yalnız istek şubesi |
| `POST /v1/sube/yazici` | `sube_cihaz.yonet` (merkez) ya da kurulum yetkisi (TAA Kur, yönetici oturumu) | Yazıcı ekle, sahibini değiştir, pasifleştir; `surum` uymazsa 409 |
| `POST /v1/sube/rol` | `sube_cihaz.yonet` (merkez) | Sıra tutan, telefon sunucusu ataması; devir §3.5 |
| `POST /v1/numara/olay` | şube cihazı | Verilen blok, kullanılan yedek numara, atlama; `olay_id` tek kayıt |

- Yazıcı kaydı ve eş listesi şubelere havuz türü olarak da dağıtılabilir (ör. `sube.yazici@1`, `sube.cihaz@1`, `sube_kapsam = {şube}`, `sube.kase@1` gibi). Seçim EB1'de; eklenirse `docs/havuz-turleri.toml` satırı ve `havuz_denetle.py` yeşil olmalı.
- Çatı ekranı: "Bu mağazanın bilgisayarları ve yazıcıları": cihaz listesi, son görülme, roller, yazıcılar ve sahipleri, numara aralıkları, atlama olayları, bekleyen yazdırma işi sayısı (eşlerden bildirilen).
- Ş2 izolasyon düzeneği A şubesinde iki cihazla koşar (D21).

### 7.2 EB2: ortak yazdırma hizmeti (TerziTakip başlığı yazar)
- Ortak kitaplık, `program = "ortak"`. Yer önerisi: mevcut `taa-yazici` crate'i (bugün `yazici.denetle@1` sunuyor) ya da yeni ortak crate; yeni crate açılırsa `modul.toml` ve `sozlesme_denetle.py` yeşil olmalı. `yazdir.sun@1` paket genelinde tek sunucuda yer alır (denetim kuralı: yetenek benzersiz).
- Eş kapısı kitaplığı (kimlik, şifreli kanal, keşif, eş listesi denetimi) EB2 ve EB3'ün **ortak** parçasıdır; tek kapı, tek güvenlik duvarı kuralı. EB2 önce yazarsa EB3 aynısını kullanır; ikinci kapı açılmaz.
- Eş hizmeti bilgisayar açıkken programlar kapalı olsa da çalışmalıdır (yazıcı sahibi Tutaa kapalıyken de basar). Windows hizmeti mi, kullanıcı oturumunda arka plan süreci mi: AÇIK-E7.
- Bağlayanlar: TAA Etiket, Tutaa (Elektron; bugün doğrudan yerel yazıcıya basıyor, kod değişikliği ister), Terzi Takip, TAA Kasa. Elektron programları köprü üzerinden `yazdir.sun@1` ister.
- Güvenlik incelemesi Opus (ana plan §16.3).

### 7.3 EB3: eşler arası eşleme ve numara aralığı (TerziTakip, TT3 ve Ş4 ile)
- §3 ve §4 bu adımın bağlayıcı kurallarıdır. TT3 (çakışma ekranı) ve Ş4 (fiş uçları) ile aynı çakışma kaydını kullanır; ayrı bir çakışma biçimi açılmaz.
- Numara biçimi, yedek bölge sınırları ve güvenlik payı Ş4 ile kesinleşir (AÇIK-E5).
- `cihaz_sira` Ş0 §4.1'deki cihaz sıra numarasıyla aynı sayaçtır; ikinci sayaç açılmaz.

### 7.4 EB4: TAA Kur (KU6 ile)
- "Bu mağazaya ikinci bilgisayar ekle" akışı: cihaz anahtarı üret, şubeye kayıt başvurusu, Çatı onayı bekle, eş listesini al, şube imza anahtarı başvurusu.
- Yazıcı seçimi: "Bu bilgisayara bağlı yazıcılar hangileri?" (etiket, fiş, A4; her biri için yok/bu bilgisayar). Kayıt sunucuya gider (§7.1). Esenyurt bilgisi bilinmediği için kurulumda sorulur (AÇIK-E3).
- Güvenlik duvarı kuralı (§5.1 madde 7); ağ profili denetimi; eş çıkarılınca kuralın kaldırılması.
- Kurulum sonunda sınama: öteki eşe bağlan, deneme sayfası iste (`yazdir.sun@1` deneme işi), sonuç ekranda.
- Tek bilgisayarlı mağazada bu akış hiç görünmez.

## 8. Açık noktalar
- **AÇIK-E1:** "Mağaza" sözcüğü bu belgede ve ana plan §16'da şube (`subeler.kod`) anlamındadır; `docs/rehber-sozlesmesi.md` §2.2'de ise `tur = magaza` bir şubenin altındaki satış noktasıdır (ör. Esenyurt Outlet). Çelişki değil ama karışıklık riski var. Öneri: kodda alan adları `sube_*` kalır; ekranda "mağaza" yazılır.
- **AÇIK-E2:** Esenyurt'taki iki bilgisayar `istanbul-merkez` şubesine mi bağlanacak, yoksa biri Outlet bölümünde mi? Varsayım: ikisi de `istanbul-merkez`, önek `MRK`.
- **AÇIK-E3:** Esenyurt'ta hangi yazıcının (etiket, fiş, A4) hangi bilgisayara bağlı olduğu **bilinmiyor**; yazıcıların USB mi ağ yazıcısı mı (ör. `tcp://IP:9100` Epson) olduğu ve iki bilgisayarın aynı modemde olup olmadığı da bilinmiyor. EB4 kurulumda sorar; önceden bilinirse EB5 senaryoları buna göre daraltılır.
- **AÇIK-E4:** Ağ yazıcısı (bir bilgisayara değil ağa bağlı) iki bilgisayardan da doğrudan basılabilir. Bu sözleşme yine tek yazıcı sahibi seçer (tek kuyruk, çift basmama); sahibi kapalıyken ötekinin doğrudan basması kendiliğinden yapılmaz, yalnız Çatı'dan sahip değiştirilerek yapılır. Yusuf daha esnek yol isterse yeniden yazılır.
- **AÇIK-E5:** Numara biçimi ayrıntıları (ana sıra/yedek bölge sınırı, blok boyu 1.000, devir güvenlik payı) öneridir; mevcut Tutaa numaralandırmasıyla uyum Ş4'te denetlenmeli (Tutaa `API-SOZLESMESI.md` bu oturumda okunmadı, AÇIK-S6 ile aynı).
- **AÇIK-E6:** Esenyurt'ta iki ayrı kasa çekmecesi mi var? Varsa TAA Kasa gün sonu cihaz başına mı, mağaza başına mı tutulur (KS1'i etkiler; §15 kapsamı mağaza başına varsayar).
- **AÇIK-E7:** Eş hizmeti Windows hizmeti (oturum kapalıyken de çalışır, DPAPI kapsamı farklı) mi, kullanıcı oturumunda arka plan süreci mi? İkincisinde yazıcılı bilgisayar açık ama oturum kapalıysa yazdırma bekler; ekran mesajı "kapalı ya da oturum açık değil" olur.
- **AÇIK-E8:** Telefon yerel sunucusu çok bilgisayarlı mağazada hangi eşte çalışır, eş kapısıyla aynı kapıyı mı paylaşır? Ş0 §5 "merkeze yalnız şube PC'si yazar" ifadesi D8 ile genişler.
- **AÇIK-E9:** Şifreli eş bağlantısı için kitaplık (yeni dış bağımlılık) Yusuf onayı ister.
- **AÇIK-E10:** Kopuk mağazada eş listesi tazeliği sınırı (öneri 7 gün, Ş0 çevrimdışı belge süresiyle aynı) ve aşımda kısıtlanacak işler.
- **AÇIK-E11:** Ana plan §16.4, şube planı §2'deki "şubede tek PC olduğu için çakışma daralır" varsayımının çok bilgisayarlı mağazada geçmediğini söylüyor; şube planı bu oturumda okunmadı, düzeltme o planın sahibine bırakıldı.
- Kapanması önerilenler: **AÇIK-S1** (önekler kesin, ana plan §16.1) ve **AÇIK-S2** (Merkez Mağaza'da sunucudan ayrı 2 kasa bilgisayarı, ana plan §16.1). İşaretleme Ş0 belgelerinde D5, D2, D20 ile yapılır.
