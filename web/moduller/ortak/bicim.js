// Sahip: Yusuf. yazan: [Claude Fable 5.2, TAA Web W1 modüller 2026-11-27]
// Ortak biçim yardımcıları: para (kuruş, tam sayı), tarih (YYYY-AA-GG), Türkçe arama katlaması.
// Saf işlevler; DOM ve ağ yok.

/** Tutar üst sınırı (10 milyar TL), crates/taa-kasa `TUTAR_EN_COK` ile aynı. */
export const TUTAR_EN_COK = 1_000_000_000_000;

/** Kuruşu "1.234,56 TL" biçiminde yazar. Tam sayı olmayan değer için "?" döner. */
export function paraYaz(kurus) {
  if (!Number.isSafeInteger(kurus)) return '?';
  const eksi = kurus < 0;
  const m = Math.abs(kurus);
  const tam = String(Math.trunc(m / 100)).replace(/\B(?=(\d{3})+(?!\d))/g, '.');
  const krs = String(m % 100).padStart(2, '0');
  return `${eksi ? '−' : ''}${tam},${krs} TL`;
}

/**
 * "1.234,56", "1234,5", "1234", "12.50", "-45" gibi yazımı kuruşa çevirir.
 * Boş metin 0'dır. Çözülemeyen yazım `null` döner (çağıran hata gösterir; tahmin edilmez).
 */
export function paraCoz(metin, secenek = {}) {
  let s = String(metin ?? '').replace(/\s|TL|₺/gi, '');
  if (s === '') return 0;
  let eksi = false;
  if (s[0] === '-' || s[0] === '−') {
    eksi = true;
    s = s.slice(1);
  } else if (s[0] === '+') {
    s = s.slice(1);
  }
  let tam;
  let krs = '';
  if (s.includes(',')) {
    const p = s.split(',');
    if (p.length !== 2) return null;
    tam = p[0];
    krs = p[1];
    if (!/^\d{1,3}(\.\d{3})*$/.test(tam) && !/^\d+$/.test(tam)) return null;
    tam = tam.replace(/\./g, '');
  } else if (/^\d{1,3}(\.\d{3})+$/.test(s)) {
    tam = s.replace(/\./g, '');
  } else if (/^\d+\.\d{1,2}$/.test(s)) {
    [tam, krs] = s.split('.');
  } else {
    tam = s;
  }
  if (!/^\d+$/.test(tam) || !/^\d{0,2}$/.test(krs)) return null;
  const k = Number(tam) * 100 + Number(krs.padEnd(2, '0'));
  if (!Number.isSafeInteger(k) || k > TUTAR_EN_COK) return null;
  if (eksi && !secenek.eksi && k !== 0) return null;
  return eksi ? -k : k;
}

/** Negatif olmayan tam sayı adet ("" = 0); geçersizse `null`. */
export function adetCoz(metin) {
  const s = String(metin ?? '').trim();
  if (s === '') return 0;
  if (!/^\d{1,9}$/.test(s)) return null;
  return Number(s);
}

/** "YYYY-AA-GG" gerçek bir takvim günü mü? */
export function tarihGecerli(t) {
  if (typeof t !== 'string' || !/^\d{4}-\d{2}-\d{2}$/.test(t)) return false;
  const [y, a, g] = t.split('-').map(Number);
  if (y < 2000 || y > 2100) return false;
  const d = new Date(Date.UTC(y, a - 1, g));
  return d.getUTCFullYear() === y && d.getUTCMonth() === a - 1 && d.getUTCDate() === g;
}

/** Türkiye saatiyle (UTC+3, yaz saati yok) bugünün tarihi. */
export function bugun(simdiMs = Date.now()) {
  return new Date(simdiMs + 3 * 3600 * 1000).toISOString().slice(0, 10);
}

/** "2026-11-27" → "27.11.2026" */
export function tarihYaz(t) {
  return tarihGecerli(t) ? `${t.slice(8, 10)}.${t.slice(5, 7)}.${t.slice(0, 4)}` : String(t ?? '');
}

const KATLA = { ı: 'i', İ: 'i', I: 'i', ş: 's', Ş: 's', ğ: 'g', Ğ: 'g', ü: 'u', Ü: 'u', ö: 'o', Ö: 'o', ç: 'c', Ç: 'c', â: 'a', Â: 'a', î: 'i', Î: 'i', û: 'u', Û: 'u' };

/** Arama için katlama: Türkçe harfler sadeleşir, küçük harfe iner, boşluklar teke düşer. */
export function katla(metin) {
  return String(metin ?? '')
    .replace(/[ıİIşŞğĞüÜöÖçÇâÂîÎûÛ]/g, (h) => KATLA[h])
    .toLowerCase()
    .replace(/\s+/g, ' ')
    .trim();
}

/** Sorgunun her sözcüğü samanlıkta geçiyor mu (ikisi de katlanır)? Boş sorgu her şeyi eşler. */
export function eslesir(samanlik, sorgu) {
  const s = katla(samanlik);
  return katla(sorgu)
    .split(' ')
    .filter(Boolean)
    .every((p) => s.includes(p));
}
