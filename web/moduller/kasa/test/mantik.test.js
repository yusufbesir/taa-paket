// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// Kasa saf kuralları: doğrulama ve karşılaştırma (crates/taa-kasa kurallarının taşınan kısmı).
import test from 'node:test';
import assert from 'node:assert/strict';
import * as K from '../mantik.js';
import { AYAR, ornekGun } from '../sinama/ornek.js';

const ayar = K.ayarDuzelt(AYAR);
const EK = { acilisKurus: 50_000, masrafKurus: 10_000 };

test('ayarDuzelt: çekmece yoksa null, varsa varsayılanlarla tamamlar', () => {
  assert.equal(K.ayarDuzelt(undefined), null);
  assert.equal(K.ayarDuzelt({ cekmeceler: [] }), null);
  assert.equal(ayar.masraf_merkez_esigi_kurus, 200_000);
  assert.equal(ayar.fark_toleransi_kurus, 0);
  assert.equal(ayar.masraf_turleri[0], 'Personel Yemek Masrafı');
  assert.equal(ayar.z_formulu.kredi_ceki_dusulur, true);
});

test('sayimToplam: geçersiz kupürü ve adedi reddeder, sıfır adedi atar', () => {
  assert.deepEqual(K.sayimToplam({ 20000: 2, 50: 3, 100: 0 }), { deger: 40150, adetler: { 20000: 2, 50: 3 } });
  assert.match(K.sayimToplam({ 300: 1 }).hata, /geçersiz para değeri/);
  assert.match(K.sayimToplam({ 100: -1 }).hata, /adet geçersiz/);
  assert.match(K.sayimToplam({ 100: 1.5 }).hata, /adet geçersiz/);
});

test('dovizTl: yarım yukarı yuvarlar, büyük sayıda taşmaz', () => {
  assert.equal(K.dovizTl({ tutar: 10_000, kur_onbinde: 421_500 }), 421_500);
  assert.equal(K.dovizTl({ tutar: 1, kur_onbinde: 5_000 }), 1);
  assert.equal(K.dovizTl({ tutar: 1, kur_onbinde: 4_999 }), 0);
  assert.equal(K.dovizTl({ tutar: 1_000_000_000_000, kur_onbinde: 100_000_000 }), 10_000_000_000_000_000);
});

test('zBeklenen: genel toplam + değişim − internet değişimi + iade − kredi çeki', () => {
  const ciro = { degisim: 500, internet_degisimi: 200, iade: 300, kredi_ceki: -100 };
  assert.equal(K.zBeklenen(10_000, ciro), 10_000 + 500 - 200 + 300 + 100);
  assert.equal(K.zBeklenen(10_000, ciro, { degisim_eklenir: false, internet_degisimi_dusulur: false, iade_eklenir: false, kredi_ceki_dusulur: false }), 10_000);
});

test('karsilastir: farksız günde dört satır tutar (nakit, Z, POS türleri, Nebim)', () => {
  const k = K.karsilastir(ayar, ornekGun(), 150_000, 50_000, 10_000);
  assert.equal(k.ciro_toplam_kurus, 360_000);
  assert.equal(k.satirlar.length, 5, 'nakit, Z, iki POS türü, Nebim');
  assert.deepEqual(k.satirlar.map((s) => s.fark_kurus), [0, 0, 0, 0, 0]);
  assert.equal(k.fark_var, false);
  assert.equal(k.kasa_ciro_uyusmuyor, false);
  const iyzico = k.satirlar.find((s) => s.ad.includes('İYZİCO POS:2'));
  assert.equal(iyzico.beklenen_kurus, 70_000, 'iki noktadaki İyzico cihazları toplanır');
});

test('karsilastir: nakit eksikse kasa ile ciro uyuşmuyor; tolerans içindeyse fark sayılmaz', () => {
  const k = K.karsilastir(ayar, ornekGun(), 149_000, 50_000, 10_000);
  assert.equal(k.satirlar[0].fark_kurus, -1_000);
  assert.equal(k.fark_var, true);
  assert.equal(k.kasa_ciro_uyusmuyor, true);
  const genis = K.ayarDuzelt({ ...AYAR, fark_toleransi_kurus: 1_000 });
  assert.equal(K.karsilastir(genis, ornekGun(), 149_000, 50_000, 10_000).fark_var, false);
});

