<!-- Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-yazdir 2026-11-27]. Durum: TASARIM (kod yok). Değişiklik: Yusuf'un onayıyla. -->
# Şube yerel düğümü (W2)

**Karar (Yusuf 2026-11-27 10:55):** "Evet, şube yerel düğümü." Şubede internet kopunca aynı şubedeki cihazlar birbirini
görmeye devam eder. Plan: `PLAN-TAA-WEB-2026-11-27.md` §2 katman 3, §8.
Bağlı belgeler: `docs/WEB-ESITLE-SOZLESMESI.md`, `docs/esleme-sozlesmesi.md` (EB0), `docs/sube-sozlesmesi.md` (Ş0),
`docs/sube-tehdit-modeli.md`, `docs/T6-KUYRUK.md`, `docs/S2-CIHAZ.md`, `docs/SUBE-TELEFON-YAZICI-PLANI.md`.

Bu belge tasarımdır; düğüm kodu yazılmadı. taa-sunucu kodu okunmadı (bkz. `WEB-ESITLE-SOZLESMESI.md` baştaki not).

## 1. Ne yapar, ne yapmaz
Düğüm, şube PC'lerinden birinde çalışan yerel hizmettir. Şubedeki tarayıcılara (telefon, tablet, kurulumsuz PC) üç şey sunar:
1. **TAA Web kabuğu ve modül dosyaları** (merkez yokken de sayfa ilk kez açılabilsin diye).
2. **Yazma uçları** `POST /v1/<uc>`: kaydı yerel deftere ekler, merkeze gidecek kuyruğa koyar, hemen yanıt verir.
3. **Okuma ucu** `GET /v1/esitle?sonra=`: şubenin yerel kopyasından delta. Böylece bir telefonun eklediği fiş, internet yokken de öteki telefonda görünür.

Yapmaz:
- Merkezin yerini tutmaz. Yetkili kaynak merkezdir; düğümdeki kopya şubenin kendi verisidir, başka şubenin verisi düğüme hiç gelmez.
- Kişi yetkisi vermez. Yetki merkezden gelen imzalı belgeyle belirlenir (§6).
- Düğüm olmasa da her cihaz tek başına çalışır (tarayıcıdaki giden kutusu). Düğüm bir kolaylıktır, zorunlu parça değildir.

## 2. Mevcut sözleşmelerle ilişki
| Mevcut | Düğümle ilişkisi |
|---|---|
| **Eş kapısı (EB0 §5):** aynı mağazadaki *kurulu* PC'ler arası eşleme, karşılıklı cihaz anahtarı, TLS | Değişmez. Düğüm eş kapısının yerine geçmez; tarayıcı eş değildir ve eş kapısına bağlanamaz (`telefon_es_kapisina_baglanamaz`) |
| **Telefon sunucusu (E8):** sıra tutan eşte çalışır, kapısı eş kapısından ayrıdır | Düğüm **budur**: `taa-fis-sunucu`nun telefon kapısı genişler. Yeni bir üçüncü kapı açılmaz |
| **`taa-kuyruk` (T6):** merkeze giden yerel kuyruk | Düğüm aldığı her kaydı aynı `no` ile buraya ekler; gönderimi mevcut `taa-kimlik gonder` yolu yapar |
| **Tek yazar (veri-sahipleri.toml)** | Düğüm bir taşıyıcıdır, veri alanı sahibi olmaz. Aldığı kaydı sahibi olan modülün işlevine verir (örnek: fişi `taa-fis` defterine `taa-fis-sunucu` yazar) |
| **"Şube bilgisayarında açık kapı olmaz" (ana plan §14)** | Telefon sunucusu kapısı zaten var olan, yerel ağla sınırlı kapıdır. Düğüm bu kapıyı büyütür; §7'deki sınırların hepsi şarttır |

Çok bilgisayarlı mağazada düğüm sıra tutan eştir. Öteki eş PC'ler kendi aralarında eş kapısından eşlenir (EB0 §4);
düğüme tarayıcıdan gelen kayıt sıra tutan eşin defterine yazıldığı için eş kapısı üzerinden öteki PC'lere de ulaşır.
Tek bilgisayarlı mağazada düğüm o bilgisayardır; eş kapısı açılmaz, düğüm kapısı yalnız "telefon/tarayıcı kullanılacak" seçilirse açılır.

## 3. Temel: `taa-fis-sunucu`
Bugün var olan (koddan):
- `tiny_http` ile HTTP sunucusu, port varsayılan 8787; telefon sayfası (PWA manifestli), masaüstü sayfası.
- Telefon eşleştirme: çifte kod + yönetici onayı; belirteç yalnız SHA-256 özeti olarak saklanır; eklemeli JSONL cihaz kaydı.
- Şube kodu `<dizin>/sube.json`; fiş defteri; `taa-kuyruk` ve `taa-kimlik` bağımlılığı; `--sunucu`, `--sertifika` seçenekleri.
- Yazdırma: `POST /api/fis/{no}/yazdir` -> `yaziciya_yaz` (dosya yolu ya da `tcp://IP[:9100]`).

