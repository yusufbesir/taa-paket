// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W2-yol 2026-11-27]
// Yol testleri: merkez → şube → yerel sıralı deneme, çevrimiçiyken doğrudan merkez, yol başarısızlığının cihaz
// günlüğüne yazılması (yalnız ekleme), günlüğün merkeze kopyalanması, kabuğun yol ayarı.
// Bellek içi sürücü ve sahte fetch ile çalışır; gerçek tarayıcı / gerçek şube düğümü / gerçek merkez DENENMEZ.
import test from 'node:test';
import assert from 'node:assert/strict';
import { acDB, TaaHata, Gunluk, Giden } from '../cekirdek/taa-cekirdek.js';
import {
  yollariCoz,
  adresDuzelt,
  yolluGonder,
  gunlukKopyala,
  subeKaliciRet,
  subeSuzgeci,
  yolAdi,
  SUBE_GOVDE_EN_COK,
  UC_GUNLUK_KOPYA,
} from '../cekirdek/yol.js';
import { yolAyariOku, yolAyariKur, yolSatirlari, yolNotu } from '../kabuk/yardimci.js';

const MERKEZ = 'https://merkez.test';
const SUBE = 'http://192.168.1.20:8788';
const YOLLAR = [{ ad: 'merkez', url: MERKEZ }, { ad: 'sube', url: SUBE }, { ad: 'yerel' }];
let sayac = 0;
const yeniDB = (ek = {}) => acDB('yol', `k${(sayac += 1)}`, { cihaz: 'test', saat: () => 1_795_000_000_000, ...ek });
const kayit = (n, program = 'kasa') => ({ program, uc: `/v1/${program}/k`, govde: { n } });

/** Sahte ağ: `plan(adres, secenek)` -> kod | {durum, govde} | Error. Günlük kopyası istekleri ayrı sayılır. */
function sahteAg(plan) {
  const cagrilar = [];
  const getir = async (url, secenek) => {
    const c = { url, koken: new URL(url).origin, yol: new URL(url).pathname, basliklar: secenek.headers, govde: JSON.parse(secenek.body) };
    cagrilar.push(c);
    const s = plan(c);
    if (s instanceof Error) throw s;
    const durum = typeof s === 'number' ? s : s.durum;
    const govde = typeof s === 'number' ? '' : s.govde ?? '';
    return { status: durum, ok: durum >= 200 && durum <= 299, text: async () => govde };
  };
  const kayitlar = () => cagrilar.filter((c) => c.yol !== UC_GUNLUK_KOPYA);
  return { getir, cagrilar, kayitlar };
}
const agYok = () => new TypeError('fetch failed');
const olaylar = async (db) => (await db.gunluk.liste()).map((s) => `${s.olay}:${s.yol ?? ''}`);

test('adresDuzelt ve yollariCoz: sıra her zaman merkez → sube → yerel; bozuk giriş atılır', () => {
  assert.equal(adresDuzelt('https://Merkez.Test/'), 'https://merkez.test');
  assert.equal(adresDuzelt('http://192.168.1.20:8788'), SUBE);
  for (const k of ['', 'ftp://x', 'http://x/yol', 'http://x?y=1', 'http://kul:par@x', 'http://x:99999', 'http://x y', 'javascript:alert(1)', null, 5]) {
    assert.equal(adresDuzelt(k), null, String(k));
  }
  const ters = yollariCoz([{ ad: 'yerel' }, { ad: 'sube', url: SUBE, sube: 'MRK01' }, { ad: 'merkez', url: `${MERKEZ}/` }]);
  assert.deepEqual(ters.yollar, [{ ad: 'merkez', url: MERKEZ }, { ad: 'sube', url: SUBE, sube: 'mrk01' }, { ad: 'yerel' }]);
  assert.deepEqual(ters.hatalar, []);

  const eksik = yollariCoz(undefined, { merkez: MERKEZ });
  assert.deepEqual(eksik.yollar, [{ ad: 'merkez', url: MERKEZ }, { ad: 'yerel' }], 'ayar yoksa merkez = kabuğun kökeni');

  const bozuk = yollariCoz(
    [{ ad: 'bulut', url: 'https://x.test' }, { ad: 'sube', url: 'http://x/yol' }, { ad: 'merkez', url: MERKEZ }, { ad: 'merkez', url: 'https://ikinci.test' }, null],
    { merkez: 'https://koken.test' },
  );
  assert.deepEqual(bozuk.yollar, [{ ad: 'merkez', url: MERKEZ }, { ad: 'yerel' }]);
  assert.equal(bozuk.hatalar.length, 4);
  assert.equal(yollariCoz('metin').hatalar.length, 1);
  assert.equal(yolAdi('sube'), 'şube bilgisayarı');
});

