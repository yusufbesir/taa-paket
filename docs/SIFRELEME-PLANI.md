# TAA şifreleme planı (Yusuf onayı 2026-11-11)

Karar: tam uçtan uca şifreleme YOK (sunucu rapor/yetki için veriyi okumalı). Taşıma şifrelemesi var (HTTPS/WireGuard/Tailscale).

## Adım A — Yedek şifreleme (taa-yedek)
- Dosya başına XChaCha20-Poly1305 (RustCrypto `chacha20poly1305`, saf Rust, yeni bağımlılık, Yusuf'un genel iznine dayanır).
- Anahtar: 32 bayt rastgele, yalnız `--anahtar <dosya>` ile verilir (Yusuf'un D:\YusufII\... klasöründe), depoya/yedeğe KONMAZ.
- Manifest şifreli dosyanın SHA256'sını ve "sifreli": true bilgisini tutar; açık metin SHA256'sı da manifestte (anahtarla doğrulama).
- `yedekle --sifrele`, `geri-yukle` anahtarla çözer; yanlış anahtar/bozuk dosya = hata (AEAD). Hiçbir şey silinmez/ezilmez.
- Test: gidiş-dönüş, yanlış anahtar, bozuk bayt, boş dosya, büyük dosya.

## Adım B — Şubeden gelen belge/fotoğraf şifreleme (cihaz anahtarı)
- Şube PC dosyayı sunucuya göndermeden önce zarf şifreler (X25519 + XChaCha20-Poly1305; alıcı: sunucu genel anahtarı).
- Sunucu şifreli saklar; açma yalnız yetkili istekte. Sunucu tarafı tutaa deposu işi (ayrı başlık).
- Bu adım sunucu sözleşmesi gerektirir; A bittikten sonra tasarım belgesi yazılır.

## Adım C — Sunucu PC diski: BitLocker (kod yok, Yusuf yapar)
- Denetim Masası > BitLocker; kurtarma anahtarı USB'ye ya da Microsoft hesabına DEĞİL, çıktı alınıp saklanır.
- Sürücüler: sunucu veri (PostgreSQL pgdata, yedekler). Adım adım anlatım A bitince verilir.
