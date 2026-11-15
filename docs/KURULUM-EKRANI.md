<!-- Sahip: Yusuf. yazan: [Claude#kur-ekrani 2026-11-15] -->
# Kurulum ekranı (TAA Kur kurucusu)

Yusuf'un isteği: kurulum sırasında bir ilerleme arayüzü; bekleme sırasında programlardan bahseden kısa kartlar,
notlar ve örnek şablonlar; profesyonel, göze hoş gelen, minimalist bir ekran.

## Yol: std-only yerel HTTP + tek HTML sayfa
`kurulum/` (TAA-Kur-Kurulum.exe) yalnız standart kütüphaneyle yazılmış bir konsol programıdır; penceresi yoktur.
`taa-kur-pencere` Win32 penceresi ayrı exe'dir ve `windows-sys` gerektirir; kurucuya bağımlılık eklemek Yusuf onayı
ister. Bu yüzden: kurucu `127.0.0.1` üzerinde rastgele bir kapıda küçük bir HTTP sunucusu açar ve varsayılan
tarayıcıda tek bir HTML sayfa açar. Sayfa satır içi CSS/JS'dir; dış bağlantı, yazı tipi, CDN yoktur.
Konsol akışı aynen kalır (sorular konsolda sorulur); sayfa yalnız gösterir, yeniden dene / bitir komutu verir.

- Varsayılan: etkileşimli kurulumda açık; `--sessiz` iken kapalı (istenirse `--arayuz`), `--arayuzsuz` ile kapatılır.
- Tarayıcı açılamazsa adres konsola yazılır; kurulum arayüzsüz sürer (arayüz hiçbir zaman kurulumu durdurmaz).
- Güvenlik: yalnız `127.0.0.1`'e bağlanır; `Host` başlığı denetlenir (DNS yeniden bağlama); komutlar (POST) sayfaya
  gömülen tek seferlik jeton ister (`X-TAA-Jeton`), başka site komut gönderemez. Müşteri verisi, anahtar gösterilmez.

## Aşamalar ve adımlar
Kurulum dört aşamadır: **Hazırlık → Çekirdek dosyalar → Seçim (konsolda) → Programlar → Bitti**.
Her program için dört adım vardır:

| Adım | Ne olur | Ağırlık |
|---|---|---|
| İndir | `yayin.json`'daki zip indirilir | 50 |
| Doğrula | boyut + SHA256 (imza `kur` içinde ed25519) | 15 |
| Kur | depoya açılır, seçilir, `dogrula` | 25 |
| Ayarla | harici bağımlılıklar (ör. Tailscale) | 10 |

Adım durumu: bekliyor, sürüyor, tamam, hata. Yüzde: tamam adım tam ağırlık, süren adım yarım ağırlık sayılır
(indirmede bayt ilerlemesi şimdilik yok; sayfa süren adımda belirsiz çubuk gösterir). Genel yüzde: çekirdek
dosyalar %10, programlar %90 (eşit pay). Bağımlılık olarak gelen paket (ör. Tailscale) listeye kendiliğinden eklenir.

Olay kaynağı: `taa-guncelle kur`, `TAA_ILERLEME=1` ortamında stderr'e makine satırı yazar:
`TAA-ILERLEME<TAB>paket<TAB>indir|dogrula|kur|ayarla<TAB>basladi|bitti`. Kurucu bu satırları yakalar,
öteki satırları konsola aynen geçirir; çıkış kodu sıfır değilse süren adım "hata" olur ve son hata satırı mesajdır.

## Hata ve yeniden dene
Başarısız programın kartında hata mesajı ve **Yeniden dene** düğmesi görünür. Programlar bitince kurucu,
başarısız olan varsa arayüzden yeniden dene ya da **Bitir** komutunu (veya konsolda Enter'ı) bekler.
Yeniden deneme adımları sıfırlar, deneme sayısını artırır.

## Bekleme kartları
Kartlar veri olarak `kurulum/kartlar.txt` içindedir (kod değişmeden düzenlenir). Her kart: program adı, 1-2 cümle,
bir not, bir örnek şablon özeti. Programlar: TAA Etiket, Tutaa, TaaRkod, TaaDön, TaaFiş / TaaTerziTakip, TAA Rehber,
TAA Veri Tabanı Deposu, TAA Video Yapıcı; ayrıca birkaç "rehber notu" kartı. Kartlar 9 saniyede bir döner; fare
üstündeyken ya da odak kartlardayken durur; ← / → ile elle gezilir. Metin uydurma özellik içermez; belirsizse genel yazılır.

## Görsel dil
- Minimalist: tek vurgu rengi, nötr gri tonlar, bol boşluk, ince çizgiler, gölgesiz; sistem yazı tipi.
- Açık / koyu: sistem tercihi varsayılan; sayfada Otomatik / Açık / Koyu seçimi.
- Renk seçici: 5 hazır vurgu + serbest renk (`<input type=color>`); seçim yalnız bu tarayıcıda hatırlanır.
- Erişim: tüm düğmeler klavyeyle; görünür odak halkası; ilerleme `role=progressbar` + `aria-valuenow`;
  durum değişiklikleri `aria-live`; `prefers-reduced-motion` varsa geçiş ve belirsiz çubuk animasyonu kapalı.
- Dil: Türkçe; telefon genişliğinde tek sütun.

## Kod yerleşimi
- `kurulum/src/ilerleme.rs`: saf mantık (adım, durum, olay, yüzde, makine satırı ayrıştırma, JSON). Birim testli.
- `kurulum/src/kartlar.rs` + `kurulum/kartlar.txt`: kart verisi ve ayrıştırıcı. Birim testli.
- `kurulum/src/arayuz.rs`: HTTP isteği ayrıştırma ve yönlendirme (saf, testli) + sunucu iş parçacığı.
- `kurulum/src/arayuz.html`: sayfa.
- Testler: `TAA_KUR_PAKET=<boş klasör> cargo test --manifest-path kurulum/Cargo.toml` (kurulum çalışma alanı dışında).

## Durum
Linux'ta birim testleri ve yerel sunucu denenebilir. Windows'ta (tarayıcı açma `cmd /c start`, konsol + tarayıcı
birlikte) ÇALIŞTIRILMADI; yalnız çapraz derleme ile tür denetimi yapılabilir. İndirme bayt yüzdesi sonraki iş.
