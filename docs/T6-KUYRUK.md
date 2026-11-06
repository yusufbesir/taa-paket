<!-- Sahip: Yusuf. yazan: [Claude#taa-kuyruk-T6 2026-11-06] -->
# T6: Giden kuyruk (`taa-kuyruk`) — merkez sunucuya çevrimdışı gönderim

Kaynak: Yusuf 2026-11-06 10:15 "Merkez sunucu: PostgreSQL" kararı ve 10:16 "ücretsiz olmalı".
Plan: `/mnt/project-files/planlar/MERKEZ-SUNUCU-UCRETSIZ.md`. Sunucu sözleşmesi: Tutaa `API-SOZLESMESI.md`
("Çevrimdışı kuyruk: istemci `no` ile idempotent gönderir"). Sunucu kodu Tutaa deposunun işidir; bu crate istemci tarafıdır.

## Ne yapar
Programlar sunucuya gidecek kaydı önce yerel kuyruğa yazar; sunucu yoksa ya da ağ kesikse kayıt birikir.
Bağlantı gelince `gonder` bekleyenleri sırayla `POST <sunucu><uç>` ile yollar. Her kaydın `no` değeri tektir;
sunucu aynı `no` ile gelen kaydı ikinci kez yazmaz, bu yüzden yarım kalan gönderim tekrar edildiğinde çift kayıt oluşmaz.

Klasör: `<veri kökü>/kuyruk/<program>/` (her program kendi klasörüne yazar; tek yazar kuralı korunur, gönderim tek yerden).

| Dosya | Satır | Anlamı |
|---|---|---|
| `giden.jsonl` | `{"no","uc","zaman","govde"}` | eklenen her kayıt; hiç değişmez |
| `gonderildi.jsonl` | `{"no","zaman","yanit"}` | sunucunun kabul ettikleri (2xx) |
| `reddedildi.jsonl` | `{"no","zaman","kod","mesaj"}` | sunucunun kalıcı reddettikleri (4xx; 408/429 hariç); yeniden denenmez, kişi bakar |

Kurallar:
- Yalnız ekleme; hiçbir satır/dosya silinmez ya da ezilmez (`taa_dosya::satir_ekle`).
- Aynı `no` ikinci kez eklenmez (`Ekleme::ZatenVar`). `no`: `[A-Za-z0-9._-]`, en çok 64. Uç: `/v1/...` biçiminde.
- Sunucuya ulaşılamıyorsa (ağ yok, zaman aşımı, 5xx, 408/429) gönderim o kayıtta durur; sıra bozulmaz, kalan sonraki çağrıya kalır.
- Bozuk JSONL satırı hata verir (dosya adı + satır), satır silinmez.
- HTTP için yeni crate yok: `CurlGonderici` `curl` çalıştırır (Windows 10 1803+ ile gelir). `--max-time 60`, yönlendirme yok, `--proto =http,https`.
- **Oturum anahtarı** komut satırına yazılmaz: geçici başlık dosyasına yazılır (`--header @dosya`), iş bitince kaldırılır; kalıcı hiçbir yere kaydedilmez. CLI'de `--oturum-dosyasi <yol>` ya da `TAA_OTURUM` ortam değişkeni.
- Kendi TLS sertifikamız: `--sertifika <pem>` (`curl --cacert`).

## Komut
```
taa-kuyruk [--kok K] ekle --program <ad> <uç> <no> <json|@dosya>
taa-kuyruk [--kok K] liste [--program <ad>] [--json]
taa-kuyruk [--kok K] gonder --sunucu <https://adres> [--program <ad>] [--oturum-dosyasi <yol>] [--sertifika <pem>] [--en-cok N] [--json]
```
Çıkış: 0 tamam (kalan olsa da), 2 kullanım/doğrulama, 3 sunucuya ulaşılamadı, 1 diğer.

## Kütüphane
```rust
let k = Kuyruk::program(&veri_kok, "taa-fis")?;
k.ekle("/v1/olay", "S1-2026-000123", json!({"tur": "kod_okundu"}))?;   // Eklendi | ZatenVar
let g = CurlGonderici::yeni("https://192.168.1.10:8443")?.oturum(&anahtar)?;
let ozet = k.gonder(&g, 0)?;   // gonderilen, reddedilen, kalan, durma_nedeni
```
`Gonderici` trait'i ile sahte gönderici yazılabilir (testler böyle çalışır).

## Sonraki adımlar
- taa-kimlik: `POST /v1/giris` ile oturum alma, anahtarı Windows Kimlik Bilgisi Yöneticisi'nde tutma (depoya/dosyaya yazılmaz).
- taa-kur: `gonder`i zamanlayıcıyla (ör. 5 dakikada bir) çalıştırma; pencerede "bekleyen N" göstergesi.
- taa-rapor: kuyruk bölümü (bekleyen/gönderilen/reddedilen).
- Sunucu (Tutaa): uçlar hazır olunca fiş/kasa/bildirim programları `ekle` çağırır.
