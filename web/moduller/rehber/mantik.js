// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// Rehber (web) saf kuralları: kişi / birim görünümü ve arama. Salt okunur; yazma yok.
// Kaynak: crates/taa-rehber-model/src/model.rs (KisiKaydi, BirimKaydi, GorevlendirmeKaydi, RolKaydi).
// Görünürlük (şube kapsamı, amir) sunucuda süzülür; istemci yalnız kendisine inen kopyayı gösterir.

import { eslesir, katla } from '../ortak/bicim.js';

export const PROGRAM = 'rehber';
export const IZIN = { oku: 'rehber.oku', tumOku: 'rehber.tum.oku', amirOku: 'rehber.amir.oku' };
export const ANAHTAR = { veri: 'rehber/veri' };

const dizi = (v) => (Array.isArray(v) ? v : v && typeof v === 'object' ? Object.values(v) : []);

/** Önbellekteki veriyi denetler: alanlar dizi ya da (Rust `BTreeMap` gibi) kimlik→kayıt nesnesi olabilir. */
export function veriDuzelt(ham) {
  const h = ham && typeof ham === 'object' ? ham : {};
  const gecerli = (k) => k && typeof k.id === 'string' && k.id !== '' && typeof k.ad === 'string';
  return {
    kisiler: dizi(h.kisiler).filter(gecerli),
    birimler: dizi(h.birimler).filter(gecerli),
    gorevlendirmeler: dizi(h.gorevlendirmeler).filter((g) => g && typeof g.kisi_id === 'string' && typeof g.birim_id === 'string'),
    roller: dizi(h.roller).filter((r) => r && typeof r.kod === 'string'),
  };
}

const etkin = (durum) => durum === undefined || durum === 'etkin';

/**
 * Kişi satırları: her kişi için etkin görevleri (birim adı, rol adı, görev, amir adı).
 * `secenek.pasifler`: pasif kişiler ve bitmiş görevler de gösterilsin mi (yalnız `rehber.tum.oku`).
 * `secenek.amir`: amir adı gösterilsin mi (yalnız `rehber.amir.oku`).
 */
export function kisiSatirlari(veri, secenek = {}) {
  const birim = new Map(veri.birimler.map((b) => [b.id, b]));
  const kisi = new Map(veri.kisiler.map((k) => [k.id, k]));
  const rol = new Map(veri.roller.map((r) => [r.kod, r.ad || r.kod]));
  return veri.kisiler
    .filter((k) => secenek.pasifler || etkin(k.durum))
    .map((k) => {
      const gorevler = veri.gorevlendirmeler
        .filter((g) => g.kisi_id === k.id && (secenek.pasifler || etkin(g.durum)))
        .map((g) => ({
          birim_id: g.birim_id,
          birim: (birim.get(g.birim_id) || { ad: g.birim_id }).ad,
          rol: g.rol ? rol.get(g.rol) || g.rol : '',
          gorev: g.gorev || '',
          tur: g.tur || '',
          amir: secenek.amir && g.amir_id ? (kisi.get(g.amir_id) || { ad: '' }).ad : '',
        }));
      return { id: k.id, ad: k.ad, eposta: k.eposta || '', pasif: !etkin(k.durum), gorevler };
    })
    .sort((a, b) => a.ad.localeCompare(b.ad, 'tr'));
}

/** Birim satırları: üst birim adı ve etkin kişi sayısıyla. */
export function birimSatirlari(veri, secenek = {}) {
  const birim = new Map(veri.birimler.map((b) => [b.id, b]));
  const etkinKisi = new Set(veri.kisiler.filter((k) => etkin(k.durum)).map((k) => k.id));
  return veri.birimler
    .filter((b) => secenek.pasifler || etkin(b.durum))
    .map((b) => ({
      id: b.id,
      ad: b.ad,
      tur: b.tur || '',
      ust: b.ust_birim_id ? (birim.get(b.ust_birim_id) || { ad: '' }).ad : '',
      sube_kodu: b.sube_kodu || '',
      ortak_eposta: b.ortak_eposta || '',
      pasif: !etkin(b.durum),
      kisi_sayisi: new Set(veri.gorevlendirmeler.filter((g) => g.birim_id === b.id && etkin(g.durum) && etkinKisi.has(g.kisi_id)).map((g) => g.kisi_id)).size,
    }))
    .sort((a, b) => a.ad.localeCompare(b.ad, 'tr'));
}

/** Kişi araması: ad, e-posta, birim, rol, görev. Türkçe harf ve büyük/küçük farkı gözetilmez. */
export function kisiAra(satirlar, sorgu) {
  if (!katla(sorgu)) return satirlar;
  return satirlar.filter((k) => eslesir([k.ad, k.eposta, ...k.gorevler.flatMap((g) => [g.birim, g.rol, g.gorev])].join(' '), sorgu));
}

/** Birim araması: ad, tür, üst birim, şube kodu, ortak e-posta. */
export function birimAra(satirlar, sorgu) {
  if (!katla(sorgu)) return satirlar;
  return satirlar.filter((b) => eslesir([b.ad, b.tur, b.ust, b.sube_kodu, b.ortak_eposta].join(' '), sorgu));
}
