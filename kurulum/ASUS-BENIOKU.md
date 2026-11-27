# TAA Web — Asus sunucuya kurulum

1. `git pull` (Forgejo ya da GitLab taa-paket main).
2. `sudo bash kurulum/asus-web-kur.sh` (varsayılan: 127.0.0.1:8790, `/opt/taa/web-kabuk`, Tailscale HTTPS :8443).
3. Tarayıcıda `https://<makine>.<tailnet>.ts.net:8443/web/kabuk/index.html` aç; çevrimdışı deneme: ağ kes, sayfayı yenile.
4. Güncelleme: aynı betiği yeniden çalıştır. Geri alma: betik başındaki satır.

Bilinen sınırlar: kabuk yalnız statik; `/v1/...` uçları taa-sunucu'dadır ve cihaz imzası ister (tarayıcı cihaz anahtarı henüz yok, docs/WEB-ESITLE-SOZLESMESI.md §10 madde 1), bu yüzden veri yazımı o iş bitene kadar sunucuda reddedilir. Şube düğümü (`taa-sube-dugum`) Asus'a değil şube PC'lerine kurulur.
