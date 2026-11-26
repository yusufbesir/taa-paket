// Müşteri takip sayfası: SEÇİLEN tasarımın (A+) örnek HTML'lerini üretir. Yalnız tasarım denemesidir; üretimdeki sayfa
// ortak/takip.mjs'dedir (onu başka bir oturum yazıyor). Veriler uydurmadır. Kullanım: node uret.mjs <çıktı klasörü>
// Sayfada istemci JavaScript yoktur; dil çekmecesi <details>, adımlar <ol>. Yazan: [Claude#terzitakip-tasarim 2026-11-26].
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const DILLER = [['tr', 'Türkçe'], ['en', 'English'], ['ar', 'العربية'], ['ru', 'Русский'], ['de', 'Deutsch'], ['fr', 'Français'],
  ['es', 'Español'], ['zh', '中文'], ['hi', 'हिन्दी'], ['fa', 'فارسی'], ['ro', 'Română'], ['el', 'Ελληνικά'], ['bg', 'Български']];

const M = {
  tr: { dir: 'ltr', durum: 'Ürününüz hazır', alt: 'Tadilatınız tamamlandı.', adimlar: ['Alındı', 'Terzide', 'Hazır', 'Teslim edildi'],
    guncel: 'Güncel', not: "Ürününüzü fişinizle birlikte saat 17:00'ye kadar teslim alabilirsiniz. Hazır olduktan sonra 14 gün saklanır.",
    ozet: 'Fiş özeti', fis: 'Fiş tarihi', teslim: 'Teslim', fisT: '26 Kasım 2026', teslimT: '5 Aralık 2026, Cumartesi · 17:00',
    kalemler: 'Tadilat kalemleri', k: [['Pantolon', 'Paça kısaltma', 'düz paça'], ['Ceket', 'Kol boyu kısaltma', '']],
    sube: 'Şube', ayd: 'Aydınlatma metni', dil: 'Dil seçimi' },
  en: { dir: 'ltr', durum: 'Your item is ready', alt: 'Your alteration is complete.', adimlar: ['Received', 'In progress', 'Ready', 'Delivered'],
    guncel: 'Current', not: 'You can pick up your item with your receipt until 17:00. It is kept for 14 days after it is ready.',
    ozet: 'Receipt summary', fis: 'Receipt date', teslim: 'Pick-up', fisT: '26 November 2026', teslimT: 'Saturday, 5 December 2026 · 17:00',
    kalemler: 'Alteration items', k: [['Trousers', 'Hem shortening', 'straight hem'], ['Jacket', 'Sleeve shortening', '']],
    sube: 'Branch', ayd: 'Privacy notice', dil: 'Language' },
  ar: { dir: 'rtl', durum: 'منتجك جاهز', alt: 'تم الانتهاء من التعديل.', adimlar: ['تم الاستلام', 'قيد الخياطة', 'جاهز', 'تم التسليم'],
    guncel: 'الحالية', not: 'يمكنك استلام منتجك مع الإيصال حتى الساعة 17:00. يُحفظ لمدة 14 يومًا بعد أن يصبح جاهزًا.',
    ozet: 'ملخص الإيصال', fis: 'تاريخ الإيصال', teslim: 'موعد التسليم', fisT: '26 نوفمبر 2026', teslimT: 'السبت 5 ديسمبر 2026 · 17:00',
    kalemler: 'بنود التعديل', k: [['بنطلون', 'تقصير الأرجل', 'ساق مستقيمة'], ['سترة', 'تقصير الكم', '']],
    sube: 'الفرع', ayd: 'إشعار الخصوصية', dil: 'اللغة' },
};