test('çevrimiçi: kayıtlar DOĞRUDAN merkeze gider; şube bilgisayarına hiç dokunulmaz', async () => {
  const db = yeniDB();
  for (const n of [1, 2, 3]) await db.giden.ekle(kayit(n));
  const ag = sahteAg(() => 200);
  const ozet = await db.giden.gonder({ yollar: YOLLAR, oturum: 'gizli', sube: 'MRK01', fetch: ag.getir });
  assert.equal(ozet.gonderilen, 3);
  assert.equal(ozet.kalan, 0);
  assert.equal(ozet.durmaNedeni, null);
  assert.equal(ozet.yol, 'merkez');
  assert.deepEqual(ozet.denemeler.map((d) => d.yol), ['merkez']);
  assert.ok(ag.cagrilar.every((c) => c.koken === MERKEZ), 'şube adresine istek yok');
  assert.equal(ag.kayitlar()[0].basliklar.Authorization, 'Bearer gizli');
  assert.ok((await db.giden.liste('gonderildi')).every((k) => k.gonderim.yol === 'merkez'));
  assert.deepEqual(await olaylar(db), [], 'başarılı gönderim günlüğe yol hatası yazmaz');
});

test('merkeze ulaşılamıyor: şube bilgisayarına gider; oturum anahtarı şubeye GİTMEZ, X-TAA-Sube gider', async () => {
  const db = yeniDB();
  for (const n of [1, 2]) await db.giden.ekle(kayit(n));
  const ag = sahteAg((c) => (c.koken === MERKEZ ? agYok() : { durum: 200, govde: '{"durum":"yazildi","kesin":false,"dugum":"sube"}' }));
  const ozet = await db.giden.gonder({ yollar: YOLLAR, oturum: 'gizli', sube: 'MRK01', fetch: ag.getir });
  assert.equal(ozet.gonderilen, 2);
  assert.equal(ozet.kalan, 0);
  assert.equal(ozet.durmaNedeni, null, 'hepsi şube bilgisayarına teslim edildi');
  assert.equal(ozet.yol, 'sube');
  assert.deepEqual(ozet.denemeler.map((d) => [d.yol, d.gonderilen, d.durmaTuru ?? null]), [['merkez', 0, 'ag'], ['sube', 2, null]]);
  const subeIstekleri = ag.cagrilar.filter((c) => c.koken === SUBE);
  assert.equal(subeIstekleri.length, 2);
  for (const c of subeIstekleri) {
    assert.equal(c.basliklar.Authorization, undefined, 'oturum anahtarı şube ağına çıkmamalı');
    assert.equal(c.basliklar['X-TAA-Sube'], 'mrk01');
    assert.equal(c.basliklar['X-TAA-No'], c.govde.no, 'aynı no (idempotent)');
    assert.equal(JSON.stringify(c).includes('gizli'), false);
  }
  const gonderilen = await db.giden.liste('gonderildi');
  assert.deepEqual(gonderilen.map((k) => k.gonderim.yol), ['sube', 'sube']);
  assert.equal(gonderilen[0].gonderim.yanit.dugum, 'sube');
  assert.deepEqual(await olaylar(db), ['yol.basarisiz:merkez', 'yol.kullanildi:sube']);
  const satir = (await db.gunluk.liste())[0];
  assert.equal(satir.ayrinti.tur, 'ag');
  assert.equal(JSON.stringify(await db.gunluk.liste()).includes('gizli'), false, 'günlüğe oturum anahtarı yazılmaz');
});

