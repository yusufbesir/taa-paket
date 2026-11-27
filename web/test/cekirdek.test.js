// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W0-cekirdek 2026-11-27]
// Çekirdek testleri (bellek içi sürücü). Çalıştırma: node --test web/test/
import test from 'node:test';
import assert from 'node:assert/strict';
import {
  acDB,
  olaylar,
  OLAY_GIDEN_DEGISTI,
  noGecerli,
  ucGecerli,
  TaaHata,
} from '../cekirdek/taa-cekirdek.js';

let sayac = 0;
/** Her test kendi şubesini kullanır (bellek veritabanları ad bazında paylaşılır). */
function yeniDB(secenek = {}) {
  sayac += 1;
  return acDB(`s${sayac}`, 'ayse', { cihaz: 'test', ...secenek });
}

/** Sahte fetch: `plan(url, secenek, sira)` -> durum kodu | {durum, govde} | Error (fırlatılır). */
function sahteFetch(plan) {
  const cagrilar = [];
  const getir = async (url, secenek) => {
    cagrilar.push({ url, secenek, govde: JSON.parse(secenek.body) });
    const s = plan(url, secenek, cagrilar.length);
    if (s instanceof Error) throw s;
    const durum = typeof s === 'number' ? s : s.durum;
    const govde = typeof s === 'number' ? '' : s.govde ?? '';
    return { status: durum, ok: durum >= 200 && durum <= 299, text: async () => govde };
  };
  return { getir, cagrilar };
}

const SUNUCU = 'https://sunucu.test';
const kayit = (n) => ({ program: 'kasa', uc: '/v1/kasa/sayim', govde: { n } });

test('acDB: ad "taa-<sube>-<kullanici>" biçimindedir', () => {
  const db = acDB('merkez', 'ayse', { cihaz: 'test' });
  assert.equal(db.ad, 'taa-merkez-ayse');
  assert.equal(db.kalici, false, 'Node testinde bellek içi sürücü');
});

test('acDB: tireli ya da boş şube/kullanıcı reddedilir (adlar çakışmasın)', () => {
  for (const [s, k] of [['a-b', 'c'], ['a', 'b-c'], ['', 'c'], ['a', ''], ['a/b', 'c'], ['..', 'c'], [undefined, 'c']]) {
    assert.throws(() => acDB(s, k), TaaHata, `${s} / ${k}`);
  }
});

test('ekle: {no, durum: "bekliyor"} döner; no cihaz-sayaç-zaman biçimindedir', async () => {
  const db = yeniDB({ saat: () => 1795000000000 });
  const s = await db.giden.ekle(kayit(1));
  assert.deepEqual(s, { no: 'test-1-1795000000000', durum: 'bekliyor' });
  assert.ok(noGecerli(s.no));
  const s2 = await db.giden.ekle(kayit(2));
  assert.equal(s2.no, 'test-2-1795000000000', 'sayaç artar; aynı milisaniyede de no tekildir');
});

test('ekle: cihaz verilmezse kimlik üretilir, kalıcıdır ve no kurala uyar', async () => {
  sayac += 1;
  const db = acDB(`s${sayac}`, 'ayse');
  const a = await db.giden.ekle(kayit(1));
  const b = await db.giden.ekle(kayit(2));
  assert.ok(noGecerli(a.no) && a.no.length <= 64);
  assert.match(a.no, /^w[a-z0-9]{11}-1-\d+$/);
  assert.equal(a.no.split('-')[0], b.no.split('-')[0]);
  const yeniden = acDB(`s${sayac}`, 'ayse');
  assert.equal(await yeniden.giden.cihaz(), a.no.split('-')[0], 'yeniden açınca aynı cihaz kimliği');
});

test('çift ekleme: aynı no ikinci kez eklenmez', async () => {
  const db = yeniDB();
  const ilk = await db.giden.ekle({ ...kayit(1), no: 'FIS-2026-000123' });
  assert.deepEqual(ilk, { no: 'FIS-2026-000123', durum: 'bekliyor' });
  const ikinci = await db.giden.ekle({ ...kayit(999), no: 'FIS-2026-000123' });
  assert.deepEqual(ikinci, { no: 'FIS-2026-000123', durum: 'bekliyor', zatenVar: true });
  const liste = await db.giden.liste();
  assert.equal(liste.length, 1);
  assert.deepEqual(liste[0].govde, { n: 1 }, 'ilk gövde ezilmez');
});

