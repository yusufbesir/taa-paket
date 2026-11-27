// Sahip: Yusuf. yazan: [Claude Fable 5.2 #taa-web-W3-tutanak 2026-11-27]
// Tutaa (tutanak) web modülü: şablondan tutanak oluşturma, fotoğraf çekme/yükleme (OCR sırası), liste ve imza.
// Yazmalar yalnız baglam.giden.ekle ile; okumalar yalnız baglam.onbellek ve giden listesiyle (çevrimdışı çalışır).
// OCR bu modülde YOK: fotoğraf giden kutusuna `ocr_bekliyor` olarak yazılır, sonuç sunucuda üretilir.
// Kasa Çift Doğrulama Tutanağı bu modülde YOK (TAA Kasa'da). Silme / iptal yok. Kurallar: ./mantik.js.

import { el, alan, bul, doldur, temizle, stilEkle, durumBolgesi, durumYaz, onbellekOku, gidenListe, gidenDegisinceYenile } from '../ortak/dom.js';
import { bugun, tarihYaz } from '../ortak/bicim.js';
import * as T from './mantik.js';

export const ad = 'tutanak';
export const baslik = 'Tutaa';

const SEKMELER = [
  ['yeni', 'Yeni tutanak', T.IZIN.olustur],
  ['foto', 'Fotoğraftan', T.IZIN.foto],
  ['liste', 'Tutanaklar', T.IZIN.oku],
  ['imza', 'İmza', T.IZIN.imza],
];

const hataMetni = (e) => (e && e.message ? e.message : String(e));

/** Modülü `kok` içine kurar. İlk çizim bitince çözülen söz döner. */
export async function ac(kok, baglam) {
  const doc = kok.ownerDocument;
  temizle(kok);
  stilEkle(kok);
  kok.appendChild(el(doc, 'link', { rel: 'stylesheet', 'data-taa-stil': 'tutanak', href: new URL('./tutanak.css', import.meta.url).href }));
  const acik = SEKMELER.filter(([, , izin]) => baglam.yetki(izin));
  if (acik.length === 0) {
    kok.appendChild(el(doc, 'p', { 'data-bolge': 'yetkisiz', text: 'Tutaa: bu ekranı açma yetkiniz yok. Yöneticinize başvurun.' }));
    return;
  }
  const durum = durumBolgesi(doc);
  const govde = el(doc, 'div', { 'data-bolge': 'govde' });
  const ortam = { doc, kok, baglam, durum, govde, sablonlar: [], imzaYetkisi: baglam.yetki(T.IZIN.imza) };
  ortam.sablonlar = T.sablonlariDuzelt(await onbellekOku(baglam, T.ANAHTAR.sablonlar));
  let secili = acik[0][0];
  const dugmeler = acik.map(([kod, yazi]) =>
    el(doc, 'button', { type: 'button', 'data-sekme': kod, 'aria-pressed': 'false', text: yazi, onclick: () => sec(kod) }),
  );
  async function sec(kod, koru = false) {
    secili = kod;
    for (const d of dugmeler) d.setAttribute('aria-pressed', d.getAttribute('data-sekme') === kod ? 'true' : 'false');
    if (!koru) durumYaz(durum, 'bilgi');
    temizle(govde);
    await CIZ[kod](ortam);
  }
  ortam.sec = sec;
  kok.appendChild(el(doc, 'h2', { text: baslik }));
  kok.appendChild(el(doc, 'div', { class: 'taa-sekmeler', role: 'group', 'aria-label': 'Tutaa bölümleri' }, dugmeler));
  kok.appendChild(durum);
  kok.appendChild(govde);
  // Liste ve imza giden kutusuna bağlıdır; kutu değişince (gönderildi / reddedildi) yenilenir.
  // Form sekmeleri yenilenmez: yazılmakta olan tutanak silinmesin.
  gidenDegisinceYenile(kok, () => {
    if (secili === 'liste') return listeCiz(ortam, true);
    if (secili === 'imza' && !ortam.imzaAcik) return imzaCiz(ortam);
    return undefined;
  });
  await sec(secili);
}

const CIZ = { yeni: yeniCiz, foto: fotoCiz, liste: listeCiz, imza: imzaCiz };

