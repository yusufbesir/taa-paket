<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T1 2026-11-05] (Fable, mimar). T1 tasarımı: taa-kur ve taa-ipc küçük sürüm. -->
# T1 tasarımı: `taa-kur` (kurulum yöneticisi) ve `taa-ipc` (programlar arası iletişim)

Kaynak: PLAN-TAA-PAKET.md §18, docs/TEK-CATI.md. Kural: yeni Cargo bağımlılığı YOK (mevcut: serde, serde_json, sha2, zip, flate2; `toml`, `interprocess`, `ed25519-dalek` Yusuf onayı bekler). Dil Rust; Python yalnız `tools/`. Hiçbir şey silinmez (`eski\`ye taşınır). Türkçe adlar ve mesajlar.

## 1. `taa-kur` (crate: lib + bin), program = "ortak", gerektirir = taa-ayar, taa-dosya
### 1.1 Kök ve düzen
Kök sırası: `TAA_KOK` ortam değişkeni → `kayit.json` içindeki `kok` → varsayılan (Windows `D:\Programlar\TAA`, diğer `~/.local/share/TAA`). Testler geçici kök kullanır. Kodda sabit yol yalnız bu varsayılandır.
```
<kok>/kayit.json            # TEK dosya: {"sema":1,"kok":...,"paketler":{ad:{"kurulu":[surum..],"secili":surum|null,"sabitle":bool}},"veri_sahipleri":{alan:ad},"ozet":"<sha256>"}
                            # ozet = ozet alanı BOŞ sayılarak kanonik (pretty) JSON'un SHA256'sı; dogrula iç özeti denetler.
                            # Eski kayit.json.sha256 dosyası artık yazılmaz ve okunmaz (varsa silinmez, yok sayılır).
<kok>/kilit                 # süreçler arası kilit: create_new ile alınır, içinde pid + zaman (UNIX sn); kur/kaldir/sec/geri-don/sabitle kilit altında
<kok>/kilit.serbest         # iş bitince kilit buraya rename edilir (silme yok; sonraki rename üzerine yazar)
<kok>/depo/<ad>/<surum>/    # paket içeriği (paket.json + dosyalar)
<kok>/programlar/<ad>/secili.json   # {"ad","surum","sha256","tarih"}
<kok>/eski/<ad>-<surum>-<YYYYMMDD-HHMMSS>/   # kaldırılan ya da üzerine yazılan içerik; silme yok
<kok>/eski/kilit-<YYYYMMDD-HHMMSS>           # bayat (>10 dk) kilit buraya taşınır, kilit yeniden alınır
<kok>/eski/yarim-<...>/                      # boyut sınırını aşıp yarım kalan kopya
```
Kilit alınamazsa (taze kilit var) `Kilitli` hatası, çıkış kodu **6**. `kaldir` taşıma (rename) yapamayıp kopyaya düşerse sürüm kayıttan düşmez: "taşınamadı, kayıt değişmedi" hatası. Kopya yolunda toplam boyut 2 GiB'ı aşarsa durulur, yarım klasör `eski/yarim-...` altına taşınır.
Ad: yalnız `[a-z0-9.+-]`, ≤64, baş/son `.`/`-` yok, `..` yok, Windows ayrılmış adları (con, prn, aux, nul, com1-9, lpt1-9) yok; sürüm aynı kurallarla rakamla başlar. Paket yolunda 0x20 altı denetim karakterleri ve 0x7F reddedilir. `tools/paket_uret.py` aynı kuralları uygular (uyumsuzluk çıkış 2) ve SHA256'yı kopyalanan dosyadan hesaplar.
Tüm yazımlar `taa_dosya::uzerine_yaz_atomik`. Veri alanları (`%APPDATA%\TAA\veri`) bu crate tarafından ASLA dokunulmaz.

### 1.2 Paket biçimi (T1: klasör; zip T2)
Paket = `paket.json` içeren klasör. `paket.json` (modul.toml'dan `tools/paket_uret.py` üretir):
```json
{"sema":1,"ad":"taa-rehber","surum":"0.1.0","arayuz":"1","program":"taarehber",
 "gerektirir":{"taa-ayar":"1","taa-nebim":"1"},"sunar":["rehber.ara@1"],"ister":[],
 "veri_sahibi":["rehber"],"veri_okur":[],
 "dosyalar":[{"yol":"bin/taa-rehber.exe","sha256":"..."}],"icerik_sha256":"..."}
```
`icerik_sha256` = dosyalar listesinin (yol+sha256, yol sırasıyla) SHA256'sı. **Kesin tanım (paket_uret.py ile Rust aynı):** dosyalar `yol`un UTF-8 bayt sırasıyla dizilir; her dosya için `"{yol}\n{sha256}\n"` (UTF-8, sha256 küçük harf onaltılık) art arda eklenir; birleşimin SHA256'sı onaltılık küçük harf `icerik_sha256` olur (boş liste = boş girdinin SHA256'sı). Bu tanım `paket.json`'a `icerik_sha256_tanim` metin alanı olarak da yazılır (bilgi amaçlı; Rust okurken yok sayar). `yol` göreli olmalı; `..`, mutlak yol, sürücü harfi, `\0` reddedilir. Dosya sayısı ≤ 10.000, toplam ≤ 2 GB.

### 1.3 Çözücü
`gerektirir` değeri arayüz ana sürümüdür ("1"). Kurulumda her gerekli modül için: kurulu ve seçili bir sürüm var mı ve `arayuz` eşit mi. Yoksa kurulum YAPILMAZ; mesaj: hangi modül, hangi arayüz, "önce `taa-kur kur <x>` çalıştırın". Aynı modülün birden çok sürümü depoda durabilir; `secili` tek. `sabitle=true` ise `sec` kullanıcı açıkça istemeden değişmez. İnce çizgi: `gerektirir` içinde `program` alanı `ortak` olmayan ve farklı programa ait paket varsa kurulum reddedilir (sozlesme_denetle ile aynı kural).

### 1.4 Komutlar (bin `taa-kur`, std::env::args ile; clap yok)
| Komut | İş | Çıkış |
|---|---|---|
| `liste [--json]` | paketler, kurulu sürümler, seçili, sabit | 0 |
| `kur <paket-klasoru> [--sec]` | doğrula (yol, sha256, çözücü) → `depo/<ad>/<surum>` kopyala (varsa önce `eski\`ye taşı) → kayıt; `--sec` ile seçili yap (ilk kurulumda otomatik seçili) | 0 / 2 doğrulama / 3 çözüm yok |
| `kaldir <ad> [--surum S]` | seçili sürümü (ya da S) `eski\`ye taşı, kayıttan düş; başka seçili sürüm varsa ona döner; bu paketin `veri_sahibi` alanlarını okuyan paketler varsa "salt okunur olacak" listesi yazılır; veri alanına dokunulmaz | 0 |
| `sec <ad> <surum>` | çözücü uyumlu ise `secili` değiştir (eski seçim kaybolmaz) | 0 / 3 |
| `geri-don <ad>` | önceki seçili sürüme dön (kayıt `onceki` tutar) | 0 / 4 yok |
| `dogrula [ad]` | depo içeriği SHA256 ↔ paket.json; kayit.json iç özeti (`ozet`); bozuksa liste, çıkış 5 | 0 / 5 |
| `tani` | kök, sürümler, sahiplik tablosu, son hatalar; sır yok | 0 |
Mesajlar Türkçe, tek satır neden + öneri. Günlük `<kok>/gunluk.jsonl` (eklemeli, `taa_dosya::satir_ekle`).

### 1.5 Testler (zorunlu)
Geçici kökte: kur→liste→sec→geri-don→kaldir senaryosu; ikinci sürüm yan yana; çözüm yok reddi; sha256 uyuşmazsa hiçbir şey değişmez (kayıt ve depo aynı kalır); `..` yolu reddi; kaldırmada eski\ içeriği birebir; veri alanı klasörüne hiç dokunulmadığı (sahte `veri/` klasörü değişmez); kayit.json atomik (yarım dosya yok). `cargo test -p taa-kur`.

## 2. `taa-ipc` (crate: lib), program = "ortak", gerektirir = taa-dosya
### 2.1 Taşıyıcı soyutlaması
`trait Tasiyici { fn kayit_ol(&self, program:&str, yetenekler:&[String]) ; fn yetenek_bul(&self, yetenek:&str)->Result<Option<String>,IpcHata> /* birden çok sunan: Belirsiz */; fn gonder(&self, zarf:&Zarf)->Result<String,IpcHata> /* dosya varsa IdCakismasi */; fn al(&self, program:&str)->Vec<Zarf>; fn yanitla(&self, ...); fn olay_yayinla(&self, ad:&str, yuk:Value); fn olaylar(&self, program:&str, son:&str)->Vec<Zarf> }`
T1 uygulaması: **`GelenKutusu`** (dosya tabanlı, §4 yedek yol). Adlandırılmış boru (`Boru`) T2'de aynı trait'le (bağımlılık onayı sonrası). Kök: `TAA_IPC_KOK` → varsayılan `%APPDATA%\TAA\ipc` (testler geçici).
```
<kok>/kayit/<program>.json         # {"program","yetenekler":[..],"pid","zaman"}  (atomik)
<kok>/<program>/gelen/<id>.json    # istek zarfı
<kok>/<program>/yanit/<id>.json    # yanıt zarfı (isteği atanın program klasöründe)
<kok>/olaylar/<YYYYMMDD>.jsonl     # eklemeli olay günlüğü; aboneler son okudukları id'yi tutar
```
### 2.2 Zarf
`{"v":1,"id":"<pid>-<sayac>-<nanos>","kaynak":"taarkod","hedef":"tutaa","tur":"istek|yanit|olay","yetenek":"veri.urunler@1","yuk":{...},"zaman":"RFC3339","yanit_id":null}`
Sınırlar: zarf ≤ 1 MiB (büyükse hata, dosya yazılmaz); `kaynak`/`hedef`/`yetenek` yalnız `[a-z0-9.@-]`; bozuk JSON dosyası atlanır ve günlüğe yazılır, okuma durmaz. Yetenek yoksa `gonder` `YetenekYok` hatası döner (kabuk düğmeyi gizler). Yanıtı alınan istek dosyası `islenen/` altına taşınır (silinmez).
### 2.3 Testler
Aynı geçici kökte iki sanal program: A `kod.uret@1` sunar; B `yetenek_bul` A'yı bulur; B istek yazar, A `al` ile okur, `yanitla`; B yanıtı alır; `veri.degisti@1` olayı yayınlanır ve abone alır; 2 MiB zarf reddedilir; bozuk dosya okumayı durdurmaz; yetenek yoksa hata. `cargo test -p taa-ipc`.

## 3. Ortak işler
- `tools/paket_uret.py <crate-klasoru> <cikti-klasoru> [--bin <exe>]`: modul.toml + Cargo version → paket.json (sha256 listesi, icerik_sha256). stdlib. Test: `tools/test_paket_uret.py`.
- İki crate'e `modul.toml` (yeni alanlar), README tablosu "var", DEGISIKLIK "Yayınlanmamış"a T1 maddeleri, `docs/sozlesmeler/` içine `kur.*`/`ipc.*` yok; yetenekler programlara aittir, taa-ipc yalnız taşır.
- CI mevcut (`cargo test --workspace`, sozlesme). `cargo clippy -D warnings` ve `cargo fmt` temiz.
- Güvenlik: yol kaçışı, boyut sınırı, sembolik bağ (kopyalamada izlenmez), kayıt bütünlüğü; sır yok.

## 4. Yusuf'a sorular (varsayılanla ilerlendi)
1. `toml` crate eklensin mi? (Şimdilik Python üretir, Rust JSON okur.) 2. Adlandırılmış boru için `interprocess` ya da `windows-sys` doğrudan bağımlılık? (T2) 3. `ed25519-dalek` imza için? (T2)


## 5. Uygulama notu (T1 sonu)
Güvenlik incelemesi sonrası: taa-ipc program adı noktasız `[a-z0-9-]`, `KayitDolu`/`Belirsiz`/`IdCakismasi` hataları, yanıt kaynağı denetimi, olaylar 7 gün ve 10.000 satır sınırı, bozuk zarf `islenen/bozuk/`. taa-kur: `<kok>/kilit` (çıkış 6), `kayit.json` tek dosya (`ozet`), küçük harf ad/sürüm + Windows ayrılmış adlar, kopyada boyut sınırı. Dosya tabanlı taşıyıcı aynı kullanıcı süreçlerine karşı kimlik doğrulamaz (docs/ipc.md "Güvenlik sınırı"); T2 Boru + taa-kimlik.
