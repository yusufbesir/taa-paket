// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// Kasa modülü DOM testi (sahte DOM + sahte baglam): izin kapısı, giden kutusuna doğru uç/gövde, fark notu, imza.
import test from 'node:test';
import assert from 'node:assert/strict';
import { ac, ad, baslik } from '../index.js';
import { AYAR } from '../sinama/ornek.js';
import { sahteBelge, bekle } from '../../ortak/sinama/sahte-dom.js';
import { sahteBaglam } from '../../ortak/sinama/sahte-baglam.js';
import { bugun } from '../../ortak/bicim.js';

const TUM = ['kasa.sayim', 'kasa.masraf', 'kasa.gunsonu', 'kasa.imza'];
const al = (kok, a) => {
  const d = kok.querySelector(`[data-alan="${a}"]`);
  assert.ok(d, `alan yok: ${a}`);
  return d;
};
const dugme = (kok, is) => {
  const d = kok.querySelector(`[data-is="${is}"]`);
  assert.ok(d, `düğme yok: ${is}`);
  return d;
};
const durum = (kok) => kok.querySelector('[data-bolge="durum"]');
async function kur(secenek = {}) {
  const { kok, pencere } = sahteBelge();
  const s = sahteBaglam({ izinler: TUM, onbellek: { 'kasa/ayar': AYAR }, ...secenek });
  await ac(kok, s.baglam);
  return { kok, pencere, ...s };
}
async function sekme(kok, kod) {
  await kok.querySelector(`[data-sekme="${kod}"]`).click();
  await bekle();
}

test('modül sözleşmesi: ad, baslik, ac', () => {
  assert.equal(ad, 'kasa');
  assert.equal(baslik, 'TAA Kasa');
  assert.equal(typeof ac, 'function');
});

test('izin yoksa hiçbir şey okunmaz ve form çıkmaz', async () => {
  const { kok, okunan, kutu } = await kur({ izinler: [] });
  assert.ok(kok.querySelector('[data-bolge="yetkisiz"]'));
  assert.equal(kok.querySelectorAll('button').length, 0);
  assert.deepEqual(okunan, []);
  assert.deepEqual(kutu, []);
});

test('yalnız izinli bölümler görünür', async () => {
  const { kok, sorulan } = await kur({ izinler: ['kasa.masraf'] });
  assert.deepEqual(kok.querySelectorAll('[data-sekme]').map((d) => d.getAttribute('data-sekme')), ['masraf']);
  assert.deepEqual(sorulan, TUM, 'her izin baglam.yetki ile sorulur');
  assert.ok(kok.querySelector('[data-is="masraf-kaydet"]'));
});

test('kasa ayarı inmemişse kayıt alınmaz', async () => {
  const { kok } = await kur({ onbellek: {} });
  assert.ok(kok.querySelector('[data-bolge="ayar-yok"]'));
  assert.equal(kok.querySelectorAll('button').length, 0);
});

test('masraf: geçerli kayıt /v1/kasa/masraf ucuna kasa.masraf@1 gövdesiyle yazılır', async () => {
  const { kok, kutu } = await kur();
  await sekme(kok, 'masraf');
  al(kok, 'masraf-tur').value = 'Kargo';
  al(kok, 'masraf-tutar').value = '1.250,50';
  al(kok, 'masraf-aciklama').value = 'Yurtiçi kargo';
  al(kok, 'masraf-not').value = 'İzmir şubesine';
  await dugme(kok, 'masraf-kaydet').click();
  assert.equal(kutu.length, 1);
  assert.deepEqual(kutu[0], {
    no: kutu[0].no,
    program: 'kasa',
    uc: '/v1/kasa/masraf',
    durum: 'bekliyor',
    govde: {
      sozlesme: 'kasa.masraf@1',
      tur: 'masraf',
      v: { tarih: bugun(), cekmece: 'C1', tur: 'Kargo', tutar_kurus: 125_050, aciklama: 'Yurtiçi kargo', not: 'İzmir şubesine', onay_duzeyi: 'mudur' },
    },
  });
  assert.equal(durum(kok).getAttribute('data-durum'), 'tamam');
  assert.match(durum(kok).textContent, new RegExp(kutu[0].no));
  assert.equal(al(kok, 'masraf-tutar').value, '', 'form temizlenir');
  assert.match(kok.querySelector('[data-bolge="masraf-bekleyen"]').textContent, /Yurtiçi kargo/);
});