Eksik olan (düğüm için eklenecek):
- Kabuk dosyalarını sunma, `/v1/<uc>` ve `/v1/esitle` uçları, yerel kopya ve sürüm sayacı, çakışma kaydı.
- **HTTPS.** Bugün düz HTTP ve `0.0.0.0` dinleniyor. Tarayıcıda service worker, IndexedDB kalıcılığı ve WebCrypto
  güvenli köken ister; düz `http://192.168.x.x` adresinde TAA Web çevrimdışı çalışamaz (§5).
- Dinleme adresinin yerel ağ arayüzüyle sınırlanması (§7).

Öneri: düğüm işlevleri `taa-fis-sunucu` içine modül olarak eklenir (ayrı ikili değil), çünkü cihaz kaydı, şube kodu ve
defter orada. Crate büyürse düğüm çekirdeği (yerel kopya + sürüm sayacı) ağdan bağımsız ayrı bir ortak crate'e alınır.

## 4. Veri akışı
```
tarayıcı (giden kutusu) --POST /v1/<uc>--> DÜĞÜM --taa-kuyruk (aynı no)--> MERKEZ
tarayıcı (önbellek)     <--GET /v1/esitle-- DÜĞÜM <--GET /v1/esitle (imzalı)-- MERKEZ
```
### 4.1 Yazma
1. Tarayıcı kaydı `no` ile gönderir (`WEB-ESITLE-SOZLESMESI.md` §4).
2. Düğüm denetler: eşleşmiş cihaz, geçerli yetki belgesi, şema, şube damgası = düğümün şubesi.
3. Aynı `no` daha önce geldiyse: içerik aynıysa `zaten_var`, farklıysa 409 `cakisma` (iki hal saklanır).
4. Yeni kayıtsa **tek adımda** yerel deftere eklenir ve yerel sürüm sayacı artar; ardından `taa-kuyruk`a aynı `no` ile eklenir (`Ekleme::ZatenVar` zararsızdır). Deftere yazılıp kuyruğa eklenemeden çökülürse açılışta defter-kuyruk karşılaştırması eksik kaydı kuyruğa ekler.
5. Yanıt 200 `{"durum": "yazildi", "surum": <yerel sürüm>, "kesin": false}`. `kesin: false`, kaydın düğümde olduğunu ama merkezin henüz onaylamadığını söyler.
6. İnternet gelince kuyruk merkeze gönderilir. Merkez aynı `no`yu ikinci kez yazmaz. Merkez kalıcı reddederse (4xx) kayıt `reddedildi.jsonl`e düşer ve düğüm bunu deltada o kaydın `durum: "merkez_reddetti"` haliyle yayar; şube müdürü bakar.

Tarayıcı açısından düğümün 2xx yanıtı "gönderildi" demektir. Bu yüzden düğüm, kaydı diske yazmadan 2xx dönmez
(`taa_dosya::satir_ekle` diske yazar) ve düğüm diski `taa-yedek` kapsamındadır.

### 4.2 Okuma
- Düğüm şubenin yerel kopyasını tutar: kendi aldığı kayıtlar + merkezden en son aldığı imzalı delta.
- Tarayıcıya verdiği delta **yerel sürüm sayacı** ile numaralanır ve düğümün cihaz anahtarıyla imzalanır (alan ayrımı etiketiyle, örnek `TAA-DUGUM-v1`; eş imzası ya da istek imzası yerine kullanılamasın diye, EB0 §5.1 madde 3 ilkesi).
- Merkez kaynaklı kayıtlar deltada merkezin özgün imzalı belgesine başvuruyla taşınır; tarayıcı merkez anahtarını kabuktan tanır.
- İnternet varken düğüm merkezden `GET /v1/esitle` ile kendi kopyasını yeniler (çekme modeli; merkez düğüme bağlanmaz).

### 4.3 Yerel kopya
Klasör `<veri kökü>/dugum/` (yeni veri alanı; sahibi düğümü taşıyan crate, `veri-sahipleri.toml`a eklenmeli):
| Dosya | İçerik |
|---|---|
| `kayit.jsonl` | `{"surum", "no", "uc", "zaman", "kaynak", "ozet", "govde"}`; yalnız ekleme |
| `merkez.jsonl` | Merkezden alınan imzalı delta belgeleri, olduğu gibi |
| `cakisma.jsonl` | Aynı `no` / aynı öncülden doğan ikinci haller ve seçim olayları |
| `durum.json` | Son merkez sürümü, son başarılı merkez bağlantısı, yerel sürüm sayacı (atomik yazım) |

