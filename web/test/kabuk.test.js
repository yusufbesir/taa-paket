// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W0-kabuk 2026-11-27]
// Kabuk testleri: saf yardımcılar + dosya tutarlılığı (sw.js listesi, moduller.json, manifest, index.html kimlikleri).
// Tarayıcı davranışı (service worker, IndexedDB, çekmeceler) burada SINANMAZ; bkz. web/kabuk/BENIOKU.md.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync, readdirSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import {
  rotaCoz,
  modulListesi,
  dosyaYoluGecerli,
  yetkiVar,
  girisYanitiCoz,
  dbParcasi,
  profilGecerli,
  gostergeMetni,
  cikisUyarisi,
  modulDosyalari,
} from '../kabuk/yardimci.js';
import { acDB } from '../cekirdek/taa-cekirdek.js';

const WEB = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const KABUK = path.join(WEB, 'kabuk');
const oku = (ad) => readFileSync(path.join(KABUK, ad), 'utf8');

test('rotaCoz: hash -> modül adı ve alt yol', () => {
  assert.deepEqual(rotaCoz('#/kasa'), { ad: 'kasa', alt: [] });
  assert.deepEqual(rotaCoz('#/kasa/defter/3'), { ad: 'kasa', alt: ['defter', '3'] });
  assert.deepEqual(rotaCoz('#/kasa?x=1'), { ad: 'kasa', alt: [] });
  for (const bos of ['', '#', '#/', undefined, '#/../x', '#/Kasa', '#/a b', '#/<script>']) {
    assert.deepEqual(rotaCoz(bos), { ad: null, alt: [] }, String(bos));
  }
});

test('modulListesi: bozuk ve yinelenen girişler atılır', () => {
  const l = modulListesi({
    moduller: [
      { ad: 'kasa', baslik: ' TAA Kasa ', dosyalar: ['moduller/kasa/mantik.js', '../sw.js', 'https://x/y.js', 5] },
      { ad: 'kasa', baslik: 'ikinci' },
      { ad: '../kotu', baslik: 'x' },
      { ad: 'cati', izin: 'yonetici' },
      null,
      'metin',
    ],
  });
  assert.deepEqual(l, [
    { ad: 'kasa', baslik: 'TAA Kasa', izin: null, aciklama: '', dosyalar: ['moduller/kasa/mantik.js'] },
    { ad: 'cati', baslik: 'cati', izin: 'yonetici', aciklama: '', dosyalar: [] },
  ]);
  assert.deepEqual(modulListesi(null), []);
  assert.deepEqual(modulListesi({ moduller: 'x' }), []);
});

test('dosyaYoluGecerli: yalnız web/moduller altı', () => {
  assert.equal(dosyaYoluGecerli('moduller/ortak/dom.js'), true);
  for (const k of ['kabuk/sw.js', 'moduller/../kabuk/sw.js', '/moduller/a.js', 'moduller//a.js', 'moduller/a.js?x', 'v1/giris', '']) {
    assert.equal(dosyaYoluGecerli(k), false, k);
  }
});

test('modulDosyalari: index.js her zaman, ek dosyalar yinelenmeden', () => {
  const l = modulListesi({
    moduller: [
      { ad: 'kasa', dosyalar: ['moduller/kasa/index.js', 'moduller/ortak/dom.js'] },
      { ad: 'rehber', dosyalar: ['moduller/ortak/dom.js'] },
    ],
  });
  assert.deepEqual(modulDosyalari(l), [
    '../moduller/kasa/index.js',
    '../moduller/ortak/dom.js',
    '../moduller/rehber/index.js',
  ]);
});

test('yetkiVar: izin, rol, * ve önek', () => {
  const p = { izinler: ['kasa.sayim', 'fis.*'], roller: ['yonetici'] };
  assert.equal(yetkiVar(p, 'kasa.sayim'), true);
  assert.equal(yetkiVar(p, 'kasa.masraf'), false);
  assert.equal(yetkiVar(p, 'fis.iptal'), true);
  assert.equal(yetkiVar(p, 'fisx'), false);
  assert.equal(yetkiVar(p, 'yonetici'), true);
  assert.equal(yetkiVar({ izinler: ['*'] }, 'her.sey'), true);
  assert.equal(yetkiVar(null, 'kasa.sayim'), false);
  assert.equal(yetkiVar(p, ''), false);
  assert.equal(yetkiVar({}, 'kasa.sayim'), false);
});

