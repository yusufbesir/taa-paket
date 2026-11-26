#!/bin/sh
# Ortak kodu iki hedefe kopyalar (düzenleme yalnız ortak/takip.mjs'de yapılır).
cd "$(dirname "$0")" && for f in takip.mjs diller.mjs ek.mjs; do cp "ortak/$f" "cloudflare/$f" && cp "ortak/$f" "vercel/lib/$f" || exit 1; done
