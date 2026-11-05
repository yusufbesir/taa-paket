// Sahip: Yusuf. yazan: [Claude#taa-paket-T2 2026-11-05]
'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const v = require('../taa-veri.js');

function ortam() {
  const kok = fs.mkdtempSync(path.join(os.tmpdir(), 'taa-veri-'));
  const env = { TAA_VERI_KOK: path.join(kok, 'veri'), APPDATA: path.join(kok, 'appdata'), XDG_DATA_HOME: path.join(kok, 'xdg'), HOME: kok };
  return { kok, env };
}

test('veriKok: ortam değişkeni öncelikli', () => {
  assert.strictEqual(v.veriKok({ TAA_VERI_KOK: '/x/y' }), '/x/y');
  assert.ok(v.veriKok({ APPDATA: '/a', XDG_DATA_HOME: '/d', HOME: '/h' }).endsWith(path.join('TAA', 'veri')));
});

test('rehber alanı istisnası: veri kökünün dışında, TAA/rehber', () => {
  const { env } = ortam();
  const y = v.alanYolu('rehber', env);
  assert.ok(y.endsWith(path.join('TAA', 'rehber')));
  assert.ok(!y.startsWith(env.TAA_VERI_KOK));
});

test('ilk açılış SAHIP.toml üretir ve yaz atomik çalışır', () => {
  const { env } = ortam();
  const s = v.acSahip({ alan: 'belgeler', program: 'taadon', surum: '1.12.0', env });
  const kayit = v.sahipTomlCoz(fs.readFileSync(path.join(s.dizin, 'SAHIP.toml'), 'utf8'));
  assert.strictEqual(kayit.sahip, 'taadon');
  assert.strictEqual(kayit.gecici, false);
  assert.strictEqual(kayit.sema, 1);
  s.yaz('alt/kayitlar.json', '{"a":1}');
  assert.strictEqual(fs.readFileSync(path.join(s.dizin, 'alt', 'kayitlar.json'), 'utf8'), '{"a":1}');
  assert.deepStrictEqual(fs.readdirSync(path.join(s.dizin, 'alt')), ['kayitlar.json']); // gecici dosya kalmadı
});

test('başka asıl sahip: BaskaSahip', () => {
  const { env } = ortam();
  v.acSahip({ alan: 'belgeler', program: 'taadon', surum: '1', env });
  assert.throws(() => v.acSahip({ alan: 'belgeler', program: 'tutaa', surum: '1', env }), (e) => e instanceof v.BaskaSahip && e.sahip === 'taadon');
});

test('geçici sahip devralma: eski SAHIP.toml eski/ altına taşınır', () => {
  const { env } = ortam();
  v.acSahip({ alan: 'rehber', program: 'tutaa', surum: '2.5.0', gecici: true, env });
  const s = v.acSahip({ alan: 'rehber', program: 'taarehber', surum: '0.1.0', env });
  assert.strictEqual(s.kayit.sahip, 'taarehber');
  const eski = fs.readdirSync(path.join(s.dizin, 'eski'));
  assert.strictEqual(eski.length, 1);
  assert.match(eski[0], /^SAHIP\.toml-\d{8}-\d{6}/);
  assert.match(fs.readFileSync(path.join(s.dizin, 'eski', eski[0]), 'utf8'), /sahip = "tutaa"/);
  // geçici sahip asıl sahibi devralamaz
  assert.throws(() => v.acSahip({ alan: 'rehber', program: 'tutaa', surum: '2.5.0', gecici: true, env }), v.BaskaSahip);
});

test('yol kaçışı: .., mutlak yol ve geçersiz alan adı reddedilir', () => {
  const { env } = ortam();
  const s = v.acSahip({ alan: 'belgeler', program: 'taadon', surum: '1', env });
  for (const kotu of ['../x', 'a/../../x', '/etc/passwd', 'C:\\x', '\\x', '']) {
    assert.throws(() => s.yaz(kotu, 'x'), (e) => e.kod === 'YolDisari', kotu);
  }
  assert.throws(() => v.acSahip({ alan: '../x', program: 'taadon', surum: '1', env }), (e) => e.kod === 'GecersizAd');
  assert.throws(() => v.acSahip({ alan: 'A'.repeat(3), program: 'taadon', surum: '1', env }), (e) => e.kod === 'GecersizAd');
  assert.throws(() => v.acSahip({ alan: 'a'.repeat(33), program: 'taadon', surum: '1', env }), (e) => e.kod === 'GecersizAd');
});

test('okuyan: oku, oner ve sahip; alan dosyası yazamaz', () => {
  const { env } = ortam();
  const s = v.acSahip({ alan: 'etiket-kuyrugu', program: 'fiyatetiketi', surum: '1.32.0', env });
  s.yaz('kuyruk.json', '[]');
  const o = v.acOkur({ alan: 'etiket-kuyrugu', program: 'taarkod', env });
  assert.strictEqual(o.oku('kuyruk.json'), '[]');
  assert.strictEqual(o.oku('yok.json'), null);
  assert.strictEqual(o.sahip().sahip, 'fiyatetiketi');
  assert.strictEqual(o.yaz, undefined);
  const id = o.oner({ ekle: [{ barkod: '123', adet: 2 }] });
  const dosya = path.join(s.dizin, 'oneriler', `${id}.json`);
  const j = JSON.parse(fs.readFileSync(dosya, 'utf8'));
  assert.strictEqual(j.sema, 1);
  assert.strictEqual(j.oneren, 'taarkod');
  assert.deepStrictEqual(j.degisiklik, { ekle: [{ barkod: '123', adet: 2 }] });
  assert.throws(() => o.oku('../SAHIP.toml'), (e) => e.kod === 'YolDisari');
});

