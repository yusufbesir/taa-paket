// Sahip: Yusuf. yazan: [Claude#terzi-takip-android 2026-11-17]
package com.taa.terzitakip

import android.view.View
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

/**
 * Kenardan kenara (edge-to-edge) çizimde içeriği durum çubuğu, gezinme çubuğu, çentik ve klavyenin altında bırakmaz.
 * targetSdk 35+ ile kenardan kenara zorunlu olduğu için adjustResize tek başına yetmez; klavye boşluğu da burada verilir.
 */
fun View.sistemCubuklariIcinBosluk() {
    ViewCompat.setOnApplyWindowInsetsListener(this) { v, ic ->
        val b = ic.getInsets(
            WindowInsetsCompat.Type.systemBars() or
                WindowInsetsCompat.Type.displayCutout() or
                WindowInsetsCompat.Type.ime()
        )
        v.setPadding(b.left, b.top, b.right, b.bottom)
        WindowInsetsCompat.CONSUMED
    }
    ViewCompat.requestApplyInsets(this)
}
