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

test('hedef gövde değilse ekranı taşıyan öğe inert olmaz', { skip: !pw && 'playwright yok' }, async () => {
  const b = await pw.chromium.launch();
  try {
    const p = await b.newPage();
    await p.setContent('<main id="kap"></main><button id="arka">Arka</button>');
    await p.addScriptTag({ path: path.join(__dirname, '..', 'taa-giris-ekrani.js') });
    await p.evaluate(() => TaaGirisEkrani.ac(document.getElementById('kap'), { api: { giris: async () => ({ tamam: false }) } }));
    assert.deepStrictEqual(await p.evaluate(() => [document.getElementById('kap').inert, document.getElementById('arka').inert]), [false, true]);
  } finally { await b.close(); }
});

const kur = async (b, html) => {
  const p = await b.newPage();
  await p.setContent(html || '<main>program</main>');
  await p.addScriptTag({ path: path.join(__dirname, '..', 'taa-giris-ekrani.js') });
  return p;
};
const menuKur = (p, ek) => p.evaluate((ek) => {
  window.cikti = 0; window.cikislar = []; window.tiklanan = [];
  const api = { cikis: async () => { window.cikti++; return { tamam: true }; } };
  const s = { api, oturum: { kullanici: 'ayse', roller: ['kasa'], sube_id: 3 }, programAdi: 'Deneme', surum: '1.2.3',
    cikinca: (n) => { window.cikislar.push(n); },
    ogeler: [{ ad: 'Ayarlar', tikla: () => window.tiklanan.push('ayarlar') }, { ayrac: true },
      { ad: 'Tema', alt: [{ ad: 'Açık', secili: true, tikla: () => window.tiklanan.push('acik') }, { ad: 'Koyu', tikla: () => window.tiklanan.push('koyu') }] }] };
  if (ek === 'hedef') s.hedef = document.getElementById('kenar');
  TaaGirisEkrani.hesapMenusu(s);
  TaaGirisEkrani.hesapMenusu(s);
}, ek);

test('Hesap menüsü: rozet, açılır menü, onaylı çıkış (Yusuf 11-25)', { skip: !pw && 'playwright yok' }, async () => {
  const b = await pw.chromium.launch();
  try {
    const p = await kur(b);
    await menuKur(p);
    assert.strictEqual(await p.locator('#taa-giris-hesap').count(), 1, 'ikinci çağrı yeni düğme eklemez');
    assert.strictEqual(await p.textContent('#taa-giris-hesap .tg-rozet'), 'A');
    assert.match(await p.textContent('#taa-giris-hesap'), /ayse/);
    assert.match(await p.textContent('#taa-giris-hesap'), /Kasa · Şube 3/);
    // Sol altta sabit.
    const k = await p.evaluate(() => { const r = document.getElementById('taa-giris-hesap').getBoundingClientRect(); return [r.left < 40, innerHeight - r.bottom < 40, getComputedStyle(document.getElementById('taa-giris-hesap')).position]; });
    assert.deepStrictEqual(k, [true, true, 'fixed']);
    await p.click('#taa-giris-hesap');
    assert.strictEqual(await p.getAttribute('#taa-giris-hesap', 'aria-expanded'), 'true');
    const metin = await p.textContent('#taa-giris-menu');
    for (const t of ['Oturum açık', 'Ayarlar', 'Tema', 'Kullanıcı değiştir', 'Ekranı kilitle', 'Ctrl+L', 'Çıkış yap', 'Deneme · sürüm 1.2.3']) assert.ok(metin.includes(t), t);
    // Menü ekranın içinde ve düğmenin üstünde.
    const yer = await p.evaluate(() => { const m = document.getElementById('taa-giris-menu').getBoundingClientRect(); const d = document.getElementById('taa-giris-hesap').getBoundingClientRect(); return [m.top >= 0, m.bottom <= d.top, m.left >= 0]; });
    assert.deepStrictEqual(yer, [true, true, true]);
    // İlk tık onay ister, çıkmaz.
    await p.click('#taa-giris-menu [data-ortak=cikis]');
    assert.match(await p.textContent('#taa-giris-menu [data-ortak=cikis]'), /Emin misiniz/);
    assert.strictEqual(await p.evaluate(() => window.cikti), 0);
    await p.click('#taa-giris-menu [data-ortak=cikis]');
    await p.waitForFunction(() => window.cikislar.length === 1);
    assert.deepStrictEqual(await p.evaluate(() => [window.cikti, window.cikislar[0], document.getElementById('taa-giris-hesap'), document.getElementById('taa-giris-menu')]),
      [1, { neden: 'cikis', kullanici: '', ileti: '' }, null, null]);
  } finally { await b.close(); }
});