const sablonAdi = (o, kimlik) => (o.sablonlar.find((s) => s.kimlik === kimlik) || {}).ad || kimlik || 'Tutanak';

// ---------------------------------------------------------------- yeni tutanak

function girisKur(o, a, veriAdi) {
  if (a.tur === 'coklu') {
    return el(
      o.doc,
      'fieldset',
      { class: 'tutanak-secenekler' },
      el(o.doc, 'legend', { class: 'taa-alan-ad', text: a.etiket + (a.zorunlu ? ' *' : '') }),
      a.secenekler.map((s, i) => el(o.doc, 'label', {}, el(o.doc, 'input', { type: 'checkbox', 'data-alan': `${veriAdi}-${i}` }), el(o.doc, 'span', { text: s }))),
    );
  }
  if (a.tur === 'onay') {
    return el(o.doc, 'div', { class: 'tutanak-secenekler' }, el(o.doc, 'label', {}, el(o.doc, 'input', { type: 'checkbox', 'data-alan': veriAdi }), el(o.doc, 'span', { text: a.etiket })));
  }
  const etiket = a.etiket + (a.zorunlu ? ' *' : '');
  if (a.tur === 'secim') return alan(o.doc, etiket, veriAdi, { secenekler: [['', 'Seçin'], ...a.secenekler.map((s) => [s, s])], value: '' });
  if (a.tur === 'uzunmetin') return alan(o.doc, etiket, veriAdi, { cokSatir: true, rows: 3, maxlength: T.UZUN_METIN_EN_COK });
  if (a.tur === 'tarih') return alan(o.doc, etiket, veriAdi, { type: 'date', value: bugun() });
  if (a.tur === 'saat') return alan(o.doc, etiket, veriAdi, { type: 'time' });
  if (a.tur === 'sayi') return alan(o.doc, etiket, veriAdi, { inputmode: 'decimal' });
  return alan(o.doc, etiket, veriAdi, { maxlength: T.METIN_EN_COK });
}

function degerOku(kok, a, veriAdi) {
  if (a.tur === 'coklu') return a.secenekler.filter((s, i) => bul(kok, `${veriAdi}-${i}`).checked);
  if (a.tur === 'onay') return bul(kok, veriAdi).checked === true;
  return bul(kok, veriAdi).value;
}

function hucreKur(o, sutun, veriAdi, deger) {
  const oz = { 'data-alan': veriAdi, 'aria-label': sutun.etiket };
  if (sutun.tur === 'secim') {
    return el(o.doc, 'select', { ...oz, value: '' }, [['', '—'], ...sutun.secenekler.map((s) => [s, s])].map(([d, y]) => el(o.doc, 'option', { value: d, text: y })));
  }
  if (sutun.tur === 'onay') return el(o.doc, 'input', { ...oz, type: 'checkbox' });
  return el(o.doc, 'input', { ...oz, type: sutun.tur === 'tarih' ? 'date' : sutun.tur === 'saat' ? 'time' : 'text', inputmode: sutun.tur === 'sayi' ? 'decimal' : null, autocomplete: 'off', value: deger });
}

