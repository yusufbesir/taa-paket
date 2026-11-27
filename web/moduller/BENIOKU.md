<!-- Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27] -->
# TAA Web modülleri (`web/moduller/`)

Plan: `/mnt/project-files/planlar/PLAN-TAA-WEB-2026-11-27.md` (W1). Her modül `web/moduller/<ad>/index.js` dosyasından
`{ad, baslik, ac(kok, baglam)}` dışa aktarır; `baglam = {giden, onbellek, yetki(izin) -> bool, tema}`. Modüller veriyi yalnız
bu bağlam üstünden okur ve yazar: ağ çağrısı yapmaz, `giden.gonder` çağırmaz, önbelleğe yazmaz. Silme yok.

| Modül | Ne yapar | Yazar mı |
|---|---|---|
| `kasa` | Sayım (açılış / ara / devir), masraf, gün sonu (canlı fark paneli, fark notu zorunlu), imza ve gerekçe (çift doğrulama) | evet |
| `terzitakip` | Fiş durumu listesi, geciken / bugün teslim / kargo süzgeçleri, arama, durum değiştirme | evet |
| `tutanak` | Tutaa: şablondan tutanak, fotoğraftan tutanak (OCR sırası), liste, imza (elle çizim / imtina) | evet |
| `rehber` | Kişi ve birim listesi, arama; salt okunur, tam çevrimdışı | hayır |
| `cati` | Yönetici ekranı iskeleti: yönetim izni yoksa hiçbir şey göstermez; izne göre ekran; salt okunur | hayır |

