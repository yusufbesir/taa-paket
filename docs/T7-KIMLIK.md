<!-- Sahip: Yusuf. yazan: [Claude#taa-kimlik-T7 2026-11-06] -->
# T7: Kimlik (`taa-kimlik`) — merkez sunucuya giriş ve oturum saklama

Kaynak: Yusuf 2026-11-06 "Merkez sunucu: PostgreSQL, ücretsiz". Sözleşme: Tutaa `API-SOZLESMESI.md`
(`POST /v1/giris {kullanici, parola[, kod]} -> {oturum, sube_id, roller[]}`; her istek `Authorization: Bearer <oturum>`).
Sunucu kodu Tutaa deposunun işidir; bu crate istemci tarafıdır. Kuyruk: `docs/T6-KUYRUK.md`.

## Ne yapar
| Parça | İş |
|---|---|
| `giris()` | `POST /v1/giris`; yanıttan `Oturum` (sunucu, kullanıcı, şube, roller, zaman, anahtar) |
| `OturumDeposu` | sunucu başına bir oturum: `kaydet / oku / cikis` |
| `KimlikDeposu` (Windows) | **Windows Kimlik Bilgisi Yöneticisi**, genel kimlik bilgisi, hedef `TAA:<sunucu>`, yerel makine kalıcılığı (`windows-sys` Credentials, Yusuf onaylı 0.61) |
| `DosyaDeposu` (Windows dışı) | `<kök>/kimlik/oturum-<sunucu>.json`, Unix 0600 — **yalnız test içindir** |
| `kuyrugu_gonder()` | depodaki oturumla `<kök>/kuyruk/*` altındaki tüm program kuyruklarını gönderir (taa-kuyruk) |

Kurallar:
- Parola komut satırına yazılmaz (stdin'den okunur), hiçbir yere kaydedilmez; yalnız giriş isteğinde sunucuya gider.
- Oturum anahtarı ekrana/JSON'a basılmaz (`Oturum::ozet()` ve `Debug` anahtarı gizler); depo dışına çıkmaz.
- HTTP için yeni crate yok: taa-kuyruk `CurlGonderici`. Kendi TLS sertifikamız `--sertifika <pem>`.
- Çıkış: Windows'ta kimlik bilgisi kaldırılır; dosya deposunda dosya silinmez, içerik boşaltılır.

## Komut
```
taa-kimlik giris  --sunucu <https://adres> --kullanici <ad> [--kod <kod>] [--sertifika <pem>] [--json]   (parola stdin'den)
taa-kimlik durum  --sunucu <https://adres> [--json]
taa-kimlik cikis  --sunucu <https://adres>
taa-kimlik gonder --sunucu <https://adres> [--kok <veri kökü>] [--sertifika <pem>] [--en-cok N] [--json]
```
`--sunucu` verilmezse `TAA_SUNUCU`. Çıkış: 0 tamam, 2 kullanım, 3 sunucuya ulaşılamadı, 4 giriş reddedildi, 5 oturum yok, 1 diğer.

## Denetim
Linux: `cargo test -p taa-kimlik` (5 test). Windows hedefi: `cargo clippy -p taa-kimlik --target x86_64-pc-windows-gnu` temiz;
Kimlik Bilgisi Yöneticisi gerçek Windows'ta elle denenmeli (CI'da yazma izni yok sayılmamalı).

## Sonraki adımlar
- taa-kur: `taa-kimlik gonder`i zamanlayıcıyla çalıştırma; pencerede Giriş düğmesi + "bekleyen N".
- `GET /v1/ben` ile oturum süresi denetimi (sözleşmede süre alanı yok; sunucu tarafı belirlenince).
- Sunucu (Tutaa) hazır olunca uçtan uca deneme.
