// TAA müşteri takip: Cloudflare ve Vercel'in ORTAK kodu (doğrulama + sayfa).
// Mağaza sunucusu (taa-fis-sunucu, musteri_takip.rs) kaydı ed25519 ile imzalayıp POST /api/yaz'a iter.
// Kayıtta kişisel veri yoktur: {v, k: takip kodu, d: durum, s: şube kodu, t: teslim günü, ts, sil} (+v2/v3 alanları aşağıda).
// Bu dosya iki hedefe de aynen kopyalanır (./kopyala.sh); düzenlerken ikisini birlikte güncelleyin.
// Yazan: [Claude Opus 5.5, TerziTakip başlığı, 2026-11-26].
// v3 (2026-11-26 04:40): isteğe bağlı kg = {d: kargo durumu, f: firma, n: takip no, t: kargolanma günü, y: "İlçe/İl"}; ad/telefon/adres/içerik YOK.
// v4 (2026-11-26 05:00): v>=2 kalemde isteğe bağlı r (RENK anahtarı) ve a (adet 2..20), kayıtta b (bağlı fiş sayısı 1..20);
//   sayfa sekmeli (?sekme=ozet|islemler|subeler|sss), şubeler + SSS + WhatsApp; hâlâ betiksiz.
// v4.1 (2026-11-26, Fable denetimi): gövde sınırı 4096 (20 dolu kalem + kargo sığsın); "burada" rozeti yalnız ürün
//   şubedeyken; kargoya hazırlanırken adım sırası kayıt durumundan; SUBELER Object.hasOwn ile; adressiz şube notu.
// v2 (2026-11-26 03:42): çok dilli sayfa + fiş özeti (g: fiş günü, h: teslim saati, u: [{u: ürün, i: işlem kodu, v: varyant}]).
import { DILLER, SAG_SOL, YEREL, METIN, urunAdi, islemAdi, varyantAdi, dilSec } from "./diller.mjs";
import { SUBELER, KARGO, KARGO_DURUM, EK, RENK, EK2, WHATSAPP_MERKEZ } from "./ek.mjs";
export { dilSec };

export const KOD_RE = /^[a-km-np-z2-9]{16}$/;
const DURUMLAR = ["alindi", "terzide", "hazir", "teslim", "iptal"];
const GUN = 86400;

export function hexBayt(h) {
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
  if (typeof govde !== "string" || govde.length > 4096) return { hata: "gövde geçersiz", kod: 400 };
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
  const araliktaTam = (x, en_az, en_cok) => Number.isInteger(x) && x >= en_az && x <= en_cok;
  const anahtar_re = /^[a-z0-9-]{1,40}$/;
  const kalemlerGecerli = (u) =>
    Array.isArray(u) && u.length <= 20 &&
    u.every((x) => x && typeof x === "object" && anahtar_re.test(x.u) &&
      (x.i === undefined || anahtar_re.test(x.i)) && (x.v === undefined || x.v === "duz" || x.v === "duble") &&
      (x.r === undefined || (typeof x.r === "string" && Object.hasOwn(RENK, x.r))) &&
      (x.a === undefined || araliktaTam(x.a, 2, 20)));
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
      !kalemlerGecerli(k.u) ||
      (k.b !== undefined && !araliktaTam(k.b, 1, 20))
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
  // Yalnız bilinen alanlar saklanır. (Sınır 4096: 20 kalem r/a/i/v dolu + kargo ≈ 1,7 KB; 1024 12 kalemde aşılıyordu.)
  const kayit = { v: k.v, k: k.k, d: k.d, s: k.s, t: k.t, ts: k.ts, sil: k.sil };
  if (k.v >= 2) {
    kayit.g = k.g;
    kayit.h = k.h;
    kayit.u = k.u.map((x) => ({
      u: x.u, ...(x.i ? { i: x.i } : {}), ...(x.v ? { v: x.v } : {}), ...(x.r ? { r: x.r } : {}), ...(x.a ? { a: x.a } : {}),
    }));
    if (k.b !== undefined) kayit.b = k.b;
  }
  if (k.v === 3 && k.kg) kayit.kg = { d: k.kg.d, f: k.kg.f, n: k.kg.n, t: k.kg.t, y: k.kg.y };
  return { kayit };
}

/** İmzalı şube tablosu (mağaza sunucusunun günlük site eşitlemesi). Biçim:
 *  {v:1, tur:"subeler", ts, wa?, subeler:{KOD:{ad, adres?, tel?, saat?, wa?, not?}}}. Yalnız kamuya açık işletme bilgisi. */
