# Terzi Takip Android uygulaması (APK)

<!-- Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17] -->

Mağaza telefonlarına kurulan küçük yerel kabuk. Kotlin ile yazıldı. Mağaza bilgisayarındaki **taa-fis-sunucu**'nun
telefon sitesini (`<sunucu>/telefon`, kaynak: `crates/taa-fis-sunucu/src/sayfa/telefon.html`) tam ekran WebView'da
açar. Bunun üstüne telefonun kendi yeteneklerini ekler: karekod okuyucu, kamerayla fotoğraf, markalı açılış ve
"sunucuya ulaşılamıyor" ekranı, sunucu adresinin saklanması.

- Paket: `com.taa.terzitakip`, uygulama adı **Terzi Takip**, sürüm 1.0.0 (versionCode 1)
- minSdk 26 (Android 8.0), targetSdk ve compileSdk 37 (Android 17), yalnız dikey
- Tek modül: `app`. Gradle Kotlin DSL ve sürüm kataloğu (`gradle/libs.versions.toml`) kullanılır.

## Sürümler (2026-11-17'de doğrulandı)

| Bileşen | Sürüm | Not |
|---|---|---|
| Android Gradle Plugin | 9.5.1 | Kotlin desteği yerleşik, ayrıca kotlin-android eklentisi gerekmez |
| Gradle (sarmalayıcı) | 9.6.1 | AGP 9.5'in en düşük sürümü 9.6.0; dağıtım SHA-256 ile sabit |
| JDK | 17 ya da üstü | Android Studio'nun içindeki JBR (21) de olur |
| SDK Build Tools | 36.0.0 | AGP kendisi seçer |
| androidx.core / core-ktx | 1.19.1 | |
| androidx.core:core-splashscreen | 1.2.0 | Android 12 SplashScreen API, Android 8–11'de de aynı görünüm |
| androidx.activity | 1.13.0 | |
| androidx.webkit | 1.18.0 | Kökene bağlı JS köprüsü (WebMessageListener) |
| androidx.camera (camera2, lifecycle, view, mlkit-vision) | 1.6.2 | Karekod okuyucu |
| com.google.mlkit:barcode-scanning | 17.3.0 | **bundled**: model APK'nın içinde, Google Play hizmeti gerekmez. Bu sürüm buradan doğrulanamadı (bkz. Riskler) |

appcompat kullanılmaz; ekranlar platform temasıyla çizilir.

## Windows'ta derleme

### 1. Bir kez yapılacak kurulum

1. **Android Studio**'yu kurun (https://developer.android.com/studio). İçinde JDK (JBR) ve Android SDK gelir.
   Yalnız komut satırı araçları da yeter: JDK 17+ (ör. Eclipse Temurin 17/21) ve "Android SDK Command-line Tools".
2. SDK Manager'dan **Android 17 (API 37) SDK Platform** ve **Android SDK Build-Tools 36** kurun. Lisansları kabul edin:
   ```bat
   "%LOCALAPPDATA%\Android\Sdk\cmdline-tools\latest\bin\sdkmanager.bat" --licenses
   ```
   Lisanslar kabul edildiyse eksik platform ve build-tools ilk derlemede kendiliğinden iner.
