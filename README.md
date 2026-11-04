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
