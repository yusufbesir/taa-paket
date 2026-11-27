// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W3-tutanak 2026-11-27]
// Tutaa (tutanak) testleri: şablon okuma (çift doğrulama YOK), tutanak doğrulama ve özet, fotoğrafın giden
// kutusuna `ocr_bekliyor` olarak yazılması (OCR yok), liste birleştirme, imza kuralları ve ekran akışı.
// Sahte DOM ile çalışır; gerçek tarayıcı (kamera, tuval, dokunmatik imza, fotoğraf küçültme) DENENMEZ.
import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';
import * as T from '../mantik.js';
import { ac, ad, baslik } from '../index.js';
import { sahteBelge, bekle } from '../../ortak/sinama/sahte-dom.js';
import { sahteBaglam } from '../../ortak/sinama/sahte-baglam.js';
import { SABLONLAR, LISTE, sahteFoto, sahteDosya, ornekCizim } from '../sinama/ornek.js';

const TUM = ['tutanak.oku', 'tutanak.olustur', 'tutanak.imza', 'tutanak.foto'];
const sablonlar = () => T.sablonlariDuzelt(SABLONLAR());
const olay = () => sablonlar().find((s) => s.kimlik === 'olay-tutanagi');
const DOLU = { tarih: '2026-11-27', saat: '14:30', yer: 'Depo', tur: 'Hasar', onlemler: ['Kamera kaydı ayrıldı'], aciklama: 'Raf devrildi.', zarar: '1.250,50', acil: true };

async function kur(secenek = {}) {
  const { kok, pencere } = sahteBelge();
  const s = sahteBaglam({ izinler: TUM, onbellek: { 'tutanak/sablonlar': SABLONLAR(), 'tutanak/liste': LISTE() }, ...secenek });
  await ac(kok, s.baglam);
  return { kok, pencere, ...s };
}
const durum = (kok) => kok.querySelector('[data-bolge="durum"]');
const alan = (kok, adi) => kok.querySelector(`[data-alan="${adi}"]`);
const sekme = (kok, kod) => kok.querySelector(`button[data-sekme="${kod}"]`).click();
const is = (kok, adi) => kok.querySelector(`button[data-is="${adi}"]`);
const kartlar = (kok) => kok.querySelectorAll('li[data-tutanak]').map((d) => d.getAttribute('data-tutanak'));

// ---------------------------------------------------------------- saf kurallar

test('şablonlar: bozuklar ve Kasa Çift Doğrulama atılır; bilinmeyen alan türü metin olur', () => {
  const l = sablonlar();
  assert.deepEqual(l.map((s) => s.kimlik), ['olay-tutanagi', 'sayim-fark']);
  assert.equal(l.some((s) => T.KASADA_OLAN.includes(s.kimlik)), false);
  const o = l[0];
  assert.equal(o.surum, 3);
  assert.equal(o.bolumler[0].alanlar.find((a) => a.id === 'bilinmeyen').tur, 'metin');
  assert.deepEqual(T.sablonRolleri(o), ['Düzenleyen', 'Tanık', 'İlgili Personel']);
  assert.deepEqual(T.sablonRolleri(null), ['Düzenleyen', 'Tanık', 'İlgili']);
  assert.equal(l[1].tablo.enAzSatir, 2);
  assert.deepEqual(T.sablonlariDuzelt(undefined), []);
  assert.deepEqual(T.sablonlariDuzelt({ sablonlar: 'x' }), []);
});

test('tutanakHazirla: zorunlu alan, tarih, saat, seçim, sayı denetimi; gövde tutanak.olustur@1', () => {
  const s = olay();
  const h = T.tutanakHazirla(s, { alanlar: DOLU }, Date.UTC(2026, 10, 27, 11, 0, 0));
  assert.deepEqual(h, {
    hatalar: [],
    govde: {
      sozlesme: 'tutanak.olustur@1',
      sablon: 'olay-tutanagi',
      surum: 3,
      alanlar: { tarih: '2026-11-27', saat: '14:30', yer: 'Depo', tur: 'Hasar', onlemler: ['Kamera kaydı ayrıldı'], aciklama: 'Raf devrildi.', zarar: 1250.5, acil: true },
      satirlar: [],
      olusturma: '2026-11-27T11:00:00.000Z',
    },
  });
  assert.equal('no' in h.govde, false, 'belge numarasını istemci üretmez');

  const kotu = T.tutanakHazirla(s, { alanlar: { tarih: '27.11.2026', saat: '25:00', yer: ' ', tur: 'Uydurma', zarar: 'çok', aciklama: 'x'.repeat(5001) } });
  assert.equal(kotu.govde, undefined);
  for (const desen of [/Olay Tarihi: tarih YYYY-AA-GG/, /Olay Saati: saat SS:DD/, /Yer: boş bırakılamaz/, /Olay Türü: listedeki/, /Tahmini Zarar \(TL\): sayı olmalı/, /Açıklama: en çok 5000/]) {
    assert.ok(kotu.hatalar.some((x) => desen.test(x)), String(desen));
  }
  assert.deepEqual(T.tutanakHazirla(null, {}).hatalar, ['şablon seçilmedi']);
  assert.match(T.tutanakHazirla({ kimlik: 'kasa-cift-dogrulama', surum: 2, bolumler: [], tablo: null, imzalar: [] }, {}).hatalar[0], /TAA Kasa/);
  // İsteğe bağlı boş alanlar gövdeye girmez.
  const az = T.tutanakHazirla(s, { alanlar: { tarih: '2026-11-27', yer: 'Depo', tur: 'Tespit', aciklama: 'Not' } });
  assert.deepEqual(Object.keys(az.govde.alanlar), ['tarih', 'yer', 'tur', 'aciklama']);
});