Modül listesi, izin adları, önbellek anahtarları ve uçlar: `LISTE.json`. Ortak biçem: `ortak/modul.css` (kabuğun `--tk-*`
tema değişkenlerini kullanır). Kurallar her modülün `mantik.js` dosyasındadır (DOM'suz, saf işlevler); `index.js` yalnız ekranı kurar.

## Varsayımlar (çekirdek ve sunucu sözleşmesi kesinleşince düzeltilecek)
Hepsi ilgili `mantik.js` dosyasının başında tek yerde durur.
- **Uç adları:** `/v1/kasa/gunsonu`, `/v1/kasa/masraf`, `/v1/fis/durum`. `docs/WEB-ESITLE-SOZLESMESI.md` bu çalışma sırasında yoktu.
- **Gövde:** kasa kayıtları JSONL satırıyla aynı zarfta gider: `{sozlesme, tur, v}` (`tur`: `sayim`, `gun_sonu`, `imza`, `gerekce`, `masraf`).
  `id`, `kisi`, `zaman` sunucuda verilir (kişi oturumdan). Henüz sunucu kimliği olmayan gün sonuna imza `istemci_no` (giden `no`) ile bağlanır.
- **İzin adları:** `kasa.sayim`, `kasa.masraf`, `kasa.gunsonu`, `kasa.imza`, `fis.oku`, `fis.guncelle`, `fis.iptal`, `rehber.oku`
  uydurma adlardır; Çatı izinleri ve `rehber.tum.oku` / `rehber.amir.oku` depodaki adlardır.
- **Önbellek anahtarları:** `kasa/ayar` (KasaAyar: çekmeceler, noktalar, POS'lar, eşik, tolerans), `kasa/gun/<tarih>/<çekmece>`
  (`{acilis_kurus, masraf_kurus, onceki_devreden_kurus, gun_sonu_var}`), `kasa/gunsonlari`, `terzitakip/fisler`, `rehber/veri`, `cati/*`.
- **`giden.liste(durum)`** kayıtları `{no, program, uc, govde, durum}` döner; **`onbellek.oku`** değeri düz ya da `{deger, surum}` döner (ikisi de kabul edilir).
- `baglam.kullanici.ad` varsa "giren onaylayamaz / aynı kişi iki kez imzalayamaz" istemcide de denetlenir; yoksa yalnız sunucuda.

## Tutaa (`tutanak/`) — yazan: [Claude Fable 5.2 #taa-web-W3-tutanak 2026-11-27]
- **Yeni tutanak:** şablonlar önbellekten (`tutanak/sablonlar`; biçim Tutaa şablonlarıyla aynı: `kimlik, ad, surum,
  bolumler[].alanlar[], tablo, imzalar[]`). Alan türleri: `metin, uzunmetin, sayi, tarih, saat, secim, coklu, onay`
  (bilinmeyen tür metin sayılır). Kayıt `POST /v1/tutanak` gövdesiyle giden kutusuna yazılır:
  `{sozlesme: 'tutanak.olustur@1', sablon, surum, alanlar, satirlar, sha256, olusturma}`. `sha256` =
  `{sablon, surum, alanlar, satirlar}` kanonik (anahtarları sıralı) JSON'unun özeti. Belge numarasını istemci üretmez.
- **Fotoğraftan:** en çok 3 fotoğraf (JPEG / PNG / WebP; tür dosyanın ilk baytlarından anlaşılır). 700 KiB'ı aşan
  fotoğraf tarayıcıda küçültülür (uzun kenar 1600 px, JPEG). Her sayfa ayrı kayıttır:
  `{sozlesme: 'tutanak.foto@1', durum: 'ocr_bekliyor', grup, sayfa, sayfa_sayisi, cekim, dosya: {ad, tur, boyut, sha256, veri_b64}[, sablon][, not]}`.
  **OCR bu modülde yok**; sonuç sunucuda üretilir. Ya bütün sayfalar sıraya girer ya hiçbiri.
- **Tutanaklar:** sunucu özetleri (`tutanak/liste`) + giden kutusundaki kayıtlar; süzgeçler: imza bekleyen,
  gönderilmeyen, OCR bekleyen, reddedilen.
- **İmza:** rol şablonun `imzalar[]` listesinden (şablon bilinmiyorsa Düzenleyen / Tanık / İlgili). İmzalayan adını yazar,
  kutuya imzasını çizer. Çizim görüntü değil nokta dizisidir: `cizim: {en, boy, cizgiler: [[x0, y0, x1, y1, ...]]}`.
  Gövde: `{sozlesme: 'tutanak.imza@1', tutanak_id | istemci_no, tutanak_sha256?, rol, imzalayan, zaman, cizim | imtina + imtina_notu}`.
  Aynı rol iki kez imzalanmaz; aynı kişi iki rolde imzalayamaz; iptal, reddedilmiş ve fotoğraf kayıtları imzalanmaz;
  imtina yalnız şablonun `imtinaKaydi` verdiği rolde, 5-300 karakter notla yazılır.
- **Kasa Çift Doğrulama Tutanağı burada yok** (TAA Kasa'ya taşındı): şablonu ve kayıtları önbellekte gelse de gösterilmez.
- **Varsayımlar:** izin adları `tutanak.oku`, `tutanak.olustur`, `tutanak.imza`, `tutanak.foto`; uçlar `/v1/tutanak/imza`
  ve `/v1/tutanak/foto` (Tutaa API sözleşmesinde yalnız `/v1/tutanak` ve `/v1/tutanak/{id}/ek` var; giden kutusu ucu
  sabit metin olduğu ve tutanağın sunucu kimliği henüz bilinmeyebildiği için düz uç seçildi); önbellek anahtarları
  `tutanak/sablonlar`, `tutanak/liste` (öğe: `{id, no, sablon, durum, tarih, baslik?, sha256?, istemci_no?, grup?, imzalar[]}`).
  `docs/WEB-ESITLE-SOZLESMESI.md` §9 madde 5 fotoğraf için ayrı yükleme ucu öngörür; bu modül görev tanımı gereği
  fotoğrafı JSON giden kutusuna (base64, 1 MiB sınırı içinde) yazar.
- İptal, düzeltme, e-posta gönderme, PDF/yazdırma bu modülde yok.

## Test
```
cd web/moduller && node --test
```
Bağımlılık yok: Node yerleşik test koşucusu, `ortak/sinama/` altında küçük sahte DOM ve sahte bağlam. Gerçek tarayıcıda denenmedi.
Tutaa modülü ek olarak 2026-11-27'de gerçek Chrome (başsız) ile depoda olmayan geçici bir duman testinden geçti (form,
tuvalde imza, fotoğraf küçültme); telefon kamerası ve dokunmatik ekran denenmedi (bkz. `../kabuk/BENIOKU.md` Test).
