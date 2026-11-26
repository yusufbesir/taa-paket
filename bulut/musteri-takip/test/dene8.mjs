// v8: teslim onayı sağlamlaştırma. (1) eşzamanlı POST/aç: ilk onay ezilmez, 5 deneme sınırı aşılmaz (yaz-bir-kez anahtarlar);
// (2) kapatılan belirteç sil'e kadar "kapandi" kalır, yeniden açılamaz; (3) aç/kapat gövdesi 1 KiB sınırı (okumadan önce);
// (4) ts penceresi (aç/kapat/oku iki yönlü ±600; sil bulut saatine göre kırpılır, toplam geçerlilik ≤ 15 dk); (5) PNG tRNS
// saydamlığı beyaz zemine; (6) "oz" kalem özeti AAD'de (aad = t + ":" + oz); (7) depo hataları (okunamadı ≠ yok, 503),
// geri okuma hatası, kapat yazma sayısı, Vercel temizliği, KV bilinen sınırı; ortak Rust/JS vektörleri.
// Yalnız sahte veri; ağ gerekmez.
import { generateKeyPairSync, sign, createPublicKey, createPrivateKey, diffieHellman, hkdfSync, createDecipheriv } from "node:crypto";
import fs from "node:fs";
import zlib from "node:zlib";
import assert from "node:assert";
import worker from "../cloudflare/worker.mjs";
import { ONAY_METIN, IMZALI_GOVDE_EN_COK, onayAad, onayIsle, dogrulaAc, dogrulaKapat, dogrulaOku, birKezYaz, KAZANDI, KAYBETTI, BILINMIYOR } from "../ortak/onay.mjs";
import { imzaIcerik } from "../ortak/imza.mjs";
import { onayDeposuKur, vercelOnay } from "../vercel/lib/onay-depo.mjs";

const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const pubHex = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
const x = generateKeyPairSync("x25519");
const pkHex = x.publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");

// KV taklidi: her işlem bir olay döngüsü turu bekler (eşzamanlı istekler gerçekten iç içe geçer; get+put atomik değil).
// Gecikme her işlem için aynıdır (yaz-bir-kez + geri okuma bu koşulda tek kazanan verir; gerçek KV'de garanti yok, README).
const tur = () => new Promise((r) => setImmediate(r));
const kv = new Map();
const env = {
  GENEL_ANAHTAR: pubHex,
  TAKIP: {
    async get(k) { await tur(); const v = kv.get(k); if (!v || (v.exp && v.exp <= Date.now() / 1000)) return null; return JSON.parse(v.deger); },
    async put(k, deger, sec = {}) { await tur(); kv.set(k, { deger, exp: sec.expiration }); },
    async delete(k) { await tur(); kv.delete(k); },
  },
};
const simdi = () => Math.floor(Date.now() / 1000);
const imzala = (g) => sign(null, Buffer.from(g), privateKey).toString("hex");
const iste = (yol, sec = {}) => worker.fetch(new Request("https://x" + yol, sec), env);
const ac = (nesne) => { const g = JSON.stringify(nesne); return iste("/api/onay/ac", { method: "POST", body: g, headers: { "x-taa-imza": imzala(g) } }); };
const kapat = (t, ts = simdi()) => { const g = JSON.stringify({ v: 1, tur: "onay_kapat", t, ts }); return iste("/api/onay/kapat", { method: "POST", body: g, headers: { "x-taa-imza": imzala(g) } }); };
const oku = (t, ts = String(simdi())) => iste("/api/onay/" + t, { headers: { "x-taa-zaman": ts, "x-taa-imza": imzala(`onay_oku:${t}:${ts}`) } });
const gonder = (t, alanlar) => iste("/o/" + t, { method: "POST", body: new URLSearchParams(alanlar).toString(), headers: { "content-type": "application/x-www-form-urlencoded" } });
const ALFABE = "abcdefghjkmnpqrstuvwxyz23456789";
const yeniT = () => Array.from({ length: 20 }, () => ALFABE[Math.floor(Math.random() * ALFABE.length)]).join("");
const acGovde = (t, ek = {}) => ({ v: 1, tur: "onay_ac", t, ts: simdi(), sil: simdi() + 900, s: "MRK01", n: 2, pk: pkHex, ...ek });
const gecerli = (i) => ({ ad: "Deneme Kişi " + "abcdefghijklmnop"[i], tel: "0555 000 00 " + String(10 + i), onay: "1", dil: "tr" });
function coz(aad, { e, iv, c }) {
  const eB = Buffer.from(e, "hex");
  const gen = createPublicKey({ key: Buffer.concat([Buffer.from("302a300506032b656e032100", "hex"), eB]), format: "der", type: "spki" });
  const anahtar = Buffer.from(hkdfSync("sha256", diffieHellman({ privateKey: x.privateKey, publicKey: gen }), Buffer.concat([eB, Buffer.from(pkHex, "hex")]), Buffer.from("taa-teslim-onay-v1"), 32));
  const cB = Buffer.from(c, "base64");
  const d = createDecipheriv("aes-256-gcm", anahtar, Buffer.from(iv, "hex"));
  d.setAAD(Buffer.from(aad, "utf8"));
  d.setAuthTag(cB.subarray(-16));
  return JSON.parse(Buffer.concat([d.update(cB.subarray(0, -16)), d.final()]).toString("utf8"));
}
const denemeSayisi = (t) => [...kv.keys()].filter((a) => a.startsWith(`onay:${t}:deneme:`)).length;

