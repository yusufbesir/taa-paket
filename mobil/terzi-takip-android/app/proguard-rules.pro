# Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17]
# Küçültme şu an kapalı (app/build.gradle.kts). Açılırsa JS köprüsünün yedek yolu korunmalı:
-keepclassmembers class com.taa.terzitakip.TaaKoprusu {
    @android.webkit.JavascriptInterface <methods>;
}
-keepattributes JavascriptInterface