test('tutanakHazirla: tablo satırları; boş satır atılır, yarım satır hata verir', () => {
  const s = sablonlar().find((x) => x.kimlik === 'sayim-fark');
  const h = T.tutanakHazirla(s, { alanlar: { reyon: 'Erkek' }, satirlar: [{ urun: 'Gömlek', kayit: '10', sayilan: '8' }, { urun: '', kayit: '', sayilan: '' }, { urun: 'Kravat', kayit: '3', sayilan: '3' }] });
  assert.deepEqual(h.govde.satirlar, [{ urun: 'Gömlek', kayit: 10, sayilan: 8 }, { urun: 'Kravat', kayit: 3, sayilan: 3 }]);
  const yarim = T.tutanakHazirla(s, { alanlar: { reyon: 'Erkek' }, satirlar: [{ urun: '', kayit: '5', sayilan: 'x' }] });
  assert.ok(yarim.hatalar.some((x) => /Satır 1, Ürün: boş bırakılamaz/.test(x)));
  assert.ok(yarim.hatalar.some((x) => /Satır 1, Sayılan: sayı olmalı/.test(x)));
  assert.match(T.tutanakHazirla(s, { alanlar: { reyon: 'E' }, satirlar: Array.from({ length: 61 }, () => ({ urun: 'a' })) }).hatalar[0], /en çok 60 satır/);
});

test('özet: kanonik JSON anahtar sırasından bağımsız; sha256 Node ile aynı', async () => {
  assert.equal(T.kanonik({ b: 1, a: [2, { d: null, c: 'ç' }] }), '{"a":[2,{"c":"ç","d":null}],"b":1}');
  assert.equal(T.kanonik({ a: 1, b: 2 }), T.kanonik({ b: 2, a: 1 }));
  const h = T.tutanakHazirla(olay(), { alanlar: DOLU });
  const g = await T.ozetEkle(h.govde);
  const beklenen = createHash('sha256').update(T.kanonik({ sablon: g.sablon, surum: g.surum, alanlar: g.alanlar, satirlar: g.satirlar })).digest('hex');
  assert.equal(g.sha256, beklenen);
  assert.match(g.sha256, /^[0-9a-f]{64}$/);
  const baska = await T.ozetEkle(T.tutanakHazirla(olay(), { alanlar: { ...DOLU, yer: 'Mağaza' } }).govde);
  assert.notEqual(baska.sha256, g.sha256);
});

test('fotoHazirla: tür baytlardan anlaşılır; gövde ocr_bekliyor, OCR sonucu alanı yok', async () => {
  assert.equal(T.fotoTuru(sahteFoto('jpeg')), 'image/jpeg');
  assert.equal(T.fotoTuru(sahteFoto('png')), 'image/png');
  assert.equal(T.fotoTuru(sahteFoto('webp')), 'image/webp');
  assert.equal(T.fotoTuru(sahteFoto('pdf')), null);
  assert.equal(T.fotoTuru(new Uint8Array(3)), null);

  const bayt = sahteFoto('jpeg', 300);
  const h = await T.fotoHazirla({ ad: 'Tutanak Fotoğrafı (1).HEIC', bayt }, { grup: 'f-abc', sayfa: 1, sayfaSayisi: 2, sablon: 'olay-tutanagi', not: ' depo ' }, Date.UTC(2026, 10, 27));
  assert.deepEqual(h.hatalar, []);
  assert.equal(h.govde.sozlesme, 'tutanak.foto@1');
  assert.equal(h.govde.durum, 'ocr_bekliyor');
  assert.deepEqual([h.govde.grup, h.govde.sayfa, h.govde.sayfa_sayisi, h.govde.sablon, h.govde.not], ['f-abc', 1, 2, 'olay-tutanagi', 'depo']);
  assert.equal(h.govde.dosya.ad, 'Tutanak_Foto_raf_1_.jpg', 'ad sadeleşir, uzantı gerçek türden gelir');
  assert.equal(h.govde.dosya.tur, 'image/jpeg');
  assert.equal(h.govde.dosya.boyut, 300);
  assert.equal(h.govde.dosya.sha256, createHash('sha256').update(bayt).digest('hex'));
  assert.deepEqual(Buffer.from(h.govde.dosya.veri_b64, 'base64'), Buffer.from(bayt));
  for (const yasak of ['metin', 'ocr', 'sonuc', 'alanlar']) assert.equal(yasak in h.govde, false, `OCR istemcide yapılmaz: ${yasak}`);

  const kotu = [
    [{ bayt: sahteFoto('pdf') }, { grup: 'f-a', sayfa: 1, sayfaSayisi: 1 }, /yalnız JPEG, PNG ya da WebP/],
    [{ bayt: sahteFoto('jpeg', T.FOTO_EN_COK + 1) }, { grup: 'f-a', sayfa: 1, sayfaSayisi: 1 }, /çok büyük/],
    [{ bayt: sahteFoto('jpeg') }, { grup: 'f-a', sayfa: 4, sayfaSayisi: 4 }, /en çok 3 fotoğraf/],
    [{ bayt: sahteFoto('jpeg') }, { grup: 'boşluk var', sayfa: 1, sayfaSayisi: 1 }, /grubu kimliği yok/],
    [{ bayt: sahteFoto('jpeg') }, { grup: 'f-a', sayfa: 1, sayfaSayisi: 1, sablon: 'kasa-cift-dogrulama' }, /belge türü geçersiz/],
    [{ bayt: new Uint8Array(0) }, { grup: 'f-a', sayfa: 1, sayfaSayisi: 1 }, /dosya okunamadı/],
    [{ bayt: sahteFoto('jpeg') }, { grup: 'f-a', sayfa: 1, sayfaSayisi: 1, not: 'x'.repeat(301) }, /not en çok 300/],
  ];
  for (const [dosya, bilgi, desen] of kotu) {
    const s = await T.fotoHazirla(dosya, bilgi);
    assert.equal(s.govde, undefined);
    assert.ok(s.hatalar.some((x) => desen.test(x)), String(desen));
  }
  // En büyük fotoğraf zarfıyla birlikte çekirdeğin 1 MiB gövde sınırının altında kalır.
  const buyuk = await T.fotoHazirla({ ad: 'a.jpg', bayt: sahteFoto('jpeg', T.FOTO_EN_COK) }, { grup: 'f-a', sayfa: 1, sayfaSayisi: 1, not: 'x'.repeat(300) });
  assert.ok(new TextEncoder().encode(JSON.stringify({ ...buyuk.govde, no: 'x'.repeat(64) })).length < 1024 * 1024);
  assert.deepEqual(T.kucultmeOlcusu(4000, 3000), { en: 1600, boy: 1200 });
  assert.deepEqual(T.kucultmeOlcusu(800, 600), { en: 800, boy: 600 }, 'büyütme yapılmaz');
  assert.equal(T.kucultmeOlcusu(0, 10), null);
});

