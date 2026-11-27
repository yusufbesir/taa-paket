// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// TAA Kasa web modülü: günlük sayım, masraf, gün sonu (canlı fark paneli, fark notu zorunlu) ve imza
// (çift doğrulama). Yazmalar yalnız baglam.giden.ekle ile, okumalar yalnız baglam.onbellek ve giden listesiyle.
// Kurallar: ./mantik.js. Silme yok; düzeltme ve iptal yönetici işidir (bu modülde yok).

import { el, alan, paraAlani, bul, doldur, temizle, stilEkle, durumBolgesi, durumYaz, onbellekOku, gidenListe, gidenDegisinceYenile } from '../ortak/dom.js';
import { paraYaz, paraCoz, adetCoz, bugun, tarihYaz } from '../ortak/bicim.js';
import * as K from './mantik.js';

export const ad = 'kasa';
export const baslik = 'TAA Kasa';

const SEKMELER = [
  ['sayim', 'Sayım', K.IZIN.sayim],
  ['masraf', 'Masraf', K.IZIN.masraf],
  ['gunsonu', 'Gün sonu', K.IZIN.gunsonu],
  ['imza', 'İmza', K.IZIN.imza],
];

/** Modülü `kok` içine kurar. İlk çizim bitince çözülen söz döner. */
export async function ac(kok, baglam) {
  const doc = kok.ownerDocument;
  temizle(kok);
  stilEkle(kok);
  const acik = SEKMELER.filter(([, , izin]) => baglam.yetki(izin));
  if (acik.length === 0) {
    kok.appendChild(el(doc, 'p', { 'data-bolge': 'yetkisiz', text: 'TAA Kasa: bu ekranı açma yetkiniz yok. Yöneticinize başvurun.' }));
    return;
  }
  const ayar = K.ayarDuzelt(await onbellekOku(baglam, K.ANAHTAR.ayar));
  kok.appendChild(el(doc, 'h2', { text: baslik }));
  if (!ayar) {
    kok.appendChild(
      el(doc, 'p', {
        'data-bolge': 'ayar-yok',
        text: 'Mağaza kasa ayarı (çekmeceler, kasa noktaları) bu cihaza henüz inmedi. Bir kez çevrimiçi açıp eşitleyin; ayar gelmeden kayıt alınmaz.',
      }),
    );
    return;
  }
  const durum = durumBolgesi(doc);
  const govde = el(doc, 'div', { 'data-bolge': 'govde' });
  const ortam = { doc, baglam, ayar, durum, govde, girenAd: baglam.kullanici && baglam.kullanici.ad ? String(baglam.kullanici.ad) : '' };
  let secili = acik[0][0];
  const dugmeler = acik.map(([kod, yazi]) =>
    el(doc, 'button', { type: 'button', 'data-sekme': kod, 'aria-pressed': 'false', text: yazi, onclick: () => sec(kod) }),
  );
  async function sec(kod) {
    secili = kod;
    for (const d of dugmeler) d.setAttribute('aria-pressed', d.getAttribute('data-sekme') === kod ? 'true' : 'false');
    durumYaz(durum, 'bilgi');
    temizle(govde);
    await CIZ[kod](ortam);
  }
  kok.appendChild(el(doc, 'div', { class: 'taa-sekmeler', role: 'group', 'aria-label': 'Kasa bölümleri' }, dugmeler));
  kok.appendChild(durum);
  kok.appendChild(govde);
  // İmza listesi giden kutusuna bağlıdır; kutu değişince (gönderildi/reddedildi) yenilenir.
  gidenDegisinceYenile(kok, () => (secili === 'imza' ? imzaCiz(ortam) : undefined));
  await sec(secili);
}

const CIZ = { sayim: sayimCiz, masraf: masrafCiz, gunsonu: gunSonuCiz, imza: imzaCiz };

function tarihCekmece(o, onek) {
  return [
    alan(o.doc, 'Tarih', `${onek}-tarih`, { type: 'date', value: bugun() }),
    alan(o.doc, 'Çekmece', `${onek}-cekmece`, { secenekler: o.ayar.cekmeceler.map((c) => [c.id, c.ad]), value: o.ayar.cekmeceler[0].id }),
  ];
}

