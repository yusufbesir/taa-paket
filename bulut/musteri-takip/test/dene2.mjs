import { generateKeyPairSync, sign } from "node:crypto";
import worker from "../cloudflare/worker.mjs";
import assert from "node:assert";
const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const pubHex = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
const kv = new Map();
const env = { GENEL_ANAHTAR: pubHex, TAKIP: { async get(k){ const v=kv.get(k); return v?JSON.parse(v):null }, async put(k,v){ kv.set(k,v) } } };
const now = Math.floor(Date.now()/1000), kod = "abcdefghjkmnpqrs";
const yaz = (rec) => { const g=JSON.stringify(rec); const s=sign(null, Buffer.from(g), privateKey).toString("hex");
  return worker.fetch(new Request("https://x/api/yaz",{method:"POST",body:g,headers:{"x-taa-imza":s}}), env) };
const temel = {v:2,k:kod,d:"terzide",s:"MRK01",t:"2026-11-29",g:"2026-11-26",h:"17:00",u:[{u:"pantolon",i:"pant-paca-kisalt",v:"duble"}],ts:now,sil:now+86400};
let r = await yaz(temel); assert.equal(r.status,200, await r.text());
r = await yaz({...temel, ts:now+1, u:[{u:"pantolon", i:"<b>", }]}); assert.equal(r.status,400);
r = await yaz({...temel, ts:now+1, u:[{u:"pantolon", not:"Ayşe"}]}); assert.equal(r.status,200);
assert(!kv.get(kod).includes("Ayşe"));
r = await yaz({...temel, ts:now+2}); 
// v4: kalemler artık İşlemler sekmesinde (özet sekmesinde yalnız durum/kargo/şube/fiş tarihleri var).
r = await worker.fetch(new Request("https://x/t/"+kod+"?sekme=islemler", {headers:{"accept-language":"de-DE,de;q=0.9"}}), env); let h = await r.text();
assert(h.includes('lang="de"') && h.includes("Saum kürzen") && h.includes("Mit Aufschlag"), h);
r = await worker.fetch(new Request("https://x/t/"+kod+"?dil=ar"), env); h = await r.text();
assert(h.includes('dir="rtl"') && h.includes("قطعتك لدى الخياط"));
assert(h.includes(`href="/t/${kod}?dil=en"`));
console.log("v2 + dil testleri geçti");
import { TABLOLAR, DILLER, METIN, dilSec as ds } from "../ortak/diller.mjs";
for (const [ad, t] of Object.entries(TABLOLAR)) for (const [k, satir] of Object.entries(t))
  assert.equal(satir.length, DILLER.length, `${ad}.${k} eksik: ${satir.length}`);
const anahtarlar = JSON.stringify(Object.keys(METIN.tr).sort());
for (const [d] of DILLER) { assert(METIN[d], d); assert.equal(JSON.stringify(Object.keys(METIN[d]).sort()), anahtarlar, d); }
assert.equal(ds(null, "en-US;q=0.5, zh-CN, fa;q=0.9"), "zh");
assert.equal(ds(null, "pt-BR, el;q=0.8"), "el");
assert.equal(ds(null, "en", "a=b; dil=bg"), "bg");
assert.equal(ds("ro", "en", "dil=bg"), "ro");
r = await worker.fetch(new Request("https://x/t/"+kod+"?dil=fa"), env); h = await r.text();
assert(r.headers.get("set-cookie").startsWith("dil=fa;") && h.includes('dir="rtl"') && h.includes("<summary") && h.includes("FA"));
r = await worker.fetch(new Request("https://x/t/"+kod, {headers:{cookie:"dil=es", "accept-language":"de"}}), env); h = await r.text();
assert(h.includes('lang="es"') && !r.headers.get("set-cookie"));
console.log("13 dil, çerez ve çekmece testleri geçti");
