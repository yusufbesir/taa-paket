// Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17]
package com.taa.terzitakip

import android.content.Context
import androidx.core.content.edit

/** Uygulama ayarları (SharedPreferences). Yalnız sunucu kökü tutulur; eşleştirme belirteci sayfanın localStorage'ındadır. */
class Ayarlar(baglam: Context) {
    private val tercihler = baglam.applicationContext.getSharedPreferences(DOSYA, Context.MODE_PRIVATE)

    var sunucuKoku: String?
        get() = tercihler.getString(ANAHTAR_KOK, null)
        set(deger) {
            tercihler.edit {
                if (deger == null) remove(ANAHTAR_KOK) else putString(ANAHTAR_KOK, deger)
            }
        }

    private companion object {
        const val DOSYA = "taa_ayarlar"
        const val ANAHTAR_KOK = "sunucu_koku"
    }
}