function kupurAlanlari(o, onek) {
  return el(
    o.doc,
    'div',
    { class: 'taa-satir' },
    K.DEGERLER.map((d) => alan(o.doc, paraYaz(d), `${onek}-adet-${d}`, { inputmode: 'numeric', placeholder: 'adet' })),
  );
}

/** Kupür adetlerini okur; `{adetler}` ya da `{hata}`. */
function kupurOku(kok, onek) {
  const adetler = {};
  for (const d of K.DEGERLER) {
    const a = adetCoz(bul(kok, `${onek}-adet-${d}`).value);
    if (a === null) return { hata: `${paraYaz(d)} adedi geçersiz (yalnız rakam)` };
    if (a > 0) adetler[d] = a;
  }
  return { adetler };
}

/** Para alanını okur; geçersizse `hatalar`a yazar ve 0 döner. */
function paraOku(kok, adi, etiket, hatalar, eksi = false) {
  const k = paraCoz(bul(kok, adi).value, { eksi });
  if (k === null) {
    hatalar.push(`${etiket}: tutar okunamadı (ör. 1.234,56)`);
    return 0;
  }
  return k;
}

async function gunOzetiOku(o, tarih, cekmece) {
  const [onb, bekleyen] = await Promise.all([onbellekOku(o.baglam, K.ANAHTAR.gun(tarih, cekmece)), gidenListe(o.baglam, K.PROGRAM, 'bekliyor')]);
  return K.gunOzeti(onb, bekleyen, tarih, cekmece);
}

async function kaydet(o, uc, hazir, basari) {
  if (hazir.hatalar.length) {
    durumYaz(o.durum, 'hata', hazir.hatalar);
    return null;
  }
  try {
    const s = await o.baglam.giden.ekle({ program: K.PROGRAM, uc, govde: hazir.govde });
    durumYaz(o.durum, 'tamam', `${basari} Giden kutusuna alındı (no ${s.no}); bağlantı gelince sunucuya gider.`);
    return s;
  } catch (e) {
    durumYaz(o.durum, 'hata', `Kayıt giden kutusuna yazılamadı: ${e && e.message ? e.message : e}`);
    return null;
  }
}

// ---------------------------------------------------------------- sayım

async function sayimCiz(o) {
  const { doc, govde } = o;
  const bilgi = el(doc, 'p', { class: 'taa-soluk', 'data-bolge': 'sayim-bilgi' });
  const yenile = async () => {
    const t = bul(govde, 'sayim-tarih').value;
    const c = bul(govde, 'sayim-cekmece').value;
    const tur = bul(govde, 'sayim-tur').value;
    const oz = await gunOzetiOku(o, t, c);
    const okuma = kupurOku(govde, 'sayim');
    const toplam = okuma.hata ? null : K.sayimToplam(okuma.adetler).deger;
    const beklenen = beklenenNakit(tur, oz);
    bilgi.textContent =
      `Sayılan: ${toplam === null ? '?' : paraYaz(toplam)}. ` +
      (beklenen === null ? 'Beklenen tutar bu cihazda bilinmiyor (fark sunucuda hesaplanır).' : `Beklenen: ${paraYaz(beklenen)}. Fark: ${toplam === null ? '?' : paraYaz(toplam - beklenen)}.`);
    return { t, c, tur, oz, okuma, beklenen };
  };
  doldur(
    govde,
    el(doc, 'h3', { text: 'Sayım' }),
    el(doc, 'div', { class: 'taa-satir', onchange: yenile }, tarihCekmece(o, 'sayim'), alan(doc, 'Sayım türü', 'sayim-tur', { secenekler: K.SAYIM_TURLERI, value: 'acilis' })),
    el(doc, 'div', { oninput: yenile }, kupurAlanlari(o, 'sayim')),
    bilgi,
    el(doc, 'div', { class: 'taa-satir' }, alan(doc, 'İkinci kişi (ara sayım ve devirde zorunlu)', 'sayim-ikinci'), alan(doc, 'Not (fark varsa zorunlu)', 'sayim-not', { cokSatir: true, maxlength: 300 })),
    el(doc, 'button', {
      type: 'button',
      class: 'taa-ana',
      'data-is': 'sayim-kaydet',
      text: 'Sayımı kaydet',
      onclick: async () => {
        const { t, c, tur, oz, okuma, beklenen } = await yenile();
        if (okuma.hata) return durumYaz(o.durum, 'hata', okuma.hata);
        if (tur === 'acilis' && oz.acilisVar) return durumYaz(o.durum, 'hata', 'bu gün ve çekmece için açılış zaten var; düzeltme yönetici girişiyle yapılır');
        const hazir = K.sayimHazirla(o.ayar, tur, { tarih: t, cekmece: c, adetler: okuma.adetler, ikinci_kisi: bul(govde, 'sayim-ikinci').value, not: bul(govde, 'sayim-not').value }, { beklenenKurus: beklenen, girenAd: o.girenAd });
        const s = await kaydet(o, K.UC_GUNSONU, hazir, 'Sayım kaydedildi.');
        if (s) await sayimCiz(o);
        return undefined;
      },
    }),
  );
  await yenile();
}