test('gunSonuHazirla: farksız gün gövdesi sözleşmeye uyar', () => {
  const h = K.gunSonuHazirla(ayar, ornekGun(), EK);
  assert.deepEqual(h.hatalar, []);
  assert.equal(h.govde.sozlesme, 'kasa.gunsonu@1');
  assert.equal(h.govde.tur, 'gun_sonu');
  const v = h.govde.v;
  assert.equal(v.sube, 'MRK');
  assert.equal(v.nakit_sayilan_kurus, 150_000);
  assert.deepEqual(v.nakit_adetler, { 10000: 1, 20000: 7 });
  assert.equal(v.kontrol.length, 7);
  assert.equal(v.karsilastirma.fark_var, false);
  assert.equal('kasa_fazlasi' in v.ciro, false);
  assert.equal('kasa_acigi' in v.ciro, false);
  assert.equal(v.acilis_biliniyor, true);
});

test('gunSonuHazirla: fark varsa not zorunlu; notla kaydedilir ve kasa açığı sayımdan yazılır', () => {
  const eksik = ornekGun({ nakit_adetler: { 20000: 7, 5000: 1 } }); // 1.450 TL: 50 TL açık
  const h1 = K.gunSonuHazirla(ayar, eksik, EK);
  assert.deepEqual(h1.hatalar, ['karşılaştırmada fark var; açıklama notu yazın']);
  assert.equal(h1.govde, undefined);
  const h2 = K.gunSonuHazirla(ayar, { ...eksik, not: '  50 TL sayım hatası  ', ciro: { ...eksik.ciro, kasa_fazlasi: 999 } }, EK);
  assert.deepEqual(h2.hatalar, []);
  assert.equal(h2.govde.v.not, '50 TL sayım hatası');
  assert.equal(h2.govde.v.ciro.kasa_acigi, 5_000);
  assert.equal('kasa_fazlasi' in h2.govde.v.ciro, false, 'elle girilen kasa fazlası atılır');
  assert.equal(h2.govde.v.karsilastirma.kasa_ciro_uyusmuyor, true);
});

test('gunSonuHazirla: kasa fazlası sayım farkından yazılır', () => {
  const fazla = ornekGun({ nakit_adetler: { 20000: 8 }, not: 'fazla çıktı' }); // 1.600 TL
  assert.equal(K.gunSonuHazirla(ayar, fazla, EK).govde.v.ciro.kasa_fazlasi, 10_000);
});

test('gunSonuHazirla: kural ihlalleri reddedilir', () => {
  const hata = (d, ek = EK) => K.gunSonuHazirla(ayar, ornekGun(d), ek).hatalar.join(' | ');
  assert.match(hata({ tarih: '27.11.2026' }), /tarih YYYY-AA-GG/);
  assert.match(hata({ cekmece: 'YOK' }), /tanımsız çekmece: YOK/);
  assert.match(hata({ noktalar: [{ nokta: 'N9', z_toplam_kurus: 0, pos: {} }] }), /bağlı olmayan kasa noktası: N9/);
  assert.match(hata({ noktalar: [{ nokta: 'N1', z_toplam_kurus: 0, pos: { X: 1 } }] }), /tanımsız POS: X/);
  assert.match(hata({ ciro: { uydurma: 1 } }), /bilinmeyen ciro alanı: uydurma/);
  assert.match(hata({ ciro: { nakit: -1 } }), /nakit: tutar 0 ile/);
  assert.match(hata({ ciro: { efatura_pos: 5 } }), /E-FATURA POS kaldırıldı/);
  assert.match(hata({ ciro: { havale_fark: -500 } }), /HAVALE \+\/- için açıklama yazın/);
  assert.match(hata({ aciklamalar: { nakit: 'x' } }), /açıklama yazılamayan alan: nakit/);
  assert.match(hata({ doviz: [{ kod: 'JPY', tutar: 1, kur_onbinde: 1 }] }), /bilinmeyen döviz: JPY/);
  assert.match(hata({ doviz: [{ kod: 'USD', tutar: 100, kur_onbinde: 0 }] }), /USD için günlük kur girin/);
  assert.match(hata({ devreden_kurus: 150_001 }), /devreden nakit sayılan nakitten büyük olamaz/);
  assert.match(hata({ nakit_adetler: { 7: 1 } }), /geçersiz para değeri/);
  assert.match(hata({ nebim_toplam_kurus: 1.5 }), /Nebim toplamı/);
});

