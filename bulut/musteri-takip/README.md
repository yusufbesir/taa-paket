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
