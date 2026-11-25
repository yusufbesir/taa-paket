#!/bin/sh
# Ortak kodu iki hedefe kopyalar (düzenleme yalnız ortak/takip.mjs'de yapılır).
cd "$(dirname "$0")" && cp ortak/takip.mjs cloudflare/takip.mjs && cp ortak/takip.mjs vercel/lib/takip.mjs
