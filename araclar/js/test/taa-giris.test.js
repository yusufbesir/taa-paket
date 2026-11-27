// Sahip: Yusuf. yazan: [Claude#KM2-hesap-giris 2026-11-25]
'use strict';
const test = require('node:test');
const assert = require('node:assert');
const fs = require('fs');
const os = require('os');
const path = require('path');
const g = require('../taa-giris.js');

// Sahte taa-kimlik: argümanları ve stdin'i dosyaya yazar, SAHTE ortam değişkenine göre yanıt verir.
function ortam(senaryo) {
  const kok = fs.mkdtempSync(path.join(os.tmpdir(), 'taa-giris-'));
  const exe = path.join(kok, 'taa-kimlik');
  fs.writeFileSync(exe, `#!${process.execPath}
const fs = require('fs');
let girdi = ''; process.stdin.on('data', (b) => girdi += b).on('end', () => {
  fs.writeFileSync(${JSON.stringify(path.join(kok, 'cagri.json'))}, JSON.stringify({ args: process.argv.slice(2), girdi }));
  const s = process.env.SAHTE || 'tamam';
  const komut = process.argv[2];
  const yaz = (v) => process.stdout.write('yol: x\\n' + JSON.stringify(v) + '\\n');
  const hata = (kod, v) => { process.stderr.write('yol: x\\n' + JSON.stringify(v) + '\\n'); process.exit(kod); };
  if (s === 'kilitli') hata(4, { hata: 'hesabınız kilitlendi.', kod: 4, http: 423 });
  if (s === 'hatali') hata(4, { hata: 'giriş reddedildi (401): {"hata":"giris_hatali","ileti":"Kullanıcı adı ya da parola hatalı"}', kod: 4, http: 401 });
  if (s === 'sinir') hata(4, { hata: 'çok fazla deneme', kod: 4, http: 429 });
  if (s === 'onay') hata(4, { hata: 'hesabınız yönetici onayı bekliyor.', kod: 4, http: 403 });
  if (s === 'sube') hata(4, { hata: 'giriş reddedildi (403): {"hata":"sube_yetkisiz","ileti":"x"}', kod: 4, http: 403 });
  if (s === 'baglanti') hata(3, { hata: 'ulaşılamadı', kod: 3 });
  if (s === 'cihaz') hata(5, { hata: 'bu bilgisayar sunucuya kayıtlı değil (cihaz anahtarı yok).', kod: 5 });
  if (s === 'parola') hata(4, { hata: 'giriş reddedildi (400): {"hata":"parola","ileti":"Şifre en az 8 karakter olmalı"}', kod: 4, http: 400 });
  if (komut === 'giris') yaz({ kullanici: 'ali', sube_id: 2, roller: ['kasa'], izinler: ['tutaa.kullan'], programlar: s === 'kapali' ? ['taarkod'] : s === 'bos' ? [] : ['tutaa'], uyari_sayisi: 2 });
  else if (komut === 'basvur') yaz({ durum: 'iletildi', dogrulama_sayisi: '07' });
  else if (komut === 'durum') yaz({ kullanici: 'ali', sube_id: 2, roller: ['kasa'], programlar: s === 'kapali' ? ['taarkod'] : s === 'bos' ? [] : ['tutaa'] });
  else if (komut === 'yetki') { if (s === 'yetkisiz') { yaz({ sonuc: 'yetkisiz', neden: ['izin yok'] }); process.exit(7); } yaz({ sonuc: 'izinli' }); }
  else if (komut === 'cikis') yaz({ durum: 'cikis' });
  else if (komut === 'personel') {
    const alt = process.argv[3];
    if (s === 'p_yetkisiz') hata(7, { hata: '{"hata":"yetki_yok","ileti":"Bu işlem için yetkiniz yok"}', kod: 7, http: 403 });
    if (s === 'p_baglanti') hata(3, { hata: 'ulaşılamadı', kod: 3 });
    if (alt === 'liste') yaz({ sube_id: 1, kisiler: [{ id: 1, ad_soyad: 'Ali Kaya', gorev: 'Kasiyer', sube_id: 1, gecici: false, bitis: null }, { id: 9, ad_soyad: 'Zeynep Ak', gorev: '', sube_id: 2, gecici: true, bitis: '2026-12-01T00:00:00Z', fazla: 'x' }, { bozuk: 1 }] });
    else if (alt === 'ara') yaz({ kisiler: [{ id: 9, ad_soyad: 'Zeynep Ak', gorev: '', sube_id: 2, sube_ad: 'Kartal' }] });
    else if (alt === 'gecici') yaz({ id: 4, bitis: '2026-11-30T10:00:00Z' });
  }
});
`);
  fs.chmodSync(exe, 0o755);
  const env = { ...process.env, TAA_KIMLIK: exe, SAHTE: senaryo || 'tamam' };
  const cagri = () => JSON.parse(fs.readFileSync(path.join(kok, 'cagri.json'), 'utf8'));
  return { env, cagri, kok };
}

