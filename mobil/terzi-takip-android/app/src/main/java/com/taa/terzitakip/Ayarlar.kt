// Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17]
// değiştiren: [Claude Fable 5.2 #taa-web-W2-kurulum 2026-11-27] (yol listesi: merkez + şube bilgisayarı)
package com.taa.terzitakip

import android.content.Context
import androidx.core.content.edit

/**
 * Uygulama ayarları (SharedPreferences). Yol listesinin adresleri tutulur: merkez sunucu ve şube bilgisayarı
 * (sıra sabit: merkez → şube → yerel, bkz. [YolListesi]). Oturum ve giden kutusu sayfanın kendi deposundadır.
 *
 * [sunucuKoku] eski tek adresli ayardır (mağaza bilgisayarı). Silinmez; yeni adresler yazılmamışsa Sunucu ayarı
 * ekranında şube bilgisayarı alanına öneri olarak gelir.
 */
class Ayarlar(baglam: Context) {
    private val tercihler = baglam.applicationContext.getSharedPreferences(DOSYA, Context.MODE_PRIVATE)

    private fun oku(anahtar: String): String? = tercihler.getString(anahtar, null)?.takeIf { it.isNotBlank() }

    private fun yaz(anahtar: String, deger: String?) {
        tercihler.edit {
            if (deger.isNullOrBlank()) remove(anahtar) else putString(anahtar, deger)
        }
    }

    /** Merkez sunucunun kökü, örn. `https://sunucu.ornek.com`. */
    var merkezKoku: String?
        get() = oku(ANAHTAR_MERKEZ)
        set(deger) = yaz(ANAHTAR_MERKEZ, deger)

    /** Şube bilgisayarının (taa-sube-dugum) kökü, örn. `http://192.168.1.20:8788`. */
    var subeKoku: String?
        get() = oku(ANAHTAR_SUBE)
        set(deger) = yaz(ANAHTAR_SUBE, deger)

    /** Eski tek adresli ayar (yalnız okunur; yeni kurulumda yazılmaz). */
    val sunucuKoku: String?
        get() = oku(ANAHTAR_KOK)

    /** Yol listesi: merkez → şube → yerel. */
    fun yollar(): List<YolListesi.Yol> = YolListesi.olustur(merkezKoku, subeKoku)

    private companion object {
        const val DOSYA = "taa_ayarlar"
        const val ANAHTAR_KOK = "sunucu_koku"
        const val ANAHTAR_MERKEZ = "merkez_koku"
        const val ANAHTAR_SUBE = "sube_koku"
    }
}
