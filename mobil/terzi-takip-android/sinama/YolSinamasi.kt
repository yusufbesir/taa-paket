// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-kurulum 2026-11-27]
// Android'e bağlı olmayan sınıfların (SunucuAdresi, YolListesi, Gunluk) JVM sınaması. Gradle projesine dahil
// DEĞİLDİR (JUnit bağımlılığı eklenmedi); `sinama/calistir.sh` yalnız Kotlin derleyicisiyle derleyip çalıştırır.
import com.taa.terzitakip.Gunluk
import com.taa.terzitakip.SunucuAdresi
import com.taa.terzitakip.YolListesi
import java.io.File
import kotlin.system.exitProcess

private var gecen = 0
private val kalan = ArrayList<String>()

private fun sina(ad: String, govde: () -> Unit) {
    try {
        govde()
        gecen++
        println("  tamam  $ad")
    } catch (e: Throwable) {
        kalan.add(ad)
        println("  KALDI  $ad: ${e.message}")
    }
}

private fun <T> esit(beklenen: T, gelen: T, not: String = "") {
    if (beklenen != gelen) throw AssertionError("beklenen <$beklenen>, gelen <$gelen> $not")
}

private fun dogru(kosul: Boolean, not: String) {
    if (!kosul) throw AssertionError(not)
}

