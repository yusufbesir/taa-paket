<!-- Sahip: Yusuf. yazan: [Claude#sozlesme-EB0 2026-11-15]. Durum: SÖZLEŞME (EB0), Fable denetimi bekler. -->
# yazdir.sun

- Tür: yetenek
- Sürüm: @1
- Sunan: ortak yazdırma hizmeti (EB2; `program = "ortak"`, yer önerisi `taa-yazici` ya da yeni ortak crate, EB2'de kesinleşir). Her bilgisayarda bir tane çalışır.
- Özet: Mağazadaki doğru yazıcıya (etiket, fiş, A4) iş gönderme, durum sorma, iptal. Yazıcı başka eşe bağlıysa iş yerel ağdan o eşe gider.
- İsteyenler: TAA Etiket, Tutaa, Terzi Takip, TAA Kasa (`ister`; yetenek yoksa bugünkü doğrudan yerel yazdırma sürer, bağımlılık yok)
- Çağrı: program → kendi bilgisayarındaki hizmet: `taa-ipc` (`docs/ipc.md`). Hizmet → yazıcı sahibi eş: eş kapısı (`docs/esleme-sozlesmesi.md` §5). Program eş kapısına doğrudan bağlanmaz.
- Kaynak: `planlar/PLAN-TAA-PAKET-ANA-2026-11-15.md` §16.2 madde 2; `docs/esleme-sozlesmesi.md` §2.

## Kavramlar
- **Yazıcı türü:** `etiket`, `fis`, `a4`. Mağazada tür başına bir etkin yazıcı (birden çok A4 gerekirse `@2` ya da ek alan; AÇIK-Y1).
- **Yazıcı kaydı:** `yazici_id`, `sube_kodu`, `tur`, `sahip_cihaz` (bağlı olduğu eşin cihaz kimliği), `baglanti` (`windows:<yazıcı adı>` ya da `tcp://<yerel IP>:9100`), `ad`, `model` (bilgi), `durum` (`etkin|pasif`), `surum`. Kaynak sunucu (EB1); kurulumda TAA Kur kaydeder (EB4), Çatı'dan değiştirilir. Her eşte önbellekte durur; sunucu yokken son kayıt kullanılır.
- **İş kimliği (`is_id`):** isteyen program "Yazdır" basıldığı anda üretir (uuid v4, küçük harf; `docs/ipc.md` `kimlik_gecerli` kuralına uyar). Bir basış = bir kimlik. Çift tık, yeniden deneme, ağ kopması sonrası yeniden gönderim **aynı kimlikle** yapılır.
- **Çift basmama:** yazıcı sahibi, aldığı her `is_id`'yi basılan işler defterine yazar (en az 30 gün). Aynı kimlik yeniden gelirse basmaz, kayıtlı durumu döner. Bir kimlik en çok bir kez basılır.
- **"Yeniden bas"** kullanıcının bilinçli eylemidir: yeni `is_id` üretilir, `yeniden_basim` alanı eski kimliği gösterir; deftere yazılır.

## İşlemler
İstek gövdesinde `islem` alanı: `gonder`, `durum`, `iptal`, `yazicilar`.

## İstek alanları
| İşlem | Alan | Tür | Zorunlu | Not |
|---|---|---|---|---|
| hepsi | `islem` | metin | evet | Yukarıdaki dört değerden biri |
| `gonder` | `is_id` | metin | evet | Tek kimlik |
| `gonder` | `tur` | `etiket\|fis\|a4` | evet | Yazıcı türü; yazıcıyı hizmet kayıttan seçer, program seçmez |
| `gonder` | `icerik_turu` | `escpos\|pdf\|etiket-ham\|metin` | evet | İçerik isteyen bilgisayarda hazırlanır; yazıcı sahibi yeniden biçimlendirmez. `etiket-ham` etiket yazıcısının kendi dili (model bilinince kesinleşir, AÇIK-Y2) |
| `gonder` | `icerik_yolu` | metin | evet | Yerel içerik dosyası (`<kök>\yazdir\is\<is_id>.bin`); IPC zarfı 1 MiB sınırı yüzünden içerik zarfa konmaz |
| `gonder` | `icerik_sha256` | metin | evet | Hizmet içeriği okuyunca doğrular; eşe de bu özetle gider |
| `gonder` | `kopya` | tam sayı | hayır | Varsayılan 1, en çok 10 |
| `gonder` | `belge` | nesne | hayır | `{tur, no}` (ör. `{"tur":"fis","no":"MRK-000123"}`); defter ve ekran için. Kişi verisi içermez |
| `gonder` | `kaynak_program` | metin | evet | `program_adi_gecerli` biçimi |
| `gonder` | `kullanici` | metin | evet | Oturumdaki kullanıcı kimliği (ad değil) |
| `gonder` | `son_gecerlilik` | RFC 3339 UTC | hayır | Verilirse bu zamandan sonra basılmaz, `suresi_doldu` olur. Verilmezse iş süresiz bekler |
| `gonder` | `yeniden_basim` | metin | hayır | Yeniden basımda eski `is_id` |
| `durum` | `is_id` | metin | evet | |
| `iptal` | `is_id` | metin | evet | Yalnız `kuyrukta` ya da `bekliyor_sahip_kapali` iken |
| `yazicilar` | — | — | — | Mağazanın yazıcıları ve anlık erişim durumu |

## Yanıt alanları
| İşlem | Alan | Not |
|---|---|---|
| `gonder`, `durum`, `iptal` | `is_id` | |
| | `durum` | `kuyrukta` (yerelde, eşe gitmedi) · `bekliyor_sahip_kapali` (yazıcı sahibi eşe ulaşılamıyor) · `sahipte` (yazıcı sahibinin kuyruğunda) · `basiliyor` · `basildi` · `hata` · `iptal` · `suresi_doldu` · `belirsiz` (basılırken çöküş; basılıp basılmadığı bilinmiyor) |
| | `yazici` | `{yazici_id, ad, tur, sahip_cihaz_adi}` |
| | `mesaj` | Ekranda gösterilecek Türkçe sade cümle (aşağıda) |
| | `hata` | Durum `hata` ise hata kodu |
| | `zaman` | Son durum değişimi, UTC |
| `yazicilar` | `yazicilar[]` | `{yazici_id, tur, ad, sahip_cihaz_adi, erisim: "bu_bilgisayar\|es_acik\|es_kapali", yazici_durumu}`; `yazici_durumu` `yazici.denetle@1`'den (varsa) |

Durum değişince hizmet `yazdir.durum@1` olayını yayınlar (`taa-ipc` olay günlüğü; yük: `is_id`, `durum`, `mesaj`). İsteyen program olayı izler ya da `durum` ile yoklar. Olay adı ayrı sözleşme dosyası ister (EB2).

## Ekran mesajları (bağlayıcı metin)
| Durum | Mesaj |
|---|---|
| `bekliyor_sahip_kapali` | "Yazıcının bağlı olduğu bilgisayar (<ad>) kapalı ya da ağda değil. İş bekliyor, kaybolmadı. Bilgisayar açılınca bir kez basılacak." |
| `sahipte` | "<ad> bilgisayarındaki yazıcıya gönderildi." |
| `basildi` | "Basıldı (<yazıcı adı>)." |
| `hata` | Hata kodunun mesajı (aşağıda) + "Düzeltip yeniden deneyin." |
| `belirsiz` | "Baskı sırasında kesinti oldu; basılıp basılmadığı bilinmiyor. Yazıcıya bakın; basılmadıysa Yeniden bas'a dokunun." |
| `suresi_doldu` | "İş zamanında basılamadı ve iptal edildi." |
Bekleyen iş, iptal edilene ya da basılana kadar isteyen programın ekranında "Bekleyen çıktılar (N)" olarak görünür. Gün değişince bekleyen iş varsa bir kez hatırlatılır.

## Kurallar
1. **Yönlendirme:** hizmet `tur` için kayıtlı yazıcıyı bulur. Sahibi bu bilgisayarsa yerelde basar; değilse eş kapısından sahibine iletir. Yazıcı seçimi programda yapılmaz.
2. **Kuyruk kalıcıdır:** iş, kabul edildiği anda isteyen bilgisayarın diskine yazılır (atomik); bilgisayar kapanıp açılınca kuyruk sürer. Eşe gidince sahibin diskine de yazılır; sahip "aldım" demeden yerel kopya silinmez (silme yok kuralı gereği işlenenler taşınır).
3. **En çok bir kez:** yazıcı sahibi işi yazıcıya göndermeden önce durumu `basiliyor` olarak diske yazar. Açılışta `basiliyor` durumunda iş bulursa onu **yeniden basmaz**, `belirsiz` yapar.
4. **Sahip değişirse** (Çatı'dan): eski sahipte `sahipte` durumunda bekleyen işler yeni sahibe aktarılır; basılan işler defteri de aktarılır. Aktarım bitene kadar eski sahip yeni iş almaz.
5. **Sahip kapalıyken** iş başka yazıcıya ya da başka eşe kendiliğinden yönlendirilmez (çift basma riski). İstisna: `a4` için kullanıcı ekranda "Bu bilgisayardan Windows paylaşımıyla bas" seçebilir; bu yeni `is_id` ile ayrı iştir ve eski iş iptal edilir.
6. **Yetki:** eş kapısından yalnız aynı mağazanın kimliği doğrulanmış eşi iş gönderebilir. Yerelden yalnız aynı Windows kullanıcısının programları (`docs/ipc.md` güvenlik sınırı).
7. **Sınırlar (öneri, ölçülünce kesinleşir):** içerik en çok 10 MiB; eş başına 60 iş/dk; sahipte bekleyen iş en çok 500. Aşımda `hiz_siniri` ya da `kuyruk_dolu`.
8. **Defter:** her iş için `is_id`, tür, yazıcı, isteyen cihaz, kullanıcı kimliği, `belge`, durum geçişleri. İçerik dosyası 30 gün sonra arşive taşınır. Defter eşlenir (`esleme-sozlesmesi` §4.2); Çatı bekleyen iş sayısını görür.
9. **Sunucu gerekmez:** gönder, durum, iptal yalnız yerel ağla çalışır. Yazıcı kaydı değişikliği sunucu ister (EB1).
10. **Tek bilgisayarlı mağaza:** tüm yazıcılar bu bilgisayardadır, eş kapısı kullanılmaz; aynı kimlik ve çift basmama kuralları yine geçerlidir.

## Hata kodları
Gövde: `{"hata": "<kod>", "mesaj": "<Türkçe sade cümle>"}` (Ş0 §9.6 biçimi). Mesaj başka mağazanın adını, kişi verisini, iç ayrıntıyı içermez.
| Kod | Ne zaman | Mesaj (öneri) |
|---|---|---|
| `gecersiz_istek` | Şema, biçim, `is_id` biçimi | "İstek geçersiz." |
| `yazici_yok` | Bu tür için kayıtlı etkin yazıcı yok | "Bu mağaza için <tür> yazıcısı tanımlı değil. Yöneticiye bildirin." |
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
`bekliyor_sahip_kapali` bir hata değil durumdur: iş kabul edilmiştir.

## Bütçe (ms)
Öneri, ölçülmedi [ÖLÇ]: `gonder` yanıtı (kuyruğa alındı) ≤ 200; aynı yerel ağda eşe ulaşma ≤ 1.000; eşe bağlanma zaman aşımı 3.000 (sonra `bekliyor_sahip_kapali`, arka planda yeniden dener).

## Kabul testleri
`yazdir_yerel_basar`, `yazdir_ese_yonlenir`, `yazdir_cift_basma_yok`, `yazdir_yeniden_gonderim_tek_baski`, `yazdir_cift_tik_tek_baski`, `yazdir_sahip_kapali_bekler`, `yazdir_sahip_acilinca_bir_kez`, `yazdir_cokus_belirsiz_durum`, `yazdir_iptal_kuyrukta`, `yazdir_iptal_basildiktan_sonra_red`, `yazdir_sahip_degisimi_aktarir`, `yazdir_yazici_yok_mesaji`, `yazdir_is_tekrar_farkli_icerik_red`, `yazdir_kayitsiz_es_red`, `yazdir_hiz_siniri`, `yazdir_kuyruk_dolu`, `yazdir_bicim_red`, `yazdir_is_deftere_yazilir`, `yazdir_suresi_doldu_basilmaz`, `yazdir_tek_pc_es_kapisi_yok`. Gerçek yazıcı denemesi: `docs/esleme-sozlesmesi.md` §6.2.

## Açık noktalar
- AÇIK-Y1: Mağazada tür başına birden çok yazıcı (ör. iki A4) gerekecek mi?
- AÇIK-Y2: Etiket yazıcısı modeli ve dili bilinmiyor; `etiket-ham` biçimi model bilinince kesinleşir.
- AÇIK-Y3: Esenyurt'ta hangi yazıcının hangi bilgisayarda olduğu bilinmiyor (`esleme-sozlesmesi` AÇIK-E3).

## Sürüm kuralı
Değişirse `@2` yeni dosya; eski `@1` en az bir sürüm daha yaşar (§18.3).
