// Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17]
package com.taa.terzitakip

import android.Manifest
import android.annotation.SuppressLint
import android.app.AlertDialog
import android.content.ActivityNotFoundException
import android.content.ClipData
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Bitmap
import android.graphics.Color
import android.net.Uri
import android.net.http.SslError
import android.os.Bundle
import android.os.Handler
import android.os.Looper
import android.provider.MediaStore
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.webkit.PermissionRequest
import android.webkit.RenderProcessGoneDetail
import android.webkit.SslErrorHandler
import android.webkit.ValueCallback
import android.webkit.WebChromeClient
import android.webkit.WebResourceError
import android.webkit.WebResourceRequest
import android.webkit.WebResourceResponse
import android.webkit.WebSettings
import android.webkit.WebView
import android.webkit.WebViewClient
import android.widget.Button
import android.widget.TextView
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.OnBackPressedCallback
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.core.content.ContextCompat
import androidx.core.content.FileProvider
import androidx.core.splashscreen.SplashScreen.Companion.installSplashScreen
import androidx.webkit.JavaScriptReplyProxy
import androidx.webkit.ScriptHandler
import androidx.webkit.WebViewCompat
import androidx.webkit.WebViewFeature
import org.json.JSONException
import org.json.JSONObject
import java.io.ByteArrayInputStream
import java.io.File
import java.io.IOException

/**
 * TAA Terzi Takip ana ekranı: mağaza sunucusundaki telefon sitesini (`<kök>/telefon`) WebView'da açar.
 *
 * - Gezinti yalnız kayıtlı sunucu kökünde; dış bağlantılar dış tarayıcıda açılır.
 * - Düz http yalnız yerel ağ IP'lerine (bkz. res/xml/ag_guvenligi.xml ve [SunucuAdresi.yerelIpMi]).
 * - Açılışta sunucuya ulaşılamazsa markalı "Mağaza sunucusuna ulaşılamıyor" ekranı; sayfa yüklendikten sonra ağ
 *   giderse telefon.html kendi çevrimdışı kuyruğunu kullanır.
 * - JS köprüsü window.TaaAndroid: karekodOku() (yerel okuyucu; sonuç `taa-karekod` olayıyla döner), ayarlar(), surum().
 * - Gizli yönetici menüsü: sayfa başlığının sol tarafına (ya da hata ekranındaki başlığa) uzun basma → Sunucu ayarı.
 */
class AnaEtkinlik : ComponentActivity() {

    private lateinit var ayarlar: Ayarlar
    private lateinit var kokGorunum: View
    private lateinit var web: WebView
    private lateinit var hataEkrani: View
    private lateinit var hataAyrinti: TextView
    private lateinit var yenidenDene: Button

    private val isleyici = Handler(Looper.getMainLooper())

    /** Kayıtlı sunucu kökü, örn. `http://192.168.1.20:8787` ya da `https://magaza-pc.ornek.ts.net`. */
    private var kok: String? = null
    private var hedefAdres: String? = null
    private var acilisHazir = false
    private var webKuruldu = false
    private var webOldu = false
    private var yuklemeHatasi = false
    private var gecmisiTemizle = false
    private var sonDokunusX = 0f
    private var sonDokunusY = 0f

    // JS köprüsü: yeni yol (WebMessageListener + belge başı betiği, kökene bağlı) ya da yedek addJavascriptInterface.
    private var betikTutamagi: ScriptHandler? = null
    private var dinleyiciVar = false
    private var eskiKopru = false
    private var yanitci: JavaScriptReplyProxy? = null
    private var karekodAcik = false

    // Dosya seçici (fiş fotoğrafı)
    private var dosyaGeriCagrisi: ValueCallback<Array<Uri>>? = null
    private var fotoUri: Uri? = null
    private var fotoDosyasi: File? = null
    private var izinSonrasi: ((Boolean) -> Unit)? = null

    private val kurulumBaslatici =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { s ->
            val yeniKok = ayarlar.sunucuKoku
            if (s.resultCode == RESULT_OK && yeniKok != null) {
                sunucuyuAc(yeniKok, s.data?.getStringExtra(KurulumEtkinligi.EK_KOD))
            } else if (kok == null) {
                finish()
            }
        }