async function yeniCiz(o) {
  const { doc } = o;
  if (o.sablonlar.length === 0) {
    o.govde.appendChild(
      el(doc, 'p', {
        'data-bolge': 'sablon-yok',
        text: 'Tutanak şablonları bu cihaza henüz inmedi. Bir kez çevrimiçi açıp eşitleyin. Kâğıt tutanağınız varsa "Fotoğraftan" bölümünden fotoğrafını sıraya alabilirsiniz.',
      }),
    );
    return;
  }
  const form = el(doc, 'div', { 'data-bolge': 'form' });
  let sablon = null;
  let satirSayisi = 0;
  let tabloGovdesi = null;

  function satirEkle() {
    const i = satirSayisi;
    satirSayisi += 1;
    tabloGovdesi.appendChild(
      el(doc, 'tr', { 'data-satir': i }, sablon.tablo.sutunlar.map((s, j) => el(doc, 'td', {}, hucreKur(o, s, `t-s-${i}-${s.id}`, j === 0 ? sablon.tablo.sabitSatirlar[i] : undefined)))),
    );
  }

  function formKur() {
    temizle(form);
    sablon = o.sablonlar.find((s) => s.kimlik === bul(o.govde, 't-sablon').value) || null;
    satirSayisi = 0;
    if (!sablon) return;
    form.appendChild(el(doc, 'h3', { text: sablon.baslik }));
    for (const b of sablon.bolumler) {
      form.appendChild(el(doc, 'div', { class: 'taa-kutu' }, b.baslik ? el(doc, 'strong', { text: b.baslik }) : null, b.alanlar.map((a) => girisKur(o, a, `t-a-${a.id}`))));
    }
    if (sablon.tablo) {
      tabloGovdesi = el(doc, 'tbody', {});
      form.appendChild(
        el(
          doc,
          'div',
          { class: 'taa-kutu' },
          sablon.tablo.baslik ? el(doc, 'strong', { text: sablon.tablo.baslik }) : null,
          el(doc, 'div', { class: 'tutanak-tablo-sarmal' }, el(doc, 'table', { class: 'taa-tablo' }, el(doc, 'thead', {}, el(doc, 'tr', {}, sablon.tablo.sutunlar.map((s) => el(doc, 'th', { scope: 'col', text: s.etiket })))), tabloGovdesi)),
          el(doc, 'button', { type: 'button', 'data-is': 'satir-ekle', text: 'Satır ekle', onclick: () => (satirSayisi < T.SATIR_EN_COK ? satirEkle() : durumYaz(o.durum, 'uyari', `Tablo en çok ${T.SATIR_EN_COK} satır olabilir.`)) }),
        ),
      );
      for (let i = 0; i < Math.max(sablon.tablo.enAzSatir, 1); i += 1) satirEkle();
    }
    const roller = T.sablonRolleri(sablon);
    form.appendChild(el(doc, 'p', { class: 'taa-soluk', 'data-bolge': 'imza-notu', text: `İmzalar kayıttan sonra "İmza" bölümünde alınır: ${roller.join(', ')}. Tutanak numarasını sunucu verir.` }));
    form.appendChild(el(doc, 'button', { type: 'button', class: 'taa-ana', 'data-is': 'tutanak-kaydet', text: 'Tutanağı kaydet', onclick: kaydet }));
  }

  async function kaydet() {
    if (!o.baglam.yetki(T.IZIN.olustur)) return durumYaz(o.durum, 'hata', 'Tutanak oluşturma yetkiniz yok.');
    if (!sablon) return durumYaz(o.durum, 'hata', 'Önce tutanak türünü seçin.');
    const girdi = { alanlar: {}, satirlar: [] };
    for (const b of sablon.bolumler) for (const a of b.alanlar) girdi.alanlar[a.id] = degerOku(form, a, `t-a-${a.id}`);
    for (let i = 0; i < satirSayisi; i += 1) {
      const satir = {};
      for (const s of sablon.tablo.sutunlar) satir[s.id] = degerOku(form, s, `t-s-${i}-${s.id}`);
      girdi.satirlar.push(satir);
    }
    const hazir = T.tutanakHazirla(sablon, girdi);
    if (hazir.hatalar.length) return durumYaz(o.durum, 'hata', hazir.hatalar);
    let sonuc;
    try {
      sonuc = await o.baglam.giden.ekle({ program: T.PROGRAM, uc: T.UC.olustur, govde: await T.ozetEkle(hazir.govde) });
    } catch (e) {
      return durumYaz(o.durum, 'hata', `Tutanak giden kutusuna yazılamadı: ${hataMetni(e)}`);
    }
    const kaydedilen = sablon.ad;
    bul(o.govde, 't-sablon').value = '';
    formKur();
    return durumYaz(
      o.durum,
      'tamam',
      `${kaydedilen} kaydedildi ve giden kutusuna alındı (no ${sonuc.no}); bağlantı gelince sunucuya gider.${o.imzaYetkisi ? ' İmzalar için "İmza" bölümüne geçin.' : ''}`,
    );
  }

  o.govde.appendChild(
    alan(doc, 'Tutanak türü', 't-sablon', { secenekler: [['', 'Seçin'], ...o.sablonlar.map((s) => [s.kimlik, s.ad])], value: '', onchange: formKur }),
  );
  o.govde.appendChild(el(doc, 'p', { class: 'taa-soluk', text: 'Kasa Çift Doğrulama Tutanağı TAA Kasa programında düzenlenir; burada yoktur.' }));
  o.govde.appendChild(form);
}