export async function dogrulaSubeler(govde, imzaHex, genelHex, simdi = Math.floor(Date.now() / 1000)) {
  const genel = hexBayt(genelHex);
  const imza = hexBayt(imzaHex);
  if (!genel || genel.length !== 32) return { hata: "sunucu ayarı eksik", kod: 500 };
  if (!imza || imza.length !== 64) return { hata: "imza yok", kod: 401 };
  if (typeof govde !== "string" || govde.length > 16384) return { hata: "gövde geçersiz", kod: 400 };
  const anahtar = await crypto.subtle.importKey("raw", genel, { name: "Ed25519" }, false, ["verify"]);
  if (!(await crypto.subtle.verify("Ed25519", anahtar, imza, new TextEncoder().encode(govde)))) return { hata: "imza geçersiz", kod: 401 };
  let k;
  try {
    k = JSON.parse(govde);
  } catch {
    return { hata: "gövde geçersiz", kod: 400 };
  }
  const yazi = (x, n) => x === undefined || (typeof x === "string" && x.length <= n && !/[<>\u0000-\u001f]/.test(x));
  const no = (x) => x === undefined || (typeof x === "string" && /^\+?\d{0,15}$/.test(x));
  if (!k || k.v !== 1 || k.tur !== "subeler" || !Number.isInteger(k.ts) || !no(k.wa) ||
    !k.subeler || typeof k.subeler !== "object" || Array.isArray(k.subeler)) return { hata: "tablo biçimi geçersiz", kod: 400 };
  if (k.ts > simdi + 600 || k.ts < simdi - 30 * GUN) return { hata: "tablo zamanı geçersiz", kod: 400 };
  const girdiler = Object.entries(k.subeler);
  if (girdiler.length < 1 || girdiler.length > 60) return { hata: "tablo biçimi geçersiz", kod: 400 };
  const subeler = {};
  for (const [kod, x] of girdiler) {
    if (!/^[A-Z0-9]{2,8}$/.test(kod) || !x || typeof x !== "object" || typeof x.ad !== "string" || !x.ad ||
      !yazi(x.ad, 60) || !yazi(x.adres, 160) || !yazi(x.saat, 40) || !yazi(x.not, 80) || !no(x.tel) || !no(x.wa)) {
      return { hata: "tablo biçimi geçersiz", kod: 400 };
    }
    subeler[kod] = { ad: x.ad, adres: x.adres || "", tel: x.tel || "", saat: x.saat || "", wa: x.wa || "", not: x.not || "" };
  }
  return { tablo: { v: 1, ts: k.ts, wa: k.wa || "", subeler } };
}

/** +90XXXXXXXXXX → "0XXX XXX XX XX" (Türkiye dışı numara olduğu gibi). */
function telGorunen(tel) {
  const m = /^\+90(\d{3})(\d{3})(\d{2})(\d{2})$/.exec(tel || "");
  return m ? `0${m[1]} ${m[2]} ${m[3]} ${m[4]}` : tel || "";
}

/** Dosyadaki ilk tabloyla imzalı tabloyu birleştirir: imzalı tablodaki dolu alanlar üstün, yeni şubeler eklenir. */
export function subeTablosu(tablo) {
  if (!tablo || !tablo.subeler) return SUBELER;
  const sonuc = {};
  for (const [kod, s] of Object.entries(SUBELER)) sonuc[kod] = { ...s };
  for (const [kod, s] of Object.entries(tablo.subeler)) {
    const eski = Object.hasOwn(sonuc, kod) ? sonuc[kod] : {};
    const yeni = { ...eski };
    for (const [a, d] of Object.entries(s)) if (d) yeni[a] = d;
    if (s.tel) yeni.tel_g = telGorunen(s.tel);
    sonuc[kod] = yeni;
  }
  return sonuc;
}

/** Eski kayıt yenisinin üzerine yazılmasın (tekrar oynatma / sıra karışması). */
export function yeniMi(eski, yeni) {
  return !eski || yeni.ts >= eski.ts;
}

const ADIMLAR = ["alindi", "terzide", "hazir", "teslim"];