const posix = process.platform !== 'win32';

test('kimlikYolu: TAA_KIMLIK, kayit.json seçili sürüm, yoksa null', () => {
  const kok = fs.mkdtempSync(path.join(os.tmpdir(), 'taa-kok-'));
  assert.strictEqual(g.kimlikYolu({ TAA_KOK: kok }), null);
  const exe = process.platform === 'win32' ? 'taa-kimlik.exe' : 'taa-kimlik';
  const bin = path.join(kok, 'depo', 'taa-kimlik', '0.3.0', 'bin');
  fs.mkdirSync(bin, { recursive: true });
  fs.writeFileSync(path.join(bin, exe), '');
  fs.writeFileSync(path.join(kok, 'kayit.json'), JSON.stringify({ paketler: { 'taa-kimlik': { secili: '0.3.0' } } }));
  assert.strictEqual(g.kimlikYolu({ TAA_KOK: kok }), path.join(bin, exe));
  fs.writeFileSync(path.join(kok, 'kayit.json'), JSON.stringify({ paketler: { 'taa-kimlik': { secili: '../../x' } } }));
  assert.strictEqual(g.kimlikYolu({ TAA_KOK: kok }), null, 'yol kaçışı kabul edilmez');
  for (const kotu of ['..', '.', '0/../..', '0..', 'a1']) {
    fs.writeFileSync(path.join(kok, 'kayit.json'), JSON.stringify({ paketler: { 'taa-kimlik': { secili: kotu } } }));
    assert.strictEqual(g.kimlikYolu({ TAA_KOK: kok }), null, kotu);
  }
  assert.strictEqual(g.kimlikYolu({ TAA_KIMLIK: path.join(kok, 'yok') }), null);
  assert.strictEqual(g.kimlikYolu({ TAA_KIMLIK: path.join(bin, exe), TAA_KOK: kok }, true) === path.join(bin, exe), false, 'paketli: ortam yok sayılır');
});

test('araç yoksa kişiye anlaşılır ileti', async () => {
  const r = await g.giris({ kullanici: 'ali', parola: 'x' }, { env: { TAA_KIMLIK: '/yok/taa-kimlik' } });
  assert.deepStrictEqual([r.tamam, r.durum], [false, 'arac_yok']);
});

test('giriş: şifre yalnız stdin ile gider, oturum ve uyarı sayısı döner', { skip: !posix }, async () => {
  const o = ortam();
  const r = await g.giris({ kullanici: ' ali ', parola: 'Gizli-Sifre-1' }, { env: o.env, program: 'tutaa' });
  assert.strictEqual(r.tamam, true);
  assert.deepStrictEqual(r.oturum, { kullanici: 'ali', sube_id: 2, roller: ['kasa'], izinler: ['tutaa.kullan'], programlar: ['tutaa'], uyari_sayisi: 2 });
  const c = o.cagri();
  assert.deepStrictEqual(c.args, ['giris', '--kullanici', 'ali', '--json']);
  assert.strictEqual(c.girdi, 'Gizli-Sifre-1\n');
  assert.ok(!JSON.stringify(r).includes('Gizli'), 'şifre sonuçta yok');
});

