// v5: imzalı şube tablosu (POST /api/subeler), birleştirme, diğer şubeler çekmecesi ve betiksiz arama. Yalnız sahte veri.
import { generateKeyPairSync, sign } from "node:crypto";
import worker from "../cloudflare/worker.mjs";
import { sayfa, subeTablosu, dogrulaSubeler } from "../ortak/takip.mjs";
import { SUBELER, EK2 } from "../ortak/ek.mjs";
import { DILLER } from "../ortak/diller.mjs";
import assert from "node:assert";
const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const pubHex = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
const kv = new Map();
const env = { GENEL_ANAHTAR: pubHex, TAKIP: { async get(k) { const v = kv.get(k); return v ? JSON.parse(v) : null; }, async put(k, v) { kv.set(k, v); } } };
const now = Math.floor(Date.now() / 1000);
const imzala = (g) => sign(null, Buffer.from(g), privateKey).toString("hex");
const gonder = (yol, nesne, imza) => { const g = JSON.stringify(nesne);
  return worker.fetch(new Request("https://x" + yol, { method: "POST", body: g, headers: { "x-taa-imza": imza ?? imzala(g) } }), env); };
const al = async (q) => worker.fetch(new Request("https://x/t/abcdefghjkmnpqrs" + q), env);
let gecti = 0; const t_ = async (ad, f) => { await f(); gecti++; };

const tablo = { v: 1, tur: "subeler", ts: now, wa: "905550001122",
  subeler: { PND01: { ad: "İstanbul Pendik Mağaza", adres: "Yeni Sahte Cad. No:1, Pendik / İstanbul", tel: "+902160000000", saat: "10:00–20:00" },
    YNI01: { ad: "Sahte Yeni Mağaza", adres: "Deneme Mah. 1. Sk., Kartal / İstanbul", tel: "+905550000001", not: "Otopark var" } } };

await t_("imzasız/bozuk tablo reddedilir", async () => {
  assert.equal((await gonder("/api/subeler", tablo, "00".repeat(64))).status, 401);
  assert.equal((await gonder("/api/subeler", { ...tablo, tur: "x" })).status, 400);
  assert.equal((await gonder("/api/subeler", { ...tablo, subeler: { "kötü": { ad: "x" } } })).status, 400);
  assert.equal((await gonder("/api/subeler", { ...tablo, subeler: { PND01: { ad: "<script>" } } })).status, 400);
  assert.equal((await gonder("/api/subeler", { ...tablo, subeler: { PND01: { ad: "x", tel: "abc" } } })).status, 400);
  assert.equal((await gonder("/api/subeler", { ...tablo, ts: now - 40 * 86400 })).status, 400);
  assert(!kv.has("_subeler")); });
await t_("geçerli tablo kaydedilir, eskisi yenisini ezemez", async () => {
  const r = await gonder("/api/subeler", tablo); assert.equal(r.status, 200); assert.equal((await r.json()).sube, 2);
  assert.equal((await gonder("/api/subeler", { ...tablo, ts: now - 100 })).status, 409);
  assert.equal(JSON.parse(kv.get("_subeler")).wa, "905550001122"); });
await t_("birleştirme: dolu alan üstün, boş alan eskisini korur, yeni şube eklenir, tel_g üretilir", async () => {
  const b = subeTablosu((await dogrulaSubeler(JSON.stringify(tablo), imzala(JSON.stringify(tablo)), pubHex)).tablo);
  assert.equal(b.PND01.adres, "Yeni Sahte Cad. No:1, Pendik / İstanbul"); assert.equal(b.PND01.tel_g, "0216 000 00 00");
  assert.equal(b.PND01.saat, "10:00–20:00"); assert.equal(b.MRK01.adres, SUBELER.MRK01.adres); assert(b.YNI01);
  assert.equal(subeTablosu(null), SUBELER); assert.notEqual(SUBELER.PND01.adres, b.PND01.adres, "dosyadaki tablo değişmez"); });

// Sayfa: takip kaydı Merkez'de; tablo KV'de.
const u = [{ u: "pantolon" }];
const kayit = { v: 3, k: "abcdefghjkmnpqrs", d: "terzide", s: "MRK01", t: "2026-11-29", g: "2026-11-26", h: "17:00", u, ts: now, sil: now + 86400 };
assert.equal((await gonder("/api/yaz", kayit)).status, 200);
await t_("şubeler sekmesi: çekmece kapalı, sayı, arama formu, imzalı tablo kullanılır", async () => {
  const r = await al("?sekme=subeler&dil=tr"); const h = await r.text();
  assert.match(r.headers.get("content-security-policy"), /form-action 'self'/);
  assert(h.includes('<details class="cekmece"><summary><h2 id="diger" class="kb">Diğer şubelerimiz</h2><span class="sayi">15</span>'), "kapalı çekmece + 15 şube");
  assert(h.includes('<form class="sube-ara" method="get" action="/t/abcdefghjkmnpqrs" role="search">') && h.includes('name="sekme" value="subeler"') && h.includes('name="dil" value="tr"'));
  assert(h.includes("Yeni Sahte Cad. No:1") && h.includes("Sahte Yeni Mağaza") && h.includes("Otopark var"));
  assert(h.includes("Outlet mağazamız hemen yanında"), "Merkez notu");
  assert(h.includes("https://wa.me/905550001122?text="), "tablodaki merkez WhatsApp"); });
await t_("arama: aksansız/büyük harf eşleşir, çekmece açık, temizle bağlantısı", async () => {
  const h = await (await al("?sekme=subeler&dil=tr&ara=SANLIURFA")).text();
  assert(h.includes('<details class="cekmece" open>'));
  const liste = h.slice(h.indexOf('<ul class="subeler">'));
  assert(liste.includes("Şanlıurfa Mağaza") && !liste.includes("Bursa Mağaza"));
  assert(h.includes('value="SANLIURFA"') && h.includes(EK2.tr.ara_temizle));
  const ilce = await (await al("?sekme=subeler&dil=tr&ara=nilufer")).text();
  assert(ilce.slice(ilce.indexOf('<ul class="subeler">')).includes("Bursa Mağaza"), "ilçe ile");
  const yok = await (await al("?sekme=subeler&dil=en&ara=zzzz")).text();
  assert(yok.includes(EK2.en.ara_yok) && !yok.includes('<ul class="subeler">')); });
await t_("arama metni kaçışlanır ve 40 karakterle sınırlı", async () => {
  const h = await (await al("?sekme=subeler&dil=tr&ara=" + encodeURIComponent('"><b>x</b>' + "a".repeat(80)))).text();
  assert(!h.includes("<b>x</b>") && h.includes("&quot;&gt;&lt;b&gt;")); assert(!h.includes("a".repeat(41))); });
await t_("13 dilde arama metinleri dolu", () => { for (const [d] of DILLER) for (const k of ["ara_b", "ara_ph", "ara_d", "ara_yok", "ara_temizle"]) assert(EK2[d][k] && EK2[d][k].trim(), d + "." + k); });
await t_("tablo yokken dosyadaki tablo, sayfa sonrası etkin tablo sıfırlanır", () => {
  const h = sayfa(kayit, { dil: "tr", sekme: "subeler", adres: "/t/x" });
  assert(h.includes(SUBELER.PND01.adres) && !h.includes("Sahte Yeni Mağaza")); });
console.log(`v5 şube tablosu/arama testleri geçti (${gecti} denetim)`);
