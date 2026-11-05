<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T0 2026-11-05] -->
# Değişiklik günlüğü

## Yayınlanmamış

T0 (tek çatı, veri ve yapı temizliği). Kırıcı değişiklik yok; yalnız ek alanlar, belge ve denetim.

- 20 `modul.toml` dosyasına `sunar`, `ister`, `veri_sahibi`, `veri_okur`, `ayar_alani` alanları eklendi (mevcut alanlar korundu).
- `tools/sozlesme_denetle.py` (+ testi) eklendi; CI'ya sözleşme denetimi adımı eklendi.
- `docs/ENVANTER.md` (veri konumları, ayar dosyaları, sürümler) eklendi.
- `docs/TEK-CATI.md` (plan §18'in depo gerçeğine uyarlanmış hali) ve `docs/program-sozlesmesi.md` (program.toml taslağı) eklendi.
- `docs/sozlesmeler/` altında yetenek sözleşmeleri eklendi (bilinmeyen alanlar "belirsiz").
- `uyumluluk.toml` (test edilmiş küme taslağı) eklendi.
- Karar: `taa-terzi` program=ortak; `taa-bildirim`, `taa-takip`, `taa-telegram` program=taafis (TaaFiş ailesi tek program). Ayrı TaaTerziTakip programı T2'de Cargo features ile.
- Denetim betiğine: gerektirir = Cargo bağımlılıkları birebir; ince çizgi (başka programın crate'ine zorunlu bağımlılık hata). CI'ya `permissions: contents: read`.
- `docs/modul-sozlesmesi.md` yeni alanlarla güncellendi; README "Modül sözleşmesi" bölümüne bağlantılar eklendi.