test('tutanakListesi: sunucu özeti + giden kutusu; çift doğrulama görünmez; fotoğraflar grupla tek satır', () => {
  const kayitlar = [
    { no: 'N-1', program: 'tutanak', uc: T.UC.olustur, durum: 'bekliyor', govde: { sablon: 'olay-tutanagi', olusturma: '2026-11-27T10:00:00.000Z', sha256: 'b'.repeat(64) } },
    { no: 'N-2', program: 'tutanak', uc: T.UC.olustur, durum: 'reddedildi', ret: { kod: 422, mesaj: 'alan eksik' }, govde: { sablon: 'sayim-fark', olusturma: '2026-11-26T10:00:00.000Z' } },
    { no: 'N-3', program: 'tutanak', uc: T.UC.olustur, durum: 'gonderildi', govde: { sablon: 'sayim-fark', olusturma: '2026-11-25T10:00:00.000Z' } },
    { no: 'N-4', program: 'tutanak', uc: T.UC.foto, durum: 'bekliyor', govde: { grup: 'f-1', sayfa: 1, sayfa_sayisi: 2, cekim: '2026-11-27T09:00:00.000Z' } },
    { no: 'N-5', program: 'tutanak', uc: T.UC.foto, durum: 'gonderildi', govde: { grup: 'f-1', sayfa: 2, sayfa_sayisi: 2, cekim: '2026-11-27T09:00:00.000Z' } },
    { no: 'N-6', program: 'tutanak', uc: T.UC.foto, durum: 'gonderildi', govde: { grup: 'f-sunucudaki', sayfa: 1, sayfa_sayisi: 1, cekim: '2026-11-24T09:00:00.000Z' } },
    { no: 'N-7', program: 'tutanak', uc: T.UC.imza, durum: 'bekliyor', govde: { tutanak_id: 41, rol: 'Tanık', imzalayan: 'Örnek Tanık' } },
    { no: 'N-8', program: 'tutanak', uc: T.UC.imza, durum: 'bekliyor', govde: { istemci_no: 'N-1', rol: 'Düzenleyen', imzalayan: 'Örnek Yazan' } },
    { no: 'N-9', program: 'tutanak', uc: T.UC.imza, durum: 'reddedildi', govde: { tutanak_id: 42, rol: 'Tanık', imzalayan: 'x' } },
    { no: 'K-1', program: 'kasa', uc: '/v1/kasa/gunsonu', durum: 'bekliyor', govde: {} },
  ];
  const l = T.tutanakListesi(LISTE(), kayitlar);
  assert.deepEqual(l.map((t) => t.anahtar), ['grup:f-1', 'no:N-1', 'no:N-2', 'no:N-3', 'id:45', 'id:43', 'id:42', 'id:41'], 'yeniden eskiye; çift doğrulama (id 44) yok');
  const bul = (a) => l.find((t) => t.anahtar === a);
  assert.deepEqual(bul('grup:f-1').sayfa, { toplam: 2, bekleyen: 1, reddedilen: 0 });
  assert.equal(bul('grup:f-1').durum, 'ocr_bekliyor');
  assert.equal(bul('id:45').tur, 'foto', 'sunucudaki OCR bekleyen kayıt; yerel grup tekrar gösterilmez');
  assert.equal(bul('no:N-1').bekliyor, true);
  assert.deepEqual(bul('no:N-1').imzalar, [{ rol: 'Düzenleyen', imzalayan: 'Örnek Yazan', imtina: false, bekliyor: true }]);
  assert.equal(bul('no:N-2').reddedildi, true);
  assert.match(bul('no:N-2').ret, /HTTP 422: alan eksik/);
  assert.equal(bul('no:N-3').listeBekliyor, true);
  assert.deepEqual(bul('id:41').imzalar.map((i) => `${i.rol}${i.bekliyor ? '*' : ''}`), ['Düzenleyen', 'Tanık*']);
  assert.equal(bul('id:42').reddedilenImza, 'Tanık');

  const s = sablonlar();
  assert.deepEqual(T.eksikRoller(bul('id:41'), olay()), ['İlgili Personel']);
  assert.deepEqual(T.eksikRoller(bul('id:43'), olay()), [], 'iptal imzalanmaz');
  assert.deepEqual(T.eksikRoller(bul('grup:f-1'), undefined), [], 'fotoğraf imzalanmaz');
  assert.deepEqual(T.suz(l, 'imza', '', s).map((t) => t.anahtar), ['no:N-1', 'no:N-3', 'id:41']);
  assert.deepEqual(T.suz(l, 'gonderilmeyen', '', s).map((t) => t.anahtar), ['grup:f-1', 'no:N-1', 'id:41']);
  assert.deepEqual(T.suz(l, 'ocr', '', s).map((t) => t.anahtar), ['grup:f-1', 'id:45']);
  assert.deepEqual(T.suz(l, 'reddedilen', '', s).map((t) => t.anahtar), ['no:N-2', 'id:42']);
  assert.deepEqual(T.suz(l, 'hepsi', 'ornek tanik', s).map((t) => t.anahtar), ['id:41']);
  assert.deepEqual(T.suz(l, 'hepsi', 'sayim fark', s).map((t) => t.anahtar), ['no:N-2', 'no:N-3']);
  assert.deepEqual(T.tutanakListesi(undefined, undefined), []);
});

