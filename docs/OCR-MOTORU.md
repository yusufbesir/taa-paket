<!-- Sahip: Yusuf. Ortak OCR motoru sözleşmesi (taslak 0.1). Karar: Yusuf 2026-11-12. Değişiklik: Yusuf'un onayıyla. -->
# TAA ortak OCR motoru (`taa-ocr`) sözleşmesi

Karar (Yusuf, 2026-11-12): OCR tek olacak, zorunlu kurulacak, eğitim ve düzey arttıkça güncellenecek.
Tutaa, TAA Terzi Takip, TaaRkod, stok kodu programı, taa-tara ve yeni programlar (TAA Rehber, TAA Eğitim)
kendi OCR'ını taşımaz; bu motoru çağırır.

## 1. Paket

| Alan | Değer |
|---|---|
| Paket adı | `taa-ocr` (motor) |
| Model paketi | `taa-ocr-model` (ayrı sürümlenir, ayrı imzalanır) |
| Katalog grubu | `Altyapı` |
| Zorunluluk | `zorunlu_roller = ["sube","sunucu"]`: her kurulumda sormadan gelir |
| Kurulum yeri | `<kök>\ocr\motor\<surum>\` ve `<kök>\ocr\modeller\<ad>\<surum>\` |
| Ağ | Yok. OCR tamamen çevrimdışı çalışır, görüntü hiçbir yere gönderilmez (KVKK). |

`modul.toml` taslağı:

```toml
ad = "taa-ocr"
surum = "0.1.0"
arayuz = "1"
gerektirir = { "taa-dosya" = "1" }
program = "ortak"
sunar = ["ocr.oku@1", "ocr.kod@1", "ocr.tablo@1", "ocr.bilgi@1"]
ister = []
veri_sahibi = ["ocr-modeller"]
veri_okur = []
ayar_alani = "programlar.taaocr"
```

`veri-sahipleri.toml` tablosuna `ocr-modeller = "taa-ocr"` satırı, motor kodu eklenirken yazılır.

## 2. Yetenekler (IPC adı `ad.fiil@N`)

| Yetenek | Girdi | Çıktı |
|---|---|---|
| `ocr.oku@1` | görüntü, PDF veya docx (dosya yolu veya bayt; bölüm 2b), `dil` ("tr"), `tur` ("basili" / "elyazisi" / "otomatik") | metin, güven, satır listesi |
| `ocr.kod@1` | görüntü, `sozluk` (geçerli kodlar listesi veya dosyası) | sözlükle kısıtlı kod adayları ve güvenleri (stok kodu, barkod altı yazısı) |
| `ocr.tablo@1` | görüntü (tutanak, fiş, fatura tablosu), `sozluk` (isteğe bağlı kod listesi) | satırlar, her tablo için tür, kutu, satırlar, hücre kutuları, adet ve kod uzlaşması (Tutaa'nın mevcut tablo okuması, birden fazla tablo) |
| `ocr.bilgi@1` | yok | motor sürümü, etkin model adı ve sürümü, desteklenen türler |

`ocr.tablo@1` bir görüntüde birden fazla tablo okuyabilir (terzi fişi, tutanak, fatura vb.). Sonuç:

```json
{
  "arayuz": 1,
  "goruntu": {"w": 0, "h": 0},
  "tablolar": [
    {
      "tur": "tutanak",
      "kutu": [x, y, g, y],
      "belirsiz": false,
      "satirlar": [
        {
          "kutu": [x, y, g, y],
          "stok": {"ham": "…", "secilen": "…", "aday": [{"deger": "…", "guven": 0.0}], "guven": 0.0, "durum": "…"},
          "adet": {"ham": "…", "deger": 0, "guven": 0.0, "durum": "…"},
          "hucreKutusu": [x, y, g, y],
          "adetKutusu": [x, y, g, y]
        }
      ]
    }
  ]
}
```

- `tur`: "tutanak", "terzi-fisi", "fatura" ya da "bilinmiyor". Tür tahmini de `guven` ile gelir, düşükse "bilinmiyor".
- `satirlar` alanları Tutaa'nın mevcut `motor/PROTOKOL.md` yanıtıyla aynıdır (stok.ham/secilen/aday/guven/durum, adet.ham/deger/guven/durum, hucreKutusu, adetKutusu). Başka belge türlerinde `stok`/`adet` yerine `hucreler: [{"sutun","kutu","metin","guven"}]` gelir; ikisi birlikte de olabilir.
- Kutu koordinatları `goruntu.w/h` piksel boyutuna göredir.
- Tablo bulunamazsa `tablolar` boş, üst düzeyde `belirsiz: true` döner.
- `durum` (stok ve adet için) yalnız "kabul", "sor" ya da "bos" olur (Tutaa `paddle_motor.py` ile doğrulandı). "kabul" güven eşiğini geçti, "sor" kullanıcıya sorulmalı, "bos" okunamadı demektir. Ürün durumları (örn. "degismis") OCR durumu değildir, buraya girmez.

### Sonuç biçimi (JSON, tek satır)

```json
{
  "arayuz": 1,
  "motor": "0.1.0",
  "model": {"ad": "elyazisi", "surum": "0.3.0"},
  "metin": "…",
  "guven": 0.0,
  "satirlar": [{"metin": "…", "guven": 0.0, "kutu": [x, y, g, y]}],
  "adaylar": [{"deger": "…", "guven": 0.0}],
  "belirsiz": true
}
```

- `guven` 0 ile 1 arasında. `belirsiz` motorun kendi eşiğinin altında demektir.
- Motor tahmin uydurmaz: okuyamazsa `belirsiz: true` ve boş `metin` döner, hata değildir.
- Eşik altı sonuçta kullanıcıya sormak çağıran programın işidir (mevcut el yazısı OCR kuralı).
- Hata (dosya yok, bozuk görüntü, motor yok) çıkış kodu ve `{"hata": "…"}` ile bildirilir.

## 2b. Girdi türleri (Yusuf, 2026-11-12)

OCR girdisi yalnız görüntü değildir. Motor şunları kabul eder: görüntü (JPEG, PNG, TIFF, fotoğraf, tarama),
PDF (metin katmanlı ve taranmış), Word (docx) ve benzeri belge biçimleri (sonradan eklenir, bölüm 5b).

- Metin katmanı varsa (PDF metni, docx metni) doğrudan çıkarılır, OCR çalıştırılmaz. Yoksa sayfa görüntüye çevrilip OCR'a gider. Karma belgede sayfa ve bölge bazında karar verilir (gömülü görsel varsa o OCR'dan geçer).
- Her sonuç ve her satır/tablo için kaynağın hangi yoldan okunduğu `kaynak` alanında bildirilir: `"metin-katmani"` ya da `"ocr"`. Metin katmanından gelen sonucun `guven` değeri 1.0, `durum` "kabul" olur.
- Çok sayfalı girdide sonuçlar `sayfalar: [{"no": 1, "goruntu": {"w":0,"h":0}, "kaynak": "...", ...}]` altında sayfa sayfa gelir; tek görüntü tek sayfa sayılır. Tek sayfalık basit kullanımda üst düzey alanlar korunur (geriye uyumlu).
- Belge türü (`tur`) listesine "defter" (el yazısı defter sayfası) eklenir; ayrıca "tutanak", "terzi-fisi", "fatura", "fotograf" (belge olmayan görüntü), "bilinmiyor".
- Girdi tespiti dosya uzantısına değil içeriğe bakar (imza baytları). Desteklenmeyen biçimde `{"hata": "desteklenmeyen-bicim"}` döner, uydurma sonuç dönmez.
- Şifreli veya parola korumalı PDF/docx açılmaz, `{"hata": "korumali"}` döner.

## 3. Çağırma yolları (yedekli, bağlantı kuralı)

1. **IPC**: `taa-ipc` üzerinden yetenek adıyla. Motor çalışıyorsa ilk yol budur.
2. **Komut satırı**: `taa-ocr oku <dosya> --json [--tur elyazisi] [--dil tr]` ve
   `taa-ocr kod <dosya> --sozluk <dosya> --json`, `taa-ocr tablo <dosya> [--sozluk <dosya>] --json`
   ve `taa-ocr bilgi --json` (sonuçta `"yetenekler": ["ocr.oku@1", "ocr.tablo@1", …]`). IPC yoksa çağıran alt süreç olarak çalıştırır.

Çağıran program iki yolu da dener, hangisi çalışırsa onu hatırlar. Yetenek hiç yoksa (motor kurulu değilse)
ilgili düğme gizlenir; program yerelde eski OCR'a düşmez (tek OCR kuralı). İstisna: geçiş döneminde
programın kendi OCR'ı `--eski-ocr` ayarıyla açıkça seçilebilir, bu ayar yeni kurulumda kapalıdır.

## 4. Model sürümleme ve güncelleme

- Motor ve model ayrı paket: eğitim ilerleyince yalnız `taa-ocr-model` yeni sürümle yayınlanır, programlara dokunulmaz.
- Model paketi `taa-yayin` kataloğunda imzalı gelir (ed25519, mevcut `taa-guncelle` akışı). İmzasız model yüklenmez.
- `<kök>\ocr\modeller\etkin.json` hangi model sürümünün etkin olduğunu tutar. Yeni sürüm yüklenince eskisi
  `eski\` altında kalır; kötü sonuçta tek komutla geri dönülür (`taa-ocr model-geri-al`). Silme yok.
- Yeni model etkinleştirilmeden önce sabit sınama kümesinde ölçülür. Ölçüm eskisinden kötüyse paket
  yayına alınmaz (mevcut durdurma kuralı: gelişim ölçülmezse terfi yok).
- `arayuz` ana sürümü yalnız sonuç biçimi uyumsuz değişirse artar; model güncellemesi arayüzü değiştirmez.

## 5. Güvenlik ve gizlilik

- Görüntüler yalnız çağıranın verdiği yoldan okunur, motor kendi başına dosya taramaz.
- Sonuçlar motorda saklanmaz, günlükte metin tutulmaz (yalnız süre, tür, güven, hata kodu).
- Geçici dosyalar `taa-dosya` geçici klasöründe, iş bitince temizlenir.

## 5b. Genişletilebilirlik (Yusuf: ileride farklı geliştirmelere açık olmalı)

- Yeni yetenek yeni adla ve sürümle eklenir (örn. `ocr.imza@1`, `ocr.form@1`); var olan yetenek ve alanlar kaldırılmaz, anlamı değişmez.
- Çağıran sonuç JSON'unda tanımadığı alanları yok sayar. Motor yeni alan ekleyebilir, eski programlar bozulmaz.
- Yetenekler `ocr.bilgi@1` ile sorgulanır (ad ve sürüm listesi, desteklenen belge türleri, etkin modeller). Program kullanmadan önce sorar; yoksa ilgili düğmeyi gizler.
- Yeni belge türü (`tur` değeri) ve yeni model, çağıran programa dokunmadan eklenir. Bilinmeyen `tur` "bilinmiyor" gibi ele alınır.
- Uyumsuz değişim yalnız `arayuz` ana sürümünü artırır; eski ve yeni arayüz bir süre yan yana sunulur (`docs/modul-sozlesmesi.md` uyumluluk kuralı).

## 6. Bağlanması gereken programlar (tespit, 2026-11-12)

Kaynak: depoların main dalı, ayrıntı `/mnt/project-files/ocr/OCR-BAGLANTI-TESPITI-2026-11-12.md`.

| Program | Şimdiki OCR | Yapılacak |
|---|---|---|
| Tutaa | tesseract.js yok. İki OCR: tarayıcıda ElYazisiOCR (JS CNN, yedek) ve masaüstü Paddle servisi (`main.js:400` sabit `D:\Programlar\El-Yazisi-OCR\venv` yolu) | iki OCR'ı kaldır; tablo okuma `ocr.tablo@1`, tek alan `ocr.kod@1`, genel metin `ocr.oku@1` |
| Stok kodu programı (fatura-urun-kodlari) | tesseract.js kopyası ve el yazısı JS motoru (proje klasöründeki kopyadan) | depoya erişilince `ocr.kod@1`/`ocr.tablo@1` (13.127 kodluk sözlük) |
| TaaRkod | depoda OCR kodu yok | bağlama gerekmiyor; "OCR geçen kod" Yusuf'tan teyit |
| TAA Etiket | yok | gerek yok |
| taa-tara (bu depo) | `ocr()` "henüz yok" hatası döner | motor çağrısına bağla |
| TAA Terzi Takip (taa-fis) | yok, "sonraki aşama" notu | fiş ve foto çözümleme için `ocr.oku@1` |
| TAA Rehber, TAA Eğitim | yeni | gerekirse baştan `ocr.oku@1` |

## 7. Hedef ve yol haritası (Yusuf, 2026-11-12)

Hedef: her türlü yazıyı okuyabilen, çok kapsamlı bir çözümleyici. Motor bunun için sürekli geliştirilir;
sözleşme (bölüm 5b) bunu çağıran programlara dokunmadan mümkün kılar. Planlanan yetenek alanları, sıra Yusuf kararıyla:

1. Basılı yazı ve belge biçimleri: görüntü, PDF, docx (`ocr.oku@1`, tür "basili", bölüm 2b).
2. El yazısı, özellikle stok kodu ve tutanak (`ocr.kod@1`, `ocr.oku@1`, tür "elyazisi"). İlk model mevcut el yazısı OCR çalışmasından gelebilir.
3. Tablo ve çoklu tablo (`ocr.tablo@1`): tutanak, terzi fişi, fatura.
4. Form alanları (`ocr.form@1`, planlı).
5. İmza bölgesi tespiti (`ocr.imza@1`, planlı). Okuma değil, imzanın varlığı ve konumu.
6. Barkod altı yazı (`ocr.oku@1` ile, kutu verilerek).
7. Belge türü tanıma (`tur` alanı: tutanak, terzi-fisi, fatura, bilinmiyor, ileride yenileri).

Her yeni yetenek ayrı model ve ayrı ölçümle gelir; sabit sınama kümesinde gelişim görülmezse yayına alınmaz.

## 8. Açık


- Motorun uygulaması (Python ve ONNX ya da Rust) bu sözleşmeden bağımsız; dil kuralı (Rust + Python, Rust ağırlıklı) ve ölçüm sonucuna göre seçilecek.
- El yazısı OCR'ın mevcut Python prototipi ve JS portu (el-yazisi-ocr) motorun ilk "elyazisi" modeli olacak mı, Yusuf kararı.
- Türkçe doğruluk düşük (Paddle'da harf %12-15), OCR eğitimi en sona bırakıldı. Motor kurulumu bunu beklemeden hazırlanabilir, model sonra gelir.