/** Sahte Blob (rastgele gecikmeli): put(allowOverwrite:false) gerçek Blob gibi koşullu ve atomiktir. */
function sahteBlob() {
  const bloblar = new Map();
  const bekle = () => new Promise((r) => setTimeout(r, Math.floor(Math.random() * 4)));
  const blob = {
    async put(y, g, sec = {}) {
      await bekle();
      if (sec.allowOverwrite === false && bloblar.has(y)) throw new Error("BlobAlreadyExistsError");
      bloblar.set(y, { govde: g, uploadedAt: new Date() });
      return { url: "https://blob.invalid/" + y };
    },
    async head(y) { await bekle(); if (!bloblar.has(y)) throw new Error("BlobNotFoundError"); return { url: "https://blob.invalid/" + y }; },
    async del(u) { await bekle(); bloblar.delete(u.replace("https://blob.invalid/", "")); },
    async list() { return { blobs: [], hasMore: false }; },
  };
  const getir = async (u) => {
    await bekle();
    const b = bloblar.get(u.replace("https://blob.invalid/", "").replace(/\?v=\d+$/, ""));
    return b ? new Response(b.govde) : new Response("", { status: 404 });
  };
  const depo = { ...onayDeposuKur(blob, getir), tablo: async () => null };
  const v = (yol, sec = {}) => vercelOnay(new Request("https://v.invalid" + yol, sec), depo, pubHex);
  return {
    bloblar, v,
    ac: (nesne) => { const g = JSON.stringify(nesne); return v("/api/onay/ac", { method: "POST", body: g, headers: { "x-taa-imza": imzala(g) } }); },
    kapat: (t) => { const g = JSON.stringify({ v: 1, tur: "onay_kapat", t, ts: simdi() }); return v("/api/onay/kapat", { method: "POST", body: g, headers: { "x-taa-imza": imzala(g) } }); },
    oku: (t) => { const ts = String(simdi()); return v(`/api/onay/${t}`, { headers: { "x-taa-zaman": ts, "x-taa-imza": imzala(`onay_oku:${t}:${ts}`) } }); },
    gonder: (t, alanlar) => v(`/o/${t}`, { method: "POST", body: new URLSearchParams(alanlar).toString(), headers: { "content-type": "application/x-www-form-urlencoded" } }),
  };
}

let gecti = 0;
const t_ = async (ad, f) => { try { await f(); } catch (e) { e.message = `[${ad}] ${e.message}`; throw e; } gecti++; };
/** Yanıtları duruma göre sayar; 200 yanıtlarında teşekkür / "alındı" ayrılır. */
async function say(yanitlar) {
  const s = { tesekkur: [], alindi: 0, cok: 0, hata: 0, diger: [] };
  for (let i = 0; i < yanitlar.length; i++) {
    const r = yanitlar[i], h = await r.text();
    if (r.status === 200 && h.includes(ONAY_METIN.tr.alindi_b)) s.alindi++;
    else if (r.status === 200 && h.includes(ONAY_METIN.tr.tesekkur_m)) s.tesekkur.push(i);
    else if (r.status === 429) s.cok++;
    else if (r.status === 400) s.hata++;
    else s.diger.push(r.status);
  }
  return s;
}

// ---------------------------------------------------------------- 1) atomiklik
await t_("KV: 10 eşzamanlı geçerli onay → tek teşekkür, 5 deneme sınırı aşılmaz, kayıt kazananın", async () => {
  for (let tekrar = 0; tekrar < 5; tekrar++) {
    const t = yeniT();
    assert.equal((await ac(acGovde(t))).status, 200);
    const s = await say(await Promise.all(Array.from({ length: 10 }, (_, i) => gonder(t, gecerli(i)))));
    assert.deepEqual(s.diger, []);
    assert.equal(s.tesekkur.length, 1, "tek kazanan");
    assert.equal(s.cok, 5, "5'ten fazla deneme işlenmedi");
    assert.equal(s.alindi, 4, "geç kalanlar 'Onayınız alındı' görür");
    assert.equal(denemeSayisi(t), 5);
    const j = await (await oku(t)).json();
    assert.equal(j.durum, "onaylandi");
    assert.equal(coz(t, j).ad, gecerli(s.tesekkur[0]).ad, "kayıttaki onay teşekkür alanınki (ezilmedi)");
    assert.equal(JSON.parse(kv.get("onay:" + t).deger).durum, "bekliyor", "ana kayıt POST'ta yazılmaz");
  }
});
await t_("KV: geçerli onay + eşzamanlı hatalı POST'lar → onay ezilmez; sonra 6. POST 429", async () => {
  const t = yeniT();
  assert.equal((await ac(acGovde(t))).status, 200);
  const alanlar = [{ ad: "x", tel: "1" }, { ad: "y", tel: "2" }, gecerli(3), { ad: "z", tel: "3" }, { ad: "Deneme Kişi", tel: "12" }];
  const s = await say(await Promise.all(alanlar.map((a) => gonder(t, a))));
  assert.deepEqual([s.tesekkur, s.hata, s.cok], [[2], 4, 0]);
  assert.equal(denemeSayisi(t), 5);
  assert.equal(coz(t, await (await oku(t)).json()).ad, gecerli(3).ad);
  const r = await gonder(t, gecerli(4));
  assert.equal(r.status, 200); assert((await r.text()).includes(ONAY_METIN.tr.alindi_b), "onaylanmış t'de deneme harcanmaz");
  const t2 = yeniT();
  assert.equal((await ac(acGovde(t2))).status, 200);
  const s2 = await say(await Promise.all(Array.from({ length: 8 }, () => gonder(t2, { ad: "x", tel: "1" }))));
  assert.deepEqual([s2.hata, s2.cok], [5, 3]);
  assert.equal((await gonder(t2, gecerli(1))).status, 429, "geçerli veriyle de");
  assert.equal(denemeSayisi(t2), 5);
});
await t_("KV: aynı t'ye 6 eşzamanlı aç → tek 200, diğerleri 409", async () => {
  const t = yeniT();
  const g = Array.from({ length: 6 }, (_, i) => acGovde(t, { n: i + 1 }));
  const kodlar = await Promise.all(g.map((b) => ac(b).then((r) => r.status)));
  assert.equal(kodlar.filter((k) => k === 200).length, 1, kodlar.join(","));
  assert.equal(kodlar.filter((k) => k === 409).length, 5);
  assert.equal(JSON.parse(kv.get("onay:" + t).deger).n, kodlar.indexOf(200) + 1, "kayıt 200 alanın");
});
await t_("Vercel (rastgele gecikmeli sahte Blob, koşullu put): eşzamanlı onay ve aç", async () => {
  for (let tekrar = 0; tekrar < 5; tekrar++) {
    const B = sahteBlob();
    const t = yeniT();
    assert.equal((await B.ac(acGovde(t))).status, 200);
    const s = await say(await Promise.all(Array.from({ length: 10 }, (_, i) => B.gonder(t, gecerli(i)))));
    assert.deepEqual(s.diger, []);
    assert.deepEqual([s.tesekkur.length, s.cok, s.alindi], [1, 5, 4]);
    assert.equal([...B.bloblar.keys()].filter((y) => y.startsWith(`onay/${t}.deneme-`)).length, 5);
    assert.equal(coz(t, await (await B.oku(t)).json()).ad, gecerli(s.tesekkur[0]).ad);
    const t2 = yeniT();
    const kodlar = await Promise.all(Array.from({ length: 6 }, (_, i) => B.ac(acGovde(t2, { n: i + 1 })).then((r) => r.status)));
    assert.deepEqual(kodlar.filter((k) => k === 200).length, 1, kodlar.join(","));
  }
});