test('gunSonuHazirla: eksi kredi çeki ve açıklamalı havale farkı kabul edilir', () => {
  const g = ornekGun({ ciro: { nakit: 100_000, ziraat_pos: 80_000, iyzico_pos: 70_000, havale: 100_000, kredi_ceki: -5_000, havale_fark: -500 }, aciklamalar: { havale_fark: 'iade' }, not: 'kredi çeki' });
  const h = K.gunSonuHazirla(ayar, g, EK);
  assert.deepEqual(h.hatalar, []);
  assert.equal(h.govde.v.ciro.kredi_ceki, -5_000);
  assert.deepEqual(h.govde.v.aciklamalar, { havale_fark: 'iade' });
});

test('gunSonuHazirla: açılış bilinmiyorsa 0 sayılır ve gövdede işaretlenir', () => {
  const h = K.gunSonuHazirla(ayar, ornekGun({ not: 'açılış bu cihazda yok' }), { acilisKurus: null, masrafKurus: 10_000 });
  assert.equal(h.govde.v.acilis_biliniyor, false);
  assert.equal(h.govde.v.acilis_kurus, 0);
  assert.equal(h.govde.v.ciro.kasa_fazlasi, 50_000);
});

test('masrafHazirla: zorunlu alanlar, eşik ve gövde', () => {
  const g = { tarih: '2026-11-27', cekmece: 'C1', tur: ' Kargo ', tutar_kurus: 15_000, aciklama: ' Yurtiçi ', not: 'x'.repeat(250) };
  const h = K.masrafHazirla(ayar, g);
  assert.deepEqual(h.hatalar, []);
  assert.deepEqual({ ...h.govde, v: { ...h.govde.v, not: undefined } }, {
    sozlesme: 'kasa.masraf@1',
    tur: 'masraf',
    v: { tarih: '2026-11-27', cekmece: 'C1', tur: 'Kargo', tutar_kurus: 15_000, aciklama: 'Yurtiçi', not: undefined, onay_duzeyi: 'mudur' },
  });
  assert.equal(h.govde.v.not.length, 200);
  assert.equal(K.masrafHazirla(ayar, { ...g, tutar_kurus: 200_000 }).govde.v.onay_duzeyi, 'merkez', 'eşik ve üstü merkez');
  assert.equal(K.masrafHazirla(ayar, { ...g, tutar_kurus: 199_999 }).govde.v.onay_duzeyi, 'mudur');
  const hata = (d) => K.masrafHazirla(ayar, { ...g, ...d }).hatalar.join(' | ');
  assert.match(hata({ tutar_kurus: 0 }), /masraf tutarı girin/);
  assert.match(hata({ tutar_kurus: -5 }), /tutar 0 ile/);
  assert.match(hata({ tur: '  ' }), /masraf türü gerekli/);
  assert.match(hata({ tur: 'x'.repeat(61) }), /en çok 60 karakter/);
  assert.match(hata({ tur: 'a\nb' }), /tek satır/);
  assert.match(hata({ aciklama: '' }), /masraf açıklaması gerekli/);
  assert.match(hata({ cekmece: '' }), /tanımsız çekmece/);
  assert.equal(K.masrafHazirla(ayar, { ...g, tur: 'Listede olmayan tür' }).hatalar.length, 0, 'serbest tür kabul edilir');
});

test('sayimHazirla: ikinci kişi, fark notu ve gövde', () => {
  const g = { tarih: '2026-11-27', cekmece: 'C1', adetler: { 20000: 2, 10000: 1 }, not: '' };
  const acilis = K.sayimHazirla(ayar, 'acilis', g, { beklenenKurus: 50_000 });
  assert.deepEqual(acilis.hatalar, []);
  assert.deepEqual(acilis.govde, {
    sozlesme: 'kasa.gunsonu@1',
    tur: 'sayim',
    v: { tur: 'acilis', tarih: '2026-11-27', cekmece: 'C1', ikinci_kisi: null, adetler: { 10000: 1, 20000: 2 }, sayilan_kurus: 50_000, beklenen_kurus: 50_000, fark_kurus: 0, not: '' },
  });
  assert.match(K.sayimHazirla(ayar, 'acilis', g, { beklenenKurus: 60_000 }).hatalar[0], /fark var; açıklama notu yazın/);
  assert.equal(K.sayimHazirla(ayar, 'acilis', { ...g, not: 'eksik' }, { beklenenKurus: 60_000 }).govde.v.fark_kurus, -10_000);
  const bilinmiyor = K.sayimHazirla(ayar, 'acilis', g, { beklenenKurus: null });
  assert.equal(bilinmiyor.govde.v.beklenen_kurus, null, 'beklenen bilinmiyorsa fark hesaplanmaz');
  assert.equal(bilinmiyor.govde.v.fark_kurus, null);
  assert.match(K.sayimHazirla(ayar, 'ara', g).hatalar[0], /ikinci kişi gerekli/);
  assert.match(K.sayimHazirla(ayar, 'devir', g).hatalar[0], /devralan kişi gerekli/);
  assert.match(K.sayimHazirla(ayar, 'ara', { ...g, ikinci_kisi: ' ayşe ' }, { girenAd: 'Ayşe' }).hatalar[0], /girenden farklı olmalı/);
  assert.equal(K.sayimHazirla(ayar, 'devir', { ...g, ikinci_kisi: 'Mehmet' }, { girenAd: 'Ayşe' }).govde.v.ikinci_kisi, 'Mehmet');
  assert.match(K.sayimHazirla(ayar, 'uydurma', g).hatalar[0], /bilinmeyen sayım türü/);
});

