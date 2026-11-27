// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// Ortak biçim yardımcıları, DOM yardımcıları ve modül listesi testleri.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile, access } from 'node:fs/promises';
import { paraYaz, paraCoz, adetCoz, tarihGecerli, bugun, katla, eslesir } from '../bicim.js';
import { el, onbellekOku, gidenDegisinceYenile } from '../dom.js';
import { sahteBelge } from '../sinama/sahte-dom.js';
import { sahteBaglam } from '../sinama/sahte-baglam.js';

test('paraYaz: kuruşu Türkçe biçimde yazar', () => {
  assert.equal(paraYaz(0), '0,00 TL');
  assert.equal(paraYaz(123456), '1.234,56 TL');
  assert.equal(paraYaz(-5), '−0,05 TL');
  assert.equal(paraYaz(1.5), '?');
});

test('paraCoz: Türkçe yazımı kuruşa çevirir, belirsizi reddeder', () => {
  assert.equal(paraCoz(''), 0);
  assert.equal(paraCoz('1.234,56'), 123456);
  assert.equal(paraCoz('1234,5'), 123450);
  assert.equal(paraCoz('1.234'), 123400);
  assert.equal(paraCoz('12.50'), 1250);
  assert.equal(paraCoz('250 TL'), 25000);
  assert.equal(paraCoz('abc'), null);
  assert.equal(paraCoz('1,2,3'), null);
  assert.equal(paraCoz('1,234'), null);
  assert.equal(paraCoz('-45'), null, 'eksi izinli değilse reddedilir');
  assert.equal(paraCoz('-45', { eksi: true }), -4500);
  assert.equal(paraCoz('99999999999999'), null, 'üst sınır');
});

test('adetCoz ve tarihGecerli', () => {
  assert.equal(adetCoz(''), 0);
  assert.equal(adetCoz('12'), 12);
  assert.equal(adetCoz('1,5'), null);
  assert.equal(adetCoz('-1'), null);
  assert.equal(tarihGecerli('2026-11-27'), true);
  assert.equal(tarihGecerli('2026-02-30'), false);
  assert.equal(tarihGecerli('27.11.2026'), false);
  assert.equal(bugun(Date.UTC(2026, 10, 27, 21, 30)), '2026-11-28', 'Türkiye saati (UTC+3)');
});

test('katla/eslesir: Türkçe harf ve büyük-küçük farkı gözetilmez', () => {
  assert.equal(katla('  İSTANBUL  Şube '), 'istanbul sube');
  assert.equal(katla('Işık'), 'isik');
  assert.equal(eslesir('Çağrı Öztürk', 'cagri ozturk'), true);
  assert.equal(eslesir('Çağrı Öztürk', 'ozturk cagri'), true);
  assert.equal(eslesir('Çağrı Öztürk', 'yilmaz'), false);
});

test('el: metni textContent ile yazar (HTML olarak yorumlanmaz)', () => {
  const { doc } = sahteBelge();
  const d = el(doc, 'p', { text: '<img src=x onerror=alert(1)>' });
  assert.equal(d.children.length, 0);
  assert.equal(d.textContent, '<img src=x onerror=alert(1)>');
});

test('onbellekOku: düz değer ve {deger,surum} sarmalı', async () => {
  assert.deepEqual(await onbellekOku(sahteBaglam({ onbellek: { a: [1] } }).baglam, 'a'), [1]);
  assert.deepEqual(await onbellekOku(sahteBaglam({ onbellek: { a: [1] }, sarmali: true }).baglam, 'a'), [1]);
  assert.equal(await onbellekOku(sahteBaglam().baglam, 'yok'), undefined);
});

test('gidenDegisinceYenile: kök DOM\'dan ayrılınca dinleyici kalkar', async () => {
  const { doc, kok, pencere } = sahteBelge();
  let n = 0;
  gidenDegisinceYenile(kok, () => {
    n += 1;
  });
  pencere.dispatchEvent({ type: 'taa-giden-degisti' });
  await new Promise((c) => setImmediate(c));
  assert.equal(n, 1);
  doc.body.removeChild(kok);
  pencere.dispatchEvent({ type: 'taa-giden-degisti' });
  await new Promise((c) => setImmediate(c));
  assert.equal(n, 1);
  assert.equal(pencere.dinleyiciSayisi('taa-giden-degisti'), 0);
});

test('LISTE.json: her modül dosyası var ve {ad, baslik, ac} dışa aktarıyor', async () => {
  const kok = new URL('../../', import.meta.url);
  const liste = JSON.parse(await readFile(new URL('LISTE.json', kok), 'utf8'));
  // [Claude Fable 5.2 #taa-web-W3-tutanak 2026-11-27] 'tutanak' (Tutaa) eklendi.
  assert.deepEqual(liste.moduller.map((m) => m.ad), ['kasa', 'terzitakip', 'tutanak', 'rehber', 'cati']);
  for (const m of liste.moduller) {
    for (const d of m.dosyalar) await access(new URL(`../${d}`, kok));
    const modul = await import(new URL(`../${m.yol}`, kok));
    assert.equal(modul.ad, m.ad);
    assert.equal(modul.baslik, m.baslik);
    assert.equal(typeof modul.ac, 'function');
    const mantik = await import(new URL(`../${m.yol.replace('index.js', 'mantik.js')}`, kok));
    assert.deepEqual([...m.izinler].sort(), Object.values(mantik.IZIN).sort(), `${m.ad} izin listesi`);
  }
});
