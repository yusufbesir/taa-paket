# Müşteri ürün takibi (Cloudflare + Vercel)

Müşteri, fişin MÜŞTERİ nüshasındaki karekodu okutup tadilatının durumunu görür (Yusuf, 2026-11-26).

## Nasıl çalışır
- Mağaza sunucusu (`taa-fis-sunucu`) internete **açılmaz**. Fiş kesilince ve durum değişince kaydı kendisi
  dışarı **iter** (dakikada bir, `curl`), iki hedefe birden: Cloudflare ve Vercel. Biri düşükse diğeri çalışır.
- Kayıt yalnız: takip kodu, durum, şube kodu, teslim günü, zaman, silinme zamanı. Ad, telefon, ölçü, fiyat,
  fiş numarası gitmez (`musteri_takip.rs` testi denetler).
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

## Kurulum
- **Ortak kod** yalnız `ortak/takip.mjs`'de düzenlenir; `./kopyala.sh` iki hedefe kopyalar.
- **Cloudflare**: `cd cloudflare && npx wrangler login && npx wrangler kv namespace create TAKIP`, çıkan kimliği
  `wrangler.toml`'a yaz, `GENEL_ANAHTAR`'ı doldur, `npx wrangler deploy`.
- **Vercel**: `vercel/` klasörü proje kökü; bir Blob deposu bağla (BLOB_READ_WRITE_TOKEN gelir),
  `GENEL_ANAHTAR` ortam değişkenini ekle, yükle. İsteğe bağlı `CRON_SECRET`.

Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].
