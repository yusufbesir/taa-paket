<!-- Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27] -->
# TAA Web modülleri (`web/moduller/`)

Plan: `/mnt/project-files/planlar/PLAN-TAA-WEB-2026-11-27.md` (W1). Her modül `web/moduller/<ad>/index.js` dosyasından
`{ad, baslik, ac(kok, baglam)}` dışa aktarır; `baglam = {giden, onbellek, yetki(izin) -> bool, tema}`. Modüller veriyi yalnız
bu bağlam üstünden okur ve yazar: ağ çağrısı yapmaz, `giden.gonder` çağırmaz, önbelleğe yazmaz. Silme yok.

| Modül | Ne yapar | Yazar mı |
|---|---|---|
| `kasa` | Sayım (açılış / ara / devir), masraf, gün sonu (canlı fark paneli, fark notu zorunlu), imza ve gerekçe (çift doğrulama) | evet |
| `terzitakip` | Fiş durumu listesi, geciken / bugün teslim / kargo süzgeçleri, arama, durum değiştirme | evet |
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

## Test
```
cd web/moduller && node --test
```
Bağımlılık yok: Node yerleşik test koşucusu, `ortak/sinama/` altında küçük sahte DOM ve sahte bağlam. Gerçek tarayıcıda denenmedi.