// ---------------------------------------------------------------- fotoğraftan (OCR sırası)

/** Tarayıcıda fotoğrafı küçültür (uzun kenar 1600 px, JPEG). Tuval / createImageBitmap yoksa `null` döner. */
async function kucult(dosya, doc) {
  const pencere = doc.defaultView || globalThis;
  if (typeof pencere.createImageBitmap !== 'function') return null;
  let resim;
  try {
    resim = await pencere.createImageBitmap(dosya);
  } catch {
    return null;
  }
  try {
    const olcu = T.kucultmeOlcusu(resim.width, resim.height);
    const tuval = doc.createElement('canvas');
    const cizer = typeof tuval.getContext === 'function' ? tuval.getContext('2d') : null;
    if (!olcu || !cizer || typeof tuval.toBlob !== 'function') return null;
    tuval.width = olcu.en;
    tuval.height = olcu.boy;
    cizer.drawImage(resim, 0, 0, olcu.en, olcu.boy);
    for (const kalite of [0.85, 0.7, 0.55, 0.4]) {
      const parca = await new Promise((coz) => tuval.toBlob(coz, 'image/jpeg', kalite));
      if (parca && parca.size <= T.FOTO_EN_COK) return new Uint8Array(await parca.arrayBuffer());
    }
    return null;
  } finally {
    if (typeof resim.close === 'function') resim.close();
  }
}

function grupKimligi() {
  const bayt = new Uint8Array(8);
  globalThis.crypto.getRandomValues(bayt);
  return `f-${[...bayt].map((b) => b.toString(16).padStart(2, '0')).join('')}`;
}

