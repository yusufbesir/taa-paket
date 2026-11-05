<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T0 2026-11-05] -->
# Değişiklik günlüğü

## Yayınlanmamış

### T2-A3

- `tools/sozlesme_denetle.py` yeni program modu: `--program <program.toml> [--surum-dosyasi <package.json|Cargo.toml>] [--paket-kok <taa-paket>]` (zorunlu alanlar, ad ve alan adı biçimi, `gerektirir` yalnız `taa-*`, `sunar`/`ister` biçimi ve tekilliği, sürüm eşitliği, `ayar_alani`, `docs/veri-sahipleri.toml` ile sahip/okur çapraz denetimi; `[veri.<alan>] gecici = true` geçici sahip uyarı verir). Mevcut paket denetimi değişmedi; 13 yeni test.
- `docs/veri-sahipleri.toml` (paket genelinde veri alanı sahip tablosu) eklendi.
- `docs/program-sozlesmesi.md`: "Taslak örnekler" yerine dört programın kesin değerleri (tutaa, taadon, taarkod, fiyatetiketi), taarehber "ayrı başlık, taslak"; geçici sahip notasyonu belgelendi.
- `araclar/js/taa-veri.js` (bağımlılıksız CommonJS veri alanı kitaplığı, programlar kopyalar) ve `araclar/js/test/taa-veri.test.js` eklendi; CI'ya `js` işi (Node 22, `node --test`) eklendi.

### T2-A1

- `crates/taa-veri` eklendi (program = ortak, gerektirir = taa-dosya; yeni Cargo bağımlılığı yok): veri alanı sahipliği (`ac_sahip`, `ac_gecici_sahip`, `ac_okur`), atomik yazım, öneri kutusu (`oner`, `oneriler`, `oneri_kapat`), `SAHIP.toml` düz TOML alt kümesi çözücü/yazıcı (hata satır numaralı). Kodda silme yok; devralınan `SAHIP.toml` ve kapatılan öneriler `eski/` altına taşınır.
- `docs/veri-alani.md` (veri alanı sözleşmesi: kök, SAHIP.toml, 6 kural, hata adları, öneri biçimi, göç kuralı) ve `docs/veri-sahipleri.toml` (paket genelinde sahip tablosu) eklendi.
- README crate tablosunda `taa-veri` "var"; `docs/TEK-CATI.md` §3'e T2 durum notu eklendi.

T1 (kurulum yöneticisi ve iletişim; yeni Cargo bağımlılığı yok).

- `taa-kur` (klasör paketi kurulumu, sürüm seçme, geri dönme, SHA256 doğrulama; zip ve imza T2) ve `taa-ipc` (dosya tabanlı gelen kutusu taşıyıcısı) crate'leri eklendi.
- `tools/paket_uret.py` (+ testi) eklendi: `modul.toml` + Cargo sürümü -> `paket.json`; `icerik_sha256` tanımı `paket.json`'a `icerik_sha256_tanim` olarak yazılır ve `docs/T1-TASARIM.md` §1.2'de kesinleştirildi.
- `tools/sozlesme_denetle.py`: iki yeni denetim (hata): `gerektirir` grafında döngü; istenen arayüz ana sürümü hedef modülün `arayuz` değerine eşit olmalı. Testler eklendi.
- README crate tablosu: `taa-kur` ve `taa-ipc` "var".
- `docs/TEK-CATI.md` §3'e T1 durum notu eklendi.

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