test('merkez yarıda kopar: gidenler merkezde kalır, kalanlar aynı sırayla şubeye gider', async () => {
  const db = yeniDB();
  for (const n of [1, 2, 3, 4]) await db.giden.ekle(kayit(n));
  let merkezSayisi = 0;
  const ag = sahteAg((c) => {
    if (c.yol === UC_GUNLUK_KOPYA) return agYok();
    if (c.koken === SUBE) return 200;
    merkezSayisi += 1;
    return merkezSayisi <= 2 ? 200 : 503;
  });
  const ozet = await db.giden.gonder({ yollar: YOLLAR, oturum: 'o', sube: 'MRK01', fetch: ag.getir });
  assert.equal(ozet.gonderilen, 4);
  assert.equal(ozet.kalan, 0);
  assert.deepEqual(ag.kayitlar().map((c) => `${c.koken === MERKEZ ? 'm' : 's'}${c.govde.n}`), ['m1', 'm2', 'm3', 's3', 's4']);
  assert.deepEqual((await db.giden.liste('gonderildi')).map((k) => k.gonderim.yol), ['merkez', 'merkez', 'sube', 'sube']);
});

test('merkez de şube de yok: kayıtlar cihazda bekler; iki yol hatası günlükte, yineleme günlüğü şişirmez', async () => {
  const db = yeniDB();
  for (const n of [1, 2]) await db.giden.ekle(kayit(n));
  const ag = sahteAg(() => agYok());
  const secenek = { yollar: YOLLAR, oturum: 'o', sube: 'MRK01', fetch: ag.getir };
  const ozet = await db.giden.gonder(secenek);
  assert.equal(ozet.gonderilen, 0);
  assert.equal(ozet.kalan, 2);
  assert.equal(ozet.yol, 'yerel');
  assert.match(ozet.durmaNedeni, /Sunucuya ulaşılamadı.*Şube bilgisayarı: Sunucuya ulaşılamadı/);
  assert.deepEqual(await db.giden.say(), { bekliyor: 2, gonderildi: 0, reddedildi: 0 }, 'kayıp yok, ret yok');
  assert.deepEqual(await olaylar(db), ['yol.basarisiz:merkez', 'yol.basarisiz:sube', 'yol.yerel:yerel']);

  for (let i = 0; i < 5; i += 1) await db.giden.gonder(secenek);
  assert.equal((await db.gunluk.liste()).length, 3, 'aynı neden sürerken yeni satır yazılmaz');

  // Bağlantı gelir: kayıtlar doğrudan merkeze gider, düzelme günlüğe yazılır (bastırılan yineleme sayısıyla).
  const acik = sahteAg(() => 200);
  const son = await db.giden.gonder({ ...secenek, fetch: acik.getir });
  assert.equal(son.gonderilen, 2);
  assert.equal(son.yol, 'merkez');
  assert.ok(acik.kayitlar().every((c) => c.koken === MERKEZ));
  const g = await db.gunluk.liste();
  assert.deepEqual(g.slice(3).map((s) => `${s.olay}:${s.yol}`), ['yol.duzeldi:merkez', 'yol.duzeldi:yerel']);
  assert.equal(g[3].ayrinti.bastirilan_tekrar, 5);
  assert.deepEqual(g.map((s) => s.sira), [1, 2, 3, 4, 5], 'eski satırlar yerinde; yalnız ekleme');
});

test('merkez yanıt veriyorsa (401, 4xx) şubeye GEÇİLMEZ: çevrimiçiyken yol merkezdir', async () => {
  const db = yeniDB();
  await db.giden.ekle(kayit(1));
  const ag = sahteAg((c) => (c.koken === MERKEZ ? 401 : 200));
  const ozet = await db.giden.gonder({ yollar: YOLLAR, oturum: 'eski', sube: 'MRK01', fetch: ag.getir });
  assert.equal(ozet.durmaKodu, 401);
  assert.equal(ozet.durmaTuru, 'oturum');
  assert.equal(ozet.yol, 'yerel');
  assert.equal(ag.cagrilar.filter((c) => c.koken === SUBE).length, 0);
  assert.deepEqual(await db.giden.say(), { bekliyor: 1, gonderildi: 0, reddedildi: 0 });

  // Oturum hiç yoksa da şubeye geçilmez (kayıt kişinin kendi oturumuyla merkeze gitmeyi bekler).
  const ag2 = sahteAg(() => 200);
  const o2 = await db.giden.gonder({ yollar: YOLLAR, oturum: '', sube: 'MRK01', fetch: ag2.getir });
  assert.equal(o2.durmaKodu, 401);
  assert.equal(ag2.cagrilar.length, 0);

  // Merkezin kalıcı reddi (422) rettir; şubeye "bir de orada deneyelim" diye gönderilmez.
  const db2 = yeniDB();
  await db2.giden.ekle(kayit(1));
  const ag3 = sahteAg((c) => (c.koken === MERKEZ ? { durum: 422, govde: 'alan eksik' } : 200));
  const o3 = await db2.giden.gonder({ yollar: YOLLAR, oturum: 'o', sube: 'MRK01', fetch: ag3.getir });
  assert.equal(o3.reddedilen, 1);
  assert.equal(ag3.cagrilar.filter((c) => c.koken === SUBE).length, 0);
  assert.equal((await db2.giden.liste('reddedildi'))[0].ret.yol, 'merkez');
});

