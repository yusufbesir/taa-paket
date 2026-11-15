<!-- Sahip: Yusuf. yazan: [Claude#sozlesme-EB0 2026-11-15]. Durum: SÖZLEŞME EKİ (EB0). Fable denetimi 2026-11-15: GEÇTİ (koşullu); K1–K9 ve D1–D23 işlendi. Değişiklik: Yusuf'un onayıyla. -->
# Eşleme sözleşmesi: aynı mağazada birden çok bilgisayar (EB0)

Kaynak: `planlar/PLAN-TAA-PAKET-ANA-2026-11-15.md` §16 (Yusuf 2026-11-15 15:19 UTC), §15 (TAA Kasa), §14 (çekme modeli, "şube bilgisayarında açık kapı olmaz"), §15 sonu Yusuf kararı (iki hal saklanır, insan seçer). Fable denetimi `DENETIM-EB0-ESLEME-YAZDIR-2026-11-15` (koşullar K1–K8, kararlar E1–E11, Y1–Y3).
Bağlı belgeler: `docs/sube-sozlesmesi.md` (Ş0), `docs/sube-tehdit-modeli.md`, `docs/sube-izolasyon-testleri.md`, `docs/rehber-sozlesmesi.md`, `docs/modul-sozlesmesi.md`, `docs/ipc.md`, `docs/T7-KIMLIK.md`, `docs/S2-CIHAZ.md`. Yetenek: `docs/sozlesmeler/yazdir.sun.md`.
Bu belge Ş0'ın **ekidir**: Ş0'da yazan her kural geçerlidir; yalnız aşağıda adı geçen yerler genişler. Değişen Ş0 satırları §1.5'te listelidir ve Ş0 belgelerine **işlenmiştir** (D1–D23, Fable denetimiyle onaylı).

**Temel ilke:** tasarım Esenyurt'a özel değildir. Her mağazada 1 ya da daha çok bilgisayar olabilir. **Tek bilgisayarlı mağazada hiçbir şey değişmez:** eş yoktur, yerel ağ kapısı açılmaz, numara aralığı gerekmez, yazdırma doğrudan yereldedir.

## 1. Mağaza ve cihaz ayrımı

### 1.1 Terimler
| Terim | Anlam |
|---|---|
| Mağaza | Bu belgede **şube ile aynı şeydir**: 15 şubeden biri, kimliği `subeler.kod` (Ş0 §3). Esenyurt Outlet ayrı mağaza değildir, `istanbul-merkez` altındadır. E1 kararı: kodda alan adları `sube_*`, ekranda "mağaza" (rehberdeki `tur = magaza` ile karışmasın) |
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
Durum: D1–D23 Fable denetimiyle onaylandı (Yusuf 15:19 kararının işlenmesi; ayrıca Yusuf onayı gerekmez) ve Ş0 belgelerine (`sube-sozlesmesi.md`, `sube-tehdit-modeli.md`, `sube-izolasyon-testleri.md`) denetim koşullarıyla (K1–K9) aynı PR'da işlendi. "Şubede tek PC" varsayımı Ş0 belgelerinden kaldırıldı; "şube PC'si" artık "şubenin bilgisayarlarından herhangi biri" demektir. Öneri sütunu iz için korunur.

**`docs/sube-sozlesmesi.md`**
| # | Yer | Bugünkü varsayım | DEĞİŞİKLİK ÖNERİSİ | Durum |
|---|---|---|---|---|
| D1 | §1 "Şube PC'si" | "Şubedeki tek kayıtlı bilgisayar (15 şube × 1 PC)" | "Şube bilgisayarı: şubeye bağlı onaylı bilgisayarlardan biri (1 ya da daha çok). Aynı şubenin bilgisayarları eştir (`docs/esleme-sozlesmesi.md`)." | UYGULANDI (commit 744e2aa) |
| D2 | §1 "Merkez Mağaza" | Sunucu PC'siyle ilişkisi belirsiz | "… Esenyurt'ta sunucudan ayrı 2 kasa bilgisayarı vardır (ana plan §16.1)." AÇIK-S2 kapanır | UYGULANDI (commit 744e2aa) |
| D3 | §1 "Şube imza anahtarı" | Tek anahtar | "… her eş bilgisayarın kendi şube imza anahtarı vardır; hepsi şube adına geçerlidir." | UYGULANDI (commit 744e2aa) |
| D4 | §2 kural 3 | "şube PC'si sunucu olmadan çalışır" | "her şube bilgisayarı sunucu ve eşi olmadan çalışır" | UYGULANDI (commit 744e2aa) |
| D5 | §3 önek tablosu | "Aşağıdaki önekler öneridir (AÇIK-S1)", sütun "(öneri)" | "Önekler kesindir (Yusuf 2026-11-15, ana plan §16.1); pilot başladıktan sonra değişmez." Sütun başlığı `onek`. AÇIK-S1 kapanır | UYGULANDI (commit 744e2aa) |
| D6 | §4 madde 1 | Tek cihaz varsayılır | Ek cümle: "Bir şubeye birden çok cihaz bağlanabilir; Çatı şube başına cihaz listesini gösterir (EB1)." | UYGULANDI (commit 744e2aa) |
| D7 | §4 madde 7 (PC devri) | Tek PC | Ek: "Devredilen PC eski şubenin eş listesinden düşer; elindeki yedek numara aralığının kullanılmamış kısmı atlandı notuyla kapanır (`esleme-sozlesmesi` §3)." | UYGULANDI (commit 744e2aa) |
| D8 | §5 tablo "Fiş, fiş olayı" | "Sahibi şube PC'si … merkeze yalnız şube PC'si yazar" | "Sahibi şube; şubenin onaylı bilgisayarlarından herhangi biri yazar, kayıtta `kaynak_cihaz` durur. Telefonlar yalnız kayıtlı telefon sunucusu eşe bağlanır (AÇIK-E8)." | UYGULANDI (commit 744e2aa) |
| D9 | §5 tablo "Şube imza anahtarı kaydı" | "Şube PC'si başvurur" | "Her şube bilgisayarı kendi anahtarıyla başvurur" | UYGULANDI (commit 744e2aa) |
| D10 | §6 | İki kanal | Ek satır: "Eş kanalı (aynı şubenin bilgisayarları arası, yerel ağ) yalnız o şubenin işlem kayıtlarını, yazdırma işlerini ve numara isteklerini taşır; havuz içeriği taşımaz." | UYGULANDI (commit 744e2aa) |
| D11 | §7.1 "Saklama" | "Şube PC'sinde üretilir" | "Her şube bilgisayarında ayrı üretilir; …" (geri kalanı aynen) | UYGULANDI (commit 744e2aa) |
| D12 | §7.3 madde 2 | "o şubenin geçerli şube imza anahtarıyla" | "o şubenin geçerli şube imza anahtarlarından biriyle" | UYGULANDI (commit 744e2aa) |
| D13 | §7.5 iptal | Tek anahtar | Ek: "Bir bilgisayarın anahtarının iptali şubenin öteki anahtarlarını etkilemez." | UYGULANDI (commit 744e2aa) |
| D14 | §8 son madde | "o cihaza aittir" | Ek: "Aynı şubenin işlem kayıtları eşlemeyle her eşte de bulunur; her kopya DPAPI ile sarılıdır." | UYGULANDI (commit 744e2aa) |
| D15 | §9.4 | Eş ve yazıcı uçları yok | EB1 uçları (öneri) eklenir: §7.1 tablosu | UYGULANDI (commit 744e2aa) |
| D16 | §10 | "Sıra: … cihaz sıra numarası" | Ek: "Cihaz sıra numarası her cihaz için ayrıdır; eşten gelen çakışma da aynı kuralla iki hal olarak saklanır (`esleme-sozlesmesi` §4.3)." | UYGULANDI (commit 744e2aa) |

**`docs/sube-tehdit-modeli.md`**
| # | Yer | DEĞİŞİKLİK ÖNERİSİ | Durum |
|---|---|---|---|
| D17 | §1 Varlıklar | V12 eş listesi (sunucu imzalı), V13 yazdırma kuyruğu, V14 numara sırası ve yedek aralıklar eklenir | UYGULANDI (commit 744e2aa) |
| D18 | §3 Saldırı yüzeyi | "Şube PC'si" maddesine "eş kapısı (yalnız çok bilgisayarlı mağazada, yalnız yerel ağ)" eklenir | UYGULANDI (commit 744e2aa) |
| D19 | §4 | T23–T30 eklenir (bu belge §5.4); denetim koşullarıyla T31–T34 de eklendi (K2, K3, K8, K6) | UYGULANDI (commit 744e2aa) |
| D20 | §8 | AÇIK-S1 ve AÇIK-S2 "kapandı (ana plan §16.1)" diye işaretlenir | UYGULANDI (commit 744e2aa) |

**`docs/sube-izolasyon-testleri.md`**
| # | Yer | DEĞİŞİKLİK ÖNERİSİ | Durum |
|---|---|---|---|
| D21 | §1 Düzenek | A şubesinde **iki** onaylı cihaz (A1, A2) olur; B'de bir cihaz kalır (tek bilgisayarlı durum da sınanır) | UYGULANDI (commit 744e2aa) |
| D22 | §4 Matris | Yeni satırlar: `sube_yazicilari`, `numara_araliklari`, eş listesi ucu (okuma, yazma, liste, sayım, günlük, hata) | UYGULANDI (commit 744e2aa) |
| D23 | §5 | Ek senaryolar 13–16: `izolasyon_es_baska_magaza`, `izolasyon_es_listesi_capraz`, `izolasyon_yazici_capraz`, `izolasyon_numara_capraz` (bu belge §6.1) | UYGULANDI (commit 744e2aa) |

## 2. Ortak yazıcı ve `yazdir.sun@1`
Ayrıntılı sözleşme: `docs/sozlesmeler/yazdir.sun.md`. Özet:
- Programlar (TAA Etiket, Tutaa, Terzi Takip, TAA Kasa) yazdırmayı **kendi bilgisayarlarındaki yazdırma hizmetine** `taa-ipc` ile ister. Hizmet, yazıcı kaydına bakar: yazıcı bu bilgisayardaysa basar, eşteyse işi eş kapısından yazıcı sahibine iletir. Kullanıcı fark görmez.
- **Hizmet yanıt vermezse (K1):** yazıcı bu bilgisayardaysa program bugünkü doğrudan yazdırmaya döner ve bunu deftere yazar (`yazdir.sun.md` Kural 11). Tek bilgisayarlı mağazada yazdırma yeni bir arıza noktası kazanmaz. Yazıcı eşteyse iş yerel kuyrukta bekler.
- **Tek yazdırma yolu (K7):** içerik üreten yetenekler aynen kalır ve çıktıyı `yazdir.sun@1`'e verir: `fis.yazdir@1` fiş içeriği, `etiket.yazdir@1` etiket içeriği üretir; `kasa.say@1` ve `kasa.kapat@1` yazdırmaz, gün sonu çıktısı `fis.yazdir@1` → `yazdir.sun@1` yolundan basılır. Hiçbiri ötekinin yerini almaz; yazıcıya giden tek yol `yazdir.sun@1`'dir (K1 dönüşü de aynı kitaplığın yerel basma kodudur). Ayrıntı `yazdir.sun.md` Kural 16.
- **Para çekmecesi uzaktan açılmaz (K2):** eşten gelen `escpos` içeriğinde çekmece açma komutu reddedilir; çekmece yalnız bağlı olduğu bilgisayardan, ayrı ve deftere yazılan `cekmece_ac` işlemiyle açılır (`yazdir.sun.md` Kural 12).
- **İçerik dosyası (K3):** hizmet yalnız kendi `yazdir\is\` klasöründen, adı `is_id` olan dosyayı okur; içerikler DPAPI ile sarılı, arşiv süresi yazılıdır (`yazdir.sun.md` Kural 13).
- **A4 PDF (K8):** yazıcı sahibi PDF'i ayrı süreçte, boyut (10 MiB) ve sayfa (50) sınırıyla basar; araç EB2'de adıyla yazılır (`yazdir.sun.md` Kural 14).
- Yazıcı türleri: `etiket`, `fis`, `a4`; aynı türden birden çok yazıcı olabilir. İstek isteğe bağlı `yazici_id` taşır; verilmezse o bilgisayarın o tür için varsayılan yazıcısı kullanılır; çıktı ekranında seçim yalnız birden çok yazıcı varsa görünür (K9, `yazdir.sun.md` Kural 1). Hangi yazıcının hangi eşe bağlı olduğu kurulumda kaydedilir, Çatı'dan değiştirilir (EB1, EB4). Ağ yazıcısında da tek sahip vardır (E4).
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
- Yedek numara `MRK-800123` gibi görünür; personel şaşırmasın diye fişte ve ekranda küçük "yedek numara" işareti kalır (E5).
- Hiç bağlantı yokken yedek aralık biterse eş yeni numara **üretmez**; ekranda "Numara aralığı bitti. Öteki bilgisayarı açın ya da yöneticiyi arayın." yazar. Fiş taslak olarak saklanır, numara bağlantı dönünce verilir.

### 3.4 Tekillik ve atlama
- Numara her durumda tekildir: ana sıra yalnız sıra tutandan, yedek numara yalnız o eşe verilmiş bloktan çıkar. Sunucu aynı `no` için farklı içerik görürse Ş0 §10 gereği 409 `cakisma` ile iki hali saklar.
- **Sıra yalnız kesintide atlayabilir:** sıra tutan numarayı verdi ama yanıt eşe ulaşmadıysa ya da yedek blokta kullanılmamış numara kalıp blok kapandıysa. Her atlama deftere `numara.atlandi` olayı olarak yazılır (tür, aralık, neden, cihaz, zaman) ve Çatı'da görünür. Sessiz atlama yoktur.
- Ana sıra ile yedek numaralar zaman sırasında karışık görünebilir (yedek numara daha sonra yazılmış olabilir); sıralama belge tarihine göre yapılır, numaraya göre değil. Bilgisayar saatleri farklı olabileceği için merkezin kabul zamanı varsa o esas alınır (denetim önerisi).

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
- **TAA Kasa (E6 kararı, Yusuf 15:34 ile netleşti):** **nakit sayımı çekmece başına**, **Z raporu ve POS gün sonu kasa noktası başına** tutulur; bir çekmeceye birden çok kasa noktası bağlanabilir (ayrıntı `planlar/PLAN-MAGAZA-ISLEYISI-2026-11-15.md` §4.1). Mağaza toplamı bu kayıtlardan kendiliğinden toplanır (ayrıca girilmez). Her mağazanın en az bir çekmecesi vardır; tek çekmeceli mağazada fark görünmez. **Esenyurt'ta tek nakit çekmece vardır, iki kasa bilgisayarı onu paylaşır.** Aynı çekmecenin sayımı iki bilgisayardan aynı anda girilirse §4.3 çakışma kuralı işler (iki hal saklanır, insan seçer) ve ekranda "Bu çekmecenin sayımı şu an <Kasa-1> bilgisayarında açık." uyarısı gösterilir (§4.4). Çekmece elle açılan sıradan çekmece olabilir; yazıcıya bağlı çekmece ise uzaktan açılmaz (K2). KS1 buna göre yazılır.
- **Ciro:** mağaza ve gün başına bir kayıt (Ş0 §5); aynı kural.
- **Telefonlar (E8 kararı):** telefon yerel sunucusu (taa-fis-sunucu) mağazada tek eşte çalışır: **sıra tutan eşte** (`telefon_sunucusu` rolü varsayılan olarak sıra tutanla aynı eşe verilir; değişiklik yalnız Çatı'dan). Telefon fişleri o eşin defterine yazılır ve eşlenir. Telefon sunucusunun kapısı **eş kapısından ayrıdır**: telefon kayıtlı eş cihaz değildir, eş kapısına bağlanamaz; "yalnız kayıtlı eş" kuralı sulanmaz.

### 4.6 İlk eşleme (K5)
Geçmişi olan bir mağazaya ikinci (ya da sonraki) bilgisayar eklendiğinde yeni eşin eski kayıtları alması:
1. **Kaynak:** önce eşten, yerel ağda (§4.2 yol 1); eş kapısı açılamıyorsa (eş kapalı, ağ yalıtımı, sürüm uyuşmazlığı, Genel profil) sunucudan, şubenin liste uçlarıyla (§4.2 yol 3). Kaynak kesintide değişebilir; aynı kayıt iki yoldan gelirse `kimlik + surum + ozet` ile tekilleşir. Eşten gelen her kayıt §4.2 son maddedeki denetimden geçer.
2. **Sıra:** (a) eş listesi, yazıcı kaydı, çekmece kaydı ve numara durumu (sıra tutan kim, bu eşin yedek bloğu); (b) açık işler: açık fişler, açık çakışmalar, bekleyen yazdırma işleri, son 30 günün kayıtları; (c) geri kalan geçmiş, yeniden eskiye; (d) büyük ekler (tutanak fotoğrafı) en son; kullanıcı bir kaydı açınca o kaydın eki öne alınır.
3. **Süre:** ölçülmedi [ÖLÇ]. Hedef (öneri): (a) ve (b) aynı yerel ağda 5 dk içinde; tüm geçmişin süresi EB5'te Esenyurt verisiyle ölçülüp `docs/olcum/EB5-<tarih>.md`'ye yazılır.
4. **Bu sürerken bilgisayar çalışır.** Ekranda "Eski kayıtlar geliyor (%N). Yeni işlem yapabilirsiniz." yazar. Henüz gelmemiş eski bir kayıt aranırsa "Bu kayıt henüz bu bilgisayara gelmedi." denir; kayıt yokmuş gibi davranılmaz. Yeni fiş açılabilir; numara §3 kurallarıyla alınır (sıra tutana ulaşılamıyor ve yedek blok henüz verilmemişse fiş taslak kalır, §3.3).
5. **Kaldığı yerden sürer:** kayıtlar `cihaz_sira` sırasıyla, büyük ekler parça parça ve her parça özetle doğrulanarak gelir; bağlantı koparsa ya da bilgisayar kapanırsa son doğrulanan parçadan devam eder, baştan başlamaz.
6. İlk eşleme bitince deftere `es.ilk_esleme_bitti` (kaynak, kayıt sayısı, süre) yazılır ve Çatı'da görünür. İki bilgisayar böylece birbirinin yerel yedeği olur; biri bozulursa öteki mağaza verisini taşır (kurulum ekranında ve Çatı'da görünür; denetim önerisi).

## 5. Yerel ağ güvenliği
### 5.1 Eş kapısı kuralları
1. **Yalnız çok bilgisayarlı mağazada açılır.** Eş listesinde başka bilgisayar yoksa kapı dinlemez.
2. **Yalnız mağaza ağı:** kapı yalnız yerel ağ arayüzünde dinler. TAA VPN / Tailscale arayüzünde (100.64.0.0/10) ve genel ağ profilinde dinlemez. Bağlanan adres yerel alt ağda değilse bağlantı kimlik denetimine geçmeden kapanır.
3. **Yalnız aynı mağazanın kayıtlı eşi:** iki uç da birbirinin açık cihaz anahtarını kendi eş listesinde bulmalıdır (karşılıklı kimlik doğrulama). Bağlantı el sıkışması cihaz anahtarıyla, alan ayrımı etiketi `TAA-ES-v1` ile imzalanır; böylece eş imzası sunucu istek imzası ya da belge imzası yerine kullanılamaz (Ş0 §7.2 ilkesi).
4. **Şifreli bağlantı:** tüm trafik şifreli; oturum anahtarı her bağlantıda yeniden üretilir (ileri gizlilik); karşı tarafın anahtarı eş listesindeki anahtara sabitlenir. Düz metin kip yoktur. Kitaplık (E9): öneri `rustls` ile TLS 1.3, karşılıklı kimlik, anahtar sabitleme (yaygın, bağımsız denetimden geçmiş). Depoda bugün şifreli bağlantı kitaplığı yoktur (yalnız `ed25519-dalek`); bu gerçekten yeni dış bağımlılıktır. **Kapandı: Yusuf `rustls` eklenmesini onayladı (2026-11-15 15:34).** EB2'de `Cargo.toml`'a sürümü sabitlenerek eklenir; `cargo audit` ile izlenir.
5. **Mesaj tekrarı reddedilir:** her mesajda oturum içi artan sayaç.
6. **İnternete ve başka mağazaya kapalı:** başka mağazanın onaylı cihazı da (eş listesinde olmadığı için) reddedilir. Modemde kapı yönlendirmesi yapılmaz, TAA Kur yapmaz.
7. **Güvenlik duvarı kuralını TAA Kur kurar (EB4):** gelen yön, yalnız eş hizmetinin exe'si, yalnız tek sabit TCP kapısı ve keşif duyurusu için tek sabit UDP kapısı (§5.5; numaralar EB2'de seçilir), yalnız Özel ağ profili, uzak adres yalnız yerel alt ağ. Ağ profili "Genel" ise TAA Kur kuralı kurmaz ve "Bu ağ Genel olarak işaretli; eş bilgisayar bağlantısı kapalı kalacak" der; yönetici onayıyla ağı "Özel"e çevirmeyi önerir (denetim önerisi; çevirme yalnız onayla). Eş çıkarılınca kural kaldırılır. Telefon sunucusunun kapısı bu kuraldan ayrıdır (E8).
8. **Hız ve boyut sınırları:** eş başına iş/istek sınırları `yazdir.sun.md` ve §3'te; aşımda istek reddedilir, deftere yazılır.

### 5.2 Eş listesi
- Kaynak: sunucu. Liste mağaza kodu, cihaz kimlikleri, açık anahtarlar, cihaz adları, rol işaretleri (yazıcı sahibi, sıra tutan, telefon sunucusu), liste sürümü ve zamanından oluşur; sunucu anahtarıyla imzalıdır.
- Her eş listeyi önbellekte tutar; imzası tutmayan liste kullanılmaz (önceki geçerli liste sürer).
- Çatı'dan cihaz iptal edilince yeni liste yayımlanır; eşler bir sonraki sunucu bağlantısında alır ve iptal edilen cihazla bağlantıyı keser.
- **Dürüst sınır:** mağaza internetten tamamen kopuksa iptal edilen bir cihaz, eşlerin listesi yenilenene kadar yerel ağda eş sayılmaya devam eder (Ş0'daki 7 günlük çevrimdışı gecikme ile aynı türden). **Karar (E10):** 7 günden eski liste ile eş kapısı yalnız okuma ve yazdırmaya izin verir; numara bloğu vermez. 5. günden itibaren ekranda uyarı: "Bilgisayar listesi <N> gündür merkezden yenilenmedi. İnternete bağlanın; 7. günden sonra yeni numara bloğu alınamaz."

### 5.3 "Şube bilgisayarında açık kapı olmaz" kuralına dar istisna
Ana plan §14 kuralı ("Şube bilgisayarında dışarıya açık kapı olmaz") geçerli kalır. Eş kapısı bilinçli ve dar bir istisnadır (ana plan §16.2 madde 6), şu sınırların **hepsi** birlikte sağlanırsa:
- yalnız çok bilgisayarlı mağazada, yalnız yerel ağ arayüzünde, yalnız Özel profilde;
- yalnız aynı mağazanın eş listesindeki cihaza, karşılıklı kimlik ve şifreli bağlantıyla;
- yalnız bu belgede sayılan işler (yazdırma, eşleme, numara, açık bildirimi); başka uç yoktur;
- internet, VPN ve başka mağaza yönünden kapalıdır.
Bunlardan biri sağlanamıyorsa kapı açılmaz; mağaza tek bilgisayarlı gibi çalışır (yazdırma yalnız yerel, eşleme yalnız merkez yoluyla).
Not: Ş0 tehdit modeli §3'teki telefon eşleştirme yerel sunucusu zaten yerel ağda dinleyen bir kapıdır. **Karar (E8):** telefon sunucusu eş kapısından **ayrı** kapıda kalır; telefon kayıtlı eş değildir ve eş kapısına bağlanamaz (`telefon_es_kapisina_baglanamaz`).

### 5.4 Tehdit modeli eki
| # | Tehdit | Saldırgan | Önlem | Test adı | Kalan risk |
|---|---|---|---|---|---|
| T23 | **Sahte eş cihaz**: yerel ağda kendini eş gibi tanıtan bilgisayar | S1, S4 | Karşılıklı kimlik; eş listesi sunucu imzalı; `TAA-ES-v1` etiketli el sıkışma; iptal edilen cihaz listeden düşer | `es_kayitsiz_cihaz_red`, `es_baska_magaza_cihazi_red`, `es_iptal_cihaz_red`, `es_liste_imza_bozuk_red`, `es_imza_etiket_karisma_red`, `es_kesif_sahte_duyuru_red` | Kopuk mağazada iptal gecikmesi (§5.2) |
| T24 | **Yerel ağda dinleme / araya girme** | S4 | Şifreli bağlantı, ileri gizlilik, anahtar sabitleme; düz metin kip yok | `es_baglanti_sifresiz_red`, `es_araya_girme_red`, `es_ag_dokumu_duz_metin_yok`, `es_mesaj_tekrar_red` | Trafik miktarı ve zamanı görünür (içerik görünmez) |
| T25 | **Yazdırma kuyruğuna sahte iş** | S1, S4, S7 | Eşten yalnız kimliği doğrulanmış eş; her iş kim, hangi cihaz, hangi belge ile deftere; eş başına hız ve kuyruk sınırı; içerik türü ve boyut sınırı | `yazdir_kayitsiz_es_red`, `yazdir_hiz_siniri`, `yazdir_kuyruk_dolu`, `yazdir_bicim_red`, `yazdir_is_deftere_yazilir` | Aynı bilgisayarda aynı kullanıcıyla çalışan kötü niyetli süreç yerel IPC'den iş gönderebilir (`docs/ipc.md` güvenlik sınırı) |
| T26 | **Numara aralığı tüketme**: bozuk ya da kötü niyetli eşin sürekli numara/blok istemesi | S2, S7 | Blok yalnız önceki blok %80 dolunca; eş başına numara isteği hız sınırı; her verilen numara ve blok defterde; atlama eşiği aşılınca Çatı uyarısı | `numara_blok_tukenme_siniri`, `numara_istek_hiz_siniri`, `numara_atlama_deftere_yazilir`, `numara_ayni_istek_ayni_no` | Sınır içinde kalan yavaş tüketme; Çatı uyarısıyla görülür |
| T27 | Eş kapısına internetten, VPN'den ya da başka alt ağdan erişim | S6, S1 | Yalnız yerel arayüz; güvenlik duvarı Özel profil + yerel alt ağ; uygulama içi adres denetimi | `es_kapi_tailscale_kapali`, `es_kapi_genel_profil_kapali`, `es_kapi_alt_ag_disi_red`, `es_tek_pc_kapi_kapali` | Mağaza modemine izinsiz bağlanan cihaz kimlik denetimine kadar ulaşır (T23 ile durur) |
| T28 | Eşleme ile başka mağaza verisinin gelmesi ya da gitmesi | S2 | Kayıt şube damgası denetlenir; başka şube damgalı kayıt uygulanmaz; PC devrinde eş listesinden düşme | `es_baska_sube_damgali_kayit_red`, `es_pc_devri_baglanti_kesilir` | — |
| T29 | Çift basma (bütünlük) | — | Tek iş kimliği; yazıcı sahibinde basılan işler defteri; "basılıyor" iken çöküşte otomatik yeniden basma yok | `yazdir_cift_basma_yok`, `yazdir_yeniden_gonderim_tek_baski`, `yazdir_cokus_belirsiz_durum` | Çöküşte "basıldı mı" belirsiz kalabilir; kullanıcıya sorulur |
| T30 | Eşten gelen kaydın sahte sürümle ezilmesi | S2 | Eklemeli kayıt; aynı öncülden iki sürüm = çakışma, iki hal saklanır; otomatik seçim yok | `es_cakisma_iki_hal_saklanir`, `es_son_yazan_kazanmaz` | Şube müdürü yanlış hali seçebilir (insan kararı, denetim izinde) |
| T31 | **Para çekmecesini uzaktan açma**: eşten gelen fiş çıktısına çekmece açma komutu gömmek | S1, S2, S4 | Eşten gelen `escpos` içeriğinde çekmece açma komutu (ESC p, DLE DC4) reddedilir; çekmece yalnız bağlı olduğu bilgisayardan, ayrı `cekmece_ac` işlemiyle açılır ve deftere yazılır; eş kapısında `cekmece_ac` yoktur | `yazdir_esten_cekmece_komutu_red`, `yazdir_cekmece_esten_ac_red`, `yazdir_cekmece_acma_deftere_yazilir` | Yazıcılı bilgisayarda oturum açmış kişi çekmeceyi yerelden açabilir (yetki ve defter ile görünür) |
| T32 | **İçerik yoluyla dosya okuma**: `icerik_yolu` ile hizmete başka dosya okutmak | S1, S7 | Hizmet yalnız kendi `yazdir\is\` klasöründen, adı `is_id` ile aynı dosyayı okur; üst klasör, mutlak başka yol, kısayol ve bağlantı (symlink, junction) reddedilir; içerikler DPAPI ile sarılı, arşiv süresi yazılı | `yazdir_yol_disari_red`, `yazdir_yol_kisayol_red`, `yazdir_icerik_sarili` | Aynı Windows kullanıcısıyla çalışan kötü niyetli süreç klasörü doğrudan okuyabilir (`docs/ipc.md` sınırı) |
| T33 | **Bozuk ya da dev PDF ile yazıcılı bilgisayarı aksatma** | S1, S7 | Boyut ve sayfa sınırı; PDF ayrı süreçte açılır, zaman aşımı; açılamayan dosya reddedilir, hizmet çalışmaya devam eder; basım aracı EB2'de adıyla yazılır | `yazdir_pdf_boyut_siniri`, `yazdir_pdf_sayfa_siniri`, `yazdir_pdf_bozuk_red` | Basım aracının kendi açığı (araç güncel tutulur) |
| T34 | **Sürüm düşürme**: eşin eski protokol sürümü bildirerek zayıf kipe zorlaması | S4, S2 | El sıkışmada protokol sürümü bildirilir; eski protokole düşülmez; uyuşmazlıkta eş kapısından iş alınmaz, mağaza "yalnız merkez yoluyla eşleme, yalnız yerel yazdırma" kipine geçer ve ekranda söyler | `es_surum_dusurme_yok`, `es_surum_uyusmaz_merkez_kipi` | Güncelleme dalgasında eşleme yalnız merkez yoluyla yürür (gecikme) |

Saldırgan kodları (S1–S7) Ş0 tehdit modeli §2'dendir. T23–T34 `docs/sube-tehdit-modeli.md` §4'e de işlendi (D19).

### 5.5 Eş keşfi (K4)
Eşin yerel adresi değişebilir (modem adres dağıtır). Eş şu sırayla bulunur:
1. **Sunucudan gelen son yerel adres:** her eş sunucuya her bağlantıda kendi yerel adresini ve kapısını bildirir (`taa_cihazlar.son_yerel_adres`, §7.1); adres eş listesiyle birlikte gelir. Bilgi amaçlıdır, imza yerine geçmez.
2. **Son başarılı bağlantı adresi** (yerel önbellek).
3. **Yerel ağda duyuru:** eş kapısı açık olan her eş, yerel alt ağa kısa aralıklarla duyuru yayar (cihaz kimliğinin özeti + kapı numarası; şube adı ve kişi verisi yok). Duyuru yalnız eş kapısının dinlediği arayüzde ve yalnız Özel profilde yapılır; Tailscale arayüzünde yapılmaz. UDP kapısı EB2'de seçilir, güvenlik duvarı kuralı §5.1 madde 7.
- **Duyuruya güvenilmez:** adres yalnız ipucudur. Kimlik her bağlantıda el sıkışmada eş listesindeki açık anahtarla doğrulanır (§5.1 madde 3). Sahte duyuru en çok başarısız bir bağlantı denemesine yol açar; denemeler hız sınırlıdır ve deftere yazılır (`es_kesif_sahte_duyuru_red`).
- Adres değişince eş duyurudan ya da sunucudan yeni adresi öğrenir; elle ayar gerekmez.
- **Dürüst sınır:** kablosuz ağda cihazların birbirini görmesi kapalıysa (istemci yalıtımı) ya da iki bilgisayar farklı alt ağdaysa kapı çalışmaz. EB4 kurulum sınaması bunu sınar ve söyler: "Bu iki bilgisayar ağda birbirini göremiyor (kablosuz ağda cihaz yalıtımı açık olabilir). Eş bağlantısı çalışmayacak; modem ayarı ya da kablo gerekir." O durumda mağaza tek bilgisayarlı gibi çalışır (§5.3 son paragraf).

### 5.6 Sürüm uyuşmazlığı (K6)
- Güncelleme dalgasında iki eş farklı program sürümünde kalabilir. El sıkışmada iki taraf **eş protokolü sürümünü** (tam sayı) ve program sürümünü (bilgi) bildirir.
- Protokol sürümü aynıysa eş kapısı normal çalışır. Uyuşmazsa **kapı kapanmaz** (dinler, kimlik doğrular, sürümü yeniden sorar), ama kapıdan eşleme, yazdırma ve numara işi yapılmaz: mağaza **"yalnız merkez yoluyla eşleme, yalnız yerel yazdırma"** kipine düşer. Numara §3.3 yedek aralıktan sürer; eşteki yazıcıya giden işler `bekliyor_surum_uyusmaz` durumunda bekler (`yazdir.sun.md` Kural 15).
- Ekranda: "Öteki bilgisayarın (<ad>) program sürümü farklı. Güncelleme bitene kadar kayıtlar merkez üzerinden eşlenir, yazdırma yalnız bu bilgisayarın yazıcılarına yapılır." Çatı'da da görünür.
- Eski protokol sürümüne düşülmez (sürüm düşürme saldırısı, T34). Sürümler eşitlenince kip kendiliğinden normale döner, bekleyen işler bir kez basılır.

## 6. Kabul testleri
### 6.1 Otomatik (EB1–EB4 bunları geçirmeden birleşmez)
1. `tek_pc_degismez`: tek bilgisayarlı mağazada kapı açılmaz, yazdırma doğrudan yerel, numara bugünkü gibi; **yazdırma hizmeti kapalıyken de basar** ve doğrudan yazdırmayı deftere yazar (K1).
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
13. Denetim koşulları: `yazdir_esten_cekmece_komutu_red` (K2), `yazdir_yol_disari_red` (K3), `es_kesif_sunucu_adresi`, `es_kesif_duyuru_adres_degisimi`, `es_kesif_sahte_duyuru_red` (K4), `ilk_esleme_calisirken_is_surer`, `ilk_esleme_kaldigi_yerden`, `ilk_esleme_ek_parcali_surer`, `ilk_esleme_sunucudan` (K5), `es_surum_uyusmaz_merkez_kipi`, `es_surum_dusurme_yok`, `es_surum_esitlenince_normal` (K6), `yazdir_pdf_boyut_siniri`, `yazdir_pdf_sayfa_siniri`, `yazdir_pdf_bozuk_red` (K8), `yazdir_varsayilan_yazici`, `yazdir_yazici_secimi` (K9), `kasa_sayim_cekmece_basina`, `kasa_z_raporu_kasa_noktasi_basina`, `kasa_sayim_ayni_anda_uyari` (E6), `telefon_es_kapisina_baglanamaz` (E8), `es_liste_5_gun_uyari`, `es_liste_7_gun_numara_bloku_yok` (E10).
14. İzolasyon ekleri: `izolasyon_es_baska_magaza` (B cihazı A'nın eş kapısına bağlanamaz), `izolasyon_es_listesi_capraz` (A, B'nin eş listesini alamaz: 404), `izolasyon_yazici_capraz` (A, B'nin yazıcı kaydını göremez, değiştiremez), `izolasyon_numara_capraz` (A, B'nin numara defterini göremez).

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
| G13 | Yazdırma hizmeti durdurulur, yazıcılı bilgisayardan fiş basılır (K1) | Doğrudan basar; deftere `yazdir.dogrudan`; hizmet dönünce çift basma yok |
| G14 | İkinci bilgisayar geçmişi olan mağazaya ilk kez eklenir (K5) | Bilgisayar hemen çalışır, "Eski kayıtlar geliyor" yazar; kablo çekilip takılınca kaldığı yerden sürer; süre ölçülür |
| G15 | Modem eşin adresini değiştirir (K4) | Eş yeniden bulunur, elle ayar gerekmez |
| G16 | Bir bilgisayar eski sürümde bırakılır (K6) | "Program sürümü farklı" mesajı; eşleme merkez yoluyla, yazdırma yalnız yerel; güncellemeden sonra normale döner |
| G17 | İki kasa bilgisayarından aynı çekmecenin sayımı aynı anda açılır (E6) | "Sayım öteki bilgisayarda açık" uyarısı; ısrar edilirse iki hal saklanır |
| G18 | Aynı türden iki yazıcı varsa çıktı ekranı (K9) | Seçim görünür; tek yazıcıda görünmez; varsayılan bilgisayar başına çalışır |

## 7. Bağımlılık ve arayüz notları (EB1–EB4)
Sıra: EB0 → (Ş2, KM1 genişler) → EB1 → EB2 ve EB3 → EB4 → EB5.

### 7.1 EB1: sunucu, bir mağazaya birden çok cihaz ve yazıcı kaydı
- `taa_cihazlar`: aynı `sube_id` için çok satır; yeni alanlar (öneri): `gorunen_ad`, `roller` (`yazici_sahibi`, `sira_tutan`, `telefon_sunucusu`; telefon sunucusu varsayılan olarak sıra tutan eştir, E8), `son_yerel_adres` (bilgi amaçlı, keşfin ilk ipucu, §5.5), `protokol_surumu` (bilgi, §5.6).
- Yeni tablolar (öneri): `sube_yazicilari` (`yazici_id`, `sube_id`, `tur` = `etiket|fis|a4`, `cihaz_id`, `baglanti`, `ad`, `model`, `durum`, `surum`), `numara_araliklari` (`sube_id`, `belge_turu`, `cihaz_id`, `bas`, `son`, `durum`), `numara_olaylari` (verildi, atlandı), `varsayilan_yazicilar` (`cihaz_id`, `tur`, `yazici_id`, `surum`; K9; aynı türden birden çok yazıcı desteklenir), `sube_cekmeceleri` (`cekmece_id`, `sube_id`, `ad`, `yazici_id` boş olabilir) ve `kasa_noktalari` (`kasa_noktasi_id`, `sube_id`, `cekmece_id`, `cihaz_id`; E6). Hepsi şubeli; RLS ve izolasyon matrisi satırı zorunlu (D22).
- Uçlar (öneri, adlar Ş3 ile kesinleşir):

| Uç | İzin | Not |
|---|---|---|
| `GET /v1/sube/esler` | şube cihazı | Sunucu imzalı eş listesi (§5.2); yalnız istek şubesi |
| `GET /v1/sube/yazicilar` | şube cihazı | Yazıcı kaydı; yalnız istek şubesi |
| `POST /v1/sube/yazici` | `sube_cihaz.yonet` (merkez) ya da kurulum yetkisi (TAA Kur, yönetici oturumu) | Yazıcı ekle, sahibini değiştir, pasifleştir; `surum` uymazsa 409 |
| `POST /v1/sube/varsayilan-yazici` | şube cihazı (yalnız kendi `cihaz_id`'si için) ya da `sube_cihaz.yonet` | Bilgisayar ve tür için varsayılan yazıcı (K9); yazıcı aynı şubenin etkin yazıcısı olmalı; `surum` uymazsa 409 |
| `POST /v1/sube/rol` | `sube_cihaz.yonet` (merkez) | Sıra tutan, telefon sunucusu ataması; devir §3.5 |
| `POST /v1/numara/olay` | şube cihazı | Verilen blok, kullanılan yedek numara, atlama; `olay_id` tek kayıt |

- Yazıcı kaydı ve eş listesi şubelere havuz türü olarak da dağıtılabilir (ör. `sube.yazici@1`, `sube.cihaz@1`, `sube_kapsam = {şube}`, `sube.kase@1` gibi). Seçim EB1'de; eklenirse `docs/havuz-turleri.toml` satırı ve `havuz_denetle.py` yeşil olmalı.
- Çatı ekranı: "Bu mağazanın bilgisayarları ve yazıcıları": cihaz listesi, son görülme, roller, yazıcılar ve sahipleri, numara aralıkları, atlama olayları, bekleyen yazdırma işi sayısı (eşlerden bildirilen).
- Ş2 izolasyon düzeneği A şubesinde iki cihazla koşar (D21).

### 7.2 EB2: ortak yazdırma hizmeti (TerziTakip başlığı yazar)
- Ortak kitaplık, `program = "ortak"`. Yer önerisi: mevcut `taa-yazici` crate'i (bugün `yazici.denetle@1` sunuyor) ya da yeni ortak crate; yeni crate açılırsa `modul.toml` ve `sozlesme_denetle.py` yeşil olmalı. `yazdir.sun@1` paket genelinde tek sunucuda yer alır (denetim kuralı: yetenek benzersiz).
- Eş kapısı kitaplığı (kimlik, şifreli kanal, keşif, eş listesi denetimi) EB2 ve EB3'ün **ortak** parçasıdır; tek kapı, tek güvenlik duvarı kuralı. EB2 önce yazarsa EB3 aynısını kullanır; ikinci kapı açılmaz.
- Eş hizmeti bilgisayar açıkken programlar kapalı olsa da çalışmalıdır (yazıcı sahibi Tutaa kapalıyken de basar). **Karar (E7, ilk sürüm):** kullanıcı oturumunda açılışta başlayan arka plan süreci. Gerekçe: kasa bilgisayarında oturum açık durur; Windows yazıcıları ve yerel şifreli depo (DPAPI) kullanıcıya bağlıdır. Oturum kapalıyken yazdırma bekler; ekran mesajı "kapalı ya da oturum açık değil" olur (`yazdir.sun.md` "Ekran mesajları"). EB5'ten sonra gerekirse Windows hizmetine geçilir.
- Şifreli kanal `rustls` ile (E9, Yusuf onaylı). Keşif §5.5, sürüm uyuşmazlığı §5.6 bu adımda yazılır.
- Hizmet kapalıyken doğrudan yazdırma (K1) aynı kitaplığın yerel basma kodudur; programlar ikinci yazdırma kodu tutmaz (K7).
- **A4 PDF basım aracı (K8):** EB2'de adıyla yazılır ve `yazdir.sun.md` Kural 14'e eklenir; boyut ve sayfa sınırı orada.
- Çekmece komutu taraması (K2) ve içerik yolu denetimi (K3) EB2'nin güvenlik incelemesinde ayrıca bakılır.
- Bağlayanlar: TAA Etiket (`etiket.yazdir@1` içeriği), Tutaa (Elektron; bugün doğrudan yerel yazıcıya basıyor, kod değişikliği ister), Terzi Takip, TAA Kasa (`fis.yazdir@1` ile gün sonu çıktısı; `kasa.say@1`/`kasa.kapat@1` yazdırmaz; K7). Elektron programları köprü üzerinden `yazdir.sun@1` ister.
- Güvenlik incelemesi Opus (ana plan §16.3).

### 7.3 EB3: eşler arası eşleme ve numara aralığı (TerziTakip, TT3 ve Ş4 ile)
- §3 ve §4 bu adımın bağlayıcı kurallarıdır. TT3 (çakışma ekranı) ve Ş4 (fiş uçları) ile aynı çakışma kaydını kullanır; ayrı bir çakışma biçimi açılmaz.
- Numara biçimi, yedek bölge sınırları ve güvenlik payı Ş4 ile kesinleşir (AÇIK-E5).
- `cihaz_sira` Ş0 §4.1'deki cihaz sıra numarasıyla aynı sayaçtır; ikinci sayaç açılmaz.

### 7.4 EB4: TAA Kur (KU6 ile)
- "Bu mağazaya ikinci bilgisayar ekle" akışı: cihaz anahtarı üret, şubeye kayıt başvurusu, Çatı onayı bekle, eş listesini al, şube imza anahtarı başvurusu.
- Yazıcı seçimi: "Bu bilgisayara bağlı yazıcılar hangileri?" (etiket, fiş, A4; her biri için yok/bu bilgisayar; aynı türden birden çok olabilir). Kayıt sunucuya gider (§7.1). Esenyurt yazıcı yerleşimi AÇIK (Yusuf'a soruldu, AÇIK-E3); yanıt gelse de kurulumda sorulur.
- Güvenlik duvarı kuralı (§5.1 madde 7); ağ profili denetimi; eş çıkarılınca kuralın kaldırılması.
- Yazıcı seçimi aynı türden birden çok yazıcıyı kabul eder; her bilgisayar için tür başına varsayılan yazıcı sorulur (K9).
- Kurulum sonunda sınama: öteki eşe bağlan (keşif §5.5), deneme sayfası iste (`yazdir.sun@1` deneme işi), sonuç ekranda. İki bilgisayar birbirini göremiyorsa (kablosuz istemci yalıtımı, farklı alt ağ) bunu açıkça söyler (K4). Protokol sürümleri uyuşmuyorsa söyler (K6).
- İlk eşlemeyi başlatır ve ilerlemesini gösterir (§4.6, K5).
- Tek bilgisayarlı mağazada bu akış hiç görünmez.

## 8. Açık noktalar ve kararlar
Kararlar: Fable denetimi `DENETIM-EB0-ESLEME-YAZDIR-2026-11-15` ve eki (Yusuf'un kart yanıtları, 2026-11-15 15:34 UTC).
- **AÇIK-E1 — kapandı (karar):** kodda alan adları `sube_*`, ekranda "mağaza". (Rehberde `tur = magaza` bir şubenin altındaki satış noktasıdır, ör. Esenyurt Outlet.)
- **AÇIK-E2 — kapandı (karar):** Esenyurt'taki iki bilgisayar `istanbul-merkez` şubesine bağlanır, önek `MRK` (Yusuf 15:19).
- **AÇIK-E3 — AÇIK (Yusuf'a soruldu):** Esenyurt'ta hangi yazıcının (etiket, fiş, A4) hangi bilgisayara bağlı olduğu; yazıcıların USB mi ağ yazıcısı mı (ör. `tcp://IP:9100`) olduğu; iki bilgisayarın aynı modemde, kabloyla mı kablosuz mu bağlı olduğu. EB4 yine kurulumda sorar; yanıt gelirse EB5 senaryoları daraltılır.
- **AÇIK-E4 — kapandı (karar):** ağ yazıcısında da tek sahip; sahibi kapalıyken ötekinin doğrudan basması yalnız Çatı'dan sahip değiştirilerek yapılır.
- **AÇIK-E5 — kapandı (öneri kabul):** numara biçimi önerisi geçerli; sayılar (bölge sınırı, blok boyu, güvenlik payı) ve Tutaa numaralandırmasıyla uyum Ş4'te kesinleşir. Fişte ve ekranda küçük "yedek numara" işareti kalır.
- **AÇIK-E6 — kapandı (karar, Yusuf 15:34 ile netleşti):** nakit sayımı çekmece başına; Z raporu ve POS gün sonu kasa noktası başına; bir çekmeceye birden çok kasa noktası bağlanabilir. Esenyurt'ta tek nakit çekmece var, iki kasa bilgisayarı paylaşır (§4.5). KS1 buna göre yazılır.
- **AÇIK-E7 — kapandı (karar, ilk sürüm):** eş hizmeti kullanıcı oturumunda açılışta başlayan arka plan sürecidir; EB5'ten sonra gerekirse Windows hizmetine geçilir (§7.2).
- **AÇIK-E8 — kapandı (karar):** telefon sunucusu sıra tutan eşte çalışır; kapısı eş kapısından ayrıdır (§4.5, §5.3).
- **AÇIK-E9 — kapandı (Yusuf 15:34):** `rustls` eklenir (TLS 1.3, karşılıklı kimlik, anahtar sabitleme) (§5.1 madde 4).
- **AÇIK-E10 — kapandı (karar):** eş listesi tazelik sınırı 7 gün; aşımda yalnız okuma ve yazdırma, numara bloğu yok; 5. günden itibaren ekranda uyarı (§5.2).
- **AÇIK-E11 — koordinatöre bırakıldı:** şube planı §2'deki "şubede tek PC olduğu için çakışma daralır" varsayımını o planın sahibi düzeltir; koordinatör iletir.
- **AÇIK-Y1 — kapandı (Yusuf 15:34, K9):** aynı türden birden çok yazıcı olabilir; `yazici_id` ve bilgisayar başına varsayılan yazıcı (`yazdir.sun.md`).
- **AÇIK-Y2 — kapandı (karar):** etiket yazıcısı Argox; dil ve model TAA Etiket başlığından alınır.
- **AÇIK-Y3 — AÇIK (Yusuf'a soruldu):** AÇIK-E3 ile aynı soru.
- **AÇIK-S1, AÇIK-S2 — kapandı** (ana plan §16.1): Ş0 belgelerinde D5, D2, D20 ile işaretlendi.
- EB2'de yazılacaklar (açık değil, adım işi): A4 PDF basım aracının adı (K8), eş TCP ve duyuru UDP kapı numaraları (§5.5), içerik arşiv süresinin KVKK saklama kuralıyla kesinleşmesi (`yazdir.sun.md` Kural 13).
