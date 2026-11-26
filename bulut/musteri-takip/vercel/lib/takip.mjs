// TAA müşteri takip: Cloudflare ve Vercel'in ORTAK kodu (doğrulama + sayfa).
// Mağaza sunucusu (taa-fis-sunucu, musteri_takip.rs) kaydı ed25519 ile imzalayıp POST /api/yaz'a iter.
// Kayıtta kişisel veri yoktur: {v, k: takip kodu, d: durum, s: şube kodu, t: teslim günü, ts, sil} (+v2/v3 alanları aşağıda).
// Bu dosya iki hedefe de aynen kopyalanır (tools/kopyala.sh); düzenlerken ikisini birlikte güncelleyin.
// Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].
// v3 (2026-11-26 04:40): isteğe bağlı kg = {d: kargo durumu, f: firma, n: takip no, t: kargolanma günü, y: "İlçe/İl"}; ad/telefon/adres/içerik YOK.
// v2 (2026-11-26 03:42): çok dilli sayfa + fiş özeti (g: fiş günü, h: teslim saati, u: [{u: ürün, i: işlem kodu, v: varyant}]).
import { DILLER, SAG_SOL, YEREL, METIN, urunAdi, islemAdi, varyantAdi, dilSec } from "./diller.mjs";
import { SUBELER, KARGO, KARGO_DURUM, EK } from "./ek.mjs";
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
  const kargoGecerli = (g) =>
    g && typeof g === "object" && KARGO_DURUM.includes(g.d) && Object.hasOwn(KARGO, g.f) &&
    typeof g.n === "string" && /^([A-Za-z0-9-]{4,40})?$/.test(g.n) &&
    typeof g.t === "string" && /^(\d{4}-\d{2}-\d{2})?$/.test(g.t) &&
    typeof g.y === "string" && g.y.length <= 60 && /^[\p{L} \/-]*$/u.test(g.y);
  if (k.v === 3 && k.kg !== undefined && !kargoGecerli(k.kg)) return { hata: "kayıt biçimi geçersiz", kod: 400 };
  if (k.v === 2 || k.v === 3) {
    if (
      typeof k.g !== "string" || !/^(\d{4}-\d{2}-\d{2})?$/.test(k.g) ||
      typeof k.h !== "string" || !/^(\d{2}:\d{2})?$/.test(k.h) ||
      !kalemlerGecerli(k.u)
    ) return { hata: "kayıt biçimi geçersiz", kod: 400 };
  }
  if (
    ![1, 2, 3].includes(k.v) ||
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
  if (k.v >= 2) {
    kayit.g = k.g;
    kayit.h = k.h;
    kayit.u = k.u.map((x) => ({ u: x.u, ...(x.i ? { i: x.i } : {}), ...(x.v ? { v: x.v } : {}) }));
  }
  if (k.v === 3 && k.kg) kayit.kg = { d: k.kg.d, f: k.kg.f, n: k.kg.n, t: k.kg.t, y: k.kg.y };
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
  if (!(kayit.v >= 2)) return "";
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

const SIMGE_TIK_KUCUK = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M5 12.5l4.5 4.5L19 7.5"/></svg>';

function zamanMetni(ts, dil) {
  try {
    return new Intl.DateTimeFormat(YEREL[dil] || "tr-TR", {
      day: "numeric", month: "long", hour: "2-digit", minute: "2-digit", timeZone: "Europe/Istanbul",
    }).format(new Date(ts * 1000));
  } catch {
    return "";
  }
}

const dugme = (href, metin, ikincil = false) =>
  `<a class="dugme${ikincil ? " ikincil" : ""}" href="${kacis(href)}" rel="noopener noreferrer"${href.startsWith("http") ? ' target="_blank"' : ""}>${metin}</a>`;

function kargoKarti(kg, dil, m) {
  if (!kg) return "";
  const firma = KARGO[kg.f] || KARGO.diger;
  const satir = (dt, dd) => (dd ? `<div><dt>${dt}</dt><dd>${dd}</dd></div>` : "");
  const no = kg.n ? `<code class="takip" dir="ltr">${kacis(kg.n)}</code>` : "";
  const url = kg.n ? firma.url(encodeURIComponent(kg.n)) : "";
  return `<section class="kart" aria-labelledby="kargo"><h3 id="kargo">${m.kargo}</h3><dl>
    ${satir(m.firma, kacis(firma.ad))}${satir(m.takip_no, no)}${satir(m.kargo_tarihi, kg.t ? kacis(gunMetni(kg.t, dil)) : "")}${satir(m.varis, kg.y ? `<bdi>${kacis(kg.y)}</bdi>` : "")}</dl>
    ${url ? `<div class="dugmeler">${dugme(url, m.takip_et)}</div>` : kg.n ? "" : `<p class="not">${m.kargo_m}</p>`}</section>`;
}

function subeKarti(kod, kargoda, m) {
  if (!kod) return "";
  const s = SUBELER[kod];
  const baslik = kargoda ? m.sube_b : m.teslim_yeri;
  if (!s) return `<section class="kart" aria-labelledby="sube"><h3 id="sube">${baslik}</h3><p class="sube-ad">${kacis(kod)}</p></section>`;
  const harita = s.adres ? `https://www.google.com/maps/search/?api=1&query=${encodeURIComponent(`Terzi Adem Altun, ${s.adres}`)}` : "";
  const dugmeler = [s.tel ? dugme(`tel:${s.tel}`, m.ara, true) : "", harita ? dugme(harita, m.yol, true) : ""].join("");
  return `<section class="kart" aria-labelledby="sube"><h3 id="sube">${baslik}</h3>
    <p class="sube-ad"><bdi lang="tr">Terzi Adem Altun · ${kacis(s.ad)}</bdi></p>${s.adres ? `<p class="adres" lang="tr"><bdi>${kacis(s.adres)}</bdi></p>` : ""}
    ${s.tel_g ? `<p class="tel"><bdi dir="ltr">${kacis(s.tel_g)}</bdi></p>` : ""}
    ${s.saat ? `<dl><div><dt>${m.saatler}</dt><dd dir="ltr">${kacis(s.saat)}</dd></div></dl>` : ""}
    ${dugmeler ? `<div class="dugmeler">${dugmeler}</div>` : ""}</section>`;
}

/** Takip sayfası (sunucuda üretilir; istemci betiği yok). dil: DILLER'den biri; adres: dil bağlantıları için /t/<kod>. */
export function sayfa(kayit, { yedek = "", dil = "tr", adres = "" } = {}) {
  if (!METIN[dil]) dil = "tr";
  const m = { ...METIN[dil], ...EK[dil] };
  const tekKart = (ikon, baslik, metin, ek = "") =>
    `<section class="kart"><div class="hero">${simge(ikon)}<div><h2>${baslik}</h2></div></div><p class="aciklama">${metin}</p>${ek}</section>`;
  let govde;
  if (!kayit) {
    govde = tekKart("soru", m.yok_b, m.yok_m,
      yedek ? `<p class="alt">${m.yedek} <a href="${kacis(yedek)}?dil=${dil}">${m.yedek_a}</a></p>` : "");
  } else if (kayit.d === "iptal") {
    govde = tekKart("carpi", m.iptal_b, m.iptal_m);
  } else {
    const kg = kayit.kg;
    // Kargoda adımlar: alındı → terzide → kargolandı → teslim. Mağazadan teslimde: alındı → terzide → hazır → teslim.
    const adimlar = kg ? ["alindi", "terzide", "kargo", "teslim"] : ADIMLAR;
    let sira = ADIMLAR.indexOf(kayit.d);
    if (kg) sira = kayit.d === "teslim" || kg.d === "teslim" ? 3 : 2;
    const ad = (a) => (a === "kargo" ? (kg.d === "hazirlaniyor" ? m.adim_kargo_h : m.adim_kargo) : m.adim[a]);
    const acik = (a) => (a === "kargo" && kg.d === "hazirlaniyor" ? m.acik.kargo_h : m.acik[a]);
    const lis = adimlar.map((a, i) => {
      const s = i < sira ? "bitti" : i === sira ? "su" : "sonra";
      return `<li class="${s}"${s === "su" ? ' aria-current="step"' : ""}><span class="nokta">${s === "bitti" ? SIMGE_TIK_KUCUK : ""}</span>
        <div class="metin"><span class="ad">${ad(a)}</span><small>${acik(a)}</small></div>${s === "su" ? `<span class="etiket">${m.guncel}</span>` : ""}</li>`;
    }).join("");
    const tahmin = kayit.t && !(kayit.v >= 2) ? `${m.tahmini}: <b>${kacis(gunMetni(kayit.t, dil))}</b>` : "";
    const not = kg ? "" : { alindi: tahmin, terzide: tahmin, hazir: m.hazir_m, teslim: m.teslim_m }[kayit.d];
    const bitti = kayit.d === "teslim" || kayit.d === "hazir" || (kg && kg.d !== "hazirlaniyor");
    const baslik = kg && kayit.d !== "teslim" ? m.kd[kg.d] : kg && kayit.d === "teslim" ? m.kd.teslim : m.durum[kayit.d];
    const zaman = zamanMetni(kayit.ts, dil);
    govde = `<section class="kart durum" aria-labelledby="durum"><div class="hero">${simge(bitti ? "tik" : "saat")}
      <div><h2 id="durum">${baslik}</h2><p class="alt">${m.baslik}</p></div></div>
      <ol class="adimlar" aria-label="${m.baslik}">${lis}</ol>${not ? `<p class="not">${not}</p>` : ""}
      ${zaman ? `<p class="guncelleme">${m.guncelleme}: <time datetime="${new Date(kayit.ts * 1000).toISOString()}">${kacis(zaman)}</time></p>` : ""}</section>
      ${kargoKarti(kg, dil, m)}${subeKarti(kayit.s, !!kg, m)}${fisOzeti(kayit, dil, m)}`;
  }
  const yon = SAG_SOL.has(dil) ? "rtl" : "ltr";
  return `<!doctype html><html lang="${dil}" dir="${yon}"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex,nofollow">
<meta name="referrer" content="no-referrer"><meta name="color-scheme" content="light dark"><title>${m.baslik} · Terzi Adem Altun</title>
<style>${CSS}</style></head><body><div class="sayfa">
<header><div class="marka"><span class="logo" aria-hidden="true">TA</span><h1>Terzi Adem Altun</h1></div>${dilCekmecesi(dil, adres)}</header>
<main>${govde}</main>
<footer><span>Terzi Adem Altun · ${m.baslik}</span></footer>
</div></body></html>`;
}

const CSS = `:root{color-scheme:light dark;--zemin:#f6f3ee;--kart:#fff;--yazi:#29251f;--ince:#665c50;--vurgu:#8a5a2b;--vurgu-yazi:#fff;--cizgi:#e4ddd2;--not:#efe8dc;--golge:0 1px 2px rgba(41,37,31,.06),0 8px 24px rgba(41,37,31,.06)}
@media (prefers-color-scheme:dark){:root{--zemin:#14181c;--kart:#1d2329;--yazi:#e8ecf3;--ince:#b7c0cc;--vurgu:#d7a46b;--vurgu-yazi:#14181c;--cizgi:#323a43;--not:#262d34;--golge:none}}
*{box-sizing:border-box}
body{margin:0;background:var(--zemin);color:var(--yazi);font:16px/1.5 system-ui,-apple-system,"Segoe UI",Roboto,"Noto Sans","Noto Sans Arabic",sans-serif;-webkit-text-size-adjust:100%}
.sayfa{max-width:480px;margin:0 auto;padding:16px 16px 8px}
header{display:flex;align-items:center;justify-content:space-between;gap:12px;min-height:48px}
.marka{display:flex;align-items:center;gap:10px}.marka h1{margin:0;font-size:19px;font-weight:700;letter-spacing:.01em}
.logo{width:36px;height:36px;border-radius:10px;background:var(--vurgu);color:var(--vurgu-yazi);display:grid;place-items:center;font:700 14px/1 Georgia,"Times New Roman",serif;letter-spacing:.04em}
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
.kart{margin-top:14px;padding:20px 18px;background:var(--kart);border:1px solid var(--cizgi);border-radius:16px;box-shadow:var(--golge)}
.kart.durum{border-top:4px solid var(--vurgu)}
.hero{display:flex;gap:14px;align-items:center}
.tik{flex:none;width:52px;height:52px;border-radius:50%;background:var(--vurgu);color:var(--vurgu-yazi);display:grid;place-items:center;box-shadow:0 0 0 6px var(--not)}
h2{margin:0;font-size:23px;line-height:1.25}
.alt{margin:2px 0 0;color:var(--ince);font-size:14px}.aciklama{margin:14px 0 0}
ol.adimlar{list-style:none;margin:22px 0 0;padding:0}
ol.adimlar li{position:relative;display:flex;align-items:flex-start;gap:14px;padding-bottom:20px}
ol.adimlar li:last-child{padding-bottom:0}
ol.adimlar li:not(:last-child)::after{content:"";position:absolute;inset-inline-start:15px;top:32px;bottom:0;width:2px;border-radius:1px;background:var(--cizgi)}
ol.adimlar li.bitti:not(:last-child)::after{background:var(--vurgu)}
.nokta{flex:none;position:relative;z-index:1;width:32px;height:32px;border-radius:50%;border:2px solid var(--cizgi);background:var(--kart);display:grid;place-items:center;color:var(--vurgu-yazi)}
li.bitti .nokta{background:var(--vurgu);border-color:var(--vurgu)}
li.su .nokta{border-color:var(--vurgu);box-shadow:0 0 0 5px var(--not)}
li.su .nokta::after{content:"";width:12px;height:12px;border-radius:50%;background:var(--vurgu)}
li.sonra .nokta{border-style:dashed}
.metin{flex:1;min-width:0;display:flex;flex-direction:column;padding-top:4px}
.metin .ad{font-weight:600;line-height:24px}.metin small{color:var(--ince);font-size:13.5px;line-height:1.4}
li.su .metin .ad{font-weight:700;font-size:17px}li.sonra .metin .ad{color:var(--ince);font-weight:500}
.etiket{flex:none;margin-top:4px;padding:2px 10px;border-radius:999px;background:var(--not);color:var(--vurgu);font-size:12.5px;font-weight:700;line-height:20px}
.not{margin:18px 0 0;padding:12px 14px;border-inline-start:4px solid var(--vurgu);border-radius:8px;background:var(--not);font-size:15px}
.guncelleme{margin:16px 0 0;padding-top:12px;border-top:1px solid var(--cizgi);color:var(--ince);font-size:13px}
h3{margin:0 0 12px;font-size:13px;letter-spacing:.06em;text-transform:uppercase;color:var(--ince)}
:lang(ar) h3,:lang(fa) h3,:lang(zh) h3,:lang(hi) h3{text-transform:none;letter-spacing:0;font-size:15px}
.sube-ad{margin:0;font-weight:700;font-size:17px}.tel{margin:4px 0 0;color:var(--ince);font-weight:600}.adres{margin:6px 0 0;color:var(--yazi)}
dl{margin:8px 0 0}dl div{display:flex;justify-content:space-between;align-items:baseline;gap:16px;padding:11px 0;border-bottom:1px solid var(--cizgi)}
dl div:last-child{border-bottom:0}
dt{color:var(--ince)}dd{margin:0;font-weight:700;text-align:end}
code.takip{font:700 15px ui-monospace,Menlo,Consolas,monospace;letter-spacing:.04em;user-select:all;-webkit-user-select:all;padding:2px 8px;border-radius:6px;background:var(--not)}
.dugmeler{display:flex;flex-wrap:wrap;gap:10px;margin-top:14px}
.dugme{flex:1 1 140px;display:flex;align-items:center;justify-content:center;min-height:48px;padding:0 16px;border-radius:12px;background:var(--vurgu);color:var(--vurgu-yazi);font-weight:700;text-decoration:none}
.dugme.ikincil{background:transparent;color:var(--vurgu);border:1.5px solid var(--vurgu)}
ul.kalemler{list-style:none;margin:14px 0 0;padding:0;display:grid;gap:8px}
ul.kalemler li{padding:12px 14px;border-radius:10px;background:var(--not)}
ul.kalemler b{display:block}ul.kalemler span{color:var(--ince);font-size:14px}
a{color:var(--vurgu)}
footer{margin-top:22px;padding:14px 0 8px;text-align:center;color:var(--ince);font-size:13px}
@media (prefers-reduced-motion:no-preference){details.dil>summary::after{transition:transform .15s}
li.su .nokta{animation:nabiz 2.4s ease-out infinite}
@keyframes nabiz{0%{box-shadow:0 0 0 0 color-mix(in srgb,var(--vurgu) 45%,transparent)}70%{box-shadow:0 0 0 10px transparent}100%{box-shadow:0 0 0 5px var(--not)}}}`;
