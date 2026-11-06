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
4. **Erişim:** yalnız şube ağı/VPN; internete açık port yok (KVKK). Fortinet VPN ile gelen telefon yalnız taafis portuna ve 9100'e erişir (güvenlik duvarı kuralı Yusuf/BT).
5. **Sunucu taşıma:** veri klasörü + ayar tek klasörde; mini PC'ye kopyala-başlat. Yedek: `taa-yedek`.

## Açık sorular (Yusuf)
- Epson modeli ve adisyon yazıcısının IP/port bilgisi her şubede sabit mi (DHCP rezervasyonu)?
- Telefon şube dışı ağdan (VPN) da basabilsin mi, yoksa yalnız şube Wi-Fi'sı mı?
- Şube PC'leri merkeze anlık mı, gün sonu mu eşitlensin?