Hiçbir satır silinmez ya da ezilmez. Dosyalar Ş0 §8'deki gibi DPAPI ile sarılır (müşteri verisi içerir).

## 5. Köken ve HTTPS (en zor kısım)
Tarayıcıda veri kökene (şema + ad + port) bağlıdır. İki sonuç:
1. Düğüm `https` ile ve tarayıcının güvendiği bir sertifikayla sunulmalı; yoksa çevrimdışı çalışma hiç başlamaz.
2. Merkez ve düğüm farklı kökenlerse **ayrı** giden kutuları olur; bir kökende biriken kayıt ötekinde görünmez.

| Seçenek | Nasıl | Artı | Eksi |
|---|---|---|---|
| **A. Şubede her zaman düğüm** (öneri) | Şubedeki tarayıcılar internet olsa da olmasa da düğümün adına bağlanır (örnek `mrk01.sube.<alan adı>`); düğüm merkeze taşır. Sertifika gerçek bir alan adı için, merkezce alınıp düğüme dağıtılır | Tek köken, tek giden kutusu; internet gidip gelince tarayıcı hiçbir şey fark etmez | İnternet yokken adın yerel ağda çözülmesi gerekir (modemde yerel ad kaydı ya da düğümün yerel ad sunucusu olması). Sertifika süresi dolmadan yenilenmeli; şube uzun süre kopuksa sertifika eskir |
| B. Kendi kök sertifikamız | Düğüm kendi CA'mızdan sertifika alır; kök sertifika her telefona kurulur | Alan adı ve ad çözümü derdi yok (IP ile de olur) | Her cihaza sertifika kurmak "kurulum derdi olmasın" hedefine ters; iPhone'da profil adımı zor |
| C. Düz HTTP + yerel IP | Bugünkü durum | Kolay | Service worker ve WebCrypto çalışmaz; TAA Web için **olmaz** |

A seçilirse mağaza dışındaki kullanım (ev, yol) merkez kökeninden olur; orada ayrı bir giden kutusu birikir ve kendi
kökeninde boşalır. Kayıt kaybı olmaz (`no` tekildir), ama "gönderilmeyen N kayıt" göstergesi köken başınadır.

`taa-yazdir` ayarındaki `izinli_kokenler` listesine düğümün kökeni de yazılmalıdır.

## 6. Kimlik ve yetki (internet yokken)
- **Cihaz:** tarayıcı düğüme mevcut eşleştirmeyle bağlanır (çifte kod + yönetici onayı; belirteç özeti saklı). Eşleşmemiş cihaz hiçbir `/v1` ucuna ulaşamaz.
- **Kişi:** düğüm parola doğrulamaz (parola özetleri merkezdedir ve şubeye inmez). İnternet yokken yalnız, daha önce merkezde giriş yapmış ve **7 günlük imzalı yetki belgesi** geçerli olan kişi çalışır. Düğüm her yazmada belgeyi doğrular: merkez imzası, süre, şube = düğümün şubesi, izin listede.
- Belgesi dolan kişi internet gelene kadar yeni kayıt gönderemez; tarayıcısındaki giden kutusu bekler (kayıt kaybolmaz).
- Düğüm yönetici izni vermez; yerel PIN yönetici izni sağlamaz (Ş0 §9.2 madde 6).
- Merkez, düğümün ilettiği her kaydı yeniden denetler (belge kimliği iptal mi, eylem zamanı belge aralığında mı; Ş0 §9.2 madde 5). Uymayan kayıt "çelişkili" işaretiyle saklanır.

