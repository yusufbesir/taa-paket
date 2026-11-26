# TAA Kasa — gün sonu programı

Program adı `taakasa`, exe `taa-kasa` (crates/taa-kasa-ekran). Çekirdek: crates/taa-kasa (`gunsonu`, `merkez`,
`sablon`, `xlsx`, `zaman`; eski `Defter` uyumlu). Fiyat Etiketi'ndeki "Kasa" ekranıyla ilgisi yoktur.

Tek yazar TAA Kasa'dır (`kasa.gunsonu@1`, `kasa.masraf@1`). **Kasa çift doğrulama tutanağı TAA Kasa'da üretilir ve raporlanır**
(Yusuf'un kararı, 2026-11-26): ana plandaki T6b ("Tutaa tutanağı üretir") iptal; Tutaa yalnız eski tutanakları tutar. Şablon
Tutaa'dan birebir alındı (`crates/taa-kasa/sablonlar/kasa-cift-dogrulama.json`). Sözleşmeler:
docs/sozlesmeler/kasa.gunsonu.md, docs/sozlesmeler/kasa.masraf.md.

## Akış

1. **Açılış sayımı** (çekmece başına, günde bir). Beklenen = önceki gün sonunun devreden tutarı.
2. **Gün içi:** ara sayım ve vardiya devri (ikinci kişi zorunlu, giren olamaz), masraf girişi.
3. **Gün sonu sihirbazı (5 adım):** nakit sayımı → döviz → Z raporu / POS → Nebim ve ciro satırları (akşam ciro
   Excel'inin satırları, aynı sırayla) → kontrol listesi. Sağda canlı fark paneli.
4. **İmza** (kapanış listesi tamamlanınca istek çıkar): 1. imza Kasa Sorumlusu, 2. imza Mağaza Sorumlusu / Vekili. En az bir
   imza zorunlu; ikinci imzayı giren atamaz; ikinci kişi yoksa "ikinci imza atmadı" yazılır (kayıt, Excel, tutanak, ekran, matris).
   Telefonla imza istemcisi hazır (`telefon_imza`, aşağıda "Telefonla imza"); Telefon Köprüsü onay akışı birleşip boru izni
   verilene kadar API 501 döner ve imza ekrandan atılır (yer tutucu, ekranda yazar).
5. **Tutanak ve raporlar:** Kasa Çift Doğrulama Tutanağı (A4, yazdırılabilir; `GET /api/tutanak?id=`; Nebim kasa Excel'i ERP
   sütununa içe aktarılabilir), WhatsApp özeti (`POST /api/whatsapp`, alanlar doldukça), akşam ciro Excel'i ve kayıt Excel'i aynı kayıtlardan.
6. **Gönder:** paket (imzalı, cihaz anahtarı varsa) kuyruğa yazılır, merkeze teslim edilir; ağ yoksa kuyrukta bekler.

Fark varsa açıklama zorunlu; nakit ya da Z↔ciro tutmazsa "kasa ile ciro uyuşmuyor" uyarısı çıkar.

## Kurallar

- Kayıt yalnız eklenir (JSONL). Silme yok; düzeltme = yönetici iptali (gerekçeli) + `duzeltir` izli yeni kayıt.
- Masraf: eşik (varsayılan 2.000 TL) altını müdür, üstünü yönetici (merkez) onaylar; giren onaylayamaz.
- İptal, düzeltme, ayarlar ve matris yalnız yönetici girişiyle; Excel giriş yapmış kişiye açık.
- **İkinci imza gerekçesi** (karar 2026-11-26, gözetimsiz varsayılan): ikinci imza atılmadan gönderimde 5–300 karakter
  gerekçe zorunlu (`POST /api/gerekce`, defter kaydı `gerekce`, son kayıt geçerli). Gerekçe gün durumunda, pakette
  (`ikinci_imza_gerekcesi`, `imzalar[1].gerekce`), A4 tutanakta, kayıt Excel'inde ve merkez matrisinde görünür. İkinci imza
  atılırsa gerekçe gösterilmez.
- **Sık tek imza uyarısı** (karar 2026-11-26): gerekçe kısa serbest not kalır. Aynı mağaza son `pencere_gun` günde en az
  `esik` gün ikinci imzasız gönderirse merkez matrisinde şube adının yanında "⚠ sık tek imza (n)" çıkar. Ayar
  `KasaAyar.tek_imza_uyarisi`, varsayılan 14 günde 3 gün; `esik: 0` kapatır. Aynı günün iki çekmecesi bir gün sayılır.
- **Kapanış listesi sihirbaza dağıtıldı** (karar 2026-11-26): madde, işin yapıldığı adımda işaretlenir
  (`KAPANIS_ADIMLARI`): "Fiili sayım tutanağı" → 1. adım (nakit); "Z raporu alındı" → 2. adım; "Z, POS ve satış sistemi
  karşılaştırıldı" → 3. adım; kasa kilidi, kamera, masraf fişleri, bozukluk → 4. adım. 4. adım ötekilerin durumunu ve
  ilgili adıma dönüş düğmesini gösterir. Kayıt biçimi aynı (7 × bool).
- **Tutanak kilidi:** ilk imzadan sonra tutanak ekleri (ERP içe aktarma, yapılan işlem) değişmez; değişiklik yönetici iptali
  ve yeniden girişle.
- **Z formülü ayarı** (`KasaAyar.z_formulu`): şablonun genel toplam formülü sabit; Z karşılaştırmasında `degisim_eklenir`,
  `internet_degisimi_dusulur` (25.11 ile doğrulandı), `iade_eklenir` (çıkarım), `kredi_ceki_dusulur`
  (bilinmiyor, varsayılan kapalı) ayarlanır. Gün örnekleri: `crates/taa-kasa/testdata/gun-ornekleri/*.json` (bugün yalnız
  sentetik); gerçek gün anonim tutarlarla eklenince test kendiliğinden doğrular.

## Neon görüşü (2026-11-26, anonim özetle; gerçek tutar/ad/firma gönderilmedi)

Gemini Neon'a akış, gerekçe/kilit ve telefon onayı soruldu. Değerlendirme:
- Mali alanların ilk imzadan sonra kilitlenmesi: zaten öyle (kayıt yalnız eklenir; düzeltme yönetici iptali + yeni kayıt,
  eski imzalar yeni kayda geçmez).
- Anlamsız kısa gerekçe riski: ekranda hazır gerekçe önerileri eklendi (serbest metin korunur). Tekrarlayan tek imzalı
  gönderimlere merkez alarmı ve kategori zorunluluğu Yusuf'a soru.
- Tek imzalı güne sonradan onay: ertesi gün müdür gözden geçirmesi (`ertesi_gun`) var.
- Telefon onayı: telefon özeti kendisi hesaplamalı (tasarımda var); onay telefonun donanım anahtarıyla imzalı olmalı ve
  merkez de doğrulamalı (köprü P-256 doğruluyor; merkezde doğrulama yok, açık); cihaz-kişi bağı ve iptali (TAA Merkez, planlı);
  60 sn süre PC saatine dayanır (yerel saat geri alınırsa uzar; tek kullanımlık kimlik tekrar kullanımı önler).
- Arayüz: nakit yalnız adetle girilir ve canlı fark paneli var; kapanış listesi maddelerini ilgili adımlara dağıtma
  önerisi Yusuf'a soru.

## Windows notları (denenmedi)

- Çapraz derleme: `x86_64-pc-windows-gnu` için `cargo check` ve clippy temiz; CI push'ta windows-latest'te de koşar.
- Pencere: Edge `--app` (ProgramFiles(x86), ProgramFiles, ProgramW6432, LOCALAPPDATA), yoksa varsayılan tarayıcı.
- İndirme adları ASCII'ye sadeleştirilir, asıl ad `filename*` ile gider (Türkçe şube adı başlığı düşürmez).
- Yazdırma: tutanak A4 (`@page`), "Yazdır / PDF" düğmesi Edge'de çalışmalı; ilk Windows denemesinde doğrulanmalı.
- Veri kökü `%APPDATA%\TAA\veri` (karar 2026-11-26): öteki TAA programlarıyla aynı yerde kalır, değişiklik yok. Dolaşan
  profilli etki alanında gerekirse `TAA_VERI_KOK` ile yerel klasör verilebilir. Excel açıkken aynı dosyaya atomik yazma
  reddedilebilir (indirmeler etkilenmez).

## Yer tutucular (henüz bağlanmadı)

| Konu | Şu an | Bağlanınca |
|---|---|---|
| Personel girişi | Ayarlar'daki listeden ad seçilir, PIN yok (ekranda açıkça yazar) | KM2 personel PIN'i / ortak giriş (`taa-giris.rs`) |
| Yönetici girişi | Geçici yerel PIN (tuzlu SHA-256 ×100.000, 5 hata → 5 dk kilit, oturum 10 dk) | KM1 `kimlik.yonetici@1` / Çatı PIN doğrulaması |
| Mağaza imzası | Cihaz anahtarı (`cihaz/cihaz.key`) varsa onunla, yoksa imzasız | Mağaza anahtarı |
| Merkeze teslim | Ayarlardaki `merkez_klasoru`na dosya kopyası | Sunucu kasa ucu (Ş3) |
| Fiş fotoğrafı | Yalnız dosya yolu | Dosya ekleme |
| Çatı paneli | Ayrı pencere | Çatı sekmesi |

Ortak giriş neden bağlanmadı: depoda `taa-fis-sunucu/src/ortak_giris.rs` (taa-giris.rs v1) var, ama sunucuda
`taakasa` program izni tanımlı değil (giriş `program_kapali` döner) ve rol eşlemesi (personel/müdür/yönetici)
henüz kararlaştırılmadı. Bağlanması ayrı iş olarak bırakıldı.

## Yanıtlanan sorular

Kanıt: 25.11.2026 ve 12.10.2026 dolu dosyaları (şablon, Z raporları, hareket özeti, ödeme listesi, WhatsApp metni); her eşitlik
kuruşu kuruşuna tuttu. Tutarlar depoya yazılmadı.

| Soru | Yanıt | Kanıt (ilişki) |
|---|---|---|
| Nebim hangi toplamla? | Son GENEL TOPLAM | Nebim "Peşin Satış Toplam" net = Excel son GENEL TOPLAM |
| Z hangi toplamla? | GENEL TOPLAM + DEĞİŞİM − İNTERNET DEĞİŞİMİ (+ İADE) | Üç yazarkasanın Z toplamı = bu formül = Nebim "Peşin Satış Normal" |
| NAKİT TL nedir? | Sistem nakdi, masraf ve para üstü düşülmüş | Nakit peşinat + para üstü − masraf = ödeme listesindeki "Elden Teslim" (her mağaza) |
| Kasa fazlası/açığı elle mi? | Hayır, sayımdan | KASA FAZLASI = sayılan − sistem nakdi; genel toplama girmez |
| Masraf 9 satırla sınırlı mı? | Hayır | "masraf eklendikçe olsun" (Yusuf); satırlar artar |
| Kredi kartı satırları? | Cihaza göre, bankaya göre değil | İyzico yazarkasasındaki tüm bankalar ve QR = İYZİCO POS:2; Nebim banka kırılımı kartı veren bankadır |
| Nebim havale? | BANKAYA HAVALE + İYZİCO ÖDEMESİ | Nebim "Banka Havalesi / EFT" = iki satırın toplamı |
| Nebim "Kredi Çeki"? | Değişim çeki | Nebim kredi çeki = Excel DEĞİŞİM |
| Havale farkı ve döviz toplamda? | Döviz ve masraf girer; havale farkı ve kasa fazlası girmez | 12.10 WhatsApp metninin üç toplamı bu kuralla tuttu |
| Kasiyer bazlı Nebim? | Mağaza ayrımı ödemelerden | Kasiyer satışları mağaza sütunlarından farklı; kasiyer ödemeleri birebir |

## Açık sorular (kalan)

Dolu şablon (V2 ve 25.11), aylık şablon (30 gün) ve ödeme listesi yeniden tarandı (yalnız yapı/dolu-boş bilgisi):

- **KASA AÇIĞI — çözüldü (şablondan):** MASRAFLAR bölümünün ilk satırıdır (`TOPLAM MASRAF = SUM(E17:E26)`), genel toplama
  masraf olarak girer; KASA FAZLASI genel toplam dışında bilgi satırıdır. Program böyle hesaplıyor. Gerçek açıklı gün örneği yok.
- **İADE — kısmen:** şablonda hiçbir toplama girmeyen bilgi satırı (DEĞİŞİM gibi). Z formülündeki yeri (`z_beklenen`'e eklenir)
  çıkarımdır: 25.11'de iade satırı boş olduğu için iadeli gün örneğiyle doğrulanmadı.
- **E-FATURA POS:2 — kaldırıldı (Yusuf, 2026-11-26: "E-fatura POS yok artık, iki POS terminali var").** Programda,
  ekranda, WhatsApp'ta, tutanakta ve Z ayarında yok; yeni girişte reddedilir ("tutarı İYZİCO POS'a yazın"). Şablonda 9. satır
  boş kalır, formüller ve satır numaraları değişmez. Eski kayıtlarda tutar varsa okunur, genel toplama ve kredi kartına girer,
  Excel'de "(ESKİ KAYIT)" etiketiyle yazılır. QR ve tüm banka kartları → İYZİCO POS eşlemesi aynen.
- **KREDİ ÇEKİ — toplamdaki yeri kanıtlandı (27.09.2026):** eksi olabilir; eksi tutar son genel toplamdan düşer (şablonda
  E31 = E28 + E29 + E30). 27.09'un iki mağaza toplamı ve mağaza genel toplamı bu formülle kuruşu kuruşuna tuttu (gerçek
  tutarlar depoya yazılmadı; depoda aynı yapıda sentetik örnek var). Kasa fazlası toplama girmedi, müşteriye para iadesi
  masraf kalemi olarak toplama girdi. **Z'deki yeri hâlâ kanıtsız** (27.09 Z verisi yok): `z_formulu.kredi_ceki_dusulur`,
  varsayılan kapalı. İade satırı 27.09'da da boş; `iade_eklenir` hâlâ çıkarım.
- **İkinci imza gerekçesi — uygulandı (gözetimsiz varsayılan):** zorunlu kısa gerekçe ve ilk imzadan sonra tutanak kilidi
  (Kurallar). Yusuf onaylamazsa ilgili commit geri alınabilir.
- Veri bilgisi: 25.11 Outlet yazarkasasında bir havale ödemesi nakit tipiyle girilmiş (Nebim ve Excel'de havale).
- Masraf türü listesi, kasa limiti, fark toleransı, gönderim son saati; mağaza imza anahtarı; sunucu kasa ucu.

## Telefonla imza (tasarım ve istemci)

Durum: istemci `crates/taa-kasa/src/telefon_imza.rs` hazır ve testli (sahte kanal); gerçek kanal bağlı değil, API 501.

- **Uç:** yeni HTTP ucu yok. Kasa ekranı `POST /api/imza {"id", "kanal":"telefon"}` aldığında `TelefonImzaOturumu::baslat`
  çağrılır; istek Telefon Köprüsü yerel hizmet borusuna (`\\.\pipe\TelefonKoprusuHizmet`) JSON satırı olarak gider. Köprü
  isteği TK2 TLS kanalıyla eşleşmiş telefona taşır (Uzak Köprü onay akışı; PC isteği P-256 imzalı, nonce 60 sn).
- **Sözleşme (önerilen, köprü taslağı gelince eşitlenecek):** `onay_iste@1` → `onay@1` | `onay_red@1`, `kapsam: taa_kasa_gun_sonu`.
  İstek: `istek_kimligi` (128 bit rastgele), `gun_sonu_id`, `sira`, `rol`, `istenen_kisi`, `ozet_metni`, `ozet_sha256`,
  `olusturma`, `son_gecerlilik` (+60 sn). Onay: `istek_kimligi`, `kapsam`, `ozet_sha256`, `cihaz{eslesme_kimligi, pc_pin}`,
  `kisi`, `kisi_dogrulandi`, `zaman`. Red: `istek_kimligi`, `neden`. Çözüm esnektir (bilinmeyen alan yok sayılır).
- **Özet:** gün sonu tutarlarının kanonik `anahtar=değer` satırları (sözlük sırası, kuruş tam sayı; kişi adı ve not yok) ve
  SHA-256'sı. Telefon metni gösterir, hash'i kendisi hesaplar. Onay gelince kasa özeti kayıttan **yeniden** hesaplar; istekten
  sonra düzeltme/iptal olduysa imza atılmaz.
- **Cihaz kimliği:** `eslesme_kimligi:pc_pin` (pc_pin = telefon sertifikasının SHA-256'sı); imza kaydında `cihaz`, onaylanan
  özet `ozet` alanına yazılır. Telefon imzasının (P-256) doğrulanması köprünün işidir.
- **Merkez doğrulaması** (karar 2026-11-26): cihaz-kişi bağı ve iptal TAA Merkez'de tutulur; kasa tarafı yalnız sonucu
  kullanır (`telefon_imza::CihazKaydi` → `gecerli` | `baska_kisi` | `iptal` | `bilinmiyor`). Kasada iptal edilmiş ya da
  başka kişiye bağlı cihazın onayı geçersizdir; sonuç imza kaydına `cihaz_dogrulama` olarak yazılır. Merkezde paket okunurken
  (`merkez::telefon_imzalarini_denetle`) imzanın `ozet`i paketteki gün sonundan yeniden hesaplanır ve cihaz yeniden
  denetlenir; sorun ve "doğrulanamadı" matris hücresinde görünür. TAA Merkez kaydı gelene kadar yer tutucu `MerkezKaydiYok`
  her cihaz için `bilinmiyor` döner (imza engellenmez).
- **Kişi doğrulama:** bugün yok; kişi ekranda seçilir, telefondaki kişi farklı bildirilirse onay geçersizdir. Planlı: telefonda
  Keystore + BiometricPrompt (`kisi_dogrulandi: true`), cihaz↔kişi bağı TAA Merkez'de.
- **Durumlar:** `bekliyor`, `onaylandi`, `reddedildi`, `zaman_asimi` (60 sn), `kanal_yok` (→ 501), `gecersiz` (özet/istek/kişi
  tutmuyor ya da imza kuralı). İmza kuralları (kapanış listesi, giren ikinci imzayı atamaz, en çok iki, gönderimden sonra yok)
  istek açılmadan denetlenir; ihlalde telefona istek gitmez (422).
- **501 yerine gerçek akış için gerekenler:** (1) köprüde u2b-tls-kip2 onay akışının birleşmesi ve `onay_iste@1` kapsamı;
  (2) borunun erişim listesine taa-kasa.exe izni; (3) `BoruKanali` içine boru okuma/yazma (Windows); (4) ekranda bekleme
  görünümü ve `GET /api/imza/durum?istek=` yoklaması (API bugün `bekliyor` durumunda istek kimliğini döndürecek şekilde hazır).

## Tutaa'dan taşınanlar (çift doğrulama)

| Tutaa | TAA Kasa karşılığı |
|---|---|
| `masaustu/sablonlar/kasa-cift-dogrulama.json` | `crates/taa-kasa/sablonlar/kasa-cift-dogrulama.json` + `tutanak::sablon()` |
| `masaustu/cekirdek/sablon.js` (fark = fiili − erp, gerekçe kuralı) | `tutanak::cift_dogrulama` / `cift_dogrulama_islem` |
| `web/paket.js` farktan sonucu seçme | `tutanak::sonuc_sec` |
| `web/kasa-excel.js` Nebim Excel içe aktarma | `tutanak::xlsx_oku`, `sutun_oner`, `eslestir`, `sayi_oku`, `excel_erp`; API `POST /api/tutanak` |
| Genel şablon A4 çıktısı | `tutanak::tutanak_html`; API `GET /api/tutanak?id=` |
| İki imza rolü (kâğıt) | `GunKaydi::Imza`, `KasaDefteri::imzala` (en az bir imza, "ikinci imza atmadı") |

## Akşam ciro Excel'i (şablon)

Kaynak: şubelerin "KASIM AYI CİRO ŞABLONU 2026" (V1) ve "… V2" dosyaları (Yusuf'un örnekleri; dosyalar depoya
konmadı, yalnız yapıları alındı). Kayıtlar ve Excel → "Akşam ciro Excel'i" aynı düzende aylık dosya üretir
(`/api/excel/ciro?ay=YYYY-AA`).

**Yapı (V1 ve V2 ortak):** ay başına bir dosya, gün başına bir sayfa (`01.11.2026` … `30.11.2026`), her sayfa
A1:E40 tablosu. A: satır adı; B, C: mağaza sütunları (OUTLET MAĞAZA, ANA MAĞAZA); D: TOPLAM DÖVİZ; E: tarih başlıklı
toplam.

| Satır | Ad | Toplama |
|---|---|---|
| 2 | NAKİT TL | E28 |
| 3–6 | DOLAR, EURO, STERLİN, İSVİÇRE FRANGI (adet; D = toplam döviz, E = TL karşılığı) | E28 |
| 7–9 | ZİRAAT BANKASI POS:1, İYZİCO POS:2, (9: eski E-FATURA POS:2, boş) | E28 |
| 10–11 | KREDİ KARTI +/-, KASA FAZLASI | bilgi |
| 12–15 | BANKAYA HAVALE: firma hesabı, İYZİCO ÖDEMESİ (E28), HAVALE +/- (bilgi) | |
| 16–27 | MASRAFLAR: KASA AÇIĞI (mağaza başına), 9 masraf satırı (açıklama A, tutar E), TOPLAM MASRAF = SUM(E17:E26) | E28 |
| 28 | GENEL TOPLAM = SUM(E2:E9, E13:E14, E27) | |
| 29–30 | KREDİ ÇEKİ, İNTERNET DEĞİŞİMİ | E31 |
| 31 | GENEL TOPLAM = E28 + E29 + E30 | |
| 32–33 | OUTLET / ANA MAĞAZA TOPLAMI | |
| 34–35 | döviz başlıkları ve GÜNLÜK DÖVİZ KURLARI (elle) | |
| 36–37 | mağaza başına döviz TL karşılığı (adet × kur) | |
| 38–40 | NOT, DEĞİŞİM, İADE (bilgi) | |

**V2'nin V1'den farkı:** satırlar, formüller ve sayfalar aynı. V2'de G sütununda "NASIL DOLDURULUR?" yardımı (sarı =
doldurulacak, gri = hesaplanır, Tab/Enter ile sıradaki kutu), sarı/gri renkler, sayfa koruması (yalnız sarı kutular
açık), B2'den bölme dondurma, yazdırma alanı A1:E40, giriş kutularında alt sınır doğrulaması, düzenli sütun
genişlikleri; İADE satırında V1'de toplam 0 sabitken V2'de `=B40+C40`. V1'de tablo biçimi (TableStyleMedium15) vardı.

**Program çıktısı:** V2 düzeni (renkler, koruma, dondurma, yazdırma alanı, G yardım sütunu, sayı biçimleri).
Her çekmece bir mağaza sütunu, sütun başlığı çekmece adı; A1 = Ayarlar'daki "Ciro Excel'i başlığı"; firma havale
satırının adı Ayarlar'dan. İki çekmecede hücre adresleri şablonla aynıdır; LibreOffice ile yeniden hesaplatılınca
program değerleriyle 30 sayfada fark çıkmadı. Şablondan ayrılan yerler:
- Masraf satırlarında B/C'ye masrafın çıktığı çekmece yazılır (şablonda gri ve boş); mağaza toplamı kendi masrafını
  ve kasa açığını içerir. Şablonda tüm masraf (outlet'in kasa açığı dahil) ANA MAĞAZA TOPLAMI'na yazılıyordu
  (`E33` içinde `E27`); iki mağaza toplamının toplamı yine GENEL TOPLAM'dır.
- Formüller tablo adıyla değil düz hücre başvurusuyla yazılır; veri doğrulama yazılmaz; G5'e "TAA Kasa
  kayıtlarından üretildi" notu eklenir.
- Kur, o gün kuru giren ilk çekmeceden alınır (şablonda tek kur satırı var).

## Okuyucular ve üçlü karşılaştırma (tools/kasa_hareket_oku.py)

- `hareket rapor.pdf`: Nebim Mağaza Hareket Özeti (dosya adı .xlsx olsa da PDF'tir; `pdftotext -raw`). Satış normal/iade/toplam,
  ödemeler (tip, para, kartı veren banka, peşinat, iade, diğer, toplam), masraf; kasiyer bazlı raporda her bölüm numaralanır
  (ad yazılmaz). `sablon` alanı ödemeleri şablon satırlarına toplar.
- `odeme liste.xlsx`: Alınan Ödemeler Listesi; mağaza ve para birimi başına satış, para üstü, masraf, elden teslim (müşteri adı okunmaz).
- `uclu rapor.pdf --z … --excel-genel-toplam … --internet-degisimi …`: Z ↔ Nebim Normal, Nebim Toplam ↔ Excel, Z ↔ genel toplam
  formülü, Nebim ödemeleri ↔ genel toplam. 25.11.2026'da dört satırda da fark 0; kasiyer bölümleri mağaza sütunlarıyla birebir.
- 25.11 verisi programa girilip üretilen Excel, `kasa_ciro_karsilastir.py --dolu-sayfa Sayfa1 --gun 25.11.2026` ile dolu dosyayla
  karşılaştırıldı: tüm toplamlar ve ödeme satırları aynı; kalan 2 fark yapısal (dolu dosyada EUR tutarı 0 iken EUR kuru yazılı;
  masraf dolu dosyada masraf satırlarında değil, toplamda). 12.10 verisi de girildi: WhatsApp metninin üç toplamı aynı çıktı.

## Karşılaştırma yöntemi

`tools/kasa_ciro_karsilastir.py dolu.xlsx program.xlsx [--gun 26.11.2026] [--json]`: şubenin doldurduğu Excel'i
programın aynı ay için ürettiği Excel'le satır adına göre (hücre adresine göre değil) karşılaştırır, farkları
satır × sütun olarak listeler; masraf satırları sırasız (toplam olarak) karşılaştırılır. Yalnız standart kitaplık.

Dolu sayfa / dolu Excel gelince: o günün kayıtları programa girilir (ya da test verisi olarak kurulur), program
Excel'i üretilir, araçla farklar çıkarılır. "Mağaza hareket özeti" (Nebim) ve "alınan ödemeler listesi" (POS/banka)
biçimleri görülünce aynı araca okuyucu eklenecek: hareket özeti toplamı ↔ `nebim_toplam_kurus` ve GENEL TOPLAM,
ödeme listesi ödeme türüne göre toplanıp ↔ ilgili ciro satırları (POS:1, POS:2, E-fatura, havale, İyzico ödemesi).

## Tasarım

Arayüz önerisi bir dış tasarım modelinden (yalnız sırsız genel özet gönderilerek) alındı: aşamaya göre değişen tek
ana düğme, 5 adımlı sihirbaz + yapışkan fark paneli, numpad odaklı giriş, zümrüt paleti, farkın renk + simge + işaretle
gösterimi (✓ ▲ ▼), "Sil" sözcüğünün hiç kullanılmaması.

## Çalıştırma

`taa-kasa [--kok <veri kökü>] [--pencere-acma]`. Veri kökü: `TAA_VERI_KOK`, yoksa `%APPDATA%\TAA\veri`.
Kayıtlar `<kök>/kasa/` altında (`gunsonu.jsonl`, `masraf.jsonl`, `ayar.json`, `giden/`, `gonderilen/`).