async function fotoCiz(o) {
  const { doc } = o;
  const secim = el(doc, 'input', { 'data-alan': 'f-dosya', type: 'file', accept: 'image/jpeg,image/png,image/webp', multiple: true });
  // `capture` bilerek yok: telefonda hem kamera hem galeri seçeneği çıksın (çekme ve yükleme).
  const ozet = el(doc, 'p', { class: 'taa-soluk', 'data-bolge': 'foto-ozet', text: 'Fotoğraf seçilmedi.' });
  secim.addEventListener('change', () => {
    const n = secim.files ? secim.files.length : 0;
    doldur(ozet, n === 0 ? 'Fotoğraf seçilmedi.' : `${n} fotoğraf seçildi.${n > T.FOTO_EN_COK_ADET ? ` En çok ${T.FOTO_EN_COK_ADET} fotoğraf gönderilebilir.` : ''}`);
  });

  async function siraya() {
    if (!o.baglam.yetki(T.IZIN.foto)) return durumYaz(o.durum, 'hata', 'Fotoğraf gönderme yetkiniz yok.');
    const dosyalar = secim.files ? Array.from(secim.files) : [];
    if (dosyalar.length === 0) return durumYaz(o.durum, 'hata', 'Önce tutanağın fotoğrafını çekin ya da seçin.');
    if (dosyalar.length > T.FOTO_EN_COK_ADET) return durumYaz(o.durum, 'hata', `Bir tutanak için en çok ${T.FOTO_EN_COK_ADET} fotoğraf gönderilebilir.`);
    const dugme = o.govde.querySelector('[data-is="foto-sirala"]');
    dugme.disabled = true;
    durumYaz(o.durum, 'bilgi', 'Fotoğraflar okunuyor…');
    try {
      const grup = grupKimligi();
      const hazir = [];
      const hatalar = [];
      for (const [i, d] of dosyalar.entries()) {
        let bayt;
        try {
          bayt = new Uint8Array(await d.arrayBuffer());
          if (bayt.length > T.FOTO_EN_COK) bayt = (await kucult(d, doc)) || bayt; // küçültülemezse sınır hatası aşağıda yazılır
        } catch (e) {
          hatalar.push(`Fotoğraf ${i + 1}: okunamadı (${hataMetni(e)})`);
          continue;
        }
        const h = await T.fotoHazirla({ ad: d.name, bayt }, { grup, sayfa: i + 1, sayfaSayisi: dosyalar.length, sablon: bul(o.govde, 'f-sablon').value || undefined, not: bul(o.govde, 'f-not').value });
        if (h.hatalar.length) hatalar.push(...h.hatalar);
        else hazir.push(h.govde);
      }
      // Ya hepsi ya hiçbiri: eksik sayfalı tutanak sıraya girmesin.
      if (hatalar.length) return durumYaz(o.durum, 'hata', [...new Set(hatalar)]);
      const nolar = [];
      try {
        for (const g of hazir) nolar.push((await o.baglam.giden.ekle({ program: T.PROGRAM, uc: T.UC.foto, govde: g })).no);
      } catch (e) {
        return durumYaz(
          o.durum,
          'hata',
          `Fotoğraf giden kutusuna yazılamadı: ${hataMetni(e)}${nolar.length ? ` (${nolar.length} sayfa yazıldı, kalanı yazılamadı; yöneticinize bildirin.)` : ''}`,
        );
      }
      secim.value = '';
      bul(o.govde, 'f-not').value = '';
      doldur(ozet, 'Fotoğraf seçilmedi.');
      return durumYaz(
        o.durum,
        'tamam',
        `${hazir.length} fotoğraf giden kutusuna alındı (OCR bekliyor). Çözümleme bu cihazda yapılmaz; bağlantı gelince sunucuya gider ve sonuç sunucuda üretilir. Durumu "Tutanaklar" bölümünden izleyin.`,
      );
    } finally {
      dugme.disabled = false;
    }
  }

  o.govde.appendChild(
    el(doc, 'p', { class: 'taa-soluk', text: `Kâğıt tutanağın fotoğrafını çekin ya da seçin (en çok ${T.FOTO_EN_COK_ADET} sayfa). Fotoğraf sıraya alınır; yazıya çevirme (OCR) sunucuda yapılır, sonuç hemen gelmez.` }),
  );
  o.govde.appendChild(el(doc, 'label', { class: 'taa-alan' }, el(doc, 'span', { class: 'taa-alan-ad', text: 'Fotoğraf (kamera ya da dosya)' }), secim));
  o.govde.appendChild(ozet);
  o.govde.appendChild(
    el(
      doc,
      'div',
      { class: 'taa-satir' },
      alan(doc, 'Belge türü (biliniyorsa)', 'f-sablon', { secenekler: [['', 'Bilinmiyor'], ...o.sablonlar.map((s) => [s.kimlik, s.ad])], value: '' }),
      alan(doc, 'Not (isteğe bağlı)', 'f-not', { maxlength: 300 }),
    ),
  );
  o.govde.appendChild(el(doc, 'button', { type: 'button', class: 'taa-ana', 'data-is': 'foto-sirala', text: 'Sıraya al', onclick: siraya }));
}

// ---------------------------------------------------------------- liste

async function listeOku(o) {
  const [ham, kayitlar] = await Promise.all([onbellekOku(o.baglam, T.ANAHTAR.liste), gidenListe(o.baglam, T.PROGRAM)]);
  return T.tutanakListesi(ham, kayitlar);
}

function rozetler(o, t) {
  const { doc } = o;
  const r = [el(doc, 'span', { class: 'taa-rozet', 'data-rozet': 'durum', 'data-tur': t.durum === 'iptal' ? 'hata' : t.durum === T.OCR_BEKLIYOR ? 'uyari' : null, text: T.durumAdi(t.durum) })];
  if (t.bekliyor) r.push(el(doc, 'span', { class: 'taa-rozet', 'data-tur': 'uyari', 'data-rozet': 'bekliyor', text: 'gönderilmeyi bekliyor' }));
  if (t.listeBekliyor) r.push(el(doc, 'span', { class: 'taa-rozet', 'data-rozet': 'liste-bekliyor', text: 'gönderildi; sunucu listesi bekleniyor' }));
  if (t.reddedildi) r.push(el(doc, 'span', { class: 'taa-rozet', 'data-tur': 'hata', 'data-rozet': 'ret', text: `sunucu reddetti${t.ret ? ` (${t.ret})` : ''}` }));
  if (t.reddedilenImza) r.push(el(doc, 'span', { class: 'taa-rozet', 'data-tur': 'hata', 'data-rozet': 'imza-ret', text: `imza reddedildi: ${t.reddedilenImza}` }));
  return r;
}

