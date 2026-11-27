// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// TerziTakip testleri: geçiş kuralı, süzgeçler, izin kapısı, giden kutusuna doğru uç/gövde.
import test from 'node:test';
import assert from 'node:assert/strict';
import * as T from '../mantik.js';
import { ac, ad, baslik } from '../index.js';
import { sahteBelge, bekle } from '../../ortak/sinama/sahte-dom.js';
import { sahteBaglam } from '../../ortak/sinama/sahte-baglam.js';
import { bugun } from '../../ortak/bicim.js';

const BUGUN = bugun();
const gun = (fark) => new Date(Date.parse(`${BUGUN}T00:00:00Z`) + fark * 86_400_000).toISOString().slice(0, 10);
/** Uydurma fişler (gerçek müşteri verisi değildir). */
const FISLER = () => [
  { no: 'S1-2026-000101', musteri: 'Örnek Müşteri A', telefon: '0500 000 00 01', teslim: gun(-2), durum: 'bekliyor', usta: 'Usta Bir', kargo: false, kalemler: [{ urun: 'Pantolon', islem: 'Paça kısalt' }], ucret_kurus: 20_000, on_odeme_kurus: 5_000 },
  { no: 'S1-2026-000102', musteri: 'Örnek Müşteri B', telefon: '0500 000 00 02', teslim: BUGUN, durum: 'terzihanede', kargo: true, kargo_ad: 'Alıcı B', kargo_adres: 'Örnek Mah. 1', kalemler: [{ urun: 'Ceket', islem: 'Kol daralt' }] },
  { no: 'S1-2026-000103', musteri: 'Çağrı Işık', telefon: '0500 000 00 03', teslim: gun(3), durum: 'hazir', raf: 'R-12', kalemler: [] },
  { no: 'S1-2026-000104', musteri: 'Teslim Edilmiş', telefon: '', teslim: gun(-5), durum: 'teslim', kargo: true, kalemler: [] },
  { no: 'S1-2026-000105', musteri: 'İptal Edilmiş', telefon: '', teslim: gun(-9), durum: 'iptal', kalemler: [] },
];
const TUM = ['fis.oku', 'fis.guncelle', 'fis.iptal'];
async function kur(secenek = {}) {
  const { kok, pencere } = sahteBelge();
  const s = sahteBaglam({ izinler: TUM, onbellek: { 'terzitakip/fisler': { fisler: FISLER() } }, ...secenek });
  await ac(kok, s.baglam);
  return { kok, pencere, ...s };
}
const kartlar = (kok) => kok.querySelectorAll('li[data-fis]').map((d) => d.getAttribute('data-fis'));
const dugme = (kok, no, yeni) => kok.querySelectorAll(`button[data-fis="${no}"]`).find((d) => d.getAttribute('data-yeni') === yeni);
const durum = (kok) => kok.querySelector('[data-bolge="durum"]');

test('geçiş kuralı: Bekliyor → (Terzihanede) → Hazır → Teslim; iptal teslimden önce; geri dönüş yok', () => {
  assert.deepEqual(T.gecisler('bekliyor'), ['terzihanede', 'hazir', 'iptal']);
  assert.deepEqual(T.gecisler('terzihanede'), ['hazir', 'iptal']);
  assert.deepEqual(T.gecisler('hazir'), ['teslim', 'iptal']);
  assert.deepEqual(T.gecisler('teslim'), []);
  assert.deepEqual(T.gecisler('iptal'), []);
  assert.equal(T.gecisIzinli('hazir', 'bekliyor'), false);
  assert.equal(T.gecisIzinli('bekliyor', 'teslim'), false);
  assert.equal(T.gecisIzinli('uydurma', 'hazir'), false);
});

test('süzgeçler: geciken, bugün teslim, kargo bekleyen, özet', () => {
  const f = FISLER();
  assert.deepEqual(T.geciken(f, BUGUN).map((x) => x.no), ['S1-2026-000101'], 'teslim ve iptal gecikmiş sayılmaz');
  assert.deepEqual(T.bugunTeslim(f, BUGUN).map((x) => x.no), ['S1-2026-000102']);
  assert.deepEqual(T.kargoBekleyen(f).map((x) => x.no), ['S1-2026-000102'], 'teslim edilmiş kargo beklemez');
  assert.deepEqual(T.ozet(f, BUGUN), { bekliyor: 1, terzihanede: 1, hazir: 1, geciken: 1, bugun_teslim: 1, kargo_bekleyen: 1 });
  assert.deepEqual(T.suz(f, 'acik', '', BUGUN).map((x) => x.no), ['S1-2026-000101', 'S1-2026-000102', 'S1-2026-000103'], 'teslim tarihine göre');
  assert.equal(T.suz(f, 'hepsi', '', BUGUN).length, 5);
  assert.deepEqual(T.suz(f, 'hepsi', 'cagri isik', BUGUN).map((x) => x.no), ['S1-2026-000103']);
  assert.deepEqual(T.suz(f, 'acik', 'paça', BUGUN).map((x) => x.no), ['S1-2026-000101']);
  assert.deepEqual(T.suz(f, 'acik', 'r-12', BUGUN).map((x) => x.no), ['S1-2026-000103']);
});