test('giriş: program kapalıysa ya da liste boşsa reddedilir, ortak oturum kapatılmaz', { skip: !posix }, async () => {
  for (const s of ['kapali', 'bos']) {
    const o = ortam(s);
    const r = await g.giris({ kullanici: 'ali', parola: 'p' }, { env: o.env, program: 'tutaa' });
    assert.deepStrictEqual([r.tamam, r.durum], [false, 'program_kapali'], s);
    assert.strictEqual(o.cagri().args[0], 'giris', 'cikis çağrılmadı');
  }
  const t = await g.giris({ kullanici: 'ali', parola: 'p' }, { env: ortam('bos').env });
  assert.strictEqual(t.tamam, true, 'program verilmezse denetim yok');
});

test('durum ve baslangic program denetimi yapar', { skip: !posix }, async () => {
  assert.strictEqual((await g.durum({ env: ortam().env, program: 'tutaa' })).kullanici, 'ali');
  assert.strictEqual(await g.durum({ env: ortam('kapali').env, program: 'tutaa' }), null);
  assert.strictEqual(await g.durum({ env: ortam('bos').env, program: 'tutaa' }), null);
  assert.deepStrictEqual(await g.baslangic({ env: { TAA_KIMLIK: '/yok' } }), { arac: false, oturum: null });
  const b = await g.baslangic({ env: ortam().env, program: 'tutaa' });
  assert.deepStrictEqual([b.arac, b.oturum.kullanici], [true, 'ali']);
});

test('giriş ret durumları HTTP koduna göre ayrılır', { skip: !posix }, async () => {
  for (const [s, d] of [['kilitli', 'kilitli'], ['hatali', 'hatali'], ['sinir', 'sinir'], ['onay', 'onay_bekliyor'], ['sube', 'sube_yetkisiz'], ['baglanti', 'baglanti']]) {
    const r = await g.giris({ kullanici: 'ali', parola: 'p' }, { env: ortam(s).env });
    assert.deepStrictEqual([r.tamam, r.durum], [false, d], s);
    assert.strictEqual(r.ileti, g.ILETI[d]);
  }
});

test('giriş: bozuk girdi araca gitmez', async () => {
  for (const i of [{ kullanici: '', parola: 'p' }, { kullanici: 'a b', parola: 'p' }, { kullanici: 'ali', parola: '' }, { kullanici: 'ali', parola: 'a\nb' }]) {
    const r = await g.giris(i, { env: { TAA_KIMLIK: '/yok' } });
    assert.strictEqual(r.durum, 'girdi');
  }
});

test('başvuru: yeni şifre stdin ile, doğrulama sayısı döner; kilit başvurusunda boş satır', { skip: !posix }, async () => {
  const o = ortam();
  const r = await g.basvur({ kullanici: 'ali', yeniParola: 'Yeni-Sifre-2026' }, { env: o.env });
  assert.deepStrictEqual([r.tamam, r.dogrulama_sayisi, r.tur], [true, '07', 'sifre']);
  assert.deepStrictEqual(o.cagri().args, ['basvur', '--kullanici', 'ali', '--json']);
  assert.strictEqual(o.cagri().girdi, 'Yeni-Sifre-2026\n');
  const k = await g.basvur({ kullanici: 'ali' }, { env: o.env });
  assert.deepStrictEqual([k.tamam, k.tur], [true, 'kilit']);
  assert.strictEqual(o.cagri().girdi, '\n');
});

test('başvuru: şifre kuralı iletisi aynen, cihaz kayıtsız ayrı', { skip: !posix }, async () => {
  const p = await g.basvur({ kullanici: 'ali', yeniParola: 'kisa' }, { env: ortam('parola').env });
  assert.deepStrictEqual([p.durum, p.ileti], ['parola', 'Şifre en az 8 karakter olmalı']);
  const c = await g.basvur({ kullanici: 'ali' }, { env: ortam('cihaz').env });
  assert.strictEqual(c.durum, 'cihaz');
});

test('durum, yetki, çıkış', { skip: !posix }, async () => {
  const o = ortam();
  assert.deepStrictEqual((await g.durum({ env: o.env })).kullanici, 'ali');
  assert.strictEqual((await g.yetki({ izin: 'fis.sil' }, { env: o.env })).izinli, true);
  assert.deepStrictEqual(o.cagri().args, ['yetki', '--izin', 'fis.sil', '--json']);
  await g.yetki({ program: 'baska', izin: 'fis.sil' }, { env: o.env, program: 'tutaa' });
  assert.deepStrictEqual(o.cagri().args, ['yetki', '--program', 'tutaa', '--izin', 'fis.sil', '--json'], 'program ana süreçte sabit');
  assert.strictEqual((await g.yetki({ izin: 'fis.sil' }, { env: ortam('yetkisiz').env })).izinli, false);
  assert.strictEqual((await g.yetki({ izin: 'bozuk izin' }, { env: o.env })).izinli, false, 'bozuk izin adı araca gitmez');
  assert.strictEqual((await g.cikis({ env: o.env })).tamam, true);
  assert.strictEqual(await g.durum({ env: ortam('baglanti').env }), null);
});

