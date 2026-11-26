import { generateKeyPairSync, sign } from "node:crypto";
import worker from "../cloudflare/worker.mjs";
import { EK } from "../ortak/ek.mjs";
import { DILLER } from "../ortak/diller.mjs";
import assert from "node:assert"; import fs from "node:fs";
// İsteğe bağlı: node test/dene3.mjs <çıktı klasörü> → ekran görüntüsü için HTML'ler (verilmezse yazılmaz).
const cikti = process.argv[2] || "";
const yazH = (ad, h) => { if (cikti) fs.writeFileSync(`${cikti}/${ad}`, h); };
const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const pubHex = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
const kv = new Map();
const env = { GENEL_ANAHTAR: pubHex, TAKIP: { async get(k){ const v=kv.get(k); return v?JSON.parse(v):null }, async put(k,v){ kv.set(k,v) } } };
let now = Math.floor(Date.now()/1000);
const yaz = (rec) => { const g=JSON.stringify(rec); const s=sign(null, Buffer.from(g), privateKey).toString("hex");
  return worker.fetch(new Request("https://x/api/yaz",{method:"POST",body:g,headers:{"x-taa-imza":s}}), env) };
const al = async (kod, q="") => (await worker.fetch(new Request("https://x/t/"+kod+q), env)).text();
// dil anahtarları tam
const ana = JSON.stringify(Object.keys(EK.tr).sort());
for (const [d] of DILLER) { assert(EK[d], d); assert.equal(JSON.stringify(Object.keys(EK[d]).sort()), ana, d);
  assert.equal(Object.keys(EK[d].acik).length, 6, d); assert.equal(Object.keys(EK[d].kd).length, 3, d); }
const u=[{u:"takim",i:"ceket-kol-kisalt"},{u:"pantolon",i:"pant-paca-kisalt",v:"duble"}];
const t = {v:3,k:"abcdefghjkmnpqrs",d:"hazir",s:"MRK01",t:"2026-11-29",g:"2026-11-26",h:"17:00",u,ts:now,sil:now+86400,
  kg:{d:"yolda",f:"yurtici",n:"1234567890",t:"2026-11-27",y:"Büyükçekmece/İstanbul"}};
let r = await yaz(t); assert.equal(r.status,200, await r.text());
let h = await al(t.k);
assert(h.includes("Ürününüz kargoya verildi") && h.includes("1234567890") && h.includes("yurticikargo.com") && h.includes("Büyükçekmece/İstanbul"));
assert(h.includes("Şubemiz") && h.includes("Esenyurt"));
yazH("v3-kargo-tr.html", h);
yazH("v3-kargo-ar.html", await al(t.k,"?dil=ar"));
// geçersiz kargo alanları
for (const kg of [{...t.kg,f:"bilinmez"},{...t.kg,n:"<x>"},{...t.kg,y:"Ali 0555 123"},{...t.kg,d:"kayip"},{...t.kg,ad:"Ayşe"}]) {
  now++; r = await yaz({...t, ts:now, kg}); 
  if (kg.ad) { assert.equal(r.status,200); assert(!kv.get(t.k).includes("Ayşe")); } else assert.equal(r.status,400, JSON.stringify(kg));
}
// kargosuz v3 = mağazadan teslim, şube kartı
const t2 = {...t, k:"bcdefghjkmnpqrst", ts:now+1}; delete t2.kg;
r = await yaz(t2); assert.equal(r.status,200);
h = await al(t2.k); assert(h.includes("Teslim alma adresi") && h.includes("Yol tarifi") && h.includes("tel:+902126900202") && h.includes("17:00"));
yazH("v3-hazir-tr.html", h);
const t3 = {...t2, k:"cdefghjkmnpqrstu", d:"terzide", ts:now+2};
r = await yaz(t3); yazH("v3-terzide-en.html", await al(t3.k,"?dil=en"));
const t4 = {...t, k:"defghjkmnpqrstuv", d:"terzide", kg:{d:"hazirlaniyor",f:"aras",n:"",t:"",y:""}, ts:now+3};
r = await yaz(t4); assert.equal(r.status,200); h=await al(t4.k); assert(h.includes("Kargoya hazırlanıyor") && h.includes("Takip numarası kargoya"));
console.log("v3 testleri geçti");