test('Hesap menüsü: klavye, Esc, dışarı tık, program öğeleri', { skip: !pw && 'playwright yok' }, async () => {
  const b = await pw.chromium.launch();
  try {
    const p = await kur(b, '<main><button id="dis">dış</button></main>');
    await menuKur(p);
    await p.focus('#taa-giris-hesap');
    await p.keyboard.press('Enter');
    const odak = () => p.evaluate(() => document.activeElement.textContent.trim());
    assert.strictEqual(await odak(), 'Ayarlar', 'ilk öğe odakta');
    await p.keyboard.press('ArrowDown'); assert.strictEqual(await odak(), 'TemaAçık›', 'çekmece kapalı: tek satır, seçili tema görünür');
    await p.keyboard.press('ArrowUp'); await p.keyboard.press('ArrowUp');
    assert.strictEqual(await odak(), 'Çıkış yap', 'yukarı ok başa dolanır');
    await p.keyboard.press('Home'); assert.strictEqual(await odak(), 'Ayarlar');
    await p.keyboard.press('Escape');
    assert.strictEqual(await p.evaluate(() => [document.getElementById('taa-giris-menu'), document.activeElement.id]).then((x) => x[0] === null && x[1]), 'taa-giris-hesap', 'Esc kapatır, odak düğmeye döner');
    assert.strictEqual(await p.evaluate(() => document.querySelector('[role=menuitemradio][aria-checked=true]')), null);
    await p.click('#taa-giris-hesap');
    assert.strictEqual(await p.getAttribute('#taa-giris-menu [role=menuitemradio]:first-of-type', 'aria-checked'), 'true');
    await p.click('#dis');
    assert.strictEqual(await p.evaluate(() => document.getElementById('taa-giris-menu')), null, 'dışarı tık kapatır');
    await p.click('#taa-giris-hesap');
    await p.click('#taa-giris-menu .tg-cekmece');
    await p.click('#taa-giris-menu >> text=Koyu');
    await p.click('#taa-giris-menu >> text=Ayarlar');
    assert.deepStrictEqual(await p.evaluate(() => [window.tiklanan, document.getElementById('taa-giris-menu'), window.cikti]), [['koyu', 'ayarlar'], null, 0]);
  } finally { await b.close(); }
});

