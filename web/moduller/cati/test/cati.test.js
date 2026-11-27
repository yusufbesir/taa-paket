// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// Çatı iskeleti testleri: yetki yoksa hiçbir şey gösterilmez/okunmaz; izne göre ekran; salt okunur.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as C from '../mantik.js';
import { ac, ad, baslik } from '../index.js';
import { sahteBelge, bekle } from '../../ortak/sinama/sahte-dom.js';
import { sahteBaglam } from '../../ortak/sinama/sahte-baglam.js';

const ekranKodlari = (kok) => kok.querySelectorAll('[data-ekran]').map((d) => d.getAttribute('data-ekran'));
async function kur(secenek = {}) {
  const { kok, pencere } = sahteBelge();
  const s = sahteBaglam(secenek);
  await ac(kok, s.baglam);
  return { kok, pencere, ...s };
}

test('gorunurEkranlar: varsayılan reddet; izne göre ekran', () => {
  const yetki = (...izinler) => (i) => izinler.includes(i);
  assert.deepEqual(C.gorunurEkranlar(yetki()), []);
  assert.deepEqual(C.gorunurEkranlar(yetki('kasa.masraf', 'rehber.oku')), [], 'yönetim dışı izin Çatı\'yı açmaz');
  assert.deepEqual(C.gorunurEkranlar(yetki('cihaz.yonet')).map((e) => e.kod), ['genel', 'cihazlar']);
  assert.deepEqual(C.gorunurEkranlar(yetki('kullanici.yonet')).map((e) => e.kod), ['genel', 'kullanicilar', 'subeler']);
  assert.deepEqual(C.gorunurEkranlar(yetki('program.yonet')).map((e) => e.kod), ['genel', 'yetkiler', 'pinler']);
  assert.deepEqual(C.gorunurEkranlar(yetki('gunluk.gor')).map((e) => e.kod), ['genel', 'gunluk']);
  assert.deepEqual(C.gorunurEkranlar(yetki('yonetici')).map((e) => e.kod), ['genel', 'raporlar']);
  assert.equal(C.gorunurEkranlar(() => 'evet').length, 0, 'yalnız kesin true izin sayılır');
});

test('tabloSatirlari: yalnız tanımlı sütunlar ekrana çıkar', () => {
  const e = C.EKRANLAR.find((x) => x.kod === 'pinler');
  assert.deepEqual(C.tabloSatirlari(e, [{ program: 'terzitakip', sube_id: null, belirlendi: true, pin: '4821', karma: 'x' }]), [['terzitakip', '', 'evet']]);
  assert.deepEqual(C.tabloSatirlari(e, undefined), []);
  assert.deepEqual(C.gidenSayilari([{ durum: 'bekliyor' }, { durum: 'bekliyor' }, { durum: 'reddedildi' }, { durum: 'uydurma' }, null]), { bekliyor: 2, gonderildi: 0, reddedildi: 1 });
});

test('modül: yetki yoksa ekran adı, menü ve veri yok; önbellek ve giden okunmaz', async () => {
  assert.equal(ad, 'cati');
  assert.equal(baslik, 'TAA Çatı');
  const { kok, baglam, okunan, sorulan } = await kur({ izinler: ['kasa.masraf', 'fis.oku'], onbellek: { 'cati/kullanicilar': [{ ad: 'Gizli Kişi' }] } });
  let listeCagrildi = false;
  baglam.giden.liste = async () => {
    listeCagrildi = true;
    return [];
  };
  const { kok: kok2 } = sahteBelge();
  await ac(kok2, baglam);
  for (const k of [kok, kok2]) {
    assert.ok(k.querySelector('[data-bolge="yetkisiz"]'));
    assert.deepEqual(ekranKodlari(k), []);
    assert.equal(k.querySelectorAll('button').length, 0);
    assert.doesNotMatch(k.textContent, /Kullanıcılar|Cihazlar|Günlük|Gizli Kişi/);
  }
  assert.deepEqual(okunan, []);
  assert.equal(listeCagrildi, false);
  assert.ok(sorulan.includes('cihaz.yonet') && sorulan.includes('yonetici'), 'izinler baglam.yetki ile sorulur');
});