function kartIcerigi(o, t) {
  const { doc } = o;
  const sablon = o.sablonlar.find((s) => s.kimlik === t.sablon);
  const eksik = T.eksikRoller(t, sablon);
  const satirlar = [
    el(doc, 'div', { class: 'taa-satir' }, el(doc, 'strong', { text: t.no || (t.tur === 'foto' ? 'Fotoğraf' : 'Numara bekleniyor') }), rozetler(o, t)),
    el(doc, 'span', { text: `${t.sablon ? sablonAdi(o, t.sablon) : t.tur === 'foto' ? 'Belge türü bilinmiyor' : 'Tutanak'}${t.tarih ? ` · ${tarihYaz(t.tarih)}` : ''}${t.baslik ? ` · ${t.baslik}` : ''}` }),
  ];
  if (t.tur === 'foto') {
    const s = t.sayfa;
    satirlar.push(
      el(doc, 'span', {
        class: 'taa-soluk',
        'data-bolge': 'foto-durum',
        text: s
          ? `${s.toplam} sayfa · ${s.bekleyen > 0 ? `${s.bekleyen} sayfa gönderilmeyi bekliyor` : 'gönderildi; sunucuda çözümlenmeyi (OCR) bekliyor'}`
          : 'Sunucuda çözümlenmeyi (OCR) bekliyor',
      }),
    );
  } else {
    if (t.imzalar.length) {
      satirlar.push(el(doc, 'span', { class: 'taa-soluk', 'data-bolge': 'imzalar', text: `İmzalar: ${t.imzalar.map((i) => `${i.rol}: ${i.imzalayan}${i.imtina ? ' (imtina)' : ''}${i.bekliyor ? ' (gönderilmeyi bekliyor)' : ''}`).join('; ')}` }));
    }
    if (eksik.length) satirlar.push(el(doc, 'span', { class: 'taa-soluk', 'data-bolge': 'eksik-imza', text: `Eksik imza: ${eksik.join(', ')}` }));
  }
  return satirlar;
}

async function listeCiz(o, yenileme = false) {
  const { doc } = o;
  if (!yenileme || !o.listeBolgesi || o.listeBolgesi.isConnected === false) {
    o.listeSuzgec = 'hepsi';
    o.listeBolgesi = el(doc, 'div', { 'data-bolge': 'liste' });
    const dugmeler = T.SUZGECLER.map(([kod, yazi]) =>
      el(doc, 'button', {
        type: 'button',
        'data-suzgec': kod,
        'aria-pressed': kod === 'hepsi' ? 'true' : 'false',
        text: yazi,
        onclick: () => {
          o.listeSuzgec = kod;
          for (const d of dugmeler) d.setAttribute('aria-pressed', d.getAttribute('data-suzgec') === kod ? 'true' : 'false');
          doldurListe();
        },
      }),
    );
    o.govde.appendChild(el(doc, 'div', { class: 'taa-sekmeler', role: 'group', 'aria-label': 'Tutanak süzgeci' }, dugmeler));
    o.govde.appendChild(alan(doc, 'Ara (numara, tür, başlık, imzalayan)', 'l-ara', { type: 'search', oninput: () => doldurListe() }));
    o.govde.appendChild(o.listeBolgesi);
  }
  function doldurListe() {
    const l = T.suz(o.liste, o.listeSuzgec, bul(o.govde, 'l-ara').value, o.sablonlar);
    doldur(
      o.listeBolgesi,
      l.length === 0
        ? el(doc, 'p', { 'data-bolge': 'bos', text: o.liste.length === 0 ? 'Bu cihazda tutanak yok. Yeni tutanak düzenleyin ya da bir kez çevrimiçi açıp eşitleyin.' : 'Bu süzgece uyan tutanak yok.' })
        : el(doc, 'ul', { class: 'taa-liste' }, l.map((t) => el(doc, 'li', { class: 'taa-kutu', 'data-tutanak': t.anahtar, 'data-tur': t.tur }, kartIcerigi(o, t)))),
    );
  }
  o.liste = await listeOku(o);
  doldurListe();
}

