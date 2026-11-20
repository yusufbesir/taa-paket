# Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17]
# Terzi Takip Android uygulamasını Windows'ta derler ve APK'yı dist\TerziTakip-<sürüm>.apk olarak kopyalar.
# JDK (17+) ve Android SDK'yı ortam değişkenlerinden ya da varsayılan kurulum yerlerinden bulur.
# Hiçbir şey silmez: dist\ içinde aynı adlı eski APK varsa dist\eski\ altına tarihli adla taşınır.
#
# Kullanım:
#   powershell -ExecutionPolicy Bypass -File .\derle.ps1           # release (imza anahtarı yoksa debug anahtarıyla)
#   powershell -ExecutionPolicy Bypass -File .\derle.ps1 -Debug    # debug
param(
    [switch]$Debug
)

$ErrorActionPreference = 'Stop'
$Kok = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $Kok

function Yaz([string]$Metin) { Write-Host "[derle] $Metin" }
function Dur([string]$Metin) { Write-Host "[derle] HATA: $Metin" -ForegroundColor Red; exit 1 }

# ---------------------------------------------------------------- JDK
function JavaSurumu([string]$JavaEv) {
    $java = Join-Path $JavaEv 'bin\java.exe'
    if (-not (Test-Path $java)) { return 0 }
    # JDK kökündeki "release" dosyası: JAVA_VERSION="17.0.12" (Temurin, Microsoft, JBR, Zulu, Corretto).
    $satir = ''
    $release = Join-Path $JavaEv 'release'
    if (Test-Path $release) {
        $satir = (Select-String -Path $release -Pattern '^JAVA_VERSION=' | Select-Object -First 1).Line
    }
    if (-not $satir) {
        $onceki = $ErrorActionPreference
        $ErrorActionPreference = 'Continue'
        $satir = ((& $java -version 2>&1) | Select-Object -First 1) -as [string]
        $ErrorActionPreference = $onceki
    }
    if ($satir -match '"(\d+)(\.(\d+))?') {
        $ana = [int]$Matches[1]
        if ($ana -eq 1 -and $Matches[3]) { $ana = [int]$Matches[3] }
        return $ana
    }
    return 0
}

$adaylar = @()
if ($env:JAVA_HOME) { $adaylar += $env:JAVA_HOME }
$adaylar += @(
    "$env:ProgramFiles\Android\Android Studio\jbr",
    "$env:LOCALAPPDATA\Programs\Android Studio\jbr"
)
foreach ($desen in @(
    "$env:ProgramFiles\Eclipse Adoptium\jdk-*",
    "$env:ProgramFiles\Microsoft\jdk-*",
    "$env:ProgramFiles\Java\jdk-*",
    "$env:ProgramFiles\Zulu\zulu-*",
    "$env:ProgramFiles\Amazon Corretto\jdk*"
)) {
    $adaylar += (Get-ChildItem -Path $desen -Directory -ErrorAction SilentlyContinue |
        Sort-Object Name -Descending | ForEach-Object { $_.FullName })
}

$jdk = $null
foreach ($a in $adaylar) {
    if (-not $a) { continue }
    $s = JavaSurumu $a
    if ($s -ge 17) { $jdk = $a; Yaz "JDK $s : $a"; break }
}
if (-not $jdk) {
    Dur "JDK 17 ya da üstü bulunamadı. Android Studio'yu kurun ya da JAVA_HOME'u bir JDK 17+ klasörüne ayarlayın."
}
$env:JAVA_HOME = $jdk
$env:Path = "$jdk\bin;$env:Path"

