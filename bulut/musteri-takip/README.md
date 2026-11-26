# Müşteri ürün takibi (Cloudflare + Vercel)

Müşteri, fişin MÜŞTERİ nüshasındaki karekodu okutup tadilatının durumunu görür (Yusuf, 2026-11-26).

## Nasıl çalışır
- Mağaza sunucusu (`taa-fis-sunucu`) internete **açılmaz**. Fiş kesilince ve durum değişince kaydı kendisi
  dışarı **iter** (dakikada bir, `curl`), iki hedefe birden: Cloudflare ve Vercel. Biri düşükse diğeri çalışır.
- Kayıt yalnız SABİT anahtarlar taşır: takip kodu, durum, şube kodu, teslim günü, zaman, silinme zamanı;
  v2+ fiş günü ve teslim saati, kalemler (ürün türü, katalog işlem kodu, varyant, renk anahtarı, adet 2..20, en çok 20 kalem),
  bağlı fiş sayısı; v3 kargo (durum, firma anahtarı, takip no, kargolanma günü, "İlçe/İl"). Ad, telefon, adres, ölçü,
  fiyat, serbest metin, fiş numarası gitmez (`musteri_takip.rs` testi denetler). Gövde sınırı 4096 bayt.
- Takip kodu fiş numarasından kuruluma özel gizli anahtarla türetilir (16 karakter, tahmin edilemez).
- Kayıtlar ed25519 ile imzalanır; bulutta yalnız GENEL anahtar durur. Özel anahtar mağaza bilgisayarında kalır.
- Teslimden 7 gün, iptalden 3 gün sonra kayıt silinir (Cloudflare KV kendi siler; Vercel günlük görevle).
- Karekod Cloudflare adresini açar; fişte altında "Açılmazsa: <Vercel adresi>" yazar.
- "Hazır" WhatsApp mesajına takip bağlantısı eklenir (`{takip}` yer tutucusu ya da sona kısa cümle).

## Mağaza sunucusu ayarı
Veri klasörüne `musteri-takip.json`:
```json
{"acik": true,
 "sayfa": "https://terzitakip.terziademaltun.workers.dev",
 "yedek_sayfa": "https://terzitakip-terziademaltun.vercel.app",
 "hedefler": ["https://terzitakip.terziademaltun.workers.dev/api/yaz", "https://terzitakip-terziademaltun.vercel.app/api/yaz"]}
```
Genel anahtar: `taa-fis-sunucu --veri <klasör> --takip-genel-anahtar` (ilk çalıştırmada anahtarlar oluşur).

## Sayfa
- Sekmeler: `/t/<kod>?sekme=ozet|islemler|subeler|sss&dil=xx` (bilinmeyen sekme → özet). Betik yok; dil çekmecesi
  ve SSS akordeonu `<details>` ile çalışır. Kayıt yok/iptal sayfasında yalnız SSS açılır.
- **Şube bilgileri** `ortak/ek.mjs` → `SUBELER`: her şube için `ad`, `adres`, `tel` (`+90...`), `tel_g` (görünen biçim),
  `saat`, `wa` (şubenin WhatsApp numarası, `905...`). Adresi olmayan şubede "açık adres için merkezi arayın" notu çıkar;
  Yol tarifi ve Ara düğmeleri adres/telefon girilince kendiliğinden görünür. (Şu an yalnız Merkez dolu.)
- **WhatsApp**: `ortak/ek.mjs` → `WHATSAPP_MERKEZ = "905..."`. Şubenin `wa`'sı öncelikli. İkisi de boşken WhatsApp
  düğmesi çıkmaz ve SSS'deki WhatsApp cümlesi WhatsApp'sız haliyle gösterilir. (Numara henüz girilmedi.)
- Değiştirdikten sonra `./kopyala.sh` çalıştırın.

## Fişsiz teslim onayı (karekodla imza)
Yusuf 2026-11-26 05:33: fişsiz teslimde müşteri imzası fiziksel değil karekodla; teslim alanın adı, telefonu, telefonun
bilgisi, tarih/saat şifreli tutulur. Ortak kod `ortak/onay.mjs` (kopyala.sh iki hedefe kopyalar).
- Sunucu `POST /api/onay/ac` (imzalı; `t` 20 karakter, `sil` ≤ ts+900, `pk` sunucunun X25519 genel anahtarı) → karekod `<birincil>/o/<t>`.
- Müşteri `/o/<t>` sayfasında (betiksiz, 13 dil) ad soyad, telefon ve "Ürünlerimi eksiksiz teslim aldım" kutusunu gönderir.
  Bulut düz metni (ad, telefon, user-agent ilk 200, IP, zaman, dil) hemen şifreler: geçici X25519 + ECDH(pk) →
  HKDF-SHA256(salt = geçici_genel‖pk, info "taa-teslim-onay-v1") → AES-256-GCM (aad = t). Kayıtta yalnız `{e, iv, c}` kalır;
  düz metin yazılmaz/loglanmaz. Bir `t` bir kez onaylanır; aynı `t`'ye en çok 5 POST.
