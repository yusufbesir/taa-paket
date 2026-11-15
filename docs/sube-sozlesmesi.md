<!-- Sahip: Yusuf. yazan: [Claude#sozlesme-S0 2026-11-15]. Durum: SÖZLEŞME (Ş0). Değişiklik: Yusuf'un onayıyla. EB0 eki işlendi: [Claude#sozlesme-EB0 2026-11-15], `docs/esleme-sozlesmesi.md` §1.5 D1–D16 (Yusuf 15:19, Fable denetimi 2026-11-15). -->
# Şube sözleşmesi (Ş0)

Kaynak: `planlar/PLAN-SUBE-ORTAK-SUNUCU-2026-11-15.md` (§2, §3.1, EK, EK 2), `planlar/PLAN-TAA-PAKET-ANA-2026-11-15.md` §4.11, §5.3.
Bağlı belgeler: `docs/sube-tehdit-modeli.md`, `docs/sube-izolasyon-testleri.md`, `docs/rehber-sozlesmesi.md`, `docs/havuz-sozlesmesi.md`, `docs/S3-CATI.md`, `docs/T7-KIMLIK.md`, `docs/esleme-sozlesmesi.md` (aynı şubede birden çok bilgisayar, EB0 eki). Mevcut uçlar: Tutaa `API-SOZLESMESI.md` (bu belge yalnız şube eklerini tanımlar).

## 1. Terimler
| Terim | Anlam |
|---|---|
| Şube | 15 mağazadan biri; kimliği `subeler.kod` (§3) |
| Şube bilgisayarı (şube PC'si) | Şubeye bağlı onaylı bilgisayarlardan biri (bir şubede 1 ya da daha çok). Aynı şubenin bilgisayarları eştir (`docs/esleme-sozlesmesi.md`). Bu belgede "şube PC'si" her zaman "şubenin bilgisayarlarından herhangi biri" demektir |
| Sunucu | TAA Sunucu (axum + PostgreSQL) = **Yusuf'un bilgisayarı** (EK 2, B3) |
| Merkez rolü | Tüm şubeleri gören yetki (yönetici, merkez personeli). Bir şube değildir |
| Merkez Mağaza | `MRK01` (eski `istanbul-merkez`) şubesi; Esenyurt Outlet buna bağlıdır, ayrı şube değildir. Esenyurt'ta sunucudan ayrı 2 kasa bilgisayarı vardır; ikisi de normal şube bilgisayarıdır (ana plan §16.1; AÇIK-S2 kapandı) |
| Merkez cihazı | `taa_cihazlar.sube_id = null` olan onaylı cihaz (Çatı, sunucu PC'si) |
| Cihaz anahtarı | ed25519; her isteği imzalar ("bu PC kayıtlı mı") |
| Şube imza anahtarı | ed25519; belgeyi imzalar (tutanak, ciro, gün sonu fiş özeti). Cihaz anahtarından **ayrıdır**. Her eş bilgisayarın kendi şube imza anahtarı vardır; hepsi şube adına geçerlidir |
| Havuz | Merkezden şubelere dağıtılan ortak içerik |
| İşlem tabloları | Şubeden merkeze akan kayıt (fiş, fiş olayı, tutanak, ciro) |

## 2. Beş temel kural (plan §2.1)
1. **Şube kimliği sunucuda belirlenir.** İstemcinin "ben şu şubeyim" beyanına güvenilmez.
2. **Tek yazar:** her kaydın tek sahibi şubedir, yalnız sahibi yazar. Merkez yalnız onay, iptal ve not olayı ekler (iptal yalnız yönetici, EK).
3. **Yerel önce:** her şube bilgisayarı sunucu ve eşi olmadan çalışır; yerel kayıt asıldır, kuyruk sonra gönderir.
4. **Eklemeli kayıt:** silme ve yerinde değiştirme yok; düzeltme yeni sürüm ya da olaydır.
5. **İki ayrı kanal** (§6). Şube kaydı havuza yazılmaz.

## 3. Şube kimliği ve 15 şube
Kaynak: `Tutaa subeler listesi (terziademaltun.com).json` (sürüm 1, 2026-11-05). Karar kapandı (EK 2, B2).
- `kod` = şube kodu, biçim **3 büyük harf + 2 rakam** (örn. `MRK01`; Yusuf 2026-11-15 kartı, TT PR #60 doğrulaması): ilk 3 harf `onek`, son 2 hane sıra numarası (aynı önekle ikinci şube `…02`). Kod şehir adını açıkça yazmaz, değişmez, yeniden kullanılmaz. Sunucuda `subeler.kod`, ilişkilerde `sube_id`. Kurucu, onaylı cihazın şubesinden `<dizin>/sube.json` dosyasına `{"kod":"MRK01"}` yazar.
- `slug` = eski okunur kimlik (listedeki `id`: küçük harf, tire). Yalnız eski kayıtlarda ve listede eşleme için durur; yeni kayıt `kod` kullanır. Geçiş: `slug` → `kod` tek yönlü eşleme tablosu aşağıdadır.
- `onek` = fiş/tutanak numarası öneki (ör. `PND-000123`, plan §2.2). Önekler kesindir (Yusuf 2026-11-15, ana plan §16.1); pilot başladıktan sonra değişmez (AÇIK-S1 kapandı). Çok bilgisayarlı şubede numara sırası `docs/esleme-sozlesmesi.md` §3'e göre tek eşte tutulur.

| # | `kod` | `slug` (eski) | Ad | `onek` |
|---|---|---|---|---|
| 1 | `MRK01` | `istanbul-merkez` | İstanbul Merkez Mağaza (Esenyurt, Outlet dahil) | MRK |
| 2 | `PND01` | `istanbul-pendik` | İstanbul Pendik Mağaza | PND |
| 3 | `UMR01` | `istanbul-umraniye` | İstanbul Ümraniye Mağaza | UMR |
| 4 | `ANK01` | `ankara` | Ankara Mağaza | ANK |
| 5 | `BRS01` | `bursa` | Bursa Mağaza | BRS |
| 6 | `SKR01` | `sakarya` | Sakarya Mağaza | SKR |
| 7 | `DYB01` | `diyarbakir` | Diyarbakır Mağaza | DYB |
| 8 | `SVS01` | `sivas` | Sivas Mağaza | SVS |
| 9 | `GZT01` | `gaziantep` | Gaziantep Mağaza | GZT |
| 10 | `URF01` | `sanliurfa` | Şanlıurfa Mağaza | URF |
| 11 | `VAN01` | `van` | Van Mağaza | VAN |
| 12 | `SRT01` | `siirt` | Siirt Mağaza | SRT |
| 13 | `ELZ01` | `elazig` | Elazığ Mağaza | ELZ |
| 14 | `IZM01` | `izmir` | İzmir Mağaza | IZM |
| 15 | `MUS01` | `mus` | Muş Mağaza | MUS |

Esenyurt Outlet için ayrı `kod` yoktur; kayıtları `MRK01` (`istanbul-merkez`) altındadır. Yeni şube yalnız Çatı'dan (`taa-cati sube ekle`) eklenir; şube kapatılırsa `pasif` olur, silinmez.

## 4. Cihazdan şube türetme
1. Cihaz başvurusu → Çatı'da onay sırasında şubeye bağlanır (`taa_cihazlar.sube_id`). Cihazın şubesi **yalnız Çatı'dan** değişir. Bir şubeye birden çok cihaz bağlanabilir; Çatı şube başına cihaz listesini gösterir (EB1).
2. Her istekte sunucu: cihaz imzasını doğrular → `cihaz.sube_id` = **istek şubesi**. Kayıtsız ya da iptal edilmiş cihaz parolaya bakılmadan reddedilir.
3. Oturum (`kullanici_rol.sube_id`) ile kesişim: kişinin o şubede rolü yoksa 403 `yetkisiz`. Kayıtlı cihaz + bilinen kişi birlikte şart (K13).
4. `X-TAA-Sube` başlığı (§9.3) ya da gövdedeki şube alanı istek şubesiyle uyuşmazsa **409 `sube_uyusmaz`** ve `yetki.red.sube` olayı denetim izine yazılır.
5. Merkez cihazında (`sube_id = null`) istek şubesi yoktur; şubeli okuma yalnız merkez rolü izniyle ve `X-TAA-Sube` süzgeciyle yapılır, denetim izine yazılır.
6. İstemci tarafı: `cihaz.json` içindeki onaylı şube kodu kayıtlara (`Fis.sube` vb.) damgalanır; **yalnız gösterim ve çevrimdışı süzgeç içindir**, yetkili kaynak sunucudur. Kuyruk öğesinin damgası cihazın şubesine uymazsa gönderilmez.
7. PC başka şubeye verilirse eski yerel veri `eski\sube-<kod>-<tarih>\` altına taşınır (silinmez), yeni alan boş açılır; şube imza anahtarı yeniden üretilir (§7.5). Devredilen PC eski şubenin eş listesinden düşer; elindeki yedek numara aralığının kullanılmamış kısmı atlandı notuyla kapanır (`docs/esleme-sozlesmesi.md` §3).

### 4.1 Yerel PIN ve çevrimdışı yönetici belgesi (KM1; ana plan §5.3 v2)
- **Kayıtlı cihazda yerel PIN yönetici yetkisi vermez.** Cihaz bir kez pakete kaydedildiyse program yerel kipe kendiliğinden geri düşemez; geri dönüş yalnız Çatı'dan cihazın çıkarılmasıyla olur. Kasiyer kilidi gibi çalışan düzeyi PIN'ler ayrıdır ve yönetici izni taşımaz. Paketsiz (hiç kaydedilmemiş) programda yerel PIN sürer.
- **Çevrimdışı yönetici belgesi** sunucu imzalıdır, 7 gün geçerlidir ve şunlara **bağlıdır**: kullanıcı, cihaz açık anahtarı, şube (`kod`; merkez cihazında `null`), izin listesi, verilme ve bitiş zamanı, belge kimliği. Başka PC'ye kopyalanan belge cihaz anahtarı uymadığı için reddedilir (ayrı donanım parmak izi gerekmez).
- **Saat geri alma denetimi:** program son gördüğü sunucu zamanını ve artan bir sayacı (cihaz sıra numarası) saklar. Sistem saati bu zamanın gerisine düşerse ya da sayaç geriye giderse yönetici eylemleri sunucuya bağlanana kadar durur; normal iş (fiş yazımı) sürer. Olay bağlanınca denetim izine yazılır.
- Belge süresi dolunca yalnız yönetici eylemleri (iptal, dışa aktarma, ayar kilidi) durur.
- **Dürüst sınır:** çevrimdışı bir şubede yetki anında iptal edilemez; 7 gün kabul edilen en uzun iptal gecikmesidir (ana plan §10 K8).

## 5. Tek yazar kuralı (ayrıntı)
| Kayıt | Yazar | Merkezin ekleyebildiği |
|---|---|---|
| Fiş, fiş olayı | Sahibi şube; şubenin onaylı bilgisayarlarından herhangi biri yazar, kayıtta `kaynak_cihaz` durur. Telefonlar yalnız telefon sunucusu rolündeki eşe bağlanır (o eşin kendi kapısı, eş kapısından ayrı; `docs/esleme-sozlesmesi.md` §4.5, E8 kararı); telefon merkeze yazmaz | iptal (yönetici), not |
| Tutanak, tutanak eki | Düzenleyen şube (`taraf_subeler` diğer şubeye yalnız okuma verir) | onay, iptal (yönetici), not |
| Ciro günlük | Sahibi şube | onay (giren onaylayamaz, K8), not |
| Havuz türleri | Katalogdaki tek yazar (`docs/havuz-turleri.toml`) | — |
| Şube imza anahtarı kaydı | Her şube bilgisayarı kendi anahtarıyla başvurur, Çatı onaylar/iptal eder | onay, iptal |

"Şube kilitli": belgedeki şube alanı cihazın şubesinden gelir, elle değişmez.

## 6. İki kanal
| | Havuz | İşlem tabloları |
|---|---|---|
| Yön | merkez → şube | şube → merkez |
| İçerik | şablon, liste, video, rehber, `ciro.sablon@1`, `tutanak.sablon@1`, `sube.kase@1` | fiş, fiş olayı, tutanak, ciro |
| Yazar | tür başına tek yazar (tablo) | kaydın sahibi şube |
| Şube süzgeci | `sube_kapsam` (boş = herkes; **rehber türlerinde boş = yalnız merkez**) | `sube_id` (uygulama + RLS) |
| Uçlar | havuz uçları (`docs/havuz-sozlesmesi.md`) | §9.4 |

Bir kanal öbürünün yerine kullanılmaz: şube kaydı havuza, havuz içeriği işlem tablosuna yazılmaz.

**Eş kanalı** (aynı şubenin bilgisayarları arası, yerel ağ; yalnız çok bilgisayarlı şubede) yalnız o şubenin işlem kayıtlarını, yazdırma işlerini ve numara isteklerini taşır; havuz içeriği, ayar, rehber ve anahtar taşımaz. Ayrıntı `docs/esleme-sozlesmesi.md` §4, §5.

## 7. Şube imzası
### 7.1 İki anahtar
| Anahtar | Amaç | Saklama |
|---|---|---|
| Cihaz anahtarı | İstek imzası | `<kök>\cihaz\` (taa-cihaz) |
| Şube imza anahtarı | Belge imzası | Her şube bilgisayarında ayrı üretilir; özel anahtar **PC'den çıkmaz**, sunucuya ve depoya gitmez, **yedeği alınmaz**; Windows DPAPI ile sarılı |

İki anahtar **aynı anahtar olamaz**; sunucu kayıtta aynı açık anahtarı görürse başvuruyu reddeder. Yönetici kimliği ≠ şube imzası (EK).

### 7.2 İmzalanan veri
Alan ayrımı etiketi taşır, böylece istek imzası belge imzası yerine kullanılamaz. Önerilen kanonik biçim (Ş3'te bayt düzeyinde kesinleşir):
```
TAA-BELGE-v1\n<sube_kodu>\n<belge_turu>\n<no>\n<sha256(belge kanonik JSON)>
```
`belge_turu`: `tutanak`, `ciro`, `fis-gunsonu`. İstek imzasının etiketi farklıdır; sunucu etiketi denetler.

### 7.3 Kabul şartı (iki anahtar birlikte)
Belge imzası yalnız şu koşullar birlikte sağlanırsa kabul edilir:
1. İstek, **aynı şubeye bağlı onaylı cihazdan** gelir (cihaz imzası geçerli).
2. Belge imzası, o şubenin **geçerli** (onaylı, iptal edilmemiş, tarih aralığı içinde) şube imza anahtarlarından biriyle doğrulanır.
3. Belgedeki `sube_kodu` istek şubesine eşittir.

### 7.4 Sunucu makbuzu
Sunucu kabul ettiği her belge için kendi anahtarıyla imzalı makbuz döner; şube "merkeze ulaştı" kanıtı olarak saklar.
| Alan | Not |
|---|---|
| `etiket` | `TAA-MAKBUZ-v1` |
| `belge_turu`, `kimlik`, `no`, `sube_kodu` | |
| `sha256` | Belgenin özeti |
| `alindi` | Sunucu zamanı (UTC) |
| `sunucu_anahtar_id` | Makbuz anahtarının kimliği (anahtar dönerse eski makbuz doğrulanabilir kalır) |
| `imza` | ed25519, yukarıdaki alanların kanonik biçimi üzerinde |

### 7.5 "Devredilemez" sınırı (dürüst not)
**Yazılımla tam devredilemezlik garanti edilemez.** DPAPI sarması dosyanın başka PC'de açılmasını engeller, ama PC'de yönetici yetkisi olan bir saldırgan anahtarı kullanabilir. Bu risk iki önlemle sınırlanır:
- **İki anahtar şartı** (§7.3): yalnız şube imza anahtarı ele geçen biri, aynı şubenin onaylı cihazı olmadan belge gönderemez.
- **İptal akışı:** Çatı'dan cihaz ve şube imza anahtarı iptal edilir (`iptal` zamanı yazılır) → oturumlar düşer → o andan sonraki imzalar reddedilir. Eski imzalar anahtarın geçerlilik aralığı içinde geçerli kalır. PC değişince yeni anahtar üretilir, Çatı'dan onaylanır, eskisi iptal edilir. Bir bilgisayarın anahtarının iptali şubenin öteki anahtarlarını etkilemez.

### 7.6 Görünür kısım
Belgede şube adı ve doğrulama karekodu (K5). Görsel kaşe isteğe bağlı (`sube.kase@1`, tek şubelik `sube_kapsam`). Tutaa (JS) anahtara dokunmaz; `imza.belge@1` yeteneğini ister, yoksa yalnız yerel HMAC ile çalışır ve belge "merkez imzasız" kuyruğa girer.

## 8. Veri yeri ve dış hizmetler
- **Müşteri verisi yalnız sunucuda, yani Yusuf'un bilgisayarındadır** (EK 2, B3). Merkez bulut yoktur; müşteri verisi **dış buluta, Google Sheets'e, dış yapay zekâya ve ajan hafızasına gitmez**. Ciro özeti Sheets aktarımı kapalı (K11).
- Taşıma şifreli (HTTPS + WireGuard/Tailscale). Uçtan uca şifreleme yok (sunucu rapor için okur). Şube ekleri zarf şifrelemesiyle gider (Ş3).
- Yedek: gece şifreli `pg_dump`; şifreli kopya da dış buluta konmaz (AÇIK-S4).
- Şube bilgisayarında yerel veri, önbellek ve kuyruk o cihaza aittir; DPAPI ile sarılır. Aynı şubenin işlem kayıtları eşlemeyle her eşte de bulunur; her kopya DPAPI ile sarılıdır.

## 9. API eki
### 9.1 Ortak kurallar
- Taban: `https://<sunucu>/v1`. Yalnız HTTPS (kendi kök sertifika, `--sertifika <pem>`). PostgreSQL yalnız 127.0.0.1 dinler.
- Gövde JSON, UTF-8. Zaman UTC RFC 3339. Tutar kuruş tam sayı. Kimlikler uuid.
- Tekrar gönderim güvenli: yazma uçları `no` ya da `olay_id` ile tek kayıt üretir.
- Liste uçları sayfalı: `?since=<sira>&limit=<1..500>`; yanıtta `sonraki`.

### 9.2 Kimlik doğrulama
1. **Cihaz imzası:** her istekte mevcut taa-cihaz başlıkları (yöntem, yol, gövde özeti, zaman, tek kullanımlık değer). Zaman ±5 dk dışında ya da değer tekrar edilmişse 401.
2. **Oturum:** `Authorization: Bearer <oturum>` (`POST /v1/giris`, T7). Yönetici hesabında TOTP zorunlu (K13).
3. **Yazma/yönetim:** son 15 dk içinde yeniden doğrulama (`POST /v1/yeniden-dogrula`); yoksa 401 `yeniden_dogrula`.
4. **Yetki:** tek kapı `yetki::izinli(oturum, izin, sube)` + PostgreSQL RLS (ikinci kilit, `docs/sube-tehdit-modeli.md` §6).
5. **Çevrimdışı:** sunucu imzalı yetki belgesi 7 gün geçerli; dolunca yerel fiş yazımı sürer, merkez ve yönetici işlevleri kapanır (K7, EK). Belge kullanıcı + cihaz anahtarı + şube + izin listesine bağlıdır; istemci saat geri alma denetimi yapar (§4.1). Sunucu, belgeyle yapılmış çevrimdışı yönetici eylemini eşitlemede yeniden denetler: belge kimliği iptal edilmişse ya da eylem zamanı belge aralığı veya son sunucu zamanı öncesindeyse eylem "çelişkili" işaretiyle saklanır, uygulanmaz.
6. **Yerel PIN:** kayıtlı cihazdan gelen istekte yerel PIN hiçbir yönetici izni sağlamaz; yönetici izinleri yalnız `kimlik.yonetici@1` oturumundan ya da geçerli çevrimdışı belgeden gelir.

### 9.3 Şube bağlamı başlığı `X-TAA-Sube`
- Değer: şube `kod`u. **Yetki değildir**, tutarlılık denetimidir (`X-TAA-Program` gibi).
- Şube cihazında: isteğe bağlı; gönderilirse cihazın şubesine eşit olmalı, değilse 409 `sube_uyusmaz` + `yetki.red.sube`.
- Merkez cihazında: şubeli okumada süzgeçtir; merkez rolü izni olmadan yok sayılmaz, 403 döner. Her kullanım denetim izine yazılır.

### 9.4 Uç noktalar (şube ekleri)
| Uç | İzin | Şube kuralı | Aşama |
|---|---|---|---|
| `POST /v1/fis/{id}/olay` | `fis.yaz` | Fiş istek şubesine ait olmalı; `olay_id` tek kayıt | Ş4 |
| `GET /v1/fis?since=` | `fis.oku` | Şube: yalnız kendisi. Merkez: maskeli liste (ad baş harfi, telefon son iki hane) | Ş4 |
| `GET /v1/fis/{id}` | `fis.oku` (+ merkezde ayrıntı izni) | Ayrıntı ayrı uçtan, denetim iziyle açılır | Ş4/Ş5 |
| `POST /v1/fis/{id}/iptal` | `fis.iptal` (yönetici) | Eklemeli iptal olayı | Ş4 |
| `POST /v1/tutanak` | `tutanak.yaz` | `no` şube önekli; şube imzası (§7.3) | Ş5 |
| `GET /v1/tutanak` | `tutanak.oku` | Şube: kendisi + `taraf_subeler` içinde olduğu tutanaklar | Ş5 |
| `POST /v1/tutanak/{id}/iptal` | `tutanak.iptal` (yalnız yönetici) | Eklemeli | Ş5 |
| `POST /v1/tutanak/{id}/ek` | `tutanak.yaz` | Yalnız düzenleyen şube; en çok 3 fotoğraf; MIME izin listesi, boyut sınırı, sha256, yürütülebilir dosya reddi; zarf şifreli | Ş5 |
| `POST /v1/ciro` | `ciro.yaz` | Şube imzalı; düzeltme yeni `surum_no` | Ş6 |
| `GET /v1/ciro?sube=&ay=` | `ciro.oku` | Şube: `sube` parametresi yok sayılır, yalnız kendisi. Merkez: süzgeç | Ş6 |
| `POST /v1/ciro/{id}/onay` | `ciro.onay` | Giren onaylayamaz (K8) | Ş6 |
| `POST /v1/sube-imza/basvuru` | `sube_imza.basvur` | Açık anahtar + cihaz; cihaz anahtarıyla aynı olamaz | Ş3 (öneri) |
| `POST /v1/sube-imza/{id}/onay` | `sube_imza.yonet` (merkez cihazı) | Çatı "şube imzasını onayla" | Ş3 (öneri) |
| `POST /v1/sube-imza/{id}/iptal` | `sube_imza.yonet` | İptal zamanı yazılır; geri alınmaz, yeni anahtar başvurur | Ş3 (öneri) |
| `GET /v1/sube-imza?sube=` | `sube_imza.oku` | Şube: kendi anahtarları; doğrulama için açık anahtarlar | Ş3 (öneri) |
| `GET /v1/makbuz-anahtari` | oturum | Sunucu makbuz açık anahtarları | Ş3 (öneri) |
| `POST /v1/yonetici-belgesi` | yönetici oturumu (TOTP) | Çevrimdışı yönetici belgesi verir; gövde cihaz açık anahtarı içermez, sunucu istek cihazından alır | KM1 (öneri) |
| `POST /v1/rehber/komut` | `rehber.yaz` (yönetici) | `rehber.yaz@1` yazma hizmeti; çevrimdışı kuyruğa alınmaz; beklenen `surum` uymazsa 409 | RH1 (öneri) |
| `POST /v1/oneri` | oturum | `veri.oner@1`; ayrı denetimli kuyruk | RH1 (öneri) |
| `GET /v1/sube/esler` | şube cihazı | Sunucu imzalı eş listesi; yalnız istek şubesi (`docs/esleme-sozlesmesi.md` §5.2) | EB1 (öneri) |
| `GET /v1/sube/yazicilar` | şube cihazı | Yazıcı kaydı; yalnız istek şubesi | EB1 (öneri) |
| `POST /v1/sube/yazici` | `sube_cihaz.yonet` (merkez) ya da kurulum yetkisi (TAA Kur, yönetici oturumu) | Yazıcı ekle, sahibini değiştir, pasifleştir; yalnız istek şubesi; `surum` uymazsa 409 | EB1 (öneri) |
| `POST /v1/sube/rol` | `sube_cihaz.yonet` (merkez) | Sıra tutan, telefon sunucusu ataması; yalnız o şubenin cihazına | EB1 (öneri) |
| `POST /v1/numara/olay` | şube cihazı | Verilen blok, kullanılan yedek numara, atlama; `olay_id` tek kayıt | EB1 (öneri) |

Belge yazan uçların yanıtı makbuzu (§7.4) içerir. Uç ayrıntıları (alanlar) Tutaa `API-SOZLESMESI.md` ve ilgili aşama PR'ında; bu tablo şube kuralını bağlar. "(öneri)" satırları planda adı geçmeyen, Ş3'te kesinleşecek uçlardır.

### 9.5 Hız sınırı
Plan: IP ve kişi başına hız sınırı, 5 yanlışta kilit (var). Sayılar ölçülmeden kesinleşmez; Ş2 için varsayılan öneri:
| Kapsam | Sınır (öneri) |
|---|---|
| Giriş | 5 yanlış → 15 dk kilit (hesap + IP) |
| Cihaz başına genel | 120 istek/dk |
| Cihaz başına yazma | 60 istek/dk |
| Ek yükleme | 10/dk, tek dosya ≤ 10 MiB |
| Merkez ayrıntı açma (maskesiz) | 30/saat/kişi, aşımda uyarı olayı |
Aşımda 429 `hiz_siniri` ve `Retry-After`. Kuyruk istemcisi `Retry-After`'a uyar.

### 9.6 Hata kodları
Gövde: `{"hata": "<kod>", "mesaj": "<Türkçe sade cümle>"}`. **Mesaj başka şubenin adını, kaydın varlığını, kişi verisini ya da iç ayrıntıyı (SQL, yol, yığın) içermez.**
| HTTP | `hata` | Ne zaman |
|---|---|---|
| 400 | `gecersiz_istek` | Şema/biçim hatası |
| 401 | `oturum_yok` | Oturum yok ya da süresi doldu |
| 401 | `cihaz_imzasi` | Cihaz imzası geçersiz, zaman sapması, tekrar |
| 401 | `yeniden_dogrula` | Son 15 dk doğrulama yok |
| 401 | `belge_gecersiz` | Çevrimdışı yönetici belgesi süresi dolmuş, iptal edilmiş ya da başka cihaza/kullanıcıya/şubeye ait |
| 409 | `saat_geri` | İstemci saati son sunucu zamanının gerisinde; yönetici eylemi reddedildi |
| 403 | `cihaz_onaysiz` | Kayıtsız ya da iptal edilmiş cihaz |
| 403 | `yetkisiz` | İzin yok |
| 404 | `bulunamadi` | Kayıt yok **ya da başka şubenin** (ikisi ayırt edilemez) |
| 409 | `sube_uyusmaz` | Beyan edilen şube ≠ cihazın şubesi |
| 409 | `cakisma` | Aynı `no`, farklı içerik; iki sürüm saklanır |
| 413 | `cok_buyuk` | Boyut sınırı |
| 415 | `mime_red` | İzin listesinde olmayan dosya türü |
| 422 | `imza_gecersiz` | Belge imzası doğrulanmadı |
| 422 | `imza_anahtari_iptal` | Anahtar iptal edilmiş ya da aralık dışı |
| 422 | `gorev_ayriligi` | Giren onaylamaya çalıştı (K8) |
| 423 | `kilitli` | Hesap kilidi |
| 429 | `hiz_siniri` | §9.5 |
| 503 | `bakim` | Sunucu bakımda; istemci kuyruğa alır |

## 10. Çakışma (plan §2.6)
- Tekrar gönderim: `no` ve `olay_id` ile tek kayıt.
- Aynı `no`, farklı içerik: 409 `cakisma`; iki sürüm `cakisma` kaydında saklanır, şube müdürü ya da yönetici geçerliyi işaretler. "Son yazan kazanır" yok.
- Sıra: sunucu alınma sırası + cihaz sıra numarası; istemci saati yalnız bilgi, 5 dk üstü sapma uyarı. Cihaz sıra numarası her cihaz için ayrıdır; eşten gelen çakışma da aynı kuralla iki hal olarak saklanır (`docs/esleme-sozlesmesi.md` §4.3).
- Kurala uymayan durum olayı reddedilmez, "çelişkili" işaretiyle saklanır.

## 11. Açık noktalar
Liste `docs/sube-tehdit-modeli.md` §8'de ortak tutulur (AÇIK-S1 … S8; S1 ve S2 kapandı); eşleme açıkları `docs/esleme-sozlesmesi.md` §8; rehber açıkları `docs/rehber-sozlesmesi.md` §11.