// ---------------------------------------------------------------- 2) kapandi
await t_("kapat: belirteç sil'e kadar 'kapandi'; imzalı aç yeniden açamaz (409); oku/sayfa 404; süre bitince açılabilir", async () => {
  const t = yeniT();
  const g = acGovde(t, { sil: simdi() + 300 });
  assert.equal((await ac(g)).status, 200);
  assert.equal((await kapat(t)).status, 200);
  const k = JSON.parse(kv.get("onay:" + t).deger);
  assert.deepEqual(k, { t, sil: g.sil, durum: "kapandi" });
  assert.equal(kv.get("onay:" + t).exp, g.sil, "KV silme zamanı = kayıt sil");
  assert.equal((await ac(acGovde(t))).status, 409, "kapandi yeniden açılamaz");
  assert.equal((await oku(t)).status, 404);
  const r = await iste("/o/" + t);
  assert.equal(r.status, 404); assert((await r.text()).includes(ONAY_METIN.tr.doldu));
  assert.equal((await gonder(t, gecerli(0))).status, 404, "kapandi'ya POST işlenmez");
  assert.equal(denemeSayisi(t), 0);
  assert.equal((await kapat(t)).status, 200, "ikinci kapat da 200, iz aynı");
  assert.deepEqual(JSON.parse(kv.get("onay:" + t).deger), k);
  // pencere bitince (sil geçti) aynı t yeniden açılabilir
  const gercek = Date.now;
  Date.now = () => gercek() + 301_000;
  try { assert.equal((await ac(acGovde(t))).status, 200); } finally { Date.now = gercek; }
});
await t_("onaylanmış belirteç: aç 409; kapat sonrası da 409, şifreli sonuç silinir", async () => {
  const t = yeniT();
  assert.equal((await ac(acGovde(t))).status, 200);
  assert.equal((await gonder(t, gecerli(0))).status, 200);
  assert.equal((await ac(acGovde(t))).status, 409);
  assert.equal((await kapat(t)).status, 200);
  assert(!kv.has(`onay:${t}:sonuc`));
  assert.equal((await ac(acGovde(t))).status, 409);
  assert.equal((await oku(t)).status, 404);
});
await t_("kayıt yokken kapat iz bırakmaz (süresi geçen karekodlar için gelen kapat'lar birikmez)", async () => {
  const t = yeniT();
  assert.equal((await kapat(t)).status, 200);
  assert(![...kv.keys()].some((a) => a.includes(t)));
});
await t_("Vercel: kapat → kapandi izi, aç 409", async () => {
  const B = sahteBlob();
  const t = yeniT();
  assert.equal((await B.ac(acGovde(t))).status, 200);
  assert.equal((await B.gonder(t, gecerli(0))).status, 200);
  assert.equal((await B.kapat(t)).status, 200);
  assert.equal(JSON.parse(B.bloblar.get(`onay/${t}.json`).govde).durum, "kapandi");
  assert(!B.bloblar.has(`onay/${t}.sonuc.json`));
  assert.equal((await B.ac(acGovde(t))).status, 409);
  assert.equal((await B.oku(t)).status, 404);
});

// ---------------------------------------------------------------- 3) gövde sınırı
await t_("aç/kapat gövdesi 1 KiB: content-length ve akış sınırı, okunmadan 413", async () => {
  assert.equal(IMZALI_GOVDE_EN_COK, 1024);
  const t = yeniT();
  const buyuk = JSON.stringify(acGovde(t, { dolgu: "x".repeat(1100) }));
  for (const yol of ["/api/onay/ac", "/api/onay/kapat"]) {
    const r = await iste(yol, { method: "POST", body: buyuk, headers: { "x-taa-imza": imzala(buyuk) } });
    assert.equal(r.status, 413, yol);
    // content-length olmayan akış: sınırda kesilir, tümü okunmaz
    let okunan = 0;
    const akim = new ReadableStream({ pull(c) { okunan += 512; if (okunan > 1024 * 64) c.close(); else c.enqueue(new Uint8Array(512).fill(32)); } });
    const ist = new Request("https://x" + yol, { method: "POST", body: akim, duplex: "half", headers: { "x-taa-imza": "00".repeat(64) } });
    assert.equal(ist.headers.get("content-length"), null);
    assert.equal((await worker.fetch(ist, env)).status, 413, yol + " akış");
    assert(okunan <= 1024 + 3 * 512, "akış erken kesildi: " + okunan);
  }
  // yalan content-length (küçük) da akış sınırına takılır; tam 1024 baytlık gövde kabul edilir
  const g = acGovde(t);
  let metin = JSON.stringify(g);
  metin = metin.slice(0, -1) + "," + JSON.stringify("d") + ":" + JSON.stringify("x".repeat(1024 - metin.length - 7)) + "}";
  assert.equal(Buffer.byteLength(metin), 1024);
  assert.equal((await iste("/api/onay/ac", { method: "POST", body: metin, headers: { "x-taa-imza": imzala(metin) } })).status, 200);
  assert.equal(kv.size > 0, true);
});