## 7. Güvenlik sınırları
1. **Yalnız yerel ağ arayüzü:** düğüm kapısı `0.0.0.0` yerine yerel ağ arayüzünün adresinde dinler; Tailscale arayüzünde (100.64.0.0/10) ve Genel ağ profilinde dinlemez. Bağlanan adres yerel alt ağda değilse bağlantı kimlik denetimine geçmeden kapanır (EB0 §5.1 madde 2 ile aynı kural).
2. **Güvenlik duvarı kuralını TAA Kur kurar:** gelen yön, yalnız düğüm exe'si, tek sabit TCP kapısı, yalnız Özel profil, uzak adres yalnız yerel alt ağ. Modemde kapı yönlendirmesi yapılmaz.
3. **Şube ayrımı:** düğümde yalnız kendi şubesinin verisi bulunur. Şube damgası uymayan kayıt uygulanmaz, deftere `dugum.red` olarak yazılır. PC başka şubeye devredilirse `dugum/` klasörü Ş0 §4 madde 7'deki gibi `eski\sube-<kod>-<tarih>\` altına taşınır (silinmez).
4. **Sınırlar:** gövde boyutu, cihaz başına istek hızı, kuyruk uzunluğu; aşımda 413 / 429, deftere yazılır.
5. **Köken denetimi:** `/v1` uçları yalnız düğümün kendi kökeninden gelen istekleri kabul eder (`Origin` ve `Host` denetimi; DNS yeniden bağlama saldırısına karşı).
6. **Çekmece ve yazdırma:** düğüm üzerinden gelen ESC/POS içeriğinde çekmece açma komutu reddedilir (`yazdir.sun.md` Kural 12).
7. **Tazelik:** merkezle son başarılı bağlantı 5 günü geçince ekranda uyarı; 7 günü geçince yetki belgeleri de dolmuş olacağından düğüm yalnız okuma verir (EB0 E10 ile aynı sınır).
8. **Dürüst sınır:** şube ağına izinsiz bağlanan cihaz düğüm kapısına ulaşır ve eşleştirme ekranını görür; eşleşmeden veri alamaz. Şubede çalınan, eşleşmiş ve belgesi geçerli bir telefon 7 güne kadar kayıt ekleyebilir; cihaz Çatı'dan iptal edilince düğüm bir sonraki merkez bağlantısında öğrenir.

## 8. Çakışma
- Ekleme tipi kayıt (fiş, fiş olayı, kasa sayımı, tutanak, masraf): `no` ile tekildir, birbirini ezmez.
- Aynı `no`, farklı içerik; ya da aynı kaydın aynı öncülden iki farklı sürümü: **çakışmadır, iki hal saklanır, insan seçer.** Otomatik birleştirme ve "son yazan kazanır" yoktur (Ş0 §10, EB0 §4.3).
- Düğüm çakışmayı kendi çözmez: iki hali `cakisma.jsonl`e yazar, ikisini de merkeze iletir, deltada "iki hali var" işaretiyle yayar. Seçim (`cakisma.secim`) şube müdürü ya da yöneticinin eylemidir ve o da eklemeli bir kayıttır.
- Merkezin daha yeni sürümü ile şubede çevrimdışı yapılan değişiklik çatışırsa merkezdeki hal seçim yapılana kadar geçerlidir, şubedeki değişiklik çakışma kutusunda durur (`WEB-ESITLE-SOZLESMESI.md` §5).
- **Belge numarası:** numaralı belge (fiş, tutanak) numarasını yalnız sıra tutan verir (EB0 §3). Düğüm sıra tutan eşte çalıştığı için tarayıcıdan açılan fiş numarasını düğümden alır; tarayıcı kendi numara üretmez. Tarayıcı düğüme de ulaşamıyorsa kayıt giden kutusunda numarasız (yalnız `no` ile) bekler.

## 9. Kurulum ve işletim
- Düğüm ayrı bir kurulum değildir: `taa-kur`daki TerziTakip/telefon sunucusu paketinin bir seçeneğidir ("Bu bilgisayar şubenin yerel düğümü olsun").
- Kurucu sırasıyla: şube kodunu onaylı cihaz kaydından alır (`sube.json`), ağ profilini denetler (Genel ise kural kurmaz ve söyler), güvenlik duvarı kuralını kurar, sertifikayı merkezden alır (§5 seçenek A), kapıyı sınar, telefon için karekodu gösterir.
- Kurulum sınaması: aynı ağdaki ikinci cihazdan düğüme ulaşılabiliyor mu (kablosuz ağda cihaz yalıtımı açıksa ulaşılamaz; EB0 §5.5 son madde).
- Çalışma biçimi: kullanıcı oturumunda açılışta başlayan arka plan süreci (EB0 E7 ile aynı karar).
- Durum ekranı (masaüstü sayfası ve Çatı): merkezle son bağlantı, kuyruktaki kayıt sayısı, reddedilen ve çakışan kayıt sayısı, eşleşmiş cihazlar, sertifika bitiş tarihi.
- Yedek: `dugum/` ve `kuyruk/` klasörleri `taa-yedek` kapsamına girer. Düğüm PC'si bozulursa merkeze ulaşmamış kayıtlar yalnız bu yedekte ve tarayıcıların önbelleğindedir.
- Taşıma: veri klasörü + ayar tek klasördedir; yeni PC'ye geçişte cihaz anahtarı taşınmaz, yeni PC Çatı'dan yeniden onaylanır.

