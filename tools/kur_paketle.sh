#!/usr/bin/env bash
# Sahip: Yusuf. yazan: [Claude#taa-paket-kurulum 2026-11-09]
# TAA Kur kurulum programını üretir: dist/TAA-Kur-Kurulum.exe (kurulum + kaldırma, tek dosya).
# İçine gömülür: taa-kur.exe, taa-kur-pencere.exe, taa-guncelle.exe, kur.toml (kaynak_url), anahtarlar/*.pub.
# Gerekenler: rustup target x86_64-pc-windows-gnu, ziglang (pip install ziglang), cargo-zigbuild.
# Kullanım: tools/kur_paketle.sh --anahtar <taa.pub> [--kaynak-url <url>] [--surum <x.y.z>]
set -euo pipefail
cd "$(dirname "$0")/.."
KAYNAK_URL="https://github.com/yusufbesir/taa-yayin/releases/latest/download"
ANAHTAR=""; SURUM=""
while [ $# -gt 0 ]; do
  case "$1" in
    --anahtar) ANAHTAR="$2"; shift 2;;
    --kaynak-url) KAYNAK_URL="$2"; shift 2;;
    --surum) SURUM="$2"; shift 2;;
    *) echo "Bilinmeyen seçenek: $1" >&2; exit 2;;
  esac
done
[ -f "$ANAHTAR" ] || { echo "HATA: --anahtar <taa.pub> (güvenilir açık anahtar) verilmeli: imzasız paket kabul edilmesin diye gömülür." >&2; exit 2; }
HEDEF=x86_64-pc-windows-gnu
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$PWD/target}"
cargo zigbuild --release --target "$HEDEF" -p taa-kur -p taa-guncelle
R="$CARGO_TARGET_DIR/$HEDEF/release"
PAKET="$(mktemp -d)"
cp "$R/taa-kur.exe" "$R/taa-kur-pencere.exe" "$R/taa-guncelle.exe" "$PAKET/"
printf 'kaynak_url = "%s"\n' "$KAYNAK_URL" > "$PAKET/kur.toml"
mkdir "$PAKET/anahtarlar" && cp "$ANAHTAR" "$PAKET/anahtarlar/$(basename "$ANAHTAR")"
export TAA_KUR_PAKET="$PAKET"
[ -n "$SURUM" ] && export TAA_KUR_SURUM="$SURUM"
cargo zigbuild --release --target "$HEDEF" --manifest-path kurulum/Cargo.toml --target-dir "$CARGO_TARGET_DIR/kurulum"
mkdir -p dist
cp "$CARGO_TARGET_DIR/kurulum/$HEDEF/release/taa-kur-kurulum.exe" dist/TAA-Kur-Kurulum.exe
sha256sum dist/TAA-Kur-Kurulum.exe | tee dist/TAA-Kur-Kurulum.exe.sha256
echo "Hazır: dist/TAA-Kur-Kurulum.exe"