const CSS = `
:root{color-scheme:light dark;--zemin:#f6f3ee;--kart:#ffffff;--yazi:#29251f;--ince:#665c50;--vurgu:#8a5a2b;--vurgu-yazi:#ffffff;--cizgi:#e4ddd2;--not:#efe8dc}
@media (prefers-color-scheme:dark){:root{--zemin:#14181c;--kart:#1d2329;--yazi:#e8ecf3;--ince:#b7c0cc;--vurgu:#d7a46b;--vurgu-yazi:#14181c;--cizgi:#323a43;--not:#262d34}}
*{box-sizing:border-box}
body{margin:0;background:var(--zemin);color:var(--yazi);font:16px/1.5 system-ui,-apple-system,"Segoe UI",Roboto,"Noto Sans",sans-serif;-webkit-text-size-adjust:100%}
.sayfa{max-width:480px;margin:0 auto;padding:16px}
header{display:flex;align-items:center;justify-content:space-between;gap:12px;min-height:48px;position:relative}
h1.marka{margin:0;font-size:20px;font-weight:700;letter-spacing:.2px}
details.dil{position:relative}
details.dil>summary{list-style:none;display:flex;align-items:center;gap:6px;min-height:48px;min-width:64px;padding:0 14px;border:1px solid var(--cizgi);border-radius:12px;background:var(--kart);font-weight:700;cursor:pointer;user-select:none}
details.dil>summary::-webkit-details-marker{display:none}
details.dil>summary::after{content:"";width:8px;height:8px;border-inline-end:2px solid currentColor;border-bottom:2px solid currentColor;transform:rotate(45deg) translateY(-2px)}
details.dil[open]>summary::after{transform:rotate(-135deg) translateY(-2px)}
details.dil>summary:focus-visible,a:focus-visible{outline:3px solid var(--vurgu);outline-offset:2px}
.dil-liste{position:absolute;inset-inline-end:0;top:calc(100% + 6px);z-index:5;width:min(320px,calc(100vw - 32px));max-height:70dvh;overflow-y:auto;margin:0;padding:6px;list-style:none;background:var(--kart);border:1px solid var(--cizgi);border-radius:12px;box-shadow:0 10px 30px rgba(0,0,0,.18)}
.dil-liste a{display:flex;align-items:center;gap:12px;min-height:48px;padding:0 12px;border-radius:8px;color:inherit;text-decoration:none}
.dil-liste a[aria-current="true"]{background:var(--not);font-weight:700}
.dil-liste b{min-width:2.2em;font:700 15px ui-monospace,Menlo,Consolas,monospace;color:var(--vurgu)}.dil-liste span{color:var(--ince)}
.kart{margin-top:16px;padding:20px 16px;background:var(--kart);border:1px solid var(--cizgi);border-radius:12px}
.hero{display:flex;gap:14px;align-items:center}
.tik{flex:none;width:48px;height:48px;border-radius:50%;background:var(--vurgu);color:var(--vurgu-yazi);display:grid;place-items:center}
h2{margin:0;font-size:24px;line-height:1.25}
.alt{margin:2px 0 0;color:var(--ince);font-size:14px}
ol.adimlar{list-style:none;margin:20px 0 0;padding:0}
ol.adimlar li{position:relative;display:flex;align-items:center;gap:14px;min-height:48px;padding-inline-start:0}
ol.adimlar li::before{content:"";flex:none;width:28px;height:28px;border-radius:50%;border:2px solid var(--cizgi);background:var(--kart)}
ol.adimlar li+li::after{content:"";position:absolute;inset-inline-start:13px;top:-14px;height:28px;width:2px;background:var(--cizgi)}
ol.adimlar li.bitti::before{background:var(--vurgu);border-color:var(--vurgu)}
ol.adimlar li.bitti+li::after,ol.adimlar li.su::after{background:var(--vurgu)}
ol.adimlar li.su::before{border-color:var(--vurgu);box-shadow:inset 0 0 0 6px var(--vurgu)}
ol.adimlar li.sonra{color:var(--ince)}
.etiket{margin-inline-start:auto;padding:2px 10px;border-radius:999px;background:var(--not);color:var(--vurgu);font-size:13px;font-weight:700}
.su{font-weight:700;font-size:18px}
.not{margin:16px 0 0;padding:12px 14px;border-inline-start:4px solid var(--vurgu);border-radius:8px;background:var(--not);font-size:15px}
h3{margin:0 0 12px;font-size:16px}
dl{margin:0}dl div{display:flex;justify-content:space-between;gap:16px;padding:10px 0;border-bottom:1px solid var(--cizgi)}
dt{color:var(--ince)}dd{margin:0;font-weight:700;text-align:end}
ul.kalemler{list-style:none;margin:16px 0 0;padding:0;display:grid;gap:8px}
ul.kalemler li{padding:12px 14px;border-radius:8px;background:var(--not)}
ul.kalemler b{display:block}ul.kalemler span{color:var(--ince);font-size:14px}
footer{display:flex;justify-content:space-between;align-items:center;gap:12px;margin-top:20px;padding:12px 0;color:var(--ince);font-size:14px}
footer a{color:inherit;display:inline-flex;align-items:center;min-height:48px}
@media (prefers-reduced-motion:no-preference){details.dil>summary::after{transition:transform .15s}}
`;

