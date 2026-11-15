<!-- Sahip: Yusuf. yazan: [Claude#sozlesme-S0 2026-11-15]. Durum: SÖZLEŞME (Ş0). Değişiklik: Yusuf'un onayıyla. EB0 eki işlendi: [Claude#sozlesme-EB0 2026-11-15], `docs/esleme-sozlesmesi.md` §1.5 D17–D20 ve Fable denetimi K2, K3, K6, K8. -->
# Şube tehdit modeli (Ş0)

Kaynak: `planlar/PLAN-SUBE-ORTAK-SUNUCU-2026-11-15.md` §2.4, §2.5, §2.6, §3.1, §6, EK, EK 2; `planlar/PLAN-TAA-PAKET-ANA-2026-11-15.md` §5.3, §12. Bağlı: `docs/sube-sozlesmesi.md`, `docs/sube-izolasyon-testleri.md`, `docs/rehber-sozlesmesi.md`, `docs/esleme-sozlesmesi.md` (aynı şubede birden çok bilgisayar), `docs/sozlesmeler/yazdir.sun.md`.
Test adları uygulama aşamasında (Ş2, Ş3, KM1, RH1, EB1–EB4) aynen kullanılır. Bir şubede 1 ya da daha çok bilgisayar olabilir; "şube PC'si" şubenin bilgisayarlarından herhangi biridir.

## 1. Varlıklar
| # | Varlık | Nerede | Değer |
|---|---|---|---|
| V1 | Müşteri verisi (ad, telefon, fiş içeriği) | Sunucu (Yusuf'un bilgisayarı) + şube bilgisayarlarının yerel defteri (çok bilgisayarlı şubede her eşte bir kopya) | KVKK, en yüksek |
| V2 | Ciro tutarları | Sunucu + şube | Ticari sır |
| V3 | Tutanaklar ve ekleri | Sunucu + şube | Hukuki kanıt |
| V4 | Rehber (çalışan iş e-postası, görevlendirme) | Sunucu + salt okunur önbellek | Kişisel veri |
| V5 | Şube imza anahtarı (özel) | Yalnız üreten şube bilgisayarı (eş başına ayrı anahtar), DPAPI sarılı | Belge geçerliliği |
| V6 | Cihaz anahtarı (özel) | Her şube bilgisayarında `<kök>\cihaz\` | İstek kimliği |
| V7 | Sunucu makbuz anahtarı, yayın imza anahtarı | Sunucu / yayın ortamı (AI dokunmaz) | Bütünlük |
| V8 | Yönetici hesapları (argon2 özet, TOTP sırrı) | Sunucu PostgreSQL | Tüm sistem yetkisi |
| V9 | Çevrimdışı yetki belgeleri (7 gün) | Şube bilgisayarı (her eş için ayrı belge) | Geçici yetki |
| V10 | Gece yedekleri | Sunucu, şifreli | Tüm veri |
| V11 | Denetim izi (`olaylar`) | Sunucu | Kanıt |
| V12 | Eş listesi (sunucu imzalı: şube kodu, cihaz kimlikleri, açık anahtarlar, roller) | Sunucu + çok bilgisayarlı şubede her eşin önbelleği | Eş kapısının kime açık olduğunu belirler |
| V13 | Yazdırma kuyruğu, iş içerikleri ve basılan işler defteri | İsteyen bilgisayar + yazıcı sahibi eş (DPAPI sarılı) | Müşteri verisi taşır; çift basma ve çekmece güvenliği |
| V14 | Numara sırası ve yedek aralıklar | Sıra tutan eş + her eşin yedek bloğu + sunucu (`numara_araliklari`) | Belge numarası tekilliği |

## 2. Saldırgan türleri
| # | Saldırgan | Yetenek |
|---|---|---|
| S1 | Meraklı içeriden (şube personeli) | Kendi şubesinde geçerli hesap ve cihaz; başka şubeyi görmek ister |
| S2 | Kötü niyetli şube yöneticisi | Şube bilgisayarlarından birinde Windows yönetici yetkisi; belgeyi değiştirmek, iz silmek ister |
| S3 | PC hırsızı | Fiziksel erişim, hesap yok |
| S4 | Ağ saldırganı | Mağaza Wi-Fi'si / modem üzerinden dinleme, araya girme, sahte sunucu |
| S5 | Eski çalışan | Bilinen eski parola, kopyalanmış dosyalar |
| S6 | Dış saldırgan (internet) | Açık kapılara tarama, parola deneme |
| S7 | Ele geçirilmiş yazılım/AI ajanı | Depoya ya da hafızaya veri sızdırma |

## 3. Saldırı yüzeyi
- Sunucu 443 (Caddy + axum): giriş, şubeli uçlar, havuz uçları, yönetim uçları. PostgreSQL yalnız 127.0.0.1.
- Taşıma: Tailscale → WireGuard → doğrudan HTTPS (ileride Cloudflare Tunnel).
- Şube bilgisayarı (her biri): yerel defter, kuyruk, havuz önbelleği, DPAPI sarılı anahtarlar; telefon eşleştirme yerel sunucusu (6 haneli kod; yalnız telefon sunucusu rolündeki eşte, eş kapısından ayrı kapı); eş kapısı (yalnız çok bilgisayarlı mağazada, yalnız yerel ağ, `docs/esleme-sozlesmesi.md` §5); yazdırma hizmeti ve iş klasörü (`yazdir.sun@1`).
- Tutaa (JS) köprüsü: yerel HMAC zinciri, `imza.belge@1` isteği.
- Yükleme yolları: tutanak eki, havuz dosyaları, içe aktarma (CSV, göç dosyası).
- Günlükler, hata mesajları, yedek dosyaları, ajan hafızası, dış AI istemleri.
- Güncelleme kanalı (taa-yayin, imzalı paket).

## 4. Tehditler, önlemler, testler
| # | Tehdit | Saldırgan | Önlem | Test adı | Kalan risk |
|---|---|---|---|---|---|
| T1 | Başka şubenin kaydını okuma/listeleme/sayma | S1 | Şube cihazdan türetilir; `yetki::izinli` + RLS; başka şube kaydına 404 | `izolasyon_*` matrisi | Uygulama ve RLS aynı hatayı yaparsa; iki kilit ayrı test edilir |
| T2 | Başka şube adına yazma (`X-TAA-Sube`/gövde sahteciliği) | S1, S2 | Beyan yok sayılır; uyuşmazlık 409 + `yetki.red.sube` | `sube_beyan_uyusmaz_409` | Yok (sunucu kuralı) |
| T3 | Çalınan şube bilgisayarı | S3 | Çatı'dan cihaz ve şube imzası iptal; oturumlar düşer; yerel veri DPAPI; Windows hesabı parolalı | `calinti_cihaz_iptal_red`, `es_iptal_cihaz_red` | İptalden önceki çevrimdışı süre (en çok 7 gün) ve Windows oturumu açık bırakılmışsa yerel veri; çok bilgisayarlı şubede çalınan PC şubenin tüm eşlenmiş kayıtlarını taşır |
| T4 | Kopyalanan şube imza anahtarı | S2, S5 | DPAPI (başka PC'de açılmaz); iki anahtar şartı; iptal | `imza_baska_cihazdan_red`, `imza_iptal_sonrasi_red` | **Yazılımla tam devredilemezlik yok**: PC'de yönetici olan S2 anahtarı o PC'de kullanabilir |
| T5 | İstek imzasının belge imzası yerine kullanılması | S2 | Alan ayrımı etiketi `TAA-BELGE-v1` | `imza_etiket_karisma_red` | Yok |
| T6 | Tekrar saldırısı | S4 | İstek imzasında zaman (±5 dk) + tek kullanımlık değer; `no`/`olay_id` tek kayıt | `istek_tekrar_red`, `olay_tekrar_tek_kayit` | Yok |
| T7 | Sahte sunucu / araya girme | S4 | Kendi kök sertifika sabitlenir; Tailscale/WireGuard; makbuz imzası doğrulanır | `sahte_sunucu_sertifika_red`, `makbuz_imza_dogrula` | Kök sertifika kurulumunun kendisi ele geçirilirse |
| T8 | Kayıp/çalınan yedek | S3, S5 | Gece `pg_dump` şifreli; anahtar yedekle aynı yerde tutulmaz; dış buluta konmaz | `yedek_sifreli_acilmaz`, `geri_yukleme_tatbikati` | Yedek anahtarı kaybolursa geri yükleme imkânsız (Yusuf'ta yazılı saklama) |
| T9 | Yönetici hesabı ele geçirme | S5, S6 | argon2, TOTP zorunlu, 5 yanlışta kilit, yeniden doğrulama 15 dk, yalnız merkez cihazından yönetim | `yonetici_totp_zorunlu`, `giris_kilit_5` | TOTP cihazı da çalınırsa |
| T10 | Yerel PIN ile yönetici yetkisi | S1, S2 | Kayıtlı cihazda yerel PIN yönetici izni vermez; yerel kipe geri düşme yok | `kayitli_cihaz_pin_yetkisiz` | Paketsiz kurulumda PIN sürer (bilerek) |
| T11 | Çevrimdışı belge kopyalama | S2, S5 | Belge kullanıcı + cihaz anahtarı + şube + izne bağlı | `cevrimdisi_belge_baska_cihaz_red` | — |
| T12 | Saat geri alarak süresi dolmuş belge kullanma | S2 | Son sunucu zamanı + artan sayaç; geri giderse yönetici eylemi durur; sunucu eşitlemede yeniden denetler | `saat_geri_alma_yonetici_durur` | Saat hiç ileri alınmadan 7 gün içinde kullanım (kabul edilen gecikme) |
| T13 | Hata mesajı/günlükten sızıntı | S1 | 404 ile "yok" ve "başkasının" ayırt edilmez; mesajda şube adı, kişi verisi, SQL yok; günlükte ad/telefon/e-posta maskeli | `hata_mesaji_sizinti`, `gunluk_maske_taramasi` | Yan kanal (yanıt süresi farkı) [ÖLÇ] |
| T14 | Sayım/istatistikten çıkarım | S1 | Sayım uçları da RLS'den geçer; şube yalnız kendi sayısını alır | `izolasyon_*_sayim` | — |
| T15 | Zararlı ek dosya | S1, S2 | MIME izin listesi, boyut sınırı, sha256, yürütülebilir dosya reddi, zarf şifreleme | `ek_mime_red`, `ek_yurutulebilir_red` | Görüntü işleyici açığı (bağımlılık güncel tutulur, `cargo audit`) |
| T16 | Havuz zehirleme (sahte şablon) | S2 | Tablo türünde tek yazar; yazma izinleri; havuz imzalı özet; önbellek imza denetimi | `havuz_yetkisiz_yazma_red`, `onbellek_imza_bozuk_kullanilmaz` | Yazar programın yönetici hesabı ele geçerse (T9) |
| T17 | RLS atlatma (uygulama rolüyle) | S7 | Uygulama rolü tablo sahibi değil, `NOBYPASSRLS`; `FORCE ROW LEVEL SECURITY` | `rls_rol_bypass_yok` | Veritabanı süper kullanıcısı (yalnız sunucu PC'sinde, yerel) |
| T18 | Müşteri/rehber verisinin dış AI'ya, hafızaya, buluta gitmesi | S7 | Kural: dış bulut yok; sır ve kişisel veri taraması; ajan notları imzalı | `kisisel_veri_taramasi`, `dis_ai_istem_denetimi` | İnsan hatası; kod incelemesiyle sınırlanır |
| T19 | Kuyruk öğesini başka şubeye kaydırma (PC devri) | S2 | Kuyruk öğesi şube damgalı; damga uymazsa gönderilmez; eski veri `eski\sube-<kod>-<tarih>\` | `pc_devri_kuyruk_gonderilmez` | — |
| T20 | Telefon eşleştirme kötüye kullanımı | S1 | 6 haneli kod yerel; telefon yalnız telefon sunucusu rolündeki eşe bağlanır, kapısı eş kapısından ayrıdır (telefon kayıtlı eş değildir, eş kapısına bağlanamaz); merkeze telefon değil şube bilgisayarı yazar | `telefon_merkeze_yazamaz`, `telefon_es_kapisina_baglanamaz` | Şube içi yetkisiz telefon (yerel ağ) |
| T21 | Hız sınırını aşan deneme/kazıma | S6, S1 | 429 + `Retry-After`; merkez maskesiz ayrıntı açma saatlik sınır | `hiz_siniri_429` | Sayılar [ÖLÇ] |
| T22 | Rehber öneri kuyruğuyla rehberi değiştirme | S1 | Öneri ayrı denetimli kuyrukta; yalnız `rehber.yaz@1` yazar | `oneri_dogrudan_yazmaz` | — |
| T23 | **Sahte eş cihaz**: yerel ağda kendini eş gibi tanıtan bilgisayar | S1, S4 | Karşılıklı kimlik; eş listesi sunucu imzalı; `TAA-ES-v1` etiketli el sıkışma; iptal edilen cihaz listeden düşer | `es_kayitsiz_cihaz_red`, `es_baska_magaza_cihazi_red`, `es_iptal_cihaz_red`, `es_liste_imza_bozuk_red`, `es_imza_etiket_karisma_red`, `es_kesif_sahte_duyuru_red` | Kopuk mağazada iptal gecikmesi (§5.2) |
| T24 | **Yerel ağda dinleme / araya girme** | S4 | Şifreli bağlantı, ileri gizlilik, anahtar sabitleme; düz metin kip yok | `es_baglanti_sifresiz_red`, `es_araya_girme_red`, `es_ag_dokumu_duz_metin_yok`, `es_mesaj_tekrar_red` | Trafik miktarı ve zamanı görünür (içerik görünmez) |
| T25 | **Yazdırma kuyruğuna sahte iş** | S1, S4, S7 | Eşten yalnız kimliği doğrulanmış eş; her iş kim, hangi cihaz, hangi belge ile deftere; eş başına hız ve kuyruk sınırı; içerik türü ve boyut sınırı | `yazdir_kayitsiz_es_red`, `yazdir_hiz_siniri`, `yazdir_kuyruk_dolu`, `yazdir_bicim_red`, `yazdir_is_deftere_yazilir` | Aynı bilgisayarda aynı kullanıcıyla çalışan kötü niyetli süreç yerel IPC'den iş gönderebilir (`docs/ipc.md` güvenlik sınırı) |
| T26 | **Numara aralığı tüketme**: bozuk ya da kötü niyetli eşin sürekli numara/blok istemesi | S2, S7 | Blok yalnız önceki blok %80 dolunca; eş başına numara isteği hız sınırı; her verilen numara ve blok defterde; atlama eşiği aşılınca Çatı uyarısı | `numara_blok_tukenme_siniri`, `numara_istek_hiz_siniri`, `numara_atlama_deftere_yazilir`, `numara_ayni_istek_ayni_no` | Sınır içinde kalan yavaş tüketme; Çatı uyarısıyla görülür |
| T27 | Eş kapısına internetten, VPN'den ya da başka alt ağdan erişim | S6, S1 | Yalnız yerel arayüz; güvenlik duvarı Özel profil + yerel alt ağ; uygulama içi adres denetimi | `es_kapi_tailscale_kapali`, `es_kapi_genel_profil_kapali`, `es_kapi_alt_ag_disi_red`, `es_tek_pc_kapi_kapali` | Mağaza modemine izinsiz bağlanan cihaz kimlik denetimine kadar ulaşır (T23 ile durur) |
| T28 | Eşleme ile başka mağaza verisinin gelmesi ya da gitmesi | S2 | Kayıt şube damgası denetlenir; başka şube damgalı kayıt uygulanmaz; PC devrinde eş listesinden düşme | `es_baska_sube_damgali_kayit_red`, `es_pc_devri_baglanti_kesilir` | — |
| T29 | Çift basma (bütünlük) | — | Tek iş kimliği; yazıcı sahibinde basılan işler defteri; "basılıyor" iken çöküşte otomatik yeniden basma yok | `yazdir_cift_basma_yok`, `yazdir_yeniden_gonderim_tek_baski`, `yazdir_cokus_belirsiz_durum` | Çöküşte "basıldı mı" belirsiz kalabilir; kullanıcıya sorulur |
| T30 | Eşten gelen kaydın sahte sürümle ezilmesi | S2 | Eklemeli kayıt; aynı öncülden iki sürüm = çakışma, iki hal saklanır; otomatik seçim yok | `es_cakisma_iki_hal_saklanir`, `es_son_yazan_kazanmaz` | Şube müdürü yanlış hali seçebilir (insan kararı, denetim izinde) |
| T31 | **Para çekmecesini uzaktan açma**: eşten gelen fiş çıktısına çekmece açma komutu gömmek | S1, S2, S4 | Eşten gelen `escpos` içeriğinde çekmece açma komutu (ESC p, DLE DC4) reddedilir; çekmece yalnız bağlı olduğu bilgisayardan, ayrı `cekmece_ac` işlemiyle açılır ve deftere yazılır; eş kapısında `cekmece_ac` yoktur | `yazdir_esten_cekmece_komutu_red`, `yazdir_cekmece_esten_ac_red`, `yazdir_cekmece_acma_deftere_yazilir` | Yazıcılı bilgisayarda oturum açmış kişi çekmeceyi yerelden açabilir (yetki ve defter ile görünür) |
| T32 | **İçerik yoluyla dosya okuma**: `icerik_yolu` ile hizmete başka dosya okutmak | S1, S7 | Hizmet yalnız kendi `yazdir\is\` klasöründen, adı `is_id` ile aynı dosyayı okur; üst klasör, mutlak başka yol, kısayol ve bağlantı (symlink, junction) reddedilir; içerikler DPAPI ile sarılı, arşiv süresi yazılı | `yazdir_yol_disari_red`, `yazdir_yol_kisayol_red`, `yazdir_icerik_sarili` | Aynı Windows kullanıcısıyla çalışan kötü niyetli süreç klasörü doğrudan okuyabilir (`docs/ipc.md` sınırı) |
| T33 | **Bozuk ya da dev PDF ile yazıcılı bilgisayarı aksatma** | S1, S7 | Boyut ve sayfa sınırı; PDF ayrı süreçte açılır, zaman aşımı; açılamayan dosya reddedilir, hizmet çalışmaya devam eder; basım aracı EB2'de adıyla yazılır | `yazdir_pdf_boyut_siniri`, `yazdir_pdf_sayfa_siniri`, `yazdir_pdf_bozuk_red` | Basım aracının kendi açığı (araç güncel tutulur) |
| T34 | **Sürüm düşürme**: eşin eski protokol sürümü bildirerek zayıf kipe zorlaması | S4, S2 | El sıkışmada protokol sürümü bildirilir; eski protokole düşülmez; uyuşmazlıkta eş kapısından iş alınmaz, mağaza "yalnız merkez yoluyla eşleme, yalnız yerel yazdırma" kipine geçer ve ekranda söyler | `es_surum_dusurme_yok`, `es_surum_uyusmaz_merkez_kipi` | Güncelleme dalgasında eşleme yalnız merkez yoluyla yürür (gecikme) |

## 5. Önlem katmanları özeti (plan §2.5)
1. Ayrı kimlik (ortak parola yok, `kullanici_rol.sube_id`). 2. Cihaz onayı. 3. Sunucuda iki kilit: `yetki::izinli` + RLS. 4. Havuz `sube_kapsam` ve izinler (rehberde boş kapsam = yalnız merkez). 5. İstemci: cihaza ait önbellek ve şube damgalı kuyruk. 6. Eklemeli denetim izi, `yetki.red.sube`, haftalık "başka şubeye erişim denemesi" özeti. 7. Ajan hafızası imzalı; şube adı, müşteri verisi ve anahtar hafızaya ve dış AI'ya yazılmaz.

## 6. PostgreSQL satır düzeyi güvenlik (ikinci kilit)
- Her şubeli tabloda `ALTER TABLE … ENABLE ROW LEVEL SECURITY` ve `FORCE ROW LEVEL SECURITY`.
- Uygulama bağlantısı ayrı rol (`taa_uygulama`): tablo sahibi değil, `NOBYPASSRLS`, süper kullanıcı değil.
- Her istek işleminde (transaction) sunucu, cihazdan türettiği değerleri atar: `SET LOCAL taa.sube_id = '<id>'`, `SET LOCAL taa.merkez = 'true|false'`. Değer istemciden gelmez.
- İlke (taslak): `USING (sube_id = current_setting('taa.sube_id', true)::bigint OR current_setting('taa.merkez', true) = 'true')`; yazmada `WITH CHECK (sube_id = current_setting('taa.sube_id', true)::bigint)` (merkez şube kaydı yazamaz, yalnız ayrı olay tablosuna onay/iptal/not ekler).
- Ayar atanmamışsa (`current_setting` boş) ilke **hiçbir satır döndürmez** (varsayılan reddet).
- `tutanaklar` için ek: `OR current_setting('taa.sube_id', true)::bigint = ANY(taraf_subeler)` yalnız okumada.
- Rehber tablolarında boş kapsam yalnız `taa.merkez = 'true'` ile görünür.
- Maliyet `rls_olc` ile ölçülür [ÖLÇ]; ölçüm olmadan "ucuz" denmez.
- Kanıt: izolasyon matrisi iki kipte koşar: (a) tam sistem, (b) uygulama süzgeci test bayrağıyla kapalı, yalnız RLS (`docs/sube-izolasyon-testleri.md` §2).

## 7. Kalan riskler (kabul edilen)
- Yazılımla tam "devredilemez anahtar" garantisi yok; iki anahtar şartı ve iptal akışıyla sınırlandı.
- Çevrimdışı şubede yetki iptali en çok 7 gün gecikir.
- Sunucu tek PC (Yusuf'un bilgisayarı): merkez görünümü kesilebilir; şubeler çalışmaya devam eder. Fiziksel güvenliği ve disk şifrelemesi (BitLocker ertelendi) Yusuf'tadır.
- Uçtan uca şifreleme yok (sunucu rapor için okur; 11-11 kararı).
- Yan kanal (yanıt süresi) ölçülmedi.
- Çok bilgisayarlı şubede internetten kopukken iptal edilen cihaz, eş listesi yenilenene kadar yerel ağda eş sayılabilir; 7 günden eski listeyle eş kapısı yalnız okuma ve yazdırmaya izin verir, numara bloğu vermez; 5. günden itibaren ekranda uyarı (`docs/esleme-sozlesmesi.md` §5.2, E10 kararı).
- Yerel ağda trafik miktarı ve zamanı görünür (içerik şifreli).

## 8. Açık noktalar (Ş0 ve RH0 ortak liste)
- **AÇIK-S1 (kapandı, ana plan §16.1; Yusuf 2026-11-15):** önekler kesindir, `docs/sube-sozlesmesi.md` §3. Eski metin: Plan şube kimliği olarak `subeler.kod` ve fiş öneki örneği `PND` veriyor; şube listesinde yalnız `id` (ör. `istanbul-pendik`) var. Sözleşme `kod = id` aldı; 3 harfli önekler öneridir, Yusuf onayı gerekir.
- **AÇIK-S2 (kapandı, ana plan §16.1; Yusuf 2026-11-15):** Esenyurt'ta (Merkez Mağaza) sunucudan ayrı 2 kasa bilgisayarı vardır; ikisi de `istanbul-merkez` şubesinin normal bilgisayarıdır, `docs/esleme-sozlesmesi.md`. Eski metin: "Merkez" iki anlamda: merkez rolü (yönetim, sunucu, `sube_id = null` cihaz) ve Merkez Mağaza şubesi (`istanbul-merkez`, Esenyurt Outlet dahil). Merkez Mağaza'nın şube PC'si sunucu PC'sinden ayrı mı, belirtilmemiş. Varsayım: ayrı, normal şube PC'si.
- **AÇIK-S3:** "Dış bulut yok" kuralı ile bağlantı yolları arasında gerilim: Tailscale koordinasyon hizmeti dış hizmettir (veri taşımaz, trafik uçtan uca WireGuard); planlanan Cloudflare Tunnel TLS'yi Cloudflare'de sonlandırırsa müşteri verisi dış hizmetten geçer. Öneri: Cloudflare Tunnel yalnız uçtan uca TLS ile (kaynakta sonlandırma) ya da hiç.
- **AÇIK-S4:** Plan "gece şifreli yedek, şifreli kopya" diyor ama kopyanın yerini söylemiyor; "dış bulut yok" kararına göre kopya yalnız yerel/çıkarılabilir diskte varsayıldı.
- **AÇIK-S5:** Hız sınırı sayıları ölçülmedi; `docs/sube-sozlesmesi.md` §9.5 değerleri öneridir.
- **AÇIK-S6:** Tutaa `API-SOZLESMESI.md` bu oturumda okunmadı (kapsam dışı tutuldu); tutanak uç alanları ve mevcut hata gövdesi biçimiyle uyum Ş5'te denetlenmeli.
- **AÇIK-S7:** Şube imza, makbuz, yönetici belgesi, rehber komut ve öneri uçlarının adları planda yok; "(öneri)" olarak yazıldı, Ş3/KM1/RH1'de kesinleşir.
- **AÇIK-S8:** Cihaz imzası başlık adları taa-cihaz'dadır; bu belge yeni başlık tanımlamadı (yalnız `X-TAA-Sube`).
- Rehber açıkları (AÇIK-R1 … R9): `docs/rehber-sozlesmesi.md` §11. Eşleme ve yazdırma açıkları (AÇIK-E*, AÇIK-Y*): `docs/esleme-sozlesmesi.md` §8, `docs/sozlesmeler/yazdir.sun.md`.
