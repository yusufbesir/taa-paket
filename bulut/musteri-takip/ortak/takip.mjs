// TAA müşteri takip: Cloudflare ve Vercel'in ORTAK kodu (doğrulama + sayfa).
// Mağaza sunucusu (taa-fis-sunucu, musteri_takip.rs) kaydı ed25519 ile imzalayıp POST /api/yaz'a iter.
// Kayıtta kişisel veri yoktur: {v, k: takip kodu, d: durum, s: şube kodu, t: teslim günü, ts, sil}.
// Bu dosya iki hedefe de aynen kopyalanır (tools/kopyala.sh); düzenlerken ikisini birlikte güncelleyin.
// Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].
// v2 (2026-11-26 03:42): çok dilli sayfa + fiş özeti (g: fiş günü, h: teslim saati, u: [{u: ürün, i: işlem kodu, v: varyant}]).
import { DILLER, SAG_SOL, YEREL, METIN, urunAdi, islemAdi, varyantAdi, dilSec } from "./diller.mjs";
export { dilSec };

export const KOD_RE = /^[a-km-np-z2-9]{16}$/;
const DURUMLAR = ["alindi", "terzide", "hazir", "teslim", "iptal"];
const GUN = 86400;

function hexBayt(h) {
  if (typeof h !== "string" || !/^([0-9a-f]{2})+$/i.test(h)) return null;
  const b = new Uint8Array(h.length / 2);
  for (let i = 0; i < b.length; i++) b[i] = parseInt(h.substr(i * 2, 2), 16);
  return b;
}

/** Gövdeyi ve imzayı doğrular; geçerliyse kaydı, değilse {hata, kod} döndürür. */
export async function dogrula(govde, imzaHex, genelHex, simdi = Math.floor(Date.now() / 1000)) {
  const genel = hexBayt(genelHex);
  const imza = hexBayt(imzaHex);
  if (!genel || genel.length !== 32) return { hata: "sunucu ayarı eksik", kod: 500 };
  if (!imza || imza.length !== 64) return { hata: "imza yok", kod: 401 };
  if (typeof govde !== "string" || govde.length > 1024) return { hata: "gövde geçersiz", kod: 400 };
  const anahtar = await crypto.subtle.importKey("raw", genel, { name: "Ed25519" }, false, ["verify"]);
  const tamam = await crypto.subtle.verify("Ed25519", anahtar, imza, new TextEncoder().encode(govde));
  if (!tamam) return { hata: "imza geçersiz", kod: 401 };
  let k;
  try {
    k = JSON.parse(govde);
  } catch {
    return { hata: "gövde geçersiz", kod: 400 };
  }
  const tamSayi = (x) => Number.isInteger(x) && x > 0;
  const anahtar_re = /^[a-z0-9-]{1,40}$/;
  const kalemlerGecerli = (u) =>
    Array.isArray(u) && u.length <= 20 &&
    u.every((x) => x && typeof x === "object" && anahtar_re.test(x.u) &&
      (x.i === undefined || anahtar_re.test(x.i)) && (x.v === undefined || x.v === "duz" || x.v === "duble"));
  if (k.v === 2) {
    if (
      typeof k.g !== "string" || !/^(\d{4}-\d{2}-\d{2})?$/.test(k.g) ||
      typeof k.h !== "string" || !/^(\d{2}:\d{2})?$/.test(k.h) ||
      !kalemlerGecerli(k.u)
    ) return { hata: "kayıt biçimi geçersiz", kod: 400 };
  }
  if (
    (k.v !== 1 && k.v !== 2) ||
    !KOD_RE.test(k.k) ||
    !DURUMLAR.includes(k.d) ||
    typeof k.s !== "string" || k.s.length > 16 ||
    typeof k.t !== "string" || !/^(\d{4}-\d{2}-\d{2})?$/.test(k.t) ||
    !tamSayi(k.ts) || !tamSayi(k.sil)
  ) {
    return { hata: "kayıt biçimi geçersiz", kod: 400 };
  }
  if (k.ts > simdi + 600) return { hata: "kayıt zamanı ileride", kod: 400 };
  if (k.ts < simdi - 30 * GUN) return { hata: "kayıt çok eski", kod: 400 };
  if (k.sil <= simdi || k.sil > simdi + 200 * GUN) return { hata: "silinme zamanı geçersiz", kod: 400 };
  // Yalnız bilinen alanlar saklanır.
  const kayit = { v: k.v, k: k.k, d: k.d, s: k.s, t: k.t, ts: k.ts, sil: k.sil };
  if (k.v === 2) {
    kayit.g = k.g;
    kayit.h = k.h;
    kayit.u = k.u.map((x) => ({ u: x.u, ...(x.i ? { i: x.i } : {}), ...(x.v ? { v: x.v } : {}) }));
  }
  return { kayit };
}

