// v4: r/a/b doğrulama, sekmeler, dil korunumu, yol tarifi, WhatsApp, 13 dil EK2/RENK tamlığı, RTL SSS düğmesi. Yalnız sahte veri.
// Kullanım: node test/dene4.mjs [çıktı klasörü]  (verilirse ekran görüntüsü için HTML'ler oraya yazılır)
import { generateKeyPairSync, sign } from "node:crypto";
import worker from "../cloudflare/worker.mjs";
import { sayfa } from "../ortak/takip.mjs";
import { EK2, RENK, SUBELER, WHATSAPP_MERKEZ } from "../ortak/ek.mjs";
import { DILLER } from "../ortak/diller.mjs";
import assert from "node:assert"; import fs from "node:fs";
const cikti = process.argv[2] || "";
const { publicKey, privateKey } = generateKeyPairSync("ed25519");
const pubHex = publicKey.export({ format: "der", type: "spki" }).subarray(-32).toString("hex");
const kv = new Map();
const env = { GENEL_ANAHTAR: pubHex, TAKIP: { async get(k){ const v=kv.get(k); return v?JSON.parse(v):null }, async put(k,v){ kv.set(k,v) } } };
let now = Math.floor(Date.now()/1000);
const yaz = (rec) => { const g=JSON.stringify(rec); const s=sign(null, Buffer.from(g), privateKey).toString("hex");
  return worker.fetch(new Request("https://x/api/yaz",{method:"POST",body:g,headers:{"x-taa-imza":s}}), env) };
const al = async (kod, q="") => (await worker.fetch(new Request("https://x/t/"+kod+q), env)).text();
let gecti = 0; const t_ = (ad, f) => { f(); gecti++; };

// 1) r / a / b doğrulama
const u = [{u:"pantolon",i:"pant-paca-kisalt",v:"duble",r:"siyah",a:2},{u:"ceket",i:"ceket-kol-kisalt",r:"lacivert"},{u:"gomlek"}];
const t = {v:3,k:"abcdefghjkmnpqrs",d:"terzide",s:"MRK01",t:"2026-11-29",g:"2026-11-26",h:"17:00",u,b:2,ts:now,sil:now+86400};
let r = await yaz(t); assert.equal(r.status,200, await r.text());
const sakli = JSON.parse(kv.get(t.k));
t_("geçerli r/a/b saklanır", () => { assert.equal(sakli.b,2); assert.deepEqual(sakli.u[0],{u:"pantolon",i:"pant-paca-kisalt",v:"duble",r:"siyah",a:2}); assert.deepEqual(sakli.u[2],{u:"gomlek"}); });
for (const [ad, deg] of [
  ["bilinmeyen renk", {u:[{u:"pantolon",r:"fosforlu"}]}], ["renk sayı", {u:[{u:"pantolon",r:5}]}],
  ["adet 1", {u:[{u:"pantolon",a:1}]}], ["adet 21", {u:[{u:"pantolon",a:21}]}], ["adet kesirli", {u:[{u:"pantolon",a:2.5}]}],
  ["adet metin", {u:[{u:"pantolon",a:"2"}]}], ["b 0", {b:0}], ["b 21", {b:21}], ["b metin", {b:"3"}], ["b kesirli", {b:1.5}],
  ["renk __proto__", {u:[{u:"pantolon",r:"__proto__"}]}],
]) { now++; r = await yaz({...t, ts:now, ...deg}); t_(ad, () => assert.equal(r.status,400, ad)); }
for (const [ad, deg] of [["adet 20", {u:[{u:"pantolon",a:20}]}], ["b 1", {b:1}], ["b 20", {b:20}], ["acik-gri", {u:[{u:"pantolon",r:"acik-gri"}]}]]) {
  now++; r = await yaz({...t, ts:now, ...deg}); t_(ad, () => assert.equal(r.status,200, ad)); }
now++; r = await yaz({...t, ts:now, u:[{u:"pantolon",r:"siyah",not:"Ayşe Yılmaz 0555"}], musteri:"Ayşe"});
t_("bilinmeyen alan saklanmaz", () => { assert.equal(r.status,200); assert(!kv.get(t.k).includes("Ayşe")); });
now++; r = await yaz({...t, ts:now}); assert.equal(r.status,200);