test('masraf: geçersiz girişte giden kutusuna yazılmaz, hatalar gösterilir', async () => {
  const { kok, kutu } = await kur();
  await sekme(kok, 'masraf');
  await dugme(kok, 'masraf-kaydet').click();
  assert.equal(kutu.length, 0);
  assert.equal(durum(kok).getAttribute('data-durum'), 'hata');
  assert.match(durum(kok).textContent, /masraf tutarı girin/);
  assert.match(durum(kok).textContent, /masraf türü gerekli/);
  assert.match(durum(kok).textContent, /masraf açıklaması gerekli/);
  al(kok, 'masraf-tutar').value = '12,345';
  await dugme(kok, 'masraf-kaydet').click();
  assert.equal(kutu.length, 0);
  assert.match(durum(kok).textContent, /tutar okunamadı/);
});

test('sayım: açılış /v1/kasa/gunsonu ucuna yazılır; ikinci açılış engellenir', async () => {
  const { kok, kutu } = await kur({ onbellek: { 'kasa/ayar': AYAR, [`kasa/gun/${bugun()}/C1`]: { onceki_devreden_kurus: 50_000 } } });
  await al(kok, 'sayim-adet-20000').yaz('2');
  await al(kok, 'sayim-adet-10000').yaz('1');
  await bekle();
  assert.match(kok.querySelector('[data-bolge="sayim-bilgi"]').textContent, /Sayılan: 500,00 TL\. Beklenen: 500,00 TL\. Fark: 0,00 TL/);
  await dugme(kok, 'sayim-kaydet').click();
  assert.equal(kutu.length, 1);
  assert.equal(kutu[0].uc, '/v1/kasa/gunsonu');
  assert.deepEqual(kutu[0].govde, {
    sozlesme: 'kasa.gunsonu@1',
    tur: 'sayim',
    v: { tur: 'acilis', tarih: bugun(), cekmece: 'C1', ikinci_kisi: null, adetler: { 10000: 1, 20000: 2 }, sayilan_kurus: 50_000, beklenen_kurus: 50_000, fark_kurus: 0, not: '' },
  });
  await al(kok, 'sayim-adet-20000').yaz('2');
  await dugme(kok, 'sayim-kaydet').click();
  assert.equal(kutu.length, 1, 'aynı gün ve çekmeceye ikinci açılış yazılmaz');
  assert.match(durum(kok).textContent, /açılış zaten var/);
});

test('sayım: fark varsa not zorunlu; ara sayımda ikinci kişi zorunlu ve girenden farklı', async () => {
  const { kok, kutu } = await kur({ kullanici: { ad: 'Ayşe' }, onbellek: { 'kasa/ayar': AYAR, [`kasa/gun/${bugun()}/C1`]: { onceki_devreden_kurus: 50_000, acilis_kurus: 50_000, masraf_kurus: 0 } } });
  await al(kok, 'sayim-tur').sec('ara');
  await al(kok, 'sayim-adet-20000').yaz('2');
  await dugme(kok, 'sayim-kaydet').click();
  assert.equal(kutu.length, 0);
  assert.match(durum(kok).textContent, /ikinci kişi gerekli/);
  assert.match(durum(kok).textContent, /fark var; açıklama notu yazın/);
  al(kok, 'sayim-ikinci').value = 'ayşe';
  al(kok, 'sayim-not').value = '100 TL bozukluk alındı';
  await dugme(kok, 'sayim-kaydet').click();
  assert.equal(kutu.length, 0);
  assert.match(durum(kok).textContent, /girenden farklı olmalı/);
  al(kok, 'sayim-ikinci').value = 'Mehmet';
  await dugme(kok, 'sayim-kaydet').click();
  assert.equal(kutu.length, 1);
  assert.equal(kutu[0].govde.v.tur, 'ara');
  assert.equal(kutu[0].govde.v.ikinci_kisi, 'Mehmet');
  assert.equal(kutu[0].govde.v.fark_kurus, -10_000);
});

