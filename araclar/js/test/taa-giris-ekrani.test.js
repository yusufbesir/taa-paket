// Sahip: Yusuf. yazan: [Claude#KM2-hesap-giris 2026-11-25]
// Ortak giriş ekranı: açıkken arkadaki sayfa inert ve ekran en üstte; girişten sonra eski haline döner.
// Tarayıcı gerekir (playwright); bulunmazsa atlanır.
'use strict';
const test = require('node:test');
const assert = require('node:assert');
const path = require('path');
let pw = null;
try { pw = require('playwright'); } catch (_) { /* yok: atla */ }

test('giriş açıkken arka sayfa kullanılamaz', { skip: !pw && 'playwright yok' }, async () => {
  const b = await pw.chromium.launch();
  try {
    const p = await b.newPage();
    await p.setContent('<button id="arka">Arka</button><div id="ust" style="position:fixed;z-index:2147483000">üst</div>');
    await p.addScriptTag({ path: path.join(__dirname, '..', 'taa-giris-ekrani.js') });
    await p.evaluate(() => {
      window.girdi = null;
      TaaGirisEkrani.ac(document.body, { programAdi: 'Deneme', api: { giris: async () => ({ tamam: true, oturum: { uyari_sayisi: 0 } }) }, girildi: (o) => { window.girdi = o; } });
    });
    assert.strictEqual(await p.evaluate(() => document.getElementById('arka').inert), true);
    await p.evaluate(() => { const d = document.createElement('button'); d.id = 'sonra'; document.body.appendChild(d); });
    assert.strictEqual(await p.evaluate(() => document.getElementById('sonra').inert), true, 'sonradan eklenen de inert');
    for (let i = 0; i < 8; i++) {
      await p.keyboard.press('Tab');
      // Odak ya ekranın içinde ya da hiçbir öğede (belge/tarayıcı) olur; arkadaki öğelere asla geçmez.
      const disarida = await p.evaluate(() => { const a = document.activeElement; return !!a && a !== document.body && a.closest('.taa-giris') === null; });
      assert.strictEqual(disarida, false, 'Tab arkadaki öğelere geçmez');
    }
    const z = await p.evaluate(() => Number(getComputedStyle(document.querySelector('.taa-giris')).zIndex));
    assert.strictEqual(z, 2147483647);
    await p.fill('.taa-giris input[name=kullanici]', 'ali');
    await p.fill('.taa-giris input[name=parola]', 'Gizli-1234');
    await p.click('.taa-giris .tg-ana');
    await p.waitForFunction(() => window.girdi !== null);
    assert.deepStrictEqual(await p.evaluate(() => [document.querySelector('.taa-giris'), document.getElementById('arka').inert, document.getElementById('sonra').inert]), [null, false, false]);
  } finally { await b.close(); }
});
