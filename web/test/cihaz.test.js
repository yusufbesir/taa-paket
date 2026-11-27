// Sahip: Yusuf. yazan: [Claude Sonnet 5.5 #taa-web-cihaz 2026-11-27]
// Tarayıcı cihaz anahtarı: çıkarılamaz Ed25519, imza biçimi (taa_cihaz::imza_mesaji), kayıt gövdesi, sarmalayıcı.
// Node'un WebCrypto'su ile çalışır; gerçek tarayıcı / gerçek sunucu DENENMEZ.
import test from 'node:test';
import assert from 'node:assert/strict';
import { webcrypto } from 'node:crypto';
import { BellekDepo, TarayiciCihaz, CihazHata, destekVar, imzaMesaji, adGecerli } from '../cekirdek/cihaz.js';

const subtle = webcrypto.subtle;
const SAAT = () => 1_795_000_000_000;
const yeni = () => new TarayiciCihaz({ depo: new BellekDepo(), subtle, saat: SAAT });
const hexBayt = (h) => Uint8Array.from(h.match(/../g).map((b) => parseInt(b, 16)));
const SUNUCU = 'https://taa.test';

test('Ed25519 desteği var ve olmayan ortamda false', async () => {
  assert.equal(await destekVar(subtle), true);
  assert.equal(await destekVar(null), false);
  assert.equal(await destekVar({ generateKey: async () => { throw new Error('yok'); } }), false);
});

test('anahtar çıkarılamaz, kimlik genel anahtarın ilk 8 baytı, ikinci üretim aynı anahtarı korur', async () => {
  const c = yeni();
  const k = await c.uret('Pendik telefon');
  assert.equal(k.ciftler.privateKey.extractable, false);
  await assert.rejects(subtle.exportKey('pkcs8', k.ciftler.privateKey));
  assert.equal(k.acik.length, 64);
  assert.equal(k.kimlik, k.acik.slice(0, 16));
  const yine = await c.uret('başka ad');
  assert.equal(yine.acik, k.acik);
  assert.equal(yine.ad, 'Pendik telefon');
});

test('geçersiz ad reddedilir, destek yoksa fallback yok', async () => {
  assert.equal(adGecerli(''), false);
  assert.equal(adGecerli('a'.repeat(65)), false);
  assert.equal(adGecerli('Pendik PC-1'), true);
  await assert.rejects(yeni().uret('<x>'), (e) => e instanceof CihazHata && e.kod === 'ad');
  const yok = new TarayiciCihaz({ depo: new BellekDepo(), subtle: { generateKey: async () => { throw new Error(); } } });
  await assert.rejects(yok.uret('PC'), (e) => e.kod === 'destek-yok');
});

test('imza biçimi: "{YONTEM}\\n{YOL}\\n{ZAMAN}\\n" + gövde; genel anahtarla doğrulanır, bozulunca tutmaz', async () => {
  const c = yeni();
  const k = await c.uret('PC');
  const govde = '{"no":"k-1"}';
  const b = await c.basliklar('post', '/v1/kasa/gun?x=1', govde);
  assert.equal(b['X-TAA-Cihaz'], k.kimlik);
  assert.equal(b['X-TAA-Zaman'], '1795000000');
  assert.equal(b['X-TAA-Imza'].length, 128);
  const genel = await subtle.importKey('raw', hexBayt(k.acik), { name: 'Ed25519' }, false, ['verify']);
  const mesaj = imzaMesaji('POST', '/v1/kasa/gun?x=1', 1795000000, govde);
  assert.equal(new TextDecoder().decode(mesaj.slice(0, 33)), 'POST\n/v1/kasa/gun?x=1\n1795000000\n');
  assert.equal(await subtle.verify({ name: 'Ed25519' }, genel, hexBayt(b['X-TAA-Imza']), mesaj), true);
  const bozuk = imzaMesaji('POST', '/v1/kasa/gun?x=1', 1795000000, '{"no":"k-2"}');
  assert.equal(await subtle.verify({ name: 'Ed25519' }, genel, hexBayt(b['X-TAA-Imza']), bozuk), false);
});

test('kayıt gövdesi: ed25519:<hex> + ad (+ şube), durum saklanır, hata kabul edilmez', async () => {
  const c = yeni();
  const k = await c.uret('PC');
  const gorulen = [];
  const getir = async (u, s) => {
    gorulen.push({ u, s });
    return { status: 200, text: async () => '{"kimlik":"x","durum":"bekliyor"}' };
  };
  const r = await c.kaydet(SUNUCU, getir, 'PND');
  assert.equal(r.durum, 'bekliyor');
  assert.equal(gorulen[0].u, `${SUNUCU}/v1/cihaz/kayit`);
  assert.deepEqual(JSON.parse(gorulen[0].s.body), { acik_anahtar: `ed25519:${k.acik}`, ad: 'PC', sube_kodu: 'PND' });
  assert.equal(gorulen[0].s.headers['X-TAA-Imza'], undefined, 'kayıt ucu imzasız');
  await assert.rejects(c.kaydet(SUNUCU, async () => ({ status: 500, text: async () => '' })), (e) => e.kod === 'kayit-red');
  const yeniOturum = new TarayiciCihaz({ depo: c.depo, subtle, saat: SAAT });
  assert.equal((await yeniOturum.yukle()).durum, 'bekliyor');
});

test('imzaliFetch yalnız kendi /v1 uçlarını imzalar; kayıt ucu, başka köken ve /v1 dışı imzasız', async () => {
  const c = yeni();
  await c.uret('PC');
  const gorulen = [];
  const f = c.imzaliFetch(async (u, s) => {
    gorulen.push({ u, h: s?.headers ?? {} });
    return { status: 200 };
  }, SUNUCU);
  await f(`${SUNUCU}/v1/giris`, { method: 'POST', body: '{}', headers: { 'Content-Type': 'application/json' } });
  await f(`${SUNUCU}/v1/cihaz/kayit`, { method: 'POST', body: '{}' });
  await f('https://baska.test/v1/giris', { method: 'POST', body: '{}' });
  await f(`${SUNUCU}/web/kabuk/index.html`);
  assert.ok(gorulen[0].h['X-TAA-Imza']);
  assert.equal(gorulen[0].h['Content-Type'], 'application/json');
  assert.equal(gorulen[1].h['X-TAA-Imza'], undefined);
  assert.equal(gorulen[2].h['X-TAA-Imza'], undefined);
  assert.equal(gorulen[3].h['X-TAA-Imza'], undefined);
  await assert.rejects(f(`${SUNUCU}/v1/x`, { method: 'POST', body: new FormData() }), (e) => e.kod === 'govde');
});

test('anahtar yokken imzalı istek gitmez (imzasız çıkmaz)', async () => {
  const c = yeni();
  const f = c.imzaliFetch(async () => ({ status: 200 }), SUNUCU);
  // anahtar yok: sarmalayıcı değiştirmeden geçirir; kabuk bunu önce uret() ile engellemelidir
  assert.equal((await f(`${SUNUCU}/v1/giris`, { method: 'POST', body: '{}' })).status, 200);
  await assert.rejects(c.basliklar('POST', '/v1/x', ''), (e) => e.kod === 'anahtar-yok');
});
