<!-- Sahip: Yusuf. yazan: [Claude#taa-raporlar-T8 2026-11-06] -->
# T8: TAA Raporlar (`taa-raporlar`) — yönetici raporu ve kural tabanlı yorum motoru

Kaynak: Yusuf 2026-11-06 15:35: "raporlar zaten sadece yöneticiler tarafından çekilebilir olmalı … yönetici hesabı ile giriş
yapılabilen bir taa raporlar programı … rapor yorumlayan bir motor geliştirmek uygun olur mu?" Karar: API anahtarı şimdilik yok.
Alternatifler: `/mnt/project-files/planlar/MERKEZ-SUNUCU-KARARLAR-2026-11-06.md` (A kural motoru + B Claude aboneliği önerildi).

## Ne yapar
| Parça | İş |
|---|---|
| Rapor | taa-rapor ile tüm alanları tek yerden okur (fiş, kasa, bildirim, katalog, pano, yedek). |
| **Yorum motoru** `yorumla()` | Eşik kurallarıyla Türkçe madde madde yorum: `uyarı` / `dikkat` / `bilgi`. İnternet ve yapay zekâ gerekmez. |
| Karşılaştırma `karsilastir()` | `<kök>/raporlar/rapor-*.json` içindeki en yeni önceki raporla fark ("geciken fiş 2 → 4 (arttı)"). |
| Yönetici denetimi | `--sunucu` verilince taa-kimlik deposunda **`yonetici`** rollü oturum şart; yoksa çıkış 5/6. Sunucu yokken yerel kip. |
| Kayıt | `--kaydet`: `<kök>/raporlar/rapor-<damga>.json` (rapor + yorum) ve `.txt`; var olan ezilmez. |

## Kurallar ve varsayılan eşikler (`<kök>/raporlar/esikler.json` ile değişir)
| Kural | Eşik (varsayılan) | Seviye |
|---|---|---|
| Teslim tarihi geçen fiş | 1+ dikkat; `geciken_uyari` = 5+ uyarı | dikkat / uyarı |
| Bugün teslim | 1+ | bilgi |
| Kargo bekleyen | `kargo_dikkat` = 3+ | dikkat |
| Açık fişlerde tahsil edilmemiş | `acik_ucret_dikkat_kurus` = 5.000 TL+ | dikkat |
| Ustada açık fiş (kapasite) | `usta_acik_dikkat` = 15+ | dikkat |
| Son kasa farkı | sıfır dışı dikkat; `kasa_fark_uyari_kurus` = 500 TL+ uyarı | dikkat / uyarı |
| Farklı kapanış sayısı | `farkli_kapanis_dikkat` = 3+ | dikkat |
| Bildirim hatası | 1+ | dikkat |
| Bekleyen bildirim | `bildirim_bekleyen_uyari` = 20+ | uyarı |
| Fiyatsız ürün | 1+ | dikkat |
| Yedek yok / eski | yok → uyarı; `yedek_gun` = 2 günden eski → uyarı | uyarı |
| Bölüm okunamadı (hata) | — | uyarı |
| Hiç bulgu yok | — | "Olağan dışı bir durum yok." |

Eşik değerleri **Yusuf/yönetici doğrulamalı**; varsayılanlar başlangıç içindir. Bozuk `esikler.json` sessizce varsayılana düşmez, hata verir.

## Komut
```
taa-raporlar [--kok K] [--fis-klasoru F] [--bugun YYYY-AA-GG] [--sunucu <https://adres>] [--json] [--kaydet] [--esikler]
```
Çıkış: 0 tamam, 2 kullanım, 5 oturum yok, 6 yönetici değil, 1 diğer.

## Sonraki adımlar
- taa-kur: bu programı yalnız `yonetici` rolüne kur; pencereye "Raporlar" düğmesi.
- Sunucu hazır olunca: tüm şubelerin verisi sunucudan (`GET /v1/...`), şube bazlı karşılaştırma kuralları.
- B seçeneği: `--kaydet` çıktısını proje dosyalarına bırakıp Claude ile yorumlatma (rutin).
- Excel: mevcut CSV (taa-rapor) + yorum sütunu.
