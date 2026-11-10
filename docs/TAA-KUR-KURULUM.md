# TAA Kur ile kurulum (2026-11-09)

## Üç grup
| Grup | Programlar | Nasıl kurulur |
|---|---|---|
| **Çatı** (programları kuran ve bağlayan ekipman) | TAA Kur, TAA Sunucu, TAA VPN, taa-yayin | Her biri ayrı ve bir kez kurulur (aşağıda) |
| **İş yapan programlar** (bağımsız çalışır) | TAA Etiket, Tutaa, TaaRkod, TAA Eğitim, TaaDön, TAA Video Yapıcı ve diğerleri | TAA Kur'un program listesinden, tek tıkla/numara ile |
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
