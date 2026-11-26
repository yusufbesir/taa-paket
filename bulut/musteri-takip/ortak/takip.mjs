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

function dilCubugu(dil, adres) {
  if (!adres) return "";
  const kisa = DILLER.find(([d]) => d === dil)[2];
  const lis = DILLER.map(([d, ad, k]) =>
    d === dil
      ? `<li><span class="secili" lang="${d}" aria-current="true"><b>${k}</b> = ${ad}</span></li>`
      : `<li><a lang="${d}" hreflang="${d}" href="${kacis(adres)}?dil=${d}"><b>${k}</b> = ${ad}</a></li>`,
  ).join("");
  // Betiksiz çekmece: <details>/<summary> klavye ve ekran okuyucuyla çalışır.
  return `<details class="diller"><summary aria-label="${kacis(METIN[dil].dil)}: ${kisa}"><span aria-hidden="true">${kisa}</span></summary>
    <ul dir="ltr">${lis}</ul></details>`;
}

function fisOzeti(kayit, dil, m) {
  if (kayit.v !== 2) return "";
  const satirlar = [];
  if (kayit.g) satirlar.push(`<dt>${m.fis_tarihi}</dt><dd>${kacis(gunMetni(kayit.g, dil))}</dd>`);
  if (kayit.t) {
    const saat = kayit.h ? ` · ${kacis(kayit.h)}` : "";
    satirlar.push(`<dt>${m.teslim_gunu}</dt><dd>${kacis(gunMetni(kayit.t, dil))}${saat}</dd>`);
  }
  const kalemler = (kayit.u || []).map((x) => {
    const islem = islemAdi(x.i, dil) || m.diger_islem;
    const varyant = varyantAdi(x.v, dil);
    return `<li><b>${kacis(urunAdi(x.u, dil))}</b><span>${kacis(islem)}${varyant ? ` (${kacis(varyant)})` : ""}</span></li>`;
  }).join("");
  if (!satirlar.length && !kalemler) return "";
  return `<section class="fis"><h2>${m.fis}</h2>${satirlar.length ? `<dl>${satirlar.join("")}</dl>` : ""}
    ${kalemler ? `<h3>${m.islemler}</h3><ul class="kalemler">${kalemler}</ul>` : ""}</section>`;
}

