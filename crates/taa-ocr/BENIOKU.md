# taa-ocr: TAA ortak OCR motoru
Sahip: Yusuf. Sözleşme: `docs/OCR-MOTORU.md`. Yeni bağımlılık yok (lopdf, zip, sha2, serde_json, taa-imza, taa-dosya).

## Durum (0.1.0)
- Girdi içeriğe göre tanınır: jpg, png, tiff, bmp, webp, pdf, docx. Parola korumalı: `{"hata":"korumali"}`; desteklenmeyen: `{"hata":"desteklenmeyen-bicim"}`.
- Metin katmanı doğrudan çıkarılır, OCR çalışmaz (`kaynak:"metin-katmani"`, güven 1.0): docx metni ve tabloları, metin katmanlı PDF.
- Görüntü, taranmış PDF sayfasındaki gömülü JPEG ve fotoğraf etkin model paketine verilir (`kaynak:"ocr"`). **Model yoksa** `belirsiz:true`, boş metin, `uyari:["model-yok"]` döner; uydurma sonuç yok.
- Sözlükle kısıtlı eşleme (`ocr.kod@1`, tablodaki stok sütunu): Tutaa `motor/PROTOKOL.md` kuralı (d1 ≤ 1, d2 > d1, güven ≥ 0.80, aksi sor, aday yoksa bos).
- Model paketi: `model-etkinlestir` imza (ed25519) ve dosya özetlerini doğrular, `ocr/modeller/<ad>/<surum>/` altına kurar; `model-geri-al` bir önceki sürüme döner, klasörler silinmez.

## Model komut sözleşmesi
`model.json`: `{"ad","surum","turler":["elyazisi"],"komut":["./calistir"],"dosyalar":{"yol":"sha256"},"imza":{...}}` (imza `taa_imza::kanonik_bayt` üzerinde).
Motor modeli `<komut...> <goruntu> --tur T --dil D [--sozluk dosya]` ile çalıştırır; model stdout'a **tek satır JSON** yazar:
`{"metin","guven","satirlar":[{"metin","guven","kutu"}],"tablolar":[...ocr.tablo@1],"goruntu":{"w","h"},"belirsiz"}`.
Model Python ya da yürütülebilir olabilir (mevcut el yazısı Python prototipi ilk model olarak sarılabilir). 180 sn zaman aşımı; görüntü geçici dosyadan okunur ve iş bitince silinir.

## Komutlar
`taa-ocr oku|kod|tablo <dosya> [--tur] [--dil] [--sozluk] --json`, `bilgi`, `model-liste`, `model-etkinlestir <klasor> [--genel-anahtar]`, `model-geri-al <ad>`. Kök: `--kok`, `TAA_KOK` ya da exe klasörü; varsayılan genel anahtar `<kök>/ocr/model-imza.pub`.

## Henüz yok
- IPC yolu (`taa-ipc`); şimdilik yalnız komut satırı.
- Görüntü OCR modeli (eğitim proje kuralıyla en sona bırakıldı). PDF sayfasını görüntüye çevirme (yalnız gömülü JPEG kullanılır). docx dışı Office biçimleri.
- Windows'ta denenmedi; testler Linux'ta (`cargo test -p taa-ocr`).