/** Farksız örnek günü forma yazar (açılış 500 TL, masraf 100 TL önbellekte). */
async function gunSonuDoldur(kok) {
  await sekme(kok, 'gunsonu');
  const yaz = (a, v) => {
    al(kok, a).value = v;
  };
  yaz('gs-adet-20000', '7');
  yaz('gs-adet-10000', '1');
  yaz('gs-devreden', '500');
  yaz('gs-z-N1', '2.500,00');
  yaz('gs-pos-N1-Z1', '800');
  yaz('gs-pos-N1-I1', '400');
  yaz('gs-z-N2', '1.100,00');
  yaz('gs-pos-N2-I2', '300');
  yaz('gs-nebim', '3.600,00');
  yaz('gs-ciro-nakit', '1.000');
  yaz('gs-ciro-ziraat_pos', '800');
  yaz('gs-ciro-iyzico_pos', '700');
  yaz('gs-ciro-havale', '1.000');
  for (let i = 0; i < 7; i += 1) al(kok, `gs-kontrol-${i}`).checked = true;
  await al(kok, 'gs-nebim').yaz('3.600,00');
}
const GUN_ONBELLEK = () => ({ 'kasa/ayar': AYAR, [`kasa/gun/${bugun()}/C1`]: { acilis_kurus: 50_000, masraf_kurus: 10_000 } });

test('gün sonu: farksız gün kasa.gunsonu@1 gövdesiyle yazılır; canlı panel tutar', async () => {
  const { kok, kutu } = await kur({ onbellek: GUN_ONBELLEK() });
  await gunSonuDoldur(kok);
  const panel = kok.querySelector('[data-bolge="fark-paneli"]');
  assert.equal(panel.querySelectorAll('tr[data-fark="var"]').length, 0);
  assert.equal(panel.querySelectorAll('tr[data-fark="yok"]').length, 5);
  assert.equal(panel.querySelector('[data-uyari="kasa-ciro"]'), null);
  assert.match(kok.querySelector('[data-bolge="cift-sonuc"]').textContent, /Fark yoktur\./);
  await dugme(kok, 'gunsonu-kaydet').click();
  assert.equal(kutu.length, 1, durum(kok).textContent);
  assert.equal(kutu[0].program, 'kasa');
  assert.equal(kutu[0].uc, '/v1/kasa/gunsonu');
  const g = kutu[0].govde;
  assert.equal(g.sozlesme, 'kasa.gunsonu@1');
  assert.equal(g.tur, 'gun_sonu');
  assert.equal(g.v.tarih, bugun());
  assert.equal(g.v.cekmece, 'C1');
  assert.equal(g.v.sube, 'MRK');
  assert.equal(g.v.nakit_sayilan_kurus, 150_000);
  assert.deepEqual(g.v.noktalar, [
    { nokta: 'N1', z_toplam_kurus: 250_000, pos: { Z1: 80_000, I1: 40_000 } },
    { nokta: 'N2', z_toplam_kurus: 110_000, pos: { I2: 30_000 } },
  ]);
  assert.equal(g.v.nebim_toplam_kurus, 360_000);
  assert.equal(g.v.ciro.nakit, 100_000);
  assert.equal(g.v.devreden_kurus, 50_000);
  assert.deepEqual(g.v.kontrol, [true, true, true, true, true, true, true]);
  assert.equal(g.v.karsilastirma.fark_var, false);
  assert.equal(g.v.karsilastirma.ciro_toplam_kurus, 360_000);
  assert.equal(g.v.acilis_kurus, 50_000);
  assert.equal(g.v.masraf_kurus, 10_000);
  await dugme(kok, 'gunsonu-kaydet').click();
  assert.equal(kutu.length, 1, 'aynı gün ve çekmeceye ikinci gün sonu yazılmaz');
  assert.match(durum(kok).textContent, /gün sonu zaten var/);
});

