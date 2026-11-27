// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// Rehber web modülü: kişi ve birim listesi, arama. Salt okunur ve tam çevrimdışı: yalnız baglam.onbellek okunur,
// giden kutusuna hiçbir şey yazılmaz, ağ çağrısı yapılmaz. Kurallar: ./mantik.js.

import { el, alan, bul, doldur, temizle, stilEkle, onbellekOku } from '../ortak/dom.js';
import * as R from './mantik.js';

export const ad = 'rehber';
export const baslik = 'Rehber';

export async function ac(kok, baglam) {
  const doc = kok.ownerDocument;
  temizle(kok);
  stilEkle(kok);
  if (!baglam.yetki(R.IZIN.oku)) {
    kok.appendChild(el(doc, 'p', { 'data-bolge': 'yetkisiz', text: 'Rehber: bu ekranı açma yetkiniz yok. Yöneticinize başvurun.' }));
    return;
  }
  const secenek = { pasifler: baglam.yetki(R.IZIN.tumOku), amir: baglam.yetki(R.IZIN.amirOku) };
  const ham = await onbellekOku(baglam, R.ANAHTAR.veri);
  const veri = R.veriDuzelt(ham);
  const kisiler = R.kisiSatirlari(veri, secenek);
  const birimler = R.birimSatirlari(veri, secenek);
  let gorunum = 'kisi';
  const liste = el(doc, 'div', { 'data-bolge': 'liste', 'aria-live': 'polite' });

  const dugmeler = [
    ['kisi', `Kişiler (${kisiler.length})`],
    ['birim', `Birimler (${birimler.length})`],
  ].map(([kod, yazi]) =>
    el(doc, 'button', {
      type: 'button',
      'data-gorunum': kod,
      'aria-pressed': kod === gorunum ? 'true' : 'false',
      text: yazi,
      onclick: () => {
        gorunum = kod;
        for (const d of dugmeler) d.setAttribute('aria-pressed', d.getAttribute('data-gorunum') === kod ? 'true' : 'false');
        ciz();
      },
    }),
  );

  function kisiKarti(k) {
    return el(
      doc,
      'li',
      { class: 'taa-kutu', 'data-kisi': k.id },
      el(doc, 'div', { class: 'taa-satir' }, el(doc, 'strong', { text: k.ad }), k.pasif ? el(doc, 'span', { class: 'taa-rozet', 'data-tur': 'uyari', text: 'pasif' }) : null),
      k.eposta ? el(doc, 'a', { href: `mailto:${encodeURIComponent(k.eposta).replace(/%40/g, '@')}`, text: k.eposta }) : null,
      k.gorevler.map((g) =>
        el(doc, 'span', { class: 'taa-soluk', text: [g.birim, g.rol, g.gorev, g.tur === 'vekil' ? 'vekil' : g.tur === 'gecici' ? 'geçici' : '', g.amir ? `amir: ${g.amir}` : ''].filter(Boolean).join(' · ') }),
      ),
    );
  }

  function birimKarti(b) {
    return el(
      doc,
      'li',
      { class: 'taa-kutu', 'data-birim': b.id },
      el(doc, 'div', { class: 'taa-satir' }, el(doc, 'strong', { text: b.ad }), b.pasif ? el(doc, 'span', { class: 'taa-rozet', 'data-tur': 'uyari', text: 'pasif' }) : null),
      el(doc, 'span', { class: 'taa-soluk', text: [b.tur, b.ust ? `üst birim: ${b.ust}` : '', b.sube_kodu ? `şube: ${b.sube_kodu}` : '', `${b.kisi_sayisi} kişi`].filter(Boolean).join(' · ') }),
      b.ortak_eposta ? el(doc, 'a', { href: `mailto:${encodeURIComponent(b.ortak_eposta).replace(/%40/g, '@')}`, text: b.ortak_eposta }) : null,
    );
  }

  function ciz() {
    const sorgu = bul(kok, 'rehber-ara').value;
    const l = gorunum === 'kisi' ? R.kisiAra(kisiler, sorgu) : R.birimAra(birimler, sorgu);
    doldur(
      liste,
      el(doc, 'p', { class: 'taa-soluk', 'data-bolge': 'sayi', text: `${l.length} kayıt` }),
      l.length === 0
        ? el(doc, 'p', { 'data-bolge': 'bos', text: ham === undefined ? 'Rehber bu cihaza henüz inmedi. Bir kez çevrimiçi açıp eşitleyin.' : 'Aramaya uyan kayıt yok.' })
        : el(doc, 'ul', { class: 'taa-liste' }, l.map(gorunum === 'kisi' ? kisiKarti : birimKarti)),
    );
  }

  kok.appendChild(el(doc, 'h2', { text: baslik }));
  kok.appendChild(el(doc, 'p', { class: 'taa-soluk', text: 'Salt okunur. Bu cihazdaki kopya gösterilir; internet olmadan da çalışır.' }));
  kok.appendChild(el(doc, 'div', { class: 'taa-sekmeler', role: 'group', 'aria-label': 'Rehber görünümü' }, dugmeler));
  kok.appendChild(alan(doc, 'Ara (ad, e-posta, birim, rol, görev)', 'rehber-ara', { type: 'search', oninput: () => ciz() }));
  kok.appendChild(liste);
  ciz();
}