test('ciftDogrulama: 8 kalem, fark = fiili − ERP, sonuç farktan seçilir', () => {
  const v = K.gunSonuHazirla(ayar, ornekGun(), EK).govde.v;
  const t = K.ciftDogrulama(ayar, v);
  assert.deepEqual(t.kalemler.map((k) => k.kod), ['nakit_tl', 'ziraat_pos', 'iyzico_fiziksel_pos', 'iyzico_sanal_pos', 'havale_eft', 'kredi_ceki', 'eticaret_degisimi', 'diger']);
  assert.equal(t.sonuc, 'Fark yoktur.');
  assert.equal(t.fark_kalem, 0);
  const eksik = K.gunSonuHazirla(ayar, ornekGun({ nakit_adetler: { 20000: 7, 5000: 1 }, not: 'sayım hatası', ciro: { nakit: 100_000, ziraat_pos: 80_000, iyzico_pos: 70_000, havale: 100_000, havale_fark: 300 }, aciklamalar: { havale_fark: 'iade' } }), EK).govde.v;
  const t2 = K.ciftDogrulama(ayar, eksik);
  assert.equal(t2.sonuc, 'Fark tespit edilmiştir.');
  assert.equal(t2.kalemler[0].fark_kurus, -5_000, 'nakit: sayılan − açılış − NAKİT TL');
  assert.equal(t2.kalemler[0].gerekce, 'sayım hatası');
  assert.equal(t2.kalemler[4].fark_kurus, 300, 'havale: havale + HAVALE +/- − havale');
  assert.equal(t2.toplam_fark_kurus, -4_700);
  assert.equal(t2.fark_tutar_kurus, 5_300);
  assert.equal(t2.fark_kalem, 2);
});

test('imzaHazirla: çift doğrulama kuralları', () => {
  const tamam = [true, true, true, true, true, true, true];
  const gun = { id: 'GS-20261127-C1-1', kisi: 'Ayşe', kontrol: tamam, imzalar: [] };
  const ilk = K.imzaHazirla(gun, 'Ayşe');
  assert.deepEqual(ilk.govde, { sozlesme: 'kasa.gunsonu@1', tur: 'imza', v: { sira: 1, rol: 'kasa_sorumlusu', kanal: 'ekran', id: 'GS-20261127-C1-1' } });
  const tek = { ...gun, imzalar: [{ sira: 1, kisi: 'Ayşe' }] };
  assert.match(K.imzaHazirla(tek, 'AYŞE').hatalar[0], /aynı kişi iki kez imzalayamaz/);
  assert.equal(K.imzaHazirla(tek, 'Mehmet').govde.v.sira, 2);
  assert.equal(K.imzaHazirla(tek, 'Mehmet').govde.v.rol, 'magaza_sorumlusu');
  assert.match(K.imzaHazirla({ ...gun, imzalar: [{ sira: 1, kisi: 'Zeynep' }] }, 'Ayşe').hatalar[0], /ikinci imzayı gün sonunu giren atamaz/);
  assert.match(K.imzaHazirla({ ...gun, kontrol: [true, true, true, true, true, true, false] }, 'Ayşe').hatalar[0], /kapanış listesi tamamlanınca/);
  assert.match(K.imzaHazirla({ ...tek, imzalar: [...tek.imzalar, { sira: 2, kisi: 'Mehmet' }] }, 'Can').hatalar[0], /iki imza da atılmış/);
  assert.match(K.imzaHazirla({ ...gun, gonderildi: true }, 'Ayşe').hatalar[0], /gönderilmiş güne imza eklenmez/);
  // Bu oturumda bekleyen kayıtlar aynı kişinindir (giden kutusu kullanıcıya özel).
  const yerel = { istemci_no: 'N-1', bekliyor: true, kontrol: tamam, imzalar: [] };
  assert.deepEqual(K.imzaHazirla(yerel).govde.v, { sira: 1, rol: 'kasa_sorumlusu', kanal: 'ekran', istemci_no: 'N-1' });
  assert.match(K.imzaHazirla({ ...yerel, imzalar: [{ sira: 1, kisi: '', bekliyor: true }] }).hatalar[0], /aynı kişi iki kez imzalayamaz/);
  assert.match(K.imzaHazirla({ ...yerel, imzalar: [{ sira: 1, kisi: 'Zeynep' }] }).hatalar[0], /giren atamaz/);
});

