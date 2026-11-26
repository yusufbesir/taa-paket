# taa-ihbar: ihbar hattı çekirdeği
Sahip: Yusuf. Plan: `PLAN-MUSTERI-HAZIR-MESAJI-2026-11-15.md` §S2.5 (RH12a-b). Yeni bağımlılık yok (serde, serde_json, sha2, taa-ayar, taa-dosya). Bu crate AĞA ÇIKMAZ.

## Durum (0.1.0)
- Karar: ayrı Telegram botu; bildirim Yusuf'un Telegram'ına ve e-postasına gider; ihbar sayfası TAA Rehber'de olacak.
- `kayit`: `Ihbar` (kategori, metin ≤ 4000 karakter, isteğe bağlı iletişim). Varsayılan ANONİM: kayıtta kimlik, cihaz, Telegram numarası alanı yok; yalnız gün (saat yok) saklanır. Takip numarası `IH-` + 10 hane, çağıranın verdiği ≥16 baytlık tohumdan türetilir.
- `defter`: yalnız-ekleme JSONL (`taa_dosya::satir_ekle`); ihbar ve teslimat kayıtları. Görüntüleme yalnız `Yonetici::bayraktan(true)` ile; bayrak yoksa hata.
- `bicim`: Telegram metni (≤ 4096, uzunsa kesilir) ve e-posta konu/gövde. Metindeki e-posta adresleri ve 7+ haneli sayı dizileri maskelenir; bırakılan iletişim bildirime yazılmaz (yalnız "bırakıldı").
- `gonderici`: `Gonderici` trait'i (Telegram ve e-posta), `SahteGonderici`, `teslim_et` (kanallar ayrı denenir; hata metninden bot anahtarı ve adresler silinir).
- `hiz`: belirteç kovası; kaynak düz metin tutulmaz (bellekteki tuzlu SHA-256 özeti, `tuz_yenile` ile unutulur), ayrıca genel kova ve en çok 10 000 kova.
- `ayar`: `ihbar.json` (çalışma verisi klasöründe, depoda değil): `telegram_bot_anahtari`, `telegram_sohbet`, `eposta_alici`, `eposta_gonderen`, hız değerleri. `Debug` sır göstermez. Boş kanal kapalıdır; en az bir kanal gerekir.
- `Hat::al`: özellik anahtarı (`ihbar`), hız sınırı, doğrulama, deftere yazma, iletim, teslimat kaydı. İletim başarısız olsa da ihbar defterde kalır.
- Testler: `cargo test -p taa-ihbar`.

## Henüz yok
- Gerçek Telegram botu ve e-posta göndericisi (`Gonderici` uygulaması), bot komutları, fotoğraf eki, Rehber ihbar sayfası ve kare kod (RH12b-c); TAA Eğitim tanıtımı.
- Plandaki "takip numarasıyla soru sorma / soruyu gönderene iletme" (yalnız bellekte ve en çok 7 gün tutulan eşleme): yok; gönderen kimliği hiç tutulmadığı için bu, botun ayrı bir bellek tablosunu gerektirir.
- RH12a sözleşme ve gizlilik kuralı (Fable denetimi zorunlu) yazılmadı; `modul.toml` sunar/ister alanları bu yüzden boş.
- Kategori listesi (hırsızlık, usulsüzlük, taciz, iş güvenliği, diğer) ve hız değerleri (kaynak başına 3 ihbar, saatte 1 yenilenir; genel 30, dakikada 1) tasarım önerisidir, Yusuf onayı gerekir.
- Maskeleme basit kurallıdır: ad, adres, plaka, IBAN'ın harfli kısmı ve "14 30" gibi kısa sayılar maskelenmez; metindeki yer, saat ve olay kişiyi ele verebilir (dürüst sınır, bot ilk mesajda söylemeli).
- Defterdeki ihbar metni özgün (maskesiz) durur ve düz JSONL'dir; diske şifreleme ve saklama süresi kararı yok. İletim hatasında yeniden deneme yok (teslimat kaydı var, tekrar gönderen yok).
- Özellik anahtarı `ihbar` ve `modul.toml` kayıt kuralı uyumluluk.toml'a işlenmedi (dokunulmadı).
- Windows'ta denenmedi; testler Linux'ta.
