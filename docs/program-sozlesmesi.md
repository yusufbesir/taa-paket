<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T0 2026-11-05] -->
# program.toml sözleşmesi

Her program kendi deposunun kökünde `program.toml` taşır (PLAN-TAA-PAKET §18.3). Programlar kendi
depolarında yaşar; bu belgedeki dosyalar yalnız TASLAK örnektir, kesin değerler her programın deposunda belirlenir.

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

## Taslak örnekler (beş program)

Uyarı: sürümler ve `gerektirir` içeriği belirsiz; plandaki uyumluluk kümesinden (tutaa 2.5.x, taadon 1.6.x,
taarkod 1.x, fiyat-etiketi 1.30.x, taa-rehber 0.x) alınmış yer tutuculardır. Yetenek adları plan sentezindeki
adlara dayanır; planda geçmeyenler "belirsiz" notludur.

### tutaa

```toml
ad = "tutaa"
gorunen_ad = "Tutaa"
surum = "2.5.0"                 # belirsiz (yalnız 2.5.x kümesi biliniyor)
arayuz = "1"
gerektirir = { taa-ayar = "1", taa-dosya = "1" }   # belirsiz: gerçek çekirdek bağımlılıkları depodan doğrulanmalı
sunar  = []                      # belirsiz
ister  = ["rehber.oku@1", "arsiv.olustur@1"]       # belirsiz: tutanak alıcı önerisi, tutanak eki
veri_sahibi = ["kimlik", "urunler"]                # §18.5: Tutaa kimlik ve (merkez varsa) urunler sahibi
veri_okur   = ["rehber", "belgeler"]
ayar_alani  = "programlar.tutaa"
```

### taadon (TaaDön)

```toml
ad = "taadon"
gorunen_ad = "TaaDön"
surum = "1.6.0"                 # belirsiz (1.6.x kümesi)
arayuz = "1"
gerektirir = { taa-ayar = "1", taa-dosya = "1" }   # belirsiz
sunar  = ["arsiv.ac@1", "arsiv.olustur@1", "arsiv.sikistir@1", "imza.denetle@1"]
ister  = ["rehber.oku@1"]        # belirsiz: gönderim taslağı için
veri_sahibi = ["belgeler"]
veri_okur   = ["rehber"]
ayar_alani  = "programlar.taadon"
```

### taarkod (TaaRkod)

```toml
ad = "taarkod"
gorunen_ad = "TaaRkod"
surum = "1.4.0"
arayuz = "1"
gerektirir = { taa-ayar = "1", taa-dosya = "1", taa-kod = "1" }
sunar  = ["kod.uret@1", "etiket.yazdir@1"]
ister  = ["urunler.ara@1"]       # belirsiz: planda "veri.urunler@1" geçiyor, sentezde "urunler.ara@1"
veri_sahibi = ["etiket-kuyrugu"]
veri_okur   = ["urunler", "rehber"]
ayar_alani  = "programlar.taarkod"
```

### fiyat-etiketi (Argox Fiyat Etiketi)

```toml
ad = "fiyat-etiketi"
gorunen_ad = "Argox Fiyat Etiketi"
surum = "1.30.0"                # belirsiz (1.30.x kümesi)
arayuz = "1"
gerektirir = { taa-ayar = "1", taa-dosya = "1" }   # belirsiz
sunar  = []                      # belirsiz
ister  = ["urunler.ara@1"]       # belirsiz
veri_sahibi = []                 # etiket-kuyrugu sahibi TaaRkod; yoksa kendi yerel kuyruğunu açar
veri_okur   = ["urunler", "etiket-kuyrugu"]
ayar_alani  = "programlar.fiyat-etiketi"
```

### taarehber (taa-rehber)

```toml
ad = "taarehber"
gorunen_ad = "Rehber"
surum = "0.1.0"                 # belirsiz (0.x kümesi)
arayuz = "1"
gerektirir = { taa-ayar = "1", taa-dosya = "1" }   # belirsiz
sunar  = ["rehber.oku@1", "rehber.dogrula@1"]
ister  = ["csv.ayristir@1"]
veri_sahibi = ["rehber"]
veri_okur   = []
ayar_alani  = "programlar.taarehber"
```

## Notlar

- `csv.ayristir@1` sunanı taa-nebim'dir (çekirdek crate değil, modül); `ister` ile isteğe bağlıdır, `gerektirir`e yazılmaz.
- `ad` biçimi: plan "taa-rehber" crate adını kullanır; program kimliği olarak `taarehber` mi `taa-rehber` mi belirsiz.
- Gerçek `program.toml` dosyaları her programın kendi deposunda tutulur (T2 aşaması).
