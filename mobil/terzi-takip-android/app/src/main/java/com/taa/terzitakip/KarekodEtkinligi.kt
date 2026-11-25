// Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17]
package com.taa.terzitakip

import android.Manifest
import android.content.Intent
import android.content.pm.PackageManager
import android.graphics.Color
import android.os.Bundle
import android.view.HapticFeedbackConstants
import android.view.View
import android.widget.Button
import androidx.activity.ComponentActivity
import androidx.activity.SystemBarStyle
import androidx.activity.enableEdgeToEdge
import androidx.activity.result.contract.ActivityResultContracts
import androidx.camera.core.ImageAnalysis
import androidx.camera.mlkit.vision.MlKitAnalyzer
import androidx.camera.view.CameraController
import androidx.camera.view.LifecycleCameraController
import androidx.camera.view.PreviewView
import androidx.core.content.ContextCompat
import com.google.mlkit.vision.barcode.BarcodeScanner
import com.google.mlkit.vision.barcode.BarcodeScannerOptions
import com.google.mlkit.vision.barcode.BarcodeScanning
import com.google.mlkit.vision.barcode.common.Barcode

/**
 * Yerel karekod okuyucu: CameraX + ML Kit barkod tarayıcı ("bundled", Google Play hizmeti gerekmez).
 * İlk okunan QR değerini [EK_DEGER] ile döndürür; vazgeçilir ya da kamera açılamazsa RESULT_CANCELED (+[EK_HATA]).
 */
class KarekodEtkinligi : ComponentActivity() {

    private var kontrol: LifecycleCameraController? = null
    private var tarayici: BarcodeScanner? = null
    private var bitti = false

    private val izinBaslatici =
        registerForActivityResult(ActivityResultContracts.RequestPermission()) { verildi ->
            if (verildi) kameraBaslat() else bitir(null, getString(R.string.kamera_izni_yok))
        }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        enableEdgeToEdge(
            statusBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
            navigationBarStyle = SystemBarStyle.dark(Color.TRANSPARENT),
        )
        setContentView(R.layout.etkinlik_karekod)
        findViewById<View>(R.id.kok).sistemCubuklariIcinBosluk()
        findViewById<Button>(R.id.vazgec).setOnClickListener { bitir(null, null) }

        if (ContextCompat.checkSelfPermission(this, Manifest.permission.CAMERA) == PackageManager.PERMISSION_GRANTED) {
            kameraBaslat()
        } else {
            izinBaslatici.launch(Manifest.permission.CAMERA)
        }
    }

    private fun kameraBaslat() {
        if (bitti || kontrol != null) return
        val secenekler = BarcodeScannerOptions.Builder()
            .setBarcodeFormats(Barcode.FORMAT_QR_CODE)
            .build()
        val t = BarcodeScanning.getClient(secenekler)
        tarayici = t
        val anaIs = ContextCompat.getMainExecutor(this)
        val k = LifecycleCameraController(this)
        k.setEnabledUseCases(CameraController.IMAGE_ANALYSIS)
        k.setImageAnalysisAnalyzer(
            anaIs,
            MlKitAnalyzer(listOf(t), ImageAnalysis.COORDINATE_SYSTEM_ORIGINAL, anaIs) { sonuc ->
                val deger = sonuc.getValue(t)
                    ?.firstNotNullOfOrNull { b -> b.rawValue?.takeIf { it.isNotBlank() } }
                if (deger != null) bitir(deger, null)
            },
        )
        try {
            k.bindToLifecycle(this)
        } catch (e: RuntimeException) {
            bitir(null, getString(R.string.kamera_acilamadi))
            return
        }
        findViewById<PreviewView>(R.id.onizleme).controller = k
        kontrol = k
    }

    private fun bitir(deger: String?, hata: String?) {
        if (bitti) return
        bitti = true
        kontrol?.clearImageAnalysisAnalyzer()
        if (deger != null) {
            window.decorView.performHapticFeedback(HapticFeedbackConstants.VIRTUAL_KEY)
        }
        val sonuc = Intent()
        if (deger != null) sonuc.putExtra(EK_DEGER, deger)
        if (hata != null) sonuc.putExtra(EK_HATA, hata)
        setResult(if (deger != null) RESULT_OK else RESULT_CANCELED, sonuc)
        finish()
    }

    override fun onDestroy() {
        tarayici?.close()
        tarayici = null
        super.onDestroy()
    }

    companion object {
        const val EK_DEGER = "deger"
        const val EK_HATA = "hata"
    }
}