// ---------------------------------------------------------------- 4) ts penceresi
// Rust'ın gerçek aç gövdesi: ts = bilgisayar saati, sil = ts + 900 (teslim_onay.rs onay_ac_ic). Bilgisayar saati buluttan
// `kayma` sn ileri/geri. Sınırlar dogrulaAc'a sabit `simdi` verilerek sınanır (saniye sınırında oynamaz).
const S = 1_795_000_000;
const rustAc = (kayma, ek = {}) => { const ts = S + kayma; return JSON.stringify(acGovde(yeniT(), { ts, sil: ts + 900, ...ek })); };
const dAc = (g) => dogrulaAc(g, imzala(g), pubHex, S);
await t_("ts: Rust gövdesi (sil = ts+900) bilgisayar saati ±600 sn kaymada kabul, sil bulut saatine kırpılır", async () => {
  for (const kayma of [-600, -300, -5, 0, 1, 5, 30, 60, 61, 300, 599, 600]) {
    const r = await dAc(rustAc(kayma));
    assert(!r.hata, `kayma ${kayma}: ${r.hata}`);
    assert.equal(r.kayit.sil, Math.min(S + kayma + 900, S + 900), `kayma ${kayma}: sil`);
    assert(r.kayit.sil <= S + 900, "toplam geçerlilik bulut saatiyle 15 dk'yı aşmaz");
  }
  for (const kayma of [601, 1000, -601, -2000]) assert.equal((await dAc(rustAc(kayma))).hata, "zaman geçersiz", `kayma ${kayma}`);
  // sil kuralları: ts < sil <= ts+900, sil > şimdi
  for (const [ad, ek, hata] of [
    ["sil = ts", { ts: S, sil: S }, "silinme zamanı geçersiz"],
    ["sil = ts+901", { ts: S, sil: S + 901 }, "silinme zamanı geçersiz"],
    ["sil geçmişte (ts geri 600, sil = şimdi)", { ts: S - 600, sil: S }, "silinme zamanı geçersiz"],
    ["ts geri 600, sil = ts+900", { ts: S - 600, sil: S + 300 }, null],
    ["ts geri 100, sil = ts+300", { ts: S - 100, sil: S + 200 }, null],
  ]) {
    const g = JSON.stringify(acGovde(yeniT(), ek));
    const r = await dAc(g);
    assert.equal(r.hata ?? null, hata, ad);
    if (!hata) assert.equal(r.kayit.sil, ek.sil, ad + ": kırpılmaz");
  }
  // kapat ve oku da ±600 (iki yönlü; aç'ın kabul ettiği her kaymada akış uçtan uca çalışır)
  const kp = (ts) => { const g = JSON.stringify({ v: 1, tur: "onay_kapat", t: "abcdefghijkmnpqrst23", ts }); return dogrulaKapat(g, imzala(g), pubHex, S); };
  for (const [d, iyi] of [[600, true], [601, false], [-600, true], [-601, false], [60, true], [300, true]]) assert.equal(!(await kp(S + d)).hata, iyi, "kapat " + d);
  const ok = (ts) => dogrulaOku("abcdefghijkmnpqrst23", String(ts), imzala(`onay_oku:abcdefghijkmnpqrst23:${ts}`), pubHex, S);
  for (const [d, iyi] of [[600, true], [601, false], [-600, true], [-601, false], [61, true], [301, true]]) assert.equal(!!(await ok(S + d)).tamam, iyi, "oku " + d);
});
await t_("ts: saati ileri/geri bilgisayarla uçtan uca aç → onay → oku → kapat (KV), kayıttaki sil bulut saatine kırpılır", async () => {
  for (const kayma of [1, 5, 30, 60, 300, 590, -590]) {
    const t = yeniT();
    const ts = simdi() + kayma;
    assert.equal((await ac(acGovde(t, { ts, sil: ts + 900 }))).status, 200, "aç kayma " + kayma);
    assert(JSON.parse(kv.get("onay:" + t).deger).sil <= simdi() + 900, "sil kırpıldı");
    assert.equal((await gonder(t, gecerli(0))).status, 200);
    const j = await (await oku(t, String(simdi() + kayma))).json();
    assert.equal(j.durum, "onaylandi", "oku kayma " + kayma);
    assert.equal((await kapat(t, simdi() + kayma)).status, 200);
    assert.equal(JSON.parse(kv.get("onay:" + t).deger).durum, "kapandi", "kapat kayma " + kayma);
  }
});

// ---------------------------------------------------------------- 5) tRNS
/** PNG kurucu (yalnız test): renk 0 (gri) ya da 2 (RGB), 8 bit; f(x,y) → gri değer; ekler IDAT'tan önce/sonra eklenir. */
function png(en, boy, renk, f, { once = [], sonra = [] } = {}) {
  const kanal = renk === 2 ? 3 : renk === 6 ? 4 : 1;
  const ham = Buffer.alloc((en * kanal + 1) * boy);
  for (let y = 0; y < boy; y++) for (let xx = 0; xx < en; xx++) {
    const v = f(xx, y), o = y * (en * kanal + 1) + 1 + xx * kanal;
    for (let k = 0; k < kanal; k++) ham[o + k] = k === 3 ? 255 : v;
  }
  const parca = (tur, veri) => { const b = Buffer.alloc(12 + veri.length); b.writeUInt32BE(veri.length, 0); b.write(tur, 4, "latin1"); veri.copy(b, 8); b.writeUInt32BE(zlib.crc32(b.subarray(4, 8 + veri.length)), 8 + veri.length); return b; };
  const ihdr = Buffer.alloc(13); ihdr.writeUInt32BE(en, 0); ihdr.writeUInt32BE(boy, 4); ihdr[8] = 8; ihdr[9] = renk;
  return Buffer.concat([Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]), parca("IHDR", ihdr),
    ...once.map(([t, v]) => parca(t, v)), parca("IDAT", zlib.deflateSync(ham)), ...sonra.map(([t, v]) => parca(t, v)), parca("IEND", Buffer.alloc(0))]).toString("base64");
}
const cizgi = (koyu) => (xx, y) => (Math.abs(y - 60 - Math.round(20 * Math.sin(xx / 10))) <= 1 && xx > 30 && xx < 270 ? koyu : 255);
const u16 = (...d) => { const b = Buffer.alloc(d.length * 2); d.forEach((v, i) => b.writeUInt16BE(v, i * 2)); return b; };
await t_("imzaIcerik: tRNS saydam renk beyaz zemine karışır (saydam 'mürekkep' boş sayılır)", async () => {
  assert.equal((await imzaIcerik(png(300, 120, 0, cizgi(17)))).tamam, true, "tRNS'siz gri imza geçer");
  assert.equal((await imzaIcerik(png(300, 120, 0, cizgi(17), { once: [["tRNS", u16(17)]] }))).hata, "bos", "gri: çizgi saydam");
  assert.equal((await imzaIcerik(png(300, 120, 0, cizgi(17), { once: [["tRNS", u16(255)]] }))).tamam, true, "zemin saydam: beyaz kalır, çizgi koyu");
  assert.equal((await imzaIcerik(png(300, 120, 0, cizgi(17), { once: [["tRNS", u16(273)]] }))).tamam, true, "16 bit değer 8 bit pikselle eşleşmez");
  assert.equal((await imzaIcerik(png(300, 120, 2, cizgi(17)))).tamam, true, "RGB imza geçer");
  assert.equal((await imzaIcerik(png(300, 120, 2, cizgi(17), { once: [["tRNS", u16(17, 17, 17)]] }))).hata, "bos", "RGB: çizgi saydam");
  assert.equal((await imzaIcerik(png(300, 120, 2, cizgi(17), { once: [["tRNS", u16(17, 17, 18)]] }))).tamam, true, "RGB: eşleşmeyen renk");
  // tamamen siyah + saydam siyah = boş beyaz sayfa (eskiden "kara tuval" diye de reddediliyordu; şimdi saydam olarak)
  assert.equal((await imzaIcerik(png(300, 120, 0, () => 0, { once: [["tRNS", u16(0)]] }))).hata, "bos");
  // geçersiz tRNS: tekrar, IDAT'tan sonra, yanlış boy, alfalı renk türünde
  for (const [ad, b64] of [
    ["tekrar", png(300, 120, 0, cizgi(17), { once: [["tRNS", u16(1)], ["tRNS", u16(2)]] })],
    ["IDAT'tan sonra", png(300, 120, 0, cizgi(17), { sonra: [["tRNS", u16(17)]] })],
    ["gri, 6 bayt", png(300, 120, 0, cizgi(17), { once: [["tRNS", u16(17, 17, 17)]] })],
    ["RGB, 2 bayt", png(300, 120, 2, cizgi(17), { once: [["tRNS", u16(17)]] })],
    ["RGBA'da tRNS", png(300, 120, 6, cizgi(17), { once: [["tRNS", u16(17, 17, 17)]] })],
  ]) assert.equal((await imzaIcerik(b64)).hata, "gecersiz", ad);
});
await t_("i=1: saydam çizgili PNG imza olarak kabul edilmez (400, h_imza)", async () => {
  const t = yeniT();
  assert.equal((await ac(acGovde(t, { i: 1 }))).status, 200);
  const r = await gonder(t, { ...gecerli(0), imza: "data:image/png;base64," + png(300, 120, 0, cizgi(17), { once: [["tRNS", u16(17)]] }) });
  assert.equal(r.status, 400); assert((await r.text()).includes(ONAY_METIN.tr.h_imza));
  assert.equal((await gonder(t, { ...gecerli(0), imza: "data:image/png;base64," + png(300, 120, 0, cizgi(17)) })).status, 200);
});

