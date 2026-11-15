<!-- Sahip: Yusuf. yazan: [Claude#kur-ekrani 2026-11-15] değiştiren: [Claude#kur-yerel-pencere 2026-11-15] -->
# Kurulum ekranı (TAA Kur kurucusu)

Yusuf'un istekleri:
- "Kurulum yapılırken yükleme arayüzü, bekleme kısımlarında programlardan bahsetsin, notlar ve örnek şablonlar gözüksün,
  minimalist, göze hoş."
- "Kurucu brave üzerinde çalışmasın, kendi ekranı olsun. Herhangi bir tarayıcıda çalışmasını istemiyorum."
- "Diğer şubelerde kurulum basit olmalı, kurulum yapılırken adım adım ayarların nasıl yapılacağı direktifleri verilmeli,
  ilgili yönlendirmeler otomatik yapılmalı."
- "Bu Taa Kur programı için masaüstü kurulum ekranı hazırla, oradan çalıştır."

## Yol: kurucunun kendi yerel penceresi (tarayıcı YOK)
`kurulum/` (TAA-Kur-Kurulum.exe) etkileşimli kurulumda kendi Win32 penceresini açar; çizim GDI ile, çift tamponlu.
Tarayıcı açılmaz, yerel HTTP sunucusu yoktur (ilk sürümdeki `arayuz.rs` / `arayuz.html` kaldırıldı).
Tek bağımlılık `windows-sys` 0.61 (yalnız Windows hedefinde; Yusuf'un onaylı bağımlılığı, taa-kur ve taa-ipc ile aynı sürüm).
Özellikler: taa-kur penceresinin kümesi + `Win32_System_Registry` (tema), `Win32_UI_HiDpi` (DPI), `Win32_Graphics_Dwm`
(koyu başlık çubuğu), `Win32_System_Console` (çift tıklamayla açılınca konsolu gizleme). Yeni crate yoktur.

- Kurulum işi ayrı iş parçacığında çalışır ve ortak durumu (`ekran::Paylasim`) günceller; pencere 120 ms'de bir anlık
  kopyayı çizer, tuş ve tıklamaları komut olarak gönderir (`ekran::Komut`).
- Varsayılan: Windows'ta etkileşimli kurulumda pencere açık. `--sessiz` ve `--arayuzsuz` iken pencere YOK (konsol);
  sessizde soru da yoktur (`--hedef`, `--rol`, `--sube`, `--programlar`, `--programsiz`, `--kisayolsuz`).
- Pencere açılamazsa uyarı yazılır ve kurulum konsolda sürer (pencere kurulumu hiçbir zaman durdurmaz).
- Kurucu çift tıklamayla açıldıysa (konsol yalnız onun) konsol penceresi gizlenir; terminalden çalıştırılınca dokunulmaz.

## Akış (şubede: Enter, Enter, şube, Enter)
1. **Onay:** "TAA Kur kurulsun mu?" Kurulum klasörü gösterilir (`--hedef` yoksa varsayılan; pencerede klasör sorulmaz).
   Kur (Enter) / Vazgeç (Esc).
2. **Çekirdek dosyalar:** taa-kur, taa-kur-pencere, taa-guncelle, kur.toml (yoksa), anahtarlar, kaldir.exe.
   **Masaüstü ve Başlat menüsü "TAA Kur" kısayolu** (hedef `taa-kur-pencere.exe`) burada oluşturulur; `--kisayolsuz` ile
   oluşturulmaz. Kaldırma kısayolları da siler (manifestteki `@` satırları).
3. **Otomatik ayarlar** (var olan dosya ezilmez): kurulum programının yanında şunlar varsa yerine kopyalanır:
   `sunucu-yollari.json` → `<veri kökü>\yol\`, `sunucu-ca.pem` → `<veri kökü>\anahtarlar\`,
   `tailscale.anahtar` → `<kök>\gizli\`. Veri kökü taa-yol / taa-cihaz ile aynı kuraldır (TAA_VERI_KOK, yoksa
   `%APPDATA%\TAA\veri`). Anahtar içeriği hiçbir yere yazdırılmaz.
4. **Rol:** Şube PC / Sunucu PC (`--rol`; kayıtlı `rol.txt` varsa sorulmaz).
5. **Şube** (yalnız Şube PC): gömülü 15 şubeden seçim (`--sube <kod>`; kayıtlı `sube.txt` varsa sorulmaz). "Sonra" denirse
   adım direktif olarak kalır. Seçilen kod `<kök>\sube.txt`'ye yazılır (kullanıcı dosyası sayılır: kurulum ezmez, kaldırma silmez).
6. **Programlar:** rolde zorunlu olanlar kendiliğinden; isteğe bağlılar gruplu çoklu seçim (Boşluk / tıklama, Enter = Kur).
7. **Bitti:** hata varsa Yeniden dene (Enter); Bitir ya da **TAA Kur'u aç** (taa-kur-pencere.exe'yi açıp kapanır).

Şube listesi `kurulum/subeler.txt` (kod, ad, il; Tutaa şube listesinden, terziademaltun.com iletişim sayfası 2026-11-05;
kamuya açık işletme bilgisi, telefon/adres/müşteri verisi gömülmez). Yeni şube: satır eklenir, kod değişmez.

## Adım adım ayar direktifleri (sağ sütun)
`direktif.rs` bilinenlere (rol.txt, sube.txt, kur.toml `kaynak_url`, kopyalanan yan dosyalar, kurulan paketler) göre en çok
altı numaralı adım üretir. Yapılanlar işaretli (soluk), yapılacaklar numaralı, bilgi satırları "i" ile gösterilir.
Kurulum boyunca güncellenir; konsolda da sonda yazılır.

| Şube PC | Sunucu PC |
|---|---|
| 1. Rol | 1. Rol |
| 2. Şube (sube.txt) | 2. Sunucu paketleri (kuruluyor / kuruldu / kurulamayanlar) |
| 3. Güncelleme kaynağı (kur.toml `kaynak_url`) | 3. Güncelleme kaynağı |
| 4. TAA Sunucu adresi (`sunucu-yollari.json` kopyalandı ya da `taa-yol ekle …`) | 4. Şubelere sunucu adresini verin (`taa-yol ekle`, dosyayı şube kurulumunun yanına koyun) |
| 5. Cihaz onayı (`taa-cihaz uret`, `taa-cihaz kayit --sube <kod>`; merkez TAA Çatı'dan onaylar) | 5. Şube cihazlarını onaylayın (TAA Çatı) |
| 6. Tailscale (kurulu / anahtar hazır / `tailscale.anahtar` gerekli) | 6. VPN (bilgi: WireGuard ayrı iş) |

Cihaz onayı her zaman elle yapılır (merkezdeki yönetici onaylar); kurucu yalnız komutu şube koduyla hazır yazar.
Merkezin şube PC'leri için yapacağı tek hazırlık: `sunucu-yollari.json` (gerekirse `sunucu-ca.pem`, `tailscale.anahtar`)
dosyalarını kurulum programıyla aynı klasöre koymak; şubede kalan tek elle adım cihaz onayıdır.

## Aşamalar ve adımlar
Aşamalar: **Hazırlık → Dosyalar → Seçim → Programlar → Bitti**. Her program için dört adım:

| Adım | Ne olur | Ağırlık |
|---|---|---|
| İndir | `yayin.json`'daki zip indirilir | 50 |
| Doğrula | boyut + SHA256 (imza `kur` içinde ed25519) | 15 |
| Kur | depoya açılır, seçilir, `dogrula` | 25 |
| Ayarla | harici bağımlılıklar (ör. Tailscale) | 10 |

Adım durumu: bekliyor, sürüyor (nabız gibi yanıp sönen halka), tamam, hata. Yüzde: tamam adım tam ağırlık, süren adım yarım
ağırlık. Genel yüzde: çekirdek dosyalar %10, programlar %90 (eşit pay). Bağımlılık olarak gelen paket (ör. Tailscale)
listeye kendiliğinden eklenir. Olay kaynağı: `taa-guncelle kur`, `TAA_ILERLEME=1` ortamında stderr'e
`TAA-ILERLEME<TAB>paket<TAB>indir|dogrula|kur|ayarla<TAB>basladi|bitti` yazar; öteki satırlar konsola aynen geçer;
çıkış kodu sıfır değilse süren adım "hata" olur ve son hata satırı mesajdır.

## Bekleme kartları
Veri `kurulum/kartlar.txt` (kod değişmeden düzenlenir): program adı, 1-2 cümle, not, örnek şablon özeti. Kartlar yalnız
beklerken (programlar kurulurken) sol alanın altında görünür; seçim ekranlarında ve bitince gizlenir. 9 saniyede bir döner,
fare kartın üstündeyken durur; Sol/Sağ ok ya da kartın ‹ › düğmeleriyle elle gezilir.

## Görsel dil ve erişim
- Minimalist: tek vurgu rengi (mavi), nötr griler, bol boşluk, ince çizgiler, gölgesiz; Segoe UI. Hata kırmızısı yalnız hata için.
- Açık / koyu: sistem uygulama teması (HKCU `...\Themes\Personalize\AppsUseLightTheme`); tema değişince (WM_SETTINGCHANGE)
  yeniden çizilir; koyu temada başlık çubuğu da koyu (DWM).
- Yüksek DPI: SetProcessDPIAware + GetDpiForWindow; tüm ölçüler 96 DPI mantıksal birimden ölçeklenir; WM_DPICHANGED
  önerilen boyuta geçer. Pencere 1040×700 (mantıksal), en az 860×600, çalışma alanının ortasında.
- Klavye: Enter = onay / Yeniden dene / Bitir; Esc = vazgeç / Bitir; Yukarı/Aşağı = seçim; Boşluk = işaretle; Sol/Sağ =
  kartlar; fare tekerleği seçimde gezinir. Tüm düğmeler fareyle de çalışır (üzerine gelince el imleci).
- Kapatma: bitmişse ya da Bitir bekleniyorsa kapanır; ilk onay ekranında Vazgeç sayılır; kurulum sürerken "yarıda kalır,
  kapatılsın mı?" sorulur (Evet = kurulum yarıda, çıkış kodu 1).

## Kod yerleşimi
- `kurulum/src/pencere.rs`: Win32 + GDI çizim katmanı (yalnız `cfg(windows)`).
- `kurulum/src/ekran.rs`: kurulum işi ↔ pencere köprüsü (ortak durum, komut kanalı; pencere yoksa konsolda seçim). Testli.
- `kurulum/src/secim.rs`: seçim modeli (tek/çoklu, imleç, konsol cevabı). Testli.
- `kurulum/src/direktif.rs`: adım üretimi + yan dosyaları kopyalama + veri kökü kuralı. Testli.
- `kurulum/src/subeler.rs` + `kurulum/subeler.txt`: gömülü şube listesi. Testli.
- `kurulum/src/ilerleme.rs`: adım, durum, olay, yüzde, makine satırı ayrıştırma. Testli.
- `kurulum/src/kartlar.rs` + `kurulum/kartlar.txt`: kart verisi, ayrıştırıcı, dönüş. Testli.
- Testler: `TAA_KUR_PAKET=<boş klasör> cargo test --manifest-path kurulum/Cargo.toml` (kurulum çalışma alanı dışında).
- Windows tür denetimi: `TAA_KUR_PAKET=<boş klasör> cargo clippy --manifest-path kurulum/Cargo.toml --target x86_64-pc-windows-gnu -- -D warnings`.

## Durum
Gerçek Windows'ta ÇALIŞTIRILMADI. Linux'ta: birim testleri; Windows hedefine çapraz derleme (mingw) ve wine 9 + Xvfb altında
sahte `taa-guncelle` ile uçtan uca deneme (onay, rol, şube, çoklu seçim, adımlar, bağımlılık, hata + yeniden dene, Bitir,
fareyle tıklama, koyu tema, kapatma onayı, `--sessiz`). Wine'ın GDI'si gerçek Windows'tan farklı yazı tipi işleyebilir;
yüksek DPI (150 %, 200 %) ve çoklu ekran gerçek Windows'ta denenmeli. İndirme bayt yüzdesi sonraki iş.
