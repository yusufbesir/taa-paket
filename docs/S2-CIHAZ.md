# S2 — Cihaz kimliği (taa-cihaz) · Sahip: Yusuf · yazan: [Claude#taa-cihaz-S2 2026-11-06]

Karar (Yusuf 2026-11-06 21:40): Fortinet/VPN kullanılmayacak; bağlantıyı bizim programlar kurar. Sunucu yalnız
**kayıtlı ve onaylı cihazdan** gelen isteği kabul eder (parolaya bakmadan önce). Sözleşme:
`/mnt/project-files/tutaa/API-SOZLESMESI.md` "Cihaz kimliği"; yetki kuralları `planlar/YOL-HARITASI-MERKEZ-2026-11-06.md` §2.

## Ne yapar
- `taa-cihaz uret` → `<kök>\cihaz\cihaz.key` (özel, Unix 0600, üzerine yazmaz), `cihaz.pub` (`ed25519:<hex>`), `cihaz.json` ({ad, kimlik, olusturma}). Anahtar üretimi taa-imza (ed25519-dalek, onaylı); yeni crate yok.
- Her istekte başlıklar: `X-TAA-Cihaz` (kimlik = genel anahtarın ilk 8 baytı, 16 hex), `X-TAA-Zaman` (unix sn), `X-TAA-Imza` (128 hex). İmzalanan bayt dizisi: `"{YONTEM}\n{YOL}\n{ZAMAN}\n"` + gövde baytları.
- `taa-cihaz kayit --sunucu URL [--sube KOD]` → `POST /v1/cihaz/kayit` (imzasız tek uç) → `{kimlik, durum: "bekliyor"}`; yönetici TAA Çatı'dan onaylar. `taa-cihaz durum` → `/v1/cihaz/durum`.
- `taa-kuyruk::Imzalayici` arayüzü: `CurlGonderici::imzalayici(Arc<dyn Imzalayici>)`; başlıklar geçici dosyayla curl'e verilir (komut satırında yok). Başlık tek satır ASCII değilse istek gitmez.
- `taa-kimlik`: `cihaz_imzalayici(kok)` anahtar varsa yükler; `giris` ve `gonder` otomatik imzalar (anahtar yoksa uyarı basar, sunucu reddedebilir; bozuksa hata). `Oturum` yeni alanlar: `izinler[]`, `programlar[]`, `gecerlilik`; `izinli("fis.sil")`, `program_acik("tutaa")` (yonetici her izni taşır; liste boşsa program açık sayılır — eski sunucu uyumu). Sunucu her istekte yine denetler (K6).

## Yapılmadı / sonraki
- Giriş yanıtındaki sunucu imzasının (`imza`, `sunucu_anahtar` kur.toml) doğrulanması ve 7 gün çevrimdışı izin dosyası → S3 ile (taa-kur/taa-cati).
- taa-kur kurulumda `taa-cihaz uret` + `kayit` çağrısı → S3.
- Windows'ta elle deneme (Credential Manager gibi) yapılmadı; testler Linux + CI Windows.

## Deneme (yerel)
```
set TAA_VERI_KOK=D:\deneme\veri
taa-cihaz uret --ad "Pendik PC"
taa-cihaz goster --json
taa-cihaz kayit --sunucu http://127.0.0.1:8443 --sube PND
echo parola | taa-kimlik giris --sunucu http://127.0.0.1:8443 --kullanici ali
```
Çıkış kodları taa-cihaz: 0 tamam, 2 kullanım, 3 ulaşılamadı, 4 sunucu reddetti, 5 anahtar yok, 1 diğer.

## S3a — Yetki belgesi ve kurulum kancası · yazan: [Claude#taa-kimlik-S3a 2026-11-06]
Sunucu başlığı (tutaa PR #23) sözleşmeyi uyguladı; istemci buna göre:
- **Giriş imzası:** `<veri kökü>\anahtarlar\sunucu.pub` (`tutaa-sunucu anahtar sunucu.pub` üretir; şube paketine konur) varsa `taa-kimlik giris` sunucu yanıtının `imza`sını doğrular (kanonik bayt, `imza` hariç); yanlış/imzasız yanıt reddedilir. Dosya yoksa uyarı basılır, doğrulanmaz (sunucu kurulana kadar).
- **Çevrimdışı yetki belgesi:** girişten sonra `POST /v1/yetki {}` (oturumlu + cihaz imzalı) → `<kök>\kimlik\yetki-<sunucu>.json` (`{"belge": <imzalı, oturumsuz>, "alinma", "sunucu"}`). `taa-kimlik yetki --sunucu URL [--program ad] [--izin program.islem]` internet olmadan karar verir: çıkış 0 izinli, 7 yetkisiz, 5 belge yok/süresi geçmiş/imzası tutmuyor (**varsayılan: reddet**, K1). `gecerlilik` yoksa alınma + 7 gün. `yonetici` rolü her izni taşır (K4); sunucu her istekte yine denetler (K6).
- **409 tekrar:** aynı saniyede birebir aynı imzalı istek sunucuda 409 alır; taa-kuyruk bunu kalıcı ret değil "yeniden dene" sayar (kuyruk durur, sonraki çalıştırmada yeniden imzalanır).
- **`taa-kur cihaz [--ad A] [--sunucu URL] [--sube K]`:** kurulumda cihaz anahtarını üretir (varsa korur), `--sunucu` verilirse kaydeder. taa-kur → taa-cihaz bağımlılığı eklendi (yeni dış bağımlılık yok).
- Çatı ilk cihazı: sunucu PC'de `tutaa-sunucu merkez-cihaz ed25519:<hex> "ad"` ile Çatı'nın anahtarı onaylı kaydedilir (onay ucu yönetici oturumu ister).
- Yapılmadı: taa-kur penceresine "Cihaz" düğmesi; taa-cati programı (S3b).