test('gerekceHazirla: yalnız tek imzalı güne, 5–300 karakter', () => {
  const gun = { id: 'GS-1', kontrol: [], imzalar: [{ sira: 1, kisi: 'Ayşe' }] };
  assert.deepEqual(K.gerekceHazirla(gun, '  Mağaza sorumlusu izinli  ').govde, { sozlesme: 'kasa.gunsonu@1', tur: 'gerekce', v: { gerekce: 'Mağaza sorumlusu izinli', id: 'GS-1' } });
  assert.match(K.gerekceHazirla(gun, 'kısa').hatalar[0], /5–300 karakter/);
  assert.match(K.gerekceHazirla(gun, 'x'.repeat(301)).hatalar[0], /5–300 karakter/);
  assert.match(K.gerekceHazirla({ ...gun, imzalar: [] }, 'yeterince uzun').hatalar[0], /yalnız tek imzalı güne/);
  assert.match(K.gerekceHazirla({ ...gun, gonderildi: true }, 'yeterince uzun').hatalar[0], /gönderilmiş güne/);
});

test('gunOzeti: önbellek + bekleyen giden kayıtları', () => {
  const bekleyen = [
    { durum: 'bekliyor', govde: { tur: 'masraf', v: { tarih: '2026-11-27', cekmece: 'C1', tutar_kurus: 2_500 } } },
    { durum: 'bekliyor', govde: { tur: 'masraf', v: { tarih: '2026-11-26', cekmece: 'C1', tutar_kurus: 9_999 } } },
    { durum: 'bekliyor', govde: { tur: 'sayim', v: { tur: 'acilis', tarih: '2026-11-27', cekmece: 'C1', sayilan_kurus: 50_000 } } },
  ];
  assert.deepEqual(K.gunOzeti({ masraf_kurus: 1_000, onceki_devreden_kurus: 50_000 }, bekleyen, '2026-11-27', 'C1'), {
    acilisKurus: 50_000,
    masrafKurus: 3_500,
    oncekiDevredenKurus: 50_000,
    gunSonuVar: false,
    acilisVar: true,
  });
  assert.deepEqual(K.gunOzeti(undefined, [], '2026-11-27', 'C1'), { acilisKurus: null, masrafKurus: 0, oncekiDevredenKurus: null, gunSonuVar: false, acilisVar: false });
  assert.equal(K.gunOzeti({ gun_sonu_var: true }, [], '2026-11-27', 'C1').gunSonuVar, true);
});

test('imzaListesi: bekleyen gün sonu, imza ve gerekçe birleşir', () => {
  const tamam = [true, true, true, true, true, true, true];
  const kayitlar = [
    { no: 'N-1', durum: 'bekliyor', govde: { tur: 'gun_sonu', v: { tarih: '2026-11-27', cekmece: 'C1', kontrol: tamam } } },
    { no: 'N-2', durum: 'bekliyor', govde: { tur: 'imza', v: { istemci_no: 'N-1', sira: 1 } } },
    { no: 'N-3', durum: 'bekliyor', govde: { tur: 'gerekce', v: { id: 'GS-1', gerekce: 'izinli' } } },
    { no: 'N-4', durum: 'gonderildi', govde: { tur: 'gun_sonu', v: { tarih: '2026-11-20', cekmece: 'C1', kontrol: tamam } } },
  ];
  const l = K.imzaListesi([{ id: 'GS-1', tarih: '2026-11-26', cekmece: 'C1', kisi: 'Ayşe', kontrol: tamam, imzalar: [{ sira: 1, kisi: 'Ayşe' }] }], kayitlar);
  assert.equal(l.length, 2, 'gönderilmiş yerel kayıt listeye girmez (sunucu görünümünden gelir)');
  assert.equal(l[0].istemci_no, 'N-1');
  assert.deepEqual(l[0].imzalar, [{ sira: 1, kisi: '', bekliyor: true }]);
  assert.equal(l[1].gerekce, 'izinli');
});
