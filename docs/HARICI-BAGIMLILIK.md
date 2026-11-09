# Harici bağımlılık kurucusu (taa-kur)

Yusuf 2026-11-09: paket programları kurulurken gereken üçüncü taraf programlar (ilk örnek Tailscale) sorulmadan
indirilsin, doğrulansın, sessiz kurulsun ve ayarlansın. Tüm paketler için geçerlidir.

## Tanım
Paketin içinde `harici.toml` (paket.json `dosyalar` listesinde; imzalı pakette tanım da imzalıdır). Örnek: `docs/harici/tailscale.toml.ornek`.
Alanlar: `ad`, `surum`, `url` (yalnız https, sürüme sabit), `sha256` (sabit), `imzalayan` (Authenticode, zorunlu),
`sessiz` (kurucu argümanları), `kurulu_yollar` (kurulu sayılacak dosyalar; `%ProgramFiles%` açılır), `ayar` (kurulum sonrası komut).
Özeti dosyadan üretmek için: `python tools/harici_sha.py <kurucu.exe>`.

## Akış (`taa-kur kur paket.zip`, pencerede Kur, güncellemeden sonra)
1. Paket kurulur; pakette `harici.toml` varsa her bağımlılık için: zaten kuruluysa atlanır.
2. İndirme (curl, https, 512 MiB sınırı) → SHA256 eşleşmesi → Authenticode imzası (PowerShell `Get-AuthenticodeSignature`, imzalayan adı) → sessiz kurulum (0/3010/1641 başarı).
3. Kurucu her durumda `eski\` altına taşınır (silme yok).
4. Ayar: `ayar.args` içindeki `{anahtar_dosya}` → `<kök>\gizli\<ad>.anahtar`. Anahtar sırası: kökteki dosya, yoksa `anahtar_paket_dosyasi` (paket içindeki dosya kopyalanır, `tskey-` biçimi ve 1 KiB sınırı denetlenir); hiçbiri yoksa ayar atlanır ve uyarı verilir.

Sıra bozulursa (özet/imza uyuşmaz) kurucu ÇALIŞTIRILMAZ. `--harici-atla` bu adımı kapatır; `taa-kur harici <harici.toml|paket>` yeniden dener.

## Kullanıcıya iş bırakmayan akış (Yusuf 2026-11-09)
Hedef: PC'ye TAA paketi kurulunca Tailscale kurulumu ve ağa katılım kimse bir şey yapmadan biter.
1. Tailscale konsolunda tek anahtar üretin: önceden yetkili, **yeniden kullanılabilir**, etiketli (`tag:taa`), 90 gün süreli. ACL'de `tag:taa` yalnız sunucuya erişsin (sızarsa etkisi sınırlı kalır).
2. Anahtarı paketi üreten PC'de paketin `gizli/tailscale.anahtar` dosyasına koyun ve `harici.toml` içinde `anahtar_paket_dosyasi` verin (örnek dosyada hazır).
3. Paketi **özel** dağıtın (USB/ağ paylaşımı). Anahtar içeren paket herkese açık GitHub yayınına (taa-yayin) KONMAZ.
4. Her PC'de `taa-kur kur paket.zip` yeterli; anahtar kopyalanır, Tailscale kurulur, `tailscale up --unattended` çalışır.
Anahtar süresi dolunca yeni anahtarlı paket üretilir. Daha temiz gelecek yol: anahtarı sunucudan (onaylı cihaza, tek kullanımlık) vermek; bu Tutaa sunucu işidir, henüz yok.

## Tailscale otomatik katılım
Hesap girişi gerekir. Önerilen yol: Tailscale yönetim konsolunda **önceden yetkili, etiketli, süreli** bir auth key üretin,
her PC'de `<kök>\gizli\tailscale.anahtar` dosyasına yazın (ayar: `up --unattended --auth-key=file:{anahtar_dosya}`; `file:` biçimi yeni Tailscale sürümlerini ister).
Anahtar koda, tanıma ya da günlüğe yazılmaz; komut satırında yalnız dosya yolu görünür. Sunucu PC'deki Tailscale ayrı işte yapılıyor, bu iş ona dokunmaz.

## Sınırlar
- Windows'ta elle denenmedi (imza denetimi, sessiz kurulum, yönetici yetkisi): testler sahte sistemle yazıldı.
- Kurucu yönetici yetkisi isterse taa-kur'un yönetici çalışması gerekir.
- Tailscale 1.104.5 değerleri (adres, SHA256) `docs/harici/tailscale.toml.ornek` içinde; adres kalıbı (`tailscale-setup-<sürüm>.exe`) indirilerek doğrulanmadı, SHA256 sabit olduğundan yanlış dosya kurulmaz.