- Sunucu `GET /api/onay/<t>` (`x-taa-zaman`, `x-taa-imza` = imza("onay_oku:"+t+":"+ts), ±300 sn) ile okur, kendisi çözer,
  `POST /api/onay/kapat` (imzalı) ile siler. Cloudflare'de KV kendi siler; Vercel'de süresi geçen kayıt okurken, her
  aç/kapat'ta ve günlük görevde silinir. Ayrıca mağaza sunucusu süresi geçen (onaylanmamış) her karekod için dakikada
  bir iki buluta da `kapat` gönderir: Vercel'de 15 dk tavanı sunucu açık ve internete ulaşıyorsa ~16 dk'dır; sunucu
  kapalıysa şifreli kayıt bir sonraki aç/kapat'a ya da 03:17 görevine kadar (en kötü ~24 saat) kalır.
- Ad kuralı: bulut `[\p{L}\p{M} .'-]` + ZWNJ/ZWJ (Hintçe virama, Farsça ZWNJ, harekeli Arapça geçer); sunucunun
  kuralı (`taa_fis::ad_karakterleri_gecerli`) bunun üst kümesidir. İki taraf aynı sahte adlarla sınanır (dene6 + Rust;
  Rust testindeki vektörler bulutun gerçek `sifrele()` çıktısıdır).
- Sayfa çerçevelenemez (`frame-ancestors 'none'`, `X-Frame-Options: DENY`).

### Parmakla imza (Yusuf 2026-11-26 06:23: "fişi yanında olmayan için")
- Aç gövdesine isteğe bağlı `"i": 0|1` eklendi (yoksa 0 = eski istemci; başka değer 400). `i = 1` → `/o/<t>` sayfasında
  parmakla imza ZORUNLU (sunucuda yöntem `qr_imza`); `i = 0` → sayfa eskisi gibi betiksiz, imza alanı yok (gelirse
  doğrulanıp yüke girer, bozuksa red). Fişsiz teslimde sunucu `i = 1` gönderir.