/** Takip sayfası (sunucuda üretilir; istemci betiği yok). dil: DILLER'den biri; adres: dil bağlantıları için /t/<kod>. */
export function sayfa(kayit, { yedek = "", dil = "tr", adres = "" } = {}) {
  if (!METIN[dil]) dil = "tr";
  const m = METIN[dil];
  let govde;
  if (!kayit) {
    govde = `<div class="kutu"><h1>${m.yok_b}</h1><p>${m.yok_m}</p>
      ${yedek ? `<p class="ince">${m.yedek} <a href="${kacis(yedek)}?dil=${dil}">${m.yedek_a}</a></p>` : ""}</div>`;
  } else if (kayit.d === "iptal") {
    govde = `<div class="kutu"><h1>${m.iptal_b}</h1><p>${m.iptal_m}</p></div>`;
  } else {
    const sira = ADIMLAR.indexOf(kayit.d);
    const adimlar = ADIMLAR.map((a, i) => {
      const s = i < sira ? "tamam" : i === sira ? "simdi" : "";
      return `<li class="${s}"><span class="nokta" aria-hidden="true"></span><span>${m.adim[a]}</span></li>`;
    }).join("");
    const tahmin = kayit.t && kayit.v !== 2 ? `${m.tahmini}: <b>${kacis(gunMetni(kayit.t, dil))}</b>` : "";
    const ek = { alindi: tahmin, terzide: tahmin, hazir: m.hazir_m, teslim: m.teslim_m }[kayit.d];
    govde = `<div class="kutu"><p class="ust">${m.baslik}${kayit.s ? ` · ${m.sube} ${kacis(kayit.s)}` : ""}</p>
      <h1>${m.durum[kayit.d]}</h1><ol class="adimlar">${adimlar}</ol>${ek ? `<p>${ek}</p>` : ""}${fisOzeti(kayit, dil, m)}</div>`;
  }
  const yon = SAG_SOL.has(dil) ? "rtl" : "ltr";
  return `<!doctype html><html lang="${dil}" dir="${yon}"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1"><meta name="robots" content="noindex,nofollow">
<meta name="referrer" content="no-referrer"><title>${m.baslik} · Terzi Adem Altun</title>
<style>
:root{--zemin:#f6f3ee;--kart:#fff;--yazi:#1d1b18;--ince:#6b655c;--vurgu:#8a5a2b;--cizgi:#e4ddd2}
@media (prefers-color-scheme:dark){:root{--zemin:#16140f;--kart:#211e18;--yazi:#f1ece4;--ince:#a59d90;--vurgu:#d7a46b;--cizgi:#3a352c}}
*{box-sizing:border-box}body{margin:0;background:var(--zemin);color:var(--yazi);font:16px/1.5 system-ui,-apple-system,"Segoe UI",Roboto,"Noto Sans Arabic",sans-serif;min-height:100vh;display:flex;flex-direction:column;align-items:center;padding:16px;position:relative}
header{font-weight:700;letter-spacing:.08em;text-transform:uppercase;font-size:13px;color:var(--vurgu);margin:8px 0 20px;min-height:36px;display:flex;align-items:center}
.diller{position:absolute;top:16px;inset-inline-end:16px;z-index:2}
.diller summary{list-style:none;cursor:pointer;min-width:48px;height:36px;padding:0 12px;border-radius:999px;border:1px solid var(--cizgi);background:var(--kart);display:flex;align-items:center;justify-content:center;gap:6px;font-weight:700;font-size:14px;letter-spacing:.04em}
.diller summary::-webkit-details-marker{display:none}
.diller summary::after{content:"";width:6px;height:6px;border:solid var(--ince);border-width:0 2px 2px 0;transform:rotate(45deg) translateY(-2px)}
.diller[open] summary::after{transform:rotate(-135deg) translateY(-1px)}
.diller summary:focus-visible,.diller a:focus-visible{outline:2px solid var(--vurgu);outline-offset:2px}
.diller ul{list-style:none;margin:8px 0 0;padding:6px;position:absolute;inset-inline-end:0;background:var(--kart);border:1px solid var(--cizgi);border-radius:12px;box-shadow:0 8px 24px rgba(0,0,0,.15);min-width:200px;max-height:70vh;overflow:auto}
.diller li a,.diller .secili{display:block;padding:9px 12px;border-radius:8px;text-decoration:none;color:var(--yazi);font-size:15px;white-space:nowrap}
.diller li a:hover{background:var(--zemin)}.diller .secili{background:var(--vurgu);color:var(--kart)}
.diller b{display:inline-block;min-width:2.2em;font-weight:700}
.kutu{background:var(--kart);border:1px solid var(--cizgi);border-radius:16px;padding:24px;max-width:440px;width:100%}
h1{font-size:24px;margin:4px 0 18px}h2{font-size:16px;margin:0 0 10px}h3{font-size:14px;margin:14px 0 6px;color:var(--ince);font-weight:600}
.ust,.ince{color:var(--ince);font-size:14px;margin:0}
.adimlar{list-style:none;padding:0;margin:0 0 18px}.adimlar li{display:flex;gap:12px;align-items:center;padding:8px 0;color:var(--ince)}
.nokta{width:14px;height:14px;border-radius:50%;border:2px solid var(--cizgi);flex:none}
.tamam{color:var(--yazi)}.tamam .nokta{background:var(--vurgu);border-color:var(--vurgu)}
.simdi{color:var(--yazi);font-weight:700}.simdi .nokta{border-color:var(--vurgu);box-shadow:0 0 0 4px color-mix(in srgb,var(--vurgu) 25%,transparent)}
.fis{border-top:1px dashed var(--cizgi);margin-top:18px;padding-top:16px}
.fis dl{display:grid;grid-template-columns:auto 1fr;gap:4px 12px;margin:0;font-size:15px}.fis dt{color:var(--ince)}.fis dd{margin:0}
.kalemler{list-style:none;padding:0;margin:0}.kalemler li{display:flex;flex-direction:column;padding:6px 0;border-bottom:1px solid var(--cizgi)}
.kalemler li:last-child{border-bottom:0}.kalemler span{color:var(--ince);font-size:15px}
a{color:var(--vurgu)}
</style></head><body>${dilCubugu(dil, adres)}<header>Terzi Adem Altun</header>${govde}
</body></html>`;
}