/** Eski kayıt yenisinin üzerine yazılmasın (tekrar oynatma / sıra karışması). */
export function yeniMi(eski, yeni) {
  return !eski || yeni.ts >= eski.ts;
}

const ADIMLAR = ["alindi", "terzide", "hazir", "teslim"];

function kacis(s) {
  return String(s).replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
}

function gunMetni(t, dil) {
  if (!t) return "";
  const [y, a, g] = t.split("-").map(Number);
  try {
    return new Intl.DateTimeFormat(YEREL[dil] || "tr-TR", {
      day: "numeric", month: "long", year: "numeric", weekday: "long", timeZone: "UTC",
    }).format(new Date(Date.UTC(y, a - 1, g)));
  } catch {
    return t;
  }
}

// Tasarım: bulut/musteri-takip/tasarim/KARAR.md (A+ "zanaat kartı", Stitch + Astra, 2026-11-26).
function dilCekmecesi(dil, adres) {
  if (!adres) return "";
  const [, ad, kisa] = DILLER.find(([d]) => d === dil);
  const lis = DILLER.map(([d, a, k]) =>
    `<li><a href="${kacis(adres)}?dil=${d}" lang="${d}" hreflang="${d}" dir="ltr"${d === dil ? ' aria-current="true"' : ""}><b>${k}</b><span aria-hidden="true">=</span><bdi lang="${d}">${a}</bdi></a></li>`,
  ).join("");
  // Betiksiz çekmece: <details>/<summary> klavye ve ekran okuyucuyla çalışır.
  return `<details class="dil"><summary aria-label="${kacis(METIN[dil].dil)}: ${kacis(ad)}">${kisa}</summary><ul class="dil-liste">${lis}</ul></details>`;
}

const SIMGE = {
  tik: '<path d="M5 12.5l4.5 4.5L19 7.5"/>',
  saat: '<circle cx="12" cy="12" r="8"/><path d="M12 8v4.5l3 2"/>',
  carpi: '<path d="M7 7l10 10M17 7L7 17"/>',
  soru: '<path d="M9.5 9a2.5 2.5 0 1 1 3.5 2.3c-.7.3-1 .9-1 1.7"/><path d="M12 17h.01"/>',
};
const simge = (ad) =>
  `<div class="tik" aria-hidden="true"><svg width="26" height="26" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round">${SIMGE[ad]}</svg></div>`;

function fisOzeti(kayit, dil, m) {
  if (kayit.v !== 2) return "";
  const satirlar = [];
  if (kayit.g) satirlar.push(`<div><dt>${m.fis_tarihi}</dt><dd>${kacis(gunMetni(kayit.g, dil))}</dd></div>`);
  if (kayit.t) {
    const saat = kayit.h ? ` · ${kacis(kayit.h)}` : "";
    satirlar.push(`<div><dt>${m.teslim_gunu}</dt><dd>${kacis(gunMetni(kayit.t, dil))}${saat}</dd></div>`);
  }
  const kalemler = (kayit.u || []).map((x) => {
    const islem = islemAdi(x.i, dil) || m.diger_islem;
    const varyant = varyantAdi(x.v, dil);
    return `<li><b>${kacis(urunAdi(x.u, dil))}</b><span>${kacis(islem)}${varyant ? ` · ${kacis(varyant)}` : ""}</span></li>`;
  }).join("");
  if (!satirlar.length && !kalemler) return "";
  return `<section class="kart" aria-labelledby="ozet"><h3 id="ozet">${m.ozet}</h3>${satirlar.length ? `<dl>${satirlar.join("")}</dl>` : ""}
    ${kalemler ? `<ul class="kalemler" aria-label="${m.islemler}">${kalemler}</ul>` : ""}</section>`;
}

