// Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17]
// değiştiren: [Claude Fable 5.2 #taa-web-W2-kurulum 2026-11-27] (yol listesi: merkez + şube bilgisayarı)
package com.taa.terzitakip

import android.content.Intent
import android.graphics.Color
import android.os.Bundle
import android.view.View
import android.view.inputmethod.EditorInfo
import android.widget.Button
import android.widget.EditText
import android.widget.TextView
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts

/**
 * İlk kurulum ve gizli "Sunucu ayarı" ekranı. Yol listesinin iki adresi yazılır: merkez sunucu ve şube bilgisayarı
 * (sıra sabit: merkez → şube → yerel; en az biri gerekir). Adres elle yazılır ya da karekod okutulur: karekoddaki
 * adres yerel ağ IP'siyse şube bilgisayarı alanına, değilse merkez alanına yazılır; kod varsa hemen kaydedilir.
 * Sonuç: RESULT_OK ve isteğe bağlı [EK_KOD]; adresler [Ayarlar]'a yazılmıştır.
 */
class KurulumEtkinligi : ComponentActivity() {

    private lateinit var merkez: EditText
    private lateinit var adres: EditText
    private lateinit var kod: EditText
    private lateinit var mesaj: TextView

    private val karekodBaslatici =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { s ->
            val deger = s.data?.getStringExtra(KarekodEtkinligi.EK_DEGER)
            if (s.resultCode == RESULT_OK && deger != null) {
                karekodIsle(deger)
            } else {
                s.data?.getStringExtra(KarekodEtkinligi.EK_HATA)?.let { mesajGoster(it) }
            }
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.light(Color.WHITE, Color.BLACK),
            navigationBarStyle = SystemBarStyle.light(Color.WHITE, Color.BLACK),
        )
        setContentView(R.layout.etkinlik_kurulum)
        findViewById<View>(R.id.kok).sistemCubuklariIcinBosluk()

        merkez = findViewById(R.id.merkez)
        adres = findViewById(R.id.adres)
        kod = findViewById(R.id.kod)
        mesaj = findViewById(R.id.mesaj)
        val ilk = intent.getBooleanExtra(EK_ILK, true)

        if (savedInstanceState == null) {
            val a = Ayarlar(this)
            merkez.setText(a.merkezKoku.orEmpty())
            // Eski tek adresli ayar (mağaza bilgisayarı) varsa şube bilgisayarı alanına öneri olarak gelir.
            adres.setText(a.subeKoku ?: a.sunucuKoku.orEmpty())
        }
        findViewById<TextView>(R.id.surum).text = getString(R.string.surum, BuildConfig.VERSION_NAME)
        if (!ilk) {
            findViewById<Button>(R.id.vazgec).apply {
                visibility = View.VISIBLE
                setOnClickListener { setResult(RESULT_CANCELED); finish() }
            }
            findViewById<View>(R.id.ayar_not).visibility = View.VISIBLE
        }

        findViewById<Button>(R.id.karekod).setOnClickListener {
            mesaj.visibility = View.GONE
            karekodBaslatici.launch(Intent(this, KarekodEtkinligi::class.java))
        }
        findViewById<Button>(R.id.kaydet).setOnClickListener { kaydet() }
        kod.setOnEditorActionListener { _, eylem, _ ->
            if (eylem == EditorInfo.IME_ACTION_DONE) {
                kaydet()
                true
            } else {
                false
            }
        }
    }

    private fun karekodIsle(deger: String) {
        val sonuc = try {
            SunucuAdresi.coz(deger)
        } catch (e: SunucuAdresi.Hata) {
            mesajGoster("Bu karekod bir sunucu adresi değil. " + (e.message ?: ""))
            return
        }
        if (YolListesi.alan(sonuc.kok) == YolListesi.SUBE) adres.setText(sonuc.kok) else merkez.setText(sonuc.kok)
        kod.setText(sonuc.kod.orEmpty())
        // Eşleştirme karekodu adresi ve kodu birlikte taşır: beklemeden kaydet ve bağlan.
        if (sonuc.kod != null) kaydet()
    }

    /** Alanı çözer: boşsa null, uygun değilse mesaj gösterip [SunucuAdresi.Hata] fırlatır. */
    private fun alanCoz(alan: EditText, ad: String): SunucuAdresi.Sonuc? {
        if (alan.text.toString().isBlank()) return null
        return try {
            SunucuAdresi.coz(alan.text.toString())
        } catch (e: SunucuAdresi.Hata) {
            mesajGoster("$ad: " + (e.message ?: "adres geçersiz."))
            alan.requestFocus()
            throw e
        }
    }

    private fun kaydet() {
        mesaj.visibility = View.GONE
        val merkezSonuc: SunucuAdresi.Sonuc?
        val subeSonuc: SunucuAdresi.Sonuc?
        try {
            merkezSonuc = alanCoz(merkez, getString(R.string.merkez_adresi))
            subeSonuc = alanCoz(adres, getString(R.string.sunucu_adresi))
        } catch (e: SunucuAdresi.Hata) {
            return
        }
        if (merkezSonuc == null && subeSonuc == null) {
            mesajGoster(getString(R.string.adres_gerekli))
            merkez.requestFocus()
            return
        }
        if (merkezSonuc != null && merkezSonuc.kok == subeSonuc?.kok) {
            mesajGoster(getString(R.string.adresler_ayni))
            adres.requestFocus()
            return
        }
        val yazilanKod = kod.text.toString().filter { it in '0'..'9' }
        if (yazilanKod.isNotEmpty() && yazilanKod.length != 6) {
            mesajGoster("Eşleştirme kodu 6 rakam olmalı (boş da bırakılabilir).")
            kod.requestFocus()
            return
        }
        Ayarlar(this).apply {
            merkezKoku = merkezSonuc?.kok
            subeKoku = subeSonuc?.kok
        }
        val gidecekKod = yazilanKod.ifEmpty { merkezSonuc?.kod ?: subeSonuc?.kod }
        setResult(RESULT_OK, Intent().putExtra(EK_KOD, gidecekKod))
        finish()
    }

    private fun mesajGoster(metin: String) {
        mesaj.text = metin
        mesaj.visibility = View.VISIBLE
    }

    companion object {
        /** true: ilk kurulum (Vazgeç yok, geri tuşu uygulamayı kapatır). */
        const val EK_ILK = "ilk"
        const val EK_KOD = "kod"
    }
}