test('çift ekleme: aynı anda iki ekleme de tek kayıt bırakır', async () => {
  const db = yeniDB();
  const [a, b] = await Promise.all([
    db.giden.ekle({ ...kayit(1), no: 'ayni' }),
    db.giden.ekle({ ...kayit(2), no: 'ayni' }),
  ]);
  assert.equal([a, b].filter((x) => x.zatenVar).length, 1);
  assert.equal((await db.giden.liste()).length, 1);
});

test('çift ekleme: gönderilmiş kayıt aynı no ile yeniden bekleyene dönmez', async () => {
  const db = yeniDB();
  await db.giden.ekle({ ...kayit(1), no: 'bir' });
  await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: sahteFetch(() => 200).getir });
  const tekrar = await db.giden.ekle({ ...kayit(1), no: 'bir' });
  assert.deepEqual(tekrar, { no: 'bir', durum: 'gonderildi', zatenVar: true });
  assert.equal((await db.giden.liste('bekliyor')).length, 0);
});

test('ekle: geçersiz girdiler reddedilir, kutuya bir şey yazılmaz', async () => {
  const db = yeniDB();
  const kotu = [
    { program: 'Kasa', uc: '/v1/a', govde: {} },
    { program: 'kasa', uc: '/v2/a', govde: {} },
    { program: 'kasa', uc: '/v1/../a', govde: {} },
    { program: 'kasa', uc: '/v1//a', govde: {} },
    { program: 'kasa', uc: '/v1/a?x=1', govde: {} },
    { program: 'kasa', uc: 'https://baska.test/v1/a', govde: {} },
    { program: 'kasa', uc: '/v1/a', govde: [] },
    { program: 'kasa', uc: '/v1/a', govde: 'metin' },
    { program: 'kasa', uc: '/v1/a', govde: null },
    { program: 'kasa', uc: '/v1/a', govde: {}, no: 'boşluk var' },
    { program: 'kasa', uc: '/v1/a', govde: {}, no: 'x'.repeat(65) },
    { program: 'kasa', uc: '/v1/a', govde: { b: 'x'.repeat(1024 * 1024) } },
  ];
  for (const k of kotu) {
    await assert.rejects(db.giden.ekle(k), TaaHata, JSON.stringify(k).slice(0, 80));
  }
  assert.equal((await db.giden.liste()).length, 0);
  assert.equal(ucGecerli('/v1/kasa/sayim'), true);
});

test('gonder: sırayla POST <sunucu><uc>; 2xx gonderildi olur', async () => {
  const db = yeniDB();
  for (const n of [1, 2, 3]) await db.giden.ekle({ program: 'kasa', uc: `/v1/kasa/k${n}`, govde: { n } });
  const { getir, cagrilar } = sahteFetch(() => ({ durum: 201, govde: '{"tamam":true}' }));
  const ozet = await db.giden.gonder({ sunucu: `${SUNUCU}/`, oturum: 'gizli', fetch: getir });
  assert.deepEqual(ozet, { gonderilen: 3, reddedilen: 0, kalan: 0, durmaNedeni: null });
  assert.deepEqual(cagrilar.map((c) => c.url), [1, 2, 3].map((n) => `${SUNUCU}/v1/kasa/k${n}`));
  const ilk = cagrilar[0];
  assert.equal(ilk.secenek.method, 'POST');
  assert.equal(ilk.secenek.headers.Authorization, 'Bearer gizli');
  assert.equal(ilk.secenek.headers['X-TAA-No'], ilk.govde.no);
  assert.equal(ilk.secenek.redirect, 'error');
  assert.equal(ilk.govde.n, 1);
  const gonderilen = await db.giden.liste('gonderildi');
  assert.equal(gonderilen.length, 3);
  assert.deepEqual(gonderilen[0].gonderim.yanit, { tamam: true });
  assert.equal(gonderilen[0].govde.no, undefined, 'saklanan gövde değişmez');
});

test('gonder: gövdede no varsa modülün no değeri korunur', async () => {
  const db = yeniDB();
  await db.giden.ekle({ program: 'kasa', uc: '/v1/a', govde: { no: 'modul-no', x: 1 }, no: 'kutu-no' });
  const { getir, cagrilar } = sahteFetch(() => 200);
  await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir });
  assert.equal(cagrilar[0].govde.no, 'modul-no');
  assert.equal(cagrilar[0].secenek.headers['X-TAA-No'], 'kutu-no');
});