// ---------------------------------------------------------------- imza

/** İmza tuvali: çizgileri nokta dizisi olarak toplar (görüntü saklanmaz). `{oge, cizim(), bosalt()}` döner. */
function imzaTuvali(doc) {
  const { en, boy } = T.CIZIM_OLCU;
  const tuval = el(doc, 'canvas', { class: 'tutanak-imza-tuvali', 'data-alan': 'i-tuval', width: en, height: boy, role: 'img', 'aria-label': 'İmza kutusu: parmağınızla ya da fareyle imzanızı atın' });
  const cizer = typeof tuval.getContext === 'function' ? tuval.getContext('2d') : null;
  let cizgiler = [];
  let etkin = null;
  const nokta = (olay) => {
    // Tuvalin ekrandaki boyutu ile iç ölçüsü farklı olabilir: iç ölçüye çevir.
    const k = typeof tuval.getBoundingClientRect === 'function' ? tuval.getBoundingClientRect() : null;
    if (k && k.width > 0 && k.height > 0 && Number.isFinite(olay.clientX)) {
      return [((olay.clientX - k.left) * en) / k.width, ((olay.clientY - k.top) * boy) / k.height];
    }
    return [Number(olay.offsetX) || 0, Number(olay.offsetY) || 0];
  };
  const ciz = (x0, y0, x1, y1) => {
    if (!cizer) return;
    cizer.lineWidth = 2.5;
    cizer.lineCap = 'round';
    cizer.strokeStyle = '#111';
    cizer.beginPath();
    cizer.moveTo(x0, y0);
    cizer.lineTo(x1, y1);
    cizer.stroke();
  };
  tuval.addEventListener('pointerdown', (olay) => {
    if (typeof olay.preventDefault === 'function') olay.preventDefault();
    if (typeof tuval.setPointerCapture === 'function' && olay.pointerId !== undefined) {
      try {
        tuval.setPointerCapture(olay.pointerId);
      } catch {
        /* yakalama olmadan da çizilir */
      }
    }
    etkin = nokta(olay);
    cizgiler.push([...etkin]);
  });
  tuval.addEventListener('pointermove', (olay) => {
    if (!etkin) return;
    const [x, y] = nokta(olay);
    const c = cizgiler[cizgiler.length - 1];
    ciz(c[c.length - 2], c[c.length - 1], x, y);
    c.push(x, y);
  });
  const bitir = () => {
    etkin = null;
  };
  tuval.addEventListener('pointerup', bitir);
  tuval.addEventListener('pointercancel', bitir);
  return {
    oge: tuval,
    cizim: () => ({ en, boy, cizgiler: cizgiler.map((c) => [...c]) }),
    bosalt: () => {
      cizgiler = [];
      etkin = null;
      if (cizer) cizer.clearRect(0, 0, en, boy);
    },
  };
}