test('gün sonu: fark varken not yoksa yazılmaz (fark notu zorunlu); notla yazılır', async () => {
  const { kok, kutu } = await kur({ onbellek: GUN_ONBELLEK() });
  await gunSonuDoldur(kok);
  await al(kok, 'gs-adet-10000').yaz('0');
  await al(kok, 'gs-adet-5000').yaz('1');
  const panel = kok.querySelector('[data-bolge="fark-paneli"]');
  assert.equal(panel.querySelectorAll('tr[data-fark="var"]').length, 1);
  assert.ok(panel.querySelector('[data-uyari="kasa-ciro"]'), '"kasa ile ciro uyuşmuyor" uyarısı');
  assert.ok(panel.querySelector('[data-uyari="fark-notu"]'));
  assert.match(kok.querySelector('[data-bolge="cift-sonuc"]').textContent, /Fark tespit edilmiştir\./);
  await dugme(kok, 'gunsonu-kaydet').click();
  assert.equal(kutu.length, 0);
  assert.equal(durum(kok).getAttribute('data-durum'), 'hata');
  assert.match(durum(kok).textContent, /karşılaştırmada fark var; açıklama notu yazın/);
  al(kok, 'gs-not').value = '50 TL sayım farkı, tutanak tutuldu';
  await dugme(kok, 'gunsonu-kaydet').click();
  assert.equal(kutu.length, 1);
  assert.equal(kutu[0].govde.v.not, '50 TL sayım farkı, tutanak tutuldu');
  assert.equal(kutu[0].govde.v.ciro.kasa_acigi, 5_000);
  assert.equal(kutu[0].govde.v.karsilastirma.kasa_ciro_uyusmuyor, true);
});

test('gün sonu: bekleyen masraf karşılaştırmaya girer; okunamayan tutar kaydı durdurur', async () => {
  const { kok, kutu, baglam } = await kur({ onbellek: { 'kasa/ayar': AYAR, [`kasa/gun/${bugun()}/C1`]: { acilis_kurus: 50_000, masraf_kurus: 0 } } });
  await baglam.giden.ekle({ program: 'kasa', uc: '/v1/kasa/masraf', govde: { sozlesme: 'kasa.masraf@1', tur: 'masraf', v: { tarih: bugun(), cekmece: 'C1', tutar_kurus: 10_000 } } });
  await gunSonuDoldur(kok);
  al(kok, 'gs-ciro-havale').value = 'bin lira';
  await dugme(kok, 'gunsonu-kaydet').click();
  assert.equal(kutu.length, 1, 'yalnız önceden eklenen masraf');
  assert.match(durum(kok).textContent, /BANKAYA HAVALE.*tutar okunamadı/);
  al(kok, 'gs-ciro-havale').value = '1.000';
  await dugme(kok, 'gunsonu-kaydet').click();
  assert.equal(kutu.length, 2, durum(kok).textContent);
  assert.equal(kutu[1].govde.v.masraf_kurus, 10_000);
  assert.equal(kutu[1].govde.v.karsilastirma.fark_var, false);
});

