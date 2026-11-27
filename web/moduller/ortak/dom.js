// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// Ortak DOM yardımcıları. Modüller DOM'u yalnız bunlarla kurar: innerHTML hiç kullanılmaz,
// sunucudan/önbellekten gelen her metin textContent ile yazılır (kaçış sorunu doğmaz).

const OZELLIK_ALANLARI = new Set(['checked', 'disabled', 'hidden', 'selected', 'readOnly']);

/**
 * Öğe kurar. `oz`: `text` → textContent; `onX` (işlev) → olay dinleyicisi; `value`/`checked`/`disabled`/`hidden`
 * → özellik; diğerleri → setAttribute. `value` çocuklar eklendikten sonra atanır (select için gerekli).
 */
export function el(doc, etiket, oz = {}, ...cocuklar) {
  const d = doc.createElement(etiket);
  for (const [k, v] of Object.entries(oz || {})) {
    if (v === null || v === undefined || k === 'value') continue;
    if (k === 'text') d.textContent = String(v);
    else if (k.startsWith('on') && typeof v === 'function') d.addEventListener(k.slice(2).toLowerCase(), v);
    else if (OZELLIK_ALANLARI.has(k)) d[k] = Boolean(v);
    else if (v === false) continue;
    else d.setAttribute(k, v === true ? '' : String(v));
  }
  for (const c of cocuklar.flat(Infinity)) {
    if (c === null || c === undefined || c === false) continue;
    d.appendChild(typeof c === 'object' ? c : doc.createTextNode(String(c)));
  }
  if (oz && oz.value !== undefined && oz.value !== null) d.value = String(oz.value);
  return d;
}

/** Öğenin içini boşaltır. */
export function temizle(d) {
  while (d.firstChild) d.removeChild(d.firstChild);
}

/** İçeriği verilen düğümlerle değiştirir. */
export function doldur(d, ...cocuklar) {
  temizle(d);
  const doc = d.ownerDocument;
  for (const c of cocuklar.flat(Infinity)) {
    if (c === null || c === undefined || c === false) continue;
    d.appendChild(typeof c === 'object' ? c : doc.createTextNode(String(c)));
  }
}

/** Etiketli giriş alanı. Giriş `data-alan="<ad>"` taşır. */
export function alan(doc, etiket, ad, oz = {}) {
  const { secenekler, cokSatir, ...kalan } = oz;
  let giris;
  if (secenekler) {
    giris = el(
      doc,
      'select',
      { 'data-alan': ad, ...kalan },
      secenekler.map(([deger, yazi]) => el(doc, 'option', { value: deger, text: yazi })),
    );
  } else if (cokSatir) {
    giris = el(doc, 'textarea', { 'data-alan': ad, rows: 2, ...kalan });
  } else {
    giris = el(doc, 'input', { 'data-alan': ad, type: 'text', autocomplete: 'off', ...kalan });
  }
  return el(doc, 'label', { class: 'taa-alan' }, el(doc, 'span', { class: 'taa-alan-ad', text: etiket }), giris);
}

/** Para alanı: telefon klavyesinde sayı tuşları açılır. */
export function paraAlani(doc, etiket, ad, oz = {}) {
  return alan(doc, etiket, ad, { inputmode: 'decimal', placeholder: '0,00', ...oz });
}

/** Durum satırı (ekran okuyucuya duyurulur). `tur`: bilgi | tamam | uyari | hata */
export function durumYaz(bolge, tur, ...satirlar) {
  const doc = bolge.ownerDocument;
  bolge.setAttribute('data-durum', tur);
  const liste = satirlar.flat(Infinity).filter(Boolean);
  if (liste.length === 0) return temizle(bolge);
  return doldur(bolge, liste.length > 1 ? el(doc, 'ul', {}, liste.map((s) => el(doc, 'li', { text: s }))) : liste[0] || '');
}

export function durumBolgesi(doc) {
  return el(doc, 'div', { class: 'taa-durum', 'data-bolge': 'durum', role: 'status', 'aria-live': 'polite' });
}

/** Alanı bulur; yoksa hata fırlatır (yanlış ad sessizce geçmesin). */
export function bul(kok, ad) {
  const d = kok.querySelector(`[data-alan="${ad}"]`);
  if (!d) throw new Error(`alan yok: ${ad}`);
  return d;
}

/**
 * Önbellekten okur. Sözleşme `oku(anahtar)` der; çekirdek değeri düz ya da `{deger, surum}` sarmalıyla
 * dönebilir diye ikisi de kabul edilir. Kayıt yoksa `undefined`.
 */
export async function onbellekOku(baglam, anahtar) {
  const v = await baglam.onbellek.oku(anahtar);
  if (v && typeof v === 'object' && !Array.isArray(v) && 'deger' in v && 'surum' in v) return v.deger;
  return v ?? undefined;
}

/** Giden kutusundan bir programın kayıtlarını okur (durum verilmezse hepsi). */
export async function gidenListe(baglam, program, durum) {
  const l = (await baglam.giden.liste(durum)) || [];
  return l.filter((k) => k && k.program === program && (!durum || k.durum === durum));
}

/** 'taa-giden-degisti' olayını dinler; kök DOM'dan ayrılınca dinleyici kendini kaldırır. */
export function gidenDegisinceYenile(kok, yenile) {
  const hedef = kok.ownerDocument?.defaultView || globalThis;
  if (!hedef || typeof hedef.addEventListener !== 'function') return () => {};
  const dinle = () => {
    if (kok.isConnected === false) {
      hedef.removeEventListener('taa-giden-degisti', dinle);
      return;
    }
    Promise.resolve()
      .then(yenile)
      .catch(() => {});
  };
  hedef.addEventListener('taa-giden-degisti', dinle);
  return () => hedef.removeEventListener('taa-giden-degisti', dinle);
}

/**
 * Modül biçemini (./modul.css) köke bağlar ve köke `taa-modul` sınıfını verir. Satır içi <style> ya da style
 * özniteliği kullanılmaz: sıkı içerik güvenlik politikasında (style-src 'self') da çalışsın diye.
 */
export function stilEkle(kok) {
  const doc = kok.ownerDocument;
  // Kabuğun köke verdiği sınıflar korunur.
  if (kok.classList) kok.classList.add('taa-modul');
  else kok.setAttribute('class', 'taa-modul');
  kok.appendChild(el(doc, 'link', { rel: 'stylesheet', 'data-taa-stil': '', href: new URL('./modul.css', import.meta.url).href }));
}