async function imzaCiz(o) {
  const { doc } = o;
  o.imzaAcik = false;
  temizle(o.govde);
  const liste = (await listeOku(o)).filter((t) => T.eksikRoller(t, o.sablonlar.find((s) => s.kimlik === t.sablon)).length > 0);
  o.govde.appendChild(el(doc, 'p', { class: 'taa-soluk', text: 'İmzası eksik tutanaklar. İmzalayan kişi adını yazar ve kutuya imzasını atar; imza giden kutusuna alınır.' }));
  if (liste.length === 0) {
    o.govde.appendChild(el(doc, 'p', { 'data-bolge': 'bos', text: 'İmza bekleyen tutanak yok.' }));
    return;
  }
  const formBolgesi = el(doc, 'div', { 'data-bolge': 'imza-formu' });

  function formAc(t) {
    const sablon = o.sablonlar.find((s) => s.kimlik === t.sablon);
    const eksik = T.eksikRoller(t, sablon);
    const tuval = imzaTuvali(doc);
    const imtinaRolleri = sablon ? sablon.imzalar.filter((i) => i.imtinaKaydi).map((i) => i.rol) : [];
    o.imzaAcik = true;
    durumYaz(o.durum, 'bilgi');

    async function imzala() {
      if (!o.imzaYetkisi) return durumYaz(o.durum, 'hata', 'İmza yetkiniz yok.');
      const imtina = imtinaRolleri.length > 0 && bul(formBolgesi, 'i-imtina').checked === true;
      const hazir = T.imzaHazirla(
        t,
        { rol: bul(formBolgesi, 'i-rol').value, imzalayan: bul(formBolgesi, 'i-imzalayan').value, cizim: tuval.cizim(), imtina, imtinaNotu: imtina ? bul(formBolgesi, 'i-imtina-notu').value : '' },
        sablon,
      );
      if (hazir.hatalar.length) return durumYaz(o.durum, 'hata', hazir.hatalar);
      let sonuc;
      try {
        sonuc = await o.baglam.giden.ekle({ program: T.PROGRAM, uc: T.UC.imza, govde: hazir.govde });
      } catch (e) {
        return durumYaz(o.durum, 'hata', `İmza giden kutusuna yazılamadı: ${hataMetni(e)}`);
      }
      durumYaz(o.durum, 'tamam', `${hazir.govde.rol} imzası (${hazir.govde.imzalayan}) giden kutusuna alındı (no ${sonuc.no}); bağlantı gelince sunucuya gider.`);
      return imzaCiz(o);
    }

    doldur(
      formBolgesi,
      el(
        doc,
        'div',
        { class: 'taa-kutu', 'data-imza-hedef': t.anahtar },
        el(doc, 'strong', { text: `${t.no || 'Numara bekleniyor'} · ${sablonAdi(o, t.sablon)}` }),
        el(
          doc,
          'div',
          { class: 'taa-satir' },
          alan(doc, 'Rol', 'i-rol', { secenekler: eksik.map((r) => [r, r]), value: eksik[0] }),
          alan(doc, 'İmzalayanın adı soyadı', 'i-imzalayan', { maxlength: 80, autocomplete: 'off' }),
        ),
        el(doc, 'span', { class: 'taa-alan-ad', text: 'İmza' }),
        tuval.oge,
        imtinaRolleri.length > 0
          ? [
              el(doc, 'div', { class: 'tutanak-secenekler' }, el(doc, 'label', {}, el(doc, 'input', { type: 'checkbox', 'data-alan': 'i-imtina' }), el(doc, 'span', { text: `İmzadan imtina etti (yalnız: ${imtinaRolleri.join(', ')})` }))),
              alan(doc, 'İmtina notu (imtina işaretliyse zorunlu, 5-300 karakter)', 'i-imtina-notu', { maxlength: 300 }),
            ]
          : null,
        el(
          doc,
          'div',
          { class: 'taa-satir' },
          el(doc, 'button', { type: 'button', 'data-is': 'imza-temizle', text: 'İmzayı temizle', onclick: () => tuval.bosalt() }),
          el(doc, 'button', { type: 'button', 'data-is': 'imza-vazgec', text: 'Vazgeç', onclick: () => { o.imzaAcik = false; temizle(formBolgesi); } }),
          el(doc, 'button', { type: 'button', class: 'taa-ana', 'data-is': 'imza-at', text: 'İmzala', onclick: imzala }),
        ),
      ),
    );
  }

  o.govde.appendChild(
    el(
      doc,
      'ul',
      { class: 'taa-liste', 'data-bolge': 'imza-listesi' },
      liste.map((t) =>
        el(
          doc,
          'li',
          { class: 'taa-kutu', 'data-tutanak': t.anahtar },
          kartIcerigi(o, t),
          o.imzaYetkisi ? el(doc, 'button', { type: 'button', 'data-is': 'imza-ac', 'data-tutanak': t.anahtar, text: 'İmza al', onclick: () => formAc(t) }) : null,
        ),
      ),
    ),
  );
  o.govde.appendChild(formBolgesi);
}