3. Ortam değişkenleri (Sistem → Gelişmiş sistem ayarları → Ortam Değişkenleri):
   - `ANDROID_HOME` = `%LOCALAPPDATA%\Android\Sdk` (Android Studio'nun varsayılan yeri)
   - `JAVA_HOME` = JDK klasörü, ör. `C:\Program Files\Android\Android Studio\jbr`

   `derle.ps1` bu ikisini tanımlı değilse varsayılan yerlerde kendisi arar.

### 2. Derleme

En kolayı betik (PowerShell):

```powershell
cd mobil\terzi-takip-android
powershell -ExecutionPolicy Bypass -File .\derle.ps1           # release
powershell -ExecutionPolicy Bypass -File .\derle.ps1 -Debug    # debug
```

Betik JDK'yı ve SDK'yı bulur, Gradle'ı çalıştırır, APK'yı `dist\TerziTakip-1.0.0.apk` (debug için
`dist\TerziTakip-1.0.0-debug.apk`) adıyla kopyalar ve SHA-256 özetini yazar. Hiçbir şey silmez: `dist\` içinde aynı adlı
eski APK varsa önce `dist\eski\` altına tarihli adla taşınır.

Elle derlemek için:

```bat
cd mobil\terzi-takip-android
gradlew.bat assembleDebug
gradlew.bat assembleRelease
```

Çıktılar:
- debug: `app\build\outputs\apk\debug\app-debug.apk`
- release: `app\build\outputs\apk\release\app-release.apk`

İlk derleme Gradle 9.6.1'i ve bağımlılıkları indirir (internet gerekir; `services.gradle.org`, `dl.google.com`,
`repo.maven.apache.org`).

### 3. Release imzası

İmza anahtarı ve şifreleri **depoya konmaz**. Anahtarı bir kez üretin ve depo dışında saklayın:

```bat
keytool -genkeypair -v -keystore C:\TAA\gizli\terzi-takip.jks -alias terzitakip -keyalg RSA -keysize 4096 -validity 10000
```

Sonra iki yoldan biri:

- `keystore.properties.ornek` dosyasını `keystore.properties` adıyla kopyalayıp doldurun (bu dosya `.gitignore`'da), ya da
- ortam değişkenleri: `TAA_IMZA_DEPO` (jks yolu), `TAA_IMZA_DEPO_SIFRE`, `TAA_IMZA_ANAHTAR` (alias), `TAA_IMZA_ANAHTAR_SIFRE`.
  Ortam değişkeni varsa dosyadaki değerin önüne geçer.

Hiçbiri yoksa release **debug anahtarıyla** imzalanır ve Gradle bunu uyarıyla yazar. Debug anahtarıyla kurulmuş
uygulamanın üstüne gerçek anahtarla imzalı APK kurulamaz (önce kaldırmak gerekir; kaldırınca telefonun eşleştirmesi ve
gönderilmemiş taslakları da gider). Mağazalara dağıtmadan önce gerçek anahtarı ayarlayın ve hep aynı anahtarı kullanın.
Anahtar kaybolursa güncelleme yapılamaz; yedekleyin.

## Telefona kurulum

USB ile (telefonda Geliştirici seçenekleri → USB hata ayıklama açık):

```bat
"%ANDROID_HOME%\platform-tools\adb.exe" devices
"%ANDROID_HOME%\platform-tools\adb.exe" install -r dist\TerziTakip-1.0.0.apk
```

`-r` var olan kurulumu verisini koruyarak günceller (eşleştirme ve taslaklar kalır). USB yoksa APK'yı telefona
kopyalayıp dosya yöneticisinden açın ("Bilinmeyen uygulamaları yükle" izni istenir).

## İlk açılış ve eşleştirme

1. Mağaza bilgisayarında Terzi Takip → **Telefonlar** ekranında eşleştirme karekodunu açın. Karekod
   `http://<mağaza-pc-ip>:8787/telefon?kod=123456` biçimindedir (Tailscale kullanılıyorsa `https://<mağaza-pc>.<ağ>.ts.net/...`).
2. Uygulama ilk açılışta **Mağaza sunucusu** ekranını gösterir. **Karekodu oku**'ya basıp karekodu okutun: sunucu adresi
   ve eşleştirme kodu karekoddan alınır, kaydedilir ve telefon sitesi kod dolu olarak açılır. Telefona bir ad verip
   **Eşleştir**'e basın.
3. Karekod yoksa adresi elle yazın: `192.168.1.20:8787` (şemasız yerel IP için `http://` eklenir) ya da
   `magaza-pc.ornek.ts.net` (geri kalan her şey için `https://` eklenir). Kod alanı boş bırakılabilir; kod sitede de yazılır.

Adres telefonda (SharedPreferences) saklanır. Eşleştirme belirteci ve çevrimdışı taslak kuyruğu telefon sitesinin
kendi `localStorage`'ındadır; uygulama yedeğe ve cihaz aktarımına dahil edilmez.

### Sunucu adresini değiştirme (yönetici)

Gizli menü: sayfanın üstündeki **Terzi Takip** başlığına (sol taraf) **uzun basın** → onay → **Sunucu ayarı**.
"Mağaza sunucusuna ulaşılamıyor" ekranında da başlığa uzun basmak aynı menüyü açar. Adres değişince telefon yeni
sunucuyla yeniden eşleştirilir; eski sunucuya gönderilmemiş taslaklar eski adresin kaydında kalır (adres geri
alınınca yine görünür).

## Davranış ve güvenlik

- **Gezinti** yalnız kayıtlı sunucu kökünde (aynı şema, ana bilgisayar, kapı). Başka adresler dış tarayıcıda açılır;
  `tel:`, `mailto:` gibi bağlantılar ilgili uygulamaya gider.
- **WebView**: JavaScript ve DOM storage açık; yerel dosya erişimi kapalı (`allowFileAccess=false`); karışık içerik
  engelli (`MIXED_CONTENT_NEVER_ALLOW`); konum kapalı; yeni pencere açılmaz. `content://` erişimi açık bırakıldı, çünkü
  dosya seçiciden dönen fotoğraflar bu yolla gelir.
- **Düz http yalnız yerel ağ IP'lerinde**: 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16 ve 100.64.0.0/10 (Tailscale).
  `network_security_config` IP aralığı tanımlayamadığı için kural kodda uygulanır: kurulum ekranı başka http adresini
  kaydetmez ve WebView bu aralıklar dışındaki her http isteğini 403 ile keser. `*.ts.net` için http tamamen kapalıdır,
  yalnız https. Ana bilgisayar adıyla http (ör. `http://magaza-pc:8787`) kabul edilmez: IP ya da https kullanın.
- **Sertifika**: sertifika hataları asla geçilmez. Kendi imzalı sertifika gerekirse kullanıcının kurduğu CA'ya güven
  **yalnız debug** derlemede açıktır (`debug-overrides`). Release derlemede geçerli sertifika gerekir (Tailscale
  `tailscale serve`/`cert` ile `*.ts.net` sertifikası verir).
- **JS köprüsü** `window.TaaAndroid` yalnız kayıtlı kökteki sayfada tanımlanır (androidx.webkit belge başı betiği +
  WebMessageListener, kökene bağlı). Çok eski WebView sürümlerinde yedek olarak `addJavascriptInterface` kullanılır ve
  her çağrıda sayfanın kökü denetlenir.
  - `TaaAndroid.karekodOku()`: yerel okuyucuyu açar. Sonuç `window` üzerinde `taa-karekod` olayıyla döner:
    `event.detail = {deger: "..."}` ya da `{iptal: true, hata?: "..."}`.
  - `TaaAndroid.ayarlar()`: Sunucu ayarı onayını açar.
  - `TaaAndroid.surum()`: `"1.0.0"`.
- **Ağ yokken**: sayfa yüklendikten sonra ağ giderse telefon sitesi kendi kuyruğunu kullanır (fiş telefonda bekler,
  ağ gelince gönderilir). Uygulama açılırken sunucuya ulaşılamazsa (ya da sunucu hata dönerse) markalı **Mağaza
  sunucusuna ulaşılamıyor / Yeniden dene** ekranı çıkar.
- **Fotoğraf**: sayfadaki dosya seçici (`<input type=file accept=image/* capture>`) doğrudan kamerayı açar; `capture`
  olmayan seçicide galeri ve kamera birlikte sunulur. Çekilen fotoğraf uygulama önbelleğinde geçici tutulur, bir saat
  sonra temizlenir.
- **Geri tuşu** WebView geçmişinde geri gider; geçmiş bitince uygulamadan çıkar. Ekranı açık tutma seçeneği yoktur.
- **Açılış**: Android 12 SplashScreen API, beyaz zemin üstünde Yaka logosu. Simge uyarlanabilir (ön plan Yaka, zemin
  beyaz) ve Android 13+ temalı simge için tek renkli sürüm içerir.

## telefon.html ile ilişki

`telefon.html`'e küçük bir ek yapıldı (uygulama yokken davranış aynı):
- `window.TaaAndroid` varsa **Kod okut** ekranı tarayıcının BarcodeDetector/kamera yolu yerine yerel okuyucuyu açar;
  okunan değer eskisi gibi `noCoz` → `fisAc` yolundan geçer.
- `window.TaaAndroid` varsa **Telefonu eşleştir** ekranında **Karekodu oku** düğmesi çıkar; karekoddaki `kod=` alanı
  kod kutusuna yazılır. Karekod başka bir sunucuya aitse uyarı verilir.

## Klasör yapısı

```
mobil/terzi-takip-android/
  settings.gradle.kts, build.gradle.kts, gradle.properties, gradle/libs.versions.toml
  gradlew, gradlew.bat, gradle/wrapper/          Gradle 9.6.1 sarmalayıcısı (jar resmi özetle aynı)
  derle.ps1                                      Windows derleme betiği
  keystore.properties.ornek                      imza ayarı örneği (gerçeği depoya girmez)
  app/build.gradle.kts, app/proguard-rules.pro
  app/src/main/AndroidManifest.xml
  app/src/main/java/com/taa/terzitakip/
    AnaEtkinlik.kt        WebView, hata ekranı, dosya seçici, JS köprüsü, gizli menü
    KurulumEtkinligi.kt   ilk kurulum / Sunucu ayarı
    KarekodEtkinligi.kt   CameraX + ML Kit karekod okuyucu
    SunucuAdresi.kt       adres ayrıştırma ve http kuralı (Android'e bağlı değil)
    Ayarlar.kt            SharedPreferences
    TaaKoprusu.kt         yedek JS köprüsü
    Yardimci.kt           kenardan kenara çizimde sistem çubuğu boşlukları
  app/src/main/res/       Koçan teması, Yaka vektörleri, ağ güvenliği, dosya yolları
```

## Riskler ve denenmemiş noktalar

Bu proje Android SDK'sı indirilemeyen bir ortamda yazıldı; **henüz hiç derlenmedi ve telefonda denenmedi.**
Yapılan denetimler: `SunucuAdresi.kt` JVM'de birim sınamalarıyla çalıştırıldı; tüm Kotlin dosyaları Android 16
çerçeve sınıflarına ve androidx/ML Kit için elle yazılmış imza taslaklarına karşı hatasız derlendi; telefon.html
betiği sözdizimi denetiminden ve taa-fis-sunucu sınamalarından geçti. İlk Windows derlemesinde bakılacaklar:

- ML Kit `barcode-scanning:17.3.0` sürümü Google Maven'a erişilemediği için doğrulanamadı. Daha yeni bir sürüm
  varsa `gradle/libs.versions.toml` içinde güncelleyin.
- androidx imzaları gerçek kütüphaneye karşı derlenmedi. Bir uyuşmazlık çıkarsa küçük bir düzeltme gerekebilir.
- WebView'ın `network_security_config` (http izni, debug'da kullanıcı CA'sı) kurallarına uyması WebView sürümüne
  bağlıdır. Kendi imzalı sertifikayı debug derlemede deneyin.
- Android 17'yi hedefleyen uygulamalarda büyük ekranlarda (tablet, açık katlanabilir) dikey kilit sistemce yok sayılır.
  Telefonlarda dikey kalır.
- Kamera uygulaması `EXTRA_OUTPUT`'u yok sayıp fotoğrafı yalnız küçük resim olarak dönerse fotoğraf eklenmez. Bu
  durumda galeriden seçin ya da başka bir kamera uygulaması deneyin.
- Başlığa uzun basma bölgesi (üst 56 dp, sol %60) telefon.html başlığına göre ayarlandı. Başlık yüksekliği değişirse
  `AnaEtkinlik.BASLIK_DP` değerine bakın.
