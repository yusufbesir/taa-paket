# S4 — Ağ: sunucuya ulaşım yolları · Sahip: Yusuf · yazan: [Claude#taa-yol-S4a 2026-11-07]

Yusuf (2026-11-07 03:15): **hepsi olsun** — alan adsız, alan adlı, Cloudflare'li ve VPN'li yollar birlikte kullanılabilsin;
alt alan adı yedek. Bilinenler (23:15): dış IP **statik**, modem yönetimi **var**, terziademaltun.com DNS paneli var ama site
Ticimax'ta; Fortinet kullanılmayacak. Kural: hangi yol olursa olsun **cihaz imzası + parola + yetki belgesi** aynı; yol
yalnız "nereden gidileceği"dir. Sunucu tüm yollardan aynı anda dinler.

## İstemci tarafı (bu PR, S4a) — hazır
- **taa-yol** (ortak crate, veri alanı `yol`): `<kök>\yol\sunucu-yollari.json`
  `{"sunucu":"merkez","yollar":[{"ad","taban","sertifika"?}]}`; sıra = öncelik. `son.json` son çalışan yol (önce o denenir).
  Deneme: `GET /v1/saglik`; sunucudan **herhangi bir HTTP yanıtı** ulaşıldı sayılır (401/403 dahil), yalnız ağ hatası
  "ulaşılamadı" (bağlantı süresi 5 sn). Komutlar: `taa-yol liste|ekle|kaldir|one-al|dene`.
- `sertifika`: verilmezse `<kök>\anahtarlar\sunucu-ca.pem` (kendi CA'mız; IP ve VPN yolları); `"sistem"` = işletim sistemi
  kökleri (Let's Encrypt, Cloudflare); başka = dosya adı (`anahtarlar\` altında) ya da tam yol.
- **Oturum anahtarı = `sunucu` mantıksal adı** (`merkez`): yol değişse de taa-kimlik oturumu ve yetki belgesi aynı kalır.
  `--sunucu` / `TAA_SUNUCU` verilirse eski davranış (tek adres, anahtar = adres) sürer.
- Bağlanan programlar: taa-kimlik (`giris`, `gonder` yol seçer; `durum`, `yetki`, `cikis` ağa çıkmaz), taa-cihaz
  (`kayit`, `durum`), taa-cati (tüm komutlar). Seçilen yol stderr'e `yol: <ad> → <adres>` olarak yazılır.
- **taa-kur yol** `--dosya sunucu-yollari.json [--ca sunucu-ca.pem]`: listeyi doğrulayıp veri köküne kurar, CA'yı
  `anahtarlar\sunucu-ca.pem` olarak kopyalar (PEM denetimi). Kurulum paketine bu iki dosya eklenir.
- taa-kuyruk: `CurlGonderici::baglanti_suresi` (`--connect-timeout`).

## Sunucu PC'si (S4b, PC işi — koordinatör slotu) — yapılacak
Üçüncü taraf programlar `D:\Programlar\Araçlar Ve İndirilenler\` altına (klasör kuralı). Yalnız 443/TCP ve 51820/UDP dışarı
açılır; 8443 (tutaa-sunucu) ve 5432 (PostgreSQL) yalnız yerel.

### Yol 1 — Doğrudan statik IP + kendi sertifikamız (Caddy iç CA)
Caddy hem ters vekil hem **iç sertifika makamı**: IP için kendisi sertifika üretir, kök sertifikayı şubelere dağıtırız.
```
# D:\Programlar\Araçlar Ve İndirilenler\caddy\Caddyfile
https://<STATİK_IP>, https://10.8.0.1 {
    tls internal
    reverse_proxy 127.0.0.1:8443
}
```
Kök sertifika: `%APPDATA%\Caddy\pki\authorities\local\root.crt` → `sunucu-ca.pem` adıyla kurulum paketine. Modem: 443 → sunucu PC.
Yol kaydı: `{"ad":"dogrudan","taban":"https://<STATİK_IP>"}`.

### Yol 2 — VPN (WireGuard, kendi sunucumuz)
WireGuard Windows uygulaması sunucu PC'de; her şube PC'si için bir eş (peer) anahtarı (Çatı sonraki sürümde üretir).
Sunucu VPN adresi `10.8.0.1/24`; modem 51820/UDP → sunucu PC. Caddy aynı Caddyfile ile 10.8.0.1'i de dinler (yukarıda).
Yol kaydı: `{"ad":"vpn","taban":"https://10.8.0.1"}`. VPN güvenliğin yerine geçmez; cihaz imzası ve parola yine zorunlu.

### Yol 3 — Alan adı + Let's Encrypt (alan adı seçilince)
DNS'e `sunucu.<alan>  A  <STATİK_IP>` (Ticimax'ı etkilemez; alt alan adı yedek plan). Caddyfile'a blok eklenir:
```
sunucu.<alan> {
    reverse_proxy 127.0.0.1:8443
}
```
Caddy sertifikayı Let's Encrypt'ten alır ve yeniler (80/TCP de açılmalı ya da DNS doğrulaması). Yol kaydı:
`{"ad":"alan-adi","taban":"https://sunucu.<alan>","sertifika":"sistem"}`. Hiçbir program değişmez; yalnız liste güncellenir.

### Yol 4 — Cloudflare Tunnel (alan adı Cloudflare'e taşınırsa)
`cloudflared` sunucu PC'de, dışarı port açmadan Cloudflare'e bağlanır; `taa.<alan>` → `http://127.0.0.1:8443`
(ya da Caddy). Yol kaydı: `{"ad":"cloudflare","taban":"https://taa.<alan>","sertifika":"sistem"}`. **Not:** TLS Cloudflare'de
açılır (veri orada görünür); müşteri verisi için 1–3 öne, 4 listenin sonuna konur. Alan adı Cloudflare DNS'inde olmalı.

### Dağıtım
Kurulum paketine `sunucu-yollari.json` + `sunucu-ca.pem` konur; `taa-kur yol --dosya … --ca …`. Statik IP değişirse yalnız
liste güncellenir (taa-guncelle ile dağıtım, sonraki). Sunucuya `GET /v1/saglik` (cihaz denetimi dışı, yalnız `{"ok":true}`)
eklenmesi sunucu başlığından istendi; olmasa da seçim çalışır (401 de "ulaşıldı" sayılır).

## Yapılmadı / sonraki
- Sunucu PC'de Caddy/WireGuard/cloudflared kurulumu (PC slotu, Yusuf'la birlikte: modem yönlendirme).
- Çatı'dan WireGuard eş anahtarı üretimi ve şube yapılandırma dosyası (S4c).
- Windows'ta elle deneme yapılmadı; testler Linux + CI Windows. **S4b PC bulgusu (2026-11-07):** Windows curl (schannel) kendi CA ile imzalı sertifikada CRL bulamayıp `CERT_TRUST_REVOCATION_STATUS_UNKNOWN` veriyordu; `CurlGonderici` artık `--cacert` verildiğinde Windows'ta `--ssl-no-revoke` ekler (yalnız kendi CA'mız; sistem kökleri `"sistem"` seçiminde eklenmez).