test('sıra bozulmaz: 5xx alan kayıtta durur, sonrakiler gönderilmez; sonra kaldığı yerden sürer', async () => {
  const db = yeniDB();
  for (const n of [1, 2, 3, 4]) await db.giden.ekle(kayit(n));
  let sunucuAyakta = false;
  const { getir, cagrilar } = sahteFetch((_u, s) => {
    const n = JSON.parse(s.body).n;
    return n === 2 && !sunucuAyakta ? 503 : 200;
  });
  const ilk = await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir });
  assert.equal(ilk.gonderilen, 1);
  assert.equal(ilk.kalan, 3);
  assert.equal(ilk.durmaKodu, 503);
  assert.match(ilk.durmaNedeni, /503/);
  assert.deepEqual(cagrilar.map((c) => c.govde.n), [1, 2], '3 ve 4 denenmedi');
  assert.deepEqual((await db.giden.liste('bekliyor')).map((k) => k.govde.n), [2, 3, 4]);

  sunucuAyakta = true;
  const ikinci = await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir });
  assert.deepEqual(ikinci, { gonderilen: 3, reddedilen: 0, kalan: 0, durmaNedeni: null });
  assert.deepEqual(cagrilar.map((c) => c.govde.n), [1, 2, 2, 3, 4], 'kayıt 1 ikinci kez gönderilmedi');
  assert.equal(cagrilar[1].govde.no, cagrilar[2].govde.no, 'yeniden denemede aynı no (sunucu çift yazmaz)');
});

test('4xx reddedilir: kayıt reddedildi olur, sıra devam eder, yeniden denenmez', async () => {
  const db = yeniDB();
  for (const n of [1, 2, 3]) await db.giden.ekle(kayit(n));
  const { getir, cagrilar } = sahteFetch((_u, s) =>
    JSON.parse(s.body).n === 2 ? { durum: 422, govde: 'tutar eksik' } : 200,
  );
  const ozet = await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir });
  assert.deepEqual(ozet, { gonderilen: 2, reddedilen: 1, kalan: 0, durmaNedeni: null });
  const red = await db.giden.liste('reddedildi');
  assert.equal(red.length, 1);
  assert.equal(red[0].govde.n, 2);
  assert.equal(red[0].ret.kod, 422);
  assert.equal(red[0].ret.mesaj, 'tutar eksik');
  await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir });
  assert.equal(cagrilar.length, 3, 'reddedilen yeniden gönderilmez');
  assert.equal((await db.giden.liste()).length, 3, 'reddedilen silinmez');
});

test('4xx: 400, 403, 404, 409, 422 reddedilir; 408 ve 429 bekler', async () => {
  for (const kod of [400, 403, 404, 409, 422]) {
    const db = yeniDB();
    await db.giden.ekle(kayit(1));
    const o = await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: sahteFetch(() => kod).getir });
    assert.equal(o.reddedilen, 1, `HTTP ${kod}`);
    assert.equal((await db.giden.liste('reddedildi'))[0].ret.kod, kod);
  }
  for (const kod of [408, 429, 500, 502, 301, 0]) {
    const db = yeniDB();
    await db.giden.ekle(kayit(1));
    const o = await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: sahteFetch(() => kod).getir });
    assert.equal(o.reddedilen, 0, `HTTP ${kod}`);
    assert.equal(o.kalan, 1, `HTTP ${kod}`);
    assert.ok(o.durmaNedeni, `HTTP ${kod}`);
    assert.equal((await db.giden.liste('bekliyor')).length, 1, `HTTP ${kod}`);
  }
});

test('401 (oturum geçersiz): durur, hiçbir kayıt reddedilmez', async () => {
  const db = yeniDB();
  for (const n of [1, 2]) await db.giden.ekle(kayit(n));
  const { getir, cagrilar } = sahteFetch(() => 401);
  const o = await db.giden.gonder({ sunucu: SUNUCU, oturum: 'eski', fetch: getir });
  assert.equal(o.durmaKodu, 401);
  assert.equal(o.kalan, 2);
  assert.equal(cagrilar.length, 1);
  assert.equal((await db.giden.liste('reddedildi')).length, 0);
});

test('oturum yokken ağa hiç çıkılmaz, kayıtlar bekler', async () => {
  const db = yeniDB();
  await db.giden.ekle(kayit(1));
  const { getir, cagrilar } = sahteFetch(() => 200);
  for (const oturum of [undefined, '', 'a\r\nX-Kotu: 1']) {
    const o = await db.giden.gonder({ sunucu: SUNUCU, oturum, fetch: getir });
    assert.equal(o.kalan, 1);
    assert.ok(o.durmaNedeni);
  }
  assert.equal(cagrilar.length, 0);
});