test('ipcKur kanalları, gönderen denetimi ve program adı', async () => {
  const kanallar = {};
  g.ipcKur({ handle: (k, f) => { kanallar[k] = f; } }, { program: 'tutaa', env: { TAA_KIMLIK: '/yok' } });
  assert.deepStrictEqual(Object.keys(kanallar).sort(), Object.values(g.KANAL).sort());
  const ana = { url: 'file:///app/index.html' };
  const iyi = { sender: { mainFrame: ana }, senderFrame: ana };
  assert.deepStrictEqual(await kanallar[g.KANAL.baslangic](iyi), { arac: false, oturum: null });
  const alt = { sender: { mainFrame: ana }, senderFrame: { url: 'file:///app/index.html' } };
  assert.throws(() => kanallar[g.KANAL.giris](alt, {}), /yetkisiz/);
  const uzak = { sender: { mainFrame: { url: 'https://x' } }, senderFrame: null };
  uzak.senderFrame = uzak.sender.mainFrame;
  assert.throws(() => kanallar[g.KANAL.durum](uzak), /yetkisiz/);
  const ozel = {};
  g.ipcKur({ handle: (k, f) => { ozel[k] = f; } }, { denetle: () => { throw new Error('hayır'); } });
  assert.throws(() => ozel[g.KANAL.cikis]({}), /hayır/);
  assert.throws(() => g.ipcKur({ handle: () => {} }, { program: 'Tutaa!' }));
  let acik;
  g.preloadKur({ exposeInMainWorld: (ad, api) => { acik = [ad, Object.keys(api).sort()]; } }, { invoke: () => {} });
  assert.deepStrictEqual(acik, ['taaGiris', ['baslangic', 'basvur', 'cikis', 'durum', 'giris', 'kilitle', 'personel', 'yetki']]);
});