test('modül: yalnız izinli ekranlar menüde; izinsiz ekranın verisi okunmaz', async () => {
  const { kok, okunan, kutu, yazilan } = await kur({
    izinler: ['cihaz.yonet'],
    onbellek: { 'cati/cihazlar': [{ ad: 'Kasa PC', sube: 'MRK', durum: 'bekliyor', anahtar: 'gizli' }], 'cati/kullanicilar': [{ ad: 'Gizli Kişi' }] },
  });
  assert.deepEqual(ekranKodlari(kok), ['genel', 'cihazlar']);
  assert.match(kok.querySelector('[data-bolge="giden-sayilari"]').textContent, /bekleyen 0, gönderilen 0, sunucunun reddettiği 0/);
  await kok.querySelector('[data-ekran="cihazlar"]').click();
  await bekle();
  const ekran = kok.querySelector('[data-bolge="ekran"]');
  assert.deepEqual(ekran.querySelectorAll('td').map((d) => d.textContent), ['Kasa PC', 'MRK', 'bekliyor']);
  assert.doesNotMatch(ekran.textContent, /gizli/);
  assert.ok(ekran.querySelector('[data-bolge="iskelet"]'), 'iskelet olduğu ekranda yazar');
  assert.equal(ekran.querySelectorAll('button').length, 0, 'iskelette işlem düğmesi yok');
  assert.deepEqual(okunan, ['cati/cihazlar']);
  assert.deepEqual(kutu, []);
  assert.deepEqual(yazilan, []);
});

test('modül: sunucudan gelen metin HTML olarak yorumlanmaz', async () => {
  const { kok } = await kur({ izinler: ['gunluk.gor'], onbellek: { 'cati/gunluk': [{ zaman: '2026-11-27 10:00', islem: '<script>alert(1)</script>', kisi: 'x' }] } });
  await kok.querySelector('[data-ekran="gunluk"]').click();
  await bekle();
  const hucreler = kok.querySelectorAll('td');
  assert.equal(hucreler[1].textContent, '<script>alert(1)</script>');
  assert.equal(kok.querySelectorAll('script').length, 0);
});

test('modül: izin oturum içinde kalkarsa ekran açılmaz', async () => {
  const { kok, izinKumesi, okunan } = await kur({ izinler: ['cihaz.yonet', 'kullanici.yonet'], onbellek: { 'cati/kullanicilar': [{ ad: 'Gizli Kişi' }] } });
  izinKumesi.delete('kullanici.yonet');
  await kok.querySelector('[data-ekran="kullanicilar"]').click();
  await bekle();
  assert.ok(kok.querySelector('[data-bolge="ekran"]').querySelector('[data-bolge="yetkisiz"]'));
  assert.doesNotMatch(kok.textContent, /Gizli Kişi/);
  assert.deepEqual(okunan, []);
});

test('modül: genel bakış giden kutusu sayılarını gösterir ve değişince yenilenir; veri inmemişse açıklar', async () => {
  const { kok, pencere, baglam } = await kur({ izinler: ['yonetici', 'gunluk.gor'] });
  await baglam.giden.ekle({ program: 'kasa', uc: '/v1/kasa/masraf', govde: {} });
  pencere.dispatchEvent({ type: 'taa-giden-degisti' });
  await bekle();
  assert.match(kok.querySelector('[data-bolge="giden-sayilari"]').textContent, /bekleyen 1/);
  await kok.querySelector('[data-ekran="gunluk"]').click();
  await bekle();
  assert.ok(kok.querySelector('[data-bolge="veri-yok"]'));
  await kok.querySelector('[data-ekran="raporlar"]').click();
  await bekle();
  assert.match(kok.querySelector('[data-bolge="ekran"]').textContent, /Yalnız yöneticiler/);
});