function sayfa(dil, ac) {
  const t = M[dil];
  const liste = DILLER.map(([k, ad]) => `<li><a href="?dil=${k}" lang="${k}" hreflang="${k}" dir="ltr"${k === dil ? ' aria-current="true"' : ''}><b>${k.toUpperCase()}</b><span aria-hidden="true">=</span><bdi lang="${k}">${ad}</bdi></a></li>`).join('');
  const adim = (i, sinif) => `<li class="${sinif}"${sinif === 'su' ? ' aria-current="step"' : ''}>${t.adimlar[i]}${sinif === 'su' ? `<span class="etiket">${t.guncel}</span>` : ''}</li>`;
  const kalem = t.k.map(([u, i, v]) => `<li><b>${u}</b><span>${i}${v ? ` · ${v}` : ''}</span></li>`).join('');
  return `<!doctype html>
<html lang="${dil}" dir="${t.dir}"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1">
<meta name="color-scheme" content="light dark"><title>Terzi Adem Altun</title><style>${CSS}</style></head>
<body><div class="sayfa">
<header><h1 class="marka">Terzi Adem Altun</h1>
<details class="dil"${ac ? ' open' : ''}><summary aria-label="${t.dil}: ${DILLER.find((d) => d[0] === dil)[1]}">${dil.toUpperCase()}</summary><ul class="dil-liste">${liste}</ul></details></header>
<main>
<section class="kart" aria-labelledby="d"><div class="hero"><div class="tik" aria-hidden="true"><svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg></div>
<div><h2 id="d">${t.durum}</h2><p class="alt">${t.alt}</p></div></div>
<ol class="adimlar" aria-label="${t.durum}">${adim(0, 'bitti')}${adim(1, 'bitti')}${adim(2, 'su')}${adim(3, 'sonra')}</ol>
<p class="not">${t.not}</p></section>
<section class="kart" aria-labelledby="o"><h3 id="o">${t.ozet}</h3>
<dl><div><dt>${t.fis}</dt><dd>${t.fisT}</dd></div><div><dt>${t.teslim}</dt><dd>${t.teslimT}</dd></div></dl>
<ul class="kalemler" aria-label="${t.kalemler}">${kalem}</ul></section>
</main>
<footer><span>${t.sube} MRK01</span><a href="/aydinlatma">${t.ayd}</a></footer>
</div></body></html>`;
}

const cikti = process.argv[2] || '.';
mkdirSync(cikti, { recursive: true });
for (const [ad, dil, ac] of [['secilen-tr', 'tr', false], ['secilen-tr-cekmece', 'tr', true], ['secilen-en', 'en', false], ['secilen-ar-cekmece', 'ar', true], ['secilen-ar', 'ar', false]]) {
  writeFileSync(join(cikti, `${ad}.html`), sayfa(dil, ac));
}
console.log('üretildi:', cikti);