// ---------------------------------------------------------------- 6) oz (kalem özeti) AAD'de
const OZ = "0123456789abcdef".repeat(4);
await t_("oz: kayda yazılır, P'ye girmez; aad = t + ':' + oz (t ya da başka oz ile çözülmez)", async () => {
  assert.equal(onayAad("t1", ""), "t1"); assert.equal(onayAad("t1", undefined), "t1"); assert.equal(onayAad("t1", OZ), "t1:" + OZ);
  const t = yeniT();
  assert.equal((await ac(acGovde(t, { oz: OZ }))).status, 200);
  assert.equal(JSON.parse(kv.get("onay:" + t).deger).oz, OZ);
  assert.equal((await gonder(t, gecerli(0))).status, 200);
  const j = await (await oku(t)).json();
  assert.deepEqual(Object.keys(j).sort(), ["c", "durum", "e", "iv"], "oku yanıtı değişmedi");
  const P = coz(`${t}:${OZ}`, j);
  assert.deepEqual(Object.keys(P), ["ad", "tel", "ua", "ip", "zaman", "dil"], "oz şifreli P'de yok");
  assert.equal(P.ad, gecerli(0).ad);
  assert.throws(() => coz(t, j), "oz'suz aad ile çözülmez");
  assert.throws(() => coz(`${t}:${"f".repeat(64)}`, j), "başka oz ile çözülmez");
  assert(!kv.get(`onay:${t}:sonuc`).deger.includes(OZ), "sonuç kaydında oz yok (ana kayıtta)");
  // oz yok → eski davranış (aad = t)
  const t2 = yeniT();
  assert.equal((await ac(acGovde(t2))).status, 200);
  assert(!("oz" in JSON.parse(kv.get("onay:" + t2).deger)));
  assert.equal((await gonder(t2, gecerli(1))).status, 200);
  assert.equal(coz(t2, await (await oku(t2)).json()).ad, gecerli(1).ad);
});
await t_("oz biçimi: 64 küçük harf hex; büyük harf, kısa, uzun, sayı, null, boş reddedilir", async () => {
  for (const [ad, oz] of [["büyük harf", OZ.toUpperCase()], ["63", OZ.slice(1)], ["65", OZ + "0"], ["sayı", 1], ["null", null], ["boş", ""], ["g", "g".repeat(64)]]) {
    assert.equal((await ac(acGovde(yeniT(), { oz }))).status, 400, ad);
  }
});
await t_("Vercel: oz kaydı ve aad", async () => {
  const B = sahteBlob();
  const t = yeniT();
  assert.equal((await B.ac(acGovde(t, { oz: OZ }))).status, 200);
  assert.equal(JSON.parse(B.bloblar.get(`onay/${t}.json`).govde).oz, OZ);
  assert.equal((await B.gonder(t, gecerli(2))).status, 200);
  assert.equal(coz(`${t}:${OZ}`, await (await B.oku(t)).json()).ad, gecerli(2).ad);
});

