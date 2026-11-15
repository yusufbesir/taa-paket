<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-T0 2026-11-05] -->
# Değişiklik günlüğü

## Yayınlanmamış

### Terzi Takip: fiş defteri ileri uyumluluğu ve zincir özeti (TT1)

- **Şube kodu zorunlu (Yusuf 2026-11-15 21:30):** şube kodu olmadan fiş yazılamaz. `taa_fis::sube_kodu_gecerli` (küçük harf, rakam, tire, harfle başlar, 2-40; `docs/sube-sozlesmesi.md` §3'teki `subeler.kod` biçimi); `Defter::ekle` boş ya da geçersiz kodu reddeder ve deftere hiçbir şey yazmaz. Basılı fişte "Şube: <kod>" satırı çıkar. Sunucu (`taa-fis-sunucu`) şube kodunu `sube.json` ({"kod":"..."}) dosyasından alır, istemcinin beyanına güvenmez: dosya yoksa ya da geçersizse 503, istemci başka şube beyan ederse 409 `sube_uyusmaz`. Telegram hesap bağlama kodu da şube kodu ister (yönetici hesabı hariç). **Dürüst sınır:** bu, fişin şube koduyla damgalanmasıdır; belgeyi şube imza anahtarıyla (ed25519) imzalama Ş3 işidir ve henüz yoktur. **Geçiş:** `sube.json` dosyasını TAA Kur yazacak; o zamana kadar el ile konmalıdır, yoksa yeni fiş yazılamaz. Eski şubesiz fişler okunur.
- Şube kapsamı (K5, Telegram botu): usta, personel ve mağaza hesapları yalnız kendi şubelerinin fişini görür, imzalar, rafa koyar, hazır yapar ve listeler; başka şubenin fişi için "fiş yok" denir. Yönetici tüm şubeleri görür. Şubesi yazılmamış eski fişe denetim uygulanamaz, geçer. `Takip::sinirla` özet ve listelerin şubeye göre daraltılmasını sağlar. Not: fiş yazan uçların `Girdi.sube` alanını doldurması ayrı adımdır; doldurulana kadar yeni fişler de şubesiz kalır ve denetime takılmaz.
- `Fis.sube` ve `Girdi.sube` eklendi; eski satırlarda boş (`serde(default)`), okuma bozulmaz. `Imza` satırına isteğe bağlı `saat` ve `cihaz` alanları eklendi (boşsa dosyaya yazılmaz); `Defter::imzala_ayrintili` bunları yazar, `imzala` aynı kaldı.
- Zincir özeti: `Defter::zincir_baslat` bir "zincir başlangıcı" satırı yazar; ondan sonraki her satır sha-256 ile zincire girer. `zincir_ucu()` (satır sayısı + özet) ve `zincir_dogrula(uc)` ile merkeze gönderilmiş uç hâlâ defterde mi denetlenir. Eski satırlar yeniden yazılmaz ve zincire girmez.
- Sınır: zincir kısmi değişikliği yakalar; dosyanın tamamını yeniden yazan birini tek başına durdurmaz, bu yüzden uç eşitlemede (TT3) merkeze gönderilir. Eski sürümde çalışan program zincir başlangıcı satırını bilmez ve defteri "bozuk" sayar: zincir, tüm şube bilgisayarları güncellendikten sonra başlatılmalıdır.
- Yeni bağımlılık yok (sha2 çalışma alanında zaten vardı).

### Terzi Takip: personel ve usta QR ile eklenir (TT12c devamı)

- Yönetici `GET /yonetim/personel` ekranında (yalnız sunucu bilgisayarı) rol (usta, personel, mağaza telefonu), şube ve kendine özel not seçip kişiye özel, tek kullanımlık, 10 dk geçerli QR üretir. Kişi QR'ı telefon kamerasıyla okutunca Telegram `t.me/<bot>?start=baglan-<kod>` ile açılır ve hesap bağlanır; telefonda ve yönetici ekranında AYNI 4 haneli doğrulama sayısı görünür, yönetici sayıyı karşılaştırıp son onayı verir. İmzanın dayanağı Telegram'ın değişmeyen kullanıcı numarası, profil adı da kayda yazılır. Kodu elle yazmak yedek yol olarak kalır. Yeni rol `personel` (fiş görür, raf yazar; usta imzası ve WhatsApp düğmesi yok).

### Terzi Takip: Telegram botu (TT12c)

- `taa-telegram`: fiş kare koduyla bot açılır (`t.me/<bot>?start=<fiş no>`, kodda ad/telefon yok); fiş maskeli gösterilir, usta her KALEM için "Bitti" der, "Bitti mi? Evet/Vazgeç" onayından sonra imzalanır (anahtar: fiş, kalem, tur; yeniden iş yeni tur). Tüm kalemler imzalanınca fiş hazır olur ve müşteri bildirimi kuyruğa yazılır. İmza olayı `imza-olaylari.jsonl`'e `taa-takip` Olay alan adlarıyla yazılır (usta = rastgele `U-` kimliği, tg_id, o anki profil adı; @kullanıcı adı alınmaz).
- Hesap bağlama (Fable denetimi K1): yönetici kısa ömürlü tek kullanımlık 8 haneli kod üretir, usta bota yazar, hesap bağlanır, yönetici son onayı verir; kodsuz hesap için hiçbir şey kaydedilmez. Hesap başına dakikada 20 komut (ayar), saatte 5 yanlış kod.
- "Müşteriye WhatsApp'tan haber ver" ve "Gönderdim" düğmeleri yalnız `magaza` rolünde; mağaza hesabı usta imzası atamaz. Düğme yerel sunucudaki tek kullanımlık, o güne ait `/wa/<belirtec>` adresine gider; `wa.me` bağlantısı (numara + hazır metin) yalnız yerel ağda üretilir, Telegram'dan müşteri numarası geçmez.
- `yoklama` (uzun yoklama): `Aktarim` arayüzü, kalıcı ofset, tek süreç kilidi, `curl -K -` ile bot anahtarı komut satırında/hata metninde yok. Gerçek Telegram'da DENENMEDİ, `CurlAktarim` yalnız birim testlerle (sahte aktarım) sınandı. Webhook yolu korunur.
- `taa-fis-sunucu`: `GET /wa/<belirtec>`, yalnız bu bilgisayardan `POST /api/yonetim/baglama-kodu`, `GET /api/yonetim/hesaplar`, `POST /api/yonetim/hesap/<tg_id>/onayla|askiya|kaldir`. Yeni bağımlılık yok (`getrandom` zaten çalışma alanında).
- Henüz YOK: şube kapsamı denetimi (K5, `Fis.sube` TT1'de gelince), yönetici ekranı düğmeleri, fişe bot kare kodu basımı (TT12d), yönetici adına işaretleme (K9), zincir özeti (TT1).

### Kurulum ekranı: yerel pencere (tarayıcısız)

- `kurulum/`: tarayıcı + yerel HTTP ekranı KALDIRILDI (`arayuz.rs`, `arayuz.html`, HTTP testleri). Yerine kurucunun kendi Win32 + GDI penceresi (`pencere.rs`, yalnız Windows; `windows-sys` 0.61, Yusuf'un onaylı bağımlılığı, yeni crate yok). Onay, rol, şube (gömülü 15 şube, `kurulum/subeler.txt` → `<kök>\sube.txt`), isteğe bağlı programlar pencerede seçilir; bekleme kartları; sağda adım adım şube ayar direktifleri (`direktif.rs`): kurulum programının yanındaki `sunucu-yollari.json`, `sunucu-ca.pem`, `tailscale.anahtar` var olanı ezmeden yerine kopyalanır, kalanlar numaralı adım. Açık/koyu (sistem teması), yüksek DPI, klavye, kapatma onayı. `--sessiz`/`--arayuzsuz` iken pencere yok; `--sube <kod>` eklendi. Saf mantık platformdan bağımsız ve testli (`secim.rs`, `ekran.rs`, `subeler.rs`, `direktif.rs`; 22 test).
- Gerçek Windows'ta DENENMEDİ; Linux'ta wine 9 + Xvfb ile sahte `taa-guncelle` üzerinden uçtan uca (klavye, fare, hata + yeniden dene, koyu tema, kapatma onayı) denendi.

### Kurulum ekranı (ilk sürüm, tarayıcılı; yerini yerel pencere aldı)

- `kurulum/` (TAA-Kur-Kurulum.exe): tarayıcıda ilerleme ekranı (std-only yerel HTTP, yalnız 127.0.0.1, Host denetimi, komutlar jetonlu). Aşamalar, program başına İndir/Doğrula/Kur/Ayarla, yüzde, hata + Yeniden dene / Bitir; bekleme kartları (8 TAA programı + rehber notları, `kurulum/kartlar.txt`); tek vurgu rengi, açık/koyu, renk seçici, klavye, `prefers-reduced-motion`. `--arayuzsuz` kapatır, `--sessiz` iken kapalı (`--arayuz` açar). Saf mantık `ilerleme.rs`, `kartlar.rs`, `arayuz.rs` (16 birim test; çalıştırma: `TAA_KUR_PAKET=<boş klasör> cargo test --manifest-path kurulum/Cargo.toml`). Yeni bağımlılık yok. `docs/KURULUM-EKRANI.md`.
- `taa-guncelle`: `TAA_ILERLEME=1` iken stderr'e `TAA-ILERLEME<TAB>paket<TAB>adım<TAB>basladi|bitti` satırları (`ilerleme_satiri`, 1 test). Ortam yoksa davranış aynı.
- Windows'ta ÇALIŞTIRILMADI; yalnız `cargo clippy --target x86_64-pc-windows-gnu` tür denetimi. Linux'ta sahte `taa-guncelle` ile uçtan uca ve başsız Chrome ekran görüntüsüyle denendi.

### Usta puanı (QR)

- `taa-takip::puan` (yeni): QR okutma olayları, tezgâhtar işareti + yönetici tutanağı, 0–10 usta puanı (verimlilik, kalite, müşteri, zamanında, düzen), küçük örnek düzeltmesi, deneme dönemi, ani değişim uyarıları (yalnız yönetici), kural tabanlı yorum ve veriye göre sınırlı kendini ayarlama (`gozden_gecir`). Özellik anahtarları `qr-uretim` ve `usta-puan`, ikisi de varsayılan KAPALI. Usta puanı göremez; izin `takip.puan@1` yalnız yöneticiye verilmelidir (çağıran program uygular). 13 birim test. `docs/USTA-PUAN.md`. 
- `taa-fis-sunucu`: `POST /api/fis/{no}/okut`, `POST /api/fis/{no}/isaret` (telefon), yönetici uçları `POST /api/yonetim/tutanak`, `GET /api/yonetim/puan/{bas}/{bit}`, `POST /api/yonetim/puan/gozden-gecir/{bas}/{bit}` (yalnız sunucu bilgisayarı). Fiş sayfasına "İşi bitirdim (okut)" ve "Yeniden iş / şikâyet işaretle" düğmeleri. Yeni bağımlılık yok. Ayrıntı `docs/USTA-PUAN.md`.

### Usta puanı ekranı

- `taa-fis-sunucu`: yönetici puan ekranı `/yonetim/puan` (yalnız sunucu bilgisayarı): puanlar, yorumlar, ani değişim uyarıları, işaretler ve tutanak formu, ayar gözden geçirme düğmesi; `GET /api/yonetim/isaretler`. Yeni bağımlılık yok.

### T3-pencere

- `taa-kur-pencere` (yeni bin) ve `taa_kur::pencere_model`: paket listesi (ad, kurulu sürüm, seçili sürüm, durum), Kur / Kaldır / Güncelle düğmeleri ve alt durum satırı. Mantık platformdan bağımsız modelde (7 birim test, gerçek `Kur`, geçici kök); Win32 çizimi `#[cfg(windows)] win.rs` içinde ince katman.
- Güncelle düğmesi `GuncelleyiciArayuzu` trait'ine bağlı; varsayılan "Güncelleme kaynağı ayarlanmadı" der (taa-guncelle başka dalda).
- Kaldırma silmez: `Kur::kaldir` ile `eski\` klasörüne taşır; veri alanlarına dokunulmaz.
- Bağımlılık: yalnız Windows'ta `windows-sys 0.61` (Yusuf onaylı; `zip` zaten dolaylı olarak aynı sürümü çekiyordu), asgari özellikler. Windows'ta elle doğrulanmadı; tür denetimi `--target x86_64-pc-windows-gnu` ile yapıldı. `docs/T3-PENCERE.md` eklendi.
### T3-imza
- Yusuf onaylı yeni bağımlılıklar: `ed25519-dalek 2` (`default-features=false`, `std`) ve `toml` (`parse`). Başka bağımlılık, HTTP ve silme yok.
- `crates/taa-imza` eklendi: ed25519 anahtar üret/imzala/doğrula, hex, `ed25519:<hex>` genel anahtar dosyası, `taa-imza uret|imzala|dogrula` CLI. Özel anahtar yalnız verilen dosyadan okunur (0600, ezmez, günlüğe/`Debug`'a yazılmaz).
- `crates/taa-kur`: `paket.json` `imza` = `{alg, anahtar_kimlik, deger}` (imzalanan veri `icerik_sha256`). `Kur::yeni/ortamdan` `<kök>\anahtarlar\*.pub` okur; anahtar varken imzasız/yanlış imzalı paket `KurHata::Dogrulama` ile reddedilir, anahtar yokken eski davranış (+ `tani` uyarısı). Bozuk `.pub` imza denetimini kapatmaz. `Kur::imza_denetle`, `tani` alanları `uyarilar` ve `guvenilir_anahtarlar`. `kur.toml` (`kaynak_klasoru`, `kanal`) `toml` ile okunur (`kur_ayari_oku`).
- `crates/taa-guncelle` eklendi: `yayin.json` şeması, `Kaynak` trait'i + `YerelKaynak`, `kontrol` (zorunlu `en_az_surum`, geri alma koruması), `uygula` (SHA256 + imza, `kur --sec`, hatada önceki sürüme dönme; yama yalnız şemada, `tam`a düşülür), `taa-guncelle kontrol|uygula [--json]`.
- `tools/paket_uret.py` artık `imza` alanı yazmaz; hazır imza `--imza-dosya` ile alınır. `docs/T3-TASARIM.md` eklendi.
### T3-boru
- `taa-ipc`: Windows adlandırılmış boru taşıyıcısı (`BoruTasiyici`, `#[cfg(windows)]` `src/boru.rs`; `Tasiyici` trait'i, dosya tabanlı `GelenKutusu`'na yedekli). Yusuf onaylı tek yeni bağımlılık `windows-sys` (yalnız `[target.'cfg(windows)'.dependencies]`). Platformdan bağımsız parça `src/boru_ortak.rs` (boru adı `\\.\pipe\taa-<kullanici>-<program>`, 4 bayt uzunluk + JSON çerçeve, 256 KiB sınır, zaman aşımı, ACL = yalnız geçerli kullanıcı, `FIRST_PIPE_INSTANCE`, `YedekliTasiyici`) Linux'ta test edilir. `docs/ipc.md` "T3: Windows adlandırılmış boru" bölümü eklendi. DOĞRULANMADI: Windows kodu yalnız `cargo check --target x86_64-pc-windows-gnu` ile tür denetiminden geçti, Windows'ta çalıştırılmadı.

### T2-A4 (güvenlik sertleştirme; Opus incelemesi, 0 yüksek / 9 orta / 7 düşük)
- `taa-veri.js`: `eskiyeTasi` dışa açıldı (programlar bozuk alan dosyasını `<alan>/eski/` altına alır). `docs/veri-alani.md`: `rehber` + `TAA_VERI_KOK` kuralı kodla hizalandı.

- `araclar/js/taa-veri.js` v2: `'wx'` + `fsync` ile atomik yazım; ilk `SAHIP.toml` yazımı yarışsız (`wx`, yeniden okuma); `oneriler()` yalnız düz dosya, ≤256 KiB, zarf doğrulaması (`sema`, `oneren`, `degisiklik`), kimlik dosya adından (yük içindeki `id` yok sayılır); `altYol` ayrılmış üst adları harfe duyarsız, Windows aygıt adlarını, sonda nokta/boşluğu ve `:` reddeder; `oneriKapat`/`eskiyeTasi` hedef adı `wx` ile ayırır (ezme yok); bilinmeyen `SAHIP.toml` anahtarları korunur; `goc` `ekler` (ör. `.bak` da `.tasindi`). Programlar kopyayı güncellemeli.
- `crates/taa-veri`: ilk `SAHIP.toml` `create_new` ile (yarışta yeniden okur); `yol_denetle` harfe duyarsız ayrılmış adlar, Windows aygıt adları, sonda nokta/boşluk, denetim karakterleri.
- `crates/taa-kur`: `yol_dogrula` Windows aygıt adları ve sonda nokta/boşluk reddi; zip yinelenen ad denetimi harfe duyarsız; `kur_zip` geçici klasörü `create_dir` ile alır (boş ve yarışsız).
- Açık (T3): sahip tablosunun çalışma anında kitaplığa gömülmesi (sahte `gecici`/sahte asıl sahip), alan içi sembolik bağ denetimi, SAHIP.toml çözücüleri için ortak test kümesi, `eski/` boyut uyarısı (`docs/T2-TASARIM.md` §6).


### T2-A3

- `tools/sozlesme_denetle.py` yeni program modu: `--program <program.toml> [--surum-dosyasi <package.json|Cargo.toml>] [--paket-kok <taa-paket>]` (zorunlu alanlar, ad ve alan adı biçimi, `gerektirir` yalnız `taa-*`, `sunar`/`ister` biçimi ve tekilliği, sürüm eşitliği, `ayar_alani`, `docs/veri-sahipleri.toml` ile sahip/okur çapraz denetimi; `[veri.<alan>] gecici = true` geçici sahip uyarı verir). Mevcut paket denetimi değişmedi; 13 yeni test.
- `docs/veri-sahipleri.toml` (paket genelinde veri alanı sahip tablosu) eklendi.
- `docs/program-sozlesmesi.md`: "Taslak örnekler" yerine dört programın kesin değerleri (tutaa, taadon, taarkod, fiyatetiketi), taarehber "ayrı başlık, taslak"; geçici sahip notasyonu belgelendi.
- `araclar/js/taa-veri.js` (bağımlılıksız CommonJS veri alanı kitaplığı, programlar kopyalar) ve `araclar/js/test/taa-veri.test.js` eklendi; CI'ya `js` işi (Node 22, `node --test`) eklendi.

### T2-A1

- `crates/taa-veri` eklendi (program = ortak, gerektirir = taa-dosya; yeni Cargo bağımlılığı yok): veri alanı sahipliği (`ac_sahip`, `ac_gecici_sahip`, `ac_okur`), atomik yazım, öneri kutusu (`oner`, `oneriler`, `oneri_kapat`), `SAHIP.toml` düz TOML alt kümesi çözücü/yazıcı (hata satır numaralı). Kodda silme yok; devralınan `SAHIP.toml` ve kapatılan öneriler `eski/` altına taşınır.
- `docs/veri-alani.md` (veri alanı sözleşmesi: kök, SAHIP.toml, 6 kural, hata adları, öneri biçimi, göç kuralı) ve `docs/veri-sahipleri.toml` (paket genelinde sahip tablosu) eklendi.
- README crate tablosunda `taa-veri` "var"; `docs/TEK-CATI.md` §3'e T2 durum notu eklendi.

T1 (kurulum yöneticisi ve iletişim; yeni Cargo bağımlılığı yok).

- `taa-kur` (klasör paketi kurulumu, sürüm seçme, geri dönme, SHA256 doğrulama; zip ve imza T2) ve `taa-ipc` (dosya tabanlı gelen kutusu taşıyıcısı) crate'leri eklendi.
- `tools/paket_uret.py` (+ testi) eklendi: `modul.toml` + Cargo sürümü -> `paket.json`; `icerik_sha256` tanımı `paket.json`'a `icerik_sha256_tanim` olarak yazılır ve `docs/T1-TASARIM.md` §1.2'de kesinleştirildi.
- `tools/sozlesme_denetle.py`: iki yeni denetim (hata): `gerektirir` grafında döngü; istenen arayüz ana sürümü hedef modülün `arayuz` değerine eşit olmalı. Testler eklendi.
- README crate tablosu: `taa-kur` ve `taa-ipc` "var".
- `docs/TEK-CATI.md` §3'e T1 durum notu eklendi.

### T2-A2

- `taa-kur kur <paket.zip>`: bağımsız değişken `.zip` ile bitiyorsa zip `<kök>/gecici-<pid>/` altına açılır (yol kuralları klasör paketiyle aynı; en çok 10.000 girdi, açılmış toplam en çok 2 GiB; bildirilen boyutlar toplanıp sınır aşılırsa hiçbir şey açılmaz; sembolik bağ ve yinelenen ad reddedilir; `paket.json` zipin kökünde olmalı), sonra mevcut klasör kurulumu çalışır; geçici klasör iş bitince (hata dahil) `eski/` altına taşınır, silinmez. `zip` bağımlılığı `taa-donustur` ile aynı sürüm ve özelliklerle (yeni sürüm ya da özellik yok).
- `paket.json` şemasına ayrılmış `imza` alanı (`null`; Rust yok sayar, doğrulamaz; T3 imza). `tools/paket_uret.py --zip <cikti.zip>` aynı içeriği zip olarak üretir; testler eklendi.
- `taa-kur tani --json`: `veri_kok` ve `veri_alanlari` (`TAA_VERI_KOK` ya da platform varsayılanı altındaki her `<alan>/SAHIP.toml` için `{alan, sahip, gecici, sahip_kurulu}`); bozuk SAHIP.toml tanı `sorunlar` listesine girer. SAHIP.toml düz TOML alt kümesi taa-kur içindeki küçük yerel çözücüyle okunur (`taa-veri`'ye bağımlılık yok).
- `taa-katalog`: `kaydet` artık `fs::remove_file` kullanmaz; eski dosya `eski/<epoch>-<ad>` olarak `rename` ile taşınır (olmazsa kopyalanır ve eski dosya `<ad>.tasindi` adını alır); `eski/` altındaki dosya hiçbir zaman ezilmez. Test eklendi.

T0 (tek çatı, veri ve yapı temizliği). Kırıcı değişiklik yok; yalnız ek alanlar, belge ve denetim.

- 20 `modul.toml` dosyasına `sunar`, `ister`, `veri_sahibi`, `veri_okur`, `ayar_alani` alanları eklendi (mevcut alanlar korundu).
- `tools/sozlesme_denetle.py` (+ testi) eklendi; CI'ya sözleşme denetimi adımı eklendi.
- `docs/ENVANTER.md` (veri konumları, ayar dosyaları, sürümler) eklendi.
- `docs/TEK-CATI.md` (plan §18'in depo gerçeğine uyarlanmış hali) ve `docs/program-sozlesmesi.md` (program.toml taslağı) eklendi.
- `docs/sozlesmeler/` altında yetenek sözleşmeleri eklendi (bilinmeyen alanlar "belirsiz").
- `uyumluluk.toml` (test edilmiş küme taslağı) eklendi.
- Karar: `taa-terzi` program=ortak; `taa-bildirim`, `taa-takip`, `taa-telegram` program=taafis (TaaFiş ailesi tek program). Ayrı TaaTerziTakip programı T2'de Cargo features ile.
- Denetim betiğine: gerektirir = Cargo bağımlılıkları birebir; ince çizgi (başka programın crate'ine zorunlu bağımlılık hata). CI'ya `permissions: contents: read`.
- `docs/modul-sozlesmesi.md` yeni alanlarla güncellendi; README "Modül sözleşmesi" bölümüne bağlantılar eklendi.

## Teslim tarihi önerisi (TaaSüre)
- taa-takip: `sure` modülü; usta kuyruğuna ve `sure.json` sürelerine göre tahmini teslim tarihi önerir, tablo yoksa "tahmin yok, elle seç". Özellik anahtarı `teslim-oneri`, yeni yetenek `teslim.oneri@1`. Fiş ekranına düğme bağlama ayrı adım.
