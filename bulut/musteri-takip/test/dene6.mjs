// v6: fişsiz teslim onayı (karekodla imza). Aç → müşteri formu → şifreli kayıt → sunucunun okuması/çözmesi → kapat.
// Şifre çözme node:crypto ile, sözleşmedeki X25519 + HKDF-SHA256 + AES-256-GCM'ye birebir. Yalnız sahte veri.
import { generateKeyPairSync, sign, createPublicKey, diffieHellman, hkdfSync, createDecipheriv } from "node:crypto";
import fs from "node:fs";
import assert from "node:assert";
import worker from "../cloudflare/worker.mjs";
import { ONAY_METIN, ONAY_ANAHTARLAR, ONAY_RE, telNormalize, adDogrula, onaySayfa } from "../ortak/onay.mjs";
import { DILLER } from "../ortak/diller.mjs";
import { SUBELER } from "../ortak/ek.mjs";
import { onayDeposuKur, onayYolu, vercelOnay } from "../vercel/lib/onay-depo.mjs";

const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const pubHex = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
const x = generateKeyPairSync("x25519");
const pkHex = x.publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");

// KV taklidi: expiration'ı saklar, süresi geçeni okurken yok sayar (Cloudflare KV gibi).
const kv = new Map();
const env = {
  GENEL_ANAHTAR: pubHex,
  TAKIP: {
    async get(k) { const v = kv.get(k); if (!v || (v.exp && v.exp <= Date.now() / 1000)) return null; return JSON.parse(v.deger); },
    async put(k, deger, sec = {}) { assert(!sec.expiration || sec.expiration >= Math.floor(Date.now() / 1000) + 60, "KV: expiration en az 60 sn ileri"); kv.set(k, { deger, exp: sec.expiration }); },
    async delete(k) { kv.delete(k); },
  },
};
const simdi = () => Math.floor(Date.now() / 1000);
const imzala = (g) => sign(null, Buffer.from(g), privateKey).toString("hex");
const iste = (yol, sec = {}) => worker.fetch(new Request("https://x" + yol, sec), env);
const ac = (nesne, imza) => { const g = JSON.stringify(nesne); return iste("/api/onay/ac", { method: "POST", body: g, headers: { "x-taa-imza": imza ?? imzala(g) } }); };
const kapat = (nesne, imza) => { const g = JSON.stringify(nesne); return iste("/api/onay/kapat", { method: "POST", body: g, headers: { "x-taa-imza": imza ?? imzala(g) } }); };
const oku = (t, ts = String(simdi()), imza) => iste("/api/onay/" + t, { headers: { "x-taa-zaman": ts, "x-taa-imza": imza ?? imzala(`onay_oku:${t}:${ts}`) } });
const gonder = (t, alanlar, basliklar = {}) => iste("/o/" + t, { method: "POST", body: new URLSearchParams(alanlar).toString(),
  headers: { "content-type": "application/x-www-form-urlencoded", ...basliklar } });
const sayfa = async (t, q = "", basliklar = {}) => { const r = await iste("/o/" + t + q, { headers: basliklar }); return { r, h: await r.text() }; };
const ALFABE = "abcdefghjkmnpqrstuvwxyz23456789";
const yeniT = () => Array.from({ length: 20 }, () => ALFABE[Math.floor(Math.random() * ALFABE.length)]).join("");
const acGovde = (t, ek = {}) => ({ v: 1, tur: "onay_ac", t, ts: simdi(), sil: simdi() + 900, s: "MRK01", n: 3, pk: pkHex, ...ek });

/** Sunucu tarafının yapacağı çözme (node:crypto). */
function coz(t, { e, iv, c }) {
  const eB = Buffer.from(e, "hex");
  const pkB = Buffer.from(pkHex, "hex");
  const gen = createPublicKey({ key: Buffer.concat([Buffer.from("302a300506032b656e032100", "hex"), eB]), format: "der", type: "spki" });
  const paylasilan = diffieHellman({ privateKey: x.privateKey, publicKey: gen });
  const anahtar = Buffer.from(hkdfSync("sha256", paylasilan, Buffer.concat([eB, pkB]), Buffer.from("taa-teslim-onay-v1"), 32));
  const cB = Buffer.from(c, "base64");
  const d = createDecipheriv("aes-256-gcm", anahtar, Buffer.from(iv, "hex"));
  d.setAAD(Buffer.from(t, "utf8"));
  d.setAuthTag(cB.subarray(-16));
  return JSON.parse(Buffer.concat([d.update(cB.subarray(0, -16)), d.final()]).toString("utf8"));
}