function beklenenNakit(tur, oz) {
  if (tur === 'acilis') return oz.oncekiDevredenKurus;
  return oz.acilisKurus === null ? null : oz.acilisKurus - oz.masrafKurus;
}

// ---------------------------------------------------------------- masraf

async function masrafCiz(o) {
  const { doc, govde, ayar } = o;
  const liste = el(doc, 'div', { 'data-bolge': 'masraf-bekleyen' });
  const listeYenile = async () => {
    const b = (await gidenListe(o.baglam, K.PROGRAM)).filter((k) => k.govde && k.govde.tur === 'masraf' && k.durum !== 'gonderildi');
    doldur(
      liste,
      b.length === 0
        ? el(doc, 'p', { class: 'taa-soluk', text: 'Bu cihazda gönderilmeyi bekleyen masraf yok.' })
        : el(
            doc,
            'table',
            { class: 'taa-tablo' },
            el(doc, 'caption', { text: 'Bu cihazda bekleyen masraflar' }),
            el(doc, 'thead', {}, el(doc, 'tr', {}, ['Tarih', 'Tür', 'Kalem', 'Tutar', 'Onay', 'Durum'].map((t) => el(doc, 'th', { scope: 'col', text: t })))),
            el(
              doc,
              'tbody',
              {},
              b.map((k) =>
                el(doc, 'tr', {}, [
                  el(doc, 'td', { text: tarihYaz(k.govde.v.tarih) }),
                  el(doc, 'td', { text: k.govde.v.tur }),
                  el(doc, 'td', { text: k.govde.v.aciklama }),
                  el(doc, 'td', { class: 'taa-sayi', text: paraYaz(k.govde.v.tutar_kurus) }),
                  el(doc, 'td', { text: k.govde.v.onay_duzeyi === 'merkez' ? 'Merkez (yönetici)' : 'Mağaza müdürü' }),
                  el(doc, 'td', { text: k.durum === 'reddedildi' ? 'Sunucu reddetti' : 'Bekliyor' }),
                ]),
              ),
            ),
          ),
    );
  };
  doldur(
    govde,
    el(doc, 'h3', { text: 'Masraf' }),
    el(doc, 'div', { class: 'taa-satir' }, tarihCekmece(o, 'masraf')),
    el(
      doc,
      'div',
      { class: 'taa-satir' },
      alan(doc, 'Tür (listeden seçin ya da yazın)', 'masraf-tur', { list: 'taa-kasa-masraf-turleri', maxlength: K.MASRAF_TURU_EN_UZUN }),
      el(doc, 'datalist', { id: 'taa-kasa-masraf-turleri' }, ayar.masraf_turleri.map((t) => el(doc, 'option', { value: t }))),
      paraAlani(doc, 'Tutar', 'masraf-tutar'),
    ),
    el(doc, 'div', { class: 'taa-satir' }, alan(doc, 'Kalem (ör. Ekmek)', 'masraf-aciklama'), alan(doc, 'Not (nereye / kime; isteğe bağlı)', 'masraf-not', { maxlength: 200 })),
    el(doc, 'p', { class: 'taa-soluk', text: `${paraYaz(ayar.masraf_merkez_esigi_kurus)} ve üstünü merkez (yönetici), altını mağaza müdürü onaylar. Giren onaylayamaz.` }),
    el(doc, 'button', {
      type: 'button',
      class: 'taa-ana',
      'data-is': 'masraf-kaydet',
      text: 'Masrafı kaydet',
      onclick: async () => {
        const hatalar = [];
        const tutar = paraOku(govde, 'masraf-tutar', 'Tutar', hatalar);
        if (hatalar.length) return durumYaz(o.durum, 'hata', hatalar);
        const hazir = K.masrafHazirla(ayar, {
          tarih: bul(govde, 'masraf-tarih').value,
          cekmece: bul(govde, 'masraf-cekmece').value,
          tur: bul(govde, 'masraf-tur').value,
          tutar_kurus: tutar,
          aciklama: bul(govde, 'masraf-aciklama').value,
          not: bul(govde, 'masraf-not').value,
        });
        const s = await kaydet(o, K.UC_MASRAF, hazir, 'Masraf kaydedildi.');
        if (s) {
          for (const a of ['masraf-tutar', 'masraf-aciklama', 'masraf-not']) bul(govde, a).value = '';
          await listeYenile();
        }
        return undefined;
      },
    }),
    liste,
  );
  await listeYenile();
}

