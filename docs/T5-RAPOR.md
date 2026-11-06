<!-- Sahip: Yusuf. yazan: [Claude#taa-rapor-T5 2026-11-06] -->
# T5: Paket raporu (`taa-rapor`)

Kaynak: Yusuf 2026-11-06 00:41 "Tek bir yerden tüm veriler sorgulanabiliyor mu? Rapor çıkartıyor mu?" → cevap YOK'tu; bu crate ilk adımdır.
Durum raporu: `/mnt/project-files/taa-paket/DURUM-2026-11-06.md`.

## Ne yapar
`taa-rapor`, tüm veri alanlarını **tek yerden, salt okunur** okur ve tek özet üretir:

| Bölüm | Kaynak (varsayılan: `<veri kökü>/<alan>/…`) | Sözleşme | Satırlar |
|---|---|---|---|
| fis | `fis-defteri/fisler.jsonl` | taa-fis defteri (`tur: fis/durum/teslim/raf/imza`) | toplam, açık, durum sayıları, geciken, bugün teslim, kargo bekleyen, açık kalan ücret, usta başına açık |
| kasa | `kasa-defteri/kasa.jsonl` | taa-kasa (`kapanis/iptal`) | kapanış, iptal, son kapanış tarihi/sayılan/fark, farklı kapanış, toplam fark |
| bildirim | `bildirim-kuyrugu/bildirimler.jsonl` | taa-bildirim (`mesaj/sonuc`) | toplam, bekliyor, gönderildi, hata |
| katalog | `urunler/katalog.json` | taa-katalog | kaynak dosya, ürün, indirimli, fiyatsız |
| pano | `pano-kartlari/*.pano.json` | taa-pano | kart sayısı + her kart |
| yedek | `yedekler/<damga>/manifest.json` | taa-yedek | yedek sayısı, son yedek, dosya sayısı, boyut |

Kurallar:
- Başka programın crate'ine bağımlı değildir (ince çizgi); dosya sözleşmesini `serde_json::Value` ile okur. `modul.toml`: `program = "ortak"`, `veri_okur` altı alan.
- Olmayan kaynak hata değildir: bölüm **yok**. Okunamayan kaynak: bölüm **hata**, rapor yine çıkar. Bozuk JSONL satırı atlanır, uyarı yazılır.
- Hiçbir alan dosyasına yazmaz; yalnız `--csv` ile verilen dosyayı yazar (`taa_dosya::atomik_yaz`, var olanı ezmez).
- Veri kökü: `--kok` > `TAA_VERI_KOK` > `%APPDATA%\TAA\veri` (Windows) / `$XDG_DATA_HOME|~/.local/share` + `/TAA/veri`.
- Bugünkü gerçek (ENVANTER §2): fiş ve bildirim dosyaları taa-fis-sunucu `--veri` klasöründedir → `--fis-klasoru <klasör>`. Kasa defteri dosya adı taa-kasa'da tanımsız; rapor `kasa-defteri/kasa.jsonl` varsayar, `--kasa <dosya>` ile değiştirilir.

## Komut
```
taa-rapor [--kok <veri kökü>] [--fis-klasoru <klasör>] [--bugun YYYY-AA-GG] [--json] [--csv <dosya>]
          [--fis|--kasa|--bildirim|--katalog <dosya>] [--pano|--yedek <klasör>]
```
Çıkış: 0 tamam, 2 kullanım/doğrulama, 1 diğer. CSV: UTF-8 BOM, `;` ayraç, `bolum;durum;anahtar;deger` (Excel çift tıkla açar).

## Sonraki adımlar
1. `taa-kur rapor` komutu ve pencerede "Rapor" düğmesi (ayrı PR).
2. Rapor JSON'u AI yorumu için girdi olur (Yusuf kararı: API anahtarı, kim görür).
3. Tutaa tutanakları ve etiket kuyruğu bölümleri (o programların dosya sözleşmesi netleşince).
