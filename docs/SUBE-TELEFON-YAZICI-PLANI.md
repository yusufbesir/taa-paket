# TerziTakip: şube PC'si + telefondan ağ yazıcısına çıktı (plan)
Sahip: Yusuf. Karar tarihi 2026-11-06. Baskı testleri SANAL (gerçek yazıcı denenmedi).

## İstek (Yusuf)
- Şimdilik bu bilgisayar deneme sunucusu; sonra mini PC'ye geçilecek (sunucu taşınabilir olmalı).
- Her şube için 1 PC bağlanacak (şube listesi: Tutaa şube listesi dosyası, ayrı oturum çekmişti).
- Şube personeli telefondan ağdaki Epson adisyon yazıcısına çıktı gönderebilmeli (TerziTakip/fiş).

## Mevcut durum (kodda)
- `taa-fis-sunucu`: `POST /api/fis/{no}/yazdir` fişi ESC/POS bayt olarak üretir; hedef `--yazici tcp://IP[:9100]` (ağ yazıcısı) ya da dosya/USB. Testi sanal (127.0.0.1 sahte yazıcı).
- Telefon sayfası aynı sunucudan `/telefon` olarak açılıyor.
=> Telefon -> şube sunucusu -> Epson (9100) akışının çekirdeği var; eksik olan şube başına yapılandırma ve çoklu şube.

## Yapılacaklar (sırayla, her biri ayrı PR)
1. **Şube başına yazıcı adresi:** sunucu ayarına `subeler[{kod, ad, yazici}]`; telefon oturumunun şubesi (eşleşmiş cihaz kaydı) hangi yazıcıya basacağını belirler. Personel başka şubenin yazıcısını seçemez.
2. **Şube PC'si:** her şubede taafis sunucusu; kendi `fisler.jsonl`. Merkezle eşitleme taa-paket planındaki hub/Tutaa merkez DB kararına bağlı (açık soru).
3. **Telefon yazdır düğmesi** + durum mesajı ("yazıcı yok/kağıt bitti" Türkçe, `yazici.denetle` ile).
4. **Erişim (güncel karar 2026-11-06, koordinatör): VPN'siz.** Telefon merkez sunucuya HTTPS + cihaz anahtarıyla bağlanır; şubelere dışarıdan port açılmaz (aşağıdaki `taa-mobil` akışı). ~~Fortinet VPN ile gelen telefon taafis portuna ve 9100'e erişir~~ pasif 2026-11-06, yerine VPN'siz akış.
5. **Sunucu taşıma:** veri klasörü + ayar tek klasörde; mini PC'ye kopyala-başlat. Yedek: `taa-yedek`.

## Açık sorular (Yusuf)
- Epson modeli ve adisyon yazıcısının IP/port bilgisi her şubede sabit mi (DHCP rezervasyonu)?
- Telefon şube dışı ağdan (VPN) da basabilsin mi, yoksa yalnız şube Wi-Fi'sı mı?
- Şube PC'leri merkeze anlık mı, gün sonu mu eşitlensin?


## taa-mobil: telefondan Epson adisyon (VPN'siz, plan)
Kaynak: YOL-HARITASI-MERKEZ-2026-11-06.md (taa-mobil satırı). Kural: Epson'a internetten ASLA doğrudan bağlanılmaz (9100 portu kimlik doğrulamasızdır); yazıcıyı yalnız şube PC'si görür.

**Akış (şubeye gelen bağlantı yok):**
1. Personel telefonda TerziTakip sayfasını açar (merkez sunucu, HTTPS, eşleşmiş cihaz anahtarı; eşleşme `telefon.eslestir` ile mevcut).
2. Fiş seçip "Yazdır"a basar → merkez sunucu fişi `baski_kuyrugu` tablosuna yazar: {fiş no, şube kodu, nüsha, isteyen cihaz, zaman}. Şube kodu cihaz kaydından gelir; telefon başka şubenin kuyruğuna yazamaz.
3. Şube PC'sindeki `taa-mobil` hizmeti merkeze dışarı doğru HTTPS ile bağlanıp (uzun yoklama, birkaç sn) kendi şubesinin kuyruğunu çeker. Bayt üretimi (ESC/POS) şube PC'sinde `taa_fis::escpos*` ile yapılır, yazıcıya `tcp://yazici-ip:9100` ile yerel ağdan basar, sonucu (basıldı/hata) merkeze bildirir.
4. Telefon sonucu görür ("Basıldı" / "Yazıcı yanıt vermiyor, PC açık mı?").

**Neden bu yol:** şubede port yönlendirme/sabit IP gerekmez; yazıcı internete açılmaz; VPN kurulumu yok. Şube PC'si kapalıysa iş kuyrukta bekler, telefon "şube PC'si çevrimdışı" gösterir (nabız `taa-izleme` ile aynı).
**Güvenlik:** cihaz anahtarı kısa ömürlü + iptal edilebilir; kuyruk işi yalnız şubesine, tek kullanımlık, süresi dolar (ör. 10 dk); her baskı erişim günlüğüne yazılır; kuyruk uçları hız sınırlı; HTTPS'i merkezdeki Caddy yapar (yol haritası Ö1). Fiş içeriği (müşteri adı/telefon) kuyrukta kalmaz, yalnız fiş numarası gider; PC fişi merkezden kimlikli çağrıyla alır.
**Aynı Wi-Fi yedeği:** merkez erişilemezse şube PC'sinde yerel sayfa (`/telefon`, mevcut fis-sunucu) hâlâ çalışır; Çatı "çevrimdışı çalışma" kararıyla uyumlu.
**Sanal test:** merkez + şube hizmeti yerelde, sahte 9100 dinleyici (mevcut `sanal_ag_yazicisi_turkce` düzeni). Gerçek Epson ve gerçek telefon denenmedi.

### Sırayla işler (her biri ayrı PR)
1. `taa-mobil` crate: kuyruk modeli + şube çekme istemcisi + yazdırma (sanal test).
2. Merkez uç noktaları: `POST /api/baski`, `GET /api/baski/bekleyen?sube=`, `POST /api/baski/{id}/sonuc` (merkez sunucu başlığıyla uyum).
3. Telefon sayfasında Yazdır düğmesi + durum.
4. Şube PC hizmeti (Windows hizmeti/başlangıç) ve `taa-izleme` nabzı.
Merkez sunucu tamamlanana kadar 1. adım yerel kuyrukla (dosya) çalışır.