test('cizimDuzelt: az nokta reddedilir, taşan nokta kırpılır, uzun çizim seyreltilir', () => {
  assert.equal(T.cizimDuzelt(null), null);
  assert.equal(T.cizimDuzelt({ en: 600, boy: 200, cizgiler: [[1, 2, 3, 4]] }), null, 'bir iki nokta imza değildir');
  assert.equal(T.cizimDuzelt({ en: 0, boy: 200, cizgiler: ornekCizim().cizgiler }), null);
  const c = T.cizimDuzelt({ en: 600, boy: 200, cizgiler: [[-5, 10.4, 700, 250, ...ornekCizim().cizgiler[0]], [1, 2, 3], 'x', [1, NaN]] });
  assert.equal(c.cizgiler.length, 1, 'bozuk çizgiler atılır');
  assert.deepEqual(c.cizgiler[0].slice(0, 4), [0, 10, 600, 200]);
  const uzun = [];
  for (let i = 0; i < 10_000; i += 1) uzun.push(i % 600, i % 200);
  const s = T.cizimDuzelt({ en: 600, boy: 200, cizgiler: [uzun] });
  assert.ok(s.nokta <= T.CIZIM_EN_COK_NOKTA + 1);
  assert.deepEqual(s.cizgiler[0].slice(-2), uzun.slice(-2), 'son nokta korunur');
});

test('imzaHazirla: rol, kişi, çizim ve imtina kuralları; gövde tutanak.imza@1', () => {
  const s = olay();
  const t = T.tutanakListesi(LISTE(), []).find((x) => x.id === 41);
  const h = T.imzaHazirla(t, { rol: 'Tanık', imzalayan: '  Örnek   Tanık ', cizim: ornekCizim() }, s, Date.UTC(2026, 10, 27, 12));
  assert.deepEqual(h, {
    hatalar: [],
    govde: { sozlesme: 'tutanak.imza@1', rol: 'Tanık', imzalayan: 'Örnek Tanık', zaman: '2026-11-27T12:00:00.000Z', tutanak_id: 41, tutanak_sha256: 'a'.repeat(64), cizim: ornekCizim() },
  });
  const hata = (girdi, tutanak = t) => T.imzaHazirla(tutanak, girdi, s).hatalar.join(' | ');
  assert.match(hata({ rol: 'Düzenleyen', imzalayan: 'Başka Kişi', cizim: ornekCizim() }), /"Düzenleyen" imzası zaten atılmış/);
  assert.match(hata({ rol: 'Müdür', imzalayan: 'Başka Kişi', cizim: ornekCizim() }), /"Müdür" rolü yok/);
  assert.match(hata({ rol: '', imzalayan: 'Başka Kişi', cizim: ornekCizim() }), /imza rolü seçilmedi/);
  assert.match(hata({ rol: 'Tanık', imzalayan: 'örnek düzenleyen', cizim: ornekCizim() }), /aynı kişi iki rolde imzalayamaz/);
  assert.match(hata({ rol: 'Tanık', imzalayan: 'A', cizim: ornekCizim() }), /adı soyadı yazılmalı/);
  assert.match(hata({ rol: 'Tanık', imzalayan: 'Başka Kişi', cizim: { en: 600, boy: 200, cizgiler: [] } }), /imza çizilmedi/);
  // İmtina yalnız şablonun izin verdiği rolde ve notla.
  assert.match(hata({ rol: 'Tanık', imzalayan: 'Başka Kişi', imtina: true, imtinaNotu: 'İmzalamak istemedi.' }), /imtina \(imzadan kaçınma\) kaydı tutulmaz/);
  assert.match(hata({ rol: 'İlgili Personel', imzalayan: 'Başka Kişi', imtina: true, imtinaNotu: 'yok' }), /imtina notu 5-300/);
  const imtina = T.imzaHazirla(t, { rol: 'İlgili Personel', imzalayan: 'Başka Kişi', imtina: true, imtinaNotu: 'İmzalamak istemedi.' }, s);
  assert.equal(imtina.govde.imtina, true);
  assert.equal(imtina.govde.imtina_notu, 'İmzalamak istemedi.');
  assert.equal('cizim' in imtina.govde, false);
  // Sunucu kimliği olmayan (gönderilmeyi bekleyen) tutanağa imza istemci_no ile bağlanır.
  const bekleyen = T.tutanakListesi([], [{ no: 'N-1', program: 'tutanak', uc: T.UC.olustur, durum: 'bekliyor', govde: { sablon: 'olay-tutanagi', olusturma: '2026-11-27T10:00:00.000Z', sha256: 'c'.repeat(64) } }])[0];
  const hb = T.imzaHazirla(bekleyen, { rol: 'Düzenleyen', imzalayan: 'Örnek Yazan', cizim: ornekCizim() }, s);
  assert.equal(hb.govde.istemci_no, 'N-1');
  assert.equal(hb.govde.tutanak_sha256, 'c'.repeat(64));
  assert.equal('tutanak_id' in hb.govde, false);
  const liste = T.tutanakListesi(LISTE(), [{ no: 'F', program: 'tutanak', uc: T.UC.foto, durum: 'bekliyor', govde: { grup: 'f-1', sayfa: 1, sayfa_sayisi: 1, cekim: '2026-11-27T09:00:00.000Z' } }]);
  assert.match(hata({ rol: 'Tanık', imzalayan: 'Başka Kişi', cizim: ornekCizim() }, liste.find((x) => x.id === 43)), /iptal edilmiş tutanak imzalanmaz/);
  assert.match(hata({ rol: 'Tanık', imzalayan: 'Başka Kişi', cizim: ornekCizim() }, liste.find((x) => x.tur === 'foto')), /fotoğraf kaydı imzalanmaz/);
  assert.deepEqual(T.imzaHazirla(null, {}, s).hatalar, ['tutanak seçilmedi']);
});