# ---------------------------------------------------------------- Android SDK
$sdk = $null
foreach ($a in @($env:ANDROID_HOME, $env:ANDROID_SDK_ROOT, "$env:LOCALAPPDATA\Android\Sdk", "$env:ProgramFiles\Android\Sdk")) {
    if (-not $a) { continue }
    if ((Test-Path (Join-Path $a 'platform-tools')) -or (Test-Path (Join-Path $a 'platforms'))) {
        $sdk = $a; break
    }
}
if (-not $sdk) {
    Dur "Android SDK bulunamadı. Android Studio → SDK Manager ile kurun ya da ANDROID_HOME'u SDK klasörüne ayarlayın."
}
$env:ANDROID_HOME = $sdk
Yaz "Android SDK: $sdk"
if (-not (Test-Path (Join-Path $sdk 'licenses\android-sdk-license'))) {
    Yaz "UYARI: SDK lisansları kabul edilmemiş görünüyor. Eksik platform inmezse şunu çalıştırın:"
    Yaz "  `"$sdk\cmdline-tools\latest\bin\sdkmanager.bat`" --licenses"
}
if (-not (Test-Path (Join-Path $sdk 'platforms\android-37'))) {
    Yaz "Not: SDK Platform 37 (Android 17) kurulu değil; lisanslar kabul edildiyse Gradle kendisi indirir."
}

# ---------------------------------------------------------------- Sürüm ve imza
$gradleDosyasi = Get-Content -Raw -Encoding UTF8 (Join-Path $Kok 'app\build.gradle.kts')
$surum = '1.0.0'
if ($gradleDosyasi -match 'versionName\s*=\s*"([^"]+)"') { $surum = $Matches[1] }
Yaz "Sürüm: $surum"

if (-not $Debug) {
    $imzaVar = (Test-Path (Join-Path $Kok 'keystore.properties')) -or ($env:TAA_IMZA_DEPO -and $env:TAA_IMZA_DEPO_SIFRE)
    if (-not $imzaVar) {
        Yaz "UYARI: release imza anahtarı yok (keystore.properties ya da TAA_IMZA_* ortam değişkenleri)."
        Yaz "       APK debug anahtarıyla imzalanacak; mağazalara dağıtmadan önce gerçek anahtarı ayarlayın (README)."
    }
}

# ---------------------------------------------------------------- Derleme
$gorev = if ($Debug) { 'assembleDebug' } else { 'assembleRelease' }
Yaz "gradlew.bat $gorev"
& (Join-Path $Kok 'gradlew.bat') $gorev
if ($LASTEXITCODE -ne 0) { Dur "Gradle derlemesi başarısız (çıkış kodu $LASTEXITCODE)." }

$tur = if ($Debug) { 'debug' } else { 'release' }
$apk = Join-Path $Kok "app\build\outputs\apk\$tur\app-$tur.apk"
if (-not (Test-Path $apk)) {
    $bulunan = Get-ChildItem -Path (Join-Path $Kok "app\build\outputs\apk\$tur") -Filter '*.apk' -ErrorAction SilentlyContinue |
        Select-Object -First 1
    if (-not $bulunan) { Dur "APK bulunamadı: $apk" }
    $apk = $bulunan.FullName
}

# ---------------------------------------------------------------- Kopyalama (silmeden)
$dist = Join-Path $Kok 'dist'
New-Item -ItemType Directory -Force -Path $dist | Out-Null
$ad = if ($Debug) { "TerziTakip-$surum-debug.apk" } else { "TerziTakip-$surum.apk" }
$hedef = Join-Path $dist $ad
if (Test-Path $hedef) {
    $eski = Join-Path $dist 'eski'
    New-Item -ItemType Directory -Force -Path $eski | Out-Null
    $zaman = (Get-Item $hedef).LastWriteTime.ToString('yyyyMMdd-HHmmss')
    $tasinan = Join-Path $eski ([IO.Path]::GetFileNameWithoutExtension($ad) + "-$zaman.apk")
    $n = 1
    while (Test-Path $tasinan) { $tasinan = Join-Path $eski ([IO.Path]::GetFileNameWithoutExtension($ad) + "-$zaman-$n.apk"); $n++ }
    Move-Item -Path $hedef -Destination $tasinan
    Yaz "Önceki APK taşındı: $tasinan"
}
Copy-Item -Path $apk -Destination $hedef
$ozet = (Get-FileHash -Algorithm SHA256 -Path $hedef).Hash.ToLower()

Yaz "Hazır: $hedef"
Yaz "SHA-256: $ozet"
Yaz "Telefona kurmak için (USB hata ayıklama açık):"
Yaz "  `"$sdk\platform-tools\adb.exe`" install -r `"$hedef`""
