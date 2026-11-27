// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// Rehber testleri: salt okuma, arama, izin kapısı, tam çevrimdışı (yalnız önbellek; giden kutusuna yazı yok).
import test from 'node:test';
import assert from 'node:assert/strict';
import * as R from '../mantik.js';
import { ac, ad, baslik } from '../index.js';
import { sahteBelge } from '../../ortak/sinama/sahte-dom.js';
import { sahteBaglam } from '../../ortak/sinama/sahte-baglam.js';

/** Uydurma rehber (gerçek kişi verisi değildir). */
const VERI = () => ({
  kisiler: [
    { id: 'K-1', ad: 'Çağrı Işık', eposta: 'cagri@ornek.test', durum: 'etkin', surum: 1 },
    { id: 'K-2', ad: 'Ayşe Örnek', eposta: 'ayse@ornek.test', durum: 'etkin', surum: 1 },
    { id: 'K-3', ad: 'Eski Personel', eposta: 'eski@ornek.test', durum: 'pasif', surum: 2 },
  ],
  // Rust tarafı BTreeMap yazar: kimlik → kayıt nesnesi de kabul edilir.
  birimler: {
    'B-1': { id: 'B-1', ad: 'Merkez', tur: 'merkez', durum: 'etkin' },
    'B-2': { id: 'B-2', ad: 'İstanbul Şubesi', tur: 'sube', ust_birim_id: 'B-1', sube_kodu: 'IST', ortak_eposta: 'ist@ornek.test', durum: 'etkin' },
    'B-3': { id: 'B-3', ad: 'Kapanan Mağaza', tur: 'magaza', durum: 'arsiv' },
  },
  gorevlendirmeler: [
    { id: 'G-1', kisi_id: 'K-1', birim_id: 'B-2', rol: 'mudur', gorev: 'Mağaza Müdürü', amir_id: 'K-2', tur: 'asil', durum: 'etkin' },
    { id: 'G-2', kisi_id: 'K-2', birim_id: 'B-1', rol: 'yonetici', tur: 'asil', durum: 'etkin' },
    { id: 'G-3', kisi_id: 'K-2', birim_id: 'B-2', rol: 'mudur', tur: 'vekil', durum: 'pasif' },
    { id: 'G-4', kisi_id: 'K-3', birim_id: 'B-2', rol: 'personel', tur: 'asil', durum: 'etkin' },
  ],
  roller: [
    { kod: 'mudur', ad: 'Mağaza Müdürü', durum: 'etkin' },
    { kod: 'yonetici', ad: 'Yönetici', durum: 'etkin' },
  ],
});
async function kur(secenek = {}) {
  const { kok } = sahteBelge();
  const s = sahteBaglam({ izinler: ['rehber.oku'], onbellek: { 'rehber/veri': VERI() }, ...secenek });
  await ac(kok, s.baglam);
  return { kok, ...s };
}
const kisiler = (kok) => kok.querySelectorAll('li[data-kisi]').map((d) => d.getAttribute('data-kisi'));
const birimler = (kok) => kok.querySelectorAll('li[data-birim]').map((d) => d.getAttribute('data-birim'));

test('kisiSatirlari: pasif kişi ve bitmiş görev varsayılan olarak gizli; amir yalnız izinle', () => {
  const v = R.veriDuzelt(VERI());
  const s = R.kisiSatirlari(v);
  assert.deepEqual(s.map((k) => k.ad), ['Ayşe Örnek', 'Çağrı Işık'], 'Türkçe sıralama, pasif yok');
  assert.deepEqual(s[1].gorevler, [{ birim_id: 'B-2', birim: 'İstanbul Şubesi', rol: 'Mağaza Müdürü', gorev: 'Mağaza Müdürü', tur: 'asil', amir: '' }]);
  assert.equal(s[0].gorevler.length, 1, 'pasif vekil görevi gizli');
  const tum = R.kisiSatirlari(v, { pasifler: true, amir: true });
  assert.equal(tum.length, 3);
  assert.equal(tum.find((k) => k.id === 'K-1').gorevler[0].amir, 'Ayşe Örnek');
  assert.equal(tum.find((k) => k.id === 'K-3').pasif, true);
  assert.equal(tum.find((k) => k.id === 'K-2').gorevler.length, 2);
});

test('birimSatirlari: üst birim, etkin kişi sayısı, arşiv gizli', () => {
  const s = R.birimSatirlari(R.veriDuzelt(VERI()));
  assert.deepEqual(s.map((b) => b.ad), ['İstanbul Şubesi', 'Merkez']);
  assert.equal(s[0].ust, 'Merkez');
  assert.equal(s[0].kisi_sayisi, 1, 'pasif kişi ve pasif görev sayılmaz');
  assert.equal(R.birimSatirlari(R.veriDuzelt(VERI()), { pasifler: true }).length, 3);
});