test('ağ kopunca durur: fetch hata fırlatırsa kayıtlar bekliyor kalır', async () => {
  const db = yeniDB();
  for (const n of [1, 2, 3]) await db.giden.ekle(kayit(n));
  let agVar = true;
  const { getir, cagrilar } = sahteFetch((_u, _s, sira) => {
    if (sira === 2) agVar = false;
    return agVar ? 200 : new TypeError('fetch failed');
  });
  const o = await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir });
  assert.equal(o.gonderilen, 1);
  assert.equal(o.reddedilen, 0);
  assert.equal(o.kalan, 2);
  assert.match(o.durmaNedeni, /ulaşılamadı/);
  assert.equal(cagrilar.length, 2, 'kopan kayıttan sonrası denenmedi');
  assert.deepEqual((await db.giden.liste('bekliyor')).map((k) => k.govde.n), [2, 3]);

  agVar = true;
  const { getir: getir2, cagrilar: c2 } = sahteFetch(() => 200);
  const o2 = await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir2 });
  assert.equal(o2.gonderilen, 2);
  assert.deepEqual(c2.map((c) => c.govde.n), [2, 3]);
});

test('ağ kopunca durur: yanıt gövdesi okunurken kopma da gönderildi sayılmaz', async () => {
  const db = yeniDB();
  await db.giden.ekle(kayit(1));
  const getir = async () => ({
    status: 200,
    text: async () => {
      throw new TypeError('terminated');
    },
  });
  const o = await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir });
  assert.equal(o.gonderilen, 0);
  assert.equal((await db.giden.liste('bekliyor')).length, 1);
});

test('zaman aşımı: yanıt gelmezse durur', async () => {
  const db = yeniDB();
  await db.giden.ekle(kayit(1));
  const getir = (_u, s) =>
    new Promise((_coz, reddet) => {
      s.signal.addEventListener('abort', () => reddet(new Error('iptal')));
    });
  const o = await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir, zamanAsimiMs: 20 });
  assert.equal(o.kalan, 1);
  assert.match(o.durmaNedeni, /zamanında/);
});

test('gonder: aynı anda iki çağrı kayıtları iki kez göndermez', async () => {
  const db = yeniDB();
  for (const n of [1, 2]) await db.giden.ekle(kayit(n));
  const { getir, cagrilar } = sahteFetch(() => 200);
  const [a, b] = await Promise.all([
    db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir }),
    db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir }),
  ]);
  assert.equal(cagrilar.length, 2);
  assert.deepEqual(a, b);
});

test('gonder: enCok sınırı ve geçersiz sunucu', async () => {
  const db = yeniDB();
  for (const n of [1, 2, 3]) await db.giden.ekle(kayit(n));
  const { getir } = sahteFetch(() => 200);
  const o = await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir, enCok: 2 });
  assert.equal(o.gonderilen, 2);
  assert.equal(o.kalan, 1);
  await assert.rejects(db.giden.gonder({ sunucu: 'sunucu.test', oturum: 'o', fetch: getir }), TaaHata);
  await assert.rejects(db.giden.gonder({ oturum: 'o', fetch: getir }), TaaHata);
});

test('liste(durum): süzer, ekleme sırasını korur, geçersiz durumu reddeder', async () => {
  const db = yeniDB();
  for (const n of [1, 2, 3]) await db.giden.ekle(kayit(n));
  await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: sahteFetch(() => 200).getir, enCok: 1 });
  assert.deepEqual((await db.giden.liste()).map((k) => k.govde.n), [1, 2, 3]);
  assert.deepEqual((await db.giden.liste('gonderildi')).map((k) => k.govde.n), [1]);
  assert.deepEqual((await db.giden.liste('bekliyor')).map((k) => k.govde.n), [2, 3]);
  assert.deepEqual(await db.giden.say(), { bekliyor: 2, gonderildi: 1, reddedildi: 0 });
  await assert.rejects(db.giden.liste('silindi'), TaaHata);
});

test('liste: döndürülen kayıt değiştirilse de kutu değişmez', async () => {
  const db = yeniDB();
  await db.giden.ekle(kayit(1));
  const [k] = await db.giden.liste();
  k.durum = 'gonderildi';
  k.govde.n = 99;
  const [yine] = await db.giden.liste();
  assert.equal(yine.durum, 'bekliyor');
  assert.equal(yine.govde.n, 1);
});