test('çevrimdışı (ağ yok): hiçbir yola istek atılmaz; kayıt bekler, günlüğe bir kez yazılır', async () => {
  const db = yeniDB();
  await db.giden.ekle(kayit(1));
  const ag = sahteAg(() => 200);
  const secenek = { yollar: YOLLAR, oturum: 'o', sube: 'MRK01', cevrimici: false, fetch: ag.getir };
  const ozet = await db.giden.gonder(secenek);
  await db.giden.gonder(secenek);
  assert.equal(ag.cagrilar.length, 0);
  assert.equal(ozet.kalan, 1);
  assert.equal(ozet.yol, 'yerel');
  assert.equal(ozet.durmaTuru, 'cevrimdisi');
  assert.match(ozet.durmaNedeni, /Çevrimdışı/);
  assert.deepEqual(await olaylar(db), ['yol.basarisiz:merkez']);
});

test('şube düğümünün reddi: yalnız 409 cakisma kalıcıdır; öteki 4xx kaydı reddetmez, kayıt merkezi bekler', async () => {
  assert.equal(subeKaliciRet(409, '{"hata":"cakisma","mesaj":"x"}'), true);
  assert.equal(subeKaliciRet(409, '{"hata":"sube_uyusmaz"}'), false);
  assert.equal(subeKaliciRet(409, 'cakisma'), false);
  assert.equal(subeKaliciRet(403, '{"hata":"cakisma"}'), false);

  const db = yeniDB();
  for (const n of [1, 2]) await db.giden.ekle(kayit(n));
  const ag = sahteAg((c) => {
    if (c.koken === MERKEZ) return agYok();
    return c.govde.n === 1 ? { durum: 409, govde: '{"hata":"cakisma","mesaj":"aynı no, farklı içerik"}' } : { durum: 403, govde: '{"hata":"ag_disi"}' };
  });
  const ozet = await db.giden.gonder({ yollar: YOLLAR, oturum: 'o', sube: 'MRK01', fetch: ag.getir });
  assert.equal(ozet.reddedilen, 1, 'çakışmada şube bilgisayarındaki hal geçerli; kaybeden reddedildi listesinde');
  assert.equal(ozet.kalan, 1);
  const ret = (await db.giden.liste('reddedildi'))[0];
  assert.equal(ret.ret.kod, 409);
  assert.equal(ret.ret.yol, 'sube');
  assert.deepEqual((await db.giden.liste('bekliyor')).map((k) => k.govde.n), [2], '403 kalıcı ret değil');
  assert.deepEqual(await olaylar(db), ['yol.basarisiz:merkez', 'yol.basarisiz:sube', 'yol.kullanildi:sube', 'yol.yerel:yerel']);
});