// ---------------------------------------------------------------- 7) depo hataları ve yazma sayısı
/** Bellek içi depo (onayIsle'ye doğrudan); `bozuk` işlevleri çağrı adına göre hata/boş döndürür. */
function bellekDepo() {
  const m = new Map();
  const sayac = { oku: 0, yaz: 0, sil: 0 };
  const bozuk = {};
  const calis = async (ad, f) => { if (bozuk[ad]) { const r = bozuk[ad](); if (r instanceof Error) throw r; if (r !== undefined) return r; } return f(); };
  const depo = {
    oku: (x) => calis("oku", () => { sayac.oku++; return m.get(x) ?? null; }),
    yaz: (k) => calis("yaz", () => { sayac.yaz++; m.set(k.t, k); }),
    yazYoksa: (k) => calis("yazYoksa", () => { if (m.has(k.t)) return false; sayac.yaz++; m.set(k.t, k); return true; }),
    sil: (x) => calis("sil", () => { sayac.sil++; m.delete(x); }),
    ekOku: (x, ad) => calis("ekOku", () => { sayac.oku++; return m.get(x + ":" + ad) ?? null; }),
    ekYazYoksa: (x, ad, d) => calis("ekYazYoksa", () => { if (m.has(x + ":" + ad)) return false; sayac.yaz++; m.set(x + ":" + ad, d); return true; }),
    ekSil: (x, ad) => calis("ekSil", () => { sayac.sil++; m.delete(x + ":" + ad); }),
  };
  const istek = (islem, t, sec) => onayIsle({ islem, t, depo, genel: pubHex, istek: new Request("https://x/" + (t || islem), sec) });
  const imzaliPost = (g) => ({ method: "POST", body: g, headers: { "x-taa-imza": imzala(g) } });
  return {
    m, sayac, bozuk, depo,
    ac: (nesne) => istek("ac", "", imzaliPost(JSON.stringify(nesne))),
    kapat: (t) => istek("kapat", "", imzaliPost(JSON.stringify({ v: 1, tur: "onay_kapat", t, ts: simdi() }))),
    oku: (t) => { const ts = String(simdi()); return istek("oku", t, { headers: { "x-taa-zaman": ts, "x-taa-imza": imzala(`onay_oku:${t}:${ts}`) } }); },
    sayfa: (t) => istek("sayfa", t, { method: "GET" }),
    gonder: (t, alanlar) => istek("sayfa", t, { method: "POST", body: new URLSearchParams(alanlar).toString(), headers: { "content-type": "application/x-www-form-urlencoded" } }),
  };
}
await t_("birKezYaz: geri okuma bir kez boşsa yeniden dener; ikisi de başarısızsa BILINMIYOR (kaybettim sayılmaz)", async () => {
  const d = { yazan: "a1" };
  const yaz = async () => true;
  let n = 0;
  assert.equal(await birKezYaz(yaz, async () => (++n === 1 ? null : { yazan: "a1" }), d), KAZANDI); assert.equal(n, 2);
  n = 0;
  assert.equal(await birKezYaz(yaz, async () => { if (++n === 1) throw new Error("ağ"); return { yazan: "b2" }; }, d), KAYBETTI);
  assert.equal(await birKezYaz(yaz, async () => null, d), BILINMIYOR);
  assert.equal(await birKezYaz(yaz, async () => { throw new Error("ağ"); }, d), BILINMIYOR);
  assert.equal(await birKezYaz(async () => false, async () => d, d), KAYBETTI);
});
await t_("aç: geri okuma başarısız → 503 (409 değil); depo yazma hatası → 503", async () => {
  const D = bellekDepo();
  let okuma = 0;
  D.bozuk.oku = () => (++okuma > 1 ? new Error("ağ") : undefined); // ilk oku (var mı) çalışır, geri okumalar hata verir
  assert.equal((await D.ac(acGovde(yeniT()))).status, 503);
  const D2 = bellekDepo();
  D2.bozuk.yazYoksa = () => new Error("ağ");
  assert.equal((await D2.ac(acGovde(yeniT()))).status, 503);
  const D3 = bellekDepo();
  D3.bozuk.oku = () => new Error("ağ");
  assert.equal((await D3.ac(acGovde(yeniT()))).status, 503, "var mı okuması da hata → 503");
});
await t_("deneme: geri okuma başarısızsa tek POST tek hak alır (5 POST sonra 429)", async () => {
  const D = bellekDepo();
  const t = yeniT();
  assert.equal((await D.ac(acGovde(t))).status, 200);
  D.bozuk.ekOku = () => new Error("ağ");
  for (let i = 0; i < 5; i++) assert.equal((await D.gonder(t, { ad: "x", tel: "1" })).status, 400, "deneme " + (i + 1));
  assert.equal([...D.m.keys()].filter((a) => a.includes(":deneme:")).length, 5);
  assert.equal((await D.gonder(t, { ad: "x", tel: "1" })).status, 429);
});
await t_("sonuç: geri okunamazsa 'bir sorun oluştu' (500); müşteri yeniden gönderince 'Onayınız alındı'", async () => {
  const D = bellekDepo();
  const t = yeniT();
  assert.equal((await D.ac(acGovde(t))).status, 200);
  D.bozuk.ekOku = () => null; // sonuç ve denemeler yazılıyor ama geri okunamıyor (durumOku da sonucu göremez)
  const r = await D.gonder(t, gecerli(0));
  assert.equal(r.status, 500); assert((await r.text()).includes(ONAY_METIN.tr.h_genel));
  assert(D.m.has(t + ":sonuc"), "sonuç yazıldı");
  delete D.bozuk.ekOku;
  const r2 = await D.gonder(t, gecerli(1));
  assert.equal(r2.status, 200); assert((await r2.text()).includes(ONAY_METIN.tr.alindi_b));
  assert.equal((await (await D.oku(t)).json()).durum, "onaylandi");
});
await t_("depo hatası: müşteri sayfası ham 500 değil (GET 503 'bir sorun', POST 500 form); oku/kapat 503", async () => {
  const D = bellekDepo();
  const t = yeniT();
  assert.equal((await D.ac(acGovde(t))).status, 200);
  D.bozuk.ekYazYoksa = () => new Error("ağ");
  const p = await D.gonder(t, gecerli(0));
  assert.equal(p.status, 500); const h = await p.text();
  assert(h.includes(ONAY_METIN.tr.h_genel) && h.includes('name="ad"'), "form + genel hata");
  delete D.bozuk.ekYazYoksa;
  D.bozuk.oku = () => new Error("ağ");
  const g = await D.sayfa(t);
  assert.equal(g.status, 503); const gh = await g.text();
  assert(gh.includes(ONAY_METIN.tr.h_genel) && !gh.includes(ONAY_METIN.tr.doldu), "okunamadı 'süresi doldu' sayılmaz");
  assert.equal((await D.oku(t)).status, 503);
  assert.equal((await D.kapat(t)).status, 503, "kapat okunamayan kaydı 'yok' sayıp 200 dönmez");
  delete D.bozuk.oku;
  assert.equal(D.m.get(t).durum, "bekliyor");
  assert.equal((await D.kapat(t)).status, 200);
  assert.equal(D.m.get(t).durum, "kapandi");
  // kapandi yazılamazsa da 503 (sunucu dakikada bir yeniden dener)
  const t2 = yeniT();
  assert.equal((await D.ac(acGovde(t2))).status, 200);
  D.bozuk.yaz = () => new Error("ağ");
  assert.equal((await D.kapat(t2)).status, 503);
});
await t_("kapat yazma sayısı: açık kayıtta 1 yazma + 1 silme; kapandi / yok kayıtta hiç yazma ya da silme yok", async () => {
  const D = bellekDepo();
  const t = yeniT();
  assert.equal((await D.ac(acGovde(t))).status, 200);
  for (let i = 0; i < 3; i++) await D.gonder(t, { ad: "x", tel: "1" });
  assert.equal((await D.gonder(t, gecerli(0))).status, 200);
  const once = { ...D.sayac };
  assert.equal((await D.kapat(t)).status, 200);
  assert.deepEqual([D.sayac.yaz - once.yaz, D.sayac.sil - once.sil], [1, 1], "kapandi yazımı + sonuç silme");
  assert(!D.m.has(t + ":sonuc"));
  const ara = { ...D.sayac };
  assert.equal((await D.kapat(t)).status, 200);
  assert.equal((await D.kapat(yeniT())).status, 200);
  assert.deepEqual([D.sayac.yaz - ara.yaz, D.sayac.sil - ara.sil], [0, 0], "ikinci kapat ve kayıtsız kapat yazmaz");
});
await t_("Vercel: okuYol 'yok' (BlobNotFoundError) ile 'okunamadı'yı ayırır; kapat okunamazsa 503, kayıt bekliyor kalmaz", async () => {
  const B = sahteBlob();
  const t = yeniT();
  assert.equal((await B.ac(acGovde(t))).status, 200);
  const blob = { put: async () => ({}), head: async () => { throw new Error("fetch failed"); }, del: async () => {}, list: async () => ({ blobs: [], hasMore: false }) };
  const bozukDepo = onayDeposuKur(blob, async () => new Response("", { status: 404 }));
  await assert.rejects(bozukDepo.oku(t), /fetch failed/, "ağ hatası null değil");
  const yokDepo = onayDeposuKur({ ...blob, head: async () => { const h = new Error("Vercel Blob: The requested blob does not exist"); h.name = "BlobNotFoundError"; throw h; } });
  assert.equal(await yokDepo.oku(t), null);
  const ozelSinif = class extends Error {};
  const sinifDepo = onayDeposuKur({ ...blob, head: async () => { throw new ozelSinif("x"); }, bulunamadiMi: (h) => h instanceof ozelSinif });
  assert.equal(await sinifDepo.oku(t), null, "api/onay.mjs BlobNotFoundError sınıfını verir");
  const cdnHata = onayDeposuKur({ ...blob, head: async () => ({ url: "https://blob.invalid/x" }) }, async () => new Response("", { status: 502 }));
  await assert.rejects(cdnHata.oku(t), /502/);
  // kapat: Blob head geçici hata → 503; kayıt değişmez; sonra 200 ve kapandi
  const gercekHead = B.bloblar;
  const kg = JSON.stringify({ v: 1, tur: "onay_kapat", t, ts: simdi() });
  const bozuk = { ...onayDeposuKur(blob), tablo: async () => null };
  assert.equal((await vercelOnay(new Request("https://v.invalid/api/onay/kapat", { method: "POST", body: kg, headers: { "x-taa-imza": imzala(kg) } }), bozuk, pubHex)).status, 503);
  assert.equal(JSON.parse(gercekHead.get(`onay/${t}.json`).govde).durum, "bekliyor");
  assert.equal((await B.kapat(t)).status, 200);
  assert.equal(JSON.parse(gercekHead.get(`onay/${t}.json`).govde).durum, "kapandi");
});
await t_("Vercel: put ağ hatası → müşteri formu 'bir sorun oluştu' (500), aç 503 (ham hata atılmaz)", async () => {
  const B = sahteBlob();
  const t = yeniT();
  assert.equal((await B.ac(acGovde(t))).status, 200);
  const bloblar = B.bloblar;
  const blob = {
    put: async () => { throw new Error("fetch failed"); },
    head: async (y) => { if (!bloblar.has(y)) throw new Error("BlobNotFoundError"); return { url: "https://blob.invalid/" + y }; },
    del: async () => {}, list: async () => ({ blobs: [], hasMore: false }),
  };
  const getir = async (u) => { const b = bloblar.get(u.replace("https://blob.invalid/", "").replace(/\?v=\d+$/, "")); return b ? new Response(b.govde) : new Response("", { status: 404 }); };
  const depo = { ...onayDeposuKur(blob, getir), tablo: async () => null };
  const v = (yol, sec = {}) => vercelOnay(new Request("https://v.invalid" + yol, sec), depo, pubHex);
  const r = await v(`/o/${t}`, { method: "POST", body: new URLSearchParams(gecerli(0)).toString(), headers: { "content-type": "application/x-www-form-urlencoded" } });
  assert.equal(r.status, 500); assert((await r.text()).includes(ONAY_METIN.tr.h_genel));
  const g = JSON.stringify(acGovde(yeniT()));
  assert.equal((await v("/api/onay/ac", { method: "POST", body: g, headers: { "x-taa-imza": imzala(g) } })).status, 503);
});
await t_("Vercel temizle: belirteç başına en çok 1 okuma (yalnız ana kayıt); ekler ana kaydın sil'iyle, sahipsiz ekler silinir", async () => {
  const bloblar = new Map();
  let okuma = 0;
  const s0 = simdi();
  const koy = (y, govde, yasSn = 0) => bloblar.set(y, { govde: JSON.stringify(govde), uploadedAt: new Date((s0 - yasSn) * 1000) });
  const blob = {
    put: async () => ({}), head: async () => ({}),
    del: async (u) => { bloblar.delete(u.replace("https://blob.invalid/", "")); },
    list: async ({ prefix }) => ({ blobs: [...bloblar].filter(([y]) => y.startsWith(prefix)).map(([y, b]) => ({ url: "https://blob.invalid/" + y, pathname: y, uploadedAt: b.uploadedAt })), hasMore: false }),
  };
  const getir = async (u) => { okuma++; const b = bloblar.get(u.replace("https://blob.invalid/", "").replace(/\?v=\d+$/, "")); return b ? new Response(b.govde) : new Response("", { status: 404 }); };
  const depo = onayDeposuKur(blob, getir);
  const ekler = (t, sil) => { koy(`onay/${t}.sonuc.json`, { t, sil }); for (let n = 1; n <= 5; n++) koy(`onay/${t}.deneme-${n}.json`, { t, sil }); };
  // geçerli (yeni): ana + 6 ek → 1 okuma, hiçbiri silinmez
  koy("onay/gecerli.json", { t: "gecerli", sil: s0 + 500 }); ekler("gecerli", s0 + 500);
  // süresi geçmiş (yeni yüklenmiş, sil geçmiş): 1 okuma, ana + 6 ek silinir
  koy("onay/bitti.json", { t: "bitti", sil: s0 - 1, durum: "kapandi" }); ekler("bitti", s0 - 1);
  // çok eski yüklenmiş: okunmadan silinir (ekleriyle)
  koy("onay/eski.json", { t: "eski", sil: s0 + 500 }, 2000); ekler("eski", s0 + 500);
  // sahipsiz ekler (ana kayıt yok): okunmadan silinir
  ekler("sahipsiz", s0 + 500);
  koy("takip/x.json", {}, 99999);
  assert.equal(await depo.temizle(s0), 7 + 7 + 6);
  assert.equal(okuma, 2, "yalnız iki yeni ana kayıt okundu (ekler okunmadı)");
  assert.deepEqual([...bloblar.keys()].filter((y) => y.startsWith("onay/")).sort(),
    ["onay/gecerli.deneme-1.json", "onay/gecerli.deneme-2.json", "onay/gecerli.deneme-3.json", "onay/gecerli.deneme-4.json", "onay/gecerli.deneme-5.json", "onay/gecerli.json", "onay/gecerli.sonuc.json"]);
  assert(bloblar.has("takip/x.json"));
  // ana kayıt okunamazsa (CDN 5xx) o belirteç dokunulmadan bir sonraki tura kalır
  const bozuk = onayDeposuKur(blob, async () => new Response("", { status: 503 }));
  assert.equal(await bozuk.temizle(s0), 0);
  assert.equal([...bloblar.keys()].filter((y) => y.startsWith("onay/gecerli")).length, 7);
});

