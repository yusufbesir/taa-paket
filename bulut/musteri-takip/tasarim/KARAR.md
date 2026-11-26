<!-- Sahip: Yusuf. yazan: [Claude#terzitakip-tasarim 2026-11-26]. Müşteri takip sayfası tasarım kararı. Sayfa kodu ortak/takip.mjs'dedir. -->
# Müşteri takip sayfası: tasarım kararı

**Seçilen: Varyant A "Zanaat kartı", B'den dikey adım listesi ve Astra'nın erişilebilirlik düzeltmeleriyle (A+).**

## Süreç
- Stitch'te (proje 306477370923004111) üç varyant üretildi: **A** zanaat kartı, **B** durum öncelikli dikey zaman çizelgesi (koyu), **C** koçan/fiş. Görüntüler: `stitch/`.
- İkinci görüş: **Astra (gpt-6-astra)** eleştirdi ve A'yı önerdi (kontrast A 5,30:1, B 7,99:1, C 5,88:1; C'de dikiş çizgisi RTL'de zorlanır; B'nin iki sütunlu alt dil paneli JS'siz çekmece koşuluna uymaz, soluk özet AA'yı belirsiz bırakır). **Gemini kullanılamadı**: kota doldu (RESOURCE_EXHAUSTED, ~117 saat). Başka model konmadı.
- Sırsız istem kullanıldı (uydurma örnek veri; müşteri adı, telefon, gerçek fiş verisi yok).
- Karar (Opus/Fable rolü): A, çünkü mevcut canlı sayfanın sıcak kahverengi diline uyuyor, JS'siz hafif; B'den dikey adım listesi alındı (uzun çeviri ve RTL'de yatay çizgi sıkışıyor); C'nin barkod/fiş no süsleri ve fiş numarası **kayıtta yok** (kişisel veri kuralı), bu yüzden elendi.

## Düzen (390 px, tek sütun, en çok 480 px)
1. Üst satır: sol "Terzi Adem Altun" (20 px, 700); sağ `<details>` dil çekmecesi (kapalıyken **kısaltma**, ör. `FR`; 48 px yükseklik, 64 px genişlik). RTL'de (ar, fa) çekmece **sol üste** geçer (`inset-inline-end`).
2. Durum kartı: 48 px yuvarlak onay simgesi + durum başlığı (24 px, 700) + alt satır (14 px). Altında **dikey** `<ol>` dört adım (Alındı → Terzide → Hazır → Teslim edildi), satır en az 48 px; biten adım dolu daire, güncel adım halkalı + "Güncel" etiketi (`aria-current="step"`), sonraki adım soluk. "Hazır" iken not kutusu: "…saat 17:00'ye kadar teslim alabilirsiniz. Hazır olduktan sonra 14 gün saklanır."
3. Fiş özeti kartı: satır satır **Fiş tarihi** ve **Teslim** (gün adı + saat), altında kalemler (ürün türü kalın, ikinci satırda işlem · varyant). Kişi adı, telefon, fiyat, fiş numarası **yok**.
4. Alt: "Şube MRK01" (sol), "Aydınlatma metni" bağlantısı (sağ, 48 px dokunma alanı).
5. Kayıt bulunamadı ve iptal ekranları: aynı üst satır; ilerleme listesi yok, tek kart + açıklama.

## Dil çekmecesi (JS'siz)
- `<details class="dil"><summary>FR</summary><ul>…</ul></details>`; `summary` erişilebilir adı "Dil seçimi: Français".
- Liste tek sütun, 13 dil, her satır `FR = Français` (kısaltma monospace + vurgu rengi, "=" , dilin kendi adı `<bdi lang>` içinde). Satır `dir="ltr"` (karma metin sırası bozulmasın), 48 px.
- Genişlik `min(320px, calc(100vw - 32px))`, `max-height: 70dvh`, dikey kaydırma. Seçili satır `aria-current="true"`, kalın, zemini `--not`.
- Her satır `?dil=xx` bağlantısıdır (sunucu çevrilmiş HTML döner); `lang`/`hreflang` verilir. Dil çerezle hatırlanır ve tarayıcı dilinden seçilir (sunucu tarafında, ayrı iş).

## Renkler
| Jeton | Açık | Koyu |
|---|---|---|
| `--zemin` | `#f6f3ee` | `#14181c` |
| `--kart` | `#ffffff` | `#1d2329` |
| `--yazi` | `#29251f` | `#e8ecf3` |
| `--ince` (ikincil) | `#665c50` | `#b7c0cc` |
| `--vurgu` | `#8a5a2b` | `#d7a46b` |
| `--vurgu-yazi` (vurgu üstünde) | `#ffffff` | `#14181c` |
| `--cizgi` | `#e4ddd2` | `#323a43` |
| `--not` (not/kalem zemini) | `#efe8dc` | `#262d34` |

Not: koyu temada vurgu Astra'nın önerdiği mavi (`#6fb1ff`) yerine sıcak `#d7a46b`; marka tutarlılığı için (açıkta kahverengi). Kontrast: `#8a5a2b` / `#f6f3ee` 5,3:1; `#d7a46b` / `#1d2329` yaklaşık 7:1 (örnek sayfalarda gözle ve araçla yeniden doğrulanmalı; tam AA denetimi uygulama sonrası).

## Ölçüler
Sistem yazı tipi (`system-ui, -apple-system, "Segoe UI", Roboto, "Noto Sans", sans-serif`); gövde 16 px / satır 1,5; marka 20 px; durum 24 px; yardımcı 14 px; etiket 13 px. Kenar ve kart içi boşluk 16 px (hero kartı 20/16), kart arası 16 px, satır arası 8 px, köşe 12 px (kalem kutusu 8 px). Dokunma hedefleri en az 48 px. Odak halkası 3 px `--vurgu`, 2 px ofset. `prefers-reduced-motion` ve `prefers-color-scheme` desteklenir; çekmece oku dönüşü 150 ms, yalnız hareket tercihi serbestse.

## Dosyalar
- `uret.mjs`: seçilen tasarımın örnek HTML'lerini üretir (`node uret.mjs <klasör>`): TR açık/çekmece, EN, AR (RTL) ve çekmece.
- `ekranlar/`: 390 px ekran görüntüleri (TR açık, TR koyu, TR çekmece açık/koyu, AR, AR çekmece).
- `stitch/`: üç Stitch varyantının ekran görüntüleri.
- Uygulama (ortak/takip.mjs ve diller.mjs) bu dosyaları dokunmadan başka oturum yazar; bu klasör yalnız tasarım girdisidir.