test('şube düğümü için fazla büyük kayıt atlanır; aynı programın sonrakileri de bekler, öteki programlar gider', async () => {
  const s = subeSuzgeci(100);
  assert.equal(s({ no: 'a', program: 'kasa', govde: { n: 1 } }), true);
  assert.equal(s({ no: 'b', program: 'tutanak', govde: { veri: 'x'.repeat(200) } }), false);
  assert.equal(s({ no: 'c', program: 'tutanak', govde: { n: 1 } }), false, 'program içi sıra korunur');
  assert.equal(s({ no: 'd', program: 'kasa', govde: { n: 2 } }), true);

  const db = yeniDB();
  await db.giden.ekle(kayit(1, 'kasa'));
  await db.giden.ekle({ program: 'tutanak', uc: '/v1/tutanak/foto', govde: { veri: 'x'.repeat(SUBE_GOVDE_EN_COK) } });
  await db.giden.ekle(kayit(3, 'tutanak'));
  await db.giden.ekle(kayit(4, 'kasa'));
  const ag = sahteAg((c) => (c.koken === MERKEZ ? agYok() : 200));
  const ozet = await db.giden.gonder({ yollar: YOLLAR, oturum: 'o', sube: 'MRK01', fetch: ag.getir });
  assert.equal(ozet.gonderilen, 2);
  assert.equal(ozet.kalan, 2);
  assert.equal(ozet.atlanan, 2);
  assert.equal(ozet.durmaTuru, 'atlandi');
  assert.match(ozet.durmaNedeni, /2 kayıt şube bilgisayarı için fazla büyük/);
  assert.deepEqual(ag.cagrilar.filter((c) => c.koken === SUBE).map((c) => c.govde.n), [1, 4]);
  assert.deepEqual((await db.giden.liste('bekliyor')).map((k) => k.program), ['tutanak', 'tutanak']);
});

test('şube kodu: yol ayarındaki kod oturumdaki şubenin önüne geçer; kod yoksa şubeye gönderilmez', async () => {
  const db = yeniDB();
  await db.giden.ekle(kayit(1));
  const ag = sahteAg((c) => (c.koken === MERKEZ ? agYok() : 200));
  const yollar = [{ ad: 'merkez', url: MERKEZ }, { ad: 'sube', url: SUBE, sube: 'IZM01' }];
  await db.giden.gonder({ yollar, oturum: 'o', sube: '3', fetch: ag.getir });
  assert.equal(ag.cagrilar.find((c) => c.koken === SUBE).basliklar['X-TAA-Sube'], 'izm01');

  const db2 = yeniDB();
  await db2.giden.ekle(kayit(1));
  const ag2 = sahteAg((c) => (c.koken === MERKEZ ? agYok() : 200));
  const o = await db2.giden.gonder({ yollar: YOLLAR, oturum: 'o', fetch: ag2.getir });
  assert.equal(ag2.cagrilar.filter((c) => c.koken === SUBE).length, 0);
  assert.equal(o.kalan, 1);
  assert.deepEqual(o.denemeler.map((d) => d.durmaTuru), ['ag', 'ayar']);
});

test('şube yolu tanımlı değilse merkez → yerel; yol listesi boşsa hata', async () => {
  const db = yeniDB();
  await db.giden.ekle(kayit(1));
  const ag = sahteAg(() => 503);
  const o = await db.giden.gonder({ yollar: [{ ad: 'merkez', url: MERKEZ }, { ad: 'yerel' }], oturum: 'o', fetch: ag.getir });
  assert.equal(o.kalan, 1);
  assert.equal(o.yol, 'yerel');
  assert.equal(o.durmaKodu, 503);
  assert.deepEqual(await olaylar(db), ['yol.basarisiz:merkez', 'yol.yerel:yerel']);
  await assert.rejects(db.giden.gonder({ yollar: [{ ad: 'yerel' }], oturum: 'o', fetch: ag.getir }), (e) => e.kod === 'gecersiz-sunucu');
  assert.throws(() => yolluGonder(null, {}), (e) => e.kod === 'gecersiz-giden');
});

test('geriye dönük uyum: gonder({sunucu, oturum}) tek yola gider, sonuç biçimi ve kayıt alanları eskisi gibi', async () => {
  const db = yeniDB();
  await db.giden.ekle(kayit(1));
  const ag = sahteAg(() => 200);
  const o = await db.giden.gonder({ sunucu: MERKEZ, oturum: 'o', fetch: ag.getir });
  assert.deepEqual(o, { gonderilen: 1, reddedilen: 0, kalan: 0, durmaNedeni: null });
  const k = (await db.giden.liste('gonderildi'))[0];
  assert.deepEqual(Object.keys(k.gonderim).sort(), ['yanit', 'zaman'], 'yol adı yalnız yollu gönderimde yazılır');
  assert.deepEqual(Object.keys(ag.cagrilar[0].basliklar).sort(), ['Authorization', 'Content-Type', 'X-TAA-No']);
  assert.deepEqual(await olaylar(db), []);
});