// 2) dört sekme + dil korunumu
const sekmeler = ["ozet","islemler","subeler","sss"];
const htmller = {};
for (const s of sekmeler) htmller[s] = await al(t.k, `?sekme=${s}&dil=tr`);
t_("sekme çubuğu + aria-current", () => { for (const s of sekmeler) {
  const h = htmller[s];
  assert(h.includes('<nav class="sekmeler"'), s);
  assert.equal((h.match(/aria-current="page">/g)||[]).length, 1, s);
  assert(h.includes(`href="/t/${t.k}?sekme=${s}&amp;dil=tr" aria-current="page"`), s);
  for (const d of sekmeler) assert(h.includes(`href="/t/${t.k}?sekme=${d}&amp;dil=tr"`), s+"→"+d);
}});
const bilinmez = await al(t.k, "?sekme=xyz&dil=tr");
t_("bilinmeyen sekme → özet (içerik)", () => assert(bilinmez.includes(`?sekme=ozet&amp;dil=tr" aria-current="page"`) && bilinmez.includes("Ürününüz terzide")));
const en = await al(t.k, "?sekme=islemler&dil=en");
t_("sekme bağlantılarında dil korunur", () => { for (const s of sekmeler) assert(en.includes(`href="/t/${t.k}?sekme=${s}&amp;dil=en"`), s);
  assert(en.includes(`href="/t/${t.k}?sekme=islemler&amp;dil=ar"`), "dil çekmecesi sekmeyi korur"); });
t_("özet içeriği", () => { const h = htmller.ozet;
  assert(h.includes("Ürününüz terzide") && h.includes("Teslim alma adresi") && h.includes("Bu fiş 2 fişle birlikte teslim edilecek.") && h.includes("Fiş tarihi"));
  assert(!h.includes("Paça kısaltma") && !h.includes("Diğer şubelerimiz")); });
t_("işlemler: adet × ürün · renk", () => { const h = htmller.islemler;
  assert(/<bdi class="adet">2<\/bdi> × Pantolon · Siyah/.test(h), "2 × Pantolon · Siyah");
  assert(h.includes("Ceket · Lacivert") && !/× Ceket/.test(h));
  assert(/<b>Gömlek<\/b>/.test(h), "renk/adet yoksa yalnız ürün");
  assert(h.includes("Fiş tarihi") && h.includes("17:00")); });
t_("işlemler RENK çevirisi", () => { assert(/<bdi class="adet">2<\/bdi> × Hose · Schwarz/.test(sayfa(sakli, {dil:"de", sekme:"islemler", adres:"/t/x"}))); });
const dirUrl = "https://www.google.com/maps/dir/?api=1&amp;destination=" + encodeURIComponent("Terzi Adem Altun, " + SUBELER.MRK01.adres);
t_("şubeler: burada rozeti, diğer şubeler, yol tarifi dir URL", () => { const h = htmller.subeler;
  assert(h.includes("Ürününüz burada") && h.includes("Diğer şubelerimiz") && h.includes("İzmir Mağaza") && h.includes("Muş Mağaza"));
  assert(h.includes(dirUrl), "dir URL"); assert(!h.includes("maps/search")); assert(h.includes("tel:+902126900202"));
  assert.equal((h.match(/İstanbul Merkez Mağaza/g)||[]).length, 1, "kayıttaki şube diğerlerinde tekrar etmez"); });
t_("özette de yol tarifi dir URL", () => assert(htmller.ozet.includes(dirUrl)));
t_("SSS: 6 akordeon, başlık, FAB yok", () => { const h = htmller.sss;
  assert.equal((h.match(/<details class="sss">/g)||[]).length, 6); assert(h.includes("Sıkça sorulan sorular"));
  assert(!h.includes('class="sss-fab"')); });
t_("diğer sekmelerde FAB var", () => { for (const s of ["ozet","islemler","subeler"]) assert(htmller[s].includes(`class="sss-fab" href="/t/${t.k}?sekme=sss&amp;dil=tr" aria-label="Sıkça sorulan sorular"`), s); });

// 3) WhatsApp
t_("WHATSAPP_MERKEZ site numarası; numara boşken düğme yok", () => { assert.equal(WHATSAPP_MERKEZ, "905448557525");
  assert(htmller.subeler.includes("https://wa.me/905448557525?text="), "varsayılan merkez numarası");
  for (const s of Object.values(SUBELER)) assert.equal(s.wa || "", "");
  for (const s of sekmeler) assert(!sayfa(sakli, { dil: "tr", sekme: s, adres: `/t/${t.k}`, wa: "" }).includes("wa.me"), s); });