test('Hesap menüsü: Kullanıcı değiştir ve Ctrl+L kilit adı dolu girişe döner', { skip: !pw && 'playwright yok' }, async () => {
  const b = await pw.chromium.launch();
  try {
    const p = await kur(b);
    await menuKur(p);
    await p.click('#taa-giris-hesap');
    await p.click('#taa-giris-menu [data-ortak=degistir]');
    await p.waitForFunction(() => window.cikislar.length === 1);
    assert.deepStrictEqual(await p.evaluate(() => window.cikislar[0]), { neden: 'degistir', kullanici: '', ileti: '' });
    await menuKur(p);
    await p.keyboard.press('Control+l');
    await p.waitForFunction(() => window.cikislar.length === 1);
    // Kilit oturumu kapatmaz (Yusuf 11-26 "Kilit yayını durdurmasın"): cikis gitmez, olay yayılır.
    assert.deepStrictEqual(await p.evaluate(() => [window.cikti, window.cikislar[0]]), [0, { neden: 'kilit', kullanici: 'ayse', ileti: '' }]);
    // Kilit sonrası giriş ekranı adı dolu (değiştirilemez) ve kilitli metniyle açılır; giriş açıkken Ctrl+L bir şey yapmaz.
    await p.evaluate(() => TaaGirisEkrani.ac(document.body, { kullanici: 'ayse', kilitli: true, api: { giris: async () => ({ tamam: false }) } }));
    assert.strictEqual(await p.inputValue('.taa-giris input[name=kullanici]'), 'ayse');
    assert.strictEqual(await p.evaluate(() => document.querySelector('.taa-giris input[name=kullanici]').readOnly), true);
    assert.match(await p.textContent('.taa-giris'), /Ekran kilitli/);
    assert.ok(await p.$('.taa-giris [data-baska]'), 'Başka kullanıcıyla gir var');
    await menuKur(p);
    await p.keyboard.press('Control+l');
    await new Promise((r) => setTimeout(r, 100));
    assert.strictEqual(await p.evaluate(() => window.cikti), 0, 'giriş ekranı açıkken kilit çalışmaz');
  } finally { await b.close(); }
});

test('Hesap menüsü: hedef öğeye yerleşir, HTML yorumlanmaz, eski çıkış düğmesi kalkar', { skip: !pw && 'playwright yok' }, async () => {
  const b = await pw.chromium.launch();
  try {
    const p = await kur(b, '<aside id="kenar"></aside><button id="taa-giris-cikis">eski</button>');
    await menuKur(p, 'hedef');
    assert.deepStrictEqual(await p.evaluate(() => { const d = document.getElementById('taa-giris-hesap'); return [d.parentNode.id, d.classList.contains('tg-yerinde'), getComputedStyle(d).position, document.getElementById('taa-giris-cikis')]; }),
      ['kenar', true, 'static', null]);
    await p.evaluate(() => TaaGirisEkrani.cikisDugmesi({ api: { cikis: async () => ({}) }, oturum: { kullanici: '<b>x</b>' }, ogeler: [{ ad: '<i>y</i>', tikla() {} }] }));
    await p.click('#taa-giris-hesap');
    assert.deepStrictEqual(await p.evaluate(() => [document.querySelector('#taa-giris-hesap b b'), document.querySelector('#taa-giris-menu i'), document.querySelector('#taa-giris-menu').textContent.includes('<i>y</i>')]), [null, null, true]);
  } finally { await b.close(); }
});

test('Hesap menüsü: kilit sürerken ikinci Ctrl+L yeniden çalışmaz; işaret yazılamazsa ileti (Fable #88)', { skip: !pw && 'playwright yok' }, async () => {
  const b = await pw.chromium.launch();
  try {
    const p = await kur(b);
    await p.evaluate(() => {
      window.cikti = 0; window.kilit = 0; window.cikislar = []; window.olaylar = []; window.kanca = [];
      document.addEventListener('taa-giris:kilitlendi', (e) => window.olaylar.push(e.detail));
      const api = { cikis: async () => { window.cikti++; return { tamam: true }; },
        kilitle: () => { window.kilit++; return new Promise((r) => setTimeout(() => r({ tamam: false }), 150)); } };
      TaaGirisEkrani.hesapMenusu({ api, oturum: { kullanici: 'ayse' }, cikinca: (n) => window.cikislar.push(n), kilitlenince: (k) => window.kanca.push(k) });
    });
    await p.keyboard.press('Control+l');
    await p.keyboard.press('Control+l');
    await p.evaluate(() => document.getElementById('taa-giris-hesap') && document.getElementById('taa-giris-hesap').click());
    await p.waitForFunction(() => window.cikislar.length === 1);
    await new Promise((r) => setTimeout(r, 250));
    const [cikti, kilit, n, olaylar, kanca] = await p.evaluate(() => [window.cikti, window.kilit, window.cikislar, window.olaylar, window.kanca]);
    assert.strictEqual(cikti, 0, 'kilit oturumu kapatmaz');
    assert.strictEqual(kilit, 1, 'kilitle bir kez gider');
    assert.strictEqual(n.length, 1, 'cikinca bir kez çalışır');
    assert.strictEqual(n[0].neden, 'kilit');
    assert.match(n[0].ileti, /kilit kalmayabilir/);
    assert.deepStrictEqual([olaylar, kanca], [[{ kullanici: 'ayse' }], [{ kullanici: 'ayse' }]]);
    await p.evaluate((ileti) => TaaGirisEkrani.ac(document.body, { kullanici: 'ayse', kilitli: true, ileti, api: { giris: async () => ({ tamam: false }) } }), n[0].ileti);
    assert.match(await p.textContent('.taa-giris .tg-ileti'), /kilit kalmayabilir/);
  } finally { await b.close(); }
});