test('tekYolGonder: ek başlık çekirdeğin başlıklarını ezemez; bozuk başlık ve yol adı reddedilir', async () => {
  const db = yeniDB();
  await db.giden.ekle(kayit(1));
  const ag = sahteAg(() => 200);
  for (const basliklar of [{ Authorization: 'x' }, { 'x-taa-no': 'y' }, { 'X-A': 'a\r\nb' }, { 'X A': 'a' }, { 'X-A': '' }]) {
    await assert.rejects(db.giden.tekYolGonder({ sunucu: SUBE, oturumsuz: true, basliklar, fetch: ag.getir }), TaaHata, JSON.stringify(basliklar));
  }
  await assert.rejects(db.giden.tekYolGonder({ sunucu: SUBE, oturum: 'o', yol: 'Şube', fetch: ag.getir }), TaaHata);
  assert.equal(ag.cagrilar.length, 0);
});

test('cihaz günlüğü: yalnız ekleme; sıra artar; silme/değiştirme işlevi yok; içerik sınırı', async () => {
  const db = yeniDB();
  const a = await db.gunluk.yaz({ olay: 'yol.basarisiz', yol: 'merkez', ayrinti: { neden: 'ağ yok' } });
  const b = await db.gunluk.yaz({ olay: 'yol.yerel' });
  assert.deepEqual(a, { sira: 1, zaman: new Date(1_795_000_000_000).toISOString(), olay: 'yol.basarisiz', yol: 'merkez', ayrinti: { neden: 'ağ yok' } });
  assert.equal(b.sira, 2);
  assert.deepEqual((await db.gunluk.liste({ sonra: 1 })).map((s) => s.sira), [2]);
  assert.deepEqual((await db.gunluk.liste({ enCok: 1 })).map((s) => s.sira), [1]);
  assert.deepEqual(await db.gunluk.say(), { toplam: 2, kopyalanmamis: 2 });
  for (const kotu of [{ olay: 'Büyük Harf' }, { olay: '' }, {}, { olay: 'a', yol: 'Merkez' }, { olay: 'a', ayrinti: 'metin' }, { olay: 'a', ayrinti: [] }, { olay: 'a', ayrinti: { x: 'y'.repeat(5000) } }]) {
    await assert.rejects(db.gunluk.yaz(kotu), TaaHata, JSON.stringify(kotu).slice(0, 60));
  }
  assert.equal((await db.gunluk.liste()).length, 2);
  // Kopya işareti satırları silmez ve geri gitmez.
  assert.equal(await db.gunluk.kopyalandi(2), 2);
  assert.equal(await db.gunluk.kopyalandi(1), 2);
  assert.deepEqual(await db.gunluk.say(), { toplam: 2, kopyalanmamis: 0 });
  assert.equal((await db.gunluk.liste()).length, 2, 'kopyalanan satır cihazda kalır');
  await assert.rejects(db.gunluk.kopyalandi(-1), TaaHata);
  for (const sinif of [Gunluk, Giden]) {
    const adlar = Object.getOwnPropertyNames(sinif.prototype);
    assert.deepEqual(adlar.filter((x) => /^(sil|temizle|bosalt|delete|clear|remove)/i.test(x)), [], sinif.name);
  }
  // Günlük şube + kullanıcıya özeldir (veritabanı gibi).
  assert.equal((await yeniDB().gunluk.liste()).length, 0);
});