/** Takip sayfası (sunucuda üretilir; istemci betiği yok). dil: DILLER'den biri; adres: dil bağlantıları için /t/<kod>. */
export function sayfa(kayit, { yedek = "", dil = "tr", adres = "" } = {}) {
  if (!METIN[dil]) dil = "tr";
  const m = METIN[dil];
  const tekKart = (ikon, baslik, metin, ek = "") =>
    `<section class="kart"><div class="hero">${simge(ikon)}<div><h2>${baslik}</h2></div></div><p class="aciklama">${metin}</p>${ek}</section>`;
  let govde;
  if (!kayit) {
    govde = tekKart("soru", m.yok_b, m.yok_m,
      yedek ? `<p class="alt">${m.yedek} <a href="${kacis(yedek)}?dil=${dil}">${m.yedek_a}</a></p>` : "");
  } else if (kayit.d === "iptal") {
    govde = tekKart("carpi", m.iptal_b, m.iptal_m);
  } else {
    const sira = ADIMLAR.indexOf(kayit.d);
    const adimlar = ADIMLAR.map((a, i) => {
      const s = i < sira ? "bitti" : i === sira ? "su" : "sonra";
      return `<li class="${s}"${s === "su" ? ' aria-current="step"' : ""}>${m.adim[a]}${s === "su" ? `<span class="etiket">${m.guncel}</span>` : ""}</li>`;
    }).join("");
    const tahmin = kayit.t && kayit.v !== 2 ? `${m.tahmini}: <b>${kacis(gunMetni(kayit.t, dil))}</b>` : "";
    const not = { alindi: tahmin, terzide: tahmin, hazir: m.hazir_m, teslim: m.teslim_m }[kayit.d];
    const ikon = kayit.d === "hazir" || kayit.d === "teslim" ? "tik" : "saat";
    govde = `<section class="kart" aria-labelledby="durum"><div class="hero">${simge(ikon)}
      <div><h2 id="durum">${m.durum[kayit.d]}</h2><p class="alt">${m.baslik}</p></div></div>
      <ol class="adimlar" aria-label="${m.baslik}">${adimlar}</ol>${not ? `<p class="not">${not}</p>` : ""}</section>${fisOzeti(kayit, dil, m)}`;
  }
  const yon = SAG_SOL.has(dil) ? "rtl" : "ltr";
  const sube = kayit && kayit.s ? `<footer><span>${m.sube} ${kacis(kayit.s)}</span></footer>` : "";
  return `<!doctype html><html lang="${dil}" dir="${yon}"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex,nofollow">
<meta name="referrer" content="no-referrer"><meta name="color-scheme" content="light dark"><title>${m.baslik} · Terzi Adem Altun</title>
<style>${CSS}</style></head><body><div class="sayfa">
<header><h1 class="marka">Terzi Adem Altun</h1>${dilCekmecesi(dil, adres)}</header>
<main>${govde}</main>${sube}
</div></body></html>`;
}

const CSS = `:root{color-scheme:light dark;--zemin:#f6f3ee;--kart:#fff;--yazi:#29251f;--ince:#665c50;--vurgu:#8a5a2b;--vurgu-yazi:#fff;--cizgi:#e4ddd2;--not:#efe8dc}
@media (prefers-color-scheme:dark){:root{--zemin:#14181c;--kart:#1d2329;--yazi:#e8ecf3;--ince:#b7c0cc;--vurgu:#d7a46b;--vurgu-yazi:#14181c;--cizgi:#323a43;--not:#262d34}}
*{box-sizing:border-box}
body{margin:0;background:var(--zemin);color:var(--yazi);font:16px/1.5 system-ui,-apple-system,"Segoe UI",Roboto,"Noto Sans","Noto Sans Arabic",sans-serif;-webkit-text-size-adjust:100%}
.sayfa{max-width:480px;margin:0 auto;padding:16px}
header{display:flex;align-items:center;justify-content:space-between;gap:12px;min-height:48px}
h1.marka{margin:0;font-size:20px;font-weight:700}
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
.alt{margin:2px 0 0;color:var(--ince);font-size:14px}.aciklama{margin:14px 0 0}
ol.adimlar{list-style:none;margin:20px 0 0;padding:0}
ol.adimlar li{position:relative;display:flex;align-items:center;gap:14px;min-height:48px}
ol.adimlar li::before{content:"";flex:none;width:28px;height:28px;border-radius:50%;border:2px solid var(--cizgi);background:var(--kart)}
ol.adimlar li+li::after{content:"";position:absolute;inset-inline-start:13px;top:-14px;height:28px;width:2px;background:var(--cizgi)}
ol.adimlar li.bitti::before{background:var(--vurgu);border-color:var(--vurgu)}
ol.adimlar li.bitti+li::after{background:var(--vurgu)}
ol.adimlar li.su::before{border-color:var(--vurgu);box-shadow:inset 0 0 0 6px var(--vurgu)}
ol.adimlar li.su{font-weight:700;font-size:18px}ol.adimlar li.sonra{color:var(--ince)}
.etiket{margin-inline-start:auto;padding:2px 10px;border-radius:999px;background:var(--not);color:var(--vurgu);font-size:13px;font-weight:700}
.not{margin:16px 0 0;padding:12px 14px;border-inline-start:4px solid var(--vurgu);border-radius:8px;background:var(--not);font-size:15px}
h3{margin:0 0 12px;font-size:16px}
dl{margin:0}dl div{display:flex;justify-content:space-between;gap:16px;padding:10px 0;border-bottom:1px solid var(--cizgi)}
dt{color:var(--ince)}dd{margin:0;font-weight:700;text-align:end}
ul.kalemler{list-style:none;margin:16px 0 0;padding:0;display:grid;gap:8px}
ul.kalemler li{padding:12px 14px;border-radius:8px;background:var(--not)}
ul.kalemler b{display:block}ul.kalemler span{color:var(--ince);font-size:14px}
a{color:var(--vurgu)}
footer{margin-top:20px;padding:12px 0;color:var(--ince);font-size:14px}
@media (prefers-reduced-motion:no-preference){details.dil>summary::after{transition:transform .15s}}`;
