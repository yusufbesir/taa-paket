// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// TerziTakip web modülü: fiş durumu listesi (işlemdeki / geciken / bugün teslim / kargo), arama, durum değiştirme.
// Liste baglam.onbellek'ten okunur (çevrimdışı çalışır); durum değişikliği baglam.giden.ekle ile yazılır.
// Yeni fiş yazma ve yazdırma bu modülde yok (yazdırma köprüsü W2). Kurallar: ./mantik.js.

import { el, alan, bul, doldur, temizle, stilEkle, durumBolgesi, durumYaz, onbellekOku, gidenListe, gidenDegisinceYenile } from '../ortak/dom.js';
import { paraYaz, bugun, tarihYaz } from '../ortak/bicim.js';
import * as T from './mantik.js';

export const ad = 'terzitakip';
export const baslik = 'TerziTakip';

export async function ac(kok, baglam) {
  const doc = kok.ownerDocument;
  temizle(kok);
  stilEkle(kok);
  if (!baglam.yetki(T.IZIN.oku)) {
    kok.appendChild(el(doc, 'p', { 'data-bolge': 'yetkisiz', text: 'TerziTakip: bu ekranı açma yetkiniz yok. Yöneticinize başvurun.' }));
    return;
  }
  const durumYetkisi = baglam.yetki(T.IZIN.durum);
  const iptalYetkisi = durumYetkisi && baglam.yetki(T.IZIN.iptal);
  const durum = durumBolgesi(doc);
  const ozetBolgesi = el(doc, 'div', { class: 'taa-satir', 'data-bolge': 'ozet' });
  const listeBolgesi = el(doc, 'div', { 'data-bolge': 'liste' });
  let suzgec = 'acik';
  let fisler = [];
  /** Onay bekleyen geri alınamaz geçiş: "<no>|<durum>". */
  let onayBekleyen = '';

  const suzgecDugmeleri = T.SUZGECLER.map(([kod, yazi]) =>
    el(doc, 'button', {
      type: 'button',
      'data-suzgec': kod,
      'aria-pressed': kod === suzgec ? 'true' : 'false',
      text: yazi,
      onclick: () => {
        suzgec = kod;
        for (const d of suzgecDugmeleri) d.setAttribute('aria-pressed', d.getAttribute('data-suzgec') === kod ? 'true' : 'false');
        ciz();
      },
    }),
  );

  async function yukle() {
    const [ham, kayitlar] = await Promise.all([onbellekOku(baglam, T.ANAHTAR.fisler), gidenListe(baglam, T.PROGRAM)]);
    fisler = T.bekleyenleriIsle(T.fisleriDuzelt(ham), kayitlar);
    return ham !== undefined;
  }

  async function degistir(f, yeni) {
    const anahtar = `${f.no}|${yeni}`;
    if (T.ONAY_ISTEYEN.includes(yeni) && onayBekleyen !== anahtar) {
      onayBekleyen = anahtar;
      durumYaz(durum, 'uyari', `${f.no}: "${T.durumAdi(yeni)}" geri alınamaz. Onaylamak için aynı düğmeye bir kez daha basın.`);
      ciz();
      return;
    }
    onayBekleyen = '';
    if (!durumYetkisi || (yeni === 'iptal' && !iptalYetkisi)) {
      durumYaz(durum, 'hata', 'Bu işlem için yetkiniz yok.');
      return;
    }
    const hazir = T.durumHazirla(f, yeni, bugun());
    if (hazir.hatalar.length) {
      durumYaz(durum, 'hata', hazir.hatalar);
      return;
    }
    try {
      const s = await baglam.giden.ekle({ program: T.PROGRAM, uc: T.UC_DURUM, govde: hazir.govde });
      durumYaz(durum, 'tamam', `${f.no}: ${T.durumAdi(yeni)}. Giden kutusuna alındı (no ${s.no}); bağlantı gelince sunucuya gider.`);
    } catch (e) {
      durumYaz(durum, 'hata', `Değişiklik giden kutusuna yazılamadı: ${e && e.message ? e.message : e}`);
      return;
    }
    await yukle();
    ciz();
  }

  function kart(f, gun) {
    const gecikti = T.acik(f) && f.teslim < gun;
    const dugmeler = durumYetkisi && !f.bekleyen_durum
      ? T.gecisler(f.durum)
          .filter((y) => y !== 'iptal' || iptalYetkisi)
          .map((y) =>
            el(doc, 'button', {
              type: 'button',
              'data-is': 'durum',
              'data-fis': f.no,
              'data-yeni': y,
              text: onayBekleyen === `${f.no}|${y}` ? `Onayla: ${T.durumAdi(y)}` : y === 'iptal' ? 'İptal et' : `→ ${T.durumAdi(y)}`,
              onclick: () => degistir(f, y),
            }),
          )
      : [];
    return el(
      doc,
      'li',
      { class: 'taa-kutu', 'data-fis': f.no, 'data-durum': f.durum },
      el(
        doc,
        'div',
        { class: 'taa-satir' },
        el(doc, 'strong', { text: f.no }),
        el(doc, 'span', { class: 'taa-rozet', text: T.durumAdi(f.durum) }),
        f.bekleyen_durum ? el(doc, 'span', { class: 'taa-rozet', 'data-tur': 'uyari', text: 'gönderilmeyi bekliyor' }) : null,
        f.reddedilen_durum ? el(doc, 'span', { class: 'taa-rozet', 'data-tur': 'hata', text: `sunucu reddetti: ${T.durumAdi(f.reddedilen_durum)}` }) : null,
        gecikti ? el(doc, 'span', { class: 'taa-rozet', 'data-tur': 'hata', 'data-rozet': 'gecikti', text: '▲ gecikti' }) : null,
        f.kargo ? el(doc, 'span', { class: 'taa-rozet', 'data-tur': 'uyari', 'data-rozet': 'kargo', text: 'KARGO' }) : null,
      ),
      el(doc, 'span', { text: `${f.musteri || ''}${f.telefon ? ` · ${f.telefon}` : ''}` }),
      el(doc, 'span', { text: `Teslim: ${tarihYaz(f.teslim)}${f.teslim_saati ? ` ${f.teslim_saati}` : ''}${f.usta ? ` · Usta: ${f.usta}` : ''}${f.raf ? ` · Raf: ${f.raf}` : ''}` }),
      f.kalemler.length ? el(doc, 'span', { class: 'taa-soluk', text: f.kalemler.map((k) => `${k.urun}: ${k.islem}`).join('; ') }) : null,
      Number.isSafeInteger(f.ucret_kurus) && Number.isSafeInteger(f.on_odeme_kurus) ? el(doc, 'span', { class: 'taa-soluk', text: `Kalan: ${paraYaz(f.ucret_kurus - f.on_odeme_kurus)}` }) : null,
      f.kargo ? el(doc, 'span', { class: 'taa-soluk', text: `Kargo: ${[f.kargo_ad, f.kargo_adres, f.kargo_telefon, f.kargo_notu].filter(Boolean).join(' · ')}` }) : null,
      dugmeler.length ? el(doc, 'div', { class: 'taa-satir' }, dugmeler) : null,
    );
  }

  function ciz() {
    const gun = bugun();
    const o = T.ozet(fisler, gun);
    doldur(
      ozetBolgesi,
      [
        ['bekliyor', 'Bekliyor'],
        ['terzihanede', 'Terzihanede'],
        ['hazir', 'Hazır'],
        ['geciken', 'Geciken'],
        ['bugun_teslim', 'Bugün teslim'],
        ['kargo_bekleyen', 'Kargo bekleyen'],
      ].map(([k, yazi]) => el(doc, 'span', { class: 'taa-rozet', 'data-ozet': k, text: `${yazi}: ${o[k]}` })),
    );
    const l = T.suz(fisler, suzgec, bul(kok, 'fis-ara').value, gun);
    doldur(
      listeBolgesi,
      l.length === 0
        ? el(doc, 'p', { 'data-bolge': 'bos', text: fisler.length === 0 ? 'Bu cihazda fiş yok. Bir kez çevrimiçi açıp eşitleyin.' : 'Bu süzgece uyan fiş yok.' })
        : el(doc, 'ul', { class: 'taa-liste' }, l.map((f) => kart(f, gun))),
    );
  }

  kok.appendChild(el(doc, 'h2', { text: baslik }));
  kok.appendChild(ozetBolgesi);
  kok.appendChild(el(doc, 'div', { class: 'taa-sekmeler', role: 'group', 'aria-label': 'Fiş süzgeci' }, suzgecDugmeleri));
  kok.appendChild(alan(doc, 'Ara (fiş no, müşteri, telefon, usta, raf, işlem)', 'fis-ara', { type: 'search', oninput: () => ciz() }));
  if (!durumYetkisi) kok.appendChild(el(doc, 'p', { class: 'taa-soluk', 'data-bolge': 'salt-okunur', text: 'Durum değiştirme yetkiniz yok; liste salt okunur.' }));
  kok.appendChild(durum);
  kok.appendChild(listeBolgesi);
  gidenDegisinceYenile(kok, async () => {
    await yukle();
    ciz();
  });
  await yukle();
  ciz();
}