test('Kilit ekranı: aynı kullanıcıyla açılınca olay; ad değiştirilemez; Başka kullanıcıyla gir gerçek çıkış yapar', { skip: !pw && 'playwright yok' }, async () => {
  const b = await pw.chromium.launch();
  try {
    const p = await kur(b);
    await p.evaluate(() => {
      window.giden = []; window.cikti = 0; window.acildi = []; window.cikildi = []; window.girdi = null;
      document.addEventListener('taa-giris:kilit-acildi', (e) => window.acildi.push(e.detail));
      document.addEventListener('taa-giris:cikildi', (e) => window.cikildi.push(e.detail));
      const api = { giris: async (k) => { window.giden.push(k); return { tamam: true, oturum: { kullanici: k } }; }, cikis: async () => { window.cikti++; return { tamam: true }; } };
      window.api = api;
      TaaGirisEkrani.ac(document.body, { kullanici: 'ayse', kilitli: true, api, girildi: (o) => { window.girdi = o; } });
    });
    // readonly alan elle değiştirilse bile kilitli kullanıcı gider
    await p.evaluate(() => { document.querySelector('.taa-giris input[name=kullanici]').value = 'veli'; });
    await p.fill('.taa-giris input[name=parola]', 'Gizli-1234');
    await p.click('.taa-giris .tg-ana');
    await p.waitForFunction(() => window.girdi !== null);
    assert.deepStrictEqual(await p.evaluate(() => [window.giden, window.acildi, window.cikti]), [['ayse'], [{ kullanici: 'ayse' }], 0]);
    await p.evaluate(() => TaaGirisEkrani.ac(document.body, { kullanici: 'ayse', kilitli: true, api: window.api }));
    await p.click('.taa-giris [data-baska]');
    await p.waitForFunction(() => window.cikti === 1);
    assert.deepStrictEqual(await p.evaluate(() => [window.cikildi, document.querySelector('.taa-giris input[name=kullanici]').value,
      document.querySelector('.taa-giris input[name=kullanici]').readOnly, !!document.querySelector('.taa-giris [data-baska]')]),
      [[{ kullanici: 'ayse' }], '', false, false]);
  } finally { await b.close(); }
});

test('gizli açılış: ekran görünmez ama arka kilitli; goster() açar ve odaklar', { skip: !pw && 'playwright yok' }, async () => {
  const b = await pw.chromium.launch();
  try {
    const p = await kur(b, '<button id="arka">Arka</button>');
    await p.evaluate(() => { window.e = TaaGirisEkrani.ac(document.body, { gizli: true, api: { giris: async () => ({ tamam: false }) } }); });
    assert.deepStrictEqual(await p.evaluate(() => [getComputedStyle(document.querySelector('.taa-giris')).visibility, document.getElementById('arka').inert]), ['hidden', true]);
    await p.evaluate(() => { window.e.goster(); window.e.goster(); });
    assert.deepStrictEqual(await p.evaluate(() => [getComputedStyle(document.querySelector('.taa-giris')).visibility, document.activeElement.name]), ['visible', 'kullanici']);
  } finally { await b.close(); }
});