test('arama: ad, e-posta, birim, rol; Türkçe harf farkı gözetilmez', () => {
  const s = R.kisiSatirlari(R.veriDuzelt(VERI()));
  assert.deepEqual(R.kisiAra(s, 'cagri isik').map((k) => k.id), ['K-1']);
  assert.deepEqual(R.kisiAra(s, 'ISTANBUL').map((k) => k.id), ['K-1']);
  assert.deepEqual(R.kisiAra(s, 'yönetici').map((k) => k.id), ['K-2']);
  assert.deepEqual(R.kisiAra(s, 'ayse@ornek').map((k) => k.id), ['K-2']);
  assert.equal(R.kisiAra(s, '   ').length, 2);
  assert.equal(R.kisiAra(s, 'yok böyle biri').length, 0);
  const b = R.birimSatirlari(R.veriDuzelt(VERI()));
  assert.deepEqual(R.birimAra(b, 'ist').map((x) => x.id), ['B-2']);
  assert.deepEqual(R.birimAra(b, 'sube merkez').map((x) => x.id), ['B-2'], 'üst birim adıyla');
});

test('veriDuzelt: bozuk ya da eksik veri çökmez', () => {
  assert.deepEqual(R.veriDuzelt(undefined), { kisiler: [], birimler: [], gorevlendirmeler: [], roller: [] });
  assert.equal(R.veriDuzelt({ kisiler: [{ id: 'K', ad: 'A' }, { ad: 'kimliksiz' }, null, 'x'] }).kisiler.length, 1);
});

test('modül: izin yoksa önbellek okunmaz', async () => {
  assert.equal(ad, 'rehber');
  assert.equal(baslik, 'Rehber');
  const { kok, okunan } = await kur({ izinler: [] });
  assert.ok(kok.querySelector('[data-bolge="yetkisiz"]'));
  assert.deepEqual(okunan, []);
  assert.deepEqual(kisiler(kok), []);
});

test('modül: kişi listesi, arama, birim görünümü', async () => {
  const { kok } = await kur();
  assert.deepEqual(kisiler(kok), ['K-2', 'K-1']);
  assert.doesNotMatch(kok.textContent, /Eski Personel/);
  assert.doesNotMatch(kok.textContent, /amir:/);
  await kok.querySelector('[data-alan="rehber-ara"]').yaz('ışık');
  assert.deepEqual(kisiler(kok), ['K-1']);
  assert.equal(kok.querySelector('[data-bolge="sayi"]').textContent, '1 kayıt');
  await kok.querySelector('[data-alan="rehber-ara"]').yaz('zzz');
  assert.match(kok.querySelector('[data-bolge="bos"]').textContent, /Aramaya uyan kayıt yok/);
  await kok.querySelector('[data-alan="rehber-ara"]').yaz('');
  await kok.querySelector('[data-gorunum="birim"]').click();
  assert.deepEqual(birimler(kok), ['B-2', 'B-1']);
  assert.match(kok.querySelector('li[data-birim="B-2"]').textContent, /üst birim: Merkez · şube: IST · 1 kişi/);
});

test('modül: rehber.tum.oku pasifleri, rehber.amir.oku amiri gösterir', async () => {
  const { kok } = await kur({ izinler: ['rehber.oku', 'rehber.tum.oku', 'rehber.amir.oku'] });
  assert.deepEqual(kisiler(kok), ['K-2', 'K-1', 'K-3']);
  assert.match(kok.querySelector('li[data-kisi="K-3"]').textContent, /pasif/);
  assert.match(kok.querySelector('li[data-kisi="K-1"]').textContent, /amir: Ayşe Örnek/);
});

test('modül: salt okunur ve tam çevrimdışı (yalnız önbellek okunur, hiçbir şey yazılmaz)', async () => {
  const { kok, baglam, kutu, okunan, yazilan } = await kur();
  baglam.giden.ekle = async () => {
    throw new Error('rehber giden kutusuna yazmamalı');
  };
  baglam.giden.liste = async () => {
    throw new Error('rehber giden kutusunu okumamalı');
  };
  const asilFetch = globalThis.fetch;
  globalThis.fetch = () => {
    throw new Error('rehber ağ çağrısı yapmamalı');
  };
  try {
    const { kok: kok2 } = sahteBelge();
    await ac(kok2, baglam);
    await kok2.querySelector('[data-alan="rehber-ara"]').yaz('ayşe');
    await kok2.querySelector('[data-gorunum="birim"]').click();
    assert.deepEqual(birimler(kok2), []);
  } finally {
    globalThis.fetch = asilFetch;
  }
  assert.deepEqual(kutu, []);
  assert.deepEqual(yazilan, []);
  assert.deepEqual([...new Set(okunan)], ['rehber/veri']);
  assert.equal(kok.querySelectorAll('input').length, 1, 'tek giriş alanı arama kutusudur');
});

test('modül: rehber inmemişse açıklama gösterir', async () => {
  const { kok } = await kur({ onbellek: {} });
  assert.match(kok.querySelector('[data-bolge="bos"]').textContent, /henüz inmedi/);
});