test('ekran kilidi: oturum kapanmaz, işaret yeniden açılışta kilitli başlatır; giriş ve çıkış kaldırır (Yusuf 11-26)', { skip: !posix }, async () => {
  const o = ortam();
  const kilitDosyasi = path.join(o.kok, 'veri', 'taa-giris-kilit.json');
  const s = { env: o.env, program: 'tutaa', kilitDosyasi };
  const k = await g.kilitle(s);
  assert.deepStrictEqual(k, { tamam: true, kullanici: 'ali' });
  assert.strictEqual(o.cagri().args[0], 'durum', 'kilit cikis çağırmaz; kullanıcı süren oturumdan okunur');
  assert.strictEqual(JSON.parse(fs.readFileSync(kilitDosyasi, 'utf8')).kullanici, 'ali');
  assert.deepStrictEqual(await g.baslangic(s), { arac: true, oturum: null, kilitli: { kullanici: 'ali' } }, 'yeniden açılış kilitli');
  const r = await g.giris({ kullanici: 'ali', parola: 'p' }, s);
  assert.strictEqual(r.tamam, true);
  assert.strictEqual(fs.readFileSync(kilitDosyasi, 'utf8'), '{}', 'başarılı giriş kilidi boşaltır (dosya silinmez)');
  assert.strictEqual((await g.baslangic(s)).oturum.kullanici, 'ali');
  await g.kilitle(s);
  await g.giris({ kullanici: 'ali', parola: 'p' }, { ...s, env: ortam('hatali').env });
  assert.strictEqual(JSON.parse(fs.readFileSync(kilitDosyasi, 'utf8')).kullanici, 'ali', 'hatalı şifre kilidi kaldırmaz');
  await g.cikis(s);
  assert.strictEqual(fs.readFileSync(kilitDosyasi, 'utf8'), '{}', 'çıkış kilidi boşaltır');
  // Oturum yoksa kilitlenmez; başka kullanıcıya ait eski işaret geçersizdir ve silinir.
  assert.deepStrictEqual(await g.kilitle({ ...s, env: ortam('cihaz').env }), { tamam: false });
  // Ctrl+L anında oturum geçici okunamazsa kilit yine yazılır: bu süreçte doğrulanmış son kullanıcıyla (kapalı yönde).
  await g.baslangic(s);
  fs.writeFileSync(kilitDosyasi, '{}');
  assert.deepStrictEqual(await g.kilitle({ ...s, env: ortam('cihaz').env }), { tamam: true, kullanici: 'ali' });
  assert.strictEqual(JSON.parse(fs.readFileSync(kilitDosyasi, 'utf8')).kullanici, 'ali', 'durum okunamasa da kilit yazılır');
  await g.cikis(s);
  assert.deepStrictEqual(await g.kilitle({ ...s, env: ortam('cihaz').env }), { tamam: false }, 'çıkıştan sonra eski kullanıcıyla kilitlenmez');
  // Çıkıştan sonra giriş yapan kullanıcı, araya durum girmeden Ctrl+L'de durum okunamasa da kilitlenir.
  assert.strictEqual((await g.giris({ kullanici: 'ali', parola: 'p' }, s)).tamam, true);
  assert.deepStrictEqual(await g.kilitle({ ...s, env: ortam('cihaz').env }), { tamam: true, kullanici: 'ali' }, 'girişten hemen sonra kilit yazılır');
  await g.cikis(s);
  // Açılışta oturum okunamazsa (geçici hata) kilit düşmez: işaret korunur, ekran kilitli açılır (kapalı yönde).
  await g.kilitle(s);
  assert.deepStrictEqual(await g.baslangic({ ...s, env: ortam('cihaz').env }), { arac: true, oturum: null, kilitli: { kullanici: 'ali' } });
  assert.strictEqual(JSON.parse(fs.readFileSync(kilitDosyasi, 'utf8')).kullanici, 'ali', 'oturum okunamayınca işaret korunur');
  await g.cikis(s);
  fs.mkdirSync(path.dirname(kilitDosyasi), { recursive: true });
  fs.writeFileSync(kilitDosyasi, JSON.stringify({ kullanici: 'veli' }));
  assert.strictEqual((await g.baslangic(s)).oturum.kullanici, 'ali');
  assert.strictEqual(fs.readFileSync(kilitDosyasi, 'utf8'), '{}', 'eski işaret boşaltılır');
  fs.writeFileSync(kilitDosyasi, JSON.stringify({ kullanici: 'bozuk ad' }));
  assert.strictEqual((await g.baslangic(s)).oturum.kullanici, 'ali', 'bozuk işaret yok sayılır');
});