test('girisYanitiCoz: T7 yanıtı profile çevrilir, anahtar profile girmez', () => {
  const s = girisYanitiCoz({ oturum: 'gizli', sube_id: 3, roller: ['kasiyer', 5] }, 'ayse', 1000);
  assert.equal(s.oturum, 'gizli');
  assert.deepEqual(s.profil, { kullanici: 'ayse', sube: '3', roller: ['kasiyer'], izinler: [], zaman: 1000 });
  assert.equal(JSON.stringify(s.profil).includes('gizli'), false);
  assert.throws(() => girisYanitiCoz({ sube_id: 3 }, 'ayse'));
  assert.throws(() => girisYanitiCoz({ oturum: 'x' }, 'ayse'));
  assert.throws(() => girisYanitiCoz(null, 'ayse'));
});

test('dbParcasi: çekirdeğin kabul ettiği, çakışmayan parça üretir', () => {
  assert.equal(dbParcasi('S1'), 'S1');
  assert.equal(dbParcasi('ana-sube'), 'ana_2dsube');
  assert.equal(dbParcasi('a_b'), 'a_5fb');
  assert.notEqual(dbParcasi('a-b'), dbParcasi('a_2db'));
  assert.equal(dbParcasi('ayşe'), 'ay_c5_9fe');
  assert.throws(() => dbParcasi(''));
  assert.throws(() => dbParcasi('x'.repeat(49)));
  // Tireli iki farklı (şube, kullanıcı) çifti ayrı veritabanına gider.
  const a = acDB(dbParcasi('a-b'), dbParcasi('c'));
  const b = acDB(dbParcasi('a'), dbParcasi('b-c'));
  assert.notEqual(a.ad, b.ad);
});

test('profilGecerli: 7 gün çevrimdışı süre', () => {
  const gun = 24 * 60 * 60 * 1000;
  const p = { kullanici: 'ayse', sube: '3', zaman: 100 * gun };
  assert.equal(profilGecerli(p, 100 * gun), true);
  assert.equal(profilGecerli(p, 106.9 * gun), true);
  assert.equal(profilGecerli(p, 107 * gun), false);
  assert.equal(profilGecerli(p, 98 * gun), false, 'saat çok geri alınmış');
  assert.equal(profilGecerli(null), false);
  assert.equal(profilGecerli({ kullanici: 'a', sube: '1' }), false);
});

test('gostergeMetni: "Gönderilmeyen N kayıt"', () => {
  assert.deepEqual(gostergeMetni({ cevrimici: true }), { metin: 'Tüm kayıtlar gönderildi', tur: 'tamam' });
  assert.deepEqual(gostergeMetni({ cevrimici: true, bekleyen: 3 }), { metin: 'Gönderilmeyen 3 kayıt', tur: 'uyari' });
  assert.deepEqual(gostergeMetni({ cevrimici: false, bekleyen: 3 }), {
    metin: 'Çevrimdışı · Gönderilmeyen 3 kayıt',
    tur: 'uyari',
  });
  assert.deepEqual(gostergeMetni({ cevrimici: false }), { metin: 'Çevrimdışı', tur: 'uyari' });
  assert.equal(gostergeMetni({ cevrimici: true, reddedilen: 1 }).tur, 'hata');
  assert.match(gostergeMetni({ cevrimici: true, bekleyen: 2, oturumVar: false }).metin, /giriş yapın/);
  assert.equal(gostergeMetni({ cevrimici: true, kalici: false }).tur, 'hata');
});

test('cikisUyarisi: bekleyen varsa uyarı, yoksa null', () => {
  assert.equal(cikisUyarisi(0), null);
  assert.match(cikisUyarisi(4), /Gönderilmemiş 4 kayıt/);
});

