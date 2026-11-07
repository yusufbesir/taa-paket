<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-S4 2026-11-07] -->
# video.uret

- Tür: yetenek
- Sürüm: @1
- Sunan: led-ekran-video (LED Ekran Video; depo dışı program, tek bilgisayar)
- Özet: dış LED ekran için yazı/çizim animasyonlu MP4 (ya da HAP) üretir
- İsteyenler: yok
- Çağrı: komut satırı `led-ekran-video --yazi "SATIR1|SATIR2" [--palet ...] [--mod sabit|kayan] [--genislik --yukseklik --fps --sure --harf --font --cikis --kodek mp4|hap]`; `taa-ipc` bağlanması sonraki aşama

## İstek alanları
yazi (satırlar `|` ile), palet, mod, genislik/yukseklik (px), fps, sure (sn), harf (px), font (yol, isteğe bağlı), cikis (dosya), kodek

## Yanıt alanları
Çıktı dosyası (cikis); `--onizleme dosya.ppm` ile tek kare. Kontrast/harf boyu/kayma hızı uyarıları (engel değil)

## Hata kodları
belirsiz (program kendi başlığında; ffmpeg yoksa durur)

## Bütçe (ms)
belirsiz

## Sürüm kuralı
Değişirse `@2` yeni dosya; eski `@1` en az bir sürüm daha yaşar (§18.3).