// ---------------------------------------------------------------- 8) KV bilinen sınırı (README "Bilinen sınırlar")
await t_("KV bilinen sınır: farklı uç noktada eski 'yok' önbelleği → ikinci onay ilkinin üzerine yazabilir (Durable Object gerekir)", async () => {
  // İki uç nokta aynı KV'yi paylaşır; ikinci uç, sonuç anahtarı için önbellekteki eski "yok"u görür (olumsuz okuma
  // önbelleği ~60 sn). Bu, beklenen (belgelenmiş) davranıştır: get + put atomik değildir. Sunucu ilk okuduğu onayı
  // sabitler ve hemen kapat gönderir (teslim_onay.rs sorgula_ve_guncelle: bilgi bir kez yazılır, yeniden sorgulanmaz).
  const ortakKv = new Map();
  const uc = (eskiYok) => ({
    GENEL_ANAHTAR: pubHex,
    TAKIP: {
      async get(k) { if (eskiYok.has(k)) return null; const v = ortakKv.get(k); return v ? JSON.parse(v.deger) : null; },
      // kendi yazdığını aynı uçta görür (önbellek yazımda yenilenir)
      async put(k, deger, sec = {}) { eskiYok.delete(k); ortakKv.set(k, { deger, exp: sec.expiration }); },
      async delete(k) { ortakKv.delete(k); },
    },
  });
  const t = yeniT();
  const birinci = uc(new Set()), ikinci = uc(new Set([`onay:${t}:sonuc`]));
  const g = JSON.stringify(acGovde(t));
  assert.equal((await worker.fetch(new Request("https://x/api/onay/ac", { method: "POST", body: g, headers: { "x-taa-imza": imzala(g) } }), birinci)).status, 200);
  const post = (env, i) => worker.fetch(new Request("https://x/o/" + t, { method: "POST", body: new URLSearchParams(gecerli(i)).toString(), headers: { "content-type": "application/x-www-form-urlencoded" } }), env);
  const r1 = await post(birinci, 0);
  assert((await r1.text()).includes(ONAY_METIN.tr.tesekkur_m));
  const ilk = JSON.parse(ortakKv.get(`onay:${t}:sonuc`).deger);
  const r2 = await post(ikinci, 1);
  assert((await r2.text()).includes(ONAY_METIN.tr.tesekkur_m), "ikinci uç da 'teşekkür' görür (bilinen sınır)");
  const son = JSON.parse(ortakKv.get(`onay:${t}:sonuc`).deger);
  assert.notEqual(son.yazan, ilk.yazan, "ilk onay ezildi: KV'de kesin tek seferlik onay yok");
  assert.equal(coz(t, son).ad, gecerli(1).ad);
});

