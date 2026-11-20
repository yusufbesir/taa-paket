// Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17]
package com.taa.terzitakip

import android.webkit.JavascriptInterface

/**
 * Yedek JS köprüsü (window.TaaAndroid), yalnız WebView'da WEB_MESSAGE_LISTENER / DOCUMENT_START_SCRIPT yoksa
 * kullanılır. Yöntemler JavaBridge iş parçacığında çağrılır; işi ana iş parçacığına aktarır. Köken denetimi
 * [AnaEtkinlik.koprudenIstek] içinde yapılır (yalnız kayıtlı sunucu kökündeki sayfa).
 */
class TaaKoprusu(private val etkinlik: AnaEtkinlik) {

    @JavascriptInterface
    fun karekodOku(): Boolean {
        etkinlik.runOnUiThread { etkinlik.koprudenIstek(AnaEtkinlik.ISTEK_KAREKOD) }
        return true
    }

    @JavascriptInterface
    fun ayarlar() {
        etkinlik.runOnUiThread { etkinlik.koprudenIstek(AnaEtkinlik.ISTEK_AYARLAR) }
    }

    @JavascriptInterface
    fun surum(): String = BuildConfig.VERSION_NAME
}