    private val karekodBaslatici =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { s ->
            karekodAcik = false
            val detay = JSONObject()
            val deger = s.data?.getStringExtra(KarekodEtkinligi.EK_DEGER)
            if (s.resultCode == RESULT_OK && deger != null) {
                detay.put("deger", deger)
            } else {
                detay.put("iptal", true)
                s.data?.getStringExtra(KarekodEtkinligi.EK_HATA)?.let { detay.put("hata", it) }
            }
            sayfayaGonder("taa-karekod", detay)
        }

    private val kameraIzniBaslatici =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { verildi ->
            val islem = izinSonrasi
            izinSonrasi = null
            islem?.invoke(verildi)
        }

    private val dosyaBaslatici =
        registerForActivityResult(ActivityResultContracts.StartActivityForResult()) { s ->
            val geri = dosyaGeriCagrisi
            dosyaGeriCagrisi = null
            var sonuc: Array<Uri>? = null
            if (s.resultCode == RESULT_OK) {
                val liste = ArrayList<Uri>()
                s.data?.clipData?.let { c -> for (i in 0 until c.itemCount) c.getItemAt(i).uri?.let { liste.add(it) } }
                if (liste.isEmpty()) s.data?.data?.let { liste.add(it) }
                if (liste.isEmpty()) {
                    val d = fotoDosyasi
                    val u = fotoUri
                    if (d != null && u != null && d.length() > 0) liste.add(u)
                }
                if (liste.isNotEmpty()) sonuc = liste.toTypedArray()
            }
            if (sonuc == null) fotoDosyasi?.takeIf { it.length() == 0L }?.delete()
            fotoUri = null
            fotoDosyasi = null
            geri?.onReceiveValue(sonuc)
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        val acilis = installSplashScreen()
        super.onCreate(savedInstanceState)
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.light(Color.WHITE, Color.BLACK),
            navigationBarStyle = SystemBarStyle.light(Color.WHITE, Color.BLACK),
        )
        acilis.setKeepOnScreenCondition { !acilisHazir }
        isleyici.postDelayed({ acilisHazir = true }, ACILIS_EN_UZUN_MS)

        setContentView(R.layout.etkinlik_ana)
        kokGorunum = findViewById(R.id.kok)
        kokGorunum.sistemCubuklariIcinBosluk()
        web = findViewById(R.id.web)
        hataEkrani = findViewById(R.id.hata)
        hataAyrinti = findViewById(R.id.hata_ayrinti)
        yenidenDene = findViewById(R.id.yeniden_dene)
        ayarlar = Ayarlar(this)

        yenidenDene.setOnClickListener { yenidenYukle() }
        findViewById<View>(R.id.hata_baslik_satiri).setOnLongClickListener {
            ayarMenusu()
            true
        }
        onBackPressedDispatcher.addCallback(this, object : OnBackPressedCallback(true) {
            override fun handleOnBackPressed() {
                if (webKuruldu && hataEkrani.visibility != View.VISIBLE && web.canGoBack()) {
                    web.goBack()
                } else {
                    isEnabled = false
                    onBackPressedDispatcher.onBackPressed()
                    isEnabled = true
                }
            }
        })