test('durumHazirla: geçersiz geçiş ve tarih reddedilir; gövde fis.guncelle@1', () => {
  const f = FISLER()[0];
  assert.deepEqual(T.durumHazirla(f, 'hazir', '2026-11-27'), {
    hatalar: [],
    govde: { sozlesme: 'fis.guncelle@1', fis_no: 'S1-2026-000101', durum: 'hazir', onceki_durum: 'bekliyor', tarih: '2026-11-27' },
  });
  assert.match(T.durumHazirla(f, 'teslim', '2026-11-27').hatalar[0], /Bekliyor durumundan Teslim edildi durumuna geçilemez/);
  assert.match(T.durumHazirla(f, 'uydurma', '2026-11-27').hatalar[0], /bilinmeyen durum/);
  assert.match(T.durumHazirla(f, 'hazir', '27.11.2026').hatalar[0], /tarih YYYY-AA-GG/);
  assert.match(T.durumHazirla(null, 'hazir', '2026-11-27').hatalar[0], /fiş seçilmedi/);
});

test('fisleriDuzelt ve bekleyenleriIsle', () => {
  assert.equal(T.fisleriDuzelt(undefined).length, 0);
  assert.equal(T.fisleriDuzelt([{ no: 'A', durum: 'hazir' }, { no: '', durum: 'hazir' }, { no: 'B', durum: 'uydurma' }, null]).length, 1);
  const l = T.bekleyenleriIsle(T.fisleriDuzelt(FISLER()), [
    { durum: 'bekliyor', uc: '/v1/fis/durum', govde: { fis_no: 'S1-2026-000101', durum: 'terzihanede' } },
    { durum: 'reddedildi', uc: '/v1/fis/durum', govde: { fis_no: 'S1-2026-000103', durum: 'teslim' } },
    { durum: 'bekliyor', uc: '/v1/baska', govde: { fis_no: 'S1-2026-000102', durum: 'hazir' } },
  ]);
  assert.equal(l[0].durum, 'terzihanede');
  assert.equal(l[0].bekleyen_durum, true);
  assert.equal(l[1].durum, 'terzihanede', 'başka ucun kaydı işlenmez');
  assert.equal(l[2].durum, 'hazir', 'reddedilen değişiklik uygulanmaz');
  assert.equal(l[2].reddedilen_durum, 'teslim');
});

test('modül: izin yoksa liste okunmaz', async () => {
  assert.equal(ad, 'terzitakip');
  assert.equal(baslik, 'TerziTakip');
  const { kok, okunan } = await kur({ izinler: [] });
  assert.ok(kok.querySelector('[data-bolge="yetkisiz"]'));
  assert.deepEqual(okunan, []);
  assert.deepEqual(kartlar(kok), []);
});

test('modül: liste, özet, süzgeç, arama; geciken ve kargo rozetleri', async () => {
  const { kok } = await kur();
  assert.deepEqual(kartlar(kok), ['S1-2026-000101', 'S1-2026-000102', 'S1-2026-000103']);
  assert.equal(kok.querySelector('[data-ozet="geciken"]').textContent, 'Geciken: 1');
  assert.equal(kok.querySelector('[data-ozet="kargo_bekleyen"]').textContent, 'Kargo bekleyen: 1');
  assert.ok(kok.querySelector('li[data-fis="S1-2026-000101"]').querySelector('[data-rozet="gecikti"]'));
  assert.equal(kok.querySelector('li[data-fis="S1-2026-000103"]').querySelector('[data-rozet="gecikti"]'), null);
  assert.ok(kok.querySelector('li[data-fis="S1-2026-000102"]').querySelector('[data-rozet="kargo"]'));
  await kok.querySelector('[data-suzgec="geciken"]').click();
  assert.deepEqual(kartlar(kok), ['S1-2026-000101']);
  await kok.querySelector('[data-suzgec="kargo"]').click();
  assert.deepEqual(kartlar(kok), ['S1-2026-000102']);
  await kok.querySelector('[data-suzgec="hepsi"]').click();
  assert.equal(kartlar(kok).length, 5);
  await kok.querySelector('[data-alan="fis-ara"]').yaz('ÇAĞRI');
  assert.deepEqual(kartlar(kok), ['S1-2026-000103']);
  await kok.querySelector('[data-alan="fis-ara"]').yaz('hiç yok');
  assert.ok(kok.querySelector('[data-bolge="bos"]'));
});

