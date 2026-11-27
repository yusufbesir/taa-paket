#!/usr/bin/env bash
# Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-kurulum 2026-11-27]
# Android'e bağlı olmayan Kotlin sınıflarını (SunucuAdresi, YolListesi, Gunluk) JVM'de derleyip sınar.
# Android SDK ve Gradle derlemesi GEREKMEZ; yalnız JDK ve Kotlin derleyicisi gerekir. Derleyici şuradan alınır:
#   1) kotlinc PATH'teyse o;  2) KOTLIN_LIB=<kotlin-compiler-embeddable ve kotlin-stdlib jar'larının klasörü>;
#   3) /opt/gradle*/lib (Gradle dağıtımının içindeki Kotlin derleyicisi).
# Uygulamanın tamamını (AnaEtkinlik, KurulumEtkinligi) DERLEMEZ; onlar için Windows'ta derle.ps1 gerekir.
set -euo pipefail
cd "$(dirname "$0")/.."
KAYNAK=app/src/main/java/com/taa/terzitakip
DOSYALAR=("$KAYNAK/SunucuAdresi.kt" "$KAYNAK/YolListesi.kt" "$KAYNAK/Gunluk.kt" sinama/YolSinamasi.kt)
CIKTI="$(mktemp -d)"
if command -v kotlinc >/dev/null 2>&1; then
  kotlinc "${DOSYALAR[@]}" -include-runtime -d "$CIKTI/sinama.jar" 2>&1 | grep -v '^warning:' || true
  exec java -Dstdout.encoding=UTF-8 -Dfile.encoding=UTF-8 -jar "$CIKTI/sinama.jar"
fi
LIB="${KOTLIN_LIB:-}"
if [ -z "$LIB" ]; then
  for k in /opt/gradle*/lib "${GRADLE_HOME:-/yok}/lib"; do
    if ls "$k"/kotlin-compiler-embeddable-*.jar >/dev/null 2>&1; then LIB="$k"; break; fi
  done
fi
[ -n "$LIB" ] || { echo "Kotlin derleyicisi bulunamadı: kotlinc kurun ya da KOTLIN_LIB verin." >&2; exit 2; }
STDLIB="$(ls "$LIB"/kotlin-stdlib-[0-9]*.jar | head -1)"
java -cp "$LIB/*" org.jetbrains.kotlin.cli.jvm.K2JVMCompiler -no-stdlib -no-reflect -cp "$STDLIB" -d "$CIKTI" "${DOSYALAR[@]}"
exec java -Dstdout.encoding=UTF-8 -Dfile.encoding=UTF-8 -cp "$CIKTI:$STDLIB" YolSinamasiKt
