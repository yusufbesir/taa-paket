<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T0 2026-11-05] -->
# veri.degisti

- Tür: olay
- Sürüm: @1
- Sunan: alan sahibi programlar
- Özet: Alan değişince yayımlanan olay: `veri_degisti(alan)`; IPC yoksa dosya zamanı ile fark edilir (§18.5)
- İsteyenler: yok (modul.toml / verilen listeye göre)
- Çağrı: `taa-ipc` (T1'de gelecek; şimdilik crate içi çağrı). Kaynak: PLAN-TAA-PAKET §18.3

## Olay yükü
- `alan`: değişen veri alanı adı (§18.5'ten)
- Diğer alanlar: belirsiz

## Hata kodları
belirsiz (olay; yanıt yok)

## Bütçe (ms)
belirsiz

## Sürüm kuralı
Değişirse `@2` yeni dosya; eski `@1` en az bir sürüm daha yaşar (§18.3).
