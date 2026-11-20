// Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17]
package com.taa.terzitakip

import java.net.URI
import java.net.URISyntaxException
import java.net.URLDecoder

/**
 * Mağaza sunucusu adresinin ayrıştırılması ve denetimi. Android'e bağlı değildir (JVM'de sınanabilir).
 *
 * Kök: `şema://ana-bilgisayar[:kapı]` (yol yok, küçük harf, varsayılan kapı yazılmaz).
 * Kural: düz http yalnız yerel ağ IP'lerinde (10/8, 172.16/12, 192.168/16, 100.64/10); geri kalan her şey https.
 * Eşleştirme karekodu `<kök>/telefon?kod=123456` biçimindedir (taa-fis-sunucu yonetici.rs); kod oradan alınır.
 */
object SunucuAdresi {

    data class Sonuc(val kok: String, val kod: String?)

    class Hata(mesaj: String) : Exception(mesaj)

    private val KOD = Regex("^\\d{6}$")

    /** Kullanıcının yazdığı ya da karekoddan okunan adresi çözer; uygun değilse [Hata] fırlatır. */
    fun coz(girdi: String): Sonuc {
        var metin = girdi.trim()
        if (metin.isEmpty()) throw Hata("Sunucu adresini yazın ya da karekodu okutun.")
        if (!metin.contains("://")) {
            val ana = metin.substringBefore('/').substringBefore('?').substringBefore(':')
            metin = (if (yerelIpMi(ana)) "http://" else "https://") + metin
        }
        val uri = try {
            URI(metin)
        } catch (e: URISyntaxException) {
            throw Hata("Adres okunamadı: $girdi")
        }
        val sema = uri.scheme?.lowercase()
        if (sema != "http" && sema != "https") throw Hata("Adres http:// ya da https:// ile başlamalı.")
        if (uri.rawUserInfo != null) throw Hata("Adreste kullanıcı adı ya da şifre olamaz.")
        val ana = uri.host?.lowercase()?.trimEnd('.')
        if (ana.isNullOrEmpty()) throw Hata("Adreste sunucu adı yok.")
        if (sema == "http" && !yerelIpMi(ana)) {
            throw Hata(
                "http yalnız yerel ağ IP adresleriyle kullanılabilir (10.x, 172.16–31.x, 192.168.x, 100.64–127.x). " +
                    "Diğer adresler için https yazın."
            )
        }
        val kapi = uri.port
        if (kapi != -1 && kapi !in 1..65535) throw Hata("Kapı numarası geçersiz: $kapi")
        val kod = sorguDegeri(uri.rawQuery, "kod")?.takeIf { KOD.matches(it) }
        return Sonuc(kokYaz(sema, ana, kapi), kod)
    }

    /** Bir adresin kökenini (kök biçiminde) döndürür; http/https değilse ya da okunamazsa null. */
    fun koken(adres: String?): String? {
        if (adres.isNullOrEmpty()) return null
        return try {
            val uri = URI(adres)
            val sema = uri.scheme?.lowercase() ?: return null
            if (sema != "http" && sema != "https") return null
            val ana = uri.host?.lowercase()?.trimEnd('.')
            if (ana.isNullOrEmpty()) return null
            kokYaz(sema, ana, uri.port)
        } catch (e: URISyntaxException) {
            null
        }
    }

    /** Adres kayıtlı kökün içinde mi (aynı şema, ana bilgisayar ve kapı)? */
    fun kokIcindeMi(adres: String?, kok: String?): Boolean = kok != null && koken(adres) == kok

    /** IPv4 yazımı ve 10/8, 172.16/12, 192.168/16, 100.64/10 aralıklarından biri mi? */
    fun yerelIpMi(ana: String?): Boolean {
        if (ana == null) return false
        val parcalar = ana.split('.')
        if (parcalar.size != 4) return false
        val s = IntArray(4)
        for (i in 0 until 4) {
            val p = parcalar[i]
            if (p.isEmpty() || p.length > 3 || !p.all { it in '0'..'9' }) return false
            s[i] = p.toInt()
            if (s[i] > 255) return false
        }
        val a = s[0]
        val b = s[1]
        return a == 10 ||
            (a == 172 && b in 16..31) ||
            (a == 192 && b == 168) ||
            (a == 100 && b in 64..127)
    }

    private fun kokYaz(sema: String, ana: String, kapi: Int): String {
        val varsayilan = if (sema == "http") 80 else 443
        return if (kapi == -1 || kapi == varsayilan) "$sema://$ana" else "$sema://$ana:$kapi"
    }

    private fun sorguDegeri(sorgu: String?, ad: String): String? {
        if (sorgu.isNullOrEmpty()) return null
        for (parca in sorgu.split('&')) {
            val esit = parca.indexOf('=')
            if (esit <= 0) continue
            if (parca.substring(0, esit) == ad) {
                return try {
                    URLDecoder.decode(parca.substring(esit + 1), "UTF-8")
                } catch (e: IllegalArgumentException) {
                    null
                }
            }
        }
        return null
    }
}