test('şube veritabanları ayrı: bir şubenin kaydı ve önbelleği diğerinde görünmez', async () => {
  const a = acDB('subeA', 'ayse', { cihaz: 'test' });
  const b = acDB('subeB', 'ayse', { cihaz: 'test' });
  const c = acDB('subeA', 'fatma', { cihaz: 'test' });
  assert.notEqual(a.ad, b.ad);
  await a.giden.ekle({ ...kayit(1), no: 'ortak-no' });
  await a.onbellek.yaz('kisi/1', { ad: 'A şubesi' }, 5);
  assert.equal((await b.giden.liste()).length, 0);
  assert.equal((await c.giden.liste()).length, 0, 'aynı şubede başka kullanıcı da ayrı');
  assert.equal(await b.onbellek.oku('kisi/1'), undefined);
  assert.equal(await c.onbellek.oku('kisi/1'), undefined);

  const bSonuc = await b.giden.ekle({ ...kayit(2), no: 'ortak-no' });
  assert.equal(bSonuc.zatenVar, undefined, 'aynı no başka şubede ayrı kayıttır');

  const { getir, cagrilar } = sahteFetch(() => 200);
  await b.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: getir });
  assert.deepEqual(cagrilar.map((x) => x.govde.n), [2], 'B gönderirken A kaydı gitmez');
  assert.equal((await a.giden.liste('bekliyor')).length, 1);

  const aYeniden = acDB('subeA', 'ayse', { cihaz: 'test' });
  assert.equal((await aYeniden.giden.liste()).length, 1, 'aynı ad aynı veriyi görür');
});

test('olay: ekleme ve gönderimde taa-giden-degisti yayınlanır', async () => {
  const db = yeniDB();
  const gelen = [];
  const dinle = (o) => {
    if (o.detail.db === db.ad) gelen.push(o.detail.neden);
  };
  olaylar.addEventListener(OLAY_GIDEN_DEGISTI, dinle);
  try {
    await db.giden.ekle({ ...kayit(1), no: 'n1' });
    await db.giden.ekle({ ...kayit(1), no: 'n1' });
    await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: sahteFetch(() => 200).getir });
    await db.giden.gonder({ sunucu: SUNUCU, oturum: 'o', fetch: sahteFetch(() => 200).getir });
  } finally {
    olaylar.removeEventListener(OLAY_GIDEN_DEGISTI, dinle);
  }
  assert.deepEqual(gelen, ['eklendi', 'gonderildi'], 'çift ekleme ve boş gönderim olay üretmez');
});

test('Onbellek: oku/yaz, sürüm geri gitmez', async () => {
  const db = yeniDB();
  assert.equal(await db.onbellek.oku('yok'), undefined);
  assert.equal(await db.onbellek.yaz('ayar', { tema: 'acik' }, 3), true);
  assert.deepEqual(await db.onbellek.oku('ayar'), { tema: 'acik' });
  assert.equal(await db.onbellek.yaz('ayar', { tema: 'eski' }, 2), false, 'eski sürüm yeniyi ezmez');
  assert.deepEqual(await db.onbellek.oku('ayar'), { tema: 'acik' });
  assert.equal(await db.onbellek.yaz('ayar', { tema: 'koyu' }, 4), true);
  assert.equal((await db.onbellek.okuKayit('ayar')).surum, 4);
  assert.equal(await db.onbellek.yaz('ayar', { tema: 'x' }), true, 'sürümsüz yazma sürümü korur');
  assert.equal((await db.onbellek.okuKayit('ayar')).surum, 4);
  assert.equal(await db.onbellek.yaz('sifir', 0, 0), true);
  assert.equal(await db.onbellek.oku('sifir'), 0);
  await assert.rejects(db.onbellek.yaz('', 1, 1), TaaHata);
  await assert.rejects(db.onbellek.yaz('a', 1, -1), TaaHata);
  await assert.rejects(db.onbellek.yaz('a', undefined, 1), TaaHata);
});

test('silme yok: çekirdek hiçbir silme işlevi sunmaz', () => {
  const db = yeniDB();
  for (const nesne of [db.giden, db.onbellek, db.surucu]) {
    const adlar = Object.getOwnPropertyNames(Object.getPrototypeOf(nesne));
    assert.deepEqual(adlar.filter((a) => /^(sil|temizle|bosalt|delete|clear|remove)/i.test(a)), []);
  }
});