// ---------------------------------------------------------------- gün sonu

async function gunSonuCiz(o) {
  const { doc, govde, ayar } = o;
  const noktaBolgesi = el(doc, 'div', { 'data-bolge': 'noktalar' });
  const panel = el(doc, 'div', { class: 'taa-kutu', 'data-bolge': 'fark-paneli', 'aria-live': 'polite' });
  let ozet = { acilisKurus: null, masrafKurus: 0, gunSonuVar: false };

  const noktalariCiz = () => {
    const cek = K.cekmeceBul(ayar, bul(govde, 'gs-cekmece').value);
    doldur(
      noktaBolgesi,
      (cek ? cek.noktalar : []).map((n) =>
        el(doc, 'div', { class: 'taa-satir' }, paraAlani(doc, `${n.ad}: Z raporu toplamı`, `gs-z-${n.id}`), n.poslar.map((p) => paraAlani(doc, `${n.ad}: ${p.ad} gün sonu`, `gs-pos-${n.id}-${p.id}`))),
      ),
    );
  };

  /** Formu okur: `{girdi, hatalar}` (hatalar yalnız okunamayan yazımlar içindir). */
  const oku = () => {
    const hatalar = [];
    const cekId = bul(govde, 'gs-cekmece').value;
    const cek = K.cekmeceBul(ayar, cekId);
    const kupur = kupurOku(govde, 'gs');
    if (kupur.hata) hatalar.push(kupur.hata);
    const ciro = {};
    for (const [k, etiket] of K.ODEME_TURLERI) ciro[k] = paraOku(govde, `gs-ciro-${k}`, etiket, hatalar, k === 'kredi_ceki');
    for (const [k, etiket] of K.BILGI_GIRILEN) ciro[k] = paraOku(govde, `gs-ciro-${k}`, etiket, hatalar, true);
    const doviz = [];
    for (const [kod, adi] of K.DOVIZLER) {
      const tutar = paraOku(govde, `gs-doviz-${kod}`, `${adi} tutarı`, hatalar);
      const kurMetin = bul(govde, `gs-kur-${kod}`).value.trim().replace(',', '.');
      let kur = 0;
      if (kurMetin !== '') {
        if (/^\d{1,4}(\.\d{1,4})?$/.test(kurMetin)) kur = Math.round(Number(kurMetin) * 10000);
        else hatalar.push(`${adi} kuru okunamadı (ör. 42,1500)`);
      }
      if (tutar > 0 || kur > 0) doviz.push({ kod, tutar, kur_onbinde: kur });
    }
    const girdi = {
      tarih: bul(govde, 'gs-tarih').value,
      cekmece: cekId,
      nakit_adetler: kupur.adetler || {},
      noktalar: (cek ? cek.noktalar : []).map((n) => ({
        nokta: n.id,
        z_toplam_kurus: paraOku(govde, `gs-z-${n.id}`, `${n.ad} Z raporu`, hatalar),
        pos: Object.fromEntries(n.poslar.map((p) => [p.id, paraOku(govde, `gs-pos-${n.id}-${p.id}`, `${n.ad} ${p.ad}`, hatalar)])),
      })),
      nebim_toplam_kurus: paraOku(govde, 'gs-nebim', 'Nebim toplamı', hatalar),
      ciro,
      doviz,
      devreden_kurus: paraOku(govde, 'gs-devreden', 'Devreden nakit', hatalar),
      kontrol: K.KAPANIS_LISTESI.map((_, i) => Boolean(bul(govde, `gs-kontrol-${i}`).checked)),
      not: bul(govde, 'gs-not').value,
      aciklamalar: Object.fromEntries(K.ACIKLAMALI_ALANLAR.map((k) => [k, bul(govde, `gs-aciklama-${k}`).value])),
    };
    return { girdi, hatalar };
  };

  const paneliCiz = () => {
    const { girdi, hatalar } = oku();
    if (hatalar.length) return doldur(panel, el(doc, 'p', { text: 'Fark paneli: önce okunamayan alanları düzeltin.' }), el(doc, 'ul', {}, hatalar.map((h) => el(doc, 'li', { text: h }))));
    const nakit = K.sayimToplam(girdi.nakit_adetler).deger;
    let k;
    try {
      k = K.karsilastir(ayar, { ...girdi, doviz: girdi.doviz.filter((d) => d.tutar > 0 && d.kur_onbinde > 0) }, nakit, ozet.acilisKurus ?? 0, ozet.masrafKurus);
    } catch {
      return doldur(panel, el(doc, 'p', { text: 'Fark paneli hesaplanamadı; tutarları denetleyin.' }));
    }
    const tol = ayar.fark_toleransi_kurus;
    const cd = K.ciftDogrulama(ayar, { ...girdi, doviz: girdi.doviz.filter((d) => d.tutar > 0 && d.kur_onbinde > 0), nakit_sayilan_kurus: nakit, acilis_kurus: ozet.acilisKurus ?? 0 });
    return doldur(
      panel,
      el(doc, 'h3', { text: 'Canlı fark paneli' }),
      el(doc, 'p', { class: 'taa-soluk', text: `Sayılan nakit ${paraYaz(nakit)}; açılış ${ozet.acilisKurus === null ? 'bu cihazda bilinmiyor (0 sayıldı)' : paraYaz(ozet.acilisKurus)}; masraflar ${paraYaz(ozet.masrafKurus)}; genel toplam ${paraYaz(k.ciro_toplam_kurus)}.` }),
      el(
        doc,
        'table',
        { class: 'taa-tablo' },
        el(doc, 'thead', {}, el(doc, 'tr', {}, [el(doc, 'th', { scope: 'col', text: 'Karşılaştırma' }), ...['Beklenen', 'Bulunan', 'Fark'].map((t) => el(doc, 'th', { scope: 'col', class: 'taa-sayi', text: t }))])),
        el(
          doc,
          'tbody',
          {},
          k.satirlar.map((s) =>
            el(doc, 'tr', { 'data-fark': Math.abs(s.fark_kurus) > tol ? 'var' : 'yok' }, [
              el(doc, 'td', { text: s.ad }),
              el(doc, 'td', { class: 'taa-sayi', text: paraYaz(s.beklenen_kurus) }),
              el(doc, 'td', { class: 'taa-sayi', text: paraYaz(s.bulunan_kurus) }),
              el(doc, 'td', { class: 'taa-sayi' }, Math.abs(s.fark_kurus) > tol ? el(doc, 'span', { class: 'taa-rozet', 'data-tur': 'hata', text: `▲ ${paraYaz(s.fark_kurus)}` }) : el(doc, 'span', { class: 'taa-rozet', 'data-tur': 'tamam', text: '✓ tuttu' })),
            ]),
          ),
        ),
      ),
      k.kasa_ciro_uyusmuyor ? el(doc, 'p', { 'data-uyari': 'kasa-ciro', text: '⚠ Kasa ile ciro uyuşmuyor.' }) : null,
      k.fark_var ? el(doc, 'p', { 'data-uyari': 'fark-notu', text: 'Fark var: açıklama notu yazmadan gün sonu kaydedilmez.' }) : null,
      el(doc, 'h3', { text: 'Çift doğrulama tutanağı (ön izleme)' }),
      el(
        doc,
        'table',
        { class: 'taa-tablo', 'data-bolge': 'cift-dogrulama' },
        el(doc, 'thead', {}, el(doc, 'tr', {}, [el(doc, 'th', { scope: 'col', text: 'Kalem' }), ...['ERP (Nebim)', 'Fiili', 'Fark'].map((t) => el(doc, 'th', { scope: 'col', class: 'taa-sayi', text: t }))])),
        el(doc, 'tbody', {}, cd.kalemler.map((c) => el(doc, 'tr', {}, [el(doc, 'td', { text: c.ad }), ...[c.erp_kurus, c.fiili_kurus, c.fark_kurus].map((t) => el(doc, 'td', { class: 'taa-sayi', text: paraYaz(t) }))]))),
      ),
      el(doc, 'p', { 'data-bolge': 'cift-sonuc', text: `${cd.sonuc} Resmî tutanak (A4) TAA Kasa'da üretilir; bu tablo ön izlemedir.` }),
    );
  };

  const gunDegisti = async () => {
    ozet = await gunOzetiOku(o, bul(govde, 'gs-tarih').value, bul(govde, 'gs-cekmece').value);
    paneliCiz();
  };

  doldur(
    govde,
    el(doc, 'h3', { text: 'Gün sonu' }),
    el(
      doc,
      'div',
      { oninput: paneliCiz, onchange: paneliCiz },
      el(
        doc,
        'div',
        {
          class: 'taa-satir',
          onchange: async () => {
            noktalariCiz();
            await gunDegisti();
          },
        },
        tarihCekmece(o, 'gs'),
      ),
      el(doc, 'h3', { text: '1. Nakit sayımı (yalnız adet)' }),
      kupurAlanlari(o, 'gs'),
      paraAlani(doc, 'Ertesi güne devreden nakit (bozukluk)', 'gs-devreden'),
      el(doc, 'h3', { text: '2. Döviz' }),
      K.DOVIZLER.map(([kod, adi]) => el(doc, 'div', { class: 'taa-satir' }, paraAlani(doc, `${adi} (${kod}) tutarı`, `gs-doviz-${kod}`), alan(doc, `${kod} günlük kuru (TL)`, `gs-kur-${kod}`, { inputmode: 'decimal', placeholder: '0,0000' }))),
      el(doc, 'h3', { text: '3. Z raporu ve POS' }),
      noktaBolgesi,
      el(doc, 'h3', { text: '4. Satış sistemi (Nebim) ve ciro satırları' }),
      paraAlani(doc, 'Nebim mağaza raporu toplamı', 'gs-nebim'),
      el(doc, 'div', { class: 'taa-satir' }, K.ODEME_TURLERI.map(([k, etiket]) => paraAlani(doc, etiket, `gs-ciro-${k}`))),
      el(doc, 'p', { class: 'taa-soluk', text: 'Bilgi satırları (genel toplama girmez, eksi olabilir). Kasa fazlası / açığı elle girilmez: sayım farkından yazılır.' }),
      el(doc, 'div', { class: 'taa-satir' }, K.BILGI_GIRILEN.map(([k, etiket]) => paraAlani(doc, etiket, `gs-ciro-${k}`))),
      el(doc, 'div', { class: 'taa-satir' }, K.ACIKLAMALI_ALANLAR.map((k) => alan(doc, `${K.BILGI_ALANLARI.find(([a]) => a === k)[1]} açıklaması (tutar varsa zorunlu)`, `gs-aciklama-${k}`, { maxlength: 300 }))),
      el(doc, 'h3', { text: '5. Kapanış listesi ve not' }),
      K.KAPANIS_LISTESI.map((m, i) => el(doc, 'label', { class: 'taa-satir' }, el(doc, 'input', { type: 'checkbox', 'data-alan': `gs-kontrol-${i}` }), el(doc, 'span', { text: m }))),
      alan(doc, 'Not (karşılaştırmada fark varsa zorunlu)', 'gs-not', { cokSatir: true }),
    ),
    panel,
    el(doc, 'button', {
      type: 'button',
      class: 'taa-ana',
      'data-is': 'gunsonu-kaydet',
      text: 'Gün sonunu kaydet',
      onclick: async () => {
        const { girdi, hatalar } = oku();
        if (hatalar.length) return durumYaz(o.durum, 'hata', hatalar);
        ozet = await gunOzetiOku(o, girdi.tarih, girdi.cekmece);
        if (ozet.gunSonuVar) return durumYaz(o.durum, 'hata', 'bu gün ve çekmece için gün sonu zaten var; düzeltme yönetici girişiyle yapılır');
        const hazir = K.gunSonuHazirla(ayar, girdi, { acilisKurus: ozet.acilisKurus, masrafKurus: ozet.masrafKurus });
        const s = await kaydet(o, K.UC_GUNSONU, hazir, 'Gün sonu kaydedildi. İmza için "İmza" bölümüne geçin.');
        if (s) await gunDegisti();
        return undefined;
      },
    }),
  );
  noktalariCiz();
  await gunDegisti();
}

