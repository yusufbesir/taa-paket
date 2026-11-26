<!-- Sahip: Yusuf. yazan: [Claude#sozlesme-RH0 2026-11-15]. Durum: SÖZLEŞME (RH0). Değişiklik: Yusuf'un onayıyla. -->
# Rehber sözleşmesi (RH0)

Kaynak kararlar: `planlar/PLAN-TAA-PAKET-ANA-2026-11-15.md` §3 ve §12 v2 (Yusuf onayı 2026-11-15 14:28: ekran TAA Rehber'de; v2: tek yazar sunucudaki yazma hizmeti), `planlar/PLAN-EPOSTA-REHBERI-SOKUM.md` §4 ve §8 (GPT/Gemini görüşleri, kabul edilenler; §4 tablosunun yerine geçer), `planlar/PLAN-ORTAK-VERI-HAVUZU-2026-11-12.md` GÜNCELLEME 2026-11-15, `planlar/PLAN-SUBE-ORTAK-SUNUCU-2026-11-15.md` §2.5.
Bağlı belgeler: `docs/havuz-sozlesmesi.md`, `docs/havuz-turleri.toml`, `docs/sube-sozlesmesi.md`, `docs/sube-izolasyon-testleri.md`.

## 1. Kapsam
- Tek doğruluk kaynağı TAA Sunucu'dur; dağıtım havuzdan yapılır.
- **Tek yazar: sunucudaki rehber yazma hizmeti `rehber.yaz@1`** (ana plan v2 §3.2). Katalogda (`havuz-turleri.toml`) doğrulayıcı biçimi gereği `rehber-yaz` adıyla durur (AÇIK-R6).
- Hizmete komutu **yalnız merkezi yönetici girişi** (`kimlik.yonetici@1`) yapmış kişi gönderir. Yönetim ekranı TAA Rehber'dedir (Yusuf kararı); TAA Çatı aynı hizmete bağlanabilir ama ikinci ekran yazılmaz.
- Yönetici girişi olmayan TAA Rehber (ör. şube PC'si) yalnız okur ve öneri bırakır (§7).
- **Çevrimdışı yazma yoktur.** Kişi, birim, görevlendirme, rol ve belge yalnız sunucuya bağlıyken değişir; çevrimdışı çakışma doğmaz. Yerel önbellek salt okunurdur (§9).
- Okuyanlar: `taarehber` (Eğitim bölümü dahil: kime hangi eğitim), `tutaa` (alıcı önerisi, onay adları), `terzitakip` (usta ve şube), `taayonetici` (Çatı / yönetici paneli). Başka program okumaz.
- Rehber yalnız adres verir; e-posta göndermez. Alıcıyı her zaman kullanıcı seçer.

## 2. Türler (katalog: `docs/havuz-turleri.toml`)
| Tür | Kip | Yazar | `kisisel` | Anahtar |
|---|---|---|---|---|
| `rehber.kisi@1` | tablo | `rehber.yaz@1` | true | `id` |
| `rehber.birim@1` | tablo | `rehber.yaz@1` | false | `id` |
| `rehber.gorevlendirme@1` | tablo | `rehber.yaz@1` | true | `id` (benzersiz: `kisi_id + birim_id + rol + baslangic`) |
| `rehber.rol@1` | tablo | `rehber.yaz@1` | false | `kod` |
| `rehber.belge@1` | tablo, sürümlü | `rehber.yaz@1` | false | `konu_id + surum` |

Kimlik değerleri (`id`) kararlıdır, yeniden kullanılmaz; biçim `[a-z0-9][a-z0-9_-]{0,63}` (sunucu üretir, öneri: uuid v4 küçük harf).

### 2.1 `rehber.kisi@1` (yalnız kimlik ve iletişim)
| Alan | Tür | Zorunlu | Not |
|---|---|---|---|
| `id` | metin | evet | **Kalıcı kimlik**; tüm eşleştirmeler bununla yapılır |
| `ad` | metin (1–120) | evet | Ad soyad |
| `eposta` | metin | evet | **Yalnız iş e-postası**; biçim denetimi (bugünkü `epostaGecerli`); etkin kişiler arasında benzersiz |
| `durum` | `etkin` \| `pasif` | evet | Ayrılan kişi `pasif` olur |
| `pasif_tarihi` | tarih `YYYY-AA-GG` | `durum=pasif` ise evet | |
| `surum` | tam sayı ≥1 | evet | Değişiklik sürümü; hizmet her değişiklikte artırır |
| `guncelleme` | zaman (UTC, RFC 3339) | evet | Sunucu yazar |

- Kişide **görev, rol, amir ve telefon alanı yoktur.** Görev, rol ve amir birime ve tarihe bağlıdır, `rehber.gorevlendirme@1` üzerindedir. Telefon ilk sürümden çıkarıldı (KVKK asgari veri; söküm planı §8).
- Departman ve mağaza **kişi değildir**; `rehber.birim@1` kaydıdır. Kişinin iş adresi (`eposta`) ile birimin ortak adresi (`rehber.birim.ortak_eposta`) ayrıdır.
- **Eşleştirme anahtarı e-posta değil, kalıcı `id`'dir.** Göçte ve öneride e-posta yalnız *aday eşleşme* üretir; birleştirme yöneticinin onayıyla olur.
- Yasak alanlar: kişisel e-posta, her türlü telefon (ilk sürüm), ev adresi, kimlik numarası, doğum tarihi, fotoğraf. Şema bilinmeyen alanı reddeder.

### 2.2 `rehber.birim@1`
| Alan | Tür | Zorunlu | Not |
|---|---|---|---|
| `id` | metin | evet | |
| `ad` | metin (1–120) | evet | |
| `tur` | `sube` \| `magaza` \| `departman` \| `merkez` | evet | `sube` = 15 şubeden biri (`subeler.kod`, `docs/sube-sozlesmesi.md` §3); `magaza` = bir şubenin altındaki satış noktası (örn. Outlet, `sube_kodu` bağlı olduğu şubedir, Esenyurt Outlet için `MRK01`); `departman` ve `merkez` şube değildir |
| `ust_birim_id` | metin | hayır | Başka birim; döngü yasak |
| `sube_kodu` | metin | `tur=sube` ya da `magaza` ise evet | `docs/sube-sozlesmesi.md` §3 şube kodu |
| `ortak_eposta` | metin | hayır | Birimin ortak adresi (mağaza, departman, merkez) |
| `durum` | `etkin` \| `pasif` \| `arsiv` | evet | Kapanan mağaza `arsiv`; silinmez |
| `surum` | tam sayı ≥1 | evet | Değişiklik sürümü |
| `guncelleme` | zaman | evet | |

### 2.3 `rehber.gorevlendirme@1` ("kim nerede görevli")
| Alan | Tür | Zorunlu | Not |
|---|---|---|---|
| `id` | metin | evet | Kalıcı kimlik |
| `kisi_id` | metin | evet | `rehber.kisi` `id` |
| `birim_id` | metin | evet | `rehber.birim` `id` |
| `rol` | metin | evet | İletişim rolü, `rehber.rol` `kod` (erişim yetkisi değildir) |
| `gorev` | metin (≤120) | hayır | İş unvanı (görünen); erişim yetkisi değildir |
| `amir_id` | metin | hayır | Amir olan kişinin `rehber.kisi` `id`'si; döngü yasak; **ayrı izinle görünür** |
| `tur` | `asil` \| `gecici` \| `vekil` | evet | Asıl görev yeri, geçici görev, vekâlet. Bir kişinin aynı anda en çok bir etkin `asil` satırı |
| `vekil_edilen_id` | metin | hayır | `tur=vekil` için yerine bakılan kişi |
| `durum` | `etkin` \| `pasif` | evet | Yanlış girilen satır `pasif` olur (silinmez) |
| `baslangic` | tarih | evet | |
| `bitis` | tarih | hayır | Boş = sürüyor; dolunca kayıt geçmişte kalır |
| `surum` | tam sayı ≥1 | evet | Değişiklik sürümü |
| `guncelleme` | zaman (UTC) | evet | Zaman damgası, sunucu yazar |

Kişi birden çok birimde olabilir. Geçmiş silinmez; görev bitince `bitis` dolar. Benzersizlik: etkin satırlarda `kisi_id + birim_id + rol + baslangic`. `gecici` ve `vekil` satırlarında `bitis` önerilir.

### 2.4 `rehber.rol@1` (iletişim rolü)
**Erişim yetkisi değildir.** Yetki rolleri kimlik hizmetindedir (`kullanici_rol`, `kimlik.yonetici@1`); iki liste karıştırılmaz ve birbirine eşlenmez.
| Alan | Tür | Zorunlu | Not |
|---|---|---|---|
| `kod` | metin `[a-z][a-z0-9_-]{0,31}` | evet | Değişmez; şablonlar bu koda bağlanır |
| `ad` | metin (1–60) | evet | Görünen ad (bugünkü `ROL_AD`) |
| `aciklama` | metin (≤200) | hayır | |
| `durum` | `etkin` \| `pasif` | evet | Pasif rol yeni atamada seçilemez, eski kayıtta kalır |

### 2.5 `rehber.belge@1` (El Kitabı)
| Alan | Tür | Zorunlu | Not |
|---|---|---|---|
| `konu_id` | metin | evet | |
| `surum` | tam sayı ≥1 | evet | Her yayın yeni sürüm; eski sürüm okunur kalır |
| `baslik` | metin (1–160) | evet | |
| `govde` | metin (Markdown alt kümesi, ≤64 KiB) | evet | Betik, HTML ve dış bağlantılı görsel yok |
| `etiketler` | metin listesi | hayır | Arama için |
| `yayin` | zaman | evet | |
| `yenilik_notu` | metin (≤300) | hayır | "Yenilikler" gösterimi |

## 3. Sürümleme
- Tür sürümü katalogda (`surum = 1`), istek `tür@1` ile. Uyumsuz değişiklik `@2` yeni türdür; `@1` en az bir sürüm daha yayımlanır (havuz kuralı).
- Satır değişikliği havuzda yeni sürüm olarak yayımlanır (tek yazar + sürüm zinciri); ilgili satırın `surum` alanı artar. Hizmet, istekteki beklenen `surum` güncel değilse 409 `cakisma` döner (iki yönetici aynı kaydı değiştirirse). İstemci `sira` imleciyle eşitler.
- Alan eklemek uyumludur (okuyucu bilinmeyen alanı yok sayar); alan kaldırmak ya da anlamını değiştirmek `@2` ister.

## 4. Silme yok
- Kişi, birim, rol: `durum = pasif`. Görevlendirme: `bitis` tarihi. Belge: yeni sürüm ya da havuzda "arşivle".
- Her değişiklik denetim izine yazılır (kim, ne zaman, hangi tür, hangi `id`; e-posta maskeli).
- Saklama süresi ve kalıcı yok etme yalnız Yusuf kararı ve hukuki görüşle (ŞUBE planı B3; çalışan aydınlatma notu ANA plan §10 G5).

## 5. Görünürlük ve izinler
Havuzun üç süzgeci geçerlidir (`docs/havuz-sozlesmesi.md`), **bir farkla: rehber türlerinde boş `sube_kapsam` "herkes" değil "yalnız merkez" sayılır.** Kapsam her istekte sunucuda uygulanır; istemcinin gönderdiği şube koduna güvenilmez; PostgreSQL satır düzeyi güvenlik ek kilittir. Rehber türleri için sunucu ek olarak **satır ve alan süzgeci** uygular; süzülen satır ve alan istemciye hiç gönderilmez, önbelleğe de girmez.

| İzin | Kime (varsayılan) | Açtığı |
|---|---|---|
| `havuz.rehber.kisi.oku` (ve diğer `rehber.*` türleri için aynı kalıp) | Okuyan programlarda oturum açan herkes | Temel okuma (aşağıdaki şube süzgeciyle) |
| `rehber.tum.oku` | Yalnız **merkez rolü** | Tüm şirketin kişi listesi |
| `rehber.dahili.oku` | (ayrılmış) | İlk sürümde telefon alanı yok; ileride eklenirse bu izinle görünür |
| `rehber.amir.oku` | Merkez rolü, şube müdürü | Görevlendirmedeki `amir_id` ve hiyerarşi ağacı |
| `rehber.disa_aktar` | **Yalnız yönetici** | CSV/JSON dışa aktarma (denetim izine yazılır) |
| `rehber.yaz` | Yalnız merkezi yönetici girişi yapmış kişi | `rehber.yaz@1` hizmetine komut: kişi, birim, görevlendirme, rol, belge; öneri onayı. Yerel PIN bu izni vermez (`docs/sube-sozlesmesi.md` §4.1) |

**Şube PC'si (cihazın şubesi S, `rehber.tum.oku` yok) görür:**
1. Etkin görevlendirmesi S şubesindeki bir birimde olan kişiler ve o görevlendirme satırları.
2. `tur = merkez` ya da `departman` birimlerinin `ortak_eposta` adresleri ve bu birimlerde görevli kişilerin yalnız `ad`, `eposta` ve görevlendirmedeki `gorev`, `rol` alanları.
3. Tüm `tur = magaza` / `sube` birimlerinin `ad` ve `ortak_eposta` alanları.
4. **Her mağazanın müdürü ya da sorumlusu** (v2): `rol` = mağaza müdürü / sorumlusu rol kodu olan etkin görevlendirmedeki kişilerin yalnız `ad`, `eposta` ve o satırın `gorev` alanı. Başka şubenin diğer kişileri görünmez.
5. `rehber.rol@1` ve `rehber.belge@1` tamamı (kişisel değil).

Kapsam üç yerde aynı uygulanır: sunucu, dağıtılan veri ve yerel önbellek; süzme arayüze bırakılmaz. Şube cihazdan türetilir (`docs/sube-sozlesmesi.md` §4); istemci beyanı esas alınmaz. Şubeler arası iletişim sıklığı pilotta ölçülür, kural ona göre gözden geçirilir. Pasif kişiler yalnız `rehber.tum.oku` ile listelenir; alıcı önerisinde hiç çıkmaz.

## 6. Kişisel veri kuralları
- Yalnız iş e-postası tutulur; telefon ilk sürümde yoktur (§2.1). Önbellek kullanıcı ve şube kapsamlıdır, alanlar asgaridir.
- Geçmiş tutanaklarda alıcı ve onay adları gönderim anındaki hâliyle saklanır; rehber değişince eski belge değişmez (RH3 testi).
- Bayat önbellekle otomatik gönderim yok: önbellek yaşı e-posta panelinde gösterilir, alıcıyı her zaman kullanıcı onaylar.
- Günlükte e-posta maskelenir: `a***@terziademaltun.com` biçimi (yerel kısmın ilk harfi + `***` + alan adı). Ad günlüğe yazılmaz; `id` yazılır.
- Rehber verisi **dış yapay zekâya (Gemini, GPT vb.) ve ajan hafızasına gönderilmez**. Örnek veri gerekirse uydurma kişilerle üretilir.
- Dışa aktarma yalnız `rehber.disa_aktar` ile; dosya adı ve satır sayısı denetim izine yazılır.
- En çok 2000 kişi (bugünkü doğrulama sınırı şemaya taşınır).

## 7. Öneri akışı (`veri.oner@1`)
Yazar olmayan programlar yazmaz, öneri bırakır. **Öneriler ayrı, denetimli bir kuyrukta bekler** (sunucuda `rehber_oneri` kuyruğu); rehberi doğrudan değiştirmez. Kuyruğun her durumu (geldi, incelendi, onaylandı, reddedildi) denetim izine yazılır; yalnız `rehber.yaz` sahibi görür.
| Alan | Tür | Zorunlu | Not |
|---|---|---|---|
| `tur` | metin | evet | Hedef tür, ör. `rehber.kisi@1` |
| `islem` | `ekle` \| `duzelt` \| `pasiflestir` | evet | |
| `hedef_id` | metin | `duzelt`, `pasiflestir` için evet | |
| `alanlar` | nesne | `ekle`, `duzelt` için evet | Hedef türün şemasından geçer |
| `gerekce` | metin (≤300) | hayır | |
| `kaynak_program` | metin | evet | Sunucu oturumdan doldurur, istemci değeri yok sayılır |
| `oneri_id` | uuid | evet | Tekrar gönderim tek kayıt |
| `dosya_kimligi` | metin | göç satırında evet | Aynı göç dosyası ikinci kez işlenirse çoğaltmaz |

- Örnek: Tutaa e-posta panelinde "bu adresi rehbere öner"; göç dosyası (`rehber-goc-<pc>-<tarih>.json`) satırları da öneri olarak girer; mükerrer ve çakışan kayıtlar (aynı e-posta, farklı ad) inceleme kuyruğuna düşer, e-posta yalnız aday eşleşmedir.
- Yönetici TAA Rehber'de onaylar ya da gerekçeyle reddeder; onaylanan öneriyi `rehber.yaz@1` yazar; sonuç önerene durum olarak döner (`bekliyor`, `onaylandi`, `reddedildi`).
- Öneri içindeki e-posta günlükte maskelidir. Öneri kendisi kişisel veridir: yalnız `rehber.yaz` sahibi görür.

## 8. Rol kodu CI denetimi
Kural: Tutaa şablonlarındaki `eposta.oneri` rol kodlarının her biri `rehber.rol@1` içinde etkin bir `kod` olmalıdır; değilse CI kırmızıdır.

Sunucudaki tablo CI'dan okunamaz. Bu yüzden **rol kodları sözleşme düzeyindedir**: kod listesi depoda `docs/rehber-rolleri.toml` (RH1'de oluşturulur, ilk liste Tutaa `rehber.js` `ROLLER` / `ROL_AD`'dan çıkarılır). `rehber.yaz@1` hizmeti (RH1) bu listede olmayan kodu reddeder; ad ve açıklama serbesttir.

**Betik taslağı (arayüz; uygulama RH1/RH3):** `tools/rehber_rol_denetle.py`, yalnız stdlib, Python 3.11+.
```
python3 tools/rehber_rol_denetle.py --roller docs/rehber-rolleri.toml \
    --sablonlar <tutaa şablon klasörü> [--sablon-deseni "*.json"] [--json]
```
| Öğe | Tanım |
|---|---|
| Girdi `--roller` | TOML: `[roller."<kod>"] ad = "…"`, `durum = "etkin" \| "pasif"` |
| Girdi `--sablonlar` | Şablon dosyaları; her dosyada isteğe bağlı `eposta.oneri` (metin listesi ya da `{rol: …}` nesneleri; RH3 Tutaa biçimini okuyup kesinleştirir) |
| Denetimler | (1) Şablondaki her rol kodu listede var; (2) kod `etkin`; (3) kod biçimi `[a-z][a-z0-9_-]{0,31}`; (4) liste içinde yinelenen kod yok; (5) hiçbir şablonda geçmeyen kod yalnız **uyarı** |
| Çıktı (metin) | `HATA: <dosya>: rol '<kod>' rehber.rol listesinde yok` …; son satır `rol denetimi: tamam` ya da `BAŞARISIZ` |
| Çıktı (`--json`) | `{"hatalar": [...], "uyarilar": [...], "sablon_sayisi": N, "rol_sayisi": M}` |
| Çıkış kodu | 0 temiz, 1 hata var, 2 kullanım/dosya okunamadı |
| Test | `tools/test_rehber_rol_denetle.py`: geçerli, eksik rol, pasif rol, bozuk TOML, boş şablon klasörü |
| Nerede koşar | Tutaa deposunda (şablonlar orada). Tutaa özel deposunun Actions kotası doluysa `ci_yerel` ile yerelde koşar, çıktı PR'a kanıt olarak eklenir. taa-paket CI'ı yalnız `rehber-rolleri.toml` biçimini denetler. |

## 9. Yerel önbellek
- Yer: `taa-havuz` program önbelleği (`<veri kökü>/havuz-onbellek/<program>/`); eski yerel `rehber` alanı (`%APPDATA%\TAA\rehber\`) yalnız bu önbelleğin salt okunur kopyasıdır.
- **İmzalı:** istemci her eşitlemede sunucunun imzalı özetini (tür, `sira`, sha256) doğrular; imza ya da sha256 tutmazsa önbellek kullanılmaz, son geçerli kopya kalır.
- **Salt okunur:** okuyan programlar önbelleğe yazmaz; düzeltme yalnız öneriyle (§7).
- Yetki daralırsa önbellek bir sonraki eşitlemede yeni kapsamla yeniden yazılır (önbellek asıl veri değil, kopyadır); kapsam dışı kalan satırlar önbellekten çıkar.
- Çevrimdışı yazma yoktur; yönetim ekranı sunucu yokken yazma düğmelerini kapatır ve "sunucuya bağlı değil" der.
- Cihazın şubesi değişirse önbellek `eski\sube-<kod>-<tarih>\` altına taşınır (silinmez), yeni şubeyle baştan eşitlenir.
- Sunucu yokken son kopya okunur; hiç kopya yoksa Tutaa alıcı kutusu boş açılır, adres elle yazılır.

## 10. Kabul testleri (RH1–RH7 bunları geçirmeden birleşmez)
1. Katalog: `python3 tools/havuz_denetle.py` temiz; beş tür tek yazarlı (`rehber-yaz` = `rehber.yaz@1`).
2. Şema: zorunlu alan eksikse, bilinmeyen alan varsa, yasak alan varsa (kişide `gorev`, `roller`, `amir_id`, `telefon`) yazım reddedilir.
3. E-posta biçimi ve benzersizliği; 2001. kişi reddedilir.
4. Görevlendirmedeki `amir_id` ve birimdeki `ust_birim_id` döngüsü reddedilir; yetim `amir_id` reddedilir.
5. Bir kişinin iki etkin `tur=asil` görevlendirmesi reddedilir; `gecici` ve `vekil` satırları asıl satırla birlikte durabilir.
6. Silme ucu yok; pasifleştirme ve `bitis` sonrası eski satır geçmişte okunur.
7. Yazma yalnız `rehber.yaz@1` hizmeti üzerinden, merkezi yönetici girişiyle; yönetici girişi olmayan oturum, yerel PIN ya da başka program 403. Sunucu yokken yazma denemesi istemcide engellenir, kuyruğa alınmaz.
8. Şube A cihazı B şubesindeki kişiyi okuyamaz, listeleyemez, sayamaz (`docs/sube-izolasyon-testleri.md` rehber satırları).
9. Şube cihazı merkez/departman ve mağaza ortak adreslerini ve her mağazanın müdürünü/sorumlusunu (yalnız ad, görev, e-posta) görür.
10. `rehber.amir.oku` yokken `amir_id` yanıtta ve önbellekte yoktur.
11. `rehber.tum.oku` yalnız merkez rolünde tam listeyi döndürür.
12. Dışa aktarma `rehber.disa_aktar` olmadan 403; olunca denetim izine yazılır.
13. Günlük taraması: hiçbir günlük satırında açık e-posta yok (maskeli biçim).
14. Dış yapay zekâ çağrısı yapan kod yolunda rehber verisi yok (kod incelemesi + sır/kişisel veri taraması).
15. `veri.oner@1`: tekrar gönderim tek kayıt; onay sonrası tek yazar yazar; ret gerekçesi önerene döner.
16. Rol denetimi: şablonda olmayan/pasif rol CI'da kırmızı (§8 testleri).
17. Önbellek: imza bozuksa kullanılmaz; çevrimdışı açılış son kopyayla çalışır; okuyan program önbelleğe yazamaz.
18. Cihaz şube değişimi: eski önbellek taşınır, yeni şube verisi karışmaz.
19. Yetki daralınca sonraki eşitlemede önbellek yeni kapsamla yeniden yazılır.
20. Eşzamanlı iki yönetici düzenlemesi: eski `surum` ile gelen yazma 409.
21. `rehber.rol@1` kodu yetki kararında kullanılmaz (kimlik hizmeti rolleriyle karışmaz).
22. `sube_kapsam` alanı olmayan ya da boş rehber satırını yazma hizmeti reddeder (`kapsam_zorunlu`); satır hiç oluşmaz. Okuma yolunda da boş kapsam şube cihazına gönderilmez (yalnız merkez, ikinci kilit).
23. Eşleştirme `id` ile: aynı e-postalı iki öneri otomatik birleşmez, inceleme kuyruğuna düşer; aynı göç dosyası iki kez işlenince kayıt çoğalmaz.
24. Öneri rehberi doğrudan değiştirmez; onaylanmamış öneri hiçbir okuyucuda görünmez.

## 11. Açık noktalar
- AÇIK-R1: Ana plan §3.2 (v2) tablosu kişide hâlâ "görev (iş unvanı), bağlı olduğu amir, isteğe bağlı dahili telefon" sayıyor; söküm planı §8 (Fable kabulü) ve koordinatör kararı bunları kişiden çıkarıyor (görev/rol/amir görevlendirmeye, telefon ilk sürümden çıkar). Bu sözleşme §8'i uyguladı; ana plan tablosu düzeltilmeli. Dahili telefonun ayrı izinle görünmesi (§3.3) ilk sürümde karşılıksız kaldı; `rehber.dahili.oku` ayrılmış ad olarak duruyor.
- KAPANDI-R9 (Fable denetimi PR #55, koşul 1): rehber satırında kapsam ZORUNLUDUR: `merkez`, `tum` (tüm şubeler) ya da şube kodu listesi. `rehber.yaz@1` kapsamsız satırı reddeder. `rehber.rol@1` ve `rehber.belge@1` açıkça `tum` kapsamıyla yayımlanır. `taa-havuz` okuma süzgecinde boş kapsam yine "yalnız merkez" sayılır (ikinci kilit); asıl kural yazmada uygulanır.
- AÇIK-R6 (koşul 2): RH1'in ilk PR'ında doğrulayıcıya ayrı `[hizmetler]` listesi eklenir; hiçbir program.toml rehber türleri için `havuz_yazar` bildiremez. Geçici olarak `rehber-yaz` program listesinde durur.
- AÇIK-R2: Daha açık rehber (şube başka şubenin kişilerini görsün mü) ve kişi ekleme yetkisi (şube müdürü de mi): ANA plan §10 K7, söküm planı §7. Varsayılan: temkinli görünürlük + mağaza müdürü/sorumlusu görünür (v2), yazma yalnız merkezi yönetici.
- AÇIK-R3: Rol kodları için depoda ikinci kopya (`docs/rehber-rolleri.toml`) gerekir; tek doğruluk kaynağı ilkesiyle gerilim. Çözüm: kodlar sözleşmedir (depo), ad/açıklama veridir (sunucu).
- AÇIK-R4: `rehber.belge@1` için ana plan kip belirtmiyor; `tablo` seçildi (metin konu). Görsel/PDF gerekirse ayrı `dosya` türü (`rehber.ek@1`) eklenir.
- AÇIK-R6: Yazar bir program değil hizmettir (`rehber.yaz@1`); `tools/havuz_denetle.py` yazarları `[programlar] adlar` listesinde ve `^[a-z0-9-]{1,32}$` biçiminde istediği için katalogda `rehber-yaz` adıyla program listesine eklendi. Doğrulayıcıya ayrı `[hizmetler]` listesi (ad `rehber.yaz@1`) eklenmeli; ayrıca `program_denetle` hiçbir programın `havuz_yazar` ile rehber türü bildirmesine izin vermemeli (RH1).
- AÇIK-R7: Mağaza müdürü/sorumlusu görünürlüğü için hangi `rehber.rol@1` kod(lar)ının "müdür/sorumlu" sayılacağı `docs/rehber-rolleri.toml`'da işaretlenmeli (ör. `sube_gorunur = true`); ilk liste RH1'de.
- AÇIK-R8: Söküm planı §4 görevlendirme anahtarını `kisi_id+birim_id+baslangic` ve `asil` (mantıksal) olarak veriyordu; §8 ve v2 ile anahtar kalıcı `id`, `asil` yerine `tur` (asıl, geçici, vekil), benzersizlik `kisi_id+birim_id+rol+baslangic` oldu. `gorev` (unvan) koordinatörün alan listesinde yok ama §8 "görev görevlendirmeye taşınır" dediği için isteğe bağlı alan olarak eklendi.
- AÇIK-R5: `rehber.kisi` saklama süresi (pasif kişiler) ve çalışan aydınlatma notu Yusuf/hukuki görüşte (G5, B3).
