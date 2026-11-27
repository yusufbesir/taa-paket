# TAA Kur ile kurulum (2026-11-09)

## Üç grup
| Grup | Programlar | Nasıl kurulur |
|---|---|---|
| **Çatı** (programları kuran ve bağlayan ekipman) | TAA Kur, TAA Sunucu, TAA VPN, taa-yayin | Her biri ayrı ve bir kez kurulur (aşağıda) |
| **İş yapan programlar** (bağımsız çalışır) | TAA Etiket, Tutaa, TaaRkod, TAA Rehber (Eğitim dahil), TaaDön, TAA Video Yapıcı ve diğerleri | TAA Kur'un program listesinden, tek tıkla/numara ile |
| **Harici bağımlılık** | Tailscale (ve ileride gerekenler) | Program paketiyle birlikte sormadan kurulur, bkz. `HARICI-BAGIMLILIK.md` |

## Şube PC'sinde kurulum (hedef akış)
1. `TAA-Kur-Kurulum.exe` çift tıklanır (D:\Programlar\TAA; D: yoksa kullanıcı klasörü ve `TAA_KOK` ayarlanır).
   Masaüstü/Başlat kısayolu, "Programlar ve Özellikler" kaldırma girişi ve `kaldir.exe` kurulur. Var olan `kur.toml` ve `anahtarlar\` ezilmez.
2. Kurucu yayındaki programları listeler; numaraları yazarsınız (örn. `1,3`; hepsi için `H`; atlamak için Enter).
   Seçilenler `taa-guncelle kur <ad>` ile indirilir, SHA256 + ed25519 imza doğrulanır, bağımlılıkları (ör. Tailscale) dahil kurulur.
3. Sonrasında TAA Kur penceresi: kurulu listesi, Güncelle, Kaldır, Rapor.
Sessiz kurulum: `TAA-Kur-Kurulum.exe --sessiz --programlar tutaa,taarkod` (soru sormaz). Program seçimi istenmezse `--programsiz`.

## Komutlar (bu PR)
- `taa-guncelle liste [--duz|--json]` yayındaki programlar ve bu PC'deki sürüm.
- `taa-guncelle kur <ad>` yayından ilk kurulum (gerektirir listesindeki paketler önce).
- `tools/kur_paketle.sh --anahtar taa.pub [--kaynak-url URL]` → `dist/TAA-Kur-Kurulum.exe` (bulutta çapraz derlenir; doğrulandı).

## Çatı ekipmanı
- **TAA Kur:** yukarıdaki tek exe.
- **TAA Sunucu:** yalnız sunucu PC; `D:\Programlar\TAA Sunucu`, SYSTEM görevleri; kurulu ve çalışıyor, TAA Kur ile KURULMAZ.
- **TAA VPN:** WireGuard (sunucu PC'de kurulu) ve şube PC'lerinde Tailscale; Tailscale paket kurulurken harici bağımlılık olarak gelir. Sunucu PC'deki kurulum ayrı iş.
- **taa-yayin:** güncelleme kaynağı (GitHub Releases: imzalı zip'ler + `yayin.json`). Şu an boş.

## Açık işler (dürüst durum)
1. **Güvenilir imza anahtarı:** üretim `.pub` dosyası henüz yok (yalnız deneme anahtarı). `kur_paketle.sh` anahtarsız exe üretmez. Gerçek exe, anahtar verilince bulutta üretilebilir.
2. **Yayın:** iş yapan programların imzalı zip paketleri ve `yayin.json` taa-yayin'e henüz konmadı; ondan önce program listesi boş gelir (kurucu "liste alınamadı" der, kurulum yine tamamlanır).
3. **Electron programlar** (Tutaa, TaaRkod, TAA Etiket, TaaDön) bugün kendi Setup.exe'leriyle kuruluyor; TAA Kur yalnız klasör açar, Setup çalıştırmaz. Önerim: paketin içine program kurucusunu koyup taa-kur'un onu sessiz çalıştırması (hazır `harici` hattıyla: sabit SHA256, paket imzası; NSIS `/S`). Bu henüz yapılmadı.
4. Windows'ta elle denenmedi (kurucu exe, kısayol, kaldırma, `taa-guncelle kur`); bulutta Linux'ta ve çapraz derleme olarak doğrulandı.

## Yedekli güncelleme kaynağı (2026-11-09)

`kur.toml` içinde `kaynak_url` (ana), `kaynak_url_yedek = ["https://...", ...]` ve `kaynak_klasoru` (son çare) sırayla denenir;
biri düşerse `taa-guncelle` sessizce sıradakine geçer, çalışan kaynağı hatırlar. Paket yine SHA256 + ed25519 ile doğrulanır.
Yarım indirme silinmez, `<hedef>.kismiN` olarak yana taşınır. `taa-cati` ve `taa-cihaz` da yol listesi (`sunucu-yollari.json`)
varsa isteklerini `YedekliGonderici` ile gönderir (`--sunucu`/`TAA_SUNUCU` verilmişse tek adres).

## Seçimli kurulum (2026-11-11)
- **Rol:** kurucu "Şube PC mi, Sunucu PC mi?" diye sorar (`--rol sube|sunucu`; seçim `<kök>\rol.txt`'ye yazılır, var olan ezilmez).
- **Zorunlu paketler:** `yayin.json`'daki paketin `zorunlu_roller` alanı (örn. `["sunucu"]`) doluysa paket yalnız o rollerde, seçim sorulmadan kurulur;
  başka rolde listede görünmez. `gerektirir` zinciri (Tailscale, PostgreSQL vb.) yine otomatiktir.
- **Seçimli paketler:** gruplu liste (`grup`, `aciklama`); numara yazılır, seçilmeyen kurulmaz. `--programlar a,b` sessiz seçim, `--programsiz` seçimsiz (zorunlular yine kurulur).
- **Kurulum klasörü:** sorulur (Enter = varsayılan), ya da `--hedef`/`--kok`. Faz 1: tek kök klasör; program başına yol sonraki iş.
- `taa-guncelle liste --duz` artık 6 sütun verir: ad, yayındaki, kurulu|-, grup|-, zorunlu roller|-, açıklama|-.
- Plan: `docs/SIFRELEME-PLANI.md`; Win32 seçim penceresi (Adım D) ve Electron sessiz kurulum (Adım E) henüz yok.

## Kurulum ekranı (2026-11-15; yerel pencere)
Yusuf: "kurucu brave üzerinde çalışmasın, kendi ekranı olsun. Herhangi bir tarayıcıda çalışmasını istemiyorum."
Etkileşimli kurulum Windows'ta kurucunun **kendi yerel penceresinde** yürür (Win32 + GDI, `windows-sys`; tarayıcı ve yerel
HTTP YOK): onay (Kur / Vazgeç), rol (Şube PC / Sunucu PC), şube (gömülü 15 şube; `<kök>\sube.txt`), isteğe bağlı programlar,
program başına İndir / Doğrula / Kur / Ayarla, yüzde, hata ve **Yeniden dene**; beklerken program kartları (not + örnek şablon);
sağda kurulumdan sonra yapılacak ayarların numaralı adımları (yapılabilenler kendiliğinden yapılır). `--arayuzsuz` ve `--sessiz`
iken pencere yok (konsol; sessizde soru yok, `--rol`, `--sube`, `--programlar` ile). Ayrıntı: `KURULUM-EKRANI.md`.
`taa-guncelle` yalnız `TAA_ILERLEME=1` ortamında stderr'e `TAA-ILERLEME` satırları yazar (başka davranış değişmez).
Gerçek Windows'ta çalıştırılmadı; Linux'ta wine 9 + Xvfb altında sahte `taa-guncelle` ile uçtan uca denendi.

## Şube bilgisayarı profili (2026-11-27)
Yusuf: "Şube bilgisayarı kendi şubesinin sunucusu gibi çalışır; merkezin yerini almaz. Kurulum, taa-yazdir ile şube düğümünü şube
bilgisayarının kendisine kurar ve başlatır. Çatı şubelere kurulmaz." Plan: `PLAN-TAA-WEB-2026-11-27.md` §9; düğüm: `SUBE-DUGUMU.md`.

**Ne yapar** (`taa-kur sube-pc kur`, kurulum exe'sinde "Şube bilgisayarı" adımı; yalnız `rol=sube`, sunucu rolünde reddeder):
- `taa-yazdir` ve `taa-sube-dugum` ikililerinin kökte olduğunu denetler (yoksa adım "kurulamadı" der; hiçbir şeyi silmez).
- Klasörler: `<veri>\gunluk\` (düğümün `YYYY-AA-GG.jsonl` günlüğü; silinmez), `<veri>\sube-pc\` (profilin kendi dosyaları).
- Yol listesi: `<veri>\sube-pc\istemci-yollari.json` = `[{ad:'merkez',url},{ad:'sube',url},{ad:'yerel'}]` (sıra sabit: merkez →
  şube bilgisayarı → cihazın kendi kutusu). Merkez adresi `--merkez` ya da `eslestirme.json`'dan; yoksa merkez satırı url'siz yazılır.
- `<veri>\yazdir\ayar.json` içindeki `izinli_kokenler`'e merkez ve şube kökenleri eklenir (eski sürüm `eski\` altına taşınır).
- Açılış kaydı: `schtasks` görevi **"TAA Sube PC"** (ONLOGON, `/RL LIMITED`); olmazsa `HKCU\...\Run` anahtarı. Komut:
  `"<kök>\taa-kur-pencere.exe" --sube-pc-baslat` (penceresiz). `--gorevsiz` ile atlanır.
- Başlatma: `taa-yazdir --kok <veri> calis` (8797) ve `taa-sube-dugum calis --kok <veri> --sube <kod> --port 8788 ...`; kapı zaten
  açıksa ikinci kez başlatmaz. Çıktılar `<veri>\sube-pc\cikti\<ad>.log`; her kurulum/başlatma `<veri>\sube-pc\kurulum.jsonl`'a eklenir.
- Profil özeti: `<kök>\sube-pc.json`.

**Komutlar:** `taa-kur sube-pc kur [--merkez URL] [--sube-adres ADRES] [--port N] [--lan-ac] [--gorevsiz] [--baslatma] [--duvarsiz]`,
`taa-kur sube-pc durum`, `taa-kur sube-pc baslat`, `taa-kur sube-pc gorev-kaldir` (yalnız açılış kaydını kaldırır; veri ve günlük kalır).
Kurulum exe'si: `--sube-pc evet|hayir` (ya da paket yanında `sube-pc.txt`; sessizde varsayılan evet), `--merkez`, `--sube-adres`,
`--dugum-port`, `--sube-lan evet`. Kaldırma önce açılış kaydını siler.

**Çatı:** şube rolünde program listesinden ve `--programlar` seçiminden düşürülür (kurulmaz; kurulu olan silinmez).

**Yerel ağ varsayılan olarak KAPALI.** Düğümde henüz kimlik doğrulama yok (`SUBE-DUGUMU.md` §13.6.1), bu yüzden profil düğümü yalnız
`127.0.0.1`'de açar ve şube adresini `http://127.0.0.1:8788` yazar. Telefonların şube bilgisayarına yazabilmesi için `--lan-ac`
(kurulumda `--sube-lan evet`) gerekir: o zaman özel ağ IPv4'ü `--lan` ile verilir, yol listesine o adres yazılır ve Windows güvenlik
duvarına "TAA Sube Dugumu" kuralı (özel profil, yerel alt ağ) eklenir. Düğüme kimlik doğrulama gelince varsayılan değiştirilecek.

**Dürüst durum / açık noktalar**
- Gerçek Windows'ta **denenmedi** (schtasks, Run anahtarı, netsh, penceresiz başlatma). Linux'ta sahte `Sistem` ile birim sınamaları,
  gerçek ikililerle uçtan uca kurulum ve `x86_64-pc-windows-gnu` tür denetimi yapıldı.
- `tools/kur_paketle.sh` bu işin kapsamı dışında: `taa-yazdir` ve `taa-sube-dugum` pakete ya da güncelleme kanalına eklenmeden adım
  "kurulamadı" der.
- Düğümün merkeze kopyalaması için merkez oturumu (`TAA_OTURUM` / `--oturum-dosyasi`) gerekir; kurulum bunu sağlamaz.
- `istemci-yollari.json`'u kabuk ve düğüm henüz okumuyor (yer önerisi); `sube-pc` ve `gunluk` alanları `veri-sahipleri.toml`'da yok.
- Güvenlik duvarı kuralı ve bazı makinelerde `schtasks` yönetici ister; kural eklenemezse uyarı yazılır, kurulum durmaz.
- Çalışan servis exe'leri kaldırmada silinemez (Windows kilidi); önce süreçler kapatılmalı.