fun main() {
    val merkez = "https://sunucu.ornek.com"
    val sube = "http://192.168.1.20:8788"

    sina("sıra her zaman merkez, şube, yerel") {
        val y = YolListesi.olustur(merkez, sube)
        esit(listOf("merkez", "sube", "yerel"), y.map { it.ad })
        esit(listOf(merkez, sube, null), y.map { it.url })
        esit(listOf("sube", "yerel"), YolListesi.olustur(null, sube).map { it.ad })
        esit(listOf("merkez", "yerel"), YolListesi.olustur(merkez, " ").map { it.ad })
        esit(listOf("yerel"), YolListesi.olustur(null, null).map { it.ad })
    }

    sina("kabuğa verilen JSON sözleşme biçiminde") {
        esit(
            "[{\"ad\":\"merkez\",\"url\":\"https://sunucu.ornek.com\"},{\"ad\":\"sube\",\"url\":\"http://192.168.1.20:8788\"},{\"ad\":\"yerel\"}]",
            YolListesi.json(YolListesi.olustur(merkez, sube)),
        )
        esit("[{\"ad\":\"yerel\"}]", YolListesi.json(YolListesi.olustur(null, null)))
    }

    sina("JSON kaçışı betiğe gömülmeye uygun") {
        esit("\"a\\\"b\\\\c\"", YolListesi.tirnakla("a\"b\\c"))
        val t = YolListesi.tirnakla("</script><b>&\n\u2028")
        dogru(!t.contains('<') && !t.contains('>') && !t.contains('&') && !t.contains('\n') && !t.contains('\u2028'), t)
        esit("\"\\u003c/script\\u003e\\u003cb\\u003e\\u0026\\n\\u2028\"", t)
    }

    sina("kabuk önce merkezden, sonra şube bilgisayarından denenir") {
        esit(listOf(merkez, sube), YolListesi.kabukKokleri(YolListesi.olustur(merkez, sube)))
        esit(listOf(sube), YolListesi.kabukKokleri(YolListesi.olustur(null, sube)))
        esit(emptyList(), YolListesi.kabukKokleri(YolListesi.olustur(null, null)))
        esit("$merkez/kabuk/#/terzitakip", YolListesi.kabukAdresi(merkez, null))
        esit("$sube/kabuk/?kod=123456#/terzitakip", YolListesi.kabukAdresi(sube, "123456"))
        esit("$sube/kabuk/#/terzitakip", YolListesi.kabukAdresi(sube, "12345a"), "bozuk kod eklenmez")
    }

    sina("düz http yalnız kayıtlı şube bilgisayarına") {
        val y = YolListesi.olustur(merkez, sube)
        dogru(YolListesi.httpIzinli("$sube/v1/fis", y), "şube ucu izinli olmalı")
        dogru(YolListesi.httpIzinli("$sube/durum", y), "şube durum izinli olmalı")
        dogru(!YolListesi.httpIzinli("http://192.168.1.20:8787/v1/fis", y), "başka kapı kesilmeli")
        dogru(!YolListesi.httpIzinli("http://192.168.1.21:8788/v1/fis", y), "başka yerel adres kesilmeli")
        dogru(!YolListesi.httpIzinli("http://sunucu.ornek.com/x", y), "merkezin http hali kesilmeli")
        dogru(!YolListesi.httpIzinli("http://ornek.com/x", y), "genel http kesilmeli")
        dogru(!YolListesi.httpIzinli("$merkez/v1/fis", y), "https bu kuralın konusu değil")
        dogru(!YolListesi.httpIzinli(null, y) && !YolListesi.httpIzinli("bozuk", y), "okunamayan adres kesilmeli")
        dogru(!YolListesi.httpIzinli("$sube/v1/fis", YolListesi.olustur(merkez, null)), "şube kayıtlı değilken kesilmeli")
    }

    sina("karışık içerik yalnız https merkez + http şube birlikteyken") {
        dogru(YolListesi.karisikIcerikGerekli(YolListesi.olustur(merkez, sube)), "ikisi varken gerekli")
        dogru(!YolListesi.karisikIcerikGerekli(YolListesi.olustur(merkez, null)), "yalnız merkez")
        dogru(!YolListesi.karisikIcerikGerekli(YolListesi.olustur(null, sube)), "yalnız şube")
        dogru(!YolListesi.karisikIcerikGerekli(YolListesi.olustur(merkez, "https://sube.ornek.com")), "ikisi de https")
    }

    sina("karekoddaki adres doğru alana yazılır") {
        esit(YolListesi.SUBE, YolListesi.alan(SunucuAdresi.coz("http://192.168.1.20:8788/telefon?kod=123456").kok))
        esit(YolListesi.SUBE, YolListesi.alan(SunucuAdresi.coz("10.0.0.5:8788").kok))
        esit(YolListesi.MERKEZ, YolListesi.alan(SunucuAdresi.coz("sunucu.ornek.com").kok))
        esit(YolListesi.MERKEZ, YolListesi.alan(SunucuAdresi.coz("https://magaza-pc.ornek.ts.net/kabuk/").kok))
    }

    sina("adres kuralları değişmedi (http yalnız yerel ağ IP'sinde)") {
        esit("http://192.168.1.20:8788", SunucuAdresi.coz("192.168.1.20:8788").kok)
        esit("https://sunucu.ornek.com", SunucuAdresi.coz("sunucu.ornek.com").kok)
        esit("123456", SunucuAdresi.coz("http://192.168.1.20:8788/telefon?kod=123456").kod)
        var reddedildi = false
        try {
            SunucuAdresi.coz("http://sunucu.ornek.com")
        } catch (e: SunucuAdresi.Hata) {
            reddedildi = true
        }
        dogru(reddedildi, "genel adreste http reddedilmeli")
    }

    sina("günlük satırı ve gün adı (UTC)") {
        val ms = 1795776901000L // 2026-11-27T10:55:01Z
        esit("2026-11-27", Gunluk.gun(ms))
        esit(
            "{\"zaman\":\"2026-11-27T10:55:01Z\",\"kaynak\":\"android\",\"olay\":\"yol.acildi\",\"ayrinti\":{\"kok\":\"x\"}}",
            Gunluk.satir(ms, "android", "yol.acildi", "{\"kok\":\"x\"}"),
        )
        esit("{\"zaman\":\"2026-11-27T10:55:01Z\",\"kaynak\":\"kabuk\",\"olay\":\"a\"}", Gunluk.satir(ms, "kabuk", "a", null))
        esit(null, Gunluk.satir(ms, "android", "Büyük Harf", null), "geçersiz olay adı")
        esit(null, Gunluk.satir(ms, "android", "a", "{\n}"), "çok satırlı ayrıntı")
        esit(null, Gunluk.satir(ms, "android", "a", "x".repeat(Gunluk.AYRINTI_EN_COK + 1)), "çok uzun ayrıntı")
        esit("2026-11-27", Gunluk.gun(1795823999000L), "23:59:59 UTC aynı gün")
        esit("2026-11-28", Gunluk.gun(1795824000000L), "gece yarısı UTC yeni gün")
    }

    sina("günlük yalnız eklenir, gün başına dosya, hiçbir şey silinmez") {
        val klasor = File(System.getProperty("java.io.tmpdir"), "taa-gunluk-sinama-${System.nanoTime()}")
        try {
            val g = Gunluk(File(klasor, "gunluk"))
            val gun1 = 1795776901000L
            val gun2 = gun1 + 24 * 3600 * 1000L
            dogru(g.yaz("uygulama.acildi", "[]", simdi = gun1), "ilk satır yazılmalı")
            dogru(g.yaz("yol.acildi", "{\"kok\":\"https://sunucu.ornek.com\"}", simdi = gun1 + 1000), "ikinci satır")
            dogru(g.yaz("kayit.eklendi", null, Gunluk.KAYNAK_SAYFA, gun2), "ertesi gün")
            dogru(!g.yaz("Geçersiz", null, simdi = gun2), "geçersiz olay yazılmaz")
            esit(listOf("2026-11-27", "2026-11-28"), g.gunler())
            val ilk = g.oku("2026-11-27")
            esit(2, ilk.size)
            dogru(ilk[0].contains("\"olay\":\"uygulama.acildi\"") && ilk[1].contains("yol.acildi"), "sıra korunur")
            esit(1, g.oku("2026-11-28").size)
            esit(1, g.oku("2026-11-27", enCok = 1).size, "enCok son satırları verir")
            dogru(g.oku("2026-11-27", enCok = 1)[0].contains("yol.acildi"), "son satır")
            esit(emptyList(), g.oku("../../etc/passwd"), "gün biçimi dışı ad okunmaz")
            esit(emptyList(), g.oku("2026-01-01"), "olmayan gün")
            // Yeni nesne aynı klasörde eskiyi ezmez, ekler.
            dogru(Gunluk(File(klasor, "gunluk")).yaz("uygulama.acildi", null, simdi = gun1 + 5000), "yeniden açılış")
            esit(3, g.oku("2026-11-27").size, "önceki satırlar yerinde")
            val ham = File(File(klasor, "gunluk"), "2026-11-27.jsonl").readText()
            dogru(ham.endsWith("\n") && ham.lines().filter { it.isNotBlank() }.size == 3, "satır sonu ve sayı")
        } finally {
            klasor.deleteRecursively() // yalnız bu sınamanın geçici klasörü
        }
    }

    println("\n$gecen geçti, ${kalan.size} kaldı" + if (kalan.isEmpty()) "" else ": " + kalan.joinToString(", "))
    exitProcess(if (kalan.isEmpty()) 0 else 1)
}