const waSss = sayfa(sakli, { dil:"tr", sekme:"sss", adres:`/t/${t.k}`, wa:"+90 (555) 000 00 00" });
const waSube = sayfa(sakli, { dil:"tr", sekme:"subeler", adres:`/t/${t.k}`, wa:"+90 (555) 000 00 00" });
const beklenenWa = `https://wa.me/905550000000?text=${encodeURIComponent(EK2.tr.wa_metin.replace("{kod}", t.k))}`;
t_("merkez numara doluyken doğru wa.me bağlantısı", () => { assert(waSss.includes(`href="${beklenenWa}"`), "sss"); assert(waSube.includes(`href="${beklenenWa}"`), "şube");
  assert(beklenenWa.includes(t.k) && !beklenenWa.includes("{kod}")); });
const eskiWa = SUBELER.MRK01.wa; SUBELER.MRK01.wa = "905551112233";
const waSube2 = sayfa(sakli, { dil:"en", sekme:"sss", adres:`/t/${t.k}`, wa:"905550000000" });
SUBELER.MRK01.wa = eskiWa;
t_("şube wa'sı merkezden önce gelir", () => assert(waSube2.includes("https://wa.me/905551112233?text=") && !waSube2.includes("wa.me/905550000000")));

// 4) kayıt yok / iptal: sekme çubuğu yok, yalnız SSS bağlantısı
const yok = await al("zzzzzzzzzzzzzzzz", "?dil=tr");
t_("kayıt yok: sekme çubuğu yok, SSS bağlantısı var", () => { assert(!yok.includes('<nav class="sekmeler"')); assert(yok.includes('href="/t/zzzzzzzzzzzzzzzz?sekme=sss&amp;dil=tr"')); });
const yokSss = await al("zzzzzzzzzzzzzzzz", "?sekme=sss&dil=tr");
t_("kayıt yok + sss: SSS içeriği", () => { assert.equal((yokSss.match(/<details class="sss">/g)||[]).length, 6); assert(!yokSss.includes('<nav class="sekmeler"')); });
now++; const ip = {...t, k:"bcdefghjkmnpqrst", d:"iptal", ts:now}; r = await yaz(ip); assert.equal(r.status,200);
const iptal = await al(ip.k, "?sekme=islemler&dil=tr");
t_("iptal: sekme çubuğu yok, içerik iptal kartı", () => { assert(!iptal.includes('<nav class="sekmeler"') && iptal.includes("İşlem iptal edildi") && iptal.includes("?sekme=sss&amp;dil=tr")); });

// 5) 13 dilde EK2 ve RENK tamlığı
const ek2Anahtar = JSON.stringify(Object.keys(EK2.tr).sort());
t_("EK2 13 dil tam", () => { for (const [d] of DILLER) { const e = EK2[d]; assert(e, d);
  assert.equal(JSON.stringify(Object.keys(e).sort()), ek2Anahtar, d);
  assert.deepEqual(Object.keys(e.sekme).sort(), [...sekmeler].sort(), d);
  for (const s of sekmeler) assert(e.sekme[s] && e.sekme[s].trim(), d+"."+s);
  assert(e.wa_metin.includes("{kod}"), d+" wa_metin"); assert(e.bagli_m.includes("{n}"), d+" bagli_m");
  assert.equal(e.sss.length, 6, d); for (const q of e.sss) assert(q.length===2 && q[0] && q[1], d);
  for (const k of ["burada","diger_subeler","wa","sss_b","sss_dugme"]) assert(e[k] && e[k].trim(), d+"."+k);
}});
t_("RENK 13 dil tam", () => { for (const [k, satir] of Object.entries(RENK)) { assert.equal(satir.length, DILLER.length, k); for (const x of satir) assert(x && x.trim(), k); } });
t_("13 dilde dört sekme üretilir", () => { for (const [d] of DILLER) for (const s of sekmeler) {
  const h = sayfa(sakli, { dil:d, sekme:s, adres:`/t/${t.k}` });
  assert(h.includes(`lang="${d}"`) && !h.includes("undefined") && h.includes(EK2[d].sekme[s]), d+"/"+s); } });