## 10. Kabul testleri (düğüm yazılırken)
| Test | Beklenen |
|---|---|
| `dugum_internet_yok_iki_telefon` | İnternet kesik: A telefonu fiş ekler, B telefonu deltada görür |
| `dugum_internet_gelince_tek_kayit` | Aynı kayıt hem düğüm kuyruğundan hem tarayıcıdan merkeze giderse merkezde tek satır |
| `dugum_cift_gonderim` | Aynı `no` düğüme iki kez -> tek satır, `zaten_var` |
| `dugum_ayni_no_farkli_icerik` | 409 `cakisma`; iki hal `cakisma.jsonl`de |
| `dugum_cokus_defter_kuyruk` | Deftere yazıldıktan sonra çöküş: açılışta kuyruğa eklenir, kayıp yok |
| `dugum_baska_sube_damgasi_red` | Uygulanmaz, `dugum.red` yazılır |
| `dugum_eslesmemis_cihaz_red` | `/v1` uçları 401 |
| `dugum_belge_dolmus_red` | Süresi geçmiş yetki belgesiyle yazma reddedilir; tarayıcıda kayıt bekler |
| `dugum_tailscale_ve_genel_profilde_kapali` | Kapı dinlemez |
| `dugum_alt_ag_disi_red` | Yerel alt ağ dışından bağlantı kimlik denetiminden önce kapanır |
| `dugum_yabanci_koken_red` | Başka kökenden gelen istek reddedilir |
| `dugum_merkez_reddi_gorunur` | Merkezin 4xx verdiği kayıt şube ekranında "merkez reddetti" diye görünür |
| `dugum_7_gun_salt_okuma` | Merkezle 7 gün bağlantı yoksa yazma kapanır, okuma sürer |

## 11. Aşamalar
- **D0:** bu belge + Yusuf'un §12 kararları.
- **D1:** yerel kopya çekirdeği (defter, sürüm sayacı, çakışma kaydı) ağdan bağımsız, testli.
- **D2:** `taa-fis-sunucu`da HTTPS, arayüz sınırı, `/v1/<uc>` ve `/v1/esitle`, kabuk sunumu.
- **D3:** `taa-kur` seçeneği, güvenlik duvarı kuralı, sertifika dağıtımı, kurulum sınaması.
- **D4:** Esenyurt'ta gerçek deneme (iki telefon, modem interneti kesilerek); EB5 ile birlikte.

## 12. Açık noktalar (Yusuf kararı gerekenler başta)
1. **Köken seçimi (§5):** A (şubede her zaman düğüm, gerçek alan adı sertifikası) mı, B (her cihaza kendi kök sertifikamız) mı? A için alan adı ve internet yokken ad çözümü (modem ayarı) gerekir.
2. **HTTPS kitaplığı:** `tiny_http` bugün TLS'siz derleniyor. EB0'da `rustls` eş kapısı için onaylandı (E9); düğüm kapısı için de kullanılması ayrıca onay ister (yeni kullanım, aynı bağımlılık).
3. **Hangi programlar düğümden geçecek:** ilk sürüm için öneri yalnız TerziTakip (fiş) ve Kasa; Tutaa ekleri ve OCR işleri merkez ister.
4. **Çevrimdışı ilk giriş:** hiç giriş yapmamış kişi internet yokken çalışamaz (parola şubede doğrulanamaz). Kabul mü?
5. **Düğüm PC'si kapalıyken:** tarayıcılar tek başına çalışır, birbirini görmez. Yedek düğüm (ikinci eş) ilk sürümde yok.
6. **taa-sunucu tarafı okunmadı:** merkezin düğümden gelen kaydı "hangi kişi, hangi belgeyle, hangi tarayıcıdan" diye nasıl saklayacağı (`kaynak_cihaz` + tarayıcı kimliği) orada kararlaştırılır.
7. **Tarayıcıda imza doğrulama** (düğüm ve merkez deltası için Ed25519) hedef telefonlarda denenmedi.
8. **KVKK:** müşteri verisinin bir kopyası düğümde ve eşleşmiş her telefonun tarayıcısında durur; saklama süresi ve telefondan kaldırma kuralı yazılmalı.
9. **Yazdırma:** düğüm üzerinden yazdırma `yazdir.sun@1` (EB2) ile mi, bugünkü `taa-fis-sunucu` yoluyla mı; `taa-yazdir` (127.0.0.1 köprüsü) yalnız yazıcının bağlı olduğu PC'deki tarayıcı içindir, telefon için değildir.