// ---------------------------------------------------------------- ekran

test('modül sözleşmesi ve izin kapısı', async () => {
  assert.equal(ad, 'tutanak');
  assert.equal(baslik, 'Tutaa');
  const { kok, sorulan, kutu, yazilan } = await kur({ izinler: [] });
  assert.match(kok.querySelector('[data-bolge="yetkisiz"]').textContent, /yetkiniz yok/);
  assert.deepEqual([...new Set(sorulan)].sort(), [...TUM].sort(), 'izin adları tek yerden sorulur');
  assert.equal(kok.querySelectorAll('button[data-sekme]').length, 0);
  assert.equal(kutu.length + yazilan.length, 0);
  // Yalnız okuma izni: tek sekme (liste), imza düğmesi yok.
  const s = await kur({ izinler: ['tutanak.oku'] });
  assert.deepEqual(s.kok.querySelectorAll('button[data-sekme]').map((d) => d.getAttribute('data-sekme')), ['liste']);
  assert.equal(s.kok.querySelectorAll('button[data-is="imza-ac"]').length, 0);
  assert.ok(kartlar(s.kok).length > 0);
});

test('yeni tutanak: tür listesinde çift doğrulama yok; form doldurulur, giden kutusuna /v1/tutanak yazılır', async () => {
  const { kok, kutu, yazilan } = await kur();
  const secenekler = alan(kok, 't-sablon').querySelectorAll('option').map((o) => o.value);
  assert.deepEqual(secenekler, ['', 'olay-tutanagi', 'sayim-fark']);
  assert.equal(kok.textContent.includes('Kasa Çift Doğrulama Tutanağı TAA Kasa programında'), true);
  assert.equal(is(kok, 'tutanak-kaydet'), null, 'tür seçilmeden form yok');

  await alan(kok, 't-sablon').sec('olay-tutanagi');
  // Zorunlu alanlar boşken kayıt alınmaz.
  await is(kok, 'tutanak-kaydet').click();
  assert.equal(durum(kok).getAttribute('data-durum'), 'hata');
  assert.match(durum(kok).textContent, /Yer: boş bırakılamaz/);
  assert.equal(kutu.length, 0);

  await alan(kok, 't-a-tarih').yaz('2026-11-27');
  await alan(kok, 't-a-saat').yaz('14:30');
  await alan(kok, 't-a-yer').yaz('Depo');
  await alan(kok, 't-a-tur').sec('Hasar');
  await alan(kok, 't-a-onlemler-1').isaretle();
  await alan(kok, 't-a-aciklama').yaz('Raf devrildi.');
  await alan(kok, 't-a-zarar').yaz('1.250,50');
  await alan(kok, 't-a-acil').isaretle();
  await is(kok, 'tutanak-kaydet').click();
  assert.equal(durum(kok).getAttribute('data-durum'), 'tamam');
  assert.match(durum(kok).textContent, /Olay \/ Tespit Tutanağı kaydedildi ve giden kutusuna alındı \(no SAHTE-1-/);
  assert.equal(kutu.length, 1);
  assert.equal(kutu[0].program, 'tutanak');
  assert.equal(kutu[0].uc, '/v1/tutanak');
  assert.deepEqual(kutu[0].govde.alanlar, { tarih: '2026-11-27', saat: '14:30', yer: 'Depo', tur: 'Hasar', onlemler: ['Kamera kaydı ayrıldı'], aciklama: 'Raf devrildi.', zarar: 1250.5, acil: true });
  assert.match(kutu[0].govde.sha256, /^[0-9a-f]{64}$/);
  assert.equal(yazilan.length, 0, 'modül önbelleğe yazmaz');
  assert.equal(is(kok, 'tutanak-kaydet'), null, 'kayıttan sonra form kapanır (çift kayıt olmasın)');
});

test('yeni tutanak: tablolu şablon, satır ekleme', async () => {
  const { kok, kutu } = await kur();
  await alan(kok, 't-sablon').sec('sayim-fark');
  assert.equal(kok.querySelectorAll('tr[data-satir]').length, 2, 'enAzSatir kadar satır');
  await is(kok, 'satir-ekle').click();
  assert.equal(kok.querySelectorAll('tr[data-satir]').length, 3);
  await alan(kok, 't-a-reyon').yaz('Erkek');
  await alan(kok, 't-s-0-urun').yaz('Gömlek');
  await alan(kok, 't-s-0-kayit').yaz('10');
  await alan(kok, 't-s-0-sayilan').yaz('8');
  await alan(kok, 't-s-2-urun').yaz('Kravat');
  await is(kok, 'tutanak-kaydet').click();
  assert.equal(durum(kok).getAttribute('data-durum'), 'tamam');
  assert.deepEqual(kutu[0].govde.satirlar, [{ urun: 'Gömlek', kayit: 10, sayilan: 8 }, { urun: 'Kravat' }]);
});

test('şablon yoksa: yeni tutanak kapalı ama fotoğraf yolu açık', async () => {
  const { kok, kutu } = await kur({ onbellek: {} });
  assert.match(kok.querySelector('[data-bolge="sablon-yok"]').textContent, /şablonları bu cihaza henüz inmedi/);
  await sekme(kok, 'foto');
  alan(kok, 'f-dosya').files = [sahteDosya('a.jpg', sahteFoto('jpeg'))];
  await is(kok, 'foto-sirala').click();
  assert.equal(durum(kok).getAttribute('data-durum'), 'tamam');
  assert.equal(kutu.length, 1);
  assert.equal('sablon' in kutu[0].govde, false);
});

test('fotoğraftan: seçilen fotoğraflar giden kutusuna ocr_bekliyor olarak yazılır; OCR çalışmaz', async () => {
  const { kok, kutu } = await kur();
  await sekme(kok, 'foto');
  await is(kok, 'foto-sirala').click();
  assert.match(durum(kok).textContent, /Önce tutanağın fotoğrafını çekin ya da seçin/);

  const giris = alan(kok, 'f-dosya');
  assert.equal(giris.getAttribute('type'), 'file');
  assert.match(giris.getAttribute('accept'), /image\/jpeg/);
  giris.files = [sahteDosya('sayfa 1.jpg', sahteFoto('jpeg', 200)), sahteDosya('sayfa2.png', sahteFoto('png', 120))];
  await giris.dispatchEvent('change');
  assert.match(kok.querySelector('[data-bolge="foto-ozet"]').textContent, /2 fotoğraf seçildi/);
  await alan(kok, 'f-sablon').sec('olay-tutanagi');
  await alan(kok, 'f-not').yaz('depo olayı');
  await is(kok, 'foto-sirala').click();
  assert.equal(durum(kok).getAttribute('data-durum'), 'tamam');
  assert.match(durum(kok).textContent, /2 fotoğraf giden kutusuna alındı \(OCR bekliyor\).*sonuç sunucuda üretilir/);
  assert.equal(kutu.length, 2);
  for (const [i, k] of kutu.entries()) {
    assert.equal(k.program, 'tutanak');
    assert.equal(k.uc, '/v1/tutanak/foto');
    assert.equal(k.govde.durum, 'ocr_bekliyor');
    assert.equal(k.govde.sayfa, i + 1);
    assert.equal(k.govde.sayfa_sayisi, 2);
    assert.equal(k.govde.sablon, 'olay-tutanagi');
    assert.equal(k.govde.grup, kutu[0].govde.grup, 'sayfalar aynı gruba bağlı');
  }
  assert.match(kutu[0].govde.grup, /^f-[0-9a-f]{16}$/);
  assert.deepEqual([kutu[0].govde.dosya.tur, kutu[1].govde.dosya.tur], ['image/jpeg', 'image/png']);
  assert.equal(is(kok, 'foto-sirala').disabled, false);

  // Liste: fotoğraf grubu tek satır, "OCR bekliyor".
  await sekme(kok, 'liste');
  const kart = kok.querySelector(`li[data-tutanak="grup:${kutu[0].govde.grup}"]`);
  assert.equal(kart.getAttribute('data-tur'), 'foto');
  assert.match(kart.textContent, /OCR bekliyor/);
  assert.match(kart.querySelector('[data-bolge="foto-durum"]').textContent, /2 sayfa · 2 sayfa gönderilmeyi bekliyor/);
});

test('fotoğraftan: geçersiz ya da fazla dosyada hiçbir şey sıraya girmez (ya hepsi ya hiçbiri)', async () => {
  const { kok, kutu } = await kur();
  await sekme(kok, 'foto');
  alan(kok, 'f-dosya').files = [sahteDosya('a.jpg', sahteFoto('jpeg')), sahteDosya('belge.pdf', sahteFoto('pdf'))];
  await is(kok, 'foto-sirala').click();
  assert.equal(durum(kok).getAttribute('data-durum'), 'hata');
  assert.match(durum(kok).textContent, /Fotoğraf 2: yalnız JPEG, PNG ya da WebP/);
  assert.equal(kutu.length, 0);

  alan(kok, 'f-dosya').files = [1, 2, 3, 4].map((n) => sahteDosya(`${n}.jpg`, sahteFoto('jpeg')));
  await is(kok, 'foto-sirala').click();
  assert.match(durum(kok).textContent, /en çok 3 fotoğraf/);
  // Sahte DOM'da küçültme (tuval) yok: sınırı aşan fotoğraf reddedilir, sessizce kırpılmaz.
  alan(kok, 'f-dosya').files = [sahteDosya('buyuk.jpg', sahteFoto('jpeg', T.FOTO_EN_COK + 10))];
  await is(kok, 'foto-sirala').click();
  assert.match(durum(kok).textContent, /çok büyük/);
  assert.equal(kutu.length, 0);

  // Yetkisi olmayan fotoğraf sekmesini görmez.
  const s = await kur({ izinler: ['tutanak.oku'] });
  assert.equal(s.kok.querySelector('button[data-sekme="foto"]'), null);
});

test('liste: süzgeçler, arama, çift doğrulama kaydı görünmez; giden kutusu değişince yenilenir', async () => {
  const { kok, pencere, baglam } = await kur();
  await sekme(kok, 'liste');
  assert.deepEqual(kartlar(kok), ['id:45', 'id:43', 'id:42', 'id:41']);
  assert.equal(kok.textContent.includes('KCD'), false, 'Kasa çift doğrulama tutanağı listelenmez');
  assert.match(kok.querySelector('li[data-tutanak="id:41"]').querySelector('[data-bolge="eksik-imza"]').textContent, /Eksik imza: Tanık, İlgili Personel/);
  await kok.querySelector('button[data-suzgec="imza"]').click();
  assert.deepEqual(kartlar(kok), ['id:41']);
  await kok.querySelector('button[data-suzgec="ocr"]').click();
  assert.deepEqual(kartlar(kok), ['id:45']);
  await kok.querySelector('button[data-suzgec="hepsi"]').click();
  await alan(kok, 'l-ara').yaz('000042');
  assert.deepEqual(kartlar(kok), ['id:42']);
  await alan(kok, 'l-ara').yaz('hiç yok');
  assert.match(kok.querySelector('[data-bolge="bos"]').textContent, /Bu süzgece uyan tutanak yok/);
  await alan(kok, 'l-ara').yaz('');

  // Başka sekmeden / kabuktan giden kutusu değişirse liste kendini yeniler; arama kutusu yerinde kalır.
  await baglam.giden.ekle({ program: 'tutanak', uc: T.UC.olustur, govde: { sablon: 'sayim-fark', olusturma: '2026-11-27T10:00:00.000Z' } });
  pencere.dispatchEvent({ type: 'taa-giden-degisti' });
  await bekle();
  assert.equal(kartlar(kok)[0], 'no:SAHTE-1-1700000000');
  assert.match(kok.querySelector('li[data-tutanak="no:SAHTE-1-1700000000"]').textContent, /Numara bekleniyor.*gönderilmeyi bekliyor/);
  assert.equal(kok.querySelectorAll('[data-alan="l-ara"]').length, 1);
});

test('imza: imzası eksik tutanak seçilir, ad yazılır, kutuya çizilir, giden kutusuna /v1/tutanak/imza yazılır', async () => {
  const { kok, kutu } = await kur();
  await sekme(kok, 'imza');
  assert.deepEqual(kartlar(kok), ['id:41'], 'yalnız imzası eksik, iptal olmayan, fotoğraf olmayan tutanak');
  await kok.querySelector('button[data-tutanak="id:41"]').click();
  assert.deepEqual(alan(kok, 'i-rol').querySelectorAll('option').map((o) => o.value), ['Tanık', 'İlgili Personel']);

  // Çizmeden imzalanmaz.
  await alan(kok, 'i-imzalayan').yaz('Örnek Tanık');
  await is(kok, 'imza-at').click();
  assert.match(durum(kok).textContent, /imza çizilmedi/);
  assert.equal(kutu.length, 0);

  const tuval = alan(kok, 'i-tuval');
  assert.equal(tuval.tagName, 'CANVAS');
  const ciz = (n, kay = 0) => {
    tuval.dispatchEvent({ type: 'pointerdown', offsetX: 10 + kay, offsetY: 50 });
    for (let i = 1; i < n; i += 1) tuval.dispatchEvent({ type: 'pointermove', offsetX: 10 + kay + i * 8, offsetY: 50 + (i % 3) * 6 });
    tuval.dispatchEvent({ type: 'pointerup' });
  };
  ciz(3);
  tuval.dispatchEvent({ type: 'pointermove', offsetX: 500, offsetY: 5 }); // basılı değilken hareket çizmez
  await is(kok, 'imza-at').click();
  assert.match(durum(kok).textContent, /imza çizilmedi/, 'üç nokta imza sayılmaz');
  await is(kok, 'imza-temizle').click();
  ciz(10);
  ciz(6, 200);
  await is(kok, 'imza-at').click();
  assert.equal(durum(kok).getAttribute('data-durum'), 'tamam');
  assert.match(durum(kok).textContent, /Tanık imzası \(Örnek Tanık\) giden kutusuna alındı/);
  assert.equal(kutu.length, 1);
  assert.equal(kutu[0].uc, '/v1/tutanak/imza');
  const g = kutu[0].govde;
  assert.deepEqual([g.sozlesme, g.tutanak_id, g.rol, g.imzalayan, g.tutanak_sha256], ['tutanak.imza@1', 41, 'Tanık', 'Örnek Tanık', 'a'.repeat(64)]);
  assert.deepEqual(g.cizim.cizgiler.map((c) => c.length / 2), [10, 6], 'temizlenen çizim gitmez; iki çizgi');
  assert.deepEqual([g.cizim.en, g.cizim.boy], [600, 200]);

  // Liste yenilendi: Tanık artık "gönderilmeyi bekliyor"; aynı rol yeniden seçilemez.
  assert.match(kok.querySelector('li[data-tutanak="id:41"]').querySelector('[data-bolge="imzalar"]').textContent, /Tanık: Örnek Tanık \(gönderilmeyi bekliyor\)/);
  await kok.querySelector('button[data-tutanak="id:41"]').click();
  assert.deepEqual(alan(kok, 'i-rol').querySelectorAll('option').map((o) => o.value), ['İlgili Personel']);

  // İmtina: çizim yerine not.
  await alan(kok, 'i-imzalayan').yaz('Örnek İlgili');
  await alan(kok, 'i-imtina').isaretle();
  await is(kok, 'imza-at').click();
  assert.match(durum(kok).textContent, /imtina notu 5-300/);
  await alan(kok, 'i-imtina-notu').yaz('İmzalamak istemedi.');
  await is(kok, 'imza-at').click();
  assert.equal(kutu.length, 2);
  assert.equal(kutu[1].govde.imtina, true);
  assert.match(kok.querySelector('[data-bolge="bos"]').textContent, /İmza bekleyen tutanak yok/);
});

test('imza: yeni yazılan (henüz gönderilmemiş) tutanak istemci_no ile imzalanır', async () => {
  const { kok, kutu } = await kur({ onbellek: { 'tutanak/sablonlar': SABLONLAR() } });
  await alan(kok, 't-sablon').sec('sayim-fark');
  await alan(kok, 't-a-reyon').yaz('Erkek');
  await is(kok, 'tutanak-kaydet').click();
  await sekme(kok, 'imza');
  assert.deepEqual(kartlar(kok), [`no:${kutu[0].no}`]);
  await is(kok, 'imza-ac').click();
  await alan(kok, 'i-imzalayan').yaz('Örnek Sayan');
  const tuval = alan(kok, 'i-tuval');
  tuval.dispatchEvent({ type: 'pointerdown', offsetX: 5, offsetY: 5 });
  for (let i = 1; i < 12; i += 1) tuval.dispatchEvent({ type: 'pointermove', offsetX: 5 + i * 4, offsetY: 5 + i });
  await is(kok, 'imza-at').click();
  assert.equal(kutu.length, 2);
  assert.equal(kutu[1].govde.istemci_no, kutu[0].no);
  assert.equal(kutu[1].govde.tutanak_sha256, kutu[0].govde.sha256);
  assert.equal(kutu[1].govde.rol, 'Sayan');
});

test('modül kuralları: ağ çağrısı, gonder, önbelleğe yazma, innerHTML ve OCR kodu yok; LISTE.json ile tutarlı', () => {
  const klasor = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
  for (const dosya of ['index.js', 'mantik.js']) {
    const s = readFileSync(path.join(klasor, dosya), 'utf8');
    assert.doesNotMatch(s, /innerHTML|outerHTML|insertAdjacentHTML|document\.write/, dosya);
    assert.doesNotMatch(s, /\bfetch\(|XMLHttpRequest|WebSocket|sendBeacon/, dosya);
    assert.doesNotMatch(s, /giden\.gonder|onbellek\.yaz|indexedDB|localStorage/, dosya);
    assert.doesNotMatch(s, /tesseract|TextDetector|recognize\(/i, `${dosya}: OCR istemcide yapılmaz`);
  }
  const liste = JSON.parse(readFileSync(path.join(klasor, '..', 'LISTE.json'), 'utf8'));
  const m = liste.moduller.find((x) => x.ad === 'tutanak');
  assert.ok(m, 'LISTE.json içinde tutanak yok');
  assert.deepEqual([...m.izinler].sort(), Object.values(T.IZIN).sort());
  assert.deepEqual([...m.uclar].sort(), Object.values(T.UC).sort());
  assert.deepEqual([...m.onbellek].sort(), Object.values(T.ANAHTAR).sort());
  assert.deepEqual([...m.sozlesmeler].sort(), Object.values(T.SOZLESME).sort());
  const kabuk = JSON.parse(readFileSync(path.join(klasor, '..', '..', 'kabuk', 'moduller.json'), 'utf8')).moduller.find((x) => x.ad === 'tutanak');
  assert.ok(kabuk, 'kabuk/moduller.json içinde tutanak yok');
  assert.deepEqual([...kabuk.dosyalar].sort(), [...m.dosyalar].sort());
  for (const d of m.dosyalar) assert.doesNotThrow(() => readFileSync(path.join(klasor, '..', '..', d)), d);
});
