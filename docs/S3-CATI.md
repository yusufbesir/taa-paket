# S3b — TAA Çatı v0 (taa-cati) · Sahip: Yusuf · yazan: [Claude#taa-cati-S3b 2026-11-06]

Karar (Yusuf 2026-11-06 21:40): Çatı programı tüm TAA paket programlarını denetler, **yalnızca sunucu bilgisayarında**
çalışır ve diğer programlardaki yetkilendirmeyi yapar. Kurallar: yol haritası §2 (K1–K13). Sunucu uçları:
`/mnt/project-files/tutaa/API-SOZLESMESI.md` "Yönetim uçları" (tutaa PR #23).

## Ne yapar
- `taa-cati` komut satırı (v0; pencere sonraki adım). Her istek **cihaz imzası + Bearer** taşır; oturum taa-kimlik
  deposundan (Windows: Kimlik Bilgisi Yöneticisi), cihaz anahtarı `<kök>\cihaz\` (yoksa hata: `taa-cihaz uret`, sunucuda
  `tutaa-sunucu merkez-cihaz`).
- **Varsayılan reddet (K1):** komut, oturumun izin listesinde gerekli izin yoksa sunucuya gitmeden çıkış 7 verir;
  `yonetici` rolü her izni taşır (K4). Sunucu yine her istekte denetler (K6).
- **Yazma işlemleri** son 15 dk içinde parola doğrulaması ister: `taa-cati dogrula` (parola stdin'den) → `POST /v1/yeniden-dogrula`.
  Sunucu 401 `yeniden_dogrula` dönerse çıkış 6 ve öneri basılır.
- **Silme yok (K10):** kullanıcı `kapat`, cihaz `iptal`; günlük yalnız okunur.
- Komutlar: `kullanici liste|ekle|rol|rol-kaldir|kapat|parola`, `rol liste`, `izin liste`, `cihaz liste [--durum]|onay|iptal`,
  `program-yetki liste|ayarla (--acik|--kapali)`, `gunluk [süzgeçler]`, `sube liste|ekle`, `dogrula`.
- Girdi doğrulama sunucuya gitmeden: tanıtıcılar `[a-z0-9][a-z0-9_.-]*` (≤64), cihaz kimliği 16 hex, parola ≥ 8,
  günlük `limit` 1..=500, tarihler `YYYY-AA-GG[THH:MM:SSZ]`.
- Parola ve oturum anahtarı hiçbir dosyaya/ekrana yazılmaz; `--parola` seçeneği reddedilir.
- taa-kuyruk: `Gonderici::al` (GET) eklendi; `uc_sorgulu_gecerli` sorgu dizesini sınırlar. Yeni dış crate yok.

## Kullanım (sunucu PC'si)
```
set TAA_SUNUCU=https://sunucu.terziademaltun.com
set TAA_VERI_KOK=D:\TAA\veri
taa-cihaz uret --ad "Merkez sunucu"          (sunucuda: tutaa-sunucu merkez-cihaz ed25519:<hex> "Merkez sunucu")
echo parola | taa-kimlik giris --kullanici yonetici
echo parola | taa-cati dogrula
taa-cati cihaz liste --durum bekliyor
taa-cati cihaz onay --kimlik 0123456789abcdef --sube 2
echo yeni-parola | taa-cati kullanici ekle --ad ayse --gorunen-ad "Ayşe Yılmaz" --rol kasa --sube 2
taa-cati program-yetki ayarla --program taakasa --rol personel --kapali
taa-cati gunluk --islem giris --limit 50
```
Çıkış kodları: 0 tamam, 2 kullanım, 3 ulaşılamadı, 4 sunucu reddetti, 5 oturum/cihaz yok, 6 `dogrula` gerekli, 7 yetkisiz, 1 diğer.

## Yapılmadı / sonraki
- Çatı penceresi (sekmeler: Kullanıcılar, Cihazlar, Program yetkileri, Günlük) → S3c; önce komut satırı sahada denenir.
- Sunucu tarafı "yalnız merkez cihazından yönetim" kilidi (cihaz `sube_id = null` olmayanın yönetim uçlarına
  erişememesi) sunucu başlığında; istemci bunu varsaymaz.
- Günlük dışa aktarma (CSV) ve dönemsel yetki gözden geçirme raporu (K12) → sonraki.
- Windows'ta elle deneme yapılmadı; testler Linux + CI Windows.