// 6) RTL'de SSS düğmesi (inset-inline-end → sol alt) ve dir="rtl"
const ar = await al(t.k, "?dil=ar");
t_("RTL SSS düğmesi", () => { assert(ar.includes('dir="rtl"') && ar.includes(`class="sss-fab" href="/t/${t.k}?sekme=sss&amp;dil=ar" aria-label="${EK2.ar.sss_dugme}"`));
  assert(/\.sss-fab\{position:fixed;inset-inline-end:16px;bottom:calc\(16px \+ env\(safe-area-inset-bottom\)\)/.test(ar));
  assert(!/\.sss-fab\{[^}]*\b(left|right):/.test(ar), "fiziksel yön yok"); assert(/@media print\{\.sss-fab/.test(ar)); });

// 7) Fable denetimi düzeltmeleri (v4.1)
// Gövde sınırı: 20 kalem r/a/i/v dolu + kargo alanı kabul edilir.
const renkler = Object.keys(RENK).sort((a, b) => b.length - a.length);
const dolu = Array.from({ length: 20 }, (_, i) => ({ u: "pantolon", i: "pant-paca-kisalt", v: "duble", r: renkler[i % renkler.length], a: 20 }));
now++; const buyuk = { ...t, v: 3, k: "cdefghjkmnpqrstu", u: dolu, b: 20, ts: now, kg: { d: "yolda", f: "kolaygelsin", n: "ABCDEFGHIJ1234567890ABCDEFGHIJ1234567890", t: "2026-11-27", y: "Büyükçekmece/İstanbul" } };
const buyukG = JSON.stringify(buyuk).length;
r = await yaz(buyuk);
t_("20 dolu kalem + kargo kabul edilir (1024'ü aşan gövde)", () => { assert(buyukG > 1024, String(buyukG)); assert.equal(r.status, 200); assert.equal(JSON.parse(kv.get(buyuk.k)).u.length, 20); });
now++; r = await yaz({ ...t, k: "cdefghjkmnpqrstu", ts: now, u: [{ u: "pantolon", i: "x".repeat(40) }], dolgu: "x".repeat(4100) });
t_("4096 baytı aşan gövde reddedilir", () => assert.equal(r.status, 400));
// "Ürününüz burada" rozeti yalnız ürün şubedeyken.
const rozet = (d, kg) => sayfa({ ...sakli, d, ...(kg ? { v: 3, kg } : {}) }, { dil: "tr", sekme: "subeler", adres: "/t/x" });
t_("rozet: alındı/terzide/hazırda var; teslimde ve kargoda yok", () => {
  for (const d of ["alindi", "terzide", "hazir"]) { const h = rozet(d); assert(h.includes("Ürününüz burada") && h.includes("Teslim alma adresi"), d); }
  const tes = rozet("teslim"); assert(!tes.includes("Ürününüz burada") && !tes.includes("Teslim alma adresi") && tes.includes("Şubemiz"), "teslim");
  const kar = rozet("hazir", { d: "yolda", f: "aras", n: "", t: "", y: "" }); assert(!kar.includes("Ürününüz burada"), "kargo"); });
// Kargoya hazırlanırken adım sırası kayıt durumundan.
const adim = (d, kgd) => sayfa({ ...sakli, v: 3, d, kg: { d: kgd, f: "diger", n: "", t: "", y: "" } }, { dil: "tr", adres: "/t/x" });
const guncel = (h) => (h.match(/<li class="su" aria-current="step">[\s\S]*?<span class="ad">([^<]+)</) || [])[1];
const bitenler = (h) => (h.match(/<li class="bitti"/g) || []).length;
t_("kargoya hazırlanıyor: alındı/terzide'de terzi adımı bitmiş görünmez", () => {
  let h = adim("alindi", "hazirlaniyor"); assert.equal(guncel(h), "Ürün alındı"); assert.equal(bitenler(h), 0); assert(!h.includes("Ürününüz kargoya hazırlanıyor"));
  h = adim("terzide", "hazirlaniyor"); assert.equal(guncel(h), "Terzide"); assert.equal(bitenler(h), 1);
  h = adim("hazir", "hazirlaniyor"); assert.equal(guncel(h), "Kargoya hazırlanıyor"); assert.equal(bitenler(h), 2);
  h = adim("terzide", "yolda"); assert.equal(guncel(h), "Kargolandı"); assert.equal(bitenler(h), 2); });
// SUBELER düz nesne erişimi: prototip üyeleri şube sayılmaz.
t_("şube kodu constructor/__proto__/toString: undefined basılmaz", () => { for (const s of ["constructor", "__proto__", "toString"]) for (const sek of ["ozet", "subeler", "sss"]) {
  const h = sayfa({ ...sakli, s }, { dil: "tr", sekme: sek, adres: "/t/x" }); assert(!h.includes("undefined"), s + "/" + sek); } });
// RTL'de adet sırası sayfa yönünü izler.
t_("adet bdi'sinde dir=ltr yok", () => { const h = sayfa(sakli, { dil: "fa", sekme: "islemler", adres: "/t/x" }); assert(h.includes('<bdi class="adet">2</bdi> × ') && !/class="adet" dir=/.test(h)); });
// Erişilebilirlik: sekme çubuğu etiketi ayrı, İşlemler/Şubeler'de h2, boş kalem durumu.
t_("nav etiketi 'Bölümler', 13 dilde dolu ve başlıktan farklı", () => { assert(htmller.ozet.includes('<nav class="sekmeler" aria-label="Bölümler">'));
  for (const [d] of DILLER) { assert(EK2[d].bolumler && EK2[d].kalem_yok && EK2[d].adres_yok && EK2[d].sss_wa_yok, d); assert(!/WhatsApp|واتس/.test(EK2[d].sss_wa_yok), d); assert(/WhatsApp|واتس/.test(EK2[d].sss[3][1]), d + " sss[3] WhatsApp'ı anmalı"); } });
t_("İşlemler ve Şubeler sekmesinde h2 var", () => { assert(/<h2 id="islemler" class="kb">/.test(htmller.islemler)); assert(/<h2 id="sube" class="kb">/.test(htmller.subeler) && /<h2 id="diger" class="kb">/.test(htmller.subeler)); });
t_("kalemsiz / v1 kayıtta İşlemler boş durum metni", () => { for (const k of [{ ...sakli, u: [] }, { v: 1, k: t.k, d: "terzide", s: "MRK01", t: "2026-11-29", ts: now, sil: now + 99 }]) {
  const h = sayfa(k, { dil: "tr", sekme: "islemler", adres: "/t/x" }); assert(h.includes("Bu fiş için işlem ayrıntısı yok.") && !h.includes('<ul class="kalemler">'), JSON.stringify(k.v)); } });
// WhatsApp numarası yokken SSS WhatsApp'ı anmaz; numara varken anar.
t_("SSS: numara yokken WhatsApp cümlesi yok, varken var", () => { const yokSss = sayfa(sakli, { dil: "tr", sekme: "sss", adres: `/t/${t.k}`, wa: "" });
  assert(!yokSss.includes("WhatsApp&#39;tan ekletebilirsiniz") && yokSss.includes(EK2.tr.sss_wa_yok));
  assert(waSss.includes("WhatsApp&#39;tan ekletebilirsiniz")); });
// Adressiz şube: not + merkez telefonu; diğer şubeler listesinde adresliler önce.
const pnd = { ...sakli, s: "PND01" };
t_("adressiz şube: merkez telefonlu not, Merkez listede ilk", () => {
  const eskiAdres = SUBELER.PND01.adres; SUBELER.PND01.adres = "";
  try {
  const oz = sayfa(pnd, { dil: "tr", adres: "/t/x" }); assert(oz.includes("Açık adres için merkez mağazamızı arayın:") && oz.includes('href="tel:+902126900202"'));
  const sb = sayfa(pnd, { dil: "tr", sekme: "subeler", adres: "/t/x" });
  const lis = sb.slice(sb.indexOf('<ul class="subeler">')); assert(lis.indexOf("İstanbul Merkez Mağaza") < lis.indexOf("Ümraniye"), "Merkez önce");
  const mrk = sayfa(sakli, { dil: "tr", sekme: "subeler", adres: "/t/x" });
  assert(mrk.includes("Açık adres için merkez mağazamızı arayın:"), "merkez kaydında diğerleri için tek not");
  assert.equal((mrk.match(/class="adres-yok"/g) || []).length, 1);
  } finally { SUBELER.PND01.adres = eskiAdres; } });
t_("daireler dolu: gelecek adım kesikli değil", () => assert(!/li\.sonra \.nokta\{[^}]*dashed/.test(htmller.ozet) && /li\.sonra \.nokta\{background:var\(--cizgi\)/.test(htmller.ozet)));

// Kişisel veri sızmıyor
t_("sayfada sahte kişisel veri yok", () => { for (const h of [...Object.values(htmller), ar, en]) assert(!h.includes("Ayşe") && !h.includes("0555")); });

// Ekran görüntüsü için HTML
const yazH = (ad, h) => { if (cikti) fs.writeFileSync(`${cikti}/${ad}.html`, h); };
for (const s of sekmeler) yazH(`v4-${s}-tr`, htmller[s]);
yazH("v4-ozet-ar", ar);
yazH("v4-subeler-en", await al(t.k, "?sekme=subeler&dil=en"));
yazH("v4-ozet-el", await al(t.k, "?dil=el"));
yazH("v4-islemler-fa", await al(t.k, "?sekme=islemler&dil=fa"));
yazH("v4-subeler-tr-wa", waSube);
console.log(`v4 testleri geçti (${gecti} denetim)`);