test('personel: şube listesi, önbellek (yalnız ağ sorununda), arama, süreli ekleme şifresi stdin ile (Yusuf 11-27)', { skip: !posix }, async () => {
  const o = ortam('tamam');
  const s = { env: o.env, veriKlasoru: o.kok };
  const l = await g.personel({ tur: 'liste' }, s);
  assert.strictEqual(l.tamam, true);
  assert.strictEqual(l.kaynak, 'sunucu');
  assert.deepStrictEqual(l.kisiler.map((k) => [k.ad_soyad, k.gecici]), [['Ali Kaya', false], ['Zeynep Ak', true]], 'bozuk satır atılır');
  assert.strictEqual(l.kisiler[1].fazla, undefined, 'bilinmeyen alan geçmez');
  // ağ yoksa son liste
  const b = await g.personel({ tur: 'liste' }, { env: { ...o.env, SAHTE: 'p_baglanti' }, veriKlasoru: o.kok });
  assert.strictEqual(b.kaynak, 'onbellek');
  assert.strictEqual(b.kisiler.length, 2);
  // yetki yoksa eski liste verilmez
  const y = await g.personel({ tur: 'liste' }, { env: { ...o.env, SAHTE: 'p_yetkisiz' }, veriKlasoru: o.kok });
  assert.deepStrictEqual([y.tamam, y.durum], [false, 'yetki_yok']);
  // önbellek hiç yoksa anlaşılır ileti
  const bos = fs.mkdtempSync(path.join(os.tmpdir(), 'taa-personel-'));
  const n = await g.personel({ tur: 'liste' }, { env: { ...o.env, SAHTE: 'p_baglanti' }, veriKlasoru: bos });
  assert.deepStrictEqual([n.tamam, n.ileti], [false, g.ILETI.personel_yok]);
  // arama
  const a = await g.personel({ tur: 'ara', metin: 'zey' }, s);
  assert.strictEqual(a.kisiler[0].sube_ad, 'Kartal');
  assert.deepStrictEqual(o.cagri().args.slice(0, 4), ['personel', 'ara', '--ara', 'zey']);
  assert.strictEqual((await g.personel({ tur: 'ara', metin: 'z' }, s)).durum, 'girdi');
  // süreli ekleme: şifre yalnız stdin
  const e = await g.personel({ tur: 'gecici', kisi: 9, gun: 3, neden: 'izin  yerine', parola: 'gizli-sifre-1' }, s);
  assert.deepStrictEqual(e, { tamam: true, bitis: '2026-11-30T10:00:00Z' });
  const c = o.cagri();
  assert.deepStrictEqual(c.args.slice(0, 8), ['personel', 'gecici', '--kisi', '9', '--gun', '3', '--neden', 'izin yerine']);
  assert.ok(!c.args.join(' ').includes('gizli-sifre-1'));
  assert.strictEqual(c.girdi, 'gizli-sifre-1\n');
  assert.strictEqual((await g.personel({ tur: 'gecici', kisi: 9, gun: 32, parola: 'x' }, s)).durum, 'girdi');
  assert.strictEqual((await g.personel({ tur: 'gecici', kisi: 9, gun: 3, parola: '' }, s)).durum, 'girdi');
});

test('ekran kilitliyken IPC kapısı: durum/yetki/personel verilmez, giriş yalnız kilitleyen kullanıcıyla (Fable K2)', { skip: !posix }, async () => {
  const o = ortam();
  const kilitDosyasi = path.join(o.kok, 'veri', 'taa-giris-kilit.json');
  const k = {};
  g.ipcKur({ handle: (ad, f) => { k[ad] = f; } }, { env: o.env, program: 'tutaa', kilitDosyasi, denetle: () => {} });
  assert.strictEqual(g.kilitliMi({ kilitDosyasi }), null);
  assert.strictEqual((await k[g.KANAL.durum]({})).kullanici, 'ali', 'kilit yokken durum verilir');
  assert.strictEqual((await k[g.KANAL.kilitle]({})).tamam, true);
  assert.deepStrictEqual(g.kilitliMi({ kilitDosyasi }), { kullanici: 'ali' });
  assert.strictEqual(await k[g.KANAL.durum]({}), null, 'kilitliyken durum null');
  assert.deepStrictEqual(await k[g.KANAL.yetki]({}, { izin: 'tutaa.yaz' }), { izinli: false, neden: ['ekran_kilitli'] });
  assert.strictEqual((await k[g.KANAL.personel]({}, {})).durum, 'ekran_kilitli');
  assert.strictEqual((await k[g.KANAL.basvur]({}, { kullanici: 'ali', yeniParola: 'x' })).durum, 'ekran_kilitli');
  assert.strictEqual((await k[g.KANAL.giris]({}, { kullanici: 'veli', parola: 'p' })).durum, 'ekran_kilitli', 'başka kullanıcı reddedilir');
  assert.deepStrictEqual(g.kilitliMi({ kilitDosyasi }), { kullanici: 'ali' }, 'reddedilen giriş işareti kaldırmaz');
  assert.strictEqual((await k[g.KANAL.giris]({}, { kullanici: 'ali', parola: 'p' })).tamam, true, 'kilitleyen açar');
  assert.strictEqual(g.kilitliMi({ kilitDosyasi }), null);
  // Başka kullanıcıyla gir: önce cikis, sonra herkes girebilir.
  await k[g.KANAL.kilitle]({});
  await k[g.KANAL.cikis]({});
  assert.strictEqual(g.kilitliMi({ kilitDosyasi }), null);
  assert.notStrictEqual((await k[g.KANAL.giris]({}, { kullanici: 'veli', parola: 'p' })).durum, 'ekran_kilitli');
});
