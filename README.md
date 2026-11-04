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
| taa-subesor | var | TaaŞubeSor: şubeler arası beden/ürün sorusu ve yanıtları (ağ taşıması sonraki aşama) |
| taa-ayarlar | planlı | Ortak ayar yönetimi (dosya/arayüz) |
| taa-kimlik | planlı | Teknik kimlikler |
| taa-ipc | planlı | Süreçler arası iletişim |
| taa-kod | planlı | Kodlama yardımcıları |
| taa-etiket | planlı | Etiketleme |
| taa-veri | planlı | Veri katmanı |
| taa-guncelle | planlı | Güncelleme |
| taa-log | planlı | Günlükleme |

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
