// Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17]
// TAA Terzi Takip: mağaza telefonları için yerel WebView kabuğu.
import java.util.Properties

plugins {
    alias(libs.plugins.android.application)
}

// ---- Release imzası: anahtar depoya KONMAZ ----
// Önce ortam değişkenleri, yoksa proje kökündeki keystore.properties (gitignore'da) okunur.
// Hiçbiri yoksa release de debug anahtarıyla imzalanır (yalnız deneme için).
val imzaDosyasi = Properties().apply {
    val f = rootProject.file("keystore.properties")
    if (f.isFile) f.inputStream().use { load(it) }
}

fun imzaDegeri(anahtar: String, ortam: String): String? =
    System.getenv(ortam)?.takeIf { it.isNotBlank() }
        ?: imzaDosyasi.getProperty(anahtar)?.takeIf { it.isNotBlank() }

val depoYolu = imzaDegeri("storeFile", "TAA_IMZA_DEPO")
val depoSifresi = imzaDegeri("storePassword", "TAA_IMZA_DEPO_SIFRE")
val anahtarAdi = imzaDegeri("keyAlias", "TAA_IMZA_ANAHTAR")
val anahtarSifresi = imzaDegeri("keyPassword", "TAA_IMZA_ANAHTAR_SIFRE")
val imzaVar = depoYolu != null && depoSifresi != null && anahtarAdi != null && anahtarSifresi != null &&
    rootProject.file(depoYolu).isFile

android {
    namespace = "com.taa.terzitakip"
    compileSdk = 37

    defaultConfig {
        applicationId = "com.taa.terzitakip"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = "1.0.0"
    }

    signingConfigs {
        if (imzaVar) {
            create("taa") {
                storeFile = rootProject.file(depoYolu!!)
                storePassword = depoSifresi
                keyAlias = anahtarAdi
                keyPassword = anahtarSifresi
            }
        }
    }

    buildTypes {
        release {
            // Küçültme kapalı: derlenmeden denenemeyen R8 kurallarıyla risk alınmasın. Açılırsa proguard-rules.pro hazır.
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = if (imzaVar) signingConfigs.getByName("taa") else signingConfigs.getByName("debug")
        }
    }

    buildFeatures {
        buildConfig = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

if (!imzaVar) {
    logger.lifecycle("TAA: release imza anahtarı bulunamadı (keystore.properties ya da TAA_IMZA_* ortam değişkenleri); release debug anahtarıyla imzalanacak.")
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.core.splashscreen)
    implementation(libs.androidx.activity.ktx)
    implementation(libs.androidx.webkit)
    implementation(libs.androidx.camera.camera2)
    implementation(libs.androidx.camera.lifecycle)
    implementation(libs.androidx.camera.view)
    implementation(libs.androidx.camera.mlkit.vision)
    implementation(libs.mlkit.barcode.scanning)
}
