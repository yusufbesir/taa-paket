<!-- TAA Paketi çekirdek deposu tanıtımı. Sahip: Yusuf. yazan: [Claude#taa-paket-A0 2026-11-04]. Değişiklik: Yusuf'un kuralları çerçevesinde. -->
# taa-paket

TAA Paketi çekirdek deposu: TaaDön ve diğer TAA programlarının paylaştığı Rust crate'leri (Cargo çalışma alanı).

## Crate'ler

| Crate | Durum | Görev |
|---|---|---|
| taa-donustur | var | Güvenli zip/gzip ve çıktı imza denetimi (TaaDön çekirdeği, CLI) |
| taa-dosya | var | Biçim algılama, atomik yazma, geçici klasör |
| taa-ayar | planlı | Ortak ayar yönetimi |
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
