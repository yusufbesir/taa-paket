// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-kurulum 2026-11-27]
package com.taa.terzitakip

import java.io.File
import java.io.FileOutputStream
import java.io.IOException
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale
import java.util.TimeZone

/**
 * Cihaz günlüğü (PLAN-TAA-WEB-2026-11-27 §9: "günlükler cihazda ve şube bilgisayarında tutulur, silinmez").
 *
 * Dosya: `<klasör>/YYYY-MM-DD.jsonl` (UTC gün), satır başına bir JSON nesnesi; şube bilgisayarındaki günlükle aynı ad
 * kuralı. Yalnız ekleme: bu sınıf hiçbir satırı ya da dosyayı silmez, ezmez. Merkeze kopyalama kabuğun işidir
 * ([oku] ile satırları alır, kendi giden kutusundan gönderir); kopyalanınca da cihazdaki dosya yerinde kalır.
 * Android'e bağlı değildir (JVM'de sınanır: sinama/YolSinamasi.kt).
 */
class Gunluk(private val klasor: File) {

    /**
     * Bir satır ekler. [olay] `[a-z0-9._-]{1,64}`; [ayrintiJson] geçerli tek satır JSON olmalıdır (çağıran doğrular);
     * uymayan girdi yazılmaz. Dönüş: yazıldı mı. Hata fırlatmaz (günlük yazılamadı diye uygulama durmaz).
     */
    @Synchronized
    fun yaz(olay: String, ayrintiJson: String? = null, kaynak: String = KAYNAK_UYGULAMA, simdi: Long = System.currentTimeMillis()): Boolean {
        val satir = satir(simdi, kaynak, olay, ayrintiJson) ?: return false
        return try {
            if (!klasor.isDirectory && !klasor.mkdirs() && !klasor.isDirectory) return false
            FileOutputStream(File(klasor, gun(simdi) + UZANTI), true).use {
                it.write((satir + "\n").toByteArray(Charsets.UTF_8))
            }
            true
        } catch (e: IOException) {
            false
        }
    }

    /** Günlüğü olan günler (eskiden yeniye). */
    fun gunler(): List<String> =
        (klasor.list() ?: emptyArray())
            .filter { it.endsWith(UZANTI) && gunGecerli(it.removeSuffix(UZANTI)) }
            .map { it.removeSuffix(UZANTI) }
            .sorted()

    /** Bir günün satırları; [enCok] verilirse yalnız son o kadar satır. Gün biçimi bozuksa ya da dosya yoksa boş liste. */
    fun oku(gun: String, enCok: Int = OKUMA_EN_COK): List<String> {
        if (!gunGecerli(gun)) return emptyList()
        val dosya = File(klasor, gun + UZANTI)
        if (!dosya.isFile) return emptyList()
        return try {
            val satirlar = dosya.readLines(Charsets.UTF_8).filter { it.isNotBlank() }
            if (enCok > 0 && satirlar.size > enCok) satirlar.subList(satirlar.size - enCok, satirlar.size) else satirlar
        } catch (e: IOException) {
            emptyList()
        }
    }

    companion object {
        const val KAYNAK_UYGULAMA = "android"
        const val KAYNAK_SAYFA = "kabuk"
        const val UZANTI = ".jsonl"
        const val AYRINTI_EN_COK = 4096
        const val OKUMA_EN_COK = 200
        private val OLAY = Regex("^[a-z0-9._-]{1,64}$")
        private val GUN = Regex("^\\d{4}-\\d{2}-\\d{2}$")

        fun gunGecerli(g: String): Boolean = GUN.matches(g)

        fun olayGecerli(o: String): Boolean = OLAY.matches(o)

        private fun bicim(kalip: String): SimpleDateFormat =
            SimpleDateFormat(kalip, Locale.ROOT).apply { timeZone = TimeZone.getTimeZone("UTC") }

        /** UTC gün: `YYYY-MM-DD`. */
        fun gun(ms: Long): String = bicim("yyyy-MM-dd").format(Date(ms))

        /** UTC RFC 3339 zaman. */
        fun zaman(ms: Long): String = bicim("yyyy-MM-dd'T'HH:mm:ss'Z'").format(Date(ms))

        /** Günlük satırı; girdi uygun değilse null. */
        fun satir(ms: Long, kaynak: String, olay: String, ayrintiJson: String?): String? {
            if (!olayGecerli(olay) || !olayGecerli(kaynak)) return null
            val b = StringBuilder(96)
            b.append("{\"zaman\":\"").append(zaman(ms)).append("\",\"kaynak\":\"").append(kaynak)
                .append("\",\"olay\":\"").append(olay).append('"')
            if (ayrintiJson != null) {
                val a = ayrintiJson.trim()
                if (a.isEmpty() || a.length > AYRINTI_EN_COK || a.any { it == '\n' || it == '\r' }) return null
                b.append(",\"ayrinti\":").append(a)
            }
            return b.append('}').toString()
        }
    }
}
