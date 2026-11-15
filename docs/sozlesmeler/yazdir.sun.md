<!-- Sahip: Yusuf. yazan: [Claude#sozlesme-EB0 2026-11-15]. Durum: SÖZLEŞME (EB0). Fable denetimi 2026-11-15: GEÇTİ (koşullu); K1, K2, K3, K6, K7, K8 ve ek K9 (Yusuf 15:34) bu dosyaya işlendi. -->
# yazdir.sun

- Tür: yetenek
- Sürüm: @1
- Sunan: ortak yazdırma hizmeti (EB2; `program = "ortak"`, yer önerisi `taa-yazici` ya da yeni ortak crate, EB2'de kesinleşir). Her bilgisayarda bir tane çalışır.
- Özet: Mağazadaki doğru yazıcıya (etiket, fiş, A4) iş gönderme, durum sorma, iptal. Yazıcı başka eşe bağlıysa iş yerel ağdan o eşe gider.
- İsteyenler: TAA Etiket, Tutaa, Terzi Takip, TAA Kasa (`ister`; yetenek yoksa ya da hizmet yanıt vermiyorsa ve yazıcı bu bilgisayardaysa doğrudan yerel yazdırma sürer, bağımlılık yok; Kural 11)
- Çağrı: program → kendi bilgisayarındaki hizmet: `taa-ipc` (`docs/ipc.md`). Hizmet → yazıcı sahibi eş: eş kapısı (`docs/esleme-sozlesmesi.md` §5). Program eş kapısına doğrudan bağlanmaz.
- Kaynak: `planlar/PLAN-TAA-PAKET-ANA-2026-11-15.md` §16.2 madde 2; `docs/esleme-sozlesmesi.md` §2; Fable denetimi `DENETIM-EB0-ESLEME-YAZDIR-2026-11-15` (K1–K8).
- Hizmetin çalışma biçimi (E7 kararı, ilk sürüm): kullanıcı oturumunda açılışta başlayan arka plan süreci; programlar kapalıyken de çalışır, oturum kapalıyken çalışmaz. EB5'ten sonra gerekirse Windows hizmetine geçilir.

## Kavramlar
- **Yazıcı türü:** `etiket`, `fis`, `a4`. Bir mağazada **aynı türden birden çok etkin yazıcı** olabilir (Yusuf 15:34, K9; AÇIK-Y1 kapandı). Etiket yazıcısı TAA Etiket'in kullandığı Argox'tur.
- **Varsayılan yazıcı (K9):** her bilgisayar için, her tür için bir varsayılan yazıcı kayıtlıdır (`varsayilan_yazici`: `cihaz_id`, `tur`, `yazici_id`, `surum`). Kaynak sunucu (EB1); kurulumda TAA Kur kaydeder, Çatı'dan ya da çıktı ekranından değişir. Türde tek yazıcı varsa o yazıcı kendiliğinden varsayılandır.
- **Çekmece:** nakit sayımı çekmece başına tutulur (`docs/esleme-sozlesmesi.md` §4.5, E6 kararı). Çekmece elle açılan sıradan bir çekmece olabilir (Esenyurt'taki tek çekmece böyledir; bu hizmetle ilgisi yoktur) ya da bir fiş yazıcısına kabloyla bağlı olabilir. Yazıcıya bağlı çekmece o yazıcının sahibi olan tek bilgisayara aittir; çekmece kaydı `cekmece_id`, `sube_kodu`, `yazici_id` (yalnız yazıcıya bağlıysa), `ad`. Yazıcıya bağlı çekmece yalnız `cekmece_ac` işlemiyle açılır (Kural 12).
- **Yazıcı kaydı:** `yazici_id`, `sube_kodu`, `tur`, `sahip_cihaz` (bağlı olduğu eşin cihaz kimliği), `baglanti` (`windows:<yazıcı adı>` ya da `tcp://<yerel IP>:9100`), `ad`, `model` (bilgi), `durum` (`etkin|pasif`), `surum`. Kaynak sunucu (EB1); kurulumda TAA Kur kaydeder (EB4), Çatı'dan değiştirilir. Her eşte önbellekte durur; sunucu yokken son kayıt kullanılır.
- **İş kimliği (`is_id`):** isteyen program "Yazdır" basıldığı anda üretir (uuid v4, küçük harf; `docs/ipc.md` `kimlik_gecerli` kuralına uyar). Bir basış = bir kimlik. Çift tık, yeniden deneme, ağ kopması sonrası yeniden gönderim **aynı kimlikle** yapılır.
- **Çift basmama:** yazıcı sahibi, aldığı her `is_id`'yi basılan işler defterine yazar (en az 30 gün). Aynı kimlik yeniden gelirse basmaz, kayıtlı durumu döner. Bir kimlik en çok bir kez basılır.
- **"Yeniden bas"** kullanıcının bilinçli eylemidir: yeni `is_id` üretilir, `yeniden_basim` alanı eski kimliği gösterir; deftere yazılır.

## İşlemler
İstek gövdesinde `islem` alanı: `gonder`, `durum`, `iptal`, `yazicilar`, `cekmece_ac`. `cekmece_ac` yalnız yerel IPC'den kabul edilir; eş kapısında bu işlem yoktur.

## İstek alanları
| İşlem | Alan | Tür | Zorunlu | Not |
|---|---|---|---|---|
| hepsi | `islem` | metin | evet | Yukarıdaki beş değerden biri |
| `gonder` | `is_id` | metin | evet | Tek kimlik |
| `gonder` | `tur` | `etiket\|fis\|a4` | evet | Yazıcı türü |
| `gonder` | `yazici_id` | metin | hayır | K9. Verilirse o yazıcı (türü `tur` ile aynı, mağazanın etkin yazıcısı olmalı); verilmezse isteyen bilgisayarın bu tür için varsayılan yazıcısı. Program yazıcıyı yalnız kullanıcı çıktı ekranında seçtiyse gönderir |
| `gonder` | `icerik_turu` | `escpos\|pdf\|etiket-ham\|metin` | evet | İçerik isteyen bilgisayarda hazırlanır; yazıcı sahibi yeniden biçimlendirmez. `etiket-ham` etiket yazıcısının kendi dili (Argox; dil ve model TAA Etiket başlığından alınır, AÇIK-Y2 kapandı) |
| `gonder` | `icerik_yolu` | metin | evet | Yerel içerik dosyası; **yalnız** `<kök>\yazdir\is\<is_id>.bin` kabul edilir (Kural 13). IPC zarfı 1 MiB sınırı yüzünden içerik zarfa konmaz |
| `gonder` | `icerik_sha256` | metin | evet | Hizmet içeriği okuyunca doğrular; eşe de bu özetle gider |
| `gonder` | `kopya` | tam sayı | hayır | Varsayılan 1, en çok 10 |
| `gonder` | `belge` | nesne | hayır | `{tur, no}` (ör. `{"tur":"fis","no":"MRK-000123"}`); defter ve ekran için. Kişi verisi içermez |
| `gonder` | `kaynak_program` | metin | evet | `program_adi_gecerli` biçimi |
| `gonder` | `kullanici` | metin | evet | Oturumdaki kullanıcı kimliği (ad değil) |
| `gonder` | `son_gecerlilik` | RFC 3339 UTC | hayır | Verilirse bu zamandan sonra basılmaz, `suresi_doldu` olur. Verilmezse iş süresiz bekler |
| `gonder` | `yeniden_basim` | metin | hayır | Yeniden basımda eski `is_id` |
| `durum` | `is_id` | metin | evet | |
| `iptal` | `is_id` | metin | evet | Yalnız `kuyrukta` ya da `bekliyor_sahip_kapali` iken |
| `yazicilar` | `tur` | metin | hayır | Verilirse yalnız o türün yazıcıları; yanıt tür başına birden çok yazıcı döner |
| `cekmece_ac` | `cekmece_id` | metin | evet | Bu bilgisayardaki fiş yazıcısına bağlı çekmece |
| `cekmece_ac` | `neden` | `satis\|para_ustu\|gun_sonu_sayim\|diger` | evet | Deftere yazılır |
| `cekmece_ac` | `belge` | nesne | hayır | `{tur, no}`; satışa bağlıysa fiş numarası |
| `cekmece_ac` | `kullanici`, `kaynak_program` | metin | evet | `gonder` ile aynı kurallar |

## Yanıt alanları
| İşlem | Alan | Not |
|---|---|---|
| `gonder`, `durum`, `iptal` | `is_id` | |
| | `durum` | `kuyrukta` (yerelde, eşe gitmedi) · `bekliyor_sahip_kapali` (yazıcı sahibi eşe ulaşılamıyor: kapalı, oturum açık değil ya da ağda değil) · `bekliyor_surum_uyusmaz` (eşle protokol sürümü uyuşmuyor, Kural 15) · `sahipte` (yazıcı sahibinin kuyruğunda) · `basiliyor` · `basildi` · `hata` · `iptal` · `suresi_doldu` · `belirsiz` (basılırken çöküş; basılıp basılmadığı bilinmiyor) |
| | `yazici` | `{yazici_id, ad, tur, sahip_cihaz_adi}` |
| | `mesaj` | Ekranda gösterilecek Türkçe sade cümle (aşağıda) |
| | `hata` | Durum `hata` ise hata kodu |
| | `zaman` | Son durum değişimi, UTC |
| `yazicilar` | `yazicilar[]` | `{yazici_id, tur, ad, sahip_cihaz_adi, varsayilan: bool (bu bilgisayar için), erisim: "bu_bilgisayar\|es_acik\|es_kapali", yazici_durumu}`; tür başına birden çok öğe olabilir; `yazici_durumu` `yazici.denetle@1`'den (varsa) |

Durum değişince hizmet `yazdir.durum@1` olayını yayınlar (`taa-ipc` olay günlüğü; yük: `is_id`, `durum`, `mesaj`). İsteyen program olayı izler ya da `durum` ile yoklar. Olay adı ayrı sözleşme dosyası ister (EB2).

## Ekran mesajları (bağlayıcı metin)
| Durum | Mesaj |
|---|---|
| `bekliyor_sahip_kapali` | "Yazıcının bağlı olduğu bilgisayar (<ad>) kapalı, oturumu açık değil ya da ağda değil. İş bekliyor, kaybolmadı. Bilgisayar açılınca bir kez basılacak." |
| `bekliyor_surum_uyusmaz` | "<ad> bilgisayarındaki program sürümü bu bilgisayarla aynı değil. Güncelleme bitene kadar bu yazıcıya buradan basılamıyor. İş bekliyor, kaybolmadı." |
| `sahipte` | "<ad> bilgisayarındaki yazıcıya gönderildi." |
| `basildi` | "Basıldı (<yazıcı adı>)." |
| `hata` | Hata kodunun mesajı (aşağıda) + "Düzeltip yeniden deneyin." |
| `belirsiz` | "Baskı sırasında kesinti oldu; basılıp basılmadığı bilinmiyor. Yazıcıya bakın; basılmadıysa Yeniden bas'a dokunun." |
| `suresi_doldu` | "İş zamanında basılamadı ve iptal edildi." |
Bekleyen iş, iptal edilene ya da basılana kadar isteyen programın ekranında "Bekleyen çıktılar (N)" olarak görünür. Gün değişince bekleyen iş varsa bir kez hatırlatılır.

## Kurallar
1. **Yönlendirme (K9):** hizmet yazıcıyı `yazici_id` verilmişse ondan, verilmemişse isteyen bilgisayarın `tur` için varsayılan yazıcısından bulur. Sahibi bu bilgisayarsa yerelde basar; değilse eş kapısından sahibine iletir. Çıktı ekranında yazıcı seçimi **yalnız o türde birden çok yazıcı varsa** görünür; tek yazıcıda kullanıcı hiçbir şey seçmez. Seçim o işe özeldir; "varsayılan yap" ayrıca işaretlenirse varsayılan değişir (sunucuya yazılır, sunucu yoksa kuyruğa alınır). Varsayılan yazıcı pasifleşirse ya da yoksa ve türde birden çok yazıcı varsa iş basılmaz, `yazici_secilmeli` döner.
2. **Kuyruk kalıcıdır:** iş, kabul edildiği anda isteyen bilgisayarın diskine yazılır (atomik); bilgisayar kapanıp açılınca kuyruk sürer. Eşe gidince sahibin diskine de yazılır; sahip "aldım" demeden yerel kopya silinmez (silme yok kuralı gereği işlenenler taşınır).
3. **En çok bir kez:** yazıcı sahibi işi yazıcıya göndermeden önce durumu `basiliyor` olarak diske yazar. Açılışta `basiliyor` durumunda iş bulursa onu **yeniden basmaz**, `belirsiz` yapar.
4. **Sahip değişirse** (Çatı'dan): eski sahipte `sahipte` durumunda bekleyen işler yeni sahibe aktarılır; basılan işler defteri de aktarılır. Aktarım bitene kadar eski sahip yeni iş almaz.
5. **Sahip kapalıyken** iş başka yazıcıya ya da başka eşe kendiliğinden yönlendirilmez (çift basma riski). İstisna: `a4` için kullanıcı ekranda "Bu bilgisayardan Windows paylaşımıyla bas" seçebilir; bu yeni `is_id` ile ayrı iştir ve eski iş iptal edilir.
6. **Yetki:** eş kapısından yalnız aynı mağazanın kimliği doğrulanmış eşi iş gönderebilir. Yerelden yalnız aynı Windows kullanıcısının programları (`docs/ipc.md` güvenlik sınırı).
7. **Sınırlar (öneri, ölçülünce kesinleşir):** içerik en çok 10 MiB; eş başına 60 iş/dk; sahipte bekleyen iş en çok 500. Aşımda `hiz_siniri` ya da `kuyruk_dolu`.
8. **Defter:** her iş için `is_id`, tür, yazıcı, isteyen cihaz, kullanıcı kimliği, `belge`, durum geçişleri. İçerik dosyası 30 gün sonra arşive taşınır. Defter eşlenir (`esleme-sozlesmesi` §4.2); Çatı bekleyen iş sayısını görür.
9. **Sunucu gerekmez:** gönder, durum, iptal yalnız yerel ağla çalışır. Yazıcı kaydı değişikliği sunucu ister (EB1).
10. **Tek bilgisayarlı mağaza:** tüm yazıcılar bu bilgisayardadır, eş kapısı kullanılmaz; aynı kimlik ve çift basmama kuralları yine geçerlidir.
11. **Hizmet yanıt vermezse (K1):** yazıcı bu bilgisayardaysa ve hizmet IPC'de yanıt vermiyorsa (bağlanılamıyor ya da 3 sn zaman aşımı) program doğrudan yerel yazdırmaya döner ve bunu yerel deftere `yazdir.dogrudan` olayı olarak yazar (`is_id`, tür, neden, zaman). Doğrudan yazdırma, ortak yazdırma kitaplığının yerel basma kodunun program içinden çağrılmasıdır; ayrı ikinci bir yazdırma kodu yoktur (Kural 16). Çift basmamak için program `is_id`'yi kendi küçük defterinde tutar; hizmet dönünce bu defter hizmetin defterine aktarılır, aynı kimlik yeniden basılmaz. Yazıcı eşteyse dönüş yoktur: iş yerel kuyrukta bekler, hizmet açılınca gönderilir. Böylece tek bilgisayarlı mağazada yazdırma yeni bir arıza noktası kazanmaz.
12. **Çekmece uzaktan açılmaz (K2):** eşten gelen `escpos` içeriği yazıcı sahibinde taranır; çekmece açma komutu (ESC p `1B 70`, DLE DC4 `10 14 01`) bulunursa iş basılmaz, `cekmece_red` döner, `es.red` deftere yazılır. Yerel `gonder` içeriği de çekmece komutu taşımaz (aynı tarama; içerik üreten yetenekler bu komutu üretmez). Yazıcıya bağlı çekmece yalnız bağlı olduğu bilgisayardan, yerel `cekmece_ac` işlemiyle açılır (elle açılan sıradan çekmece bu hizmetin dışındadır; Esenyurt'taki tek çekmece böyledir); her açılış deftere `cekmece.acildi` (cekmece, kullanıcı, neden, belge, zaman) olarak yazılır ve eşlenir.
13. **İçerik yolu sınırı (K3):** hizmet yalnız kendi `<kök>\yazdir\is\` klasöründen, adı `<is_id>.bin` olan dosyayı okur. Yol kanonik hale getirilip denetlenir; başka klasör, üst klasör (`..`), başka sürücü, UNC yolu, kısayol ve bağlantı (symlink, junction, hard link) reddedilir: `yol_red`. Bekleyen ve basılmış iş içerikleri müşteri verisi taşır: diskte öteki yerel veriler gibi DPAPI ile sarılı durur (isteyende ve yazıcı sahibinde). Arşiv süresi (KARAR, Yusuf 2026-11-15 15:56): baskı kopyası (içerik dosyası) basıldıktan 30 gün sonra arşive alınır, arşivde 180 gün kalır, sonra yalnız içerik dosyası kaldırılır; baskı kaydının kendisi (defter satırı, kişi verisi içermez) kalır. Bu, "silme yok" kuralının bilinçli ve dar istisnasıdır (yalnız baskı kopyası içeriği).
14. **A4 PDF basımı (K8):** yazıcı sahibi eşten gelen PDF'i ayrı ve düşük yetkili bir süreçte açıp basar; kullanılacak araç EB2'de adıyla yazılır (bu sözleşmeye eklenir). Sınırlar: dosya en çok 10 MiB, en çok 50 sayfa (öneri, ölçülünce kesinleşir), açma ve basma için 60 sn zaman aşımı. Sınırı aşan dosya `cok_buyuk`, açılamayan ya da bozuk dosya `pdf_bozuk` ile reddedilir; araç çökerse ya da zaman aşarsa süreç sonlandırılır, iş `hata` olur, hizmet ve yazıcılı bilgisayar çalışmaya devam eder. PDF içindeki betik, ek dosya ve bağlantı çalıştırılmaz.
15. **Sürüm uyuşmazlığı (K6):** eş kapısı el sıkışmasında iki taraf protokol sürümünü bildirir (`docs/esleme-sozlesmesi.md` §5.6). Uyuşmazsa kapı kapanmaz ama eşten iş alınmaz ve eşe iş gönderilmez; mağaza "yalnız merkez yoluyla eşleme, yalnız yerel yazdırma" kipine düşer. Yazıcısı bu bilgisayarda olan işler basılır; eşteki yazıcıya giden işler `bekliyor_surum_uyusmaz` durumunda bekler, sürümler eşitlenince bir kez basılır. Eski protokol sürümüne düşülmez.
16. **Mevcut yeteneklerle ilişki (K7):** içerik üreten yetenekler aynen kalır, yazıcıya kendileri basmaz, ürettikleri çıktıyı `yazdir.sun@1`'e verir: `fis.yazdir@1` (taa-fis) fiş için `escpos`/`metin` içerik üretir; `etiket.yazdir@1` (TAA Etiket) etiket için `etiket-ham` içerik üretir; `kasa.say@1` ve `kasa.kapat@1` (taa-kasa) sayım ve kapanış kaydı üretir, yazdırmaz; gün sonu çıktısı `fis.yazdir@1` ile içeriğe çevrilir ve `yazdir.sun@1` ile basılır. Yazıcıya giden tek yol bu hizmettir (Kural 11'deki doğrudan yazdırma da aynı kitaplığın yerel basma kodudur); hiçbir program ikinci bir yazdırma yolu tutmaz. Çekmece açma `fis.yazdir@1` çıktısına gömülmez, yalnız `cekmece_ac` ile yapılır.

## Hata kodları
Gövde: `{"hata": "<kod>", "mesaj": "<Türkçe sade cümle>"}` (Ş0 §9.6 biçimi). Mesaj başka mağazanın adını, kişi verisini, iç ayrıntıyı içermez.
| Kod | Ne zaman | Mesaj (öneri) |
|---|---|---|
| `gecersiz_istek` | Şema, biçim, `is_id` biçimi | "İstek geçersiz." |
| `yazici_yok` | Bu tür için kayıtlı etkin yazıcı yok, ya da verilen `yazici_id` bu mağazanın etkin yazıcısı değil ya da türü uymuyor | "Bu mağaza için <tür> yazıcısı tanımlı değil. Yöneticiye bildirin." |
| `yazici_secilmeli` | Türde birden çok yazıcı var, `yazici_id` yok ve geçerli varsayılan yok (K9) | "Birden çok <tür> yazıcısı var; bir yazıcı seçin." |
| `bicim_desteklenmez` | `icerik_turu` bu yazıcı türünde basılamaz | "Bu çıktı bu yazıcıda basılamaz." |
| `icerik_bozuk` | Dosya yok ya da sha256 tutmuyor | "Çıktı dosyası okunamadı; yeniden deneyin." |
| `cok_buyuk` | İçerik sınırı aşıldı | "Çıktı çok büyük." |
| `is_tekrar` | Aynı `is_id`, farklı içerik özeti | "Bu iş kimliği başka bir çıktıya ait." |
| `is_bulunamadi` | `durum`/`iptal` için bilinmeyen kimlik | "İş bulunamadı." |
| `iptal_edilemez` | İş `basiliyor`, `basildi` ya da bitmiş | "İş basıldığı ya da basılmakta olduğu için iptal edilemez." |
| `yazici_hatasi` | Yazıcı kağıt yok, kapak açık, çevrimdışı (ayrıntı `yazici.denetle@1`) | "Yazıcı hata verdi: <kısa neden>." |
| `es_red` | Eş kimliği doğrulanamadı, eş listesinde yok ya da başka mağaza | "Bu bilgisayar mağazanın kayıtlı bilgisayarı değil." |
| `hiz_siniri` | Eş başına sınır aşıldı | "Çok fazla çıktı isteği; biraz sonra deneyin." |
| `kuyruk_dolu` | Bekleyen iş sınırı | "Bekleyen çıktı çok; önce bekleyenleri basın ya da iptal edin." |
| `yol_red` | `icerik_yolu` iş klasörü dışında, adı `is_id` değil, kısayol ya da bağlantı (Kural 13) | "Çıktı dosyası geçersiz yerde." |
| `cekmece_red` | Eşten gelen içerikte çekmece komutu, eş kapısından `cekmece_ac`, ya da çekmece bu bilgisayara bağlı değil (Kural 12) | "Para çekmecesi yalnız bağlı olduğu bilgisayardan açılır." |
| `pdf_bozuk` | PDF açılamadı, bozuk ya da basım aracı zaman aştı (Kural 14) | "PDF dosyası açılamadı; dosyayı yeniden oluşturun." |
`bekliyor_sahip_kapali` ve `bekliyor_surum_uyusmaz` hata değil durumdur: iş kabul edilmiştir.

## Bütçe (ms)
Öneri, ölçülmedi [ÖLÇ]: `gonder` yanıtı (kuyruğa alındı) ≤ 200; aynı yerel ağda eşe ulaşma ≤ 1.000; eşe bağlanma zaman aşımı 3.000 (sonra `bekliyor_sahip_kapali`, arka planda yeniden dener).

## Kabul testleri
`yazdir_yerel_basar`, `yazdir_ese_yonlenir`, `yazdir_cift_basma_yok`, `yazdir_yeniden_gonderim_tek_baski`, `yazdir_cift_tik_tek_baski`, `yazdir_sahip_kapali_bekler`, `yazdir_sahip_acilinca_bir_kez`, `yazdir_cokus_belirsiz_durum`, `yazdir_iptal_kuyrukta`, `yazdir_iptal_basildiktan_sonra_red`, `yazdir_sahip_degisimi_aktarir`, `yazdir_yazici_yok_mesaji`, `yazdir_is_tekrar_farkli_icerik_red`, `yazdir_kayitsiz_es_red`, `yazdir_hiz_siniri`, `yazdir_kuyruk_dolu`, `yazdir_bicim_red`, `yazdir_is_deftere_yazilir`, `yazdir_suresi_doldu_basilmaz`, `yazdir_tek_pc_es_kapisi_yok`, `yazdir_hizmet_kapali_dogrudan_basar` (K1; `tek_pc_degismez` içinde de koşar), `yazdir_dogrudan_deftere_yazilir`, `yazdir_dogrudan_sonra_cift_basma_yok`, `yazdir_esten_cekmece_komutu_red` (K2), `yazdir_cekmece_esten_ac_red`, `yazdir_cekmece_acma_deftere_yazilir`, `yazdir_yol_disari_red` (K3), `yazdir_yol_kisayol_red`, `yazdir_icerik_sarili`, `yazdir_pdf_boyut_siniri` (K8), `yazdir_pdf_sayfa_siniri`, `yazdir_pdf_bozuk_red`, `yazdir_surum_uyusmaz_yerel_kip` (K6), `yazdir_varsayilan_yazici` (K9), `yazdir_yazici_secimi` (K9; seçim yalnız birden çok yazıcıda görünür), `yazdir_yazici_id_baska_magaza_red`, `yazdir_varsayilan_yok_secim_ister`. Gerçek yazıcı denemesi: `docs/esleme-sozlesmesi.md` §6.2.

## Açık noktalar
- AÇIK-Y1 — kapandı (Yusuf 2026-11-15 15:34): aynı türden birden çok yazıcı olabilir; `yazici_id` ve bilgisayar başına varsayılan yazıcı eklendi (K9).
- AÇIK-Y2 — kapandı (Fable kararı): etiket yazıcısı TAA Etiket'in kullandığı Argox'tur; `etiket-ham` dili ve model TAA Etiket başlığından alınır, Yusuf'a sorulmaz.
- AÇIK-Y3 — AÇIK (Yusuf'a soruldu): Esenyurt'ta hangi yazıcının hangi bilgisayarda olduğu, USB mi ağ mı olduğu (`esleme-sozlesmesi` AÇIK-E3). EB4 yine kurulumda sorar.
- PDF basım aracının adı EB2'de yazılır (Kural 14).

## Sürüm kuralı
Değişirse `@2` yeni dosya; eski `@1` en az bir sürüm daha yaşar (§18.3).