test('günlük merkeze ulaşınca kopyalanır; satırlar cihazda kalır; ret sonrası bir süre yeniden denenmez', async () => {
  let simdi = 1_795_000_000_000;
  const db = yeniDB({ saat: () => simdi });
  for (const n of [1, 2]) await db.giden.ekle(kayit(n));
  const kapali = sahteAg(() => agYok());
  const secenek = { yollar: YOLLAR, oturum: 'gizli', sube: 'MRK01' };
  await db.giden.gonder({ ...secenek, fetch: kapali.getir });
  assert.deepEqual(await db.gunluk.say(), { toplam: 3, kopyalanmamis: 3 });
  assert.equal(kapali.cagrilar.filter((c) => c.yol === UC_GUNLUK_KOPYA).length, 0, 'merkeze ulaşılmadan kopya denenmez');

  const acik = sahteAg(() => 200);
  await db.giden.gonder({ ...secenek, fetch: acik.getir });
  const kopya = acik.cagrilar.filter((c) => c.yol === UC_GUNLUK_KOPYA);
  assert.equal(kopya.length, 1);
  assert.equal(kopya[0].koken, MERKEZ, 'günlük yalnız merkeze kopyalanır');
  assert.equal(kopya[0].govde.cihaz, 'test');
  assert.equal(kopya[0].govde.sube, 'mrk01');
  assert.equal(kopya[0].govde.no, `GNLC-test-1-${kopya[0].govde.son}`);
  assert.equal(kopya[0].basliklar['X-TAA-No'], kopya[0].govde.no);
  assert.deepEqual(kopya[0].govde.satirlar.map((s) => s.olay).slice(0, 3), ['yol.basarisiz', 'yol.basarisiz', 'yol.yerel']);
  const say = await db.gunluk.say();
  assert.equal(say.kopyalanmamis, 0);
  assert.equal(say.toplam, kopya[0].govde.satirlar.length, 'hiçbir satır silinmedi');

  // Merkez günlük ucunu tanımıyorsa (404): gönderim etkilenmez, satırlar bekler, 10 dakika yeniden denenmez.
  await db.gunluk.yaz({ olay: 'deneme' });
  await db.giden.ekle(kayit(3));
  const yok = sahteAg((c) => (c.yol === UC_GUNLUK_KOPYA ? 404 : 200));
  const o = await db.giden.gonder({ ...secenek, fetch: yok.getir });
  assert.equal(o.gonderilen, 1);
  assert.equal(o.durmaNedeni, null);
  assert.equal((await db.gunluk.say()).kopyalanmamis, 1);
  assert.equal((await db.giden.say()).reddedildi, 0, 'günlük kopyası giden kutusunu kirletmez');
  const once = (await db.gunluk.liste()).length;
  assert.equal((await gunlukKopyala(db.giden, db.gunluk, { merkez: MERKEZ, oturum: 'gizli', fetch: yok.getir, saat: () => simdi })).durum, 'ertelendi');
  simdi += 11 * 60 * 1000;
  const sonra = await gunlukKopyala(db.giden, db.gunluk, { merkez: MERKEZ, oturum: 'gizli', fetch: acik.getir, saat: () => simdi });
  assert.deepEqual(sonra, { kopyalanan: 1, durum: 'tamam' });
  assert.equal((await db.gunluk.liste()).length, once, 'kopya başarısızlığı günlüğe yazılmaz (kendini büyütmez)');
  assert.equal((await gunlukKopyala(db.giden, db.gunluk, { merkez: MERKEZ, oturum: '', fetch: acik.getir })).durum, 'yok');
});

test('eşzamanlı iki gönderim çağrısı tek tur yapar (çift POST yok)', async () => {
  const db = yeniDB();
  for (const n of [1, 2]) await db.giden.ekle(kayit(n));
  const ag = sahteAg(() => 200);
  const secenek = { yollar: YOLLAR, oturum: 'o', sube: 'MRK01', fetch: ag.getir };
  const [a, b] = await Promise.all([db.giden.gonder(secenek), db.giden.gonder(secenek)]);
  assert.deepEqual(a, b);
  assert.equal(ag.kayitlar().length, 2);
});

