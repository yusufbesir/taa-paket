// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// TerziTakip (web) saf kuralları: fiş durumu, geçiş izni, geciken / kargo / bugün teslim süzgeçleri, arama.
// Kaynak: crates/taa-fis/src/lib.rs (Durum, Fis), defter.rs `durum_ata` (geçiş kuralı), crates/taa-takip/src/lib.rs
// (geciken, kargo_bekleyen, bugun_teslim, ozet). Buradaki denetim ön denetimdir; asıl karar sunucudadır.

import { tarihGecerli, eslesir } from '../ortak/bicim.js';

export const PROGRAM = 'terzitakip';
/** Sunucu ucu: docs/WEB-ESITLE-SOZLESMESI.md (taa-sunucu'da uygulanacak). Ad varsayımdır, tek yerden değişir. */
export const UC_DURUM = '/v1/fis/durum';
export const SOZLESME = 'fis.guncelle@1';

/** İzin adları (baglam.yetki). Sunucudaki adlar kesinleşince yalnız burası değişir. */
export const IZIN = { oku: 'fis.oku', durum: 'fis.guncelle', iptal: 'fis.iptal' };

export const ANAHTAR = { fisler: 'terzitakip/fisler' };

export const DURUMLAR = [
  ['bekliyor', 'Bekliyor'],
  ['terzihanede', 'Terzihanede'],
  ['hazir', 'Hazır'],
  ['teslim', 'Teslim edildi'],
  ['iptal', 'İptal'],
];
export const durumAdi = (d) => (DURUMLAR.find(([k]) => k === d) || [d, d])[1];

const GECISLER = {
  bekliyor: ['terzihanede', 'hazir', 'iptal'],
  terzihanede: ['hazir', 'iptal'],
  hazir: ['teslim', 'iptal'],
  teslim: [],
  iptal: [],
};

/** Geri alınamayan geçişler: ikinci kez basılarak onaylanır. */
export const ONAY_ISTEYEN = ['teslim', 'iptal'];

/** `eski` durumundan geçilebilecek durumlar (geri dönüş yok). */
export function gecisler(eski) {
  return GECISLER[eski] ? [...GECISLER[eski]] : [];
}

export function gecisIzinli(eski, yeni) {
  return gecisler(eski).includes(yeni);
}

/** İşlemdeki fiş: bekliyor, terzihanede ya da hazır. */
export const acik = (f) => f.durum === 'bekliyor' || f.durum === 'terzihanede' || f.durum === 'hazir';

export const geciken = (fisler, bugun) => fisler.filter((f) => acik(f) && String(f.teslim) < bugun);
export const bugunTeslim = (fisler, bugun) => fisler.filter((f) => acik(f) && f.teslim === bugun);
export const kargoBekleyen = (fisler) => fisler.filter((f) => f.kargo === true && acik(f));

export function ozet(fisler, bugun) {
  const say = (d) => fisler.filter((f) => f.durum === d).length;
  return {
    bekliyor: say('bekliyor'),
    terzihanede: say('terzihanede'),
    hazir: say('hazir'),
    geciken: geciken(fisler, bugun).length,
    bugun_teslim: bugunTeslim(fisler, bugun).length,
    kargo_bekleyen: kargoBekleyen(fisler).length,
  };
}

export const SUZGECLER = [
  ['acik', 'İşlemdekiler'],
  ['geciken', 'Geciken'],
  ['bugun', 'Bugün teslim'],
  ['kargo', 'Kargo bekleyen'],
  ['hazir', 'Hazır'],
  ['hepsi', 'Hepsi'],
];

/** Süzgeç + arama (fiş no, müşteri, telefon, usta, raf, kalemler). Teslim tarihine göre sıralar. */
export function suz(fisler, suzgec, sorgu, bugun) {
  let l;
  if (suzgec === 'geciken') l = geciken(fisler, bugun);
  else if (suzgec === 'bugun') l = bugunTeslim(fisler, bugun);
  else if (suzgec === 'kargo') l = kargoBekleyen(fisler);
  else if (suzgec === 'hazir') l = fisler.filter((f) => f.durum === 'hazir');
  else if (suzgec === 'hepsi') l = [...fisler];
  else l = fisler.filter(acik);
  if (String(sorgu ?? '').trim()) {
    l = l.filter((f) =>
      eslesir(
        [f.no, f.musteri, f.telefon, f.usta, f.raf, f.kargo_ad, ...(Array.isArray(f.kalemler) ? f.kalemler.map((k) => `${k.urun} ${k.islem}`) : [])].join(' '),
        sorgu,
      ),
    );
  }
  return l.sort((a, b) => (a.teslim < b.teslim ? -1 : a.teslim > b.teslim ? 1 : String(a.no).localeCompare(String(b.no))));
}

/** Önbellekten gelen listeyi denetler: `[...]` ya da `{fisler:[...]}`; `no` ve `durum` taşımayan satır atılır. */
export function fisleriDuzelt(ham) {
  const l = Array.isArray(ham) ? ham : ham && Array.isArray(ham.fisler) ? ham.fisler : [];
  return l
    .filter((f) => f && typeof f.no === 'string' && f.no !== '' && DURUMLAR.some(([k]) => k === f.durum))
    .map((f) => ({ ...f, teslim: String(f.teslim || ''), kalemler: Array.isArray(f.kalemler) ? f.kalemler : [] }));
}

/**
 * Giden kutusunda bekleyen durum değişikliklerini listeye işler (ekran, gönderilmemiş değişikliği de göstersin).
 * Reddedilenler işlenmez; fişe `bekleyen_durum` / `reddedilen_durum` işareti konur.
 */
export function bekleyenleriIsle(fisler, kayitlar) {
  const harita = new Map(fisler.map((f) => [f.no, { ...f }]));
  for (const k of kayitlar || []) {
    const g = k.govde;
    if (!g || k.uc !== UC_DURUM) continue;
    const f = harita.get(g.fis_no);
    if (!f) continue;
    if (k.durum === 'bekliyor' && gecisIzinli(f.durum, g.durum)) {
      f.durum = g.durum;
      f.bekleyen_durum = true;
    } else if (k.durum === 'reddedildi') {
      f.reddedilen_durum = g.durum;
    }
  }
  return [...harita.values()];
}

/** Durum değişikliği gövdesini doğrular ve hazırlar. `{hatalar, govde}`. */
export function durumHazirla(fis, yeni, tarih) {
  const hatalar = [];
  if (!fis || !fis.no) hatalar.push('fiş seçilmedi');
  else if (!DURUMLAR.some(([k]) => k === yeni)) hatalar.push(`bilinmeyen durum: ${yeni}`);
  else if (!gecisIzinli(fis.durum, yeni)) hatalar.push(`${durumAdi(fis.durum)} durumundan ${durumAdi(yeni)} durumuna geçilemez`);
  if (!tarihGecerli(tarih)) hatalar.push('tarih YYYY-AA-GG olmalı');
  if (hatalar.length) return { hatalar };
  return { hatalar, govde: { sozlesme: SOZLESME, fis_no: fis.no, durum: yeni, onceki_durum: fis.durum, tarih } };
}
