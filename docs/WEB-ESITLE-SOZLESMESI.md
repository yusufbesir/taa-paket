<!-- Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-yazdir 2026-11-27]. Durum: ÖNERİ (sunucuda uygulanmadı). Değişiklik: Yusuf'un onayıyla. -->
# TAA Web eşitleme uç sözleşmesi (taa-sunucu'ya eklenecek)

Plan: `PLAN-TAA-WEB-2026-11-27.md` §2, §6 (W0). İstemci tarafı: `web/cekirdek/taa-cekirdek.js` (`Giden`, `Onbellek`),
`web/kabuk/BENIOKU.md`. Bağlı belgeler: `docs/sube-sozlesmesi.md` (Ş0), `docs/esleme-sozlesmesi.md` (EB0),
`docs/T6-KUYRUK.md`, `docs/T7-KIMLIK.md`, `docs/S2-CIHAZ.md`, `docs/SUBE-DUGUMU.md`.

> **Önemli: taa-sunucu kodu bu oturumda elimde yoktu ve okunmadı.** (Özel depo, Asus'ta.) Bu belge o depoda neyin var
> olduğunu bilmeden, yalnız taa-paket'teki sözleşmelere ve tarayıcı çekirdeğinin bugünkü davranışına bakılarak yazıldı.
> Aşağıdaki uç adları, alan adları ve hata kodları **öneridir**; Asus'ta taa-sunucu okununca oradaki gerçek adlarla
> karşılaştırılıp düzeltilmelidir. Çelişki olursa Ş0 ve taa-sunucu'daki mevcut davranış geçerlidir, bu belge uyarlanır.

## 1. Kapsam
Tarayıcıdaki TAA Web'in iki ihtiyacı:
1. **Yazma:** giden kutusundaki kayıtları `POST /v1/<uc>` ile, `no` üstünden tek kayıt olacak biçimde göndermek.
2. **Okuma:** sunucudaki verinin şubeye ait kısmını `GET /v1/esitle?sonra=<surum>` ile imzalı delta olarak almak.

Masaüstü istemciler (`taa-kuyruk`) aynı `no` sözleşmesini kullanır; iki istemci türü aynı sunucuya karışık yazabilir.

## 2. Ortak kurallar
Ş0 §9.1 aynen: taban `https://<sunucu>/v1`, yalnız HTTPS, gövde JSON UTF-8, zaman UTC RFC 3339, tutar kuruş tam sayı.
Hata gövdesi Ş0'daki gibi `{"hata": "<kod>", "mesaj": "<Türkçe sade cümle>"}`; mesaj başka şubenin adını, kaydın
varlığını, kişi verisini ya da iç ayrıntıyı içermez.

Kabuk ve `/v1` **aynı kökenden** sunulur (`https://<sunucu>/kabuk/`, `https://<sunucu>/v1/...`); böylece CORS gerekmez.
Service worker `/v1/` isteklerine dokunmaz (önbelleğe almaz).

## 3. Kimlik
| Katman | Bugün tarayıcıda | Sözleşme |
|---|---|---|
| Oturum | `POST /v1/giris {kullanici, parola[, kod]} -> {oturum, sube_id, roller[]}`; her istekte `Authorization: Bearer <oturum>` | T7 aynen |
| Cihaz imzası | **Yok.** Çekirdek yalnız `Authorization` ve `X-TAA-No` gönderir | Ş0 §9.2 madde 1 her istekte cihaz imzası ister, kayıtsız cihazı parolaya bakmadan reddeder. Tarayıcı için karar gerekir (§10 madde 1) |
| Çevrimdışı yetki belgesi | Kabuk imzasız yerel profille "son girişten sonra 7 gün" kuralını uygular | §7 |

## 4. Yazma: `POST /v1/<uc>` (idempotent)
### 4.1 İstek
```
POST /v1/kasa/sayim
Authorization: Bearer <oturum>
X-TAA-No: K7Q2-000184-20261127T105501Z
Content-Type: application/json

{"no": "K7Q2-000184-20261127T105501Z", ...kayıt alanları...}
```
- `no`: `[A-Za-z0-9._-]`, 1-64 karakter. Tarayıcıda `cihaz-sayaç-zaman`; masaüstünde T6 biçimi. Sunucu biçime anlam yüklemez, yalnız tekillik anahtarı olarak kullanır.
- Çekirdek gövdede `no` yoksa ekler. Sunucu başlıktaki ve gövdedeki `no` farklıysa 400 `no_uyusmaz` döner.
- `<uc>` her zaman `/v1/` ile başlar. Hangi uçların olduğu program sözleşmelerinde yazılıdır (`kasa.*`, `fis.*`, `tutanak.*` ...); bu belge yalnız hepsinin uyacağı ortak davranışı tanımlar.

### 4.2 Sunucu davranışı
| Durum | Yanıt | Not |
|---|---|---|
| `no` ilk kez geldi, kayıt geçerli | 200 ya da 201 `{"no", "durum": "yazildi", "surum": <n>, "sunucu_zamani"}` | Kayıt ve `no` aynı veritabanı işleminde yazılır |
| Aynı `no`, aynı içerik | 200 `{"no", "durum": "zaten_var", "surum": <ilk yazımdaki n>}` | **İkinci kayıt oluşmaz.** Yarım kalan gönderimin tekrarı güvenlidir |
| Aynı `no`, farklı içerik | 409 `cakisma` | Ş0 §10: iki hal `cakisma` kaydında saklanır, insan seçer. "Son yazan kazanır" yok |
| Kayıt başka şubeye ait ya da gövdedeki şube istek şubesine uymuyor | 409 `sube_uyusmaz` | Ş0 §4 madde 4; denetim izine `yetki.red.sube` |
| İzin yok | 403 `yetkisiz` | |
| Şema bozuk | 400 ya da 422 | |
| Gövde çok büyük | 413 | Sınır uç başına; sunucuda yazılır |
| Oturum süresi dolmuş | 401 | Aşağıdaki tabloya bakın |
| Sunucu geçici olarak yazamıyor (bakım, veritabanı, kilit) | 503 (ya da 5xx) | **Geçici durum için asla 4xx dönülmez** |
| Hız sınırı | 429 | |

"Aynı içerik" karşılaştırması için öneri: sunucu `no` ile birlikte gövdenin kanonik JSON özetini (sha256) saklar
(kanonik biçim: `taa_imza::kanonik_bayt` ile aynı kural). taa-sunucu'da bugün nasıl yapıldığı bilinmiyor.

Şube kimliği **sunucuda** belirlenir (oturum + cihaz); istemcinin gövdeye yazdığı şube yalnız tutarlılık denetimidir (Ş0 §2 kural 1).
Tekillik anahtarı `no`dur. Başka şubeden aynı `no` gelirse sunucu kaydın varlığını belli etmeden 409 döner.

### 4.3 İstemcinin yanıta tepkisi (çekirdekte bugün böyle)
| Yanıt | Giden kutusunda | Sunucu için anlamı |
|---|---|---|
| 2xx | `gonderildi` | Kalıcı kabul. 2xx döndükten sonra kayıt kaybolmamalı |
| 4xx (401, 408, 429 hariç) | `reddedildi`, yeniden denenmez, kişi bakar | Kalıcı ret yalnız gerçekten kalıcıysa dönülür |
| 401 | Gönderim o kayıtta **durur**, kayıt `bekliyor` kalır | Süresi dolan oturum bütün kutuyu reddettirmesin diye. Yeniden girişten sonra sürer |
| 408, 429, 5xx, ağ hatası, zaman aşımı | Gönderim o kayıtta durur, sıra bozulmaz | |

İstemci kayıtları sırayla gönderir; bir kayıt 2xx almadan sonraki gönderilmez. Sunucu yine de sıraya güvenmez
(iki sekme, iki cihaz, masaüstü kuyruğu aynı anda gönderebilir): sıra bilgisi sunucu alınma sırası + cihaz sıra numarasıdır (Ş0 §10).

Silme ucu yoktur. İptal, düzeltme, pasife alma ayrı eklemeli kayıtlardır.

## 5. Düzenlenebilir kayıtlar: sürüm ve çakışma kutusu
Ekleme tipi kayıtlar (fiş, kasa sayımı, tutanak, puan) §4 ile biter: yalnız eklenir, birbirini ezmez.
Düzenlenebilir kayıtlarda (kişi, ayar gibi) istemci hangi sürümün üstüne yazdığını bildirir:

```
{"no": "...", "onceki_surum": 41, ...yeni değerler...}
```
| Durum | Yanıt |
|---|---|
| Kaydın sunucudaki sürümü `onceki_surum` ile aynı | 200 `{"durum": "yazildi", "surum": 42}` |
| Sunucudaki sürüm daha yeni | 409 `{"hata": "surum_eski", "mesaj", "cakisma_id", "gecerli_surum"}` |

409 `surum_eski` durumunda:
- Sunucudaki hal **geçerli kalır**; gelen değişiklik atılmaz, **çakışma kutusuna** yazılır (iki hal de saklanır).
- Kimse sessizce ezilmez: istemcide kayıt `reddedildi` listesine düşer ve modül "çakışma kutusunda" diye gösterir.
- Hangisinin geçerli olacağını insan seçer (şube müdürü ya da yönetici). Seçim de eklemeli bir olaydır (`cakisma.secim`).

Çakışma kutusu uçları (öneri):
| Uç | İzin | Not |
|---|---|---|
| `GET /v1/cakisma?durum=acik` | `cakisma.oku` | Şube: yalnız kendi şubesi. İki hal yan yana döner |
| `POST /v1/cakisma/{id}/secim` | `cakisma.sec` | Gövde `{"no", "secilen": "sunucu"\|"gelen"}`; eklemeli; seçim yeni sürüm üretir |

Bu, plan §2'deki "çakışmada sunucu kazanır, kaybeden çakışma kutusuna düşer" cümlesinin Ş0 §10 ve EB0 §4.3 ile
("iki hal saklanır, insan seçer, son yazan kazanmaz") birlikte okunmuş halidir: sunucudaki hal seçim yapılana kadar
geçerlidir, ama son söz insandadır.

## 6. Okuma: `GET /v1/esitle?sonra=<surum>` (imzalı delta)
### 6.1 İstek
`GET /v1/esitle?sonra=<surum>[&limit=<1..500>][&alan=<ad>]`
- `sonra`: istemcinin elindeki son sürüm; ilk eşitlemede `0`.
- `alan`: isteğe bağlı süzgeç (örnek `rehber`, `kasa`); verilmezse istemcinin görebildiği her şey.

### 6.2 Yanıt
```json
{
  "belge": {
    "sube": "MRK01",
    "gorunum": "g-3f9a",
    "sonra": 120,
    "surum": 184,
    "tam": false,
    "devami": false,
    "sunucu_zamani": "2026-11-27T10:55:01Z",
    "kayitlar": [
      {"anahtar": "kasa/gunsonu/2026-11-26", "surum": 181, "deger": {"...": "..."}},
      {"anahtar": "rehber/kisi/K-0042", "surum": 184, "deger": {"...": "..."}, "pasif": true}
    ]
  },
  "imza": {"alg": "ed25519", "anahtar_kimlik": "...", "deger": "<hex>"}
}
```
- `surum`: sunucunun **tekdüze artan değişiklik sayacı**; her yazılan ya da değişen kayıt yeni bir sayı alır. `belge.surum` bu yanıtın kapsadığı en büyük sayıdır; istemci bir sonraki istekte `sonra` olarak bunu gönderir.
- `kayitlar` `surum`a göre artan sıradadır. Aynı anahtar bir yanıtta en çok bir kez (son haliyle) bulunur.
- `devami: true` ise istemci `sonra=<belge.surum>` ile hemen yeniden ister.
- `tam: true`: sunucu deltayı veremedi (istemcinin `sonra`sı saklanan geçmişten eski, ya da `gorunum` değişti); yanıt görünümün tam halidir.
- Değişiklik yoksa 200 ve boş `kayitlar` döner (`surum` = `sonra`).
- Silme yoktur. Geçerliliğini yitiren kayıt `"pasif": true` ile gelir; istemci kaydı tutar, ekranda göstermez.

### 6.3 İmza
- İmzalanan: `belge` nesnesinin kanonik JSON baytları (`taa_imza::kanonik_bayt` kuralı). İmza nesnesi `taa_imza::Imza` ile aynı alanları taşır.
- İstemci imzası tutmayan deltayı **uygulamaz**, önceki önbelleği kullanmayı sürdürür. Sunucu açık anahtarı kabuk dosyalarıyla birlikte gelir (anahtar değişimi kabuk sürümüyle yapılır).
- `belge.sube` ve `belge.sonra` imzanın içindedir: başka şubenin ya da başka bir isteğin yanıtı yerine konamaz.

### 6.4 İstemcide uygulama
Her kayıt için `onbellek.yaz(anahtar, deger, surum)`. Çekirdek eski sürümün yeniyi ezmesine izin vermez, bu yüzden
aynı deltanın iki kez uygulanması ya da iki sekmenin aynı anda eşitlemesi zararsızdır. Son `belge.surum` önbellekte
ayrı bir anahtarda tutulur (öneri: `esitle/son`).

Giden kutusu ile ilişki: istemcinin kendi gönderdiği kayıt da bir sonraki deltada sunucu haliyle geri gelir.
Modül ekranda "bekliyor" kayıtları giden kutusundan, kesinleşmiş kayıtları önbellekten gösterir.

## 7. Çevrimdışı yetki belgesi (7 gün)
Mevcut kural (S2-CIHAZ, Ş0 §9.2 madde 5): girişten sonra `POST /v1/yetki {}` sunucu imzalı belge verir; belge kullanıcı +
cihaz anahtarı + şube + tam `izinler` listesine bağlıdır; `gecerlilik` yoksa alınma + 7 gün; varsayılan reddet.

TAA Web için:
- Kabuk girişten hemen sonra belgeyi alır ve yerelde saklar (oturum anahtarı belgenin içinde yoktur).
- Çevrimdışı açılışta `baglam.yetki(izin)` kararı **bu belgeden** verilir: imza tutmuyorsa, süresi geçmişse ya da izin listede yoksa `false`.
- Belge dolunca: yerel kayıt ekleme (giden kutusu) sürer; merkez ve yönetici işlevleri kapanır (Ş0 K7). Giden kutusu yeniden girişe kadar bekler.
- İstemci saat geri alma denetimi yapar (son görülen sunucu zamanından eski saatle belge kabul edilmez).
- Sunucu her istekte yetkiyi yeniden denetler; belge yalnız çevrimdışı ekran kararı içindir, sunucuda yetki vermez.
- Bugünkü kabuk bunu yapmıyor (imzasız profil + 7 gün). İmzalı belgeye geçiş W0 kapanış işidir.

## 8. Şube ayrımı
- Şube oturumu yalnız kendi şubesinin kayıtlarını ve merkezin herkese yayımladığı salt okunur kayıtları (rehber, ortak ayar) alır.
- Şube cihazında `sube` parametresi ve `X-TAA-Sube` yetki değildir; uyuşmazsa 409 `sube_uyusmaz` (Ş0 §9.3).
- Merkez oturumunda şubeli okuma `X-TAA-Sube` süzgeciyle ve merkez izniyle yapılır; her kullanım denetim izine yazılır.
- İkinci kilit sunucuda PostgreSQL RLS'tir (Ş0 §9.2 madde 4): uç kodu süzgeci unutsa da başka şubenin satırı dönmez.
- `gorunum`: kullanıcının şubesi ve izinlerinden türeyen kısa kimlik. İstemci farklı bir `gorunum` görürse `sonra=0` ile baştan eşitler. Tarayıcı veritabanı zaten şube + kullanıcı başına ayrıdır (`taa-<sube>-<kullanici>`).

## 9. Sunucuda yazılacak kabul testleri
| Test | Beklenen |
|---|---|
| `esitle_cift_gonderim_tek_kayit` | Aynı `no` iki kez -> tek satır; ikinci yanıt `zaten_var` |
| `esitle_yarim_kalan_gonderim` | İstek yazıldı, yanıt istemciye ulaşmadı, istemci yeniden gönderdi -> tek satır |
| `esitle_ayni_no_farkli_icerik` | 409 `cakisma`; iki hal saklı |
| `esitle_surum_eski_kutuya_duser` | 409 `surum_eski`; sunucudaki hal değişmedi; kutuda kayıt var |
| `esitle_gecici_hata_4xx_degil` | Veritabanı kapalıyken 503; istemcide kayıt `bekliyor` kalır |
| `esitle_oturum_doldu_401` | 401; kutudaki hiçbir kayıt `reddedildi` olmaz |
| `esitle_sube_izolasyonu` | A şubesi oturumu B şubesinin kaydını ne yazabilir ne deltada görür |
| `esitle_delta_imza` | Bozulan `belge` istemcide uygulanmaz |
| `esitle_delta_sirali_ve_tekrarsiz` | `sonra` ile ilerleyen istemci hiçbir kaydı atlamaz; aynı deltanın tekrarı zararsız |
| `esitle_gorunum_degisti_tam` | İzin daralınca `tam: true`, eski kayıtlar `pasif` |
| `esitle_masaustu_ve_web_karisik` | `taa-kuyruk` ve tarayıcı aynı `no` ile gönderirse tek kayıt |

Uçtan uca (plan §6): ağı kes -> işlem yap -> ağı aç -> sunucuda tek kayıt. Bu test gerçek tarayıcı ister; depoda
Playwright yoktur (yeni dış bağımlılık olurdu).

## 10. Açık noktalar
1. **Tarayıcıda cihaz imzası — UYGULANDI (seçenek a).** `web/cekirdek/cihaz.js`: Ed25519 anahtar çifti WebCrypto'da çıkarılamaz üretilir, IndexedDB'de `CryptoKey` olarak saklanır; her `/v1` isteği `X-TAA-Cihaz/Zaman/Imza` taşır (S2-CIHAZ ile aynı bayt dizisi; JS imzası Rust `taa_cihaz::dogrula` ile `crates/taa-cihaz/tests/web_imza_uyumu.rs`'te doğrulanır). Kayıt `POST /v1/cihaz/kayit {acik_anahtar, ad}` (imzasız, idempotent), yönetici Çatı'dan onaylar; onaya kadar giriş ekranı "yönetici onayı bekliyor" der. Ed25519 desteği yoksa fallback yoktur, açık hata verilir. **Denenmedi:** gerçek tarayıcı (Safari 17+/Chrome/Firefox), gerçek taa-sunucu. Ayrı "tek kullanımlık eşleştirme kodu" yazılmadı: onay Çatı'daki mevcut cihaz onayıdır.
2. **taa-sunucu'daki gerçek durum bilinmiyor:** `no` tablosu, hata kodları, sürüm sayacı, RLS, `POST /v1/yetki` yanıt biçimi.
3. **Sürüm sayacının kapsamı:** tek genel sayaç mı, şube başına mı. İstemci için fark etmez (yalnız artan olması yeter); şube
   başına sayaç başka şubedeki hareketin sıklığını sızdırmaz, tercih edilir.
4. **Delta geçmişinin saklama süresi** (`tam: true` ne zaman gerekir) ve tam görünümün boyut sınırı.
5. **Fotoğraf ve büyük ekler** (tutanak eki, OCR işi) JSON giden kutusuna sığmaz; ayrı yükleme ucu ve tarayıcıda ayrı depo gerekir (W3).
6. **`pasif` kayıtların tarayıcıdan gerçekten kaldırılması** (KVKK saklama süresi): çekirdekte silme işlevi yok; karar gerekir.
7. **Çakışma kutusu izinleri** (`cakisma.oku`, `cakisma.sec`) rol tablosuna eklenmeli; adlar öneridir.
8. **Yeniden doğrulama** (Ş0 §9.2 madde 3: yazmada son 15 dk): çevrimdışı birikmiş kayıtların toplu gönderiminde nasıl uygulanacağı belirsiz.