test('kabuk yol ayarı: okuma, form denetimi, gösterim', () => {
  const KOKEN = 'https://taa.ornek.test';
  assert.deepEqual(yolAyariOku(null, KOKEN), { yollar: [{ ad: 'merkez', url: KOKEN }, { ad: 'yerel' }], hatalar: [] });
  assert.equal(yolAyariOku('{bozuk', KOKEN).hatalar.length, 1);
  assert.deepEqual(yolAyariOku('{bozuk', KOKEN).yollar, [{ ad: 'merkez', url: KOKEN }, { ad: 'yerel' }]);
  // Saklanan merkez adresi ne olursa olsun merkez kabuğun kökenidir.
  const sakli = JSON.stringify([{ ad: 'merkez', url: 'https://baska.test' }, { ad: 'sube', url: SUBE, sube: 'MRK01' }, { ad: 'yerel' }]);
  assert.deepEqual(yolAyariOku(sakli, KOKEN).yollar, [{ ad: 'merkez', url: KOKEN }, { ad: 'sube', url: SUBE, sube: 'mrk01' }, { ad: 'yerel' }]);

  assert.deepEqual(yolAyariKur({}, KOKEN), { ayar: [{ ad: 'merkez', url: KOKEN }, { ad: 'yerel' }], hatalar: [] });
  assert.deepEqual(yolAyariKur({ subeAdresi: ' 192.168.1.20 ', subeKodu: 'MRK01' }, KOKEN).ayar, [
    { ad: 'merkez', url: KOKEN },
    { ad: 'sube', url: SUBE, sube: 'mrk01' },
    { ad: 'yerel' },
  ]);
  assert.equal(yolAyariKur({ subeAdresi: 'https://kasa-pc.sube.test' }, KOKEN).ayar[1].url, 'https://kasa-pc.sube.test:8788');
  assert.equal(yolAyariKur({ subeAdresi: 'http://[fd00::1]' }, KOKEN).ayar[1].url, 'http://[fd00::1]:8788');
  for (const kotu of [{ subeAdresi: 'http://192.168.1.20:9000' }, { subeAdresi: 'http://x/yol' }, { subeAdresi: 'ftp://x' }, { subeKodu: 'MRK01' }, { subeAdresi: '192.168.1.20', subeKodu: 'şube 1' }]) {
    const s = yolAyariKur(kotu, KOKEN);
    assert.equal(s.ayar, null, JSON.stringify(kotu));
    assert.ok(s.hatalar.length > 0);
  }
  assert.equal(yolAyariKur({ subeAdresi: 'https://taa.ornek.test:8788' }, 'https://taa.ornek.test:8788').ayar, null, 'şube = merkez olamaz');
  // Kaydedilen ayar geri okununca aynı liste çıkar.
  const kur = yolAyariKur({ subeAdresi: SUBE, subeKodu: 'MRK01' }, KOKEN).ayar;
  assert.deepEqual(yolAyariOku(JSON.stringify(kur), KOKEN).yollar, kur);

  assert.deepEqual(yolSatirlari(kur).map((s) => [s.ad, s.var]), [['merkez', true], ['sube', true], ['yerel', true]]);
  assert.match(yolSatirlari(kur)[1].metin, /^Şube bilgisayarı: http:\/\/192\.168\.1\.20:8788 \(şube mrk01\)$/);
  assert.deepEqual(yolSatirlari([{ ad: 'merkez', url: KOKEN }, { ad: 'yerel' }])[1], { ad: 'sube', metin: 'Şube bilgisayarı: tanımlı değil', var: false });
  assert.equal(yolNotu({}), '');
  assert.match(yolNotu({ sonYol: 'sube', subeyeTeslim: 3 }), /şube bilgisayarı üzerinden.*teslim edilen 3 kayıt/);
});

test('kabuk tutarlılığı: sw.js yol.js dosyasını önbelleğe alır; güvenlik politikası şube kapısına izin verir', async () => {
  const { readFileSync } = await import('node:fs');
  const oku = (ad) => readFileSync(new URL(`../kabuk/${ad}`, import.meta.url), 'utf8');
  assert.match(oku('sw.js'), /'\.\.\/cekirdek\/yol\.js'/, 'yol.js çevrimdışı açılış için önbellekte olmalı');
  const csp = /Content-Security-Policy" content="([^"]+)"/.exec(oku('index.html'))[1];
  const baglanti = /connect-src ([^;]+)/.exec(csp)[1].split(' ');
  assert.deepEqual(baglanti, ["'self'", 'http://*:8788', 'https://*:8788'], 'yalnız kendi kökeni ve şube düğümü kapısı');
  // Ayar formu politikanın izin vermediği bir kapıyı kabul etmez (kabul etse istek sessizce engellenirdi).
  assert.equal(yolAyariKur({ subeAdresi: 'http://192.168.1.20:8787' }, 'https://taa.ornek.test').ayar, null);
  const html = oku('index.html');
  for (const kimlik of ['yol-sirasi', 'yol-sube', 'yol-sube-kodu', 'yol-kaydet', 'yol-hata', 'yol-durum', 'giden-yol']) {
    assert.ok(html.includes(`id="${kimlik}"`), kimlik);
  }
  assert.doesNotMatch(readFileSync(new URL('../cekirdek/yol.js', import.meta.url), 'utf8'), /indexedDB|deleteDatabase|localStorage|innerHTML/);
});
