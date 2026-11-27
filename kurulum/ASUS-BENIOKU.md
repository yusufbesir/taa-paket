# TAA Web — Asus sunucuya kurulum

1. `git pull` (Forgejo ya da GitLab taa-paket main).
2. `sudo bash kurulum/asus-web-kur.sh` (varsayılan: 127.0.0.1:8791, `/opt/taa/web-kabuk`, Tailscale HTTPS :8444 (taa-sunucu 127.0.0.1:8443 ile çakışmasın; `--https-port` ile değişir)).
3. Tarayıcıda `https://<makine>.<tailnet>.ts.net:8444/web/kabuk/index.html` aç; çevrimdışı deneme: ağ kes, sayfayı yenile.
4. Güncelleme: aynı betiği yeniden çalıştır. Geri alma: betik başındaki satır.

Not: kabuk `/v1/...` uçlarını kendi kökünden çağırır; betik `/v1` yolunu `--sunucu` ile verilen taa-sunucu adresine yönlendirir (varsayılan http://127.0.0.1:8080, gerçek porta göre ver). Yönlendirme yoksa giriş HTTP 501 verir (statik sunucu POST bilmez).

Bilinen sınırlar: kabuk yalnız statik; `/v1/...` uçları taa-sunucu'dadır ve cihaz imzası ister (tarayıcı cihaz anahtarı henüz yok, docs/WEB-ESITLE-SOZLESMESI.md §10 madde 1), bu yüzden veri yazımı o iş bitene kadar sunucuda reddedilir. Şube düğümü (`taa-sube-dugum`) Asus'a değil şube PC'lerine kurulur.
