<!-- TAA modül sözleşmesi belgesi. Sahip: Yusuf. yazan: [Claude#taa-paket-A0 2026-11-04]. Değişiklik: Yusuf'un onayıyla. -->
# modul.toml sözleşmesi

Her TAA modülü (crate veya program bileşeni) kök klasöründe bir `modul.toml` taşır.

| Alan | Tür | Açıklama |
|---|---|---|
| `ad` | metin | Modül adı (crate adıyla aynı). |
| `surum` | metin | SemVer; Cargo.toml `version` ile aynı tutulur. |
| `arayuz` | metin | Arayüz ana sürümü ("1", "2" ...). Modülün dışa sunduğu sözleşmenin sürümü. |
| `gerektirir` | tablo | Gerekli modüller: `{ "taa-dosya" = "1" }` (ad = en düşük arayüz ana sürümü). Boşsa `{}`. Cargo taa-* bağımlılıklarıyla birebirdir; `--bin` paketine yazılmaz (statik). |
| `calisma_gerektirir` | tablo (isteğe bağlı) | Derlenmiş ikilinin çalışma anında kurulu olmasını istediği paketler; `paket_uret.py --bin` paket.json `gerektirir` alanını buradan alır (`docs/paket-zip.md`). |
| `ozellikler` | liste | Modülün sunduğu yetenek adları (örn. `["arsiv", "imza"]`). |
| `program` | metin | Bağlı program (örn. `taadon`); birden çok programca paylaşılıyorsa `ortak`. |
| `butce` | tablo | İsteğe bağlı performans bütçeleri (örn. `{ acilis_ms = 800 }`). |
| `sunar` | liste | IPC yetenekleri, `ad.fiil@N` biçiminde (örn. `["kod.uret@1"]`). İsteğe bağlı. |
| `ister` | liste | Başka modüllerin sunduğu, isteğe bağlı yetenekler. Yoksa ilgili düğme gizlenir. |
| `veri_sahibi` | liste | Yalnız bu modülün yazdığı veri alanları. |
| `veri_okur` | liste | Salt okunur kullanılan veri alanları. |
| `ayar_alani` | metin | `taa-ayar` içindeki ayar alanı (örn. `programlar.taarkod`). |

Not: `ozellikler` (eski, serbest etiket listesi) ile `sunar` (sürümlü IPC yeteneği) ilişkisi belirsiz;
`ozellikler` şimdilik değiştirilmeden kalır.

## Uyumluluk kuralı

- `arayuz` ana sürümü değişmedikçe eski ve yeni modül sürümleri birlikte çalışmak zorundadır
  (yalnız ekleme; var olan davranış/alan kaldırılmaz veya anlamı değiştirilmez).
- Uyumsuz değişiklik `arayuz` ana sürümünü artırır; bu durumda eski ve yeni arayüz bir süre
  yan yana sunulur, tüketiciler kendi hızında geçer.
- `surum` her yayında artar; `arayuz` yalnız uyumsuz değişimde artar.

## Örnek

```toml
ad = "taa-dosya"
surum = "0.1.0"
arayuz = "1"
gerektirir = {}
ozellikler = ["bicim", "atomik-yazma", "gecici"]
program = "ortak"
```

## Yetenek adı biçimi

`ad.fiil@N` — küçük harf, tire serbest; `ad` = konu, `fiil` = eylem, `N` = yetenek sürümü (tam sayı).
Örnek: `urunler.ara@1`, `dosya.yaz@1`. Paket genelinde her ad benzersizdir.
Yetenek değişirse `@N+1` yayımlanır; eski `@N` en az bir sürüm daha sunulur.

## `gerektirir` yalnız çekirdek crate

- `gerektirir` içinde yalnız çekirdek crate adları (`taa-ayar`, `taa-dosya`, `taa-kod` ...) yazılır.
- Program adı (`tutaa`, `taadon`, `taarkod`, `fiyat-etiketi`, `taarehber` ...) yazılamaz.
- Programlar arası ilişki yalnız `ister` (isteğe bağlı yetenek) veya `veri_okur` (paylaşılan veri) ile kurulur.
- Sonuç: bir program kaldırılınca diğerleri açılır ve çalışır; en fazla bir düğme gizlenir.

## Veri sahipliği: tek yazar, çok okur

- Her veri alanının paket genelinde tam bir sahibi vardır (`veri_sahibi`); yalnız sahip yazar.
- Okuyanlar `veri_okur` ile bildirir ve hiçbir zaman yazmaz; değişiklik önerisi sahibe iletilir.
- Bir alanda iki sahip yasaktır. Sahibi olmayan alan okunamaz (okuyan bildirimi geçersiz sayılır).
- Sahip kaldırılırsa veri silinmez; okuyanlar "son kopya, salt okunur" ile çalışır.
- Yazım atomiktir (`taa-dosya` `yaz@1` yeteneği / `uzerine_yaz_atomik`).

## CI denetim listesi

`python3 tools/sozlesme_denetle.py` her PR'da (`ci.yml` işi `sozlesme`) çalışır; kırmızıysa PR birleşmez:

1. `gerektirir` içinde program adı yok (yalnız çekirdek crate).
2. Her `veri_okur` alanının paket genelinde tam bir sahibi var; iki sahip yok.
3. Yetenek adları benzersiz ve `ad.fiil@N` biçiminde.
4. `surum` Cargo.toml `version` ile aynı.
5. `arayuz` değiştiyse DEGISIKLIK.md'de "kırıcı" satırı var (planlı; betikte henüz yok).
6. `gerektirir` anahtarları Cargo.toml `[dependencies]` içindeki `taa-*` crate'lerle birebir (dev-bağımlılık hariç).
7. İnce çizgi: `gerektirir` hedefi `program = "ortak"` ya da aynı programın crate'i olmalı; başka programın crate'ine zorunlu bağımlılık hata.
8. `ad` klasör adı ve Cargo.toml `name` ile aynı.

Denetimin tam kapsamı betiğin kendisindedir (`tools/sozlesme_denetle.py`); fark olursa betik esastır.
Ayrıntı: PLAN-TAA-PAKET §18.3.