test('Hesap menüsü: Tema çekmecesi (Yusuf 11-26)', { skip: !pw && 'playwright yok' }, async () => {
  const b = await pw.chromium.launch();
  try {
    const p = await kur(b);
    await menuKur(p);
    await p.click('#taa-giris-hesap');
    const c = '#taa-giris-menu .tg-cekmece';
    const durum = () => p.evaluate((c) => { const e = document.querySelector(c); return [e.getAttribute('aria-expanded'), e.nextElementSibling.hidden, e.querySelector('.tg-secili').textContent, document.activeElement.textContent.trim()]; }, c);
    assert.deepStrictEqual((await durum()).slice(0, 3), ['false', true, 'Açık'], 'kapalı, seçili ad görünür');
    // Klavye: sağ ok açar ve seçili olana odaklanır; sol ok kapatır, odak satıra döner.
    await p.focus(c);
    await p.keyboard.press('ArrowRight');
    assert.deepStrictEqual(await durum(), ['true', false, 'Açık', 'Açık']);
    await p.keyboard.press('ArrowDown'); assert.strictEqual((await durum())[3], 'Koyu');
    await p.keyboard.press('ArrowLeft');
    assert.deepStrictEqual(await durum(), ['false', true, 'Açık', 'TemaAçık›']);
    // Esc çekmecedeyken yalnız çekmeceyi kapatır.
    await p.keyboard.press('Enter'); await p.keyboard.press('Escape');
    assert.deepStrictEqual((await durum()).slice(0, 2), ['false', true]);
    assert.ok(await p.evaluate(() => !!document.getElementById('taa-giris-menu')), 'menü açık kalır');
    // Seçim: işaret ve ad güncellenir, çekmece kapanır, menü açık kalır, tikla çağrılır.
    await p.keyboard.press('Enter'); await p.keyboard.press('ArrowDown'); await p.keyboard.press('Enter');
    assert.deepStrictEqual(await durum(), ['false', true, 'Koyu', 'TemaKoyu›']);
    assert.deepStrictEqual(await p.evaluate(() => [window.tiklanan, document.querySelector('#taa-giris-menu [role=menuitemradio][aria-checked=true]').textContent, !!document.getElementById('taa-giris-menu')]), [['koyu'], 'Koyu', true]);
    // Menü ekranda kalır (çekmece açılınca yeniden yerleşir).
    await p.click(c);
    assert.ok(await p.evaluate(() => { const r = document.getElementById('taa-giris-menu').getBoundingClientRect(); return r.top >= 0 && r.bottom <= innerHeight; }));
  } finally { await b.close(); }
});

test('Hesap menüsü: ogeler işlev olabilir, her açılışta yeniden okunur', { skip: !pw && 'playwright yok' }, async () => {
  const b = await pw.chromium.launch();
  try {
    const p = await kur(b);
    await p.evaluate(() => {
      window.yon = false;
      TaaGirisEkrani.hesapMenusu({ api: { cikis: async () => ({}) }, oturum: { kullanici: 'ayse' },
        ogeler: () => [{ ad: window.yon ? 'Yönetici kipinden çık' : 'Yönetici girişi', tikla() { window.yon = !window.yon; } }] });
    });
    await p.click('#taa-giris-hesap');
    await p.click('#taa-giris-menu >> text=Yönetici girişi');
    await p.click('#taa-giris-hesap');
    assert.ok((await p.textContent('#taa-giris-menu')).includes('Yönetici kipinden çık'));
    await p.evaluate(() => TaaGirisEkrani.hesapMenusu({ api: { cikis: async () => ({}) }, oturum: { kullanici: 'ayse' }, ogeler: () => { throw new Error('x'); } }));
    await p.click('#taa-giris-hesap');
    assert.ok((await p.textContent('#taa-giris-menu')).includes('Kullanıcı değiştir'), 'işlev hata verse de menü açılır');
  } finally { await b.close(); }
});