        val kayitliKok = ayarlar.sunucuKoku
        if (kayitliKok == null) {
            acilisHazir = true
            // Etkinlik yeniden kurulduysa kurulum ekranı zaten açıktır; sonucu yine buraya gelir.
            if (savedInstanceState == null) kurulumAc(ilk = true)
            return
        }
        webKur()
        kok = kayitliKok
        kopruKur(kayitliKok)
        val geri = savedInstanceState?.getBundle(DURUM_WEB)
        if (geri != null && savedInstanceState?.getString(DURUM_KOK) == kayitliKok && web.restoreState(geri) != null) {
            hedefAdres = web.url
        } else {
            yukle(baslangicAdresi(kayitliKok, intent.getStringExtra(KurulumEtkinligi.EK_KOD)))
        }
    }

    // ---------------------------------------------------------------- WebView

    @SuppressLint("SetJavaScriptEnabled", "ClickableViewAccessibility")
    private fun webKur() {
        if (webKuruldu) return
        webKuruldu = true
        WebView.setWebContentsDebuggingEnabled(BuildConfig.DEBUG)
        web.settings.apply {
            javaScriptEnabled = true
            domStorageEnabled = true
            // Yerel dosya erişimi kapalı; content:// açık kalır çünkü dosya seçiciden gelen fotoğraflar böyle gelir.
            allowFileAccess = false
            allowContentAccess = true
            mixedContentMode = WebSettings.MIXED_CONTENT_NEVER_ALLOW
            setGeolocationEnabled(false)
            setSupportMultipleWindows(false)
            javaScriptCanOpenWindowsAutomatically = false
            userAgentString = "$userAgentString TaaTerziTakip/${BuildConfig.VERSION_NAME}"
        }
        web.webViewClient = Istemci()
        web.webChromeClient = KromIstemci()

        // Gizli yönetici menüsü: sayfanın yapışkan başlığında (üst 56dp, sol %60: marka) uzun basma.
        web.setOnTouchListener { _, e ->
            if (e.actionMasked == MotionEvent.ACTION_DOWN) {
                sonDokunusX = e.x
                sonDokunusY = e.y
            }
            false
        }
        web.setOnLongClickListener {
            val yogunluk = resources.displayMetrics.density
            if (sonDokunusY < BASLIK_DP * yogunluk && sonDokunusX < web.width * 0.6f) {
                ayarMenusu()
                true
            } else {
                false
            }
        }
    }

    private fun baslangicAdresi(kok: String, kod: String?): String =
        if (kod != null && kod.length == 6 && kod.all { it in '0'..'9' }) "$kok/telefon?kod=$kod" else "$kok/telefon"

    private fun yukle(adres: String) {
        hedefAdres = adres
        web.loadUrl(adres)
    }

    private fun yenidenYukle() {
        val adres = hedefAdres ?: kok?.let { baslangicAdresi(it, null) } ?: return
        yenidenDene.isEnabled = false
        yenidenDene.setText(R.string.yeniden_dene)
        yukle(adres)
    }

    /** Yeni ya da değişen sunucu kökünü açar (ilk kurulumdan ve Sunucu ayarından sonra). */
    private fun sunucuyuAc(yeniKok: String, kod: String?) {
        webKur()
        val degisti = yeniKok != kok
        kok = yeniKok
        if (degisti || betikTutamagi == null && !dinleyiciVar && !eskiKopru) kopruKur(yeniKok)
        gecmisiTemizle = true
        yukle(baslangicAdresi(yeniKok, kod))
    }

    private fun hataGoster(ayrinti: String) {
        yuklemeHatasi = true
        acilisHazir = true
        hataAyrinti.text = listOfNotNull(hedefAdres ?: kok, ayrinti.takeIf { it.isNotBlank() }).joinToString("\n")
        yenidenDene.isEnabled = true
        hataEkrani.visibility = View.VISIBLE
        web.visibility = View.INVISIBLE
    }

    private fun sayfaGoster() {
        hataEkrani.visibility = View.GONE
        web.visibility = View.VISIBLE
        yenidenDene.isEnabled = true
    }

    private fun disAc(uri: Uri) {
        val sema = uri.scheme?.lowercase()
        if (sema !in DIS_SEMALAR) return
        try {
            startActivity(Intent(Intent.ACTION_VIEW, uri).addCategory(Intent.CATEGORY_BROWSABLE))
        } catch (e: ActivityNotFoundException) {
            Toast.makeText(this, R.string.tarayici_yok, Toast.LENGTH_SHORT).show()
        }
    }

    private inner class Istemci : WebViewClient() {

        override fun shouldOverrideUrlLoading(view: WebView, request: WebResourceRequest): Boolean {
            if (SunucuAdresi.kokIcindeMi(request.url.toString(), kok)) return false
            // Kök dışı: ana çerçevedeyse dış tarayıcıda aç, alt çerçevedeyse engelle.
            if (request.isForMainFrame) disAc(request.url)
            return true
        }

        override fun shouldInterceptRequest(view: WebView, request: WebResourceRequest): WebResourceResponse? {
            val u = request.url
            if (u.scheme.equals("http", ignoreCase = true) && !SunucuAdresi.yerelIpMi(u.host)) {
                return WebResourceResponse(
                    "text/plain", "utf-8", 403, "Forbidden", emptyMap(),
                    ByteArrayInputStream(ByteArray(0)),
                )
            }
            return null
        }

        override fun onPageStarted(view: WebView, url: String, favicon: Bitmap?) {
            yuklemeHatasi = false
            yanitci = null
            if (SunucuAdresi.kokIcindeMi(url, kok)) hedefAdres = url
        }

        override fun onPageFinished(view: WebView, url: String) {
            acilisHazir = true
            if (!yuklemeHatasi) sayfaGoster()
            if (gecmisiTemizle) {
                gecmisiTemizle = false
                view.clearHistory()
            }
        }

        override fun onReceivedError(view: WebView, request: WebResourceRequest, error: WebResourceError) {
            if (request.isForMainFrame) hataGoster(error.description?.toString().orEmpty())
        }

        override fun onReceivedHttpError(view: WebView, request: WebResourceRequest, errorResponse: WebResourceResponse) {
            if (request.isForMainFrame && errorResponse.statusCode >= 400) {
                hataGoster(getString(R.string.hata_http, errorResponse.statusCode))
            }
        }

        @SuppressLint("WebViewClientOnReceivedSslError")
        override fun onReceivedSslError(view: WebView, handler: SslErrorHandler, error: SslError) {
            // Sertifika hatası asla geçilmez. Kendi imzalı sertifika yalnız debug derlemede (kullanıcı CA'sı) çalışır.
            handler.cancel()
            if (SunucuAdresi.kokIcindeMi(error.url, kok) || SunucuAdresi.kokIcindeMi(view.url, kok)) {
                hataGoster(getString(R.string.hata_sertifika))
            }
        }

        override fun onRenderProcessGone(view: WebView, detail: RenderProcessGoneDetail): Boolean {
            // WebView süreci çöktü ya da sistemce kapatıldı: görünümü bırak, etkinliği yeniden kur.
            webOldu = true
            (view.parent as? ViewGroup)?.removeView(view)
            view.destroy()
            recreate()
            return true
        }
    }

    private inner class KromIstemci : WebChromeClient() {

        override fun onShowFileChooser(
            webView: WebView,
            filePathCallback: ValueCallback<Array<Uri>>,
            fileChooserParams: FileChooserParams,
        ): Boolean {
            dosyaGeriCagrisi?.onReceiveValue(null)
            dosyaGeriCagrisi = filePathCallback
            val turler = fileChooserParams.acceptTypes.filter { it.isNotBlank() }
            val goruntu = turler.isEmpty() || turler.any { it.startsWith("image/") || it == "*/*" }
            val yalnizKamera = fileChooserParams.isCaptureEnabled
            if (!goruntu) {
                dosyaSeciciAc(fileChooserParams, kamera = false, yalnizKamera = false)
            } else {
                kameraIzniIle { izin -> dosyaSeciciAc(fileChooserParams, kamera = izin, yalnizKamera = yalnizKamera) }
            }
            return true
        }

        override fun onPermissionRequest(request: PermissionRequest) {
            // Sayfanın getUserMedia yolu (köprü yokken): yalnız kayıtlı kök ve yalnız kamera.
            val kamera = PermissionRequest.RESOURCE_VIDEO_CAPTURE
            if (!SunucuAdresi.kokIcindeMi(request.origin.toString(), kok) || kamera !in request.resources) {
                request.deny()
                return
            }
            kameraIzniIle { izin -> if (izin) request.grant(arrayOf(kamera)) else request.deny() }
        }
    }

    // ---------------------------------------------------------------- Dosya seçici

    private fun kameraIzniIle(islem: (Boolean) -> Unit) {
        if (ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
            islem(true)
            return
        }
        izinSonrasi?.invoke(false)
        izinSonrasi = islem
        kameraIzniBaslatici.launch(Manifest.permission.CAMERA)
    }

    private fun dosyaSeciciAc(params: WebChromeClient.FileChooserParams, kamera: Boolean, yalnizKamera: Boolean) {
        val kameraNiyeti = if (kamera) fotoNiyeti() else null
        val niyet = if (kameraNiyeti != null && yalnizKamera) {
            kameraNiyeti
        } else {
            val secici = params.createIntent()
            Intent.createChooser(secici, getString(R.string.fotograf_ekle)).apply {
                if (kameraNiyeti != null) putExtra(Intent.EXTRA_INITIAL_INTENTS, arrayOf(kameraNiyeti))
            }
        }
        try {
            dosyaBaslatici.launch(niyet)
        } catch (e: ActivityNotFoundException) {
            dosyaGeriCagrisi?.onReceiveValue(null)
            dosyaGeriCagrisi = null
            fotoUri = null
            fotoDosyasi = null
        }
    }

    /** Kamera uygulamasına verilecek çekim niyeti; fotoğraf önbellekteki geçici dosyaya yazılır. */
    private fun fotoNiyeti(): Intent? {
        return try {
            val klasor = File(cacheDir, "fotograflar").apply { mkdirs() }
            // Bir saatten eski geçici fotoğraflar (yüklenmiş ya da vazgeçilmiş) temizlenir.
            val sinir = System.currentTimeMillis() - 60 * 60 * 1000L
            klasor.listFiles()?.filter { it.lastModified() < sinir }?.forEach { it.delete() }
            val dosya = File.createTempFile("fis_", ".jpg", klasor)
            val uri = FileProvider.getUriForFile(this, "$packageName.dosyalar", dosya)
            fotoDosyasi = dosya
            fotoUri = uri
            Intent(MediaStore.ACTION_IMAGE_CAPTURE).apply {
                putExtra(MediaStore.EXTRA_OUTPUT, uri)
                addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_READ_URI_PERMISSION)
                clipData = ClipData.newRawUri("", uri)
            }
        } catch (e: IOException) {
            null
        } catch (e: IllegalArgumentException) {
            null
        }
    }

    // ---------------------------------------------------------------- JS köprüsü

    private fun kopruKur(kok: String) {
        kopruKaldir()
        val yeniYol = WebViewFeature.isFeatureSupported(WebViewFeature.WEB_MESSAGE_LISTENER) &&
            WebViewFeature.isFeatureSupported(WebViewFeature.DOCUMENT_START_SCRIPT)
        if (yeniYol) {
            val kurallar = setOf(kok)
            WebViewCompat.addWebMessageListener(web, YEREL_NESNE, kurallar) { _, ileti, kaynak, anaCerceve, yanit ->
                if (anaCerceve && SunucuAdresi.kokIcindeMi(kaynak.toString(), this.kok)) {
                    yanitci = yanit
                    istekIsle(ileti.data)
                }
            }
            dinleyiciVar = true
            betikTutamagi = WebViewCompat.addDocumentStartJavaScript(web, kopruBetigi(), kurallar)
        } else {
            web.addJavascriptInterface(TaaKoprusu(this), "TaaAndroid")
            eskiKopru = true
        }
    }

    private fun kopruKaldir() {
        betikTutamagi?.remove()
        betikTutamagi = null
        if (dinleyiciVar) {
            WebViewCompat.removeWebMessageListener(web, YEREL_NESNE)
            dinleyiciVar = false
        }
        if (eskiKopru) {
            web.removeJavascriptInterface("TaaAndroid")
            eskiKopru = false
        }
        yanitci = null
    }

    private fun istekIsle(veri: String?) {
        if (veri == null) return
        val istek = try {
            JSONObject(veri).optString("istek")
        } catch (e: JSONException) {
            return
        }
        when (istek) {
            ISTEK_KAREKOD -> karekodBaslat()
            ISTEK_AYARLAR -> ayarMenusu()
        }
    }

    /** Yedek köprüden (TaaKoprusu) gelen istek; ana iş parçacığında çağrılır. */
    fun koprudenIstek(istek: String) {
        if (!webKuruldu || !SunucuAdresi.kokIcindeMi(web.url, kok)) return
        when (istek) {
            ISTEK_KAREKOD -> karekodBaslat()
            ISTEK_AYARLAR -> ayarMenusu()
        }
    }

    private fun karekodBaslat() {
        if (karekodAcik) return
        karekodAcik = true
        karekodBaslatici.launch(Intent(this, KarekodEtkinligi::class.java))
    }

    /** Sayfaya olay gönderir: `window` üzerinde CustomEvent(olay, {detail: detay}). */
    private fun sayfayaGonder(olay: String, detay: JSONObject) {
        val y = yanitci
        if (y != null) {
            y.postMessage(JSONObject().put("olay", olay).put("detay", detay).toString())
            return
        }
        if (eskiKopru && webKuruldu && SunucuAdresi.kokIcindeMi(web.url, kok)) {
            web.evaluateJavascript(
                "window.dispatchEvent(new CustomEvent(${JSONObject.quote(olay)},{detail:$detay}));",
                null,
            )
        }
    }

    /** Belge başında çalışan, yalnız kayıtlı kökte etkin küçük betik: window.TaaAndroid'i tanımlar. */
    private fun kopruBetigi(): String = """
        (function(){
          if (window.TaaAndroid) return;
          var y = window.$YEREL_NESNE; if (!y) return;
          y.onmessage = function(e){
            var d; try { d = JSON.parse(e.data); } catch (x) { return; }
            if (d && typeof d.olay === 'string') window.dispatchEvent(new CustomEvent(d.olay, {detail: d.detay}));
          };
          function gonder(istek){ y.postMessage(JSON.stringify({istek: istek})); }
          window.TaaAndroid = Object.freeze({
            karekodOku: function(){ gonder('$ISTEK_KAREKOD'); return true; },
            ayarlar: function(){ gonder('$ISTEK_AYARLAR'); },
            surum: function(){ return '${BuildConfig.VERSION_NAME}'; }
          });
        })();
    """.trimIndent()

    // ---------------------------------------------------------------- Ayarlar (gizli menü)

    private fun ayarMenusu() {
        if (isFinishing) return
        AlertDialog.Builder(this)
            .setTitle(R.string.ayarlar_baslik)
            .setMessage("Yönetici içindir: mağaza sunucusunun adresini değiştirir.")
            .setPositiveButton("Aç") { _, _ -> kurulumAc(ilk = false) }
            .setNegativeButton(R.string.vazgec, null)
            .show()
    }

    private fun kurulumAc(ilk: Boolean) {
        kurulumBaslatici.launch(
            Intent(this, KurulumEtkinligi::class.java).putExtra(KurulumEtkinligi.EK_ILK, ilk),
        )
    }

    // ---------------------------------------------------------------- Yaşam döngüsü

    override fun onResume() {
        super.onResume()
        if (webKuruldu && !webOldu) web.onResume()
    }

    override fun onPause() {
        if (webKuruldu && !webOldu) web.onPause()
        super.onPause()
    }

    override fun onSaveInstanceState(outState: Bundle) {
        super.onSaveInstanceState(outState)
        if (webKuruldu && !webOldu && kok != null) {
            val b = Bundle()
            web.saveState(b)
            outState.putBundle(DURUM_WEB, b)
            outState.putString(DURUM_KOK, kok)
        }
    }

    override fun onDestroy() {
        isleyici.removeCallbacksAndMessages(null)
        dosyaGeriCagrisi?.onReceiveValue(null)
        dosyaGeriCagrisi = null
        if (::web.isInitialized && !webOldu) {
            (web.parent as? ViewGroup)?.removeView(web)
            web.destroy()
        }
        super.onDestroy()
    }

    companion object {
        const val ISTEK_KAREKOD = "karekod"
        const val ISTEK_AYARLAR = "ayarlar"
        private const val YEREL_NESNE = "TaaYerel"
        private const val DURUM_WEB = "web"
        private const val DURUM_KOK = "kok"
        private const val ACILIS_EN_UZUN_MS = 3000L
        private const val BASLIK_DP = 56
        private val DIS_SEMALAR = setOf("http", "https", "tel", "mailto", "sms", "geo", "whatsapp")
    }
}