let gecti = 0;
const t_ = async (ad, f) => { try { await f(); } catch (e) { e.message = `[${ad}] ${e.message}`; throw e; } gecti++; };

const T = yeniT();
await t_("imzasız/bozuk aç reddedilir", async () => {
  assert.equal((await ac(acGovde(T), "")).status, 401);
  assert.equal((await ac(acGovde(T), "00".repeat(64))).status, 401);
  for (const [ad, ek] of [["n 0", { n: 0 }], ["n 51", { n: 51 }], ["n kesirli", { n: 1.5 }], ["sil > ts+900", { sil: simdi() + 901 }],
    ["sil <= ts", { sil: simdi() }], ["pk kısa", { pk: "ab" }], ["t 'l' içerir", { t: "l".repeat(20) }], ["t 16", { t: T.slice(0, 16) }],
    ["s uzun", { s: "A".repeat(17) }], ["s <", { s: "<b>" }], ["tur", { tur: "onay_kapat" }], ["v 2", { v: 2 }], ["ts eski", { ts: simdi() - 4000, sil: simdi() - 3500 }],
    ["i 2", { i: 2 }], ["i true", { i: true }], ["i '1'", { i: "1" }], ["i null", { i: null }]]) {
    assert.equal((await ac(acGovde(T, ek))).status, 400, ad);
  }
  assert.equal(kv.size, 0);
});
await t_("aç: kayıt bekliyor olarak saklanır, KV silme zamanı = sil, aynı t → 409", async () => {
  const g = acGovde(T);
  const r = await ac(g);
  assert.equal(r.status, 200, await r.clone().text());
  const v = kv.get("onay:" + T);
  assert.deepEqual(JSON.parse(v.deger), { t: T, s: "MRK01", n: 3, pk: pkHex, sil: g.sil, durum: "bekliyor", p: 0, i: 0 }, "i yoksa 0 (eski istemci)");
  assert.equal(v.exp, g.sil);
  assert.equal((await ac(acGovde(T))).status, 409);
});
await t_("müşteri sayfası: şube adı, başlık, ürün sayısı, form alanları, betiksiz, CSP", async () => {
  const { r, h } = await sayfa(T, "", { "accept-language": "tr-TR" });
  assert.equal(r.status, 200);
  assert.match(r.headers.get("content-security-policy"), /form-action 'self'/);
  assert(h.includes(`Terzi Adem Altun · ${SUBELER.MRK01.ad}`), "şube adı");
  assert(h.includes("Teslim onayı") && h.includes("3 ürün") && h.includes("Ürünlerimi eksiksiz teslim aldım") && h.includes(">Gönder<"));
  assert(h.includes(`<form class="onay" method="post" action="/o/${T}">`));
  assert(/<input type="text" name="ad"[^>]*autocomplete="name"/.test(h), "ad autocomplete=name");
  assert(/<input type="tel" name="tel"[^>]*inputmode="tel" autocomplete="tel"/.test(h), "tel inputmode/autocomplete");
  assert(/<input type="checkbox" name="onay" value="1" required/.test(h), "zorunlu onay kutusu");
  assert(!/<script/i.test(h), "betik yok");
  assert(h.includes(`href="/o/${T}?dil=en"`), "dil çekmecesi");
  const ar = await sayfa(T, "?dil=ar");
  assert(ar.h.includes('dir="rtl"') && ar.h.includes(ONAY_METIN.ar.b) && ar.r.headers.get("set-cookie").startsWith("dil=ar"));
  assert((await sayfa(T, "", { cookie: "dil=fa" })).h.includes('lang="fa" dir="rtl"'));
});
await t_("oku: onaydan önce bekliyor", async () => {
  const r = await oku(T);
  assert.equal(r.status, 200);
  assert.deepEqual(await r.json(), { durum: "bekliyor" });
});
await t_("hatalı form: 400, hatalar gösterilir, değerler kaçışlanır, hiçbir şey şifrelenmez", async () => {
  const r = await gonder(T, { ad: '<script>alert(1)</script>', tel: "12", dil: "tr" });
  const h = await r.text();
  assert.equal(r.status, 400);
  assert(h.includes(ONAY_METIN.tr.h_ad) && h.includes(ONAY_METIN.tr.h_tel) && h.includes(ONAY_METIN.tr.h_kutu));
  assert(!h.includes("<script>alert") && h.includes("&lt;script&gt;alert(1)&lt;/script&gt;"), "kaçış");
  assert(h.includes('aria-invalid="true" aria-describedby="h-ad"'));
  const k = JSON.parse(kv.get("onay:" + T).deger);
  assert.equal(k.durum, "bekliyor"); assert.equal(k.p, 1); assert(!kv.get("onay:" + T).deger.includes("script"));
  const r2 = await gonder(T, { ad: "Deneme Kişi", tel: "0555 000 00 01", dil: "en" });
  assert.equal(r2.status, 400); assert((await r2.text()).includes(ONAY_METIN.en.h_kutu), "dil formdan");
});
const UA = "Mozilla/5.0 (Linux; Android 14; Sahte Telefon) " + "x".repeat(300);
await t_("geçerli onay: teşekkür sayfası, kayıtta düz metin yok", async () => {
  const r = await gonder(T, { ad: "  Ayşe   Deneme-Örnek ", tel: "0 (555) 000-00-01", onay: "1", dil: "tr" },
    { "user-agent": UA, "cf-connecting-ip": "203.0.113.7", "x-forwarded-for": "198.51.100.1" });
  const h = await r.text();
  assert.equal(r.status, 200);
  assert(h.includes(ONAY_METIN.tr.tesekkur_b) && h.includes(ONAY_METIN.tr.tesekkur_m));
  const ham = kv.get("onay:" + T).deger;
  const k = JSON.parse(ham);
  assert.deepEqual(Object.keys(k).sort(), ["c", "durum", "e", "iv", "n", "s", "sil", "t"]);
  assert.equal(k.durum, "onaylandi"); assert.match(k.e, /^[0-9a-f]{64}$/); assert.match(k.iv, /^[0-9a-f]{24}$/);
  for (const duz of ["Ayşe", "Deneme", "555", "Mozilla", "Android", "203.0.113", "198.51"]) assert(!ham.includes(duz), "düz metin: " + duz);
});
let ilkC;
await t_("oku + node:crypto ile çözme: düz metin birebir", async () => {
  const r = await oku(T);
  assert.equal(r.status, 200);
  const j = await r.json();
  assert.deepEqual(Object.keys(j).sort(), ["c", "durum", "e", "iv"]);
  assert.equal(j.durum, "onaylandi");
  ilkC = j.c;
  const P = coz(T, j);
  assert.deepEqual(Object.keys(P), ["ad", "tel", "ua", "ip", "zaman", "dil"]);
  assert.equal(P.ad, "Ayşe Deneme-Örnek"); assert.equal(P.tel, "+905550000001"); assert.equal(P.ua, UA.slice(0, 200));
  assert.equal(P.ip, "203.0.113.7"); assert.equal(P.dil, "tr"); assert(Math.abs(P.zaman - simdi()) <= 5);
  assert.throws(() => coz(yeniT(), j), "aad = t: başka t ile çözülemez");
});
await t_("ikinci onay reddedilir (kayıt değişmez), sayfa 'Onayınız alındı'", async () => {
  const r = await gonder(T, { ad: "Başka Kişi", tel: "05550000002", onay: "1", dil: "tr" });
  assert.equal(r.status, 200); assert((await r.text()).includes(ONAY_METIN.tr.alindi_b));
  assert.equal(JSON.parse(kv.get("onay:" + T).deger).c, ilkC);
  const { h } = await sayfa(T, "?dil=de");
  assert(h.includes(ONAY_METIN.de.alindi_b) && !h.includes("<form"));
});
await t_("oku: imzasız, eski zamanlı, ileri zamanlı, bozuk imza, bilinmeyen t reddedilir", async () => {
  assert.equal((await iste("/api/onay/" + T)).status, 401);
  assert.equal((await oku(T, String(simdi()), "")).status, 401);
  assert.equal((await oku(T, String(simdi() - 301))).status, 401);
  assert.equal((await oku(T, String(simdi() + 301))).status, 401);
  assert.equal((await oku(T, "abc")).status, 401);
  const ts = String(simdi());
  assert.equal((await oku(T, ts, imzala(`onay_oku:${yeniT()}:${ts}`))).status, 401, "başka t'nin imzası");
  assert.equal((await oku(yeniT())).status, 404);
  assert.equal((await oku("kisa")).status, 404);
});
await t_("kapat: imzasız red, imzalı silme; sonra oku 404, sayfa 'süresi doldu'", async () => {
  assert.equal((await kapat({ v: 1, tur: "onay_kapat", t: T, ts: simdi() }, "00".repeat(64))).status, 401);
  assert.equal((await kapat({ v: 1, tur: "onay_kapat", t: T, ts: simdi() - 4000 })).status, 400);
  assert(kv.has("onay:" + T));
  assert.equal((await kapat({ v: 1, tur: "onay_kapat", t: T, ts: simdi() })).status, 200);
  assert(!kv.has("onay:" + T));
  assert.equal((await oku(T)).status, 404);
  const { r, h } = await sayfa(T);
  assert.equal(r.status, 404); assert(h.includes("Bu karekodun süresi doldu, mağaza görevlisinden yenisini isteyin"));
  assert((await sayfa("bilinmeyen")).h.includes(ONAY_METIN.tr.doldu));
});
await t_("süre dolumu: sil geçince sayfa doldu, oku 404, POST kabul edilmez", async () => {
  const t2 = yeniT();
  assert.equal((await ac(acGovde(t2, { sil: simdi() + 30 }))).status, 200);
  assert.equal(kv.get("onay:" + t2).exp >= simdi() + 60, true, "KV en az 60 sn");
  const gercek = Date.now;
  Date.now = () => gercek() + 31_000; // KV henüz silmedi, ama sil geçti
  try {
    assert(kv.has("onay:" + t2));
    const r = await gonder(t2, { ad: "Deneme Kişi", tel: "05550000003", onay: "1" });
    assert.equal(r.status, 404); assert((await r.text()).includes(ONAY_METIN.tr.doldu));
    assert.equal((await oku(t2)).status, 404);
    assert(!kv.has("onay:" + t2), "süresi geçen okurken silinir");
  } finally { Date.now = gercek; }
});
await t_("hız sınırı: aynı t'ye 5 POST'tan sonra 429 (geçerli veriyle de)", async () => {
  const t3 = yeniT();
  assert.equal((await ac(acGovde(t3, { n: 1 }))).status, 200);
  for (let i = 0; i < 5; i++) assert.equal((await gonder(t3, { ad: "x", tel: "1" })).status, 400);
  const r = await gonder(t3, { ad: "Deneme Kişi", tel: "05550000004", onay: "1", dil: "ru" });
  assert.equal(r.status, 429); assert((await r.text()).includes(ONAY_METIN.ru.cok));
  assert.equal(JSON.parse(kv.get("onay:" + t3).deger).durum, "bekliyor");
});
await t_("telefon normalizasyonu", () => {
  for (const [g, b] of [["05551234567", "+905551234567"], ["0555 123 45 67", "+905551234567"], ["+90 555 123 45 67", "+905551234567"],
    ["905551234567", "+905551234567"], ["5551234567", "+905551234567"], ["(0216) 000 00 00", "+902160000000"], ["+49 30 1234567", "+49301234567"],
    ["0049 30 1234567", "+49301234567"], ["+1 202 555 0100", "+12025550100"]]) assert.equal(telNormalize(g), b, g);
  for (const g of ["", "12", "0555123456", "055512345678", "+90555123456", "+1234567", "+1234567890123456", "abc", "+90 05551234567", "<0555>", "+0 1234 5678"]) {
    assert.equal(telNormalize(g), null, g);
  }
});
await t_("ad doğrulama", () => {
  for (const [g, b] of [["Ali Veli", "Ali Veli"], ["  Jean-Luc   O'Neil ", "Jean-Luc O'Neil"], ["O’Brien", "O'Brien"], ["张伟", "张伟"],
    ["محمد علی", "محمد علی"], ["राम कुमार", "राम कुमार"], ["J. R. Smith", "J. R. Smith"]]) assert.equal(adDogrula(g), b, g);
  for (const g of ["", "A", "a".repeat(61), "Ali1", "<b>Ali</b>", "Ali@Veli", "...", "--"]) assert.equal(adDogrula(g), null, g);
});
await t_("13 dil metin tamlığı ve sayfa üretimi", () => {
  assert.equal(Object.keys(ONAY_METIN).length, 13);
  for (const [d] of DILLER) {
    assert(ONAY_METIN[d], d);
    for (const k of ONAY_ANAHTARLAR) assert(typeof ONAY_METIN[d][k] === "string" && ONAY_METIN[d][k].trim(), d + "." + k);
    assert.deepEqual(Object.keys(ONAY_METIN[d]).sort(), [...ONAY_ANAHTARLAR].sort(), d + " fazladan/eksik anahtar");
    assert(ONAY_METIN[d].urun.includes("{n}"), d + ".urun {n}");
    for (const hal of ["form", "doldu", "alindi", "tesekkur", "cok"]) {
      const h = onaySayfa(hal, { dil: d, t: T, sube: "MRK01", n: 2, hatalar: hal === "form" ? { ad: true, tel: true, kutu: true, genel: true } : {} });
      assert(h.includes(`<html lang="${d}" dir="${d === "ar" || d === "fa" ? "rtl" : "ltr"}">`), d + " " + hal);
      assert(!h.includes("undefined"), d + " " + hal + " undefined");
    }
  }
  assert(ONAY_RE.test(T) && !ONAY_RE.test("o".repeat(20)));
});
await t_("şube adı kaçışlanır, bilinmeyen şube yalnız marka", () => {
  const tablo = { v: 1, ts: 1, wa: "", subeler: { ZZ01: { ad: '<img src=x onerror=1>', adres: "", tel: "", saat: "", wa: "", not: "" } } };
  const h = onaySayfa("form", { t: T, sube: "ZZ01", n: 1, tablo });
  assert(!h.includes("<img") && h.includes("&lt;img src=x onerror=1&gt;"));
  const b = onaySayfa("form", { t: T, sube: "__proto__", n: 1 });
  assert(b.includes('<bdi lang="tr">Terzi Adem Altun</bdi>'));
});
await t_("düz metin loglanmaz: onay kodunda console yok; Vercel yeniden yazımları", () => {
  for (const f of ["../ortak/onay.mjs", "../ortak/imza.mjs", "../cloudflare/worker.mjs", "../vercel/api/onay.mjs", "../vercel/lib/depo.mjs", "../vercel/lib/onay-depo.mjs"]) {
    assert(!fs.readFileSync(new URL(f, import.meta.url), "utf8").includes("console."), f);
  }
  const v = JSON.parse(fs.readFileSync(new URL("../vercel/vercel.json", import.meta.url), "utf8"));
  const yeniden = Object.fromEntries(v.rewrites.map((r) => [r.source, r.destination]));
  assert.equal(yeniden["/o/:t"], "/api/onay?islem=sayfa&t=:t");
  assert.equal(yeniden["/api/onay/:t"], "/api/onay?islem=oku&t=:t");
  const sira = v.rewrites.map((r) => r.source);
  assert(sira.indexOf("/api/onay/ac") < sira.indexOf("/api/onay/:t") && sira.indexOf("/api/onay/kapat") < sira.indexOf("/api/onay/:t"));
  for (const f of ["onay.mjs", "takip.mjs", "imza.mjs"]) {
    const o = fs.readFileSync(new URL("../ortak/" + f, import.meta.url), "utf8");
    assert.equal(fs.readFileSync(new URL("../cloudflare/" + f, import.meta.url), "utf8"), o, "kopyala.sh: cloudflare/" + f);
    assert.equal(fs.readFileSync(new URL("../vercel/lib/" + f, import.meta.url), "utf8"), o, "kopyala.sh: vercel/lib/" + f);
  }
});
await t_("çok dilli adlar: Hintçe (virama), Farsça (ZWNJ), harekeli Arapça geçer; yön işaretleri geçmez (Rust ile aynı liste)", async () => {
  // Aynı adlar crates/taa-fis/src/lib.rs ve crates/taa-fis-sunucu/src/teslim_onay.rs testlerinde de sınanır.
  for (const g of ["राहुल शर्मा", "علی\u200Cرضا محمدی", "مُحَمَّد عَلِي", "Zoë Ünlü-Çelik", "ਗੁਰਪ੍ਰੀਤ ਸਿੰਘ", "क्षत्रिय\u200D नाम"]) {
    assert.equal(adDogrula(g), g.normalize("NFC"), g);
  }
  for (const g of ["Zey\u200Enep Kaya", "Zey\u202Enep Kaya", "Zey\u2066nep Kaya", "Zey\u200Bnep Kaya", "\u200C\u200C"]) {
    assert.equal(adDogrula(g), null, JSON.stringify(g));
  }
  // U+FEFF JS'te boşluk sayılır: boşluğa çevrilir (sunucuya giden adda görünmez karakter kalmaz).
  assert.equal(adDogrula("Zey\uFEFFnep"), "Zey nep");
  // uçtan uca: Farsça ad onaylanır ve şifreli kayıttan aynen çözülür
  const t = yeniT();
  assert.equal((await ac(acGovde(t))).status, 200);
  const r = await gonder(t, { ad: "علی\u200Cرضا محمدی", tel: "+98 912 000 1234", onay: "1", dil: "fa" });
  assert.equal(r.status, 200);
  const k = JSON.parse(kv.get("onay:" + t).deger);
  assert.equal(coz(t, k).ad, "علی\u200Cرضا محمدی");
  await kapat({ v: 1, tur: "onay_kapat", t, ts: simdi() });
});
await t_("çerçeveleme yasak: frame-ancestors 'none' ve X-Frame-Options DENY", async () => {
  const t = yeniT();
  assert.equal((await ac(acGovde(t))).status, 200);
  for (const r of [(await sayfa(t)).r, (await sayfa(yeniT())).r]) {
    assert.match(r.headers.get("content-security-policy"), /frame-ancestors 'none'/);
    assert.equal(r.headers.get("x-frame-options"), "DENY");
  }
  await kapat({ v: 1, tur: "onay_kapat", t, ts: simdi() });
});
await t_("Vercel: onay-depo.mjs sahte Blob ile aç → form → onay → oku → kapat → temizle; yol çözümü", async () => {
  // Sahte @vercel/blob: put/head/del/list ve CDN fetch'i (gerçek Vercel denenmedi; README notu).
  const bloblar = new Map();
  const secenekler = [];
  const blob = {
    async put(yol, govde, sec) { secenekler.push(sec); bloblar.set(yol, { govde, uploadedAt: new Date() }); return { url: "https://blob.invalid/" + yol }; },
    async head(yol) { if (!bloblar.has(yol)) throw new Error("BlobNotFoundError"); return { url: "https://blob.invalid/" + yol, uploadedAt: bloblar.get(yol).uploadedAt }; },
    async del(url) { bloblar.delete(url.replace("https://blob.invalid/", "")); },
    async list({ prefix }) { return { blobs: [...bloblar].filter(([y]) => y.startsWith(prefix)).map(([y, b]) => ({ url: "https://blob.invalid/" + y, pathname: y, uploadedAt: b.uploadedAt })), hasMore: false }; },
  };
  const getir = async (url) => {
    const y = url.replace("https://blob.invalid/", "").replace(/\?v=\d+$/, "");
    assert(/\?v=\d+$/.test(url), "CDN önbelleği ?v= ile aşılır");
    const b = bloblar.get(y);
    return b ? new Response(b.govde, { status: 200 }) : new Response("", { status: 404 });
  };
  const depo = { ...onayDeposuKur(blob, getir), tablo: async () => null };
  const v = (yol, sec = {}) => vercelOnay(new Request("https://v.invalid" + yol, sec), depo, pubHex);
  // yol çözümü: asıl yol ve yeniden yazılmış sorgu aynı sonucu verir
  const t = yeniT();
  for (const [a, b] of [[`/o/${t}`, `/api/onay?islem=sayfa&t=${t}`], [`/api/onay/${t}`, `/api/onay?islem=oku&t=${t}`], ["/api/onay/ac", "/api/onay?islem=ac"], ["/api/onay/kapat", "/api/onay?islem=kapat"]]) {
    assert.deepEqual(onayYolu("https://v.invalid" + a), onayYolu("https://v.invalid" + b), a);
  }
  assert.equal((await v("/api/onay?islem=yok")).status, 404);
  const g = JSON.stringify(acGovde(t));
  assert.equal((await v("/api/onay/ac", { method: "POST", body: g, headers: { "x-taa-imza": imzala(g) } })).status, 200);
  assert(secenekler.every((s) => s.cacheControlMaxAge >= 60 && s.allowOverwrite && !s.addRandomSuffix), "Blob seçenekleri");
  assert.equal((await v("/api/onay/ac", { method: "POST", body: g, headers: { "x-taa-imza": imzala(g) } })).status, 409);
  const ts = String(simdi());
  const okuV = () => v(`/api/onay/${t}`, { headers: { "x-taa-zaman": ts, "x-taa-imza": imzala(`onay_oku:${t}:${ts}`) } });
  assert.deepEqual(await (await okuV()).json(), { durum: "bekliyor" });
  const f = await v(`/api/onay?islem=sayfa&t=${t}`, { method: "POST", body: new URLSearchParams({ ad: "Deneme Kişi", tel: "0532 000 45 67", onay: "1" }).toString(),
    headers: { "content-type": "application/x-www-form-urlencoded", "user-agent": "Deneme/1.0", "x-forwarded-for": "203.0.113.5, 10.0.0.1" } });
  assert.equal(f.status, 200);
  const ham = bloblar.get(`onay/${t}.json`).govde;
  for (const yasak of ["Deneme", "4567", "203.0.113", "pk"]) assert(!ham.includes(yasak), "Blob'da düz metin: " + yasak);
  const o = await (await okuV()).json();
  assert.equal(o.durum, "onaylandi");
  const d = coz(t, o);
  assert.equal(d.ip, "203.0.113.5");
  assert.equal(d.tel, "+905320004567");
  const kg = JSON.stringify({ v: 1, tur: "onay_kapat", t, ts: simdi() });
  assert.equal((await v("/api/onay/kapat", { method: "POST", body: kg, headers: { "x-taa-imza": imzala(kg) } })).status, 200);
  assert(!bloblar.has(`onay/${t}.json`));
  assert.equal((await okuV()).status, 404);
  // temizle: süresi geçen (sil geçmiş) ve çok eski yüklenmiş kayıtlar silinir, geçerli kalır
  const s0 = simdi();
  bloblar.set("onay/a.json", { govde: JSON.stringify({ sil: s0 - 1 }), uploadedAt: new Date() });
  bloblar.set("onay/b.json", { govde: JSON.stringify({ sil: s0 + 500 }), uploadedAt: new Date() });
  bloblar.set("onay/c.json", { govde: JSON.stringify({ sil: s0 + 500 }), uploadedAt: new Date((s0 - 2000) * 1000) });
  bloblar.set("takip/x.json", { govde: "{}", uploadedAt: new Date(0) });
  assert.equal(await depo.temizle(s0), 2);
  assert.deepEqual([...bloblar.keys()].sort(), ["onay/b.json", "takip/x.json"]);
});
console.log(`v6 teslim onayı testleri geçti (${gecti} denetim)`);
