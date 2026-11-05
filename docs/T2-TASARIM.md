<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T2 2026-11-05] (Fable, mimar). T2 "büyük iş" tasarımı. -->
# T2 tasarımı: program.toml, veri alanları (`taa-veri`), zip paket, program depolarına geçiş

Kaynak: PLAN-TAA-PAKET.md §18, docs/TEK-CATI.md, docs/T1-TASARIM.md, docs/ENVANTER.md.
Kurallar (değişmez): yeni Cargo bağımlılığı YOK (`toml`, `interprocess`, `ed25519-dalek` Yusuf onayı bekler; onay gelmediği için **Python/JS üretir ve okur, Rust JSON ya da düz-TOML alt kümesi okur**). Hiçbir şey silinmez (`eski\`/`.tasindi`). Rust + Python; Electron programlarında zorunlu olduğu için küçük, bağımlılıksız bir JS modülü (`araclar/js/taa-veri.js`) kaynağı bu depodur, programlar **kopyalar** (program→program bağımlılığı yok; §18.1). Türkçe adlar ve mesajlar. Her iş ayrı PR; CI yeşil olmadan birleşmez.

## 0. T2'de yapılan / ertelenen

| İş | T2 | Not |
|---|---|---|
| `program.toml` sözleşmesi + denetim | **evet** (PR-A3, PR-B1..B4) | denetleyici `--program <dosya>` modu |
| Veri alanı kitaplığı `taa-veri` (Rust) + `taa-veri.js` | **evet** (PR-A1, PR-A3) | SAHIP.toml düz alt küme |
| `rehber` sahipliği | **geçici sahip Tutaa**, sahip programı `taarehber` kurulunca devralır | E-posta rehberi ayrı program başlığında; T2 yalnız sözleşme + devralma kuralı |
| `etiket-kuyrugu` | sahip **Fiyat Etiketi** (plandaki "TaaRkod" düzeltildi: kuyruk gerçekte Fiyat Etiketi `kuyruk.json`'dadır; TaaRkod'da kuyruk yok) | TaaRkod okur / `oner` |
| `belgeler` | sahip TaaDön (`gecmis.json` → `belgeler/kayitlar.json`) | Tutaa okuma T3 |
| `urunler` | sahip Tutaa (plan); Tutaa yazmıyor → alan YOK; Fiyat Etiketi kendi listesiyle çalışır, alan doğarsa salt okunur okur | göç yok |
| zip paket (`taa-kur kur x.zip`) | **evet** (PR-A2), `zip` crate zaten taa-donustur'da var | imza alanı ayrılır, doğrulanmaz (imza T3, ed25519 onayı) |
| taa-katalog `remove_file` → `eski\` | **evet** (PR-A2) | varsayılan karar (Yusuf itiraz etmedi) |
| `taa-guncelle` / taa-yayin delta | **T3** | imza bağımlılığı kararı |
| Adlandırılmış boru + `taa-kimlik` | **T3** | bağımlılık kararı |
| `taa-kur` penceresi (GUI) | **T3** | GUI bağımlılığı kararı; T2'de `taa-kur tani --json` kabuklara veri verir |
| TaaTerziTakip Cargo features | **T3** | denetleyiciye `secimlik` alanı gerekir |

## 1. Veri alanı sözleşmesi (`docs/veri-alani.md`'de kesinleşir)

Kök: `TAA_VERI_KOK` ortam değişkeni → yoksa `%APPDATA%\TAA\veri` (Windows) / `$XDG_DATA_HOME|~/.local/share/TAA/veri`. **İstisna:** `rehber` alanı mevcut konumunda kalır: `%APPDATA%\TAA\rehber\` (rehber.json orada; başka başlığın kuralı). Alan adı `[a-z0-9-]`, ≤32.

```
<kok>/<alan>/SAHIP.toml           # sahip kaydı (düz TOML alt kümesi, aşağıda)
<kok>/<alan>/<veri dosyaları>     # yalnız sahip yazar, atomik (gecici + rename)
<kok>/<alan>/oneriler/<id>.json   # okuyanların değişiklik önerileri (okuyan yalnız buraya yazar)
<kok>/<alan>/eski/<ad>-<YYYYMMDD-HHMMSS>  # üzerine yazılan SAHIP.toml / taşınan dosyalar; silme yok
```
`SAHIP.toml` (düz alt küme: `anahtar = "metin"` | `anahtar = true|false` | `anahtar = 123`; tablo, dizi, çok satır yok; `#` yorum):
```toml
sema = 1
alan = "etiket-kuyrugu"
sahip = "fiyatetiketi"      # program adı (program.toml `ad`)
gecici = false              # true: geçici sahip; asıl sahip kurulunca devralır
yazan_surum = "1.33.0"
son_yazim = "2026-11-05T14:00:00Z"
```
Kurallar:
1. **Tek yazar.** Alan dosyalarını yalnız `sahip` yazar. Okuyan hiçbir zaman alan dosyası yazmaz; değişiklik isterse `oneriler/` altına `{"sema":1,"oneren":"taarkod","zaman":..,"degisiklik":{...}}` bırakır. Sahip uygular ya da `oneriler/eski/`ye taşır.
2. **Açılış:** sahip `ac_sahip(alan)` çağırır: SAHIP.toml yoksa yazar; varsa ve `sahip` kendisi ise günceller; varsa ve `sahip` başka program ise → `gecici = true` ise **devralır** (eski SAHIP.toml `eski/`ye), değilse **hata `BaskaSahip`** (program alanı salt okunur açar ve kullanıcıya söyler). İki asıl sahip CI'da yasak (sozlesme_denetle).
3. **Sahip yokken:** SAHIP.toml var ama sahip program kurulu değilse (taa-kur `kayit.json` ya da program yoksa) okuyanlar **son kopyayı salt okunur** kullanır; UI'da "sahip kurulu değil" notu.
4. **Değişiklik bildirimi:** sahip yazdıktan sonra `taa-ipc` `veri.degisti@1` olayı (`yuk: {"alan":..}`) yayınlar; IPC yoksa okuyan dosya zamanına bakar.
5. **Göç (yedekli):** program ilk açılışta eski konumdaki dosyayı alana **kopyalar**, eskisini `<eski ad>.tasindi` olarak **yeniden adlandırır** (silme yok). Alan yazılamıyorsa eski konumla çalışmaya devam eder ve günlüğe yazar. Göç bir kez; `.tasindi` varsa tekrarlanmaz.
6. Kişisel veri alanlarında (`rehber`, `kimlik`) dışa aktarma yetki ister; günlükte maskeleme (§15).

### 1.1 `crates/taa-veri` (Rust, program = ortak, gerektirir = taa-dosya)
API: `VeriKok::ortamdan()`, `VeriKok::yeni(yol)`, `kok.ac_sahip(alan, program, surum) -> Result<Sahip, VeriHata>`, `kok.ac_okur(alan, program) -> Result<Okur, VeriHata>`, `Sahip::yaz(dosya, &[u8])` (atomik, `..`/mutlak yol reddi), `Sahip::oneriler() -> Vec<Oneri>`, `Sahip::oneri_kapat(id)` (→ `oneriler/eski/`), `Okur::oku(dosya)`, `Okur::oner(Value) -> id`, `Okur::sahip() -> SahipKaydi`, `Okur::sahip_kurulu(&kayit_json_yolu) -> bool`, `sahip_toml_coz(&str) -> Result<BTreeMap<String, Deger>, VeriHata>` ve `sahip_toml_yaz`. Hatalar: `BaskaSahip{alan,sahip}`, `SahipYok`, `GecersizAd`, `GecersizToml(satir)`, `YolDisari`, `Io`. Testler geçici kökte: ilk açılış SAHIP.toml üretir; başka asıl sahip → `BaskaSahip`; `gecici` devralma eski dosyayı `eski/`ye koyar; okuyan yazamaz (API'de yazma yok, `oner` yalnız oneriler/); bozuk TOML satır numarasıyla hata; `..` yolu reddi; hiçbir `fs::remove_*` yok.

### 1.2 `araclar/js/taa-veri.js` (Node, bağımlılıksız, CommonJS, tek dosya ≤300 satır)
Aynı sözleşme: `veriKok(env)`, `acSahip({alan, program, surum, env})`, `acOkur({alan, program, env})`, `sahip.yaz(ad, metin)`, `okur.oku(ad)`, `okur.oner(obj)`, `sahipTomlCoz(str)`, `sahipTomlYaz(obj)`, `goc({eskiYol, alan, hedefAd, ...})` (kopyala + `.tasindi`). Başlık satırı: `// taa-veri.js v1 — kaynak: yusufbesir/taa-paket/araclar/js/taa-veri.js; programlar kopyalar, elle düzenlemez`. Test `araclar/js/test/taa-veri.test.js` (`node --test`). CI'a `node --test araclar/js/test` adımı (`actions/setup-node@v4`, Node 22).

## 2. `program.toml` (program depolarının kökünde) ve denetim
Alanlar `docs/program-sozlesmesi.md`'deki gibi; ek: `surum` program `package.json`/Cargo `version` ile aynı olmalı. Denetleyici `tools/sozlesme_denetle.py --program <program.toml> [--surum-dosyasi package.json|Cargo.toml] [--paket-kok <taa-paket>]`: zorunlu alanlar, `ad` `[a-z0-9-]`, `gerektirir` yalnız `taa-*`, `sunar`/`ister` biçimi, `surum` eşitliği, `veri_sahibi` alan adları `[a-z0-9-]`. `--paket-kok` verilirse `docs/veri-sahipleri.toml` (yeni, paket genelinde sahip tablosu: `[alanlar] etiket-kuyrugu = "fiyatetiketi"`, `belgeler = "taadon"`, `rehber = "taarehber"`, `urunler = "tutaa"`, `kimlik = "tutaa"`, `ayar-ortak = "taa-kur"`) ile çapraz denetim: `veri_sahibi` tabloya uymalı; `veri_okur` tabloda olmalı. Programlar denetleyiciyi **kopyalar** (`taa/sozlesme_denetle.py`, başlıkta kaynak sürümü) ve kendi CI'ında çalıştırır. Kesin `program.toml` değerleri:

| ad | gorunen_ad | surum | gerektirir | sunar | ister | veri_sahibi | veri_okur |
|---|---|---|---|---|---|---|---|
| tutaa | Tutaa | package.json (tutaa-paketi/masaustu) | {} (Electron; Rust yok) | `tutanak.olustur@1` | `rehber.ara@1` | `rehber` (**gecici = true** notu `[veri.rehber] gecici = true`) | — |
| taadon | TaaDön | package.json | { taa-donustur = "1" } | `belge.donustur@1` | — | `belgeler` | — |
| taarkod | TaaRkod | masaustu/package.json | {} | `kod.uret@1` | `etiket.kuyruga_ekle@1` | — | `etiket-kuyrugu` |
| fiyatetiketi | Fiyat Etiketi | package.json | {} (çekirdek kendi wasm'ı) | `etiket.yazdir@1`, `etiket.kuyruga_ekle@1` | — | `etiket-kuyrugu` | `urunler` |
`ayar_alani = "programlar.<ad>"`, `arayuz = "1"`. `sunar`/`ister` T2'de yalnız bildirim (IPC bağlanması T3); `ister` yoksa düğme gizli kuralı belgede.

## 3. Program depoları (PR-B1..B4; her biri kendi deposunda, dal `claude/tek-cati-t2`)
Ortak: kökte `program.toml`; `taa/taa-veri.js` ve `taa/sozlesme_denetle.py` kopyası (başlıkta kaynak commit); `npm test`e denetim + yeni testler; README'ye "TAA tek çatı" bölümü (3-5 satır); sürüm **yükseltilmez** (Yusuf sürüm/teslim kararı verir), DEGISIKLIK/notlara "Yayınlanmamış" maddesi. Mevcut testler bozulmaz. "Diğerleri kaldırılmış" testi: `TAA_VERI_KOK` boş geçici klasörken program açılış mantığı hatasız çalışır ve sahip olmadığı alanları salt okunur/boş görür.
- **B1 Fiyat Etiketi** (`fiyat-etiketi`, 1.32.0): `depo.js` `kuyruk` koleksiyonu → `etiket-kuyrugu` alanı sahibi: ilk açılışta `userData/kuyruk.json` → `<kok>/etiket-kuyrugu/kuyruk.json` göçü (`.tasindi`), sonra kuyruk okuma/yazma alandan (`sahip.yaz`, atomik); alan yazılamazsa userData'ya düşer (günlük). `oneriler/` okunur: TaaRkod önerisi (`{"ekle":[{barkod, adet}]}`) kullanıcı onayıyla kuyruğa eklenir (UI: kuyruk ekranında "Gelen öneriler (n)"; onay/ret, ret `oneriler/eski/`). `urunler` okuru: alan + SAHIP varsa listeyi salt okunur gösterir, yoksa mevcut davranış. Testler: göç, atomik yazım, öneri onayı, alan yokken eski davranış.
- **B2 TaaRkod** (`taarkod/masaustu`, 1.8.0): `etiket-kuyrugu` okuru; "Fiyat Etiketi kuyruğuna gönder" düğmesi yalnız alan + SAHIP varsa görünür (`ister` kuralı), `okur.oner({ekle:[...]})` yazar; alan yoksa düğme gizli. Testler: düğme görünürlüğü, öneri dosyası biçimi, alan yokken hata yok.
- **B3 TaaDön** (`dosya-donusturucu`, 1.12.0): `belgeler` sahibi: `userData/gecmis.json` → `<kok>/belgeler/kayitlar.json` göçü; geçmiş yazımı alandan; alan yazılamazsa eski yol. Testler: göç, yazım, alan yokken.
- **B4 Tutaa** (`tutaa/tutaa-paketi/masaustu`, aktif ağaç; `masaustu/` eski kopyaya dokunulmaz): `rehber` alanı **geçici sahip**: `rehber-kaynak.js` yazarken `%APPDATA%\TAA\rehber\SAHIP.toml` yoksa `gecici = true` ile yazar; `sahip` başka program (`taarehber`) ve `gecici=false` ise Tutaa rehberi **salt okunur** açar (düzenleme düğmeleri kapalı, "Rehber artık <program> tarafından yönetiliyor"). Yazma biçimi değişmez (rehber.json şeması aynı). Testler: SAHIP yokken geçici sahip yazılır; başka sahipte salt okunur; rehber.json içerik değişmez.

## 4. taa-paket PR'ları
- **PR-A1** (`claude/tek-cati-t2`): bu belge, `docs/veri-alani.md`, `docs/veri-sahipleri.toml`, `crates/taa-veri` (+modul.toml, README tablosu, DEGISIKLIK), `docs/TEK-CATI.md` T2 notu, plan tablosu düzeltmesi (etiket-kuyrugu sahibi).
- **PR-A2** (`claude/tek-cati-t2-kur`): taa-kur `kur <x.zip>` (zip → geçici klasöre aç → mevcut klasör kurulumu; `zip` crate `default-features=false, deflate-flate2-zlib-rs` **aynı sürüm** taa-donustur ile; `paket.json` zipin kökünde; zip bombası sınırı: açılmış toplam ≤2 GiB, dosya ≤10.000, yol kuralları aynı), `paket.json`'a `imza` alanı (ayrılmış, `null`; Rust yok sayar; `tools/paket_uret.py --zip` zip üretir), `tani --json` çıktısına veri alanları (`TAA_VERI_KOK` taranır: alan, sahip, gecici, sahip kurulu mu); taa-katalog `remove_file` → `eski/<epoch>-<ad>` taşıma (test: eski dosya `eski/` altında, hiçbir `remove_file` yok). DEGISIKLIK.
- **PR-A3** (`claude/tek-cati-t2-denetim`): `tools/sozlesme_denetle.py --program` modu + `veri-sahipleri.toml` çapraz denetimi + testler; `docs/program-sozlesmesi.md` kesin değerler (§2 tablosu); `araclar/js/taa-veri.js` + test + CI adımı.
Sıra: A1 ve A3 birbirinden bağımsız, A2 bağımsız; B1..B4 **A1+A3 birleştikten sonra** (JS modülü ve denetleyici kopyalanır). Çakışma: üç A PR'ı DEGISIKLIK.md'ye ayrı başlık satırı ekler (`### T2-A1` vb.); README tablosunda yalnız A1 satır ekler.

## 5. Testler ve kanıt
- taa-paket: `cargo test --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`, `python3 tools/sozlesme_denetle.py`, `python3 -m unittest discover tools`, `node --test araclar/js/test`.
- Programlar: mevcut `npm test` + yeni testler; denetleyici `python3 taa/sozlesme_denetle.py --program program.toml --surum-dosyasi package.json` CI'da (programda CI yoksa `npm test` içine `node taa/denetim.js` eklenmez; yalnız yerelde çalıştırılır ve PR'da sonuç yazılır).
- Güvenlik incelemesi (Opus): yol kaçışı (alan/dosya adları), zip açma sınırları, SAHIP devralma kötüye kullanımı (sahte `gecici`), öneri dosyası enjeksiyonu (JSON doğrulama, boyut ≤256 KiB), kişisel veri (rehber) günlükte maskeleme.

## 5.1 Güvenlik incelemesi sonucu (Opus, 2026-11-05)
0 yüksek, 9 orta, 7 düşük. Orta bulgular T2-A4 (taa-paket) ve program PR'larında düzeltildi: `taa-veri.js` v2 (wx+fsync, öneri doğrulaması, ayrılmış adlar), Rust `create_new`/aygıt adları/zip casefold, Fiyat Etiketi `.bak` göçü ve bozuk alan dosyasının `eski/`ye alınması, TaaDön salt okunur kipte userData'ya yazma, Tutaa eski rehber konumundan göç. T3'e kalan: sahip tablosunun kitaplığa gömülmesi, alan içi sembolik bağ denetimi, ortak SAHIP.toml test kümesi, TaaRkod↔Fiyat Etiketi öneri sınırlarının tek yerde tanımı, `eski/` boyut uyarısı.

## 6. Yusuf'a açık sorular (varsayılanla ilerlendi)
1. `etiket-kuyrugu` sahibi Fiyat Etiketi (plan TaaRkod diyordu). 2. `rehber` geçici sahip Tutaa; `taarehber` programı devralır. 3. T3 için bağımlılıklar: `toml`, `interprocess`/`windows-sys`, `ed25519-dalek`, GUI (egui?) — onay gelmeden boru/imza/pencere yapılmaz. 4. Program sürümleri yükseltilmedi; teslim kararı sizde.
