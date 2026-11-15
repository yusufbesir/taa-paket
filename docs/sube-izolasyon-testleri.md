<!-- Sahip: Yusuf. yazan: [Claude#sozlesme-S0 2026-11-15]. Durum: SÖZLEŞME (Ş0). Değişiklik: Yusuf'un onayıyla. -->
# Şube izolasyon testleri: "A şubesi B'yi göremez" (Ş0)

Kaynak: `planlar/PLAN-SUBE-ORTAK-SUNUCU-2026-11-15.md` §2.5 ("Kanıt"), §3.5. **Bu matris geçmeden hiçbir şubeli uç birleşmez** (kapı ölçütü). Bağlı: `docs/sube-sozlesmesi.md`, `docs/sube-tehdit-modeli.md` §6, `docs/rehber-sozlesmesi.md` §10.

## 1. Düzenek
- Gerçek PostgreSQL 16 (sunucu entegrasyon katmanı); `ci_yerel` ile koşar, çıktı PR'a kanıt olarak eklenir.
- Test verisi uydurmadır (gerçek müşteri, çalışan ya da şube personeli verisi yok). Şubeler: **A**, **B** (iki test şubesi), **M** merkez cihazı (`sube_id = null`).
- Her şube için: onaylı cihaz + şube personeli + şube müdürü; ayrıca A şubesinin iptal edilmiş cihazı. Merkezde yönetici (TOTP) ve merkez personeli.
- Her tabloya B'ye ait en az bir kayıt, A'ya ait en az bir kayıt tohumlanır.
- Test adı biçimi: `izolasyon_<tablo>_<boyut>` (ör. `izolasyon_fisler_okuma`).

## 2. İki kip (iki kilit ayrı ayrı kanıtlanır)
| Kip | Ne | Beklenen |
|---|---|---|
| **K-tam** | Tam sistem (uygulama süzgeci + RLS) | Matristeki her hücre geçer |
| **K-rls** | Uygulama şube süzgeci yalnız test derlemesinde bayrakla kapatılır; yalnız RLS | Okuma, liste, sayım, yazma hücreleri yine geçer |
| **K-uyg** | RLS kapalı test veritabanı; yalnız uygulama süzgeci | Aynı hücreler yine geçer |
Üretim derlemesinde bayrak yoktur; varlığı CI'da denetlenir.

## 3. Boyutlar (sütunlar)
| Boyut | A cihazı/kişisi B kaydı için dener | Beklenen |
|---|---|---|
| **okuma** | `GET` ayrıntı (B'nin kimliğiyle) | 404 `bulunamadi` (403 değil; varlık sızmaz) |
| **yazma** | B kaydına olay/ek/iptal/onay; gövdede ya da `X-TAA-Sube`'de B kodu | 404 (B kaydı) ya da 409 `sube_uyusmaz`; `yetki.red.sube` olayı yazılır; B kaydı değişmez |
| **liste** | Liste ucu (`?sube=B` dahil) | Yalnız A kayıtları; `sube` parametresi yok sayılır |
| **sayım** | Sayım/özet/sayfalama toplamı | Yalnız A sayısı; toplam B'yi içermez |
| **dosya** | B'nin eki ya da şubeye özel havuz dosyası (indir, `Range`) | 404; sha256 ya da boyut sızmaz |
| **günlük** | Çatı "Günlük" / denetim izi okuma (şube rolü) | B olayları görünmez; A'nın günlüğünde B'nin adı, kişi verisi yok |
| **hata** | Geçersiz istekler, sınır aşımı, 409 çakışma | Mesajda B'nin şube adı, kodu, kayıt no'su, kişi verisi, SQL/yol/yığın yok; "yok" ile "başkasının" yanıtı aynı (gövde ve durum kodu) |

## 4. Matris (tablo × boyut)
✓ = test zorunlu, — = boyut tabloya uygulanmaz.
| Tablo / kaynak | okuma | yazma | liste | sayım | dosya | günlük | hata | Not |
|---|---|---|---|---|---|---|---|---|
| `fisler` | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | Merkez listesi maskeli; ayrıntı denetim izli |
| `fis_olaylari` | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | `olay_id` tekrarı B'nin olayıyla çakışmaz |
| `tutanaklar` | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | `taraf_subeler` içindeki şube okur, yazamaz |
| `tutanak_ek` | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Yalnız düzenleyen şube ekler |
| `ciro_gunluk` | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | `GET /v1/ciro?sube=B` A'ya yalnız A'yı verir |
| `cakisma` | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | |
| `sube_imza_anahtarlari` | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | A, B'nin anahtarını onaylayamaz/iptal edemez; B anahtarıyla A adına imza reddedilir |
| `taa_cihazlar` | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | Cihazın şubesi yalnız Çatı'dan değişir |
| `kullanici_rol` | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | Şube müdürü başka şubeye kullanıcı ekleyemez |
| `olaylar` (denetim izi) | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | Yazma = eklemeli; güncelleme/silme her rolde reddedilir |
| Havuz öğeleri (`sube_kapsam = {B}`) | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | ✓ | Ör. `sube.kase@1` |
| `rehber.kisi` / `rehber.gorevlendirme` | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | B'nin personeli A'ya gitmez (B mağaza müdürünün ad/e-posta/görev hariç); boş kapsam = yalnız merkez |
| `rehber` öneri kuyruğu | ✓ | ✓ | ✓ | ✓ | — | ✓ | ✓ | Öneri yalnız `rehber.yaz` sahibine görünür |
| Kuyruk (istemci, A PC'si) | — | ✓ | — | — | — | — | — | B damgalı öğe gönderilmez |
| Havuz/rehber önbelleği (istemci) | ✓ | ✓ | — | — | ✓ | — | — | Önbellekte B verisi yok; okuyan program yazamaz |

## 5. Ek zorunlu senaryolar
1. `izolasyon_beyan_uyusmaz`: A cihazı `X-TAA-Sube: B` gönderir → 409 + `yetki.red.sube`.
2. `izolasyon_kisi_sube_uyusmaz`: A cihazında yalnız B'de rolü olan kişi → 403.
3. `izolasyon_iptal_cihaz`: iptal edilmiş A cihazı → 403 `cihaz_onaysiz`, parola denenmez.
4. `izolasyon_merkez_tum`: merkez yönetici A ve B'yi görür; maskesiz ayrıntı açma denetim izine yazılır.
5. `izolasyon_merkez_yazamaz`: merkez, şube kaydının içeriğini değiştiremez; yalnız onay/iptal/not olayı ekler (iptal yalnız yönetici).
6. `izolasyon_rls_ayar_yok`: `taa.sube_id` atanmamış işlem hiçbir şubeli satır döndürmez.
7. `izolasyon_rls_rol`: `taa_uygulama` rolünün `BYPASSRLS` ve tablo sahipliği yoktur.
8. `izolasyon_pc_devri`: A PC'si B'ye devredilir → eski yerel veri `eski\sube-A-<tarih>\` altına taşınır, yeni alan boş; eski kuyruk B adına gönderilmez.
9. `izolasyon_yanit_esitligi`: B kaydı için 404 yanıtı ile var olmayan kayıt için 404 yanıtı bayt düzeyinde aynı (zaman damgası/istek kimliği hariç).
10. `izolasyon_haftalik_ozet`: reddedilen çapraz şube denemeleri haftalık özette görünür.
11. `izolasyon_imza_capraz`: A cihazından B şube imza anahtarıyla imzalı belge → 422 `imza_gecersiz`.
12. `izolasyon_yuk_15`: 15 şube eşzamanlı yükte (Ş9) rastgele örneklenen yanıtlarda başka şube kaydı yok.

## 6. Kapı
- Her yeni şubeli uç ya da tablo bu matrise satır ekler; satırsız uç birleşmez.
- K-tam, K-rls, K-uyg üçü yeşil olmadan Ş2 kapısı geçmez (Fable denetimi zorunlu).
- Sonuç dosyası: `docs/olcum/IZOLASYON-<tarih>.md` (uç sayısı, hücre sayısı, geçen/kalan).
