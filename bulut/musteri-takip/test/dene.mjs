import { generateKeyPairSync, sign } from "node:crypto";
import worker from "../cloudflare/worker.mjs";
import assert from "node:assert";
const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const pubHex = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
const kv = new Map();
const env = { GENEL_ANAHTAR: pubHex, YEDEK_SAYFA: "https://y.example",
  TAKIP: { async get(k){ const v=kv.get(k); return v?JSON.parse(v):null }, async put(k,v,o){ assert(o.expiration>0); kv.set(k,v) } } };
const now = Math.floor(Date.now()/1000);
const kod = "abcdefghjkmnpqrs";
const yaz = async (rec, imzaBoz=false) => { const g=JSON.stringify(rec); let s=sign(null, Buffer.from(g), privateKey).toString("hex"); if(imzaBoz) s=s.replace(/^./,c=>c==="a"?"b":"a");
  return worker.fetch(new Request("https://x/api/yaz",{method:"POST",body:g,headers:{"x-taa-imza":s}}), env) };
let r = await yaz({v:1,k:kod,d:"terzide",s:"MRK01",t:"2026-11-29",ts:now,sil:now+86400});
assert.equal(r.status,200, await r.text());
r = await yaz({v:1,k:kod,d:"hazir",s:"MRK01",t:"2026-11-29",ts:now+1,sil:now+86400}, true);
assert.equal(r.status,401);
r = await yaz({v:1,k:kod,d:"hazir",s:"MRK01",t:"2026-11-29",ts:now+1,sil:now+86400,musteri:"Ayşe"});
assert.equal(r.status,200);
assert(!kv.get(kod).includes("Ayşe"), "fazla alan saklanmamalı");
r = await yaz({v:1,k:kod,d:"alindi",s:"MRK01",t:"",ts:now-5,sil:now+86400}); // eski kayıt
assert.equal(JSON.parse(kv.get(kod)).d,"hazir");
r = await worker.fetch(new Request("https://x/t/"+kod), env); let h=await r.text();
assert.equal(r.status,200); assert(h.includes("Ürününüz hazır")); assert(h.includes("17:00"));
r = await worker.fetch(new Request("https://x/t/zzzzzzzzzzzzzzzz"), env); h=await r.text();
assert.equal(r.status,404); assert(h.includes("Kayıt bulunamadı") && h.includes("y.example/t/zzzz"));
r = await worker.fetch(new Request("https://x/t/<script>"), env); h=await r.text(); assert(!h.includes("<script>"));
r = await yaz({v:1,k:"x",d:"hazir",s:"",t:"",ts:now,sil:now+10});
assert.equal(r.status,400);
console.log("hepsi geçti");
// İsteğe bağlı: node test/dene.mjs <çıktı klasörü> → ornek.html (depoya yazmaz).
import { writeFileSync } from "node:fs";
if (process.argv[2]) writeFileSync(process.argv[2] + "/ornek.html", (await (await worker.fetch(new Request("https://x/t/"+kod), env)).text()));