test('imza (çift doğrulama): 1. imza yazılır, aynı oturumdan 2. imza engellenir, gerekçe yazılır', async () => {
  const { kok, kutu } = await kur({ onbellek: GUN_ONBELLEK() });
  await gunSonuDoldur(kok);
  await dugme(kok, 'gunsonu-kaydet').click();
  await sekme(kok, 'imza');
  const no = kutu[0].no;
  assert.ok(kok.querySelector(`[data-gun="${no}"]`), 'bekleyen gün sonu imza listesinde');
  await dugme(kok, 'imzala').click();
  assert.equal(kutu.length, 2);
  assert.equal(kutu[1].uc, '/v1/kasa/gunsonu');
  assert.deepEqual(kutu[1].govde, { sozlesme: 'kasa.gunsonu@1', tur: 'imza', v: { sira: 1, rol: 'kasa_sorumlusu', kanal: 'ekran', istemci_no: no } });
  await dugme(kok, 'imzala').click();
  assert.equal(kutu.length, 2, 'aynı oturumdan ikinci imza yazılmaz');
  assert.match(durum(kok).textContent, /aynı kişi iki kez imzalayamaz/);
  al(kok, `gerekce-${no}`).value = 'yok';
  await dugme(kok, 'gerekce-kaydet').click();
  assert.equal(kutu.length, 2);
  assert.match(durum(kok).textContent, /5–300 karakter/);
  al(kok, `gerekce-${no}`).value = 'Mağaza sorumlusu izinli';
  await dugme(kok, 'gerekce-kaydet').click();
  assert.equal(kutu.length, 3);
  assert.deepEqual(kutu[2].govde, { sozlesme: 'kasa.gunsonu@1', tur: 'gerekce', v: { gerekce: 'Mağaza sorumlusu izinli', istemci_no: no } });
  assert.match(kok.querySelector(`[data-gun="${no}"]`).textContent, /ikinci imza atmadı \(gerekçe: Mağaza sorumlusu izinli\)/);
});

test('imza: sunucudan inen günde ikinci imzayı giren atamaz, başka yönetici atar; liste eksikse imza açılmaz', async () => {
  const tamam = [true, true, true, true, true, true, true];
  const gunler = [
    { id: 'GS-20261126-C1-1', tarih: '2026-11-26', cekmece: 'C1', kisi: 'Ayşe', kontrol: tamam, imzalar: [{ sira: 1, kisi: 'Zeynep' }] },
    { id: 'GS-20261125-C1-1', tarih: '2026-11-25', cekmece: 'C1', kisi: 'Can', kontrol: [true, true, true, true, true, true, false], imzalar: [] },
  ];
  const giren = await kur({ izinler: ['kasa.imza'], kullanici: { ad: 'Ayşe' }, onbellek: { 'kasa/ayar': AYAR, 'kasa/gunsonlari': gunler } });
  await giren.kok.querySelector('[data-gun="GS-20261126-C1-1"]').querySelector('[data-is="imzala"]').click();
  assert.equal(giren.kutu.length, 0);
  assert.match(durum(giren.kok).textContent, /ikinci imzayı gün sonunu giren atamaz/);
  const eksik = giren.kok.querySelector('[data-gun="GS-20261125-C1-1"]');
  assert.ok(eksik.querySelector('[data-uyari="kontrol"]'));
  await eksik.querySelector('[data-is="imzala"]').click();
  assert.equal(giren.kutu.length, 0);
  assert.match(durum(giren.kok).textContent, /kapanış listesi tamamlanınca/);

  const mudur = await kur({ izinler: ['kasa.imza'], kullanici: { ad: 'Mehmet' }, onbellek: { 'kasa/ayar': AYAR, 'kasa/gunsonlari': gunler } });
  await mudur.kok.querySelector('[data-gun="GS-20261126-C1-1"]').querySelector('[data-is="imzala"]').click();
  assert.deepEqual(mudur.kutu[0].govde, { sozlesme: 'kasa.gunsonu@1', tur: 'imza', v: { sira: 2, rol: 'magaza_sorumlusu', kanal: 'ekran', id: 'GS-20261126-C1-1' } });
});

test('modül yalnız baglam üstünden yazar: önbelleğe yazmaz, gonder çağırmaz', async () => {
  const { kok, yazilan, kutu } = await kur({ onbellek: GUN_ONBELLEK() });
  await gunSonuDoldur(kok);
  await dugme(kok, 'gunsonu-kaydet').click();
  assert.equal(kutu.length, 1);
  assert.deepEqual(yazilan, []);
});
