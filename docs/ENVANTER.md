# TAA paketi envanteri ve sözleşme denetimi (T0, 2026-11-05)
Kaynak: PLAN-TAA-PAKET §18.6 T0. Araç: `python3 -I tools/sozlesme_denetle.py [--siki] [--json]`. yazan: [Claude#taa-paket-mimari 2026-11-05]

## Durum
- 20 crate, 20 `modul.toml`; hepsi ad = klasör, surum = Cargo.toml, biçim ve yetenek adları geçerli, döngü yok, `ortak` modüller yalnız `ortak` modüllere bağlı. Hata: 0.
- `programlar/*/program.toml` henüz yok (T2); denetim varsa veri sahipliğini (tek yazar, okuyanın sahibi var) ve `gerektirir`'in crate olmasını da bakar.

## Uyarılar (ince çizgi kuralı: program başka programın crate'ine bağlanmamalı)
| Bağ | Öneri |
|---|---|
| taa-fis → taa-terzi (taafis → taaterzi) | taa-terzi (işlem kataloğu, sınır denetimi) fiş dışında da kullanılıyor: `program = "ortak"` yap |
| taa-fis-sunucu → taa-terzi | aynı |
| taa-takip → taa-fis (taatakip → taafis) | fiş veri modeli ortak çekirdeğe (`taa-fis` ortak) ya da takip `veri_okur = ["fisler"]` ile salt okur okusun |
| taa-telegram → taa-fis, taa-bildirim | bildirim kuyruğu `ortak`, fiş verisi salt okunur alan; Telegram yalnız isteğe bağlı yetenek çağırsın |
Bu sınıflandırma kararı ilgili program sahiplerinindir (iş sahipliği); denetim şu an uyarı verir, `--siki` ile hata olur. `taa-terzi` için önerilen ilk adım `program = "ortak"`.

## Sonraki
- T1: `taa-kur` (kayıt, çözücü, kur/kaldır/seç/geri-dön) ve `taa-ipc` küçük sürüm.
- CI adımı: `.github/workflows/ci.yml`'e `python3 -I tools/sozlesme_denetle.py` eklenmesi (aşağıdaki not).
