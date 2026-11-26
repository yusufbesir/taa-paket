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
