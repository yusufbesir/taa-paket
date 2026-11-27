// Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17]
// değiştiren: [Claude Fable 5.2 #taa-web-W2-kurulum 2026-11-27] (yollar, günlük)
package com.taa.terzitakip

import android.webkit.JavascriptInterface

/**
 * Yedek JS köprüsü (window.TaaAndroid), yalnız WebView'da WEB_MESSAGE_LISTENER / DOCUMENT_START_SCRIPT yoksa
 * kullanılır. Yöntemler JavaBridge iş parçacığında çağrılır; işi ana iş parçacığına aktarır. Köken denetimi
 * [AnaEtkinlik.koprudenIstek] içinde yapılır (yalnız yol listesindeki kökenlerde açık sayfa).
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

    /** Yol listesi (JSON metni): `[{"ad":"merkez","url":…},{"ad":"sube","url":…},{"ad":"yerel"}]`. */
    @JavascriptInterface
    fun yollar(): String = etkinlik.yollarJson()

    /** Cihaz günlüğüne satır ekler; [ayrinti] JSON nesnesi metni ya da boş. */
    @JavascriptInterface
    fun gunlukYaz(olay: String?, ayrinti: String?) {
        etkinlik.runOnUiThread { etkinlik.koprudenIstek(AnaEtkinlik.ISTEK_GUNLUK_YAZ, olay, ayrinti) }
    }

    /** Bir günün günlük satırlarını ister ([gun] boşsa bugün); yanıt `taa-gunluk` olayıyla döner. */
    @JavascriptInterface
    fun gunlukOku(gun: String?) {
        etkinlik.runOnUiThread { etkinlik.koprudenIstek(AnaEtkinlik.ISTEK_GUNLUK_OKU, gun, null) }
    }
}