export function kacis(s) {
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
export function dilCekmecesi(dil, adres, sekme = "ozet") {
  if (!adres) return "";
  const [, ad, kisa] = DILLER.find(([d]) => d === dil);
  // Dil değişince açık sekme korunur (özet varsayılan olduğu için yazılmaz).
  const ek = sekme === "ozet" ? "" : `sekme=${sekme}&`;
  const lis = DILLER.map(([d, a, k]) =>
    `<li><a href="${kacis(`${adres}?${ek}dil=${d}`)}" lang="${d}" hreflang="${d}" dir="ltr"${d === dil ? ' aria-current="true"' : ""}><b>${k}</b><span aria-hidden="true">=</span><bdi lang="${d}">${a}</bdi></a></li>`,
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

const dugme = (href, metin, ikincil = false, sinif = "") =>
  `<a class="dugme${ikincil ? " ikincil" : ""}${sinif ? ` ${sinif}` : ""}" href="${kacis(href)}" rel="noopener noreferrer"${href.startsWith("http") ? ' target="_blank"' : ""}>${metin}</a>`;

export const SEKMELER = ["ozet", "islemler", "subeler", "sss"];
const sekmeSec = (s) => (SEKMELER.includes(s) ? s : "ozet");

/** Sekme bağlantısı: dil her zaman korunur (?sekme=x&dil=y). */
const sekmeAdresi = (adres, sekme, dil) => `${adres}?sekme=${sekme}&dil=${dil}`;

/** Yol tarifi: telefonda harita uygulamasında navigasyonu açar. */
const yolTarifi = (adres) =>
  `https://www.google.com/maps/dir/?api=1&destination=${encodeURIComponent(`Terzi Adem Altun, ${adres}`)}`;

/** WhatsApp numarası: şubenin wa'sı, yoksa merkez; yalnız rakamlar. Boşsa "" (düğme çizilmez). */
const waNumarasi = (sube, merkez) => String((sube && sube.wa) || merkez || "").replace(/\D/g, "");

function waDugmesi(no, kod, m) {
  if (!no) return "";
  const metin = kod ? `?text=${encodeURIComponent(m.wa_metin.replace("{kod}", kod))}` : "";
  return dugme(`https://wa.me/${no}${metin}`, `${SIMGE_WA}<span>${m.wa}</span>`, true, "wa");
}
const SIMGE_WA = '<svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 20l1.3-3.9A8 8 0 1 1 8 19z"/><path d="M9.5 9.5c0 2.5 2.5 5 5 5l1-1.2-1.8-.8-.7.7a3.5 3.5 0 0 1-2-2l.7-.7-.8-1.8z"/></svg>';

function renkAdi(r, dil) {
  if (!r || !RENK[r]) return "";
  const i = DILLER.findIndex(([d]) => d === dil);
  return RENK[r][i] || RENK[r][0];
}

/** Fiş tarihi ve teslim günü/saati satırları (v>=2). */
function fisSatirlari(kayit, dil, m) {
  const satirlar = [];
  if (kayit.g) satirlar.push(`<div><dt>${m.fis_tarihi}</dt><dd>${kacis(gunMetni(kayit.g, dil))}</dd></div>`);
  if (kayit.t) {
    const saat = kayit.h ? ` · ${kacis(kayit.h)}` : "";
    satirlar.push(`<div><dt>${m.teslim_gunu}</dt><dd>${kacis(gunMetni(kayit.t, dil))}${saat}</dd></div>`);
  }
  return satirlar.length ? `<dl>${satirlar.join("")}</dl>` : "";
}

const bagliNot = (kayit, m) => (kayit.b ? `<p class="not">${kacis(m.bagli_m.replace("{n}", kayit.b))}</p>` : "");

/** Özet sekmesindeki fiş kartı: tarih/teslim satırları + bağlı fiş notu. */
function fisKarti(kayit, dil, m) {
  if (!(kayit.v >= 2)) return "";
  const satirlar = fisSatirlari(kayit, dil, m);
  const bagli = bagliNot(kayit, m);
  if (!satirlar && !bagli) return "";
  return `<section class="kart" aria-labelledby="ozet"><h3 id="ozet">${m.ozet}</h3>${satirlar}${bagli}</section>`;
}

/** İşlemler sekmesi: her kalem bir kart satırı ("2 × Pantolon · Siyah", altında işlem · varyant). */
function islemlerKarti(kayit, dil, m) {
  const ust = kayit.v >= 2 ? fisSatirlari(kayit, dil, m)
    : kayit.t ? `<dl><div><dt>${m.tahmini}</dt><dd>${kacis(gunMetni(kayit.t, dil))}</dd></div></dl>` : "";
  const kalemler = (kayit.u || []).map((x) => {
    const islem = islemAdi(x.i, dil) || m.diger_islem;
    const varyant = varyantAdi(x.v, dil);
    const renk = renkAdi(x.r, dil);
    // Adet yönü sayfayı izler (RTL'de de "2 × ürün" sırası doğal okunur).
    const adet = x.a ? `<bdi class="adet">${x.a}</bdi> × ` : "";
    return `<li><div><b>${adet}${kacis(urunAdi(x.u, dil))}${renk ? ` · ${kacis(renk)}` : ""}</b>
      <span>${kacis(islem)}${varyant ? ` · ${kacis(varyant)}` : ""}</span></div></li>`;
  }).join("");
  return `<section class="kart" aria-labelledby="islemler"><h2 id="islemler" class="kb">${m.islemler}</h2>${ust}
    ${kalemler ? `<ul class="kalemler">${kalemler}</ul>` : `<p class="bos">${kacis(m.kalem_yok)}</p>`}${bagliNot(kayit, m)}</section>`;
}

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

/** Şube kodu → SUBELER kaydı; Object.prototype üyeleri ("constructor", "__proto__") şube sayılmaz. */
// sayfa() eşzamanlıdır; başında etkin tablo atanır, bitince dosyadaki tabloya döner.
let AKTIF = SUBELER;
const subeBul = (kod) => (typeof kod === "string" && Object.hasOwn(AKTIF, kod) ? AKTIF[kod] : null);

const subeAdi = (s) => `<bdi lang="tr">Terzi Adem Altun · ${kacis(s.ad)}</bdi>`;

/** Adresi girilmemiş şubede: "açık adres için merkezi arayın" + merkez telefonu (merkezin telefonu varsa). */
function adresYokNotu(s, m) {
  const merkez = subeBul("MRK01");
  if (s.adres || !merkez || !merkez.tel || s === merkez) return "";
  return `<p class="adres-yok">${kacis(m.adres_yok)} <a href="${kacis(`tel:${merkez.tel}`)}"><bdi dir="ltr">${kacis(merkez.tel_g || merkez.tel)}</bdi></a></p>`;
}

/** Ürün şubede mi (müşteri oradan alacak/alabilir): kargoda değil ve teslim edilmemiş. */
const urunSubede = (durum, kargoda) => !kargoda && ["alindi", "terzide", "hazir"].includes(durum);

/** Şube kartı. tam: telefon, saatler ve WhatsApp da gösterilir (Şubeler sekmesi, başlık h2); değilse kısa kart (Özet).
 *  durum: kayıt durumu; teslim edilmiş ya da kargodaki üründe başlık "Şubemiz" olur ve "burada" rozeti çıkmaz. */
function subeKarti(kod, { kargoda, durum = "", m, tam = false, wa = "", takipKodu = "" }) {
  if (!kod) return "";
  const s = subeBul(kod);
  const subede = urunSubede(durum, kargoda);
  const baslik = subede ? m.teslim_yeri : m.sube_b;
  const h = tam ? "h2" : "h3";
  const bas = `<${h} id="sube"${tam ? ' class="kb"' : ""}>${baslik}</${h}>`;
  const rozet = tam && subede ? `<span class="rozet">${SIMGE_KONUM}${m.burada}</span>` : "";
  if (!s) return `<section class="kart sube-kart" aria-labelledby="sube">${bas}${rozet}<p class="sube-ad">${kacis(kod)}</p></section>`;
  const dugmeler = [
    s.adres ? dugme(yolTarifi(s.adres), m.yol) : "",
    s.tel ? dugme(`tel:${s.tel}`, m.ara, true) : "",
    tam ? waDugmesi(waNumarasi(s, wa), takipKodu, m) : "",
  ].join("");
  return `<section class="kart sube-kart${tam ? " burada" : ""}" aria-labelledby="sube">${bas}${rozet}
    <p class="sube-ad">${subeAdi(s)}</p>${s.adres ? `<p class="adres" lang="tr"><bdi>${kacis(s.adres)}</bdi></p>` : adresYokNotu(s, m)}${subeNotu(s)}
    ${tam && s.tel_g ? `<p class="tel"><bdi dir="ltr">${kacis(s.tel_g)}</bdi></p>` : ""}
    ${tam && s.saat ? `<dl><div><dt>${m.saatler}</dt><dd dir="ltr">${kacis(s.saat)}</dd></div></dl>` : ""}
    ${dugmeler ? `<div class="dugmeler">${dugmeler}</div>` : ""}</section>`;
}
const SIMGE_KONUM = '<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 21s-7-6.2-7-11.5a7 7 0 0 1 14 0C19 14.8 12 21 12 21z"/><circle cx="12" cy="9.5" r="2.5"/></svg>';

const subeNotu = (s) => (s.not ? `<p class="sube-not" lang="tr"><bdi>${kacis(s.not)}</bdi></p>` : "");

/** Arama metnini sadeleştirir: Türkçe küçük harf, aksansız (ı/i, ş/s... eşit sayılır). */
const sade = (x) => String(x || "").toLocaleLowerCase("tr-TR").normalize("NFD").replace(/[\u0300-\u036f]/g, "").replace(/ı/g, "i");

/** Şubeler sekmesi: kayıttaki şube en üstte tam kart; diğer şubeler çekmecede (details), içinde arama kutusu.
 *  Arama betiksiz: GET ?sekme=subeler&ara=... ile sunucuda süzülür; arama varken çekmece açık gelir. */
function subelerSekmesi(kayit, m, wa, { adres = "", dil = "tr", ara = "" } = {}) {
  const ust = subeKarti(kayit.s, { kargoda: !!kayit.kg, durum: kayit.d, m, tam: true, wa, takipKodu: kayit.k });
  const tum = Object.entries(AKTIF).filter(([k]) => k !== kayit.s);
  // Kararlı sıralama: adresli şubeler önce, kendi aralarında dosyadaki sıra korunur.
  tum.sort(([, a], [, b]) => (b.adres ? 1 : 0) - (a.adres ? 1 : 0));
  const q = sade(ara).trim();
  const digerleri = q ? tum.filter(([, s]) => sade(`${s.ad} ${s.adres || ""} ${s.not || ""}`).includes(q)) : tum;
  const adressizVar = digerleri.some(([, s]) => !s.adres);
  const lis = digerleri.map(([, s]) => {
    const d = [s.adres ? dugme(yolTarifi(s.adres), m.yol, true, "kucuk") : "", s.tel ? dugme(`tel:${s.tel}`, m.ara, true, "kucuk") : ""].join("");
    return `<li><p class="sube-ad">${subeAdi(s)}</p>${s.adres ? `<p class="adres" lang="tr"><bdi>${kacis(s.adres)}</bdi></p>` : ""}${subeNotu(s)}${s.tel_g ? `<p class="tel"><bdi dir="ltr">${kacis(s.tel_g)}</bdi></p>` : ""}${d ? `<div class="dugmeler">${d}</div>` : ""}</li>`;
  }).join("");
  const not = adressizVar ? adresYokNotu({}, m) : "";
  const form = adres
    ? `<form class="sube-ara" method="get" action="${kacis(adres)}" role="search"><input type="hidden" name="sekme" value="subeler"><input type="hidden" name="dil" value="${kacis(dil)}">
      <label class="gizli" for="ara">${kacis(m.ara_b)}</label><input id="ara" type="search" name="ara" value="${kacis(ara)}" maxlength="40" placeholder="${kacis(m.ara_ph)}" autocomplete="off" enterkeyhint="search">
      <button type="submit">${SIMGE_ARA}<span>${kacis(m.ara_d)}</span></button></form>${q ? `<p class="ara-sonuc"><a href="${kacis(sekmeAdresi(adres, "subeler", dil))}">${kacis(m.ara_temizle)}</a></p>` : ""}`
    : "";
  const liste = digerleri.length ? `<ul class="subeler">${lis}</ul>` : `<p class="not">${kacis(m.ara_yok)}</p>`;
  return `${ust}<section class="kart" aria-labelledby="diger"><details class="cekmece"${q ? " open" : ""}><summary><h2 id="diger" class="kb">${m.diger_subeler}</h2><span class="sayi">${tum.length}</span></summary>
    ${form}${liste}${not}</details></section>`;
}
const SIMGE_ARA = '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6" stroke-linecap="round" aria-hidden="true"><circle cx="11" cy="11" r="6.5"/><path d="M16 16l4.5 4.5"/></svg>';

/** EK2.sss içinde WhatsApp'ı anan sorunun sırası (ek işlem). */
const SSS_WA_SIRA = 3;

/** SSS sekmesi: <details> akordeon + WhatsApp düğmesi (numara varsa). Numara yoksa WhatsApp'ı anan yanıt
 *  WhatsApp'sız haliyle (sss_wa_yok) gösterilir; müşteri sayfada olmayan bir bağlantıya yönlendirilmez. */
function sssSekmesi(m, waNo, takipKodu) {
  const sorular = m.sss.map(([s, c], i) => {
    const yanit = !waNo && i === SSS_WA_SIRA ? m.sss_wa_yok : c;
    return `<details class="sss"><summary>${kacis(s)}</summary><p>${kacis(yanit)}</p></details>`;
  }).join("");
  const wa = waDugmesi(waNo, takipKodu, m);
  return `<section class="kart" aria-labelledby="sss"><h2 id="sss" class="sss-b">${m.sss_b}</h2><div class="sss-liste">${sorular}</div>
    ${wa ? `<div class="dugmeler">${wa}</div>` : ""}</section>`;
}

function sekmeCubugu(sekme, dil, adres, m) {
  const a = SEKMELER.map((s) =>
    `<a href="${kacis(sekmeAdresi(adres, s, dil))}"${s === sekme ? ' aria-current="page"' : ""}>${m.sekme[s]}</a>`).join("");
  return `<nav class="sekmeler" aria-label="${kacis(m.bolumler)}">${a}</nav>`;
}

/** Takip sayfası (sunucuda üretilir; istemci betiği yok).
 *  dil: DILLER'den biri; adres: bağlantılar için /t/<kod>; sekme: SEKMELER'den biri (bilinmeyen → ozet);
 *  wa: merkez WhatsApp numarası (varsayılan WHATSAPP_MERKEZ; testler dosyayı değiştirmeden geçebilsin diye seçenek). */
export function sayfa(kayit, secenek = {}) {
  const tablo = secenek.tablo || null;
  AKTIF = subeTablosu(tablo);
  try {
    return sayfaUret(kayit, { ...secenek, wa: secenek.wa ?? (tablo && tablo.wa) ?? WHATSAPP_MERKEZ });
  } finally {
    AKTIF = SUBELER;
  }
}

function sayfaUret(kayit, { yedek = "", dil = "tr", adres = "", sekme = "ozet", wa = WHATSAPP_MERKEZ, ara = "" } = {}) {
  if (!METIN[dil]) dil = "tr";
  ara = typeof ara === "string" ? ara.slice(0, 40) : "";
  sekme = sekmeSec(sekme);
  const m = { ...METIN[dil], ...EK[dil], ...EK2[dil] };
  const tekKart = (ikon, baslik, metin, ek = "") =>
    `<section class="kart"><div class="hero">${simge(ikon)}<div><h2>${baslik}</h2></div></div><p class="aciklama">${metin}</p>${ek}</section>`;
  const sssBag = (s) => `<p class="sss-bag"><a href="${kacis(sekmeAdresi(adres, s, dil))}">${s === "sss" ? m.sss_b : m.sekme.ozet}</a></p>`;
  let govde;
  let cubuk = "";
  const tamKayit = kayit && kayit.d !== "iptal";
  if (!tamKayit && sekme === "sss") {
    // Kayıt yok/iptal: yalnız SSS açılabilir; geri bağlantısı ana karta döner.
    govde = sssSekmesi(m, waNumarasi(kayit && subeBul(kayit.s), wa), kayit ? kayit.k : "") + sssBag("ozet");
  } else if (!kayit) {
    govde = tekKart("soru", m.yok_b, m.yok_m,
      yedek ? `<p class="alt">${m.yedek} <a href="${kacis(yedek)}?dil=${dil}">${m.yedek_a}</a></p>` : "") + sssBag("sss");
  } else if (kayit.d === "iptal") {
    govde = tekKart("carpi", m.iptal_b, m.iptal_m) + sssBag("sss");
  } else {
    cubuk = sekmeCubugu(sekme, dil, adres, m);
    const kg = kayit.kg;
    if (sekme === "islemler") govde = islemlerKarti(kayit, dil, m);
    else if (sekme === "subeler") govde = subelerSekmesi(kayit, m, wa, { adres, dil, ara });
    else if (sekme === "sss") govde = sssSekmesi(m, waNumarasi(subeBul(kayit.s), wa), kayit.k);
    else {
      // Kargoda adımlar: alındı → terzide → kargolandı → teslim. Mağazadan teslimde: alındı → terzide → hazır → teslim.
      const adimlar = kg ? ["alindi", "terzide", "kargo", "teslim"] : ADIMLAR;
      let sira = ADIMLAR.indexOf(kayit.d);
      // Kargoya hazırlanırken (mağaza sunucusu fiş kesilirken bunu gönderir) sıra kayıt durumundan gelir:
      // alındı → 0, terzide → 1, hazır → 2 (kargo adımı güncel). Yolda/teslimde kargo adımı ya da teslim.
      const hazirlaniyor = kg && kg.d === "hazirlaniyor" && kayit.d !== "teslim";
      if (kg && !hazirlaniyor) sira = kayit.d === "teslim" || kg.d === "teslim" ? 3 : 2;
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
      // Ürün daha terziye girmemiş/terzideyken başlık kayıt durumunu söyler; kargo başlığı ancak hazır olunca.
      const baslik = hazirlaniyor && sira < 2 ? m.durum[kayit.d]
        : kg && kayit.d !== "teslim" ? m.kd[kg.d] : kg && kayit.d === "teslim" ? m.kd.teslim : m.durum[kayit.d];
      const zaman = zamanMetni(kayit.ts, dil);
      govde = `<section class="kart durum" aria-labelledby="durum"><div class="hero">${simge(bitti ? "tik" : "saat")}
      <div><h2 id="durum">${baslik}</h2><p class="alt">${m.baslik}</p></div></div>
      <ol class="adimlar" aria-label="${m.baslik}">${lis}</ol>${not ? `<p class="not">${not}</p>` : ""}
      ${zaman ? `<p class="guncelleme">${m.guncelleme}: <time datetime="${new Date(kayit.ts * 1000).toISOString()}">${kacis(zaman)}</time></p>` : ""}</section>
      ${kargoKarti(kg, dil, m)}${subeKarti(kayit.s, { kargoda: !!kg, durum: kayit.d, m })}${fisKarti(kayit, dil, m)}`;
    }
  }
  const yon = SAG_SOL.has(dil) ? "rtl" : "ltr";
  // Sabit "?" SSS düğmesi: SSS sekmesindeyken gösterilmez.
  const fab = sekme === "sss" ? ""
    : `<a class="sss-fab" href="${kacis(sekmeAdresi(adres, "sss", dil))}" aria-label="${kacis(m.sss_dugme)}"><span aria-hidden="true">?</span></a>`;
  return `<!doctype html><html lang="${dil}" dir="${yon}"><head><meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1,viewport-fit=cover"><meta name="robots" content="noindex,nofollow">
<meta name="referrer" content="no-referrer"><meta name="color-scheme" content="light dark"><title>${m.baslik} · Terzi Adem Altun</title>
<style>${CSS}</style></head><body><div class="sayfa">
<header><div class="marka"><span class="logo" aria-hidden="true">TA</span><h1>Terzi Adem Altun</h1></div>${dilCekmecesi(dil, adres, sekme)}</header>
${cubuk}<main>${govde}</main>
<footer><span>Terzi Adem Altun · ${m.baslik}</span></footer>
</div>${fab}</body></html>`;
}

export const CSS = `:root{color-scheme:light dark;--zemin:#f6f3ee;--kart:#fff;--yazi:#29251f;--ince:#665c50;--vurgu:#8a5a2b;--vurgu-yazi:#fff;--cizgi:#e4ddd2;--not:#efe8dc;--golge:0 1px 2px rgba(41,37,31,.06),0 8px 24px rgba(41,37,31,.06)}
@media (prefers-color-scheme:dark){:root{--zemin:#14181c;--kart:#1d2329;--yazi:#e8ecf3;--ince:#b7c0cc;--vurgu:#d7a46b;--vurgu-yazi:#14181c;--cizgi:#323a43;--not:#262d34;--golge:none}}
*{box-sizing:border-box}
body{margin:0;background:var(--zemin);color:var(--yazi);font:16px/1.5 system-ui,-apple-system,"Segoe UI",Roboto,"Noto Sans","Noto Sans Arabic",sans-serif;-webkit-text-size-adjust:100%}
.sayfa{max-width:480px;margin:0 auto;padding:16px 16px calc(88px + env(safe-area-inset-bottom))}
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
li.su .nokta{background:var(--vurgu);border-color:var(--vurgu);box-shadow:0 0 0 5px var(--not)}
li.su .nokta::after{content:"";width:10px;height:10px;border-radius:50%;background:var(--vurgu-yazi)}
li.sonra .nokta{background:var(--cizgi);border-color:var(--cizgi)}
.metin{flex:1;min-width:0;display:flex;flex-direction:column;padding-top:4px}
.metin .ad{font-weight:600;line-height:24px}.metin small{color:var(--ince);font-size:13.5px;line-height:1.4}
li.su .metin .ad{font-weight:700;font-size:17px}li.sonra .metin .ad{color:var(--ince);font-weight:500}
.etiket{flex:none;margin-top:4px;padding:2px 10px;border-radius:999px;background:var(--not);color:var(--vurgu);font-size:12.5px;font-weight:700;line-height:20px}
.not{margin:18px 0 0;padding:12px 14px;border-inline-start:4px solid var(--vurgu);border-radius:8px;background:var(--not);font-size:15px}
.guncelleme{margin:16px 0 0;padding-top:12px;border-top:1px solid var(--cizgi);color:var(--ince);font-size:13px}
h3,h2.kb{margin:0 0 12px;font-size:13px;line-height:1.5;letter-spacing:.06em;text-transform:uppercase;color:var(--ince)}
:lang(ar) h3,:lang(fa) h3,:lang(zh) h3,:lang(hi) h3,:lang(ar) h2.kb,:lang(fa) h2.kb,:lang(zh) h2.kb,:lang(hi) h2.kb{text-transform:none;letter-spacing:0;font-size:15px}
.bos{margin:8px 0 0;color:var(--ince)}.adres-yok{margin:6px 0 0;color:var(--ince);font-size:14px}.adres-yok a{font-weight:700}
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
.sekmeler{display:flex;gap:4px;margin-top:14px;padding:4px;overflow-x:auto;scrollbar-width:none;background:var(--kart);border:1px solid var(--cizgi);border-radius:14px;box-shadow:var(--golge)}
.sekmeler a{flex:1 1 auto;min-width:0;min-height:44px;display:flex;align-items:center;justify-content:center;padding:4px 8px;border-radius:10px;color:var(--ince);font-size:14.5px;font-weight:600;line-height:1.2;text-align:center;text-decoration:none;white-space:nowrap}
.sekmeler::-webkit-scrollbar{display:none}
:lang(el) .sekmeler a,:lang(ru) .sekmeler a,:lang(bg) .sekmeler a,:lang(de) .sekmeler a,:lang(ro) .sekmeler a{padding:4px 5px;font-size:13px}
.sekmeler a[aria-current="page"]{background:var(--vurgu);color:var(--vurgu-yazi);font-weight:700}
.sekmeler a:not([aria-current]):hover{background:var(--not);color:var(--yazi)}
.rozet{display:inline-flex;align-items:center;gap:6px;margin:0 0 10px;padding:4px 12px;border-radius:999px;background:var(--vurgu);color:var(--vurgu-yazi);font-size:13px;font-weight:700;line-height:20px}
.kart.burada{border-top:4px solid var(--vurgu)}
.dugme.wa{gap:8px}
.dugme.kucuk{flex:0 1 auto;min-height:44px;padding:0 14px;font-size:14.5px}
details.cekmece>summary{list-style:none;display:flex;align-items:center;justify-content:space-between;gap:12px;min-height:48px;cursor:pointer}details.cekmece>summary::-webkit-details-marker{display:none}details.cekmece>summary h2{margin:0}details.cekmece>summary .sayi{margin-inline-start:auto;font-size:13px;font-weight:700;color:var(--ince);border:1px solid var(--cizgi);border-radius:999px;padding:2px 10px}details.cekmece>summary::after{content:"";flex:none;width:8px;height:8px;border-inline-end:2px solid var(--vurgu);border-bottom:2px solid var(--vurgu);transform:rotate(45deg) translateY(-2px)}details.cekmece[open]>summary::after{transform:rotate(-135deg) translateY(-2px)}details.cekmece>summary:focus-visible{outline:3px solid var(--vurgu);outline-offset:2px;border-radius:8px}form.sube-ara{display:flex;gap:8px;margin:12px 0}form.sube-ara input[type=search]{flex:1;min-width:0;min-height:48px;padding:0 14px;border:1px solid var(--cizgi);border-radius:12px;background:var(--kart);color:inherit;font:inherit;font-size:16px}form.sube-ara input:focus-visible,form.sube-ara button:focus-visible{outline:3px solid var(--vurgu);outline-offset:2px}form.sube-ara button{display:inline-flex;align-items:center;gap:6px;min-height:48px;padding:0 16px;border:0;border-radius:12px;background:var(--vurgu);color:var(--vurgu-yazi);font:inherit;font-weight:700;cursor:pointer}.gizli{position:absolute;width:1px;height:1px;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap}.ara-sonuc{margin:0 0 8px;font-size:14px}.sube-not{font-size:14px;color:var(--vurgu);margin:4px 0 0}ul.subeler .tel{font-size:14px;color:var(--ince);margin:2px 0 0}
ul.subeler{list-style:none;margin:0;padding:0}
ul.subeler li{padding:12px 0;border-bottom:1px solid var(--cizgi)}ul.subeler li:first-child{padding-top:0}ul.subeler li:last-child{border-bottom:0;padding-bottom:0}
ul.subeler .sube-ad{font-size:16px;font-weight:600}ul.subeler .adres{font-size:14px;color:var(--ince)}ul.subeler .dugmeler{margin-top:10px}
.sss-b{font-size:21px;margin-bottom:10px}
.sss-liste{display:grid;gap:8px}
details.sss{border:1px solid var(--cizgi);border-radius:12px;background:var(--kart)}
details.sss>summary{list-style:none;display:flex;align-items:center;justify-content:space-between;gap:12px;min-height:48px;padding:10px 14px;font-weight:600;cursor:pointer}
details.sss>summary::-webkit-details-marker{display:none}
details.sss>summary::after{content:"";flex:none;width:8px;height:8px;border-inline-end:2px solid var(--vurgu);border-bottom:2px solid var(--vurgu);transform:rotate(45deg) translateY(-2px)}
details.sss[open]>summary::after{transform:rotate(-135deg) translateY(-2px)}
details.sss[open]{background:var(--not);border-color:transparent}
details.sss p{margin:0;padding:0 14px 14px;color:var(--yazi);font-size:15px}
details.sss>summary:focus-visible,.sss-fab:focus-visible{outline:3px solid var(--vurgu);outline-offset:2px}
.sss-bag{margin:14px 0 0;text-align:center}.sss-bag a{display:inline-flex;align-items:center;min-height:44px;padding:0 12px;font-weight:700}
.sss-fab{position:fixed;inset-inline-end:16px;bottom:calc(16px + env(safe-area-inset-bottom));z-index:10;width:56px;height:56px;border-radius:50%;display:grid;place-items:center;background:var(--vurgu);color:var(--vurgu-yazi);font:700 26px/1 Georgia,"Times New Roman",serif;text-decoration:none;box-shadow:0 6px 18px rgba(0,0,0,.22),0 0 0 4px var(--zemin)}
@media print{.sss-fab,.sekmeler,details.dil{display:none}.sayfa{padding-bottom:8px}}
footer{margin-top:22px;padding:14px 0 8px;text-align:center;color:var(--ince);font-size:13px}
@media (prefers-reduced-motion:no-preference){details.dil>summary::after{transition:transform .15s}
li.su .nokta{animation:nabiz 2.4s ease-out infinite}
@keyframes nabiz{0%{box-shadow:0 0 0 0 color-mix(in srgb,var(--vurgu) 45%,transparent)}70%{box-shadow:0 0 0 10px transparent}100%{box-shadow:0 0 0 5px var(--not)}}}`;
