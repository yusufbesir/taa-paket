<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T3 2026-11-05] değiştiren: [Claude#kurucu-arayuz 2026-11-17] (Görünüm ve temalar bölümü) -->
# T3: taa-kur penceresi ("bu program ne?")

`taa-kur-pencere` (Windows): kurulu paketleri gösterir, Kur / Kaldır / Güncelle düğmeleri verir. Komut satırı `taa-kur` değişmedi; pencere aynı `taa_kur::Kur` kitaplığını çağırır.

## Ekran düzeni
```
+--------------------------------------------------------------------+
| TAA Kur — paketler                                          [_][□][x]|
+--------------------------------------------------------------------+
| Paket        | Kurulu sürüm  | Seçili sürüm | Durum                 |
|--------------+---------------+--------------+-----------------------|
| taa-ayar     | 0.1.0         | 0.1.0        | Hazır                 |
| taa-ornek    | 0.1.0, 0.2.0  | 0.1.0        | Daha yeni sürüm kurulu|
|              |               |              |  (0.2.0), seçili değil|
+--------------------------------------------------------------------+
| [ Kur... ]  [ Kaldır ]  [ Güncelle ]  [ Rapor ]                    |
| Kuruldu: taa-ornek 0.2.0 (seçili değil).   [Kök: D:\Programlar\TAA]|
+--------------------------------------------------------------------+
```
Durum metinleri: "Hazır", "Hazır (sabit)", "Seçili sürüm yok", "Kurulu değil", "Daha yeni sürüm kurulu (X), seçili değil".

## Düğme kuralları
| Düğme | Etkin olduğu durum | İş |
|---|---|---|
| Kur... | her zaman | dosya seçici (`paket.zip` ya da klasör paketinin `paket.json` dosyası) -> `Kur::kur(yol, false)` |
| Kaldır | seçili satırın seçili sürümü varsa | onay sorusu -> `Kur::kaldir(ad, None)`; içerik silinmez, `eski\` klasörüne taşınır; veri alanlarına dokunulmaz |
| Rapor | her zaman | soru yok (salt okunur). `PencereModel::rapor` → `Kur::rapor` (taa-rapor, veri kökü `TAA_VERI_KOK`/`%APPDATA%\TAA\veri`) metni bilgi kutusunda gösterir; CSV `kök\raporlar\rapor-<damga>.csv` yazılır, yolu durum satırında. T5, docs/T5-RAPOR.md |
| Güncelle | bir satır seçiliyse | onay sorusu → `GuncelleyiciArayuzu::guncelle`. `KomutGuncelleyici`: `taa-guncelle uygula --json --paket <ad>` (TAA_KOK kök; konsol penceresi açılmaz); komut exe'nin yanında ya da depodaki seçili `taa-guncelle` paketinde aranır. Bulunamazsa `KaynaksizGuncelleyici`: "Güncelleme kaynağı ayarlanmadı" |

Durum satırı her işlemin sonucunu (ya da hata + öneriyi) Türkçe tek satır yazar; hata varsa kayıt değişmez.

## Mimari
- `src/pencere_model.rs`: platformdan bağımsız (komut -> `Kur`, satır modeli, düğme kuralları, metinler, `GuncelleyiciArayuzu`). Linux'ta birim testli (geçici kök, gerçek `Kur`).
- `src/win.rs` (`#[cfg(windows)]`): ince Win32 katmanı, `windows-sys` doğrudan (ListView, düğmeler, `GetOpenFileNameW`, `MessageBoxW`). Yeni GUI bağımlılığı yok.
- `src/pencere_main.rs`: bin `taa-kur-pencere`; Windows dışında "Bu pencere yalnız Windows'ta çalışır" deyip 1 ile çıkar. `main.rs` değişmedi.
- Güncelleyici: `win::calistir` `KomutGuncelleyici::bul` ile taa-guncelle komutunu arar; JSON çıktısı `guncelle_ciktisi_yorumla` ile tek satır Türkçe mesaja çevrilir (güncellendi / güncel / atlandı / reddedildi / hata + önceki sürüm seçili). Kaynak `kur.toml` (`kaynak_url` ya da `kaynak_klasoru`) ile ayarlanır.

## Sınırlar
- Windows'ta elle doğrulanmadı; yalnız `cargo check/clippy --target x86_64-pc-windows-gnu` tür denetimi yapıldı.
- İşlemler arayüz iş parçacığında eşzamanlı (büyük zip kurulurken pencere kısa süre yanıtsız kalabilir).
- Klasör seçici yok; klasör paketi için `paket.json` seçilir.

## Görünüm ve temalar
<!-- yazan: [Claude#kurucu-arayuz 2026-11-17] -->
Pencere TAA Kurucu'nun bağlayıcı tema kararını (arayuz-modernlesme/taa-kurucu/ekip/KARAR-FABLE.md) uygular. Renkler, ölçüler, yazılar ve logo `crates/taa-kur-tema` crate'inden gelir (docs/TEMA.md). Yukarıdaki "Ekran düzeni" krokisi ilk sürümün düzenidir; düğme kuralları, iş mantığı, onay soruları ve durum metinleri değişmedi.

Başlık satırında "Geçme Temel" logosu (GDI `Polygon`, ana parça `metin`, vurgu parçası `vurgu` rengi) ve "TAA Kurucu · Paketler" yazısı vardır. Pencere anında açılır; Kurucu'daki 1,2 sn'lik markalı yükleme ekranı burada yok, logo yeter.

### Tema başına yerleşim
| | İlmek (varsayılan) | Kalıp | Mezura |
|---|---|---|---|
| Eylemler | Üst araç çubuğu: `Kur…` vurgu dolgulu, `Kaldır`, `Güncelle`, `Rapor` düz | Tablonun altında işlem çubuğu; `Rapor` sağda | Büyük `Paketler` başlığının sağında `Kur…`; `Kaldır` / `Güncelle` / `Rapor` tablonun altında |
| Ayarlar | Başlıktaki `Ayarlar` düğmesi (Ayarlar ekranında `Paketlere dön`) | Üstte `Paketler | Ayarlar` sekmeleri | Başlıktaki `Ayarlar` düğmesi |
| Kök klasör | Tablonun altında: `Kök klasör: …` + `Kopyala` | Tablodan önce çerçeveli bant | Eylemlerin altında |
| Tablo | Yuvarlak köşeli yüzey kartı, satır arası 1 px teyel | 1 px kenar çerçeve | Üstte ve altta 2 px çizgi |
| Seçili satır | `yuzey2` + kalın yazı | Solda 3 px vurgu şeridi | `yuzey2` + solda 6 px sarı-siyah şerit + `▸` |
| Satır / düğme | 48 / 44 px | 40 / 36 px | 52 / 48 px |

Durum sütunu her zaman işaret + metindir: `✓ Hazır`, `✓ Hazır (sabit)`, `! Seçili sürüm yok`, `! Daha yeni sürüm kurulu (X), seçili değil`, `○ Kurulu değil`. İşaret `basari` / `uyari` / `soluk` renginde, metin `metin` renginde. Paket adı sığmazsa "…" ile kısalır; sürümler ve durum kesilmez, iki satıra sarılır. Kök klasör yolu ortadan kısalabilir; tam yol `Kopyala` ile panoya alınır.

Etiketler kesilmez: her düğme metni GDI ile ölçülür, genişlik = metin + 2 x iç boşluk (en az 96 px); satıra sığmayan düğme alt satıra, tek başına sığmayan etiket iki satıra iner. Klavye odağı her temada 2 px görünür (Mezura'da boşluklu çift çerçeve; Mezura açıkta sarı yerine `kenar`).

### Ayarlar ekranı
Tema (İlmek / Kalıp / Mezura, her biri canlı GDI önizlemeli), görünüm modu (Açık / Koyu / Sistemi izle), vurgu rengi (`Renk seç…` Windows renk seçicisi, `Temanın rengine dön`), `Hareketi azalt`, `Tanıtımı tekrar göster`, `Varsayılana dön`.

- Seçim anında uygulanır ve `%APPDATA%\TAA\veri\kurucu\gorunum.json` dosyasına atomik yazılır (`TAA_VERI_KOK` varsa onun altına). Kurucu ile aynı dosyadır. Açık TAA pencereleri kayıtlı `TAA_KUR_GORUNUM_DEGISTI` iletisiyle haberdar edilir ve dosyayı yeniden okur.
- Vurgu rengi kontrast korumasından geçer (`vurgu_duzelt`): gerekirse koyulaştırılır (koyu modda açılır) ve not gösterilir; hata rengine çok yakınsa uyarı çıkar; düzeltilemezse reddedilir, temanın rengi kalır. Renk tema başına ayrı saklanır.
- `Tanıtımı tekrar göster` bu pencerede yalnız tercihi sıfırlar (tanıtım TAA Kurucu'nun bir sonraki açılışında gösterilir).
- `Varsayılana dön` onay sorar; yalnız görünüm tercihlerini sıfırlar, paketlere dokunmaz.
- Varsayılanlar: tema İlmek, mod Açık. Kullanıcı tema seçmemişse ve Windows Yüksek Karşıtlık açıksa tema Mezura olur (Ayarlar'da notu çıkar). `Hareketi azalt` seçilmemişse Windows "animasyonları göster" ayarının tersidir.
- `WM_SETTINGCHANGE` / `WM_THEMECHANGED` / `WM_SYSCOLORCHANGE` geldiğinde sistem koyu modu, Yüksek Karşıtlık ve animasyon ayarı yeniden okunur ("Sistemi izle" anında değişir). Koyu modda başlık çubuğu ve kaydırma çubukları da koyudur.

### Çizim ve hareket
- Pencere arka planı, başlık, bantlar ve metinler bellek DC'sine çizilip tek `BitBlt` ile basılır; düğmeler sahip-çizimli `BUTTON` (Tab, Boşluk, ekran okuyucu adı korunur), her biri kendi bellek DC'sinde çizilir. Liste gerçek ListView'dır: satırlar ve başlık `NM_CUSTOMDRAW` ile çizilir, `LVS_EX_DOUBLEBUFFER` açıktır, satır yüksekliği tema ölçüsündedir. Gradyan, gölge, bitmap yok.
- Hareketler 120 ms tikli: İlmek 2 adımlı renk geçişi (yalnız dolgular harmanlanır), Kalıp ekran değişiminde 8 -> 3 -> 0 px oturma, Mezura anlık (yalnız büyük başlığın 4 px alt çizgisi 2 adımda uzar). Hareketi azalt açıkken hepsi anlık.
- DPI: `SetProcessDPIAware` + `GetDpiForWindow` + `WM_DPICHANGED`; hedef 1040x700, en az 860x600 (mantıksal piksel). Ayarlar ekranı dar pencereye sığmazsa önizlemeler küçülür, Hareket ve Tanıtım tek satıra iner.

### Mimari ekleri
- `src/pencere_tema.rs` (platformdan bağımsız, birim testli, `src/testler_pencere_tema.rs`): tema -> yerleşim kararı, ekran düzeni (`duzen`, `Olcer` ile ölçülen metinler), durum işaretleri, seçili satır biçimi, geçiş kareleri, vurgu seçimi notları, sütun genişlikleri.
- `PencereModel::kok_yolu` ve `PencereModel::durum_ata` eklendi (Kopyala ve görünüm mesajları için); Kur / Kaldır / Güncelle / Rapor kodu değişmedi.
- Yeni dış bağımlılık yok: `taa-kur-tema` aynı çalışma alanında. windows-sys'e yalnız özellik eklendi (Dwm, DataExchange, Memory, Ole, Registry, Accessibility, HiDpi).
- Sınır: bu bölümdeki her şey de Windows'ta elle doğrulanmadı; yalnız `cargo check/clippy --target x86_64-pc-windows-gnu` yapıldı.