// ---------------------------------------------------------------- imza (çift doğrulama)

async function imzaCiz(o) {
  const { doc, govde } = o;
  const [onb, kayitlar] = await Promise.all([onbellekOku(o.baglam, K.ANAHTAR.gunSonlari), gidenListe(o.baglam, K.PROGRAM)]);
  const liste = K.imzaListesi(onb, kayitlar);
  const satir = (g) => {
    const anahtar = g.id || g.istemci_no;
    const imzaYazi = (sira) => {
      const i = g.imzalar.find((x) => x.sira === sira);
      if (!i) return sira === 2 && g.gerekce ? `${K.IKINCI_IMZA_ATMADI} (gerekçe: ${g.gerekce})` : 'bekliyor';
      return i.bekliyor ? 'imzalandı (gönderilmeyi bekliyor)' : `imzaladı: ${i.kisi || '?'}`;
    };
    const kontrolTamam = g.kontrol.length >= K.KAPANIS_LISTESI.length && g.kontrol.every(Boolean);
    const sira = g.imzalar.length + 1;
    return el(
      doc,
      'div',
      { class: 'taa-kutu', 'data-gun': anahtar },
      el(doc, 'strong', { text: `${tarihYaz(g.tarih)} · ${(K.cekmeceBul(o.ayar, g.cekmece) || { ad: g.cekmece }).ad}${g.bekliyor ? ' · gün sonu gönderilmeyi bekliyor' : ''}${g.gonderildi ? ' · merkeze gönderildi' : ''}` }),
      el(doc, 'span', { text: `1. imza (${K.IMZA_ROLLERI[0][1]}): ${imzaYazi(1)}` }),
      el(doc, 'span', { text: `2. imza (${K.IMZA_ROLLERI[1][1]}): ${imzaYazi(2)}` }),
      kontrolTamam ? null : el(doc, 'span', { 'data-uyari': 'kontrol', text: 'Kapanış listesi tamamlanmamış: imza açılmaz.' }),
      g.gonderildi || sira > 2
        ? null
        : el(doc, 'button', {
            type: 'button',
            class: 'taa-ana',
            'data-is': 'imzala',
            text: `${sira}. imzayı at (${K.IMZA_ROLLERI[sira - 1][1]})`,
            onclick: async () => {
              const s = await kaydet(o, K.UC_GUNSONU, K.imzaHazirla(g, o.girenAd), 'İmza kaydedildi.');
              if (s) await imzaCiz(o);
            },
          }),
      g.gonderildi || g.imzalar.length !== 1 || g.gerekce
        ? null
        : el(
            doc,
            'div',
            { class: 'taa-satir' },
            alan(doc, `İkinci imza atılmayacaksa gerekçe (${K.GEREKCE_EN_AZ}–${K.GEREKCE_EN_COK} karakter)`, `gerekce-${anahtar}`, { list: 'taa-kasa-gerekceler', maxlength: K.GEREKCE_EN_COK }),
            el(doc, 'button', {
              type: 'button',
              'data-is': 'gerekce-kaydet',
              text: 'Gerekçeyi kaydet',
              onclick: async () => {
                const s = await kaydet(o, K.UC_GUNSONU, K.gerekceHazirla(g, bul(govde, `gerekce-${anahtar}`).value), 'Gerekçe kaydedildi.');
                if (s) await imzaCiz(o);
              },
            }),
          ),
    );
  };
  doldur(
    govde,
    el(doc, 'h3', { text: 'İmza (çift doğrulama)' }),
    el(doc, 'p', { class: 'taa-soluk', text: 'Yalnız mağaza müdürü ve yönetici imzalar. İkinci imzayı gün sonunu giren atamaz; ikinci imza başka kişinin kendi oturumundan atılır (gün sonu sunucuya ulaştıktan sonra görünür).' }),
    el(doc, 'datalist', { id: 'taa-kasa-gerekceler' }, o.ayar.gerekce_sablonlari.map((t) => el(doc, 'option', { value: t }))),
    liste.length === 0 ? el(doc, 'p', { 'data-bolge': 'imza-bos', text: 'İmza bekleyen gün sonu yok.' }) : liste.map(satir),
  );
}
