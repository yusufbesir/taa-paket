<!-- TAA Paketi çekirdek deposu tanıtımı. Sahip: Yusuf. yazan: [Claude#taa-paket-A0 2026-11-04]. Değişiklik: Yusuf'un kuralları çerçevesinde. -->
# taa-paket

TAA Paketi çekirdek deposu: TaaDön ve diğer TAA programlarının paylaştığı Rust crate'leri (Cargo çalışma alanı).

## Crate'ler

| Crate | Durum | Görev |
|---|---|---|
| taa-donustur | var | Güvenli zip/gzip ve çıktı imza denetimi (TaaDön çekirdeği, CLI) |
| taa-dosya | var | Biçim algılama, atomik yazma, geçici klasör |
| taa-ayar | var | Özellik anahtarı (her özellik ortam değişkeni/ayar dosyasıyla kapatılır) |
| taa-nebim | var | Nebim CSV dışa aktarım alıcısı, YALNIZ OKUMA (canlı bağlantı yok; xlsx yok) |
| taa-katalog | var | Nebim aynası ürün kataloğu, fiyat farkı; eski dosya `eski\`ye taşınır |
| taa-yedek | var | Klasör yedekleme, SHA256 doğrulama, üzerine yazmayan geri yükleme |
| taa-pdf | var | PDF sayfa seç/sırala/böl, birleştir, sıkıştır |
| taa-rehber | var | E-posta rehberi CSV, doğrulama, hiyerarşi |
| taa-yazici | var | Yazıcı Sağlık tanısı (Windows'ta salt okunur Get-Printer) |
| taa-tara | var | JPEG'den PDF (OCR/kenar düzeltme/PDF-A henüz yok) |
| taa-pano | var | Salt okunur şube panosu kartları (stok yok; stok kaynağı Nebim) |
| taa-kasa | var | TaaKasaDefteri: günlük kasa sayımı, fark notu zorunlu, eklemeli defter |
| taa-fis | var | TaaFiş: terzi fişi defteri, durum takibi, düz metin ve ESC/POS çıktı (telefon sayfası/OCR sonraki aşama) |
| taa-fis-sunucu | var | TaaFiş yerel sunucusu: telefon sayfası (PWA), QR/kodla eşleştirme, yazdırma. Çalıştır: `taa-fis-sunucu --yazici <yol>`, yönetim: `http://localhost:8787/yonetim` |
| taa-terzi | var | Tadilat işlem kataloğu (JSON, mağaza `terzi-kurallari.json` ile değiştirir) ve ölçü sınır denetimi |
| taa-kombin | var | TaaKombin: takım rengine göre gömlek/kravat/ayakkabı/kemer önerisi |
| taa-bildirim | var | TaaBildirim: müşteriye KVKK uyumlu "ürününüz hazır" mesajı kuyruğu (WhatsApp Business için gönderici bağlanacak; şimdilik yalnız kuyruk) |
| taa-takip | var | TaaTerziTakip raporları: kargo bekleyen, kuru temizleme, geciken, bugün teslim, usta performansı (yalnız sayı); salt okunur, yerel defterden |
| taa-telegram | var | TaaTelegram: izinli kullanıcılar için fiş komutları (/durum, /hazir, /raf, /imza); webhook yanıtı üretir, ağ kodu yok; varsayılan KAPALI |
| taa-subesor | var | TaaŞubeSor: şubeler arası beden/ürün sorusu ve yanıtları (ağ taşıması sonraki aşama) |
| taa-ayarlar | planlı | Ortak ayar yönetimi (dosya/arayüz) |
| taa-kimlik | var | Merkez sunucuya giriş (T7), cihaz imzalı istek, imzalı çevrimdışı yetki belgesi (S3a) |
| taa-cihaz | var | Cihaz kimliği (S2): kurulumda ed25519 anahtar, her isteğe `X-TAA-*` imza başlıkları; VPN yerine |
| taa-yol | var | Sunucuya ulaşım yolları (S4a): doğrudan IP / VPN / alan adı / Cloudflare sıralı listesi, ilk ulaşılan seçilir; oturum yol değişse de korunur |
| taa-dns | var | Dinamik DNS güncelleyici (yalnız sunucu PC'si): DuckDNS ya da kendi güncelleme adresi; belirteç dosyada, komut satırında/günlükte yok; `taa-dns zamanla` 5 dk görev |
| taa-cati | var | TAA Çatı (S3b): yalnız sunucu PC'sinde; kullanıcı/rol, cihaz onayı, program yetkisi, günlük, şube; silme yok, varsayılan reddet |
| taa-kur | var | Kurulum yöneticisi (T1): klasör paketi kur/sec/geri-don/kaldir/dogrula, SHA256; `eski\`ye taşır, silmez; veri alanına dokunmaz |
| taa-ipc | var | Programlar arası iletişim (T1): dosya tabanlı gelen kutusu, yetenek kaydı, zarf ve olay günlüğü; Windows adlandırılmış boru taşıyıcısı (T3, dosyaya yedekli; Windows'ta henüz doğrulanmadı) |
| taa-kod | planlı | Kodlama yardımcıları |
| taa-etiket | planlı | Etiketleme |
| taa-veri | var | Veri alanları (T2): tek yazar sahipliği, SAHIP.toml, salt okunur okuyucu, öneri kutusu; silmez (`eski\`); bkz. `docs/veri-alani.md` |
| taa-guncelle | var | Güncelleme (T3): `yayin.json` kontrolü, geri alma koruması, SHA256 + ed25519 doğrulama, `kur --sec`, hatada önceki sürüme dönme; yerel klasör kaynağı (HTTP T4) |
| taa-imza | var | ed25519 imza (T3): anahtar üret/imzala/doğrula, `ed25519:<hex>` genel anahtar dosyası; özel anahtar yalnız verilen dosyadan okunur, depoya yazılmaz |
| taa-log | planlı | Günlükleme |

## Terzi Takip: teslim sepeti (2026-11-26)

Teslimde birden çok fiş tek teslimatta verilir: bağlı fişlerden birinin karekodu okutulunca bütün bağlı fişler sepete
gelir, başka fişler sırayla okutulur ya da no'su yazılır; sepetin tamamı tek teslim alan bilgisi, tek karekod onayı ya da
tek imzayla, ortak bir teslim no'suyla (`T-YYYYAAGG-XXXXXX`) kaydedilir (ya hepsi ya hiçbiri).

**Fişli / fişsiz:** fiş yalnız **karekodu okutulursa fişli** sayılır (müşteri nüshasındaki takip karekodu; müşteri takip
ayarlı değilse gizli anahtarlı kanıt parçası `&k=` taşıyan fiş karekodu). Numarası elle yazılan, bağlı zincirden
kendiliğinden gelen ya da karekodunda kanıt parçası olmayan (2026-11-26 öncesi basılmış) fiş **fişsizdir**. Sepette
fişsiz fiş varsa fişsiz teslim yetkisi, her müşteri için telefonun son 4 hanesi ve imza zorunludur. Kasiyerin "fişli"
işaretini elle koyma yolu kaldırıldı (eski `teslim-al` ucunda `fisli: true` karekod kanıtı olmadan fişsiz sayılır;
`"bagli": [..]` ile gelen bağlı fişler sepet akışına çevrilir ve fişsizdir). Bir teslimatta en çok 30 fiş. Sınır:
tabanca klavyeden ayırt edilemez; mağaza cihazının müşteri nüshasını yeniden yazdırması artık `yeniden-yazdir`
yetkisine bağlıdır (varsayılan açık, yönetici kapatabilir; her yeniden basım `basim.jsonl`'da). Eski (kanıt öncesi
basılmış) fişler için yönetici **Ayarlar > Yönetim**'den gün olarak bir **eski fiş geçiş süresi** açabilir (varsayılan 0 =
kapalı; süre bitince kendiliğinden kapanır, başlangıçtan en çok 180 gün; her kullanım günlükte, günlük yazılamazsa
teslim yapılmaz). Yönetici girişi TAA ortak giriş kullanıcısıyla (yönetici rolü) ve o kişinin şifresi yeniden
sorularak yapılır, PIN yedektir; ortak girişte kişi değişince yönetici oturumu kapanır; günlüklerde "PC" yerine
oturumdaki kişi (PIN'le "PC (PIN)") yazılır. Yeniden yazdırma yetkisi kapalı cihaz müşteri karekodunu ekranda da
yalnız bir kez alır (ilk gösterim basım sayılır) ve hazır bildirimindeki takip bağlantısını maskeli görür.
Ayrıntı, uçlar ve atomiklik: [bulut/musteri-takip/README.md](bulut/musteri-takip/README.md#teslim-sepeti-tek-karekodla-zincir-çok-fiş-tek-teslimat-yusuf-2026-11-26-0625).

## Mobil

| Proje | Durum | Görev |
|---|---|---|
| mobil/terzi-takip-android | var (derlenmedi) | Terzi Takip Android uygulaması: mağaza telefonları için WebView kabuğu, yerel karekod okuyucu, sunucu adresi ve eşleştirme. Derleme ve kurulum: `mobil/terzi-takip-android/README.md` |

## Derleme

```
cargo build --release
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Çalıştırılabilir: `target/release/taa-donustur(.exe)`.

## Modül sözleşmesi

Her crate bir `modul.toml` taşır: bkz. [docs/modul-sozlesmesi.md](docs/modul-sozlesmesi.md).

Tek çatı belgeleri (plan §18):

- [docs/TEK-CATI.md](docs/TEK-CATI.md): kural, kurulum yöneticisi, sürüm uyumu, veri sahipliği
- [docs/T3-TASARIM.md](docs/T3-TASARIM.md): imza, güvenilir anahtarlar, güncelleme
- [docs/ENVANTER.md](docs/ENVANTER.md): veri konumları ve sürüm envanteri
- [docs/program-sozlesmesi.md](docs/program-sozlesmesi.md): program.toml sözleşmesi (taslak)
- [docs/sozlesmeler/](docs/sozlesmeler/): yetenek sözleşmeleri
- `tools/sozlesme_denetle.py`: CI sözleşme denetimi ([tools/sozlesme_denetle.py](tools/sozlesme_denetle.py))
