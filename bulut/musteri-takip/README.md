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
- **Fişli teslimde imza isteğe bağlıdır** ve "fişli" işaretini kasiyer koyar (fişin gerçekten okutulduğu doğrulanmaz):
  fişsiz teslimi "fişli" işaretleyen kasiyer imzasız teslim edebilir. Önceki tasarımdan gelir; Yusuf'a soruldu
  (ör. fişli teslimde fiş karekodunun okutulması zorunlu olsun mu).
- Şifreli yükte düz metin P = `{ad, tel, ua, ip, zaman, dil, imza}`; `imza` öneksiz base64 PNG'dir (≤ 81 920 karakter).
  En büyük imzayla `GET /api/onay/<t>` yanıtı ~110 KB'dir (sunucu en az 128 KiB okumalı).
- Bilinen sınır: tuval klavyeyle kullanılamaz; imza atamayan müşteri için mağaza cihazında imza (`cihaz_imza`) yolu vardır.
- Gerçek telefonlarda (iOS Safari, Android Chrome) dokunmatik çizim ve gönderim elle denenmedi; yalnız sahte DOM testi var.

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
