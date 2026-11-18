<!-- Sahip: Yusuf. yazan: [Claude#hata-bildirimi 2026-11-18] -->
# Hata bildirimi (taa-hata)

Karar (Yusuf 2026-11-18): paket programları çökünce ya da hata alınca hata bildirimi göndersin; düzeltmeyi kolaylaştırsın.

## Akış
1. Program `main` başında `taa_hata::kur_varsayilan("<program>", env!("CARGO_PKG_VERSION"))` çağırır (panik kancası).
2. Çökme/hata → `Rapor` (program, sürüm, tür, yer, temizlenmiş mesaj, son ≤20 kayıt satırı, işletim sistemi, saat, parmak izi).
3. Rapor yerel kuyruğa yazılır: `<veri kökü>/kuyruk/hata-<program>/` (taa-kuyruk). Kanca ağa gitmez.
4. Sunucuya ulaşılınca `taa_hata::bekleyenleri_gonder` (taa-kur zamanlayıcısı ya da program açılışı) `POST /v1/hata-raporu` yollar.
   Aynı hata aynı gün tek kayıt (`no = H-<parmak izi>-<gün>`); sunucu `no` ile tekrarı yazmaz.

## Gizlilik (dışarıya veri yok)
`temizle`: tırnak içi değerler `<metin>`, e-posta, 4+ rakamlı sayılar, 24+ karakterli belirteçler, sır anahtarından sonraki değer
(`parola=`, `token`, `bearer`…) çıkarılır; yollar yalnız dosya adına iner. Rapor yalnız TAA Sunucu'ya gider. Dosya içeriği/müşteri verisi eklenmez.
Kuyruk gövdesi şifrelidir: `taa_kuyruk::sifre` (ChaCha20-Poly1305, `<kök>/anahtar/kuyruk.key`, makine başına otomatik); JS tarafı aynı biçimi
üretir (Node ↔ Rust çalışması denendi). `Kuyruk::gonder` gönderirken çözüp sunucuya düz gövdeyi TLS ile yollar; anahtar yoksa/yanlışsa kayıt
`reddedildi.jsonl`ye düşer. Kapsam: kuyruk dosyasının tek başına okunması; anahtar dosyasına da erişen saldırgana karşı değil (rapor zaten temizlenmiş).

## Sunucu sözleşmesi (Tutaa sunucusu yazacak)
`POST /v1/hata-raporu` gövde: `{bicim:1, program, surum, tur:"cokme"|"hata", yer, mesaj, son_satirlar:[..], parmak_izi, isletim, zaman}`.
Cihaz imzası + Bearer (diğer uçlar gibi). Yanıt 2xx; kayıt `hata_raporlari` tablosunda `(program, parmak_izi)` ile birleşir, sayaç artar. TAA Çatı'da "Hatalar" listesi.

## Yaygınlaştırma durumu
- Çatı: `taa-cati hata liste [--program]` ve panelde "Hatalar" sekmesi (`GET /v1/hata-raporu`, `gunluk.gor`).
- Sunucu ucu: tutaa #45. Electron: tutaa #46, taarkod #8, dosya-donusturucu #14, fiyat-etiketi #12 (hata-bildirimi.js).
- Rust (bu depo): taa-kur, taa-cati, taa-donustur, taa-fis-sunucu, taa-guncelle, taa-ocr, taa-rapor, taa-raporlar, taa-yedek bağlandı (kimlik/cihaz/imza/yol/kuyruk küçük araçlar, bağlanmadı). Gönderim: `taa-kimlik zamanla kur` (Windows) 5 dakikada bir `taa-kimlik gonder` çalıştırır (pencere açmayan `taa-kimlik-arka`); tüm kuyruklar, hata raporları dahil, oturumla gider. Windows'ta elle denenmedi.
- Electron programları (Tutaa, Etiket, TaaRkod, TaaDön) ve Python/diğer: aynı biçimi üreten küçük JS yardımcı, kendi depolarında.
