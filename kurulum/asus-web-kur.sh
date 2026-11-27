#!/usr/bin/env bash
# TAA Web kabuğunu Asus Linux sunucuya kurar (statik sunum + systemd + Tailscale HTTPS).
# Kullanım (depo kökünden):  sudo bash kurulum/asus-web-kur.sh [--port 8791] [--hedef /opt/taa/web-kabuk] [--https-port 8444] [--sunucu http://127.0.0.1:<taa-sunucu-portu>]
# Geri alma: sudo systemctl disable --now taa-web && sudo tailscale serve --https=8444 off
# Not: taa-sunucu (axum) bu klasörü kendi içinden sunacak hale gelince bu betik gereksiz kalır;
#      o zamana dek kabuk ayrı küçük bir statik sunucudan çıkar. Veri uçları (/v1/...) merkez sunucudadır.
set -euo pipefail
PORT=8791; HTTPSPORT=8444; HEDEF=/opt/taa/web-kabuk; SUNUCU=http://127.0.0.1:8080  # taa-sunucu (axum) yerel adresi; gerçek porta göre --sunucu ile ver
while [ $# -gt 0 ]; do case "$1" in
  --port) PORT="$2"; shift 2;; --hedef) HEDEF="$2"; shift 2;; --sunucu) SUNUCU="$2"; shift 2;; --https-port) HTTPSPORT="$2"; shift 2;; *) echo "bilinmeyen: $1"; exit 2;; esac; done
KOK="$(cd "$(dirname "$0")/.." && pwd)"
[ -d "$KOK/web/kabuk" ] || { echo "web/kabuk yok: $KOK"; exit 1; }
command -v python3 >/dev/null || { echo "python3 gerekli"; exit 1; }

# 1) Dosyalar (yalnız web/, test klasörü hariç)
mkdir -p "$HEDEF"
rsync -a --delete --exclude 'test/' "$KOK/web/" "$HEDEF/web/"
echo "sürüm: $(git -C "$KOK" rev-parse --short HEAD 2>/dev/null || echo bilinmiyor)" > "$HEDEF/SURUM.txt"

# 2) systemd: yalnız 127.0.0.1'e bağlanır (dışarı Tailscale serve açar)
cat > /etc/systemd/system/taa-web.service <<UNIT
[Unit]
Description=TAA Web kabuğu (statik)
After=network.target
[Service]
ExecStart=/usr/bin/python3 -m http.server $PORT --bind 127.0.0.1 --directory $HEDEF
Restart=on-failure
DynamicUser=yes
ProtectSystem=strict
NoNewPrivileges=yes
[Install]
WantedBy=multi-user.target
UNIT
systemctl daemon-reload
systemctl enable --now taa-web
sleep 1
curl -fsS "http://127.0.0.1:$PORT/web/kabuk/index.html" >/dev/null && echo "yerel sunum TAMAM"

# 3) HTTPS (service worker + WebCrypto güvenli bağlam ister; düz http://100.x çalışmaz)
if command -v tailscale >/dev/null; then
  # Kabuk "/v1/..." uçlarını kendi kökünden çağırır (uygulama.js: SUNUCU = location.origin).
  # python http.server POST'a 501 döner; bu yüzden /v1 yolu taa-sunucu'ya yönlendirilir, geri kalanı statik sunulur.
  tailscale serve --bg --https=$HTTPSPORT "http://127.0.0.1:$PORT" || echo "UYARI: tailscale serve başarısız (HTTPS sertifikası tailnet ayarından açılmalı)"
  tailscale serve --bg --https=$HTTPSPORT --set-path /v1 "${SUNUCU%/}/v1" || echo "UYARI: /v1 yönlendirmesi kurulamadı"
  tailscale serve status || true
  echo "Adres: https://<makine-adı>.<tailnet>.ts.net:$HTTPSPORT/web/kabuk/index.html"
else
  echo "UYARI: tailscale yok; HTTPS olmadan kabuk çevrimdışı çalışmaz."
fi
echo "Doğrulama: aşağıdaki istek 501 DEĞİL, taa-sunucu yanıtı (örn. 200/401/404 uc_yok) vermeli:"
curl -ks -o /dev/null -w "POST /v1/giris -> HTTP %{http_code}\n" -X POST -H 'Content-Type: application/json' -d '{}' "https://$(tailscale status --json 2>/dev/null | python3 -c 'import json,sys;print(json.load(sys.stdin)["Self"]["DNSName"].rstrip("."))' 2>/dev/null):$HTTPSPORT/v1/giris" || true
