<!-- Sahip: Yusuf. yazan: [Claude#taa-paket-yeni 2026-11-06] -->
# Usta puanı (QR) — `taa-takip::puan`

Tam gerekçe ve araştırma: `arastirma/USTA-PUAN-CETVELI-2026-11-06.md` (proje dosyaları).

## Kurallar (Yusuf 2026-11-06)
- Usta işi bitirince QR okutur; süre tutulmaz. Kazanılan dakika = Σ norm süre (okutma anında kaydedilir).
- Yeniden iş / müşteri şikâyetini TEZGÂHTAR işaretler; YÖNETİCİ açıklamalı tutanak yazar. Tutanaksız işaret yarım ağırlıkla sayılır; "usta kaynaklı değil" ise puana girmez.
- Usta kendi puanını GÖRMEZ. Veriyi yalnız yönetici görür. Kütüphane izin denetlemez: `takip.puan@1` iznini çağıran program yalnız yöneticiye vermelidir.
- Özellikler `qr-uretim` (kayıt) ve `usta-puan` (hesap), varsayılan KAPALI. Gölge mod: kayıt açık, hesap yönetici ekranında.

## Veri
`<dizin>/usta-puan.jsonl`: eklemeli (silme/değiştirme yok). Kayıt türleri: `olay`, `isaret`, `tutanak`, `izin`, `ayar_degisti`. Aynı fiş+kalem+işlem iki kez okutulamaz. Ayar: `<dizin>/puan-ayar.json` (yoksa varsayılan).

## Hesap
Puan = 10 × Σ ağırlık × bileşen, bileşenler 0–1:
- verimlilik = min(kazanılan_net / mevcut dk / hedef, 1). Net = kazanılan − ceza (yeniden iş 2×, şikâyet 3× norm dk × ağırlık). Mevcut = çalışılan gün × `gunluk_dakika` (kapalı günler ve izin hariç).
- kalite = 1 − yeniden iş ağırlığı / iş adedi; ağır hata tutanağı varsa en çok 0,60.
- müşteri = 1 − şikâyet ağırlığı / iş adedi. zamanında = okutma ≤ fiş teslim tarihi (veri yoksa önsel). düzen = 1 − toplu okutulan iş oranı (aynı gün peş peşe ≤ 2 dk aralıkla 6+).
- Küçük örnek: (C·ekip ortalaması + n·değer) / (C + n), C=30. Deneme dönemi (ilk 28 gün): puan yok.
- Varsayılan ağırlıklar 35/35/15/10/5, hedef %85. Sınırlar: verimlilik ve kalite 0,20–0,40, diğerleri 0,05–0,30.
- Henüz yok: zamana göre azalan ağırlık (yarı ömür), işlem bazlı hedef için veri yok (`hedef_islem` boş).

## Kendini ayarlama (`gozden_gecir`)
Öneri üretir, çağıran uygular ve `PuanDefteri::ayar_degisti` ile kaydeder. Koşullar: `otomatik` açık, ≥200 okutma, ≥28 gün veri, son ayardan ≥28 gün, ≥2 deneme dışı usta. Hedef oran, ustaların medyan verimliliğinin 1,1 katına doğru en çok 0,05 adımla; ağırlıklar (≥3 usta) bileşen ayrışmasına doğru en çok 0,03 adımla, sınırlar içinde, toplam 1. Elle değişiklik her zaman geçerli; `otomatik: false` ile kapatılır.

## Anomali (yalnız yönetici)
Verimlilikte ani düşüş (son 7 gün, önceki 90 güne göre ≥2 sapma), yeniden iş artışı (14 gün, 2×), toplu okutma, norm gözden geçir (30 gün ortalama > %115).

## Sunucu uçları (taa-fis-sunucu)
- Fişteki mevcut QR (`/telefon#fis=F-…`) telefonda fişi açar; fiş sayfasında **İşi bitirdim (okut)** ve **Yeniden iş / şikâyet işaretle** düğmeleri var.
- `POST /api/fis/{no}/okut` (telefon belirteci): gövde isteğe bağlı `{kalemler:[0,1], zor:[0], usta:"Ali"}`. Kalem verilmezse hepsi. Usta yoksa telefonun adı. Norm süre `sure.json`'dan okutma anında yazılır. Yanıt `{okutulan, zaten}`; aynı kalem iki kez okutulmaz. `qr-uretim` kapalıysa 503.
- `POST /api/fis/{no}/isaret` (telefon, tezgâhtar): `{kalem, tur: yeniden_is|sikayet, neden}`. Usta = kalemi okutan usta, yoksa fişteki usta.
- Yönetici (yalnız sunucu bilgisayarından, `yerel`): `POST /api/yonetim/tutanak` `{isaret_id, sonuc: usta_kaynakli|usta_kaynakli_degil|kismen, yuzde, agir, aciklama, yonetici}`; `GET /api/yonetim/puan/{başlangıç}/{bitiş}` (puanlar + anomaliler); `POST /api/yonetim/puan/gozden-gecir/{başlangıç}/{bitiş}` (koşullar sağlanırsa ayarı günceller, kaydeder, deftere yazar). `usta-puan` kapalıysa 503.
- Yönetici ekranı: `http://<sunucu>/yonetim/puan` (yalnız sunucu bilgisayarından). Aralık seçilir; puanlar, bileşenler, yorumlar, ani değişim uyarıları, tutanak bekleyen/yazılmış işaretler ve tutanak formu, "ayarları veriye göre gözden geçir" düğmesi. Veri `GET /api/yonetim/isaretler` ve yukarıdaki uçlardan gelir.
- Telefon belirteci ustaya da verilebildiği için puan uçları telefondan KAPALI; Çatı rolleri (S3) gelince yönetici izni oradan yönetilecek.
- Fişe ayrıca kalem başı QR basılmadı: fiş QR'ı yeterli, okutmada fişin kalemleri birlikte kapanır; tek kalemlik okutma `kalemler` ile yapılır.

## Yapılmadı
Kalem başı QR etiketi, zor durum sorusunun okutmada sorulması (`zor` dizisi uçta var, ekranda yok), KVKK aydınlatma metni (avukat).