test('modül: durum değişikliği /v1/fis/durum ucuna yazılır ve liste bekleyeni gösterir', async () => {
  const { kok, kutu, yazilan } = await kur();
  await dugme(kok, 'S1-2026-000101', 'terzihanede').click();
  await bekle();
  assert.equal(kutu.length, 1);
  assert.deepEqual(kutu[0], {
    no: kutu[0].no,
    program: 'terzitakip',
    uc: '/v1/fis/durum',
    durum: 'bekliyor',
    govde: { sozlesme: 'fis.guncelle@1', fis_no: 'S1-2026-000101', durum: 'terzihanede', onceki_durum: 'bekliyor', tarih: BUGUN },
  });
  const kart = kok.querySelector('li[data-fis="S1-2026-000101"]');
  assert.equal(kart.getAttribute('data-durum'), 'terzihanede');
  assert.match(kart.textContent, /gönderilmeyi bekliyor/);
  assert.equal(kok.querySelectorAll('button[data-fis="S1-2026-000101"]').length, 0, 'bekleyen değişiklik varken yeni değişiklik düğmesi yok');
  assert.equal(durum(kok).getAttribute('data-durum'), 'tamam');
  assert.deepEqual(yazilan, [], 'önbelleğe yazılmaz');
});

test('modül: teslim ve iptal ikinci basışta yazılır (geri alınamaz)', async () => {
  const { kok, kutu } = await kur();
  await dugme(kok, 'S1-2026-000103', 'teslim').click();
  assert.equal(kutu.length, 0, 'ilk basış yalnız onay ister');
  assert.equal(durum(kok).getAttribute('data-durum'), 'uyari');
  assert.equal(dugme(kok, 'S1-2026-000103', 'teslim').textContent, 'Onayla: Teslim edildi');
  await dugme(kok, 'S1-2026-000103', 'teslim').click();
  await bekle();
  assert.equal(kutu.length, 1);
  assert.equal(kutu[0].govde.durum, 'teslim');
  assert.equal(kutu[0].govde.onceki_durum, 'hazir');
  await dugme(kok, 'S1-2026-000102', 'iptal').click();
  await dugme(kok, 'S1-2026-000102', 'hazir').click();
  await bekle();
  assert.equal(kutu.length, 2, 'başka düğmeye basınca iptal onayı düşer');
  assert.equal(kutu[1].govde.durum, 'hazir');
});

test('modül: güncelleme izni yoksa düğme yok; iptal izni yoksa iptal düğmesi yok', async () => {
  const salt = await kur({ izinler: ['fis.oku'] });
  assert.equal(salt.kok.querySelectorAll('[data-is="durum"]').length, 0);
  assert.ok(salt.kok.querySelector('[data-bolge="salt-okunur"]'));
  const iptalsiz = await kur({ izinler: ['fis.oku', 'fis.guncelle'] });
  assert.ok(dugme(iptalsiz.kok, 'S1-2026-000101', 'hazir'));
  assert.equal(dugme(iptalsiz.kok, 'S1-2026-000101', 'iptal'), undefined);
});

test('modül: önbellek boşken açıklama gösterir; giden değişince yenilenir', async () => {
  const bos = await kur({ onbellek: {} });
  assert.match(bos.kok.querySelector('[data-bolge="bos"]').textContent, /Bu cihazda fiş yok/);
  const { kok, pencere, baglam } = await kur();
  await baglam.giden.ekle({ program: 'terzitakip', uc: '/v1/fis/durum', govde: { fis_no: 'S1-2026-000102', durum: 'hazir' } });
  pencere.dispatchEvent({ type: 'taa-giden-degisti' });
  await bekle();
  assert.equal(kok.querySelector('li[data-fis="S1-2026-000102"]').getAttribute('data-durum'), 'hazir');
});
