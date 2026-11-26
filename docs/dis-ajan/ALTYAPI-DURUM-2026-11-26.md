<!-- description: Altyapı parçalarının depodaki gerçek durumu (kod taraması). sahip: Yusuf; yazan: [Claude Sonnet 5.5, oturum_0172VKmzcptJNFEZH73wSUqt]; tarih: 2026-11-26. Silme yok; güncelleme üstünü çizerek. -->
# Altyapı durumu (kod taraması, derleme/test YAPILAMADI)

Yöntem: `crates/` satır sayıları ve dosya başlıkları okundu. Derleme denendi, makinede bellek yetersizliğiyle düştü (Windows hata 1455 "disk belleği dosyası çok küçük", rustc "out of memory", boş RAM ~780 MB). Bu yüzden aşağıdaki "var" demek "kod var", "testi geçti" demek DEĞİL.

| Parça | Depoda | Not |
|---|---|---|
| TAA İmza (taa-imza) | VAR, 822 satır | ed25519 imza/doğrulama, anahtar dosyası kuralları |
| Veri alanları (taa-veri) | VAR, 1073 satır | tek yazar, SAHIP.toml, öneri kutusu |
| Veri havuzu istemcisi (taa-havuz) | VAR, 1029 satır | önbellek, sha256 doğrulamalı indirme |
| Kuyruk (taa-kuyruk) | VAR, 1893 satır | yerel kuyruk, aynı `no` çift yazılmaz |
| Ortak OCR motoru (taa-ocr) | VAR, 0.1.0, 1262 satır | metin katmanı çalışır; görüntü OCR modeli YOK (model yoksa `belirsiz`); IPC yok; Windows'ta denenmedi |
| Rehber (taa-rehber) | Yalnız çekirdek, 242 satır | CSV okuma, hiyerarşi; arayüz/program yok |
| TAA İşlem Gücü | Kod YOK | plan onaylı; tehdit/test taslağı: GPT-ISLEM-GUCU-TEHDIT-VE-TEST.md |
| TAA Eğitim, İhbar hattı | Kod YOK (taranan crate listesinde yok) | |

Sonuç: "TAA İmza ve veri tabanı deposu büyük olasılıkla yapılmadı" varsayımı kısmen yanlış: imza, veri, havuz, kuyruk crate'leri var. Eksik olan: Windows'ta derleme/test kanıtı, işlem gücü dağıtıcısı, defter (zincir+Merkle), eğitim, ihbar hattı, OCR modeli.