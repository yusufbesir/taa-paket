<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T0 2026-11-05] -->
# program.toml sözleşmesi

Her program kendi deposunun kökünde `program.toml` taşır (PLAN-TAA-PAKET §18.3). Programlar kendi
depolarında yaşar; kesin değerler aşağıdaki "Kesin örnekler" bölümündedir (T2).

## Alanlar

| Alan | Tür | Açıklama |
|---|---|---|
| `ad` | metin | Program kimliği (küçük harf, tire). |
| `gorunen_ad` | metin | Kullanıcıya görünen ad. |
| `surum` | metin | SemVer; programın Cargo.toml sürümüyle aynı. |
| `arayuz` | metin | Arayüz ana sürümü. Değişirse DEGISIKLIK.md'de "kırıcı" satırı gerekir. |
| `gerektirir` | tablo | YALNIZ çekirdek crate'ler (`{ taa-ayar = "1" }`); program adı yazılamaz. |
| `sunar` | liste | IPC yetenekleri, `ad.fiil@N`. |
| `ister` | liste | İsteğe bağlı yetenekler; yoksa düğme gizlenir. |
| `veri_sahibi` | liste | Yalnız bu programın yazdığı veri alanları. |
| `veri_okur` | liste | Salt okunur alanlar; her birinin paket genelinde tam bir sahibi olmalı. |
| `ayar_alani` | metin | Ayar alanı, `programlar.<ad>`. |

Örnek (§18.3):

```toml
ad = "taarkod"
gorunen_ad = "TaaRkod"
surum = "1.4.0"
arayuz = "1"
gerektirir = { taa-ayar = "1", taa-dosya = "1", taa-kod = "1" }
sunar  = ["kod.uret@1", "etiket.yazdir@1"]
ister  = ["veri.urunler@1"]
veri_sahibi = ["etiket-kuyrugu"]
veri_okur   = ["urunler", "rehber"]
ayar_alani  = "programlar.taarkod"
```

## Kesin örnekler (T2, dört program)

Kaynak: docs/T2-TASARIM.md §2. `surum` programın `package.json` `version` değeriyle aynı olmalıdır
(`tools/sozlesme_denetle.py --program ... --surum-dosyasi package.json`). T2'de sürüm yükseltilmez.
`sunar`/`ister` T2'de yalnız bildirimdir (IPC bağlanması T3); `ister` karşılığı yoksa ilgili düğme gizlenir.

### Geçici sahip notasyonu

Asıl sahip programı henüz kurulmamış bir alanı başka bir program geçici olarak yazıyorsa, o programın
`program.toml` dosyasında alan `veri_sahibi`ne yazılır ve ayrıca şu tablo eklenir:

```toml
[veri.rehber]
gecici = true
```

`veri-sahipleri.toml` tablosundaki asıl sahip başka bir program olduğu için denetleyici bunu hata değil
UYARI sayar. Asıl sahip kurulunca alanı devralır (docs/T2-TASARIM.md §1, kural 2). `gecici = true` olmadan
tablodaki sahipten başka bir programın alanı sahiplenmesi hatadır.

### tutaa

```toml
ad = "tutaa"
gorunen_ad = "Tutaa"
surum = "2.5.0"                 # tutaa-paketi/masaustu/package.json ile aynı olmalı; denetimde doğrulanır
arayuz = "1"
gerektirir = {}                 # Electron; Rust bağımlılığı yok
sunar  = ["tutanak.olustur@1"]
ister  = ["rehber.ara@1"]
veri_sahibi = ["rehber"]        # GEÇİCİ sahip; taarehber kurulunca devralır
veri_okur   = []
ayar_alani  = "programlar.tutaa"

[veri.rehber]
gecici = true
```

### taadon (TaaDön)

```toml
ad = "taadon"
gorunen_ad = "TaaDön"
surum = "1.12.0"
arayuz = "1"
gerektirir = { taa-donustur = "1" }
sunar  = ["belge.donustur@1"]
ister  = []
veri_sahibi = ["belgeler"]      # userData/gecmis.json -> belgeler/kayitlar.json
veri_okur   = []
ayar_alani  = "programlar.taadon"
```

### taarkod (TaaRkod)

```toml
ad = "taarkod"
gorunen_ad = "TaaRkod"
surum = "1.8.0"                 # masaustu/package.json
arayuz = "1"
gerektirir = {}
sunar  = ["kod.uret@1"]
ister  = ["etiket.kuyruga_ekle@1"]
veri_sahibi = []
veri_okur   = ["etiket-kuyrugu"]   # yazmaz; değişikliği oneriler/ altına bırakır
ayar_alani  = "programlar.taarkod"
```

### fiyatetiketi (Fiyat Etiketi)

```toml
ad = "fiyatetiketi"
gorunen_ad = "Fiyat Etiketi"
surum = "1.32.0"
arayuz = "1"
gerektirir = {}                 # çekirdek kendi wasm'ıdır
sunar  = ["etiket.yazdir@1", "etiket.kuyruga_ekle@1"]
ister  = []
veri_sahibi = ["etiket-kuyrugu"]
veri_okur   = ["urunler"]       # alan doğarsa salt okunur
ayar_alani  = "programlar.fiyatetiketi"
```

### taarehber (ayrı başlık, taslak)

E-posta rehberi programı ayrı bir başlıkta ele alınır; aşağıdaki yalnız taslaktır, T2 kapsamında değildir.
`rehber` alanının asıl sahibi `docs/veri-sahipleri.toml`da `taarehber`dir; kurulunca Tutaa'dan devralır.

```toml
ad = "taarehber"
gorunen_ad = "Rehber"
surum = "0.1.0"                 # taslak, belirsiz
arayuz = "1"
gerektirir = {}                 # taslak, belirsiz
sunar  = ["rehber.ara@1"]       # taslak
ister  = []
veri_sahibi = ["rehber"]
veri_okur   = []
ayar_alani  = "programlar.taarehber"
```

## Notlar

- `ad` biçimi `[a-z0-9-]`; veri alanı adları `[a-z0-9-]`, en çok 32 karakter. Program kimliği olarak `taarehber` kullanılır.
- Denetim: `python3 tools/sozlesme_denetle.py --program program.toml --surum-dosyasi package.json --paket-kok <taa-paket>`.
- Gerçek `program.toml` dosyaları her programın kendi deposunda tutulur (T2 aşaması).
