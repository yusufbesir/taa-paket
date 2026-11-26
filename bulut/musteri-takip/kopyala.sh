#!/bin/sh
# Ortak kodu iki hedefe kopyalar (düzenleme yalnız ortak/ klasöründe yapılır: takip, diller, ek, onay, imza).
cd "$(dirname "$0")" && for f in takip.mjs diller.mjs ek.mjs onay.mjs imza.mjs; do cp "ortak/$f" "cloudflare/$f" && cp "ortak/$f" "vercel/lib/$f" || exit 1; done
