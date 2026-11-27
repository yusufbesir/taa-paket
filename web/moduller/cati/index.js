// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// TAA Çatı web modülü (yönetici ekranı İSKELETİ). Yönetim izni olmayana hiçbir ekran, menü ya da veri gösterilmez
// ve önbellek okunmaz. İzni olan, yalnız iznine uyan ekranları görür. Yönetim işlemleri (cihaz onayı, kullanıcı
// ekleme, PIN, rapor) henüz taşınmadı: bu sürüm salt okunurdur ve giden kutusuna yazmaz. Kurallar: ./mantik.js.

import { el, doldur, temizle, stilEkle, onbellekOku, gidenDegisinceYenile } from '../ortak/dom.js';
import * as C from './mantik.js';

export const ad = 'cati';
export const baslik = 'TAA Çatı';

export async function ac(kok, baglam) {
  const doc = kok.ownerDocument;
  temizle(kok);
  const yetki = (i) => baglam.yetki(i);
  const ekranlar = C.gorunurEkranlar(yetki);
  if (ekranlar.length === 0) {
    // Yetkisiz: ekran adları dahil hiçbir yönetim bilgisi yazılmaz.
    stilEkle(kok);
    kok.appendChild(el(doc, 'p', { 'data-bolge': 'yetkisiz', text: 'Bu bölüm yalnız yöneticilere açıktır.' }));
    return;
  }
  stilEkle(kok);
  const govde = el(doc, 'div', { class: 'taa-kutu', 'data-bolge': 'ekran' });
  let secili = ekranlar[0].kod;
  const dugmeler = ekranlar.map((e) => el(doc, 'button', { type: 'button', 'data-ekran': e.kod, 'aria-pressed': 'false', text: e.ad, onclick: () => sec(e.kod) }));

  async function genelCiz() {
    const s = C.gidenSayilari(await baglam.giden.liste());
    doldur(
      govde,
      el(doc, 'h3', { text: 'Genel bakış' }),
      el(doc, 'p', { 'data-bolge': 'giden-sayilari', text: `Bu cihazın giden kutusu: bekleyen ${s.bekliyor}, gönderilen ${s.gonderildi}, sunucunun reddettiği ${s.reddedildi}.` }),
      s.reddedildi > 0 ? el(doc, 'p', { 'data-uyari': 'reddedilen', text: '⚠ Reddedilen kayıtlar yeniden denenmez; bir kişinin bakması gerekir.' }) : null,
      el(doc, 'p', { class: 'taa-soluk', text: `Açabildiğiniz ekranlar: ${ekranlar.map((e) => e.ad).join(', ')}.` }),
    );
  }

  async function ekranCiz(e) {
    const parcalar = [el(doc, 'h3', { text: e.ad }), el(doc, 'p', { class: 'taa-soluk', text: e.aciklama })];
    if (e.anahtar) {
      const ham = await onbellekOku(baglam, e.anahtar);
      const satirlar = C.tabloSatirlari(e, ham);
      parcalar.push(
        ham === undefined
          ? el(doc, 'p', { 'data-bolge': 'veri-yok', text: 'Bu liste bu cihaza henüz inmedi. Bir kez çevrimiçi açıp eşitleyin.' })
          : satirlar.length === 0
            ? el(doc, 'p', { 'data-bolge': 'bos', text: 'Kayıt yok.' })
            : el(
                doc,
                'table',
                { class: 'taa-tablo' },
                el(doc, 'thead', {}, el(doc, 'tr', {}, e.sutunlar.map(([, yazi]) => el(doc, 'th', { scope: 'col', text: yazi })))),
                el(doc, 'tbody', {}, satirlar.map((s) => el(doc, 'tr', {}, s.map((h) => el(doc, 'td', { text: h }))))),
              ),
      );
    }
    parcalar.push(el(doc, 'p', { 'data-bolge': 'iskelet', text: 'İskelet: bu ekranın işlemleri tarayıcıya henüz taşınmadı. İşlem için masaüstü TAA Çatı panelini kullanın.' }));
    doldur(govde, parcalar);
  }

  async function sec(kod) {
    // İzin her açılışta yeniden sorulur (yetki oturum içinde değişmiş olabilir); izin yoksa ekran açılmaz.
    const e = C.gorunurEkranlar(yetki).find((x) => x.kod === kod);
    if (!e) {
      doldur(govde, el(doc, 'p', { 'data-bolge': 'yetkisiz', text: 'Bu ekranı açma yetkiniz yok.' }));
      return;
    }
    secili = kod;
    for (const d of dugmeler) d.setAttribute('aria-pressed', d.getAttribute('data-ekran') === kod ? 'true' : 'false');
    if (kod === 'genel') await genelCiz();
    else await ekranCiz(e);
  }

  kok.appendChild(el(doc, 'h2', { text: baslik }));
  kok.appendChild(el(doc, 'nav', { class: 'taa-sekmeler', 'aria-label': 'Çatı ekranları' }, dugmeler));
  kok.appendChild(govde);
  gidenDegisinceYenile(kok, () => (secili === 'genel' ? genelCiz() : undefined));
  await sec(secili);
}
