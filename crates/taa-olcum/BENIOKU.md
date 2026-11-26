<!-- Sahip: Yusuf. yazan: [Claude#olcum 2026-11-26] -->
# taa-olcum: DG1a ölçüm aracı
Sahip: Yusuf. Plan: `planlar/PLAN-DAGITIK-ISLEM-UYGULAMA-2026-11-15.md` §3 DG1a, `PLAN-TAA-ISLEM-GUCU-BIRLESIK-2026-11-15.md`.
Yöntem belgesi: `docs/olcum/YONTEM.md` (ölçümden önce kilitli). Yeni bağımlılık yok (sha2, serde, serde_json).

## Durum (0.1.0)
- `taa-olcum envanter [--json]`: işlemci modeli, mantıksal ve fiziksel çekirdek, bellek (toplam/boş), disk türü ve boş alan, OS sürümü, güç (pil/priz), **rastgele işçi numarası** (`isci-<32 hex>`). Kişi/bilgisayar/kullanıcı adı, seri, MAC, IP okunmaz (testle doğrulanır).
- `taa-olcum olcum [--sure-ms N] [--pay ...] [--cikti dosya.json] [--json]`: yüzde 5, 8, 10, 12, 15 payların **her biri ayrı** ölçülür; 1 ısınma, 3 geçerli tekrarın ortancası; her biten iş birimi referans özetle bit bit doğrulanır, tutmayan tekrar sayılmaz (3 geçerli yoksa hız yazılmaz, en çok 3 ek deneme). Pay başına: hız, taşıma süresi alanı, ön plan gecikmesi (taban ve yük ortancası, artış yüzdesi, en kötü ek ms, eşik sonucu). Rapor: bilgisayar başına bir JSON + okunur Türkçe özet; `--cikti` var olan dosyanın üzerine yazmaz.
- `taa-olcum karsilastir a.json b.json`: iki koşunun pay başına hız farkı (kabul: sapma küçük; öneri sınırı %10, YONTEM.md §8).
- `taa-olcum yontem`: kilitli eşikler ve belge özeti. Rapor, belgenin SHA-256 özetini taşır; `yontem.rs` içindeki `YONTEM_SHA256_KILIT` ile eşleşmeyen belge testi kırar (kilit düzeneği).
- Linux'ta limit: iş parçacığı sayısı + görev oranı. Linux ve Windows için `cfg` ile derlenir.
- Testler: `cargo test -p taa-olcum` (21 test, Linux).

## Henüz yok
- **Windows'ta denenmedi.** Windows kolları (PowerShell/CIM envanteri, `limit::is_nesnesi`) yazıldı ama derlenmedi ve çalıştırılmadı (bu ortamda yalnız Linux hedefi var). **Windows iş nesnesi (Job Object) limiti yalnız yer tutucudur**: `uygula` hata döner, rapor `is_nesnesi: false` yazar. Gerçek işçide kesin işlemci üst sınırı, düşük öncelik ve bellek üst sınırı zorunlu; bu araçta düşük öncelik ve bellek sınırı da uygulanmıyor (rapora `false` yazılır). Windows için `windows-sys` gerekebilir; çalışma alanında var ama bu görevde yeni bağımlılık eklenmedi, Yusuf'a sorulacak.
- Gerçek iş türleri: OCR (standart sayfa/saat), dönüştürme (birim/saat), video (kaynak saniyesi/saat). Şimdilik sentetik SHA-256 zinciri; hız birimi `sentetik-birim/saat`.
- Gerçek taşıma süresi: alan var ama değer bellek içi serileştirme iskeletidir (`tasima_yontemi = bellek-ici-iskelet`); sunucuya giderek ölçüm DG1b'de.
- Nebim V3 POS ve yazarkasa temsilî işlemleri: ön plan etkisi şimdilik küçük sentetik işlemle ölçülür; gerçek kasa işlemi ve kasa programı açıkken ölçüm DG1b'de. Boş bellek, disk kuyruğu ve ısıl kısıtlama izlenmiyor.
- Envanterde toplanmayanlar: GPU, sunucuya hat hızı, hibrit (P/E) çekirdek ayrımı, Windows güncelleme durumu (raporda `toplanmadi` listesi).
- Eşik yorumu belirsiz, Opus kesinleştirecek: "hiçbir işlem 200 ms üstü olmaz" ifadesi burada **tabana göre ek gecikme** (plandaki "200 ms'den fazla uzamaz") olarak alındı; mutlak 200 ms ise YONTEM.md §5 ve `TEK_ISLEM_EK_GECIKME_MS` kullanımı değişir. "Ön planı etkilemeyen en yüksek pay" ardışık-geçen en yüksek basamak olarak tanımlandı (YONTEM.md §6).
- Disk türü sanal makinelerde güvenilmez: bu ortamda sanal disk `hdd` göründü (çekirdeğin `rotational` bayrağı). Gerçek bilgisayarda doğrulanmadı.
- Koşu tekrarlanabilirliği: bu paylaşımlı sanal makinede 1 sn'lik iki koşu arasındaki hız farkı %4-12 çıktı (bir basamak %10 önerisini aştı). Kabul ölçütü (iki koşu arası küçük fark) uzun tekrar süresiyle (varsayılan 2 sn, DG1b'de daha uzun) ve gerçek sunucu PC'de yeniden değerlendirilmeli.
- IPC/TAA Kur entegrasyonu (kurulumda önce ölçüm) ve raporun sunucuya gönderilmesi (DG5); şimdilik yalnız dosyaya/stdout'a yazar.