- Betik satır içi değildir: `GET /o/imza.js` (Cloudflare'de worker, Vercel'de `vercel.json` yeniden yazımı `/o/:t`'den
  önce). CSP yalnız imzalı form sayfasında `script-src 'self'` ile açılır; diğer bütün sayfalar betiksiz kalır.
  Kaynak `ortak/imza.mjs` (`imzaIstemci`, `griPng`; toString ile metne çevrilir, test/dene7 sunulan metni sahte DOM'da çalıştırır).
- Tuval 600×240, pointer olayları (dokunmatik, fare, kalem basıncıyla kalınlık), Temizle düğmesi, "en az birkaç çizgi"
  denetimi (toplam ≥ 150 px ve ≥ 2 vuruş ya da ≥ 400 px). Çıktı 8 bit gri tonlamalı PNG (CompressionStream); sınırı
  aşarsa 4 gri düzeye iner; CompressionStream yoksa `toDataURL` (RGBA) yedeği, gerekirse 300×120.
- Bulut doğrular: `data:image/png;base64,` öneki, çözülmüş ≤ 61 440 bayt, PNG imzası, IHDR en ≤ 800, boy ≤ 400, IEND.
  Ardından **içerik** (`imzaIcerik`): PNG gerçekten çözülür (parça CRC'leri, IDAT açılır ve tam boy, satır süzgeçleri) ve
  koyu piksel kuralı uygulanır: koyu (beyaz zemine göre parlaklık < 128) ≥ 60 ve ≥ binde 1,5; ≤ %60; koyu sütun aralığı
  ≥ en/10. Boş beyaz tuval, tek nokta, kara tuval ve yalnız başlığı doğru sahte PNG reddedilir (400, imza hatası). Mağaza
  sunucusu aynı kuralı ayrıca uygular (`crates/taa-fis-sunucu/src/imza_png.rs`; hem `qr_imza` hem `cihaz_imza`; boş ya da
  çözülemeyen imza 422 `imza_gerekli`). Bu kural "gerçek imza" kanıtı değildir; yalnız boş/sahte gönderimi keser.
  Gövde sınırı 131 072 bayt (önceden 2 048); gövde akıştan okunur ve sınır aşılınca kesilir (content-length olmasa da).
  JavaScript kapalıysa `<noscript>` uyarısı görünür ve imzasız gönderim reddedilir.
- `/o/imza.js` işlevlerin `toString` çıktısından üretilir. Paketleyici hedefi düşürürse (async yardımcıları) betik
  sunulmaz (503). **Dağıtımdan sonra** canlı `/o/imza.js` yanıtı `node --check` ile sınanmalı ve iOS Safari + Android
  Chrome'da elle bir imza denenmeli.
- **Fişli teslimde imza isteğe bağlıdır.** 2026-11-26'dan beri "fişli" işaretini kasiyer KOYAMAZ: fiş ancak karekodu
  okutulursa fişli sayılır (bkz. aşağıda "Teslim sepeti"). Eski not (kasiyerin elle "fişli" işaretleyip imzasız teslim
  edebilmesi) bu kararla kapandı.
- Şifreli yükte düz metin P = `{ad, tel, ua, ip, zaman, dil, imza}`; `imza` öneksiz base64 PNG'dir (≤ 81 920 karakter).
  En büyük imzayla `GET /api/onay/<t>` yanıtı ~110 KB'dir (sunucu en az 128 KiB okumalı).
- Bilinen sınır: tuval klavyeyle kullanılamaz; imza atamayan müşteri için mağaza cihazında imza (`cihaz_imza`) yolu vardır.
- Gerçek telefonlarda (iOS Safari, Android Chrome) dokunmatik çizim ve gönderim elle denenmedi; yalnız sahte DOM testi var.

### Teslim sepeti: tek karekodla zincir, çok fiş tek teslimat (Yusuf 2026-11-26 06:25)
"teslim edilirken de teslim edilen ürünler zincirlenebilsin karekod zinciri ile yani tek karekod ile de teslim
edilebilsin, birden fazla karekod da tek teslimatta yapılabilsin." Mağaza sunucusu: `crates/taa-fis-sunucu/src/teslim_sepet.rs`,
arayüz: `ortak.js` `TKD.teslimSor` (masaüstü ve telefon teslim penceresi artık bu sepettir).
- **Tek karekodla zincir:** bağlı fişlerden birinin karekodu okutulunca (ya da no yazılınca) bütün bağlı fişler sepete
  gelir; varsayılan hepsi seçili, tek tek çıkarılabilir.
- **Çok karekodlu sepet:** "Karekod okut / no yaz" alanına tabanca/klavyeyle (telefonda kamera: Android uygulamasında yerel
  okuyucu, yoksa BarcodeDetector) sırayla başka fişler eklenir; bağlı olmaları gerekmez. Liste: fiş no, müşteri adı (fiş
  listesindeki düzeyde), ürün sayısı, durum, fişli/fişsiz. Çift okutma yok sayılır; teslim edilmiş ya da iptal fiş eklenmez;
  hazır olmayan fiş uyarıyla görünür ama **teslim edilmez** (istisna yok; teslim isteğinde varsa 409 `hazir_degil` ve
  hiçbir fiş teslim edilmez).
- **Tek teslimat:** sepetin tamamı tek teslim alan bilgisi, tek karekod onayı (`n` = sepetteki toplam ürün; sepette fişsiz
  fiş varsa `i = 1`; belirteç sepet kimliğine — sıralı fiş no'larının özetine — ve cihaza bağlı, sepet değişirse onay
  geçmez) ya da tek cihaz imzasıyla tamamlanır. Her fişe ayrı `TeslimKaydi` yazılır; hepsinde ortak **`teslim_id`**
  (`T-YYYYAAGG-XXXXXX`, rastgele), `ana_fis` = ilk okutulan fiş. **İmza bir kez** şifreli yazılır
  (`imzalar/<ana>-<n>.png.enc`), bütün kayıtlar aynı dosyaya ve özete başvurur. Kişisel bilgi bloğu her fiş için ayrı
  şifrelenir (aynı düz metin, aad = fiş no: bir fişin bloğu başka fişe taşınamaz).
- **Ya hepsi ya hiçbiri:** yetki, hazır durumu, karekod onayı, son 4, imza ve şifreleme YAZMADAN önce denetlenir; kayıtlar
  `Defter::teslim_et_toplu` ile tek yazımda (`taa_dosya::satirlar_ekle`) eklenir, araya başka satır girmez. Yazım yine de
  hata verirse hiçbir fiş teslim sayılmaz, yeni imza dosyası `imzalar/artik/` altına alınır (silinmez), 500
  `teslim_yazilamadi` döner ve `teslim-sepet.jsonl` günlüğüne kişisel veri olmadan yazılır. Neden "ters kayıt" değil:
  defter yalnız eklemelidir ve teslim satırı aynı zamanda Hazır → Teslim geçişidir; yarım kalmış teslimi geri alan bir
  satır türü yeni bir "teslimden geri dönüş" yolu açardı. Tek yazım bu riski en aza indirir; işletim sistemi tek yazımın
  elektrik kesintisinde bölünmeyeceğini garanti etmez (bu durumda son satır yarım kalabilir; defter okuma hatası verir ve
  elle onarım gerekir — tek satırlık teslimde de aynı sınır vardır). Yarım satırdan sonra eklenen satır ona
  yapışmaz: `satir_ekle`/`satirlar_ekle` dosya satır sonuyla bitmiyorsa önce satır sonu yazar (onarımda yalnız yarım
  satır çıkarılır). `kismi_hata_hicbiri_yazilmaz` testi yazımdan önceki hatayı sınar; gerçek yarım yazım sınanmadı.
- **Müşteri takip (bulut):** her fiş ayrı ayrı "teslim" durumuna geçer (mevcut kuyruk). Teslim alan bilgisi ve
  `teslim_id` buluta gitmez.

#### Fişli / fişsiz kararı (Yusuf gözetimsiz; denetçinin önerisi)
- **Fiş karekodu okutularak** sepete giren fiş **fişli**dir. Kanıt sunucuda yeniden doğrulanır (istemci okunan metni
  gönderir): müşteri nüshasındaki **takip karekodu** (`<bulut>/t/<kod>`; kod gizli anahtarla üretilir, tahmin edilemez)
  her zaman kanıttır. Personel karekodu artık **gizli anahtarlı bir kanıt parçası** taşır:
  `…/telefon#fis=<no>&k=<HMAC(yerel anahtar, "fis:"+no) ilk 12 karakter>` (anahtar `musteri-takip/anahtar`, takip/bulut
  ayarı olmasa da yerelde üretilir; 60 bit, fiş no'dan tahmin edilemez). Bu karekod yalnız müşteri takip **ayarlı
  değilken** ve `k` doğruysa kanıttır (o zaman müşteri nüshasına bu karekod basılır); takip ayarlıyken o karekod yalnız
  terzihane nüshasındadır, mağazada durur ve kanıt sayılmaz (ekranda "terzihane nüshasının karekodu: fişsiz" yazar).
  `k`'siz ya da yanlış `k`'li metin (2026-11-26 öncesi basılmış fişler, elle yazılmış "#fis=<no>") **fişsizdir**
  (`kanit: "karekod-kanitsiz"`). (Denetçi bulgusu: önceki sürümde takip ayarsız kurulumda "#fis=<no>" elle yazılarak
  fişsiz kuralları tamamen atlanabiliyordu.)
- **Yapısal sınırlar (açıkça):** (1) Tabanca klavye gibi yazar; sunucu tabancayla okutmayı klavyeyle yazmaktan ayırt
  edemez. Kanıt "karekodun içeriği elde" demektir, "şimdi okutuldu" değil. (2) Mağaza cihazı fişin müşteri nüshasını
  **yeniden yazdırabilir** (`POST /api/fis/<no>/yazdir`) ve fiş karekodu ekranda (`/api/fis/<no>/karekod.svg`) görünür;
  personel bu yolla kanıtlı karekodu elde edip fişsiz kuralları atlayabilir. Bu, mağaza içi güven sınırıdır; yeniden
  yazdırma ve teslimler günlükte (`kim`) görünür. **Sıkılaştırma (2026-11-26):** yeniden basım artık
  `yeniden-yazdir` cihaz yetkisine bağlıdır (varsayılan açık; yönetici kapatabilir), bkz. aşağıda
  "[Yeniden basım yetkisi](#yeniden-basım-yetkisi-yeniden-yazdir)".
- **Elle no yazılarak** ya da **bağlı zincirden otomatik gelen** ama karekodu okutulmayan fiş **fişsiz**dir. Sonradan
  karekodu okutulursa fişliye yükselir.
- Sepette **en az bir fişsiz** fiş varsa: cihazın `fissiz-teslim` yetkisi (yoksa 403, hiçbiri teslim edilmez), **her
  fişsiz müşteri için telefonun son 4 hanesi** (aynı telefonlu fişlerde bir kez; yanlışta gruptaki her fişin sayacı artar,
  5 yanlışta 15 dk kilit) ve **imza** zorunlu (karekod sayfasında `qr_imza` ya da cihaz ekranında `cihaz_imza`).
- **Kasiyerin "fişli" işaretini elle koyma yolu kaldırıldı.** Arayüzde "Fişiyle geldi / Fişi yok" seçeneği yoktur.
  Geri uyum: eski `POST /api/fis/<no>/teslim-al` ucu durur ve `fisli` alanı yok sayılmaz (`fisli: false` yine fişsizdir),
  ama `fisli: true` yalnız `fis_karekodu` (okunan karekod metni) bu fişin kanıtıysa geçer; kanıt yoksa istek fişsiz
  kurallarıyla değerlendirilir ve hata yanıtında `"fis_karekodu_yok": true` bulunur. Sepette `okutuldu: true` denilip
  kanıt verilmeyen fiş de fişsiz sayılır (yanıtta `kanitsiz`).
- **Eski uçta bağlı fişler:** `POST /api/fis/<no>/teslim-al` gövdesinde `"bagli": [..]` doluysa istek iç olarak teslim
  sepetine çevrilir (ana fiş `fisli`/`fis_karekodu` ile, bağlı fişler `okutuldu: false`). Bağlı fişler **fişsizdir**:
  yetki + her fişsiz müşteri için son 4 + imza; yazım ya hepsi ya hiçbiri; ortak `teslim_id`; yanıt eski alanları
  (`birlikte_teslim`, `birlikte_atlanan: []`) taşır. Eski tek `telefon_son4` metni sepette tek telefon grubu varsa
  geçer; farklı telefonlu bağlı fişte `{<no>: "1234"}` nesnesi gerekir (422 `son4_gerekli` hangi fişler için olduğunu
  söyler). Tek fişe açılmış karekod belirteci bağlı fişlerle kullanılamaz (409 `sepet_gerekli`; sepet karekodu açın).
  (Denetçi bulgusu: önceki sürümde bağlı fişe ana fişin kaydı `fisli=true`, imzasız kopyalanıyordu; yetkisiz telefon
  farklı telefonlu bağlı fişi son 4 ve imza olmadan teslim edebiliyordu.)

#### Uçlar (yetkili cihaz: mağaza PC ya da eşleşmiş telefon)
- `POST /api/teslim/karekod-coz {metin}` → `{no, kanit: "takip"|"fis-karekodu"|"elle", okutuldu, not}` (404 bulunamadı).
  Masaüstü tabancası ve telefon kamerası takip karekodunu bununla fişe çevirir.
- `POST /api/teslim/sepet/denetle {fisler: [{no, okutuldu, karekod?}], ekle?: "<okunan/yazılan>"}` → `{fisler: [{no,
  musteri, urun_sayisi, durum, hazir, okutuldu, kanit, karekod?, uyari?}], toplam_urun, fissiz, fissiz_var, fissiz_yetki,
  imza_zorunlu, son4_gruplari: [{anahtar, fisler, musteri}], sepet_kimligi, eklenen, zaten_sepette, yok_sayilan,
  reddedilen: [{no, neden}], kanitsiz, uyarilar, en_cok, fazla, dolu}`. Durumsuzdur: istemci listeyi her istekte geri
  gönderir; arayüz istekleri sıraya koyar (art arda okutmada bir yanıt öbür okutmayı silmesin).
- **Sepet sınırı:** bir teslimatta en çok **30 fiş**. Fazlası sessizce kırpılmaz: denetlede `fazla` ve `reddedilen`
  ("sepet sınırı aşıldı") olarak bildirilir, dolu sepete okutma `ekle_hatasi` verir; teslim ve karekod isteğinde sepet
  **tümden** reddedilir (413 `sepet_cok_buyuk`, hiçbir fiş teslim edilmez).
- **Karekod onayında ürün sınırı:** bulut `n` için 1..50 kabul eder. Toplam 50'yi aşarsa sayı sıkıştırılmaz (müşteri
  "50 ürünü teslim aldım" derken fazlası teslim edilmesin): 409 `qr_cok_urun`, arayüz ekran imzasına geçer.
- `POST /api/teslim/sepet/onay-ac {fisler}` → karekod (`t, url, qr_svg, sil, kalan_sn, imza_gerekli, sepet_kimligi,
  toplam_urun`); `POST /api/teslim/sepet/onay-durum {fisler, t}` → `teslim-qr` durumuyla aynı; `fis_telefonu` sepet için
  `ayni | farkli | kismen | yok` (farkli/kismen'de ilişki yakını/diğer zorunlu).
- `POST /api/teslim/sepet {fisler, teslim_alan?, iliski?, onay_belirteci?, imza_png_base64?, telefon_son4?: {<no>: "1234"}}`
  (tek telefon grubu varsa `telefon_son4` düz metin de olur) → 200 `{tamam, teslim_id, ana_fis, fisler: [{no, musteri,
  urun_sayisi, fisli, dogrulama, ana_fis, teslim_id, imzali, tarih, saat}], yok_sayilan, kanitsiz, birlikte_teslim,
  teslim_kaydi}`. Hatalar: 400/413 biçim, 403 `yetki_yok` (+`fissiz`), 409 `hazir_degil` / `sepet_gecersiz`, 409
  `onay_bekliyor`, 413 `sepet_cok_buyuk`, 422 `onay_yok` / `iliski_gerekli` / `imza_gerekli` / `son4_gerekli` / `son4_yanlis` / `telefon_yok`,
  429 `son4_kilitli`, 500 `teslim_yazilamadi`. Gövde sınırı teslim-al ile aynı (512 KB, imza base64).

#### Eski fiş geçiş süresi (koordinatör kararı 2026-11-26, geri alınabilir)
- Yönetici ayarı **"eski fiş geçiş süresi"** (gün; **varsayılan 0 = kapalı**, en çok 90). Masaüstünde **Ayarlar > Yönetim**
  (yalnız yönetici oturumuyla görünür ve değişir); uç `GET/POST /api/yonetim/ayar {eski_fis_gecis_gun}` (yalnız bu
  bilgisayar + yönetici oturumu; 422 sınır dışı). Süre kaydedildiği anda başlar, bitince **kendiliğinden kapanır** (ayar
  dosyası değişmez, karar zamana bakar; yeniden kaydetmek süreyi yeniler, ama geçiş `fisli_kanit_baslangic`'tan **en
  çok 180 gün** sonra hiçbir şekilde açık kalmaz — geri alınabilir varsayılan, koordinatöre bildirildi). Ayar
  `yonetim-ayar.json` dosyasındadır.
- Açıkken, **kanıt mekanizmasından önce basılmış** fişlerde kanıt parçasız personel karekodu (`karekod-kanitsiz`) ya da
  elle yazılan fiş no (`elle`) **fişli** sayılır. "Önce basılmış": fiş tarihi < `fisli_kanit_baslangic` (programın bu
  sürümle ilk açıldığı gün; teslim sepeti günlüğü — kanıt mekanizmasıyla, 5b1f004'te gelen — daha eskiyse o gün; ilk
  açılışta `yonetim-ayar.json`'a bir kez yazılır, değişmez, yönetim günlüğüne `fisli_kanit_baslangic` olarak da yazılır;
  dosya silinirse günlükten geri alınır, dosya bozuksa yeniden yazılmaz ve geçiş kapalı kalır) ya da aynı gün ve fiş no ≤
  `fisli_kanit_son_eski_fis` (ilk açılıştaki son fiş; sıra sayısal karşılaştırılır). Yeni fişler, terzihane nüshasının kanıtlı karekodu (takip
  ayarlıyken) ve bağlı zincirden kendiliğinden gelen (okutulmamış) fişler geçişle fişli **olmaz**.
- Yanıtlarda satır `gecis: true` taşır, `uyarilar`/`kanit_notu` "eski fiş geçiş süresi" der; `karekod-coz` yanıtında
  `gecis`. Her kullanım `teslim-sepet.jsonl` günlüğüne `eski_fis_gecisi` olayıyla (fiş no'ları, kim, teslim_id, aşama
  `teslim`|`teslim-al`, zaman; kişisel veri yok) yazılır: satır teslim kaydından hemen önce, bütün denetimler geçtikten
  sonra düşülür; **yazılamazsa teslim yapılmaz** (500 `gunluk_yazilamadi`); kayıt sonra başarısız olursa aynı teslim_id
  ile hata satırı. Ayar değişikliği `eslestirme.jsonl`'a (`olay: "ayar"`, eski/yeni gün, kim) yazılır.

#### Yeniden basım yetkisi (`yeniden-yazdir`)
- Yeni cihaz yetkisi `yeniden-yazdir` (`cihaz::YETKILER`; **varsayılan her yerde açık**, eski davranış). Yönetici
  Telefonlar ekranındaki yetki anahtarlarından cihaz bazında (mağaza bilgisayarı dahil) kapatır.
- "Yeniden": fişin müşteri nüshası daha önce verildiyse (yazıcıya basıldı; Windows yazdırma penceresi için karekod
  `GET /api/fis/<no>/karekod?basim=1` ile alındı; yetkisi kapalı cihaz karekodu ekranda gördü). Fişli kanıttan önce
  basılmış fişler (`fisli_kanit_baslangic`'tan eski) basım kaydı olmasa da **verilmiş** sayılır. İlk basım serbesttir.
- Yetkisi kapalı cihazda `GET /api/fis/<no>/karekod` (`basim=1` olmasa da) **her gösterim müşteri nüshasının verilmesi
  sayılır**: ilki günlüğe `basim` (yöntem `karekod`) yazılarak verilir, sonrakiler 403. İstisna: cihaz fişi bu süreçte
  kendisi oluşturduysa 10 dakika boyunca önizleme günlüksüz (yeni fiş ekranı). Yetkisi kapalı cihazın ilk yazıcı basımı
  önce günlüğe yazılır (yazılamazsa basılmaz; yazıcı hatasında `basim_iptal`). Yetkili cihazda basım yapılıp günlük
  yazılamazsa fiş yeniden başlatılana dek bellekte "verildi" sayılır.
- Yetki kapalıyken müşteri nüshalı basım (`POST /api/fis/<no>/yazdir` gövdesiz) ve müşteri karekodunun yeniden gösterimi
  (`GET /api/fis/<no>/karekod`) **403 `yetki_yok`** (`terzihane_yazdirilabilir: true`). Terzihane nüshası
  (`{"nusha":1}`; karekodunda yalnız fiş no var) etkilenmez; arayüz reddedilince "yalnız terzihane nüshası" önerir
  (Windows yazdırma penceresinde de: `GET /api/fis/<no>/karekod?nusha=1` yalnız fiş no'lu karekod verir, günlüğe
  `terzihane`).
- Takip ayarlıyken takip bağlantısı da kanıttır: `GET /api/bildirimler` hazır bildirimindeki takip bağlantısını ve kodu
  yetkisi kapalı cihaza **maskeli** verir. Müşteriye giden bildirim (WhatsApp) bağlantıyı taşır (müşteri tarafı).
- Günlük `basim.jsonl` (kişisel veri yok): `basim` / `yeniden_basim` (müşteri nüshası, yöntem `yazici`|`tarayici`),
  terzihane basımı, `karekod_gosterim` (yeniden gösterim), `basim_iptal`, `yeniden_basim_reddedildi`; her satırda fiş no, cihaz, kim,
  tarih, saat. Yeniden verilen karekod günlüğe yazılamazsa verilmez (500).

### Yönetici girişi ve günlükte "kim" (koordinatör kararı 2026-11-26)
- Yönetici girişi **TAA ortak giriş** kullanıcısıyla yapılır: `POST /api/yonetici/giris {ortak:true, parola}` bu
  bilgisayardaki taa-kimlik oturumunu sorar; kullanıcı TAA Çatı'da yönetici rolündeyse (`yonetim-ayar.json`
  `yonetici_rolleri`, varsayılan `["yonetici"]`; rol adı TAA Çatı tarafıyla henüz teyit edilmedi, yanlışsa ortak girişle
  yönetici açılmaz, PIN'e düşer) **o kişinin şifresi** ortak girişin `giris` akışıyla (aynı kullanıcı için taa-kimlik)
  yeniden doğrulanır ve oturum o kişiyle açılır (yanıtta `kim`, `kaynak: "ortak-giris"`). Oturum yoksa 409
  `ortak_oturum_yok`, rol yoksa 403 `yonetici_degil`, şifre yoksa 400 `parola_gerekli`, yanlışsa 401 `parola_hatali`
  (günlükte `yonetici_giris_red`, şifre yazılmaz). Ortak girişte biri oturuyor diye bilgisayar başındaki herkes yönetici
  olamaz.
- **Oturum kişiye bağlıdır:** ortak girişte bu kişi çıkınca ya da başka biri girince ortak girişle açılmış yönetici
  oturumu geçersizdir (sunucu her istekte denetler: yönetim 401, günlükte yeni kişi; masaüstü `taa-giris-degisti`
  olayında kişi değiştiyse kilitler). Bellekteki ortak kullanıcı `/api/giris/*` yanıtlarından (main.rs'de ayrı iş
  parçacığında da) ve 45 sn'de bir taa-kimlik'ten yeniden sorularak güncellenir; kişi başka bir TAA programından
  çıktıysa en çok bu süre kadar eski kişi yazılabilir.
- **PIN yolu geri uyum için durur** (`{pin}`): ortak giriş kapalı/erişilemezken girilir; günlükte yalnız `"PC (PIN)"`
  (PIN kişiyi belirlemez; ortak girişteki kişinin adı eklenmez). Yönetim satırlarında ayrı `kim_kaynak` alanı
  (`ortak-giris` | `pin` | `ortak-oturum` | `yok`). Ortak kullanıcı adı `PC`, `PC (…)` ya da kişi/cihaz kimliğine
  benziyorsa (`K-`, `U-`, `T-`) günlükte `ortak:` önekiyle yazılır.
- Günlüklerde mağaza bilgisayarı için `"PC"` yerine oturumdaki kişi yazılır: teslim denetim günlüğü
  (`teslim-ayrinti.jsonl`: `teslim_ayrintisi`, `teslim_acik`, `imza_goruntuleme`), `teslim-sepet.jsonl`, `basim.jsonl`,
  yönetim günlüğü (`eslestirme.jsonl`: `yetki`, `kisi_yetki`, `ayar`, yeni `yonetici_giris`), fiş düzeltme kayıtları ve
  teslim kaydındaki `kim`, tutanak/Telegram hesap işlemlerindeki yönetici adı (istemcinin beyanı yalnız yedek). Kişi:
  geçerli yönetici oturumunun kişisi, yoksa ortak girişle giren kullanıcı (sunucu belleğinde; `/api/giris/*`
  yanıtlarından güncellenir, çıkışta silinir); ikisi de yoksa eskisi gibi `"PC"` / `"Mağaza PC"`.
- `GET /api/yonetici/durum` artık `kim`, `kaynak` ve `ortak: {kullanici, yonetici}` döndürür (PIN ekranındaki ortak giriş şifre kutusu
  buna bakar); `GET /api/oturum` mağaza bilgisayarında `oturum_kisi` döndürür (hesap kartı menüsündeki
  ad).

### Şifre anahtarı
- Teslim şifre anahtarının yeri ve yedeği şimdilik değişmez: **sunucuya kuruluma geçerken sorulacak** (Yusuf 2026-11-26).
  Aynı anda sorulacak: şifreli defter yedeklerinin (`yedek/fisler-*.jsonl.enc`) saklama süresi (şu an süresiz).
- Not: anahtar varsayılan olarak veri klasöründedir (`--teslim-anahtar` ile dışarı alınabilir). Anahtar veri klasöründe
  dururken şifreleme yalnız **kısmi kopyaya** (yalnız defter, yalnız imzalar, yalnız yedek) karşı korur; veri klasörünü
  bütün olarak kopyalayan biri şifreli yedeği ve imzaları da çözebilir.

## Teslim bilgisi: maskeleme, `teslim-acik` yetkisi, eski kayıt geçişi (Yusuf 2026-11-26)
### Maskeli görünüm ve açık görme yetkisi
- Teslimden sonra bütün ekranlarda ve basılı mağaza nüshasında teslim bilgisi **maskelidir**: ad "M*** Y***", telefon son
  4 hane dışında gizli, IP hiç gösterilmez, okutan cihazın yalnız özeti ("Android 14 · Chrome"); imza görüntüsü yok.
- Açık hâl (`GET /api/fis/<no>/teslim-acik`: tam ad, telefon, IP, ham user-agent; imza `GET /api/fis/<no>/imza`) yalnız
  `teslim-acik` yetkisiyle verilir. Karar sırası: (1) mağaza bilgisayarında yönetici (PIN) oturumu; (2) cihaza
  yöneticinin yazdığı kayıt (ver/geri al); (3) kişiye yöneticinin yazdığı kayıt — yalnız kişi hâlâ açık listedeyse
  (kişi kapatılınca kayıt da geri alınır); (4) kayıt yoksa rol varsayılanı: "yonetici" ya da "magaza-muduru" — **yalnız
  rolü yöneticinin bu bilgisayarda verdiği kişide** (Personel ekranı / ilk kurulum). TAA Çatı'dan (unvan metninden
  tahmin) ya da Telegram'dan gelen rol kendiliğinden yetki vermez; "müdür yardımcısı" gibi unvanlar personel sayılır.
  Yönetici bu yetkiyi Telefonlar ekranından cihaza ya da kişiye verip geri alır.
- Her açık görüntüleme (ve imza görüntüleme) `teslim-ayrinti.jsonl` denetim günlüğüne **önce** yazılır: olay, fiş no,
  kim (cihaz / kişi kimliği / "PC"), hangi yetkiyle, tarih-saat. Günlükte teslim alanın kişisel verisi yoktur. Günlük
  yazılamazsa (disk dolu, dosya kilitli) açık veri **verilmez** (500 `gunluk_yazilamadi`).
- Mağaza bilgisayarında "kim" yalnız "PC" yazar: PIN ortak olduğundan hangi yöneticinin baktığı ayırt edilmez (Yusuf'a
  soruldu: yönetici girişine kişi seçimi eklensin mi).
- Ekrandaki açık görünüm 60 sn sonra, "Maskele" ile, fiş değişince ya da yönetici oturumu kapanınca yeniden maskelenir.

### Eski kayıt geçişi
- Sunucu **açılışında, istek almadan önce** bir kez çalışır (idempotent; `teslim_gecis.rs`). Defterde (`fisler.jsonl`,
  yalnız eklemeli) düz metin `teslim_alan` taşıyan satırlar şifreli biçime çevrilir. Bunun için defter bir kez yeniden
  yazılır: önce defterin tamamı `yedek/fisler-<YYYYMMDD-HHMMSS>.jsonl.enc` olarak **yerel anahtarla tek blok AES-256-GCM
  şifreli** yedeklenir ve geri okunup doğrulanır; sonra defter atomik yazılır (okuma sonrası defter değiştiyse yazılmaz,
  sonraki açılışta yeniden dener). Yedekte eski düz metin vardır ama dosya şifrelidir.
- Sunucu çalışırken defter yeniden yazılmaz (eklenen satır kaybolabilirdi): `POST /api/yonetim/teslim-gecis` yalnız
  imzaları şifreler ve gerekiyorsa "yeniden başlatın" der; `GET` durumu verir. Masaüstü Telefonlar ekranında uyarı kartı
  bekleyen defter satırı ve şifresiz imza sayısını gösterir.
- İmzalar: `imzalar/<no>-<n>.png` için şifreli kopya `imzalar/<no>-<n>.png.enc` yazılır ve doğrulanır. **Şifresiz PNG
  program tarafından silinmez** (kalıcı silme yok): `imzalar/silinecek-duz-imzalar.json` listesine yazılır; yönetici
  listeye bakıp dosyaları kendisi Geri Dönüşüm Kutusu'na siler.
- Teslim kaydı yazılamazsa yeni yazılmış (hiçbir kayda bağlanmamış) şifreli imza dosyası silinmez, `imzalar/artik/`
  altına taşınır.
- **Kalan düz metin (bilerek):** (1) yönetici silene kadar şifresiz imza PNG'leri (liste dosyasındakiler); (2) veri
  klasörü dışındaki eski kopyalar/yedekler (program onlara erişmez); (3) atomik yeniden yazım eski dosyanın disk
  bloklarını güvenle silmez (dosya sistemi düzeyi; SSD'de de garanti yok).
- **Zincir uyarısı:** geçiş defter zinciri (TT1) başlatılmadan **önce** tamamlanmalıdır. Zincir başlatılmışsa yeniden
  yazım merkezdeki zincir ucunu bozar; bu yüzden zincir varken defter yeniden yazılmaz, yönetici açıkça onaylarsa
  program `--teslim-gecis-zincir-onay` ile açılır (merkezdeki uç yenilenmeli). Bugün zincir üretim kodunda başlatılmıyor.

### Bilinen sınırlar (Yusuf'a söylenmeli)
- **Platform erişim günlükleri:** Kod hiçbir şeyi loglamaz, ama Cloudflare ve Vercel'in kendi erişim günlükleri
  `/o/<t>` yolunu, IP'yi ve user-agent'ı kod dışında tutar (ad ve telefon gövdede olduğu için oraya düşmez). "IP bulutta
  loglanmaz" sözü bu günlükler için verilemez. Cloudflare'de `wrangler.toml`'da `observability` kapalıdır, Logpush
  açılmamalı; Vercel'de log drain açılmamalı.
- **Cloudflare KV sonunda tutarlıdır:** bir uç noktada okunan değer ~60 sn önbellekte kalabilir. Müşterinin telefonu
  başka bir uç noktaya düşerse kasadaki "bekliyor" durumu onaydan sonra 1 dakikaya kadar sürebilir (kasa ekranında bu
  not yazar). Aynı nedenle "bir kez onay" ve 5 POST sınırı kesin değildir (dar pencerede ikinci POST ilkinin üzerine
  yazabilir; sunucu ilk okuduğunu alıp kapatır). Gerçek dağıtımda mağaza ağı + mobil şebekeyle gecikme ölçülmeli;
  kabul edilemezse onay kaydı Durable Object'e (ya da D1'e) taşınmalı (atomik tek seferlik onay da böyle çözülür).
- **Vercel yolu gerçek dağıtımda denenmedi:** `vercel/lib/onay-depo.mjs` sahte Blob ile test edilir (dene6), ama Blob
  CDN önbelleği (`cacheControlMaxAge: 60` + `?v=` ile aşma), yeniden yazımda `istek.url`'in biçimi (işlem hem asıl
  yoldan hem sorgudan bulunur) ve `head/put/del` davranışı bir Vercel önizleme dağıtımında elle denenmeli
  (aç → form → onay → oku → kapat). Blob public'tir; adres t bilinmeden tahmin edilemez (bekleyen kayıtta pk, şube kodu
  ve ürün sayısı açık, kişisel veri yok).
- **Telefon doğrulanmaz:** karekodda yazılan ad ve telefon müşterinin beyanıdır (SMS kodu yok); "kimin telefonundan"
  bilgisi user-agent + IP'dir (user-agent istemcinin gönderdiği değerdir). Sunucu karekoddaki telefonu fişteki numarayla
  karşılaştırır (numarayı açmadan): farklıysa kasada "Farklı" yazar ve ilişki (yakını/diğer) seçilmeden teslim edilmez.

## Test
`node bulut/musteri-takip/test/hepsi.mjs` (depo kökünden; yalnız sahte veri, ağ gerekmez, depoya dosya yazmaz).
Ekran görüntüsü için HTML isterseniz: `node bulut/musteri-takip/test/dene4.mjs <klasör>`.

## Kurulum
- **Ortak kod** yalnız `ortak/takip.mjs`'de düzenlenir; `./kopyala.sh` iki hedefe kopyalar.
- **Cloudflare**: `cd cloudflare && npx wrangler login && npx wrangler kv namespace create TAKIP`, çıkan kimliği
  `wrangler.toml`'a yaz, `GENEL_ANAHTAR`'ı doldur, `npx wrangler deploy`.
- **Vercel**: `vercel/` klasörü proje kökü; bir Blob deposu bağla (BLOB_READ_WRITE_TOKEN gelir),
  `GENEL_ANAHTAR` ortam değişkenini ekle, yükle. İsteğe bağlı `CRON_SECRET`.

Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].