## 13. Uygulama (`crates/taa-sube-dugum`, 0.1.0)
<!-- yazan: [Claude Fable 5.2 #taa-web-W2-sube-dugum 2026-11-27]. Bu bölüm koddan yazıldı; yukarıdaki bölümler tasarımdır. -->
Yusuf'un 2026-11-27 ek kararları (plan §9) tasarımı şöyle değiştirdi; çelişkide bu bölüm geçerlidir:
- Çevrimiçiyken cihaz **doğrudan merkeze** gönderir. Yol sırası: merkez → şube bilgisayarı (bu düğüm) → cihazın kendi kutusu. §5'teki "A: şubede her zaman düğüm" önerisi geçersizdir.
- Düğüm `taa-fis-sunucu` içinde modül değil, **ayrı ikilidir**: `taa-sube-dugum`, varsayılan kapı **8788** (§3'teki öneri ve 8787 geçersiz).
- Aynı şubede çakışmada **şube bilgisayarındaki (ilk gelen) hal geçerlidir**; kaybeden çakışma kutusuna yazılır.
- Başlıktaki "kod yok" ve §1 üstündeki "düğüm kodu yazılmadı" cümleleri bu crate ile eskidi.

### 13.1 Çalıştırma
```
taa-sube-dugum calis --kok <veri kökü> [--sube MRK01] [--port 8788] [--lan 192.168.1.20] [--ad kasa-pc]
                     [--koken https://sunucu.ornek.com] [--merkez https://...] [--sertifika CA.pem]
                     [--oturum-dosyasi <yol>] [--aralik 30]
taa-sube-dugum durum --kok <veri kökü> [--sube MRK01]
```
- **Dinleme adresi:** varsayılan yalnız `127.0.0.1`. Telefonların ulaşması için `--lan <yerel ağ IPv4>` **açıkça** verilir; düğüm o adreste ikinci bir kapı açar. `0.0.0.0`, genel adres ve Tailscale (100.64.0.0/10) reddedilir (çıkış kodu 2).
- **Şube kodu:** `--sube`, yoksa `<kök>/sube.json` (`{"kod": "..."}`; TAA Kur'un yazdığı dosya).
- **Merkez adresi:** `--merkez` → `TAA_SUNUCU` → `taa-yol` yol listesi (`<kök>/yol/sunucu-yollari.json`, yollar sırayla denenir). Hiçbiri yoksa düğüm çalışır, kayıtlar birikir.
- **Merkez oturumu:** `--oturum-dosyasi` ya da `TAA_OTURUM`; her turda yeniden okunur, hiçbir yere yazılmaz.
- `--koken`: düğüme başka kökenden yazmasına izin verilen sayfa (merkez kabuğunun kökeni). Yalnız bu kökenlere CORS başlıkları ve ön uçuş (özel ağ izni dahil) yanıtı verilir.

### 13.2 Uçlar
| Uç | Davranış |
|---|---|
| `POST /v1/<uç>` | Merkez ile aynı gövde ve `X-TAA-No`. `X-TAA-Sube` zorunlu ve düğümün şubesiyle aynı olmalı. `no` başlıkta ya da gövdede olabilir; ikisi farklıysa 400 `no_uyusmaz`; gövdede yoksa eklenir. Yanıt 200 `{"no", "durum": "yazildi"\|"zaten_var", "surum", "kesin", "dugum": "sube", "sunucu_zamani"}`. Kayıt diske yazılmadan 2xx dönülmez |
| `GET /v1/esitle?sonra=<n>[&limit=1..500]` | Şube önbelleğinden delta (`WEB-ESITLE-SOZLESMESI.md` §6.2 `belge` biçimi, `kaynak: "sube"`). `X-TAA-Sube` zorunlu. Kayıt başına `durum` (`bekliyor` / `gonderildi` / `merkez_reddetti`) ve `kesin`. `imza` şimdilik `null` |
| `GET /durum` | `{"bekleyen", "gonderilen", "reddedilen", "son_merkez_zamani"}` + `sube`, `cakisma`, `surum`, `gunluk_bekleyen`, `sunucu_zamani` |
| `GET /v1/saglik` | 200; istemcinin yol denemesi için |

Ret kodları: 400 `sube_eksik` / `no_eksik` / `no_bozuk` / `no_uyusmaz` / `govde_bozuk`; 403 `ag_disi` (bağlanan adres yerel ağda değil) / `ad_uyusmaz` (`Host` düğümün adı değil) / `koken_izinsiz`; 404 `uc_yok`; 409 `sube_uyusmaz` / `cakisma`; 413 `govde_buyuk` (256 KiB); 503 `gecici` (disk yazılamadı; geçici durum için 4xx dönülmez). Şube ve ağ retleri günlüğe `dugum.red` olarak yazılır.

**Kuyruğa alınmayan uçlar** (404 `uc_yok`): `/v1/giris`, `/v1/cikis`, `/v1/yetki`, `/v1/cihaz/*`, `/v1/esitle`, `/v1/saglik`, `/v1/yol-bilgisi`, `/v1/gunluk/*`, `/v1/cakisma/*`. Giriş gövdesi parola taşır; düğüm onu hiçbir dosyaya yazmaz. İnternet yokken giriş yapılamaz (§12 madde 4 aynen).

### 13.3 Dosyalar (`<veri kökü>` altında; JSONL dosyaları yalnız eklenir, hiçbir satır silinmez)
| Dosya | İçerik |
|---|---|
| `kuyruk/sube-dugum/{giden,gonderildi,reddedildi}.jsonl` | `taa-kuyruk` kuyruğu. Gövdeler `ekle_sifreli` ile şifreli (`anahtar/kuyruk.key`) |
| `kuyruk/sube-dugum-gunluk/` | Günlük özetlerinin kuyruğu (asıl kayıtları bekletmesin diye ayrı) |
| `dugum/surum.jsonl` | Yerel sürüm sayacı `{"surum", "no", "olay": "alindi"\|"gonderildi"\|"reddedildi", "zaman"}` |
| `dugum/cakisma.jsonl` | Çakışma kutusu `{"no", "zaman", "uc", "gecerli_uc", "gecerli", "kaybeden", "kaynak", "durum": "acik"}` |
| `dugum/durum.json` | `{"son_merkez_zamani"}` (atomik yazılır; ezilen tek dosya budur) |
| `gunluk/YYYY-MM-DD.jsonl` | Günlük (UTC gün): `dugum.basladi`, `kayit.alindi`, `kayit.zaten_var`, `kayit.cakisma`, `dugum.red`, `dugum.hata`, `merkez.ulasildi`, `merkez.ulasilamadi`, `merkez.gonderildi`, `merkez.reddetti`, `gunluk.ozet_kuyrukta`. Kayıt içeriği günlüğe yazılmaz (yalnız `no`, uç, kaynak adresi) |

§4.3'teki `kayit.jsonl` ayrı tutulmadı: kayıt defteri `taa-kuyruk`un `giden.jsonl` dosyasıdır (aynı kayıt iki yerde durmasın diye). `merkez.jsonl` yok (13.6 madde 3).

### 13.4 Merkeze gönderim
- Arka plan döngüsü her `--aralik` saniyede ve her yeni kayıtta bir tur atar: bekleyenler **sırayla** `Kuyruk::gonder` ile gider. Merkez yoksa ilk kayıtta durur, sonraki tura kalır; sıra bozulmaz.
- Merkeze giden her isteğe `X-TAA-Sube` ve `X-TAA-No` eklenir.
- `taa-kuyruk` davranışına iki uyarlama (`merkez::uyarla`; `WEB-ESITLE-SOZLESMESI.md` §4.3): **401** kalıcı ret sayılmaz, kayıt bekler; **409** `cakisma` / `sube_uyusmaz` / `surum_eski` / `no_uyusmaz` kalıcıdır, kayıt `reddedildi.jsonl`e düşer ve deltada `merkez_reddetti` görünür. Öteki 409 (`tekrar`) eskisi gibi bekler.
- Gönderilecek kayıt yokken merkez `GET /v1/saglik` ile yoklanır; yanıt gelirse `son_merkez_zamani` güncellenir.
- **Günlük kopyası:** biten her günün özeti (`gun`, `satir`, `bayt`, `sha256`, `olaylar`, `ilk`, `son`, `sube`) `no = GNL-<şube>-<gün>` ile `POST /v1/gunluk/ozet` için kuyruğa girer ve merkeze ulaşınca gider. Günlük dosyası yerinde kalır. Bugünün dosyası hâlâ yazıldığı için özeti ertesi gün çıkar.
- **Çöküş onarımı:** açılışta ve her yazmada kuyruk ile sürüm sayacı karşılaştırılır; eksik sürüm satırı eklenir.

### 13.5 Kancalar (bu crate doğrulama uydurmaz)
| Kanca | Nerede | Bağlanacak |
|---|---|---|
| Merkeze giden isteklerin cihaz imzası | `merkez::baslikli(.., cihaz: Option<Arc<dyn taa_kuyruk::Imzalayici>>)`; ikili bugün `None` verir | `taa_cihaz::CihazAnahtari` bu arayüzü gerçekler; `main.rs`te tek satır + `taa-cihaz` bağımlılığı |
| Gelen isteklerin cihaz / kişi denetimi (§6: eşleşmiş cihaz, 7 günlük yetki belgesi) | `GelenDenetim` (şube denetiminden sonra, yazma ve `esitle` için çağrılır); ikili bugün `DenetimYok` verir | Telefon eşleştirme belirteci ya da cihaz imzası + yetki belgesi doğrulaması |
| Delta imzası (§4.2) | `esitle` yanıtında `imza: null` | Düğüm cihaz anahtarıyla, alan ayrımı etiketli imza |

### 13.6 Açık noktalar ve tasarımdan sapmalar
1. **Kimlik denetimi yok.** `--lan` açıkken şube ağındaki her cihaz `X-TAA-Sube` değerini bilerek yazabilir ve deltayı okuyabilir. §7 madde 8'deki "eşleşmeden veri alamaz" güvencesi **henüz sağlanmıyor**. Kancalar bağlanmadan düğüm gerçek şubede `--lan` ile açılmamalıdır.
2. **HTTPS yok** (düz HTTP, `tiny_http`). Merkez kabuğu `https` kökeninden sunulduğunda tarayıcı `http://<LAN>:8788` adresine isteği karışık içerik sayıp engeller; `--koken` ve ön uçuş yanıtı tek başına yetmez. §5 ve §12 madde 1-2 kararı gerekir. Tarayıcıdan gerçek deneme **yapılmadı**.
3. **Merkezden delta çekilmiyor.** `esitle` yalnız düğümün kendi aldığı kayıtları yayar; merkezin imzalı deltası (rehber, ortak ayar) düğümde önbelleğe alınmıyor.
4. **Merkez ucu `POST /v1/gunluk/ozet` öneridir**; taa-sunucu'da karşılığı doğrulanmadı. Merkez 404 dönerse özet `reddedildi.jsonl`e düşer; asıl kayıtlar ayrı kuyrukta olduğu için etkilenmez.
5. **Merkez kimliği:** düğüm kayıtları kendi oturumuyla gönderir; kaydı hangi kişinin hangi telefondan girdiği merkeze ayrıca taşınmıyor (§12 madde 6). Tarayıcının `Authorization` başlığı düğümde saklanmaz.
6. **Masaüstü `taa-kuyruk` istemcisi** 409'u geçici sayar: düğümün 409 `cakisma` / `sube_uyusmaz` yanıtında o istemcinin kuyruğu o kayıtta bekler. Düzeltme `taa-kuyruk`ta yapılmalı (bu crate'in dışında).
7. **§7'den uygulanmayanlar:** alt ağ maskesi denetimi (yalnız "özel adres mi" bakılır), ağ profili (Genel/Özel) denetimi, cihaz başına hız sınırı, kuyruk uzunluğu sınırı, 7 gün bağlantısızlıkta salt okuma, çekmece komutu süzgeci (düğüm yazdırmaz).
8. **§8 belge numarası** ve `onceki_surum` (düzenlenebilir kayıt) düğümde işlenmiyor; yalnız "aynı `no`, farklı içerik" çakışması var. Çakışma seçimi (`cakisma.secim`) ucu yok; kutu yalnız dosyadır ve çakışan hal merkeze iletilmez.
9. **Ölçek:** her yazmada kuyruk dosyaları baştan okunur (`taa-kuyruk` de böyle). Şube ölçeğinde yeterli; on binlerce kayıtta yavaşlar.
10. **Veri sahipliği:** `dugum` ve `gunluk` alanları `docs/veri-sahipleri.toml`a eklenmedi (`modul.toml` `veri_sahibi` boş). `uyumluluk.toml`a da eklenmedi.
11. **Tek süreç:** aynı veri kökünde iki düğüm çalıştırılmamalıdır (kilit süreç içidir; aynı kapıda ikincisi zaten açılamaz).
12. **Denenmedi:** Windows derlemesi ve Windows'ta çalıştırma, gerçek telefon/tarayıcı, gerçek taa-sunucu, güvenlik duvarı kuralı, kurulumdan başlatma (TAA Kur). Yalnız Linux'ta `cargo test` + `cargo clippy -D warnings` ve yerel `curl` denemesi yapıldı.

### 13.7 Testler
`cargo test -p taa-sube-dugum` (18 test): `no` tekilliği, çakışma kutusu, merkez yokken bekleme ve sıra, kalıcı ret, 401/409 uyarlaması, delta ve sayfalama, çöküş onarımı, günlük ve özet, şube kapsamı, kapalı uçlar (parola diske yazılmaz), ağ / `Host` / köken sınırları, denetim kancası, gerçek soketle uçtan uca (telefon → düğüm → sahte merkez, `curl` ile), arka plan döngüsü. §10'daki kabul testlerinden karşılananlar: `dugum_cift_gonderim`, `dugum_ayni_no_farkli_icerik`, `dugum_cokus_defter_kuyruk`, `dugum_baska_sube_damgasi_red`, `dugum_yabanci_koken_red`, `dugum_merkez_reddi_gorunur` (delta düzeyinde), `dugum_internet_yok_iki_telefon` (delta düzeyinde; gerçek telefonla değil). Karşılanmayanlar: `dugum_eslesmemis_cihaz_red`, `dugum_belge_dolmus_red`, `dugum_7_gun_salt_okuma`, genel ağ profili ve alt ağ testleri.