test('sw.js: KABUK listesindeki her dosya var; kabuk klasöründeki her dosya listede', () => {
  const sw = oku('sw.js');
  const govde = sw.match(/const KABUK = \[([\s\S]*?)\];/)[1];
  const liste = [...govde.matchAll(/'([^']+)'/g)].map((m) => m[1]);
  assert.ok(liste.includes('./index.html') && liste.includes('../cekirdek/taa-cekirdek.js'));
  for (const yol of liste) {
    if (yol === './') continue;
    assert.ok(existsSync(path.join(KABUK, yol)), `sw.js listesinde olup diskte olmayan: ${yol}`);
  }
  const haric = new Set(['sw.js', 'BENIOKU.md']);
  for (const ad of readdirSync(KABUK)) {
    if (haric.has(ad)) continue;
    assert.ok(liste.includes(`./${ad}`), `kabuk dosyası sw.js listesinde yok (çevrimdışı açılmaz): ${ad}`);
  }
  assert.match(sw, /\/v1\\\//, 'veri uçları önbellek dışı tutulmalı');
});

test('moduller.json: geçerli ve listelenen her dosya diskte', () => {
  const ham = JSON.parse(oku('moduller.json'));
  const liste = modulListesi(ham);
  assert.equal(liste.length, ham.moduller.length, 'atılan giriş olmamalı');
  for (const m of liste) {
    assert.equal(m.dosyalar.length, ham.moduller.find((x) => x.ad === m.ad).dosyalar?.length ?? 0, `${m.ad}: geçersiz dosya yolu`);
  }
  for (const yol of modulDosyalari(liste)) {
    assert.ok(existsSync(path.join(KABUK, yol)), `moduller.json içinde olup diskte olmayan: ${yol}`);
  }
});

test('modüller sözleşmeye uyar: {ad, baslik, ac} dışa aktarır', async () => {
  for (const m of modulListesi(JSON.parse(oku('moduller.json')))) {
    const dosya = await import(path.join(WEB, 'moduller', m.ad, 'index.js'));
    const modul = typeof dosya.ac === 'function' ? dosya : dosya.default;
    assert.equal(typeof modul?.ac, 'function', `${m.ad}: ac yok`);
    assert.equal(modul.ad, m.ad, `${m.ad}: ad uyuşmuyor`);
    assert.equal(typeof modul.baslik, 'string');
  }
});

test('manifest: geçerli JSON, simgeler diskte', () => {
  const m = JSON.parse(oku('manifest.webmanifest'));
  assert.equal(m.lang, 'tr');
  assert.equal(m.display, 'standalone');
  assert.ok(m.icons.some((i) => i.sizes === '192x192') && m.icons.some((i) => i.sizes === '512x512'));
  for (const i of m.icons) assert.ok(existsSync(path.join(KABUK, i.src)), i.src);
  assert.ok(existsSync(path.join(KABUK, m.start_url)));
});

test('index.html: uygulama.js içinde kullanılan her kimlik sayfada var; satır içi betik yok', () => {
  const html = oku('index.html');
  const js = oku('uygulama.js');
  const kimlikler = new Set([...js.matchAll(/\$\('([a-z0-9-]+)'\)/g)].map((m) => m[1]));
  assert.ok(kimlikler.size > 20);
  for (const k of kimlikler) assert.ok(html.includes(`id="${k}"`), `index.html içinde id="${k}" yok`);
  assert.match(html, /<html lang="tr"/);
  assert.match(html, /Content-Security-Policy/);
  assert.doesNotMatch(html, /<script(?![^>]*\bsrc=)[^>]*>/, 'CSP gereği satır içi betik olmamalı');
  assert.doesNotMatch(html, /\son[a-z]+="/, 'satır içi olay işleyici olmamalı');
  for (const m of html.matchAll(/(?:src|href)="([^"#][^"]*)"/g)) {
    assert.ok(existsSync(path.join(KABUK, m[1])), `index.html şu dosyayı istiyor ama yok: ${m[1]}`);
  }
});

test('kabuk kodu innerHTML ve silme çağrısı kullanmaz', () => {
  for (const ad of ['uygulama.js', 'yardimci.js', 'tema.js']) {
    const s = oku(ad);
    assert.doesNotMatch(s, /innerHTML|outerHTML|insertAdjacentHTML|document\.write/, ad);
    assert.doesNotMatch(s, /deleteDatabase|indexedDB/, ad);
  }
  assert.doesNotMatch(readFileSync(path.join(WEB, 'cekirdek', 'taa-cekirdek.js'), 'utf8'), /deleteDatabase|\.delete\(|\.clear\(/);
});
