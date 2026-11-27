// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-kurulum 2026-11-27]
package com.taa.terzitakip

/**
 * İstemci yol listesi (PLAN-TAA-WEB-2026-11-27 §9). Sıra sabittir: merkez → şube bilgisayarı → yerel (cihazın kendi
 * kutusu). Çevrimiçiyken cihaz doğrudan merkeze gönderir; merkeze ulaşamazsa şube bilgisayarına, ona da ulaşamazsa
 * kayıt cihazda bekler. Android'e bağlı değildir (JVM'de sınanır: sinama/YolSinamasi.kt).
 *
 * Kabuğa verilen biçim (sözleşme): `[{"ad":"merkez","url":"https://…"},{"ad":"sube","url":"http://…:8788"},{"ad":"yerel"}]`.
 * Adresi yazılmamış yol listeye girmez; `yerel` her zaman sondadır.
 */
object YolListesi {

    const val MERKEZ = "merkez"
    const val SUBE = "sube"
    const val YEREL = "yerel"

    /** TAA Web kabuğunun sunucudaki yolu (docs/WEB-ESITLE-SOZLESMESI.md §2). */
    const val KABUK_YOLU = "/kabuk/"

    /** Kabukta açılan modül (rota `#/<ad>`, web/kabuk/moduller.json). */
    const val MODUL = "terzitakip"

    data class Yol(val ad: String, val url: String?)

    /** Kayıtlı köklerden yol listesi. Kökler [SunucuAdresi.coz] ile doğrulanmış olmalıdır. */
    fun olustur(merkez: String?, sube: String?): List<Yol> {
        val liste = ArrayList<Yol>(3)
        if (!merkez.isNullOrBlank()) liste.add(Yol(MERKEZ, merkez))
        if (!sube.isNullOrBlank()) liste.add(Yol(SUBE, sube))
        liste.add(Yol(YEREL, null))
        return liste
    }

    /** Kabuğun açılabileceği kökler, deneme sırasıyla (merkez, sonra şube). `yerel`in adresi yoktur. */
    fun kabukKokleri(yollar: List<Yol>): List<String> = yollar.mapNotNull { it.url }

    /** Gezintiye, JS köprüsüne ve kameraya izinli kökenler: listedeki adresli yollar. */
    fun izinliKokler(yollar: List<Yol>): Set<String> = kabukKokleri(yollar).toSet()

    /** Kabuğun adresi: `<kök>/kabuk/[?kod=123456]#/terzitakip`. Kod yalnız 6 rakamsa eklenir. */
    fun kabukAdresi(kok: String, kod: String?): String {
        val sorgu = if (kod != null && kod.length == 6 && kod.all { it in '0'..'9' }) "?kod=$kod" else ""
        return "$kok$KABUK_YOLU$sorgu#/$MODUL"
    }

    /**
     * Düz http isteğine izin var mı: yalnız listedeki bir yolun kökeninde ve yerel ağ IP'sinde (şube bilgisayarı).
     * Merkez kabuğu (https) şube bilgisayarına (http, yerel ağ) yazabilsin diye WebView'da karışık içerik açılır;
     * bu kural onu kayıtlı şube adresiyle sınırlar.
     */
    fun httpIzinli(adres: String?, yollar: List<Yol>): Boolean {
        val koken = SunucuAdresi.koken(adres) ?: return false
        if (!koken.startsWith("http://")) return false
        if (koken !in izinliKokler(yollar)) return false
        return SunucuAdresi.yerelIpMi(koken.removePrefix("http://").substringBefore(':'))
    }

    /** Karışık içerik (https sayfadan http isteği) gerekir mi: https bir yol ile http bir yol birlikte varsa. */
    fun karisikIcerikGerekli(yollar: List<Yol>): Boolean {
        val kokler = kabukKokleri(yollar)
        return kokler.any { it.startsWith("https://") } && kokler.any { it.startsWith("http://") }
    }

    /** Karekoddan ya da elle gelen kök hangi alana yazılır: yerel ağ IP'si şube bilgisayarıdır, gerisi merkez. */
    fun alan(kok: String): String {
        val ana = kok.substringAfter("://").substringBefore(':')
        return if (SunucuAdresi.yerelIpMi(ana)) SUBE else MERKEZ
    }

    /** Kabuğa verilen JSON metni (sözleşme biçimi). */
    fun json(yollar: List<Yol>): String =
        yollar.joinToString(",", "[", "]") { y ->
            if (y.url == null) "{\"ad\":${tirnakla(y.ad)}}" else "{\"ad\":${tirnakla(y.ad)},\"url\":${tirnakla(y.url)}}"
        }

    /** JSON metin değeri (kaçışlı, tırnaklı). `<` kaçışlanır: metin `<script>` içine gömülse de etiketi kapatamaz. */
    fun tirnakla(s: String): String {
        val b = StringBuilder(s.length + 2).append('"')
        for (c in s) {
            when {
                c == '"' -> b.append("\\\"")
                c == '\\' -> b.append("\\\\")
                c == '\n' -> b.append("\\n")
                c == '\r' -> b.append("\\r")
                c == '\t' -> b.append("\\t")
                c == '<' || c == '>' || c == '&' || c < ' ' || c == ' ' || c == ' ' ->
                    b.append("\\u").append(String.format("%04x", c.code))
                else -> b.append(c)
            }
        }
        return b.append('"').toString()
    }
}