// ---------------------------------------------------------------- 9) ortak Rust/JS vektörleri (test/ortak-vektorler.json)
const V = JSON.parse(fs.readFileSync(new URL("ortak-vektorler.json", import.meta.url), "utf8"));
await t_("ortak vektör: bulutun gerçek sifrele() çıktısı (aad = t:oz) 0x11 anahtarıyla çözülür, t ile çözülmez (Rust da sınar)", async () => {
  const o = V.onay_oz;
  const ozel = createPrivateKey({ key: Buffer.concat([Buffer.from("302e020100300506032b656e04220420", "hex"), Buffer.from(o.ozel, "hex")]), format: "der", type: "pkcs8" });
  assert.equal(Buffer.from(createPublicKey(ozel).export({ format: "jwk" }).x, "base64url").toString("hex"), o.pk);
  const coz2 = (aad) => {
    const eB = Buffer.from(o.e, "hex");
    const gen = createPublicKey({ key: Buffer.concat([Buffer.from("302a300506032b656e032100", "hex"), eB]), format: "der", type: "spki" });
    const anahtar = Buffer.from(hkdfSync("sha256", diffieHellman({ privateKey: ozel, publicKey: gen }), Buffer.concat([eB, Buffer.from(o.pk, "hex")]), Buffer.from("taa-teslim-onay-v1"), 32));
    const cB = Buffer.from(o.c, "base64");
    const d = createDecipheriv("aes-256-gcm", anahtar, Buffer.from(o.iv, "hex"));
    d.setAAD(Buffer.from(aad, "utf8")); d.setAuthTag(cB.subarray(-16));
    return JSON.parse(Buffer.concat([d.update(cB.subarray(0, -16)), d.final()]).toString("utf8"));
  };
  const P = coz2(onayAad(o.t, o.oz));
  assert.deepEqual([P.ad, P.tel, P.zaman], [o.ad, o.tel, o.zaman]);
  assert.throws(() => coz2(o.t), "aad = t ile çözülmez");
});
await t_("ortak vektör: tRNS PNG'leri JS imzaIcerik ile beklenen sonucu verir (Rust imza_png aynı dosyayı sınar)", async () => {
  assert(V.png.length >= 6);
  for (const { ad, b64, beklenen } of V.png) {
    const s = await imzaIcerik(b64);
    assert.equal(s.tamam ? "tamam" : s.hata, beklenen, ad);
  }
});
console.log(`v8 teslim onayı sağlamlaştırma testleri geçti (${gecti} denetim)`);