test('öneri: kimlikler benzersiz, boyut 256 KiB üstü reddedilir, sahip kapatır', () => {
  const { env } = ortam();
  const s = v.acSahip({ alan: 'etiket-kuyrugu', program: 'fiyatetiketi', surum: '1', env });
  const o = v.acOkur({ alan: 'etiket-kuyrugu', program: 'taarkod', env });
  const a = o.oner({ n: 1 });
  const b = o.oner({ n: 2 });
  assert.notStrictEqual(a, b);
  assert.throws(() => o.oner({ buyuk: 'x'.repeat(300 * 1024) }), (e) => e.kod === 'BoyutAsildi');
  assert.deepStrictEqual(s.oneriler().map((x) => x.id), [a, b].sort());
  s.oneriKapat(a);
  assert.ok(fs.existsSync(path.join(s.dizin, 'oneriler', 'eski', `${a}.json`)));
  assert.strictEqual(s.oneriler().length, 1);
  assert.throws(() => s.oneriKapat('../x'), (e) => e.kod === 'GecersizAd');
});

test('SAHIP.toml yokken okuyucu SahipYok alır', () => {
  const { env } = ortam();
  assert.throws(() => v.acOkur({ alan: 'belgeler', program: 'tutaa', env }), (e) => e.kod === 'SahipYok');
});

test('sahipTomlCoz/Yaz: gidiş-dönüş, yorum, satır numaralı hata', () => {
  const nesne = { sema: 1, alan: 'x', gecici: false, not: 'a "b" \\ c' };
  assert.deepStrictEqual(v.sahipTomlCoz(v.sahipTomlYaz(nesne)), nesne);
  assert.deepStrictEqual(v.sahipTomlCoz('# yorum\n\na = 1 # son\nb = "m"\n'), { a: 1, b: 'm' });
  assert.throws(() => v.sahipTomlCoz('a = 1\nbozuk satir\n'), (e) => e.kod === 'GecersizToml' && e.satir === 2);
  assert.throws(() => v.sahipTomlCoz('[tablo]'), (e) => e.satir === 1);
  assert.throws(() => v.sahipTomlYaz({ a: 'iki\nsatir' }), (e) => e.kod === 'GecersizToml');
});

test('goc: kopyalar, .tasindi bırakır, tekrarlanmaz', () => {
  const { kok, env } = ortam();
  const eski = path.join(kok, 'userData', 'kuyruk.json');
  fs.mkdirSync(path.dirname(eski), { recursive: true });
  fs.writeFileSync(eski, '[1,2]');
  const r = v.goc({ eskiYol: eski, alan: 'etiket-kuyrugu', hedefAd: 'kuyruk.json', env });
  assert.strictEqual(r.durum, 'tasindi');
  assert.strictEqual(fs.readFileSync(r.hedef, 'utf8'), '[1,2]');
  assert.ok(!fs.existsSync(eski));
  assert.strictEqual(fs.readFileSync(`${eski}.tasindi`, 'utf8'), '[1,2]');
  // yeni bir eski dosya doğsa bile .tasindi varken tekrarlanmaz
  fs.writeFileSync(eski, '[9]');
  assert.strictEqual(v.goc({ eskiYol: eski, alan: 'etiket-kuyrugu', hedefAd: 'kuyruk.json', env }).durum, 'zaten');
  assert.strictEqual(fs.readFileSync(r.hedef, 'utf8'), '[1,2]');
  assert.strictEqual(v.goc({ eskiYol: path.join(kok, 'yok'), alan: 'belgeler', hedefAd: 'a.json', env }).durum, 'yok');
});

test('goc: alan yazılamazsa eski konum korunur', () => {
  const { kok, env } = ortam();
  const eski = path.join(kok, 'gecmis.json');
  fs.writeFileSync(eski, '{}');
  fs.writeFileSync(env.TAA_VERI_KOK, 'dosya'); // kök bir dosya: alan oluşturulamaz
  const r = v.goc({ eskiYol: eski, alan: 'belgeler', hedefAd: 'kayitlar.json', env });
  assert.strictEqual(r.durum, 'hata');
  assert.ok(fs.existsSync(eski));
});

test('kaynakta silme çağrısı yok', () => {
  const kaynak = fs.readFileSync(path.join(__dirname, '..', 'taa-veri.js'), 'utf8');
  assert.ok(!/\b(unlink|rmSync|rmdir|rm)\w*\s*\(/.test(kaynak.replace(/\/\/.*$/gm, '')));
  assert.ok(kaynak.split('\n').length <= 300);
});
