<!-- Sahip: Yusuf. yazan: [Claude#dg0-sozlesme 2026-11-26]. Durum: TASLAK (DG0). Fable denetimi YAPILDI 2026-11-26: Kapı 2 onayı VERİLMEDİ; bloklayıcı düzeltmeler §17'de (önceki metinle çelişirse §17 geçerli). Rapor: planlar yanı denetim/DG0-FABLE-DENETIMI-2026-11-26.md. Değişiklik: Yusuf'un onayıyla. Kod yok. -->
# TAA İşlem Gücü: dağıtım sözleşmesi (DG0)

Kaynaklar (okundu): `planlar/PLAN-TAA-ISLEM-GUCU-BIRLESIK-2026-11-15.md` (güncel tek belge), `planlar/PLAN-DAGITIK-ISLEM-2026-11-15.md` (kurgu, tarihçe), `planlar/PLAN-DAGITIK-ISLEM-UYGULAMA-2026-11-15.md` §3 DG0 ve §7b, `denetim/DENETIM-DAGITIK-ISLEM-KURGU-2026-11-15.md` (K1-K7, Y1-Y8, R1-R9), `planlar/briefler/BRIF-DG0-SOZLESME.md`. Uygulanmış kod: `crates/taa-zincir` (defter, bkz. §4). Ölçüm: `crates/taa-olcum`, `docs/olcum/YONTEM.md`.
Çelişki sırası: Yusuf'un en son kararı, birleşik plan, ana plan §14, bu belge.

**Kural:** belirsiz yer "AÇIK:" ile işaretlidir ve tahmin edilmemiştir. "Öneri" yazan değer, Yusuf ya da Fable onaylayana kadar bağlayıcı değildir. Sayısal değer "DG1b" ya da "ölçüm" diye işaretliyse ölçümden önce kesinleşmez. Bu belge Windows'ta denenmemiştir; Windows araçları (§5, §8, §10) tasarım okumasıdır, ölçüm değildir.

## 0. Kapsam, adım kodları (K1) ve denetim koşulları eşlemesi

Adım kodları yalnız DG ile yazılır (K1). Belgedeki eski "D0-D7" kodları ana planın dalga kodlarıyla çakıştığı için bu belgede kullanılmaz.

| Kod | İş | Durum |
|---|---|---|
| DG0 | Bu sözleşme | Taslak, Fable onayı bekliyor |
| DG1a / DG1b | Ölçüm aracı / gerçek ölçüm | DG1a iskelet var (`taa-olcum`); DG1b yapılmadı |
| DG2a | Ortak crate (`taa-zincir`) | Yapıldı, 12 test geçiyor |
| DG2 / DG3 / DG4 | Dağıtıcı / işçi / iş türleri | Kapı 1 ve Kapı 2 bekliyor; kod yok |
| DG5 / DG6 | TAA Kur paketi / pilot | Kapı 3 bekliyor |

Bu belge yalnız sözleşmedir. Dağıtıcı, işçi ve zarf kodu Kapı 1 (ölçüm kazancı) ve Kapı 2 (bu belgenin Fable onayı) geçmeden yazılmaz.

| Koşul | Karşılandığı yer | Özet |
|---|---|---|
| K1 adım kodları | §0 | Yalnız DG kodları |
| K2 anahtar ve zarf ayrıntıları | §5, §6 | Ayrı anahtarlar; zarfa ve imzaya bağlanan alanlar; parça anahtarı bir kez, kirada yalnız sarma; depolama adı şifreli hâlin özeti; eski genel anahtarlar saklanır |
| K3 çift çalıştırma | §1.4, §2.7 | "Bit bit aynı" işareti kapalı başlar; tek uyuşmazlık karantina değil |
| K4 veri sınıfı (yeni hâl) | §9 | Sınıfsız iş en sıkı sınıf; B ve C yalnız aşama açılınca, işaretli işçiye, yönetici anahtarı girilmişken |
| K5 boşta algılama | §8.5 | Boşta algılama kullanılmaz (Yusuf kararı 3); yerine ölçülmüş limit; kasa gecikmesini işçinin nasıl göreceği AÇIK |
| K6 dosya çözümleyici yüzeyi | §8.2 | Ağsız, bellek ve süre sınırlı alt süreç; Windows araçları adıyla |
| K7 iki katmanlı anahtar düzeni | §5.5 | Yönetici parolası → sunucu ana anahtarı → parça ve dosya anahtarları; bellek, yeniden başlatma, kurtarma |
| Y2 defter tanımı | §4 | İlk sürüm: sunucuda yalnız eklenen, özet zincirli, imzalı defter; işçi yerel zinciri sonraki aşama |
| Y3 zarf biçimi | §5.2 | Kendi şifreleme düzeni yok; hazır biçim (öncelik HPKE, RFC 9180); kütüphane seçimi AÇIK |
| R1-R9 | §9.4 | Kişisel verili aşama şartları |

## 1. İş türü

### 1.1 Tanım alanları
Her iş türü sürümlü, imzalı bir **iş türü paketinde** tanımlanır. Alanlar:

| Alan | Anlam |
|---|---|
| `ad` | Nokta ile ayrılmış küçük harf adı (ör. `ocr.genel`). Ad, veri sınıfını belirler (§9.1) |
| `surum` | Tamsayı. Parça, zarf ve sonuç bu sürüme bağlıdır; sürüm farklıysa sonuç reddedilir |
| `veri_sinifi` | `A`, `B` ya da `C`. Yazılmamış ya da tanınmayan değer `C` sayılır (K4) |
| `bit_bit_ayni` | `false` ya da `true`. **Kapalı (`false`) başlar**; yalnız iki gerçek bilgisayarda denendikten sonra ve Fable onayıyla `true` olur (K3) |
| `dogrulama` | Doğrulama yöntemi listesi (§2.7): `bicim`, `sayfa_eslesmesi`, `ornek_yeniden_hesap`, `cift_calistirma` |
| `tepe_bellek_bayt` | Bir parçanın işlenirken ulaşabileceği en yüksek bellek (ölçülür, §1.3). Bu değer işçinin boş belleğine sığmıyorsa parça o işçiye verilmez |
| `parca_sure_ust_sn` | Bir parçanın ölçülmüş tek işçide en uzun süresi; kira süresinden belirgin kısa olmalı (§2.3) |
| `girdi_sema`, `sonuc_sema` | Girdi ve sonuç için biçim tanımı (alan adları, boyut sınırları) |
| `calistirici` | İmzalı paketin içindeki çalıştırılabilir (OCR motoru ve dil modeli dahil). İşçi dışarıdan hiçbir şey indirmez |
| `paket_sha256`, `imza` | Paket özeti ve imzası (§6) |

Sınıf kararı iş türü paketindedir. İşi gönderen program sınıfı seçemez ve değiştiremez (K4).

### 1.2 İlk iş türleri
Birleşik plan §7 ve uygulama planı DG4 ile:

| Ad | Sınıf | İçerik | Kim işler | Açılış |
|---|---|---|---|---|
| `ocr.genel` | A | Kişisel verisiz (etiket, katalog, şablon) | Her onaylı işçi | İlk sürüm ve pilot |
| `ocr.sube` | B | Şubenin kendi belgesi | Yalnız "işaretli" işçi (§9), hangi şubede olursa olsun | Kapı 5'ten sonra |
| `ocr.kisisel` | C | Müşteri ya da personel bilgisi taşıyan fiş ve belge | Yalnız "işaretli" işçi | Kapı 5'ten sonra |

Fiş, tutanak, istihkak ve müşteri formu akışları `ocr.kisisel`dir. Belgenin sınıfı kaynağına (hangi programın hangi akışı) göre verilir; otomatik tahmine bırakılmaz. Sonraki türler (dosya dönüştürme, video bölütü) aynı tanım alanlarıyla eklenir; dağıtılmayanlar: anlık işler (tutanak kaydı, etiket basımı) ve tek büyük yapay zekâ çağrısı.

### 1.3 Tepe bellek
Tepe bellek, iş türü başına gerçek parçalarla ve işçideki limit düzeneği altında ölçülür (`taa-olcum` aynı düzeneği kullanır). Ölçülmeden iş türü açılmaz. AÇIK: ölçüm yönteminin tepe belleği nasıl okuyacağı (Windows'ta iş nesnesi sayacı mı, süreç sayacı mı) DG3 ve DG1b'de belirlenir; `taa-olcum` şimdilik bellek üst sınırını uygulamıyor ve tepe belleği ölçmüyor (`crates/taa-olcum/BENIOKU.md`).

### 1.4 "Bit bit aynı" işareti (K3)
OCR ve video çıktısı farklı işlemcide ya da kütüphane sürümünde bit bit aynı çıkmayabilir. Bu yüzden:
- İşaret kapalı başlar. Kapalıyken iki sonucun karşılaştırması bit bit eşitlik aramaz; §2.7'deki biçim ve eşleşme denetimleri kullanılır.
- İşaret açılırsa karşılaştırma bit bit yapılır. Açma koşulu: iki gerçek bilgisayarda, farklı işlemci ailesiyle, aynı girdilerde bit bit aynı çıktı; kanıt `docs/olcum/` altına yazılır; Fable onayı.
- Tek uyuşmazlık işçiyi karantinaya almaz (§2.7). AÇIK: karantina eşiği (kaç uyuşmazlık, hangi pencerede, üçüncü çalıştırma sonrası kaçıncı tekrar) ölçümsüz sayı olmaz; DG4'te iki bilgisayarda deneme sonrası.

## 2. Manifest, parça, kira, belirteç, durum makinesi

### 2.1 Kimlikler
- `is_kimligi`: sunucunun ürettiği rastgele 128 bit (onaltılık 32 hane). İşin kendisi.
- `parca_kimligi`: sunucunun ürettiği rastgele 128 bit. İşçiye giden pakette parça için yalnız bu kimlik vardır. Parçanın hangi şubenin hangi belgesinin kaçıncı sayfası olduğu bilgisi yalnız sunucu tablosunda ve imzalı manifestte durur; işçi bunu bilmez ve buna karar veremez (birleşik plan §4 madde 2).
- `isci_kimligi`: `isci-<32 hex>` biçimi (`taa-olcum` envanterindeki rastgele işçi numarasıyla aynı biçim). Kişi, bilgisayar ve şube adı içermez. AÇIK: işçi numarasını işçi imza anahtarına bağlayan kayıt kanıtının biçimi (kayıtta ölçüm raporuyla birlikte imzalı gönderim mi) DG2.3'te belirlenir.
- `kira_kimligi`, `deneme_no`: §2.3.

### 2.2 Manifest
İmzalı manifest, işin sunucudaki tek ve değiştirilemez tanımıdır. Alanlar:

| Alan | Anlam |
|---|---|
| `is_kimligi` | Bkz. §2.1 |
| `is_turu`, `is_turu_surumu`, `paket_sha256` | Hangi iş türü paketi |
| `veri_sinifi` | İş türünden gelir (§1.1) |
| `parcalar` | Her biri: `parca_kimligi`, `sira_no` (belge içi sıra), `girdi_sifreli_sha256` (şifreli hâlin özeti), `girdi_acik_sha256` (açık hâlin özeti), `bayt` |
| `olusturma_zamani` | Sunucu saati, RFC 3339 UTC |
| `imza` | Sunucu manifest imza anahtarı (§6.1), `taa_imza::Imza` biçimi |

- `girdi_acik_sha256` yalnız imzalı manifestte durur. Depolama adı ve işçiye giden her yerde yalnız `girdi_sifreli_sha256` bulunur; B sınıfında içerik eşitliği sızmasın diye (K2).
- Manifestteki `sira_no` ve şube/belge eşlemesi işçiye gitmez. İşçiye giden parça paketinde manifestin yalnız o parçanın satırı ve `imza`sı kısmen bulunur: AÇIK: işçinin manifest imzasını nasıl doğrulayacağı (tüm manifest imzalıysa tamamı işçiye gitmeden doğrulanamaz). Çözüm adayı: parça başına ayrı imzalı "parça bildirimi" (`is_kimligi`, `parca_kimligi`, iş türü ve sürüm, `girdi_sifreli_sha256`); manifest bunların köküdür. Seçim DG2.1'de.

### 2.3 Kira
- Kira süresi **5 dakika (300 sn)**; Yusuf kararı, birleşik plan karar 9.
- Kalp atışı aralığı: **30-60 sn** (öneri; DG0 bu aralığı yazar, DG3 ölçümüyle sabitlenir). Her kabul edilen kalp atışı kira bitişini **sunucu kabul zamanı + 300 sn**e taşır. Kira süresi ve zaman değerlendirmesinde yalnız sunucu saati kullanılır; işçi saati yalnız bilgidir.
- Kalp atışı kesilirse (bilgisayar kapandı, hat koptu) kira en geç bitiş zamanında düşer; parça yeni `deneme_no` ile başka işçiye verilir.
- Hizmet düzgün kapanırken elindeki parçayı kendisi geri bırakır (`kira.birakildi`); 5 dakika beklenmez.
- Bir parça 5 dakikadan **belirgin kısa** sürmelidir (OCR'da tek sayfa). AÇIK: "belirgin kısa"nın sayısı (ör. kira süresinin yüzde kaçı) DG1b ölçümünden sonra yazılır.
- AÇIK: kalp atışıyla uzayan bir kiranın toplam süre üst sınırı var mı. Sınırsız uzayan kira, takılı bir işçide parçayı sonsuza kadar tutar. Sınır `parca_sure_ust_sn` ile ilişkilendirilebilir; sayı ölçümsüz yazılmaz.
- AÇIK: sunucu saati geriye alınırsa ya da sunucu uykudan döndüğünde süresi dolmuş kiraların toplu geri alınma sırası; öneri: açılışta tek taramayla süresi dolmuş tüm kiralar düşürülür, defter kaydı yazılır (birleşik plan §6).

### 2.4 Sahiplik belirteci
- Kira açılırken sunucu rastgele bir **belirteç** (256 bit) üretir ve işçiye verir. Sunucu belirtecin kendisini değil SHA-256 özetini saklar (öneri; belirteç sızsa bile veri tabanı sızıntısıyla kullanılamaz).
- Belirteç tek bir (`parca_kimligi`, `kira_kimligi`, `deneme_no`, `isci_kimligi`) dörtlüsüne bağlıdır. İşçi her kalp atışında ve sonuçta belirteci sunar; sunucu tüm bağları denetler.
- Belirteç tek başına yetki değildir: istek ayrıca işçinin cihaz imzasıyla (§6) imzalanır. Belirteç, kira bitince ve işçi iptal edilince geçersizdir.

### 2.5 Durum makinesi
Parça durumları:

| Durum | Anlam |
|---|---|
| `bekliyor` | Kuyrukta, verilebilir |
| `kirada` | Bir işçide, aktif kira var |
| `sonuc_geldi` | Sonuç alındı, doğrulanıyor |
| `dogrulandi` | Doğrulama geçti, sonuç saklı; bir daha yapılmaz |
| `reddedildi_gecici` | Sonuç reddedildi (geçersiz imza, biçim hatası); parça `bekliyor`a döner, `deneme_no` artar |
| `basarisiz` | Kalıcı hata (deneme üst sınırı aşıldı ya da girdi işlenemez) |
| `iptal` | İş iptal edildi |

Geçişler:

| Nereden | Nereye | Koşul | Defter kaydı türü |
|---|---|---|---|
| (yok) | `bekliyor` | İş bölündü | `parca.olustu` |
| `bekliyor` | `kirada` | İşçi iş istedi, ölçüm, sınıf, bellek, işaret ve limit denetimleri geçti | `parca.verildi` |
| `kirada` | `kirada` | Geçerli kalp atışı | (deftere yazılmaz, yalnız tabloda; AÇIK: yoğunluk) |
| `kirada` | `bekliyor` | Kira düştü ya da işçi bıraktı; `deneme_no` +1 | `kira.dustu` / `kira.birakildi` |
| `kirada` | `sonuc_geldi` | Aktif kirada geçerli sonuç geldi | `sonuc.alindi` |
| `sonuc_geldi` | `dogrulandi` | Doğrulama geçti | `sonuc.dogrulandi` |
| `sonuc_geldi` | `reddedildi_gecici` → `bekliyor` | Doğrulama kaldı | `sonuc.reddedildi` |
| `bekliyor`, `kirada` | `basarisiz` | Deneme üst sınırı aşıldı | `parca.basarisiz` |
| herhangi (bitmiş değil) | `iptal` | İş iptal | `is.iptal` |

- Kapanma yüzünden hiçbir parça `basarisiz` sayılmaz; iş için zaman aşımı yoktur (birleşik plan §6). Deneme üst sınırı yalnız işlenemeyen girdi (çalıştırıcı hatası, doğrulama kaldı) içindir. AÇIK: deneme üst sınırının sayısı.
- Bir parça için en çok bir aktif kira vardır. Bu, veri tabanında bir kısmî benzersiz kısıtla (`parca_kimligi` başına bir `aktif` kira) ve `FOR UPDATE SKIP LOCKED` ile sağlanır; uygulama kodundaki denetime tek başına güvenilmez.

İş durumları: `alindi` → `bolundu` → `calisiyor` → `birlestiriliyor` → `bitti`; yan durumlar `duraklatildi` (anahtar bekliyor, §5.5 ve §9) ve `iptal`.
- `duraklatildi`: B ya da C sınıfı iş için yönetici anahtarı girilmemişse ya da kişisel veri aşaması açık değilse parçalar dağıtılmaz, sunucuda bekler.
- Birleştirme: tüm parçalar `dogrulandi` olunca sunucu birleştirir; yorumlama yalnız sunucuda, Çatı yönetici panelinde yapılır.

### 2.6 Sonuç kabulü, geç ve yinelenen sonuç
Sunucu bir sonucu şu koşulların **hepsi** sağlanırsa kabul eder:
1. Belirteç eşleşir ve kira **aktiftir** (sunucu kabul zamanı < kira bitişi).
2. `deneme_no`, `kira_kimligi`, `isci_kimligi` kayıtlı kiradakiyle aynıdır.
3. İşçi cihaz imzası geçerlidir, cihaz iptal edilmemiştir (§7).
4. Sonuç imzası `is_kimligi`, `parca_kimligi`, iş türü ve sürümü, `kira_kimligi`, `deneme_no`, `girdi_sifreli_sha256` ve `sonuc_sha256` değerlerini kapsar (K2).
5. `sonuc_sha256`, çözülen sonucun özetiyle eşleşir; sonuç `sonuc_sema`ya uyar.
6. İş türü sürümü işin sürümüyle aynıdır.

- **Geç sonuç:** kira bitmiş ya da `deneme_no` eskimişse sonuç reddedilir (`sonuc.reddedildi`, neden: `gec`). Parça henüz başka işçiye verilmemiş olsa bile reddedilir: Yusuf kararı "eski işçinin geç sonucu belirteçle reddedilir". AÇIK: henüz yeniden verilmemiş parçanın geç sonucunu kabul etmek verimi artırır; plan reddetmeyi söylediği için reddedilir, değişiklik Yusuf kararıdır.
- **Yinelenen sonuç:** aynı (`parca_kimligi`, `deneme_no`) için aynı `sonuc_sha256` ikinci kez gelirse yanıt aynıdır ve hiçbir yeni kayıt oluşmaz (tek kayıt). Aynı (`parca_kimligi`, `deneme_no`) için **farklı** `sonuc_sha256` gelirse ikincisi reddedilir ve `sonuc.celisti` defter kaydı yazılır (olay incelemesi). Bunu veri tabanında (`parca_kimligi`, `deneme_no`) benzersiz kısıtı garanti eder.
- **Sahte sonuç:** imzası geçmeyen, başka işe/parçaya/kiraya bağlı, biçime uymayan sonuç reddedilir. Reddedilen sonuç parçanın durumunu `dogrulandi` yapamaz.
- **Şube kararı sunucudadır:** sonuç, sunucunun **kendi tablosundaki** `parca_kimligi` eşlemesine göre asıl şubenin verisine yazılır. İşçinin söylediği şube bilgisi kullanılmaz (birleşik plan §4 madde 6).

### 2.7 Doğrulama
Doğrulama iş türüne göredir (birleşik plan §7). Yöntemler:
- `bicim`: sonuç şemaya uyar, boyut sınırları içindedir.
- `sayfa_eslesmesi`: OCR'da sonucun sayfa sayısı ve sırası parçayla eşleşir.
- `ornek_yeniden_hesap`: sunucu örnek parçaları kendi işçisinde yeniden hesaplar ve sonucu karşılaştırır. AÇIK: örnekleme oranı ve karşılaştırma ölçütü (OCR'da metin benzerliği eşiği) ölçümden önce yazılamaz; bit bit eşitlik yalnız `bit_bit_ayni = true` iken kullanılır.
- `cift_calistirma`: gerekirse; iki aynı sonuç doğruluk kanıtı değildir (ortak hata olabilir), yalnız bozuk işçi ya da donanımı yakalar.

Tek uyuşmazlık işçiyi karantinaya almaz; üçüncü çalıştırma yapılır ve uyuşmazlık sayısı izlenir (eşik AÇIK, §1.4). Dağıtmak okumayı hızlandırır, doğruluğu artırmaz: Türkçe OCR'da harf hatası bugün yüzde 12-15 (birleşik plan §11); bu bir doğrulama değil, ayrı bir OCR doğruluk çalışmasıdır.

## 3. Veri tabanı ve kuyruk (sözleşme düzeyi)
Ayrıntılı şema DG2.1'dedir; sözleşme şunları bağlar:
- Dağıtıcı Rust + axum + PostgreSQL; işletim sistemine özel kod yok; Windows sunucuda çalışır, Linux'a değişmeden taşınabilir olmalıdır.
- Kuyruk veri tabanındadır; parça seçimi kısa işlemde `FOR UPDATE SKIP LOCKED` ile yapılır.
- Şube alanı zorunludur ve satır düzeyinde yalıtım vardır (birleşik plan §10 "Karışma"); işçiler veri tabanına doğrudan bağlanmaz, yalnız dağıtıcı uçlarını kullanır. Uçlar yalnız VPN içinden erişilir ve cihaz imzası ister.
- İşçi sunucuya bağlanır, sunucu işçiye bağlanmaz (çekme modeli).

## 4. Defter (taa-zincir)

### 4.1 Tanım ve kapsam (Y2)
İlk sürümde defter, **sunucuda yalnız eklenen, özet zincirli, ed25519 mühürlü tek tablodur**. İşçi yerel zinciri ve tanık imzası sonraki aşamadır (kurgu §5'teki iki katmanlı defter bu sürüm için geçerli değildir). Madencilik ve oylama yoktur; tek yetkili sunucudur. Defter kayıtlarına **içerik yazılmaz**: OCR metni, belge görüntüsü, ad, telefon, tutar, şube adı yazılmaz; yalnız rastgele kimlikler ve özetler yazılır.

### 4.2 Uygulanmış API (okundu: `crates/taa-zincir/src/lib.rs`)
Bu bölüm koddan okunmuştur; kodla çelişirse kod geçerlidir ve bu belge düzeltilir.

- `Kayit { sira: u64, onceki: String, zaman: String, tur: String, veri: serde_json::Value, ozet: String }`.
- `KOK_ONCEKI`: 64 sıfır; ilk kaydın `onceki` değeridir.
- `kayit_ozeti(sira, onceki, zaman, tur, veri)`: SHA-256 üzerinde `taa_imza::kanonik_bayt` ile kanonikleştirilmiş `{"sira","onceki","zaman","tur","veri"}`; çıktı onaltılık metin. Kanonik bayt: nesne anahtarları sıralı, boşluksuz (`serde_json` varsayılanı), **en üst düzeydeki** `imza` alanı çıkarılır.
- Merkle: RFC 6962. Yaprak `SHA-256(0x00 || kayıt özeti baytları)`, iç düğüm `SHA-256(0x01 || sol || sağ)`, bölme noktası n'den küçük en büyük 2'nin kuvveti; boş liste `SHA-256("")`.
- `Muhur { sayi, son_ozet, merkle_kok, zaman, imza: taa_imza::Imza }`. İmzalanan bayt: kanonik `{"tur":"taa-zincir-muhur@1","sayi","son_ozet","merkle_kok","zaman"}`. İmza ed25519 (`taa-imza`, hazır kütüphane; elle şifreleme yok).
- `Defter`: `yeni()`, `kayitlardan(Vec<Kayit>)` (zincir doğrulamasından geçmeden kabul etmez), `len`, `is_empty`, `kayitlar`, `son_ozet`, `ekle(zaman, tur, veri)`, `dogrula()` (sıra 1..n, önceki bağlantı, özet), `merkle_kok(n)` (ilk n kayıt), `muhurle(&OzelAnahtar, zaman)`, `muhur_dogrula(&GenelAnahtar, &Muhur)`.
- `Defter` bellektedir; **kalıcılık çağıranındır** (JSONL yazılıp `kayitlardan` ile yüklenir). `ekle` tek yazar varsayar (`&mut self`).
- `muhur_dogrula`: imza geçerli, zincir temiz olmalı; defter mühürden **kısaysa** (silinmiş kayıt) reddedilir; mühürden **uzunsa** (sonradan eklenmiş) geçerlidir; ilk `sayi` kaydın son özeti ve Merkle kökü mührünkiyle aynı olmalıdır.
- Testler: 12 test (SHA-256 bilinen yanıt, RFC 6962 boş/tek/üç yaprak, temiz zincir, içerik değişikliği, silme, araya kayıt sokma, mühür ve uzayan defter, kısalan defter, yeniden yazılmış defter, mühür alanı değişimi, başka anahtarla imza). `cargo test -p taa-zincir` bu belge yazılırken geçti.

`taa_imza::Imza.anahtar_kimlik` genel anahtarın ilk 8 baytıdır; güvenlik kimlikten değil tam anahtardan gelir (`taa-imza` açıklaması). Bu yüzden iptal listesi ve anahtar geçmişi **tam genel anahtarla** tutulur (§7).

### 4.3 Defter kayıt türleri
Tür adı `ad.eylem` biçimindedir. İlk liste (genişletilebilir, her yeni tür bu belgeye eklenir):

| Tür | `veri` alanları (özet) |
|---|---|
| `is.alindi`, `is.bolundu`, `is.bitti`, `is.iptal` | `is_kimligi`, `is_turu`, `is_turu_surumu`, `veri_sinifi`, `manifest_sha256` |
| `parca.olustu`, `parca.basarisiz` | `is_kimligi`, `parca_kimligi`, `girdi_sifreli_sha256` |
| `parca.verildi` (dağıtım kaydı, R3) | `is_kimligi`, `parca_kimligi`, `isci_kimligi`, `kira_kimligi`, `deneme_no`, `veri_sinifi` |
| `kira.dustu`, `kira.birakildi` | `parca_kimligi`, `kira_kimligi`, `deneme_no`, `isci_kimligi` |
| `sonuc.alindi`, `sonuc.dogrulandi` | `parca_kimligi`, `deneme_no`, `isci_kimligi`, `sonuc_sha256` |
| `sonuc.reddedildi`, `sonuc.celisti` | `parca_kimligi`, `deneme_no`, `isci_kimligi`, `neden` |
| `isci.kaydoldu`, `isci.onaylandi`, `isci.isaretlendi`, `isci.iptal` | `isci_kimligi`, `genel_anahtar_imza`, `genel_anahtar_zarf`, `gecerlilik_baslangic` |
| `anahtar.yenilendi` | `isci_kimligi`, eski ve yeni genel anahtarlar, tarih |
| `sinir.asildi` (R2) | `isci_kimligi`, `sinir_adi`, `deger` |
| `limit.degisti` | `isci_kimligi`, `is_turu`, eski ve yeni pay, onaylayan işlem (kimlik yazılmaz, işlem kimliği yazılır) |
| `muhur` | Mührün kendisi (aşağıda) |

- `veri` içinde kayan noktalı sayı kullanılmaz; yalnız tamsayı, metin, mantıksal değer, dizi ve nesne. Gerekçe: `kanonik_bayt` RFC 8785 (JCS) değildir, `serde_json` biçimidir; başka bir dilde yazılacak bir doğrulayıcıda sayı biçimi farklı çıkabilir. AÇIK: defter başka bir dilde doğrulanacaksa kanonikleştirme RFC 8785'e geçirilecek mi (bu `kayit_ozeti`yi ve mevcut defterleri değiştirir; sürüm alanı gerekir).
- AÇIK: şube kimliğinin deftere yazılıp yazılmayacağı. Olay incelemesi (bir bilgisayar ele geçirilirse neyin açığa çıktığı, R3) yalnız sayfa kimliği ve işçi ile yapılabilir (şube bilgisi sunucu tablosundadır); ama defter silinmez, şube kimliği yazılırsa silme/unutma ile çelişebilir. Bu belge şube kimliğini deftere **yazmaz**.
- `zaman`: **sunucunun kabul zamanı** (RFC 3339 UTC). İşçi saati yalnız bilgidir ve ayrı bir alanda (`isci_zamani`) tutulabilir.
- Aylık arşiv: defter **silinmez**; ay sonunda mührü atılır ve arşiv dosyası oluşturulur (`kayitlar.jsonl` + son mühür); aktif tabloda kalan zincir arşivin son özetinden devam eder. AÇIK: ay sınırında zincirin devamı (ilk kaydın `onceki` değeri `KOK_ONCEKI` değil arşivin son özeti olacaksa `Defter::kayitlardan` bunu desteklemiyor; crate'te ek bir "başlangıç özeti" kavramı ya da arşiv öncesi mührün imzalı tutanak olarak taşınması gerekir). Bu eksik crate'te kapatılmadan arşiv kodu yazılmaz.

### 4.4 Mühür
- Mühür **sayıya ve süreye** bağlıdır: her N kayıtta ve her T sürede (kayıt az olan günde de) bir mühür atılır (denetim önerisi). AÇIK: N ve T değerleri (ölçümsüz yazılmaz); önerilen yaklaşım: ikisi de ayar.
- Mühür bir `muhur` türünde deftere de yazılır; ayrıca mühür tablosunda tutulur.
- Dış tanık: son mührün (Merkle kökü) sunucu dışında bir yere (sunucu yöneticisinin elinde olmayan bir kanal) yazılması. AÇIK: tanığın ne olduğu (ayrı bilgisayar mı, e-posta mı, yönetici telefonu mu) belirlenmedi; birleşik plan "dış tanık mührü" der ama hedefi söylemez. Tanık seçilmeden "defter kurcalanamaz" iddiası yazılmaz: mühür anahtarı sunucudaysa sunucuyu ele geçiren defteri baştan yazıp yeniden mühürleyebilir.
- Mühür imza anahtarı, manifest imza anahtarıyla aynı olabilir mi: AÇIK. Öneri: ayrı (§6.1).
- Doğrulama: yönetici Çatı'da "defter doğrula" ile `Defter::dogrula` ve son mühürlerle `muhur_dogrula` çalıştırır; sonuç ekranda görünür.

### 4.5 Eşzamanlılık ve kalıcılık
`Defter` tek yazarlıdır. PostgreSQL'de ekleme tek işlemde, sıra numarasının benzersiz olduğu ve bir danışma kilidiyle serileştirildiği biçimde yapılır (öneri). Kayıt ekleme ile ilgili durum değişikliği aynı işlemde olur: parça durumu değişti ama defter kaydı yazılmadı hâli olmamalı. AÇIK: yüksek kayıt hızında (kalp atışı) tek yazar darboğazı; kalp atışı deftere yazılmaz (§2.5), ölçüm DG2'de.

## 5. Şifreli zarf ve anahtarlar

### 5.1 İlkeler
- Şifreleme elle yazılmaz. Yalnız hazır, denetlenmiş kütüphane ve standart biçim kullanılır (Y3).
- **İmza anahtarı** (ed25519) ve **zarf anahtarı** (X25519) **ayrı** üretilir; biri ötekinden türetilmez (K2). Aynı çifti iki iş için kullanmak yasaktır.
- Anahtarlar işçide Windows'un şifreli deposunda (DPAPI ya da eşdeğeri) durur. AÇIK: depo seçimi ve makine kapsamı mı kullanıcı kapsamı mı olduğu DG3.1'de, Windows'ta denenerek.
- Şifreleme yolda ve diskte korur; işleme anında parça bellekte açıktır. Ele geçirilmiş bir işleyen bilgisayar o sırada işlediği sayfaları görür (§9.6, dürüst sınır).

### 5.2 Zarf biçimi (Y3)
- Biçim: **HPKE, RFC 9180**, tek atımlık şifreleme (Base modu). Önerilen paket: KEM `DHKEM(X25519, HKDF-SHA256)`, KDF `HKDF-SHA256`, AEAD `ChaCha20-Poly1305`. Alıcı X25519 genel anahtarı işçinin zarf anahtarıdır.
- **Kütüphane seçimi AÇIK:** Y3 gerekçeli seçim ister. Bu belge yazılırken çalışma alanının `Cargo.lock` dosyasında `chacha20poly1305`, `ed25519-dalek`, `aead`, `rand`, `getrandom`, `zeroize` var; HPKE uygulayan bir crate (ya da `age`) **yok**. Yeni dış bağımlılık eklemek bu belgenin yetkisinde değildir. Kütüphaneyi seçmek için gerekenler: denetim durumu, bakım durumu, RFC 9180 test vektörleriyle uyumu, `no_std` olmaması gerekmediği, Windows'ta derlenmesi. Bu bilgiler doğrulanmadığı için kütüphane adı yazılmadı. Seçilene kadar zarf kodu yazılmaz (Kapı 2).
- AÇIK: HPKE yerine `age` biçimi seçilirse "ek doğrulanan veri" bağlama özelliği biçimde yoksa (K2 bunu şart koşar) seçim geçersizdir; aday kütüphane ek doğrulanan veriyi (HPKE `info` ve `aad`) desteklemelidir.

### 5.3 Zarfa bağlanan alanlar (K2)
HPKE `info` ya da `aad` içine, kanonik biçimde, şu alanlar girer. Biri değişirse zarf açılmaz:

`sozlesme_surumu`, `is_kimligi` (§9.5'teki AÇIK ile birlikte), `parca_kimligi`, `is_turu`, `is_turu_surumu`, `kira_kimligi`, `deneme_no`, `isci_kimligi`, `girdi_sifreli_sha256`.

Gerekçe: bu alanlar yoksa geçerli bir parça başka işin ya da başka kiranın yerine konabilir. Kimlikler rastgeledir; şube ve belge sırası zarfa girmez (§2.1).
AÇIK: K2 "parca_no" der. Burada `sira_no` bilerek bağlanmadı çünkü işçiye gidecek bir sıra numarası aynı belgenin sayfalarının sırasını sızdırır (birleşik plan §4: işçi belgenin yapısını bilmez). Parça kimliği bu amaca yeter; Fable bu değişikliği onaylamalıdır.

### 5.4 Parça anahtarı ve depolama
- **Parça anahtarı** parça başına **bir kez** üretilir (256 bit, sistem rastgele kaynağı). Parça içeriği bu anahtarla AEAD ile şifrelenir; şifreli hâl depolanır.
- **Depolama adı = şifreli hâlin SHA-256 özeti** (`girdi_sifreli_sha256`). Açık hâlin özeti yalnız imzalı manifestte durur (B sınıfında içerik eşitliği sızmasın, K2).
- **Her kirada yalnız parça anahtarı yeniden sarılır:** sunucu parça anahtarını o kiradaki işçinin zarf anahtarına HPKE ile sarar; şifreli parça gövdesi aynı kalır (içerik adresli). Böylece aynı parça yeniden verildiğinde şifreli gövdenin yeniden indirilmesi gerekmez (24 saat kuralı, §10.3), yalnız yeni sarılmış anahtar iner.
- Sarılmış anahtar zarfı §5.3'teki alanlara bağlıdır; bu nedenle bir kiradaki zarf başka bir kirada açılamaz.
- AÇIK: parça gövdesi için AEAD'nin parça başına tek bir anahtar ve sabit bir nonce ile mi, ya da akışlı bir düzenle mi yapılacağı kütüphane seçimine bağlıdır; nonce yeniden kullanımı yasaktır ve seçim Fable denetimine girer.
- AÇIK: büyük parçalarda (video) akışlı şifre çözme bellek tavanını nasıl koruyacak; OCR'da tek sayfa için gereksiz.
- A sınıfında zarf: birleşik plan §1 "parçalar şifreli taşınır" der; kurgu taslağında "A sınıfında ilk sürümde zarf atlanabilir" notu vardı (reddedilen bir görüş içinde geçiyordu). Bu belge **A sınıfında da zarfı zorunlu** yazar (daha sıkı). AÇIK: performans ölçümü zarfı pahalı gösterirse A sınıfında gevşetmek Yusuf kararıdır.

### 5.5 Anahtar düzeni: yönetici parolası, sunucu ana anahtarı, parça anahtarları (K7)
Katmanlar (Yusuf kararı 4, 14:53; birleşik plan §7 "anahtar düzeni"):
1. **Yönetici parolası:** yalnız yöneticinin bildiği parola.
2. **Sunucu ana anahtarı:** rastgele 256 bit anahtar; veri tabanında (ya da dosyada) **yalnız yönetici parolasından türetilen anahtarla sarılmış hâlde** durur.
3. **Parça ve dosya anahtarları:** her biri sunucu ana anahtarıyla sarılı durur.

Kurallar:
- **Yönetici parolası ve ondan türetilen anahtar hiçbir dosyaya, günlüğe, deftere, hata iletisine ya da hafızaya yazılmaz.** Bu bir kabul testidir (§13 G-ANAHTAR-01): günlük, defter ve hata çıktıları parolanın kendisi ve türetilmiş anahtar için taranır.
- **Parola türetme:** hazır bir parola tabanlı anahtar türetme işlevi (bellek zorlayıcı; öncelik Argon2id) kullanılır, elle yazılmaz. AÇIK: işlev ve kütüphane seçimi ve parametreleri (bellek, tur sayısı): çalışma alanında böyle bir bağımlılık yok (`Cargo.lock` taraması: `argon2` yok), yeni bağımlılık kararı Yusuf'a ait.
- **Bellekte tutma:** sunucu ana anahtarı parola girilince açılır ve yalnız sunucu işleminin belleğinde tutulur; belleğe kilitli sayfa ve sıfırlama (`zeroize`) kullanılır (çalışma alanında `zeroize` var). AÇIK: bellekte kalma süresi (süresiz mi, süreli mi); süreli olursa süre dolunca B/C parça dağıtımı durur. Sayı Yusuf kararıdır.
- **Yeniden başlatma:** sunucu açıldığında ana anahtar **kilitlidir**. Yönetici anahtarı girmeden: B ve C sınıfı iş dağıtılmaz (iş `duraklatildi`, sunucuda bekler), A sınıfı akmaya devam eder; yönetici anahtarı girmeden dosyalar sistem dışından okunamaz.
- **Yanlış deneme:** parola denemeleri sınırlanır ve her başarısız deneme deftere (`anahtar.acma.basarisiz`, parolasız) yazılır. AÇIK: sınır ve bekleme süresi.
- **Kurtarma:** yönetici parolası kaybolursa B ve C sınıfı veri açılamaz. Bu yüzden **yazılı kurtarma anahtarı** zorunludur: kurulumda ayrı bir rastgele kurtarma anahtarı üretilir, ana anahtar onunla da sarılır (ikinci sarma), kurtarma anahtarı yöneticiye **bir kez** gösterilir ve **kâğıda yazılarak** saklanır (denetim K7(d) ve birleşik plan). Kurtarma anahtarı sunucuda saklanmaz. AÇIK: kurtarma anahtarının kodlama biçimi (onaltılık mı, sözcük listesi mi), kâğıdın saklanacağı yer ve kimin bildiği Yusuf kararıdır.
- **Parola değişimi:** yalnız ana anahtarın yeni parola ile yeniden sarılmasıdır; parça anahtarları değişmez. AÇIK: şüphe durumunda ana anahtar yenileme (tüm parça anahtarlarını yeniden sarma) süreci.
- **A sınıfı** için aynı katman kullanılır mı: AÇIK. A sınıfında parça anahtarlarının ana anahtarla sarılması yönetici girişi şartını getirir; bu A sınıfının "kilitli sunucuda da akar" kuralıyla çelişir. Öneri: A sınıfı parça anahtarları ayrı bir sunucu anahtarıyla (yönetici parolasına bağlı olmayan) sarılır; Fable onayı gerekir.

## 6. İmza, cihaz kimliği ve manifest anahtarları

### 6.1 Anahtar türleri
| Anahtar | Tür | Kimde | Ne için |
|---|---|---|---|
| Sunucu manifest imza anahtarı | ed25519 | Sunucu | Manifest ve parça bildirimleri |
| Sunucu mühür imza anahtarı (öneri: ayrı) | ed25519 | Sunucu | Defter mühürleri |
| Sunucu zarf anahtarı | X25519 | Sunucu | İşçiden sunucuya sonuç zarfı |
| İş türü paketi imza anahtarı | ed25519 | Paket üreticisi (taa-yayin hattı) | İş türü paketleri |
| İşçi imza anahtarı | ed25519 | İşçi | Cihaz kimliği, istek ve sonuç imzası |
| İşçi zarf anahtarı | X25519 | İşçi | Sunucudan işçiye parça zarfı |

Hepsi ayrı çifttir. Sunucu özel anahtarlarının nerede durduğu (dosya, veri tabanı) ve hangilerinin yönetici parolasıyla sarılacağı AÇIK: mühür ve manifest imza anahtarı sunucu başlarken (parolasız) gerekir; zarf alma anahtarı B/C için kilitli olabilir. Bu ayrım DG2'de yazılır.
- İmzalama yalnız `taa-imza` ile yapılır (`Imza { alg: "ed25519", anahtar_kimlik, deger }`; imza bayt dizisi `kanonik_bayt` üzerindedir).

### 6.2 İşçi sonuç imzası (K2)
İşçi sonucu imzalar; imza kanonik JSON üzerindedir ve §2.6 madde 4'teki alanları (`is_kimligi`, `parca_kimligi`, iş türü ve sürümü, `kira_kimligi`, `deneme_no`, `girdi_sifreli_sha256`, `sonuc_sha256`) ve `isci_kimligi` alanını kapsar. Bu nedenle eski bir sonucun yeniden gönderilmesi, `kira_kimligi` ve `deneme_no` farklı olduğundan reddedilir. Sonuç gövdesi sunucunun zarf anahtarına HPKE ile şifreli gider (§5.2); sunucu imzayı **çözdükten sonra** da doğrular.

### 6.3 İşçi kaydı ve onayı
- İşçi kurulumda iki ayrı anahtar çifti üretir ve genel anahtarları `isci.kaydoldu` ile gönderir; kayıt `taa-olcum` envanteriyle (rastgele işçi numarası; kişi, bilgisayar, kullanıcı adı, seri, MAC, IP yok) birlikte gelir.
- **Kayıt iş almak için yetmez:** yönetici Çatı'da işçiyi onaylar (`isci.onaylandi`). Ölçülmemiş işçiye iş verilmez (Yusuf kararı 10). Pay onayı ayrıca verilir (§11).
- "Kişisel veri işleyebilir" işareti ayrı bir yönetici eylemidir (`isci.isaretlendi`), yalnız §9.3 koşulları sağlanıyorsa verilebilir.
- AÇIK: ilk kayıtta cihaz kanıtı (kayıt isteğini kimin gönderdiğinin doğrulanması). Plan "cihaz imzası" der ama ilk güven noktasını söylemez; öneri: kaydın yönetici onayına kadar iş almaması yeterli kalkan sayılır, kurulum paketine gömülü tek kullanımlık kayıt kodu bir seçenektir. Karar DG2.3'te.

## 7. Cihaz iptali ve anahtar yenileme

- **Kim:** yalnız yönetici Çatı'da iptal eder. İptal imzalı bir defter kaydıdır (`isci.iptal`: `isci_kimligi`, iki genel anahtar, zaman, işlem kimliği).
- **Etki (aynı işlemde):** (1) işçinin tüm aktif kiraları düşer ve belirteçleri geçersiz olur; (2) bu işçiden gelen hiçbir istek kabul edilmez (iş çekme, kalp atışı, sonuç); (3) iptalden sonra gelen sonuç reddedilir (`sonuc.reddedildi`, neden `iptal`).
- **İptal gecikmesi:** işçi bir sonraki kalp atışında (30-60 sn) ret alır; en kötü durumda elindeki parçayı işlemeye devam eder ve sonucu reddedilir. İptal edilmiş cihazın iptalden **önce** kabul edilmiş sonuçları geçerli kalır; şüphe varsa yönetici o işçinin sonuçlarını yeniden doğrulatabilir. AÇIK: şüpheli sonuçların toplu yeniden doğrulanmasının otomatik mi elle mi olacağı.
- **Açığa çıkan veri:** iptal edilen cihaza B/C parçası verilmişse, `parca.verildi` dağıtım defteri (R3) o cihazın hangi sayfa kimliklerini aldığını gösterir; olay raporu bu listeyle üretilir. Parça anahtarları ve zarf anahtarı iptalle değişmez ama aynı parçalar yeni parça anahtarıyla yeniden şifrelenmeden başka işçiye **verilmez**: iptal edilen cihaza verilmiş B/C parçaları için yeni parça anahtarı üretilir ve şifreli gövde yeniden oluşturulur. AÇIK: bu kuralın maliyeti ve yalnız B/C sınıfı için mi olduğu.
- **Anahtar yenileme:** işçi yeni anahtar çiftleri üretir; sunucu `anahtar.yenilendi` kaydı yazar. **Eski genel anahtarlar tarihleriyle (`gecerlilik_baslangic`, `gecerlilik_bitis`) saklanır**; yoksa eski defter kayıtları ve eski imzalı sonuçlar doğrulanamaz (K2). Anahtar yenilendikten sonra eski anahtarla yeni istek kabul edilmez.
- İptal listesi tam genel anahtarla tutulur, `anahtar_kimlik` (8 bayt) ile değil (§4.2).
- AÇIK: iptal edilmiş bir cihazın aynı fiziksel bilgisayarda yeniden kaydı (yeni kimlikle mi, yönetici açık onayıyla mı). Öneri: yeni kimlik ve yeni ölçüm.
- AÇIK: sunucu manifest, mühür ve zarf anahtarlarının yenilenmesi (merkez anahtarının çalınması) ve işçilerin yeni sunucu zarf anahtarını nasıl güvenle alacağı; çalınmış anahtar senaryosu tehdit modelinde (§12) yer alıyor ama kurtarma adımları yazılmadı.

## 8. İşçi ve çalıştırıcı sınırları

### 8.1 İşçi
`taa-isci`: yerel Windows hizmeti, kısıtlı hizmet hesabı, **varsayılan kapalı**. Sunucuya bağlanır, sunucu işçiye bağlanmaz; işçi veri tabanına bağlanmaz; yalnız VPN içinden iş çeker. Serbest komut yoktur: yalnız sürümlü, imzalı, izinli iş türü paketleri çalışır. İşçi dışarıdan model ya da çalıştırıcı indirmez.

### 8.2 Alt süreç yalıtımı (K6)
Dosya çözümleyiciler asıl saldırı yüzeyidir (girdi dışarıdan gelebilir). Çözümleme ve işleme **ayrı alt süreçte** yapılır; alt süreçte:
- **Ağ erişimi yok.** Windows'ta olası araçlar: alt süreç için AppContainer (hiçbir yetenek olmadan, ağ yetenekleri verilmeden) ya da Windows Güvenlik Duvarı'nda çalıştırıcı programına özel giden bağlantıyı engelleyen kural. İşçinin kendisi için ayrı kural: yalnız sunucuya çıkış.
- **Kısıtlı hizmet hesabı** (yönetici değil, standart ayrıcalıksız), alt süreç için kısıtlı belirteç.
- **İş nesnesi (Job Object):** alt süreç askıda oluşturulur, iş nesnesine atılır, sonra başlatılır; tüm alt süreçler tek toplam bütçededir; işlemci hız denetimi (sert üst sınır), bellek üst sınırı, süreç sayısı sınırı, işçi kapanınca alt sürecin kapanması (`KILL_ON_JOB_CLOSE`), yazma erişim sınırı (yalnız geçici, bellek diski ya da çalışma klasörü).
- **Süre sınırı:** `parca_sure_ust_sn` aşılınca alt süreç öldürülür.
- Açık hâl ve okunan metin diske ve günlüğe yazılmaz (§9.3); alt süreç girdiyi standart girdi/bellek paylaşımıyla alır, sonucu aynı yolla verir. AÇIK: Windows'ta bellek paylaşımı mı borulu aktarım mı; ikisi de denenmeden seçilmez.
- **Bu bölümün hiçbiri Windows'ta denenmemiştir.** AppContainer ile OCR motorunun birlikte çalışıp çalışmayacağı ve iş nesnesinin sınırları gerçek Windows'ta DG3.2'de sınanır. Denenmeden "bozuk dosya çalıştırıcıyı aşamaz" yazılmaz; bu bir kabul testidir (§13 G-CALISTIRICI-01).

### 8.3 Geçici dosya ve bellek
SSD'de üzerine yazma güvenilir silme değildir (denetim önerisi). Bu yüzden: açık hâl yalnız bellekte tutulur; diskte yalnız şifreli hâl bulunur; anahtarın atılması silme sayılır (§10.3). Sayfa dosyasına (sayfalama) düşme riski AÇIK: açık hâlin sayfa dosyasına yazılmasını önlemek için bellek kilitleme Windows'ta denenmeden söz verilmez.

### 8.4 Hizmet kapanışı
Hizmet düzgün kapanırken elindeki parçayı geri bırakır; anahtarlar bellekten silinir; alt süreçler iş nesnesiyle kapanır.

### 8.5 Boşta algılama yok (K5)
Yusuf kararı 3: "boştayken değil, işlem limiti". Denetim K5'in Windows hizmetinden klavye/fare okuma uyarısı bu yüzden **uygulanmaz**: bu sözleşme boşta algılama kullanmaz. Yerine işlem limiti, koruma eşikleri ve kasa gecikmesi izleme gelir (§11). **Denenmeden "kasiyer etkilenmez" yazılmaz.** Garanti verilen şey ölçülmüş bir tavandır; Nebim V3 POS ve yazarkasa bağlantısının gerçek gecikmesi DG1b'de, kasa programı açıkken ölçülür.
AÇIK: işçinin kasa programının gecikmesini koşarken nasıl göreceği. Birleşik plan §5 "kasa işlemleri ölçülen sınırdan fazla gecikirse payını düşürür ya da durur" der; bu işçinin kasa programını nasıl izleyeceğini (bir yardımcı süreç mi, sistem sayaçları mı, Nebim'den bir sinyal mi) söylemez. Seçenek ölçülmeden yazılamaz; o zamana dek çalışan sinyaller yalnız sistem düzeyindedir (boş bellek, disk, pil, ısıl). Denetim kasa gecikmesi izlemeyi DG1b ölçümünden sonra ekler.

## 9. Veri sınıfı ve kişisel veri şartları

### 9.1 Sınıflar (K4, yeni hâli)
| Sınıf | İçerik | İş türleri | Kim işler |
|---|---|---|---|
| A | Kişisel verisiz | `ocr.genel` | Her onaylı, ölçülmüş işçi |
| B | Şubenin kendi belgesi | `ocr.sube` | Yalnız işaretli işçi (şube önemsiz) |
| C | Müşteri ya da personel bilgisi | `ocr.kisisel` | Yalnız işaretli işçi (şube önemsiz) |

- Sınıfı yazılmamış ya da tanınmayan iş **en sıkı sınıf (C) sayılır**.
- Sınıfı **iş türü** belirler; gönderen program belirlemez (K4).
- "Yalnız o şubenin işçisi" denetimi kalktı (17:33 yönü, 18:17 kararı); yerine **"yalnız işaretli işçi"** denetimi **sunucuda** yapılır ve izolasyon testine girer (§13 G-SINIF-02, G-ISARET-01).
- **İlk sürüm ve pilot yalnız `ocr.genel`.** B ve C yalnız Kapı 5 geçilince, kişisel veri aşaması açıldıktan sonra dağıtılır. O zamana dek sunucu B/C belgesini yalnız kendisi okur; Terzi Takip "fiş yalnız yerel OCR" kuralı (O1) aşama açılana kadar aynen geçerlidir. Müşteri alanı fotoğrafı Telegram'dan geçmez (değişmedi).
- Sınıf kararı sunucuda verilir; işçi kendi sınıfını beyan etmez.

### 9.2 Aşamalı açılış
Kapı 5'in koşulları (birleşik plan §8): Kapı 4 geçmiş (pilot), Fable güvenlik denetimi ve karışma testi geçmiş, işaretli bilgisayarlar koşulları sağlıyor, dağıtım defteri çalışıyor (R3), avukat görüşü alınmış (R9). Geçemezse kişisel verili belge yalnız sunucuda okunur. Kişisel veri aşaması **bir ayardır** ve varsayılan **kapalıdır**; sunucuda açık/kapalı durumu yönetici eylemidir ve deftere yazılır (`asama.degisti`).

### 9.3 İşaretli işçi koşulları (§7 şart 1, R4)
Bir bilgisayar "kişisel veri işleyebilir" diye işaretlenebilir ancak:
1. Disk şifreli (AÇIK: bunu işçinin nasıl doğrulayacağı; Windows'ta BitLocker durumu okunabilir ama doğrulama yöntemi denenmedi, DG3'te).
2. İşçi kısıtlı hizmet hesabında çalışıyor.
3. Windows güncel ve desteklenen sürüm (destek dışı sürüm işaretlenemez; sürüm envanterde görünür). AÇIK: "güncel"in tanımı (hangi güncelleme tarihi); `taa-olcum` Windows güncelleme durumunu şimdilik toplamıyor.
4. O bilgisayardaki mağaza kullanıcıları yönetici yetkili değil (R4): yönetici yetkili ortak hesap varsa işaretlenemez.
İşaret koşulları değişirse (örn. disk şifresi kapanırsa) işaret otomatik düşer; işçi B/C parçası alamaz.

### 9.4 Kişisel verili belge için şartlar (şart 1-8) ve R1-R9
Bütün bunlar kişisel veri aşaması açılmadan önce uygulanır ve test edilir.

| Madde | Şart | Bu sözleşmede karşılığı |
|---|---|---|
| Ş1 | Yalnız işaretli bilgisayar | §9.3; §13 G-ISARET-01 |
| Ş2 | Yönetici anahtarı girilmişken; değilse dağıtılmaz | §5.5, §2.5 `duraklatildi`; §13 G-ANAHTAR-02 |
| Ş3 | Parça tek sayfa; mümkünse bölgelere ayrılır; belge tek bilgisayarda toplanmaz | Parça tek sayfa; R2 sınırı; bölge ayırma R6 |
| Ş4 | Açık hâl ve okunan metin işçide diske ve günlüğe yazılmaz | §8.2, §8.3, R5; §13 G-DISK-01 |
| Ş5 | İşçi belgenin hangi şubeye ait olduğunu bilmez | §2.1, §5.3; §13 G-KARISMA-03 |
| Ş6 | Aşamalı açılış; ilk sürüm yalnız `ocr.genel` | §9.2 |
| Ş7 | Şube ekranlarında ve raporlarda şube ayrımı sürer | Sunucu tarafı; §13 G-KARISMA-01 |
| Ş8 | Hukuki görüş kapsamı | R9 |
| R1 | Önce sunucu: kişisel verili belge önce sunucuda okunur; yalnız taşan iş dağıtılır | AÇIK: "sunucunun yetişemediği" eşiği ayar, pilotta belirlenir. Dağıtıcı B/C parçasını önce sunucu işçisine ister; sunucu işçisi doluysa ya da uygun değilse (§11.4) ve eşik aşıldıysa dağıtılır. |
| R2 | Toplama sınırı: bir bilgisayar, bir günde aynı şubenin ve aynı belgenin sınırlı sayıda sayfasını alır; aşım uyarı üretir | Sunucu sayar (sunucu şube/belge eşlemesini bilir). AÇIK: sayfa sayısı sınırları (ayar, pilotta). Aşımda yeni parça verilmez ve `sinir.asildi` yazılır. |
| R3 | Dağıtım defteri (içerik değil) | §4.3 `parca.verildi`; defter çalışmadan aşama açılmaz |
| R4 | İşaretli bilgisayarda mağaza kullanıcıları yönetici olmamalı | §9.3 |
| R5 | Kişisel verili parçada 24 saat önbellek yok; kira bitince şifreli parça silinir, anahtar hemen atılır, sonuç işçide tutulmaz | §10.3 |
| R6 | Bölgelere ayırma mümkün olan yerde, sunucuda; doğruluğu düşürüyorsa uygulanmaz | Söz verilmez, ölçülür. AÇIK: bölge ayırmanın doğruluğa etkisi ölçülmedi. |
| R7 | Sonuç taslaktır; ad, telefon, tutar kendiliğinden kesin kayıt olmaz; "onay bekleyen" olarak eklenir, bir kişi onaylar | Sunucu sonucu `taslak` durumuyla yazar; kesin duruma geçiş insan eylemidir; §13 G-TASLAK-01 |
| R8 | Yorumlama yalnız yerel; kişisel verili sonuç dış yapay zekâya gitmez | Dağıtıcıdan dışarı çağrı yok; yorumcu yerel model ya da kural tabanlı; §13 G-YORUM-01 |
| R9 | Avukat görüşü gelmeden aşama açılmaz | §9.2; AÇIK: görüş alınmadı |

### 9.5 Kimlik bağlama ve toplama
`is_kimligi` aynı iş için tüm parçalarda aynıysa, B/C'de bir işçi aynı işe ait farklı parçaları birbirine bağlayabilir (R2 toplama sınırının amacına ters). AÇIK: B/C sınıfında her parça için ayrı rastgele bir "dağıtım iş kimliği" (parçaya özel, sunucu tablosunda gerçek işe bağlı) kullanmak; bu sözleşme bunun kararını Fable denetimine bırakır. Karar verilene kadar B/C zarfa `is_kimligi` yerine bu parçaya özel kimliğin girmesi tercih edilir; A sınıfında `is_kimligi` yeterlidir.

### 9.6 Dürüst sınır
Kimliklendirme karışmayı, şifreleme yolda ve diskte okunmayı önler. Sayfayı okuyan bilgisayar onu bellekte açmak zorundadır; ele geçirilmiş işleyen bilgisayar o sırada işlediği sayfaları görür. R1-R4 bu riski küçültür, sıfırlamaz. Bu risk Yusuf'a açıkça söylenmiştir ve kişisel veri aşaması ayrı bir karardır.

## 10. Önbellek, saklama ve temizlik
1. **Diskte yalnız şifreli hâl** durur; açık hâl yalnız bellekte tutulur.
2. İş bitince ya da kira düşünce **parça anahtarı hemen atılır**; dosya o andan sonra açılamaz.
3. **Kişisel verisiz parça (A):** şifreli dosya en geç **24 saat** sonra silinir (Yusuf kararı 8). 24 saat içinde aynı parça yeniden gerekirse tekrar indirilmez, yalnız anahtar yeniden sarılır (§5.4).
4. **B ve C parçası:** 24 saatlik önbellek **yoktur** (R5). Kira bitince (iş tamamlanınca ya da 5 dakika dolunca) şifreli parça silinir ve anahtar atılır; sonuç metni işçide tutulmaz.
5. Silme: dosya silinir (SSD'de üzerine yazma güvence değildir; asıl koruma anahtarın atılması ve şifreli saklamadır). AÇIK: "silme" işçi tarafında kalıcı silme kuralıyla çelişir mi: depo geneli "kalıcı silme yok" kuralı kullanıcı verisi içindir, burada geçici işçi önbelleği söz konusudur (birleşik plan: "bunlar kullanıcı verisi değil, geçici kopyadır"). Yusuf bu yoruma onay vermelidir.
6. Defter ve manifest silinmez (aylık arşiv, §4.3).

## 11. İşlem limiti ve koruma eşikleri

### 11.1 Pay
- **Tavan: bilgisayar başına yüzde 15** (Yusuf kararı 10). Hedef değil tavandır. Tavanı yalnız Yusuf değiştirir.
- **Önce ölçüm:** ölçülmemiş bilgisayara iş verilmez. Pay her bilgisayar ve her iş türü (ve sürüm) için ayrı ölçülür.
- Pay basamakları ölçümde: yüzde **5, 8, 10, 12, 15** (her biri ayrı; `docs/olcum/YONTEM.md` §2). Pay, bütün ölçütleri geçen en yüksek **ardışık** basamağın **bir altı**dır ve 15'i aşmaz (birleşik plan §5; YONTEM.md §6).
- Pay yöneticinin onayıyla açılır; **artış yeni onay ister, azaltma ve durdurma otomatiktir.** Onay ve değişiklik `limit.degisti` olarak deftere yazılır.
- Pay hesabı: "mantıksal çekirdek sayısı × pay" kadar toplam işlemci süresi (YONTEM.md §2).
- Çevrim içi hız düzeltmesi: ölçülen hız ile gözlenen hızın ağırlıklı ortalaması (uygulama planı DG2.4 önerisi: 0,8 eski + 0,2 gözlenen); AÇIK: bu düzeltme pay artırmaz, yalnız eşleştirmede parça seçimini etkiler.

### 11.2 Nasıl zorlanır
Windows iş nesnesiyle kesin işlemci üst sınırı (tüm alt süreçler tek toplam bütçede), sınırlı iş parçacığı sayısı, düşük öncelik, bellek üst sınırı, kendi aktarım hızı sınırımız. Bu bir tavandır; "kasayı hiç yavaşlatmaz" garantisi değildir.
**Yer tutucu uyarısı:** `taa-olcum`'da Windows iş nesnesi limiti yalnız yer tutucudur (`uygula` hata döner, rapor `is_nesnesi: false` yazar); düşük öncelik ve bellek üst sınırı uygulanmıyor. Gerçek işçide üçü de zorunludur ve Windows'ta denenmeden Kapı 3 geçmez.

### 11.3 Koruma eşikleri
İşçi şu durumlarda **yeni iş almaz** ve gerekirse payını düşürür ya da durur:

| Eşik | Başlangıç değeri | Durum |
|---|---|---|
| Boş bellek | en az 2 GB ya da yüzde 20 (öneri) | AÇIK: DG1b ölçümüyle kesinleşir |
| Pil | Bilgisayar pildeyse iş almaz | Kesin |
| Disk | Disk doluysa iş almaz | AÇIK: "dolu" eşiği (boş alan bayt ya da yüzde) |
| Isıl | Yalnız sensör doğrulanırsa kullanılır; sensör yoksa "normal" sayılmaz | Kesin ilke; AÇIK: sensör doğrulama yöntemi |
| Kasa gecikmesi | Temsilî işlemlerin ortanca süresi tabana göre en çok yüzde 10 artar; hiçbir işlem tabana göre 200 ms'den fazla uzamaz | Öneri; `docs/olcum/YONTEM.md` §5'te ölçümden önce kilitli. AÇIK: "200 ms" mutlak mı tabana göre ek mi (BENIOKU `taa-olcum` bunu belirsiz bırakmış); izleme yolu §8.5'te AÇIK |

Başlangıç değerleri ölçümle kesinleşir; sonuca bakarak eşik seçilmez. Sunucu bağlantısı kopsa da işçi yerelde bu eşiklere uyar.

### 11.4 Sunucu işçisi ve gece payı
- Aynı işçi sunucu bilgisayarda da çalışır; aynı kuyruk, kira ve imza kuralları. Sunucu işçisi de ölçüm kapısından geçer. Sunucunun asıl işleri (şube istekleri, veri tabanı, yedek) her zaman önce gelir; yedek ve bakım saatinde sunucu işçisi durur. Sunucuda bellek dardır (14 GB, denemelerde 1-2 GB boş); ölçüm "uygun değil" derse sunucu işçisi açılmaz, iş sabaha bekler.
- Gündüz tavanı yüzde 15.
- **Sunucunun gece payı (Yusuf kararı 14, 18:31):** yalnız sunucu için; şube bilgisayarlarının tavanı yüzde 15 kalır. "Gece" saatle değil durumla tanımlanır: şubelerden kasa isteği gelmiyor ve şube işçileri kapalı; Yusuf isterse saat aralığı da ekler. Pay cihazın durumuna göre basamak basamak yükselir (öneri: 15, 25, 40, 60) ve **her basamak sunucuda önceden ölçülmüş olmalıdır**; ölçülmemiş basamağa çıkılmaz (not: `taa-olcum` bugün yalnız 5-15 basamaklarını ölçer, 25-60 ayrı ölçülecek, AÇIK).
- Yükselme koşulları: boş bellek eşiğin üstünde, prizde, disk dolu değil, ısıl kısıtlama yok, sunucu hizmetlerinin yanıt süresi sınırın içinde, yedek ya da bakım çalışmıyor. Koşullardan biri bozulursa ya da bir şubeden istek gelirse pay hemen yüzde 15'e ya da altına iner; iniş otomatiktir. Üst basamak Çatı'da görünür ve Yusuf oradan sınırlayabilir. Sunucu Yusuf'un kendi bilgisayarıdır; kendisi kullanıyorsa gece payı devreye girmez.
- AÇIK: sunucu hizmetlerinin yanıt süresi sınırının sayısı (DG1b).

### 11.5 Kapatma
- Çatı'dan **bilgisayar bazında ve genel kapatma düğmesi**; şubede yerel durdurma. Kapatma "saniyeler içinde" durdurur (§13 L-KAPAT-01). AÇIK: "saniyeler"in üst sınırı (öneri: 10 sn; ölçümsüz kesin değil).
- Bilgisayar açık tutulmaz ve ağdan uyandırılmaz.

### 11.6 Gece ve kapanma
Şube işçileri kapanınca kiraları en geç 5 dakikada düşer; kalan parçaları sunucu işçisi çeker ve sabaha kadar sürdürür. Kuyruk ve defter kalıcıdır. Sunucu da kapanırsa iş bekler; açılınca yalnız yarım kalan parça yeniden yapılır, biten yapılmaz; kapanma yüzünden iş başarısız sayılmaz ve toplam süre için zaman aşımı yoktur. Sabah şube işçileri açılınca kuyruktan kaldığı yerden pay alır; ayrı bir devir adımı yoktur.

## 12. Tehdit modeli

### 12.1 Varlıklar ve güven sınırları
- Varlıklar: B/C sınıfı belge içeriği ve OCR sonucu; manifest, mühür ve zarf anahtarları; yönetici parolası ve kurtarma anahtarı; defter; kasa programının çalışma süresi; sunucunun asıl hizmetleri.
- Güvenilen: sunucu (tek yetkili, Yusuf'un bilgisayarı), yönetici.
- Güvenilmeyen: işçi bilgisayarları (şube PC'leri), şube ağı, girdi dosyaları.
- **Kapsam dışı (bu sürüm):** sunucu bilgisayarının tamamen ele geçirilmesi (o hâlde defter ve anahtarlar da gitmiştir; yalnız §4.4 dış tanığı kısmen sınırlar); işletim sistemi çekirdeği ve donanım saldırıları; VPN'in kendisinin kırılması (ayrı belgeler: sunucu yolları, ağ).

### 12.2 Tehditler ve karşılıklar
| # | Tehdit | Karşılık | Test |
|---|---|---|---|
| T1 | İki işçi aynı parçayı alır | Bir aktif kira kısıtı + `SKIP LOCKED` (§2.5) | Q-AYNI-PARCA-01 |
| T2 | Eski işçi kira bittikten sonra sonuç gönderir | Geç sonuç reddi (§2.6) | Q-GEC-SONUC-01 |
| T3 | Aynı sonuç iki kez gelir / farklı sonuç aynı kira için gelir | Tek kayıt; çelişki `sonuc.celisti` (§2.6) | Q-YINELENEN-01, Q-CELISKI-01 |
| T4 | Sahte sonuç (imzasız, başka işe bağlı, biçime uymayan) | İmza ve bağlar (§2.6, §6.2) | G-SAHTE-SONUC-01 |
| T5 | İptal edilmiş cihaz iş çeker ya da sonuç gönderir | Anında düşen kira ve ret (§7) | G-IPTAL-01 |
| T6 | Çalınmış ya da kopyalanmış işçi anahtarı | İptal; yenileme; anahtar Windows deposunda | G-IPTAL-01, G-KOPYA-01 |
| T7 | Ele geçirilmiş işçi B/C sayfası görür | Yalnız işaretli işçi, R1-R4, R5, R3 dağıtım defteri; **tam çözüm yok** (§9.6) | G-ISARET-01, G-TOPLAMA-01 |
| T8 | Sayfa yanlış şubeye yazılır / karışma | Şube bilgisi yalnız sunucu tablosunda; işçi beyanı kullanılmaz; etiket doğrulama | G-KARISMA-01..03 |
| T9 | Bir zarf başka işin ya da başka kiranın yerine konur | Zarf bağlama (§5.3) | G-ZARF-01 |
| T10 | Bozuk girdi dosyası çözümleyiciyi aşar (K6) | Ağsız, kısıtlı, sınırlı alt süreç (§8.2) | G-CALISTIRICI-01 |
| T11 | Defterde araya kayıt sokma, silme, değiştirme, yeniden yazma | Özet zinciri + Merkle + mühür (§4) | D-ZINCIR-01..04 |
| T12 | Sunucu mühür anahtarı çalınır (defter yeniden yazılır) | Dış tanık (§4.4, AÇIK) ve anahtar yenileme (§7, AÇIK) | D-TANIK-01 (tanık seçilince) |
| T13 | Yönetici parolası sızar ya da günlüğe düşer | Parola hiçbir yere yazılmaz; testle taranır (§5.5) | G-ANAHTAR-01 |
| T14 | Parola kaybolur | Yazılı kurtarma anahtarı (§5.5) | G-KURTARMA-01 |
| T15 | Sunucu yeniden başlar, B/C akıtılır | Kilitli ana anahtar; B/C dağıtılmaz (§5.5) | G-ANAHTAR-02 |
| T16 | İşçi payı aşar, kasayı yavaşlatır | İş nesnesi, eşikler, kapatma (§11) | L-PAY-01..03 |
| T17 | Hizmet reddi: bir işçi çok iş çeker ya da sunucuyu yoklar | İstek, boyut ve süre kotası (öneri); işçi başına eşzamanlı kira üst sınırı. AÇIK: kota sayıları | L-KOTA-01 |
| T18 | Zehirli sonuç (kasıtlı yanlış OCR metni) | Doğrulama (§2.7) ve R7 (taslak, insan onayı); **tam çözüm yok**: iki aynı sonuç doğruluk kanıtı değildir | V-ORNEK-01 |
| T19 | Yerel yönetici, fidye yazılımı, disk dolması | İşaret koşulları (R4), eşikler; işçi yalnız şifreli veri tutar | G-ISARET-01 |
| T20 | Ağ dinleme / ortadaki adam | Yalnız VPN içinden; zarf yolda korur; HTTPS katmanı ayrıca (AÇIK: işçi-sunucu taşıma güvenliğinin seçimi DG2.3) | G-ZARF-01 |
| T21 | İçeriden yönetici kötüye kullanımı | `isci.isaretlendi`, `limit.degisti` gibi eylemler deftere yazılır; yönetici eylemleri imzalı olmalı (AÇIK: yönetici kimliği ve imzası) | D-YONETICI-01 |
| T22 | Eski sürümle yeniden oynatma / sürüm düşürme | Zarf ve imza sürüm bağlar; imzalı güncellemede sürüm düşürme koruması (kurgu araştırması) | G-SURUM-01 |

### 12.3 Bilinen sınırlar
Şifreleme işleme anında korumaz (§9.6). Çift çalıştırma doğruluk kanıtı değildir. Defterin dış tanığı seçilmeden tek-sunucu kurcalanmaz sayılmaz. Windows'ta limit, ağsızlık ve sandbox denenmedi. Dağıtmak doğruluğu artırmaz. Kazanç mütevazı olabilir (15 bilgisayarda yüzde 15 payla bir iki bilgisayarlık güç); Kapı 1 bu yüzden var.

## 13. Kabul testleri
Test adları uygulamada aynen kullanılır. "Kapı" sütunu ilk geçmesi gereken kapıdır. Test verisi uydurmadır; gerçek müşteri, personel ve şube verisi kullanılmaz.

### 13.1 Kuyruk ve kira (Q)
| Ad | Beklenen | Kapı |
|---|---|---|
| Q-AYNI-PARCA-01 | İki işçi eşzamanlı aynı parçayı ister; yalnız biri kira alır (çok iş parçacıklı yarış testi, veri tabanı kısıtı) | 3 |
| Q-KIRA-DUSER-01 | Kalp atışı kesilir; kira bitişinde parça `bekliyor`a döner, `deneme_no` artar, `kira.dustu` yazılır | 3 |
| Q-GEC-SONUC-01 | Kira düşmüş işçi sonucu gönderir; reddedilir, parça durumu değişmez, `sonuc.reddedildi (gec)` yazılır | 3 |
| Q-YINELENEN-01 | Aynı sonuç iki kez gelir; tek kayıt, aynı yanıt | 3 |
| Q-CELISKI-01 | Aynı kira için farklı `sonuc_sha256`; ikincisi reddedilir, `sonuc.celisti` yazılır | 3 |
| Q-BELIRTEC-01 | Yanlış belirteç ya da başka işçinin belirteci reddedilir | 3 |
| Q-KAPANMA-01 | Şube işçisi iş ortasında kapanır; parça 5 dakikada sunucu işçisine geçer | 3 |
| Q-SUNUCU-KAPANMA-01 | Sunucu kapanıp açılır; biten parçalar yeniden yapılmaz, yarım kalan yapılır; iş başarısız sayılmaz | 3 |
| Q-SUNUCU-ISCI-01 | Sunucu işçisi çalışırken şube isteklerinin yanıt süresi sınırı aşmaz | 3 |
| Q-IPTAL-IS-01 | İş iptalinde kiradaki işçinin sonucu reddedilir | 3 |

### 13.2 Güvenlik (G)
| Ad | Beklenen | Kapı |
|---|---|---|
| G-SINIFSIZ-01 | Sınıfsız ya da tanınmayan iş türü dağıtılmaz (C sayılır, aşama kapalıyken sunucuda kalır) | 3 |
| G-SINIF-02 | `veri_sinifi` işi gönderen program tarafından değiştirilemez | 3 |
| G-ISARET-01 | İşaretsiz işçi B/C parçası çekemez; işaret koşulu bozulunca düşer | 5 |
| G-ANAHTAR-01 | Yönetici parolası ve türetilmiş anahtar günlük, defter, hata çıktısı ve disk dosyalarında bulunmaz (tarama) | 2 sonrası, 5 |
| G-ANAHTAR-02 | Yönetici anahtarı yokken B/C iş dağıtılmaz; A akar; yeniden başlatma sonrası kilitli | 5 |
| G-KURTARMA-01 | Kurtarma anahtarıyla ana anahtar açılır; parolasız ve kurtarmasız açılamaz | 5 |
| G-ZARF-01 | İmzasız, değiştirilmiş ya da başka işe/parçaya/kiraya/işçiye bağlı zarf açılmaz (bağlanan her alan için ayrı durum) | 3 |
| G-SAHTE-SONUC-01 | İmzasız, başka işe bağlı, biçime uymayan sonuç reddedilir | 3 |
| G-IPTAL-01 | İptal edilmiş cihaz iş çekemez, kalp atışı ve sonuç gönderemez; kiraları düşer | 3 |
| G-KOPYA-01 | Eski anahtarla (yenilemeden sonra) gelen istek reddedilir; eski anahtar tarihiyle saklı olduğu için eski kayıtlar doğrulanır | 3 |
| G-CALISTIRICI-01 | Bozuk/zararlı girdi çalıştırıcıyı aşamaz: ağ erişimi yok, bellek ve süre sınırı uygulanır, alt süreç iş nesnesi dışına çıkamaz. **Gerçek Windows'ta koşulur** | 3 |
| G-DISK-01 | İşçi diskinde açık hâl ve okunan metin kalmaz (iş sonrası dosya sistemi taraması; B/C'de şifreli gövde de silinmiş) | 5 |
| G-KARISMA-01..03 | İki ve daha çok şubeli sahte belgelerle X'in sayfası Y'de okunur ve sonuç X'e düşer (01); etiketi bozulmuş sonuç ve yanlış kira belirteci reddedilir (02); işçi belgenin şubesini paketten öğrenemez (03); Y şubesinin ekranında X'in belgesi ve sonucu görünmez; veri tabanında şube alanı zorunlu ve satır düzeyinde yalıtım var | 5 |
| G-TOPLAMA-01 | Bir işçi bir günde aynı şubenin/belgenin sınırı aşan sayıda sayfa alamaz; aşım `sinir.asildi` üretir (R2) | 5 |
| G-TASLAK-01 | Kişisel verili sonuç "onay bekleyen" olarak yazılır; kendiliğinden kesin kayıt olmaz (R7) | 5 |
| G-YORUM-01 | Kişisel verili sonuç dış yapay zekâ servisine gönderilmez (ağ çağrısı yok, yerel yorumcu) (R8) | 5 |
| G-SURUM-01 | Farklı iş türü sürümüyle gelen sonuç ve zarf reddedilir | 3 |

### 13.3 Defter (D)
`crates/taa-zincir` testleri DG2a'da yazılı ve geçiyor (12 test; bkz. §4.2). Dağıtıcıya özel ek:
| Ad | Beklenen | Kapı |
|---|---|---|
| D-ZINCIR-01 | Araya kayıt sokma yakalanır (mevcut test: `araya_kayit_sokma_yakalanir`) | geçti |
| D-ZINCIR-02 | Kayıt silme yakalanır (`silme_yakalanir`, `kisalan_defter_muhurle_uyusmaz`) | geçti |
| D-ZINCIR-03 | İçerik değişikliği ve yeniden yazılmış defter mühürle uyuşmaz (`icerik_degisikligi_yakalanir`, `yeniden_yazilmis_defter_muhurle_uyusmaz`) | geçti |
| D-ZINCIR-04 | Mühür alanı değişirse imza düşer, başka anahtarla imza reddedilir | geçti |
| D-BIRLIKTE-01 | Parça durumu değişikliği ve defter kaydı aynı işlemdedir; biri yazılıp ötekinin yazılmadığı durum oluşmaz (işlem geri alma testi) | 3 |
| D-DAGITIM-01 | Her `parca.verildi` kaydı sayfa kimliği, işçi ve zaman içerir, içerik içermez; B/C aşamasında kayıtsız dağıtım yapılamaz (R3) | 5 |
| D-MUHUR-ZAMAN-01 | Az kayıt olan günde de süreye bağlı mühür atılır (N ve T seçilince) | 3 |
| D-ARSIV-01 | Aylık arşiv sonrası zincir devamı doğrulanır (§4.3 AÇIK kapanınca) | 3 |
| D-TANIK-01 | Dış tanığa yazılan mühür, sunucudaki defter yeniden yazıldığında uyuşmazlık gösterir (tanık seçilince) | 3 |
| D-YONETICI-01 | Yönetici eylemleri imzalı ve deftere yazılı | 3 |

### 13.4 Limit (L)
| Ad | Beklenen | Kapı |
|---|---|---|
| L-PAY-01 | Toplam işlemci payı (tüm alt süreçler toplamı) ayarı aşmaz; Windows'ta iş nesnesiyle | 3 |
| L-PAY-02 | Ölçülmemiş bilgisayara ve ölçülmemiş basamağa iş verilmez | 3 |
| L-PAY-03 | Pay artışı onaysız uygulanmaz; azaltma ve durdurma otomatiktir | 3 |
| L-ESIK-01 | Boş bellek, pil, disk eşiği altında işçi yeni iş almaz | 3 |
| L-KAPAT-01 | Kapatma düğmesi saniyeler içinde durdurur (AÇIK: üst sınır) | 3 |
| L-KOTA-01 | İstek, boyut ve süre kotası aşımı reddedilir (AÇIK: sayılar) | 3 |
| L-KASA-01 | Kasa programı açıkken, ölçülmüş payda temsilî işlem gecikmeleri kilitli eşiğin içinde (Nebim V3 POS ve yazarkasa bağlantısı; DG1b) | 1, 4 |

### 13.5 Doğrulama (V)
| Ad | Beklenen | Kapı |
|---|---|---|
| V-BICIM-01 | Biçime uymayan sonuç reddedilir | 3 |
| V-ORNEK-01 | Örnek yeniden hesaplama yanlış sonucu yakalar (oran AÇIK) | 3 |
| V-BITBIT-01 | `bit_bit_ayni` işareti iki gerçek bilgisayarda farklı işlemciyle denenmeden açılamaz | 3 |
| V-KARANTINA-01 | Tek uyuşmazlık işçiyi karantinaya almaz | 3 |

## 14. Kapılarla ilişki
| Kapı | Bu belgeyle ilgisi |
|---|---|
| 1 | Ölçüm kazancı; L-KASA-01; eşik sayıları YONTEM.md'de kilitli |
| 2 | Bu belgenin Fable onayı: K1-K7 karşılanmış, §0 tablosu; kütüphane seçimi (Y3), bellekte tutma süresi, A sınıfı anahtar ayrımı, `parca_no` yerine `parca_kimligi` kararı |
| 3 | Q, G (A sınıfı), D, L, V testleri; Windows'ta G-CALISTIRICI-01 |
| 4 | Pilot şube: kasa şikâyeti ve eşik ihlali yok; kapatma düğmesi |
| 5 | B/C aşaması: G-ISARET, G-ANAHTAR, G-KARISMA, G-TOPLAMA, G-TASLAK, G-YORUM, D-DAGITIM; R9 avukat görüşü |

## 15. AÇIK maddeler özeti
Yazarken tahmin edilmeyen, onay ya da ölçüm bekleyenler (numaralar yalnız bu belgede başvuru içindir):
1. Zarf kütüphanesi (HPKE uygulaması) ve parola türetme kütüphanesi; çalışma alanında yok, yeni bağımlılık kararı Yusuf'a ait (§5.2, §5.5).
2. A sınıfı zarf zorunluluğu ve A anahtarlarının yönetici parolasından bağımsızlığı (§5.4, §5.5).
3. `parca_no` yerine `parca_kimligi` ve B/C'de parçaya özel iş kimliği (§5.3, §9.5).
4. Parça bildiriminin işçide doğrulanması: manifest bölme biçimi (§2.2).
5. Mühür N ve T değerleri; dış tanık nedir; mühür anahtarının ayrılması; arşiv sonrası zincir devamı için crate eksikliği; şube kimliğinin deftere yazılması; kanonikleştirme (JCS) kararı (§4).
6. Kira için toplam süre üst sınırı; "belirgin kısa" sayısı; deneme üst sınırı; karantina eşiği (§1.4, §2.3, §2.5).
7. Anahtarın bellekte kalma süresi; yanlış parola sınırı; kurtarma anahtarı biçimi ve saklama yeri; ana anahtar yenileme (§5.5).
8. Merkez anahtarlarının çalınması durumunda kurtarma ve işçilerin yeni anahtarı güvenle alması (§7).
9. İşçi ilk kayıt güven noktası; işçi numarası ile imza anahtarı bağı; yönetici kimliği ve imzası (§2.1, §6.3, T21).
10. Windows: anahtar deposu, AppContainer ile OCR motorunun uyumu, iş nesnesi, disk şifresi ve güncel Windows doğrulaması, sayfa dosyası riski. Hiçbiri denenmedi (§8, §9.3).
11. Kasa gecikmesini işçinin nasıl göreceği; "200 ms" yorumu; disk eşiği; ısıl sensör doğrulaması; sunucu yanıt süresi sınırı; kapatma süresi; bellek ve ölçülmemiş gece basamakları (§8.5, §11).
12. R1 "taşan iş" eşiği; R2 sayfa sınırları; R6 bölge ayırmanın doğruluğa etkisi; R9 avukat görüşü (§9.4).
13. Geç ama henüz yeniden verilmemiş parça sonucunun kabulü (verim): Yusuf kararı; plan reddi söylüyor (§2.6).
14. Geçici işçi önbelleğinin silinmesinin "kalıcı silme yok" kuralıyla ilişkisi: Yusuf onayı (§10).
15. Doğrulama: örnekleme oranı ve OCR karşılaştırma ölçütü (§2.7).
16. Yoğun kalp atışı ve defter yazma hızı (§4.5); işçi-sunucu taşıma güvenliği seçimi (T20); iptal edilen cihaza verilmiş parçaların yeniden şifrelenmesinin kapsamı (§7).

## 16. Denetim notu
**Fable denetimi bekliyor.** Bu belge DG0 taslağıdır; Kapı 2 ancak Fable'ın K1-K7'yi (ve Y2, Y3, R1-R9) bu belgede karşılanmış bulup yazılı onay vermesiyle geçilir. O zamana kadar dağıtıcı, işçi ve zarf kodu yazılmaz. Denetimde özellikle bakılması istenenler: §15 madde 1-3 (kütüphane, A sınıfı zarf, kimlik bağlama), §4.3 arşiv zinciri eksikliği, §8.2 Windows araçlarının tasarım okuması olması, §9.5 toplama riski. Yazan: [Claude#dg0-sozlesme 2026-11-26].

## 17. Fable denetimi düzeltmeleri (2026-11-26; bloklayıcılar)

Bu bölüm denetimin B-1..B-6 ve B-8 bulgularını bağlayıcı kural olarak yazar. Önceki bölümlerle çelişirse **bu bölüm geçerlidir**; ilgili satırlar Kapı 2 onayından önce yerinde düzeltilecektir. Kapı 2 onayı hâlâ yoktur.

- **D-1 (B-1, §2.5) iki sayaç:** `deneme_no` her kirada artar (sınırsız, yalnız sıra bilgisi). `hata_sayisi` yalnız çalıştırıcı hatasında ve doğrulama reddinde artar; `basarisiz` yalnız `hata_sayisi` üst sınırıyla (başlangıç: 3 hata, en az 2 farklı işçide) olur. Kapanma ve kira düşmesi parçayı `basarisiz` yapmaz.
- **D-2 (B-2, §2.6) sonuç kabul sırası:** önce (`parca_kimligi`, `deneme_no`, `sonuc_sha256`) kayıtlı mı bakılır (kayıtlıysa aynı yanıt döner); kayıtlı değilse "kira aktif" koşulu uygulanır. Test Q-YINELENEN-01 bu sırayı sınar.
- **D-3 (B-3, §7, §2.2, §5.4) iptalde yeniden şifreleme yok:** manifestteki `girdi_sifreli_sha256` değişmez; "iptal sonrası yeni parça anahtarı" kuralı kaldırıldı. Yerine olay raporu (R3). Gerekirse ayrı "manifest düzeltme kaydı" tasarlanır.
- **D-4 (B-4, §6.1, §5.5) iki zarf anahtarı:** A sınıfı sunucu zarf anahtarı parolasız açılır (işletim sistemi korumalı depo); B/C zarf anahtarı ana anahtarla sarılıdır. "A kilitli sunucuda akar" kuralı bu sayede tutar.
- **D-5 (B-5, §2.4, §6) istek imzası:** her istek imzası uç adını, gövde özetini, `kira_kimligi`'ni ve artan sayaç ya da sunucu tek kullanımlık değerini kapsar; TLS zorunludur. Test G-TEKRAR-01: kaydedilmiş kalp atışı yeniden gönderilirse reddedilir, kira uzamaz.
- **D-6 (B-6, §5.2-§5.4) gönderen doğrulama:** HPKE Base modu göndereni doğrulamaz. Kira yanıtı (belirteç özeti, sarılmış anahtarın özeti, §5.3 alanları) sunucu manifest anahtarıyla imzalanır; işçi imzayı doğrulamadan zarfı açmaz. Sonuç zarfı (işçi → sunucu) için §5.3'ün eşi yazılacaktır (alan listesi DG0 son halinde).
- **D-8 (B-8, §9.3, §12.1) işaret koşulları dürüst sınırı:** disk şifresi, güncel Windows ve hesap türü işçinin kendi beyanıdır; işçi güvenilmeyen sınıftadır. Beyan hijyen denetimidir, güvenlik kanıtı değildir; işareti veren yöneticinin yerinde doğrulaması şarttır (Çatı'da onay kutusu + defter kaydı).

**Açık kalanlar (Yusuf kararı gerekir):** B-7 (Kapı 1 kazanç eşiğinin sayısı ve "5 geçerse pay yok" kuralı; `YONTEM.md` kilit özetiyle birlikte güncellenecek), Y-1 yeni bağımlılık (HPKE, Argon2id), Y-2 dış tanık, Y-3 merkez anahtarı kurtarma, Y-8 önbellek silmenin "kalıcı silme yok" kuralına girmediği, `ocr.genel` akış listesi. Ayrıntı ve Fable önerileri denetim raporunda.

### D-9 Kriptografi kütüphaneleri (koordinatör kararı 2026-11-27, Yusuf değiştirebilir)
HPKE (RFC 9180) ve Argon2id için yerleşik, bakımı süren hazır kütüphaneler kullanılır; kripto el ile yazılmaz. Somut paketler uygulama aşamasında sürüm ve lisans denetimiyle seçilir.

### D-10 Dış tanık (Yusuf kararı 2026-11-27)
Zincir mührü özetleri iki bağımsız yerde tutulur: Yusuf'un telefonu ve ikinci bir bilgisayar. İkisi de sunucu yöneticisinin denetimi dışındadır. Biri sunucudakiyle uyuşmazsa uyarı verilir.

### D-11 Merkez anahtarı çalınırsa (Yusuf kararı 2026-11-27)
Çevrimdışı saklanan yedek anahtarla yeni merkez anahtarı üretilir. Eski kayıtlar dış tanıkların (telefon, ikinci bilgisayar) özetleriyle doğrulanır ve yeni anahtarla yeniden mühürlenir. Yedek anahtar şifreli ve sunucu dışında tutulur; kayıt silinmez.

### D-12 İşçi önbelleği (Yusuf kararı 2026-11-27)
İşçi bilgisayardaki geçici, yeniden üretilebilir önbelleğin program tarafından temizlenmesi "kalıcı silme yok" kuralına girmez. Kullanıcı dosyalarına, kayıtlara ve zincir verisine bu kural dışında dokunulmaz.

### D-13 ocr.genel kapsamı (Yusuf kararı 2026-11-27)
Tüm belge türleri `ocr.genel` havuzuna girebilir. Kişisel veri içeren belgeler yine yalnız HPKE ile şifreli paketlenir ve işçiye kısa süreli kirayla gider; işçi çözülmüş içeriği diske yazmaz, iş bitince bellekten siler (§17 D-3, D-4 ile birlikte okunur).
